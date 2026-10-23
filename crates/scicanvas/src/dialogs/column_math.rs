use super::*;

impl Dialog {
    pub(super) fn show_column_math(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
    ) -> Action {
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
        window(ctx, "列运算").open(open).show(ctx, |ui| {
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
            ui.label(
                RichText::new(
                    "结果写成新列放在第一列右边。两列对应不同的 X 列时（如原始曲线减拟合曲线），\n先把第二列按线性插值到第一列的 X 上；除以 0 或缺失值处留空。\n只选一列时与常数运算，选两列时按点选顺序。",
                )
                .small()
                .color(pal.weak),
            );
            let (ok, cancel) = buttons(ui, "计算");
            if ok {
                action = Action::ColumnMath(*op, use_constant.then_some(*constant));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
