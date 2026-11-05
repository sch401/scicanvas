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
        press_dialog_button(&mut app, "置信/预测带（95%）");
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
    assert!(dialog_rendered_texts(&mut app)
        .iter()
        .any(|text| text.contains("不是参数名")));
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
fn weighted_fit_checkbox_reaches_the_report_and_reedit() {
    use scicanvas_core::nlfit::Model;
    let mut app = app();
    let book = app.project.book_mut(1).unwrap();
    let xs: Vec<f64> = (0..41).map(|i| i as f64 * 0.2).collect();
    book.columns[0].data = xs.iter().map(|x| fmt_num(*x)).collect();
    book.columns[1].data = xs
        .iter()
        .enumerate()
        .map(|(i, x)| fmt_num(0.3 + 5.0 * (-x / 1.7).exp() + 0.05 * (1.7 * i as f64).sin()))
        .collect();
    book.columns[2].role = scicanvas_core::Role::YError;
    book.columns[2].data = xs.iter().map(|x| fmt_num(0.02 + 0.05 * x)).collect();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let before = app.project.clone();
    let ctx = egui::Context::default();
    assert!(app.apply_dialog_action(
        &ctx,
        dialogs::Action::NlFit(Model::ExpDec1, vec![], String::new(), true)
    ));
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("weighted fit report should open: {}", app.status);
    };
    assert!(reports[0].fit.weighted);
    let report = reports[0].clone();
    assert_eq!(app.history.undo_label(), Some("非线性拟合"));
    app.edit_nlfit_report(report);
    assert!(matches!(app.dialog, Some(Dialog::NlFit { weighted: true, .. })));
    app.dialog = None;
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
    app.column_math(ColumnOp::Sub, None).unwrap();

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
    app.frequency_counts(Some(4), None).unwrap();
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
    app.subtract_baseline(scicanvas_core::baseline::BaselineMethod::Line)
        .unwrap();

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
    app.find_peaks(0.05).unwrap();

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
    app.fill_sequence(scicanvas_core::stats::Sequence::Arithmetic { start: 0.0, step: 0.5 }, 3)
        .unwrap();
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
        (Some(Dialog::Table { title, rows, note }), Ok(r)) => {
            assert_eq!(rows.len(), 4);
            assert_eq!(rows[0][0], "参数");
            assert!(title.contains(book.column("B").unwrap().display_name()));
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
    app.fft_filter(scicanvas_core::fft::FilterKind::LowPass(lo)).unwrap();
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 1, "{}", app.status);
}

#[test]
fn normality_three_tests_and_grubbs_report_picked_columns() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    app.normality();
    match &app.dialog {
        Some(Dialog::Table { rows, .. }) => {
            assert_eq!(rows.len(), 7);
            let w = scicanvas_core::stats::shapiro_wilk(&app.project.book(1).unwrap().column("B").unwrap().values())
                .unwrap();
            assert_eq!(rows[1][3], fmt_num(round_sig(w.w, 6)));
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
fn calibrate_prefills_available_columns_and_allows_missing_choices() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into()]);
    app.calibrate();
    assert!(
        matches!(&app.dialog,Some(Dialog::Calibration{keys,..}) if keys==&["A".to_string(),"B".to_string(),String::new()])
    );
    let cols = app.project.book(1).unwrap().columns.len();
    app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
    app.calibrate();
    assert!(matches!(&app.dialog, Some(Dialog::Calibration { forward: false, .. })));
    press_dialog_button(&mut app, "计算");
    assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 3, "{}", app.status);
    assert!(app.status.contains("标准曲线结果"));
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
                        advanced: 0,
                        count: 10,
                        spline: false,
                        order: 2,
                        iterations: 100,
                        anchors: String::new(),
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
                        mode: 0,
                        target: None,
                        single: 0.,
                        result: Vec::new(),
                        options: Vec::new(),
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
                (
                    Dialog::Peaks {
                        percent: 5.0,
                        derivative: false,
                        window: 11,
                    },
                    "寻峰",
                    "寻峰",
                )
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
        let at = crate::test_ui::text_rect(&output, "置信/预测带（95%）")
            .unwrap()
            .center();
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
    for _ in 0..4 {
        harness.frame_value(&mut app, vec![]);
    }
    let rect = ctx
        .memory(|m| m.area_rect(egui::Id::new(("scicanvas-dialog", "列属性 — B"))))
        .unwrap();
    harness.frame_value(&mut app, vec![egui::Event::PointerMoved(rect.center())]);
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
        dialogs::Action::NlFit(Model::ExpDec1, vec![], "y0 + A1 = q".into(), false)
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
        press_dialog_button(&mut app, "置信/预测带（95%）");
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
    assert!(note.text.contains("删除前的报告"));
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
fn analysis_dialog_enter_submits_valid_input_and_escape_closes() {
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
        ctx.clone(),
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
    );
    for _ in 0..3 {
        h.frame(&mut app, vec![]);
    }
    let at = ctx
        .data(|d| d.get_temp::<(egui::Rect, egui::WidgetInfo)>(egui::Id::new(("dialog-primary", "拟合"))))
        .unwrap()
        .0
        .center();
    h.frame(&mut app, vec![egui::Event::PointerMoved(at)]);
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
    let at = ctx
        .data(|d| d.get_temp::<(egui::Rect, egui::WidgetInfo)>(egui::Id::new(("dialog-primary", "拟合"))))
        .unwrap()
        .0
        .center();
    h.frame(&mut app, vec![egui::Event::PointerMoved(at)]);
    h.frame(&mut app, vec![key(egui::Key::Enter)]);
    assert!(matches!(app.dialog, Some(Dialog::FitReport(..))));
    assert_ne!(app.project, before);
    assert_eq!(app.history.generation(), generation + 1);
    roundtrip(&mut app, &before);
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

#[test]
fn nine_parameter_real_dialog_inputs_fit_report_reedit_and_undo() {
    let mut app = app();
    let xs: Vec<_> = (0..96).map(|i| i as f64 * std::f64::consts::TAU / 96.0).collect();
    app.project.books[0].columns[0].data = xs.iter().map(ToString::to_string).collect();
    app.project.books[0].columns[1].data = xs
        .iter()
        .map(|x| {
            (1..=9)
                .map(|i| i as f64 / 10.0 * (i as f64 * x).sin())
                .sum::<f64>()
                .to_string()
        })
        .collect();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.open_custom_nlfit_dialog();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut ui = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1400.0, 1300.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    for _ in 0..3 {
        ui.frame(&mut app, vec![]);
    }
    let formula = (1..=9)
        .map(|i| format!("b{i}*sin({i}*x)"))
        .collect::<Vec<_>>()
        .join("+");
    let names = (1..=9).map(|i| format!("b{i}")).collect::<Vec<_>>().join(", ");
    for (old, new) in [
        ("自定义函数", "NineSine".to_string()),
        ("y = a*exp(-b*x)+c", formula),
        ("a, b, c", names),
        ("2, 0.3, 0", ["0"; 9].join(", ")),
    ] {
        let out = ui.frame(&mut app, vec![]);
        let at = crate::test_ui::text_positions(&out, old)
            .into_iter()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap();
        ui.click(&mut app, at + egui::vec2(5.0, 5.0));
        ui.modifiers = egui::Modifiers::COMMAND;
        ui.frame(
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
        );
        ui.modifiers = egui::Modifiers::NONE;
        ui.frame(&mut app, vec![egui::Event::Text(new)]);
    }
    let Some(Dialog::CustomNlFit(editor)) = &app.dialog else {
        panic!()
    };
    assert_eq!(editor.definition().unwrap().parameters.len(), 9);
    let before = app.project.clone();
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    let report = reports[0].clone();
    assert!(report.fit.converged);
    assert_eq!(report.fit.params.len(), 9);
    for (i, actual) in report.fit.params.iter().enumerate() {
        assert!((actual - (i + 1) as f64 / 10.0).abs() < 1e-7);
    }
    if let Ok(path) = std::env::var("SCICANVAS_9PARAM_UI_PROJECT") {
        scicanvas_core::project_file::save(std::path::Path::new(&path), &app.project).unwrap();
    }
    roundtrip(&mut app, &before);
    app.dialog = Some(Dialog::NlFitReport(vec![report.clone()]));
    press_dialog_button(&mut app, "重新编辑拟合设置");
    for _ in 0..3 {
        ui.frame(&mut app, vec![]);
    }
    let Some(Dialog::CustomNlFit(editor)) = &app.dialog else {
        panic!()
    };
    assert_eq!(editor.limits.len(), 9);
    assert_eq!(editor.target.as_ref().unwrap().report.id, report.id);
    // 正常工作区的滚动和取消按钮点击由下一项严格裁剪回归覆盖。
    ui.frame(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.dialog.is_none());
    let after = app.project.clone();
    app.undo();
    assert_eq!(app.project, before);
    app.redo();
    assert_eq!(app.project, after);
}

