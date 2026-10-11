use sekiro_sim::brain::actions::PlanError;
use sekiro_sim::brain::coach::{CHECKPOINTS, GAME_PLAN_VERSION, KEEP};
use sekiro_sim::brain::{BossActions, GamePlan, Plan, SetOffer, Weighted};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Backend, Failure, HostedConfig};

/// The one question's name in both backends.
pub(crate) const QUESTION: &str = "boss_action";

pub(crate) fn instructions(actions: &BossActions) -> String {
    let mut text = format!(
        "You are {} fighting Wolf. Your goal: make Wolf play worse by fighting him. You are scored on the damage you deal to Wolf (his HP and posture) per minute, the attacks you land per minute, fewer perfect deflects by him, and fewer of your actions he punishes. How long the fight lasts is not a goal. Stalling, running away or passivity counts as failure: a fight where no attack of yours started in the last 4 s more than 35% of the time failed, whatever else happened, plans without an attack are ignored, you must start an attack at least every 6 s and may back off or retreat only once per 8 s. Movement tactics exist to set up attacks (baits, spacing, timing mixups), not to avoid fighting. Think like a swordsman: turning your back or retreating while the opponent is in sword range leaves you open, so tactics should set up attacks safely. Probe instead of repeating: try untried options, drop what keeps being deflected or guarded, bait his habits (stand still briefly, walk in slowly, strafe, mix timings) and punish them. Input: obs is a fair view of both fighters (plan is what runs now); tally counts this fight per action or tactic as [n, hit, blocked, deflected, whiffed, punished]; memory holds every fight so far (the same counts, Wolf's habits, and trend: his deflect%, guard%, hits you landed, hp% lost, attacks per minute, damage per minute and passive percent, and kill seconds for information only, over the last fights, now = this fight so far); why is what triggered this question. You are asked only a few times a fight. The boss samples from your probabilities all fight, so spread them over several good options. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave an act to your own AI. Options are single actions, movement tactics (distances from Wolf, left and right are yours) and plans of up to {} steps joined with > (consecutive attacks joined with +):",
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
        Backend::Decisions | Backend::Coach => json!({
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
        Backend::Decisions | Backend::Coach => {
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

// ---- the coach ----

/// The coach's structured output name.
pub(crate) const GAME_PLAN: &str = "game_plan";
/// The checkpoint chooser's question name.
pub(crate) const SET_QUESTION: &str = "plan_set";

/// What the coach is told: its job, the input, the game plan it writes and every choice it may
/// use, with labels.
pub(crate) fn coach_instructions(actions: &BossActions) -> String {
    let mut text = format!(
        "You are the coach of {name}, who fights the player (Wolf) again and again. After each fight you write the game plan for the next one. Your goal: make Wolf play worse by fighting him. You are scored on the damage you deal to Wolf (HP and posture) per minute (dpm), your attacks per minute (apm), fewer perfect deflects (defl) and fewer of your actions he punishes. How long the fight lasts, and how fast he kills you, are NOT goals (kill time is information only). Stalling, running away or passivity counts as failure: passive = the percent of the fight with no attack of yours started in the last 4 s, and a fight with passive above 35 failed regardless of every other number; if the last fight failed this way, make the next plan attack more. Movement tactics exist to set up attacks (baits, spacing, timing mixups such as a hold or a slow walk-in before a strike), not to avoid fighting. Think like a swordsman: turning your back or retreating while the opponent is in sword range leaves you open, so tactics should set up attacks safely. The fighter enforces limits you cannot override: a plan in a set must contain an attack (plans without one are dropped), an attack must start at least every 6 s, and back_off, retreat or back-step plans run at most once per 8 s. Players learn patterns, so probe with untried tactics, exploit the habits you see and retire what keeps failing. Test one idea at a time and write down what you learn.\n\
How combat works: Posture is the real bar: breaking it opens a deathblow; the deathblow spends one enemy life. Guarding costs the guarder posture; deflects cost the attacker posture and less to the deflector. HP loss slows posture recovery. Pressure only pays off while the enemy can still act; stop planning during a deathblow opening or stagger. Attack until deflected, then answer the counter. Jump and stomp sweeps, Mikiri thrusts (or deflect), and dodge grabs.\n\
Input (compact JSON about the fight just over): fight = fights so far; result = Wolf's result (won = who won, secs, defl/guard = Wolf's perfect-deflect and guard percent, hits = hits you landed, hp/posture = percent of his bars lost, punishes = your actions he hit, apm, dpm = percent of a bar per minute, passive percent); trend = his deflect%, guard%, hits, hp%, kill seconds (information only), apm, dpm and passive over the last fights; plans = this fight per plan text, as [exchanges, hit, blocked, deflected, whiffed, punished] (lua = your own AI's acts; exchanges include tactics, but only attacks can whiff); range = per-plan mean [attack-start distance, observed melee reach] in metres, including root travel, hit capsules and Wolf's radius; absent for projectile-only attacks; a radial envelope relative to the starting position, so distance above reach suggests insufficient range; target movement, facing and timing still affect contact; sets = this fight per set of the game plan played: secs and the same counts (opener = the opener, lua = own AI); wolf = his attacks this fight, same counts; overall = every fight per plan text; habits = how Wolf behaves; played = the game plan this fight ran; notes = your own notes from last time. With no played plan this fight was your plain AI, the baseline.\n\
Output: one game plan. opener = a plan run first in the fight (may be tactics such as hold3.5>walk_in4, or lua for none). sets = 2 to 4 named sets (name of letters, digits, _ or -, at most 24 characters), each with a one-line description for the fighter who picks the set (what it does and when it suits), 2 to 6 weighted plans from the choices below (weights relative), and when = the checkpoint it suits or null. start_set = the set played from the start. avoid = choices not to use, each with a short reason. notes = at most 400 characters: your hypotheses about this player, kept and shown to you next time so your reasoning accumulates; revise them with the new evidence. In the fight a cheap model only picks which set to switch to at the checkpoints (start, pressure, pressure_critical, half_hp (fallback), mark_lost, punished_twice, deflected_thrice, or keep), and your plans run locally, sampled by weight with a penalty for plans that keep being deflected, so spread weights over good options and make the sets clearly different. pressure means about 6 Wolf hits to break, pressure_critical about 3 but at least 2; both re-arm above 70% remaining posture. mark_lost only fires with lives still left; checkpoints wait until the boss can act. half_hp is the backup event at 50% HP left. Reflexes (guarding, deflecting, counters, reactions) are automatic. Use only the choices listed; lua leaves an act to the boss's own AI. A choice is a single action, a movement tactic (distances from Wolf, left and right are the boss's) or a plan of up to {max} steps joined with > (consecutive attacks joined with +). Choices:",
        name = actions.name,
        max = actions.max_plan
    );
    for choice in actions.choices() {
        text.push_str(&format!("\n{}: {}", choice.value, choice.label));
    }
    text.push_str("\nlua: leave the act to the boss's own AI");
    text
}

/// Every value the coach may write as a plan (the choices and `lua`).
fn choice_values(actions: &BossActions) -> Vec<String> {
    let mut v: Vec<String> = actions.choices().into_iter().map(|c| c.value).collect();
    v.push("lua".into());
    v
}

/// The game plan's JSON schema for strict structured output: every property required, no extra
/// ones, choices enum-constrained through one shared definition. Length limits (set count,
/// notes) are enforced after the answer, by `GamePlan::validate`.
pub(crate) fn game_plan_schema(actions: &BossActions) -> Value {
    let choice = json!({"$ref": "#/$defs/choice"});
    let checkpoints: Vec<Value> = CHECKPOINTS
        .iter()
        .map(|c| json!(c))
        .chain(std::iter::once(Value::Null))
        .collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["version", "opener", "sets", "start_set", "avoid", "notes"],
        "$defs": {"choice": {"type": "string", "enum": choice_values(actions)}},
        "properties": {
            "version": {"type": "integer", "enum": [GAME_PLAN_VERSION]},
            "opener": choice,
            "sets": {"type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["name", "description", "plans", "when"],
                "properties": {
                    "name": {"type": "string"},
                    "description": {"type": "string"},
                    "plans": {"type": "array", "items": {
                        "type": "object", "additionalProperties": false,
                        "required": ["plan", "weight"],
                        "properties": {"plan": choice, "weight": {"type": "number"}}}},
                    "when": {"type": ["string", "null"], "enum": checkpoints}
                }}},
            "start_set": {"type": "string"},
            "avoid": {"type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["choice", "reason"],
                "properties": {"choice": choice, "reason": {"type": "string"}}}},
            "notes": {"type": "string"}
        }
    })
}

/// `POST /v1/responses` for the coach (verified against the Responses reference and the
/// structured outputs and reasoning guides on 2026-10-11, see `docs/BRAIN.md`).
pub(crate) fn coach_body(config: &HostedConfig, actions: &BossActions, input: &str) -> Value {
    json!({
        "model": config.coach_model,
        "reasoning": {"effort": config.coach_effort},
        "instructions": coach_instructions(actions),
        "input": [{"role": "user", "content": input}],
        "text": {"format": {"type": "json_schema", "name": GAME_PLAN, "strict": true,
            "schema": game_plan_schema(actions)}},
        "max_output_tokens": config.coach_max_output_tokens,
        "store": false
    })
}

/// The game plan inside a Responses answer (one assistant `output_text` holding the JSON).
pub(crate) fn coach_plan(response: &Value) -> Result<GamePlan, Failure> {
    let output = response["output"]
        .as_array()
        .ok_or(Failure::InvalidAnswer)?;
    let mut texts = Vec::new();
    for item in output {
        if item["type"] != "message" || item["role"] != "assistant" {
            continue;
        }
        for part in item["content"].as_array().into_iter().flatten() {
            if part["type"] == "refusal" {
                return Err(Failure::Refusal);
            }
            if part["type"] == "output_text" {
                texts.push(part["text"].as_str().ok_or(Failure::InvalidAnswer)?);
            }
        }
    }
    if response["status"] != "completed" {
        return Err(Failure::Incomplete);
    }
    if texts.len() != 1 {
        return Err(Failure::InvalidAnswer);
    }
    GamePlan::from_json(texts[0]).map_err(|_| Failure::InvalidAnswer)
}

/// Pulls the game plan out of free text from a CLI: the JSON object between the first `{` and
/// the last `}` (a model may wrap it in a code fence).
pub(crate) fn plan_from_text(text: &str) -> Result<GamePlan, Failure> {
    let start = text.find('{').ok_or(Failure::InvalidAnswer)?;
    let end = text.rfind('}').ok_or(Failure::InvalidAnswer)?;
    if end < start {
        return Err(Failure::InvalidAnswer);
    }
    GamePlan::from_json(&text[start..=end]).map_err(|_| Failure::InvalidAnswer)
}

/// Instructions of the checkpoint chooser.
pub(crate) fn chooser_instructions(actions: &BossActions) -> String {
    format!(
        "You are {}'s fighting instinct. The coach prepared these plan sets; pick one for what just happened, or keep the current one.",
        actions.name
    )
}

/// The checkpoint chooser's Decisions body: one choice question over the set names and `keep`.
pub(crate) fn chooser_body(
    config: &HostedConfig,
    actions: &BossActions,
    input: &str,
    sets: &[SetOffer],
) -> Value {
    let mut choices: Vec<Value> = sets
        .iter()
        .map(|s| json!({"value": s.name, "description": s.description}))
        .collect();
    choices.push(json!({"value": KEEP, "description": "stay on the current set"}));
    json!({
        "model": config.model, "input": input,
        "questions": [{"type": "choice", "name": SET_QUESTION,
            "instructions": chooser_instructions(actions), "choices": choices}]
    })
}

/// The chosen set name (or `keep`) of a Decisions answer, checked against the offers.
pub(crate) fn chooser_choice(response: &Value, sets: &[SetOffer]) -> Result<String, Failure> {
    let answers = response["answers"]
        .as_array()
        .ok_or(Failure::InvalidAnswer)?;
    if answers.len() != 1 || answers[0]["name"] != SET_QUESTION {
        return Err(Failure::InvalidAnswer);
    }
    match answers[0]["type"].as_str() {
        Some("refusal") => Err(Failure::Refusal),
        Some("choice") => {
            let value = answers[0]["choice"]
                .as_str()
                .ok_or(Failure::InvalidAnswer)?;
            if value == KEEP || sets.iter().any(|s| s.name == value) {
                Ok(value.to_owned())
            } else {
                Err(Failure::UnknownAction)
            }
        }
        _ => Err(Failure::InvalidAnswer),
    }
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
    fn coach_effort_is_configurable_and_checkpoints_are_in_the_schema() {
        let mut config = HostedConfig::new(Backend::Coach, ApiKey::new("fake"));
        config.coach_effort = "high".into();
        let b = coach_body(&config, &DOC_BOSS, "{}");
        assert_eq!(b["reasoning"]["effort"], "high");
        let schema = b["text"]["format"]["schema"].to_string();
        assert!(schema.contains("pressure") && schema.contains("pressure_critical"));
        assert!(!schema.contains("posture_half") && !schema.contains("posture_critical"));
        let instructions = coach_instructions(&DOC_BOSS);
        let combat = instructions
            .split_once("How combat works: ")
            .unwrap()
            .1
            .split('\n')
            .next()
            .unwrap();
        assert!(combat.chars().count() <= 700);
        for fact in [
            "Posture is the real bar",
            "Guarding costs the guarder",
            "deflects cost the attacker",
            "HP loss slows",
            "enemy can still act",
            "Jump",
            "Mikiri",
            "dodge grabs",
        ] {
            assert!(combat.contains(fact), "missing {fact}: {combat}");
        }
    }

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
