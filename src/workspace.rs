// One workspace: panes arranged by a split layout, each pane holding tabs.
//
// - `Layout` (layout.rs) arranges the panes as a tree.
// - Each `Pane` (pane.rs) lists its tabs and which one is shown.
// - Each tab is one `Terminal`.
//
// When the layout changes (split, close pane) we rebuild the widgets from the
// tree: a pane becomes its `PaneView`, a split becomes a `gtk::Paned`.
// Switching tabs only changes which terminal a pane's stack shows.

use std::collections::HashMap;

use gtk::prelude::*;

use crate::commands::{Command, CommandSender};
use crate::layout::{CloseResult, Layout, Node, PaneId, SplitDirection};
use crate::pane::{Pane, TabCloseResult, TabId};
use crate::pane_view::PaneView;
use crate::settings::TerminalSettings;
use crate::shortcuts::GtkShortcuts;
use crate::terminal::Terminal;
use crate::theme::Theme;

// What the window should do after a command.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    // Keep going.
    Continue,
    // The last tab of the last pane was closed: the workspace is empty.
    Empty,
}

pub struct Workspace {
    layout: Layout,
    // Each pane's tabs, and its widgets.
    panes: HashMap<PaneId, Pane>,
    views: HashMap<PaneId, PaneView>,
    // Each tab's terminal.
    terminals: HashMap<TabId, Terminal>,
    // The number for the next new tab.
    next_tab: u32,
    // The one widget the window shows. Its content is replaced on every rebuild.
    root: gtk::Box,
    // The pane that has (or last had) keyboard focus. Commands like
    // "split" and "new tab" act on it.
    focused: PaneId,
    // For new terminals, so they can report "I got focus" and "I exited".
    commands: CommandSender,
    // What every new terminal needs.
    shortcuts: GtkShortcuts,
    settings: TerminalSettings,
    theme: Theme,
}

impl Workspace {
    // A workspace with one pane, holding one tab.
    pub fn new(
        shortcuts: &GtkShortcuts,
        settings: TerminalSettings,
        theme: Theme,
        commands: CommandSender,
    ) -> Self {
        let layout = Layout::new();
        let first_pane = layout.panes()[0];

        let mut workspace = Self {
            layout,
            panes: HashMap::new(),
            views: HashMap::new(),
            terminals: HashMap::new(),
            next_tab: 0,
            root: gtk::Box::new(gtk::Orientation::Vertical, 0),
            focused: first_pane,
            commands,
            shortcuts: shortcuts.clone(),
            settings,
            theme,
        };

        workspace.add_pane(first_pane);
        workspace.rebuild();
        workspace
    }

    // The widget to place inside a window.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    // Apply one command. This is the only way to change a workspace from
    // outside, so changes always happen one at a time.
    pub fn handle(&mut self, command: Command) -> Outcome {
        match command {
            Command::SplitRight => self.split(self.focused, SplitDirection::Right),
            Command::SplitDown => self.split(self.focused, SplitDirection::Down),
            Command::Split(pane, direction) => self.split(pane, direction),
            Command::NewTab => self.new_tab(self.focused),
            Command::NewTabIn(pane) => {
                self.focused = pane;
                self.new_tab(pane);
            }
            Command::CloseFocusedTab => {
                if let Some(pane) = self.panes.get(&self.focused) {
                    return self.close_tab(pane.active());
                }
            }
            Command::CloseTab(tab) | Command::TabExited(tab) => return self.close_tab(tab),
            Command::SelectTab(tab) => self.select_tab(tab),
            Command::TabFocused(tab) => {
                if let Some(pane) = self.pane_of(tab) {
                    self.focused = pane;
                }
            }
        }
        Outcome::Continue
    }

    // Split `pane` in two. The new pane (with one new tab) goes to the
    // right or below, and gets the focus.
    fn split(&mut self, pane: PaneId, direction: SplitDirection) {
        let Some(new_pane) = self.layout.split(pane, direction) else {
            return; // `pane` is not in the layout
        };
        self.add_pane(new_pane);
        self.focused = new_pane;
        self.rebuild();
    }

