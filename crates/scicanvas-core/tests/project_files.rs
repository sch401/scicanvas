#![cfg(feature = "serde")]

use scicanvas_core::{project_file, svg, PlotType, Project};

const V0: &str = include_str!("fixtures/legacy-v0.scproj");
const V1: &str = include_str!("fixtures/legacy-v1.scproj");

fn fixture_fonts() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| assert!(scicanvas_core::text::initialize(scicanvas_core::text::Fonts::bundled()).is_ok()));
}

#[test]
fn display_formats_round_trip_and_leave_stored_values_alone() {
    fixture_fonts();
    use scicanvas_core::format::{DateFormat, DisplayFormat, NumberFormat};
    let mut p = Project::sample();
    // 没设格式时不写入文件，旧项目保存后不变
    assert!(!project_file::to_json(&p).unwrap().contains("\"format\""));
    let raw = p.books[0].columns[1].data.text_at(0).into_owned();
    let col = &mut p.books[0].columns[1];
    col.format = DisplayFormat {
        number: NumberFormat::Scientific { places: 2 },
        date: DateFormat::Original,
        utc_offset: 0,
    };
    let v = col.data.f64_at(0).unwrap();
    assert_eq!(
        col.display_at(0),
        NumberFormat::Scientific { places: 2 }.apply(v).unwrap()
    );
    assert_eq!(col.data.text_at(0), raw, "存储的原始写法不变");
    let restored = project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap();
    assert_eq!(restored, p);
    // 日期列
    let mut dates: scicanvas_core::ColumnData = vec!["2026/10/18 14:30".to_string()].into();
    dates = dates.converted(scicanvas_core::DataKind::DateTime).unwrap();
    let col = &mut p.books[0].columns[2];
    col.data = dates;
    assert_eq!(col.display_at(0), "2026/10/18 14:30");
    col.format.date = DateFormat::Chinese;
    assert_eq!(col.display_at(0), "2026年10月18日");
    let restored = project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap();
    assert_eq!(restored.books[0].columns[2].format.date, DateFormat::Chinese);
}

#[test]
fn pages_without_a_saved_size_keep_the_old_default_while_new_graphs_use_a4() {
    fixture_fonts();
    // v0/v1 旧文件没有页面尺寸：读取后保持当时的 89 × 66.75 mm
    for p in [
        project_file::from_json(V0).unwrap(),
        project_file::from_json(V1).unwrap(),
    ] {
        assert!(p
            .graphs
            .iter()
            .all(|g| (g.page.width_mm, g.page.height_mm) == scicanvas_core::page::LEGACY_PAGE_MM));
    }
    // 记录了尺寸的页面按记录读取；新图形默认 A4 纸宽、4:3
    let mut p = Project::sample();
    p.graphs[0].page.width_mm = 120.0;
    let restored = project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap();
    assert_eq!(restored.graphs[0].page.width_mm, 120.0);
    let page = scicanvas_core::page::Page::default();
    assert_eq!((page.width_mm, page.height_mm), (210.0, 157.5));
}

#[test]
fn old_versions_preserve_data_and_all_four_plot_layouts() {
    fixture_fonts();
    let mut p = project_file::from_json(V0).unwrap();
    assert_eq!(p, project_file::from_json(V1).unwrap());
    assert_eq!(p.version, project_file::CURRENT_VERSION);
    assert!(!p.graphs[0].first_layer().draft().x_by_y.is_empty());
    assert!(!p.graphs[0].first_layer().draft().line_only.is_empty());
    assert_eq!(project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap(), p);
    for (kind, expected) in [
        (PlotType::Line, include_str!("fixtures/legacy-line.svg")),
        (PlotType::Scatter, include_str!("fixtures/legacy-scatter.svg")),
        (PlotType::LineSymbol, include_str!("fixtures/legacy-linesym.svg")),
        (PlotType::Bar, include_str!("fixtures/legacy-bar.svg")),
    ] {
        p.graphs[0].first_layer_mut().plot_type = kind;
        assert_eq!(svg::render(&p.graphs[0], p.book(1), 800.0, 600.0), expected);
    }
}

