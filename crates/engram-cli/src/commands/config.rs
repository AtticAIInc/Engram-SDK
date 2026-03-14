use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use engram_core::config::GlobalConfig;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a configuration value
    Set {
        /// Config key (e.g. anthropic_api_key, summarize_model)
        key: String,
        /// Value to set
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Config key (e.g. anthropic_api_key, summarize_model)
        key: String,
    },
    /// List all configuration values
    List,
    /// Show the path to the global config file
    Path,
}

pub fn run(args: &ConfigArgs) -> Result<()> {
    match &args.action {
        ConfigAction::Set { key, value } => {
            let mut config = GlobalConfig::load().context("Failed to load global config")?;
            config
                .set(key, value)
                .context("Failed to set config value")?;
            config.save().context("Failed to save global config")?;

            let display_value = if key.contains("api_key") {
                mask_key(value)
            } else {
                value.clone()
            };
            println!("Set {key} = {display_value}");
            Ok(())
        }
        ConfigAction::Get { key } => {
            let config = GlobalConfig::load().context("Failed to load global config")?;
            match config.get(key) {
                Some(value) => {
                    let display = if key.contains("api_key") {
                        mask_key(&value)
                    } else {
                        value
                    };
                    println!("{display}");
                }
                None => {
                    println!("(not set)");
                }
            }
            Ok(())
        }
        ConfigAction::List => {
            let config = GlobalConfig::load().context("Failed to load global config")?;
            let entries = config.list();

            if entries.iter().all(|(_, v)| v.is_none()) {
                println!("No configuration values set.");
                println!();
                println!("Available keys:");
                println!("  anthropic_api_key      Anthropic API key for LLM summarization");
                println!(
                    "  summarize_model        Model override (default: claude-haiku-4-5-20251001)"
                );
                println!("  update_check_enabled   Check for new versions (default: true)");
                return Ok(());
            }

            for (key, value) in &entries {
                let display = match value {
                    Some(v) if key.contains("api_key") => mask_key(v),
                    Some(v) => v.clone(),
                    None => "(not set)".to_string(),
                };
                println!("{key} = {display}");
            }
            Ok(())
        }
        ConfigAction::Path => {
            match GlobalConfig::config_path() {
                Some(path) => println!("{}", path.display()),
                None => println!("(could not determine config path)"),
            }
            Ok(())
        }
    }
}

/// Mask an API key for display: show first 4 and last 4 chars.
fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 10 {
        "****".to_string()
    } else {
        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{prefix}...{suffix}")
    }
}
