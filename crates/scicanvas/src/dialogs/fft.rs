use super::*;

impl Dialog {
    pub(super) fn show_fft(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::Fft { window: w } = self else { unreachable!() };
        let mut action = Action::None;
        window(ctx, "FFT").centered().open(open).show(ctx, |ui| {
            data_hint(ui);
            ui.horizontal_wrapped(|ui| {
                ui.label("窗函数")
                    .on_hover_text("幅值按正弦振幅归一，加窗时按窗函数相干增益补偿。");
                for m in FftWindow::ALL {
                    ui.radio_value(w, m, m.label());
                }
            });
            hint(ui, "X 需要等间距。");
            let (ok, cancel) = buttons(ui, "变换", Ok(()));
            if ok {
                action = Action::Fft(*w);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
