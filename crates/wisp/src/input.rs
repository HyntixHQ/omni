use std::time::Instant;

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
    SelectLeft,
    SelectRight,
    SelectStart,
    SelectEnd,
    SelectWordLeft,
    SelectWordRight,
    SelectAll,
    Copy,
    Cut,
    Paste,
}

// ── GPUI-aligned mouse types ─────────────────────────────

/// Mouse button matching GPUI's `MouseButton`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

// ScrollDelta lives in wisp::scroll (it's a scroll primitive)

// ── WispInput ────────────────────────────────────────────

/// Input handler — keyboard + mouse state (matches GPUI's input model).
pub struct WispInput {
    pub xkb_context: xkb::Context,
    pub xkb_keymap: Option<xkb::Keymap>,
    pub xkb_state: Option<xkb::State>,
    pub active_key: Option<u32>,
    pub last_key_time: Option<Instant>,
    pub is_repeating: bool,
    pub repeat_delay: i32,
    pub repeat_rate: i32,
    scroll_axis: f32,
    scroll_discrete: i32,
    // Mouse state (GPUI: MouseMoveEvent, MouseExitEvent)
    mouse_y: f32,
    mouse_inside: bool,
    mouse_pressed_button: Option<MouseButton>,
}

impl WispInput {
    pub fn new() -> Self {
        Self {
            xkb_context: xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
            xkb_keymap: None,
            xkb_state: None,
            active_key: None,
            last_key_time: None,
            is_repeating: false,
            repeat_delay: 200,
            repeat_rate: 25,
            scroll_axis: 0.0,
            scroll_discrete: 0,
            mouse_y: 0.0,
            mouse_inside: false,
            mouse_pressed_button: None,
        }
    }

    pub fn set_keymap(&mut self, keymap_string: &str) {
        if let Some(keymap) = xkb::Keymap::new_from_string(
            &self.xkb_context,
            keymap_string.to_string(),
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        ) {
            let state = xkb::State::new(&keymap);
            self.xkb_keymap = Some(keymap);
            self.xkb_state = Some(state);
        }
    }

    pub fn update_modifiers(
        &mut self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    ) {
        if let Some(ref mut state) = self.xkb_state {
            state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
        }
    }

    pub fn handle_key(&mut self, keycode: u32, pressed: bool) -> InputAction {
        if pressed {
            self.active_key = Some(keycode);
            self.last_key_time = Some(Instant::now());
            self.is_repeating = false;
        } else {
            if self.active_key == Some(keycode) {
                self.active_key = None;
                self.last_key_time = None;
                self.is_repeating = false;
            }
        }

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

            let ctrl = state.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE);
            let shift = state.mod_name_is_active(xkb::MOD_NAME_SHIFT, xkb::STATE_MODS_EFFECTIVE);
            let alt = state.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE);
            let super_ = state.mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE);

            let action = translate_sym(sym, ctrl, shift, alt, super_);
            if action != InputAction::None {
                return action;
            }

            if !utf8.is_empty() {
                let ch = utf8.chars().next().unwrap();
                if !ch.is_control() {
                    return InputAction::AppendChar(ch);
                }
            }
            InputAction::None
        } else if pressed {
            translate_raw(keycode)
        } else {
            InputAction::None
        }
    }

    pub fn handle_repeat(&mut self, keycode: u32) -> InputAction {
        self.handle_key(keycode, true)
    }

    pub fn keydown(&mut self, keycode: u32) -> InputAction {
        self.handle_key(keycode, true)
    }

    pub fn keyup(&mut self, keycode: u32) {
        self.handle_key(keycode, false);
    }

    /// Feed a scroll axis event (from wl_pointer::Axis).
    pub fn scroll_axis(&mut self, value: f32) {
        self.scroll_axis += value;
    }

    /// Feed a scroll discrete event (from wl_pointer::AxisDiscrete).
    pub fn scroll_discrete(&mut self, discrete: i32) {
        self.scroll_discrete += discrete;
    }

    /// Drain accumulated scroll delta, converting discrete ticks to pixels via row_height.
    /// Returns pixel delta (positive = scroll down in old convention).
    pub fn drain_scroll(&mut self, row_height: f32) -> f32 {
        let delta = if self.scroll_discrete != 0 {
            self.scroll_discrete as f32 * row_height
        } else {
            self.scroll_axis
        };
        self.scroll_axis = 0.0;
        self.scroll_discrete = 0;
        delta
    }

    // ── Mouse state (GPUI-aligned) ─────────────────────

    pub fn mouse_y(&self) -> f32 {
        self.mouse_y
    }

    pub fn mouse_inside(&self) -> bool {
        self.mouse_inside
    }

    pub fn mouse_pressed_button(&self) -> Option<MouseButton> {
        self.mouse_pressed_button
    }

    /// Called on wl_pointer::Enter (GPUI: inferred from MouseMoveEvent).
    pub fn mouse_enter(&mut self, y: f32) {
        self.mouse_inside = true;
        self.mouse_y = y;
    }

    /// Called on wl_pointer::Motion (GPUI: MouseMoveEvent).
    pub fn mouse_move(&mut self, y: f32) {
        self.mouse_y = y;
    }

    /// Called on wl_pointer::Leave (GPUI: MouseExitEvent).
    pub fn mouse_exit(&mut self) {
        self.mouse_inside = false;
        self.mouse_pressed_button = None;
    }

    /// Called on wl_pointer::Button with pressed state (GPUI: MouseDownEvent / MouseUpEvent).
    pub fn mouse_button(&mut self, button: MouseButton, pressed: bool) {
        if pressed {
            self.mouse_pressed_button = Some(button);
        } else {
            self.mouse_pressed_button = None;
        }
    }
}

impl Default for WispInput {
    fn default() -> Self {
        Self::new()
    }
}

fn translate_sym(sym: xkb::Keysym, ctrl: bool, shift: bool, _alt: bool, _super: bool) -> InputAction {
    use xkb::Keysym;
    // Clipboard actions (Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+A)
    if ctrl {
        match sym {
            Keysym::c | Keysym::C => return InputAction::Copy,
            Keysym::x | Keysym::X => return InputAction::Cut,
            Keysym::v | Keysym::V => return InputAction::Paste,
            Keysym::a | Keysym::A => return InputAction::SelectAll,
            _ => {}
        }
    }

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
        Keysym::Left | Keysym::KP_Left if shift && ctrl => InputAction::SelectWordLeft,
        Keysym::Right | Keysym::KP_Right if shift && ctrl => InputAction::SelectWordRight,
        Keysym::Left | Keysym::KP_Left if shift => InputAction::SelectLeft,
        Keysym::Right | Keysym::KP_Right if shift => InputAction::SelectRight,
        Keysym::Left | Keysym::KP_Left if ctrl => InputAction::MoveWordLeft,
        Keysym::Right | Keysym::KP_Right if ctrl => InputAction::MoveWordRight,
        Keysym::Left | Keysym::KP_Left => InputAction::MoveCursorLeft,
        Keysym::Right | Keysym::KP_Right => InputAction::MoveCursorRight,
        Keysym::Home | Keysym::KP_Home if shift => InputAction::SelectStart,
        Keysym::End | Keysym::KP_End if shift => InputAction::SelectEnd,
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
