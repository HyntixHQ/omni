use tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

/// Renders SVG data directly into a pixmap at the specified location and size.
/// Note: SVG rendering is relatively expensive; for performance-critical paths,
/// prefer using a cached Pixmap.
pub fn draw_svg_data(
    pixmap: &mut Pixmap,
    svg_data: &[u8],
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Option<()> {
    let opt = Options::default();
    let tree = Tree::from_data(svg_data, &opt).ok()?;

    let svg_size = tree.size();
    let scale_x = width / svg_size.width();
    let scale_y = height / svg_size.height();
    let scale = scale_x.min(scale_y);

    // Center within the requested box if aspect ratio differs
    let dx = x + (width - svg_size.width() * scale) / 2.0;
    let dy = y + (height - svg_size.height() * scale) / 2.0;

    let transform = Transform::from_translate(dx, dy).post_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(())
}

/// A simple helper to draw a pre-rendered icon Pixmap (e.g. from IconCache).
pub fn draw_icon_pixmap(
    dst: &mut Pixmap,
    src: &Pixmap,
    x: f32,
    y: f32,
) {
    crate::render::draw_pixmap_within(dst, src, x, y, 0.0, dst.height() as f32);
}

/// Draws an icon with clipping support.
pub fn draw_icon_pixmap_within(
    dst: &mut Pixmap,
    src: &Pixmap,
    x: f32,
    y: f32,
    min_y: f32,
    max_y: f32,
) {
    crate::render::draw_pixmap_within(dst, src, x, y, min_y, max_y);
}
