//! 数据模型：项目、工作簿、列、图形。

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const PROJECT_VERSION: u32 = 36;
pub const GRAPH_TEMPLATE_VERSION: u32 = 1;

/// 列的绘图角色，对应 Origin 里的 X / Y / 忽略。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Role {
    X,
    Y,
    XError,
    YError,
    Ignore,
    /// Z 值（三维数据、等高线和热图的第三维）；二维作图和分析不把它当 Y。
    Z,
    /// 标签列：数据标签选“指定列文本”而未指定列时使用。
    Label,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::X => "X",
            Role::Y => "Y",
            Role::XError => "X Error",
            Role::YError => "Y Error",
            Role::Ignore => "—",
            Role::Z => "Z",
            Role::Label => "L",
        }
    }
}

/// 列移动的目标位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnMove {
    Left,
    Right,
    First,
    Last,
}

/// 工作表中的一列。类型化存储，稀疏文本保留用户输入的原样。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Column {
    /// 列标识：A、B、C……，创建后不再变化，图形通过它引用列。
    pub key: String,
    pub role: Role,
    pub long_name: String,
    pub units: String,
    pub comment: String,
    pub data: crate::SharedData,
    /// 独立采样网格不参与来源表的整行操作。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "std::ops::Not::not"))]
    pub independent_rows: bool,
    /// 由哪一列计算得来（拟合、平滑等结果列），新的结果列会排在同一来源的结果列后面。
    #[cfg_attr(feature = "serde", serde(default))]
    pub derived_from: Option<String>,
    /// 误差列关联的 X/Y 列；空字符串表示来源已删除，None 按列顺序自动关联。
    #[cfg_attr(feature = "serde", serde(default))]
    pub error_for: Option<String>,
    /// 保存的列公式：引用的列变化时自动重算（见 `Project::recalc_computed`）。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub formula: Option<String>,
    /// 生成这一列的分析操作（平滑、微分等），按其重算方式随来源更新。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub op: Option<crate::ops::Operation>,
    /// 结果列拥有的数值报告，来源删除后仍保留上次结果。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub fit_report: Option<Box<crate::ops::NlFitReport>>,
    /// 显示格式：只影响工作表里的显示。全部为“原样”时不写入文件。
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "crate::format::DisplayFormat::is_auto")
    )]
    pub format: crate::format::DisplayFormat,
}

impl Column {
    pub fn new(key: impl Into<String>, role: Role) -> Self {
        Column {
            key: key.into(),
            role,
            long_name: String::new(),
            units: String::new(),
            comment: String::new(),
            data: Default::default(),
            independent_rows: false,
            derived_from: None,
            error_for: None,
            formula: None,
            op: None,
            fit_report: None,
            format: Default::default(),
        }
    }

    /// 工作表里显示的文字：按显示格式写数值和日期；缺失、文本、分类和“原样”格式显示原始写法。
    pub fn display_at(&self, row: usize) -> std::borrow::Cow<'_, str> {
        use crate::ColumnData;
        let formatted = match &*self.data {
            ColumnData::Num { .. } => self.data.f64_at(row).and_then(|v| self.format.number.apply(v)),
            ColumnData::DateTime { .. } => self
                .data
                .f64_at(row)
                .and_then(|v| self.format.date.apply_in(v as i64, self.format.utc_offset)),
            _ => None,
        };
        formatted.map_or_else(|| self.data.text_at(row), std::borrow::Cow::Owned)
    }

    /// 解析为数值；空单元格或非数值为 NaN。
    pub fn values(&self) -> Vec<f64> {
        (0..self.data.len())
            .map(|row| self.data.f64_at(row).unwrap_or(f64::NAN))
            .collect()
    }

    /// 图例里用的名字：有长名称用长名称，否则用列标识。
    pub fn display_name(&self) -> &str {
        if self.long_name.trim().is_empty() {
            &self.key
        } else {
            self.long_name.trim()
        }
    }

    /// 坐标轴标题：长名称 (单位)。
    /// 两列构成的范围（如浮动柱的下端到上端）：单位相同时只写一次。
    pub fn range_label(&self, top: &Column) -> String {
        let (a, b) = (self.units.trim(), top.units.trim());
        if a == b {
            let names = format!("{} – {}", self.display_name(), top.display_name());
            if a.is_empty() {
                names
            } else {
                format!("{names} ({a})")
            }
        } else {
            format!("{} – {}", self.axis_label(), top.axis_label())
        }
    }

    pub fn axis_label(&self) -> String {
        let n = self.display_name();
        if self.units.trim().is_empty() {
            n.to_string()
        } else {
            format!("{} ({})", n, self.units.trim())
        }
    }

    /// 写入一个单元格；写空值时会去掉末尾多余的空行。
    pub fn set(&mut self, row: usize, value: String) {
        if row >= self.data.len() && value.trim().is_empty() {
            return;
        }
        self.data.set_text(row, value);
        if self.format.utc_offset != 0 {
            self.data.reinterpret_local(row, self.format.utc_offset);
        }
        while self.data.last().is_some_and(|s| s.trim().is_empty()) {
            self.data.pop();
        }
    }

    /// 设置日期列的时区：不带时区写的日期保持“墙上时间”（按新时区重新换算成 UTC），带时区的保持原时刻。
    pub fn set_utc_offset(&mut self, minutes: i32) {
        self.format.utc_offset = minutes;
        for row in 0..self.data.len() {
            self.data.reinterpret_local(row, minutes);
        }
    }
}

impl crate::ColumnView for Column {
    fn len(&self) -> usize {
        self.data.len()
    }
    fn f64_at(&self, row: usize) -> Option<f64> {
        self.data.f64_at(row)
    }
    fn text_at(&self, row: usize) -> std::borrow::Cow<'_, str> {
        self.data.text_at(row)
    }
}

/// 解析一个单元格。允许千分位逗号。
pub fn parse_num(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    let cleaned: String = t.chars().filter(|c| *c != ',').collect();
    cleaned.parse::<f64>().unwrap_or(f64::NAN)
}

pub fn looks_numeric(s: &str) -> bool {
    !s.trim().is_empty() && parse_num(s).is_finite()
}

/// 按有效数字取整，消除 0.30000000000000004 这类浮点尾巴。
pub fn round_sig(x: f64, digits: i32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let mag = x.abs().log10().ceil() as i32;
    let p = 10f64.powi(digits - mag);
    (x * p).round() / p
}

/// 工作表里显示数值的格式。
pub fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return String::new();
    }
    let a = v.abs();
    if a != 0.0 && !(1e-4..1e6).contains(&a) {
        return format!("{:.4e}", v);
    }
    format!("{}", round_sig(v, 8))
}

