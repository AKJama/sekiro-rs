//! Hosted boss choices. Only the worker performs HTTP; the simulation never leaves its thread.

mod config;
mod protocol;
mod worker;

pub use config::{ApiKey, Backend, ConfigError, HostedConfig};
pub use worker::{Failure, HostedBrain, HostedStats, StatsHandle, TokenUsage};

use sekiro_sim::brain::{BossActions, BossBrain, BrainKind, LuaBrain};

/// Whether `--brain <name>` is a hosted model (which remembers across sessions by default).
pub fn is_hosted(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "decisions" | "responses"
    )
}

/// Game CLI boundary for the enemy `chr` (the hosted brain offers that enemy's actions).
/// Configuration or worker startup failure leaves the Lua AI in control. Constructing a
/// controller does not send a request; only `submit` does.
pub fn from_cli(
    name: &str,
    model: Option<String>,
    chr: &str,
) -> Result<Box<dyn BossBrain>, String> {
    let backend = match name.to_ascii_lowercase().as_str() {
        "decisions" => Backend::Decisions,
        "responses" => Backend::Responses,
        _ => {
            return BrainKind::by_name(name)
                .map(|kind| kind.build(chr))
                .map_err(|_| {
                    "unknown brain: known are lua, scripted, decisions, responses".into()
                });
        }
    };
    let Some(actions) = BossActions::for_chr(chr) else {
        eprintln!("--brain {backend}: no action list for {chr}; falling back to Lua AI");
        return Ok(Box::new(LuaBrain));
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
        HostedBrain::new(config, actions)
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
