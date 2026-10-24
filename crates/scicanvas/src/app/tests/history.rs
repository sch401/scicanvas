use super::*;

#[test]
fn commands_restore_columns_graphs_fits_and_sorting() {
    for command in [
        "col.delete",
        "col.left",
        "col.right",
        "col.first",
        "col.last",
        "sort.desc",
        "fit.linear",
        "plot.bar",
        "plot.stacked_bar",
        "plot.percent_stacked_bar",
        "plot.horizontal_bar",
        "plot.area",
        "plot.spline",
        "plot.spline_symbol",
        "plot.vertical_drop",
        "plot.step_horizontal",
        "plot.step_vertical",
        "col.setXError",
        "col.setYError",
        "col.setZ",
        "col.setLabel",
        "book.new",
    ] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        let before = app.project.clone();
        app.run_command(command);
        roundtrip(&mut app, &before);
    }
}

#[test]
fn undo_shortcut_respects_text_focus_and_accepts_button_focus() {
    let ctx = egui::Context::default();
    let mut app = app();
    let before = app.project.clone();
    app.run_command("book.new");
    let after = app.project.clone();
    let text_id = egui::Id::new("文字框");
    egui::TextEdit::store_state(&ctx, text_id, Default::default());
    for (id, expected) in [(text_id, &after), (egui::Id::new("按钮"), &before)] {
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            ctx.memory_mut(|m| m.request_focus(id));
            app.shortcuts(ctx);
        });
        assert_eq!(&app.project, expected);
    }
}

#[test]
fn continuous_edit_is_one_step_and_noop_preserves_redo() {
    let mut app = app();
    let before = app.project.clone();
    let group = Some(egui::Id::new("名称"));
    app.begin_edit("修改长名称");
    app.project.books[0].columns[0].long_name = "时".into();
    app.history.changed();
    app.finish_frame_edit(group);
    app.project.books[0].columns[0].long_name = "时间轴".into();
    app.history.changed();
    app.finish_frame_edit(group);
    roundtrip(&mut app, &before);
    app.run_command("edit.undo");
    assert!(app.history.undo_label().is_none());
    app.run_command("col.delete"); // 当前图形，没有工作簿，命令不可用
    assert!(app.history.redo_label().is_some());
    app.load_project(Project::sample(), None);
    assert!(app.history.redo_label().is_none());
    assert!(app.history.undo_label().is_none());
}

// 滑块测试的事务边界保持在实际绘制之前和之后。
fn slider_frame(
    harness: &mut crate::test_ui::Harness<impl FnMut(&mut SciApp, &egui::Context) -> egui::Rect>,
    app: &mut SciApp,
    events: Vec<egui::Event>,
) -> egui::Rect {
    app.begin_edit("线宽");
    let before = app.project.graphs[0].first_layer().line_width;
    app.history_group = None;
    let rect = harness.frame_value(app, events);
    if app.project.graphs[0].first_layer().line_width != before {
        app.history.changed();
    }
    app.finish_frame_edit(app.history_group);
    rect
}

#[test]
fn separate_slider_drags_are_separate_undo_steps() {
    let ctx = egui::Context::default();
    let mut app = app();
    let initial = app.project.clone();
    let mut harness = crate::test_ui::Harness::new(ctx.clone(), None, |app: &mut SciApp, ctx: &egui::Context| {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::Slider::new(&mut app.project.graphs[0].first_layer_mut().line_width, 0.5..=6.0)
                        .show_value(false),
                );
                edit_group(&response, &mut app.history_group);
                response.rect
            })
            .inner
    });

    let r = slider_frame(&mut harness, &mut app, vec![]);
    let mut states = vec![initial];
    for (start, end) in [(0.2, 0.5), (0.6, 0.9)] {
        let at = |f: f32| egui::pos2(r.left() + r.width() * f, r.center().y);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        slider_frame(
            &mut harness,
            &mut app,
            vec![egui::Event::PointerMoved(at(start)), button(at(start), true)],
        );
        slider_frame(&mut harness, &mut app, vec![egui::Event::PointerMoved(at(end))]);
        slider_frame(&mut harness, &mut app, vec![button(at(end), false)]);
        slider_frame(&mut harness, &mut app, vec![]);
        states.push(app.project.clone());
    }
    assert_ne!(states[0], states[1]);
    assert_ne!(states[1], states[2]);
    app.run_command("edit.undo");
    assert_eq!(app.project, states[1]);
    app.run_command("edit.undo");
    assert_eq!(app.project, states[0]);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn dialog_confirmation_is_recorded_and_cancellation_is_empty() {
    let mut app = app();
    let before = app.project.clone();
    app.run_command("fit.poly");
    assert!(app.history.undo_label().is_none());
    app.dialog = None;
    app.begin_edit("多项式拟合");
    app.fit(2);
    app.history.changed();
    app.finish_edit();
    roundtrip(&mut app, &before);
}

#[test]
fn headless_frames_and_history_shortcuts() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    egui_extras::install_image_loaders(&ctx);
    theme::install(&ctx);
    let mut app = app();
    let before = app.project.clone();
    app.run_command("book.new");
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0))),
        events: vec![egui::Event::Key {
            key: egui::Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
        ..Default::default()
    };
    let output = ctx.run(input, |ctx| app.ui_frame(ctx));
    assert!(!output.shapes.is_empty());
    assert_eq!(app.project, before);
    assert!(app.history.redo_label().is_some());
}