#[test]
fn custom_fit_parameter_controls_scroll_into_view_in_a_normal_workspace() {
    for (width, height, count) in [(1229.0, 706.0, 9), (900.0, 650.0, 12)] {
        let mut app = app();
        let xs: Vec<_> = (0..64).map(|i| i as f64 * std::f64::consts::TAU / 64.0).collect();
        app.project.books[0].columns[0].data = xs.iter().map(ToString::to_string).collect();
        app.project.books[0].columns[1].data = xs
            .iter()
            .map(|x| {
                (1..=count)
                    .map(|i| i as f64 / 10.0 * (i as f64 * x).sin())
                    .sum::<f64>()
                    .to_string()
            })
            .collect();
        let function = scicanvas_core::nlfit::CustomFitFunction::new(
            "Scroll",
            &(1..=count)
                .map(|i| format!("b{i}*sin({i}*x)"))
                .collect::<Vec<_>>()
                .join("+"),
            (1..=count).map(|i| format!("b{i}")).collect(),
        )
        .unwrap();
        let (_, reports) = app
            .project
            .nlfit_custom(
                &scicanvas_core::ops::Source::Book {
                    id: 1,
                    selected: vec!["B".into()],
                    rows: None,
                },
                function,
                &vec![0.0; count],
                &[],
                &[],
            )
            .unwrap();
        app.active = Some(1);
        app.edit_nlfit_report(reports[0].clone());
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut ui = crate::test_ui::Harness::new(
            ctx,
            Some(egui::vec2(width, height)),
            |app: &mut SciApp, ctx: &egui::Context| app.ui_frame(ctx),
        )
        .with_step(0.02);
        let visible = |out: &egui::FullOutput, label: &str| -> Option<egui::Rect> {
            out.shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        let rect = text.galley.rect.translate(text.pos.to_vec2());
                        (shape.clip_rect.contains_rect(rect)
                            && egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height))
                                .contains_rect(rect))
                        .then_some(rect)
                    }
                    _ => None,
                })
                .next_back()
        };
        let mut out = ui.frame(&mut app, vec![]);
        for _ in 0..5 {
            out = ui.frame(&mut app, vec![]);
        }
        let id = egui::Id::new(("scicanvas-dialog", "自定义非线性拟合"));
        let label = format!("b{count}");
        for _ in 0..40 {
            if let Some(last) = visible(&out, &label) {
                let upper_visible = ui
                    .ctx
                    .data(|d| {
                        d.get_temp::<egui::Rect>(egui::Id::new((
                            "parameter-toggle",
                            "custom-limits",
                            label.as_str(),
                            2_usize,
                        )))
                    })
                    .is_some_and(|r| {
                        r.center().y >= last.top() && out.shapes.iter().any(|s| s.clip_rect.contains_rect(r))
                    });
                if upper_visible && visible(&out, "拟合").is_some() {
                    break;
                }
            }
            let rect = ui.ctx.memory(|m| m.area_rect(id)).unwrap();
            let at = egui::pos2(rect.left() + 100.0, rect.bottom() - 80.0);
            ui.frame(
                &mut app,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -120.0),
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            for _ in 0..3 {
                out = ui.frame(&mut app, vec![]);
            }
        }
        let _last = visible(&out, &label).unwrap_or_else(|| {
            panic!("{width}x{height}: {label} must be fully inside the clip and screen after scrolling")
        });
        assert!(visible(&out, "拟合").is_some());
        let upper = ui
            .ctx
            .data(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new((
                    "parameter-toggle",
                    "custom-limits",
                    label.as_str(),
                    2_usize,
                )))
            })
            .expect("last parameter upper-limit checkbox");
        ui.click(&mut app, upper.center());
        assert!(matches!(&app.dialog,Some(Dialog::CustomNlFit(editor)) if editor.limits[count-1].upper==Some(0.0)));
        let rect = ui.ctx.memory(|m| m.area_rect(id)).unwrap();
        assert!(
            rect.height() < height - 100.0,
            "parameter count must not expand the dialog beyond the workspace"
        );
        let out = ui.frame(&mut app, vec![]);
        ui.click(&mut app, visible(&out, "取消").unwrap().center());
        assert!(app.dialog.is_none());
        assert_eq!(app.project, before);
    }
}

#[test]
fn nonlinear_fit_warning_is_visible_in_report() {
    use scicanvas_core::{nlfit::CustomFitFunction, ops::Source};
    let mut app = app();
    app.project.books[0].columns[0].data = ["1", "2", "3", "5", "7", "10"].into_iter().map(String::from).collect();
    app.project.books[0].columns[1].data = ["109", "149", "149", "191", "213", "224"]
        .into_iter()
        .map(String::from)
        .collect();
    let function = CustomFitFunction::new("BoxBOD", "b1*(1-exp(-b2*x))", vec!["b1".into(), "b2".into()]).unwrap();
    let source = Source::Book {
        id: 1,
        selected: vec!["B".into()],
        rows: None,
    };
    let (_, reports) = app
        .project
        .nlfit_custom(&source, function, &[1.0, 1.0], &[], &[])
        .unwrap();
    let warning = reports[0].fit.warnings[0].clone();
    app.dialog = Some(Dialog::NlFitReport(reports));
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    );
    for _ in 0..3 {
        harness.frame(&mut app, vec![]);
    }
    let output = harness.frame(&mut app, vec![]);
    assert!(crate::test_ui::find_text(&output, &warning).is_some());
}

#[test]
fn regression_uses_only_visible_rows_when_requested() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into()]);
    let book = app.project.book_mut(1).unwrap();
    book.columns[0].data = (1..=6).map(|v| v.to_string()).collect::<Vec<_>>().into();
    book.columns[1].data = [2, 4, 6, 8, 10, 100]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .into();
    let mut filter = scicanvas_core::filter::Filter::default();
    filter.rules.insert(
        "A".into(),
        scicanvas_core::filter::FilterRule::Number {
            min: None,
            max: Some(5.0),
        },
    );
    filter.visible_only = true;
    book.filter = Some(filter);
    app.regression();
    let Some(Dialog::Table { note, .. }) = &app.dialog else {
        panic!("应显示回归报告：{}", app.status)
    };
    assert!(note.contains("N = 5"), "{note}");
}

#[test]
fn ifft_rejects_filtered_spectra_when_only_visible_rows_are_selected() {
    let mut app = app();
    app.active = Some(1);
    let mut filter = scicanvas_core::filter::Filter::default();
    filter.rules.insert(
        "A".into(),
        scicanvas_core::filter::FilterRule::Number {
            min: Some(1.0),
            max: None,
        },
    );
    filter.visible_only = true;
    app.project.book_mut(1).unwrap().filter = Some(filter);
    app.open_ifft_dialog();
    assert!(app.dialog.is_none());
    assert!(app.status.contains("IFFT 需要完整频谱"));
}

#[test]
fn a1_ks_dixon_reports_and_two_way_dialog_save_note_undo() {
    use scicanvas_core::ColumnData;
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut app = SciApp::initial(theme::palette(theme));
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        app.run_command("stats.ks2");
        assert!(matches!(&app.dialog,Some(Dialog::Table { rows,.. }) if rows.len()==2 && rows[1][6]=="精确"));
        press_dialog_button(&mut app, "关闭");
        let book = app.project.book_mut(1).unwrap();
        book.columns[0].data = ColumnData::Text(
            ["low", "low", "low", "low", "high", "high", "high", "high"]
                .map(str::to_string)
                .to_vec(),
        )
        .into();
        book.columns[1].data = ColumnData::Text(
            ["left", "left", "right", "right", "left", "left", "right", "right"]
                .map(str::to_string)
                .to_vec(),
        )
        .into();
        book.columns[2].data = [1., 2., 4., 5., 3., 5., 8., 9.]
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .into();
        let before = app.project.clone();
        app.selection.insert(1, vec!["C".into()]);
        app.run_command("stats.dixon");
        assert!(matches!(&app.dialog,Some(Dialog::Table { rows,.. }) if rows.len()==2 && rows[0].len()==8));
        press_dialog_button(&mut app, "关闭");
        app.selection.insert(1, vec!["C".into(), "A".into()]);
        app.run_command("stats.ks2");
        assert!(app.status.contains("有效数值"));
        app.selection.insert(1, vec!["C".into()]);
        app.run_command("stats.ks2");
        assert!(app.status.contains("恰好"));
        app.run_command("stats.normality");
        assert!(matches!(&app.dialog,Some(Dialog::Table { rows,.. }) if rows.len()==4));
        press_dialog_button(&mut app, "关闭");
        app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
        app.run_command("stats.anova2");
        assert!(matches!(&app.dialog,Some(Dialog::TwoWayAnova { keys,.. }) if keys== &["A","B","C"]));
        press_dialog_button(&mut app, "计算");
        assert!(
            matches!(&app.dialog,Some(Dialog::Table { rows,note,.. }) if rows.len()==6 && note.contains("N = 8")),
            "{}",
            app.status
        );
        assert_eq!(app.project, before);
        press_dialog_button(&mut app, "保存为 Note");
        assert!(app.project.notes.last().unwrap().text.contains("Type II"));
        roundtrip(&mut app, &before);
    }
}

