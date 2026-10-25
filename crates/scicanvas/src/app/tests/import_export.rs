use super::*;

/// 百万行十列的首帧测量（不进 CI）：
/// `cargo test --release -p scicanvas --bin scicanvas million_row_first_frame -- --ignored --nocapture`
/// 计时包括导入、建图和完整界面首帧（布局与三角化，不含 GPU 提交）；进程峰值内存由外部采样。
#[test]
#[ignore]
fn million_row_first_frame() {
    use std::fmt::Write;
    use std::time::Instant;
    let rows = 1_000_000;
    let mut input = String::from("X,Y1,Y2,Y3,Y4,Y5,Y6,Y7,Y8,Y9\n");
    for i in (0..rows).rev() {
        writeln!(
            input,
            "{i},{},{},{},{},{},{},{},{},{}",
            i % 97,
            i % 89,
            i % 83,
            i % 79,
            i % 73,
            i % 71,
            i % 67,
            i % 61,
            i % 59
        )
        .unwrap();
    }
    let mut app = app();
    let t = Instant::now();
    let id = app.project.alloc_id();
    let book = scicanvas_core::csv::book_from_text(id, "Million", &input);
    drop(input);
    app.project.books.push(book);
    let import = t.elapsed();
    let t = Instant::now();
    let (gid, _) = app
        .project
        .plot_from_selection(id, &["B".into()], PlotType::Scatter)
        .unwrap();
    app.show_item(id);
    app.show_item(gid);
    let plot = t.elapsed();
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let mut harness = crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(1600.0, 900.0)), SciApp::ui_frame);
    let t = Instant::now();
    let out = harness.frame(&mut app, vec![]);
    let meshes = ctx.tessellate(out.shapes, out.pixels_per_point).len();
    let first = t.elapsed();
    let t = Instant::now();
    let out = harness.frame(&mut app, vec![]);
    let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
    let second = t.elapsed();
    println!(
        "PERF import_ms={:.1} plot_ms={:.1} first_frame_ms={:.1} second_frame_ms={:.1} meshes={meshes}",
        import.as_secs_f64() * 1000.0,
        plot.as_secs_f64() * 1000.0,
        first.as_secs_f64() * 1000.0,
        second.as_secs_f64() * 1000.0
    );
}

#[test]
fn excel_workbooks_import_each_non_empty_sheet() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scicanvas-core/tests/fixtures/sample.xlsx");
    let mut app = app();
    let before = app.project.clone();
    app.open_dropped(vec![path]);
    let names: Vec<_> = app.project.books.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(
        names,
        ["Book1", "sample-测量", "sample-对照"],
        "the empty sheet is skipped"
    );
    let book = &app.project.books[1];
    let head: Vec<_> = book
        .columns
        .iter()
        .map(|c| (c.long_name.as_str(), c.units.as_str()))
        .collect();
    assert_eq!(head, [("时间", "s"), ("电压", "mV"), ("备注", "")]);
    assert_eq!(book.columns[1].data.text_at(5), "0.3");
    assert!(app.status.contains("sample-测量（6 行 × 3 列）"));
    roundtrip(&mut app, &before);
}

#[test]
fn dropped_data_files_import_as_one_step_and_project_files_open() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/file-tests")
        .join(format!("drop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("温度.csv");
    let b = dir.join("压力.txt");
    std::fs::write(&a, "时间 (s),温度 (°C)\n0,20\n1,21.5\n").unwrap();
    std::fs::write(&b, "1\t2\n3\t4\n5\t6\n").unwrap();

    let mut app = app();
    let before = app.project.clone();
    let ctx = egui::Context::default();
    let drop = |app: &mut SciApp, paths: &[&std::path::Path]| {
        let input = egui::RawInput {
            dropped_files: paths
                .iter()
                .map(|p| egui::DroppedFile {
                    path: Some(p.to_path_buf()),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| app.dropped_files(ctx));
    };
    drop(&mut app, &[&a, &b]);
    let names: Vec<_> = app.project.books.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Book1", "温度", "压力"]);
    let t = app.project.books[1].clone();
    assert_eq!(
        (t.columns[1].long_name.as_str(), t.columns[1].units.as_str()),
        ("温度", "°C")
    );
    assert_eq!(app.project.books[2].nrows(), 3);
    assert_eq!(app.history.undo_label(), Some("导入数据"));
    roundtrip(&mut app, &before);

    let scv = dir.join("项目.scv");
    let mut other = Project::sample();
    other.books[0].name = "拖入的项目".into();
    scicanvas_core::project_file::save(&scv, &other).unwrap();
    drop(&mut app, &[&a, &scv]);
    assert_eq!(app.project, other, "a project file wins over data files");
    assert_eq!(app.project_path.as_deref(), Some(scv.as_path()));
    assert!(app.history.undo_label().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mixed_imports_keep_successful_files_folder_windows_and_one_undo_step() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/file-tests")
        .join(format!("mixed-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("data.csv");
    let bad = dir.join("bad.xlsx");
    let empty = dir.join("empty.csv");
    let absent = dir.join("absent.csv");
    std::fs::write(&csv, "X,Y\n0,1\n1,2\n").unwrap();
    std::fs::write(&bad, "broken Excel").unwrap();
    std::fs::write(&empty, "").unwrap();
    let mut app = app();
    let parent = app.project.add_folder(None).unwrap();
    app.explorer_target = Some(parent);
    let before = app.project.clone();
    app.open_dropped(vec![csv.clone(), bad.clone(), absent, csv.clone(), empty.clone()]);
    assert_eq!(app.project.books.len(), before.books.len() + 2);
    let ids: Vec<_> = app
        .project
        .books
        .iter()
        .skip(before.books.len())
        .map(|b| b.id)
        .collect();
    assert_eq!(app.project.book(ids[0]).unwrap().name, "data");
    assert_eq!(app.project.book(ids[1]).unwrap().name, "data_1");
    for id in &ids {
        assert_eq!(app.project.folder_of(*id), Some(parent));
        assert_eq!(app.open.get(id), Some(&true));
    }
    assert_eq!(app.active, ids.last().copied());
    assert_eq!(app.current_folder(), Some(parent));
    assert!(
        app.status.contains("无法导入") && app.status.contains("无法读取") && app.status.ends_with("文件里没有数据")
    );
    assert_eq!(app.history.undo_label(), Some("导入数据"));
    let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
    assert_eq!(scicanvas_core::project_file::from_scv(&saved).unwrap(), app.project);
    roundtrip(&mut app, &before);
    app.undo();
    assert!(app.history.undo_label().is_none());
    for path in [csv, bad, empty] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn failed_imports_preserve_redo_selection_and_existing_project() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/file-tests")
        .join(format!("failed-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let empty = dir.join("empty.csv");
    std::fs::write(&empty, "").unwrap();
    let mut app = app();
    app.run_command("book.new");
    app.undo();
    let before = app.project.clone();
    let active = app.active;
    let selection = app.explorer_selection.clone();
    let redo = app.history.redo_label().unwrap().to_string();
    app.open_dropped(vec![dir.join("absent.csv"), empty.clone()]);
    assert_eq!(app.project, before);
    assert_eq!(app.active, active);
    assert_eq!(app.explorer_selection, selection);
    assert!(!app.history.is_pending());
    assert!(app.history.undo_label().is_none());
    assert_eq!(app.history.redo_label(), Some(redo.as_str()));
    std::fs::remove_file(empty).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
