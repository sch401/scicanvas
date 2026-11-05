use super::*;
pub(super) fn text_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter(|s| !s.is_empty())
        .map(|s| s.split('\t').map(str::to_owned).collect())
        .collect()
}
pub(super) fn result_actions(
    ui: &mut egui::Ui,
    title: &str,
    rows: &[Vec<String>],
    markdown: String,
    text: String,
) -> Action {
    let mut action = Action::None;
    ui.add_space(crate::theme::space::S);
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        center_buttons(ui, 4);
        if ui.add(plain("复制结果")).clicked() {
            action = Action::CopyResult(text, rows.len());
        }
        if ui.add(plain("写入新表")).clicked() {
            let mut table = rows.to_vec();
            let width = table.iter().map(Vec::len).max().unwrap_or(0);
            for row in &mut table {
                row.resize(width, String::new());
            }
            action = Action::Statistics(Box::new(StatisticsAction::WriteTable(title.into(), table)));
        }
        if ui.add(plain("保存为 Note")).clicked() {
            action = Action::SaveNote(title.into(), markdown);
        }
        if ui.add(primary(ui, "关闭")).clicked() {
            action = Action::Close;
        }
    });
    action
}
