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
        let scicanvas_core::ColumnData::Num { values, .. } = &*app
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
    app.smooth(scicanvas_core::analysis::SmoothMethod::MovingAverage, 3);

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
    app.apply_column_props(1, edit).unwrap();

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
    assert!(app.set_column_values("B * 2", true));

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
    assert!(app.set_column_values("B * 2", false));

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
    app.apply_column_props(1, edit.clone()).unwrap();

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
        app.project.books[0].columns[0].data = (match kind {
            DataKind::Num => ColumnData::Text(vec![first.into(), "2".into()]),
            DataKind::DateTime => ColumnData::Text(vec![first.into(), "2026".into()]),
            _ => vec![first.into(), "2".into()].into(),
        })
        .into();
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
fn populated_numeric_grid_paste_adapts_and_preserves_existing_values() {
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
    assert_eq!(
        app.project.books[0].columns[0].data.kind(),
        scicanvas_core::DataKind::Text
    );
    assert_eq!(app.project.books[0].columns[0].data.text_at(1), "text");
    assert_eq!(
        app.project.books[0].columns[0].data.f64_at(3),
        before.books[0].columns[0].data.f64_at(3)
    );
    assert_eq!(
        app.project.books[0].columns[0].long_name,
        before.books[0].columns[0].long_name
    );
    roundtrip(&mut app, &before);
}

#[test]
fn excel_paste_after_column_header_selection_draws_text_x_and_undoes_both_steps() {
    use scicanvas_core::{ColumnData, Role};
    let mut app = app();
    app.active = Some(1);
    for c in &mut app.project.books[0].columns {
        c.data = (ColumnData::default()).into();
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
        if !valid {
            app.project.books[0].columns[0].data = vec!["2026-10-24".into(), "2026-10-25".into()].into();
            app.project.books[0].columns[0].data = (app.project.books[0].columns[0]
                .data
                .converted(scicanvas_core::DataKind::DateTime)
                .unwrap())
            .into();
        }
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

#[test]
fn populated_columns_including_aa_and_beyond_paste_identically_in_grid_and_numeric_editor() {
    let cases = [
        ("72933.03969", vec!["72933.03969"]),
        ("72933.03969\r\n", vec!["72933.03969"]),
        ("4-10\r\n", vec!["4-10"]),
        ("4-10\r\n10-40\r\n40-100\r\n", vec!["4-10", "10-40", "40-100"]),
    ];
    for column in [0, 1, 2, 3, 25, 26, 51, 99] {
        for editing in [false, true] {
            for (text, expected) in &cases {
                let mut app = app();
                app.active = Some(1);
                app.project.books[0].ensure_columns(100);
                for c in &mut app.project.books[0].columns[3..] {
                    c.data = vec!["100.25".into(); 12].into();
                }
                app.project.books[0].columns[2].role = Role::YError;
                app.project.books[0].columns[2].error_for = Some("B".into());
                let before = app.project.clone();
                app.sheets.insert(
                    1,
                    SheetState {
                        range: Some(Range::single(column, 5)),
                        edit: editing.then(|| Edit {
                            cell: Cell { c: column, r: 5 },
                            buf: before.books[0].columns[column].data.text_at(5).into_owned(),
                            focused: false,
                        }),
                        ..Default::default()
                    },
                );
                let ctx = egui::Context::default();
                crate::fonts::setup(&ctx);
                ctx.memory_mut(|m| m.request_focus(egui::Id::new(("sheet-focus", 1_u64))));
                editor_frame(&mut app, &ctx, vec![], false);
                editor_frame(&mut app, &ctx, vec![egui::Event::Paste(text.to_string())], false);
                app.finish_edit();
                assert!(app.sheets[&1].edit.is_none(), "col {column}, edit {editing}, {text:?}");
                let after_col = &app.project.books[0].columns[column];
                for (i, value) in expected.iter().enumerate() {
                    assert_eq!(after_col.data.text_at(5 + i), *value, "col {column}, edit {editing}");
                }
                for row in (0..5).chain(5 + expected.len()..before.books[0].columns[column].data.len()) {
                    assert_eq!(
                        after_col.data.text_at(row),
                        before.books[0].columns[column].data.text_at(row)
                    );
                    assert_eq!(
                        after_col.data.f64_at(row),
                        before.books[0].columns[column].data.f64_at(row)
                    );
                }
                assert_eq!(after_col.role, before.books[0].columns[column].role);
                assert_eq!(after_col.error_for, before.books[0].columns[column].error_for);
                for other in (0..before.books[0].columns.len()).filter(|other| *other != column) {
                    assert_eq!(app.project.books[0].columns[other], before.books[0].columns[other]);
                }
                roundtrip(&mut app, &before);
                let json = scicanvas_core::project_file::to_json(&app.project).unwrap();
                assert_eq!(scicanvas_core::project_file::from_json(&json).unwrap(), app.project);
            }
        }
    }
}

#[test]
fn b_column_paste_works_with_nonmodal_column_properties_open() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("col.props");
    assert!(matches!(app.dialog, Some(Dialog::ColumnProps { .. })));
    app.selection.remove(&1);
    app.sheets.insert(
        1,
        SheetState {
            range: Some(Range::single(1, 5)),
            ..Default::default()
        },
    );
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(("sheet-focus", 1_u64))));
    editor_frame(&mut app, &ctx, vec![], false);
    editor_frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("72933.03969\r\n".into())],
        false,
    );
    app.finish_edit();
    assert_eq!(app.project.books[0].columns[1].data.f64_at(5), Some(72933.03969));
    roundtrip(&mut app, &before);
}

