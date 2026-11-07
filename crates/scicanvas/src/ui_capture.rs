#![cfg(test)]
//! Explicit offscreen captures of the real egui frame using the application's existing wgpu renderer.
use super::*;
use eframe::{egui_wgpu, wgpu};
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

fn block_on<T>(future: impl Future<Output = T>) -> T {
    struct ThreadWake(std::thread::Thread);
    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

struct Capture {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: egui_wgpu::Renderer,
}
impl Capture {
    fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).expect("offscreen adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).expect("offscreen device");
        let renderer = egui_wgpu::Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None, 1, false);
        Self {
            device,
            queue,
            renderer,
        }
    }
    fn frame(&mut self, ctx: &egui::Context, output: egui::FullOutput, path: &std::path::Path) {
        for (id, delta) in &output.textures_delta.set {
            self.renderer.update_texture(&self.device, &self.queue, *id, delta);
        }
        let jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [1280, 820],
            pixels_per_point: output.pixels_per_point,
        };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui review"),
            size: wgpu::Extent3d {
                width: 1280,
                height: 820,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let commands = self
            .renderer
            .update_buffers(&self.device, &self.queue, &mut encoder, &jobs, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui review"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui pixels"),
            size: 1280 * 820 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(1280 * 4),
                    rows_per_image: Some(820),
                },
            },
            wgpu::Extent3d {
                width: 1280,
                height: 820,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(commands.into_iter().chain([encoder.finish()]));
        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| sender.send(result).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();
        let bytes = slice.get_mapped_range().to_vec();
        buffer.unmap();
        resvg::tiny_skia::Pixmap::from_vec(bytes, resvg::tiny_skia::IntSize::from_wh(1280, 820).unwrap())
            .unwrap()
            .save_png(path)
            .unwrap();
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
    fn upload(&mut self, output: &egui::FullOutput) {
        for (id, delta) in &output.textures_delta.set {
            self.renderer.update_texture(&self.device, &self.queue, *id, delta);
        }
    }
}

#[test]
#[ignore = "Explicit offscreen UI screenshot collection, no desktop control"]
fn capture_u0_dialogs() {
    let dir = std::env::var("SCICANVAS_UI_CAPTURE_DIR").expect("capture directory");
    let directory = std::path::Path::new(&dir);
    std::fs::create_dir_all(directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for (name, command) in [
            ("peaks", "signal.peaks"),
            ("fft", "signal.fft"),
            ("frequency", "stats.frequency"),
            ("global-fit", "fit.global"),
            ("calibration", "fit.calibration"),
            ("column-math", "math.colmath"),
        ] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            crate::icons::install_loaders(&ctx);
            ctx.set_theme(theme);
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(1);
            app.selection.insert(
                1,
                if name == "calibration" {
                    vec!["A".into(), "B".into(), "C".into()]
                } else {
                    vec!["B".into(), "C".into()]
                },
            );
            app.run_command(command);
            assert!(app.dialog.is_some(), "{command}: {}", app.status);
            for frame in 0..8 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 820.))),
                        time: Some(frame as f64 * 0.02),
                        ..Default::default()
                    },
                    |ctx| app.ui_frame(ctx),
                );
                if frame == 7 {
                    capture.frame(&ctx, output, &directory.join(format!("{name}-{theme:?}.png")));
                } else {
                    capture.upload(&output);
                }
            }
        }
    }
}

impl Capture {
    fn app_frame(&mut self, ctx: &egui::Context, app: &mut SciApp, events: Vec<egui::Event>) -> egui::FullOutput {
        let time = ctx.input(|i| i.time) + 0.02;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 820.))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| app.ui_frame(ctx),
        );
        self.upload(&output);
        output
    }
    fn app_capture(&mut self, ctx: &egui::Context, app: &mut SciApp, path: &std::path::Path) {
        for _ in 0..7 {
            self.app_frame(ctx, app, vec![]);
        }
        let mut output = self.app_frame(ctx, app, vec![]);
        output.textures_delta.set.clear();
        self.frame(ctx, output, path);
    }
}

