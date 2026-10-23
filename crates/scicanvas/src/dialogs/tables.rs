use super::*;

/// Markdown 表格单元格：竖线转义，换行改成空格。
pub(super) fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace(['\r', '\n'], " ")
}

/// Markdown 表格（首行为表头），末尾带换行；空表返回空串。
pub(crate) fn md_table(rows: &[Vec<String>]) -> String {
    let Some(head) = rows.first() else { return String::new() };
    let line = |r: &Vec<String>| {
        let cells: Vec<String> = (0..head.len())
            .map(|i| md_cell(r.get(i).map_or("", String::as_str)))
            .collect();
        format!("| {} |\n", cells.join(" | "))
    };
    let mut out = line(head);
    out.push_str(&format!("|{}\n", " --- |".repeat(head.len())));
    for r in &rows[1..] {
        out.push_str(&line(r));
    }
    out
}

/// 两列“名称 | 值”的 Markdown 表格。
pub(super) fn md_pairs(head: [&str; 2], rows: &[(String, String)]) -> String {
    let mut table = vec![head.map(String::from).to_vec()];
    table.extend(rows.iter().map(|(k, v)| vec![k.clone(), v.clone()]));
    md_table(&table)
}

/// 结果表格：首行为表头（淡色），首列为名称，其余等宽数字。
pub(super) fn table_grid(ui: &mut egui::Ui, id: impl std::hash::Hash, rows: &[Vec<String>], pal: &Palette) {
    egui::Grid::new(id)
        .striped(true)
        .num_columns(rows.first().map_or(1, Vec::len))
        .spacing([20.0, 4.0])
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
}