#[test]
fn incompatible_date_grid_paste_keeps_types_headers_and_values() {
    let mut app = app();
    app.active = Some(1);
    app.project.books[0].columns[0].data = vec!["2026-10-24".into(), "2026-10-25".into()].into();
    app.project.books[0].columns[0].data = (app.project.books[0].columns[0]
        .data
        .converted(scicanvas_core::DataKind::DateTime)
        .unwrap())
    .into();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    let mut state = SheetState {
        range: Some(Range::single(0, 1)),
        ..Default::default()
    };
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Paste("wrong\t4\r\nwrong again\t6".into())],
            ..Default::default()
        },
        |ctx| app.sheet_keys(ctx, 1, &mut state),
    );
    app.finish_edit();
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn b_paste_replaces_pending_conversion_instead_of_leaving_a_stale_write() {
    let mut app = app();
    app.active = Some(1);
    let before = app.project.clone();
    app.dialog = Some(Dialog::ConvertText {
        book: 1,
        key: "B".into(),
        row: 5,
        value: "older input".into(),
    });
    app.sheets.insert(
        1,
        SheetState {
            range: Some(Range::single(1, 5)),
            ..Default::default()
        },
    );
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(("sheet-focus", 1_u64))));
    editor_frame(&mut app, &ctx, vec![], false);
    editor_frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("72933.03969\r\n".into())],
        false,
    );
    app.finish_edit();
    assert!(app.dialog.is_none());
    assert_eq!(app.project.books[0].columns[1].data.f64_at(5), Some(72933.03969));
    roundtrip(&mut app, &before);
}

#[test]
fn wide_paste_from_added_columns_expands_book_and_undoes_as_one_edit() {
    for start in [3, 25, 26, 99] {
        let mut app = app();
        app.active = Some(1);
        app.project.books[0].ensure_columns(start + 2);
        for c in &mut app.project.books[0].columns[3..] {
            c.data = vec!["100.25".into(); 12].into();
        }
        let before = app.project.clone();
        app.sheets.insert(
            1,
            SheetState {
                range: Some(Range::single(start, 5)),
                ..Default::default()
            },
        );
        let rows: Vec<Vec<String>> = (0..3)
            .map(|r| {
                (0..40)
                    .map(|c| {
                        if c % 2 == 0 {
                            format!("group-{r}-{c}")
                        } else {
                            format!("{}", r * 100 + c)
                        }
                    })
                    .collect()
            })
            .collect();
        let text = rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\r\n") + "\r\n";
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(("sheet-focus", 1_u64))));
        editor_frame(&mut app, &ctx, vec![], false);
        editor_frame(&mut app, &ctx, vec![egui::Event::Paste(text)], false);
        app.finish_edit();
        let book = &app.project.books[0];
        assert_eq!(book.columns.len(), start + 40);
        assert_eq!(&book.columns[..start], &before.books[0].columns[..start]);
        for (r, row) in rows.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                assert_eq!(book.columns[start + c].data.text_at(5 + r), *value);
            }
        }
        for c in start..start + 2 {
            assert_eq!(book.columns[c].data.text_at(0), "100.25");
            assert_eq!(book.columns[c].data.text_at(11), "100.25");
        }
        assert_eq!(app.sheets[&1].range.unwrap().cols(), 40);
        assert_eq!(app.sheets[&1].range.unwrap().rows(), 3);
        roundtrip(&mut app, &before);
        let scv = scicanvas_core::project_file::to_scv(&app.project).unwrap();
        assert_eq!(scicanvas_core::project_file::from_scv(&scv).unwrap(), app.project);
    }
}

