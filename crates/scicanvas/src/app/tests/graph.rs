use super::*;

#[test]
fn object_creation_live_edit_drag_delete_and_auto_legend_are_undoable() {
    use plot::{ObjectAction as A, ObjectTarget as T};
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(2);
    app.explorer_focus = false;
    let initial = app.project.clone();
    app.run_command("graph.text.add");
    assert_eq!(app.project, initial, "choosing the tool is not an edit");
    let page = object_frame(&mut app, &ctx, vec![], false);
    let at = page.min + page.size() * 0.25;
    object_frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(at), object_button(at, true)],
        false,
    );
    object_frame(&mut app, &ctx, vec![object_button(at, false)], false);
    let id = app.project.graph(2).unwrap().page.annotations[0].id;
    assert!(matches!(app.object_editor, Some(ObjectEditor { target: T::Text(t), .. }) if t == id));
    roundtrip(&mut app, &initial);
    app.apply_object_action(2, A::Edit(T::Text(id)));
    object_frame(&mut app, &ctx, vec![], true);
    object_frame(&mut app, &ctx, vec![], true);
    let before_edit = app.project.clone();
    object_frame(&mut app, &ctx, vec![egui::Event::Text("CO_{2} α".into())], true);
    assert_eq!(app.project.graph(2).unwrap().page.text(id).unwrap().text, "CO_{2} α");
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
        true,
    );
    assert_eq!(
        app.project.graph(2).unwrap().page.annotations.len(),
        1,
        "Delete in the editor never deletes the object"
    );
    app.finish_edit();
    roundtrip(&mut app, &before_edit);
    app.object_editor = None;

    for target in [T::Text(id), T::Legend(0)] {
        let before = app.project.clone();
        let page = object_frame(&mut app, &ctx, vec![], false);
        let scene = scicanvas_core::scene::layout_page(app.project.graph(2).unwrap(), &app.project, &[]);
        let bounds = scene.objects.iter().find(|o| o.target == target).unwrap().bounds;
        let at =
            page.min + egui::vec2((bounds[0] + 4.0) as f32, (bounds[1] + 4.0) as f32) * (page.width() / scene.w as f32);
        object_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(at), object_button(at, true)],
            false,
        );
        for delta in [20.0, 50.0, 80.0] {
            object_frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(at + egui::vec2(delta, -20.0))],
                false,
            );
        }
        object_frame(
            &mut app,
            &ctx,
            vec![object_button(at + egui::vec2(80.0, -20.0), false)],
            false,
        );
        roundtrip(&mut app, &before);
        let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
        assert_eq!(scicanvas_core::project_file::from_scv(&saved).unwrap(), app.project);
    }
    let before = app.project.clone();
    app.run_command("graph.legend.auto");
    assert_eq!(app.project.graph(2).unwrap().first_layer().legend_position, None);
    roundtrip(&mut app, &before);
    for target in [T::Text(id), T::Legend(0)] {
        let before = app.project.clone();
        app.apply_object_action(2, A::Delete(target));
        app.finish_edit();
        match target {
            T::Text(id) => assert!(app.project.graph(2).unwrap().page.text(id).is_none()),
            T::Legend(_) => assert!(!app.project.graph(2).unwrap().first_layer().legend),
            T::ColorScale(_) | T::Shape(_) => unreachable!(),
        }
        roundtrip(&mut app, &before);
    }
}

#[test]
fn floating_dialog_moves_beyond_screen_edge_and_reset_preserves_live_changes() {
    let mut app = app();
    app.run_command("layer.contents");
    let Some(Dialog::LayerContents(edit)) = &mut app.dialog else {
        panic!()
    };
    edit.draft.legend = !edit.before.legend;
    let draft = edit.draft.clone();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0));
    let id = egui::Id::new(("scicanvas-dialog", "图层内容"));
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(screen.size()),
        |app: &mut SciApp, ctx: &egui::Context| app.ui_frame(ctx),
    )
    .with_step(0.02);

    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
    let start = egui::pos2(rect.center().x, rect.top() + 12.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.frame(&mut app, vec![egui::Event::PointerMoved(start), button(start, true)]);
    harness.frame(&mut app, vec![egui::Event::PointerMoved(start + egui::vec2(60.0, 0.0))]);
    let end = egui::pos2(screen.right() + 150.0, start.y);
    harness.frame(&mut app, vec![egui::Event::PointerMoved(end)]);
    harness.frame(&mut app, vec![button(end, false)]);
    harness.frame(&mut app, vec![]);
    let moved = ctx.memory(|m| m.area_rect(id)).unwrap();
    assert!(moved.right() > screen.right() + 100.0, "{moved:?}");
    harness.frame(&mut app, vec![]);
    assert_eq!(ctx.memory(|m| m.area_rect(id)).unwrap().min, moved.min);
    app.geoms.get_mut(&1).unwrap().rect =
        egui::Rect::from_min_size(egui::pos2(3000.0, 3000.0), egui::vec2(400.0, 300.0));
    app.ui.open_palette();
    harness.frame(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::R,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                ctrl: true,
                command: true,
                shift: true,
                ..Default::default()
            },
        }],
    );
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let reset = ctx.memory(|m| m.area_rect(id)).unwrap();
    assert!(reset.left() >= screen.left() && reset.right() <= screen.right());
    assert!(app.geoms[&1].rect.left() < app.ws.width());
    let Some(Dialog::LayerContents(edit)) = &app.dialog else {
        panic!()
    };
    assert_eq!(edit.draft, draft);
    assert_ne!(app.project, before);
    assert!(app.history.undo_label().is_some());
    roundtrip(&mut app, &before);
}

#[test]
fn docked_properties_and_layer_contents_share_sidebar_and_preserve_live_preview() {
    let mut app = app();
    let before = app.project.clone();
    let gid = app.active.unwrap();
    app.run_command("graph.props");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    crate::icons::install_loaders(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1500.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.ui_frame(ctx),
    )
    .with_step(0.04);
    macro_rules! settle {
        () => {
            for _ in 0..6 {
                harness.frame(&mut app, vec![]);
            }
        };
    }
    macro_rules! dock_left {
        ($id:expr, $y:expr) => {{
            let rect = ctx.memory(|m| m.area_rect($id)).unwrap();
            let start = egui::pos2(rect.center().x, rect.top() + 12.0);
            let end = egui::pos2(150.0, $y);
            harness.drag(start, start + egui::vec2(30.0, 0.0), end, |ctx| app.ui_frame(ctx));
            settle!();
        }};
    }
    settle!();
    let props_id = egui::Id::new("scicanvas-props");
    dock_left!(props_id, 700.0);
    assert!(dialogs::has_docks(&ctx, dialogs::DockSide::Left));
    let rect = ctx.memory(|m| m.area_rect(props_id)).unwrap();
    assert!(
        rect.right() <= app.ws.left() + 1.0,
        "props {rect:?}, workspace {:?}",
        app.ws
    );
    app.run_command("layer.contents");
    let Some(Dialog::LayerContents(edit)) = &mut app.dialog else {
        panic!()
    };
    edit.draft.legend = !edit.before.legend;
    let draft = edit.draft.clone();
    settle!();
    let contents_id = egui::Id::new(("scicanvas-dialog", "图层内容"));
    dock_left!(contents_id, 920.0);
    let props = ctx.memory(|m| m.area_rect(props_id)).unwrap();
    let contents = ctx.memory(|m| m.area_rect(contents_id)).unwrap();
    assert!(props.bottom() <= contents.top() + 1.0, "{props:?} {contents:?}");
    assert!(
        contents.right() <= app.ws.left() + 1.0 && contents.bottom() <= 980.0,
        "{contents:?} {:?}",
        app.ws
    );
    assert_eq!(app.preview_graph(gid).unwrap().page.layers()[0], draft);
    assert_ne!(app.project, before);
    assert!(app.history.undo_label().is_some());
    app.run_command("win.reset");
    settle!();
    assert!(!dialogs::has_docks(&ctx, dialogs::DockSide::Left));
    assert_eq!(app.preview_graph(gid).unwrap().page.layers()[0], draft);
    let Some(Dialog::LayerContents(edit)) = app.dialog.take() else {
        panic!()
    };
    app.apply_layer_contents(&edit).unwrap();
    roundtrip(&mut app, &before);
}

