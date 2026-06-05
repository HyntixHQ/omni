use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Debug, Clone, Copy)]
pub struct PreviewSpec {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl PreviewSpec {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub const fn full() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        }
    }
    pub const fn left_half() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 0.5,
            h: 1.0,
        }
    }
    pub const fn right_half() -> Self {
        Self {
            x: 0.5,
            y: 0.0,
            w: 0.5,
            h: 1.0,
        }
    }
    pub const fn top_half() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 0.5,
        }
    }
    pub const fn bottom_half() -> Self {
        Self {
            x: 0.0,
            y: 0.5,
            w: 1.0,
            h: 0.5,
        }
    }
    pub const fn top_left() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 0.5,
            h: 0.5,
        }
    }
    pub const fn top_right() -> Self {
        Self {
            x: 0.5,
            y: 0.0,
            w: 0.5,
            h: 0.5,
        }
    }
    pub const fn bottom_left() -> Self {
        Self {
            x: 0.0,
            y: 0.5,
            w: 0.5,
            h: 0.5,
        }
    }
    pub const fn bottom_right() -> Self {
        Self {
            x: 0.5,
            y: 0.5,
            w: 0.5,
            h: 0.5,
        }
    }
    pub const fn center() -> Self {
        Self {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.8,
        }
    }
    pub const fn left_third() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0 / 3.0,
            h: 1.0,
        }
    }
    pub const fn center_third() -> Self {
        Self {
            x: 1.0 / 3.0,
            y: 0.0,
            w: 1.0 / 3.0,
            h: 1.0,
        }
    }
    pub const fn right_third() -> Self {
        Self {
            x: 2.0 / 3.0,
            y: 0.0,
            w: 1.0 / 3.0,
            h: 1.0,
        }
    }
    pub const fn two_thirds_left() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 2.0 / 3.0,
            h: 1.0,
        }
    }
    pub const fn two_thirds_center() -> Self {
        Self {
            x: 1.0 / 6.0,
            y: 0.0,
            w: 2.0 / 3.0,
            h: 1.0,
        }
    }
    pub const fn top_left_sixth() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0 / 3.0,
            h: 0.5,
        }
    }
    pub const fn top_center_sixth() -> Self {
        Self {
            x: 1.0 / 3.0,
            y: 0.0,
            w: 1.0 / 3.0,
            h: 0.5,
        }
    }
    pub const fn top_right_sixth() -> Self {
        Self {
            x: 2.0 / 3.0,
            y: 0.0,
            w: 1.0 / 3.0,
            h: 0.5,
        }
    }
    pub const fn bottom_left_sixth() -> Self {
        Self {
            x: 0.0,
            y: 0.5,
            w: 1.0 / 3.0,
            h: 0.5,
        }
    }
    pub const fn bottom_center_sixth() -> Self {
        Self {
            x: 1.0 / 3.0,
            y: 0.5,
            w: 1.0 / 3.0,
            h: 0.5,
        }
    }
    pub const fn bottom_right_sixth() -> Self {
        Self {
            x: 2.0 / 3.0,
            y: 0.5,
            w: 1.0 / 3.0,
            h: 0.5,
        }
    }
    pub const fn smaller() -> Self {
        Self {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.8,
        }
    }
    pub const fn larger() -> Self {
        Self {
            x: -0.05,
            y: -0.05,
            w: 1.1,
            h: 1.1,
        }
    }
}

pub struct PreviewColors {
    pub accent: Color,
    pub outline: Color,
}

impl Default for PreviewColors {
    fn default() -> Self {
        Self {
            accent: Color::from_rgba8(74, 158, 255, 255),
            outline: Color::from_rgba8(100, 100, 100, 200),
        }
    }
}

/// Draw a mini-screen outline with a filled rect at the spec's normalized position.
/// `x`, `y`, `w`, `h` define the screen outline box.
/// `spec` is in 0-1 normalized screen space.
pub fn draw_preview(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    spec: PreviewSpec,
    colors: &PreviewColors,
) {
    let radius = 2.0;
    let screen_padding = 1.5;

    let mut pb = PathBuilder::new();
    let r = radius;
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
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(colors.outline);
        paint.anti_alias = true;
        let stroke = Stroke {
            width: 1.0,
            ..Stroke::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    let inner_x = x + screen_padding;
    let inner_y = y + screen_padding;
    let inner_w = w - screen_padding * 2.0;
    let inner_h = h - screen_padding * 2.0;

    let win_x = inner_x + spec.x * inner_w;
    let win_y = inner_y + spec.y * inner_h;
    let win_w = spec.w * inner_w;
    let win_h = spec.h * inner_h;

    if win_w <= 0.0 || win_h <= 0.0 {
        return;
    }

    let win_clamp_x = win_x.max(inner_x);
    let win_clamp_y = win_y.max(inner_y);
    let win_clamp_w = (win_x + win_w).min(inner_x + inner_w) - win_clamp_x;
    let win_clamp_h = (win_y + win_h).min(inner_y + inner_h) - win_clamp_y;
    if win_clamp_w <= 0.0 || win_clamp_h <= 0.0 {
        return;
    }

    let mut pb = PathBuilder::new();
    let wr = 1.0;
    pb.move_to(win_clamp_x + wr, win_clamp_y);
    pb.line_to(win_clamp_x + win_clamp_w - wr, win_clamp_y);
    pb.quad_to(
        win_clamp_x + win_clamp_w,
        win_clamp_y,
        win_clamp_x + win_clamp_w,
        win_clamp_y + wr,
    );
    pb.line_to(win_clamp_x + win_clamp_w, win_clamp_y + win_clamp_h - wr);
    pb.quad_to(
        win_clamp_x + win_clamp_w,
        win_clamp_y + win_clamp_h,
        win_clamp_x + win_clamp_w - wr,
        win_clamp_y + win_clamp_h,
    );
    pb.line_to(win_clamp_x + wr, win_clamp_y + win_clamp_h);
    pb.quad_to(
        win_clamp_x,
        win_clamp_y + win_clamp_h,
        win_clamp_x,
        win_clamp_y + win_clamp_h - wr,
    );
    pb.line_to(win_clamp_x, win_clamp_y + wr);
    pb.quad_to(win_clamp_x, win_clamp_y, win_clamp_x + wr, win_clamp_y);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(colors.accent);
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}
