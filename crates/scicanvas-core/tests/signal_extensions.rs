#![cfg(feature = "serde")]
use scicanvas_core::{
    baseline::{self, BaselineMethod},
    interpolate::InterpMethod,
    ops::{MergeDuplicates, OpKind, Source},
    plotspec::DataRef,
    signal::{self, ConvolutionMode},
    Book, Column, PlotType, Project, Role,
};
use serde_json::Value;
fn vec(case: &Value, key: &str) -> Vec<f64> {
    case[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}
#[test]
fn a6_baselines_match_numpy_and_natural_spline() {
    let doc: Value = serde_json::from_str(include_str!("reference/baseline_extensions.json")).unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let method = if case["kind"] == "Polynomial" {
            BaselineMethod::Polynomial {
                order: case["order"].as_u64().unwrap() as u8,
                iterations: case["iterations"].as_u64().unwrap() as u16,
            }
        } else {
            BaselineMethod::Anchors {
                count: case["count"].as_u64().unwrap() as u16,
                spline: case["spline"].as_bool().unwrap(),
            }
        };
        let actual = baseline::with_anchors(&vec(case, "x"), &vec(case, "y"), method, &vec(case, "anchors")).unwrap();
        for (a, b) in actual.iter().zip(vec(case, "baseline")) {
            assert!((a - b).abs() < 1e-8, "{method:?}: {a} != {b}");
        }
    }
}
#[test]
fn a6_derivative_peaks_match_scipy_crossings_and_half_sample_positions() {
    let doc: Value = serde_json::from_str(include_str!("reference/derivative_peaks.json")).unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let x = vec(case, "x");
        let y = vec(case, "y");
        let peaks = scicanvas_core::peaks::find_peaks_derivative(&x, &y, 11, 0.05);
        let indices: Vec<_> = peaks.iter().map(|p| p.index).collect();
        let expected: Vec<_> = case["indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(indices, expected);
        assert_eq!(peaks.len(), 2);
        for (p, center) in peaks.iter().zip(vec(case, "centers")) {
            assert!((p.x - center).abs() < (x[1] - x[0]) / 2.);
        }
    }
}
#[test]
fn a6_discrete_convolution_and_deconvolution_match_numpy() {
    let doc: Value = serde_json::from_str(include_str!("reference/signal_extensions.json")).unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let actual = if case["kind"] == "Convolve" {
            let mode = match case["mode"].as_str().unwrap() {
                "Full" => ConvolutionMode::Full,
                "Same" => ConvolutionMode::Same,
                _ => ConvolutionMode::Valid,
            };
            signal::convolve(&vec(case, "a"), &vec(case, "b"), mode)
        } else {
            signal::deconvolve(&vec(case, "y"), &vec(case, "h"), case["epsilon"].as_f64().unwrap())
        };
        let expected = vec(case, "result");
        assert_eq!(actual.len(), expected.len());
        for (a, b) in actual.iter().zip(expected) {
            assert!((a - b).abs() < 1e-10, "{a} != {b}");
        }
        if case["kind"] == "Deconvolve" && case["epsilon"] == 0. {
            for (a, b) in actual.iter().zip(vec(case, "original")) {
                assert!((a - b).abs() < 1e-9);
            }
        }
    }
}
fn book(id: u64, x: &[f64], y: &[f64]) -> Book {
    let mut book = Book::new(id, "A6");
    for (key, role, values) in [("A", Role::X, x), ("B", Role::Y, y)] {
        let mut c = Column::new(key, role);
        c.data = values.iter().map(ToString::to_string).collect();
        book.columns.push(c);
    }
    book
}
#[test]
fn a6_merge_stable_order_duplicates_and_original_books() {
    for duplicate in [MergeDuplicates::Mean, MergeDuplicates::KeepAll] {
        let mut p = Project::default();
        let a = p.alloc_id();
        p.books.push(book(a, &[4., 0., 2.], &[4., 0., 2.]));
        let b = p.alloc_id();
        p.books.push(book(b, &[5., 1., 2.], &[50., 10., 20.]));
        let (gid, _) = p.plot_from_selection(a, &[], PlotType::Line).unwrap();
        p.add_curves(gid, 0, b, &["B".into()]).unwrap();
        let before = p.books.clone();
        let (out, _) = p.merge_curves(&Source::Graph(gid), duplicate).unwrap();
        assert_eq!(p.books[..2], before);
        let book = p.book(out).unwrap();
        let x = book.column("A").unwrap().values();
        let y = book.column("B").unwrap().values();
        let reference: Value = serde_json::from_str(include_str!("reference/curve_tools.json")).unwrap();
        let kind = if duplicate == MergeDuplicates::Mean {
            "Mean"
        } else {
            "KeepAll"
        };
        let case = reference["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["kind"] == kind)
            .unwrap();
        assert_eq!(x, vec(case, "x"));
        assert_eq!(y, vec(case, "y"));
    }
}
#[test]
fn a6_target_x_order_missing_values_and_single_query_are_read_only() {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut b = book(id, &[0., 1., 2., 3.], &[0., 2., 4., 6.]);
    let mut c = Column::new("C", Role::X);
    c.data = ["2.5", "", "0.5", "1.5"].map(str::to_owned).to_vec().into();
    b.columns.push(c);
    p.books.push(b);
    let src = Source::Book {
        id,
        selected: vec!["B".into()],
        rows: None,
    };
    let before = p.clone();
    let query = p.interpolate_value(&src, InterpMethod::Linear, 1.25).unwrap();
    let reference: Value = serde_json::from_str(include_str!("reference/curve_tools.json")).unwrap();
    let reference = reference["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "Interpolation")
        .unwrap();
    assert_eq!(query[0].1, reference["single_y"].as_f64().unwrap());
    assert_eq!(p, before);
    let keys = p
        .interpolate_at(&src, InterpMethod::Linear, &DataRef::new(id, "C"))
        .unwrap();
    let values = p.book(id).unwrap().column(&keys[0]).unwrap().values();
    assert_eq!(values[0], 5.);
    assert!(values[1].is_nan());
    assert_eq!(&values[2..], &[1., 3.]);
    assert_eq!(
        p.book(id).unwrap().column_index(&keys[0]),
        Some(p.book(id).unwrap().column_index("C").unwrap() + 1)
    );
}
#[test]
fn a6_anchor_recipe_save_recalculate_and_invalid_input_is_atomic() {
    let mut p = Project::default();
    let id = p.alloc_id();
    p.books
        .push(book(id, &[0., 1., 2., 3., 4., 5., 6.], &[1., 2., 4., 2., 4., 2., 1.]));
    let src = Source::Book {
        id,
        selected: vec!["B".into()],
        rows: None,
    };
    let method = BaselineMethod::Anchors { count: 3, spline: true };
    let before = p.clone();
    assert!(p.subtract_baseline_with_anchors(&src, method, &[1.]).is_err());
    assert_eq!(p, before);
    let keys = p.subtract_baseline_with_anchors(&src, method, &[0., 3., 6.]).unwrap();
    let c = p.book(id).unwrap().column(&keys[0].0).unwrap();
    assert!(matches!(&c.op.as_ref().unwrap().kind,OpKind::Baseline{anchors,..} if anchors==&[0.,3.,6.]));
    let original = c.values();
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    let mut p = scicanvas_core::project_file::from_json(&json).unwrap();
    assert!(p.recalc_computed(true).is_empty());
    assert_eq!(p.book(id).unwrap().column(&keys[0].0).unwrap().values(), original);
}
