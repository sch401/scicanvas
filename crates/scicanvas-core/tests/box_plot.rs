#![cfg(feature = "serde")]
//! 箱线图的类别轴、样本来源、标记及项目保存。
use scicanvas_core::{
    page::CurveStyle, plotspec::Transform, project_file, scene, Book, Column, PlotType, Project, Role,
};

fn project(columns: &[(&str, &[&str])], x: Option<Column>) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut book = Book::new(id, "箱线图样本");
    book.columns.extend(x);
    for (key, values) in columns {
        let mut column = Column::new(*key, Role::Y);
        column.long_name = format!("样品{key}");
        column.units = "mg/L".into();
        column.data = values.iter().map(|s| s.to_string()).collect();
        book.columns.push(column);
    }
    p.books.push(book);
    p.plot_from_selection(id, &[], PlotType::Box).unwrap();
    p
}

#[test]
fn boxes_use_column_names_on_a_category_axis_and_hide_the_legend_by_default() {
    let mut p = project(
        &[("A", &["1", "2", "3", "4", "40"]), ("B", &["2", "4", "6", "8", "10"])],
        None,
    );
    let layer = p.graphs[0].first_layer();
    assert!(!layer.legend);
    assert_eq!(layer.x_axis_title(Some(&p.books[0])), "");
    assert_eq!(layer.y_axis_title(Some(&p.books[0])), "样品A (mg/L)");
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert_eq!((sc.axes.x.lo, sc.axes.x.hi), (0.5, 2.5));
    assert_eq!(sc.axes.x.ticks, [1.0, 2.0]);
    assert!(sc.axes.left_y.hi > 40.0);
    for label in ["样品A", "样品B"] {
        assert!(sc
            .items
            .iter()
            .any(|i| matches!(&i.prim, scene::Prim::Text { text, .. } if text == label)));
    }
    p.graphs[0].first_layer_mut().legend = true;
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert_eq!(
        sc.items
            .iter()
            .filter(|item| matches!(&item.prim, scene::Prim::Text { text, .. } if text == "样品A"))
            .count(),
        2
    );
}

#[test]
fn boxes_ignore_x_mean_transforms_curve_types_and_measurement_errors() {
    let mut x = Column::new("X", Role::X);
    x.data = ["", "NaN", "1", "1", "1"].map(String::from).to_vec().into();
    let mut p = project(&[("A", &["1", "2", "3", "4", "40"])], Some(x));
    let mut error = Column::new("E", Role::YError);
    error.error_for = Some("A".into());
    error.data = vec!["10".to_string(); 5].into();
    p.books[0].columns.push(error);
    let layer = p.graphs[0].first_layer_mut();
    layer.transform = Transform::Mean;
    layer.set_plot_style(
        "A",
        Some(CurveStyle {
            kind: Some(PlotType::Bar),
            ..Default::default()
        }),
    );
    let (_, plots) = layer.prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(plots[0].style.kind, PlotType::Box);
    assert_eq!(plots[0].series.points.len(), 5);
    assert!(plots[0].series.points.iter().all(|p| p[0] == 1.0));
    assert!(plots[0].series.errors.iter().all(|e| *e == [None, None]));
    assert_eq!(plots[0].box_stats.as_ref().unwrap().outliers, [40.0]);
}

#[test]
fn boxes_draw_full_width_bodies_outliers_and_hollow_means() {
    let p = project(&[("A", &["1", "2", "3", "4", "40"])], None);
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    let widths: Vec<_> = sc
        .items
        .iter()
        .filter_map(|item| match item.prim {
            scene::Prim::Rect {
                w,
                fill: Some((_, opacity)),
                ..
            } if item.clip.is_some() => {
                assert_eq!(opacity, 0.35);
                Some(w)
            }
            _ => None,
        })
        .collect();
    assert_eq!(widths.len(), 1);
    assert!((widths[0] - sc.plot[2] * 0.6).abs() < 1e-10);
    assert!(sc
        .items
        .iter()
        .any(|item| matches!(&item.prim, scene::Prim::Symbols { pts, .. } if pts.len() == 1)));
    assert!(sc.items.iter().any(|item| matches!(&item.prim, scene::Prim::StyledSymbols { symbols } if symbols.len() == 1 && symbols[0].fill.is_none())));
}

#[test]
fn empty_constant_and_single_value_boxes_render_without_panicking() {
    for values in [&[][..], &["NaN"][..], &["5"][..], &["5", "5", "5", "5"][..]] {
        let p = project(&[("A", values)], None);
        let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
        assert_eq!(plots[0].box_stats.is_some(), values.contains(&"5"));
        scene::layout_page(&p.graphs[0], &p, &[]);
    }
}

#[test]
fn boxes_roundtrip_in_v27_and_read_v26_projects() {
    let p = project(&[("A", &["1", "2", "3", "4", "40"])], None);
    assert_eq!(project_file::from_scv(&project_file::to_scv(&p).unwrap()).unwrap(), p);
    assert_eq!(p.version, scicanvas_core::model::PROJECT_VERSION);
    let mut legacy: serde_json::Value =
        serde_json::from_str(&project_file::to_json(&Project::sample()).unwrap()).unwrap();
    legacy["version"] = 26.into();
    assert_eq!(
        project_file::from_json(&legacy.to_string()).unwrap().version,
        scicanvas_core::model::PROJECT_VERSION
    );
}
