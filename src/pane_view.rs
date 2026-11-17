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
use gtk::{gdk, glib};

use crate::commands::{Command, CommandSender};
use crate::layout::{PaneId, SplitDirection};
use crate::pane::TabId;

// Tab title width, in characters. Titles longer than the max end with "…".
// Tabs shrink down to the min when space runs out; after that, the tab bar
// scrolls instead (without a min, tabs would shrink to "…" and never scroll).
const MIN_TITLE_CHARS: i32 = 12;
const MAX_TITLE_CHARS: i32 = 24;

// How far one click of a mouse wheel scrolls the tabs, in pixels.
const WHEEL_STEP: f64 = 40.0;

// The smallest a pane can be dragged to, in pixels: room for the tab bar
// and a few lines of terminal. (The dividers in workspace.rs respect this.)
const MIN_PANE_WIDTH: i32 = 200;
const MIN_PANE_HEIGHT: i32 = 120;

pub struct PaneView {
    // The whole pane: the tab bar above the stack.
    container: gtk::Box,
    // Holds the tabs. Emptied and refilled by `update_tabs`.
    tabs: gtk::Box,
    // Scrolls the tabs sideways when there are more than fit.
    tabs_viewport: gtk::Viewport,
    stack: gtk::Stack,
    commands: CommandSender,
}

impl PaneView {
    pub fn new(pane: PaneId, commands: CommandSender) -> Self {
        let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        // When there are more tabs than fit, they scroll sideways. A scrollbar
        // only appears when needed. The scroll area takes all the width the
        // right-side buttons don't.
        let tabs_viewport = gtk::Viewport::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        tabs_viewport.set_child(Some(&tabs));
        let tabs_scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .child(&tabs_viewport)
            .hexpand(true)
            .build();
        scroll_sideways_with_wheel(&tabs_scroller);

        // "pane-bar" is the style name used in resources/style.css.
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        bar.add_css_class("pane-bar");
        bar.append(&tabs_scroller);
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
        stack.add_css_class("pane-terminals");

        // "pane" (and "focused", see `set_focused`) are used in style.css.
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("pane");
        container.set_size_request(MIN_PANE_WIDTH, MIN_PANE_HEIGHT);
        container.append(&bar);
        container.append(&stack);

        // Scrolling over a pane belongs to the pane: its terminal (history, or
        // the program running in it) or its tab strip. Whatever they leave
        // over is swallowed here, so it never moves the workspace. The
        // workspace only scrolls by dragging its scrollbars.
        let keep_scrolls =
            gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        keep_scrolls.connect_scroll(|_controller, _dx, _dy| glib::Propagation::Stop);
        container.add_controller(keep_scrolls);

        Self {
            container,
            tabs,
            tabs_viewport,
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

    // Mark this pane as the one with keyboard focus (or not). The look comes
    // from the "focused" style in style.css: other panes are dimmed.
    pub fn set_focused(&self, focused: bool) {
        if focused {
            self.container.add_css_class("focused");
        } else {
            self.container.remove_css_class("focused");
        }
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
        let mut active_widget = None;
        for (tab, title) in tabs {
            let widget = self.tab_widget(*tab, title, *tab == active);
            self.tabs.append(&widget);
            if *tab == active {
                active_widget = Some(widget);
            }
        }

        // Scroll the active tab into view. GTK can only do that once the new
        // tabs are laid out on screen, so we ask for it when GTK is idle,
        // which is after the layout.
        if let Some(active_widget) = active_widget {
            let viewport = self.tabs_viewport.clone();
            glib::idle_add_local_once(move || {
                viewport.scroll_to(&active_widget, None);
            });
        }
    }

    // One tab: a flat rectangle with its title and a ✕.
    // Click anywhere on it to show it; click the ✕ to close it.
    // (Its look comes from the "tab" and "active" styles in style.css.)
    fn tab_widget(&self, tab: TabId, title: &str, active: bool) -> gtk::Box {
        let label = gtk::Label::new(Some(title));
        label.set_width_chars(MIN_TITLE_CHARS);
        label.set_max_width_chars(MAX_TITLE_CHARS);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End); // cut long titles with "…"

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

// Let a normal mouse wheel (up/down) scroll the tabs sideways. GTK only does
// that by itself while Shift is held; trackpad sideways swipes already work.
// Only when the tabs don't fit: otherwise the scroll goes on to the
// workspace, so scrolling over a tab bar moves the workspace.
fn scroll_sideways_with_wheel(scroller: &gtk::ScrolledWindow) {
    let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    // The closure holds the adjustment (the scroll position), not the
    // scroller itself, so there is no reference cycle.
    let position = scroller.hadjustment();
    wheel.connect_scroll(move |wheel, _dx, dy| {
        let tabs_overflow = position.upper() - position.lower() > position.page_size();
        if !tabs_overflow {
            return glib::Propagation::Proceed; // nothing to scroll here
        }
        let step = if wheel.unit() == gdk::ScrollUnit::Wheel {
            WHEEL_STEP
        } else {
            1.0 // trackpads already report pixels
        };
        position.set_value(position.value() + dy * step);
        glib::Propagation::Stop
    });
    scroller.add_controller(wheel);
}
