// Shell integration: our small scripts that make shells tell easymux what
// they are doing (current folder, running command), whatever the user's
// shell config is.
//
// The scripts are bundled in the program (resources/shell-integration).
// Shells can only load real files, so at startup we write them out to a cache
// folder. Each shell is then started so that it loads our script:
//   - zsh:  ZDOTDIR points at our folder (see shell-integration/zsh/.zshenv)
//   - bash: `bash --rcfile <our script>` (see shell-integration/bash/easymux.bash)

use std::path::{Path, PathBuf};

use gtk::{gio, glib};

// Where the scripts are inside our bundle.
const BUNDLE_FOLDER: &str = "/dev/easymux/Easymux/shell-integration";

// The scripts to write out, as paths relative to the folder.
const SCRIPTS: &[&str] = &["zsh/.zshenv", "zsh/easymux.zsh", "bash/easymux.bash"];

// What to start for a shell: the command with its arguments, and the
// environment variables to add.
#[derive(Debug, PartialEq, Eq)]
pub struct ShellCommand {
    pub arguments: Vec<String>,
    pub environment: Vec<String>,
}

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

    // How to start `shell` (e.g. "/bin/zsh") with our integration.
    pub fn command_for(&self, shell: &str) -> ShellCommand {
        let user_zdotdir = std::env::var("ZDOTDIR").ok();
        shell_command(shell, self.folder.as_deref(), user_zdotdir, LOGIN_SHELLS)
    }
}

// Whether shells start as "login" shells, which load the user's profile files
// (~/.zprofile, ~/.bash_profile, ...: often where PATH is set up, e.g. for
// Homebrew). On macOS, apps opened from the Dock don't get that setup, so
// every Mac terminal starts login shells. On Linux the desktop session already
// did it once, so Linux terminals start normal (non-login) shells.
const LOGIN_SHELLS: bool = cfg!(target_os = "macos");

// The user's shell: $SHELL, or /bin/sh if it is not set.
pub fn user_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

