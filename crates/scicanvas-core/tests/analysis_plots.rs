//! 分析操作改用统一曲线入口后，结果绑定与旧实现一致。
use scicanvas_core::analysis::SmoothMethod;
use scicanvas_core::baseline::BaselineMethod;
use scicanvas_core::fft::FilterKind;
use scicanvas_core::history::History;
use scicanvas_core::interpolate::InterpMethod;
use scicanvas_core::nlfit::Model;
use scicanvas_core::ops::{Item, Source};
use scicanvas_core::page::{CurveStyle, Layer};
use scicanvas_core::plotspec::{DataRef, PlotSpec, Transform};
use scicanvas_core::{svg, Graph, PlotType, Project, Role};

fn project() -> (Project, u64, u64, u64) {
    let mut p = Project::default();
    let a = p.new_book();
    let b = p.new_book();
    for (book, offset) in [(a, 0), (b, 20)] {
        let book = p.book_mut(book).unwrap();
        book.columns[0].data = (0..12).map(|i| (i + offset).to_string()).collect();
        book.columns[1].data = (0..12).map(|i| (3 + 2 * (i + offset)).to_string()).collect();
        book.columns[1].long_name = "电压".into();
    }
    let (gid, _) = p.plot_from_selection(a, &[], PlotType::LineSymbol).unwrap();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let mut target = p.graph(gid).unwrap().first_layer().clone();
    target.rows = Some((2, 9));
    let key = DataRef::new(b, "B").key(target.book_id);
    target.edit_draft(|draft| draft.row_by_y.insert(key, Some((1, 7))));
    for (source, right_axis) in [(a, false), (b, true)] {
        let key = DataRef::new(source, "B").key(target.book_id);
        target.edit_styles(|styles| {
            styles.insert(
                key,
                CurveStyle {
                    right_axis,
                    color: Some([70, 90, 120]),
                    symbol_size: Some(8.0),
                    legend_label: "原数据".into(),
                    ..Default::default()
                },
            )
        });
    }
    p.graph_mut(gid).unwrap().page.add_layer(target);
    (p, a, b, gid)
}

fn check_roundtrip(p: &Project) {
    #[cfg(feature = "serde")]
    {
        use scicanvas_core::project_file;
        let reopened = project_file::from_json(&project_file::to_json(p).unwrap()).unwrap();
        assert_eq!(reopened, *p);
        assert_eq!(project_file::from_scv(&project_file::to_scv(p).unwrap()).unwrap(), *p);
        for graph in &p.graphs {
            assert_eq!(
                svg::render_page(graph, p),
                svg::render_page(reopened.graph(graph.id).unwrap(), &reopened)
            );
        }
    }
    #[cfg(not(feature = "serde"))]
    let _ = p;
}

fn check_bindings(before: &Project, after: &Project, gid: u64, bindings: &[(Item, String, Option<String>)]) {
    let mut legacy = before.graph(gid).unwrap().page.layers()[1].clone();
    for (item, result, x) in bindings {
        legacy.add_analysis_result(item.book_id, &item.y, result, x.as_deref());
    }
    let graph = after.graph(gid).unwrap();
    assert_eq!(graph.page.layers()[1], legacy);
    assert_eq!(graph.first_layer(), before.graph(gid).unwrap().first_layer());
    let mut control = after.clone();
    control.graph_mut(gid).unwrap().page.layers_mut()[1] = legacy;
    assert_eq!(
        svg::render_page(graph, after),
        svg::render_page(control.graph(gid).unwrap(), &control)
    );
    check_roundtrip(after);
}

