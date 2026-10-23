use super::*;

#[test]
fn fit_reports_add_confidence_and_prediction_bands_in_one_step() {
    for nonlinear in [false, true] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        if nonlinear {
            app.nlfit(scicanvas_core::nlfit::Model::PseudoFirstOrder, &[], &[]);
        } else {
            app.fit(1);
        }
        let before = app.project.clone();
        let columns = app.project.book(1).unwrap().columns.len();
        press_dialog_button(&mut app, "置信/预测带");
        let book = app.project.book(1).unwrap();
        assert_eq!(book.columns.len(), columns + 4, "{}", app.status);
        let names: Vec<&str> = book.columns[columns..].iter().map(|c| c.long_name.as_str()).collect();
        assert!(names.iter().any(|n| n.ends_with("95% 置信下限")) && names.iter().any(|n| n.ends_with("95% 预测上限")));
        // 结果对话框保留
        assert!(app.dialog.is_some());
        roundtrip(&mut app, &before);
    }
}

#[test]
fn nlfit_dialog_linear_constraint_holds_and_bad_text_keeps_the_dialog() {
    use scicanvas_core::nlfit::Model;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.run_command("fit.nonlinear");
    let Some(Dialog::NlFit { model, constraints, .. }) = app.dialog.as_mut() else {
        panic!("nlfit dialog not open");
    };
    *model = Model::ExpDec1;
    *constraints = "y0 + A1 = q".into();
    press_dialog_button(&mut app, "拟合");
    assert!(matches!(app.dialog, Some(Dialog::NlFit { .. })), "dialog kept");
    assert!(app.status.contains("不是参数名"), "{}", app.status);
    if let Some(Dialog::NlFit { constraints, .. }) = app.dialog.as_mut() {
        *constraints = "y0 + A1 = 0".into();
    }
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("no report: {}", app.status);
    };
    let f = &reports[0].fit;
    assert!((f.params[0] + f.params[1]).abs() < 1e-9, "{:?}", f.params);
    assert_eq!(f.constraints, ["y0 + A1 = 0"]);
}

#[test]
fn nlfit_dialog_upper_bound_stops_the_parameter_and_marks_it() {
    use scicanvas_core::nlfit::Model;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.run_command("fit.nonlinear");
    let Some(Dialog::NlFit { model, limits, .. }) = app.dialog.as_mut() else {
        panic!("nlfit dialog not open");
    };
    // C 列是 8(1 − e^{−t/4}) 加噪声；ExpDec1 的 t1 约为 4，限制 t1 ≤ 3
    *model = Model::ExpDec1;
    limits[2].4 = true;
    limits[2].5 = 3.0;
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("no report: {}", app.status);
    };
    let f = &reports[0].fit;
    assert_eq!(f.params[2], 3.0);
    assert_eq!(f.at_bound, vec![false, false, true]);
}

#[test]
fn category_order_dialog_reorders_levels_in_one_step() {
    let mut app = app();
    let b = app.project.book_mut(1).unwrap();
    b.columns[2].data = ["中", "低", "高"].iter().map(|s| s.to_string()).collect();
    b.convert_columns(&["C".into()], scicanvas_core::DataKind::Category)
        .unwrap();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.run_command("col.categories");
    let Some(Dialog::CategoryOrder { levels, .. }) = app.dialog.as_mut() else {
        panic!("dialog not open: {}", app.status);
    };
    *levels = vec!["低".into(), "中".into(), "高".into()];
    let before = app.project.clone();
    press_dialog_button(&mut app, "×");
    assert!(app.dialog.is_none(), "{}", app.status);
    let data = &app.project.book(1).unwrap().columns[2].data;
    assert_eq!(data.category_levels().unwrap(), ["低", "中", "高"]);
    assert_eq!(data.text_at(0), "中");
    roundtrip(&mut app, &before);
    // 非分类列给出提示
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("col.categories");
    assert!(app.status.contains("不是分类列"), "{}", app.status);
}