#[test]
#[ignore = "Explicit maintainer action; review the SVG diff before committing"]
fn update_legacy_svg_fixtures() {
    fixture_fonts();
    let mut p = project_file::from_json(V0).unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for (kind, name) in [
        (PlotType::Line, "line"),
        (PlotType::Scatter, "scatter"),
        (PlotType::LineSymbol, "linesym"),
        (PlotType::Bar, "bar"),
    ] {
        p.graphs[0].first_layer_mut().plot_type = kind;
        let svg = svg::render(&p.graphs[0], p.book(1), 800.0, 600.0);
        std::fs::write(dir.join(format!("legacy-{name}.svg")), svg).unwrap();
    }
}

#[test]
fn unsupported_or_damaged_files_fail_and_stale_allocator_recovers() {
    fixture_fonts();
    assert!(project_file::from_json("[").is_err());
    assert!(project_file::from_json("[]").is_err());
    for version in [
        serde_json::json!(999),
        serde_json::json!(null),
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("1"),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(V1).unwrap();
        value["version"] = version;
        assert!(project_file::from_json(&value.to_string()).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_str(V1).unwrap();
    value["graphs"][0]["id"] = value["books"][0]["id"].clone();
    assert!(project_file::from_json(&value.to_string()).is_err());
    let mut value: serde_json::Value = serde_json::from_str(V0).unwrap();
    value["next_id"] = 1.into();
    let mut p = project_file::from_json(&value.to_string()).unwrap();
    let id = p.new_book();
    assert!(id > 2);
    let mut value: serde_json::Value = serde_json::from_str(V1).unwrap();
    value["next_id"] = u64::MAX.into();
    assert!(project_file::from_json(&value.to_string()).is_err());
    value["next_id"] = 1.into();
    value["graphs"][0]["id"] = (u64::MAX - 1).into();
    assert!(project_file::from_json(&value.to_string()).is_err());
}

#[test]
fn atomic_save_replaces_existing_file_and_failure_preserves_it() {
    fixture_fonts();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/file-tests")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("项目.scproj");
    let first = Project::sample();
    project_file::save(&path, &first).unwrap();
    let mut second = first.clone();
    second.books[0].name = "更新".into();
    project_file::save(&path, &second).unwrap();
    assert_eq!(
        project_file::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap(),
        second
    );
    let original = std::fs::read(&path).unwrap();
    second.graphs[0].first_layer_mut().line_width = f32::NAN;
    assert!(project_file::save(&path, &second).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(project_file::save(&directory, &first).is_err());
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn v2_layers_roundtrip_and_invalid_page_is_rejected() {
    fixture_fonts();
    let mut p = project_file::from_json(V1).unwrap();
    let mut second = p.graphs[0].page.layers()[0].clone();
    second.title = "第二图层".into();
    second.transform = scicanvas_core::plotspec::Transform::Mean;
    second.color_map = scicanvas_core::colormap::ColorMap::Viridis;
    p.graphs[0].page.add_layer(second);
    p.graphs[0].page.columns = 2;
    p.graphs[0].page.width_mm = 180.0;
    let text = project_file::to_json(&p).unwrap();
    assert_eq!(project_file::from_json(&text).unwrap(), p);
    for page in [
        serde_json::json!({}),
        serde_json::json!({"layers": []}),
        serde_json::json!({"layers": [null]}),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["graphs"][0]["page"] = page;
        assert!(project_file::from_json(&value.to_string()).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["graphs"][0]["page"]["columns"] = 0.into();
    assert!(project_file::from_json(&value.to_string()).is_err());
}

// 迁移前曲线存储投影，用于逐字节核对旧字段、顺序和省略规则。
fn legacy_curve_projection(p: &Project) -> String {
    let mut value = serde_json::to_value(p).unwrap();
    for (graph, wire) in p.graphs.iter().zip(value["graphs"].as_array_mut().unwrap()) {
        for (layer, wire) in graph
            .page
            .layers()
            .iter()
            .zip(wire["page"]["layers"].as_array_mut().unwrap())
        {
            if layer.book_id.is_none() {
                continue;
            }
            let plots: Vec<_> = layer.plots_in(&|id| p.book(id)).into_iter().map(|plot| {
                let mut curve = serde_json::json!({"mapping": plot.mapping, "style": plot.style, "x_override": plot.x_override});
                if layer.draft().base_by_y.get(&plot.key).is_some_and(String::is_empty) {
                    curve["base_missing"] = true.into();
                }
                curve
            }).collect();
            wire["plots"] = plots.into();
            for key in ["y_keys", "x_by_y", "base_by_y", "line_only", "curves", "row_by_y"] {
                wire.as_object_mut().unwrap().remove(key);
            }
        }
    }
    value["version"] = project_file::CURRENT_VERSION.into();
    serde_json::to_string_pretty(&value).unwrap()
}

fn legacy_v14_projection(p: &Project) -> String {
    let mut value: serde_json::Value = serde_json::from_str(&legacy_curve_projection(p)).unwrap();
    value["version"] = 14.into();
    for graph in value["graphs"].as_array_mut().unwrap() {
        for layer in graph["page"]["layers"].as_array_mut().unwrap() {
            let object = layer.as_object_mut().unwrap();
            for key in ["y2_auto", "y2_min", "y2_max", "y2_format"] {
                object.remove(key);
            }
        }
    }
    serde_json::to_string_pretty(&value).unwrap()
}

#[test]
fn unified_export_preserves_current_bytes_previews_and_legacy_projects() {
    fixture_fonts();
    let mut original = Project::sample();
    let foreign = original.new_book();
    let book = original.book_mut(foreign).unwrap();
    book.push_column(scicanvas_core::Role::Y);
    for (i, col) in book.columns.iter_mut().enumerate() {
        col.data = vec![(i + 1).to_string(), (i + 3).to_string()].into();
    }
    let keys = ["B".into(), "C".into()];
    let (gid, _) = original.plot_from_selection(1, &keys, PlotType::FloatingBar).unwrap();
    original.add_curves(gid, 0, foreign, &keys).unwrap();
    let layer = original.graph_mut(gid).unwrap().first_layer_mut();
    layer.rows = Some((0, 1));
    layer.edit_draft(|draft| draft.row_by_y.insert("C".into(), Some((0, 1))));
    layer.edit_draft(|draft| draft.x_by_y.insert("C".into(), "A".into()));
    layer.edit_styles(|styles| styles.insert("C".into(), scicanvas_core::page::CurveStyle::default()));
    for mode in ["bound", "missing-base", "unbound", "inactive"] {
        let mut p = original.clone();
        match mode {
            "missing-base" => p.delete_columns(1, &["B".into()]),
            "unbound" => p.graphs[0].first_layer_mut().book_id = None,
            "inactive" => p
                .graph_mut(gid)
                .unwrap()
                .first_layer_mut()
                .spec
                .edit_draft(|draft| draft.y_keys.clear()),
            _ => {}
        }
        let json = project_file::to_json(&p).unwrap();
        assert_eq!(json, legacy_curve_projection(&p), "{mode}");
        let restored = project_file::from_json(&json).unwrap();
        assert_eq!(project_file::to_json(&restored).unwrap(), json);
        let archive = project_file::to_scv(&p).unwrap();
        assert_eq!(project_file::to_scv(&p).unwrap(), archive);
        assert_eq!(project_file::from_scv(&archive).unwrap(), restored);
        for graph in &p.graphs {
            assert_eq!(
                svg::render_page(graph, &p),
                svg::render_page(restored.graph(graph.id).unwrap(), &restored)
            );
        }
    }
    // v0/v1 使用真实旧文件；v10 使用旧平行表，v11/v13 使用旧类型化曲线。
    let mut legacy_files = vec![V0.to_string(), V1.to_string()];
    for version in [10, 11, 13, 14] {
        let mut wire = if version == 10 {
            legacy_editor_projection(&original)
        } else if version == 14 {
            serde_json::from_str(&legacy_v14_projection(&original)).unwrap()
        } else {
            serde_json::from_str(&legacy_curve_projection(&original)).unwrap()
        };
        wire["version"] = version.into();
        legacy_files.push(wire.to_string());
    }
    for file in legacy_files {
        let p = project_file::from_json(&file).unwrap();
        let json = project_file::to_json(&p).unwrap();
        assert_eq!(json, legacy_curve_projection(&p));
        // 旧格式会省略与默认值相同的冗余行范围覆盖，核对权威保存投影和图形语义。
        let restored = project_file::from_json(&json).unwrap();
        assert_eq!(project_file::to_json(&restored).unwrap(), json);
        for graph in &p.graphs {
            assert_eq!(
                svg::render_page(graph, &p),
                svg::render_page(restored.graph(graph.id).unwrap(), &restored)
            );
        }
    }
}

#[test]
fn v14_defaults_keep_the_old_right_axis_appearance_and_v15_settings_roundtrip() {
    use scicanvas_core::{format::NumberFormat, page::CurveStyle};
    fixture_fonts();
    let mut p = Project::sample();
    let key = p.graphs[0].first_layer().draft().y_keys[0].clone();
    let layer = p.graphs[0].first_layer_mut();
    layer.set_plot_style(
        &key,
        Some(CurveStyle {
            right_axis: true,
            ..Default::default()
        }),
    );
    layer.y_format = NumberFormat::Scientific { places: 1 };
    let old_svg = svg::render_page(&p.graphs[0], &p);
    let v14 = legacy_v14_projection(&p);
    let restored = project_file::from_json(&v14).unwrap();
    let layer = restored.graphs[0].first_layer();
    assert!(layer.y2_auto);
    assert_eq!((layer.y2_min, layer.y2_max), (0.0, 10.0));
    assert_eq!(layer.y2_format, None);
    assert_eq!(svg::render_page(&restored.graphs[0], &restored), old_svg);

    let mut manual = restored.clone();
    let layer = manual.graphs[0].first_layer_mut();
    layer.y2_auto = false;
    layer.y2_min = 40.0;
    layer.y2_max = 80.0;
    layer.y2_format = Some(NumberFormat::Engineering { places: 2 });
    let json = project_file::to_json(&manual).unwrap();
    let loaded = project_file::from_json(&json).unwrap();
    assert_eq!(loaded, manual);
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&manual).unwrap()).unwrap(),
        manual
    );
}

#[test]
fn v15_projects_default_reference_lines_and_current_version_roundtrips_them() {
    use scicanvas_core::page::{ReferenceAxis, ReferenceLine};
    let project = Project::sample();
    let mut legacy = serde_json::to_value(&project).unwrap();
    legacy["version"] = 15.into();
    let loaded = project_file::from_json(&legacy.to_string()).unwrap();
    assert_eq!(loaded.version, scicanvas_core::project_file::CURRENT_VERSION);
    assert!(loaded.graphs[0].first_layer().reference_lines.is_empty());

    let mut current = loaded;
    current.graphs[0].first_layer_mut().reference_lines.push(ReferenceLine {
        axis: ReferenceAxis::YLeft,
        value: 2.5,
        color: [20, 40, 60],
        width: 1.5,
        dashed: false,
    });
    let json = project_file::to_json(&current).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["version"],
        scicanvas_core::project_file::CURRENT_VERSION
    );
    assert_eq!(project_file::from_json(&json).unwrap(), current);
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&current).unwrap()).unwrap(),
        current
    );
}