/// 列标识：0 → A，25 → Z，26 → AA。
pub fn column_letter(mut i: usize) -> String {
    let mut s = Vec::new();
    i += 1;
    while i > 0 {
        let m = (i - 1) % 26;
        s.push((b'A' + m as u8) as char);
        i = (i - 1) / 26;
    }
    s.iter().rev().collect()
}

/// 工作簿（Origin 的 Book），这一版每个工作簿只有一张工作表。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Book {
    pub id: u64,
    pub name: String,
    pub columns: Vec<Column>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub filter: Option<crate::filter::Filter>,
}

impl Book {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Book {
            id,
            name: name.into(),
            columns: Vec::new(),
            filter: None,
        }
    }

    pub fn nrows(&self) -> usize {
        self.columns.iter().map(|c| c.data.len()).max().unwrap_or(0)
    }

    pub fn convert_columns(&mut self, keys: &[String], kind: crate::DataKind) -> Result<(), String> {
        let converted = keys
            .iter()
            .map(|key| {
                let i = self.column_index(key).ok_or_else(|| format!("找不到列 {key}"))?;
                Ok((i, self.columns[i].data.converted(kind)?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        for (i, data) in converted {
            self.columns[i].data = data.into();
        }
        Ok(())
    }

    /// 下一个没用过的列标识。
    pub fn next_key(&self) -> String {
        (0..)
            .map(column_letter)
            .find(|k| !self.columns.iter().any(|c| &c.key == k))
            .unwrap()
    }

    pub fn column(&self, key: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.key == key)
    }

    pub fn column_index(&self, key: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.key == key)
    }

    pub fn error_source(&self, index: usize) -> Option<&str> {
        let col = &self.columns[index];
        if !matches!(col.role, Role::XError | Role::YError) {
            return None;
        }
        if let Some(key) = col.error_for.as_deref() {
            return self
                .column(key)
                .filter(|c| c.role == Role::Y || (col.role == Role::XError && c.role == Role::X))
                .map(|c| c.key.as_str());
        }
        self.columns[..index]
            .iter()
            .rev()
            .find(|c| c.derived_from.is_none() && !matches!(c.role, Role::XError | Role::YError))
            .filter(|c| c.role == Role::Y || (col.role == Role::XError && c.role == Role::X))
            .map(|c| c.key.as_str())
    }

    pub fn set_roles(&mut self, keys: &[String], role: Role) {
        self.freeze_error_sources();
        for c in &mut self.columns {
            if keys.contains(&c.key) {
                c.role = role;
                c.error_for = None;
            }
        }
        self.freeze_error_sources();
    }

    pub(crate) fn freeze_error_sources(&mut self) {
        let sources: Vec<_> = (0..self.columns.len())
            .map(|i| self.error_source(i).map(str::to_string))
            .collect();
        for (col, source) in self.columns.iter_mut().zip(sources) {
            if matches!(col.role, Role::XError | Role::YError) && col.error_for.is_none() {
                col.error_for = source;
            }
        }
    }

    /// 在 index 处插入新列，返回新列标识。
    pub fn insert_column(&mut self, index: usize, role: Role) -> String {
        self.freeze_error_sources();
        let key = self.next_key();
        let index = index.min(self.columns.len());
        self.columns.insert(index, Column::new(key.clone(), role));
        key
    }

    /// 移动选中的列（左移、右移一格，或移到最前、最后），选中列之间的相对顺序不变。
    /// 误差列先固定所属数据列，移动后仍关联原来的列。返回列顺序是否改变。
    pub fn move_columns(&mut self, keys: &[String], to: ColumnMove) -> bool {
        let picked = |c: &Column| keys.contains(&c.key);
        if !self.columns.iter().any(picked) {
            return false;
        }
        let before: Vec<String> = self.columns.iter().map(|c| c.key.clone()).collect();
        self.freeze_error_sources();
        let n = self.columns.len();
        match to {
            ColumnMove::Left => {
                for i in 1..n {
                    if picked(&self.columns[i]) && !picked(&self.columns[i - 1]) {
                        self.columns.swap(i, i - 1);
                    }
                }
            }
            ColumnMove::Right => {
                for i in (0..n.saturating_sub(1)).rev() {
                    if picked(&self.columns[i]) && !picked(&self.columns[i + 1]) {
                        self.columns.swap(i, i + 1);
                    }
                }
            }
            ColumnMove::First | ColumnMove::Last => {
                let (mut sel, mut rest): (Vec<Column>, Vec<Column>) =
                    std::mem::take(&mut self.columns).into_iter().partition(|c| picked(c));
                if to == ColumnMove::First {
                    sel.append(&mut rest);
                    self.columns = sel;
                } else {
                    rest.append(&mut sel);
                    self.columns = rest;
                }
            }
        }
        self.columns.iter().map(|c| &c.key).ne(before.iter())
    }

    pub fn push_column(&mut self, role: Role) -> String {
        self.insert_column(self.columns.len(), role)
    }

    /// 保证至少有 n 列，不够时补 Y 列。
    pub fn ensure_columns(&mut self, n: usize) {
        while self.columns.len() < n {
            self.push_column(Role::Y);
        }
    }

    /// 单元格区域（列 c0..=c1，行 r0..=r1）的内容，制表符分隔，可直接粘贴到 Excel。
    pub fn range_tsv(&self, c0: usize, c1: usize, r0: usize, r1: usize) -> String {
        let c1 = c1.min(self.columns.len().saturating_sub(1));
        (r0..=r1)
            .map(|r| {
                (c0..=c1)
                    .map(|c| self.columns[c].data.text_at(r))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 清空单元格区域。
    pub fn clear_range(&mut self, c0: usize, c1: usize, r0: usize, r1: usize) {
        self.fill_range(c0, c1, r0, r1, "");
    }

    /// 用同一个值填满单元格区域。
    pub fn fill_range(&mut self, c0: usize, c1: usize, r0: usize, r1: usize, value: &str) {
        let c1 = c1.min(self.columns.len().saturating_sub(1));
        if self.columns.is_empty() {
            return;
        }
        for c in c0..=c1 {
            // 从下往上写，清空时末尾的空行能一次去掉
            for r in (r0..=r1).rev() {
                self.columns[c].set(r, value.to_string());
            }
        }
    }
}

/// 多面板图的版式：面板数、排版列数和页面尺寸（Nature 单栏 89 mm / 双栏 183 mm）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panels {
    /// 上下两面板
    Stacked,
    /// 左右两面板
    SideBySide,
    /// 2 × 2 四面板
    Grid4,
}

impl Panels {
    pub const ALL: [Panels; 3] = [Panels::Stacked, Panels::SideBySide, Panels::Grid4];

    pub fn label(self) -> &'static str {
        match self {
            Panels::Stacked => "上下两面板",
            Panels::SideBySide => "左右两面板",
            Panels::Grid4 => "四面板",
        }
    }
    pub fn count(self) -> usize {
        match self {
            Panels::Grid4 => 4,
            _ => 2,
        }
    }
    pub fn columns(self) -> usize {
        match self {
            Panels::Stacked => 1,
            _ => 2,
        }
    }
    pub fn page_mm(self) -> (f64, f64) {
        match self {
            // 页宽统一为 A4 纸宽；上下两面板每块 3:2，整页仍可放进 A4 竖版
            Panels::Stacked => (210.0, 280.0),
            Panels::SideBySide => (210.0, 105.0),
            Panels::Grid4 => (210.0, 157.5),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlotType {
    Line,
    Scatter,
    LineSymbol,
    Bar,
    StackedBar,
    PercentStackedBar,
    FloatingBar,
    HorizontalBar,
    Area,
    StepHorizontal,
    StepVertical,
    Spline,
    SplineSymbol,
    VerticalDrop,
    Histogram,
    QQ,
    Box,
    StackedHorizontalBar,
    PercentStackedHorizontalBar,
    StackedArea,
    FillBetween,
    ErrorBand,
    ColumnScatter,
    Bubble,
    ColorScatter,
    BubbleColor,
    OffsetLine,
    Waterfall,
    LineSeries,
    Pie,
    Violin,
    PolarLine,
    PolarScatter,
}

impl PlotType {
    pub const ALL: [PlotType; 33] = [
        PlotType::Line,
        PlotType::Scatter,
        PlotType::LineSymbol,
        PlotType::Bar,
        PlotType::StackedBar,
        PlotType::PercentStackedBar,
        PlotType::FloatingBar,
        PlotType::HorizontalBar,
        PlotType::Area,
        PlotType::StepHorizontal,
        PlotType::StepVertical,
        PlotType::Spline,
        PlotType::SplineSymbol,
        PlotType::VerticalDrop,
        PlotType::Histogram,
        PlotType::QQ,
        PlotType::Box,
        PlotType::StackedHorizontalBar,
        PlotType::PercentStackedHorizontalBar,
        PlotType::StackedArea,
        PlotType::FillBetween,
        PlotType::ErrorBand,
        PlotType::ColumnScatter,
        PlotType::Bubble,
        PlotType::ColorScatter,
        PlotType::BubbleColor,
        PlotType::OffsetLine,
        PlotType::Waterfall,
        PlotType::LineSeries,
        PlotType::Pie,
        PlotType::Violin,
        PlotType::PolarLine,
        PlotType::PolarScatter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PlotType::Line => "折线图",
            PlotType::Scatter => "散点图",
            PlotType::LineSymbol => "点线图",
            PlotType::Bar => "柱状图",
            PlotType::StackedBar => "堆叠柱图",
            PlotType::PercentStackedBar => "百分比堆叠柱图",
            PlotType::FloatingBar => "浮动柱图",
            PlotType::HorizontalBar => "条形图",
            PlotType::Area => "面积图",
            PlotType::StepHorizontal => "水平阶梯图",
            PlotType::StepVertical => "垂直阶梯图",
            PlotType::Spline => "样条曲线图",
            PlotType::SplineSymbol => "样条点线图",
            PlotType::VerticalDrop => "垂线图",
            PlotType::Histogram => "直方图",
            PlotType::QQ => "Q-Q 图",
            PlotType::Box => "箱线图",
            PlotType::StackedHorizontalBar => "堆叠条形图",
            PlotType::PercentStackedHorizontalBar => "百分比堆叠条形图",
            PlotType::StackedArea => "堆叠面积图",
            PlotType::FillBetween => "两曲线间填充",
            PlotType::ErrorBand => "误差带图",
            PlotType::ColumnScatter => "列散点图",
            PlotType::Bubble => "气泡图",
            PlotType::ColorScatter => "颜色映射散点图",
            PlotType::BubbleColor => "气泡颜色映射图",
            PlotType::OffsetLine => "偏移堆叠线图",
            PlotType::Waterfall => "二维瀑布图",
            PlotType::LineSeries => "线系列图",
            PlotType::Pie => "饼图",
            PlotType::Violin => "小提琴图",
            PlotType::PolarLine => "极坐标线图",
            PlotType::PolarScatter => "极坐标散点图",
        }
    }

    pub(crate) fn uses_y_samples(self) -> bool {
        matches!(
            self,
            Self::QQ | Self::Box | Self::Violin | Self::Histogram | Self::ColumnScatter | Self::LineSeries
        )
    }

    pub fn has_line(self) -> bool {
        crate::presets::recipe(self).has_line()
    }
    pub fn is_bar(self) -> bool {
        crate::presets::recipe(self).is_bar()
    }
    pub fn is_stacked(self) -> bool {
        crate::presets::recipe(self).position.is_stacked()
    }
    pub fn has_symbol(self) -> bool {
        crate::presets::recipe(self).has_symbol()
    }
    pub fn is_spline(self) -> bool {
        crate::presets::recipe(self).path == crate::mark::Path::Natural
    }
}

/// 默认的颜色递增顺序（Nature 风格的克制色系）：深蓝、砖红、青、紫、中灰、绿……
/// 相邻颜色同时在色相和明度上拉开，不只靠红绿区分。
pub const PALETTE: [(u8, u8, u8); 9] = [
    (15, 77, 146),   // #0F4D92 深蓝
    (182, 67, 66),   // #B64342 砖红
    (66, 148, 158),  // #42949E 青
    (154, 77, 142),  // #9A4D8E 紫
    (118, 118, 118), // #767676 中灰
    (46, 158, 68),   // #2E9E44 绿
    (39, 39, 39),    // #272727 近黑
    (55, 117, 186),  // #3775BA 中蓝
    (233, 166, 161), // #E9A6A1 浅砖红
];

pub fn palette_hex(i: usize) -> String {
    let (r, g, b) = PALETTE[i % PALETTE.len()];
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// 符号递增顺序：圆、方块、上三角、下三角、菱形……
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Symbol {
    Square,
    Circle,
    Up,
    Down,
    Diamond,
    Left,
    Right,
    Hexagon,
}

pub const SYMBOLS: [Symbol; 8] = [
    Symbol::Circle,
    Symbol::Square,
    Symbol::Up,
    Symbol::Down,
    Symbol::Diamond,
    Symbol::Left,
    Symbol::Right,
    Symbol::Hexagon,
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointStyle {
    pub color: Option<[u8; 3]>,
    pub size: Option<f32>,
    pub symbol: Option<Symbol>,
}

/// 一条曲线的数据。
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub key: String,
    pub name: String,
    pub points: Vec<[f64; 2]>,
    pub point_styles: Vec<PointStyle>,
    /// 与 points 逐点对齐；无效或缺失的误差为 None。
    pub errors: Vec<[Option<f64>; 2]>,
    /// 原始行用于堆叠对齐；均值变换后按 X 对齐。
    pub rows: Vec<usize>,
    /// 柱段起点；points 的 Y 为终点。
    pub bases: Vec<f64>,
    /// 只画线（拟合曲线、平滑结果），不跟随图类型。
    pub line_only: bool,
    /// 结果曲线的来源列（用于避免拟合线和原数据同色）。
    pub source: Option<String>,
}

/// 拟合 / 平滑曲线的颜色：默认砖红；如果它的原始数据本身是砖红，改用深蓝。
pub const FIT_RGB: (u8, u8, u8) = (182, 67, 66);
pub const FIT_ALT_RGB: (u8, u8, u8) = (15, 77, 146);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesColor {
    /// 默认颜色顺序里的第 i 个。
    Palette(usize),
    Fit,
    FitAlt,
    Rgb((u8, u8, u8)),
}

impl SeriesColor {
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            SeriesColor::Palette(i) => PALETTE[i % PALETTE.len()],
            SeriesColor::Fit => FIT_RGB,
            SeriesColor::FitAlt => FIT_ALT_RGB,
            SeriesColor::Rgb(color) => color,
        }
    }

    pub fn hex(self) -> String {
        let (r, g, b) = self.rgb();
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }
}

/// 每条曲线的画法。SVG 导出和界面共用，保证两边一致。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeriesStyle {
    /// 结果曲线固定为 Line，其余跟随图类型（可被逐曲线设置覆盖）。
    pub kind: PlotType,
    pub color: SeriesColor,
    pub symbol: Symbol,
    /// 柱状图里这条曲线是第几组、一共几组。
    pub bar_index: usize,
    pub bar_count: usize,
    pub line_width: f32,
    pub symbol_size: f32,
    /// 画在右侧 Y 轴上。
    pub y_axis: crate::coord::YAxis,
}

/// 颜色和符号只在数据曲线之间递增，结果曲线不占顺序。
pub fn series_styles(series: &[Series], plot_type: PlotType) -> Vec<SeriesStyle> {
    let bar_count = if plot_type.is_bar() {
        series.iter().filter(|s| !s.line_only).count()
    } else {
        0
    };
    let mut data_index = 0;
    let mut by_key = std::collections::HashMap::new();
    let mut out: Vec<SeriesStyle> = series
        .iter()
        .map(|s| {
            if s.line_only {
                SeriesStyle {
                    kind: PlotType::Line,
                    color: SeriesColor::Fit,
                    symbol: Symbol::Square,
                    bar_index: 0,
                    bar_count,
                    line_width: 1.0,
                    symbol_size: 4.5,
                    y_axis: crate::coord::YAxis::Left,
                }
            } else {
                let i = data_index;
                data_index += 1;
                by_key.insert(s.key.clone(), i);
                SeriesStyle {
                    kind: plot_type,
                    color: SeriesColor::Palette(i),
                    symbol: SYMBOLS[i % SYMBOLS.len()],
                    bar_index: i,
                    bar_count,
                    line_width: 1.0,
                    symbol_size: 4.5,
                    y_axis: crate::coord::YAxis::Left,
                }
            }
        })
        .collect();
    for (st, s) in out.iter_mut().zip(series) {
        if s.line_only {
            let src = s.source.as_ref().and_then(|k| by_key.get(k));
            if let Some(&i) = src {
                if PALETTE[i % PALETTE.len()] == FIT_RGB {
                    st.color = SeriesColor::FitAlt;
                }
            }
        }
    }
    out
}

/// 图形窗口持有页面；调用方显式选择图层。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Graph {
    pub id: u64,
    pub name: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub page: crate::page::Page,
}
impl Graph {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            page: Default::default(),
        }
    }
}
impl Graph {
    pub fn first_layer(&self) -> &crate::page::Layer {
        &self.page.layers()[0]
    }
    pub fn first_layer_mut(&mut self) -> &mut crate::page::Layer {
        &mut self.page.layers_mut()[0]
    }
}

