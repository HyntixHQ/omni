//! Global shortcuts via hyprland-global-shortcuts-v1 protocol.
//! Generated bindings + Dispatch implementation.

#![allow(unused)]

use wayland_backend;
use wayland_client;
use wayland_client::protocol::__interfaces::*;
use wayland_client::{Dispatch, QueueHandle};

use crate::daemon::Inner;

wayland_scanner::generate_interfaces!("./protocols/hyprland-global-shortcuts-v1.xml");
wayland_scanner::generate_client_code!("./protocols/hyprland-global-shortcuts-v1.xml");

pub use hyprland_global_shortcuts_manager_v1::HyprlandGlobalShortcutsManagerV1;

#[derive(Debug, Clone)]
pub enum ShortcutAction {
    ShowLauncher,
    ShowCalculator,
    ShowClipboard,
    ShowWindowManager,
    ShowSystem,
    ShowSnippets,
    ShowHelp,
}

pub struct ShortcutManager {
    pub manager: Option<HyprlandGlobalShortcutsManagerV1>,
}

impl ShortcutManager {
    pub fn new() -> Self {
        Self { manager: None }
    }

    pub fn init(
        &mut self,
        mgr: &HyprlandGlobalShortcutsManagerV1,
        qh: &QueueHandle<Inner>,
    ) {
        self.manager = Some(mgr.clone());

        mgr.register_shortcut(
            "omni:launcher".to_string(),
            "omni".to_string(),
            "Open application launcher".to_string(),
            "Super+Space".to_string(),
            qh,
            ShortcutAction::ShowLauncher,
        );

        mgr.register_shortcut(
            "omni:calculator".to_string(),
            "omni".to_string(),
            "Open calculator".to_string(),
            "Super+Alt+C".to_string(),
            qh,
            ShortcutAction::ShowCalculator,
        );

        mgr.register_shortcut(
            "omni:clipboard".to_string(),
            "omni".to_string(),
            "Open clipboard manager".to_string(),
            "Super+V".to_string(),
            qh,
            ShortcutAction::ShowClipboard,
        );

        mgr.register_shortcut(
            "omni:wm".to_string(),
            "omni".to_string(),
            "Open window manager".to_string(),
            "Super+Alt+W".to_string(),
            qh,
            ShortcutAction::ShowWindowManager,
        );

        mgr.register_shortcut(
            "omni:system".to_string(),
            "omni".to_string(),
            "Open system commands".to_string(),
            "Super+Alt+X".to_string(),
            qh,
            ShortcutAction::ShowSystem,
        );

        mgr.register_shortcut(
            "omni:snippets".to_string(),
            "omni".to_string(),
            "Open snippets".to_string(),
            "Super+Alt+S".to_string(),
            qh,
            ShortcutAction::ShowSnippets,
        );

        mgr.register_shortcut(
            "omni:help".to_string(),
            "omni".to_string(),
            "Open keyboard shortcuts".to_string(),
            "F1".to_string(),
            qh,
            ShortcutAction::ShowHelp,
        );
    }
}

impl Dispatch<hyprland_global_shortcut_v1::HyprlandGlobalShortcutV1, ShortcutAction> for Inner {
    fn event(
        state: &mut Self,
        _proxy: &hyprland_global_shortcut_v1::HyprlandGlobalShortcutV1,
        event: hyprland_global_shortcut_v1::Event,
        data: &ShortcutAction,
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let hyprland_global_shortcut_v1::Event::Pressed { .. } = event {
            state.pending_shortcut = Some(data.clone());
        }
    }
}
