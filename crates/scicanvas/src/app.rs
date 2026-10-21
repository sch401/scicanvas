//! 应用状态与界面调度、图形属性和分析操作。
//! 项目管理与子窗口位于 workspace，工作表显示与编辑位于 worksheet。

mod notes;
mod worksheet;
mod workspace;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use eframe::egui::{self, RichText};
use egui_extras::{Column as TCol, TableBuilder};
use scicanvas_core::analysis::{NormalizeMethod, SmoothMethod};
use scicanvas_core::history::History;
use scicanvas_core::ops::Source;
use scicanvas_core::{csv, fmt_num, round_sig, svg, PlotType, Project, Role};

use crate::commands;
use crate::dialogs::{self, Dialog};
use crate::mdi::{self, Geom, WinState};
use crate::plot;
use crate::ribbon::{self, Ctx, UiState};
use crate::sheet::{Cell, Drag, Edit, Range, SheetState};
use crate::theme::{self, Palette};

const META_ROWS: [&str; 3] = ["长名称", "单位", "注释"];

#[derive(Clone, Copy)]
struct ProjectDrag(u64);

#[derive(Clone, PartialEq)]
enum BookPreviewEdit {
    Properties(u64, dialogs::ColumnEdit),
    Categories(u64, String, Vec<String>),
    Sort(u64, Vec<(String, bool)>, Option<Vec<String>>),
}

struct BookPreview {
    edit: BookPreviewEdit,
    result: Result<scicanvas_core::Book, String>,
}

/// 列标题右键菜单里的操作。
#[derive(Clone, Copy)]
enum ColAction {
    Props,
    SetRole(Role),
    SetDataKind(scicanvas_core::DataKind),
    InsertRight,
    CategoryOrder,
    Move(scicanvas_core::ColumnMove),
    Clear,
    Delete,
    Plot(PlotType),
    AddToGraph(u64),
}

pub struct SciApp {
    analysis_curves: HashMap<(u64, usize), Vec<scicanvas_core::plotspec::DataRef>>,
    pub(crate) project: Project,
    pub(crate) history: History,
    history_group: Option<egui::Id>,
    last_history_group: Option<egui::Id>,
    /// 每个工作簿/图形的窗口是否显示。
    pub(crate) open: HashMap<u64, bool>,
    /// 每个工作簿里选中的列（保持点击顺序）。
    pub(crate) selection: HashMap<u64, Vec<String>>,
    /// 当前激活的窗口。
    pub(crate) active: Option<u64>,
    pub(crate) explorer_target: Option<u64>,
    explorer_selection: Vec<u64>,
    explorer_anchor: Option<u64>,
    explorer_visible: Vec<u64>,
    explorer_sort: workspace::ExplorerSort,
    /// 处于编辑状态的 Note（其余显示 Markdown 预览）。
    note_editing: HashSet<u64>,
    pub(crate) folder_destination: Option<u64>,
    pub(crate) project_name: String,
    explorer_click: Option<(u64, f64)>,
    /// Object ID and whether the inline name field needs initial focus.
    explorer_rename: Option<(u64, bool)>,
    /// Whether the last pointer press landed in the project explorer panel.
    explorer_focus: bool,
    /// 正在编辑属性的图形。
    pub(crate) props_for: Option<u64>,
    active_layers: HashMap<u64, usize>,
    pub(crate) plot_draft: Option<scicanvas_core::plotspec::PlotSpec>,
    pub(crate) status: String,
    pub(crate) show_explorer: bool,
    pub(crate) show_help: bool,
    pub(crate) confirm_delete: Option<u64>,
    /// 每个工作簿的单元格选区和编辑状态。
    pub(crate) sheets: HashMap<u64, SheetState>,
    /// 命令要复制到剪贴板的文字（命令拿不到 egui 上下文，在帧末尾交给 egui）。
    pub(crate) pending_copy: Option<String>,
    /// 下一帧要提到最前面的窗口。
    pub(crate) raise: Option<u64>,
    pub(crate) project_path: Option<PathBuf>,
    /// 当前主题的配色，每帧按 egui 的深浅色更新。
    pub(crate) pal: Palette,
    pub(crate) theme_pref: egui::ThemePreference,
    /// 命令改了主题后，下一帧交给 egui。
    pub(crate) pending_theme: Option<egui::ThemePreference>,
    pub(crate) pending_window_reset: bool,
    pending_repaint: bool,
    /// 功能区、文件页、命令面板的界面状态。
    pub(crate) ui: UiState,
    /// 当前打开的分析对话框。
    pub(crate) dialog: Option<Dialog>,
    book_preview: Option<BookPreview>,
    pub(crate) quit: bool,
    /// 子窗口的位置、大小和最小化 / 最大化状态。
    pub(crate) geoms: HashMap<u64, Geom>,
    /// 上一帧的工作区矩形（排列窗口时用）。
    pub(crate) ws: egui::Rect,
    /// 贴靠后等待用户选择填空窗口的贴靠助手。
    pub(crate) snap_assist: Option<crate::assist::SnapAssist>,
}

/// 状态栏里描述选区的文字。
fn range_status(book: &scicanvas_core::Book, r: &Range) -> String {
    format!(
        "已选中 {}（{} 行 × {} 列）— Ctrl+C 复制，Delete 清除，点绘图按钮按选区作图",
        r.label(book),
        r.rows(),
        r.cols()
    )
}

fn win_id(id: u64) -> egui::Id {
    egui::Id::new(("scicanvas-window", id))
}

fn header_field(column: &mut scicanvas_core::Column, row: usize) -> &mut String {
    match row {
        0 => &mut column.long_name,
        1 => &mut column.units,
        _ => &mut column.comment,
    }
}

/// 同一文字框焦点周期或一次鼠标拖动共享一个历史记录。
fn edit_group(response: &egui::Response, group: &mut Option<egui::Id>) {
    let text_focus = response.has_focus() && egui::TextEdit::load_state(&response.ctx, response.id).is_some();
    if text_focus || response.is_pointer_button_down_on() || response.dragged() || response.drag_stopped() {
        *group = Some(response.id);
    }
}

