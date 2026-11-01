use super::*;

#[derive(Clone, Copy)]
pub(crate) enum TransformKind {
    Stack,
    Split,
    Reduce,
}

pub struct SheetTransform {
    pub(crate) book: u64,
    pub(crate) kind: TransformKind,
    pub(crate) columns: Vec<(String, String, bool)>,
    pub(crate) value: usize,
    pub(crate) category: usize,
    pub(crate) step: usize,
    pub(crate) average: bool,
}

pub enum TransformAction {
    Stack(Vec<String>),
    Split(String, String),
    Reduce(String, usize, bool),
}

impl SheetTransform {
    pub(super) fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let mut action = Action::None;
        let title = match self.kind {
            TransformKind::Stack => "堆叠列",
            TransformKind::Split => "拆分列",
            TransformKind::Reduce => "数据精简",
        };
        window(ctx, title).open(open).show(ctx, |ui| {
            match self.kind {
                TransformKind::Stack => {
                    ui.label("选择要堆叠的列：");
                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                        for (_, name, selected) in &mut self.columns {
                            ui.checkbox(selected, name.as_str());
                        }
                    });
                }
                TransformKind::Split => {
                    column_choice(ui, "值列", &self.columns, &mut self.value);
                    column_choice(ui, "分类列", &self.columns, &mut self.category);
                }
                TransformKind::Reduce => {
                    column_choice(ui, "源列", &self.columns, &mut self.value);
                    ui.horizontal(|ui| {
                        ui.label("每 N 行");
                        ui.add(egui::DragValue::new(&mut self.step).range(1..=usize::MAX));
                    });
                    ui.radio_value(&mut self.average, false, "取第一行");
                    ui.radio_value(&mut self.average, true, "取平均值（忽略缺失和非有限数值）");
                }
            }
            let (ok, cancel) = buttons(ui, "确定");
            if ok {
                let selection = match self.kind {
                    TransformKind::Stack => {
                        TransformAction::Stack(self.columns.iter().filter(|c| c.2).map(|c| c.0.clone()).collect())
                    }
                    TransformKind::Split => TransformAction::Split(
                        self.columns[self.value].0.clone(),
                        self.columns[self.category].0.clone(),
                    ),
                    TransformKind::Reduce => {
                        TransformAction::Reduce(self.columns[self.value].0.clone(), self.step, self.average)
                    }
                };
                action = Action::SheetTransform(self.book, selection);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}

fn column_choice(ui: &mut egui::Ui, label: &str, columns: &[(String, String, bool)], selected: &mut usize) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(label)
            .selected_text(&columns[*selected].1)
            .show_ui(ui, |ui| {
                for (index, (_, name, _)) in columns.iter().enumerate() {
                    ui.selectable_value(selected, index, name);
                }
            });
    });
}
