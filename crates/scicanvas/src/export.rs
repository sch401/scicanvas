//! 图形导出为 PNG、复制到剪贴板：把 SVG 导出结果交给 resvg 栅格化，
//! 屏幕、SVG 和位图共用同一份版面。PNG 写入物理分辨率（pHYs），
//! Word / PPT 插入时按页面毫米尺寸显示。页面始终是白底，与 SVG 一致。

use std::sync::OnceLock;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb, TreeParsing, TreeTextToPath};

/// 复制到剪贴板时使用的分辨率。
pub const CLIPBOARD_DPI: u32 = 300;
/// 单边像素上限，防止误输入极大的 DPI 时耗尽内存。
const MAX_SIDE: u32 = 20_000;

fn fonts() -> &'static fontdb::Database {
    static DB: OnceLock<fontdb::Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        let shared = scicanvas_core::text::fonts();
        for face in shared.faces() {
            db.load_font_data(face.data().to_vec());
        }
        db.set_sans_serif_family(&shared.faces()[shared.primary(false)].family);
        db
    })
}

/// 按 `dpi` 栅格化 SVG。SVG 的宽高以毫米给出，usvg 按 96 dpi 换成像素。
pub fn rasterize(svg: &str, dpi: u32) -> Result<Pixmap, String> {
    let mut tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| format!("解析 SVG 失败：{e}"))?;
    tree.convert_text(fonts());
    let scale = f64::from(dpi) / 96.0;
    let side = |v: f32| (f64::from(v) * scale).round().max(1.0);
    let (w, h) = (side(tree.size.width()), side(tree.size.height()));
    if w > f64::from(MAX_SIDE) || h > f64::from(MAX_SIDE) {
        return Err(format!("图像过大（{w}×{h} 像素），请降低 DPI"));
    }
    let mut pixmap = Pixmap::new(w as u32, h as u32).ok_or("无法分配位图")?;
    let s = scale as f32;
    resvg::Tree::from_usvg(&tree).render(Transform::from_scale(s, s), &mut pixmap.as_mut());
    Ok(pixmap)
}

/// 编码 PNG，并在 IHDR 后插入 pHYs 块记录分辨率。
pub fn png_bytes(pixmap: &Pixmap, dpi: u32) -> Result<Vec<u8>, String> {
    let png = pixmap.encode_png().map_err(|e| format!("编码 PNG 失败：{e}"))?;
    // 8 字节签名 + IHDR（4 长度 + 4 类型 + 13 数据 + 4 CRC）
    const AFTER_IHDR: usize = 8 + 25;
    if png.len() < AFTER_IHDR || &png[12..16] != b"IHDR" {
        return Err("PNG 结构异常".into());
    }
    let ppm = (f64::from(dpi) / 0.0254).round() as u32;
    let mut chunk = Vec::with_capacity(21);
    chunk.extend_from_slice(&9u32.to_be_bytes());
    chunk.extend_from_slice(b"pHYs");
    chunk.extend_from_slice(&ppm.to_be_bytes());
    chunk.extend_from_slice(&ppm.to_be_bytes());
    chunk.push(1); // 单位：米
    let crc = crc32fast::hash(&chunk[4..]);
    chunk.extend_from_slice(&crc.to_be_bytes());
    let mut out = Vec::with_capacity(png.len() + chunk.len());
    out.extend_from_slice(&png[..AFTER_IHDR]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&png[AFTER_IHDR..]);
    Ok(out)
}

