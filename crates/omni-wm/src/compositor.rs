use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compositor {
    Sway,
    Hyprland,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub id: u32,
    pub name: String,
}

pub fn detect_compositor() -> Compositor {
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        Compositor::Hyprland
    } else if std::env::var("SWAYSOCK").is_ok() {
        Compositor::Sway
    } else {
        Compositor::Unknown
    }
}

pub fn compositor_name(c: Compositor) -> &'static str {
    match c {
        Compositor::Sway => "Sway",
        Compositor::Hyprland => "Hyprland",
        Compositor::Unknown => "Unknown",
    }
}

pub fn command_for(c: Compositor) -> &'static str {
    match c {
        Compositor::Sway => "swaymsg",
        Compositor::Hyprland => "hyprctl",
        Compositor::Unknown => "",
    }
}

pub fn query_workspaces(c: Compositor) -> Vec<u32> {
    let output = match c {
        Compositor::Sway => Command::new("swaymsg").args(["-t", "get_workspaces"]).output().ok(),
        Compositor::Hyprland => Command::new("hyprctl").args(["workspaces", "-j"]).output().ok(),
        Compositor::Unknown => return Vec::new(),
    };
    let Some(out) = output else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let nums: Vec<u32> = match c {
        Compositor::Sway => {
            let mut v = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("\"num\"") {
                    if let Some(num_str) = rest.split(':').nth(1) {
                        if let Ok(n) = num_str.trim().trim_matches(',').parse::<u32>() {
                            v.push(n);
                        }
                    }
                }
            }
            v
        }
        Compositor::Hyprland => {
            let mut v = Vec::new();
            let mut depth = 0i32;
            let mut current = String::new();
            for ch in text.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        current.clear();
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(idx) = current.find("\"id\"") {
                                let after = &current[idx + 4..];
                                if let Some(colon) = after.find(':') {
                                    let num_str = after[colon + 1..].trim().split(|c: char| !c.is_ascii_digit()).next().unwrap_or("");
                                    if let Ok(n) = num_str.parse::<i64>() {
                                        if n >= 0 {
                                            v.push(n as u32);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ if depth == 1 => current.push(ch),
                    _ => {}
                }
            }
            v
        }
        Compositor::Unknown => Vec::new(),
    };
    let mut nums = nums;
    nums.sort();
    nums.dedup();
    nums
}

pub fn query_monitors(c: Compositor) -> Vec<Monitor> {
    let output = match c {
        Compositor::Sway => Command::new("swaymsg").args(["-t", "get_outputs"]).output().ok(),
        Compositor::Hyprland => Command::new("hyprctl").args(["monitors", "-j"]).output().ok(),
        Compositor::Unknown => return Vec::new(),
    };
    let Some(out) = output else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    match c {
        Compositor::Sway => {
            let mut monitors = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("\"name\"") {
                    if let Some(colon) = rest.find(':') {
                        let name = rest[colon + 1..].trim().trim_matches(',').trim_matches('"').to_string();
                        if !name.is_empty() {
                            let id = monitors.len() as u32 + 1;
                            monitors.push(Monitor { id, name });
                        }
                    }
                }
            }
            monitors
        }
        Compositor::Hyprland => {
            let mut monitors = Vec::new();
            let mut depth = 0i32;
            let mut current_name = String::new();
            let mut current_id: Option<u32> = None;
            for ch in text.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        current_name.clear();
                        current_id = None;
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(name) = current_name.strip_prefix("\"").and_then(|s| s.split('"').next()).map(|s| s.to_string()) {
                                let id = current_id.unwrap_or(monitors.len() as u32 + 1);
                                monitors.push(Monitor { id, name });
                            }
                        }
                    }
                    _ if depth == 1 => {
                        current_name.push(ch);
                        if current_name.ends_with("\"name\"") {
                            let after = &current_name[6..];
                            if let Some(q1) = after.find('"') {
                                if let Some(q2) = after[q1 + 1..].find('"') {
                                    let name = &after[q1 + 1..q1 + 1 + q2];
                                    if !name.is_empty() {
                                        let id = monitors.len() as u32 + 1;
                                        current_id = Some(id);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            monitors
        }
        Compositor::Unknown => Vec::new(),
    }
}
