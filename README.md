# easymux

> **Work in progress.** easymux is in early development. Expect missing features,
> rough edges and breaking changes.

A terminal app for Linux with the look and feel of [cmux](https://github.com/manaflow-ai/cmux):
split panes, tabs inside each pane, and (planned) a vertical sidebar of workspaces and
notifications for AI coding agents. cmux is macOS-only; easymux aims to bring a similar
experience to Linux.

easymux is an independent project. It is not affiliated with cmux or Ghostty.

## Status

Linux is the target. So far, easymux has been developed and run on **macOS**;
testing on Linux (Ubuntu) is still to come.

**Working now**

- Terminal with the Rose Pine color theme and JetBrains Mono font
- Split panes (right / down), with draggable dividers and a minimum pane size
- Several tabs per pane, with titles that follow the current folder or the running command
- Shell integration for zsh and bash (titles work with any shell config)
- Copy / paste, scrollback with a scrollbar
- Keyboard shortcuts (configurable defaults, Ghostty-style)

**Planned**

- Workspaces in a vertical sidebar (git branch, folder, ports)
- Notifications for AI coding agents
- A CLI and socket API to control easymux from scripts
- Session restore, settings screen, theme picker, built-in browser pane

## Keyboard shortcuts

| Action      | macOS | Linux        |
|-------------|-------|--------------|
| Copy        | ⌘C    | Ctrl+Shift+C |
| Paste       | ⌘V    | Ctrl+Shift+V |
| Split right | ⌘D    | Ctrl+Shift+O |
| Split down  | ⌘⇧D   | Ctrl+Shift+E |
| New tab     | ⌘T    | Ctrl+Shift+T |
| Close tab   | ⌘W    | Ctrl+Shift+W |

## Building

easymux is written in Rust with GTK 4, libadwaita and VTE. You need
[Rust](https://rustup.rs) (stable) and these libraries:

**Ubuntu / Debian**

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libvte-2.91-gtk4-dev
```

**macOS (Homebrew)**

```sh
brew install pkgconf gtk4 libadwaita vte3
```

Then:

```sh
cargo run
```

## Credits

- Terminal widget: [VTE](https://gitlab.gnome.org/GNOME/vte)
- Icons based on [Lucide](https://lucide.dev) (ISC license, see `resources/icons/LICENSE-lucide.txt`)
- Inspired by [cmux](https://github.com/manaflow-ai/cmux) and [Ghostty](https://ghostty.org)