#[test]
fn grouped_column_properties_reject_atomically_without_losing_valid_input_or_copying_data() {
    let mut app = app();
    let before = app.project.clone();
    let group = Some(egui::Id::new("column-name"));
    app.history_group = group;
    let allocation = |app: &SciApp| match &*app.project.books[0].columns[1].data {
        scicanvas_core::ColumnData::Num { values, .. } => values.as_ptr(),
        _ => panic!("numeric"),
    };
    let original = allocation(&app);
    let mut edit = dialogs::ColumnEdit::from(&app.project.books[0].columns[1]);
    edit.long_name = "typing".into();
    app.apply_column_props(1, edit.clone()).unwrap();
    let first = app.project.clone();
    let mut invalid = edit.clone();
    invalid.kind = scicanvas_core::DataKind::DateTime;
    invalid.long_name = "rejected".into();
    assert!(app.apply_column_props(1, invalid).unwrap_err().contains("无法转换"));
    assert_eq!(app.project, first);
    assert!(app.history.is_pending());
    assert_eq!(allocation(&app), original);
    edit.units = "V".into();
    app.apply_column_props(1, edit).unwrap();
    assert_eq!(allocation(&app), original);
    app.finish_edit();
    roundtrip(&mut app, &before);
    app.undo();
    assert!(app.history.undo_label().is_none());
    let before = app.project.clone();
    let redo = app.history.redo_label().unwrap().to_string();
    let mut invalid = dialogs::ColumnEdit::from(&app.project.books[0].columns[1]);
    invalid.kind = scicanvas_core::DataKind::DateTime;
    assert!(app.apply_column_props(1, invalid).is_err());
    assert_eq!(app.project, before);
    assert!(!app.history.is_pending());
    assert_eq!(app.history.redo_label(), Some(redo.as_str()));
}

#[test]
fn worksheet_headers_group_real_typing_and_separate_fields_without_copying_columns() {
    let mut app = app();
    app.show_item(1);
    app.project.books[0].columns[0].long_name = "HeaderOne".into();
    app.project.books[0].columns[1].long_name = "HeaderTwo".into();
    let before = app.project.clone();
    let scicanvas_core::ColumnData::Num { values, .. } = &*app.project.books[0].columns[0].data else {
        panic!("numeric");
    };
    let allocation = values.as_ptr();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1600.0, 1100.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.ui_frame(ctx),
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    let at = crate::test_ui::text_rect(&output, "HeaderOne").unwrap().center();
    harness.click(&mut app, at);
    harness.frame(&mut app, vec![egui::Event::Text("a".into())]);
    harness.frame(&mut app, vec![egui::Event::Text("b".into())]);
    let first = app.project.clone();
    assert_ne!(first, before);
    assert!(app.history.is_pending());
    let output = harness.frame(&mut app, vec![]);
    let rect = app.geoms[&1].rect.translate(app.ws.min.to_vec2());
    let at = crate::test_ui::text_positions(&output, "HeaderTwo")
        .into_iter()
        .rfind(|p| rect.contains(*p))
        .unwrap()
        + egui::vec2(3.0, 3.0);
    harness.click(&mut app, at);
    harness.frame(&mut app, vec![egui::Event::Text("c".into())]);
    let second = app.project.clone();
    assert_ne!(second, first);
    let scicanvas_core::ColumnData::Num { values, .. } = &*app.project.books[0].columns[0].data else {
        panic!("numeric");
    };
    assert_eq!(values.as_ptr(), allocation);
    app.finish_edit();
    app.undo();
    assert_eq!(app.project, first);
    app.undo();
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
    app.redo();
    app.redo();
    assert_eq!(app.project, second);
}

