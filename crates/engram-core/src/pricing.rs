use crate::model::TokenUsage;

/// Per-million-token pricing for a model.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cache_read_per_million: f64,
    pub cache_write_per_million: f64,
}

/// Known model pricing entries. Ordered most-specific first so `contains()` matching
/// picks the right variant (e.g. "gpt-4o" before "gpt-4").
const PRICING_TABLE: &[(&str, ModelPricing)] = &[
    // Anthropic Claude 4 / 4.5 / 4.6
    (
        "claude-opus-4",
        ModelPricing {
            input_per_million: 15.0,
            output_per_million: 75.0,
            cache_read_per_million: 1.50,
            cache_write_per_million: 18.75,
        },
    ),
    (
        "claude-sonnet-4",
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_read_per_million: 0.30,
            cache_write_per_million: 3.75,
        },
    ),
    // Anthropic Claude 3.5
    (
        "claude-3-5-sonnet",
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_read_per_million: 0.30,
            cache_write_per_million: 3.75,
        },
    ),
    (
        "claude-3-5-haiku",
        ModelPricing {
            input_per_million: 0.80,
            output_per_million: 4.0,
            cache_read_per_million: 0.08,
            cache_write_per_million: 1.0,
        },
    ),
    // Anthropic Claude 3
    (
        "claude-3-opus",
        ModelPricing {
            input_per_million: 15.0,
            output_per_million: 75.0,
            cache_read_per_million: 1.50,
            cache_write_per_million: 18.75,
        },
    ),
    (
        "claude-3-sonnet",
        ModelPricing {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_read_per_million: 0.30,
            cache_write_per_million: 3.75,
        },
    ),
    (
        "claude-3-haiku",
        ModelPricing {
            input_per_million: 0.25,
            output_per_million: 1.25,
            cache_read_per_million: 0.03,
            cache_write_per_million: 0.30,
        },
    ),
    // OpenAI — gpt-4o before gpt-4
    (
        "gpt-4o",
        ModelPricing {
            input_per_million: 2.50,
            output_per_million: 10.0,
            cache_read_per_million: 1.25,
            cache_write_per_million: 3.125,
        },
    ),
    (
        "gpt-4-turbo",
        ModelPricing {
            input_per_million: 10.0,
            output_per_million: 30.0,
            cache_read_per_million: 5.0,
            cache_write_per_million: 12.50,
        },
    ),
    (
        "gpt-4",
        ModelPricing {
            input_per_million: 30.0,
            output_per_million: 60.0,
            cache_read_per_million: 15.0,
            cache_write_per_million: 37.50,
        },
    ),
    (
        "gpt-3.5",
        ModelPricing {
            input_per_million: 0.50,
            output_per_million: 1.50,
            cache_read_per_million: 0.25,
            cache_write_per_million: 0.625,
        },
    ),
    // OpenAI o1/o3 reasoning models
    (
        "o3",
        ModelPricing {
            input_per_million: 2.0,
            output_per_million: 8.0,
            cache_read_per_million: 1.0,
            cache_write_per_million: 2.50,
        },
    ),
    (
        "o1",
        ModelPricing {
            input_per_million: 15.0,
            output_per_million: 60.0,
            cache_read_per_million: 7.50,
            cache_write_per_million: 18.75,
        },
    ),
];

/// Look up pricing for a model name. Normalizes to lowercase and uses `contains()` matching.
pub fn lookup_model(model: &str) -> Option<&'static ModelPricing> {
    let lower = model.to_lowercase();
    for (pattern, pricing) in PRICING_TABLE {
        if lower.contains(pattern) {
            return Some(pricing);
        }
    }
    None
}

/// Estimate cost from model name and token counts.
/// Returns `None` if the model is unrecognized.
pub fn estimate_cost(model: Option<&str>, usage: &TokenUsage) -> Option<f64> {
    let pricing = lookup_model(model?)?;

    // Standard (non-cached) input tokens = input_tokens - cache_read - cache_write
    let standard_input = usage
        .input_tokens
        .saturating_sub(usage.cache_read_tokens)
        .saturating_sub(usage.cache_write_tokens);

    let cost = (standard_input as f64 * pricing.input_per_million
        + usage.output_tokens as f64 * pricing.output_per_million
        + usage.cache_read_tokens as f64 * pricing.cache_read_per_million
        + usage.cache_write_tokens as f64 * pricing.cache_write_per_million)
        / 1_000_000.0;

    Some(cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TokenUsage;

    #[test]
    fn test_lookup_opus() {
        let p = lookup_model("claude-opus-4-6").unwrap();
        assert!((p.input_per_million - 15.0).abs() < 1e-10);
        assert!((p.output_per_million - 75.0).abs() < 1e-10);
    }

    #[test]
    fn test_lookup_sonnet() {
        let p = lookup_model("claude-sonnet-4-5-20250514").unwrap();
        assert!((p.input_per_million - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_lookup_case_insensitive() {
        let p = lookup_model("Claude-Sonnet-4-5").unwrap();
        assert!((p.input_per_million - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_lookup_gpt4o_before_gpt4() {
        let p = lookup_model("gpt-4o-2024-08-06").unwrap();
        assert!((p.input_per_million - 2.50).abs() < 1e-10);
    }

    #[test]
    fn test_lookup_gpt4_turbo() {
        let p = lookup_model("gpt-4-turbo-2024-04-09").unwrap();
        assert!((p.input_per_million - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_lookup_unknown_returns_none() {
        assert!(lookup_model("llama-3.1-70b").is_none());
    }

    #[test]
    fn test_estimate_cost_basic() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 1_100_000,
            cost_usd: None,
        };
        let cost = estimate_cost(Some("claude-sonnet-4-5"), &usage).unwrap();
        // 1M input * $3/M + 100K output * $15/M = $3 + $1.5 = $4.5
        assert!((cost - 4.5).abs() < 1e-10);
    }

    #[test]
    fn test_estimate_cost_with_cache() {
        let usage = TokenUsage {
            input_tokens: 100_000, // total input including cache
            output_tokens: 10_000,
            cache_read_tokens: 50_000,
            cache_write_tokens: 20_000,
            total_tokens: 110_000,
            cost_usd: None,
        };
        let cost = estimate_cost(Some("claude-sonnet-4-5"), &usage).unwrap();
        // standard_input = 100K - 50K - 20K = 30K
        // 30K * $3/M + 10K * $15/M + 50K * $0.30/M + 20K * $3.75/M
        // = 0.09 + 0.15 + 0.015 + 0.075 = 0.33
        assert!((cost - 0.33).abs() < 1e-10);
    }

    #[test]
    fn test_estimate_cost_no_model() {
        let usage = TokenUsage::default();
        assert!(estimate_cost(None, &usage).is_none());
    }

    #[test]
    fn test_estimate_cost_unknown_model() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
            ..Default::default()
        };
        assert!(estimate_cost(Some("llama-3.1-70b"), &usage).is_none());
    }

    #[test]
    fn test_estimate_cost_claude_35_sonnet() {
        let usage = TokenUsage {
            input_tokens: 500_000,
            output_tokens: 50_000,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 550_000,
            cost_usd: None,
        };
        let cost = estimate_cost(Some("claude-3-5-sonnet-20241022"), &usage).unwrap();
        // 500K * $3/M + 50K * $15/M = $1.5 + $0.75 = $2.25
        assert!((cost - 2.25).abs() < 1e-10);
    }
}
