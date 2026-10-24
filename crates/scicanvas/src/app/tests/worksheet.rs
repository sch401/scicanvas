use super::*;

#[test]
fn column_preview_updates_sheet_and_plot_reuses_data_and_commits_to_original_book() {
    use scicanvas_core::format::NumberFormat;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let other = app.project.new_book();
    app.project.book_mut(1).unwrap().columns[1].set(0, "12.3456".into());
    let before = app.project.clone();
    app.run_command("col.props");
    let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
        panic!()
    };
    edit.long_name = "Preview voltage".into();
    edit.units = "preview_unit".into();
    edit.format.number = NumberFormat::from_index(1, 2);
    assert!(app.refresh_book_preview());
    let allocation = |app: &SciApp| {
        let scicanvas_core::ColumnData::Num { values, .. } = &app
            .book_preview
            .as_ref()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .column("B")
            .unwrap()
            .data
        else {
            panic!()
        };
        values.as_ptr()
    };
    let ptr = allocation(&app);
    assert!(!app.refresh_book_preview());
    let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
        panic!()
    };
    edit.long_name.push_str(" live");
    assert!(app.refresh_book_preview());
    assert_eq!(allocation(&app), ptr, "typing and display formats reuse preview data");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let has_text = |out: &egui::FullOutput, text: &str| {
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text.contains(text)))
    };
    let mut sheet = egui::FullOutput::default();
    let mut graph = egui::FullOutput::default();
    let gid = app.project.graphs[0].id;
    for _ in 0..3 {
        sheet = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
        });
        graph = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.graph_ui(ui, gid));
        });
    }
    assert!(has_text(&sheet, "Preview voltage live"));
    assert!(has_text(&sheet, "12.35"));
    assert!(has_text(&graph, "Preview voltage live"));
    assert!(has_text(&graph, "preview_unit"));
    assert_eq!(app.project, before, "read-only plotting restores the canonical table");
    app.active = Some(other);
    press_dialog_button(&mut app, "×");
    assert!(app.dialog.is_none());
    assert!(app.book_preview.is_none());
    assert_eq!(
        app.project.book(1).unwrap().column("B").unwrap().long_name,
        "Preview voltage live"
    );
    assert_eq!(app.project.book(other), before.book(other));
    roundtrip(&mut app, &before);
    app.active = Some(1);
    app.run_command("col.props");
    let committed = app.project.clone();
    let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
        panic!()
    };
    edit.long_name = "kept after close".into();
    app.refresh_book_preview();
    press_dialog_button(&mut app, "×");
    assert!(app.book_preview.is_none());
    assert_ne!(app.project, committed);
    roundtrip(&mut app, &committed);
}

#[test]
fn typed_column_category_and_sort_previews_validate_and_remain_undoable() {
    use scicanvas_core::DataKind;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into()]);
    app.project.book_mut(1).unwrap().columns[0].data = vec!["b".into(), "a".into(), "c".into()].into();
    app.run_command("col.props");
    let before = app.project.clone();
    let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
        panic!()
    };
    edit.kind = DataKind::Num;
    app.refresh_book_preview();
    assert!(app.book_preview.as_ref().unwrap().result.is_err());
    assert_eq!(app.project, before);
    let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
        panic!()
    };
    edit.kind = DataKind::Category;
    app.refresh_book_preview();
    assert_eq!(
        app.book_preview.as_ref().unwrap().result.as_ref().unwrap().columns[0]
            .data
            .kind(),
        DataKind::Category
    );
    press_dialog_button(&mut app, "×");
    roundtrip(&mut app, &before);
    let before = app.project.clone();
    app.run_command("col.categories");
    let Some(Dialog::CategoryOrder { levels, .. }) = &mut app.dialog else {
        panic!()
    };
    levels.reverse();
    let levels = levels.clone();
    app.refresh_book_preview();
    let book = app.book_preview.as_ref().unwrap().result.as_ref().unwrap();
    assert_eq!(book.columns[0].data.category_levels().unwrap(), levels);
    assert_eq!(
        book.columns[0].data.text_at(0),
        "b",
        "reordering categories preserves cell labels"
    );
    assert_eq!(app.project, before);
    press_dialog_button(&mut app, "×");
    roundtrip(&mut app, &before);
    app.run_command("sort.custom");
    app.refresh_book_preview();
    let sorted = app.book_preview.as_ref().unwrap().result.as_ref().unwrap().clone();
    let before = app.project.clone();
    assert_ne!(&sorted, before.book(1).unwrap());
    press_dialog_button(&mut app, "取消");
    assert_eq!(app.project, before);
    app.run_command("sort.custom");
    let Some(Dialog::Sort { keys, .. }) = &mut app.dialog else {
        panic!()
    };
    *keys = [(None, true); 3];
    app.refresh_book_preview();
    assert!(app.book_preview.as_ref().unwrap().result.is_err());
    let Some(Dialog::Sort { keys, .. }) = &mut app.dialog else {
        panic!()
    };
    keys[0] = (Some(0), true);
    app.refresh_book_preview();
    assert!(app.book_preview.as_ref().unwrap().result.is_ok());
    press_dialog_button(&mut app, "排序");
    assert_eq!(app.project.book(1), Some(&sorted));
    roundtrip(&mut app, &before);
}

