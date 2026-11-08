use adw::prelude::*;
use vte::prelude::*;

// A unique name for our app, in "reverse domain" style.
// The desktop uses it to tell apps apart.
const APP_ID: &str = "dev.easymux.Easymux";

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();

    // "activate" fires when the app starts. Then we build our window.
    app.connect_activate(build_window);

    // Start the app. This runs until the last window is closed.
    app.run()
}

fn build_window(app: &adw::Application) {
    // The title bar at the top of the window.
    let header = adw::HeaderBar::new();

    // A container with bars (like the header) around a main content area.
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);

    // The terminal widget fills the main area, below the header.
    let terminal = vte::Terminal::new();
    toolbar.set_content(Some(&terminal));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("easymux")
        .default_width(1000)
        .default_height(700)
        .content(&toolbar)
        .build();

    // When the shell exits (for example you type "exit"), close the window.
    // The closure needs its own handle to the window, so we clone it.
    let window_handle = window.clone();
    terminal.connect_child_exited(move |_terminal, _status| window_handle.close());

    spawn_shell(&terminal);
    window.present();
}

// Start the user's shell (zsh, bash, ...) inside the terminal.
fn spawn_shell(terminal: &vte::Terminal) {
    // $SHELL holds the user's shell. Fall back to /bin/sh if it is not set.
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let home = gtk::glib::home_dir();

    terminal.spawn_async(
        vte::PtyFlags::DEFAULT,
        home.to_str(),                   // start in the home folder
        &[&shell],                       // the command to run
        &[],                             // no extra environment variables
        gtk::glib::SpawnFlags::DEFAULT,
        || {},                           // nothing extra to set up in the child
        -1,                              // use VTE's default time limit for starting
        None::<&gtk::gio::Cancellable>,  // no way to cancel
        |result| {
            if let Err(err) = result {
                eprintln!("Failed to start shell: {err}");
            }
        },
    );
}
