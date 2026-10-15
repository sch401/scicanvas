//! 命令注册表。
//!
//! 每个用户操作都是一条命令：功能区按钮、右键菜单、Ctrl+K 命令面板和快捷键都调用同一套命令。
//! 以后的脚本接口、MCP 服务器（让 AI 代理驱动软件）也会通过它执行操作。

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
        modifies: matches!(
            id,
            "book.new"
                | "file.import"
                | "edit.clear"
                | "fit.linear"
                | "math.diff"
                | "math.integ"
                | "stats.columns"
                | "sort.asc"
                | "sort.desc"
        ) || id.starts_with("plot.")
            || id.starts_with("layer.")
            || id.starts_with("col.")
            || matches!(id, "graph.logx" | "graph.logy" | "graph.grid" | "graph.legend"),
        checked: None,
    }
}

impl Command {
    fn checked(mut self, f: fn(&SciApp) -> bool) -> Self {
        self.checked = Some(f);
        self
    }
}

fn build() -> Vec<Command> {
    vec![
        // 文件
        c(
            "book.new",
            "新建工作簿",
            "newbook",
            "Ctrl+N",
            "文件",
            "new workbook sheet",
            always,
            |a| a.new_book(),
        ),
        c(
            "file.import",
            "导入数据",
            "import",
            "Ctrl+I",
            "文件",
            "import csv txt excel",
            always,
            |a| a.import_csv(),
        ),
        c(
            "file.open",
            "打开项目",
            "open",
            "Ctrl+O",
            "文件",
            "open project",
            always,
            |a| a.open_project(),
        ),
        c(
            "file.save",
            "保存项目",
            "save",
            "Ctrl+S",
            "文件",
            "save project",
            always,
            |a| a.save_project(false),
        ),
        c(
            "file.saveas",
            "项目另存为",
            "save",
            "",
            "文件",
            "save as",
            always,
            |a| a.save_project(true),
        ),
        c(
            "graph.export",
            "导出图形",
            "export",
            "Ctrl+E",
            "文件",
            "export svg image",
            need_any_graph,
            |a| a.export_svg(),
        ),
        c(
            "project.reset",
            "恢复示例项目",
            "reset",
            "",
            "文件",
            "reset sample",
            always,
            |a| a.reset_project(),
        ),
        c("app.quit", "退出", "quit", "", "文件", "quit exit", always, |a| {
            a.quit = true
        }),
        // 帮助
        c(
            "help.show",
            "快速上手",
            "help",
            "F1",
            "帮助",
            "help",
            always,
            |a| a.show_help = true,
        ),
        c(
            "palette.open",
            "搜索命令",
            "search",
            "Ctrl+K",
            "帮助",
            "command palette search",
            always,
            |a| a.ui.open_palette(),
        ),
        // 选区
        c(
            "edit.undo",
            "撤销",
            "undo",
            "Ctrl+Z",
            "编辑",
            "undo 撤销",
            need_undo,
            |a| a.undo(),
        ),
        c(
            "edit.redo",
            "重做",
            "redo",
            "Ctrl+Y / Ctrl+Shift+Z",
            "编辑",
            "redo 重做",
            need_redo,
            |a| a.redo(),
        ),
        c(
            "edit.copy",
            "复制",
            "copy",
            "Ctrl+C",
            "选区",
            "copy cells clipboard",
            need_range,
            |a| a.copy_range(),
        ),
        c(
            "edit.clear",
            "清除内容",
            "clear",
            "Delete",
            "选区",
            "clear cells delete",
            need_range,
            |a| a.clear_range(),
        ),
        // 绘图
        c(
            "plot.builder",
            "作图面板与图库",
            "props",
            "",
            "绘图",
            "gallery builder 拖放 图库",
            need_data,
            |a| a.open_plot_builder(),
        ),
        c(
            "plot.apply_spec",
            "创建预览图形",
            "linesym",
            "",
            "绘图",
            "plot spec",
            need_plot_draft,
            |a| a.apply_plot_spec(),
        ),
        c(
            "plot.line",
            "折线图",
            "line",
            "",
            "绘图",
            "plot line",
            need_data,
            |a| a.plot(PlotType::Line),
        ),
        c(
            "plot.scatter",
            "散点图",
            "scatter",
            "",
            "绘图",
            "plot scatter",
            need_data,
            |a| a.plot(PlotType::Scatter),
        ),
        c(
            "plot.linesym",
            "点线图",
            "linesym",
            "",
            "绘图",
            "plot line symbol",
            need_data,
            |a| a.plot(PlotType::LineSymbol),
        ),
        c(
            "plot.bar",
            "柱状图",
            "bar",
            "",
            "绘图",
            "plot bar column",
            need_data,
            |a| a.plot(PlotType::Bar),
        ),
        // 图形
        c(
            "layer.add",
            "添加图层",
            "newbook",
            "",
            "图形",
            "layer panel subplot 添加图层",
            need_graph,
            |a| a.add_layer(),
        ),
        c(
            "layer.remove",
            "删除图层",
            "coldel",
            "",
            "图形",
            "layer remove 删除图层",
            need_multi_layer,
            |a| a.remove_layer(),
        ),
        c(
            "graph.props",
            "绘图属性",
            "props",
            "",
            "图形",
            "properties format axis",
            need_any_graph,
            |a| a.open_props(),
        ),
        c(
            "graph.logx",
            "X 对数",
            "logx",
            "",
            "图形",
            "log scale x",
            need_graph,
            |a| a.toggle_graph(|g| &mut g.x_log, "X 轴对数刻度"),
        )
        .checked(|a| graph_flag(a, |g| g.x_log)),
        c(
            "graph.logy",
            "Y 对数",
            "logy",
            "",
            "图形",
            "log scale y",
            need_graph,
            |a| a.toggle_graph(|g| &mut g.y_log, "Y 轴对数刻度"),
        )
        .checked(|a| graph_flag(a, |g| g.y_log)),
        c(
            "graph.grid",
            "网格线",
            "grid",
            "",
            "图形",
            "grid",
            need_graph,
            |a| a.toggle_graph(|g| &mut g.grid, "网格线"),
        )
        .checked(|a| graph_flag(a, |g| g.grid)),
        c(
            "graph.legend",
            "图例",
            "legend",
            "",
            "图形",
            "legend",
            need_graph,
            |a| a.toggle_graph(|g| &mut g.legend, "图例"),
        )
        .checked(|a| graph_flag(a, |g| g.legend)),
        // 列与工作表
        c(
            "col.setX",
            "设为 X",
            "setx",
            "",
            "列",
            "set x column",
            need_cols,
            |a| a.set_role(Role::X),
        ),
        c(
            "col.setY",
            "设为 Y",
            "sety",
            "",
            "列",
            "set y column",
            need_cols,
            |a| a.set_role(Role::Y),
        ),
        c(
            "col.setIgnore",
            "设为忽略",
            "ignore",
            "",
            "列",
            "disregard",
            need_cols,
            |a| a.set_role(Role::Ignore),
        ),
        c(
            "col.insert",
            "插入列",
            "colins",
            "",
            "列",
            "add column",
            need_book,
            |a| a.add_column(),
        ),
        c(
            "col.delete",
            "删除列",
            "coldel",
            "",
            "列",
            "delete column",
            need_cols,
            |a| a.delete_columns(),
        ),
        c(
            "col.clear",
            "清空列",
            "colclear",
            "",
            "列",
            "clear column",
            need_cols,
            |a| a.clear_columns(),
        ),
        c(
            "sort.asc",
            "升序排序",
            "sortasc",
            "",
            "排序",
            "sort ascending",
            need_sortable,
            |a| a.sort_active(true),
        ),
        c(
            "sort.desc",
            "降序排序",
            "sortdesc",
            "",
            "排序",
            "sort descending",
            need_sortable,
            |a| a.sort_active(false),
        ),
        // 分析
        c(
            "fit.linear",
            "线性拟合",
            "fitlin",
            "",
            "分析",
            "linear fit regression",
            need_data,
            |a| a.fit(1),
        ),
        c(
            "fit.poly",
            "多项式拟合",
            "fitpoly",
            "",
            "分析",
            "polynomial fit regression",
            need_data,
            |a| a.open_poly_dialog(),
        ),
        c(
            "math.smooth",
            "平滑",
            "smooth",
            "",
            "分析",
            "smooth savitzky golay moving average",
            need_data,
            |a| a.open_smooth_dialog(),
        ),
        c(
            "math.diff",
            "微分",
            "diff",
            "",
            "分析",
            "derivative differentiate",
            need_data,
            |a| a.differentiate(),
        ),
        c(
            "math.integ",
            "积分",
            "integ",
            "",
            "分析",
            "integrate area",
            need_data,
            |a| a.integrate(),
        ),
        c(
            "math.norm",
            "归一化",
            "norm",
            "",
            "分析",
            "normalize",
            need_data,
            |a| a.open_normalize_dialog(),
        ),
        c(
            "stats.columns",
            "列统计",
            "stats",
            "",
            "统计",
            "descriptive statistics mean sd",
            need_data,
            |a| a.stats_on_columns(),
        ),
        // 视图
        c(
            "view.explorer",
            "项目管理器",
            "explorer",
            "",
            "视图",
            "project explorer",
            always,
            |a| a.show_explorer = !a.show_explorer,
        )
        .checked(|a| a.show_explorer),
        c(
            "win.showall",
            "显示全部窗口",
            "showall",
            "",
            "窗口",
            "show all windows",
            always,
            |a| a.show_all(),
        ),
        c(
            "win.cascade",
            "层叠",
            "cascade",
            "",
            "窗口",
            "cascade windows",
            always,
            |a| a.arrange_windows(false),
        ),
        c(
            "win.tile",
            "平铺",
            "tile",
            "",
            "窗口",
            "tile windows",
            always,
            |a| a.arrange_windows(true),
        ),
        c(
            "win.minall",
            "全部最小化",
            "minall",
            "",
            "窗口",
            "minimize all windows",
            always,
            |a| a.minimize_all(),
        ),
        c(
            "win.hideall",
            "隐藏全部窗口",
            "hideall",
            "",
            "窗口",
            "hide all windows",
            always,
            |a| a.hide_all(),
        ),
        c(
            "theme.light",
            "浅色",
            "sun",
            "",
            "主题",
            "light theme",
            always,
            |a| a.set_theme(ThemePreference::Light),
        )
        .checked(|a| a.theme_pref == ThemePreference::Light),
        c(
            "theme.dark",
            "深色",
            "moon",
            "",
            "主题",
            "dark theme",
            always,
            |a| a.set_theme(ThemePreference::Dark),
        )
        .checked(|a| a.theme_pref == ThemePreference::Dark),
        c(
            "theme.system",
            "跟随系统",
            "system",
            "",
            "主题",
            "system theme",
            always,
            |a| a.set_theme(ThemePreference::System),
        )
        .checked(|a| a.theme_pref == ThemePreference::System),
        c(
            "ribbon.toggle",
            "折叠功能区",
            "chevron",
            "Ctrl+F1",
            "视图",
            "collapse ribbon",
            always,
            |a| a.ui.toggle_collapsed(),
        ),
    ]
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
