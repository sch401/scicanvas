use super::*;

impl Dialog {
    pub(super) fn show_column_math(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::ColumnMath {
            a,
            b,
            op,
            use_constant,
            constant,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "列运算").centered().open(open).show(ctx, |ui| {
            if b.is_none() {
                *use_constant = true;
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(a.as_str()).strong());
                for o in ColumnOp::ALL {
                    ui.selectable_value(op, o, o.symbol());
                }
                if *use_constant {
                    ui.add(egui::DragValue::new(constant).speed(0.1));
                } else if let Some(b) = b.as_ref() {
                    ui.label(RichText::new(b.as_str()).strong());
                }
            });
            ui.add_enabled_ui(b.is_some(), |ui| {
                ui.checkbox(use_constant, "用常数代替第二列");
            });
            hint(ui, "两列 X 不同时，先把第二列插值到第一列的 X 上。");
            let (ok, cancel) = buttons(ui, "计算", Ok(()));
            if ok {
                action = Action::ColumnMath(*op, use_constant.then_some(*constant));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
