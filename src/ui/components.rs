use egui::{Align, Color32, Layout, Response, RichText, Rounding, Stroke, Ui};
use crate::app::{AppScreen, AppState};
use crate::ui::theme::Theme;

/// Renders the top navigation header bar.
pub fn render_header(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        // App Title & Frontier Logo
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("BLACKWATER")
                    .font(Theme::heading_font())
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
            );
            ui.label(
                RichText::new("PHOTO EXTRACTOR")
                    .font(Theme::mono_font())
                    .color(Theme::ACCENT_GOLD),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Settings button
            if state.current_screen != AppScreen::Processing {
                let settings_btn = if state.current_screen == AppScreen::Settings {
                    ui.button(RichText::new("\u{2715} Close Settings").font(Theme::body_font()).color(Theme::TEXT_PRIMARY))
                } else {
                    ui.button(RichText::new("\u{2699} Settings").font(Theme::body_font()).color(Theme::TEXT_SECONDARY))
                };

                if settings_btn.clicked() {
                    if state.current_screen == AppScreen::Settings {
                        state.current_screen = AppScreen::Start;
                    } else {
                        state.current_screen = AppScreen::Settings;
                    }
                }
            }

            ui.add_space(8.0);

            // Local Only Privacy Badge
            render_pill_badge(ui, "LOCAL ONLY", Theme::SUCCESS, Theme::BG_SECONDARY);
        });
    });

    ui.add_space(4.0);
    ui.painter().line_segment(
        [
            egui::pos2(ui.min_rect().min.x, ui.cursor().top()),
            egui::pos2(ui.min_rect().max.x, ui.cursor().top()),
        ],
        Stroke::new(1.0_f32, Theme::BORDER_SUBTLE),
    );
    ui.add_space(10.0);
}

/// Renders the system and memory statistics footer.
pub fn render_footer(ui: &mut Ui, state: &mut AppState) {
    ui.painter().line_segment(
        [
            egui::pos2(ui.min_rect().min.x, ui.cursor().top()),
            egui::pos2(ui.min_rect().max.x, ui.cursor().top()),
        ],
        Stroke::new(1.0_f32, Theme::BORDER_SUBTLE),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        // Privacy guarantee
        ui.label(
            RichText::new("Offline \u{2022} No Network \u{2022} Direct File I/O")
                .font(Theme::mono_font())
                .color(Theme::TEXT_MUTED),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Real Windows RAM info
            if state.current_working_set_mb > 0.0 {
                let ram_text = format!(
                    "RAM: {:.1} MB (Peak: {:.1} MB)",
                    state.current_working_set_mb, state.peak_working_set_mb
                );
                ui.label(
                    RichText::new(ram_text)
                        .font(Theme::mono_font())
                        .color(Theme::TEXT_SECONDARY),
                );
            }

            ui.label(
                RichText::new(format!(
                    "Mode: {} ({} worker{})",
                    state.settings.memory_mode.label(),
                    state.settings.worker_count,
                    if state.settings.worker_count > 1 { "s" } else { "" }
                ))
                .font(Theme::mono_font())
                .color(Theme::ACCENT_GOLD),
            );
        });
    });
}

/// Renders a stylish rounded status pill badge.
pub fn render_pill_badge(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    let frame = egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, fg.linear_multiply(0.5)))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(10.0, 4.0));

    frame.show(ui, |ui| {
        ui.label(RichText::new(text).font(Theme::mono_font()).color(fg).strong());
    });
}

/// Renders an action card with an icon, title, description, and button.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(
        RichText::new(text)
            .font(Theme::subhead_font())
            .color(Color32::BLACK)
            .strong(),
    )
    .fill(Theme::ACCENT_GOLD)
    .stroke(Stroke::new(1.0_f32, Theme::ACCENT_HOVER))
    .rounding(Rounding::same(5.0))
    .min_size(egui::vec2(160.0, 36.0));

    ui.add(button)
}

pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(
        RichText::new(text)
            .font(Theme::subhead_font())
            .color(Theme::TEXT_PRIMARY),
    )
    .fill(Theme::BG_SECONDARY)
    .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
    .rounding(Rounding::same(5.0))
    .min_size(egui::vec2(130.0, 34.0));

    ui.add(button)
}

pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(
        RichText::new(text)
            .font(Theme::subhead_font())
            .color(Color32::WHITE)
            .strong(),
    )
    .fill(Color32::from_rgb(153, 27, 27))
    .stroke(Stroke::new(1.0_f32, Theme::ERROR))
    .rounding(Rounding::same(5.0))
    .min_size(egui::vec2(120.0, 34.0));

    ui.add(button)
}