impl SciApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::setup(&cc.egui_ctx);
        theme::install(&cc.egui_ctx);
        // 功能区图标是 SVG，需要 egui_extras 的图片加载器
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self::initial(theme::palette(cc.egui_ctx.theme()))
    }

    fn initial(pal: Palette) -> Self {
        let mut app = SciApp {
            analysis_curves: HashMap::new(),
            book_preview: None,
            project: Project::default(),
            history: History::default(),
            history_group: None,
            last_history_group: None,
            open: HashMap::new(),
            selection: HashMap::new(),
            active: None,
            explorer_target: None,
            explorer_selection: Vec::new(),
            explorer_anchor: None,
            explorer_visible: Vec::new(),
            explorer_sort: workspace::ExplorerSort::default(),
            note_editing: HashSet::new(),
            folder_destination: None,
            project_name: String::new(),
            explorer_click: None,
            explorer_rename: None,
            explorer_focus: false,
            props_for: None,
            active_layers: HashMap::new(),
            plot_draft: None,
            status: String::new(),
            show_explorer: true,
            show_help: false,
            confirm_delete: None,
            sheets: HashMap::new(),
            pending_copy: None,
            raise: None,
            project_path: None,
            pal,
            theme_pref: egui::ThemePreference::System,
            pending_theme: None,
            pending_window_reset: false,
            pending_repaint: false,
            ui: UiState::default(),
            dialog: None,
            quit: false,
            geoms: HashMap::new(),
            snap_assist: None,
            ws: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 650.0)),
        };
        app.load_project(Project::sample(), None);
        app.status = "就绪 — 单击列标题选列，再点功能区里的绘图按钮；Ctrl+K 搜索任何命令，F1 查看快速上手".into();
        app
    }

    fn load_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.book_preview = None;
        self.analysis_curves.clear();
        self.active_layers.clear();
        self.plot_draft = None;
        self.history = History::default();
        self.history_group = None;
        self.last_history_group = None;
        self.open.clear();
        self.selection.clear();
        self.geoms.clear();
        self.snap_assist = None;
        for b in &project.books {
            self.open.insert(b.id, true);
        }
        for g in &project.graphs {
            self.open.insert(g.id, true);
        }
        for note in &project.notes {
            self.open.insert(note.id, true);
        }
        self.active = project
            .graphs
            .last()
            .map(|g| g.id)
            .or(project.books.last().map(|b| b.id))
            .or(project.notes.last().map(|n| n.id));
        self.project = project;
        self.explorer_target = self.active;
        self.explorer_selection = self.active.into_iter().collect();
        self.explorer_anchor = self.active;
        self.explorer_visible.clear();
        self.folder_destination = None;
        self.project_name.clear();
        self.explorer_click = None;
        self.explorer_rename = None;
        self.project_path = path;
        self.props_for = None;
        self.sheets.clear();
    }

    fn active_book(&mut self) -> Option<u64> {
        match self.active {
            Some(id) if self.project.book(id).is_some() => Some(id),
            _ => {
                self.status = "请先单击一个工作簿窗口，使它成为当前窗口".into();
                None
            }
        }
    }

    /// 当前图形；当前窗口不是图形时取最后一个打开的图形。
    fn target_graph(&mut self) -> Option<u64> {
        if let Some(id) = self.active {
            if self.project.graph(id).is_some() {
                return Some(id);
            }
        }
        let found = self
            .project
            .graphs
            .iter()
            .rev()
            .find(|g| self.open.get(&g.id).copied().unwrap_or(false))
            .map(|g| g.id);
        if found.is_none() {
            self.status = "请先打开一个图形窗口".into();
        }
        found
    }

    pub(crate) fn plot(&mut self, t: PlotType) {
        if let Some(id) = self.active {
            let index = self.layer_index(id);
            if let Some(g) = self.project.graph_mut(id) {
                self.status = match g.page.layers_mut()[index].set_plot_type(t) {
                    Ok(()) => format!("{} 已改为{}", g.name, t.label()),
                    Err(e) => e,
                };
                return;
            }
        }
        let Some(bid) = self.active_book() else { return };
        let (sel, rows) = self.picked(bid);
        match self.project.plot_from_selection(bid, &sel, t) {
            Ok((gid, msg)) => {
                if let Some(g) = self.project.graph_mut(gid) {
                    g.first_layer_mut().rows = rows;
                }
                self.show_item(gid);
                self.status = match rows {
                    Some((a, b)) => format!("{}（第 {}–{} 行）", msg, a + 1, b + 1),
                    None => msg,
                };
            }
            Err(e) => self.status = e,
        }
    }

    /// 用当前工作簿选中的 Y 列画多面板折线图，选区行范围对每个面板都生效。
    pub(crate) fn plot_panels(&mut self, panels: scicanvas_core::Panels) {
        let Some(bid) = self.active_book() else { return };
        let (sel, rows) = self.picked(bid);
        match self.project.plot_panels(bid, &sel, PlotType::Line, panels) {
            Ok((gid, msg)) => {
                if let Some(g) = self.project.graph_mut(gid) {
                    for layer in g.page.layers_mut() {
                        layer.rows = rows;
                    }
                }
                self.show_item(gid);
                self.status = msg;
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn import_csv(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("导入数据")
            .add_filter("数据文件", &["csv", "txt", "tsv", "dat", "xlsx", "xlsm", "xls"])
            .add_filter("Excel 工作簿", &["xlsx", "xlsm", "xls"])
            .add_filter("所有文件", &["*"])
            .pick_file()
        else {
            return;
        };
        self.import_path(&path);
    }

    /// 把一个数据文件导入成新工作簿，放在当前文件夹里。Excel 工作簿的每张非空工作表各成一个工作簿。
    fn import_path(&mut self, path: &std::path::Path) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status = format!("无法读取 {}：{}", path.display(), e);
                return;
            }
        };
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Book".into());
        let excel = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ["xlsx", "xlsm", "xls"].iter().any(|x| e.eq_ignore_ascii_case(x)));
        // Excel 读成行表；CSV 直接由文本建表（流式写入类型化列，大文件占内存少）
        enum Table {
            Rows(Vec<Vec<String>>),
            Text(String),
        }
        let tables: Vec<(String, Table)> = if excel {
            match scicanvas_core::xlsx::read(&bytes) {
                Ok(sheets) => {
                    let sheets: Vec<_> = sheets.into_iter().filter(|s| !s.rows.is_empty()).collect();
                    let single = sheets.len() == 1;
                    sheets
                        .into_iter()
                        .map(|s| {
                            let name = if single {
                                stem.clone()
                            } else {
                                format!("{stem}-{}", s.name)
                            };
                            (name, Table::Rows(s.rows))
                        })
                        .collect()
                }
                Err(e) => {
                    self.status = format!("无法导入 {}：{e}", path.display());
                    return;
                }
            }
        } else {
            vec![(stem, Table::Text(csv::decode(&bytes)))]
        };
        let tables: Vec<_> = tables
            .into_iter()
            .filter(|(_, t)| match t {
                Table::Rows(rows) => !rows.is_empty(),
                Table::Text(text) => !text.trim().is_empty(),
            })
            .collect();
        if tables.is_empty() {
            self.status = "文件里没有数据".into();
            return;
        }
        let mut done = vec![];
        let mut last = None;
        for (base, table) in tables {
            let name = if self.project.books.iter().any(|b| b.name == base) {
                self.project.unique_name(&format!("{}_", base))
            } else {
                base
            };
            let id = self.project.alloc_id();
            let book = match table {
                Table::Rows(rows) => csv::book_from_table(id, &name, rows),
                Table::Text(text) => csv::book_from_text(id, &name, &text),
            };
            done.push(format!("{}（{} 行 × {} 列）", name, book.nrows(), book.columns.len()));
            self.project.books.push(book);
            let parent = self.current_folder();
            self.project.move_project_item(id, parent).unwrap();
            last = Some(id);
        }
        if let Some(id) = last {
            self.show_item(id);
        }
        self.status = format!("已导入 {}：{}", path.display(), done.join("、"));
    }

    pub(crate) fn open_project(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("打开项目")
            .add_filter("SciCanvas 项目", &["scv", "scproj"])
            .pick_file()
        else {
            return;
        };
        self.open_path(&path);
    }

    fn open_path(&mut self, path: &std::path::Path) {
        match scicanvas_core::project_file::load(path) {
            Ok(p) => {
                self.load_project(p, Some(path.to_path_buf()));
                self.status = format!("已打开 {}", path.display());
            }
            Err(e) => self.status = format!("无法打开项目：{}", e),
        }
    }

    fn is_project_file(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("scv") || e.eq_ignore_ascii_case("scproj"))
    }

    /// 拖放到窗口的文件：项目文件直接打开（只打开第一个），其余文件逐个导入为工作簿，作为一步撤销。
    fn open_dropped(&mut self, paths: Vec<PathBuf>) {
        if let Some(project) = paths.iter().find(|p| Self::is_project_file(p)) {
            self.open_path(project);
            if paths.len() > 1 {
                self.status.push_str("（一次只打开一个项目，其余文件已忽略）");
            }
            return;
        }
        if paths.is_empty() {
            return;
        }
        self.finish_edit();
        self.begin_edit("导入数据");
        let mut done = Vec::new();
        for path in &paths {
            self.import_path(path);
            done.push(std::mem::take(&mut self.status));
        }
        self.history.changed();
        self.finish_edit();
        self.status = done.join("；");
    }

    /// 处理拖入窗口的文件，拖动经过时显示提示层。
    fn dropped_files(&mut self, ctx: &egui::Context) {
        let (hovered, dropped) = ctx.input(|i| {
            (
                i.raw
                    .hovered_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect::<Vec<_>>(),
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect::<Vec<_>>(),
            )
        });
        if !hovered.is_empty() {
            let text = if hovered.iter().any(|p| Self::is_project_file(p)) {
                "松开以打开项目".to_string()
            } else {
                format!("松开以导入 {} 个数据文件", hovered.len())
            };
            let screen = ctx.screen_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop-hint")));
            painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha(110));
            painter.rect_stroke(screen.shrink(10.0), 8.0, egui::Stroke::new(3.0_f32, self.pal.accent));
            painter.text(
                screen.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(24.0),
                egui::Color32::WHITE,
            );
        }
        if !dropped.is_empty() {
            self.open_dropped(dropped);
        }
    }

    pub(crate) fn save_project(&mut self, save_as: bool) {
        let path = match (&self.project_path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let stem = self
                    .project_path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map_or_else(|| "项目".into(), |s| s.to_string_lossy());
                let Some(p) = rfd::FileDialog::new()
                    .set_title("保存项目")
                    .add_filter("SciCanvas 项目", &["scv"])
                    .add_filter("旧版 JSON 项目", &["scproj"])
                    .set_file_name(format!("{stem}.scv"))
                    .save_file()
                else {
                    return;
                };
                p
            }
        };
        let result = scicanvas_core::project_file::save(&path, &self.project);
        match result {
            Ok(()) => {
                self.status = format!("已保存到 {}", path.display());
                self.project_path = Some(path);
            }
            Err(e) => self.status = format!("保存失败：{}", e),
        }
    }

    pub(crate) fn export_svg(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(g) = self.project.graph(gid).cloned() else {
            return;
        };
        let content = svg::render_page(&g, &self.project);
        let Some(path) = rfd::FileDialog::new()
            .set_title("导出图形")
            .add_filter("SVG 矢量图", &["svg"])
            .set_file_name(format!("{}.svg", g.name))
            .save_file()
        else {
            return;
        };
        self.status = match std::fs::write(&path, content) {
            Ok(()) => format!("已导出 {}", path.display()),
            Err(e) => format!("导出失败：{}", e),
        };
    }

    pub(crate) fn export_png_dialog(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(g) = self.project.graph(gid) else { return };
        self.dialog = Some(Dialog::Png {
            dpi: 600,
            page_mm: (g.page.width_mm, g.page.height_mm),
        });
    }

    pub(crate) fn export_png(&mut self, dpi: u32) {
        let Some(gid) = self.target_graph() else { return };
        let Some(g) = self.project.graph(gid).cloned() else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title("导出 PNG")
            .add_filter("PNG 图像", &["png"])
            .set_file_name(format!("{}.png", g.name))
            .save_file()
        else {
            return;
        };
        let result = crate::export::rasterize(&svg::render_page(&g, &self.project), dpi).and_then(|pixmap| {
            let bytes = crate::export::png_bytes(&pixmap, dpi)?;
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            Ok((pixmap.width(), pixmap.height()))
        });
        self.status = match result {
            Ok((w, h)) => format!("已导出 {}（{w} × {h} 像素，{dpi} dpi）", path.display()),
            Err(e) => format!("导出失败：{e}"),
        };
    }

    /// 当前工作簿导出为 CSV（UTF-8 BOM，Excel 可直接打开，表头含单位，可重新导入）。
    pub(crate) fn export_csv(&mut self) {
        let Some(id) = self.active_book() else { return };
        let Some(book) = self.project.book(id) else { return };
        let content = scicanvas_core::project_file::book_csv(book);
        let (name, rows, cols) = (book.name.clone(), book.nrows(), book.columns.len());
        let Some(path) = rfd::FileDialog::new()
            .set_title("导出 CSV")
            .add_filter("CSV 表格", &["csv"])
            .set_file_name(format!("{name}.csv"))
            .save_file()
        else {
            return;
        };
        self.status = match std::fs::write(&path, content) {
            Ok(()) => format!("已导出 {}（{rows} 行 × {cols} 列）", path.display()),
            Err(e) => format!("导出失败：{e}"),
        };
    }

    /// 按 300 dpi 把当前图形复制为位图，可直接粘贴到 Word、PPT。
    pub(crate) fn copy_graph(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(g) = self.project.graph(gid).cloned() else {
            return;
        };
        let dpi = crate::export::CLIPBOARD_DPI;
        let result = crate::export::rasterize(&svg::render_page(&g, &self.project), dpi)
            .and_then(|pixmap| crate::export::copy_to_clipboard(&pixmap));
        self.status = match result {
            Ok(()) => format!("已复制 {}（{dpi} dpi 位图），可粘贴到 Word、PPT", g.name),
            Err(e) => format!("复制失败：{e}"),
        };
    }

    // ---------------------------------------------------------------- 命令用到的查询与操作

    /// 执行一条命令（功能区、快捷键、命令面板都走这里）。不能执行时在状态栏说明原因。
    pub(crate) fn run_command(&mut self, id: &str) {
        let Some(c) = commands::find(id) else { return };
        self.pending_repaint = true;
        self.finish_edit();
        match (c.when)(self) {
            Ok(()) => {
                let label = if matches!(id, "col.delete" | "col.clear") {
                    let keys = self
                        .active_book_id()
                        .map(|bid| self.selected_keys(bid))
                        .unwrap_or_default();
                    format!("{} {}", c.label, keys.join(", "))
                } else {
                    c.label.to_string()
                };
                if c.modifies {
                    self.begin_edit(&label);
                }
                (c.run)(self);
                if c.modifies {
                    self.history.changed();
                    self.finish_edit();
                }
            }
            Err(why) => self.status = why.to_string(),
        }
    }

    fn begin_edit(&mut self, label: &str) {
        self.pending_repaint = true;
        self.book_preview = None;
        self.history.begin(&self.project, label);
    }

    fn finish_edit(&mut self) {
        self.recalc_formulas();
        self.history.commit(&self.project);
        self.last_history_group = None;
    }

    /// “重算”命令：公式列和全部分析结果列（含手动的）按依赖顺序重算，一步撤销。
    pub(crate) fn recalc_all(&mut self) {
        let problems = self.project.recalc_computed(true);
        self.status = if problems.is_empty() {
            "已重算公式列和分析结果".into()
        } else {
            format!("重算出错：{}", problems.join("；"))
        };
    }

    /// 有改动时按保存的列公式重算（并入同一步撤销）；出错时在状态栏说明。
    fn recalc_formulas(&mut self) {
        if self.history.has_changes() {
            let problems = self.project.recalc_computed(false);
            if !problems.is_empty() {
                self.status = format!("公式重算出错：{}", problems.join("；"));
            }
        }
    }

    fn finish_frame_edit(&mut self, group: Option<egui::Id>) {
        // 连续输入时每帧重算，结果即时更新
        self.recalc_formulas();
        if group.is_none() {
            self.finish_edit();
        } else {
            self.last_history_group = group;
        }
    }

    pub(crate) fn undo(&mut self) {
        self.finish_edit();
        let label = self.history.undo_label().unwrap_or("").to_string();
        if let Some(project) = self.history.undo(&self.project) {
            self.restore_history(project);
            self.status = format!("已撤销：{label}");
        }
    }

    pub(crate) fn redo(&mut self) {
        self.finish_edit();
        let label = self.history.redo_label().unwrap_or("").to_string();
        if let Some(project) = self.history.redo(&self.project) {
            self.restore_history(project);
            self.status = format!("已重做：{label}");
        }
    }

    fn restore_history(&mut self, project: Project) {
        self.book_preview = None;
        self.project = project;
        self.analysis_curves.retain(|(graph, index), curves| {
            let Some(layer) = self.project.graph(*graph).and_then(|g| g.page.layers().get(*index)) else {
                return false;
            };
            curves.retain(|r| {
                layer
                    .y_keys
                    .iter()
                    .any(|key| scicanvas_core::plotspec::DataRef::from_key(layer.book_id, key).as_ref() == Some(r))
            });
            true
        });
        self.active_layers.retain(|id, index| {
            if let Some(graph) = self.project.graph(*id) {
                *index = (*index).min(graph.page.layers().len() - 1);
                true
            } else {
                false
            }
        });
        let ids = self.all_ids();
        self.open.retain(|id, _| ids.contains(id));
        self.geoms.retain(|id, _| ids.contains(id));
        self.selection.retain(|id, keys| {
            if let Some(book) = self.project.book(*id) {
                keys.retain(|k| book.column(k).is_some());
                true
            } else {
                false
            }
        });
        self.sheets.retain(|id, state| {
            if let Some(book) = self.project.book(*id) {
                state.edit = None;
                state.drag = None;
                if book.columns.is_empty() {
                    state.range = None;
                } else if let Some(range) = state.range.as_mut() {
                    range.clamp(book.columns.len());
                }
                true
            } else {
                false
            }
        });
        for id in &ids {
            self.open.entry(*id).or_insert(true);
        }
        if !self.active.is_some_and(|id| ids.contains(&id)) {
            self.active = ids.last().copied();
        }
        if self.props_for.is_some_and(|id| self.project.graph(id).is_none()) {
            self.props_for = None;
        }
        self.confirm_delete = None;
        self.explorer_selection.retain(|id| self.project.name_of(*id).is_some());
        self.explorer_click = None;
        self.explorer_rename = None;
        self.project_name = self
            .project_target()
            .and_then(|id| self.project.name_of(id))
            .unwrap_or("")
            .to_string();
        if self
            .explorer_target
            .is_some_and(|id| self.project.name_of(id).is_none())
        {
            self.explorer_target = self.active;
        }
        if self
            .folder_destination
            .is_some_and(|id| self.project.folder(id).is_none())
        {
            self.folder_destination = None;
        }
        self.dialog = None;
        self.history_group = None;
        self.last_history_group = None;
    }

    pub(crate) fn active_book_id(&self) -> Option<u64> {
        self.active.filter(|id| self.project.book(*id).is_some())
    }

    pub(crate) fn active_graph_id(&self) -> Option<u64> {
        self.active.filter(|id| self.project.graph(*id).is_some())
    }

    pub(crate) fn layer_index(&self, id: u64) -> usize {
        self.project
            .graph(id)
            .map(|g| {
                self.active_layers
                    .get(&id)
                    .copied()
                    .unwrap_or(0)
                    .min(g.page.layers().len() - 1)
            })
            .unwrap_or(0)
    }

    pub(crate) fn active_layer(&self) -> Option<&scicanvas_core::page::Layer> {
        let id = self.active_graph_id()?;
        self.project.graph(id)?.page.layers().get(self.layer_index(id))
    }

    fn select_layer(&mut self, id: u64, index: usize) {
        if self.project.graph(id).is_some_and(|g| index < g.page.layers().len()) {
            self.active_layers.insert(id, index);
            self.active = Some(id);
        }
    }

    /// 当前窗口是工作簿还是图形，决定显示哪个上下文选项卡。
    pub(crate) fn active_ctx(&self) -> Option<Ctx> {
        if self.active_book_id().is_some() {
            Some(Ctx::Book)
        } else if self.active_graph_id().is_some() {
            Some(Ctx::Graph)
        } else {
            None
        }
    }

    pub(crate) fn any_open_graph(&self) -> Option<u64> {
        self.active_graph_id().or_else(|| {
            self.project
                .graphs
                .iter()
                .rev()
                .find(|g| self.open.get(&g.id).copied().unwrap_or(false))
                .map(|g| g.id)
        })
    }

    /// 标题栏里显示的当前窗口名称。
    pub(crate) fn doc_title(&self) -> String {
        match self.active.and_then(|id| self.project.name_of(id).map(|n| (id, n))) {
            Some((id, n)) if self.project.book(id).is_some() => format!("{} · 工作簿", n),
            Some((id, n)) if self.project.note(id).is_some() => format!("{} · Note", n),
            Some((_, n)) => format!("{} · 图形", n),
            None => "未选中窗口".to_string(),
        }
    }

    /// 分析的输入：当前图形里的数据曲线，或当前工作簿里选中的列。
    pub(crate) fn analysis_source(&self) -> Option<Source> {
        if let Some(g) = self.active_graph_id() {
            let index = self.layer_index(g);
            if let Some(curves) = self.analysis_curves.get(&(g, index)) {
                return Some(Source::Curves {
                    graph_id: g,
                    index,
                    curves: curves.clone(),
                });
            }
            return self
                .active_layer()
                .and_then(|layer| layer.book_id)
                .map(|_| Source::Layer {
                    graph_id: g,
                    index: self.layer_index(g),
                });
        }
        self.active_book_id().map(|id| {
            let (selected, rows) = self.picked(id);
            Source::Book { id, selected, rows }
        })
    }

    pub(crate) fn reset_project(&mut self) {
        self.load_project(Project::sample(), None);
        self.status = "已恢复示例项目".into();
    }

    pub(crate) fn open_layer_contents(&mut self) {
        let Some(graph) = self.any_open_graph() else { return };
        let index = self.layer_index(graph);
        let layer = self.project.graph(graph).unwrap().page.layers()[index].clone();
        let active = layer.y_keys.first().cloned();
        self.dialog = Some(Dialog::LayerContents(Box::new(dialogs::LayerContents {
            graph,
            index,
            before: layer.clone(),
            draft: layer,
            selected: Vec::new(),
            active,
            error: None,
        })));
    }

    fn apply_layer_contents(&mut self, edit: &dialogs::LayerContents) -> Result<(), String> {
        let layer = self
            .project
            .graph(edit.graph)
            .and_then(|g| g.page.layers().get(edit.index))
            .ok_or("找不到目标图层")?;
        if layer != &edit.before {
            return Err("图层已更新，请重新打开图层内容".into());
        }
        if !edit.draft.y_keys.is_empty() {
            let book = edit
                .draft
                .book_id
                .and_then(|id| self.project.book(id))
                .ok_or("找不到来源表")?;
            edit.draft.validate_in(book, &|id| self.project.book(id))?;
        }
        if layer == &edit.draft {
            return Ok(());
        }
        self.finish_edit();
        self.begin_edit("图层内容");
        self.project.graph_mut(edit.graph).unwrap().page.layers_mut()[edit.index] = edit.draft.clone();
        self.history.changed();
        self.finish_edit();
        if let Some(selected) = self.analysis_curves.get_mut(&(edit.graph, edit.index)) {
            selected.retain(|reference| {
                edit.draft.y_keys.iter().any(|key| {
                    scicanvas_core::plotspec::DataRef::from_key(edit.draft.book_id, key).as_ref() == Some(reference)
                })
            });
        }
        self.show_item(edit.graph);
        self.status = "图层内容已更新".into();
        Ok(())
    }

    pub(crate) fn open_props(&mut self) {
        if let Some(g) = self.any_open_graph() {
            self.props_for = Some(g);
        }
    }

    pub(crate) fn add_layer(&mut self) {
        let Some(id) = self.active_graph_id() else { return };
        let index = self.layer_index(id);
        let graph = self.project.graph_mut(id).unwrap();
        let mut layer = graph.page.layers()[index].clone();
        layer.placement = None;
        graph.page.add_layer(layer);
        let n = graph.page.layers().len();
        graph.page.columns = (n as f64).sqrt().ceil() as usize;
        self.props_for = Some(id);
        self.active_layers.insert(id, n - 1);
        self.status = "已添加图层，可在绘图属性中选择各图层的数据和样式".into();
    }

    pub(crate) fn open_plot_builder(&mut self) {
        if let Some(id) = self.active_book_id() {
            let (keys, rows) = self.picked(id);
            match scicanvas_core::plotspec::PlotSpec::from_selection(
                self.project.book(id).unwrap(),
                &keys,
                PlotType::LineSymbol,
            ) {
                Ok(mut spec) => {
                    spec.rows = rows;
                    self.plot_draft = Some(spec);
                }
                Err(e) => self.status = e,
            }
        } else if let Some(id) = self.active_graph_id() {
            self.plot_draft = Some(
                self.project.graph(id).unwrap().page.layers()[self.layer_index(id)]
                    .spec
                    .clone(),
            );
        }
    }

    pub(crate) fn apply_plot_spec(&mut self) {
        let Some(spec) = self.plot_draft.clone() else { return };
        match self.project.plot_spec(spec) {
            Ok(id) => {
                self.show_item(id);
                self.plot_draft = None;
                self.status = "已按作图描述创建图形".into();
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn remove_layer(&mut self) {
        let Some(id) = self.active_graph_id() else { return };
        let index = self.layer_index(id);
        let graph = self.project.graph_mut(id).unwrap();
        match graph.page.remove_layer(index) {
            Ok(_) => {
                self.analysis_curves.retain(|(graph_id, _), _| *graph_id != id);
                self.active_layers.insert(id, index.min(graph.page.layers().len() - 1));
                self.status = "已删除图层".into();
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn toggle_graph(&mut self, field: fn(&mut scicanvas_core::page::Layer) -> &mut bool, label: &str) {
        let Some(gid) = self.active_graph_id() else { return };
        let index = self.layer_index(gid);
        if let Some(g) = self.project.graph_mut(gid) {
            let v = field(&mut g.page.layers_mut()[index]);
            *v = !*v;
            self.status = format!("{}：{}", label, if *v { "开" } else { "关" });
        }
    }

    pub(crate) fn set_theme(&mut self, pref: egui::ThemePreference) {
        self.theme_pref = pref;
        self.pending_theme = Some(pref);
        self.status = match pref {
            egui::ThemePreference::Light => "已切换到浅色主题",
            egui::ThemePreference::Dark => "已切换到深色主题",
            egui::ThemePreference::System => "主题跟随系统",
        }
        .into();
    }

    /// 结果写进工作簿后，确保相关窗口可见。
    fn after_analysis(&mut self, graph: Option<u64>) {
        if let Some(g) = graph {
            if !self.open.get(&g).copied().unwrap_or(false) {
                self.show_item(g);
            }
        }
    }

    pub(crate) fn fit(&mut self, order: usize) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.fit_poly(&src, order) {
            Ok((gid, reports)) => {
                self.after_analysis(Some(gid));
                self.status = format!("已完成 {} 条数据的{}", reports.len(), reports[0].kind);
                self.dialog = Some(Dialog::FitReport(reports));
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_nlfit_dialog(&mut self) {
        self.dialog = Some(Dialog::NlFit {
            model: scicanvas_core::nlfit::Model::Gauss,
            limits: [(false, 0.0, false, 0.0, false, 1.0); scicanvas_core::nlfit::MAX_PARAMS],
            constraints: String::new(),
        });
    }

    fn nlfit(
        &mut self,
        model: scicanvas_core::nlfit::Model,
        limits: &[scicanvas_core::nlfit::ParamLimits],
        constraints: &[scicanvas_core::nlfit::LinearConstraint],
    ) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.nlfit(&src, model, limits, constraints) {
            Ok((gid, reports)) => {
                self.after_analysis(Some(gid));
                let unconverged = reports.iter().filter(|r| !r.fit.converged).count();
                self.status = if unconverged == 0 {
                    format!("已完成 {} 条数据的 {} 拟合", reports.len(), model.name())
                } else {
                    format!("{} 拟合有 {unconverged} 条未收敛，请查看结果", model.name())
                };
                self.dialog = Some(Dialog::NlFitReport(reports));
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_poly_dialog(&mut self) {
        self.dialog = Some(Dialog::Poly { order: 2 });
    }

    /// IFFT：选中的前两列作实部、虚部；没选列时找 FFT 结果表里的“实部”“虚部”。
    pub(crate) fn open_ifft_dialog(&mut self) {
        let Some(id) = self.active_book() else { return };
        let Some(book) = self.project.book(id) else { return };
        let named = |name: &str| book.columns.iter().find(|c| c.long_name == name).map(|c| c.key.clone());
        let sel = self.selected_keys(id);
        let (re, im) = if sel.is_empty() {
            (named("实部"), named("虚部"))
        } else {
            (sel.first().cloned(), sel.get(1).cloned())
        };
        let Some(re) = re else {
            self.status = "请选中实部列（和虚部列），或在 FFT 结果表里运行".into();
            return;
        };
        let label = |k: &String| {
            (
                k.clone(),
                book.column(k)
                    .map_or(k.clone(), |c| format!("{}（{}）", c.key, c.display_name())),
            )
        };
        let rows = [Some(&re), im.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|k| book.column(k))
            .map(|c| c.values().iter().rposition(|v| v.is_finite()).map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0);
        self.dialog = Some(Dialog::Ifft {
            book: id,
            re: label(&re),
            im: im.as_ref().map(label),
            rows,
            two_sided: false,
            odd: false,
        });
    }

    pub(crate) fn open_sg_diff_dialog(&mut self) {
        self.dialog = Some(Dialog::SgDiff {
            window: 7,
            order: 2,
            deriv: 1,
        });
    }

    pub(crate) fn open_smooth_dialog(&mut self) {
        self.dialog = Some(Dialog::Smooth {
            method: SmoothMethod::SavitzkyGolay,
            window: 7,
        });
    }

    pub(crate) fn open_baseline_dialog(&mut self) {
        self.dialog = Some(Dialog::Baseline {
            als: true,
            log_lambda: 5.0,
            p: 0.01,
        });
    }

    fn subtract_baseline(&mut self, method: scicanvas_core::baseline::BaselineMethod) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.subtract_baseline(&src, method) {
            Ok(made) => {
                let keys: Vec<&str> = made.iter().map(|(_, k)| k.as_str()).collect();
                self.status = format!("已扣除基线（{}），结果在新列 {}", method.label(), keys.join(", "));
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_peaks_dialog(&mut self) {
        self.dialog = Some(Dialog::Peaks { percent: 5.0 });
    }

    fn find_peaks(&mut self, min_prominence: f64) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.find_peaks(&src, min_prominence) {
            Ok(made) => {
                let mut parts = vec![];
                for id in &made {
                    if let Some(book) = self.project.book(*id) {
                        parts.push(format!("{}（{} 个峰）", book.name, book.columns[0].data.len()));
                    }
                }
                if let Some(id) = made.last() {
                    self.show_item(*id);
                }
                self.status = format!("峰表在 {}", parts.join("、"));
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_fft_dialog(&mut self) {
        self.dialog = Some(Dialog::Fft {
            window: scicanvas_core::fft::Window::Hann,
        });
    }

    fn fft(&mut self, window: scicanvas_core::fft::Window) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.fft(&src, window) {
            Ok(made) => {
                let names: Vec<String> = made
                    .iter()
                    .filter_map(|(b, _)| self.project.name_of(*b).map(str::to_owned))
                    .collect();
                if let Some((_, g)) = made.last() {
                    self.show_item(*g);
                }
                self.status = format!("FFT 结果在 {}，幅值谱已画成图", names.join("、"));
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_column_math_dialog(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(book) = self.project.book(id) else { return };
        let name = |k: Option<&String>| k.and_then(|k| book.column(k)).map(|c| c.display_name().to_owned());
        let Some(a) = name(keys.first()) else {
            self.status = "请先单击列标题选中一列或两列".into();
            return;
        };
        let b = name(keys.get(1));
        self.dialog = Some(Dialog::ColumnMath {
            a,
            use_constant: b.is_none(),
            b,
            op: scicanvas_core::ops::ColumnOp::Sub,
            constant: 1.0,
        });
    }

    fn column_math(&mut self, op: scicanvas_core::ops::ColumnOp, constant: Option<f64>) {
        use scicanvas_core::ops::Operand;
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(a) = keys.first().cloned() else { return };
        let operand = match (constant, keys.get(1)) {
            (Some(c), _) => Operand::Constant(c),
            (None, Some(b)) => Operand::Column(b.clone()),
            (None, None) => {
                self.status = "请再选中第二列，或改用常数".into();
                return;
            }
        };
        match self.project.column_math(id, &a, op, &operand) {
            Ok(key) => self.status = format!("列运算结果在新列 {key}"),
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_frequency_dialog(&mut self) {
        // 默认范围：要统计的各列里全部数值的最小、最大值
        let (from, to) = self
            .analysis_source()
            .and_then(|src| self.project.value_range(&src))
            .unwrap_or((0.0, 1.0));
        self.dialog = Some(Dialog::Frequency {
            auto: true,
            bins: 10,
            custom: false,
            from,
            to,
        });
    }

    fn frequency_counts(&mut self, bins: Option<usize>, range: Option<(f64, f64)>) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.frequency_counts(&src, bins, range) {
            Ok(ids) => {
                let names: Vec<String> = ids
                    .iter()
                    .filter_map(|id| self.project.name_of(*id).map(str::to_owned))
                    .collect();
                if let Some(last) = ids.last() {
                    self.show_item(*last);
                }
                self.status = format!("频数统计结果在 {}", names.join("、"));
            }
            Err(e) => self.status = e,
        }
    }

    /// 当前工作簿里按点选顺序参与检验的列（没有点选列时用单元格选区覆盖的列）。
    pub(crate) fn open_analysis_curves(&mut self) {
        let Some(graph) = self.active_graph_id() else { return };
        let layer = self.layer_index(graph);
        match self.project.resolve(&Source::Layer {
            graph_id: graph,
            index: layer,
        }) {
            Ok(resolved) => {
                let selected = self.analysis_curves.get(&(graph, layer));
                let curves = resolved
                    .items
                    .iter()
                    .map(|item| {
                        let reference = scicanvas_core::plotspec::DataRef::new(item.book_id, &item.y);
                        let book = self.project.book(item.book_id).unwrap();
                        let name = format!("{}：{}", book.name, book.column(&item.y).unwrap().display_name());
                        let checked = selected.is_none_or(|s| s.contains(&reference));
                        (reference, name, checked)
                    })
                    .collect();
                self.dialog = Some(Dialog::AnalysisCurves { graph, layer, curves });
            }
            Err(error) => self.status = error,
        }
    }

    fn statistical_columns(&self) -> Option<Vec<scicanvas_core::ops::AnalysisColumn>> {
        if self.active_book_id().is_some_and(|id| self.picked(id).0.is_empty()) {
            return Some(Vec::new());
        }
        self.project.analysis_columns(&self.analysis_source()?).ok()
    }

    fn ttest_columns(&self) -> Option<(u64, Vec<String>)> {
        let id = self.active_book_id()?;
        let book = self.project.book(id)?;
        let keys: Vec<String> = self
            .picked(id)
            .0
            .into_iter()
            .filter(|k| book.column(k).is_some())
            .collect();
        Some((id, keys))
    }

    pub(crate) fn open_fft_filter_dialog(&mut self) {
        // 用第一条曲线的采样间隔估计奈奎斯特频率，给出默认截止频率
        let dt = self
            .analysis_source()
            .and_then(|src| self.project.resolve(&src).ok())
            .and_then(|r| {
                let item = r.items.first()?;
                let book = self.project.book(item.book_id)?;
                let (xs, _, _) = scicanvas_core::ops::get_xy(book, item);
                match item.x {
                    Some(_) => scicanvas_core::fft::sample_interval(&xs).ok(),
                    None => Some(1.0),
                }
            })
            .unwrap_or(1.0);
        let nyquist = 0.5 / dt;
        self.dialog = Some(Dialog::FftFilter {
            kind: 0,
            lo: round_sig(nyquist / 10.0, 3),
            hi: round_sig(nyquist / 4.0, 3),
            nyquist,
        });
    }

    fn fft_filter(&mut self, kind: scicanvas_core::fft::FilterKind) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.fft_filter(&src, kind) {
            Ok(keys) => self.status = format!("已 FFT 滤波（{}），结果在新列 {}", kind.label(), keys.join(", ")),
            Err(e) => self.status = e,
        }
    }

    /// 自相关（选一列）或互相关（选两列）。
    pub(crate) fn correlation_function(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        match self.project.correlation_function(id, &keys) {
            Ok((bid, gid)) => {
                let name = self.project.name_of(bid).unwrap_or_default().to_owned();
                self.show_item(gid);
                self.status = format!("相关函数结果在 {name}，已画成图");
            }
            Err(e) => self.status = e,
        }
    }

    /// 当前图形的各条曲线（或选中的 Y 列）插值到公共 X 上求平均。
    pub(crate) fn average_curves(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.average_curves(&src) {
            Ok((bid, gid)) => {
                let name = self.project.name_of(bid).unwrap_or_default().to_owned();
                self.show_item(gid);
                self.status = format!("平均曲线在 {name}（标准差为 Y 误差列）");
            }
            Err(e) => self.status = e,
        }
    }

    /// 对点选的各列逐行统计（平行样的均值、标准差等）。
    pub(crate) fn row_stats(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        match self.project.row_stats(id, &keys) {
            Ok(made) => self.status = format!("行统计结果在新列 {}（标准差已设为 Y 误差列）", made.join(", ")),
            Err(e) => self.status = format!("{e}：请 Ctrl+单击选中平行样的各列"),
        }
    }

    /// 多元线性回归：最后点选的列为因变量，其余为自变量。
    pub(crate) fn regression(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.len() < 2 {
            self.status = "请先点选自变量列，最后点选因变量（Y）列".into();
            return;
        }
        let book = self.project.book(id).unwrap();
        let (ykey, xkeys) = keys.split_last().unwrap();
        let xs: Vec<Vec<f64>> = xkeys.iter().map(|k| book.column(k).unwrap().values()).collect();
        let y = book.column(ykey).unwrap();
        match scicanvas_core::stats::multiple_regression(&xs, &y.values()) {
            Ok(r) => {
                let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
                let names = std::iter::once("截距".to_owned())
                    .chain(xkeys.iter().map(|k| book.column(k).unwrap().display_name().to_owned()));
                for (i, name) in names.enumerate() {
                    rows.push(vec![
                        name,
                        fmt_num(round_sig(r.coef[i], 8)),
                        fmt_num(round_sig(r.se[i], 6)),
                        fmt_num(round_sig(r.t[i], 6)),
                        fmt_num(round_sig(r.p[i], 6)),
                    ]);
                }
                let note = format!(
                    "因变量 {}，N = {}。R² = {}，调整 R² = {}，残差标准差 = {}；\nF = {}，p = {}（检验所有斜率是否同时为 0）。参数的 p 为双侧 t 检验；任一变量缺失的行已跳过。",
                    y.display_name(),
                    r.n,
                    fmt_num(round_sig(r.r2, 6)),
                    fmt_num(round_sig(r.adj_r2, 6)),
                    fmt_num(round_sig(r.s, 6)),
                    fmt_num(round_sig(r.f, 6)),
                    fmt_num(round_sig(r.f_p, 6))
                );
                self.status = format!("多元线性回归：R² = {}", fmt_num(round_sig(r.r2, 6)));
                self.dialog = Some(Dialog::Table {
                    title: "多元线性回归".into(),
                    rows,
                    note,
                });
            }
            Err(e) => self.status = e,
        }
    }

    /// 点选的各列分别做 Shapiro–Wilk 正态性检验。
    pub(crate) fn normality(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.is_empty() {
            self.status = "请先在工作表里单击选中要检验的列".into();
            return;
        }
        let mut rows = vec![["列", "N", "W", "p", "α = 0.05"].map(String::from).to_vec()];
        let mut problems = vec![];
        for c in &columns {
            match scicanvas_core::stats::shapiro_wilk(&c.values.clone()) {
                Ok(r) => rows.push(vec![
                    c.name.as_str().to_owned(),
                    r.n.to_string(),
                    fmt_num(round_sig(r.w, 6)),
                    fmt_num(round_sig(r.p, 6)),
                    if r.p < 0.05 {
                        "偏离正态"
                    } else {
                        "不能拒绝正态"
                    }
                    .into(),
                ]),
                Err(e) => problems.push(format!("{}：{e}", c.name.as_str())),
            }
        }
        if rows.len() == 1 {
            self.status = problems.join("；");
            return;
        }
        self.status = if problems.is_empty() {
            format!("已对 {} 列做正态性检验", rows.len() - 1)
        } else {
            problems.join("；")
        };
        self.dialog = Some(Dialog::Table {
            title: "正态性检验（Shapiro–Wilk）".into(),
            rows,
            note: "原假设：数据来自正态分布。p < 0.05 时认为偏离正态；缺失值已跳过，3 ≤ N ≤ 5000。".into(),
        });
    }

    /// 点选的各列分别做 Grubbs 异常值检验（双侧，α = 0.05）。
    pub(crate) fn grubbs(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.is_empty() {
            self.status = "请先在工作表里单击选中要检验的列".into();
            return;
        }
        let mut rows = vec![["列", "N", "可疑值", "所在行", "G", "临界值 G(0.05)", "p", "结论"]
            .map(String::from)
            .to_vec()];
        let mut problems = vec![];
        for c in &columns {
            match scicanvas_core::stats::grubbs(&c.values.clone(), 0.05) {
                Ok(r) => rows.push(vec![
                    c.name.as_str().to_owned(),
                    r.n.to_string(),
                    fmt_num(r.value),
                    (r.index + 1).to_string(),
                    fmt_num(round_sig(r.g, 6)),
                    fmt_num(round_sig(r.critical, 6)),
                    fmt_num(round_sig(r.p, 6)),
                    if r.g > r.critical {
                        "是异常值"
                    } else {
                        "不是异常值"
                    }
                    .into(),
                ]),
                Err(e) => problems.push(format!("{}：{e}", c.name.as_str())),
            }
        }
        if rows.len() == 1 {
            self.status = problems.join("；");
            return;
        }
        self.status = if problems.is_empty() {
            format!("已对 {} 列做 Grubbs 检验", rows.len() - 1)
        } else {
            problems.join("；")
        };
        self.dialog = Some(Dialog::Table {
            title: "Grubbs 异常值检验".into(),
            rows,
            note: "检验离均值最远的一个点（双侧，α = 0.05），假设其余数据近似正态。\n剔除异常值后如需继续检验，请再运行一次。".into(),
        });
    }

    /// 标准曲线反查：按点选顺序取标准浓度、标准响应、样品响应三列。
    pub(crate) fn calibrate(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.len() != 3 {
            self.status = "请按顺序 Ctrl+单击选中三列：标准浓度（X）、标准响应（Y）、样品响应".into();
            return;
        }
        match self.project.calibrate(id, &keys[0], &keys[1], &keys[2]) {
            Ok((cal, k0, k1)) => {
                self.status = format!(
                    "标准曲线 y = {} + {}·x，R² = {}；反查结果在新列 {k0}、{k1}",
                    fmt_num(round_sig(cal.intercept, 6)),
                    fmt_num(round_sig(cal.slope, 6)),
                    fmt_num(round_sig(cal.r2, 6))
                );
            }
            Err(e) => self.status = e,
        }
    }

    /// 每个点选的列作为一组做单因素方差分析。
    pub(crate) fn anova(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.len() < 2 {
            self.status = "请先在工作表里单击选中至少两列（每列一组）".into();
            return;
        }
        let groups: Vec<Vec<f64>> = columns.iter().map(|c| c.values.clone()).collect();
        let names: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
        match scicanvas_core::stats::one_way_anova(&groups) {
            Ok(result) => {
                self.status = format!(
                    "单因素方差分析：F = {}，p = {}",
                    round_sig(result.f, 4),
                    round_sig(result.p, 4)
                );
                let tukey = scicanvas_core::stats::tukey_hsd(&result, 0.95);
                self.dialog = Some(Dialog::Anova { result, names, tukey });
            }
            Err(e) => self.status = e,
        }
    }

    /// 点选的列两两计算 Pearson、Spearman、Kendall 相关系数。
    pub(crate) fn correlations(&mut self) {
        use scicanvas_core::stats::{correlation, CorrMethod};
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.len() < 2 {
            self.status = "请先在工作表里单击选中至少两列（Ctrl+单击多选）".into();
            return;
        }
        let mut pairs = vec![];
        let mut problems = vec![];
        for i in 0..columns.len() {
            for j in i + 1..columns.len() {
                let (a, b) = (&columns[i], &columns[j]);
                let found: Result<Vec<_>, String> = CorrMethod::ALL
                    .iter()
                    .map(|m| correlation(&a.values.clone(), &b.values.clone(), *m))
                    .collect();
                match found {
                    Ok(cs) => pairs.push((a.name.as_str().to_owned(), b.name.as_str().to_owned(), cs)),
                    Err(e) => problems.push(format!("{} 与 {}：{e}", a.name.as_str(), b.name.as_str())),
                }
            }
        }
        if pairs.is_empty() {
            self.status = problems.join("；");
            return;
        }
        self.status = if problems.is_empty() {
            format!("已计算 {} 对列的相关系数", pairs.len())
        } else {
            format!("已计算 {} 对列；{}", pairs.len(), problems.join("；"))
        };
        self.dialog = Some(Dialog::Correlations(pairs));
    }

    pub(crate) fn open_ttest_dialog(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        let count = columns.len();
        let columns = columns.iter().take(2).map(|c| c.name.clone()).collect();
        self.dialog = Some(Dialog::TTest {
            columns,
            kind: if count >= 2 { 1 } else { 0 },
            mu: 0.0,
            equal_var: false,
            level: 0.95,
        });
    }

    fn run_ttest(&mut self, kind: scicanvas_core::stats::TTestKind, level: f64) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        let names = columns.iter().take(2).map(|c| c.name.clone()).collect();
        let empty = Vec::new();
        let a = columns.first().map(|c| &c.values).unwrap_or(&empty);
        let b = columns.get(1).map(|c| &c.values).unwrap_or(&empty);
        match scicanvas_core::stats::t_test(kind, a, b, level) {
            Ok(result) => {
                self.status = format!(
                    "{}：t = {}，p = {}",
                    kind.label(),
                    round_sig(result.t, 4),
                    round_sig(result.p, 4)
                );
                self.dialog = Some(Dialog::TTestReport { result, names });
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_interpolate_dialog(&mut self) {
        let (from, to) = self
            .analysis_source()
            .and_then(|src| self.project.x_range(&src))
            .unwrap_or((0.0, 1.0));
        self.dialog = Some(Dialog::Interpolate {
            method: scicanvas_core::interpolate::InterpMethod::Spline,
            points: 100,
            custom: false,
            from,
            to,
        });
    }

    fn interpolate(
        &mut self,
        method: scicanvas_core::interpolate::InterpMethod,
        points: usize,
        range: Option<(f64, f64)>,
    ) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.interpolate(&src, method, points, range) {
            Ok((gid, keys)) => {
                self.show_item(gid);
                self.status = format!("已{}插值，结果在新列 {}", method.label(), keys.join(", "));
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_normalize_dialog(&mut self) {
        self.dialog = Some(Dialog::Normalize {
            method: NormalizeMethod::MinMax,
        });
    }

    fn smooth(&mut self, method: SmoothMethod, window: usize) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.smooth(&src, method, window) {
            Ok(keys) => self.status = format!("已平滑，结果在新列 {}", keys.join(", ")),
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn differentiate(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.differentiate(&src) {
            Ok(keys) => self.status = format!("已求导数，结果在新列 {}", keys.join(", ")),
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn integrate(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.integrate(&src) {
            Ok((keys, areas)) => {
                self.status = format!("已积分，累积积分在新列 {}", keys.join(", "));
                self.dialog = Some(Dialog::Areas(areas));
            }
            Err(e) => self.status = e,
        }
    }

    fn normalize(&mut self, method: NormalizeMethod) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.normalize(&src, method) {
            Ok(keys) => self.status = format!("已归一化，结果在新列 {}", keys.join(", ")),
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn stats_on_columns(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.stats_on_columns(&src) {
            Ok(id) => {
                let name = self.project.name_of(id).unwrap_or("").to_string();
                self.show_item(id);
                self.status = format!("统计结果在 {}（标准差为样本标准差，分位数用线性插值）", name);
            }
            Err(e) => self.status = e,
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(d) = self.dialog.as_mut() else { return };
        let layer_before = match d {
            Dialog::LayerContents(edit) => Some(edit.draft.clone()),
            _ => None,
        };
        let action = dialogs::show(ctx, d, &self.pal, &self.project);
        if !matches!(action, dialogs::Action::None) {
            ctx.request_repaint();
        }
        match action {
            dialogs::Action::LayerContents(edit) => match self.apply_layer_contents(&edit) {
                Ok(()) => self.dialog = None,
                Err(error) => {
                    if let Some(Dialog::LayerContents(contents)) = &mut self.dialog {
                        contents.error = Some(error);
                    }
                }
            },
            dialogs::Action::AnalysisCurves(graph, layer, curves) => {
                self.analysis_curves.insert((graph, layer), curves);
                self.dialog = None;
            }
            dialogs::Action::SaveNote(title, text) => {
                self.finish_edit();
                self.begin_edit("保存结果为 Note");
                self.save_note(&title, text);
                self.history.changed();
                self.finish_edit();
                self.dialog = None;
            }
            dialogs::Action::FitBands(reports) => {
                self.finish_edit();
                self.begin_edit("置信/预测带");
                let made: Result<Vec<_>, _> = reports.iter().map(|r| self.project.poly_bands(r, 0.95)).collect();
                self.status = match made {
                    Ok(_) => "已写入 95% 置信带和预测带，并加到图中".into(),
                    Err(e) => e,
                };
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::NlFitBands(reports) => {
                self.finish_edit();
                self.begin_edit("置信/预测带");
                let made: Result<Vec<_>, _> = reports.iter().map(|r| self.project.nlfit_bands(r, 0.95)).collect();
                self.status = match made {
                    Ok(_) => "已写入 95% 置信带和预测带，并加到图中".into(),
                    Err(e) => e,
                };
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::ResidualPlot(reports) => {
                self.finish_edit();
                self.begin_edit("残差图");
                match self.project.residual_plot(&reports) {
                    Ok(id) => {
                        self.show_item(id);
                        self.status = "已生成残差图（残差 = Y − 拟合值）".into();
                        self.history.changed();
                    }
                    Err(error) => self.status = error,
                }
                self.finish_edit();
            }
            dialogs::Action::None => {}
            dialogs::Action::Close => self.dialog = None,
            dialogs::Action::ColumnProps(book, edit) => {
                self.finish_edit();
                self.begin_edit("列属性");
                match self.apply_column_props(book, edit) {
                    Ok(()) => {
                        self.dialog = None;
                        self.history.changed();
                    }
                    Err(e) => {
                        if let Some(Dialog::ColumnProps { error, .. }) = &mut self.dialog {
                            *error = Some(e);
                        }
                    }
                }
                self.finish_edit();
            }
            dialogs::Action::ConvertText(book, key, row, value) => {
                self.finish_edit();
                self.begin_edit("转换为文本列并写入");
                if let Some(column) = self
                    .project
                    .book_mut(book)
                    .and_then(|b| b.columns.iter_mut().find(|c| c.key == key))
                {
                    column.data = column.data.converted(scicanvas_core::DataKind::Text).unwrap();
                    column.set(row, value);
                    self.history.changed();
                    self.status = format!("列 {key} 已转换为文本；已有内容保留");
                }
                self.dialog = None;
                self.finish_edit();
            }
            dialogs::Action::Fit(order) => {
                self.finish_edit();
                self.begin_edit("多项式拟合");
                self.dialog = None;
                self.fit(order);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Ifft(book, re, im, input) => {
                self.finish_edit();
                self.begin_edit("IFFT");
                self.dialog = None;
                match self.project.ifft(book, &re, im.as_deref(), input) {
                    Ok((b, g)) => {
                        self.show_item(g);
                        let name = self.project.name_of(b).unwrap_or("").to_string();
                        self.status = format!("IFFT 结果在 {name}，实部已画成图");
                    }
                    Err(e) => self.status = e,
                }
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Sort(book, keys, within) => {
                self.finish_edit();
                self.begin_edit("排序");
                match self.project.sort_rows(book, &keys, within.as_deref()) {
                    Ok(()) => {
                        let names: Vec<String> = keys
                            .iter()
                            .map(|(k, asc)| format!("{k}{}", if *asc { "升序" } else { "降序" }))
                            .collect();
                        self.status = match &within {
                            Some(cols) => format!("已按 {} 排序 {} 列，其余列不动", names.join("、"), cols.join("、")),
                            None => format!("已按 {} 排序整张工作表", names.join("、")),
                        };
                        self.dialog = None;
                        self.history.changed();
                    }
                    Err(e) => self.status = e,
                }
                self.finish_edit();
            }
            dialogs::Action::CategoryOrder(book, key, levels) => {
                self.finish_edit();
                self.begin_edit("类别顺序");
                let result = self
                    .project
                    .book_mut(book)
                    .and_then(|b| b.columns.iter_mut().find(|c| c.key == key))
                    .ok_or_else(|| "找不到这一列".to_string())
                    .and_then(|c| c.data.set_category_order(&levels));
                match result {
                    Ok(()) => {
                        self.status = format!("已设置 {key} 列的类别顺序");
                        self.dialog = None;
                        self.history.changed();
                    }
                    Err(e) => self.status = e,
                }
                self.finish_edit();
            }
            dialogs::Action::SgDiff(w, order, deriv) => {
                self.finish_edit();
                self.begin_edit("SG 平滑求导");
                self.dialog = None;
                if let Some(src) = self.analysis_source() {
                    match self.project.differentiate_sg(&src, w, order, deriv) {
                        Ok(keys) => self.status = format!("已求导数，结果在新列 {}", keys.join(", ")),
                        Err(e) => self.status = e,
                    }
                }
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Smooth(m, w) => {
                self.finish_edit();
                self.begin_edit("平滑");
                self.dialog = None;
                self.smooth(m, w);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Interpolate(method, points, range) => {
                self.finish_edit();
                self.begin_edit("插值");
                self.dialog = None;
                self.interpolate(method, points, range);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::TTest(kind, level) => self.run_ttest(kind, level),
            dialogs::Action::NlFit(model, limits, text) => {
                let constraints = match scicanvas_core::nlfit::parse_constraints(model, &text) {
                    Ok(c) => c,
                    Err(e) => {
                        // 约束写错时保留对话框，方便修改
                        self.status = e;
                        return;
                    }
                };
                self.finish_edit();
                self.begin_edit("非线性拟合");
                self.dialog = None;
                self.nlfit(model, &limits, &constraints);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::SetValues(formula, keep) => {
                self.finish_edit();
                self.begin_edit("设置列值");
                if self.set_column_values(&formula, keep) {
                    self.dialog = None;
                    self.history.changed();
                }
                self.finish_edit();
            }
            dialogs::Action::Fill(kind, rows) => {
                self.finish_edit();
                self.begin_edit("填充数据序列");
                self.dialog = None;
                self.fill_sequence(kind, rows);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::FftFilter(kind) => {
                self.finish_edit();
                self.begin_edit("FFT 滤波");
                self.dialog = None;
                self.fft_filter(kind);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Baseline(method) => {
                self.finish_edit();
                self.begin_edit("扣除基线");
                self.dialog = None;
                self.subtract_baseline(method);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Peaks(min_prominence) => {
                self.finish_edit();
                self.begin_edit("寻峰");
                self.dialog = None;
                self.find_peaks(min_prominence);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Fft(window) => {
                self.finish_edit();
                self.begin_edit("FFT");
                self.dialog = None;
                self.fft(window);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::ColumnMath(op, constant) => {
                self.finish_edit();
                self.begin_edit("列运算");
                self.dialog = None;
                self.column_math(op, constant);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::Frequency(bins, range) => {
                self.finish_edit();
                self.begin_edit("频数统计");
                self.dialog = None;
                self.frequency_counts(bins, range);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::ExportPng(dpi) => {
                self.dialog = None;
                self.export_png(dpi);
            }
            dialogs::Action::Normalize(m) => {
                self.finish_edit();
                self.begin_edit("归一化");
                self.dialog = None;
                self.normalize(m);
                self.history.changed();
                self.finish_edit();
            }
        }
        if let Some(Dialog::LayerContents(edit)) = &mut self.dialog {
            if layer_before.as_ref() != Some(&edit.draft) {
                ctx.request_repaint();
            }
            if self
                .project
                .graph(edit.graph)
                .and_then(|g| g.page.layers().get(edit.index))
                != Some(&edit.before)
            {
                edit.error = Some("图层已更新，请重新打开图层内容".into());
            }
        }
        if self.refresh_book_preview() {
            ctx.request_repaint();
        }
    }

    // ---------------------------------------------------------------- 输入

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        // 正在输入文字时，Ctrl+Z 留给 TextEdit 自己的文字历史。
        let text_focus = ctx
            .memory(|m| m.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some());
        if !text_focus {
            let redo_mod = Modifiers {
                shift: true,
                ..Modifiers::COMMAND
            };
            let action = ctx.input_mut(|i| {
                if i.consume_key(redo_mod, Key::Z) || i.consume_key(Modifiers::COMMAND, Key::Y) {
                    Some("edit.redo")
                } else if i.consume_key(Modifiers::COMMAND, Key::Z) {
                    Some("edit.undo")
                } else {
                    None
                }
            });
            if let Some(id) = action {
                self.run_command(id);
            }
        }
        // 命令面板打开时，方向键、回车由面板自己处理
        let keys: [(Modifiers, Key, &str); 9] = [
            (Modifiers::COMMAND, Key::N, "book.new"),
            (Modifiers::COMMAND, Key::O, "file.open"),
            (Modifiers::COMMAND, Key::S, "file.save"),
            (Modifiers::COMMAND, Key::I, "file.import"),
            (Modifiers::COMMAND, Key::E, "graph.export"),
            (Modifiers::COMMAND, Key::J, "graph.copy"),
            (Modifiers::COMMAND, Key::K, "palette.open"),
            (Modifiers::COMMAND, Key::F1, "ribbon.toggle"),
            (Modifiers::NONE, Key::F1, "help.show"),
        ];
        let hit: Vec<&str> = ctx.input_mut(|inp| {
            keys.iter()
                .filter(|(m, k, _)| inp.consume_key(*m, *k))
                .map(|(_, _, id)| *id)
                .collect()
        });
        for id in hit {
            self.run_command(id);
        }
        if self.ui.backstage && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.ui.backstage = false;
        }
    }

    // ---------------------------------------------------------------- 界面

    fn stats_text(&self) -> String {
        let Some(id) = self.active else { return String::new() };
        let Some(b) = self.project.book(id) else {
            return String::new();
        };
        let sel = self.selected_keys(id);
        let v: Vec<f64> = match self.multi_range(id) {
            Some(r) if sel.is_empty() && !b.columns.is_empty() => b.columns[r.c0()..=r.c1().min(b.columns.len() - 1)]
                .iter()
                .flat_map(|c| (r.r0()..=r.r1()).map(move |i| c.data.f64_at(i).unwrap_or(f64::NAN)))
                .filter(|x| x.is_finite())
                .collect(),
            _ => b
                .columns
                .iter()
                .filter(|c| sel.contains(&c.key))
                .flat_map(|c| c.values())
                .filter(|x| x.is_finite())
                .collect(),
        };
        if v.is_empty() {
            return String::new();
        }
        let sum: f64 = v.iter().sum();
        let min = v.iter().copied().fold(f64::INFINITY, f64::min);
        let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        format!(
            "平均值 = {}   总和 = {}   计数 = {}   最小 = {}   最大 = {}",
            scicanvas_core::fmt_num(sum / v.len() as f64),
            scicanvas_core::fmt_num(sum),
            v.len(),
            scicanvas_core::fmt_num(min),
            scicanvas_core::fmt_num(max)
        )
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let stats = self.stats_text();
        ui.horizontal(|ui| {
            ui.label(self.status.as_str());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(stats).monospace().weak());
            });
        });
    }

    fn refresh_book_preview(&mut self) -> bool {
        let edit = match &self.dialog {
            Some(Dialog::ColumnProps { book, edit, .. }) => BookPreviewEdit::Properties(*book, edit.clone()),
            Some(Dialog::CategoryOrder { book, key, levels, .. }) => {
                BookPreviewEdit::Categories(*book, key.clone(), levels.clone())
            }
            Some(Dialog::Sort {
                book,
                columns,
                keys,
                selected,
                only_selected,
                ..
            }) => BookPreviewEdit::Sort(
                *book,
                keys.iter()
                    .filter_map(|(i, asc)| i.map(|i| (columns[i].0.clone(), *asc)))
                    .collect(),
                (*only_selected && !selected.is_empty()).then(|| selected.clone()),
            ),
            _ => return self.book_preview.take().is_some(),
        };
        if self.book_preview.as_ref().is_some_and(|p| p.edit == edit) {
            return false;
        }
        let id = match &edit {
            BookPreviewEdit::Properties(id, _)
            | BookPreviewEdit::Categories(id, ..)
            | BookPreviewEdit::Sort(id, ..) => *id,
        };
        let reusable = matches!((&self.book_preview, &edit),
            (Some(BookPreview { edit: BookPreviewEdit::Properties(old_id, old), result: Ok(_) }),
             BookPreviewEdit::Properties(id, new))
            if old_id == id && old.kind == new.kind && old.role == new.role
                && old.format.utc_offset == new.format.utc_offset);
        // ponytail: type/role/category/sort changes copy one table; shared column storage can reduce this for very large tables.
        // Names, units and display formats reuse the cached data, including during continuous typing.
        let base = if reusable {
            self.book_preview.take().unwrap().result
        } else {
            self.project.book(id).cloned().ok_or_else(|| "找不到工作簿".to_string())
        };
        let result = base.and_then(|mut book| {
            match &edit {
                BookPreviewEdit::Properties(_, edit) => edit.apply(&mut book)?,
                BookPreviewEdit::Categories(_, key, levels) => book
                    .columns
                    .iter_mut()
                    .find(|c| &c.key == key)
                    .ok_or("找不到列")?
                    .data
                    .set_category_order(levels)?,
                BookPreviewEdit::Sort(_, keys, within) => {
                    let mut project = Project::default();
                    project.books.push(book);
                    project.sort_rows(id, keys, within.as_deref())?;
                    book = project.books.remove(0);
                }
            }
            Ok(book)
        });
        if let Some(Dialog::ColumnProps {
            error, sample, edit, ..
        }) = &mut self.dialog
        {
            *error = result.as_ref().err().cloned();
            if let Ok(book) = &result {
                *sample = book
                    .column(&edit.key)
                    .and_then(|c| (0..c.data.len()).find_map(|r| c.data.f64_at(r).filter(|v| v.is_finite())));
            }
        }
        if let Some(Dialog::CategoryOrder { error, .. } | Dialog::Sort { error, .. }) = &mut self.dialog {
            *error = result.as_ref().err().cloned();
        }
        self.book_preview = Some(BookPreview { edit, result });
        true
    }

    fn preview_graph(&self, id: u64) -> Option<std::borrow::Cow<'_, scicanvas_core::Graph>> {
        use std::borrow::Cow;
        let graph = self.project.graph(id)?;
        if let Some(Dialog::LayerContents(edit)) = &self.dialog {
            if edit.graph == id && graph.page.layers().get(edit.index) == Some(&edit.before) {
                let valid = edit.draft.y_keys.is_empty()
                    || edit
                        .draft
                        .book_id
                        .and_then(|id| self.project.book(id))
                        .is_some_and(|book| edit.draft.validate_in(book, &|id| self.project.book(id)).is_ok());
                if valid && edit.draft != edit.before {
                    let mut graph = graph.clone();
                    graph.page.layers_mut()[edit.index] = edit.draft.clone();
                    return Some(Cow::Owned(graph));
                }
            }
        }
        Some(Cow::Borrowed(graph))
    }

    fn graph_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        // The plot call is read-only. Temporarily supply the preview table to the shared scene builder,
        // then restore it before processing any commands, exports or history operations.
        let mut preview = self.book_preview.take();
        let preview_book = preview.as_mut().and_then(|p| p.result.as_mut().ok());
        let at = preview_book
            .as_ref()
            .and_then(|b| self.project.books.iter().position(|source| source.id == b.id));
        if let (Some(at), Some(book)) = (at, preview_book) {
            std::mem::swap(&mut self.project.books[at], book);
        }
        let out = self
            .preview_graph(id)
            .map(|g| plot::show(ui, &g, &self.project, self.layer_index(id)));
        if let Some(at) = at {
            std::mem::swap(
                &mut self.project.books[at],
                preview.as_mut().unwrap().result.as_mut().unwrap(),
            );
        }
        self.book_preview = preview;
        let Some(out) = out else { return };
        if let Some(index) = out.selected_layer {
            self.select_layer(id, index);
        }
        let rect = out.response.rect;

        // 右下角显示鼠标处的坐标
        if let Some((x, y)) = out.hover {
            // 吸附到数据点时显示该点的值，否则显示鼠标处坐标
            let text = match &out.point {
                Some(hit) => format!(
                    "图层 {} · {} · x = {}   y = {}",
                    out.hover_layer.unwrap_or(0) + 1,
                    hit.series,
                    scicanvas_core::fmt_num(hit.x),
                    scicanvas_core::fmt_num(hit.y)
                ),
                None => format!(
                    "图层 {} · x = {}   y = {}",
                    out.hover_layer.unwrap_or(0) + 1,
                    round_sig(x, 4),
                    round_sig(y, 4)
                ),
            };
            ui.painter().text(
                rect.right_bottom() + egui::vec2(-8.0, -6.0),
                egui::Align2::RIGHT_BOTTOM,
                text,
                egui::FontId::proportional(11.5),
                egui::Color32::from_gray(110),
            );
        } else if out.zoomed {
            ui.painter().text(
                rect.right_bottom() + egui::vec2(-8.0, -6.0),
                egui::Align2::RIGHT_BOTTOM,
                "双击恢复原始范围",
                egui::FontId::proportional(11.5),
                egui::Color32::from_gray(140),
            );
        }

        let mut act: Option<&'static str> = None;
        let resp = out.response;
        // 右键时吸附到的曲线（菜单打开期间保持）
        let curve_id = egui::Id::new(("graph-curve-menu", id));
        if resp.secondary_clicked() {
            let hit = out
                .point
                .as_ref()
                .map(|p| (out.hover_layer.unwrap_or(0), p.key.clone(), p.series.clone()));
            ui.data_mut(|d| d.insert_temp(curve_id, hit));
        }
        let menu_curve = ui
            .data(|d| d.get_temp::<Option<(usize, String, String)>>(curve_id))
            .flatten();
        let mut remove: Option<(usize, String)> = None;
        resp.context_menu(|ui| {
            if let Some((layer, key, name)) = &menu_curve {
                if ui.button(format!("移除曲线 {name}")).clicked() {
                    remove = Some((*layer, key.clone()));
                    ui.close_menu();
                }
                ui.separator();
            }
            for (cid, label) in [
                ("layer.contents", "图层内容…"),
                ("graph.props", "绘图属性…"),
                ("graph.copy", "复制图形"),
                ("graph.export", "导出 SVG…"),
                ("graph.export_png", "导出 PNG…"),
                ("", ""),
                ("graph.legend", "图例"),
                ("graph.grid", "网格线"),
                ("graph.logx", "X 对数刻度"),
                ("graph.logy", "Y 对数刻度"),
            ] {
                if cid.is_empty() {
                    ui.separator();
                    continue;
                }
                if ui.button(label).clicked() {
                    act = Some(cid);
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("恢复原始范围").clicked() {
                plot::reset_view(ui.ctx(), id, self.layer_index(id));
                ui.close_menu();
            }
        });
        if let Some((layer, key)) = remove {
            self.remove_curve(id, layer, &key);
        }
        if let Some(cid) = act {
            self.active = Some(id);
            self.run_command(cid);
        }
    }

    /// 把工作簿 `book_id` 里选中的列加到图形当前图层（一步撤销）。
    pub(crate) fn add_to_graph(&mut self, book_id: u64, gid: u64) {
        let keys = self.selected_keys(book_id);
        let layer = self.layer_index(gid);
        self.begin_edit("添加曲线");
        match self.project.add_curves(gid, layer, book_id, &keys) {
            Ok(n) => {
                if n > 0 {
                    self.history.changed();
                }
                self.status = if n > 0 {
                    format!("已添加 {n} 条曲线")
                } else {
                    "这些列已在图层里".into()
                };
                self.show_item(gid);
            }
            Err(why) => self.status = why,
        }
        self.finish_edit();
    }

    /// 从图层移除一条曲线（一步撤销）。
    pub(crate) fn remove_curve(&mut self, gid: u64, layer: usize, key: &str) {
        self.begin_edit("移除曲线");
        if let Some(l) = self
            .project
            .graph_mut(gid)
            .and_then(|g| g.page.layers_mut().get_mut(layer))
        {
            if l.y_keys.iter().any(|k| k == key) {
                l.remove_curve(key);
                self.history.changed();
                self.status = "已移除曲线".into();
            }
        }
        self.finish_edit();
    }

    fn props_window(&mut self, ctx: &egui::Context) {
        let Some(gid) = self.props_for else { return };
        let Some(g0) = self.project.graph(gid) else {
            self.props_for = None;
            return;
        };
        let graph_before = g0.clone();
        let column_options: HashMap<u64, Vec<(String, String)>> = self
            .project
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
        // 每个工作簿的默认列映射（与“选中工作簿后作图”相同），切换数据来源时套用
        let book_specs: Vec<(u64, String, Option<scicanvas_core::plotspec::PlotSpec>)> = self
            .project
            .books
            .iter()
            .map(|b| {
                let spec = scicanvas_core::plotspec::PlotSpec::from_selection(b, &[], PlotType::LineSymbol).ok();
                (b.id, b.name.clone(), spec)
            })
            .collect();
        // 可加入图层的其他工作簿曲线：(列标识, 显示名, 默认 X 列)；以及带工作簿名的曲线显示名
        use scicanvas_core::plotspec::{default_x, foreign_key, split_key};
        let mut curve_options: HashMap<u64, Vec<(String, String, Option<String>)>> = HashMap::new();
        let mut curve_names: HashMap<String, String> = HashMap::new();
        for b in &self.project.books {
            for c in &b.columns {
                curve_names.insert(foreign_key(b.id, &c.key), format!("{}：{}", b.name, c.display_name()));
                if matches!(c.role, Role::Y | Role::Z) {
                    curve_options.entry(b.id).or_default().push((
                        c.key.clone(),
                        c.display_name().to_string(),
                        default_x(b, &c.key),
                    ));
                }
            }
        }
        let mut props_layer = self.layer_index(gid);
        let title = format!("绘图属性 — {}", g0.name);
        let history_group = &mut self.history_group;
        let project = &mut self.project;

        let mut open = true;
        let mut close = false;
        dialogs::window(ctx, &title)
            .id(egui::Id::new("scicanvas-props"))
            .open(&mut open)
            .default_width(360.0)
            .default_height(ctx.screen_rect().height() * 0.75)
            .show(ctx, |ui| {
                let Some(graph) = project.graph_mut(gid) else { return };
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
                let cols = g
                    .book_id
                    .and_then(|id| column_options.get(&id))
                    .cloned()
                    .unwrap_or_default();
                egui::Grid::new(("props-grid", gid, props_layer))
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("数据工作簿");
                        let current = book_specs
                            .iter()
                            .find(|(id, _, _)| Some(*id) == g.book_id)
                            .map_or("（未选择）", |(_, name, _)| name.as_str());
                        egui::ComboBox::from_id_salt("props-book")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (id, name, spec) in &book_specs {
                                    if ui.selectable_label(g.book_id == Some(*id), name).clicked()
                                        && g.book_id != Some(*id)
                                    {
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
                                        .add_enabled(
                                            valid.is_ok(),
                                            egui::SelectableLabel::new(g.plot_type == t, t.label()),
                                        )
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
                                for (k, l) in &cols {
                                    ui.selectable_value(&mut g.x_key, Some(k.clone()), l.as_str());
                                }
                            });
                        ui.end_row();

                        ui.label("Y 列");
                        ui.vertical(|ui| {
                            for (k, l) in &cols {
                                let mut on = g.y_keys.contains(k);
                                if ui.checkbox(&mut on, l.as_str()).changed() {
                                    if on {
                                        g.y_keys.push(k.clone());
                                    } else {
                                        g.y_keys.retain(|x| x != k);
                                    }
                                }
                            }
                        });
                        ui.end_row();

                        // 其他工作簿的曲线：同一图层可以放不同工作簿的数据
                        let foreign: Vec<String> = g
                            .y_keys
                            .iter()
                            .filter(|k| split_key(k).1.is_some_and(|b| Some(b) != g.book_id))
                            .cloned()
                            .collect();
                        if book_specs.len() > 1 || !foreign.is_empty() {
                            ui.label("其他工作簿");
                            ui.vertical(|ui| {
                                for k in &foreign {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(curve_names.get(k).map_or(k.as_str(), String::as_str));
                                        if ui.small_button("✕").on_hover_text("移除").clicked() {
                                            g.remove_curve(k);
                                        }
                                    });
                                }
                                ui.menu_button("＋ 添加曲线", |ui| {
                                    for (bid, name, _) in &book_specs {
                                        if Some(*bid) == g.book_id {
                                            continue;
                                        }
                                        let Some(options) = curve_options.get(bid) else {
                                            continue;
                                        };
                                        ui.menu_button(name, |ui| {
                                            for (key, label, x) in options {
                                                let q = foreign_key(*bid, key);
                                                let on = g.y_keys.contains(&q);
                                                if ui.add_enabled(!on, egui::Button::new(label)).clicked() {
                                                    g.y_keys.push(q.clone());
                                                    if let Some(x) = x {
                                                        g.x_by_y.insert(q, foreign_key(*bid, x));
                                                    }
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    }
                                });
                            });
                            ui.end_row();
                        }

                        if g.plot_type == PlotType::FloatingBar {
                            ui.label("浮动柱下端");
                            crate::builder::floating_columns(ui, &mut g.spec, &cols);
                            ui.end_row();
                        }
                        ui.label("图标题");
                        edit_group(
                            &ui.add(egui::TextEdit::singleline(&mut g.title).hint_text("（无）")),
                            history_group,
                        );
                        ui.end_row();
                        ui.label("X 轴标题");
                        edit_group(
                            &ui.add(egui::TextEdit::singleline(&mut g.x_title).hint_text("自动：长名称 (单位)")),
                            history_group,
                        );
                        ui.end_row();
                        ui.label("Y 轴标题");
                        edit_group(
                            &ui.add(egui::TextEdit::singleline(&mut g.y_title).hint_text("自动：长名称 (单位)")),
                            history_group,
                        );
                        ui.end_row();

                        // 刻度格式：日期列做 X 时自动用日期刻度，这里只设数字格式
                        for (label, format, salt) in [
                            ("X 刻度格式", &mut g.x_format, "props-x-format"),
                            ("Y 刻度格式", &mut g.y_format, "props-y-format"),
                        ] {
                            use scicanvas_core::format::NumberFormat;
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
                        ui.label("Y 范围");
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
                            ui.checkbox(&mut g.y_log, "对数");
                        });
                        ui.end_row();

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
                                        for (k, name) in &cols {
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
                    });
                // 逐曲线样式：选一条数据曲线，单独设图型、颜色、符号、线宽、符号大小和右侧 Y 轴
                let curve_keys: Vec<String> = g.y_keys.iter().filter(|k| !g.line_only.contains(k)).cloned().collect();
                if !curve_keys.is_empty() {
                    egui::CollapsingHeader::new("曲线样式")
                        .id_salt(("props-curves", gid, props_layer))
                        .show(ui, |ui| {
                            use scicanvas_core::page::CurveStyle;
                            let pick_id = egui::Id::new(("props-curve-pick", gid, props_layer));
                            let mut key = ui
                                .data(|d| d.get_temp::<String>(pick_id))
                                .filter(|k| curve_keys.contains(k))
                                .unwrap_or_else(|| curve_keys[0].clone());
                            let name_of = |k: &str| {
                                cols.iter()
                                    .find(|(c, _)| c == k)
                                    .map(|(_, n)| n.clone())
                                    .or_else(|| curve_names.get(k).cloned())
                                    .unwrap_or_else(|| k.to_string())
                            };
                            egui::ComboBox::from_id_salt(("props-curve", gid))
                                .selected_text(name_of(&key))
                                .show_ui(ui, |ui| {
                                    for k in &curve_keys {
                                        ui.selectable_value(&mut key, k.clone(), name_of(k));
                                    }
                                });
                            ui.data_mut(|d| d.insert_temp(pick_id, key.clone()));
                            let mut style = g.curves.get(&key).cloned().unwrap_or_default();
                            egui::Grid::new(("props-curve-grid", gid, props_layer))
                                .num_columns(2)
                                .spacing([10.0, 6.0])
                                .show(ui, |ui| {
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
                                            let r = ui
                                                .add_enabled(own, egui::DragValue::new(&mut v).range(range).speed(0.1));
                                            edit_group(&r, history_group);
                                            *value = own.then_some(v);
                                        });
                                        ui.end_row();
                                    }
                                    ui.label("坐标轴");
                                    ui.checkbox(&mut style.right_axis, "右侧 Y 轴");
                                    ui.end_row();
                                });
                            if style.is_default() {
                                g.curves.remove(&key);
                            } else if g.curves.get(&key) != Some(&style) {
                                g.curves.insert(key, style);
                            }
                            if g.curves.values().any(|c| c.right_axis) {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("右轴标题");
                                    edit_group(
                                        &ui.add(
                                            egui::TextEdit::singleline(&mut g.y2_title)
                                                .hint_text("自动：长名称 (单位)"),
                                        ),
                                        history_group,
                                    );
                                });
                            }
                        });
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new("修改即时生效。导出 SVG 时使用这里的设置。")
                        .small()
                        .weak(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("关闭").clicked() {
                        close = true;
                    }
                });
            });
        if self.project.graph(gid).unwrap() != &graph_before {
            ctx.request_repaint();
            if !self.history.is_pending() {
                let after = std::mem::replace(self.project.graph_mut(gid).unwrap(), graph_before);
                self.begin_edit("绘图属性");
                *self.project.graph_mut(gid).unwrap() = after;
            }
            self.history.changed();
        }
        if props_layer != self.layer_index(gid) {
            self.select_layer(gid, props_layer);
        }
        if !open || close {
            self.props_for = None;
        }
    }

    fn help_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("快速上手")
            .id(egui::Id::new("scicanvas-help"))
            .constrain(false)
            .open(&mut self.show_help)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label("1. 功能区：顶部选项卡按任务分组。激活工作簿时出现紫色的“工作表”选项卡，激活图形时出现“图形”选项卡。双击选项卡或按 Ctrl+F1 折叠功能区；Ctrl+K 搜索任何命令。");
                ui.label("2. 输入数据：在工作簿里直接键入；或从 Excel 复制一块区域，单击单元格后 Ctrl+V，首行是文字时自动作为长名称。也可以 文件 → 导入数据。");
                ui.label("3. 选列与绘图：单击列标题，Ctrl 或 Shift 多选；再点“绘图”选项卡里的图类型。没有选列时使用全部 Y 列。");
                ui.label("4. 分析：“分析”选项卡里有线性 / 多项式拟合、平滑、微分、积分、归一化，“统计”选项卡里有列统计。对当前图形的曲线，或工作簿里选中的 Y 列计算，结果写进紧挨原列的新列。");
                ui.label("5. 导出与保存：文件 → 导出图形（SVG）；文件 → 保存项目（.scv，可用解压软件查看其中的表格和图）。");
            });
    }

    fn confirm_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.confirm_delete else { return };
        let name = self.project.name_of(id).unwrap_or("").to_string();
        let mut decision: Option<bool> = None;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            decision = Some(false);
        }
        let weak = self.pal.weak;
        dialogs::window(ctx, "删除").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new(format!("确定删除“{name}”吗？")).size(14.0).strong());
                ui.add_space(4.0);
                ui.label(RichText::new("删除后可以用“撤销”（Ctrl+Z）恢复。").color(weak));
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                dialogs::center_buttons(ui, 2);
                let danger = egui::Button::new(RichText::new("删除").color(egui::Color32::WHITE).strong())
                    .fill(egui::Color32::from_rgb(0xC4, 0x2B, 0x1C))
                    .min_size(dialogs::BUTTON);
                if ui.add(danger).clicked() {
                    decision = Some(true);
                }
                if ui.add(dialogs::plain("取消")).clicked() {
                    decision = Some(false);
                }
            });
        });
        match decision {
            Some(true) => {
                self.finish_edit();
                self.confirm_delete = None;
                self.delete_item(id);
                self.finish_edit();
            }
            Some(false) => self.confirm_delete = None,
            None => {}
        }
    }
}

impl eframe::App for SciApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_frame(ctx);
    }
}

impl SciApp {
    fn ui_frame(&mut self, ctx: &egui::Context) {
        let reset = egui::Modifiers {
            shift: true,
            ..egui::Modifiers::COMMAND
        };
        if ctx.input_mut(|input| input.consume_key(reset, egui::Key::R)) {
            self.run_command("win.reset");
        }
        if std::mem::take(&mut self.pending_window_reset) {
            dialogs::reset_docks(ctx);
            ctx.memory_mut(|memory| memory.reset_areas());
        }
        dialogs::begin_docks(ctx);
        crate::chrome::edges(ctx, &self.pal);
        self.dropped_files(ctx);
        if self.last_history_group.is_some()
            && ctx.input(|i| {
                i.pointer.any_pressed()
                    || i.key_pressed(egui::Key::Tab)
                    || i.key_pressed(egui::Key::Enter)
                    || i.key_pressed(egui::Key::Escape)
            })
        {
            self.finish_edit();
        }
        self.history_group = None;
        if let Some(pref) = self.pending_theme.take() {
            ctx.set_theme(pref);
        }
        self.pal = theme::palette(ctx.theme());
        // 命令面板打开时不处理全局快捷键，避免和输入框冲突
        if self.ui.palette.is_none() {
            self.shortcuts(ctx);
        }

        // 功能区和命令面板只读地访问程序状态，被点击的命令在绘制完成后执行
        let mut clicked: Option<&'static str> = None;
        let mut ui_state = std::mem::take(&mut self.ui);
        let pal = self.pal;
        ribbon::show(ctx, self, &mut ui_state, &pal, &mut clicked);

        let style = ctx.style();
        let bar = egui::Frame::side_top_panel(&style).inner_margin(egui::Margin::symmetric(8.0, 3.0));
        egui::TopBottomPanel::bottom("statusbar")
            .frame(bar)
            .show(ctx, |ui| self.status_bar(ui));

        if ui_state.backstage {
            if let Some(id) = ribbon::backstage(ctx, self, &mut ui_state, &pal, &mut clicked) {
                ui_state.backstage = false;
                self.show_item(id);
            }
        } else {
            let left_docked = dialogs::has_docks(ctx, dialogs::DockSide::Left);
            if self.show_explorer || left_docked {
                let explorer = self.show_explorer;
                egui::SidePanel::left("explorer")
                    .frame(
                        egui::Frame::side_top_panel(&style)
                            .fill(style.visuals.window_fill)
                            .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
                    )
                    .resizable(true)
                    .default_width(190.0)
                    .width_range(if left_docked { 320.0..=600.0 } else { 140.0..=600.0 })
                    .show(ctx, |ui| {
                        if explorer {
                            if left_docked {
                                egui::TopBottomPanel::top("explorer-tree")
                                    .resizable(true)
                                    .default_height(200.0)
                                    .height_range(80.0..=(ui.available_height() * 0.65).max(80.0))
                                    .show_inside(ui, |ui| {
                                        egui::ScrollArea::vertical().show(ui, |ui| self.explorer(ui));
                                    });
                            } else {
                                self.explorer(ui);
                            }
                        } else {
                            self.explorer_focus = false;
                        }
                        let mut rect = ui.available_rect_before_wrap();
                        if !left_docked {
                            rect = ui.max_rect();
                            rect.min.y += rect.height().min(200.0);
                        }
                        dialogs::place_docks(ui, dialogs::DockSide::Left, rect);
                    });
            } else {
                self.explorer_focus = false;
            }
            if dialogs::has_docks(ctx, dialogs::DockSide::Right) {
                egui::SidePanel::right("settings-right")
                    .resizable(true)
                    .default_width(360.0)
                    .width_range(320.0..=600.0)
                    .show(ctx, |ui| {
                        dialogs::place_docks(ui, dialogs::DockSide::Right, ui.available_rect_before_wrap())
                    });
            }
            // 子窗口放在面板之间剩下的区域里（Origin 的 MDI 工作区）
            self.refresh_book_preview();
            self.child_windows(ctx);
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(self.pal.workspace))
                .show(ctx, |_ui| {});
        }
        ribbon::palette(ctx, self, &mut ui_state, &pal, &mut clicked);
        self.ui = ui_state;

        if let Some(id) = clicked {
            self.run_command(id);
        }
        if let Some(t) = self.pending_copy.take() {
            ctx.copy_text(t);
        }

        self.props_window(ctx);
        self.help_window(ctx);
        self.confirm_window(ctx);
        self.dialogs(ctx);
        if crate::builder::show(ctx, &self.project, &mut self.plot_draft) {
            self.run_command("plot.apply_spec");
        }
        dialogs::end_docks(ctx);
        self.finish_frame_edit(self.history_group);
        if std::mem::take(&mut self.pending_repaint) {
            ctx.request_repaint();
        }

        if self.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;

    fn app() -> SciApp {
        SciApp::initial(theme::palette(egui::Theme::Light))
    }

    /// 百万行十列的首帧测量（不进 CI）：
    /// `cargo test --release -p scicanvas --bin scicanvas million_row_first_frame -- --ignored --nocapture`
    /// 计时包括导入、建图和完整界面首帧（布局与三角化，不含 GPU 提交）；进程峰值内存由外部采样。
    #[test]
    #[ignore]
    fn million_row_first_frame() {
        use std::fmt::Write;
        use std::time::Instant;
        let rows = 1_000_000;
        let mut input = String::from("X,Y1,Y2,Y3,Y4,Y5,Y6,Y7,Y8,Y9\n");
        for i in (0..rows).rev() {
            writeln!(
                input,
                "{i},{},{},{},{},{},{},{},{},{}",
                i % 97,
                i % 89,
                i % 83,
                i % 79,
                i % 73,
                i % 71,
                i % 67,
                i % 61,
                i % 59
            )
            .unwrap();
        }
        let mut app = app();
        let t = Instant::now();
        let id = app.project.alloc_id();
        let book = scicanvas_core::csv::book_from_text(id, "Million", &input);
        drop(input);
        app.project.books.push(book);
        let import = t.elapsed();
        let t = Instant::now();
        let (gid, _) = app
            .project
            .plot_from_selection(id, &["B".into()], PlotType::Scatter)
            .unwrap();
        app.show_item(id);
        app.show_item(gid);
        let plot = t.elapsed();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        egui_extras::install_image_loaders(&ctx);
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0))),
            ..Default::default()
        };
        let frame = |app: &mut SciApp| {
            let t = Instant::now();
            let out = ctx.run(input(), |ctx| app.ui_frame(ctx));
            let meshes = ctx.tessellate(out.shapes, out.pixels_per_point);
            (t.elapsed(), meshes.len())
        };
        let (first, meshes) = frame(&mut app);
        let (second, _) = frame(&mut app);
        println!(
            "PERF import_ms={:.1} plot_ms={:.1} first_frame_ms={:.1} second_frame_ms={:.1} meshes={meshes}",
            import.as_secs_f64() * 1000.0,
            plot.as_secs_f64() * 1000.0,
            first.as_secs_f64() * 1000.0,
            second.as_secs_f64() * 1000.0
        );
    }

    fn roundtrip(app: &mut SciApp, before: &Project) {
        let after = app.project.clone();
        assert_ne!(&after, before);
        app.run_command("edit.undo");
        assert_eq!(&app.project, before);
        app.run_command("edit.redo");
        assert_eq!(app.project, after);
    }

    #[test]
    fn floating_dialog_moves_beyond_screen_edge_and_reset_preserves_its_draft() {
        let mut app = app();
        app.run_command("layer.contents");
        let Some(Dialog::LayerContents(edit)) = &mut app.dialog else {
            panic!()
        };
        edit.draft.legend = !edit.before.legend;
        let draft = edit.draft.clone();
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0));
        let id = egui::Id::new(("scicanvas-dialog", "图层内容"));
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.02;
            let _ = ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            );
            ctx.memory(|memory| memory.area_rect(id)).unwrap()
        };
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        let rect = frame(&mut app, vec![]);
        let start = egui::pos2(rect.center().x, rect.top() + 12.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![egui::Event::PointerMoved(start), button(start, true)]);
        frame(&mut app, vec![egui::Event::PointerMoved(start + egui::vec2(60.0, 0.0))]);
        let end = egui::pos2(screen.right() + 150.0, start.y);
        frame(&mut app, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, vec![button(end, false)]);
        let moved = frame(&mut app, vec![]);
        assert!(moved.right() > screen.right() + 100.0, "{moved:?}");
        assert_eq!(frame(&mut app, vec![]).min, moved.min);
        app.geoms.get_mut(&1).unwrap().rect =
            egui::Rect::from_min_size(egui::pos2(3000.0, 3000.0), egui::vec2(400.0, 300.0));
        app.ui.open_palette();
        frame(
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::R,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    shift: true,
                    ..Default::default()
                },
            }],
        );
        frame(&mut app, vec![]);
        let reset = frame(&mut app, vec![]);
        assert!(reset.left() >= screen.left() && reset.right() <= screen.right());
        assert!(app.geoms[&1].rect.left() < app.ws.width());
        let Some(Dialog::LayerContents(edit)) = &app.dialog else {
            panic!()
        };
        assert_eq!(edit.draft, draft);
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn docked_properties_and_layer_contents_share_sidebar_and_preserve_live_preview() {
        let mut app = app();
        let before = app.project.clone();
        let gid = app.active.unwrap();
        app.run_command("graph.props");
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        egui_extras::install_image_loaders(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.04;
            ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1500.0, 1000.0))),
                    events,
                    ..Default::default()
                },
                |ctx| app.ui_frame(ctx),
            )
        };
        macro_rules! settle {
            () => {
                for _ in 0..6 {
                    frame(&mut app, vec![]);
                }
            };
        }
        macro_rules! dock_left {
            ($id:expr, $y:expr) => {{
                let rect = ctx.memory(|m| m.area_rect($id)).unwrap();
                let start = egui::pos2(rect.center().x, rect.top() + 12.0);
                let end = egui::pos2(150.0, $y);
                let button = |pos, pressed| egui::Event::PointerButton {
                    pos,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: egui::Modifiers::NONE,
                };
                frame(&mut app, vec![egui::Event::PointerMoved(start)]);
                frame(&mut app, vec![button(start, true)]);
                frame(
                    &mut app,
                    vec![egui::Event::PointerMoved(start + egui::vec2(30.0, 0.0))],
                );
                frame(&mut app, vec![egui::Event::PointerMoved(end)]);
                frame(&mut app, vec![button(end, false)]);
                settle!();
            }};
        }
        settle!();
        let props_id = egui::Id::new("scicanvas-props");
        dock_left!(props_id, 700.0);
        assert!(dialogs::has_docks(&ctx, dialogs::DockSide::Left));
        let rect = ctx.memory(|m| m.area_rect(props_id)).unwrap();
        assert!(
            rect.right() <= app.ws.left() + 1.0,
            "props {rect:?}, workspace {:?}",
            app.ws
        );
        app.run_command("layer.contents");
        let Some(Dialog::LayerContents(edit)) = &mut app.dialog else {
            panic!()
        };
        edit.draft.legend = !edit.before.legend;
        let draft = edit.draft.clone();
        settle!();
        let contents_id = egui::Id::new(("scicanvas-dialog", "图层内容"));
        dock_left!(contents_id, 920.0);
        let props = ctx.memory(|m| m.area_rect(props_id)).unwrap();
        let contents = ctx.memory(|m| m.area_rect(contents_id)).unwrap();
        assert!(props.bottom() <= contents.top() + 1.0, "{props:?} {contents:?}");
        assert!(
            contents.right() <= app.ws.left() + 1.0 && contents.bottom() <= 980.0,
            "{contents:?} {:?}",
            app.ws
        );
        assert_eq!(app.preview_graph(gid).unwrap().page.layers()[0], draft);
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
        app.run_command("win.reset");
        settle!();
        assert!(!dialogs::has_docks(&ctx, dialogs::DockSide::Left));
        assert_eq!(app.preview_graph(gid).unwrap().page.layers()[0], draft);
        let Some(Dialog::LayerContents(edit)) = app.dialog.take() else {
            panic!()
        };
        app.apply_layer_contents(&edit).unwrap();
        roundtrip(&mut app, &before);
    }

    #[test]
    fn layer_contents_real_controls_commit_one_target_layer_and_cancel_cleanly() {
        let mut app = app();
        let mut p = Project::default();
        let a = p.new_book();
        let b = p.new_book();
        for id in [a, b] {
            let book = p.book_mut(id).unwrap();
            book.columns[0].data = (0..5).map(|i| i.to_string()).collect();
            book.columns[1].data = (0..5).map(|i| (i * 2).to_string()).collect();
        }
        let (gid, _) = p.plot_from_selection(a, &[], PlotType::Line).unwrap();
        let layer = p.graph(gid).unwrap().first_layer().clone();
        p.graph_mut(gid).unwrap().page.add_layer(layer);
        app.load_project(p, None);
        app.active_layers.insert(gid, 1);
        let before = app.project.clone();
        app.run_command("layer.contents");
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.02;
            ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0))),
                    events,
                    ..Default::default()
                },
                |ctx| app.dialogs(ctx),
            )
        };
        let positions = |out: &egui::FullOutput, label: &str| {
            out.shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + egui::vec2(4.0, 4.0)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        macro_rules! click {
            ($pos:expr) => {{
                let pos = $pos;
                frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
                for pressed in [true, false] {
                    frame(
                        &mut app,
                        vec![egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        }],
                    );
                }
            }};
        }
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        for label in ["添加 →", "← 移除", "上移", "下移", "按名称排序", "确定", "取消"] {
            let pos = positions(&output, label)[0];
            let rect = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if r.rect.contains(pos) => Some(r.rect),
                    _ => None,
                })
                .min_by(|a, b| a.area().total_cmp(&b.area()))
                .unwrap();
            assert_eq!(rect.size(), dialogs::BUTTON, "button {label}");
        }
        click!(*positions(&output, "B (Y)").last().expect("source column"));
        let output = frame(&mut app, vec![]);
        click!(positions(&output, "添加 →")[0]);
        let key = scicanvas_core::plotspec::foreign_key(b, "B");
        assert_eq!(app.project, before, "draft changes do not touch the project");
        assert!(app.preview_graph(gid).unwrap().page.layers()[1].y_keys.contains(&key));
        let output = frame(&mut app, vec![]);
        assert!(!positions(&output, "Book2 / A").is_empty());
        assert!(!positions(&output, "Book2 / B").is_empty());
        let row = positions(&output, "Book2：B")[0];
        let axis = positions(&output, "左 Y")
            .into_iter()
            .min_by(|a, b| (a.y - row.y).abs().total_cmp(&(b.y - row.y).abs()))
            .unwrap();
        click!(axis);
        let output = frame(&mut app, vec![]);
        click!(*positions(&output, "右 Y").last().unwrap());
        let output = frame(&mut app, vec![]);
        click!(positions(&output, "按名称排序")[0]);
        let output = frame(&mut app, vec![]);
        click!(positions(&output, "上移")[0]);
        let Some(Dialog::LayerContents(edit)) = &app.dialog else {
            panic!()
        };
        assert_eq!(edit.draft.y_keys[0], key);
        assert!(edit.draft.curves[&key].right_axis);
        assert_eq!(app.preview_graph(gid).unwrap().page.layers()[1], edit.draft);
        assert_eq!(app.project, before);
        let output = frame(&mut app, vec![]);
        click!(positions(&output, "确定")[0]);
        assert!(app.dialog.is_none());
        assert_eq!(
            app.project.graph(gid).unwrap().first_layer(),
            before.graph(gid).unwrap().first_layer()
        );
        let layer = &app.project.graph(gid).unwrap().page.layers()[1];
        assert_eq!(layer.y_keys, vec![key.clone(), "B".into()]);
        assert!(layer.curves[&key].right_axis);
        let saved = scicanvas_core::project_file::to_json(&app.project).unwrap();
        assert_eq!(scicanvas_core::project_file::from_json(&saved).unwrap(), app.project);
        roundtrip(&mut app, &before);
        let committed = app.project.clone();
        app.run_command("layer.contents");
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        click!(positions(&output, "← 移除")[0]);
        assert_eq!(app.project, committed);
        assert_ne!(
            app.preview_graph(gid).unwrap().as_ref(),
            app.project.graph(gid).unwrap()
        );
        let output = frame(&mut app, vec![]);
        click!(positions(&output, "取消")[0]);
        assert!(app.dialog.is_none());
        assert_eq!(app.project, committed);
        assert_eq!(
            app.preview_graph(gid).unwrap().as_ref(),
            app.project.graph(gid).unwrap()
        );
        app.run_command("layer.contents");
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        click!(positions(&output, "确定")[0]);
        app.run_command("edit.undo");
        assert_eq!(app.project, before, "confirming an unchanged draft adds no undo step");
    }

    #[test]
    fn layer_contents_rejects_stale_drafts_and_supports_removing_all_curves() {
        let mut app = app();
        app.run_command("layer.contents");
        let Some(Dialog::LayerContents(edit)) = &app.dialog else {
            panic!()
        };
        let mut edit = edit.clone();
        let gid = edit.graph;
        app.project.graph_mut(gid).unwrap().page.layers_mut()[edit.index].grid = !edit.before.grid;
        let changed = app.project.clone();
        edit.draft.legend = !edit.before.legend;
        assert!(app.apply_layer_contents(&edit).is_err());
        assert_eq!(app.project, changed);
        assert!(app.history.undo_label().is_none());
        app.run_command("layer.contents");
        let Some(Dialog::LayerContents(edit)) = &app.dialog else {
            panic!()
        };
        let mut edit = edit.clone();
        for key in edit.draft.y_keys.clone() {
            edit.draft.remove_curve(&key);
        }
        app.apply_layer_contents(&edit).unwrap();
        assert!(app.project.graph(gid).unwrap().page.layers()[edit.index]
            .y_keys
            .is_empty());
        roundtrip(&mut app, &changed);
    }

    #[test]
    fn layer_preview_repaints_the_actual_plot_and_cancel_restores_it() {
        let mut app = app();
        let gid = app.project.graphs[0].id;
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let render = |app: &mut SciApp| {
            let mut shapes = Vec::new();
            for _ in 0..3 {
                shapes = ctx
                    .run(egui::RawInput::default(), |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| app.graph_ui(ui, gid));
                    })
                    .shapes;
            }
            shapes
        };
        let original = render(&mut app);
        app.run_command("layer.contents");
        let Some(Dialog::LayerContents(edit)) = &mut app.dialog else {
            panic!()
        };
        edit.draft.remove_curve(&edit.draft.y_keys[0].clone());
        assert_ne!(
            render(&mut app),
            original,
            "the plot, including its cached scene, previews the draft"
        );
        assert_eq!(app.project, before);
        assert_eq!(
            scicanvas_core::project_file::to_json(&app.project).unwrap(),
            scicanvas_core::project_file::to_json(&before).unwrap()
        );
        press_dialog_button(&mut app, "取消");
        assert_eq!(render(&mut app), original);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn column_preview_updates_sheet_and_plot_reuses_data_and_commits_to_original_book() {
        use scicanvas_core::format::NumberFormat;
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        let other = app.project.new_book();
        app.project.book_mut(1).unwrap().columns[1].set(0, "12.3456".into());
        let before = app.project.clone();
        app.run_command("col.props");
        let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
            panic!()
        };
        edit.long_name = "Preview voltage".into();
        edit.units = "preview_unit".into();
        edit.format.number = NumberFormat::from_index(1, 2);
        assert!(app.refresh_book_preview());
        let allocation = |app: &SciApp| {
            let scicanvas_core::ColumnData::Num { values, .. } = &app
                .book_preview
                .as_ref()
                .unwrap()
                .result
                .as_ref()
                .unwrap()
                .column("B")
                .unwrap()
                .data
            else {
                panic!()
            };
            values.as_ptr()
        };
        let ptr = allocation(&app);
        assert!(!app.refresh_book_preview());
        let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
            panic!()
        };
        edit.long_name.push_str(" live");
        assert!(app.refresh_book_preview());
        assert_eq!(allocation(&app), ptr, "typing and display formats reuse preview data");
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let has_text = |out: &egui::FullOutput, text: &str| {
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text.contains(text)))
        };
        let mut sheet = egui::FullOutput::default();
        let mut graph = egui::FullOutput::default();
        let gid = app.project.graphs[0].id;
        for _ in 0..3 {
            sheet = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
            });
            graph = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.graph_ui(ui, gid));
            });
        }
        assert!(has_text(&sheet, "Preview voltage live"));
        assert!(has_text(&sheet, "12.35"));
        assert!(has_text(&graph, "Preview voltage live"));
        assert!(has_text(&graph, "preview_unit"));
        assert_eq!(app.project, before, "read-only plotting restores the canonical table");
        app.active = Some(other);
        press_dialog_button(&mut app, "确定");
        assert!(app.dialog.is_none());
        assert!(app.book_preview.is_none());
        assert_eq!(
            app.project.book(1).unwrap().column("B").unwrap().long_name,
            "Preview voltage live"
        );
        assert_eq!(app.project.book(other), before.book(other));
        roundtrip(&mut app, &before);
        app.active = Some(1);
        app.run_command("col.props");
        let committed = app.project.clone();
        let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
            panic!()
        };
        edit.long_name = "discard".into();
        app.refresh_book_preview();
        press_dialog_button(&mut app, "取消");
        assert!(app.book_preview.is_none());
        assert_eq!(app.project, committed);
    }

    #[test]
    fn typed_column_category_and_sort_previews_validate_and_remain_undoable() {
        use scicanvas_core::DataKind;
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into()]);
        app.project.book_mut(1).unwrap().columns[0].data = vec!["b".into(), "a".into(), "c".into()].into();
        app.run_command("col.props");
        let before = app.project.clone();
        let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
            panic!()
        };
        edit.kind = DataKind::Num;
        app.refresh_book_preview();
        assert!(app.book_preview.as_ref().unwrap().result.is_err());
        assert_eq!(app.project, before);
        let Some(Dialog::ColumnProps { edit, .. }) = &mut app.dialog else {
            panic!()
        };
        edit.kind = DataKind::Category;
        app.refresh_book_preview();
        assert_eq!(
            app.book_preview.as_ref().unwrap().result.as_ref().unwrap().columns[0]
                .data
                .kind(),
            DataKind::Category
        );
        press_dialog_button(&mut app, "确定");
        roundtrip(&mut app, &before);
        let before = app.project.clone();
        app.run_command("col.categories");
        let Some(Dialog::CategoryOrder { levels, .. }) = &mut app.dialog else {
            panic!()
        };
        levels.reverse();
        let levels = levels.clone();
        app.refresh_book_preview();
        let book = app.book_preview.as_ref().unwrap().result.as_ref().unwrap();
        assert_eq!(book.columns[0].data.category_levels().unwrap(), levels);
        assert_eq!(
            book.columns[0].data.text_at(0),
            "b",
            "reordering categories preserves cell labels"
        );
        assert_eq!(app.project, before);
        press_dialog_button(&mut app, "确定");
        roundtrip(&mut app, &before);
        app.run_command("sort.custom");
        app.refresh_book_preview();
        let sorted = app.book_preview.as_ref().unwrap().result.as_ref().unwrap().clone();
        let before = app.project.clone();
        assert_ne!(&sorted, before.book(1).unwrap());
        press_dialog_button(&mut app, "取消");
        assert_eq!(app.project, before);
        app.run_command("sort.custom");
        let Some(Dialog::Sort { keys, .. }) = &mut app.dialog else {
            panic!()
        };
        *keys = [(None, true); 3];
        app.refresh_book_preview();
        assert!(app.book_preview.as_ref().unwrap().result.is_err());
        let Some(Dialog::Sort { keys, .. }) = &mut app.dialog else {
            panic!()
        };
        keys[0] = (Some(0), true);
        app.refresh_book_preview();
        assert!(app.book_preview.as_ref().unwrap().result.is_ok());
        press_dialog_button(&mut app, "排序");
        assert_eq!(app.project.book(1), Some(&sorted));
        roundtrip(&mut app, &before);
    }

    #[test]
    fn graph_analysis_curve_dialog_selects_foreign_source_and_history_is_atomic() {
        let mut app = app();
        let mut p = Project::default();
        let a = p.new_book();
        let b = p.new_book();
        for (id, slope) in [(a, 2.0), (b, 3.0)] {
            let book = p.book_mut(id).unwrap();
            book.columns[0].data = (1..13).map(|x| x.to_string()).collect();
            book.columns[1].data = (1..13).map(|x| (slope * x as f64 + 1.0).to_string()).collect();
        }
        let (gid, _) = p.plot_from_selection(a, &[], PlotType::LineSymbol).unwrap();
        p.add_curves(gid, 0, b, &["B".into()]).unwrap();
        app.load_project(p, None);
        let before = app.project.clone();
        app.run_command("analysis.curves");
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.02;
            ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0))),
                    events,
                    ..Default::default()
                },
                |ctx| app.dialogs(ctx),
            )
        };
        let at = |output: &egui::FullOutput, label: &str| {
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + egui::vec2(4.0, 4.0)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing {label}"))
        };
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        let pos = at(&output, "Book1：B");
        for pressed in [true, false] {
            frame(
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        let pos = at(&frame(&mut app, vec![]), "应用");
        for pressed in [true, false] {
            frame(
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(app.dialog.is_none());
        assert!(app.history.undo_label().is_none());
        assert_eq!(
            app.project.resolve(&app.analysis_source().unwrap()).unwrap().items[0].book_id,
            b
        );
        app.run_command("fit.linear");
        assert_eq!(app.project.book(a), before.book(a));
        assert!(app.project.book(b).unwrap().columns.len() > before.book(b).unwrap().columns.len());
        let Some(Dialog::FitReport(reports)) = &app.dialog else {
            panic!("{}", app.status)
        };
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].book_id, b);
        assert!((reports[0].fit.coef[1] - 3.0).abs() < 1e-10);
        roundtrip(&mut app, &before);
        app.analysis_curves.clear();
        app.anova();
        let Some(Dialog::Anova { names, .. }) = &app.dialog else {
            panic!("{}", app.status)
        };
        assert_eq!(names, &["Book1：B", "Book2：B"]);
        app.correlations();
        assert!(matches!(&app.dialog, Some(Dialog::Correlations(pairs)) if pairs.len() == 1));
    }

    #[test]
    fn columns_from_another_workbook_join_a_graph_and_leave_it_in_one_undo_step() {
        let mut app = app();
        let b = app.project.new_book();
        let book = app.project.book_mut(b).unwrap();
        book.columns[0].data = (0..4).map(|i| i.to_string()).collect();
        book.columns[1].data = (0..4).map(|i| (i * i).to_string()).collect();
        let key = scicanvas_core::plotspec::foreign_key(b, "B");
        app.selection.insert(b, vec!["A".into(), "B".into()]);
        let before = app.project.clone();
        app.add_to_graph(b, 2);
        assert!(app.project.graph(2).unwrap().first_layer().y_keys.contains(&key));
        assert_eq!(app.status, "已添加 1 条曲线");
        roundtrip(&mut app, &before);
        let before = app.project.clone();
        app.remove_curve(2, 0, &key);
        assert!(!app.project.graph(2).unwrap().first_layer().y_keys.contains(&key));
        roundtrip(&mut app, &before);
        // 已在图层里的列不再记一步
        app.add_to_graph(b, 2);
        let before = app.project.clone();
        app.add_to_graph(b, 2);
        assert_eq!(app.project, before);
        assert_eq!(app.status, "这些列已在图层里");
    }

    #[test]
    fn embedded_plot_examples_preserve_current_project_and_undo_as_one_step() {
        assert_eq!(crate::examples::ALL.len(), PlotType::ALL.len());
        for kind in PlotType::ALL {
            let example = crate::examples::ALL.iter().find(|e| e.kind == kind).unwrap();
            assert!(commands::find(example.plot_command).is_some());
            let mut app = app();
            let folder = app.project.add_folder(None).unwrap();
            app.explorer_target = Some(folder);
            app.project_path = Some(PathBuf::from("user-project.scv"));
            let before = app.project.clone();
            app.run_command(example.command);
            assert_eq!(&app.project.books[..before.books.len()], &before.books);
            assert_eq!(&app.project.graphs[..before.graphs.len()], &before.graphs);
            let book = app.project.books.last().unwrap();
            let graph = app.project.graphs.last().unwrap();
            assert_eq!(graph.first_layer().plot_type, kind);
            assert_eq!(graph.first_layer().book_id, Some(book.id));
            assert_eq!(app.project.folder_of(book.id), Some(folder));
            assert_eq!(app.project.folder_of(graph.id), Some(folder));
            assert!(book.columns.iter().any(|c| !c.comment.is_empty()));
            assert_eq!(app.active, Some(graph.id));
            assert_eq!(app.project_path, Some(PathBuf::from("user-project.scv")));
            scicanvas_core::project_file::to_scv(&app.project).unwrap();
            roundtrip(&mut app, &before);
            app.run_command(example.command);
            let names: Vec<_> = app.project.books.iter().map(|b| &b.name).collect();
            assert_ne!(names[names.len() - 1], names[names.len() - 2]);
        }
    }

    #[test]
    fn ribbon_hover_example_button_is_clickable_even_without_plot_data() {
        for empty in [false, true] {
            let mut app = app();
            if empty {
                app.load_project(Project::default(), None);
            }
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            ctx.style_mut(|s| s.interaction.tooltip_delay = 0.0);
            let mut state = UiState::default();
            state.tab = "plot";
            let mut time = 0.0;
            let mut frame = |events| {
                time += 0.02;
                let mut clicked = None;
                let output = ctx.run(
                    egui::RawInput {
                        time: Some(time),
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1800.0, 600.0))),
                        events,
                        ..Default::default()
                    },
                    |ctx| ribbon::show(ctx, &app, &mut state, &app.pal, &mut clicked),
                );
                (output, clicked)
            };
            let at = |out: &egui::FullOutput, label: &str| {
                out.shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + egui::vec2(5.0, 5.0)),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing label {label}"))
            };
            frame(vec![]);
            let tile = at(&frame(vec![]).0, "折线图");
            frame(vec![egui::Event::PointerMoved(tile)]);
            frame(vec![]);
            let output = frame(vec![]).0;
            let button = at(&output, "打开示例");
            let hovered = frame(vec![egui::Event::PointerMoved(button)]).0;
            let button = at(&hovered, "打开示例");
            frame(vec![egui::Event::PointerMoved(button)]);
            let mut command = None;
            for pressed in [true, false] {
                let (_, clicked) = frame(vec![egui::Event::PointerButton {
                    pos: button,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }]);
                command = command.or(clicked);
            }
            assert_eq!(command, Some("example.line"));
            app.run_command(command.unwrap());
            assert_eq!(
                app.project.graphs.last().unwrap().first_layer().plot_type,
                PlotType::Line
            );
        }
    }

    struct ExplorerUi {
        ctx: egui::Context,
        time: f64,
        modifiers: egui::Modifiers,
        /// Draws the explorer as a side panel followed by this active sheet, as the app does.
        sheet: Option<SheetState>,
    }

    impl ExplorerUi {
        fn new() -> Self {
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            ctx.style_mut(|s| s.animation_time = 0.0);
            Self {
                ctx,
                time: 0.0,
                modifiers: egui::Modifiers::NONE,
                sheet: None,
            }
        }

        fn frame(&mut self, app: &mut SciApp, events: Vec<egui::Event>) -> egui::FullOutput {
            self.time += 0.02;
            self.ctx.run(
                egui::RawInput {
                    time: Some(self.time),
                    modifiers: self.modifiers,
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ctx| match &mut self.sheet {
                    Some(sheet) => {
                        egui::SidePanel::left("explorer")
                            .exact_width(190.0)
                            .show(ctx, |ui| app.explorer(ui));
                        app.sheet_keys(ctx, 1, sheet);
                    }
                    None => {
                        egui::CentralPanel::default().show(ctx, |ui| app.explorer(ui));
                    }
                },
            )
        }

        fn at(&mut self, app: &mut SciApp, name: &str) -> egui::Pos2 {
            self.frame(app, vec![]);
            self.frame(app, vec![])
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.job.text == name => Some(t.pos + egui::vec2(4.0, 4.0)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing explorer label: {name}"))
        }

        fn button(&mut self, app: &mut SciApp, pos: egui::Pos2, pressed: bool) {
            self.frame(
                app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }

        fn click(&mut self, app: &mut SciApp, pos: egui::Pos2) {
            self.button(app, pos, true);
            self.button(app, pos, false);
        }

        fn key(&mut self, app: &mut SciApp, key: egui::Key) {
            self.frame(
                app,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            self.frame(
                app,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }

        fn drag(&mut self, app: &mut SciApp, source: &str, destination: &str) {
            let from = self.at(app, source);
            let to = self.at(app, destination);
            self.frame(app, vec![egui::Event::PointerMoved(from)]);
            self.button(app, from, true);
            self.frame(app, vec![egui::Event::PointerMoved(from + egui::vec2(12.0, 0.0))]);
            self.frame(app, vec![egui::Event::PointerMoved(to)]);
            self.button(app, to, false);
            self.frame(app, vec![]);
        }
    }

    #[test]
    fn explorer_slow_second_click_renames_folders_books_and_graphs_with_history() {
        for id in [1, 2, 3] {
            let mut app = app();
            assert_eq!(app.project.add_folder(None).unwrap(), 3);
            let before = app.project.clone();
            let name = app.project.name_of(id).unwrap().to_owned();
            let mut ui = ExplorerUi::new();
            let at = ui.at(&mut app, &name);
            ui.click(&mut app, at);
            ui.time += 0.7;
            ui.click(&mut app, at);
            assert_eq!(app.explorer_rename.map(|r| r.0), Some(id));
            ui.frame(&mut app, vec![]);
            ui.frame(&mut app, vec![egui::Event::Text("重命名 α".into())]);
            ui.key(&mut app, egui::Key::Enter);
            assert_eq!(app.project.name_of(id), Some("重命名 α"));
            assert!(app.explorer_rename.is_none());
            roundtrip(&mut app, &before);
        }
    }

    #[test]
    fn explorer_fast_double_click_escape_and_focus_commit_have_distinct_behavior() {
        let mut app = app();
        let before = app.project.clone();
        let mut ui = ExplorerUi::new();
        let at = ui.at(&mut app, "Book1");
        ui.click(&mut app, at);
        ui.click(&mut app, at);
        assert!(app.explorer_rename.is_none());
        ui.key(&mut app, egui::Key::F2);
        ui.frame(&mut app, vec![egui::Event::Text("取消的名称".into())]);
        ui.key(&mut app, egui::Key::Escape);
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
        ui.key(&mut app, egui::Key::F2);
        ui.key(&mut app, egui::Key::Backspace);
        ui.key(&mut app, egui::Key::Enter);
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
        ui.key(&mut app, egui::Key::F2);
        ui.frame(&mut app, vec![egui::Event::Text("失焦提交".into())]);
        let root = ui.at(&mut app, "项目根目录");
        ui.click(&mut app, root);
        assert_eq!(app.project.name_of(1), Some("失焦提交"));
        assert_eq!(app.explorer_target, None);
        roundtrip(&mut app, &before);
    }

    #[test]
    fn f2_goes_to_the_focused_explorer_or_sheet_but_not_both() {
        let mut app = app();
        app.active = Some(1);
        app.explorer_target = Some(1);
        let mut ui = ExplorerUi::new();
        ui.sheet = Some(SheetState {
            range: Some(Range::single(0, 0)),
            ..Default::default()
        });
        ui.click(&mut app, egui::pos2(300.0, 300.0));
        ui.key(&mut app, egui::Key::F2);
        assert!(app.explorer_rename.is_none());
        assert!(ui.sheet.as_mut().unwrap().edit.take().is_some());

        let at = ui.at(&mut app, "Book1");
        ui.click(&mut app, at);
        ui.key(&mut app, egui::Key::F2);
        assert_eq!(app.explorer_rename.map(|r| r.0), Some(1));
        assert!(ui.sheet.as_ref().unwrap().edit.is_none());
    }

    #[test]
    fn sheet_keys_only_apply_after_clicking_the_sheet_not_the_explorer() {
        let mut app = app();
        app.active = Some(1);
        app.explorer_target = Some(1);
        let before = app.project.clone();
        let mut ui = ExplorerUi::new();
        ui.sheet = Some(SheetState {
            range: Some(Range::single(0, 0)),
            ..Default::default()
        });
        let at = ui.at(&mut app, "Book1");
        ui.click(&mut app, at);
        ui.key(&mut app, egui::Key::Delete);
        ui.key(&mut app, egui::Key::ArrowDown);
        ui.frame(&mut app, vec![egui::Event::Text("9".into())]);
        assert_eq!(app.project, before);
        let sheet = ui.sheet.as_ref().unwrap();
        assert_eq!(sheet.range, Some(Range::single(0, 0)));
        assert!(sheet.edit.is_none());

        ui.click(&mut app, egui::pos2(300.0, 300.0));
        ui.key(&mut app, egui::Key::Delete);
        assert_ne!(app.project, before);
        app.run_command("edit.undo");
        assert_eq!(app.project, before);
    }

    #[test]
    fn explorer_new_table_and_graph_buttons_use_folder_and_history() {
        for (label, command) in [("数据表", "book.new"), ("图形", "graph.new")] {
            let mut app = app();
            let folder = app.project.add_folder(None).unwrap();
            app.explorer_target = Some(folder);
            let before = app.project.clone();
            let mut ui = ExplorerUi::new();
            // 新建命令合并在“＋ 新建”菜单里：先打开菜单再点菜单项
            let pos = ui.at(&mut app, "＋ 新建");
            ui.click(&mut app, pos);
            let pos = ui.at(&mut app, label);
            ui.click(&mut app, pos);
            let id = app.active.unwrap();
            assert_eq!(app.project.folder_of(id), Some(folder));
            assert!(app.open[&id]);
            if command == "graph.new" {
                let graph = app.project.graph(id).unwrap();
                assert_eq!(graph.first_layer().book_id, None);
                assert_eq!(app.project.books, before.books);
            } else {
                assert!(app.project.book(id).is_some());
            }
            roundtrip(&mut app, &before);
        }
        let mut app = app();
        app.load_project(Project::default(), None);
        app.run_command("graph.new");
        assert_eq!(app.project.graphs.len(), 1);
        assert!(app.project.books.is_empty());
    }

    #[test]
    fn explorer_shift_ranges_follow_sort_order_and_skip_collapsed_contents() {
        let mut app = app();
        let note = app.project.add_note("M-Note", "结果".into());
        app.project.rename_project_item(1, "Z-Table").unwrap();
        app.project.rename_project_item(2, "A-Graph").unwrap();
        let mut ui = ExplorerUi::new();
        let table_pos = ui.at(&mut app, "Z-Table");
        ui.click(&mut app, table_pos);
        assert_eq!(app.explorer_visible, vec![1, 2, note]);
        ui.modifiers.shift = true;
        let note_pos = ui.at(&mut app, "M-Note1");
        ui.click(&mut app, note_pos);
        assert_eq!(app.explorer_selection, vec![1, 2, note]);
        let graph_pos = ui.at(&mut app, "A-Graph");
        ui.click(&mut app, graph_pos);
        assert_eq!(app.explorer_selection, vec![1, 2]);
        ui.modifiers.ctrl = true;
        ui.click(&mut app, note_pos);
        assert_eq!(app.explorer_selection, vec![1, 2, note]);
        assert!(app.explorer_rename.is_none());
        ui.modifiers = egui::Modifiers::NONE;
        app.explorer_sort = workspace::ExplorerSort::Name;
        ui.frame(&mut app, vec![]);
        assert_eq!(app.explorer_visible, vec![2, note, 1]);
        let folder = app.project.add_folder(None).unwrap();
        app.project.move_project_item(1, Some(folder)).unwrap();
        let mut collapsed = egui::collapsing_header::CollapsingState::load_with_default_open(
            &ui.ctx,
            egui::Id::new(("project-folder", folder)),
            false,
        );
        collapsed.set_open(false);
        collapsed.store(&ui.ctx);
        ui.frame(&mut app, vec![]);
        assert_eq!(app.explorer_visible, vec![folder, 2, note]);
        let name = app.project.name_of(folder).unwrap().to_owned();
        let folder_pos = ui.at(&mut app, &name);
        ui.click(&mut app, folder_pos);
        ui.modifiers.shift = true;
        let note_pos = ui.at(&mut app, "M-Note1");
        ui.click(&mut app, note_pos);
        assert_eq!(app.explorer_selection, vec![folder, 2, note]);
        assert!(app.history.undo_label().is_none());
    }

    /// 渲染当前对话框并点击文字为 `label` 的按钮。
    fn press_dialog_button(app: &mut SciApp, label: &str) {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.02;
            ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0))),
                    events,
                    ..Default::default()
                },
                |ctx| app.dialogs(ctx),
            )
        };
        // Narrow grids need several frames to settle their column widths and wrapped rows.
        for _ in 0..5 {
            frame(app, vec![]);
        }
        let output = frame(app, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + egui::vec2(5.0, 5.0)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing button {label}"));
        frame(app, vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(
                app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
    }

    #[test]
    fn fit_reports_add_confidence_and_prediction_bands_in_one_step() {
        for nonlinear in [false, true] {
            let mut app = app();
            app.active = Some(1);
            app.selection.insert(1, vec!["C".into()]);
            if nonlinear {
                app.nlfit(scicanvas_core::nlfit::Model::PseudoFirstOrder, &[], &[]);
            } else {
                app.fit(1);
            }
            let before = app.project.clone();
            let columns = app.project.book(1).unwrap().columns.len();
            press_dialog_button(&mut app, "置信/预测带");
            let book = app.project.book(1).unwrap();
            assert_eq!(book.columns.len(), columns + 4, "{}", app.status);
            let names: Vec<&str> = book.columns[columns..].iter().map(|c| c.long_name.as_str()).collect();
            assert!(
                names.iter().any(|n| n.ends_with("95% 置信下限")) && names.iter().any(|n| n.ends_with("95% 预测上限"))
            );
            // 结果对话框保留
            assert!(app.dialog.is_some());
            roundtrip(&mut app, &before);
        }
    }

    #[test]
    fn nlfit_dialog_linear_constraint_holds_and_bad_text_keeps_the_dialog() {
        use scicanvas_core::nlfit::Model;
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.run_command("fit.nonlinear");
        let Some(Dialog::NlFit { model, constraints, .. }) = app.dialog.as_mut() else {
            panic!("nlfit dialog not open");
        };
        *model = Model::ExpDec1;
        *constraints = "y0 + A1 = q".into();
        press_dialog_button(&mut app, "拟合");
        assert!(matches!(app.dialog, Some(Dialog::NlFit { .. })), "dialog kept");
        assert!(app.status.contains("不是参数名"), "{}", app.status);
        if let Some(Dialog::NlFit { constraints, .. }) = app.dialog.as_mut() {
            *constraints = "y0 + A1 = 0".into();
        }
        press_dialog_button(&mut app, "拟合");
        let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
            panic!("no report: {}", app.status);
        };
        let f = &reports[0].fit;
        assert!((f.params[0] + f.params[1]).abs() < 1e-9, "{:?}", f.params);
        assert_eq!(f.constraints, ["y0 + A1 = 0"]);
    }

    #[test]
    fn nlfit_dialog_upper_bound_stops_the_parameter_and_marks_it() {
        use scicanvas_core::nlfit::Model;
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.run_command("fit.nonlinear");
        let Some(Dialog::NlFit { model, limits, .. }) = app.dialog.as_mut() else {
            panic!("nlfit dialog not open");
        };
        // C 列是 8(1 − e^{−t/4}) 加噪声；ExpDec1 的 t1 约为 4，限制 t1 ≤ 3
        *model = Model::ExpDec1;
        limits[2].4 = true;
        limits[2].5 = 3.0;
        press_dialog_button(&mut app, "拟合");
        let Some(Dialog::NlFitReport(reports)) = &app.dialog else {
            panic!("no report: {}", app.status);
        };
        let f = &reports[0].fit;
        assert_eq!(f.params[2], 3.0);
        assert_eq!(f.at_bound, vec![false, false, true]);
    }

    #[test]
    fn category_order_dialog_reorders_levels_in_one_step() {
        let mut app = app();
        let b = app.project.book_mut(1).unwrap();
        b.columns[2].data = ["中", "低", "高"].iter().map(|s| s.to_string()).collect();
        b.convert_columns(&["C".into()], scicanvas_core::DataKind::Category)
            .unwrap();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.run_command("col.categories");
        let Some(Dialog::CategoryOrder { levels, .. }) = app.dialog.as_mut() else {
            panic!("dialog not open: {}", app.status);
        };
        *levels = vec!["低".into(), "中".into(), "高".into()];
        let before = app.project.clone();
        press_dialog_button(&mut app, "确定");
        assert!(app.dialog.is_none(), "{}", app.status);
        let data = &app.project.book(1).unwrap().columns[2].data;
        assert_eq!(data.category_levels().unwrap(), ["低", "中", "高"]);
        assert_eq!(data.text_at(0), "中");
        roundtrip(&mut app, &before);
        // 非分类列给出提示
        app.selection.insert(1, vec!["B".into()]);
        app.run_command("col.categories");
        assert!(app.status.contains("不是分类列"), "{}", app.status);
    }

    #[test]
    fn custom_sort_dialog_sorts_by_two_keys_or_only_selected_columns() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        app.run_command("sort.custom");
        let Some(Dialog::Sort {
            keys, only_selected, ..
        }) = app.dialog.as_mut()
        else {
            panic!("sort dialog not open");
        };
        // 主关键字默认为选中的 B 列；改为降序、只排 B 列
        assert_eq!(keys[0], (Some(1), true));
        keys[0].1 = false;
        *only_selected = true;
        let before = app.project.clone();
        let a_before = app.project.book(1).unwrap().columns[0].values();
        press_dialog_button(&mut app, "排序");
        assert!(app.dialog.is_none(), "{}", app.status);
        let book = app.project.book(1).unwrap();
        let b = book.columns[1].values();
        assert!(b.windows(2).all(|w| w[0] >= w[1]));
        assert_eq!(book.columns[0].values(), a_before);
        roundtrip(&mut app, &before);
    }

    #[test]
    fn ifft_dialog_finds_fft_columns_and_creates_a_signal_table() {
        let mut app = app();
        let src = scicanvas_core::ops::Source::Book {
            id: 1,
            selected: vec!["B".into()],
            rows: None,
        };
        let (spec, _) = app.project.fft(&src, scicanvas_core::fft::Window::Rect).unwrap()[0];
        app.active = Some(spec);
        app.run_command("signal.ifft");
        let Some(Dialog::Ifft { re, im, rows, .. }) = &app.dialog else {
            panic!("IFFT dialog not open: {}", app.status);
        };
        assert!(re.1.contains("实部") && im.as_ref().unwrap().1.contains("虚部"));
        let m = *rows;
        let before = app.project.clone();
        press_dialog_button(&mut app, "变换");
        let out = app.project.books.last().unwrap();
        assert!(out.name.contains("IFFT"), "{}", app.status);
        assert_eq!(out.columns[1].values().len(), 2 * (m - 1));
        roundtrip(&mut app, &before);
    }

    #[test]
    fn sg_derivative_dialog_writes_second_derivative_column_and_undoes() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        app.run_command("math.diff_sg");
        if let Some(Dialog::SgDiff { deriv, .. }) = app.dialog.as_mut() {
            *deriv = 2;
        } else {
            panic!("SG dialog not open");
        }
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.02;
            ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0))),
                    events,
                    ..Default::default()
                },
                |ctx| app.dialogs(ctx),
            )
        };
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "求导" => Some(t.pos + egui::vec2(5.0, 5.0)),
                _ => None,
            })
            .expect("OK button");
        frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(
                &mut app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        assert!(app.dialog.is_none());
        let book = app.project.book(1).unwrap();
        let col = book
            .columns
            .iter()
            .find(|c| c.long_name.starts_with("d²("))
            .expect("derivative column");
        assert!(col.comment.contains("Savitzky–Golay，7 点 2 阶"), "{}", col.comment);
        roundtrip(&mut app, &before);
    }

    #[test]
    fn fit_report_residual_plot_button_creates_scatter_and_undoes() {
        let mut app = app();
        app.active = Some(1);
        app.fit(1);
        let before = app.project.clone();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|style| style.animation_time = 0.0);
        let mut time = 0.0;
        let mut frame = |app: &mut SciApp, events| {
            time += 0.02;
            ctx.run(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0))),
                    events,
                    ..Default::default()
                },
                |ctx| app.dialogs(ctx),
            )
        };
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "残差图" => Some(t.pos + egui::vec2(5.0, 5.0)),
                _ => None,
            })
            .expect("residual plot button");
        frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(
                &mut app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        let graph = app.project.graphs.last().unwrap();
        assert_eq!(app.active, Some(graph.id));
        assert_eq!(graph.first_layer().plot_type, PlotType::Scatter);
        let book = app.project.book(1).unwrap();
        for key in &graph.first_layer().y_keys {
            assert!(book.column(key).unwrap().long_name.ends_with("残差"));
        }
        // 对话框保留，可以继续保存或复制
        assert!(app.dialog.is_some());
        roundtrip(&mut app, &before);
    }

    #[test]
    fn fit_report_save_note_button_persists_report_and_undoes() {
        for nonlinear in [false, true] {
            let mut app = app();
            app.active = Some(1);
            if nonlinear {
                app.nlfit(scicanvas_core::nlfit::Model::ALL[0], &[], &[]);
            } else {
                app.fit(1);
            }
            assert!(app.dialog.is_some(), "nonlinear={nonlinear}: {}", app.status);
            let before = app.project.clone();
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            ctx.style_mut(|style| style.animation_time = 0.0);
            let mut time = 0.0;
            let mut frame = |app: &mut SciApp, events| {
                time += 0.02;
                ctx.run(
                    egui::RawInput {
                        time: Some(time),
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0))),
                        events,
                        ..Default::default()
                    },
                    |ctx| app.dialogs(ctx),
                )
            };
            frame(&mut app, vec![]);
            frame(&mut app, vec![]);
            let output = frame(&mut app, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == "保存为 Note" => Some(t.pos + egui::vec2(5.0, 5.0)),
                    _ => None,
                })
                .expect("save report button");
            frame(&mut app, vec![egui::Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                frame(
                    &mut app,
                    vec![egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            let note = app.project.notes.last().expect("saved Note");
            assert!(note.text.contains("R"));
            // 保存为 Markdown：标题、参数表和统计量表
            assert!(note.text.starts_with("## "), "{}", note.text);
            assert!(
                note.text
                    .contains("| 参数 | 值 | 标准误 | t | p |\n| --- | --- | --- | --- | --- |"),
                "{}",
                note.text
            );
            assert!(note.text.contains("| 统计量 | 值 |"), "{}", note.text);
            assert!(!note.text.contains('\t'));
            assert_eq!(app.active, Some(note.id));
            assert!(app.dialog.is_none());
            let saved =
                scicanvas_core::project_file::from_scv(&scicanvas_core::project_file::to_scv(&app.project).unwrap())
                    .unwrap();
            assert_eq!(saved.notes, app.project.notes);
            roundtrip(&mut app, &before);
        }
    }

    #[test]
    fn explorer_ctrl_click_toggles_selection_without_rename_or_data_changes() {
        let mut app = app();
        let folder = app.project.add_folder(None).unwrap();
        let before = app.project.clone();
        let mut ui = ExplorerUi::new();
        let book = ui.at(&mut app, "Book1");
        ui.click(&mut app, book);
        assert_eq!(app.explorer_selection, vec![1]);
        let graph = ui.at(&mut app, "Graph1");
        ui.modifiers.ctrl = true;
        ui.click(&mut app, graph);
        assert_eq!(app.explorer_selection, vec![1, 2]);
        let name = app.project.name_of(folder).unwrap().to_owned();
        let folder_pos = ui.at(&mut app, &name);
        ui.click(&mut app, folder_pos);
        assert_eq!(app.explorer_selection, vec![1, 2, folder]);
        ui.time += 0.7;
        ui.click(&mut app, folder_pos);
        assert_eq!(app.explorer_selection, vec![1, 2]);
        assert!(app.explorer_rename.is_none());
        ui.click(&mut app, graph);
        assert_eq!(app.explorer_selection, vec![1]);
        ui.click(&mut app, book);
        assert!(app.explorer_selection.is_empty());
        assert_eq!(app.explorer_target, None);
        ui.modifiers = egui::Modifiers::NONE;
        ui.click(&mut app, graph);
        assert_eq!(app.explorer_selection, vec![2]);
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn explorer_multi_drag_keeps_folder_contents_and_undoes_in_one_step() {
        let mut app = app();
        let parent = app.project.add_folder(None).unwrap();
        let destination = app.project.add_folder(None).unwrap();
        app.project.rename_project_item(parent, "来源").unwrap();
        app.project.rename_project_item(destination, "目标").unwrap();
        app.project.move_project_item(1, Some(parent)).unwrap();
        let before = app.project.clone();
        let mut ui = ExplorerUi::new();
        let source = ui.at(&mut app, "来源");
        ui.click(&mut app, source);
        ui.modifiers.ctrl = true;
        let book = ui.at(&mut app, "Book1");
        ui.click(&mut app, book);
        let graph = ui.at(&mut app, "Graph1");
        ui.click(&mut app, graph);
        ui.modifiers = egui::Modifiers::NONE;
        ui.drag(&mut app, "Graph1", "目标");
        assert_eq!(app.project.folder(parent).unwrap().parent, Some(destination));
        assert_eq!(app.project.folder_of(1), Some(parent));
        assert_eq!(app.project.folder_of(2), Some(destination));
        roundtrip(&mut app, &before);
        // 有一个目标会形成目录循环时，整批移动保持原样。
        app.explorer_selection = vec![2, destination];
        app.explorer_target = Some(2);
        app.folder_destination = Some(parent);
        let before = app.project.clone();
        let label = app.history.undo_label().map(str::to_owned);
        app.run_command("project.move");
        assert_eq!(app.project, before);
        assert_eq!(app.history.undo_label().map(str::to_owned), label);
    }

    #[test]
    fn explorer_drag_moves_all_object_types_to_folders_and_root_with_history() {
        for id in [1, 2, 3] {
            let mut app = app();
            assert_eq!(app.project.add_folder(None).unwrap(), 3);
            let destination = app.project.add_folder(None).unwrap();
            app.project.rename_project_item(destination, "目标").unwrap();
            let source = app.project.name_of(id).unwrap().to_owned();
            let before = app.project.clone();
            let mut ui = ExplorerUi::new();
            ui.drag(&mut app, &source, "目标");
            let parent = if id == 3 {
                app.project.folder(id).unwrap().parent
            } else {
                app.project.folder_of(id)
            };
            assert_eq!(parent, Some(destination));
            roundtrip(&mut app, &before);
            ui.drag(&mut app, &source, "项目根目录");
            assert_eq!(app.project, before);
            app.run_command("edit.undo");
            let parent = if id == 3 {
                app.project.folder(id).unwrap().parent
            } else {
                app.project.folder_of(id)
            };
            assert_eq!(parent, Some(destination));
        }
    }

    #[test]
    fn explorer_invalid_folder_drop_and_same_parent_drop_leave_history_unchanged() {
        let mut app = app();
        let parent = app.project.add_folder(None).unwrap();
        let child = app.project.add_folder(Some(parent)).unwrap();
        app.project.rename_project_item(parent, "上级").unwrap();
        app.project.rename_project_item(child, "下级").unwrap();
        let before = app.project.clone();
        let mut ui = ExplorerUi::new();
        ui.drag(&mut app, "上级", "下级");
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
        ui.drag(&mut app, "上级", "上级");
        ui.drag(&mut app, "Book1", "项目根目录");
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn workspace_folder_commands_share_history_and_keep_window_actions_in_session() {
        let mut app = app();
        app.explorer_target = None;
        let before = app.project.clone();
        app.run_command("project.folder.new");
        let folder = app.explorer_target.unwrap();
        roundtrip(&mut app, &before);
        app.explorer_target = Some(folder);
        app.project_name = "实验一".into();
        let before = app.project.clone();
        app.run_command("project.rename");
        roundtrip(&mut app, &before);
        app.explorer_target = Some(folder);
        app.run_command("book.new");
        let book = app.active.unwrap();
        assert_eq!(app.project.folder_of(book), Some(folder));
        app.explorer_target = Some(2);
        app.folder_destination = Some(folder);
        let before = app.project.clone();
        app.run_command("project.move");
        roundtrip(&mut app, &before);
        app.explorer_target = Some(folder);
        let before = app.project.clone();
        app.run_command("project.folder.remove");
        assert!(app.project.book(book).is_some());
        roundtrip(&mut app, &before);
        app.history = History::default();
        app.explorer_target = Some(book);
        let before = app.project.clone();
        for (cmd, state) in [
            ("window.minimize", WinState::Min),
            ("window.maximize", WinState::Max),
            ("window.restore", WinState::Normal),
        ] {
            app.run_command(cmd);
            assert_eq!(app.geoms[&book].state, state);
        }
        app.run_command("project.hide");
        assert!(!app.open[&book]);
        app.run_command("project.show");
        assert!(app.open[&book]);
        assert_eq!(app.active, Some(book));
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn workspace_command_ids_and_theme_actions_are_consistent() {
        let mut ids = std::collections::BTreeSet::new();
        for c in commands::commands() {
            assert!(ids.insert(c.id), "重复命令 {}", c.id);
            assert!(!c.label.is_empty());
        }
        let mut app = app();
        let before = app.project.clone();
        for (cmd, pref) in [
            ("theme.dark", egui::ThemePreference::Dark),
            ("theme.light", egui::ThemePreference::Light),
            ("theme.system", egui::ThemePreference::System),
        ] {
            app.run_command(cmd);
            assert_eq!(app.theme_pref, pref);
        }
        app.explorer_target = None;
        app.project_name = "根目录选择不应重命名活动窗口".into();
        app.run_command("project.rename");
        assert_eq!(app.project, before);
        app.active = None;
        app.run_command("window.minimize");
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn explorer_new_folder_button_and_tree_render_real_project_data() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.explorer_target = None;
        let frame = |app: &mut SciApp, events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.explorer(ui));
                },
            )
        };
        let before = app.project.clone();
        // “＋ 新建”菜单 → “文件夹”
        for label in ["＋ 新建", "文件夹"] {
            frame(&mut app, vec![]);
            let output = frame(&mut app, vec![]);
            let at = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => {
                        Some(t.pos + t.galley.rect.center().to_vec2())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing {label}"));
            for pressed in [true, false] {
                frame(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        assert_eq!(app.project.folders.len(), 1);
        roundtrip(&mut app, &before);
        let output = frame(&mut app, vec![]);
        assert!(output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text=="文件夹1")));
    }

    #[test]
    fn commands_restore_columns_graphs_fits_and_sorting() {
        for command in [
            "col.delete",
            "col.left",
            "col.right",
            "col.first",
            "col.last",
            "sort.desc",
            "fit.linear",
            "plot.bar",
            "plot.stacked_bar",
            "plot.percent_stacked_bar",
            "plot.horizontal_bar",
            "plot.area",
            "plot.spline",
            "plot.spline_symbol",
            "plot.vertical_drop",
            "plot.step_horizontal",
            "plot.step_vertical",
            "col.setXError",
            "col.setYError",
            "col.setZ",
            "col.setLabel",
            "book.new",
        ] {
            let mut app = app();
            app.active = Some(1);
            app.selection.insert(1, vec!["B".into()]);
            let before = app.project.clone();
            app.run_command(command);
            roundtrip(&mut app, &before);
        }
    }

    #[test]
    fn deleted_workbook_returns_and_new_windows_are_cleaned_up() {
        let mut app = app();
        let before = app.project.clone();
        app.active = Some(1);
        app.sheets.insert(1, SheetState::default());
        app.delete_item(1);
        app.finish_edit();
        app.run_command("edit.undo");
        assert_eq!(app.project, before);
        assert_eq!(app.open.get(&1), Some(&true));
        app.run_command("book.new");
        let id = app.active.unwrap();
        app.geoms
            .insert(id, Geom::new(egui::Pos2::ZERO, egui::vec2(300.0, 200.0)));
        app.sheets.insert(id, SheetState::default());
        app.run_command("edit.undo");
        assert!(!app.open.contains_key(&id));
        assert!(!app.geoms.contains_key(&id));
        assert!(!app.sheets.contains_key(&id));
        assert_ne!(app.active, Some(id));
        app.run_command("edit.redo");
        assert_eq!(app.open.get(&id), Some(&true));
    }

    #[test]
    fn nonlinear_fit_opens_a_report_and_undoes_as_one_step() {
        use scicanvas_core::nlfit::Model;
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        let before = app.project.clone();
        app.open_nlfit_dialog();
        assert!(matches!(
            app.dialog,
            Some(Dialog::NlFit {
                model: Model::Gauss,
                ..
            })
        ));
        app.finish_edit();
        app.begin_edit("非线性拟合");
        app.dialog = None;
        app.nlfit(Model::ExpDec1, &[], &[]);
        app.history.changed();
        app.finish_edit();
        match &app.dialog {
            Some(Dialog::NlFitReport(reports)) => {
                assert_eq!(reports.len(), 1);
                assert!(reports[0].fit.converged);
            }
            _ => panic!("report should open"),
        }
        assert_eq!(app.history.undo_label(), Some("非线性拟合"));
        roundtrip(&mut app, &before);
    }

    #[test]
    fn column_math_dialog_follows_the_picked_columns_and_undoes() {
        use scicanvas_core::ops::ColumnOp;
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        app.selection.insert(1, vec!["C".into(), "B".into()]);
        app.open_column_math_dialog();
        assert!(matches!(
            &app.dialog,
            Some(Dialog::ColumnMath { a, b: Some(b), use_constant: false, .. }) if a == "电压 B" && b == "电压 A"
        ));
        app.finish_edit();
        app.begin_edit("列运算");
        app.column_math(ColumnOp::Sub, None);
        app.history.changed();
        app.finish_edit();
        let book = app.project.book(1).unwrap();
        let result = book.columns.iter().find(|c| c.long_name == "电压 B − 电压 A").unwrap();
        let want = book.column("C").unwrap().values()[3] - book.column("B").unwrap().values()[3];
        assert!((result.values()[3] - want).abs() < 1e-9);
        roundtrip(&mut app, &before);

        app.selection.insert(1, vec!["B".into()]);
        app.open_column_math_dialog();
        assert!(matches!(
            app.dialog,
            Some(Dialog::ColumnMath {
                b: None,
                use_constant: true,
                ..
            })
        ));
    }

    #[test]
    fn frequency_dialog_defaults_to_the_picked_columns_value_range() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into()]);
        app.open_frequency_dialog();
        assert!(matches!(
            app.dialog,
            Some(Dialog::Frequency { auto: true, custom: false, from, to, .. }) if from == 0.0 && to == 20.0
        ));
        let before = app.project.clone();
        app.frequency_counts(Some(4), None);
        assert_eq!(app.project.books.len(), before.books.len() + 1);
        assert_eq!(
            app.active,
            app.project.books.last().map(|b| b.id),
            "the new table is shown"
        );
    }

    #[test]
    fn manual_analysis_results_update_only_on_recalc_command() {
        use scicanvas_core::ops::RecalcMode;
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        app.begin_edit("平滑");
        app.smooth(scicanvas_core::analysis::SmoothMethod::MovingAverage, 3);
        app.history.changed();
        app.finish_edit();
        let key = app
            .project
            .book(1)
            .unwrap()
            .columns
            .iter()
            .find(|c| c.op.is_some())
            .unwrap()
            .key
            .clone();
        // 列属性改为手动
        app.selection.insert(1, vec![key.clone()]);
        app.run_command("col.props");
        let Some(Dialog::ColumnProps { edit, .. }) = app.dialog.take() else {
            panic!()
        };
        assert_eq!(edit.recalc, Some(RecalcMode::Auto));
        let mut edit = edit;
        edit.recalc = Some(RecalcMode::Manual);
        app.begin_edit("列属性");
        app.apply_column_props(1, edit).unwrap();
        app.history.changed();
        app.finish_edit();
        let smoothed = |app: &SciApp| app.project.book(1).unwrap().column(&key).unwrap().values();
        let old = smoothed(&app);
        app.begin_edit("编辑单元格");
        app.project.book_mut(1).unwrap().columns[1].set(1, "99".into());
        app.history.changed();
        app.finish_edit();
        assert_eq!(smoothed(&app), old);
        let before = app.project.clone();
        app.run_command("data.recalc");
        assert_ne!(smoothed(&app), old);
        roundtrip(&mut app, &before);
    }

    #[test]
    fn saved_formula_recalculates_when_a_source_cell_changes_in_the_same_undo_step() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.begin_edit("设置列值");
        assert!(app.set_column_values("B * 2", true));
        app.history.changed();
        app.finish_edit();
        app.open_set_values_dialog();
        assert!(matches!(&app.dialog, Some(Dialog::SetValues { formula, keep: true, .. }) if formula == "B * 2"));
        app.dialog = None;
        let before = app.project.clone();
        // 改 B 列一个单元格：提交编辑时 C 跟着重算，撤销时一起回退
        app.begin_edit("编辑单元格");
        app.project.book_mut(1).unwrap().columns[1].set(0, "100".into());
        app.history.changed();
        app.finish_edit();
        assert_eq!(app.project.book(1).unwrap().column("C").unwrap().values()[0], 200.0);
        app.undo();
        assert_eq!(app.project, before);
    }

    #[test]
    fn set_values_keeps_the_dialog_open_on_errors_and_undoes_in_one_step() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.open_set_values_dialog();
        assert!(matches!(&app.dialog, Some(Dialog::SetValues { target, .. }) if target.starts_with('C')));
        assert!(!app.set_column_values("B *", false));
        assert!(matches!(&app.dialog, Some(Dialog::SetValues { error: Some(_), .. })));
        let before = app.project.book(1).unwrap().columns[2].data.clone();
        app.begin_edit("设置列值");
        assert!(app.set_column_values("B * 2", false));
        app.history.changed();
        app.finish_edit();
        let book = app.project.book(1).unwrap();
        let b = book.column("B").unwrap().values();
        let c = book.column("C").unwrap().values();
        assert!(b
            .iter()
            .zip(&c)
            .all(|(b, c)| (b * 2.0 - c).abs() < 1e-9 * b.abs().max(1.0)));
        app.undo();
        assert_eq!(app.project.book(1).unwrap().columns[2].data, before);
    }

    #[test]
    fn baseline_subtraction_runs_on_picked_columns_in_one_undo_step() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        let cols = app.project.book(1).unwrap().columns.len();
        app.begin_edit("扣除基线");
        app.subtract_baseline(scicanvas_core::baseline::BaselineMethod::Line);
        app.history.changed();
        app.finish_edit();
        let book = app.project.book(1).unwrap();
        assert_eq!(book.columns.len(), cols + 2);
        assert_eq!(book.columns[3].long_name, "电压 A 扣除基线");
        assert!(app.status.contains("两端连线"), "{}", app.status);
        app.undo();
        assert_eq!(app.project.book(1).unwrap().columns.len(), cols);
    }

    #[test]
    fn peaks_open_a_table_per_curve_in_one_undo_step() {
        let mut app = app();
        app.active = Some(1);
        let book = app.project.book_mut(1).unwrap();
        book.columns[0].data = (0..61).map(|i| scicanvas_core::model::fmt_num(i as f64)).collect();
        book.columns[1].data = (0..61)
            .map(|i| {
                scicanvas_core::model::fmt_num(
                    3.0 * (-((i as f64 - 20.0) / 3.0).powi(2)).exp() + (-((i as f64 - 45.0) / 4.0).powi(2)).exp(),
                )
            })
            .collect();
        app.selection.insert(1, vec!["B".into()]);
        let books = app.project.books.len();
        app.begin_edit("寻峰");
        app.find_peaks(0.05);
        app.history.changed();
        app.finish_edit();
        assert_eq!(app.project.books.len(), books + 1);
        let table = app.project.books.last().unwrap();
        assert_eq!(table.columns[0].values(), [20.0, 45.0]);
        assert!(app.status.contains("2 个峰"), "{}", app.status);
        app.undo();
        assert_eq!(app.project.books.len(), books);
    }

    #[test]
    fn correlation_fill_and_average_create_results() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        let (books, graphs) = (app.project.books.len(), app.project.graphs.len());
        app.correlation_function();
        assert_eq!(
            (app.project.books.len(), app.project.graphs.len()),
            (books + 1, graphs + 1),
            "{}",
            app.status
        );
        app.active = Some(1);
        app.average_curves();
        assert_eq!(app.project.books.len(), books + 2, "{}", app.status);

        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.open_fill_dialog();
        assert!(matches!(&app.dialog, Some(Dialog::Fill { target, .. }) if target.starts_with('C')));
        app.dialog = None;
        app.fill_sequence(scicanvas_core::stats::Sequence::Arithmetic { start: 0.0, step: 0.5 }, 3);
        assert_eq!(
            app.project.book(1).unwrap().column("C").unwrap().data,
            ["0", "0.5", "1"]
        );
    }

    #[test]
    fn regression_uses_the_last_picked_column_as_y() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into(), "C".into(), "B".into()]);
        app.regression();
        let book = app.project.book(1).unwrap();
        let want = scicanvas_core::stats::multiple_regression(
            &[book.column("A").unwrap().values(), book.column("C").unwrap().values()],
            &book.column("B").unwrap().values(),
        );
        match (&app.dialog, want) {
            (Some(Dialog::Table { rows, note, .. }), Ok(r)) => {
                assert_eq!(rows.len(), 4);
                assert_eq!(rows[2][0], book.column("A").unwrap().display_name());
                assert_eq!(rows[1][1], fmt_num(round_sig(r.coef[0], 8)));
                assert!(note.contains(book.column("B").unwrap().display_name()));
            }
            (d, w) => panic!("unexpected {:?} / {:?}", d.is_some(), w),
        }
    }

    #[test]
    fn row_stats_and_fft_filter_add_columns_in_one_undo_step() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        let cols = app.project.book(1).unwrap().columns.len();
        app.begin_edit("行统计");
        app.row_stats();
        app.history.changed();
        app.finish_edit();
        assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 4, "{}", app.status);
        app.undo();
        assert_eq!(app.project.book(1).unwrap().columns.len(), cols);

        app.selection.insert(1, vec!["B".into()]);
        app.open_fft_filter_dialog();
        let Some(Dialog::FftFilter { lo, nyquist, .. }) = &app.dialog else {
            panic!("filter dialog should open")
        };
        let (lo, nyquist) = (*lo, *nyquist);
        assert!(lo > 0.0 && lo < nyquist);
        app.dialog = None;
        app.fft_filter(scicanvas_core::fft::FilterKind::LowPass(lo));
        assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 1, "{}", app.status);
    }

    #[test]
    fn normality_and_grubbs_report_one_row_per_picked_column() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        app.normality();
        match &app.dialog {
            Some(Dialog::Table { rows, .. }) => {
                assert_eq!(rows.len(), 3);
                let w =
                    scicanvas_core::stats::shapiro_wilk(&app.project.book(1).unwrap().column("B").unwrap().values())
                        .unwrap();
                assert_eq!(rows[1][2], fmt_num(round_sig(w.w, 6)));
            }
            _ => panic!("normality report should open"),
        }
        app.dialog = None;
        app.grubbs();
        assert!(matches!(&app.dialog, Some(Dialog::Table { rows, .. }) if rows.len() == 3 && rows[0].len() == 8));
        app.dialog = None;
        app.selection.insert(1, vec![]);
        app.normality();
        assert!(app.dialog.is_none() && app.status.contains("选中"));
    }

    #[test]
    fn calibrate_needs_three_columns_in_click_order() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into(), "B".into()]);
        app.calibrate();
        assert!(app.status.contains("三列"));
        let cols = app.project.book(1).unwrap().columns.len();
        app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
        app.begin_edit("标准曲线反查");
        app.calibrate();
        app.history.changed();
        app.finish_edit();
        assert_eq!(app.project.book(1).unwrap().columns.len(), cols + 2, "{}", app.status);
        assert!(app.status.contains("R²"));
        app.undo();
        assert_eq!(app.project.book(1).unwrap().columns.len(), cols);
    }

    #[test]
    fn anova_treats_each_picked_column_as_a_group() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        app.anova();
        let book = app.project.book(1).unwrap();
        let want = scicanvas_core::stats::one_way_anova(&[
            book.column("B").unwrap().values(),
            book.column("C").unwrap().values(),
        ])
        .unwrap();
        match &app.dialog {
            Some(Dialog::Anova { result, names, tukey }) => {
                assert_eq!(result, &want);
                assert_eq!(tukey, &scicanvas_core::stats::tukey_hsd(&want, 0.95));
                assert_eq!(names, &["电压 A", "电压 B"]);
            }
            _ => panic!("report should open"),
        }
        app.dialog = None;
        app.selection.insert(1, vec!["B".into()]);
        app.anova();
        assert!(app.dialog.is_none() && app.status.contains("至少两列"));
    }

    #[test]
    fn correlations_cover_every_pair_of_picked_columns() {
        use scicanvas_core::stats::{correlation, CorrMethod};
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into(), "B".into(), "C".into()]);
        app.correlations();
        let Some(Dialog::Correlations(pairs)) = &app.dialog else {
            panic!("report should open")
        };
        let names: Vec<_> = pairs.iter().map(|(a, b, _)| (a.as_str(), b.as_str())).collect();
        assert_eq!(names, [("时间", "电压 A"), ("时间", "电压 B"), ("电压 A", "电压 B")]);
        let book = app.project.book(1).unwrap();
        let want = correlation(
            &book.column("A").unwrap().values(),
            &book.column("C").unwrap().values(),
            CorrMethod::Spearman,
        )
        .unwrap();
        assert_eq!(pairs[1].2[1], want);
        app.dialog = None;
        app.selection.insert(1, vec!["A".into()]);
        app.correlations();
        assert!(app.dialog.is_none() && app.status.contains("至少两列"));
    }

    #[test]
    fn ttest_uses_the_picked_columns_in_click_order_and_leaves_data_alone() {
        use scicanvas_core::stats::{t_test, TTestKind};
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        app.selection.insert(1, vec!["C".into(), "B".into()]);
        app.open_ttest_dialog();
        match &app.dialog {
            Some(Dialog::TTest { columns, kind, .. }) => {
                assert_eq!(columns, &["电压 B", "电压 A"]);
                assert_eq!(*kind, 1, "two picked columns default to a two-sample test");
            }
            _ => panic!("t-test dialog should open"),
        }
        let kind = TTestKind::TwoSample { equal_var: false };
        app.run_ttest(kind, 0.95);
        let book = app.project.book(1).unwrap();
        let col = |k: &str| book.column(k).unwrap().values();
        let want = t_test(kind, &col("C"), &col("B"), 0.95).unwrap();
        match &app.dialog {
            Some(Dialog::TTestReport { result, names }) => {
                assert_eq!(result, &want);
                assert_eq!(names, &["电压 B", "电压 A"]);
            }
            _ => panic!("report should open"),
        }
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());

        app.selection.insert(1, vec!["B".into()]);
        app.open_ttest_dialog();
        assert!(matches!(app.dialog, Some(Dialog::TTest { kind: 0, .. })));
    }

    #[test]
    fn excel_workbooks_import_each_non_empty_sheet() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scicanvas-core/tests/fixtures/sample.xlsx");
        let mut app = app();
        let before = app.project.clone();
        app.open_dropped(vec![path]);
        let names: Vec<_> = app.project.books.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(
            names,
            ["Book1", "sample-测量", "sample-对照"],
            "the empty sheet is skipped"
        );
        let book = &app.project.books[1];
        let head: Vec<_> = book
            .columns
            .iter()
            .map(|c| (c.long_name.as_str(), c.units.as_str()))
            .collect();
        assert_eq!(head, [("时间", "s"), ("电压", "mV"), ("备注", "")]);
        assert_eq!(book.columns[1].data.text_at(5), "0.3");
        assert!(app.status.contains("sample-测量（6 行 × 3 列）"));
        roundtrip(&mut app, &before);
    }

    #[test]
    fn dropped_data_files_import_as_one_step_and_project_files_open() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/file-tests")
            .join(format!("drop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("温度.csv");
        let b = dir.join("压力.txt");
        std::fs::write(&a, "时间 (s),温度 (°C)\n0,20\n1,21.5\n").unwrap();
        std::fs::write(&b, "1\t2\n3\t4\n5\t6\n").unwrap();

        let mut app = app();
        let before = app.project.clone();
        let ctx = egui::Context::default();
        let drop = |app: &mut SciApp, paths: &[&std::path::Path]| {
            let input = egui::RawInput {
                dropped_files: paths
                    .iter()
                    .map(|p| egui::DroppedFile {
                        path: Some(p.to_path_buf()),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| app.dropped_files(ctx));
        };
        drop(&mut app, &[&a, &b]);
        let names: Vec<_> = app.project.books.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["Book1", "温度", "压力"]);
        let t = app.project.books[1].clone();
        assert_eq!(
            (t.columns[1].long_name.as_str(), t.columns[1].units.as_str()),
            ("温度", "°C")
        );
        assert_eq!(app.project.books[2].nrows(), 3);
        assert_eq!(app.history.undo_label(), Some("导入数据"));
        roundtrip(&mut app, &before);

        let scv = dir.join("项目.scv");
        let mut other = Project::sample();
        other.books[0].name = "拖入的项目".into();
        scicanvas_core::project_file::save(&scv, &other).unwrap();
        drop(&mut app, &[&a, &scv]);
        assert_eq!(app.project, other, "a project file wins over data files");
        assert_eq!(app.project_path.as_deref(), Some(scv.as_path()));
        assert!(app.history.undo_label().is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn row_commands_follow_the_selection_and_undo_as_one_step() {
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        let rows = |app: &SciApp| {
            app.project.book(1).unwrap().columns[0]
                .data
                .iter()
                .map(std::borrow::Cow::into_owned)
                .collect::<Vec<_>>()
        };
        let original = rows(&app);
        let select = |app: &mut SciApp| {
            app.sheets.insert(
                1,
                SheetState {
                    range: Some(Range {
                        a: Cell { c: 1, r: 2 },
                        b: Cell { c: 2, r: 4 },
                    }),
                    ..Default::default()
                },
            );
        };

        select(&mut app);
        app.run_command("row.insert");
        assert_eq!(rows(&app).len(), original.len() + 3);
        assert_eq!(rows(&app)[2..5], [String::new(), String::new(), String::new()]);
        assert_eq!(app.history.undo_label(), Some("插入行"));
        roundtrip(&mut app, &before);
        app.run_command("edit.undo");

        select(&mut app);
        app.run_command("row.delete");
        assert_eq!(
            rows(&app)[2],
            original[5],
            "whole rows go, whatever columns were selected"
        );
        assert_eq!(app.sheets[&1].range, Some(Range::single(1, 2)));
        roundtrip(&mut app, &before);
    }

    #[test]
    fn paste_cut_and_clear_use_project_history() {
        let ctx = egui::Context::default();
        for event in [
            egui::Event::Paste("1\t2\n3\t4".into()),
            egui::Event::Cut,
            egui::Event::Key {
                key: egui::Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ] {
            let mut app = app();
            app.active = Some(1);
            let before = app.project.clone();
            let mut sheet = SheetState {
                range: Some(Range::single(0, 0)),
                ..Default::default()
            };
            let input = egui::RawInput {
                events: vec![event],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| app.sheet_keys(ctx, 1, &mut sheet));
            app.finish_edit();
            roundtrip(&mut app, &before);
        }
    }

    #[test]
    fn cell_enter_commits_one_undoable_edit() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        app.sheets.insert(
            1,
            SheetState {
                range: Some(Range::single(0, 0)),
                edit: Some(Edit {
                    cell: Cell { c: 0, r: 0 },
                    buf: "123".into(),
                    focused: false,
                }),
                ..Default::default()
            },
        );
        for events in [
            vec![],
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
                },
            );
            app.finish_edit();
        }
        assert_eq!(app.project.books[0].columns[0].data.text_at(0), "123");
        roundtrip(&mut app, &before);
    }

    #[test]
    fn undo_shortcut_respects_text_focus_and_accepts_button_focus() {
        let ctx = egui::Context::default();
        let mut app = app();
        let before = app.project.clone();
        app.run_command("book.new");
        let after = app.project.clone();
        let text_id = egui::Id::new("文字框");
        egui::TextEdit::store_state(&ctx, text_id, Default::default());
        for (id, expected) in [(text_id, &after), (egui::Id::new("按钮"), &before)] {
            let input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Z,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                }],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                ctx.memory_mut(|m| m.request_focus(id));
                app.shortcuts(ctx);
            });
            assert_eq!(&app.project, expected);
        }
    }

    #[test]
    fn continuous_edit_is_one_step_and_noop_preserves_redo() {
        let mut app = app();
        let before = app.project.clone();
        let group = Some(egui::Id::new("名称"));
        app.begin_edit("修改长名称");
        app.project.books[0].columns[0].long_name = "时".into();
        app.history.changed();
        app.finish_frame_edit(group);
        app.project.books[0].columns[0].long_name = "时间轴".into();
        app.history.changed();
        app.finish_frame_edit(group);
        roundtrip(&mut app, &before);
        app.run_command("edit.undo");
        assert!(app.history.undo_label().is_none());
        app.run_command("col.delete"); // 当前图形，没有工作簿，命令不可用
        assert!(app.history.redo_label().is_some());
        app.load_project(Project::sample(), None);
        assert!(app.history.redo_label().is_none());
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn separate_slider_drags_are_separate_undo_steps() {
        let ctx = egui::Context::default();
        let mut app = app();
        let initial = app.project.clone();
        let mut rect = egui::Rect::NOTHING;
        let mut frame = |app: &mut SciApp, events: Vec<egui::Event>| {
            app.begin_edit("线宽");
            let before = app.project.graphs[0].first_layer().line_width;
            app.history_group = None;
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = ui.add(
                            egui::Slider::new(&mut app.project.graphs[0].first_layer_mut().line_width, 0.5..=6.0)
                                .show_value(false),
                        );
                        rect = response.rect;
                        edit_group(&response, &mut app.history_group);
                    });
                },
            );
            if app.project.graphs[0].first_layer().line_width != before {
                app.history.changed();
            }
            app.finish_frame_edit(app.history_group);
            rect
        };
        let r = frame(&mut app, vec![]);
        let mut states = vec![initial];
        for (start, end) in [(0.2, 0.5), (0.6, 0.9)] {
            let at = |f: f32| egui::pos2(r.left() + r.width() * f, r.center().y);
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            frame(
                &mut app,
                vec![egui::Event::PointerMoved(at(start)), button(at(start), true)],
            );
            frame(&mut app, vec![egui::Event::PointerMoved(at(end))]);
            frame(&mut app, vec![button(at(end), false)]);
            frame(&mut app, vec![]);
            states.push(app.project.clone());
        }
        assert_ne!(states[0], states[1]);
        assert_ne!(states[1], states[2]);
        app.run_command("edit.undo");
        assert_eq!(app.project, states[1]);
        app.run_command("edit.undo");
        assert_eq!(app.project, states[0]);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn dialog_confirmation_is_recorded_and_cancellation_is_empty() {
        let mut app = app();
        let before = app.project.clone();
        app.run_command("fit.poly");
        assert!(app.history.undo_label().is_none());
        app.dialog = None;
        app.begin_edit("多项式拟合");
        app.fit(2);
        app.history.changed();
        app.finish_edit();
        roundtrip(&mut app, &before);
    }

    #[test]
    fn headless_frames_and_history_shortcuts() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        egui_extras::install_image_loaders(&ctx);
        theme::install(&ctx);
        let mut app = app();
        let before = app.project.clone();
        app.run_command("book.new");
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0))),
            events: vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| app.ui_frame(ctx));
        assert!(!output.shapes.is_empty());
        assert_eq!(app.project, before);
        assert!(app.history.redo_label().is_some());
    }

    #[test]
    fn layers_builder_and_saved_project_share_history_and_headless_ui() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        theme::install(&ctx);
        egui_extras::install_image_loaders(&ctx);
        let mut app = app();
        let before = app.project.clone();
        app.run_command("layer.add");
        assert_eq!(app.project.graphs[0].page.layers().len(), 2);
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
        roundtrip(&mut app, &before);
        let two_layers = app.project.clone();
        app.run_command("layer.remove");
        roundtrip(&mut app, &two_layers);
        app.props_for = None;
        app.active = Some(1);
        let before = app.project.clone();
        app.run_command("plot.builder");
        assert_eq!(app.project, before);
        let output = ctx.run(egui::RawInput::default(), |ctx| app.ui_frame(ctx));
        assert!(!output.shapes.is_empty());
        app.run_command("plot.apply_spec");
        roundtrip(&mut app, &before);
        let text = scicanvas_core::project_file::to_json(&app.project).unwrap();
        assert_eq!(scicanvas_core::project_file::from_json(&text).unwrap(), app.project);
    }

    #[test]
    fn selecting_layer_is_session_state_and_commands_follow_that_layer() {
        let mut app = app();
        app.run_command("layer.add");
        app.history = History::default();
        app.select_layer(2, 0);
        app.select_layer(2, 1);
        app.finish_edit();
        assert!(app.history.undo_label().is_none());
        assert_eq!(app.analysis_source(), Some(Source::Layer { graph_id: 2, index: 1 }));
        let before = app.project.clone();
        app.run_command("graph.grid");
        assert!(!app.project.graphs[0].page.layers()[0].grid);
        assert!(app.project.graphs[0].page.layers()[1].grid);
        roundtrip(&mut app, &before);
        app.run_command("plot.bar");
        assert_eq!(app.project.graphs[0].page.layers()[0].plot_type, PlotType::LineSymbol);
        assert_eq!(app.project.graphs[0].page.layers()[1].plot_type, PlotType::Bar);
        app.run_command("layer.remove");
        assert_eq!(app.layer_index(2), 0);
        app.run_command("edit.undo");
        assert_eq!(app.project.graphs[0].page.layers().len(), 2);
    }

    fn editor_frame(app: &mut SciApp, ctx: &egui::Context, events: Vec<egui::Event>, props: bool) -> egui::FullOutput {
        app.history_group = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 900.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                if props {
                    app.props_window(ctx);
                } else {
                    egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
                }
            },
        );
        app.finish_frame_edit(app.history_group);
        output
    }

    #[test]
    fn bar_variant_commands_preserve_layer_targets_pairing_and_history() {
        for command in [
            "plot.stacked_bar",
            "plot.percent_stacked_bar",
            "plot.floating_bar",
            "plot.horizontal_bar",
            "plot.area",
        ] {
            let mut app = app();
            app.active = Some(1);
            app.selection.insert(1, vec!["B".into(), "C".into()]);
            let before = app.project.clone();
            app.run_command(command);
            let id = app.active.unwrap();
            assert_ne!(id, 1);
            roundtrip(&mut app, &before);
            let one_layer = app.project.graph(id).unwrap().first_layer().clone();
            app.run_command("layer.add");
            let before = app.project.clone();
            app.run_command(if command == "plot.stacked_bar" {
                "plot.percent_stacked_bar"
            } else {
                "plot.stacked_bar"
            });
            assert_eq!(*app.project.graph(id).unwrap().first_layer(), one_layer);
            roundtrip(&mut app, &before);
        }
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        let before = app.project.clone();
        app.run_command("plot.floating_bar");
        assert_eq!(app.project, before);
        assert!(app.status.contains("成对"));
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn floating_baseline_dropdown_is_an_undoable_property_edit() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        app.run_command("plot.floating_bar");
        let id = app.active.unwrap();
        app.props_for = Some(id);
        editor_frame(&mut app, &ctx, vec![], true);
        let output = editor_frame(&mut app, &ctx, vec![], true);
        let at = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text.starts_with("C 的下端：B") => {
                    Some(t.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .unwrap();
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        editor_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(at), click(at, true)],
            true,
        );
        editor_frame(&mut app, &ctx, vec![click(at, false)], true);
        let output = editor_frame(&mut app, &ctx, vec![], true);
        let at = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text.starts_with("A ·") => Some(t.pos + egui::vec2(4.0, 4.0)),
                _ => None,
            })
            .next_back()
            .unwrap();
        let before = app.project.clone();
        editor_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(at), click(at, true)],
            true,
        );
        editor_frame(&mut app, &ctx, vec![click(at, false)], true);
        assert_eq!(
            app.project
                .graph(id)
                .unwrap()
                .first_layer()
                .base_by_y
                .get("C")
                .map(String::as_str),
            Some("A")
        );
        roundtrip(&mut app, &before);
    }

    #[test]
    fn area_opacity_slider_is_an_undoable_property_edit() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        app.run_command("plot.area");
        let id = app.active.unwrap();
        app.props_for = Some(id);
        editor_frame(&mut app, &ctx, vec![], true);
        let output = editor_frame(&mut app, &ctx, vec![], true);
        let y = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "面积填充" => Some(t.pos.y + t.galley.size().y / 2.0),
                _ => None,
            })
            .unwrap();
        let at = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Circle(c) if (c.center.y - y).abs() < 3.0 => Some(c.center),
                _ => None,
            })
            .unwrap();
        let before = app.project.clone();
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        editor_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(at), click(at, true)],
            true,
        );
        let end = at + egui::vec2(20.0, 0.0);
        editor_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)], true);
        editor_frame(&mut app, &ctx, vec![click(end, false)], true);
        assert!(app.project.graph(id).unwrap().first_layer().area_opacity > 0.35);
        roundtrip(&mut app, &before);
    }

    #[test]
    fn data_label_command_and_actual_checkbox_are_undoable() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.active = Some(2);
        let before = app.project.clone();
        app.run_command("graph.labels");
        assert!(app.project.graph(2).unwrap().first_layer().labels.visible);
        roundtrip(&mut app, &before);
        app.props_for = Some(2);
        editor_frame(&mut app, &ctx, vec![], true);
        let output = editor_frame(&mut app, &ctx, vec![], true);
        let at = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "数据标签" => {
                    Some(t.pos + t.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap();
        let before = app.project.clone();
        for pressed in [true, false] {
            editor_frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                true,
            );
        }
        assert!(!app.project.graph(2).unwrap().first_layer().labels.visible);
        roundtrip(&mut app, &before);
    }

    #[test]
    fn error_role_commands_and_visibility_control_are_undoable() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        let before = app.project.clone();
        app.run_command("col.setYError");
        let b = app.project.book(1).unwrap();
        assert_eq!(b.columns[2].role, Role::YError);
        assert_eq!(b.columns[2].error_for.as_deref(), Some("B"));
        let (_, series) = app.project.graph(2).unwrap().first_layer().series(b);
        assert_eq!(series.len(), 1);
        assert_eq!(app.project.resolve(&Source::Graph(2)).unwrap().items.len(), 1);
        roundtrip(&mut app, &before);
        app.props_for = Some(2);
        editor_frame(&mut app, &ctx, vec![], true);
        let output = editor_frame(&mut app, &ctx, vec![], true);
        let at = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "显示 X / Y 误差" => {
                    Some(t.pos + t.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap();
        let before = app.project.clone();
        let before_svg = scicanvas_core::svg::render_page(app.project.graph(2).unwrap(), &app.project);
        for pressed in [true, false] {
            editor_frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                true,
            );
        }
        assert!(!app.project.graph(2).unwrap().first_layer().error_bars.visible);
        assert_ne!(
            scicanvas_core::svg::render_page(app.project.graph(2).unwrap(), &app.project),
            before_svg
        );
        roundtrip(&mut app, &before);
    }

    #[test]
    fn actual_header_and_property_text_edits_form_one_transaction_each() {
        for props in [false, true] {
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            let mut app = app();
            let mut p = Project::sample();
            p.graphs[0].first_layer_mut().title = "原图".into();
            app.load_project(p, None);
            app.active = Some(1);
            if props {
                app.props_for = Some(2);
            }
            let before = app.project.clone();
            editor_frame(&mut app, &ctx, vec![], props);
            let output = editor_frame(&mut app, &ctx, vec![], props);
            assert!(!app.history.is_pending());
            let text = if props { "原图" } else { "时间" };
            let at = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == text => Some(t.pos + t.galley.rect.center().to_vec2()),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    panic!(
                        "未找到 {text}，当前文字：{:?}",
                        output
                            .shapes
                            .iter()
                            .filter_map(|s| match &s.shape {
                                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                    )
                });
            let button = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            editor_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at), button(true)], props);
            editor_frame(&mut app, &ctx, vec![button(false)], props);
            editor_frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                }],
                props,
            );
            for text in ["a", "b", "c"] {
                editor_frame(&mut app, &ctx, vec![egui::Event::Text(text.into())], props);
            }
            assert!(app.history.is_pending());
            if props {
                assert_eq!(app.project.graphs[0].first_layer().title, "abc");
            } else {
                assert_eq!(app.project.books[0].columns[0].long_name, "abc");
            }
            roundtrip(&mut app, &before);
            app.run_command("edit.undo");
            assert!(app.history.undo_label().is_none());
        }
    }

    #[test]
    fn blank_graphs_can_be_bound_to_a_book_and_examples_use_the_default_page() {
        let mut app = app();
        app.run_command("graph.new");
        let gid = app.active.unwrap();
        assert_eq!(app.project.graph(gid).unwrap().first_layer().book_id, None);
        // 绘图属性里选择工作簿：与界面相同，用该表的默认映射
        let book = app.project.book(1).unwrap();
        let defaults = scicanvas_core::plotspec::PlotSpec::from_selection(book, &[], PlotType::LineSymbol).ok();
        let layer = app.project.graph_mut(gid).unwrap().first_layer_mut();
        layer.spec = layer.spec.rebound(1, defaults.as_ref());
        let graph = app.project.graph(gid).unwrap();
        let (_, series) = graph.first_layer().series(app.project.book(1).unwrap());
        assert!(!series.is_empty() && series.iter().all(|s| !s.points.is_empty()));

        // 打开示例：页面与新建图形相同（A4 宽），不沿用示例文件里的旧 89 mm
        app.run_command("example.line");
        let gid = app.active.unwrap();
        let page = &app.project.graph(gid).unwrap().page;
        let default = scicanvas_core::page::Page::default();
        assert_eq!((page.width_mm, page.height_mm), (default.width_mm, default.height_mm));
    }

    #[test]
    fn column_properties_apply_in_one_step_and_failed_conversions_change_nothing() {
        use scicanvas_core::format::{DisplayFormat, NumberFormat};
        use scicanvas_core::DataKind;
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        app.run_command("col.props");
        let Some(Dialog::ColumnProps { edit, sample, .. }) = &app.dialog else {
            panic!("column properties should open")
        };
        let col = app.project.book(1).unwrap().column("B").unwrap();
        assert_eq!(
            (edit.key.as_str(), edit.long_name.as_str(), edit.kind),
            ("B", col.long_name.as_str(), DataKind::Num)
        );
        assert_eq!(*sample, col.data.f64_at(0));
        let mut edit = edit.clone();
        let raw = col.data.text_at(0).into_owned();

        // 一次确定：名称、单位、注释、角色、格式一起改，作为一步撤销
        let before = app.project.clone();
        edit.long_name = "电压".into();
        edit.units = "V".into();
        edit.comment = "换算后".into();
        edit.role = Role::Ignore;
        edit.format = DisplayFormat {
            number: NumberFormat::Decimal { places: 1 },
            ..Default::default()
        };
        app.dialog = None;
        app.begin_edit("列属性");
        app.apply_column_props(1, edit.clone()).unwrap();
        app.history.changed();
        app.finish_edit();
        let col = app.project.book(1).unwrap().column("B").unwrap();
        assert_eq!(
            (col.long_name.as_str(), col.units.as_str(), col.comment.as_str()),
            ("电压", "V", "换算后")
        );
        assert_eq!(col.role, Role::Ignore);
        assert_eq!(col.data.text_at(0), raw, "原始写法不变");
        assert_eq!(col.display_at(0), format!("{:.1}", col.data.f64_at(0).unwrap()));
        assert_eq!(app.history.undo_label(), Some("列属性"));
        roundtrip(&mut app, &before);

        // 数值列转日期失败：整次不改，错误留在对话框
        let before = app.project.clone();
        let label = app.history.undo_label().map(str::to_owned);
        edit.kind = DataKind::DateTime;
        app.open_column_props();
        app.finish_edit();
        app.begin_edit("列属性");
        let err = app.apply_column_props(1, edit).unwrap_err();
        app.finish_edit();
        assert!(err.contains("无法转换"), "{err}");
        assert_eq!(app.project, before);
        assert_eq!(
            app.history.undo_label().map(str::to_owned),
            label,
            "失败时不产生撤销步骤"
        );
    }

    #[test]
    fn data_type_commands_are_atomic_and_undoable() {
        use scicanvas_core::{ColumnData, DataKind};
        for (command, kind) in [
            ("col.type.num", DataKind::Num),
            ("col.type.text", DataKind::Text),
            ("col.type.category", DataKind::Category),
            ("col.type.datetime", DataKind::DateTime),
        ] {
            let mut app = app();
            app.active = Some(1);
            app.selection.insert(1, vec!["A".into()]);
            // 日期列用常见日期写法；其余用带前导零的数字，检查原始写法保留
            let first = if kind == DataKind::DateTime { "2026/10/18" } else { "01" };
            app.project.books[0].columns[0].data = match kind {
                DataKind::Num => ColumnData::Text(vec![first.into(), "2".into()]),
                DataKind::DateTime => ColumnData::Text(vec![first.into(), "2026".into()]),
                _ => vec![first.into(), "2".into()].into(),
            };
            let before = app.project.clone();
            app.run_command(command);
            assert_eq!(app.project.books[0].columns[0].data.kind(), kind);
            assert_eq!(app.project.books[0].columns[0].data.text_at(0), first);
            roundtrip(&mut app, &before);
        }
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["A".into(), "B".into()]);
        let before = app.project.clone();
        app.run_command("col.type.datetime"); // fractional B cells are not millisecond timestamps
        assert_eq!(app.project, before);
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn non_numeric_cell_requires_confirmation_and_can_cancel_or_undo_conversion() {
        use scicanvas_core::DataKind;
        for confirm in [false, true] {
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            let mut app = app();
            app.active = Some(1);
            let before = app.project.clone();
            app.sheets.insert(
                1,
                SheetState {
                    range: Some(Range::single(0, 0)),
                    edit: Some(Edit {
                        cell: Cell { c: 0, r: 0 },
                        buf: "text input".into(),
                        focused: false,
                    }),
                    ..Default::default()
                },
            );
            editor_frame(&mut app, &ctx, vec![], false);
            editor_frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                false,
            );
            assert!(matches!(app.dialog, Some(Dialog::ConvertText { .. })));
            assert_eq!(app.project, before);
            let frame = |app: &mut SciApp, events| {
                ctx.run(
                    egui::RawInput {
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 900.0))),
                        ..Default::default()
                    },
                    |ctx| app.dialogs(ctx),
                )
            };
            frame(&mut app, vec![]);
            let out = frame(&mut app, vec![]);
            let label = if confirm { "转换为文本并写入" } else { "取消" };
            let at = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + egui::vec2(4.0, 4.0)),
                    _ => None,
                })
                .unwrap();
            for pressed in [true, false] {
                frame(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert!(app.dialog.is_none());
            if confirm {
                assert_eq!(app.project.books[0].columns[0].data.kind(), DataKind::Text);
                assert_eq!(app.project.books[0].columns[0].data.text_at(0), "text input");
                assert_eq!(
                    app.project.books[0].columns[0].data.text_at(1),
                    before.books[0].columns[0].data.text_at(1)
                );
                roundtrip(&mut app, &before);
            } else {
                assert_eq!(app.project, before);
                assert!(app.history.undo_label().is_none());
            }
        }
    }

    #[test]
    fn incompatible_grid_paste_leaves_typed_columns_and_headers_unchanged() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.active = Some(1);
        let before = app.project.clone();
        let mut state = SheetState {
            range: Some(Range::single(0, 1)),
            ..Default::default()
        };
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Paste("text\t4\n5\t6".into())],
                ..Default::default()
            },
            |ctx| app.sheet_keys(ctx, 1, &mut state),
        );
        app.finish_edit();
        assert_eq!(app.project, before);
        assert!(app.status.contains("转换为文本列"));
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn idle_large_book_opens_no_history_transaction() {
        let ctx = egui::Context::default();
        let mut app = app();
        let mut p = Project::sample();
        p.books[0].columns[0].data = vec!["1".into(); 100_000].into();
        app.load_project(p, None);
        app.active = Some(1);
        for _ in 0..10 {
            editor_frame(&mut app, &ctx, vec![], false);
            assert!(!app.history.is_pending());
        }
        assert!(app.history.undo_label().is_none());
    }

    #[test]
    fn layer_placement_control_is_undoable() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = app();
        app.props_for = Some(2);
        let before = app.project.clone();
        editor_frame(&mut app, &ctx, vec![], true);
        let output = editor_frame(&mut app, &ctx, vec![], true);
        let at = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "自定义图层位置" => {
                    Some(t.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .unwrap();
        for pressed in [true, false] {
            editor_frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                true,
            );
        }
        assert!(app.project.graphs[0].first_layer().placement.is_some());
        roundtrip(&mut app, &before);
    }
}

