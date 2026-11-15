// The main easymux window: a header bar on top and a workspace below it.

use adw::prelude::*;
use gtk::{gio, glib};

use crate::commands::{Command, CommandSender};
use crate::settings::TerminalSettings;
use crate::shortcuts::GtkShortcuts;
use crate::theme::Theme;
use crate::workspace::{Outcome, Workspace};

pub fn build_window(app: &adw::Application, shortcuts: &GtkShortcuts) {
    // The command channel: buttons, shortcuts and terminals send commands
    // into `sender`; the loop below receives them from `receiver`.
    let (sender, receiver) = async_channel::unbounded::<Command>();

    let mut workspace = Workspace::new(
        shortcuts,
        TerminalSettings::default(),
        Theme::rose_pine(),
        sender.clone(),
    );

    // The title bar, with buttons for the split actions.
    let header = adw::HeaderBar::new();
    header.pack_start(&action_button(
        "easymux-new-tab-symbolic",
        "New tab",
        "win.new-tab",
    ));
    header.pack_start(&action_button(
        "easymux-split-right-symbolic",
        "Split right",
        "win.split-right",
    ));
    header.pack_start(&action_button(
        "easymux-split-down-symbolic",
        "Split down",
        "win.split-down",
    ));
    header.pack_start(&action_button(
        "easymux-close-pane-symbolic",
        "Close tab",
        "win.close-tab",
    ));

    // A container with bars (like the header) around a main content area.
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(workspace.widget()));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("easymux")
        .default_width(1000)
        .default_height(700)
        .content(&toolbar)
        .build();

    // Window actions ("win.split-right", ...). Buttons and shortcuts trigger
    // them by name; each one just sends its command.
    add_command_action(&window, "split-right", Command::SplitRight, &sender);
    add_command_action(&window, "split-down", Command::SplitDown, &sender);
    add_command_action(&window, "new-tab", Command::NewTab, &sender);
    add_command_action(&window, "close-tab", Command::CloseFocusedTab, &sender);

    // When the window closes, close the channel. The loop below then ends and
    // drops the workspace, which frees its terminals and stops their shells.
    // (Without this, the loop would wait forever and keep everything alive.)
    let sender_for_close = sender.clone();
    window.connect_close_request(move |_window| {
        sender_for_close.close();
        glib::Propagation::Proceed
    });

    window.present();

    // The command loop. It owns the workspace: nothing else can change it,
    // so commands are applied strictly one at a time.
    // When the last pane is closed, the workspace is empty: close the window.
    glib::spawn_future_local(async move {
        while let Ok(command) = receiver.recv().await {
            if workspace.handle(command) == Outcome::Empty {
                window.close();
                break;
            }
        }
    });
}

// A header-bar button with an icon and a tooltip, that triggers
// the action called `action_name` when clicked.
fn action_button(icon_name: &str, tooltip: &str, action_name: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon_name);
    button.set_tooltip_text(Some(tooltip));
    button.set_action_name(Some(action_name));
    button
}

// A window action called `name` that sends `command` when triggered.
fn add_command_action(
    window: &adw::ApplicationWindow,
    name: &str,
    command: Command,
    sender: &CommandSender,
) {
    let action = gio::SimpleAction::new(name, None);
    let sender = sender.clone();
    action.connect_activate(move |_action, _parameter| {
        let _ = sender.try_send(command);
    });
    window.add_action(&action);
}
