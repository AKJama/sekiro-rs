use crate::{ConfigError, TokenUsage};

#[derive(Debug, Clone)]
struct Price {
    model: String,
    operation: String,
    input: f64,
    cached: f64,
    output: f64,
    reasoning: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct PriceTable {
    prices: Vec<Price>,
}

impl Default for PriceTable {
    fn default() -> Self {
        Self::parse(include_str!("../../../config/brain-prices.toml")).expect("bundled prices")
    }
}

impl PriceTable {
    /// An absent file uses the bundled table; a malformed file fails closed.
    pub fn load() -> Result<Self, ConfigError> {
        match std::fs::read_to_string("config/brain-prices.toml") {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(_) => Err(ConfigError::InvalidConfig),
        }
    }
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let doc = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(|_| ConfigError::InvalidConfig)?;
        // Billing classification comes from authentication, not an editable switch.
        for backend in ["codex", "claude"] {
            if doc["backends"][backend]["subscription"].as_bool() != Some(true) {
                return Err(ConfigError::InvalidConfig);
            }
        }
        let rows = doc["prices"]
            .as_array_of_tables()
            .ok_or(ConfigError::InvalidConfig)?;
        let mut prices = Vec::new();
        for row in rows {
            let number = |key: &str| -> Result<f64, ConfigError> {
                let n = row
                    .get(key)
                    .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|n| n as f64)))
                    .ok_or(ConfigError::InvalidConfig)?;
                if n.is_finite() && n >= 0.0 {
                    Ok(n)
                } else {
                    Err(ConfigError::InvalidConfig)
                }
            };
            let string = |key: &str| {
                row.get(key)
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .ok_or(ConfigError::InvalidConfig)
            };
            let p = Price {
                model: string("model")?,
                operation: string("operation")?,
                input: number("input")?,
                cached: number("cached_input")?,
                output: number("output")?,
                reasoning: row
                    .contains_key("reasoning")
                    .then(|| number("reasoning"))
                    .transpose()?,
            };
            if prices
                .iter()
                .any(|old: &Price| old.model == p.model && old.operation == p.operation)
            {
                return Err(ConfigError::InvalidConfig);
            }
            prices.push(p);
        }
        Ok(Self { prices })
    }
    pub fn estimate(&self, model: &str, operation: &str, u: TokenUsage) -> Option<f64> {
        let p = self
            .prices
            .iter()
            .find(|p| p.model == model && p.operation == operation)?;
        let cached = u.cached_input_tokens.min(u.input_tokens);
        let reasoning = u.reasoning_tokens.min(u.output_tokens);
        let output = match p.reasoning {
            Some(rate) => (u.output_tokens - reasoning) as f64 * p.output + reasoning as f64 * rate,
            None => u.output_tokens as f64 * p.output,
        };
        Some(
            ((u.input_tokens - cached) as f64 * p.input + cached as f64 * p.cached + output)
                / 1_000_000.0,
        )
    }
}
