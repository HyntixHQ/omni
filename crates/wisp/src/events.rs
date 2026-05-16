use crate::cursor_style::CursorStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionId {
    Header,
    Body,
    Footer,
    ScrollIndicator,
}

#[derive(Debug, Clone, Copy)]
struct Region {
    id: RegionId,
    y: f32,
    h: f32,
    cursor: CursorStyle,
}

/// Per-frame hit region registry.
/// Register rectangular regions each frame with their cursor,
/// then resolve which cursor to show based on mouse position.
///
/// Components set their cursor when registering — the framework
/// automatically picks the right cursor for the hovered region.
pub struct MouseDispatcher {
    regions: Vec<Region>,
}

impl MouseDispatcher {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }

    /// Register a region with a cursor.
    /// The framework will set this cursor when the mouse hovers this region.
    pub fn register(&mut self, id: RegionId, y: f32, h: f32, cursor: CursorStyle) {
        self.regions.push(Region { id, y, h, cursor });
    }

    /// Convenience: register body with a cursor.
    pub fn register_body(&mut self, y: f32, h: f32) {
        self.register(RegionId::Body, y, h, CursorStyle::Arrow);
    }

    /// Find which region the cursor is at, and return its cursor style.
    pub fn cursor_at(&self, y: f32) -> CursorStyle {
        self.regions
            .iter()
            .find(|r| y >= r.y && y <= r.y + r.h)
            .map(|r| r.cursor)
            .unwrap_or(CursorStyle::Arrow)
    }

    pub fn region_at(&self, y: f32, _x: f32) -> Option<RegionId> {
        self.regions
            .iter()
            .find(|r| y >= r.y && y <= r.y + r.h)
            .map(|r| r.id)
    }

    pub fn body_bounds(&self) -> Option<(f32, f32)> {
        self.regions
            .iter()
            .find(|r| r.id == RegionId::Body)
            .map(|r| (r.y, r.h))
    }
}

impl Default for MouseDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
