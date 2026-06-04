use tiny_skia::Pixmap;
use wisp::draw;

use crate::kbd::{kbd, measure_kbd, KbdProps};

#[derive(Debug, Clone)]
pub struct KbdComboProps<'a> {
    pub keys: &'a str,
    pub font_size: f32,
    pub separator: &'a str,
    pub padding_x: f32,
}

impl<'a> Default for KbdComboProps<'a> {
    fn default() -> Self {
        Self {
            keys: "",
            font_size: 11.0,
            separator: "+",
            padding_x: 4.0,
        }
    }
}

/// Render a key combo (e.g. "Super+Alt+S") as a row of `kbd` chips separated by `+` glyphs.
/// Returns the total (width, height) of the rendered combo.
pub fn kbd_combo(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    x: f32,
    y: f32,
    props: &KbdComboProps,
    font_family: &str,
) -> (f32, f32) {
    let parts: Vec<&str> = if props.keys.contains('+') {
        props.keys.split('+').map(str::trim).filter(|s| !s.is_empty()).collect()
    } else {
        vec![props.keys]
    };

    if parts.is_empty() {
        return (0.0, 0.0);
    }

    let mut cursor_x = x;
    let mut total_w = 0.0;
    let mut max_h = 0.0;
    let sep_w = if parts.len() > 1 {
        draw::text_width(font_system, props.separator, props.font_size, font_family)
    } else {
        0.0
    };
    let sep_advance = sep_w + 4.0;

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            draw::draw_text(
                pixmap,
                font_system,
                swash_cache,
                props.separator,
                cursor_x,
                y + 3.0,
                props.font_size,
                font_family,
                draw::color_from_hex("#888888"),
            );
            cursor_x += sep_advance;
            total_w += sep_advance;
        }
        let (w, h) = kbd(
            pixmap,
            font_system,
            swash_cache,
            cursor_x,
            y,
            &KbdProps {
                keys: part,
                font_size: props.font_size,
                padding_x: props.padding_x,
            },
            font_family,
        );
        cursor_x += w;
        total_w += w;
        if h > max_h {
            max_h = h;
        }
    }

    (total_w, max_h)
}

/// Measure the rendered width of a key combo without drawing it.
pub fn measure_kbd_combo(
    font_system: &mut cosmic_text::FontSystem,
    props: &KbdComboProps,
    font_family: &str,
) -> f32 {
    let parts: Vec<&str> = if props.keys.contains('+') {
        props.keys.split('+').map(str::trim).filter(|s| !s.is_empty()).collect()
    } else {
        vec![props.keys]
    };
    if parts.is_empty() {
        return 0.0;
    }
    let sep_advance = if parts.len() > 1 {
        draw::text_width(font_system, props.separator, props.font_size, font_family) + 4.0
    } else {
        0.0
    };
    let mut total = 0.0;
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            total += sep_advance;
        }
        let (w, _h) = measure_kbd(font_system, part, props.font_size, props.padding_x, font_family);
        total += w;
    }
    total
}

#[cfg(test)]
mod tests {
    #[test]
    fn splits_on_plus() {
        assert_eq!(
            "Super+Alt+S".split('+').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
            vec!["Super", "Alt", "S"]
        );
    }

    #[test]
    fn no_plus_is_single_part() {
        let parts: Vec<&str> = if "F1".contains('+') {
            "F1".split('+').collect()
        } else {
            vec!["F1"]
        };
        assert_eq!(parts, vec!["F1"]);
    }

    #[test]
    fn empty_string_yields_no_parts() {
        let parts: Vec<&str> = if "".contains('+') {
            "".split('+').map(str::trim).filter(|s| !s.is_empty()).collect()
        } else {
            vec![""]
        };
        assert!(parts.iter().any(|p| p.is_empty()));
    }
}