#[test]
fn v16_curve_styles_default_new_point_mappings_and_current_version_roundtrips_them() {
    use scicanvas_core::page::{CurveStyle, SymbolFill};
    let mut project = Project::sample();
    let key = project.graphs[0].first_layer().draft().y_keys[0].clone();
    project.graphs[0].first_layer_mut().set_plot_style(
        &key,
        Some(CurveStyle {
            right_axis: true,
            color: Some([80, 40, 120]),
            ..Default::default()
        }),
    );
    let mut v16 = serde_json::to_value(&project).unwrap();
    v16["version"] = 16.into();
    fn remove_v17_fields(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(fields) => {
                for name in [
                    "symbol_fill",
                    "symbol_border_color",
                    "symbol_border_width",
                    "color_column",
                    "size_column",
                    "symbol_column",
                    "size_range",
                ] {
                    fields.remove(name);
                }
                for child in fields.values_mut() {
                    remove_v17_fields(child);
                }
            }
            serde_json::Value::Array(values) => values.iter_mut().for_each(remove_v17_fields),
            _ => {}
        }
    }
    remove_v17_fields(&mut v16);
    let mut loaded = project_file::from_json(&v16.to_string()).unwrap();
    assert_eq!(loaded.version, scicanvas_core::project_file::CURRENT_VERSION);
    let style = loaded.graphs[0].first_layer().plot_style(&key).unwrap();
    assert_eq!(style.color, Some([80, 40, 120]));
    assert!(style.color_column.is_none() && style.symbol_border_color.is_none());

    loaded.graphs[0].first_layer_mut().set_plot_style(
        &key,
        Some(CurveStyle {
            right_axis: true,
            color: Some([80, 40, 120]),
            color_column: Some("C".into()),
            size_column: Some("C".into()),
            size_range: Some([2.0, 11.0]),
            symbol_fill: Some(SymbolFill::Hollow),
            symbol_border_color: Some([10, 20, 30]),
            symbol_border_width: Some(0.7),
            ..Default::default()
        }),
    );
    let json = project_file::to_json(&loaded).unwrap();
    assert_eq!(project_file::from_json(&json).unwrap(), loaded);
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&loaded).unwrap()).unwrap(),
        loaded
    );
}

