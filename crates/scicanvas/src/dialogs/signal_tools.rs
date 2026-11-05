use super::*;
use scicanvas_core::{
    ops::{MergeDuplicates, Source},
    plotspec::DataRef,
    signal::{ConvolutionMode, SignalOperation},
};
pub(crate) enum SignalAction {
    Baseline(BaselineMethod, Vec<f64>),
    Peaks(usize, f64),
    Interpolate(InterpMethod, DataRef),
    Query(InterpMethod, f64),
    Merge(Option<MergeDuplicates>),
    Signal(Source, SignalOperation),
    SignalSwapped(Source, SignalOperation),
}
pub(crate) struct SignalEditor {
    pub source: Source,
    pub swapped: bool,
    pub labels: Vec<String>,
    pub inverse: bool,
    pub mode: ConvolutionMode,
    pub epsilon: f64,
}
impl SignalEditor {
    pub fn show(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "卷积/反卷积").centered().open(open).show(ctx, |ui| {
            ui.label(format!("依次为信号、卷积核：{}", self.labels.join("、")));
            if ui.button("交换信号/卷积核").clicked() {
                self.labels.swap(0, 1);
                self.swapped = !self.swapped;
            }
            form(ui, "signal-form", |f| {
                f.row("操作", |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.inverse, false, "卷积");
                        ui.radio_value(&mut self.inverse, true, "反卷积");
                    });
                });
                if self.inverse {
                    f.row("正则系数 ε", |ui| {
                        ui.add(egui::DragValue::new(&mut self.epsilon).speed(0.0001).range(0.0..=1.))
                            .on_hover_text("ε 越大越平滑，0 表示纯除法。");
                    });
                } else {
                    f.row("结果长度", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (mode, label) in [
                                (ConvolutionMode::Full, "Full（全长）"),
                                (ConvolutionMode::Same, "Same（同长）"),
                                (ConvolutionMode::Valid, "Valid（有效）"),
                            ] {
                                ui.radio_value(&mut self.mode, mode, label);
                            }
                        });
                    });
                }
            });
            hint(ui, "结果不乘采样间隔，写入新列并附图。");
            if self.inverse && (!self.epsilon.is_finite() || self.epsilon < 0.) {
                field_error(ui, "ε 应为有限非负数");
            }
            let (ok, cancel) = buttons(
                ui,
                "计算",
                if !self.inverse || (self.epsilon.is_finite() && self.epsilon >= 0.) {
                    Ok(())
                } else {
                    Err("ε 应为有限非负数".into())
                },
            );
            if ok {
                let operation = if self.inverse {
                    SignalOperation::Deconvolve { epsilon: self.epsilon }
                } else {
                    SignalOperation::Convolve(self.mode)
                };
                action = Action::SignalTools(Box::new(if self.swapped {
                    SignalAction::SignalSwapped(self.source.clone(), operation)
                } else {
                    SignalAction::Signal(self.source.clone(), operation)
                }));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
impl Dialog {
    pub(super) fn show_average(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let Self::Average { merge, mean } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "曲线平均与合并").centered().open(open).show(ctx, |ui| {
            data_hint(ui);
            data_hint(ui);
            ui.horizontal(|ui| {
                ui.radio_value(merge, false, "插值平均");
                ui.radio_value(merge, true, "直接合并");
            });
            if *merge {
                ui.horizontal(|ui| {
                    ui.label("重复 X");
                    ui.radio_value(mean, true, "取均值");
                    ui.radio_value(mean, false, "保留全部点");
                });
            }
            ui.label(if *merge {
                "拼接各曲线后按 X 稳定排序，输出到新工作簿和线图。"
            } else {
                "在线性插值的共同 X 区间上求平均，并输出标准差。"
            });
            let (ok, cancel) = buttons(ui, "计算", Ok(()));
            if ok {
                action = Action::SignalTools(Box::new(SignalAction::Merge(if *merge {
                    Some(if *mean {
                        MergeDuplicates::Mean
                    } else {
                        MergeDuplicates::KeepAll
                    })
                } else {
                    None
                })));
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
