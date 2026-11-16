// Commands: every change to a workspace, as a message.
//
// Buttons, shortcuts and terminals don't change the workspace themselves.
// They send a `Command` into a channel. One loop receives the commands and
// applies them one at a time, so two changes can never overlap.

use crate::layout::{PaneId, SplitDirection};
use crate::pane::TabId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    // Split the focused pane; the new pane goes to the right.
    SplitRight,
    // Split the focused pane; the new pane goes below.
    SplitDown,
    // Split this pane (for example from its own tab bar's split button).
    Split(PaneId, SplitDirection),
    // Open a new tab in the focused pane.
    NewTab,
    // Open a new tab in this pane (for example from its tab bar's + button).
    NewTabIn(PaneId),
    // Close the focused pane's active tab.
    CloseFocusedTab,
    // Show this tab (for example the user clicked it in the tab bar).
    SelectTab(TabId),
    // Close this tab (for example the user clicked its ✕).
    CloseTab(TabId),
    // A tab's terminal received keyboard focus (sent by the terminal itself).
    TabFocused(TabId),
    // The shell in a tab exited, for example the user typed "exit"
    // (sent by the terminal itself).
    TabExited(TabId),
}

// The sending end of the command channel. Cheap to clone: every clone
// sends into the same channel.
pub type CommandSender = async_channel::Sender<Command>;
