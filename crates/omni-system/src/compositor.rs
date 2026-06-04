#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compositor {
    Sway,
    Hyprland,
    #[default]
    Unknown,
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
