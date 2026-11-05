use super::*;

impl Dialog {
    pub(super) fn show_anova(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::Anova { result, names, tukey } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "单因素方差分析").centered().open(open).show(ctx, |ui| {
            let (table, groups) = anova_rows(result, names);
            let pairs = tukey_rows(tukey, names);
            for (gi, rows) in [table, groups, pairs].iter().enumerate() {
                if gi == 2 {
                    ui.label(RichText::new("Tukey HSD 多重比较（95% 置信区间）").strong());
                }
                egui::Grid::new(("anova-grid", gi))
                    .striped(true)
                    .num_columns(rows[0].len())
                    .spacing([18.0, 4.0])
                    .show(ui, |ui| {
                        for (ri, row) in rows.iter().enumerate() {
                            for (ci, cell) in row.iter().enumerate() {
                                if ri == 0 {
                                    ui.label(RichText::new(cell).color(pal.weak));
                                } else if ci == 0 {
                                    ui.label(cell);
                                } else {
                                    ui.label(RichText::new(cell).monospace());
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
            }
            let verdict = if result.p < 0.05 {
                "α = 0.05 时各组均值有显著差异"
            } else {
                "α = 0.05 时不能认为各组均值有差异"
            };
            ui.label(
                RichText::new(format!(
                    "{verdict}。缺失值已跳过。"
                ))
                .small()
                .color(pal.weak),
            );
            ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                let (mut rows,groups)=anova_rows(result,names);let pairs=tukey_rows(tukey,names);
                let markdown=format!("## 单因素方差分析\n\n{}\n### 各组统计量\n\n{}\n### Tukey HSD 多重比较\n\n{}\n\nTukey HSD 的 p 值已作多重比较校正，组大小不等时使用 Tukey–Kramer。",md_table(&rows),md_table(&groups),md_table(&pairs));
                rows.extend(groups);rows.extend(pairs);let text=rows.iter().map(|r|r.join("\t")).collect::<Vec<_>>().join("\n");
                action=result_actions(ui,"方差分析结果",&rows,markdown,text);
        });
        action
    }
}

/// Tukey HSD 两两比较表（首行为表头）。
fn tukey_rows(pairs: &[TukeyPair], names: &[String]) -> Vec<Vec<String>> {
    let mut rows = vec![["组 A", "组 B", "均值差 A−B", "下限", "上限", "p", "α = 0.05"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()];
    for t in pairs {
        rows.push(vec![
            names[t.a].clone(),
            names[t.b].clone(),
            fmt_num(t.diff),
            fmt_num(t.low),
            fmt_num(t.high),
            fmt_num(t.p),
            if t.p < 0.05 { "显著" } else { "不显著" }.into(),
        ]);
    }
    rows
}

/// ANOVA 表与各组统计量（首行为表头），供窗口和复制共用。
fn anova_rows(a: &Anova, names: &[String]) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let row = |cells: &[&str]| cells.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let table = vec![
        row(&["来源", "平方和", "自由度", "均方", "F", "p"]),
        vec![
            "组间".into(),
            fmt_num(a.ss_between),
            fmt_num(a.df_between),
            fmt_num(a.ms_between()),
            fmt_num(a.f),
            fmt_num(a.p),
        ],
        vec![
            "组内".into(),
            fmt_num(a.ss_within),
            fmt_num(a.df_within),
            fmt_num(a.ms_within()),
            String::new(),
            String::new(),
        ],
        vec![
            "总计".into(),
            fmt_num(a.ss_between + a.ss_within),
            fmt_num(a.df_between + a.df_within),
            String::new(),
            String::new(),
            String::new(),
        ],
    ];
    let mut groups = vec![row(&["组", "N", "均值", "标准差"])];
    for (i, name) in names.iter().enumerate() {
        groups.push(vec![
            name.clone(),
            a.n[i].to_string(),
            fmt_num(a.mean[i]),
            fmt_num(a.sd[i]),
        ]);
    }
    (table, groups)
}
