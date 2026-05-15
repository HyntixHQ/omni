use tiny_skia::{Pixmap, PathBuilder, Transform, Stroke};
use crate::render::sk_paint;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

pub fn draw_divider(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    length: f32,
    thickness: f32,
    orientation: Orientation,
    color_hex: &str,
    alpha: u8,
) {
    let mut paint = sk_paint(color_hex);
    let [r, g, b, _] = crate::render::hex_color(color_hex);
    paint.set_color(tiny_skia::Color::from_rgba8(r, g, b, alpha));

    let mut pb = PathBuilder::new();
    match orientation {
        Orientation::Horizontal => {
            pb.move_to(x, y);
            pb.line_to(x + length, y);
        }
        Orientation::Vertical => {
            pb.move_to(x, y);
            pb.line_to(x, y + length);
        }
    }

    if let Some(path) = pb.finish() {
        let stroke = Stroke {
            width: thickness,
            ..Stroke::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}
