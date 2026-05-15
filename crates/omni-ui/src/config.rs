use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
    pub theme: ThemeConfig,
    pub font: FontConfig,
    pub icons: IconConfig,
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
    
    // Fallback to gsettings if available
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output() {
        if output.status.success() {
            if let Ok(theme) = String::from_utf8(output.stdout) {
                let theme = theme.trim().trim_matches('\'').trim_matches('"');
                if !theme.is_empty() {
                    return theme.to_string();
                }
            }
        }
    }

    "Papirus".to_string()
}

fn get_system_font() -> (String, f64) {
    let home = std::env::var("HOME").unwrap_or_default();
    let paths = [
        format!("{}/.config/gtk-3.0/settings.ini", home),
        format!("{}/.config/gtk-4.0/settings.ini", home),
    ];

    let mut font_str = String::new();

    for path in paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(font) = line.strip_prefix("gtk-font-name=") {
                    font_str = font.trim().to_string();
                    break;
                }
            }
        }
        if !font_str.is_empty() { break; }
    }
    
    if font_str.is_empty() {
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "font-name"])
            .output() {
            if output.status.success() {
                if let Ok(font) = String::from_utf8(output.stdout) {
                    font_str = font.trim().trim_matches('\'').trim_matches('"').to_string();
                }
            }
        }
    }

    if font_str.is_empty() {
        return ("Sans".to_string(), 11.0);
    }

    // Split "Family Name Size"
    if let Some(last_space_idx) = font_str.rfind(' ') {
        let (family, size_str) = font_str.split_at(last_space_idx);
        if let Ok(size) = size_str.trim().parse::<f64>() {
            return (family.trim().to_string(), size);
        }
    }

    (font_str, 11.0)
}

impl Default for Config {
    fn default() -> Self {
        let (sys_font, sys_size) = get_system_font();
        Config {
            window: WindowConfig {
                width: 640,
                height: 410,
                margin: 0,
            },
            theme: ThemeConfig {
                bg: "#1e1e2e".into(),
                fg: "#cdd6f4".into(),
                selected_bg: "#89b4fa".into(),
                selected_fg: "#11111b".into(),
                border: "#89b4fa".into(),
                border_radius: 12.0,
                entry_bg: "#313244".into(),
                desc_fg: "#a6adc8".into(),
                caret: "#89b4fa".into(),
                placeholder_fg: "#585b70".into(),
                badge_default_bg: "#89b4fa".into(),
                badge_default_fg: "#11111b".into(),
                badge_secondary_bg: "#45475a".into(),
                badge_secondary_fg: "#cdd6f4".into(),
                badge_destructive_bg: "#f38ba8".into(),
                badge_destructive_fg: "#11111b".into(),
                badge_outline_border: "#585b70".into(),
                badge_outline_fg: "#a6adc8".into(),
            },
            font: FontConfig {
                family: sys_font,
                size: sys_size + 4.0, // Increased for better readability
                entry_size: sys_size + 6.0, // Prominent search input
            },
            icons: IconConfig {
                theme: get_system_icon_theme(),
                cursor_theme: get_system_cursor_theme(),
                size: 28,
            },
        }
    }
}

pub const MAX_SEARCH_RESULTS: usize = 20;
pub const RELAXED_FUZZY_MIN_RATIO: f64 = 0.5;

impl Config {
    pub fn load() -> Self {
        let config_paths = [
            dirs().0.join("omni/config.toml"),
            PathBuf::from("/etc/omni/config.toml"),
        ];
        for path in &config_paths {
            if path.exists() {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        match toml::from_str(&content) {
                            Ok(cfg) => return cfg,
                            Err(e) => tracing::warn!("config parse error: {e}"),
                        }
                    }
                    Err(e) => tracing::warn!("config read error: {e}"),
                }
            }
        }
        Config::default()
    }

    pub fn theme_css() -> &'static str {
        ""
    }
}

fn dirs() -> (PathBuf, PathBuf) {
    let home = std::env::var("HOME").unwrap_or_default();
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(&home).join(".config"));
    let xdg_data = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(&home).join(".local/share"));
    (xdg_config, xdg_data)
}

pub fn desktop_file_dirs() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut dirs = vec![
        "/usr/share/applications".into(),
        "/usr/local/share/applications".into(),
        "/var/lib/snapd/desktop/applications".into(),
        format!("{home}/.local/share/applications"),
        format!("{home}/.config/autostart"),
    ];
    if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for d in data_dirs.split(':') {
            let p = format!("{d}/applications");
            if !dirs.contains(&p) {
                dirs.push(p);
            }
        }
    }
    dirs
}
