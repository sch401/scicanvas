//! 确认框独立于可停靠设置面板，始终居中并阻挡背后指针。
use eframe::egui::{self, Align2, Color32, Id, LayerId, Order, RichText};

pub(crate) fn backdrop(ctx: &egui::Context) -> Color32 {
    crate::theme::palette_for(ctx).scrim(if ctx.style().visuals.dark_mode { 120 } else { 70 })
}

pub(crate) fn modal<R>(ctx: &egui::Context, title: &str, contents: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    let id = Id::new(("scicanvas-modal", title));
    let screen = ctx.screen_rect();
    let caption = crate::chrome::caption(ctx);
    let shaded = caption.map_or(screen, |caption| {
        egui::Rect::from_min_max(egui::pos2(screen.left(), caption.bottom()), screen.max)
    });
    egui::Area::new(id.with("backdrop"))
        .order(Order::Foreground)
        .fixed_pos(shaded.min)
        .show(ctx, |ui| {
            ui.allocate_rect(shaded, egui::Sense::click_and_drag());
            ui.painter().rect_filled(shaded, 0., backdrop(ctx));
        });
    if let Some(caption) = caption {
        let pal = crate::theme::palette_for(ctx);
        egui::Area::new(id.with("caption"))
            .order(Order::Foreground)
            .fixed_pos(caption.min)
            .show(ctx, |ui| crate::chrome::modal_caption(ui, caption, &pal));
    }
    let out = egui::Area::new(id)
        .order(Order::Foreground)
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .movable(false)
        .show(ctx, |ui| {
            egui::Frame::window(ui.style())
                .inner_margin(egui::Margin::same(14.))
                .show(ui, |ui| {
                    ui.set_min_width(320.);
                    ui.set_max_width(460.);
                    ui.heading(RichText::new(title).strong());
                    ui.add_space(6.);
                    contents(ui)
                })
                .inner
        });
    ctx.move_to_top(LayerId::new(Order::Foreground, id));
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
