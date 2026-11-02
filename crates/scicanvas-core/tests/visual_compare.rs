#![cfg(feature = "serde")]
use serde_json::Value;
use std::{fs, path::PathBuf};

fn compare(actual: &Value, expected: &Value, tolerance: f64, path: &str) {
    match (actual, expected) {
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: 图元/顶点数量不同");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                compare(a, b, tolerance, &format!("{path}/{i}"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{path}");
            for (key, a) in a {
                compare(a, &b[key], tolerance, &format!("{path}/{key}"));
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!(
                (a - b).abs() <= tolerance,
                "{path}: {a:.17} != {b:.17}, 误差 {}",
                (a - b).abs()
            );
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

#[test]
fn matplotlib_relative_positions_match_committed_scenes() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/parity/visual");
    let mut count = 0;
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if !path.join("scicanvas.json").is_file() {
            continue;
        }
        let read = |name| serde_json::from_slice::<Value>(&fs::read(path.join(name)).unwrap()).unwrap();
        let actual = read("scicanvas.json");
        let expected = read("matplotlib.json");
        let name = actual["case"].as_str().unwrap();
        assert_eq!(expected["versions"]["matplotlib"], "3.11.2");
        assert_eq!(
            actual["layers"].as_array().unwrap().len(),
            expected["layers"].as_array().unwrap().len()
        );
        for (i, (a, b)) in actual["layers"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected["layers"].as_array().unwrap())
            .enumerate()
        {
            compare(&a["axes"], &b["axes"], 1e-6, &format!("{name}/{i}/axes"));
            compare(&a["marks"], &b["marks"], 1e-6, &format!("{name}/{i}/marks"));
            if matches!(a["kind"].as_str(), Some("Spline" | "SplineSymbol")) {
                for (line_a, line_b) in a["marks"]["polylines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(b["marks"]["polylines"].as_array().unwrap())
                {
                    for (point_a, point_b) in line_a.as_array().unwrap().iter().zip(line_b.as_array().unwrap()) {
                        let y = point_a[1].as_f64().unwrap();
                        let reference = point_b[1].as_f64().unwrap();
                        assert!(
                            (y - reference).abs() <= 1e-9 * reference.abs().max(1e-12),
                            "{name}: 自然样条相对 Y 误差超过 1e-9"
                        );
                    }
                }
            }
        }
        for artifact in ["scicanvas.svg", "matplotlib.png", "README.md"] {
            assert!(path.join(artifact).is_file(), "{name}: 缺少 {artifact}");
        }
        count += 1;
    }
    assert_eq!(count, 27, "必须覆盖全部约定样例，禁止静默跳过目录");
}
