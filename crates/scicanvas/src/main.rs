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

/// 原生显示器工作区保留全局原点，按当前窗口的 DPI 转换到 egui 坐标。
mod monitors {
    use eframe::egui::{self, Rect};
    pub fn visible_window(rect: Rect, areas: &[Rect]) -> Option<Rect> {
        let title = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 32.0_f32.min(rect.height())));
        if areas.iter().any(|area| {
            let visible = area.intersect(title);
            visible.width() >= 100.0_f32.min(title.width()) && visible.height() >= 16.0_f32.min(title.height())
        }) {
            return None;
        }
        let area = areas.iter().min_by(|a, b| {
            a.center()
                .distance_sq(rect.center())
                .total_cmp(&b.center().distance_sq(rect.center()))
        })?;
        let margin = egui::vec2(40., 40.).min(area.size() / 4.);
        Some(Rect::from_min_size(
            area.min + margin,
            rect.size().min(area.size() - 2. * margin),
        ))
    }
    pub fn restore_size(outer: Rect, inner: Rect, target: Rect) -> Option<egui::Vec2> {
        if outer.width() <= target.width() && outer.height() <= target.height() {
            return None;
        }
        let chrome = (outer.size() - inner.size()).max(egui::Vec2::ZERO);
        Some((target.size() - chrome).max(egui::vec2(1., 1.)))
    }
    pub fn work_areas(scale: f32) -> Vec<Rect> {
        #[cfg(windows)]
        {
            use std::ffi::c_void;
            #[repr(C)]
            #[derive(Default)]
            struct NativeRect {
                left: i32,
                top: i32,
                right: i32,
                bottom: i32,
            }
            #[repr(C)]
            #[derive(Default)]
            struct Info {
                size: u32,
                monitor: NativeRect,
                work: NativeRect,
                flags: u32,
            }
            #[link(name = "user32")]
            unsafe extern "system" {
                fn EnumDisplayMonitors(
                    hdc: *mut c_void,
                    clip: *const NativeRect,
                    callback: Option<
                        unsafe extern "system" fn(*mut c_void, *mut c_void, *mut NativeRect, isize) -> i32,
                    >,
                    data: isize,
                ) -> i32;
                fn GetMonitorInfoW(monitor: *mut c_void, info: *mut Info) -> i32;
            }
            unsafe extern "system" fn collect(
                monitor: *mut c_void,
                _hdc: *mut c_void,
                _clip: *mut NativeRect,
                data: isize,
            ) -> i32 {
                let mut info = Info {
                    size: std::mem::size_of::<Info>() as u32,
                    ..Default::default()
                };
                // EnumDisplayMonitors 同步调用；data 指向本次枚举期间独占的 Vec。
                if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
                    let output = unsafe { &mut *(data as *mut Vec<Rect>) };
                    output.push(Rect::from_min_max(
                        egui::pos2(info.work.left as f32, info.work.top as f32),
                        egui::pos2(info.work.right as f32, info.work.bottom as f32),
                    ));
                }
                1
            }
            let mut result: Vec<Rect> = Vec::new();
            // 回调和 MONITORINFO 布局与 Win32 API 一致。
            unsafe {
                EnumDisplayMonitors(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    Some(collect),
                    &mut result as *mut Vec<Rect> as isize,
                );
            }
            result
                .into_iter()
                .map(|r| Rect::from_min_max((r.min.to_vec2() / scale).to_pos2(), (r.max.to_vec2() / scale).to_pos2()))
                .collect()
        }
        #[cfg(not(windows))]
        {
            let _ = scale;
            Vec::new()
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn restored_inner_size_accounts_for_window_chrome() {
            let outer = Rect::from_min_size(egui::pos2(8000., 8000.), egui::vec2(1280., 800.));
            let inner = Rect::from_min_size(outer.min, egui::vec2(1264., 761.));
            let moved = Rect::from_min_size(egui::pos2(40., 40.), outer.size());
            assert_eq!(restore_size(outer, inner, moved), None);
            let target = Rect::from_min_size(moved.min, egui::vec2(900., 600.));
            let size = restore_size(outer, inner, target).unwrap();
            assert_eq!(size, egui::vec2(884., 561.));
            assert_eq!(size + outer.size() - inner.size(), target.size());
        }
        #[test]
        fn monitor_origins_and_title_visibility_control_recovery() {
            let areas = [
                Rect::from_min_max(egui::pos2(-1920., 0.), egui::pos2(0., 1080.)),
                Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1920., 1080.)),
            ];
            assert!(visible_window(
                Rect::from_min_size(egui::pos2(-1800., 40.), egui::vec2(900., 600.)),
                &areas
            )
            .is_none());
            assert!(visible_window(
                Rect::from_min_size(egui::pos2(1850., 40.), egui::vec2(900., 600.)),
                &areas
            )
            .is_some());
            assert!(visible_window(
                Rect::from_min_size(egui::pos2(1750., 40.), egui::vec2(900., 600.)),
                &areas
            )
            .is_none());
            assert!(visible_window(
                Rect::from_min_size(egui::pos2(20., -200.), egui::vec2(900., 600.)),
                &areas
            )
            .is_some());
            let target = visible_window(
                Rect::from_min_size(egui::pos2(8000., 8000.), egui::vec2(4000., 3000.)),
                &areas,
            )
            .unwrap();
            assert!(areas[1].contains_rect(target));
        }
    }
}