#[test]
fn rejected_sheet_paste_preserves_redo_editor_selection_and_pending_conversion() {
    let mut app = app();
    app.project.books[0].columns[0].data = vec!["2026-10-25".into(), "2026-10-26".into()].into();
    app.project.books[0].columns[0].data = (app.project.books[0].columns[0]
        .data
        .converted(scicanvas_core::DataKind::DateTime)
        .unwrap())
    .into();
    app.run_command("book.new");
    app.undo();
    let before = app.project.clone();
    let redo = app.history.redo_label().unwrap().to_string();
    app.selection.insert(1, vec!["A".into()]);
    app.dialog = Some(Dialog::ConvertText {
        book: 1,
        key: "B".into(),
        row: 0,
        value: "older input".into(),
    });
    let mut state = SheetState {
        range: Some(Range::single(0, 0)),
        edit: Some(Edit {
            cell: Cell { c: 0, r: 0 },
            buf: "unfinished".into(),
            focused: false,
        }),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Paste("bad\t9\r\n".into())],
            ..Default::default()
        },
        |ctx| app.sheet_keys(ctx, 1, &mut state),
    );
    assert_eq!(app.project, before);
    assert_eq!(state.edit.as_ref().unwrap().buf, "unfinished");
    assert_eq!(app.selected_keys(1), ["A"]);
    assert!(matches!(&app.dialog,Some(Dialog::ConvertText {value,..}) if value=="older input"));
    assert!(!app.history.is_pending());
    assert!(app.history.undo_label().is_none());
    assert_eq!(app.history.redo_label(), Some(redo.as_str()));
    state.edit = None;
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Copy],
            ..Default::default()
        },
        |ctx| app.sheet_keys(ctx, 1, &mut state),
    );
    assert_eq!(app.project, before);
    assert!(!app.history.is_pending());
    assert_eq!(app.history.redo_label(), Some(redo.as_str()));
}

#[test]
fn migrated_worksheet_dialogs_match_core_results_and_commit_one_step() {
    use scicanvas_core::{project_file, stats::Sequence};
    for case in 0..4 {
        let mut app = app();
        app.active = Some(1);
        if case == 3 {
            app.project.books[0].columns[0].data = vec!["b".into(), "a".into(), "b".into()].into();
            app.project.books[0].columns[0].data = (app.project.books[0].columns[0]
                .data
                .converted(scicanvas_core::DataKind::Category)
                .unwrap())
            .into();
        }
        let before = app.project.clone();
        let mut expected = before.clone();
        let label = match case {
            0 => {
                app.selection.insert(1, vec!["A".into()]);
                app.open_sort_dialog();
                if let Some(Dialog::Sort { keys, .. }) = &mut app.dialog {
                    keys[0] = (Some(0), false);
                } else {
                    panic!("sort");
                }
                expected.sort_rows(1, &[("A".into(), false)], None).unwrap();
                press_dialog_button(&mut app, "排序");
                "排序"
            }
            1 => {
                app.selection.insert(1, vec!["C".into()]);
                app.open_set_values_dialog();
                if let Some(Dialog::SetValues { formula, keep, .. }) = &mut app.dialog {
                    *formula = "B * 2".into();
                    *keep = true;
                } else {
                    panic!("formula");
                }
                expected.set_column_formula(1, "C", "B * 2", true).unwrap();
                press_dialog_button(&mut app, "计算");
                "设置列值"
            }
            2 => {
                app.selection.insert(1, vec!["B".into()]);
                app.open_fill_dialog();
                if let Some(Dialog::Fill { a, b, rows, .. }) = &mut app.dialog {
                    *a = 1.0;
                    *b = 2.0;
                    *rows = 4;
                } else {
                    panic!("fill");
                }
                expected
                    .fill_sequence(1, "B", Sequence::Arithmetic { start: 1.0, step: 2.0 }, 4, 0)
                    .unwrap();
                press_dialog_button(&mut app, "填充");
                "填充数据序列"
            }
            _ => {
                app.selection.insert(1, vec!["A".into()]);
                app.open_category_order_dialog();
                if let Some(Dialog::CategoryOrder { levels, .. }) = &mut app.dialog {
                    levels.reverse();
                } else {
                    panic!("categories");
                }
                let Some(Dialog::CategoryOrder { levels, .. }) = &app.dialog else {
                    panic!("categories");
                };
                expected.books[0].columns[0].data.set_category_order(levels).unwrap();
                app.refresh_book_preview();
                press_dialog_button(&mut app, "×");
                "类别顺序"
            }
        };
        expected.recalc_computed(false);
        assert_eq!(app.project, expected, "case {case}: {}", app.status);
        assert_eq!(app.history.undo_label(), Some(label));
        assert_eq!(
            project_file::from_scv(&project_file::to_scv(&app.project).unwrap()).unwrap(),
            app.project
        );
        roundtrip(&mut app, &before);
        app.undo();
        assert!(app.history.undo_label().is_none());
    }
}

