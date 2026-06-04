use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::compositor::Compositor;
use crate::preview::PreviewSpec;

#[derive(Debug, Clone)]
pub struct WmEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    pub sway_cmd: Option<String>,
    pub hypr_cmd: Option<String>,
    pub preview: Option<PreviewSpec>,
}

impl WmEntry {
    pub fn command_for(&self, c: Compositor) -> Option<&str> {
        match c {
            Compositor::Sway => self.sway_cmd.as_deref(),
            Compositor::Hyprland => self.hypr_cmd.as_deref(),
            Compositor::Unknown => None,
        }
    }

    pub fn execute(&self, c: Compositor) {
        let Some(cmd) = self.command_for(c) else { return };
        if cmd.is_empty() {
            return;
        }
        let result = Command::new("sh").args(["-c", cmd]).spawn();
        if let Err(e) = result {
            tracing::warn!("Failed to execute WM command '{}': {}", cmd, e);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomLayout {
    pub name: String,
    pub position: String,
    #[serde(default = "default_relative")]
    pub relative_width: f32,
    #[serde(default = "default_relative")]
    pub relative_height: f32,
    #[serde(default)]
    pub relative_x_offset: f32,
    #[serde(default)]
    pub relative_y_offset: f32,
}

fn default_relative() -> f32 {
    1.0
}

pub fn all_operations(custom_layouts: &[CustomLayout]) -> Vec<WmEntry> {
    let mut entries = Vec::new();

    entries.extend(window_layout_ops());
    entries.extend(focus_ops());
    entries.extend(move_ops());
    entries.extend(resize_ops());
    entries.extend(half_ops());
    entries.extend(third_ops());
    entries.extend(quarter_ops());
    entries.extend(maximize_ops());
    entries.extend(size_ops());
    entries.extend(sixth_ops());
    entries.extend(workspace_static_ops());
    entries.extend(scratchpad_ops());
    entries.extend(display_ops());
    entries.extend(display_switch_ops());
    entries.extend(custom_layouts_to_ops(custom_layouts));

    entries
}

pub fn workspace_dynamic_ops(c: Compositor, workspaces: &[u32]) -> Vec<WmEntry> {
    let mut entries = Vec::new();
    for &ws in workspaces {
        let name = format!("Workspace {}", ws);
        let desc = format!("Switch to workspace {}", ws);
        let sway = Some(format!("swaymsg workspace {}", ws));
        let hypr = Some(format!("hyprctl dispatch workspace {}", ws));
        entries.push(WmEntry {
            name,
            description: desc,
            category: "Workspace".into(),
            sway_cmd: sway,
            hypr_cmd: hypr,
            preview: None,
        });

        let name = format!("Move to Workspace {}", ws);
        let desc = format!("Move focused window to workspace {}", ws);
        let sway = Some(format!("swaymsg move container to workspace {}", ws));
        let hypr = Some(format!("hyprctl dispatch movetoworkspace {}", ws));
        entries.push(WmEntry {
            name,
            description: desc,
            category: "Workspace".into(),
            sway_cmd: sway,
            hypr_cmd: hypr,
            preview: None,
        });
    }
    if matches!(c, Compositor::Sway) {
        entries.push(WmEntry {
            name: "Next Workspace".into(),
            description: "Switch to next workspace".into(),
            category: "Workspace".into(),
            sway_cmd: Some("swaymsg workspace next".into()),
            hypr_cmd: Some("hyprctl dispatch workspace e+1".into()),
            preview: None,
        });
        entries.push(WmEntry {
            name: "Previous Workspace".into(),
            description: "Switch to previous workspace".into(),
            category: "Workspace".into(),
            sway_cmd: Some("swaymsg workspace prev".into()),
            hypr_cmd: Some("hyprctl dispatch workspace e-1".into()),
            preview: None,
        });
    } else {
        entries.push(WmEntry {
            name: "Next Workspace".into(),
            description: "Switch to next workspace".into(),
            category: "Workspace".into(),
            sway_cmd: Some("swaymsg workspace next".into()),
            hypr_cmd: Some("hyprctl dispatch workspace e+1".into()),
            preview: None,
        });
        entries.push(WmEntry {
            name: "Previous Workspace".into(),
            description: "Switch to previous workspace".into(),
            category: "Workspace".into(),
            sway_cmd: Some("swaymsg workspace prev".into()),
            hypr_cmd: Some("hyprctl dispatch workspace e-1".into()),
            preview: None,
        });
    }
    entries
}

pub fn custom_layouts_to_ops(layouts: &[CustomLayout]) -> Vec<WmEntry> {
    layouts.iter().map(|l| {
        let sway = sway_custom_cmd(l);
        let hypr = hypr_custom_cmd(l);
        let preview = custom_preview(l);
        WmEntry {
            name: l.name.clone(),
            description: format!("{} ({:.0}%×{:.0}%)", position_label(&l.position), l.relative_width * 100.0, l.relative_height * 100.0),
            category: "Custom".into(),
            sway_cmd: sway,
            hypr_cmd: hypr,
            preview,
        }
    }).collect()
}

fn sway_custom_cmd(l: &CustomLayout) -> Option<String> {
    let width_ppt = (l.relative_width * 100.0).round() as i32;
    let height_ppt = (l.relative_height * 100.0).round() as i32;
    let x_off = (l.relative_x_offset * 100.0).round() as i32;
    let y_off = (l.relative_y_offset * 100.0).round() as i32;

    let mut parts = Vec::new();
    if !l.relative_width.to_bits().eq(&1.0f32.to_bits()) {
        parts.push(format!("resize set width {}ppt", width_ppt));
    }
    if !l.relative_height.to_bits().eq(&1.0f32.to_bits()) {
        parts.push(format!("resize set height {}ppt", height_ppt));
    }
    match l.position.as_str() {
        "top-left" => {}
        "top-right" => {
            parts.push("move right".to_string());
        }
        "bottom-left" => {
            parts.push("move down".to_string());
        }
        "bottom-right" => {
            parts.push("move right".to_string());
            parts.push("move down".to_string());
        }
        "center" => {
            parts.push("move position center".to_string());
        }
        _ => {}
    }
    if x_off != 0 || y_off != 0 {
        parts.push(format!("move position {} {}", x_off, y_off));
    }
    if parts.is_empty() {
        return None;
    }
    let cmd = format!("swaymsg {}", parts.join("; "));
    Some(cmd)
}

fn hypr_custom_cmd(l: &CustomLayout) -> Option<String> {
    let w = (l.relative_width * 100.0).round() as i32;
    let h = (l.relative_height * 100.0).round() as i32;
    let mut parts = Vec::new();
    if w != 100 || h != 100 {
        parts.push(format!("resizeactive exact {}% exact {}%", w, h));
    }
    match l.position.as_str() {
        "top-left" => {}
        "top-right" => parts.push("movewindow r".to_string()),
        "bottom-left" => parts.push("movewindow d".to_string()),
        "bottom-right" => {
            parts.push("movewindow r".to_string());
            parts.push("movewindow d".to_string());
        }
        "center" => parts.push("centerwindow".to_string()),
        _ => {}
    }
    if parts.is_empty() {
        return None;
    }
    if parts.len() == 1 {
        Some(format!("hyprctl dispatch {}", parts[0]))
    } else {
        Some(format!("hyprctl dispatch --batch \"{}\"", parts.join(" ; ")))
    }
}

fn custom_preview(l: &CustomLayout) -> Option<PreviewSpec> {
    let x = match l.position.as_str() {
        "top-right" => 1.0 - l.relative_width,
        "bottom-left" => 0.0,
        "bottom-right" => 1.0 - l.relative_width,
        "center" => (1.0 - l.relative_width) / 2.0,
        _ => 0.0,
    };
    let y = match l.position.as_str() {
        "bottom-left" | "bottom-right" => 1.0 - l.relative_height,
        "center" => (1.0 - l.relative_height) / 2.0,
        _ => 0.0,
    };
    Some(PreviewSpec {
        x: x + l.relative_x_offset,
        y: y + l.relative_y_offset,
        w: l.relative_width,
        h: l.relative_height,
    })
}

fn position_label(p: &str) -> &'static str {
    match p {
        "top-left" => "Top Left",
        "top-right" => "Top Right",
        "bottom-left" => "Bottom Left",
        "bottom-right" => "Bottom Right",
        "center" => "Center",
        _ => "Custom",
    }
}

fn window_layout_ops() -> Vec<WmEntry> {
    vec![
        WmEntry {
            name: "Toggle Split".into(),
            description: "Toggle split direction".into(),
            category: "Window & Layout".into(),
            sway_cmd: Some("swaymsg split toggle".into()),
            hypr_cmd: Some("hyprctl dispatch togglesplit".into()),
            preview: None,
        },
        WmEntry {
            name: "Toggle Floating".into(),
            description: "Toggle floating mode".into(),
            category: "Window & Layout".into(),
            sway_cmd: Some("swaymsg floating toggle".into()),
            hypr_cmd: Some("hyprctl dispatch togglefloating".into()),
            preview: None,
        },
        WmEntry {
            name: "Toggle Fullscreen".into(),
            description: "Toggle fullscreen mode".into(),
            category: "Window & Layout".into(),
            sway_cmd: Some("swaymsg fullscreen toggle".into()),
            hypr_cmd: Some("hyprctl dispatch fullscreen 0".into()),
            preview: Some(PreviewSpec::full()),
        },
        WmEntry {
            name: "Toggle Tabbed Group".into(),
            description: "Toggle tabbed/stacked group".into(),
            category: "Window & Layout".into(),
            sway_cmd: Some("swaymsg layout toggle".into()),
            hypr_cmd: Some("hyprctl dispatch togglegroup".into()),
            preview: None,
        },
        WmEntry {
            name: "Kill Window".into(),
            description: "Close focused window".into(),
            category: "Window & Layout".into(),
            sway_cmd: Some("swaymsg kill".into()),
            hypr_cmd: Some("hyprctl dispatch killactive".into()),
            preview: None,
        },
        WmEntry {
            name: "Force Kill".into(),
            description: "Force close (SIGKILL) — Hyprland only".into(),
            category: "Window & Layout".into(),
            sway_cmd: None,
            hypr_cmd: Some("hyprctl dispatch forcekillactive".into()),
            preview: None,
        },
        WmEntry {
            name: "Center Window".into(),
            description: "Center floating window — Hyprland only".into(),
            category: "Window & Layout".into(),
            sway_cmd: None,
            hypr_cmd: Some("hyprctl dispatch centerwindow".into()),
            preview: Some(PreviewSpec::center()),
        },
    ]
}

fn focus_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Focus Left".into(), description: "Focus window to the left".into(), category: "Focus".into(), sway_cmd: Some("swaymsg focus left".into()), hypr_cmd: Some("hyprctl dispatch movefocus l".into()), preview: None },
        WmEntry { name: "Focus Right".into(), description: "Focus window to the right".into(), category: "Focus".into(), sway_cmd: Some("swaymsg focus right".into()), hypr_cmd: Some("hyprctl dispatch movefocus r".into()), preview: None },
        WmEntry { name: "Focus Up".into(), description: "Focus window above".into(), category: "Focus".into(), sway_cmd: Some("swaymsg focus up".into()), hypr_cmd: Some("hyprctl dispatch movefocus u".into()), preview: None },
        WmEntry { name: "Focus Down".into(), description: "Focus window below".into(), category: "Focus".into(), sway_cmd: Some("swaymsg focus down".into()), hypr_cmd: Some("hyprctl dispatch movefocus d".into()), preview: None },
    ]
}

