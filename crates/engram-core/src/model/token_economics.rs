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
}