#[test]
fn nonlinear_recalc_uses_column_modes_and_one_undo_step() {
    use scicanvas_core::{nlfit::Model, ops::RecalcMode, project_file};
    for mode in [RecalcMode::Auto, RecalcMode::Manual, RecalcMode::None] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.nlfit(Model::ExpDec1, &[], &[]);
        let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
            panic!("report");
        };
        let y = reports[0].curve.1.clone();
        app.selection.insert(1, vec![y.clone()]);
        app.run_command("col.props");
        let Some(Dialog::ColumnProps { mut edit, .. }) = app.dialog.take() else {
            panic!("column props");
        };
        assert_eq!(edit.recalc, Some(RecalcMode::Auto));
        edit.recalc = Some(mode);
        app.apply_column_props(1, edit).unwrap();
        let before = app.project.clone();
        let old = before.book(1).unwrap().column(&y).unwrap().data.clone();
        app.edit("修改拟合来源", |project| {
            project.book_mut(1).unwrap().columns[2].set(0, "10".into());
            Ok(Some(()))
        })
        .unwrap();
        assert_eq!(
            app.project.book(1).unwrap().column(&y).unwrap().data == old,
            mode != RecalcMode::Auto
        );
        roundtrip(&mut app, &before);
        let before = app.project.clone();
        app.run_command("data.recalc");
        if mode == RecalcMode::Manual {
            assert_ne!(app.project.book(1).unwrap().column(&y).unwrap().data, old);
            roundtrip(&mut app, &before);
        }
        if mode == RecalcMode::None {
            assert_eq!(app.project.book(1).unwrap().column(&y).unwrap().data, old);
        }
        let saved = project_file::from_scv(&project_file::to_scv(&app.project).unwrap()).unwrap();
        assert_eq!(saved, app.project);
    }
}

#[test]
fn selection_status_statistics_use_only_finite_selected_cells() {
    let mut app = app();
    app.active = Some(1);
    let book = app.project.book_mut(1).unwrap();
    book.columns[1].data = ["2", "", "-4", "6", "NaN"].map(str::to_owned).to_vec().into();
    app.selection.insert(1, vec!["B".into()]);
    assert_eq!(
        app.stats_text(),
        "平均值 = 1.3333333   总和 = 4   计数 = 3   最小 = -4   最大 = 6"
    );
    app.selection.clear();
    app.sheets.insert(
        1,
        SheetState {
            range: Some(Range {
                a: Cell { c: 1, r: 0 },
                b: Cell { c: 1, r: 2 },
            }),
            ..Default::default()
        },
    );
    assert_eq!(
        app.stats_text(),
        "平均值 = -1   总和 = -2   计数 = 2   最小 = -4   最大 = 2"
    );
}

#[test]
fn all_column_roles_apply_and_undo_without_changing_values() {
    for role in [
        Role::X,
        Role::Y,
        Role::Z,
        Role::XError,
        Role::YError,
        Role::Label,
        Role::Ignore,
    ] {
        let mut app = app();
        app.active = Some(1);
        app.project.book_mut(1).unwrap().columns[1].role = if role == Role::Y { Role::X } else { Role::Y };
        app.selection.insert(1, vec!["B".into()]);
        app.run_command("col.props");
        let Some(Dialog::ColumnProps { edit, .. }) = &app.dialog else {
            panic!("column properties should open")
        };
        let mut edit = edit.clone();
        edit.role = role;
        app.dialog = None;
        let before = app.project.clone();
        let values = before.book(1).unwrap().column("B").unwrap().data.clone();
        app.apply_column_props(1, edit.clone()).unwrap();
        let column = app.project.book(1).unwrap().column("B").unwrap();
        assert_eq!(column.role, role);
        assert_eq!(column.data, values);
        roundtrip(&mut app, &before);
        let current = app.project.clone();
        edit.key = "missing".into();
        assert!(app.apply_column_props(1, edit).is_err());
        assert_eq!(app.project, current);
    }
}