#[test]
fn layer_contents_real_controls_apply_live_and_header_close_keeps_changes() {
    let mut app = app();
    let mut p = Project::default();
    let a = p.new_book();
    let b = p.new_book();
    for id in [a, b] {
        let book = p.book_mut(id).unwrap();
        book.columns[0].data = (0..5).map(|i| i.to_string()).collect();
        book.columns[1].data = (0..5).map(|i| (i * 2).to_string()).collect();
    }
    let (gid, _) = p.plot_from_selection(a, &[], PlotType::Line).unwrap();
    let layer = p.graph(gid).unwrap().first_layer().clone();
    p.graph_mut(gid).unwrap().page.add_layer(layer);
    app.load_project(p, None);
    app.active_layers.insert(gid, 1);
    let before = app.project.clone();
    app.run_command("layer.contents");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    let positions = |out: &egui::FullOutput, label: &str| {
        crate::test_ui::text_positions(out, label)
            .into_iter()
            .map(|pos| pos + egui::vec2(4.0, 4.0))
            .collect::<Vec<_>>()
    };

    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    for label in ["添加到图层", "从图层移除", "上移", "下移", "按名称排序"] {
        let pos = positions(&output, label)[0];
        let rect = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.rect.contains(pos) => Some(r.rect),
                _ => None,
            })
            .min_by(|a, b| a.area().total_cmp(&b.area()))
            .unwrap();
        assert_eq!(rect.size(), dialogs::BUTTON, "button {label}");
    }
    harness.click(&mut app, *positions(&output, "B (Y)").last().expect("source column"));
    let output = harness.frame(&mut app, vec![]);
    harness.click(&mut app, positions(&output, "添加到图层")[0]);
    let key = scicanvas_core::plotspec::foreign_key(b, "B");
    assert_ne!(app.project, before, "controls update the project immediately");
    assert!(app.preview_graph(gid).unwrap().page.layers()[1]
        .draft()
        .y_keys
        .contains(&key));
    let output = harness.frame(&mut app, vec![]);
    assert!(!positions(&output, "Book2 / A").is_empty());
    assert!(!positions(&output, "Book2 / B").is_empty());
    let row = positions(&output, "Book2：B")[0];
    let axis = positions(&output, "左 Y")
        .into_iter()
        .min_by(|a, b| (a.y - row.y).abs().total_cmp(&(b.y - row.y).abs()))
        .unwrap();
    harness.click(&mut app, axis);
    let output = harness.frame(&mut app, vec![]);
    harness.click(&mut app, *positions(&output, "右 Y").last().unwrap());
    let output = harness.frame(&mut app, vec![]);
    harness.click(&mut app, positions(&output, "按名称排序")[0]);
    let output = harness.frame(&mut app, vec![]);
    harness.click(&mut app, positions(&output, "上移")[0]);
    let Some(Dialog::LayerContents(edit)) = &app.dialog else {
        panic!()
    };
    assert_eq!(edit.draft.draft().y_keys[0], key);
    assert!(edit.draft.styles()[&key].right_axis);
    assert_eq!(app.preview_graph(gid).unwrap().page.layers()[1], edit.draft);
    assert_ne!(app.project, before);
    let id = egui::Id::new(("scicanvas-dialog", "图层内容"));
    let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
    harness.click(&mut app, egui::pos2(rect.right() - 16.0, rect.top() + 15.0));
    assert!(app.dialog.is_none());
    assert_eq!(
        app.project.graph(gid).unwrap().first_layer(),
        before.graph(gid).unwrap().first_layer()
    );
    let layer = &app.project.graph(gid).unwrap().page.layers()[1];
    assert_eq!(layer.draft().y_keys, vec![key.clone(), "B".into()]);
    assert!(layer.styles()[&key].right_axis);
    let saved = scicanvas_core::project_file::to_json(&app.project).unwrap();
    assert_eq!(scicanvas_core::project_file::from_json(&saved).unwrap(), app.project);
    let changed = app.project.clone();
    let mut steps = 0;
    while app.project != before {
        app.run_command("edit.undo");
        steps += 1;
        assert!(steps <= 6);
    }
    for _ in 0..steps {
        app.run_command("edit.redo");
    }
    assert_eq!(app.project, changed);

    let committed = app.project.clone();
    app.run_command("layer.contents");
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    harness.click(&mut app, positions(&output, "从图层移除")[0]);
    assert_ne!(app.project, committed);
    assert_eq!(
        app.preview_graph(gid).unwrap().as_ref(),
        app.project.graph(gid).unwrap()
    );
    let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
    harness.click(&mut app, egui::pos2(rect.right() - 16.0, rect.top() + 15.0));
    assert!(app.dialog.is_none());
    roundtrip(&mut app, &committed);
    app.run_command("edit.undo");
    assert_eq!(app.project, committed);
    assert_eq!(
        app.preview_graph(gid).unwrap().as_ref(),
        app.project.graph(gid).unwrap()
    );
    let previous_undo = app.history.undo_label().map(str::to_owned);
    app.run_command("layer.contents");
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
    harness.click(&mut app, egui::pos2(rect.right() - 16.0, rect.top() + 15.0));
    assert_eq!(
        app.history.undo_label(),
        previous_undo.as_deref(),
        "closing unchanged settings adds no step"
    );
}

#[test]
fn layer_contents_rejects_stale_drafts_and_supports_removing_all_curves() {
    let mut app = app();
    app.run_command("layer.contents");
    let Some(Dialog::LayerContents(edit)) = &app.dialog else {
        panic!()
    };
    let mut edit = edit.clone();
    let gid = edit.graph;
    app.project.graph_mut(gid).unwrap().page.layers_mut()[edit.index].grid = !edit.before.grid;
    let changed = app.project.clone();
    edit.draft.legend = !edit.before.legend;
    assert!(app.apply_layer_contents(&edit).is_err());
    assert_eq!(app.project, changed);
    assert!(app.history.undo_label().is_none());
    app.run_command("layer.contents");
    let Some(Dialog::LayerContents(edit)) = &app.dialog else {
        panic!()
    };
    let mut edit = edit.clone();
    for key in edit.draft.draft().y_keys.clone() {
        edit.draft.remove_curve(&key);
    }
    app.apply_layer_contents(&edit).unwrap();
    assert!(app.project.graph(gid).unwrap().page.layers()[edit.index]
        .draft()
        .y_keys
        .is_empty());
    roundtrip(&mut app, &changed);
}

#[test]
fn layer_changes_repaint_and_close_preserves_them_until_undo() {
    let mut app = app();
    let gid = app.project.graphs[0].id;
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let render = |app: &mut SciApp| {
        let mut shapes = Vec::new();
        for _ in 0..3 {
            shapes = ctx
                .run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.graph_ui(ui, gid));
                })
                .shapes;
        }
        shapes
    };
    let original = render(&mut app);
    app.run_command("layer.contents");
    let Some(Dialog::LayerContents(edit)) = &mut app.dialog else {
        panic!()
    };
    edit.draft.remove_curve(&edit.draft.draft().y_keys[0].clone());
    assert_ne!(
        render(&mut app),
        original,
        "the plot, including its cached scene, previews the draft"
    );
    assert_eq!(app.project, before);
    assert_eq!(
        scicanvas_core::project_file::to_json(&app.project).unwrap(),
        scicanvas_core::project_file::to_json(&before).unwrap()
    );
    press_dialog_button(&mut app, "×");
    assert_ne!(app.project, before);
    assert_ne!(render(&mut app), original);
    app.run_command("edit.undo");
    assert_eq!(render(&mut app), original);
    assert_eq!(app.project, before);
}

#[test]
fn graph_analysis_curve_dialog_selects_foreign_source_and_history_is_atomic() {
    let mut app = app();
    let mut p = Project::default();
    let a = p.new_book();
    let b = p.new_book();
    for (id, slope) in [(a, 2.0), (b, 3.0)] {
        let book = p.book_mut(id).unwrap();
        book.columns[0].data = (1..13).map(|x| x.to_string()).collect();
        book.columns[1].data = (1..13).map(|x| (slope * x as f64 + 1.0).to_string()).collect();
    }
    let (gid, _) = p.plot_from_selection(a, &[], PlotType::LineSymbol).unwrap();
    p.add_curves(gid, 0, b, &["B".into()]).unwrap();
    app.load_project(p, None);
    let before = app.project.clone();
    app.run_command("analysis.curves");
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1000.0, 700.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    let at = |output: &egui::FullOutput, label: &str| {
        crate::test_ui::find_text(output, label)
            .map(|pos| pos + egui::vec2(4.0, 4.0))
            .unwrap_or_else(|| panic!("missing {label}"))
    };
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    let pos = at(&output, "Book1：B");
    for pressed in [true, false] {
        harness.frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let _ = harness.frame(&mut app, vec![]);
    let rect = ctx
        .memory(|m| m.area_rect(egui::Id::new(("scicanvas-dialog", "分析曲线"))))
        .unwrap();
    let pos = egui::pos2(rect.right() - 16.0, rect.top() + 15.0);
    for pressed in [true, false] {
        harness.frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(app.dialog.is_none());
    assert!(app.history.undo_label().is_none());
    assert_eq!(
        app.project.resolve(&app.analysis_source().unwrap()).unwrap().items[0].book_id,
        b
    );
    app.run_command("fit.linear");
    assert_eq!(app.project.book(a), before.book(a));
    assert!(app.project.book(b).unwrap().columns.len() > before.book(b).unwrap().columns.len());
    let Some(Dialog::FitReport(reports)) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].book_id, b);
    assert!((reports[0].fit.coef[1] - 3.0).abs() < 1e-10);
    roundtrip(&mut app, &before);
    app.analysis_curves.clear();
    app.anova();
    let Some(Dialog::Anova { names, .. }) = &app.dialog else {
        panic!("{}", app.status)
    };
    assert_eq!(names, &["Book1：B", "Book2：B"]);
    app.correlations();
    assert!(matches!(&app.dialog, Some(Dialog::Correlations(pairs)) if pairs.len() == 1));
}

