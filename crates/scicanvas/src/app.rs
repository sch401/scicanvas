//! 应用状态与界面调度、图形属性和分析操作。
//! 项目管理与子窗口位于 workspace，工作表显示与编辑位于 worksheet。

mod notes;
mod props;
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
use crate::dialogs::{self, edit_group, Dialog, PlotObjectEditor as ObjectEditor};
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
    applied: bool,
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
    adding_text: Option<u64>,
    object_editor: Option<ObjectEditor>,
    plot_epoch: u64,
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
            adding_text: None,
            object_editor: None,
            plot_epoch: 0,
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
        app.status = "就绪".into();
        app
    }

    fn load_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.book_preview = None;
        self.analysis_curves.clear();
        self.active_layers.clear();
        self.adding_text = None;
        self.object_editor = None;
        self.plot_epoch = self.plot_epoch.wrapping_add(1);
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

    pub(crate) fn export_pdf(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(g) = self.project.graph(gid) else { return };
        let Some(path) = rfd::FileDialog::new()
            .set_title("导出 PDF")
            .add_filter("PDF 矢量图（嵌入字体）", &["pdf"])
            .set_file_name(format!("{}.pdf", g.name))
            .save_file()
        else {
            return;
        };
        // Finish conversion and font checks before opening/replacing the chosen output file.
        let result = crate::export::pdf_bytes(&svg::render_page(g, &self.project))
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        self.status = match result {
            Ok(()) => format!("已导出 {}（矢量 PDF，已嵌入字体）", path.display()),
            Err(e) => format!("导出失败：{e}"),
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
                // ponytail: 迁移期命令可能多建一次快照，旧调用迁完后移除外层 begin。
                if c.modifies {
                    self.begin_edit(&label);
                }
                (c.run)(self);
                // 已迁移的操作自行提交或回滚，旧命令仍由外层收尾。
                if c.modifies && self.history.is_pending() {
                    self.history.changed();
                    self.finish_edit();
                }
            }
            Err(why) => self.status = why.to_string(),
        }
    }

    /// 一次性项目编辑：Some 返回结果，None 取消，Err 回滚并显示原因。
    /// 实际改变才重算并记一步；重算诊断继续沿用既有 finish_edit 规则。
    fn edit<T>(&mut self, label: &str, f: impl FnOnce(&mut Project) -> Result<Option<T>, String>) -> Option<T> {
        self.finish_edit();
        self.history_group = None;
        self.begin_edit(label);
        match f(&mut self.project) {
            Ok(Some(value)) => {
                self.history.changed_if_different(&self.project);
                self.finish_edit();
                Some(value)
            }
            result => {
                self.project = self.history.rollback().unwrap();
                if let Err(error) = result {
                    self.status = error;
                }
                None
            }
        }
    }

    /// 连续编辑只接收已验证的实际修改；同组复用快照，换组或离散操作分步。
    /// 校验留在原草稿/局部编辑视图中，避免每帧复制整个项目来处理失败。
    fn edit_grouped(&mut self, label: &str, group: Option<egui::Id>, f: impl FnOnce(&mut Project)) {
        if self.history.is_pending() && (group.is_none() || self.last_history_group != group) {
            self.finish_edit();
        }
        if !self.history.is_pending() {
            self.begin_edit(label);
        } else {
            self.pending_repaint = true;
        }
        f(&mut self.project);
        self.history.changed();
        self.history_group = group;
        self.finish_frame_edit(group);
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
        self.adding_text = None;
        self.plot_epoch = self.plot_epoch.wrapping_add(1);
        self.object_editor = self.object_editor.filter(|edit| match edit.target {
            plot::ObjectTarget::Text(id) => project.graph(edit.graph).is_some_and(|g| g.page.text(id).is_some()),
            plot::ObjectTarget::Legend(index) => project.graph(edit.graph).is_some_and(|g| {
                g.page.layers().get(index).is_some_and(|l| l.legend)
                    && self
                        .project
                        .graph(edit.graph)
                        .is_some_and(|old| old.page.layers().len() == g.page.layers().len())
            }),
        });
        self.book_preview = None;
        self.project = project;
        match &mut self.dialog {
            Some(Dialog::LayerContents(edit)) => {
                if let Some(layer) = self
                    .project
                    .graph(edit.graph)
                    .and_then(|g| g.page.layers().get(edit.index))
                {
                    edit.before = layer.clone();
                    edit.draft = layer.clone();
                    edit.error = None;
                } else {
                    self.dialog = None;
                }
            }
            Some(Dialog::ColumnProps {
                book,
                before,
                edit,
                error,
                ..
            }) => {
                if let Some(column) = self.project.book(*book).and_then(|b| b.column(&edit.key)) {
                    *edit = dialogs::ColumnEdit::from(column);
                    *before = edit.clone();
                    *error = None;
                } else {
                    self.dialog = None;
                }
            }
            Some(Dialog::CategoryOrder {
                book,
                key,
                levels,
                error,
            }) => {
                if let Some(current) = self
                    .project
                    .book(*book)
                    .and_then(|b| b.column(key))
                    .and_then(|c| c.data.category_levels())
                {
                    *levels = current;
                    *error = None;
                } else {
                    self.dialog = None;
                }
            }
            _ => {}
        }
        self.analysis_curves.retain(|(graph, index), curves| {
            let Some(layer) = self.project.graph(*graph).and_then(|g| g.page.layers().get(*index)) else {
                return false;
            };
            curves.retain(|r| layer.plot(r, &|id| self.project.book(id)).is_some());
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
        if !matches!(
            self.dialog,
            Some(Dialog::LayerContents(_) | Dialog::ColumnProps { .. } | Dialog::CategoryOrder { .. })
        ) {
            self.dialog = None;
        }
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
        let active = layer.plot_keys(true).first().cloned();
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
        if !edit.draft.plot_keys(true).is_empty() {
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
        self.edit("图层内容", |project| {
            project.graph_mut(edit.graph).unwrap().page.layers_mut()[edit.index] = edit.draft.clone();
            Ok(Some(()))
        });
        if let Some(selected) = self.analysis_curves.get_mut(&(edit.graph, edit.index)) {
            selected.retain(|reference| edit.draft.plot(reference, &|id| self.project.book(id)).is_some());
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
        self.plot_epoch = self.plot_epoch.wrapping_add(1);
        self.object_editor = self
            .object_editor
            .filter(|e| e.graph != id || matches!(e.target, plot::ObjectTarget::Text(_)));
        let index = self.layer_index(id);
        let graph = self.project.graph_mut(id).unwrap();
        let mut layer = graph.page.layers()[index].clone();
        layer.placement = None;
        layer.legend_position = None;
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
            self.plot_draft = Some(self.project.graph(id).unwrap().page.layers()[self.layer_index(id)].draft());
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
                self.plot_epoch = self.plot_epoch.wrapping_add(1);
                self.object_editor = self
                    .object_editor
                    .filter(|e| e.graph != id || matches!(e.target, plot::ObjectTarget::Text(_)));
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
        // Clean live controls follow edits made through the ribbon or another open panel.
        match d {
            Dialog::LayerContents(edit) if edit.draft == edit.before => {
                if let Some(layer) = self
                    .project
                    .graph(edit.graph)
                    .and_then(|g| g.page.layers().get(edit.index))
                {
                    if layer != &edit.before {
                        edit.before = layer.clone();
                        edit.draft = layer.clone();
                        edit.error = None;
                    }
                }
            }
            Dialog::ColumnProps { book, before, edit, .. } if edit == before => {
                if let Some(column) = self.project.book(*book).and_then(|b| b.column(&edit.key)) {
                    let current = dialogs::ColumnEdit::from(column);
                    if &current != before {
                        *before = current.clone();
                        *edit = current;
                        self.book_preview = None;
                    }
                }
            }
            _ => {}
        }
        let layer_before = match d {
            Dialog::LayerContents(edit) => Some(edit.draft.clone()),
            _ => None,
        };
        let mut action = dialogs::show(ctx, d, &self.pal, &self.project);
        let closing = matches!(action, dialogs::Action::Close);
        if self.refresh_book_preview() {
            ctx.request_repaint();
        }
        if matches!(action, dialogs::Action::None | dialogs::Action::Close) {
            let live = match &self.dialog {
                Some(Dialog::AnalysisCurves { graph, layer, curves }) => {
                    let chosen: Vec<_> = curves
                        .iter()
                        .filter(|(_, _, on)| *on)
                        .map(|(r, _, _)| r.clone())
                        .collect();
                    (self.analysis_curves.get(&(*graph, *layer)) != Some(&chosen))
                        .then_some(dialogs::Action::AnalysisCurves(*graph, *layer, chosen))
                }
                Some(Dialog::LayerContents(edit)) if edit.draft != edit.before => {
                    Some(dialogs::Action::LayerContents(edit.clone()))
                }
                Some(Dialog::ColumnProps { book, edit, .. })
                    if self
                        .book_preview
                        .as_ref()
                        .filter(|p| !p.applied)
                        .and_then(|p| p.result.as_ref().ok())
                        .is_some_and(|preview| self.project.book(*book) != Some(preview)) =>
                {
                    Some(dialogs::Action::ColumnProps(*book, edit.clone()))
                }
                Some(Dialog::CategoryOrder { book, key, levels, .. })
                    if self
                        .book_preview
                        .as_ref()
                        .filter(|p| !p.applied)
                        .and_then(|p| p.result.as_ref().ok())
                        .is_some_and(|preview| self.project.book(*book) != Some(preview)) =>
                {
                    Some(dialogs::Action::CategoryOrder(*book, key.clone(), levels.clone()))
                }
                _ => None,
            };
            if let Some(live) = live {
                action = live;
            }
        }
        if !matches!(action, dialogs::Action::None) {
            ctx.request_repaint();
        }
        match action {
            dialogs::Action::LayerContents(edit) => match self.apply_layer_contents(&edit) {
                Ok(()) => {
                    if let Some(Dialog::LayerContents(contents)) = &mut self.dialog {
                        contents.before = edit.draft.clone();
                        contents.error = None;
                    }
                }
                Err(error) => {
                    if let Some(Dialog::LayerContents(contents)) = &mut self.dialog {
                        contents.error = Some(error);
                    }
                }
            },
            dialogs::Action::AnalysisCurves(graph, layer, curves) => {
                self.analysis_curves.insert((graph, layer), curves);
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
                let mut preview = self.book_preview.take();
                self.begin_edit("列属性");
                match self.apply_column_props(book, edit) {
                    Ok(()) => {
                        self.history.changed();
                        if let Some(Dialog::ColumnProps { before, edit, .. }) = &mut self.dialog {
                            *before = edit.clone();
                        }
                        if let Some(preview) = &mut preview {
                            preview.applied = true;
                        }
                    }
                    Err(e) => {
                        if let Some(Dialog::ColumnProps { error, .. }) = &mut self.dialog {
                            *error = Some(e);
                        }
                    }
                }
                self.book_preview = preview;
                let focus = ctx
                    .memory(|m| m.focused())
                    .filter(|id| egui::TextEdit::load_state(ctx, *id).is_some());
                if focus.is_some() || ctx.input(|i| i.pointer.primary_down()) {
                    self.history_group = Some(focus.unwrap_or_else(|| egui::Id::new("live-column-properties")));
                }
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
        if closing {
            self.finish_edit();
            self.dialog = None;
            self.book_preview = None;
            self.history_group = None;
        } else if self.refresh_book_preview() {
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
            (Some(BookPreview { edit: BookPreviewEdit::Properties(old_id, old), result: Ok(_), .. }),
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
        let applied = result.as_ref().is_ok_and(|book| self.project.book(id) == Some(book));
        self.book_preview = Some(BookPreview { edit, result, applied });
        true
    }

    fn preview_graph(&self, id: u64) -> Option<std::borrow::Cow<'_, scicanvas_core::Graph>> {
        use std::borrow::Cow;
        let graph = self.project.graph(id)?;
        if let Some(Dialog::LayerContents(edit)) = &self.dialog {
            if edit.graph == id && graph.page.layers().get(edit.index) == Some(&edit.before) {
                let valid = edit.draft.plot_keys(true).is_empty()
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

    pub(crate) fn start_text_annotation(&mut self) {
        if let Some(id) = self.target_graph() {
            self.finish_edit();
            self.show_item(id);
            self.explorer_focus = false;
            self.adding_text = Some(id);
            self.object_editor = None;
            self.status = "单击页面放置文字，Esc 取消".into();
        }
    }

    pub(crate) fn manage_annotations(&mut self) {
        if let Some(graph) = self.target_graph() {
            if let Some(text) = self.project.graph(graph).and_then(|g| g.page.annotations.last()) {
                self.object_editor = Some(ObjectEditor {
                    graph,
                    target: plot::ObjectTarget::Text(text.id),
                    focus: true,
                });
            } else {
                self.status = "当前图形没有文字标注".into();
            }
        }
    }

    pub(crate) fn auto_legend(&mut self) {
        if let Some(graph) = self.target_graph() {
            self.apply_object_action(graph, plot::ObjectAction::AutoLegend(self.layer_index(graph)));
        }
    }

    fn apply_object_action(&mut self, graph: u64, action: plot::ObjectAction) {
        use plot::{ObjectAction as A, ObjectTarget as T};
        if self.project.graph(graph).is_none() {
            return;
        }
        match action {
            A::CancelAdd => {
                self.adding_text = None;
                return;
            }
            A::Edit(target) => {
                self.finish_edit();
                self.object_editor = Some(ObjectEditor {
                    graph,
                    target,
                    focus: true,
                });
                return;
            }
            _ => {}
        }
        self.begin_edit(match action {
            A::Move(..) => "移动图形对象",
            A::AddText(..) => "添加文字",
            A::Delete(..) => "删除图形对象",
            _ => "图例自动避让",
        });
        let page = &mut self.project.graph_mut(graph).unwrap().page;
        let mut changed = false;
        match action {
            A::AddText(position) => match page.add_text(position) {
                Ok(id) => {
                    changed = true;
                    self.adding_text = None;
                    self.object_editor = Some(ObjectEditor {
                        graph,
                        target: T::Text(id),
                        focus: true,
                    });
                }
                Err(e) => self.status = e,
            },
            A::Move(T::Text(id), position) => {
                if let Some(text) = page.text_mut(id) {
                    changed = text.position != position;
                    text.position = position;
                }
            }
            A::Move(T::Legend(index), position) => {
                if let Some(layer) = page.layers_mut().get_mut(index) {
                    changed = layer.legend_position != Some(position);
                    layer.legend_position = Some(position);
                }
            }
            A::Delete(T::Text(id)) => {
                let n = page.annotations.len();
                page.annotations.retain(|t| t.id != id);
                changed = n != page.annotations.len();
            }
            A::Delete(T::Legend(index)) => {
                if let Some(layer) = page.layers_mut().get_mut(index) {
                    changed = layer.legend;
                    layer.legend = false;
                }
            }
            A::AutoLegend(index) => {
                if let Some(layer) = page.layers_mut().get_mut(index) {
                    changed = !layer.legend || layer.legend_position.is_some();
                    layer.legend = true;
                    layer.legend_position = None;
                }
            }
            _ => {}
        }
        if matches!(action, A::Delete(target) if self.object_editor.is_some_and(|e| e.graph == graph && e.target == target))
        {
            self.object_editor = None;
        }
        if changed {
            self.history.changed();
            self.pending_repaint = true;
        }
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
        let out = self.preview_graph(id).map(|g| {
            plot::show(
                ui,
                &g,
                &self.project,
                self.layer_index(id),
                plot::ObjectUi {
                    adding: self.adding_text == Some(id),
                    epoch: self.plot_epoch,
                    keyboard: self.active == Some(id) && !self.explorer_focus,
                },
            )
        });
        if let Some(at) = at {
            std::mem::swap(
                &mut self.project.books[at],
                preview.as_mut().unwrap().result.as_mut().unwrap(),
            );
        }
        self.book_preview = preview;
        let Some(out) = out else { return };
        if let Some(action) = out.action {
            self.apply_object_action(id, action);
        }
        if out.object_dragging {
            self.history_group = Some(egui::Id::new(("object-drag", id)));
        }
        if out.response.contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
            self.explorer_focus = false;
        }
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
        let object_id = curve_id.with("object");
        if resp.secondary_clicked() {
            ui.data_mut(|d| d.insert_temp(object_id, out.object));
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
        let menu_object = ui
            .data(|d| d.get_temp::<Option<plot::ObjectTarget>>(object_id))
            .flatten();
        let mut object_action = None;
        resp.context_menu(|ui| {
            if let Some(target) = menu_object {
                if ui.button("编辑文字/图例…").clicked() {
                    object_action = Some(plot::ObjectAction::Edit(target));
                    ui.close_menu();
                }
                if ui.button("删除对象").clicked() {
                    object_action = Some(plot::ObjectAction::Delete(target));
                    ui.close_menu();
                }
                if let plot::ObjectTarget::Legend(index) = target {
                    if ui.button("恢复自动避让").clicked() {
                        object_action = Some(plot::ObjectAction::AutoLegend(index));
                        ui.close_menu();
                    }
                }
                ui.separator();
            }
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
                ("graph.text.add", "添加文字标注"),
                ("graph.text.manage", "管理文字标注…"),
                ("graph.legend.auto", "图例自动避让"),
                ("graph.copy", "复制图形"),
                ("graph.export", "导出 SVG…"),
                ("graph.export_pdf", "导出 PDF…"),
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
        if let Some(action) = object_action {
            self.apply_object_action(id, action);
        }
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
        let present = self
            .project
            .graph(gid)
            .and_then(|g| g.page.layers().get(layer))
            .is_some_and(|l| l.plot_keys(true).iter().any(|k| k == key));
        if !present {
            self.finish_edit();
            return;
        }
        self.status = "已移除曲线".into();
        self.edit("移除曲线", |project| {
            project.graph_mut(gid).unwrap().page.layers_mut()[layer].remove_curve(key);
            Ok(Some(()))
        });
    }

    fn object_window(&mut self, ctx: &egui::Context) {
        use plot::ObjectTarget as T;
        let Some(mut edit) = self.object_editor else { return };
        let Some(before) = self.project.graph(edit.graph).cloned() else {
            self.object_editor = None;
            return;
        };
        let valid = match edit.target {
            T::Text(id) => before.page.text(id).is_some(),
            T::Legend(i) => before.page.layers().get(i).is_some_and(|l| l.legend),
        };
        if !valid {
            self.object_editor = None;
            return;
        }
        let (open, action) = edit.show(
            ctx,
            self.project.graph_mut(edit.graph).unwrap(),
            &mut self.history_group,
        );
        if self.project.graph(edit.graph) != Some(&before) {
            let after = std::mem::replace(self.project.graph_mut(edit.graph).unwrap(), before);
            self.edit_grouped("编辑图形对象", self.history_group, |project| {
                *project.graph_mut(edit.graph).unwrap() = after;
            });
            ctx.request_repaint();
        }
        self.object_editor = open.then_some(edit);
        if let Some(action) = action {
            self.finish_edit();
            self.apply_object_action(edit.graph, action);
        }
    }

    fn help_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("快速上手")
            .id(egui::Id::new("scicanvas-help"))
            .constrain_to(dialogs::floating_bounds(ctx))
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
        self.object_window(ctx);
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
#[path = "app/tests/mod.rs"]
mod history_tests;

#[cfg(test)]
#[path = "app/tests/windows.rs"]
mod snap_assist_tests;

#[cfg(test)]
#[path = "ui_review.rs"]
mod ui_review;