#[test]
fn mouse_drag_shift_extension_and_copy_select_the_visible_cells() {
    let mut app = app();
    app.active = Some(1);
    let book = app.project.book_mut(1).unwrap();
    book.columns[0].data = ["10101", "10102", "10103", "10104"].map(str::to_string).to_vec().into();
    book.columns[1].data = ["20201", "20202", "20203", "20204"].map(str::to_string).to_vec().into();
    book.columns[2].data = ["30301", "30302", "30303", "30304"].map(str::to_string).to_vec().into();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1200.0, 900.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
        },
    )
    .with_step(0.05);
    let mut output = harness.frame(&mut app, vec![]);
    for _ in 0..3 {
        output = harness.frame(&mut app, vec![]);
    }
    let pos = |label| crate::test_ui::find_text(&output, label).unwrap() + egui::vec2(4.0, 4.0);
    let start = pos("10101");
    let end = pos("20203");
    let extend = pos("30304");
    harness.frame(&mut app, vec![egui::Event::PointerMoved(start)]);
    harness.frame(&mut app, vec![crate::test_ui::button(start, true)]);
    harness.frame(&mut app, vec![egui::Event::PointerMoved(end)]);
    harness.frame(&mut app, vec![crate::test_ui::button(end, false)]);
    assert_eq!(
        app.sheets[&1].range,
        Some(Range {
            a: Cell { c: 0, r: 0 },
            b: Cell { c: 1, r: 2 }
        })
    );
    harness.modifiers.shift = true;
    harness.click(&mut app, extend);
    harness.modifiers.shift = false;
    assert_eq!(
        app.sheets[&1].range,
        Some(Range {
            a: Cell { c: 0, r: 0 },
            b: Cell { c: 2, r: 3 }
        })
    );
    let copied = harness
        .frame(&mut app, vec![egui::Event::Copy])
        .platform_output
        .copied_text;
    assert_eq!(
        copied,
        "10101\t20201\t30301\n10102\t20202\t30302\n10103\t20203\t30303\n10104\t20204\t30304"
    );
    assert_eq!(app.project, before);
}

#[test]
fn million_rows_scroll_to_the_last_value_without_rendering_every_row() {
    let mut app = app();
    app.active = Some(1);
    let rows = 1_000_000usize;
    let mut values = vec![0.0; rows];
    values[rows - 1] = 98765.4321;
    let book = app.project.book_mut(1).unwrap();
    book.columns.truncate(1);
    book.columns[0].data = scicanvas_core::ColumnData::Num {
        values,
        missing: vec![0; rows.div_ceil(64)],
        lexemes: Default::default(),
    }
    .into();
    app.sheets.insert(
        1,
        SheetState {
            scroll_to: Some(rows - 1),
            ..Default::default()
        },
    );
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1200.0, 900.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
        },
    )
    .with_step(1.0);
    let mut output = harness.frame(&mut app, vec![]);
    for _ in 0..3 {
        output = harness.frame(&mut app, vec![]);
    }
    assert!(
        crate::test_ui::find_text(&output, "98765.4321").is_some(),
        "last row should be visible"
    );
    assert!(output.shapes.len() < 20_000, "only visible rows should be drawn");
}

#[test]
fn find_replace_searches_selected_cells_wraps_and_matches_whole_cells() {
    let mut app = app();
    app.active = Some(1);
    let book = app.project.book_mut(1).unwrap();
    book.columns[1].data = ["Alpha", "ALPHA", "other"].map(str::to_string).to_vec().into();
    book.columns[2].data = ["alpha", "alphabet", "other"].map(str::to_string).to_vec().into();
    app.sheets.insert(
        1,
        SheetState {
            range: Some(Range {
                a: Cell { c: 1, r: 0 },
                b: Cell { c: 2, r: 1 },
            }),
            ..Default::default()
        },
    );
    app.run_command("edit.find");
    let Some(Dialog::FindReplace(state)) = &mut app.dialog else {
        panic!("find dialog")
    };
    state.needle = "alpha".into();
    state.whole_cell = true;
    let before = app.project.clone();
    for cell in [
        Cell { c: 1, r: 0 },
        Cell { c: 2, r: 0 },
        Cell { c: 1, r: 1 },
        Cell { c: 1, r: 0 },
    ] {
        app.find_next();
        assert_eq!(app.sheets[&1].range, Some(Range::single(cell.c, cell.r)));
    }
    let Some(Dialog::FindReplace(state)) = &mut app.dialog else {
        panic!()
    };
    state.match_case = true;
    state.cursor = None;
    app.find_next();
    assert_eq!(app.sheets[&1].range, Some(Range::single(2, 0)));
    assert_eq!(app.project, before);
    let Some(Dialog::FindReplace(state)) = &mut app.dialog else {
        panic!()
    };
    state.match_case = false;
    state.replacement = "done".into();
    app.replace_matches(true);
    let book = app.project.book(1).unwrap();
    assert_eq!(book.columns[1].data.text_at(0), "done");
    assert_eq!(book.columns[1].data.text_at(1), "done");
    assert_eq!(book.columns[2].data.text_at(0), "done");
    assert_eq!(book.columns[2].data.text_at(1), "alphabet");
    roundtrip(&mut app, &before);
}

