//! 基线计算结果的项目写入与附图。
use super::*;

impl Project {
    /// 扣除基线：每条曲线在 Y 右边写两列（基线、扣除基线后的 Y）。来源是图形时，基线也加到图里。
    /// 返回 (基线列, 扣除后列) 标识。
    pub fn subtract_baseline(
        &mut self,
        src: &Source,
        method: crate::baseline::BaselineMethod,
    ) -> Result<Vec<(String, String)>, String> {
        let (r, base) = self.per_item_column(src, OpKind::Baseline { method }, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            (
                crate::baseline::baseline(xs, ys, method),
                format!("{} 基线", y.display_name()),
                y.units.clone(),
                method.label(),
            )
        })?;
        let mut made = vec![];
        for (item, bkey) in &base {
            let book = self.book(item.book_id).unwrap();
            let (_, ys, idx) = get_xy(book, item);
            let bv = book.column(bkey).unwrap().values();
            let y = book.column(&item.y).unwrap();
            let mut c = Column::new("", Role::Y);
            c.long_name = format!("{} 扣除基线", y.display_name());
            c.units = y.units.clone();
            c.comment = method.label();
            c.op = Some(Operation {
                kind: OpKind::BaselineSubtracted { method },
                y: item.y.clone(),
                x: item.x.clone(),
                rows: item.rows,
                visible_only: item.visible_only,
                mode: RecalcMode::Auto,
            });
            c.data = vec![String::new(); book.nrows()].into();
            for (k, &row) in idx.iter().enumerate() {
                let v = ys[k] - bv.get(row).copied().unwrap_or(f64::NAN);
                if v.is_finite() {
                    c.data.set_text(row, fmt_num(v));
                }
            }
            while c.data.last().is_some_and(|s| s.is_empty()) {
                c.data.pop();
            }
            let key = self
                .write_result(item, vec![c], ResultLink::None)?
                .columns
                .pop()
                .unwrap();
            made.push((bkey.clone(), key));
        }
        if let Some(gid) = r.graph_id {
            for (item, key) in &base {
                self.bind_results(item, std::slice::from_ref(key), ResultLink::Layer(gid, r.layer_index))?;
            }
        }
        Ok(made)
    }
}
