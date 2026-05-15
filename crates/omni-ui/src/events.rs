use crate::components::scaffold::LayoutRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionId {
    Header,
    Body,
    Footer,
}

#[derive(Debug, Clone, Copy)]
pub struct HitRegion {
    pub id: RegionId,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Per-frame hit region dispatcher inspired by GPUI's hitbox model.
/// Components register regions during painting; mouse events hit-test against them.
#[derive(Debug, Clone)]
pub struct MouseDispatcher {
    regions: Vec<HitRegion>,
}

impl MouseDispatcher {
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    /// Register regions for the current frame. Clears previous frame's regions.
    pub fn set_regions(
        &mut self,
        header: Option<LayoutRect>,
        body: LayoutRect,
        footer: Option<LayoutRect>,
    ) {
        self.regions.clear();
        if let Some(h) = header {
            self.regions.push(HitRegion {
                id: RegionId::Header,
                x: h.x,
                y: h.y,
                w: h.w,
                h: h.h,
            });
        }
        self.regions.push(HitRegion {
            id: RegionId::Body,
            x: body.x,
            y: body.y,
            w: body.w,
            h: body.h,
        });
        if let Some(f) = footer {
            self.regions.push(HitRegion {
                id: RegionId::Footer,
                x: f.x,
                y: f.y,
                w: f.w,
                h: f.h,
            });
        }
    }

    /// Returns the topmost region at the given position (back-to-front order).
    /// Layout rects are non-overlapping for header/body/footer, so this
    /// effectively returns the single active region.
    pub fn region_at(&self, x: f32, y: f32) -> Option<RegionId> {
        for r in self.regions.iter().rev() {
            if x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h {
                return Some(r.id);
            }
        }
        None
    }

    /// Returns the body region bounds (top y, height) for list item hit-testing.
    pub fn body_bounds(&self) -> Option<(f32, f32)> {
        self.regions
            .iter()
            .find(|r| r.id == RegionId::Body)
            .map(|r| (r.y, r.h))
    }
}
