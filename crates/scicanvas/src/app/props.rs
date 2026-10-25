//! 绘图属性窗口：保持各分区的绘制顺序与实时编辑事务。

use super::*;
use scicanvas_core::plotspec::{default_x, BookLookup, DataRef};

impl SciApp {
    pub(super) fn props_window(&mut self, ctx: &egui::Context) {
        let Some(gid) = self.props_for else { return };
        let Some(g0) = self.project.graph(gid) else {
            self.props_for = None;
            return;
        };
        let graph_before = g0.clone();
        let options = source_options(&self.project);
        let mut props_layer = self.layer_index(gid);
        let title = format!("绘图属性 — {}", g0.name);
        let history_group = &mut self.history_group;
        let project = &mut self.project;

        let mut open = true;
        dialogs::window(ctx, &title)
            .id(egui::Id::new("scicanvas-props"))
            .open(&mut open)
            .default_width(360.0)
            .default_height(ctx.screen_rect().height() * 0.75)
            .show(ctx, |ui| {
                let books = &project.books;
                let lookup = |id| books.iter().find(|book| book.id == id);
                let Some(graph) = project.graphs.iter_mut().find(|graph| graph.id == gid) else {
                    return;
                };
                page(ui, graph, history_group);
                props_layer = page_layout(ui, graph, props_layer, history_group);
                layer(ui, graph, gid, props_layer, history_group);
                let g = &mut graph.page.layers_mut()[props_layer];
                let cols = g
                    .book_id
                    .and_then(|id| options.column_options.get(&id))
                    .cloned()
                    .unwrap_or_default();
                egui::Grid::new(("props-grid", gid, props_layer))
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        data_source(ui, g, &options.book_specs);
                        columns(ui, g, &cols);
                        foreign_curves(
                            ui,
                            g,
                            &options.book_specs,
                            &options.curve_options,
                            &options.curve_names,
                            &lookup,
                        );
                        if g.plot_type == PlotType::FloatingBar {
                            ui.label("浮动柱下端");
                            crate::builder::floating_columns(ui, g, &cols);
                            ui.end_row();
                        }
                        titles(ui, g, history_group);
                        axes(ui, g, history_group);
                        layer_style(ui, g, history_group);
                        labels(ui, g, &cols, history_group);
                        error_bars(ui, g, history_group);
                        reference_lines(ui, g, gid, history_group);
                    });
                curve_style(ui, g, gid, props_layer, &cols, &options, history_group);
            });
        if self.project.graph(gid).unwrap() != &graph_before {
            ctx.request_repaint();
            let after = std::mem::replace(self.project.graph_mut(gid).unwrap(), graph_before);
            self.edit_grouped("绘图属性", self.history_group, |project| {
                *project.graph_mut(gid).unwrap() = after;
            });
        }
        if props_layer != self.layer_index(gid) {
            self.select_layer(gid, props_layer);
        }
        if !open {
            self.props_for = None;
        }
    }
}

type CurveOptions = HashMap<u64, Vec<(String, String, Option<String>)>>;

#[derive(Clone, Default)]
struct MappingColumns {
    color: Vec<(String, String)>,
    size: Vec<(String, String)>,
    symbol: Vec<(String, String)>,
}

struct SourceOptions {
    column_options: HashMap<u64, Vec<(String, String)>>,
    book_specs: Vec<(u64, String, Option<scicanvas_core::plotspec::PlotSpec>)>,
    curve_options: CurveOptions,
    curve_names: HashMap<DataRef, String>,
    mapping_columns: HashMap<u64, MappingColumns>,
}

