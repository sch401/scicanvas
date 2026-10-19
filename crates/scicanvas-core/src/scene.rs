//! 图形版面：把一张图排成像素坐标下的图元列表。
//!
//! 界面上的绘制和 SVG 导出都从这里取图元，所以屏幕上看到的和导出的论文插图完全一致。
//! 默认是 Nature 图稿风格：白底、只保留左轴和下轴、刻度朝外、不画网格、
//! 无边框图例自动避开数据；按 96 单位 = 1 英寸，常用页宽 89 mm 时文字为 6–7 pt。

use crate::coord::{Cartesian, Coord};
use crate::model::{series_styles, Book, Graph, PlotType, Symbol};
use crate::page::Layer;
use crate::ticks::{axis, Axis, TickLabel};

pub type Rgb = (u8, u8, u8);

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
    Polyline { pts: Vec<[f64; 2]>, color: Rgb, width: f64 },
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
    pub layers: Vec<LayerView>,
}

/// 鼠标吸附到的数据点：曲线名、原始行（统计变换后为 None）、数据值和页面位置。
#[derive(Clone, Debug, PartialEq)]
pub struct PointHit {
    pub series: String,
    pub row: Option<usize>,
    pub x: f64,
    pub y: f64,
    pub at: [f64; 2],
}

