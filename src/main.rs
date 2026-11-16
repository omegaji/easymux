mod commands;
mod keybindings;
mod layout;
mod pane;
mod pane_view;
mod settings;
mod shell_integration;
mod shortcuts;
mod terminal;
mod theme;
mod title;
mod window;
mod workspace;

use adw::prelude::*;

use crate::keybindings::Keybindings;
use crate::shell_integration::ShellIntegration;
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
        // Always dark, to match the dark terminal theme (a setting later).
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

        let shortcuts = GtkShortcuts::new(&Keybindings::defaults());

        // Write our shell integration scripts out once, for every window.
        let shell_integration = ShellIntegration::install();

        // "activate" fires when the app should show a window.
        app.connect_activate(move |app| {
            window::build_window(app, &shortcuts, &shell_integration)
        });
    });

    // Start the app. This runs until the last window is closed.
    app.run()
}