fn source_options(project: &Project) -> SourceOptions {
    let column_options: HashMap<u64, Vec<(String, String)>> = project
        .books
        .iter()
        .map(|b| {
            (
                b.id,
                b.columns
                    .iter()
                    .map(|c| {
                        let label = if c.long_name.trim().is_empty() {
                            c.key.clone()
                        } else {
                            format!("{} · {}", c.key, c.long_name.trim())
                        };
                        (c.key.clone(), label)
                    })
                    .collect(),
            )
        })
        .collect();
    let mapping_columns: HashMap<u64, MappingColumns> = project
        .books
        .iter()
        .map(|book| {
            let mut options = MappingColumns::default();
            for column in &book.columns {
                let item = (column.key.clone(), column.display_name().to_owned());
                match column.data.kind() {
                    scicanvas_core::DataKind::Num => {
                        options.color.push(item.clone());
                        options.size.push(item);
                    }
                    scicanvas_core::DataKind::DateTime => options.color.push(item),
                    scicanvas_core::DataKind::Category => {
                        options.color.push(item.clone());
                        options.symbol.push(item);
                    }
                    scicanvas_core::DataKind::Text => {}
                }
            }
            (book.id, options)
        })
        .collect();
    // 每个工作簿的默认列映射（与“选中工作簿后作图”相同），切换数据来源时套用
    let book_specs: Vec<(u64, String, Option<scicanvas_core::plotspec::PlotSpec>)> = project
        .books
        .iter()
        .map(|b| {
            let spec = scicanvas_core::plotspec::PlotSpec::from_selection(b, &[], PlotType::LineSymbol).ok();
            (b.id, b.name.clone(), spec)
        })
        .collect();
    // 可加入图层的其他工作簿曲线：(列标识, 显示名, 默认 X 列)；以及带工作簿名的曲线显示名
    let mut curve_options: CurveOptions = HashMap::new();
    let mut curve_names: HashMap<DataRef, String> = HashMap::new();
    for b in &project.books {
        for c in &b.columns {
            curve_names.insert(DataRef::new(b.id, &c.key), format!("{}：{}", b.name, c.display_name()));
            if matches!(c.role, Role::Y | Role::Z) {
                curve_options.entry(b.id).or_default().push((
                    c.key.clone(),
                    c.display_name().to_string(),
                    default_x(b, &c.key),
                ));
            }
        }
    }
    SourceOptions {
        column_options,
        book_specs,
        curve_options,
        curve_names,
        mapping_columns,
    }
}

