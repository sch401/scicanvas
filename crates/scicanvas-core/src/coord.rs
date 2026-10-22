//! 数据坐标与页面坐标的双向映射。线性、对数轴共用直角坐标实现。
use crate::ticks::Axis;

pub trait Coord {
    fn project(&self, point: [f64; 2]) -> [f64; 2];
    fn unproject(&self, point: [f64; 2]) -> [f64; 2];
}

pub struct Cartesian<'a> {
    pub x: &'a Axis,
    pub y: &'a Axis,
    pub rect: [f64; 4],
}

impl Coord for Cartesian<'_> {
    fn project(&self, point: [f64; 2]) -> [f64; 2] {
        let [left, top, width, height] = self.rect;
        [
            left + self.x.frac(point[0]) * width,
            top + height - self.y.frac(point[1]) * height,
        ]
    }
    fn unproject(&self, point: [f64; 2]) -> [f64; 2] {
        let [left, top, width, height] = self.rect;
        [
            self.x.value((point[0] - left) / width),
            self.y.value((top + height - point[1]) / height),
        ]
    }
}

/// Orientation changes data coordinates before scales and drawing; inverse picking uses the same choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Orientation {
    Vertical,
    Horizontal,
}
impl Orientation {
    pub(crate) fn apply(self, series: &mut [crate::Series]) {
        if self == Self::Horizontal {
            for s in series {
                for p in &mut s.points {
                    p.swap(0, 1);
                }
                for errors in &mut s.errors {
                    errors.swap(0, 1);
                }
            }
        }
    }
}
