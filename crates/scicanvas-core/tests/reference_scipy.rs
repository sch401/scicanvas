#![cfg(feature = "serde")]
//! 只读取开发机生成的参考值；CI 不依赖 Python 科学计算包。
use scicanvas_core::{
    analysis::{self, NormalizeMethod},
    ops::{ColumnOp, Operand, Source},
    Column, Project, Role,
};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn cases(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/reference")
        .join(format!("{name}.json"));
    let data: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(data["generator"], "tools/reference/gen_scipy.py");
    assert_eq!(data["versions"]["NumPy"], "2.4.4");
    assert_eq!(data["versions"]["SciPy"], "1.17.1");
    data["cases"].as_array().unwrap().clone()
}

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap_or(f64::NAN))
        .collect()
}

fn near(actual: f64, expected: &Value, tolerance: f64, context: &str) {
    if let Some(expected) = expected.as_f64() {
        let error = (actual - expected).abs();
        let scale = if expected == 0.0 { 1.0 } else { expected.abs() };
        assert!(
            actual.is_finite() && error <= tolerance * scale,
            "{context}: {actual:.17e} != {expected:.17e}, relative {}",
            error / scale
        );
    } else {
        assert!(!actual.is_finite(), "{context}: 应为缺失 / 非有限值，实际 {actual}");
    }
}

fn array(actual: &[f64], expected: &Value, tolerance: f64, context: &str) {
    let values = expected.as_array().unwrap();
    assert_eq!(actual.len(), values.len(), "{context}");
    for (i, (actual, expected)) in actual.iter().zip(values).enumerate() {
        near(*actual, expected, tolerance, &format!("{context}[{i}]"));
    }
}

fn column(key: &str, role: Role, values: &Value) -> Column {
    let mut c = Column::new(key, role);
    c.data = values
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().map(|v| v.to_string()).unwrap_or_default())
        .collect::<Vec<_>>()
        .into();
    c
}

#[test]
fn integrate_matches_numpy_scipy_with_missing_rows_and_absolute_area() {
    for case in cases("integrate") {
        let mut p = Project::default();
        let id = p.new_book();
        p.book_mut(id).unwrap().columns = vec![column("A", Role::X, &case["x"]), column("B", Role::Y, &case["y"])];
        let (keys, reports) = p
            .integrate(&Source::Book {
                id,
                selected: vec!["B".into()],
                rows: None,
            })
            .unwrap();
        let mut values = p.book(id).unwrap().column(&keys[0]).unwrap().values();
        values.resize(case["y"].as_array().unwrap().len(), f64::NAN);
        array(&values, &case["cumulative"], 1e-12, case["name"].as_str().unwrap());
        near(reports[0].area, &case["area"], 1e-12, "积分总面积");
        near(reports[0].abs_area, &case["absolute"], 1e-12, "积分绝对面积");
    }
}

#[test]
fn normalize_matches_numpy_and_scipy_sample_zscore() {
    for case in cases("normalize") {
        assert_eq!(case["ddof"], 1);
        let input: Vec<_> = numbers(&case["input"]).into_iter().filter(|v| v.is_finite()).collect();
        for (key, method) in [
            ("max", NormalizeMethod::DivideByMax),
            ("minmax", NormalizeMethod::MinMax),
            ("zscore", NormalizeMethod::ZScore),
        ] {
            array(
                &analysis::normalize(&input, method),
                &case[key],
                1e-12,
                &format!("{}/{}", case["name"], key),
            );
        }
    }
}

#[test]
fn describe_matches_numpy_and_scipy_linear_quantiles() {
    for case in cases("describe") {
        let result = analysis::describe(&numbers(&case["input"])).unwrap();
        let expected = &case["result"];
        assert_eq!(result.n, expected["n"].as_u64().unwrap() as usize);
        for (key, value) in [
            ("mean", result.mean),
            ("sd", result.sd),
            ("se", result.se),
            ("min", result.min),
            ("q1", result.q1),
            ("median", result.median),
            ("q3", result.q3),
            ("max", result.max),
            ("sum", result.sum),
        ] {
            near(value, &expected[key], 1e-10, &format!("{}/{}", case["name"], key));
        }
    }
}