fn page(ui: &mut egui::Ui, graph: &mut scicanvas_core::Graph, history_group: &mut Option<egui::Id>) {
    ui.horizontal_wrapped(|ui| {
        ui.label("页面 mm");
        edit_group(
            &ui.add(
                egui::DragValue::new(&mut graph.page.width_mm)
                    .range(1.0..=1000.0)
                    .speed(0.5),
            ),
            history_group,
        );
        ui.label("×");
        edit_group(
            &ui.add(
                egui::DragValue::new(&mut graph.page.height_mm)
                    .range(1.0..=1000.0)
                    .speed(0.5),
            ),
            history_group,
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("页边距 mm");
        let page = &mut graph.page;
        for (i, side) in ["左", "上", "右", "下"].into_iter().enumerate() {
            // 对边之和留出至少 1 mm 给图层
            let span = if i % 2 == 0 { page.width_mm } else { page.height_mm };
            let max = (span - page.margin_mm[(i + 2) % 4] - 1.0).max(0.0);
            ui.label(side);
            edit_group(
                &ui.add(egui::DragValue::new(&mut page.margin_mm[i]).range(0.0..=max).speed(0.5)),
                history_group,
            );
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("页面背景");
        let mut rgb = graph.page.background.unwrap_or([255, 255, 255]);
        let response = ui.color_edit_button_srgb(&mut rgb);
        if response.changed() {
            graph.page.background = (rgb != [255, 255, 255]).then_some(rgb);
        }
        edit_group(&response, history_group);
        if graph.page.background.is_some() && ui.small_button("白色").clicked() {
            graph.page.background = None;
        }
    });
}

fn page_layout(
    ui: &mut egui::Ui,
    graph: &mut scicanvas_core::Graph,
    mut props_layer: usize,
    history_group: &mut Option<egui::Id>,
) -> usize {
    ui.horizontal_wrapped(|ui| {
        ui.label("排版列数");
        let n = graph.page.layers().len();
        edit_group(
            &ui.add(egui::DragValue::new(&mut graph.page.columns).range(1..=n)),
            history_group,
        );
        egui::ComboBox::from_id_salt("props-layer")
            .selected_text(format!("图层 {}", props_layer + 1))
            .show_ui(ui, |ui| {
                for i in 0..graph.page.layers().len() {
                    ui.selectable_value(&mut props_layer, i, format!("图层 {}", i + 1));
                }
            });
        ui.label("间距 mm");
        for gap in graph.page.gap_mm.iter_mut() {
            edit_group(
                &ui.add(egui::DragValue::new(gap).range(0.0..=100.0).speed(0.5)),
                history_group,
            );
        }
    });
    props_layer
}

fn layer(
    ui: &mut egui::Ui,
    graph: &mut scicanvas_core::Graph,
    gid: u64,
    props_layer: usize,
    history_group: &mut Option<egui::Id>,
) {
    let layer_count = graph.page.layers().len();
    if layer_count > 1 {
        let g = &mut graph.page.layers_mut()[props_layer];
        ui.horizontal_wrapped(|ui| {
            for (label, link, salt) in [
                ("联动 X 轴", &mut g.link_x, "props-link-x"),
                ("联动 Y 轴", &mut g.link_y, "props-link-y"),
            ] {
                ui.label(label);
                let text = link.map_or("无".to_string(), |j| format!("图层 {}", j + 1));
                egui::ComboBox::from_id_salt(salt)
                    .selected_text(text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(link, None, "无");
                        for j in (0..layer_count).filter(|&j| j != props_layer) {
                            ui.selectable_value(link, Some(j), format!("图层 {}", j + 1));
                        }
                    });
            }
        });
    }
    let g = &mut graph.page.layers_mut()[props_layer];
    ui.horizontal_wrapped(|ui| {
        let mut custom = g.placement.is_some();
        if ui.checkbox(&mut custom, "自定义图层位置").changed() {
            g.placement = custom.then(scicanvas_core::page::LayerRect::default);
        }
        ui.checkbox(&mut g.transparent, "透明图层背景");
    });
    if let Some(rect) = &mut g.placement {
        ui.label("位置和尺寸（页面比例，0–1）");
        egui::Grid::new(("layer-rectangle", gid, props_layer))
            .num_columns(2)
            .show(ui, |ui| {
                let min_width = f64::MIN_POSITIVE.min(rect.width);
                let min_height = f64::MIN_POSITIVE.min(rect.height);
                ui.label("X");
                edit_group(
                    &ui.add(
                        egui::DragValue::new(&mut rect.x)
                            .range(0.0..=1.0 - rect.width)
                            .speed(0.01),
                    ),
                    history_group,
                );
                ui.end_row();
                ui.label("Y");
                edit_group(
                    &ui.add(
                        egui::DragValue::new(&mut rect.y)
                            .range(0.0..=1.0 - rect.height)
                            .speed(0.01),
                    ),
                    history_group,
                );
                ui.end_row();
                ui.label("宽");
                edit_group(
                    &ui.add(
                        egui::DragValue::new(&mut rect.width)
                            .range(min_width..=1.0 - rect.x)
                            .speed(0.01),
                    ),
                    history_group,
                );
                ui.end_row();
                ui.label("高");
                edit_group(
                    &ui.add(
                        egui::DragValue::new(&mut rect.height)
                            .range(min_height..=1.0 - rect.y)
                            .speed(0.01),
                    ),
                    history_group,
                );
                ui.end_row();
            });
    }
}

fn data_source(
    ui: &mut egui::Ui,
    g: &mut scicanvas_core::page::Layer,
    book_specs: &[(u64, String, Option<scicanvas_core::plotspec::PlotSpec>)],
) {
    ui.label("数据工作簿");
    let current = book_specs
        .iter()
        .find(|(id, _, _)| Some(*id) == g.book_id)
        .map_or("（未选择）", |(_, name, _)| name.as_str());
    egui::ComboBox::from_id_salt("props-book")
        .selected_text(current)
        .show_ui(ui, |ui| {
            for (id, name, spec) in book_specs {
                if ui.selectable_label(g.book_id == Some(*id), name).clicked() && g.book_id != Some(*id) {
                    // 换用该工作簿的默认 X/Y 列，保留图类型、统计变换和配色
                    g.spec = g.spec.rebound(*id, spec.as_ref());
                }
            }
        })
        .response
        .on_hover_text("图层使用的数据表；切换后按该表的 X 列和全部 Y 列作图，可再在下方调整");
    ui.end_row();

    ui.label("图类型");
    egui::ComboBox::from_id_salt("props-type")
        .selected_text(g.plot_type.label())
        .show_ui(ui, |ui| {
            for t in PlotType::ALL {
                let mut next = g.spec.clone();
                let valid = next.set_plot_type(t);
                if ui
                    .add_enabled(valid.is_ok(), egui::SelectableLabel::new(g.plot_type == t, t.label()))
                    .on_hover_text(valid.as_ref().err().map_or(t.label(), String::as_str))
                    .clicked()
                {
                    g.spec = next;
                }
            }
        });
    ui.end_row();

    if matches!(g.plot_type, PlotType::Line | PlotType::LineSymbol) {
        ui.label("连接方式");
        ui.horizontal_wrapped(|ui| {
            for c in scicanvas_core::page::Connect::ALL {
                ui.radio_value(&mut g.connect, c, c.label());
            }
        });
        ui.end_row();
    }

    if g.plot_type == PlotType::HorizontalBar {
        ui.label("条形方向");
        ui.label("X 列为纵向位置，Y 列为横向数值");
        ui.end_row();
    }
    ui.label("统计变换");
    egui::ComboBox::from_id_salt("props-transform")
        .selected_text(if g.transform == scicanvas_core::plotspec::Transform::Mean {
            "按 X 求均值"
        } else {
            "原始数据"
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut g.transform,
                scicanvas_core::plotspec::Transform::Identity,
                "原始数据",
            );
            ui.selectable_value(
                &mut g.transform,
                scicanvas_core::plotspec::Transform::Mean,
                "按 X 求均值",
            );
        });
    ui.end_row();
}

