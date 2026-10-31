use scicanvas_core::{history::History, Project};

#[test]
fn notes_share_project_ids_folders_and_history() {
    let mut p = Project::sample();
    let folder = p.add_folder(None).unwrap();
    let before = p.clone();
    let mut history = History::default();
    history.begin(&p, "结果 Note");
    let id = p.add_note("拟合结果", "斜率\t1.25\n截距\t-0.3\nR²\t0.99".into());
    p.move_project_item(id, Some(folder)).unwrap();
    p.rename_project_item(id, "线性拟合报告").unwrap();
    assert_eq!(p.folder_of(id), Some(folder));
    assert_eq!(p.name_of(id), Some("线性拟合报告"));
    p.validate_folders().unwrap();
    history.changed();
    assert!(history.commit(&p));
    let saved = p.clone();
    p = history.undo(&p).unwrap();
    assert_eq!(p, before);
    p = history.redo(&p).unwrap();
    assert_eq!(p, saved);
    p.remove_folder(folder).unwrap();
    assert_eq!(p.folder_of(id), None);
    p.remove(id);
    assert!(p.note(id).is_none());
}

#[cfg(feature = "serde")]
#[test]
fn v10_notes_roundtrip_and_export_plain_text_inside_scv() {
    use scicanvas_core::project_file;
    use std::io::Read;
    let mut p = Project::default();
    let id = p.add_note("报告/:?", "α\t1.234\nR²\t0.999\n中文与换行\n".into());
    let folder = p.add_folder(None).unwrap();
    p.move_project_item(id, Some(folder)).unwrap();
    let json = project_file::to_json(&p).unwrap();
    assert_eq!(project_file::from_json(&json).unwrap(), p);
    let bytes = project_file::to_scv(&p).unwrap();
    assert_eq!(project_file::from_scv(&bytes).unwrap(), p);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let path = archive
        .file_names()
        .map(|n| n.unwrap().into_owned())
        .find(|n| n.starts_with("notes/"))
        .unwrap()
        .to_owned();
    assert!(path.ends_with(".txt"));
    let mut text = String::new();
    archive.by_name(&path).unwrap().read_to_string(&mut text).unwrap();
    assert_eq!(text, p.note(id).unwrap().text);
    let mut bad: serde_json::Value = serde_json::from_str(&json).unwrap();
    bad["notes"][0]["id"] = bad["folders"][0]["id"].clone();
    assert!(project_file::from_json(&bad.to_string()).is_err());
}

#[cfg(feature = "serde")]
#[test]
fn old_projects_get_empty_notes_and_note_ids_repair_next_id() {
    use scicanvas_core::project_file;
    let mut old = serde_json::to_value(Project::sample()).unwrap();
    for version in 2..=9 {
        old["version"] = version.into();
        let p = project_file::from_json(&old.to_string()).unwrap();
        assert!(p.notes.is_empty());
        assert_eq!(p.version, project_file::CURRENT_VERSION);
    }
    let mut p = Project::default();
    p.add_note("Note", "result".into());
    p.notes[0].id = 123;
    p.next_id = 1;
    let mut loaded = project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap();
    assert!(loaded.add_note("Next", String::new()) > 123);
}