#[test]
fn analysis_results_match_legacy_layer_bindings_and_undo_for_each_operation() {
    for operation in ["poly", "nonlinear", "interpolate", "smooth", "baseline", "filter"] {
        let (mut p, _, _, gid) = project();
        let source = Source::Layer {
            graph_id: gid,
            index: 1,
        };
        let items = p.resolve(&source).unwrap().items;
        let before = p.clone();
        let mut history = History::default();
        history.begin(&p, operation);
        let bindings: Vec<(Item, String, Option<String>)> = match operation {
            "poly" => {
                let (target, reports) = p.fit_poly(&source, 1).unwrap();
                assert_eq!(target, gid);
                items
                    .iter()
                    .cloned()
                    .zip(reports)
                    .map(|(item, r)| (item, r.curve.1, Some(r.curve.0)))
                    .collect()
            }
            "nonlinear" => {
                let (target, reports) = p.nlfit(&source, Model::Allometric2, &[], &[]).unwrap();
                assert_eq!(target, gid);
                items
                    .iter()
                    .cloned()
                    .zip(reports)
                    .map(|(item, r)| (item, r.curve.1, Some(r.curve.0)))
                    .collect()
            }
            "interpolate" => {
                let (target, keys) = p.interpolate(&source, InterpMethod::Linear, 30, None).unwrap();
                assert_eq!(target, gid);
                items
                    .iter()
                    .cloned()
                    .zip(keys)
                    .map(|(item, key)| {
                        let x = p
                            .book(item.book_id)
                            .unwrap()
                            .columns
                            .iter()
                            .find(|c| c.role == Role::X && c.derived_from.as_deref() == Some(item.y.as_str()))
                            .unwrap()
                            .key
                            .clone();
                        (item, key, Some(x))
                    })
                    .collect()
            }
            _ => {
                let keys = match operation {
                    "smooth" => p.smooth(&source, SmoothMethod::MovingAverage, 3).unwrap(),
                    "baseline" => p
                        .subtract_baseline(&source, BaselineMethod::Line)
                        .unwrap()
                        .into_iter()
                        .map(|pair| pair.0)
                        .collect(),
                    "filter" => p.fft_filter(&source, FilterKind::LowPass(0.2)).unwrap(),
                    _ => unreachable!(),
                };
                items
                    .iter()
                    .cloned()
                    .zip(keys)
                    .map(|(item, key)| {
                        let x = item.x.clone();
                        (item, key, x)
                    })
                    .collect()
            }
        };
        assert_eq!(bindings.len(), 2, "{operation}");
        history.changed();
        history.commit(&p);
        check_bindings(&before, &p, gid, &bindings);
        assert_eq!(history.undo(&p).unwrap(), before, "{operation}");
        assert_eq!(history.redo(&before).unwrap(), p, "{operation}");
    }
}

#[test]
fn bands_reach_all_matching_layers_and_residuals_keep_each_reports_x_policy() {
    let (mut p, _, _, gid) = project();
    let (_, reports) = p
        .fit_poly(
            &Source::Layer {
                graph_id: gid,
                index: 1,
            },
            1,
        )
        .unwrap();
    let extra = p.alloc_id();
    let mut graph = Graph::new(extra, "共享拟合曲线");
    *graph.first_layer_mut() = p.graph(gid).unwrap().page.layers()[1].clone();
    p.graphs.push(graph);
    let before = p.clone();
    let mut legacy = before.clone();
    let mut history = History::default();
    history.begin(&p, "拟合带");
    for report in &reports {
        let keys = p.poly_bands(report, 0.95).unwrap();
        assert_eq!(keys.len(), 4);
        for graph in &mut legacy.graphs {
            for layer in graph.page.layers_mut() {
                if layer
                    .draft()
                    .y_keys
                    .contains(&layer.result_key(report.book_id, &report.curve.1))
                {
                    for key in &keys {
                        layer.add_analysis_result(report.book_id, &report.curve.1, key, Some(&report.curve.0));
                    }
                }
            }
        }
    }
    legacy.books = p.books.clone();
    assert_eq!(p, legacy);
    history.changed();
    history.commit(&p);
    assert_eq!(history.undo(&p).unwrap(), before);
    assert_eq!(history.redo(&before).unwrap(), p);
    for implicit_foreign_x in [false, true] {
        let mut reports = reports.clone();
        if implicit_foreign_x {
            reports[1].x_key = None;
        }
        let previous = p.graphs.clone();
        let residual = p.residual_plot(&reports).unwrap();
        let mut spec = PlotSpec {
            book_id: Some(reports[0].book_id),
            x_key: reports[0].x_key.clone(),
            plot_type: PlotType::Scatter,
            ..Default::default()
        };
        for report in &reports {
            let key = DataRef::new(report.book_id, &report.residual_key).key(spec.book_id);
            spec.y_keys.push(key.clone());
            if let Some(x) = &report.x_key {
                spec.x_by_y
                    .insert(key, DataRef::new(report.book_id, x).key(spec.book_id));
            }
        }
        assert_eq!(p.graph(residual).unwrap().first_layer().draft(), spec);
        assert_eq!(&p.graphs[..previous.len()], previous.as_slice());
        check_roundtrip(&p);
    }
}