/// 不含工作簿数据或编号、按列标识重绑定的图形页面模板。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(deny_unknown_fields))]
pub struct GraphTemplate {
    pub version: u32,
    pub name: String,
    pub page: crate::page::Page,
}

/// 随项目保存的文本笔记或计算报告。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Note {
    pub id: u64,
    pub name: String,
    pub text: String,
}

/// 整个项目。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Project {
    /// 文件格式版本；无版本号的旧文件按第 0 版迁移。
    #[cfg_attr(feature = "serde", serde(default))]
    pub version: u32,
    pub books: Vec<Book>,
    pub graphs: Vec<Graph>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub notes: Vec<Note>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub fit_functions: Vec<crate::nlfit::SavedFitFunction>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub graph_themes: Vec<crate::graph_theme::GraphTheme>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub folders: Vec<crate::project_tree::Folder>,
    pub next_id: u64,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            version: PROJECT_VERSION,
            books: Vec::new(),
            graphs: Vec::new(),
            notes: Vec::new(),
            fit_functions: Vec::new(),
            graph_themes: Vec::new(),
            folders: Vec::new(),
            next_id: 1,
        }
    }
}

impl Project {
    pub fn note(&self, id: u64) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn add_note(&mut self, name: &str, text: String) -> u64 {
        let id = self.alloc_id();
        let name = self.unique_name(name);
        self.notes.push(Note { id, name, text });
        id
    }

