//! 命令注册表。
//!
//! 每个用户操作都是一条命令：功能区按钮、右键菜单、Ctrl+K 命令面板和快捷键都调用同一套命令。
//! 以后的脚本接口、MCP 服务器（让 AI 代理驱动软件）也会通过它执行操作。

mod analysis;
mod data;
mod edit;
mod file;
mod plot;
mod view;
mod window;
mod workspace;

use std::sync::OnceLock;

use eframe::egui::ThemePreference;
use scicanvas_core::{PlotType, Role};

use crate::app::SciApp;

/// 命令当前能否执行；不能时给出原因（显示在提示和命令面板里）。
pub type When = fn(&SciApp) -> Result<(), &'static str>;

pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    /// 快捷键提示文字（实际按键在 app.rs 的 shortcuts 里处理）。
    pub keys: &'static str,
    /// 分组名，显示在命令面板里。
    pub group: &'static str,
    /// 搜索用的额外关键词（英文名等）。
    pub kw: &'static str,
    pub when: When,
    pub run: fn(&mut SciApp),
    /// 是否直接修改项目。对话框在确认提交时单独记录。
    pub modifies: bool,
    /// 开关类命令的当前状态。
    pub checked: Option<fn(&SciApp) -> bool>,
}

fn always(_: &SciApp) -> Result<(), &'static str> {
    Ok(())
}
fn need_undo(a: &SciApp) -> Result<(), &'static str> {
    a.history.undo_label().map(|_| ()).ok_or("当前没有可撤销的操作")
}
fn need_redo(a: &SciApp) -> Result<(), &'static str> {
    a.history.redo_label().map(|_| ()).ok_or("当前没有可重做的操作")
}
fn need_book(a: &SciApp) -> Result<(), &'static str> {
    a.active_book_id().map(|_| ()).ok_or("请先单击一个工作簿窗口")
}
fn need_project_item(a: &SciApp) -> Result<(), &'static str> {
    a.project_target().map(|_| ()).ok_or("请在项目管理器中选择对象")
}
fn need_folder(a: &SciApp) -> Result<(), &'static str> {
    a.project_target()
        .filter(|id| a.project.folder(*id).is_some())
        .map(|_| ())
        .ok_or("请先选择文件夹")
}
fn need_window(a: &SciApp) -> Result<(), &'static str> {
    a.project_target()
        .filter(|id| a.project.book(*id).is_some() || a.project.graph(*id).is_some() || a.project.note(*id).is_some())
        .map(|_| ())
        .ok_or("请先选择工作簿或图形")
}
fn need_graph(a: &SciApp) -> Result<(), &'static str> {
    a.active_graph_id().map(|_| ()).ok_or("请先单击一个图形窗口")
}
fn need_any_graph(a: &SciApp) -> Result<(), &'static str> {
    a.any_open_graph().map(|_| ()).ok_or("还没有打开的图形")
}
fn need_multi_layer(a: &SciApp) -> Result<(), &'static str> {
    need_graph(a)?;
    let graph = a.project.graph(a.active_graph_id().unwrap()).unwrap();
    if graph.page.layers().len() > 1 {
        Ok(())
    } else {
        Err("页面至少保留一个图层")
    }
}
fn need_plot_draft(a: &SciApp) -> Result<(), &'static str> {
    if a.plot_draft.is_some() {
        Ok(())
    } else {
        Err("请先打开作图面板并选择数据")
    }
}
fn need_data(a: &SciApp) -> Result<(), &'static str> {
    a.analysis_source().map(|_| ()).ok_or("请先单击一个工作簿或图形窗口")
}
fn need_cols(a: &SciApp) -> Result<(), &'static str> {
    let id = a.active_book_id().ok_or("请先单击一个工作簿窗口")?;
    if a.picked(id).0.is_empty() {
        Err("请先单击列标题选中列，或拖动选择单元格")
    } else {
        Ok(())
    }
}
fn need_range(a: &SciApp) -> Result<(), &'static str> {
    let id = a.active_book_id().ok_or("请先单击一个工作簿窗口")?;
    a.sheets
        .get(&id)
        .and_then(|s| s.range)
        .map(|_| ())
        .ok_or("请先单击或拖动选择单元格")
}
fn need_sortable(a: &SciApp) -> Result<(), &'static str> {
    let id = a.active_book_id().ok_or("请先单击一个工作簿窗口")?;
    a.sort_column(id).map(|_| ()).ok_or("请先选中一列，或单击一个单元格")
}
fn graph_flag(a: &SciApp, f: fn(&scicanvas_core::page::Layer) -> bool) -> bool {
    a.active_layer().map(f).unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn c(
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    keys: &'static str,
    group: &'static str,
    kw: &'static str,
    when: When,
    run: fn(&mut SciApp),
) -> Command {
    Command {
        id,
        label,
        icon,
        keys,
        group,
        kw,
        when,
        run,
        modifies: false,
        checked: None,
    }
}

impl Command {
    fn modifies(mut self) -> Self {
        self.modifies = true;
        self
    }
    fn checked(mut self, f: fn(&SciApp) -> bool) -> Self {
        self.checked = Some(f);
        self
    }
}

fn build() -> Vec<Command> {
    let mut commands = Vec::new();
    window::reset(&mut commands);
    plot::layer_contents(&mut commands);
    analysis::curve_selection(&mut commands);
    workspace::objects(&mut commands);
    window::current(&mut commands);
    file::project(&mut commands);
    file::recent(&mut commands);
    plot::annotations(&mut commands);
    file::graphics(&mut commands);
    edit::graph_copy(&mut commands);
    file::session(&mut commands);
    view::help(&mut commands);
    edit::selection(&mut commands);
    plot::builder(&mut commands);
    plot::templates(&mut commands);
    plot::basic(&mut commands);
    plot::decorations(&mut commands);
    plot::graph(&mut commands);
    data::types_and_roles(&mut commands);
    data::rows(&mut commands);
    data::columns(&mut commands);
    data::sort(&mut commands);
    analysis::fits(&mut commands);
    analysis::signal(&mut commands);
    analysis::calculus(&mut commands);
    analysis::statistics(&mut commands);
    view::explorer(&mut commands);
    window::arrange(&mut commands);
    view::appearance(&mut commands);
    workspace::note(&mut commands);
    file::note(&mut commands);
    plot::examples(&mut commands);
    commands
}

pub fn commands() -> &'static [Command] {
    static C: OnceLock<Vec<Command>> = OnceLock::new();
    C.get_or_init(build)
}

