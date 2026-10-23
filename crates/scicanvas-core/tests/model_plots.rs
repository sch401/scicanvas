//! 模型迁移的兼容回归：用迁移前的旧表规则核对图层状态。
use scicanvas_core::history::History;
use scicanvas_core::page::{CurveStyle, Layer};
use scicanvas_core::plotspec::{DataRef, PlotSpec};
use scicanvas_core::{svg, Graph, Panels, PlotType, Project, Role};

fn project() -> (Project, u64, u64, u64) {
    let mut p = Project::default();
    let a = p.new_book();
    let b = p.new_book();
    for id in [a, b] {
        let book = p.book_mut(id).unwrap();
        book.columns[0].data = (0..12).map(|i| i.to_string()).collect();
        book.columns[1].data = (0..12).map(|i| (i + 2).to_string()).collect();
        for offset in [10, 20] {
            let key = book.push_column(Role::Y);
            book.columns.iter_mut().find(|c| c.key == key).unwrap().data =
                (0..12).map(|i| (i + offset).to_string()).collect();
        }
    }
    let (gid, _) = p
        .plot_from_selection(a, &["B".into(), "C".into()], PlotType::FloatingBar)
        .unwrap();
    p.add_curves(gid, 0, b, &["B".into(), "C".into()]).unwrap();
    let mut other = Layer {
        spec: PlotSpec::from_selection(p.book(b).unwrap(), &["B".into(), "C".into()], PlotType::FloatingBar).unwrap(),
        ..Default::default()
    };
    other
        .add_book_curves(p.book(a).unwrap(), &["B".into(), "C".into()], true)
        .unwrap();
    p.graph_mut(gid).unwrap().page.add_layer(other);
    for (i, layer) in p.graph_mut(gid).unwrap().page.layers_mut().iter_mut().enumerate() {
        layer.rows = Some((2, 7));
        layer.labels.column = Some("C".into());
        for id in [a, b] {
            let key = DataRef::new(id, "C").key(layer.book_id);
            layer.curves.insert(
                key,
                CurveStyle {
                    right_axis: id == b,
                    color: Some([20, 60, 120]),
                    ..Default::default()
                },
            );
            layer.add_analysis_result(id, "C", "D", Some("A"));
        }
        // 一条显式范围恰好等于默认范围，一条保持继承，派生线显式使用全行。
        if i == 0 {
            layer.row_by_y.insert("C".into(), Some((2, 7)));
        }
        let foreign = if layer.book_id == Some(a) { b } else { a };
        let key = DataRef::new(foreign, "C").key(layer.book_id);
        layer.row_by_y.insert(key, Some((1, 5)));
        layer.row_by_y.insert("D".into(), None);
        layer.row_by_y.insert("hidden".into(), Some((100, 105)));
        layer.x_by_y.insert("hidden".into(), "A".into());
        layer.base_by_y.insert("hidden".into(), "B".into());
        layer.line_only.push("hidden".into());
        layer.curves.insert(
            "hidden".into(),
            CurveStyle {
                right_axis: true,
                ..Default::default()
            },
        );
    }
    (p, a, b, gid)
}

fn legacy_delete(layer: &mut Layer, book: u64, keys: &[String]) {
    if layer.book_id != Some(book) {
        layer.drop_foreign(book, Some(keys));
        return;
    }
    layer.y_keys.retain(|k| !keys.contains(k));
    layer.line_only.retain(|k| !keys.contains(k));
    layer.row_by_y.retain(|k, _| !keys.contains(k));
    layer.curves.retain(|k, _| !keys.contains(k));
    layer.x_by_y.retain(|y, x| !keys.contains(y) && !keys.contains(x));
    layer.base_by_y.retain(|y, _| !keys.contains(y));
    if layer.labels.column.as_ref().is_some_and(|k| keys.contains(k)) {
        layer.labels.column = None;
    }
    for base in layer.base_by_y.values_mut() {
        if keys.contains(base) {
            base.clear();
        }
    }
    if layer.x_key.as_ref().is_some_and(|k| keys.contains(k)) {
        layer.x_key = None;
    }
}