#[test]
fn a1_dixon_reports_original_row_inside_selection() {
    let mut app = app();
    app.active = Some(1);
    let book = app.project.book_mut(1).unwrap();
    book.columns[1].data = ["99", "99", "1", "2", "3", "4", "50"]
        .map(str::to_string)
        .to_vec()
        .into();
    app.selection.remove(&1);
    app.sheets.entry(1).or_default().range = Some(crate::sheet::Range {
        a: crate::sheet::Cell { c: 1, r: 2 },
        b: crate::sheet::Cell { c: 1, r: 6 },
    });
    app.run_command("stats.dixon");
    assert!(
        matches!(&app.dialog,Some(Dialog::Table { rows,.. }) if rows[1][4]=="7"),
        "{}",
        app.status
    );
}

#[test]
fn b1_new_plot_commands_create_and_undo_in_one_step() {
    for command in [
        "plot.stacked_horizontal_bar",
        "plot.percent_stacked_horizontal_bar",
        "plot.stacked_area",
        "plot.fill_between",
    ] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        let before = app.project.clone();
        app.run_command(command);
        assert_eq!(
            app.project.graphs.len(),
            before.graphs.len() + 1,
            "{command}: {}",
            app.status
        );
        roundtrip(&mut app, &before);
    }
}

#[test]
fn a2_residual_qq_comparison_and_nonlinear_calibration_follow_real_controls() {
    use scicanvas_core::nlfit::Model;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let xs: Vec<_> = (0..61).map(|i| i as f64 * 0.15).collect();
    let book = app.project.book_mut(1).unwrap();
    book.columns.iter_mut().find(|c| c.key == "A").unwrap().data = xs.iter().map(ToString::to_string).collect();
    book.columns.iter_mut().find(|c| c.key == "B").unwrap().data = xs
        .iter()
        .enumerate()
        .map(|(i, x)| {
            (0.3 + 3.0 * (-x / 1.2).exp() + 1.4 * (-x / 4.5).exp() + 0.02 * (i as f64 * 1.7).sin()).to_string()
        })
        .collect();
    app.nlfit(Model::ExpDec1, &[], &[]);
    let before = app.project.clone();
    press_dialog_button(&mut app, "残差 Q-Q");
    assert!(app
        .project
        .graphs
        .iter()
        .any(|g| g.first_layer().plot_type == PlotType::QQ));
    roundtrip(&mut app, &before);
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.nlfit(Model::ExpDec2, &[], &[]);
    app.open_fit_compare();
    assert!(matches!(&app.dialog, Some(Dialog::FitCompare { .. })), "{}", app.status);
    press_dialog_button(&mut app, "比较");
    assert!(
        matches!(&app.dialog,Some(Dialog::Table {rows,note,..}) if rows.len()==3 && note.contains("K =") && note.contains("F ="))
    );
    let before = app.project.clone();
    press_dialog_button(&mut app, "保存为 Note");
    roundtrip(&mut app, &before);
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
    app.project
        .book_mut(1)
        .unwrap()
        .columns
        .iter_mut()
        .find(|c| c.key == "C")
        .unwrap()
        .data = ["0", "1", "2"].map(str::to_string).to_vec().into();
    app.calibrate();
    let Some(Dialog::Calibration { model, forward, .. }) = &mut app.dialog else {
        panic!("calibration dialog")
    };
    *model = Some(0);
    *forward = true;
    let before = app.project.clone();
    let count = before.book(1).unwrap().columns.len();
    press_dialog_button(&mut app, "计算");
    assert_eq!(app.project.book(1).unwrap().columns.len(), count + 3, "{}", app.status);
    roundtrip(&mut app, &before);
}

#[test]
fn b2_commands_mapping_and_color_scale_actions_undo() {
    for command in [
        "plot.error_band",
        "plot.column_scatter",
        "plot.bubble",
        "plot.color_scatter",
        "plot.bubble_color",
    ] {
        let mut app = app();
        app.active = Some(1);
        let book = app.project.book_mut(1).unwrap();
        if command == "plot.error_band" {
            book.columns.iter_mut().find(|c| c.key == "C").unwrap().role = Role::YError;
        }
        if command == "plot.bubble_color" {
            let key = book.push_column(Role::Y);
            book.columns.iter_mut().find(|c| c.key == key).unwrap().data =
                ["0", "1", "2", "3"].map(str::to_string).to_vec().into();
        }
        let keys = if command == "plot.bubble_color" {
            vec!["A".into(), "B".into(), "C".into(), "D".into()]
        } else {
            vec!["A".into(), "B".into(), "C".into()]
        };
        app.selection.insert(1, keys);
        let before = app.project.clone();
        app.run_command(command);
        assert_eq!(
            app.project.graphs.len(),
            before.graphs.len() + 1,
            "{command}: {}",
            app.status
        );
        roundtrip(&mut app, &before);
        if command == "plot.color_scatter" {
            let id = app.project.graphs.last().unwrap().id;
            let before = app.project.clone();
            app.apply_object_action(
                id,
                crate::plot::ObjectAction::Move(crate::plot::ObjectTarget::ColorScale(0), [0.7, 0.2]),
            );
            app.finish_edit();
            assert_eq!(
                app.project
                    .graph(id)
                    .unwrap()
                    .first_layer()
                    .color_scale
                    .as_ref()
                    .unwrap()
                    .position,
                Some([0.7, 0.2])
            );
            roundtrip(&mut app, &before);
        }
    }
}

#[test]
fn b2_color_scale_actual_pointer_drag_is_undoable() {
    let mut app = app();
    app.active = Some(2);
    let spec = scicanvas_core::plotspec::PlotSpec::from_selection(
        app.project.book(1).unwrap(),
        &["A".into(), "B".into(), "C".into()],
        PlotType::ColorScatter,
    )
    .unwrap();
    app.project.graph_mut(2).unwrap().first_layer_mut().spec = spec.into();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let rect = object_frame(&mut app, &ctx, vec![], false);
    let sc = scicanvas_core::scene::layout_page(app.project.graph(2).unwrap(), &app.project, &[]);
    let object = sc
        .objects
        .iter()
        .find(|o| o.target == crate::plot::ObjectTarget::ColorScale(0))
        .unwrap();
    let at = egui::pos2(
        rect.left() + (object.bounds[0] + 4.0) as f32 * rect.width() / sc.w as f32,
        rect.top() + (object.bounds[1] + 40.0) as f32 * rect.height() / sc.h as f32,
    );
    let before = app.project.clone();
    let end = at + egui::vec2(-40., 30.);
    object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at)], false);
    object_frame(&mut app, &ctx, vec![object_button(at, true)], false);
    object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)], false);
    object_frame(&mut app, &ctx, vec![object_button(end, false)], false);
    assert!(app
        .project
        .graph(2)
        .unwrap()
        .first_layer()
        .color_scale
        .as_ref()
        .is_some_and(|c| c.position.is_some()));
    roundtrip(&mut app, &before);
}

#[test]
fn a3_global_fit_real_dialog_writes_curves_note_and_undoes_once() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    let x: Vec<_> = (0..41).map(|i| i as f64 * 0.2).collect();
    let book = app.project.book_mut(1).unwrap();
    for (key, values) in [
        ("A", x.clone()),
        ("B", x.iter().map(|x| 0.2 + 4.0 * (-x / 1.7).exp()).collect()),
        ("C", x.iter().map(|x| 0.4 + 6.0 * (-x / 1.7).exp()).collect()),
    ] {
        book.columns.iter_mut().find(|c| c.key == key).unwrap().data = values.iter().map(ToString::to_string).collect();
    }
    app.run_command("fit.global");
    let Some(Dialog::GlobalFit(edit)) = &mut app.dialog else {
        panic!("{}", app.status)
    };
    edit.shared = vec![false, false, true];
    let before = app.project.clone();
    let count = before.book(1).unwrap().columns.len();
    press_dialog_button(&mut app, "拟合");
    assert_eq!(app.project.book(1).unwrap().columns.len(), count + 4, "{}", app.status);
    assert_eq!(app.project.notes.len(), before.notes.len() + 1);
    assert!(matches!(&app.dialog,Some(Dialog::Table{rows,note,..}) if rows.len()==3 && note.contains("t1")));
    roundtrip(&mut app, &before);
}

#[test]
fn b3_new_commands_create_offset_or_row_series_and_undo() {
    for command in ["plot.offset_line", "plot.waterfall", "plot.line_series"] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        let before = app.project.clone();
        app.run_command(command);
        assert_eq!(app.project.graphs.len(), before.graphs.len() + 1, "{}", app.status);
        roundtrip(&mut app, &before);
    }
}

