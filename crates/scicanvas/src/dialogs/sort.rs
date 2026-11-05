use super::*;

impl Dialog {
    pub(super) fn show_sort(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::Sort {
            book,
            columns,
            keys,
            selected,
            only_selected,
            error,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "排序").centered().open(open).show(ctx, |ui| {
            form(ui, "sort-grid", |ui| {
                for (level, (col, asc)) in keys.iter_mut().enumerate() {
                    ui.label(["主关键字", "次关键字", "第三关键字"][level]);
                    let text = col.map_or("无".to_string(), |i| columns[i].1.clone());
                    egui::ComboBox::from_id_salt(("sort-key", level))
                        .selected_text(text)
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            if level > 0 {
                                ui.selectable_value(col, None, "无");
                            }
                            for (i, (_, label)) in columns.iter().enumerate() {
                                ui.selectable_value(col, Some(i), label);
                            }
                        });
                    ui.end_row();
                    ui.label("方向");
                    ui.horizontal(|ui| {
                        ui.radio_value(asc, true, "升序");
                        ui.radio_value(asc, false, "降序");
                    });
                    ui.end_row();
                }
            });
            if !selected.is_empty() {
                ui.checkbox(
                    only_selected,
                    format!("只排选中列（{}），其余列不动", selected.join("、")),
                );
            }
            if let Some(error) = error {
                field_error(ui, error.as_str());
            }
            let selected_keys: Vec<_> = keys.iter().filter_map(|(key, _)| *key).collect();
            let valid: Result<(), String> = if selected_keys
                .iter()
                .enumerate()
                .any(|(i, k)| selected_keys[..i].contains(k))
            {
                Err("排序关键字不能重复".into())
            } else {
                Ok(())
            };
            if let Err(error) = &valid {
                field_error(ui, error);
            }
            let (ok, cancel) = buttons(ui, "排序", valid);
            if ok {
                let keys: Vec<(String, bool)> = keys
                    .iter()
                    .filter_map(|(c, asc)| c.map(|i| (columns[i].0.clone(), *asc)))
                    .collect();
                let within = (*only_selected && !selected.is_empty()).then(|| selected.clone());
                action = Action::Sort(*book, keys, within);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
