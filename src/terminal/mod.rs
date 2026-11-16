// One terminal: a screen (the VTE widget) with a shell running inside it.
//
// The rest of the app only uses this `Terminal` type, never VTE directly.
// That way we can replace VTE later (for example with libghostty) by
// changing only this file.

use std::cell::Cell;
use std::rc::Rc;

use gtk::gio;
use gtk::prelude::*;
use vte::prelude::*;

use crate::shortcuts::GtkShortcuts;
use crate::theme::{Color, Theme};
use crate::title;

// Names of the values the shell can set (VTE calls them "termprops").
const TITLE: &str = "xterm.title"; // the title, set with an escape code (OSC 0/2)
const CURRENT_FOLDER: &str = "vte.cwd"; // the current folder (OSC 7)

// Which of the two the shell (or a program) changed most recently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatestTitle {
    Nothing, // neither was set yet
    Title,   // e.g. "cargo run", or Claude Code's own title
    Folder,  // e.g. ".../omji/my_work/easymux"
}

pub struct Terminal {
    vte: vte::Terminal,
    // What the window shows: the terminal with its scrollbar beside it.
    container: gtk::Box,
    // Which one to show: "most recent wins". Shared with the closure in `new`
    // that updates it, hence `Rc<Cell<...>>`.
    latest_title: Rc<Cell<LatestTitle>>,
}

impl Terminal {
    pub fn new(shortcuts: &GtkShortcuts) -> Self {
        let vte = vte::Terminal::new();
        add_actions(&vte);

        // Remember whether the title or the folder changed last.
        let latest_title = Rc::new(Cell::new(LatestTitle::Nothing));
        let latest_for_closure = latest_title.clone();
        vte.connect_termprop_changed(None, move |_terminal, name| {
            if name == TITLE {
                latest_for_closure.set(LatestTitle::Title);
            } else if name == CURRENT_FOLDER {
                latest_for_closure.set(LatestTitle::Folder);
            }
        });

        // Listen to the shared shortcut list. When it changes (for example from
        // settings), this terminal picks up the new keys automatically.
        // "Capture" means we see the keys before VTE does; otherwise VTE would
        // send them to the shell first.
        let controller = gtk::ShortcutController::for_model(shortcuts.list());
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        vte.add_controller(controller);

        // A scrollbar beside the terminal. Both share the terminal's
        // "adjustment" (the scroll position), so they always move together.
        // VTE's docs recommend this instead of a GtkScrolledWindow.
        let adjustment = vte.vadjustment();
        let scrollbar = gtk::Scrollbar::new(gtk::Orientation::Vertical, adjustment.as_ref());
        vte.set_hexpand(true); // the terminal takes all the width the scrollbar doesn't

        // Show the scrollbar only when there is something to scroll.
        // "changed" fires whenever the total or visible height changes.
        // The closure holds a *weak* handle: the scrollbar already holds the
        // adjustment, so a normal handle here would make a cycle.
        scrollbar.set_visible(false);
        if let Some(adjustment) = adjustment {
            let weak_scrollbar = scrollbar.downgrade();
            adjustment.connect_changed(move |adjustment| {
                if let Some(scrollbar) = weak_scrollbar.upgrade() {
                    let total = adjustment.upper() - adjustment.lower();
                    scrollbar.set_visible(total > adjustment.page_size());
                }
            });
        }

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        container.append(&vte);
        container.append(&scrollbar);

        Self {
            vte,
            container,
            latest_title,
        }
    }

    // The widget to place inside a window.
    pub fn widget(&self) -> &gtk::Widget {
        self.container.upcast_ref()
    }

    // Apply a color theme to this terminal.
    pub fn set_theme(&self, theme: &Theme) {
        // Convert all 16 palette colors to GTK colors.
        let palette = theme.palette.map(to_rgba);

        self.vte.set_colors(
            Some(&to_rgba(theme.foreground)),
            Some(&to_rgba(theme.background)),
            &palette.each_ref(),
        );
        self.vte.set_color_cursor(Some(&to_rgba(theme.cursor)));
        self.vte
            .set_color_cursor_foreground(Some(&to_rgba(theme.cursor_text)));
        self.vte
            .set_color_highlight(Some(&to_rgba(theme.selection_background)));
        self.vte
            .set_color_highlight_foreground(Some(&to_rgba(theme.selection_foreground)));
    }

    // Set the font, for example family "JetBrains Mono" and size 13 (in points).
    // If that font is not installed, the system's monospace font is used instead.
    pub fn set_font(&self, family: &str, size: u32) {
        let font = gtk::pango::FontDescription::from_string(&format!("{family}, monospace {size}"));
        self.vte.set_font(Some(&font));
    }

    // How many old lines the user can scroll back to.
    pub fn set_scrollback_lines(&self, lines: u32) {
        self.vte.set_scrollback_lines(lines.into());
    }

