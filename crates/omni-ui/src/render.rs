use cosmic_text::{Attrs, Color, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{
    FillRule, Paint, Path, PathBuilder, Pixmap, Transform,
};

use crate::components::badge::{BadgeStyle, BadgeVariant};
use crate::components::input_field::draw_input_field;
use crate::config::Config;
use crate::state::OmniApp;

pub(crate) fn hex_color(hex: &str) -> [u8; 4] {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
    } else {
        255
    };
    [r, g, b, a]
}

pub(crate) fn sk_paint(hex: &str) -> Paint<'_> {
    let [r, g, b, a] = hex_color(hex);
    let mut p = Paint::default();
    p.set_color(tiny_skia::Color::from_rgba8(r, g, b, a));
    p.anti_alias = true;
    p
}

pub(crate) fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Path {
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}

pub const HEADER_HEIGHT: f32 = 42.0;
pub const FOOTER_HEIGHT: f32 = 28.0;
pub const PADDING: f32 = 10.0;
pub const GAP: f32 = 8.0;

pub fn draw_frame(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    app: &mut OmniApp,
    cfg: &Config,
    cursor_visible: bool,
) {
    let header_height = HEADER_HEIGHT;
    let footer_height = FOOTER_HEIGHT;

    let scaffold = crate::components::scaffold::Scaffold::new(PADDING);
    let (header, body, footer) = scaffold.draw(pixmap, cfg, header_height, footer_height);

    app.mouse.set_regions(header, body, footer);

    if let Some(h) = header {
        draw_input_field(
            pixmap, font_system, swash_cache, app, cfg, cursor_visible,
            h.x, h.y, h.w, h.h,
        );
    }

    for result in &app.results {
        let icon_name = result.entry.icon.as_deref().unwrap_or("application-x-executable");
        app.icon_cache.get_icon(icon_name);
    }

    let mut list_items = Vec::with_capacity(app.results.len());
    for (i, result) in app.results.iter().enumerate() {
        let icon_name = result.entry.icon.as_deref().unwrap_or("application-x-executable");
        let prefix_icon = app.icon_cache.get_icon_ref(icon_name);

        list_items.push(crate::components::list_view::ListItem {
            title: &result.entry.name,
            subtitle: result.entry.description.as_deref(),
            prefix_icon,
            suffix_icon: None,
            selected: i == app.selected_index,
        });
    }

    if let Some(f) = footer {
        let font_size = (cfg.font.size as f32 - 2.0).max(12.0);
        let family = &cfg.font.family;
        let fg = hex_color(&cfg.theme.desc_fg);

        let badge_style = BadgeStyle {
            variant: BadgeVariant::Secondary,
            font_size,
            ..Default::default()
        };

        let badge_y = f.y + (f.h - (badge_style.font_size * 1.2 + 4.0)) / 2.0;

        let left_shortcuts = [
            ("system-run-symbolic", "↵", "Launch"),
            ("input-keyboard-symbolic", "↑↓", "Navigate"),
        ];

        let mut current_x = f.x + 10.0;

        for (icon_name, key, label) in &left_shortcuts {
            let icon_pixmap = app.icon_cache.get_icon_ref(icon_name);
            let (bw, bh) = crate::components::badge::draw_badge(
                pixmap, font_system, swash_cache,
                current_x, badge_y, key, icon_pixmap, &badge_style, family, cfg,
            );
            let label_ty = badge_y + bh / 2.0 - font_size / 2.0;
            current_x += bw + 8.0;
            render_text(
                pixmap, font_system, swash_cache,
                current_x, label_ty, label, font_size, family, fg,
                f.w - (current_x - f.x),
            );
            current_x += 100.0;
        }

        let esc_label = "Quit";
        let esc_key_str = "ESC";
        let esc_icon = app.icon_cache.get_icon_ref("window-close-symbolic");
        let esc_badge_w = crate::components::badge::badge_text_width(font_system, esc_key_str, badge_style.font_size, family)
            + badge_style.padding_x * 2.0;
        let esc_label_w = measure_text_width(font_system, esc_label, font_size, family, f32::MAX, f.h);
        let esc_total_w = esc_badge_w + 8.0 + esc_label_w;
        let esc_x = f.x + f.w - 10.0 - esc_total_w;

        let (_, esc_bh) = crate::components::badge::draw_badge(
            pixmap, font_system, swash_cache,
            esc_x, badge_y, esc_key_str, esc_icon, &badge_style, family, cfg,
        );
        let esc_label_ty = badge_y + esc_bh / 2.0 - font_size / 2.0;
        render_text(
            pixmap, font_system, swash_cache,
            esc_x + esc_badge_w + 8.0, esc_label_ty, esc_label, font_size, family, fg,
            esc_label_w,
        );
    }

    crate::components::list_view::draw_list_view(
        pixmap,
        font_system,
        swash_cache,
        body.x,
        body.y,
        body.w,
        body.h,
        &list_items,
        &mut app.list_state,
        cfg,
    );
}

