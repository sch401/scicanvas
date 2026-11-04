use scicanvas_core::{
    colormap::ColorMap,
    coord::{Cartesian, Coord},
    page::Layer,
    plotspec::{PlotSpec, Transform},
    scene, svg, Book, Column, Graph, PlotType, Project, Role,
};

#[test]
fn page_text_and_manual_legends_keep_page_positions_when_axes_zoom() {
    use scene::ObjectTarget as T;
    let mut p = Project::sample();
    let automatic = scene::layout_page(&p.graphs[0], &p, &[]);
    let id = p.graphs[0].page.add_text([0.3, 0.2]).unwrap();
    let text = p.graphs[0].page.text_mut(id).unwrap();
    text.text = "CO_{2} α\n\\i{k}".into();
    text.size_pt = 11.0;
    text.color = [20, 30, 40];
    p.graphs[0].first_layer_mut().legend_position = Some([0.05, 0.05]);
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert_eq!(sc.plot, automatic.plot, "moving objects does not move data/axes");
    for object in &sc.objects {
        assert!(object.items.start < object.items.end && object.items.end <= sc.items.len());
        assert_eq!(
            sc.object_at(object.bounds[0] + 2.0, object.bounds[1] + 2.0),
            Some(object.target)
        );
    }
    let text = sc.objects.iter().find(|o| o.target == T::Text(id)).unwrap();
    assert!((text.bounds[0] / sc.w - 0.3).abs() < 1e-9);
    assert!((text.bounds[1] / sc.h - 0.2).abs() < 1e-9);
    assert!(matches!(
        sc.items[text.items.start].prim,
        scene::Prim::Text {
            rich: true,
            color: (20, 30, 40),
            ..
        }
    ));
    let zoomed = scene::layout_page(
        &p.graphs[0],
        &p,
        &[Some(scene::View {
            x: (2.0, 8.0),
            y: (-2.0, 6.0),
        })],
    );
    assert_ne!(sc.axes, zoomed.axes);
    for object in &sc.objects {
        let after = zoomed.objects.iter().find(|o| o.target == object.target).unwrap();
        assert_eq!(&object.bounds[..2], &after.bounds[..2]);
    }
    let svg = svg::render_page(&p.graphs[0], &p);
    assert!(svg.contains("CO2 α") && svg.contains("#141e28"));
    p.graphs[0].first_layer_mut().legend_position = None;
    p.graphs[0].page.annotations.clear();
    assert_eq!(
        scene::layout_page(&p.graphs[0], &p, &[]).items,
        automatic.items,
        "automatic avoidance is restored exactly"
    );
    let next = p.graphs[0].page.add_text([0.4, 0.3]).unwrap();
    assert!(next > id, "deleted text IDs are not reused");
}

#[test]
fn right_y_axis_range_and_number_format_are_independent() {
    use scicanvas_core::{format::NumberFormat, page::CurveStyle};
    let mut project = Project::sample();
    let key = project.graphs[0].first_layer().draft().y_keys[0].clone();
    let layer = project.graphs[0].first_layer_mut();
    layer.set_plot_style(
        &key,
        Some(CurveStyle {
            right_axis: true,
            ..Default::default()
        }),
    );
    layer.y_format = NumberFormat::Decimal { places: 0 };
    layer.y2_auto = false;
    layer.y2_min = 40.0;
    layer.y2_max = 80.0;
    layer.y2_format = Some(NumberFormat::Scientific { places: 1 });

    let graph = &project.graphs[0];
    let scene = scene::layout_page(graph, &project, &[]);
    let right = scene.layers[0].axes.right_y.as_ref().unwrap();
    assert_eq!((right.lo, right.hi), (40.0, 80.0));
    assert!((scene.layers[0].axes.left_y.lo, scene.layers[0].axes.left_y.hi) != (right.lo, right.hi));
    assert!(scene.items.iter().any(|item| matches!(
        &item.prim,
        scene::Prim::Text { text, .. } if text == "4.0E1"
    )));

    let zoomed = scene::layout_page(
        graph,
        &project,
        &[Some(scene::View {
            x: (2.0, 8.0),
            y: (-1.0, 2.0),
        })],
    );
    assert_eq!(
        (
            zoomed.layers[0].axes.right_y.as_ref().unwrap().lo,
            zoomed.layers[0].axes.right_y.as_ref().unwrap().hi
        ),
        (40.0, 80.0)
    );
    assert!(svg::render_page(graph, &project).contains("4.0E1"));
}

