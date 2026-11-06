//! 拖放列映射、实时预览与使用当前数据绘制的基础图库。
use eframe::egui;
use scicanvas_core::{colormap::ColorMap, page::Layer, plotspec::PlotSpec, Graph, PlotType, Project};

pub fn floating_columns(ui: &mut egui::Ui, layer: &mut Layer, cols: &[(String, String)]) {
    if layer.plot_type != PlotType::FloatingBar {
        return;
    }
    ui.vertical(|ui| {
        for key in layer.plot_keys(false).iter() {
            let mut base = layer.plot_base_key(key).unwrap_or_default();
            let before = base.clone();
            egui::ComboBox::from_id_salt(("floating-base", key))
                .selected_text(format!(
                    "{key} 的下端：{}",
                    if base.is_empty() { "请选择" } else { &base }
                ))
                .show_ui(ui, |ui| {
                    for (k, label) in cols.iter().filter(|(k, _)| k != key) {
                        ui.selectable_value(&mut base, k.clone(), label);
                    }
                });
            if base != before {
                layer.set_plot_base_key(key, base);
            }
        }
    });
}

pub fn show(ctx: &egui::Context, project: &Project, draft: &mut Option<PlotSpec>) -> bool {
    let Some(spec) = draft.as_mut() else { return false };
    let Some(book) = spec.book_id.and_then(|id| project.book(id)) else {
        *draft = None;
        return false;
    };
    // 草稿每帧移入图层编辑视图，结束后移回；不复制数据，也不改变应用的草稿类型。
    let mut layer = Layer {
        spec: std::mem::take(spec).into(),
        ..Default::default()
    };
    let mut open = true;
    let mut apply = false;
    crate::dialogs::window(ctx, "作图面板与图库")
        .id(egui::Id::new("plot-builder"))
        .open(&mut open)
        .default_width(360.0)
        .default_height(620.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(format!("数据：{}", book.name));
            egui::CollapsingHeader::new("列映射").default_open(true).show(ui, |ui| {
                column_chips(ui, book);
                let frame = egui::Frame::group(ui.style());
                let (_, dropped) = ui.dnd_drop_zone::<String, _>(frame, |ui| {
                    ui.label("X：拖入一列");
                    egui::ComboBox::from_id_salt("builder-x")
                        .selected_text(
                            layer
                                .x_key
                                .as_ref()
                                .and_then(|k| book.column(k))
                                .map_or("行号".into(), |c| format!("{} · {}", c.key, c.display_name())),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut layer.x_key, None, "行号");
                            for c in &book.columns {
                                ui.selectable_value(
                                    &mut layer.x_key,
                                    Some(c.key.clone()),
                                    format!("{} · {}", c.key, c.display_name()),
                                );
                            }
                        });
                });
                if let Some(key) = dropped.filter(|k| book.column(k).is_some()) {
                    layer.x_key = Some((*key).clone());
                }
                let (_, dropped) = ui.dnd_drop_zone::<String, _>(frame, |ui| {
                    ui.label("Y：拖入一列或多选");
                    let keys = layer.plot_keys(true);
                    for col in &book.columns {
                        let mut selected = keys.contains(&col.key);
                        if ui
                            .checkbox(&mut selected, format!("{} · {}", col.key, col.display_name()))
                            .changed()
                        {
                            if selected {
                                layer.edit_plot_keys(|keys| keys.push(col.key.clone()));
                            } else {
                                layer.edit_plot_keys(|keys| keys.retain(|k| k != &col.key));
                            }
                        }
                    }
                });
                if let Some(key) = dropped.filter(|k| book.column(k).is_some()) {
                    if !layer.plot_keys(true).contains(&key) {
                        layer.edit_plot_keys(|keys| keys.push((*key).clone()));
                    }
                }
                let cols: Vec<_> = book
                    .columns
                    .iter()
                    .filter(|c| !matches!(c.role, scicanvas_core::Role::XError | scicanvas_core::Role::YError))
                    .map(|c| (c.key.clone(), format!("{} · {}", c.key, c.display_name())))
                    .collect();
                if layer.plot_type == PlotType::FloatingBar {
                    ui.label("浮动柱下端");
                    floating_columns(ui, &mut layer, &cols);
                }
                ui.label("配色");
                palette_choices(ui, &mut layer);
            });
            let valid = layer.validate(book).and_then(|_| {
                if layer.series(book).1.iter().any(|s| !s.points.is_empty()) {
                    Ok(())
                } else {
                    Err("所选列没有有效数值点".into())
                }
            });
            if let Err(e) = &valid {
                ui.colored_label(crate::theme::palette_for(ui.ctx()).error, e);
            }
            if ui
                .add_enabled(valid.is_ok(), crate::dialogs::primary(ui, "创建图形"))
                .clicked()
            {
                apply = true;
            }

            egui::CollapsingHeader::new("图库 · 当前数据")
                .default_open(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        egui::Grid::new("builder-gallery").num_columns(1).show(ui, |ui| {
                            for (i, preset) in scicanvas_core::presets::all().into_iter().enumerate() {
                                ui.vertical(|ui| {
                                    ui.label(preset.name);
                                    let mut g = Graph::new(i as u64, "预览");
                                    g.first_layer_mut().spec = layer.spec.clone();
                                    let preview_valid = select_plot_type(g.first_layer_mut(), preset.kind)
                                        .and_then(|_| g.first_layer().validate(book));
                                    let selected = layer.plot_type == preset.kind;
                                    let response = ui
                                        .add_enabled_ui(preview_valid.is_ok(), |ui| {
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(ui.available_width().min(300.0), 170.0),
                                                egui::Sense::click(),
                                            );
                                            if ui.is_rect_visible(rect) {
                                                let sc = scicanvas_core::scene::layout_page(&g, project, &[]);
                                                crate::plot::paint(ui, rect, &sc);
                                            }
                                            if selected {
                                                ui.painter().rect_stroke(
                                                    rect,
                                                    0.0,
                                                    egui::Stroke::new(2.0_f32, ui.visuals().selection.bg_fill),
                                                );
                                            }
                                            response
                                        })
                                        .inner;
                                    if response
                                        .on_hover_text(
                                            preview_valid.as_ref().err().map_or("点击选择此图型", String::as_str),
                                        )
                                        .clicked()
                                    {
                                        layer.spec = g.first_layer().spec.clone();
                                    }
                                });
                                ui.end_row();
                            }
                        });
                    });
                });
        });
    *draft = open.then(|| selection_draft(&layer));
    apply
}

