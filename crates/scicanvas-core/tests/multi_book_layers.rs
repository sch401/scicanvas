//! 同一图层放来自不同工作簿的曲线（A2）：添加、取数据、排版、删列删表时的清理和项目往返。
use scicanvas_core::{plotspec, scene, svg, PlotType, Project, Role};

#[test]
fn shared_curve_addition_handles_empty_graphs_duplicates_and_floating_pairs() {
    let mut p = Project::default();
    let book = p.new_book();
    let c = p.book_mut(book).unwrap().push_column(Role::Y);
    let d = p.book_mut(book).unwrap().push_column(Role::Y);
    let e = p.book_mut(book).unwrap().push_column(Role::Y);
    let (gid, _) = p
        .plot_from_selection(book, &["B".into(), c.clone()], PlotType::FloatingBar)
        .unwrap();
    assert_eq!(
        p.add_curves(gid, 0, book, &[d.clone(), e.clone(), d.clone()]).unwrap(),
        1
    );
    assert_eq!(p.add_curves(gid, 0, book, &[d.clone(), e.clone()]).unwrap(), 0);
    let layer = p.graph(gid).unwrap().first_layer();
    assert_eq!(layer.y_keys, vec![c.clone(), e.clone()]);
    assert_eq!(layer.base_by_y[&e], d);
    let before = p.clone();
    assert!(p.add_curves(gid, 0, book, &["B".into(), e]).is_err());
    assert_eq!(p, before, "an invalid interval does not change existing curves");
    let mut empty = scicanvas_core::page::Layer::default();
    assert_eq!(
        empty
            .add_book_curves(p.book(book).unwrap(), &["B".into(), "B".into()], false)
            .unwrap(),
        1
    );
    assert_eq!(empty.book_id, Some(book));
    assert_eq!(empty.y_keys, vec!["B"]);
}

#[test]
fn removing_primary_workbook_preserves_remaining_curves_and_analysis() {
    use scicanvas_core::ops::Source;
    let (mut p, a, b, gid) = project();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    p.remove(a);
    assert_eq!(p.graph(gid).unwrap().first_layer().book_id, Some(b));
    let resolved = p.resolve(&Source::Graph(gid)).unwrap();
    assert_eq!(resolved.items.len(), 1);
    assert_eq!(resolved.items[0].book_id, b);
    assert!(svg::render_page(p.graph(gid).unwrap(), &p).contains("电流"));
    let (_, reports) = p.fit_poly(&Source::Graph(gid), 1).unwrap();
    assert_eq!(reports[0].book_id, b);
    assert!((reports[0].fit.coef[1] - 1.0).abs() < 1e-10);
}

#[test]
fn nonlinear_fits_and_failed_fft_validation_respect_foreign_sources() {
    use scicanvas_core::{nlfit::Model, ops::Source};
    let (mut p, a, b, gid) = project();
    for (id, amplitude) in [(a, 4.0), (b, 12.0)] {
        let book = p.book_mut(id).unwrap();
        book.columns[0].data = (0..20).map(|i| (i as f64 * 0.2).to_string()).collect();
        book.columns[1].data = (0..20)
            .map(|i| (2.0 + amplitude * (-i as f64 * 0.2 / 1.5).exp()).to_string())
            .collect();
    }
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let source = Source::Graph(gid);
    let (_, reports) = p.nlfit(&source, Model::ExpDec1, &[], &[]).unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports.iter().map(|r| r.book_id).collect::<Vec<_>>(), vec![a, b]);
    for report in &reports {
        assert!(report.fit.r2 > 0.999999);
    }
    p.book_mut(b).unwrap().columns[0].data.set_text(2, "0.31".into());
    let before = p.clone();
    assert!(p
        .fft_filter(&source, scicanvas_core::fft::FilterKind::LowPass(0.2))
        .is_err());
    assert_eq!(p, before);
}

