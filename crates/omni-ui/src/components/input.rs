use xkbcommon::xkb;

/// Actions produced by processing a keyboard event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    None,
    AppendChar(char),
    Backspace,
    Delete,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorStart,
    MoveCursorEnd,
    DeleteWordBackward,
    DeleteWordForward,
    MoveWordLeft,
    MoveWordRight,
    SelectPrev,
    SelectNext,
    Confirm,
    Cancel,
}

/// Centralized keyboard input handler supporting both xkb and raw evdev fallback.
pub struct InputHandler {
    pub xkb_context: xkb::Context,
    pub xkb_keymap: Option<xkb::Keymap>,
    pub xkb_state: Option<xkb::State>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            xkb_context: xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
            xkb_keymap: None,
            xkb_state: None,
        }
    }

    /// Load a keymap from a keymap string (e.g. from the compositor's wl_keyboard.keymap event).
    pub fn set_keymap(&mut self, keymap_string: &str) {
        tracing::info!("Setting keymap, string length={}", keymap_string.len());
        if let Some(keymap) = xkb::Keymap::new_from_string(
            &self.xkb_context,
            keymap_string.to_string(),
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        ) {
            let state = xkb::State::new(&keymap);
            self.xkb_keymap = Some(keymap);
            self.xkb_state = Some(state);
            tracing::info!("XKB state initialized successfully");
        } else {
            tracing::error!("Failed to create XKB keymap from string");
        }
    }

    /// Update the xkb state with modifiers from Wayland.
    pub fn update_modifiers(
        &mut self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    ) {
        if let Some(ref mut state) = self.xkb_state {
            let component = state.update_mask(
                mods_depressed,
                mods_latched,
                mods_locked,
                0,
                0,
                group,
            );
            tracing::debug!("XKB state update mask: component_bits={:x}", component);
        }
    }

    /// Process a keyboard event.
    /// `keycode` is the raw evdev keycode, `pressed` is true for key-down events.
    pub fn handle_key(&mut self, keycode: u32, pressed: bool) -> InputAction {
        if let Some(ref mut state) = self.xkb_state {
            let kc = (keycode + 8).into();
            let dir = if pressed {
                xkb::KeyDirection::Down
            } else {
                xkb::KeyDirection::Up
            };
            state.update_key(kc, dir);
            if !pressed {
                return InputAction::None;
            }

            let sym = state.key_get_one_sym(kc);
            let utf8 = state.key_get_utf8(kc);
            tracing::debug!("Key: {:?} (keycode={}) -> sym={:?} ({}), utf8='{}'", kc, keycode, sym, xkb::keysym_get_name(sym), utf8);

            // Check modifiers for navigation/editing
            let ctrl = state.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE);

            // 1. Try to handle special keys (Confirm, Cancel, Backspace, etc.)
            let action = translate_sym(sym, ctrl);
            if action != InputAction::None {
                return action;
            }

            // 2. If it's not a special key, get the UTF-8 representation from XKB state
            // This correctly handles Shift, Caps Lock, AltGr, etc.
            if !utf8.is_empty() {
                let ch = utf8.chars().next().unwrap();
                // Only produce AppendChar for non-control characters
                if !ch.is_control() {
                    return InputAction::AppendChar(ch);
                }
            }
            
            InputAction::None
        } else {
            if !pressed {
                return InputAction::None;
            }
            tracing::warn!("XKB state not initialized, using hardcoded fallback for keycode {}", keycode);
            translate_raw(keycode)
        }
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

fn translate_sym(sym: xkb::Keysym, ctrl: bool) -> InputAction {
    use xkb::Keysym;
    match sym {
        Keysym::Return | Keysym::KP_Enter | Keysym::ISO_Enter => InputAction::Confirm,
        Keysym::Escape => InputAction::Cancel,
        Keysym::BackSpace => {
            if ctrl {
                InputAction::DeleteWordBackward
            } else {
                InputAction::Backspace
            }
        }
        Keysym::Delete | Keysym::KP_Delete => {
            if ctrl {
                InputAction::DeleteWordForward
            } else {
                InputAction::Delete
            }
        }
        Keysym::Left | Keysym::KP_Left => {
            if ctrl {
                InputAction::MoveWordLeft
            } else {
                InputAction::MoveCursorLeft
            }
        }
        Keysym::Right | Keysym::KP_Right => {
            if ctrl {
                InputAction::MoveWordRight
            } else {
                InputAction::MoveCursorRight
            }
        }
        Keysym::Home | Keysym::KP_Home => InputAction::MoveCursorStart,
        Keysym::End | Keysym::KP_End => InputAction::MoveCursorEnd,
        Keysym::Up | Keysym::KP_Up | Keysym::p | Keysym::P if ctrl => InputAction::SelectPrev,
        Keysym::Down | Keysym::KP_Down | Keysym::n | Keysym::N if ctrl => InputAction::SelectNext,
        Keysym::Up | Keysym::KP_Up => InputAction::SelectPrev,
        Keysym::Down | Keysym::KP_Down => InputAction::SelectNext,
        _ => InputAction::None,
    }
}

fn translate_raw(keycode: u32) -> InputAction {
    match keycode {
        1 => InputAction::Cancel,
        14 => InputAction::Backspace,
        28 | 96 => InputAction::Confirm,
        103 => InputAction::SelectPrev,
        108 => InputAction::SelectNext,
        _ => keycode_to_ascii(keycode)
            .map(InputAction::AppendChar)
            .unwrap_or(InputAction::None),
    }
}

/// US QWERTY evdev keycode → ASCII lookup (fallback when no xkb keymap is available).
fn keycode_to_ascii(keycode: u32) -> Option<char> {
    if keycode >= 2 && keycode <= 13 {
        let s = b"1234567890-_=";
        return s.get((keycode - 2) as usize).map(|&c| c as char);
    }
    if keycode >= 16 && keycode <= 26 {
        let s = b"qwertyuiop[]\\";
        return s.get((keycode - 16) as usize).map(|&c| c as char);
    }
    if keycode >= 30 && keycode <= 40 {
        let s = b"asdfghjkl;'";
        return s.get((keycode - 30) as usize).map(|&c| c as char);
    }
    if keycode >= 44 && keycode <= 50 {
        let s = b"zxcvbnm,./";
        return s.get((keycode - 44) as usize).map(|&c| c as char);
    }
    if keycode == 57 {
        return Some(' ');
    }
    if keycode == 15 {
        return Some('\t');
    }
    None
}
