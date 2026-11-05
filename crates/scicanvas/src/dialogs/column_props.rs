use super::*;

/// 列属性对话框里编辑的内容。
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnEdit {
    pub key: String,
    pub long_name: String,
    pub units: String,
    pub comment: String,
    pub role: scicanvas_core::Role,
    pub kind: scicanvas_core::DataKind,
    pub format: scicanvas_core::format::DisplayFormat,
    /// 分析结果列的重算方式（普通列为 None）。
    pub recalc: Option<scicanvas_core::ops::RecalcMode>,
}

impl From<&scicanvas_core::Column> for ColumnEdit {
    fn from(col: &scicanvas_core::Column) -> Self {
        Self {
            key: col.key.clone(),
            long_name: col.long_name.clone(),
            units: col.units.clone(),
            comment: col.comment.clone(),
            role: col.role,
            kind: col.data.kind(),
            format: col.format,
            recalc: col.op.as_ref().map(|op| op.mode),
        }
    }
}

impl ColumnEdit {
    pub fn apply(&self, book: &mut scicanvas_core::Book) -> Result<(), String> {
        let col = book.column(&self.key).ok_or("找不到列")?;
        if col.data.kind() != self.kind {
            book.convert_columns(std::slice::from_ref(&self.key), self.kind)?;
        }
        if book.column(&self.key).is_some_and(|c| c.role != self.role) {
            book.set_roles(std::slice::from_ref(&self.key), self.role);
        }
        let col = book.columns.iter_mut().find(|c| c.key == self.key).unwrap();
        col.long_name.clone_from(&self.long_name);
        col.units.clone_from(&self.units);
        col.comment.clone_from(&self.comment);
        if let (Some(op), Some(mode)) = (col.op.as_mut(), self.recalc) {
            op.mode = mode;
        }
        let offset_changed = col.format.utc_offset != self.format.utc_offset;
        col.format = self.format;
        if offset_changed && col.data.kind() == scicanvas_core::DataKind::DateTime {
            col.set_utc_offset(self.format.utc_offset);
        }
        Ok(())
    }
}

impl Dialog {
    pub(super) fn show_column_props(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
    ) -> Action {
        let Self::ColumnProps {
            book: _,
            edit,
            sample,
            error,
            ..
        } = self
        else {
            unreachable!()
        };
        use scicanvas_core::format::{DateFormat, NumberFormat};
        use scicanvas_core::{DataKind, Role};
        let title = format!("列属性 — {}", edit.key);
        window(ctx, &title).open(open).show(ctx, |ui| {
            egui::Grid::new("column-props")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("短名");
                    ui.label(RichText::new(edit.key.as_str()).strong())
                        .on_hover_text("图形通过短名引用列，创建后不再改变");
                    ui.end_row();
                    for (label, field) in [
                        ("长名称", &mut edit.long_name),
                        ("单位", &mut edit.units),
                        ("注释", &mut edit.comment),
                    ] {
                        ui.label(label);
                        ui.add(egui::TextEdit::singleline(field).desired_width(200.0));
                        ui.end_row();
                    }
                    ui.label("列属性");
                    let roles = [
                        (Role::X, "X"),
                        (Role::Y, "Y"),
                        (Role::XError, "X 误差"),
                        (Role::YError, "Y 误差"),
                        (Role::Z, "Z"),
                        (Role::Label, "标签"),
                        (Role::Ignore, "忽略"),
                    ];
                    let current = roles.iter().find(|(r, _)| *r == edit.role).map_or("Y", |(_, n)| *n);
                    egui::ComboBox::from_id_salt("column-role")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for (role, name) in roles {
                                ui.selectable_value(&mut edit.role, role, name);
                            }
                        });
                    ui.end_row();
                    ui.label("数据类型");
                    egui::ComboBox::from_id_salt("column-kind")
                        .selected_text(edit.kind.label())
                        .show_ui(ui, |ui| {
                            for kind in DataKind::ALL {
                                ui.selectable_value(&mut edit.kind, kind, kind.label());
                            }
                        });
                    ui.end_row();
                    if let Some(mode) = &mut edit.recalc {
                        ui.label("重算方式");
                        ui.horizontal_wrapped(|ui| {
                            for m in scicanvas_core::ops::RecalcMode::ALL {
                                ui.radio_value(mode, m, m.label());
                            }
                        });
                        ui.end_row();
                    }
                    ui.label("显示格式");
                    match edit.kind {
                        DataKind::Num => {
                            ui.horizontal_wrapped(|ui| {
                                let mut index = edit.format.number.index();
                                let mut places = edit.format.number.places();
                                egui::ComboBox::from_id_salt("column-number-format")
                                    .selected_text(NumberFormat::KINDS[index])
                                    .show_ui(ui, |ui| {
                                        for (i, name) in NumberFormat::KINDS.iter().enumerate() {
                                            ui.selectable_value(&mut index, i, *name);
                                        }
                                    });
                                ui.add_enabled_ui(index != 0, |ui| {
                                    ui.label("小数位");
                                    ui.add(
                                        egui::DragValue::new(&mut places)
                                            .range(0..=scicanvas_core::format::MAX_PLACES)
                                            .speed(0.1),
                                    );
                                });
                                edit.format.number = NumberFormat::from_index(index, places);
                            });
                        }
                        DataKind::DateTime => {
                            ui.horizontal_wrapped(|ui| {
                                egui::ComboBox::from_id_salt("column-date-format")
                                    .selected_text(edit.format.date.label())
                                    .show_ui(ui, |ui| {
                                        for f in DateFormat::ALL {
                                            ui.selectable_value(&mut edit.format.date, f, f.label());
                                        }
                                    });
                                use scicanvas_core::format::{utc_offset_label, utc_offsets};
                                egui::ComboBox::from_id_salt("column-utc-offset")
                                    .selected_text(utc_offset_label(edit.format.utc_offset))
                                    .show_ui(ui, |ui| {
                                        for m in utc_offsets() {
                                            ui.selectable_value(&mut edit.format.utc_offset, m, utc_offset_label(m));
                                        }
                                    });
                            });
                        }
                        _ => {
                            ui.label(RichText::new("文本和分类列按原样显示").color(pal.weak));
                        }
                    }
                    ui.end_row();
                    let preview = match (edit.kind, sample) {
                        (DataKind::Num, Some(v)) => Some(edit.format.number.apply(*v).unwrap_or_else(|| fmt_num(*v))),
                        (DataKind::DateTime, Some(ms)) => edit.format.date.apply_in(*ms as i64, edit.format.utc_offset),
                        _ => None,
                    };
                    if let Some(text) = preview {
                        ui.label("第 1 行显示为");
                        ui.label(RichText::new(text).monospace());
                        ui.end_row();
                    }
                });
            if let Some(e) = error {
                ui.add_space(4.0);
                ui.label(RichText::new(e.as_str()).color(ui.visuals().error_fg_color));
            }
            ui.label(RichText::new("只改变显示，不改变存储的数值。").small().color(pal.weak));
        });
        Action::None
    }
}
