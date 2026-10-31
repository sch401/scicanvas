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
    for key in &graph.first_layer().draft().y_keys {
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
    app.dialog = None;
    app.nlfit(Model::ExpDec1, &[], &[]);
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
fn custom_fit_dialog_keeps_invalid_input_and_creates_an_undoable_report() {
    let mut app = app();
    let book = app.project.book_mut(1).unwrap();
    let xs: Vec<f64> = (0..40).map(|row| row as f64 * 0.25).collect();
    book.columns[0].data = xs.iter().map(|x| fmt_num(*x)).collect();
    book.columns[1].data = xs.iter().map(|x| fmt_num(2.2 * (-0.4 * x).exp() + 0.3)).collect();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.open_nlfit_dialog();
    press_dialog_button(&mut app, "自定义函数…");
    assert!(matches!(app.dialog, Some(Dialog::CustomNlFit(..))));
    let ctx = egui::Context::default();

    let invalid = dialogs::Action::CustomNlFit {
        name: "Decay".into(),
        formula: "a*x+missing".into(),
        parameters: "a".into(),
        initial: "1".into(),
        constraints: String::new(),
    };
    assert!(!app.apply_dialog_action(&ctx, invalid));
    assert!(matches!(app.dialog, Some(Dialog::CustomNlFit(..))));

    let before = app.project.clone();
    let valid = dialogs::Action::CustomNlFit {
        name: "Decay".into(),
        formula: "y = a*exp(-b*x)+c".into(),
        parameters: "a, b, c".into(),
        initial: "2, 0.3, 0.2".into(),
        constraints: String::new(),
    };
    assert!(app.apply_dialog_action(&ctx, valid));
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("custom fit report should open: {}", app.status);
    };
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].fit.model.name(), "Decay");
    assert!(reports[0].fit.converged);
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
    app.column_math(ColumnOp::Sub, None);

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
    app.subtract_baseline(scicanvas_core::baseline::BaselineMethod::Line);

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
    app.find_peaks(0.05);

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

#[test]
fn migrated_analysis_dialogs_match_core_results_and_undo_as_one_step() {
    use scicanvas_core::{
        baseline::BaselineMethod,
        fft::{FilterKind, Window},
        interpolate::InterpMethod,
        ops::{ColumnOp, Operand},
    };
    for case in 0..10 {
        let mut app = app();
        let src = Source::Graph(2);
        if case == 9 {
            app.active = Some(1);
            app.selection.insert(1, vec!["B".into()]);
        }
        let before = app.project.clone();
        let mut expected = before.clone();
        let (dialog, button, label) = match case {
            0 => {
                expected.smooth(&src, SmoothMethod::MovingAverage, 3).unwrap();
                (
                    Dialog::Smooth {
                        method: SmoothMethod::MovingAverage,
                        window: 3,
                    },
                    "平滑",
                    "平滑",
                )
            }
            1 => {
                expected.normalize(&src, NormalizeMethod::MinMax).unwrap();
                (
                    Dialog::Normalize {
                        method: NormalizeMethod::MinMax,
                    },
                    "归一化",
                    "归一化",
                )
            }
            2 => {
                expected.subtract_baseline(&src, BaselineMethod::Line).unwrap();
                (
                    Dialog::Baseline {
                        als: false,
                        log_lambda: 5.0,
                        p: 0.01,
                    },
                    "扣除",
                    "扣除基线",
                )
            }
            3 => {
                expected.interpolate(&src, InterpMethod::Linear, 10, None).unwrap();
                (
                    Dialog::Interpolate {
                        method: InterpMethod::Linear,
                        points: 10,
                        custom: false,
                        from: 0.0,
                        to: 20.0,
                    },
                    "插值",
                    "插值",
                )
            }
            4 => {
                expected.differentiate_sg(&src, 7, 2, 1).unwrap();
                (
                    Dialog::SgDiff {
                        window: 7,
                        order: 2,
                        deriv: 1,
                    },
                    "求导",
                    "SG 平滑求导",
                )
            }
            5 => {
                expected.fft_filter(&src, FilterKind::LowPass(0.2)).unwrap();
                (
                    Dialog::FftFilter {
                        kind: 0,
                        lo: 0.2,
                        hi: 0.4,
                        nyquist: 1.0,
                    },
                    "滤波",
                    "FFT 滤波",
                )
            }
            6 => {
                expected.fft(&src, Window::Rect).unwrap();
                (Dialog::Fft { window: Window::Rect }, "变换", "FFT")
            }
            7 => {
                expected.frequency_counts(&src, Some(4), None).unwrap();
                (
                    Dialog::Frequency {
                        auto: false,
                        bins: 4,
                        custom: false,
                        from: 0.0,
                        to: 1.0,
                    },
                    "统计",
                    "频数统计",
                )
            }
            8 => {
                expected.find_peaks(&src, 0.05).unwrap();
                (Dialog::Peaks { percent: 5.0 }, "寻峰", "寻峰")
            }
            _ => {
                expected
                    .column_math(1, "B", ColumnOp::Add, &Operand::Constant(1.5))
                    .unwrap();
                (
                    Dialog::ColumnMath {
                        a: "电压 A".into(),
                        b: None,
                        use_constant: true,
                        op: ColumnOp::Add,
                        constant: 1.5,
                    },
                    "计算",
                    "列运算",
                )
            }
        };
        expected.recalc_computed(false);
        app.dialog = Some(dialog);
        press_dialog_button(&mut app, button);
        assert_eq!(app.project, expected, "case {case}: {}", app.status);
        assert!(app.dialog.is_none());
        assert_eq!(app.history.undo_label(), Some(label));
        assert!(!app.history.is_pending());
        let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
        assert_eq!(scicanvas_core::project_file::from_scv(&saved).unwrap(), app.project);
        roundtrip(&mut app, &before);
        app.undo();
        assert!(app.history.undo_label().is_none());
    }
}

