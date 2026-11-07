//! 应用状态与界面调度、图形属性和分析操作。
//! 项目管理与子窗口位于 workspace，工作表显示与编辑位于 worksheet。

mod analysis_ops;
mod batch_export;
mod batch_plot;
mod dialog_actions;
mod excel_export;
mod filter;
mod find_replace;
mod graphics_tools;
mod notes;
mod peak_fit;
mod props;
mod signal_tools;
mod statistical_reports;
mod status;
use status::{Status, StatusKind};
mod tiff_export;
mod windows;
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
    Command(&'static str),
    Props,
    Filter,
    ClearFilter,
    AddToGraph(u64),
}

// 嵌套操作共享最外层事务，内部取消或失败使整次命令回滚。
enum EditAbort {
    Cancelled,
    Failed(String),
}

#[derive(Clone, Debug)]
pub(crate) enum PendingDiscard {
    Quit,
    Open(PathBuf),
    OpenDialog,
    Reset,
    New,
    Recover(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
struct RecalcInputs(Vec<RecalcBook>);
#[derive(Clone, Debug, PartialEq)]
struct RecalcBook {
    id: u64,
    filter: Option<scicanvas_core::filter::Filter>,
    columns: Vec<RecalcColumn>,
}
#[derive(Clone, Debug, PartialEq)]
struct RecalcColumn {
    key: String,
    version: u64,
    formula: Option<String>,
    operation: Option<scicanvas_core::ops::Operation>,
    role: Role,
    error_for: Option<String>,
}
impl RecalcInputs {
    fn of(project: &Project) -> Self {
        Self(
            project
                .books
                .iter()
                .map(|b| RecalcBook {
                    id: b.id,
                    filter: b.filter.clone(),
                    columns: b
                        .columns
                        .iter()
                        .map(|c| RecalcColumn {
                            key: c.key.clone(),
                            version: c.data.version(),
                            formula: c.formula.clone(),
                            operation: c.op.clone(),
                            role: c.role,
                            error_for: c.error_for.clone(),
                        })
                        .collect(),
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug)]
struct RecoveryCandidate {
    path: PathBuf,
    source: Option<PathBuf>,
    notice: Option<&'static str>,
    modified: Option<std::time::SystemTime>,
}
impl RecoveryCandidate {
    fn read(path: PathBuf) -> Self {
        use scicanvas_core::project_file::session;
        Self {
            source: session::recovery_source(&path),
            notice: session::recovery_notice(&path),
            modified: path.metadata().and_then(|m| m.modified()).ok(),
            path,
        }
    }
    fn time_text(&self) -> String {
        self.modified
            .and_then(|t| t.elapsed().ok())
            .map_or("未知".into(), |age| {
                let minutes = age.as_secs() / 60;
                if minutes < 1 {
                    "刚刚".into()
                } else if minutes < 60 {
                    format!("{minutes} 分钟前")
                } else if minutes < 24 * 60 {
                    format!("{} 小时前", minutes / 60)
                } else {
                    format!("{} 天前", minutes / (24 * 60))
                }
            })
    }
}

struct CleanupTask {
    started: std::time::Instant,
    worker: std::thread::JoinHandle<Result<(), String>>,
    after: Option<PendingDiscard>,
}

pub struct SciApp {
    cleanup_task: Option<CleanupTask>,
    deferred_discard: Option<PendingDiscard>,
    backup_job: Option<std::thread::JoinHandle<Result<(), String>>>,
    window_checked: bool,
    session_enabled: bool,
    smoke_test: bool,
    saved_generation: Option<u64>,
    pending_discard: Option<PendingDiscard>,
    close_confirmed: bool,
    pub(crate) settings: scicanvas_core::project_file::session::Settings,
    save_error: Option<String>,
    recovery_candidates: Vec<RecoveryCandidate>,
    modal_focus_restore: Option<egui::Id>,
    modal_was_active: bool,
    background_focus: Option<egui::Id>,
    background_blocked: bool,
    recovery_file: Option<PathBuf>,
    recovery_lock: Option<std::fs::File>,
    last_recovery: std::time::Instant,
    recalc_inputs: Option<RecalcInputs>,
    analysis_curves: HashMap<(u64, usize), Vec<scicanvas_core::plotspec::DataRef>>,
    pub(crate) project: Project,
    pub(crate) history: History,
    history_group: Option<egui::Id>,
    last_history_group: Option<egui::Id>,
    edit_depth: usize,
    edit_abort: Option<EditAbort>,
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
    reference_for: Option<(u64, usize)>,
    adding_text: Option<u64>,
    shape_tool: Option<(u64, scicanvas_core::page::ShapeKind)>,
    object_editor: Option<ObjectEditor>,
    plot_epoch: u64,
    active_layers: HashMap<u64, usize>,
    dialog_error: Option<String>,
    dialog_error_for: Option<std::mem::Discriminant<Dialog>>,
    pub(crate) plot_draft: Option<scicanvas_core::plotspec::PlotSpec>,
    pub(crate) status: Status,
    status_messages: std::collections::VecDeque<(String, Status)>,
    show_status_messages: bool,
    busy_action: Option<dialog_actions::DialogFrame>,
    busy_presented: bool,
    pub(crate) show_explorer: bool,
    pub(crate) show_help: bool,
    pub(crate) show_about: bool,
    renderer_info: String,
    pub(crate) confirm_delete: Option<u64>,
    /// 每个工作簿的单元格选区和编辑状态。
    pub(crate) sheets: HashMap<u64, SheetState>,
    /// 命令要复制到剪贴板的文字（命令拿不到 egui 上下文，在帧末尾交给 egui）。
    pub(crate) pending_copy: Option<String>,
    pending_paste: Option<String>,
    pub(crate) style_clipboard: Option<scicanvas_core::graph_theme::StyleClip>,
    pub(crate) style_target: Option<(u64, usize, Option<String>)>,
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

#[cfg(windows)]
fn renderer_notice(kind: eframe::wgpu::DeviceType) -> Option<&'static str> {
    (kind == eframe::wgpu::DeviceType::Cpu)
        .then_some("未检测到显卡加速，正在使用软件渲染，界面可能较慢；请更新显卡驱动")
}

impl SciApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let has_cjk = crate::fonts::setup(&cc.egui_ctx);
        theme::install(&cc.egui_ctx);
        // 功能区图标经共用渲染栈加载。
        crate::icons::install_loaders(&cc.egui_ctx);
        let mut app = Self::initial(theme::palette(cc.egui_ctx.theme()));
        app.session_enabled = true;
        app.smoke_test = std::env::var_os("SCICANVAS_SMOKE_TEST").is_some_and(|v| v == "1");
        app.settings = scicanvas_core::project_file::session::load_settings();
        app.theme_pref = match app.settings.theme.as_str() {
            "light" => egui::ThemePreference::Light,
            "dark" => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::System,
        };
        app.pending_theme = Some(app.theme_pref);
        if let Some(directory) = scicanvas_core::project_file::session::recovery_dir() {
            app.recovery_candidates = scicanvas_core::project_file::session::recoveries(&directory)
                .into_iter()
                .map(RecoveryCandidate::read)
                .collect();
        }
        if !has_cjk {
            app.status_warn("系统缺少中文字体，可通过 SCICANVAS_FONT 指定字体文件".into());
        }
        #[cfg(windows)]
        if let Some(render) = cc.wgpu_render_state.as_ref() {
            let info = render.adapter.get_info();
            app.renderer_info = format!(
                "{:?}；{}；{:?}；{} {}",
                info.backend, info.name, info.device_type, info.driver, info.driver_info
            );
            scicanvas_core::project_file::session::write_startup_log(&format!("图形后端：{}", app.renderer_info));
            if let Some(notice) = renderer_notice(info.device_type) {
                {
                    let status_kind = StatusKind::Warn;
                    let text: String = if app.status.is_empty() {
                        notice.into()
                    } else {
                        format!("{}；{notice}", app.status)
                    };
                    app.set_status(status_kind, text);
                };
            }
        }
        if let Some(path) = std::env::args_os().nth(1) {
            app.open_startup_path(std::path::Path::new(&path));
        }
        app
    }

    fn initial(pal: Palette) -> Self {
        let mut app = SciApp {
            cleanup_task: None,
            deferred_discard: None,
            backup_job: None,
            window_checked: false,
            session_enabled: false,
            smoke_test: false,
            saved_generation: Some(0),
            pending_discard: None,
            close_confirmed: false,
            settings: Default::default(),
            save_error: None,
            recovery_candidates: Vec::new(),
            modal_focus_restore: None,
            modal_was_active: false,
            background_focus: None,
            background_blocked: false,
            recovery_file: None,
            recovery_lock: None,
            last_recovery: std::time::Instant::now(),
            recalc_inputs: None,
            analysis_curves: HashMap::new(),
            book_preview: None,
            project: Project::default(),
            history: History::default(),
            history_group: None,
            last_history_group: None,
            edit_depth: 0,
            edit_abort: None,
            adding_text: None,
            shape_tool: None,
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
            reference_for: None,
            active_layers: HashMap::new(),
            dialog_error: None,
            dialog_error_for: None,
            plot_draft: None,
            status: Status::default(),
            status_messages: Default::default(),
            show_status_messages: false,
            busy_action: None,
            busy_presented: false,
            show_explorer: true,
            show_help: false,
            show_about: false,
            renderer_info: "未检测到图形后端".into(),
            confirm_delete: None,
            sheets: HashMap::new(),
            pending_copy: None,
            pending_paste: None,
            style_clipboard: None,
            style_target: None,
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
        app.status_info("就绪".into());
        app
    }

    fn load_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.reference_for = None;
        self.dialog = None;
        self.book_preview = None;
        self.analysis_curves.clear();
        self.active_layers.clear();
        self.adding_text = None;
        self.shape_tool = None;
        self.object_editor = None;
        self.plot_epoch = self.plot_epoch.wrapping_add(1);
        self.plot_draft = None;
        self.history = History::default();
        self.saved_generation = Some(self.history.generation());
        self.recalc_inputs = None;
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
        self.recalc_inputs = Some(RecalcInputs::of(&self.project));
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
                self.status_warn("请先单击一个工作簿窗口，使它成为当前窗口".into());
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
            self.status_warn("请先打开一个图形窗口".into());
        }
        found
    }

    pub(crate) fn plot(&mut self, t: PlotType) {
        if let Some(id) = self.active {
            let index = self.layer_index(id);
            if let Some(g) = self.project.graph_mut(id) {
                {
                    let mut status_kind = StatusKind::Success;
                    let text: String = match g.page.layers_mut()[index].set_plot_type(t) {
                        Ok(()) => format!("已把“{}”改为{}（Ctrl+Z 撤销）", g.name, t.label()),
                        Err(e) => {
                            status_kind = StatusKind::Error;
                            e
                        }
                    };
                    self.set_status(status_kind, text);
                };
                if t == PlotType::Pie {
                    self.append_pie_skipped(id);
                }
                return;
            }
        }
        let Some(bid) = self.active_book() else { return };
        let (sel, rows) = self.picked(bid);
        let visible_only = self.project.book(bid).is_some_and(|book| book.uses_visible_rows());
        match self.project.plot_from_selection(bid, &sel, t) {
            Ok((gid, msg)) => {
                if let Some(g) = self.project.graph_mut(gid) {
                    g.first_layer_mut().rows = rows;
                    g.first_layer_mut().visible_only = visible_only;
                }
                self.show_item(gid);
                {
                    let status_kind = StatusKind::Success;
                    let text: String = match rows {
                        Some((a, b)) => format!("{}（第 {}–{} 行）", msg, a + 1, b + 1),
                        None => msg,
                    };
                    self.set_status(status_kind, text);
                };
            }
            Err(e) => self.status_err(e),
        }
        if t == PlotType::Pie {
            if let Some(g) = self.active.and_then(|id| self.project.graph(id)) {
                let id = g.id;
                self.append_pie_skipped(id);
            }
        }
    }

    fn append_pie_skipped(&mut self, id: u64) {
        if let Some(g) = self.project.graph(id) {
            let skipped: usize = g
                .page
                .layers()
                .iter()
                .filter(|l| l.plot_type == PlotType::Pie)
                .map(|l| scicanvas_core::scene::pie_skipped(l, &|id| self.project.book(id)))
                .sum();
            self.status_warn(format!("{}；跳过 {skipped} 行非正值或缺失值", self.status));
        }
    }

    /// 用当前工作簿选中的 Y 列画多面板折线图，选区行范围对每个面板都生效。
    pub(crate) fn plot_panels(&mut self, panels: scicanvas_core::Panels) {
        let Some(bid) = self.active_book() else { return };
        let (sel, rows) = self.picked(bid);
        let visible_only = self.project.book(bid).is_some_and(|book| book.uses_visible_rows());
        match self.project.plot_panels(bid, &sel, PlotType::Line, panels) {
            Ok((gid, msg)) => {
                if let Some(g) = self.project.graph_mut(gid) {
                    for layer in g.page.layers_mut() {
                        layer.rows = rows;
                        layer.visible_only = visible_only;
                    }
                }
                self.show_item(gid);
                self.status_ok(msg);
            }
            Err(e) => self.status_err(e),
        }
    }

    pub(crate) fn import_csv(&mut self) {
        let Some(path) = self
            .file_dialog()
            .set_title("导入数据")
            .add_filter("数据文件", &["csv", "txt", "tsv", "dat", "xlsx", "xlsm", "xls"])
            .add_filter("Excel 工作簿", &["xlsx", "xlsm", "xls"])
            .add_filter("所有文件", &["*"])
            .pick_file()
        else {
            return;
        };
        let excel = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ["xlsx", "xlsm", "xls"].iter().any(|x| e.eq_ignore_ascii_case(x)));
        if excel {
            self.import_path(&path);
        } else {
            match std::fs::read(&path) {
                Ok(bytes) => {
                    self.dialog = Some(Dialog::ImportWizard(Box::new(dialogs::ImportWizard::new(path, bytes))))
                }
                Err(e) => self.status_err(format!("无法读取文件：{e}")),
            }
        }
    }

    pub(super) fn import_with_options(&mut self, path: std::path::PathBuf, options: csv::ImportOptions) -> bool {
        let result = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| csv::decode_import(&bytes, &options))
            .and_then(|text| csv::book_from_import(0, "", &text, &options));
        let mut book = match result {
            Ok(book) => book,
            Err(e) => {
                if let Some(Dialog::ImportWizard(wizard)) = &mut self.dialog {
                    wizard.error = Some(e);
                }
                return false;
            }
        };
        self.settings.last_dir = path.parent().map(std::path::Path::to_path_buf);
        let parent = self.current_folder();
        let base = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Book".into());
        self.edit_with(
            "导入数据",
            |project| {
                book.name = if project.books.iter().any(|b| b.name == base) {
                    project.unique_name(&format!("{base}_"))
                } else {
                    base.clone()
                };
                book.id = project.alloc_id();
                let id = book.id;
                project.books.push(book);
                project.move_project_item(id, parent)?;
                Ok(Some(id))
            },
            |app, id| {
                app.show_item(id);
                app.status_ok(format!("已导入 {}", path.display()));
            },
        );
        self.dialog = None;
        true
    }