#[test]
fn manual_analysis_results_update_only_on_recalc_command() {
    use scicanvas_core::ops::RecalcMode;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.begin_edit("平滑");
    app.smooth(scicanvas_core::analysis::SmoothMethod::MovingAverage, 3);
    app.history.changed();
    app.finish_edit();
    let key = app
        .project
        .book(1)
        .unwrap()
        .columns
        .iter()
        .find(|c| c.op.is_some())
        .unwrap()
        .key
        .clone();
    // 列属性改为手动
    app.selection.insert(1, vec![key.clone()]);
    app.run_command("col.props");
    let Some(Dialog::ColumnProps { edit, .. }) = app.dialog.take() else {
        panic!()
    };
    assert_eq!(edit.recalc, Some(RecalcMode::Auto));
    let mut edit = edit;
    edit.recalc = Some(RecalcMode::Manual);
    app.begin_edit("列属性");
    app.apply_column_props(1, edit).unwrap();
    app.history.changed();
    app.finish_edit();
    let smoothed = |app: &SciApp| app.project.book(1).unwrap().column(&key).unwrap().values();
    let old = smoothed(&app);
    app.begin_edit("编辑单元格");
    app.project.book_mut(1).unwrap().columns[1].set(1, "99".into());
    app.history.changed();
    app.finish_edit();
    assert_eq!(smoothed(&app), old);
    let before = app.project.clone();
    app.run_command("data.recalc");
    assert_ne!(smoothed(&app), old);
    roundtrip(&mut app, &before);
}

#[test]
fn saved_formula_recalculates_when_a_source_cell_changes_in_the_same_undo_step() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.begin_edit("设置列值");
    assert!(app.set_column_values("B * 2", true));
    app.history.changed();
    app.finish_edit();
    app.open_set_values_dialog();
    assert!(matches!(&app.dialog, Some(Dialog::SetValues { formula, keep: true, .. }) if formula == "B * 2"));
    app.dialog = None;
    let before = app.project.clone();
    // 改 B 列一个单元格：提交编辑时 C 跟着重算，撤销时一起回退
    app.begin_edit("编辑单元格");
    app.project.book_mut(1).unwrap().columns[1].set(0, "100".into());
    app.history.changed();
    app.finish_edit();
    assert_eq!(app.project.book(1).unwrap().column("C").unwrap().values()[0], 200.0);
    app.undo();
    assert_eq!(app.project, before);
}

#[test]
fn set_values_keeps_the_dialog_open_on_errors_and_undoes_in_one_step() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.open_set_values_dialog();
    assert!(matches!(&app.dialog, Some(Dialog::SetValues { target, .. }) if target.starts_with('C')));
    assert!(!app.set_column_values("B *", false));
    assert!(matches!(&app.dialog, Some(Dialog::SetValues { error: Some(_), .. })));
    let before = app.project.book(1).unwrap().columns[2].data.clone();
    app.begin_edit("设置列值");
    assert!(app.set_column_values("B * 2", false));
    app.history.changed();
    app.finish_edit();
    let book = app.project.book(1).unwrap();
    let b = book.column("B").unwrap().values();
    let c = book.column("C").unwrap().values();
    assert!(b
        .iter()
        .zip(&c)
        .all(|(b, c)| (b * 2.0 - c).abs() < 1e-9 * b.abs().max(1.0)));
    app.undo();
    assert_eq!(app.project.book(1).unwrap().columns[2].data, before);
}