fn move_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Move Left".into(), description: "Move window left".into(), category: "Move".into(), sway_cmd: Some("swaymsg move left".into()), hypr_cmd: Some("hyprctl dispatch movewindow l".into()), preview: None },
        WmEntry { name: "Move Right".into(), description: "Move window right".into(), category: "Move".into(), sway_cmd: Some("swaymsg move right".into()), hypr_cmd: Some("hyprctl dispatch movewindow r".into()), preview: None },
        WmEntry { name: "Move Up".into(), description: "Move window up".into(), category: "Move".into(), sway_cmd: Some("swaymsg move up".into()), hypr_cmd: Some("hyprctl dispatch movewindow u".into()), preview: None },
        WmEntry { name: "Move Down".into(), description: "Move window down".into(), category: "Move".into(), sway_cmd: Some("swaymsg move down".into()), hypr_cmd: Some("hyprctl dispatch movewindow d".into()), preview: None },
    ]
}

fn resize_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Grow Width".into(), description: "Increase width by 30px".into(), category: "Resize".into(), sway_cmd: Some("swaymsg resize grow width 30".into()), hypr_cmd: Some("hyprctl dispatch resizeactive 30 0".into()), preview: None },
        WmEntry { name: "Shrink Width".into(), description: "Decrease width by 30px".into(), category: "Resize".into(), sway_cmd: Some("swaymsg resize shrink width 30".into()), hypr_cmd: Some("hyprctl dispatch resizeactive -30 0".into()), preview: None },
        WmEntry { name: "Grow Height".into(), description: "Increase height by 30px".into(), category: "Resize".into(), sway_cmd: Some("swaymsg resize grow height 30".into()), hypr_cmd: Some("hyprctl dispatch resizeactive 0 30".into()), preview: None },
        WmEntry { name: "Shrink Height".into(), description: "Decrease height by 30px".into(), category: "Resize".into(), sway_cmd: Some("swaymsg resize shrink height 30".into()), hypr_cmd: Some("hyprctl dispatch resizeactive 0 -30".into()), preview: None },
    ]
}

