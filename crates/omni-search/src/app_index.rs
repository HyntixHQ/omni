use omni_core::models::AppEntry;

pub struct AppIndex {
    apps: Vec<AppEntry>,
}

impl AppIndex {
    pub fn new() -> Self {
        Self { apps: Vec::new() }
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        self.apps = self.scan_desktop_files()?;
        tracing::info!("Indexed {} applications", self.apps.len());
        Ok(())
    }

    pub fn apps(&self) -> &[AppEntry] {
        &self.apps
    }

    fn scan_desktop_files(&self) -> anyhow::Result<Vec<AppEntry>> {
        let mut apps = Vec::new();
        let home = std::env::var("HOME").unwrap_or_default();
        let dirs = [
            "/usr/share/applications",
            "/usr/local/share/applications",
            "/var/lib/snapd/desktop/applications",
            &format!("{home}/.local/share/applications"),
            &format!("{home}/.config/autostart"),
        ];

        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "desktop") {
                        if let Ok(app) = parse_desktop_file(&path) {
                            apps.push(app);
                        }
                    }
                }
            }
        }

        apps.sort_by(|a, b| a.name.cmp(&b.name));
        apps.dedup_by(|a, b| a.id == b.id);
        Ok(apps)
    }
}

fn extract_exec_name(exec: &str) -> Option<String> {
    let trimmed = exec.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first = trimmed.split_whitespace().next()?;
    let name = std::path::Path::new(first)
        .file_name()?
        .to_str()?
        .to_string();
    Some(name)
}

fn parse_desktop_file(path: &std::path::Path) -> anyhow::Result<AppEntry> {
    use std::io::BufRead;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    let mut name = String::new();
    let mut generic_name = None;
    let mut description = None;
    let mut exec = String::new();
    let mut icon = None;
    let mut categories = Vec::new();
    let mut keywords = Vec::new();
    let mut no_display = false;

    let in_desktop_entry = std::cell::Cell::new(false);

    for line in reader.lines().flatten() {
        let line = line.trim().to_string();
        if line == "[Desktop Entry]" {
            in_desktop_entry.set(true);
            continue;
        }
        if line.starts_with('[') {
            in_desktop_entry.set(false);
            continue;
        }

        if in_desktop_entry.get() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "Name" if name.is_empty() => name = value.to_string(),
                    "GenericName" if generic_name.is_none() => {
                        generic_name = Some(value.to_string())
                    }
                    "Comment" if description.is_none() => {
                        description = Some(value.to_string())
                    }
                    "Exec" => exec = value.to_string(),
                    "Icon" => icon = Some(value.to_string()),
                    "Categories" => {
                        categories = value.split(';').map(|s| s.to_string()).collect();
                    }
                    "Keywords" => {
                        keywords = value.split(';').map(|s| s.to_string()).collect();
                    }
                    "NoDisplay" => no_display = value == "true",
                    _ => {}
                }
            }
        }
    }

    if name.is_empty() || no_display {
        anyhow::bail!("Skipping invalid or hidden desktop entry");
    }

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&name)
        .to_string();

    let exec_name = extract_exec_name(&exec);

    Ok(AppEntry {
        id,
        name,
        generic_name,
        description,
        keywords,
        exec,
        exec_name,
        icon,
        categories,
    })
}
