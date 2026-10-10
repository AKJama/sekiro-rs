use sekiro_sim::brain::{BossActions, Combo, Plan};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Backend, Failure, HostedConfig};

/// The one question's name in both backends.
pub(crate) const QUESTION: &str = "boss_action";

pub(crate) fn instructions(actions: &BossActions) -> String {
    let mut text = format!(
        "You are {} fighting Wolf. Pick your next action. Input: obs is a fair view of both fighters; tally counts this fight per action as [n, hit, blocked, deflected, whiffed, punished]; memory holds every fight so far (earlier fights and wins, the same counts, and Wolf's habits: his attack distance, time at each range, what he does after deflects, guards and your back steps, how he answers and how fast he punishes each of your actions). Bait his habits and punish them. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave this act to your own AI. Actions:",
        actions.name
    );
    for action in actions.actions {
        text.push_str(&format!("\n{}: {}", action.id, action.label));
    }
    for combo in combos(actions) {
        text.push_str(&format!("\n{}: {}", combo.value(), combo.label));
    }
    text
}

/// The boss's named chains that fit in one plan.
fn combos(actions: &BossActions) -> impl Iterator<Item = &'static Combo> {
    let max = actions.max_plan;
    actions.combos.iter().filter(move |c| c.ids.len() <= max)
}

/// Schemas verified against official documentation on 2026-10-10:
/// https://developers.openai.com/api/reference/resources/decisions/methods/create
/// https://developers.openai.com/api/docs/guides/decisions
/// https://developers.openai.com/api/reference/resources/responses/methods/create
/// Decisions takes text input and 2..=255 unique choices in one named question.
/// Its answers are typed (choice or refusal), with model and usage at the top level.
pub(crate) fn body(config: &HostedConfig, actions: &BossActions, input: &str) -> Value {
    let mut choices: Vec<Value> = actions
        .actions
        .iter()
        .map(|a| json!({"value": a.id.to_string(), "description": a.label}))
        .collect();
    for combo in combos(actions) {
        choices.push(json!({"value": combo.value(), "description": combo.label}));
    }
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
    let ids: Vec<i32> = value
        .split('+')
        .map(|part| {
            let id = part.parse().map_err(|_| Failure::UnknownAction)?;
            if actions.get(id).is_none() || part != id.to_string() {
                return Err(Failure::UnknownAction);
            }
            Ok(id)
        })
        .collect::<Result<_, _>>()?;
    if ids.is_empty() || ids.len() > actions.max_plan {
        return Err(Failure::InvalidPlan);
    }
    Ok(Some(Plan { actions: ids }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApiKey;
    use sekiro_sim::brain::{OWL, SOLDIER};

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
                body(&config, &OWL, r#"{"id":1,"obs":{},"tally":{},"memory":{}}"#)
            );
        }
    }

    #[test]
    fn plans_validate_every_component_and_length() {
        assert_eq!(
            plan("3000+3001+3002", &OWL).unwrap().unwrap().actions,
            [3000, 3001, 3002]
        );
        assert_eq!(plan("3000+9999+3002", &OWL), Err(Failure::UnknownAction));
        assert_eq!(plan("3000+3001+3002+3000", &OWL), Err(Failure::InvalidPlan));
        for invalid in ["", "+3000", "3000+", "03000", "lua+3000", "3000++3001"] {
            assert!(plan(invalid, &OWL).is_err());
        }
    }

    #[test]
    fn the_soldier_gets_his_own_vocabulary_and_instructions() {
        let config = HostedConfig::new(Backend::Decisions, ApiKey::new("test-key-not-real"));
        let b = body(&config, &SOLDIER, "{}");
        let q = &b["questions"][0];
        assert_eq!(q["name"], QUESTION);
        let values: Vec<_> = q["choices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["value"].as_str().unwrap().to_string())
            .collect();
        let mut want: Vec<_> = SOLDIER.actions.iter().map(|a| a.id.to_string()).collect();
        want.extend(["3000+3001", "3005+3007", "lua"].map(String::from));
        assert_eq!(values, want);
        let text = q["instructions"].as_str().unwrap();
        assert!(
            text.starts_with("You are an Ashina soldier with a one-handed sword fighting Wolf.")
        );
        assert!(text.contains("\n3013: perilous thrust"), "{text}");
        assert!(
            text.contains("\n3005+3007: kick, then rising slash"),
            "{text}"
        );
        assert!(text.contains("Bait his habits and punish them."));
        assert!(!text.contains("Owl"));
        // His plans validate against his table only.
        assert_eq!(
            plan("3005+3007", &SOLDIER).unwrap().unwrap().actions,
            [3005, 3007]
        );
        assert_eq!(plan("3031", &SOLDIER), Err(Failure::UnknownAction));
        assert_eq!(plan("3004", &SOLDIER), Err(Failure::UnknownAction));
    }
}
