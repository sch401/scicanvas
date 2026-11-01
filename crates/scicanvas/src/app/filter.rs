use super::*;
use scicanvas_core::{DataKind, filter::FilterRule};

impl SciApp {
    pub(super) fn open_filter_dialog(&mut self, id: u64, key: &str) {
        let Some(book) = self.project.book(id) else { return };
        let Some(column) = book.column(key) else { return };
        let existing = book.filter.as_ref().and_then(|f| f.rules.get(key));
        let editor = match column.data.kind() {
            DataKind::Num => {
                let (min, max) = match existing { Some(FilterRule::Number { min, max }) => (*min, *max), _ => (None, None) };
                dialogs::FilterEditor::Number { min: min.map_or(String::new(), |v| v.to_string()), max: max.map_or(String::new(), |v| v.to_string()) }
            }
            DataKind::Text | DataKind::DateTime => {
                let contains = match existing { Some(FilterRule::Text { contains }) => contains.clone(), _ => String::new() };
                dialogs::FilterEditor::Text { contains }
            }
            DataKind::Category => {
                let mut names = column.data.category_levels().unwrap_or_default();
                names.push(String::new());
                let selected = match existing { Some(FilterRule::Category { values }) => Some(values), _ => None };
                let levels = names.into_iter().map(|name| { let enabled = selected.is_none_or(|values| values.contains(&name)); (name, enabled) }).collect();
                dialogs::FilterEditor::Category { levels }
            }
        };
        let visible_only = self.sheets.get(&id).is_some_and(|state| state.visible_only);
        self.dialog = Some(Dialog::Filter(Box::new(dialogs::FilterDialog {
            book: id, key: key.into(), title: format!("{}（{}）", key, column.display_name()), editor, visible_only, error: None,
        })));
    }

    pub(super) fn apply_filter(&mut self, id: u64, key: String, rule: Option<FilterRule>, visible_only: bool) {
        let result = self.edit_with("筛选工作表", |project| {
            let book = project.book_mut(id).ok_or("找不到工作簿")?;
            let mut filter = book.filter.clone().unwrap_or_default();
            if let Some(rule) = rule { filter.rules.insert(key.clone(), rule); } else { filter.rules.remove(&key); }
            filter.validate(book)?;
            book.filter = if filter.rules.is_empty() { None } else { Some(filter) };
            Ok(Some(()))
        }, |app, ()| {
            app.sheets.entry(id).or_default().visible_only = visible_only;
            app.status = format!("已更新 {key} 列筛选");
            app.dialog = None;
        });
        if result.is_none() {
            if let Some(Dialog::Filter(dialog)) = &mut self.dialog { dialog.error = Some(self.status.clone()); }
        }
    }
}