#[test]
fn reference_lines_follow_each_axis_and_export_through_the_shared_scene() {
    use scicanvas_core::page::{CurveStyle, ReferenceAxis, ReferenceLine};
    let mut project = Project::sample();
    let key = project.graphs[0].first_layer().draft().y_keys[0].clone();
    let layer = project.graphs[0].first_layer_mut();
    layer.set_plot_style(
        &key,
        Some(CurveStyle {
            right_axis: true,
            ..Default::default()
        }),
    );
    layer.x_auto = false;
    layer.x_min = 1.0;
    layer.x_max = 100.0;
    layer.y_auto = false;
    layer.y_min = 1.0;
    layer.y_max = 100.0;
    layer.y2_auto = false;
    layer.y2_min = 1.0;
    layer.y2_max = 100.0;
    layer.x_log = true;
    layer.y_log = true;
    layer.reference_lines = vec![
        ReferenceLine {
            axis: ReferenceAxis::X,
            value: 10.0,
            color: [200, 0, 0],
            ..Default::default()
        },
        ReferenceLine {
            axis: ReferenceAxis::YLeft,
            value: 10.0,
            color: [0, 200, 0],
            ..Default::default()
        },
        ReferenceLine {
            axis: ReferenceAxis::YRight,
            value: 50.0,
            color: [0, 0, 200],
            ..Default::default()
        },
        ReferenceLine {
            axis: ReferenceAxis::YLeft,
            value: 0.0,
            color: [201, 0, 201],
            ..Default::default()
        },
    ];

    let graph = &project.graphs[0];
    let scene = scene::layout_page(graph, &project, &[]);
    let reference_segments: Vec<_> = scene
        .items
        .iter()
        .filter_map(|item| match &item.prim {
            scene::Prim::Segments {
                segs,
                color,
                dashed: true,
                ..
            } if *color != (217, 217, 217) => Some((*color, segs[0])),
            _ => None,
        })
        .collect();
    assert_eq!(reference_segments.len(), 3);
    let left = scene.axes.cartesian(scene.plot, scicanvas_core::coord::YAxis::Left);
    let right = scene.axes.cartesian(scene.plot, scicanvas_core::coord::YAxis::Right);
    let x = left.project([10.0, scene.axes.left_y.lo])[0];
    let y_left = left.project([scene.axes.x.lo, 10.0])[1];
    let y_right = right.project([scene.axes.x.lo, 50.0])[1];
    assert!(reference_segments
        .iter()
        .any(|(c, s)| *c == (200, 0, 0) && (s[0] - x).abs() < 1e-9 && (s[2] - x).abs() < 1e-9));
    assert!(reference_segments
        .iter()
        .any(|(c, s)| *c == (0, 200, 0) && (s[1] - y_left).abs() < 1e-9 && (s[3] - y_left).abs() < 1e-9));
    assert!(reference_segments
        .iter()
        .any(|(c, s)| *c == (0, 0, 200) && (s[1] - y_right).abs() < 1e-9 && (s[3] - y_right).abs() < 1e-9));
    let svg = svg::render_page(graph, &project);
    assert!(svg.contains("#c80000") && svg.contains("#00c800") && svg.contains("#0000c8"));
    assert!(
        !svg.contains("#c900c9"),
        "nonpositive reference values are skipped on log axes"
    );
}

