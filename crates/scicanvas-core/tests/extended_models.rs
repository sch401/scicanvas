#![cfg(feature = "serde")]
use scicanvas_core::{
    nlfit::{self, FitFunction, Model},
    ops::{FitMethod, OpKind, Source},
    Project,
};
#[test]
fn a4_bet_domain_and_damped_origin_save_recalculate_and_reedit() {
    let x = [-1., 0., 0.05, 0.1, 0.2, 0.3, 1., 2.];
    let y: Vec<_> = x
        .iter()
        .map(|&x| {
            if x > 0. && x < 1. {
                Model::BET.eval(x, &[2., 6.])
            } else {
                99.
            }
        })
        .collect();
    let s = [0., 0., 1., 1., 1., 1., 0., 0.];
    let fit = nlfit::fit_function_weighted(FitFunction::Builtin(Model::BET), &x, &y, Some(&s), None, &[], &[]).unwrap();
    assert_eq!(fit.n, 4);
    assert!((fit.params[0] - 2.).abs() < 1e-8);
    assert!((fit.params[1] - 6.).abs() < 1e-8);
    assert!(Model::BET.eval(1., &fit.params).is_nan());
    let mut p = Project::sample();
    let x: Vec<_> = (0..201).map(|i| 5. + i as f64 * 0.05).collect();
    let y: Vec<_> = x
        .iter()
        .map(|x| 0.3 + 3. * (-(x - 5.) / 6.).exp() * (std::f64::consts::TAU * (x - 5.3) / 2.5).sin())
        .collect();
    for (key, values) in [("A", x), ("B", y)] {
        p.book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == key)
            .unwrap()
            .data = values.iter().map(ToString::to_string).collect();
    }
    let (_, r) = p
        .nlfit(
            &Source::Book {
                id: 1,
                selected: vec!["B".into()],
                rows: None,
            },
            Model::SineDamp,
            &[],
            &[],
        )
        .unwrap();
    let report = &r[0];
    assert!(matches!(report.fit.model,FitFunction::SineDamp {origin} if origin==5.));
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
    let mut recipe = match &p.nlfit_operation(report).unwrap().kind {
        OpKind::Nonlinear(r) => *r.clone(),
        _ => panic!(),
    };
    assert!(matches!(recipe.function,FitMethod::SineDamp{origin} if origin==5.));
    recipe.initial = Some(report.fit.params.clone());
    p.edit_nlfit(report, recipe).unwrap();
    assert!(p.recalc_computed(true).is_empty());
    assert!(matches!(p.nlfit_reports()[0].fit.model,FitFunction::SineDamp {origin} if origin==5.));
}
