pub mod components;
pub mod theme;
pub mod views;

use std::path::PathBuf;
use std::time::Duration;
use egui::{CentralPanel, Frame, Margin, RichText, Rounding, Stroke, TopBottomPanel};
use crate::app::{AppScreen, AppState};
use crate::ui::components::{render_footer, render_header};
use crate::ui::theme::Theme;

pub struct BlackwaterApp {
    state: AppState,
}

impl BlackwaterApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Theme::apply(&cc.egui_ctx);
        Self {
            state: AppState::new(),
        }
    }
}

impl eframe::App for BlackwaterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll background events
        self.state.poll_scan_events();
        self.state.poll_worker_events();
        self.state.poll_memory_usage();

        // Handle Drag & Drop
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            let paths: Vec<PathBuf> = dropped_files.into_iter().filter_map(|f| f.path).collect();
            if !paths.is_empty() && self.state.current_screen != AppScreen::Processing {
                self.state.start_scanning(paths);
            }
        }

        // Request repaint while actively working
        if self.state.is_scanning || self.state.batch_stats.is_running {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        // Top Header
        TopBottomPanel::top("header_panel")
            .frame(Frame::none().fill(Theme::BG_PRIMARY).inner_margin(Margin::symmetric(20.0, 14.0)))
            .show(ctx, |ui| {
                render_header(ui, &mut self.state);
            });

        // Bottom Footer
        TopBottomPanel::bottom("footer_panel")
            .frame(Frame::none().fill(Theme::BG_PRIMARY).inner_margin(Margin::symmetric(20.0, 10.0)))
            .show(ctx, |ui| {
                render_footer(ui, &mut self.state);
            });

        // Central Content
        CentralPanel::default()
            .frame(Frame::none().fill(Theme::BG_PRIMARY).inner_margin(Margin::symmetric(20.0, 10.0)))
            .show(ctx, |ui| {
                match self.state.current_screen {
                    AppScreen::Start => views::render_start_view(ui, &mut self.state),
                    AppScreen::Scanning => views::render_scan_view(ui, &mut self.state),
                    AppScreen::FormatSelect => views::render_format_view(ui, &mut self.state),
                    AppScreen::Processing => views::render_processing_view(ui, &mut self.state),
                    AppScreen::Result => views::render_result_view(ui, &mut self.state),
                    AppScreen::Settings => views::render_settings_view(ui, &mut self.state),
                }

                // Render floating notification toast if active
                if let Some((msg, created_at)) = &self.state.status_notification {
                    if created_at.elapsed().as_secs() < 4 {
                        egui::Area::new(egui::Id::new("toast_notification"))
                            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
                            .show(ctx, |ui| {
                                egui::Frame::none()
                                    .fill(Theme::BG_SECONDARY)
                                    .stroke(Stroke::new(1.0_f32, Theme::ACCENT_GOLD))
                                    .rounding(Rounding::same(6.0))
                                    .inner_margin(Margin::symmetric(18.0, 10.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(msg)
                                                .font(Theme::body_font())
                                                .color(Theme::TEXT_PRIMARY),
                                        );
                                    });
                            });
                    }
                }
            });
    }
}