#[test]
fn row_number_and_floating_sources_keep_legacy_baseline_bindings() {
    for floating in [false, true] {
        let (mut p, a, b, gid) = project();
        if floating {
            for id in [a, b] {
                let book = p.book_mut(id).unwrap();
                let key = book.push_column(Role::Y);
                book.columns.iter_mut().find(|c| c.key == key).unwrap().data =
                    (0..12).map(|i| (30 + i).to_string()).collect();
            }
            let spec =
                PlotSpec::from_selection(p.book(a).unwrap(), &["B".into(), "C".into()], PlotType::FloatingBar).unwrap();
            p.graph_mut(gid).unwrap().page.layers_mut()[1] = Layer {
                spec: spec.into(),
                ..Default::default()
            };
            p.add_curves(gid, 1, b, &["B".into(), "C".into()]).unwrap();
            let layer = &mut p.graph_mut(gid).unwrap().page.layers_mut()[1];
            let key = DataRef::new(b, "C").key(layer.book_id);
            layer.edit_styles(|styles| {
                styles.insert(
                    key,
                    CurveStyle {
                        right_axis: true,
                        ..Default::default()
                    },
                )
            });
        } else {
            for id in [a, b] {
                p.book_mut(id).unwrap().columns[0].role = Role::Ignore;
            }
            let layer = &mut p.graph_mut(gid).unwrap().page.layers_mut()[1];
            layer.x_key = None;
            layer.edit_draft(|draft| draft.x_by_y.clear());
        }
        let source = Source::Layer {
            graph_id: gid,
            index: 1,
        };
        let items = p.resolve(&source).unwrap().items;
        let before = p.clone();
        let pairs = p.subtract_baseline(&source, BaselineMethod::Line).unwrap();
        let bindings: Vec<_> = items
            .into_iter()
            .zip(pairs)
            .map(|(item, (key, corrected))| {
                let source = DataRef::new(item.book_id, corrected);
                assert!(p.graph(gid).unwrap().page.layers()[1]
                    .plot(&source, &|id| p.book(id))
                    .is_none());
                let x = item.x.clone();
                (item, key, x)
            })
            .collect();
        assert_eq!(bindings.len(), 2);
        check_bindings(&before, &p, gid, &bindings);
    }
}

#[test]
fn book_analysis_creates_and_reuses_scatter_layers_with_the_original_defaults() {
    let (mut p, a, _, _) = project();
    let book = p.book_mut(a).unwrap();
    let c = book.push_column(Role::Y);
    book.columns.iter_mut().find(|col| col.key == c).unwrap().data = (0..12).map(|i| (5 + 3 * i).to_string()).collect();
    for graph in &mut p.graphs {
        for layer in graph.page.layers_mut() {
            layer.transform = Transform::Mean;
        }
    }
    let count = p.graphs.len();
    let (_, reports) = p
        .fit_poly(
            &Source::Book {
                id: a,
                selected: vec!["B".into(), c],
                rows: Some((2, 8)),
            },
            1,
        )
        .unwrap();
    assert_eq!(p.graphs.len(), count + 2);
    for (graph, report) in p.graphs[count..].iter().zip(&reports) {
        let mut legacy = Graph::new(graph.id, &graph.name);
        let layer = legacy.first_layer_mut();
        layer.book_id = Some(a);
        layer.x_key = report.x_key.clone();
        layer.edit_draft(|draft| draft.y_keys = vec![report.y_key.clone()]);
        layer.plot_type = PlotType::Scatter;
        layer.rows = Some((2, 8));
        layer.add_analysis_result(a, &report.y_key, &report.curve.1, Some(&report.curve.0));
        assert_eq!(*graph, legacy);
    }
    let reused = p.graphs[count].id;
    let (target, _) = p
        .fit_poly(
            &Source::Book {
                id: a,
                selected: vec!["B".into()],
                rows: Some((3, 6)),
            },
            1,
        )
        .unwrap();
    assert_eq!(target, reused);
    assert_eq!(p.graphs.len(), count + 2);
    assert_eq!(p.graph(target).unwrap().first_layer().rows, Some((2, 8)));
    check_roundtrip(&p);
}

