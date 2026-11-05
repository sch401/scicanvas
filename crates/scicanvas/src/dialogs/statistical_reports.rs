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
        window(ctx, "并排统计比较").open(open).show(ctx, |ui| {
            ui.label(format!("数据列：{}", names.join("、")));
            ui.checkbox(boxes, "同时生成箱线图");
            ui.label("结果写入新表。");
            let (ok, cancel) = buttons(ui, "生成");
            if ok {
                action = Action::Statistics(Box::new(StatisticsAction::Compare(source.clone(), *boxes)));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
