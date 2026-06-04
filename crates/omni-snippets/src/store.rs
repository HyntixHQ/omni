use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::snippet::Snippet;

pub const SNIPPETS_FILE: &str = "snippets.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SnippetsFile {
    #[serde(default, rename = "snippet")]
    snippets: Vec<Snippet>,
}

pub fn snippets_path() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("omni");
    Some(dir.join(SNIPPETS_FILE))
}

pub fn exists() -> bool {
    snippets_path().is_some_and(|p| p.exists())
}

pub fn load_from_disk() -> Vec<Snippet> {
    let Some(path) = snippets_path() else { return Vec::new() };
    let Ok(content) = fs::read_to_string(&path) else { return Vec::new() };
    match toml::from_str::<SnippetsFile>(&content) {
        Ok(f) => f.snippets,
        Err(e) => {
            tracing::warn!("failed to parse {}: {e}", path.display());
            Vec::new()
        }
    }
}

pub fn save_to_disk(snippets: &[Snippet]) -> Result<(), String> {
    let Some(path) = snippets_path() else {
        return Err("no config dir available".into());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {e}"))?;
    }
    let body = toml::to_string_pretty(&SnippetsFile { snippets: snippets.to_vec() })
        .map_err(|e| format!("toml::to_string: {e}"))?;
    fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let path = snippets_path().expect("config dir");
        let original = load_from_disk();
        let sample = vec![Snippet {
            name: "Test Roundtrip".into(),
            content: "hello {date}".into(),
            shortcut: Some(";rt".into()),
        }];
        save_to_disk(&sample).expect("save");
        let loaded = load_from_disk();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Test Roundtrip");
        assert_eq!(loaded[0].shortcut.as_deref(), Some(";rt"));
        save_to_disk(&original).expect("restore");
        let _ = path;
    }
}