#[test]
fn graph_template_files_rebind_columns_and_keep_page_and_curve_styles() {
    use scicanvas_core::{format::NumberFormat, page::CurveStyle};
    fixture_fonts();
    let mut source = Project::sample();
    let key = source.graphs[0].first_layer().draft().y_keys[0].clone();
    let layer = source.graphs[0].first_layer_mut();
    layer.set_plot_style(
        &key,
        Some(CurveStyle {
            right_axis: true,
            symbol_size: Some(9.0),
            ..Default::default()
        }),
    );
    layer.y2_auto = false;
    layer.y2_min = 40.0;
    layer.y2_max = 80.0;
    layer.y2_format = Some(NumberFormat::Engineering { places: 1 });
    source.graphs[0].page.width_mm = 180.0;
    let template = project_file::graph_template_from_graph(&source.graphs[0], &source).unwrap();
    assert!(template.page.layers().iter().all(|layer| layer.book_id.is_none()));

    let path = std::env::temp_dir().join(format!(
        "scicanvas-template-{}-{}.scgt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    project_file::save_graph_template(&path, &template).unwrap();
    let loaded = project_file::load_graph_template(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(loaded, template);
    let json = project_file::graph_template_to_json(&loaded).unwrap();
    assert!(!json.contains("\"book\":"), "templates store no source book IDs");

    let mut target = source.clone();
    let target_id = target.alloc_id();
    let mut book = target.book(1).unwrap().clone();
    book.id = target_id;
    book.name = "Target".into();
    let column = book.column_index(&key).unwrap();
    book.columns[column].data = ["50", "60", "70"].map(str::to_owned).to_vec().into();
    target.books.push(book);
    let graph_id = target.apply_graph_template(&loaded, target_id).unwrap();
    let graph = target.graph(graph_id).unwrap();
    assert_eq!(graph.page.width_mm, 180.0);
    assert_eq!(graph.page.columns, 1);
    assert_eq!(graph.first_layer().book_id, Some(target_id));
    assert_eq!((graph.first_layer().y2_min, graph.first_layer().y2_max), (40.0, 80.0));
    assert_eq!(graph.first_layer().plot_style(&key).unwrap().symbol_size, Some(9.0));
    let series = graph.first_layer().series(target.book(target_id).unwrap()).1;
    assert!(series.iter().any(|s| s.points.iter().any(|point| point[1] == 60.0)));

    let project = project_file::to_json(&target).unwrap();
    assert_eq!(project_file::from_json(&project).unwrap(), target);
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&target).unwrap()).unwrap(),
        target
    );
}