fn columns(ui: &mut egui::Ui, g: &mut scicanvas_core::page::Layer, cols: &[(String, String)]) {
    ui.label("X 列");
    let x_text = g
        .x_key
        .as_ref()
        .and_then(|k| cols.iter().find(|(kk, _)| kk == k))
        .map(|(_, l)| l.clone())
        .unwrap_or_else(|| "行号".into());
    egui::ComboBox::from_id_salt("props-x")
        .selected_text(x_text)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut g.x_key, None, "行号");
            for (k, l) in cols {
                ui.selectable_value(&mut g.x_key, Some(k.clone()), l.as_str());
            }
        });
    ui.end_row();

    ui.label("Y 列");
    let keys = g.plot_sources(true);
    ui.vertical(|ui| {
        for (k, l) in cols {
            let source = DataRef::new(g.book_id.unwrap_or(0), k);
            let mut on = keys.contains(&source);
            if ui.checkbox(&mut on, l.as_str()).changed() {
                g.set_plot_selected(&source, on);
            }
        }
    });
    ui.end_row();
}

fn foreign_curves(
    ui: &mut egui::Ui,
    g: &mut scicanvas_core::page::Layer,
    book_specs: &[(u64, String, Option<scicanvas_core::plotspec::PlotSpec>)],
    curve_options: &CurveOptions,
    curve_names: &HashMap<DataRef, String>,
    books: BookLookup<'_, '_>,
) {
    // 其他工作簿的曲线：同一图层可以放不同工作簿的数据
    let foreign: Vec<_> = g
        .plots(books)
        .into_iter()
        .filter(|plot| Some(plot.mapping.y.book) != g.book_id)
        .collect();
    if book_specs.len() > 1 || !foreign.is_empty() {
        ui.label("其他工作簿");
        ui.vertical(|ui| {
            for plot in &foreign {
                let k = &plot.key;
                ui.horizontal_wrapped(|ui| {
                    ui.label(curve_names.get(&plot.mapping.y).map_or(k.as_str(), String::as_str));
                    if ui.small_button("✕").on_hover_text("移除").clicked() {
                        g.remove_plot(&plot.mapping.y);
                    }
                });
            }
            ui.menu_button("＋ 添加曲线", |ui| {
                for (bid, name, _) in book_specs {
                    if Some(*bid) == g.book_id {
                        continue;
                    }
                    let Some(options) = curve_options.get(bid) else {
                        continue;
                    };
                    ui.menu_button(name, |ui| {
                        for (key, label, x) in options {
                            let source = DataRef::new(*bid, key);
                            let on = g.plot_sources(true).contains(&source);
                            if ui.add_enabled(!on, egui::Button::new(label)).clicked() {
                                let x = x.as_ref().map(|x| DataRef::new(*bid, x));
                                g.select_plot(&source, x.as_ref());
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
        });
        ui.end_row();
    }
}

fn titles(ui: &mut egui::Ui, g: &mut scicanvas_core::page::Layer, history_group: &mut Option<egui::Id>) {
    ui.label("文字格式");
    ui.checkbox(&mut g.rich_text, "启用科学文字格式")
        .on_hover_text("支持上下标、局部粗体斜体和希腊字母。关闭时按原文显示。");
    ui.end_row();
    ui.label("图标题");
    edit_group(
        &dialogs::scientific_text(ui, &mut g.title, &mut g.rich_text, "（无）"),
        history_group,
    );
    ui.end_row();
    ui.label("X 轴标题");
    edit_group(
        &dialogs::scientific_text(ui, &mut g.x_title, &mut g.rich_text, "自动：长名称 (单位)"),
        history_group,
    );
    ui.end_row();
    ui.label("Y 轴标题");
    edit_group(
        &dialogs::scientific_text(ui, &mut g.y_title, &mut g.rich_text, "自动：长名称 (单位)"),
        history_group,
    );
    ui.end_row();
}

fn axes(ui: &mut egui::Ui, g: &mut scicanvas_core::page::Layer, history_group: &mut Option<egui::Id>) {
    use scicanvas_core::format::NumberFormat;
    // 刻度格式：日期列做 X 时自动用日期刻度，这里只设数字格式
    for (label, format, salt) in [
        ("X 刻度格式", &mut g.x_format, "props-x-format"),
        ("左 Y 刻度格式", &mut g.y_format, "props-y-format"),
    ] {
        ui.label(label);
        ui.horizontal_wrapped(|ui| {
            let mut kind = format.index();
            let mut places = format.places();
            egui::ComboBox::from_id_salt(salt)
                .selected_text(NumberFormat::KINDS[kind])
                .show_ui(ui, |ui| {
                    for (i, name) in NumberFormat::KINDS.iter().enumerate() {
                        ui.selectable_value(&mut kind, i, *name);
                    }
                });
            if kind != 0 {
                ui.label("小数位");
                ui.add(egui::DragValue::new(&mut places).range(0..=10));
            }
            *format = NumberFormat::from_index(kind, places);
        });
        ui.end_row();
    }
    if g.has_right_axis_style() {
        let y2_format = g.y2_format;
        let linked_places = y2_format.unwrap_or(g.y_format).places();
        ui.label("右 Y 刻度格式");
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("props-y2-format")
                .selected_text(y2_format.map_or("跟随左 Y", |format| NumberFormat::KINDS[format.index()]))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut g.y2_format, None, "跟随左 Y");
                    for (i, name) in NumberFormat::KINDS.iter().enumerate() {
                        ui.selectable_value(
                            &mut g.y2_format,
                            Some(NumberFormat::from_index(i, linked_places)),
                            *name,
                        );
                    }
                });
            if let Some(format) = g.y2_format {
                if format.index() != 0 {
                    let mut places = format.places();
                    ui.label("小数位");
                    let response = ui.add(egui::DragValue::new(&mut places).range(0..=10));
                    edit_group(&response, history_group);
                    g.y2_format = Some(NumberFormat::from_index(format.index(), places));
                }
            }
        });
        ui.end_row();
    }

    ui.label("X 范围");
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut g.x_auto, "自动");
        edit_group(
            &ui.add_enabled(!g.x_auto, egui::DragValue::new(&mut g.x_min).speed(0.1)),
            history_group,
        );
        ui.label("到");
        edit_group(
            &ui.add_enabled(!g.x_auto, egui::DragValue::new(&mut g.x_max).speed(0.1)),
            history_group,
        );
        ui.checkbox(&mut g.x_log, "对数");
    });
    ui.end_row();
    ui.label("左 Y 范围");
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut g.y_auto, "自动");
        edit_group(
            &ui.add_enabled(!g.y_auto, egui::DragValue::new(&mut g.y_min).speed(0.1)),
            history_group,
        );
        ui.label("到");
        edit_group(
            &ui.add_enabled(!g.y_auto, egui::DragValue::new(&mut g.y_max).speed(0.1)),
            history_group,
        );
        ui.checkbox(&mut g.y_log, "对数（左右共用）");
    });
    ui.end_row();
    if g.has_right_axis_style() {
        ui.label("右 Y 范围");
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut g.y2_auto, "自动");
            edit_group(
                &ui.add_enabled(!g.y2_auto, egui::DragValue::new(&mut g.y2_min).speed(0.1)),
                history_group,
            );
            ui.label("到");
            edit_group(
                &ui.add_enabled(!g.y2_auto, egui::DragValue::new(&mut g.y2_max).speed(0.1)),
                history_group,
            );
        });
        ui.end_row();
    }
}

