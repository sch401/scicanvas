//! 选项卡式功能区（类似 Word）、文件页和 Ctrl+K 命令面板。
//!
//! 这里只负责显示和收集点击：被点击的命令 id 写进 `clicked`，由 app.rs 在绘制结束后执行。
//! 这样渲染时只需要只读地访问程序状态。

use eframe::egui::{self, vec2, Align, Align2, Color32, FontId, Layout, Rect, RichText, Sense, Stroke};

use crate::app::SciApp;
use crate::commands::{self, Command};
use crate::icons;
use crate::theme::Palette;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ctx {
    Book,
    Graph,
}

pub enum Item {
    /// 大按钮：图标在上，文字在下。
    Big(&'static str),
    /// 一列小按钮（最多三个）。
    Smalls(&'static [&'static str]),
    /// 图类型画廊（带缩略图）。
    Gallery(&'static [&'static str]),
}

pub struct Group {
    pub label: &'static str,
    pub items: &'static [Item],
}

pub struct Tab {
    pub id: &'static str,
    pub label: &'static str,
    /// 上下文选项卡：只在激活对应窗口时出现。
    pub ctx: Option<Ctx>,
    pub groups: &'static [Group],
}

use Item::*;

pub static TABS: &[Tab] = &[
    Tab {
        id: "home",
        label: "开始",
        ctx: None,
        groups: &[
            Group {
                label: "项目",
                items: &[
                    Big("book.new"),
                    Smalls(&["note.new", "file.import"]),
                    Smalls(&["file.open", "file.save"]),
                ],
            },
            Group {
                label: "选区",
                items: &[
                    Smalls(&["edit.undo", "edit.redo"]),
                    Smalls(&["edit.copy", "edit.clear"]),
                ],
            },
            Group {
                label: "快速绘图",
                items: &[Big("plot.linesym"), Smalls(&["plot.line", "plot.scatter", "plot.bar"])],
            },
            Group {
                label: "分析",
                items: &[Big("fit.linear"), Smalls(&["math.smooth", "stats.columns"])],
            },
            Group {
                label: "窗口",
                items: &[Smalls(&["win.cascade", "win.tile", "win.minall"])],
            },
        ],
    },
    Tab {
        id: "plot",
        label: "绘图",
        ctx: None,
        groups: &[
            Group {
                label: "二维图",
                items: &[
                    Gallery(&[
                        "plot.line",
                        "plot.scatter",
                        "plot.linesym",
                        "plot.bar",
                        "plot.stacked_bar",
                        "plot.percent_stacked_bar",
                        "plot.floating_bar",
                        "plot.horizontal_bar",
                        "plot.area",
                        "plot.spline",
                        "plot.spline_symbol",
                        "plot.vertical_drop",
                        "plot.step_horizontal",
                        "plot.step_vertical",
                    ]),
                    Big("plot.builder"),
                ],
            },
            Group {
                label: "多面板",
                items: &[Smalls(&[
                    "plot.panels_stacked",
                    "plot.panels_side",
                    "plot.panels_grid4",
                ])],
            },
            Group {
                label: "图形",
                items: &[
                    Big("graph.props"),
                    Smalls(&["graph.export", "graph.export_pdf", "graph.export_png"]),
                    Smalls(&["graph.copy"]),
                ],
            },
            Group {
                label: "模板",
                items: &[Smalls(&["graph.template.save", "graph.template.apply"])],
            },
        ],
    },
    Tab {
        id: "analysis",
        label: "分析",
        ctx: None,
        groups: &[
            Group {
                label: "数据选择",
                items: &[Smalls(&["analysis.curves"])],
            },
            Group {
                label: "拟合",
                items: &[
                    Big("fit.linear"),
                    Big("fit.poly"),
                    Big("fit.nonlinear"),
                    Smalls(&["fit.reports"]),
                    Big("fit.calibration"),
                    Big("fit.mlr"),
                ],
            },
            Group {
                label: "数学",
                items: &[
                    Smalls(&["math.smooth", "math.diff", "math.integ"]),
                    Smalls(&["math.norm", "math.interp", "math.colmath"]),
                    Smalls(&["math.average", "math.diff_sg", "data.recalc"]),
                ],
            },
            Group {
                label: "信号处理",
                items: &[
                    Big("signal.baseline"),
                    Big("signal.peaks"),
                    Big("signal.fft"),
                    Big("signal.ifft"),
                    Big("signal.filter"),
                    Big("signal.correlate"),
                ],
            },
        ],
    },
    Tab {
        id: "stats",
        label: "统计",
        ctx: None,
        groups: &[
            Group {
                label: "描述统计",
                items: &[Big("stats.columns"), Big("stats.rows"), Big("stats.frequency")],
            },
            Group {
                label: "假设检验",
                items: &[Big("stats.ttest"), Big("stats.anova"), Big("stats.correlation")],
            },
            Group {
                label: "数据检验",
                items: &[Big("stats.normality"), Big("stats.grubbs")],
            },
        ],
    },
    Tab {
        id: "sheet",
        label: "工作表",
        ctx: Some(Ctx::Book),
        groups: &[
            Group {
                label: "选区",
                items: &[
                    Smalls(&["edit.copy", "edit.clear"]),
                    Smalls(&["edit.find", "edit.replace"]),
                ],
            },
            Group {
                label: "列属性",
                items: &[
                    Smalls(&["col.setX", "col.setY", "col.setIgnore"]),
                    Smalls(&["col.setXError", "col.setYError"]),
                ],
            },
            Group {
                label: "行",
                items: &[Smalls(&["row.insert", "row.delete"])],
            },
            Group {
                label: "列",
                items: &[
                    Big("col.props"),
                    Big("col.setvalues"),
                    Big("col.fill"),
                    Smalls(&["col.insert", "col.delete", "col.clear"]),
                    Smalls(&["col.left", "col.right"]),
                ],
            },
            Group {
                label: "排序",
                items: &[Smalls(&["sort.asc", "sort.desc", "sort.custom"])],
            },
            Group {
                label: "变换",
                items: &[
                    Smalls(&["book.transpose", "sheet.stack", "sheet.split"]),
                    Smalls(&["sheet.reduce", "sheet.freeze", "sheet.unfreeze"]),
                ],
            },
            Group {
                label: "模板",
                items: &[Smalls(&["book.template.save", "book.template.open"])],
            },
            Group {
                label: "导出",
                items: &[Smalls(&["book.export_csv", "project.export_xlsx"])],
            },
            Group {
                label: "绘图",
                items: &[Big("plot.linesym"), Smalls(&["plot.line", "plot.scatter", "plot.bar"])],
            },
        ],
    },
    Tab {
        id: "graph",
        label: "图形",
        ctx: Some(Ctx::Graph),
        groups: &[
            Group {
                label: "图类型",
                items: &[
                    Smalls(&["plot.line", "plot.scatter", "plot.linesym"]),
                    Smalls(&["plot.bar", "plot.step_horizontal", "plot.step_vertical"]),
                    Smalls(&["plot.stacked_bar", "plot.percent_stacked_bar", "plot.floating_bar"]),
                    Smalls(&["plot.horizontal_bar", "plot.area"]),
                    Smalls(&["plot.spline", "plot.spline_symbol", "plot.vertical_drop"]),
                ],
            },
            Group {
                label: "坐标轴",
                items: &[
                    Smalls(&["graph.logx", "graph.logy", "graph.grid"]),
                    Smalls(&["graph.reference.vertical", "graph.reference.horizontal"]),
                ],
            },
            Group {
                label: "图例",
                items: &[Big("graph.legend"), Big("graph.labels")],
            },
            Group {
                label: "标注",
                items: &[Smalls(&["graph.text.add", "graph.text.manage", "graph.legend.auto"])],
            },
            Group {
                label: "设置",
                items: &[
                    Big("graph.props"),
                    Big("layer.contents"),
                    Smalls(&["layer.add", "layer.remove"]),
                ],
            },
            Group {
                label: "导出",
                items: &[
                    Big("graph.export"),
                    Smalls(&[
                        "graph.export_pdf",
                        "graph.export_png",
                        "graph.export_tiff",
                        "graph.copy",
                    ]),
                ],
            },
        ],
    },
    Tab {
        id: "view",
        label: "视图",
        ctx: None,
        groups: &[
            Group {
                label: "主题",
                items: &[Smalls(&["theme.light", "theme.dark", "theme.system"])],
            },
            Group {
                label: "显示",
                items: &[Big("view.explorer"), Smalls(&["ribbon.toggle", "project.folder.new"])],
            },
            Group {
                label: "窗口",
                items: &[
                    Smalls(&["win.cascade", "win.tile", "win.minall"]),
                    Smalls(&["win.showall", "win.hideall", "win.reset"]),
                ],
            },
            Group {
                label: "帮助",
                items: &[Big("help.show"), Smalls(&["palette.open"])],
            },
        ],
    },
];