fn half_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Left Half".into(), description: "Fill left half of screen".into(), category: "Halves".into(), sway_cmd: Some("swaymsg split h; resize set width 50ppt".into()), hypr_cmd: Some("hyprctl dispatch resizeactive exact 50% 0".into()), preview: Some(PreviewSpec::left_half()) },
        WmEntry { name: "Right Half".into(), description: "Fill right half of screen".into(), category: "Halves".into(), sway_cmd: Some("swaymsg split h; resize set width 50ppt; move right".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 50% 0 ; movewindow r\"".into()), preview: Some(PreviewSpec::right_half()) },
        WmEntry { name: "Top Half".into(), description: "Fill top half of screen".into(), category: "Halves".into(), sway_cmd: Some("swaymsg split v; resize set height 50ppt".into()), hypr_cmd: Some("hyprctl dispatch resizeactive 0 exact 50%".into()), preview: Some(PreviewSpec::top_half()) },
        WmEntry { name: "Bottom Half".into(), description: "Fill bottom half of screen".into(), category: "Halves".into(), sway_cmd: Some("swaymsg split v; resize set height 50ppt; move down".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive 0 exact 50% ; movewindow d\"".into()), preview: Some(PreviewSpec::bottom_half()) },
    ]
}

fn third_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "First Third".into(), description: "Fill first third horizontally".into(), category: "Thirds".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt".into()), hypr_cmd: Some("hyprctl dispatch resizeactive exact 33% 0".into()), preview: Some(PreviewSpec::left_third()) },
        WmEntry { name: "Center Third".into(), description: "Fill center third horizontally".into(), category: "Thirds".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; move right".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; movewindow r\"".into()), preview: Some(PreviewSpec::center_third()) },
        WmEntry { name: "Last Third".into(), description: "Fill last third horizontally".into(), category: "Thirds".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; move right; move right".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; movewindow r ; movewindow r\"".into()), preview: Some(PreviewSpec::right_third()) },
        WmEntry { name: "First Two Thirds".into(), description: "Fill first 2/3 horizontally".into(), category: "Thirds".into(), sway_cmd: Some("swaymsg split h; resize set width 66ppt".into()), hypr_cmd: Some("hyprctl dispatch resizeactive exact 66% 0".into()), preview: Some(PreviewSpec::two_thirds_left()) },
        WmEntry { name: "Center Two Thirds".into(), description: "Centered 2/3 horizontal".into(), category: "Thirds".into(), sway_cmd: Some("swaymsg split h; resize set width 66ppt; move right".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 66% 0 ; movewindow r\"".into()), preview: Some(PreviewSpec::two_thirds_center()) },
    ]
}

