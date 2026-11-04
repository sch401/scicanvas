use super::*;
impl Project {
    pub(super) fn add_peak_components(
        &mut self,
        item: &Item,
        resolved: &Resolved,
        fit: &crate::nlfit::NlFit,
        grid: &[f64],
        x: &str,
    ) -> Result<Vec<String>, String> {
        let crate::nlfit::FitFunction::PeakSum(peak) = &fit.model else {
            return Ok(Vec::new());
        };
        let (gid, index) = self.graph_for_result(resolved, item)?;
        let mut keys = Vec::new();
        let units = self.book(item.book_id).unwrap().column(&item.y).unwrap().units.clone();
        for i in 0..usize::from(peak.count) {
            let mut col = Column::new("", Role::Y);
            col.long_name = format!("{} 分量 {}", fit.model.name(), i + 1);
            col.units = units.clone();
            col.independent_rows = true;
            col.comment = "多峰拟合分量（不含基线），与总曲线共用重算".into();
            col.data = grid
                .iter()
                .map(|x| fmt_num(peak.component(*x, &fit.params, i)))
                .collect();
            let key = self.insert_result_column(item.book_id, &item.y, col).unwrap();
            self.attach_result(gid, index, item, &key, Some(x))?;
            keys.push(key);
        }
        Ok(keys)
    }
}
impl NlFitReport {
    pub fn peak_rows(&self) -> Vec<Vec<String>> {
        let crate::nlfit::FitFunction::PeakSum(peak) = &self.fit.model else {
            return Vec::new();
        };
        let mut rows = vec![vec![
            "峰".into(),
            "中心".into(),
            "高度".into(),
            "面积".into(),
            "FWHM".into(),
            "SE(中心)".into(),
            "SE(高度)".into(),
            "SE(面积)".into(),
            "SE(FWHM)".into(),
        ]];
        for i in 0..usize::from(peak.count) {
            let mut row = vec![(i + 1).to_string()];
            row.extend(peak.metrics(&self.fit.params, i).into_iter().map(fmt_num));
            row.extend(peak.metric_errors(&self.fit, i).into_iter().map(fmt_num));
            rows.push(row);
        }
        rows
    }
}
