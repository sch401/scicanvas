use super::*;

impl Dialog {
    pub(super) fn show_calibration(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let Self::Calibration {
            book,
            columns,
            keys,
            reports,
            model,
            forward,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "标准曲线正反查").open(open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(forward, false, "由 Y 求 X");
                ui.selectable_value(forward, true, "由 X 求 Y");
            });
            egui::ComboBox::from_id_salt("calibration-model")
                .selected_text(model.map_or("线性".to_owned(), |i| {
                    format!("{}：{}", reports[i].name, reports[i].fit.model.name())
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(model, None, "线性");
                    for (i, r) in reports.iter().enumerate() {
                        ui.selectable_value(model, Some(i), format!("{}：{}", r.name, r.fit.model.name()));
                    }
                });
            for (i, label) in ["标准 X", "标准 Y", "样品"].iter().enumerate() {
                ui.add_enabled_ui(i == 2 || model.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(*label);
                        egui::ComboBox::from_id_salt(("calibration-column", i))
                            .selected_text(
                                columns
                                    .iter()
                                    .find(|(k, _)| k == &keys[i])
                                    .map_or("", |(_, n)| n.as_str()),
                            )
                            .show_ui(ui, |ui| {
                                for (key, name) in columns.iter() {
                                    ui.selectable_value(&mut keys[i], key.clone(), name);
                                }
                            });
                    });
                });
            }
            ui.label("结果写在样品列右侧，附 95% 区间。");
            let (ok, cancel) = buttons(ui, "计算");
            if ok {
                action = Action::EvaluateCalibration {
                    book: *book,
                    keys: keys.clone(),
                    report: model.map(|i| Box::new(reports[i].clone())),
                    forward: *forward,
                };
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
