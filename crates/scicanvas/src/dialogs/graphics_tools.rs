use super::*;
use scicanvas_core::{graph_theme::GraphTheme, page::FunctionCurve};
#[derive(Clone)]
pub(crate) struct FunctionPlotEditor {
    pub expr: String,
    pub points: u32,
    pub lo: f64,
    pub hi: f64,
    pub target: Option<(u64, usize)>,
    pub add: bool,
    pub index: Option<usize>,
    pub style: scicanvas_core::page::CurveStyle,
    pub error: Option<String>,
}
impl FunctionPlotEditor {
    pub fn curve(&self) -> Result<FunctionCurve, String> {
        let curve = FunctionCurve {
            expr: self.expr.clone(),
            points: self.points,
            style: self.style.clone(),
        };
        curve.validate()?;
        if !self.lo.is_finite() || !self.hi.is_finite() || self.lo >= self.hi {
            return Err("X 范围应为递增有限数".into());
        }
        Ok(curve)
    }
    pub fn show(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "函数绘图").default_width(440.).open(open).show(ctx, |ui| {
            ui.label("表达式（仅变量 x）");
            ui.text_edit_singleline(&mut self.expr);
            ui.horizontal(|ui| {
                ui.label("X 从");
                ui.add(egui::DragValue::new(&mut self.lo));
                ui.label("到");
                ui.add(egui::DragValue::new(&mut self.hi));
            });
            ui.horizontal(|ui| {
                ui.label("采样点数");
                ui.add(egui::DragValue::new(&mut self.points).range(2..=100_000));
            });
            ui.add_enabled(
                self.target.is_some() && self.index.is_none(),
                egui::Checkbox::new(&mut self.add, "添加到当前图层"),
            );
            if let Some(error) = &self.error {
                ui.colored_label(pal.error, error);
            }
            let (ok, cancel) = buttons(ui, "绘制");
            if ok {
                match self.curve() {
                    Ok(_) => action = Action::FunctionPlot(Box::new(self.clone())),
                    Err(error) => self.error = Some(error),
                }
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
pub(crate) struct GraphThemeEditor {
    pub theme: GraphTheme,
    pub default: bool,
    pub error: Option<String>,
}
impl GraphThemeEditor {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        project: &scicanvas_core::Project,
    ) -> Action {
        let mut action = Action::None;
        window(ctx, "图形主题与选项")
            .default_width(480.)
            .open(open)
            .show(ctx, |ui| {
                ui.label("主题只改样式，不改数据、范围和标题。");
                ui.horizontal(|ui| {
                    ui.label("保存当前样式为");
                    ui.text_edit_singleline(&mut self.theme.name);
                    if ui.button("保存为主题").clicked() {
                        match self.theme.validate() {
                            Ok(()) => action = Action::GraphTheme(GraphThemeAction::Save(self.theme.clone())),
                            Err(error) => self.error = Some(error),
                        }
                    }
                });
                if ui.checkbox(&mut self.default, "新图使用当前样式主题").changed() {
                    action = Action::GraphTheme(GraphThemeAction::Default(self.default.then(|| self.theme.clone())));
                }
                ui.horizontal(|ui| {
                    if ui.button("导入 .sctheme").clicked() {
                        action = Action::GraphTheme(GraphThemeAction::Import);
                    }
                    if ui.button("导出当前 .sctheme").clicked() {
                        action = Action::GraphTheme(GraphThemeAction::Export(self.theme.clone()));
                    }
                });
                for theme in std::iter::once(GraphTheme::nature()).chain(project.graph_themes.iter().cloned()) {
                    ui.horizontal(|ui| {
                        ui.label(&theme.name);
                        if ui.button("应用").clicked() {
                            action = Action::GraphTheme(GraphThemeAction::Apply(theme.clone()));
                        }
                        if theme.name != GraphTheme::BUILTIN && ui.button("删除").clicked() {
                            action = Action::GraphTheme(GraphThemeAction::Remove(theme.name.clone()));
                        }
                    });
                }
                if let Some(error) = &self.error {
                    ui.colored_label(pal.error, error);
                }
                if ui.button("关闭").clicked() {
                    action = Action::Close;
                }
            });
        action
    }
}

pub(crate) enum GraphThemeAction {
    Save(GraphTheme),
    Apply(GraphTheme),
    Remove(String),
    Import,
    Export(GraphTheme),
    Default(Option<GraphTheme>),
}
