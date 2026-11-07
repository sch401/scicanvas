//! Graph themes copy appearance, leaving mappings, ranges and title text intact.
use crate::{
    colormap::ColorMap,
    page::{CurveStyle, Layer, TickDir},
};
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct GraphTheme {
    pub name: String,
    pub line_width: f32,
    pub symbol_size: f32,
    pub area_opacity: f32,
    pub grid: bool,
    pub frame: bool,
    pub tick_dir: TickDir,
    pub legend: bool,
    pub color_map: ColorMap,
    #[cfg_attr(feature = "serde", serde(default))]
    pub axis_style: crate::page::AxisStyle,
}
impl GraphTheme {
    pub const BUILTIN: &'static str = "内置默认";
    pub fn builtin() -> Self {
        let mut theme = Self::capture(&Layer::default());
        theme.name = Self::BUILTIN.into();
        theme
    }
    pub fn capture(layer: &Layer) -> Self {
        Self {
            name: "主题".into(),
            line_width: layer.line_width,
            symbol_size: layer.symbol_size,
            area_opacity: layer.area_opacity,
            grid: layer.grid,
            frame: layer.frame,
            tick_dir: layer.tick_dir,
            legend: layer.legend,
            color_map: layer.color_map,
            axis_style: layer.axis_style,
        }
    }
    pub fn apply(&self, layer: &mut Layer) {
        layer.line_width = self.line_width;
        layer.symbol_size = self.symbol_size;
        layer.area_opacity = self.area_opacity;
        layer.grid = self.grid;
        layer.frame = self.frame;
        layer.tick_dir = self.tick_dir;
        layer.legend = self.legend;
        layer.color_map = self.color_map;
        layer.axis_style = self.axis_style;
    }
    pub fn validate(&self) -> Result<(), String> {
        self.axis_style.validate()?;
        if self.name.trim().is_empty()
            || self.name.len() > 256
            || !self.line_width.is_finite()
            || self.line_width <= 0.
            || !self.symbol_size.is_finite()
            || self.symbol_size <= 0.
            || !self.area_opacity.is_finite()
            || !(0.0..=1.0).contains(&self.area_opacity)
        {
            return Err("主题名称、线宽、符号大小或填充透明度无效".into());
        }
        Ok(())
    }
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map(|s| s + "\n")
            .map_err(|e| e.to_string())
    }
    #[cfg(feature = "serde")]
    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() > 1024 * 1024 {
            return Err("主题文件超过 1 MiB".into());
        }
        let theme: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        theme.validate()?;
        Ok(theme)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum StyleClip {
    Layer(GraphTheme),
    Curve(CurveStyle),
}
impl StyleClip {
    pub fn curve(style: &CurveStyle) -> Self {
        let mut style = style.clone();
        style.legend_label.clear();
        style.color_column = None;
        style.size_column = None;
        style.symbol_column = None;
        Self::Curve(style)
    }
}
impl crate::Project {
    pub fn save_graph_theme(&mut self, mut theme: GraphTheme) -> Result<(), String> {
        theme.name = theme.name.trim().into();
        theme.validate()?;
        if theme.name == GraphTheme::BUILTIN {
            return Err("内置主题保留原值，请使用新名称".into());
        }
        if let Some(existing) = self.graph_themes.iter_mut().find(|t| t.name == theme.name) {
            *existing = theme
        } else {
            self.graph_themes.push(theme)
        }
        Ok(())
    }
    pub fn remove_graph_theme(&mut self, name: &str) -> Result<(), String> {
        if name == GraphTheme::BUILTIN {
            return Err("内置主题保留".into());
        }
        self.graph_themes.retain(|t| t.name != name);
        Ok(())
    }
}

impl crate::Project {
    pub(crate) fn rename_builtin_graph_theme(&mut self) -> bool {
        let Some(index) = self
            .graph_themes
            .iter()
            .position(|theme| theme.name == GraphTheme::BUILTIN)
        else {
            return false;
        };
        let names: std::collections::BTreeSet<_> = self.graph_themes.iter().map(|theme| theme.name.as_str()).collect();
        let mut name = "内置默认（项目）".to_string();
        let mut number = 2;
        while names.contains(name.as_str()) {
            name = format!("内置默认（项目 {number}）");
            number += 1;
        }
        self.graph_themes[index].name = name;
        true
    }

    pub(crate) fn validate_graph_themes(&self) -> Result<(), String> {
        let mut theme_names = std::collections::BTreeSet::new();
        for theme in &self.graph_themes {
            theme.validate()?;
            if theme.name == crate::graph_theme::GraphTheme::BUILTIN || !theme_names.insert(&theme.name) {
                return Err("项目主题名称重复或占用内置名称".into());
            }
        }
        Ok(())
    }
}
