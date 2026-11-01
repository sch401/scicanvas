use super::*;

impl SciApp {
    pub(crate) fn open_export_xlsx(&mut self) {
        if self.project.books.is_empty() {
            self.status = "项目没有可导出的工作簿".into();
            return;
        }
        if !self.flush_sheet_edits() {
            return;
        }
        self.dialog = Some(Dialog::ExportXlsx { headers: true });
    }

    pub(super) fn export_xlsx(&mut self, headers: bool) {
        let Some(path) = self
            .file_dialog()
            .set_title("导出 Excel")
            .add_filter("Excel 工作簿", &["xlsx"])
            .set_file_name("SciCanvas.xlsx")
            .save_file()
        else {
            return;
        };
        self.settings.last_dir = path.parent().map(std::path::Path::to_path_buf);
        self.status = match scicanvas_core::project_file::save_xlsx_export(&path, &self.project, headers) {
            Ok(()) => format!("已导出 {}", path.display()),
            Err(error) => format!("导出失败：{error}"),
        };
    }
}