// How to start `shell`, loading our scripts from `folder` (if we have them),
// as a login shell if `login` is true. Shells we have no script for are
// started as they are (with `-l` for login, which most shells understand).
fn shell_command(
    shell: &str,
    folder: Option<&Path>,
    user_zdotdir: Option<String>,
    login: bool,
) -> ShellCommand {
    let mut arguments = vec![shell.to_string()];
    let mut environment = vec![
        "TERM_PROGRAM=easymux".to_string(),
        format!("TERM_PROGRAM_VERSION={}", env!("CARGO_PKG_VERSION")),
    ];

    let bash_with_our_script = folder.is_some() && shell_name(shell) == "bash";
    if login {
        if bash_with_our_script {
            // `--rcfile` only works for non-login bash, so bash stays
            // non-login and our script loads the login files itself.
            environment.push("EASYMUX_BASH_LOGIN=1".to_string());
        } else {
            arguments.push("-l".to_string());
        }
    }

    if let Some(folder) = folder {
        match shell_name(shell) {
            "zsh" => {
                // zsh loads its startup files from ZDOTDIR: point it at our
                // folder. Our .zshenv restores the user's own ZDOTDIR.
                environment.push(format!("EASYMUX_SHELL_INTEGRATION_DIR={}", folder.display()));
                environment.push(format!("ZDOTDIR={}", folder.join("zsh").display()));
                if let Some(original) = user_zdotdir {
                    environment.push(format!("EASYMUX_ZSH_ZDOTDIR={original}"));
                }
            }
            "bash" => {
                // bash loads our script instead of ~/.bashrc; the script
                // loads ~/.bashrc itself, then adds our hooks.
                arguments.push("--rcfile".to_string());
                arguments.push(folder.join("bash").join("easymux.bash").display().to_string());
            }
            _ => {} // no script for this shell (yet)
        }
    }

    ShellCommand {
        arguments,
        environment,
    }
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

    const FOLDER: &str = "/cache/si";

    // A non-login shell (as on Linux).
    fn command(shell: &str, user_zdotdir: Option<&str>) -> ShellCommand {
        shell_command(
            shell,
            Some(Path::new(FOLDER)),
            user_zdotdir.map(str::to_string),
            false,
        )
    }

    // A login shell (as on macOS).
    fn login_command(shell: &str) -> ShellCommand {
        shell_command(shell, Some(Path::new(FOLDER)), None, true)
    }

    #[test]
    fn shell_name_is_the_last_part_of_the_path() {
        assert_eq!(shell_name("/bin/zsh"), "zsh");
        assert_eq!(shell_name("/opt/homebrew/bin/bash"), "bash");
        assert_eq!(shell_name("zsh"), "zsh");
    }

    #[test]
    fn every_shell_gets_term_program() {
        for shell in ["/bin/zsh", "/bin/bash", "/usr/bin/fish"] {
            let env = command(shell, None).environment;
            assert!(env.contains(&"TERM_PROGRAM=easymux".to_string()), "{shell}");
        }
    }

    #[test]
    fn zsh_gets_zdotdir_pointing_at_our_scripts() {
        let zsh = command("/bin/zsh", None);
        assert_eq!(zsh.arguments, vec!["/bin/zsh"]);
        assert!(zsh.environment.contains(&"ZDOTDIR=/cache/si/zsh".to_string()));
        assert!(zsh.environment.contains(&"EASYMUX_SHELL_INTEGRATION_DIR=/cache/si".to_string()));
        // The user had no ZDOTDIR, so there is nothing to restore.
        assert!(!zsh.environment.iter().any(|var| var.starts_with("EASYMUX_ZSH_ZDOTDIR=")));
    }

    #[test]
    fn zsh_remembers_the_users_own_zdotdir() {
        let zsh = command("/bin/zsh", Some("/Users/om/.config/zsh"));
        assert!(zsh.environment.contains(&"EASYMUX_ZSH_ZDOTDIR=/Users/om/.config/zsh".to_string()));
    }

    #[test]
    fn bash_loads_our_script_with_rcfile() {
        let bash = command("/bin/bash", None);
        assert_eq!(
            bash.arguments,
            vec!["/bin/bash", "--rcfile", "/cache/si/bash/easymux.bash"]
        );
    }

    #[test]
    fn other_shells_start_as_they_are() {
        assert_eq!(command("/usr/bin/fish", None).arguments, vec!["/usr/bin/fish"]);
    }

    #[test]
    fn without_scripts_every_shell_starts_as_it_is() {
        let bash = shell_command("/bin/bash", None, None, false);
        assert_eq!(bash.arguments, vec!["/bin/bash"]);
        assert!(bash.environment.contains(&"TERM_PROGRAM=easymux".to_string()));
    }

    #[test]
    fn non_login_shells_get_no_login_settings() {
        for shell in ["/bin/zsh", "/bin/bash", "/usr/bin/fish"] {
            let cmd = command(shell, None);
            assert!(!cmd.arguments.contains(&"-l".to_string()), "{shell}");
            assert!(!cmd.environment.contains(&"EASYMUX_BASH_LOGIN=1".to_string()), "{shell}");
        }
    }

    #[test]
    fn login_zsh_gets_dash_l() {
        assert_eq!(login_command("/bin/zsh").arguments, vec!["/bin/zsh", "-l"]);
    }

    #[test]
    fn login_bash_stays_non_login_and_our_script_loads_the_profile() {
        let bash = login_command("/bin/bash");
        // No -l: --rcfile only works for non-login bash.
        assert_eq!(
            bash.arguments,
            vec!["/bin/bash", "--rcfile", "/cache/si/bash/easymux.bash"]
        );
        assert!(bash.environment.contains(&"EASYMUX_BASH_LOGIN=1".to_string()));
    }

    #[test]
    fn login_without_scripts_uses_dash_l() {
        let bash = shell_command("/bin/bash", None, None, true);
        assert_eq!(bash.arguments, vec!["/bin/bash", "-l"]);
    }
}
