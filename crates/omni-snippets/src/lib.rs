pub mod snippet;
pub mod state;

pub use snippet::{expand_placeholders, Snippet};
pub use state::{compute_row_height, draw_snippets, handle_action, SnippetsState, SNIPPETS_HEIGHT, SNIPPETS_WIDTH};