    /// 界面、预设和未来脚本共用的声明式作图入口；校验成功后才修改项目。
    pub fn plot_spec(&mut self, spec: crate::plotspec::PlotSpec) -> Result<u64, String> {
        let book = self.book(spec.book_id.ok_or("请选择工作簿")?).ok_or("找不到工作簿")?;
        spec.validate_in(book, &|id| self.book(id))?;
        let parent = self.folder_of(book.id);
        let id = self.alloc_id();
        let mut graph = Graph::new(id, self.unique_name("Graph"));
        graph.first_layer_mut().spec = spec.into();
        if matches!(
            graph.first_layer().plot_type,
            PlotType::OffsetLine | PlotType::Waterfall
        ) {
            graph.first_layer_mut().offset = Some(Default::default());
        }
        if matches!(
            graph.first_layer().plot_type,
            PlotType::Box | PlotType::ColumnScatter | PlotType::LineSeries
        ) {
            graph.first_layer_mut().legend = false;
        }
        if matches!(
            graph.first_layer().plot_type,
            PlotType::PolarLine | PlotType::PolarScatter
        ) {
            graph.first_layer_mut().projection = crate::page::Projection::Polar { degrees: true };
        }
        self.graphs.push(graph);
        self.move_project_item(id, parent)?;
        Ok(id)
    }

