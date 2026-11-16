// Shell integration: our small scripts that make shells tell easymux what
// they are doing (current folder, running command), whatever the user's
// shell config is.
//
// The scripts are bundled in the program (resources/shell-integration).
// Shells can only load real files, so at startup we write them out to a cache
// folder. Each shell is then started with a few environment variables that
// make it load our script first (see resources/shell-integration/zsh/.zshenv).

use std::path::{Path, PathBuf};

use gtk::{gio, glib};

// Where the scripts are inside our bundle.
const BUNDLE_FOLDER: &str = "/dev/easymux/Easymux/shell-integration";

// The scripts to write out, as paths relative to the folder.
const SCRIPTS: &[&str] = &["zsh/.zshenv", "zsh/easymux.zsh"];

// Cheap to clone: it only holds a path.
#[derive(Debug, Clone)]
pub struct ShellIntegration {
    // Where the scripts were written. `None` if writing them failed: shells
    // then start without integration (tab titles fall back to "Terminal").
    folder: Option<PathBuf>,
}

impl ShellIntegration {
    // Write the bundled scripts to easymux's cache folder.
    pub fn install() -> Self {
        let folder = glib::user_cache_dir()
            .join("easymux")
            .join("shell-integration");
        match write_scripts(&folder) {
            Ok(()) => Self {
                folder: Some(folder),
            },
            Err(err) => {
                eprintln!("Shell integration is off: could not write its scripts: {err}");
                Self { folder: None }
            }
        }
    }

    // The environment variables to start `shell` (e.g. "/bin/zsh") with.
    pub fn environment_for(&self, shell: &str) -> Vec<String> {
        let mut env = vec![
            "TERM_PROGRAM=easymux".to_string(),
            format!("TERM_PROGRAM_VERSION={}", env!("CARGO_PKG_VERSION")),
        ];
        if let Some(folder) = &self.folder {
            let user_zdotdir = std::env::var("ZDOTDIR").ok();
            env.extend(integration_environment(shell, folder, user_zdotdir));
        }
        env
    }
}

// The user's shell: $SHELL, or /bin/sh if it is not set.
pub fn user_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

// The variables that make `shell` load our scripts from `folder`.
// Shells we have no script for yet get none.
fn integration_environment(shell: &str, folder: &Path, user_zdotdir: Option<String>) -> Vec<String> {
    let mut env = Vec::new();
    if shell_name(shell) == "zsh" {
        // zsh loads its startup files from ZDOTDIR: point it at our folder.
        // Our .zshenv then restores the user's own ZDOTDIR (if they had one).
        env.push(format!("EASYMUX_SHELL_INTEGRATION_DIR={}", folder.display()));
        env.push(format!("ZDOTDIR={}", folder.join("zsh").display()));
        if let Some(original) = user_zdotdir {
            env.push(format!("EASYMUX_ZSH_ZDOTDIR={original}"));
        }
    }
    env
}

// "/bin/zsh" -> "zsh"
fn shell_name(shell: &str) -> &str {
    Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shell)
}

// Copy each bundled script to `folder`, creating sub-folders as needed.
fn write_scripts(folder: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for script in SCRIPTS {
        let contents = gio::resources_lookup_data(
            &format!("{BUNDLE_FOLDER}/{script}"),
            gio::ResourceLookupFlags::NONE,
        )?;
        let path = folder.join(script);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &*contents)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_name_is_the_last_part_of_the_path() {
        assert_eq!(shell_name("/bin/zsh"), "zsh");
        assert_eq!(shell_name("/opt/homebrew/bin/bash"), "bash");
        assert_eq!(shell_name("zsh"), "zsh");
    }

    #[test]
    fn zsh_gets_zdotdir_pointing_at_our_scripts() {
        let env = integration_environment("/bin/zsh", Path::new("/cache/si"), None);
        assert!(env.contains(&"ZDOTDIR=/cache/si/zsh".to_string()));
        assert!(env.contains(&"EASYMUX_SHELL_INTEGRATION_DIR=/cache/si".to_string()));
        // The user had no ZDOTDIR, so there is nothing to restore.
        assert!(!env.iter().any(|var| var.starts_with("EASYMUX_ZSH_ZDOTDIR=")));
    }

    #[test]
    fn zsh_remembers_the_users_own_zdotdir() {
        let env = integration_environment(
            "/bin/zsh",
            Path::new("/cache/si"),
            Some("/Users/om/.config/zsh".to_string()),
        );
        assert!(env.contains(&"EASYMUX_ZSH_ZDOTDIR=/Users/om/.config/zsh".to_string()));
    }

    #[test]
    fn shells_without_a_script_get_nothing() {
        let env = integration_environment("/bin/bash", Path::new("/cache/si"), None);
        assert!(env.is_empty());
    }
}
