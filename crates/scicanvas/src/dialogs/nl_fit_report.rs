use super::nl_fit::CustomFitAction;
use super::*;

impl Dialog {
    pub(super) fn show_nl_fit_report(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
        project: &scicanvas_core::Project,
    ) -> Action {
        let Self::NlFitReport(reports) = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "非线性拟合结果").open(open).show(ctx, |ui| {
            egui::ScrollArea::both()
                .max_height((ctx.available_rect().height() - 220.0).clamp(100.0, 480.0))
                .show(ui, |ui| {
                    for (ri, r) in reports.iter().enumerate() {
                        let live = project.nlfit_report(r);
                        let r = live.unwrap_or(r);
                        let f = &r.fit;
                        let status = if live.is_none() {
                            "结果列已删除，以下为窗口保留的报告快照".into()
                        } else {
                            report_status(r)
                        };
                        ui.label(RichText::new(status).small().color(pal.weak));
                        ui.label(
                            RichText::new(format!(
                                "{} · 结果列 {} · 报告 #{}",
                                project.name_of(r.book_id).unwrap_or("已删除的工作簿"),
                                r.curve.1,
                                r.id
                            ))
                            .small()
                            .color(pal.weak),
                        );
                        ui.label(RichText::new(format!("{} · {}", r.name, f.model.name())).strong());
                        ui.label(RichText::new(f.model.formula()).monospace().small());
                        for c in &f.constraints {
                            ui.label(RichText::new(format!("约束：{c}")).small().color(pal.weak));
                        }
                        if !f.converged {
                            ui.colored_label(
                                pal.error,
                                format!(
                                    "迭代 {} 次后停止，尚未收敛；可调整初值、参数约束或检查数据",
                                    f.iterations
                                ),
                            );
                        }
                        for warning in &f.warnings {
                            ui.colored_label(pal.warn, warning);
                        }
                        table_grid(ui, ("nlfit-grid", ri), &nlfit_param_rows(f), pal);
                        ui.add_space(4.0);
                        let stats: Vec<Vec<String>> = [vec!["统计量".to_string(), "值".to_string()]]
                            .into_iter()
                            .chain(nlfit_stats(f).into_iter().map(|(k, v)| vec![k.to_string(), v]))
                            .collect();
                        table_grid(ui, ("nlfit-stats", ri), &stats, pal);
                        if live.is_some()
                            && project.nlfit_operation(r).is_ok()
                            && ui.add(plain("重新编辑拟合设置")).clicked()
                        {
                            action = Action::EditNlFit(Box::new(r.clone()));
                        }
                        if live.is_none()
                            && matches!(f.model, scicanvas_core::nlfit::FitFunction::Custom(_))
                            && ui.add(plain("重新编辑函数")).clicked()
                        {
                            let definition = scicanvas_core::nlfit::CustomFitDefinition {
                                name: f.model.name().into(),
                                formula: f.model.formula().into(),
                                parameters: f.model.params().into_iter().map(str::to_owned).collect(),
                                initial: f.params.clone(),
                                constraints: f.constraints.join("\n"),
                            };
                            action = Action::CustomFitLibrary(Box::new(CustomFitAction::Edit(definition)));
                        }

                        ui.add_space(10.0);
                    }
                });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                center_buttons(ui, 1);
                if ui.add(plain("置信/预测带")).on_hover_text("95%").clicked() {
                    action = Action::NlFitBands(reports.clone());
                }
            });
            ui.horizontal_wrapped(|ui| {
                center_buttons(ui, 3);
                if ui
                    .add(plain("复制结果"))
                    .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                    .clicked()
                {
                    ui.ctx().copy_text(nlfit_text(reports));
                }
                if ui.add(plain("保存为 Note")).clicked() {
                    action = Action::SaveNote("非线性拟合结果".into(), nlfit_markdown(reports));
                }
                if ui.add(primary(ui, "关闭")).clicked() {
                    action = Action::Close;
                }
            });
        });
        action
    }
}

/// 标准误文字：固定参数写“固定”，停在上下限上的参数后面注明“（边界）”。
fn nlfit_se(f: &scicanvas_core::nlfit::NlFit, k: usize) -> String {
    if f.fixed[k] {
        "固定".into()
    } else if f.at_bound[k] {
        format!("{}（边界）", fmt_num(f.se[k]))
    } else {
        fmt_num(f.se[k])
    }
}

