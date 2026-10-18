//! SciCanvas 核心库。
//!
//! 这里只放与界面无关的逻辑：数据模型、CSV/剪贴板解析、坐标轴刻度、SVG 导出。
//! 界面层（egui）、命令行工具、将来的 Python 绑定都调用这一层。

pub mod analysis;
pub mod baseline;
pub mod colormap;
pub mod coord;
pub mod csv;
pub mod data;
pub mod fft;
pub mod formula;
pub mod history;
pub mod interpolate;
pub mod model;
pub mod nlfit;
pub mod ops;
pub mod page;
pub mod peaks;
pub mod plotspec;
pub mod presets;
#[cfg(feature = "serde")]
pub mod project_file;
pub mod project_tree;
pub mod scene;
pub mod spline;
pub mod stats;
pub mod svg;
pub mod ticks;
#[cfg(feature = "xlsx")]
pub mod xlsx;

pub use data::{ColumnData, ColumnView, DataKind};
pub use model::*;
