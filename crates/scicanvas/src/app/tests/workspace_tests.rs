use super::*;

#[test]
fn explorer_slow_second_click_renames_folders_books_and_graphs_with_history() {
    for id in [1, 2, 3] {
        let mut app = app();
        assert_eq!(app.project.add_folder(None).unwrap(), 3);
        let before = app.project.clone();
        let name = app.project.name_of(id).unwrap().to_owned();
        let mut ui = ExplorerUi::new();
        let at = ui.at(&mut app, &name);
        ui.click(&mut app, at);
        ui.harness.time += 0.7;
        ui.click(&mut app, at);
        assert_eq!(app.explorer_rename.map(|r| r.0), Some(id));
        ui.frame(&mut app, vec![]);
        ui.frame(&mut app, vec![egui::Event::Text("重命名 α".into())]);
        ui.key(&mut app, egui::Key::Enter);
        assert_eq!(app.project.name_of(id), Some("重命名 α"));
        assert!(app.explorer_rename.is_none());
        roundtrip(&mut app, &before);
    }
}

#[test]
fn explorer_fast_double_click_escape_and_focus_commit_have_distinct_behavior() {
    let mut app = app();
    let before = app.project.clone();
    let mut ui = ExplorerUi::new();
    let at = ui.at(&mut app, "Book1");
    ui.click(&mut app, at);
    ui.click(&mut app, at);
    assert!(app.explorer_rename.is_none());
    ui.key(&mut app, egui::Key::F2);
    ui.frame(&mut app, vec![egui::Event::Text("取消的名称".into())]);
    ui.key(&mut app, egui::Key::Escape);
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
    ui.key(&mut app, egui::Key::F2);
    ui.key(&mut app, egui::Key::Backspace);
    ui.key(&mut app, egui::Key::Enter);
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
    ui.key(&mut app, egui::Key::F2);
    ui.frame(&mut app, vec![egui::Event::Text("失焦提交".into())]);
    let root = ui.at(&mut app, "项目根目录");
    ui.click(&mut app, root);
    assert_eq!(app.project.name_of(1), Some("失焦提交"));
    assert_eq!(app.explorer_target, None);
    roundtrip(&mut app, &before);
}

#[test]
fn f2_goes_to_the_focused_explorer_or_sheet_but_not_both() {
    let mut app = app();
    app.active = Some(1);
    app.explorer_target = Some(1);
    let mut ui = ExplorerUi::new();
    ui.sheet = Some(SheetState {
        range: Some(Range::single(0, 0)),
        ..Default::default()
    });
    ui.click(&mut app, egui::pos2(300.0, 300.0));
    ui.key(&mut app, egui::Key::F2);
    assert!(app.explorer_rename.is_none());
    assert!(ui.sheet.as_mut().unwrap().edit.take().is_some());

    let at = ui.at(&mut app, "Book1");
    ui.click(&mut app, at);
    ui.key(&mut app, egui::Key::F2);
    assert_eq!(app.explorer_rename.map(|r| r.0), Some(1));
    assert!(ui.sheet.as_ref().unwrap().edit.is_none());
}

#[test]
fn sheet_keys_only_apply_after_clicking_the_sheet_not_the_explorer() {
    let mut app = app();
    app.active = Some(1);
    app.explorer_target = Some(1);
    let before = app.project.clone();
    let mut ui = ExplorerUi::new();
    ui.sheet = Some(SheetState {
        range: Some(Range::single(0, 0)),
        ..Default::default()
    });
    let at = ui.at(&mut app, "Book1");
    ui.click(&mut app, at);
    ui.key(&mut app, egui::Key::Delete);
    ui.key(&mut app, egui::Key::ArrowDown);
    ui.frame(&mut app, vec![egui::Event::Text("9".into())]);
    assert_eq!(app.project, before);
    let sheet = ui.sheet.as_ref().unwrap();
    assert_eq!(sheet.range, Some(Range::single(0, 0)));
    assert!(sheet.edit.is_none());

    ui.click(&mut app, egui::pos2(300.0, 300.0));
    ui.key(&mut app, egui::Key::Delete);
    assert_ne!(app.project, before);
    app.run_command("edit.undo");
    assert_eq!(app.project, before);
}

