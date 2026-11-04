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
        "plot.histogram",
        "plot.qq",
        "plot.box",
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
    app.fit(2);

    roundtrip(&mut app, &before);
}

#[test]
fn headless_frames_and_history_shortcuts() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    crate::icons::install_loaders(&ctx);
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

#[test]
fn grouped_edit_recalculates_live_and_keeps_one_snapshot_until_the_group_ends() {
    let mut app = app();
    app.project.set_column_formula(1, "C", "B * 2", true).unwrap();
    let before = app.project.clone();
    let group = Some(egui::Id::new("数值连续编辑"));
    for value in [100, 110, 120] {
        app.edit_grouped("连续源数据", group, |project| {
            project.books[0].columns[1].set(0, value.to_string())
        });
        assert_eq!(
            app.project.books[0].column("C").unwrap().data.f64_at(0),
            Some((value * 2) as f64)
        );
        assert!(app.history.is_pending());
        assert_eq!(app.history.undo_label(), Some("连续源数据"));
        app.finish_frame_edit(group);
    }
    app.finish_edit();
    roundtrip(&mut app, &before);
    app.undo();
    assert!(app.history.undo_label().is_none());
}

#[test]
fn grouped_edit_switches_groups_and_separates_discrete_changes() {
    let mut app = app();
    let mut states = vec![app.project.clone()];
    for (id, name) in [
        (Some(egui::Id::new("a")), "first"),
        (Some(egui::Id::new("b")), "second"),
        (None, "third"),
    ] {
        app.edit_grouped(name, id, |project| project.books[0].name = name.into());
        states.push(app.project.clone());
    }
    assert!(!app.history.is_pending());
    for before in states[..3].iter().rev() {
        app.undo();
        assert_eq!(&app.project, before);
    }
    assert!(app.history.undo_label().is_none());
    for after in states.iter().skip(1) {
        app.redo();
        assert_eq!(&app.project, after);
    }
}

#[test]
fn grouped_edit_returning_to_start_preserves_redo_and_discrete_failure_keeps_the_group() {
    let mut app = app();
    app.run_command("book.new");
    app.undo();
    let before = app.project.clone();
    let redo = app.history.redo_label().unwrap().to_string();
    let group = Some(egui::Id::new("name"));
    app.edit_grouped("名称", group, |project| project.books[0].name = "temporary".into());
    app.edit_grouped("名称", group, |project| {
        project.books[0].name = before.books[0].name.clone()
    });
    app.finish_edit();
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
    assert_eq!(app.history.redo_label(), Some(redo.as_str()));
    app.edit_grouped("有效名称", group, |project| project.books[0].name = "kept".into());
    let kept = app.project.clone();
    assert_eq!(
        app.edit::<()>("失败", |project| {
            project.new_book();
            Err("失败".into())
        }),
        None
    );
    assert_eq!(app.project, kept);
    assert_eq!(app.history.undo_label(), Some("有效名称"));
    app.undo();
    assert_eq!(app.project, before);
}

#[test]
fn migrated_delete_command_uses_one_step_and_retains_formula_diagnostics() {
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.project.set_column_formula(1, "C", "B * 2", true).unwrap();
    let before = app.project.clone();
    app.run_command("col.delete");
    assert_eq!(app.history.undo_label(), Some("删除列 B"));
    assert!(!app.history.is_pending());
    assert!(app.status.starts_with("公式重算出错："));
    roundtrip(&mut app, &before);
    app.undo();
    assert!(app.history.undo_label().is_none());
    app.run_command("book.new");
    assert!(app.history.undo_label().is_some());
}