#[test]
fn idle_large_book_opens_no_history_transaction() {
    let ctx = egui::Context::default();
    let mut app = app();
    let mut p = Project::sample();
    p.books[0].columns[0].data = vec!["1".into(); 100_000].into();
    app.load_project(p, None);
    app.active = Some(1);
    for _ in 0..10 {
        editor_frame(&mut app, &ctx, vec![], false);
        assert!(!app.history.is_pending());
    }
    assert!(app.history.undo_label().is_none());
}

#[test]
fn unified_edit_returns_results_and_groups_project_changes_into_one_step() {
    let mut app = app();
    let before = app.project.clone();
    let (book, graph) = app
        .edit("建立表和图", |project| {
            let book = project.new_book();
            let b = project.book_mut(book).unwrap();
            b.columns[0].data = vec!["1".into(), "2".into()].into();
            b.columns[1].data = vec!["3".into(), "4".into()].into();
            let (graph, _) = project.plot_from_selection(book, &[], PlotType::LineSymbol)?;
            Ok(Some((book, graph)))
        })
        .unwrap();
    assert!(app.project.book(book).is_some() && app.project.graph(graph).is_some());
    assert_eq!(app.history.undo_label(), Some("建立表和图"));
    assert!(!app.history.is_pending());
    let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
    assert_eq!(scicanvas_core::project_file::from_scv(&saved).unwrap(), app.project);
    roundtrip(&mut app, &before);
    app.undo();
    assert!(app.history.undo_label().is_none());
}

#[test]
fn unified_edit_rolls_back_partial_errors_and_cancellation_without_losing_redo() {
    for fail in [false, true] {
        let mut app = app();
        app.run_command("book.new");
        let redo = app.project.clone();
        app.undo();
        let before = app.project.clone();
        let label = app.history.redo_label().unwrap().to_string();
        let result: Option<u64> = app.edit("放弃修改", |project| {
            project.books[0].columns[1].set(0, "临时文字".into());
            project.new_book();
            project.delete_columns(1, &["C".into()]);
            if fail {
                Err("计算失败".into())
            } else {
                Ok(None)
            }
        });
        assert_eq!(result, None);
        assert_eq!(app.project, before);
        assert!(!app.history.is_pending());
        assert!(app.history.undo_label().is_none());
        assert_eq!(app.history.redo_label(), Some(label.as_str()));
        if fail {
            assert_eq!(app.status, "计算失败");
        }
        app.redo();
        assert_eq!(app.project, redo);
    }
}

#[test]
fn unified_edit_noops_preserve_redo_and_do_not_recalculate_stale_formulas() {
    let mut app = app();
    app.run_command("book.new");
    app.undo();
    // 用未重算的公式检测无改动入口是否错误触发重算。
    app.project.books[0].columns[2].formula = Some("B * 2".into());
    let before = app.project.clone();
    let redo = app.history.redo_label().unwrap().to_string();
    for returns_value in [false, true] {
        let result = app.edit("无改动", |project| {
            project.books[0].name.push('!');
            project.books[0].name.pop();
            Ok(returns_value.then_some(42))
        });
        assert_eq!(result, returns_value.then_some(42));
        assert_eq!(app.project, before);
        assert!(!app.history.is_pending());
        assert!(app.history.undo_label().is_none());
        assert_eq!(app.history.redo_label(), Some(redo.as_str()));
    }
}

#[test]
fn unified_edit_recalculates_dependencies_in_the_same_step_and_preserves_diagnostics() {
    let mut app = app();
    let derived = app.project.books[0].push_column(Role::Y);
    app.project.set_column_formula(1, "C", "B * 2", true).unwrap();
    app.project.set_column_formula(1, &derived, "C + 1", true).unwrap();
    let before = app.project.clone();
    assert_eq!(
        app.edit("改源数据", |project| {
            project.books[0].columns[1].set(0, "100".into());
            Ok(Some(()))
        }),
        Some(())
    );
    let book = app.project.book(1).unwrap();
    assert_eq!(book.column("C").unwrap().data.f64_at(0), Some(200.0));
    assert_eq!(book.column(&derived).unwrap().data.f64_at(0), Some(201.0));
    roundtrip(&mut app, &before);
    let before = app.project.clone();
    assert_eq!(
        app.edit("删除公式来源", |project| {
            project.delete_columns(1, &["B".into()]);
            Ok(Some(()))
        }),
        Some(())
    );
    assert!(app.project.book(1).unwrap().column("B").is_none());
    assert!(app.status.starts_with("公式重算出错："));
    roundtrip(&mut app, &before);
}

#[test]
fn unified_edit_closes_previous_continuous_edits_without_merging_or_undoing_them() {
    for outcome in ["success", "cancel", "error"] {
        let mut app = app();
        let initial = app.project.clone();
        let group = Some(egui::Id::new("连续编辑"));
        app.begin_edit("连续名称");
        app.project.books[0].name = "完成的名称".into();
        app.history.changed();
        app.history_group = group;
        app.finish_frame_edit(group);
        let first = app.project.clone();
        assert!(app.history.is_pending());
        let result = app.edit("下一步", |project| {
            project.new_book();
            match outcome {
                "success" => Ok(Some(())),
                "cancel" => Ok(None),
                _ => Err("失败".into()),
            }
        });
        assert_eq!(result, outcome.eq("success").then_some(()));
        assert!(!app.history.is_pending());
        assert!(app.history_group.is_none() && app.last_history_group.is_none());
        if outcome == "success" {
            assert_eq!(app.history.undo_label(), Some("下一步"));
            app.undo();
            assert_eq!(app.project, first);
        } else {
            assert_eq!(app.project, first);
        }
        assert_eq!(app.history.undo_label(), Some("连续名称"));
        app.undo();
        assert_eq!(app.project, initial);
        assert!(app.history.undo_label().is_none());
    }
}