#[test]
fn columns_from_another_workbook_join_a_graph_and_leave_it_in_one_undo_step() {
    let mut app = app();
    let b = app.project.new_book();
    let book = app.project.book_mut(b).unwrap();
    book.columns[0].data = (0..4).map(|i| i.to_string()).collect();
    book.columns[1].data = (0..4).map(|i| (i * i).to_string()).collect();
    let key = scicanvas_core::plotspec::foreign_key(b, "B");
    app.selection.insert(b, vec!["A".into(), "B".into()]);
    let before = app.project.clone();
    app.add_to_graph(b, 2);
    assert!(app
        .project
        .graph(2)
        .unwrap()
        .first_layer()
        .draft()
        .y_keys
        .contains(&key));
    assert_eq!(app.status, "已添加 1 条曲线");
    roundtrip(&mut app, &before);
    let before = app.project.clone();
    app.remove_curve(2, 0, &key);
    assert!(!app
        .project
        .graph(2)
        .unwrap()
        .first_layer()
        .draft()
        .y_keys
        .contains(&key));
    roundtrip(&mut app, &before);
    // 已在图层里的列不再记一步
    app.add_to_graph(b, 2);
    let before = app.project.clone();
    app.add_to_graph(b, 2);
    assert_eq!(app.project, before);
    assert_eq!(app.status, "这些列已在图层里");
}

#[test]
fn embedded_plot_examples_preserve_current_project_and_undo_as_one_step() {
    assert_eq!(crate::examples::ALL.len(), PlotType::ALL.len());
    for kind in PlotType::ALL {
        let example = crate::examples::ALL.iter().find(|e| e.kind == kind).unwrap();
        assert!(commands::find(example.plot_command).is_some());
        let mut app = app();
        let folder = app.project.add_folder(None).unwrap();
        app.explorer_target = Some(folder);
        app.project_path = Some(PathBuf::from("user-project.scv"));
        let before = app.project.clone();
        app.run_command(example.command);
        assert_eq!(&app.project.books[..before.books.len()], &before.books);
        assert_eq!(&app.project.graphs[..before.graphs.len()], &before.graphs);
        let book = app.project.books.last().unwrap();
        let graph = app.project.graphs.last().unwrap();
        assert_eq!(graph.first_layer().plot_type, kind);
        assert_eq!(graph.first_layer().book_id, Some(book.id));
        assert_eq!(app.project.folder_of(book.id), Some(folder));
        assert_eq!(app.project.folder_of(graph.id), Some(folder));
        assert!(book.columns.iter().any(|c| !c.comment.is_empty()));
        assert_eq!(app.active, Some(graph.id));
        assert_eq!(app.project_path, Some(PathBuf::from("user-project.scv")));
        scicanvas_core::project_file::to_scv(&app.project).unwrap();
        roundtrip(&mut app, &before);
        app.run_command(example.command);
        let names: Vec<_> = app.project.books.iter().map(|b| &b.name).collect();
        assert_ne!(names[names.len() - 1], names[names.len() - 2]);
    }
}

// 同一帧可能重复排版，命令收集保留在绘制回调外，与原测试一致。
fn ribbon_frame(
    harness: &mut crate::test_ui::Harness,
    app: &SciApp,
    state: &mut UiState,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<&'static str>) {
    let mut clicked = None;
    let output = harness
        .run(events, |ctx| ribbon::show(ctx, app, state, &app.pal, &mut clicked))
        .0;
    (output, clicked)
}

#[test]
fn ribbon_hover_example_button_is_clickable_even_without_plot_data() {
    for empty in [false, true] {
        let mut app = app();
        if empty {
            app.load_project(Project::default(), None);
        }
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.interaction.tooltip_delay = 0.0);
        let mut state = UiState::default();
        state.tab = "plot";
        let mut harness =
            crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(1800.0, 600.0)), ()).with_step(0.02);

        let at = |out: &egui::FullOutput, label: &str| {
            crate::test_ui::find_text(out, label)
                .map(|pos| pos + egui::vec2(5.0, 5.0))
                .unwrap_or_else(|| panic!("missing label {label}"))
        };
        ribbon_frame(&mut harness, &app, &mut state, vec![]);
        let tile = at(&ribbon_frame(&mut harness, &app, &mut state, vec![]).0, "折线图");
        ribbon_frame(&mut harness, &app, &mut state, vec![egui::Event::PointerMoved(tile)]);
        ribbon_frame(&mut harness, &app, &mut state, vec![]);
        let output = ribbon_frame(&mut harness, &app, &mut state, vec![]).0;
        let button = at(&output, "打开示例");
        let hovered = ribbon_frame(&mut harness, &app, &mut state, vec![egui::Event::PointerMoved(button)]).0;
        let button = at(&hovered, "打开示例");
        ribbon_frame(&mut harness, &app, &mut state, vec![egui::Event::PointerMoved(button)]);
        let mut command = None;
        for pressed in [true, false] {
            let (_, clicked) = ribbon_frame(
                &mut harness,
                &app,
                &mut state,
                vec![egui::Event::PointerButton {
                    pos: button,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            command = command.or(clicked);
        }
        assert_eq!(command, Some("example.line"));
        app.run_command(command.unwrap());
        assert_eq!(
            app.project.graphs.last().unwrap().first_layer().plot_type,
            PlotType::Line
        );
    }
}

#[test]
fn layers_builder_and_saved_project_share_history_and_headless_ui() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    theme::install(&ctx);
    crate::icons::install_loaders(&ctx);
    let mut app = app();
    let before = app.project.clone();
    app.run_command("layer.add");
    assert_eq!(app.project.graphs[0].page.layers().len(), 2);
    let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
    roundtrip(&mut app, &before);
    let two_layers = app.project.clone();
    app.run_command("layer.remove");
    roundtrip(&mut app, &two_layers);
    app.props_for = None;
    app.active = Some(1);
    let before = app.project.clone();
    app.run_command("plot.builder");
    assert_eq!(app.project, before);
    let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
    assert!(!output.shapes.is_empty());
    app.run_command("plot.apply_spec");
    roundtrip(&mut app, &before);
    let text = scicanvas_core::project_file::to_json(&app.project).unwrap();
    assert_eq!(scicanvas_core::project_file::from_json(&text).unwrap(), app.project);
}

#[test]
fn selecting_layer_is_session_state_and_commands_follow_that_layer() {
    let mut app = app();
    app.run_command("layer.add");
    app.history = History::default();
    app.select_layer(2, 0);
    app.select_layer(2, 1);
    app.finish_edit();
    assert!(app.history.undo_label().is_none());
    assert_eq!(app.analysis_source(), Some(Source::Layer { graph_id: 2, index: 1 }));
    let before = app.project.clone();
    app.run_command("graph.grid");
    assert!(!app.project.graphs[0].page.layers()[0].grid);
    assert!(app.project.graphs[0].page.layers()[1].grid);
    roundtrip(&mut app, &before);
    app.run_command("plot.bar");
    assert_eq!(app.project.graphs[0].page.layers()[0].plot_type, PlotType::LineSymbol);
    assert_eq!(app.project.graphs[0].page.layers()[1].plot_type, PlotType::Bar);
    app.run_command("layer.remove");
    assert_eq!(app.layer_index(2), 0);
    app.run_command("edit.undo");
    assert_eq!(app.project.graphs[0].page.layers().len(), 2);
}

