// Short, readable tab titles from folder paths.
//
// Plain Rust, no GTK. Like cmux, a long path keeps only its last three parts:
//   /Users/om/omji/my_work/easymux  ->  .../omji/my_work/easymux
//   /Users/om/projects              ->  ~/projects

use std::path::{Path, PathBuf};

// How many path parts a title keeps before shortening with ".../".
const MAX_PARTS: usize = 3;

// The title when nothing better is known.
pub const DEFAULT_TITLE: &str = "Terminal";

// The title to show, by priority:
// 1. a custom name the user gave (overrides everything),
// 2. an automatic title (the program's title or the current folder),
// 3. the default "Terminal".
// Empty names count as "not set".
pub fn tab_title(custom: Option<&str>, automatic: Option<&str>) -> String {
    custom
        .filter(|name| !name.is_empty())
        .or(automatic.filter(|title| !title.is_empty()))
        .unwrap_or(DEFAULT_TITLE)
        .to_string()
}

// `path` as a short title. The home folder is shown as "~".
pub fn short_path(path: &Path, home: &Path) -> String {
    let display = match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => return "~".to_string(),
        Ok(rest) => PathBuf::from("~").join(rest),
        Err(_) => path.to_path_buf(), // not inside the home folder
    };

    // The parts of the path, e.g. ["~", "omji", "my_work", "easymux"].
    let parts: Vec<_> = display.iter().map(|part| part.to_string_lossy()).collect();
    if parts.len() <= MAX_PARTS {
        return display.to_string_lossy().into_owned();
    }
    format!(".../{}", parts[parts.len() - MAX_PARTS..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short(path: &str) -> String {
        short_path(Path::new(path), Path::new("/Users/om"))
    }

    #[test]
    fn custom_name_overrides_everything() {
        assert_eq!(tab_title(Some("api server"), Some("cargo run")), "api server");
    }

    #[test]
    fn automatic_title_when_no_custom_name() {
        assert_eq!(tab_title(None, Some("cargo run")), "cargo run");
    }

    #[test]
    fn empty_names_count_as_not_set() {
        assert_eq!(tab_title(Some(""), Some("cargo run")), "cargo run");
        assert_eq!(tab_title(Some(""), Some("")), DEFAULT_TITLE);
    }

    #[test]
    fn default_when_nothing_is_known() {
        assert_eq!(tab_title(None, None), DEFAULT_TITLE);
    }

    #[test]
    fn home_is_a_tilde() {
        assert_eq!(short("/Users/om"), "~");
    }

    #[test]
    fn short_paths_inside_home_start_with_tilde() {
        assert_eq!(short("/Users/om/projects"), "~/projects");
        assert_eq!(short("/Users/om/omji/my_work"), "~/omji/my_work");
    }

    #[test]
    fn long_paths_keep_their_last_three_parts() {
        assert_eq!(short("/Users/om/omji/my_work/easymux"), ".../omji/my_work/easymux");
    }

    #[test]
    fn paths_outside_home_are_kept_or_shortened() {
        assert_eq!(short("/"), "/");
        assert_eq!(short("/usr/local"), "/usr/local");
        assert_eq!(short("/usr/local/share/fonts"), ".../local/share/fonts");
    }
}