pub(crate) fn measure_text_width(
    font_system: &mut FontSystem,
    text: &str,
    font_size: f32,
    family: &str,
    max_w: f32,
    max_h: f32,
) -> f32 {
    let metrics = Metrics::new(font_size, font_size * 1.2);
    let mut buf = cosmic_text::Buffer::new(font_system, metrics);
    buf.set_size(font_system, Some(max_w), Some(max_h));
    let attrs = Attrs::new().family(cosmic_text::Family::Name(family));
    buf.set_text(font_system, text, attrs, Shaping::Advanced);
    buf.shape_until_scroll(font_system, true);
    buf.layout_runs().fold(0.0, |acc, run| acc + run.line_w)
}

pub(crate) fn render_text(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    family: &str,
    fg: [u8; 4],
    max_w: f32,
) {
    let ph = pixmap.height() as f32;
    render_text_within(pixmap, font_system, swash_cache, x, y, text, font_size, family, fg, 0.0, ph, max_w);
}

pub(crate) fn render_text_within(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    family: &str,
    fg: [u8; 4],
    min_y: f32,
    max_y: f32,
    max_w: f32,
) {
    let metrics = Metrics::new(font_size, font_size * 1.2);
    let attrs = Attrs::new().family(cosmic_text::Family::Name(family));

    // Fast path: Check if we need truncation
    let mut display_text = text.to_string();
    let current_w = measure_text_width(font_system, text, font_size, family, f32::MAX, f32::MAX);
    
    if current_w > max_w {
        let ellipsis = "...";
        let ellipsis_w = measure_text_width(font_system, ellipsis, font_size, family, f32::MAX, f32::MAX);
        let target_w = max_w - ellipsis_w;
        
        // Simple linear truncation for now (could be binary search for performance with very long strings)
        while display_text.len() > 0 && measure_text_width(font_system, &display_text, font_size, family, f32::MAX, f32::MAX) > target_w {
            display_text.pop();
        }
        display_text.push_str(ellipsis);
    }

    let mut buf = cosmic_text::Buffer::new(font_system, metrics);
    buf.set_size(
        font_system,
        Some(pixmap.width() as f32),
        Some(pixmap.height() as f32),
    );
    buf.set_text(font_system, &display_text, attrs, Shaping::Advanced);
    buf.shape_until_scroll(font_system, true);

    let pw = pixmap.width();
    let ph = pixmap.height();
    let data = pixmap.data_mut();
    let color = Color::rgba(fg[0], fg[1], fg[2], fg[3]);

    buf.draw(
        font_system,
        swash_cache,
        color,
        |gx, gy, gw, gh, gcolor: Color| {
            let sa = gcolor.a() as u32;
            if sa == 0 {
                return;
            }
            let sr = gcolor.r() as u32;
            let sg = gcolor.g() as u32;
            let sb = gcolor.b() as u32;
            for dy in 0..gh {
                let py_f = y + gy as f32 + dy as f32;
                if py_f < min_y || py_f >= max_y {
                    continue;
                }
                let py = py_f as i32;
                if py < 0 || py >= ph as i32 {
                    continue;
                }
                for dx in 0..gw {
                    let px = (x + gx as f32 + dx as f32) as i32;
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
        },
    );
}

pub(crate) fn draw_pixmap_within(
    dst: &mut Pixmap,
    src: &Pixmap,
    x: f32,
    y: f32,
    min_y: f32,
    max_y: f32,
) {
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let sw = src.width() as i32;
    let sh = src.height() as i32;

    let dst_data = dst.data_mut();
    let src_data = src.data();

    let start_x = x.floor() as i32;
    let start_y = y.floor() as i32;

    for sy in 0..sh {
        let dy = start_y + sy;
        if dy < 0 || dy >= dh || (dy as f32) < min_y || (dy as f32) >= max_y {
            continue;
        }

        for sx in 0..sw {
            let dx = start_x + sx;
            if dx < 0 || dx >= dw {
                continue;
            }

            let s_idx = (sy * sw + sx) as usize * 4;
            let d_idx = (dy * dw + dx) as usize * 4;

            let sa = src_data[s_idx + 3] as u32;
            if sa == 0 {
                continue;
            }
            
            if sa == 255 {
                dst_data[d_idx..d_idx + 4].copy_from_slice(&src_data[s_idx..s_idx + 4]);
            } else {
                let sr = src_data[s_idx] as u32;
                let sg = src_data[s_idx + 1] as u32;
                let sb = src_data[s_idx + 2] as u32;
                let dr = dst_data[d_idx] as u32;
                let dg = dst_data[d_idx + 1] as u32;
                let db = dst_data[d_idx + 2] as u32;
                let inv = 255 - sa;
                dst_data[d_idx] = (sr + (dr * inv) / 255) as u8;
                dst_data[d_idx + 1] = (sg + (dg * inv) / 255) as u8;
                dst_data[d_idx + 2] = (sb + (db * inv) / 255) as u8;
                dst_data[d_idx + 3] = 255;
            }
        }
    }
}

pub(crate) fn fill_rect_within(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    paint: &Paint,
    min_y: f32,
    max_y: f32,
) {
    if y >= max_y || y + h <= min_y {
        return;
    }

    let draw_y = y.max(min_y);
    let draw_h = (y + h).min(max_y) - draw_y;
    
    if draw_h <= 0.0 {
        return;
    }

    let path = rounded_rect_path(x, draw_y, w, draw_h, r);
    pixmap.fill_path(&path, paint, FillRule::Winding, Transform::identity(), None);
}

pub fn get_text_metrics(
    font_system: &mut FontSystem,
    text: &str,
    font_size: f32,
    family: &str,
) -> (f32, f32) {
    let metrics = Metrics::new(font_size, font_size * 1.2);
    let mut buf = cosmic_text::Buffer::new(font_system, metrics);
    let attrs = Attrs::new().family(cosmic_text::Family::Name(family));
    buf.set_text(font_system, text, attrs, Shaping::Advanced);
    buf.shape_until_scroll(font_system, true);

    if let Some(run) = buf.layout_runs().next() {
        let ascent = run.line_y;
        let descent = (font_size * 1.2) - ascent;
        (ascent, descent)
    } else {
        (font_size * 0.9, font_size * 0.3)
    }
}

pub(crate) fn draw_pixmap_scaled_within(
    dst: &mut Pixmap,
    src: &Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    min_y: f32,
    max_y: f32,
) {
    if y >= max_y || y + h <= min_y {
        return;
    }

    let scale_x = w / src.width() as f32;
    let scale_y = h / src.height() as f32;
    
    let draw_y = y.max(min_y);
    let draw_h = (y + h).min(max_y) - draw_y;
    
    let mut paint = Paint::default();
    paint.shader = tiny_skia::Pattern::new(
        src.as_ref(),
        tiny_skia::SpreadMode::Pad,
        tiny_skia::FilterQuality::Bicubic,
        1.0,
        Transform::from_translate(x, y).pre_scale(scale_x, scale_y),
    );

    let rect = match tiny_skia::Rect::from_xywh(x, draw_y, w, draw_h) {
        Some(r) => r,
        None => return,
    };

    dst.fill_rect(rect, &paint, Transform::identity(), None);
}

pub fn rgba_to_bgra(src: &[u8], dst: &mut [u8]) {
    for i in 0..src.len() / 4 {
        dst[i * 4] = src[i * 4 + 2];     // B = R
        dst[i * 4 + 1] = src[i * 4 + 1]; // G = G
        dst[i * 4 + 2] = src[i * 4];     // R = B
        dst[i * 4 + 3] = src[i * 4 + 3]; // A = A
    }
}
