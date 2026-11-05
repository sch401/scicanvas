//! SVG 是导出的共用版面：PNG/剪贴板经 resvg 栅格化，PDF 保留矢量和嵌入字体子集。
//! PNG 写入 pHYs，PDF 以 72 pt/in 换算页面毫米尺寸。背景与保存的页面设置一致。

use std::sync::OnceLock;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb};

/// 复制到剪贴板时使用的分辨率。
pub const CLIPBOARD_DPI: u32 = 300;
/// 单边像素上限，防止误输入极大的 DPI 时耗尽内存。
const MAX_SIDE: u32 = 20_000;

/// Export the shared page SVG as vectors, with selectable text and embedded font subsets.
pub fn pdf_bytes(svg: &str) -> Result<Vec<u8>, String> {
    use resvg::usvg as pdf_svg;
    let tree = svg_tree(svg)?;
    fn used_fonts(group: &pdf_svg::Group, ids: &mut std::collections::BTreeSet<pdf_svg::fontdb::ID>) {
        for node in group.children() {
            match node {
                pdf_svg::Node::Group(g) => used_fonts(g, ids),
                pdf_svg::Node::Text(t) => {
                    for span in t.layouted() {
                        ids.extend(span.positioned_glyphs.iter().map(|glyph| glyph.font));
                    }
                }
                _ => {}
            }
            node.subroots(|g| used_fonts(g, ids));
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    used_fonts(tree.root(), &mut ids);
    for id in ids {
        let face = tree.fontdb().face(id).ok_or("PDF 字体信息缺失")?;
        let permission = tree
            .fontdb()
            .with_face_data(id, scicanvas_core::text::check_subset_embedding)
            .ok_or("PDF 字体数据缺失")?;
        permission.map_err(|e| format!("{}：{e}，请更换字体后导出", face.post_script_name))?;
    }
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions {
            embed_text: true,
            ..Default::default()
        },
        // The shared SVG uses CSS pixels (96/in); PDF page dimensions use points (72/in).
        svg2pdf::PageOptions { dpi: 96.0 },
    )
    .map_err(|e| format!("生成 PDF 失败：{e}"))
}

fn fonts() -> &'static std::sync::Arc<fontdb::Database> {
    static DB: OnceLock<std::sync::Arc<fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        let shared = scicanvas_core::text::fonts();
        for face in shared.faces() {
            db.load_font_data(face.data().to_vec());
        }
        db.set_sans_serif_family(&shared.faces()[shared.primary(false)].family);
        std::sync::Arc::new(db)
    })
}

/// PNG 与 PDF 共用同一解析器和实际字体，避免两条导出路径各自选择字体。
pub(crate) fn svg_tree(svg: &str) -> Result<usvg::Tree, String> {
    let options = usvg::Options {
        fontdb: fonts().clone(),
        ..Default::default()
    };
    usvg::Tree::from_str(svg, &options).map_err(|e| format!("解析 SVG 失败：{e}"))
}