#[test]
fn write_only_results_preserve_graphs_column_order_error_sources_and_history() {
    use scicanvas_core::analysis::NormalizeMethod;
    use scicanvas_core::ops::{ColumnOp, Operand};
    for operation in [
        "smooth",
        "filter",
        "baseline",
        "normalize",
        "differentiate",
        "row_stats",
        "math",
        "calibrate",
    ] {
        let (mut p, a, _, _) = project();
        // 已存在的结果列和自动关联误差列，不能被新结果列插入改变。
        let error = p.book_mut(a).unwrap().push_column(Role::YError);
        p.book_mut(a).unwrap().columns.last_mut().unwrap().data = vec!["0.5".into(); 12].into();
        let expected_errors = p
            .book(a)
            .unwrap()
            .error_source(p.book(a).unwrap().column_index(&error).unwrap())
            .map(str::to_owned);
        let mut existing = scicanvas_core::Column::new("", Role::Y);
        existing.long_name = "已有结果".into();
        let existing_key = p.insert_result_column(a, "B", existing).unwrap();
        let before = p.clone();
        let mut history = History::default();
        history.begin(&p, operation);
        let source = Source::Book {
            id: a,
            selected: vec!["B".into()],
            rows: Some((2, 8)),
        };
        let keys = match operation {
            "smooth" => p.smooth(&source, SmoothMethod::MovingAverage, 3).unwrap(),
            "filter" => p.fft_filter(&source, FilterKind::LowPass(0.2)).unwrap(),
            "baseline" => {
                let pairs = p.subtract_baseline(&source, BaselineMethod::Line).unwrap();
                pairs.into_iter().flat_map(|(a, b)| [a, b]).collect()
            }
            "normalize" => p.normalize(&source, NormalizeMethod::DivideByMax).unwrap(),
            "differentiate" => p.differentiate(&source).unwrap(),
            "row_stats" => p.row_stats(a, &["A".into(), "B".into()]).unwrap(),
            "math" => vec![p.column_math(a, "B", ColumnOp::Add, &Operand::Constant(2.0)).unwrap()],
            "calibrate" => {
                let (_, concentration, interval) = p.calibrate(a, "A", "B", "B").unwrap();
                vec![concentration, interval]
            }
            _ => unreachable!(),
        };
        assert_eq!(p.graphs, before.graphs, "{operation} changed graphs for a Book source");
        let book = p.book(a).unwrap();
        let position = book.column_index(&existing_key).unwrap();
        assert_eq!(
            book.columns[position + 1..position + 1 + keys.len()]
                .iter()
                .map(|c| c.key.clone())
                .collect::<Vec<_>>(),
            keys
        );
        for key in &keys {
            assert_eq!(book.column(key).unwrap().derived_from.as_deref(), Some("B"));
        }
        assert_eq!(
            book.error_source(book.column_index(&error).unwrap()).map(str::to_owned),
            expected_errors
        );
        assert!(book.column(&error).is_some());
        history.changed();
        history.commit(&p);
        assert_eq!(history.undo(&p).unwrap(), before);
        assert_eq!(history.redo(&before).unwrap(), p);
        check_roundtrip(&p);
    }
}