fn check_files_and_svg(p: &Project, legacy: &Project) {
    assert_eq!(p, legacy);
    for graph in &p.graphs {
        assert_eq!(
            svg::render_page(graph, p),
            svg::render_page(legacy.graph(graph.id).unwrap(), legacy)
        );
    }
    #[cfg(feature = "serde")]
    assert_eq!(
        scicanvas_core::project_file::to_json(p),
        scicanvas_core::project_file::to_json(legacy)
    );
    check_roundtrip(p);
}

fn check_roundtrip(p: &Project) {
    #[cfg(feature = "serde")]
    {
        use scicanvas_core::project_file;
        let json = project_file::to_json(p).unwrap();
        let reopened = project_file::from_json(&json).unwrap();
        assert_eq!(project_file::to_json(&reopened).unwrap(), json);
        assert_eq!(
            project_file::from_scv(&project_file::to_scv(p).unwrap()).unwrap(),
            reopened
        );
    }
    #[cfg(not(feature = "serde"))]
    let _ = p;
}

#[test]
fn column_cleanup_matches_legacy_including_hidden_items_and_missing_bases() {
    for foreign in [false, true] {
        for key in ["A", "B", "C", "D", "hidden"] {
            let (mut p, a, b, _) = project();
            let book = if foreign { b } else { a };
            let before = p.clone();
            let mut legacy = before.clone();
            let keys = vec![key.to_string()];
            let mut history = History::default();
            history.begin(&p, "删列");
            p.delete_columns(book, &keys);
            legacy.books = p.books.clone();
            for graph in &mut legacy.graphs {
                for layer in graph.page.layers_mut() {
                    legacy_delete(layer, book, &keys);
                }
            }
            check_files_and_svg(&p, &legacy);
            history.changed();
            history.commit(&p);
            assert_eq!(history.undo(&p).unwrap(), before);
            assert_eq!(history.redo(&before).unwrap(), p);
            #[cfg(feature = "serde")]
            if key == "B" {
                // 缺失下端保留隐藏状态，同时允许正常保存。
                assert!(scicanvas_core::project_file::to_json(&p).is_ok());
            }
        }
    }
}

fn shift(range: &mut Option<(usize, usize)>, map: &impl Fn(usize) -> Option<usize>) {
    if let Some((a, b)) = *range {
        *range = match ((a..=b).find_map(map), (a..=b).rev().find_map(map)) {
            (Some(a), Some(b)) if a <= b => Some((a, b)),
            _ => None,
        };
    }
}

#[test]
fn row_edits_preserve_explicit_inherited_hidden_and_disabled_ranges() {
    for book_index in [0, 1] {
        for action in ["insert", "insert_past_data", "delete", "delete_all"] {
            let (mut p, a, b, _) = project();
            let book = if book_index == 0 { a } else { b };
            let before = p.clone();
            let mut legacy = before.clone();
            let map = |r| match action {
                "insert" => Some(if r >= 1 { r + 2 } else { r }),
                "insert_past_data" => Some(if r >= 100 { r + 2 } else { r }),
                "delete" => {
                    if r < 2 {
                        Some(r)
                    } else if r > 4 {
                        Some(r - 3)
                    } else {
                        None
                    }
                }
                "delete_all" => {
                    if r > 200 {
                        Some(r - 201)
                    } else {
                        None
                    }
                }
                _ => unreachable!(),
            };
            let mut history = History::default();
            history.begin(&p, action);
            match action {
                "insert" => p.insert_rows(book, 1, 2),
                "insert_past_data" => p.insert_rows(book, 100, 2),
                "delete" => p.delete_rows(book, 2, 4),
                "delete_all" => p.delete_rows(book, 0, 200),
                _ => unreachable!(),
            }
            legacy.books = p.books.clone();
            for graph in &mut legacy.graphs {
                for layer in graph.page.layers_mut() {
                    let main = layer.book_id;
                    if main == Some(book) {
                        shift(&mut layer.rows, &map);
                    }
                    for (key, range) in &mut layer.spec.row_by_y {
                        if DataRef::from_key(main, key).is_some_and(|r| r.book == book) {
                            shift(range, &map);
                        }
                    }
                }
            }
            check_files_and_svg(&p, &legacy);
            history.changed();
            history.commit(&p);
            if p != before {
                assert_eq!(history.undo(&p).unwrap(), before);
                assert_eq!(history.redo(&before).unwrap(), p);
            }
        }
    }
}