#[test]
fn export_still_rejects_inactive_ranges_and_preserves_validation_error_order() {
    for (key, range, expected) in [
        ("hidden", (3, 1), "图层行范围无效"),
        ("hidden", (0, usize::MAX), "图层行范围无效"),
        ("B", (3, 1), "曲线行范围无效"),
    ] {
        let mut p = Project::sample();
        p.graphs[0]
            .first_layer_mut()
            .edit_draft(|draft| draft.row_by_y.insert(key.into(), Some(range)));
        assert_eq!(project_file::to_json(&p).unwrap_err(), expected);
        assert_eq!(project_file::to_scv(&p).unwrap_err(), expected);
    }
}

fn vector_compat_fixture(mode: &str) -> Project {
    use scicanvas_core::{page::CurveStyle, plotspec::DataRef, Role};
    let mut p = Project::sample();
    let foreign = p.new_book();
    let book = p.book_mut(foreign).unwrap();
    book.push_column(Role::Y);
    for (i, c) in book.columns.iter_mut().enumerate() {
        c.data = (0..6).map(|r| (r + i * 10).to_string()).collect();
    }
    p.add_curves(2, 0, foreign, &["B".into()]).unwrap();
    {
        let books = &p.books;
        let lookup = |id| books.iter().find(|b| b.id == id);
        let layer = p.graphs[0].first_layer_mut();
        layer.rows = Some((1, 3));
        layer.labels.column = Some("C".into());
        layer
            .plot_mut(&DataRef::new(foreign, "B"), &lookup, |plot| {
                plot.mapping.rows = Some((0, 2));
                plot.style = Some(CurveStyle {
                    right_axis: true,
                    legend_label: "β^{2}".into(),
                    ..Default::default()
                });
            })
            .unwrap();
        layer.set_plot_style("C", Some(CurveStyle::default()));
        layer.set_plot_style(
            "hidden",
            Some(CurveStyle {
                right_axis: true,
                ..Default::default()
            }),
        );
    }
    let (gid, _) = p
        .plot_from_selection(1, &["B".into(), "C".into()], PlotType::FloatingBar)
        .unwrap();
    p.add_curves(gid, 0, foreign, &["B".into(), "C".into()]).unwrap();
    match mode {
        "missing" => p.delete_columns(1, &["B".into()]),
        "unbound" => p.graphs[0].first_layer_mut().book_id = None,
        "inactive" => p.graphs[0]
            .first_layer_mut()
            .edit_plot_keys(|keys| keys.retain(|k| k != "C")),
        "deleted-main" => {
            p.remove(1);
        }
        "analysis" => {
            p.smooth(
                &scicanvas_core::ops::Source::Graph(2),
                scicanvas_core::analysis::SmoothMethod::MovingAverage,
                3,
            )
            .unwrap();
        }
        "rows" => {
            p.insert_rows(foreign, 1, 2);
            p.delete_rows(1, 0, 1);
        }
        _ => {}
    }
    p
}

