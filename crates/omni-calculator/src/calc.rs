use tiny_skia::Pixmap;
use wisp::draw;

pub const CALC_WIDTH: i32 = 360;
pub const CALC_HEIGHT: i32 = 220;

pub struct CalcState {
    pub expression: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

impl CalcState {
    pub fn new(initial: &str) -> Self {
        let mut slf = Self {
            expression: initial.to_string(),
            result: None,
            error: None,
        };
        slf.evaluate();
        slf
    }

    pub fn append_char(&mut self, ch: char) {
        if ch.is_ascii_digit() || "+-*/.%^()".contains(ch) {
            self.expression.push(ch);
            self.evaluate();
        }
    }

    pub fn backspace(&mut self) {
        self.expression.pop();
        self.evaluate();
    }

    pub fn clear(&mut self) {
        self.expression.clear();
        self.result = None;
        self.error = None;
    }

    pub fn evaluate(&mut self) {
        if self.expression.is_empty() {
            self.result = None;
            self.error = None;
            return;
        }
        match evalexpr::eval(&self.expression) {
            Ok(evalexpr::Value::Int(val)) => {
                self.error = None;
                self.result = Some(val.to_string());
            }
            Ok(evalexpr::Value::Float(val)) => {
                if val.is_infinite() || val.is_nan() {
                    self.error = Some("Math error".into());
                    self.result = None;
                } else {
                    self.error = None;
                    self.result = Some(format_float(val));
                }
            }
            Ok(_) => {
                self.error = Some("Not a number".into());
                self.result = None;
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.result = None;
            }
        }
    }
}

fn format_float(val: f64) -> String {
    if val.fract() == 0.0 && val.abs() < 1e15 {
        format!("{}", val as i64)
    } else {
        let s = format!("{:.10}", val);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    }
}

pub fn draw_calculator(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    bg: tiny_skia::Color,
    fg: tiny_skia::Color,
    error_color: tiny_skia::Color,
    dim_color: tiny_skia::Color,
    entry_bg: tiny_skia::Color,
    border: tiny_skia::Color,
    font_family: &str,
    window_radius: f32,
    state: &CalcState,
    cursor_visible: bool,
) {
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let pad = 16.0;
    let inner_w = w - pad * 2.0;

    if window_radius > 0.0 {
        draw::fill_rounded_rect(pixmap, 0.0, 0.0, w, h, [window_radius; 4], bg);
    } else {
        pixmap.fill(bg);
    }

    // Label above input
    let label_y = 14.0;
    wisp_components::label::label(
        pixmap, font_system, swash_cache, pad, label_y,
        &wisp_components::label::LabelProps {
            text: "Expression",
            font_size: 14.0,
            color: dim_color,
            ..Default::default()
        },
        font_family,
    );

    // Input area
    wisp_components::input::input(
        pixmap, font_system, swash_cache, pad, label_y + 22.0, inner_w, 42.0,
        &wisp_components::input::InputProps {
            value: &state.expression,
            cursor_at: state.expression.len(),
            placeholder: "0",
            cursor_visible,
            font_size: 20.0,
            font_family,
            bg: entry_bg,
            fg,
            placeholder_fg: dim_color,
            caret_color: fg,
            border_color: border,
            border_radius: 8.0,
            focused: true,
            ring_color: border,
            ring_width: 2.0,
            size: wisp::Size::Medium,
            prefix_icon: None,
            suffix_icon: None,
            cleanable: false,
            loading: false,
            disabled: false,
            shadow: false,
            selection: None,
            selection_bg: tiny_skia::Color::from_rgba8(59, 130, 246, 80),
            gap: 6.0,
        },
    );

    // Result / error
    let ry = label_y + 22.0 + 42.0 + 14.0;
    if let Some(ref err) = state.error {
        draw::draw_text(pixmap, font_system, swash_cache, err, pad + 4.0, ry.round(), 13.0, font_family, error_color);
    } else if let Some(ref res) = state.result {
        let text = format!("= {}", res);
        let rw = draw::text_width(font_system, &text, 24.0, font_family);
        let rx = if rw > inner_w { pad + 4.0 } else { w - pad - rw };
        draw::draw_text(pixmap, font_system, swash_cache, &text, rx.round(), ry.round(), 24.0, font_family, fg);
    }

    // Footer hint — Badge "Esc" + label "to return"
    let hint_y = h - 28.0;
    wisp_components::badge::badge(
        pixmap, font_system, swash_cache, pad, hint_y,
        &wisp_components::badge::BadgeProps {
            text: "Esc",
            variant: wisp_components::badge::BadgeVariant::Secondary,
            ..Default::default()
        },
        font_family,
    );
    let badge_w = wisp_components::badge::badge_size("Esc", font_system, font_family);
    let label_x = pad + badge_w + 6.0;
    draw::draw_text(
        pixmap, font_system, swash_cache,
        "to return", label_x, hint_y + 4.0, 11.0, font_family, dim_color,
    );
}
