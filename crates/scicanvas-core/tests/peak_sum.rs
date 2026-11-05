use scicanvas_core::{
    nlfit::{self, FitFunction, Model, PeakBaseline, PeakSum},
    ops::{FitWeight, OpKind, Source},
    Project,
};
#[test]
fn a5_single_constant_peak_matches_builtin_bits_after_permutation() {
    let x: Vec<_> = (0..201).map(|i| -5. + i as f64 * 0.05).collect();
    let y: Vec<_> = x
        .iter()
        .enumerate()
        .map(|(i, x)| Model::Gauss.eval(*x, &[0.3, 0.4, 1.2, 3.]) + 0.002 * (i as f64 * 1.7).sin())
        .collect();
    let ordinary = nlfit::fit_function(FitFunction::Builtin(Model::Gauss), &x, &y, None, &[], &[]).unwrap();
    let peak = PeakSum::new(Model::Gauss, 1, PeakBaseline::Constant).unwrap();
    let result = nlfit::fit_function(FitFunction::PeakSum(peak), &x, &y, None, &[], &[]).unwrap();
    let order = [1, 2, 3, 0];
    for (j, i) in order.into_iter().enumerate() {
        assert_eq!(ordinary.params[i].to_bits(), result.params[j].to_bits());
        assert_eq!(ordinary.se[i].to_bits(), result.se[j].to_bits());
        for (b, a) in order.into_iter().enumerate() {
            assert_eq!(ordinary.cov[i][a].to_bits(), result.cov[j][b].to_bits());
        }
    }
    assert_eq!(ordinary.rss.to_bits(), result.rss.to_bits());
}
#[cfg(feature = "serde")]
#[test]
fn a5_peak_sum_matches_scipy_both_starts_and_component_integrals() {
    let doc: serde_json::Value = serde_json::from_str(include_str!("reference/peak_sums.json")).unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let shape = if case["shape"] == "Gauss" {
            Model::Gauss
        } else {
            Model::Lorentz
        };
        let baseline = if case["baseline"] == "Linear" {
            PeakBaseline::Linear
        } else {
            PeakBaseline::Constant
        };
        let peak = PeakSum::new(shape, case["count"].as_u64().unwrap() as u8, baseline).unwrap();
        let values = |key: &str| {
            case[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect::<Vec<_>>()
        };
        let x = values("x");
        let y = values("y");
        let initial = values("initial");
        for start in [None, Some(initial.as_slice())] {
            let fit = nlfit::fit_function(FitFunction::PeakSum(peak.clone()), &x, &y, start, &[], &[]).unwrap();
            assert!(fit.converged, "{}", fit.model.name());
            for (a, b) in fit.params.iter().zip(values("params")) {
                assert!((a - b).abs() < 1e-6 * b.abs().max(1.));
            }
            for (a, b) in fit.se.iter().zip(values("se")) {
                assert!((a - b).abs() < 1e-5 * b.abs().max(1e-8));
            }
            let components: Vec<Vec<_>> = (0..peak.count as usize)
                .map(|i| x.iter().map(|x| peak.component(*x, &fit.params, i)).collect())
                .collect();
            let n = usize::from(peak.count) * 3;
            let total: Vec<_> = x
                .iter()
                .map(|x| {
                    fit.eval(*x)
                        - fit.params[n]
                        - if baseline == PeakBaseline::Linear {
                            fit.params[n + 1] * x
                        } else {
                            0.
                        }
                })
                .collect();
            let trap = |y: &[f64]| {
                x.windows(2)
                    .zip(y.windows(2))
                    .map(|(x, y)| (x[1] - x[0]) * (y[0] + y[1]) / 2.)
                    .sum::<f64>()
            };
            assert!((components.iter().map(|c| trap(c)).sum::<f64>() - trap(&total)).abs() < 1e-6);
        }
    }
}
#[test]
fn a5_peak_count_limits_and_missing_peaks() {
    assert!(PeakSum::new(Model::Gauss, 11, PeakBaseline::Constant).is_err());
    assert!(PeakSum::new(Model::ExpDec1, 2, PeakBaseline::None).is_err());
    assert_eq!(
        PeakSum::new(Model::Voigt, 10, PeakBaseline::Linear)
            .unwrap()
            .params()
            .len(),
        42
    );
    assert!(PeakSum::new(Model::Gauss, 2, PeakBaseline::None)
        .unwrap()
        .guess(&[0., 1., 2.], &[0., 1., 0.], 0.05)
        .unwrap_err()
        .contains("降低峰数"));
}
#[cfg(feature = "serde")]
#[test]
fn a5_components_share_recalculation_ownership_and_save() {
    let mut p = Project::sample();
    let x: Vec<_> = (0..201).map(|i| -5. + i as f64 * 0.05).collect();
    let peak = PeakSum::new(Model::Gauss, 2, PeakBaseline::Constant).unwrap();
    let initial = [-1.5, 1.2, 3., 1.5, 1.4, 4., 0.3];
    let y: Vec<_> = x.iter().map(|x| peak.eval(*x, &initial)).collect();
    for (key, values) in [("A", x), ("B", y)] {
        p.book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == key)
            .unwrap()
            .data = values.iter().map(ToString::to_string).collect();
    }
    let source = Source::Book {
        id: 1,
        selected: vec!["B".into()],
        rows: None,
    };
    let (_, reports) = p
        .nlfit_weighted(
            &source,
            FitFunction::PeakSum(peak),
            Some(&initial),
            &[],
            &[],
            FitWeight::None,
        )
        .unwrap();
    let report = &reports[0];
    let col = p.book(1).unwrap().column(&report.curve.1).unwrap();
    let OpKind::Nonlinear(recipe) = &col.op.as_ref().unwrap().kind else {
        panic!()
    };
    assert_eq!(recipe.components.len(), 2);
    for key in &recipe.components {
        assert_eq!(
            p.book(1).unwrap().nonlinear_output_owner(key),
            Some(report.curve.1.as_str())
        );
    }
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    let mut reopened = scicanvas_core::project_file::from_json(&json).unwrap();
    let before = reopened.clone();
    assert!(reopened.recalc_computed(true).is_empty());
    assert_eq!(reopened, before);
    assert_eq!(reports[0].peak_rows().len(), 3);
}
