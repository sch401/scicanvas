//! 拖放列映射、实时预览与使用当前数据绘制的基础图库。
use eframe::egui;
use scicanvas_core::{colormap::ColorMap, plotspec::PlotSpec, Graph, Project};

pub fn show(ctx: &egui::Context, project: &Project, draft: &mut Option<PlotSpec>) -> bool {
    let Some(spec) = draft.as_mut() else { return false };
    let Some(book) = spec.book_id.and_then(|id| project.book(id)) else {
        *draft = None;
        return false;
    };
    let mut open = true;
    let mut apply = false;
    egui::Window::new("作图面板与图库")
        .id(egui::Id::new("plot-builder"))
        .open(&mut open)
        .default_width(880.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(format!(
                "数据：{} · 拖动列名到 X / Y，也可使用下拉框和勾选框",
                book.name
            ));
            ui.columns(2, |columns| {
                columns[0].heading("列映射");
                columns[0].horizontal_wrapped(|ui| {
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
                let frame = egui::Frame::group(columns[0].style());
                let (_, dropped) = columns[0].dnd_drop_zone::<String, _>(frame, |ui| {
                    ui.label("X：拖入一列");
                    egui::ComboBox::from_id_salt("builder-x")
                        .selected_text(spec.x_key.as_deref().unwrap_or("行号"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut spec.x_key, None, "行号");
                            for c in &book.columns {
                                ui.selectable_value(&mut spec.x_key, Some(c.key.clone()), c.key.as_str());
                            }
                        });
                });
                if let Some(key) = dropped.filter(|k| book.column(k).is_some()) {
                    spec.x_key = Some((*key).clone());
                }
                let (_, dropped) = columns[0].dnd_drop_zone::<String, _>(frame, |ui| {
                    ui.label("Y：拖入一列或多选");
                    for col in &book.columns {
                        let mut selected = spec.y_keys.contains(&col.key);
                        if ui
                            .checkbox(&mut selected, format!("{} · {}", col.key, col.display_name()))
                            .changed()
                        {
                            if selected {
                                spec.y_keys.push(col.key.clone());
                            } else {
                                spec.y_keys.retain(|k| k != &col.key);
                            }
                        }
                    }
                });
                if let Some(key) = dropped.filter(|k| book.column(k).is_some()) {
                    if !spec.y_keys.contains(&key) {
                        spec.y_keys.push((*key).clone());
                    }
                }
                columns[0].label("配色");
                columns[0].horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut spec.color_map, ColorMap::Origin, "Origin");
                    ui.selectable_value(&mut spec.color_map, ColorMap::Viridis, "Viridis");
                    ui.selectable_value(&mut spec.color_map, ColorMap::Colorblind, "色盲友好");
                });
                let valid = spec.validate(book).and_then(|_| {
                    if spec.series(book).1.iter().any(|s| !s.points.is_empty()) {
                        Ok(())
                    } else {
                        Err("所选列没有有效数值点".into())
                    }
                });
                if let Err(e) = &valid {
                    columns[0].colored_label(egui::Color32::from_rgb(180, 65, 45), e);
                }
                if columns[0]
                    .add_enabled(valid.is_ok(), egui::Button::new("创建图形"))
                    .clicked()
                {
                    apply = true;
                }

                columns[1].heading("图库 · 当前数据");
                egui::Grid::new("builder-gallery")
                    .num_columns(2)
                    .show(&mut columns[1], |ui| {
                        for (i, preset) in scicanvas_core::presets::all().into_iter().enumerate() {
                            ui.vertical(|ui| {
                                ui.label(preset.name);
                                let mut g = Graph::new(i as u64, "预览");
                                g.spec = spec.clone();
                                g.plot_type = preset.kind;
                                let selected = spec.plot_type == preset.kind;
                                let response = ui
                                    .add_enabled_ui(valid.is_ok(), |ui| {
                                        let (rect, response) =
                                            ui.allocate_exact_size(egui::vec2(180.0, 140.0), egui::Sense::click());
                                        let sc = scicanvas_core::scene::layout_page(&g, project, &[]);
                                        crate::plot::paint(ui, rect, &sc);
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
                                    .on_hover_text(valid.as_ref().err().map_or("点击选择此图型", String::as_str))
                                    .clicked()
                                {
                                    spec.plot_type = preset.kind;
                                }
                            });
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
            });
        });
    if !open {
        *draft = None;
    }
    apply
}