    // Run `callback` whenever this terminal receives keyboard focus
    // (for example the user clicks into it).
    pub fn connect_focused<F: Fn() + 'static>(&self, callback: F) {
        let focus = gtk::EventControllerFocus::new();
        focus.connect_enter(move |_controller| callback());
        self.vte.add_controller(focus);
    }

    // Move keyboard focus into this terminal.
    pub fn grab_focus(&self) {
        self.vte.grab_focus();
    }

    // Run `callback` when the shell exits (for example the user types "exit").
    pub fn connect_exited<F: Fn() + 'static>(&self, callback: F) {
        self.vte
            .connect_child_exited(move |_terminal, _status| callback());
    }

    // The automatic title for this terminal's tab: whichever changed most
    // recently, the title (set by the shell or a program) or the current
    // folder. `None` if neither was set yet.
    pub fn automatic_title(&self) -> Option<String> {
        match self.latest_title.get() {
            LatestTitle::Title => self.shell_title().or_else(|| self.folder_title()),
            LatestTitle::Folder => self.folder_title().or_else(|| self.shell_title()),
            LatestTitle::Nothing => None,
        }
    }

    // The title set with OSC 0/2, if any.
    fn shell_title(&self) -> Option<String> {
        let (text, _length) = self.vte.termprop_string(TITLE);
        text.map(|text| text.to_string()).filter(|text| !text.is_empty())
    }

    // The current folder reported with OSC 7, shortened, if any.
    fn folder_title(&self) -> Option<String> {
        // The vte4 library can't read the "vte.cwd" value directly yet, so we
        // use the older function, which returns the same value.
        #[allow(deprecated)]
        let uri = self.vte.current_directory_uri()?;
        // "file:///Users/om/projects" -> "/Users/om/projects"
        let (path, _host) = gtk::glib::filename_from_uri(&uri).ok()?;
        Some(title::short_path(&path, &gtk::glib::home_dir()))
    }

    // Run `callback` whenever the title or the current folder changes.
    pub fn connect_title_changed<F: Fn() + 'static>(&self, callback: F) {
        self.vte.connect_termprop_changed(None, move |_terminal, name| {
            if name == TITLE || name == CURRENT_FOLDER {
                callback();
            }
        });
    }

    // Start `shell` (e.g. "/bin/zsh") inside the terminal, with `environment`
    // ("NAME=value" entries) added to the environment it inherits.
    pub fn spawn_shell(&self, shell: &str, environment: &[String]) {
        let home = gtk::glib::home_dir();
        // VTE wants a list of `&str`, so borrow each `String` as one.
        let environment: Vec<&str> = environment.iter().map(String::as_str).collect();

        self.vte.spawn_async(
            vte::PtyFlags::DEFAULT,
            home.to_str(), // start in the home folder
            &[shell],      // the command to run
            &environment,  // extra environment variables
            gtk::glib::SpawnFlags::DEFAULT,
            || {},                          // nothing extra to set up in the child
            -1,                             // use VTE's default time limit for starting
            None::<&gtk::gio::Cancellable>, // no way to cancel
            |result| {
                if let Err(err) = result {
                    eprintln!("Failed to start shell: {err}");
                }
            },
        );
    }
}

// The actions this terminal offers, in the action group "term".
// Shortcuts (see shortcuts.rs) and, later, menus trigger them by name,
// for example "term.copy". This code knows nothing about keys.
fn add_actions(vte: &vte::Terminal) {
    let actions = gio::SimpleActionGroup::new();

    // Each closure holds a *weak* handle to the terminal. A normal handle
    // would make a cycle (terminal -> actions -> closure -> terminal), and the
    // terminal could never be freed.
    let copy = gio::SimpleAction::new("copy", None);
    let weak_vte = vte.downgrade();
    copy.connect_activate(move |_action, _parameter| {
        if let Some(vte) = weak_vte.upgrade() {
            vte.copy_clipboard_format(vte::Format::Text);
        }
    });
    actions.add_action(&copy);

    let paste = gio::SimpleAction::new("paste", None);
    let weak_vte = vte.downgrade();
    paste.connect_activate(move |_action, _parameter| {
        if let Some(vte) = weak_vte.upgrade() {
            vte.paste_clipboard();
        }
    });
    actions.add_action(&paste);

    vte.insert_action_group("term", Some(&actions));
}

// Convert our `Color` (0 to 255 per channel) into GTK's color type,
// which uses 0.0 to 1.0 per channel, plus alpha (1.0 = fully opaque).
fn to_rgba(color: Color) -> gtk::gdk::RGBA {
    gtk::gdk::RGBA::new(
        f32::from(color.red) / 255.0,
        f32::from(color.green) / 255.0,
        f32::from(color.blue) / 255.0,
        1.0,
    )
}