#[test]
fn migrated_properties_group_real_typing_and_separate_focus_changes() {
    let mut app = app();
    app.props_for = Some(2);
    app.project.graphs[0].first_layer_mut().title = "GroupingTitle".into();
    app.project.graphs[0].first_layer_mut().x_title = "GroupingAxis".into();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1600.0, 1100.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.ui_frame(ctx),
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    let at = |output: &egui::FullOutput, text: &str| {
        let rect = ctx.memory(|m| m.area_rect(egui::Id::new("scicanvas-props"))).unwrap();
        crate::test_ui::text_positions(output, text)
            .into_iter()
            .rfind(|pos| rect.contains(*pos))
            .unwrap()
            + egui::vec2(5.0, 5.0)
    };
    harness.click(&mut app, at(&output, "GroupingTitle"));
    harness.frame(&mut app, vec![egui::Event::Text("a".into())]);
    harness.frame(&mut app, vec![egui::Event::Text("b".into())]);
    let first = app.project.clone();
    assert_ne!(first, before);
    assert!(app.history.is_pending());
    let output = harness.frame(&mut app, vec![]);
    harness.click(&mut app, at(&output, "GroupingAxis"));
    harness.frame(&mut app, vec![egui::Event::Text("x".into())]);
    let second = app.project.clone();
    assert_ne!(second, first);
    app.finish_edit();
    app.undo();
    assert_eq!(app.project, first);
    app.undo();
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
    app.redo();
    app.redo();
    assert_eq!(app.project, second);
    let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
    assert_eq!(scicanvas_core::project_file::from_scv(&saved).unwrap(), app.project);
}

#[test]
fn transaction_success_presentation_precedes_formula_diagnostics_and_is_not_run_on_error() {
    let mut app = app();
    app.project.set_column_formula(1, "C", "B * 2", true).unwrap();
    let before = app.project.clone();
    app.edit_with(
        "删除来源",
        |project| {
            project.delete_columns(1, &["B".into()]);
            Ok(Some(()))
        },
        |app, ()| app.status = "成功提示".into(),
    );
    assert!(app.status.starts_with("公式重算出错："));
    roundtrip(&mut app, &before);
    let before = app.project.clone();
    let mut shown = false;
    assert_eq!(
        app.edit_with::<(), ()>(
            "失败",
            |project| {
                project.new_book();
                Err("计算失败".into())
            },
            |_, ()| shown = true
        ),
        None
    );
    assert!(!shown);
    assert_eq!(app.project, before);
    assert_eq!(app.status, "计算失败");
}

#[test]
fn all_marked_commands_use_the_unified_transaction_and_roundtrip_in_one_step() {
    let mut changed = std::collections::HashSet::new();
    for command in commands::commands()
        .iter()
        .filter(|c| c.modifies && !matches!(c.id, "file.import" | "graph.template.apply"))
    {
        for scope in 0..3 {
            let mut app = app();
            let folder = app.project.add_folder(None).unwrap();
            app.active = Some(if scope == 1 { 2 } else { 1 });
            app.explorer_target = Some(match scope {
                0 => 1,
                1 => 2,
                _ => folder,
            });
            app.explorer_selection = vec![app.explorer_target.unwrap()];
            app.project_name = "Renamed".into();
            app.folder_destination = Some(folder);
            app.selection.insert(1, vec!["B".into(), "C".into()]);
            let before = app.project.clone();
            app.run_command(command.id);
            assert_eq!(app.edit_depth, 0, "{} scope {scope}", command.id);
            assert!(app.edit_abort.is_none());
            assert!(!app.history.is_pending(), "{} scope {scope}", command.id);
            if app.project != before {
                changed.insert(command.id);
                roundtrip(&mut app, &before);
                app.undo();
                assert!(app.history.undo_label().is_none(), "{} scope {scope}", command.id);
            } else {
                assert!(app.history.undo_label().is_none(), "{} scope {scope}", command.id);
            }
        }
    }
    for id in [
        "book.new",
        "graph.new",
        "note.new",
        "col.delete",
        "fit.linear",
        "project.rename",
        "project.move",
        "project.folder.remove",
        "example.line",
    ] {
        assert!(changed.contains(id), "command was not exercised: {id}");
    }
}

