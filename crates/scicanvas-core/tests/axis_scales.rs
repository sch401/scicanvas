#![cfg(feature = "serde")]
use scicanvas_core::{
    coord::{Coord, YAxis},
    page::CurveStyle,
    scene,
    ticks::{axis_scaled, AxisScale},
    Project,
};
use serde_json::Value;
fn values(case: &Value, key: &str) -> Vec<f64> {
    case[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}
fn scale(name: &str) -> AxisScale {
    match name {
        "Linear" => AxisScale::Linear,
        "Log10" => AxisScale::Log10,
        "Log2" => AxisScale::Log2,
        "Ln" => AxisScale::Ln,
        "Reciprocal" => AxisScale::Reciprocal,
        "Probability" => AxisScale::Probability,
        _ => AxisScale::Logit,
    }
}
#[test]
fn b6_all_transforms_inverse_ticks_and_numpy_positions() {
    let doc: Value = serde_json::from_str(include_str!("reference/axis_scales.json")).unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let scale = scale(case["scale"].as_str().unwrap());
        let data = values(case, "values");
        let axis = axis_scaled(
            &data,
            false,
            case["lo"].as_f64().unwrap(),
            case["hi"].as_f64().unwrap(),
            scale,
            false,
            &Default::default(),
        );
        for (v, f) in data.iter().zip(values(case, "fractions")) {
            assert!((axis.frac(*v) - f).abs() < 1e-12);
            assert!((scale.inverse(scale.forward(*v)) - v).abs() < 1e-11 * v.abs().max(1.));
            assert!((axis.value(axis.frac(*v)) - v).abs() < 1e-11 * v.abs().max(1.));
        }
        let ticks = values(case, "ticks");
        assert_eq!(axis.ticks.len(), ticks.len(), "{scale:?}");
        for (a, b) in axis.ticks.iter().zip(ticks) {
            assert!((a - b).abs() < 1e-12 * b.abs().max(1.));
        }
        for (v, f) in axis.ticks.iter().zip(values(case, "tick_fractions")) {
            assert!((axis.frac(*v) - f).abs() < 1e-12);
        }
    }
    assert!(!AxisScale::Probability.domain_ok(0.));
    assert!(!AxisScale::Probability.domain_ok(100.));
    assert!(!AxisScale::Logit.domain_ok(1.));
    assert!(!AxisScale::Reciprocal.domain_ok(0.));
    assert!(AxisScale::Reciprocal.domain_ok(-2.));
    assert!(!AxisScale::Reciprocal.range_ok(-1., 1.));
}
#[test]
fn b6_save_legacy_flags_log10_and_invalid_manual_range() {
    let mut p = Project::sample();
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.set_axis_scale(true, AxisScale::Log2).unwrap();
    layer.set_axis_scale(false, AxisScale::Probability).unwrap();
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.set_axis_scale(true, AxisScale::Log10).unwrap();
    assert!(layer.x_log && layer.x_scale.is_none());
    let json: Value = serde_json::from_str(&scicanvas_core::project_file::to_json(&p).unwrap()).unwrap();
    assert_eq!(json["graphs"][0]["page"]["layers"][0]["x_log"], true);
    let mut old: Value =
        serde_json::from_str(&scicanvas_core::project_file::to_json(&Project::sample()).unwrap()).unwrap();
    old["version"] = 37.into();
    let old = scicanvas_core::project_file::from_json(&old.to_string()).unwrap();
    assert_eq!(old.graphs[0].first_layer().effective_x_scale(), AxisScale::Linear);
    assert!(!scicanvas_core::project_file::to_json(&old).unwrap().contains("x_scale"));
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.x_auto = false;
    layer.x_min = -1.;
    layer.x_max = 1.;
    let before = layer.clone();
    assert!(layer.set_axis_scale(true, AxisScale::Reciprocal).is_err());
    assert_eq!(*layer, before);
}
#[test]
fn b6_probability_domains_reciprocal_branches_and_right_axis_pick() {
    let mut p = Project::sample();
    for (key, data) in [
        ("A", vec![1., 2., 3., 4.]),
        ("B", vec![1., 2., 4., 8.]),
        ("C", vec![-1., -2., -4., -8.]),
    ] {
        p.book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == key)
            .unwrap()
            .data = data.iter().map(ToString::to_string).collect();
    }
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.edit_draft(|d| d.y_keys = vec!["B".into(), "C".into()]);
    layer.set_plot_style(
        "C",
        Some(CurveStyle {
            right_axis: true,
            ..Default::default()
        }),
    );
    layer.set_axis_scale(false, AxisScale::Reciprocal).unwrap();
    let (_, plots) = p
        .graph(2)
        .unwrap()
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|_| None);
    assert!(plots.iter().all(|p| p.series.points.len() == 4));
    let sc = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
    assert!(sc.axes.left_y.lo > 0.);
    assert!(sc.axes.right_y.as_ref().unwrap().hi < 0.);
    let at = sc.axes.cartesian(sc.plot, YAxis::Right).project([3., -4.]);
    let hit = scene::nearest_point(
        p.graph(2).unwrap().first_layer(),
        &sc.layers[0],
        &|id| p.book(id),
        at[0],
        at[1],
        1.,
    )
    .unwrap();
    assert_eq!((hit.key, hit.row, hit.y), ("C".into(), Some(2), -4.));
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.set_plot_style("C", None);
    layer.edit_draft(|d| d.y_keys = vec!["B".into()]);
    layer.set_axis_scale(false, AxisScale::Probability).unwrap();
    p.book_mut(1)
        .unwrap()
        .columns
        .iter_mut()
        .find(|c| c.key == "B")
        .unwrap()
        .data = ["0", "25", "100", "75"].map(str::to_owned).to_vec().into();
    let (_, plots) = p
        .graph(2)
        .unwrap()
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|_| None);
    assert_eq!(plots[0].series.rows, vec![1, 3]);
}
