use std::fs;
use std::path::Path;

use crate::error::CoreError;

const HOOKS: &[&str] = &["prepare-commit-msg", "post-commit"];

/// Install engram git hooks into the repository's hooks directory.
///
/// For each hook, if an existing hook script is present, it is renamed
/// to `<hook>.pre-engram` and the new hook chains to it.
pub fn install_hooks(git_dir: &Path) -> Result<(), CoreError> {
    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    for hook_name in HOOKS {
        let hook_path = hooks_dir.join(hook_name);
        let backup_path = hooks_dir.join(format!("{hook_name}.pre-engram"));

        // If there's an existing hook (and it's not ours), back it up
        if hook_path.exists() {
            let content = fs::read_to_string(&hook_path)?;
            if !content.contains("engram hook-handler") {
                fs::rename(&hook_path, &backup_path)?;
            }
        }

        let script = generate_hook_script(hook_name, backup_path.exists());
        fs::write(&hook_path, script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))?;
        }
    }

    Ok(())
}

/// Uninstall engram hooks, restoring originals if they were backed up.
pub fn uninstall_hooks(git_dir: &Path) -> Result<(), CoreError> {
    let hooks_dir = git_dir.join("hooks");

    for hook_name in HOOKS {
        let hook_path = hooks_dir.join(hook_name);
        let backup_path = hooks_dir.join(format!("{hook_name}.pre-engram"));

        if hook_path.exists() {
            let content = fs::read_to_string(&hook_path).unwrap_or_default();
            if content.contains("engram hook-handler") {
                fs::remove_file(&hook_path)?;
            }
        }

        if backup_path.exists() {
            fs::rename(&backup_path, &hook_path)?;
        }
    }

    Ok(())
}

fn generate_hook_script(hook_name: &str, has_backup: bool) -> String {
    let mut script = String::from("#!/bin/sh\n");
    script.push_str("# Engram git hook — auto-generated, do not edit\n\n");

    // Chain to existing hook if backed up
    if has_backup {
        script.push_str(&format!(
            "# Run original hook\n\
             if [ -x \"$(dirname \"$0\")/{hook_name}.pre-engram\" ]; then\n\
             \t\"$(dirname \"$0\")/{hook_name}.pre-engram\" \"$@\"\n\
             \tHOOK_EXIT=$?\n\
             \tif [ $HOOK_EXIT -ne 0 ]; then\n\
             \t\texit $HOOK_EXIT\n\
             \tfi\n\
             fi\n\n"
        ));
    }

    // Run engram hook handler (fail silently — hooks should not break git)
    script.push_str(&format!(
        "# Run engram hook handler\n\
         if command -v engram >/dev/null 2>&1; then\n\
         \tengram hook-handler {hook_name} \"$@\" || true\n\
         fi\n"
    ));

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_install_hooks() {
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path();
        fs::create_dir_all(git_dir.join("hooks")).unwrap();

        install_hooks(git_dir).unwrap();

        for hook_name in HOOKS {
            let hook_path = git_dir.join("hooks").join(hook_name);
            assert!(hook_path.exists(), "Hook {hook_name} should exist");
            let content = fs::read_to_string(&hook_path).unwrap();
            assert!(content.contains("engram hook-handler"));
        }
    }

    #[test]
    fn test_install_chains_existing() {
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path();
        let hooks_dir = git_dir.join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();

        // Create an existing hook
        let existing = hooks_dir.join("prepare-commit-msg");
        fs::write(&existing, "#!/bin/sh\necho original\n").unwrap();

        install_hooks(git_dir).unwrap();

        // Original should be backed up
        let backup = hooks_dir.join("prepare-commit-msg.pre-engram");
        assert!(backup.exists());
        let backup_content = fs::read_to_string(&backup).unwrap();
        assert!(backup_content.contains("echo original"));

        // New hook should chain
        let new_content = fs::read_to_string(&existing).unwrap();
        assert!(new_content.contains("pre-engram"));
        assert!(new_content.contains("engram hook-handler"));
    }

    #[test]
    fn test_uninstall_hooks() {
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path();
        let hooks_dir = git_dir.join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();

        // Create existing hook, install, then uninstall
        let existing = hooks_dir.join("prepare-commit-msg");
        fs::write(&existing, "#!/bin/sh\necho original\n").unwrap();

        install_hooks(git_dir).unwrap();
        uninstall_hooks(git_dir).unwrap();

        // Original should be restored
        let content = fs::read_to_string(&existing).unwrap();
        assert!(content.contains("echo original"));
        assert!(!hooks_dir.join("prepare-commit-msg.pre-engram").exists());
    }
}