#[test]
fn a4_damped_model_real_report_reedit_preserves_fixed_origin() {
    use scicanvas_core::nlfit::Model;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let x: Vec<_> = (0..201).map(|i| 5. + i as f64 * 0.05).collect();
    let y: Vec<_> = x
        .iter()
        .map(|x| 0.3 + 3. * (-(x - 5.) / 6.).exp() * (std::f64::consts::TAU * (x - 5.3) / 2.5).sin())
        .collect();
    for (key, values) in [("A", x), ("B", y)] {
        app.project
            .book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == key)
            .unwrap()
            .data = values.iter().map(ToString::to_string).collect();
    }
    let before = app.project.clone();
    app.nlfit(Model::SineDamp, &[], &[]);
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert!(reports[0].fit.converged);
    assert!(matches!(reports[0].fit.model,scicanvas_core::nlfit::FitFunction::SineDamp{origin} if origin==5.));
    press_dialog_button(&mut app, "重新编辑拟合设置");
    assert!(matches!(
        &app.dialog,
        Some(Dialog::NlFit {
            model: Model::SineDamp,
            target: Some(_),
            ..
        })
    ));
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert!(matches!(reports[0].fit.model,scicanvas_core::nlfit::FitFunction::SineDamp{origin} if origin==5.));
    assert_ne!(app.project, before);
}

#[test]
fn b4_new_plot_commands_save_and_undo() {
    for command in ["plot.pie", "plot.violin", "plot.polar_line", "plot.polar_scatter"] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        let before = app.project.clone();
        app.run_command(command);
        assert_eq!(app.project.graphs.len(), before.graphs.len() + 1, "{}", app.status);
        let json = scicanvas_core::project_file::to_json(&app.project).unwrap();
        assert_eq!(app.project, scicanvas_core::project_file::from_json(&json).unwrap());
        roundtrip(&mut app, &before);
    }
}

#[test]
fn a5_peak_fit_real_preview_fit_report_reedit_and_undo() {
    use scicanvas_core::nlfit::{Model, PeakBaseline, PeakSum};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let peak = PeakSum::new(Model::Gauss, 2, PeakBaseline::Constant).unwrap();
    let x: Vec<_> = (0..201).map(|i| -5. + i as f64 * 0.05).collect();
    let y: Vec<_> = x
        .iter()
        .map(|x| peak.eval(*x, &[-1.5, 1.2, 3., 1.5, 1.4, 4., 0.3]))
        .collect();
    for (key, values) in [("A", x), ("B", y)] {
        app.project
            .book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == key)
            .unwrap()
            .data = values.iter().map(ToString::to_string).collect();
    }
    let before = app.project.clone();
    app.run_command("fit.peaks");
    assert!(matches!(&app.dialog, Some(Dialog::PeakFit(_))));
    press_dialog_button(&mut app, "预览初值");
    assert!(matches!(&app.dialog,Some(Dialog::PeakFit(edit)) if edit.preview.is_some()));
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert!(reports[0].fit.converged);
    assert_eq!(reports[0].peak_rows().len(), 3);
    let keys = reports[0].curve.clone();
    assert_eq!(
        app.project.book(1).unwrap().columns.len(),
        before.book(1).unwrap().columns.len() + 4
    );
    roundtrip(&mut app, &before);
    press_dialog_button(&mut app, "重新编辑拟合设置");
    assert!(matches!(&app.dialog,Some(Dialog::PeakFit(edit)) if edit.target.is_some()&&edit.count==2));
    press_dialog_button(&mut app, "拟合");
    let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert_eq!(reports[0].curve, keys);
}

#[test]
fn b5_function_dialog_validates_creates_edits_and_undoes() {
    let mut app = app();
    let before = app.project.clone();
    app.run_command("plot.function");
    let Some(Dialog::FunctionPlot(edit)) = &mut app.dialog else {
        panic!()
    };
    edit.expr = "unknown+x".into();
    press_dialog_button(&mut app, "绘制");
    assert_eq!(app.project, before);
    let Some(Dialog::FunctionPlot(edit)) = &mut app.dialog else {
        panic!()
    };
    edit.expr = "sin(x)".into();
    press_dialog_button(&mut app, "绘制");
    let id = app.project.graphs.last().unwrap().id;
    assert_eq!(app.project.books, before.books);
    assert_eq!(app.project.graph(id).unwrap().first_layer().functions.len(), 1);
    roundtrip(&mut app, &before);
    app.edit_function_plot(id, 0, 0);
    let Some(Dialog::FunctionPlot(edit)) = &mut app.dialog else {
        panic!()
    };
    edit.expr = "exp(-x*x)".into();
    let before = app.project.clone();
    press_dialog_button(&mut app, "绘制");
    assert_eq!(
        app.project.graph(id).unwrap().first_layer().functions[0].expr,
        "exp(-x*x)"
    );
    roundtrip(&mut app, &before);
}
#[test]
fn b5_style_paste_preserves_target_mapping_and_theme_defaults_are_undoable() {
    use scicanvas_core::{
        graph_theme::{GraphTheme, StyleClip},
        page::CurveStyle,
    };
    let mut app = app();
    app.active = Some(2);
    app.project.graph_mut(2).unwrap().first_layer_mut().line_width = 3.;
    app.style_target = Some((2, 0, None));
    app.run_command("graph.style.copy");
    let id = app.project.alloc_id();
    let mut graph = app.project.graph(2).unwrap().clone();
    graph.id = id;
    graph.first_layer_mut().line_width = 1.;
    app.project.graphs.push(graph);
    app.active = Some(id);
    app.style_target = Some((id, 0, None));
    let before = app.project.clone();
    app.run_command("graph.style.paste");
    assert_eq!(app.project.graph(id).unwrap().first_layer().line_width, 3.);
    roundtrip(&mut app, &before);
    app.active = Some(2);
    app.style_target = Some((2, 0, Some("B".into())));
    app.project.graph_mut(2).unwrap().first_layer_mut().set_plot_style(
        "B",
        Some(CurveStyle {
            line_width: Some(4.),
            color_column: Some("C".into()),
            legend_label: "source".into(),
            ..Default::default()
        }),
    );
    app.run_command("graph.style.copy");
    assert!(
        matches!(&app.style_clipboard,Some(StyleClip::Curve(style)) if style.color_column.is_none()&&style.legend_label.is_empty())
    );
    app.project.graph_mut(id).unwrap().first_layer_mut().set_plot_style(
        "B",
        Some(CurveStyle {
            color_column: Some("C".into()),
            legend_label: "target".into(),
            ..Default::default()
        }),
    );
    app.active = Some(id);
    app.style_target = Some((id, 0, Some("B".into())));
    let before = app.project.clone();
    app.run_command("graph.style.paste");
    let style = app.project.graph(id).unwrap().first_layer().plot_style("B").unwrap();
    assert_eq!(style.legend_label, "target");
    assert_eq!(style.color_column.as_deref(), Some("C"));
    assert_eq!(style.line_width, Some(4.));
    roundtrip(&mut app, &before);
    let mut theme = GraphTheme::nature();
    theme.line_width = 2.5;
    app.settings.default_graph_theme = Some(theme);
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let before = app.project.clone();
    app.run_command("plot.line");
    assert_eq!(app.project.graphs.last().unwrap().first_layer().line_width, 2.5);
    roundtrip(&mut app, &before);
}

