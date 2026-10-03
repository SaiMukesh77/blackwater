use egui::{Align, Layout, RichText, Rounding, Stroke, Ui, Vec2};
use rfd::FileDialog;
use crate::app::{AppScreen, AppState};
use crate::models::{MemoryMode, OutputFormat};
use crate::ui::components::{primary_button, secondary_button};
use crate::ui::theme::Theme;

pub fn render_format_view(ui: &mut Ui, state: &mut AppState) {
    let valid_count = state.scanned_items.iter().filter(|i| i.is_valid).count();

    ui.vertical_centered(|ui| {
        ui.add_space(10.0);

        ui.label(
            RichText::new("Configure Extraction & Output")
                .font(Theme::heading_font())
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );

        ui.add_space(2.0);
        ui.label(
            RichText::new(format!("{valid_count} valid RDR2 photo{} ready for extraction", if valid_count == 1 { "" } else { "s" }))
                .font(Theme::subhead_font())
                .color(Theme::ACCENT_GOLD),
        );

        ui.add_space(16.0);

        // Format Card
        let format_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(24.0, 16.0));

        format_frame.show(ui, |ui| {
            ui.set_width(560.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Output Format")
                        .font(Theme::subhead_font())
                        .color(Theme::TEXT_PRIMARY)
                        .strong(),
                );
                ui.add_space(6.0);

                // Radio: JPEG
                ui.horizontal(|ui| {
                    ui.radio_value(&mut state.settings.output_format, OutputFormat::Jpeg, "");
                    ui.vertical(|ui| {
                        ui.label(RichText::new("JPEG (.jpg)").font(Theme::body_font()).color(Theme::TEXT_PRIMARY).strong());
                        ui.label(
                            RichText::new("Extract original embedded JPEG without re-encoding. 100% byte-exact preservation.")
                                .font(Theme::mono_font())
                                .color(Theme::TEXT_SECONDARY),
                        );
                    });
                });

                ui.add_space(10.0);

                // Radio: PNG
                ui.horizontal(|ui| {
                    ui.radio_value(&mut state.settings.output_format, OutputFormat::Png, "");
                    ui.vertical(|ui| {
                        ui.label(RichText::new("PNG (.png)").font(Theme::body_font()).color(Theme::TEXT_PRIMARY).strong());
                        ui.label(
                            RichText::new("Convert the extracted JPEG to lossless PNG.")
                                .font(Theme::mono_font())
                                .color(Theme::TEXT_SECONDARY),
                        );
                    });
                });

                // PNG Explanation Note
                if state.settings.output_format == OutputFormat::Png {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(Theme::BG_TERTIARY)
                        .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("\u{2139} PNG is lossless, but it cannot restore information already discarded by the original JPEG compression.")
                                    .font(Theme::body_font())
                                    .color(Theme::WARNING),
                            );
                        });
                }
            });
        });

        ui.add_space(14.0);

        // Destination Folder Card
        let dest_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(24.0, 16.0));

        dest_frame.show(ui, |ui| {
            ui.set_width(560.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Output Directory")
                        .font(Theme::subhead_font())
                        .color(Theme::TEXT_PRIMARY)
                        .strong(),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    let path_str = state.settings.output_directory.to_string_lossy().to_string();
                    egui::Frame::none()
                        .fill(Theme::BG_TERTIARY)
                        .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                        .show(ui, |ui| {
                            ui.set_width(380.0);
                            ui.label(RichText::new(path_str).font(Theme::mono_font()).color(Theme::TEXT_PRIMARY));
                        });

                    if secondary_button(ui, "Choose Folder").clicked() {
                        if let Some(folder) = FileDialog::new()
                            .set_directory(&state.settings.output_directory)
                            .pick_folder()
                        {
                            state.settings.output_directory = folder;
                            state.settings.save();
                        }
                    }
                });
            });
        });

        ui.add_space(14.0);

        // Concurrency / Low RAM control
        let perf_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(24.0, 12.0));

        perf_frame.show(ui, |ui| {
            ui.set_width(560.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Worker Concurrency:").font(Theme::body_font()).color(Theme::TEXT_SECONDARY));

                let is_1 = state.settings.worker_count == 1;
                if ui.selectable_label(is_1, "1 Worker (Lowest RAM)").clicked() {
                    state.settings.worker_count = 1;
                    state.settings.memory_mode = MemoryMode::LowMemory;
                    state.settings.save();
                }

                let is_2 = state.settings.worker_count == 2;
                if ui.selectable_label(is_2, "2 Workers (Balanced)").clicked() {
                    state.settings.worker_count = 2;
                    state.settings.memory_mode = MemoryMode::Balanced;
                    state.settings.save();
                }
            });

            if state.settings.worker_count > 1 {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("\u{26A0} More workers increase RAM usage.")
                        .font(Theme::mono_font())
                        .color(Theme::WARNING),
                );
            }
        });

        ui.add_space(20.0);

        // Bottom Action Buttons
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(14.0, 0.0);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space((ui.available_width() - 340.0).max(0.0) / 2.0);

                if secondary_button(ui, "Back").clicked() {
                    state.current_screen = AppScreen::Start;
                }

                if primary_button(ui, "\u{25B6} Start Conversion").clicked() {
                    state.start_processing();
                }
            });
        });
    });
}