#[test]
fn row_commands_follow_the_selection_and_undo_as_one_step() {
    let mut app = app();
    app.active = Some(1);
    let before = app.project.clone();
    let rows = |app: &SciApp| {
        app.project.book(1).unwrap().columns[0]
            .data
            .iter()
            .map(std::borrow::Cow::into_owned)
            .collect::<Vec<_>>()
    };
    let original = rows(&app);
    let select = |app: &mut SciApp| {
        app.sheets.insert(
            1,
            SheetState {
                range: Some(Range {
                    a: Cell { c: 1, r: 2 },
                    b: Cell { c: 2, r: 4 },
                }),
                ..Default::default()
            },
        );
    };

    select(&mut app);
    app.run_command("row.insert");
    assert_eq!(rows(&app).len(), original.len() + 3);
    assert_eq!(rows(&app)[2..5], [String::new(), String::new(), String::new()]);
    assert_eq!(app.history.undo_label(), Some("插入行"));
    roundtrip(&mut app, &before);
    app.run_command("edit.undo");

    select(&mut app);
    app.run_command("row.delete");
    assert_eq!(
        rows(&app)[2],
        original[5],
        "whole rows go, whatever columns were selected"
    );
    assert_eq!(app.sheets[&1].range, Some(Range::single(1, 2)));
    roundtrip(&mut app, &before);
}

#[test]
fn paste_cut_and_clear_use_project_history() {
    let ctx = egui::Context::default();
    for event in [
        egui::Event::Paste("1\t2\n3\t4".into()),
        egui::Event::Cut,
        egui::Event::Key {
            key: egui::Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
    ] {
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        let mut sheet = SheetState {
            range: Some(Range::single(0, 0)),
            ..Default::default()
        };
        let input = egui::RawInput {
            events: vec![event],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| app.sheet_keys(ctx, 1, &mut sheet));
        app.finish_edit();
        roundtrip(&mut app, &before);
    }
}

#[test]
fn cell_enter_commits_one_undoable_edit() {
    let ctx = egui::Context::default();
    let mut app = app();
    app.active = Some(1);
    let before = app.project.clone();
    app.sheets.insert(
        1,
        SheetState {
            range: Some(Range::single(0, 0)),
            edit: Some(Edit {
                cell: Cell { c: 0, r: 0 },
                buf: "123".into(),
                focused: false,
            }),
            ..Default::default()
        },
    );
    for events in [
        vec![],
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
            },
        );
        app.finish_edit();
    }
    assert_eq!(app.project.books[0].columns[0].data.text_at(0), "123");
    roundtrip(&mut app, &before);
}

#[test]
fn actual_header_and_property_text_edits_form_one_transaction_each() {
    for props in [false, true] {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        let mut p = Project::sample();
        p.graphs[0].first_layer_mut().title = "原图".into();
        app.load_project(p, None);
        app.active = Some(1);
        if props {
            app.props_for = Some(2);
        }
        let before = app.project.clone();
        editor_frame(&mut app, &ctx, vec![], props);
        let output = editor_frame(&mut app, &ctx, vec![], props);
        assert!(!app.history.is_pending());
        let text = if props { "原图" } else { "时间" };
        let at = crate::test_ui::text_rect(&output, text)
            .map(|rect| rect.center())
            .unwrap_or_else(|| {
                panic!(
                    "未找到 {text}，当前文字：{:?}",
                    output
                        .shapes
                        .iter()
                        .filter_map(|s| match &s.shape {
                            egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                )
            });
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        editor_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at), button(true)], props);
        editor_frame(&mut app, &ctx, vec![button(false)], props);
        editor_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            props,
        );
        for text in ["a", "b", "c"] {
            editor_frame(&mut app, &ctx, vec![egui::Event::Text(text.into())], props);
        }
        assert!(app.history.is_pending());
        if props {
            assert_eq!(app.project.graphs[0].first_layer().title, "abc");
        } else {
            assert_eq!(app.project.books[0].columns[0].long_name, "abc");
        }
        roundtrip(&mut app, &before);
        app.run_command("edit.undo");
        assert!(app.history.undo_label().is_none());
    }
}