const QAT: &[&str] = &[
    "book.new",
    "file.import",
    "file.save",
    "edit.undo",
    "edit.redo",
    "graph.export",
];
const BACKSTAGE_NAV: &[&str] = &[
    "project.new",
    "book.new",
    "file.import",
    "file.open",
    "file.save",
    "file.saveas",
    "graph.export",
    "graph.export_png",
    "graph.export_pdf",
    "book.export_csv",
    "project.reset",
    "help.show",
    "app.quit",
];

pub struct PaletteState {
    pub query: String,
    pub sel: usize,
    pub focus: bool,
}

/// 功能区自己的界面状态。
pub struct UiState {
    pub tab: &'static str,
    pub collapsed: bool,
    /// 折叠时单击选项卡临时弹出的面板。
    pub flyout: bool,
    pub backstage: bool,
    pub(crate) recent_open: Option<usize>,
    pub palette: Option<PaletteState>,
    /// 上一帧选项卡栏所在区域，用来判断是否点在弹出面板外面。
    tab_rect: Rect,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            tab: "home",
            collapsed: false,
            flyout: false,
            backstage: false,
            recent_open: None,
            palette: None,
            tab_rect: Rect::NOTHING,
        }
    }
}

impl UiState {
    pub fn open_palette(&mut self) {
        self.backstage = false;
        self.palette = Some(PaletteState {
            query: String::new(),
            sel: 0,
            focus: true,
        });
    }

    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
        self.flyout = false;
    }
}