fn layer_style(ui: &mut egui::Ui, g: &mut scicanvas_core::page::Layer, history_group: &mut Option<egui::Id>) {
    ui.label("线宽");
    edit_group(&ui.add(egui::Slider::new(&mut g.line_width, 0.5..=6.0)), history_group);
    ui.end_row();
    ui.label("符号大小");
    edit_group(
        &ui.add(egui::Slider::new(&mut g.symbol_size, 2.0..=20.0)),
        history_group,
    );
    ui.end_row();
    if g.plot_type == PlotType::Area {
        ui.label("填充透明度");
        edit_group(
            &ui.add(egui::Slider::new(&mut g.area_opacity, 0.0..=1.0).text("面积填充")),
            history_group,
        );
        ui.end_row();
    }
    ui.label("显示");
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut g.grid, "网格线");
        ui.checkbox(&mut g.legend, "图例");
        ui.checkbox(&mut g.labels.visible, "数据标签");
        ui.checkbox(&mut g.frame, "四边框");
    });
    ui.end_row();
    ui.label("刻度方向");
    ui.horizontal_wrapped(|ui| {
        for dir in scicanvas_core::page::TickDir::ALL {
            ui.radio_value(&mut g.tick_dir, dir, dir.label());
        }
    });
    ui.end_row();
}

