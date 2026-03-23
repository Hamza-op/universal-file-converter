use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

// ──── Obsidian Rose — Deep charcoal with warm rose/coral accents ────

// Dark palette core
const BG_APP: Color32 = Color32::from_rgb(14, 12, 16);
const BG_PANEL: Color32 = Color32::from_rgb(22, 19, 26);
const BG_CARD: Color32 = Color32::from_rgb(30, 26, 35);
const BG_CARD_ALT: Color32 = Color32::from_rgb(38, 33, 44);
const BG_HOVER: Color32 = Color32::from_rgb(50, 42, 58);
const BORDER_DARK: Color32 = Color32::from_rgb(62, 52, 72);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 234, 228);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(162, 148, 138);

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
    v.widgets.noninteractive.corner_radius = CornerRadius::same(10);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_DARK);

    v.widgets.inactive.bg_fill = BG_CARD_ALT;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.inactive.corner_radius = CornerRadius::same(10);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_DARK);

    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.hovered.corner_radius = CornerRadius::same(10);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);

    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(10);

    v.widgets.open.bg_fill = BG_HOVER;
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.open.corner_radius = CornerRadius::same(10);

    v.window_stroke = Stroke::new(1.0, BORDER_DARK);
    v.window_corner_radius = CornerRadius::same(16);

    v
}

// ──── Light palette — Warm parchment with rose accents ────
pub fn light_theme() -> Visuals {
    let mut v = Visuals::light();
    v.window_fill = Color32::from_rgb(252, 248, 244);
    v.panel_fill = Color32::from_rgb(252, 248, 244);
    v.faint_bg_color = Color32::from_rgb(244, 238, 232);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(10);
    v.widgets.inactive.corner_radius = CornerRadius::same(10);
    v.widgets.hovered.corner_radius = CornerRadius::same(10);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(10);
    v
}

// ──── Accent colors — Rose / Coral spectrum ────
pub const ACCENT: Color32 = Color32::from_rgb(212, 78, 88);        // Warm rose
pub const ACCENT_HI: Color32 = Color32::from_rgb(244, 114, 120);   // Bright coral
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(248, 152, 148); // Soft peach
pub const ACCENT_WARM: Color32 = Color32::from_rgb(242, 172, 68);  // Warm amber
pub const SUCCESS: Color32 = Color32::from_rgb(72, 199, 142);      // Mint green
pub const ERROR: Color32 = Color32::from_rgb(232, 72, 85);         // Vivid red
pub const TEXT_DIM: Color32 = Color32::from_rgb(136, 124, 118);    // Warm muted
pub const PROGRESS_BG: Color32 = Color32::from_rgb(36, 30, 42);    // Deep violet-black
pub const PANEL_STROKE: Color32 = BORDER_DARK;

// ──── Semantic surface helpers ────

pub fn surface_primary(dark_mode: bool) -> Color32 {
    if dark_mode {
        BG_CARD
    } else {
        Color32::from_rgb(255, 252, 249)
    }
}

pub fn surface_secondary(dark_mode: bool) -> Color32 {
    if dark_mode {
        BG_CARD_ALT
    } else {
        Color32::from_rgb(246, 240, 234)
    }
}

pub fn surface_tertiary(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(52, 44, 60)
    } else {
        Color32::from_rgb(238, 232, 226)
    }
}

pub fn soft_fill(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(58, 48, 66)
    } else {
        Color32::from_rgb(232, 224, 218)
    }
}

pub fn inactive_chip(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(46, 38, 54)
    } else {
        Color32::from_rgb(228, 222, 216)
    }
}

pub fn inactive_tab(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(40, 34, 48)
    } else {
        Color32::from_rgb(236, 230, 224)
    }
}

pub fn text_primary(dark_mode: bool) -> Color32 {
    if dark_mode {
        TEXT_PRIMARY
    } else {
        Color32::from_rgb(38, 30, 24)
    }
}

pub fn text_secondary(dark_mode: bool) -> Color32 {
    if dark_mode {
        TEXT_DIM
    } else {
        Color32::from_rgb(108, 96, 88)
    }
}

pub fn configure_fonts(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(19.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(11.5, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
    ].into();

    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(14);

    ctx.set_style(style);
}
