pub mod compositor;
pub mod ops;
pub mod preview;
pub mod state;

pub use compositor::{detect_compositor, query_monitors, query_workspaces, Compositor, Monitor};
pub use ops::{all_operations, CustomLayout, WmEntry};
pub use preview::{draw_preview, PreviewColors, PreviewSpec};
pub use state::{draw_wm, handle_action, WmState, WM_HEIGHT, WM_WIDTH};