    /// 把一个数据文件导入成新工作簿，放在当前文件夹里。Excel 工作簿的每张非空工作表各成一个工作簿。
    fn import_path(&mut self, path: &std::path::Path) {
        self.import_paths(&[path.to_path_buf()]);
    }

    fn import_paths(&mut self, paths: &[PathBuf]) {
        if let Some(path) = paths.first() {
            self.settings.last_dir = path.parent().map(std::path::Path::to_path_buf);
        }
        let parent = self.current_folder();
        self.edit_with(
            "导入数据",
            |project| {
                let mut shown = Vec::new();
                let mut messages = Vec::new();
                let mut failed = 0;
                for path in paths {
                    match Self::import_into(project, path, parent) {
                        Ok((id, message)) => {
                            shown.push(id);
                            messages.push(message);
                        }
                        Err(error) => {
                            failed += 1;
                            messages.push(error);
                        }
                    }
                }
                Ok(Some((shown, messages, failed)))
            },
            |app, (shown, messages, failed)| {
                let kind = if failed == 0 {
                    StatusKind::Success
                } else if shown.is_empty() {
                    StatusKind::Error
                } else {
                    StatusKind::Warn
                };
                for id in shown {
                    app.show_item(id);
                }
                app.set_status(kind, messages.join("；"));
            },
        );
    }

