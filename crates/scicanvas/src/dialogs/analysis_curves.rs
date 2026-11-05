use super::*;

impl Dialog {
    pub(super) fn show_analysis_curves(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::AnalysisCurves { curves, .. } = self else {
            unreachable!()
        };
        window(ctx, "分析曲线").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("全选").clicked() {
                    for (_, _, checked) in curves.iter_mut() {
                        *checked = true;
                    }
                }
                if ui.button("清空").clicked() {
                    for (_, _, checked) in curves.iter_mut() {
                        *checked = false;
                    }
                }
            });
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for (_, name, checked) in curves.iter_mut() {
                    ui.checkbox(checked, name.as_str());
                }
            });
        });
        Action::None
    }
}
