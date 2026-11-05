use super::*;

impl Dialog {
    pub(super) fn show_fill(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::Fill {
            target,
            kind,
            a,
            b,
            rows,
            filtered,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "填充数据序列").open(open).show(ctx, |ui| {
            ui.label(RichText::new(format!("目标列：{target}")).strong());
            if *filtered {
                ui.colored_label(pal.warn, "工作表已筛选：填充会从第 1 行开始写入，包括隐藏行。");
            }
            ui.horizontal_wrapped(|ui| {
                for (i, name) in ["等差序列", "均匀分布随机数", "正态分布随机数"].into_iter().enumerate()
                {
                    ui.radio_value(kind, i as u8, name);
                }
            });
            let (la, lb) = match kind {
                0 => ("首项", "公差"),
                1 => ("最小值", "最大值"),
                _ => ("均值", "标准差"),
            };
            egui::Grid::new("fill-grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label(la);
                    ui.add(egui::DragValue::new(a).speed(0.1));
                    ui.end_row();
                    ui.label(lb);
                    ui.add(egui::DragValue::new(b).speed(0.1));
                    ui.end_row();
                    ui.label("行数");
                    ui.add(egui::DragValue::new(rows).range(1..=1_000_000).speed(1.0));
                    ui.end_row();
                });
            ui.label(RichText::new("结果覆盖目标列。").small().color(pal.weak));
            let (ok, cancel) = buttons(ui, "填充");
            if ok {
                let seq = match kind {
                    0 => Sequence::Arithmetic { start: *a, step: *b },
                    1 => Sequence::Uniform {
                        min: a.min(*b),
                        max: a.max(*b),
                    },
                    _ => Sequence::Normal { mean: *a, sd: b.abs() },
                };
                action = Action::Fill(seq, *rows);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