    fn import_into(
        project: &mut Project,
        path: &std::path::Path,
        parent: Option<u64>,
    ) -> Result<(u64, String), String> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                return Err(format!("无法读取 {}：{}", path.display(), e));
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
                    return Err(format!("无法导入 {}：{e}", path.display()));
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
            return Err("文件里没有数据".into());
        }
        let mut done = vec![];
        let mut last = None;
        for (base, table) in tables {
            let name = if project.books.iter().any(|b| b.name == base) {
                project.unique_name(&format!("{}_", base))
            } else {
                base
            };
            let id = project.alloc_id();
            let book = match table {
                Table::Rows(rows) => csv::book_from_table(id, &name, rows),
                Table::Text(text) => csv::book_from_text(id, &name, &text),
            };
            done.push(format!("{}（{} 行 × {} 列）", name, book.nrows(), book.columns.len()));
            project.books.push(book);
            project.move_project_item(id, parent)?;
            last = Some(id);
        }
        Ok((last.unwrap(), format!("已导入 {}：{}", path.display(), done.join("、"))))
    }

    pub(super) fn file_dialog(&self) -> rfd::FileDialog {
        rfd::FileDialog::new().set_directory(
            self.settings
                .last_dir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new(".")),
        )
    }
    pub(crate) fn open_project(&mut self) {
        self.request_discard(PendingDiscard::OpenDialog);
    }
    fn pick_project(&mut self) {
        let Some(path) = self
            .file_dialog()
            .set_title("打开项目")
            .add_filter("SciCanvas 项目", &["scv", "scproj"])
            .set_directory(
                self.settings
                    .last_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new(".")),
            )
            .pick_file()
        else {
            return;
        };
        self.open_path_now(&path);
    }

    pub(crate) fn save_graph_template(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(graph) = self.project.graph(gid).cloned() else {
            return;
        };
        let template = match scicanvas_core::project_file::graph_template_from_graph(&graph, &self.project) {
            Ok(template) => template,
            Err(error) => {
                self.status_err(error);
                return;
            }
        };
        let Some(path) = self
            .file_dialog()
            .set_title("保存图形模板")
            .add_filter("SciCanvas 图形模板", &["scgt"])
            .set_file_name(format!("{}.scgt", graph.name))
            .save_file()
        else {
            return;
        };
        {
            let mut status_kind = StatusKind::Success;
            let text: String = match scicanvas_core::project_file::save_graph_template(&path, &template) {
                Ok(()) => format!("已保存图形模板 {}", path.display()),
                Err(error) => {
                    status_kind = StatusKind::Error;
                    format!("保存图形模板失败：{error}")
                }
            };
            self.set_status(status_kind, text);
        };
    }

    pub(crate) fn apply_graph_template_dialog(&mut self) {
        let Some(book_id) = self.active_book_id() else { return };
        let Some(path) = self
            .file_dialog()
            .set_title("应用图形模板")
            .add_filter("SciCanvas 图形模板", &["scgt"])
            .pick_file()
        else {
            return;
        };
        match scicanvas_core::project_file::load_graph_template(&path) {
            Ok(template) => {
                self.apply_graph_template_to_book(template, book_id);
            }
            Err(error) => self.status_err(format!("无法应用图形模板：{error}")),
        }
    }

    pub(crate) fn apply_graph_template_to_book(
        &mut self,
        template: scicanvas_core::GraphTemplate,
        book_id: u64,
    ) -> Option<u64> {
        let template_name = template.name.clone();
        self.edit_with(
            "应用图形模板",
            |project| project.apply_graph_template(&template, book_id).map(Some),
            move |app, id| {
                app.show_item(id);
                app.status_ok(format!("已应用图形模板：{template_name}"));
                id
            },
        )
    }

    fn open_startup_path(&mut self, path: &std::path::Path) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if Self::is_project_file(path) {
            self.open_path_now(path);
        } else if ext.eq_ignore_ascii_case("scgt") {
            match scicanvas_core::project_file::load_graph_template(path) {
                Ok(template) => {
                    if let Some(id) = self.project.books.first().map(|book| book.id) {
                        self.apply_graph_template_to_book(template, id);
                    }
                }
                Err(error) => self.status_err(format!("无法应用图形模板：{error}")),
            }
        } else if ext.eq_ignore_ascii_case("scfn") {
            self.import_custom_fit_function(path);
        }
    }

    fn open_path(&mut self, path: &std::path::Path) {
        self.request_discard(PendingDiscard::Open(path.to_path_buf()));
    }
    fn open_path_now(&mut self, path: &std::path::Path) {
        match scicanvas_core::project_file::load_with_theme_notice(path) {
            Ok((p, renamed_theme)) => {
                self.clear_recovery();
                self.load_project(p, Some(path.to_path_buf()));
                self.settings.remember(path);
                self.persist_settings();
                self.project_loaded_status(format!("已打开 {}", path.display()), renamed_theme);
            }
            Err(e) => self.status_err(format!("无法打开项目：{}", e)),
        }
    }

    fn project_loaded_status(&mut self, message: String, renamed_theme: bool) {
        if renamed_theme {
            self.status_warn(format!("{message}；与内置默认主题重名的项目主题已自动重命名"));
        } else {
            self.status_ok(message);
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
                self.status_warn(format!("{}（一次只打开一个项目，其余文件已忽略）", self.status));
            }
            return;
        }
        if paths.is_empty() {
            return;
        }
        self.import_paths(&paths);
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
            painter.rect_filled(screen, 0.0, self.pal.scrim(110));
            painter.rect_stroke(screen.shrink(10.0), 8.0, egui::Stroke::new(3.0_f32, self.pal.accent));
            painter.text(
                screen.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(crate::theme::size::OVERLAY),
                self.pal.on_accent,
            );
        }
        if !dropped.is_empty() {
            self.open_dropped(dropped);
        }
    }

    pub(crate) fn save_project(&mut self, save_as: bool) {
        self.try_save_project(save_as);
    }

    fn try_save_project(&mut self, save_as: bool) -> bool {
        if !self.flush_sheet_edits() {
            return false;
        }
        self.finish_edit();
        let path = match (&self.project_path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let stem = self
                    .project_path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map_or_else(|| "项目".into(), |s| s.to_string_lossy());
                let Some(p) = self
                    .file_dialog()
                    .set_title("保存项目")
                    .set_directory(
                        self.settings
                            .last_dir
                            .as_deref()
                            .unwrap_or_else(|| std::path::Path::new(".")),
                    )
                    .add_filter("SciCanvas 项目", &["scv"])
                    .add_filter("旧版 JSON 项目", &["scproj"])
                    .set_file_name(format!("{stem}.scv"))
                    .save_file()
                else {
                    return false;
                };
                p
            }
        };
        self.save_project_to(&path)
    }

    fn save_project_to(&mut self, path: &std::path::Path) -> bool {
        if !self.flush_sheet_edits() {
            return false;
        }
        self.finish_edit();
        match scicanvas_core::project_file::save(path, &self.project) {
            Ok(()) => {
                self.saved_generation = Some(self.history.generation());
                self.project_path = Some(path.to_path_buf());
                self.settings.remember(path);
                self.persist_settings();
                self.clear_recovery();
                self.save_error = None;
                self.status_ok(format!("已保存到 {}", path.display()));
                true
            }
            Err(e) => {
                self.status_err(format!("保存失败：{e}"));
                self.save_error = Some(e);
                false
            }
        }
    }

    pub(crate) fn export_svg(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(g) = self.project.graph(gid).cloned() else {
            return;
        };
        let content = svg::render_page(&g, &self.project);
        let Some(path) = self
            .file_dialog()
            .set_title("导出图形")
            .add_filter("SVG 矢量图", &["svg"])
            .set_file_name(format!("{}.svg", g.name))
            .save_file()
        else {
            return;
        };
        self.settings.last_dir = path.parent().map(std::path::Path::to_path_buf);
        {
            let mut status_kind = StatusKind::Success;
            let text: String = match scicanvas_core::project_file::write_file_atomic(&path, content.as_bytes()) {
                Ok(()) => format!("已导出 {}", path.display()),
                Err(e) => {
                    status_kind = StatusKind::Error;
                    format!("导出失败：{}", e)
                }
            };
            self.set_status(status_kind, text);
        };
    }

    pub(crate) fn export_pdf(&mut self) {
        let Some(gid) = self.target_graph() else { return };
        let Some(g) = self.project.graph(gid) else { return };
        let Some(path) = self
            .file_dialog()
            .set_title("导出 PDF")
            .add_filter("PDF 矢量图（嵌入字体）", &["pdf"])
            .set_file_name(format!("{}.pdf", g.name))
            .save_file()
        else {
            return;
        };
        self.settings.last_dir = path.parent().map(std::path::Path::to_path_buf);
        // Finish conversion and font checks before opening/replacing the chosen output file.
        let result = crate::export::pdf_bytes(&svg::render_page(g, &self.project))
            .and_then(|bytes| scicanvas_core::project_file::write_file_atomic(&path, &bytes));
        {
            let mut status_kind = StatusKind::Success;
            let text: String = match result {
                Ok(()) => format!("已导出 {}（矢量 PDF，已嵌入字体）", path.display()),
                Err(e) => {
                    status_kind = StatusKind::Error;
                    format!("导出失败：{e}")
                }
            };
            self.set_status(status_kind, text);
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
        let Some(path) = self
            .file_dialog()
            .set_title("导出 PNG")
            .add_filter("PNG 图像", &["png"])
            .set_file_name(format!("{}.png", g.name))
            .save_file()
        else {
            return;
        };
        let result = crate::export::rasterize(&svg::render_page(&g, &self.project), dpi).and_then(|pixmap| {
            let bytes = crate::export::png_bytes(&pixmap, dpi)?;
            scicanvas_core::project_file::write_file_atomic(&path, &bytes)?;
            Ok((pixmap.width(), pixmap.height()))
        });
        {
            let mut status_kind = StatusKind::Success;
            let text: String = match result {
                Ok((w, h)) => format!("已导出 {}（{w} × {h} 像素，{dpi} dpi）", path.display()),
                Err(e) => {
                    status_kind = StatusKind::Error;
                    format!("导出失败：{e}")
                }
            };
            self.set_status(status_kind, text);
        };
    }

    /// 当前工作簿导出为 CSV（UTF-8 BOM，Excel 可直接打开，表头含单位，可重新导入）。
    pub(crate) fn export_csv(&mut self) {
        let Some(id) = self.active_book() else { return };
        let Some(book) = self.project.book(id) else { return };
        let content = scicanvas_core::project_file::book_csv(book);
        let (name, rows, cols) = (book.name.clone(), book.nrows(), book.columns.len());
        let Some(path) = self
            .file_dialog()
            .set_title("导出 CSV")
            .add_filter("CSV 表格", &["csv"])
            .set_file_name(format!("{name}.csv"))
            .save_file()
        else {
            return;
        };
        self.settings.last_dir = path.parent().map(std::path::Path::to_path_buf);
        {
            let mut status_kind = StatusKind::Success;
            let text: String = match scicanvas_core::project_file::write_file_atomic(&path, content.as_bytes()) {
                Ok(()) => format!("已导出 {}（{rows} 行 × {cols} 列）", path.display()),
                Err(e) => {
                    status_kind = StatusKind::Error;
                    format!("导出失败：{e}")
                }
            };
            self.set_status(status_kind, text);
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
        {
            let mut status_kind = StatusKind::Success;
            let text: String = match result {
                Ok(()) => format!("已复制 {}（{dpi} dpi 位图），可粘贴到 Word、PPT", g.name),
                Err(e) => {
                    status_kind = StatusKind::Error;
                    format!("复制失败：{e}")
                }
            };
            self.set_status(status_kind, text);
        };
    }

    // ---------------------------------------------------------------- 命令用到的查询与操作

    /// 执行一条命令（功能区、快捷键、命令面板都走这里）。不能执行时在状态栏说明原因。
    pub(crate) fn run_command(&mut self, id: &str) {
        if self.background_blocked || self.busy_action.is_some() {
            return;
        }
        let Some(c) = commands::find(id) else { return };
        self.dialog_error = None;
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
                    self.edit_action_with(
                        &label,
                        |app| {
                            (c.run)(app);
                            Ok(Some(()))
                        },
                        |_, ()| {},
                    );
                } else {
                    (c.run)(self);
                }
            }
            Err(why) => self.status_err(why.to_string()),
        }
    }

    /// 一次性项目编辑：Some 返回结果，None 取消，Err 回滚并显示原因。
    /// 实际改变才重算并记一步；重算诊断继续沿用既有 finish_edit 规则。
    fn edit<T>(&mut self, label: &str, f: impl FnOnce(&mut Project) -> Result<Option<T>, String>) -> Option<T> {
        self.edit_with(label, f, |_, value| value)
    }

    /// 展示成功结果后统一重算和提交，保留状态提示优先级；show 只更新界面。
    fn edit_with<T, R>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Project) -> Result<Option<T>, String>,
        show: impl FnOnce(&mut Self, T) -> R,
    ) -> Option<R> {
        self.edit_action_with(label, |app| f(&mut app.project), show)
    }

    /// 旧命令包含界面副作用，适配到同一事务实现；嵌套入口复用原快照。
    fn edit_action_with<T, R>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Self) -> Result<Option<T>, String>,
        show: impl FnOnce(&mut Self, T) -> R,
    ) -> Option<R> {
        if self.background_blocked {
            return None;
        }
        let outer = self.edit_depth == 0;
        if outer {
            self.finish_edit();
            self.history_group = None;
            self.edit_abort = None;
            self.begin_edit(label);
        } else if self.edit_abort.is_some() {
            return None;
        }
        let first_new_id = self.project.next_id;
        self.edit_depth += 1;
        let outcome = f(self);
        self.edit_depth -= 1;
        let result = match outcome {
            Ok(Some(value)) if self.edit_abort.is_none() => {
                if outer {
                    if let Some(theme) = &self.settings.default_graph_theme {
                        for graph in self.project.graphs.iter_mut().filter(|g| g.id >= first_new_id) {
                            for layer in graph.page.layers_mut() {
                                theme.apply(layer);
                            }
                        }
                    }
                }
                self.history.changed_if_different(&self.project);
                Some(show(self, value))
            }
            Ok(_) => {
                if self.edit_abort.is_none() {
                    self.edit_abort = Some(EditAbort::Cancelled);
                }
                None
            }
            Err(error) => {
                self.status_err(error.clone());
                if self.edit_abort.is_none() {
                    self.edit_abort = Some(EditAbort::Failed(error));
                }
                None
            }
        };
        if outer {
            if let Some(abort) = self.edit_abort.take() {
                self.project = self.history.rollback().unwrap();
                self.recalc_inputs = Some(RecalcInputs::of(&self.project));
                if let EditAbort::Failed(error) = abort {
                    self.status_err(error);
                }
            } else {
                self.finish_edit();
            }
        }
        result
    }

    /// 连续编辑只接收已验证的实际修改；同组复用快照，换组或离散操作分步。
    /// 校验留在原草稿/局部编辑视图中，避免每帧复制整个项目来处理失败。
    fn edit_grouped(&mut self, label: &str, group: Option<egui::Id>, f: impl FnOnce(&mut Project)) {
        self.edit_grouped_with(
            label,
            group,
            |project| {
                f(project);
                Ok(())
            },
            |_, ()| {},
        )
        .unwrap();
    }

    /// 原子校验失败不写项目，保留组内此前的有效输入；成功展示先于重算。
    /// f 返回 Err 时须保持项目原样，适用于既有的原子列属性操作。
    fn edit_grouped_with<T>(
        &mut self,
        label: &str,
        group: Option<egui::Id>,
        f: impl FnOnce(&mut Project) -> Result<T, String>,
        show: impl FnOnce(&mut Self, T),
    ) -> Result<(), String> {
        if self.background_blocked {
            return Ok(());
        }
        if self.history.is_pending() && (group.is_none() || self.last_history_group != group) {
            self.finish_edit();
        }
        let fresh = !self.history.is_pending();
        if fresh {
            self.begin_edit(label);
        } else {
            self.pending_repaint = true;
        }
        self.history_group = group;
        match f(&mut self.project) {
            Ok(value) => {
                self.history.changed();
                show(self, value);
                self.finish_frame_edit(group);
                Ok(())
            }
            Err(error) => {
                if fresh {
                    self.project = self.history.rollback().unwrap();
                    self.recalc_inputs = Some(RecalcInputs::of(&self.project));
                }
                Err(error)
            }
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
        self.recalc_inputs = Some(RecalcInputs::of(&self.project));
        {
            let status_kind = if problems.is_empty() {
                StatusKind::Success
            } else {
                StatusKind::Error
            };
            let text: String = if problems.is_empty() {
                "已重算公式列和分析结果".into()
            } else {
                format!("重算出错：{}", problems.join("；"))
            };
            self.set_status(status_kind, text);
        };
    }

    /// 有改动时按保存的列公式重算（并入同一步撤销）；出错时在状态栏说明。
    fn recalc_formulas(&mut self) {
        if self.history.has_changes() && self.recalc_inputs.as_ref() != Some(&RecalcInputs::of(&self.project)) {
            let problems = self.project.recalc_computed(false);
            self.recalc_inputs = Some(RecalcInputs::of(&self.project));
            if !problems.is_empty() {
                self.status_err(format!("公式重算出错：{}", problems.join("；")));
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
            self.status_ok(format!("已撤销：{label}"));
        }
    }

    pub(crate) fn redo(&mut self) {
        self.finish_edit();
        let label = self.history.redo_label().unwrap_or("").to_string();
        if let Some(project) = self.history.redo(&self.project) {
            self.restore_history(project);
            self.status_ok(format!("已重做：{label}"));
        }
    }

    fn restore_history(&mut self, project: Project) {
        self.adding_text = None;
        self.shape_tool = None;
        self.plot_epoch = self.plot_epoch.wrapping_add(1);
        self.object_editor = self.object_editor.filter(|edit| match edit.target {
            plot::ObjectTarget::Text(id) => project.graph(edit.graph).is_some_and(|g| g.page.text(id).is_some()),
            plot::ObjectTarget::Shape(id) => project.graph(edit.graph).is_some_and(|g| g.page.shape(id).is_some()),
            plot::ObjectTarget::ColorScale(index) => project.graph(edit.graph).is_some_and(|g| {
                g.page
                    .layers()
                    .get(index)
                    .is_some_and(|l| !l.color_scale.as_ref().is_some_and(|c| !c.visible))
            }),
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
        self.recalc_inputs = Some(RecalcInputs::of(&self.project));
        match &mut self.dialog {
            Some(Dialog::CustomNlFit(edit)) => edit.sync_saved(&self.project.fit_functions),
            Some(d @ Dialog::NlFitReport(_)) => d.sync_fit_reports(&self.project),
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
        self.restore_sheet_states();
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
            Some(
                Dialog::LayerContents(_)
                    | Dialog::ColumnProps { .. }
                    | Dialog::CategoryOrder { .. }
                    | Dialog::CustomNlFit(_)
                    | Dialog::NlFitReport(_)
                    | Dialog::NlFit { .. }
            )
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
        let title = match self.active.and_then(|id| self.project.name_of(id).map(|n| (id, n))) {
            Some((id, n)) if self.project.book(id).is_some() => format!("{} · 工作簿", n),
            Some((id, n)) if self.project.note(id).is_some() => format!("{} · Note", n),
            Some((_, n)) => format!("{} · 图形", n),
            None => "未选中窗口".to_string(),
        };
        if self.is_dirty() {
            format!("{title} •")
        } else {
            title
        }
    }

    /// 分析的输入：当前图形里的数据曲线，或当前工作簿里选中的列。
    pub(crate) fn analysis_source(&self) -> Option<Source> {
        if let Some(g) = self.active_graph_id() {
            let index = self.layer_index(g);
            let visible_only = self.active_layer().is_some_and(|layer| layer.visible_only);
            let source = if let Some(curves) = self.analysis_curves.get(&(g, index)) {
                Some(Source::Curves {
                    graph_id: g,
                    index,
                    curves: curves.clone(),
                })
            } else {
                self.active_layer()
                    .and_then(|layer| layer.book_id)
                    .map(|_| Source::Layer { graph_id: g, index })
            };
            return source.map(|source| {
                if visible_only {
                    Source::Visible(Box::new(source))
                } else {
                    source
                }
            });
        }
        self.active_book_id().map(|id| {
            let (selected, rows) = self.picked(id);
            let source = Source::Book { id, selected, rows };
            if self.project.book(id).is_some_and(|book| book.uses_visible_rows()) {
                Source::Visible(Box::new(source))
            } else {
                source
            }
        })
    }

    pub(crate) fn reset_project(&mut self) {
        self.request_discard(PendingDiscard::Reset);
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
        self.status_ok("图层内容已更新".into());
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
        self.object_editor = self.object_editor.filter(|e| {
            e.graph != id || matches!(e.target, plot::ObjectTarget::Text(_) | plot::ObjectTarget::Shape(_))
        });
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
        self.status_ok("已添加图层。".into());
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
                    spec.visible_only = self.project.book(id).is_some_and(|book| book.uses_visible_rows());
                    self.plot_draft = Some(spec);
                }
                Err(e) => self.status_err(e),
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
                self.status_ok("已按作图描述创建图形".into());
            }
            Err(e) => self.status_err(e),
        }
    }

    pub(crate) fn remove_layer(&mut self) {
        let Some(id) = self.active_graph_id() else { return };
        let index = self.layer_index(id);
        let graph = self.project.graph_mut(id).unwrap();
        match graph.page.remove_layer(index) {
            Ok(_) => {
                self.plot_epoch = self.plot_epoch.wrapping_add(1);
                self.object_editor = self.object_editor.filter(|e| {
                    e.graph != id || matches!(e.target, plot::ObjectTarget::Text(_) | plot::ObjectTarget::Shape(_))
                });
                self.analysis_curves.retain(|(graph_id, _), _| *graph_id != id);
                self.active_layers.insert(id, index.min(graph.page.layers().len() - 1));
                self.status_ok("已删除图层".into());
            }
            Err(e) => self.status_err(e),
        }
    }

    pub(crate) fn choose_axis_scale(&mut self, x: bool, scale: scicanvas_core::ticks::AxisScale) {
        let Some(gid) = self.active_graph_id() else { return };
        let index = self.layer_index(gid);
        {
            let mut status_kind = StatusKind::Success;
            let text: String =
                match self.project.graph_mut(gid).unwrap().page.layers_mut()[index].set_axis_scale(x, scale) {
                    Ok(()) => format!("{} 轴：{}", if x { "X" } else { "Y" }, scale.label()),
                    Err(error) => {
                        status_kind = StatusKind::Error;
                        error
                    }
                };
            self.set_status(status_kind, text);
        };
    }

    pub(crate) fn toggle_graph(&mut self, field: fn(&mut scicanvas_core::page::Layer) -> &mut bool, label: &str) {
        let Some(gid) = self.active_graph_id() else { return };
        let index = self.layer_index(gid);
        if let Some(g) = self.project.graph_mut(gid) {
            let v = field(&mut g.page.layers_mut()[index]);
            *v = !*v;
            let message = format!("{}：{}", label, if *v { "开" } else { "关" });
            self.status_ok(message);
        }
    }

    pub(crate) fn manage_reference_lines(&mut self) {
        if let Some(id) = self.active_graph_id() {
            self.reference_for = Some((id, self.layer_index(id)));
        }
    }

    pub(crate) fn add_reference_line(&mut self, axis: scicanvas_core::page::ReferenceAxis) {
        let Some(gid) = self.active_graph_id() else { return };
        let index = self.layer_index(gid);
        let layer = &mut self.project.graph_mut(gid).unwrap().page.layers_mut()[index];
        if axis == scicanvas_core::page::ReferenceAxis::YRight && !layer.has_right_axis_style() {
            self.status_warn("请先把一条曲线设置为右 Y 轴".into());
            return;
        }
        let scale = match axis {
            scicanvas_core::page::ReferenceAxis::X => layer.effective_x_scale(),
            scicanvas_core::page::ReferenceAxis::YLeft | scicanvas_core::page::ReferenceAxis::YRight => {
                layer.effective_y_scale()
            }
        };
        layer.reference_lines.push(scicanvas_core::page::ReferenceLine {
            axis,
            value: scale.default_value(),
            ..Default::default()
        });
        self.status_ok(format!("已添加{}参考线", axis.label()));
    }

    pub(crate) fn set_theme(&mut self, pref: egui::ThemePreference) {
        self.theme_pref = pref;
        self.pending_theme = Some(pref);
        {
            let status_kind = StatusKind::Success;
            let text: String = match pref {
                egui::ThemePreference::Light => "已切换到浅色主题",
                egui::ThemePreference::Dark => "已切换到深色主题",
                egui::ThemePreference::System => "主题跟随系统",
            }
            .into();
            self.set_status(status_kind, text);
        };
    }
}

impl eframe::App for SciApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_frame(ctx);
    }
}