#[test]
#[ignore = "Explicit before/after ribbon screenshots"]
fn capture_u1_tabs() {
    let dir = std::env::var("SCICANVAS_UI_CAPTURE_DIR").expect("capture directory");
    let directory = std::path::Path::new(&dir);
    std::fs::create_dir_all(directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for tab in crate::ribbon::TABS {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            crate::icons::install_loaders(&ctx);
            ctx.set_theme(theme);
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(if tab.ctx == Some(crate::ribbon::Ctx::Book) {
                1
            } else {
                2
            });
            app.ui.tab = tab.id;
            capture.app_capture(&ctx, &mut app, &directory.join(format!("tab-{}-{theme:?}.png", tab.id)));
            if tab.id == "plot" {
                capture.app_frame(&ctx, &mut app, vec![]);
                let at = ctx
                    .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("ribbon-gallery-menu-rect")))
                    .expect("gallery menu")
                    .center();
                capture.app_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, true)],
                );
                capture.app_frame(&ctx, &mut app, vec![crate::test_ui::button(at, false)]);
                capture.app_capture(&ctx, &mut app, &directory.join(format!("gallery-{theme:?}.png")));
            }
        }
    }
}

#[test]
#[ignore = "Explicit before/after parameter-dialog screenshots"]
fn capture_u2_dialogs() {
    let dir = std::env::var("SCICANVAS_UI_CAPTURE_DIR").expect("capture directory");
    let directory = std::path::Path::new(&dir);
    std::fs::create_dir_all(directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for (name, command) in [
            ("baseline", "signal.baseline"),
            ("calibration", "fit.calibration"),
            ("interpolate", "math.interp"),
            ("peaks", "signal.peaks"),
            ("global-fit", "fit.global"),
            ("peak-fit", "fit.peaks"),
            ("peak-preview", "fit.peaks"),
            ("t-test", "stats.ttest"),
            ("invalid-baseline", "signal.baseline"),
        ] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            crate::icons::install_loaders(&ctx);
            ctx.set_theme(theme);
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(1);
            app.selection.insert(
                1,
                match name {
                    "calibration" => vec!["A".into(), "B".into(), "C".into()],
                    "peak-fit" | "peak-preview" => vec!["B".into()],
                    _ => vec!["B".into(), "C".into()],
                },
            );
            if name == "peak-preview" {
                let book = app.project.book_mut(1).unwrap();
                let x = book.column("A").unwrap().values();
                book.columns.iter_mut().find(|c| c.key == "B").unwrap().data = x
                    .iter()
                    .map(|x| {
                        (0.3 + 6. * (-0.5 * ((x - 6.) / 1.5).powi(2)).exp()
                            + 4. * (-0.5 * ((x - 14.) / 2.).powi(2)).exp())
                        .to_string()
                    })
                    .collect();
            }
            app.run_command(command);
            assert!(app.dialog.is_some(), "{command}: {}", app.status);
            if name == "peak-preview" {
                if let Some(Dialog::PeakFit(edit)) = &mut app.dialog {
                    edit.preview();
                }
            }
            if name == "invalid-baseline" {
                if let Some(Dialog::Baseline { advanced, anchors, .. }) = &mut app.dialog {
                    *advanced = 1;
                    *anchors = "bad".into();
                }
            }
            capture.app_capture(&ctx, &mut app, &directory.join(format!("{name}-{theme:?}.png")));
        }
    }
}

