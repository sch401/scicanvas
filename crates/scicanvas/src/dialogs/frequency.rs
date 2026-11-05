use super::*;

impl Dialog {
    pub(super) fn show_frequency(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::Frequency {
            auto,
            bins,
            custom,
            from,
            to,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "频数统计").centered().open(open).show(ctx, |ui| {
            form(ui, "freq-grid", |ui| {
                ui.label("区间数");
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(auto, "自动（Sturges）");
                    ui.add_enabled(!*auto, egui::DragValue::new(bins).range(1..=10_000));
                });
                ui.end_row();
                ui.label("统计范围");
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(custom, "自定义");
                    ui.add_enabled(*custom, egui::DragValue::new(from).speed(0.1));
                    ui.label("到");
                    ui.add_enabled(*custom, egui::DragValue::new(to).speed(0.1));
                });
                ui.end_row();
            });
            hint(ui, "区间等宽，最后一个区间含右端点。");
            let valid = !*custom || (from.is_finite() && to.is_finite() && from < to);
            if !valid {
                field_error(ui, "起点必须小于终点");
            }
            let (ok, cancel) = buttons(
                ui,
                "统计",
                if valid {
                    Ok(())
                } else {
                    Err("起点必须小于终点".into())
                },
            );
            if ok && valid {
                action = Action::Frequency((!*auto).then_some(*bins), custom.then_some((*from, *to)));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
