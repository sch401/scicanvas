use super::*;

impl Dialog {
    pub(super) fn show_fit_report(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
    ) -> Action {
        let Self::FitReport(reports) = self else { unreachable!() };
        let mut action = Action::None;
        window(ctx, "拟合结果").open(open).show(ctx, |ui| {
            egui::ScrollArea::both()
                .max_height((ctx.available_rect().height() - 220.0).clamp(100.0, 480.0))
                .show(ui, |ui| {
                    for (ri, r) in reports.iter().enumerate() {
                        ui.label(RichText::new(format!("{} · {}", r.name, r.kind)).strong());
                        table_grid(ui, ("fit-grid", ri), &fit_param_rows(r, false), pal);
                        ui.add_space(4.0);
                        let mut stats = vec![
                            ("R²".to_string(), fmt_num(r.fit.r2)),
                            ("调整后 R²".to_string(), fmt_num(r.fit.adj_r2)),
                        ];
                        if let Some(pr) = r.fit.pearson_r() {
                            stats.push(("Pearson r".into(), fmt_num(pr)));
                        }
                        stats.push(("残差平方和".into(), fmt_num(r.fit.rss)));
                        stats.push(("数据点数 N".into(), r.fit.n.to_string()));
                        let stats: Vec<Vec<String>> = stats.into_iter().map(|(k, v)| vec![k, v]).collect();
                        table_grid(
                            ui,
                            ("fit-stats", ri),
                            &[vec!["统计量".into(), "值".into()]]
                                .into_iter()
                                .chain(stats)
                                .collect::<Vec<_>>(),
                            pal,
                        );
                        ui.add_space(4.0);
                        ui.label(RichText::new("方差分析").color(pal.weak));
                        table_grid(ui, ("fit-anova", ri), &fit_anova_rows(r), pal);
                        ui.add_space(4.0);
                        ui.label(RichText::new(r.fit.equation()).monospace());
                        ui.add_space(10.0);
                    }
                });
            ui.label(
                RichText::new("拟合曲线和逐点残差已写入工作簿，拟合曲线加到图形中。")
                    .small()
                    .color(pal.weak),
            );
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                center_buttons(ui, 3);
                if ui.add(plain("残差 Q-Q")).clicked() {
                    action = Action::ResidualQQ(reports.clone());
                }
                if ui.add(plain("残差图")).clicked() {
                    action = Action::ResidualPlot(reports.clone());
                }
                if ui.add(plain("置信/预测带")).on_hover_text("95%").clicked() {
                    action = Action::FitBands(reports.clone());
                }
            });
            ui.horizontal_wrapped(|ui| {
                center_buttons(ui, 3);
                if ui
                    .add(plain("复制结果"))
                    .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                    .clicked()
                {
                    ui.ctx().copy_text(fit_text(reports));
                }
                if ui.add(plain("保存为 Note")).clicked() {
                    action = Action::SaveNote("拟合结果".into(), fit_markdown(reports));
                }
                if ui.add(primary(ui, "关闭")).clicked() {
                    action = Action::Close;
                }
            });
        });
        action
    }
}

/// 线性/多项式参数表：参数、值、标准误、t、p。`exact` 为真时数值保留全部位数（复制到 Excel 用）。
fn fit_param_rows(r: &FitReport, exact: bool) -> Vec<Vec<String>> {
    let num = |v: f64| if exact { format!("{v}") } else { fmt_num(v) };
    let linear = r.fit.coef.len() == 2;
    let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
    for (k, ((c, se), (t, p))) in r.fit.coef.iter().zip(&r.fit.se).zip(r.fit.t_tests()).enumerate() {
        let name = match (linear, k) {
            (true, 0) => "截距 A".to_string(),
            (true, _) => "斜率 B".to_string(),
            (false, 0) => "B0（常数项）".to_string(),
            _ => format!("B{k}"),
        };
        rows.push(vec![name, num(*c), num(*se), num(t), num(p)]);
    }
    rows
}

/// 拟合方差分析表：来源、自由度、平方和、均方、F、p。
fn fit_anova_rows(r: &FitReport) -> Vec<Vec<String>> {
    let a = r.fit.anova();
    vec![
        ["来源", "自由度", "平方和", "均方", "F", "p"]
            .map(String::from)
            .to_vec(),
        vec![
            "回归".into(),
            fmt_num(a.df_reg),
            fmt_num(a.ss_reg),
            fmt_num(a.ms_reg()),
            fmt_num(a.f),
            fmt_num(a.p),
        ],
        vec![
            "残差".into(),
            fmt_num(a.df_err),
            fmt_num(a.ss_err),
            fmt_num(a.ms_err()),
            String::new(),
            String::new(),
        ],
        vec![
            "总计".into(),
            fmt_num(a.df_tot),
            fmt_num(a.ss_tot),
            String::new(),
            String::new(),
            String::new(),
        ],
    ]
}

fn fit_markdown(reports: &[FitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let stats = [
                ("R²", fmt_num(r.fit.r2)),
                ("调整后 R²", fmt_num(r.fit.adj_r2)),
                ("残差平方和", fmt_num(r.fit.rss)),
                ("N", r.fit.n.to_string()),
            ]
            .map(|(k, v)| (k.to_string(), v));
            format!(
                "## {} — {}\n\n`{}`\n\n{}\n{}\n### 方差分析\n\n{}",
                md_cell(&r.name),
                r.kind,
                r.fit.equation(),
                md_table(&fit_param_rows(r, false)),
                md_pairs(["统计量", "值"], &stats),
                md_table(&fit_anova_rows(r))
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn fit_text(reports: &[FitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let mut lines = vec![format!("{}\t{}", r.name, r.kind), r.fit.equation()];
            lines.extend(fit_param_rows(r, true).iter().map(|row| row.join("\t")));
            lines.push(format!("R²\t{}", r.fit.r2));
            lines.push(format!("调整后 R²\t{}", r.fit.adj_r2));
            lines.push(format!("残差平方和\t{}", r.fit.rss));
            lines.push(format!("N\t{}", r.fit.n));
            lines.push(String::new());
            lines.extend(fit_anova_rows(r).iter().map(|row| row.join("\t")));
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
