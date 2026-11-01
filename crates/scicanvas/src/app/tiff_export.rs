use super::*;

impl SciApp {
    pub(crate) fn export_tiff_dialog(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(graph) = self.project.graph(gid) else { return };
        self.dialog = Some(Dialog::Tiff {
            dpi: 600,
            page_mm: (graph.page.width_mm, graph.page.height_mm),
        });
    }

    pub(super) fn export_tiff(&mut self, dpi: u32) {
        let Some(gid) = self.target_graph() else { return };
        let Some(graph) = self.project.graph(gid).cloned() else {
            return;
        };
        let Some(path) = self
            .file_dialog()
            .set_title("导出 TIFF")
            .add_filter("TIFF 图像", &["tif", "tiff"])
            .set_file_name(format!("{}.tif", graph.name))
            .save_file()
        else {
            return;
        };
        let result = crate::export::rasterize(&svg::render_page(&graph, &self.project), dpi).and_then(|pixmap| {
            let bytes = crate::export::tiff_bytes(&pixmap, dpi)?;
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            Ok((pixmap.width(), pixmap.height()))
        });
        self.status = match result {
            Ok((width, height)) => format!("已导出 {}（{width} × {height} 像素，{dpi} dpi）", path.display()),
            Err(error) => format!("导出失败：{error}"),
        };
    }
}
