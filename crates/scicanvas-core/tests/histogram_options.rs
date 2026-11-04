#![cfg(feature = "serde")]
//! 直方图设置、分布曲线以及 HIST-1 的取样和取点回归。
use scicanvas_core::{
    page::{HistogramBins, HistogramOptions},
    plotspec::Transform,
    project_file, scene, Book, Column, PlotType, Project, Role,
};

fn project(values: &[&str], x: Option<Column>) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut book = Book::new(id, "分布");
    book.columns.extend(x);
    let mut y = Column::new("A", Role::Y);
    y.data = values.iter().map(|v| v.to_string()).collect();
    book.columns.push(y);
    p.books.push(book);
    p.plot_from_selection(id, &[], PlotType::Histogram).unwrap();
    p
}

fn missing_x() -> Column {
    let mut x = Column::new("X", Role::X);
    x.data = ["", "NaN", "1", "1", "1", "1"].map(String::from).to_vec().into();
    x
}

#[test]
fn histogram_sampling_keeps_y_values_when_x_is_missing() {
    let p = project(&["1", "2", "3", "4", "5", "6"], Some(missing_x()));
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(plots[0].series.points.iter().map(|p| p[1]).sum::<f64>(), 6.0);
}

#[test]
fn histogram_skips_the_x_group_mean_transform() {
    let mut p = project(&["1", "2", "3", "4", "5", "6"], Some(missing_x()));
    let (_, before) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    p.graphs[0].first_layer_mut().transform = Transform::Mean;
    let (_, after) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(before[0].series.points, after[0].series.points);
}

#[test]
fn histogram_bin_hits_have_no_worksheet_row() {
    use scicanvas_core::coord::Coord;
    let p = project(&["1", "2", "3", "4", "5", "6"], None);
    let layer = p.graphs[0].first_layer();
    let (_, plots) = layer.prepared_plots_in(&p.books[0], &|_| None);
    let point = plots[0].series.points[0];
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    let view = &sc.layers[0];
    let [x, y] = view
        .axes
        .cartesian(view.plot, scicanvas_core::coord::YAxis::Left)
        .project(point);
    let hit = scene::nearest_point(layer, view, &|id| p.book(id), x, y, 2.0).unwrap();
    assert_eq!(hit.row, None);
    assert_eq!([hit.x, hit.y], point);
}

#[test]
fn invalid_histogram_options_fall_back_to_automatic_bins() {
    let mut p = project(&["1", "2", "3", "4", "5", "6"], None);
    let (_, default) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    for options in [
        HistogramOptions {
            bins: HistogramBins::Count(0),
            ..Default::default()
        },
        HistogramOptions {
            bins: HistogramBins::Count(usize::MAX),
            ..Default::default()
        },
        HistogramOptions {
            bins: HistogramBins::Width(0.0),
            ..Default::default()
        },
        HistogramOptions {
            bins: HistogramBins::Width(-1.0),
            ..Default::default()
        },
        HistogramOptions {
            bins: HistogramBins::Width(f64::NAN),
            ..Default::default()
        },
        HistogramOptions {
            range: Some([3.0, 2.0]),
            ..Default::default()
        },
        HistogramOptions {
            range: Some([2.0, 2.0]),
            ..Default::default()
        },
        HistogramOptions {
            range: Some([f64::NAN, 2.0]),
            ..Default::default()
        },
    ] {
        assert!(!options.is_valid());
        p.graphs[0].first_layer_mut().histogram = options;
        let (_, actual) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
        assert_eq!(actual[0].series.points, default[0].series.points);
    }
    p.graphs[0].first_layer_mut().histogram = HistogramOptions {
        bins: HistogramBins::Width(1e-300),
        ..Default::default()
    };
    let (_, actual) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(actual[0].series.points, default[0].series.points);
}

#[test]
fn one_bin_keeps_its_actual_width_and_the_normal_curve_sets_the_y_range() {
    use scicanvas_core::coord::Coord;
    let mut p = project(&["-5", "0", "0", "0", "0", "5"], None);
    p.graphs[0].first_layer_mut().histogram = HistogramOptions {
        bins: HistogramBins::Count(1),
        normal_curve: true,
        ..Default::default()
    };
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(plots[0].histogram_width, Some(10.0));
    let peak = plots[0].paths[0].iter().map(|p| p[1]).fold(0.0, f64::max);
    assert!(peak > 6.0);
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert!(sc.axes.left_y.hi >= peak);
    assert!(sc.axes.x.lo <= -5.0 && sc.axes.x.hi >= 5.0);
    let view = &sc.layers[0];
    let map = view.axes.cartesian(view.plot, scicanvas_core::coord::YAxis::Left);
    let expected = map.project([5.0, 0.0])[0] - map.project([-5.0, 0.0])[0];
    let width = sc
        .items
        .iter()
        .find_map(|item| match item.prim {
            scene::Prim::Rect { w, .. } if item.clip.is_some() => Some(w),
            _ => None,
        })
        .unwrap();
    assert!((width - expected).abs() < 1e-10);
    assert!(
        sc.items
            .iter()
            .any(|item| item.clip.is_some()
                && matches!(&item.prim, scene::Prim::Polyline { pts, .. } if pts.len() == 100))
    );
}

#[test]
fn constant_and_single_value_samples_keep_bars_without_a_normal_curve() {
    for values in [&["3"][..], &["3", "3", "3"][..]] {
        let mut p = project(values, None);
        p.graphs[0].first_layer_mut().histogram.normal_curve = true;
        let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
        assert!(plots[0].paths.is_empty());
        assert_eq!(
            plots[0].series.points.iter().map(|p| p[1]).sum::<f64>(),
            values.len() as f64
        );
    }
}

#[test]
fn histogram_defaults_are_omitted_and_options_roundtrip_in_v28() {
    let mut p = project(&["1", "2", "3", "4", "5", "6"], None);
    assert!(!project_file::to_json(&p).unwrap().contains("\"histogram\""));
    p.graphs[0].first_layer_mut().histogram = HistogramOptions {
        bins: HistogramBins::Width(0.5),
        range: Some([1.0, 5.0]),
        cumulative: true,
        normal_curve: true,
    };
    assert_eq!(project_file::from_scv(&project_file::to_scv(&p).unwrap()).unwrap(), p);
    assert_eq!(p.version, scicanvas_core::PROJECT_VERSION);
    assert_eq!(p.graphs[0].first_layer().y_axis_title(Some(&p.books[0])), "累积计数");
    let mut legacy: serde_json::Value =
        serde_json::from_str(&project_file::to_json(&Project::sample()).unwrap()).unwrap();
    legacy["version"] = 27.into();
    let loaded = project_file::from_json(&legacy.to_string()).unwrap();
    assert!(loaded.graphs[0].first_layer().histogram.is_default());
    assert_eq!(loaded.version, scicanvas_core::PROJECT_VERSION);
}