pub fn find(id: &str) -> Option<&'static Command> {
    commands().iter().find(|c| c.id == id)
}

/// 鼠标悬停提示：名称（快捷键） — 不可用的原因
pub fn tooltip(app: &SciApp, cmd: &Command) -> String {
    let mut s = cmd.label.to_string();
    let label = match cmd.id {
        "edit.undo" => app.history.undo_label(),
        "edit.redo" => app.history.redo_label(),
        _ => None,
    };
    if let Some(label) = label {
        s.push_str(&format!("：{label}"));
    }
    if !cmd.keys.is_empty() {
        s.push_str(&format!("（{}）", cmd.keys));
    }
    if let Err(why) = (cmd.when)(app) {
        s.push_str(" — ");
        s.push_str(why);
    }
    s
}

pub(crate) fn axis_scale_command(x: bool, scale: scicanvas_core::ticks::AxisScale) -> &'static str {
    match (x, scale) {
        (true, scicanvas_core::ticks::AxisScale::Linear) => "graph.scale.x.linear",
        (true, scicanvas_core::ticks::AxisScale::Log10) => "graph.scale.x.log10",
        (true, scicanvas_core::ticks::AxisScale::Log2) => "graph.scale.x.log2",
        (true, scicanvas_core::ticks::AxisScale::Ln) => "graph.scale.x.ln",
        (true, scicanvas_core::ticks::AxisScale::Reciprocal) => "graph.scale.x.reciprocal",
        (true, scicanvas_core::ticks::AxisScale::Probability) => "graph.scale.x.probability",
        (true, scicanvas_core::ticks::AxisScale::Logit) => "graph.scale.x.logit",
        (false, scicanvas_core::ticks::AxisScale::Linear) => "graph.scale.y.linear",
        (false, scicanvas_core::ticks::AxisScale::Log10) => "graph.scale.y.log10",
        (false, scicanvas_core::ticks::AxisScale::Log2) => "graph.scale.y.log2",
        (false, scicanvas_core::ticks::AxisScale::Ln) => "graph.scale.y.ln",
        (false, scicanvas_core::ticks::AxisScale::Reciprocal) => "graph.scale.y.reciprocal",
        (false, scicanvas_core::ticks::AxisScale::Probability) => "graph.scale.y.probability",
        (false, scicanvas_core::ticks::AxisScale::Logit) => "graph.scale.y.logit",
    }
}