#[cfg(test)]
mod snap_assist_tests {
    use super::*;
    use crate::mdi::{snap_rect, Snap};

    const SCREEN: egui::Vec2 = egui::vec2(1200.0, 800.0);

    fn ws() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)
    }

    fn frame(ctx: &egui::Context, app: &mut SciApp, events: Vec<egui::Event>) {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(ws()),
                events,
                ..Default::default()
            },
            |ctx| app.child_windows(ctx),
        );
    }

    fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn click(ctx: &egui::Context, app: &mut SciApp, pos: egui::Pos2) {
        frame(ctx, app, vec![egui::Event::PointerMoved(pos), button(pos, true)]);
        frame(ctx, app, vec![button(pos, false)]);
        frame(ctx, app, vec![]);
    }

    fn card(app: &SciApp, id: u64) -> egui::Rect {
        let assist = app.snap_assist.as_ref().expect("snap assist is showing");
        assist
            .cards
            .iter()
            .find(|(c, _)| *c == id)
            .expect("window has a card")
            .1
    }

    fn setup() -> (egui::Context, SciApp) {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        // 新的 egui Area 第一帧只量尺寸，第二帧起才可见、可交互。
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        (ctx, app)
    }

    #[test]
    fn dragging_a_window_to_an_edge_offers_the_other_windows_for_the_other_half() {
        let (ctx, mut app) = setup();
        let book = app.geoms[&1].rect;
        let start = egui::pos2(book.center().x, book.top() + mdi::TITLE_H / 2.0);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(egui::pos2(150.0, 300.0))],
        );
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(egui::pos2(2.0, 300.0))]);
        frame(&ctx, &mut app, vec![button(egui::pos2(2.0, 300.0), false)]);
        frame(&ctx, &mut app, vec![]);
        assert_eq!(app.geoms[&1].rect, snap_rect(ws(), Snap::Left));
        let assist = app.snap_assist.as_ref().expect("snap assist after snapping");
        assert_eq!(assist.zones, [Snap::Right]);
        let graph = card(&app, 2);
        assert!(snap_rect(ws(), Snap::Right).contains_rect(graph));

        let normal = app.geoms[&2].rect.size();
        click(&ctx, &mut app, graph.center());
        assert!(app.snap_assist.is_none());
        let geom = app.geoms[&2];
        assert_eq!(
            (geom.rect, geom.state),
            (snap_rect(ws(), Snap::Right), WinState::Normal)
        );
        assert_eq!(geom.restore, Some(normal), "the filled window remembers its size");
        assert_eq!(app.active, Some(2));
    }

    #[test]
    fn quarter_snap_fills_the_remaining_quarters_one_by_one() {
        let (ctx, mut app) = setup();
        let book = app.project.new_book();
        app.open.insert(book, true);
        app.geoms.get_mut(&2).unwrap().state = WinState::Min;
        frame(&ctx, &mut app, vec![]);
        app.start_snap_assist(1, Snap::TopLeft);
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        assert_eq!(
            app.snap_assist.as_ref().unwrap().cards.len(),
            2,
            "minimized windows are offered too"
        );

        let at = card(&app, book).center();
        click(&ctx, &mut app, at);
        assert_eq!(app.geoms[&book].rect, snap_rect(ws(), Snap::TopRight));
        assert_eq!(
            app.snap_assist.as_ref().unwrap().zones,
            [Snap::BottomLeft, Snap::BottomRight]
        );

        let graph = card(&app, 2);
        assert!(snap_rect(ws(), Snap::BottomLeft).contains_rect(graph));
        click(&ctx, &mut app, graph.center());
        let geom = app.geoms[&2];
        assert_eq!(
            (geom.rect, geom.state),
            (snap_rect(ws(), Snap::BottomLeft), WinState::Normal)
        );
        assert!(app.snap_assist.is_none(), "no windows left to offer");
    }

    #[test]
    fn escape_or_pressing_outside_the_cards_dismisses_without_moving_windows() {
        let (ctx, mut app) = setup();
        let before = app.geoms[&2].rect;
        app.start_snap_assist(1, Snap::Left);
        frame(&ctx, &mut app, vec![]);
        assert!(app.snap_assist.is_some());
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.snap_assist.is_none());

        app.start_snap_assist(1, Snap::Left);
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        let outside = snap_rect(ws(), Snap::Right).right_bottom() - egui::vec2(20.0, 20.0);
        assert!(!card(&app, 2).contains(outside));
        click(&ctx, &mut app, outside);
        assert!(app.snap_assist.is_none());
        assert_eq!(app.geoms[&2].rect, before);

        // 只有一个窗口时不显示助手。
        app.open.insert(2, false);
        app.start_snap_assist(1, Snap::Left);
        assert!(app.snap_assist.is_none());
    }

    #[test]
    fn cards_fit_the_zone_and_center_a_short_last_row() {
        let zone = egui::Rect::from_min_size(egui::pos2(600.0, 0.0), egui::vec2(600.0, 800.0));
        for n in 1..=7 {
            let cards = crate::assist::card_rects(zone, n);
            assert_eq!(cards.len(), n);
            for (i, a) in cards.iter().enumerate() {
                assert!(zone.contains_rect(*a), "{n}: {a:?}");
                for b in &cards[i + 1..] {
                    assert!(!a.intersects(*b), "{n}: cards overlap");
                }
            }
        }
        let three = crate::assist::card_rects(zone, 3);
        assert!((three[2].center().x - zone.center().x).abs() < 0.5 || three[0].top() != three[2].top());
    }
}
