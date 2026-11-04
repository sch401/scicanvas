#![cfg(feature = "serde")]
//! 场景导出属于验收测试，不改变运行时 / 文件格式。
use scicanvas_core::{
    page::{Connect, CurveStyle, ReferenceAxis, ReferenceLine, SymbolFill},
    plotspec::PlotSpec,
    scene::{self, Prim},
    svg,
    ticks::Axis,
    Column, ColumnData, DataKind, Graph, PlotType, Project, Role,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/parity/visual")
}

fn column(key: &str, role: Role, values: &[&str]) -> Column {
    let mut c = Column::new(key, role);
    c.data = values.iter().map(|s| s.to_string()).collect::<Vec<_>>().into();
    c
}

fn case(name: &str, kind: PlotType) -> (String, Project, Graph) {
    let mut p = Project::default();
    p.new_book();
    p.book_mut(1).unwrap().columns = vec![
        column("A", Role::X, &["0", "1", "2", "3", "4"]),
        column("B", Role::Y, &["-2", "3", "", "5", "-1"]),
    ];
    if kind.is_bar() || matches!(kind, PlotType::QQ | PlotType::StackedArea | PlotType::FillBetween) {
        p.book_mut(1)
            .unwrap()
            .columns
            .push(column("C", Role::Y, &["1", "-2", "4", "2", "-3"]));
    } else if kind == PlotType::Box {
        p.book_mut(1)
            .unwrap()
            .columns
            .push(column("C", Role::Y, &["1", "2", "3", "4", "40"]));
    }
    if matches!(kind, PlotType::Spline | PlotType::SplineSymbol) || name == "bspline-bezier" {
        p.book_mut(1).unwrap().columns[1].data = ["-2", "3", "1", "5", "-1"].map(str::to_owned).to_vec().into();
    }
    let mut g = Graph::new(2, name);
    g.first_layer_mut().spec = PlotSpec::from_selection(p.book(1).unwrap(), &[], kind).unwrap().into();
    g.first_layer_mut().legend = false;
    if name == "scatter" {
        g.first_layer_mut().edit_draft(|draft| draft.rows = Some((1, 3)));
    }
    (name.into(), p, g)
}

