use super::*;

impl Dialog {
    pub(super) fn show_areas(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::Areas(areas) = self else { unreachable!() };
        let mut action = Action::None;
        window(ctx, "积分结果").open(open).show(ctx, |ui| {
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
            hint(
                ui,
                "梯形法。绝对面积把 X 轴下方的部分也按正值计入。累积积分已写入新列。",
            );
            if actions(ui, &[(ActionKind::Plain, "关闭")], Keys::None, Ok(())).is_some() {
                action = Action::Close;
            }
        });
        action
    }
}