#[test]
fn nested_edit_reuses_the_outer_step_and_aborts_the_whole_command_on_failure_or_cancel() {
    for outcome in ["success", "cancel", "error"] {
        let mut app = app();
        app.run_command("book.new");
        app.undo();
        let before = app.project.clone();
        let redo = app.history.redo_label().unwrap().to_string();
        let mut shown = false;
        let result = app.edit_action_with(
            "outer",
            |app| {
                assert_eq!(app.edit_depth, 1);
                let id = app.edit("inner", |project| {
                    let id = project.new_book();
                    match outcome {
                        "success" => Ok(Some(id)),
                        "cancel" => Ok(None),
                        _ => Err("nested failure".into()),
                    }
                });
                if outcome != "success" {
                    app.project.new_book();
                }
                Ok(Some(id))
            },
            |_, id| {
                shown = true;
                id
            },
        );
        assert_eq!(app.edit_depth, 0);
        assert!(app.edit_abort.is_none());
        assert!(!app.history.is_pending());
        if outcome == "success" {
            assert!(result.flatten().is_some());
            assert!(shown);
            assert_eq!(app.history.undo_label(), Some("outer"));
            roundtrip(&mut app, &before);
            app.undo();
            assert!(app.history.undo_label().is_none());
        } else {
            assert_eq!(result, None);
            assert!(!shown);
            assert_eq!(app.project, before);
            assert_eq!(app.history.redo_label(), Some(redo.as_str()));
            assert!(app.history.undo_label().is_none());
            if outcome == "error" {
                assert_eq!(app.status, "nested failure");
            }
        }
    }
}

#[test]
fn dirty_savepoint_and_discard_confirmations_preserve_user_work() {
    let mut app = app();
    assert!(!app.is_dirty());
    app.run_command("book.new");
    assert!(app.is_dirty());
    let current = app.project.clone();
    app.run_command("project.new");
    assert_eq!(app.project, current);
    assert!(matches!(app.pending_discard, Some(PendingDiscard::New)));
    app.pending_discard = None;
    app.run_command("app.quit");
    assert!(!app.quit);
    assert!(matches!(app.pending_discard, Some(PendingDiscard::Quit)));
    app.pending_discard = None;
    let path = std::env::temp_dir().join(format!("scicanvas-savepoint-{}.scv", std::process::id()));
    assert!(app.save_project_to(&path));
    assert!(!app.is_dirty());
    app.run_command("book.new");
    assert!(app.is_dirty());
    app.undo();
    assert!(!app.is_dirty());
    app.redo();
    assert!(app.is_dirty());
    assert!(!app.save_project_to(&path.join("missing.scv")));
    assert!(app.is_dirty());
    assert!(app.save_error.is_some());
    app.execute_discard(PendingDiscard::New);
    assert_eq!(app.project.books.len(), 1);
    assert!(app.project.graphs.is_empty());
    assert!(!app.is_dirty());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn graph_only_edits_reuse_recalculation_inputs() {
    let mut app = app();
    app.run_command("book.new");
    let cached = app.recalc_inputs.clone().unwrap();
    let identities: Vec<_> = app
        .project
        .books
        .iter()
        .flat_map(|b| &b.columns)
        .map(|c| c.data.identity())
        .collect();
    app.edit("图形标题", |p| {
        p.graphs[0].first_layer_mut().title = "拖动与排版".into();
        Ok(Some(()))
    })
    .unwrap();
    assert_eq!(app.recalc_inputs.as_ref().unwrap(), &cached);
    assert_eq!(
        app.project
            .books
            .iter()
            .flat_map(|b| &b.columns)
            .map(|c| c.data.identity())
            .collect::<Vec<_>>(),
        identities
    );
    app.saved_generation = Some(app.history.generation());
    assert!(app.recovery_file.is_none());
    app.backup_recovery().unwrap();
    assert!(app.recovery_file.is_none());
}

#[test]
fn actual_discard_buttons_cancel_save_and_discard_the_right_project() {
    let mut app = app();
    app.run_command("book.new");
    let edited = app.project.clone();
    app.run_command("project.new");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1000., 800.)),
        |app: &mut SciApp, ctx: &egui::Context| app.session_dialogs(ctx),
    )
    .with_step(0.02);
    for _ in 0..3 {
        harness.frame(&mut app, vec![]);
    }
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "取消");
    assert_eq!(app.project, edited);
    assert!(app.pending_discard.is_none());
    app.run_command("project.new");
    for _ in 0..3 {
        harness.frame(&mut app, vec![]);
    }
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "不保存");
    assert!(app.project.graphs.is_empty());
    assert!(!app.is_dirty());
    app.run_command("book.new");
    let path = std::env::temp_dir().join(format!("scicanvas-confirm-save-{}.scv", std::process::id()));
    app.project_path = Some(path.clone());
    app.run_command("project.new");
    for _ in 0..3 {
        harness.frame(&mut app, vec![]);
    }
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "保存");
    assert!(path.exists());
    assert!(!app.is_dirty());
    assert_eq!(app.project.books.len(), 1);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn closing_with_a_cell_draft_keeps_it_pending_until_save() {
    let mut app = app();
    let sheet = app.sheets.entry(1).or_default();
    sheet.edit = Some(Edit {
        cell: Cell { c: 0, r: 0 },
        buf: "123".into(),
        focused: true,
    });
    assert!(app.is_dirty());
    app.request_discard(PendingDiscard::Quit);
    assert!(app.sheets[&1].edit.is_some());
    assert_eq!(app.project.books[0].columns[0].data.text_at(0), "0");
    assert!(app.pending_discard.is_some());
    assert!(!app.quit);
}

