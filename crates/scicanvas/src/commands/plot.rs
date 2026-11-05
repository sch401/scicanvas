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

pub(super) fn templates(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "graph.template.batch",
            "批量绘图…",
            "grid",
            "",
            "绘图",
            "batch plot 多工作簿 批量绘图",
            always,
            |a| a.open_batch_plot(),
        ),
        c(
            "graph.template.save",
            "保存为模板",
            "save",
            "",
            "绘图",
            "graph template save style layout 模板 保存 样式",
            need_any_graph,
            |a| a.save_graph_template(),
        ),
        c(
            "graph.template.apply",
            "应用模板",
            "open",
            "",
            "绘图",
            "graph template apply workbook columns 应用 模板 工作簿",
            need_book,
            |a| a.apply_graph_template_dialog(),
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
    scatter_variants(commands);
    offset_series(commands);
    radial_plots(commands);
    graphics_tools(commands);
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
            "plot.histogram",
            "直方图",
            "bar",
            "",
            "绘图",
            "histogram distribution frequency 直方图 分布 频数",
            need_data,
            |a| a.plot(PlotType::Histogram),
        )
        .modifies(),
        c(
            "plot.qq",
            "Q-Q 图",
            "scatter",
            "",
            "绘图",
            "qq normal probability 正态 分位数 Q-Q 图",
            need_data,
            |a| a.plot(PlotType::QQ),
        )
        .modifies(),
        c(
            "plot.box",
            "箱线图",
            "bar",
            "",
            "绘图",
            "box plot whisker quartile 箱线图 四分位数 离群点",
            need_data,
            |a| a.plot(PlotType::Box),
        )
        .modifies(),
        c(
            "plot.stacked_horizontal_bar",
            "堆叠条形图",
            "area",
            "",
            "绘图",
            "堆叠条形图",
            need_data,
            |a| a.plot(PlotType::StackedHorizontalBar),
        )
        .modifies(),
        c(
            "plot.percent_stacked_horizontal_bar",
            "百分比堆叠条形图",
            "area",
            "",
            "绘图",
            "百分比堆叠条形图",
            need_data,
            |a| a.plot(PlotType::PercentStackedHorizontalBar),
        )
        .modifies(),
        c(
            "plot.stacked_area",
            "堆叠面积图",
            "area",
            "",
            "绘图",
            "堆叠面积图",
            need_data,
            |a| a.plot(PlotType::StackedArea),
        )
        .modifies(),
        c(
            "plot.fill_between",
            "两曲线间填充",
            "area",
            "",
            "绘图",
            "两曲线间填充",
            need_data,
            |a| a.plot(PlotType::FillBetween),
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
            "graph.reference.vertical",
            "添加垂直参考线",
            "stepv",
            "",
            "图形",
            "reference line vertical x value 参考线 垂直 X 轴值",
            need_graph,
            |a| a.add_reference_line(scicanvas_core::page::ReferenceAxis::X),
        )
        .modifies(),
        c(
            "graph.reference.horizontal",
            "添加水平参考线",
            "fitlin",
            "",
            "图形",
            "reference line horizontal y value 参考线 水平 Y 轴值",
            need_graph,
            |a| a.add_reference_line(scicanvas_core::page::ReferenceAxis::YLeft),
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
            |a| a.toggle_axis_scale(true),
        )
        .modifies()
        .checked(|a| graph_flag(a, |g| g.effective_x_scale().is_log())),
        c(
            "graph.logy",
            "Y 对数",
            "logy",
            "",
            "图形",
            "log scale y",
            need_graph,
            |a| a.toggle_axis_scale(false),
        )
        .modifies()
        .checked(|a| graph_flag(a, |g| g.effective_y_scale().is_log())),
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

fn scatter_variants(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "plot.error_band",
            "误差带图",
            "scatter",
            "",
            "绘图",
            "误差带图",
            need_data,
            |a| a.plot(PlotType::ErrorBand),
        )
        .modifies(),
        c(
            "plot.column_scatter",
            "列散点图",
            "scatter",
            "",
            "绘图",
            "列散点图",
            need_data,
            |a| a.plot(PlotType::ColumnScatter),
        )
        .modifies(),
        c(
            "plot.bubble",
            "气泡图",
            "scatter",
            "",
            "绘图",
            "气泡图",
            need_data,
            |a| a.plot(PlotType::Bubble),
        )
        .modifies(),
        c(
            "plot.color_scatter",
            "颜色映射散点图",
            "scatter",
            "",
            "绘图",
            "颜色映射散点图",
            need_data,
            |a| a.plot(PlotType::ColorScatter),
        )
        .modifies(),
        c(
            "plot.bubble_color",
            "气泡颜色映射图",
            "scatter",
            "",
            "绘图",
            "气泡颜色映射图",
            need_data,
            |a| a.plot(PlotType::BubbleColor),
        )
        .modifies(),
    ]);
}