#[test]
fn vec_storage_matches_pre_refactor_v14_json_and_svg_bytes() {
    fixture_fonts();
    let hash = |bytes: &[u8]| {
        bytes.iter().fold(0xcbf29ce484222325_u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        })
    };
    // 基线由 a8665fd 的真实 release 核心库生成，使用内置字体。
    for (mode, json_hash, svg_hash) in [
        ("plain", 10388380240591533238, 14501551175884022413),
        ("missing", 5903541544429849591, 6683444684052755176),
        ("unbound", 14290394467229394458, 14193909908209621572),
        ("inactive", 9382714990286152969, 120544546637645886),
        ("deleted-main", 5278304510421783194, 1315333848789204538),
        ("rows", 5101616968945152157, 15733227137332039755),
        ("analysis", 13022549547118744312, 10274796173482420218),
    ] {
        let p = vector_compat_fixture(mode);
        let json = legacy_v14_projection(&p);
        let svg = p
            .graphs
            .iter()
            .map(|g| svg::render_page(g, &p))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(hash(json.as_bytes()), json_hash, "{mode}: v14 JSON");
        assert_eq!(hash(svg.as_bytes()), svg_hash, "{mode}: SVG");
        let saved = project_file::from_json(&json).unwrap();
        assert_eq!(
            project_file::to_json(&saved).unwrap(),
            project_file::to_json(&p).unwrap()
        );
        assert_eq!(
            project_file::from_json(&project_file::to_json(&saved).unwrap()).unwrap(),
            saved
        );
        assert_eq!(
            project_file::from_scv(&project_file::to_scv(&p).unwrap()).unwrap(),
            saved
        );
    }
}