    // Open a new tab in `pane`, right after its active tab, and show it.
    fn new_tab(&mut self, pane: PaneId) {
        let tab = self.add_terminal(pane);
        if let Some(pane_tabs) = self.panes.get_mut(&pane) {
            pane_tabs.add(tab);
        }
        self.refresh_tab_bar(pane);
        self.show_active_tab(pane);
    }

    // Show `tab` in its pane, and focus that pane.
    fn select_tab(&mut self, tab: TabId) {
        let Some(pane) = self.pane_of(tab) else {
            return;
        };
        if let Some(pane_tabs) = self.panes.get_mut(&pane) {
            pane_tabs.activate(tab);
        }
        self.focused = pane;
        self.refresh_tab_bar(pane);
        self.show_active_tab(pane);
    }

    // Close `tab`. If it was its pane's last tab, the pane closes too.
    fn close_tab(&mut self, tab: TabId) -> Outcome {
        // Not found: e.g. the tab was already closed by its ✕, and its shell
        // reported "exited" afterwards.
        let Some(pane) = self.pane_of(tab) else {
            return Outcome::Continue;
        };
        let Some(pane_tabs) = self.panes.get_mut(&pane) else {
            return Outcome::Continue;
        };

        match pane_tabs.close(tab) {
            TabCloseResult::Closed => {
                self.remove_terminal(pane, tab);
                self.refresh_tab_bar(pane);
                self.show_active_tab(pane);
                Outcome::Continue
            }
            TabCloseResult::LastTab => self.close_pane(pane),
            TabCloseResult::NotFound => Outcome::Continue,
        }
    }

    // Close `pane` with all its tabs. Its sibling takes the space.
    // The pane before it (or after it, if it was the first) gets the focus.
    fn close_pane(&mut self, pane: PaneId) -> Outcome {
        let panes_before = self.layout.panes();
        match self.layout.close(pane) {
            CloseResult::Closed => {}
            CloseResult::LastPane => return Outcome::Empty,
            CloseResult::NotFound => return Outcome::Continue,
        }

        // Drop the pane's terminals and widgets.
        if let Some(pane_tabs) = self.panes.remove(&pane) {
            for tab in pane_tabs.tabs() {
                self.terminals.remove(tab);
            }
        }
        self.views.remove(&pane);

        if self.focused == pane {
            let index = panes_before.iter().position(|p| *p == pane).unwrap_or(0);
            let neighbor = if index > 0 {
                panes_before[index - 1]
            } else {
                panes_before[1]
            };
            self.focused = neighbor;
        }
        self.rebuild();
        Outcome::Continue
    }

    // Create the tabs and widgets for a new pane, with one new tab.
    fn add_pane(&mut self, pane: PaneId) {
        self.views
            .insert(pane, PaneView::new(pane, self.commands.clone()));

        let tab = self.add_terminal(pane);
        self.panes.insert(pane, Pane::new(tab));
        self.refresh_tab_bar(pane);
        self.show_active_tab(pane);
    }

    // Create a terminal for a new tab in `pane`, and put it in the pane's stack.
    fn add_terminal(&mut self, pane: PaneId) -> TabId {
        let tab = TabId(self.next_tab);
        self.next_tab += 1;

        let terminal = self.new_terminal(tab);
        if let Some(view) = self.views.get(&pane) {
            view.add_terminal(terminal.widget());
        }
        self.terminals.insert(tab, terminal);
        tab
    }

    // Drop `tab`'s terminal and take it out of `pane`'s stack.
    fn remove_terminal(&mut self, pane: PaneId, tab: TabId) {
        let Some(terminal) = self.terminals.remove(&tab) else {
            return;
        };
        if let Some(view) = self.views.get(&pane) {
            view.remove_terminal(terminal.widget());
        }
    }