#[test]
fn a6_single_interpolation_query_keeps_history_and_target_column_undoes() {
    use scicanvas_core::{interpolate::InterpMethod, plotspec::DataRef};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    for (key, values) in [
        ("A", vec![0., 1., 2., 3.]),
        ("B", vec![0., 2., 4., 6.]),
        ("C", vec![2.5, f64::NAN, 0.5, 1.5]),
    ] {
        app.project
            .book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == key)
            .unwrap()
            .data = values.iter().map(ToString::to_string).collect();
    }
    app.open_interpolate_dialog();
    let Some(Dialog::Interpolate {
        mode, single, method, ..
    }) = &mut app.dialog
    else {
        panic!()
    };
    *mode = 2;
    *single = 1.25;
    *method = InterpMethod::Linear;
    let before = app.project.clone();
    let undo = app.history.undo_label().map(str::to_owned);
    press_dialog_button(&mut app, "查询");
    assert_eq!(app.project, before);
    assert_eq!(app.history.undo_label().map(str::to_owned), undo);
    assert!(matches!(&app.dialog,Some(Dialog::Interpolate{result,..}) if result[0].1==2.5));
    let Some(Dialog::Interpolate { mode, target, .. }) = &mut app.dialog else {
        panic!()
    };
    *mode = 1;
    *target = Some(DataRef::new(1, "C"));
    press_dialog_button(&mut app, "插值");
    assert_eq!(
        app.project.book(1).unwrap().columns.len(),
        before.book(1).unwrap().columns.len() + 1
    );
    roundtrip(&mut app, &before);
}
#[test]
fn a6_anchor_signal_and_merge_real_dialogs_are_single_undo_steps() {
    for operation in 0..4 {
        let mut app = app();
        app.active = Some(1);
        let x: Vec<_> = (0..101).map(|i| i as f64 * 0.1).collect();
        let y: Vec<_> = x
            .iter()
            .map(|x| 0.2 + 3. * (-((x - 3.) / 0.5).powi(2)).exp() + 2. * (-((x - 7.) / 0.6).powi(2)).exp())
            .collect();
        for (key, values) in [("A", x), ("B", y), ("C", vec![0.25, 0.5, 0.25])] {
            app.project
                .book_mut(1)
                .unwrap()
                .columns
                .iter_mut()
                .find(|c| c.key == key)
                .unwrap()
                .data = values.iter().map(ToString::to_string).collect();
        }
        app.selection.insert(
            1,
            if operation == 0 || operation == 3 {
                vec!["B".into()]
            } else {
                vec!["B".into(), "C".into()]
            },
        );
        let before = app.project.clone();
        if operation == 0 {
            app.open_baseline_dialog();
            let Some(Dialog::Baseline {
                advanced,
                anchors,
                spline,
                ..
            }) = &mut app.dialog
            else {
                panic!()
            };
            *advanced = 1;
            *anchors = "0,5,10".into();
            *spline = true;
            press_dialog_button(&mut app, "扣除");
            assert_eq!(
                app.project.book(1).unwrap().columns.len(),
                before.book(1).unwrap().columns.len() + 2
            );
        } else if operation == 1 {
            app.open_signal_dialog();
            assert!(matches!(&app.dialog, Some(Dialog::Signal(_))));
            press_dialog_button(&mut app, "计算");
            assert_eq!(
                app.project.book(1).unwrap().columns.len(),
                before.book(1).unwrap().columns.len() + 2
            );
        } else if operation == 2 {
            app.open_average_dialog();
            let Some(Dialog::Average { merge, mean }) = &mut app.dialog else {
                panic!()
            };
            *merge = true;
            *mean = false;
            press_dialog_button(&mut app, "计算");
            assert_eq!(app.project.books.len(), before.books.len() + 1);
        } else {
            app.open_peaks_dialog();
            let Some(Dialog::Peaks { derivative, .. }) = &mut app.dialog else {
                panic!()
            };
            *derivative = true;
            press_dialog_button(&mut app, "寻峰");
            assert_eq!(app.project.books.len(), before.books.len() + 1, "{}", app.status);
            assert_eq!(app.project.books.last().unwrap().columns[0].data.len(), 2);
        }
        roundtrip(&mut app, &before);
    }
}

#[test]
fn b6_axis_scale_combo_real_window_undo() {
    use scicanvas_core::ticks::AxisScale;
    let mut app = app();
    app.active = Some(2);
    app.props_for = Some(2);
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.);
    let mut ui = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1600., 1400.)),
        |app: &mut SciApp, ctx: &egui::Context| app.props_window(ctx),
    );
    for _ in 0..5 {
        ui.frame(&mut app, vec![]);
    }
    let out = ui.frame(&mut app, vec![]);
    ui.click_text(&mut app, &out, "线性");
    let out = ui.frame(&mut app, vec![]);
    ui.click_text(&mut app, &out, "Log2");
    app.finish_edit();
    assert_eq!(
        app.project.graph(2).unwrap().first_layer().effective_x_scale(),
        AxisScale::Log2
    );
    roundtrip(&mut app, &before);
}

#[test]
fn a7_reports_real_write_compare_and_summary_undo() {
    let mut app = app();
    app.active = Some(1);
    app.selection.remove(&1);
    let before = app.project.clone();
    let undo = app.history.undo_label().map(str::to_owned);
    app.run_command("stats.whole");
    assert_eq!(app.project, before);
    assert_eq!(app.history.undo_label().map(str::to_owned), undo);
    assert!(matches!(&app.dialog,Some(Dialog::Table{title,..}) if title=="整表统计"));
    press_dialog_button(&mut app, "写入新表");
    assert_eq!(app.project.books.len(), before.books.len() + 1);
    roundtrip(&mut app, &before);
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    let before = app.project.clone();
    app.run_command("stats.compare");
    let Some(Dialog::StatisticsCompare { boxes, .. }) = &mut app.dialog else {
        panic!()
    };
    *boxes = true;
    press_dialog_button(&mut app, "生成");
    assert_eq!(app.project.books.len(), before.books.len() + 1);
    assert_eq!(app.project.graphs.len(), before.graphs.len() + 1);
    roundtrip(&mut app, &before);
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let before = app.project.clone();
    app.run_command("stats.summary");
    assert_eq!(app.project.graphs.len(), before.graphs.len() + 1);
    assert_eq!(app.project.notes.len(), before.notes.len() + 1);
    assert_eq!(app.project.graphs.last().unwrap().page.layers().len(), 3);
    roundtrip(&mut app, &before);
}

#[test]
fn b7_actual_shape_create_move_resize_delete_undo() {
    use scicanvas_core::page::ShapeKind;
    for (kind, command) in [
        (ShapeKind::Line, "graph.shape.line"),
        (ShapeKind::Arrow, "graph.shape.arrow"),
        (ShapeKind::Rect, "graph.shape.rect"),
        (ShapeKind::Ellipse, "graph.shape.ellipse"),
    ] {
        let mut app = app();
        app.active = Some(2);
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let before = app.project.clone();
        app.run_command(command);
        let page = object_frame(&mut app, &ctx, vec![], false);
        let at = |p: [f64; 2]| {
            egui::pos2(
                page.left() + page.width() * p[0] as f32,
                page.top() + page.height() * p[1] as f32,
            )
        };
        let a = at([0.15, 0.15]);
        let b = at([0.65, 0.45]);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(a)], false);
        object_frame(&mut app, &ctx, vec![object_button(a, true)], false);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(b)], false);
        object_frame(&mut app, &ctx, vec![object_button(b, false)], false);
        assert_eq!(
            app.project.graph(2).unwrap().page.shapes.len(),
            1,
            "{kind:?}: {}",
            app.status
        );
        assert_eq!(app.project.graph(2).unwrap().page.shapes[0].kind, kind);
        roundtrip(&mut app, &before);
        let shape = app.project.graph(2).unwrap().page.shapes[0];
        let center = at([(shape.a[0] + shape.b[0]) / 2., (shape.a[1] + shape.b[1]) / 2.]);
        let before = app.project.clone();
        let end = center + egui::vec2(page.width() * 0.04, page.height() * 0.03);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(center)], false);
        object_frame(&mut app, &ctx, vec![object_button(center, true)], false);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)], false);
        object_frame(&mut app, &ctx, vec![object_button(end, false)], false);
        let moved = app.project.graph(2).unwrap().page.shapes[0];
        assert!((moved.origin()[0] - shape.origin()[0] - 0.04).abs() < 1e-5, "{kind:?}");
        assert!((moved.origin()[1] - shape.origin()[1] - 0.03).abs() < 1e-5);
        roundtrip(&mut app, &before);
        let shape = app.project.graph(2).unwrap().page.shapes[0];
        let center = at([(shape.a[0] + shape.b[0]) / 2., (shape.a[1] + shape.b[1]) / 2.]);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(center)], false);
        object_frame(&mut app, &ctx, vec![object_button(center, true)], false);
        object_frame(&mut app, &ctx, vec![object_button(center, false)], false);
        let before = app.project.clone();
        let handle = at(shape.a);
        let end = handle + egui::vec2(-page.width() * 0.02, page.height() * 0.01);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(handle)], false);
        object_frame(&mut app, &ctx, vec![object_button(handle, true)], false);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)], false);
        object_frame(&mut app, &ctx, vec![object_button(end, false)], false);
        let resized = app.project.graph(2).unwrap().page.shapes[0];
        assert!((resized.a[0] - shape.a[0] + 0.02).abs() < 1e-5, "{kind:?}");
        assert!((resized.a[1] - shape.a[1] - 0.01).abs() < 1e-5);
        for i in 0..2 {
            assert!((resized.b[i] - shape.b[i]).abs() < 1e-12);
        }
        roundtrip(&mut app, &before);
        let shape = app.project.graph(2).unwrap().page.shapes[0];
        let center = at([(shape.a[0] + shape.b[0]) / 2., (shape.a[1] + shape.b[1]) / 2.]);
        object_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(center)], false);
        object_frame(&mut app, &ctx, vec![object_button(center, true)], false);
        object_frame(&mut app, &ctx, vec![object_button(center, false)], false);
        let before = app.project.clone();
        object_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            false,
        );
        assert!(app.project.graph(2).unwrap().page.shapes.is_empty());
        roundtrip(&mut app, &before);
    }
}

