use super::*;

impl Dialog {
    pub(super) fn show_category_order(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
    ) -> Action {
        let Self::CategoryOrder {
            book: _,
            key: _,
            levels,
            error,
        } = self
        else {
            unreachable!()
        };
        window(ctx, "类别顺序").open(open).show(ctx, |ui| {
            let mut swap = None;
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                egui::Grid::new("category-order")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        let n = levels.len();
                        for (i, name) in levels.iter().enumerate() {
                            ui.label(RichText::new(format!("{}", i + 1)).color(pal.weak));
                            ui.label(name);
                            ui.horizontal_wrapped(|ui| {
                                if ui.add_enabled(i > 0, egui::Button::new("↑").small()).clicked() {
                                    swap = Some((i, i - 1));
                                }
                                if ui.add_enabled(i + 1 < n, egui::Button::new("↓").small()).clicked() {
                                    swap = Some((i, i + 1));
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
            if let Some((a, b)) = swap {
                levels.swap(a, b);
            }
            if ui.add(plain("按名称排序")).clicked() {
                levels.sort();
            }
            if let Some(error) = error {
                ui.colored_label(ui.visuals().error_fg_color, error.as_str());
            }
        });
        Action::None
    }
}
