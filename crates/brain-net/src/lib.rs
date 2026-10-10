//! Hosted boss choices. Only the worker performs HTTP; the simulation never leaves its thread.

mod config;
mod protocol;
mod worker;

pub use config::{ApiKey, Backend, ConfigError, HostedConfig};
pub use worker::{Failure, HostedBrain, HostedStats, StatsHandle, TokenUsage};

use sekiro_sim::brain::{BossActions, BossBrain, BrainKind, LuaBrain};

/// Game CLI boundary. Configuration or worker startup failure leaves the Lua AI in control.
/// Constructing a controller does not send a request; only `submit` does.
pub fn from_cli(name: &str, model: Option<String>) -> Result<Box<dyn BossBrain>, String> {
    let backend = match name.to_ascii_lowercase().as_str() {
        "decisions" => Backend::Decisions,
        "responses" => Backend::Responses,
        _ => {
            return BrainKind::by_name(name).map(BrainKind::build).map_err(|_| {
                "unknown brain: known are lua, scripted, decisions, responses".into()
            });
        }
    };
    let result = HostedConfig::from_env(backend).and_then(|mut config| {
        if let Some(model) = model {
            config.model = model;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        config.log_path = Some(format!("logs/brain-{stamp}.jsonl").into());
        HostedBrain::new(config, &sekiro_sim::brain::OWL)
    });
    Ok(fallback(backend, result))
}

fn fallback(backend: Backend, result: Result<HostedBrain, ConfigError>) -> Box<dyn BossBrain> {
    match result {
        Ok(brain) => Box::new(brain),
        Err(error) => {
            eprintln!("--brain {backend}: {error}; falling back to Lua AI");
            Box::new(LuaBrain)
        }
    }
}

/// Named plans shared by both backends, offered only when the table permits them.
fn combo(actions: &BossActions) -> Option<&'static str> {
    (actions.max_plan >= 3
        && [3000, 3001, 3002]
            .iter()
            .all(|&id| actions.get(id).is_some()))
    .then_some("3000+3001+3002")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_falls_back_without_opening_real_credential_sources() {
        for backend in [Backend::Decisions, Backend::Responses] {
            let brain = fallback(backend, Err(ConfigError::MissingKey));
            assert_eq!(brain.name(), "lua");
            assert!(!brain.plans());
        }
    }
}