// 将测试输入明确投影为 v10 的旧字段，保持原有旧格式验收场景。
fn legacy_editor_projection(p: &Project) -> serde_json::Value {
    let mut value = serde_json::to_value(p).unwrap();
    for (graph, wire) in p.graphs.iter().zip(value["graphs"].as_array_mut().unwrap()) {
        for (layer, wire) in graph
            .page
            .layers()
            .iter()
            .zip(wire["page"]["layers"].as_array_mut().unwrap())
        {
            wire.as_object_mut().unwrap().remove("plots");
            let draft = serde_json::to_value(layer.draft()).unwrap();
            for (key, value) in draft.as_object().unwrap() {
                wire[key] = value.clone();
            }
            if !layer.styles().is_empty() {
                wire["curves"] = serde_json::to_value(layer.styles()).unwrap();
            }
        }
    }
    value
}

#[test]
fn vector_wire_keeps_plots_authoritative_and_legacy_required_fields_checked() {
    fixture_fonts();
    let p = Project::sample();
    let mut wire: serde_json::Value = serde_json::from_str(&project_file::to_json(&p).unwrap()).unwrap();
    let layer = &mut wire["graphs"][0]["page"]["layers"][0];
    for key in ["y_keys", "x_by_y", "base_by_y", "row_by_y", "line_only", "curves"] {
        layer[key] = "obsolete".into();
    }
    assert_eq!(project_file::from_json(&wire.to_string()).unwrap(), p);
    let mut legacy = legacy_editor_projection(&p);
    legacy["version"] = 10.into();
    legacy["graphs"][0]["page"]["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("y_keys");
    assert!(project_file::from_json(&legacy.to_string()).is_err());
}

#[test]
fn legacy_invalid_explicit_refs_remain_distinct_from_unbound_and_missing_base() {
    fixture_fonts();
    for field in ["y_keys", "x_by_y", "base_by_y"] {
        for invalid in ["B@0", "@1"] {
            let mut value = legacy_editor_projection(&Project::sample());
            value["version"] = 10.into();
            let layer = &mut value["graphs"][0]["page"]["layers"][0];
            if field == "y_keys" {
                layer[field] = serde_json::json!([invalid]);
            } else {
                layer[field] = serde_json::json!({"B":invalid});
            }
            assert!(
                project_file::from_json(&value.to_string()).is_err(),
                "{field}: {invalid}"
            );
        }
    }
    let mut p = Project::sample();
    p.graphs[0].first_layer_mut().book_id = None;
    let text = project_file::to_json(&p).unwrap();
    assert_eq!(project_file::from_json(&text).unwrap(), p);
}

#[test]
fn custom_fit_library_v18_roundtrips_and_v17_defaults_to_an_empty_library() {
    use scicanvas_core::nlfit::{CustomFitDefinition, FitFunction};
    fixture_fonts();
    let mut project = Project::sample();
    let old_svg = svg::render_page(&project.graphs[0], &project);
    let mut legacy = serde_json::to_value(&project).unwrap();
    legacy["version"] = 17.into();
    assert!(legacy.get("fit_functions").is_none());
    let loaded = project_file::from_json(&legacy.to_string()).unwrap();
    assert!(loaded.fit_functions.is_empty());
    assert_eq!(loaded.version, project_file::CURRENT_VERSION);
    assert_eq!(svg::render_page(&loaded.graphs[0], &loaded), old_svg);
    let definition = CustomFitDefinition::from_text("Decay", "a*exp(-b*x)+c", "a,b,c", "2,0.3,0.2", "b >= 0").unwrap();
    let id = project.save_fit_function(None, definition.clone()).unwrap();
    let file = project_file::fit_function_to_json(&definition).unwrap();
    assert!(!file.contains("\"id\""));
    let restored = project_file::fit_function_from_json(&file).unwrap();
    assert_eq!(restored, definition);
    assert_eq!(
        project_file::from_json(&project_file::to_json(&project).unwrap()).unwrap(),
        project
    );
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&project).unwrap()).unwrap(),
        project
    );
    let mut stale = serde_json::to_value(&project).unwrap();
    stale["next_id"] = 1.into();
    assert!(project_file::from_json(&stale.to_string()).unwrap().next_id > id);
    let xs: Vec<f64> = (0..40).map(|i| i as f64 * 0.25).collect();
    let ys: Vec<f64> = xs.iter().map(|x| 2.2 * (-0.4 * x).exp() + 0.3).collect();
    let (function, constraints) = restored.resolve().unwrap();
    let fit = scicanvas_core::nlfit::fit_function(
        FitFunction::Custom(function),
        &xs,
        &ys,
        Some(&restored.initial),
        &[],
        &constraints,
    )
    .unwrap();
    assert!(fit.converged);
    for (actual, expected) in fit.params.iter().zip([2.2, 0.4, 0.3]) {
        assert!((actual - expected).abs() < 1e-7);
    }
    for kind in ["duplicate-name", "duplicate-id", "formula", "initial"] {
        let mut wire = serde_json::to_value(&project).unwrap();
        match kind {
            "duplicate-name" => {
                let mut duplicate = wire["fit_functions"][0].clone();
                duplicate["id"] = (id + 100).into();
                wire["fit_functions"].as_array_mut().unwrap().push(duplicate);
            }
            "duplicate-id" => wire["fit_functions"][0]["id"] = project.books[0].id.into(),
            "formula" => wire["fit_functions"][0]["definition"]["formula"] = "a*x+missing".into(),
            _ => wire["fit_functions"][0]["definition"]["initial"] = serde_json::json!([1]),
        }
        assert!(project_file::from_json(&wire.to_string()).is_err(), "{kind}");
    }
}

