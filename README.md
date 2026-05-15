# Omni

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![GitHub Repo](https://img.shields.io/badge/GitHub-HyntixHQ%2Fomni-181717?logo=github)](https://github.com/HyntixHQ/omni)

A high-performance, Wayland-native application launcher and productivity tool. Built with **Rust**, Omni provides a minimalist command-palette interface for launching applications with sub-millisecond search.

## Features

- **Instant Search** — Multi-strategy fuzzy matching (exact, skim, substring, word-prefix, relaxed) over 31+ indexed applications
- **Wayland-Native** — Uses `wlr-layer-shell-v1` for seamless overlay placement on any Wayland compositor
- **Custom Rendering** — CPU-based 2D pipeline using `tiny-skia` with `cosmic-text` for high-quality typography
- **Keyboard-First** — Navigate with arrow keys, `Ctrl-n/p`, or vim-style bindings. `Enter` to launch, `Esc` to dismiss
- **Theming** — TOML-configurable colors, fonts, and dimensions. Default Catppuccin Mocha theme
- **Icons** — System icon theme support with LRU-cached SVG/PNG rendering via `resvg`
- **Configurable** — Font family/size, window dimensions, icon theme, and cursor theme all via `~/.config/omni/config.toml`

## Architecture

```
omni                      # Binary entry point (Wayland event loop, SHM buffers)
├── crates/
│   ├── omni-core/        # Data models (AppEntry, SearchResult, LauncherItem)
│   ├── omni-search/      # Desktop file indexer + multi-strategy fuzzy search engine
│   └── omni-ui/          # Wayland shell, rendering pipeline, input handling, theming
│       ├── scaffold      # Layout shell with optional header/body/footer slots
│       ├── list-view     # Scrollable list with GPUI-style row-based navigation
│       ├── input-field   # Search input with cursor, placeholder, word editing
│       ├── badge         # shadcn-style badge (4 variants: default/secondary/destructive/outline)
│       └── events        # GPUI-inspired hit region dispatcher for mouse events
```

**Rendering Pipeline:** `tiny-skia` → CPU rasterize → SHM buffer → Wayland `wl_buffer` → compositor

## Requirements

- A Wayland compositor with `wlr-layer-shell` support (Sway, Hyprland, River, Wayfire, etc.)
- Fontconfig (system fonts)
- XKB common (keyboard layout)

## Installation

### From source

```sh
git clone <repo-url> && cd omni
cargo build --release
install -Dm755 target/release/omni ~/.local/bin/omni
```

### Using the Makefile

```sh
make build       # cargo build --release
make install     # install binary to $PREFIX/bin/omni
```

## Setup

### systemd (autostart)

```sh
make install-service   # copy service file
systemctl --user enable --now omni-launcher
```

### Sway keybinding

Add to `~/.config/sway/config`:

```
bindsym $mod+slash exec ~/.local/bin/omni
```

### Hyprland

```
bind = $mainMod, slash, exec, ~/.local/bin/omni
```

### Direct launch

```sh
~/.local/bin/omni
```

## Configuration

Omni reads `~/.config/omni/config.toml` (or `/etc/omni/config.toml`). A minimal config:

```toml
[window]
width = 640
height = 410

[theme]
bg = "#1e1e2e"
fg = "#cdd6f4"
selected_bg = "#89b4fa"
selected_fg = "#11111b"
border = "#89b4fa"
border_radius = 12.0

[font]
family = "Noto Sans"
size = 15.0

[icons]
theme = "Papirus"
size = 28
```

If no config file is found, Omni auto-detects the system font, icon theme, and cursor theme from GTK settings.

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
