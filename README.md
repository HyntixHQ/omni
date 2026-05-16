# Omni

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![GitHub Repo](https://img.shields.io/badge/GitHub-HyntixHQ%2Fomni-181717?logo=github)](https://github.com/HyntixHQ/omni)

A high-performance, Wayland-native application launcher and productivity tool. Built with **Rust**, Omni provides a minimalist command-palette interface for launching applications with sub-millisecond search.

## Features

- **Instant Search** — Multi-strategy fuzzy matching over 31+ indexed applications
- **Wayland-Native** — Uses `wlr-layer-shell-v1` for seamless overlay placement on any Wayland compositor
- **CPU Rendering** — `tiny-skia` + `cosmic-text` pipeline, no GPU dependency, ~20 MB RSS
- **shadcn/GPUI-styled UI** — Input fields, list views, badges, keyboard shortcuts matching modern design conventions
- **Keyboard-First** — Navigate with arrow keys, `Ctrl+n/p`, word jumps, selection, clipboard operations
- **Theming** — TOML-configurable colors, fonts, and dimensions with auto-detection from GTK settings
- **Icons** — System icon theme support with LRU-cached SVG/PNG rendering via `resvg`

## Architecture

```
src/main.rs                — Thin entrypoint (daemon init, event loop, draws launcher)
  crates/
  **wisp/**                    — GUI framework (Wayland, draw primitives, input, text, scroll, cursors)
  **wisp-components/**         — shadcn/GPUI-style widget library (input, list_view, badge, kbd, etc.)
  omni-core/               — Data models (AppEntry, SearchResult)
  omni-search/             — Desktop file indexer + fuzzy search engine
  omni-daemon/             — Pure business logic: Wayland event loop, dispatch, surface management
  omni-app-launcher/       — Launcher application (Config, OmniApp state, IconCache, draw)
```

> **Note:** **wisp** (GUI framework) and **wisp-components** (widget library) are actively developed within the Omni project. Once they reach maturity, they will be released as standalone crates for general Rust GUI development. For now, they remain coupled to Omni during development.

**Rendering Pipeline:** `tiny-skia` → CPU rasterize → SHM buffer → Wayland `wl_buffer` → compositor

## Requirements

- A Wayland compositor with `wlr-layer-shell` support (Sway, Hyprland, River, Wayfire, etc.)
- Fontconfig (system fonts)
- XKB common (keyboard layout)

## Installation

### From source

```sh
git clone https://github.com/HyntixHQ/omni && cd omni
cargo build --release
cp target/release/omni ~/.local/bin/omni
```

### Sway keybinding

Add to `~/.config/sway/config`:

```
bindsym $mod+d exec ~/.local/bin/omni
```

### Hyprland

```
bind = $mainMod, d, exec, ~/.local/bin/omni
```

## Configuration

Omni reads `~/.config/omni/config.toml` (or `/etc/omni/config.toml`). If no config file is found, Omni auto-detects the system font, icon theme, and cursor theme from GTK settings.

```toml
[window]
width = 600
height = 400

[theme]
bg = "#1a1a1a"
fg = "#e0e0e0"
selected_bg = "#404040"
selected_fg = "#ffffff"
border = "#333333"
border_radius = 12.0

[font]
family = "Noto Sans"
size = 14.0

[icons]
theme = "Papirus"
size = 28
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Launch selected application |
| `Esc` | Close Omni |
| `↑` / `↓` | Navigate list |
| `Ctrl+p` / `Ctrl+n` | Navigate list |
| `Backspace` | Delete character before cursor |
| `Ctrl+Backspace` | Delete word backward |
| `Ctrl+←` / `Ctrl+→` | Move by word |
| `Home` / `End` | Move to start/end |
| `Delete` | Delete character after cursor |
| `Shift+←` / `Shift+→` | Select text |
| `Ctrl+A` | Select all |
| `Ctrl+C` / `Ctrl+V` / `Ctrl+X` | Copy/paste/cut |

## Performance

| Metric | Value |
|--------|-------|
| Binary size | ~6.9 MB (release, stripped) |
| RAM usage | ~19 MB RSS |
| Startup time | ~50ms (cold cache) |
| Search latency | <1ms |
| Render frame | <16ms (60 FPS) |

## License

GNU General Public License v3.0 — see [LICENSE](LICENSE) for details.
