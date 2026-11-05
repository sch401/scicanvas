use super::*;

impl Dialog {
    pub(super) fn show_table(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::Table { title, rows, note } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, title).centered().open(open).show(ctx, |ui| {
            egui::ScrollArea::both().max_height(420.0).show(ui, |ui| {
                egui::Grid::new("result-table")
                    .striped(true)
                    .num_columns(rows.first().map_or(1, Vec::len))
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for (ri, row) in rows.iter().enumerate() {
                            for (ci, cell) in row.iter().enumerate() {
                                if ri == 0 {
                                    ui.label(RichText::new(cell).color(pal.weak));
                                } else if ci == 0 {
                                    ui.label(cell);
                                } else {
                                    ui.label(RichText::new(cell).monospace());
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
            ui.label(RichText::new(note.as_str()).small().color(pal.weak));
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);
            let text = rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n");
            action = result_actions(
                ui,
                title,
                rows,
                format!("## {title}\n\n{}\n{note}", md_table(rows)),
                text,
            );
        });
        action
    }
}
