use super::*;

pub(super) fn layer_contents(commands: &mut Vec<Command>) {
    commands.extend([c(
        "layer.contents",
        "图层内容",
        "props",
        "",
        "图形",
        "layer contents 添加 移除 排序 左右轴 曲线 数据源",
        need_any_graph,
        |a| a.open_layer_contents(),
    )]);
}

pub(super) fn annotations(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "graph.text.add",
            "添加文字标注",
            "note",
            "",
            "图形",
            "text annotation label 文字 标注",
            need_any_graph,
            |a| a.start_text_annotation(),
        ),
        c(
            "graph.text.manage",
            "管理文字标注",
            "props",
            "",
            "图形",
            "text annotation edit 文字 编辑",
            need_any_graph,
            |a| a.manage_annotations(),
        ),
        c(
            "graph.legend.auto",
            "图例自动避让",
            "legend",
            "",
            "图形",
            "legend automatic position reset 图例 位置 自动",
            need_any_graph,
            |a| a.auto_legend(),
        )
        .modifies(),
    ]);
}

pub(super) fn builder(commands: &mut Vec<Command>) {
    commands.extend([
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
    ]);
}

pub(super) fn basic(commands: &mut Vec<Command>) {
    commands.extend([
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
    ]);
}

pub(super) fn decorations(commands: &mut Vec<Command>) {
    commands.extend([
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
    ]);
}

pub(super) fn graph(commands: &mut Vec<Command>) {
    commands.extend([
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
    ]);
}

pub(super) fn examples(commands: &mut Vec<Command>) {
    commands.extend(crate::examples::ALL.iter().map(|example| {
        c(
            example.command,
            example.label,
            "book",
            "",
            "图型示例",
            "example 示例 用法",
            always,
            example.run,
        )
        .modifies()
    }));
}
