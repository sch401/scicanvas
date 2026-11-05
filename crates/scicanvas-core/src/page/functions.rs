//! Runtime function curves have no worksheet mapping or hidden columns.
use super::*;
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionCurve {
    pub expr: String,
    pub points: u32,
    pub style: CurveStyle,
}
impl FunctionCurve {
    pub fn new(expr: impl Into<String>) -> Self {
        Self {
            expr: expr.into(),
            points: 500,
            style: CurveStyle::default(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(2..=100_000).contains(&self.points) {
            return Err("函数采样点数为 2–100000".into());
        }
        let _ = crate::formula::compile_x(&self.expr)?;
        Ok(())
    }
}
impl Layer {
    pub(crate) fn append_functions(&self, plots: &mut Vec<PreparedPlot>) {
        if self.functions.is_empty() {
            return;
        }
        let xs: Vec<_> = plots
            .iter()
            .filter(|p| !p.series.line_only)
            .flat_map(|p| p.series.points.iter().map(|p| p[0]))
            .filter(|v| self.effective_x_scale().domain_ok(*v))
            .collect();
        let (lo, hi) = if self.x_auto && !xs.is_empty() {
            (
                xs.iter().copied().fold(f64::INFINITY, f64::min),
                xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            )
        } else {
            (self.x_min, self.x_max)
        };
        let mut y_sides = std::array::from_fn::<_, 2, _>(|right| {
            let (auto, min) = if right == 0 {
                (self.y_auto, self.y_min)
            } else {
                (self.y2_auto, self.y2_min)
            };
            if auto {
                plots
                    .iter()
                    .filter(|p| (p.style.y_axis == crate::coord::YAxis::Right) == (right == 1))
                    .flat_map(|p| p.series.points.iter().map(|p| p[1]))
                    .find(|v| self.effective_y_scale().domain_ok(*v))
                    .map(f64::signum)
            } else {
                Some(min.signum())
            }
        });
        for (i, function) in self.functions.iter().enumerate() {
            let Ok(eval) = crate::formula::compile_x(&function.expr) else {
                continue;
            };
            let n = function.points as usize;
            let mut points = Vec::new();
            let mut rows = Vec::new();
            for row in 0..n {
                let f = row as f64 / (n - 1) as f64;
                let scale = self.effective_x_scale();
                let x = if scale == crate::ticks::AxisScale::Log10 {
                    (lo.ln() + (hi.ln() - lo.ln()) * f).exp()
                } else {
                    scale.inverse(scale.forward(lo) + (scale.forward(hi) - scale.forward(lo)) * f)
                };
                let y = eval(x);
                if scale.domain_ok(x) && self.effective_y_scale().domain_ok(y) {
                    if self.effective_y_scale() == crate::ticks::AxisScale::Reciprocal
                        && *y_sides[usize::from(function.style.right_axis)].get_or_insert(y.signum()) != y.signum()
                    {
                        continue;
                    }
                    points.push([x, y]);
                    rows.push(row);
                }
            }
            let key = format!("fn:{i}");
            let len = points.len();
            let color = function.style.color.map_or_else(
                || {
                    if self.color_map == crate::colormap::ColorMap::Origin {
                        crate::SeriesColor::Palette(plots.len())
                    } else {
                        crate::SeriesColor::Rgb(
                            self.color_map
                                .sample(i as f64 / self.functions.len().saturating_sub(1).max(1) as f64),
                        )
                    }
                },
                |[r, g, b]| crate::SeriesColor::Rgb((r, g, b)),
            );
            let series = crate::Series {
                key: key.clone(),
                name: if function.style.legend_label.is_empty() {
                    format!("y = {}", function.expr)
                } else {
                    function.style.legend_label.clone()
                },
                points,
                rows,
                bases: vec![0.; len],
                errors: vec![[None, None]; len],
                point_styles: vec![crate::PointStyle::default(); len],
                line_only: true,
                source: None,
            };
            let style = crate::SeriesStyle {
                kind: crate::PlotType::Line,
                color,
                symbol: crate::Symbol::Circle,
                bar_index: 0,
                bar_count: 1,
                line_width: function.style.line_width.unwrap_or(self.line_width),
                symbol_size: function.style.symbol_size.unwrap_or(self.symbol_size),
                y_axis: if function.style.right_axis {
                    crate::coord::YAxis::Right
                } else {
                    crate::coord::YAxis::Left
                },
            };
            // This transient description satisfies PreparedPlot; only FunctionCurve is persisted.
            let plot = crate::plotspec::Plot {
                key: key.clone(),
                mapping: crate::plotspec::CurveMapping {
                    y: crate::plotspec::DataRef::new(0, &key),
                    x: None,
                    base: None,
                    rows: None,
                    line_only: true,
                },
                style: Some(function.style.clone()),
                x_override: false,
            };
            plots.push(PreparedPlot {
                plot,
                series,
                style,
                paths: Vec::new(),
                box_stats: None,
                histogram_width: None,
                offset: [0.; 2],
            });
        }
    }
    pub fn set_source_rows(
        &mut self,
        source: &crate::plotspec::DataRef,
        rows: Option<(usize, usize)>,
    ) -> Result<(), String> {
        if rows.is_some_and(|(a, b)| a > b || b == usize::MAX) {
            return Err("行范围起点应不大于终点".into());
        }
        let key = self.plot_key(source).ok_or("找不到曲线")?;
        self.edit_draft(|draft| {
            draft.row_by_y.insert(key, rows);
        });
        Ok(())
    }
}
impl crate::Project {
    pub fn plot_function(
        &mut self,
        curve: FunctionCurve,
        range: (f64, f64),
        target: Option<(u64, usize)>,
    ) -> Result<u64, String> {
        curve.validate()?;
        if !range.0.is_finite() || !range.1.is_finite() || range.0 >= range.1 {
            return Err("函数 X 范围应为递增有限数".into());
        }
        if let Some((id, index)) = target {
            let layer = self
                .graph_mut(id)
                .and_then(|g| g.page.layers_mut().get_mut(index))
                .ok_or("找不到图层")?;
            layer.functions.push(curve);
            return Ok(id);
        }
        let id = self.alloc_id();
        let mut graph = crate::Graph::new(id, self.unique_name("Function"));
        let layer = graph.first_layer_mut();
        layer.x_auto = false;
        layer.x_min = range.0;
        layer.x_max = range.1;
        layer.functions.push(curve);
        self.graphs.push(graph);
        Ok(id)
    }
}
