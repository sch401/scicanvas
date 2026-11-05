use super::*;

fn page_position_editor(ui: &mut egui::Ui, position: &mut [f64; 2], group: &mut Option<egui::Id>) {
    ui.horizontal(|ui| {
        for (index, label) in ["X ", "Y "].into_iter().enumerate() {
            let mut percent = position[index] * 100.0;
            let r = ui.add(
                egui::DragValue::new(&mut percent)
                    .range(0.0..=100.0)
                    .speed(0.1)
                    .prefix(label)
                    .suffix("%"),
            );
            if r.changed() {
                position[index] = percent / 100.0;
            }
            edit_group(&r, group);
        }
    });
}

#[derive(Clone, Copy)]
pub(crate) struct PlotObjectEditor {
    pub graph: u64,
    pub target: crate::plot::ObjectTarget,
    pub focus: bool,
}
impl PlotObjectEditor {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        g: &mut scicanvas_core::Graph,
        history_group: &mut Option<egui::Id>,
    ) -> (bool, Option<crate::plot::ObjectAction>) {
        use crate::plot::{self, ObjectTarget as T};
        let edit = self;
        let title = object_title(edit.target);
        let mut open = true;
        let mut action = None;
        window(ctx, title).centered().open(&mut open).show(ctx, |ui| {
            ui.push_id((edit.graph, edit.target), |ui| {
                ui.label(&g.name);
                match edit.target {
                    T::Shape(id) => shape_properties(ui, g.page.shape_mut(id).unwrap(), history_group),
                    T::Text(id) => {
                        egui::ComboBox::from_id_salt("text-object")
                            .selected_text(g.page.annotations.iter().position(|t| t.id == id).map_or(
                                String::new(),
                                |i| {
                                    format!(
                                        "文字 {}/{} · {}",
                                        i + 1,
                                        g.page.annotations.len(),
                                        scicanvas_core::text::plain_text(
                                            &g.page.annotations[i].text,
                                            g.page.annotations[i].rich
                                        )
                                        .chars()
                                        .take(24)
                                        .collect::<String>()
                                    )
                                },
                            ))
                            .show_ui(ui, |ui| {
                                for (position, text) in g.page.annotations.iter().enumerate() {
                                    let name: String = scicanvas_core::text::plain_text(&text.text, text.rich)
                                        .chars()
                                        .take(24)
                                        .collect();
                                    if ui
                                        .selectable_label(
                                            text.id == id,
                                            format!("文字 {}/{} · {}", position + 1, g.page.annotations.len(), name),
                                        )
                                        .clicked()
                                    {
                                        edit.target = T::Text(text.id);
                                        edit.focus = true;
                                    }
                                }
                            });
                        let text = g.page.text_mut(id).unwrap();
                        form(ui, "text-object-properties", |ui| {
                            ui.label("内容");
                            let r = scientific_text_multiline(ui, &mut text.text, &mut text.rich);
                            if edit.focus && edit.target == T::Text(id) {
                                r.request_focus();
                                if let Some(mut state) = egui::TextEdit::load_state(ctx, r.id) {
                                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(0),
                                        egui::text::CCursor::new(text.text.chars().count()),
                                    )));
                                    state.store(ctx, r.id);
                                }
                                edit.focus = false;
                            }
                            edit_group(&r, history_group);
                            ui.end_row();
                            ui.label("科学文字");
                            ui.checkbox(&mut text.rich, "启用");
                            ui.end_row();
                            ui.label("字号");
                            let r = ui.add(
                                egui::DragValue::new(&mut text.size_pt)
                                    .range(1.0..=256.0)
                                    .speed(0.25)
                                    .suffix(" pt"),
                            );
                            edit_group(&r, history_group);
                            ui.end_row();
                            ui.label("颜色");
                            edit_group(&ui.color_edit_button_srgb(&mut text.color), history_group);
                            ui.end_row();
                            ui.label("页面位置");
                            page_position_editor(ui, &mut text.position, history_group);
                            ui.end_row();
                        });
                    }
                    T::ColorScale(index) => {
                        let color = g.page.layers_mut()[index]
                            .color_scale
                            .get_or_insert_with(Default::default);
                        ui.checkbox(&mut color.visible, "显示色阶条");
                        if let Some(position) = &mut color.position {
                            page_position_editor(ui, position, history_group);
                        }
                        if ui.button("恢复自动位置").clicked() {
                            color.position = None;
                        }
                    }
                    T::Legend(index) => {
                        ui.label(format!("图层 {}", index + 1));
                        let layer = &mut g.page.layers_mut()[index];
                        if let Some(position) = &mut layer.legend_position {
                            page_position_editor(ui, position, history_group);
                        }
                        if ui.button("恢复自动位置").clicked() {
                            action = Some(plot::ObjectAction::AutoLegend(index));
                        }
                        ui.checkbox(&mut layer.rich_text, "启用科学文字格式");
                        for key in layer.plot_keys(true) {
                            let name = ctx
                                .data(|d| {
                                    d.get_temp::<std::collections::HashMap<(usize, String), String>>(egui::Id::new(
                                        "object-curve-names",
                                    ))
                                })
                                .and_then(|names| names.get(&(index, key.clone())).cloned())
                                .unwrap_or_else(|| key.clone());
                            ui.label(name);
                            let mut style = layer.plot_style(&key).cloned().unwrap_or_default();
                            let r =
                                scientific_text(ui, &mut style.legend_label, &mut layer.rich_text, "自动：数据列名称");
                            if edit.focus {
                                r.request_focus();
                                edit.focus = false;
                            }
                            edit_group(&r, history_group);
                            layer.set_plot_style(&key, (!style.is_default()).then_some(style));
                        }
                    }
                }
                live_hint(ui);
                ui.separator();
                ui.horizontal(|ui| {
                    let label = match edit.target {
                        T::Legend(_) => "隐藏图例",
                        T::ColorScale(_) => "隐藏色阶条",
                        _ => "删除",
                    };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(label).color(crate::theme::palette_for(ctx).on_accent))
                                .fill(crate::theme::palette_for(ctx).danger),
                        )
                        .clicked()
                    {
                        action = Some(plot::ObjectAction::Delete(edit.target));
                    }
                });
            });
        });
        (open, action)
    }
}

