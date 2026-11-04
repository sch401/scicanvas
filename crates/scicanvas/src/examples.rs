//! 内置图型说明和示例，直接复用 samples 中的项目。

use crate::app::SciApp;
use scicanvas_core::PlotType;

pub struct Example {
    pub kind: PlotType,
    pub plot_command: &'static str,
    pub command: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub bytes: &'static [u8],
    pub run: fn(&mut SciApp),
}

macro_rules! example {
    ($id:literal, $kind:ident, $file:literal, $description:literal) => {
        Example {
            kind: PlotType::$kind,
            plot_command: concat!("plot.", $id),
            command: concat!("example.", $id),
            label: concat!("打开", $file, "示例"),
            description: $description,
            bytes: include_bytes!(concat!("../../../samples/", $file, ".scv")),
            run: |app| app.open_plot_example(PlotType::$kind),
        }
    };
}

pub const ALL: &[Example] = &[
    example!(
        "line",
        Line,
        "01-折线图",
        "观察连续数据的变化趋势。一个 X 列后接一个或多个 Y 列，每个 Y 列画成一条线。"
    ),
    example!(
        "scatter",
        Scatter,
        "02-散点图",
        "观察两个变量的关系和离散程度。使用 X、Y 列，每行数据对应一个点。"
    ),
    example!(
        "linesym",
        LineSymbol,
        "03-点线图",
        "同时显示实测点和变化趋势。使用 X、Y 列，Y 右侧可放 Y 误差列，示例展示自动关联的误差棒。"
    ),
    example!(
        "bar",
        Bar,
        "04-柱状图",
        "比较不同类别的数值。X 列用 1、2、3 等数字编号，多个 Y 列在同一类别处并排显示。"
    ),
    example!(
        "stacked_bar",
        StackedBar,
        "05-堆叠柱图",
        "比较总量及各部分组成。一个 X 列加多个 Y 列，各 Y 列按顺序堆叠，正负值分别累计。"
    ),
    example!(
        "percent_stacked_bar",
        PercentStackedBar,
        "06-百分比堆叠柱图",
        "比较各部分所占比例。填入 X 和多个 Y 列的原始数值，正负值分别归一化为百分比。"
    ),
    example!(
        "floating_bar",
        FloatingBar,
        "07-浮动柱图",
        "展示起点到终点的区间。X 列后，Y 列按“下端、上端”两两配对，例如最低值和最高值。"
    ),
    example!(
        "horizontal_bar",
        HorizontalBar,
        "08-条形图",
        "用横条比较数值。数据排列与柱状图相同，X 类别编号显示在纵轴，Y 决定横条长度。"
    ),
    example!(
        "area",
        Area,
        "09-面积图",
        "突出曲线与基线之间的面积。使用 X、Y 列，填充到 0，负值显示在基线下方。"
    ),
    example!(
        "step_horizontal",
        StepHorizontal,
        "10-水平阶梯图",
        "展示分段保持的数据。使用 X、Y 列，先水平保持，再在下一个 X 处变化。"
    ),
    example!(
        "step_vertical",
        StepVertical,
        "11-垂直阶梯图",
        "展示先变化后保持的数据。使用 X、Y 列，在当前 X 处先竖直变化，再水平延伸。"
    ),
    example!(
        "spline",
        Spline,
        "12-样条曲线图",
        "用自然三次样条连接数据点。使用 X、Y 列，适合查看平滑趋势，点间可能出现过冲。"
    ),
    example!(
        "spline_symbol",
        SplineSymbol,
        "13-样条点线图",
        "同时显示平滑样条和原始测量点。数据排列与样条曲线相同，符号标明实际采样位置。"
    ),
    example!(
        "vertical_drop",
        VerticalDrop,
        "14-垂线图",
        "展示谱峰或离散信号。使用 X、Y 列，从 0 到每个数据点画一条垂线。"
    ),
    example!(
        "histogram",
        Histogram,
        "15-直方图",
        "观察一组测量值的分布。每个 Y 列是一组测量值，不需要 X 列；按等宽区间计数后画成相连的柱子。"
    ),
];
