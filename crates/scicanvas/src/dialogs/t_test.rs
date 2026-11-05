use super::*;

impl Dialog {
    pub(super) fn show_t_test(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::TTest {
            columns,
            kind,
            mu,
            equal_var,
            level,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "t 检验").centered().open(open).show(ctx, |ui| {
            const KINDS: [&str; 3] = ["单样本", "双样本（独立）", "配对"];
            ui.label(format!("参与检验的列：{}", columns.join("、")));
            ui.horizontal_wrapped(|ui| {
                for (i, label) in KINDS.iter().enumerate() {
                    ui.radio_value(kind, i, *label);
                }
            });
            form(ui, "ttest-grid", |ui| {
                if *kind == 0 {
                    ui.label("假设均值");
                    ui.add(egui::DragValue::new(mu).speed(0.1));
                    ui.end_row();
                }
                if *kind == 1 {
                    ui.label("方差");
                    ui.checkbox(equal_var, "假设两组方差相等（不勾选为 Welch 检验）");
                    ui.end_row();
                }
                ui.label("置信水平");
                ui.horizontal_wrapped(|ui| {
                    for l in [0.9, 0.95, 0.99] {
                        ui.selectable_value(level, l, format!("{}%", fmt_num(l * 100.0)));
                    }
                });
                ui.end_row();
            });
            let need = if *kind == 0 { 1 } else { 2 };
            let ready = columns.len() >= need;
            let note = match *kind {
                0 => "使用第 1 列。".to_string(),
                1 => "比较第 1 列（A）与第 2 列（B）两组独立样本。".to_string(),
                _ => "按行配对第 1 列（A）与第 2 列（B），只用两列都有数值的行。".to_string(),
            };
            hint(ui, &note);
            if !ready {
                field_error(ui, "请先在工作表里按顺序单击选中两列（Ctrl+单击多选）");
            }
            if *kind == 0 && !mu.is_finite() {
                field_error(ui, "假设均值应为有限数");
            }
            let (ok, cancel) = buttons(
                ui,
                "计算",
                if !ready {
                    Err("请选择足够的数据列".into())
                } else if *kind == 0 && !mu.is_finite() {
                    Err("假设均值应为有限数".into())
                } else {
                    Ok(())
                },
            );
            if ok && ready {
                let k = match *kind {
                    0 => TTestKind::OneSample { mu: *mu },
                    1 => TTestKind::TwoSample { equal_var: *equal_var },
                    _ => TTestKind::Paired,
                };
                action = Action::TTest(k, *level);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