fn offset_series(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "plot.offset_line",
            "偏移堆叠线图",
            "area",
            "",
            "绘图",
            "偏移堆叠线图",
            need_data,
            |a| a.plot(PlotType::OffsetLine),
        )
        .modifies(),
        c(
            "plot.waterfall",
            "二维瀑布图",
            "area",
            "",
            "绘图",
            "二维瀑布图",
            need_data,
            |a| a.plot(PlotType::Waterfall),
        )
        .modifies(),
        c(
            "plot.line_series",
            "线系列图",
            "area",
            "",
            "绘图",
            "线系列图",
            need_data,
            |a| a.plot(PlotType::LineSeries),
        )
        .modifies(),
    ]);
}

fn radial_plots(commands: &mut Vec<Command>) {
    commands.push(
        c(
            "plot.pie",
            "饼图",
            "scatter",
            "",
            "绘图",
            "饼图",
            need_data,
            |a| a.plot(PlotType::Pie),
        )
        .modifies(),
    );
    commands.push(
        c(
            "plot.violin",
            "小提琴图",
            "scatter",
            "",
            "绘图",
            "小提琴图",
            need_data,
            |a| a.plot(PlotType::Violin),
        )
        .modifies(),
    );
    commands.push(
        c(
            "plot.polar_line",
            "极坐标线图",
            "scatter",
            "",
            "绘图",
            "极坐标线图",
            need_data,
            |a| a.plot(PlotType::PolarLine),
        )
        .modifies(),
    );
    commands.push(
        c(
            "plot.polar_scatter",
            "极坐标散点图",
            "scatter",
            "",
            "绘图",
            "极坐标散点图",
            need_data,
            |a| a.plot(PlotType::PolarScatter),
        )
        .modifies(),
    );
}

fn graphics_tools(commands: &mut Vec<Command>) {
    shape_tools(commands);
    commands.extend([
        c(
            "plot.function",
            "函数图…",
            "line",
            "",
            "绘图",
            "function expression 函数",
            |_| Ok(()),
            |a| a.open_function_plot(),
        ),
        c(
            "graph.themes",
            "图形主题与选项…",
            "graph",
            "",
            "图形",
            "theme Nature import export 新图使用主题",
            |_| Ok(()),
            |a| a.open_graph_themes(),
        ),
        c(
            "graph.style.copy",
            "复制格式",
            "copy",
            "Ctrl+Shift+C",
            "图形",
            "copy format style",
            need_graph,
            |a| a.copy_style(),
        ),
        c(
            "graph.style.paste",
            "粘贴格式",
            "paste",
            "Ctrl+Shift+V",
            "图形",
            "paste format style",
            need_graph,
            |a| a.paste_style(),
        )
        .modifies(),
    ]);
}

fn shape_tools(commands: &mut Vec<Command>) {
    commands.push(c(
        "graph.shape.line",
        "绘制直线",
        "area",
        "",
        "图形",
        "draw shape line",
        need_graph,
        |a| a.start_shape_tool(scicanvas_core::page::ShapeKind::Line),
    ));
    commands.push(c(
        "graph.shape.arrow",
        "绘制箭头",
        "area",
        "",
        "图形",
        "draw shape arrow",
        need_graph,
        |a| a.start_shape_tool(scicanvas_core::page::ShapeKind::Arrow),
    ));
    commands.push(c(
        "graph.shape.rect",
        "绘制矩形",
        "area",
        "",
        "图形",
        "draw shape rect",
        need_graph,
        |a| a.start_shape_tool(scicanvas_core::page::ShapeKind::Rect),
    ));
    commands.push(c(
        "graph.shape.ellipse",
        "绘制椭圆",
        "area",
        "",
        "图形",
        "draw shape ellipse",
        need_graph,
        |a| a.start_shape_tool(scicanvas_core::page::ShapeKind::Ellipse),
    ));
}