#[test]
fn find_replace_is_atomic_preserves_invalid_numbers_and_handles_unicode() {
    let mut app = app();
    app.active = Some(1);
    let book = app.project.book_mut(1).unwrap();
    book.columns[0].data = ["12", "31", "44"].map(str::to_string).to_vec().into();
    book.columns[1].data = ["a1b", "A1", "İSTANBUL"].map(str::to_string).to_vec().into();
    book.columns[2].data.clear();
    app.run_command("edit.replace");
    let Some(Dialog::FindReplace(state)) = &mut app.dialog else {
        panic!("replace dialog")
    };
    state.needle = "1".into();
    state.replacement = "x".into();
    let before = app.project.clone();
    app.replace_matches(true);
    let book = app.project.book(1).unwrap();
    assert_eq!(book.columns[0].data.text_at(0), "12");
    assert_eq!(book.columns[0].data.text_at(1), "31");
    assert_eq!(book.columns[1].data.text_at(0), "axb");
    assert_eq!(book.columns[1].data.text_at(1), "Ax");
    assert!(matches!(&app.dialog, Some(Dialog::FindReplace(state)) if state.message.contains("2 个因类型不符")));
    assert_eq!(app.history.undo_label(), Some("替换"));
    roundtrip(&mut app, &before);
    app.active = Some(1);
    app.open_find_replace(true);
    let Some(Dialog::FindReplace(state)) = &mut app.dialog else {
        panic!()
    };
    state.needle = "i".into();
    state.replacement = "X".into();
    state.cursor = Some(Cell { c: 1, r: 2 });
    app.replace_matches(false);
    assert_eq!(app.project.book(1).unwrap().columns[1].data.text_at(2), "XSTANBUL");
    app.undo();
    let current = app.project.clone();
    let redo = app.history.redo_label().unwrap().to_owned();
    app.active = Some(1);
    app.open_find_replace(true);
    let Some(Dialog::FindReplace(state)) = &mut app.dialog else {
        panic!()
    };
    state.needle = "no-match".into();
    app.replace_matches(true);
    assert_eq!(app.project, current);
    assert_eq!(app.history.redo_label(), Some(redo.as_str()));
}

