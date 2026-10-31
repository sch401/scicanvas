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
                    ui.add(egui::DragValue::new(points).range(2..=100_000).speed(1.0));
                    ui.end_row();
                    ui.label("X 范围");
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(custom, "自定义");
                        ui.add_enabled(*custom, egui::DragValue::new(from).speed(0.1));
                        ui.label("到");
                        ui.add_enabled(*custom, egui::DragValue::new(to).speed(0.1));
                    });
                    ui.end_row();
                });
            ui.label(
                RichText::new(
                    "在等间距的 X 上求值，结果写成新的 X、Y 列并画成线。超出数据范围时按端点那一段外推。\nAkima 在数据突变处比三次样条更少过冲。",
                )
                .small()
                .color(pal.weak),
            );
            let valid = !*custom || (from.is_finite() && to.is_finite() && from < to);
            if !valid {
                ui.colored_label(egui::Color32::from_rgb(182, 67, 66), "起点必须小于终点");
            }
            let (ok, cancel) = buttons(ui, "插值");
            if ok && valid {
                action = Action::Interpolate(*method, *points, custom.then_some((*from, *to)));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
