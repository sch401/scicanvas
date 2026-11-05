use super::*;

impl Dialog {
    pub(super) fn show_normalize(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::Normalize { method } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "归一化").centered().open(open).show(ctx, |ui| {
            for m in NormalizeMethod::ALL {
                ui.radio_value(method, m, m.label());
            }
            let (ok, cancel) = buttons(ui, "归一化", Ok(()));
            if ok {
                action = Action::Normalize(*method);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
