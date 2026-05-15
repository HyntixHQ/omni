# AGENTS.md — Omni

## Project

Rust 2024 edition Wayland-native app launcher. CPU-rendered with `tiny-skia` + `cosmic-text`.

## Commands

```sh
cargo build                    # debug build
cargo build --release          # release with LTO, stripped
cargo build -p omni-ui         # build single crate
```

No tests, linters, or formatters set up beyond `cargo clippy` (see workspace config).

## Architecture

```
src/main.rs            — binary entrypoint (Wayland event loop, SHM buffers, jemalloc)
crates/
  omni-core/           — data models (AppEntry, SearchResult, LauncherItem)
  omni-search/         — desktop file indexer + fuzzy search (skim-based)
  omni-ui/             — Wayland shell, rendering, input, theming, events
    components/
      scaffold         — layout shell with optional header/body/footer slots
      list_view        — scrollable list (GPUI uniform_list replica)
      badge            — shadcn-style (4 variants)
      input_field      — search input with cursor
      input            — XKB keyboard handler (US QWERTY fallback)
    events.rs          — MouseDispatcher: per-frame hit region registration + hit testing
    render.rs          — draw pipeline (tiny-skia → SHM buffer → Wayland)
    config.rs          — TOML at ~/.config/omni/config.toml, system font auto-detect
    state.rs           — OmniApp (search, cursor, selection, icon_cache, mouse dispatcher)
```

## Key conventions

- **No `unwrap()`** — handle errors with `?` or `.log_err()`.
- **No comments** unless explaining non-obvious logic.
- **No mod.rs** — use `src/module_name.rs` for all modules.
- **Full variable names** — no abbreviations.
- **`dbg_macro` = deny, `todo` = deny** in Clippy config.
- **`edition = "2024"`** across workspace.
- **Release profile**: `lto = true`, `codegen-units = 1`, `strip = "symbols"`.

## Rendering quirk

All draw positions (`row_y`, `icon_y`, `text_y`) must be `.round()` to integer pixels. Tiny-Skia `floor()` and cosmic-text truncation diverge on fractional positions, causing 1px jitter between icons and text.

## Scroll / navigation

`ListViewState::ensure_visible` uses GPUI's integer-row logic:

```
visible_rows = floor(viewport_height / row_height)
current_row  = round(scroll_offset / row_height)
last_visible = current_row + visible_rows - 1

UP:   sel < current_row   → scroll_offset = sel * row_height
DOWN: sel > last_visible  → scroll_offset = (sel - visible_rows + 1) * row_height
```

## Font loading

Only loads the configured font family (detected from GTK settings). See `src/main.rs:95-110` — temp scan → extract path → load single file. No `load_system_fonts()` in the final database.

## Wayland

- `wlr-layer-shell-v1` for overlay placement.
- Custom SHM buffer pool for pixel data.
- XKB keymap from `wl_keyboard.keymap` event.
- Mouse regions registered per-frame by the scaffold.

## Config

Path: `~/.config/omni/config.toml` or `/etc/omni/config.toml`. All theme fields are hex colors (`#rrggbb`). Font family/size auto-detected from GTK `settings.ini` or `gsettings`.
