//! 分析用的对话框：参数设置（多项式阶数、平滑、归一化）和结果报告（拟合、积分）。

use eframe::egui::{self, Align2, RichText};
use scicanvas_core::analysis::{NormalizeMethod, SmoothMethod};
use scicanvas_core::fft::Window as FftWindow;
use scicanvas_core::fmt_num;
use scicanvas_core::interpolate::InterpMethod;
use scicanvas_core::nlfit::Model;
use scicanvas_core::ops::{AreaReport, ColumnOp, FitReport, NlFitReport};
use scicanvas_core::stats::{Anova, Correlation, TTest, TTestKind};

use crate::theme::Palette;

pub enum Dialog {
    Poly {
        order: usize,
    },
    Smooth {
        method: SmoothMethod,
        window: usize,
    },
    Normalize {
        method: NormalizeMethod,
    },
    FitReport(Vec<FitReport>),
    Areas(Vec<AreaReport>),
    /// 插值：方法、点数，以及是否使用自定义 X 范围（默认是数据的 X 范围）。
    Interpolate {
        method: InterpMethod,
        points: usize,
        custom: bool,
        from: f64,
        to: f64,
    },
    /// 单因素方差分析结果和各组（列）名称。
    Anova {
        result: Anova,
        names: Vec<String>,
    },
    /// 相关系数：每一对列（名称 A、名称 B）的三种相关系数。
    Correlations(Vec<(String, String, Vec<Correlation>)>),
    /// 非线性拟合：选择内置函数；每个参数可固定为指定值（最多 4 个参数）。
    NlFit {
        model: Model,
        fixed: [(bool, f64); 4],
    },
    /// 非线性拟合结果。
    NlFitReport(Vec<NlFitReport>),
    /// 寻峰：最小突出度（占 Y 范围的百分比）。
    Peaks {
        percent: f64,
    },
    /// FFT：窗函数。
    Fft {
        window: FftWindow,
    },
    /// 列运算：A 与 B 的列名（只选一列时 B 为空，只能用常数）、运算符和常数。
    ColumnMath {
        a: String,
        b: Option<String>,
        op: ColumnOp,
        use_constant: bool,
        constant: f64,
    },
    /// 频数统计：区间数（自动时按 Sturges 规则）和统计范围（默认是数据范围）。
    Frequency {
        auto: bool,
        bins: usize,
        custom: bool,
        from: f64,
        to: f64,
    },
    /// t 检验设置：`columns` 是参与检验的列名（按点选顺序）。
    TTest {
        columns: Vec<String>,
        kind: usize,
        mu: f64,
        equal_var: bool,
        level: f64,
    },
    /// t 检验结果和参与检验的列名。
    TTestReport {
        result: TTest,
        names: Vec<String>,
    },
    /// 导出 PNG：分辨率和页面毫米尺寸（用来显示像素数）。
    Png {
        dpi: u32,
        page_mm: (f64, f64),
    },
}

/// 对话框里用户做出的选择，由 app.rs 执行。
pub enum Action {
    None,
    Close,
    Fit(usize),
    Smooth(SmoothMethod, usize),
    Normalize(NormalizeMethod),
    ExportPng(u32),
    Interpolate(InterpMethod, usize, Option<(f64, f64)>),
    TTest(TTestKind, f64),
    Frequency(Option<usize>, Option<(f64, f64)>),
    Fft(FftWindow),
    /// 最小突出度（占 Y 范围的比例）。
    Peaks(f64),
    NlFit(Model, Vec<Option<f64>>),
    /// 运算符，以及常数（为空时用列 B）。
    ColumnMath(ColumnOp, Option<f64>),
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

fn window(title: &str) -> egui::Window<'_> {
    egui::Window::new(title)
        .id(egui::Id::new("scicanvas-dialog"))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
}

fn buttons(ui: &mut egui::Ui, ok: &str) -> (bool, bool) {
    let mut res = (false, false);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        res.0 = ui.button(RichText::new(ok).strong()).clicked();
        res.1 = ui.button("取消").clicked();
    });
    res
}

