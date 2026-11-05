use super::*;

impl Dialog {
    pub(super) fn show_fit_compare(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let Self::FitCompare { reports, first, second } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "拟合比较").open(open).show(ctx, |ui| {
            let mut target = (reports[*first].book_id, reports[*first].y_key.clone());
            for (label, index) in [("模型 1", &mut *first), ("模型 2", &mut *second)] {
                if label == "模型 2" && (reports[*index].book_id, reports[*index].y_key.clone()) != target {
                    *index = reports
                        .iter()
                        .position(|r| r.book_id == target.0 && r.y_key == target.1)
                        .unwrap();
                }
                ui.horizontal(|ui| {
                    ui.label(label);
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(format!(
                            "{}：{}（{}）",
                            reports[*index].name,
                            reports[*index].fit.model.name(),
                            reports[*index].curve.1
                        ))
                        .show_ui(ui, |ui| {
                            for (i, r) in reports.iter().enumerate() {
                                if label == "模型 2" && (r.book_id != target.0 || r.y_key != target.1) {
                                    continue;
                                }
                                ui.selectable_value(
                                    index,
                                    i,
                                    format!("{}：{}（{}）", r.name, r.fit.model.name(), r.curve.1),
                                );
                            }
                        });
                });
                if label == "模型 1" {
                    target = (reports[*index].book_id, reports[*index].y_key.clone());
                }
            }
            ui.label("F 检验只适用于嵌套模型。");
            let (ok, cancel) = buttons(ui, "比较");
            if ok {
                action = Action::CompareFits(Box::new([reports[*first].clone(), reports[*second].clone()]));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
