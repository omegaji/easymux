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
    SplitRight,
    SplitDown,
    NewTab,
    CloseTab,
}

impl Action {
    // Every action, so other code can loop over all of them.
    // When you add an action above, add it here too.
    pub const ALL: [Action; 6] = [
        Action::Copy,
        Action::Paste,
        Action::SplitRight,
        Action::SplitDown,
        Action::NewTab,
        Action::CloseTab,
    ];
}

// The default shortcuts, the same as Ghostty's (so they feel familiar to
// cmux users). macOS uses Cmd. Linux uses Ctrl+Shift, because plain Ctrl+key
// combinations belong to the programs in the terminal (Ctrl+C stops the
// running program, Ctrl+D ends the shell, Ctrl+W deletes a word, ...).
#[cfg(target_os = "macos")]
const DEFAULTS: &[(Action, &str)] = &[
    (Action::Copy, "Cmd+C"),
    (Action::Paste, "Cmd+V"),
    (Action::SplitRight, "Cmd+D"),
    (Action::SplitDown, "Cmd+Shift+D"),
    (Action::NewTab, "Cmd+T"),
    (Action::CloseTab, "Cmd+W"),
];

#[cfg(not(target_os = "macos"))]
const DEFAULTS: &[(Action, &str)] = &[
    (Action::Copy, "Ctrl+Shift+C"),
    (Action::Paste, "Ctrl+Shift+V"),
    (Action::SplitRight, "Ctrl+Shift+O"),
    (Action::SplitDown, "Ctrl+Shift+E"),
    (Action::NewTab, "Ctrl+Shift+T"),
    (Action::CloseTab, "Ctrl+Shift+W"),
];

// The table: for each action, the keys that trigger it.
#[derive(Debug, Clone)]
pub struct Keybindings {
    keys: HashMap<Action, String>,
}

impl Keybindings {
    // The default shortcuts for this system (see `DEFAULTS` above).
    pub fn defaults() -> Self {
        let keys = DEFAULTS
            .iter()
            .map(|(action, keys)| (*action, keys.to_string()))
            .collect();
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
    fn every_action_has_a_default() {
        let bindings = Keybindings::defaults();
        for action in Action::ALL {
            assert!(bindings.keys_for(action).is_some(), "{action:?} has no default");
        }
    }

    #[test]
    fn no_two_actions_share_the_same_default_keys() {
        let bindings = Keybindings::defaults();
        let mut seen = std::collections::HashSet::new();
        for action in Action::ALL {
            let keys = bindings.keys_for(action).unwrap();
            assert!(seen.insert(keys), "{keys} is used twice");
        }
    }

    #[test]
    fn set_replaces_the_keys() {
        let mut bindings = Keybindings::defaults();
        bindings.set(Action::Copy, "F5");
        assert_eq!(bindings.keys_for(Action::Copy), Some("F5"));
    }
}
