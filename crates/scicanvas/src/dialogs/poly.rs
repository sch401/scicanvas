use super::*;

impl Dialog {
    pub(super) fn show_poly(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::Poly { order } = self else { unreachable!() };
        let mut action = Action::None;
        window(ctx, "多项式拟合").open(open).show(ctx, |ui| {
            data_hint(ui);
            ui.horizontal_wrapped(|ui| {
                ui.label("阶数")
                    .on_hover_text("阶数越高越容易过拟合，一般用 2 或 3 阶。");
                egui::ComboBox::from_id_salt("poly-order")
                    .selected_text(order.to_string())
                    .show_ui(ui, |ui| {
                        for k in 2..=6 {
                            ui.selectable_value(order, k, k.to_string());
                        }
                    });
            });
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
