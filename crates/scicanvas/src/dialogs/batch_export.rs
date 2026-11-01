use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BatchFormat {
    Svg,
    Pdf,
    Png,
    Tiff,
}
impl BatchFormat {
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Png => "png",
            Self::Tiff => "tif",
        }
    }
}

pub struct BatchExport {
    pub(crate) graphs: Vec<(u64, String, bool)>,
    pub(crate) format: BatchFormat,
    pub(crate) dpi: u32,
}

impl BatchExport {
    pub(super) fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "批量导出图形").open(open).show(ctx, |ui| {
            ui.label("选择图形：");
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for (_, name, selected) in &mut self.graphs {
                    ui.checkbox(selected, name.as_str());
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("格式");
                for (kind, label) in [
                    (BatchFormat::Svg, "SVG"),
                    (BatchFormat::Pdf, "PDF"),
                    (BatchFormat::Png, "PNG"),
                    (BatchFormat::Tiff, "TIFF"),
                ] {
                    ui.radio_value(&mut self.format, kind, label);
                }
            });
            if matches!(self.format, BatchFormat::Png | BatchFormat::Tiff) {
                ui.horizontal(|ui| {
                    ui.label("分辨率");
                    ui.add(egui::DragValue::new(&mut self.dpi).range(72..=2400).suffix(" dpi"));
                });
            }
            let selected: Vec<_> = self.graphs.iter().filter(|g| g.2).map(|g| g.0).collect();
            if selected.is_empty() {
                ui.label("请选择至少一张图。");
            }
            let (ok, cancel) = buttons(ui, "选择目录并导出…");
            if ok && !selected.is_empty() {
                action = Action::BatchExport(selected, self.format, self.dpi);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