#[test]
fn per_curve_rows_are_shared_by_rendering_analysis_and_v11_files() {
    use scicanvas_core::ops::Source;
    let (mut p, a, b, gid) = project();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let key = plotspec::foreign_key(b, "B");
    p.graph_mut(gid)
        .unwrap()
        .first_layer_mut()
        .row_by_y
        .insert(key, Some((1, 3)));
    let source = Source::Curves {
        graph_id: gid,
        index: 0,
        curves: vec![plotspec::DataRef::new(b, "B")],
    };
    assert_eq!(p.resolve(&source).unwrap().items[0].rows, Some((1, 3)));
    let lookup = |id| p.book(id);
    let (_, series) = p
        .graph(gid)
        .unwrap()
        .first_layer()
        .series_in(p.book(a).unwrap(), &lookup);
    assert_eq!(series[1].rows, vec![1, 2, 3]);
    let (_, reports) = p.fit_poly(&source, 1).unwrap();
    assert_eq!(reports[0].fit.n, 3);
    p.insert_rows(b, 0, 1);
    assert_eq!(p.resolve(&source).unwrap().items[0].rows, Some((2, 4)));
    #[cfg(feature = "serde")]
    {
        use scicanvas_core::project_file;
        assert_eq!(project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap(), p);
    }
}

#[test]
fn cross_book_analysis_writes_fits_residuals_and_bands_to_each_source() {
    use scicanvas_core::ops::Source;
    let (mut p, a, b, gid) = project();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let key = plotspec::foreign_key(b, "B");
    p.graph_mut(gid).unwrap().first_layer_mut().curves.insert(
        key,
        scicanvas_core::page::CurveStyle {
            right_axis: true,
            ..Default::default()
        },
    );
    let resolved = p.resolve(&Source::Graph(gid)).unwrap();
    assert_eq!(resolved.items.iter().map(|i| i.book_id).collect::<Vec<_>>(), vec![a, b]);
    let (_, reports) = p.fit_poly(&Source::Graph(gid), 1).unwrap();
    assert_eq!(reports.len(), 2);
    for (report, book, slope) in [(&reports[0], a, 2.0), (&reports[1], b, 1.0)] {
        assert_eq!(report.book_id, book);
        assert!((report.fit.coef[1] - slope).abs() < 1e-10);
        assert!(report.name.contains(&p.book(book).unwrap().name));
        assert!(p.book(book).unwrap().column(&report.residual_key).is_some());
        let key = plotspec::DataRef::new(book, &report.curve.1).key(Some(a));
        let layer = p.graph(gid).unwrap().first_layer();
        assert!(layer.line_only.contains(&key));
        if book == b {
            assert!(layer.curves[&key].right_axis);
        }
    }
    let residual = p.residual_plot(&reports).unwrap();
    assert_eq!(p.resolve(&Source::Graph(residual)).unwrap().items.len(), 2);
    let bands = p.poly_bands(&reports[1], 0.95).unwrap();
    for key in bands {
        let key = plotspec::foreign_key(b, &key);
        assert!(p.graph(gid).unwrap().first_layer().curves[&key].right_axis);
    }
    #[cfg(feature = "serde")]
    {
        let json = scicanvas_core::project_file::to_json(&p).unwrap();
        assert_eq!(scicanvas_core::project_file::from_json(&json).unwrap(), p);
    }
}

#[test]
fn selected_foreign_curve_operations_leave_main_table_unchanged() {
    use scicanvas_core::{
        analysis::{NormalizeMethod, SmoothMethod},
        ops::Source,
    };
    type Operation = fn(&mut Project, &Source) -> Result<Vec<String>, String>;
    let operations: [Operation; 5] = [
        |p, s| p.differentiate(s),
        |p, s| p.normalize(s, NormalizeMethod::MinMax),
        |p, s| p.smooth(s, SmoothMethod::MovingAverage, 3),
        |p, s| p.integrate(s).map(|r| r.0),
        |p, s| p.fft_filter(s, scicanvas_core::fft::FilterKind::LowPass(0.2)),
    ];
    for operation in operations {
        let (mut p, a, b, gid) = project();
        p.add_curves(gid, 0, b, &["B".into()]).unwrap();
        let before = p.book(a).unwrap().clone();
        let source = Source::Curves {
            graph_id: gid,
            index: 0,
            curves: vec![plotspec::DataRef::new(b, "B")],
        };
        let made = operation(&mut p, &source).unwrap();
        assert_eq!(p.book(a).unwrap(), &before);
        assert_eq!(made.len(), 1);
        assert!(p.book(b).unwrap().column(&made[0]).is_some());
        let mapping = p.graph(gid).unwrap().first_layer();
        assert!(mapping.validate_in(p.book(a).unwrap(), &|id| p.book(id)).is_ok());
    }
}