#[test]
fn bar_variant_commands_preserve_layer_targets_pairing_and_history() {
    for command in [
        "plot.stacked_bar",
        "plot.percent_stacked_bar",
        "plot.floating_bar",
        "plot.horizontal_bar",
        "plot.area",
    ] {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        let before = app.project.clone();
        app.run_command(command);
        let id = app.active.unwrap();
        assert_ne!(id, 1);
        roundtrip(&mut app, &before);
        let one_layer = app.project.graph(id).unwrap().first_layer().clone();
        app.run_command("layer.add");
        let before = app.project.clone();
        app.run_command(if command == "plot.stacked_bar" {
            "plot.percent_stacked_bar"
        } else {
            "plot.stacked_bar"
        });
        assert_eq!(*app.project.graph(id).unwrap().first_layer(), one_layer);
        roundtrip(&mut app, &before);
    }
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    let before = app.project.clone();
    app.run_command("plot.floating_bar");
    assert_eq!(app.project, before);
    assert!(app.status.contains("成对"));
    assert!(app.history.undo_label().is_none());
}

#[test]
fn floating_baseline_dropdown_is_an_undoable_property_edit() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into(), "C".into()]);
    app.run_command("plot.floating_bar");
    let id = app.active.unwrap();
    app.props_for = Some(id);
    editor_frame(&mut app, &ctx, vec![], true);
    let output = editor_frame(&mut app, &ctx, vec![], true);
    let at = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.starts_with("C 的下端：B") => {
                Some(t.pos + egui::vec2(4.0, 4.0))
            }
            _ => None,
        })
        .unwrap();
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    editor_frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(at), click(at, true)],
        true,
    );
    editor_frame(&mut app, &ctx, vec![click(at, false)], true);
    let output = editor_frame(&mut app, &ctx, vec![], true);
    let at = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.starts_with("A ·") => Some(t.pos + egui::vec2(4.0, 4.0)),
            _ => None,
        })
        .next_back()
        .unwrap();
    let before = app.project.clone();
    editor_frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(at), click(at, true)],
        true,
    );
    editor_frame(&mut app, &ctx, vec![click(at, false)], true);
    assert_eq!(
        app.project
            .graph(id)
            .unwrap()
            .first_layer()
            .draft()
            .base_by_y
            .get("C")
            .map(String::as_str),
        Some("A")
    );
    roundtrip(&mut app, &before);
}

#[test]
fn area_opacity_slider_is_an_undoable_property_edit() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("plot.area");
    let id = app.active.unwrap();
    app.props_for = Some(id);
    editor_frame(&mut app, &ctx, vec![], true);
    open_property_section(&mut app, &ctx, "外观");
    let output = editor_frame(&mut app, &ctx, vec![], true);
    let y = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "填充透明度" => Some(t.pos.y + t.galley.size().y / 2.0),
            _ => None,
        })
        .unwrap();
    let at = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Circle(c) if (c.center.y - y).abs() < 3.0 => Some(c.center),
            _ => None,
        })
        .unwrap();
    let before = app.project.clone();
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    editor_frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(at), click(at, true)],
        true,
    );
    let end = at + egui::vec2(20.0, 0.0);
    editor_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)], true);
    editor_frame(&mut app, &ctx, vec![click(end, false)], true);
    assert!(app.project.graph(id).unwrap().first_layer().area_opacity > 0.35);
    roundtrip(&mut app, &before);
}

#[test]
fn data_label_command_and_actual_checkbox_are_undoable() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(2);
    let before = app.project.clone();
    app.run_command("graph.labels");
    assert!(app.project.graph(2).unwrap().first_layer().labels.visible);
    roundtrip(&mut app, &before);
    app.props_for = Some(2);
    editor_frame(&mut app, &ctx, vec![], true);
    open_property_section(&mut app, &ctx, "标注");
    let output = editor_frame(&mut app, &ctx, vec![], true);
    let at = crate::test_ui::text_rect(&output, "数据标签")
        .map(|rect| rect.center())
        .unwrap();
    let before = app.project.clone();
    for pressed in [true, false] {
        editor_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            true,
        );
    }
    assert!(!app.project.graph(2).unwrap().first_layer().labels.visible);
    roundtrip(&mut app, &before);
}

#[test]
fn error_role_commands_and_visibility_control_are_undoable() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["C".into()]);
    let before = app.project.clone();
    app.run_command("col.setYError");
    let b = app.project.book(1).unwrap();
    assert_eq!(b.columns[2].role, Role::YError);
    assert_eq!(b.columns[2].error_for.as_deref(), Some("B"));
    let (_, series) = app.project.graph(2).unwrap().first_layer().series(b);
    assert_eq!(series.len(), 1);
    assert_eq!(app.project.resolve(&Source::Graph(2)).unwrap().items.len(), 1);
    roundtrip(&mut app, &before);
    app.props_for = Some(2);
    editor_frame(&mut app, &ctx, vec![], true);
    open_property_section(&mut app, &ctx, "标注");
    let output = editor_frame(&mut app, &ctx, vec![], true);
    let at = crate::test_ui::text_rect(&output, "显示 X / Y 误差")
        .map(|rect| rect.center())
        .unwrap();
    let before = app.project.clone();
    let before_svg = scicanvas_core::svg::render_page(app.project.graph(2).unwrap(), &app.project);
    for pressed in [true, false] {
        editor_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            true,
        );
    }
    assert!(!app.project.graph(2).unwrap().first_layer().error_bars.visible);
    assert_ne!(
        scicanvas_core::svg::render_page(app.project.graph(2).unwrap(), &app.project),
        before_svg
    );
    roundtrip(&mut app, &before);
}

#[test]
fn scientific_title_toolbar_is_live_saved_and_undoable() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    let mut p = Project::sample();
    p.graphs[0].first_layer_mut().title = "CO2".into();
    app.load_project(p, None);
    app.props_for = Some(2);
    let before = app.project.clone();
    editor_frame(&mut app, &ctx, vec![], true);
    let output = editor_frame(&mut app, &ctx, vec![], true);
    let at_text = |output: &egui::FullOutput, text: &str| {
        crate::test_ui::text_rect(output, text)
            .map(|rect| rect.center())
            .unwrap_or_else(|| panic!("missing text: {text}"))
    };
    let click = |app: &mut SciApp, pos| {
        for pressed in [true, false] {
            editor_frame(
                app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                true,
            );
        }
    };
    click(&mut app, at_text(&output, "CO2"));
    let id = ctx.memory(|m| m.focused()).unwrap();
    let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
        egui::text::CCursor::new(2),
        egui::text::CCursor::new(3),
    )));
    state.store(&ctx, id);
    let output = editor_frame(&mut app, &ctx, vec![], true);
    click(&mut app, at_text(&output, "x₂"));
    let layer = app.project.graphs[0].first_layer();
    assert_eq!(layer.title, "CO_{2}");
    assert!(layer.rich_text);
    let svg = scicanvas_core::svg::render_page(&app.project.graphs[0], &app.project);
    assert!(svg.contains("aria-label=\"CO2\""));
    let saved = scicanvas_core::project_file::to_json(&app.project).unwrap();
    assert_eq!(scicanvas_core::project_file::from_json(&saved).unwrap(), app.project);
    roundtrip(&mut app, &before);

    let before_symbol = app.project.clone();
    let output = editor_frame(&mut app, &ctx, vec![], true);
    click(&mut app, at_text(&output, "Ω"));
    let output = editor_frame(&mut app, &ctx, vec![], true);
    click(&mut app, at_text(&output, "α"));
    assert_eq!(app.project.graphs[0].first_layer().title, "CO_{α}");
    roundtrip(&mut app, &before_symbol);
}

#[test]
fn blank_graphs_can_be_bound_to_a_book_and_examples_use_the_default_page() {
    let mut app = app();
    app.run_command("graph.new");
    let gid = app.active.unwrap();
    assert_eq!(app.project.graph(gid).unwrap().first_layer().book_id, None);
    // 绘图属性里选择工作簿：与界面相同，用该表的默认映射
    let book = app.project.book(1).unwrap();
    let defaults = scicanvas_core::plotspec::PlotSpec::from_selection(book, &[], PlotType::LineSymbol).ok();
    let layer = app.project.graph_mut(gid).unwrap().first_layer_mut();
    layer.spec = layer.spec.rebound(1, defaults.as_ref());
    let graph = app.project.graph(gid).unwrap();
    let (_, series) = graph.first_layer().series(app.project.book(1).unwrap());
    assert!(!series.is_empty() && series.iter().all(|s| !s.points.is_empty()));

    // 打开示例：页面与新建图形相同（A4 宽），不沿用示例文件里的旧 89 mm
    app.run_command("example.line");
    let gid = app.active.unwrap();
    let page = &app.project.graph(gid).unwrap().page;
    let default = scicanvas_core::page::Page::default();
    assert_eq!((page.width_mm, page.height_mm), (default.width_mm, default.height_mm));
}