fn visible_tabs(ctx: Option<Ctx>) -> impl Iterator<Item = &'static Tab> {
    TABS.iter().filter(move |t| t.ctx.is_none() || t.ctx == ctx)
}

/// 绘制标题栏、选项卡栏和功能区。被点击的命令写进 `clicked`。
pub fn show(ctx: &egui::Context, app: &SciApp, st: &mut UiState, pal: &Palette, clicked: &mut Option<&'static str>) {
    let active_ctx = app.active_ctx();
    if !visible_tabs(active_ctx).any(|t| t.id == st.tab) {
        st.tab = "home";
    }
    let dark = pal.dark;

    egui::TopBottomPanel::top("ribbon")
        .frame(egui::Frame::none().fill(pal.chrome))
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
            // 标题栏：快速访问 + 名称 + 搜索 + 窗口按钮；空白处拖动移窗、双击最大化
            let title_rect = Rect::from_min_size(
                ui.cursor().min,
                vec2(ui.available_width(), crate::chrome::CAPTION_HEIGHT),
            );
            crate::chrome::store_caption(ctx, title_rect);
            crate::chrome::drag_area(ui, title_rect);
            ui.allocate_ui_with_layout(title_rect.size(), Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(6.0);
                for id in QAT {
                    if let Some(c) = commands::find(id) {
                        if icon_button(ui, app, c, dark, pal, 18.0).clicked() {
                            *clicked = Some(c.id);
                        }
                    }
                }
                ui.add_space(8.0);
                ui.label(RichText::new("SciCanvas").strong().color(pal.accent));
                ui.add_space(6.0);
                ui.label(RichText::new(app.doc_title()).color(pal.weak));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if crate::chrome::buttons(ui, pal) {
                        *clicked = Some("app.quit");
                    }
                    ui.add_space(12.0);
                    let text = RichText::new("搜索命令            Ctrl K").color(pal.weak);
                    let b = egui::Button::image_and_text(
                        icons::icon("search", dark).fit_to_exact_size(vec2(14.0, 14.0)),
                        text,
                    )
                    .min_size(vec2(220.0, 24.0));
                    if ui.add(b).on_hover_text("搜索任何命令（Ctrl+K）").clicked() {
                        *clicked = Some("palette.open");
                    }
                });
            });
            ui.add_space(4.0);

            // 选项卡栏
            let tabs_resp = ui.horizontal(|ui| {
                ui.add_space(6.0);
                if file_tab(ui, pal).clicked() {
                    st.backstage = true;
                    st.palette = None;
                }
                for t in visible_tabs(active_ctx) {
                    let r = tab_button(ui, t, st.tab == t.id, pal);
                    if r.clicked() {
                        st.tab = t.id;
                        if st.collapsed {
                            st.flyout = true;
                        }
                    }
                    if r.double_clicked() {
                        st.toggle_collapsed();
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let name = if st.collapsed { "chevrondown" } else { "chevron" };
                    let b = egui::ImageButton::new(icons::icon(name, dark).fit_to_exact_size(vec2(14.0, 14.0)))
                        .frame(false);
                    if ui
                        .add(b)
                        .on_hover_text(if st.collapsed {
                            "展开功能区（Ctrl+F1）"
                        } else {
                            "折叠功能区（Ctrl+F1）"
                        })
                        .clicked()
                    {
                        st.toggle_collapsed();
                    }
                });
            });
            st.tab_rect = tabs_resp.response.rect;

            if !st.collapsed {
                panel(ui, app, st.tab, pal, clicked);
            }
        });

    // 折叠状态下单击选项卡：临时弹出面板
    if st.collapsed && st.flyout {
        let pos = st.tab_rect.left_bottom();
        let area = egui::Area::new(egui::Id::new("ribbon-flyout"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.set_width(ctx.screen_rect().width());
                panel(ui, app, st.tab, pal, clicked);
            });
        let area_rect = area.response.rect;
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !area_rect.contains(p) && !st.tab_rect.contains(p))
        });
        if outside || clicked.is_some() {
            st.flyout = false;
        }
    }
}

