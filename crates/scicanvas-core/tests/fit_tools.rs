#![cfg(feature = "serde")]
use scicanvas_core::{
    nlfit::{self, FitFunction, InvertError, Model},
    ops::{compare_fits, OpKind, Source},
    Project,
};
fn source() -> Source {
    Source::Book {
        id: 1,
        selected: vec!["B".into()],
        rows: None,
    }
}
fn project() -> Project {
    let mut p = Project::sample();
    let xs: Vec<_> = (0..41).map(|i| i as f64 * 0.2).collect();
    let book = p.book_mut(1).unwrap();
    book.columns.iter_mut().find(|c| c.key == "A").unwrap().data = xs.iter().map(ToString::to_string).collect();
    book.columns.iter_mut().find(|c| c.key == "B").unwrap().data = xs
        .iter()
        .enumerate()
        .map(|(i, x)| (0.3 + 5. * (-x / 1.7).exp() + 0.03 * (i as f64).sin()).to_string())
        .collect();
    p
}

#[test]
fn a2_nonlinear_residuals_recalculate_keep_rows_delete_and_legacy_read() {
    let mut p = project();
    let (_, reports) = p.nlfit(&source(), Model::ExpDec1, &[], &[]).unwrap();
    let report = &reports[0];
    let key = p.nlfit_residuals(report).unwrap();
    let original = p.book(1).unwrap().column(&key).unwrap().values();
    assert_eq!(p.nlfit_residuals(report).unwrap(), key);
    p.book_mut(1)
        .unwrap()
        .columns
        .iter_mut()
        .find(|c| c.key == "B")
        .unwrap()
        .data
        .set_text(4, "4.8".into());
    assert!(p.recalc_computed(true).is_empty());
    let after = p.book(1).unwrap().column(&key).unwrap().values();
    assert_ne!(original, after);
    let current = p.nlfit_reports().into_iter().next().unwrap();
    let qq = p.nlfit_residual_plot(&[current], true).unwrap();
    let (_, plots) = p
        .graph(qq)
        .unwrap()
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|_| None);
    let mut ordered = after.clone();
    ordered.sort_by(f64::total_cmp);
    assert_eq!(plots[0].series.points.iter().map(|p| p[1]).collect::<Vec<_>>(), ordered);
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
    p.delete_columns(1, std::slice::from_ref(&key));
    let op = p.book(1).unwrap().column(&report.curve.1).unwrap().op.as_ref().unwrap();
    assert!(matches!(&op.kind,OpKind::Nonlinear(r) if r.residual.is_none()));
    let mut legacy: serde_json::Value =
        serde_json::from_str(&scicanvas_core::project_file::to_json(&p).unwrap()).unwrap();
    legacy["version"] = 29.into();
    assert!(scicanvas_core::project_file::from_json(&legacy.to_string()).is_ok());
}

#[test]
fn a2_aic_counts_variance_and_comparison_rejects_different_data() {
    let mut p = project();
    let (_, r) = p.nlfit(&source(), Model::ExpDec1, &[], &[]).unwrap();
    let mut a = r[0].clone();
    let f = &a.fit;
    let k = (f.n - f.dof + 1) as f64;
    assert_eq!(f.aic(), f.n as f64 * (f.rss / f.n as f64).ln() + 2. * k);
    assert_eq!(f.aicc(), f.aic() + 2. * k * (k + 1.) / (f.n as f64 - k - 1.));
    assert!(compare_fits(&a, &a).unwrap().f.is_nan());
    let mut b = a.clone();
    b.y_key = "C".into();
    assert!(compare_fits(&a, &b).is_err());
    b = a.clone();
    b.fit.n += 1;
    assert!(compare_fits(&a, &b).is_err());
    b = a.clone();
    b.book_id += 1;
    assert!(compare_fits(&a, &b).is_err());
    a.fit.dof = 2;
    assert!(a.fit.aicc().is_nan());
}

#[test]
fn a2_inverse_outside_and_multiple_solutions() {
    let xs: Vec<_> = (0..61).map(|i| -5. + i as f64 / 6.).collect();
    let ys: Vec<_> = xs.iter().map(|x| Model::Gauss.eval(*x, &[0., 0., 2., 5.])).collect();
    let fit = nlfit::fit_function_weighted(
        FitFunction::Builtin(Model::Gauss),
        &xs,
        &ys,
        None,
        Some(&[0., 0., 2., 5.]),
        &[],
        &[],
    )
    .unwrap();
    assert!(matches!(fit.invert(1.),Err(InvertError::Multiple(v)) if v.len()==2));
    assert_eq!(fit.invert(10.), Err(InvertError::Outside));
    assert_eq!(fit.invert(f64::NAN), Err(InvertError::Outside));
}