#[test]
fn layer_placement_control_is_undoable() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.props_for = Some(2);
    let before = app.project.clone();
    editor_frame(&mut app, &ctx, vec![], true);
    open_property_section(&mut app, &ctx, "页面与图层");
    let output = editor_frame(&mut app, &ctx, vec![], true);
    let at = crate::test_ui::find_text(&output, "自定义图层位置")
        .map(|pos| pos + egui::vec2(4.0, 4.0))
        .unwrap();
    for pressed in [true, false] {
        editor_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            true,
        );
    }
    assert!(app.project.graphs[0].first_layer().placement.is_some());
    roundtrip(&mut app, &before);
}

#[test]
fn properties_y_toggle_keeps_curve_configuration_and_undo_history() {
    let mut app = app();
    let layer = app.project.graph_mut(2).unwrap().first_layer_mut();
    layer.edit_draft(|draft| draft.x_by_y.insert("B".into(), "A".into()));
    layer.edit_draft(|draft| draft.base_by_y.insert("B".into(), "C".into()));
    layer.edit_draft(|draft| draft.row_by_y.insert("B".into(), Some((1, 3))));
    layer.edit_styles(|styles| {
        styles.insert(
            "B".into(),
            scicanvas_core::page::CurveStyle {
                right_axis: true,
                color: Some([80, 40, 120]),
                ..Default::default()
            },
        )
    });
    let before = app.project.clone();
    app.props_for = Some(2);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1500.0, 1300.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            app.history_group = None;
            app.props_window(ctx);
            app.finish_frame_edit(app.history_group);
        },
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "B · 电压 A");
    let off = app.project.clone();
    assert!(!off.graph(2).unwrap().first_layer().draft().y_keys.contains(&"B".into()));
    let layer = off.graph(2).unwrap().first_layer();
    let original = before.graph(2).unwrap().first_layer();
    assert_eq!(layer.styles(), original.styles());
    assert_eq!(layer.draft().x_by_y, original.draft().x_by_y);
    assert_eq!(layer.draft().base_by_y, original.draft().base_by_y);
    assert_eq!(layer.draft().row_by_y, original.draft().row_by_y);
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "B · 电压 A");
    let after = app.project.clone();
    let mut expected = before.clone();
    expected
        .graph_mut(2)
        .unwrap()
        .first_layer_mut()
        .spec
        .edit_draft(|draft| draft.y_keys = vec!["C".into(), "B".into()]);
    assert_eq!(after, expected);
    let json = scicanvas_core::project_file::to_json(&after).unwrap();
    assert_eq!(scicanvas_core::project_file::from_json(&json).unwrap(), after);
    app.run_command("edit.undo");
    assert_eq!(app.project, off);
    app.run_command("edit.undo");
    assert_eq!(app.project, before);
    app.run_command("edit.redo");
    app.run_command("edit.redo");
    assert_eq!(app.project, after);
}

#[test]
fn right_y_axis_range_and_format_controls_are_live_and_undoable() {
    use scicanvas_core::{format::NumberFormat, page::CurveStyle};
    let mut app = app();
    let layer = app.project.graph_mut(2).unwrap().first_layer_mut();
    let key = layer.draft().y_keys[0].clone();
    layer.set_plot_style(
        &key,
        Some(CurveStyle {
            right_axis: true,
            ..Default::default()
        }),
    );
    let before = app.project.clone();
    app.props_for = Some(2);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1500.0, 1300.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            app.history_group = None;
            app.props_window(ctx);
            app.finish_frame_edit(app.history_group);
        },
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    assert_eq!(
        app.project, before,
        "opening the properties panel does not edit the graph"
    );
    let output = harness.frame(&mut app, vec![]);
    let right_range = crate::test_ui::text_rect(&output, "右 Y 范围").unwrap();
    assert!(crate::test_ui::text_rect(&output, "右 Y 刻度格式").is_some());
    let at = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "自动" => {
                Some((text.pos + egui::vec2(5.0, 5.0), text.pos.y + text.galley.size().y / 2.0))
            }
            _ => None,
        })
        .min_by(|a, b| {
            (a.1 - right_range.center().y)
                .abs()
                .total_cmp(&(b.1 - right_range.center().y).abs())
        })
        .map(|(position, _)| position)
        .unwrap();
    let before_range = app.project.clone();
    harness.click(&mut app, at);
    assert!(!app.project.graph(2).unwrap().first_layer().y2_auto);
    roundtrip(&mut app, &before_range);
    assert!(!app.project.graph(2).unwrap().first_layer().y2_auto);

    let before_format = app.project.clone();
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "跟随左 Y");
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "科学计数");
    assert_eq!(
        app.project.graph(2).unwrap().first_layer().y2_format,
        Some(NumberFormat::Scientific { places: 2 })
    );
    roundtrip(&mut app, &before_format);
    assert_eq!(
        app.project.graph(2).unwrap().first_layer().y2_format,
        Some(NumberFormat::Scientific { places: 2 })
    );
}

#[test]
fn curve_point_style_properties_map_columns_and_undo_the_changes() {
    use scicanvas_core::page::SymbolFill;
    let mut app = app();
    let category = {
        let book = app.project.book_mut(1).unwrap();
        let key = book.push_column(Role::Ignore);
        let rows = book.nrows();
        let column = book.columns.iter_mut().find(|column| column.key == key).unwrap();
        column.long_name = "批次".into();
        column.data = (scicanvas_core::ColumnData::Category {
            codes: (0..rows).map(|row| (row % 2) as u32).collect(),
            levels: vec!["A".into(), "B".into()],
            ordered: true,
        })
        .into();
        key
    };
    app.props_for = Some(2);
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1500.0, 3000.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            app.history_group = None;
            app.props_window(ctx);
            app.finish_frame_edit(app.history_group);
        },
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "曲线样式");
    let output = harness.frame(&mut app, vec![]);
    let row = crate::test_ui::text_rect(&output, "颜色映射列").unwrap();
    let none = crate::test_ui::text_positions(&output, "无")
        .into_iter()
        .min_by(|a, b| (a.y - row.center().y).abs().total_cmp(&(b.y - row.center().y).abs()))
        .unwrap();
    harness.click(&mut app, none + egui::vec2(5.0, 5.0));
    let popup = harness.frame(&mut app, vec![]);
    let at = popup
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "电压 B" => {
                Some((shape.clip_rect.area(), text.pos + egui::vec2(5., 5.)))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap()
        .1;
    harness.click(&mut app, at);
    assert_eq!(
        app.project
            .graph(2)
            .unwrap()
            .first_layer()
            .plot_style("B")
            .unwrap()
            .color_column
            .as_deref(),
        Some("C")
    );
    roundtrip(&mut app, &before);

    let before_size = app.project.clone();
    let output = harness.frame(&mut app, vec![]);
    let row = crate::test_ui::text_rect(&output, "大小映射列").unwrap();
    let none = crate::test_ui::text_positions(&output, "无")
        .into_iter()
        .min_by(|a, b| (a.y - row.center().y).abs().total_cmp(&(b.y - row.center().y).abs()))
        .unwrap();
    harness.click(&mut app, none + egui::vec2(5.0, 5.0));
    let popup = harness.frame(&mut app, vec![]);
    let at = popup
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "电压 B" => {
                Some((shape.clip_rect.area(), text.pos + egui::vec2(5., 5.)))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap()
        .1;
    harness.click(&mut app, at);
    assert_eq!(
        app.project
            .graph(2)
            .unwrap()
            .first_layer()
            .plot_style("B")
            .unwrap()
            .size_column
            .as_deref(),
        Some("C")
    );
    roundtrip(&mut app, &before_size);

    let before_symbol = app.project.clone();
    let output = harness.frame(&mut app, vec![]);
    let row = crate::test_ui::text_rect(&output, "形状映射列").unwrap();
    let none = crate::test_ui::text_positions(&output, "无")
        .into_iter()
        .min_by(|a, b| (a.y - row.center().y).abs().total_cmp(&(b.y - row.center().y).abs()))
        .unwrap();
    harness.click(&mut app, none + egui::vec2(5.0, 5.0));
    let popup = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &popup, "批次");
    assert_eq!(
        app.project
            .graph(2)
            .unwrap()
            .first_layer()
            .plot_style("B")
            .unwrap()
            .symbol_column
            .as_deref(),
        Some(category.as_str())
    );
    roundtrip(&mut app, &before_symbol);

    let before_fill = app.project.clone();
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "跟随曲线颜色");
    let popup = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &popup, "空心");
    assert_eq!(
        app.project
            .graph(2)
            .unwrap()
            .first_layer()
            .plot_style("B")
            .unwrap()
            .symbol_fill,
        Some(SymbolFill::Hollow)
    );
    roundtrip(&mut app, &before_fill);
}

