use super::*;

impl Dialog {
    pub(super) fn show_fft_filter(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::FftFilter { kind, lo, hi, nyquist } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "FFT 滤波").centered().open(open).show(ctx, |ui| {
            data_hint(ui);
            ui.horizontal_wrapped(|ui| {
                for (i, name) in ["低通", "高通", "带通", "带阻"].into_iter().enumerate() {
                    ui.radio_value(kind, i as u8, name);
                }
            });
            let band = *kind >= 2;
            form(ui, "fft-filter-grid", |ui| {
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
                RichText::new(format!("奈奎斯特频率 {}。X 需要等间距。", fmt_num(*nyquist)))
                    .small()
                    .color(pal.weak),
            );
            let valid: Result<(), String> = if !lo.is_finite() || *lo <= 0. || *lo > *nyquist {
                Err("截止频率应在 (0, 奈奎斯特] 内".into())
            } else if band && (!hi.is_finite() || *hi > *nyquist || *lo >= *hi) {
                Err("下限应小于上限，且上限不超过奈奎斯特频率".into())
            } else {
                Ok(())
            };
            if let Err(error) = &valid {
                field_error(ui, error);
            }
            let (ok, cancel) = buttons(ui, "滤波", valid);
            if ok {
                let (a, b) = (*lo, *hi);
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