#[test]
#[ignore = "Explicit before/after property-panel screenshots"]
fn capture_u3_properties() {
    let dir = std::env::var("SCICANVAS_UI_CAPTURE_DIR").expect("capture directory");
    let directory = std::path::Path::new(&dir);
    std::fs::create_dir_all(directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for color in [false, true] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            crate::icons::install_loaders(&ctx);
            ctx.set_theme(theme);
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(2);
            app.props_for = Some(2);
            if color {
                let layer = app.project.graph_mut(2).unwrap().first_layer_mut();
                layer.plot_type = scicanvas_core::PlotType::ColorScatter;
                layer.edit_styles(|styles| {
                    styles.entry("B".into()).or_default().color_column = Some("C".into());
                });
                layer.color_map = scicanvas_core::colormap::ColorMap::Viridis;
            }
            if color {
                for _ in 0..5 {
                    capture.app_frame(&ctx, &mut app, vec![]);
                }
                let out = capture.app_frame(&ctx, &mut app, vec![]);
                if let Some(rect) = crate::test_ui::text_rect(&out, "坐标轴") {
                    let at = rect.center();
                    capture.app_frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, true)],
                    );
                    capture.app_frame(&ctx, &mut app, vec![crate::test_ui::button(at, false)]);
                }
            }
            let name = if color { "color-properties" } else { "properties" };
            capture.app_capture(&ctx, &mut app, &directory.join(format!("{name}-{theme:?}.png")));
        }
    }
}

#[test]
#[ignore = "Explicit before/after status screenshots"]
fn capture_u4_status() {
    let dir = std::env::var("SCICANVAS_UI_CAPTURE_DIR").expect("capture directory");
    let directory = std::path::Path::new(&dir);
    std::fs::create_dir_all(directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for name in ["error", "success", "history"] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            crate::icons::install_loaders(&ctx);
            ctx.set_theme(theme);
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(2);
            app.status_info("就绪".into());
            app.status_warn("已跳过 3 行缺失值".into());
            if name == "error" {
                app.status_err("拟合失败：有效数据点不足".into());
            } else {
                app.status_ok("已生成拟合曲线和报告".into());
            }
            if name == "history" {
                app.show_status_messages = true;
            }
            capture.app_capture(&ctx, &mut app, &directory.join(format!("{name}-{theme:?}.png")));
        }
    }
}

#[test]
#[ignore = "Explicit menu screenshots"]
fn capture_u6_menus() {
    let directory = std::path::PathBuf::from(std::env::var("SCICANVAS_UI_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for menu in ["cell", "column", "graph", "project"] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            crate::icons::install_loaders(&ctx);
            ctx.set_theme(theme);
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(if menu == "graph" { 2 } else { 1 });
            app.selection.insert(1, vec!["B".into()]);
            for _ in 0..8 {
                capture.app_frame(&ctx, &mut app, vec![]);
            }
            let out = capture.app_frame(&ctx, &mut app, vec![]);
            let at = match menu {
                "cell" => ctx
                    .read_response(egui::Id::new(("cell", 1_u64, 1_usize, 0_usize)))
                    .unwrap()
                    .rect
                    .center(),
                "column" => crate::test_ui::text_rect(&out, "B(Y)").unwrap().center(),
                "project" => out
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.job.text == "Book1" && t.pos.x < 180. => {
                            Some(t.pos + egui::vec2(8., 6.))
                        }
                        _ => None,
                    })
                    .unwrap(),
                _ => egui::pos2(1050., 400.),
            };
            capture.app_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(at)]);
            for pressed in [true, false] {
                capture.app_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            capture.app_capture(&ctx, &mut app, &directory.join(format!("u6-{menu}-{theme:?}.png")));
        }
    }
}

