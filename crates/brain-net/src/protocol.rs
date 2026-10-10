use sekiro_sim::brain::actions::PlanError;
use sekiro_sim::brain::{BossActions, Plan, Weighted};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Backend, Failure, HostedConfig};

/// The one question's name in both backends.
pub(crate) const QUESTION: &str = "boss_action";

pub(crate) fn instructions(actions: &BossActions) -> String {
    let mut text = format!(
        "You are {} fighting Wolf. Your goal: make Wolf play worse. Wolf's score is his perfect-deflect rate, his guard rate, the hits you land on him, his damage and posture taken and how fast he kills you; you win by lowering his deflects and guards, landing more hits and surviving longer. Probe instead of repeating: try untried options, drop what keeps being deflected or guarded, bait his habits (stand still, walk in slowly, strafe, back off, run away so he chases, mix timings) and punish them. Input: obs is a fair view of both fighters (plan is what runs now); tally counts this fight per action or tactic as [n, hit, blocked, deflected, whiffed, punished]; memory holds every fight so far (the same counts, Wolf's habits, and trend: his deflect%, guard%, hits you landed, hp% lost and kill seconds over the last fights, now = this fight so far); why is what triggered this question. You are asked only a few times a fight. The boss samples from your probabilities all fight, so spread them over several good options. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave an act to your own AI. Options are single actions, movement tactics (distances from Wolf, left and right are yours) and plans of up to {} steps joined with > (consecutive attacks joined with +):",
        actions.name, actions.max_plan
    );
    for choice in actions.choices() {
        text.push_str(&format!("\n{}: {}", choice.value, choice.label));
    }
    text
}

