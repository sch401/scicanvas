use super::*;
use scicanvas_core::filter::FilterRule;

pub enum FilterEditor {
    Number { min: String, max: String },
    Text { contains: String },
    Category { levels: Vec<(String, bool)> },
}

pub struct FilterDialog {
    pub(crate) book: u64,
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) editor: FilterEditor,
    pub(crate) visible_only: bool,
    pub(crate) error: Option<String>,
}

impl FilterDialog {
    pub(super) fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "筛选列").open(open).show(ctx, |ui| {
            ui.label(&self.title);
            match &mut self.editor {
                FilterEditor::Number { min, max } => {
                    ui.horizontal(|ui| { ui.label("最小值"); ui.text_edit_singleline(min); });
                    ui.horizontal(|ui| { ui.label("最大值"); ui.text_edit_singleline(max); });
                    ui.label("留空表示无边界，缺失值不匹配。");
                }
                FilterEditor::Text { contains } => { ui.label("包含文字（区分大小写）"); ui.text_edit_singleline(contains); }
                FilterEditor::Category { levels } => {
                    ui.label("选择类别：");
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (name, selected) in levels { ui.checkbox(selected, if name.is_empty() { "（空值）" } else { name.as_str() }); }
                    });
                }
            }
            ui.checkbox(&mut self.visible_only, "绘图和分析只使用可见行");
            if let Some(error) = &self.error { ui.colored_label(ui.visuals().error_fg_color, error); }
            let (ok, cancel) = buttons(ui, "应用");
            if ok {
                let rule = match &self.editor {
                    FilterEditor::Number { min, max } => {
                        let parse = |text: &str| if text.trim().is_empty() { Ok(None) } else { text.trim().parse::<f64>().map(Some).map_err(|_| "请输入有效数值".to_string()) };
                        parse(min).and_then(|min| parse(max).map(|max| FilterRule::Number { min, max }))
                    }
                    FilterEditor::Text { contains } => Ok(FilterRule::Text { contains: contains.clone() }),
                    FilterEditor::Category { levels } => Ok(FilterRule::Category { values: levels.iter().filter(|(_, selected)| *selected).map(|(name, _)| name.clone()).collect() }),
                };
                match rule {
                    Ok(rule) => { self.error = None; action = Action::SetFilter(self.book, self.key.clone(), Some(rule), self.visible_only); }
                    Err(error) => self.error = Some(error),
                }
            } else if cancel { action = Action::Close; }
            if ui.button("清除本列筛选").clicked() { action = Action::SetFilter(self.book, self.key.clone(), None, self.visible_only); }
        });
        action
    }
}
