use super::*;
impl Dialog {
    pub(super) fn show_baseline(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::Baseline {
            advanced,
            count,
            spline,
            order,
            iterations,
            anchors,
            als,
            log_lambda,
            p,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx,"扣除基线").default_width(480.).open(open).show(ctx,|ui|{
            ui.horizontal_wrapped(|ui|{ui.radio_value(advanced,0,"基础方法");ui.radio_value(advanced,1,"锚点法");ui.radio_value(advanced,2,"多项式基线");});
            if *advanced==0 {
                ui.horizontal_wrapped(|ui|{ui.radio_value(als,false,"两端连线");ui.radio_value(als,true,"自动基线（ALS）");});
                ui.add_enabled_ui(*als,|ui|{ui.horizontal(|ui|{ui.label("log₁₀ λ");ui.add(egui::DragValue::new(log_lambda).range(0.0..=12.0).speed(0.05));ui.label("p");ui.add(egui::DragValue::new(p).range(0.0001..=0.5).speed(0.001));});});
            }else if *advanced==1 {
                ui.horizontal(|ui|{ui.label("自动分段数");ui.add(egui::DragValue::new(count).range(2..=1000));ui.checkbox(spline,"自然三次样条");});ui.label("手动锚点 X（逗号分隔，留空自动选择各段最小值并包含首末点）");ui.text_edit_multiline(anchors);
            }else {ui.horizontal(|ui|{ui.label("阶数");ui.add(egui::DragValue::new(order).range(0..=10));ui.label("最多迭代");ui.add(egui::DragValue::new(iterations).range(1..=1000));});}
            ui.label("结果写入基线与扣除后两列，随源数据重算。modpoly 迭代截去基线以上的峰，直到相对变化小于 1e-6 或达到迭代数。");
            let parsed=anchors.split([',','，',' ','\n',';','；']).filter(|s|!s.trim().is_empty()).map(|s|s.trim().parse::<f64>().map_err(|_|"锚点 X 应为逗号分隔的有限数")).collect::<Result<Vec<_>,_>>();let valid=*advanced!=1||parsed.as_ref().is_ok_and(|a|a.iter().all(|x|x.is_finite()));if !valid{ui.colored_label(pal.error,"锚点 X 应为逗号分隔的有限数");}
            let(ok,cancel)=buttons(ui,"扣除");if ok&&valid {action=match *advanced{1=>Action::SignalTools(Box::new(SignalAction::Baseline(BaselineMethod::Anchors{count:*count,spline:*spline},parsed.unwrap()))),2=>Action::SignalTools(Box::new(SignalAction::Baseline(BaselineMethod::Polynomial{order:*order,iterations:*iterations},Vec::new()))),_=>Action::Baseline(if *als{BaselineMethod::Als{lambda:10f64.powf(*log_lambda),p:*p}}else{BaselineMethod::Line})};}else if cancel{action=Action::Close;}
        });
        action
    }
}
