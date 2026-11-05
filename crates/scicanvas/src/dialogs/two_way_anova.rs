use super::*;

impl Dialog {
    pub(super) fn show_two_way_anova(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let Self::TwoWayAnova { book, columns, keys } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "双因素方差分析").open(open).show(ctx, |ui| {
            for (i, label) in ["因子 A", "因子 B", "响应"].iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(*label);
                    egui::ComboBox::from_id_salt(("two-way-column", i))
                        .selected_text(
                            columns
                                .iter()
                                .find(|(k, _)| k == &keys[i])
                                .map_or("", |(_, name)| name.as_str()),
                        )
                        .show_ui(ui, |ui| {
                            for (key, name) in columns.iter() {
                                ui.selectable_value(&mut keys[i], key.clone(), name);
                            }
                        });
                });
            }
            ui.label("模型含交互项，有缺失值的行跳过。");
            let valid = keys[0] != keys[1] && keys[0] != keys[2] && keys[1] != keys[2];
            if !valid {
                ui.label("请选择三个不同的列");
            }
            let (ok, cancel) = buttons(ui, "计算");
            if ok && valid {
                action = Action::TwoWayAnova(*book, keys.clone());
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
