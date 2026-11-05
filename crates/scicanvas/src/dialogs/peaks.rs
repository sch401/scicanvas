use super::*;

impl Dialog {
    pub(super) fn show_peaks(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::Peaks {
            percent,
            derivative,
            window,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        super::window(ctx, "寻峰").centered().open(open).show(ctx, |ui| {
            data_hint(ui);
            form(ui, "peaks-form", |f| {
                f.row("方法", |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(derivative, false, "局部极大");
                        ui.radio_value(derivative, true, "导数法");
                    });
                });
                if *derivative {
                    f.row("SG 窗口", |ui| {
                        odd_window(ui, window);
                    });
                }
                f.label("最小突出度")
                    .on_hover_text("峰顶高出两侧较高谷底的量，调大可滤掉小峰。");
                f.add(egui::DragValue::new(percent).range(0.1..=100.).speed(0.2).suffix(" %"));
                f.end_row();
            });
            let valid: Result<(), String> = if !percent.is_finite() {
                Err("突出度应为有限数".into())
            } else if *derivative && (*window < 3 || *window % 2 == 0) {
                Err("SG 窗口应为不小于 3 的奇数".into())
            } else {
                Ok(())
            };
            if let Err(error) = &valid {
                field_error(ui, error);
            }
            hint(ui, "每条曲线生成一个峰表。");
            let (ok, cancel) = buttons(ui, "寻峰", valid);
            if ok {
                action = if *derivative {
                    Action::SignalTools(Box::new(SignalAction::Peaks(*window, *percent / 100.)))
                } else {
                    Action::Peaks(*percent / 100.0)
                };
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