#[test]
fn find_replace_buttons_and_shortcuts_open_and_apply() {
    let ctx = egui::Context::default();
    let mut app = app();
    app.active = Some(1);
    let input = egui::RawInput {
        modifiers: egui::Modifiers::COMMAND,
        events: vec![egui::Event::Key {
            key: egui::Key::H,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| app.shortcuts(ctx));
    let Some(Dialog::FindReplace(state)) = &mut app.dialog else {
        panic!("Ctrl+H should open replace")
    };
    assert!(state.replace_mode);
    state.needle = "0".into();
    press_dialog_button(&mut app, "查找下一个");
    assert!(matches!(&app.dialog, Some(Dialog::FindReplace(state)) if state.cursor.is_some()));
    press_dialog_button(&mut app, "关闭");
    assert!(app.dialog.is_none());
    let input = egui::RawInput {
        modifiers: egui::Modifiers::COMMAND,
        events: vec![egui::Event::Key {
            key: egui::Key::F,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| app.shortcuts(ctx));
    assert!(matches!(&app.dialog, Some(Dialog::FindReplace(state)) if !state.replace_mode));
}

#[test]
fn find_open_commits_valid_cell_draft_and_keeps_invalid_draft_for_confirmation() {
    let mut app = app();
    app.active = Some(1);
    app.sheets.insert(
        1,
        SheetState {
            range: Some(Range::single(0, 0)),
            edit: Some(Edit {
                cell: Cell { c: 0, r: 0 },
                buf: "777".into(),
                focused: false,
            }),
            ..Default::default()
        },
    );
    app.open_find_replace(false);
    assert_eq!(app.project.book(1).unwrap().columns[0].data.text_at(0), "777");
    assert!(app.sheets[&1].edit.is_none());
    assert!(matches!(app.dialog, Some(Dialog::FindReplace(_))));
    app.dialog = None;
    app.sheets.get_mut(&1).unwrap().edit = Some(Edit {
        cell: Cell { c: 0, r: 0 },
        buf: "not-a-number".into(),
        focused: false,
    });
    app.open_find_replace(false);
    assert_eq!(app.project.book(1).unwrap().columns[0].data.text_at(0), "777");
    assert!(matches!(app.dialog, Some(Dialog::ConvertText { .. })));
    assert!(app.sheets[&1].edit.is_some());
}

#[test]
fn transpose_command_creates_a_workbook_in_one_undo_step() {
    let mut app = app();
    app.active = Some(1);
    let before = app.project.clone();
    app.run_command("book.transpose");
    assert_eq!(app.project.books.len(), before.books.len() + 1);
    let result = app.project.books.last().unwrap();
    assert_eq!(
        result.columns[0].data.text_at(0),
        before.books[0].columns[0].display_name()
    );
    assert_eq!(
        result.columns[1].data.text_at(0),
        before.books[0].columns[0].data.text_at(0)
    );
    assert_eq!(app.history.undo_label(), Some("转置"));
    roundtrip(&mut app, &before);
}

#[test]
fn sheet_transforms_each_create_one_undo_step() {
    use crate::dialogs::{Action, TransformAction};
    let ctx = egui::Context::default();
    for operation in 0..3 {
        let mut app = app();
        app.active = Some(1);
        let book = app.project.book(1).unwrap();
        let a = book.columns[0].key.clone();
        let b = book.columns[1].key.clone();
        let action = match operation {
            0 => TransformAction::Stack(vec![a, b]),
            1 => {
                let keys = app.project.stack_columns(1, &[a, b]).unwrap();
                TransformAction::Split(keys[0].clone(), keys[1].clone())
            }
            _ => TransformAction::Reduce(a, 2, true),
        };
        let before = app.project.clone();
        assert!(app.apply_dialog_action(&ctx, Action::SheetTransform(1, action)));
        assert_eq!(app.history.undo_label(), Some("工作表变换"));
        roundtrip(&mut app, &before);
    }
}

#[test]
fn create_workbook_from_template_is_one_undo_step() {
    let mut app = app();
    let template = scicanvas_core::project_file::workbook_template_from_book(app.project.book(1).unwrap());
    let before = app.project.clone();
    let id = app.create_book_from_template(template).unwrap();
    assert_eq!(app.project.book(id).unwrap().nrows(), 0);
    assert_eq!(app.history.undo_label(), Some("从模板新建工作簿"));
    roundtrip(&mut app, &before);
}

#[test]
fn frozen_columns_scroll_horizontally_and_keep_rows_aligned() {
    let mut app = app();
    app.active = Some(1);
    let book = app.project.book_mut(1).unwrap();
    book.ensure_columns(8);
    for (column, col) in book.columns.iter_mut().enumerate() {
        col.data = (0..100)
            .map(|row| (10101 + column * 1000 + row).to_string())
            .collect::<Vec<_>>()
            .into();
    }
    let before = app.project.clone();
    app.selection.insert(1, vec!["A".into()]);
    app.run_command("sheet.freeze");
    assert_eq!(app.sheets[&1].frozen_columns, 1);
    app.sheets.get_mut(&1).unwrap().scroll_to = Some(70);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(600.0, 450.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
        },
    )
    .with_step(1.0);
    let mut output = harness.frame(&mut app, vec![]);
    for _ in 0..3 {
        output = harness.frame(&mut app, vec![]);
    }
    let fixed = crate::test_ui::find_text(&output, "10171").expect("fixed row visible");
    let right = crate::test_ui::find_text(&output, "11171").expect("scrolling row visible");
    assert!((fixed.y - right.y).abs() < 1.0, "frozen and scrolling rows align");
    let y_before = app.sheets[&1].frozen_scroll_y;
    harness.frame(
        &mut app,
        vec![
            egui::Event::PointerMoved(egui::pos2(70.0, 200.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -66.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..3 {
        output = harness.frame(&mut app, vec![]);
    }
    assert!(
        app.sheets[&1].frozen_scroll_y > y_before,
        "wheel over frozen region scrolls both sides"
    );
    let fixed = crate::test_ui::find_text(&output, "10174").expect("new fixed row visible");
    let right = crate::test_ui::find_text(&output, "11174").expect("new scrolling row visible");
    assert!((fixed.y - right.y).abs() < 1.0);
    harness.frame(
        &mut app,
        vec![
            egui::Event::PointerMoved(egui::pos2(450.0, 200.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(-300.0, 0.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..3 {
        output = harness.frame(&mut app, vec![]);
    }
    let after = crate::test_ui::find_text(&output, "10174").expect("fixed column remains visible");
    assert!((after.x - fixed.x).abs() < 1.0);
    if let Some(after_right) = crate::test_ui::find_text(&output, "11174") {
        assert!(after_right.x < right.x - 20.0, "scrolling columns move horizontally");
    }
    harness.click(&mut app, after + egui::vec2(4.0, 4.0));
    assert_eq!(app.sheets[&1].range.unwrap().a.c, 0);
    assert_eq!(app.project, before);
    app.run_command("sheet.unfreeze");
    assert_eq!(app.sheets[&1].frozen_columns, 0);
}
