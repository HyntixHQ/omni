use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub exec: String,
    pub exec_name: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
}

impl AppEntry {
    pub fn searchable_text(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        parts.push(&self.name);
        if let Some(ref g) = self.generic_name {
            parts.push(g);
        }
        if let Some(ref d) = self.description {
            parts.push(d);
        }
        for kw in &self.keywords {
            parts.push(kw);
        }
        if let Some(ref e) = self.exec_name {
            parts.push(e);
        }
        parts.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub score: i64,
    pub matches: Vec<usize>,
    pub entry: AppEntry,
}

#[derive(Debug, Clone)]
pub struct WindowEntry {
    pub id: u64,
    pub title: String,
    pub app_name: String,
    pub workspace: u32,
}

#[derive(Debug, Clone)]
pub enum LauncherItem {
    App(AppEntry),
    Window(WindowEntry),
    Command(CommandItem),
}

#[derive(Debug, Clone)]
pub struct CommandItem {
    pub id: String,
    pub label: String,
    pub description: String,
    pub action: CommandAction,
}

#[derive(Debug, Clone)]
pub enum CommandAction {
    SystemToggle(SystemToggle),
    RunShell(String),
    Custom(String),
}

#[derive(Debug, Clone)]
pub enum SystemToggle {
    Volume,
    Brightness,
    Network,
    Bluetooth,
    Power,
}
