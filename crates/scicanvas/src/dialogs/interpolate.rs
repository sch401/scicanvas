use super::*;

impl Dialog {
    pub(super) fn show_interpolate(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
    ) -> Action {
        let Self::Interpolate {
            mode,
            target,
            single,
            result,
            options,
            method,
            points,
            custom,
            from,
            to,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "插值").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("输出 X");
                ui.radio_value(mode, 0, "均匀网格");
                ui.radio_value(mode, 1, "目标 X 列");
                ui.radio_value(mode, 2, "单点查询");
            });
            if *mode == 1 {
                egui::ComboBox::from_id_salt("interpolation-target")
                    .selected_text(
                        options
                            .iter()
                            .find(|o| Some(&o.0) == target.as_ref())
                            .map_or("请选择列", |o| o.1.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        for (key, name) in options.iter() {
                            ui.selectable_value(target, Some(key.clone()), name);
                        }
                    });
            }
            if *mode == 2 {
                ui.horizontal(|ui| {
                    ui.label("查询 X");
                    ui.add(egui::DragValue::new(single).speed(0.1));
                });
                for (name, value) in result.iter() {
                    ui.label(format!("{name}: Y = {}", fmt_num(*value)));
                }
            }

            egui::Grid::new("interp-grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("方法");
                    ui.horizontal_wrapped(|ui| {
                        for m in InterpMethod::ALL {
                            ui.radio_value(method, m, m.label());
                        }
                    });
                    ui.end_row();
                    ui.label("点数");
                    ui.add_enabled(*mode == 0, egui::DragValue::new(points).range(2..=100_000).speed(1.0));
                    ui.end_row();
                    ui.label("X 范围");
                    ui.horizontal_wrapped(|ui| {
                        ui.add_enabled(*mode == 0, egui::Checkbox::new(custom, "自定义"));
                        ui.add_enabled(*mode == 0 && *custom, egui::DragValue::new(from).speed(0.1));
                        ui.label("到");
                        ui.add_enabled(*mode == 0 && *custom, egui::DragValue::new(to).speed(0.1));
                    });
                    ui.end_row();
                });
            ui.label(
                RichText::new(match *mode {
                    1 => "结果写在目标列右侧。",
                    2 => "结果只显示，不写表。",
                    _ => "结果写成新的 X、Y 列。",
                })
                .small()
                .color(pal.weak),
            );
            let valid = match *mode {
                1 => target.is_some(),
                2 => single.is_finite(),
                _ => !*custom || (from.is_finite() && to.is_finite() && from < to),
            };
            if !valid {
                ui.colored_label(
                    pal.error,
                    match *mode {
                        1 => "请选择目标 X 列",
                        2 => "查询 X 应为有限数",
                        _ => "起点必须小于终点",
                    },
                );
            }
            let (ok, cancel) = buttons(ui, if *mode == 2 { "查询" } else { "插值" });
            if ok && valid {
                action = match *mode {
                    1 => Action::SignalTools(Box::new(SignalAction::Interpolate(*method, target.clone().unwrap()))),
                    2 => Action::SignalTools(Box::new(SignalAction::Query(*method, *single))),
                    _ => Action::Interpolate(*method, *points, custom.then_some((*from, *to))),
                };
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