fn panel(ui: &mut egui::Ui, app: &SciApp, tab_id: &str, pal: &Palette, clicked: &mut Option<&'static str>) {
    let Some(tab) = TABS.iter().find(|t| t.id == tab_id) else {
        return;
    };
    egui::Frame::none()
        .fill(pal.panel)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .inner_margin(egui::Margin {
            left: 6.0,
            right: 6.0,
            top: 4.0,
            bottom: 2.0,
        })
        .show(ui, |ui| {
            // 不收缩宽度：面板铺满整个窗口宽度
            egui::ScrollArea::horizontal()
                .id_salt("ribbon-scroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.set_min_height(92.0);
                        for (gi, g) in tab.groups.iter().enumerate() {
                            if gi > 0 {
                                ui.separator();
                            }
                            group(ui, app, g, pal, clicked);
                        }
                    });
                });
        });
}

fn group(ui: &mut egui::Ui, app: &SciApp, g: &Group, pal: &Palette, clicked: &mut Option<&'static str>) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 1.0);
        let items = ui.horizontal_top(|ui| {
            ui.set_min_height(72.0);
            for it in g.items {
                match it {
                    Big(id) => {
                        if let Some(c) = commands::find(id) {
                            if big_button(ui, app, c, pal).clicked() {
                                *clicked = Some(c.id);
                            }
                        }
                    }
                    Smalls(ids) => {
                        ui.vertical(|ui| {
                            for id in *ids {
                                if let Some(c) = commands::find(id) {
                                    if small_button(ui, app, c, pal, clicked).clicked() {
                                        *clicked = Some(c.id);
                                    }
                                }
                            }
                        });
                    }
                    Gallery(ids) => {
                        egui::Frame::none()
                            .fill(ui.visuals().extreme_bg_color)
                            .stroke(Stroke::new(1.0_f32, pal.line))
                            .rounding(4.0)
                            .inner_margin(2.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    for id in *ids {
                                        if let Some(c) = commands::find(id) {
                                            if gallery_tile(ui, app, c, pal, clicked).clicked() {
                                                *clicked = Some(c.id);
                                            }
                                        }
                                    }
                                });
                            });
                    }
                }
            }
        });
        let w = items.response.rect.width().max(40.0);
        ui.add_space((72.0 - items.response.rect.height()).max(0.0));
        let (rect, _) = ui.allocate_exact_size(vec2(w, 16.0), Sense::hover());
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            g.label,
            FontId::proportional(crate::theme::size::SMALL),
            pal.weak,
        );
    });
}

fn button_bg(ui: &egui::Ui, rect: Rect, resp: &egui::Response, enabled: bool, checked: bool, pal: &Palette) {
    let (fill, stroke) = if enabled && resp.is_pointer_button_down_on() {
        (pal.press, Stroke::new(1.0_f32, pal.hover_line))
    } else if enabled && resp.hovered() {
        (pal.hover, Stroke::new(1.0_f32, pal.hover_line))
    } else if checked {
        (pal.sel_bg, Stroke::new(1.0_f32, pal.hover_line))
    } else {
        return;
    };
    ui.painter().rect(rect, 4.0, fill, stroke);
}

fn dim(enabled: bool, pal: &Palette) -> Color32 {
    if enabled {
        pal.on_accent
    } else {
        pal.on_accent_alpha(105)
    }
}

fn text_color(enabled: bool, pal: &Palette) -> Color32 {
    if enabled {
        pal.ink
    } else {
        pal.ink.gamma_multiply(0.42)
    }
}