#[test]
fn custom_sort_dialog_sorts_by_two_keys_or_only_selected_columns() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("sort.custom");
    let Some(Dialog::Sort {
        keys, only_selected, ..
    }) = app.dialog.as_mut()
    else {
        panic!("sort dialog not open");
    };
    // 主关键字默认为选中的 B 列；改为降序、只排 B 列
    assert_eq!(keys[0], (Some(1), true));
    keys[0].1 = false;
    *only_selected = true;
    let before = app.project.clone();
    let a_before = app.project.book(1).unwrap().columns[0].values();
    press_dialog_button(&mut app, "排序");
    assert!(app.dialog.is_none(), "{}", app.status);
    let book = app.project.book(1).unwrap();
    let b = book.columns[1].values();
    assert!(b.windows(2).all(|w| w[0] >= w[1]));
    assert_eq!(book.columns[0].values(), a_before);
    roundtrip(&mut app, &before);
}

#[test]
fn ifft_dialog_finds_fft_columns_and_creates_a_signal_table() {
    let mut app = app();
    let src = scicanvas_core::ops::Source::Book {
        id: 1,
        selected: vec!["B".into()],
        rows: None,
    };
    let (spec, _) = app.project.fft(&src, scicanvas_core::fft::Window::Rect).unwrap()[0];
    app.active = Some(spec);
    app.run_command("signal.ifft");
    let Some(Dialog::Ifft { re, im, rows, .. }) = &app.dialog else {
        panic!("IFFT dialog not open: {}", app.status);
    };
    assert!(re.1.contains("实部") && im.as_ref().unwrap().1.contains("虚部"));
    let m = *rows;
    let before = app.project.clone();
    press_dialog_button(&mut app, "变换");
    let out = app.project.books.last().unwrap();
    assert!(out.name.contains("IFFT"), "{}", app.status);
    assert_eq!(out.columns[1].values().len(), 2 * (m - 1));
    roundtrip(&mut app, &before);
}

#[test]
fn sg_derivative_dialog_writes_second_derivative_column_and_undoes() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("math.diff_sg");
    if let Some(Dialog::SgDiff { deriv, .. }) = app.dialog.as_mut() {
        *deriv = 2;
    } else {
        panic!("SG dialog not open");
    }
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    let pos = crate::test_ui::find_text(&output, "求导")
        .map(|pos| pos + egui::vec2(5.0, 5.0))
        .expect("OK button");
    harness.frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        harness.frame(
            &mut app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    assert!(app.dialog.is_none());
    let book = app.project.book(1).unwrap();
    let col = book
        .columns
        .iter()
        .find(|c| c.long_name.starts_with("d²("))
        .expect("derivative column");
    assert!(col.comment.contains("Savitzky–Golay，7 点 2 阶"), "{}", col.comment);
    roundtrip(&mut app, &before);
}

#[test]
fn fit_report_residual_plot_button_creates_scatter_and_undoes() {
    let mut app = app();
    app.active = Some(1);
    app.fit(1);
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    let pos = crate::test_ui::find_text(&output, "残差图")
        .map(|pos| pos + egui::vec2(5.0, 5.0))
        .expect("residual plot button");
    harness.frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        harness.frame(
            &mut app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    let graph = app.project.graphs.last().unwrap();
    assert_eq!(app.active, Some(graph.id));
    assert_eq!(graph.first_layer().plot_type, PlotType::Scatter);
    let book = app.project.book(1).unwrap();
    for key in &graph.first_layer().y_keys {
        assert!(book.column(key).unwrap().long_name.ends_with("残差"));
    }
    // 对话框保留，可以继续保存或复制
    assert!(app.dialog.is_some());
    roundtrip(&mut app, &before);
}

#[test]
fn fit_report_save_note_button_persists_report_and_undoes() {
    for nonlinear in [false, true] {
        let mut app = app();
        app.active = Some(1);
        if nonlinear {
            app.nlfit(scicanvas_core::nlfit::Model::ALL[0], &[], &[]);
        } else {
            app.fit(1);
        }
        assert!(app.dialog.is_some(), "nonlinear={nonlinear}: {}", app.status);
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(1400.0, 1000.0)),
            |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
        )
        .with_step(0.02);
        harness.frame(&mut app, vec![]);
        harness.frame(&mut app, vec![]);
        let output = harness.frame(&mut app, vec![]);
        let pos = crate::test_ui::find_text(&output, "保存为 Note")
            .map(|pos| pos + egui::vec2(5.0, 5.0))
            .expect("save report button");
        harness.frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            harness.frame(
                &mut app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        let note = app.project.notes.last().expect("saved Note");
        assert!(note.text.contains("R"));
        // 保存为 Markdown：标题、参数表和统计量表
        assert!(note.text.starts_with("## "), "{}", note.text);
        assert!(
            note.text
                .contains("| 参数 | 值 | 标准误 | t | p |\n| --- | --- | --- | --- | --- |"),
            "{}",
            note.text
        );
        assert!(note.text.contains("| 统计量 | 值 |"), "{}", note.text);
        assert!(!note.text.contains('\t'));
        assert_eq!(app.active, Some(note.id));
        assert!(app.dialog.is_none());
        let saved =
            scicanvas_core::project_file::from_scv(&scicanvas_core::project_file::to_scv(&app.project).unwrap())
                .unwrap();
        assert_eq!(saved.notes, app.project.notes);
        roundtrip(&mut app, &before);
    }
}