#[test]
fn applying_a_graph_template_adds_one_undoable_graph() {
    let mut app = app();
    let template =
        scicanvas_core::project_file::graph_template_from_graph(app.project.graph(2).unwrap(), &app.project).unwrap();
    let target_id = app.project.alloc_id();
    let mut book = app.project.book(1).unwrap().clone();
    book.id = target_id;
    book.name = "TemplateTarget".into();
    app.project.books.push(book);
    let before = app.project.clone();

    let graph_id = app.apply_graph_template_to_book(template, target_id).unwrap();
    let graph = app.project.graph(graph_id).unwrap();
    assert_eq!(graph.first_layer().book_id, Some(target_id));
    assert_eq!(
        graph.first_layer().draft().y_keys,
        before.graph(2).unwrap().first_layer().draft().y_keys
    );
    roundtrip(&mut app, &before);
}

#[test]
fn adding_a_reference_line_is_undoable_and_redoable() {
    let mut app = app();
    app.active = Some(2);
    let before = app.project.clone();
    app.run_command("graph.reference.horizontal");
    assert_eq!(app.project.graph(2).unwrap().first_layer().reference_lines.len(), 1);
    assert_eq!(
        app.project.graph(2).unwrap().first_layer().reference_lines[0].axis,
        scicanvas_core::page::ReferenceAxis::YLeft
    );
    roundtrip(&mut app, &before);
    assert_eq!(app.project.graph(2).unwrap().first_layer().reference_lines.len(), 1);
}

#[test]
fn reference_line_properties_add_and_remove_live_with_history() {
    let mut app = app();
    app.props_for = Some(2);
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1500.0, 1300.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            app.history_group = None;
            app.props_window(ctx);
            app.finish_frame_edit(app.history_group);
        },
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "坐标轴");
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "标注");
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "＋水平");
    assert_eq!(app.project.graph(2).unwrap().first_layer().reference_lines.len(), 1);
    roundtrip(&mut app, &before);

    let before_remove = app.project.clone();
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "删除");
    assert!(app.project.graph(2).unwrap().first_layer().reference_lines.is_empty());
    roundtrip(&mut app, &before_remove);
    assert!(app.project.graph(2).unwrap().first_layer().reference_lines.is_empty());
}

#[test]
fn reference_manager_deletes_both_orientations_with_undo() {
    for command in ["graph.reference.vertical", "graph.reference.horizontal"] {
        let mut app = app();
        app.active = Some(2);
        app.run_command(command);
        let before = app.project.clone();
        app.run_command("graph.reference.manage");
        assert_eq!(app.reference_for, Some((2, 0)));
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.);
        let mut h = crate::test_ui::Harness::new(
            ctx,
            Some(egui::vec2(900., 700.)),
            |a: &mut SciApp, c: &egui::Context| {
                a.history_group = None;
                a.reference_window(c);
                a.finish_frame_edit(a.history_group);
            },
        );
        for _ in 0..4 {
            h.frame(&mut app, vec![]);
        }
        let out = h.frame(&mut app, vec![]);
        h.click_text(&mut app, &out, "删除");
        assert!(app.project.graph(2).unwrap().first_layer().reference_lines.is_empty());
        roundtrip(&mut app, &before);
    }
}

#[test]
fn builder_edits_keep_draft_mapping_and_create_with_the_existing_undo_transaction() {
    for kind in [PlotType::LineSymbol, PlotType::FloatingBar] {
        let mut app = app();
        app.active = Some(1);
        app.run_command("plot.builder");
        let draft = app.plot_draft.as_mut().unwrap();
        draft.set_plot_type(kind).unwrap();
        draft.x_by_y.insert("B".into(), "A".into());
        draft.row_by_y.insert("B".into(), Some((0, 2)));
        draft.base_by_y.insert("hidden".into(), String::new());
        draft.row_by_y.insert("hidden".into(), None);
        let before_draft = draft.clone();
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut harness = crate::test_ui::Harness::new(
            ctx,
            Some(egui::vec2(1400.0, 1200.0)),
            |app: &mut SciApp, ctx: &egui::Context| {
                if crate::builder::show(ctx, &app.project, &mut app.plot_draft) {
                    app.run_command("plot.apply_spec");
                }
            },
        )
        .with_step(0.02);
        harness.frame(&mut app, vec![]);
        harness.frame(&mut app, vec![]);
        let output = harness.frame(&mut app, vec![]);
        assert_eq!(app.plot_draft.as_ref(), Some(&before_draft));
        let key = if kind == PlotType::FloatingBar { "C" } else { "B" };
        let name = if key == "C" { "C · 电压 B" } else { "B · 电压 A" };
        let pos = crate::test_ui::text_positions(&output, name).last().copied().unwrap() + egui::vec2(4.0, 4.0);
        harness.click(&mut app, pos);
        assert!(!app.plot_draft.as_ref().unwrap().y_keys.contains(&key.into()));
        assert_eq!(app.project, before);
        let output = harness.frame(&mut app, vec![]);
        let pos = crate::test_ui::text_positions(&output, name).last().copied().unwrap() + egui::vec2(4.0, 4.0);
        harness.click(&mut app, pos);
        let mut expected_draft = before_draft.clone();
        expected_draft.y_keys.retain(|k| k != key);
        expected_draft.y_keys.push(key.into());
        assert_eq!(app.plot_draft.as_ref(), Some(&expected_draft));
        if kind == PlotType::FloatingBar {
            let output = harness.frame(&mut app, vec![]);
            harness.click_text(&mut app, &output, "C 的下端：B");
            let output = harness.frame(&mut app, vec![]);
            let pos = crate::test_ui::text_positions(&output, "A · 时间")
                .last()
                .copied()
                .unwrap()
                + egui::vec2(4.0, 4.0);
            harness.click(&mut app, pos);
            expected_draft.base_by_y.insert("C".into(), "A".into());
            assert_eq!(app.plot_draft.as_ref(), Some(&expected_draft));
        }
        assert_eq!(app.project, before);
        let output = harness.frame(&mut app, vec![]);
        harness.click_text(&mut app, &output, "创建图形");
        assert!(app.plot_draft.is_none());
        let mut expected = before.clone();
        expected.plot_spec(expected_draft).unwrap();
        assert_eq!(app.project, expected);
        roundtrip(&mut app, &before);
        let json = scicanvas_core::project_file::to_json(&app.project).unwrap();
        assert_eq!(
            scicanvas_core::project_file::to_json(&scicanvas_core::project_file::from_json(&json).unwrap()).unwrap(),
            json
        );
    }
}

#[test]
fn mapped_area_color_is_live_undoable_and_saved_without_a_format_change() {
    use scicanvas_core::{project_file, scene, svg};
    let mut app = app();
    let layer = app.project.graph_mut(2).unwrap().first_layer_mut();
    layer.set_plot_type(PlotType::Area).unwrap();
    layer.edit_draft(|draft| draft.y_keys = vec!["B".into()]);
    app.props_for = Some(2);
    let before = app.project.clone();
    let before_svg = svg::render_page(app.project.graph(2).unwrap(), &app.project);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1500.0, 3000.0)),
        |app: &mut SciApp, ctx: &egui::Context| {
            app.history_group = None;
            app.props_window(ctx);
            app.finish_frame_edit(app.history_group);
        },
    )
    .with_step(0.02);
    harness.frame(&mut app, vec![]);
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "曲线样式");
    let output = harness.frame(&mut app, vec![]);
    let row = crate::test_ui::text_rect(&output, "颜色映射列").unwrap();
    let pick = crate::test_ui::text_positions(&output, "无")
        .into_iter()
        .min_by(|a, b| (a.y - row.center().y).abs().total_cmp(&(b.y - row.center().y).abs()))
        .unwrap();
    harness.click(&mut app, pick + egui::vec2(5.0, 5.0));
    let popup = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &popup, "电压 B");
    let graph = app.project.graph(2).unwrap();
    let sc = scene::layout_page(graph, &app.project, &[]);
    let fill_colors: std::collections::HashSet<_> = sc
        .items
        .iter()
        .filter_map(|it| match &it.prim {
            scene::Prim::Triangles { color, .. } if it.clip.is_some() => Some(*color),
            _ => None,
        })
        .collect();
    let outline_colors: std::collections::HashSet<_> = sc
        .items
        .iter()
        .flat_map(|it| match &it.prim {
            scene::Prim::ColoredSegments { segments, .. } if it.clip.is_some() => {
                segments.iter().map(|s| s.color).collect::<Vec<_>>()
            }
            _ => Vec::new(),
        })
        .collect();
    assert!(fill_colors.len() > 1);
    assert_eq!(fill_colors, outline_colors);
    let after = app.project.clone();
    let after_svg = svg::render_page(graph, &app.project);
    assert_ne!(before_svg, after_svg);
    app.run_command("edit.undo");
    assert_eq!(app.project, before);
    assert_eq!(
        svg::render_page(app.project.graph(2).unwrap(), &app.project),
        before_svg
    );
    app.run_command("edit.redo");
    assert_eq!(app.project, after);
    assert_eq!(svg::render_page(app.project.graph(2).unwrap(), &app.project), after_svg);
    let restored = project_file::from_scv(&project_file::to_scv(&app.project).unwrap()).unwrap();
    assert_eq!(restored, app.project);
    assert_eq!(restored.version, scicanvas_core::project_file::CURRENT_VERSION);
    assert_eq!(svg::render_page(restored.graph(2).unwrap(), &restored), after_svg);
}

