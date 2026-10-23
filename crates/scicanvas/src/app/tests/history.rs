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

#[test]
fn separate_slider_drags_are_separate_undo_steps() {
    let ctx = egui::Context::default();
    let mut app = app();
    let initial = app.project.clone();
    let mut rect = egui::Rect::NOTHING;
    let mut frame = |app: &mut SciApp, events: Vec<egui::Event>| {
        app.begin_edit("线宽");
        let before = app.project.graphs[0].first_layer().line_width;
        app.history_group = None;
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = ui.add(
                        egui::Slider::new(&mut app.project.graphs[0].first_layer_mut().line_width, 0.5..=6.0)
                            .show_value(false),
                    );
                    rect = response.rect;
                    edit_group(&response, &mut app.history_group);
                });
            },
        );
        if app.project.graphs[0].first_layer().line_width != before {
            app.history.changed();
        }
        app.finish_frame_edit(app.history_group);
        rect
    };
    let r = frame(&mut app, vec![]);
    let mut states = vec![initial];
    for (start, end) in [(0.2, 0.5), (0.6, 0.9)] {
        let at = |f: f32| egui::pos2(r.left() + r.width() * f, r.center().y);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &mut app,
            vec![egui::Event::PointerMoved(at(start)), button(at(start), true)],
        );
        frame(&mut app, vec![egui::Event::PointerMoved(at(end))]);
        frame(&mut app, vec![button(at(end), false)]);
        frame(&mut app, vec![]);
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
