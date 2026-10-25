#![cfg_attr(not(test), warn(clippy::too_many_lines))]
// 发布版在 Windows 上不弹出黑色控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assist;
mod builder;
mod chrome;
mod commands;
mod dialogs;
mod examples;
mod export;
mod fonts;
mod icons;
mod markdown;
mod mdi;
mod plot;
mod ribbon;
mod sheet;
#[cfg(test)]
mod test_ui;
mod theme;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SciCanvas")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([800.0, 500.0])
            // 不用系统标题栏：顶部栏兼作标题栏，窗口按钮画在右上角（见 chrome.rs）
            .with_decorations(false),
        ..Default::default()
    };
    eframe::run_native("SciCanvas", options, Box::new(|cc| Ok(Box::new(app::SciApp::new(cc)))))
}