fn shape_properties(ui: &mut egui::Ui, shape: &mut scicanvas_core::page::ShapeObject, group: &mut Option<egui::Id>) {
    use scicanvas_core::page::ShapeKind;
    let mut candidate = *shape;
    form(ui, "shape-properties", |ui| {
        ui.label("类型");
        egui::ComboBox::from_id_salt("shape-kind")
            .selected_text(candidate.kind.label())
            .show_ui(ui, |ui| {
                for kind in ShapeKind::ALL {
                    ui.selectable_value(&mut candidate.kind, kind, kind.label());
                }
            });
        ui.end_row();
        ui.label("端点/角点 A");
        page_position_editor(ui, &mut candidate.a, group);
        ui.end_row();
        ui.label("端点/角点 B");
        page_position_editor(ui, &mut candidate.b, group);
        ui.end_row();
        ui.label("线条颜色");
        edit_group(&ui.color_edit_button_srgb(&mut candidate.stroke), group);
        ui.end_row();
        ui.label("线宽");
        edit_group(
            &ui.add(
                egui::DragValue::new(&mut candidate.width_pt)
                    .range(0.0..=32.)
                    .speed(0.1)
                    .suffix(" pt"),
            ),
            group,
        );
        ui.end_row();
        ui.label("线型");
        ui.checkbox(&mut candidate.dashed, "虚线");
        ui.end_row();
        if candidate.kind == ShapeKind::Arrow {
            ui.label("箭头大小");
            edit_group(
                &ui.add(
                    egui::DragValue::new(&mut candidate.head_pt)
                        .range(0.0..=64.)
                        .speed(0.5)
                        .suffix(" pt"),
                ),
                group,
            );
            ui.end_row();
        }
        if matches!(candidate.kind, ShapeKind::Rect | ShapeKind::Ellipse) {
            let mut enabled = candidate.fill.is_some();
            let (mut color, mut alpha) = candidate.fill.unwrap_or((candidate.stroke, 0.25));
            ui.label("填充");
            ui.checkbox(&mut enabled, "启用");
            ui.end_row();
            if enabled {
                ui.label("填充颜色");
                edit_group(&ui.color_edit_button_srgb(&mut color), group);
                ui.end_row();
                ui.label("不透明度");
                edit_group(
                    &ui.add(egui::DragValue::new(&mut alpha).range(0.0..=1.).speed(0.01)),
                    group,
                );
                ui.end_row();
            }
            candidate.fill = enabled.then_some((color, alpha));
        }
    });
    match candidate.validate() {
        Ok(()) => *shape = candidate,
        Err(error) => {
            field_error(ui, &error);
        }
    }
}

fn object_title(target: crate::plot::ObjectTarget) -> &'static str {
    match target {
        crate::plot::ObjectTarget::Shape(_) => "编辑绘图对象",
        crate::plot::ObjectTarget::Text(_) => "编辑文字",
        crate::plot::ObjectTarget::ColorScale(_) => "编辑色阶条",
        crate::plot::ObjectTarget::Legend(_) => "编辑图例",
    }
}
