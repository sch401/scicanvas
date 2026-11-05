#![cfg(feature = "serde")]
use scicanvas_core::{
    nlfit::{self, FitFunction, GlobalSpec, Model},
    ops::{FitWeight, Source},
    Book, Column, Project, Role,
};

fn data() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let x: Vec<_> = (0..41).map(|i| i as f64 * 0.2).collect();
    let y = x
        .iter()
        .enumerate()
        .map(|(i, x)| 0.3 + 5. * (-x / 1.7).exp() + 0.02 * (i as f64).sin())
        .collect();
    let s = x.iter().map(|x| 0.05 + 0.01 * x).collect();
    (x, y, s)
}

#[test]
fn a3_single_group_and_all_shared_are_byte_identical_to_ordinary_fit() {
    for weighted in [false, true] {
        let (x, y, s) = data();
        let sigma = weighted.then_some(s);
        let direct = nlfit::fit_function_weighted(
            FitFunction::Builtin(Model::ExpDec1),
            &x,
            &y,
            sigma.as_deref(),
            None,
            &[],
            &[],
        )
        .unwrap();
        let global = nlfit::fit_global(
            GlobalSpec {
                base: FitFunction::Builtin(Model::ExpDec1),
                groups: vec![(x.clone(), y.clone(), sigma.clone())],
                shared: vec![false; 3],
            },
            None,
            &[],
            "",
        )
        .unwrap();
        assert_eq!(
            serde_json::to_vec(&global.per_group[0]).unwrap(),
            serde_json::to_vec(&direct).unwrap()
        );
        let merged_x: Vec<_> = x.iter().chain(&x).copied().collect();
        let merged_y: Vec<_> = y.iter().chain(&y).copied().collect();
        let merged_sigma = sigma.as_ref().map(|s| s.iter().chain(s).copied().collect::<Vec<_>>());
        let direct = nlfit::fit_function_weighted(
            FitFunction::Builtin(Model::ExpDec1),
            &merged_x,
            &merged_y,
            merged_sigma.as_deref(),
            None,
            &[],
            &[],
        )
        .unwrap();
        let global = nlfit::fit_global(
            GlobalSpec {
                base: FitFunction::Builtin(Model::ExpDec1),
                groups: vec![(x.clone(), y.clone(), sigma.clone()), (x, y, sigma)],
                shared: vec![true; 3],
            },
            None,
            &[],
            "",
        )
        .unwrap();
        assert_eq!(
            serde_json::to_vec(&global.joint).unwrap(),
            serde_json::to_vec(&direct).unwrap()
        );
    }
}

#[test]
fn a3_layout_limits_and_once_only_project_output() {
    assert!(nlfit::global_layout(&FitFunction::Builtin(Model::Gauss), 16, &[false; 4]).is_err());
    assert_eq!(
        nlfit::global_layout(&FitFunction::Builtin(Model::Gauss), 100, &[true; 4])
            .unwrap()
            .names
            .len(),
        4
    );
    let (x, y, _) = data();
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut b = Book::new(id, "global");
    for (key, role, values) in [
        ("A", Role::X, x),
        ("B", Role::Y, y.clone()),
        ("C", Role::Y, y.iter().map(|v| v * 1.4 + 0.2).collect()),
    ] {
        let mut c = Column::new(key, role);
        c.data = values.iter().map(ToString::to_string).collect();
        b.columns.push(c);
    }
    p.books.push(b);
    let report = p
        .global_fit(
            &Source::Book {
                id,
                selected: vec!["B".into(), "C".into()],
                rows: None,
            },
            FitFunction::Builtin(Model::ExpDec1),
            &[false, false, true],
            &[],
            "",
            FitWeight::None,
        )
        .unwrap();
    assert_eq!(report.fit.per_group.len(), 2);
    assert_eq!(p.books[0].columns.len(), 7);
    assert_eq!(p.notes.len(), 1);
    assert!(p.notes[0].text.contains("暂不支持自动重算"));
    assert_eq!(
        p,
        scicanvas_core::project_file::from_json(&scicanvas_core::project_file::to_json(&p).unwrap()).unwrap()
    );
}

#[test]
fn review_global_rejects_mixed_weights_before_both_solver_paths() {
    for shared in [vec![true; 3], vec![false; 3]] {
        for reverse in [false, true] {
            let (x, y, s) = data();
            let mut groups = vec![(x.clone(), y.clone(), Some(s)), (x, y, None)];
            if reverse {
                groups.reverse();
            }
            let error = nlfit::fit_global(
                GlobalSpec {
                    base: FitFunction::Builtin(Model::ExpDec1),
                    groups,
                    shared: shared.clone(),
                },
                None,
                &[],
                "",
            )
            .unwrap_err();
            assert_eq!(error, "全局拟合中各组需同时提供或同时不提供 Y 误差");
        }
    }
}
