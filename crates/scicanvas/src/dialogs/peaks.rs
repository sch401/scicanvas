use super::*;

impl Dialog {
    pub(super) fn show_peaks(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::Peaks {
            percent,
            derivative,
            window,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        super::window(ctx, "寻峰与峰积分").open(open).show(ctx, |ui| {
            ui.horizontal(|ui|{ui.radio_value(derivative,false,"局部极大");ui.radio_value(derivative,true,"导数法");});
            if *derivative{ui.horizontal(|ui|{ui.label("SG 窗口");ui.add(egui::DragValue::new(window).range(3..=501));});}
            ui.horizontal_wrapped(|ui| {
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
                action = if *derivative{Action::SignalTools(Box::new(SignalAction::Peaks(*window,*percent/100.)))}else{Action::Peaks(*percent / 100.0)};
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