/// 非预乘的 RGBA，供剪贴板使用。
pub fn straight_rgba(pixmap: &Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

pub fn copy_to_clipboard(pixmap: &Pixmap) -> Result<(), String> {
    let image = arboard::ImageData {
        width: pixmap.width() as usize,
        height: pixmap.height() as usize,
        bytes: straight_rgba(pixmap).into(),
    };
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_image(image))
        .map_err(|e| format!("无法写入剪贴板：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use scicanvas_core::{svg, Project};

    fn chunk<'a>(png: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
        let mut at = 8;
        while at + 8 <= png.len() {
            let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
            if &png[at + 4..at + 8] == kind {
                return Some(&png[at + 8..at + 8 + len]);
            }
            at += 12 + len;
        }
        None
    }

    #[test]
    fn export_resolves_the_exact_shared_font_faces() {
        for face in scicanvas_core::text::fonts().faces() {
            let id = fonts()
                .query(&fontdb::Query {
                    families: &[fontdb::Family::Name(&face.family)],
                    weight: fontdb::Weight(face.weight),
                    ..Default::default()
                })
                .expect("shared font family is loaded");
            fonts()
                .with_face_data(id, |bytes, index| {
                    assert_eq!(index, face.index);
                    assert_eq!(bytes, face.data());
                })
                .unwrap();
        }
    }

    #[test]
    fn measured_title_bounds_match_raster_export_at_multiple_dpi() {
        let mut p = Project::sample();
        let g = p.graph_mut(2).unwrap();
        g.page.width_mm = 89.0;
        g.page.height_mm = 80.0;
        g.first_layer_mut().title = "WWW iii μ ± 电压 mixed font measurement".into();
        let sc = scicanvas_core::scene::layout_page(p.graph(2).unwrap(), &p, &[]);
        let (x, y, text, size) = sc
            .items
            .iter()
            .find_map(|item| match &item.prim {
                scicanvas_core::scene::Prim::Text {
                    x,
                    y,
                    text,
                    size,
                    bold: true,
                    ..
                } => Some((*x, *y, text, *size)),
                _ => None,
            })
            .unwrap();
        let metrics = scicanvas_core::text::layout(text, size, true);
        let page = svg::render_page(p.graph(2).unwrap(), &p);
        for dpi in [96, 300, 600] {
            let pixmap = rasterize(&page, dpi).unwrap();
            let scale = f64::from(dpi) / 96.0;
            let top_end = ((y + metrics.height / 2.0 + 1.0) * scale).ceil() as u32;
            let mut count = 0;
            for yy in 0..top_end.min(pixmap.height()) {
                for xx in 0..pixmap.width() {
                    let color = pixmap.pixel(xx, yy).unwrap();
                    if color.red() < 100 && color.green() < 100 && color.blue() < 100 && color.alpha() > 100 {
                        assert!(
                            f64::from(xx) >= (x - metrics.width / 2.0 - 1.0) * scale
                                && f64::from(xx) <= (x + metrics.width / 2.0 + 1.0) * scale,
                            "{dpi}: {xx},{yy}"
                        );
                        count += 1;
                    }
                }
            }
            assert!(count > 30, "the title is rendered: {dpi}");
        }
        if let Ok(path) = std::env::var("SCICANVAS_TEXT_PREVIEW") {
            let transparent = page.replace("fill=\"#ffffff\"", "fill=\"none\"");
            std::fs::write(path, png_bytes(&rasterize(&transparent, 150).unwrap(), 150).unwrap()).unwrap();
        }
    }

    #[test]
    fn png_size_and_resolution_follow_page_millimetres_and_dpi() {
        let p = Project::sample();
        let g = &p.graphs[0];
        let page = svg::render_page(g, &p);
        for dpi in [96, 300, 600] {
            let pixmap = rasterize(&page, dpi).unwrap();
            let expected = |mm: f64| (mm / 25.4 * f64::from(dpi)).round() as u32;
            assert!(pixmap.width().abs_diff(expected(g.page.width_mm)) <= 1, "{dpi}");
            assert!(pixmap.height().abs_diff(expected(g.page.height_mm)) <= 1, "{dpi}");
            let png = png_bytes(&pixmap, dpi).unwrap();
            let phys = chunk(&png, b"pHYs").expect("pHYs chunk");
            let ppm = u32::from_be_bytes(phys[0..4].try_into().unwrap());
            assert_eq!(ppm, (f64::from(dpi) / 0.0254).round() as u32);
            assert_eq!(phys[8], 1);
            // 插入 pHYs 后仍是合法 PNG，能解码回同样的尺寸
            let back = Pixmap::decode_png(&png).unwrap();
            assert_eq!((back.width(), back.height()), (pixmap.width(), pixmap.height()));
        }
    }

    #[test]
    fn page_is_opaque_white_and_data_and_text_are_drawn() {
        let p = Project::sample();
        let pixmap = rasterize(&svg::render_page(&p.graphs[0], &p), 300).unwrap();
        let rgba = straight_rgba(&pixmap);
        assert_eq!(&rgba[0..4], &[255, 255, 255, 255], "corner is white paper");
        assert!(rgba.chunks(4).all(|c| c[3] == 255), "opaque page");
        let dark = rgba
            .chunks(4)
            .filter(|c| c[0] < 100 && c[1] < 100 && c[2] < 100)
            .count();
        let blue = rgba.chunks(4).filter(|c| c[2] > 100 && c[0] < 60).count();
        assert!(dark > 2_000, "axes and text are drawn: {dark}");
        assert!(blue > 500, "first series in deep blue: {blue}");
    }

    #[test]
    fn absurd_dpi_is_rejected_instead_of_allocating() {
        let p = Project::sample();
        assert!(rasterize(&svg::render_page(&p.graphs[0], &p), 100_000).is_err());
    }
}