    pub fn apply_graph_template(&mut self, template: &GraphTemplate, book_id: u64) -> Result<u64, String> {
        if template.version != GRAPH_TEMPLATE_VERSION {
            return Err(format!("不支持的图形模板版本 {}", template.version));
        }
        if template.name.trim().is_empty() {
            return Err("图形模板名称不能为空".into());
        }
        let mut page = template.page.clone();
        let mut curve_count = 0;
        {
            let book = self.book(book_id).ok_or("找不到目标工作簿")?;
            for layer in page.layers_mut() {
                if layer.book_id.is_some() || !layer.foreign_books().is_empty() {
                    return Err("图形模板仍绑定源工作簿，请重新保存模板".into());
                }
                layer.book_id = Some(book_id);
                let count = layer.draft().y_keys.len();
                if count > 0 {
                    layer.validate(book)?;
                    curve_count += count;
                }
            }
        }
        if curve_count == 0 {
            return Err("图形模板没有可应用的曲线".into());
        }
        page.validate()?;
        let name = self.unique_name(template.name.trim());
        let parent = self.folder_of(book_id);
        let next_id = self.next_id;
        let id = self.alloc_id();
        self.graphs.push(Graph { id, name, page });
        if let Err(error) = self.move_project_item(id, parent) {
            self.graphs.pop();
            self.next_id = next_id;
            return Err(error);
        }
        Ok(id)
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn book(&self, id: u64) -> Option<&Book> {
        self.books.iter().find(|b| b.id == id)
    }

    pub fn book_mut(&mut self, id: u64) -> Option<&mut Book> {
        self.books.iter_mut().find(|b| b.id == id)
    }

    pub fn graph(&self, id: u64) -> Option<&Graph> {
        self.graphs.iter().find(|g| g.id == id)
    }

    pub fn graph_mut(&mut self, id: u64) -> Option<&mut Graph> {
        self.graphs.iter_mut().find(|g| g.id == id)
    }

    pub fn name_of(&self, id: u64) -> Option<&str> {
        self.book(id)
            .map(|b| b.name.as_str())
            .or_else(|| self.graph(id).map(|g| g.name.as_str()))
            .or_else(|| self.note(id).map(|n| n.name.as_str()))
            .or_else(|| self.folder(id).map(|f| f.name.as_str()))
    }

    pub fn name_of_mut(&mut self, id: u64) -> Option<&mut String> {
        if let Some(i) = self.books.iter().position(|b| b.id == id) {
            return Some(&mut self.books[i].name);
        }
        if let Some(g) = self.graphs.iter_mut().find(|g| g.id == id) {
            return Some(&mut g.name);
        }
        if let Some(n) = self.notes.iter_mut().find(|n| n.id == id) {
            return Some(&mut n.name);
        }
        self.folders.iter_mut().find(|f| f.id == id).map(|f| &mut f.name)
    }

    /// Book1、Book2……里第一个没用过的名字。
    pub fn unique_name(&self, base: &str) -> String {
        (1..)
            .map(|i| format!("{}{}", base, i))
            .find(|n| {
                !self.books.iter().any(|b| &b.name == n)
                    && !self.graphs.iter().any(|g| &g.name == n)
                    && !self.notes.iter().any(|note| &note.name == n)
            })
            .unwrap()
    }

    /// 删除工作簿或图形。删除工作簿时，引用它的图形变成空图。
    pub fn remove(&mut self, id: u64) {
        for f in &mut self.folders {
            f.items.retain(|item| *item != id);
        }
        self.books.retain(|b| b.id != id);
        self.graphs.retain(|g| g.id != id);
        self.notes.retain(|n| n.id != id);
        let books = &self.books;
        for g in &mut self.graphs {
            for layer in g.page.layers_mut() {
                layer.remove_source_book(id, &|id| books.iter().find(|b| b.id == id));
            }
        }
    }

    /// 把工作簿的若干数据列加为图层曲线，工作簿可以与图层原来的不同；
    /// 图层还没有数据时改绑到这个工作簿。已在图层里的列不重复添加，返回新加的条数。
    pub fn add_curves(&mut self, graph_id: u64, layer: usize, book_id: u64, keys: &[String]) -> Result<usize, String> {
        let book = self.books.iter().find(|b| b.id == book_id).ok_or("找不到工作簿")?;
        let graph = self.graphs.iter_mut().find(|g| g.id == graph_id).ok_or("找不到图形")?;
        let layer = graph.page.layers_mut().get_mut(layer).ok_or("找不到图层")?;
        let main_alive = layer.book_id.is_some_and(|id| self.books.iter().any(|b| b.id == id));
        layer.add_book_curves(book, keys, main_alive)
    }

    /// 删除工作簿中的若干列，并从图形里去掉对它们的引用。
    pub fn delete_columns(&mut self, book_id: u64, keys: &[String]) {
        if let Some(b) = self.book_mut(book_id) {
            b.freeze_error_sources();
            for col in &mut b.columns {
                if col.error_for.as_ref().is_some_and(|key| keys.contains(key)) {
                    col.error_for = Some(String::new());
                }
            }
            for col in &mut b.columns {
                if let Some(op) = &mut col.op {
                    if let crate::ops::OpKind::Nonlinear(recipe) = &mut op.kind {
                        if keys.contains(&op.y)
                            || op.x.as_ref().is_some_and(|x| keys.contains(x))
                            || keys.contains(&recipe.output_x)
                        {
                            col.op = None; // 来源或必要输出删除后成为快照，列标识复用不会重绑。
                            if let Some(report) = &mut col.fit_report {
                                report.current = false;
                                report.error = Some("来源或必要输出已删除，保留上次结果快照".into());
                            }
                        } else {
                            recipe.bands.retain(|b| !b.columns.iter().any(|key| keys.contains(key)));
                            if recipe.components.iter().any(|key| keys.contains(key)) {
                                recipe.components.clear();
                            }
                            if recipe.residual.as_ref().is_some_and(|k| keys.contains(k)) {
                                recipe.residual = None;
                            }
                        }
                    }
                }
            }
            b.columns.retain(|c| !keys.contains(&c.key));
            if let Some(filter) = &mut b.filter {
                filter.rules.retain(|key, _| !keys.contains(key));
                if filter.rules.is_empty() {
                    b.filter = None;
                }
            }
        }
        for g in &mut self.graphs {
            for layer in g.page.layers_mut() {
                layer.remove_columns(book_id, keys);
            }
        }
    }

    /// 在第 `at` 行之前插入 `n` 个空行；图形里引用这个工作簿的行范围随之下移。
    pub fn insert_rows(&mut self, book_id: u64, at: usize, n: usize) {
        let Some(b) = self.book_mut(book_id) else { return };
        if n == 0 {
            return;
        }
        for col in b.columns.iter_mut().filter(|c| !c.independent_rows) {
            // 插入点在数据末尾之后时，那里本来就是空行，不必补齐
            if at < col.data.len() {
                col.data.insert_empty(at, n);
            }
        }
        self.shift_row_ranges(book_id, |r| if r >= at { Some(r + n) } else { Some(r) });
    }

    /// 删除第 `r0..=r1` 行（整行，所有列）；图形的行范围相应上移，整段被删掉时改回使用全部行。
    pub fn delete_rows(&mut self, book_id: u64, r0: usize, r1: usize) {
        let Some(b) = self.book_mut(book_id) else { return };
        if r0 > r1 {
            return;
        }
        for col in b.columns.iter_mut().filter(|c| !c.independent_rows) {
            if r0 < col.data.len() {
                col.data.delete_rows(r0, r1);
            }
            while col.data.last().is_some_and(|s| s.trim().is_empty()) {
                col.data.pop();
            }
        }
        let k = r1 - r0 + 1;
        self.shift_row_ranges(book_id, |r| {
            if r < r0 {
                Some(r)
            } else if r > r1 {
                Some(r - k)
            } else {
                None
            }
        });
    }

    /// 按 `map` 平移引用 `book_id` 的图层行范围；端点被删除时向内收缩，整段删除则清除范围。
    fn shift_row_ranges(&mut self, book_id: u64, map: impl Fn(usize) -> Option<usize>) {
        let shift = |range: &mut Option<(usize, usize)>| {
            if let Some((a, b)) = *range {
                let start = (a..=b).find_map(&map);
                let end = (a..=b).rev().find_map(&map);
                *range = match (start, end) {
                    (Some(s), Some(e)) if s <= e => Some((s, e)),
                    _ => None,
                };
            }
        };
        if let Some(book) = self.book_mut(book_id) {
            for col in &mut book.columns {
                if let Some(operation) = &mut col.op {
                    shift(&mut operation.rows);
                }
            }
        }
        let affected: std::collections::HashSet<String> = self
            .book(book_id)
            .into_iter()
            .flat_map(|b| &b.columns)
            .filter(|c| c.independent_rows)
            .map(|c| c.key.clone())
            .collect();
        for g in &mut self.graphs {
            for layer in g.page.layers_mut() {
                layer.map_row_ranges(book_id, &affected, shift);
            }
        }
    }

    pub fn new_book(&mut self) -> u64 {
        let id = self.alloc_id();
        let mut b = Book::new(id, self.unique_name("Book"));
        b.push_column(Role::X);
        b.push_column(Role::Y);
        self.books.push(b);
        id
    }

    /// 多面板图：选中的 Y 列按顺序平均分到各面板（每个面板一个图层），X 列的选取规则与普通绘图相同。
    pub fn plot_panels(
        &mut self,
        book_id: u64,
        selected: &[String],
        plot_type: PlotType,
        panels: Panels,
    ) -> Result<(u64, String), String> {
        if plot_type == PlotType::FloatingBar {
            return Err("浮动柱需要成对的 Y 列，请用单面板绘制".into());
        }
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let layer = crate::page::Layer {
            spec: crate::plotspec::PlotSpec::from_selection(book, selected, plot_type)?.into(),
            ..Default::default()
        };
        let n = panels.count();
        let plots = layer.plots(&|id| self.book(id));
        let ys: Vec<_> = plots.iter().map(|plot| plot.mapping.y.clone()).collect();
        if ys.len() < n {
            return Err(format!(
                "{}需要至少 {} 个 Y 列，当前只有 {} 个",
                panels.label(),
                n,
                ys.len()
            ));
        }
        let book_name = book.name.clone();
        // 前面的面板多分一列，例如 5 列分四面板为 2、1、1、1
        let mut chunks = Vec::with_capacity(n);
        let mut start = 0;
        for i in 0..n {
            let len = ys.len() / n + usize::from(i < ys.len() % n);
            chunks.push(ys[start..start + len].to_vec());
            start += len;
        }
        let layers: Vec<_> = chunks
            .into_iter()
            .map(|sources| {
                let mut panel = layer.clone();
                panel.retain_plots(&|id| self.book(id), |plot| sources.contains(&plot.mapping.y));
                panel
            })
            .collect();
        for layer in &layers {
            layer.spec.validate(book)?;
        }
        let parent = self.folder_of(book_id);
        let id = self.alloc_id();
        let mut graph = Graph::new(id, self.unique_name("Graph"));
        let mut layers = layers.into_iter();
        *graph.first_layer_mut() = layers.next().unwrap();
        for layer in layers {
            graph.page.add_layer(layer);
        }
        graph.page.columns = panels.columns();
        (graph.page.width_mm, graph.page.height_mm) = panels.page_mm();
        self.graphs.push(graph);
        self.move_project_item(id, parent)?;
        Ok((
            id,
            format!("已用 {} 的 {} 个 Y 列绘制{}", book_name, ys.len(), panels.label()),
        ))
    }

    /// 按选中的列绘图，规则与 Origin 一致：
    /// 选中的 Y 列各成一条曲线；X 取选中的 X 列，没有就取工作表里位于第一条 Y 列左边最近的 X 列；
    /// 什么都没选时使用全部 Y 列。
    pub fn plot_from_selection(
        &mut self,
        book_id: u64,
        selected: &[String],
        plot_type: PlotType,
    ) -> Result<(u64, String), String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let auto = selected.is_empty();
        let spec = crate::plotspec::PlotSpec::from_selection(book, selected, plot_type)?;
        let x_key = spec.x_key.clone();
        let ys: Vec<_> = crate::page::Layer {
            spec: spec.clone().into(),
            ..Default::default()
        }
        .plots(&|id| self.book(id))
        .into_iter()
        .map(|plot| plot.key)
        .collect();
        let book_name = book.name.clone();

        let id = self.plot_spec(spec)?;

        let msg = format!(
            "已用 {} 的 {} 列绘制{}，{}{}",
            book_name,
            ys.join(", "),
            plot_type.label(),
            match &x_key {
                Some(k) => format!("X = {}", k),
                None => "X 为行号".to_string(),
            },
            if auto {
                "（未选列，已使用全部 Y 列）"
            } else {
                ""
            }
        );
        Ok((id, msg))
    }

