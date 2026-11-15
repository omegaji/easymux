// Keybindings: which keys trigger which action.
//
// This module is plain Rust, with no GTK. It only holds the table, in our own
// readable key format like "Ctrl+Shift+C". A settings screen or config file
// can work with it directly. Turning it into real GTK shortcuts happens elsewhere.

use std::collections::HashMap;

// Every action that can have a keyboard shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Copy,
    Paste,
}

impl Action {
    // Every action, so other code can loop over all of them.
    // When you add an action above, add it here too.
    pub const ALL: [Action; 2] = [Action::Copy, Action::Paste];
}

// The table: for each action, the keys that trigger it.
#[derive(Debug, Clone)]
pub struct Keybindings {
    keys: HashMap<Action, String>,
}

impl Keybindings {
    // The default shortcuts.
    // Linux uses Ctrl+Shift, because plain Ctrl+C must reach the shell
    // (it stops the running program). macOS uses Cmd.
    pub fn defaults() -> Self {
        let (copy, paste) = if cfg!(target_os = "macos") {
            ("Cmd+C", "Cmd+V")
        } else {
            ("Ctrl+Shift+C", "Ctrl+Shift+V")
        };

        let mut keys = HashMap::new();
        keys.insert(Action::Copy, copy.to_string());
        keys.insert(Action::Paste, paste.to_string());
        Self { keys }
    }

    // The keys for one action, for example "Ctrl+Shift+C".
    // `None` means the action has no shortcut.
    pub fn keys_for(&self, action: Action) -> Option<&str> {
        self.keys.get(&action).map(|keys| keys.as_str())
    }

    // Change the keys for one action.
    pub fn set(&mut self, action: Action, keys: &str) {
        self.keys.insert(action, keys.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_copy_and_paste() {
        let bindings = Keybindings::defaults();
        assert!(bindings.keys_for(Action::Copy).is_some());
        assert!(bindings.keys_for(Action::Paste).is_some());
    }

    #[test]
    fn set_replaces_the_keys() {
        let mut bindings = Keybindings::defaults();
        bindings.set(Action::Copy, "F5");
        assert_eq!(bindings.keys_for(Action::Copy), Some("F5"));
    }
}