#[test]
fn column_arithmetic_and_interpolated_subtraction_match_numpy() {
    for case in cases("column_math") {
        let mut p = Project::default();
        let id = p.new_book();
        let book = p.book_mut(id).unwrap();
        book.columns = vec![column("A", Role::X, &case["x"]), column("B", Role::Y, &case["a"])];
        let other = if case.get("bx").is_some() {
            book.columns.push(column("C", Role::X, &case["bx"]));
            book.columns.push(column("D", Role::Y, &case["b"]));
            "D"
        } else {
            book.columns.push(column("C", Role::Y, &case["b"]));
            "C"
        };
        let op = match case["op"].as_str().unwrap() {
            "add" => ColumnOp::Add,
            "sub" => ColumnOp::Sub,
            "mul" => ColumnOp::Mul,
            "div" => ColumnOp::Div,
            _ => unreachable!(),
        };
        let key = p.column_math(id, "B", op, &Operand::Column(other.into())).unwrap();
        let mut values = p.book(id).unwrap().column(&key).unwrap().values();
        values.resize(case["result"].as_array().unwrap().len(), f64::NAN);
        array(&values, &case["result"], 1e-12, case["name"].as_str().unwrap());
    }
}

#[test]
fn polynomial_degrees_two_to_six_match_numpy_with_large_x_offset() {
    for case in cases("polynomial") {
        let xs = numbers(&case["x"]);
        let fit = analysis::polyfit(&xs, &numbers(&case["y"]), case["order"].as_u64().unwrap() as usize).unwrap();
        let name = case["name"].as_str().unwrap();
        let tolerance = case.get("tolerance").and_then(Value::as_f64).unwrap_or(1e-12);
        array(&fit.coef, &case["coef"], tolerance, &format!("{name}/coef"));
        array(&fit.se, &case["se"], tolerance, &format!("{name}/se"));
        near(fit.rss, &case["rss"], tolerance, &format!("{name}/rss"));
        array(
            &xs.iter().map(|x| fit.eval(*x)).collect::<Vec<_>>(),
            &case["fitted"],
            tolerance,
            &format!("{name}/eval"),
        );
    }
}

