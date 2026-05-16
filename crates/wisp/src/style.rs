/// Size variant matching GPUI-component's Size enum.
/// Framework-level sizing — all widgets inherit from this.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Size {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    /// Horizontal padding for input-like widgets (GPUI: input_px)
    pub fn padding_x(self) -> f32 {
        match self {
            Size::Large => 16.0,
            Size::Medium => 12.0,
            Size::Small => 8.0,
            Size::XSmall => 4.0,
        }
    }

    /// Vertical padding for input-like widgets.
    /// shadcn py-1 = 4px vertical padding.
    pub fn padding_y(self) -> f32 {
        match self {
            Size::Large => 8.0,
            Size::Medium => 4.0,
            Size::Small => 2.0,
            Size::XSmall => 0.0,
        }
    }

    /// Font size based on size variant.
    /// shadcn: text-xs=12px, text-sm=14px, text-base=16px, text-lg=18px
    /// Input mapping: XSmall→text-xs, Small/Medium→text-sm, Large→text-base
    pub fn font_size(self) -> f32 {
        match self {
            Size::Large => 16.0,
            Size::Medium => 14.0,
            Size::Small => 14.0,
            Size::XSmall => 12.0,
        }
    }

    /// Line height matching Tailwind text-size line-heights.
    /// shadcn: text-xs→1rem(16px), text-sm→1.25rem(20px), text-base→1.5rem(24px)
    /// Note: Tailwind uses fixed line-heights, not proportional to font-size.
    pub fn line_height(self) -> f32 {
        match self {
            Size::Large => 24.0,
            Size::Medium | Size::Small => 20.0,
            Size::XSmall => 16.0,
        }
    }

    /// Gap between elements (GPUI: Small→4, Large→8, _→6)
    pub fn gap(self) -> f32 {
        match self {
            Size::Large => 8.0,
            Size::Medium => 6.0,
            Size::Small => 4.0,
            Size::XSmall => 6.0,
        }
    }

    /// Button text size mapping (GPUI: button_text_size)
    pub fn button_text_size(self) -> f32 {
        match self {
            Size::Large => 16.0,
            Size::Medium | Size::Small => 14.0,
            Size::XSmall => 12.0,
        }
    }

    /// List item horizontal padding
    pub fn list_padding_x(self) -> f32 {
        match self {
            Size::Large => 16.0,
            Size::Medium => 12.0,
            Size::Small => 8.0,
            Size::XSmall => 4.0,
        }
    }

    /// List item vertical padding
    pub fn list_padding_y(self) -> f32 {
        match self {
            Size::Large => 8.0,
            Size::Medium => 6.0,
            Size::Small => 4.0,
            Size::XSmall => 2.0,
        }
    }
}