impl SciApp {
    fn ui_frame(&mut self, ctx: &egui::Context) {
        dialogs::begin_keyboard_frame(ctx);
        if self.closing_frame(ctx) {
            return;
        }
        let modal_was_open = self.modal_active();
        self.prepare_modal_focus(ctx);
        if modal_was_open {
            self.background_blocked = false;
            self.show_modal(ctx);
            self.prepare_modal_focus(ctx);
        }
        self.background_frame(ctx);
        self.background_blocked = false;
        if !modal_was_open {
            self.show_modal(ctx);
        }
        self.restore_modal_focus(ctx);
        if !self.modal_active() {
            self.background_focus = ctx.memory(|m| m.focused());
        }
        self.session_tick(ctx);
        if self.smoke_test {
            self.quit = true;
            self.close_confirmed = true;
        }
        if self.quit && self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            ctx.request_repaint();
        }
    }
    fn show_modal(&mut self, ctx: &egui::Context) {
        if self.cleanup_blocks_ui() {
            self.cleanup_progress(ctx);
        } else {
            self.confirm_window(ctx);
            self.session_dialogs(ctx);
        }
    }
    fn background_frame(&mut self, ctx: &egui::Context) {
        self.pal = theme::palette(ctx.theme());
        // 命令面板打开时不处理全局快捷键，避免和输入框冲突
        if !self.background_blocked {
            self.shortcuts(ctx);
        }
        if std::mem::take(&mut self.pending_window_reset) {
            dialogs::reset_docks(ctx);
            ctx.memory_mut(|memory| memory.reset_areas());
        }
        dialogs::begin_settings(ctx);
        dialogs::begin_docks(ctx);
        crate::chrome::edges(ctx, &self.pal);
        if !self.background_blocked {
            self.dropped_files(ctx);
        }
        if !self.background_blocked
            && self.last_history_group.is_some()
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

        self.prepare_modal_focus(ctx);
        // 功能区和命令面板只读地访问程序状态，被点击的命令在绘制完成后执行
        let mut clicked: Option<&'static str> = None;
        let mut ui_state = std::mem::take(&mut self.ui);
        let pal = self.pal;
        ribbon::show(ctx, self, &mut ui_state, &pal, &mut clicked);
        if clicked == Some("app.quit") && !self.background_blocked {
            self.run_command("app.quit");
            clicked = None;
            self.prepare_modal_focus(ctx);
        }

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
            self.prepare_modal_focus(ctx);
            // 子窗口放在面板之间剩下的区域里
            self.refresh_book_preview();
            self.child_windows(ctx);
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(self.pal.workspace))
                .show(ctx, |_ui| {});
        }
        ribbon::palette(ctx, self, &mut ui_state, &pal, &mut clicked);
        self.ui = ui_state;

        if !self.background_blocked {
            if let Some(id) = clicked {
                self.run_command(id);
            }
        }
        self.prepare_modal_focus(ctx);
        if let Some(text) = self.pending_paste.take() {
            self.paste_text(ctx, text);
        }
        if let Some(t) = self.pending_copy.take() {
            ctx.copy_text(t);
        }

        self.props_window(ctx);
        self.reference_window(ctx);
        self.object_window(ctx);
        self.help_window(ctx);
        self.about_window(ctx);
        if self.background_blocked {
            if let Some(dialog) = self.dialog.as_mut() {
                let _ = dialogs::show(ctx, dialog, &self.pal, &self.project);
            }
        } else {
            self.dialogs(ctx);
        }
        if crate::builder::show(ctx, &self.project, &mut self.plot_draft) && !self.background_blocked {
            self.run_command("plot.apply_spec");
        }
        dialogs::end_docks(ctx);
        if !self.background_blocked {
            self.finish_frame_edit(self.history_group);
        }
        self.status_messages_window(ctx);
        dialogs::raise_settings(ctx);
        if std::mem::take(&mut self.pending_repaint) {
            ctx.request_repaint();
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

impl SciApp {
    pub(crate) fn is_dirty(&self) -> bool {
        self.history.has_changes()
            || self.saved_generation != Some(self.history.generation())
            || self.sheets.iter().any(|(id, s)| {
                s.edit.as_ref().is_some_and(|e| {
                    self.project
                        .book(*id)
                        .and_then(|b| b.columns.get(e.cell.c))
                        .is_some_and(|c| c.data.text_at(e.cell.r) != e.buf)
                })
            })
    }
    pub(crate) fn request_discard(&mut self, action: PendingDiscard) {
        // 未提交的单元格草稿也参与 dirty 判断；只有选择保存时才提交和重算。
        self.finish_edit();
        if self.is_dirty() {
            self.pending_discard = Some(action);
        } else {
            self.execute_discard(action);
        }
    }
    fn execute_discard(&mut self, action: PendingDiscard) {
        self.pending_discard = None;
        if self.cleanup_task.is_some() {
            self.deferred_discard = Some(action);
            return;
        }
        self.start_cleanup(Some(action));
    }
    fn complete_discard(&mut self, action: PendingDiscard) {
        self.pending_discard = None;
        match action {
            PendingDiscard::Quit => {
                self.close_confirmed = true;
                self.quit = true;
            }
            PendingDiscard::Open(path) => self.open_path_now(&path),
            PendingDiscard::Recover(path) => {
                self.restore_recovery(&path);
            }
            PendingDiscard::OpenDialog => self.pick_project(),
            PendingDiscard::Reset => {
                self.load_project(Project::sample(), None);
                self.status_ok("已恢复示例项目，撤销历史已清空".into());
            }
            PendingDiscard::New => {
                let mut p = Project::default();
                p.new_book();
                self.load_project(p, None);
                self.status_ok("已新建空白项目".into());
            }
        }
    }
    fn persist_settings(&mut self) {
        if !self.session_enabled {
            return;
        }
        self.settings.theme = match self.theme_pref {
            egui::ThemePreference::Light => "light",
            egui::ThemePreference::Dark => "dark",
            _ => "system",
        }
        .into();
        if let Err(error) = scicanvas_core::project_file::session::save_settings(&self.settings) {
            self.status_err(error);
        }
    }
    fn finish_backup(&mut self) -> Result<(), String> {
        self.backup_job.take().map_or(Ok(()), |job| {
            job.join().unwrap_or_else(|_| Err("自动备份任务中断".into()))
        })
    }
    fn clear_recovery(&mut self) {
        self.start_cleanup(None);
    }
    fn session_dialogs(&mut self, ctx: &egui::Context) {
        use dialogs::{
            actions, modal,
            ActionKind::{Danger, Plain, Primary},
        };
        if let Some(error) = self.save_error.clone() {
            let choice = modal(ctx, "保存失败", |ui| {
                ui.colored_label(self.pal.error, error);
                actions(ui, &[(Primary, "知道了")], dialogs::Keys::Modal, Ok(()))
            })
            .flatten();
            if choice.is_some() || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.save_error = None;
            }
            return;
        }
        if self.pending_discard.is_some() {
            let name = self
                .project_path
                .as_deref()
                .and_then(|p| p.file_name())
                .map_or("未命名项目".into(), |n| n.to_string_lossy().into_owned());
            let choice = modal(ctx, "未保存的修改", |ui| {
                ui.label(format!("“{name}”有未保存的修改，是否保存？"));
                actions(
                    ui,
                    &[(Primary, "保存"), (Danger, "不保存"), (Plain, "取消")],
                    dialogs::Keys::Modal,
                    Ok(()),
                )
            })
            .flatten();
            if choice == Some(2) {
                self.pending_discard = None;
            } else if choice == Some(1) || (choice == Some(0) && self.try_save_project(false)) {
                if let Some(action) = self.pending_discard.take() {
                    self.execute_discard(action);
                }
            }
            return;
        }
        if let Some(candidate) = self.recovery_candidates.first().cloned() {
            let title = if self.recovery_candidates.len() > 1 {
                format!("恢复未保存的工作（1/{}）", self.recovery_candidates.len())
            } else {
                "恢复未保存的工作".into()
            };
            let choice = modal(ctx, &title, |ui| {
                ui.label("发现上次会话的自动备份，是否恢复？");
                ui.label(
                    candidate
                        .source
                        .as_ref()
                        .map_or("未命名项目".into(), |p| p.display().to_string()),
                );
                ui.label(format!("备份时间：{}", candidate.time_text()));
                if let Some(notice) = candidate.notice {
                    ui.colored_label(self.pal.warn, notice);
                }
                actions(
                    ui,
                    &[(Primary, "恢复"), (Danger, "放弃这份备份"), (Plain, "稍后")],
                    dialogs::Keys::Modal,
                    Ok(()),
                )
            })
            .flatten();
            match choice {
                Some(0) => {
                    self.request_discard(PendingDiscard::Recover(candidate.path));
                    self.recovery_candidates.clear();
                }
                Some(1) => match scicanvas_core::project_file::session::discard_recovery(&candidate.path) {
                    Ok(()) => {
                        self.recovery_candidates.remove(0);
                    }
                    Err(error) => self.status_err(error),
                },
                Some(2) => self.recovery_candidates.clear(),
                _ => {}
            }
        }
    }
    fn restore_recovery(&mut self, path: &std::path::Path) {
        match (
            scicanvas_core::project_file::session::lock_recovery(path),
            scicanvas_core::project_file::load_with_theme_notice(path),
        ) {
            (Ok(lock), Ok((mut project, renamed_theme))) => {
                project.recalc_computed(false);
                let source = scicanvas_core::project_file::session::recovery_source(path);
                self.load_project(project, source);
                self.saved_generation = None;
                self.recovery_file = Some(path.to_path_buf());
                self.recovery_lock = Some(lock);
                self.project_loaded_status("已恢复自动备份，请保存项目".into(), renamed_theme);
            }
            (_, Err(error)) => self.status_err(format!("恢复失败：{error}")),
            (Err(error), _) => self.status_err(format!("恢复文件正在使用：{error}")),
        }
    }
    fn backup_recovery(&mut self) -> Result<(), String> {
        use scicanvas_core::project_file::session;
        if !self.is_dirty() || self.backup_job.is_some() || self.cleanup_task.is_some() {
            return Ok(());
        }
        if self.recovery_file.is_none() {
            let directory = session::recovery_dir().ok_or("没有可用的恢复目录")?;
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = directory.join(format!("recovery-{}-{stamp}.scproj", std::process::id()));
            self.recovery_lock = Some(session::lock_recovery(&path).map_err(|e| e.to_string())?);
            self.recovery_file = Some(path);
        }
        let mut snapshot = self.project.clone();
        let drafts: Vec<_> = self
            .sheets
            .iter()
            .filter_map(|(&id, s)| s.edit.as_ref().map(|e| (id, e.clone())))
            .collect();
        let path = self.recovery_file.as_ref().unwrap().clone();
        let source = self.project_path.clone();
        self.backup_job = Some(
            std::thread::Builder::new()
                .name("scicanvas-recovery".into())
                .spawn(move || {
                    for (id, e) in drafts {
                        if let Some(col) = snapshot.book_mut(id).and_then(|b| b.columns.get_mut(e.cell.c)) {
                            if !col.data.accepts(&e.buf) {
                                col.data = col.data.converted(scicanvas_core::DataKind::Text)?.into();
                            }
                            col.set(e.cell.r, e.buf);
                        }
                    }
                    session::save_recovery(&path, &snapshot, source.as_deref())
                })
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }
    fn session_tick(&mut self, ctx: &egui::Context) {
        if self.cleanup_task.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        self.check_window_position(ctx);
        if self.backup_job.as_ref().is_some_and(|j| j.is_finished()) {
            if let Err(error) = self.finish_backup() {
                self.status_err(error);
            }
        }
        if self.backup_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        ctx.input(|i| {
            let v = i.viewport();
            self.settings.maximized = v.maximized.unwrap_or(false);
            if !self.settings.maximized {
                if let Some(r) = v.outer_rect {
                    self.settings.window = Some([r.left(), r.top(), r.width(), r.height()]);
                }
            }
        });
        if self.session_enabled && self.is_dirty() {
            let interval = std::time::Duration::from_secs(300);
            let elapsed = self.last_recovery.elapsed();
            if elapsed >= interval {
                if let Err(error) = self.backup_recovery() {
                    self.status_err(error);
                }
                self.last_recovery = std::time::Instant::now();
            }
            ctx.request_repaint_after(interval.saturating_sub(self.last_recovery.elapsed()));
        }
    }
}

impl SciApp {
    /// 保存和切换项目先接收有效的单元格草稿，类型转换沿用原确认窗口。
    fn flush_sheet_edits(&mut self) -> bool {
        let edits: Vec<_> = self
            .sheets
            .iter()
            .filter_map(|(&id, s)| s.edit.as_ref().map(|e| (id, e.clone())))
            .collect();
        for (id, edit) in &edits {
            if let Some(col) = self.project.book(*id).and_then(|b| b.columns.get(edit.cell.c)) {
                if !col.data.accepts(&edit.buf) {
                    self.dialog = Some(Dialog::ConvertText {
                        book: *id,
                        key: col.key.clone(),
                        row: edit.cell.r,
                        value: edit.buf.clone(),
                    });
                    self.pending_discard = None;
                    self.status_warn("请先确认当前单元格的类型转换".into());
                    return false;
                }
            }
        }
        if edits.is_empty() {
            return true;
        }
        self.edit("编辑单元格", |p| {
            for (id, e) in &edits {
                if let Some(c) = p.book_mut(*id).and_then(|b| b.columns.get_mut(e.cell.c)) {
                    if c.data.text_at(e.cell.r) != e.buf {
                        c.set(e.cell.r, e.buf.clone());
                    }
                }
            }
            Ok(Some(()))
        });
        for (id, _) in edits {
            if let Some(sheet) = self.sheets.get_mut(&id) {
                sheet.edit = None;
            }
        }
        true
    }
}

impl SciApp {
    fn restore_sheet_states(&mut self) {
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
    }
}

impl SciApp {
    fn check_window_position(&mut self, ctx: &egui::Context) {
        if self.window_checked {
            return;
        }
        let Some((rect, inner)) = ctx.input(|i| i.viewport().outer_rect.zip(i.viewport().inner_rect)) else {
            return;
        };
        let areas = crate::monitors::work_areas(ctx.pixels_per_point());
        if areas.is_empty() {
            return;
        }
        self.window_checked = true;
        if let Some(target) = crate::monitors::visible_window(rect, &areas) {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(target.min));
            if let Some(size) = crate::monitors::restore_size(rect, inner, target) {
                if size.x < 800. || size.y < 500. {
                    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(800., 500.).min(size)));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            }
        }
    }
}

