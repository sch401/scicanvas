use super::*;

impl Dialog {
    pub(super) fn show_export_xlsx(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let Self::ExportXlsx { headers } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "导出 Excel").open(open).show(ctx, |ui| {
            ui.label("每个工作簿导出为一个工作表。");
            ui.label("日期按列的显示时区导出，Excel 不保存时区。");
            ui.label("非有限数值作为文本导出。");
            ui.checkbox(headers, "包含长名、单位和注释三行表头");
            let (ok, cancel) = buttons(ui, "导出");
            if ok {
                action = Action::ExportXlsx(*headers);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
