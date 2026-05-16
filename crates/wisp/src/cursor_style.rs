use wayland_client::protocol::{wl_pointer, wl_shm, wl_surface};
use wayland_client::Connection;

/// 18 cursor styles matching GPUI's `CursorStyle` enum exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Arrow,
    IBeam,
    Crosshair,
    ClosedHand,
    OpenHand,
    PointingHand,
    ResizeLeft,
    ResizeRight,
    ResizeLeftRight,
    ResizeUp,
    ResizeDown,
    ResizeUpDown,
    ResizeUpLeftDownRight,
    ResizeUpRightDownLeft,
    ResizeColumn,
    ResizeRow,
    IBeamCursorForVerticalLayout,
    OperationNotAllowed,
    DragLink,
    DragCopy,
    ContextualMenu,
}

impl CursorStyle {
    /// XCursor theme name for this cursor style.
    /// Maps GPUI-style cursor names to the standard XCursor naming convention.
    pub fn xcursor_name(self) -> &'static str {
        match self {
            CursorStyle::Arrow => "default",
            CursorStyle::IBeam => "text",
            CursorStyle::Crosshair => "crosshair",
            CursorStyle::ClosedHand => "grabbing",
            CursorStyle::OpenHand => "grab",
            CursorStyle::PointingHand => "pointer",
            CursorStyle::ResizeLeft => "w-resize",
            CursorStyle::ResizeRight => "e-resize",
            CursorStyle::ResizeLeftRight => "ew-resize",
            CursorStyle::ResizeUp => "n-resize",
            CursorStyle::ResizeDown => "s-resize",
            CursorStyle::ResizeUpDown => "ns-resize",
            CursorStyle::ResizeUpLeftDownRight => "nesw-resize",
            CursorStyle::ResizeUpRightDownLeft => "nwse-resize",
            CursorStyle::ResizeColumn => "col-resize",
            CursorStyle::ResizeRow => "row-resize",
            CursorStyle::IBeamCursorForVerticalLayout => "vertical-text",
            CursorStyle::OperationNotAllowed => "not-allowed",
            CursorStyle::DragLink => "alias",
            CursorStyle::DragCopy => "copy",
            CursorStyle::ContextualMenu => "context-menu",
        }
    }

    /// Fallback cursor name for when the preferred name is not available.
    pub fn fallback_name(self) -> &'static str {
        match self {
            CursorStyle::Arrow => "left_ptr",
            CursorStyle::IBeam => "xterm",
            CursorStyle::ClosedHand => "closedhand",
            CursorStyle::OpenHand => "openhand",
            CursorStyle::PointingHand => "hand2",
            CursorStyle::ResizeLeft => "left_side",
            CursorStyle::ResizeRight => "right_side",
            CursorStyle::ResizeLeftRight => "sb_h_double_arrow",
            CursorStyle::ResizeUp => "top_side",
            CursorStyle::ResizeDown => "bottom_side",
            CursorStyle::ResizeUpDown => "sb_v_double_arrow",
            CursorStyle::ResizeUpLeftDownRight => "fd_double_arrow",
            CursorStyle::ResizeUpRightDownLeft => "bd_double_arrow",
            CursorStyle::OperationNotAllowed => "crossed_circle",
            _ => "left_ptr",
        }
    }
}

/// Manages cursor theme loading and cursor image display.
/// Wraps `wayland_cursor::CursorTheme` and a cursor surface.
/// The cursor surface must be created by the caller (who owns the QueueHandle).
pub struct CursorManager {
    theme: Option<wayland_cursor::CursorTheme>,
    cursor_surface: Option<wl_surface::WlSurface>,
    current: Option<CursorStyle>,
    serial: u32,
}

impl CursorManager {
    /// Create a new cursor manager.
    /// The caller provides an already-created cursor surface.
    pub fn new(
        conn: &Connection,
        shm: &wl_shm::WlShm,
        cursor_surface: wl_surface::WlSurface,
        theme_name: &str,
        size: u32,
    ) -> Self {
        unsafe { std::env::set_var("XCURSOR_THEME", theme_name); }
        let theme = wayland_cursor::CursorTheme::load(conn, shm.clone(), size).ok();
        Self {
            theme,
            cursor_surface: Some(cursor_surface),
            current: None,
            serial: 0,
        }
    }

    /// Update the serial number (from pointer enter/button events).
    pub fn set_serial(&mut self, serial: u32) {
        self.serial = serial;
    }

    /// Set the cursor to the given style on the given pointer.
    /// Only changes if the style is different from the current cursor.
    pub fn set_cursor(&mut self, pointer: &wl_pointer::WlPointer, style: CursorStyle) {
        if self.current == Some(style) {
            return;
        }
        self.current = Some(style);

        let theme = match self.theme.as_mut() {
            Some(t) => t,
            None => return,
        };
        let cursor_surface = match self.cursor_surface.as_ref() {
            Some(s) => s,
            None => return,
        };

        // Try preferred name, then fallback
        let cursor_name = if theme.get_cursor(style.xcursor_name()).is_some() {
            style.xcursor_name()
        } else {
            style.fallback_name()
        };
        let cursor = theme.get_cursor(cursor_name);
        let Some(cursor_image) = cursor else { return };

        let image = &cursor_image[0];
        let (w, h) = image.dimensions();
        let (hx, hy) = image.hotspot();
        cursor_surface.attach(Some(image), 0, 0);
        cursor_surface.damage_buffer(0, 0, w as i32, h as i32);
        cursor_surface.commit();
        pointer.set_cursor(self.serial, Some(cursor_surface), hx as i32, hy as i32);
    }

    /// Reset to the default arrow cursor.
    pub fn reset(&mut self, pointer: &wl_pointer::WlPointer) {
        self.current = None;
        self.set_cursor(pointer, CursorStyle::Arrow);
    }

    /// Get the current cursor style.
    pub fn current(&self) -> Option<CursorStyle> {
        self.current
    }
}
