use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

pub const PADDING: f32 = 12.0;
pub const GAP: f32 = 4.0;
pub const HEADER_HEIGHT: f32 = 32.0;
pub const FOOTER_HEIGHT: f32 = 28.0;

pub fn fill_rect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: Color) {
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

pub fn fill_rounded_rect(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: [f32; 4],
    color: Color,
) {
    let r = radius[0];
    if let Some(path) = build_rounded_rect_path(x, y, w, h, r) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

pub fn stroke_rounded_rect(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: Color,
    stroke_width: f32,
) {
    if let Some(path) = build_rounded_rect_path(x, y, w, h, radius) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let stroke = tiny_skia::Stroke {
            width: stroke_width,
            ..tiny_skia::Stroke::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn build_rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
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
    pb.finish()
}

pub fn stroke_rect(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    stroke_width: f32,
) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke {
        width: stroke_width,
        ..tiny_skia::Stroke::default()
    };
    let path = {
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(x, y);
        pb.line_to(x + w, y);
        pb.line_to(x + w, y + h);
        pb.line_to(x, y + h);
        pb.close();
        pb.finish().unwrap()
    };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

pub fn clear(pixmap: &mut Pixmap, color: Color) {
    pixmap.fill(color);
}

pub fn rgba_to_bgra(src: &[u8], dst: &mut [u8]) {
    debug_assert_eq!(src.len(), dst.len());
    for (chunk, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = chunk[2];
        d[1] = chunk[1];
        d[2] = chunk[0];
        d[3] = chunk[3];
    }
}

pub fn draw_pixmap_at(pixmap: &mut Pixmap, icon: &Pixmap, x: f32, y: f32) {
    let px = x as i32;
    let py = y as i32;
    if px >= 0 && py >= 0 {
        let paint = tiny_skia::PixmapPaint::default();
        pixmap.draw_pixmap(px, py, icon.as_ref(), &paint, Transform::identity(), None);
    }
}

pub fn draw_text(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    font_family: &str,
    color: Color,
) -> f32 {
    use cosmic_text::Color as CtColor;

    let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::Name(font_family));
    let mut buffer =
        cosmic_text::Buffer::new(font_system, cosmic_text::Metrics::new(font_size, font_size));
    buffer.set_text(font_system, text, attrs, cosmic_text::Shaping::Advanced);
    buffer.set_size(
        font_system,
        Some(pixmap.width() as f32),
        Some(pixmap.height() as f32),
    );

    let red = (color.red() * 255.0) as u8;
    let green = (color.green() * 255.0) as u8;
    let blue = (color.blue() * 255.0) as u8;
    let alpha = (color.alpha() * 255.0) as u8;

    buffer.draw(
        font_system,
        swash_cache,
        CtColor::rgba(red, green, blue, alpha),
        |draw_x, draw_y, _w, _h, pixel| {
            let pixel_val: u32 = pixel.0;
            if pixel_val > 0 {
                let px = draw_x + x as i32;
                let py = draw_y + y as i32;
                if px >= 0 && px < pixmap.width() as i32 && py >= 0 && py < pixmap.height() as i32 {
                    let idx = (py as u32 * pixmap.width() + px as u32) as usize * 4;
                    let src_alpha = (pixel_val >> 24) & 0xFF;
                    let src_r = (pixel_val >> 16) & 0xFF;
                    let src_g = (pixel_val >> 8) & 0xFF;
                    let src_b = pixel_val & 0xFF;
                    let blend = src_alpha as f32 / 255.0;
                    if idx + 3 < pixmap.data().len() {
                        let data = pixmap.data_mut();
                        data[idx] = (data[idx] as f32 * (1.0 - blend) + src_r as f32 * blend) as u8;
                        data[idx + 1] =
                            (data[idx + 1] as f32 * (1.0 - blend) + src_g as f32 * blend) as u8;
                        data[idx + 2] =
                            (data[idx + 2] as f32 * (1.0 - blend) + src_b as f32 * blend) as u8;
                        data[idx + 3] =
                            (data[idx + 3] as f32 * (1.0 - blend) + alpha as f32 * blend) as u8;
                    }
                }
            }
        },
    );
    y + font_size
}

pub fn text_width(
    font_system: &mut cosmic_text::FontSystem,
    text: &str,
    font_size: f32,
    font_family: &str,
) -> f32 {
    let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::Name(font_family));
    let mut buffer =
        cosmic_text::Buffer::new(font_system, cosmic_text::Metrics::new(font_size, font_size));
    buffer.set_text(font_system, text, attrs, cosmic_text::Shaping::Advanced);
    buffer.shape_until_scroll(font_system, true);
    buffer.lines.iter().fold(0.0f32, |max, line| {
        let w = line
            .layout_opt()
            .as_ref()
            .map(|l| l.iter().map(|g| g.w).sum())
            .unwrap_or(0.0);
        max.max(w)
    })
}

/// Returns (ascent, descent) for a given font size and family.
pub fn text_metrics(
    font_system: &mut cosmic_text::FontSystem,
    text: &str,
    font_size: f32,
    font_family: &str,
) -> (f32, f32) {
    let metrics = cosmic_text::Metrics::new(font_size, font_size * 1.2);
    let mut buf = cosmic_text::Buffer::new(font_system, metrics);
    let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::Name(font_family));
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

/// Draw text clipped to a vertical range [min_y, max_y).
pub fn draw_text_clipped(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    font_family: &str,
    color: Color,
    min_y: f32,
    max_y: f32,
    max_w: f32,
) {
    use cosmic_text::Color as CtColor;

    // Truncate text with ellipsis if it exceeds max_w
    let display_text = if max_w > 0.0 {
        let full_w = text_width(font_system, text, font_size, font_family);
        if full_w > max_w {
            let ellipsis = "\u{2026}";
            let ellipsis_w = text_width(font_system, ellipsis, font_size, font_family);
            let target_w = max_w - ellipsis_w;
            // Binary search on character indices to avoid splitting multi-byte chars
            let char_indices: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
            let mut lo = 0usize;
            let mut hi = char_indices.len();
            while lo < hi {
                let mid_char = (lo + hi).div_ceil(2);
                let mid_byte = char_indices.get(mid_char).copied().unwrap_or(text.len());
                let prefix = &text[..mid_byte];
                let w = text_width(font_system, prefix, font_size, font_family);
                if w <= target_w {
                    lo = mid_char;
                } else {
                    hi = mid_char - 1;
                }
            }
            let byte_end = char_indices.get(lo).copied().unwrap_or(text.len());
            let mut result = text[..byte_end].to_string();
            result.push('\u{2026}');
            result
        } else {
            text.to_string()
        }
    } else {
        text.to_string()
    };

    let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::Name(font_family));
    let mut buffer =
        cosmic_text::Buffer::new(font_system, cosmic_text::Metrics::new(font_size, font_size));
    buffer.set_text(
        font_system,
        &display_text,
        attrs,
        cosmic_text::Shaping::Advanced,
    );
    buffer.set_size(
        font_system,
        Some(pixmap.width() as f32),
        Some(pixmap.height() as f32),
    );
    buffer.set_wrap(font_system, cosmic_text::Wrap::None);

    let text_left = x;
    let text_right = if max_w > 0.0 {
        x + max_w
    } else {
        pixmap.width() as f32
    };

    let red = (color.red() * 255.0) as u8;
    let green = (color.green() * 255.0) as u8;
    let blue = (color.blue() * 255.0) as u8;
    let alpha = (color.alpha() * 255.0) as u8;

    buffer.draw(
        font_system,
        swash_cache,
        CtColor::rgba(red, green, blue, alpha),
        |draw_x, draw_y, _w, _h, pixel| {
            let pixel_val: u32 = pixel.0;
            if pixel_val > 0 {
                let px = draw_x + x as i32;
                let py = draw_y + y as i32;
                if py < 0 || (py as f32) < min_y || (py as f32) >= max_y {
                    return;
                }
                // Horizontal clipping at max_w boundary
                let px_f = draw_x as f32 + x;
                if px_f < text_left || px_f >= text_right {
                    return;
                }
                if px < 0 || px >= pixmap.width() as i32 {
                    return;
                }
                let idx = (py as u32 * pixmap.width() + px as u32) as usize * 4;
                let src_alpha = (pixel_val >> 24) & 0xFF;
                let src_r = (pixel_val >> 16) & 0xFF;
                let src_g = (pixel_val >> 8) & 0xFF;
                let src_b = pixel_val & 0xFF;
                let blend = src_alpha as f32 / 255.0;
                if idx + 3 < pixmap.data().len() {
                    let data = pixmap.data_mut();
                    data[idx] = (data[idx] as f32 * (1.0 - blend) + src_r as f32 * blend) as u8;
                    data[idx + 1] =
                        (data[idx + 1] as f32 * (1.0 - blend) + src_g as f32 * blend) as u8;
                    data[idx + 2] =
                        (data[idx + 2] as f32 * (1.0 - blend) + src_b as f32 * blend) as u8;
                    data[idx + 3] =
                        (data[idx + 3] as f32 * (1.0 - blend) + alpha as f32 * blend) as u8;
                }
            }
        },
    );
}