fn big_button(ui: &mut egui::Ui, app: &SciApp, c: &Command, pal: &Palette) -> egui::Response {
    let enabled = (c.when)(app).is_ok();
    let checked = c.checked.is_some_and(|f| f(app));
    let galley = ui.painter().layout_no_wrap(
        c.label.to_string(),
        FontId::proportional(crate::theme::size::COMPACT),
        text_color(enabled, pal),
    );
    let size = vec2((galley.size().x + 14.0).max(58.0), 70.0);
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    button_bg(ui, rect, &resp, enabled, checked, pal);
    let icon_rect = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 22.0), vec2(30.0, 30.0));
    icons::icon(c.icon, pal.dark)
        .tint(dim(enabled, pal))
        .paint_at(ui, icon_rect);
    let text_pos = egui::pos2(rect.center().x - galley.size().x / 2.0, rect.top() + 44.0);
    ui.painter().galley(text_pos, galley, pal.ink);
    resp.on_hover_text(commands::tooltip(app, c))
}

fn small_button(
    ui: &mut egui::Ui,
    app: &SciApp,
    c: &Command,
    pal: &Palette,
    clicked: &mut Option<&'static str>,
) -> egui::Response {
    let enabled = (c.when)(app).is_ok();
    let checked = c.checked.is_some_and(|f| f(app));
    let galley = ui.painter().layout_no_wrap(
        c.label.to_string(),
        FontId::proportional(crate::theme::size::COMPACT),
        text_color(enabled, pal),
    );
    let size = vec2(galley.size().x + 34.0, 23.0);
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    button_bg(ui, rect, &resp, enabled, checked, pal);
    let icon_rect = Rect::from_min_size(egui::pos2(rect.left() + 5.0, rect.center().y - 8.5), vec2(17.0, 17.0));
    icons::icon(c.icon, pal.dark)
        .tint(dim(enabled, pal))
        .paint_at(ui, icon_rect);
    let text_pos = egui::pos2(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, pal.ink);
    // 图型命令（如“图形”选项卡里的图类型）与画廊相同：悬停显示缩略图、用法和示例
    if crate::examples::ALL.iter().any(|e| e.plot_command == c.id) {
        resp.on_hover_ui(|ui| plot_hover(ui, app, c, clicked))
    } else {
        resp.on_hover_text(commands::tooltip(app, c))
    }
}

/// 图型命令的悬停说明：较大的缩略图、用途与数据排列说明，以及“打开示例”。
fn plot_hover(ui: &mut egui::Ui, app: &SciApp, c: &Command, clicked: &mut Option<&'static str>) {
    ui.set_max_width(300.0);
    ui.strong(c.label);
    if let Some(example) = crate::examples::ALL.iter().find(|e| e.plot_command == c.id) {
        let kind = c.id.split('.').nth(1).unwrap_or("line");
        ui.add(icons::thumb(kind).fit_to_exact_size(vec2(145.0, 100.0)));
        ui.label(example.description);
        ui.separator();
        if ui.button("打开示例").clicked() {
            *clicked = Some(example.command);
        }
    }
    if let Err(reason) = (c.when)(app) {
        ui.label(reason);
    }
}

fn gallery_tile(
    ui: &mut egui::Ui,
    app: &SciApp,
    c: &Command,
    pal: &Palette,
    clicked: &mut Option<&'static str>,
) -> egui::Response {
    let enabled = (c.when)(app).is_ok();
    let label = ui.painter().layout_no_wrap(
        c.label.to_owned(),
        FontId::proportional(crate::theme::size::LABEL),
        text_color(enabled, pal),
    );
    let width = (label.size().x + 12.0).max(72.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 62.0), if enabled { Sense::click() } else { Sense::hover() });
    button_bg(ui, rect, &resp, enabled, false, pal);
    let kind = c.id.split('.').nth(1).unwrap_or("line");
    let thumb_rect = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 24.0), vec2(58.0, 40.0));
    icons::thumb(kind).tint(dim(enabled, pal)).paint_at(ui, thumb_rect);
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 9.0),
        Align2::CENTER_CENTER,
        c.label,
        FontId::proportional(crate::theme::size::LABEL),
        text_color(enabled, pal),
    );
    resp.on_hover_ui(|ui| plot_hover(ui, app, c, clicked))
}