#[test]
fn b7_shape_fill_property_checkbox_undo() {
    use scicanvas_core::page::ShapeKind;
    let mut app = app();
    app.active = Some(2);
    let id = app
        .project
        .graph_mut(2)
        .unwrap()
        .page
        .add_shape(ShapeKind::Rect, [0.2, 0.2], [0.6, 0.6])
        .unwrap();
    app.apply_object_action(2, crate::plot::ObjectAction::Edit(crate::plot::ObjectTarget::Shape(id)));
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.);
    let mut ui = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1200., 900.)),
        |app: &mut SciApp, ctx: &egui::Context| app.object_window(ctx),
    );
    for _ in 0..4 {
        ui.frame(&mut app, vec![]);
    }
    let out = ui.frame(&mut app, vec![]);
    ui.click_text(&mut app, &out, "启用");
    app.finish_edit();
    assert!(app.project.graph(2).unwrap().page.shape(id).unwrap().fill.is_some());
    roundtrip(&mut app, &before);
}

#[test]
fn review_global_weight_checkbox_reports_missing_group_errors_immediately() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    app.run_command("fit.global");
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1400., 1000.)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    for _ in 0..5 {
        harness.frame(&mut app, vec![]);
    }
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "按各组关联的 Y 误差加权（1/σ²）");
    harness.frame(&mut app, vec![]);
    let response = ctx
        .data(|d| {
            d.get_temp::<(egui::Rect, bool)>(egui::Id::new(("weighted-control", "按各组关联的 Y 误差加权（1/σ²）")))
        })
        .unwrap();
    assert!(!response.1);
    assert!(matches!(&app.dialog,Some(Dialog::GlobalFit(edit)) if !edit.weighted));
    assert_eq!(app.project, before);
}

#[test]
fn review_pie_status_counts_nonfirst_pie_layer() {
    let mut app = app();
    let book = app.project.book_mut(1).unwrap();
    book.columns.iter_mut().find(|c| c.key == "B").unwrap().data =
        ["1", "-2", "3", "0", ""].map(str::to_owned).to_vec().into();
    let g = app.project.graph_mut(2).unwrap();
    let layer = g.first_layer().clone();
    g.page.add_layer(layer);
    app.active = Some(2);
    app.active_layers.insert(2, 1);
    app.run_command("plot.pie");
    assert_ne!(app.project.graph(2).unwrap().first_layer().plot_type, PlotType::Pie);
    assert_eq!(app.project.graph(2).unwrap().page.layers()[1].plot_type, PlotType::Pie);
    assert!(app.status.contains("跳过 3 行"), "{}", app.status);
}

#[test]
fn u2_invalid_fields_disable_actual_primary_widgets() {
    for case in 0..3 {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
        let (command, label) = match case {
            0 => ("signal.baseline", "扣除"),
            1 => ("fit.calibration", "计算"),
            _ => ("math.interp", "插值"),
        };
        app.run_command(command);
        match app.dialog.as_mut().unwrap() {
            Dialog::Baseline { advanced, anchors, .. } => {
                *advanced = 1;
                *anchors = "bad".into();
            }
            Dialog::Calibration { keys, .. } => keys[1] = keys[0].clone(),
            Dialog::Interpolate { custom, from, to, .. } => {
                *custom = true;
                *from = 2.;
                *to = 1.;
            }
            _ => unreachable!(),
        }
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(1280., 820.)),
            |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
        )
        .with_step(0.02);
        for _ in 0..6 {
            harness.frame(&mut app, vec![]);
        }
        let (rect, info) = ctx
            .data(|d| d.get_temp::<(egui::Rect, egui::WidgetInfo)>(egui::Id::new(("dialog-primary", label))))
            .unwrap();
        assert!(!info.enabled, "case={case}");
        harness.click(&mut app, rect.center());
        harness.key(egui::Key::Enter, |ctx| app.dialogs(ctx));
        assert_eq!(app.project, before);
        assert!(app.dialog.is_some());
    }
}

#[test]
fn u2_failed_fft_retains_parameters_and_displays_error() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.project
        .book_mut(1)
        .unwrap()
        .columns
        .iter_mut()
        .find(|c| c.key == "B")
        .unwrap()
        .data = vec!["1".into()].into();
    app.run_command("signal.fft");
    if let Some(Dialog::Fft { window }) = &mut app.dialog {
        *window = scicanvas_core::fft::Window::Hamming;
    }
    let before = app.project.clone();
    press_dialog_button(&mut app, "变换");
    assert!(matches!(
        app.dialog,
        Some(Dialog::Fft {
            window: scicanvas_core::fft::Window::Hamming
        })
    ));
    assert_eq!(app.project, before);
    let error = app.dialog_error.clone().expect("inline execution error");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
    );
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    let out = h.frame(&mut app, vec![]);
    assert!(crate::test_ui::find_text(&out, &error).is_some(), "{error}");
}

#[test]
fn u2_enter_submits_poly_smooth_and_interpolation_once() {
    for command in ["fit.poly", "math.smooth", "math.interp"] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.run_command(command);
        let before = app.project.clone();
        let generation = app.history.generation();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut h = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(1280., 820.)),
            |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
        );
        for _ in 0..5 {
            h.frame(&mut app, vec![]);
        }
        let at = ctx
            .data(|d| {
                d.get_temp::<(egui::Rect, egui::WidgetInfo)>(egui::Id::new((
                    "dialog-primary",
                    if command == "fit.poly" {
                        "拟合"
                    } else if command == "math.smooth" {
                        "平滑"
                    } else {
                        "插值"
                    },
                )))
            })
            .unwrap()
            .0
            .center();
        h.frame(&mut app, vec![egui::Event::PointerMoved(at)]);
        h.key(egui::Key::Enter, |ctx| app.dialogs(ctx));
        assert_ne!(app.project, before, "{command}: {}", app.status);
        assert_eq!(app.history.generation(), generation + 1, "{command}");
        let after = app.project.clone();
        for _ in 0..2 {
            h.frame(&mut app, vec![]);
        }
        assert_eq!(app.project, after);
        roundtrip(&mut app, &before);
    }
}

#[test]
fn u2_global_constraint_enter_inserts_newline() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    app.run_command("fit.global");
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
    )
    .with_step(0.02);
    for _ in 0..6 {
        h.frame(&mut app, vec![]);
    }
    let rect = ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("dialog-multiline-rect")))
        .unwrap();
    h.click(&mut app, rect.center());
    h.key(egui::Key::Enter, |ctx| app.dialogs(ctx));
    assert_eq!(app.project, before);
    assert!(matches!(&app.dialog,Some(Dialog::GlobalFit(edit)) if edit.constraints.contains('\n')));
}

#[test]
fn u2_control_enter_submits_multiline_formula() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("col.setvalues");
    if let Some(Dialog::SetValues { formula, .. }) = &mut app.dialog {
        *formula = "A*2".into();
    } else {
        panic!("formula dialog");
    }
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
    );
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    let out = h.frame(&mut app, vec![]);
    let at = crate::test_ui::text_rect(&out, "A*2").unwrap().center();
    h.click(&mut app, at);
    h.frame(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }],
    );
    assert!(app.dialog.is_none());
    assert_ne!(app.project, before);
    roundtrip(&mut app, &before);
}

#[test]
fn u2_parameter_window_centers_once_and_remembers_drag() {
    let mut app = app();
    app.active = Some(1);
    app.run_command("fit.poly");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.);
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
    )
    .with_step(0.02);
    for _ in 0..6 {
        h.frame(&mut app, vec![]);
    }
    let id = egui::Id::new(("scicanvas-dialog", "多项式拟合"));
    let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
    assert!(rect.center().distance(egui::pos2(640., 410.)) < 4., "{rect:?}");
    let at = egui::pos2(rect.center().x, rect.top() + 12.);
    let end = at + egui::vec2(100., 40.);
    h.drag(at, at + egui::vec2(20., 0.), end, |ctx| app.dialogs(ctx));
    let moved = ctx.memory(|m| m.area_rect(id)).unwrap();
    for _ in 0..3 {
        h.frame(&mut app, vec![]);
    }
    assert_eq!(ctx.memory(|m| m.area_rect(id)).unwrap().min, moved.min);
    assert!(moved.center().distance(rect.center()) > 40.);
}