#[test]
fn batch_plot_controls_return_selected_workbook() {
    let mut app = app();
    let target = app.project.new_book();
    app.open_batch_plot();
    let mut dialog = app.dialog.take().unwrap();
    let project = app.project.clone();
    let pal = app.pal;
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(900.0, 700.0)),
        |dialog: &mut Dialog, ctx: &egui::Context| dialogs::show(ctx, dialog, &pal, &project),
    );
    let mut out = h.frame(&mut dialog, vec![]);
    for _ in 0..2 {
        out = h.frame(&mut dialog, vec![]);
    }
    let name = project.book(target).unwrap().name.clone();
    out = h.click_text(&mut dialog, &out, &name);
    let position = crate::test_ui::find_text(&out, "生成图形").unwrap() + egui::vec2(5.0, 5.0);
    h.frame_value(&mut dialog, vec![egui::Event::PointerMoved(position)]);
    h.frame_value(&mut dialog, vec![crate::test_ui::button(position, true)]);
    let action = h.frame_value(&mut dialog, vec![crate::test_ui::button(position, false)]);
    assert!(matches!(action, dialogs::Action::BatchPlot(2, books) if books == vec![target]));
}

#[test]
fn batch_export_controls_return_selected_graphs_and_format() {
    let mut app = app();
    let first = app.project.graphs[0].id;
    let mut second = app.project.graphs[0].clone();
    second.id = app.project.alloc_id();
    second.name = "Other Graph".into();
    app.project.graphs.push(second);
    app.open_batch_export();
    let mut dialog = app.dialog.take().unwrap();
    let project = app.project.clone();
    let pal = app.pal;
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut h = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(900.0, 700.0)),
        |dialog: &mut Dialog, ctx: &egui::Context| dialogs::show(ctx, dialog, &pal, &project),
    );
    let mut out = h.frame(&mut dialog, vec![]);
    for _ in 0..2 {
        out = h.frame(&mut dialog, vec![]);
    }
    out = h.click_text(&mut dialog, &out, "Other Graph");
    out = h.click_text(&mut dialog, &out, "TIFF");
    let position = crate::test_ui::find_text(&out, "选择目录并导出…").unwrap() + egui::vec2(5.0, 5.0);
    h.frame_value(&mut dialog, vec![egui::Event::PointerMoved(position)]);
    h.frame_value(&mut dialog, vec![crate::test_ui::button(position, true)]);
    let action = h.frame_value(&mut dialog, vec![crate::test_ui::button(position, false)]);
    assert!(
        matches!(action, dialogs::Action::BatchExport(graphs, dialogs::BatchFormat::Tiff, 300) if graphs == vec![first])
    );
}

#[test]
fn supplement_builder_color_choice_defaults_then_preserves_manual_palette() {
    use scicanvas_core::colormap::ColorMap;
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
    app.run_command("plot.builder");
    assert_eq!(app.plot_draft.as_ref().unwrap().plot_type, PlotType::LineSymbol);
    let before = app.project.clone();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1400., 1200.)),
        |app: &mut SciApp, ctx: &egui::Context| {
            if crate::builder::show(ctx, &app.project, &mut app.plot_draft) {
                app.run_command("plot.apply_spec");
            }
        },
    )
    .with_step(0.02);
    for _ in 0..5 {
        harness.frame(&mut app, vec![]);
    }
    let output = harness.frame(&mut app, vec![]);
    let scroll = crate::test_ui::find_text(&output, "图库 · 当前数据").unwrap() + egui::vec2(100., 100.);
    let mut selected = false;
    for _ in 0..55 {
        let output = harness.frame(&mut app, vec![]);
        let tile = output.shapes.iter().find_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                let at = text.pos + egui::vec2(80., 80.);
                if text.galley.job.text == "颜色映射散点图" && shape.clip_rect.contains(at) {
                    return Some(at);
                }
            }
            None
        });
        if let Some(at) = tile {
            harness.click(&mut app, at);
            selected = true;
            break;
        }
        harness.frame(
            &mut app,
            vec![
                egui::Event::PointerMoved(scroll),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0., -160.),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        for _ in 0..3 {
            harness.frame(&mut app, vec![]);
        }
    }
    assert!(selected, "color scatter gallery tile is reachable");
    assert_eq!(app.plot_draft.as_ref().unwrap().plot_type, PlotType::ColorScatter);
    assert_eq!(app.plot_draft.as_ref().unwrap().color_map, ColorMap::Viridis);
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "经典分类色");
    for _ in 0..3 {
        harness.frame(&mut app, vec![]);
    }
    assert_eq!(app.plot_draft.as_ref().unwrap().color_map, ColorMap::Classic);
    let output = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &output, "创建图形");
    assert!(app.plot_draft.is_none());
    assert_eq!(
        app.project.graphs.last().unwrap().first_layer().color_map,
        ColorMap::Classic
    );
    roundtrip(&mut app, &before);
    let id = app.project.graphs.last().unwrap().id;
    app.active = Some(id);
    app.open_plot_builder();
    assert_eq!(app.plot_draft.as_ref().unwrap().color_map, ColorMap::Classic);
}

fn open_property_section(app: &mut SciApp, ctx: &egui::Context, title: &str) {
    ctx.style_mut(|style| style.animation_time = 0.0);
    for label in ["坐标轴", title] {
        let output = editor_frame(app, ctx, vec![], true);
        let at = crate::test_ui::text_rect(&output, label).unwrap().center();
        for pressed in [true, false] {
            editor_frame(
                app,
                ctx,
                vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, pressed)],
                true,
            );
        }
    }
    editor_frame(app, ctx, vec![], true);
}

#[test]
fn u3_color_mapping_controls_are_saved_and_undoable() {
    use scicanvas_core::{colormap::ColorMap, project_file, scene};
    let mut app = app();
    app.active = Some(1);
    app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
    app.run_command("plot.color_scatter");
    let gid = app.active.unwrap();
    assert!(app.project.graph(gid).is_some(), "{}", app.status);
    app.props_for = Some(gid);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(egui::vec2(1400., 2200.)),
        |app: &mut SciApp, ctx: &egui::Context| {
            app.props_window(ctx);
            app.finish_edit();
        },
    );
    for _ in 0..4 {
        harness.frame(&mut app, vec![]);
    }
    let before = app.project.clone();
    let out = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &out, "Viridis（连续）");
    let out = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &out, "经典分类色");
    assert_eq!(
        app.project.graph(gid).unwrap().first_layer().color_map,
        ColorMap::Classic
    );
    let after = app.project.clone();
    app.undo();
    assert_eq!(app.project, before);
    app.redo();
    assert_eq!(app.project, after);
    assert_eq!(
        project_file::from_scv(&project_file::to_scv(&app.project).unwrap()).unwrap(),
        app.project
    );
    let has_scale = |app: &SciApp| {
        scene::layout_page(app.project.graph(gid).unwrap(), &app.project, &[])
            .objects
            .iter()
            .any(|o| o.target == scicanvas_core::scene::ObjectTarget::ColorScale(0))
    };
    assert!(has_scale(&app));
    let out = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &out, "显示色阶条");
    assert!(!has_scale(&app));
    let out = harness.frame(&mut app, vec![]);
    harness.click_text(&mut app, &out, "显示色阶条");
    assert!(has_scale(&app));
}

