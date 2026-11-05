//! Geometric hatches are shared by screen and all exports; no SVG-only patterns.
use super::{Item, Prim, Rgb};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PatternKind {
    Hatch45,
    Hatch135,
    Cross,
    Horizontal,
    Vertical,
    Dots,
}
impl PatternKind {
    pub const ALL: [Self; 6] = [
        Self::Hatch45,
        Self::Hatch135,
        Self::Cross,
        Self::Horizontal,
        Self::Vertical,
        Self::Dots,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Hatch45 => "45° 斜线",
            Self::Hatch135 => "135° 斜线",
            Self::Cross => "交叉线",
            Self::Horizontal => "横线",
            Self::Vertical => "竖线",
            Self::Dots => "圆点",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct FillPattern {
    pub kind: PatternKind,
    pub spacing_pt: f32,
    pub width_pt: f32,
    pub color: Option<[u8; 3]>,
}
impl Default for FillPattern {
    fn default() -> Self {
        Self {
            kind: PatternKind::Hatch45,
            spacing_pt: 6.,
            width_pt: 0.5,
            color: None,
        }
    }
}
impl FillPattern {
    pub fn validate(&self) -> Result<(), String> {
        if !self.spacing_pt.is_finite() || self.spacing_pt <= 0. || !self.width_pt.is_finite() || self.width_pt < 0. {
            return Err("图案间距应为有限正数，线宽应为有限非负数".into());
        }
        Ok(())
    }
}
fn inside(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}
fn line_interval(direction: [f64; 2], normal: [f64; 2], c: f64, rect: [f64; 4]) -> Option<(f64, f64)> {
    let mut low = f64::NEG_INFINITY;
    let mut high = f64::INFINITY;
    for i in 0..2 {
        let origin = normal[i] * c;
        let a = rect[i];
        let b = a + rect[i + 2];
        if direction[i] == 0. {
            if origin < a || origin > b {
                return None;
            }
        } else {
            let x = (a - origin) / direction[i];
            let y = (b - origin) / direction[i];
            low = low.max(x.min(y));
            high = high.min(x.max(y));
        }
    }
    (high > low).then_some((low, high))
}
fn geometry(polygons: &[&[[f64; 2]]], pattern: &FillPattern, fallback: Rgb, clip: Option<[f64; 4]>) -> Prim {
    let color = pattern.color.map_or(fallback, |[r, g, b]| (r, g, b));
    let width = f64::from(pattern.width_pt) * 96. / 72.;
    let empty = || Prim::Segments {
        segs: Vec::new(),
        color,
        width,
        dashed: false,
    };
    if pattern.validate().is_err() || polygons.is_empty() {
        return empty();
    }
    let spacing = f64::from(pattern.spacing_pt) * 96. / 72.;
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in polygons.iter().flat_map(|poly| poly.iter()) {
        for i in 0..2 {
            if !p[i].is_finite() {
                return empty();
            }
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    if let Some([x, y, w, h]) = clip {
        min[0] = min[0].max(x);
        min[1] = min[1].max(y);
        max[0] = max[0].min(x + w);
        max[1] = max[1].min(y + h);
    }
    if min[0] >= max[0] || min[1] >= max[1] {
        return empty();
    }
    let bounds = [[min[0], min[1]], [max[0], min[1]], [max[0], max[1]], [min[0], max[1]]];
    if pattern.kind == PatternKind::Dots {
        let mut pts = Vec::new();
        for iy in (min[1] / spacing - 0.5).ceil() as i64..=(max[1] / spacing - 0.5).floor() as i64 {
            for ix in (min[0] / spacing - 0.5).ceil() as i64..=(max[0] / spacing - 0.5).floor() as i64 {
                let p = [(ix as f64 + 0.5) * spacing, (iy as f64 + 0.5) * spacing];
                if polygons.iter().any(|poly| inside(p, poly)) {
                    pts.push(p);
                }
            }
        }
        return Prim::Symbols {
            sym: crate::Symbol::Circle,
            pts,
            r: width / 2.,
            color,
        };
    }
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let directions = match pattern.kind {
        PatternKind::Hatch45 => vec![[s, -s]],
        PatternKind::Hatch135 => vec![[s, s]],
        PatternKind::Cross => vec![[s, -s], [s, s]],
        PatternKind::Horizontal => vec![[1., 0.]],
        PatternKind::Vertical => vec![[0., 1.]],
        PatternKind::Dots => unreachable!(),
    };
    let mut segs = Vec::new();
    // ponytail: O(scanlines × polygon edges); index edges if dense imported meshes become slow.
    for direction in directions {
        let normal = [-direction[1], direction[0]];
        let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
        let lo = bounds.iter().map(|p| dot(normal, *p)).fold(f64::INFINITY, f64::min);
        let hi = bounds.iter().map(|p| dot(normal, *p)).fold(f64::NEG_INFINITY, f64::max);
        for line in (lo / spacing).floor() as i64..=(hi / spacing).ceil() as i64 {
            let c = line as f64 * spacing;
            let interval = clip.map_or(Some((f64::NEG_INFINITY, f64::INFINITY)), |rect| {
                line_interval(direction, normal, c, rect)
            });
            let Some((clip_lo, clip_hi)) = interval else { continue };
            let mut spans = Vec::new();
            for poly in polygons {
                let mut intersections = Vec::new();
                for i in 0..poly.len() {
                    let a = poly[i];
                    let b = poly[(i + 1) % poly.len()];
                    let da = dot(normal, a);
                    let db = dot(normal, b);
                    if (da <= c && db > c) || (db <= c && da > c) {
                        let t = (c - da) / (db - da);
                        intersections.push(dot(direction, [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]));
                    }
                }
                intersections.sort_by(f64::total_cmp);
                for pair in intersections.as_chunks::<2>().0 {
                    if pair[1] > pair[0] {
                        spans.push((pair[0], pair[1]));
                    }
                }
            }
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged: Vec<(f64, f64)> = Vec::new();
            for (a, b) in spans {
                if let Some(last) = merged.last_mut().filter(|last| a <= last.1 + 1e-9) {
                    last.1 = last.1.max(b);
                } else {
                    merged.push((a, b));
                }
            }
            for (a, b) in merged {
                let a = a.max(clip_lo);
                let b = b.min(clip_hi);
                if b <= a {
                    continue;
                }
                let at = |t| [direction[0] * t + normal[0] * c, direction[1] * t + normal[1] * c];
                let a = at(a);
                let b = at(b);
                segs.push([a[0], a[1], b[0], b[1]]);
            }
        }
    }
    Prim::Segments {
        segs,
        color,
        width,
        dashed: false,
    }
}
pub fn hatch_rect(rect: [f64; 4], pattern: &FillPattern, color: Rgb) -> Prim {
    let [x, y, w, h] = rect;
    if w <= 0. || h <= 0. {
        return Prim::Segments {
            segs: Vec::new(),
            color,
            width: 0.,
            dashed: false,
        };
    }
    let poly = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
    geometry(&[&poly], pattern, color, None)
}
pub fn hatch_polygon(poly: &[[f64; 2]], pattern: &FillPattern, color: Rgb) -> Prim {
    geometry(&[poly], pattern, color, None)
}
fn hatch_mesh(points: &[[f64; 2]], pattern: &FillPattern, color: Rgb, clip: Option<[f64; 4]>) -> Prim {
    let polygons: Vec<_> = points
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| p.as_slice())
        .filter(|p| ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0])).abs() > 0.)
        .collect();
    geometry(&polygons, pattern, color, clip)
}
fn ink((r, g, b): Rgb) -> Rgb {
    if u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 >= 128_000 {
        (0, 0, 0)
    } else {
        (255, 255, 255)
    }
}
pub(crate) fn overlay(items: &mut Vec<Item>, start: usize, pattern: &FillPattern) {
    let originals: Vec<_> = items.drain(start..).collect();
    for mut item in originals {
        let clip = item.clip;
        let (mut hatch, mut outline) = (None, None);
        match &mut item.prim {
            Prim::Rect {
                x,
                y,
                w,
                h,
                fill: Some((color, _)),
                stroke,
            } => {
                let poly = [[*x, *y], [*x + *w, *y], [*x + *w, *y + *h], [*x, *y + *h]];
                hatch = Some(geometry(&[&poly], pattern, ink(*color), clip));
                if let Some(stroke) = stroke.take() {
                    outline = Some(Prim::Rect {
                        x: *x,
                        y: *y,
                        w: *w,
                        h: *h,
                        fill: None,
                        stroke: Some(stroke),
                    });
                }
            }
            Prim::Triangles { pts, color, .. } => hatch = Some(hatch_mesh(pts, pattern, ink(*color), clip)),
            _ => {}
        }
        items.push(item);
        if let Some(prim) = hatch {
            items.push(Item { prim, clip });
        }
        if let Some(prim) = outline {
            items.push(Item { prim, clip });
        }
    }
}
