use super::*;

pub struct BatchPlot {
    pub(crate) graphs: Vec<(u64, String, bool)>,
    pub(crate) selected_graph: usize,
    pub(crate) books: Vec<(u64, String, bool)>,
}

impl BatchPlot {
    pub(super) fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "批量绘图").centered().open(open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("模板图形");
                egui::ComboBox::from_id_salt("batch-plot-template")
                    .selected_text(&self.graphs[self.selected_graph].1)
                    .show_ui(ui, |ui| {
                        for (index, (_, name, _)) in self.graphs.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_graph, index, name);
                        }
                    });
            });
            ui.weak("按长名称匹配列，没有长名称时按列标识（A、B…）匹配。");
            if self.graphs[self.selected_graph].2 {
                ui.colored_label(
                    crate::theme::palette_for(ctx).warn,
                    "此图形引用了其他工作簿的列，这些列同样会在所选工作簿中按列名匹配，不会保留原工作簿。",
                );
            }
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for (_, name, selected) in &mut self.books {
                    ui.checkbox(selected, name.as_str());
                }
            });
            let books: Vec<_> = self.books.iter().filter(|b| b.2).map(|b| b.0).collect();
            if books.is_empty() {
                field_error(ui, "请选择至少一个工作簿。");
            }
            let (ok, cancel) = buttons(
                ui,
                "生成图形",
                if !books.is_empty() {
                    Ok(())
                } else {
                    Err("请选择至少一个工作簿".into())
                },
            );
            if ok && !books.is_empty() {
                action = Action::BatchPlot(self.graphs[self.selected_graph].0, books);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
