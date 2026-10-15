//! 分析用的对话框：参数设置（多项式阶数、平滑、归一化）和结果报告（拟合、积分）。

use eframe::egui::{self, Align2, RichText};
use scicanvas_core::analysis::{NormalizeMethod, SmoothMethod};
use scicanvas_core::fmt_num;
use scicanvas_core::ops::{AreaReport, FitReport};

use crate::theme::Palette;

pub enum Dialog {
    Poly { order: usize },
    Smooth { method: SmoothMethod, window: usize },
    Normalize { method: NormalizeMethod },
    FitReport(Vec<FitReport>),
    Areas(Vec<AreaReport>),
}

/// 对话框里用户做出的选择，由 app.rs 执行。
pub enum Action {
    None,
    Close,
    Fit(usize),
    Smooth(SmoothMethod, usize),
    Normalize(NormalizeMethod),
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
    }
    if esc && matches!(action, Action::None) {
        action = Action::Close;
    }
    action
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