fn icon_button(ui: &mut egui::Ui, app: &SciApp, c: &Command, dark: bool, pal: &Palette, size: f32) -> egui::Response {
    let enabled = (c.when)(app).is_ok();
    let (rect, resp) = ui.allocate_exact_size(
        vec2(size + 10.0, size + 8.0),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    button_bg(ui, rect, &resp, enabled, false, pal);
    icons::icon(c.icon, dark)
        .tint(dim(enabled, pal))
        .paint_at(ui, Rect::from_center_size(rect.center(), vec2(size, size)));
    resp.on_hover_text(commands::tooltip(app, c))
}

fn file_tab(ui: &mut egui::Ui, pal: &Palette) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        "文件".to_string(),
        FontId::proportional(crate::theme::size::BODY),
        pal.on_accent,
    );
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 28.0, 26.0), Sense::click());
    let fill = if resp.hovered() {
        pal.accent.gamma_multiply(1.15)
    } else {
        pal.accent
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, pal.on_accent);
    resp.on_hover_text("新建、打开、保存、导出")
}

fn tab_button(ui: &mut egui::Ui, t: &Tab, selected: bool, pal: &Palette) -> egui::Response {
    let color = if t.ctx.is_some() {
        pal.ctx
    } else if selected {
        pal.accent
    } else {
        pal.ink
    };
    let font = FontId::proportional(crate::theme::size::BODY);
    let galley = ui.painter().layout_no_wrap(t.label.to_string(), font, color);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 28.0, 30.0), Sense::click());
    let p = ui.painter();
    if selected {
        let r = Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.max.y + 1.0));
        p.rect(
            r,
            egui::Rounding {
                nw: 5.0,
                ne: 5.0,
                sw: 0.0,
                se: 0.0,
            },
            pal.panel,
            Stroke::new(1.0_f32, pal.line),
        );
        // 盖住面板顶边，让选中的选项卡和面板连在一起
        p.line_segment(
            [
                egui::pos2(rect.left() + 1.0, rect.bottom() + 0.5),
                egui::pos2(rect.right() - 1.0, rect.bottom() + 0.5),
            ],
            Stroke::new(2.0_f32, pal.panel),
        );
    } else if resp.hovered() {
        p.rect_filled(rect.shrink2(vec2(0.0, 2.0)), 4.0, pal.hover);
    }
    if t.ctx.is_some() {
        let bar = Rect::from_min_size(
            egui::pos2(rect.left() + 6.0, rect.top()),
            vec2(rect.width() - 12.0, 3.0),
        );
        p.rect_filled(bar, 1.5, pal.ctx);
    }
    p.galley(rect.center() - galley.size() / 2.0, galley, color);
    resp.on_hover_text(match t.ctx {
        Some(Ctx::Book) => "工作簿工具：激活工作簿时出现",
        Some(Ctx::Graph) => "图形工具：激活图形时出现",
        None => "双击折叠或展开功能区",
    })
}

/// 文件页（Backstage）。返回要打开的窗口（单击项目内容时）。
pub fn backstage(
    ctx: &egui::Context,
    app: &SciApp,
    st: &mut UiState,
    pal: &Palette,
    clicked: &mut Option<&'static str>,
) -> Option<u64> {
    let mut open = None;
    let dark = pal.dark;
    egui::SidePanel::left("backstage-nav")
        .resizable(false)
        .exact_width(210.0)
        .frame(
            egui::Frame::none()
                .fill(pal.backstage)
                .inner_margin(egui::Margin::symmetric(0.0, 12.0)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if nav_item(ui, "back", "返回", "Esc", true, pal).clicked() {
                st.backstage = false;
            }
            ui.add_space(8.0);
            for id in BACKSTAGE_NAV {
                if let Some(c) = commands::find(id) {
                    let enabled = (c.when)(app).is_ok();
                    let r = nav_item(ui, c.icon, c.label, c.keys, enabled, pal);
                    if enabled && r.clicked() {
                        st.backstage = false;
                        *clicked = Some(c.id);
                    }
                }
            }
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(ctx.style().visuals.window_fill)
                .inner_margin(egui::Margin::symmetric(36.0, 28.0)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if !app.settings.recent.is_empty() {
                    ui.heading("最近打开");
                    for (index, path) in app.settings.recent.iter().enumerate() {
                        if ui.button(path.display().to_string()).clicked() {
                            st.recent_open = Some(index);
                            *clicked = Some("file.recent");
                            st.backstage = false;
                        }
                    }
                    ui.separator();
                }
                ui.heading(RichText::new("当前项目").size(crate::theme::size::TITLE));
                ui.add_space(14.0);
                let nb = app.project.books.len();
                let ng = app.project.graphs.len();
                ui.label(
                    RichText::new(format!(
                        "项目内容 · {} 个工作簿，{} 个图形，{} 个 Note",
                        nb,
                        ng,
                        app.project.notes.len()
                    ))
                    .strong()
                    .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    for b in &app.project.books {
                        let sub = format!("工作簿 · {} 列 × {} 行", b.columns.len(), b.nrows());
                        if project_card(ui, "book", &b.name, &sub, dark, pal).clicked() {
                            open = Some(b.id);
                        }
                    }
                    for g in &app.project.graphs {
                        let sub = if g.page.layers().len() == 1 {
                            format!("图形 · {}", g.first_layer().plot_type.label())
                        } else {
                            format!("图形 · {} 个图层", g.page.layers().len())
                        };
                        if project_card(ui, "graph", &g.name, &sub, dark, pal).clicked() {
                            open = Some(g.id);
                        }
                    }
                    for note in &app.project.notes {
                        if project_card(ui, "note", &note.name, "Note · 文本与计算结果", dark, pal).clicked() {
                            open = Some(note.id);
                        }
                    }
                });
                ui.add_space(20.0);
                ui.label(RichText::new("保存位置").strong().color(pal.weak));
                ui.add_space(4.0);
                let where_ = match &app.project_path {
                    Some(p) => format!("当前项目文件：{}", p.display()),
                    None => "当前项目还没有保存。用“保存项目”存为 .scv 文件。".to_string(),
                };
                ui.label(where_);
                ui.add_space(20.0);
                ui.label(RichText::new("常用快捷键").strong().color(pal.weak));
                ui.add_space(4.0);
                egui::Grid::new("backstage-keys")
                    .num_columns(2)
                    .spacing([18.0, 4.0])
                    .show(ui, |ui| {
                        for c in commands::commands().iter().filter(|c| !c.keys.is_empty()) {
                            ui.label(RichText::new(c.keys).monospace());
                            ui.label(c.label);
                            ui.end_row();
                        }
                        ui.label(RichText::new("双击选项卡").monospace());
                        ui.label("折叠 / 展开功能区");
                        ui.end_row();
                    });
            });
        });
    open
}

