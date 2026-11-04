//! 图形版面：把一张图排成像素坐标下的图元列表。
//!
//! 界面上的绘制和 SVG 导出都从这里取图元，所以屏幕上看到的和导出的论文插图完全一致。
//! 默认是 Nature 图稿风格：白底、只保留左轴和下轴、刻度朝外、不画网格、
//! 无边框图例自动避开数据；按 96 单位 = 1 英寸，常用页宽 89 mm 时文字为 6–7 pt。

use crate::coord::{Cartesian, Coord};
use crate::model::{Book, Graph, PlotType, Symbol};
use crate::page::Layer;
use crate::plotspec::BookLookup;
use crate::ticks::{axis, Axis, TickLabel};
mod color_scale;
mod layout;
mod legend;

pub type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyledSymbol {
    pub at: [f64; 2],
    pub symbol: Symbol,
    pub radius: f64,
    pub fill: Option<Rgb>,
    pub stroke: Option<(Rgb, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColoredSegment {
    pub points: [f64; 4],
    pub color: Rgb,
}

pub const INK: Rgb = (0, 0, 0);
pub const PAPER: Rgb = (255, 255, 255);
const GRID: Rgb = (217, 217, 217);
const MUTED: Rgb = (136, 136, 136);

/// 坐标轴线宽、主刻度 / 次刻度长度（逻辑单位，1 pt = 4/3 单位）：0.75 pt 轴线，刻度朝外。
pub const AXIS_WIDTH: f64 = 1.0;
pub const MAJOR_TICK: f64 = 4.0;
pub const MINOR_TICK: f64 = 2.3;
/// 刻度标签与轴标题的间距。
const TICK_GAP: f64 = 3.0;
/// 刻度标签、轴标题、图例、图标题的字号：约 6.4 pt 与 7 pt，落在 Nature 要求的 5–7 pt 内。
pub const TICK_FONT: f64 = 8.5;
pub const LABEL_FONT: f64 = 9.33;
pub const LEGEND_FONT: f64 = 8.5;
pub const TITLE_FONT: f64 = 9.33;

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
    Polyline {
        pts: Vec<[f64; 2]>,
        color: Rgb,
        width: f64,
    },
    /// 每三个顶点组成一个填充三角形，屏幕与 SVG 使用同一份剖分。
    Triangles {
        pts: Vec<[f64; 2]>,
        color: Rgb,
        opacity: f64,
    },
    /// 一组数据符号，r 为半径。
    Symbols {
        sym: Symbol,
        pts: Vec<[f64; 2]>,
        r: f64,
        color: Rgb,
    },
    StyledSymbols {
        symbols: Vec<StyledSymbol>,
    },
    ColoredSegments {
        segments: Vec<ColoredSegment>,
        width: f64,
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
        rich: bool,
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
    pub axes: crate::coord::AxisSet,
    pub items: Vec<Item>,
    pub layer_plots: Vec<[f64; 4]>,
    pub layers: Vec<LayerView>,
    pub objects: Vec<ObjectLayout>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObjectTarget {
    Text(u64),
    Legend(usize),
    ColorScale(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLayout {
    pub target: ObjectTarget,
    pub bounds: [f64; 4],
    pub items: std::ops::Range<usize>,
    pub overlay: bool,
}
fn contains(rect: [f64; 4], x: f64, y: f64) -> bool {
    let [l, t, w, h] = rect;
    x >= l && x <= l + w && y >= t && y <= t + h
}

/// 鼠标吸附到的数据点：曲线名、原始行（统计变换后为 None）、数据值和页面位置。
#[derive(Clone, Debug, PartialEq)]
pub struct PointHit {
    pub series: String,
    /// 曲线键（图层 y_keys 里的项）。
    pub key: String,
    pub row: Option<usize>,
    pub x: f64,
    pub y: f64,
    pub at: [f64; 2],
}

/// 页面坐标 (x, y) 附近 `radius` 以内最近的数据点（不含拟合等结果曲线）。
/// 堆叠柱的 Y 取该段自身的值；水平条形图按交换后的轴定位。
pub fn nearest_point<'a>(
    layer: &Layer,
    view: &LayerView,
    books: BookLookup<'a, '_>,
    x: f64,
    y: f64,
    radius: f64,
) -> Option<PointHit> {
    let (_, plots) = layer.prepared_plots_in(books(layer.book_id?)?, books);
    let horizontal = crate::presets::recipe(layer.plot_type).orientation == crate::coord::Orientation::Horizontal;
    let identity = (layer.transform == crate::plotspec::Transform::Identity
        || matches!(layer.plot_type, PlotType::ColumnScatter | PlotType::LineSeries))
        && !matches!(layer.plot_type, PlotType::QQ | PlotType::Histogram);
    let categories: Vec<_> = plots
        .iter()
        .filter(|p| !p.series.line_only)
        .flat_map(|p| p.series.points.iter().map(|point| point[usize::from(horizontal)]))
        .collect();
    let spacing = min_spacing(&categories);
    let mut best: Option<(f64, PointHit)> = None;
    for plot in plots.iter().filter(|p| !p.series.line_only) {
        let s = &plot.series;
        let map = view.axes.cartesian(view.plot, plot.style.y_axis);
        let offset = if plot.style.kind.is_bar() {
            crate::presets::recipe(plot.style.kind)
                .position
                .offset(spacing, plot.style.bar_index, plot.style.bar_count)
        } else {
            0.0
        };
        for (i, point) in s.points.iter().enumerate() {
            let mut positioned = *point;
            positioned[usize::from(horizontal)] += offset;
            let at = map.project(positioned);
            if !at.iter().all(|v| v.is_finite()) || !view.contains(at[0], at[1]) {
                continue;
            }
            let d = (at[0] - x).hypot(at[1] - y);
            if d <= radius && best.as_ref().is_none_or(|(b, _)| d < *b) {
                let value = point[usize::from(!horizontal)]
                    - plot.offset[usize::from(!horizontal)]
                    - if layer.plot_type.is_stacked() { s.bases[i] } else { 0.0 };
                best = Some((
                    d,
                    PointHit {
                        series: s.name.clone(),
                        key: plot.plot.key.clone(),
                        row: identity.then_some(s.rows[i]),
                        x: point[usize::from(horizontal)] - plot.offset[usize::from(horizontal)],
                        y: value,
                        at,
                    },
                ));
            }
        }
    }
    best.map(|(_, hit)| hit)
}

/// 各图层的版面与坐标轴，用于定位鼠标和独立视图交互。
#[derive(Clone, Debug, PartialEq)]
pub struct LayerView {
    pub bounds: [f64; 4],
    pub plot: [f64; 4],
    pub axes: crate::coord::AxisSet,
    pub opaque: bool,
}
impl LayerView {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let [l, t, w, h] = self.plot;
        x >= l && x <= l + w && y >= t && y <= t + h
    }
    pub fn data_at(&self, x: f64, y: f64) -> (f64, f64) {
        let [x, y] = Cartesian {
            x: &self.axes.x,
            y: &self.axes.left_y,
            rect: self.plot,
        }
        .unproject([x, y]);
        (x, y)
    }
    pub fn view(&self) -> View {
        View {
            x: (self.axes.x.lo, self.axes.x.hi),
            y: (self.axes.left_y.lo, self.axes.left_y.hi),
        }
    }
}

impl Scene {
    pub fn object_at(&self, x: f64, y: f64) -> Option<ObjectTarget> {
        self.objects
            .iter()
            .rev()
            .find(|object| {
                if !contains(object.bounds, x, y) {
                    return false;
                }
                if let ObjectTarget::Legend(index) = object.target {
                    if !object.overlay
                        && self.layers[index + 1..]
                            .iter()
                            .any(|l| l.opaque && contains(l.bounds, x, y))
                    {
                        return false;
                    }
                }
                true
            })
            .map(|o| o.target)
    }
    /// 返回最上方图层；不穿透上层的背景与轴标签区域。
    pub fn layer_at(&self, x: f64, y: f64) -> Option<usize> {
        self.layers.iter().rposition(|layer| {
            let [l, t, w, h] = layer.bounds;
            x >= l && x <= l + w && y >= t && y <= t + h
        })
    }
    pub fn px(&self, v: f64) -> f64 {
        self.coord().project([v, self.axes.left_y.lo])[0]
    }

    pub fn py(&self, v: f64) -> f64 {
        self.coord().project([self.axes.x.lo, v])[1]
    }

    /// 像素坐标换算回数据坐标。
    pub fn data_at(&self, x: f64, y: f64) -> (f64, f64) {
        let [x, y] = self.coord().unproject([x, y]);
        (x, y)
    }

    fn coord(&self) -> Cartesian<'_> {
        self.axes.cartesian(self.plot, crate::coord::YAxis::Left)
    }

    /// 当前坐标范围，作为缩放平移的起点。
    pub fn view(&self) -> View {
        View {
            x: (self.axes.x.lo, self.axes.x.hi),
            y: (self.axes.left_y.lo, self.axes.left_y.hi),
        }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        let [l, t, w, h] = self.plot;
        x >= l && x <= l + w && y >= t && y <= t + h
    }
}

/// Width in fixed page/CSS pixels, measured from the same fonts as screen/SVG/PNG.
pub fn text_width(s: &str, px: f64) -> f64 {
    crate::text::layout(s, px, false).width
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
        rich: false,
        color: INK,
    }
}

fn formatted_text(x: f64, y: f64, s: impl Into<String>, size: f64, anchor: Anchor, rich: bool) -> Prim {
    let mut prim = text(x, y, s, size, anchor);
    if let Prim::Text { rich: value, .. } = &mut prim {
        *value = rich;
    }
    prim
}

/// 排版整张图。`book` 为 None 时是一张提示“没有关联数据”的空图。
pub fn build(g: &Graph, book: Option<&Book>, w: f64, h: f64, view: Option<View>) -> Scene {
    layout::compose(g, |id| book.filter(|b| b.id == id), w, h, &[view])
}

/// 页面各图层独立解析工作簿；屏幕与整页导出使用同一入口。
pub fn build_page(g: &Graph, project: &crate::Project, w: f64, h: f64, view: Option<View>) -> Scene {
    layout::compose(g, |id| project.book(id), w, h, &[view])
}

pub fn build_page_views(g: &Graph, project: &crate::Project, w: f64, h: f64, views: &[Option<View>]) -> Scene {
    layout::compose(g, |id| project.book(id), w, h, views)
}

/// 屏幕、图库与出版导出共用的页面排版；窗口大小只影响显示缩放。
pub fn layout_page(g: &Graph, project: &crate::Project, views: &[Option<View>]) -> Scene {
    let [w, h] = g.page.logical_size();
    build_page_views(g, project, w, h, views)
}

/// 端点上有数据点时把线性轴的这一端外扩 4%（刻度不变）。
fn pad_ends(a: &mut Axis, vals: impl Iterator<Item = f64>) {
    let span = a.hi - a.lo;
    if a.log || !span.is_finite() || span <= 0.0 {
        return;
    }
    let (mut lo, mut hi) = (false, false);
    for v in vals {
        lo |= (v - a.lo).abs() <= span * 1e-9;
        hi |= (v - a.hi).abs() <= span * 1e-9;
    }
    if lo {
        a.lo -= span * 0.04;
    }
    if hi {
        a.hi += span * 0.04;
    }
}

/// 整体放大一个图元：坐标、尺寸、字号、线宽、符号半径同乘 k。
fn scale_prim(prim: &mut Prim, k: f64) {
    if k == 1.0 {
        return;
    }
    let scale_pts = |pts: &mut Vec<[f64; 2]>| pts.iter_mut().for_each(|p| *p = p.map(|v| v * k));
    match prim {
        Prim::Rect { x, y, w, h, stroke, .. } => {
            (*x, *y, *w, *h) = (*x * k, *y * k, *w * k, *h * k);
            if let Some((_, width)) = stroke {
                *width *= k;
            }
        }
        Prim::Text { x, y, size, .. } => (*x, *y, *size) = (*x * k, *y * k, *size * k),
        Prim::Polyline { pts, width, .. } => {
            scale_pts(pts);
            *width *= k;
        }
        Prim::Triangles { pts, .. } => scale_pts(pts),
        Prim::Symbols { pts, r, .. } => {
            scale_pts(pts);
            *r *= k;
        }
        Prim::StyledSymbols { symbols } => {
            for symbol in symbols {
                symbol.at = symbol.at.map(|value| value * k);
                symbol.radius *= k;
                if let Some((_, width)) = &mut symbol.stroke {
                    *width *= k;
                }
            }
        }
        Prim::ColoredSegments { segments, width } => {
            for segment in segments {
                segment.points = segment.points.map(|value| value * k);
            }
            *width *= k;
        }
        Prim::Segments { segs, width, .. } => {
            segs.iter_mut().for_each(|s| *s = s.map(|v| v * k));
            *width *= k;
        }
    }
}

fn translate(prim: &mut Prim, dx: f64, dy: f64) {
    match prim {
        Prim::Rect { x, y, .. } | Prim::Text { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        Prim::Polyline { pts, .. } | Prim::Symbols { pts, .. } | Prim::Triangles { pts, .. } => {
            for p in pts {
                p[0] += dx;
                p[1] += dy;
            }
        }
        Prim::StyledSymbols { symbols } => {
            for symbol in symbols {
                symbol.at[0] += dx;
                symbol.at[1] += dy;
            }
        }
        Prim::ColoredSegments { segments, .. } => {
            for segment in segments {
                segment.points[0] += dx;
                segment.points[1] += dy;
                segment.points[2] += dx;
                segment.points[3] += dy;
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

/// 图例要避开的数据几何（页面像素坐标）。
#[derive(Clone, Copy, Debug)]
enum Obstacle {
    /// [左, 上, 右, 下]：柱子、标签文字。
    Box([f64; 4]),
    /// 线段及其半线宽：折线、阶梯线、误差棒、垂线。
    Seg([f64; 4], f64),
    Tri([[f64; 2]; 3]),
    /// 圆心与半径：数据符号。
    Dot([f64; 2], f64),
}

impl Obstacle {
    fn bounds(&self) -> [f64; 4] {
        match *self {
            Obstacle::Box(b) => b,
            Obstacle::Seg([x1, y1, x2, y2], hw) => [x1.min(x2) - hw, y1.min(y2) - hw, x1.max(x2) + hw, y1.max(y2) + hw],
            Obstacle::Tri([a, b, c]) => [
                a[0].min(b[0]).min(c[0]),
                a[1].min(b[1]).min(c[1]),
                a[0].max(b[0]).max(c[0]),
                a[1].max(b[1]).max(c[1]),
            ],
            Obstacle::Dot([x, y], r) => [x - r, y - r, x + r, y + r],
        }
    }

    /// 点 q 是否落在几何上，或离它不超过 tol。
    fn hits(&self, q: [f64; 2], tol: f64) -> bool {
        match *self {
            Obstacle::Box([l, t, r, b]) => q[0] >= l - tol && q[0] <= r + tol && q[1] >= t - tol && q[1] <= b + tol,
            Obstacle::Seg([x1, y1, x2, y2], hw) => seg_dist(q, [x1, y1], [x2, y2]) <= hw + tol,
            Obstacle::Tri([a, b, c]) => {
                let side = |p: [f64; 2], u: [f64; 2]| (u[0] - p[0]) * (q[1] - p[1]) - (u[1] - p[1]) * (q[0] - p[0]);
                let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
                let inside = !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0));
                inside || seg_dist(q, a, b) <= tol || seg_dist(q, b, c) <= tol || seg_dist(q, c, a) <= tol
            }
            Obstacle::Dot(c, r) => (q[0] - c[0]).hypot(q[1] - c[1]) <= r + tol,
        }
    }
}

fn seg_dist(q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (q[0] - a[0] - t * dx).hypot(q[1] - a[1] - t * dy)
}

/// 把已排好的数据图元拆成图例避让用的几何。
fn obstacles(items: &[Item]) -> Vec<Obstacle> {
    let mut out = Vec::new();
    for item in items {
        match &item.prim {
            Prim::Rect { x, y, w, h, .. } => out.push(Obstacle::Box([*x, *y, x + w, y + h])),
            Prim::Segments { segs, width, .. } => out.extend(segs.iter().map(|s| Obstacle::Seg(*s, width / 2.0))),
            Prim::Polyline { pts, width, .. } => out.extend(
                pts.windows(2)
                    .map(|p| Obstacle::Seg([p[0][0], p[0][1], p[1][0], p[1][1]], width / 2.0)),
            ),
            Prim::Triangles { pts, .. } => out.extend(pts.as_chunks::<3>().0.iter().map(|t| Obstacle::Tri(*t))),
            // 多边形符号最远伸到 1.3 倍半径
            Prim::Symbols { pts, r, .. } => out.extend(pts.iter().map(|p| Obstacle::Dot(*p, r * 1.3))),
            Prim::StyledSymbols { symbols } => out.extend(symbols.iter().map(|s| Obstacle::Dot(s.at, s.radius * 1.3))),
            Prim::ColoredSegments { segments, width } => {
                out.extend(segments.iter().map(|s| Obstacle::Seg(s.points, width / 2.0)))
            }
            Prim::Text {
                x,
                y,
                text,
                size,
                anchor,
                vertical,
                bold,
                rich,
                ..
            } => {
                let measured = crate::text::layout_formatted(text, *size, *bold, *rich);
                let shift = match anchor {
                    Anchor::Start => 0.0,
                    Anchor::Middle => measured.width / 2.0,
                    Anchor::End => measured.width,
                };
                let bounds = if *vertical {
                    [
                        *x - measured.height / 2.0,
                        *y + shift - measured.width,
                        *x + measured.height / 2.0,
                        *y + shift,
                    ]
                } else {
                    [
                        *x - shift,
                        *y - measured.height / 2.0,
                        *x - shift + measured.width,
                        *y + measured.height / 2.0,
                    ]
                };
                out.push(Obstacle::Box(bounds));
            }
        }
    }
    out
}

/// 在矩形 [左, 上, 宽, 高] 内按网格采样，返回落在数据几何上的采样点数（近似重叠面积）。
fn coverage(obstacles: &[Obstacle], [x, y, w, h]: [f64; 4]) -> usize {
    const STEP: f64 = 2.0;
    // 半个网格对角线：任何穿过网格的细线、细柱都至少碰到一个采样点
    let tol = STEP * std::f64::consts::FRAC_1_SQRT_2;
    let near: Vec<&Obstacle> = obstacles
        .iter()
        .filter(|o| {
            let [l, t, r, b] = o.bounds();
            r + tol >= x && l - tol <= x + w && b + tol >= y && t - tol <= y + h
        })
        .collect();
    if near.is_empty() {
        return 0;
    }
    let (nx, ny) = ((w / STEP).ceil() as usize, (h / STEP).ceil() as usize);
    let mut count = 0;
    for i in 0..=nx {
        for j in 0..=ny {
            let q = [x + w * i as f64 / nx.max(1) as f64, y + h * j as f64 / ny.max(1) as f64];
            if near.iter().any(|o| o.hits(q, tol)) {
                count += 1;
            }
        }
    }
    count
}

fn build_layer<'a>(
    g: &Layer,
    book: Option<&'a Book>,
    others: BookLookup<'a, '_>,
    w: f64,
    h: f64,
    view: Option<View>,
) -> Scene {
    build_layer_min(g, book, others, w, h, view, [0.0; 4])
}

/// `min` 为绘图区四周的最小边距 [左, 上, 右, 下]，用于多面板对齐。
fn build_layer_min<'a>(
    g: &Layer,
    book: Option<&'a Book>,
    others: BookLookup<'a, '_>,
    w: f64,
    h: f64,
    view: Option<View>,
    min: [f64; 4],
) -> Scene {
    build_layer_full(LayerContext {
        layer: g,
        book,
        others,
        w,
        h,
        view,
        min,
        legend_outside: false,
    })
}

/// `legend_outside` 为真时，图例放在坐标框右侧（右边距已按图例宽度留出）。
#[derive(Clone, Copy)]
struct LayerContext<'g, 'a, 'b> {
    layer: &'g Layer,
    book: Option<&'a Book>,
    others: BookLookup<'a, 'b>,
    w: f64,
    h: f64,
    view: Option<View>,
    min: [f64; 4],
    legend_outside: bool,
}
struct LayerData {
    plots: Vec<crate::page::PreparedPlot>,
    all_x: Vec<f64>,
    all_y: Vec<f64>,
    all_y2: Vec<f64>,
    horizontal: bool,
    is_bar: bool,
    x_date: bool,
    date_offset: f64,
    categories: Option<Vec<String>>,
    dx: f64,
}
fn prepare_layer_data(ctx: &LayerContext<'_, '_, '_>) -> LayerData {
    let g = ctx.layer;
    let book = ctx.book;
    let others = ctx.others;
    let recipe = crate::presets::recipe(g.plot_type);
    let horizontal = recipe.orientation == crate::coord::Orientation::Horizontal;
    let (x_col, mut plots) = match book {
        Some(b) => g.prepared_plots_in(b, others),
        None => (None, Vec::new()),
    };
    // 统计图的横轴由 Y 样本生成，不使用 X 列的日期或类别格式。
    let x_col = x_col.filter(|_| {
        !matches!(
            g.plot_type,
            PlotType::Histogram | PlotType::QQ | PlotType::Box | PlotType::ColumnScatter | PlotType::LineSeries
        )
    });
    let x_date = !horizontal && !g.x_log && x_col.is_some_and(|c| c.data.kind() == crate::DataKind::DateTime);
    let date_offset = x_col.map_or(0.0, |c| f64::from(c.format.utc_offset) * 60_000.0);
    let categories = if matches!(
        g.plot_type,
        PlotType::Box | PlotType::ColumnScatter | PlotType::LineSeries
    ) {
        Some(
            plots
                .iter()
                .filter(|p| !p.series.line_only)
                .map(|p| p.series.name.clone())
                .collect(),
        )
    } else {
        x_col.and_then(|c| c.data.plot_category_levels())
    };
    for plot in &mut plots {
        if plot.style.kind.is_spline() && !plot.series.line_only {
            plot.paths = crate::stat::spline_paths(&plot.series);
        }
    }
    let mut all_x: Vec<f64> = plots
        .iter()
        .map(|p| &p.series)
        .flat_map(|s| s.points.iter().map(|p| p[0]))
        .collect();
    // 右侧 Y 轴的曲线单独定范围
    let mut all_y: Vec<f64> = plots
        .iter()
        .map(|p| (&p.series, &p.style))
        .filter(|(_, st)| st.y_axis == crate::coord::YAxis::Left)
        .flat_map(|(s, _)| s.points.iter().map(|p| p[1]))
        .collect();
    let mut all_y2: Vec<f64> = plots
        .iter()
        .map(|p| (&p.series, &p.style))
        .filter(|(_, st)| st.y_axis == crate::coord::YAxis::Right)
        .flat_map(|(s, _)| s.points.iter().map(|p| p[1]))
        .collect();
    for (paths, st) in plots.iter().map(|p| (&p.paths, &p.style)) {
        let target = if st.y_axis == crate::coord::YAxis::Right {
            &mut all_y2
        } else {
            &mut all_y
        };
        target.extend(paths.iter().flatten().map(|p| p[1]).filter(|v| !g.y_log || *v > 0.0));
    }
    if (g.plot_type.is_bar() && recipe.position != crate::position::Position::Dodge)
        || matches!(g.plot_type, PlotType::StackedArea | PlotType::FillBetween)
    {
        for p in plots
            .iter()
            .filter(|p| !p.series.line_only && (g.plot_type != PlotType::FillBetween || p.plot.mapping.base.is_some()))
        {
            let (target, log) = if horizontal {
                (&mut all_x, g.x_log)
            } else if p.style.y_axis == crate::coord::YAxis::Right {
                (&mut all_y2, g.y_log)
            } else {
                (&mut all_y, g.y_log)
            };
            target.extend(p.series.bases.iter().copied().filter(|v| !log || *v > 0.0));
        }
    }
    if g.error_bars.visible {
        for (s, st) in plots.iter().map(|p| (&p.series, &p.style)) {
            for (point, errors) in s.points.iter().zip(&s.errors) {
                let ys = if st.y_axis == crate::coord::YAxis::Right {
                    &mut all_y2
                } else {
                    &mut all_y
                };
                for (dim, values, log) in [(0, &mut all_x, g.x_log), (1, ys, g.y_log)] {
                    if let Some(error) = errors[dim] {
                        values.extend(
                            [point[dim] - error, point[dim] + error]
                                .into_iter()
                                .filter(|v| v.is_finite() && (!log || *v > 0.0)),
                        );
                    }
                }
            }
        }
    }
    let is_bar = g.plot_type.is_bar() || plots.iter().map(|p| &p.style).any(|st| st.kind.is_bar());
    // 柱宽只按数据曲线的 X 间距算（拟合曲线是 200 个密集点）
    let data_x: Vec<f64> = plots
        .iter()
        .map(|p| &p.series)
        .filter(|s| !s.line_only)
        .flat_map(|s| s.points.iter().map(|p| p[usize::from(horizontal)]))
        .collect();
    // 单个区间没有相邻中点可推断宽度，使用分箱时保留的实际宽度。
    let dx = plots
        .iter()
        .filter(|p| p.series.points.len() == 1)
        .find_map(|p| p.histogram_width)
        .unwrap_or_else(|| min_spacing(&data_x));

    LayerData {
        plots,
        all_x,
        all_y,
        all_y2,
        horizontal,
        is_bar,
        x_date,
        date_offset,
        categories,
        dx,
    }
}

struct AxisRanges {
    xa: Axis,
    ya: Axis,
    ya2: Option<Axis>,
    x_label: TickLabel,
    y_label: TickLabel,
    y2_label: TickLabel,
    ylab2_w: f64,
}
fn axis_ranges(ctx: &LayerContext<'_, '_, '_>, data: &LayerData) -> AxisRanges {
    let g = ctx.layer;
    let view = ctx.view;
    let plots = &data.plots;
    let all_x = &data.all_x;
    let all_y = &data.all_y;
    let all_y2 = &data.all_y2;
    let horizontal = data.horizontal;
    let is_bar = data.is_bar;
    let dx = data.dx;
    let x_date = data.x_date;
    let categories = &data.categories;
    let (xa, ya): (Axis, Axis) = match view {
        Some(v) => (
            axis(&[], false, v.x.0, v.x.1, g.x_log, false),
            axis(&[], false, v.y.0, v.y.1, g.y_log, false),
        ),
        None => {
            let xa = if is_bar && !horizontal && g.x_auto && !g.x_log && !all_x.is_empty() {
                let lo = all_x.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = all_x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                axis(&[lo - dx * 0.6, hi + dx * 0.6], true, 0.0, 0.0, false, false)
            } else {
                axis(all_x, g.x_auto, g.x_min, g.x_max, g.x_log, horizontal)
            };
            let ya = if horizontal && g.y_auto && !g.y_log && !all_y.is_empty() {
                let lo = all_y.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = all_y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                axis(&[lo - dx * 0.6, hi + dx * 0.6], true, 0.0, 0.0, false, false)
            } else {
                axis(
                    all_y,
                    g.y_auto,
                    g.y_min,
                    g.y_max,
                    g.y_log,
                    (is_bar && !horizontal && g.plot_type != PlotType::FloatingBar)
                        || matches!(g.plot_type, PlotType::Area | PlotType::VerticalDrop),
                )
            };
            let (mut xa, mut ya) = (xa, ya);
            // 带符号的曲线：数据点正好落在自动范围的端点时，这一端向外留 4% 边距，
            // 免得符号压在坐标轴上；刻度保持不变，手动范围和对数轴不动
            let marks: Vec<[f64; 2]> = plots
                .iter()
                .map(|p| (&p.series, &p.style))
                .filter(|(s, st)| {
                    !s.line_only
                        && (st.kind.has_symbol()
                            || matches!(st.kind, PlotType::Box | PlotType::ColumnScatter | PlotType::LineSeries))
                        && st.y_axis == crate::coord::YAxis::Left
                })
                .flat_map(|(s, _)| s.points.iter().copied())
                .collect();
            if !marks.is_empty() {
                if g.x_auto {
                    pad_ends(&mut xa, marks.iter().map(|p| p[0]));
                }
                if g.y_auto {
                    pad_ends(&mut ya, marks.iter().map(|p| p[1]));
                }
            }
            (xa, ya)
        }
    };
    let zoomed = view.is_some();
    let (xa, ya, x_cat, y_cat) = match &categories {
        // 缩放后保留视图范围，刻度仍在各类别上
        Some(names) if !horizontal => {
            let mut axis = crate::ticks::category_axis(names.len());
            if zoomed {
                axis.lo = xa.lo;
                axis.hi = xa.hi;
            }
            (axis, ya, Some(names.clone()), None)
        }
        Some(names) => {
            let mut axis = crate::ticks::category_axis(names.len());
            if zoomed {
                axis.lo = ya.lo;
                axis.hi = ya.hi;
            }
            (xa, axis, None, Some(names.clone()))
        }
        None => (xa, ya, None, None),
    };
    let (xa, x_label) = if let Some(names) = x_cat {
        (xa, TickLabel::Category(names))
    } else if x_date {
        // 在当地时间上取整刻度，再换回 UTC 坐标
        let off = data.date_offset;
        let (mut axis, unit) = match view {
            Some(v) => crate::ticks::date_axis(v.x.0 + off, v.x.1 + off, false),
            None if g.x_auto && !all_x.is_empty() => {
                let lo = all_x.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = all_x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                crate::ticks::date_axis(lo + off, hi + off, true)
            }
            None => crate::ticks::date_axis(xa.lo + off, xa.hi + off, false),
        };
        axis.lo -= off;
        axis.hi -= off;
        axis.ticks
            .iter_mut()
            .chain(axis.minor.iter_mut())
            .for_each(|t| *t -= off);
        (axis, TickLabel::Date(unit, off))
    } else {
        (xa, TickLabel::Number(g.x_format))
    };
    let y_label = match y_cat {
        Some(names) => TickLabel::Category(names),
        None => TickLabel::Number(g.y_format),
    };
    // 右侧 Y 轴独立控制范围；自动范围时，符号落在端点会留 4%
    let ya2 = (!all_y2.is_empty()).then(|| {
        let mut a = axis(all_y2, g.y2_auto, g.y2_min, g.y2_max, g.y_log, false);
        if g.y2_auto {
            let marks: Vec<f64> = plots
                .iter()
                .map(|p| (&p.series, &p.style))
                .filter(|(s, st)| {
                    !s.line_only
                        && (st.kind.has_symbol()
                            || matches!(st.kind, PlotType::Box | PlotType::ColumnScatter | PlotType::LineSeries))
                        && st.y_axis == crate::coord::YAxis::Right
                })
                .flat_map(|(s, _)| s.points.iter().map(|p| p[1]))
                .collect();
            pad_ends(&mut a, marks.into_iter());
        }
        a
    });
    let y2_label = TickLabel::Number(g.y2_format.unwrap_or(g.y_format));
    let ylab2_w = ya2.as_ref().map_or(0.0, |a| {
        a.ticks
            .iter()
            .map(|t| text_width(&y2_label.text(*t), TICK_FONT))
            .fold(8.0, f64::max)
    });

    AxisRanges {
        xa,
        ya,
        ya2,
        x_label,
        y_label,
        y2_label,
        ylab2_w,
    }
}

struct AxisLayout {
    axes: crate::coord::AxisSet,
    x_label: TickLabel,
    y_label: TickLabel,
    y2_label: TickLabel,
    x_title: String,
    y_title: String,
    y2_title: String,
    title: String,
    title_height: f64,
    has_title: bool,
    rect: [f64; 4],
    tick_in: f64,
    tick_out: f64,
    ylab_w: f64,
    ylab2_w: f64,
    tick_height: f64,
    axis_height: f64,
}
fn axis_layout(ctx: &LayerContext<'_, '_, '_>, ranges: AxisRanges) -> AxisLayout {
    let g = ctx.layer;
    let book = ctx.book;
    let others = ctx.others;
    let w = ctx.w;
    let h = ctx.h;
    let min = ctx.min;
    let AxisRanges {
        xa,
        ya,
        ya2,
        x_label,
        y_label,
        y2_label,
        ylab2_w,
    } = ranges;
    // 版面：刻度朝外，刻度标签和轴标题依次排在左轴、下轴外侧
    let ylab_w = ya
        .ticks
        .iter()
        .map(|t| text_width(&y_label.text(*t), TICK_FONT))
        .fold(8.0, f64::max);
    let tick_height = crate::text::layout("0", TICK_FONT, false).height;
    let axis_height = crate::text::layout("Ag", LABEL_FONT, false).height;
    let tick_half_width = xa
        .ticks
        .iter()
        .map(|t| text_width(&x_label.text(*t), TICK_FONT) / 2.0)
        .fold(0.0, f64::max);
    let x_title = g.x_axis_title_in(book, others);
    let y_title = g.y_axis_title_in(book, others);
    let y2_title = g.y2_axis_title_in(book, others);
    // 刻度在框外的长度：朝内时刻度标签直接贴近坐标轴
    let (fout, fin) = g.tick_dir.factors();
    let out = MAJOR_TICK * fout;
    let left = (out + TICK_GAP + ylab_w + 5.0 + axis_height + 4.0)
        .round()
        .max(min[0])
        .max(tick_half_width + 4.0);
    let right = if ya2.is_some() {
        (out + TICK_GAP + ylab2_w + 5.0 + axis_height + 4.0).round()
    } else {
        14.0
    }
    .max(min[2])
    .max(tick_half_width + 4.0);
    let title = crate::text::fonts().wrap_formatted(
        &g.title,
        TITLE_FONT,
        true,
        (w - (left - right).abs() - 12.0).max(20.0),
        g.rich_text,
    );
    let title_height = crate::text::layout_formatted(&title, TITLE_FONT, true, g.rich_text).height;
    let has_title = !title.trim().is_empty();
    let top = (if has_title { 6.0 + title_height + 8.0 } else { 10.0f64 }).max(min[1]);
    let bottom = (out + TICK_GAP + tick_height + 5.0 + axis_height + 5.0)
        .round()
        .max(min[3]);
    let color_width = crate::plotspec::color_scale_data(g, &|id| {
        if book.is_some_and(|b| b.id == id) {
            book
        } else {
            others(id)
        }
    })
    .map_or(0.0, |(_, range)| {
        0.04 * w
            + range
                .iter()
                .map(|v| text_width(&crate::fmt_num(*v), TICK_FONT))
                .fold(0.0, f64::max)
            + 22.0
    });
    let pw = (w - left - right - color_width).max(20.0);
    let ph = (h - top - bottom).max(20.0);
    AxisLayout {
        axes: crate::coord::AxisSet {
            x: xa,
            left_y: ya,
            right_y: ya2,
        },
        x_label,
        y_label,
        y2_label,
        x_title,
        y_title,
        y2_title,
        title,
        title_height,
        has_title,
        rect: [left, top, pw, ph],
        tick_in: fin,
        tick_out: fout,
        ylab_w,
        ylab2_w,
        tick_height,
        axis_height,
    }
}

fn draw_layer_data(
    ctx: &LayerContext<'_, '_, '_>,
    data: &LayerData,
    layout: &AxisLayout,
) -> (Vec<Item>, Vec<ObjectLayout>, usize) {
    let g = ctx.layer;
    let book = ctx.book;
    let others = ctx.others;
    let w = ctx.w;
    let h = ctx.h;
    let plots = &data.plots;
    let all_x = &data.all_x;
    let horizontal = data.horizontal;
    let dx = data.dx;
    let axes = &layout.axes;
    let [left, top, pw, ph] = layout.rect;
    let xa = &axes.x;
    let ya = &axes.left_y;
    let coord = axes.cartesian([left, top, pw, ph], crate::coord::YAxis::Left);
    let px = |v: f64| coord.project([v, ya.lo])[0];
    let py = |v: f64| coord.project([xa.lo, v])[1];
    let mut items: Vec<Item> = Vec::new();
    let objects = Vec::new();
    let data_start;
    {
        // 记录本图层数据图元范围，供图例避让。
        let mut push = |prim: Prim, clip: bool| {
            items.push(Item {
                prim,
                clip: clip.then_some([left, top, pw, ph]),
            });
            items.len()
        };
        let mut start = push(
            Prim::Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
                fill: (!g.transparent).then_some((PAPER, 1.0)),
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
            start = push(
                Prim::Segments {
                    segs,
                    color: GRID,
                    width: 1.0,
                    dashed: true,
                },
                true,
            );
        }
        data_start = start;

        for line in &g.reference_lines {
            let seg = match line.axis {
                crate::page::ReferenceAxis::X => {
                    if axes.x.log && line.value <= 0.0 {
                        continue;
                    }
                    let x = coord.project([line.value, ya.lo])[0];
                    [x, top, x, top + ph]
                }
                crate::page::ReferenceAxis::YLeft => {
                    if ya.log && line.value <= 0.0 {
                        continue;
                    }
                    let y = coord.project([xa.lo, line.value])[1];
                    [left, y, left + pw, y]
                }
                crate::page::ReferenceAxis::YRight => {
                    let Some(right) = axes.right_y.as_ref() else { continue };
                    if right.log && line.value <= 0.0 {
                        continue;
                    }
                    let y = axes
                        .cartesian([left, top, pw, ph], crate::coord::YAxis::Right)
                        .project([xa.lo, line.value])[1];
                    [left, y, left + pw, y]
                }
            };
            push(
                Prim::Segments {
                    segs: vec![seg],
                    color: (line.color[0], line.color[1], line.color[2]),
                    width: f64::from(line.width),
                    dashed: line.dashed,
                },
                true,
            );
        }

        // 曲线：线和柱子裁剪到坐标框内；符号只画范围内的点、不裁剪，落在坐标轴上的点保持完整。
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
                    rich: false,
                    color: MUTED,
                },
                false,
            );
        }
    }
    {
        let context = crate::mark::Context {
            layer: g,
            book,
            others,
            axes,
            rect: [left, top, pw, ph],
            horizontal,
            dx,
        };
        crate::mark::draw(&context, plots, &mut items);
    }
    (items, objects, data_start)
}