/// Draw a pixmap (icon) clipped to a vertical range [min_y, max_y).
pub fn draw_pixmap_clipped(
    pixmap: &mut Pixmap,
    src: &Pixmap,
    x: f32,
    y: f32,
    min_y: f32,
    max_y: f32,
) {
    let dw = pixmap.width() as i32;
    let dh = pixmap.height() as i32;
    let sw = src.width() as i32;
    let sh = src.height() as i32;

    let dst_data = pixmap.data_mut();
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

/// Fill a rect clipped to a vertical range [min_y, max_y), with rounded corners.
pub fn fill_rect_clipped(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    color: Color,
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

    let mut pb = tiny_skia::PathBuilder::new();
    let rr = r;
    pb.move_to(x + rr, draw_y);
    pb.line_to(x + w - rr, draw_y);
    pb.quad_to(x + w, draw_y, x + w, draw_y + rr);
    pb.line_to(x + w, draw_y + draw_h - rr);
    pb.quad_to(x + w, draw_y + draw_h, x + w - rr, draw_y + draw_h);
    pb.line_to(x + rr, draw_y + draw_h);
    pb.quad_to(x, draw_y + draw_h, x, draw_y + draw_h - rr);
    pb.line_to(x, draw_y + rr);
    pb.quad_to(x, draw_y, x + rr, draw_y);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// Parse a hex color string (#rrggbb or #rrggbbaa) into a Color.
pub fn color_from_hex(hex: &str) -> Color {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
    } else {
        255
    };
    Color::from_rgba8(r, g, b, a)
}