    /// 示例项目：一个工作簿（阻尼振荡 + 充电曲线）和一张点线图。
    pub fn sample() -> Project {
        let mut p = Project::default();
        let bid = p.alloc_id();
        let mut book = Book::new(bid, "Book1");
        let mut a = Column::new("A", Role::X);
        a.long_name = "时间".into();
        a.units = "s".into();
        a.comment = "示例数据".into();
        let mut b = Column::new("B", Role::Y);
        b.long_name = "电压 A".into();
        b.units = "mV".into();
        b.comment = "阻尼振荡".into();
        let mut c = Column::new("C", Role::Y);
        c.long_name = "电压 B".into();
        c.units = "mV".into();
        c.comment = "充电曲线".into();

        // 固定种子的伪随机噪声，保证每次打开都一样
        let mut seed: u64 = 7;
        let mut rnd = move || {
            seed = (seed * 16807) % 2147483647;
            seed as f64 / 2147483647.0 - 0.5
        };
        for i in 0..=40 {
            let t = i as f64 * 0.5;
            a.data.push(fmt_num(t));
            let vb = 10.0 * (-t / 6.0).exp() * (1.2 * t).cos() + 0.35 * rnd();
            let vc = 8.0 * (1.0 - (-t / 4.0).exp()) + 0.25 * rnd();
            b.data.push(fmt_num((vb * 1000.0).round() / 1000.0));
            c.data.push(fmt_num((vc * 1000.0).round() / 1000.0));
        }
        book.columns = vec![a, b, c];
        p.books.push(book);

        let gid = p.alloc_id();
        let mut g = Graph::new(gid, "Graph1");
        g.first_layer_mut().book_id = Some(bid);
        g.first_layer_mut().x_key = Some("A".into());
        for y in ["B", "C"] {
            let plot = crate::plotspec::Plot {
                key: y.into(),
                mapping: crate::plotspec::CurveMapping {
                    y: crate::plotspec::DataRef::new(bid, y),
                    x: Some(crate::plotspec::DataRef::new(bid, "A")),
                    base: None,
                    rows: None,
                    line_only: false,
                },
                style: None,
                x_override: false,
            };
            g.first_layer_mut().add_plot(plot, &|id| p.book(id)).unwrap();
        }
        p.graphs.push(g);
        p
    }
}

