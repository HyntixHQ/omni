use crate::models::LauncherItem;

#[derive(Debug, Clone)]
pub enum SearchMode {
    Apps,
    Windows,
    Commands,
    All,
}

#[derive(Debug, Clone)]
pub struct LauncherState {
    pub query: String,
    pub mode: SearchMode,
    pub results: Vec<LauncherItem>,
    pub selected_index: usize,
    pub visible: bool,
}

impl LauncherState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            mode: SearchMode::All,
            results: Vec::new(),
            selected_index: 0,
            visible: false,
        }
    }
}
