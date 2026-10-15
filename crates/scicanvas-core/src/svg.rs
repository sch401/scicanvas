//! 把图形渲染成 SVG（矢量图），用于导出论文插图。
//! 版面来自 scene 模块，和界面上看到的完全一致。

use crate::model::{Book, Graph, Symbol};
use crate::scene::{self, Anchor, Prim, Rgb};
use std::fmt::Write;

const FONT: &str = "Arial, 'Microsoft YaHei', 'PingFang SC', 'Noto Sans CJK SC', sans-serif";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// 符号路径，r 为半径。
pub fn symbol_path(sym: Symbol, x: f64, y: f64, r: f64) -> String {
    match scene::symbol_polygon(sym, x, y, r) {
        None => format!(
            "M{:.2},{:.2}a{r:.2},{r:.2} 0 1,0 {:.2},0a{r:.2},{r:.2} 0 1,0 {:.2},0",
            x - r,
            y,
            2.0 * r,
            -2.0 * r,
            r = r
        ),
        Some(pts) => {
            let mut p = String::new();
            for (i, q) in pts.iter().enumerate() {
                let _ = write!(p, "{}{:.2},{:.2}", if i == 0 { 'M' } else { 'L' }, q[0], q[1]);
            }
            p.push('z');
            p
        }
    }
}