/// 页面坐标 (x, y) 附近 `radius` 以内最近的数据点（不含拟合等结果曲线）。
/// 堆叠柱的 Y 取该段自身的值；水平条形图按交换后的轴定位。
pub fn nearest_point(
    layer: &Layer,
    view: &LayerView,
    book: Option<&Book>,
    x: f64,
    y: f64,
    radius: f64,
) -> Option<PointHit> {
    let (_, series) = layer.series(book?);
    let horizontal = layer.plot_type == PlotType::HorizontalBar;
    let identity = layer.transform == crate::plotspec::Transform::Identity;
    let map = Cartesian {
        x: &view.xa,
        y: &view.ya,
        rect: view.plot,
    };
    let right_axis = view.ya2.clone().unwrap_or_else(|| view.ya.clone());
    let map_right = Cartesian {
        x: &view.xa,
        y: &right_axis,
        rect: view.plot,
    };
    let mut best: Option<(f64, PointHit)> = None;
    for s in series.iter().filter(|s| !s.line_only) {
        let right = view.ya2.is_some() && layer.curves.get(&s.key).is_some_and(|c| c.right_axis);
        let map = if right { &map_right } else { &map };
        for (i, p) in s.points.iter().enumerate() {
            let at = map.project(if horizontal { [p[1], p[0]] } else { *p });
            if !at.iter().all(|v| v.is_finite()) || !view.contains(at[0], at[1]) {
                continue;
            }
            let d = (at[0] - x).hypot(at[1] - y);
            if d <= radius && best.as_ref().is_none_or(|(b, _)| d < *b) {
                let value = if layer.plot_type.is_stacked() {
                    p[1] - s.bases.get(i).copied().unwrap_or(0.0)
                } else {
                    p[1]
                };
                best = Some((
                    d,
                    PointHit {
                        series: s.name.clone(),
                        row: identity.then(|| s.rows.get(i).copied()).flatten(),
                        x: p[0],
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
    pub xa: Axis,
    pub ya: Axis,
    /// 右侧 Y 轴（有曲线放在右轴时）。
    pub ya2: Option<Axis>,
}
impl LayerView {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let [l, t, w, h] = self.plot;
        x >= l && x <= l + w && y >= t && y <= t + h
    }
    pub fn data_at(&self, x: f64, y: f64) -> (f64, f64) {
        let [x, y] = Cartesian {
            x: &self.xa,
            y: &self.ya,
            rect: self.plot,
        }
        .unproject([x, y]);
        (x, y)
    }
    pub fn view(&self) -> View {
        View {
            x: (self.xa.lo, self.xa.hi),
            y: (self.ya.lo, self.ya.hi),
        }
    }
}

impl Scene {
    /// 返回最上方图层；不穿透上层的背景与轴标签区域。
    pub fn layer_at(&self, x: f64, y: f64) -> Option<usize> {
        self.layers.iter().rposition(|layer| {
            let [l, t, w, h] = layer.bounds;
            x >= l && x <= l + w && y >= t && y <= t + h
        })
    }
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
    compose(g, |id| book.filter(|b| b.id == id), w, h, &[view])
}

/// 页面各图层独立解析工作簿；屏幕与整页导出使用同一入口。
pub fn build_page(g: &Graph, project: &crate::Project, w: f64, h: f64, view: Option<View>) -> Scene {
    compose(g, |id| project.book(id), w, h, &[view])
}

pub fn build_page_views(g: &Graph, project: &crate::Project, w: f64, h: f64, views: &[Option<View>]) -> Scene {
    compose(g, |id| project.book(id), w, h, views)
}

/// 屏幕、图库与出版导出共用的页面排版；窗口大小只影响显示缩放。
pub fn layout_page(g: &Graph, project: &crate::Project, views: &[Option<View>]) -> Scene {
    let [w, h] = g.page.logical_size();
    build_page_views(g, project, w, h, views)
}

fn compose<'a>(g: &Graph, book: impl Fn(u64) -> Option<&'a Book>, w: f64, h: f64, views: &[Option<View>]) -> Scene {
    // 网格区域：整页扣除页边距（按当前画布与逻辑页面的比例换算）
    let [lw0, lh0] = g.page.logical_size();
    let [ox, oy, gw, gh] = g.page.content_rect();
    let (sx, sy) = (w / lw0, h / lh0);
    // 用整页减去边距，边距为 0 时与整页完全相同
    let (ox, oy) = (ox * sx, oy * sy);
    let (gw, gh) = (w - (lw0 - gw) * sx, h - (lh0 - gh) * sy);
    let paper: Rgb = g.page.background.map_or(PAPER, |[r, gg, b]| (r, gg, b));
    let grid_count = g
        .page
        .layers()
        .iter()
        .filter(|layer| layer.placement.is_none())
        .count()
        .max(1);
    let cols = g.page.columns.min(grid_count).max(1);
    let rows = grid_count.div_ceil(cols);
    // 网格间距（逻辑单位），不超过可用宽高
    let (gap_x, gap_y) = (
        (g.page.gap_mm[0] * 96.0 / 25.4 * sx).min(gw * 0.5 / cols.max(2) as f64),
        (g.page.gap_mm[1] * 96.0 / 25.4 * sy).min(gh * 0.5 / rows.max(2) as f64),
    );
    let (cw, ch) = (
        (gw - gap_x * (cols - 1) as f64) / cols as f64,
        (gh - gap_y * (rows - 1) as f64) / rows as f64,
    );
    // 坐标轴联动：先按各自设置求出目标图层的范围，再把联动图层的视图设成目标范围
    let mut linked: Vec<Option<View>> = (0..g.page.layers().len())
        .map(|i| views.get(i).copied().flatten())
        .collect();
    if g.page.layers().iter().any(|l| l.link_x.is_some() || l.link_y.is_some()) {
        let own: Vec<View> = g
            .page
            .layers()
            .iter()
            .enumerate()
            .map(|(i, layer)| {
                build_layer(layer, layer.book_id.and_then(&book), 400.0, 300.0, linked[i]).layers[0].view()
            })
            .collect();
        for (i, layer) in g.page.layers().iter().enumerate() {
            if layer.link_x.is_none() && layer.link_y.is_none() {
                continue;
            }
            let pick = |link: Option<usize>, axis: fn(&View) -> (f64, f64)| {
                link.and_then(|j| own.get(j)).map_or(axis(&own[i]), axis)
            };
            linked[i] = Some(View {
                x: pick(layer.link_x, |v| v.x),
                y: pick(layer.link_y, |v| v.y),
            });
        }
    }
    let views = &linked[..];
    // 各图层先按 89 mm 一栏的参考尺寸排版，再把坐标、字号、线宽和符号同乘 k：
    // A4 等宽页面上的文字、点和线与单栏图成比例，版面相同只是整体放大。
    let k = g.page.scale();
    // 网格面板：先各自排版取自然边距，同一行取最大的上下边距、同一列取最大的左右边距，
    // 再按统一边距排版，使各面板的坐标框对齐（标题、刻度文字宽度不同也一样）。插图不参与。
    let mut cells = Vec::new();
    let mut aligned = vec![[0.0; 4]; g.page.layers().len()];
    if grid_count > 1 {
        let mut natural = Vec::new();
        for (i, layer) in g.page.layers().iter().enumerate() {
            if layer.placement.is_none() {
                let (rw, rh) = (cw / k, ch / k);
                let sc = build_layer(
                    layer,
                    layer.book_id.and_then(&book),
                    rw,
                    rh,
                    views.get(i).copied().flatten(),
                );
                let [l, t, pw, ph] = sc.plot;
                let index = cells.len();
                cells.push((i, index / cols, index % cols));
                natural.push([l, t, rw - l - pw, rh - t - ph]);
            }
        }
        for &(i, row, col) in &cells {
            let same = |f: &dyn Fn(usize, usize) -> bool, k: usize| {
                cells
                    .iter()
                    .zip(&natural)
                    .filter(|((_, r, c), _)| f(*r, *c))
                    .map(|(_, m)| m[k])
                    .fold(0.0, f64::max)
            };
            aligned[i] = [
                same(&|_, c| c == col, 0),
                same(&|r, _| r == row, 1),
                same(&|_, c| c == col, 2),
                same(&|r, _| r == row, 3),
            ];
        }
    }
    let mut grid_index = 0;
    let mut scenes = g.page.layers().iter().enumerate().map(|(i, layer)| {
        let [dx, dy, lw, lh] = match layer.placement {
            Some(rect) => [rect.x * w, rect.y * h, rect.width * w, rect.height * h],
            None => {
                let index = grid_index;
                grid_index += 1;
                [
                    ox + (index % cols) as f64 * (cw + gap_x),
                    oy + (index / cols) as f64 * (ch + gap_y),
                    cw,
                    ch,
                ]
            }
        };
        let mut sc = build_layer_min(
            layer,
            layer.book_id.and_then(&book),
            lw / k,
            lh / k,
            views.get(i).copied().flatten(),
            aligned[i],
        );
        for item in &mut sc.items {
            scale_prim(&mut item.prim, k);
            translate(&mut item.prim, dx, dy);
            if let Some(rect) = &mut item.clip {
                *rect = rect.map(|v| v * k);
                rect[0] += dx;
                rect[1] += dy;
            }
        }
        sc.plot = sc.plot.map(|v| v * k);
        sc.plot[0] += dx;
        sc.plot[1] += dy;
        sc.layers[0].plot = sc.plot;
        sc.layers[0].bounds = [dx, dy, lw, lh];
        sc
    });
    let mut page = scenes
        .next()
        .unwrap_or_else(|| build_layer(&Layer::default(), None, w, h, None));
    page.w = w;
    page.h = h;
    if g.page.layers().len() > 1
        || g.page.layers().iter().any(|l| l.transparent || l.placement.is_some())
        || g.page.background.is_some()
        || g.page.margin_mm != [0.0; 4]
    {
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
        page.layers.extend(sc.layers);
        page.items.extend(sc.items);
    }
    // 页面背景：页面、图层底色和图例底色都用同一颜色
    if paper != PAPER {
        for item in &mut page.items {
            if let Prim::Rect { fill: Some((c, _)), .. } = &mut item.prim {
                if *c == PAPER {
                    *c = paper;
                }
            }
        }
    }
    page
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

fn area_triangles(points: &[[f64; 2]], base: f64) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a[0] == b[0] || (a[1] == base && b[1] == base) {
            continue;
        }
        if (a[1] < base && b[1] > base) || (a[1] > base && b[1] < base) {
            let cross = [a[0] + (b[0] - a[0]) * ((base - a[1]) / (b[1] - a[1])), base];
            out.extend([a, cross, [a[0], base], b, [b[0], base], cross]);
        } else {
            out.extend([a, b, [b[0], base], a, [b[0], base], [a[0], base]]);
        }
    }
    out
}

/// 按原始行连续的段：缺失值、无效值或对数轴筛掉的行把曲线断开。所有连线图型共用。
fn row_runs(s: &crate::Series) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < s.points.len() {
        let mut end = start + 1;
        while end < s.points.len() && s.rows[end] == s.rows[end - 1] + 1 {
            end += 1;
        }
        out.push(start..end);
        start = end;
    }
    out
}

/// 面积和样条用的分段：每段按 X 排序。
fn data_runs(s: &crate::Series) -> Vec<Vec<[f64; 2]>> {
    row_runs(s)
        .into_iter()
        .map(|range| {
            let mut run = s.points[range].to_vec();
            run.sort_by(|a, b| a[0].total_cmp(&b[0]));
            run
        })
        .collect()
}

fn spline_paths(s: &crate::Series) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    for run in data_runs(s) {
        let mut start = 0;
        while start < run.len() {
            let mut end = start + 1;
            while end < run.len() && run[end][0] > run[end - 1][0] {
                end += 1;
            }
            // ponytail: 每区间固定 16 段采样；大数据绘图后续改为按页面误差自适应采样。
            out.push(crate::spline::natural(&run[start..end], 16).unwrap_or_else(|_| run[start..end].to_vec()));
            start = end;
        }
    }
    out
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
            Prim::Text {
                x,
                y,
                text,
                size,
                anchor,
                ..
            } => {
                let w = text_width(text, *size);
                let l = match anchor {
                    Anchor::Start => *x,
                    Anchor::Middle => x - w / 2.0,
                    Anchor::End => x - w,
                };
                out.push(Obstacle::Box([l, y - size / 2.0, l + w, y + size / 2.0]));
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

fn build_layer(g: &Layer, book: Option<&Book>, w: f64, h: f64, view: Option<View>) -> Scene {
    build_layer_min(g, book, w, h, view, [0.0; 4])
}

/// `min` 为绘图区四周的最小边距 [左, 上, 右, 下]，用于多面板对齐。
fn build_layer_min(g: &Layer, book: Option<&Book>, w: f64, h: f64, view: Option<View>, min: [f64; 4]) -> Scene {
    build_layer_full(g, book, w, h, view, min, false)
}

/// `legend_outside` 为真时，图例放在坐标框右侧（右边距已按图例宽度留出）。
fn build_layer_full(
    g: &Layer,
    book: Option<&Book>,
    w: f64,
    h: f64,
    view: Option<View>,
    min: [f64; 4],
    legend_outside: bool,
) -> Scene {
    let horizontal = g.plot_type == PlotType::HorizontalBar;
    // 数据（对数轴去掉非正值）
    let (x_col, mut series) = match book {
        Some(b) => g.series(b),
        None => (None, Vec::new()),
    };
    // 日期时间列做 X：用日期刻度（值为 UTC 毫秒）；分类列做 X：分类轴（水平条形图在 Y 轴）
    let x_date = !horizontal && !g.x_log && x_col.is_some_and(|c| c.data.kind() == crate::DataKind::DateTime);
    let categories = x_col.and_then(|c| c.data.category_levels());
    if horizontal {
        for s in &mut series {
            for p in &mut s.points {
                p.swap(0, 1);
            }
            for e in &mut s.errors {
                e.swap(0, 1);
            }
        }
    }
    for s in &mut series {
        // 均值变换后的点没有原始行，按顺序编号，连线不在组之间断开
        if (g.plot_type == PlotType::Area || g.plot_type.is_spline() || g.plot_type.has_line())
            && g.transform == crate::plotspec::Transform::Mean
            && !s.line_only
        {
            s.rows = (0..s.points.len()).collect();
        }
        let keep: Vec<_> = s
            .points
            .iter()
            .zip(&s.bases)
            .map(|(p, base)| {
                p[1].is_finite()
                    && base.is_finite()
                    && (!g.x_log || p[0] > 0.0)
                    && (!g.y_log || p[1] > 0.0 || (g.plot_type.is_bar() && !s.line_only && *base > 0.0))
            })
            .collect();
        let mut flags = keep.iter();
        s.points.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.errors.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.bases.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.rows.retain(|_| *flags.next().unwrap());
    }
    // 样式：默认按图层递增，再用逐曲线设置覆盖（图型、颜色、符号、线宽、符号大小、右侧 Y 轴）
    let mut styles = series_styles(&series, g.plot_type);
    if g.color_map != crate::colormap::ColorMap::Origin {
        let n = series.iter().filter(|s| !s.line_only).count();
        for style in &mut styles {
            if let crate::SeriesColor::Palette(i) = style.color {
                style.color = crate::SeriesColor::Rgb(g.color_map.sample(i as f64 / n.saturating_sub(1).max(1) as f64));
            }
        }
    }
    for (s, st) in series.iter().zip(&mut styles) {
        st.line_width = g.line_width;
        st.symbol_size = g.symbol_size;
        let Some(c) = g.curves.get(&s.key).filter(|_| !s.line_only) else {
            continue;
        };
        if let Some(kind) = c.kind.filter(|_| !g.plot_type.is_stacked() && !horizontal) {
            st.kind = kind;
        }
        if let Some([r, gg, b]) = c.color {
            st.color = crate::SeriesColor::Rgb((r, gg, b));
        }
        if let Some(sym) = c.symbol {
            st.symbol = sym;
        }
        if let Some(w) = c.line_width {
            st.line_width = w;
        }
        if let Some(z) = c.symbol_size {
            st.symbol_size = z;
        }
        st.right = c.right_axis && !horizontal;
    }
    // 图型覆盖后重新给柱子分组
    if g.curves.values().any(|c| c.kind.is_some()) {
        let bars: Vec<usize> = (0..series.len())
            .filter(|&i| styles[i].kind.is_bar() && !series[i].line_only)
            .collect();
        for (n, &i) in bars.iter().enumerate() {
            styles[i].bar_index = n;
            styles[i].bar_count = bars.len();
        }
    }
    let mut all_x: Vec<f64> = series.iter().flat_map(|s| s.points.iter().map(|p| p[0])).collect();
    // 右侧 Y 轴的曲线单独定范围
    let mut all_y: Vec<f64> = series
        .iter()
        .zip(&styles)
        .filter(|(_, st)| !st.right)
        .flat_map(|(s, _)| s.points.iter().map(|p| p[1]))
        .collect();
    let mut all_y2: Vec<f64> = series
        .iter()
        .zip(&styles)
        .filter(|(_, st)| st.right)
        .flat_map(|(s, _)| s.points.iter().map(|p| p[1]))
        .collect();
    let splines: Vec<_> = series
        .iter()
        .zip(&styles)
        .map(|(s, st)| {
            if st.kind.is_spline() && !s.line_only {
                spline_paths(s)
            } else {
                Vec::new()
            }
        })
        .collect();
    for (paths, st) in splines.iter().zip(&styles) {
        let target = if st.right { &mut all_y2 } else { &mut all_y };
        target.extend(paths.iter().flatten().map(|p| p[1]).filter(|v| !g.y_log || *v > 0.0));
    }
    if g.plot_type.is_bar() && g.plot_type != PlotType::Bar && !horizontal {
        all_y.extend(
            series
                .iter()
                .filter(|s| !s.line_only)
                .flat_map(|s| s.bases.iter().copied())
                .filter(|v| !g.y_log || *v > 0.0),
        );
    }
    if g.error_bars.visible {
        for (s, st) in series.iter().zip(&styles) {
            for (point, errors) in s.points.iter().zip(&s.errors) {
                let ys = if st.right { &mut all_y2 } else { &mut all_y };
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
    let is_bar = g.plot_type.is_bar() || styles.iter().any(|st| st.kind.is_bar());
    // 柱宽只按数据曲线的 X 间距算（拟合曲线是 200 个密集点）
    let data_x: Vec<f64> = series
        .iter()
        .filter(|s| !s.line_only)
        .flat_map(|s| s.points.iter().map(|p| p[usize::from(horizontal)]))
        .collect();
    let dx = min_spacing(&data_x);

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
                axis(&all_x, g.x_auto, g.x_min, g.x_max, g.x_log, horizontal)
            };
            let ya = if horizontal && g.y_auto && !g.y_log && !all_y.is_empty() {
                let lo = all_y.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = all_y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                axis(&[lo - dx * 0.6, hi + dx * 0.6], true, 0.0, 0.0, false, false)
            } else {
                axis(
                    &all_y,
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
            let marks: Vec<[f64; 2]> = series
                .iter()
                .zip(&styles)
                .filter(|(s, st)| !s.line_only && st.kind.has_symbol() && !st.right)
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
        let off = x_col.map_or(0.0, |c| f64::from(c.format.utc_offset) * 60_000.0);
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
    // 右侧 Y 轴：自动范围；带符号的点落在端点时同样留 4%
    let ya2 = (!all_y2.is_empty()).then(|| {
        let mut a = axis(&all_y2, true, 0.0, 0.0, g.y_log, false);
        let marks: Vec<f64> = series
            .iter()
            .zip(&styles)
            .filter(|(s, st)| !s.line_only && st.kind.has_symbol() && st.right)
            .flat_map(|(s, _)| s.points.iter().map(|p| p[1]))
            .collect();
        pad_ends(&mut a, marks.into_iter());
        a
    });
    let y2_label = TickLabel::Number(g.y_format);
    let ylab2_w = ya2.as_ref().map_or(0.0, |a| {
        a.ticks
            .iter()
            .map(|t| text_width(&y2_label.text(*t), TICK_FONT))
            .fold(8.0, f64::max)
    });

    // 版面：刻度朝外，刻度标签和轴标题依次排在左轴、下轴外侧
    let ylab_w = ya
        .ticks
        .iter()
        .map(|t| text_width(&y_label.text(*t), TICK_FONT))
        .fold(8.0, f64::max);
    let has_title = !g.title.trim().is_empty();
    // 刻度在框外的长度：朝内时刻度标签直接贴近坐标轴
    let (fout, fin) = g.tick_dir.factors();
    let out = MAJOR_TICK * fout;
    let left = (out + TICK_GAP + ylab_w + 5.0 + LABEL_FONT + 4.0).round().max(min[0]);
    let right = if ya2.is_some() {
        (out + TICK_GAP + ylab2_w + 5.0 + LABEL_FONT + 4.0).round()
    } else {
        14.0
    }
    .max(min[2]);
    let top = (if has_title { 12.0 + TITLE_FONT + 6.0 } else { 10.0f64 }).max(min[1]);
    let bottom = (out + TICK_GAP + TICK_FONT + 5.0 + LABEL_FONT + 5.0)
        .round()
        .max(min[3]);
    let pw = (w - left - right).max(20.0);
    let ph = (h - top - bottom).max(20.0);
    let coord = Cartesian {
        x: &xa,
        y: &ya,
        rect: [left, top, pw, ph],
    };
    let px = |v: f64| coord.project([v, ya.lo])[0];
    let py = |v: f64| coord.project([xa.lo, v])[1];
    // 右侧 Y 轴的映射；曲线按自己的轴选用
    let ya2_or = ya2.clone().unwrap_or_else(|| ya.clone());
    let coord2 = Cartesian {
        x: &xa,
        y: &ya2_or,
        rect: [left, top, pw, ph],
    };
    let py2 = |v: f64| coord2.project([xa.lo, v])[1];
    let py_left: &dyn Fn(f64) -> f64 = &py;
    let py_right: &dyn Fn(f64) -> f64 = &py2;
    let inside_x = |x: f64| x >= left - 0.5 && x <= left + pw + 0.5;
    let inside_y = |y: f64| y >= top - 0.5 && y <= top + ph + 0.5;

    let mut items: Vec<Item> = Vec::new();
    // 返回推入后的图元数，用来记下本图层数据图元的范围（图例按它避让）
    let mut push = |prim: Prim, clip: bool| {
        items.push(Item {
            prim,
            clip: clip.then_some([left, top, pw, ph]),
        });
        items.len()
    };
    let mut data_start = push(
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
        data_start = push(
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
    for ((s, st), paths) in series.iter().zip(&styles).zip(&splines) {
        let (ya, py) = if st.right { (&ya2_or, py_right) } else { (&ya, py_left) };
        let color = st.color.rgb();
        if st.kind.is_bar() {
            let gw = dx * 0.7;
            let count = if st.kind.is_stacked() { 1 } else { st.bar_count.max(1) };
            let index = if st.kind.is_stacked() { 0 } else { st.bar_index };
            let bw = gw / count as f64;
            for (p, base) in s.points.iter().zip(&s.bases) {
                let (category, value, ca, va) = if horizontal {
                    (p[1], p[0], ya, &xa)
                } else {
                    (p[0], p[1], &xa, ya)
                };
                let start = category - gw / 2.0 + index as f64 * bw;
                let end = category - gw / 2.0 + (index as f64 + 1.0) * bw;
                let category = |v: f64| if ca.log { v.max(ca.lo) } else { v };
                let base = base.clamp(va.lo, va.hi);
                let value = if va.log { value.max(va.lo) } else { value };
                let (a, b) = if horizontal {
                    ([px(base), py(category(start))], [px(value), py(category(end))])
                } else {
                    ([px(category(start)), py(base)], [px(category(end)), py(value)])
                };
                push(
                    Prim::Rect {
                        x: a[0].min(b[0]),
                        y: a[1].min(b[1]),
                        w: (b[0] - a[0]).abs(),
                        h: (a[1] - b[1]).abs(),
                        fill: Some((color, 1.0)),
                        stroke: Some((INK, 0.5)),
                    },
                    true,
                );
            }
            continue;
        }
        if st.kind == PlotType::Area {
            let base = py(if ya.log { ya.lo } else { 0.0f64.clamp(ya.lo, ya.hi) });
            for run in data_runs(s) {
                let pts: Vec<_> = run.iter().map(|p| [px(p[0]), py(p[1])]).collect();
                let triangles = area_triangles(&pts, base);
                if !triangles.is_empty() {
                    push(
                        Prim::Triangles {
                            pts: triangles,
                            color,
                            opacity: f64::from(g.area_opacity),
                        },
                        true,
                    );
                }
                if pts.len() > 1 {
                    push(
                        Prim::Polyline {
                            pts,
                            color,
                            width: f64::from(st.line_width),
                        },
                        true,
                    );
                }
            }
            continue;
        }
        if st.kind == PlotType::VerticalDrop {
            let base = py(if ya.log { ya.lo } else { 0.0f64.clamp(ya.lo, ya.hi) });
            push(
                Prim::Segments {
                    segs: s.points.iter().map(|p| [px(p[0]), base, px(p[0]), py(p[1])]).collect(),
                    color,
                    width: f64::from(st.line_width),
                    dashed: false,
                },
                true,
            );
        }
        if st.kind.is_spline() {
            for path in paths {
                for run in path.split(|p| (g.x_log && p[0] <= 0.0) || (g.y_log && p[1] <= 0.0)) {
                    if run.len() > 1 {
                        push(
                            Prim::Polyline {
                                pts: run.iter().map(|p| [px(p[0]), py(p[1])]).collect(),
                                color,
                                width: f64::from(st.line_width),
                            },
                            true,
                        );
                    }
                }
            }
        } else if st.kind.has_line() {
            // 与面积、样条一致：遇到缺失行断开，不跨过缺失值连线
            for range in row_runs(s).into_iter().filter(|r| r.len() > 1) {
                let run = &s.points[range];
                let pts = if matches!(st.kind, PlotType::StepHorizontal | PlotType::StepVertical) {
                    let mut pts = vec![[px(run[0][0]), py(run[0][1])]];
                    for pair in run.windows(2) {
                        let bend = if st.kind == PlotType::StepHorizontal {
                            [pair[1][0], pair[0][1]]
                        } else {
                            [pair[0][0], pair[1][1]]
                        };
                        pts.push([px(bend[0]), py(bend[1])]);
                        pts.push([px(pair[1][0]), py(pair[1][1])]);
                    }
                    pts
                } else {
                    // B 样条、Bezier 在数据坐标里算曲线（落在控制点凸包内，不改坐标范围）
                    let curve = match g.connect {
                        crate::page::Connect::BSpline if !s.line_only => crate::spline::bspline(run, 16),
                        crate::page::Connect::Bezier if !s.line_only => crate::spline::bezier(run, 16),
                        _ => run.to_vec(),
                    };
                    curve.iter().map(|p| [px(p[0]), py(p[1])]).collect()
                };
                push(
                    Prim::Polyline {
                        pts,
                        color,
                        width: f64::from(st.line_width),
                    },
                    true,
                );
            }
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
                r: f64::from(st.symbol_size) / 2.0,
                color,
            });
        }
    }
    if g.error_bars.visible {
        for (s, st) in series.iter().zip(&styles) {
            let (ya, py) = if st.right { (&ya2_or, py_right) } else { (&ya, py_left) };
            let mut segs = Vec::new();
            let offset = if st.kind.is_bar() && !st.kind.is_stacked() {
                dx * 0.7 * ((st.bar_index as f64 + 0.5) / st.bar_count.max(1) as f64 - 0.5)
            } else {
                0.0
            };
            let cap = f64::from(g.error_bars.cap) / 2.0;
            for (p, errors) in s.points.iter().zip(&s.errors) {
                let (x, y) = if horizontal {
                    (p[0], p[1] + offset)
                } else {
                    (p[0] + offset, p[1])
                };
                if let Some(e) = errors[0] {
                    let ends = [x - e, x + e];
                    let yy = py(y);
                    if (!ya.log || y > 0.0) && ends[1] >= xa.lo && ends[0] <= xa.hi {
                        segs.push([px(ends[0].clamp(xa.lo, xa.hi)), yy, px(ends[1].clamp(xa.lo, xa.hi)), yy]);
                        for end in ends {
                            if (xa.lo..=xa.hi).contains(&end) {
                                let xx = px(end);
                                segs.push([xx, yy - cap, xx, yy + cap]);
                            }
                        }
                    }
                }
                if let Some(e) = errors[1] {
                    let ends = [y - e, y + e];
                    let xx = px(x);
                    if (xa.lo..=xa.hi).contains(&x) && ends[1] >= ya.lo && ends[0] <= ya.hi {
                        segs.push([xx, py(ends[0].clamp(ya.lo, ya.hi)), xx, py(ends[1].clamp(ya.lo, ya.hi))]);
                        for end in ends {
                            if (ya.lo..=ya.hi).contains(&end) {
                                let yy = py(end);
                                segs.push([xx - cap, yy, xx + cap, yy]);
                            }
                        }
                    }
                }
            }
            if !segs.is_empty() {
                push(
                    Prim::Segments {
                        segs,
                        color: st.color.rgb(),
                        width: f64::from(g.error_bars.width),
                        dashed: false,
                    },
                    true,
                );
            }
        }
    }
    for s in symbols {
        push(s, false);
    }

    if g.labels.visible {
        use crate::page::LabelSource;
        for (s, st) in series.iter().zip(&styles).filter(|(s, _)| !s.line_only) {
            let py = if st.right { py_right } else { py_left };
            let offset = if st.kind.is_bar() && !st.kind.is_stacked() {
                dx * 0.7 * ((st.bar_index as f64 + 0.5) / st.bar_count.max(1) as f64 - 0.5)
            } else {
                0.0
            };
            for (i, p) in s.points.iter().enumerate() {
                let (x, y) = if horizontal {
                    (p[0], p[1] + offset)
                } else {
                    (p[0] + offset, p[1])
                };
                if !inside_x(px(x)) || !inside_y(py(y)) {
                    continue;
                }
                let value = if horizontal {
                    p[0]
                } else if st.kind.is_stacked() {
                    p[1] - s.bases[i]
                } else {
                    p[1]
                };
                let value_text = format!(
                    "{}{}",
                    crate::fmt_num(value),
                    if st.kind == PlotType::PercentStackedBar {
                        "%"
                    } else {
                        ""
                    }
                );
                let label = match g.labels.source {
                    LabelSource::Value => value_text,
                    LabelSource::XY => format!(
                        "({}, {})",
                        crate::fmt_num(if horizontal { p[1] } else { p[0] }),
                        value_text
                    ),
                    LabelSource::Row if g.transform == crate::plotspec::Transform::Identity => {
                        (s.rows[i] + 1).to_string()
                    }
                    LabelSource::Column if g.transform == crate::plotspec::Transform::Identity => g
                        .labels
                        .column
                        .as_deref()
                        .and_then(|k| book.and_then(|b| b.column(k)))
                        // 没有指定列时用工作表里的标签列
                        .or_else(|| book.and_then(|b| b.columns.iter().find(|c| c.role == crate::Role::Label)))
                        .and_then(|c| c.data.get(s.rows[i]))
                        .map(std::borrow::Cow::into_owned)
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                let label = label.replace(['\r', '\n'], " ");
                if label.trim().is_empty() {
                    continue;
                }
                let positive = if st.kind == PlotType::FloatingBar {
                    value >= s.bases[i]
                } else {
                    value >= 0.0
                };
                // 有误差棒时，标签放到误差棒端点以外，不压在误差棒和端帽上
                let reach = if g.error_bars.visible {
                    s.errors[i][usize::from(!horizontal)].map_or(0.0, |e| {
                        let d = if horizontal {
                            px(if positive { x + e } else { x - e }) - px(x)
                        } else {
                            py(if positive { y + e } else { y - e }) - py(y)
                        };
                        if d.is_finite() {
                            d.abs()
                        } else {
                            0.0
                        }
                    })
                } else {
                    0.0
                };
                let delta = f64::from(g.labels.offset) + f64::from(g.labels.size) / 2.0 + reach;
                let (xx, yy, anchor) = if horizontal {
                    (
                        px(x) + if positive { delta } else { -delta },
                        py(y),
                        if positive { Anchor::Start } else { Anchor::End },
                    )
                } else {
                    (px(x), py(y) + if positive { -delta } else { delta }, Anchor::Middle)
                };
                let width = text_width(&label, f64::from(g.labels.size)).min(pw);
                let xx = match anchor {
                    Anchor::Middle => xx.clamp(left + width / 2.0, (left + pw - width / 2.0).max(left + width / 2.0)),
                    Anchor::Start => xx.clamp(left, left + pw - width),
                    Anchor::End => xx.clamp(left + width, left + pw),
                };
                let half = (f64::from(g.labels.size) / 2.0).min(ph / 2.0);
                let yy = yy.clamp(top + half, top + ph - half);
                push(
                    Prim::Text {
                        x: xx,
                        y: yy,
                        text: label,
                        size: f64::from(g.labels.size),
                        anchor,
                        vertical: false,
                        bold: false,
                        color: st.color.rgb(),
                    },
                    true,
                );
            }
        }
    }

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
    for t in &xa.ticks {
        let x = px(*t);
        if !inside_x(x) {
            continue;
        }
        x_tick(&mut ticks, x, MAJOR_TICK);
        push(
            text(
                x,
                top + ph + out + TICK_GAP + TICK_FONT / 2.0,
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

    // 轴标题与图标题
    push(
        text(
            left + pw / 2.0,
            top + ph + out + TICK_GAP + TICK_FONT + 5.0 + LABEL_FONT / 2.0,
            g.x_axis_title(book),
            LABEL_FONT,
            Anchor::Middle,
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
                text: g.y2_axis_title(book),
                size: LABEL_FONT,
                anchor: Anchor::Middle,
                vertical: true,
                bold: false,
                color: INK,
            },
            false,
        );
    }
    push(
        Prim::Text {
            x: left - out - TICK_GAP - ylab_w - 5.0 - LABEL_FONT / 2.0,
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
                y: 6.0 + TITLE_FONT / 2.0,
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
        let lh = LEGEND_FONT + 4.0;
        let sw = 18.0;
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
        // 按本图层实际画出的几何（柱、线、面积、误差棒、符号、标签）给四个角打分
        let obstacles = obstacles(&items[data_start..data_end]);
        let score = |(x, y): (f64, f64)| coverage(&obstacles, [x - 4.0, y - 4.0, lw + 8.0, lhh + 8.0]);
        let (mut lx, mut ly) = corners.iter().copied().min_by_key(|c| score(*c)).unwrap_or(corners[0]);
        // 四个角都会压住数据（如百分比堆叠柱占满整个高度）：重排，图例放到坐标框右侧
        let gap = 10.0;
        if legend_outside {
            (lx, ly) = (left + pw + gap, top);
        } else if score((lx, ly)) > 0 && lw + gap + 14.0 < w * 0.35 {
            let mut wider = min;
            wider[2] = wider[2].max(lw + gap + 14.0);
            return build_layer_full(g, book, w, h, view, wider, true);
        }
        // 上面借用过 items，图例图元（都不裁剪）改用新的 push 推入
        let mut push = |prim: Prim| items.push(Item { prim, clip: None });
        push(Prim::Rect {
            x: lx,
            y: ly,
            w: lw,
            h: lhh,
            fill: Some((PAPER, 1.0)),
            stroke: None,
        });
        for (i, (s, st)) in series.iter().zip(&styles).enumerate() {
            let color = st.color.rgb();
            let cy = ly + 2.0 + lh * i as f64 + lh / 2.0;
            let x0 = lx + 4.0;
            if st.kind.is_bar() || st.kind == PlotType::Area {
                push(Prim::Rect {
                    x: x0 + 4.0,
                    y: cy - 3.5,
                    w: sw - 8.0,
                    h: 7.0,
                    fill: Some((
                        color,
                        if st.kind == PlotType::Area {
                            f64::from(g.area_opacity)
                        } else {
                            1.0
                        },
                    )),
                    stroke: Some((INK, 0.5)),
                });
            } else {
                if st.kind.has_line() {
                    push(Prim::Polyline {
                        pts: vec![[x0, cy], [x0 + sw, cy]],
                        color,
                        width: f64::from(st.line_width),
                    });
                }
                if st.kind.has_symbol() {
                    push(Prim::Symbols {
                        sym: st.symbol,
                        pts: vec![[x0 + sw / 2.0, cy]],
                        r: f64::from(st.symbol_size) / 2.0,
                        color,
                    });
                }
            }
            push(text(x0 + sw + 4.0, cy, s.name.clone(), LEGEND_FONT, Anchor::Start));
        }
    }

    Scene {
        w,
        h,
        plot: [left, top, pw, ph],
        layers: vec![LayerView {
            bounds: [0.0, 0.0, w, h],
            plot: [left, top, pw, ph],
            xa: xa.clone(),
            ya: ya.clone(),
            ya2: ya2.clone(),
        }],
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