fn labels(
    ui: &mut egui::Ui,
    g: &mut scicanvas_core::page::Layer,
    cols: &[(String, String)],
    history_group: &mut Option<egui::Id>,
) {
    if g.labels.visible {
        ui.label("标签来源");
        use scicanvas_core::page::LabelSource;
        egui::ComboBox::from_id_salt("label-source")
            .selected_text(match g.labels.source {
                LabelSource::Value => "Y 数值",
                LabelSource::XY => "X / Y",
                LabelSource::Row => "原始行号",
                LabelSource::Column => "指定列文本",
            })
            .show_ui(ui, |ui| {
                for (value, label) in [
                    (LabelSource::Value, "Y 数值"),
                    (LabelSource::XY, "X / Y"),
                    (LabelSource::Row, "原始行号"),
                    (LabelSource::Column, "指定列文本"),
                ] {
                    ui.selectable_value(&mut g.labels.source, value, label);
                }
            });
        ui.end_row();
        if g.labels.source == LabelSource::Column {
            ui.label("标签列");
            egui::ComboBox::from_id_salt("label-column")
                .selected_text(g.labels.column.as_deref().unwrap_or("请选择"))
                .show_ui(ui, |ui| {
                    for (k, name) in cols {
                        ui.selectable_value(&mut g.labels.column, Some(k.clone()), name);
                    }
                });
            ui.end_row();
        }
        if g.transform == scicanvas_core::plotspec::Transform::Mean {
            ui.label("均值视图");
            ui.label("使用数值或 X/Y 标签");
            ui.end_row();
        }
        ui.label("标签字号");
        edit_group(
            &ui.add(egui::Slider::new(&mut g.labels.size, 6.0..=32.0)),
            history_group,
        );
        ui.end_row();
        ui.label("标签偏移");
        edit_group(
            &ui.add(egui::Slider::new(&mut g.labels.offset, 0.0..=40.0)),
            history_group,
        );
        ui.end_row();
    }
}

fn error_bars(ui: &mut egui::Ui, g: &mut scicanvas_core::page::Layer, history_group: &mut Option<egui::Id>) {
    ui.label("误差棒");
    ui.checkbox(&mut g.error_bars.visible, "显示 X / Y 误差");
    ui.end_row();
    ui.label("误差端帽");
    edit_group(
        &ui.add(egui::Slider::new(&mut g.error_bars.cap, 0.0..=20.0)),
        history_group,
    );
    ui.end_row();
    ui.label("误差线宽");
    edit_group(
        &ui.add(egui::Slider::new(&mut g.error_bars.width, 0.5..=6.0)),
        history_group,
    );
    ui.end_row();
}

