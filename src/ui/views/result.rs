use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use egui::{Align, Layout, RichText, Rounding, ScrollArea, Stroke, Ui};
use rfd::FileDialog;
use crate::app::AppState;
use crate::archive::{read_zip_entry_to_cursor, StreamingZipExporter};
use crate::converter::generate_thumbnail;
use crate::filesystem::open_folder_in_native_file_manager;
use crate::models::{PhotoSource, ProcessStatus};
use crate::ui::components::{primary_button, secondary_button};
use crate::ui::theme::Theme;

pub fn render_result_view(ui: &mut Ui, state: &mut AppState) {
    let is_cancelled = state.batch_stats.is_cancelled;
    let total_processed = state.processed_photos.len();
    let success_count = state.processed_photos.iter().filter(|p| p.status == ProcessStatus::Success).count();
    let failed_count = total_processed - success_count;

    let mut request_thumbnail_load = None;
    let mut request_hide_thumbnail = false;
    let mut request_open_file = None;

    ui.vertical(|ui| {
        // Summary Header Card
        let summary_frame = egui::Frame::none()
            .fill(Theme::BG_SECONDARY)
            .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(24.0, 16.0));

        summary_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let title = if is_cancelled {
                        "Conversion Stopped by User"
                    } else {
                        "Conversion Complete"
                    };
                    ui.label(
                        RichText::new(title)
                            .font(Theme::heading_font())
                            .color(if is_cancelled { Theme::WARNING } else { Theme::SUCCESS })
                            .strong(),
                    );
                    ui.add_space(2.0);

                    let summary_str = format!(
                        "{} files processed  \u{2022}  {} successful  \u{2022}  {} failed",
                        total_processed, success_count, failed_count
                    );
                    ui.label(RichText::new(summary_str).font(Theme::body_font()).color(Theme::TEXT_SECONDARY));

                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Output:").font(Theme::mono_font()).color(Theme::TEXT_MUTED));
                        ui.label(
                            RichText::new(state.settings.output_directory.to_string_lossy().to_string())
                                .font(Theme::mono_font())
                                .color(Theme::ACCENT_GOLD),
                        );
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if secondary_button(ui, "\u{21BB} Convert More").clicked() {
                        state.reset_to_start();
                        return;
                    }

                    if primary_button(ui, "\u{1F4C2} Open Output Folder").clicked() {
                        open_folder_in_native_file_manager(&state.settings.output_directory);
                    }

                    if secondary_button(ui, "\u{1F5C3} Export ZIP").clicked() {
                        export_results_to_zip(state);
                    }

                    if failed_count > 0 {
                        if secondary_button(ui, "\u{26A0} Retry Failed").clicked() {
                            state.retry_failed_items();
                        }
                    }
                });
            });
        });

        ui.add_space(10.0);

        // Result List Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Processed Items ({total_processed})"))
                    .font(Theme::subhead_font())
                    .color(Theme::TEXT_PRIMARY)
                    .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("Previews generated on demand (RAM-safe)")
                        .font(Theme::mono_font())
                        .color(Theme::TEXT_MUTED),
                );
            });
        });

        ui.add_space(4.0);

        // Compact Scrollable Result List
        ScrollArea::vertical()
            .max_height(340.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for photo in &state.processed_photos {
                    let item_frame = egui::Frame::none()
                        .fill(Theme::BG_SECONDARY)
                        .stroke(Stroke::new(1.0_f32, Theme::BORDER_SUBTLE))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 8.0));

                    item_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Status Icon
                            match &photo.status {
                                ProcessStatus::Success => {
                                    ui.label(RichText::new("\u{2714}").font(Theme::subhead_font()).color(Theme::SUCCESS));
                                }
                                ProcessStatus::Failed(_) => {
                                    ui.label(RichText::new("\u{2716}").font(Theme::subhead_font()).color(Theme::ERROR));
                                }
                                _ => {
                                    ui.label(RichText::new("\u{2022}").font(Theme::subhead_font()).color(Theme::TEXT_MUTED));
                                }
                            }

                            // Name & Source
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(photo.source.display_name())
                                        .font(Theme::body_font())
                                        .color(Theme::TEXT_PRIMARY)
                                        .strong(),
                                );
                                if let Some(meta) = &photo.metadata {
                                    let details = format!(
                                        "{}  \u{2022}  {}  \u{2022}  {:.1} MB",
                                        meta.resolution_str(),
                                        state.settings.output_format.display_name(),
                                        meta.output_size_bytes as f64 / (1024.0 * 1024.0)
                                    );
                                    ui.label(RichText::new(details).font(Theme::mono_font()).color(Theme::TEXT_MUTED));
                                }
                            });

                            // Action buttons right aligned
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if let Some(out_path) = &photo.output_path {
                                    if ui.button(RichText::new("Open").font(Theme::body_font())).clicked() {
                                        request_open_file = Some(out_path.clone());
                                    }

                                    let key = photo.source.full_path_str();
                                    let has_cached_thumb = state.thumbnail_cache.get(&key).is_some();
                                    let preview_label = if has_cached_thumb { "Hide Preview" } else { "Preview" };

                                    if ui.button(RichText::new(preview_label).font(Theme::body_font())).clicked() {
                                        if has_cached_thumb {
                                            request_hide_thumbnail = true;
                                        } else {
                                            request_thumbnail_load = Some((photo.source.clone(), photo.metadata.clone()));
                                        }
                                    }
                                } else if let ProcessStatus::Failed(err) = &photo.status {
                                    ui.label(RichText::new(err).font(Theme::mono_font()).color(Theme::ERROR));
                                }
                            });
                        });

                        // Render on-demand thumbnail preview if cached
                        let key = photo.source.full_path_str();
                        if let Some(tex) = state.thumbnail_cache.get(&key) {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.image((tex.id(), egui::vec2(160.0, 90.0)));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Bounded thumbnail cache (max 320px)").font(Theme::mono_font()).color(Theme::TEXT_MUTED));
                                    if let Some(meta) = &photo.metadata {
                                        ui.label(RichText::new(format!("Source offset: 0x{:X}", meta.jpeg_offset)).font(Theme::mono_font()).color(Theme::TEXT_SECONDARY));
                                    }
                                });
                            });
                        }
                    });
                    ui.add_space(4.0);
                }
            });
    });

    // Handle deferred actions
    if let Some(path) = request_open_file {
        open_folder_in_native_file_manager(&path);
    }

    if request_hide_thumbnail {
        state.thumbnail_cache.clear();
        state.active_preview_source = None;
    }

    if let Some((source, meta)) = request_thumbnail_load {
        load_thumbnail_for_photo(ui.ctx(), state, &source, meta.as_ref());
        state.active_preview_source = Some(source);
    }
}