#[test]
fn report_band_failure_rolls_back_earlier_results_and_preserves_redo() {
    for nonlinear in [false, true] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        if nonlinear {
            app.nlfit(scicanvas_core::nlfit::Model::PseudoFirstOrder, &[], &[]);
        } else {
            app.fit(1);
        }
        let reports = app.dialog.take().unwrap();
        app.run_command("book.new");
        app.undo();
        let before = app.project.clone();
        let active = app.active;
        let open = app.open.clone();
        let redo = app.history.redo_label().unwrap().to_string();
        let undo = app.history.undo_label().unwrap().to_string();
        let mut trial = before.clone();
        app.dialog = Some(match reports {
            Dialog::FitReport(mut reports) => {
                let good = reports[0].clone();
                let mut bad = good.clone();
                bad.book_id = 999999;
                assert!(trial.poly_bands(&good, 0.95).is_ok());
                assert!(trial.poly_bands(&bad, 0.95).is_err());
                reports = vec![good, bad];
                Dialog::FitReport(reports)
            }
            Dialog::NlFitReport(mut reports) => {
                let good = reports[0].clone();
                let mut bad = good.clone();
                bad.book_id = 999999;
                assert!(trial.nlfit_bands(&good, 0.95).is_ok());
                assert!(trial.nlfit_bands(&bad, 0.95).is_err());
                reports = vec![good, bad];
                Dialog::NlFitReport(reports)
            }
            _ => panic!("report"),
        });
        assert_ne!(
            trial, before,
            "the old collection path can write before the later error"
        );
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut harness = crate::test_ui::Harness::new(
            ctx,
            Some(egui::vec2(1800.0, 1600.0)),
            |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
        )
        .with_step(0.02);
        for _ in 0..10 {
            harness.frame(&mut app, vec![]);
        }
        let output = harness.frame(&mut app, vec![]);
        let at = crate::test_ui::text_rect(&output, "置信/预测带").unwrap().center();
        harness.click(&mut app, at);
        assert_eq!(app.project, before);
        assert_eq!(app.active, active);
        assert_eq!(app.open, open);
        assert!(app.dialog.is_some());
        assert_eq!(app.status, "找不到工作簿", "nonlinear={nonlinear}");
        assert_eq!(app.history.redo_label(), Some(redo.as_str()));
        assert_eq!(app.history.undo_label(), Some(undo.as_str()));
        assert!(!app.history.is_pending());
    }
}

