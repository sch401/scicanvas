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
            ui.label("按列比较 N、均值、样本标准差、SE、分位数、极值和缺失数。结果写入新表，箱线图引用原数据。");
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
