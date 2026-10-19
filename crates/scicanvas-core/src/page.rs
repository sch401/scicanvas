//! 页面 → 图层 → 作图描述。图层样式和数据引用均可独立设置。
use crate::{plotspec::PlotSpec, Book};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct Page {
    pub width_mm: f64,
    pub height_mm: f64,
    pub columns: usize,
    layers: Vec<Layer>,
    /// 页面背景色，None 为白色。图层和图例的底色跟随页面背景。
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub background: Option<[u8; 3]>,
    /// 页边距 mm（左、上、右、下）：网格排版的图层放在边距以内；自定义位置的图层仍按整页定位。
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "no_margin"))]
    pub margin_mm: [f64; 4],
    /// 网格图层之间的间距 mm（横向、纵向）。
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "no_gap"))]
    pub gap_mm: [f64; 2],
}

#[cfg(feature = "serde")]
fn no_gap(g: &[f64; 2]) -> bool {
    *g == [0.0; 2]
}

#[cfg(feature = "serde")]
fn no_margin(m: &[f64; 4]) -> bool {
    *m == [0.0; 4]
}
/// 字号、刻度、点和线宽按这一栏宽设计（Nature 单栏 89 mm）；更宽的栏按比例整体放大。
pub const REFERENCE_COLUMN_MM: f64 = 89.0;

/// 新图形的默认页宽：A4 纸宽。
pub const A4_WIDTH_MM: f64 = 210.0;

/// 早期项目文件没有记录页面尺寸时使用的旧默认值（Nature 单栏 89 mm，4:3），读取时补上以保持原样。
pub const LEGACY_PAGE_MM: (f64, f64) = (89.0, 66.75);

impl Default for Page {
    fn default() -> Self {
        Self {
            // A4 纸宽 210 mm，4:3
            width_mm: A4_WIDTH_MM,
            height_mm: A4_WIDTH_MM * 0.75,
            columns: 1,
            layers: vec![Layer::default()],
            background: None,
            margin_mm: [0.0; 4],
            gap_mm: [0.0; 2],
        }
    }
}
impl Page {
    /// 页面固定逻辑画布，96 个逻辑单位对应一英寸。
    pub fn logical_size(&self) -> [f64; 2] {
        [self.width_mm * 96.0 / 25.4, self.height_mm * 96.0 / 25.4]
    }
    /// 版面放大倍数：每栏宽度 / 89 mm，至少为 1（单栏及更窄的页面保持原尺寸）。
    /// 网格排版区域（逻辑单位）：[左, 上, 宽, 高]，即整页扣除页边距。
    pub fn content_rect(&self) -> [f64; 4] {
        let [w, h] = self.logical_size();
        let px = 96.0 / 25.4;
        let [l, t, r, b] = self.margin_mm.map(|m| m * px);
        [l, t, w - l - r, h - t - b]
    }