#[test]
fn dialog_action_generation_leaves_project_unchanged_until_application() {
    let mut app = app();
    app.open_layer_contents();
    let Some(Dialog::LayerContents(contents)) = &mut app.dialog else {
        panic!("layer dialog");
    };
    contents.draft.title = "generated action".into();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.generate_dialog_action(ctx),
    );
    let frame = harness.frame_value(&mut app, vec![]).unwrap();
    assert!(matches!(&frame.action, dialogs::Action::LayerContents(_)));
    assert!(!frame.closing);
    assert_eq!(app.project, before);
    assert!(!app.history.is_pending());
    assert!(app.history.undo_label().is_none());
    assert!(app.apply_dialog_action(&ctx, frame.action));
    app.finish_dialog_frame(&ctx, frame.closing, frame.layer_before);
    assert_eq!(app.project.graph(2).unwrap().first_layer().title, "generated action");
    assert_eq!(app.history.undo_label(), Some("图层内容"));
    roundtrip(&mut app, &before);
    app.undo();
    assert!(app.history.undo_label().is_none());
}

#[test]
fn close_generates_live_action_before_clearing_dialog_and_preview() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.open_column_props();
    let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
        panic!("column dialog");
    };
    edit.long_name = "last valid setting".into();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.generate_dialog_action(ctx),
    );
    let frame = harness
        .frame_value(
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
        .unwrap();
    assert!(frame.closing);
    assert!(matches!(&frame.action, dialogs::Action::ColumnProps(1, _)));
    assert_eq!(app.project, before);
    assert!(app.book_preview.is_some());
    assert!(app.dialog.is_some());
    assert!(app.apply_dialog_action(&ctx, frame.action));
    app.finish_dialog_frame(&ctx, frame.closing, frame.layer_before);
    assert_eq!(
        app.project.book(1).unwrap().column("B").unwrap().long_name,
        "last valid setting"
    );
    assert!(app.dialog.is_none() && app.book_preview.is_none() && app.history_group.is_none());
    assert!(!app.history.is_pending());
    let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
    assert_eq!(scicanvas_core::project_file::from_scv(&saved).unwrap(), app.project);
    roundtrip(&mut app, &before);
    app.undo();
    assert!(app.history.undo_label().is_none());
}

#[test]
fn invalid_nlfit_action_preserves_input_and_signals_early_return() {
    use scicanvas_core::nlfit::Model;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.open_nlfit_dialog();
    let Some(Dialog::NlFit { model, constraints, .. }) = &mut app.dialog else {
        panic!("nlfit dialog");
    };
    *model = Model::ExpDec1;
    *constraints = "y0 + A1 = q".into();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    assert!(!app.apply_dialog_action(
        &ctx,
        dialogs::Action::NlFit(Model::ExpDec1, vec![], "y0 + A1 = q".into())
    ));
    assert!(matches!(&app.dialog,Some(Dialog::NlFit {constraints,..}) if constraints=="y0 + A1 = q"));
    assert!(app.status.contains("不是参数名"));
    assert_eq!(app.project, before);
    assert!(!app.history.is_pending());
    assert!(app.history.undo_label().is_none());
}