#[test]
fn column_mappings_drive_point_style_and_independent_symbol_fill_and_border() {
    use scicanvas_core::page::{CurveStyle, SymbolFill};
    let mut project = Project::sample();
    let mapping_key = {
        let book = project.book_mut(1).unwrap();
        let key = book.push_column(Role::Ignore);
        let rows = book.nrows();
        book.columns.iter_mut().find(|column| column.key == key).unwrap().data = (0..rows)
            .map(|row| {
                if row == 1 {
                    String::new()
                } else {
                    scicanvas_core::fmt_num(if row == 0 {
                        -1e308
                    } else if row + 1 == rows {
                        1e308
                    } else {
                        0.0
                    })
                }
            })
            .collect();
        key
    };
    let category_key = {
        let book = project.book_mut(1).unwrap();
        let key = book.push_column(Role::Ignore);
        let rows = book.nrows();
        book.columns.iter_mut().find(|column| column.key == key).unwrap().data =
            (scicanvas_core::ColumnData::Category {
                codes: (0..rows).map(|row| (row % 3) as u32).collect(),
                levels: vec!["Alpha".into(), "Beta".into(), "Gamma".into()],
                ordered: true,
            })
            .into();
        key
    };
    project.graph_mut(2).unwrap().first_layer_mut().set_plot_style(
        "B",
        Some(CurveStyle {
            color_column: Some(mapping_key.clone()),
            size_column: Some(mapping_key),
            symbol_column: Some(category_key),
            size_range: Some([2.0, 10.0]),
            symbol_fill: Some(SymbolFill::Hollow),
            symbol_border_color: Some([20, 30, 40]),
            symbol_border_width: Some(0.8),
            ..Default::default()
        }),
    );
    let graph = project.graph(2).unwrap();
    let book = project.book(1).unwrap();
    let (_, plots) = graph.first_layer().prepared_plots_in(book, &|_| None);
    let plot = plots.iter().find(|plot| plot.series.key == "B").unwrap();
    assert_eq!(plot.series.points.len(), plot.series.point_styles.len());
    assert_eq!(plot.series.point_styles[0].size, Some(2.0));
    assert_eq!(plot.series.point_styles.last().unwrap().size, Some(10.0));
    assert!(plot.series.point_styles[1].color.is_none() && plot.series.point_styles[1].size.is_none());
    assert_eq!(
        plot.series
            .point_styles
            .iter()
            .take(3)
            .map(|style| style.symbol)
            .collect::<Vec<_>>(),
        [
            Some(scicanvas_core::Symbol::Circle),
            Some(scicanvas_core::Symbol::Square),
            Some(scicanvas_core::Symbol::Up),
        ]
    );
    assert!(
        plot.series
            .point_styles
            .iter()
            .filter_map(|style| style.color)
            .collect::<std::collections::HashSet<_>>()
            .len()
            > 1
    );

    let scene = scene::layout_page(graph, &project, &[]);
    assert!(scene.items.iter().any(|item| matches!(
        &item.prim,
        scene::Prim::StyledSymbols { symbols }
            if symbols.iter().any(|symbol| symbol.fill.is_none() && symbol.stroke.is_some())
    )));
    assert!(scene.items.iter().any(|item| matches!(
        &item.prim,
        scene::Prim::ColoredSegments { segments, .. } if segments.iter().map(|segment| segment.color).collect::<std::collections::HashSet<_>>().len() > 1
    )));
    let svg = svg::render_page(graph, &project);
    assert!(svg.contains("fill=\"none\"") && svg.contains("stroke=\"#141e28\""));
    assert!(svg.contains(" · D"), "the legend shows the mapped color column");
}

