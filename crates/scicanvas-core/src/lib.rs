//! SciCanvas 核心库。
//!
//! 这里只放与界面无关的逻辑：数据模型、CSV/剪贴板解析、坐标轴刻度、SVG 导出。
//! 界面层（egui）、命令行工具、将来的 Python 绑定都调用这一层。

pub mod analysis;
pub mod colormap;
pub mod coord;
pub mod csv;
pub mod history;
pub mod model;
pub mod ops;
pub mod page;
pub mod plotspec;
pub mod presets;
#[cfg(feature = "serde")]
pub mod project_file;
pub mod scene;
pub mod spline;
pub mod svg;
pub mod ticks;

pub use model::*;
