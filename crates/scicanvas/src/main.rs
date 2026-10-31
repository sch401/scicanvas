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
    scicanvas_core::project_file::session::install_panic_log();
    let settings = scicanvas_core::project_file::session::load_settings();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("SciCanvas")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([800.0, 500.0])
        .with_decorations(false);
    if let Some([x, y, w, h]) = settings
        .window
        .filter(|values| values.iter().all(|v| v.is_finite()) && values[2] >= 800.0 && values[3] >= 500.0)
    {
        viewport = viewport.with_position([x, y]).with_inner_size([w, h]);
    }
    viewport = viewport.with_maximized(settings.maximized);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    #[cfg(target_os = "windows")]
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: Some(1),
            ..Default::default()
        },
        ..options
    };
    eframe::run_native("SciCanvas", options, Box::new(|cc| Ok(Box::new(app::SciApp::new(cc)))))
}
