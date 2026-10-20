//! 同一图层放来自不同工作簿的曲线（A2）：添加、取数据、排版、删列删表时的清理和项目往返。
use scicanvas_core::{plotspec, scene, svg, PlotType, Project, Role};

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
