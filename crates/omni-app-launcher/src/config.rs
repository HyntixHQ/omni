use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub window: WindowConfig,
    pub theme: ThemeConfig,
    pub font: FontConfig,
    pub icons: IconConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
    #[serde(default)]
    pub wm: WmConfig,
    #[serde(default)]
    pub system: SystemConfig,
    #[serde(default)]
    pub snippets: Vec<omni_snippets::Snippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WmConfig {
    #[serde(default)]
    pub custom: Vec<omni_wm::CustomLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemConfig {
    #[serde(default)]
    pub custom: Vec<omni_system::commands::SystemEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
    pub margin: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub bg: String,
    pub fg: String,
    pub selected_bg: String,
    pub selected_fg: String,
    pub border: String,
    pub border_radius: f64,
    pub entry_bg: String,
    pub desc_fg: String,
    pub caret: String,
    pub placeholder_fg: String,
    pub badge_default_bg: String,
    pub badge_default_fg: String,
    pub badge_secondary_bg: String,
    pub badge_secondary_fg: String,
    pub badge_destructive_bg: String,
    pub badge_destructive_fg: String,
    pub badge_outline_border: String,
    pub badge_outline_fg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f64,
    pub entry_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconConfig {
    pub theme: String,
    pub cursor_theme: String,
    pub size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    #[serde(default)]
    pub launcher: Option<String>,
    #[serde(default)]
    pub calculator: Option<String>,
    #[serde(default)]
    pub clipboard: Option<String>,
    #[serde(default)]
    pub wm: Option<String>,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub snippets: Option<String>,
    #[serde(default)]
    pub help: Option<String>,
    #[serde(default)]
    pub apps: HashMap<String, String>,
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            launcher: Some("Super+Space".into()),
            calculator: Some("Super+Alt+C".into()),
            clipboard: Some("Super+V".into()),
            wm: Some("Super+Alt+W".into()),
            system: Some("Super+Alt+X".into()),
            snippets: Some("Super+Alt+S".into()),
            help: Some("F1".into()),
            apps: HashMap::new(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            bg: "#1a1a1a".into(),
            fg: "#e0e0e0".into(),
            selected_bg: "#404040".into(),
            selected_fg: "#ffffff".into(),
            border: "#333333".into(),
            border_radius: 12.0,
            entry_bg: "#1a1a1a".into(),
            desc_fg: "#888888".into(),
            caret: "#4a9eff".into(),
            placeholder_fg: "#666666".into(),
            badge_default_bg: "#333333".into(),
            badge_default_fg: "#e0e0e0".into(),
            badge_secondary_bg: "#2a2a2a".into(),
            badge_secondary_fg: "#aaaaaa".into(),
            badge_destructive_bg: "#e05a5a".into(),
            badge_destructive_fg: "#ffffff".into(),
            badge_outline_border: "#404040".into(),
            badge_outline_fg: "#e0e0e0".into(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { width: 600, height: 400, margin: 0 }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self { family: "sans-serif".into(), size: 14.0, entry_size: 14.0 }
    }
}

impl Default for IconConfig {
    fn default() -> Self {
        Self { theme: "Adwaita".into(), cursor_theme: "Adwaita".into(), size: 28 }
    }
}

pub const MAX_SEARCH_RESULTS: usize = 50;

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let user_path = format!("{}/.config/omni/config.toml", home);
        let etc_path = PathBuf::from("/etc/omni/config.toml");

        let mut cfg = Config {
            font: FontConfig {
                family: detect_system_font(),
                ..FontConfig::default()
            },
            ..Default::default()
        };

        for path in [PathBuf::from(&user_path), etc_path] {
            if let Ok(content) = std::fs::read_to_string(&path) {
                match toml::from_str::<Config>(&content) {
                    Ok(t) => {
                        cfg.apply_overrides(t);
                        tracing::info!("Loaded config from {:?}", path);
                        break;
                    }
                    Err(e) => tracing::warn!("Failed to parse config {:?}: {}", path, e),
                }
            }
        }

        cfg.icons.theme = get_system_icon_theme();
        cfg.icons.cursor_theme = get_system_cursor_theme();

        cfg
    }

    fn apply_overrides(&mut self, overrides: Config) {
        let Config { window, theme, font, icons, shortcuts, wm, system, snippets } = overrides;
        if window.width != WindowConfig::default().width { self.window.width = window.width; }
        self.window.height = window.height;
        self.window.margin = window.margin;
        self.theme = theme;
        if font.family != FontConfig::default().family { self.font.family = font.family; }
        if font.size != FontConfig::default().size { self.font.size = font.size; }
        if font.entry_size != FontConfig::default().entry_size { self.font.entry_size = font.entry_size; }
        if !icons.theme.is_empty() { self.icons.theme = icons.theme; }
        if !icons.cursor_theme.is_empty() { self.icons.cursor_theme = icons.cursor_theme; }
        self.icons.size = icons.size;
        self.shortcuts = shortcuts;
        self.wm = wm;
        self.system = system;
        self.snippets = snippets;
    }
}

fn detect_system_font() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let paths = [
        format!("{}/.config/gtk-3.0/settings.ini", home),
        format!("{}/.config/gtk-4.0/settings.ini", home),
    ];
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(family) = line.strip_prefix("gtk-font-name=") {
                    let family = family.trim().trim_matches('"');
                    if let Some((name, _)) = family.rsplit_once(' ') {
                        return name.to_string();
                    }
                    return family.to_string();
                }
            }
        }
    }
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "font-name"])
        .output()
    {
        if output.status.success() {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let name = name.trim().trim_matches('\'').trim_matches('"');
                if let Some((family, _)) = name.rsplit_once(' ') {
                    return family.to_string();
                }
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    "sans-serif".to_string()
}

fn get_system_cursor_theme() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let paths = [
        format!("{}/.config/gtk-3.0/settings.ini", home),
        format!("{}/.config/gtk-4.0/settings.ini", home),
    ];
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(theme) = line.strip_prefix("gtk-cursor-theme-name=") {
                    return theme.trim().to_string();
                }
            }
        }
    }
    std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "Adwaita".to_string())
}

fn get_system_icon_theme() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let paths = [
        format!("{}/.config/gtk-3.0/settings.ini", home),
        format!("{}/.config/gtk-4.0/settings.ini", home),
    ];
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(theme) = line.strip_prefix("gtk-icon-theme-name=") {
                    return theme.trim().to_string();
                }
            }
        }
    }
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
    {
        if output.status.success() {
            if let Ok(theme) = String::from_utf8(output.stdout) {
                let theme = theme.trim().trim_matches('\'').trim_matches('"');
                if !theme.is_empty() {
                    return theme.to_string();
                }
            }
        }
    }
    "Adwaita".to_string()
}