fn prim(out: &mut String, p: &Prim) {
    match p {
        Prim::Rect {
            x,
            y,
            w,
            h,
            fill,
            stroke,
        } => {
            let _ = write!(out, r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}""#);
            match fill {
                Some((c, a)) => {
                    let _ = write!(out, r#" fill="{}""#, hex(*c));
                    if *a < 1.0 {
                        let _ = write!(out, r#" fill-opacity="{a}""#);
                    }
                }
                None => out.push_str(r#" fill="none""#),
            }
            if let Some((c, wd)) = stroke {
                let _ = write!(out, r#" stroke="{}" stroke-width="{wd}""#, hex(*c));
            }
            out.push_str("/>");
        }
        Prim::Segments {
            segs,
            color,
            width,
            dashed,
        } => {
            if segs.is_empty() {
                return;
            }
            let mut d = String::new();
            for s in segs {
                let _ = write!(d, "M{:.2},{:.2}L{:.2},{:.2}", s[0], s[1], s[2], s[3]);
            }
            let dash = if *dashed { r#" stroke-dasharray="3 3""# } else { "" };
            let _ = write!(
                out,
                r#"<path d="{d}" fill="none" stroke="{}" stroke-width="{width}"{dash}/>"#,
                hex(*color)
            );
        }
        Prim::Polyline { pts, color, width } => {
            let mut d = String::new();
            for (i, q) in pts.iter().enumerate() {
                let _ = write!(d, "{}{:.2},{:.2}", if i == 0 { 'M' } else { 'L' }, q[0], q[1]);
            }
            let _ = write!(
                out,
                r#"<path d="{d}" fill="none" stroke="{}" stroke-width="{width}" stroke-linejoin="round" stroke-linecap="round"/>"#,
                hex(*color)
            );
        }
        Prim::Triangles { pts, color, opacity } => {
            let mut d = String::new();
            for triangle in pts.chunks_exact(3) {
                let _ = write!(
                    d,
                    "M{:.2},{:.2}L{:.2},{:.2}L{:.2},{:.2}Z",
                    triangle[0][0], triangle[0][1], triangle[1][0], triangle[1][1], triangle[2][0], triangle[2][1]
                );
            }
            let _ = write!(
                out,
                r#"<path d="{d}" fill="{}" fill-opacity="{opacity}" stroke="none"/>"#,
                hex(*color)
            );
        }
        Prim::Symbols { sym, pts, r, color } => {
            if pts.is_empty() {
                return;
            }
            let mut d = String::new();
            for q in pts {
                d.push_str(&symbol_path(*sym, q[0], q[1], *r));
            }
            let _ = write!(out, r#"<path d="{d}" fill="{}"/>"#, hex(*color));
        }
        Prim::Text {
            x,
            y,
            text,
            size,
            anchor,
            vertical,
            bold,
            color,
        } => {
            let a = match anchor {
                Anchor::Start => "start",
                Anchor::Middle => "middle",
                Anchor::End => "end",
            };
            let pos = if *vertical {
                format!(r#"transform="translate({x:.2},{y:.2}) rotate(-90)""#)
            } else {
                format!(r#"x="{x:.2}" y="{y:.2}""#)
            };
            let weight = if *bold { r#" font-weight="700""# } else { "" };
            let _ = write!(
                out,
                r#"<text {pos} text-anchor="{a}" dominant-baseline="central" font-size="{size}"{weight} fill="{}">{}</text>"#,
                hex(*color),
                esc(text)
            );
        }
    }
}

/// 渲染整张图。`book` 为 None 时画一张提示“没有关联数据”的空图。
pub fn render(g: &Graph, book: Option<&Book>, w: f64, h: f64) -> String {
    let sc = scene::build(g, book, w, h, None);
    render_scene(g.id, &sc, w, h, None)
}

pub fn render_page(g: &Graph, project: &crate::Project) -> String {
    let sc = scene::layout_page(g, project, &[]);
    render_scene(g.id, &sc, sc.w, sc.h, Some((g.page.width_mm, g.page.height_mm)))
}

fn render_scene(id: u64, sc: &scene::Scene, w: f64, h: f64, physical: Option<(f64, f64)>) -> String {
    let mut clips = vec![sc.plot];
    for item in &sc.items {
        if let Some(rect) = item.clip {
            if !clips.contains(&rect) {
                clips.push(rect);
            }
        }
    }
    let clip_name = |i: usize| {
        if i == 0 {
            format!("clip{id}")
        } else {
            format!("clip{id}-{i}")
        }
    };
    let mut out = String::new();
    let dimensions = match physical {
        Some((width, height)) => format!(r#"width="{width}mm" height="{height}mm""#),
        None => format!(r#"width="{w}" height="{h}""#),
    };
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" {dimensions} viewBox="0 0 {w} {h}" font-family="{FONT}">"#
    );
    out.push_str("<defs>");
    for (i, [l, t, pw, ph]) in clips.iter().enumerate() {
        let clip = clip_name(i);
        let _ = write!(
            out,
            r#"<clipPath id="{clip}"><rect x="{l}" y="{t}" width="{pw}" height="{ph}"/></clipPath>"#
        );
    }
    out.push_str("</defs>");
    let mut in_clip = None;
    for it in &sc.items {
        if it.clip != in_clip {
            if in_clip.is_some() {
                out.push_str("</g>");
            }
            if let Some(rect) = it.clip {
                let clip = clip_name(clips.iter().position(|r| *r == rect).unwrap());
                let _ = write!(out, r#"<g clip-path="url(#{clip})">"#);
            }
            in_clip = it.clip;
        }
        prim(&mut out, &it.prim);
    }
    if in_clip.is_some() {
        out.push_str("</g>");
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PlotType, Project};

    #[test]
    fn renders_sample() {
        let p = Project::sample();
        let g = p.graph(2).unwrap();
        let svg = render(g, p.book(1), 800.0, 600.0);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("电压 A"));
        assert!(svg.contains("时间 (s)"));
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn renders_every_type_and_log() {
        let mut p = Project::sample();
        for t in PlotType::ALL {
            let g = p.graph_mut(2).unwrap();
            g.first_layer_mut().plot_type = t;
            g.first_layer_mut().y_log = true;
            g.first_layer_mut().grid = true;
            g.first_layer_mut().title = "测试 <标题>".into();
            let svg = render(p.graph(2).unwrap(), p.book(1), 640.0, 480.0);
            assert!(svg.contains("测试 &lt;标题&gt;"));
            assert!(!svg.contains("NaN"));
        }
    }

    #[test]
    fn fit_curve_colors() {
        let mut p = Project::sample();
        p.fit_poly(&crate::ops::Source::Graph(2), 1).unwrap();
        let svg = render(p.graph(2).unwrap(), p.book(1), 800.0, 600.0);
        // B 是黑色数据，拟合线红色；C 是红色数据，拟合线改用蓝色
        assert!(svg.contains("#e8182c"));
        assert!(svg.contains("#1f6fe5"));
        assert!(svg.contains("电压 A 线性拟合"));
    }

    #[test]
    fn empty_graph() {
        let g = Graph::new(9, "空");
        let svg = render(&g, None, 400.0, 300.0);
        assert!(svg.contains("没有关联数据"));
    }
}
