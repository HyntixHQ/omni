use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ab_glyph::{Font, FontArc, PxScale};
use lucide_icons::{Icon, LUCIDE_FONT_BYTES};
use tiny_skia::{Color, Pixmap};

type CacheKey = (u16, u8);

struct IconRenderer {
    font: FontArc,
    cache: Mutex<HashMap<CacheKey, Pixmap>>,
}

static RENDERER: OnceLock<IconRenderer> = OnceLock::new();

fn renderer() -> &'static IconRenderer {
    RENDERER.get_or_init(|| {
        let font =
            FontArc::try_from_slice(LUCIDE_FONT_BYTES).expect("Lucide font data should be valid");
        IconRenderer {
            font,
            cache: Mutex::new(HashMap::new()),
        }
    })
}

fn rasterize(icon: Icon, size: f32) -> Pixmap {
    let r = renderer();
    let c = char::from(icon);
    let glyph = r
        .font
        .glyph_id(c)
        .with_scale_and_position(PxScale::from(size), (0.0f32, 0.0f32));

    let Some(outline) = r.font.outline_glyph(glyph) else {
        return Pixmap::new(1, 1).expect("1x1 fallback pixmap");
    };

    let bounds = outline.px_bounds();
    let w = bounds.width().ceil().max(1.0) as u32;
    let h = bounds.height().ceil().max(1.0) as u32;
    let mut pixmap = Pixmap::new(w, h).expect("icon pixmap");

    outline.draw(|x, y, coverage| {
        if x < w && y < h {
            let idx = (y * w + x) as usize * 4;
            let alpha = (coverage * 255.0) as u8;
            pixmap.data_mut()[idx] = alpha;
            pixmap.data_mut()[idx + 1] = alpha;
            pixmap.data_mut()[idx + 2] = alpha;
            pixmap.data_mut()[idx + 3] = alpha;
        }
    });

    pixmap
}

fn get_or_rasterize(icon: Icon, size: f32) -> Pixmap {
    let r = renderer();
    let key = (icon as u16, size.round() as u8);
    let mut cache = r.cache.lock().expect("icon cache lock");
    cache
        .entry(key)
        .or_insert_with(|| rasterize(icon, size))
        .clone()
}

pub fn draw_lucide_icon(
    pixmap: &mut Pixmap,
    icon: Icon,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) -> f32 {
    let src = get_or_rasterize(icon, size);
    let w = src.width();
    let h = src.height();

    let draw_x = (x + (size - w as f32) / 2.0).round() as i32;
    let draw_y = (y + (size - h as f32) / 2.0).round() as i32;

    let cr = (color.red() * 255.0) as u32;
    let cg = (color.green() * 255.0) as u32;
    let cb = (color.blue() * 255.0) as u32;
    let dw = pixmap.width() as i32;
    let dh = pixmap.height() as i32;
    let src_data = src.data();
    let dst_data = pixmap.data_mut();

    for sy in 0..h {
        let py = draw_y + sy as i32;
        if py < 0 || py >= dh {
            continue;
        }
        for sx in 0..w {
            let px = draw_x + sx as i32;
            if px < 0 || px >= dw {
                continue;
            }
            let s_idx = (sy * w + sx) as usize * 4;
            let d_idx = (py as u32 * dw as u32 + px as u32) as usize * 4;
            let src_alpha = src_data[s_idx + 3] as u32;
            if src_alpha == 0 {
                continue;
            }

            let sa = src_alpha as f32 / 255.0;
            let inv = 1.0 - sa;

            let dr = dst_data[d_idx] as f32;
            let dg = dst_data[d_idx + 1] as f32;
            let db = dst_data[d_idx + 2] as f32;
            let da = dst_data[d_idx + 3] as f32;

            dst_data[d_idx] = (cr as f32 * sa + dr * inv) as u8;
            dst_data[d_idx + 1] = (cg as f32 * sa + dg * inv) as u8;
            dst_data[d_idx + 2] = (cb as f32 * sa + db * inv) as u8;
            dst_data[d_idx + 3] = (255.0 * sa + da * inv) as u8;
        }
    }

    w as f32
}

pub fn lucide_icon_width(icon: Icon, size: f32) -> f32 {
    let r = renderer();
    let c = char::from(icon);
    let glyph = r
        .font
        .glyph_id(c)
        .with_scale_and_position(PxScale::from(size), (0.0f32, 0.0f32));
    match r.font.outline_glyph(glyph) {
        Some(outline) => outline.px_bounds().width(),
        None => size,
    }
}
