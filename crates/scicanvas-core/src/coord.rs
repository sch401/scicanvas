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