#[cfg(feature = "serde")]
#[test]
fn v13_objects_roundtrip_and_legacy_defaults_keep_literal_text_and_auto_legends() {
    use scicanvas_core::project_file as file;
    let mut p = Project::sample();
    p.graphs[0].first_layer_mut().title = "literal x_{2}".into();
    for version in [11, 12] {
        let mut wire: serde_json::Value = serde_json::from_str(&file::to_json(&p).unwrap()).unwrap();
        wire["version"] = version.into();
        let page = wire["graphs"][0]["page"].as_object_mut().unwrap();
        page.remove("annotations");
        page.remove("next_text_id");
        let old = file::from_json(&wire.to_string()).unwrap();
        assert_eq!(old, p);
        assert!(old.graphs[0].page.annotations.is_empty());
        assert_eq!(old.graphs[0].first_layer().legend_position, None);
    }
    let id = p.graphs[0].page.add_text([0.2, 0.4]).unwrap();
    p.graphs[0].page.text_mut(id).unwrap().text = "R^{2} α".into();
    p.graphs[0].first_layer_mut().legend_position = Some([0.6, 0.1]);
    let json = file::to_json(&p).unwrap();
    let restored = file::from_json(&json).unwrap();
    assert_eq!(restored, p);
    assert_eq!(file::from_scv(&file::to_scv(&p).unwrap()).unwrap(), p);
    assert_eq!(
        svg::render_page(&restored.graphs[0], &restored),
        svg::render_page(&p.graphs[0], &p)
    );
    for bad in ["id", "size_pt", "position", "duplicate", "counter", "legend"] {
        let mut wire: serde_json::Value = serde_json::from_str(&json).unwrap();
        let page = &mut wire["graphs"][0]["page"];
        match bad {
            "id" => page["annotations"][0]["id"] = 0.into(),
            "size_pt" => page["annotations"][0]["size_pt"] = (-1).into(),
            "position" => page["annotations"][0]["position"] = serde_json::json!([1.2, 0.0]),
            "duplicate" => {
                let duplicate = page["annotations"][0].clone();
                page["annotations"].as_array_mut().unwrap().push(duplicate);
            }
            "counter" => page["next_text_id"] = 0.into(),
            _ => page["layers"][0]["legend_position"] = serde_json::json!([0.2, -0.1]),
        }
        assert!(file::from_json(&wire.to_string()).is_err(), "reject {bad}");
    }
    p.graphs[0].page.text_mut(id).unwrap().position[0] = f64::NAN;
    assert!(file::to_json(&p).is_err());
}

