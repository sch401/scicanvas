//! 图形版面：把一张图排成像素坐标下的图元列表。
//!
//! 界面上的绘制和 SVG 导出都从这里取图元，所以屏幕上看到的和导出的论文插图完全一致。
//! 默认是常见的 SCI 期刊风格：白底、四边闭合的黑色坐标框、刻度朝内且四边都有、
//! 不画网格、图例无边框并自动避开数据。

use crate::coord::{Cartesian, Coord};
use crate::model::{series_styles, Book, Graph, PlotType, Symbol};
use crate::page::Layer;
use crate::ticks::{axis, format_tick, Axis};

pub type Rgb = (u8, u8, u8);

pub const INK: Rgb = (0, 0, 0);
pub const PAPER: Rgb = (255, 255, 255);
const GRID: Rgb = (217, 217, 217);
const MUTED: Rgb = (136, 136, 136);

/// 坐标框线宽、主刻度 / 次刻度长度（像素）。
pub const AXIS_WIDTH: f64 = 1.2;
pub const MAJOR_TICK: f64 = 5.0;
pub const MINOR_TICK: f64 = 2.8;
/// 刻度标签、轴标题、图例、图标题的字号。
pub const TICK_FONT: f64 = 13.0;
pub const LABEL_FONT: f64 = 14.0;
pub const LEGEND_FONT: f64 = 12.5;
pub const TITLE_FONT: f64 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// 矩形；fill 为 (颜色, 不透明度)，stroke 为 (颜色, 线宽)。
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: Option<(Rgb, f64)>,
        stroke: Option<(Rgb, f64)>,
    },
    /// 一组独立线段 [x1, y1, x2, y2]（刻度、网格）。
    Segments {
        segs: Vec<[f64; 4]>,
        color: Rgb,
        width: f64,
        dashed: bool,
    },
    /// 折线。
    Polyline { pts: Vec<[f64; 2]>, color: Rgb, width: f64 },
    /// 一组数据符号，r 为半径。
    Symbols {
        sym: Symbol,
        pts: Vec<[f64; 2]>,
        r: f64,
        color: Rgb,
    },
    /// 文字。(x, y) 是锚点：水平方向按 anchor 对齐，垂直方向居中。
    /// vertical 为真时逆时针旋转 90°（Y 轴标题）。
    Text {
        x: f64,
        y: f64,
        text: String,
        size: f64,
        anchor: Anchor,
        vertical: bool,
        bold: bool,
        color: Rgb,
    },
}

/// 一个图元，以及它是否要裁剪到坐标框以内。
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub prim: Prim,
    pub clip: Option<[f64; 4]>,
}

/// 视图范围（数据单位），用于界面上的缩放和平移；None 表示按图形设置自动或手动范围。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub x: (f64, f64),
    pub y: (f64, f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub w: f64,
    pub h: f64,
    /// 坐标框 [左, 上, 宽, 高]。
    pub plot: [f64; 4],
    pub xa: Axis,
    pub ya: Axis,
    pub items: Vec<Item>,
    pub layer_plots: Vec<[f64; 4]>,
}

impl Scene {
    pub fn px(&self, v: f64) -> f64 {
        self.coord().project([v, self.ya.lo])[0]
    }

    pub fn py(&self, v: f64) -> f64 {
        self.coord().project([self.xa.lo, v])[1]
    }

    /// 像素坐标换算回数据坐标。
    pub fn data_at(&self, x: f64, y: f64) -> (f64, f64) {
        let [x, y] = self.coord().unproject([x, y]);
        (x, y)
    }

    fn coord(&self) -> Cartesian<'_> {
        Cartesian {
            x: &self.xa,
            y: &self.ya,
            rect: self.plot,
        }
    }

    /// 当前坐标范围，作为缩放平移的起点。
    pub fn view(&self) -> View {
        View {
            x: (self.xa.lo, self.xa.hi),
            y: (self.ya.lo, self.ya.hi),
        }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        let [l, t, w, h] = self.plot;
        x >= l && x <= l + w && y >= t && y <= t + h
    }
}

