// The GTK side of keyboard shortcuts.
//
// `Keybindings` (in keybindings.rs) is our own table, with no GTK. This module
// turns it into GTK shortcuts, kept in one shared list. Every terminal listens
// to that list, so when `apply` changes it, all open terminals update at once.

use gtk::gio;
use gtk::prelude::*;

use crate::keybindings::{Action, Keybindings};

// Cloning a `GtkShortcuts` is cheap: every clone is a handle to the same list.
#[derive(Clone)]
pub struct GtkShortcuts {
    list: gio::ListStore,
}

impl GtkShortcuts {
    pub fn new(bindings: &Keybindings) -> Self {
        let shortcuts = Self {
            list: gio::ListStore::new::<gtk::Shortcut>(),
        };
        shortcuts.apply(bindings);
        shortcuts
    }

    // Replace all shortcuts with the ones in `bindings`.
    // Everything listening to the list picks up the change immediately.
    pub fn apply(&self, bindings: &Keybindings) {
        let mut new_shortcuts = Vec::new();

        for action in Action::ALL {
            let Some(keys) = bindings.keys_for(action) else {
                continue; // this action has no shortcut
            };
            let Some(gtk_keys) = to_gtk_keys(keys) else {
                eprintln!("Ignoring shortcut {keys:?} for {action:?}: unknown key name");
                continue;
            };
            new_shortcuts.push(gtk::Shortcut::new(
                gtk::ShortcutTrigger::parse_string(&gtk_keys),
                Some(gtk::NamedAction::new(gtk_action_name(action))),
            ));
        }

        // Swap the whole list in one go, so listeners get a single update.
        self.list.splice(0, self.list.n_items(), &new_shortcuts);
    }

    // The shared list, for a widget's shortcut controller to listen to.
    pub fn list(&self) -> &gio::ListStore {
        &self.list
    }
}

// The GTK action each of our actions triggers. These must match the actions
// defined elsewhere: "term.*" by each terminal (terminal/mod.rs), "win.*" by
// the window (window.rs). GTK looks for the name on the focused terminal and
// all its parents, up to the window, so terminal shortcuts reach both.
fn gtk_action_name(action: Action) -> &'static str {
    match action {
        Action::Copy => "term.copy",
        Action::Paste => "term.paste",
        Action::SplitRight => "win.split-right",
        Action::SplitDown => "win.split-down",
        Action::NewTab => "win.new-tab",
        Action::CloseTab => "win.close-tab",
    }
}

// Convert our key format into GTK's: "Ctrl+Shift+C" -> "<Control><Shift>c".
// Returns `None` if a modifier name is unknown.
fn to_gtk_keys(keys: &str) -> Option<String> {
    let mut parts: Vec<&str> = keys.split('+').collect();
    let key = parts.pop()?; // the last part is the key, the rest are modifiers

    let mut gtk_keys = String::new();
    for modifier in parts {
        let gtk_modifier = match modifier {
            "Ctrl" => "<Control>",
            "Shift" => "<Shift>",
            "Alt" => "<Alt>",
            "Cmd" => "<Meta>",
            "Super" => "<Super>",
            _ => return None,
        };
        gtk_keys.push_str(gtk_modifier);
    }

    // GTK wants single letters in lowercase ("c"), other keys as named ("F5").
    if key.len() == 1 {
        gtk_keys.push_str(&key.to_lowercase());
    } else {
        gtk_keys.push_str(key);
    }
    Some(gtk_keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_modifiers_and_letter() {
        assert_eq!(
            to_gtk_keys("Ctrl+Shift+C").as_deref(),
            Some("<Control><Shift>c")
        );
        assert_eq!(to_gtk_keys("Cmd+V").as_deref(), Some("<Meta>v"));
    }

    #[test]
    fn keeps_named_keys() {
        assert_eq!(to_gtk_keys("F5").as_deref(), Some("F5"));
    }

    #[test]
    fn rejects_unknown_modifier() {
        assert_eq!(to_gtk_keys("Hyper+C"), None);
    }
}
