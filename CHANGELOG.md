# Changelog

## [0.1.0] — 2026-05-15

Initial release.

### Added

- Wayland layer-shell integration via `wlr-layer-shell-v1` protocol
- Application indexing from XDG desktop files (system and user directories)
- Multi-strategy fuzzy search engine (exact, skim, substring, word-prefix, relaxed)
- Custom CPU-based rendering pipeline using `tiny-skia` and `cosmic-text`
- shadcn-style UI component library (scaffold, list-view, badge, input-field)
- TOML configuration with Catppuccin Mocha default theme
- System font, icon theme, and cursor theme auto-detection from GTK settings
- Keyboard-first navigation with arrow keys, Ctrl+n/p, and vim-style bindings
- XKB-based keyboard input handling with US QWERTY fallback
- Wayland cursor theme support with per-region cursor shapes
- SVG/PNG icon rendering via `resvg` with LRU cache
- Desktop file field sanitization for safe command execution
- systemd user service for automatic startup
- Sway/Hyprland toggle script
- GPUI-inspired mouse hit region dispatcher
- Row-aligned list view navigation (GPUI uniform_list replica)
- Font optimization — loads only the configured family (~19 MB RSS)
