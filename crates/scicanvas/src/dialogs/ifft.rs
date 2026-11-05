use super::*;

impl Dialog {
    pub(super) fn show_ifft(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::Ifft {
            book,
            re,
            im,
            rows,
            two_sided,
            odd,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "IFFT").centered().open(open).show(ctx, |ui| {
            form(ui, "ifft-grid", |ui| {
                ui.label("实部");
                ui.label(&re.1);
                ui.end_row();
                ui.label("虚部");
                ui.label(im.as_ref().map_or("无（按 0）", |c| c.1.as_str()));
                ui.end_row();
                ui.label("输入");
                ui.horizontal_wrapped(|ui| {
                    ui.radio_value(two_sided, false, "单边谱");
                    ui.radio_value(two_sided, true, "双边谱");
                });
                ui.end_row();
                if !*two_sided {
                    use scicanvas_core::fft::InverseInput;
                    ui.label("输出点数");
                    ui.horizontal_wrapped(|ui| {
                        let even = InverseInput::OneSided { odd: false }.points(*rows);
                        let oddn = InverseInput::OneSided { odd: true }.points(*rows);
                        ui.radio_value(odd, false, even.to_string());
                        ui.radio_value(odd, true, oddn.to_string());
                    });
                    ui.end_row();
                }
            });
            let (ok, cancel) = buttons(ui, "变换", Ok(()));
            if ok {
                let input = if *two_sided {
                    scicanvas_core::fft::InverseInput::TwoSided
                } else {
                    scicanvas_core::fft::InverseInput::OneSided { odd: *odd }
                };
                action = Action::Ifft(*book, re.0.clone(), im.as_ref().map(|c| c.0.clone()), input);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