/// 粗略估计文字宽度（像素）：中日韩字符按一个字号宽，其余按 0.56 个字号。
pub fn text_width(s: &str, px: f64) -> f64 {
    s.chars()
        .map(|c| if (c as u32) >= 0x2E80 { px } else { px * 0.56 })
        .sum()
}

/// 符号的多边形顶点；圆形返回 None（由调用方画圆）。
pub fn symbol_polygon(sym: Symbol, x: f64, y: f64, r: f64) -> Option<Vec<[f64; 2]>> {
    let p = match sym {
        Symbol::Circle => return None,
        Symbol::Square => vec![[x - r, y - r], [x + r, y - r], [x + r, y + r], [x - r, y + r]],
        Symbol::Up => vec![[x, y - r * 1.2], [x + r * 1.1, y + r * 0.8], [x - r * 1.1, y + r * 0.8]],
        Symbol::Down => vec![[x, y + r * 1.2], [x - r * 1.1, y - r * 0.8], [x + r * 1.1, y - r * 0.8]],
        Symbol::Diamond => vec![[x, y - r * 1.3], [x + r * 1.1, y], [x, y + r * 1.3], [x - r * 1.1, y]],
        Symbol::Left => vec![[x - r * 1.2, y], [x + r * 0.8, y - r * 1.1], [x + r * 0.8, y + r * 1.1]],
        Symbol::Right => vec![[x + r * 1.2, y], [x - r * 0.8, y + r * 1.1], [x - r * 0.8, y - r * 1.1]],
        Symbol::Hexagon => (0..6)
            .map(|i| {
                let a = std::f64::consts::PI / 3.0 * i as f64;
                [x + r * 1.15 * a.cos(), y + r * 1.15 * a.sin()]
            })
            .collect(),
    };
    Some(p)
}

/// 柱状图：相邻 X 的最小间距，决定柱宽。
fn min_spacing(xs: &[f64]) -> f64 {
    let mut u: Vec<f64> = xs.to_vec();
    u.sort_by(|a, b| a.partial_cmp(b).unwrap());
    u.dedup();
    let d = u.windows(2).map(|w| w[1] - w[0]).fold(f64::INFINITY, f64::min);
    if d.is_finite() && d > 0.0 {
        d
    } else {
        1.0
    }
}

fn text(x: f64, y: f64, s: impl Into<String>, size: f64, anchor: Anchor) -> Prim {
    Prim::Text {
        x,
        y,
        text: s.into(),
        size,
        anchor,
        vertical: false,
        bold: false,
        color: INK,
    }
}

/// 排版整张图。`book` 为 None 时是一张提示“没有关联数据”的空图。
pub fn build(g: &Graph, book: Option<&Book>, w: f64, h: f64, view: Option<View>) -> Scene {
    compose(g, |id| book.filter(|b| b.id == id), w, h, view)
}

/// 页面各图层独立解析工作簿；屏幕与整页导出使用同一入口。
pub fn build_page(g: &Graph, project: &crate::Project, w: f64, h: f64, view: Option<View>) -> Scene {
    compose(g, |id| project.book(id), w, h, view)
}

fn compose<'a>(g: &Graph, book: impl Fn(u64) -> Option<&'a Book>, w: f64, h: f64, view: Option<View>) -> Scene {
    let cols = g.page.columns.max(1);
    let rows = g.page.layers().len().div_ceil(cols).max(1);
    let (cw, ch) = (w / cols as f64, h / rows as f64);
    let mut scenes = g.page.layers().iter().enumerate().map(|(i, layer)| {
        let mut sc = build_layer(
            layer,
            layer.book_id.and_then(&book),
            cw,
            ch,
            if i == 0 { view } else { None },
        );
        let (dx, dy) = ((i % cols) as f64 * cw, (i / cols) as f64 * ch);
        for item in &mut sc.items {
            translate(&mut item.prim, dx, dy);
            if let Some(rect) = &mut item.clip {
                rect[0] += dx;
                rect[1] += dy;
            }
        }
        sc.plot[0] += dx;
        sc.plot[1] += dy;
        sc
    });
    let mut page = scenes
        .next()
        .unwrap_or_else(|| build_layer(&Layer::default(), None, w, h, None));
    page.w = w;
    page.h = h;
    if g.page.layers().len() > 1 {
        page.items.insert(
            0,
            Item {
                prim: Prim::Rect {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                    fill: Some((PAPER, 1.0)),
                    stroke: None,
                },
                clip: None,
            },
        );
    }
    page.layer_plots = vec![page.plot];
    for sc in scenes {
        page.layer_plots.push(sc.plot);
        page.items.extend(sc.items);
    }
    page
}

