use super::{valid_position, Page};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShapeKind {
    Line,
    Arrow,
    Rect,
    Ellipse,
}
impl ShapeKind {
    pub const ALL: [Self; 4] = [Self::Line, Self::Arrow, Self::Rect, Self::Ellipse];
    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "直线",
            Self::Arrow => "箭头",
            Self::Rect => "矩形",
            Self::Ellipse => "椭圆",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShapeObject {
    pub id: u64,
    pub kind: ShapeKind,
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub stroke: [u8; 3],
    pub width_pt: f64,
    pub dashed: bool,
    pub fill: Option<([u8; 3], f64)>,
    pub head_pt: f64,
}
impl ShapeObject {
    pub fn new(kind: ShapeKind, a: [f64; 2], b: [f64; 2]) -> Self {
        Self {
            id: 0,
            kind,
            a,
            b,
            stroke: [0, 0, 0],
            width_pt: 1.,
            dashed: false,
            fill: None,
            head_pt: 6.,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !valid_position(self.a)
            || !valid_position(self.b)
            || !self.width_pt.is_finite()
            || self.width_pt < 0.
            || !self.head_pt.is_finite()
            || self.head_pt < 0.
            || self
                .fill
                .is_some_and(|(_, alpha)| !alpha.is_finite() || !(0.0..=1.0).contains(&alpha))
        {
            return Err("图形位置应在页面内，线宽、箭头大小和透明度应为有效数值".into());
        }
        Ok(())
    }
    pub fn origin(&self) -> [f64; 2] {
        [self.a[0].min(self.b[0]), self.a[1].min(self.b[1])]
    }
    pub fn span(&self) -> [f64; 2] {
        [(self.a[0] - self.b[0]).abs(), (self.a[1] - self.b[1]).abs()]
    }
    pub fn clamp_origin(&self, position: [f64; 2]) -> [f64; 2] {
        let span = self.span();
        std::array::from_fn(|i| position[i].clamp(0., (1. - span[i]).max(0.)))
    }
    pub fn move_to(&mut self, position: [f64; 2]) {
        let position = self.clamp_origin(position);
        let span = self.span();
        for i in 0..2 {
            let forward = self.a[i] <= self.b[i];
            self.a[i] = position[i] + if forward { 0. } else { span[i] };
            self.b[i] = position[i] + if forward { span[i] } else { 0. };
        }
    }
}
impl Page {
    pub fn add_shape(&mut self, kind: ShapeKind, a: [f64; 2], b: [f64; 2]) -> Result<u64, String> {
        let mut shape = ShapeObject::new(kind, a, b);
        shape.validate()?;
        let next = self.next_shape_id.checked_add(1).ok_or("图形编号已用尽")?;
        shape.id = self.next_shape_id;
        self.next_shape_id = next;
        let id = shape.id;
        self.shapes.push(shape);
        Ok(id)
    }
    pub fn shape(&self, id: u64) -> Option<&ShapeObject> {
        self.shapes.iter().find(|s| s.id == id)
    }
    pub fn shape_mut(&mut self, id: u64) -> Option<&mut ShapeObject> {
        self.shapes.iter_mut().find(|s| s.id == id)
    }
    pub fn remove_shape(&mut self, id: u64) {
        self.shapes.retain(|s| s.id != id);
    }
    pub(super) fn validate_shapes(&self) -> Result<(), String> {
        if self.next_shape_id == 0 {
            return Err("图形编号无效".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for shape in &self.shapes {
            shape.validate()?;
            if shape.id == 0 || shape.id >= self.next_shape_id || !ids.insert(shape.id) {
                return Err("图形编号无效或重复".into());
            }
        }
        Ok(())
    }
}
