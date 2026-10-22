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
    assert!(!p.graphs[0].first_layer().x_by_y.is_empty());
    assert!(!p.graphs[0].first_layer().line_only.is_empty());
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