#[test]
fn background_backup_clear_waits_and_cannot_recreate_deleted_files() {
    let mut app = app();
    app.run_command("book.new");
    let dir = std::env::temp_dir().join(format!(
        "scicanvas-job-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("backup.scproj");
    app.recovery_lock = Some(scicanvas_core::project_file::session::lock_recovery(&path).unwrap());
    app.recovery_file = Some(path.clone());
    app.backup_recovery().unwrap();
    assert!(app.backup_job.is_some());
    app.finish_backup().unwrap();
    assert_eq!(scicanvas_core::project_file::load(&path).unwrap(), app.project);
    app.backup_recovery().unwrap();
    app.clear_recovery();
    app.finish_cleanup();
    assert!(app.backup_job.is_none());
    assert!(!path.exists());
    assert!(!path.with_extension("source").exists());
    std::fs::remove_dir(dir).unwrap();
}
#[test]
fn continuous_edit_cache_does_not_hold_column_data() {
    let mut app = app();
    app.run_command("book.new");
    app.begin_edit("continuous");
    let mut identities = std::collections::HashSet::new();
    for i in 0..60 {
        app.project.books[0].columns[0].set(0, (i + 100).to_string());
        app.history.changed();
        app.recalc_formulas();
        identities.insert(app.project.books[0].columns[0].data.identity());
    }
    assert_eq!(identities.len(), 1);
    app.finish_edit();
}

#[test]
fn backup_keeps_visible_fit_and_restoring_recalculates_drafts() {
    use scicanvas_core::{
        nlfit::Model,
        project_file::{self, session},
    };
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    app.nlfit(Model::ExpDec1, &[], &[]);
    let report = app.project.nlfit_reports()[0].clone();
    app.sheets.entry(1).or_default().edit = Some(Edit {
        cell: Cell { c: 2, r: 0 },
        buf: "10".into(),
        focused: true,
    });
    let dir = std::env::temp_dir().join(format!(
        "scicanvas-visible-backup-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("backup.scproj");
    app.recovery_lock = Some(session::lock_recovery(&path).unwrap());
    app.recovery_file = Some(path.clone());
    app.backup_recovery().unwrap();
    app.finish_backup().unwrap();
    let backup = project_file::load(&path).unwrap();
    assert_eq!(backup.nlfit_report(&report).unwrap().fit, report.fit);
    assert_eq!(backup.books[0].column("C").unwrap().data.text_at(0), "10");
    assert_ne!(app.project.books[0].column("C").unwrap().data.text_at(0), "10");
    app.recovery_lock = None;
    app.recovery_file = None;
    app.restore_recovery(&path);
    assert_ne!(app.project.nlfit_report(&report).unwrap().fit, report.fit);
    assert!(app.is_dirty());
    app.clear_recovery();
    app.finish_cleanup();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn shutdown_does_not_wait_on_the_ui_thread_and_can_return_to_the_project() {
    for cancel in [false, true] {
        let mut app = app();
        let before = app.project.clone();
        let (release, wait) = std::sync::mpsc::channel();
        app.backup_job = Some(std::thread::spawn(move || {
            let _ = wait.recv_timeout(std::time::Duration::from_secs(5));
            Ok(())
        }));
        app.request_discard(PendingDiscard::Quit);
        assert!(
            app.cleanup_task.is_some(),
            "close must return before the backup finishes"
        );
        assert!(app.cleanup_blocks_ui());
        assert!(!app.close_confirmed);
        assert!(!app.quit);
        if cancel {
            app.cancel_cleanup_discard();
            assert!(!app.cleanup_blocks_ui());
        }
        release.send(()).unwrap();
        app.finish_cleanup();
        assert_eq!(app.project, before);
        assert_eq!(app.quit, !cancel);
        assert_eq!(app.close_confirmed, !cancel);
    }
}
#[test]
fn clean_system_close_is_not_canceled_and_programmatic_close_requests_a_frame() {
    let mut app = app();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 900.))),
        ..Default::default()
    };
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    let out = ctx.run(input, |ctx| app.ui_frame(ctx));
    let commands = &out.viewport_output[&egui::ViewportId::ROOT].commands;
    assert!(app.close_confirmed);
    assert!(commands.contains(&egui::ViewportCommand::Close));
    assert!(!commands.contains(&egui::ViewportCommand::CancelClose));
    assert!(out.viewport_output[&egui::ViewportId::ROOT].repaint_delay.is_zero());
}

#[test]
fn invalid_draft_save_dismisses_close_confirmation_for_the_conversion_dialog() {
    let mut app = app();
    app.sheets.entry(1).or_default().edit = Some(Edit {
        cell: Cell { c: 0, r: 0 },
        buf: "invalid".into(),
        focused: true,
    });
    app.request_discard(PendingDiscard::Quit);
    assert!(app.pending_discard.is_some());
    assert!(!app.try_save_project(false));
    assert!(app.pending_discard.is_none());
    assert!(matches!(app.dialog, Some(Dialog::ConvertText { .. })));
    assert!(!app.quit);
    assert!(app.sheets[&1].edit.is_some());
}

#[test]
fn modal_keeps_background_and_drafts_while_blocking_commands() {
    let mut app = app();
    app.sheets.entry(1).or_default().edit = Some(Edit {
        cell: Cell { c: 0, r: 0 },
        buf: "123".into(),
        focused: false,
    });
    let before = app.project.clone();
    app.request_discard(PendingDiscard::Quit);
    let undo = app.history.undo_label().map(str::to_owned);
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    )
    .with_step(0.02);
    let mut output = h.frame(&mut app, vec![]);
    for _ in 0..3 {
        output = h.frame(&mut app, vec![]);
    }
    assert!(
        crate::test_ui::text_rect(&output, "SciCanvas").is_some(),
        "背景功能区继续绘制"
    );
    let event = egui::Event::Key {
        key: egui::Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    };
    h.modifiers = egui::Modifiers::COMMAND;
    h.frame(&mut app, vec![event]);
    h.modifiers = egui::Modifiers::NONE;
    output = h.frame(&mut app, vec![]);
    h.click_text(&mut app, &output, "新建工作簿");
    assert_eq!(app.project, before);
    assert_eq!(app.history.undo_label(), undo.as_deref());
    assert_eq!(app.sheets[&1].edit.as_ref().unwrap().buf, "123");
    output = h.frame(&mut app, vec![]);
    h.click_text(&mut app, &output, "取消");
    assert!(!app.quit);
    assert_eq!(app.project, before);
    assert_eq!(app.sheets[&1].edit.as_ref().unwrap().buf, "123");
}
#[test]
fn full_frame_modal_save_flushes_the_cell_draft() {
    let mut app = app();
    let path = std::env::temp_dir().join(format!("scicanvas-modal-save-{}.scv", std::process::id()));
    app.project_path = Some(path.clone());
    app.sheets.entry(1).or_default().edit = Some(Edit {
        cell: Cell { c: 0, r: 0 },
        buf: "123".into(),
        focused: false,
    });
    app.request_discard(PendingDiscard::New);
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    )
    .with_step(0.02);
    let mut output = h.frame(&mut app, vec![]);
    for _ in 0..3 {
        output = h.frame(&mut app, vec![]);
    }
    h.click_text(&mut app, &output, "保存");
    assert_eq!(
        scicanvas_core::project_file::load(&path).unwrap().books[0].columns[0]
            .data
            .text_at(0),
        "123"
    );
    assert!(!app.is_dirty());
    std::fs::remove_file(path).unwrap();
}
#[test]
fn cached_recovery_information_is_not_reloaded_each_frame() {
    let mut app = app();
    let path = std::env::temp_dir().join(format!("scicanvas-candidate-{}.scproj", std::process::id()));
    scicanvas_core::project_file::session::save_recovery(
        &path,
        &app.project,
        Some(std::path::Path::new("C:/cached-project.scv")),
    )
    .unwrap();
    app.recovery_candidates = vec![RecoveryCandidate::read(path.clone())];
    std::fs::remove_file(path.with_extension("source")).unwrap();
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..3 {
        h.frame(&mut app, vec![]);
    }
    for _ in 0..4 {
        let out = h.frame(&mut app, vec![]);
        assert!(out
            .shapes
            .iter()
            .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text.contains("cached-project.scv"))));
    }
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn quick_cleanup_keeps_the_ribbon_and_does_not_show_progress() {
    let mut app = app();
    let (release, wait) = std::sync::mpsc::channel();
    app.backup_job = Some(std::thread::spawn(move || {
        let _ = wait.recv_timeout(std::time::Duration::from_secs(5));
        Ok(())
    }));
    app.request_discard(PendingDiscard::Quit);
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    // 此处单帧前显式设定起点，避免字体加载耗时进入进度阈值。
    app.cleanup_task.as_mut().unwrap().started = std::time::Instant::now();
    let out = h.frame(&mut app, vec![]);
    assert!(crate::test_ui::text_rect(&out, "SciCanvas").is_some());
    assert!(crate::test_ui::text_rect(&out, "正在完成自动备份和恢复文件清理…").is_none());
    release.send(()).unwrap();
    app.finish_cleanup();
    assert!(app.quit);
}