#[test]
fn removing_books_preserves_survivor_descriptions_styles_and_empty_editor_state() {
    let (mut p, a, b, gid) = project();
    let layer = p.graph_mut(gid).unwrap().first_layer_mut();
    layer.x_by_y.remove(&DataRef::new(b, "C").key(Some(a)));
    let before = p.clone();
    let surviving: Vec<_> = before
        .graph(gid)
        .unwrap()
        .page
        .layers()
        .iter()
        .map(|layer| {
            layer
                .plots(&|id| before.book(id))
                .into_iter()
                .filter(|p| p.mapping.y.book == b)
                .collect::<Vec<_>>()
        })
        .collect();
    let mut history = History::default();
    history.begin(&p, "删除主表");
    p.remove(a);
    for (layer, expected) in p.graph(gid).unwrap().page.layers().iter().zip(&surviving) {
        assert_eq!(layer.book_id, Some(b));
        let plots = layer.plots(&|id| p.book(id));
        assert_eq!(plots.len(), expected.len());
        for (actual, expected) in plots.iter().zip(expected) {
            assert_eq!(actual.mapping, expected.mapping);
            assert_eq!(actual.style, expected.style);
            assert_eq!(actual.key, actual.mapping.y.key(Some(b)));
        }
    }
    assert!(p.graph(gid).unwrap().first_layer().labels.column.is_none());
    assert!(p
        .graph(gid)
        .unwrap()
        .first_layer()
        .plots(&|id| p.book(id))
        .iter()
        .all(|p| p.x_override));
    history.changed();
    history.commit(&p);
    assert_eq!(history.undo(&p).unwrap(), before);
    assert_eq!(history.redo(&before).unwrap(), p);
    let graph_before = p.graph(gid).unwrap().clone();
    p.remove(b);
    let mut expected = graph_before;
    for layer in expected.page.layers_mut() {
        layer.book_id = None;
    }
    assert_eq!(p.graph(gid).unwrap(), &expected);
    #[cfg(feature = "serde")]
    assert!(scicanvas_core::project_file::to_json(&p).is_ok());
    check_roundtrip(&p);
}

#[test]
fn panel_grouping_messages_and_sample_match_the_legacy_construction() {
    for panels in Panels::ALL {
        for kind in [
            PlotType::LineSymbol,
            PlotType::StackedBar,
            PlotType::HorizontalBar,
            PlotType::Area,
        ] {
            let mut p = Project::default();
            let book = p.new_book();
            for _ in 0..4 {
                p.book_mut(book).unwrap().push_column(Role::Y);
            }
            for (ci, col) in p.book_mut(book).unwrap().columns.iter_mut().enumerate() {
                col.data = (0..10).map(|r| (r + ci).to_string()).collect();
            }
            let spec = PlotSpec::from_selection(p.book(book).unwrap(), &[], kind).unwrap();
            let (gid, message) = p.plot_panels(book, &[], kind, panels).unwrap();
            let actual = p.graph(gid).unwrap();
            let mut legacy = Graph::new(gid, &actual.name);
            let n = panels.count();
            let mut start = 0;
            for i in 0..n {
                let len = spec.y_keys.len() / n + usize::from(i < spec.y_keys.len() % n);
                let layer = Layer {
                    spec: PlotSpec {
                        y_keys: spec.y_keys[start..start + len].to_vec(),
                        ..spec.clone()
                    },
                    ..Default::default()
                };
                if i == 0 {
                    *legacy.first_layer_mut() = layer;
                } else {
                    legacy.page.add_layer(layer);
                }
                start += len;
            }
            legacy.page.columns = panels.columns();
            (legacy.page.width_mm, legacy.page.height_mm) = panels.page_mm();
            assert_eq!(*actual, legacy);
            assert_eq!(message, format!("已用 Book1 的 5 个 Y 列绘制{}", panels.label()));
            let mut control = p.clone();
            *control.graph_mut(gid).unwrap() = legacy;
            check_files_and_svg(&p, &control);
            let before = p.clone();
            assert!(p.plot_panels(book, &[], PlotType::FloatingBar, panels).is_err());
            assert_eq!(p, before);
        }
    }
    let mut p = Project::sample();
    let mut legacy = p.clone();
    legacy.graphs[0].first_layer_mut().spec = PlotSpec {
        book_id: Some(1),
        x_key: Some("A".into()),
        y_keys: vec!["B".into(), "C".into()],
        ..Default::default()
    };
    check_files_and_svg(&p, &legacy);
    let (gid, message) = p
        .plot_from_selection(1, &["C".into(), "A".into(), "B".into()], PlotType::LineSymbol)
        .unwrap();
    assert_eq!(
        p.graph(gid).unwrap().first_layer().spec,
        legacy.graphs[0].first_layer().spec
    );
    assert_eq!(message, "已用 Book1 的 B, C 列绘制点线图，X = A");
}