fn cases() -> Vec<(String, Project, Graph)> {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| assert!(scicanvas_core::text::initialize(scicanvas_core::text::Fonts::bundled()).is_ok()));
    let mut cases: Vec<_> = scicanvas_core::presets::all()
        .into_iter()
        .map(|p| case(p.id, p.kind))
        .collect();
    let (_, mut p, mut g) = case("xy-errors", PlotType::Scatter);
    for (key, role, values) in [
        ("D", Role::XError, ["0.1", "0.2", "0.1", "0.3", "0.2"]),
        ("E", Role::YError, ["0.5", "1", "", "0.4", "0.2"]),
    ] {
        let mut c = column(key, role, &values);
        c.error_for = Some(if role == Role::XError { "A" } else { "B" }.into());
        p.book_mut(1).unwrap().columns.push(c);
    }
    g.first_layer_mut().error_bars.cap = 0.0; // 验收仅比较端点，端帽长度另属样式。
    cases.push(("xy-errors".into(), p, g));
    let (_, mut p, mut g) = case("log-axes", PlotType::LineSymbol);
    p.book_mut(1).unwrap().columns[0].data = ["0.1", "1", "10", "100", "1000"].map(str::to_owned).to_vec().into();
    p.book_mut(1).unwrap().columns[1].data = ["0.2", "2", "1", "20", "200"].map(str::to_owned).to_vec().into();
    g.first_layer_mut().x_log = true;
    g.first_layer_mut().y_log = true;
    cases.push(("log-axes".into(), p, g));
    let (_, mut p, mut g) = case("date-axis", PlotType::LineSymbol);
    let c = &mut p.book_mut(1).unwrap().columns[0];
    c.data = [
        "2026-10-30T00:00:00+08:00",
        "2026-10-31T00:00:00+08:00",
        "2026-11-01T00:00:00+08:00",
        "2026-11-02T00:00:00+08:00",
        "2026-11-03T00:00:00+08:00",
    ]
    .map(str::to_owned)
    .to_vec()
    .into();
    c.data = c.data.converted(DataKind::DateTime).unwrap().into();
    c.set_utc_offset(480);
    g.first_layer_mut().x_title = "UTC+8 date".into();
    cases.push(("date-axis".into(), p, g));
    let (_, mut p, g) = case("category-axis", PlotType::Scatter);
    p.book_mut(1).unwrap().columns[0].data = ColumnData::Category {
        codes: vec![2, 0, 1, 2, 0],
        levels: vec!["Low".into(), "Middle".into(), "High".into()],
        ordered: true,
    }
    .into();
    cases.push(("category-axis".into(), p, g));
    let (_, mut p, mut g) = case("dual-y", PlotType::LineSymbol);
    p.book_mut(1)
        .unwrap()
        .columns
        .push(column("C", Role::Y, &["100", "250", "500", "400", "800"]));
    g.first_layer_mut().spec = PlotSpec::from_selection(p.book(1).unwrap(), &[], PlotType::LineSymbol)
        .unwrap()
        .into();
    g.first_layer_mut().set_plot_style(
        "C",
        Some(CurveStyle {
            right_axis: true,
            ..Default::default()
        }),
    );
    cases.push(("dual-y".into(), p, g));
    for (name, n, columns) in [("two-panels", 2, 1), ("two-panels-side", 2, 2), ("four-panels", 4, 2)] {
        let (_, p, mut g) = case(name, PlotType::LineSymbol);
        let l = g.first_layer().clone();
        for _ in 1..n {
            g.page.add_layer(l.clone());
        }
        g.page.columns = columns;
        for (i, l) in g.page.layers_mut().iter_mut().enumerate() {
            l.title = format!("Panel {}", i + 1);
            l.y_auto = false;
            l.y_min = -3.0 - i as f64;
            l.y_max = 6.0 + i as f64;
        }
        cases.push((name.into(), p, g));
    }
    let (_, p, mut g) = case("reference-lines", PlotType::Line);
    g.first_layer_mut().reference_lines = vec![
        ReferenceLine {
            axis: ReferenceAxis::X,
            value: 1.5,
            ..Default::default()
        },
        ReferenceLine {
            axis: ReferenceAxis::YLeft,
            value: 2.0,
            ..Default::default()
        },
    ];
    cases.push(("reference-lines".into(), p, g));
    let (_, mut p, mut g) = case("color-size-shape-map", PlotType::Scatter);
    p.book_mut(1)
        .unwrap()
        .columns
        .push(column("C", Role::Ignore, &["0", "0.25", "0.5", "0.75", "1"]));
    let mut cat = column("D", Role::Ignore, &[]);
    cat.data = ColumnData::Category {
        codes: vec![0, 1, 2, 2, 0],
        levels: vec!["Alpha".into(), "Beta".into(), "Gamma".into()],
        ordered: true,
    }
    .into();
    p.book_mut(1).unwrap().columns.push(cat);
    g.first_layer_mut().color_map = scicanvas_core::colormap::ColorMap::Viridis;
    g.first_layer_mut().set_plot_style(
        "B",
        Some(CurveStyle {
            color_column: Some("C".into()),
            size_column: Some("C".into()),
            symbol_column: Some("D".into()),
            size_range: Some([2.0, 10.0]),
            ..Default::default()
        }),
    );
    cases.push(("color-size-shape-map".into(), p, g));
    let (_, p, mut g) = case("bspline-bezier", PlotType::Line);
    g.first_layer_mut().connect = Connect::BSpline;
    let mut l = g.first_layer().clone();
    l.connect = Connect::Bezier;
    g.page.add_layer(l);
    g.page.columns = 2;
    cases.push(("bspline-bezier".into(), p, g));
    // F083：先画 B、C，移除 C 后加入 D；核对剩余曲线位置。
    let (_, mut p, mut g) = case("add-remove-curve", PlotType::LineSymbol);
    p.book_mut(1).unwrap().columns.extend([
        column("C", Role::Y, &["1", "-2", "4", "2", "-3"]),
        column("D", Role::Y, &["0.5", "1.5", "2.5", "-1.5", "4"]),
    ]);
    let selected = ["A".to_string(), "B".to_string(), "C".to_string()];
    g.first_layer_mut().spec = PlotSpec::from_selection(p.book(1).unwrap(), &selected, PlotType::LineSymbol)
        .unwrap()
        .into();
    g.first_layer_mut().remove_curve("C");
    assert_eq!(
        g.first_layer_mut()
            .add_book_curves(p.book(1).unwrap(), &["D".to_string()], true),
        Ok(1)
    );
    cases.push(("add-remove-curve".into(), p, g));
    // F497：空心加红色边框、自定义填充加黑色边框；核对位置与内部填充，边框只在并排图展示。
    let (_, mut p, mut g) = case("symbol-fill-border", PlotType::Scatter);
    p.book_mut(1)
        .unwrap()
        .columns
        .push(column("C", Role::Y, &["1", "-2", "4", "2", "-3"]));
    g.first_layer_mut().spec = PlotSpec::from_selection(p.book(1).unwrap(), &[], PlotType::Scatter)
        .unwrap()
        .into();
    g.first_layer_mut().set_plot_style(
        "B",
        Some(CurveStyle {
            symbol_fill: Some(SymbolFill::Hollow),
            symbol_border_color: Some([200, 30, 30]),
            symbol_border_width: Some(1.2),
            ..Default::default()
        }),
    );
    g.first_layer_mut().set_plot_style(
        "C",
        Some(CurveStyle {
            symbol_fill: Some(SymbolFill::Color([31, 119, 180])),
            symbol_border_color: Some([0, 0, 0]),
            ..Default::default()
        }),
    );
    cases.push(("symbol-fill-border".into(), p, g));
    let (_, p, mut g) = case("histogram-options", PlotType::Histogram);
    g.first_layer_mut().histogram.bins = scicanvas_core::page::HistogramBins::Width(2.0);
    g.first_layer_mut().histogram.normal_curve = true;
    cases.push(("histogram-options".into(), p, g));
    cases
}

