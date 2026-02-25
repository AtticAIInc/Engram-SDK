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
    /// All automation features are ON by default — opt out with CLI flags.
    pub fn default_init() -> Self {
        Self {
            enabled: true,
            auto_capture: true,
            default_agent: None,
            push_on_push: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_repo_config() -> (TempDir, Config) {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let repo = git2::Repository::open(tmp.path()).unwrap();
        let config = repo.config().unwrap();
        (tmp, config)
    }

    #[test]
    fn test_load_defaults_when_empty() {
        let (_tmp, config) = make_repo_config();
        let engram_config = EngramConfig::load(&config).unwrap();
        assert!(!engram_config.enabled);
        assert!(!engram_config.auto_capture);
        assert!(!engram_config.push_on_push);
        assert!(engram_config.default_agent.is_none());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let (_tmp, mut config) = make_repo_config();
        let original = EngramConfig {
            enabled: true,
            auto_capture: true,
            default_agent: Some("claude-code".into()),
            push_on_push: true,
        };
        original.save(&mut config).unwrap();

        let loaded = EngramConfig::load(&config).unwrap();
        assert!(loaded.enabled);
        assert!(loaded.auto_capture);
        assert!(loaded.push_on_push);
        assert_eq!(loaded.default_agent, Some("claude-code".into()));
    }

    #[test]
    fn test_save_without_agent() {
        let (_tmp, mut config) = make_repo_config();
        let original = EngramConfig {
            enabled: true,
            auto_capture: false,
            default_agent: None,
            push_on_push: false,
        };
        original.save(&mut config).unwrap();

        let loaded = EngramConfig::load(&config).unwrap();
        assert!(loaded.enabled);
        assert!(!loaded.auto_capture);
        assert!(!loaded.push_on_push);
        // default_agent stays None when not set
        assert!(loaded.default_agent.is_none());
    }

    #[test]
    fn test_default_init() {
        let config = EngramConfig::default_init();
        assert!(config.enabled);
        assert!(config.auto_capture);
        assert!(config.push_on_push);
        assert!(config.default_agent.is_none());
    }

    #[test]
    fn test_overwrite_config() {
        let (_tmp, mut config) = make_repo_config();

        // Save initial config
        let first = EngramConfig {
            enabled: true,
            auto_capture: false,
            default_agent: None,
            push_on_push: false,
        };
        first.save(&mut config).unwrap();

        // Overwrite with new config
        let second = EngramConfig {
            enabled: true,
            auto_capture: true,
            default_agent: Some("aider".into()),
            push_on_push: true,
        };
        second.save(&mut config).unwrap();

        let loaded = EngramConfig::load(&config).unwrap();
        assert!(loaded.auto_capture);
        assert!(loaded.push_on_push);
        assert_eq!(loaded.default_agent, Some("aider".into()));
    }
}
