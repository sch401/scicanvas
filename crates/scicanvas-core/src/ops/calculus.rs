//! 微分、SG 求导与积分。
use super::*;

#[derive(Clone, Debug)]
pub struct AreaReport {
    pub name: String,
    pub area: f64,
    pub abs_area: f64,
    pub n: usize,
}

impl Project {
    /// 数值微分，结果写成新列 d(Y)/dX。
    pub fn differentiate(&mut self, src: &Source) -> Result<Vec<String>, String> {
        let (_, made) = self.per_item_column(src, OpKind::Derivative, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            let xu = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let units = if !y.units.is_empty() && !xu.is_empty() {
                format!("{}/{}", y.units, xu)
            } else {
                String::new()
            };
            (
                analysis::derivative(xs, ys),
                format!("d({})/dX", y.display_name()),
                units,
                "中心差分".into(),
            )
        })?;
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }

    /// Savitzky–Golay 平滑求导（1 或 2 阶导数），结果写成新列。
    pub fn differentiate_sg(
        &mut self,
        src: &Source,
        window: usize,
        order: usize,
        deriv: usize,
    ) -> Result<Vec<String>, String> {
        let deriv = deriv.clamp(1, 2);
        let (_, made) = self.per_item_column(
            src,
            OpKind::SgDerivative { window, order, deriv },
            |book, item, xs, ys| {
                let y = book.column(&item.y).unwrap();
                let xu = item
                    .x
                    .as_deref()
                    .and_then(|k| book.column(k))
                    .map(|c| c.units.clone())
                    .unwrap_or_default();
                let (name, units) = if deriv == 1 {
                    (format!("d({})/dX", y.display_name()), format!("{}/{}", y.units, xu))
                } else {
                    (format!("d²({})/dX²", y.display_name()), format!("{}/{}²", y.units, xu))
                };
                let units = if y.units.is_empty() || xu.is_empty() {
                    String::new()
                } else {
                    units
                };
                (
                    analysis::sg_derivative(xs, ys, window, order, deriv),
                    name,
                    units,
                    format!("Savitzky–Golay，{} 点 {} 阶多项式", window.max(3) | 1, order),
                )
            },
        )?;
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }

    /// 梯形法累积积分，结果写成新列，并返回每组数据的总面积。
    pub fn integrate(&mut self, src: &Source) -> Result<(Vec<String>, Vec<AreaReport>), String> {
        let mut areas = vec![];
        let (_, made) = self.per_item_column(src, OpKind::Integral, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            let (cum, abs) = analysis::cumulative_trapezoid(xs, ys);
            let total = *cum.last().unwrap_or(&0.0);
            areas.push(AreaReport {
                name: y.display_name().to_string(),
                area: total,
                abs_area: abs,
                n: xs.len(),
            });
            let xu = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let units = if !y.units.is_empty() && !xu.is_empty() {
                format!("{}·{}", y.units, xu)
            } else {
                String::new()
            };
            (
                cum,
                format!("∫{} dX", y.display_name()),
                units,
                format!("梯形法累积积分，总面积 {}", fmt_num(total)),
            )
        })?;
        if made
            .first()
            .is_some_and(|(first, _)| made.iter().any(|(item, _)| item.book_id != first.book_id))
        {
            for (area, (item, _)) in areas.iter_mut().zip(&made) {
                area.name = format!("{}：{}", self.book(item.book_id).unwrap().name, area.name);
            }
        }
        Ok((made.into_iter().map(|(_, k)| k).collect(), areas))
    }
}