fn nav_item(ui: &mut egui::Ui, icon: &str, label: &str, keys: &str, enabled: bool, pal: &Palette) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 36.0), if enabled { Sense::click() } else { Sense::hover() });
    if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, pal.on_accent_alpha(40));
    }
    let alpha = if enabled { 255 } else { 120 };
    // 图标在蓝底上统一染成白色
    icons::icon(icon, true).tint(pal.on_accent_alpha(alpha)).paint_at(
        ui,
        Rect::from_min_size(egui::pos2(rect.left() + 18.0, rect.center().y - 9.0), vec2(18.0, 18.0)),
    );
    let white = pal.on_accent_alpha(alpha);
    ui.painter().text(
        egui::pos2(rect.left() + 46.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(crate::theme::size::MENU),
        white,
    );
    if !keys.is_empty() {
        ui.painter().text(
            egui::pos2(rect.right() - 14.0, rect.center().y),
            Align2::RIGHT_CENTER,
            keys,
            FontId::proportional(crate::theme::size::SMALL),
            pal.on_accent_alpha(alpha / 2 + 40),
        );
    }
    resp
}

fn project_card(ui: &mut egui::Ui, icon: &str, name: &str, sub: &str, dark: bool, pal: &Palette) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(230.0, 52.0), Sense::click());
    let stroke = if resp.hovered() { pal.accent } else { pal.line };
    ui.painter().rect(rect, 6.0, pal.panel, Stroke::new(1.0_f32, stroke));
    icons::icon(icon, dark).paint_at(
        ui,
        Rect::from_min_size(egui::pos2(rect.left() + 12.0, rect.center().y - 11.0), vec2(22.0, 22.0)),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 44.0, rect.top() + 17.0),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(crate::theme::size::BODY),
        pal.ink,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 44.0, rect.top() + 35.0),
        Align2::LEFT_CENTER,
        sub,
        FontId::proportional(crate::theme::size::SMALL),
        pal.weak,
    );
    resp
}

