//! Note 文档与计算报告的保存和编辑。
use super::*;

impl SciApp {
    pub(crate) fn new_note(&mut self) {
        self.save_note("Note", String::new());
    }

    pub(super) fn save_note(&mut self, title: &str, text: String) {
        let parent = self.current_folder();
        let id = self.project.add_note(title, text);
        self.project.move_project_item(id, parent).unwrap();
        self.show_item(id);
        self.status = "Note 已添加到项目，保存项目即可保留内容".into();
    }

    pub(crate) fn export_note(&mut self) {
        let Some(note) = self.active.and_then(|id| self.project.note(id)) else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.txt", note.name))
            .add_filter("UTF-8 文本", &["txt"])
            .save_file()
        {
            self.status = match std::fs::write(path, &note.text) {
                Ok(()) => "已导出 Note 文本".into(),
                Err(error) => format!("导出失败：{error}"),
            };
        }
    }

    pub(super) fn note_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        if ui.button("导出文本…").clicked() {
            self.active = Some(id);
            self.run_command("note.export");
        }
        let Some(note) = self.project.note(id) else { return };
        let mut text = note.text.clone();
        let response = egui::ScrollArea::vertical()
            .id_salt(("note-scroll", id))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .id_salt(("note-text", id))
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(20),
                )
            })
            .inner;
        if response.changed() {
            self.begin_edit("编辑 Note");
            self.project.notes.iter_mut().find(|n| n.id == id).unwrap().text = text;
            self.history.changed();
        }
        edit_group(&response, &mut self.history_group);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_editor_changes_text_and_roundtrips_history_and_project() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.run_command("note.new");
        let id = app.active.unwrap();
        app.project.notes[0].text = "原始结果".into();
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.02;
            ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    app.history_group = None;
                    egui::CentralPanel::default().show(ctx, |ui| app.note_ui(ui, id));
                    app.finish_frame_edit(app.history_group);
                },
            )
        };
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "原始结果" => Some(t.pos + egui::vec2(4.0, 4.0)),
                _ => None,
            })
            .unwrap();
        for pressed in [true, false] {
            frame(
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
        frame(
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            }],
        );
        frame(&mut app, vec![egui::Event::Text("斜率 = 2\nR² = 0.99".into())]);
        assert_eq!(app.project.note(id).unwrap().text, "斜率 = 2\nR² = 0.99");
        let after = app.project.clone();
        app.run_command("edit.undo");
        assert_eq!(app.project, before);
        app.run_command("edit.redo");
        assert_eq!(app.project, after);
        let bytes = scicanvas_core::project_file::to_scv(&app.project).unwrap();
        app.load_project(scicanvas_core::project_file::from_scv(&bytes).unwrap(), None);
        assert!(app.open[&id]);
        assert_eq!(app.project.note(id).unwrap().text, "斜率 = 2\nR² = 0.99");
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
    }
}
