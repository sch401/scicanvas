use super::*;

#[derive(Clone)]
pub struct LayerContents {
    pub graph: u64,
    pub index: usize,
    pub before: scicanvas_core::page::Layer,
    pub draft: scicanvas_core::page::Layer,
    pub selected: Vec<scicanvas_core::plotspec::DataRef>,
    pub active: Option<String>,
    pub error: Option<String>,
}

impl LayerContents {
    fn add_selected(&mut self, project: &scicanvas_core::Project) {
        let mut draft = self.draft.clone();
        for book in &project.books {
            let keys: Vec<_> = self
                .selected
                .iter()
                .filter(|r| r.book == book.id)
                .map(|r| r.col.clone())
                .collect();
            if keys.is_empty() {
                continue;
            }
            let alive = draft.book_id.is_some_and(|id| project.book(id).is_some());
            if let Err(error) = draft.add_book_curves(book, &keys, alive) {
                self.error = Some(error);
                return;
            }
        }
        self.active = draft
            .y_keys
            .iter()
            .find(|key| !self.draft.y_keys.contains(key))
            .cloned()
            .or_else(|| self.active.clone());
        self.draft = draft;
        self.selected.clear();
        self.error = None;
    }

    fn source_name(project: &scicanvas_core::Project, reference: Option<&scicanvas_core::plotspec::DataRef>) -> String {
        reference
            .map(|r| {
                format!(
                    "{} / {}",
                    project.book(r.book).map_or("已删除的表", |b| b.name.as_str()),
                    r.col
                )
            })
            .unwrap_or_else(|| "行号".into())
    }

    pub(super) fn show(&mut self, ctx: &egui::Context, project: &scicanvas_core::Project, pal: &Palette) -> Action {
        let mut open = true;
        window(ctx, "图层内容")
            .open(&mut open)
            .default_width(360.0)
            .default_height(660.0)
            .show(ctx, |ui| {
                ui.strong(format!(
                    "{} · 图层 {}",
                    project.name_of(self.graph).unwrap_or("图形"),
                    self.index + 1
                ));
                ui.separator();
                ui.strong("可用数据");
                egui::ScrollArea::vertical()
                    .id_salt("contents-data")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        for book in &project.books {
                            egui::CollapsingHeader::new(&book.name)
                                .id_salt(("contents-book", book.id))
                                .default_open(true)
                                .show(ui, |ui| {
                                    for col in &book.columns {
                                        let reference = scicanvas_core::plotspec::DataRef::new(book.id, &col.key);
                                        let present = self.draft.y_keys.iter().any(|key| {
                                            scicanvas_core::plotspec::DataRef::from_key(self.draft.book_id, key)
                                                .as_ref()
                                                == Some(&reference)
                                        });
                                        let mut selected = present || self.selected.contains(&reference);
                                        let name = if col.display_name() == col.key {
                                            format!("{} ({})", col.key, col.role.label())
                                        } else {
                                            format!("{} ({})  {}", col.key, col.role.label(), col.display_name())
                                        };
                                        let response = ui.add_enabled(
                                            matches!(col.role, scicanvas_core::Role::Y | scicanvas_core::Role::Z)
                                                && !present,
                                            egui::Checkbox::new(&mut selected, name),
                                        );
                                        if response.changed() {
                                            self.selected.retain(|r| r != &reference);
                                            if selected {
                                                self.selected.push(reference);
                                            }
                                        }
                                        if present {
                                            response.on_hover_text("已在当前图层");
                                        }
                                    }
                                });
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(!self.selected.is_empty(), plain("添加 →")).clicked() {
                        self.add_selected(project);
                    }
                });
                ui.separator();
                ui.strong("当前图层曲线");
                let keys = self.draft.y_keys.clone();
                egui::ScrollArea::vertical()
                    .id_salt("contents-curves")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for key in &keys {
                            ui.push_id(key, |ui| {
                                let name = self.draft.curve_label(
                                    self.draft.book_id.and_then(|id| project.book(id)),
                                    &|id| project.book(id),
                                    key,
                                );
                                let before = self.draft.curves.get(key).is_some_and(|style| style.right_axis);
                                let mut right_axis = before;
                                ui.horizontal_wrapped(|ui| {
                                    if ui.selectable_label(self.active.as_ref() == Some(key), name).clicked() {
                                        self.active = Some(key.clone());
                                    }
                                    egui::ComboBox::from_id_salt("contents-axis")
                                        .width(72.0)
                                        .selected_text(if right_axis { "右 Y" } else { "左 Y" })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut right_axis, false, "左 Y");
                                            ui.selectable_value(&mut right_axis, true, "右 Y");
                                        });
                                });
                                if right_axis != before {
                                    let style = self.draft.curves.entry(key.clone()).or_default();
                                    style.right_axis = right_axis;
                                    if style.is_default()
                                        && !self
                                            .before
                                            .curves
                                            .get(key)
                                            .is_some_and(scicanvas_core::page::CurveStyle::is_default)
                                    {
                                        self.draft.curves.remove(key);
                                    }
                                }
                                let plot = self.draft.plot_in(key, &|id| project.book(id));
                                let mapping = plot.as_ref().map(|p| &p.mapping);
                                let source = mapping.map(|m| &m.y);
                                egui::Grid::new("contents-source")
                                    .num_columns(2)
                                    .spacing([8.0, 2.0])
                                    .show(ui, |ui| {
                                        ui.label("X 来源");
                                        ui.label(Self::source_name(
                                            project,
                                            mapping.as_ref().and_then(|m| m.x.as_ref()),
                                        ));
                                        ui.end_row();
                                        ui.label("Y 来源");
                                        ui.label(
                                            source
                                                .as_ref()
                                                .map(|r| Self::source_name(project, Some(r)))
                                                .unwrap_or_else(|| "未绑定".into()),
                                        );
                                        ui.end_row();
                                    });
                                ui.separator();
                            });
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    let at = self
                        .active
                        .as_ref()
                        .and_then(|key| self.draft.y_keys.iter().position(|k| k == key));
                    if ui.add_enabled(at.is_some(), plain("← 移除")).clicked() {
                        if let Some(at) = at {
                            self.draft.remove_curve(&keys[at]);
                            self.active = self
                                .draft
                                .y_keys
                                .get(at.min(self.draft.y_keys.len().saturating_sub(1)))
                                .cloned();
                        }
                    }
                    if ui.add_enabled(at.is_some_and(|i| i > 0), plain("上移")).clicked() {
                        let i = at.unwrap();
                        self.draft.spec.y_keys.swap(i, i - 1);
                    }
                    if ui
                        .add_enabled(at.is_some_and(|i| i + 1 < keys.len()), plain("下移"))
                        .clicked()
                    {
                        let i = at.unwrap();
                        self.draft.spec.y_keys.swap(i, i + 1);
                    }
                    if ui.add(plain("按名称排序")).clicked() {
                        let names: std::collections::BTreeMap<_, _> = keys
                            .iter()
                            .map(|key| {
                                (
                                    key.clone(),
                                    self.draft
                                        .curve_label(
                                            self.draft.book_id.and_then(|id| project.book(id)),
                                            &|id| project.book(id),
                                            key,
                                        )
                                        .to_lowercase(),
                                )
                            })
                            .collect();
                        self.draft
                            .spec
                            .y_keys
                            .sort_by_key(|key| names.get(key).cloned().unwrap_or_default());
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(pal.ink, error);
                }
            });
        if open {
            Action::None
        } else {
            Action::Close
        }
    }
}