/// Schemas verified against official documentation on 2026-10-10:
/// https://developers.openai.com/api/reference/resources/decisions/methods/create
/// https://developers.openai.com/api/docs/guides/decisions
/// https://developers.openai.com/api/reference/resources/responses/methods/create
/// Decisions takes text input and 2..=255 unique choices in one named question.
/// Its answers are typed (choice or refusal), with model and usage at the top level; a choice
/// answer carries `probabilities`, one `{value, probability}` per choice.
/// `excluded` plan texts are left out of the choices of a Responses request (it has one answer
/// and no probabilities to sample); a Decisions request offers every choice and the driver
/// penalises the repelled ones when sampling.
pub(crate) fn body(
    config: &HostedConfig,
    actions: &BossActions,
    input: &str,
    excluded: &[String],
) -> Value {
    let skip =
        |value: &str| config.backend == Backend::Responses && excluded.iter().any(|e| e == value);
    let mut choices: Vec<Value> = actions
        .choices()
        .into_iter()
        .filter(|c| !skip(&c.value))
        .map(|c| json!({"value": c.value, "description": c.label}))
        .collect();
    choices.push(json!({"value": "lua", "description": "leave it to his own AI"}));
    let instructions = instructions(actions);
    match config.backend {
        Backend::Decisions => json!({
            "model": config.model, "input": input,
            "questions": [{"type": "choice", "name": QUESTION,
                "instructions": instructions, "choices": choices}]
        }),
        Backend::Responses => {
            let values: Vec<_> = choices.iter().map(|c| c["value"].clone()).collect();
            json!({
                "model": config.model, "reasoning": {"effort": "none"},
                "instructions": instructions, "input": [{"role": "user", "content": input}],
                "text": {"format": {"type": "json_schema", "name": QUESTION, "strict": true,
                    "schema": {"type": "object", "properties": {"action": {"type": "string", "enum": values}},
                        "required": ["action"], "additionalProperties": false}}},
                "max_output_tokens": 16, "store": false
            })
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionAnswer {
    action: String,
}

pub(crate) fn choice(backend: Backend, response: &Value) -> Result<String, Failure> {
    match backend {
        Backend::Decisions => {
            let answers = response["answers"]
                .as_array()
                .ok_or(Failure::InvalidAnswer)?;
            if answers.len() != 1 || answers[0]["name"] != QUESTION {
                return Err(Failure::InvalidAnswer);
            }
            let answer = &answers[0];
            match answer["type"].as_str() {
                Some("refusal") => Err(Failure::Refusal),
                Some("choice") => answer["choice"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(Failure::InvalidAnswer),
                _ => Err(Failure::InvalidAnswer),
            }
        }
        Backend::Responses => {
            let output = response["output"]
                .as_array()
                .ok_or(Failure::InvalidAnswer)?;
            let mut texts = Vec::new();
            for item in output {
                if item["type"] != "message" || item["role"] != "assistant" {
                    continue;
                }
                if let Some(content) = item["content"].as_array() {
                    for part in content {
                        if part["type"] == "refusal" {
                            return Err(Failure::Refusal);
                        }
                        if part["type"] == "output_text" {
                            texts.push(part["text"].as_str().ok_or(Failure::InvalidAnswer)?);
                        }
                    }
                }
            }
            if response["status"] != "completed" {
                return Err(Failure::Incomplete);
            }
            if texts.len() != 1 {
                return Err(Failure::InvalidAnswer);
            }
            serde_json::from_str::<ActionAnswer>(texts[0])
                .map(|answer| answer.action)
                .map_err(|_| Failure::InvalidAnswer)
        }
    }
}

pub(crate) fn plan(value: &str, actions: &BossActions) -> Result<Option<Plan>, Failure> {
    if value == "lua" {
        return Ok(None);
    }
    match actions.parse(value) {
        Ok(plan) => Ok(Some(plan)),
        Err(PlanError::Empty | PlanError::TooLong(_)) => Err(Failure::InvalidPlan),
        Err(PlanError::Unknown(_) | PlanError::UnknownToken(_)) => Err(Failure::UnknownAction),
    }
}

/// The plan set behind a Decisions answer: its `probabilities`, each value read as a plan
/// (`lua` leaves the act to the Lua AI). Values that are not a valid plan are skipped; an
/// answer without probabilities gives an empty set (the chosen plan stands alone).
pub(crate) fn plan_set(backend: Backend, response: &Value, actions: &BossActions) -> Vec<Weighted> {
    if backend != Backend::Decisions {
        return Vec::new();
    }
    let Some(list) = response["answers"][0]["probabilities"].as_array() else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let weight = entry["probability"].as_f64()? as f32;
            let plan = plan(entry["value"].as_str()?, actions).ok()?;
            (weight.is_finite() && weight > 0.0).then_some(Weighted { plan, weight })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApiKey;
    use sekiro_sim::brain::actions::{ActionKind, BossAction};
    use sekiro_sim::brain::{OWL, SOLDIER, Tactic};

    /// A boss small enough to print whole in the docs: one attack, a back step, two tactics
    /// and one combined plan.
    pub(crate) const DOC_BOSS: BossActions = BossActions {
        chr: "c0000",
        name: "a doc example boss",
        actions: &[
            BossAction {
                id: 3005,
                label: "front kick",
                kind: ActionKind::Attack,
                approach: 2.8,
                pressure: None,
            },
            BossAction {
                id: 5201,
                label: "back step",
                kind: ActionKind::Step,
                approach: 0.0,
                pressure: None,
            },
        ],
        reflex_acts: &[],
        max_plan: 3,
        combos: &[],
        back_steps: &[5201],
        scripted: [&[3005], &[3005]],
        chains: &["walk_in4>3005"],
        tactics: &[Tactic::Hold(2.0), Tactic::WalkIn(4.0)],
    };

    #[test]
    fn documented_requests_match_wire_bodies() {
        let docs = include_str!("../../../docs/BRAIN.md").replace("\r\n", "\n");
        for (backend, heading) in [
            (Backend::Decisions, "### Decisions request"),
            (Backend::Responses, "### Responses request"),
        ] {
            let section = docs.split_once(heading).unwrap().1;
            let example = section
                .split_once("```json\n")
                .unwrap()
                .1
                .split_once("\n```")
                .unwrap()
                .0;
            let documented: Value = serde_json::from_str(example).unwrap();
            let config = HostedConfig::new(backend, ApiKey::new("test-key-not-real"));
            assert_eq!(
                documented,
                body(
                    &config,
                    &DOC_BOSS,
                    r#"{"id":1,"obs":{},"tally":{},"memory":{}}"#,
                    &[]
                )
            );
        }
    }

    #[test]
    fn plans_validate_every_component_and_length() {
        assert_eq!(
            plan("3000+3001+3002", &OWL).unwrap().unwrap().actions(),
            [3000, 3001, 3002]
        );
        assert_eq!(plan("3000+9999+3002", &OWL), Err(Failure::UnknownAction));
        assert_eq!(plan("3000+3001+3002+3000", &OWL), Err(Failure::InvalidPlan));
        for invalid in [
            "",
            "+3000",
            "3000+",
            "03000",
            "lua+3000",
            "3000++3001",
            ">3000",
            "walk_in5>3000",
            "hold2+3000",
            "walk_in4>hold2>3000>3001",
        ] {
            assert!(plan(invalid, &OWL).is_err(), "{invalid}");
        }
        let p = plan("walk_in4>hold2>3005", &OWL).unwrap().unwrap();
        assert_eq!(p.value(), "walk_in4>hold2>3005");
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn probabilities_become_a_plan_set() {
        let response = json!({"answers": [{"name": "boss_action", "type": "choice",
            "choice": "3000", "probabilities": [
                {"value": "3000", "probability": 0.5},
                {"value": "walk_in4>3005", "probability": 0.3},
                {"value": "lua", "probability": 0.15},
                {"value": "9999", "probability": 0.04},
                {"value": "3005", "probability": 0.0}]}]});
        let set = plan_set(Backend::Decisions, &response, &OWL);
        let values: Vec<_> = set
            .iter()
            .map(|w| (w.plan.as_ref().map(Plan::value), w.weight))
            .collect();
        assert_eq!(
            values,
            [
                (Some("3000".to_string()), 0.5),
                (Some("walk_in4>3005".to_string()), 0.3),
                (None, 0.15),
            ]
        );
        assert!(plan_set(Backend::Responses, &response, &OWL).is_empty());
        let bare =
            json!({"answers": [{"name": "boss_action", "type": "choice", "choice": "3000"}]});
        assert!(plan_set(Backend::Decisions, &bare, &OWL).is_empty());
    }

    #[test]
    fn both_bosses_offer_tactics_within_the_api_limit() {
        for table in [&OWL, &SOLDIER] {
            let choices = table.choices();
            // Plus `lua`: 2 to 255 unique values.
            assert!(
                (2..=254).contains(&(choices.len() + 1)),
                "{}",
                choices.len()
            );
            let mut seen = std::collections::HashSet::new();
            for c in &choices {
                assert!(seen.insert(c.value.clone()), "duplicate {}", c.value);
                let plan = table
                    .parse(&c.value)
                    .unwrap_or_else(|e| panic!("{}: {e}", c.value));
                assert_eq!(plan.value(), c.value, "canonical text form");
                assert!(table.act_for_plan(&plan, 0.5).is_ok(), "{}", c.value);
            }
            for t in Tactic::ALL {
                assert!(seen.contains(&t.token()));
            }
            eprintln!(
                "{}: {} choices with lua, instructions {} bytes",
                table.chr,
                choices.len() + 1,
                instructions(table).len()
            );
        }
    }

    #[test]
    fn the_soldier_gets_his_own_vocabulary_and_instructions() {
        let config = HostedConfig::new(Backend::Decisions, ApiKey::new("test-key-not-real"));
        let b = body(&config, &SOLDIER, "{}", &[]);
        let q = &b["questions"][0];
        assert_eq!(q["name"], QUESTION);
        let values: Vec<_> = q["choices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["value"].as_str().unwrap().to_string())
            .collect();
        for want in [
            "3013",
            "3000+3001",
            "3005+3007",
            "hold2",
            "walk_in4>3013",
            "lua",
        ] {
            assert!(values.iter().any(|v| v == want), "{want}");
        }
        assert_eq!(values.last().map(String::as_str), Some("lua"));
        let text = q["instructions"].as_str().unwrap();
        assert!(
            text.starts_with("You are an Ashina soldier with a one-handed sword fighting Wolf.")
        );
        assert!(text.contains("\n3013: perilous thrust"), "{text}");
        assert!(
            text.contains("\n3005+3007: kick, then rising slash"),
            "{text}"
        );
        assert!(text.contains("make Wolf play worse"));
        assert!(!text.contains("Owl"));
        assert_eq!(
            plan("3005+3007", &SOLDIER).unwrap().unwrap().actions(),
            [3005, 3007]
        );
        assert_eq!(plan("3031", &SOLDIER), Err(Failure::UnknownAction));
        assert_eq!(plan("3004", &SOLDIER), Err(Failure::UnknownAction));
    }

    #[test]
    fn responses_requests_leave_out_repelled_plans() {
        let config = HostedConfig::new(Backend::Responses, ApiKey::new("test-key-not-real"));
        let excluded = vec!["3003".to_string(), "walk_in4>3013".to_string()];
        let b = body(&config, &SOLDIER, "{}", &excluded);
        let values = b["text"]["format"]["schema"]["properties"]["action"]["enum"]
            .as_array()
            .unwrap();
        assert!(!values.iter().any(|v| v == "3003" || v == "walk_in4>3013"));
        assert!(values.iter().any(|v| v == "3000"));
        // Decisions offers every choice and lets the sampler penalise.
        let config = HostedConfig::new(Backend::Decisions, ApiKey::new("test-key-not-real"));
        let b = body(&config, &SOLDIER, "{}", &excluded);
        assert!(
            b["questions"][0]["choices"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["value"] == "3003")
        );
    }
}