#[test]
fn custom_fit_library_real_controls_save_rename_reuse_and_reedit_with_history() {
    fn rename<F: FnMut(&mut SciApp, &egui::Context)>(
        harness: &mut crate::test_ui::Harness<F>,
        app: &mut SciApp,
        old: &str,
        new: &str,
    ) {
        let output = harness.frame(app, vec![]);
        let point = crate::test_ui::text_positions(&output, old)
            .into_iter()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap_or_else(|| {
                let visible: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                        _ => None,
                    })
                    .collect();
                panic!("missing name {old}; visible text: {visible:?}");
            });
        harness.click(app, point + egui::vec2(5.0, 5.0));
        harness.modifiers = egui::Modifiers::COMMAND;
        harness.frame(
            app,
            vec![egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
        );
        harness.modifiers = egui::Modifiers::NONE;
        harness.frame(app, vec![egui::Event::Text(new.into())]);
    }
    let mut app = app();
    let xs: Vec<f64> = (0..40).map(|i| i as f64 * 0.25).collect();
    app.project.books[0].columns[0].data = xs.iter().map(|x| fmt_num(*x)).collect();
    app.project.books[0].columns[1].data = xs.iter().map(|x| fmt_num(2.2 * (-0.4 * x).exp() + 0.3)).collect();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.open_custom_nlfit_dialog();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1400.0, 1200.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    for _ in 0..3 {
        harness.frame(&mut app, vec![]);
    }
    rename(&mut harness, &mut app, "自定义函数", "SavedDecay");
    let before = app.project.clone();
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "保存函数");
    assert_eq!(app.project.fit_functions.len(), 1, "{}", app.status);
    let id = app.project.fit_functions[0].id;
    assert_eq!(app.project.fit_functions[0].definition.name, "SavedDecay");
    roundtrip(&mut app, &before);
    harness.frame(&mut app, vec![]);
    let original = app.project.clone();
    rename(&mut harness, &mut app, "SavedDecay", "RenamedDecay");
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "保存函数");
    assert_eq!(app.project.fit_functions[0].id, id);
    assert_eq!(app.project.fit_functions[0].definition.name, "RenamedDecay");
    let edited = app.project.clone();
    app.run_command("edit.undo");
    harness.frame(&mut app, vec![]);
    assert_eq!(app.project, original);
    assert!(matches!(&app.dialog,Some(Dialog::CustomNlFit(edit)) if edit.name=="SavedDecay"));
    app.run_command("edit.redo");
    harness.frame(&mut app, vec![]);
    assert_eq!(app.project, edited);
    assert!(matches!(&app.dialog,Some(Dialog::CustomNlFit(edit)) if edit.name=="RenamedDecay"));
    let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
    app.project = scicanvas_core::project_file::from_scv(&saved).unwrap();
    app.dialog = None;
    app.open_custom_nlfit_dialog();
    assert!(matches!(&app.dialog,Some(Dialog::CustomNlFit(edit)) if edit.before.as_ref().unwrap().id==id));
    let before_fit = app.project.clone();
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert_eq!(reports[0].fit.model.name(), "RenamedDecay");
    assert!(reports[0].fit.converged);
    let reference = reports[0].clone();
    let original_initial = match &app.project.nlfit_operation(&reference).unwrap().kind {
        scicanvas_core::ops::OpKind::Nonlinear(r) => r.initial.clone().unwrap(),
        _ => unreachable!(),
    };
    let after_fit = app.project.clone();
    press_dialog_button(&mut app, "重新编辑拟合设置");
    let Some(Dialog::CustomNlFit(editor)) = &app.dialog else {
        panic!("editor should reopen")
    };
    assert_eq!(editor.definition().unwrap().initial, original_initial);
    assert_eq!(editor.target.as_ref().unwrap().report.id, reference.id);
    assert_eq!(
        app.project, after_fit,
        "reopening an editor leaves fitted output unchanged"
    );
    app.run_command("edit.undo");
    assert_eq!(app.project, before_fit);
    app.run_command("edit.redo");
    assert_eq!(app.project, after_fit);
}

