use std::time::Instant;

// ── Scroll Delta ──────────────────────────────────────────

/// Scroll delta matching GPUI's `ScrollDelta`.
#[derive(Debug, Clone, Copy)]
pub enum ScrollDelta {
    /// Precise pixel delta (trackpad).
    Pixels(f32),
    /// Inexact line delta (mouse wheel).
    Lines(f32),
}

impl ScrollDelta {
    pub fn is_precise(&self) -> bool {
        matches!(self, ScrollDelta::Pixels(_))
    }

    /// Convert to pixel delta using the given line height.
    /// GPUI: `ScrollDelta::pixel_delta(line_height)`
    pub fn pixel_delta(&self, line_height: f32) -> f32 {
        match self {
            ScrollDelta::Pixels(d) => *d,
            ScrollDelta::Lines(d) => d * line_height,
        }
    }

    pub fn coalesce(self, other: ScrollDelta) -> ScrollDelta {
        match (self, other) {
            (ScrollDelta::Pixels(a), ScrollDelta::Pixels(b)) => {
                ScrollDelta::Pixels(if a.signum() == b.signum() { a + b } else { b })
            }
            (ScrollDelta::Lines(a), ScrollDelta::Lines(b)) => {
                ScrollDelta::Lines(if a.signum() == b.signum() { a + b } else { b })
            }
            _ => other,
        }
    }
}

// ── Scroll Handle ──────────────────────────────────────────

/// Scroll state matching GPUI's `ScrollHandle`.
/// `offset` is 0 at top, negative when scrolled down.
#[derive(Debug, Clone)]
pub struct ScrollHandle {
    offset: f32,
    viewport_size: f32,
    content_size: f32,
    line_height: f32,
    min_thumb_size: f32,
}

impl ScrollHandle {
    pub fn new() -> Self {
        Self {
            offset: 0.0,
            viewport_size: 0.0,
            content_size: 0.0,
            line_height: 20.0,
            min_thumb_size: 48.0,
        }
    }

    pub fn offset(&self) -> f32 {
        self.offset
    }

    pub fn set_offset(&mut self, offset: f32) {
        self.offset = offset.clamp(self.scroll_min(), self.scroll_max());
    }

    pub fn scroll_by(&mut self, delta: f32) {
        self.set_offset(self.offset + delta);
    }

    pub fn scroll_min(&self) -> f32 {
        -(self.content_size - self.viewport_size).max(0.0)
    }

    pub fn scroll_max(&self) -> f32 {
        0.0
    }

    pub fn max_scroll(&self) -> f32 {
        (self.content_size - self.viewport_size).max(0.0)
    }

    pub fn set_viewport(&mut self, viewport_size: f32) {
        self.viewport_size = viewport_size;
        self.clamp_offset();
    }

    pub fn set_content(&mut self, content_size: f32) {
        self.content_size = content_size;
        self.clamp_offset();
    }

    pub fn viewport_size(&self) -> f32 {
        self.viewport_size
    }

    pub fn content_size(&self) -> f32 {
        self.content_size
    }