pub fn show(ctx: &egui::Context, dialog: &mut Dialog, pal: &Palette) -> Action {
    let mut action = Action::None;
    let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    match dialog {
        Dialog::Poly { order } => {
            window("多项式拟合").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("阶数");
                    egui::ComboBox::from_id_salt("poly-order")
                        .selected_text(order.to_string())
                        .show_ui(ui, |ui| {
                            for k in 2..=6 {
                                ui.selectable_value(order, k, k.to_string());
                            }
                        });
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列拟合。\n阶数越高越容易过拟合，一般用 2 或 3 阶。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "拟合");
                if ok {
                    action = Action::Fit(*order);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Smooth { method, window: w } => {
            window("平滑").show(ctx, |ui| {
                egui::Grid::new("smooth-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("方法");
                        egui::ComboBox::from_id_salt("smooth-method")
                            .selected_text(method.label())
                            .show_ui(ui, |ui| {
                                for m in [SmoothMethod::SavitzkyGolay, SmoothMethod::MovingAverage] {
                                    ui.selectable_value(method, m, m.label());
                                }
                            });
                        ui.end_row();
                        ui.label("窗口点数");
                        ui.add(egui::DragValue::new(w).range(3..=101).speed(1.0));
                        ui.end_row();
                    });
                ui.label(
                    RichText::new("窗口点数取奇数。Savitzky-Golay 能更好地保留峰形。")
                        .small()
                        .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "平滑");
                if ok {
                    action = Action::Smooth(*method, (*w).max(3) | 1);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Normalize { method } => {
            window("归一化").show(ctx, |ui| {
                for m in NormalizeMethod::ALL {
                    ui.radio_value(method, m, m.label());
                }
                let (ok, cancel) = buttons(ui, "归一化");
                if ok {
                    action = Action::Normalize(*method);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::FitReport(reports) => {
            window("拟合结果").show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                    for (ri, r) in reports.iter().enumerate() {
                        ui.label(RichText::new(format!("{} · {}", r.name, r.kind)).strong());
                        egui::Grid::new(("fit-grid", ri))
                            .striped(true)
                            .num_columns(3)
                            .spacing([24.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("参数").color(pal.weak));
                                ui.label(RichText::new("值").color(pal.weak));
                                ui.label(RichText::new("标准误").color(pal.weak));
                                ui.end_row();
                                let linear = r.fit.coef.len() == 2;
                                for (k, (c, se)) in r.fit.coef.iter().zip(&r.fit.se).enumerate() {
                                    let name = if linear {
                                        if k == 0 {
                                            "截距 A".to_string()
                                        } else {
                                            "斜率 B".to_string()
                                        }
                                    } else if k == 0 {
                                        "B0（常数项）".to_string()
                                    } else {
                                        format!("B{}", k)
                                    };
                                    ui.label(name);
                                    ui.label(RichText::new(fmt_num(*c)).monospace());
                                    ui.label(RichText::new(fmt_num(*se)).monospace());
                                    ui.end_row();
                                }
                                let mut row = |k: &str, v: String| {
                                    ui.label(k);
                                    ui.label(RichText::new(v).monospace());
                                    ui.label("");
                                    ui.end_row();
                                };
                                row("R²", fmt_num(r.fit.r2));
                                row("调整后 R²", fmt_num(r.fit.adj_r2));
                                if let Some(pr) = r.fit.pearson_r() {
                                    row("Pearson r", fmt_num(pr));
                                }
                                row("残差平方和", fmt_num(r.fit.rss));
                                row("数据点数 N", r.fit.n.to_string());
                            });
                        ui.add_space(4.0);
                        ui.label(RichText::new(r.fit.equation()).monospace());
                        ui.add_space(10.0);
                    }
                });
                ui.label(
                    RichText::new("拟合曲线已加到工作簿（新增“拟合 X”和拟合值两列）和图形中。")
                        .small()
                        .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("复制结果")
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        ui.ctx().copy_text(fit_text(reports));
                    }
                    if ui.button(RichText::new("关闭").strong()).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Areas(areas) => {
            window("积分结果").show(ctx, |ui| {
                egui::Grid::new("area-grid")
                    .striped(true)
                    .num_columns(4)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
                        for h in ["数据", "面积", "绝对面积", "点数"] {
                            ui.label(RichText::new(h).color(pal.weak));
                        }
                        ui.end_row();
                        for a in areas.iter() {
                            ui.label(&a.name);
                            ui.label(RichText::new(fmt_num(a.area)).monospace());
                            ui.label(RichText::new(fmt_num(a.abs_area)).monospace());
                            ui.label(a.n.to_string());
                            ui.end_row();
                        }
                    });
                ui.label(
                    RichText::new("梯形法。绝对面积把 X 轴下方的部分也按正值计入。累积积分已写入新列。")
                        .small()
                        .color(pal.weak),
                );
                if ui.button("关闭").clicked() {
                    action = Action::Close;
                }
            });
        }
        Dialog::Interpolate {
            method,
            points,
            custom,
            from,
            to,
        } => {
            window("插值").show(ctx, |ui| {
                egui::Grid::new("interp-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("方法");
                        ui.horizontal(|ui| {
                            for m in InterpMethod::ALL {
                                ui.radio_value(method, m, m.label());
                            }
                        });
                        ui.end_row();
                        ui.label("点数");
                        ui.add(egui::DragValue::new(points).range(2..=100_000).speed(1.0));
                        ui.end_row();
                        ui.label("X 范围");
                        ui.horizontal(|ui| {
                            ui.checkbox(custom, "自定义");
                            ui.add_enabled(*custom, egui::DragValue::new(from).speed(0.1));
                            ui.label("到");
                            ui.add_enabled(*custom, egui::DragValue::new(to).speed(0.1));
                        });
                        ui.end_row();
                    });
                ui.label(
                    RichText::new(
                        "在等间距的 X 上求值，结果写成新的 X、Y 列并画成线。超出数据范围时按端点那一段外推。\nAkima 在数据突变处比三次样条更少过冲。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let valid = !*custom || (from.is_finite() && to.is_finite() && from < to);
                if !valid {
                    ui.colored_label(egui::Color32::from_rgb(182, 67, 66), "起点必须小于终点");
                }
                let (ok, cancel) = buttons(ui, "插值");
                if ok && valid {
                    action = Action::Interpolate(*method, *points, custom.then_some((*from, *to)));
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Anova { result, names } => {
            window("单因素方差分析").show(ctx, |ui| {
                let (table, groups) = anova_rows(result, names);
                for (gi, rows) in [table, groups].iter().enumerate() {
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
                        "{verdict}。每个点选的列作为一组，缺失值已跳过；多重比较待后续版本。"
                    ))
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("复制结果")
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let (table, groups) = anova_rows(result, names);
                        let text: Vec<String> = table
                            .iter()
                            .chain([vec![]].iter())
                            .chain(groups.iter())
                            .map(|r| r.join("\t"))
                            .collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.button(RichText::new("关闭").strong()).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Correlations(pairs) => {
            window("相关系数").show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    egui::Grid::new("corr-grid")
                        .striped(true)
                        .num_columns(9)
                        .spacing([14.0, 4.0])
                        .show(ui, |ui| {
                            for h in correlation_header() {
                                ui.label(RichText::new(h).color(pal.weak));
                            }
                            ui.end_row();
                            for row in correlation_rows(pairs) {
                                for (i, cell) in row.iter().enumerate() {
                                    if i < 2 {
                                        ui.label(cell);
                                    } else {
                                        ui.label(RichText::new(cell).monospace());
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
                ui.label(
                    RichText::new(
                        "p 值为双侧。Pearson 和 Spearman 的 p 值用 t 分布，Kendall τ-b 用含并列校正的正态近似；\n只用两列都有数值的行。",
                    )
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("复制结果")
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let mut lines = vec![correlation_header().join("\t")];
                        lines.extend(correlation_rows(pairs).into_iter().map(|r| r.join("\t")));
                        ui.ctx().copy_text(lines.join("\n"));
                    }
                    if ui.button(RichText::new("关闭").strong()).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::NlFit { model, fixed } => {
            window("非线性拟合").show(ctx, |ui| {
                egui::Grid::new("nlfit-models")
                    .num_columns(3)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        for m in Model::ALL {
                            ui.label(RichText::new(m.category()).color(pal.weak));
                            ui.radio_value(model, m, m.name());
                            ui.label(RichText::new(m.formula()).monospace());
                            ui.end_row();
                        }
                    });
                ui.label(
                    RichText::new(format!(
                        "参数：{}。初值由数据自动估计，Levenberg–Marquardt 迭代；\n对当前图形的数据曲线，或工作簿里选中的 Y 列拟合，拟合曲线加到工作簿和图中。",
                        model.params().join("、")
                    ))
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for (name, (on, value)) in model.params().iter().zip(fixed.iter_mut()) {
                        ui.checkbox(on, format!("固定 {name}"));
                        ui.add_enabled(*on, egui::DragValue::new(value).speed(0.01));
                        ui.add_space(8.0);
                    }
                });
                let (ok, cancel) = buttons(ui, "拟合");
                if ok {
                    let held = model
                        .params()
                        .iter()
                        .zip(fixed.iter())
                        .map(|(_, (on, v))| on.then_some(*v))
                        .collect();
                    action = Action::NlFit(*model, held);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::NlFitReport(reports) => {
            window("非线性拟合结果").show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                    for (ri, r) in reports.iter().enumerate() {
                        let f = &r.fit;
                        ui.label(RichText::new(format!("{} · {}", r.name, f.model.name())).strong());
                        ui.label(RichText::new(f.model.formula()).monospace().small());
                        if !f.converged {
                            ui.colored_label(
                                egui::Color32::from_rgb(182, 67, 66),
                                "迭代 500 次仍未收敛，结果仅供参考；可换用其他函数或检查数据",
                            );
                        }
                        egui::Grid::new(("nlfit-grid", ri))
                            .striped(true)
                            .num_columns(3)
                            .spacing([24.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("参数").color(pal.weak));
                                ui.label(RichText::new("值").color(pal.weak));
                                ui.label(RichText::new("标准误").color(pal.weak));
                                ui.end_row();
                                for (k, name) in f.model.params().iter().enumerate() {
                                    ui.label(*name);
                                    ui.label(RichText::new(fmt_num(f.params[k])).monospace());
                                    if f.fixed[k] {
                                        ui.label(RichText::new("固定").color(pal.weak));
                                    } else {
                                        ui.label(RichText::new(fmt_num(f.se[k])).monospace());
                                    }
                                    ui.end_row();
                                }
                                for (k, v) in nlfit_stats(f) {
                                    ui.label(k);
                                    ui.label(RichText::new(v).monospace());
                                    ui.label("");
                                    ui.end_row();
                                }
                            });
                        ui.add_space(10.0);
                    }
                });
                ui.horizontal(|ui| {
                    if ui
                        .button("复制结果")
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        ui.ctx().copy_text(nlfit_text(reports));
                    }
                    if ui.button(RichText::new("关闭").strong()).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Peaks { percent } => {
            window("寻峰与峰积分").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("最小突出度");
                    ui.add(egui::DragValue::new(percent).range(0.1..=100.0).speed(0.2).suffix(" %"));
                    ui.label(RichText::new("（占 Y 范围）").color(pal.weak));
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列寻峰（算法同 SciPy find_peaks）。\n突出度是峰顶比两侧较高的那个谷底高出多少，调大可以滤掉噪声小峰。\n每条曲线生成一个峰表：峰位、峰高、突出度、半峰宽（半突出度处）、面积和质心；\n面积在两侧到相邻峰之间的最低点内、扣除两端连线（线性基线）后积分。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "寻峰");
                if ok {
                    action = Action::Peaks(*percent / 100.0);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Fft { window: w } => {
            window("FFT").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("窗函数");
                    for m in FftWindow::ALL {
                        ui.radio_value(w, m, m.label());
                    }
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列做 FFT。X 需要等间距（X 以 s 为单位时频率单位为 Hz）。\n每条曲线生成一个频谱表（频率、实部、虚部、幅值、相位、功率）和一张幅值谱图；\n幅值按正弦振幅归一，加窗时按窗函数的相干增益补偿。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "变换");
                if ok {
                    action = Action::Fft(*w);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::ColumnMath {
            a,
            b,
            op,
            use_constant,
            constant,
        } => {
            window("列运算").show(ctx, |ui| {
                if b.is_none() {
                    *use_constant = true;
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(a.as_str()).strong());
                    for o in ColumnOp::ALL {
                        ui.selectable_value(op, o, o.symbol());
                    }
                    if *use_constant {
                        ui.add(egui::DragValue::new(constant).speed(0.1));
                    } else if let Some(b) = b.as_ref() {
                        ui.label(RichText::new(b.as_str()).strong());
                    }
                });
                ui.add_enabled_ui(b.is_some(), |ui| {
                    ui.checkbox(use_constant, "用常数代替第二列");
                });
                ui.label(
                    RichText::new(
                        "结果写成新列放在第一列右边。两列对应不同的 X 列时（如原始曲线减拟合曲线），\n先把第二列按线性插值到第一列的 X 上；除以 0 或缺失值处留空。\n只选一列时与常数运算，选两列时按点选顺序。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "计算");
                if ok {
                    action = Action::ColumnMath(*op, use_constant.then_some(*constant));
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Frequency {
            auto,
            bins,
            custom,
            from,
            to,
        } => {
            window("频数统计").show(ctx, |ui| {
                egui::Grid::new("freq-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("区间数");
                        ui.horizontal(|ui| {
                            ui.checkbox(auto, "自动（Sturges）");
                            ui.add_enabled(!*auto, egui::DragValue::new(bins).range(1..=10_000));
                        });
                        ui.end_row();
                        ui.label("统计范围");
                        ui.horizontal(|ui| {
                            ui.checkbox(custom, "自定义");
                            ui.add_enabled(*custom, egui::DragValue::new(from).speed(0.1));
                            ui.label("到");
                            ui.add_enabled(*custom, egui::DragValue::new(to).speed(0.1));
                        });
                        ui.end_row();
                    });
                ui.label(
                    RichText::new(
                        "每个选中的列生成一个新表：区间下限、中点、上限、频数、累积频数、相对频率和累积相对频率。\n区间等宽，最后一个区间包含右端点（与 numpy.histogram 相同）；范围外的值不计。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let valid = !*custom || (from.is_finite() && to.is_finite() && from < to);
                if !valid {
                    ui.colored_label(egui::Color32::from_rgb(182, 67, 66), "起点必须小于终点");
                }
                let (ok, cancel) = buttons(ui, "统计");
                if ok && valid {
                    action = Action::Frequency((!*auto).then_some(*bins), custom.then_some((*from, *to)));
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::TTest {
            columns,
            kind,
            mu,
            equal_var,
            level,
        } => {
            window("t 检验").show(ctx, |ui| {
                const KINDS: [&str; 3] = ["单样本", "双样本（独立）", "配对"];
                ui.label(format!("参与检验的列：{}", columns.join("、")));
                ui.horizontal(|ui| {
                    for (i, label) in KINDS.iter().enumerate() {
                        ui.radio_value(kind, i, *label);
                    }
                });
                egui::Grid::new("ttest-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        if *kind == 0 {
                            ui.label("假设均值");
                            ui.add(egui::DragValue::new(mu).speed(0.1));
                            ui.end_row();
                        }
                        if *kind == 1 {
                            ui.label("方差");
                            ui.checkbox(equal_var, "假设两组方差相等（不勾选为 Welch 检验）");
                            ui.end_row();
                        }
                        ui.label("置信水平");
                        ui.horizontal(|ui| {
                            for l in [0.9, 0.95, 0.99] {
                                ui.selectable_value(level, l, format!("{}%", fmt_num(l * 100.0)));
                            }
                        });
                        ui.end_row();
                    });
                let need = if *kind == 0 { 1 } else { 2 };
                let ready = columns.len() >= need;
                let hint = match *kind {
                    0 => "使用第 1 列。".to_string(),
                    1 => "比较第 1 列（A）与第 2 列（B）两组独立样本。".to_string(),
                    _ => "按行配对第 1 列（A）与第 2 列（B），只用两列都有数值的行。".to_string(),
                };
                ui.label(RichText::new(hint).small().color(pal.weak));
                if !ready {
                    ui.colored_label(
                        egui::Color32::from_rgb(182, 67, 66),
                        "请先在工作表里按顺序单击选中两列（Ctrl+单击多选）",
                    );
                }
                let (ok, cancel) = buttons(ui, "计算");
                if ok && ready {
                    let k = match *kind {
                        0 => TTestKind::OneSample { mu: *mu },
                        1 => TTestKind::TwoSample { equal_var: *equal_var },
                        _ => TTestKind::Paired,
                    };
                    action = Action::TTest(k, *level);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::TTestReport { result, names } => {
            window("t 检验结果").show(ctx, |ui| {
                let rows = ttest_rows(result, names);
                egui::Grid::new("ttest-report")
                    .striped(true)
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
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
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("复制结果")
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let text: Vec<String> = rows.iter().map(|(k, v)| format!("{k}\t{v}")).collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.button(RichText::new("关闭").strong()).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Png { dpi, page_mm } => {
            window("导出 PNG").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("分辨率");
                    for preset in [150, 300, 600, 1200] {
                        ui.selectable_value(dpi, preset, format!("{preset} dpi"));
                    }
                    ui.add(egui::DragValue::new(dpi).range(72..=2400).suffix(" dpi"));
                });
                let px = |mm: f64| (mm / 25.4 * f64::from(*dpi)).round();
                ui.label(format!(
                    "页面 {} × {} mm，输出约 {} × {} 像素",
                    fmt_num(page_mm.0),
                    fmt_num(page_mm.1),
                    px(page_mm.0),
                    px(page_mm.1)
                ));
                ui.label(
                    RichText::new("期刊投稿的线条图一般用 600 dpi；PNG 记录了分辨率，插入 Word 时按页面实际尺寸显示。")
                        .small()
                        .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "导出…");
                if ok {
                    action = Action::ExportPng(*dpi);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
    }
    if esc && matches!(action, Action::None) {
        action = Action::Close;
    }
    action
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

fn correlation_header() -> [&'static str; 9] {
    [
        "列 A",
        "列 B",
        "N",
        "Pearson r",
        "p",
        "Spearman ρ",
        "p",
        "Kendall τ-b",
        "p",
    ]
}

fn correlation_rows(pairs: &[(String, String, Vec<Correlation>)]) -> Vec<Vec<String>> {
    pairs
        .iter()
        .map(|(a, b, cs)| {
            let mut row = vec![
                a.clone(),
                b.clone(),
                cs.first().map_or(String::new(), |c| c.n.to_string()),
            ];
            for c in cs {
                row.push(fmt_num(c.r));
                row.push(fmt_num(c.p));
            }
            row
        })
        .collect()
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

fn nlfit_text(reports: &[NlFitReport]) -> String {
    let mut lines = vec![];
    for r in reports {
        let f = &r.fit;
        lines.push(format!("{}\t{}\t{}", r.name, f.model.name(), f.model.formula()));
        lines.push("参数\t值\t标准误".into());
        for (k, name) in f.model.params().iter().enumerate() {
            let se = if f.fixed[k] {
                "固定".to_string()
            } else {
                fmt_num(f.se[k])
            };
            lines.push(format!("{name}\t{}\t{se}", fmt_num(f.params[k])));
        }
        for (k, v) in nlfit_stats(f) {
            lines.push(format!("{k}\t{v}"));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn fit_text(reports: &[FitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let mut s = format!("{}\t{}\n{}\n参数\t值\t标准误\n", r.name, r.kind, r.fit.equation());
            for (k, (c, se)) in r.fit.coef.iter().zip(&r.fit.se).enumerate() {
                s.push_str(&format!("B{}\t{}\t{}\n", k, c, se));
            }
            s.push_str(&format!(
                "R²\t{}\n调整后 R²\t{}\n残差平方和\t{}\nN\t{}",
                r.fit.r2, r.fit.adj_r2, r.fit.rss, r.fit.n
            ));
            s
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