#[test]
fn nonlinear_fit_opens_a_report_and_undoes_as_one_step() {
    use scicanvas_core::nlfit::Model;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    let before = app.project.clone();
    app.open_nlfit_dialog();
    assert!(matches!(
        app.dialog,
        Some(Dialog::NlFit {
            model: Model::Gauss,
            ..
        })
    ));
    app.finish_edit();
    app.begin_edit("非线性拟合");
    app.dialog = None;
    app.nlfit(Model::ExpDec1, &[], &[]);
    app.history.changed();
    app.finish_edit();
    match &app.dialog {
        Some(Dialog::NlFitReport(reports)) => {
            assert_eq!(reports.len(), 1);
            assert!(reports[0].fit.converged);
        }
        _ => panic!("report should open"),
    }
    assert_eq!(app.history.undo_label(), Some("非线性拟合"));
    roundtrip(&mut app, &before);
}

#[test]
fn column_math_dialog_follows_the_picked_columns_and_undoes() {
    use scicanvas_core::ops::ColumnOp;
    let mut app = app();
    app.active = Some(1);
    let before = app.project.clone();
    app.selection.insert(1, vec!["C".into(), "B".into()]);
    app.open_column_math_dialog();
    assert!(matches!(
        &app.dialog,
        Some(Dialog::ColumnMath { a, b: Some(b), use_constant: false, .. }) if a == "电压 B" && b == "电压 A"
    ));
    app.finish_edit();
    app.begin_edit("列运算");
    app.column_math(ColumnOp::Sub, None);
    app.history.changed();
    app.finish_edit();
    let book = app.project.book(1).unwrap();
    let result = book.columns.iter().find(|c| c.long_name == "电压 B − 电压 A").unwrap();
    let want = book.column("C").unwrap().values()[3] - book.column("B").unwrap().values()[3];
    assert!((result.values()[3] - want).abs() < 1e-9);
    roundtrip(&mut app, &before);

    app.selection.insert(1, vec!["B".into()]);
    app.open_column_math_dialog();
    assert!(matches!(
        app.dialog,
        Some(Dialog::ColumnMath {
            b: None,
            use_constant: true,
            ..
        })
    ));
}

#[test]
fn frequency_dialog_defaults_to_the_picked_columns_value_range() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into()]);
    app.open_frequency_dialog();
    assert!(matches!(
        app.dialog,
        Some(Dialog::Frequency { auto: true, custom: false, from, to, .. }) if from == 0.0 && to == 20.0
    ));
    let before = app.project.clone();
    app.frequency_counts(Some(4), None);
    assert_eq!(app.project.books.len(), before.books.len() + 1);
    assert_eq!(
        app.active,
        app.project.books.last().map(|b| b.id),
        "the new table is shown"
    );
}

#[test]
fn baseline_subtraction_runs_on_picked_columns_in_one_undo_step() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let cols = app.project.book(1).unwrap().columns.len();
    app.begin_edit("扣除基线");
    app.subtract_baseline(scicanvas_core::baseline::BaselineMethod::Line);
    app.history.changed();
    app.finish_edit();
    let book = app.project.book(1).unwrap();
    assert_eq!(book.columns.len(), cols + 2);
    assert_eq!(book.columns[3].long_name, "电压 A 扣除基线");
    assert!(app.status.contains("两端连线"), "{}", app.status);
    app.undo();
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols);
}