fn quarter_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Top Left Quarter".into(), description: "Top-left quarter".into(), category: "Quarters".into(), sway_cmd: Some("swaymsg split h; resize set width 50ppt; split v; resize set height 50ppt".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 50% 0 ; resizeactive 0 exact 50%\"".into()), preview: Some(PreviewSpec::top_left()) },
        WmEntry { name: "Top Right Quarter".into(), description: "Top-right quarter".into(), category: "Quarters".into(), sway_cmd: Some("swaymsg split h; resize set width 50ppt; move right; split v; resize set height 50ppt".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 50% 0 ; movewindow r ; resizeactive 0 exact 50%\"".into()), preview: Some(PreviewSpec::top_right()) },
        WmEntry { name: "Bottom Left Quarter".into(), description: "Bottom-left quarter".into(), category: "Quarters".into(), sway_cmd: Some("swaymsg split h; resize set width 50ppt; split v; resize set height 50ppt; move down".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 50% 0 ; resizeactive 0 exact 50% ; movewindow d\"".into()), preview: Some(PreviewSpec::bottom_left()) },
        WmEntry { name: "Bottom Right Quarter".into(), description: "Bottom-right quarter".into(), category: "Quarters".into(), sway_cmd: Some("swaymsg split h; resize set width 50ppt; move right; split v; resize set height 50ppt; move down".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 50% 0 ; movewindow r ; resizeactive 0 exact 50% ; movewindow d\"".into()), preview: Some(PreviewSpec::bottom_right()) },
    ]
}

