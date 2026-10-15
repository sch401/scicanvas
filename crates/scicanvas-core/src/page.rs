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
}
impl Default for Page {
    fn default() -> Self {
        Self {
            width_mm: 85.0,
            height_mm: 63.75,
            columns: 1,
            layers: vec![Layer::default()],
        }
    }
}
impl Page {
    /// 页面固定逻辑画布，96 个逻辑单位对应一英寸。
    pub fn logical_size(&self) -> [f64; 2] {
        [self.width_mm * 96.0 / 25.4, self.height_mm * 96.0 / 25.4]
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
            line_width: 1.5,
            symbol_size: 6.0,
            area_opacity: default_area_opacity(),
            error_bars: Default::default(),
            labels: Default::default(),
            grid: false,
            legend: true,
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
        book.and_then(|b| self.y_keys.first().and_then(|k| b.column(k)))
            .map(|c| c.axis_label())
            .unwrap_or_default()
    }
}
