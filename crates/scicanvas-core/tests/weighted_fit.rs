#![cfg(feature = "serde")]
//! 加权非线性拟合：误差列来源、改误差后重算、v24 保存。
use scicanvas_core::{
    nlfit::{self, FitFunction, Model},
    ops::{FitWeight, OpKind, Source},
    project_file, Project, Role,
};

fn source() -> Source {
    Source::Book {
        id: 1,
        selected: vec!["B".into()],
        rows: None,
    }
}

/// 示例工作簿：A 为 X，B 为指数衰减加小扰动，C 改为 B 的 Y 误差列。
fn project(sigma: impl Fn(f64) -> f64) -> Project {
    let mut p = Project::sample();
    let book = p.book_mut(1).unwrap();
    let xs: Vec<f64> = (0..41).map(|i| i as f64 * 0.2).collect();
    book.columns[0].data = xs.iter().map(ToString::to_string).collect();
    book.columns[1].data = xs
        .iter()
        .enumerate()
        .map(|(i, x)| (0.3 + 5.0 * (-x / 1.7).exp() + 0.05 * (1.7 * i as f64).sin()).to_string())
        .collect();
    book.columns[2].role = Role::YError;
    book.columns[2].data = xs.iter().map(|x| sigma(*x).to_string()).collect();
    p
}

fn direct_fit(p: &Project) -> nlfit::NlFit {
    let book = p.book(1).unwrap();
    let sigma = book.columns[2].values();
    nlfit::fit_function_weighted(
        FitFunction::Builtin(Model::ExpDec1),
        &book.columns[0].values(),
        &book.columns[1].values(),
        Some(&sigma),
        None,
        &[],
        &[],
    )
    .unwrap()
}

#[test]
fn weighted_fit_uses_the_linked_error_column_and_follows_error_edits() {
    let mut p = project(|x| 0.02 + 0.05 * x);
    let (_, reports) = p
        .nlfit_weighted(
            &source(),
            FitFunction::Builtin(Model::ExpDec1),
            None,
            &[],
            &[],
            FitWeight::YError,
        )
        .unwrap();
    assert!(reports[0].fit.weighted);
    assert_eq!(reports[0].fit.params, direct_fit(&p).params);
    let key = reports[0].curve.1.clone();
    // 改误差列后自动重算，结果与直接按新误差拟合一致。
    let xs = p.book(1).unwrap().columns[0].values();
    p.book_mut(1).unwrap().columns[2].data = xs.iter().map(|x| (0.2 - 0.02 * x).to_string()).collect();
    assert!(p.recalc_computed(false).is_empty());
    let report = p.book(1).unwrap().column(&key).unwrap().fit_report.as_deref().unwrap();
    assert!(report.current && report.fit.weighted);
    assert_eq!(report.fit.params, direct_fit(&p).params);
}

#[test]
fn weighted_fit_without_an_error_column_reports_why() {
    let mut p = project(|_| 0.1);
    p.book_mut(1).unwrap().columns[2].role = Role::Y;
    let error = p
        .nlfit_weighted(
            &source(),
            FitFunction::Builtin(Model::ExpDec1),
            None,
            &[],
            &[],
            FitWeight::YError,
        )
        .unwrap_err();
    assert!(error.contains("没有关联的 Y 误差列"), "{error}");
}

#[test]
fn weight_is_saved_in_v24_and_omitted_when_unweighted() {
    let mut p = project(|x| 0.02 + 0.05 * x);
    let (_, reports) = p
        .nlfit_weighted(
            &source(),
            FitFunction::Builtin(Model::ExpDec1),
            None,
            &[],
            &[],
            FitWeight::YError,
        )
        .unwrap();
    let key = reports[0].curve.1.clone();
    let json = project_file::to_json(&p).unwrap();
    assert!(json.contains("YError"));
    let loaded = project_file::from_scv(&project_file::to_scv(&p).unwrap()).unwrap();
    assert_eq!(loaded, p);
    let column = loaded.book(1).unwrap().column(&key).unwrap();
    let Some(OpKind::Nonlinear(recipe)) = column.op.as_ref().map(|op| &op.kind) else {
        panic!("拟合结果列应保留非线性拟合定义");
    };
    assert_eq!(recipe.weight, FitWeight::YError);

    // 不加权的结果不写 weight；没有该字段的 v23 文件按不加权读取。
    let mut plain = project(|x| 0.02 + 0.05 * x);
    let (_, reports) = plain.nlfit(&source(), Model::ExpDec1, &[], &[]).unwrap();
    let json = project_file::to_json(&plain).unwrap();
    assert!(!json.contains("\"weight\""));
    let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
    old["version"] = 23.into();
    let loaded = project_file::from_json(&old.to_string()).unwrap();
    assert_eq!(loaded.version, scicanvas_core::model::PROJECT_VERSION);
    let column = loaded.book(1).unwrap().column(&reports[0].curve.1).unwrap();
    let Some(OpKind::Nonlinear(recipe)) = column.op.as_ref().map(|op| &op.kind) else {
        panic!("拟合结果列应保留非线性拟合定义");
    };
    assert_eq!(recipe.weight, FitWeight::None);
    assert!(!column.fit_report.as_deref().unwrap().fit.weighted);
}
