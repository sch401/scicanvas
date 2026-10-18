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
        .filter(|id| a.project.book(*id).is_some() || a.project.graph(*id).is_some())
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
    vec![
        // 文件
        c(
            "project.folder.new",
            "新建文件夹",
            "open",
            "",
            "工作区",
            "folder new 文件夹",
            always,
            |a| a.new_folder(),
        )
        .modifies(),
        c(
            "project.folder.remove",
            "删除文件夹（保留内容）",
            "coldel",
            "",
            "工作区",
            "folder remove",
            need_folder,
            |a| a.remove_project_folder(),
        )
        .modifies(),
        c(
            "project.rename",
            "重命名项目对象",
            "props",
            "",
            "工作区",
            "rename folder book graph",
            need_project_item,
            |a| a.rename_project_target(),
        )
        .modifies(),
        c(
            "project.move",
            "移动项目对象",
            "open",
            "",
            "工作区",
            "move folder",
            need_project_item,
            |a| a.move_project_target(),
        )
        .modifies(),
        c(
            "project.show",
            "显示窗口",
            "book",
            "",
            "工作区",
            "show window",
            need_window,
            |a| a.show_project_window(),
        ),
        c(
            "project.hide",
            "隐藏窗口",
            "book",
            "",
            "工作区",
            "hide window",
            need_window,
            |a| a.hide_project_window(),
        ),
        c(
            "project.delete",
            "删除项目对象…",
            "coldel",
            "",
            "工作区",
            "delete item",
            need_window,
            |a| a.confirm_delete = a.project_target(),
        ),
        c(
            "window.minimize",
            "最小化当前窗口",
            "book",
            "",
            "工作区",
            "minimize",
            need_window,
            |a| a.set_project_window_state(crate::mdi::WinState::Min),
        ),
        c(
            "window.maximize",
            "最大化当前窗口",
            "book",
            "",
            "工作区",
            "maximize",
            need_window,
            |a| a.set_project_window_state(crate::mdi::WinState::Max),
        ),
        c(
            "window.restore",
            "还原当前窗口",
            "book",
            "",
            "工作区",
            "restore",
            need_window,
            |a| a.set_project_window_state(crate::mdi::WinState::Normal),
        ),
        c(
            "book.new",
            "新建工作簿",
            "newbook",
            "Ctrl+N",
            "文件",
            "new workbook sheet",
            always,
            |a| a.new_book(),
        )
        .modifies(),
        c(
            "file.import",
            "导入数据",
            "import",
            "Ctrl+I",
            "文件",
            "import csv txt excel",
            always,
            |a| a.import_csv(),
        )
        .modifies(),
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
            "导出 SVG",
            "export",
            "Ctrl+E",
            "文件",
            "export svg image vector 矢量",
            need_any_graph,
            |a| a.export_svg(),
        ),
        c(
            "book.export_csv",
            "导出 CSV",
            "export",
            "",
            "文件",
            "export csv excel table worksheet 表格 工作表",
            need_book,
            |a| a.export_csv(),
        ),
        c(
            "graph.export_png",
            "导出 PNG",
            "export",
            "",
            "文件",
            "export png bitmap raster dpi image 位图 分辨率",
            need_any_graph,
            |a| a.export_png_dialog(),
        ),
        c(
            "graph.copy",
            "复制图形",
            "copy",
            "Ctrl+J",
            "编辑",
            "copy graph clipboard image word ppt 剪贴板 粘贴",
            need_any_graph,
            |a| a.copy_graph(),
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
        )
        .modifies(),
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
        )
        .modifies(),
        c(
            "plot.panels_stacked",
            "上下两面板",
            "tile",
            "",
            "绘图",
            "multi panel stack vertical two panels 多面板 上下 堆叠",
            need_book,
            |a| a.plot_panels(scicanvas_core::Panels::Stacked),
        )
        .modifies(),
        c(
            "plot.panels_side",
            "左右两面板",
            "tile",
            "",
            "绘图",
            "multi panel side by side horizontal two panels 多面板 左右 并排",
            need_book,
            |a| a.plot_panels(scicanvas_core::Panels::SideBySide),
        )
        .modifies(),
        c(
            "plot.panels_grid4",
            "四面板",
            "tile",
            "",
            "绘图",
            "multi panel four 2x2 grid 多面板 四面板 网格",
            need_book,
            |a| a.plot_panels(scicanvas_core::Panels::Grid4),
        )
        .modifies(),
        c(
            "plot.line",
            "折线图",
            "line",
            "",
            "绘图",
            "plot line",
            need_data,
            |a| a.plot(PlotType::Line),
        )
        .modifies(),
        c(
            "plot.scatter",
            "散点图",
            "scatter",
            "",
            "绘图",
            "plot scatter",
            need_data,
            |a| a.plot(PlotType::Scatter),
        )
        .modifies(),
        c(
            "plot.linesym",
            "点线图",
            "linesym",
            "",
            "绘图",
            "plot line symbol",
            need_data,
            |a| a.plot(PlotType::LineSymbol),
        )
        .modifies(),
        c(
            "plot.bar",
            "柱状图",
            "bar",
            "",
            "绘图",
            "plot bar column",
            need_data,
            |a| a.plot(PlotType::Bar),
        )
        .modifies(),
        c(
            "plot.stacked_bar",
            "堆叠柱图",
            "bar",
            "",
            "绘图",
            "stacked column bar",
            need_data,
            |a| a.plot(PlotType::StackedBar),
        )
        .modifies(),
        c(
            "plot.percent_stacked_bar",
            "百分比堆叠柱图",
            "bar",
            "",
            "绘图",
            "100 percent stacked column bar",
            need_data,
            |a| a.plot(PlotType::PercentStackedBar),
        )
        .modifies(),
        c(
            "plot.floating_bar",
            "浮动柱图",
            "bar",
            "",
            "绘图",
            "floating column range bar",
            need_data,
            |a| a.plot(PlotType::FloatingBar),
        )
        .modifies(),
        c(
            "plot.horizontal_bar",
            "条形图",
            "bar",
            "",
            "绘图",
            "horizontal bar 条形",
            need_data,
            |a| a.plot(PlotType::HorizontalBar),
        )
        .modifies(),
        c(
            "plot.area",
            "面积图",
            "line",
            "",
            "绘图",
            "area fill 面积",
            need_data,
            |a| a.plot(PlotType::Area),
        )
        .modifies(),
        c(
            "plot.spline",
            "样条曲线图",
            "line",
            "",
            "绘图",
            "spline cubic 样条",
            need_data,
            |a| a.plot(PlotType::Spline),
        )
        .modifies(),
        c(
            "plot.spline_symbol",
            "样条点线图",
            "linesym",
            "",
            "绘图",
            "spline symbol 样条点线",
            need_data,
            |a| a.plot(PlotType::SplineSymbol),
        )
        .modifies(),
        c(
            "plot.vertical_drop",
            "垂线图",
            "bar",
            "",
            "绘图",
            "vertical drop stem 垂线",
            need_data,
            |a| a.plot(PlotType::VerticalDrop),
        )
        .modifies(),
        c(
            "graph.labels",
            "数据标签",
            "props",
            "",
            "图形",
            "data labels 数据标签",
            need_graph,
            |a| a.toggle_graph(|g| &mut g.labels.visible, "数据标签"),
        )
        .modifies()
        .checked(|a| graph_flag(a, |g| g.labels.visible)),
        // 图形
        c(
            "plot.step_horizontal",
            "水平阶梯图",
            "steph",
            "",
            "绘图",
            "step horizontal staircase",
            need_data,
            |a| a.plot(PlotType::StepHorizontal),
        )
        .modifies(),
        c(
            "plot.step_vertical",
            "垂直阶梯图",
            "stepv",
            "",
            "绘图",
            "step vertical staircase",
            need_data,
            |a| a.plot(PlotType::StepVertical),
        )
        .modifies(),
        c(
            "layer.add",
            "添加图层",
            "newbook",
            "",
            "图形",
            "layer panel subplot 添加图层",
            need_graph,
            |a| a.add_layer(),
        )
        .modifies(),
        c(
            "layer.remove",
            "删除图层",
            "coldel",
            "",
            "图形",
            "layer remove 删除图层",
            need_multi_layer,
            |a| a.remove_layer(),
        )
        .modifies(),
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
        .modifies()
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
        .modifies()
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
        .modifies()
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
        .modifies()
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
        )
        .modifies(),
        c(
            "col.setY",
            "设为 Y",
            "sety",
            "",
            "列",
            "set y column",
            need_cols,
            |a| a.set_role(Role::Y),
        )
        .modifies(),
        c(
            "col.setIgnore",
            "设为忽略",
            "ignore",
            "",
            "列",
            "disregard",
            need_cols,
            |a| a.set_role(Role::Ignore),
        )
        .modifies(),
        c(
            "col.setXError",
            "设为 X 误差",
            "setx",
            "",
            "列",
            "x error",
            need_cols,
            |a| a.set_role(Role::XError),
        )
        .modifies(),
        c(
            "col.setYError",
            "设为 Y 误差",
            "sety",
            "",
            "列",
            "y error",
            need_cols,
            |a| a.set_role(Role::YError),
        )
        .modifies(),
        c(
            "row.insert",
            "插入行",
            "rowins",
            "",
            "行",
            "insert row blank 插入行 空行",
            need_range,
            |a| a.insert_rows(),
        )
        .modifies(),
        c(
            "row.delete",
            "删除行",
            "rowdel",
            "",
            "行",
            "delete remove row 删除行",
            need_range,
            |a| a.delete_rows(),
        )
        .modifies(),
        c(
            "col.insert",
            "插入列",
            "colins",
            "",
            "列",
            "add column",
            need_book,
            |a| a.add_column(),
        )
        .modifies(),
        c(
            "col.delete",
            "删除列",
            "coldel",
            "",
            "列",
            "delete column",
            need_cols,
            |a| a.delete_columns(),
        )
        .modifies(),
        c(
            "col.clear",
            "清空列",
            "colclear",
            "",
            "列",
            "clear column",
            need_cols,
            |a| a.clear_columns(),
        )
        .modifies(),
        c(
            "sort.asc",
            "升序排序",
            "sortasc",
            "",
            "排序",
            "sort ascending",
            need_sortable,
            |a| a.sort_active(true),
        )
        .modifies(),
        c(
            "sort.desc",
            "降序排序",
            "sortdesc",
            "",
            "排序",
            "sort descending",
            need_sortable,
            |a| a.sort_active(false),
        )
        .modifies(),
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
        )
        .modifies(),
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
            "fit.nonlinear",
            "非线性拟合",
            "fitpoly",
            "",
            "分析",
            "nonlinear fit curve fitting levenberg marquardt gauss lorentz exponential boltzmann logistic power 非线性拟合 峰 指数 S形",
            need_data,
            |a| a.open_nlfit_dialog(),
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
            "signal.peaks",
            "寻峰",
            "integ",
            "",
            "分析",
            "peak find peaks prominence fwhm width area integrate centroid 寻峰 峰 突出度 半峰宽 峰面积 积分 质心",
            need_data,
            |a| a.open_peaks_dialog(),
        ),
        c(
            "signal.fft",
            "FFT",
            "diff",
            "",
            "分析",
            "fft fourier transform spectrum frequency amplitude phase power window hann 傅里叶 频谱 幅值 相位 功率",
            need_data,
            |a| a.open_fft_dialog(),
        ),
        c(
            "math.colmath",
            "列运算",
            "colins",
            "",
            "分析",
            "column arithmetic add subtract multiply divide curve subtract 列运算 加减乘除 曲线相减",
            need_book,
            |a| a.open_column_math_dialog(),
        ),
        c(
            "math.interp",
            "插值",
            "smooth",
            "",
            "分析",
            "interpolate interpolation spline akima linear extrapolate 插值 外推 样条",
            need_data,
            |a| a.open_interpolate_dialog(),
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
        )
        .modifies(),
        c(
            "math.integ",
            "积分",
            "integ",
            "",
            "分析",
            "integrate area",
            need_data,
            |a| a.integrate(),
        )
        .modifies(),
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
            "stats.frequency",
            "频数统计",
            "stats",
            "",
            "统计",
            "frequency count histogram bin distribution 频数 分箱 直方图 分布",
            need_data,
            |a| a.open_frequency_dialog(),
        ),
        c(
            "stats.anova",
            "单因素方差分析",
            "stats",
            "",
            "统计",
            "anova one way analysis of variance f test 方差分析 单因素 F 检验",
            need_book,
            |a| a.anova(),
        ),
        c(
            "stats.correlation",
            "相关系数",
            "stats",
            "",
            "统计",
            "correlation pearson spearman kendall rank 相关 相关系数 秩相关",
            need_book,
            |a| a.correlations(),
        ),
        c(
            "stats.ttest",
            "t 检验",
            "stats",
            "",
            "统计",
            "t test ttest student welch paired one sample hypothesis 假设检验 显著性 p 值",
            need_book,
            |a| a.open_ttest_dialog(),
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
        )
        .modifies(),
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
