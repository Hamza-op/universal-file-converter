use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

const BG_APP: Color32 = Color32::from_rgb(11, 15, 24);
const BG_PANEL: Color32 = Color32::from_rgb(16, 22, 34);
const BG_CARD: Color32 = Color32::from_rgb(22, 29, 43);
const BG_CARD_ALT: Color32 = Color32::from_rgb(28, 37, 54);
const BG_HOVER: Color32 = Color32::from_rgb(34, 46, 67);
const BORDER_DARK: Color32 = Color32::from_rgb(52, 66, 92);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(232, 238, 247);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(142, 156, 182);

pub fn dark_theme() -> Visuals {
    let mut v = Visuals::dark();

    v.window_fill = BG_APP;
    v.panel_fill = BG_PANEL;
    v.faint_bg_color = BG_CARD;
    v.extreme_bg_color = BG_APP;

    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0, ACCENT_HI);
    v.hyperlink_color = ACCENT_HI;

    v.widgets.noninteractive.bg_fill = BG_CARD;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(12);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_DARK);

    v.widgets.inactive.bg_fill = BG_CARD_ALT;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.inactive.corner_radius = CornerRadius::same(12);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_DARK);

    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.hovered.corner_radius = CornerRadius::same(12);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);

    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(12);

    v.widgets.open.bg_fill = BG_HOVER;
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.open.corner_radius = CornerRadius::same(12);

    v.window_stroke = Stroke::new(1.0, BORDER_DARK);
    v.window_corner_radius = CornerRadius::same(18);

    v
}

// ──── Light palette ────
pub fn light_theme() -> Visuals {
    let mut v = Visuals::light();
    v.window_fill = Color32::from_rgb(244, 247, 252);
    v.panel_fill = Color32::from_rgb(244, 247, 252);
    v.faint_bg_color = Color32::from_rgb(234, 239, 248);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(12);
    v.widgets.inactive.corner_radius = CornerRadius::same(12);
    v.widgets.hovered.corner_radius = CornerRadius::same(12);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(12);
    v
}

pub const ACCENT: Color32 = Color32::from_rgb(17, 132, 220);
pub const ACCENT_HI: Color32 = Color32::from_rgb(75, 180, 255);
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(110, 193, 255);
pub const ACCENT_WARM: Color32 = Color32::from_rgb(255, 167, 38);
pub const SUCCESS: Color32 = Color32::from_rgb(34, 197, 121);
pub const ERROR: Color32 = Color32::from_rgb(240, 91, 86);
pub const TEXT_DIM: Color32 = Color32::from_rgb(148, 153, 172);
pub const PROGRESS_BG: Color32 = Color32::from_rgb(33, 42, 59);
pub const PANEL_STROKE: Color32 = BORDER_DARK;

pub fn surface_primary(dark_mode: bool) -> Color32 {
    if dark_mode {
        BG_CARD
    } else {
        Color32::from_rgb(250, 252, 255)
    }
}

pub fn surface_secondary(dark_mode: bool) -> Color32 {
    if dark_mode {
        BG_CARD_ALT
    } else {
        Color32::from_rgb(236, 241, 248)
    }
}

pub fn surface_tertiary(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(66, 76, 96)
    } else {
        Color32::from_rgb(228, 234, 244)
    }
}

pub fn soft_fill(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(70, 79, 98)
    } else {
        Color32::from_rgb(220, 227, 239)
    }
}

pub fn inactive_chip(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(52, 62, 82)
    } else {
        Color32::from_rgb(216, 224, 238)
    }
}

pub fn inactive_tab(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(44, 53, 70)
    } else {
        Color32::from_rgb(224, 231, 242)
    }
}

pub fn text_primary(dark_mode: bool) -> Color32 {
    if dark_mode {
        TEXT_PRIMARY
    } else {
        Color32::from_rgb(28, 35, 49)
    }
}

pub fn text_secondary(dark_mode: bool) -> Color32 {
    if dark_mode {
        TEXT_DIM
    } else {
        Color32::from_rgb(96, 108, 128)
    }
}

pub fn configure_fonts(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(11.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
    ].into();

    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12);

    ctx.set_style(style);
}
