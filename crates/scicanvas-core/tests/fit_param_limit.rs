#![cfg(feature = "serde")]
use scicanvas_core::{
    nlfit::{self, CustomFitDefinition},
    project_file, Project,
};

fn definition(n: usize) -> CustomFitDefinition {
    CustomFitDefinition {
        name: format!("Sine{n}"),
        formula: (1..=n)
            .map(|i| format!("b{i}*sin({i}*x)"))
            .collect::<Vec<_>>()
            .join("+"),
        parameters: (1..=n).map(|i| format!("b{i}")).collect(),
        initial: vec![0.0; n],
        constraints: String::new(),
    }
}

#[test]
fn eight_to_twelve_parameters_roundtrip_project_and_function_files() {
    assert_eq!(nlfit::MAX_PARAMS, 12);
    let mut project = Project::default();
    for n in 8..=12 {
        let definition = definition(n);
        definition.resolve().unwrap();
        let file = project_file::fit_function_to_json(&definition).unwrap();
        assert_eq!(project_file::fit_function_from_json(&file).unwrap(), definition);
        project.save_fit_function(None, definition).unwrap();
    }
    assert_eq!(
        project_file::from_json(&project_file::to_json(&project).unwrap()).unwrap(),
        project
    );
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&project).unwrap()).unwrap(),
        project
    );
    assert_eq!(project.version, scicanvas_core::model::PROJECT_VERSION);
    let bad = definition(13);
    assert_eq!(bad.resolve().unwrap_err(), "参数数量必须在 1 到 12 之间");
    let function = serde_json::json!({"version":1,"function":bad});
    assert!(project_file::fit_function_from_json(&function.to_string())
        .unwrap_err()
        .contains("参数数量必须在 1 到 12 之间"));
    let mut wire = serde_json::to_value(&project).unwrap();
    wire["fit_functions"][0]["definition"] = serde_json::to_value(definition(13)).unwrap();
    assert!(project_file::from_json(&wire.to_string())
        .unwrap_err()
        .contains("参数数量必须在 1 到 12 之间"));
}

#[test]
fn twelve_parameter_custom_function_uses_the_existing_solver() {
    let definition = definition(12);
    let (function, constraints) = definition.resolve().unwrap();
    let xs: Vec<_> = (0..128).map(|i| i as f64 * std::f64::consts::TAU / 128.0).collect();
    let truth: Vec<_> = (1..=12).map(|i| i as f64 / 10.0).collect();
    let ys: Vec<_> = xs
        .iter()
        .map(|x| {
            truth
                .iter()
                .enumerate()
                .map(|(i, b)| b * ((i + 1) as f64 * x).sin())
                .sum()
        })
        .collect();
    let fit = nlfit::fit_custom(function, &xs, &ys, &definition.initial, &[], &constraints).unwrap();
    assert!(fit.converged);
    assert_eq!(fit.params.len(), 12);
    for (actual, expected) in fit.params.iter().zip(truth) {
        assert!((actual - expected).abs() < 1e-8);
    }
    assert!(fit.rss < 1e-14);
}
