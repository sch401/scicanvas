//! 主界面：功能区、项目管理器、多文档子窗口（工作簿 / 图形）、状态栏，以及各个命令背后的操作。

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui::{self, RichText};
use egui_extras::{Column as TCol, TableBuilder};
use scicanvas_core::analysis::{NormalizeMethod, SmoothMethod};
use scicanvas_core::history::History;
use scicanvas_core::ops::Source;
use scicanvas_core::{csv, round_sig, svg, PlotType, Project, Role};

use crate::commands;
use crate::dialogs::{self, Dialog};
use crate::mdi::{self, Geom, WinState};
use crate::plot;
use crate::ribbon::{self, Ctx, UiState};
use crate::sheet::{Cell, Drag, Edit, Range, SheetState};
use crate::theme::{self, Palette};

const META_ROWS: [&str; 3] = ["长名称", "单位", "注释"];

/// 列标题右键菜单里的操作。
#[derive(Clone, Copy)]
enum ColAction {
    SetRole(Role),
    InsertRight,
    Clear,
    Delete,
    Plot(PlotType),
}

pub struct SciApp {
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
    /// 功能区、文件页、命令面板的界面状态。
    pub(crate) ui: UiState,
    /// 当前打开的分析对话框。
    pub(crate) dialog: Option<Dialog>,
    pub(crate) quit: bool,
    /// 子窗口的位置、大小和最小化 / 最大化状态。
    pub(crate) geoms: HashMap<u64, Geom>,
    /// 上一帧的工作区矩形（排列窗口时用）。
    pub(crate) ws: egui::Rect,
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
            project: Project::default(),
            history: History::new(&Project::default()),
            history_group: None,
            last_history_group: None,
            open: HashMap::new(),
            selection: HashMap::new(),
            active: None,
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
            ui: UiState::default(),
            dialog: None,
            quit: false,
            geoms: HashMap::new(),
            ws: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 650.0)),
        };
        app.load_project(Project::sample(), None);
        app.status = "就绪 — 单击列标题选列，再点功能区里的绘图按钮；Ctrl+K 搜索任何命令，F1 查看快速上手".into();
        app
    }

    fn load_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.active_layers.clear();
        self.plot_draft = None;
        self.history = History::new(&project);
        self.history_group = None;
        self.last_history_group = None;
        self.open.clear();
        self.selection.clear();
        self.geoms.clear();
        for b in &project.books {
            self.open.insert(b.id, true);
        }
        for g in &project.graphs {
            self.open.insert(g.id, true);
        }
        self.active = project
            .graphs
            .last()
            .map(|g| g.id)
            .or(project.books.last().map(|b| b.id));
        self.project = project;
        self.project_path = path;
        self.props_for = None;
        self.sheets.clear();
    }

    fn show_item(&mut self, id: u64) {
        self.open.insert(id, true);
        if let Some(g) = self.geoms.get_mut(&id) {
            if g.state == WinState::Min {
                g.state = WinState::Normal;
            }
        }
        self.active = Some(id);
        self.raise = Some(id);
    }

    fn all_ids(&self) -> Vec<u64> {
        self.project
            .books
            .iter()
            .map(|b| b.id)
            .chain(self.project.graphs.iter().map(|g| g.id))
            .collect()
    }

    // ---------------------------------------------------------------- 操作

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

    pub(crate) fn selected_keys(&self, book_id: u64) -> Vec<String> {
        self.selection.get(&book_id).cloned().unwrap_or_default()
    }

    /// 单元格选区（一个格子不算）。
    pub(crate) fn multi_range(&self, book_id: u64) -> Option<Range> {
        self.sheets.get(&book_id).and_then(|s| s.range).filter(|r| r.is_multi())
    }

    /// 要处理的列和行：优先用列标题选择；没有选列时用单元格选区覆盖的列和行。
    pub(crate) fn picked(&self, book_id: u64) -> (Vec<String>, Option<(usize, usize)>) {
        let sel = self.selected_keys(book_id);
        if !sel.is_empty() {
            return (sel, None);
        }
        match (self.multi_range(book_id), self.project.book(book_id)) {
            (Some(r), Some(b)) if !b.columns.is_empty() => {
                let keys = b.columns[r.c0()..=r.c1().min(b.columns.len() - 1)]
                    .iter()
                    .map(|c| c.key.clone())
                    .collect();
                let rows = if r.r1() > r.r0() { Some((r.r0(), r.r1())) } else { None };
                (keys, rows)
            }
            _ => (sel, None),
        }
    }

    pub(crate) fn copy_range(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        if let Some(b) = self.project.book(bid) {
            self.pending_copy = Some(b.range_tsv(r.c0(), r.c1(), r.r0(), r.r1()));
            self.status = format!("已复制 {} 行 × {} 列，可直接粘贴到 Excel", r.rows(), r.cols());
        }
    }

    pub(crate) fn clear_range(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        if let Some(b) = self.project.book_mut(bid) {
            b.clear_range(r.c0(), r.c1(), r.r0(), r.r1());
            self.status = format!("已清除 {} 行 × {} 列", r.rows(), r.cols());
        }
    }

    pub(crate) fn new_book(&mut self) {
        let id = self.project.new_book();
        self.show_item(id);
        self.status = format!(
            "已新建 {}，可直接输入数据，或从 Excel 复制一块区域后在单元格里粘贴",
            self.project.name_of(id).unwrap_or("")
        );
    }

    pub(crate) fn plot(&mut self, t: PlotType) {
        if let Some(id) = self.active {
            let index = self.layer_index(id);
            if let Some(g) = self.project.graph_mut(id) {
                g.page.layers_mut()[index].plot_type = t;
                self.status = format!("{} 已改为{}", g.name, t.label());
                return;
            }
        }
        let Some(bid) = self.active_book() else { return };
        let (sel, rows) = self.picked(bid);
        match self.project.plot_from_selection(bid, &sel, t) {
            Ok((gid, msg)) => {
                if let Some(g) = self.project.graph_mut(gid) {
                    g.rows = rows;
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

    pub(crate) fn set_role(&mut self, role: Role) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.picked(bid).0;
        if sel.is_empty() {
            self.status = "请先单击列标题选中列".into();
            return;
        }
        if let Some(b) = self.project.book_mut(bid) {
            for c in &mut b.columns {
                if sel.contains(&c.key) {
                    c.role = role;
                }
            }
        }
        let name = match role {
            Role::X => "X",
            Role::Y => "Y",
            Role::Ignore => "忽略",
        };
        self.status = format!("已将 {} 列设为 {}", sel.join(", "), name);
    }

    pub(crate) fn add_column(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.selected_keys(bid);
        if let Some(b) = self.project.book_mut(bid) {
            let at = sel
                .last()
                .and_then(|k| b.column_index(k))
                .map(|i| i + 1)
                .unwrap_or(b.columns.len());
            let key = b.insert_column(at, Role::Y);
            self.status = format!("已添加列 {}", key);
        }
    }

    pub(crate) fn clear_columns(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.selected_keys(bid);
        if sel.is_empty() {
            self.status = "请先单击列标题选中列".into();
            return;
        }
        if let Some(b) = self.project.book_mut(bid) {
            for c in &mut b.columns {
                if sel.contains(&c.key) {
                    c.data.clear();
                }
            }
        }
        self.status = format!("已清空 {} 列", sel.join(", "));
    }

    pub(crate) fn delete_columns(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.selected_keys(bid);
        if sel.is_empty() {
            self.status = "请先单击列标题选中要删除的列".into();
            return;
        }
        self.project.delete_columns(bid, &sel);
        self.selection.remove(&bid);
        self.sheets.remove(&bid);
        self.status = format!("已删除 {} 列", sel.join(", "));
    }

    fn delete_item(&mut self, id: u64) {
        self.active_layers.remove(&id);
        let name = self.project.name_of(id).unwrap_or("").to_string();
        self.project.remove(id);
        self.open.remove(&id);
        self.selection.remove(&id);
        if self.active == Some(id) {
            self.active = None;
        }
        if self.props_for == Some(id) {
            self.props_for = None;
        }
        self.sheets.remove(&id);
        self.status = format!("已删除 {}", name);
    }

    pub(crate) fn import_csv(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("导入数据")
            .add_filter("数据文件", &["csv", "txt", "tsv", "dat"])
            .add_filter("所有文件", &["*"])
            .pick_file()
        else {
            return;
        };
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.status = format!("无法读取 {}：{}", path.display(), e);
                return;
            }
        };
        let rows = csv::parse_table(&csv::decode(&bytes));
        if rows.is_empty() {
            self.status = "文件里没有数据".into();
            return;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Book".into());
        let name = if self.project.books.iter().any(|b| b.name == stem) {
            self.project.unique_name(&format!("{}_", stem))
        } else {
            stem
        };
        let id = self.project.alloc_id();
        let book = csv::book_from_table(id, &name, rows);
        self.status = format!(
            "已导入 {}：{} 行 × {} 列",
            path.display(),
            book.nrows(),
            book.columns.len()
        );
        self.project.books.push(book);
        self.show_item(id);
    }

    pub(crate) fn open_project(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("打开项目")
            .add_filter("SciCanvas 项目", &["scproj"])
            .pick_file()
        else {
            return;
        };
        let result = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| scicanvas_core::project_file::from_json(&t));
        match result {
            Ok(p) => {
                self.load_project(p, Some(path.clone()));
                self.status = format!("已打开 {}", path.display());
            }
            Err(e) => self.status = format!("无法打开项目：{}", e),
        }
    }

    pub(crate) fn save_project(&mut self, save_as: bool) {
        let path = match (&self.project_path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let Some(p) = rfd::FileDialog::new()
                    .set_title("保存项目")
                    .add_filter("SciCanvas 项目", &["scproj"])
                    .set_file_name("项目.scproj")
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

    // ---------------------------------------------------------------- 命令用到的查询与操作

    /// 执行一条命令（功能区、快捷键、命令面板都走这里）。不能执行时在状态栏说明原因。
    pub(crate) fn run_command(&mut self, id: &str) {
        let Some(c) = commands::find(id) else { return };
        self.record_changes("编辑项目", self.history_group);
        self.last_history_group = None;
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
                (c.run)(self);
                if c.modifies {
                    self.record_changes(&label, None);
                }
            }
            Err(why) => self.status = why.to_string(),
        }
    }

    /// 收集命令之外的实际修改；空操作保留重做，文本输入和拖动合并成一步。
    fn record_changes(&mut self, label: &str, group: Option<egui::Id>) {
        let merge = group.is_some() && self.last_history_group == group;
        if self.history.record(&self.project, label, merge) {
            self.last_history_group = group;
        } else if self.last_history_group != group {
            self.last_history_group = None;
        }
    }

    pub(crate) fn undo(&mut self) {
        self.record_changes("编辑项目", self.history_group);
        let label = self.history.undo_label().unwrap_or("").to_string();
        if let Some(project) = self.history.undo(&self.project) {
            self.restore_history(project);
            self.status = format!("已撤销：{label}");
        }
    }

    pub(crate) fn redo(&mut self) {
        self.record_changes("编辑项目", self.history_group);
        let label = self.history.redo_label().unwrap_or("").to_string();
        if let Some(project) = self.history.redo(&self.project) {
            self.restore_history(project);
            self.status = format!("已重做：{label}");
        }
    }

    fn restore_history(&mut self, project: Project) {
        self.project = project;
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
            Some((_, n)) => format!("{} · 图形", n),
            None => "未选中窗口".to_string(),
        }
    }

    /// 分析的输入：当前图形里的数据曲线，或当前工作簿里选中的列。
    pub(crate) fn analysis_source(&self) -> Option<Source> {
        if let Some(g) = self.active_graph_id() {
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

    /// 排序依据的列：选中的第一列，或当前单元格所在列。
    pub(crate) fn sort_column(&self, book_id: u64) -> Option<usize> {
        let book = self.project.book(book_id)?;
        if let Some(k) = self.selected_keys(book_id).first() {
            return book.column_index(k);
        }
        self.sheets
            .get(&book_id)
            .and_then(|s| s.range)
            .map(|r| r.a.c)
            .filter(|c| *c < book.columns.len())
    }

    pub(crate) fn reset_project(&mut self) {
        self.load_project(Project::sample(), None);
        self.status = "已恢复示例项目".into();
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
        let layer = graph.page.layers()[index].clone();
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

    pub(crate) fn sort_active(&mut self, ascending: bool) {
        let Some(bid) = self.active_book_id() else { return };
        let Some(col) = self.sort_column(bid) else { return };
        match self.project.sort_book(bid, col, ascending) {
            Ok(()) => {
                let key = self
                    .project
                    .book(bid)
                    .map(|b| b.columns[col].key.clone())
                    .unwrap_or_default();
                self.status = format!(
                    "已按 {} 列{}排序整张工作表，各行数据一起移动",
                    key,
                    if ascending { "升序" } else { "降序" }
                );
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn show_all(&mut self) {
        for id in self.all_ids() {
            self.open.insert(id, true);
        }
    }

    /// 层叠（false）或平铺（true）所有打开的窗口。
    pub(crate) fn arrange_windows(&mut self, tile: bool) {
        let ids: Vec<u64> = self
            .all_ids()
            .into_iter()
            .filter(|id| self.open.get(id).copied().unwrap_or(false))
            .collect();
        if ids.is_empty() {
            return;
        }
        let (w, h) = (self.ws.width(), self.ws.height());
        let cols = (ids.len() as f32).sqrt().ceil() as usize;
        let rows = ids.len().div_ceil(cols);
        for (i, id) in ids.iter().enumerate() {
            let rect = if tile {
                let (cw, ch) = (w / cols as f32, h / rows as f32);
                egui::Rect::from_min_size(
                    egui::pos2((i % cols) as f32 * cw, (i / cols) as f32 * ch),
                    egui::vec2(cw, ch),
                )
            } else {
                let off = 26.0 * i as f32;
                egui::Rect::from_min_size(
                    egui::pos2(12.0 + off, 10.0 + off),
                    egui::vec2((w * 0.62).clamp(360.0, 760.0), (h * 0.72).clamp(260.0, 560.0)),
                )
            };
            self.geoms.insert(
                *id,
                Geom {
                    rect,
                    state: WinState::Normal,
                },
            );
        }
        if let Some(last) = ids.last() {
            self.active = Some(*last);
            self.raise = Some(*last);
        }
    }

    pub(crate) fn minimize_all(&mut self) {
        for (id, open) in &self.open {
            if *open {
                if let Some(g) = self.geoms.get_mut(id) {
                    g.state = WinState::Min;
                }
            }
        }
    }

    pub(crate) fn hide_all(&mut self) {
        for v in self.open.values_mut() {
            *v = false;
        }
        self.active = None;
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

    pub(crate) fn open_poly_dialog(&mut self) {
        self.dialog = Some(Dialog::Poly { order: 2 });
    }

    pub(crate) fn open_smooth_dialog(&mut self) {
        self.dialog = Some(Dialog::Smooth {
            method: SmoothMethod::SavitzkyGolay,
            window: 7,
        });
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
        match dialogs::show(ctx, d, &self.pal) {
            dialogs::Action::None => {}
            dialogs::Action::Close => self.dialog = None,
            dialogs::Action::Fit(order) => {
                self.record_changes("编辑项目", self.history_group);
                self.dialog = None;
                self.fit(order);
                self.record_changes("多项式拟合", None);
            }
            dialogs::Action::Smooth(m, w) => {
                self.record_changes("编辑项目", self.history_group);
                self.dialog = None;
                self.smooth(m, w);
                self.record_changes("平滑", None);
            }
            dialogs::Action::Normalize(m) => {
                self.record_changes("编辑项目", self.history_group);
                self.dialog = None;
                self.normalize(m);
                self.record_changes("归一化", None);
            }
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
        let keys: [(Modifiers, Key, &str); 8] = [
            (Modifiers::COMMAND, Key::N, "book.new"),
            (Modifiers::COMMAND, Key::O, "file.open"),
            (Modifiers::COMMAND, Key::S, "file.save"),
            (Modifiers::COMMAND, Key::I, "file.import"),
            (Modifiers::COMMAND, Key::E, "graph.export"),
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
                .flat_map(|c| {
                    (r.r0()..=r.r1()).map(move |i| c.data.get(i).map_or(f64::NAN, |s| scicanvas_core::parse_num(s)))
                })
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

    fn explorer(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(RichText::new("项目管理器").strong());
        ui.separator();
        let items: Vec<(u64, bool, String)> = self
            .project
            .books
            .iter()
            .map(|b| (b.id, true, b.name.clone()))
            .chain(self.project.graphs.iter().map(|g| (g.id, false, g.name.clone())))
            .collect();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::CollapsingHeader::new("文件夹1")
                .default_open(true)
                .show(ui, |ui| {
                    for (id, is_book, name) in items {
                        let shown = self.open.get(&id).copied().unwrap_or(false);
                        let active = self.active == Some(id);
                        let resp = ui
                            .horizontal(|ui| {
                                let (tag, c) = if is_book {
                                    ("表", self.pal.book)
                                } else {
                                    ("图", self.pal.graph)
                                };
                                ui.label(RichText::new(tag).small().strong().color(c));
                                let text = if shown {
                                    RichText::new(name)
                                } else {
                                    RichText::new(name).weak()
                                };
                                ui.selectable_label(active, text)
                            })
                            .inner;
                        let resp = resp.on_hover_text(if shown {
                            "单击激活，右键重命名或删除"
                        } else {
                            "窗口已隐藏，单击重新打开"
                        });
                        if resp.clicked() {
                            self.show_item(id);
                        }
                        resp.context_menu(|ui| {
                            if ui.button("打开").clicked() {
                                self.show_item(id);
                                ui.close_menu();
                            }
                            ui.horizontal(|ui| {
                                ui.label("名称");
                                if let Some(n) = self.project.name_of_mut(id) {
                                    let response = ui.text_edit_singleline(n);
                                    edit_group(&response, &mut self.history_group);
                                }
                            });
                            ui.separator();
                            if ui.button("删除…").clicked() {
                                self.confirm_delete = Some(id);
                                ui.close_menu();
                            }
                        });
                    }
                });
        });
    }

    fn child_windows(&mut self, ctx: &egui::Context) {
        let ws = ctx.available_rect();
        self.ws = ws;
        let ids = self.all_ids();

        // 鼠标按下时，按下位置所在的窗口成为当前窗口
        let press = ctx.input(|i| {
            if i.pointer.any_pressed() {
                i.pointer.interact_pos()
            } else {
                None
            }
        });
        if let Some(pos) = press {
            if let Some(layer) = ctx.layer_id_at(pos) {
                if let Some(id) = ids.iter().copied().find(|id| win_id(*id) == layer.id) {
                    self.active = Some(id);
                }
            }
        }

        let mut min_slot = 0;
        for id in ids {
            if !self.open.get(&id).copied().unwrap_or(false) {
                continue;
            }
            let is_book = self.project.book(id).is_some();
            let name = self.project.name_of(id).unwrap_or("").to_string();
            if self.raise == Some(id) {
                ctx.move_to_top(mdi::layer_id(win_id(id)));
            }
            let slot = (id.saturating_sub(1) % 6) as f32;
            let mut geom = *self.geoms.entry(id).or_insert_with(|| {
                if is_book {
                    Geom::new(
                        egui::pos2(12.0 + 26.0 * slot, 10.0 + 24.0 * slot),
                        egui::vec2(420.0, 440.0),
                    )
                } else {
                    Geom::new(
                        egui::pos2(450.0 + 26.0 * slot, 10.0 + 24.0 * slot),
                        egui::vec2(560.0, 440.0),
                    )
                }
            });
            let this_slot = min_slot;
            if geom.state == WinState::Min {
                min_slot += 1;
            }
            let active = self.active == Some(id);
            let pal = self.pal;
            let resp = mdi::window(
                ctx,
                win_id(id),
                ws,
                &mut geom,
                this_slot,
                &name,
                if is_book { "book" } else { "graph" },
                active,
                &pal,
                |ui| {
                    if is_book {
                        self.book_ui(ui, id);
                    } else {
                        self.graph_ui(ui, id);
                    }
                },
            );
            self.geoms.insert(id, geom);
            if resp.pressed {
                self.active = Some(id);
            }
            if resp.close {
                self.open.insert(id, false);
                if self.active == Some(id) {
                    self.active = None;
                }
                self.status = format!("{} 已隐藏，可在项目管理器中重新打开", name);
            }
        }
        self.raise = None;
    }

    /// 工作表：列标题、长名称 / 单位 / 注释三行和数据区。
    /// 数据区像 Excel 一样：单击选格、拖动选区域、拖动行号选整行、拖动列标题选多列，
    /// 双击或 F2 或直接打字进入编辑。
    fn book_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        let pal = self.pal;
        let active = self.active == Some(id);
        let (modifiers, pressed, down, dbl) = ui.input(|i| {
            (
                i.modifiers,
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.button_double_clicked(egui::PointerButton::Primary),
            )
        });
        let ptr = ui.input(|i| i.pointer.interact_pos());
        let focus_id = egui::Id::new(("sheet-focus", id));
        let edit_id = egui::Id::new(("sheet-edit", id));
        let selected = self.selected_keys(id);
        let mut st = self.sheets.remove(&id).unwrap_or_default();

        // 表格本身是一个可获得键盘焦点的部件：方向键、Tab、Esc 留给表格，不让 egui 拿去切换焦点
        ui.interact(ui.max_rect(), focus_id, egui::Sense::focusable_noninteractive());
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                focus_id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            )
        });
        if active
            && st.edit.is_none()
            && self.dialog.is_none()
            && self.ui.palette.is_none()
            && ui.memory(|m| m.has_focus(focus_id))
        {
            self.sheet_keys(ui.ctx(), id, &mut st);
        }

        #[derive(Clone, Copy)]
        enum Target {
            Cell(usize, usize),
            Row(usize),
            Col(usize),
        }

        let Some(book) = self.project.book_mut(id) else {
            self.sheets.insert(id, st);
            return;
        };
        let keys: Vec<String> = book.columns.iter().map(|c| c.key.clone()).collect();
        let sel_flags: Vec<bool> = keys.iter().map(|k| selected.contains(k)).collect();
        let ncols = book.columns.len();
        if ncols == 0 {
            st.range = None;
            st.edit = None;
        }
        if let Some(r) = st.range.as_mut() {
            r.clamp(ncols);
        }
        let range = st.range;
        let multi = range.filter(|r| r.is_multi());
        let nrows = (book.nrows() + 20)
            .max(40)
            .max(range.map_or(0, |r| r.r1().max(r.a.r) + 10))
            .max(st.edit.as_ref().map_or(0, |e| e.cell.r + 10));
        let drag = st.drag;
        let scroll_to = st.scroll_to.take();
        let mut edit = st.edit.take();
        let text_color = ui.visuals().text_color();
        let sel_stroke = egui::Stroke::new(2.0_f32, pal.accent);

        let mut clicked: Option<usize> = None;
        let mut action: Option<(usize, ColAction)> = None;
        let mut press: Option<Target> = None;
        let mut hover: Option<Target> = None;
        let mut dbl_cell: Option<(usize, usize)> = None;
        let mut right_cell: Option<(usize, usize)> = None;
        let mut menu_cmd: Option<&'static str> = None;
        let mut edit_end: Option<(bool, i64, i64)> = None;
        let mut autoscrolled = false;
        let dragging_cells = matches!(drag, Some(Drag::Cells | Drag::Rows)) && down;
        // 可见的表格区域（拖到上下边缘时自动滚动）
        let view = ui.clip_rect();
        let grid = egui::Stroke::new(1.0_f32, pal.meta_bg);

        ui.push_id(("sheet", id), |ui| {
            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .drag_to_scroll(false)
                .show(ui, |ui| {
                    // 单元格之间不留空隙，选区和网格线才能连成一片
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    let mut table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .auto_shrink([false, false])
                        .min_scrolled_height(0.0)
                        .drag_to_scroll(false)
                        .column(TCol::exact(58.0))
                        .columns(TCol::initial(96.0).at_least(40.0).clip(true), ncols);
                    if let Some(r) = scroll_to {
                        table = table.scroll_to_row(r + META_ROWS.len(), None);
                    }
                    table
                        .header(24.0, |mut header| {
                            header.col(|ui| {
                                ui.label("");
                            });
                            for (ci, col) in book.columns.iter().enumerate() {
                                header.col(|ui| {
                                    let rect = ui.max_rect();
                                    let text = RichText::new(format!("{}({})", col.key, col.role.label())).strong();
                                    let resp = ui
                                        .centered_and_justified(|ui| {
                                            ui.add(egui::SelectableLabel::new(sel_flags[ci], text))
                                        })
                                        .inner
                                        .on_hover_text("单击选中列，Ctrl / Shift 多选，拖动选多列，右键更多操作");
                                    if multi.is_some_and(|r| ci >= r.c0() && ci <= r.c1()) {
                                        ui.painter().line_segment(
                                            [
                                                rect.left_bottom() + egui::vec2(0.0, -1.0),
                                                rect.right_bottom() + egui::vec2(0.0, -1.0),
                                            ],
                                            sel_stroke,
                                        );
                                    }
                                    if pressed && ui.rect_contains_pointer(rect) {
                                        press = Some(Target::Col(ci));
                                    }
                                    if matches!(drag, Some(Drag::Cols(_))) && down && ui.rect_contains_pointer(rect) {
                                        hover = Some(Target::Col(ci));
                                    }
                                    if resp.clicked() {
                                        clicked = Some(ci);
                                    }
                                    resp.context_menu(|ui| {
                                        let mut pick = |ui: &mut egui::Ui, label: &str, a: ColAction| {
                                            if ui.button(label).clicked() {
                                                action = Some((ci, a));
                                                ui.close_menu();
                                            }
                                        };
                                        pick(ui, "设为 X", ColAction::SetRole(Role::X));
                                        pick(ui, "设为 Y", ColAction::SetRole(Role::Y));
                                        pick(ui, "设为忽略", ColAction::SetRole(Role::Ignore));
                                        ui.separator();
                                        for t in PlotType::ALL {
                                            pick(ui, t.label(), ColAction::Plot(t));
                                        }
                                        ui.separator();
                                        pick(ui, "在右侧插入列", ColAction::InsertRight);
                                        pick(ui, "清空", ColAction::Clear);
                                        pick(ui, "删除列", ColAction::Delete);
                                    });
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(22.0, META_ROWS.len() + nrows, |mut row| {
                                let ri = row.index();
                                if ri < META_ROWS.len() {
                                    row.col(|ui| {
                                        ui.painter().rect_filled(ui.max_rect(), 0.0, pal.meta_bg);
                                        ui.horizontal_centered(|ui| {
                                            ui.add_space(6.0);
                                            ui.label(META_ROWS[ri]);
                                        });
                                    });
                                    for (ci, col) in book.columns.iter_mut().enumerate() {
                                        row.col(|ui| {
                                            let bg = if sel_flags[ci] { pal.meta_sel_bg } else { pal.meta_bg };
                                            let rect = ui.max_rect();
                                            ui.painter().rect_filled(rect, 0.0, bg);
                                            ui.painter().line_segment(
                                                [rect.right_top(), rect.right_bottom()],
                                                egui::Stroke::new(1.0_f32, pal.window),
                                            );
                                            let field = match ri {
                                                0 => &mut col.long_name,
                                                1 => &mut col.units,
                                                _ => &mut col.comment,
                                            };
                                            let response = ui.add(
                                                egui::TextEdit::singleline(field)
                                                    .frame(false)
                                                    .margin(egui::vec2(5.0, 3.0))
                                                    .desired_width(f32::INFINITY),
                                            );
                                            edit_group(&response, &mut self.history_group);
                                        });
                                    }
                                    return;
                                }

                                let r = ri - META_ROWS.len();
                                // 行号：拖动选整行
                                row.col(|ui| {
                                    let rect = ui.max_rect();
                                    let in_rows = multi.is_some_and(|g| r >= g.r0() && r <= g.r1());
                                    if in_rows {
                                        ui.painter().rect_filled(rect, 0.0, pal.sel_bg);
                                    }
                                    let resp = ui.interact(
                                        rect,
                                        egui::Id::new(("row-head", id, r)),
                                        egui::Sense::click_and_drag(),
                                    );
                                    if resp.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
                                    }
                                    ui.painter().with_clip_rect(rect).text(
                                        rect.left_center() + egui::vec2(6.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        (r + 1).to_string(),
                                        egui::FontId::monospace(12.0),
                                        pal.weak,
                                    );
                                    if pressed && ui.rect_contains_pointer(rect) {
                                        press = Some(Target::Row(r));
                                    }
                                    if dragging_cells && ui.rect_contains_pointer(rect) {
                                        hover = Some(Target::Row(r));
                                    }
                                    // 拖到表格上下边缘时自动滚动
                                    if dragging_cells && !autoscrolled {
                                        autoscrolled = true;
                                        if let Some(p) = ptr {
                                            if p.y > view.bottom() - 12.0 {
                                                ui.scroll_with_delta(egui::vec2(0.0, -16.0));
                                            } else if p.y < view.top() + 24.0 + 6.0 {
                                                ui.scroll_with_delta(egui::vec2(0.0, 16.0));
                                            }
                                        }
                                    }
                                });

                                for (ci, col) in book.columns.iter_mut().enumerate() {
                                    row.col(|ui| {
                                        let rect = ui.max_rect();
                                        let here = Cell { c: ci, r };
                                        let painter =
                                            ui.painter().with_clip_rect(rect.expand(1.0).intersect(ui.clip_rect()));

                                        if let Some(e) = edit.as_mut().filter(|e| e.cell == here) {
                                            painter.rect_filled(rect, 0.0, pal.window);
                                            let resp = ui.add(
                                                egui::TextEdit::singleline(&mut e.buf)
                                                    .id(edit_id)
                                                    .frame(false)
                                                    .margin(egui::vec2(5.0, 3.0))
                                                    .desired_width(f32::INFINITY)
                                                    .font(egui::TextStyle::Monospace),
                                            );
                                            if !e.focused {
                                                resp.request_focus();
                                                e.focused = true;
                                            }
                                            // Tab、上下方向键由表格处理（结束编辑并移动），不让 egui 切换焦点
                                            ui.memory_mut(|m| {
                                                m.set_focus_lock_filter(
                                                    edit_id,
                                                    egui::EventFilter {
                                                        tab: true,
                                                        horizontal_arrows: true,
                                                        vertical_arrows: true,
                                                        escape: false,
                                                    },
                                                )
                                            });
                                            let (enter, tab, up, dn, esc, shift) = ui.input(|i| {
                                                (
                                                    i.key_pressed(egui::Key::Enter),
                                                    i.key_pressed(egui::Key::Tab),
                                                    i.key_pressed(egui::Key::ArrowUp),
                                                    i.key_pressed(egui::Key::ArrowDown),
                                                    i.key_pressed(egui::Key::Escape),
                                                    i.modifiers.shift,
                                                )
                                            });
                                            if esc {
                                                edit_end = Some((false, 0, 0));
                                            } else if tab {
                                                edit_end = Some((true, if shift { -1 } else { 1 }, 0));
                                            } else if up {
                                                edit_end = Some((true, 0, -1));
                                            } else if dn {
                                                edit_end = Some((true, 0, 1));
                                            } else if resp.lost_focus() {
                                                edit_end = Some((
                                                    true,
                                                    0,
                                                    if enter {
                                                        if shift {
                                                            -1
                                                        } else {
                                                            1
                                                        }
                                                    } else {
                                                        0
                                                    },
                                                ));
                                            }
                                            painter.rect_stroke(rect.shrink(1.0), 0.0, sel_stroke);
                                            return;
                                        }

                                        let in_range = multi.is_some_and(|g| g.contains(ci, r));
                                        if sel_flags[ci] || in_range {
                                            painter.rect_filled(rect, 0.0, pal.sel_bg);
                                        }
                                        painter.line_segment([rect.right_top(), rect.right_bottom()], grid);
                                        painter.line_segment([rect.left_bottom(), rect.right_bottom()], grid);
                                        if let Some(s) = col.data.get(r).filter(|s| !s.is_empty()) {
                                            ui.painter().with_clip_rect(rect.intersect(ui.clip_rect())).text(
                                                rect.right_center() - egui::vec2(5.0, 0.0),
                                                egui::Align2::RIGHT_CENTER,
                                                s,
                                                egui::FontId::monospace(12.5),
                                                text_color,
                                            );
                                        }
                                        let resp = ui.interact(
                                            rect,
                                            egui::Id::new(("cell", id, ci, r)),
                                            egui::Sense::click_and_drag(),
                                        );
                                        if pressed && ui.rect_contains_pointer(rect) {
                                            press = Some(Target::Cell(ci, r));
                                        }
                                        if dbl && ui.rect_contains_pointer(rect) {
                                            dbl_cell = Some((ci, r));
                                        }
                                        if dragging_cells && ui.rect_contains_pointer(rect) {
                                            hover = Some(Target::Cell(ci, r));
                                        }
                                        if resp.secondary_clicked() {
                                            right_cell = Some((ci, r));
                                        }
                                        resp.context_menu(|ui| {
                                            for (cid, label) in [
                                                ("edit.copy", "复制"),
                                                ("edit.clear", "清除内容"),
                                                ("", ""),
                                                ("plot.line", "折线图"),
                                                ("plot.scatter", "散点图"),
                                                ("plot.linesym", "点线图"),
                                                ("plot.bar", "柱状图"),
                                            ] {
                                                if cid.is_empty() {
                                                    ui.separator();
                                                } else if ui.button(label).clicked() {
                                                    menu_cmd = Some(cid);
                                                    ui.close_menu();
                                                }
                                            }
                                        });

                                        // 选区外框和活动单元格
                                        if let Some(g) = range {
                                            if g.is_multi() && g.contains(ci, r) {
                                                let (lt, rt, lb, rb) = (
                                                    rect.left_top(),
                                                    rect.right_top(),
                                                    rect.left_bottom(),
                                                    rect.right_bottom(),
                                                );
                                                if ci == g.c0() {
                                                    painter.line_segment([lt, lb], sel_stroke);
                                                }
                                                if ci == g.c1() {
                                                    painter.line_segment([rt, rb], sel_stroke);
                                                }
                                                if r == g.r0() {
                                                    painter.line_segment([lt, rt], sel_stroke);
                                                }
                                                if r == g.r1() {
                                                    painter.line_segment([lb, rb], sel_stroke);
                                                }
                                            }
                                            if g.a == here && active {
                                                painter.rect_stroke(rect.shrink(1.0), 0.0, sel_stroke);
                                            }
                                        }
                                    });
                                }
                            });
                        });
                });
        });

        // ---- 处理这一帧的鼠标和编辑结果
        let last_col = ncols.saturating_sub(1);
        let mut status: Option<String> = None;
        let mut clear_cols = false;
        let mut new_cols: Option<(usize, usize)> = None;
        let mut want_focus = false;
        let commit = |book: &mut scicanvas_core::Book, e: &Edit| {
            if let Some(col) = book.columns.get_mut(e.cell.c) {
                col.set(e.cell.r, e.buf.clone());
            }
        };

        if let Some((ok, dc, dr)) = edit_end {
            if let Some(e) = edit.take() {
                if ok {
                    commit(book, &e);
                }
                let c = (e.cell.c as i64 + dc).clamp(0, last_col as i64) as usize;
                let r = (e.cell.r as i64 + dr).max(0) as usize;
                st.range = Some(Range::single(c, r));
                st.scroll_to = Some(r);
                want_focus = true;
            }
        }

        if let Some(t) = press {
            // 正在编辑别的格子：先保存
            let same = matches!((t, &edit), (Target::Cell(c, r), Some(e)) if e.cell == Cell { c, r });
            if !same {
                if let Some(e) = edit.take() {
                    commit(book, &e);
                }
            }
            match t {
                Target::Cell(c, r) if !same => {
                    clear_cols = true;
                    match st.range.as_mut() {
                        Some(g) if modifiers.shift => g.b = Cell { c, r },
                        _ => st.range = Some(Range::single(c, r)),
                    }
                    st.drag = Some(Drag::Cells);
                    st.moved = false;
                    want_focus = true;
                }
                Target::Row(r) if ncols > 0 => {
                    clear_cols = true;
                    let a = match st.range {
                        Some(g) if modifiers.shift => Cell { c: 0, r: g.a.r },
                        _ => Cell { c: 0, r },
                    };
                    st.range = Some(Range {
                        a,
                        b: Cell { c: last_col, r },
                    });
                    st.drag = Some(Drag::Rows);
                    st.moved = true;
                    want_focus = true;
                }
                Target::Col(c) => {
                    st.drag = Some(Drag::Cols(c));
                    st.moved = false;
                }
                _ => {}
            }
        }

        if down {
            match (st.drag, hover) {
                (Some(Drag::Cells), Some(Target::Cell(c, r))) => {
                    if let Some(g) = st.range.as_mut() {
                        if g.b != (Cell { c, r }) {
                            g.b = Cell { c, r };
                            st.moved = true;
                        }
                    }
                }
                (Some(Drag::Cells), Some(Target::Row(r))) => {
                    if let Some(g) = st.range.as_mut() {
                        g.b.r = r;
                        st.moved = true;
                    }
                }
                (Some(Drag::Rows), Some(Target::Cell(_, r) | Target::Row(r))) => {
                    if let Some(g) = st.range.as_mut() {
                        g.b = Cell { c: last_col, r };
                    }
                }
                (Some(Drag::Cols(start)), Some(Target::Col(c))) if c != start || st.moved => {
                    st.moved = true;
                    new_cols = Some((start.min(c), start.max(c)));
                }
                _ => {}
            }
        } else if let Some(d) = st.drag.take() {
            if st.moved && !matches!(d, Drag::Cols(_)) {
                if let Some(g) = st.range.filter(|g| g.is_multi()) {
                    status = Some(range_status(book, &g));
                }
            }
        }

        if let Some((c, r)) = dbl_cell {
            let buf = book
                .columns
                .get(c)
                .and_then(|col| col.data.get(r))
                .cloned()
                .unwrap_or_default();
            edit = Some(Edit {
                cell: Cell { c, r },
                buf,
                focused: false,
            });
            st.range = Some(Range::single(c, r));
            st.drag = None;
        }
        if let Some((c, r)) = right_cell {
            if !st.range.is_some_and(|g| g.contains(c, r)) {
                st.range = Some(Range::single(c, r));
                clear_cols = true;
            }
        }
        st.edit = edit;
        self.sheets.insert(id, st);

        if want_focus || dbl_cell.is_some() || right_cell.is_some() || press.is_some() {
            self.active = Some(id);
        }
        if want_focus {
            ui.memory_mut(|m| m.request_focus(focus_id));
        }
        if clear_cols {
            self.selection.remove(&id);
        }
        if let Some((a, b)) = new_cols {
            self.selection.insert(id, keys[a..=b].to_vec());
            if let Some(s) = self.sheets.get_mut(&id) {
                s.range = None;
            }
            self.status = format!("已选中 {} 列", keys[a..=b].join(", "));
        }
        if let Some(s) = status {
            self.status = s;
        }

        // 单击列标题：选择列
        if let Some(ci) = clicked.filter(|_| new_cols.is_none()) {
            let key = keys[ci].clone();
            let sel = self.selection.entry(id).or_default();
            if modifiers.command {
                if let Some(p) = sel.iter().position(|k| *k == key) {
                    sel.remove(p);
                } else {
                    sel.push(key);
                }
            } else if modifiers.shift && !sel.is_empty() {
                let last = sel.last().cloned().unwrap_or_default();
                let a = keys.iter().position(|k| *k == last).unwrap_or(ci);
                for k in &keys[a.min(ci)..=a.max(ci)] {
                    if !sel.contains(k) {
                        sel.push(k.clone());
                    }
                }
            } else {
                sel.clear();
                sel.push(key);
            }
            self.active = Some(id);
            self.status = format!("已选中 {} 列", sel.join(", "));
            if let Some(s) = self.sheets.get_mut(&id) {
                s.range = None;
            }
        }

        // 列标题右键菜单
        if let Some((ci, act)) = action {
            let key = keys[ci].clone();
            let sel = self.selection.entry(id).or_default();
            if !sel.contains(&key) {
                sel.clear();
                sel.push(key);
            }
            self.active = Some(id);
            let command = match act {
                ColAction::SetRole(Role::X) => "col.setX",
                ColAction::SetRole(Role::Y) => "col.setY",
                ColAction::SetRole(Role::Ignore) => "col.setIgnore",
                ColAction::InsertRight => "col.insert",
                ColAction::Clear => "col.clear",
                ColAction::Delete => "col.delete",
                ColAction::Plot(PlotType::Line) => "plot.line",
                ColAction::Plot(PlotType::Scatter) => "plot.scatter",
                ColAction::Plot(PlotType::LineSymbol) => "plot.linesym",
                ColAction::Plot(PlotType::Bar) => "plot.bar",
            };
            self.run_command(command);
        }

        // 单元格右键菜单
        if let Some(cmd) = menu_cmd {
            self.active = Some(id);
            self.run_command(cmd);
        }
    }

    /// 表格有键盘焦点时的按键：方向键移动（Shift 扩展选区）、Tab / 回车移动、
    /// Delete 清除、Ctrl+C / Ctrl+X / Ctrl+V、Ctrl+A 全选、Esc 取消选区、F2 或直接打字开始编辑。
    fn sheet_keys(&mut self, ctx: &egui::Context, id: u64, st: &mut SheetState) {
        use egui::{Event, Key};
        let Some(book) = self.project.book_mut(id) else { return };
        if book.columns.is_empty() {
            return;
        }
        let Some(mut g) = st.range else { return };
        let events: Vec<Event> = ctx.input_mut(|i| {
            let mut mine = Vec::new();
            i.events.retain(|e| {
                let take = match e {
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        matches!(
                            key,
                            Key::ArrowUp
                                | Key::ArrowDown
                                | Key::ArrowLeft
                                | Key::ArrowRight
                                | Key::Enter
                                | Key::Tab
                                | Key::F2
                                | Key::Delete
                                | Key::Backspace
                                | Key::Escape
                                | Key::Home
                                | Key::End
                                | Key::PageUp
                                | Key::PageDown
                        ) || (modifiers.command && *key == Key::A)
                    }
                    Event::Copy | Event::Cut | Event::Paste(_) => true,
                    Event::Text(t) => !t.chars().all(char::is_control),
                    _ => false,
                };
                if take {
                    mine.push(e.clone());
                }
                !take
            });
            mine
        });
        if events.is_empty() {
            return;
        }

        let mut status: Option<String> = None;
        let mut start_edit: Option<String> = None;
        let mut copy: Option<String> = None;
        for e in events {
            let last_col = book.columns.len() - 1;
            match e {
                Event::Key { key, modifiers, .. } => {
                    let (dc, dr): (i64, i64) = match key {
                        Key::ArrowUp => (0, -1),
                        Key::ArrowDown => (0, 1),
                        Key::ArrowLeft => (-1, 0),
                        Key::ArrowRight => (1, 0),
                        Key::PageUp => (0, -20),
                        Key::PageDown => (0, 20),
                        Key::Enter => (0, if modifiers.shift { -1 } else { 1 }),
                        Key::Tab => (if modifiers.shift { -1 } else { 1 }, 0),
                        _ => (0, 0),
                    };
                    let step = |cell: Cell| Cell {
                        c: (cell.c as i64 + dc).clamp(0, last_col as i64) as usize,
                        r: (cell.r as i64 + dr).max(0) as usize,
                    };
                    match key {
                        Key::A => {
                            g = Range {
                                a: Cell { c: 0, r: 0 },
                                b: Cell {
                                    c: last_col,
                                    r: book.nrows().saturating_sub(1),
                                },
                            };
                            status = Some(range_status(book, &g));
                        }
                        Key::F2 => {
                            start_edit = Some(book.columns[g.a.c].data.get(g.a.r).cloned().unwrap_or_default());
                        }
                        Key::Delete | Key::Backspace => {
                            book.clear_range(g.c0(), g.c1(), g.r0(), g.r1());
                            status = Some(format!("已清除 {} 行 × {} 列", g.rows(), g.cols()));
                        }
                        Key::Escape => g = Range { a: g.a, b: g.a },
                        Key::Home => {
                            let r = if modifiers.command { 0 } else { g.a.r };
                            g = Range::single(0, r);
                            st.scroll_to = Some(r);
                        }
                        Key::End => {
                            let r = if modifiers.command {
                                book.nrows().saturating_sub(1)
                            } else {
                                g.a.r
                            };
                            g = Range::single(last_col, r);
                            st.scroll_to = Some(r);
                        }
                        _ if dc != 0 || dr != 0 => {
                            let extend = modifiers.shift && !matches!(key, Key::Enter | Key::Tab);
                            if extend {
                                g.b = step(g.b);
                                st.scroll_to = Some(g.b.r);
                            } else {
                                let n = step(g.a);
                                g = Range::single(n.c, n.r);
                                st.scroll_to = Some(n.r);
                            }
                        }
                        _ => {}
                    }
                }
                Event::Copy | Event::Cut => {
                    copy = Some(book.range_tsv(g.c0(), g.c1(), g.r0(), g.r1()));
                    if matches!(e, Event::Cut) {
                        book.clear_range(g.c0(), g.c1(), g.r0(), g.r1());
                        status = Some(format!("已剪切 {} 行 × {} 列", g.rows(), g.cols()));
                    } else {
                        status = Some(format!("已复制 {} 行 × {} 列，可直接粘贴到 Excel", g.rows(), g.cols()));
                    }
                }
                Event::Paste(text) => {
                    if csv::is_block_paste(&text) {
                        let (c0, r0) = (g.c0(), g.r0());
                        let (nr, nc) = csv::paste_into(book, c0, r0, &text);
                        g = Range {
                            a: Cell { c: c0, r: r0 },
                            b: Cell {
                                c: c0 + nc.max(1) - 1,
                                r: r0 + nr.max(1) - 1,
                            },
                        };
                        status = Some(format!("已粘贴 {} 行 × {} 列", nr, nc));
                    } else {
                        let v = text.trim();
                        book.fill_range(g.c0(), g.c1(), g.r0(), g.r1(), v);
                        status = Some(if g.is_multi() {
                            format!("已把 {} 填入 {} 个单元格", v, g.rows() * g.cols())
                        } else {
                            "已粘贴".to_string()
                        });
                    }
                }
                Event::Text(t) => {
                    start_edit.get_or_insert_with(String::new).push_str(&t);
                }
                _ => {}
            }
        }
        g.clamp(book.columns.len());
        st.range = Some(g);
        if let Some(buf) = start_edit {
            st.edit = Some(Edit {
                cell: g.a,
                buf,
                focused: false,
            });
            st.range = Some(Range::single(g.a.c, g.a.r));
        }
        if let Some(t) = copy {
            ctx.copy_text(t);
        }
        if let Some(s) = status {
            self.status = s;
        }
    }

    fn graph_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(g) = self.project.graph(id) else {
            return;
        };
        let out = plot::show(ui, g, &self.project, self.layer_index(id));
        if let Some(index) = out.selected_layer {
            self.select_layer(id, index);
        }
        let rect = out.response.rect;

        // 右下角显示鼠标处的坐标
        if let Some((x, y)) = out.hover {
            let text = format!(
                "图层 {} · x = {}   y = {}",
                out.hover_layer.unwrap_or(0) + 1,
                round_sig(x, 4),
                round_sig(y, 4)
            );
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
        resp.context_menu(|ui| {
            for (cid, label) in [
                ("graph.props", "绘图属性…"),
                ("graph.export", "导出 SVG…"),
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
        if let Some(cid) = act {
            self.active = Some(id);
            self.run_command(cid);
        }
    }

    fn props_window(&mut self, ctx: &egui::Context) {
        let Some(gid) = self.props_for else { return };
        let Some(g0) = self.project.graph(gid) else {
            self.props_for = None;
            return;
        };
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
        let mut props_layer = self.layer_index(gid);
        let title = format!("绘图属性 — {}", g0.name);
        let history_group = &mut self.history_group;
        let project = &mut self.project;

        let mut open = true;
        let mut close = false;
        egui::Window::new(title)
            .id(egui::Id::new("scicanvas-props"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                let Some(graph) = project.graph_mut(gid) else { return };
                ui.horizontal(|ui| {
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
                ui.horizontal(|ui| {
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
                });
                let g = &mut graph.page.layers_mut()[props_layer];
                let cols = g
                    .book_id
                    .and_then(|id| column_options.get(&id))
                    .cloned()
                    .unwrap_or_default();
                egui::Grid::new(("props-grid", gid, props_layer))
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("图类型");
                        egui::ComboBox::from_id_salt("props-type")
                            .selected_text(g.plot_type.label())
                            .show_ui(ui, |ui| {
                                for t in PlotType::ALL {
                                    ui.selectable_value(&mut g.plot_type, t, t.label());
                                }
                            });
                        ui.end_row();

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

                        ui.label("X 范围");
                        ui.horizontal(|ui| {
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
                        ui.horizontal(|ui| {
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
                        ui.label("显示");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut g.grid, "网格线");
                            ui.checkbox(&mut g.legend, "图例");
                        });
                        ui.end_row();
                    });
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
            .open(&mut self.show_help)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label("1. 功能区：顶部选项卡按任务分组。激活工作簿时出现紫色的“工作表”选项卡，激活图形时出现“图形”选项卡。双击选项卡或按 Ctrl+F1 折叠功能区；Ctrl+K 搜索任何命令。");
                ui.label("2. 输入数据：在工作簿里直接键入；或从 Excel 复制一块区域，单击单元格后 Ctrl+V，首行是文字时自动作为长名称。也可以 文件 → 导入数据。");
                ui.label("3. 选列与绘图：单击列标题，Ctrl 或 Shift 多选；再点“绘图”选项卡里的图类型。没有选列时使用全部 Y 列。");
                ui.label("4. 分析：“分析”选项卡里有线性 / 多项式拟合、平滑、微分、积分、归一化，“统计”选项卡里有列统计。对当前图形的曲线，或工作簿里选中的 Y 列计算，结果写进紧挨原列的新列。");
                ui.label("5. 导出与保存：文件 → 导出图形（SVG）；文件 → 保存项目（.scproj）。");
            });
    }

    fn confirm_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.confirm_delete else { return };
        let name = self.project.name_of(id).unwrap_or("").to_string();
        let mut decision: Option<bool> = None;
        egui::Window::new("删除")
            .id(egui::Id::new("scicanvas-confirm"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("确定删除 {}？可通过撤销恢复。", name));
                ui.horizontal(|ui| {
                    if ui.button("删除").clicked() {
                        decision = Some(true);
                    }
                    if ui.button("取消").clicked() {
                        decision = Some(false);
                    }
                });
            });
        match decision {
            Some(true) => {
                self.record_changes("编辑项目", self.history_group);
                self.confirm_delete = None;
                self.delete_item(id);
                self.record_changes(&format!("删除 {name}"), None);
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
            if self.show_explorer {
                egui::SidePanel::left("explorer")
                    .frame(
                        egui::Frame::side_top_panel(&style)
                            .fill(style.visuals.window_fill)
                            .inner_margin(egui::Margin::symmetric(10.0, 6.0)),
                    )
                    .resizable(true)
                    .default_width(190.0)
                    .show(ctx, |ui| self.explorer(ui));
            }
            // 子窗口放在面板之间剩下的区域里（Origin 的 MDI 工作区）
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
        self.record_changes("编辑项目", self.history_group);

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

    fn roundtrip(app: &mut SciApp, before: &Project) {
        let after = app.project.clone();
        assert_ne!(&after, before);
        app.run_command("edit.undo");
        assert_eq!(&app.project, before);
        app.run_command("edit.redo");
        assert_eq!(app.project, after);
    }

    #[test]
    fn commands_restore_columns_graphs_fits_and_sorting() {
        for command in ["col.delete", "sort.desc", "fit.linear", "plot.bar", "book.new"] {
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
        app.record_changes("删除工作簿", None);
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
            app.record_changes("编辑工作表", None);
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
            app.record_changes("编辑单元格", None);
        }
        assert_eq!(app.project.books[0].columns[0].data[0], "123");
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
        app.project.books[0].columns[0].long_name = "时".into();
        app.record_changes("修改长名称", group);
        app.project.books[0].columns[0].long_name = "时间轴".into();
        app.record_changes("修改长名称", group);
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
            app.history_group = None;
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = ui
                            .add(egui::Slider::new(&mut app.project.graphs[0].line_width, 0.5..=6.0).show_value(false));
                        rect = response.rect;
                        edit_group(&response, &mut app.history_group);
                    });
                },
            );
            app.record_changes("线宽", app.history_group);
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
        app.fit(2);
        app.record_changes("多项式拟合", None);
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
        app.history = History::new(&app.project);
        app.select_layer(2, 0);
        app.select_layer(2, 1);
        app.record_changes("选择图层", None);
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
}
