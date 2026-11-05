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
