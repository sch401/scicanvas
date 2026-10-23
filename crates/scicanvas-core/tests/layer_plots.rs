//! 统一曲线入口与旧平行表、保存格式和事务的兼容性。
use scicanvas_core::page::{CurveStyle, Layer};
use scicanvas_core::plotspec::{CurveMapping, DataRef, Plot, PlotSpec};
use scicanvas_core::{history::History, svg, PlotType, Project, Role};

fn project() -> (Project, u64) {
    let mut p = Project::sample();
    let foreign = p.new_book();
    let b = p.book_mut(foreign).unwrap();
    b.columns[0].data = (0..6).map(|i| i.to_string()).collect();
    b.columns[1].data = (0..6).map(|i| (10 + i).to_string()).collect();
    let c = b.push_column(Role::Y);
    b.columns.iter_mut().find(|col| col.key == c).unwrap().data = (0..6).map(|i| (20 + i).to_string()).collect();
    p.add_curves(2, 0, foreign, &["B".into()]).unwrap();
    (p, foreign)
}

fn descriptor(book: u64, col: &str) -> Plot {
    Plot {
        key: "由来源生成".into(),
        mapping: CurveMapping {
            y: DataRef::new(book, col),
            x: Some(DataRef::new(book, "A")),
            base: None,
            rows: None,
            line_only: false,
        },
        style: None,
        x_override: false,
    }
}

fn roundtrip(p: &Project) {
    #[cfg(feature = "serde")]
    {
        use scicanvas_core::project_file;
        let json = project_file::to_json(p).unwrap();
        assert_eq!(project_file::from_json(&json).unwrap(), *p);
        assert_eq!(project_file::from_scv(&project_file::to_scv(p).unwrap()).unwrap(), *p);
    }
    #[cfg(not(feature = "serde"))]
    let _ = p;
}

#[test]
fn typed_lookup_preserves_order_and_distinguishes_books_and_at_signs() {
    let (mut p, foreign) = project();
    let b = p.book_mut(foreign).unwrap();
    let key = b.push_column(Role::Y);
    b.columns.iter_mut().find(|c| c.key == key).unwrap().key = "signal@2".into();
    let books = &p.books;
    let lookup = |id| books.iter().find(|b| b.id == id);
    let layer = p.graphs[0].first_layer_mut();
    assert!(layer.add_plot(descriptor(foreign, "signal@2"), &lookup).unwrap());
    assert_eq!(layer.plots(&lookup), layer.plots_in(&lookup));
    assert_eq!(
        layer
            .plots(&lookup)
            .iter()
            .map(|p| p.mapping.y.clone())
            .collect::<Vec<_>>(),
        [
            DataRef::new(1, "B"),
            DataRef::new(1, "C"),
            DataRef::new(foreign, "B"),
            DataRef::new(foreign, "signal@2")
        ]
    );
    for source in [
        DataRef::new(1, "B"),
        DataRef::new(foreign, "B"),
        DataRef::new(foreign, "signal@2"),
    ] {
        assert_eq!(layer.plot(&source, &lookup).unwrap().mapping.y, source);
    }
    assert!(layer.plot(&DataRef::new(999, "B"), &lookup).is_none());
    assert_eq!(
        layer.plot(&DataRef::new(foreign, "signal@2"), &lookup).unwrap().key,
        format!("signal@2@{foreign}")
    );
    let mut snapshot = layer.plot(&DataRef::new(1, "B"), &lookup).unwrap();
    snapshot.style = Some(CurveStyle {
        right_axis: true,
        ..Default::default()
    });
    assert!(layer.plot(&DataRef::new(1, "B"), &lookup).unwrap().style.is_none());
    roundtrip(&p);
}

