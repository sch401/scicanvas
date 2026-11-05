use super::*;
use scicanvas_core::{filter::FilterRule, DataKind};

impl SciApp {
    pub(super) fn open_filter_dialog(&mut self, id: u64, key: &str) {
        let Some(book) = self.project.book(id) else { return };
        let Some(column) = book.column(key) else { return };
        let existing = book.filter.as_ref().and_then(|f| f.rules.get(key));
        let editor = match column.data.kind() {
            DataKind::Num => {
                let (min, max) = match existing {
                    Some(FilterRule::Number { min, max }) => (*min, *max),
                    _ => (None, None),
                };
                dialogs::FilterEditor::Number {
                    min: min.map_or(String::new(), |v| v.to_string()),
                    max: max.map_or(String::new(), |v| v.to_string()),
                }
            }
            DataKind::Text | DataKind::DateTime => {
                let contains = match existing {
                    Some(FilterRule::Text { contains }) => contains.clone(),
                    _ => String::new(),
                };
                dialogs::FilterEditor::Text { contains }
            }
            DataKind::Category => {
                let mut names = column.data.category_levels().unwrap_or_default();
                names.push(String::new());
                let selected = match existing {
                    Some(FilterRule::Category { values }) => Some(values),
                    _ => None,
                };
                let levels = names
                    .into_iter()
                    .map(|name| {
                        let enabled = selected.is_none_or(|values| values.contains(&name));
                        (name, enabled)
                    })
                    .collect();
                dialogs::FilterEditor::Category { levels }
            }
        };
        let visible_only = book.uses_visible_rows();
        self.dialog = Some(Dialog::Filter(Box::new(dialogs::FilterDialog {
            book: id,
            key: key.into(),
            title: format!("{}（{}）", key, column.display_name()),
            editor,
            visible_only,
            error: None,
        })));
    }

    pub(super) fn apply_filter(&mut self, id: u64, key: String, rule: Option<FilterRule>, visible_only: bool) {
        let result = self.edit_with(
            "筛选工作表",
            |project| {
                let book = project.book_mut(id).ok_or("找不到工作簿")?;
                let mut filter = book.filter.clone().unwrap_or_default();
                if let Some(rule) = rule {
                    filter.rules.insert(key.clone(), rule);
                } else {
                    filter.rules.remove(&key);
                }
                filter.visible_only = visible_only;
                filter.validate(book)?;
                book.filter = if filter.rules.is_empty() { None } else { Some(filter) };
                Ok(Some(()))
            },
            |app, ()| {
                app.status_ok(format!("已更新 {key} 列筛选"));
                app.dialog = None;
            },
        );
        if result.is_none() {
            if let Some(Dialog::Filter(dialog)) = &mut self.dialog {
                dialog.error = Some(self.status.text.clone());
            }
        }
    }

    /// 只切换“绘图和分析只使用可见行”，不改筛选规则。
    pub(super) fn set_filter_visible_only(&mut self, id: u64, visible_only: bool) {
        self.edit_with(
            "切换只用可见行",
            |project| {
                let filter = project
                    .book_mut(id)
                    .and_then(|book| book.filter.as_mut())
                    .ok_or("工作表没有筛选")?;
                if filter.visible_only == visible_only {
                    return Ok(None);
                }
                filter.visible_only = visible_only;
                Ok(Some(()))
            },
            |app, ()| {
                {
                    let status_kind = StatusKind::Success;
                    let text: String = if visible_only {
                        "绘图和分析只使用可见行".into()
                    } else {
                        "绘图和分析使用全部行".into()
                    };
                    app.set_status(status_kind, text);
                };
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_and_undoing_a_filter_preserves_original_data() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        let before = app.project.book(1).unwrap().columns.clone();
        app.open_filter_dialog(1, "A");
        assert!(matches!(app.dialog, Some(Dialog::Filter(_))));
        app.apply_filter(
            1,
            "A".into(),
            Some(FilterRule::Number {
                min: Some(2.0),
                max: Some(4.0),
            }),
            true,
        );
        let book = app.project.book(1).unwrap();
        assert_eq!(book.columns, before);
        assert!(!book.visible_rows().is_empty());
        assert!(book.visible_rows().len() < book.nrows());
        assert!(app.project.book(1).unwrap().uses_visible_rows());
        app.run_command("edit.undo");
        assert!(app.project.book(1).unwrap().filter.is_none());
        app.run_command("edit.redo");
        assert!(app.project.book(1).unwrap().uses_visible_rows());
        assert!(app.project.book(1).unwrap().filter.is_some());
        assert_eq!(app.project.book(1).unwrap().columns, before);
    }

    #[test]
    fn filtered_sheet_shows_row_count_and_toggle_is_undoable() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.apply_filter(
            1,
            "A".into(),
            Some(FilterRule::Number {
                min: Some(2.0),
                max: Some(4.0),
            }),
            false,
        );
        let book = app.project.book(1).unwrap();
        assert!(!book.uses_visible_rows());
        let text = format!("已筛选：显示 {} / {} 行", book.visible_rows().len(), book.nrows());
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
            });
        }
        assert!(crate::test_ui::find_text(&output, &text).is_some());
        app.set_filter_visible_only(1, true);
        assert!(app.project.book(1).unwrap().uses_visible_rows());
        app.run_command("edit.undo");
        assert!(!app.project.book(1).unwrap().uses_visible_rows());
        assert!(app.project.book(1).unwrap().filter.is_some());
    }

    #[test]
    fn sheet_without_filter_has_no_filter_bar() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
            });
        }
        let has_filter_bar = output
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.starts_with("已筛选")));
        assert!(!has_filter_bar);
    }
}
