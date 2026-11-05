use super::*;

impl Dialog {
    pub(super) fn show_png(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let (dpi, page_mm, is_tiff) = match self {
            Self::Png { dpi, page_mm } => (dpi, page_mm, false),
            Self::Tiff { dpi, page_mm } => (dpi, page_mm, true),
            _ => unreachable!(),
        };
        let mut action = Action::None;
        window(ctx, if is_tiff { "导出 TIFF" } else { "导出 PNG" })
            .centered()
            .open(open)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("分辨率");
                    for preset in [150, 300, 600, 1200] {
                        ui.selectable_value(dpi, preset, format!("{preset} dpi"));
                    }
                    ui.add(egui::DragValue::new(dpi).range(72..=2400).suffix(" dpi"));
                });
                let px = |mm: f64| (mm / 25.4 * f64::from(*dpi)).round();
                ui.label(format!(
                    "页面 {} × {} mm，输出约 {} × {} 像素",
                    fmt_num(page_mm.0),
                    fmt_num(page_mm.1),
                    px(page_mm.0),
                    px(page_mm.1)
                ));
                hint(ui, "线条图投稿一般用 600 dpi。");
                let (ok, cancel) = buttons(ui, "导出…", Ok(()));
                if ok {
                    action = if is_tiff {
                        Action::ExportTiff(*dpi)
                    } else {
                        Action::ExportPng(*dpi)
                    };
                } else if cancel {
                    action = Action::Close;
                }
            });
        action
    }
}
