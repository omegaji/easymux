// Terminal color themes.
//
// `Theme` is our own type. It does not depend on GTK or VTE, so any part of
// the app can use it, and themes from other apps (Ghostty, Kitty, ...) can be
// converted into it.

// One color, as red, green and blue (each 0 to 255).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    // Make a color from a hex number, like 0x191724 (the same as "#191724").
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            red: (hex >> 16) as u8,
            green: (hex >> 8) as u8,
            blue: hex as u8,
        }
    }
}

// All the colors a terminal needs.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub cursor_text: Color,
    pub selection_background: Color,
    pub selection_foreground: Color,
    // The 16 standard terminal colors: black, red, green, yellow, blue,
    // magenta, cyan, white, then the "bright" version of each.
    pub palette: [Color; 16],
}

impl Theme {
    // Rose Pine (https://rosepinetheme.com), the default theme for now.
    pub fn rose_pine() -> Self {
        Self {
            name: "Rose Pine".to_string(),
            background: Color::from_hex(0x191724),
            foreground: Color::from_hex(0xe0def4),
            cursor: Color::from_hex(0xe0def4),
            cursor_text: Color::from_hex(0x191724),
            selection_background: Color::from_hex(0x403d52),
            selection_foreground: Color::from_hex(0xe0def4),
            palette: [
                Color::from_hex(0x26233a), // black
                Color::from_hex(0xeb6f92), // red
                Color::from_hex(0x31748f), // green
                Color::from_hex(0xf6c177), // yellow
                Color::from_hex(0x9ccfd8), // blue
                Color::from_hex(0xc4a7e7), // magenta
                Color::from_hex(0xebbcba), // cyan
                Color::from_hex(0xe0def4), // white
                Color::from_hex(0x6e6a86), // bright black
                Color::from_hex(0xeb6f92), // bright red
                Color::from_hex(0x31748f), // bright green
                Color::from_hex(0xf6c177), // bright yellow
                Color::from_hex(0x9ccfd8), // bright blue
                Color::from_hex(0xc4a7e7), // bright magenta
                Color::from_hex(0xebbcba), // bright cyan
                Color::from_hex(0xe0def4), // bright white
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hex_splits_into_red_green_blue() {
        let color = Color::from_hex(0x191724);
        assert_eq!(color.red, 0x19);
        assert_eq!(color.green, 0x17);
        assert_eq!(color.blue, 0x24);
    }
}
