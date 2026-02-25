use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_write_tokens: u64,
    pub total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

impl TokenUsage {
    /// Returns `cost_usd` if already set, otherwise estimates from model pricing.
    pub fn effective_cost(&self, model: Option<&str>) -> Option<f64> {
        self.cost_usd
            .or_else(|| crate::pricing::estimate_cost(model, self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
        assert!(usage.cost_usd.is_none());
    }

    #[test]
    fn test_serde_roundtrip() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_write_tokens: 100,
            total_tokens: 1800,
            cost_usd: Some(0.23),
        };
        let json = serde_json::to_string(&usage).unwrap();
        let parsed: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage, parsed);
    }

    #[test]
    fn test_cost_usd_omitted_when_none() {
        let usage = TokenUsage::default();
        let json = serde_json::to_string(&usage).unwrap();
        assert!(!json.contains("cost_usd"));
    }

    #[test]
    fn test_effective_cost_uses_explicit_when_set() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
            cost_usd: Some(0.42),
            ..Default::default()
        };
        // Should use the explicit cost, not estimate
        let cost = usage.effective_cost(Some("claude-sonnet-4-5")).unwrap();
        assert!((cost - 0.42).abs() < 1e-10);
    }

    #[test]
    fn test_effective_cost_estimates_when_none() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            total_tokens: 1_100_000,
            cost_usd: None,
            ..Default::default()
        };
        let cost = usage.effective_cost(Some("claude-sonnet-4-5")).unwrap();
        // 1M * $3/M + 100K * $15/M = $3 + $1.5 = $4.5
        assert!((cost - 4.5).abs() < 1e-10);
    }

    #[test]
    fn test_effective_cost_none_for_unknown_model() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
            cost_usd: None,
            ..Default::default()
        };
        assert!(usage.effective_cost(Some("llama-70b")).is_none());
    }
}