fn axis(a: &Axis) -> Value {
    json!({"lo":a.lo,"hi":a.hi,"log":a.log,"ticks":a.ticks.iter().map(|v|json!({"value":v,"position":a.frac(*v)})).collect::<Vec<_>>(),"minor":a.minor.iter().map(|v|json!({"value":v,"position":a.frac(*v)})).collect::<Vec<_>>()})
}

fn point([x, y]: [f64; 2], [l, t, w, h]: [f64; 4]) -> [f64; 2] {
    [(x - l) / w, 1.0 - (y - t) / h]
}

fn export(p: &Project, g: &Graph) -> (Value, String) {
    let scene = scene::layout_page(g, p, &[]);
    let mut layers = Vec::new();
    for (index, view) in scene.layers.iter().enumerate() {
        let rect = if scene.layers.len() == 1 {
            scene.plot
        } else {
            scene.layer_plots[index]
        };
        let l = &g.page.layers()[index];
        let mut symbols = Vec::new();
        let mut lines = Vec::new();
        let mut bars = Vec::new();
        let mut segments = Vec::new();
        let mut area = Vec::new();
        let mut colors = Vec::new();
        for item in &scene.items {
            let clipped = item.clip == Some(rect);
            let inside = |pts: &[[f64; 2]]| {
                !pts.is_empty()
                    && pts.iter().all(|p| {
                        p[0] >= rect[0] - 0.5
                            && p[0] <= rect[0] + rect[2] + 0.5
                            && p[1] >= rect[1] - 0.5
                            && p[1] <= rect[1] + rect[3] + 0.5
                    })
            };
            match &item.prim {
                Prim::Symbols { pts, .. } if inside(pts) => symbols.extend(pts.iter().map(|p| point(*p, rect))),
                Prim::StyledSymbols { symbols: marks } if inside(&marks.iter().map(|m| m.at).collect::<Vec<_>>()) => {
                    symbols.extend(marks.iter().map(|m| point(m.at, rect)));
                    colors.extend(marks.iter().map(|m| json!(m.fill.map(|(r, g, b)| [r, g, b]))));
                }
                Prim::Polyline { pts, .. } if clipped => {
                    lines.push(pts.iter().map(|p| point(*p, rect)).collect::<Vec<_>>())
                }
                Prim::Rect { x, y, w, h, .. } if clipped => {
                    let a = point([*x, *y], rect);
                    let b = point([x + w, y + h], rect);
                    bars.push(
                        if matches!(
                            l.plot_type,
                            PlotType::HorizontalBar
                                | PlotType::StackedHorizontalBar
                                | PlotType::PercentStackedHorizontalBar
                        ) {
                            json!([a[0].min(b[0]), a[0].max(b[0]), (a[1] + b[1]) / 2.0])
                        } else {
                            json!([(a[0] + b[0]) / 2.0, a[1].min(b[1]), a[1].max(b[1])])
                        },
                    );
                }
                Prim::Segments { segs, .. } if clipped => {
                    for s in segs {
                        if s[0] != s[2] || s[1] != s[3] {
                            let mut ends = [point([s[0], s[1]], rect), point([s[2], s[3]], rect)];
                            ends.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
                            segments.push(ends);
                        }
                    }
                }
                Prim::Triangles { pts, .. } if clipped => area.extend(pts.iter().map(|p| point(*p, rect))),
                _ => {}
            }
        }
        area.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let mut bounds: Vec<[f64; 3]> = Vec::new();
        for [x, y] in area {
            if let Some(last) = bounds.last_mut().filter(|p| p[0] == x) {
                last[1] = last[1].min(y);
                last[2] = last[2].max(y);
            } else {
                bounds.push([x, y, y]);
            }
        }
        let draft = l.draft();
        let plots = l.plots_in(&|id| p.book(id));
        let right: Vec<_> = plots
            .iter()
            .filter(|p| p.style.as_ref().is_some_and(|s| s.right_axis))
            .map(|p| p.key.clone())
            .collect();
        let mut entry = json!({"axes":{"x":axis(&view.axes.x),"y":axis(&view.axes.left_y),"right_y":view.axes.right_y.as_ref().map(axis)},"plot":rect,"kind":format!("{:?}",l.plot_type),"connect":format!("{:?}",l.connect),"y_keys":draft.y_keys,"base_by_y":draft.base_by_y,"right_keys":right,"rows":draft.rows,"reference_lines":l.reference_lines.iter().map(|r|json!({"axis":format!("{:?}",r.axis),"value":r.value})).collect::<Vec<_>>(),"marks":{"symbols":symbols,"polylines":lines,"bars":bars,"segments":segments,"area":bounds,"colors":colors}});
        if !l.histogram.is_default() {
            entry["histogram"] = serde_json::to_value(l.histogram).unwrap();
        }
        layers.push(entry);
    }
    let data:Vec<_>=p.book(1).unwrap().columns.iter().map(|c| {
        let category=if let ColumnData::Category {codes,levels,..}=&*c.data {Some(json!({"codes":codes,"levels":levels}))}else{None};
        json!({"key":c.key,"role":format!("{:?}",c.role),"kind":format!("{:?}",c.data.kind()),"values":c.values(),"category":category,"utc_offset":c.format.utc_offset})
    }).collect();
    (
        json!({"case":g.name,"page":{"w":scene.w,"h":scene.h},"data":data,"layers":layers}),
        svg::render_page(g, p),
    )
}

#[test]
fn committed_scenes_match_current_layout() {
    for (name, p, g) in cases() {
        let (actual, _) = export(&p, &g);
        let expected: Value =
            serde_json::from_str(&fs::read_to_string(directory().join(&name).join("scicanvas.json")).unwrap()).unwrap();
        assert_eq!(
            actual, expected,
            "场景改变：{name}；显式重新生成，并复核 matplotlib 对照"
        );
    }
}

#[test]
#[ignore = "维护者显式生成并复核参考对照后提交"]
fn update_visual_cases() {
    for (name, p, g) in cases() {
        let (data, svg) = export(&p, &g);
        let dir = directory().join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("scicanvas.json"),
            serde_json::to_string_pretty(&data).unwrap() + "\n",
        )
        .unwrap();
        fs::write(dir.join("scicanvas.svg"), svg).unwrap();
    }
}
