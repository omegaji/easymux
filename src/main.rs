mod commands;
mod keybindings;
mod layout;
mod pane;
mod settings;
mod shortcuts;
mod terminal;
mod theme;
mod window;
mod workspace;

use adw::prelude::*;

use crate::keybindings::Keybindings;
use crate::shortcuts::GtkShortcuts;

// A unique name for our app, in "reverse domain" style.
// The desktop uses it to tell apps apart.
const APP_ID: &str = "dev.easymux.Easymux";

fn main() -> gtk::glib::ExitCode {
    // Make our bundled files (icons, ...) available. build.rs created the
    // bundle; this puts it inside the program and tells GLib about it.
    gtk::gio::resources_register_include!("easymux.gresource")
        .expect("failed to register easymux resources");

    let app = adw::Application::builder().application_id(APP_ID).build();

    // "startup" fires once, when GTK is ready. We create the shared shortcuts
    // here, so every window (and every terminal in it) uses the same list.
    app.connect_startup(|app| {
        let shortcuts = GtkShortcuts::new(&Keybindings::defaults());

        // "activate" fires when the app should show a window.
        app.connect_activate(move |app| window::build_window(app, &shortcuts));
    });

    // Start the app. This runs until the last window is closed.
    app.run()
}
