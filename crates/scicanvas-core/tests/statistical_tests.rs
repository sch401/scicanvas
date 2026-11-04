use scicanvas_core::stats;

#[test]
fn dixon_recommended_ratios_original_indices_and_hand_calculations() {
    for (n, ratio, q, critical) in [
        (5, "r10", 7.0 / 10.0, 0.710),
        (9, "r11", 11.0 / 17.0, 0.570),
        (20, "r22", 23.0 / 38.0, 0.491),
    ] {
        let mut x: Vec<f64> = (0..n - 1).map(|v| v as f64).collect();
        x.push((n * 2) as f64);
        x.insert(0, f64::NAN);
        let result = stats::dixon_q(&x, 0.05).unwrap();
        assert_eq!(result.ratio, ratio);
        assert_eq!(result.index, n);
        assert!((result.q - q).abs() < 1e-14, "n={n}: {}", result.q);
        assert_eq!(result.critical, critical);
    }
    let low = stats::dixon_q(&[-20., 0., 1., 2., 3.], 0.05).unwrap();
    assert_eq!(low.index, 0);
    assert!((low.q - 20.0 / 23.0).abs() < 1e-14);
    // 独立抽查 CRAN outliers::qdixon 的原始表，按双侧 α 转为单侧 α/2。
    // https://github.com/cran/outliers/blob/master/R/qdixon.R
    for (n, alpha, expected) in [
        (3, 0.1, 0.941),
        (5, 0.05, 0.710),
        (7, 0.01, 0.680),
        (8, 0.05, 0.615),
        (11, 0.05, 0.625),
        (14, 0.01, 0.674),
        (20, 0.05, 0.491),
        (30, 0.01, 0.483),
    ] {
        let x: Vec<_> = (0..n).map(|i| i as f64).collect();
        assert_eq!(stats::dixon_q(&x, alpha).unwrap().critical, expected);
    }
    assert!(stats::dixon_q(&[1., 2.], 0.05).is_err());
    assert!(stats::dixon_q(&[1.; 31], 0.05).is_err());
    assert!(stats::dixon_q(&[1.; 5], 0.05).is_err());
    assert!(stats::dixon_q(&[1., 2., 3.], 0.02).is_err());
}

#[test]
fn two_way_anova_rejects_empty_cells_and_no_residual_degrees_of_freedom() {
    let strings = |v: &[&str]| v.iter().map(|v| v.to_string()).collect::<Vec<_>>();
    let error =
        stats::two_way_anova(&strings(&["a", "a", "b"]), &strings(&["x", "y", "y"]), &[1., 2., 3.]).unwrap_err();
    assert!(error.contains("空单元格"));
    assert!(stats::two_way_anova(
        &strings(&["a", "a", "b", "b"]),
        &strings(&["x", "y", "x", "y"]),
        &[1., 2., 3., 4.]
    )
    .unwrap_err()
    .contains("重复观测"));
    assert!(stats::two_way_anova(&strings(&["a"]), &strings(&["x", "y"]), &[1.]).is_err());
}

#[test]
fn balanced_two_way_type_ii_main_effects_equal_sequential_type_i() {
    let (mut a, mut b, mut y) = (vec![], vec![], vec![]);
    for i in 0..2 {
        for j in 0..3 {
            for k in 0..3 {
                a.push(i.to_string());
                b.push(j.to_string());
                y.push((i * 3 + j * 2 + i * j) as f64 + k as f64 * 0.4);
            }
        }
    }
    let result = stats::two_way_anova(&a, &b, &y).unwrap();
    let grand = y.iter().sum::<f64>() / y.len() as f64;
    for (factor, row) in [(&a, &result.rows[0]), (&b, &result.rows[1])] {
        let mut levels = factor.clone();
        levels.sort();
        levels.dedup();
        let between: f64 = levels
            .iter()
            .map(|level| {
                let values: Vec<_> = y
                    .iter()
                    .zip(factor)
                    .filter(|(_, f)| *f == level)
                    .map(|(v, _)| *v)
                    .collect();
                values.len() as f64 * (values.iter().sum::<f64>() / values.len() as f64 - grand).powi(2)
            })
            .sum();
        assert!((row.ss - between).abs() < 1e-10);
    }
}

#[test]
fn normality_and_ks_input_limits() {
    assert!(stats::lilliefors(&[1., 2., 3.]).is_err());
    assert!(stats::anderson_normal(&[1.; 4]).is_err());
    assert!(stats::lilliefors(&[1.; 4]).is_err());
    assert!(stats::ks_2samp(&[f64::NAN], &[1.]).is_err());
    let tied = stats::ks_2samp(&[1.; 8], &[1.; 9]).unwrap();
    assert_eq!((tied.d, tied.p), (0., 1.));
    assert_eq!(stats::ks_2samp(&[0.], &[1.]).unwrap().p, 1.);
}