#[cfg(feature = "serde")]
#[test]
fn v11_uses_typed_curve_objects_and_reads_v10_without_changing_svg() {
    use scicanvas_core::project_file;
    let (mut p, _, b, gid) = project();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let before = svg::render_page(p.graph(gid).unwrap(), &p);
    let mut old = serde_json::to_value(&p).unwrap();
    old["version"] = 10.into();
    let migrated = project_file::from_json(&old.to_string()).unwrap();
    assert_eq!(svg::render_page(migrated.graph(gid).unwrap(), &migrated), before);
    let json = project_file::to_json(&migrated).unwrap();
    let wire: serde_json::Value = serde_json::from_str(&json).unwrap();
    let layer = &wire["graphs"][0]["page"]["layers"][0];
    assert!(layer.get("y_keys").is_none());
    assert_eq!(
        layer["plots"][1]["mapping"]["y"],
        serde_json::json!({"book":b,"col":"B"})
    );
    assert_eq!(project_file::from_json(&json).unwrap(), migrated);
    let mut inherited = migrated.clone();
    inherited
        .graph_mut(gid)
        .unwrap()
        .first_layer_mut()
        .x_by_y
        .remove(&plotspec::foreign_key(b, "B"));
    assert_eq!(
        project_file::from_json(&project_file::to_json(&inherited).unwrap()).unwrap(),
        inherited
    );
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&migrated).unwrap()).unwrap(),
        migrated
    );
    for field in ["zero_book", "duplicate", "mixed_xy", "bad_rows"] {
        let mut bad = wire.clone();
        let plots = &mut bad["graphs"][0]["page"]["layers"][0]["plots"];
        match field {
            "zero_book" => plots[1]["mapping"]["y"]["book"] = 0.into(),
            "duplicate" => plots[1] = plots[0].clone(),
            "mixed_xy" => plots[1]["mapping"]["x"]["book"] = 999.into(),
            _ => plots[1]["mapping"]["rows"] = serde_json::json!([5, 1]),
        }
        assert!(project_file::from_json(&bad.to_string()).is_err(), "{field}");
    }
}