    pub fn scale(&self) -> f64 {
        let cols = self
            .columns
            .clamp(1, self.layers.iter().filter(|l| l.placement.is_none()).count().max(1));
        let inner = self.width_mm - self.margin_mm[0] - self.margin_mm[2];
        let k = inner / cols as f64 / REFERENCE_COLUMN_MM;
        if k.is_finite() {
            k.max(1.0)
        } else {
            1.0
        }
    }
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }
    pub fn layers_mut(&mut self) -> &mut [Layer] {
        &mut self.layers
    }
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }
    pub fn remove_layer(&mut self, index: usize) -> Result<Layer, String> {
        if self.layers.len() <= 1 {
            return Err("页面至少保留一个图层".into());
        }
        if index >= self.layers.len() {
            return Err("找不到图层".into());
        }
        let layer = self.layers.remove(index);
        self.columns = self.columns.min(self.layers.len());
        // 联动到被删图层的取消，后面的图层序号前移
        for l in &mut self.layers {
            for link in [&mut l.link_x, &mut l.link_y] {
                *link = match *link {
                    Some(j) if j == index => None,
                    Some(j) if j > index => Some(j - 1),
                    other => other,
                };
            }
        }
        Ok(layer)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.layers.is_empty() || self.columns == 0 || self.columns > self.layers.len() {
            return Err("页面需要至少一个图层，排版列数应在 1 与图层数之间".into());
        }
        if !self.width_mm.is_finite()
            || !self.height_mm.is_finite()
            || self.width_mm <= 0.0
            || self.height_mm <= 0.0
            || self.logical_size().iter().any(|v| !v.is_finite())
        {
            return Err("页面毫米尺寸必须为有限正数".into());
        }
        let [l, t, r, b] = self.margin_mm;
        if self.margin_mm.iter().any(|m| !m.is_finite() || *m < 0.0)
            || self.width_mm - l - r < 1.0
            || self.height_mm - t - b < 1.0
        {
            return Err("页边距必须非负，且留给图层的区域至少 1 mm".into());
        }
        if self.gap_mm.iter().any(|g| !g.is_finite() || *g < 0.0) {
            return Err("网格间距必须是非负数".into());
        }
        for (i, layer) in self.layers.iter().enumerate() {
            for link in [layer.link_x, layer.link_y].into_iter().flatten() {
                if link == i || link >= self.layers.len() {
                    return Err("坐标轴联动的目标图层无效".into());
                }
            }
        }
        for layer in &self.layers {
            if let Some(rect) = layer.placement {
                rect.validate()?;
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Layer {
    /// None 使用网格排版；自定义矩形按页面比例定位，可与其他图层重叠。
    #[cfg_attr(feature = "serde", serde(default))]
    pub placement: Option<LayerRect>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub transparent: bool,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub spec: PlotSpec,
    pub title: String,
    pub x_title: String,
    pub y_title: String,
    pub x_auto: bool,
    pub x_min: f64,
    pub x_max: f64,
    pub y_auto: bool,
    pub y_min: f64,
    pub y_max: f64,
    pub x_log: bool,
    pub y_log: bool,
    pub line_width: f32,
    pub symbol_size: f32,
    #[cfg_attr(feature = "serde", serde(default = "default_area_opacity"))]
    pub area_opacity: f32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub error_bars: ErrorBars,
    #[cfg_attr(feature = "serde", serde(default))]
    pub labels: DataLabels,
    pub grid: bool,
    pub legend: bool,
    /// 四边框：补上顶边和右边，并在这两边画不带标签的镜像刻度。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub frame: bool,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "TickDir::is_out"))]
    pub tick_dir: TickDir,
    /// 逐曲线样式（按 Y 列标识）：图型、颜色、符号、线宽、符号大小、右侧 Y 轴。
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")
    )]
    pub curves: std::collections::BTreeMap<String, CurveStyle>,
    /// 右侧 Y 轴标题（空则用右轴第一条曲线的列名和单位）。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "String::is_empty"))]
    pub y2_title: String,
    /// 坐标轴联动：X、Y 轴的范围跟随另一个图层（图层序号）。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub link_x: Option<usize>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub link_y: Option<usize>,
    /// 刻度标签的数字格式（日期列做 X 时自动用日期刻度）。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_auto_number"))]
    pub x_format: crate::format::NumberFormat,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_auto_number"))]
    pub y_format: crate::format::NumberFormat,
    /// 折线、点线图的连接方式。
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Connect::is_straight"))]
    pub connect: Connect,
}

/// 一条曲线的单独样式；None 表示跟随图层设置。
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct CurveStyle {
    pub kind: Option<crate::PlotType>,
    pub color: Option<[u8; 3]>,
    pub symbol: Option<crate::Symbol>,
    pub line_width: Option<f32>,
    pub symbol_size: Option<f32>,
    /// 画在右侧 Y 轴上（双 Y 轴）。
    pub right_axis: bool,
}

impl CurveStyle {
    /// 逐曲线可选的图型（同一图层里可以混合）；堆叠类、浮动柱和条形图按整层设置。
    pub const KINDS: [crate::PlotType; 10] = [
        crate::PlotType::Line,
        crate::PlotType::Scatter,
        crate::PlotType::LineSymbol,
        crate::PlotType::Bar,
        crate::PlotType::Area,
        crate::PlotType::Spline,
        crate::PlotType::SplineSymbol,
        crate::PlotType::VerticalDrop,
        crate::PlotType::StepHorizontal,
        crate::PlotType::StepVertical,
    ];

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// 连线方式（只用于折线和点线图；样条、阶梯另有图型）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Connect {
    #[default]
    Straight,
    BSpline,
    Bezier,
}

impl Connect {
    pub const ALL: [Connect; 3] = [Connect::Straight, Connect::BSpline, Connect::Bezier];

    pub fn label(self) -> &'static str {
        match self {
            Connect::Straight => "直线",
            Connect::BSpline => "B 样条",
            Connect::Bezier => "Bezier",
        }
    }

    pub fn is_straight(&self) -> bool {
        *self == Connect::Straight
    }
}

#[cfg(feature = "serde")]
fn is_false(v: &bool) -> bool {
    !*v
}

#[cfg(feature = "serde")]
fn is_auto_number(f: &crate::format::NumberFormat) -> bool {
    *f == crate::format::NumberFormat::Auto
}

/// 刻度方向（相对坐标框）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TickDir {
    #[default]
    Out,
    In,
    Both,
}

impl TickDir {
    pub const ALL: [TickDir; 3] = [TickDir::Out, TickDir::In, TickDir::Both];