impl Book {
    /// 非线性结果组的所有权与重算模式无关。
    pub fn nonlinear_output_owner(&self, key: &str) -> Option<&str> {
        self.columns.iter().find_map(|c| {
            let crate::ops::Operation {
                kind: crate::ops::OpKind::Nonlinear(r),
                ..
            } = c.op.as_ref()?
            else {
                return None;
            };
            (c.key == key
                || r.output_x == key
                || r.residual.as_deref() == Some(key)
                || r.components.iter().any(|k| k == key)
                || r.bands.iter().any(|b| b.columns.iter().any(|k| k == key)))
            .then_some(c.key.as_str())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_columns_keeps_selection_order_and_error_links() {
        let mut b = Book::new(1, "Book1");
        for role in [Role::X, Role::Y, Role::YError, Role::Y, Role::Y] {
            b.push_column(role);
        }
        let order = |b: &Book| b.columns.iter().map(|c| c.key.clone()).collect::<Vec<_>>().join("");
        assert_eq!(order(&b), "ABCDE");
        // C 是 B 的误差列；把 B 右移到 D 后面，C 仍关联 B
        assert!(b.move_columns(&["B".into()], ColumnMove::Right));
        assert_eq!(order(&b), "ACBDE");
        assert!(b.move_columns(&["B".into()], ColumnMove::Right));
        assert_eq!(order(&b), "ACDBE");
        let c = b.column_index("C").unwrap();
        assert_eq!(b.error_source(c), Some("B"));
        // 不相邻的多列一起左移，相对顺序不变；已在最左的不动
        assert!(b.move_columns(&["A".into(), "B".into(), "E".into()], ColumnMove::Left));
        assert_eq!(order(&b), "ACBED");
        assert!(b.move_columns(&["B".into(), "E".into()], ColumnMove::First));
        assert_eq!(order(&b), "BEACD");
        assert!(b.move_columns(&["B".into(), "E".into()], ColumnMove::Last));
        assert_eq!(order(&b), "ACDBE");
        assert!(!b.move_columns(&["B".into(), "E".into()], ColumnMove::Last));
        assert!(!b.move_columns(&["Z".into()], ColumnMove::Left));
    }

    #[test]
    fn range_copy_clear_fill() {
        let mut b = Book::new(1, "B");
        b.columns.push(Column::new("A", Role::X));
        b.columns.push(Column::new("B", Role::Y));
        for r in 0..3 {
            b.columns[0].set(r, format!("{}", r));
            b.columns[1].set(r, format!("{}", r * 10));
        }
        assert_eq!(b.range_tsv(0, 1, 1, 2), "1\t10\n2\t20");
        assert_eq!(b.range_tsv(1, 5, 0, 0), "0");
        b.clear_range(0, 1, 1, 2);
        assert_eq!(b.nrows(), 1);
        b.fill_range(1, 1, 0, 3, "7");
        assert_eq!(b.columns[1].data, vec!["7"; 4]);
    }

    #[test]
    fn letters() {
        assert_eq!(column_letter(0), "A");
        assert_eq!(column_letter(25), "Z");
        assert_eq!(column_letter(26), "AA");
        assert_eq!(column_letter(27), "AB");
    }

    #[test]
    fn number_parsing_and_format() {
        assert_eq!(parse_num(" 1,234.5 "), 1234.5);
        assert!(parse_num("abc").is_nan());
        assert!(parse_num("").is_nan());
        assert_eq!(fmt_num(0.1 + 0.2), "0.3");
        assert_eq!(fmt_num(2.0), "2");
    }

    #[test]
    fn set_cell_trims_trailing_blanks() {
        let mut c = Column::new("A", Role::Y);
        c.set(3, "5".into());
        assert_eq!(c.data.len(), 4);
        c.set(3, "".into());
        assert_eq!(c.data.len(), 0);
        c.set(10, "".into());
        assert_eq!(c.data.len(), 0);
    }

    #[test]
    fn plot_selection_rules() {
        let mut p = Project::sample();
        // 什么都不选：全部 Y 列，X = A
        let (gid, _) = p.plot_from_selection(1, &[], PlotType::Line).unwrap();
        let g = p.graph(gid).unwrap().first_layer();
        assert_eq!(g.draft().y_keys, vec!["B", "C"]);
        assert_eq!(g.x_key.as_deref(), Some("A"));
        // 只选 C：X 自动找左边的 A
        let (gid, _) = p.plot_from_selection(1, &["C".to_string()], PlotType::Scatter).unwrap();
        assert_eq!(p.graph(gid).unwrap().first_layer().x_key.as_deref(), Some("A"));
        // 只选 X 列：报错
        assert!(p.plot_from_selection(1, &["A".to_string()], PlotType::Bar).is_err());
    }

    #[test]
    fn series_skip_blank_rows() {
        let p = Project::sample();
        let g = p.graph(2).unwrap().first_layer();
        let (x, s) = g.series(p.book(1).unwrap());
        assert_eq!(x.unwrap().key, "A");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].points.len(), 41);
        assert_eq!(s[0].name, "电压 A");
    }

    #[test]
    fn panels_split_y_columns_in_order_and_lay_out_on_a_grid() {
        let mut p = Project::sample();
        for (panels, columns, size) in [
            (Panels::Stacked, 1, (210.0, 280.0)),
            (Panels::SideBySide, 2, (210.0, 105.0)),
        ] {
            let (id, _) = p.plot_panels(1, &[], PlotType::Line, panels).unwrap();
            let page = &p.graph(id).unwrap().page;
            let keys: Vec<_> = page.layers().iter().map(|l| l.draft().y_keys.clone()).collect();
            assert_eq!(keys, [vec!["B".to_string()], vec!["C".to_string()]]);
            assert_eq!((page.columns, (page.width_mm, page.height_mm)), (columns, size));
            assert!(page.layers().iter().all(|l| l.x_key.as_deref() == Some("A")));
        }
        assert!(p
            .plot_panels(1, &[], PlotType::Line, Panels::Grid4)
            .unwrap_err()
            .contains("至少 4 个"));
        assert!(p.plot_panels(1, &[], PlotType::FloatingBar, Panels::Stacked).is_err());

        let book = p.book_mut(1).unwrap();
        for _ in 0..3 {
            let k = book.push_column(Role::Y);
            let data = book.columns[1].data.clone();
            book.columns.iter_mut().find(|c| c.key == k).unwrap().data = data;
        }
        let (id, msg) = p.plot_panels(1, &[], PlotType::Scatter, Panels::Grid4).unwrap();
        assert!(msg.contains("5 个 Y 列"));
        let g = p.graph(id).unwrap();
        let sizes: Vec<_> = g.page.layers().iter().map(|l| l.draft().y_keys.len()).collect();
        assert_eq!(sizes, [2, 1, 1, 1], "earlier panels take the extra columns");
        assert!(g.page.layers().iter().all(|l| l.plot_type == PlotType::Scatter));
        let sc = crate::scene::layout_page(g, &p, &[]);
        let xs: std::collections::BTreeSet<i64> = sc.layers.iter().map(|l| l.plot[0].round() as i64).collect();
        let ys: std::collections::BTreeSet<i64> = sc.layers.iter().map(|l| l.plot[1].round() as i64).collect();
        assert_eq!((sc.layers.len(), xs.len(), ys.len()), (4, 2, 2), "a 2 × 2 grid");
    }

    #[test]
    fn inserting_and_deleting_rows_moves_cells_and_graph_row_ranges() {
        let mut p = Project::sample();
        let a = p.book(1).unwrap().columns[0].data.clone();
        p.graph_mut(2).unwrap().first_layer_mut().rows = Some((5, 10));

        p.insert_rows(1, 3, 2);
        let col = &p.book(1).unwrap().columns[0].data;
        assert_eq!(col.len(), a.len() + 2);
        assert_eq!(
            col.iter().take(3).collect::<Vec<_>>(),
            a.iter().take(3).collect::<Vec<_>>()
        );
        assert_eq!(col.text_at(3), "");
        assert_eq!(col.text_at(4), "");
        assert_eq!(
            col.iter().skip(5).collect::<Vec<_>>(),
            a.iter().skip(3).collect::<Vec<_>>()
        );
        assert_eq!(p.graph(2).unwrap().first_layer().rows, Some((7, 12)));

        p.delete_rows(1, 3, 4);
        assert_eq!(p.book(1).unwrap().columns[0].data, a);
        assert_eq!(p.graph(2).unwrap().first_layer().rows, Some((5, 10)));

        // 删掉范围的起点：范围从删除段之后的第一行开始
        p.delete_rows(1, 4, 6);
        assert_eq!(p.book(1).unwrap().columns[0].data.text_at(4), a.text_at(7));
        assert_eq!(p.graph(2).unwrap().first_layer().rows, Some((4, 7)));
        // 整段删除：改回使用全部行
        p.delete_rows(1, 3, 9);
        assert_eq!(p.graph(2).unwrap().first_layer().rows, None);
    }

    #[test]
    fn row_edits_leave_short_columns_alone_and_trim_trailing_blanks() {
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        book.columns[2].data.truncate(3);
        book.columns[1].data.push(String::new());
        let short = book.columns[2].data.clone();
        p.insert_rows(1, 10, 4);
        assert_eq!(
            p.book(1).unwrap().columns[2].data,
            short,
            "rows past a column's end are already blank"
        );

        let book = p.book_mut(1).unwrap();
        let last = book.columns[0].data.len() - 1;
        book.columns[0].data.set_text(last, String::new());
        p.delete_rows(1, 0, 0);
        let book = p.book(1).unwrap();
        assert!(book
            .columns
            .iter()
            .all(|c| c.data.last().is_none_or(|s| !s.trim().is_empty())));
        assert_eq!(
            book.columns[2].data.iter().collect::<Vec<_>>(),
            short.iter().skip(1).collect::<Vec<_>>()
        );
        // 区间倒置、插入 0 行都不改动数据
        let before = p.clone();
        p.delete_rows(1, 5, 2);
        p.insert_rows(1, 0, 0);
        assert_eq!(p, before);
    }

    #[test]
    fn delete_columns_updates_graphs() {
        let mut p = Project::sample();
        p.delete_columns(1, &["A".to_string(), "B".to_string()]);
        let g = p.graph(2).unwrap().first_layer();
        assert_eq!(g.x_key, None);
        assert_eq!(g.draft().y_keys, vec!["C"]);
        assert_eq!(p.book(1).unwrap().columns.len(), 1);
        // 新列标识不会和剩下的 C 冲突
        assert_eq!(p.book(1).unwrap().next_key(), "A");
    }

    #[test]
    fn unique_names() {
        let mut p = Project::sample();
        assert_eq!(p.unique_name("Book"), "Book2");
        let id = p.new_book();
        assert_eq!(p.name_of(id), Some("Book2"));
    }
}
