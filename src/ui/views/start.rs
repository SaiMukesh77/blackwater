use egui::{Align, Layout, RichText, Rounding, Stroke, Ui, Vec2};
use rfd::FileDialog;
use crate::app::AppState;
use crate::ui::components::{primary_button, secondary_button};
use crate::ui::theme::Theme;

pub fn render_start_view(ui: &mut Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);

        // Cinematic Title Banner
        ui.label(
            RichText::new("BLACKWATER")
                .font(Theme::title_font())
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );

        ui.add_space(4.0);
        ui.label(
            RichText::new("Private photo extraction, built for your machine.")
                .font(Theme::subhead_font())
                .color(Theme::ACCENT_GOLD),
        );

        ui.add_space(16.0);

        // Hardware / RAM notification banner if 8GB or less detected
        if let Some(notification) = &state.ram_mode_detected_notification {
            egui::Frame::none()
                .fill(Theme::BG_ACCENT_SUBTLE)
                .stroke(Stroke::new(1.0_f32, Theme::ACCENT_GOLD))
                .rounding(Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("\u{2139} ")
                                .font(Theme::subhead_font())
                                .color(Theme::ACCENT_AMBER),
                        );
                        ui.label(
                            RichText::new(notification)
                                .font(Theme::body_font())
                                .color(Theme::TEXT_PRIMARY),
                        );
                    });
                });
            ui.add_space(16.0);
        }

        // Action Buttons Row
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(14.0, 0.0);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space((ui.available_width() - 480.0).max(0.0) / 2.0);

                if primary_button(ui, "\u{1F5C1} Select Files").clicked() {
                    if let Some(files) = FileDialog::new().pick_files() {
                        if !files.is_empty() {
                            state.start_scanning(files);
                        }
                    }
                }

                if secondary_button(ui, "\u{1F4C1} Select Folder").clicked() {
                    if let Some(folder) = FileDialog::new().pick_folder() {
                        state.start_scanning(vec![folder]);
                    }
                }

                if secondary_button(ui, "\u{1F5C3} Select ZIP").clicked() {
                    if let Some(zip_file) = FileDialog::new()
                        .add_filter("ZIP Archives", &["zip"])
                        .pick_file()
                    {
                        state.start_scanning(vec![zip_file]);
                    }
                }
            });
        });

        ui.add_space(30.0);

        // Drag and Drop Zone
        let drop_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(30.0, 24.0));

        drop_frame.show(ui, |ui| {
            ui.set_width(520.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("\u{2935} Drop Photos, Folders, or ZIPs here")
                        .font(Theme::subhead_font())
                        .color(Theme::TEXT_SECONDARY),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Handles PRDR files, raw container bins, dat files, and archives")
                        .font(Theme::mono_font())
                        .color(Theme::TEXT_MUTED),
                );
            });
        });

        ui.add_space(24.0);

        // Helpful default path hint for RDR2
        let hint_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0));

        hint_frame.show(ui, |ui| {
            ui.set_width(520.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Default RDR2 Camera Location:")
                            .font(Theme::body_font())
                            .color(Theme::ACCENT_GOLD)
                            .strong(),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("%USERPROFILE%\\Documents\\Rockstar Games\\Red Dead Redemption 2\\Profiles\\<ProfileID>")
                        .font(Theme::mono_font())
                        .color(Theme::TEXT_SECONDARY),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("RDR2 saves photos as 'PRDR...' container files without extensions. Blackwater parses their binary structure directly.")
                        .font(Theme::body_font())
                        .color(Theme::TEXT_MUTED),
                );
            });
        });
    });
}