#[test]
fn custom_fit_files_validate_inputs_and_failed_saves_preserve_existing_bytes() {
    use scicanvas_core::nlfit::CustomFitDefinition;
    let definition = CustomFitDefinition::from_text("直线", "a*x+b", "a,b", "2,1", "").unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("scicanvas-function-{}-{stamp}.scfn", std::process::id()));
    project_file::save_fit_function(&path, &definition).unwrap();
    assert_eq!(project_file::load_fit_function(&path).unwrap(), definition);
    let original = std::fs::read(&path).unwrap();
    for kind in [
        "newer-version",
        "formula",
        "initial",
        "parameters",
        "constraints",
        "unknown-field",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match kind {
            "newer-version" => value["version"] = 2.into(),
            "formula" => value["function"]["formula"] = "a*x+missing".into(),
            "initial" => value["function"]["initial"] = serde_json::json!([null, 1]),
            "parameters" => value["function"]["parameters"] = serde_json::json!(["a", "a"]),
            "constraints" => value["function"]["constraints"] = "c >= 1".into(),
            _ => value["extra"] = true.into(),
        }
        assert!(
            project_file::fit_function_from_json(&value.to_string()).is_err(),
            "{kind}"
        );
    }
    assert!(project_file::fit_function_from_json(&" ".repeat(65537)).is_err());
    assert!(project_file::fit_function_from_json("not JSON").is_err());
    let bad = CustomFitDefinition {
        initial: vec![f64::INFINITY, 1.0],
        ..definition
    };
    assert!(project_file::save_fit_function(&path, &bad).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}