/// 按 `dpi` 栅格化 SVG。SVG 的宽高以毫米给出，usvg 按 96 dpi 换成像素。
pub fn rasterize(svg: &str, dpi: u32) -> Result<Pixmap, String> {
    let tree = svg_tree(svg)?;
    let scale = f64::from(dpi) / 96.0;
    let side = |v: f32| (f64::from(v) * scale).round().max(1.0);
    let (w, h) = (side(tree.size().width()), side(tree.size().height()));
    if w > f64::from(MAX_SIDE) || h > f64::from(MAX_SIDE) {
        return Err(format!("图像过大（{w}×{h} 像素），请降低 DPI"));
    }
    let mut pixmap = Pixmap::new(w as u32, h as u32).ok_or("无法分配位图")?;
    let s = scale as f32;
    resvg::render(&tree, Transform::from_scale(s, s), &mut pixmap.as_mut());
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

/// 使用与 PNG 相同的图形结果，写入 LZW 压缩 TIFF 和每英寸分辨率。
pub fn tiff_bytes(pixmap: &Pixmap, dpi: u32) -> Result<Vec<u8>, String> {
    use tiff::encoder::{colortype::RGBA8, Compression, Rational, TiffEncoder};
    use tiff::tags::{ResolutionUnit, Tag};
    let mut output = std::io::Cursor::new(Vec::new());
    {
        let mut encoder = TiffEncoder::new(&mut output)
            .map_err(|e| format!("编码 TIFF 失败：{e}"))?
            .with_compression(Compression::Lzw);
        let mut image = encoder
            .new_image::<RGBA8>(pixmap.width(), pixmap.height())
            .map_err(|e| format!("编码 TIFF 失败：{e}"))?;
        image.resolution(ResolutionUnit::Inch, Rational { n: dpi, d: 1 });
        image
            .encoder()
            .write_tag(Tag::ExtraSamples, &[2_u16][..])
            .map_err(|e| format!("编码 TIFF 失败：{e}"))?;
        image
            .write_data(&straight_rgba(pixmap))
            .map_err(|e| format!("编码 TIFF 失败：{e}"))?;
    }
    Ok(output.into_inner())
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

/// 剪贴板位图：非预乘 RGBA，尺寸与栅格化结果一致。
pub fn clipboard_image(pixmap: &Pixmap) -> arboard::ImageData<'static> {
    arboard::ImageData {
        width: pixmap.width() as usize,
        height: pixmap.height() as usize,
        bytes: straight_rgba(pixmap).into(),
    }
}

pub fn copy_to_clipboard(pixmap: &Pixmap) -> Result<(), String> {
    let image = clipboard_image(pixmap);
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_image(image))
        .map_err(|e| format!("无法写入剪贴板：{e}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn b7_shapes_hatches_export_as_vectors_and_rasterize() {
        use scicanvas_core::{
            page::{CurveStyle, FillPattern, PatternKind, ShapeKind},
            PlotType, Project,
        };
        let mut p = Project::sample();
        let layer = p.graphs[0].first_layer_mut();
        layer.set_plot_type(PlotType::Area).unwrap();
        layer.set_plot_style(
            "B",
            Some(CurveStyle {
                pattern: Some(FillPattern {
                    kind: PatternKind::Cross,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );
        for (kind, a, b) in [
            (ShapeKind::Line, [0.1, 0.1], [0.3, 0.1]),
            (ShapeKind::Arrow, [0.1, 0.2], [0.3, 0.3]),
            (ShapeKind::Rect, [0.6, 0.1], [0.8, 0.3]),
            (ShapeKind::Ellipse, [0.6, 0.5], [0.8, 0.7]),
        ] {
            p.graphs[0].page.add_shape(kind, a, b).unwrap();
        }
        let page = scicanvas_core::svg::render_page(&p.graphs[0], &p);
        assert!(!page.contains("<pattern"));
        let image = super::rasterize(&page, 150).unwrap();
        assert!(image.width() > 100 && image.height() > 100);
        let png = super::png_bytes(&image, 150).unwrap();
        assert!(png.starts_with(&[137, 80, 78, 71]));
        let pdf = super::pdf_bytes(&page).unwrap();
        assert!(pdf.starts_with(b"%PDF-1."));
        assert!(!String::from_utf8_lossy(&pdf).contains("/Subtype /Image"));
    }

    use super::*;
    use scicanvas_core::{svg, Project};

    #[test]
    fn pdf_exports_vectors_embedded_fonts_and_page_sizes_for_every_preset() {
        use scicanvas_core::page::{CurveStyle, LabelSource, LayerRect};
        let mut cases = Vec::new();
        for preset in scicanvas_core::presets::all() {
            let mut p = Project::default();
            let book = scicanvas_core::presets::example_for(preset.kind);
            let mut graph = scicanvas_core::Graph::new(2, preset.name);
            graph.first_layer_mut().spec = scicanvas_core::plotspec::PlotSpec::from_selection(&book, &[], preset.kind)
                .unwrap()
                .into();
            p.books.push(book);
            p.graphs.push(graph);
            cases.push((preset.id.to_string(), p));
        }
        let mut scientific = Project::sample();
        let layer = scientific.graphs[0].first_layer_mut();
        layer.rich_text = true;
        layer.title = r"CO_{2} R^{2} \b{normal} \i{rate} α 中文".into();
        layer.x_title = r"Time / s^{−1}".into();
        layer.y_title = r"\i{c} / mol L^{−1}".into();
        layer.y2_title = r"\b{R}_{0}".into();
        layer.edit_styles(|styles| {
            styles.insert(
                "B".into(),
                CurveStyle {
                    legend_label: r"\i{CO}_{2} β".into(),
                    ..Default::default()
                },
            )
        });
        layer.edit_styles(|styles| {
            styles.insert(
                "C".into(),
                CurveStyle {
                    right_axis: true,
                    ..Default::default()
                },
            )
        });
        layer.labels.visible = true;
        layer.labels.source = LabelSource::Column;
        layer.labels.column = Some("E".into());
        let mut error = scicanvas_core::Column::new("D", scicanvas_core::Role::YError);
        error.data = vec!["0.5".to_string(); 41].into();
        error.error_for = Some("B".into());
        scientific.books[0].columns.push(error);
        let mut label = scicanvas_core::Column::new("E", scicanvas_core::Role::Label);
        label.data = vec![r"\i{p}_{0}".to_string()].into();
        scientific.books[0].columns.push(label);
        scientific.graphs[0].page.width_mm = 89.0;
        scientific.graphs[0].page.height_mm = 80.0;
        scientific.graphs[0].first_layer_mut().legend_position = Some([0.52, 0.73]);
        let id = scientific.graphs[0].page.add_text([0.15, 0.15]).unwrap();
        scientific.graphs[0].page.text_mut(id).unwrap().text = "Annotation α 中文\n\\b{Result} R^{2}".into();
        cases.push(("scientific".into(), scientific.clone()));
        let mut inset = scientific.graphs[0].first_layer().clone();
        scientific.graphs[0].page.add_layer(inset.clone());
        scientific.graphs[0].page.columns = 2;
        inset.transparent = true;
        inset.placement = Some(LayerRect {
            x: 0.6,
            y: 0.05,
            width: 0.35,
            height: 0.35,
        });
        scientific.graphs[0].page.add_layer(inset);
        scientific.graphs[0].page.width_mm = 210.0;
        scientific.graphs[0].page.height_mm = 157.5;
        cases.push(("panels-inset".into(), scientific));
        for (name, p) in cases {
            let before = p.clone();
            let page = svg::render_page(&p.graphs[0], &p);
            let pdf = pdf_bytes(&page).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(pdf.starts_with(b"%PDF-1."));
            let structure = String::from_utf8_lossy(&pdf);
            assert!(
                structure.contains("/FontFile2") || structure.contains("/FontFile3"),
                "{name}: font streams embedded"
            );
            assert!(structure.contains("/ToUnicode"), "{name}: selectable Unicode text");
            assert!(
                !structure.contains("/Subtype /Image"),
                "{name}: vectors, no page raster"
            );
            let box_text = structure
                .split("/MediaBox [")
                .nth(1)
                .unwrap()
                .split(']')
                .next()
                .unwrap();
            let dimensions: Vec<f64> = box_text.split_whitespace().map(|v| v.parse().unwrap()).collect();
            assert_eq!(&dimensions[..2], &[0.0, 0.0]);
            assert!((dimensions[2] - p.graphs[0].page.width_mm * 72.0 / 25.4).abs() < 0.01);
            assert!((dimensions[3] - p.graphs[0].page.height_mm * 72.0 / 25.4).abs() < 0.01);
            assert_eq!(p, before, "export leaves project/history inputs unchanged");
            if let Ok(directory) = std::env::var("SCICANVAS_PDF_PROBE_DIR") {
                let dir = std::path::Path::new(&directory);
                std::fs::create_dir_all(dir).unwrap();
                std::fs::write(dir.join(format!("{name}.pdf")), pdf).unwrap();
                std::fs::write(dir.join(format!("{name}.svg")), &page).unwrap();
                let png = rasterize(&page, 150).unwrap();
                std::fs::write(dir.join(format!("{name}.png")), png_bytes(&png, 150).unwrap()).unwrap();
            }
        }
        assert!(pdf_bytes("invalid SVG").is_err());
    }

    #[test]
    fn mapped_area_fill_and_outline_match_screen_svg_png_and_vector_pdf() {
        use eframe::egui;
        use scicanvas_core::{
            colormap::ColorMap,
            page::CurveStyle,
            scene::{self, Prim},
            PlotType,
        };
        let mut project = Project::sample();
        for (column, values) in project.books[0].columns.iter_mut().zip([
            vec!["0", "1", "2", "3"],
            vec!["4", "4", "4", "4"],
            vec!["0", "0", "1", "1"],
        ]) {
            column.data = values.into_iter().map(str::to_owned).collect();
        }
        let graph = &mut project.graphs[0];
        graph.page.width_mm = 89.0;
        graph.page.height_mm = 70.0;
        let layer = graph.first_layer_mut();
        layer.set_plot_type(PlotType::Area).unwrap();
        layer.edit_draft(|draft| draft.y_keys = vec!["B".into()]);
        layer.legend = false;
        layer.x_auto = false;
        layer.x_min = 0.0;
        layer.x_max = 3.0;
        layer.y_auto = false;
        layer.y_min = 0.0;
        layer.y_max = 5.0;
        layer.area_opacity = 0.5;
        layer.color_map = ColorMap::Viridis;
        layer.set_plot_style(
            "B",
            Some(CurveStyle {
                color_column: Some("C".into()),
                line_width: Some(3.0),
                ..Default::default()
            }),
        );
        let sc = scene::layout_page(&project.graphs[0], &project, &[]);
        let page = svg::render_page(&project.graphs[0], &project);
        let expected = [ColorMap::Viridis.sample(0.0), ColorMap::Viridis.sample(1.0)];
        for color in expected {
            let hex = format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2);
            assert!(page.contains(&format!("fill=\"{hex}\"")));
            assert!(page.contains(&format!("stroke=\"{hex}\" stroke-width=\"3\" stroke-linecap=\"round\"")));
        }
        assert!(sc
            .items
            .iter()
            .any(|it| matches!(it.prim, Prim::Triangles { opacity: 0.5, .. })));
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        for scale in [0.75, 1.5] {
            let rect = egui::Rect::from_min_size(egui::pos2(20.0, 25.0), egui::vec2(sc.w as f32, sc.h as f32) * scale);
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| crate::plot::paint(ui, rect, &sc));
            });
            for (r, g, b) in expected {
                let color = egui::Color32::from_rgb(r, g, b);
                assert!(output.shapes.iter().any(|item| matches!(&item.shape,
                    egui::Shape::Mesh(mesh) if !mesh.vertices.is_empty() && mesh.vertices.iter().all(|v| v.color == color.gamma_multiply(0.5))
                )));
                assert!(output.shapes.iter().any(|item| matches!(&item.shape,
                    egui::Shape::LineSegment { stroke, .. } if stroke.color == eframe::epaint::ColorMode::Solid(color)
                )));
            }
        }
        for dpi in [96, 192] {
            let pixmap = rasterize(&page, dpi).unwrap();
            let scale = f64::from(dpi) / 96.0;
            for (x, color) in [(0.75, expected[0]), (2.25, expected[1])] {
                let px = (sc.px(x) * scale).round() as u32;
                let py = (sc.py(2.0) * scale).round() as u32;
                let pixel = pixmap.pixel(px, py).unwrap();
                let blended =
                    [color.0, color.1, color.2].map(|channel| (f64::from(channel) * 0.5 + 127.5).round() as u8);
                for (actual, expected) in [pixel.red(), pixel.green(), pixel.blue()].into_iter().zip(blended) {
                    assert!(
                        actual.abs_diff(expected) <= 2,
                        "{dpi}: fill at {x}: {actual} != {expected}"
                    );
                }
                let y = (sc.py(4.0) * scale).round() as i32;
                assert!(
                    (-2..=2).any(|dy| {
                        let pixel = pixmap.pixel(px, (y + dy) as u32).unwrap();
                        [pixel.red(), pixel.green(), pixel.blue()]
                            .into_iter()
                            .zip([color.0, color.1, color.2])
                            .all(|(a, b)| a.abs_diff(b) <= 2)
                    }),
                    "{dpi}: opaque outline at {x}"
                );
            }
        }
        let pdf = pdf_bytes(&page).unwrap();
        assert!(pdf.starts_with(b"%PDF-1."));
        assert!(!String::from_utf8_lossy(&pdf).contains("/Subtype /Image"));
        if let Ok(directory) = std::env::var("SCICANVAS_PDF_PROBE_DIR") {
            let dir = std::path::Path::new(&directory);
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("mapped-area.pdf"), pdf).unwrap();
            std::fs::write(dir.join("mapped-area.svg"), &page).unwrap();
            let png = rasterize(&page, 150).unwrap();
            std::fs::write(dir.join("mapped-area.png"), png_bytes(&png, 150).unwrap()).unwrap();
        }
    }

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
                    style: if face.italic {
                        fontdb::Style::Italic
                    } else {
                        fontdb::Style::Normal
                    },
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
    fn scientific_text_exports_real_styles_scripts_and_scaled_raster_bounds() {
        let mut p = Project::sample();
        let g = &mut p.graphs[0];
        g.page.width_mm = 89.0;
        g.page.height_mm = 80.0;
        let layer = g.first_layer_mut();
        layer.rich_text = true;
        layer.title = r"CO_{2} R^{2} \b{normal} \i{rate} α".into();
        layer.x_title = r"Time / s^{−1}".into();
        layer.y_title = r"\i{c} / mol L^{−1}".into();
        let page = svg::render_page(&p.graphs[0], &p);
        assert!(page.contains("font-style=\"italic\""));
        assert!(page.contains("font-weight=\"700\""));
        assert!(page.contains("aria-label=\"CO2 R2 normal rate α\""));
        let sc = scicanvas_core::scene::layout_page(&p.graphs[0], &p, &[]);
        let (x, y, layout) = sc
            .items
            .iter()
            .find_map(|i| match &i.prim {
                scicanvas_core::scene::Prim::Text {
                    x,
                    y,
                    text,
                    size,
                    bold: true,
                    rich,
                    ..
                } => Some((*x, *y, scicanvas_core::text::layout_formatted(text, *size, true, *rich))),
                _ => None,
            })
            .unwrap();
        assert!(layout.runs.iter().any(|r| r.dy < 0.0));
        assert!(layout.runs.iter().any(|r| r.dy > 0.0));
        let mut bounds = Vec::new();
        for dpi in [96, 300, 600] {
            let pixmap = rasterize(&page, dpi).unwrap();
            let scale = f64::from(dpi) / 96.0;
            let end = ((y + layout.height / 2.0 + 2.0) * scale).ceil() as u32;
            let (mut xmin, mut ymin, mut xmax, mut ymax) = (u32::MAX, u32::MAX, 0, 0);
            for yy in 0..end.min(pixmap.height()) {
                for xx in 0..pixmap.width() {
                    let c = pixmap.pixel(xx, yy).unwrap();
                    // Include antialiased italic edges, rather than only their dark interiors.
                    if c.alpha() > 100 && c.red() < 220 && c.green() < 220 && c.blue() < 220 {
                        xmin = xmin.min(xx);
                        ymin = ymin.min(yy);
                        xmax = xmax.max(xx);
                        ymax = ymax.max(yy);
                    }
                }
            }
            assert!(xmin < xmax && ymin < ymax, "formatted title is visible");
            let b = [xmin, ymin, xmax, ymax].map(|n| f64::from(n) / scale);
            assert!(b[0] >= x - layout.width / 2.0 - 1.0 && b[2] <= x + layout.width / 2.0 + 1.0);
            assert!(b[1] >= y - layout.height / 2.0 - 1.0 && b[3] <= y + layout.height / 2.0 + 1.0);
            bounds.push(b);
        }
        for b in &bounds[1..] {
            assert!(
                b.iter().zip(bounds[0]).all(|(a, b)| (a - b).abs() < 1.5),
                "DPI changes scale the same layout: {bounds:?}"
            );
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
    fn clipboard_copy_uses_a_300_dpi_straight_rgba_page_image() {
        let p = Project::sample();
        let g = &p.graphs[0];
        let pixmap = rasterize(&svg::render_page(g, &p), CLIPBOARD_DPI).unwrap();
        let image = clipboard_image(&pixmap);
        let expected = |mm: f64| (mm / 25.4 * f64::from(CLIPBOARD_DPI)).round() as usize;
        assert!(image.width.abs_diff(expected(g.page.width_mm)) <= 1);
        assert!(image.height.abs_diff(expected(g.page.height_mm)) <= 1);
        assert_eq!(image.bytes.len(), image.width * image.height * 4);
        assert_eq!(&image.bytes[0..4], &[255, 255, 255, 255], "页面左上角是不透明白色");
    }

    /// docs/parity/export 下导出样例使用的图：示例项目第一张图，单栏 89 × 80 mm。
    fn export_sample() -> (Project, String) {
        let mut p = Project::sample();
        p.graphs[0].page.width_mm = 89.0;
        p.graphs[0].page.height_mm = 80.0;
        let page = svg::render_page(&p.graphs[0], &p);
        (p, page)
    }

    fn export_samples_directory() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/parity/export")
    }

    #[test]
    #[ignore = "维护者在 Windows 上显式生成导出样例，目视复核后提交"]
    fn update_export_samples() {
        let (_, page) = export_sample();
        let dir = export_samples_directory();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sample.svg"), &page).unwrap();
        let pixmap = rasterize(&page, 300).unwrap();
        std::fs::write(dir.join("sample-300dpi.png"), png_bytes(&pixmap, 300).unwrap()).unwrap();
        std::fs::write(dir.join("sample.pdf"), pdf_bytes(&page).unwrap()).unwrap();
    }

    #[test]
    fn committed_export_samples_keep_page_size_and_resolution() {
        let (p, page) = export_sample();
        let g = &p.graphs[0];
        let dir = export_samples_directory();
        assert!(dir.join("sample.svg").is_file() && dir.join("README.md").is_file());
        let png = std::fs::read(dir.join("sample-300dpi.png")).unwrap();
        let committed = Pixmap::decode_png(&png).unwrap();
        let current = rasterize(&page, 300).unwrap();
        assert_eq!(
            (committed.width(), committed.height()),
            (current.width(), current.height()),
            "页面尺寸变了：重新生成导出样例并复核"
        );
        let phys = chunk(&png, b"pHYs").expect("pHYs chunk");
        assert_eq!(
            u32::from_be_bytes(phys[0..4].try_into().unwrap()),
            (300.0_f64 / 0.0254).round() as u32
        );
        let pdf = std::fs::read(dir.join("sample.pdf")).unwrap();
        assert!(pdf.starts_with(b"%PDF-1."));
        let structure = String::from_utf8_lossy(&pdf);
        let box_text = structure
            .split("/MediaBox [")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap();
        let dimensions: Vec<f64> = box_text.split_whitespace().map(|v| v.parse().unwrap()).collect();
        assert!((dimensions[2] - g.page.width_mm * 72.0 / 25.4).abs() < 0.01);
        assert!((dimensions[3] - g.page.height_mm * 72.0 / 25.4).abs() < 0.01);
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

    #[test]
    fn tiff_preserves_rgba_lzw_and_dpi() {
        use tiff::decoder::{Decoder, DecodingResult};
        use tiff::tags::Tag;
        let mut pixmap = Pixmap::new(3, 2).unwrap();
        pixmap.fill(resvg::tiny_skia::Color::from_rgba8(120, 80, 200, 128));
        let expected = straight_rgba(&pixmap);
        let encoded = tiff_bytes(&pixmap, 600).unwrap();
        let mut decoder = Decoder::new(std::io::Cursor::new(encoded)).unwrap();
        assert_eq!(decoder.dimensions().unwrap(), (3, 2));
        assert_eq!(decoder.get_tag_u32(Tag::Compression).unwrap(), 5);
        assert_eq!(decoder.get_tag_u32(Tag::ResolutionUnit).unwrap(), 2);
        assert_eq!(
            decoder.get_tag(Tag::XResolution).unwrap(),
            tiff::decoder::ifd::Value::Rational(600, 1)
        );
        assert_eq!(
            decoder.get_tag(Tag::YResolution).unwrap(),
            tiff::decoder::ifd::Value::Rational(600, 1)
        );
        assert_eq!(decoder.get_tag_u32(Tag::ExtraSamples).unwrap(), 2);
        let DecodingResult::U8(actual) = decoder.read_image().unwrap() else {
            panic!("expected RGBA8")
        };
        assert_eq!(actual, expected);
    }
}
