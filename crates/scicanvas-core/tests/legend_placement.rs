//! 图例自动摆放：按实际画出的柱子、阶梯线、面积填充避让，而不是只看原始数据点。
//! 每个用例都留出一个空角，且原始数据点给出的“最空”角正好压在数据上。

use scicanvas_core::{
    scene::{self, Prim, Scene, PAPER},
    Column, PlotType, Project, Role,
};

fn project(kind: PlotType, xs: &[f64], ys: &[&[f64]]) -> (Project, u64) {
    let mut p = Project::default();
    p.new_book();
    let b = p.book_mut(1).unwrap();
    b.columns.clear();
    let mut x = Column::new("A", Role::X);
    x.data = xs.iter().map(|v| v.to_string()).collect();
    b.columns.push(x);
    for values in ys {
        b.push_column(Role::Y);
        b.columns.last_mut().unwrap().data = values.iter().map(|v| v.to_string()).collect();
    }
    let (id, _) = p.plot_from_selection(1, &[], kind).unwrap();
    (p, id)
}

fn legend(sc: &Scene) -> [f64; 4] {
    let found: Vec<_> = sc
        .items
        .iter()
        .filter_map(|it| match it.prim {
            Prim::Rect {
                x,
                y,
                w,
                h,
                fill: Some((PAPER, _)),
                stroke: None,
            } if it.clip.is_none() && (x, y) != (0.0, 0.0) => Some([x, y, w, h]),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "exactly one legend box");
    found[0]
}

fn inside([l, t, w, h]: [f64; 4], q: [f64; 2]) -> bool {
    q[0] > l && q[0] < l + w && q[1] > t && q[1] < t + h
}

fn in_triangle(q: [f64; 2], [a, b, c]: [[f64; 2]; 3]) -> bool {
    let side = |p: [f64; 2], u: [f64; 2]| (u[0] - p[0]) * (q[1] - p[1]) - (u[1] - p[1]) * (q[0] - p[0]);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

/// 断言图例框与所有数据几何（柱、线段、三角形、符号）都不相交。
fn assert_legend_clear(kind: PlotType, sc: &Scene) {
    let box_ = legend(sc);
    let [l, t, w, h] = box_;
    let mut segments: Vec<[[f64; 2]; 2]> = Vec::new();
    let mut areas: Vec<[[f64; 2]; 3]> = Vec::new();
    for it in &sc.items {
        match &it.prim {
            Prim::Rect { x, y, w: rw, h: rh, .. } if it.clip.is_some() => {
                let overlap = x + rw > l && *x < l + w && y + rh > t && *y < t + h;
                assert!(!overlap, "{kind:?}: legend {box_:?} covers bar {:?}", [x, y, rw, rh]);
            }
            Prim::Polyline { pts, .. } if it.clip.is_some() => {
                segments.extend(pts.windows(2).map(|p| [p[0], p[1]]));
            }
            Prim::Segments { segs, .. } if it.clip.is_some() => {
                segments.extend(segs.iter().map(|s| [[s[0], s[1]], [s[2], s[3]]]));
            }
            Prim::Triangles { pts, .. } => areas.extend_from_slice(pts.as_chunks::<3>().0),
            Prim::Symbols { pts, r, .. } => {
                for p in pts {
                    let expanded = [l - r, t - r, w + 2.0 * r, h + 2.0 * r];
                    assert!(!inside(expanded, *p), "{kind:?}: legend {box_:?} covers symbol {p:?}");
                }
            }
            _ => {}
        }
    }
    for tri in &areas {
        segments.extend([[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]]);
    }
    for [a, b] in segments {
        let n = ((b[0] - a[0]).hypot(b[1] - a[1]) * 2.0).ceil().max(1.0) as usize;
        for k in 0..=n {
            let f = k as f64 / n as f64;
            let q = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
            assert!(!inside(box_, q), "{kind:?}: legend {box_:?} covers line at {q:?}");
        }
    }
    let (mut x, step) = (l, 1.0);
    while x <= l + w {
        let mut y = t;
        while y <= t + h {
            assert!(
                !areas.iter().any(|tri| in_triangle([x, y], *tri)),
                "{kind:?}: legend {box_:?} covers filled area at {:?}",
                [x, y]
            );
            y += step;
        }
        x += step;
    }
}

fn check(kind: PlotType, xs: &[f64], ys: &[&[f64]], adjust: impl FnOnce(&mut scicanvas_core::page::Layer)) {
    let (mut p, id) = project(kind, xs, ys);
    adjust(p.graph_mut(id).unwrap().first_layer_mut());
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert_legend_clear(kind, &sc);
}

#[test]
fn bars_hanging_from_the_top_keep_the_legend_off_the_bars() {
    // 负值柱从顶端的 0 往下画，柱顶数据点都在下方；右下角两根短柱上方才是空角。
    check(
        PlotType::Bar,
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[&[-10.0, -10.0, -10.0, -2.0, -2.0]],
        |_| {},
    );
}

#[test]
fn stacked_bars_use_the_stacked_heights() {
    // 未堆叠的原始值都不超过 5，但堆叠后左侧三根柱高到 10。
    check(
        PlotType::StackedBar,
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[&[5.0; 5], &[5.0, 5.0, 5.0, 1.0, 1.0]],
        |_| {},
    );
}

#[test]
fn percent_stacked_bars_fill_the_full_height() {
    // 每根柱都到 100%，只有右侧手动留出的空白可以放图例。
    check(
        PlotType::PercentStackedBar,
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[&[1.0; 5], &[2.0; 5]],
        |l| (l.x_auto, l.x_min, l.x_max) = (false, 0.0, 9.0),
    );
}

#[test]
fn floating_bars_count_their_whole_span() {
    // 浮动柱的两端点在上下边，中间整段都是柱子；最右两根很短，右上角是空的。
    check(
        PlotType::FloatingBar,
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[&[0.0; 5], &[10.0, 10.0, 10.0, 2.0, 2.0]],
        |_| {},
    );
}

#[test]
fn horizontal_bars_keep_the_legend_off_the_bars() {
    // 上方两根条较短，右上角是空的；左上角没有条端点，但被条身占满。
    check(
        PlotType::HorizontalBar,
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        &[&[10.0, 10.0, 10.0, 10.0, 4.0, 4.0]],
        |_| {},
    );
}

#[test]
fn step_lines_follow_their_horizontal_and_vertical_runs() {
    // 端点连线是对角线，但阶梯线沿顶部走过一个上角，另一侧的下角才是空的。
    check(
        PlotType::StepHorizontal,
        &[1.0, 10.0, 19.0],
        &[&[9.0, 9.0, 0.0]],
        |_| {},
    );
    check(PlotType::StepVertical, &[1.0, 10.0, 19.0], &[&[0.0, 9.0, 9.0]], |_| {});
}

#[test]
fn area_fill_is_avoided_not_just_its_outline() {
    // 负值面积从顶端的 0 填到曲线；左侧曲线在底部，左上角整块被填充。
    check(
        PlotType::Area,
        &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        &[&[-10.0, -10.0, -10.0, -10.0, -1.0, -1.0, -1.0, -1.0, -1.0]],
        |_| {},
    );
}