    // Redraw `pane`'s tab bar from its tab list.
    // (Titles are a placeholder for now; real titles come next.)
    fn refresh_tab_bar(&self, pane: PaneId) {
        let (Some(pane_tabs), Some(view)) = (self.panes.get(&pane), self.views.get(&pane)) else {
            return;
        };
        let tabs: Vec<(TabId, String)> = pane_tabs
            .tabs()
            .iter()
            .map(|tab| (*tab, "Terminal".to_string()))
            .collect();
        view.update_tabs(&tabs, pane_tabs.active());
    }

    // Show `pane`'s active tab. If `pane` is the focused pane, also give its
    // terminal the keyboard focus.
    fn show_active_tab(&self, pane: PaneId) {
        let (Some(pane_tabs), Some(view)) = (self.panes.get(&pane), self.views.get(&pane)) else {
            return;
        };
        let Some(terminal) = self.terminals.get(&pane_tabs.active()) else {
            return;
        };
        view.show_terminal(terminal.widget());
        if pane == self.focused {
            terminal.grab_focus();
        }
    }

    // The pane that holds `tab`, if any.
    fn pane_of(&self, tab: TabId) -> Option<PaneId> {
        self.panes
            .iter()
            .find(|(_, pane_tabs)| pane_tabs.tabs().contains(&tab))
            .map(|(pane, _)| *pane)
    }

    // Create a terminal for `tab` with our settings, and start its shell.
    fn new_terminal(&self, tab: TabId) -> Terminal {
        let terminal = Terminal::new(&self.shortcuts);
        terminal.set_theme(&self.theme);
        terminal.set_font(&self.settings.font_family, self.settings.font_size);
        terminal.set_scrollback_lines(self.settings.scrollback_lines);

        // When this terminal gets focus, or its shell exits, tell the
        // workspace (through the channel).
        let commands = self.commands.clone();
        terminal.connect_focused(move || {
            let _ = commands.try_send(Command::TabFocused(tab));
        });
        let commands = self.commands.clone();
        terminal.connect_exited(move || {
            let _ = commands.try_send(Command::TabExited(tab));
        });

        terminal.spawn_shell();
        terminal
    }

    // Throw away the old layout widgets and build new ones from the tree.
    fn rebuild(&self) {
        if let Some(old) = self.root.first_child() {
            self.root.remove(&old);
            // Free the pane widgets from the old dividers, so they can be
            // placed into the new ones (a widget can only have one parent).
            detach_children(&old);
        }
        let new = self.build(self.layout.root());
        self.root.append(&new);

        // Moving widgets around loses the keyboard focus, so give it back.
        self.show_active_tab(self.focused);
    }

    // Turn one node of the tree into a widget.
    fn build(&self, node: &Node) -> gtk::Widget {
        match node {
            Node::Pane(id) => self.views[id].widget().clone(),
            Node::Split {
                direction,
                first,
                second,
            } => {
                let orientation = match direction {
                    SplitDirection::Right => gtk::Orientation::Horizontal,
                    SplitDirection::Down => gtk::Orientation::Vertical,
                };
                let paned = gtk::Paned::new(orientation);

                // Start the divider in the middle. GTK only knows the size once
                // the divider is laid out, and then announces it as
                // "max-position". We set the middle then, unless a position was
                // already set (by us earlier, or by the user dragging it).
                paned.connect_max_position_notify(|paned| {
                    if !paned.is_position_set() {
                        paned.set_position(paned.max_position() / 2);
                    }
                });

                paned.set_start_child(Some(&self.build(first)));
                paned.set_end_child(Some(&self.build(second)));
                paned.upcast()
            }
        }
    }
}

// Empty every divider (`gtk::Paned`) below `widget`, so their children have
// no parent anymore and can be reused.
fn detach_children(widget: &gtk::Widget) {
    let Some(paned) = widget.downcast_ref::<gtk::Paned>() else {
        return; // a pane: nothing inside to detach
    };
    if let Some(child) = paned.start_child() {
        paned.set_start_child(None::<&gtk::Widget>);
        detach_children(&child);
    }
    if let Some(child) = paned.end_child() {
        paned.set_end_child(None::<&gtk::Widget>);
        detach_children(&child);
    }
}
