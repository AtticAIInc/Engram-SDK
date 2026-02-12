use git2::Config;

use crate::error::CoreError;

#[derive(Debug, Clone)]
pub struct EngramConfig {
    pub enabled: bool,
    pub auto_capture: bool,
    pub default_agent: Option<String>,
    pub push_on_push: bool,
}

impl EngramConfig {
    /// Read config from the repo's .git/config [engram] section.
    pub fn load(config: &Config) -> Result<Self, CoreError> {
        Ok(Self {
            enabled: config.get_bool("engram.enabled").unwrap_or(false),
            auto_capture: config.get_bool("engram.autoCapture").unwrap_or(false),
            default_agent: config.get_string("engram.defaultAgent").ok(),
            push_on_push: config.get_bool("engram.pushOnPush").unwrap_or(false),
        })
    }

    /// Write config to the repo's .git/config [engram] section.
    pub fn save(&self, config: &mut Config) -> Result<(), CoreError> {
        config
            .set_bool("engram.enabled", self.enabled)
            .map_err(CoreError::Git)?;
        config
            .set_bool("engram.autoCapture", self.auto_capture)
            .map_err(CoreError::Git)?;
        if let Some(agent) = &self.default_agent {
            config
                .set_str("engram.defaultAgent", agent)
                .map_err(CoreError::Git)?;
        }
        config
            .set_bool("engram.pushOnPush", self.push_on_push)
            .map_err(CoreError::Git)?;
        Ok(())
    }

    /// Default config for `engram init`.
    pub fn default_init() -> Self {
        Self {
            enabled: true,
            auto_capture: false,
            default_agent: None,
            push_on_push: false,
        }
    }
}
