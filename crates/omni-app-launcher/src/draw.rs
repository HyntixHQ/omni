use tiny_skia::Pixmap;
use wisp::draw::{self, color_from_hex};
use wisp_components::list_view::{draw_list, ListItem, ListColors};
use cosmic_text::{FontSystem, SwashCache};

use crate::config::Config;
use crate::state::OmniApp;

pub const HEADER_HEIGHT: f32 = 42.0;
pub const FOOTER_HEIGHT: f32 = 28.0;
pub const PADDING: f32 = 10.0;
pub const GAP: f32 = 8.0;

pub fn compute_row_height(font_system: &mut FontSystem, font_size: f32, font_family: &str) -> f32 {
    let desc_size = font_size - 2.0;
    let (n_ascent, n_descent) = draw::text_metrics(font_system, "Ag", font_size, font_family);
    let (d_ascent, d_descent) = draw::text_metrics(font_system, "Ag", desc_size, font_family);
    let name_h = n_ascent + n_descent;
    let desc_h = d_ascent + d_descent;
    name_h + desc_h + 2.0 + 12.0
}

pub fn draw_launcher_frame(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    app: &mut OmniApp,
    cfg: &Config,
    cursor_visible: bool,
    mouse_y: f32,
) {
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let font_size = cfg.font.size as f32;
    let font_family = &cfg.font.family;

    // Window background (like GPUI — direct fill, no Scaffold container)
    let bg_color = color_from_hex(&cfg.theme.bg);
    let window_radius = cfg.theme.border_radius as f32;
    if window_radius > 0.0 {
        draw::fill_rounded_rect(pixmap, 0.0, 0.0, w, h, [window_radius; 4], bg_color);
    } else {
        pixmap.fill(bg_color);
    }

    // Header / search input
    let header_y = PADDING;
    let header_h = HEADER_HEIGHT;
    let inner_w = w - PADDING * 2.0;

    app.mouse.clear();
    app.mouse.register(
        wisp::events::RegionId::Header, header_y, header_h,
        wisp::CursorStyle::IBeam,
    );
    draw_search_input(
        pixmap, font_system, swash_cache, app, cfg, cursor_visible,
        PADDING, header_y, inner_w, header_h,
    );

    // Body / list area
    let body_y = header_y + header_h + GAP;
    let body_h = h - PADDING - body_y;
    app.mouse.register(
        wisp::events::RegionId::Body, body_y, body_h,
        wisp::CursorStyle::PointingHand,
    );

    // Pre-load icons
    for result in &app.results {
        let icon_name = result.entry.icon.as_deref().unwrap_or("application-x-executable");
        app.icon_cache.get_icon(icon_name);
    }

    let list_colors = ListColors {
        bg: color_from_hex(&cfg.theme.bg),
        hover: {
            let mut c = color_from_hex(&cfg.theme.selected_bg);
            c = tiny_skia::Color::from_rgba8(
                (c.red() * 255.0) as u8,
                (c.green() * 255.0) as u8,
                (c.blue() * 255.0) as u8,
                50,
            );
            c
        },
        active_bg: color_from_hex(&cfg.theme.selected_bg),
        active_border: color_from_hex(&cfg.theme.border),
        active_highlight: true,
        fg: color_from_hex(&cfg.theme.fg),
        selected_fg: color_from_hex(&cfg.theme.selected_fg),
        desc_fg: color_from_hex(&cfg.theme.desc_fg),
        selected_desc_fg: color_from_hex(&cfg.theme.selected_fg),
    };

    let list_items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let icon_name = result.entry.icon.as_deref().unwrap_or("application-x-executable");
            ListItem {
                title: &result.entry.name,
                subtitle: result.entry.description.as_deref(),
                prefix_icon: app.icon_cache.get_icon_ref(icon_name),
                suffix_icon: None,
                selected: i == app.selected_index,
                disabled: false,
            }
        })
        .collect();

    draw_list(
        pixmap, font_system, swash_cache,
        PADDING, body_y, inner_w, body_h, mouse_y,
        &list_items, &mut app.list_state,
        font_size, font_family, &list_colors,
    );
}

fn draw_search_input(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    app: &mut OmniApp,
    cfg: &Config,
    cursor_visible: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    wisp_components::input::input(
        pixmap, font_system, swash_cache, x, y, w, h,
        &wisp_components::input::InputProps {
            value: app.query(),
            cursor_at: app.cursor_at(),
            placeholder: "Type to search...",
            cursor_visible,
            font_size: cfg.font.size as f32,
            font_family: &cfg.font.family,
            bg: color_from_hex(&cfg.theme.entry_bg),
            fg: color_from_hex(&cfg.theme.fg),
            placeholder_fg: color_from_hex(&cfg.theme.placeholder_fg),
            caret_color: color_from_hex(&cfg.theme.caret),
            border_color: color_from_hex(&cfg.theme.border),
            border_radius: 6.0,
            focused: true,
            ring_color: color_from_hex(&cfg.theme.border),
            ring_width: 3.0,
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
}
