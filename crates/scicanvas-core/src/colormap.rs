//! 曲线配色与连续色映射，供图例、热图和曲面复用。
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColorMap {
    #[default]
    Origin,
    Viridis,
    Colorblind,
}

impl ColorMap {
    pub fn sample(self, t: f64) -> (u8, u8, u8) {
        let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
        match self {
            Self::Origin => crate::PALETTE[(t * (crate::PALETTE.len() - 1) as f64).round() as usize],
            Self::Colorblind => {
                const COLORS: [(u8, u8, u8); 8] = [
                    (0, 114, 178),
                    (230, 159, 0),
                    (0, 158, 115),
                    (213, 94, 0),
                    (204, 121, 167),
                    (86, 180, 233),
                    (240, 228, 66),
                    (0, 0, 0),
                ];
                COLORS[(t * 7.0).round() as usize]
            }
            Self::Viridis => {
                // ponytail: 五节点插值；需要完整色表精度时增加采样表，接口保持不变。
                const NODES: [[f64; 3]; 5] = [
                    [68.0, 1.0, 84.0],
                    [59.0, 82.0, 139.0],
                    [33.0, 145.0, 140.0],
                    [94.0, 201.0, 98.0],
                    [253.0, 231.0, 37.0],
                ];
                let p = t * 4.0;
                let i = (p as usize).min(3);
                let channel = |c: usize| (NODES[i][c] + (NODES[i + 1][c] - NODES[i][c]) * (p - i as f64)).round() as u8;
                (channel(0), channel(1), channel(2))
            }
        }
    }
}
