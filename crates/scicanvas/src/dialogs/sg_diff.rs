use super::*;

impl Dialog {
    pub(super) fn show_sg_diff(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::SgDiff {
            window: w,
            order,
            deriv,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "SG 平滑求导").centered().open(open).show(ctx, |ui| {
            data_hint(ui);
            form(ui, "sgdiff-grid", |ui| {
                ui.label("导数阶数");
                ui.horizontal_wrapped(|ui| {
                    ui.radio_value(deriv, 1, "一阶");
                    ui.radio_value(deriv, 2, "二阶");
                });
                ui.end_row();
                ui.label("窗口点数");
                odd_window(ui, w);
                ui.end_row();
                ui.label("多项式阶数");
                ui.add(egui::DragValue::new(order).range(1..=6).speed(0.1));
                ui.end_row();
            });
            // 窗口取奇数；阶数不低于导数阶数，并小于窗口点数 − 1
            *order = (*order).clamp((*deriv).max(1), *w - 2);
            let (ok, cancel) = buttons(
                ui,
                "求导",
                if *w % 2 == 1 {
                    Ok(())
                } else {
                    Err("窗口点数应为奇数".into())
                },
            );
            if ok {
                action = Action::SgDiff(*w, *order, *deriv);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
