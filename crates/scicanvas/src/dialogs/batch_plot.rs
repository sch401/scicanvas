use super::*;

pub struct BatchPlot {
    pub(crate) graphs: Vec<(u64, String)>,
    pub(crate) selected_graph: usize,
    pub(crate) books: Vec<(u64, String, bool)>,
}

impl BatchPlot {
    pub(super) fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "批量绘图").open(open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("模板图形");
                egui::ComboBox::from_id_salt("batch-plot-template")
                    .selected_text(&self.graphs[self.selected_graph].1)
                    .show_ui(ui, |ui| {
                        for (index, (_, name)) in self.graphs.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_graph, index, name);
                        }
                    });
            });
            ui.label("选择工作簿，按列名匹配：");
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for (_, name, selected) in &mut self.books {
                    ui.checkbox(selected, name.as_str());
                }
            });
            let books: Vec<_> = self.books.iter().filter(|b| b.2).map(|b| b.0).collect();
            if books.is_empty() {
                ui.label("请选择至少一个工作簿。");
            }
            let (ok, cancel) = buttons(ui, "生成图形");
            if ok && !books.is_empty() {
                action = Action::BatchPlot(self.graphs[self.selected_graph].0, books);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
