use super::*;

impl Dialog {
    pub(super) fn show_fft_filter(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
    ) -> Action {
        let Self::FftFilter { kind, lo, hi, nyquist } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "FFT 滤波").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, name) in ["低通", "高通", "带通", "带阻"].into_iter().enumerate() {
                    ui.radio_value(kind, i as u8, name);
                }
            });
            let band = *kind >= 2;
            egui::Grid::new("fft-filter-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(if band { "下限频率" } else { "截止频率" });
                ui.add(egui::DragValue::new(lo).range(0.0..=f64::MAX).speed(*nyquist / 200.0));
                ui.end_row();
                if band {
                    ui.label("上限频率");
                    ui.add(egui::DragValue::new(hi).range(0.0..=f64::MAX).speed(*nyquist / 200.0));
                    ui.end_row();
                }
            });
            ui.label(
                RichText::new(format!(
                    "奈奎斯特频率 {}（频率单位为 1/X，X 以 s 为单位时为 Hz）。\n对当前图形的数据曲线，或工作簿里选中的 Y 列做理想滤波：变换后把不通过的频率分量置零再逆变换。\n结果写成新列；来源是图形时也画进图里。X 需要等间距。",
                    fmt_num(*nyquist)
                ))
                .small()
                .color(pal.weak),
            );
            let (ok, cancel) = buttons(ui, "滤波");
            if ok {
                let (a, b) = (lo.min(*hi), lo.max(*hi));
                action = Action::FftFilter(match kind {
                    0 => FilterKind::LowPass(*lo),
                    1 => FilterKind::HighPass(*lo),
                    2 => FilterKind::BandPass(a, b),
                    _ => FilterKind::BandStop(a, b),
                });
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