#[test]
fn custom_fit_library_file_import_is_undoable_and_name_conflicts_require_save() {
    use scicanvas_core::{nlfit::CustomFitDefinition, project_file};
    let mut app = app();
    app.open_custom_nlfit_dialog();
    let definition = CustomFitDefinition::from_text("Imported", "a*x+b", "a,b", "2,1", "").unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("scicanvas-app-fit-{}-{stamp}.scfn", std::process::id()));
    project_file::save_fit_function(&path, &definition).unwrap();
    let before = app.project.clone();
    assert!(app.import_custom_fit_function(&path));
    let imported = app.project.clone();
    assert_eq!(imported.fit_functions.len(), 1);
    app.run_command("edit.undo");
    assert_eq!(app.project, before);
    std::fs::write(&path, "broken").unwrap();
    assert!(!app.import_custom_fit_function(&path));
    assert_eq!(app.project, before);
    app.run_command("edit.redo");
    assert_eq!(app.project, imported, "failed imports preserve redo");
    let changed = scicanvas_core::nlfit::CustomFitDefinition {
        formula: "y = a*x+b+1".into(),
        ..definition
    };
    project_file::save_fit_function(&path, &changed).unwrap();
    assert!(app.import_custom_fit_function(&path));
    assert_eq!(
        app.project, imported,
        "same-name files stay in the editor until explicitly saved"
    );
    press_dialog_button(&mut app, "保存函数");
    assert_eq!(app.project.fit_functions[0].id, imported.fit_functions[0].id);
    assert_eq!(app.project.fit_functions[0].definition, changed);
    roundtrip(&mut app, &imported);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn custom_fit_library_dirty_editor_keeps_input_and_rejects_stale_saves() {
    use scicanvas_core::nlfit::CustomFitDefinition;
    let mut app = app();
    let definition = CustomFitDefinition::from_text("Line", "a*x+b", "a,b", "2,1", "").unwrap();
    assert!(app.save_custom_fit_definition(None, definition));
    let original = app.project.fit_functions[0].clone();
    let Some(Dialog::CustomNlFit(edit)) = app.dialog.as_mut() else {
        panic!()
    };
    edit.formula = "y = a*x+b+1".into();
    let mut external = original.definition.clone();
    external.initial = vec![3.0, 2.0];
    app.project.save_fit_function(Some(&original), external).unwrap();
    let before = app.project.clone();
    press_dialog_button(&mut app, "保存函数");
    assert_eq!(app.project, before);
    assert!(app.status.contains("别处更改"), "{}", app.status);
    assert!(matches!(&app.dialog,Some(Dialog::CustomNlFit(edit)) if edit.formula=="y = a*x+b+1"));
}

