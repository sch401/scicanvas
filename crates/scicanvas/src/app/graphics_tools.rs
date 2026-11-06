use super::*;
use scicanvas_core::graph_theme::{GraphTheme, StyleClip};
impl SciApp {
    pub(crate) fn open_function_plot(&mut self) {
        let target = self.target_graph().map(|id| (id, self.layer_index(id)));
        self.dialog = Some(Dialog::FunctionPlot(Box::new(dialogs::FunctionPlotEditor {
            expr: "sin(x)".into(),
            points: 500,
            lo: 0.,
            hi: 10.,
            target,
            add: false,
            index: None,
            style: Default::default(),
            error: None,
        })));
    }
    pub(super) fn edit_function_plot(&mut self, gid: u64, layer: usize, index: usize) {
        let g = &self.project.graph(gid).unwrap().page.layers()[layer];
        let f = &g.functions[index];
        self.dialog = Some(Dialog::FunctionPlot(Box::new(dialogs::FunctionPlotEditor {
            expr: f.expr.clone(),
            points: f.points,
            lo: g.x_min,
            hi: g.x_max,
            target: Some((gid, layer)),
            add: true,
            index: Some(index),
            style: f.style.clone(),
            error: None,
        })));
    }
    pub(super) fn run_function_plot(&mut self, edit: dialogs::FunctionPlotEditor) {
        let curve = match edit.curve() {
            Ok(curve) => curve,
            Err(error) => {
                self.status_err(error);
                return;
            }
        };
        self.edit_with(
            "函数绘图",
            |project| {
                if let Some(index) = edit.index {
                    let (gid, layer) = edit.target.unwrap();
                    project.graph_mut(gid).unwrap().page.layers_mut()[layer].functions[index] = curve;
                    Ok(Some(gid))
                } else {
                    project
                        .plot_function(curve, (edit.lo, edit.hi), if edit.add { edit.target } else { None })
                        .map(Some)
                }
            },
            |app, gid| {
                app.show_item(gid);
                app.status_ok("函数图已更新".into());
                app.dialog = None;
            },
        );
    }
    pub(crate) fn open_graph_themes(&mut self) {
        let theme = self
            .target_graph()
            .and_then(|id| {
                self.project
                    .graph(id)
                    .map(|g| GraphTheme::capture(&g.page.layers()[self.layer_index(id)]))
            })
            .unwrap_or_else(GraphTheme::builtin);
        self.dialog = Some(Dialog::GraphThemes(Box::new(dialogs::GraphThemeEditor {
            pending_remove: None,
            default: self.settings.default_graph_theme.as_ref().is_some_and(|t| {
                let mut current = theme.clone();
                current.name = t.name.clone();
                current == *t
            }),
            theme,
            error: None,
        })));
    }
    pub(super) fn save_graph_theme(&mut self, theme: GraphTheme) {
        self.edit_with(
            "保存图形主题",
            |p| p.save_graph_theme(theme).map(|_| Some(())),
            |app, ()| app.status_ok("主题已保存到项目".into()),
        );
    }
    pub(super) fn apply_graph_theme(&mut self, theme: GraphTheme) {
        let Some(gid) = self.target_graph() else {
            self.status_warn("请选择图形".into());
            return;
        };
        let index = self.layer_index(gid);
        self.edit_with(
            "应用图形主题",
            |p| {
                theme.validate()?;
                theme.apply(&mut p.graph_mut(gid).unwrap().page.layers_mut()[index]);
                Ok(Some(()))
            },
            |app, ()| app.status_ok("已应用主题".into()),
        );
    }
    pub(super) fn remove_graph_theme(&mut self, name: &str) {
        self.edit_with(
            "删除图形主题",
            |p| p.remove_graph_theme(name).map(|_| Some(())),
            |app, ()| app.status_ok("主题已删除".into()),
        );
    }
    pub(super) fn import_graph_theme(&mut self) {
        let Some(path) = self.file_dialog().add_filter("图形主题", &["sctheme"]).pick_file() else {
            return;
        };
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| GraphTheme::from_json(&text))
        {
            Ok(theme) => self.save_graph_theme(theme),
            Err(error) => self.status_err(error),
        }
    }
    pub(super) fn export_graph_theme(&mut self, theme: GraphTheme) {
        let data = match theme.to_json() {
            Ok(text) => text,
            Err(error) => {
                self.status_err(error);
                return;
            }
        };
        let Some(path) = self
            .file_dialog()
            .add_filter("图形主题", &["sctheme"])
            .set_file_name(format!("{}.sctheme", theme.name))
            .save_file()
        else {
            return;
        };
        {
            let mut status_kind = StatusKind::Success;
            let text: String = match scicanvas_core::project_file::write_file_atomic(&path, data.as_bytes()) {
                Ok(()) => "主题已导出".into(),
                Err(error) => {
                    status_kind = StatusKind::Error;
                    error
                }
            };
            self.set_status(status_kind, text);
        };
    }
    fn style_target(&mut self) -> Option<(u64, usize, Option<String>)> {
        let gid = self.target_graph()?;
        self.style_target
            .as_ref()
            .filter(|t| t.0 == gid)
            .cloned()
            .or_else(|| Some((gid, self.layer_index(gid), None)))
    }
    pub(crate) fn copy_style(&mut self) {
        let Some((gid, index, key)) = self.style_target() else {
            return;
        };
        let layer = &self.project.graph(gid).unwrap().page.layers()[index];
        self.style_clipboard = Some(if let Some(key) = key {
            let mut style = layer.plot_style(&key).cloned().unwrap_or_default();
            if let Some(book) = layer.book_id.and_then(|id| self.project.book(id)) {
                let (_, plots) = layer.prepared_plots_in(book, &|id| self.project.book(id));
                if let Some(plot) = plots.iter().find(|p| p.plot.key == key) {
                    let (r, g, b) = plot.style.color.rgb();
                    style.color = Some([r, g, b]);
                    style.line_width = Some(plot.style.line_width);
                    style.symbol_size = Some(plot.style.symbol_size);
                    style.symbol = Some(plot.style.symbol);
                }
            }
            StyleClip::curve(&style)
        } else {
            StyleClip::Layer(GraphTheme::capture(layer))
        });
        self.status_ok("格式已复制".into());
    }
    pub(crate) fn paste_style(&mut self) {
        let (Some((gid, index, key)), Some(clip)) = (self.style_target(), self.style_clipboard.clone()) else {
            return;
        };
        self.edit_with(
            "粘贴格式",
            |p| {
                let layer = &mut p.graph_mut(gid).ok_or("找不到图形")?.page.layers_mut()[index];
                match (clip, key) {
                    (StyleClip::Layer(theme), None) => theme.apply(layer),
                    (StyleClip::Curve(mut style), Some(key)) => {
                        let target = layer.plot_style(&key).cloned().unwrap_or_default();
                        style.legend_label = target.legend_label;
                        style.color_column = target.color_column;
                        style.size_column = target.size_column;
                        style.symbol_column = target.symbol_column;
                        layer.set_plot_style(&key, Some(style));
                    }
                    _ => return Err("请粘贴到相同类型的图层或曲线".into()),
                }
                Ok(Some(()))
            },
            |app, ()| app.status_ok("格式已粘贴".into()),
        );
    }
}

impl SciApp {
    pub(super) fn apply_graph_theme_action(&mut self, action: dialogs::GraphThemeAction) {
        use dialogs::GraphThemeAction as A;
        match action {
            A::Save(t) => self.save_graph_theme(t),
            A::Apply(t) => self.apply_graph_theme(t),
            A::Remove(n) => self.remove_graph_theme(&n),
            A::Import => self.import_graph_theme(),
            A::Export(t) => self.export_graph_theme(t),
            A::Default(theme) => {
                if let Some(t) = &theme {
                    if let Err(error) = t.validate() {
                        self.status_err(error);
                        return;
                    }
                }
                self.settings.default_graph_theme = theme;
                self.status_ok("新图主题偏好已更新".into());
            }
        }
    }
}