fn maximize_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Maximize".into(), description: "Maximize window".into(), category: "Maximize & Center".into(), sway_cmd: Some("swaymsg fullscreen".into()), hypr_cmd: Some("hyprctl dispatch fullscreen 1".into()), preview: Some(PreviewSpec::full()) },
        WmEntry { name: "Almost Maximize".into(), description: "Resize to 90% × 90%".into(), category: "Maximize & Center".into(), sway_cmd: None, hypr_cmd: Some("hyprctl dispatch resizeactive exact 90% exact 90%".into()), preview: Some(PreviewSpec::center()) },
        WmEntry { name: "Center".into(), description: "Center the window".into(), category: "Maximize & Center".into(), sway_cmd: Some("swaymsg floating enable; move position center".into()), hypr_cmd: Some("hyprctl dispatch centerwindow".into()), preview: Some(PreviewSpec::center()) },
        WmEntry { name: "Maximize to Monitor".into(), description: "Maximize across all monitors".into(), category: "Maximize & Center".into(), sway_cmd: None, hypr_cmd: Some("hyprctl dispatch fullscreen 0 1".into()), preview: Some(PreviewSpec::full()) },
    ]
}

fn size_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Smaller".into(), description: "Shrink by 10%".into(), category: "Size".into(), sway_cmd: Some("swaymsg resize shrink width 10ppt; resize shrink height 10ppt".into()), hypr_cmd: Some("hyprctl dispatch resizeactive -10% -10%".into()), preview: Some(PreviewSpec::smaller()) },
        WmEntry { name: "Larger".into(), description: "Grow by 10%".into(), category: "Size".into(), sway_cmd: Some("swaymsg resize grow width 10ppt; resize grow height 10ppt".into()), hypr_cmd: Some("hyprctl dispatch resizeactive 10% 10%".into()), preview: Some(PreviewSpec::larger()) },
    ]
}