    pub fn set_line_height(&mut self, line_height: f32) {
        self.line_height = line_height;
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn visible_ratio(&self) -> f32 {
        if self.content_size <= 0.0 {
            return 1.0;
        }
        (self.viewport_size / self.content_size).min(1.0)
    }

    pub fn is_scrollable(&self) -> bool {
        self.content_size > self.viewport_size
    }

    // ── Thumb layout (matching GPUI scrollbar) ──

    /// Thumb size: `max(container * visible_ratio, min_thumb_size)`
    pub fn thumb_size(&self) -> f32 {
        let ratio = self.visible_ratio();
        (self.viewport_size * ratio).max(self.min_thumb_size)
    }

    /// Thumb position as offset from track top (0..viewport-thumb).
    /// `-(scroll_offset / max_scroll) * (viewport - thumb)`
    pub fn thumb_offset(&self) -> f32 {
        let max = self.max_scroll();
        if max <= 0.0 {
            return 0.0;
        }
        (-self.offset / max) * (self.viewport_size - self.thumb_size())
    }

    // ── Scroll to item (matching GPUI ScrollStrategy) ──

    /// Scroll the minimal amount to make the item fully visible.
    /// `item_start` and `item_end` are relative to the content.
    /// GPUI equivalent: `scroll_to_item()` with `ScrollStrategy::FirstVisible`
    /// Uses EPSILON to avoid jitter from floating-point micro-adjustments.
    pub fn ensure_visible(&mut self, item_start: f32, item_end: f32) {
        const EPSILON: f32 = 0.5;
        let visible_top = -self.offset;
        let visible_bottom = visible_top + self.viewport_size;

        // Only scroll if the item is significantly out of view (≥ EPSILON pixels)
        if item_start < visible_top - EPSILON {
            self.set_offset(-item_start);
        } else if item_end > visible_bottom + EPSILON {
            self.set_offset(-(item_end - self.viewport_size));
        }
    }

    /// Scroll to make the item the first visible element.
    /// GPUI equivalent: `scroll_to_top_of_item()` with `ScrollStrategy::Top`
    pub fn scroll_to_top(&mut self, item_start: f32) {
        self.set_offset(-item_start);
    }

    fn clamp_offset(&mut self) {
        self.offset = self.offset.clamp(self.scroll_min(), self.scroll_max());
    }
}

impl Default for ScrollHandle {
    fn default() -> Self {
        Self::new()
    }
}

// ── Scrollbar Style ────────────────────────────────────────

/// Colors and dimensions for a scrollbar thumb, matching GPUI scrollbar states.
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarColors {
    pub thumb: tiny_skia::Color,
    pub thumb_hover: tiny_skia::Color,
    pub track: tiny_skia::Color,
    pub border: tiny_skia::Color,
}

impl Default for ScrollbarColors {
    fn default() -> Self {
        Self {
            thumb: tiny_skia::Color::from_rgba8(82, 82, 82, 230),
            thumb_hover: tiny_skia::Color::from_rgba8(130, 130, 130, 230),
            track: tiny_skia::Color::from_rgba8(0, 0, 0, 0),
            border: tiny_skia::Color::from_rgba8(0, 0, 0, 0),
        }
    }
}

/// Scrollbar display mode matching GPUI's `ScrollbarShow`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollbarMode {
    /// Show when scrolling, fade after idle.
    Scrolling,
    /// Show on hover.
    Hover,
    /// Always show.
    Always,
}

// ── Scrollbar State (hover/drag/fade) ──────────────────────

/// Runtime state for scrollbar interaction, matching GPUI's `ScrollbarStateInner`.
#[derive(Debug, Clone)]
pub struct ScrollbarState {
    last_scroll: Instant,
    is_hovered: bool,
    is_dragging: bool,
}

impl ScrollbarState {
    pub fn new() -> Self {
        Self {
            last_scroll: Instant::now(),
            is_hovered: false,
            is_dragging: false,
        }
    }

    /// Call when a scroll event occurs.
    pub fn mark_scroll(&mut self) {
        self.last_scroll = Instant::now();
    }

    pub fn set_hovered(&mut self, hovered: bool) {
        self.is_hovered = hovered;
        if hovered {
            self.last_scroll = Instant::now();
        }
    }

    pub fn set_dragging(&mut self, dragging: bool) {
        self.is_dragging = dragging;
    }

    /// GPUI: `is_scrollbar_visible()` — visible during drag or within fade window.
    pub fn is_visible(&self, mode: ScrollbarMode) -> bool {
        match mode {
            ScrollbarMode::Always => true,
            ScrollbarMode::Hover => self.is_hovered,
            ScrollbarMode::Scrolling => {
                if self.is_dragging || self.is_hovered {
                    return true;
                }
                // GPUI: FADE_OUT_DURATION = 3.0s
                self.last_scroll.elapsed().as_secs_f32() < 3.0
            }
        }
    }

    /// Opacity for the fade animation (GPUI: 2s delay + 1s fade with pow(10)).
    pub fn thumb_opacity(&self) -> f32 {
        let elapsed = self.last_scroll.elapsed().as_secs_f32();
        // GPUI: FADE_OUT_DELAY = 2.0
        if elapsed < 2.0 {
            1.0
        // GPUI: FADE_OUT_DURATION = 3.0
        } else if elapsed < 3.0 {
            let t = elapsed - 2.0;
            (1.0 - t.powi(10)).max(0.0)
        } else {
            0.0
        }
    }

    /// Resolve the effective thumb color based on state (matching GPUI style_for_* methods).
    pub fn thumb_color(&self, colors: &ScrollbarColors, mode: ScrollbarMode) -> tiny_skia::Color {
        if !self.is_visible(mode) {
            return tiny_skia::Color::from_rgba8(0, 0, 0, 0);
        }
        let base = if self.is_dragging || (self.is_hovered && self.is_visible(mode)) {
            colors.thumb_hover
        } else {
            colors.thumb
        };
        // Apply fade opacity
        let alpha = (base.alpha() * self.thumb_opacity()).min(1.0);
        tiny_skia::Color::from_rgba8(
            (base.red() * 255.0) as u8,
            (base.green() * 255.0) as u8,
            (base.blue() * 255.0) as u8,
            (alpha * 255.0) as u8,
        )
    }
}

impl Default for ScrollbarState {
    fn default() -> Self {
        Self::new()
    }
}
