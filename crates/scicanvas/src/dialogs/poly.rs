use super::*;

impl Dialog {
    pub(super) fn show_poly(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::Poly { order } = self else { unreachable!() };
        let mut action = Action::None;
        window(ctx, "多项式拟合").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("阶数");
                egui::ComboBox::from_id_salt("poly-order")
                    .selected_text(order.to_string())
                    .show_ui(ui, |ui| {
                        for k in 2..=6 {
                            ui.selectable_value(order, k, k.to_string());
                        }
                    });
            });
            ui.label(
                RichText::new(
                    "对当前图形的数据曲线，或工作簿里选中的 Y 列拟合。\n阶数越高越容易过拟合，一般用 2 或 3 阶。",
                )
                .small()
                .color(pal.weak),
            );
            let (ok, cancel) = buttons(ui, "拟合");
            if ok {
                action = Action::Fit(*order);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