#[test]
fn smoke_test_draws_the_workspace_and_requests_close() {
    let mut app = app();
    app.smoke_test = true;
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 820.)),
        |a: &mut SciApp, ctx: &egui::Context| a.ui_frame(ctx),
    );
    let output = h.frame(&mut app, vec![]);
    assert!(app.close_confirmed && app.quit);
    assert!(output
        .viewport_output
        .values()
        .any(|v| v.commands.contains(&egui::ViewportCommand::Close)));
    assert!(crate::test_ui::find_text(&output, "新建工作簿").is_some());
}

#[test]
fn modal_caption_controls_remain_live_without_changing_the_project() {
    let mut app = app();
    app.sheets.entry(1).or_default().edit = Some(Edit {
        cell: Cell { c: 0, r: 0 },
        buf: "123".into(),
        focused: false,
    });
    app.request_discard(PendingDiscard::Quit);
    let project = app.project.clone();
    let undo = app.history.undo_label().map(str::to_owned);
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, ctx: &egui::Context| a.ui_frame(ctx),
    );
    for _ in 0..4 {
        h.frame(&mut app, vec![]);
    }
    let caption = crate::chrome::caption(&h.ctx).unwrap();
    let output = h.click(&mut app, egui::pos2(caption.right() - 46. * 2.5, caption.center().y));
    assert!(output
        .viewport_output
        .values()
        .any(|v| v.commands.contains(&egui::ViewportCommand::Minimized(true))));
    assert!(app.pending_discard.is_some());
    let output = h.click(&mut app, egui::pos2(caption.right() - 23., caption.center().y));
    assert!(output
        .viewport_output
        .values()
        .any(|v| v.commands.contains(&egui::ViewportCommand::Close)));
    assert!(app.pending_discard.is_some());
    h.click(&mut app, egui::pos2(caption.left() + 18., caption.center().y));
    assert_eq!(app.project, project);
    assert_eq!(app.history.undo_label(), undo.as_deref());
    assert_eq!(app.sheets[&1].edit.as_ref().unwrap().buf, "123");
    assert!(!app.quit);
}

