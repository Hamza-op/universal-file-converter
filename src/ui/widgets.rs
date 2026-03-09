use egui::{Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use crate::ui::theme;

/// Accent button with gradient effect
pub fn accent_button(ui: &mut Ui, text: &str) -> Response {
    let btn = egui::Button::new(
        egui::RichText::new(text).color(Color32::WHITE).size(13.0).strong(),
    )
    .fill(theme::ACCENT)
    .stroke(Stroke::new(1.0, theme::ACCENT_HI))
    .corner_radius(CornerRadius::same(12))
    .min_size(Vec2::new(108.0, 30.0));
    ui.add(btn)
}

/// Ghost/outline button
pub fn outline_button(ui: &mut Ui, text: &str) -> Response {
    let btn = egui::Button::new(egui::RichText::new(text).size(12.0))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, theme::PANEL_STROKE))
        .corner_radius(CornerRadius::same(12))
        .min_size(Vec2::new(0.0, 28.0));
    ui.add(btn)
}

/// Red danger button
pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    let btn = egui::Button::new(
        egui::RichText::new(text).color(Color32::WHITE).size(12.0),
    )
    .fill(theme::ERROR)
    .corner_radius(CornerRadius::same(12))
    .min_size(Vec2::new(0.0, 28.0));
    ui.add(btn)
}

/// Custom animated progress bar with rounded ends
pub fn smooth_progress(ui: &mut Ui, pct: f32, label: &str) {
    let desired_size = Vec2::new(ui.available_width(), 24.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let rounding = CornerRadius::same(12);

        ui.painter().rect_filled(rect, rounding, theme::PROGRESS_BG);
        ui.painter().rect_stroke(
            rect,
            rounding,
            Stroke::new(1.0, theme::PANEL_STROKE),
            StrokeKind::Inside,
        );

        let fill_pct = pct.clamp(0.0, 1.0);
        if fill_pct > 0.005 {
            let fill_width = rect.width() * fill_pct;
            let fill_rect = Rect::from_min_size(
                rect.min,
                Vec2::new(fill_width.max(22.0), rect.height()),
            )
            .intersect(rect);

            ui.painter().rect_filled(fill_rect, rounding, theme::ACCENT);
            let highlight_rect = Rect::from_min_size(
                fill_rect.min,
                Vec2::new(fill_rect.width(), fill_rect.height() * 0.4),
            );
            ui.painter().rect_filled(
                highlight_rect,
                CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                Color32::from_rgba_unmultiplied(255, 255, 255, 22),
            );
        }

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );
    }
}

pub fn drop_zone(ui: &mut Ui, is_hovering: bool, has_files: bool) -> Response {
    let height = if has_files { 76.0 } else { 96.0 };
    let desired_size = Vec2::new(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let dark_mode = ui.visuals().dark_mode;
        
        let mut bg = theme::surface_secondary(dark_mode);
        let mut border = theme::PANEL_STROKE;
        let text_color = theme::text_primary(dark_mode);
        let mut icon_color = theme::TEXT_DIM;
        
        if is_hovering {
            bg = if dark_mode { Color32::from_rgb(24, 48, 78) } else { Color32::from_rgb(217, 235, 252) };
            border = theme::ACCENT_LIGHT;
            icon_color = theme::ACCENT_LIGHT;
        }

        ui.painter().rect_filled(rect, CornerRadius::same(14), bg);
        
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(14),
            Stroke::new(if is_hovering { 1.5 } else { 1.0 }, border),
            StrokeKind::Inside,
        );

        let title = if has_files {
            "Add more media"
        } else {
            "Drop media anywhere"
        };
        let subtitle = if has_files {
            "Click to add files or drop a folder"
        } else {
            "Drag files here, or click to browse"
        };

        let icon_y = if has_files { 20.0 } else { 28.0 };
        let title_y = if has_files { 40.0 } else { 54.0 };
        let sub_y = if has_files { 56.0 } else { 74.0 };

        ui.painter().text(
            rect.center_top() + egui::vec2(0.0, icon_y),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(if has_files { 20.0 } else { 26.0 }),
            icon_color,
        );
        ui.painter().text(
            rect.center_top() + egui::vec2(0.0, title_y),
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(14.0),
            text_color,
        );
        ui.painter().text(
            rect.center_top() + egui::vec2(0.0, sub_y),
            egui::Align2::CENTER_CENTER,
            subtitle,
            egui::FontId::proportional(11.0),
            theme::TEXT_DIM,
        );
    }

    response
}