fn floating_project() -> (Project, u64, u64, u64) {
    let mut p = Project::default();
    let a = p.new_book();
    let b = p.new_book();
    for id in [a, b] {
        let book = p.book_mut(id).unwrap();
        for _ in 0..3 {
            book.push_column(Role::Y);
        }
        for (i, col) in book.columns.iter_mut().enumerate() {
            col.data = vec![(i + 1).to_string(), (i + 2).to_string()].into();
        }
    }
    let keys = ["B".into(), "C".into(), "D".into(), "E".into()];
    let (gid, _) = p.plot_from_selection(a, &keys, PlotType::FloatingBar).unwrap();
    p.add_curves(gid, 0, b, &keys).unwrap();
    (p, a, b, gid)
}

fn assert_missing_bar(p: &Project, gid: u64, source: u64) {
    let layer = p.graph(gid).unwrap().first_layer();
    let key = DataRef::new(source, "C").key(layer.book_id);
    assert_eq!(layer.base_by_y[&key], "");
    let plot = layer.plot(&DataRef::new(source, "C"), &|id| p.book(id)).unwrap();
    assert!(plot.mapping.base.is_none());
    plot.mapping.validate().unwrap();
    let series = layer
        .series_in(p.book(layer.book_id.unwrap()).unwrap(), &|id| p.book(id))
        .1;
    assert!(!series.iter().any(|s| s.key == key));
    assert!(series
        .iter()
        .any(|s| s.key == DataRef::new(source, "E").key(layer.book_id)));
}

#[test]
fn deleted_floating_base_stays_hidden_through_reuse_rebinding_and_history() {
    for foreign in [false, true] {
        let (mut p, a, b, gid) = floating_project();
        let source = if foreign { b } else { a };
        let before = p.clone();
        let mut history = History::default();
        history.begin(&p, "删下端列");
        p.delete_columns(source, &["B".into()]);
        history.changed();
        history.commit(&p);
        assert_missing_bar(&p, gid, source);
        let svg = svg::render_page(p.graph(gid).unwrap(), &p);
        let undone = history.undo(&p).unwrap();
        assert_eq!(undone, before);
        let redone = history.redo(&undone).unwrap();
        assert_eq!(redone, p);
        assert_eq!(svg::render_page(redone.graph(gid).unwrap(), &redone), svg);
        let book = p.book_mut(source).unwrap();
        assert_eq!(book.push_column(Role::Y), "B");
        book.columns.iter_mut().find(|c| c.key == "B").unwrap().data = vec!["100".into(), "200".into()].into();
        assert_missing_bar(&p, gid, source);
        assert_eq!(svg::render_page(p.graph(gid).unwrap(), &p), svg);
        // 改样式仍可使用统一访问接口，且保留缺失标记。
        let books = &p.books;
        p.graphs
            .iter_mut()
            .find(|g| g.id == gid)
            .unwrap()
            .first_layer_mut()
            .plot_mut(
                &DataRef::new(source, "C"),
                &|id| books.iter().find(|b| b.id == id),
                |plot| {
                    plot.style = Some(CurveStyle {
                        right_axis: true,
                        ..Default::default()
                    });
                },
            )
            .unwrap();
        assert_missing_bar(&p, gid, source);
        if foreign {
            p.remove(a);
            assert_eq!(p.graph(gid).unwrap().first_layer().book_id, Some(b));
            assert_missing_bar(&p, gid, source);
        }
        check_roundtrip(&p);
    }
}

