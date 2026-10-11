//! Session-local accounting, independent of providers and persistent boss memory.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostTotals {
    pub calls: u64,
    /// Input includes cached input; output includes reasoning. Never add those subsets twice.
    pub input: u64,
    pub cached: u64,
    pub output: u64,
    pub reasoning: u64,
    pub billed_usd: f64,
    pub api_equivalent_usd: f64,
    pub unknown_prices: u64,
    pub unknown_billed: u64,
}

impl CostTotals {
    pub fn tokens(&self) -> u64 {
        self.input.saturating_add(self.output)
    }
    pub fn since(&self, old: &Self) -> Self {
        Self {
            calls: self.calls.saturating_sub(old.calls),
            input: self.input.saturating_sub(old.input),
            cached: self.cached.saturating_sub(old.cached),
            output: self.output.saturating_sub(old.output),
            reasoning: self.reasoning.saturating_sub(old.reasoning),
            billed_usd: (self.billed_usd - old.billed_usd).max(0.0),
            api_equivalent_usd: (self.api_equivalent_usd - old.api_equivalent_usd).max(0.0),
            unknown_prices: self.unknown_prices.saturating_sub(old.unknown_prices),
            unknown_billed: self.unknown_billed.saturating_sub(old.unknown_billed),
        }
    }
    pub fn line(&self, label: &str) -> String {
        format!(
            "{label}: {} calls · {} tok · {} billed · {} api-equiv",
            self.calls,
            self.tokens(),
            usd(self.billed_usd, self.unknown_billed),
            usd(self.api_equivalent_usd, self.unknown_prices)
        )
    }
}

pub fn usd(value: f64, unknown: u64) -> String {
    if unknown > 0 {
        format!("unknown (${value:.6} known)")
    } else {
        format!("${value:.6}")
    }
}

#[derive(Debug, Clone, Default)]
pub struct CostRow {
    pub kind: String,
    pub model: String,
    pub totals: CostTotals,
}

#[derive(Debug, Clone, Default)]
pub struct CostSnapshot {
    pub totals: CostTotals,
    pub rows: Vec<CostRow>,
    pub budget_notice: Option<String>,
}