#[test]
fn rational_models_run_from_dialog_save_reopen_and_undo() {
    use scicanvas_core::{nlfit::Model, project_file, svg};
    for (model, truth) in [
        (Model::Rational1, vec![2., 1.5, 0.2]),
        (Model::Rational1, vec![2., 1.5, -0.35]),
        (Model::Rational12, vec![2., 1.5, 0.2, 0.08]),
        (Model::Rational22, vec![1.2, -0.7, 0.9, 0.3, 0.15]),
        (Model::Rational33, vec![1.2, -0.7, 0.9, 0.12, 0.3, 0.15, 0.04]),
    ] {
        let mut app = app();
        let book = app.project.book_mut(1).unwrap();
        let xs: Vec<f64> = (0..60).map(|row| row as f64 / 12.).collect();
        book.columns[0].data = xs.iter().map(|x| x.to_string()).collect();
        book.columns[1].data = xs.iter().map(|x| model.eval(*x, &truth).to_string()).collect();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        let before = app.project.clone();
        app.open_nlfit_dialog();
        let Some(Dialog::NlFit { model: selected, .. }) = app.dialog.as_mut() else {
            panic!("fit dialog");
        };
        *selected = Model::Rational1;
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut ui = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(1400., 1000.)),
            |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
        )
        .with_step(0.02);
        for _ in 0..5 {
            ui.frame(&mut app, vec![]);
        }
        let output = ui.frame(&mut app, vec![]);
        ui.click_text(&mut app, &output, model.name());
        let mut clicked = false;
        for _ in 0..8 {
            let output = ui.frame(&mut app, vec![]);
            if crate::test_ui::find_text(&output, "拟合").is_some() {
                ui.click_text(&mut app, &output, "拟合");
                clicked = true;
                break;
            }
            let rect = ctx
                .memory(|m| m.area_rect(egui::Id::new(("scicanvas-dialog", "非线性拟合"))))
                .unwrap();
            ui.frame(
                &mut app,
                vec![
                    egui::Event::PointerMoved(rect.center()),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0., -600.),
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            for _ in 0..5 {
                ui.frame(&mut app, vec![]);
            }
        }
        assert!(clicked, "{}: fit action must be reachable by scrolling", model.name());
        let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
            panic!("{}: {}", model.name(), app.status);
        };
        let reports = reports.clone();
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        assert_eq!(report.fit.model.name(), model.name());
        assert!(report.fit.converged);
        assert!(
            report.fit.rss < 1e-12,
            "{} {:?}: {}",
            model.name(),
            truth,
            report.fit.rss
        );
        let curve = app.project.book(1).unwrap().column(&report.curve.1).unwrap();
        assert_eq!(curve.data.len(), 200);
        if model == Model::Rational1 && truth[2] < 0.0 {
            assert!(
                curve.values().iter().any(|value| value.is_nan()),
                "fit curve breaks at its pole"
            );
        }
        assert!(curve.comment.contains(model.formula()));
        let saved = project_file::from_scv(&project_file::to_scv(&app.project).unwrap()).unwrap();
        assert_eq!(saved, app.project);
        let graph = app.project.graphs.last().unwrap();
        assert_eq!(
            svg::render_page(graph, &app.project),
            svg::render_page(saved.graph(graph.id).unwrap(), &saved)
        );
        assert_eq!(app.history.undo_label(), Some("非线性拟合"));
        roundtrip(&mut app, &before);
        let before_bands = app.project.clone();
        app.dialog = Some(Dialog::NlFitReport(reports));
        press_dialog_button(&mut app, "置信/预测带");
        let book = app.project.book(1).unwrap();
        let fit_curve = book.columns[book.columns.len() - 5].values();
        for column in &book.columns[book.columns.len() - 4..] {
            for (y, band) in fit_curve.iter().zip(column.values()) {
                if y.is_nan() {
                    assert!(band.is_nan(), "bands retain fit gaps");
                } else {
                    assert!(band.is_finite());
                }
            }
        }
        let saved = project_file::from_scv(&project_file::to_scv(&app.project).unwrap()).unwrap();
        assert_eq!(saved, app.project);
        roundtrip(&mut app, &before_bands);
    }
}

#[test]
fn persisted_fit_reports_refresh_with_undo_and_keep_notes_as_snapshots() {
    use scicanvas_core::{nlfit::Model, project_file};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.nlfit(Model::ExpDec1, &[], &[]);
    press_dialog_button(&mut app, "保存为 Note");
    let note = app.project.notes.last().unwrap().clone();
    app.run_command("fit.reports");
    let before = app.project.clone();
    let reference = app.project.nlfit_reports()[0].clone();
    app.edit("来源修改", |p| {
        p.books[0].columns[2].set(0, "10".into());
        Ok(Some(()))
    })
    .unwrap();
    let current = app.project.nlfit_report(&reference).unwrap().clone();
    assert_ne!(current.fit, reference.fit);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    );
    harness.frame(&mut app, vec![]);
    assert!(matches!(&app.dialog, Some(Dialog::NlFitReport(r)) if r[0] == current));
    app.undo();
    assert_eq!(app.project, before);
    assert!(matches!(&app.dialog, Some(Dialog::NlFitReport(r)) if r[0] == reference));
    app.redo();
    assert!(matches!(&app.dialog, Some(Dialog::NlFitReport(r)) if r[0] == current));
    assert_eq!(app.project.notes.last(), Some(&note));
    app.project = project_file::from_scv(&project_file::to_scv(&app.project).unwrap()).unwrap();
    app.dialog = None;
    app.run_command("fit.reports");
    assert!(matches!(&app.dialog, Some(Dialog::NlFitReport(r)) if r[0] == current));
    press_dialog_button(&mut app, "保存为 Note");
    assert!(app
        .project
        .notes
        .last()
        .unwrap()
        .text
        .contains(&scicanvas_core::fmt_num(current.fit.rss)));
    assert_eq!(app.project.notes[0], note);
}