fn reference_lines(
    ui: &mut egui::Ui,
    g: &mut scicanvas_core::page::Layer,
    gid: u64,
    history_group: &mut Option<egui::Id>,
) {
    use scicanvas_core::page::{ReferenceAxis, ReferenceLine};
    ui.label("参考线");
    ui.vertical(|ui| {
        ui.horizontal_wrapped(|ui| {
            let vertical = ui.button("＋垂直");
            edit_group(&vertical, history_group);
            if vertical.clicked() {
                g.reference_lines.push(ReferenceLine {
                    axis: ReferenceAxis::X,
                    value: if g.x_log { 1.0 } else { 0.0 },
                    ..Default::default()
                });
            }
            let horizontal = ui.button("＋水平");
            edit_group(&horizontal, history_group);
            if horizontal.clicked() {
                g.reference_lines.push(ReferenceLine {
                    axis: ReferenceAxis::YLeft,
                    value: if g.y_log { 1.0 } else { 0.0 },
                    ..Default::default()
                });
            }
        });
        let mut remove = None;
        let has_right_axis = g.has_right_axis_style();
        for (index, line) in g.reference_lines.iter_mut().enumerate() {
            ui.horizontal_wrapped(|ui| {
                let axis = egui::ComboBox::from_id_salt(("reference-axis", gid, index))
                    .selected_text(line.axis.label())
                    .show_ui(ui, |ui| {
                        for axis in ReferenceAxis::ALL {
                            ui.add_enabled_ui(axis != ReferenceAxis::YRight || has_right_axis, |ui| {
                                ui.selectable_value(&mut line.axis, axis, axis.label());
                            });
                        }
                    })
                    .response;
                edit_group(&axis, history_group);
                let value = ui.add(egui::DragValue::new(&mut line.value).speed(0.1).max_decimals(6));
                edit_group(&value, history_group);
                let color = ui.color_edit_button_srgb(&mut line.color);
                edit_group(&color, history_group);
                let width = ui.add(egui::DragValue::new(&mut line.width).range(0.2..=8.0).speed(0.1));
                edit_group(&width, history_group);
                let dashed = ui.checkbox(&mut line.dashed, "虚线");
                edit_group(&dashed, history_group);
                let delete = ui.small_button("删除");
                edit_group(&delete, history_group);
                if delete.clicked() {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            g.reference_lines.remove(index);
        }
    });
    ui.end_row();
}

fn curve_style(
    ui: &mut egui::Ui,
    g: &mut scicanvas_core::page::Layer,
    gid: u64,
    props_layer: usize,
    cols: &[(String, String)],
    options: &SourceOptions,
    history_group: &mut Option<egui::Id>,
) {
    // 逐曲线样式：选一条数据曲线，单独设图型、颜色、符号、线宽、符号大小和右侧 Y 轴
    let curve_keys = g.plot_sources(false);
    if !curve_keys.is_empty() {
        egui::CollapsingHeader::new("曲线样式")
            .id_salt(("props-curves", gid, props_layer))
            .show(ui, |ui| {
                use scicanvas_core::page::CurveStyle;
                let pick_id = egui::Id::new(("props-curve-pick", gid, props_layer));
                let mut key = ui
                    .data(|d| d.get_temp::<DataRef>(pick_id))
                    .filter(|k| curve_keys.contains(k))
                    .unwrap_or_else(|| curve_keys[0].clone());
                let name_of = |source: &DataRef| {
                    cols.iter()
                        .find(|(c, _)| Some(source.book) == g.book_id && c == &source.col)
                        .map(|(_, n)| n.clone())
                        .or_else(|| options.curve_names.get(source).cloned())
                        .unwrap_or_else(|| {
                            if source.book == 0 {
                                source.col.clone()
                            } else {
                                source.key(g.book_id)
                            }
                        })
                };
                egui::ComboBox::from_id_salt(("props-curve", gid))
                    .selected_text(name_of(&key))
                    .show_ui(ui, |ui| {
                        for k in &curve_keys {
                            ui.selectable_value(&mut key, k.clone(), name_of(k));
                        }
                    });
                ui.data_mut(|d| d.insert_temp(pick_id, key.clone()));
                let mut style = g.source_style(&key).cloned().unwrap_or_default();
                let mapping = options.mapping_columns.get(&key.book).cloned().unwrap_or_default();
                egui::Grid::new(("props-curve-grid", gid, props_layer))
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("图例文字");
                        edit_group(
                            &dialogs::scientific_text(ui, &mut style.legend_label, &mut g.rich_text, "自动：曲线名称"),
                            history_group,
                        );
                        ui.end_row();
                        ui.label("图型");
                        let text = style.kind.map_or("跟随图层", |k| k.label());
                        ui.add_enabled_ui(
                            !g.plot_type.is_stacked() && g.plot_type != PlotType::HorizontalBar,
                            |ui| {
                                egui::ComboBox::from_id_salt(("props-curve-kind", gid))
                                    .selected_text(text)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut style.kind, None, "跟随图层");
                                        for k in CurveStyle::KINDS {
                                            ui.selectable_value(&mut style.kind, Some(k), k.label());
                                        }
                                    });
                            },
                        );
                        ui.end_row();
                        ui.label("颜色");
                        ui.horizontal_wrapped(|ui| {
                            let mut own = style.color.is_some();
                            ui.checkbox(&mut own, "自定");
                            let mut rgb = style.color.unwrap_or([15, 77, 146]);
                            let r = ui.add_enabled_ui(own, |ui| ui.color_edit_button_srgb(&mut rgb)).inner;
                            edit_group(&r, history_group);
                            style.color = own.then_some(rgb);
                        });
                        ui.end_row();
                        ui.label("符号");
                        const NAMES: [&str; 8] =
                            ["圆", "方块", "上三角", "下三角", "菱形", "左三角", "右三角", "六边形"];
                        let label = |sym: scicanvas_core::Symbol| {
                            scicanvas_core::SYMBOLS
                                .iter()
                                .position(|s| *s == sym)
                                .map_or("", |i| NAMES[i])
                        };
                        egui::ComboBox::from_id_salt(("props-curve-symbol", gid))
                            .selected_text(style.symbol.map_or("跟随图层", label))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut style.symbol, None, "跟随图层");
                                for sym in scicanvas_core::SYMBOLS {
                                    ui.selectable_value(&mut style.symbol, Some(sym), label(sym));
                                }
                            });
                        ui.end_row();
                        for (name, value, default, range) in [
                            ("线宽", &mut style.line_width, g.line_width, 0.2..=8.0),
                            ("符号大小", &mut style.symbol_size, g.symbol_size, 1.0..=20.0),
                        ] {
                            ui.label(name);
                            ui.horizontal_wrapped(|ui| {
                                let mut own = value.is_some();
                                ui.checkbox(&mut own, "自定");
                                let mut v = value.unwrap_or(default);
                                let r = ui.add_enabled(own, egui::DragValue::new(&mut v).range(range).speed(0.1));
                                edit_group(&r, history_group);
                                *value = own.then_some(v);
                            });
                            ui.end_row();
                        }
                        ui.label("坐标轴");
                        ui.checkbox(&mut style.right_axis, "右侧 Y 轴");
                        ui.end_row();
                    });
                point_style_properties(ui, gid, &key, &mut style, &mapping, history_group);
                g.set_source_style(&key, (!style.is_default()).then_some(style));
                if g.has_right_axis_style() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("右轴标题");
                        edit_group(
                            &dialogs::scientific_text(ui, &mut g.y2_title, &mut g.rich_text, "自动：长名称 (单位)"),
                            history_group,
                        );
                    });
                }
            });
    }
}

