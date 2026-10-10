use sekiro_sim::brain::{BossActions, Plan};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Backend, Failure, HostedConfig, combo};

pub(crate) fn instructions(actions: &BossActions) -> String {
    let mut text = String::from(
        "You are the Great Shinobi Owl fighting Wolf. Pick the next action using only the fair observation and outcome tally. Reflexes (guarding, deflecting, counters and reactions) are automatic. Choose lua to leave this act to your own AI. Actions:",
    );
    for action in actions.actions {
        text.push_str(&format!("\n{}: {}", action.id, action.label));
    }
    if let Some(combo) = combo(actions) {
        text.push_str(&format!("\n{combo}: three-slash combo"));
    }
    text
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
    if let Some(combo) = combo(actions) {
        choices.push(json!({"value": combo, "description": "three-slash combo"}));
    }
    choices.push(json!({"value": "lua", "description": "leave it to his own AI"}));
    let instructions = instructions(actions);
    match config.backend {
        Backend::Decisions => json!({
            "model": config.model, "input": input,
            "questions": [{"type": "choice", "name": "owl_action",
                "instructions": instructions, "choices": choices}]
        }),
        Backend::Responses => {
            let values: Vec<_> = choices.iter().map(|c| c["value"].clone()).collect();
            json!({
                "model": config.model, "reasoning": {"effort": "none"},
                "instructions": instructions, "input": [{"role": "user", "content": input}],
                "text": {"format": {"type": "json_schema", "name": "owl_action", "strict": true,
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
            if answers.len() != 1 || answers[0]["name"] != "owl_action" {
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
    use sekiro_sim::brain::OWL;

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
                body(&config, &OWL, r#"{"id":1,"obs":{},"tally":{}}"#)
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
}
