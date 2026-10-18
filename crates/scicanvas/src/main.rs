// 发布版在 Windows 上不弹出黑色控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assist;
mod builder;
mod commands;
mod dialogs;
mod fonts;
mod icons;
mod mdi;
mod plot;
mod ribbon;
mod sheet;
mod theme;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SciCanvas")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native("SciCanvas", options, Box::new(|cc| Ok(Box::new(app::SciApp::new(cc)))))
}
