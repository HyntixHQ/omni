use cosmic_text::{Attrs, FontSystem, SwashCache, Weight};
use tiny_skia::{FillRule, Pixmap, Stroke, Transform};

use crate::config::Config;
use crate::render::{
    draw_pixmap_scaled_within, hex_color,
    rounded_rect_path, sk_paint,
};

pub fn badge_text_width(font_system: &mut FontSystem, text: &str, font_size: f32, family: &str) -> f32 {
    let metrics = cosmic_text::Metrics::new(font_size, font_size * 1.2);
    let mut buf = cosmic_text::Buffer::new(font_system, metrics);
    buf.set_size(font_system, Some(f32::MAX), Some(f32::MAX));
    let attrs = Attrs::new()
        .family(cosmic_text::Family::Name(family))
        .weight(Weight::MEDIUM);
    buf.set_text(font_system, text, attrs, cosmic_text::Shaping::Advanced);
    buf.shape_until_scroll(font_system, true);
    buf.layout_runs().fold(0.0, |acc, run| acc + run.line_w)
}

fn badge_text_metrics(font_system: &mut FontSystem, text: &str, font_size: f32, family: &str) -> (f32, f32) {
    let metrics = cosmic_text::Metrics::new(font_size, font_size * 1.2);
    let mut buf = cosmic_text::Buffer::new(font_system, metrics);
    let attrs = Attrs::new()
        .family(cosmic_text::Family::Name(family))
        .weight(Weight::MEDIUM);
    buf.set_text(font_system, text, attrs, cosmic_text::Shaping::Advanced);
    buf.shape_until_scroll(font_system, true);
    if let Some(run) = buf.layout_runs().next() {
        let ascent = run.line_y;
        let descent = (font_size * 1.2) - ascent;
        (ascent, descent)
    } else {
        (font_size * 0.9, font_size * 0.3)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
}

pub struct BadgeStyle {
    pub variant: BadgeVariant,
    pub font_size: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub gap: f32,
}

impl Default for BadgeStyle {
    fn default() -> Self {
        Self {
            variant: BadgeVariant::Secondary,
            font_size: 11.0,
            padding_x: 6.0,
            padding_y: 2.0,
            gap: 4.0,
        }
    }
}

fn resolve_colors(variant: BadgeVariant, t: &crate::config::ThemeConfig) -> ([u8; 4], [u8; 4], Option<[u8; 4]>) {
    match variant {
        BadgeVariant::Default => (
            hex_color(&t.badge_default_bg),
            hex_color(&t.badge_default_fg),
            None,
        ),
        BadgeVariant::Secondary => (
            hex_color(&t.badge_secondary_bg),
            hex_color(&t.badge_secondary_fg),
            None,
        ),
        BadgeVariant::Destructive => (
            hex_color(&t.badge_destructive_bg),
            hex_color(&t.badge_destructive_fg),
            None,
        ),
        BadgeVariant::Outline => (
            [0, 0, 0, 0],
            hex_color(&t.badge_outline_fg),
            Some(hex_color(&t.badge_outline_border)),
        ),
    }
}

pub fn draw_badge_within(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    x: f32,
    y: f32,
    text: &str,
    prefix_icon: Option<&Pixmap>,
    style: &BadgeStyle,
    family: &str,
    cfg: &Config,
    min_y: f32,
    max_y: f32,
) -> (f32, f32) {
    let (ascent, descent) = badge_text_metrics(font_system, text, style.font_size, family);
    let text_h = ascent + descent;
    let text_w = badge_text_width(font_system, text, style.font_size, family);

    let icon_size = (style.font_size * 0.9).round().max(1.0);
    let icon_w = if prefix_icon.is_some() { icon_size + style.gap } else { 0.0 };

    let content_w = icon_w + text_w;
    let content_h = text_h.max(icon_size);
    let badge_w = content_w + style.padding_x * 2.0;
    let badge_h = (content_h + style.padding_y * 2.0).round();
    let corner_r = (badge_h / 3.0).max(3.0);

    let (bg, fg, border) = resolve_colors(style.variant, &cfg.theme);

    if bg[3] > 0 {
        let mut paint = sk_paint(&cfg.theme.bg);
        paint.set_color(tiny_skia::Color::from_rgba8(bg[0], bg[1], bg[2], bg[3]));
        let path = rounded_rect_path(x, y, badge_w, badge_h, corner_r);
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }

    if let Some(border_color) = border {
        let mut paint = sk_paint(&cfg.theme.bg);
        paint.set_color(tiny_skia::Color::from_rgba8(
            border_color[0], border_color[1], border_color[2], border_color[3],
        ));
        let path = rounded_rect_path(x + 0.5, y + 0.5, badge_w - 1.0, badge_h - 1.0, corner_r);
        let stroke = Stroke { width: 1.0, ..Stroke::default() };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    let content_x = x + (badge_w - content_w) / 2.0;
    let mut current_x = content_x;

    if let Some(icon) = prefix_icon {
        let icon_y = y + (badge_h - icon_size) / 2.0;
        draw_pixmap_scaled_within(pixmap, icon, current_x, icon_y, icon_size, icon_size, min_y, max_y);
        current_x += icon_w;
    }

    let text_y = y + (badge_h - text_h) / 2.0;
    let attrs = Attrs::new()
        .family(cosmic_text::Family::Name(family))
        .weight(cosmic_text::Weight::MEDIUM);

    let metrics = cosmic_text::Metrics::new(style.font_size, style.font_size * 1.2);
    let mut buf = cosmic_text::Buffer::new(font_system, metrics);
    buf.set_size(font_system, Some(pixmap.width() as f32), Some(pixmap.height() as f32));
    buf.set_text(font_system, text, attrs, cosmic_text::Shaping::Advanced);
    buf.shape_until_scroll(font_system, true);

    let pw = pixmap.width();
    let ph = pixmap.height();
    let data = pixmap.data_mut();
    let color = cosmic_text::Color::rgba(fg[0], fg[1], fg[2], fg[3]);

    buf.draw(font_system, swash_cache, color, |gx, gy, gw, gh, gcolor| {
        let sa = gcolor.a() as u32;
        if sa == 0 {
            return;
        }
        let sr = gcolor.r() as u32;
        let sg = gcolor.g() as u32;
        let sb = gcolor.b() as u32;
        for dy in 0..gh {
            let py_f = text_y + gy as f32 + dy as f32;
            if py_f < min_y || py_f >= max_y {
                continue;
            }
            let py = py_f as i32;
            if py < 0 || py >= ph as i32 {
                continue;
            }
            for dx in 0..gw {
                let px = (current_x + gx as f32 + dx as f32) as i32;
                if px < 0 || px >= pw as i32 {
                    continue;
                }
                let idx = (py as u32 * pw + px as u32) as usize * 4;
                let dr = data[idx] as u32;
                let dg = data[idx + 1] as u32;
                let db = data[idx + 2] as u32;
                let inv = 255 - sa;
                data[idx] = ((sr * sa + dr * inv) / 255) as u8;
                data[idx + 1] = ((sg * sa + dg * inv) / 255) as u8;
                data[idx + 2] = ((sb * sa + db * inv) / 255) as u8;
                data[idx + 3] = 255;
            }
        }
    });

    (badge_w, badge_h)
}

pub fn draw_badge(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    x: f32,
    y: f32,
    text: &str,
    prefix_icon: Option<&Pixmap>,
    style: &BadgeStyle,
    family: &str,
    cfg: &Config,
) -> (f32, f32) {
    draw_badge_within(
        pixmap, font_system, swash_cache, x, y, text, prefix_icon, style, family, cfg,
        0.0, pixmap.height() as f32,
    )
}
