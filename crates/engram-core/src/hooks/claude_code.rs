use std::path::Path;

use crate::error::CoreError;

const ENGRAM_HOOK_MARKER: &str = "hook-handler session-end";

/// Install a Claude Code `SessionEnd` hook that auto-imports sessions.
///
/// Creates or updates `.claude/settings.json` in the project root.
/// Idempotent: will not duplicate the hook if already present.
pub fn install_claude_code_hook(project_root: &Path) -> Result<(), CoreError> {
    let claude_dir = project_root.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let settings_path = claude_dir.join("settings.json");

    // Read existing settings or start with empty object
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&content).map_err(|e| {
                CoreError::Config(format!("Failed to parse .claude/settings.json: {e}"))
            })?
        }
    } else {
        serde_json::json!({})
    };

    // Check if our hook already exists
    if let Some(hooks) = settings.get("hooks") {
        if let Some(session_end) = hooks.get("SessionEnd") {
            let content = serde_json::to_string(session_end).unwrap_or_default();
            if content.contains(ENGRAM_HOOK_MARKER) {
                return Ok(());
            }
        }
    }

    // Resolve the engram binary path
    let engram_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "engram".to_string());

    let hook_command = format!("{engram_path} hook-handler session-end");

    // Build the hook entry
    let hook_entry = serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": hook_command
            }
        ]
    });

    // Merge into existing settings
    let hooks_obj = settings
        .as_object_mut()
        .ok_or_else(|| CoreError::Config("settings.json is not a JSON object".into()))?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    let session_end_arr = hooks_obj
        .as_object_mut()
        .ok_or_else(|| CoreError::Config("hooks is not a JSON object".into()))?
        .entry("SessionEnd")
        .or_insert_with(|| serde_json::json!([]));

    session_end_arr
        .as_array_mut()
        .ok_or_else(|| CoreError::Config("SessionEnd is not an array".into()))?
        .push(hook_entry);

    // Write back with pretty formatting
    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| CoreError::Config(format!("Failed to serialize settings: {e}")))?;
    std::fs::write(&settings_path, output)?;

    Ok(())
}

/// Return whether the engram Claude Code `SessionEnd` hook is installed in
/// the project's `.claude/settings.json`.
pub fn claude_code_hook_installed(project_root: &Path) -> bool {
    let settings_path = project_root.join(".claude").join("settings.json");
    let Ok(content) = std::fs::read_to_string(&settings_path) else {
        return false;
    };
    content.contains(ENGRAM_HOOK_MARKER)
}

/// Remove the Claude Code `SessionEnd` hook for engram.
///
/// Preserves all other settings and hooks.
pub fn uninstall_claude_code_hook(project_root: &Path) -> Result<(), CoreError> {
    let settings_path = project_root.join(".claude").join("settings.json");
    if !settings_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&settings_path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| CoreError::Config(format!("Failed to parse settings: {e}")))?;

    if let Some(hooks) = settings.get_mut("hooks") {
        if let Some(session_end) = hooks.get_mut("SessionEnd") {
            if let Some(arr) = session_end.as_array_mut() {
                arr.retain(|entry| {
                    let s = serde_json::to_string(entry).unwrap_or_default();
                    !s.contains(ENGRAM_HOOK_MARKER)
                });
            }
        }
    }

    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| CoreError::Config(format!("Failed to serialize settings: {e}")))?;
    std::fs::write(&settings_path, output)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_install_creates_settings_file() {
        let dir = TempDir::new().unwrap();
        install_claude_code_hook(dir.path()).unwrap();

        let settings_path = dir.path().join(".claude/settings.json");
        assert!(settings_path.exists());

        let content = std::fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify structure
        let session_end = &settings["hooks"]["SessionEnd"];
        assert!(session_end.is_array());
        assert_eq!(session_end.as_array().unwrap().len(), 1);

        let hook = &session_end[0]["hooks"][0];
        assert_eq!(hook["type"], "command");
        let cmd = hook["command"].as_str().unwrap();
        assert!(cmd.contains("hook-handler session-end"));
    }

    #[test]
    fn test_install_preserves_existing_settings() {
        let dir = TempDir::new().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();

        // Write existing settings with other keys
        let existing = serde_json::json!({
            "permissions": { "allow": ["Read", "Glob"] },
            "hooks": {
                "PreToolUse": [{ "matcher": "Bash", "hooks": [] }]
            }
        });
        std::fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        install_claude_code_hook(dir.path()).unwrap();

        let content = std::fs::read_to_string(claude_dir.join("settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Original keys preserved
        assert!(settings["permissions"]["allow"].is_array());
        assert!(settings["hooks"]["PreToolUse"].is_array());

        // New hook added
        assert!(settings["hooks"]["SessionEnd"].is_array());
        assert_eq!(settings["hooks"]["SessionEnd"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_install_idempotent() {
        let dir = TempDir::new().unwrap();

        install_claude_code_hook(dir.path()).unwrap();
        install_claude_code_hook(dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Only one hook entry, not two
        assert_eq!(settings["hooks"]["SessionEnd"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_uninstall_removes_hook() {
        let dir = TempDir::new().unwrap();

        install_claude_code_hook(dir.path()).unwrap();
        uninstall_claude_code_hook(dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // SessionEnd array should be empty
        assert_eq!(settings["hooks"]["SessionEnd"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_uninstall_noop_when_no_file() {
        let dir = TempDir::new().unwrap();
        // Should not error
        uninstall_claude_code_hook(dir.path()).unwrap();
    }

    #[test]
    fn test_install_handles_empty_file() {
        let dir = TempDir::new().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.json"), "{}").unwrap();

        install_claude_code_hook(dir.path()).unwrap();

        let content = std::fs::read_to_string(claude_dir.join("settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(settings["hooks"]["SessionEnd"].as_array().unwrap().len(), 1);
    }
}
