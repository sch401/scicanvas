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
        window(ctx, "标准曲线反查")
            .centered()
            .default_width(crate::theme::dialog::MEDIUM)
            .open(open)
            .show(ctx, |ui| {
                form(ui, "calibration-form", |f| {
                    f.row("方向", |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(forward, false, "由 Y 求 X");
                            ui.selectable_value(forward, true, "由 X 求 Y");
                        });
                    });
                    f.row("模型", |ui| {
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
                    });
                    for (i, label) in ["标准 X", "标准 Y", "样品"].iter().enumerate() {
                        f.row(label, |ui| {
                            ui.add_enabled_ui(i == 2 || model.is_none(), |ui| {
                                ui.vertical(|ui| {
                                    egui::ComboBox::from_id_salt(("calibration-column", i))
                                        .selected_text(
                                            columns
                                                .iter()
                                                .find(|(k, _)| k == &keys[i])
                                                .map_or("请选择列", |(_, name)| name.as_str()),
                                        )
                                        .show_ui(ui, |ui| {
                                            for (key, name) in columns.iter() {
                                                ui.selectable_value(&mut keys[i], key.clone(), name);
                                            }
                                        });
                                    if !columns.iter().any(|(key, _)| key == &keys[i]) {
                                        field_error(ui, "请选择数据列");
                                    } else if model.is_none() && keys[..i].contains(&keys[i]) {
                                        field_error(ui, "请选择不同的列");
                                    }
                                });
                            });
                        });
                    }
                });
                hint(ui, "结果写在样品列右侧，附 95% 区间。");
                let required = if model.is_none() { keys.as_slice() } else { &keys[2..] };
                let valid = if required.iter().any(|key| !columns.iter().any(|(k, _)| k == key)) {
                    Err("请选择标准列和样品列".into())
                } else if required.iter().enumerate().any(|(i, k)| required[..i].contains(k)) {
                    Err("标准 X、标准 Y 和样品应为不同列".into())
                } else {
                    Ok(())
                };
                let (ok, cancel) = buttons(ui, "计算", valid);
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