fn draw_axis_ticks(ctx: &LayerContext<'_, '_, '_>, layout: &AxisLayout, items: &mut Vec<Item>) -> usize {
    let g = ctx.layer;
    let [left, top, pw, ph] = layout.rect;
    let axes = &layout.axes;
    let xa = &axes.x;
    let ya = &axes.left_y;
    let ya2 = &axes.right_y;
    let x_label = &layout.x_label;
    let y_label = &layout.y_label;
    let (fin, fout) = (layout.tick_in, layout.tick_out);
    let out = MAJOR_TICK * fout;
    let tick_height = layout.tick_height;
    let coord = axes.cartesian([left, top, pw, ph], crate::coord::YAxis::Left);
    let px = |v: f64| coord.project([v, ya.lo])[0];
    let py = |v: f64| coord.project([xa.lo, v])[1];
    let inside_x = |x: f64| x >= left - 0.5 && x <= left + pw + 0.5;
    let inside_y = |y: f64| y >= top - 0.5 && y <= top + ph + 0.5;
    let mut push = |prim: Prim, clip: bool| {
        items.push(Item {
            prim,
            clip: clip.then_some([left, top, pw, ph]),
        });
        items.len()
    };
    // 坐标轴：默认只画左轴和下轴（Nature 风格，不闭合）；四边框时闭合
    let mut axis_pts = vec![[left, top], [left, top + ph], [left + pw, top + ph]];
    if g.frame {
        axis_pts.extend([[left + pw, top], [left, top]]);
    }
    let data_end = push(
        Prim::Polyline {
            pts: axis_pts,
            color: INK,
            width: AXIS_WIDTH,
        },
        false,
    ) - 1;

    // 刻度：左轴和下轴带标签，方向按 tick_dir；四边框时顶边和右边画镜像刻度（无标签）
    let mut ticks = Vec::new();
    let x_tick = |ticks: &mut Vec<[f64; 4]>, x: f64, len: f64| {
        ticks.push([x, top + ph - len * fin, x, top + ph + len * fout]);
        if g.frame {
            ticks.push([x, top + len * fin, x, top - len * fout]);
        }
    };
    let mut previous_right = f64::NEG_INFINITY;
    for t in &xa.ticks {
        let x = px(*t);
        if !inside_x(x) {
            continue;
        }
        x_tick(&mut ticks, x, MAJOR_TICK);
        let half = text_width(&x_label.text(*t), TICK_FONT) / 2.0;
        if x - half < previous_right + 4.0 {
            continue;
        }
        previous_right = x + half;
        push(
            text(
                x,
                top + ph + out + TICK_GAP + tick_height / 2.0,
                x_label.text(*t),
                TICK_FONT,
                Anchor::Middle,
            ),
            false,
        );
    }
    for t in &xa.minor {
        let x = px(*t);
        if x > left && x < left + pw {
            x_tick(&mut ticks, x, MINOR_TICK);
        }
    }
    let y_tick = |ticks: &mut Vec<[f64; 4]>, y: f64, len: f64| {
        ticks.push([left + len * fin, y, left - len * fout, y]);
        if g.frame && ya2.is_none() {
            ticks.push([left + pw - len * fin, y, left + pw + len * fout, y]);
        }
    };
    for t in &ya.ticks {
        let y = py(*t);
        if !inside_y(y) {
            continue;
        }
        y_tick(&mut ticks, y, MAJOR_TICK);
        push(
            text(left - out - TICK_GAP, y, y_label.text(*t), TICK_FONT, Anchor::End),
            false,
        );
    }
    for t in &ya.minor {
        let y = py(*t);
        if y > top && y < top + ph {
            y_tick(&mut ticks, y, MINOR_TICK);
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

    data_end
}

fn draw_axis_titles(ctx: &LayerContext<'_, '_, '_>, layout: &AxisLayout, items: &mut Vec<Item>) {
    let g = ctx.layer;
    let [left, top, pw, ph] = layout.rect;
    let out = MAJOR_TICK * layout.tick_out;
    let tick_height = layout.tick_height;
    let axis_height = layout.axis_height;
    let ylab_w = layout.ylab_w;
    let ylab2_w = layout.ylab2_w;
    let (x_title, y_title, y2_title) = (&layout.x_title, &layout.y_title, &layout.y2_title);
    let axes = &layout.axes;
    let ya2 = &axes.right_y;
    let y2_label = &layout.y2_label;
    let (fin, fout) = (layout.tick_in, layout.tick_out);
    let coord2 = axes.cartesian([left, top, pw, ph], crate::coord::YAxis::Right);
    let py2 = |v: f64| coord2.project([axes.x.lo, v])[1];
    let inside_y = |y: f64| y >= top - 0.5 && y <= top + ph + 0.5;
    let has_title = layout.has_title;
    let title_height = layout.title_height;
    let title = layout.title.clone();
    let mut push = |prim: Prim, clip: bool| {
        items.push(Item {
            prim,
            clip: clip.then_some([left, top, pw, ph]),
        })
    };
    // 轴标题与图标题
    push(
        formatted_text(
            left + pw / 2.0,
            top + ph + out + TICK_GAP + tick_height + 5.0 + axis_height / 2.0,
            x_title.clone(),
            crate::text::fit_size_formatted(x_title, LABEL_FONT, false, pw, g.rich_text),
            Anchor::Middle,
            g.rich_text,
        ),
        false,
    );
    // 右侧 Y 轴：轴线、刻度、标签与标题
    if let Some(a2) = &ya2 {
        let mut ticks2 = Vec::new();
        for t in &a2.ticks {
            let y = py2(*t);
            if !inside_y(y) {
                continue;
            }
            ticks2.push([left + pw - MAJOR_TICK * fin, y, left + pw + MAJOR_TICK * fout, y]);
            push(
                text(
                    left + pw + out + TICK_GAP,
                    y,
                    y2_label.text(*t),
                    TICK_FONT,
                    Anchor::Start,
                ),
                false,
            );
        }
        for t in &a2.minor {
            let y = py2(*t);
            if y > top && y < top + ph {
                ticks2.push([left + pw - MINOR_TICK * fin, y, left + pw + MINOR_TICK * fout, y]);
            }
        }
        if !g.frame {
            ticks2.push([left + pw, top, left + pw, top + ph]);
        }
        push(
            Prim::Segments {
                segs: ticks2,
                color: INK,
                width: AXIS_WIDTH,
                dashed: false,
            },
            false,
        );
        push(
            Prim::Text {
                x: left + pw + out + TICK_GAP + ylab2_w + 5.0 + LABEL_FONT / 2.0,
                y: top + ph / 2.0,
                text: y2_title.clone(),
                size: crate::text::fit_size_formatted(y2_title, LABEL_FONT, false, ph, g.rich_text),
                anchor: Anchor::Middle,
                vertical: true,
                bold: false,
                rich: g.rich_text,
                color: INK,
            },
            false,
        );
    }
    push(
        Prim::Text {
            x: left - out - TICK_GAP - ylab_w - 5.0 - LABEL_FONT / 2.0,
            y: top + ph / 2.0,
            text: y_title.clone(),
            size: crate::text::fit_size_formatted(y_title, LABEL_FONT, false, ph, g.rich_text),
            anchor: Anchor::Middle,
            vertical: true,
            bold: false,
            rich: g.rich_text,
            color: INK,
        },
        false,
    );
    if has_title {
        push(
            Prim::Text {
                x: left + pw / 2.0,
                y: 6.0 + title_height / 2.0,
                text: title,
                size: TITLE_FONT,
                anchor: Anchor::Middle,
                vertical: false,
                bold: true,
                rich: g.rich_text,
                color: INK,
            },
            false,
        );
    }
}

fn build_layer_full(ctx: LayerContext<'_, '_, '_>) -> Scene {
    let g = ctx.layer;
    let w = ctx.w;
    let h = ctx.h;
    let data = prepare_layer_data(&ctx);
    let ranges = axis_ranges(&ctx, &data);
    let layout = axis_layout(&ctx, ranges);
    let (mut items, mut objects, data_start) = draw_layer_data(&ctx, &data, &layout);
    let data_end = draw_axis_ticks(&ctx, &layout, &mut items);
    draw_axis_titles(&ctx, &layout, &mut items);
    if let Some(scene) = legend::draw_layer_legend(&ctx, &data, &layout, &mut items, &mut objects, data_start, data_end)
    {
        return scene;
    }
    Scene {
        w,
        h,
        plot: layout.rect,
        layers: vec![LayerView {
            bounds: [0.0, 0.0, w, h],
            plot: layout.rect,
            axes: layout.axes.clone(),
            opaque: !g.transparent,
        }],
        axes: layout.axes,
        items,
        layer_plots: vec![layout.rect],
        objects,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mark::path::{decimate_line, decimate_symbols};
    use crate::model::Project;

    #[test]
    fn outward_ticks_on_left_and_bottom_axes_only() {
        let p = Project::sample();
        let sc = build(p.graph(2).unwrap(), p.book(1), 800.0, 600.0, None);
        let [l, t, w, h] = sc.plot;
        // 只有左轴和下轴：一条三点折线，没有闭合的坐标框
        assert!(sc.items.iter().any(|i| matches!(&i.prim,
            Prim::Polyline { pts, color, .. } if *color == INK && pts.len() == 3
                && pts.iter().zip([[l, t], [l, t + h], [l + w, t + h]]).all(|(a, b)| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9))));
        assert!(!sc.items.iter().any(|i| matches!(
            i.prim,
            Prim::Rect {
                fill: None,
                stroke: Some(_),
                ..
            }
        )));
        // 刻度朝外：一端在左轴或下轴上，另一端在坐标框外；上边和右边没有刻度
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
        // 版面按页宽整体放大：刻度长度乘同一倍数，坐标允许末位舍入差异
        let k = p.graph(2).unwrap().page.scale();
        let eq = |a: f64, b: f64| (a - b).abs() < 1e-9;
        for s in segs {
            let bottom = eq(s[1], t + h) && s[3] > t + h && eq(s[0], s[2]) && s[0] >= l - 1e-9 && s[0] <= l + w + 1e-9;
            let left = eq(s[0], l) && s[2] < l && eq(s[1], s[3]) && s[1] >= t - 1e-9 && s[1] <= t + h + 1e-9;
            assert!(
                bottom || left,
                "tick {s:?} should point outward from the left or bottom axis"
            );
            assert!(s[3] - s[1] <= MAJOR_TICK * k + 1e-9 && s[0] - s[2] <= MAJOR_TICK * k + 1e-9);
        }
    }

    #[test]
    fn decimation_keeps_extremes_per_pixel_column_and_small_data_untouched() {
        let small: Vec<[f64; 2]> = (0..10).map(|i| [i as f64, (i * i) as f64]).collect();
        assert_eq!(decimate_line(small.clone(), 100.0), small);
        assert_eq!(decimate_symbols(small.clone(), 100.0), small);
        // 10 万点挤在 100 个像素列里：每列至多 4 点，保留每列的最高与最低点
        let pts: Vec<[f64; 2]> = (0..100_000)
            .map(|i| [i as f64 / 1000.0, ((i as f64) * 0.37).sin() * 50.0])
            .collect();
        let out = decimate_line(pts.clone(), 100.0);
        assert!(out.len() <= 400, "{}", out.len());
        for column in 0..100 {
            let group: Vec<&[f64; 2]> = pts.iter().filter(|p| p[0].floor() == column as f64).collect();
            let kept: Vec<&[f64; 2]> = out.iter().filter(|p| p[0].floor() == column as f64).collect();
            let max = group.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
            let min = group.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
            assert!(kept.iter().any(|p| p[1] == max) && kept.iter().any(|p| p[1] == min));
            assert_eq!(kept.first().unwrap(), group.first().unwrap());
            assert_eq!(kept.last().unwrap(), group.last().unwrap());
        }
        assert!(decimate_symbols(pts, 100.0).len() < 100_000);
    }

    #[test]
    fn frame_closes_the_box_and_inward_ticks_stay_inside() {
        let mut p = Project::sample();
        let base = build(p.graph(2).unwrap(), p.book(1), 800.0, 600.0, None);
        let layer = p.graph_mut(2).unwrap().first_layer_mut();
        layer.frame = true;
        layer.tick_dir = crate::page::TickDir::In;
        let sc = build(p.graph(2).unwrap(), p.book(1), 800.0, 600.0, None);
        let [l, t, w, h] = sc.plot;
        let eq = |a: f64, b: f64| (a - b).abs() < 1e-9;
        // 闭合坐标框：五点折线回到起点
        assert!(sc.items.iter().any(|i| matches!(&i.prim,
            Prim::Polyline { pts, color, .. } if *color == INK && pts.len() == 5
                && eq(pts[3][0], l + w) && eq(pts[3][1], t) && pts[4] == pts[0])));
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
        // 朝内：全部刻度在坐标框内；四条边都有刻度
        let inside = |x: f64, y: f64| x >= l - 1e-9 && x <= l + w + 1e-9 && y >= t - 1e-9 && y <= t + h + 1e-9;
        assert!(
            segs.iter().all(|s| inside(s[0], s[1]) && inside(s[2], s[3])),
            "{segs:?}"
        );
        for edge in [
            &(|s: &[f64; 4]| eq(s[3], t + h) || eq(s[1], t + h)) as &dyn Fn(&[f64; 4]) -> bool,
            &|s| eq(s[1], t) || eq(s[3], t),
            &|s| eq(s[0], l) || eq(s[2], l),
            &|s| eq(s[0], l + w) || eq(s[2], l + w),
        ] {
            assert!(segs.iter().any(edge));
        }
        // 刻度不占框外空间：坐标框比默认（朝外）更大
        assert!(sc.plot[2] > base.plot[2] && sc.plot[3] > base.plot[3]);
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
