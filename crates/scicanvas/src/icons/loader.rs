//! SVG 图标加载与尺寸缓存，使用导出共用的 resvg。
// 尺寸提示与缓存协议参考 egui_extras 0.29.1 的 SVG 加载器实现。
// 上游版权声明依 MIT 许可保留原文：Copyright (c) 2018-2021 Emil Ernerfeldt <emil.ernerfeldt@gmail.com>
// MIT 许可全文随包保存在 licenses/third-party/egui_extras-0.29.1/upstream/LICENSE-MIT。
use eframe::egui::{
    self,
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage,
};
use resvg::{
    tiny_skia::{IntSize, Pixmap, Transform},
    usvg,
};
use std::{collections::HashMap, sync::Arc};

type Entry = Result<Arc<ColorImage>, String>;
#[derive(Default)]
struct SvgLoader {
    cache: Mutex<HashMap<(String, SizeHint), Entry>>,
}
const ID: &str = "scicanvas::svg-loader";

pub fn install_loaders(ctx: &egui::Context) {
    if !ctx.is_loader_installed(ID) {
        ctx.add_image_loader(Arc::new(SvgLoader::default()));
    }
}

impl ImageLoader for SvgLoader {
    fn id(&self) -> &str {
        ID
    }
    fn load(&self, ctx: &egui::Context, uri: &str, size: SizeHint) -> ImageLoadResult {
        if std::path::Path::new(uri).extension().and_then(|e| e.to_str()) != Some("svg") {
            return Err(LoadError::NotSupported);
        }
        let mut cache = self.cache.lock();
        let key = (uri.to_owned(), size);
        let result = if let Some(result) = cache.get(&key) {
            result.clone()
        } else {
            let bytes = match ctx.try_load_bytes(uri)? {
                BytesPoll::Pending { size } => return Ok(ImagePoll::Pending { size }),
                BytesPoll::Ready { bytes, .. } => bytes,
            };
            let result = load_svg(&bytes, size).map(Arc::new);
            cache.insert(key, result.clone());
            result
        };
        result
            .map(|image| ImagePoll::Ready { image })
            .map_err(LoadError::Loading)
    }
    fn forget(&self, uri: &str) {
        self.cache.lock().retain(|(u, _), _| u != uri);
    }
    fn forget_all(&self) {
        self.cache.lock().clear();
    }
    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|entry| match entry {
                Ok(image) => image.pixels.len() * std::mem::size_of::<egui::Color32>(),
                Err(e) => e.len(),
            })
            .sum()
    }
}

pub(super) fn load_svg(bytes: &[u8], hint: SizeHint) -> Result<ColorImage, String> {
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let native = tree.size();
    let size = native.to_int_size();
    let size = match hint {
        SizeHint::Size(w, h) => size.scale_to(IntSize::from_wh(w, h).ok_or("无效图标尺寸")?),
        SizeHint::Width(w) => size.scale_to_width(w).ok_or("无效图标宽度")?,
        SizeHint::Height(h) => size.scale_to_height(h).ok_or("无效图标高度")?,
        SizeHint::Scale(z) => size.scale_by(z.into_inner()).ok_or("无效图标缩放")?,
    };
    let (w, h) = (size.width(), size.height());
    let mut pixmap = Pixmap::new(w, h).ok_or("无法分配图标位图")?;
    let scale = (w as f32 / native.width()).min(h as f32 / native.height());
    let dx = (w as f32 - native.width() * scale) / 2.0;
    let dy = (h as f32 - native.height() * scale) / 2.0;
    resvg::render(
        &tree,
        Transform::from_row(scale, 0.0, 0.0, scale, dx, dy),
        &mut pixmap.as_mut(),
    );
    // 沿用既有图标的颜色转换规则，避免改变半透明边缘。
    Ok(ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        pixmap.data(),
    ))
}
