use egui::{RichText, Rounding, Stroke, Ui};
use rfd::FileDialog;
use crate::app::{AppScreen, AppState};
use crate::models::MemoryMode;
use crate::ui::components::{primary_button, secondary_button};
use crate::ui::theme::Theme;

pub fn render_settings_view(ui: &mut Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(10.0);

        ui.label(
            RichText::new("Application Settings")
                .font(Theme::heading_font())
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );

        ui.add_space(14.0);

        let settings_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(28.0, 18.0));

        settings_frame.show(ui, |ui| {
            ui.set_width(560.0);
            ui.vertical(|ui| {
                // Section: Performance & Memory
                ui.label(
                    RichText::new("Memory & Performance")
                        .font(Theme::subhead_font())
                        .color(Theme::ACCENT_GOLD)
                        .strong(),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Operating Mode:").font(Theme::body_font()).color(Theme::TEXT_SECONDARY));

                    let is_low = state.settings.memory_mode == MemoryMode::LowMemory;
                    if ui.selectable_label(is_low, "LOW MEMORY (1 Worker, 20 Thumbs)").clicked() {
                        state.settings.memory_mode = MemoryMode::LowMemory;
                        state.settings.worker_count = 1;
                        state.thumbnail_cache.set_capacity(20);
                        state.settings.save();
                    }

                    let is_bal = state.settings.memory_mode == MemoryMode::Balanced;
                    if ui.selectable_label(is_bal, "BALANCED (2 Workers, 40 Thumbs)").clicked() {
                        state.settings.memory_mode = MemoryMode::Balanced;
                        state.settings.worker_count = 2;
                        state.thumbnail_cache.set_capacity(40);
                        state.settings.save();
                    }
                });

                ui.add_space(10.0);

                // Section: Output Preferences
                ui.label(
                    RichText::new("Extraction Preferences")
                        .font(Theme::subhead_font())
                        .color(Theme::ACCENT_GOLD)
                        .strong(),
                );
                ui.add_space(4.0);

                ui.checkbox(&mut state.settings.preserve_filenames, "Preserve original file names");
                ui.checkbox(&mut state.settings.preserve_folder_structure, "Preserve folder subdirectories");
                ui.checkbox(&mut state.settings.overwrite_existing, "Overwrite existing files (default: append collision index)");
                ui.checkbox(&mut state.settings.generate_thumbnails, "Allow on-demand thumbnail generation");

                ui.add_space(10.0);

                // Section: Default Output Folder
                ui.label(
                    RichText::new("Default Output Directory")
                        .font(Theme::subhead_font())
                        .color(Theme::ACCENT_GOLD)
                        .strong(),
                );
                ui.add_space(4.0);

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

                    if secondary_button(ui, "Browse").clicked() {
                        if let Some(folder) = FileDialog::new().pick_folder() {
                            state.settings.output_directory = folder;
                            state.settings.save();
                        }
                    }
                });

                ui.add_space(10.0);

                // Cache management
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Clear Thumbnail Cache").font(Theme::body_font())).clicked() {
                        state.thumbnail_cache.clear();
                        state.show_notification("Thumbnail cache cleared.");
                    }
                });
            });
        });

        ui.add_space(16.0);

        if primary_button(ui, "Save & Close").clicked() {
            state.settings.save();
            state.current_screen = AppScreen::Start;
        }
    });
}