#[test]
fn modal_keyboard_saves_or_cancels_without_committing_cancelled_drafts() {
    for save in [false, true] {
        let mut app = app();
        let path = std::env::temp_dir().join(format!("scicanvas-modal-keys-{}-{save}.scv", std::process::id()));
        app.project_path = Some(path.clone());
        app.sheets.entry(1).or_default().edit = Some(Edit {
            cell: Cell { c: 0, r: 0 },
            buf: "123".into(),
            focused: false,
        });
        let project = app.project.clone();
        app.request_discard(PendingDiscard::New);
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        let mut h = crate::test_ui::Harness::new(
            ctx,
            Some(egui::vec2(1280., 900.)),
            |a: &mut SciApp, ctx: &egui::Context| a.ui_frame(ctx),
        );
        for _ in 0..4 {
            h.frame(&mut app, vec![]);
        }
        let key = if save { egui::Key::Enter } else { egui::Key::Escape };
        h.frame(
            &mut app,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.pending_discard.is_none());
        if save {
            assert_eq!(
                scicanvas_core::project_file::load(&path).unwrap().books[0].columns[0]
                    .data
                    .text_at(0),
                "123"
            );
            assert!(!app.is_dirty());
            std::fs::remove_file(path).unwrap();
        } else {
            assert_eq!(app.project, project);
            assert_eq!(app.sheets[&1].edit.as_ref().unwrap().buf, "123");
        }
    }
}