    pub fn label(self) -> &'static str {
        match self {
            TickDir::Out => "朝外",
            TickDir::In => "朝内",
            TickDir::Both => "内外",
        }
    }

    pub fn is_out(&self) -> bool {
        *self == TickDir::Out
    }

    /// (框外长度系数, 框内长度系数)。
    pub fn factors(self) -> (f64, f64) {
        match self {
            TickDir::Out => (1.0, 0.0),
            TickDir::In => (0.0, 1.0),
            TickDir::Both => (1.0, 1.0),
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            placement: None,
            transparent: false,
            spec: Default::default(),
            title: String::new(),
            x_title: String::new(),
            y_title: String::new(),
            x_auto: true,
            x_min: 0.0,
            x_max: 10.0,
            y_auto: true,
            y_min: 0.0,
            y_max: 10.0,
            x_log: false,
            y_log: false,
            line_width: 1.0,
            symbol_size: 4.5,
            area_opacity: default_area_opacity(),
            error_bars: Default::default(),
            labels: Default::default(),
            grid: false,
            legend: true,
            frame: false,
            tick_dir: TickDir::Out,
            connect: Connect::Straight,
            curves: Default::default(),
            y2_title: String::new(),
            link_x: None,
            link_y: None,
            x_format: crate::format::NumberFormat::Auto,
            y_format: crate::format::NumberFormat::Auto,
        }
    }
}

fn default_area_opacity() -> f32 {
    0.35
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LabelSource {
    #[default]
    Value,
    XY,
    Row,
    Column,
}
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct DataLabels {
    pub visible: bool,
    pub source: LabelSource,
    pub column: Option<String>,
    pub size: f32,
    pub offset: f32,
}
impl Default for DataLabels {
    fn default() -> Self {
        Self {
            visible: false,
            source: LabelSource::Value,
            column: None,
            size: 11.0,
            offset: 6.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct ErrorBars {
    pub visible: bool,
    pub cap: f32,
    pub width: f32,
}
impl Default for ErrorBars {
    fn default() -> Self {
        Self {
            visible: true,
            cap: 6.0,
            width: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LayerRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Default for LayerRect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}
impl LayerRect {
    pub fn validate(self) -> Result<(), String> {
        if [self.x, self.y, self.width, self.height].iter().any(|v| !v.is_finite())
            || !(0.0..1.0).contains(&self.x)
            || !(0.0..1.0).contains(&self.y)
            || !(0.0..=1.0).contains(&self.width)
            || !(0.0..=1.0).contains(&self.height)
            || self.width == 0.0
            || self.height == 0.0
            || self.x + self.width > 1.0 + 1e-12
            || self.y + self.height > 1.0 + 1e-12
        {
            return Err("图层位置和尺寸应为页面内的有效比例".into());
        }
        Ok(())
    }
}
impl std::ops::Deref for Layer {
    type Target = PlotSpec;
    fn deref(&self) -> &Self::Target {
        &self.spec
    }
}
impl std::ops::DerefMut for Layer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.spec
    }
}
impl Layer {
    pub fn x_axis_title(&self, book: Option<&Book>) -> String {
        if !self.x_title.trim().is_empty() {
            return self.x_title.clone();
        }
        if self.plot_type == crate::PlotType::HorizontalBar {
            return book
                .and_then(|b| self.y_keys.first().and_then(|k| b.column(k)))
                .map(|c| c.axis_label())
                .unwrap_or_default();
        }
        book.and_then(|b| self.x_key.as_deref().and_then(|k| b.column(k)))
            .map(|c| c.axis_label())
            .unwrap_or_else(|| "行号".to_string())
    }

    pub fn y_axis_title(&self, book: Option<&Book>) -> String {
        if !self.y_title.trim().is_empty() {
            return self.y_title.clone();
        }
        if self.plot_type == crate::PlotType::HorizontalBar {
            return book
                .and_then(|b| self.x_key.as_deref().and_then(|k| b.column(k)))
                .map(|c| c.axis_label())
                .unwrap_or_else(|| "行号".into());
        }
        if self.plot_type == crate::PlotType::PercentStackedBar {
            return "百分比 (%)".into();
        }
        if self.plot_type == crate::PlotType::FloatingBar {
            let range = book.and_then(|b| {
                let top = self.y_keys.first()?;
                Some(b.column(self.base_by_y.get(top)?)?.range_label(b.column(top)?))
            });
            if let Some(range) = range {
                return range;
            }
        }
        // 右侧 Y 轴的曲线不参与左轴标题
        book.and_then(|b| {
            self.y_keys
                .iter()
                .find(|k| !self.curves.get(*k).is_some_and(|c| c.right_axis))
                .and_then(|k| b.column(k))
        })
        .map(|c| c.axis_label())
        .unwrap_or_default()
    }

    /// 右侧 Y 轴标题：自定义标题，或右轴第一条曲线的列名和单位。
    pub fn y2_axis_title(&self, book: Option<&Book>) -> String {
        if !self.y2_title.trim().is_empty() {
            return self.y2_title.clone();
        }
        book.and_then(|b| {
            self.y_keys
                .iter()
                .find(|k| self.curves.get(*k).is_some_and(|c| c.right_axis))
                .and_then(|k| b.column(k))
        })
        .map(|c| c.axis_label())
        .unwrap_or_default()
    }
}
