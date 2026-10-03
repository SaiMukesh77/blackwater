use egui::{Align, Layout, ProgressBar, RichText, Rounding, Stroke, Ui};
use crate::app::AppState;
use crate::ui::components::danger_button;
use crate::ui::theme::Theme;

pub fn render_scan_view(ui: &mut Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);

        ui.label(
            RichText::new("Scanning files...")
                .font(Theme::heading_font())
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new("Inspecting binary headers for embedded JPEG streams...")
                .font(Theme::body_font())
                .color(Theme::TEXT_SECONDARY),
        );

        ui.add_space(24.0);

        let valid_count = state.scanned_items.iter().filter(|i| i.is_valid).count();
        let invalid_count = state.scanned_items.len() - valid_count;
        let total_count = state.scanned_items.len();

        // Stats Card
        let card = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(32.0, 20.0));

        card.show(ui, |ui| {
            ui.set_width(420.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Files discovered:").font(Theme::subhead_font()).color(Theme::TEXT_SECONDARY));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{total_count}")).font(Theme::subhead_font()).color(Theme::TEXT_PRIMARY).strong());
                    });
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Valid RDR2 photos:").font(Theme::subhead_font()).color(Theme::SUCCESS));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{valid_count}")).font(Theme::subhead_font()).color(Theme::SUCCESS).strong());
                    });
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Non-photos / invalid:").font(Theme::subhead_font()).color(Theme::TEXT_MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{invalid_count}")).font(Theme::subhead_font()).color(Theme::TEXT_MUTED));
                    });
                });
            });
        });

        ui.add_space(24.0);

        // Animated progress bar
        let progress_bar = ProgressBar::new(0.99)
            .animate(true)
            .fill(Theme::ACCENT_GOLD)
            .desired_width(420.0)
            .desired_height(6.0);
        ui.add(progress_bar);

        ui.add_space(30.0);

        if danger_button(ui, "Cancel Scan").clicked() {
            state.cancel_scan();
        }
    });
}
