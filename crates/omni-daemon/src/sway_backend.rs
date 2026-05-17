use std::process::Command;

/// Register shortcuts via swaymsg bindsym.
/// Each shortcut is registered as `swaymsg bindsym <keys> exec <omni_path> --shortcut <id>`.
/// Returns true if swaymsg was available and each command succeeded.
pub fn register_shortcuts(entries: &[ShortcutEntry]) -> bool {
    let sock = match std::env::var("SWAYSOCK").or_else(|_| std::env::var("I3SOCK")) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let omni_path = std::env::current_exe().ok().map(|p| p.to_string_lossy().to_string());
    let omni_path = match omni_path {
        Some(ref p) => p.clone(),
        None => {
            tracing::warn!("Cannot determine omni executable path, skipping sway registration");
            return false;
        }
    };

    let mut all_ok = true;
    for entry in entries {
        let sway_keys = sway_normalize_keys(&entry.keys);
        let message = format!(
            "bindsym {} exec {} --shortcut {}",
            sway_keys, omni_path, entry.id
        );
        let status = Command::new("swaymsg")
            .arg(&message)
            .env("SWAYSOCK", &sock)
            .status();

        match status {
            Ok(s) if s.success() => {
                tracing::info!("Registered sway shortcut: {} -> {}", entry.keys, entry.id);
            }
            Ok(s) => {
                tracing::warn!("swaymsg failed for {} (exit: {:?}): {}", entry.keys, s.code(), entry.id);
                all_ok = false;
            }
            Err(e) => {
                tracing::warn!("swaymsg not found, skipping shortcut registration: {}", e);
                return false;
            }
        }
    }

    all_ok
}

pub struct ShortcutEntry {
    pub id: String,
    pub keys: String,
}

/// Translate common modifier names to Sway-compatible names.
/// Sway uses XKB names: Mod4=Super, Mod1=Alt, Control=Ctrl.
fn sway_normalize_keys(keys: &str) -> String {
    keys.split('+')
        .map(|part| match part {
            "Super" => "Mod4",
            "Alt" => "Mod1",
            "Ctrl" => "Control",
            other => other,
        })
        .collect::<Vec<_>>()
        .join("+")
}