#[test]
fn peaks_open_a_table_per_curve_in_one_undo_step() {
    let mut app = app();
    app.active = Some(1);
    let book = app.project.book_mut(1).unwrap();
    book.columns[0].data = (0..61).map(|i| scicanvas_core::model::fmt_num(i as f64)).collect();
    book.columns[1].data = (0..61)
        .map(|i| {
            scicanvas_core::model::fmt_num(
                3.0 * (-((i as f64 - 20.0) / 3.0).powi(2)).exp() + (-((i as f64 - 45.0) / 4.0).powi(2)).exp(),
            )
        })
        .collect();
    app.selection.insert(1, vec!["B".into()]);
    let books = app.project.books.len();
    app.begin_edit("寻峰");
    app.find_peaks(0.05);
    app.history.changed();
    app.finish_edit();
    assert_eq!(app.project.books.len(), books + 1);
    let table = app.project.books.last().unwrap();
    assert_eq!(table.columns[0].values(), [20.0, 45.0]);
    assert!(app.status.contains("2 个峰"), "{}", app.status);
    app.undo();
    assert_eq!(app.project.books.len(), books);
}

#[test]
fn correlation_fill_and_average_create_results() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    let (books, graphs) = (app.project.books.len(), app.project.graphs.len());
    app.correlation_function();
    assert_eq!(
        (app.project.books.len(), app.project.graphs.len()),
        (books + 1, graphs + 1),
        "{}",
        app.status
    );
    app.active = Some(1);
    app.average_curves();
    assert_eq!(app.project.books.len(), books + 2, "{}", app.status);

    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.open_fill_dialog();
    assert!(matches!(&app.dialog, Some(Dialog::Fill { target, .. }) if target.starts_with('C')));
    app.dialog = None;
    app.fill_sequence(scicanvas_core::stats::Sequence::Arithmetic { start: 0.0, step: 0.5 }, 3);
    assert_eq!(
        app.project.book(1).unwrap().column("C").unwrap().data,
        ["0", "0.5", "1"]
    );
}

#[test]
fn regression_uses_the_last_picked_column_as_y() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "C".into(), "B".into()]);
    app.regression();
    let book = app.project.book(1).unwrap();
    let want = scicanvas_core::stats::multiple_regression(
        &[book.column("A").unwrap().values(), book.column("C").unwrap().values()],
        &book.column("B").unwrap().values(),
    );
    match (&app.dialog, want) {
        (Some(Dialog::Table { rows, note, .. }), Ok(r)) => {
            assert_eq!(rows.len(), 4);
            assert_eq!(rows[2][0], book.column("A").unwrap().display_name());
            assert_eq!(rows[1][1], fmt_num(round_sig(r.coef[0], 8)));
            assert!(note.contains(book.column("B").unwrap().display_name()));
        }
        (d, w) => panic!("unexpected {:?} / {:?}", d.is_some(), w),
    }
}

#[test]
fn row_stats_and_fft_filter_add_columns_in_one_undo_step() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    let cols = app.project.book(1).unwrap().columns.len();
    app.begin_edit("行统计");
    app.row_stats();
    app.history.changed();
    app.finish_edit();
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 4, "{}", app.status);
    app.undo();
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols);

    app.selection.insert(1, vec!["B".into()]);
    app.open_fft_filter_dialog();
    let Some(Dialog::FftFilter { lo, nyquist, .. }) = &app.dialog else {
        panic!("filter dialog should open")
    };
    let (lo, nyquist) = (*lo, *nyquist);
    assert!(lo > 0.0 && lo < nyquist);
    app.dialog = None;
    app.fft_filter(scicanvas_core::fft::FilterKind::LowPass(lo));
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 1, "{}", app.status);
}