fn sixth_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Top Left Sixth".into(), description: "Top-left sixth".into(), category: "Sixths".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; split v; resize set height 50ppt".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; resizeactive 0 exact 50%\"".into()), preview: Some(PreviewSpec::top_left_sixth()) },
        WmEntry { name: "Top Center Sixth".into(), description: "Top-center sixth".into(), category: "Sixths".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; move right; split v; resize set height 50ppt".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; movewindow r ; resizeactive 0 exact 50%\"".into()), preview: Some(PreviewSpec::top_center_sixth()) },
        WmEntry { name: "Top Right Sixth".into(), description: "Top-right sixth".into(), category: "Sixths".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; move right; move right; split v; resize set height 50ppt".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; movewindow r ; movewindow r ; resizeactive 0 exact 50%\"".into()), preview: Some(PreviewSpec::top_right_sixth()) },
        WmEntry { name: "Bottom Left Sixth".into(), description: "Bottom-left sixth".into(), category: "Sixths".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; split v; resize set height 50ppt; move down".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; resizeactive 0 exact 50% ; movewindow d\"".into()), preview: Some(PreviewSpec::bottom_left_sixth()) },
        WmEntry { name: "Bottom Center Sixth".into(), description: "Bottom-center sixth".into(), category: "Sixths".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; move right; split v; resize set height 50ppt; move down".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; movewindow r ; resizeactive 0 exact 50% ; movewindow d\"".into()), preview: Some(PreviewSpec::bottom_center_sixth()) },
        WmEntry { name: "Bottom Right Sixth".into(), description: "Bottom-right sixth".into(), category: "Sixths".into(), sway_cmd: Some("swaymsg split h; resize set width 33ppt; move right; move right; split v; resize set height 50ppt; move down".into()), hypr_cmd: Some("hyprctl dispatch --batch \"resizeactive exact 33% 0 ; movewindow r ; movewindow r ; resizeactive 0 exact 50% ; movewindow d\"".into()), preview: Some(PreviewSpec::bottom_right_sixth()) },
    ]
}

fn workspace_static_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Next Workspace".into(), description: "Switch to next workspace".into(), category: "Workspace".into(), sway_cmd: Some("swaymsg workspace next".into()), hypr_cmd: Some("hyprctl dispatch workspace e+1".into()), preview: None },
        WmEntry { name: "Previous Workspace".into(), description: "Switch to previous workspace".into(), category: "Workspace".into(), sway_cmd: Some("swaymsg workspace prev".into()), hypr_cmd: Some("hyprctl dispatch workspace e-1".into()), preview: None },
    ]
}

fn scratchpad_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Show Scratchpad".into(), description: "Show the scratchpad".into(), category: "Scratchpad".into(), sway_cmd: Some("swaymsg scratchpad show".into()), hypr_cmd: Some("hyprctl dispatch togglespecialworkspace".into()), preview: None },
        WmEntry { name: "Move to Scratchpad".into(), description: "Move focused window to scratchpad".into(), category: "Scratchpad".into(), sway_cmd: Some("swaymsg move scratchpad".into()), hypr_cmd: Some("hyprctl dispatch movetoworkspacesilent special".into()), preview: None },
    ]
}

fn display_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Screens Off".into(), description: "Turn off all screens".into(), category: "Display".into(), sway_cmd: Some("swaymsg output * power off".into()), hypr_cmd: Some("hyprctl dispatch dpms off".into()), preview: None },
        WmEntry { name: "Screens On".into(), description: "Turn on all screens".into(), category: "Display".into(), sway_cmd: Some("swaymsg output * power on".into()), hypr_cmd: Some("hyprctl dispatch dpms on".into()), preview: None },
        WmEntry { name: "Toggle Screens".into(), description: "Toggle screen power — Hyprland only".into(), category: "Display".into(), sway_cmd: None, hypr_cmd: Some("hyprctl dispatch dpms toggle".into()), preview: None },
    ]
}

fn display_switch_ops() -> Vec<WmEntry> {
    vec![
        WmEntry { name: "Next Display".into(), description: "Send window to next monitor".into(), category: "Display Switch".into(), sway_cmd: Some("swaymsg move container to output right; focus output right".into()), hypr_cmd: Some("hyprctl dispatch movewindow mon:+1".into()), preview: None },
        WmEntry { name: "Previous Display".into(), description: "Send window to previous monitor".into(), category: "Display Switch".into(), sway_cmd: Some("swaymsg move container to output left; focus output left".into()), hypr_cmd: Some("hyprctl dispatch movewindow mon:-1".into()), preview: None },
    ]
}