#[test]
fn r16_sheet_enter_and_escape_do_not_trigger_parameter_window() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for key in [egui::Key::Enter, egui::Key::Escape] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            ctx.set_theme(theme);
            let mut app = app();
            app.active = Some(1);
            app.selection.insert(1, vec!["C".into()]);
            app.run_command("fit.poly");
            let mut h = crate::test_ui::Harness::new(
                ctx.clone(),
                Some(egui::vec2(1280., 820.)),
                |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
            );
            for _ in 0..6 {
                h.frame(&mut app, vec![]);
            }
            let before = app.project.clone();
            app.sheets.entry(1).or_default().edit = Some(Edit {
                cell: Cell { c: 0, r: 0 },
                buf: "123".into(),
                focused: false,
            });
            for _ in 0..2 {
                h.frame(&mut app, vec![]);
            }
            let focus = ctx.memory(|m| m.focused()).expect("worksheet editor has focus");
            assert!(ctx.read_response(focus).is_some());
            let out = h.frame(
                &mut app,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(
                matches!(app.dialog, Some(Dialog::Poly { .. })),
                "{key:?}: {}",
                app.status
            );
            assert!(app.sheets[&1].edit.is_none());
            let mut expected = before;
            if key == egui::Key::Enter {
                expected.book_mut(1).unwrap().columns[0].set(0, "123".into());
            }
            assert_eq!(app.project, expected);
            assert!(out
                .shapes
                .iter()
                .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text=="多项式拟合")));
        }
    }
}

#[test]
fn r16_whole_statistics_keep_method_in_note_and_sheet_errors_outside_dialog() {
    let mut app = app();
    app.active = Some(1);
    app.whole_table_statistics();
    let Some(Dialog::Table { rows, note, .. }) = &app.dialog else {
        panic!("statistics result");
    };
    assert!(!rows.iter().flatten().any(|s| s == "说明"));
    assert!(note.contains("n−1"));
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("fit.poly");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    let before = app.project.clone();
    assert!(!app.set_column_values("unknown(", false));
    let error = app.status.to_string();
    assert!(app.dialog_error.is_none());
    h.frame(&mut app, vec![]);
    assert!(matches!(app.dialog, Some(Dialog::Poly { .. })));
    assert_eq!(app.project, before);
    assert!(app.dialog_error.is_none());
    assert!(!dialog_rendered_texts(&mut app).iter().any(|s| s == &error));
    app.record_dialog_error("old dialog error".into());
    app.dialog = Some(Dialog::Smooth {
        method: scicanvas_core::analysis::SmoothMethod::MovingAverage,
        window: 5,
    });
    h.frame(&mut app, vec![]);
    assert!(app.dialog_error.is_none());
}

#[test]
fn u6_every_advertised_shortcut_dispatches_from_real_frame_events() {
    for command in commands::commands().iter().filter(|c| !c.keys.is_empty()) {
        let bindings = commands::shortcut_bindings(command);
        assert!(!bindings.is_empty(), "{}: {}", command.id, command.keys);
        for (modifiers, key) in bindings {
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            let mut app = app();
            app.active = Some(if command.id.starts_with("graph.") { 2 } else { 1 });
            let mut h = crate::test_ui::Harness::new(
                ctx.clone(),
                Some(egui::vec2(1280., 820.)),
                |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
            );
            app.selection.insert(1, vec!["B".into()]);
            app.sheets.entry(1).or_default().range = Some(Range::single(1, 0));
            if command.id == "graph.style.paste" {
                app.run_command("graph.style.copy");
            }
            h.frame(&mut app, vec![]);
            if app.active == Some(1) {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new(("sheet-focus", 1_u64))));
            }
            let enabled = (command.when)(&app).is_ok();
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("shortcut-probe"), true));
            h.modifiers = modifiers;
            let event = if key == egui::Key::C && modifiers.command {
                egui::Event::Copy
            } else if key == egui::Key::V && modifiers.command {
                egui::Event::Paste("test".into())
            } else {
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }
            };
            h.frame(&mut app, vec![event]);
            let dispatched = ctx
                .data(|d| d.get_temp::<Vec<&'static str>>(egui::Id::new("shortcut-dispatched")))
                .unwrap_or_default();
            assert_eq!(
                dispatched,
                if enabled { vec![command.id] } else { vec![] },
                "{} / {}",
                command.id,
                command.keys
            );
        }
    }
}

#[test]
fn u6_native_shift_clipboard_changes_graph_format_but_never_pastes_sheet_text() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(2);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..3 {
        h.frame(&mut app, vec![]);
    }
    h.modifiers = egui::Modifiers {
        shift: true,
        ..egui::Modifiers::COMMAND
    };
    h.frame(&mut app, vec![egui::Event::Copy]);
    assert!(app.style_clipboard.is_some());
    let width = app.project.graph(2).unwrap().first_layer().line_width;
    app.project.graph_mut(2).unwrap().first_layer_mut().line_width = 4.;
    h.frame(&mut app, vec![egui::Event::Paste("unrelated clipboard text".into())]);
    assert_eq!(app.project.graph(2).unwrap().first_layer().line_width, width);
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.sheets.entry(1).or_default().edit = Some(Edit {
        cell: Cell { c: 0, r: 0 },
        buf: "42".into(),
        focused: false,
    });
    h.modifiers = egui::Modifiers::NONE;
    for _ in 0..2 {
        h.frame(&mut app, vec![]);
    }
    let before = app.project.clone();
    h.modifiers = egui::Modifiers {
        shift: true,
        ..egui::Modifiers::COMMAND
    };
    h.frame(&mut app, vec![egui::Event::Paste("999".into())]);
    assert_eq!(app.project, before);
    assert_eq!(app.sheets[&1].edit.as_ref().unwrap().buf, "42");
    h.modifiers = egui::Modifiers::COMMAND;
    h.frame(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    assert!(matches!(app.dialog, Some(Dialog::SetValues { .. })));
}

#[test]
fn u6_palette_wheel_scroll_is_retained_and_search_ignores_case() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.ui.open_palette();
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    let (_, rect) = ctx
        .data(|d| d.get_temp::<(f32, egui::Rect)>(egui::Id::new("palette-scroll")))
        .unwrap();
    h.frame(&mut app, vec![egui::Event::PointerMoved(rect.center())]);
    for _ in 0..4 {
        h.frame(
            &mut app,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -240.),
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    assert!(
        ctx.data(|d| d.get_temp::<(f32, egui::Rect)>(egui::Id::new("palette-scroll")))
            .unwrap()
            .0
            > 100.
    );
    for (query, label) in [("voigt", "多峰拟合…"), ("nature", "图形主题与选项…")] {
        app.ui.palette.as_mut().unwrap().query = query.into();
        let out = h.frame(&mut app, vec![]);
        assert!(crate::test_ui::find_text(&out, label).is_some(), "{query}");
    }
}

#[test]
fn u6_insertion_and_menu_states_follow_current_selection() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into(), "A".into()]);
    let before = app.project.clone();
    app.run_command("col.insert");
    assert_eq!(app.project.book(1).unwrap().columns[3].key, "D");
    roundtrip(&mut app, &before);
    app.selection.insert(1, vec!["A".into()]);
    let state = crate::menus::MenuState::new(&app);
    assert_eq!(state.0["col.setX"].1, Some(true));
    assert!(state.0["col.type.num"].0.is_err());
    assert!((commands::find("sheet.unfreeze").unwrap().when)(&app).is_err());
    app.active = Some(2);
    assert!((commands::find("graph.style.paste").unwrap().when)(&app).is_err());
}

#[test]
fn h1_nonmodal_parameters_leave_sheet_navigation_edit_copy_and_delete_working() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.run_command("fit.poly");
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..6 {
        h.frame(&mut app, vec![]);
    }
    let cell = ctx
        .read_response(egui::Id::new(("cell", 1_u64, 0_usize, 0_usize)))
        .unwrap()
        .rect
        .center();
    h.click(&mut app, cell);
    h.key(egui::Key::ArrowDown, |c| app.ui_frame(c));
    assert_eq!(app.sheets[&1].range.unwrap().a.r, 1);
    h.frame(&mut app, vec![egui::Event::Text("123".into())]);
    assert!(app.sheets[&1].edit.is_some());
    h.frame(&mut app, vec![]);
    h.key(egui::Key::Enter, |c| app.ui_frame(c));
    let out = h.frame(&mut app, vec![egui::Event::Copy]);
    assert!(!out.platform_output.copied_text.is_empty());
    h.key(egui::Key::Delete, |c| app.ui_frame(c));
    assert!(matches!(app.dialog, Some(Dialog::Poly { .. })));
    assert_eq!(app.project.book(1).unwrap().columns.len(), 3);
    assert_eq!(app.project.graphs.len(), 1);
}

#[test]
fn h3_statistical_buttons_count_columns_and_graph_curves() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec![]);
    for id in [
        "stats.normality",
        "stats.dixon",
        "stats.grubbs",
        "stats.ks2",
        "stats.anova",
        "stats.correlation",
        "fit.mlr",
        "stats.summary",
    ] {
        let err = (commands::find(id).unwrap().when)(&app).unwrap_err();
        assert!(err.contains("列"), "{id}: {err}");
    }
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    for id in ["stats.ks2", "stats.anova", "stats.correlation", "fit.mlr"] {
        assert!((commands::find(id).unwrap().when)(&app).is_ok(), "{id}");
    }
    app.active = Some(2);
    assert_eq!(app.statistical_column_count(), app.statistical_columns().unwrap().len());
    assert!((commands::find("stats.ks2").unwrap().when)(&app).is_ok());
}