#[test]
fn normality_and_grubbs_report_one_row_per_picked_column() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    app.normality();
    match &app.dialog {
        Some(Dialog::Table { rows, .. }) => {
            assert_eq!(rows.len(), 3);
            let w = scicanvas_core::stats::shapiro_wilk(&app.project.book(1).unwrap().column("B").unwrap().values())
                .unwrap();
            assert_eq!(rows[1][2], fmt_num(round_sig(w.w, 6)));
        }
        _ => panic!("normality report should open"),
    }
    app.dialog = None;
    app.grubbs();
    assert!(matches!(&app.dialog, Some(Dialog::Table { rows, .. }) if rows.len() == 3 && rows[0].len() == 8));
    app.dialog = None;
    app.selection.insert(1, vec![]);
    app.normality();
    assert!(app.dialog.is_none() && app.status.contains("选中"));
}

#[test]
fn calibrate_needs_three_columns_in_click_order() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into()]);
    app.calibrate();
    assert!(app.status.contains("三列"));
    let cols = app.project.book(1).unwrap().columns.len();
    app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
    app.begin_edit("标准曲线反查");
    app.calibrate();
    app.history.changed();
    app.finish_edit();
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 2, "{}", app.status);
    assert!(app.status.contains("R²"));
    app.undo();
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols);
}

#[test]
fn anova_treats_each_picked_column_as_a_group() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    app.anova();
    let book = app.project.book(1).unwrap();
    let want =
        scicanvas_core::stats::one_way_anova(&[book.column("B").unwrap().values(), book.column("C").unwrap().values()])
            .unwrap();
    match &app.dialog {
        Some(Dialog::Anova { result, names, tukey }) => {
            assert_eq!(result, &want);
            assert_eq!(tukey, &scicanvas_core::stats::tukey_hsd(&want, 0.95));
            assert_eq!(names, &["电压 A", "电压 B"]);
        }
        _ => panic!("report should open"),
    }
    app.dialog = None;
    app.selection.insert(1, vec!["B".into()]);
    app.anova();
    assert!(app.dialog.is_none() && app.status.contains("至少两列"));
}

#[test]
fn correlations_cover_every_pair_of_picked_columns() {
    use scicanvas_core::stats::{correlation, CorrMethod};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
    app.correlations();
    let Some(Dialog::Correlations(pairs)) = &app.dialog else {
        panic!("report should open")
    };
    let names: Vec<_> = pairs.iter().map(|(a, b, _)| (a.as_str(), b.as_str())).collect();
    assert_eq!(names, [("时间", "电压 A"), ("时间", "电压 B"), ("电压 A", "电压 B")]);
    let book = app.project.book(1).unwrap();
    let want = correlation(
        &book.column("A").unwrap().values(),
        &book.column("C").unwrap().values(),
        CorrMethod::Spearman,
    )
    .unwrap();
    assert_eq!(pairs[1].2[1], want);
    app.dialog = None;
    app.selection.insert(1, vec!["A".into()]);
    app.correlations();
    assert!(app.dialog.is_none() && app.status.contains("至少两列"));
}

#[test]
fn ttest_uses_the_picked_columns_in_click_order_and_leaves_data_alone() {
    use scicanvas_core::stats::{t_test, TTestKind};
    let mut app = app();
    app.active = Some(1);
    let before = app.project.clone();
    app.selection.insert(1, vec!["C".into(), "B".into()]);
    app.open_ttest_dialog();
    match &app.dialog {
        Some(Dialog::TTest { columns, kind, .. }) => {
            assert_eq!(columns, &["电压 B", "电压 A"]);
            assert_eq!(*kind, 1, "two picked columns default to a two-sample test");
        }
        _ => panic!("t-test dialog should open"),
    }
    let kind = TTestKind::TwoSample { equal_var: false };
    app.run_ttest(kind, 0.95);
    let book = app.project.book(1).unwrap();
    let col = |k: &str| book.column(k).unwrap().values();
    let want = t_test(kind, &col("C"), &col("B"), 0.95).unwrap();
    match &app.dialog {
        Some(Dialog::TTestReport { result, names }) => {
            assert_eq!(result, &want);
            assert_eq!(names, &["电压 B", "电压 A"]);
        }
        _ => panic!("report should open"),
    }
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());

    app.selection.insert(1, vec!["B".into()]);
    app.open_ttest_dialog();
    assert!(matches!(app.dialog, Some(Dialog::TTest { kind: 0, .. })));
}
