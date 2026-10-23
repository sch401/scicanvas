use super::*;

impl Dialog {
    pub(super) fn show_baseline(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::Baseline { als, log_lambda, p } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "扣除基线").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.radio_value(als, false, "两端连线");
                ui.radio_value(als, true, "自动基线（ALS）");
            });
            ui.add_enabled_ui(*als, |ui| {
                egui::Grid::new("als-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("平滑度 log₁₀ λ");
                    ui.add(egui::DragValue::new(log_lambda).range(0.0..=12.0).speed(0.05).fixed_decimals(1));
                    ui.end_row();
                    ui.label("非对称系数 p");
                    ui.add(egui::DragValue::new(p).range(0.0001..=0.5).speed(0.001).fixed_decimals(4));
                    ui.end_row();
                });
            });
            ui.label(
                RichText::new(
                    "对当前图形的数据曲线，或工作簿里选中的 Y 列求基线，在 Y 右边写入“基线”和“扣除基线”两列；\n来源是图形时基线也画进图里。ALS（非对称最小二乘）适合光谱等峰朝上的数据：\nλ 越大基线越平直（常用 10²–10⁹），p 越小基线越贴近谷底（常用 0.001–0.05）。",
                )
                .small()
                .color(pal.weak),
            );
            let (ok, cancel) = buttons(ui, "扣除");
            if ok {
                action = Action::Baseline(if *als {
                    BaselineMethod::Als {
                        lambda: 10f64.powf(*log_lambda),
                        p: *p,
                    }
                } else {
                    BaselineMethod::Line
                });
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
