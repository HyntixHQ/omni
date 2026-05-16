# Contributing to Omni

## Getting Started

- Rust 2024 edition
- Wayland development headers (`libwayland-dev` on Debian, `wayland` on Arch)
- A Wayland compositor with `wlr-layer-shell` support (Sway, Hyprland, River, Wayfire)

## Development

```sh
# Build debug
cargo build

# Build release
cargo build --release

# Run
target/release/omni
```

## Project Structure

```
src/main.rs              — Thin entrypoint (daemon init, event loop, draw launcher frame)
crates/
  wisp/                  — GUI framework (Wayland, draw, input, text, events, style, scroll, cursors)
  wisp-components/       — Widget library (badge, command, input, kbd, list_view, scroll_area, separator)
  omni-core/             — Data models (AppEntry, SearchResult)
  omni-search/           — Desktop file indexer + fuzzy search engine
  omni-daemon/           — Business logic: Wayland connection, dispatch, event loop, surface management
  omni-app-launcher/     — Launcher application: Config, OmniApp state, IconCache, draw function
```

## Code Style

- Follow existing patterns in the codebase
- No comments unless explaining non-obvious logic
- Handle errors properly: use `?` to propagate, `.log_err()` when discarding
- Full words for variable names (no abbreviations)
- Prefer modifying existing files over creating new ones
- Never panic with `unwrap()` — propagate errors instead

## Pull Request Process

1. Ensure `cargo clippy` passes with zero warnings
2. Keep PRs focused on a single change
3. Use imperative PR titles (e.g. "Fix crash in search", not "Fixed crash")
4. Include a `Release Notes:` section in the PR body

## Commit Messages

- Concise, imperative mood
- No conventional commit prefixes (`fix:`, `feat:`, etc.)
- No trailing punctuation

## Repository

- GitHub: [HyntixHQ/omni](https://github.com/HyntixHQ/omni)

## License

By contributing, you agree that your contributions will be licensed under the GNU General Public License v3.0.