#[test]
fn explorer_new_table_and_graph_buttons_use_folder_and_history() {
    for (label, command) in [("工作簿", "book.new"), ("图形", "graph.new")] {
        let mut app = app();
        let folder = app.project.add_folder(None).unwrap();
        app.explorer_target = Some(folder);
        let before = app.project.clone();
        let mut ui = ExplorerUi::new();
        // 新建命令合并在“＋ 新建”菜单里：先打开菜单再点菜单项
        let pos = ui.at(&mut app, "＋ 新建");
        ui.click(&mut app, pos);
        let pos = ui.at(&mut app, label);
        ui.click(&mut app, pos);
        let id = app.active.unwrap();
        assert_eq!(app.project.folder_of(id), Some(folder));
        assert!(app.open[&id]);
        if command == "graph.new" {
            let graph = app.project.graph(id).unwrap();
            assert_eq!(graph.first_layer().book_id, None);
            assert_eq!(app.project.books, before.books);
        } else {
            assert!(app.project.book(id).is_some());
        }
        roundtrip(&mut app, &before);
    }
    let mut app = app();
    app.load_project(Project::default(), None);
    app.run_command("graph.new");
    assert_eq!(app.project.graphs.len(), 1);
    assert!(app.project.books.is_empty());
}

#[test]
fn explorer_shift_ranges_follow_sort_order_and_skip_collapsed_contents() {
    let mut app = app();
    let note = app.project.add_note("M-Note", "结果".into());
    app.project.rename_project_item(1, "Z-Table").unwrap();
    app.project.rename_project_item(2, "A-Graph").unwrap();
    let mut ui = ExplorerUi::new();
    let table_pos = ui.at(&mut app, "Z-Table");
    ui.click(&mut app, table_pos);
    assert_eq!(app.explorer_visible, vec![1, 2, note]);
    ui.harness.modifiers.shift = true;
    let note_pos = ui.at(&mut app, "M-Note1");
    ui.click(&mut app, note_pos);
    assert_eq!(app.explorer_selection, vec![1, 2, note]);
    let graph_pos = ui.at(&mut app, "A-Graph");
    ui.click(&mut app, graph_pos);
    assert_eq!(app.explorer_selection, vec![1, 2]);
    ui.harness.modifiers.ctrl = true;
    ui.click(&mut app, note_pos);
    assert_eq!(app.explorer_selection, vec![1, 2, note]);
    assert!(app.explorer_rename.is_none());
    ui.harness.modifiers = egui::Modifiers::NONE;
    app.explorer_sort = workspace::ExplorerSort::Name;
    ui.frame(&mut app, vec![]);
    assert_eq!(app.explorer_visible, vec![2, note, 1]);
    let folder = app.project.add_folder(None).unwrap();
    app.project.move_project_item(1, Some(folder)).unwrap();
    let mut collapsed = egui::collapsing_header::CollapsingState::load_with_default_open(
        &ui.ctx,
        egui::Id::new(("project-folder", folder)),
        false,
    );
    collapsed.set_open(false);
    collapsed.store(&ui.ctx);
    ui.frame(&mut app, vec![]);
    assert_eq!(app.explorer_visible, vec![folder, 2, note]);
    let name = app.project.name_of(folder).unwrap().to_owned();
    let folder_pos = ui.at(&mut app, &name);
    ui.click(&mut app, folder_pos);
    ui.harness.modifiers.shift = true;
    let note_pos = ui.at(&mut app, "M-Note1");
    ui.click(&mut app, note_pos);
    assert_eq!(app.explorer_selection, vec![folder, 2, note]);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn explorer_ctrl_click_toggles_selection_without_rename_or_data_changes() {
    let mut app = app();
    let folder = app.project.add_folder(None).unwrap();
    let before = app.project.clone();
    let mut ui = ExplorerUi::new();
    let book = ui.at(&mut app, "Book1");
    ui.click(&mut app, book);
    assert_eq!(app.explorer_selection, vec![1]);
    let graph = ui.at(&mut app, "Graph1");
    ui.harness.modifiers.ctrl = true;
    ui.click(&mut app, graph);
    assert_eq!(app.explorer_selection, vec![1, 2]);
    let name = app.project.name_of(folder).unwrap().to_owned();
    let folder_pos = ui.at(&mut app, &name);
    ui.click(&mut app, folder_pos);
    assert_eq!(app.explorer_selection, vec![1, 2, folder]);
    ui.harness.time += 0.7;
    ui.click(&mut app, folder_pos);
    assert_eq!(app.explorer_selection, vec![1, 2]);
    assert!(app.explorer_rename.is_none());
    ui.click(&mut app, graph);
    assert_eq!(app.explorer_selection, vec![1]);
    ui.click(&mut app, book);
    assert!(app.explorer_selection.is_empty());
    assert_eq!(app.explorer_target, None);
    ui.harness.modifiers = egui::Modifiers::NONE;
    ui.click(&mut app, graph);
    assert_eq!(app.explorer_selection, vec![2]);
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn explorer_multi_drag_keeps_folder_contents_and_undoes_in_one_step() {
    let mut app = app();
    let parent = app.project.add_folder(None).unwrap();
    let destination = app.project.add_folder(None).unwrap();
    app.project.rename_project_item(parent, "来源").unwrap();
    app.project.rename_project_item(destination, "目标").unwrap();
    app.project.move_project_item(1, Some(parent)).unwrap();
    let before = app.project.clone();
    let mut ui = ExplorerUi::new();
    let source = ui.at(&mut app, "来源");
    ui.click(&mut app, source);
    ui.harness.modifiers.ctrl = true;
    let book = ui.at(&mut app, "Book1");
    ui.click(&mut app, book);
    let graph = ui.at(&mut app, "Graph1");
    ui.click(&mut app, graph);
    ui.harness.modifiers = egui::Modifiers::NONE;
    ui.drag(&mut app, "Graph1", "目标");
    assert_eq!(app.project.folder(parent).unwrap().parent, Some(destination));
    assert_eq!(app.project.folder_of(1), Some(parent));
    assert_eq!(app.project.folder_of(2), Some(destination));
    roundtrip(&mut app, &before);
    // 有一个目标会形成目录循环时，整批移动保持原样。
    app.explorer_selection = vec![2, destination];
    app.explorer_target = Some(2);
    app.folder_destination = Some(parent);
    let before = app.project.clone();
    let label = app.history.undo_label().map(str::to_owned);
    app.run_command("project.move");
    assert_eq!(app.project, before);
    assert_eq!(app.history.undo_label().map(str::to_owned), label);
}

#[test]
fn explorer_drag_moves_all_object_types_to_folders_and_root_with_history() {
    for id in [1, 2, 3] {
        let mut app = app();
        assert_eq!(app.project.add_folder(None).unwrap(), 3);
        let destination = app.project.add_folder(None).unwrap();
        app.project.rename_project_item(destination, "目标").unwrap();
        let source = app.project.name_of(id).unwrap().to_owned();
        let before = app.project.clone();
        let mut ui = ExplorerUi::new();
        ui.drag(&mut app, &source, "目标");
        let parent = if id == 3 {
            app.project.folder(id).unwrap().parent
        } else {
            app.project.folder_of(id)
        };
        assert_eq!(parent, Some(destination));
        roundtrip(&mut app, &before);
        ui.drag(&mut app, &source, "项目根目录");
        assert_eq!(app.project, before);
        app.run_command("edit.undo");
        let parent = if id == 3 {
            app.project.folder(id).unwrap().parent
        } else {
            app.project.folder_of(id)
        };
        assert_eq!(parent, Some(destination));
    }
}

#[test]
fn explorer_invalid_folder_drop_and_same_parent_drop_leave_history_unchanged() {
    let mut app = app();
    let parent = app.project.add_folder(None).unwrap();
    let child = app.project.add_folder(Some(parent)).unwrap();
    app.project.rename_project_item(parent, "上级").unwrap();
    app.project.rename_project_item(child, "下级").unwrap();
    let before = app.project.clone();
    let mut ui = ExplorerUi::new();
    ui.drag(&mut app, "上级", "下级");
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
    ui.drag(&mut app, "上级", "上级");
    ui.drag(&mut app, "Book1", "项目根目录");
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn workspace_folder_commands_share_history_and_keep_window_actions_in_session() {
    let mut app = app();
    app.explorer_target = None;
    let before = app.project.clone();
    app.run_command("project.folder.new");
    let folder = app.explorer_target.unwrap();
    roundtrip(&mut app, &before);
    app.explorer_target = Some(folder);
    app.project_name = "实验一".into();
    let before = app.project.clone();
    app.run_command("project.rename");
    roundtrip(&mut app, &before);
    app.explorer_target = Some(folder);
    app.run_command("book.new");
    let book = app.active.unwrap();
    assert_eq!(app.project.folder_of(book), Some(folder));
    app.explorer_target = Some(2);
    app.folder_destination = Some(folder);
    let before = app.project.clone();
    app.run_command("project.move");
    roundtrip(&mut app, &before);
    app.explorer_target = Some(folder);
    let before = app.project.clone();
    app.run_command("project.folder.remove");
    assert!(app.project.book(book).is_some());
    roundtrip(&mut app, &before);
    app.history = History::default();
    app.explorer_target = Some(book);
    let before = app.project.clone();
    for (cmd, state) in [
        ("window.minimize", WinState::Min),
        ("window.maximize", WinState::Max),
        ("window.restore", WinState::Normal),
    ] {
        app.run_command(cmd);
        assert_eq!(app.geoms[&book].state, state);
    }
    app.run_command("project.hide");
    assert!(!app.open[&book]);
    app.run_command("project.show");
    assert!(app.open[&book]);
    assert_eq!(app.active, Some(book));
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn workspace_command_ids_and_theme_actions_are_consistent() {
    let mut ids = std::collections::BTreeSet::new();
    for c in commands::commands() {
        assert!(ids.insert(c.id), "重复命令 {}", c.id);
        assert!(!c.label.is_empty());
    }
    let mut app = app();
    let before = app.project.clone();
    for (cmd, pref) in [
        ("theme.dark", egui::ThemePreference::Dark),
        ("theme.light", egui::ThemePreference::Light),
        ("theme.system", egui::ThemePreference::System),
    ] {
        app.run_command(cmd);
        assert_eq!(app.theme_pref, pref);
    }
    app.explorer_target = None;
    app.project_name = "根目录选择不应重命名活动窗口".into();
    app.run_command("project.rename");
    assert_eq!(app.project, before);
    app.active = None;
    app.run_command("window.minimize");
    assert_eq!(app.project, before);
    assert!(app.history.undo_label().is_none());
}

#[test]
fn explorer_new_folder_button_and_tree_render_real_project_data() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = app();
    app.explorer_target = None;
    let mut harness = crate::test_ui::Harness::new(ctx.clone(), None, |app: &mut SciApp, ctx: &egui::Context| {
        egui::CentralPanel::default().show(ctx, |ui| app.explorer(ui));
    });
    let before = app.project.clone();
    // “＋ 新建”菜单 → “文件夹”
    for label in ["＋ 新建", "文件夹"] {
        harness.frame(&mut app, vec![]);
        let output = harness.frame(&mut app, vec![]);
        let at = crate::test_ui::text_rect(&output, label)
            .map(|rect| rect.center())
            .unwrap_or_else(|| panic!("missing {label}"));
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
    }
    assert_eq!(app.project.folders.len(), 1);
    roundtrip(&mut app, &before);
    let output = harness.frame(&mut app, vec![]);
    assert!(output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text=="文件夹1")));
}

#[test]
fn deleted_workbook_returns_and_new_windows_are_cleaned_up() {
    let mut app = app();
    let before = app.project.clone();
    app.active = Some(1);
    app.sheets.insert(1, SheetState::default());
    app.delete_item(1);
    app.finish_edit();
    app.run_command("edit.undo");
    assert_eq!(app.project, before);
    assert_eq!(app.open.get(&1), Some(&true));
    app.run_command("book.new");
    let id = app.active.unwrap();
    app.geoms
        .insert(id, Geom::new(egui::Pos2::ZERO, egui::vec2(300.0, 200.0)));
    app.sheets.insert(id, SheetState::default());
    app.run_command("edit.undo");
    assert!(!app.open.contains_key(&id));
    assert!(!app.geoms.contains_key(&id));
    assert!(!app.sheets.contains_key(&id));
    assert_ne!(app.active, Some(id));
    app.run_command("edit.redo");
    assert_eq!(app.open.get(&id), Some(&true));
}

#[test]
fn u6_project_destination_submenu_moves_without_losing_target() {
    let mut app = app();
    let folder = app.project.add_folder(None).unwrap();
    app.project.rename_project_item(folder, "目标目录").unwrap();
    let before = app.project.clone();
    let mut ui = ExplorerUi::new();
    let at = ui.at(&mut app, "Book1");
    ui.frame(&mut app, vec![egui::Event::PointerMoved(at)]);
    for pressed in [true, false] {
        ui.frame(
            &mut app,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    let out = ui.frame(&mut app, vec![]);
    let at = crate::test_ui::text_rect(&out, "移动到").unwrap().center();
    ui.click(&mut app, at);
    let out = ui.frame(&mut app, vec![]);
    let at = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "目标目录" => Some(t.pos + egui::vec2(4., 4.)),
            _ => None,
        })
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    ui.click(&mut app, at);
    assert_eq!(app.project.folder_of(1), Some(folder));
    roundtrip(&mut app, &before);
}
