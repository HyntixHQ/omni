pub mod snippet;
pub mod state;
pub mod store;

pub use snippet::{expand_placeholders, Snippet};
pub use state::{
    compute_row_height, draw_snippets, handle_action, FormField, SnippetMode, SnippetsState,
    SNIPPETS_HEIGHT, SNIPPETS_WIDTH,
};
pub use store::{load_from_disk, save_to_disk, snippets_path, SNIPPETS_FILE};