#[test]
#[ignore = "Explicit round18 dialog and properties screenshots"]
fn capture_u7_details() {
    let directory = std::path::PathBuf::from(std::env::var("SCICANVAS_UI_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for name in [
            "props-page",
            "props-axes",
            "builtin-fit",
            "custom-fit",
            "global-fit",
            "peak-fit",
            "result",
        ] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            crate::icons::install_loaders(&ctx);
            ctx.set_theme(theme);
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(1);
            app.selection.insert(
                1,
                if name == "global-fit" || name == "result" {
                    vec!["B".into(), "C".into()]
                } else {
                    vec!["B".into()]
                },
            );
            match name {
                "props-page" | "props-axes" => {
                    app.active = Some(2);
                    app.props_for = Some(2);
                }
                "builtin-fit" => app.run_command("fit.nonlinear"),
                "custom-fit" => app.open_custom_nlfit_dialog(),
                "global-fit" => app.run_command("fit.global"),
                "peak-fit" => app.run_command("fit.peaks"),
                _ => app.run_command("stats.anova"),
            }
            for _ in 0..8 {
                capture.app_frame(&ctx, &mut app, vec![]);
            }
            let labels = match name {
                "props-page" => vec!["数据", "坐标轴", "页面与图层"],
                "props-axes" => vec!["数据"],
                "peak-fit" => vec!["参数限制"],
                _ => vec![],
            };
            for label in labels {
                let out = capture.app_frame(&ctx, &mut app, vec![]);
                let at = crate::test_ui::text_rect(&out, label)
                    .unwrap_or_else(|| panic!("{name}: {label}"))
                    .center();
                capture.app_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, true)],
                );
                capture.app_frame(&ctx, &mut app, vec![crate::test_ui::button(at, false)]);
            }
            capture.app_capture(&ctx, &mut app, &directory.join(format!("u7-{name}-{theme:?}.png")));
        }
    }
}

#[test]
#[ignore = "Explicit gallery alignment screenshots"]
fn capture_gallery_alignment() {
    let directory = std::path::PathBuf::from(std::env::var("SCICANVAS_UI_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    let mut capture = Capture::new();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        crate::icons::install_loaders(&ctx);
        ctx.set_theme(theme);
        let mut app = SciApp::initial(crate::theme::palette(theme));
        app.active = Some(1);
        app.ui.tab = "plot";
        for _ in 0..8 {
            capture.app_frame(&ctx, &mut app, vec![]);
        }
        let at = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("ribbon-gallery-menu-rect")))
            .unwrap()
            .center();
        capture.app_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, true)],
        );
        capture.app_frame(&ctx, &mut app, vec![crate::test_ui::button(at, false)]);
        for (section, delta) in [("top", 0.), ("middle", -380.), ("bottom", -800.)] {
            if delta != 0. {
                capture.app_frame(
                    &ctx,
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(at + egui::vec2(80., 160.)),
                        egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0., delta),
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            capture.app_capture(
                &ctx,
                &mut app,
                &directory.join(format!("gallery-{section}-{theme:?}.png")),
            );
        }
    }
}

#[test]
#[ignore = "Explicit default/custom axis-style comparison using the real screen renderer"]
fn capture_axis_style_comparison() {
    let path = std::path::PathBuf::from(std::env::var("SCICANVAS_UI_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&path).unwrap();
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let p = scicanvas_core::Project::sample();
    let mut original = p.graph(2).unwrap().clone();
    original.page.width_mm = 89.;
    original.page.height_mm = 89.;
    original.first_layer_mut().title = "默认样式".into();
    let mut custom = original.clone();
    custom.first_layer_mut().axis_style = scicanvas_core::page::AxisStyle {
        axis_width_pt: Some(2.),
        major_len_pt: Some(8.),
        tick_font_pt: Some(12.),
        title_font_pt: Some(16.),
        label_font_pt: Some(12.),
        ..Default::default()
    };
    custom.first_layer_mut().title = "调整后的样式".into();
    let mut capture = Capture::new();
    for pass in 0..3 {
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 820.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none())
                    .show(ctx, |ui| {
                        ui.columns(2, |columns| {
                            for (ui, g) in columns.iter_mut().zip([&original, &custom]) {
                                let (rect, _) = ui
                                    .allocate_exact_size(egui::vec2(ui.available_width(), 600.), egui::Sense::hover());
                                crate::plot::paint(ui, rect, &scicanvas_core::scene::layout_page(g, &p, &[]));
                            }
                        });
                    });
            },
        );
        if pass == 2 {
            capture.frame(&ctx, out, &path.join("comparison.png"));
        } else {
            capture.upload(&out);
        }
    }
}
