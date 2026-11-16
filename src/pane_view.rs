// The widgets of one pane: a tab bar on top, and a stack of terminals below.
//
//   ┌─────────────────────────────────────────────────┐
//   │ [Terminal ✕] [Terminal ✕]          [+] [⫿] [⊟]  │  ← tab bar
//   ├─────────────────────────────────────────────────┤
//   │ $ _                                             │  ← stack: shows the active tab
//
// A `gtk::Stack` holds all the pane's terminals but shows one at a time.
// The buttons don't change anything themselves: they send commands.

use gtk::prelude::*;

use crate::commands::{Command, CommandSender};
use crate::layout::{PaneId, SplitDirection};
use crate::pane::TabId;

pub struct PaneView {
    // The whole pane: the tab bar above the stack.
    container: gtk::Box,
    // Holds the tab buttons. Emptied and refilled by `update_tabs`.
    tabs: gtk::Box,
    stack: gtk::Stack,
    commands: CommandSender,
}

impl PaneView {
    pub fn new(pane: PaneId, commands: CommandSender) -> Self {
        // The tabs take all the width the right-side buttons don't.
        let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        tabs.set_hexpand(true);

        // "pane-bar" is the style name used in resources/style.css.
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        bar.add_css_class("pane-bar");
        bar.append(&tabs);
        bar.append(&command_button(
            "easymux-new-tab-symbolic",
            "New tab",
            Command::NewTabIn(pane),
            &commands,
        ));
        bar.append(&command_button(
            "easymux-split-right-symbolic",
            "Split right",
            Command::Split(pane, SplitDirection::Right),
            &commands,
        ));
        bar.append(&command_button(
            "easymux-split-down-symbolic",
            "Split down",
            Command::Split(pane, SplitDirection::Down),
            &commands,
        ));

        let stack = gtk::Stack::new();
        stack.set_vexpand(true); // the terminals take all the height

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.append(&bar);
        container.append(&stack);

        Self {
            container,
            tabs,
            stack,
            commands,
        }
    }

    // The widget to place in the layout.
    pub fn widget(&self) -> &gtk::Widget {
        self.container.upcast_ref()
    }

    // Add a tab's terminal widget to the stack (not shown yet).
    pub fn add_terminal(&self, terminal: &gtk::Widget) {
        self.stack.add_child(terminal);
    }

    // Take a tab's terminal widget out of the stack.
    pub fn remove_terminal(&self, terminal: &gtk::Widget) {
        self.stack.remove(terminal);
    }

    // Show this terminal (the active tab's).
    pub fn show_terminal(&self, terminal: &gtk::Widget) {
        self.stack.set_visible_child(terminal);
    }

    // Redraw the tab buttons: one per tab, in order, with `active` highlighted.
    pub fn update_tabs(&self, tabs: &[(TabId, String)], active: TabId) {
        while let Some(child) = self.tabs.first_child() {
            self.tabs.remove(&child);
        }
        for (tab, title) in tabs {
            self.tabs.append(&self.tab_widget(*tab, title, *tab == active));
        }
    }

    // One tab: a flat rectangle with its title and a ✕.
    // Click anywhere on it to show it; click the ✕ to close it.
    // (Its look comes from the "tab" and "active" styles in style.css.)
    fn tab_widget(&self, tab: TabId, title: &str, active: bool) -> gtk::Box {
        let label = gtk::Label::new(Some(title));

        let close = gtk::Button::from_icon_name("easymux-close-pane-symbolic");
        close.add_css_class("flat");
        close.add_css_class("tab-close");
        close.set_tooltip_text(Some("Close tab"));
        let commands = self.commands.clone();
        close.connect_clicked(move |_| {
            let _ = commands.try_send(Command::CloseTab(tab));
        });

        let tab_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        tab_box.add_css_class("tab");
        if active {
            tab_box.add_css_class("active");
        }
        tab_box.append(&label);
        tab_box.append(&close);

        // A click on the tab (but not on its ✕, which handles its own clicks)
        // shows it.
        let click = gtk::GestureClick::new();
        let commands = self.commands.clone();
        click.connect_released(move |_gesture, _presses, _x, _y| {
            let _ = commands.try_send(Command::SelectTab(tab));
        });
        tab_box.add_controller(click);

        tab_box
    }
}

// A flat button with an icon and a tooltip, that sends `command` when clicked.
fn command_button(
    icon_name: &str,
    tooltip: &str,
    command: Command,
    commands: &CommandSender,
) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon_name);
    button.add_css_class("flat");
    button.set_tooltip_text(Some(tooltip));
    let commands = commands.clone();
    button.connect_clicked(move |_| {
        let _ = commands.try_send(command);
    });
    button
}