fn point_style_properties(
    ui: &mut egui::Ui,
    gid: u64,
    key: &DataRef,
    style: &mut scicanvas_core::page::CurveStyle,
    mapping: &MappingColumns,
    history_group: &mut Option<egui::Id>,
) {
    use scicanvas_core::page::SymbolFill;
    ui.label(RichText::new("符号边框、填充与逐点映射").strong());
    egui::Grid::new(("props-point-style-grid", gid, key.clone()))
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("符号填充");
            let mut fill = style.symbol_fill.unwrap_or_default();
            let fill_label = match fill {
                SymbolFill::MatchCurve => "跟随曲线颜色",
                SymbolFill::Hollow => "空心",
                SymbolFill::Color(_) => "自定义颜色",
            };
            let response = egui::ComboBox::from_id_salt(("props-symbol-fill", gid, key.clone()))
                .selected_text(fill_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut fill, SymbolFill::MatchCurve, "跟随曲线颜色");
                    ui.selectable_value(&mut fill, SymbolFill::Hollow, "空心");
                    ui.selectable_value(&mut fill, SymbolFill::Color([15, 77, 146]), "自定义颜色");
                })
                .response;
            edit_group(&response, history_group);
            if let SymbolFill::Color(mut color) = fill {
                let response = ui.color_edit_button_srgb(&mut color);
                edit_group(&response, history_group);
                fill = SymbolFill::Color(color);
            }
            style.symbol_fill = (fill != SymbolFill::MatchCurve).then_some(fill);
            ui.end_row();

            ui.label("边框");
            ui.horizontal_wrapped(|ui| {
                let mut enabled = style.symbol_border_color.is_some() || style.symbol_border_width.is_some();
                let response = ui.checkbox(&mut enabled, "启用");
                edit_group(&response, history_group);
                if enabled {
                    let mut color = style.symbol_border_color.unwrap_or([0, 0, 0]);
                    let response = ui.color_edit_button_srgb(&mut color);
                    edit_group(&response, history_group);
                    style.symbol_border_color = Some(color);
                    let mut width = style.symbol_border_width.unwrap_or(0.6);
                    let response = ui.add(egui::DragValue::new(&mut width).range(0.0..=8.0).speed(0.1));
                    edit_group(&response, history_group);
                    style.symbol_border_width = Some(width);
                } else {
                    style.symbol_border_color = None;
                    style.symbol_border_width = None;
                }
            });
            ui.end_row();

            ui.label("颜色映射列");
            mapped_column_combo(
                ui,
                ("props-color-map", gid, key.clone()),
                &mut style.color_column,
                &mapping.color,
                history_group,
            );
            ui.end_row();
            ui.label("大小映射列");
            mapped_column_combo(
                ui,
                ("props-size-map", gid, key.clone()),
                &mut style.size_column,
                &mapping.size,
                history_group,
            );
            ui.end_row();
            if style.size_column.is_some() {
                ui.label("映射尺寸范围");
                let mut range = style.size_range.unwrap_or([2.0, 12.0]);
                ui.horizontal_wrapped(|ui| {
                    ui.label("最小");
                    let response = ui.add(egui::DragValue::new(&mut range[0]).range(1.0..=20.0).speed(0.1));
                    edit_group(&response, history_group);
                    ui.label("最大");
                    let response = ui.add(egui::DragValue::new(&mut range[1]).range(1.0..=20.0).speed(0.1));
                    edit_group(&response, history_group);
                });
                style.size_range = Some(range);
                ui.end_row();
            } else {
                style.size_range = None;
            }
            ui.label("形状映射列");
            mapped_column_combo(
                ui,
                ("props-symbol-map", gid, key.clone()),
                &mut style.symbol_column,
                &mapping.symbol,
                history_group,
            );
            ui.end_row();
        });
}

fn mapped_column_combo(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    value: &mut Option<String>,
    columns: &[(String, String)],
    history_group: &mut Option<egui::Id>,
) {
    let response = egui::ComboBox::from_id_salt(id)
        .selected_text(value.as_deref().unwrap_or("无"))
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "无");
            for (key, label) in columns {
                ui.selectable_value(value, Some(key.clone()), label);
            }
        })
        .response;
    edit_group(&response, history_group);
}