fn nlfit_stats(f: &scicanvas_core::nlfit::NlFit) -> Vec<(&'static str, String)> {
    vec![
        ("R²", fmt_num(f.r2)),
        ("调整后 R²", fmt_num(f.adj_r2)),
        ("残差平方和", fmt_num(f.rss)),
        ("约化 χ²", fmt_num(f.reduced_chi2)),
        ("数据点数 N", f.n.to_string()),
        ("迭代次数", f.iterations.to_string()),
    ]
}

/// 非线性参数表：参数、值、标准误、t、p（固定参数只写值）。
fn nlfit_param_rows(f: &scicanvas_core::nlfit::NlFit) -> Vec<Vec<String>> {
    let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
    for (k, (name, (t, p))) in f.model.params().iter().zip(f.t_tests()).enumerate() {
        let (t, p) = if f.fixed[k] {
            (String::new(), String::new())
        } else {
            (fmt_num(t), fmt_num(p))
        };
        rows.push(vec![name.to_string(), fmt_num(f.params[k]), nlfit_se(f, k), t, p]);
    }
    rows
}

fn nlfit_markdown(reports: &[NlFitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let f = &r.fit;
            let stats: Vec<(String, String)> = nlfit_stats(f).into_iter().map(|(k, v)| (k.to_string(), v)).collect();
            let constraints: String = f.constraints.iter().map(|c| format!("约束：`{c}`\n\n")).collect();
            let warnings: String = f
                .warnings
                .iter()
                .map(|warning| format!("{}\n\n", md_cell(warning)))
                .collect();
            format!(
                "## {} — {}\n\n`{}`\n\n{}\n\n{}{}{}\n{}",
                md_cell(&r.name),
                f.model.name(),
                f.model.formula(),
                report_status(r),
                warnings,
                constraints,
                md_table(&nlfit_param_rows(f)),
                md_pairs(["统计量", "值"], &stats)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn nlfit_text(reports: &[NlFitReport]) -> String {
    let mut lines = vec![];
    for r in reports {
        let f = &r.fit;
        lines.push(format!("{}\t{}\t{}", r.name, f.model.name(), f.model.formula()));
        lines.push(report_status(r));
        lines.extend(f.warnings.iter().cloned());
        lines.extend(f.constraints.iter().map(|c| format!("约束\t{c}")));
        lines.extend(nlfit_param_rows(f).iter().map(|row| row.join("\t")));
        for (k, v) in nlfit_stats(f) {
            lines.push(format!("{k}\t{v}"));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn report_status(r: &NlFitReport) -> String {
    if let Some(error) = &r.error {
        format!("本次重算失败：{error}。以下为上次成功结果。")
    } else if !r.current {
        if r.fit.converged {
            "上次成功结果 · 来源或设置已改变，等待重算".into()
        } else {
            "未收敛：无法继续降低残差，请检查初值或模型".into()
        }
    } else if r.fit.status == scicanvas_core::nlfit::FitStatus::PrecisionLimit {
        "已达到浮点精度极限，结果可用".into()
    } else {
        "当前有效结果 · 已收敛".into()
    }
}

impl Dialog {
    pub(crate) fn sync_fit_reports(&mut self, project: &scicanvas_core::Project) {
        if let Self::NlFitReport(reports) = self {
            for report in reports {
                if let Some(current) = project.nlfit_report(report) {
                    *report = current.clone();
                } else {
                    report.current = false;
                    report.error = Some("结果列已删除，当前为窗口保留的报告快照".into());
                }
            }
        }
    }
}

#[cfg(test)]
mod warning_tests {
    use super::*;
    use scicanvas_core::{nlfit::CustomFitFunction, ops::Source, Project};

    #[test]
    fn warning_is_in_copied_and_saved_report() {
        let mut project = Project::sample();
        project.books[0].columns[0].data = ["1", "2", "3", "5", "7", "10"].into_iter().map(String::from).collect();
        project.books[0].columns[1].data = ["109", "149", "149", "191", "213", "224"]
            .into_iter()
            .map(String::from)
            .collect();
        let function = CustomFitFunction::new("BoxBOD", "b1*(1-exp(-b2*x))", vec!["b1".into(), "b2".into()]).unwrap();
        let source = Source::Book {
            id: 1,
            selected: vec!["B".into()],
            rows: None,
        };
        let (_, reports) = project.nlfit_custom(&source, function, &[1.0, 1.0], &[], &[]).unwrap();
        let warning = &reports[0].fit.warnings[0];
        assert!(nlfit_text(&reports).contains(warning));
        assert!(nlfit_markdown(&reports).contains(warning));
    }
}