fn select_plot_type(layer: &mut Layer, kind: PlotType) -> Result<(), String> {
    let previous = layer.plot_type;
    layer.set_plot_type(kind)?;
    if previous != kind
        && matches!(kind, PlotType::ColorScatter | PlotType::BubbleColor)
        && layer.color_map == ColorMap::Classic
    {
        layer.color_map = ColorMap::Viridis;
    }
    Ok(())
}

fn selection_draft(layer: &Layer) -> PlotSpec {
    let mut draft = layer.draft();
    if matches!(
        layer.plot_type,
        PlotType::Bubble | PlotType::ColorScatter | PlotType::BubbleColor
    ) {
        for style in layer.styles().values() {
            for key in [&style.size_column, &style.color_column].into_iter().flatten() {
                if !draft.y_keys.contains(key) {
                    draft.y_keys.push(key.clone());
                }
            }
        }
    }
    draft
}

fn column_chips(ui: &mut egui::Ui, book: &scicanvas_core::Book) {
    ui.horizontal_wrapped(|ui| {
        for col in &book.columns {
            ui.dnd_drag_source(
                egui::Id::new(("plot-column", book.id, &col.key)),
                col.key.clone(),
                |ui| {
                    ui.label(format!("{} · {}", col.key, col.display_name()));
                },
            );
        }
    });
}

fn palette_choices(ui: &mut egui::Ui, layer: &mut Layer) {
    ui.horizontal_wrapped(|ui| {
        for color_map in [ColorMap::Classic, ColorMap::Viridis, ColorMap::Colorblind] {
            ui.selectable_value(&mut layer.color_map, color_map, color_map.label());
        }
    });
}