#[test]
fn axis_style_auto_initial_value_drag_history_and_fixed_spacing_controls() {
    let mut app = app();
    app.props_for = Some(2);
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.);
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1800., 2600.)),
        |a: &mut SciApp, c: &egui::Context| {
            a.history_group = None;
            a.props_window(c);
            a.finish_frame_edit(a.history_group);
        },
    )
    .with_step(0.02);
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    let value = ctx
        .data(|d| d.get_temp::<(egui::Rect, bool, f32)>(egui::Id::new(("axis-style-value", "tick-font"))))
        .unwrap();
    let automatic = (scicanvas_core::scene::TICK_FONT * app.project.graph(2).unwrap().page.scale()
        / scicanvas_core::scene::pt_to_units(1.)) as f32;
    assert!(!value.1);
    assert_eq!(value.2, automatic);
    let toggle = ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("axis-style-auto", "tick-font"))))
        .unwrap();
    h.click(&mut app, toggle.center());
    assert_eq!(
        app.project.graph(2).unwrap().first_layer().axis_style.tick_font_pt,
        Some(automatic)
    );
    let before = app.project.clone();
    let value = ctx
        .data(|d| d.get_temp::<(egui::Rect, bool, f32)>(egui::Id::new(("axis-style-value", "tick-font"))))
        .unwrap();
    assert!(value.1);
    let start = value.0.center();
    h.frame(&mut app, vec![egui::Event::PointerMoved(start)]);
    h.frame(&mut app, vec![crate::test_ui::button(start, true)]);
    for delta in [12., 24., 36.] {
        h.frame(&mut app, vec![egui::Event::PointerMoved(start + egui::vec2(delta, 0.))]);
    }
    h.frame(
        &mut app,
        vec![crate::test_ui::button(start + egui::vec2(36., 0.), false)],
    );
    assert_ne!(app.project, before);
    roundtrip(&mut app, &before);
    let g = app.project.graph_mut(2).unwrap().first_layer_mut();
    g.x_scale = Some(scicanvas_core::ticks::AxisScale::Logit);
    g.x_ticks.major_step = Some(0.1);
    h.frame(&mut app, vec![]);
    let (_, enabled) = ctx
        .data(|d| d.get_temp::<(egui::Rect, bool)>(egui::Id::new(("axis-spacing-value", "X"))))
        .unwrap();
    assert!(!enabled);
}

#[test]
fn linked_axis_spacing_controls_follow_effective_scale_inheritance() {
    use scicanvas_core::{scene, ticks::AxisScale};
    let mut app = app();
    let graph = app.project.graph_mut(2).unwrap();
    graph.first_layer_mut().x_auto = false;
    graph.first_layer_mut().x_min = 1.;
    graph.first_layer_mut().x_max = 1e6;
    let mut layer = graph.first_layer().clone();
    layer.link_x = Some(0);
    layer.x_ticks.major_step = Some(1.);
    graph.page.add_layer(layer);
    graph.page.columns = 2;
    app.select_layer(2, 1);
    app.props_for = Some(2);
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.);
    let mut h = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1800., 2600.)),
        |a: &mut SciApp, c: &egui::Context| {
            a.history_group = None;
            a.props_window(c);
            a.finish_frame_edit(a.history_group);
        },
    )
    .with_step(0.02);
    for _ in 0..5 {
        h.frame(&mut app, vec![]);
    }
    let field = || {
        ctx.data(|d| d.get_temp::<(egui::Rect, bool)>(egui::Id::new(("axis-spacing-value", "X"))))
            .unwrap()
    };
    assert!(!field().1);
    app.project.graph_mut(2).unwrap().page.layers_mut()[1]
        .set_axis_scale(true, AxisScale::Log10)
        .unwrap();
    for _ in 0..3 {
        h.frame(&mut app, vec![]);
    }
    assert!(field().1);
    h.click(&mut app, field().0.center());
    h.frame(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }],
    );
    h.frame(&mut app, vec![egui::Event::Text("2".into())]);
    h.frame(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(
        app.project.graph(2).unwrap().page.layers()[1].x_ticks.major_step,
        Some(2.)
    );
    let scene = scene::layout_page(app.project.graph(2).unwrap(), &app.project, &[]);
    assert_eq!(scene.layers[1].axes.x.ticks, [1., 100., 1e4, 1e6]);
}

fn open_example_test_gallery(h: &mut crate::test_ui::Harness, app: &SciApp, state: &mut UiState) {
    for _ in 0..4 {
        ribbon_frame(h, app, state, vec![]);
    }
    if state.collapsed && !state.flyout {
        let out = ribbon_frame(h, app, state, vec![]).0;
        let at = crate::test_ui::find_text(&out, "绘图").unwrap() + egui::vec2(5., 5.);
        for pressed in [true, false] {
            ribbon_frame(
                h,
                app,
                state,
                vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, pressed)],
            );
        }
    }
    for _ in 0..3 {
        ribbon_frame(h, app, state, vec![]);
    }
    let at = h
        .ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("ribbon-gallery-menu-rect")))
        .unwrap()
        .center();
    for pressed in [true, false] {
        ribbon_frame(
            h,
            app,
            state,
            vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, pressed)],
        );
    }
    ribbon_frame(
        h,
        app,
        state,
        vec![
            egui::Event::PointerMoved(at + egui::vec2(40., 100.)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -1800.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..12 {
        ribbon_frame(h, app, state, vec![]);
    }
}

#[test]
fn dropdown_examples_and_collapsed_gallery_preserve_full_clicks() {
    for (width, kind, id, collapsed) in [
        (1800., PlotType::Pie, "pie", false),
        (900., PlotType::PolarScatter, "polar_scatter", false),
        (900., PlotType::Pie, "pie", true),
    ] {
        let mut app = app();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| {
            s.animation_time = 0.;
            s.interaction.tooltip_delay = 0.;
        });
        let mut h = crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(width, 820.)), ()).with_step(0.02);
        let mut state = UiState::default();
        state.tab = "plot";
        state.collapsed = collapsed;
        let plot = format!("plot.{id}");
        open_example_test_gallery(&mut h, &app, &mut state);
        let tile = ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("gallery-grid-tile", plot.as_str()))))
            .unwrap();
        ribbon_frame(&mut h, &app, &mut state, vec![egui::Event::PointerMoved(tile.center())]);
        let button = ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("gallery-example-button", plot.as_str()))))
            .unwrap()
            .center();
        let mut command = None;
        for pressed in [true, false] {
            let (_, clicked) = ribbon_frame(
                &mut h,
                &app,
                &mut state,
                vec![
                    egui::Event::PointerMoved(button),
                    crate::test_ui::button(button, pressed),
                ],
            );
            command = command.or(clicked);
        }
        assert_eq!(
            command,
            Some(crate::examples::ALL.iter().find(|e| e.kind == kind).unwrap().command)
        );
        let count = app.project.books.len();
        app.run_command(command.unwrap());
        assert_eq!(app.project.books.len(), count + 1);
        assert_eq!(app.project.graphs.last().unwrap().first_layer().plot_type, kind);
        open_example_test_gallery(&mut h, &app, &mut state);
        let tile = ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("gallery-grid-tile", plot.as_str()))))
            .unwrap();
        let at = tile.center() - egui::vec2(12., 0.);
        let mut command = None;
        for pressed in [true, false] {
            let (_, clicked) = ribbon_frame(
                &mut h,
                &app,
                &mut state,
                vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, pressed)],
            );
            command = command.or(clicked);
        }
        assert_eq!(command, Some(plot.as_str()));
        if collapsed {
            open_example_test_gallery(&mut h, &app, &mut state);
            let at = egui::pos2(width - 10., 810.);
            ribbon_frame(
                &mut h,
                &app,
                &mut state,
                vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, true)],
            );
            assert!(!state.flyout);
        }
    }
}

#[test]
fn collapsed_overflow_menu_keeps_commands_alive() {
    let mut app = app();
    app.active = Some(2);
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.);
    let mut h = crate::test_ui::Harness::new(ctx, Some(egui::vec2(900., 820.)), ()).with_step(0.05);
    let mut state = UiState::default();
    state.tab = "graph";
    state.collapsed = true;
    state.flyout = true;
    for _ in 0..4 {
        ribbon_frame(&mut h, &app, &mut state, vec![]);
    }
    for label in ["更多功能", "导出", commands::find("graph.export").unwrap().label] {
        let out = ribbon_frame(&mut h, &app, &mut state, vec![]).0;
        let at =
            crate::test_ui::find_text(&out, label).unwrap_or_else(|| panic!("missing {label}")) + egui::vec2(5., 5.);
        let mut command = None;
        for pressed in [true, false] {
            let (_, clicked) = ribbon_frame(
                &mut h,
                &app,
                &mut state,
                vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, pressed)],
            );
            command = command.or(clicked);
        }
        if label == commands::find("graph.export").unwrap().label {
            assert_eq!(command, Some("graph.export"));
        } else {
            assert!(state.flyout);
        }
        for _ in 0..3 {
            ribbon_frame(&mut h, &app, &mut state, vec![]);
        }
    }
}