#[test]
fn no_op_and_style_edits_keep_explicit_and_inherited_defaults_distinct() {
    let (mut p, _) = project();
    let books = &p.books;
    let lookup = |id| books.iter().find(|b| b.id == id);
    let layer = p.graphs[0].first_layer_mut();
    layer.rows = Some((1, 3));
    layer.x_by_y.insert("B".into(), "A".into());
    layer.row_by_y.insert("B".into(), Some((1, 3)));
    let before = layer.clone();
    assert!(!layer.plot_mut(&DataRef::new(1, "B"), &lookup, |_| {}).unwrap());
    assert_eq!(*layer, before);
    assert!(layer
        .plot_mut(&DataRef::new(1, "B"), &lookup, |plot| {
            plot.style = Some(CurveStyle {
                color: Some([23, 45, 67]),
                right_axis: true,
                ..Default::default()
            });
        })
        .unwrap());
    assert_eq!(layer.x_by_y, before.x_by_y);
    assert_eq!(layer.row_by_y, before.row_by_y);
    assert!(layer.plot(&DataRef::new(1, "C"), &lookup).unwrap().style.is_none());
    layer.rows = Some((2, 4));
    assert_eq!(
        layer.plot(&DataRef::new(1, "B"), &lookup).unwrap().mapping.rows,
        Some((1, 3))
    );
    assert_eq!(
        layer.plot(&DataRef::new(1, "C"), &lookup).unwrap().mapping.rows,
        Some((2, 4))
    );
    assert!(layer.plot(&DataRef::new(1, "B"), &lookup).unwrap().x_override);
    roundtrip(&p);
}

#[test]
fn floating_and_derived_mapping_edits_write_through_all_old_tables() {
    let mut p = Project::sample();
    let books = &p.books;
    let lookup = |id| books.iter().find(|b| b.id == id);
    let layer = p.graphs[0].first_layer_mut();
    layer.spec = PlotSpec::from_selection(&books[0], &["B".into(), "C".into()], PlotType::FloatingBar).unwrap();
    layer.rows = Some((1, 3));
    let source = DataRef::new(1, "C");
    assert!(layer
        .plot_mut(&source, &lookup, |plot| {
            plot.mapping.x = Some(DataRef::new(1, "B"));
            plot.mapping.base = Some(DataRef::new(1, "A"));
            plot.mapping.rows = None;
            plot.style = Some(CurveStyle {
                right_axis: true,
                ..Default::default()
            });
        })
        .unwrap());
    assert_eq!(layer.x_by_y["C"], "B");
    assert_eq!(layer.base_by_y["C"], "A");
    assert_eq!(layer.row_by_y["C"], None);
    assert!(layer.curves["C"].right_axis);
    assert!(layer.plot(&source, &lookup).unwrap().x_override);
    assert!(layer
        .plot_mut(&source, &lookup, |plot| {
            plot.mapping.x = Some(DataRef::new(1, "A"));
            plot.x_override = false;
            plot.mapping.base = None;
            plot.mapping.line_only = true;
            plot.style = None;
        })
        .unwrap());
    assert!(layer.x_by_y.is_empty() && layer.base_by_y.is_empty());
    assert!(layer.row_by_y.is_empty() && layer.curves.is_empty());
    assert_eq!(layer.line_only, ["C"]);
    assert_eq!(layer.rows, Some((1, 3)));
    let plot = layer.plot(&source, &lookup).unwrap();
    assert!(plot.mapping.line_only && plot.mapping.rows.is_none() && !plot.x_override);
    roundtrip(&p);
}

