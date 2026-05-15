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

## Code Style

- Follow existing patterns in the codebase
- No comments unless explaining non-obvious logic
- Handle errors properly: use `?` to propagate, `.log_err()` when discarding
- Full words for variable names (no abbreviations)
- Prefer modifying existing files over creating new ones
- Never panic with `unwrap()` — propagate errors instead
- Use variable shadowing to scope clones in async contexts

## Project Structure

```
src/main.rs          — Binary entry point (Wayland event loop, SHM buffers)
crates/
  omni-core/         — Data models (AppEntry, SearchResult, etc.)
  omni-search/       — Desktop file indexer + fuzzy search engine
  omni-ui/           — Wayland shell, rendering, input handling, theming
    components/      — Reusable UI components (scaffold, list-view, badge, etc.)
    events.rs        — GPUI-inspired mouse hit region dispatcher
    render.rs        — Drawing pipeline (tiny-skia + cosmic-text)
    config.rs        — TOML configuration and system font/theme detection
    state.rs         — Application state (OmniApp)
```

## Pull Request Process

1. Ensure the build passes with no warnings
2. Keep PRs focused on a single change
3. Use imperative PR titles (e.g. "Fix crash in search", not "Fixed crash" or "fix: crash")
4. Include a `Release Notes:` section in the PR body with one bullet:
   - `- Added ...` / `- Fixed ...` / `- Improved ...` for user-facing changes
   - `- N/A` for non-user-facing changes

## Commit Messages

- Concise, imperative mood
- No conventional commit prefixes (`fix:`, `feat:`, etc.)
- No trailing punctuation

## Repository

- GitHub: [HyntixHQ/omni](https://github.com/HyntixHQ/omni)

## License

By contributing, you agree that your contributions will be licensed under the GNU General Public License v3.0.
