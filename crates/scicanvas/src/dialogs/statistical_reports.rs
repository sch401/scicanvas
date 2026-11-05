use super::*;
use scicanvas_core::ops::Source;
pub(crate) enum StatisticsAction {
    Compare(Source, bool),
    WriteTable(String, Vec<Vec<String>>),
}
impl Dialog {
    pub(super) fn show_statistics_compare(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let Self::StatisticsCompare { source, names, boxes } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "并排统计比较").centered().open(open).show(ctx, |ui| {
            form(ui, "statistics-compare-form", |f| {
                f.row("数据列", |ui| {
                    ui.label(names.join("、"));
                });
                f.row("附图", |ui| {
                    ui.checkbox(boxes, "同时生成箱线图");
                });
            });
            hint(ui, "结果写入新表。");
            let (ok, cancel) = buttons(ui, "生成", Ok(()));
            if ok {
                action = Action::Statistics(Box::new(StatisticsAction::Compare(source.clone(), *boxes)));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