#[test]
fn additions_normalize_keys_deduplicate_sources_and_replace_stale_entries() {
    let (p, foreign) = project();
    let lookup = |id| p.book(id);
    let mut layer = Layer {
        spec: PlotSpec {
            book_id: Some(1),
            x_key: Some("A".into()),
            rows: Some((1, 3)),
            ..Default::default()
        },
        labels: scicanvas_core::page::DataLabels {
            column: Some("C".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let key = DataRef::new(foreign, "B").key(layer.book_id);
    layer.x_by_y.insert(key.clone(), format!("C@{foreign}"));
    layer.base_by_y.insert(key.clone(), format!("C@{foreign}"));
    layer.row_by_y.insert(key.clone(), Some((0, 1)));
    layer.line_only.push(key.clone());
    layer.curves.insert(
        key.clone(),
        CurveStyle {
            right_axis: true,
            ..Default::default()
        },
    );
    assert!(layer.add_plot(descriptor(foreign, "B"), &lookup).unwrap());
    assert_eq!(layer.y_keys, [key]);
    assert!(layer.x_by_y.is_empty() && layer.base_by_y.is_empty() && layer.row_by_y.is_empty());
    assert!(layer.line_only.is_empty() && layer.curves.is_empty());
    let before = layer.clone();
    let mut duplicate = descriptor(foreign, "B");
    duplicate.key = "另一个旧键".into();
    duplicate.style = Some(CurveStyle {
        right_axis: true,
        ..Default::default()
    });
    assert!(!layer.add_plot(duplicate, &lookup).unwrap());
    assert_eq!(layer, before);
    assert_eq!(
        (
            layer.book_id,
            layer.x_key.as_deref(),
            layer.rows,
            layer.labels.column.as_deref()
        ),
        (Some(1), Some("A"), Some((1, 3)), Some("C"))
    );
    let mut row_numbers = Layer {
        spec: PlotSpec {
            book_id: Some(1),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut plot = descriptor(1, "B");
    plot.mapping.x = None;
    assert!(row_numbers.add_plot(plot, &lookup).unwrap());
    assert!(row_numbers
        .plot(&DataRef::new(1, "B"), &lookup)
        .unwrap()
        .mapping
        .x
        .is_none());
    let mut unbound = Layer {
        spec: PlotSpec {
            y_keys: vec!["B".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    let before = unbound.clone();
    assert!(unbound.plots(&lookup).is_empty());
    assert_eq!(unbound.retain_plots(&lookup, |_| false), 0);
    assert_eq!(unbound, before);
}

#[test]
fn removal_and_retention_clear_every_table_without_affecting_other_sources() {
    let (mut p, foreign) = project();
    let books = &p.books;
    let lookup = |id| books.iter().find(|b| b.id == id);
    let layer = p.graphs[0].first_layer_mut();
    layer.y_keys.insert(1, "B@1".into());
    for key in layer.y_keys.clone() {
        let source = DataRef::from_key(layer.book_id, &key).unwrap();
        let x = DataRef::new(source.book, "A").key(layer.book_id);
        layer.x_by_y.insert(key.clone(), x.clone());
        layer.base_by_y.insert(key.clone(), x);
        layer.row_by_y.insert(key.clone(), Some((0, 1)));
        layer.line_only.push(key.clone());
        layer.curves.insert(
            key,
            CurveStyle {
                right_axis: true,
                ..Default::default()
            },
        );
    }
    let kept = layer.plot(&DataRef::new(foreign, "B"), &lookup).unwrap();
    assert!(layer.remove_plot(&DataRef::new(1, "B")));
    for key in ["B", "B@1"] {
        assert!(!layer.y_keys.iter().any(|k| k == key));
        assert!(!layer.line_only.iter().any(|k| k == key));
        assert!(!layer.x_by_y.contains_key(key) && !layer.base_by_y.contains_key(key));
        assert!(!layer.row_by_y.contains_key(key) && !layer.curves.contains_key(key));
    }
    let before = layer.clone();
    assert!(!layer.remove_plot(&DataRef::new(1, "B")));
    assert_eq!(*layer, before);
    assert_eq!(layer.retain_plots(&lookup, |p| p.mapping.y.book == foreign), 1);
    assert_eq!(layer.plots(&lookup), [kept]);
    assert_eq!(layer.y_keys.len(), 1);
    assert_eq!(layer.line_only.len(), 1);
    assert_eq!(layer.x_by_y.len(), 1);
    assert_eq!(layer.base_by_y.len(), 1);
    assert_eq!(layer.row_by_y.len(), 1);
    assert_eq!(layer.curves.len(), 1);
    roundtrip(&p);
}

#[test]
fn invalid_edits_and_missing_sources_leave_the_whole_layer_unchanged() {
    let (mut p, foreign) = project();
    let books = &p.books;
    let lookup = |id| books.iter().find(|b| b.id == id);
    let layer = p.graphs[0].first_layer_mut();
    let source = DataRef::new(1, "B");
    let invalid: [fn(&mut Plot); 4] = [
        |p| p.mapping.y.col = "C".into(),
        |p| p.key = "C".into(),
        |p| p.mapping.rows = Some((3, 1)),
        |p| {
            p.mapping.x = None;
            p.style = Some(CurveStyle {
                right_axis: true,
                ..Default::default()
            });
        },
    ];
    for edit in invalid {
        let before = layer.clone();
        assert!(layer.plot_mut(&source, &lookup, edit).is_err());
        assert_eq!(*layer, before);
    }
    let before = layer.clone();
    assert!(layer
        .plot_mut(&source, &lookup, |p| p.mapping.x = Some(DataRef::new(foreign, "A")))
        .is_err());
    let mut bad = descriptor(foreign, "C");
    bad.mapping.rows = Some((0, usize::MAX));
    assert!(layer.add_plot(bad, &lookup).is_err());
    let mut called = false;
    assert!(layer
        .plot_mut(&DataRef::new(1, "missing"), &lookup, |_| called = true)
        .is_err());
    assert!(!called);
    assert_eq!(*layer, before);
}

#[test]
fn new_entries_match_legacy_rendering_history_and_current_persistence() {
    let (mut p, foreign) = project();
    let before = p.clone();
    let mut legacy = before.clone();
    let style = CurveStyle {
        right_axis: true,
        color: Some([130, 30, 90]),
        line_width: Some(2.0),
        ..Default::default()
    };
    let mut history = History::default();
    history.begin(&p, "曲线访问入口");
    {
        let books = &p.books;
        let lookup = |id| books.iter().find(|b| b.id == id);
        let layer = p.graphs[0].first_layer_mut();
        assert!(layer
            .plot_mut(&DataRef::new(foreign, "B"), &lookup, |plot| {
                plot.mapping.rows = Some((1, 3));
                plot.style = Some(style.clone());
            })
            .unwrap());
        let mut derived = descriptor(foreign, "C");
        derived.mapping.line_only = true;
        derived.x_override = true;
        derived.style = Some(CurveStyle {
            right_axis: true,
            ..Default::default()
        });
        assert!(layer.add_plot(derived, &lookup).unwrap());
        assert_eq!(layer.retain_plots(&lookup, |p| p.mapping.y != DataRef::new(1, "C")), 1);
    }
    history.changed();
    history.commit(&p);
    let layer = legacy.graphs[0].first_layer_mut();
    let key = DataRef::new(foreign, "B").key(layer.book_id);
    layer.row_by_y.insert(key.clone(), Some((1, 3)));
    layer.curves.insert(key, style);
    layer.add_analysis_result(foreign, "B", "C", Some("A"));
    layer.remove_curve("C");
    assert_eq!(p, legacy);
    assert_eq!(
        svg::render_page(&p.graphs[0], &p),
        svg::render_page(&legacy.graphs[0], &legacy)
    );
    assert_eq!(history.undo(&p).unwrap(), before);
    assert_eq!(history.redo(&before).unwrap(), p);
    #[cfg(feature = "serde")]
    {
        use scicanvas_core::project_file;
        let json = project_file::to_json(&p).unwrap();
        assert_eq!(json, project_file::to_json(&legacy).unwrap());
        let wire: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(wire["version"], scicanvas_core::project_file::CURRENT_VERSION);
        assert!(wire["graphs"][0]["page"]["layers"][0].get("y_keys").is_none());
        assert_eq!(
            wire["graphs"][0]["page"]["layers"][0]["plots"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }
    roundtrip(&p);
}

fn compare_editor_state(p: &Project, legacy: &Project) {
    assert_eq!(p, legacy);
    assert_eq!(
        svg::render_page(&p.graphs[0], p),
        svg::render_page(&legacy.graphs[0], legacy)
    );
    #[cfg(feature = "serde")]
    {
        use scicanvas_core::project_file;
        let json = project_file::to_json(p).unwrap();
        assert_eq!(json, project_file::to_json(legacy).unwrap());
        let saved = project_file::from_json(&json).unwrap();
        assert_eq!(project_file::to_json(&saved).unwrap(), json);
        assert_eq!(
            project_file::from_scv(&project_file::to_scv(p).unwrap()).unwrap(),
            saved
        );
    }
}

#[test]
fn editor_selection_and_order_preserve_inactive_unbound_and_missing_base_states() {
    for bound in [false, true] {
        let (mut p, foreign) = project();
        let layer = p.graphs[0].first_layer_mut();
        if !bound {
            layer.book_id = None;
        }
        let key = format!("B@{foreign}");
        layer.line_only.push(key.clone());
        layer.base_by_y.insert("B".into(), String::new());
        layer.x_by_y.insert("B".into(), "A".into());
        layer.row_by_y.insert("B".into(), None);
        layer.curves.insert(
            "B".into(),
            CurveStyle {
                right_axis: true,
                ..Default::default()
            },
        );
        let before = p.clone();
        let mut legacy = p.clone();
        let mut history = History::default();
        history.begin(&p, "选择曲线");
        p.graphs[0]
            .first_layer_mut()
            .edit_plot_keys(|keys| keys.retain(|key| key != "B"));
        legacy.graphs[0].first_layer_mut().spec.y_keys.retain(|key| key != "B");
        compare_editor_state(&p, &legacy);
        assert!(p.graphs[0].first_layer().has_right_axis_style());
        p.graphs[0].first_layer_mut().edit_plot_keys(|keys| {
            keys.push("B".into());
            keys.swap(0, 1);
            keys.sort();
        });
        let old = legacy.graphs[0].first_layer_mut();
        old.spec.y_keys.push("B".into());
        old.spec.y_keys.swap(0, 1);
        old.spec.y_keys.sort();
        compare_editor_state(&p, &legacy);
        let layer = p.graphs[0].first_layer();
        assert_eq!(layer.plot_keys(true), layer.y_keys);
        assert_eq!(
            layer.plot_keys(false),
            layer
                .y_keys
                .iter()
                .filter(|key| !layer.line_only.contains(key))
                .cloned()
                .collect::<Vec<_>>()
        );
        assert_eq!(layer.base_by_y["B"], "");
        assert_eq!(layer.row_by_y["B"], None);
        history.changed();
        history.commit(&p);
        let undone = history.undo(&p).unwrap();
        assert_eq!(undone, before);
        assert_eq!(history.redo(&undone).unwrap(), p);
    }
}

#[test]
fn editor_foreign_addition_preserves_pending_mappings_and_explicit_defaults() {
    for with_x in [false, true] {
        let (mut p, foreign) = project();
        let key = format!("C@{foreign}");
        let layer = p.graphs[0].first_layer_mut();
        layer.x_by_y.insert(key.clone(), format!("A@{foreign}"));
        layer.base_by_y.insert(key.clone(), String::new());
        layer.row_by_y.insert(key.clone(), None);
        layer.line_only.push(key.clone());
        layer.curves.insert(key.clone(), CurveStyle::default());
        let mut legacy = p.clone();
        let x = with_x.then(|| DataRef::new(foreign, "B"));
        p.graphs[0]
            .first_layer_mut()
            .select_plot(&DataRef::new(foreign, "C"), x.as_ref());
        let old = legacy.graphs[0].first_layer_mut();
        old.spec.y_keys.push(key.clone());
        if with_x {
            old.spec.x_by_y.insert(key.clone(), format!("B@{foreign}"));
        }
        compare_editor_state(&p, &legacy);
        let layer = p.graphs[0].first_layer();
        let plot = layer.plot(&DataRef::new(foreign, "C"), &|id| p.book(id)).unwrap();
        assert!(plot.x_override && plot.mapping.line_only);
        assert_eq!(plot.mapping.rows, None);
        assert_eq!(plot.style, Some(CurveStyle::default()));
        assert_eq!(layer.base_by_y[&key], "");
    }
}

#[test]
fn editor_styles_share_storage_without_changing_pending_or_unbound_mappings() {
    let (mut p, _) = project();
    p.graphs[0].first_layer_mut().book_id = None;
    // 未绑定的编辑条目及尚未完整的映射仍允许只改样式。
    p.graphs[0]
        .first_layer_mut()
        .base_by_y
        .insert("B".into(), String::new());
    for key in ["B", "hidden"] {
        for style in [
            Some(CurveStyle::default()),
            Some(CurveStyle {
                right_axis: true,
                legend_label: "β^2".into(),
                ..Default::default()
            }),
            None,
        ] {
            let mut legacy = p.clone();
            p.graphs[0].first_layer_mut().set_plot_style(key, style.clone());
            let old = legacy.graphs[0].first_layer_mut();
            if let Some(style) = style.clone() {
                old.curves.insert(key.into(), style);
            } else {
                old.curves.remove(key);
            }
            compare_editor_state(&p, &legacy);
            assert_eq!(p.graphs[0].first_layer().plot_style(key), style.as_ref());
        }
    }
}

#[test]
fn builder_base_and_cache_references_preserve_hidden_and_foreign_editor_state() {
    let (p, foreign) = project();
    for bound in [None, Some(1)] {
        for explicit_x in [None, Some(""), Some("A")] {
            let mut layer = p.graphs[0].first_layer().clone();
            layer.book_id = bound;
            layer.labels.column = Some("C".into());
            layer.base_by_y.insert("C".into(), String::new());
            layer.x_by_y.insert("hidden".into(), format!("C@{foreign}"));
            layer.base_by_y.insert("hidden".into(), "B".into());
            let key = format!("B@{foreign}");
            if let Some(x) = explicit_x {
                layer.x_by_y.insert(
                    key,
                    if x.is_empty() {
                        String::new()
                    } else {
                        format!("{x}@{foreign}")
                    },
                );
            } else {
                layer.x_by_y.remove(&key);
            }
            for book in &p.books {
                for col in &book.columns {
                    let local = bound == Some(book.id);
                    let qualified = format!("{}@{}", col.key, book.id);
                    let matches = |x: &str| x == qualified || (local && x == col.key);
                    let old_default =
                        layer
                            .y_keys
                            .iter()
                            .any(|raw_y| match scicanvas_core::plotspec::split_key(raw_y) {
                                (y, Some(id)) if id == book.id && !layer.x_by_y.contains_key(raw_y) => {
                                    scicanvas_core::plotspec::default_x(book, y).as_deref() == Some(&col.key)
                                }
                                _ => false,
                            });
                    let expected = (local && layer.x_key.as_deref() == Some(&col.key))
                        || layer.y_keys.iter().any(|y| matches(y))
                        || layer.x_by_y.values().any(|x| matches(x))
                        || layer.base_by_y.values().any(|b| matches(b))
                        || (local && layer.labels.column.as_deref() == Some(&col.key))
                        || (!local && old_default);
                    assert_eq!(layer.references_plot_column(book, &col.key), expected);
                }
            }
            for key in ["C", "hidden", "not-selected"] {
                assert_eq!(layer.plot_base_key(key), layer.base_by_y.get(key).map(String::as_str));
                let mut legacy = layer.clone();
                layer.set_plot_base_key(key, "A".into());
                legacy.spec.base_by_y.insert(key.into(), "A".into());
                assert_eq!(layer, legacy);
                layer.set_plot_base_key(key, String::new());
                legacy.spec.base_by_y.insert(key.into(), String::new());
                assert_eq!(layer, legacy);
            }
        }
    }
}
