//! 确认框独立于可停靠设置面板，始终居中并阻挡背后指针。
use eframe::egui::{self, Color32, Id, RichText};

pub(crate) fn backdrop(ctx: &egui::Context) -> Color32 {
    crate::theme::palette_for(ctx).scrim(if ctx.style().visuals.dark_mode { 120 } else { 70 })
}

pub(crate) fn modal<R>(ctx: &egui::Context, title: &str, contents: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    let id = Id::new(("scicanvas-modal", title));
    let caption = crate::chrome::caption(ctx);
    let frame = egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::same(14.));
    let out = egui::Modal::new(id)
        .frame(frame)
        .backdrop_color(backdrop(ctx))
        .show(ctx, |ui| {
            let frame = ctx.cumulative_pass_nr();
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("keyboard-modal-layer"), (frame, ui.layer_id())));
            ui.set_min_width(320.);
            ui.set_max_width(460.);
            ui.heading(RichText::new(title).strong());
            ui.add_space(6.);
            let result = contents(ui);
            if let Some(caption) = caption {
                let pal = crate::theme::palette_for(ctx);
                let mut caption_ui = ui.new_child(egui::UiBuilder::new().max_rect(caption));
                crate::chrome::modal_caption(&mut caption_ui, caption, &pal);
            }
            result
        });
    Some(out.inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modal_does_not_register_dock_state() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            modal(ctx, "未保存的修改", |ui| {
                ui.label("内容");
            });
        });
        assert!(super::super::docking::docks(&ctx).panels.is_empty());
    }
}
