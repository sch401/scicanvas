//! Note 文档与计算报告的保存和编辑。
use super::*;

impl SciApp {
    pub(crate) fn new_note(&mut self) {
        self.save_note("Note", String::new());
        // 空白 Note 直接进入编辑
        if let Some(id) = self.active {
            self.note_editing.insert(id);
        }
    }

    pub(super) fn save_note(&mut self, title: &str, text: String) {
        let parent = self.current_folder();
        let id = self.project.add_note(title, text);
        self.project.move_project_item(id, parent).unwrap();
        self.show_item(id);
        self.status_ok("Note 已添加到项目，保存项目即可保留内容".into());
    }

    pub(crate) fn export_note(&mut self) {
        let Some(note) = self.active.and_then(|id| self.project.note(id)) else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.md", note.name))
            .add_filter("Markdown", &["md"])
            .add_filter("UTF-8 文本", &["txt"])
            .save_file()
        {
            {
                let mut status_kind = StatusKind::Success;
                let text: String = match std::fs::write(path, &note.text) {
                    Ok(()) => "已导出 Note".into(),
                    Err(error) => {
                        status_kind = StatusKind::Error;
                        format!("导出失败：{error}")
                    }
                };
                self.set_status(status_kind, text);
            };
        }
    }

    pub(super) fn note_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        let mut editing = self.note_editing.contains(&id);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut editing, false, "预览");
            ui.selectable_value(&mut editing, true, "编辑");
            ui.separator();
            if ui.button("导出…").clicked() {
                self.active = Some(id);
                self.run_command("note.export");
            }
        });
        if editing {
            self.note_editing.insert(id);
        } else {
            self.note_editing.remove(&id);
        }
        let Some(note) = self.project.note(id) else { return };
        if !editing {
            let pal = self.pal;
            egui::ScrollArea::vertical()
                .id_salt(("note-view", id))
                .auto_shrink(false)
                .show(ui, |ui| {
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                        .show(ui, |ui| crate::markdown::show(ui, &note.text, &pal));
                });
            return;
        }
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
        edit_group(&response, &mut self.history_group);
        if response.changed() {
            self.edit_grouped("编辑 Note", self.history_group, |project| {
                project.notes.iter_mut().find(|n| n.id == id).unwrap().text = text;
            });
        }
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
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(800.0, 600.0)),
            |app: &mut SciApp, ctx: &egui::Context| {
                app.history_group = None;
                egui::CentralPanel::default().show(ctx, |ui| app.note_ui(ui, id));
                app.finish_frame_edit(app.history_group);
            },
        )
        .with_step(0.02);
        harness.frame(&mut app, vec![]);
        let output = harness.frame(&mut app, vec![]);
        let pos = crate::test_ui::find_text(&output, "原始结果")
            .map(|pos| pos + egui::vec2(4.0, 4.0))
            .unwrap();
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
        harness.frame(
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
        harness.frame(&mut app, vec![egui::Event::Text("斜率 = 2\nR² = 0.99".into())]);
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

    #[test]
    fn saved_markdown_notes_render_as_tables_and_switch_to_editing() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.save_note(
            "拟合结果",
            "## 电压 A\n\n| 参数 | 值 |\n| --- | --- |\n| B0 | 1.5 |".into(),
        );
        let id = app.active.unwrap();
        assert!(!app.note_editing.contains(&id));
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(800.0, 600.0)),
            |app: &mut SciApp, ctx: &egui::Context| {
                egui::CentralPanel::default().show(ctx, |ui| app.note_ui(ui, id));
            },
        )
        .with_step(0.02);
        let texts = |output: &egui::FullOutput| -> Vec<(String, egui::Pos2)> {
            output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some((t.galley.job.text.clone(), t.pos + egui::vec2(4.0, 4.0))),
                    _ => None,
                })
                .collect()
        };
        harness.frame(&mut app, vec![]);
        let shown = texts(&harness.frame(&mut app, vec![]));
        let has = |t: &str| shown.iter().any(|(s, _)| s == t);
        // 预览：标题和单元格分别显示，不出现 Markdown 原文
        assert!(has("电压 A") && has("参数") && has("B0") && has("1.5"), "{shown:?}");
        assert!(
            !shown.iter().any(|(s, _)| s.contains('|') || s.contains("##")),
            "{shown:?}"
        );
        let pos = shown.iter().find(|(s, _)| s == "编辑").unwrap().1;
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
        assert!(app.note_editing.contains(&id));
        let shown = texts(&harness.frame(&mut app, vec![]));
        assert!(shown.iter().any(|(s, _)| s.contains("| 参数 | 值 |")), "{shown:?}");
    }

    #[test]
    fn note_preview_text_can_be_selected_and_copied_but_not_edited() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        let text = "## 电压 A\n\ny = 1 + 2·x\n\n| 参数 | 值 |\n| --- | --- |\n| B0 | 1.5 |";
        app.save_note("拟合结果", text.into());
        let id = app.active.unwrap();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(800.0, 600.0)),
            |app: &mut SciApp, ctx: &egui::Context| {
                egui::CentralPanel::default().show(ctx, |ui| app.note_ui(ui, id));
            },
        )
        .with_step(0.02);
        harness.frame(&mut app, vec![]);
        let output = harness.frame(&mut app, vec![]);
        let at = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + t.galley.rect.size()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing {label}"))
        };
        // 从标题左上角拖到表格最后一个单元格右下角
        let start = at("电压 A") - egui::vec2(60.0, 20.0);
        let end = at("1.5");
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        harness.frame(&mut app, vec![egui::Event::PointerMoved(start), press(start, true)]);
        for k in 1..=10 {
            let p = start + (end - start) * (k as f32 / 10.0);
            harness.frame(&mut app, vec![egui::Event::PointerMoved(p)]);
        }
        harness.frame(&mut app, vec![press(end, false)]);
        let copied = harness
            .frame(&mut app, vec![egui::Event::Copy])
            .platform_output
            .copied_text;
        for part in ["电压 A", "y = 1 + 2·x", "参数", "B0", "1.5"] {
            assert!(copied.contains(part), "{copied:?}");
        }
        // 预览只读：键入文字不改内容
        harness.frame(&mut app, vec![egui::Event::Text("改".into())]);
        assert_eq!(app.project.note(id).unwrap().text, text);
    }

    #[test]
    fn double_clicking_a_table_cell_selects_the_whole_number() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        let text = "| 参数 | 值 |\n| --- | --- |\n| B1 | -0.061816028 |";
        app.save_note("拟合结果", text.into());
        let id = app.active.unwrap();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(800.0, 600.0)),
            |app: &mut SciApp, ctx: &egui::Context| {
                egui::CentralPanel::default().show(ctx, |ui| app.note_ui(ui, id));
            },
        )
        .with_step(0.02);
        harness.frame(&mut app, vec![]);
        let output = harness.frame(&mut app, vec![]);
        // 点在“0618”中间：egui 自带的双击选词只会选中小数点后的一段
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "-0.061816028" => {
                    Some(t.pos + egui::vec2(t.galley.rect.width() * 0.6, t.galley.rect.height() / 2.0))
                }
                _ => None,
            })
            .unwrap();
        let press = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        harness.frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
        for _ in 0..2 {
            harness.frame(&mut app, vec![press(true)]);
            harness.frame(&mut app, vec![press(false)]);
        }
        for _ in 0..3 {
            harness.frame(&mut app, vec![]);
        }
        let copied = harness
            .frame(&mut app, vec![egui::Event::Copy])
            .platform_output
            .copied_text;
        assert_eq!(copied, "-0.061816028");
        // 只读：键入文字不改内容
        harness.frame(&mut app, vec![egui::Event::Text("9".into())]);
        assert_eq!(app.project.note(id).unwrap().text, text);
    }
}