/// 命令面板（Ctrl+K）：输入关键词，方向键选择，回车执行。
pub fn palette(ctx: &egui::Context, app: &SciApp, st: &mut UiState, pal: &Palette, clicked: &mut Option<&'static str>) {
    let Some(ps) = st.palette.as_mut() else { return };
    let q = ps.query.trim().to_lowercase();
    let mut list: Vec<&'static Command> = commands::commands()
        .iter()
        .filter(|c| {
            q.is_empty()
                || c.label.to_lowercase().contains(&q)
                || c.group.contains(&q)
                || c.kw.contains(&q)
                || c.id.contains(&q)
        })
        .collect();
    // 可用的命令排在前面
    list.sort_by_key(|c| (c.when)(app).is_err());
    if !list.is_empty() {
        ps.sel = ps.sel.min(list.len() - 1);
    }

    let (up, down, enter, esc) = ctx.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
        )
    });
    if esc {
        st.palette = None;
        return;
    }
    if up {
        ps.sel = ps.sel.saturating_sub(1);
    }
    if down && ps.sel + 1 < list.len() {
        ps.sel += 1;
    }
    let mut run: Option<&'static str> = None;
    if enter {
        if let Some(c) = list.get(ps.sel) {
            run = Some(c.id);
        }
    }

    let mut close = false;
    egui::Window::new("命令面板")
        .id(egui::Id::new("scicanvas-palette"))
        .constrain_to(crate::dialogs::floating_bounds(ctx))
        .resizable(false)
        .collapsible(false)
        .pivot(Align2::CENTER_TOP)
        .default_pos(egui::pos2(ctx.screen_rect().center().x, 80.0))
        .fixed_size(vec2(520.0, 440.0))
        .show(ctx, |ui| {
            let te = ui.add(
                egui::TextEdit::singleline(&mut ps.query)
                    .hint_text("输入功能名称，例如 拟合、排序、导出")
                    .desired_width(f32::INFINITY)
                    .font(FontId::proportional(crate::theme::size::INPUT)),
            );
            if ps.focus {
                te.request_focus();
                ps.focus = false;
            }
            if te.changed() {
                ps.sel = 0;
            }
            ui.separator();
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                if list.is_empty() {
                    ui.label(RichText::new("没有找到匹配的命令").color(pal.weak));
                }
                for (i, c) in list.iter().enumerate() {
                    let ok = (c.when)(app);
                    let selected = i == ps.sel;
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
                    if selected {
                        ui.painter().rect_filled(rect, 5.0, pal.accent);
                        resp.scroll_to_me(None);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, 5.0, pal.hover);
                    }
                    let fg = if selected {
                        pal.on_accent
                    } else if ok.is_ok() {
                        pal.ink
                    } else {
                        pal.ink.gamma_multiply(0.45)
                    };
                    let sub = if selected { pal.on_accent_alpha(200) } else { pal.weak };
                    let icon_tint = if ok.is_ok() {
                        pal.on_accent
                    } else {
                        pal.on_accent_alpha(110)
                    };
                    icons::icon(c.icon, pal.dark || selected).tint(icon_tint).paint_at(
                        ui,
                        Rect::from_min_size(egui::pos2(rect.left() + 10.0, rect.center().y - 9.0), vec2(18.0, 18.0)),
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 38.0, rect.top() + 12.0),
                        Align2::LEFT_CENTER,
                        c.label,
                        FontId::proportional(crate::theme::size::MENU),
                        fg,
                    );
                    let detail = match ok {
                        Ok(()) => c.group.to_string(),
                        Err(why) => format!("{} · {}", c.group, why),
                    };
                    ui.painter().text(
                        egui::pos2(rect.left() + 38.0, rect.top() + 28.0),
                        Align2::LEFT_CENTER,
                        detail,
                        FontId::proportional(crate::theme::size::SMALL),
                        sub,
                    );
                    if !c.keys.is_empty() {
                        ui.painter().text(
                            egui::pos2(rect.right() - 10.0, rect.center().y),
                            Align2::RIGHT_CENTER,
                            c.keys,
                            FontId::monospace(crate::theme::size::SMALL),
                            sub,
                        );
                    }
                    if resp.clicked() {
                        run = Some(c.id);
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new("↑ ↓ 选择 · 回车执行 · Esc 关闭").small().color(pal.weak));
        });
    // 点到面板外面也关闭
    if ctx.input(|i| i.pointer.any_pressed()) {
        let inside = ctx
            .layer_id_at(ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default())
            .is_some_and(|l| l.id == egui::Id::new("scicanvas-palette"));
        if !inside {
            close = true;
        }
    }
    if let Some(id) = run {
        *clicked = Some(id);
        close = true;
    }
    if close {
        st.palette = None;
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;
    #[test]
    fn pdf_export_is_registered_without_growing_the_ribbon_rows() {
        let command = commands::find("graph.export_pdf").unwrap();
        assert!(!command.modifies);
        assert!(BACKSTAGE_NAV.contains(&command.id));
        let mut entries = 0;
        for tab in TABS {
            for group in tab.groups {
                for item in group.items {
                    if let Smalls(ids) = item {
                        if ids.contains(&command.id) {
                            assert!(ids.len() <= 3, "{} / {}", tab.id, group.label);
                            entries += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(entries, 2, "home and graph export entry points");
    }
}
