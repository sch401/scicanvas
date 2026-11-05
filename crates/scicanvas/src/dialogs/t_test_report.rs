use super::*;

impl Dialog {
    pub(super) fn show_t_test_report(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::TTestReport { result, names } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "t 检验结果").open(open).show(ctx, |ui| {
            let rows = ttest_rows(result, names);
            form(ui, "ttest-report", |ui| {
                for (k, v) in &rows {
                    ui.label(k);
                    ui.label(RichText::new(v).monospace());
                    ui.end_row();
                }
            });
            let verdict = if result.p < 1.0 - result.level {
                "在该显著性水平下拒绝原假设"
            } else {
                "在该显著性水平下不能拒绝原假设"
            };
            ui.label(
                RichText::new(format!(
                    "显著性水平 α = {}：{verdict}。缺失值和非数值单元格已跳过。",
                    fmt_num(1.0 - result.level)
                ))
                .small()
                .color(pal.weak),
            );
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                center_buttons(ui, 3);
                if ui
                    .add(plain("复制结果"))
                    .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                    .clicked()
                {
                    let text: Vec<String> = rows.iter().map(|(k, v)| format!("{k}\t{v}")).collect();
                    ui.ctx().copy_text(text.join("\n"));
                }
                if ui.add(plain("保存为 Note")).clicked() {
                    let text = format!("## t 检验\n\n{}", md_pairs(["项目", "值"], &rows));
                    action = Action::SaveNote("t 检验结果".into(), text);
                }
                if ui.add(primary(ui, "关闭")).clicked() {
                    action = Action::Close;
                }
            });
        });
        action
    }
}

/// t 检验报告的行（标签、各组或总体的值），供窗口和复制共用。
fn ttest_rows(r: &TTest, names: &[String]) -> Vec<(String, String)> {
    let mut rows = vec![("检验".to_string(), r.kind.label().to_string())];
    let groups = r.n.len();
    for g in 0..groups {
        let name = names.get(g).cloned().unwrap_or_default();
        let tag = if groups > 1 {
            format!("{}（{}）", ["A", "B"][g], name)
        } else {
            name
        };
        rows.push((format!("{tag} N"), r.n[g].to_string()));
        rows.push((format!("{tag} 均值"), fmt_num(r.mean[g])));
        rows.push((format!("{tag} 标准差"), fmt_num(r.sd[g])));
    }
    let (estimate, null) = match r.kind {
        TTestKind::OneSample { mu } => ("均值", format!("均值 = {}", fmt_num(mu))),
        TTestKind::TwoSample { .. } => ("均值差 A − B", "两组均值相等".to_string()),
        TTestKind::Paired => ("差值均值 A − B", "差值均值为 0".to_string()),
    };
    rows.push(("原假设".into(), null));
    rows.push((estimate.into(), fmt_num(r.estimate)));
    rows.push(("标准误".into(), fmt_num(r.se)));
    rows.push(("t".into(), fmt_num(r.t)));
    rows.push(("自由度".into(), fmt_num(r.df)));
    rows.push(("p（双侧）".into(), fmt_num(r.p)));
    rows.push((
        format!("{}% 置信区间", fmt_num(r.level * 100.0)),
        format!("{} ~ {}", fmt_num(r.ci.0), fmt_num(r.ci.1)),
    ));
    rows
}
