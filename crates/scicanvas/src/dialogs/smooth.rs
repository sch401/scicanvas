use super::*;

impl Dialog {
    pub(super) fn show_smooth(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::Smooth { method, window: w } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "平滑").centered().open(open).show(ctx, |ui| {
            form(ui, "smooth-grid", |ui| {
                ui.label("方法");
                egui::ComboBox::from_id_salt("smooth-method")
                    .selected_text(method.label())
                    .show_ui(ui, |ui| {
                        for m in SmoothMethod::ALL {
                            ui.selectable_value(method, m, m.label());
                        }
                    });
                ui.end_row();
                ui.label("窗口点数");
                ui.add(egui::DragValue::new(w).range(3..=101).speed(1.0));
                ui.end_row();
            });
            ui.label(
                RichText::new(match method {
                    SmoothMethod::Lowess => "窗口点数即每点参与局部回归的近邻数。",
                    SmoothMethod::FftFilter => "滤掉高于 1/(窗口点数·ΔX) 的频率，需要等间距 X。",
                    _ => "窗口点数取奇数。Savitzky-Golay 能更好地保留峰形。",
                })
                .small()
                .color(pal.weak),
            );
            let (ok, cancel) = buttons(ui, "平滑", Ok(()));
            if ok {
                action = Action::Smooth(*method, (*w).max(3) | 1);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
