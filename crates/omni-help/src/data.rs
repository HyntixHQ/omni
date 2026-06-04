use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ShortcutEntry {
    pub keys_display: String,
    pub description: String,
    pub app: String,
}

impl ShortcutEntry {
    pub fn new(keys: impl Into<String>, description: impl Into<String>, app: impl Into<String>) -> Self {
        Self {
            keys_display: keys.into(),
            description: description.into(),
            app: app.into(),
        }
    }

    pub fn matches(&self, q: &str) -> bool {
        if q.is_empty() {
            return true;
        }
        let q = q.to_lowercase();
        self.keys_display.to_lowercase().contains(&q) || self.description.to_lowercase().contains(&q)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ShortcutCategory {
    pub name: String,
    pub entries: Vec<ShortcutEntry>,
}

impl ShortcutCategory {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entries: Vec::new(),
        }
    }
}

pub fn build_categories(
    global_launcher: Option<&str>,
    global_calculator: Option<&str>,
    global_clipboard: Option<&str>,
    global_wm: Option<&str>,
    global_system: Option<&str>,
    global_snippets: Option<&str>,
    global_help: Option<&str>,
    custom_apps: &[(String, String)],
) -> Vec<ShortcutCategory> {
    let mut cats = Vec::new();

    let mut global = ShortcutCategory::new("Global");
    if let Some(k) = global_launcher {
        global.entries.push(ShortcutEntry::new(k, "Open Launcher", "Launcher"));
    }
    if let Some(k) = global_calculator {
        global.entries.push(ShortcutEntry::new(k, "Open Calculator", "Calculator"));
    }
    if let Some(k) = global_clipboard {
        global.entries.push(ShortcutEntry::new(k, "Open Clipboard", "Clipboard"));
    }
    if let Some(k) = global_wm {
        global.entries.push(ShortcutEntry::new(k, "Open Window Manager", "Window Manager"));
    }
    if let Some(k) = global_system {
        global.entries.push(ShortcutEntry::new(k, "Open System Commands", "System"));
    }
    if let Some(k) = global_snippets {
        global.entries.push(ShortcutEntry::new(k, "Open Snippets", "Snippets"));
    }
    if let Some(k) = global_help {
        global.entries.push(ShortcutEntry::new(k, "Open Shortcuts", "Shortcuts"));
    }
    cats.push(global);

    if !custom_apps.is_empty() {
        let mut custom = ShortcutCategory::new("Custom Apps");
        let mut sorted: Vec<_> = custom_apps.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, k) in sorted {
            custom.entries.push(ShortcutEntry::new(k, format!("Open {id}"), id.clone()));
        }
        cats.push(custom);
    }

    let mut snip = ShortcutCategory::new("Snippets");
    snip.entries.push(ShortcutEntry::new("Ctrl+N", "New snippet", "Snippets"));
    snip.entries.push(ShortcutEntry::new("F2", "Edit selected", "Snippets"));
    snip.entries.push(ShortcutEntry::new("Delete", "Delete selected", "Snippets"));
    snip.entries.push(ShortcutEntry::new("Tab", "Next field (in form)", "Snippets"));
    snip.entries.push(ShortcutEntry::new("Shift+Tab", "Previous field (in form)", "Snippets"));
    snip.entries.push(ShortcutEntry::new("Ctrl+S", "Save (in form)", "Snippets"));
    cats.push(snip);

    let mut general = ShortcutCategory::new("General");
    general.entries.push(ShortcutEntry::new("Up Down", "Navigate list", "General"));
    general.entries.push(ShortcutEntry::new("Enter", "Select", "General"));
    general.entries.push(ShortcutEntry::new("Esc", "Close", "General"));
    general.entries.push(ShortcutEntry::new("Click", "Select item", "General"));
    general.entries.push(ShortcutEntry::new("Scroll", "Navigate list", "General"));
    cats.push(general);

    cats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt(s: &str) -> Option<&str> {
        Some(s)
    }

    #[test]
    fn build_includes_all_global() {
        let cats = build_categories(
            opt("Super+Space"),
            opt("Super+Alt+C"),
            opt("Super+V"),
            opt("Super+Alt+W"),
            opt("Super+Alt+X"),
            opt("Super+Alt+S"),
            opt("F1"),
            &[],
        );
        assert_eq!(cats[0].name, "Global");
        assert_eq!(cats[0].entries.len(), 7);
        assert_eq!(cats[0].entries[0].description, "Open Launcher");
        assert_eq!(cats[0].entries[6].description, "Open Shortcuts");
    }

    #[test]
    fn custom_apps_section_hidden_when_empty() {
        let cats = build_categories(opt("k"), None, None, None, None, None, None, &[]);
        let names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
        assert!(!names.contains(&"Custom Apps"));
    }

    #[test]
    fn custom_apps_sorted() {
        let apps = vec![
            ("zeta".into(), "Super+Alt+Z".into()),
            ("alpha".into(), "Super+Alt+A".into()),
        ];
        let cats = build_categories(None, None, None, None, None, None, None, &apps);
        let custom = cats.iter().find(|c| c.name == "Custom Apps").unwrap();
        assert_eq!(custom.entries[0].app, "alpha");
        assert_eq!(custom.entries[1].app, "zeta");
    }

    #[test]
    fn snippet_hardcoded_includes_form_keys() {
        let cats = build_categories(None, None, None, None, None, None, None, &[]);
        let snip = cats.iter().find(|c| c.name == "Snippets").unwrap();
        let keys: Vec<&str> = snip.entries.iter().map(|e| e.keys_display.as_str()).collect();
        assert!(keys.contains(&"Ctrl+N"));
        assert!(keys.contains(&"Ctrl+S"));
        assert!(keys.contains(&"Tab"));
        assert!(keys.contains(&"Shift+Tab"));
    }

    #[test]
    fn general_section_present() {
        let cats = build_categories(None, None, None, None, None, None, None, &[]);
        let general = cats.iter().find(|c| c.name == "General").unwrap();
        assert!(general.entries.len() >= 4);
    }

    #[test]
    fn entry_matches_filter() {
        let e = ShortcutEntry::new("Super+V", "Open Clipboard", "Clipboard");
        assert!(e.matches("clip"));
        assert!(e.matches("super"));
        assert!(e.matches(""));
        assert!(!e.matches("xyz"));
    }
}
