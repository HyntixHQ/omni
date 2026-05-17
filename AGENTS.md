# AGENTS.md — Omni

## Project

Rust 2024 edition Wayland-native app launcher. CPU-rendered with `tiny-skia` + `cosmic-text`.

## Commands

```sh
cargo build                    # debug build
cargo build --release          # release with LTO, stripped
cargo clippy                   # lint check (zero warnings required)
```

## Architecture

```
src/main.rs            — entrypoint: daemon mode or IPC client
  run_daemon()         — persistent background process
  run_client()         — connects to daemon via IPC socket, sends command, exits

crates/
  wisp/                — GUI framework
    surface.rs         — WispSurface (layer-shell, SHM buffers)
    input.rs           — WispInput (XKB), InputAction, MouseButton, scroll tracking
    draw.rs            — fill_rect, draw_text, draw_pixmap, rounded rects, clipping
    events.rs          — MouseDispatcher (hit regions, cursor resolution)
    text.rs            — TextEditor (grapheme cursor, selection, clipboard)
    style.rs           — Size enum (xs/sm/md/lg)
    scroll.rs          — ScrollHandle, ScrollDelta, ScrollbarState
    cursor.rs          — CursorBlink (500ms interval, 300ms pause)
    cursor_style.rs    — CursorStyle (18 variants), CursorManager

  wisp-components/     — shadcn/GPUI-style widgets
    input.rs           — shadcn Input (size variants, prefix/suffix, focus ring)
    list_view.rs       — ListState + draw_list (ScrollHandle, hover, selection)
    badge.rs           — 4 variants (default/secondary/destructive/outline)
    label.rs           — shadcn Label (text-sm, font-medium, disabled opacity)
    kbd.rs             — Keyboard shortcut display
    separator.rs       — Horizontal/vertical divider
    scroll_area.rs     — Scrollbar thumb
    command.rs         — Command palette layout

  omni-core/           — data models (AppEntry, SearchResult)
  omni-search/         — desktop file indexer + fuzzy search
  omni-daemon/         — IPC-aware daemon (Wayland dispatch, poll loop, socket)
  omni-app-launcher/   — launcher Config, OmniApp, IconCache, draw
  omni-calculator/     — CalcState, draw_calculator (evalexpr)
```

### Daemon IPC

- Abstract Unix socket `@omni-ipc`
- Single `poll()`: Wayland fd + IPC listener + client fds
- LF-delimited commands: `launcher`, `calculator`, `calculator:expr`, `quit`
- Surface hides on dismiss (attach null), stays alive for next IPC command

## Key conventions

- No `unwrap()` — handle errors with `?` or `.log_err()`.
- No comments unless explaining non-obvious logic.
- No `mod.rs` — use `src/module_name.rs`.
- Full variable names — no abbreviations.
- `dbg_macro` = deny, `todo` = deny in Clippy config.
- `edition = "2024"` across workspace.
- Release profile: `lto = true`, `codegen-units = 1`, `strip = "symbols"`.

## Rendering quirk

All draw positions must be `.round()` to integer pixels. Tiny-Skia and cosmic-text diverge on fractional positions.

## Scroll

- `ScrollHandle::ensure_visible` uses 0.5px EPSILON to prevent floating-point jitter.
- Only fully visible items draw the active border. Background clips via `fill_rect_clipped`.

## Cursor blink

- 500ms interval, 300ms pause after `CursorBlink::mark_activity()`.

## Input

- `WispInput` tracks keyboard (XKB) + scroll + mouse state.
- GPUI-aligned mouse methods: `mouse_enter/move/exit/button`.
- `TextEditor` for grapheme-cluster cursor, selection, clipboard.

## Config

Path: `~/.config/omni/config.toml` or `/etc/omni/config.toml`. Theme fields use `#rrggbb` hex. Font family/size auto-detected from GTK.
