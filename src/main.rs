#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use blackwater::ui::BlackwaterApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([740.0, 600.0])
            .with_min_inner_size([640.0, 520.0])
            .with_title("Blackwater \u{2014} RDR2 Photo Extractor")
            .with_active(true)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Blackwater",
        native_options,
        Box::new(|cc| Ok(Box::new(BlackwaterApp::new(cc)))),
    )
}