#[test]
fn histogram_correlations_and_t_tests_match_generated_scipy_reference() {
    use scicanvas_core::stats::{self, CorrMethod, TTestKind};
    for case in cases("statistics") {
        let name = case["name"].as_str().unwrap();
        let kind = name.split('-').next().unwrap();
        match kind {
            "histogram" => {
                let limits = case.get("limits").map(numbers).unwrap_or_else(|| vec![-1.0, 3.0]);
                let bins = stats::histogram(&numbers(&case["values"]), 4, Some((limits[0], limits[1]))).unwrap();
                let counts: Vec<_> = bins.iter().map(|bin| bin.count as u64).collect();
                let expected: Vec<_> = case["counts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_u64().unwrap())
                    .collect();
                assert_eq!(counts, expected);
                let mut edges: Vec<_> = bins.iter().map(|bin| bin.lo).collect();
                edges.push(bins.last().unwrap().hi);
                array(&edges, &case["edges"], 1e-12, name);
            }
            "pearson" | "spearman" | "kendall" => {
                let method = match kind {
                    "pearson" => CorrMethod::Pearson,
                    "spearman" => CorrMethod::Spearman,
                    _ => CorrMethod::Kendall,
                };
                let r = stats::correlation(&numbers(&case["x"]), &numbers(&case["y"]), method).unwrap();
                near(r.r, &case["r"], 1e-9, name);
                near(r.p, &case["p"], 1e-9, name);
            }
            _ => {
                let kind = match kind {
                    "one" => TTestKind::OneSample {
                        mu: case.get("mu").and_then(Value::as_f64).unwrap_or(5.0),
                    },
                    "student" => TTestKind::TwoSample { equal_var: true },
                    "welch" => TTestKind::TwoSample { equal_var: false },
                    "paired" => TTestKind::Paired,
                    _ => panic!("unexpected case {name}"),
                };
                let r = stats::t_test(kind, &numbers(&case["a"]), &numbers(&case["b"]), 0.95).unwrap();
                near(r.t, &case["t"], 1e-9, name);
                near(r.df, &case["df"], 1e-9, name);
                near(r.p, &case["p"], 1e-9, name);
                array(&[r.ci.0, r.ci.1], &case["ci"], 1e-9, name);
            }
        }
    }
}

#[test]
fn fft_filters_and_correlations_match_generated_numpy_reference() {
    use scicanvas_core::{
        fft::{self, FilterKind, Window},
        stats,
    };
    for case in cases("signals") {
        let name = case["name"].as_str().unwrap();
        match name {
            "fft" => {
                let y = numbers(&case["y"]);
                let spectrum = fft::spectrum(&y, case["dt"].as_f64().unwrap(), Window::Rect).unwrap();
                let tolerance = case.get("tolerance").and_then(Value::as_f64).unwrap_or(1e-10);
                array(&spectrum.freq, &case["freq"], tolerance, name);
                array(&spectrum.re, &case["re"], tolerance, name);
                array(&spectrum.im, &case["im"], tolerance, name);
                array(
                    &fft::inverse_real(&spectrum.re, &spectrum.im, y.len()),
                    &case["y"],
                    tolerance,
                    name,
                );
            }
            "low" | "high" | "band" | "stop" => {
                let kind = match name {
                    "low" => FilterKind::LowPass(1.5),
                    "high" => FilterKind::HighPass(1.5),
                    "band" => FilterKind::BandPass(1.0, 3.0),
                    _ => FilterKind::BandStop(1.0, 3.0),
                };
                let out = fft::filter(&numbers(&case["y"]), case["dt"].as_f64().unwrap(), kind).unwrap();
                array(&out, &case["result"], 1e-10, name);
            }
            "fft-missing" => {
                assert!(fft::spectrum(&numbers(&case["y"]), case["dt"].as_f64().unwrap(), Window::Rect).is_err());
            }
            "correlation" => {
                let x = numbers(&case["x"]);
                let (lags, corr) = stats::cross_correlation(&x, &numbers(&case["z"])).unwrap();
                let want: Vec<_> = case["lags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_i64().unwrap())
                    .collect();
                assert_eq!(lags, want);
                array(&corr, &case["corr"], 1e-10, name);
                let (_, auto) = stats::cross_correlation(&x, &x).unwrap();
                array(&auto, &case["auto"], 1e-10, name);
            }
            _ => panic!("unexpected case {name}"),
        }
    }
}

#[test]
fn interpolation_matches_generated_scipy_reference() {
    use scicanvas_core::interpolate::{self, InterpMethod};
    for case in cases("interpolation") {
        let name = case["name"].as_str().unwrap();
        for (method, key) in [
            (InterpMethod::Linear, "linear"),
            (InterpMethod::Spline, "spline"),
            (InterpMethod::Akima, "akima"),
        ] {
            let got = interpolate::interpolate(
                &numbers(&case["x"]),
                &numbers(&case["y"]),
                method,
                &numbers(&case["at"]),
            );
            if name == "duplicate" {
                assert!(got.is_err());
            } else {
                array(&got.unwrap(), &case[key], 1e-10, name);
            }
        }
    }
}

#[test]
fn numerical_derivatives_match_generated_scipy_reference() {
    for case in cases("derivatives") {
        let name = case["name"].as_str().unwrap();
        let gradient = analysis::derivative(&numbers(&case["x"]), &numbers(&case["y"]));
        array(&gradient, &case["gradient"], 1e-10, name);
        let got = analysis::sg_derivative(
            &numbers(&case["x"]),
            &numbers(&case["y"]),
            case["window"].as_u64().unwrap() as usize,
            case["order"].as_u64().unwrap() as usize,
            case["derivative"].as_u64().unwrap() as usize,
        );
        if case.get("scipy_error").is_some() {
            assert!(got.iter().any(|value| !value.is_finite()), "{name}");
        } else {
            let expected = numbers(&case["result"]);
            assert_eq!(got.len(), expected.len());
            for (actual, expected) in got.iter().zip(expected) {
                assert!(
                    (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
                    "{name}: {actual} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn peaks_match_generated_scipy_and_numpy_reference() {
    for case in cases("peaks") {
        let name = case["name"].as_str().unwrap();
        let got = scicanvas_core::peaks::find_peaks(&numbers(&case["x"]), &numbers(&case["y"]), 0.5);
        let expected = case["peaks"].as_array().unwrap();
        assert_eq!(got.len(), expected.len(), "{name}");
        for (peak, reference) in got.iter().zip(expected) {
            assert_eq!(peak.index as u64, reference["index"].as_u64().unwrap());
            assert_eq!(peak.start as u64, reference["start"].as_u64().unwrap());
            assert_eq!(peak.end as u64, reference["end"].as_u64().unwrap());
            for (actual, key) in [
                (peak.prominence, "prominence"),
                (peak.width, "width"),
                (peak.fwhm, "width"),
                (peak.left_ip, "left"),
                (peak.right_ip, "right"),
                (peak.area, "area"),
                (peak.centroid, "centroid"),
            ] {
                near(actual, &reference[key], 1e-10, name);
            }
        }
    }
}

#[test]
fn constrained_fits_match_generated_scipy_reference() {
    use scicanvas_core::nlfit::{self, Model, ParamLimits};
    for case in cases("constraints") {
        let name = case["name"].as_str().unwrap();
        let x = numbers(&case["x"]);
        let y = numbers(&case["y"]);
        if name == "small" {
            assert!(nlfit::fit(Model::ExpDec1, &x, &y, None, &[]).is_err());
            continue;
        }
        let fit = if case["model"] == "ExpDec2" {
            let constraints = nlfit::parse_constraints(Model::ExpDec2, case["constraint"].as_str().unwrap()).unwrap();
            nlfit::fit_constrained(Model::ExpDec2, &x, &y, None, &[], &constraints).unwrap()
        } else if name == "bound" {
            let limits = [
                ParamLimits::default(),
                ParamLimits::default(),
                ParamLimits {
                    upper: Some(1.5),
                    ..Default::default()
                },
            ];
            nlfit::fit_with(Model::ExpDec1, &x, &y, Some(&numbers(&case["initial"])), &limits).unwrap()
        } else {
            nlfit::fit(
                Model::ExpDec1,
                &x,
                &y,
                Some(&numbers(&case["initial"])),
                &[Some(case["fixed"].as_f64().unwrap())],
            )
            .unwrap()
        };
        assert!(fit.converged, "{name}");
        array(&fit.params, &case["params"], 1e-5, name);
        array(&fit.se, &case["se"], 1e-3, name);
        near(fit.rss, &case["rss"], 1e-6, name);
    }
}

#[test]
fn builtin_models_match_generated_scipy_reference() {
    use scicanvas_core::nlfit::{self, Model};
    for case in cases("builtin_fits") {
        let name = case["name"].as_str().unwrap();
        let model_name = case["model"].as_str().unwrap();
        let model = Model::ALL
            .iter()
            .copied()
            .find(|model| model.name() == model_name)
            .unwrap();
        let result = nlfit::fit(model, &numbers(&case["x"]), &numbers(&case["y"]), None, &[]);
        if name == "small" {
            assert!(result.is_err());
            continue;
        }
        let fit = result.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(fit.converged, "{name}");
        array(&fit.params, &case["params"], 1e-5, name);
        array(&fit.se, &case["se"], 1e-3, name);
        near(fit.rss, &case["rss"], 1e-7, name);
    }
}

#[test]
fn smoothing_matches_generated_numpy_scipy_reference() {
    use scicanvas_core::analysis::SmoothMethod;
    for case in cases("smoothing") {
        let name = case["name"].as_str().unwrap();
        let x = numbers(&case["x"]);
        let y = numbers(&case["y"]);
        let window = case["window"].as_u64().unwrap() as usize;
        for (method, key) in [
            (SmoothMethod::MovingAverage, "moving"),
            (SmoothMethod::SavitzkyGolay, "sg"),
            (SmoothMethod::FftFilter, "fft"),
            (SmoothMethod::Lowess, "lowess"),
        ] {
            if case.get(key).is_none() {
                if key == "fft" {
                    assert!(scicanvas_core::fft::sample_interval(&x).is_err());
                }
                continue;
            }
            let got = analysis::smooth(&x, &y, method, window);
            let expected = numbers(&case[key]);
            assert_eq!(got.len(), expected.len(), "{name}/{key}");
            for (actual, expected) in got.iter().zip(expected) {
                if expected.is_finite() {
                    assert!(
                        (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
                        "{name}/{key}: {actual} vs {expected}"
                    );
                } else {
                    assert!(!actual.is_finite(), "{name}/{key}: {actual}");
                }
            }
        }
    }
}

#[test]
fn percent_stacked_bars_match_generated_numpy_reference() {
    use scicanvas_core::PlotType;
    for case in cases("percent_bars") {
        let name = case["name"].as_str().unwrap();
        let inputs = case["values"].as_array().unwrap();
        let mut project = Project::default();
        let id = project.new_book();
        let book = project.book_mut(id).unwrap();
        book.columns.clear();
        let mut x = Column::new("A", Role::X);
        x.data = (1..=inputs[0].as_array().unwrap().len())
            .map(|i| i.to_string())
            .collect();
        book.columns.push(x);
        for values in inputs {
            let key = book.push_column(Role::Y);
            let index = book.column_index(&key).unwrap();
            book.columns[index].data = values
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().map(|n| n.to_string()).unwrap_or_default())
                .collect();
        }
        let (graph, _) = project
            .plot_from_selection(id, &[], PlotType::PercentStackedBar)
            .unwrap();
        let series = project
            .graph(graph)
            .unwrap()
            .first_layer()
            .series(project.book(id).unwrap())
            .1;
        for (j, s) in series.iter().enumerate() {
            for (i, &row) in s.rows.iter().enumerate() {
                near(s.bases[i], &case["base"][j][row], 1e-10, name);
                near(s.points[i][1], &case["top"][j][row], 1e-10, name);
            }
        }
    }
}

#[test]
fn fit_reports_match_generated_numpy_scipy_reference() {
    use scicanvas_core::nlfit::{self, Model};
    for case in cases("fit_reports") {
        let name = case["name"].as_str().unwrap();
        let x = numbers(&case["x"]);
        let y = numbers(&case["y"]);
        let at = numbers(&case["at"]);
        if case["kind"] == "poly" {
            let (xs, ys): (Vec<_>, Vec<_>) = x
                .iter()
                .zip(&y)
                .filter(|(a, b)| a.is_finite() && b.is_finite())
                .map(|(&a, &b)| (a, b))
                .unzip();
            let f = analysis::polyfit(&xs, &ys, case["order"].as_u64().unwrap() as usize).unwrap();
            let tolerance = case.get("tolerance").and_then(Value::as_f64).unwrap_or(1e-10);
            array(&f.coef, &case["params"], tolerance, name);
            array(&f.se, &case["se"], tolerance, name);
            near(f.rss, &case["rss"], tolerance, name);
            near(f.tss, &case["tss"], tolerance, name);
            let a = f.anova();
            near(a.f, &case["f"], tolerance, name);
            near(a.p, &case["fp"], 1e-8, name);
            let (t, p): (Vec<_>, Vec<_>) = f.t_tests().into_iter().unzip();
            array(&t, &case["t"], tolerance, name);
            array(&p, &case["p"], 1e-8, name);
            let (conf, pred): (Vec<_>, Vec<_>) = at.iter().map(|&x| f.band(x, 0.95)).unzip();
            array(&conf, &case["conf"], tolerance, name);
            array(&pred, &case["pred"], tolerance, name);
        } else {
            let f = nlfit::fit(Model::ExpDec1, &x, &y, None, &[]).unwrap();
            assert!(f.converged);
            array(&f.params, &case["params"], 1e-5, name);
            array(&f.se, &case["se"], 1e-3, name);
            near(f.rss, &case["rss"], 1e-6, name);
            let (t, p): (Vec<_>, Vec<_>) = f.t_tests().into_iter().unzip();
            array(&t, &case["t"], 1e-3, name);
            array(&p, &case["p"], case["p_tolerance"].as_f64().unwrap(), name);
            let (conf, pred): (Vec<_>, Vec<_>) = at.iter().map(|&x| f.band(x, 0.95)).unzip();
            array(&conf, &case["conf"], 1e-5, name);
            array(&pred, &case["pred"], 1e-5, name);
        }
    }
}
