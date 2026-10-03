use egui::{Align, Layout, ProgressBar, RichText, Rounding, Stroke, Ui, Vec2};
use crate::app::AppState;
use crate::ui::components::{danger_button, secondary_button};
use crate::ui::theme::Theme;

pub fn render_processing_view(ui: &mut Ui, state: &mut AppState) {
    let total = state.batch_stats.total_valid.max(1);
    let completed = state.batch_stats.processed_count;
    let fraction = (completed as f32 / total as f32).clamp(0.0, 1.0);

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);

        ui.label(
            RichText::new("Converting Photos")
                .font(Theme::heading_font())
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );

        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{completed} / {total} photos processed ({:.0}%)", fraction * 100.0))
                .font(Theme::subhead_font())
                .color(Theme::ACCENT_GOLD),
        );

        ui.add_space(16.0);

        // Progress bar
        let progress_bar = ProgressBar::new(fraction)
            .show_percentage()
            .fill(Theme::ACCENT_GOLD)
            .desired_width(540.0)
            .desired_height(14.0);
        ui.add(progress_bar);

        ui.add_space(20.0);

        // Stats card
        let stats_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(28.0, 18.0));

        stats_frame.show(ui, |ui| {
            ui.set_width(540.0);
            ui.vertical(|ui| {
                // Current File
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Current file:").font(Theme::body_font()).color(Theme::TEXT_SECONDARY));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let cur = if state.batch_stats.current_file_name.is_empty() {
                            "Initializing...".to_string()
                        } else {
                            state.batch_stats.current_file_name.clone()
                        };
                        ui.label(RichText::new(cur).font(Theme::mono_font()).color(Theme::TEXT_PRIMARY).strong());
                    });
                });

                ui.add_space(6.0);

                // Successful count
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Successful:").font(Theme::body_font()).color(Theme::SUCCESS));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{}", state.batch_stats.success_count)).font(Theme::mono_font()).color(Theme::SUCCESS).strong());
                    });
                });

                ui.add_space(6.0);

                // Failed count
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Failed:").font(Theme::body_font()).color(if state.batch_stats.failed_count > 0 { Theme::ERROR } else { Theme::TEXT_MUTED }));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{}", state.batch_stats.failed_count)).font(Theme::mono_font()).color(if state.batch_stats.failed_count > 0 { Theme::ERROR } else { Theme::TEXT_MUTED }));
                    });
                });

                ui.add_space(6.0);

                // Concurrency info
                ui.horizontal(|ui| {
                    ui.label(RichText::new("RAM-conscious processing:").font(Theme::body_font()).color(Theme::TEXT_SECONDARY));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} worker (1 image in RAM at a time)", state.settings.worker_count))
                                .font(Theme::mono_font())
                                .color(Theme::ACCENT_GOLD),
                        );
                    });
                });

                // Real Process Memory usage
                if state.current_working_set_mb > 0.0 {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Actual Process RAM:").font(Theme::body_font()).color(Theme::TEXT_SECONDARY));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{:.1} MB (Peak: {:.1} MB)", state.current_working_set_mb, state.peak_working_set_mb))
                                    .font(Theme::mono_font())
                                    .color(Theme::SUCCESS),
                            );
                        });
                    });
                }
            });
        });

        ui.add_space(24.0);

        // Control buttons
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(16.0, 0.0);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space((ui.available_width() - 280.0).max(0.0) / 2.0);

                let pause_label = if state.batch_stats.is_paused {
                    "\u{25B6} Resume"
                } else {
                    "\u{23F8} Pause"
                };

                if secondary_button(ui, pause_label).clicked() {
                    state.toggle_pause();
                }

                if danger_button(ui, "\u{2715} Cancel").clicked() {
                    state.cancel_processing();
                }
            });
        });
    });
}