#[cfg(windows)]
#[test]
fn software_renderer_notice_only_applies_to_cpu() {
    use eframe::wgpu::DeviceType;
    assert!(renderer_notice(DeviceType::Cpu).unwrap().contains("软件渲染"));
    for kind in [
        DeviceType::DiscreteGpu,
        DeviceType::IntegratedGpu,
        DeviceType::VirtualGpu,
        DeviceType::Other,
    ] {
        assert_eq!(renderer_notice(kind), None);
    }
}

#[test]
fn delete_confirmation_requires_a_click_when_no_button_has_focus() {
    let mut app = app();
    app.confirm_delete = Some(1);
    let original = app.project.clone();
    let generation = app.history.generation();
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1280., 900.)),
        |a: &mut SciApp, c: &egui::Context| a.ui_frame(c),
    );
    for _ in 0..4 {
        h.frame(&mut app, vec![]);
    }
    assert!(h.ctx.memory(|m| m.focused().is_none()));
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    h.frame(&mut app, vec![key(egui::Key::Enter)]);
    assert_eq!(app.confirm_delete, Some(1));
    assert_eq!(app.project, original);
    assert_eq!(app.history.generation(), generation);
    h.frame(&mut app, vec![key(egui::Key::Escape)]);
    assert!(app.confirm_delete.is_none());
    assert_eq!(app.project, original);
    assert_eq!(app.history.generation(), generation);
    app.confirm_delete = Some(1);
    let mut output = h.frame(&mut app, vec![]);
    for _ in 0..3 {
        output = h.frame(&mut app, vec![]);
    }
    let delete = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "删除" => Some(t.pos),
            _ => None,
        })
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap();
    h.click(&mut app, delete + egui::vec2(5., 5.));
    assert!(app.project.book(1).is_none());
    app.run_command("edit.undo");
    assert_eq!(app.project, original);
}