#[test]
fn column_properties_apply_in_one_step_and_failed_conversions_change_nothing() {
    use scicanvas_core::format::{DisplayFormat, NumberFormat};
    use scicanvas_core::DataKind;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("col.props");
    let Some(Dialog::ColumnProps { edit, sample, .. }) = &app.dialog else {
        panic!("column properties should open")
    };
    let col = app.project.book(1).unwrap().column("B").unwrap();
    assert_eq!(
        (edit.key.as_str(), edit.long_name.as_str(), edit.kind),
        ("B", col.long_name.as_str(), DataKind::Num)
    );
    assert_eq!(*sample, col.data.f64_at(0));
    let mut edit = edit.clone();
    let raw = col.data.text_at(0).into_owned();

    // 一次确定：名称、单位、注释、角色、格式一起改，作为一步撤销
    let before = app.project.clone();
    edit.long_name = "电压".into();
    edit.units = "V".into();
    edit.comment = "换算后".into();
    edit.role = Role::Ignore;
    edit.format = DisplayFormat {
        number: NumberFormat::Decimal { places: 1 },
        ..Default::default()
    };
    app.dialog = None;
    app.begin_edit("列属性");
    app.apply_column_props(1, edit.clone()).unwrap();
    app.history.changed();
    app.finish_edit();
    let col = app.project.book(1).unwrap().column("B").unwrap();
    assert_eq!(
        (col.long_name.as_str(), col.units.as_str(), col.comment.as_str()),
        ("电压", "V", "换算后")
    );
    assert_eq!(col.role, Role::Ignore);
    assert_eq!(col.data.text_at(0), raw, "原始写法不变");
    assert_eq!(col.display_at(0), format!("{:.1}", col.data.f64_at(0).unwrap()));
    assert_eq!(app.history.undo_label(), Some("列属性"));
    roundtrip(&mut app, &before);

    // 数值列转日期失败：整次不改，错误留在对话框
    let before = app.project.clone();
    let label = app.history.undo_label().map(str::to_owned);
    edit.kind = DataKind::DateTime;
    app.open_column_props();
    app.finish_edit();
    app.begin_edit("列属性");
    let err = app.apply_column_props(1, edit).unwrap_err();
    app.finish_edit();
    assert!(err.contains("无法转换"), "{err}");
    assert_eq!(app.project, before);
    assert_eq!(
        app.history.undo_label().map(str::to_owned),
        label,
        "失败时不产生撤销步骤"
    );
}

#[test]
fn data_type_commands_are_atomic_and_undoable() {
    use scicanvas_core::{ColumnData, DataKind};
    for (command, kind) in [
        ("col.type.num", DataKind::Num),
        ("col.type.text", DataKind::Text),
        ("col.type.category", DataKind::Category),
        ("col.type.datetime", DataKind::DateTime),
    ] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into()]);
        // 日期列用常见日期写法；其余用带前导零的数字，检查原始写法保留
        let first = if kind == DataKind::DateTime { "2026/10/18" } else { "01" };
        app.project.books[0].columns[0].data = match kind {
            DataKind::Num => ColumnData::Text(vec![first.into(), "2".into()]),
            DataKind::DateTime => ColumnData::Text(vec![first.into(), "2026".into()]),
            _ => vec![first.into(), "2".into()].into(),
        };
        let before = app.project.clone();
        app.run_command(command);
        assert_eq!(app.project.books[0].columns[0].data.kind(), kind);
        assert_eq!(app.project.books[0].columns[0].data.text_at(0), first);
        roundtrip(&mut app, &before);
    }
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into()]);
    let before = app.project.clone();
    app.run_command("col.type.datetime"); // fractional B cells are not millisecond timestamps
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn non_numeric_cell_requires_confirmation_and_can_cancel_or_undo_conversion() {
    use scicanvas_core::DataKind;
    for confirm in [false, true] {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        app.sheets.insert(
            1,
            SheetState {
                range: Some(Range::single(0, 0)),
                edit: Some(Edit {
                    cell: Cell { c: 0, r: 0 },
                    buf: "text input".into(),
                    focused: false,
                }),
                ..Default::default()
            },
        );
        editor_frame(&mut app, &ctx, vec![], false);
        editor_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            false,
        );
        assert!(matches!(app.dialog, Some(Dialog::ConvertText { .. })));
        assert_eq!(app.project, before);
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(1000.0, 900.0)),
            |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
        );
        harness.frame(&mut app, vec![]);
        let out = harness.frame(&mut app, vec![]);
        let label = if confirm { "转换为文本并写入" } else { "取消" };
        let at = crate::test_ui::find_text(&out, label)
            .map(|pos| pos + egui::vec2(4.0, 4.0))
            .unwrap();
        for pressed in [true, false] {
            harness.frame(
                &mut app,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(app.dialog.is_none());
        if confirm {
            assert_eq!(app.project.books[0].columns[0].data.kind(), DataKind::Text);
            assert_eq!(app.project.books[0].columns[0].data.text_at(0), "text input");
            assert_eq!(
                app.project.books[0].columns[0].data.text_at(1),
                before.books[0].columns[0].data.text_at(1)
            );
            roundtrip(&mut app, &before);
        } else {
            assert_eq!(app.project, before);
            assert!(app.history.undo_label().is_none());
        }
    }
}