#[test]
fn builtin_refit_editor_preserves_group_and_keeps_invalid_or_failed_drafts() {
    use scicanvas_core::{nlfit::Model, ops::OpKind};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.nlfit(Model::ExpDec1, &[], &[]);
    let before = app.project.clone();
    press_dialog_button(&mut app, "重新编辑拟合设置");
    let Some(Dialog::NlFit { initial, target, .. }) = &mut app.dialog else {
        panic!("editor")
    };
    assert!(initial.is_empty());
    assert!(target.is_some());
    *initial = "invalid".into();
    press_dialog_button(&mut app, "拟合");
    assert_eq!(app.project, before);
    assert!(matches!(app.dialog, Some(Dialog::NlFit { .. })));
    let Some(Dialog::NlFit { initial, .. }) = &mut app.dialog else {
        unreachable!()
    };
    *initial = "0, -8, -1e-300".into();
    press_dialog_button(&mut app, "拟合");
    assert_eq!(app.project, before);
    assert!(matches!(app.dialog, Some(Dialog::NlFit { .. })));
    let Some(Dialog::NlFit {
        initial,
        limits,
        constraints,
        ..
    }) = &mut app.dialog
    else {
        unreachable!()
    };
    *initial = "8, -8, 4".into();
    limits[2] = (false, 0.0, true, 1.0, true, 3.0);
    *constraints = "y0 + A1 = 0".into();
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert_eq!(reports[0].fit.params[2], 3.0);
    assert_eq!(app.project.books[0].columns.len(), before.books[0].columns.len());
    assert_eq!(app.project.graphs, before.graphs);
    assert_eq!(reports[0].id, before.nlfit_reports()[0].id);
    assert_eq!(app.history.undo_label(), Some("修改拟合设置"));
    roundtrip(&mut app, &before);
    press_dialog_button(&mut app, "重新编辑拟合设置");
    let Some(Dialog::NlFit {
        initial,
        target,
        limits,
        constraints,
        ..
    }) = &app.dialog
    else {
        panic!("editor")
    };
    assert_eq!(initial, "8, -8, 4");
    assert_eq!(limits[2], (false, 0.0, true, 1.0, true, 3.0));
    assert_eq!(constraints, "y0 + A1 = 0");
    assert!(matches!(&target.as_ref().unwrap().before.kind, OpKind::Nonlinear(_)));
    let after = app.project.clone();
    press_dialog_button(&mut app, "取消");
    assert_eq!(app.project, after);
}

#[test]
fn custom_refit_editor_reloads_limits_and_original_initial_and_updates_in_place() {
    use scicanvas_core::nlfit::{CustomFitFunction, ParamLimits};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    let f = CustomFitFunction::new("Decay", "a+b*exp(-x/t)", vec!["a".into(), "b".into(), "t".into()]).unwrap();
    assert!(app.custom_nlfit(
        f,
        vec![8., -8., 4.],
        vec![
            ParamLimits::default(),
            ParamLimits::default(),
            ParamLimits {
                fixed: None,
                lower: Some(1.),
                upper: Some(6.)
            }
        ],
        vec![]
    ));
    let before = app.project.clone();
    press_dialog_button(&mut app, "重新编辑拟合设置");
    let Some(Dialog::CustomNlFit(edit)) = &mut app.dialog else {
        panic!("editor")
    };
    assert_eq!(edit.initial, "8, -8, 4");
    assert_eq!(edit.limits[2].upper, Some(6.));
    edit.limits[2].upper = Some(3.);
    edit.constraints = "a + b = 0".into();
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert_eq!(reports[0].fit.params[2], 3.);
    assert_eq!(reports[0].id, before.nlfit_reports()[0].id);
    assert_eq!(app.project.books[0].columns.len(), before.books[0].columns.len());
    assert_eq!(app.project.graphs, before.graphs);
    roundtrip(&mut app, &before);
}

