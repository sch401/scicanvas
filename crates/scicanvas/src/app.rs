//! 主界面：功能区、项目管理器、多文档子窗口（工作簿 / 图形）、状态栏，以及各个命令背后的操作。

use std::collections::HashMap;
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

/// 列标题右键菜单里的操作。
#[derive(Clone, Copy)]
enum ColAction {
    Props,
    SetRole(Role),
    SetDataKind(scicanvas_core::DataKind),
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
    pub(crate) explorer_target: Option<u64>,
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
    /// 功能区、文件页、命令面板的界面状态。
    pub(crate) ui: UiState,
    /// 当前打开的分析对话框。
    pub(crate) dialog: Option<Dialog>,
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
            project: Project::default(),
            history: History::default(),
            history_group: None,
            last_history_group: None,
            open: HashMap::new(),
            selection: HashMap::new(),
            active: None,
            explorer_target: None,
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
        self.active = project
            .graphs
            .last()
            .map(|g| g.id)
            .or(project.books.last().map(|b| b.id));
        self.project = project;
        self.explorer_target = self.active;
        self.folder_destination = None;
        self.project_name.clear();
        self.explorer_click = None;
        self.explorer_rename = None;
        self.project_path = path;
        self.props_for = None;
        self.sheets.clear();
    }

    fn show_item(&mut self, id: u64) {
        self.explorer_target = Some(id);
        self.project_name = self.project.name_of(id).unwrap_or("").to_string();
        self.open.insert(id, true);
        if let Some(g) = self.geoms.get_mut(&id) {
            if g.state == WinState::Min {
                g.state = WinState::Normal;
            }
        }
        self.active = Some(id);
        self.raise = Some(id);
    }

    pub(crate) fn all_ids(&self) -> Vec<u64> {
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

    /// 在选区上方插入与选中行数相同的空行。
    pub(crate) fn insert_rows(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        self.project.insert_rows(bid, r.r0(), r.rows());
        self.status = format!("已在第 {} 行上方插入 {} 行", r.r0() + 1, r.rows());
    }

    /// 删除选区覆盖的整行（所有列）。
    pub(crate) fn delete_rows(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        self.project.delete_rows(bid, r.r0(), r.r1());
        if let Some(st) = self.sheets.get_mut(&bid) {
            st.range = Some(Range::single(r.c0(), r.r0()));
        }
        self.status = if r.rows() == 1 {
            format!("已删除第 {} 行", r.r0() + 1)
        } else {
            format!("已删除第 {}–{} 行", r.r0() + 1, r.r1() + 1)
        };
    }

    pub(crate) fn new_book(&mut self) {
        let parent = self.current_folder();
        let id = self.project.new_book();
        self.project.move_project_item(id, parent).unwrap();
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

    pub(crate) fn set_role(&mut self, role: Role) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.picked(bid).0;
        if sel.is_empty() {
            self.status = "请先单击列标题选中列".into();
            return;
        }
        if let Some(b) = self.project.book_mut(bid) {
            b.set_roles(&sel, role);
        }
        let name = match role {
            Role::X => "X",
            Role::Y => "Y",
            Role::XError => "X 误差",
            Role::YError => "Y 误差",
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

    /// 打开第一个选中列的属性对话框。
    pub(crate) fn open_column_props(&mut self) {
        let Some(id) = self.active_book() else { return };
        let Some(key) = self.selected_keys(id).into_iter().next() else {
            self.status = "请先单击列标题选中列".into();
            return;
        };
        let Some(col) = self.project.book(id).and_then(|b| b.column(&key)) else {
            return;
        };
        let sample = (0..col.data.len()).find_map(|r| col.data.f64_at(r).filter(|v| v.is_finite()));
        self.dialog = Some(Dialog::ColumnProps {
            edit: dialogs::ColumnEdit {
                key,
                long_name: col.long_name.clone(),
                units: col.units.clone(),
                comment: col.comment.clone(),
                role: col.role,
                kind: col.data.kind(),
                format: col.format,
            },
            sample,
            error: None,
        });
    }

    /// 应用列属性；类型转换失败时不改动任何内容，错误留在对话框里。
    fn apply_column_props(&mut self, edit: dialogs::ColumnEdit) -> Result<(), String> {
        let id = self.active_book().ok_or("找不到工作簿")?;
        let book = self.project.book_mut(id).ok_or("找不到工作簿")?;
        let col = book.column(&edit.key).ok_or("找不到列")?;
        if col.data.kind() != edit.kind {
            book.convert_columns(std::slice::from_ref(&edit.key), edit.kind)?;
        }
        if book.column(&edit.key).is_some_and(|c| c.role != edit.role) {
            book.set_roles(std::slice::from_ref(&edit.key), edit.role);
        }
        let col = book.columns.iter_mut().find(|c| c.key == edit.key).unwrap();
        col.long_name = edit.long_name;
        col.units = edit.units;
        col.comment = edit.comment;
        col.format = edit.format;
        self.status = format!("已更新列 {} 的属性", edit.key);
        Ok(())
    }

    pub(crate) fn set_data_kind(&mut self, kind: scicanvas_core::DataKind) {
        let Some(id) = self.active_book() else { return };
        let keys = self.selected_keys(id);
        self.status = match self.project.book_mut(id).unwrap().convert_columns(&keys, kind) {
            Ok(()) => format!("已将 {} 设为{}；原始写法保留", keys.join(", "), kind.label()),
            Err(error) => error,
        };
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
        self.begin_edit("删除对象");
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
        self.history.changed();
    }

    pub(crate) fn import_csv(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("导入数据")
            .add_filter("数据文件", &["csv", "txt", "tsv", "dat", "xlsx", "xlsm"])
            .add_filter("Excel 工作簿", &["xlsx", "xlsm"])
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
        let tables: Vec<(String, Vec<Vec<String>>)> = if excel {
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
                            (name, s.rows)
                        })
                        .collect()
                }
                Err(e) => {
                    self.status = format!("无法导入 {}：{e}", path.display());
                    return;
                }
            }
        } else {
            vec![(stem, csv::parse_table(&csv::decode(&bytes)))]
        };
        let tables: Vec<_> = tables.into_iter().filter(|(_, rows)| !rows.is_empty()).collect();
        if tables.is_empty() {
            self.status = "文件里没有数据".into();
            return;
        }
        let mut done = vec![];
        let mut last = None;
        for (base, rows) in tables {
            let name = if self.project.books.iter().any(|b| b.name == base) {
                self.project.unique_name(&format!("{}_", base))
            } else {
                base
            };
            let id = self.project.alloc_id();
            let book = csv::book_from_table(id, &name, rows);
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

    pub(crate) fn project_target(&self) -> Option<u64> {
        self.explorer_target.filter(|id| self.project.name_of(*id).is_some())
    }
    fn current_folder(&self) -> Option<u64> {
        self.explorer_target.and_then(|id| {
            if self.project.folder(id).is_some() {
                Some(id)
            } else {
                self.project.folder_of(id)
            }
        })
    }
    pub(crate) fn new_folder(&mut self) {
        match self.project.add_folder(self.current_folder()) {
            Ok(id) => {
                self.explorer_target = Some(id);
                self.status = "已新建文件夹，可在右键菜单中重命名".into();
                self.project_name = self.project.name_of(id).unwrap_or("").to_string();
            }
            Err(e) => self.status = e,
        }
    }
    pub(crate) fn rename_project_target(&mut self) {
        if let Some(id) = self.project_target() {
            self.status = match self.project.rename_project_item(id, &self.project_name) {
                Ok(()) => "已重命名".into(),
                Err(e) => e,
            };
        }
    }
    pub(crate) fn move_project_target(&mut self) {
        if let Some(id) = self.project_target() {
            self.status = match self.project.move_project_item(id, self.folder_destination) {
                Ok(()) => "已移动项目对象".into(),
                Err(e) => e,
            };
        }
    }
    pub(crate) fn remove_project_folder(&mut self) {
        if let Some(id) = self.project_target() {
            let parent = self.project.folder(id).and_then(|f| f.parent);
            self.status = match self.project.remove_folder(id) {
                Ok(()) => {
                    self.explorer_target = parent;
                    self.project_name = parent.and_then(|id| self.project.name_of(id)).unwrap_or("").to_string();
                    "已删除文件夹，内容已移到上级".into()
                }
                Err(e) => e,
            };
        }
    }
    pub(crate) fn hide_project_window(&mut self) {
        if let Some(id) = self.project_target() {
            self.open.insert(id, false);
            if self.active == Some(id) {
                self.active = None;
            }
            if self.props_for == Some(id) {
                self.props_for = None;
            }
            self.status = "窗口已隐藏，可从项目管理器重新打开".into();
        }
    }
    pub(crate) fn show_project_window(&mut self) {
        if let Some(id) = self.project_target() {
            self.show_item(id);
        }
    }
    pub(crate) fn set_project_window_state(&mut self, state: WinState) {
        if let Some(id) = self.project_target() {
            self.open.insert(id, true);
            self.geoms
                .entry(id)
                .or_insert_with(|| Geom::new(egui::pos2(20.0, 20.0), egui::vec2(420.0, 440.0)))
                .state = state;
            self.active = Some(id);
            self.raise = Some(id);
        }
    }

    fn begin_edit(&mut self, label: &str) {
        self.history.begin(&self.project, label);
    }

    fn finish_edit(&mut self) {
        self.history.commit(&self.project);
        self.last_history_group = None;
    }

    fn finish_frame_edit(&mut self, group: Option<egui::Id>) {
        if group.is_none() {
            self.finish_edit();
        } else {
            self.last_history_group = group;
        }
    }

    fn track_headers(&mut self, id: u64, edits: &[(usize, usize, String, String)]) {
        if edits.is_empty() {
            return;
        }
        if !self.history.is_pending() {
            for (col, row, before, _) in edits {
                *header_field(&mut self.project.book_mut(id).unwrap().columns[*col], *row) = before.clone();
            }
            self.begin_edit("修改列属性");
            for (col, row, _, after) in edits {
                *header_field(&mut self.project.book_mut(id).unwrap().columns[*col], *row) = after.clone();
            }
        }
        self.history.changed();
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
                    restore: None,
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

    pub(crate) fn open_nlfit_dialog(&mut self) {
        self.dialog = Some(Dialog::NlFit {
            model: scicanvas_core::nlfit::Model::Gauss,
            fixed: [(false, 0.0); 4],
        });
    }

    fn nlfit(&mut self, model: scicanvas_core::nlfit::Model, fixed: &[Option<f64>]) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.nlfit(&src, model, fixed) {
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

    pub(crate) fn open_smooth_dialog(&mut self) {
        self.dialog = Some(Dialog::Smooth {
            method: SmoothMethod::SavitzkyGolay,
            window: 7,
        });
    }

    pub(crate) fn open_set_values_dialog(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(book) = self.project.book(id) else { return };
        let Some(target) = keys.first().and_then(|k| book.column(k)) else {
            self.status = "请先单击列标题选中要设置的列".into();
            return;
        };
        self.dialog = Some(Dialog::SetValues {
            target: format!("{}（{}）", target.key, target.display_name()),
            formula: String::new(),
            columns: book
                .columns
                .iter()
                .map(|c| (c.key.clone(), c.display_name().to_owned()))
                .collect(),
            error: None,
        });
    }

    /// 按公式设置第一个点选列的值；公式有误时留在对话框里显示错误。
    fn set_column_values(&mut self, formula: &str) -> bool {
        let Some((id, keys)) = self.ttest_columns() else {
            return false;
        };
        let Some(target) = keys.first().cloned() else {
            return false;
        };
        match self.project.set_column_values(id, &target, formula) {
            Ok(n) => {
                self.status = format!("已按公式设置列 {target}（{n} 行）");
                true
            }
            Err(e) => {
                if let Some(Dialog::SetValues { error, .. }) = &mut self.dialog {
                    *error = Some(e.clone());
                }
                self.status = e;
                false
            }
        }
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
                let book = self.project.book(r.book_id)?;
                let item = r.items.first()?;
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

    pub(crate) fn open_fill_dialog(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(book) = self.project.book(id) else { return };
        let Some(target) = keys.first().and_then(|k| book.column(k)) else {
            self.status = "请先单击列标题选中要填充的列".into();
            return;
        };
        self.dialog = Some(Dialog::Fill {
            target: format!("{}（{}）", target.key, target.display_name()),
            kind: 0,
            a: 1.0,
            b: 1.0,
            rows: if book.nrows() > 0 { book.nrows() } else { 10 },
        });
    }

    fn fill_sequence(&mut self, kind: scicanvas_core::stats::Sequence, rows: usize) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(target) = keys.first().cloned() else { return };
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        match self.project.fill_sequence(id, &target, kind, rows, seed) {
            Ok(()) => self.status = format!("已填充列 {target}（{rows} 行）"),
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
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.is_empty() {
            self.status = "请先在工作表里单击选中要检验的列".into();
            return;
        }
        let book = self.project.book(id).unwrap();
        let mut rows = vec![["列", "N", "W", "p", "α = 0.05"].map(String::from).to_vec()];
        let mut problems = vec![];
        for k in &keys {
            let c = book.column(k).unwrap();
            match scicanvas_core::stats::shapiro_wilk(&c.values()) {
                Ok(r) => rows.push(vec![
                    c.display_name().to_owned(),
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
                Err(e) => problems.push(format!("{}：{e}", c.display_name())),
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
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.is_empty() {
            self.status = "请先在工作表里单击选中要检验的列".into();
            return;
        }
        let book = self.project.book(id).unwrap();
        let mut rows = vec![["列", "N", "可疑值", "所在行", "G", "临界值 G(0.05)", "p", "结论"]
            .map(String::from)
            .to_vec()];
        let mut problems = vec![];
        for k in &keys {
            let c = book.column(k).unwrap();
            match scicanvas_core::stats::grubbs(&c.values(), 0.05) {
                Ok(r) => rows.push(vec![
                    c.display_name().to_owned(),
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
                Err(e) => problems.push(format!("{}：{e}", c.display_name())),
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
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.len() < 2 {
            self.status = "请先在工作表里单击选中至少两列（每列一组）".into();
            return;
        }
        let book = self.project.book(id).unwrap();
        let groups: Vec<Vec<f64>> = keys.iter().map(|k| book.column(k).unwrap().values()).collect();
        let names: Vec<String> = keys
            .iter()
            .map(|k| book.column(k).unwrap().display_name().to_owned())
            .collect();
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
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.len() < 2 {
            self.status = "请先在工作表里单击选中至少两列（Ctrl+单击多选）".into();
            return;
        }
        let book = self.project.book(id).unwrap();
        let mut pairs = vec![];
        let mut problems = vec![];
        for i in 0..keys.len() {
            for j in i + 1..keys.len() {
                let (a, b) = (book.column(&keys[i]).unwrap(), book.column(&keys[j]).unwrap());
                let found: Result<Vec<_>, String> = CorrMethod::ALL
                    .iter()
                    .map(|m| correlation(&a.values(), &b.values(), *m))
                    .collect();
                match found {
                    Ok(cs) => pairs.push((a.display_name().to_owned(), b.display_name().to_owned(), cs)),
                    Err(e) => problems.push(format!("{} 与 {}：{e}", a.display_name(), b.display_name())),
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
        let Some((id, keys)) = self.ttest_columns() else { return };
        let book = self.project.book(id).unwrap();
        let columns = keys
            .iter()
            .take(2)
            .map(|k| book.column(k).unwrap().display_name().to_owned())
            .collect();
        self.dialog = Some(Dialog::TTest {
            columns,
            kind: if keys.len() >= 2 { 1 } else { 0 },
            mu: 0.0,
            equal_var: false,
            level: 0.95,
        });
    }

    fn run_ttest(&mut self, kind: scicanvas_core::stats::TTestKind, level: f64) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let book = self.project.book(id).unwrap();
        let values = |k: Option<&String>| k.and_then(|k| book.column(k)).map(|c| c.values()).unwrap_or_default();
        let names = keys
            .iter()
            .take(2)
            .map(|k| book.column(k).unwrap().display_name().to_owned())
            .collect();
        match scicanvas_core::stats::t_test(kind, &values(keys.first()), &values(keys.get(1)), level) {
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
        match dialogs::show(ctx, d, &self.pal) {
            dialogs::Action::None => {}
            dialogs::Action::Close => self.dialog = None,
            dialogs::Action::ColumnProps(edit) => {
                self.finish_edit();
                self.begin_edit("列属性");
                match self.apply_column_props(edit) {
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
            dialogs::Action::NlFit(model, fixed) => {
                self.finish_edit();
                self.begin_edit("非线性拟合");
                self.dialog = None;
                self.nlfit(model, &fixed);
                self.history.changed();
                self.finish_edit();
            }
            dialogs::Action::SetValues(formula) => {
                self.finish_edit();
                self.begin_edit("设置列值");
                if self.set_column_values(&formula) {
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

    fn project_item_menu(&mut self, ui: &mut egui::Ui, id: u64) {
        self.explorer_target = Some(id);
        let folder = self.project.folder(id).is_some();
        if folder {
            if ui.button("新建子文件夹").clicked() {
                self.run_command("project.folder.new");
                ui.close_menu();
            }
        } else {
            for (command, label) in [
                ("project.show", "显示窗口"),
                ("project.hide", "隐藏窗口"),
                ("window.minimize", "最小化"),
                ("window.maximize", "最大化"),
                ("window.restore", "还原"),
            ] {
                if ui.button(label).clicked() {
                    self.run_command(command);
                    ui.close_menu();
                }
            }
        }
        ui.separator();
        ui.label("名称");
        let response = ui.text_edit_singleline(&mut self.project_name);
        if ui.button("重命名").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            self.run_command("project.rename");
            ui.close_menu();
        }
        ui.separator();
        egui::ComboBox::from_id_salt(("move-project", id))
            .selected_text(
                self.folder_destination
                    .and_then(|d| self.project.folder_path(d))
                    .unwrap_or_else(|| "项目根目录".into()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.folder_destination, None, "项目根目录");
                for f in &self.project.folders {
                    if f.id != id {
                        ui.selectable_value(
                            &mut self.folder_destination,
                            Some(f.id),
                            self.project.folder_path(f.id).unwrap(),
                        );
                    }
                }
            });
        if ui.button("移动到所选位置").clicked() {
            self.run_command("project.move");
            ui.close_menu();
        }
        ui.separator();
        if folder {
            if ui.button("删除文件夹（保留内容）").clicked() {
                self.run_command("project.folder.remove");
                ui.close_menu();
            }
        } else if ui.button("删除对象…").clicked() {
            self.run_command("project.delete");
            ui.close_menu();
        }
    }

    fn start_explorer_rename(&mut self, id: u64) {
        self.explorer_target = Some(id);
        self.project_name = self.project.name_of(id).unwrap_or("").into();
        self.explorer_click = None;
        self.explorer_rename = Some((id, true));
    }

    fn finish_explorer_rename(&mut self, cancel: bool) {
        if let Some((id, _)) = self.explorer_rename.take() {
            if !cancel {
                self.explorer_target = Some(id);
                self.run_command("project.rename");
            }
            self.project_name = self.project.name_of(id).unwrap_or("").into();
        }
    }

    fn explorer_label(&mut self, ui: &mut egui::Ui, id: u64, text: RichText) -> egui::Response {
        if let Some((editing, focus)) = self.explorer_rename {
            if editing == id {
                let mut output = egui::TextEdit::singleline(&mut self.project_name)
                    .id_salt(("project-rename", id))
                    .desired_width(ui.available_width())
                    .show(ui);
                if focus {
                    output.response.request_focus();
                    output.state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.project_name.chars().count()),
                    )));
                    output.state.store(ui.ctx(), output.response.id);
                    self.explorer_rename = Some((id, false));
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.finish_explorer_rename(true);
                } else if output.response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.finish_explorer_rename(false);
                }
                return output.response;
            }
        }
        let response = ui.add(
            egui::Button::new(text)
                .frame(false)
                .selected(self.explorer_target == Some(id))
                .sense(egui::Sense::click_and_drag()),
        );
        response.dnd_set_drag_payload(ProjectDrag(id));
        if response.drag_started() {
            self.finish_explorer_rename(false);
            self.explorer_target = Some(id);
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            self.explorer_click = None;
        }
        if response.clicked() {
            self.finish_explorer_rename(false);
            let now = ui.input(|i| i.time);
            let slow_second_click = self.explorer_target == Some(id)
                && self
                    .explorer_click
                    .is_some_and(|(last, time)| last == id && (0.5..=2.0).contains(&(now - time)));
            self.explorer_target = Some(id);
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            if self.project.folder(id).is_none() {
                self.run_command("project.show");
            }
            if slow_second_click {
                self.start_explorer_rename(id);
            } else {
                self.explorer_click = Some((id, now));
            }
        }
        if response.secondary_clicked() {
            self.finish_explorer_rename(false);
            self.explorer_click = None;
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            self.folder_destination = self
                .project
                .folder(id)
                .map_or_else(|| self.project.folder_of(id), |f| f.parent);
        }
        response.context_menu(|ui| self.project_item_menu(ui, id));
        response.on_hover_text("拖到文件夹或项目根目录移动；选中后隔 0.5–2 秒再单击名称重命名，也可按 F2")
    }

    fn explorer_drop(&mut self, ui: &mut egui::Ui, response: &egui::Response, destination: Option<u64>) {
        if let Some(payload) = response.dnd_hover_payload::<ProjectDrag>() {
            let mut ancestor = destination;
            let mut valid = true;
            while let Some(id) = ancestor {
                if id == payload.0 {
                    valid = false;
                    break;
                }
                ancestor = self.project.folder(id).and_then(|f| f.parent);
            }
            ui.ctx().set_cursor_icon(if valid {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::NotAllowed
            });
            ui.painter()
                .rect_stroke(response.rect.expand(2.0), 3.0, ui.visuals().selection.stroke);
        }
        if let Some(payload) = response.dnd_release_payload::<ProjectDrag>() {
            self.finish_explorer_rename(false);
            self.explorer_click = None;
            let source = payload.0;
            let current = self
                .project
                .folder(source)
                .map_or_else(|| self.project.folder_of(source), |f| f.parent);
            if current != destination {
                self.explorer_target = Some(source);
                self.folder_destination = destination;
                self.run_command("project.move");
                if let Some(id) = destination {
                    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        egui::Id::new(("project-folder", id)),
                        true,
                    );
                    state.set_open(true);
                    state.store(ui.ctx());
                }
            }
        }
    }

    fn explorer_folder(&mut self, ui: &mut egui::Ui, parent: Option<u64>) {
        let folders: Vec<_> = self
            .project
            .folders
            .iter()
            .filter(|f| f.parent == parent)
            .map(|f| (f.id, f.name.clone()))
            .collect();
        for (id, name) in folders {
            let (arrow, header, _) = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                egui::Id::new(("project-folder", id)),
                true,
            )
            .show_header(ui, |ui| self.explorer_label(ui, id, RichText::new(name)))
            .body(|ui| self.explorer_folder(ui, Some(id)));
            if arrow.clicked() {
                self.explorer_click = None;
            }
            self.explorer_drop(ui, &header.inner.union(arrow), Some(id));
        }
        let items: Vec<_> = self
            .project
            .books
            .iter()
            .map(|b| (b.id, true, b.name.clone()))
            .chain(self.project.graphs.iter().map(|g| (g.id, false, g.name.clone())))
            .filter(|(id, _, _)| self.project.folder_of(*id) == parent)
            .collect();
        for (id, is_book, name) in items {
            let shown = self.open.get(&id).copied().unwrap_or(false);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_book { "表" } else { "图" })
                        .small()
                        .strong()
                        .color(if is_book { self.pal.book } else { self.pal.graph }),
                );
                self.explorer_label(
                    ui,
                    id,
                    if shown {
                        RichText::new(&name)
                    } else {
                        RichText::new(&name).weak()
                    },
                )
            });
        }
    }

    fn explorer(&mut self, ui: &mut egui::Ui) {
        if let Some(pos) = ui.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten()) {
            self.explorer_focus = ui.max_rect().contains(pos) && ui.ctx().layer_id_at(pos) == Some(ui.layer_id());
        }
        ui.add_space(4.0);
        ui.label(RichText::new("项目管理器").strong());
        ui.horizontal(|ui| {
            let root = ui.button("项目根目录");
            if root.clicked() {
                self.finish_explorer_rename(false);
                self.explorer_target = None;
                self.project_name.clear();
                self.explorer_click = None;
            }
            self.explorer_drop(ui, &root, None);
            if ui.button("新建文件夹").clicked() {
                self.run_command("project.folder.new");
            }
        });
        // 面板先于子窗口绘制：只在项目管理器有焦点时处理 F2，并消费按键，避免工作表同时开始编辑单元格。
        if self.explorer_rename.is_none() && self.explorer_focus && !ui.ctx().wants_keyboard_input() {
            if let Some(id) = self.project_target() {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
                    self.start_explorer_rename(id);
                }
            }
        }
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| self.explorer_folder(ui, None));
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
                    if self.explorer_target != Some(id) {
                        self.project_name = self.project.name_of(id).unwrap_or("").to_string();
                    }
                    self.explorer_target = Some(id);
                }
            }
        }

        let mut min_slot = 0;
        let mut snapped = None;
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
            if let Some(snap) = resp.snapped {
                snapped = Some((id, snap));
            }
            if resp.pressed {
                self.active = Some(id);
                if self.explorer_target != Some(id) {
                    self.project_name = self.project.name_of(id).unwrap_or("").to_string();
                }
                self.explorer_target = Some(id);
            }
            if let Some(state) = resp.state {
                self.explorer_target = Some(id);
                self.run_command(match state {
                    WinState::Min => "window.minimize",
                    WinState::Max => "window.maximize",
                    WinState::Normal => "window.restore",
                });
            }
            if resp.close {
                self.explorer_target = Some(id);
                self.run_command("project.hide");
            }
        }
        self.raise = None;
        if let Some((id, snap)) = snapped {
            self.start_snap_assist(id, snap);
        }
        self.snap_assist_ui(ctx, ws);
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
        let mut metadata_edits = Vec::new();
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
                                    // 双击列标题打开列属性（与 Origin 相同）
                                    if resp.double_clicked() {
                                        action = Some((ci, ColAction::Props));
                                    }
                                    resp.context_menu(|ui| {
                                        let mut pick = |ui: &mut egui::Ui, label: &str, a: ColAction| {
                                            if ui.button(label).clicked() {
                                                action = Some((ci, a));
                                                ui.close_menu();
                                            }
                                        };
                                        pick(ui, "列属性…", ColAction::Props);
                                        ui.separator();
                                        pick(ui, "设为 X", ColAction::SetRole(Role::X));
                                        pick(ui, "设为 Y", ColAction::SetRole(Role::Y));
                                        pick(ui, "设为 X 误差", ColAction::SetRole(Role::XError));
                                        pick(ui, "设为 Y 误差", ColAction::SetRole(Role::YError));
                                        pick(ui, "设为忽略", ColAction::SetRole(Role::Ignore));
                                        ui.separator();
                                        ui.label(format!("数据类型：{}", col.data.kind().label()));
                                        for kind in scicanvas_core::DataKind::ALL {
                                            pick(
                                                ui,
                                                &format!("转换为{}列", kind.label()),
                                                ColAction::SetDataKind(kind),
                                            );
                                        }
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
                                        ui.centered_and_justified(|ui| {
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
                                            let field = header_field(col, ri);
                                            let before = field.clone();
                                            let response = ui.add(
                                                egui::TextEdit::singleline(field)
                                                    .frame(false)
                                                    .horizontal_align(egui::Align::Center)
                                                    .margin(egui::vec2(5.0, 3.0))
                                                    .desired_width(f32::INFINITY),
                                            );
                                            if response.changed() {
                                                metadata_edits.push((ci, ri, before, field.clone()));
                                            }
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
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
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
                                            edit_group(&resp, &mut self.history_group);
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
                                        let shown = (r < col.data.len()).then(|| col.display_at(r));
                                        if let Some(s) = shown.filter(|s| !s.is_empty()) {
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
                                                ("row.insert", "插入行"),
                                                ("row.delete", "删除行"),
                                                ("", ""),
                                                ("plot.line", "折线图"),
                                                ("plot.scatter", "散点图"),
                                                ("plot.linesym", "点线图"),
                                                ("plot.bar", "柱状图"),
                                                ("plot.stacked_bar", "堆叠柱图"),
                                                ("plot.percent_stacked_bar", "百分比堆叠柱图"),
                                                ("plot.floating_bar", "浮动柱图"),
                                                ("plot.horizontal_bar", "条形图"),
                                                ("plot.area", "面积图"),
                                                ("plot.spline", "样条曲线图"),
                                                ("plot.spline_symbol", "样条点线图"),
                                                ("plot.vertical_drop", "垂线图"),
                                                ("plot.step_horizontal", "水平阶梯图"),
                                                ("plot.step_vertical", "垂直阶梯图"),
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
        self.track_headers(id, &metadata_edits);
        if edit_end.is_some_and(|(ok, _, _)| ok) || (press.is_some() && edit.is_some()) {
            self.begin_edit("编辑单元格");
        }
        let book = self.project.book_mut(id).unwrap();
        let last_col = ncols.saturating_sub(1);
        let mut status: Option<String> = None;
        let mut clear_cols = false;
        let mut new_cols: Option<(usize, usize)> = None;
        let mut want_focus = false;
        let mut cell_changed = false;
        let mut conversion = None;
        let mut commit = |book: &mut scicanvas_core::Book, e: &Edit| {
            if let Some(col) = book.columns.get_mut(e.cell.c) {
                if !col.data.accepts(&e.buf) {
                    conversion = Some(Dialog::ConvertText {
                        book: id,
                        key: col.key.clone(),
                        row: e.cell.r,
                        value: e.buf.clone(),
                    });
                    return;
                }
                cell_changed = true;
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
                .map(std::borrow::Cow::into_owned)
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
        if let Some(dialog) = conversion {
            self.dialog = Some(dialog);
        }
        if cell_changed {
            self.history.changed();
        }
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
                ColAction::Props => "col.props",
                ColAction::SetDataKind(scicanvas_core::DataKind::Num) => "col.type.num",
                ColAction::SetDataKind(scicanvas_core::DataKind::Text) => "col.type.text",
                ColAction::SetDataKind(scicanvas_core::DataKind::DateTime) => "col.type.datetime",
                ColAction::SetDataKind(scicanvas_core::DataKind::Category) => "col.type.category",
                ColAction::SetRole(Role::X) => "col.setX",
                ColAction::SetRole(Role::Y) => "col.setY",
                ColAction::SetRole(Role::XError) => "col.setXError",
                ColAction::SetRole(Role::YError) => "col.setYError",
                ColAction::SetRole(Role::Ignore) => "col.setIgnore",
                ColAction::InsertRight => "col.insert",
                ColAction::Clear => "col.clear",
                ColAction::Delete => "col.delete",
                ColAction::Plot(PlotType::Line) => "plot.line",
                ColAction::Plot(PlotType::Scatter) => "plot.scatter",
                ColAction::Plot(PlotType::LineSymbol) => "plot.linesym",
                ColAction::Plot(PlotType::Bar) => "plot.bar",
                ColAction::Plot(PlotType::StackedBar) => "plot.stacked_bar",
                ColAction::Plot(PlotType::PercentStackedBar) => "plot.percent_stacked_bar",
                ColAction::Plot(PlotType::FloatingBar) => "plot.floating_bar",
                ColAction::Plot(PlotType::HorizontalBar) => "plot.horizontal_bar",
                ColAction::Plot(PlotType::Area) => "plot.area",
                ColAction::Plot(PlotType::Spline) => "plot.spline",
                ColAction::Plot(PlotType::SplineSymbol) => "plot.spline_symbol",
                ColAction::Plot(PlotType::VerticalDrop) => "plot.vertical_drop",
                ColAction::Plot(PlotType::StepHorizontal) => "plot.step_horizontal",
                ColAction::Plot(PlotType::StepVertical) => "plot.step_vertical",
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
        // 项目管理器先于子窗口绘制：最后一次点击落在项目管理器时，按键不属于工作表。
        if self.explorer_focus {
            return;
        }
        let Some(book) = self.project.book(id) else { return };
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
        let writes = events.iter().any(|e| {
            matches!(
                e,
                Event::Cut
                    | Event::Paste(_)
                    | Event::Key {
                        key: Key::Delete | Key::Backspace,
                        ..
                    }
            )
        });
        if writes {
            self.begin_edit("编辑工作表");
        }
        let book = self.project.book_mut(id).unwrap();

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
                            start_edit = Some(
                                book.columns[g.a.c]
                                    .data
                                    .get(g.a.r)
                                    .map(std::borrow::Cow::into_owned)
                                    .unwrap_or_default(),
                            );
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
                    let conflicts: Vec<String> = if csv::is_block_paste(&text) {
                        csv::paste_conflicts(book, g.c0(), g.r0(), &text)
                    } else {
                        book.columns[g.c0()..=g.c1()]
                            .iter()
                            .filter(|c| !c.data.accepts(text.trim()))
                            .map(|c| c.key.clone())
                            .collect()
                    };
                    if !conflicts.is_empty() {
                        status = Some(format!(
                            "列 {} 与粘贴内容类型不符，数据保持原样。请从列头右键转换为文本列后再粘贴。",
                            conflicts.join(", ")
                        ));
                        continue;
                    }
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
        if writes {
            self.history.changed();
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
            // 内容较长：默认高到屏幕高度的 85%，再长时滚动
            .default_height(ctx.screen_rect().height() * 0.85)
            .vscroll(true)
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
                ui.horizontal(|ui| {
                    let mut custom = g.placement.is_some();
                    if ui.checkbox(&mut custom, "自定义图层位置").changed() {
                        g.placement = custom.then(scicanvas_core::page::LayerRect::default);
                    }
                    ui.checkbox(&mut g.transparent, "透明图层背景");
                });
                if let Some(rect) = &mut g.placement {
                    ui.label("位置和尺寸（页面比例，0–1）");
                    egui::Grid::new(("layer-rectangle", gid, props_layer))
                        .num_columns(4)
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
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
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
                        if g.plot_type == PlotType::Area {
                            ui.label("填充透明度");
                            edit_group(
                                &ui.add(egui::Slider::new(&mut g.area_opacity, 0.0..=1.0).text("面积填充")),
                                history_group,
                            );
                            ui.end_row();
                        }
                        ui.label("显示");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut g.grid, "网格线");
                            ui.checkbox(&mut g.legend, "图例");
                            ui.checkbox(&mut g.labels.visible, "数据标签");
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
            } else {
                self.explorer_focus = false;
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
        self.finish_frame_edit(self.history_group);

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

    struct ExplorerUi {
        ctx: egui::Context,
        time: f64,
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
                sheet: None,
            }
        }

        fn frame(&mut self, app: &mut SciApp, events: Vec<egui::Event>) -> egui::FullOutput {
            self.time += 0.02;
            self.ctx.run(
                egui::RawInput {
                    time: Some(self.time),
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
        frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        let at = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "新建文件夹" => {
                    Some(t.pos + t.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap();
        let before = app.project.clone();
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
        app.nlfit(Model::ExpDec1, &[]);
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
    fn set_values_keeps_the_dialog_open_on_errors_and_undoes_in_one_step() {
        let mut app = app();
        app.active = Some(1);
        app.selection.insert(1, vec!["C".into()]);
        app.open_set_values_dialog();
        assert!(matches!(&app.dialog, Some(Dialog::SetValues { target, .. }) if target.starts_with('C')));
        assert!(!app.set_column_values("B *"));
        assert!(matches!(&app.dialog, Some(Dialog::SetValues { error: Some(_), .. })));
        let before = app.project.book(1).unwrap().columns[2].data.clone();
        app.begin_edit("设置列值");
        assert!(app.set_column_values("B * 2"));
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
        app.apply_column_props(edit.clone()).unwrap();
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
        let err = app.apply_column_props(edit).unwrap_err();
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
