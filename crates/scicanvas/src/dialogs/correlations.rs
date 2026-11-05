use super::*;

impl Dialog {
    pub(super) fn show_correlations(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::Correlations(pairs) = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "相关系数").centered().open(open).show(ctx, |ui| {
            egui::ScrollArea::both().max_height(420.0).show(ui, |ui| {
                egui::Grid::new("corr-grid")
                    .striped(true)
                    .num_columns(9)
                    .spacing([14.0, 4.0])
                    .show(ui, |ui| {
                        for h in correlation_header() {
                            ui.label(RichText::new(h).color(pal.weak));
                        }
                        ui.end_row();
                        for row in correlation_rows(pairs) {
                            for (i, cell) in row.iter().enumerate() {
                                if i < 2 {
                                    ui.label(cell);
                                } else {
                                    ui.label(RichText::new(cell).monospace());
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
            ui.label(
                RichText::new("p 值为双侧，只用两列都有数值的行。")
                    .small()
                    .color(pal.weak),
            );
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);
            let mut rows = vec![correlation_header().map(String::from).to_vec()];
            rows.extend(correlation_rows(pairs));
            let text = rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n");
            action = result_actions(ui, "相关系数结果", &rows, correlation_markdown(&rows), text);
        });
        action
    }
}

fn correlation_header() -> [&'static str; 9] {
    [
        "列 A",
        "列 B",
        "N",
        "Pearson r",
        "p",
        "Spearman ρ",
        "p",
        "Kendall τ-b",
        "p",
    ]
}

fn correlation_rows(pairs: &[(String, String, Vec<Correlation>)]) -> Vec<Vec<String>> {
    pairs
        .iter()
        .map(|(a, b, cs)| {
            let mut row = vec![
                a.clone(),
                b.clone(),
                cs.first().map_or(String::new(), |c| c.n.to_string()),
            ];
            for c in cs {
                row.push(fmt_num(c.r));
                row.push(fmt_num(c.p));
            }
            row
        })
        .collect()
}

fn correlation_markdown(rows: &[Vec<String>]) -> String {
    format!(
        "## 相关系数\n\n{}\n\n{}",
        md_table(rows),
        "p 值为双侧。Pearson 和 Spearman 用 t 分布，Kendall τ-b 用含并列校正的正态近似；只使用成对有效行。"
    )
}