#[test]
fn manual_legends_and_page_text_are_overlays_above_insets() {
    use scene::ObjectTarget as T;
    let mut p = Project::sample();
    let mut upper = p.graphs[0].first_layer().clone();
    upper.placement = Some(scicanvas_core::page::LayerRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    upper.legend = false;
    p.graphs[0].page.add_layer(upper);
    let auto = scene::layout_page(&p.graphs[0], &p, &[]);
    let hidden = auto.objects.iter().find(|o| o.target == T::Legend(0)).unwrap();
    assert_eq!(auto.object_at(hidden.bounds[0] + 2.0, hidden.bounds[1] + 2.0), None);
    p.graphs[0].first_layer_mut().legend_position = Some([0.2, 0.2]);
    let manual = scene::layout_page(&p.graphs[0], &p, &[]);
    let legend = manual.objects.iter().find(|o| o.target == T::Legend(0)).unwrap();
    assert!(legend.overlay);
    assert_eq!(legend.items.end, manual.items.len());
    assert_eq!(
        manual.object_at(legend.bounds[0] + 2.0, legend.bounds[1] + 2.0),
        Some(T::Legend(0))
    );
    let id = p.graphs[0].page.add_text([0.2, 0.2]).unwrap();
    let text = scene::layout_page(&p.graphs[0], &p, &[]);
    assert_eq!(
        text.object_at(legend.bounds[0] + 2.0, legend.bounds[1] + 2.0),
        Some(T::Text(id))
    );
}

#[test]
fn scientific_titles_legends_and_labels_share_layout_history_and_project_files() {
    use scicanvas_core::page::{CurveStyle, LabelSource};
    let mut p = Project::sample();
    p.graphs[0].first_layer_mut().title = r"literal CO_{2} \b{x}".into();
    let original = p.clone();
    let old_svg = svg::render_page(&p.graphs[0], &p);
    #[cfg(feature = "serde")]
    {
        for version in [10, 11] {
            let mut wire: serde_json::Value =
                serde_json::from_str(&scicanvas_core::project_file::to_json(&p).unwrap()).unwrap();
            wire["version"] = version.into();
            wire["graphs"][0]["page"]["layers"][0]
                .as_object_mut()
                .unwrap()
                .remove("rich_text");
            let restored = scicanvas_core::project_file::from_json(&wire.to_string()).unwrap();
            assert_eq!(restored, p);
            assert_eq!(svg::render_page(&restored.graphs[0], &restored), old_svg);
        }
    }
    let mut history = scicanvas_core::history::History::default();
    history.begin(&p, "Scientific text");
    let layer = p.graphs[0].first_layer_mut();
    layer.rich_text = true;
    layer.title = r"CO_{2} R^{2} \b{normal} \i{rate} α".into();
    layer.x_title = r"Time / s^{−1}".into();
    layer.y_title = r"\i{c} / mol L^{−1}".into();
    layer.y2_title = r"\b{R}_{0}".into();
    layer.edit_styles(|styles| {
        styles.insert(
            "B".into(),
            CurveStyle {
                legend_label: r"\i{CO}_{2} \beta".into(),
                ..Default::default()
            },
        )
    });
    layer.edit_styles(|styles| {
        styles.insert(
            "C".into(),
            CurveStyle {
                right_axis: true,
                ..Default::default()
            },
        )
    });
    layer.labels.visible = true;
    layer.labels.source = LabelSource::Column;
    layer.labels.column = Some("B".into());
    history.changed();
    history.commit(&p);
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert!(sc.items.iter().any(|i| matches!(
        &i.prim,
        scene::Prim::Text {
            rich: true,
            vertical: true,
            ..
        }
    )));
    let mut scripts = 0;
    for item in &sc.items {
        if let scene::Prim::Text {
            text, size, bold, rich, ..
        } = &item.prim
        {
            let l = scicanvas_core::text::layout_formatted(text, *size, *bold, *rich);
            scripts += l.runs.iter().filter(|r| r.dy != 0.0).count();
            assert!(l
                .runs
                .iter()
                .all(|r| r.size > 0.0 && r.size.is_finite() && r.x.is_finite()));
        }
    }
    assert!(scripts >= 6);
    let rendered = svg::render_page(&p.graphs[0], &p);
    assert!(rendered.contains("CO2 R2 normal rate α"));
    assert!(!rendered.contains("textLength=") && !rendered.contains("lengthAdjust="));
    #[cfg(feature = "serde")]
    {
        let json = scicanvas_core::project_file::to_json(&p).unwrap();
        assert!(json.contains("\"rich_text\": true"));
        let loaded = scicanvas_core::project_file::from_json(&json).unwrap();
        assert_eq!(loaded, p);
        assert_eq!(svg::render_page(&loaded.graphs[0], &loaded), rendered);
        let zipped = scicanvas_core::project_file::to_scv(&p).unwrap();
        assert_eq!(scicanvas_core::project_file::from_scv(&zipped).unwrap(), p);
    }
    let edited = p.clone();
    p = history.undo(&p).unwrap();
    assert_eq!(p, original);
    assert_eq!(svg::render_page(&p.graphs[0], &p), old_svg);
    p = history.redo(&p).unwrap();
    assert_eq!(p, edited);
    assert_eq!(svg::render_page(&p.graphs[0], &p), rendered);
}

#[test]
fn presets_have_example_data_and_use_the_shared_scene() {
    let book = scicanvas_core::presets::example();
    let mut project = Project::default();
    project.books.push(book.clone());
    let presets = scicanvas_core::presets::all();
    assert_eq!(presets.len(), 17);
    let mut ids = std::collections::BTreeSet::new();
    for preset in presets {
        assert!(ids.insert(preset.id));
        let spec = PlotSpec::from_selection(&book, &[], preset.kind).unwrap();
        let mut graph = Graph::new(20, preset.name);
        graph.first_layer_mut().spec = spec.into();
        let sc = scene::build_page(&graph, &project, 800.0, 600.0, None);
        assert!(!sc.items.is_empty());
        assert_eq!(sc.layer_plots.len(), 1);
        // 新图形默认 A4 纸宽、4:3
        assert!(svg::render_page(&graph, &project).contains(r#"width="210mm" height="157.5mm""#));
        let layout = scene::layout_page(&graph, &project, &[]);
        assert_eq!([layout.w, layout.h], graph.page.logical_size());
        assert!(svg::render_page(&graph, &project).contains(&format!("viewBox=\"0 0 {} {}\"", layout.w, layout.h)));
    }
}

#[test]
fn invalid_plot_does_not_mutate_project_and_selection_rules_are_shared() {
    let mut project = Project::sample();
    let before = project.clone();
    let spec = PlotSpec {
        book_id: Some(1),
        y_keys: vec!["missing".into()],
        ..Default::default()
    };
    assert!(project.plot_spec(spec).is_err());
    assert_eq!(project, before);
    let spec = PlotSpec::from_selection(project.book(1).unwrap(), &["B".into()], PlotType::Line).unwrap();
    assert_eq!(spec.x_key.as_deref(), Some("A"));
    let id = project.plot_spec(spec).unwrap();
    assert_eq!(project.graph(id).unwrap().first_layer().draft().y_keys, vec!["B"]);
    let mut book = Book::new(10, "空表");
    book.push_column(Role::X);
    assert_eq!(
        PlotSpec::from_selection(&book, &[], PlotType::Line).unwrap_err(),
        "工作簿里没有 Y 列，请先把列设为 Y"
    );
    assert_eq!(
        PlotSpec::from_selection(&book, &["A".into()], PlotType::Line).unwrap_err(),
        "选中的列里没有 Y 列，请至少选中一个 Y 列"
    );
}

#[test]
fn mean_transform_handles_unsorted_groups_blanks_and_derived_curves() {
    let mut book = Book::new(1, "重复测量");
    for (key, role, data) in [
        ("A", Role::X, vec!["2", "1", "2", "1", "3", "4"]),
        ("B", Role::Y, vec!["4", "2", "8", "6", "", "bad"]),
        ("C", Role::Y, vec!["4", "2", "8", "6", "", "bad"]),
    ] {
        let mut c = Column::new(key, role);
        c.data = data.into_iter().map(String::from).collect();
        book.columns.push(c);
    }
    let mut spec = PlotSpec::from_selection(&book, &[], PlotType::Scatter).unwrap();
    spec.transform = Transform::Mean;
    spec.line_only.push("C".into());
    let (_, curves) = spec.series(&book);
    assert_eq!(curves[0].points, vec![[1.0, 4.0], [2.0, 6.0]]);
    assert_eq!(curves[1].points, vec![[2.0, 4.0], [1.0, 2.0], [2.0, 8.0], [1.0, 6.0]]);
}

#[test]
fn multiple_books_and_layers_have_independent_layout_and_clipping() {
    let mut project = Project::sample();
    let mut second = project.books[0].clone();
    second.id = project.alloc_id();
    second.name = "第二本".into();
    second.columns[1].long_name = "第二图层曲线".into();
    let layer = Layer {
        spec: PlotSpec::from_selection(&second, &["B".into()], PlotType::Bar)
            .unwrap()
            .into(),
        title: "第二图层标题".into(),
        ..Default::default()
    };
    project.books.push(second);
    project.graphs[0].page.add_layer(layer);
    project.graphs[0].page.columns = 2;
    let graph = &project.graphs[0];
    let sc = scene::build_page(graph, &project, 1000.0, 450.0, None);
    assert_eq!(sc.layer_plots.len(), 2);
    assert!(sc.layer_plots[0][0] < 500.0 && sc.layer_plots[1][0] > 500.0);
    let clipped: Vec<_> = sc.items.iter().filter_map(|item| item.clip).collect();
    assert!(clipped.contains(&sc.layer_plots[0]));
    assert!(clipped.contains(&sc.layer_plots[1]));
    let image = svg::render_page(graph, &project);
    assert!(image.contains("clip2-1"));
    assert!(image.contains("第二图层标题"));
    assert!(image.contains("第二图层曲线"));

    let key = "B".to_string();
    project.delete_columns(1, &[key]);
    assert!(!project.graphs[0].page.layers()[0].draft().y_keys.contains(&"B".into()));
    assert!(project.graphs[0].page.layers()[1].draft().y_keys.contains(&"B".into()));
    project.remove(1);
    assert_eq!(project.graphs[0].page.layers()[0].book_id, None);
    assert!(project.graphs[0].page.layers()[1].book_id.is_some());
}

#[test]
fn coordinate_inverse_and_palette_boundaries() {
    for log in [false, true] {
        let x = scicanvas_core::ticks::axis(&[], false, 1.0, 100.0, log, false);
        let y = scicanvas_core::ticks::axis(&[], false, 1.0, 1000.0, log, false);
        let coord = Cartesian {
            x: &x,
            y: &y,
            rect: [30.0, 20.0, 700.0, 500.0],
        };
        for point in [[1.0, 1.0], [10.0, 100.0], [100.0, 1000.0]] {
            let recovered = coord.unproject(coord.project(point));
            assert!((point[0] - recovered[0]).abs() < 1e-9);
            assert!((point[1] - recovered[1]).abs() < 1e-9);
        }
    }
    assert_eq!(ColorMap::Viridis.sample(0.0), (68, 1, 84));
    assert_eq!(ColorMap::Viridis.sample(1.0), (253, 231, 37));
    assert_eq!(ColorMap::Viridis.sample(f64::NAN), ColorMap::Viridis.sample(0.0));
    assert_eq!(ColorMap::Colorblind.sample(-1.0), ColorMap::Colorblind.sample(0.0));
    let mut p = Project::sample();
    let original = svg::render(&p.graphs[0], p.book(1), 800.0, 600.0);
    p.graphs[0].first_layer_mut().color_map = ColorMap::Viridis;
    let colored = svg::render(&p.graphs[0], p.book(1), 800.0, 600.0);
    assert_ne!(original, colored);
    assert!(colored.contains("#440154"));
}

fn example_graph(kind: PlotType) -> (Project, Graph) {
    let book = scicanvas_core::presets::example();
    let mut project = Project::default();
    project.books.push(book.clone());
    let mut graph = Graph::new(20, "示例");
    graph.first_layer_mut().spec = PlotSpec::from_selection(&book, &[], kind).unwrap().into();
    (project, graph)
}

#[test]
fn legend_moves_outside_when_every_corner_covers_data() {
    // 百分比堆叠柱占满整个高度：四个角都会压住柱子，图例放到坐标框右侧
    let (project, graph) = example_graph(PlotType::PercentStackedBar);
    let sc = scene::layout_page(&graph, &project, &[]);
    let [l, _, w, _] = sc.plot;
    let legend = sc
        .items
        .iter()
        .filter_map(|i| match i.prim {
            scene::Prim::Rect {
                x,
                w: rw,
                fill: Some((c, _)),
                stroke: None,
                ..
            } if c == scene::PAPER && rw < sc.w * 0.5 => Some(x),
            _ => None,
        })
        .next_back()
        .unwrap();
    assert!(legend >= l + w, "legend at {legend}, plot ends at {}", l + w);
    // 有空角的图仍放在坐标框内
    let (project, graph) = example_graph(PlotType::Line);
    let sc = scene::layout_page(&graph, &project, &[]);
    let [l, _, w, _] = sc.plot;
    assert!(sc.items.iter().any(|i| matches!(i.prim,
        scene::Prim::Rect { x, fill: Some((c, _)), stroke: None, .. } if c == scene::PAPER && x > l && x < l + w)));
}

#[test]
fn symbols_at_auto_axis_ends_get_a_margin_but_lines_do_not() {
    // 示例数据 X 为 0–4、Y 的最小值和最大值正好落在整刻度上
    let (project, graph) = example_graph(PlotType::Scatter);
    let sc = scene::layout_page(&graph, &project, &[]);
    assert!(
        sc.axes.x.lo < 0.0 && sc.axes.x.hi > 4.0,
        "{:?}",
        (sc.axes.x.lo, sc.axes.x.hi)
    );
    assert_eq!(
        (sc.axes.x.ticks.first(), sc.axes.x.ticks.last()),
        (Some(&0.0), Some(&4.0))
    );
    assert!(
        sc.axes.left_y.lo < *sc.axes.left_y.ticks.first().unwrap()
            && sc.axes.left_y.hi > *sc.axes.left_y.ticks.last().unwrap()
    );
    // 纯折线保持到整刻度为止
    let (project, graph) = example_graph(PlotType::Line);
    let sc = scene::layout_page(&graph, &project, &[]);
    assert_eq!((sc.axes.x.lo, sc.axes.x.hi), (0.0, 4.0));
}

#[test]
fn rebinding_a_blank_layer_uses_the_book_defaults_and_keeps_its_settings() {
    let book = scicanvas_core::presets::example();
    let defaults = PlotSpec::from_selection(&book, &[], PlotType::LineSymbol).ok();
    // 空白图形：没有关联表，换成示例表后使用它的 X 列和全部 Y 列
    let blank = PlotSpec {
        plot_type: PlotType::Scatter,
        transform: Transform::Mean,
        ..Default::default()
    };
    let next = blank.rebound(book.id, defaults.as_ref());
    assert_eq!(next.book_id, Some(book.id));
    assert_eq!(next.x_key.as_deref(), Some("A"));
    assert_eq!(next.y_keys, ["B", "C"]);
    assert_eq!((next.plot_type, next.transform), (PlotType::Scatter, Transform::Mean));
    // 新表没有 Y 列：只关联表，保留图类型，列留空
    let next = blank.rebound(99, None);
    assert_eq!(
        (next.book_id, next.y_keys.len(), next.plot_type),
        (Some(99), 0, PlotType::Scatter)
    );
    // 图类型不适用于新表（浮动柱需要成对的 Y 列）时用默认类型
    let mut one_y = book.clone();
    one_y.columns.truncate(2);
    let defaults = PlotSpec::from_selection(&one_y, &[], PlotType::LineSymbol).ok();
    let floating = PlotSpec {
        plot_type: PlotType::FloatingBar,
        ..Default::default()
    };
    let next = floating.rebound(one_y.id, defaults.as_ref());
    assert_eq!((next.y_keys.len(), next.plot_type), (1, PlotType::LineSymbol));
}

#[test]
fn real_text_metrics_keep_long_titles_legends_and_labels_within_their_page() {
    let mut p = Project::sample();
    let title = "Wide WWW and narrow iii — 长标题与科学符号 μ ± ∞ repeated repeatedly";
    let g = p.graph_mut(2).unwrap();
    g.page.width_mm = 89.0;
    g.page.height_mm = 80.0;
    let layer = g.first_layer_mut();
    layer.title = title.into();
    layer.x_title = "A very long horizontal scientific axis title 时间 (s)".into();
    layer.y_title = "A very long vertical scientific axis title 电压 (mV)".into();
    p.book_mut(1).unwrap().columns[1].long_name =
        "A long legend label with WWW iii and 中文 that wraps into rows".into();
    let sc = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
    let mut titles = 0;
    for item in &sc.items {
        if let scene::Prim::Text {
            x,
            y,
            text,
            size,
            bold,
            anchor,
            vertical,
            ..
        } = &item.prim
        {
            let measured = scicanvas_core::text::layout(text, *size, *bold);
            let offset = match anchor {
                scene::Anchor::Start => 0.0,
                scene::Anchor::Middle => measured.width / 2.0,
                scene::Anchor::End => measured.width,
            };
            let bounds = if *vertical {
                [
                    *x - measured.height / 2.0,
                    *y + offset - measured.width,
                    *x + measured.height / 2.0,
                    *y + offset,
                ]
            } else {
                [
                    *x - offset,
                    *y - measured.height / 2.0,
                    *x - offset + measured.width,
                    *y + measured.height / 2.0,
                ]
            };
            assert!(
                bounds[0] >= -0.01 && bounds[1] >= -0.01 && bounds[2] <= sc.w + 0.01 && bounds[3] <= sc.h + 0.01,
                "{text:?}: {bounds:?}"
            );
            if *bold {
                assert!(text.contains('\n'));
                titles += 1;
            }
        }
    }
    assert_eq!(titles, 1);
    let output = svg::render_page(p.graph(2).unwrap(), &p);
    assert!(!output.contains("textLength=") && !output.contains("lengthAdjust="));
    #[cfg(feature = "serde")]
    {
        let loaded =
            scicanvas_core::project_file::from_json(&scicanvas_core::project_file::to_json(&p).unwrap()).unwrap();
        assert_eq!(loaded, p);
        assert_eq!(svg::render_page(loaded.graph(2).unwrap(), &loaded), output);
    }
}
