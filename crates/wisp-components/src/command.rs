use crate::list_view::{ListItem as LvItem, ListState};
use tiny_skia::Pixmap;
use wisp::draw;
use wisp::draw::color_from_hex;

/// shadcn-style Command item data (displayed in the list).
#[derive(Debug, Clone)]
pub struct CommandItem<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub keywords: &'a str,
    pub icon: Option<&'a Pixmap>,
}

/// Colors for the Command component, matching shadcn's default theme.
#[derive(Debug, Clone)]
pub struct CommandColors {
    pub bg: tiny_skia::Color,
    pub fg: tiny_skia::Color,
    pub selected_bg: tiny_skia::Color,
    pub selected_fg: tiny_skia::Color,
    pub desc_fg: tiny_skia::Color,
    pub placeholder_fg: tiny_skia::Color,
    pub border: tiny_skia::Color,
    pub caret: tiny_skia::Color,
}

impl Default for CommandColors {
    fn default() -> Self {
        Self {
            bg: color_from_hex("#1e1e2e"),
            fg: color_from_hex("#cdd6f4"),
            selected_bg: color_from_hex("#585b70"),
            selected_fg: color_from_hex("#cdd6f4"),
            desc_fg: color_from_hex("#a6adc8"),
            placeholder_fg: color_from_hex("#6c7086"),
            border: color_from_hex("#45475a"),
            caret: color_from_hex("#89b4fa"),
        }
    }
}

/// Props for drawing the entire Command palette window.
#[derive(Debug, Clone)]
pub struct CommandProps<'a> {
    pub items: &'a [CommandItem<'a>],
    pub selected_index: usize,
    pub scroll_offset: f32,
    pub query: &'a str,
    pub cursor_at: usize,
    pub cursor_visible: bool,
    pub font_size: f32,
    pub desc_size: f32,
    pub font_family: &'a str,
    pub colors: CommandColors,
    pub window_w: f32,
    pub window_h: f32,
    pub border_radius: f32,
}

/// Draw the complete Command palette (shadcn-style).
/// Returns the body bounds (y, h) for mouse hit-testing.
pub fn command_palette(
    pixmap: &mut Pixmap,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    state: &mut ListState,
    props: &CommandProps,
) -> (f32, f32) {
    let padding = 10.0;
    let header_h = 42.0;
    let footer_h = 28.0;
    let w = props.window_w;
    let h = props.window_h;

    // Clear
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    // Window background + border
    draw::fill_rounded_rect(
        pixmap,
        0.0,
        0.0,
        w,
        h,
        [props.border_radius; 4],
        props.colors.bg,
    );
    draw::stroke_rect(pixmap, 0.5, 0.5, w - 1.0, h - 1.0, props.colors.border, 1.0);

    // Header / search input
    let input_y = padding;
    let input_h = header_h;
    let _input_bottom = crate::input::input(
        pixmap,
        font_system,
        swash_cache,
        padding,
        input_y,
        w - padding * 2.0,
        input_h,
        &crate::input::InputProps {
            value: props.query,
            cursor_at: props.cursor_at,
            placeholder: "Type a command...",
            cursor_visible: props.cursor_visible,
            font_size: props.font_size,
            font_family: props.font_family,
            bg: props.colors.bg,
            fg: props.colors.fg,
            placeholder_fg: props.colors.placeholder_fg,
            caret_color: props.colors.caret,
            border_color: props.colors.border,
            border_radius: 6.0,
            focused: true,
            ring_color: props.colors.border,
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

    // Divider after header
    let divider_y = input_y + input_h + 8.0;
    draw::fill_rect(pixmap, 0.0, divider_y, w, 1.0, props.colors.border);

    // Body / list area
    let body_y = divider_y + 4.0;
    let body_h = h - padding - footer_h - body_y - 8.0;

    // Convert CommandItems to ListItems
    let list_items: Vec<LvItem> = props
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| LvItem {
            title: item.title,
            subtitle: item.description,
            prefix_icon: item.icon,
            suffix_icon: None,
            selected: i == props.selected_index,
            disabled: false,
        })
        .collect();

    // Draw list
    crate::list_view::draw_list(
        pixmap,
        font_system,
        swash_cache,
        padding,
        body_y,
        w - padding * 2.0,
        body_h,
        -1.0, // mouse_y: no hover tracking for command palette
        &list_items,
        state,
        props.font_size,
        props.font_family,
        &crate::list_view::ListColors {
            bg: props.colors.bg,
            hover: tiny_skia::Color::from_rgba8(
                (props.colors.selected_bg.red() * 255.0) as u8,
                (props.colors.selected_bg.green() * 255.0) as u8,
                (props.colors.selected_bg.blue() * 255.0) as u8,
                50,
            ),
            active_bg: props.colors.selected_bg,
            active_border: props.colors.border,
            active_highlight: true,
            fg: props.colors.fg,
            selected_fg: props.colors.selected_fg,
            desc_fg: props.colors.desc_fg,
            selected_desc_fg: props.colors.selected_fg,
        },
    );

    // Scrollbar
    let total_h = props.items.len() as f32 * 50.0; // approximate row height
    crate::scroll_area::scrollbar(
        pixmap,
        w - 10.0,
        body_y,
        body_h,
        body_h / total_h,
        props.scroll_offset,
        total_h,
        6.0,
        &crate::scroll_area::ScrollBarColors::default(),
    );

    (body_y, body_h)
}