#[test]
fn deleted_report_export_is_a_snapshot_and_stale_editor_preserves_redo() {
    use scicanvas_core::{nlfit::Model, ops::OpKind};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.nlfit(Model::ExpDec1, &[], &[]);
    let reference = app.project.nlfit_reports()[0].clone();
    app.edit_nlfit_report(reference.clone());
    app.run_command("book.new");
    app.undo();
    let before = app.project.clone();
    let Some(Dialog::NlFit { target, .. }) = app.dialog.take() else {
        panic!("editor")
    };
    let target = target.unwrap();
    let OpKind::Nonlinear(recipe) = &target.before.kind else {
        unreachable!()
    };
    let recipe = *recipe.clone();
    app.edit("删除拟合结果", |p| {
        p.delete_columns(1, std::slice::from_ref(&reference.curve.1));
        Ok(Some(()))
    })
    .unwrap();
    let deleted = app.project.clone();
    assert!(!app.refit_report(*target, Ok(recipe)));
    assert_eq!(app.project, deleted);
    app.dialog = Some(Dialog::NlFitReport(vec![reference.clone()]));
    press_dialog_button(&mut app, "保存为 Note");
    let note = app.project.notes.last().unwrap();
    assert!(note.text.contains("报告快照"));
    assert!(!note.text.contains("当前有效结果"));
    app.undo();
    app.undo();
    assert_eq!(app.project, before);
    assert!(app.project.nlfit_report(&reference).is_some());
}

#[test]
fn legacy_editor_keeps_file_limits_for_unchanged_loaded_expressions() {
    let formula = std::iter::repeat_n("a*x", 150).collect::<Vec<_>>().join("+");
    let definition = scicanvas_core::nlfit::CustomFitDefinition::from_text_with(
        "old",
        &formula,
        "a",
        "1",
        "",
        &scicanvas_core::formula::FILE_LIMITS,
    )
    .unwrap();
    let mut edit = dialogs::CustomFitEditor::from_definition(&definition);
    edit.initial = "2".into();
    assert_eq!(edit.definition().unwrap().initial, vec![2.]);
    edit.formula.push_str("+a*x");
    assert!(edit.definition().is_err());
}

#[test]
fn legacy_library_function_fits_through_the_real_dialog_button() {
    let mut app = app();
    let formula = std::iter::repeat_n("a*x", 150).collect::<Vec<_>>().join("+");
    let definition = scicanvas_core::nlfit::CustomFitDefinition::from_text_with(
        "legacy",
        &formula,
        "a",
        "1",
        "",
        &scicanvas_core::formula::FILE_LIMITS,
    )
    .unwrap();
    app.project.save_fit_function(None, definition).unwrap();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.open_custom_nlfit_dialog();
    press_dialog_button(&mut app, "拟合");
    assert!(
        matches!(&app.dialog,Some(Dialog::NlFitReport(r)) if r.len()==1),
        "{}",
        app.status
    );
}

#[test]
fn analysis_dialog_ignores_enter_without_focus() {
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.open_poly_dialog();
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
    );
    for _ in 0..3 {
        h.frame(&mut app, vec![]);
    }
    h.frame(&mut app, vec![key(egui::Key::Escape)]);
    assert!(app.dialog.is_none());
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.open_poly_dialog();
    for _ in 0..3 {
        h.frame(&mut app, vec![]);
    }
    let before = app.project.clone();
    let generation = app.history.generation();
    h.frame(&mut app, vec![key(egui::Key::Enter)]);
    assert!(matches!(app.dialog, Some(Dialog::Poly { .. })));
    assert_eq!(app.project, before);
    assert_eq!(app.history.generation(), generation);
}
#[test]
fn nonlinear_report_uses_the_book_name() {
    let mut app = app();
    app.project.books[0].name = "实验数据".into();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.nlfit(scicanvas_core::nlfit::Model::ExpDec1, &[], &[]);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
    );
    let mut out = h.frame(&mut app, vec![]);
    for _ in 0..3 {
        out = h.frame(&mut app, vec![]);
    }
    let text = out
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                Some(t.galley.job.text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("实验数据 · 结果列"));
    assert!(!text.contains("Book 1"));
}