#[test]
fn cross_book_statistics_average_and_fft_use_all_selected_sources() {
    use scicanvas_core::ops::Source;
    let (mut p, a, b, gid) = project();
    p.book_mut(b).unwrap().columns[0].data = (0..5).map(|x| x.to_string()).collect();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let source = Source::Graph(gid);
    let columns = p.analysis_columns(&source).unwrap();
    assert_eq!(columns.iter().map(|c| c.source.book).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(columns[1].values, vec![110.0, 111.0, 112.0, 113.0, 114.0]);
    let statistics = p.stats_on_columns(&source).unwrap();
    assert_eq!(p.book(statistics).unwrap().nrows(), 2);
    let (average, _) = p.average_curves(&source).unwrap();
    let mean = p
        .book(average)
        .unwrap()
        .columns
        .iter()
        .find(|c| c.long_name == "平均")
        .unwrap()
        .values();
    assert_eq!(mean, vec![55.0, 56.5, 58.0, 59.5, 61.0]);
    assert_eq!(p.fft(&source, scicanvas_core::fft::Window::Rect).unwrap().len(), 2);
    assert_eq!(p.frequency_counts(&source, Some(2), None).unwrap().len(), 2);
}

/// Book1：X = 0..5，Y = 2x；Book2：X = 10..15，Y = x + 100，另有一列不在 X 右侧的 Y。
fn project() -> (Project, u64, u64, u64) {
    let mut p = Project::default();
    let a = p.new_book();
    let b = p.new_book();
    let book = p.book_mut(a).unwrap();
    book.columns[0].data = (0..5).map(|i| i.to_string()).collect();
    book.columns[1].data = (0..5).map(|i| (2 * i).to_string()).collect();
    book.columns[1].long_name = "电流".into();
    let book = p.book_mut(b).unwrap();
    book.name = "对照组".into();
    book.columns[0].data = (10..15).map(|i| i.to_string()).collect();
    book.columns[1].data = (10..15).map(|i| (i + 100).to_string()).collect();
    book.columns[1].long_name = "电流".into();
    let c = book.push_column(Role::Y);
    let col = book.columns.iter_mut().find(|x| x.key == c).unwrap();
    col.data = (0..5).map(|i| (-i).to_string()).collect();
    let (gid, _) = p.plot_from_selection(a, &[], PlotType::LineSymbol).unwrap();
    (p, a, b, gid)
}

#[test]
fn keys_name_the_other_workbook_and_split_back() {
    assert_eq!(plotspec::foreign_key(12, "B"), "B@12");
    assert_eq!(plotspec::split_key("B@12"), ("B", Some(12)));
    assert_eq!(plotspec::split_key("B"), ("B", None));
    assert_eq!(plotspec::split_key("B@x"), ("B@x", None));
}

#[test]
fn curves_from_another_workbook_use_their_own_x_and_rows() {
    let (mut p, a, b, gid) = project();
    // 只看本表的行范围不影响其他工作簿的曲线
    p.graph_mut(gid).unwrap().first_layer_mut().rows = Some((1, 2));
    assert_eq!(p.add_curves(gid, 0, b, &["B".into(), "C".into()]).unwrap(), 2);
    // 重复添加不会多出曲线
    assert_eq!(p.add_curves(gid, 0, b, &["B".into()]).unwrap(), 0);
    let layer = p.graph(gid).unwrap().first_layer();
    assert_eq!(layer.y_keys, vec!["B".to_string(), format!("B@{b}"), format!("C@{b}")]);
    assert_eq!(layer.x_by_y.get(&format!("B@{b}")), Some(&format!("A@{b}")));
    assert_eq!(layer.foreign_books(), vec![b]);

    let lookup = |id: u64| p.book(id);
    let (_, series) = layer.series_in(p.book(a).unwrap(), &lookup);
    assert_eq!(series.len(), 3);
    assert_eq!(series[0].points, vec![[1.0, 2.0], [2.0, 4.0]]);
    assert_eq!(
        series[1].points,
        (10..15).map(|i| [f64::from(i), f64::from(i + 100)]).collect::<Vec<_>>()
    );
    assert_eq!(series[2].points[4], [14.0, -4.0]);
    // 混合了多个工作簿时，图例带工作簿名
    assert_eq!(series[0].name, "Book1：电流");
    assert_eq!(series[1].name, "对照组：电流");
    assert_eq!(series[1].key, format!("B@{b}"));
    assert_eq!(layer.curve_label(p.book(a), &lookup, &format!("B@{b}")), "对照组：电流");
    assert!(layer.validate_in(p.book(a).unwrap(), &lookup).is_ok());
    // 只给本表时其他工作簿的曲线被跳过
    assert_eq!(layer.series(p.book(a).unwrap()).1.len(), 1);

    // 自动范围覆盖两个工作簿的数据
    let g = p.graph(gid).unwrap();
    let sc = scene::layout_page(g, &p, &[None]);
    let view = sc.layers[0].view();
    assert!(view.x.0 <= 1.0 && view.x.1 >= 14.0, "{view:?}");
    assert!(view.y.0 <= -4.0 && view.y.1 >= 114.0, "{view:?}");
    let page = svg::render_page(g, &p);
    assert!(page.contains("对照组：电流"));

    // 鼠标吸附到其他工作簿的点，返回曲线键
    let lv = &sc.layers[0];
    let [x0, y0, w, h] = lv.plot;
    let at = |x: f64, y: f64| {
        (
            x0 + (x - lv.xa.lo) / (lv.xa.hi - lv.xa.lo) * w,
            y0 + h - (y - lv.ya.lo) / (lv.ya.hi - lv.ya.lo) * h,
        )
    };
    let (px, py) = at(12.0, 112.0);
    let hit = scene::nearest_point(g.first_layer(), lv, &lookup, px, py, 4.0).unwrap();
    assert_eq!(hit.key, format!("B@{b}"));
    assert_eq!((hit.row, hit.x, hit.y), (Some(2), 12.0, 112.0));
}

#[test]
fn missing_x_mapping_falls_back_to_the_other_workbooks_x_column() {
    let (mut p, a, b, gid) = project();
    let key = plotspec::foreign_key(b, "B");
    let layer = p.graph_mut(gid).unwrap().first_layer_mut();
    layer.y_keys.push(key.clone());
    let lookup = |id: u64| p.book(id);
    let layer = p.graph(gid).unwrap().first_layer();
    let (_, series) = layer.series_in(p.book(a).unwrap(), &lookup);
    assert_eq!(series[1].points[0], [10.0, 110.0]);
}

#[test]
fn deleting_columns_or_the_workbook_drops_only_its_curves() {
    let (mut p, a, b, gid) = project();
    p.add_curves(gid, 0, b, &["B".into(), "C".into()]).unwrap();
    let key = format!("C@{b}");
    p.graph_mut(gid)
        .unwrap()
        .first_layer_mut()
        .curves
        .insert(key.clone(), Default::default());
    p.delete_columns(b, &["C".into()]);
    let layer = p.graph(gid).unwrap().first_layer();
    assert_eq!(layer.y_keys, vec!["B".to_string(), format!("B@{b}")]);
    assert!(!layer.curves.contains_key(&key));
    // 删掉其他工作簿的 X 列：依赖它的曲线一起去掉
    p.delete_columns(b, &["A".into()]);
    assert_eq!(p.graph(gid).unwrap().first_layer().y_keys, vec!["B"]);
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    p.remove(b);
    let layer = p.graph(gid).unwrap().first_layer();
    assert_eq!(layer.book_id, Some(a));
    assert_eq!(layer.y_keys, vec!["B"]);
}

#[test]
fn adding_to_an_empty_layer_binds_it_to_the_workbook() {
    let (mut p, _, b, gid) = project();
    let layer = p.graph_mut(gid).unwrap().first_layer_mut();
    layer.book_id = None;
    layer.y_keys.clear();
    assert_eq!(p.add_curves(gid, 0, b, &["A".into(), "C".into()]).unwrap(), 1);
    let layer = p.graph(gid).unwrap().first_layer();
    assert_eq!(layer.book_id, Some(b));
    assert_eq!(layer.x_key.as_deref(), Some("A"));
    assert_eq!(layer.y_keys, vec!["C"]);
    assert!(p.add_curves(gid, 0, b, &["A".into()]).is_err());
}

#[test]
fn removing_a_curve_clears_its_mappings_and_style() {
    let (mut p, _, b, gid) = project();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let key = format!("B@{b}");
    let layer = p.graph_mut(gid).unwrap().first_layer_mut();
    layer.curves.insert(key.clone(), Default::default());
    layer.remove_curve(&key);
    assert_eq!(layer.y_keys, vec!["B"]);
    assert!(layer.x_by_y.is_empty() && layer.curves.is_empty());
}

#[cfg(feature = "serde")]
#[test]
fn mixed_layers_round_trip_through_the_project_file() {
    let (mut p, _, b, gid) = project();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    let text = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(scicanvas_core::project_file::from_json(&text).unwrap(), p);
}