#[test]
fn incompatible_grid_paste_leaves_typed_columns_and_headers_unchanged() {
    let ctx = egui::Context::default();
    let mut app = app();
    app.active = Some(1);
    let before = app.project.clone();
    let mut state = SheetState {
        range: Some(Range::single(0, 1)),
        ..Default::default()
    };
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Paste("text\t4\n5\t6".into())],
            ..Default::default()
        },
        |ctx| app.sheet_keys(ctx, 1, &mut state),
    );
    app.finish_edit();
    assert_eq!(app.project, before);
    assert!(app.status.contains("转换为文本列"));
    assert!(app.history.undo_label().is_none());
}

#[test]
fn excel_paste_after_column_header_selection_draws_text_x_and_undoes_both_steps() {
    use scicanvas_core::{ColumnData, Role};
    let mut app = app();
    app.active = Some(1);
    for c in &mut app.project.books[0].columns {
        c.data = ColumnData::default();
        c.long_name.clear();
        c.units.clear();
        c.comment.clear();
    }
    app.project.books[0].columns[2].role = Role::YError;
    app.project.books[0].columns[2].error_for = Some("B".into());
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    editor_frame(&mut app, &ctx, vec![], false);
    let output = editor_frame(&mut app, &ctx, vec![], false);
    let pos = crate::test_ui::find_text(&output, "A(X)").unwrap() + egui::vec2(5.0, 5.0);
    for pressed in [true, false] {
        editor_frame(&mut app, &ctx, vec![crate::test_ui::button(pos, pressed)], false);
    }
    assert!(app.sheets[&1].range.is_none());
    assert_eq!(app.selected_keys(1), ["A"]);
    editor_frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste(
            "4-10\t73000\t1500\r\n10-40\t76000\t11000\r\n40-100\t81000\t5400\r\n".into(),
        )],
        false,
    );
    assert_eq!(app.project.books[0].columns[0].data.text_at(0), "4-10");
    assert_eq!(app.project.books[0].columns[0].data.text_at(2), "40-100");
    assert_eq!(app.project.books[0].columns[1].data.f64_at(0), Some(73000.0));
    assert!(app.selected_keys(1).is_empty());
    assert_eq!(app.sheets[&1].range.unwrap().rows(), 3);
    let pasted = app.project.clone();
    app.run_command("plot.bar");
    let graph = app.project.graphs.last().unwrap();
    let series = graph.first_layer().series(app.project.book(1).unwrap()).1;
    assert_eq!(series.len(), 1);
    assert_eq!(series[0].points.len(), 3);
    assert_eq!(series[0].errors[0][1], Some(1500.0));
    let svg = scicanvas_core::svg::render_page(graph, &app.project);
    assert!(svg.contains("4-10") && svg.contains("40-100"));
    let after = app.project.clone();
    app.run_command("edit.undo");
    assert_eq!(app.project, pasted);
    app.run_command("edit.undo");
    assert_eq!(app.project, before);
    app.run_command("edit.redo");
    app.run_command("edit.redo");
    assert_eq!(app.project, after);
    let reopened =
        scicanvas_core::project_file::from_scv(&scicanvas_core::project_file::to_scv(&app.project).unwrap()).unwrap();
    assert_eq!(reopened, app.project);
}

#[test]
fn excel_block_paste_in_cell_editor_routes_to_grid_and_conflicts_keep_the_edit() {
    for valid in [true, false] {
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        app.sheets.insert(
            1,
            SheetState {
                range: Some(Range::single(0, 1)),
                edit: Some(Edit {
                    cell: Cell { c: 0, r: 1 },
                    buf: "unfinished".into(),
                    focused: false,
                }),
                ..Default::default()
            },
        );
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        editor_frame(&mut app, &ctx, vec![], false);
        let text = if valid {
            "1\t2\r\n3\t4\r\n"
        } else {
            "wrong\t2\r\nstill wrong\t4\r\n"
        };
        editor_frame(&mut app, &ctx, vec![egui::Event::Paste(text.into())], false);
        app.finish_edit();
        if valid {
            assert!(app.sheets[&1].edit.is_none());
            assert_eq!(app.project.books[0].columns[0].data.f64_at(1), Some(1.0));
            assert_eq!(app.project.books[0].columns[1].data.f64_at(2), Some(4.0));
            roundtrip(&mut app, &before);
        } else {
            assert_eq!(app.project, before);
            assert_eq!(app.sheets[&1].edit.as_ref().unwrap().buf, "unfinished");
            assert!(app.status.contains("转换为文本列"));
            assert!(app.history.undo_label().is_none());
        }
    }
}
