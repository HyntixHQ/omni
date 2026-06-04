use serde::{Deserialize, Serialize};

use crate::compositor::Compositor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntry {
    pub name: String,
    pub description: String,
    pub command: String,
    pub category: String,
    #[serde(default)]
    pub destructive: bool,
}

impl SystemEntry {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        command: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            command: command.into(),
            category: category.into(),
            destructive: false,
        }
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn execute(&self) {
        tracing::info!(command = %self.command, "executing system command: {}", self.name);
        let _ = std::process::Command::new("sh")
            .args(["-c", &self.command])
            .spawn();
    }
}

#[allow(clippy::vec_init_then_push)]
pub fn preset_commands(compositor: Compositor) -> Vec<SystemEntry> {
    let mut entries: Vec<SystemEntry> = Vec::new();

    entries.push(SystemEntry::new(
        "Lock Screen",
        "Lock the current session",
        "loginctl lock-session",
        "Session",
    ));
    entries.push(SystemEntry::new(
        "Logout",
        "Exit the compositor and end the session",
        match compositor {
            Compositor::Sway => "swaymsg exit",
            Compositor::Hyprland => "hyprctl dispatch exit",
            Compositor::Unknown => "loginctl terminate-user $USER",
        },
        "Session",
    ));
    entries.push(SystemEntry::new(
        "Suspend",
        "Suspend to RAM",
        "systemctl suspend",
        "Session",
    ));
    entries.push(SystemEntry::new(
        "Hibernate",
        "Hibernate to disk",
        "systemctl hibernate",
        "Session",
    ));
    entries.push(SystemEntry::new(
        "Hybrid Sleep",
        "Suspend then hibernate on low battery",
        "systemctl hybrid-sleep",
        "Session",
    ));
    entries.push(SystemEntry::new(
        "Suspend-Then-Hibernate",
        "Suspend now, hibernate on wake",
        "systemctl suspend-then-hibernate",
        "Session",
    ));

    entries.push(SystemEntry::new(
        "Shutdown",
        "Power off the system",
        "systemctl poweroff",
        "Power",
    ).destructive());
    entries.push(SystemEntry::new(
        "Reboot",
        "Restart the system",
        "systemctl reboot",
        "Power",
    ).destructive());
    entries.push(SystemEntry::new(
        "Reboot to Firmware",
        "Restart into BIOS/UEFI setup",
        "systemctl reboot --firmware-setup",
        "Power",
    ).destructive());
    entries.push(SystemEntry::new(
        "Reboot to Recovery",
        "Restart into recovery mode",
        "systemctl reboot --recovery",
        "Power",
    ).destructive());

    entries.push(SystemEntry::new(
        "Turn Off Screens",
        "Power off all displays",
        match compositor {
            Compositor::Sway => "swaymsg output * power off",
            Compositor::Hyprland => "hyprctl dispatch dpms off",
            Compositor::Unknown => "",
        },
        "Display",
    ));
    entries.push(SystemEntry::new(
        "Turn On Screens",
        "Power on all displays",
        match compositor {
            Compositor::Sway => "swaymsg output * power on",
            Compositor::Hyprland => "hyprctl dispatch dpms on",
            Compositor::Unknown => "",
        },
        "Display",
    ));
    entries.push(SystemEntry::new(
        "Toggle Screens",
        "Toggle display power",
        match compositor {
            Compositor::Sway => "swaymsg output * power toggle",
            Compositor::Hyprland => "hyprctl dispatch dpms toggle",
            Compositor::Unknown => "",
        },
        "Display",
    ));

    entries.push(SystemEntry::new(
        "Volume Up",
        "Increase speaker volume by 5%",
        "pactl set-sink-volume @DEFAULT_SINK@ +5%",
        "Audio",
    ));
    entries.push(SystemEntry::new(
        "Volume Down",
        "Decrease speaker volume by 5%",
        "pactl set-sink-volume @DEFAULT_SINK@ -5%",
        "Audio",
    ));
    entries.push(SystemEntry::new(
        "Mute",
        "Mute the default speaker",
        "pactl set-sink-mute @DEFAULT_SINK@ 1",
        "Audio",
    ));
    entries.push(SystemEntry::new(
        "Unmute",
        "Unmute the default speaker",
        "pactl set-sink-mute @DEFAULT_SINK@ 0",
        "Audio",
    ));
    entries.push(SystemEntry::new(
        "Toggle Mute",
        "Toggle the default speaker mute",
        "pactl set-sink-mute @DEFAULT_SINK@ toggle",
        "Audio",
    ));
    entries.push(SystemEntry::new(
        "Mic Mute",
        "Mute the default microphone",
        "pactl set-source-mute @DEFAULT_SOURCE@ 1",
        "Audio",
    ));
    entries.push(SystemEntry::new(
        "Mic Unmute",
        "Unmute the default microphone",
        "pactl set-source-mute @DEFAULT_SOURCE@ 0",
        "Audio",
    ));
    entries.push(SystemEntry::new(
        "Toggle Mic",
        "Toggle the default microphone mute",
        "pactl set-source-mute @DEFAULT_SOURCE@ toggle",
        "Audio",
    ));

    entries.push(SystemEntry::new(
        "Brightness Up",
        "Increase display brightness by 5%",
        "brightnessctl set +5%",
        "Brightness",
    ));
    entries.push(SystemEntry::new(
        "Brightness Down",
        "Decrease display brightness by 5%",
        "brightnessctl set 5%-",
        "Brightness",
    ));
    entries.push(SystemEntry::new(
        "Brightness 25%",
        "Set brightness to 25%",
        "brightnessctl set 25%",
        "Brightness",
    ));
    entries.push(SystemEntry::new(
        "Brightness 50%",
        "Set brightness to 50%",
        "brightnessctl set 50%",
        "Brightness",
    ));
    entries.push(SystemEntry::new(
        "Brightness 75%",
        "Set brightness to 75%",
        "brightnessctl set 75%",
        "Brightness",
    ));
    entries.push(SystemEntry::new(
        "Brightness 100%",
        "Set brightness to 100%",
        "brightnessctl set 100%",
        "Brightness",
    ));

    entries.push(SystemEntry::new(
        "Bluetooth On",
        "Turn Bluetooth adapter on",
        "bluetoothctl power on",
        "Bluetooth",
    ));
    entries.push(SystemEntry::new(
        "Bluetooth Off",
        "Turn Bluetooth adapter off",
        "bluetoothctl power off",
        "Bluetooth",
    ));
    entries.push(SystemEntry::new(
        "Toggle Bluetooth",
        "Toggle Bluetooth power",
        "bluetoothctl power toggle",
        "Bluetooth",
    ));
    entries.push(SystemEntry::new(
        "Bluetooth Scan",
        "Start scanning for devices",
        "bluetoothctl scan on",
        "Bluetooth",
    ));
    entries.push(SystemEntry::new(
        "Bluetooth Stop Scan",
        "Stop scanning for devices",
        "bluetoothctl scan off",
        "Bluetooth",
    ));
    entries.push(SystemEntry::new(
        "Show Bluetooth",
        "Show Bluetooth adapter info",
        "bluetoothctl show",
        "Bluetooth",
    ));

    entries.push(SystemEntry::new(
        "Wi-Fi On",
        "Enable Wi-Fi radio",
        "rfkill unblock wifi",
        "Network",
    ));
    entries.push(SystemEntry::new(
        "Wi-Fi Off",
        "Disable Wi-Fi radio",
        "rfkill block wifi",
        "Network",
    ));
    entries.push(SystemEntry::new(
        "Toggle Wi-Fi",
        "Toggle Wi-Fi radio",
        "rfkill toggle wifi",
        "Network",
    ));
    entries.push(SystemEntry::new(
        "Show Wi-Fi",
        "Show Wi-Fi station info",
        "iwctl station wlan0 show",
        "Network",
    ));

    entries
}