/// Generates a bounded small thumbnail on demand and inserts into the LRU cache.
fn load_thumbnail_for_photo(
    ctx: &egui::Context,
    state: &mut AppState,
    source: &PhotoSource,
    meta: Option<&crate::models::PhotoMetadata>,
) {
    let metadata = match meta {
        Some(m) => m,
        None => return,
    };

    let key = source.full_path_str();

    let rgba_res = match source {
        PhotoSource::LocalFile(path) => {
            if let Ok(file) = File::open(path) {
                let mut reader = BufReader::with_capacity(64 * 1024, file);
                generate_thumbnail(&mut reader, metadata, 320)
            } else {
                return;
            }
        }
        PhotoSource::ZipEntry { zip_path, entry_index, .. } => {
            if let Ok(mut cursor) = read_zip_entry_to_cursor(zip_path, *entry_index) {
                generate_thumbnail(&mut cursor, metadata, 320)
            } else {
                return;
            }
        }
    };

    if let Ok(rgba_img) = rgba_res {
        let (w, h) = (rgba_img.width() as usize, rgba_img.height() as usize);
        let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba_img.as_raw());
        let texture = ctx.load_texture(&key, color_image, egui::TextureOptions::LINEAR);
        state.thumbnail_cache.insert(key, texture);
    }
}

/// Streams all successfully converted files directly into a new ZIP archive.
fn export_results_to_zip(state: &mut AppState) {
    let successful: Vec<PathBuf> = state
        .processed_photos
        .iter()
        .filter_map(|p| p.output_path.clone())
        .collect();

    if successful.is_empty() {
        state.show_notification("No converted photos available to export.");
        return;
    }

    if let Some(zip_dest) = FileDialog::new()
        .set_file_name("Blackwater_Photos.zip")
        .add_filter("ZIP Archive", &["zip"])
        .save_file()
    {
        match StreamingZipExporter::new(&zip_dest) {
            Ok(mut exporter) => {
                let mut count = 0;
                for file_path in successful {
                    if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                        if exporter.add_file_from_disk(file_name, &file_path).is_ok() {
                            count += 1;
                        }
                    }
                }
                if let Ok(size) = exporter.finish() {
                    let mb = size as f64 / (1024.0 * 1024.0);
                    state.show_notification(format!("Exported {count} photos to ZIP ({mb:.1} MB)."));
                }
            }
            Err(e) => {
                state.show_notification(format!("Failed to create ZIP: {e:?}"));
            }
        }
    }
}
