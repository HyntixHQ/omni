# Changelog

## [0.2.0] — 2026-05-16

### Added
- `wisp` GUI framework crate extracted from `omni-ui` — Wayland, draw, input, text, events, style, scroll, cursors
- `wisp-components` widget library — shadcn/GPUI-style Input, ListView, Badge, Kbd, Separator, ScrollArea
- `omni-daemon` crate — pure business logic, Wayland dispatch, event loop (no UI code)
- `omni-app-launcher` crate — launcher Config, OmniApp state, IconCache, draw function
- GPUI-aligned mouse tracking: `mouse_enter/move/exit/button`, `mouse_inside`, `MouseButton` enum
- GPUI-aligned scroll system: `ScrollHandle`, `ScrollDelta`, `ScrollbarState` with hover/drag/fade
- GPUI-aligned cursor system: `CursorStyle` (21 variants), `CursorManager` with theme loading and fallback
- `CursorBlink` — 500ms interval, 300ms pause after input
- `TextEditor` — grapheme-cluster cursor movement, text selection, clipboard ops
- shadcn Input: size variants (xs/sm/md/lg), prefix/suffix icons, clean button, loading spinner, disabled state, shadow, text selection, focus ring
- ListView: `ListState` wrapping `ScrollHandle`, hover tracking via `mouse_y`, active border clipped to fully visible items
- `draw_text_clipped` — text truncation with ellipsis via binary search + horizontal pixel clipping
- `stroke_rounded_rect` — rounded rectangle stroke function
- `fill_rect_clipped` — rounded rect with vertical clipping (GPUI ContentMask pattern)

### Changed
- Removed `omni-ui` crate — fully migrated to wisp + wisp-components + omni-daemon + omni-app-launcher
- Removed Scaffold — window background is a direct `fill` or `fill_rounded_rect` (GPUI-like)
- Input field: icons scaled to 28px (matches IconCache), shadcn padding_x=8, gap=8
- `ensure_visible` uses 0.5px EPSILON to prevent floating-point jitter
- Default theme changed from Catppuccin Mocha to neutral dark (`#1a1a1a` background)
- Window height snaps to `N * row_height + chrome` for clean item boundaries
- All font loading: `omni-daemon` loads system fonts (no single-font filtering)

### Fixed
- Mouse scroll fighting `ensure_selected_visible` — removed from `draw_list`, only called on selection change
- Input cursor height — changed from full container height to `0.85 * line_height`
- Input focus ring — changed from 3px outer ring to border replacement (shadcn style)
- Partial item border overflow — border only draws when item is fully visible
- Hover persisting after mouse leaves window — bounds check + `mouse_inside` flag
- Text truncation in list items — `draw_text_clipped` now truncates with ellipsis

### Removed
- `omni-ui` crate
- Scaffold component (window is a direct fill)
- `omni-ipc` crate (keybinding registration — user handles in compositor config)
- Catppuccin Mocha default theme
- `ListViewState` and `ListViewColors` (replaced by `ListState` and `ListColors`)

## [0.1.0] — 2026-05-15

Initial release. Wayland layer-shell app launcher with `tiny-skia` rendering, fuzzy search, and keyboard-first navigation.
