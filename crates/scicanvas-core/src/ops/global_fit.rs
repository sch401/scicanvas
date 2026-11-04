use super::*;
use crate::nlfit::{self, FitFunction, GlobalFit, GlobalSpec, ParamLimits};

pub struct GlobalFitReport {
    pub graph: u64,
    pub names: Vec<String>,
    pub fit: GlobalFit,
    pub shared: Vec<bool>,
}
impl GlobalFitReport {
    pub fn rows(&self) -> Vec<Vec<String>> {
        let params = self.fit.per_group[0].model.params();
        let mut header = vec!["数据集".into()];
        header.extend(params.iter().map(|name| (*name).to_owned()));
        header.extend(params.iter().map(|name| format!("SE({name})")));
        let mut rows = vec![header];
        for (name, fit) in self.names.iter().zip(&self.fit.per_group) {
            let mut row = vec![name.clone()];
            row.extend(fit.params.iter().map(|v| fmt_num(*v)));
            row.extend(
                fit.se
                    .iter()
                    .enumerate()
                    .map(|(i, v)| if fit.fixed[i] { "固定".into() } else { fmt_num(*v) }),
            );
            rows.push(row);
        }
        rows
    }
    pub fn note(&self) -> String {
        let fit = &self.fit.joint;
        let origin = if let FitFunction::SineDamp { origin } = self.fit.per_group[0].model {
            format!(" 固定时间原点 x₀={}。", fmt_num(origin))
        } else {
            String::new()
        };
        let names = self.fit.per_group[0].model.params();
        let shared = names
            .iter()
            .zip(&self.shared)
            .filter(|(_, shared)| **shared)
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join("、");
        format!("模型：{}。共享参数：{}；有效 N={}，联合残差自由度={}，{}={}。\n一次性结果，暂不支持自动重算。标准误取自联合协方差；各组 R²：{}。{}",fit.model.name(),if shared.is_empty(){"无"}else{&shared},fit.n,fit.dof,if fit.weighted {"χ²"}else{"RSS"},fmt_num(fit.rss),self.fit.per_group.iter().map(|f|fmt_num(f.r2)).collect::<Vec<_>>().join("、"),origin + &fit.warnings.join("\n"))
    }
    fn markdown(&self) -> String {
        let rows = self.rows();
        let cell = |value: &str| value.replace('|', "\\|").replace('\n', " ");
        let line = |row: &Vec<String>| format!("| {} |", row.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | "));
        let mut lines = vec![
            "## 全局拟合".into(),
            String::new(),
            line(&rows[0]),
            format!("| {} |", vec!["---"; rows[0].len()].join(" | ")),
        ];
        lines.extend(rows[1..].iter().map(line));
        lines.push(String::new());
        lines.push(self.note());
        lines.join("\n")
    }
}

impl Project {
    pub fn global_fit(
        &mut self,
        src: &Source,
        model: FitFunction,
        shared: &[bool],
        limits: &[ParamLimits],
        constraints: &str,
        weight: FitWeight,
    ) -> Result<GlobalFitReport, String> {
        let resolved = self.resolve(src)?;
        let mut groups = Vec::new();
        let mut names = Vec::new();
        for item in &resolved.items {
            let book = self.book(item.book_id).ok_or("找不到工作簿")?;
            let (x, y, rows) = get_xy(book, item);
            let sigma = if weight == FitWeight::YError {
                Some(y_errors(book, &item.y, &rows)?)
            } else {
                None
            };
            groups.push((x, y, sigma));
            names.push(resolved.name(book, item));
        }
        let fit = nlfit::fit_global(
            GlobalSpec {
                base: model,
                groups,
                shared: shared.to_vec(),
            },
            None,
            limits,
            constraints,
        )?;
        if !fit.joint.converged {
            return Err("全局拟合尚未收敛，请检查共享方式或参数限制".into());
        }
        let mut graph = None;
        for ((item, local), name) in resolved.items.iter().zip(&fit.per_group).zip(&names) {
            let book = self.book(item.book_id).unwrap();
            let units = book.column(&item.y).unwrap().units.clone();
            let x_units = item
                .x
                .as_ref()
                .and_then(|k| book.column(k))
                .map_or(String::new(), |c| c.units.clone());
            let x = interpolate::grid(local.x_min, local.x_max, 200);
            let mut cx = Column::new("", Role::X);
            cx.long_name = format!("{name} 全局拟合 X");
            cx.units = x_units;
            cx.data = x.iter().map(|x| fmt_num(*x)).collect();
            let mut cy = Column::new("", Role::Y);
            cy.long_name = format!("{name} 全局拟合");
            cy.units = units;
            cy.derived_from = Some(item.y.clone());
            cy.comment = "全局拟合一次性结果，源数据修改后请重新运行".into();
            cy.data = local.sample(&x).into_iter().map(fmt_num).collect();
            graph = self
                .write_result(
                    item,
                    vec![cx, cy],
                    ResultLink::Auto {
                        resolved: &resolved,
                        x: 0,
                        y: 1,
                    },
                )?
                .graph
                .or(graph);
        }
        let report = GlobalFitReport {
            graph: graph.ok_or("没有全局拟合曲线")?,
            names,
            fit,
            shared: shared.to_vec(),
        };
        self.add_note("全局拟合", report.markdown());
        Ok(report)
    }
}
