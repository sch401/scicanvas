use super::*;

/// Shared title/legend editor. Selection indices are Unicode character positions, as in egui.
pub(crate) fn scientific_text(ui: &mut egui::Ui, text: &mut String, rich: &mut bool, hint: &str) -> egui::Response {
    scientific_text_editor(ui, text, rich, hint, false)
}
pub(crate) fn scientific_text_multiline(ui: &mut egui::Ui, text: &mut String, rich: &mut bool) -> egui::Response {
    scientific_text_editor(ui, text, rich, "", true)
}
fn scientific_text_editor(
    ui: &mut egui::Ui,
    text: &mut String,
    rich: &mut bool,
    hint: &str,
    multiline: bool,
) -> egui::Response {
    ui.vertical(|ui| {
        let widget = if multiline {
            egui::TextEdit::multiline(text).desired_rows(3)
        } else {
            egui::TextEdit::singleline(text)
        };
        let mut edit = widget.hint_text(hint).desired_width(f32::INFINITY).show(ui);
        let mut action = None;
        ui.horizontal(|ui| {
            for (label, tip, command) in [
                ("B", "粗体", "\\b{"),
                ("I", "斜体", "\\i{"),
                ("x²", "上标", "^{"),
                ("x₂", "下标", "_{"),
            ] {
                let label = match command {
                    "\\b{" => RichText::new(label).strong(),
                    "\\i{" => RichText::new(label).italics(),
                    _ => RichText::new(label),
                };
                if ui.small_button(label).on_hover_text(tip).clicked() {
                    action = Some((command, true));
                }
            }
            egui::ComboBox::from_id_salt(edit.response.id.with("greek"))
                .width(36.0)
                .selected_text("Ω")
                .show_ui(ui, |ui| {
                    egui::Grid::new(edit.response.id.with("symbols"))
                        .num_columns(8)
                        .show(ui, |ui| {
                            for (i, (name, symbol)) in scicanvas_core::text::GREEK.iter().enumerate() {
                                if ui.selectable_label(false, *symbol).on_hover_text(*name).clicked() {
                                    action = Some((*symbol, false));
                                    ui.close_menu();
                                }
                                if i % 8 == 7 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            if ui.small_button("清除").on_hover_text("清除所选文字的格式").clicked() {
                action = Some(("", true));
            }
        });
        if let Some((command, format)) = action {
            let range = edit
                .state
                .cursor
                .char_range()
                .map(|r| r.sorted())
                .unwrap_or([egui::text::CCursor::new(text.chars().count()); 2]);
            let (selection, caret) = format_selection(text, range[0].index..range[1].index, command, format);
            if format && !command.is_empty() {
                *rich = true;
            }
            edit.state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(selection),
                egui::text::CCursor::new(caret),
            )));
            edit.state.store(ui.ctx(), edit.response.id);
            edit.response.request_focus();
            edit.response.mark_changed();
        }
        edit.response
    })
    .inner
}

pub(super) fn format_selection(
    text: &mut String,
    range: std::ops::Range<usize>,
    command: &str,
    format: bool,
) -> (usize, usize) {
    let byte = |i| text.char_indices().nth(i).map_or(text.len(), |(n, _)| n);
    let (mut start, mut end) = (byte(range.start), byte(range.end));
    let selected = text[start..end].to_string();
    let mut first = range.start;
    let replacement = if format && command.is_empty() {
        // Strip enclosing formats too when a toolbar-created inner selection is still active.
        while let Some(prefix) = ["\\b{", "\\i{", "^{", "_{"]
            .into_iter()
            .find(|p| text[..start].ends_with(p) && text[end..].starts_with('}'))
        {
            start -= prefix.len();
            end += 1;
            first -= prefix.chars().count();
        }
        scicanvas_core::text::plain_text(&selected, true)
    } else if format && text[..start].ends_with(command) && text[end..].starts_with('}') {
        start -= command.len();
        end += 1;
        first -= command.chars().count();
        selected.clone()
    } else if format {
        first += command.chars().count();
        format!("{command}{selected}}}")
    } else {
        command.to_string()
    };
    let last = if format {
        first
            + scicanvas_core::text::plain_text(&selected, command.is_empty())
                .chars()
                .count()
    } else {
        first + replacement.chars().count()
    };
    text.replace_range(start..end, &replacement);
    (if format { first } else { last }, last)
}
