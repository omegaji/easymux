// User settings, with their default values in one place.
//
// Plain Rust, no GTK. Later a settings screen and a config file will
// read and change these.

// Settings for every terminal.
#[derive(Debug, Clone)]
pub struct TerminalSettings {
    pub font_family: String,
    pub font_size: u32,
    // How many old lines you can scroll back to.
    pub scrollback_lines: u32,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 13,
            scrollback_lines: 10_000,
        }
    }
}
