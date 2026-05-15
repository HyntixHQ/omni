use tiny_skia::{FillRule, Pixmap, Transform};

use crate::config::Config;
use crate::render::{rounded_rect_path, sk_paint};

#[derive(Debug, Clone, Copy)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Scaffold {
    padding: f32,
}

impl Scaffold {
    pub fn new(padding: f32) -> Self {
        Self { padding }
    }

    /// Draws the shell (background + border) and returns layout rects.
    /// Pass `0.0` for header_height or footer_height to skip that slot.
    /// Returns `(header_rect, body_rect, footer_rect)` — each is `None` when its height is 0.
    pub fn draw(
        &self,
        pixmap: &mut Pixmap,
        cfg: &Config,
        header_height: f32,
        footer_height: f32,
    ) -> (Option<LayoutRect>, LayoutRect, Option<LayoutRect>) {
        let w = pixmap.width() as f32;
        let h = pixmap.height() as f32;
        let padding = self.padding;

        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let bg_path = rounded_rect_path(0.0, 0.0, w, h, cfg.theme.border_radius as f32);
        pixmap.fill_path(
            &bg_path,
            &sk_paint(&cfg.theme.bg),
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        let border_path =
            rounded_rect_path(0.5, 0.5, w - 1.0, h - 1.0, cfg.theme.border_radius as f32);
        let mut border_paint = sk_paint(&cfg.theme.border);
        border_paint.anti_alias = true;
        let border_stroke = tiny_skia::Stroke {
            width: 1.0,
            ..tiny_skia::Stroke::default()
        };
        pixmap.stroke_path(&border_path, &border_paint, &border_stroke, Transform::identity(), None);

        let inner_w = w - (padding * 2.0);

        let header_rect = if header_height > 0.0 {
            Some(LayoutRect {
                x: padding,
                y: padding,
                w: inner_w,
                h: header_height,
            })
        } else {
            None
        };

        let footer_rect = if footer_height > 0.0 {
            let gap = 8.0;
            let line_y = h - padding - footer_height - gap;
            crate::components::divider::draw_divider(
                pixmap,
                0.0,
                line_y,
                w,
                1.0,
                crate::components::divider::Orientation::Horizontal,
                &cfg.theme.border,
                40,
            );
            Some(LayoutRect {
                x: padding,
                y: h - padding - footer_height,
                w: inner_w,
                h: footer_height,
            })
        } else {
            None
        };

        let header_end = header_rect
            .map(|r| r.y + r.h + 8.0)
            .unwrap_or(padding);
        let footer_start = footer_rect
            .map(|r| r.y - 8.0)
            .unwrap_or(h - padding);
        let body_y = header_end;
        let body_h = (footer_start - body_y).max(0.0);

        let body_rect = LayoutRect {
            x: padding,
            y: body_y,
            w: inner_w,
            h: body_h,
        };

        (header_rect, body_rect, footer_rect)
    }
}
