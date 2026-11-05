use super::*;
use scicanvas_core::filter::FilterRule;

pub enum FilterEditor {
    Number { min: String, max: String },
    Text { contains: String },
    Category { levels: Vec<(String, bool)> },
}

impl FilterEditor {
    /// 根据编辑器内容得到筛选规则；Ok(None) 表示清除本列筛选。
    fn rule(&self) -> Result<Option<FilterRule>, String> {
        match self {
            FilterEditor::Number { min, max } => {
                let parse = |text: &str| {
                    if text.trim().is_empty() {
                        Ok(None)
                    } else {
                        text.trim()
                            .parse::<f64>()
                            .map(Some)
                            .map_err(|_| "请输入有效数值".to_string())
                    }
                };
                parse(min).and_then(|min| parse(max).map(|max| Some(FilterRule::Number { min, max })))
            }
            // 空文本匹配所有行，等于不筛选
            FilterEditor::Text { contains } if contains.is_empty() => Ok(None),
            FilterEditor::Text { contains } => Ok(Some(FilterRule::Text {
                contains: contains.clone(),
            })),
            // 全部勾选等于不筛选
            FilterEditor::Category { levels } if levels.iter().all(|(_, selected)| *selected) => Ok(None),
            FilterEditor::Category { levels } if levels.iter().all(|(_, selected)| !*selected) => {
                Err("请至少选择一个类别".to_string())
            }
            FilterEditor::Category { levels } => Ok(Some(FilterRule::Category {
                values: levels
                    .iter()
                    .filter(|(_, selected)| *selected)
                    .map(|(name, _)| name.clone())
                    .collect(),
            })),
        }
    }
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
                    ui.horizontal(|ui| {
                        ui.label("最小值");
                        ui.text_edit_singleline(min);
                    });
                    ui.horizontal(|ui| {
                        ui.label("最大值");
                        ui.text_edit_singleline(max);
                    });
                    ui.label("留空表示无边界，缺失值不匹配。");
                }
                FilterEditor::Text { contains } => {
                    ui.label("包含文字（区分大小写）");
                    ui.text_edit_singleline(contains);
                }
                FilterEditor::Category { levels } => {
                    ui.label("选择类别：");
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (name, selected) in levels {
                            ui.checkbox(selected, if name.is_empty() { "（空值）" } else { name.as_str() });
                        }
                    });
                }
            }
            ui.checkbox(&mut self.visible_only, "绘图和分析只使用可见行");
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            let (ok, cancel) = buttons(ui, "应用", Ok(()));
            if ok {
                match self.editor.rule() {
                    Ok(rule) => {
                        self.error = None;
                        action = Action::SetFilter(self.book, self.key.clone(), rule, self.visible_only);
                    }
                    Err(error) => self.error = Some(error),
                }
            } else if cancel {
                action = Action::Close;
            }
            if ui.button("清除本列筛选").clicked() {
                action = Action::SetFilter(self.book, self.key.clone(), None, self.visible_only);
            }
        });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_and_full_category_clear_the_rule() {
        assert_eq!(
            FilterEditor::Text {
                contains: String::new()
            }
            .rule(),
            Ok(None)
        );
        assert_eq!(
            FilterEditor::Text { contains: " a".into() }.rule(),
            Ok(Some(FilterRule::Text { contains: " a".into() }))
        );
        let all = FilterEditor::Category {
            levels: vec![("x".into(), true), (String::new(), true)],
        };
        assert_eq!(all.rule(), Ok(None));
        let none = FilterEditor::Category {
            levels: vec![("x".into(), false), (String::new(), false)],
        };
        assert!(none.rule().is_err());
        let some = FilterEditor::Category {
            levels: vec![("x".into(), true), (String::new(), false)],
        };
        assert_eq!(
            some.rule(),
            Ok(Some(FilterRule::Category {
                values: vec!["x".into()]
            }))
        );
        let blank_number = FilterEditor::Number {
            min: String::new(),
            max: String::new(),
        };
        assert_eq!(
            blank_number.rule(),
            Ok(Some(FilterRule::Number { min: None, max: None }))
        );
        let bad_number = FilterEditor::Number {
            min: "abc".into(),
            max: String::new(),
        };
        assert!(bad_number.rule().is_err());
    }
}
