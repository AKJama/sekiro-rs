use std::fmt;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Duration;

/// Never expose credentials through Debug, Display, errors or telemetry.
pub struct ApiKey(pub(crate) String);

impl ApiKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Decisions,
    Responses,
}

impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Self::Decisions => "decisions",
            Self::Responses => "responses",
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

pub struct HostedConfig {
    pub backend: Backend,
    pub model: String,
    pub base_url: String,
    pub timeout: Duration,
    pub log_path: Option<PathBuf>,
    pub api_key: ApiKey,
}

impl fmt::Debug for HostedConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Public configuration can accidentally contain the credential too.
        f.debug_struct("HostedConfig")
            .field("backend", &self.backend)
            .field("model", &self.redact(&self.model))
            .field("base_url", &self.redact(&self.base_url))
            .field("timeout", &self.timeout)
            .field(
                "log_path",
                &self
                    .log_path
                    .as_ref()
                    .map(|p| self.redact(&p.to_string_lossy())),
            )
            .field("api_key", &self.api_key)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    MissingKey,
    InvalidConfig,
    LogOpen,
    WorkerStart,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingKey => "OPENAI_API_KEY is missing; set it in the environment or working directory .env",
            Self::InvalidConfig => "invalid hosted configuration: use a nonempty model, HTTP(S) base URL, positive finite timeout, and a nonempty key without control characters",
            Self::LogOpen => "cannot open hosted brain JSONL log",
            Self::WorkerStart => "cannot start hosted brain worker",
        })
    }
}

impl std::error::Error for ConfigError {}

impl HostedConfig {
    pub fn new(backend: Backend, api_key: ApiKey) -> Self {
        Self {
            backend,
            model: "gpt-6-luna".into(),
            base_url: "https://api.openai.com".into(),
            timeout: Duration::from_millis(1500),
            log_path: None,
            api_key,
        }
    }

    /// Reads no other dotenv settings. Never includes file contents in an error.
    pub fn from_env(backend: Backend) -> Result<Self, ConfigError> {
        let key = read_key(std::env::var("OPENAI_API_KEY").ok(), || {
            std::fs::File::open(".env")
                .ok()
                .and_then(|file| key_from_dotenv(BufReader::new(file)))
        })?;
        Ok(Self::new(backend, ApiKey::new(key)))
    }

    pub(crate) fn redact(&self, value: &str) -> String {
        if self.api_key.0.is_empty() {
            value.to_owned()
        } else {
            value.replace(&self.api_key.0, "***")
        }
    }
}

fn read_key(
    env: Option<String>,
    dotenv: impl FnOnce() -> Option<String>,
) -> Result<String, ConfigError> {
    env.filter(|key| !key.trim().is_empty())
        .or_else(dotenv)
        .ok_or(ConfigError::MissingKey)
}

pub(crate) fn key_from_dotenv(reader: impl BufRead) -> Option<String> {
    for line in reader.lines() {
        let line = line.ok()?;
        if let Some(value) = line.trim().strip_prefix("OPENAI_API_KEY=") {
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            return (!value.is_empty()).then(|| value.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_sources_are_lazy_and_dotenv_reads_only_the_named_line() {
        let fake = "test-key-not-real";
        let key = read_key(Some(fake.into()), || {
            panic!("dotenv must not open with an environment key")
        })
        .unwrap();
        assert!(key == fake);
        for text in [
            "OTHER=ignored\nOPENAI_API_KEY=test-key-not-real\nOTHER=ignored\n",
            "OPENAI_API_KEY=\"test-key-not-real\"\n",
            "OPENAI_API_KEY='test-key-not-real'\n",
        ] {
            assert!(read_key(None, || key_from_dotenv(text.as_bytes())).unwrap() == fake);
        }
        let error = read_key(None, || key_from_dotenv(b"OTHER=ignored\n".as_slice())).unwrap_err();
        assert_eq!(error, ConfigError::MissingKey);
        assert!(!error.to_string().contains(fake));
        assert_eq!(key_from_dotenv(b"OPENAI_API_KEY=\"\"\n".as_slice()), None);
        assert_eq!(
            key_from_dotenv(b"# OPENAI_API_KEY=ignored\n".as_slice()),
            None
        );
    }
}
