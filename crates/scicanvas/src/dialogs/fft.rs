use super::*;

impl Dialog {
    pub(super) fn show_fft(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::Fft { window: w } = self else { unreachable!() };
        let mut action = Action::None;
        window(ctx, "FFT").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("窗函数");
                for m in FftWindow::ALL {
                    ui.radio_value(w, m, m.label());
                }
            });
            ui.label(
                RichText::new(
                    "对当前图形的数据曲线，或工作簿里选中的 Y 列做 FFT。X 需要等间距（X 以 s 为单位时频率单位为 Hz）。\n每条曲线生成一个频谱表（频率、实部、虚部、幅值、相位、功率）和一张幅值谱图；\n幅值按正弦振幅归一，加窗时按窗函数的相干增益补偿。",
                )
                .small()
                .color(pal.weak),
            );
            let (ok, cancel) = buttons(ui, "变换");
            if ok {
                action = Action::Fft(*w);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