fn translate(prim: &mut Prim, dx: f64, dy: f64) {
    match prim {
        Prim::Rect { x, y, .. } | Prim::Text { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        Prim::Polyline { pts, .. } | Prim::Symbols { pts, .. } => {
            for p in pts {
                p[0] += dx;
                p[1] += dy;
            }
        }
        Prim::Segments { segs, .. } => {
            for p in segs {
                p[0] += dx;
                p[1] += dy;
                p[2] += dx;
                p[3] += dy;
            }
        }
    }
}

fn build_layer(g: &Layer, book: Option<&Book>, w: f64, h: f64, view: Option<View>) -> Scene {
    // 数据（对数轴去掉非正值）
    let (_, mut series) = match book {
        Some(b) => g.series(b),
        None => (None, Vec::new()),
    };
    for s in &mut series {
        s.points
            .retain(|p| (!g.x_log || p[0] > 0.0) && (!g.y_log || p[1] > 0.0));
    }
    let all_x: Vec<f64> = series.iter().flat_map(|s| s.points.iter().map(|p| p[0])).collect();
    let all_y: Vec<f64> = series.iter().flat_map(|s| s.points.iter().map(|p| p[1])).collect();
    let is_bar = g.plot_type == PlotType::Bar;
    let mut styles = series_styles(&series, g.plot_type);
    if g.color_map != crate::colormap::ColorMap::Origin {
        let n = series.iter().filter(|s| !s.line_only).count();
        for style in &mut styles {
            if let crate::SeriesColor::Palette(i) = style.color {
                style.color = crate::SeriesColor::Rgb(g.color_map.sample(i as f64 / n.saturating_sub(1).max(1) as f64));
            }
        }
    }
    // 柱宽只按数据曲线的 X 间距算（拟合曲线是 200 个密集点）
    let data_x: Vec<f64> = series
        .iter()
        .filter(|s| !s.line_only)
        .flat_map(|s| s.points.iter().map(|p| p[0]))
        .collect();
    let dx = min_spacing(&data_x);

    let (xa, ya): (Axis, Axis) = match view {
        Some(v) => (
            axis(&[], false, v.x.0, v.x.1, g.x_log, false),
            axis(&[], false, v.y.0, v.y.1, g.y_log, false),
        ),
        None => {
            let xa = if is_bar && g.x_auto && !g.x_log && !all_x.is_empty() {
                let lo = all_x.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = all_x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                axis(&[lo - dx * 0.6, hi + dx * 0.6], true, 0.0, 0.0, false, false)
            } else {
                axis(&all_x, g.x_auto, g.x_min, g.x_max, g.x_log, false)
            };
            (xa, axis(&all_y, g.y_auto, g.y_min, g.y_max, g.y_log, is_bar))
        }
    };

    // 版面：刻度朝内，标签紧贴坐标框
    let ylab_w = ya
        .ticks
        .iter()
        .map(|t| text_width(&format_tick(*t), TICK_FONT))
        .fold(14.0, f64::max);
    let has_title = !g.title.trim().is_empty();
    let left = (ylab_w + 36.0).round();
    let right = 18.0;
    let top = if has_title { 40.0 } else { 16.0 };
    let bottom = 50.0;
    let pw = (w - left - right).max(20.0);
    let ph = (h - top - bottom).max(20.0);
    let coord = Cartesian {
        x: &xa,
        y: &ya,
        rect: [left, top, pw, ph],
    };
    let px = |v: f64| coord.project([v, ya.lo])[0];
    let py = |v: f64| coord.project([xa.lo, v])[1];
    let inside_x = |x: f64| x >= left - 0.5 && x <= left + pw + 0.5;
    let inside_y = |y: f64| y >= top - 0.5 && y <= top + ph + 0.5;

    let mut items: Vec<Item> = Vec::new();
    let mut push = |prim: Prim, clip: bool| {
        items.push(Item {
            prim,
            clip: clip.then_some([left, top, pw, ph]),
        })
    };
    push(
        Prim::Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            fill: Some((PAPER, 1.0)),
            stroke: None,
        },
        false,
    );

    // 网格（默认关闭）
    if g.grid {
        let mut segs = Vec::new();
        for t in &ya.ticks {
            let y = py(*t);
            if y > top + 0.5 && y < top + ph - 0.5 {
                segs.push([left, y, left + pw, y]);
            }
        }
        for t in &xa.ticks {
            let x = px(*t);
            if x > left + 0.5 && x < left + pw - 0.5 {
                segs.push([x, top, x, top + ph]);
            }
        }
        push(
            Prim::Segments {
                segs,
                color: GRID,
                width: 1.0,
                dashed: true,
            },
            true,
        );
    }

    // 曲线：线和柱子裁剪到坐标框内；符号只画范围内的点、不裁剪，落在坐标轴上的点保持完整
    if all_x.is_empty() {
        let msg = if book.is_some() {
            "没有可绘制的数值数据"
        } else {
            "没有关联数据"
        };
        push(
            Prim::Text {
                x: left + pw / 2.0,
                y: top + ph / 2.0,
                text: msg.into(),
                size: 13.0,
                anchor: Anchor::Middle,
                vertical: false,
                bold: false,
                color: MUTED,
            },
            false,
        );
    }
    let mut symbols: Vec<Prim> = Vec::new();
    for (s, st) in series.iter().zip(&styles) {
        let color = st.color.rgb();
        if st.kind == PlotType::Bar {
            let gw = dx * 0.7;
            let bw = gw / st.bar_count.max(1) as f64;
            let base = if ya.log { ya.lo } else { 0f64.clamp(ya.lo, ya.hi) };
            let y0 = py(base);
            for p in &s.points {
                let x1 = px(p[0] - gw / 2.0 + st.bar_index as f64 * bw);
                let x2 = px(p[0] - gw / 2.0 + (st.bar_index as f64 + 1.0) * bw);
                let yy = py(p[1]);
                push(
                    Prim::Rect {
                        x: x1.min(x2),
                        y: yy.min(y0),
                        w: (x2 - x1).abs(),
                        h: (y0 - yy).abs(),
                        fill: Some((color, 1.0)),
                        stroke: Some((INK, 0.8)),
                    },
                    true,
                );
            }
            continue;
        }
        if st.kind.has_line() && s.points.len() > 1 {
            push(
                Prim::Polyline {
                    pts: s.points.iter().map(|p| [px(p[0]), py(p[1])]).collect(),
                    color,
                    width: g.line_width as f64,
                },
                true,
            );
        }
        if st.kind.has_symbol() {
            let pts: Vec<[f64; 2]> = s
                .points
                .iter()
                .map(|p| [px(p[0]), py(p[1])])
                .filter(|p| inside_x(p[0]) && inside_y(p[1]))
                .collect();
            symbols.push(Prim::Symbols {
                sym: st.symbol,
                pts,
                r: g.symbol_size as f64 / 2.0,
                color,
            });
        }
    }
    for s in symbols {
        push(s, false);
    }

    // 坐标框：四边闭合
    push(
        Prim::Rect {
            x: left,
            y: top,
            w: pw,
            h: ph,
            fill: None,
            stroke: Some((INK, AXIS_WIDTH)),
        },
        false,
    );

    // 刻度：四边都有，朝内；标签只在下边和左边
    let mut ticks = Vec::new();
    for t in &xa.ticks {
        let x = px(*t);
        if !inside_x(x) {
            continue;
        }
        ticks.push([x, top + ph, x, top + ph - MAJOR_TICK]);
        ticks.push([x, top, x, top + MAJOR_TICK]);
        push(
            text(
                x,
                top + ph + 6.0 + TICK_FONT / 2.0,
                format_tick(*t),
                TICK_FONT,
                Anchor::Middle,
            ),
            false,
        );
    }
    for t in &xa.minor {
        let x = px(*t);
        if x > left && x < left + pw {
            ticks.push([x, top + ph, x, top + ph - MINOR_TICK]);
            ticks.push([x, top, x, top + MINOR_TICK]);
        }
    }
    for t in &ya.ticks {
        let y = py(*t);
        if !inside_y(y) {
            continue;
        }
        ticks.push([left, y, left + MAJOR_TICK, y]);
        ticks.push([left + pw, y, left + pw - MAJOR_TICK, y]);
        push(text(left - 6.0, y, format_tick(*t), TICK_FONT, Anchor::End), false);
    }
    for t in &ya.minor {
        let y = py(*t);
        if y > top && y < top + ph {
            ticks.push([left, y, left + MINOR_TICK, y]);
            ticks.push([left + pw, y, left + pw - MINOR_TICK, y]);
        }
    }
    push(
        Prim::Segments {
            segs: ticks,
            color: INK,
            width: AXIS_WIDTH,
            dashed: false,
        },
        false,
    );

    // 轴标题与图标题
    push(
        text(
            left + pw / 2.0,
            top + ph + 6.0 + TICK_FONT + 6.0 + LABEL_FONT / 2.0,
            g.x_axis_title(book),
            LABEL_FONT,
            Anchor::Middle,
        ),
        false,
    );
    push(
        Prim::Text {
            x: left - 6.0 - ylab_w - 8.0 - LABEL_FONT / 2.0,
            y: top + ph / 2.0,
            text: g.y_axis_title(book),
            size: LABEL_FONT,
            anchor: Anchor::Middle,
            vertical: true,
            bold: false,
            color: INK,
        },
        false,
    );
    if has_title {
        push(
            Prim::Text {
                x: left + pw / 2.0,
                y: top / 2.0,
                text: g.title.clone(),
                size: TITLE_FONT,
                anchor: Anchor::Middle,
                vertical: false,
                bold: true,
                color: INK,
            },
            false,
        );
    }

    // 图例：坐标框内，白底无边框
    if g.legend && !series.is_empty() {
        let lh = LEGEND_FONT + 6.0;
        let sw = 26.0;
        let lw = series
            .iter()
            .map(|s| text_width(&s.name, LEGEND_FONT))
            .fold(0.0, f64::max)
            + sw
            + 16.0;
        let lhh = series.len() as f64 * lh + 4.0;
        // 自动放在四个角里数据最少的一个，尽量不遮挡曲线
        let pad_x = MAJOR_TICK + 6.0;
        let pad_y = MAJOR_TICK + 4.0;
        let corners = [
            (left + pad_x, top + pad_y),
            (left + pw - pad_x - lw, top + pad_y),
            (left + pw - pad_x - lw, top + ph - pad_y - lhh),
            (left + pad_x, top + ph - pad_y - lhh),
        ];
        let mut probe: Vec<[f64; 2]> = Vec::new();
        for (s, st) in series.iter().zip(&styles) {
            let pts: Vec<[f64; 2]> = s.points.iter().map(|p| [px(p[0]), py(p[1])]).collect();
            probe.extend(&pts);
            if st.kind.has_line() {
                for w2 in pts.windows(2) {
                    for k in 1..8 {
                        let f = k as f64 / 8.0;
                        probe.push([
                            w2[0][0] + (w2[1][0] - w2[0][0]) * f,
                            w2[0][1] + (w2[1][1] - w2[0][1]) * f,
                        ]);
                    }
                }
            }
        }
        let score = |(x, y): (f64, f64)| {
            probe
                .iter()
                .filter(|q| q[0] >= x - 4.0 && q[0] <= x + lw + 4.0 && q[1] >= y - 4.0 && q[1] <= y + lhh + 4.0)
                .count()
        };
        let (lx, ly) = corners.iter().copied().min_by_key(|c| score(*c)).unwrap_or(corners[0]);
        push(
            Prim::Rect {
                x: lx,
                y: ly,
                w: lw,
                h: lhh,
                fill: Some((PAPER, 1.0)),
                stroke: None,
            },
            false,
        );
        for (i, (s, st)) in series.iter().zip(&styles).enumerate() {
            let color = st.color.rgb();
            let cy = ly + 2.0 + lh * i as f64 + lh / 2.0;
            let x0 = lx + 4.0;
            if st.kind == PlotType::Bar {
                push(
                    Prim::Rect {
                        x: x0 + 5.0,
                        y: cy - 5.5,
                        w: sw - 10.0,
                        h: 11.0,
                        fill: Some((color, 1.0)),
                        stroke: Some((INK, 0.8)),
                    },
                    false,
                );
            } else {
                if st.kind.has_line() {
                    push(
                        Prim::Polyline {
                            pts: vec![[x0, cy], [x0 + sw, cy]],
                            color,
                            width: g.line_width as f64,
                        },
                        false,
                    );
                }
                if st.kind.has_symbol() {
                    push(
                        Prim::Symbols {
                            sym: st.symbol,
                            pts: vec![[x0 + sw / 2.0, cy]],
                            r: g.symbol_size as f64 / 2.0,
                            color,
                        },
                        false,
                    );
                }
            }
            push(
                text(x0 + sw + 6.0, cy, s.name.clone(), LEGEND_FONT, Anchor::Start),
                false,
            );
        }
    }

    Scene {
        w,
        h,
        plot: [left, top, pw, ph],
        xa,
        ya,
        items,
        layer_plots: vec![[left, top, pw, ph]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Project;

    #[test]
    fn inward_ticks_and_closed_frame() {
        let p = Project::sample();
        let sc = build(p.graph(2).unwrap(), p.book(1), 800.0, 600.0, None);
        let [l, t, w, h] = sc.plot;
        // 坐标框是闭合矩形
        assert!(sc.items.iter().any(|i| matches!(i.prim,
            Prim::Rect { x, y, w: rw, h: rh, fill: None, stroke: Some(_) } if x == l && y == t && rw == w && rh == h)));
        // 所有刻度线都在坐标框以内（朝内）
        let segs: Vec<[f64; 4]> = sc
            .items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Segments {
                    segs, dashed: false, ..
                } => Some(segs.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        assert!(!segs.is_empty());
        for s in segs {
            for (x, y) in [(s[0], s[1]), (s[2], s[3])] {
                assert!(x >= l - 1e-9 && x <= l + w + 1e-9 && y >= t - 1e-9 && y <= t + h + 1e-9);
            }
        }
    }

    #[test]
    fn view_roundtrip() {
        let p = Project::sample();
        let g = p.graph(2).unwrap();
        let v = View {
            x: (2.0, 8.0),
            y: (-1.0, 3.0),
        };
        let sc = build(g, p.book(1), 640.0, 480.0, Some(v));
        assert_eq!(sc.view(), v);
        let (x, y) = sc.data_at(sc.px(5.0), sc.py(1.5));
        assert!((x - 5.0).abs() < 1e-9 && (y - 1.5).abs() < 1e-9);
    }

    #[test]
    fn symbols_outside_view_are_dropped() {
        let p = Project::sample();
        let g = p.graph(2).unwrap();
        let v = View {
            x: (2.0, 4.0),
            y: (-100.0, 100.0),
        };
        let sc = build(g, p.book(1), 640.0, 480.0, Some(v));
        for i in &sc.items {
            if let Prim::Symbols { pts, .. } = &i.prim {
                for q in pts {
                    let [l, t, w, h] = sc.plot;
                    assert!(q[0] >= l - 0.6 && q[0] <= l + w + 0.6 && q[1] >= t - 0.6 && q[1] <= t + h + 0.6);
                }
            }
        }
    }
}
