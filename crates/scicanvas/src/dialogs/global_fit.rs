use super::*;
use scicanvas_core::{
    nlfit::{global_layout, FitFunction, ParamLimits},
    ops::Source,
};

#[derive(Clone)]
pub(crate) struct GlobalFitEditor {
    pub source: Source,
    pub labels: Vec<String>,
    pub model: Model,
    pub shared: Vec<bool>,
    pub limits: Vec<ParamLimits>,
    pub constraints: String,
    pub weighted: bool,
    names: Vec<String>,
    error: Option<String>,
}
impl GlobalFitEditor {
    pub fn new(source: Source, labels: Vec<String>) -> Self {
        let mut edit = Self {
            source,
            labels,
            model: Model::ExpDec1,
            shared: vec![true; 3],
            limits: Vec::new(),
            constraints: String::new(),
            weighted: false,
            names: Vec::new(),
            error: None,
        };
        edit.sync();
        edit
    }
    fn sync(&mut self) {
        if self.shared.len() != self.model.params().len() {
            self.shared = vec![true; self.model.params().len()];
            self.constraints.clear();
        }
        match global_layout(&FitFunction::Builtin(self.model), self.labels.len(), &self.shared) {
            Ok(layout) => {
                if layout.names != self.names {
                    self.limits = layout
                        .names
                        .iter()
                        .map(|name| {
                            self.names
                                .iter()
                                .position(|n| n == name)
                                .map_or(ParamLimits::default(), |i| self.limits[i])
                        })
                        .collect();
                    self.names = layout.names;
                }
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        self.sync();
        let mut action = Action::None;
        window(ctx,"全局拟合").default_width(600.0).open(open).show(ctx,|ui| {
            ui.label(format!("数据集：{}",self.labels.join("、")));
            let before=self.model;
            egui::ComboBox::from_id_salt("global-model").selected_text(self.model.name()).show_ui(ui,|ui| {
                for model in Model::ALL {ui.selectable_value(&mut self.model,model,model.name());}
            });
            if before!=self.model {self.shared=vec![true;self.model.params().len()];self.constraints.clear();self.sync();}
            ui.label(self.model.formula());
            ui.horizontal_wrapped(|ui| {
                for (i,name) in self.model.params().iter().enumerate() {ui.checkbox(&mut self.shared[i],format!("共享 {name}"));}
            });
            self.sync();
            egui::ScrollArea::both().max_height(300.0).show(ui,|ui| {
                egui::Grid::new("global-parameters").striped(true).show(ui,|ui| {
                    for label in ["参数","固定","下限","上限"] {ui.label(label);}ui.end_row();
                    for (name,limit) in self.names.iter().zip(&mut self.limits) {
                        ui.label(name);
                        for field in [&mut limit.fixed,&mut limit.lower,&mut limit.upper] {
                            ui.horizontal(|ui| {
                                let mut enabled=field.is_some();let mut value=field.unwrap_or(0.0);
                                ui.checkbox(&mut enabled,"");ui.add_enabled(enabled,egui::DragValue::new(&mut value).speed(0.1));*field=enabled.then_some(value);
                            });
                        }
                        ui.end_row();
                    }
                });
            });
            ui.checkbox(&mut self.weighted,"按各组关联的 Y 误差加权（1/σ²）");
            ui.label("线性约束（展开的参数名可用于等式或不等式）");ui.text_edit_multiline(&mut self.constraints);
            ui.label("初值自动估计，共享参数取各组中位数。结果曲线和 Note 为一次性快照，暂不支持自动重算。最多 60 个展开参数。");
            if let Some(error)=&self.error {ui.colored_label(pal.error,error);}
            let (ok,cancel)=buttons(ui,"拟合");
            if ok && self.error.is_none() {action=Action::GlobalFit(Box::new(self.clone()));}
            else if cancel {action=Action::Close;}
        });
        action
    }
}