#[test]
fn h4_reports_copy_with_feedback_and_write_one_undoable_workbook() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    app.anova();
    press_dialog_button(&mut app, "复制结果");
    assert!(app.status.contains("已复制") && app.status.contains("Excel"));
    let before = app.project.clone();
    press_dialog_button(&mut app, "写入新表");
    assert_eq!(app.project.books.len(), before.books.len() + 1);
    roundtrip(&mut app, &before);
}

#[test]
fn h6_new_custom_fit_keeps_limits_and_positional_error_weighting() {
    use scicanvas_core::nlfit::{CustomFitDefinition, FitFunction, ParamLimits};
    use scicanvas_core::ops::FitWeight;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let n = app.project.book(1).unwrap().nrows();
    {
        let error = &mut app.project.book_mut(1).unwrap().columns[2];
        error.role = Role::YError;
        error.error_for = None;
        error.data = (0..n)
            .map(|row| (0.1 + row as f64 * 0.02).to_string())
            .collect::<Vec<_>>()
            .into();
    }
    let definition = CustomFitDefinition::from_text("linear", "a*x+b", "a,b", "1,0", "").unwrap();
    let mut edit = dialogs::CustomFitEditor::from_definition(&definition);
    let limits = vec![
        ParamLimits {
            lower: Some(0.),
            ..Default::default()
        },
        ParamLimits::default(),
    ];
    edit.limits = limits.clone();
    edit.weighted = true;
    let mut expected = app.project.clone();
    let (function, constraints) = definition.resolve().unwrap();
    expected
        .nlfit_weighted(
            &Source::Book {
                id: 1,
                selected: vec!["B".into()],
                rows: None,
            },
            FitFunction::Custom(function),
            Some(&definition.initial),
            &limits,
            &constraints,
            FitWeight::YError,
        )
        .unwrap();
    app.dialog = Some(Dialog::CustomNlFit(edit));
    let before = app.project.clone();
    press_dialog_button(&mut app, "拟合");
    assert_eq!(app.project, expected);
    roundtrip(&mut app, &before);
}

#[test]
fn h10_find_opens_with_a_selected_keyboard_focus() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    app.run_command("edit.find");
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    assert!(ctx.memory(|m| m.focused()).is_some());
    h.frame(&mut app, vec![egui::Event::Text("needle".into())]);
    assert!(matches!(&app.dialog,Some(Dialog::FindReplace(edit)) if edit.needle=="needle"));
}

#[test]
fn p1_graph_delete_and_copy_leave_sheet_shortcuts_alone() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(2);
    let text = app.project.graph_mut(2).unwrap().page.add_text([0.3, 0.3]).unwrap();
    app.project.graph_mut(2).unwrap().page.text_mut(text).unwrap().text = "Delete this annotation".into();
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    let out = h.frame(&mut app, vec![]);
    h.click_text(&mut app, &out, "Delete this annotation");
    let before = app.project.clone();
    h.key(egui::Key::Delete, |c| app.ui_frame(c));
    assert!(
        app.project.graph(2).unwrap().page.text(text).is_none(),
        "{}",
        app.status
    );
    assert_ne!(app.status.kind, StatusKind::Error);
    h.frame(&mut app, vec![egui::Event::Copy]);
    assert_ne!(app.status.kind, StatusKind::Error);
    roundtrip(&mut app, &before);
}

#[test]
fn p1_explorer_paste_does_not_modify_sheet() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    app.sheets.entry(1).or_default().range = Some(Range::single(1, 0));
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..4 {
        h.frame(&mut app, vec![]);
    }
    app.explorer_focus = true;
    let before = app.project.clone();
    h.frame(&mut app, vec![egui::Event::Paste("999\n888".into())]);
    assert_eq!(app.project, before);
    assert_ne!(app.status.kind, StatusKind::Error);
}

#[test]
fn p2_header_paste_and_menu_queue_share_anchor_and_cleanup() {
    let mut projects = Vec::new();
    for menu in [false, true] {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.active = Some(1);
        let mut h = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(1280., 820.)),
            |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
        );
        for _ in 0..4 {
            h.frame(&mut app, vec![]);
        }
        app.selection.insert(1, vec!["B".into()]);
        app.sheets.entry(1).or_default().range = None;
        assert!(app.paste_available().is_ok());
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(("sheet-focus", 1_u64))));
        if menu {
            // Menu command queues its clipboard payload for the same frame handler.
            app.pending_paste = Some("123\n456".into());
            h.frame(&mut app, vec![]);
        } else {
            h.frame(&mut app, vec![egui::Event::Paste("123\n456".into())]);
        }
        let values = app.project.book(1).unwrap().column("B").unwrap().values();
        assert_eq!(&values[..2], &[123., 456.]);
        assert!(app.selected_keys(1).is_empty());
        assert_eq!(ctx.memory(|m| m.focused()), Some(egui::Id::new(("sheet-focus", 1_u64))));
        projects.push(app.project);
    }
    assert_eq!(projects[0], projects[1]);
}

#[test]
fn p8_idle_books_do_not_compute_context_menu_states() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..4 {
        h.frame(&mut app, vec![]);
    }
    assert_eq!(
        ctx.data(|d| d.get_temp::<bool>(egui::Id::new(("sheet-menu-built", 1_u64)))),
        Some(false)
    );
    let at = ctx
        .read_response(egui::Id::new(("cell", 1_u64, 0_usize, 0_usize)))
        .unwrap()
        .rect
        .center();
    for pressed in [true, false] {
        h.frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    h.frame(&mut app, vec![]);
    assert_eq!(
        ctx.data(|d| d.get_temp::<bool>(egui::Id::new(("sheet-menu-built", 1_u64)))),
        Some(true)
    );
}

#[test]
fn d4_wheel_view_becomes_one_undoable_axis_edit() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(2);
    // Use the real graph widget and wheel event, then its shared menu action.
    let page = object_frame(&mut app, &ctx, vec![], false);
    let scene = scicanvas_core::scene::layout_page(app.project.graph(2).unwrap(), &app.project, &[]);
    let [x, y, w, h] = scene.plot;
    let scale = page.width() / scene.w as f32;
    let at = page.min + egui::vec2((x + w / 2.) as f32 * scale, (y + h / 2.) as f32 * scale);
    object_frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., 120.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
        false,
    );
    let view = plot::current_view(&ctx, 2, 0).expect("wheel creates a zoomed view");
    let before = app.project.clone();
    app.commit_view_range(&ctx, 2, 0);
    let layer = app.project.graph(2).unwrap().first_layer();
    assert_eq!((layer.x_min, layer.x_max), view.x);
    assert_eq!((layer.y_min, layer.y_max), view.y);
    assert!(!layer.x_auto && !layer.y_auto);
    roundtrip(&mut app, &before);
}

#[test]
fn p13_settings_stay_above_documents_below_menus_and_modal() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("fit.poly");
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..6 {
        h.frame(&mut app, vec![]);
    }
    let id = egui::Id::new(("scicanvas-dialog", "多项式拟合"));
    let settings = egui::LayerId::new(egui::Order::Middle, id);
    let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
    let cell = ctx
        .read_response(egui::Id::new(("cell", 1_u64, 0_usize, 0_usize)))
        .unwrap()
        .rect
        .center();
    h.click(&mut app, cell);
    h.frame(&mut app, vec![]);
    let overlap = rect
        .intersect(app.geoms[&1].rect.translate(app.ws.min.to_vec2()))
        .shrink(8.);
    assert!(overlap.is_positive(), "{rect:?} / {:?}", app.geoms[&1].rect);
    assert_eq!(ctx.layer_id_at(overlap.center()), Some(settings));
    for pressed in [true, false] {
        h.frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(cell),
                egui::Event::PointerButton {
                    pos: cell,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let out = h.frame(&mut app, vec![]);
    assert!(out.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "复制"
            && ctx.layer_id_at(text.pos + egui::vec2(3., 3.)).is_some_and(|layer| layer.order == egui::Order::Foreground))
    }), "context menu must be above Middle settings");
    h.key(egui::Key::Escape, |c| app.ui_frame(c));
    app.edit_grouped("改项目名", None, |project| {
        project.books[0].name.push_str(" modified")
    });
    app.request_discard(PendingDiscard::Quit);
    h.frame(&mut app, vec![]);
    h.frame(&mut app, vec![]);
    assert_eq!(
        ctx.layer_id_at(overlap.center()).unwrap().order,
        egui::Order::Foreground
    );
    let (_, modal) = ctx
        .data(|d| d.get_temp::<(u64, egui::LayerId)>(egui::Id::new("keyboard-modal-layer")))
        .expect("unsaved modal");
    assert_eq!(modal.order, egui::Order::Foreground);
}