#[cfg(feature = "serde")]
#[test]
fn missing_floating_base_saves_to_both_formats_even_after_switching_plot_type() {
    use scicanvas_core::project_file;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/file-tests/missing-floating-base")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    for foreign in [false, true] {
        for kind in [PlotType::FloatingBar, PlotType::Line] {
            let (mut p, a, b, gid) = floating_project();
            let source = if foreign { b } else { a };
            p.delete_columns(source, &["B".into()]);
            p.graph_mut(gid).unwrap().first_layer_mut().set_plot_type(kind).unwrap();
            let expected = svg::render_page(p.graph(gid).unwrap(), &p);
            for ext in ["scproj", "scv"] {
                let path = dir.join(format!("project.{ext}"));
                project_file::save(&path, &p).unwrap();
                let mut restored = project_file::load(&path).unwrap();
                assert_eq!(restored, p);
                assert_eq!(svg::render_page(restored.graph(gid).unwrap(), &restored), expected);
                restored
                    .graph_mut(gid)
                    .unwrap()
                    .first_layer_mut()
                    .set_plot_type(PlotType::FloatingBar)
                    .unwrap();
                assert_missing_bar(&restored, gid, source);
                assert_eq!(restored.book_mut(source).unwrap().push_column(Role::Y), "B");
                assert_missing_bar(&restored, gid, source);
            }
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn missing_base_legacy_states_migrate_and_conflicting_references_are_rejected() {
    use scicanvas_core::project_file;
    let (mut p, a, _, gid) = floating_project();
    p.delete_columns(a, &["B".into()]);
    let expected = svg::render_page(p.graph(gid).unwrap(), &p);
    // 旧平行表中的空下端不是合法 DataRef，但它是既有的隐藏标记。
    for version in 5..=13 {
        let mut wire = serde_json::to_value(&p).unwrap();
        wire["version"] = version.into();
        let restored = project_file::from_json(&wire.to_string()).unwrap();
        assert_missing_bar(&restored, gid, a);
        assert_eq!(svg::render_page(restored.graph(gid).unwrap(), &restored), expected);
        check_roundtrip(&restored);
    }
    let json = project_file::to_json(&p).unwrap();
    let mut wire: serde_json::Value = serde_json::from_str(&json).unwrap();
    let plot = &mut wire["graphs"][0]["page"]["layers"][0]["plots"][0];
    assert_eq!(plot["base_missing"], true);
    assert!(plot["mapping"]["base"].is_null());
    plot["mapping"]["base"] = serde_json::json!({"book": a, "col": "D"});
    assert!(project_file::from_json(&wire.to_string()).is_err());
    wire["graphs"][0]["page"]["layers"][0]["plots"][0]["base_missing"] = false.into();
    wire["graphs"][0]["page"]["layers"][0]["plots"][0]["mapping"]["base"]["col"] = "".into();
    assert!(project_file::from_json(&wire.to_string()).is_err());
    // 旧版类型化曲线没有新字段，普通下端映射保持原义。
    let (p, _, _, _) = floating_project();
    let mut wire: serde_json::Value = serde_json::from_str(&project_file::to_json(&p).unwrap()).unwrap();
    wire["version"] = 13.into();
    let mut restored = project_file::from_json(&wire.to_string()).unwrap();
    restored.version = p.version;
    assert_eq!(restored, p);
}