impl SciApp {
    /// 完成备份、文件清理和退出偏好写入都在工作线程；界面只轮询结果。
    fn start_cleanup(&mut self, after: Option<PendingDiscard>) {
        if self.cleanup_task.is_some() {
            return;
        }
        let preferences = if self.session_enabled && matches!(after, Some(PendingDiscard::Quit)) {
            let mut settings = self.settings.clone();
            settings.theme = match self.theme_pref {
                egui::ThemePreference::Light => "light",
                egui::ThemePreference::Dark => "dark",
                _ => "system",
            }
            .into();
            Some(settings)
        } else {
            None
        };
        if self.backup_job.is_none() && self.recovery_file.is_none() && preferences.is_none() {
            if let Some(action) = after {
                self.complete_discard(action);
            }
            return;
        }
        let backup = self.backup_job.take();
        let file = self.recovery_file.take();
        let lock = self.recovery_lock.take();
        let worker = std::thread::Builder::new()
            .name("scicanvas-cleanup".into())
            .spawn(move || {
                use scicanvas_core::project_file::session;
                let result = backup.map_or(Ok(()), |job| {
                    job.join().unwrap_or_else(|_| Err("自动备份任务中断".into()))
                });
                let removed = file.as_ref().is_some_and(|path| {
                    let _ = session::retire_recovery(path);
                    std::fs::remove_file(path).map_or_else(|e| e.kind() == std::io::ErrorKind::NotFound, |_| true)
                });
                drop(lock);
                if let Some(path) = file {
                    let _ = std::fs::remove_file(path.with_extension("lock"));
                    if removed {
                        let _ = std::fs::remove_file(path.with_extension("source"));
                    }
                }
                preferences
                    .map_or(Ok(()), |settings| session::save_settings(&settings))
                    .and(result)
            });
        match worker {
            Ok(worker) => {
                self.cleanup_task = Some(CleanupTask {
                    worker,
                    after,
                    started: std::time::Instant::now(),
                });
                self.pending_repaint = true;
            }
            Err(error) => self.status_err(format!("启动退出清理失败：{error}")),
        }
        self.last_recovery = std::time::Instant::now();
    }
    fn finish_cleanup(&mut self) {
        let Some(task) = self.cleanup_task.take() else { return };
        if let Err(error) = task
            .worker
            .join()
            .unwrap_or_else(|_| Err("恢复文件清理任务中断".into()))
        {
            self.status_err(error);
        }
        self.last_recovery = std::time::Instant::now();
        if let Some(action) = self.deferred_discard.take() {
            self.execute_discard(action);
        } else if let Some(action) = task.after {
            self.complete_discard(action);
        }
    }
    fn poll_cleanup(&mut self, ctx: &egui::Context) {
        if self.cleanup_task.as_ref().is_some_and(|t| t.worker.is_finished()) {
            self.finish_cleanup();
        }
        if self.cleanup_task.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
    fn cleanup_blocks_ui(&self) -> bool {
        self.deferred_discard.is_some() || self.cleanup_task.as_ref().is_some_and(|t| t.after.is_some())
    }
    fn cancel_cleanup_discard(&mut self) {
        self.deferred_discard = None;
        if let Some(task) = &mut self.cleanup_task {
            task.after = None;
        }
    }
    fn cleanup_progress(&mut self, ctx: &egui::Context) {
        if !self
            .cleanup_task
            .as_ref()
            .is_some_and(|t| t.started.elapsed() >= std::time::Duration::from_millis(300))
        {
            return;
        }
        let choice = dialogs::modal(ctx, "正在完成", |ui| {
            ui.spinner();
            ui.label("正在完成自动备份和恢复文件清理…");
            dialogs::actions(
                ui,
                &[(dialogs::ActionKind::Plain, "返回项目")],
                dialogs::Keys::Modal,
                Ok(()),
            )
        })
        .flatten();
        if choice.is_some() {
            self.cancel_cleanup_discard();
        }
    }
}

impl SciApp {
    fn closing_frame(&mut self, ctx: &egui::Context) -> bool {
        self.poll_cleanup(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.close_confirmed {
            self.request_discard(PendingDiscard::Quit);
            if !self.close_confirmed {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        if self.quit && self.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            ctx.request_repaint();
            return true;
        }
        false
    }
}

impl SciApp {
    pub(crate) fn modal_active(&self) -> bool {
        self.pending_discard.is_some()
            || self.save_error.is_some()
            || !self.recovery_candidates.is_empty()
            || self.confirm_delete.is_some()
            || self.cleanup_blocks_ui()
    }
    fn prepare_modal_focus(&mut self, ctx: &egui::Context) {
        self.background_blocked = self.modal_active();
        if self.background_blocked && !self.modal_was_active {
            self.modal_focus_restore = ctx.memory(|m| m.focused()).or(self.background_focus);
            if let Some(id) = self.modal_focus_restore {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
            self.modal_was_active = true;
        }
    }
    fn restore_modal_focus(&mut self, ctx: &egui::Context) {
        if self.modal_was_active && !self.modal_active() {
            if let Some(id) = self.modal_focus_restore.take() {
                ctx.memory_mut(|m| m.request_focus(id));
            }
            self.modal_was_active = false;
            ctx.request_repaint();
        }
    }
}
