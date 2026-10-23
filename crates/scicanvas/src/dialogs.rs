//! 分析用的对话框：参数设置（多项式阶数、平滑、归一化）和结果报告（拟合、积分）。

use eframe::egui::{self, Align2, RichText};
use scicanvas_core::analysis::{NormalizeMethod, SmoothMethod};
use scicanvas_core::baseline::BaselineMethod;
use scicanvas_core::fft::{FilterKind, Window as FftWindow};
use scicanvas_core::fmt_num;
use scicanvas_core::interpolate::InterpMethod;
use scicanvas_core::nlfit::Model;
use scicanvas_core::ops::{AreaReport, ColumnOp, FitReport, NlFitReport};
use scicanvas_core::stats::Sequence;
use scicanvas_core::stats::{Anova, Correlation, TTest, TTestKind, TukeyPair};

use crate::theme::Palette;

/// A text focus cycle or continuous pointer edit shares one history transaction.
pub(crate) fn edit_group(response: &egui::Response, group: &mut Option<egui::Id>) {
    let text_focus = response.has_focus() && egui::TextEdit::load_state(&response.ctx, response.id).is_some();
    if text_focus || response.is_pointer_button_down_on() || response.dragged() || response.drag_stopped() {
        *group = Some(response.id);
    }
}

fn page_position_editor(ui: &mut egui::Ui, position: &mut [f64; 2], group: &mut Option<egui::Id>) {
    ui.horizontal(|ui| {
        for (index, label) in ["X ", "Y "].into_iter().enumerate() {
            let mut percent = position[index] * 100.0;
            let r = ui.add(
                egui::DragValue::new(&mut percent)
                    .range(0.0..=100.0)
                    .speed(0.1)
                    .prefix(label)
                    .suffix("%"),
            );
            if r.changed() {
                position[index] = percent / 100.0;
            }
            edit_group(&r, group);
        }
    });
}

#[derive(Clone, Copy)]
pub(crate) struct PlotObjectEditor {
    pub graph: u64,
    pub target: crate::plot::ObjectTarget,
    pub focus: bool,
}
impl PlotObjectEditor {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        g: &mut scicanvas_core::Graph,
        history_group: &mut Option<egui::Id>,
    ) -> (bool, Option<crate::plot::ObjectAction>) {
        use crate::plot::{self, ObjectTarget as T};
        let edit = self;
        let title = if matches!(edit.target, T::Text(_)) {
            "文字标注"
        } else {
            "图例"
        };
        let mut open = true;
        let mut action = None;
        window(ctx, title).open(&mut open).show(ctx, |ui| {
            ui.push_id((edit.graph, edit.target), |ui| {
                ui.label(&g.name);
                match edit.target {
                    T::Text(id) => {
                        egui::ComboBox::from_id_salt("text-object")
                            .selected_text(format!("文字 {id}"))
                            .show_ui(ui, |ui| {
                                for text in &g.page.annotations {
                                    let name: String = scicanvas_core::text::plain_text(&text.text, text.rich)
                                        .chars()
                                        .take(24)
                                        .collect();
                                    if ui
                                        .selectable_label(text.id == id, format!("{} · {}", text.id, name))
                                        .clicked()
                                    {
                                        edit.target = T::Text(text.id);
                                        edit.focus = true;
                                    }
                                }
                            });
                        let text = g.page.text_mut(id).unwrap();
                        egui::Grid::new("text-object-properties").num_columns(2).show(ui, |ui| {
                            ui.label("内容");
                            let r = scientific_text_multiline(ui, &mut text.text, &mut text.rich);
                            if edit.focus && edit.target == T::Text(id) {
                                r.request_focus();
                                if let Some(mut state) = egui::TextEdit::load_state(ctx, r.id) {
                                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(0),
                                        egui::text::CCursor::new(text.text.chars().count()),
                                    )));
                                    state.store(ctx, r.id);
                                }
                                edit.focus = false;
                            }
                            edit_group(&r, history_group);
                            ui.end_row();
                            ui.label("科学文字");
                            ui.checkbox(&mut text.rich, "启用");
                            ui.end_row();
                            ui.label("字号");
                            let r = ui.add(
                                egui::DragValue::new(&mut text.size_pt)
                                    .range(1.0..=256.0)
                                    .speed(0.25)
                                    .suffix(" pt"),
                            );
                            edit_group(&r, history_group);
                            ui.end_row();
                            ui.label("颜色");
                            edit_group(&ui.color_edit_button_srgb(&mut text.color), history_group);
                            ui.end_row();
                            ui.label("页面位置");
                            page_position_editor(ui, &mut text.position, history_group);
                            ui.end_row();
                        });
                    }
                    T::Legend(index) => {
                        ui.label(format!("图层 {}", index + 1));
                        let layer = &mut g.page.layers_mut()[index];
                        if let Some(position) = &mut layer.legend_position {
                            page_position_editor(ui, position, history_group);
                        }
                        if ui.button("恢复自动避让").clicked() {
                            action = Some(plot::ObjectAction::AutoLegend(index));
                        }
                        ui.checkbox(&mut layer.rich_text, "启用科学文字格式");
                        for key in layer.y_keys.clone() {
                            ui.label(format!("曲线 {key}"));
                            let mut style = layer.curves.get(&key).cloned().unwrap_or_default();
                            let r =
                                scientific_text(ui, &mut style.legend_label, &mut layer.rich_text, "自动：数据列名称");
                            if edit.focus {
                                r.request_focus();
                                edit.focus = false;
                            }
                            edit_group(&r, history_group);
                            if style.is_default() {
                                layer.curves.remove(&key);
                            } else {
                                layer.curves.insert(key, style);
                            }
                        }
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.add(plain("删除")).clicked() {
                        action = Some(plot::ObjectAction::Delete(edit.target));
                    }
                });
            });
        });
        (open, action)
    }
}

/// Shared title/legend editor. Selection indices are Unicode character positions, as in egui.
pub(crate) fn scientific_text(ui: &mut egui::Ui, text: &mut String, rich: &mut bool, hint: &str) -> egui::Response {
    scientific_text_editor(ui, text, rich, hint, false)
}
pub(crate) fn scientific_text_multiline(ui: &mut egui::Ui, text: &mut String, rich: &mut bool) -> egui::Response {
    scientific_text_editor(ui, text, rich, "", true)
}
fn scientific_text_editor(
    ui: &mut egui::Ui,
    text: &mut String,
    rich: &mut bool,
    hint: &str,
    multiline: bool,
) -> egui::Response {
    ui.vertical(|ui| {
        let widget = if multiline {
            egui::TextEdit::multiline(text).desired_rows(3)
        } else {
            egui::TextEdit::singleline(text)
        };
        let mut edit = widget.hint_text(hint).desired_width(f32::INFINITY).show(ui);
        let mut action = None;
        ui.horizontal(|ui| {
            for (label, tip, command) in [
                ("B", "粗体", "\\b{"),
                ("I", "斜体", "\\i{"),
                ("x²", "上标", "^{"),
                ("x₂", "下标", "_{"),
            ] {
                let label = match command {
                    "\\b{" => RichText::new(label).strong(),
                    "\\i{" => RichText::new(label).italics(),
                    _ => RichText::new(label),
                };
                if ui.small_button(label).on_hover_text(tip).clicked() {
                    action = Some((command, true));
                }
            }
            egui::ComboBox::from_id_salt(edit.response.id.with("greek"))
                .width(36.0)
                .selected_text("Ω")
                .show_ui(ui, |ui| {
                    egui::Grid::new(edit.response.id.with("symbols"))
                        .num_columns(8)
                        .show(ui, |ui| {
                            for (i, (name, symbol)) in scicanvas_core::text::GREEK.iter().enumerate() {
                                if ui.selectable_label(false, *symbol).on_hover_text(*name).clicked() {
                                    action = Some((*symbol, false));
                                    ui.close_menu();
                                }
                                if i % 8 == 7 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            if ui.small_button("清除").on_hover_text("清除所选文字的格式").clicked() {
                action = Some(("", true));
            }
        });
        if let Some((command, format)) = action {
            let range = edit
                .state
                .cursor
                .char_range()
                .map(|r| r.sorted())
                .unwrap_or([egui::text::CCursor::new(text.chars().count()); 2]);
            let (selection, caret) = format_selection(text, range[0].index..range[1].index, command, format);
            if format && !command.is_empty() {
                *rich = true;
            }
            edit.state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(selection),
                egui::text::CCursor::new(caret),
            )));
            edit.state.store(ui.ctx(), edit.response.id);
            edit.response.request_focus();
            edit.response.mark_changed();
        }
        edit.response
    })
    .inner
}

fn format_selection(text: &mut String, range: std::ops::Range<usize>, command: &str, format: bool) -> (usize, usize) {
    let byte = |i| text.char_indices().nth(i).map_or(text.len(), |(n, _)| n);
    let (mut start, mut end) = (byte(range.start), byte(range.end));
    let selected = text[start..end].to_string();
    let mut first = range.start;
    let replacement = if format && command.is_empty() {
        // Strip enclosing formats too when a toolbar-created inner selection is still active.
        while let Some(prefix) = ["\\b{", "\\i{", "^{", "_{"]
            .into_iter()
            .find(|p| text[..start].ends_with(p) && text[end..].starts_with('}'))
        {
            start -= prefix.len();
            end += 1;
            first -= prefix.chars().count();
        }
        scicanvas_core::text::plain_text(&selected, true)
    } else if format && text[..start].ends_with(command) && text[end..].starts_with('}') {
        start -= command.len();
        end += 1;
        first -= command.chars().count();
        selected.clone()
    } else if format {
        first += command.chars().count();
        format!("{command}{selected}}}")
    } else {
        command.to_string()
    };
    let last = if format {
        first
            + scicanvas_core::text::plain_text(&selected, command.is_empty())
                .chars()
                .count()
    } else {
        first + replacement.chars().count()
    };
    text.replace_range(start..end, &replacement);
    (if format { first } else { last }, last)
}

#[cfg(test)]
mod scientific_text_tests {
    use super::*;
    #[test]
    fn selected_unicode_formats_toggle_nest_clear_and_insert_symbols() {
        let mut text = "电压CO2".to_string();
        let (a, b) = format_selection(&mut text, 4..5, "_{", true);
        assert_eq!(text, "电压CO_{2}");
        assert_eq!(&text.chars().skip(a).take(b - a).collect::<String>(), "2");
        let (a, b) = format_selection(&mut text, a..b, "\\b{", true);
        let (a, b) = format_selection(&mut text, a..b, "\\i{", true);
        assert_eq!(text, r"电压CO_{\b{\i{2}}}");
        format_selection(&mut text, a..b, "", true);
        assert_eq!(text, "电压CO2");
        let (a, b) = format_selection(&mut text, 4..5, "^{", true);
        format_selection(&mut text, a..b, "^{", true);
        assert_eq!(text, "电压CO2");
        let (a, b) = format_selection(&mut text, 0..2, "α", false);
        assert_eq!(text, "αCO2");
        assert_eq!((a, b), (1, 1));
        let (a, b) = format_selection(&mut text, 1..1, "\\i{", true);
        assert_eq!(a, b);
        assert_eq!(text, r"α\i{}CO2");
    }
}

/// 列属性对话框里编辑的内容。
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnEdit {
    pub key: String,
    pub long_name: String,
    pub units: String,
    pub comment: String,
    pub role: scicanvas_core::Role,
    pub kind: scicanvas_core::DataKind,
    pub format: scicanvas_core::format::DisplayFormat,
    /// 分析结果列的重算方式（普通列为 None）。
    pub recalc: Option<scicanvas_core::ops::RecalcMode>,
}

impl From<&scicanvas_core::Column> for ColumnEdit {
    fn from(col: &scicanvas_core::Column) -> Self {
        Self {
            key: col.key.clone(),
            long_name: col.long_name.clone(),
            units: col.units.clone(),
            comment: col.comment.clone(),
            role: col.role,
            kind: col.data.kind(),
            format: col.format,
            recalc: col.op.as_ref().map(|op| op.mode),
        }
    }
}

impl ColumnEdit {
    pub fn apply(&self, book: &mut scicanvas_core::Book) -> Result<(), String> {
        let col = book.column(&self.key).ok_or("找不到列")?;
        if col.data.kind() != self.kind {
            book.convert_columns(std::slice::from_ref(&self.key), self.kind)?;
        }
        if book.column(&self.key).is_some_and(|c| c.role != self.role) {
            book.set_roles(std::slice::from_ref(&self.key), self.role);
        }
        let col = book.columns.iter_mut().find(|c| c.key == self.key).unwrap();
        col.long_name.clone_from(&self.long_name);
        col.units.clone_from(&self.units);
        col.comment.clone_from(&self.comment);
        if let (Some(op), Some(mode)) = (col.op.as_mut(), self.recalc) {
            op.mode = mode;
        }
        let offset_changed = col.format.utc_offset != self.format.utc_offset;
        col.format = self.format;
        if offset_changed && col.data.kind() == scicanvas_core::DataKind::DateTime {
            col.set_utc_offset(self.format.utc_offset);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct LayerContents {
    pub graph: u64,
    pub index: usize,
    pub before: scicanvas_core::page::Layer,
    pub draft: scicanvas_core::page::Layer,
    pub selected: Vec<scicanvas_core::plotspec::DataRef>,
    pub active: Option<String>,
    pub error: Option<String>,
}

impl LayerContents {
    fn add_selected(&mut self, project: &scicanvas_core::Project) {
        let mut draft = self.draft.clone();
        for book in &project.books {
            let keys: Vec<_> = self
                .selected
                .iter()
                .filter(|r| r.book == book.id)
                .map(|r| r.col.clone())
                .collect();
            if keys.is_empty() {
                continue;
            }
            let alive = draft.book_id.is_some_and(|id| project.book(id).is_some());
            if let Err(error) = draft.add_book_curves(book, &keys, alive) {
                self.error = Some(error);
                return;
            }
        }
        self.active = draft
            .y_keys
            .iter()
            .find(|key| !self.draft.y_keys.contains(key))
            .cloned()
            .or_else(|| self.active.clone());
        self.draft = draft;
        self.selected.clear();
        self.error = None;
    }

    fn source_name(project: &scicanvas_core::Project, reference: Option<&scicanvas_core::plotspec::DataRef>) -> String {
        reference
            .map(|r| {
                format!(
                    "{} / {}",
                    project.book(r.book).map_or("已删除的表", |b| b.name.as_str()),
                    r.col
                )
            })
            .unwrap_or_else(|| "行号".into())
    }

    fn show(&mut self, ctx: &egui::Context, project: &scicanvas_core::Project, pal: &Palette) -> Action {
        let mut open = true;
        window(ctx, "图层内容")
            .open(&mut open)
            .default_width(360.0)
            .default_height(660.0)
            .show(ctx, |ui| {
                ui.strong(format!(
                    "{} · 图层 {}",
                    project.name_of(self.graph).unwrap_or("图形"),
                    self.index + 1
                ));
                ui.separator();
                ui.strong("可用数据");
                egui::ScrollArea::vertical()
                    .id_salt("contents-data")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        for book in &project.books {
                            egui::CollapsingHeader::new(&book.name)
                                .id_salt(("contents-book", book.id))
                                .default_open(true)
                                .show(ui, |ui| {
                                    for col in &book.columns {
                                        let reference = scicanvas_core::plotspec::DataRef::new(book.id, &col.key);
                                        let present = self.draft.y_keys.iter().any(|key| {
                                            scicanvas_core::plotspec::DataRef::from_key(self.draft.book_id, key)
                                                .as_ref()
                                                == Some(&reference)
                                        });
                                        let mut selected = present || self.selected.contains(&reference);
                                        let name = if col.display_name() == col.key {
                                            format!("{} ({})", col.key, col.role.label())
                                        } else {
                                            format!("{} ({})  {}", col.key, col.role.label(), col.display_name())
                                        };
                                        let response = ui.add_enabled(
                                            matches!(col.role, scicanvas_core::Role::Y | scicanvas_core::Role::Z)
                                                && !present,
                                            egui::Checkbox::new(&mut selected, name),
                                        );
                                        if response.changed() {
                                            self.selected.retain(|r| r != &reference);
                                            if selected {
                                                self.selected.push(reference);
                                            }
                                        }
                                        if present {
                                            response.on_hover_text("已在当前图层");
                                        }
                                    }
                                });
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(!self.selected.is_empty(), plain("添加 →")).clicked() {
                        self.add_selected(project);
                    }
                });
                ui.separator();
                ui.strong("当前图层曲线");
                let keys = self.draft.y_keys.clone();
                egui::ScrollArea::vertical()
                    .id_salt("contents-curves")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for key in &keys {
                            ui.push_id(key, |ui| {
                                let name = self.draft.curve_label(
                                    self.draft.book_id.and_then(|id| project.book(id)),
                                    &|id| project.book(id),
                                    key,
                                );
                                let before = self.draft.curves.get(key).is_some_and(|style| style.right_axis);
                                let mut right_axis = before;
                                ui.horizontal_wrapped(|ui| {
                                    if ui.selectable_label(self.active.as_ref() == Some(key), name).clicked() {
                                        self.active = Some(key.clone());
                                    }
                                    egui::ComboBox::from_id_salt("contents-axis")
                                        .width(72.0)
                                        .selected_text(if right_axis { "右 Y" } else { "左 Y" })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut right_axis, false, "左 Y");
                                            ui.selectable_value(&mut right_axis, true, "右 Y");
                                        });
                                });
                                if right_axis != before {
                                    let style = self.draft.curves.entry(key.clone()).or_default();
                                    style.right_axis = right_axis;
                                    if style.is_default()
                                        && !self
                                            .before
                                            .curves
                                            .get(key)
                                            .is_some_and(scicanvas_core::page::CurveStyle::is_default)
                                    {
                                        self.draft.curves.remove(key);
                                    }
                                }
                                let plot = self.draft.plot_in(key, &|id| project.book(id));
                                let mapping = plot.as_ref().map(|p| &p.mapping);
                                let source = mapping.map(|m| &m.y);
                                egui::Grid::new("contents-source")
                                    .num_columns(2)
                                    .spacing([8.0, 2.0])
                                    .show(ui, |ui| {
                                        ui.label("X 来源");
                                        ui.label(Self::source_name(
                                            project,
                                            mapping.as_ref().and_then(|m| m.x.as_ref()),
                                        ));
                                        ui.end_row();
                                        ui.label("Y 来源");
                                        ui.label(
                                            source
                                                .as_ref()
                                                .map(|r| Self::source_name(project, Some(r)))
                                                .unwrap_or_else(|| "未绑定".into()),
                                        );
                                        ui.end_row();
                                    });
                                ui.separator();
                            });
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    let at = self
                        .active
                        .as_ref()
                        .and_then(|key| self.draft.y_keys.iter().position(|k| k == key));
                    if ui.add_enabled(at.is_some(), plain("← 移除")).clicked() {
                        if let Some(at) = at {
                            self.draft.remove_curve(&keys[at]);
                            self.active = self
                                .draft
                                .y_keys
                                .get(at.min(self.draft.y_keys.len().saturating_sub(1)))
                                .cloned();
                        }
                    }
                    if ui.add_enabled(at.is_some_and(|i| i > 0), plain("上移")).clicked() {
                        let i = at.unwrap();
                        self.draft.spec.y_keys.swap(i, i - 1);
                    }
                    if ui
                        .add_enabled(at.is_some_and(|i| i + 1 < keys.len()), plain("下移"))
                        .clicked()
                    {
                        let i = at.unwrap();
                        self.draft.spec.y_keys.swap(i, i + 1);
                    }
                    if ui.add(plain("按名称排序")).clicked() {
                        let names: std::collections::BTreeMap<_, _> = keys
                            .iter()
                            .map(|key| {
                                (
                                    key.clone(),
                                    self.draft
                                        .curve_label(
                                            self.draft.book_id.and_then(|id| project.book(id)),
                                            &|id| project.book(id),
                                            key,
                                        )
                                        .to_lowercase(),
                                )
                            })
                            .collect();
                        self.draft
                            .spec
                            .y_keys
                            .sort_by_key(|key| names.get(key).cloned().unwrap_or_default());
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(pal.ink, error);
                }
            });
        if open {
            Action::None
        } else {
            Action::Close
        }
    }
}

pub enum Dialog {
    LayerContents(Box<LayerContents>),
    AnalysisCurves {
        graph: u64,
        layer: usize,
        curves: Vec<(scicanvas_core::plotspec::DataRef, String, bool)>,
    },
    /// 列属性：编辑内容、第一行的数值（数值列或日期列的毫秒，用于格式预览）、转换失败等错误。
    ColumnProps {
        book: u64,
        before: ColumnEdit,
        edit: ColumnEdit,
        sample: Option<f64>,
        error: Option<String>,
    },
    ConvertText {
        book: u64,
        key: String,
        row: usize,
        value: String,
    },
    Poly {
        order: usize,
    },
    Smooth {
        method: SmoothMethod,
        window: usize,
    },
    /// 类别顺序：工作簿、列和当前顺序的类别。
    CategoryOrder {
        book: u64,
        key: String,
        levels: Vec<String>,
        error: Option<String>,
    },
    /// IFFT：工作簿、实部列、虚部列（可无）及显示名、频点数、是否双边谱、单边谱原信号点数是否为奇数。
    Ifft {
        book: u64,
        re: (String, String),
        im: Option<(String, String)>,
        rows: usize,
        two_sided: bool,
        odd: bool,
    },
    /// 自定义排序：可选列（标识、显示名）、最多三级关键字（列序号，None 为不用；是否升序）、
    /// 选中的列及是否只排选中列。
    Sort {
        book: u64,
        columns: Vec<(String, String)>,
        keys: [(Option<usize>, bool); 3],
        selected: Vec<String>,
        only_selected: bool,
        error: Option<String>,
    },
    /// Savitzky–Golay 平滑求导：窗口点数、多项式阶数、导数阶数。
    SgDiff {
        window: usize,
        order: usize,
        deriv: usize,
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
        /// Tukey HSD 两两比较（95% 置信）。
        tukey: Vec<TukeyPair>,
    },
    /// 相关系数：每一对列（名称 A、名称 B）的三种相关系数。
    Correlations(Vec<(String, String, Vec<Correlation>)>),
    /// 非线性拟合：选择内置函数；每个参数可固定为指定值（最多 4 个参数）。
    NlFit {
        model: Model,
        /// 每个参数：(固定?, 固定值, 有下限?, 下限, 有上限?, 上限)。
        limits: [(bool, f64, bool, f64, bool, f64); scicanvas_core::nlfit::MAX_PARAMS],
        /// 参数间线性约束，一行一条。
        constraints: String,
    },
    /// 非线性拟合结果。
    NlFitReport(Vec<NlFitReport>),
    /// FFT 滤波：类型（0 低通、1 高通、2 带通、3 带阻）、截止频率与奈奎斯特频率（仅作提示）。
    FftFilter {
        kind: u8,
        lo: f64,
        hi: f64,
        nyquist: f64,
    },
    /// 填充序列：目标列名、类型（0 等差、1 均匀随机、2 正态随机）、两个参数与行数。
    Fill {
        target: String,
        kind: u8,
        a: f64,
        b: f64,
        rows: usize,
    },
    /// 通用结果表：标题、表头、各行和说明，可复制到 Excel。
    Table {
        title: String,
        rows: Vec<Vec<String>>,
        note: String,
    },
    /// 设置列值：目标列名、公式和可引用的列（短名、显示名）。
    SetValues {
        target: String,
        formula: String,
        /// 保存公式，数据变化时自动重算。
        keep: bool,
        columns: Vec<(String, String)>,
        error: Option<String>,
    },
    /// 扣除基线：是否用 ALS、log10 λ 与非对称系数 p。
    Baseline {
        als: bool,
        log_lambda: f64,
        p: f64,
    },
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
    LayerContents(Box<LayerContents>),
    AnalysisCurves(u64, usize, Vec<scicanvas_core::plotspec::DataRef>),
    SaveNote(String, String),
    ResidualPlot(Vec<FitReport>),
    FitBands(Vec<FitReport>),
    NlFitBands(Vec<NlFitReport>),
    ColumnProps(u64, ColumnEdit),
    None,
    Close,
    ConvertText(u64, String, usize, String),
    Fit(usize),
    Smooth(SmoothMethod, usize),
    SgDiff(usize, usize, usize),
    CategoryOrder(u64, String, Vec<String>),
    Sort(u64, Vec<(String, bool)>, Option<Vec<String>>),
    Ifft(u64, String, Option<String>, scicanvas_core::fft::InverseInput),
    Normalize(NormalizeMethod),
    ExportPng(u32),
    Interpolate(InterpMethod, usize, Option<(f64, f64)>),
    TTest(TTestKind, f64),
    Frequency(Option<usize>, Option<(f64, f64)>),
    Fft(FftWindow),
    /// 最小突出度（占 Y 范围的比例）。
    Peaks(f64),
    Baseline(BaselineMethod),
    FftFilter(FilterKind),
    Fill(Sequence, usize),
    SetValues(String, bool),
    NlFit(Model, Vec<scicanvas_core::nlfit::ParamLimits>, String),
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

/// Settings share one docking implementation and keep their existing widget IDs and drafts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DockSide {
    Left,
    Right,
}

#[derive(Clone)]
struct DockPanel {
    id: egui::Id,
    side: Option<DockSide>,
    visible: bool,
    seen: bool,
    rect: Option<egui::Rect>,
    share: f32,
    float_pos: Option<egui::Pos2>,
    collapsed: bool,
    chrome_height: Option<f32>,
    header_height: Option<f32>,
}

#[derive(Clone, Default)]
struct Docks {
    panels: Vec<DockPanel>,
    left: Option<egui::Rect>,
    right: Option<egui::Rect>,
}

fn docks(ctx: &egui::Context) -> Docks {
    ctx.data(|d| d.get_temp(egui::Id::new("settings-docks")))
        .unwrap_or_default()
}

fn store_docks(ctx: &egui::Context, state: Docks) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("settings-docks"), state));
}

pub(crate) fn reset_docks(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<Docks>(egui::Id::new("settings-docks")));
}

pub(crate) fn begin_docks(ctx: &egui::Context) {
    let mut state = docks(ctx);
    state.left = None;
    state.right = None;
    for panel in &mut state.panels {
        panel.seen = false;
        panel.rect = None;
    }
    store_docks(ctx, state);
}

pub(crate) fn end_docks(ctx: &egui::Context) {
    let mut state = docks(ctx);
    let mut changed = false;
    for panel in &mut state.panels {
        if panel.visible && !panel.seen {
            panel.visible = false;
            changed = true;
        }
    }
    store_docks(ctx, state);
    if changed {
        ctx.request_repaint();
    }
}

pub(crate) fn has_docks(ctx: &egui::Context, side: DockSide) -> bool {
    docks(ctx).panels.iter().any(|p| p.visible && p.side == Some(side))
}

fn panel_header_height(ctx: &egui::Context) -> f32 {
    let title = RichText::new("面板").strong();
    let style = ctx.style();
    ctx.fonts(|fonts| title.font_height(fonts, &style))
        .max(style.spacing.interact_size.y)
        + 2.0 * PANEL_MARGIN
}

/// Reserve sidebar space before drawing document windows; each expanded panel gets a scrollable row.
pub(crate) fn place_docks(ui: &mut egui::Ui, side: DockSide, rect: egui::Rect) {
    let ctx = ui.ctx().clone();
    let mut state = docks(&ctx);
    match side {
        DockSide::Left => state.left = Some(rect),
        DockSide::Right => state.right = Some(rect),
    }
    let indices: Vec<_> = state
        .panels
        .iter()
        .enumerate()
        .filter(|(_, p)| p.visible && p.side == Some(side))
        .map(|(i, _)| i)
        .collect();
    let header = panel_header_height(&ctx);
    let expanded: Vec<_> = indices
        .iter()
        .map(|&i| {
            let id = state.panels[i].id.with("collapsing");
            let open = egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, id, true).is_open();
            // Docked rows redistribute space immediately, without animated bodies overlapping their neighbours.
            ctx.animate_bool_with_time(id, open, 0.0);
            open
        })
        .collect();
    let shares: f32 = indices
        .iter()
        .zip(&expanded)
        .filter(|(_, open)| **open)
        .map(|(&i, _)| state.panels[i].share)
        .sum();
    let headers: Vec<_> = indices
        .iter()
        .zip(&expanded)
        .map(|(&i, open)| {
            if *open {
                state.panels[i].chrome_height.unwrap_or(header + 8.0) + 1.0
            } else {
                state.panels[i].header_height.unwrap_or(header)
            }
        })
        .collect();
    let free = (rect.height() - headers.iter().sum::<f32>() - 4.0 * indices.len().saturating_sub(1) as f32).max(0.0);
    let mut y = rect.top();
    for (at, &i) in indices.iter().enumerate() {
        let h = headers[at]
            + if expanded[at] {
                free * state.panels[i].share / shares.max(0.01)
            } else {
                0.0
            };
        let panel_rect = egui::Rect::from_min_size(egui::pos2(rect.left(), y), egui::vec2(rect.width(), h));
        state.panels[i].rect = Some(panel_rect);
        y += h + 4.0;
        if at + 1 < indices.len() && expanded[at] && expanded[at + 1] {
            let divider = egui::Rect::from_min_size(egui::pos2(rect.left(), y - 4.0), egui::vec2(rect.width(), 4.0));
            let response = ui
                .interact(divider, state.panels[i].id.with("dock-divider"), egui::Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
            ui.painter().hline(
                divider.x_range(),
                divider.center().y,
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            if response.dragged() && !response.drag_started() {
                let j = indices[at + 1];
                let pair = state.panels[i].share + state.panels[j].share;
                let delta = response.drag_delta().y * shares / free.max(1.0);
                state.panels[i].share = (state.panels[i].share + delta).clamp(pair * 0.1, pair * 0.9);
                state.panels[j].share = pair - state.panels[i].share;
                ctx.request_repaint();
            }
        }
    }
    store_docks(&ctx, state);
}

fn dock_target(ctx: &egui::Context, state: &Docks, pos: egui::Pos2) -> Option<(DockSide, egui::Rect)> {
    let rect = ctx.available_rect();
    let left = state
        .left
        .unwrap_or_else(|| egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + 64.0, rect.bottom())));
    let right = state
        .right
        .unwrap_or_else(|| egui::Rect::from_min_max(egui::pos2(rect.right() - 64.0, rect.top()), rect.max));
    if left.contains(pos) {
        Some((DockSide::Left, left))
    } else if right.contains(pos) {
        Some((DockSide::Right, right))
    } else {
        None
    }
}

fn dock_panel(ctx: &egui::Context, id: egui::Id, side: Option<DockSide>, pos: Option<egui::Pos2>) {
    let mut state = docks(ctx);
    let Some(at) = state.panels.iter().position(|p| p.id == id) else {
        return;
    };
    let mut panel = state.panels.remove(at);
    panel.side = side;
    panel.rect = None;
    if side.is_none() {
        panel.float_pos = pos;
    }
    let at = side
        .and_then(|side| {
            pos.and_then(|pos| {
                state
                    .panels
                    .iter()
                    .position(|p| p.visible && p.side == Some(side) && p.rect.is_some_and(|r| pos.y < r.center().y))
            })
        })
        .unwrap_or(state.panels.len());
    state.panels.insert(at, panel);
    store_docks(ctx, state);
    ctx.request_repaint();
}

const PANEL_MARGIN: f32 = 8.0;

pub(crate) struct SettingsWindow<'open> {
    window: egui::Window<'open>,
    id: egui::Id,
}

impl<'open> SettingsWindow<'open> {
    pub(crate) fn id(mut self, id: egui::Id) -> Self {
        self.id = id;
        self.window = self.window.id(id);
        self
    }
    pub(crate) fn open(mut self, open: &'open mut bool) -> Self {
        self.window = self.window.open(open);
        self
    }
    pub(crate) fn resizable(mut self, value: bool) -> Self {
        self.window = self.window.resizable(value);
        self
    }
    pub(crate) fn default_width(mut self, width: f32) -> Self {
        self.window = self.window.default_width(width);
        self
    }
    pub(crate) fn default_height(mut self, height: f32) -> Self {
        self.window = self.window.default_height(height);
        self
    }
    pub(crate) fn show<R>(
        mut self,
        ctx: &egui::Context,
        contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<egui::InnerResponse<Option<R>>> {
        let mut state = docks(ctx);
        if !state.panels.iter().any(|p| p.id == self.id) {
            state.panels.push(DockPanel {
                id: self.id,
                side: None,
                visible: true,
                seen: true,
                rect: None,
                share: 1.0,
                float_pos: None,
                collapsed: false,
                chrome_height: None,
                header_height: None,
            });
        }
        let panel = state.panels.iter_mut().find(|p| p.id == self.id).unwrap();
        panel.seen = true;
        panel.visible = true;
        let docked = panel.side;
        let mut dock_size = None;
        if let Some(rect) = panel.rect {
            let size = (rect.size()
                - egui::vec2(
                    2.0 * PANEL_MARGIN,
                    panel
                        .chrome_height
                        .unwrap_or_else(|| panel_header_height(ctx) + 2.0 * PANEL_MARGIN),
                ))
            .max(egui::vec2(1.0, 1.0));
            dock_size = Some(size);
            self.window = self.window.current_pos(rect.min).fixed_size(size);
        } else if let Some(pos) = panel.float_pos.take() {
            self.window = self.window.current_pos(pos);
        }
        store_docks(ctx, state);
        let out = self.window.show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
            ui.spacing_mut().slider_width = 100.0;
            ui.spacing_mut().text_edit_width = 200.0;
            if let Some(size) = dock_size {
                ui.set_min_size(size);
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, dock_size.is_none()])
                .show(ui, contents)
                .inner
        });
        if let Some(out) = &out {
            // The native title consumes clicks for collapsing. Give its middle portion a panel menu,
            // while keeping the native collapse/close buttons and background dragging intact.
            let mut minimized =
                egui::collapsing_header::CollapsingState::load_with_default_open(ctx, self.id.with("collapsing"), true);
            let mut button = out.response.clone();
            button.id = self.id.with("minimize");
            let style = ctx.style();
            let header_height = panel_header_height(ctx);
            let button_size = egui::Vec2::splat(style.spacing.icon_width);
            let close_center = egui::pos2(
                out.response.rect.right() - header_height / 2.0,
                out.response.rect.top() + header_height / 2.0,
            );
            button.rect = egui::Rect::from_center_size(
                close_center - egui::vec2(button_size.x + style.spacing.item_spacing.x, 0.0),
                button_size,
            );
            button.interact_rect = button.rect;
            button.sense = egui::Sense::hover();
            let button = button.interact(egui::Sense::click());
            let painter = ctx.layer_painter(out.response.layer_id);
            let visuals = style.interact(&button);
            let icon_rect = button.rect.shrink(2.0).expand(visuals.expansion);
            let center = icon_rect.center();
            painter.line_segment([icon_rect.left_center(), icon_rect.right_center()], visuals.fg_stroke);
            if !minimized.is_open() {
                painter.line_segment(
                    [
                        egui::pos2(center.x, icon_rect.top()),
                        egui::pos2(center.x, icon_rect.bottom()),
                    ],
                    visuals.fg_stroke,
                );
            }
            let controls_left = button.rect.left() - style.spacing.item_spacing.x;
            if button.clicked() {
                minimized.set_open(!minimized.is_open());
                minimized.store(ctx);
                ctx.request_repaint();
            }
            button.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    button.enabled(),
                    if minimized.is_open() {
                        "最小化面板"
                    } else {
                        "还原面板"
                    },
                )
            });
            button.on_hover_text(if minimized.is_open() {
                "最小化面板"
            } else {
                "还原面板"
            });
            let mut title = out.response.clone();
            title.id = self.id.with("dock-title-menu");
            title.rect = egui::Rect::from_min_max(
                out.response.rect.min + egui::vec2(32.0, 0.0),
                egui::pos2(controls_left, out.response.rect.top() + panel_header_height(ctx)),
            );
            title.interact_rect = title.rect;
            title.sense = egui::Sense::hover();
            let title = title.interact(egui::Sense::click());
            if title.double_clicked() {
                let mut collapse = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ctx,
                    self.id.with("collapsing"),
                    true,
                );
                collapse.set_open(!collapse.is_open());
                collapse.store(ctx);
                ctx.request_repaint();
            }
            title.context_menu(|ui| {
                for (label, side) in [
                    ("吸附到左侧", Some(DockSide::Left)),
                    ("吸附到右侧", Some(DockSide::Right)),
                    ("浮动", None),
                ] {
                    if ui.button(label).clicked() {
                        dock_panel(ctx, self.id, side, Some(out.response.rect.min));
                        ui.close_menu();
                    }
                }
            });
            if out.response.dragged()
                && !out.response.drag_started()
                && docked.is_some()
                && ctx.input(|input| input.pointer.delta().length_sq() > 0.0)
            {
                dock_panel(ctx, self.id, None, None);
            }
            if out.response.dragged() || out.response.drag_stopped() {
                if let Some(pos) = ctx.pointer_interact_pos() {
                    if let Some((side, rect)) = dock_target(ctx, &docks(ctx), pos) {
                        let layer = egui::LayerId::new(egui::Order::Foreground, self.id.with("dock-guide"));
                        let painter = ctx.layer_painter(layer);
                        let color = ctx.style().visuals.selection.bg_fill;
                        painter.rect_filled(rect, 0.0, color.gamma_multiply(0.25));
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0_f32, color));
                        if out.response.drag_stopped() {
                            dock_panel(ctx, self.id, Some(side), Some(pos));
                        }
                    }
                }
            }
            // Collapsing changes the rows reserved by the sidebar on the next frame.
            let mut state = docks(ctx);
            if let Some(panel) = state.panels.iter_mut().find(|p| p.id == self.id) {
                let collapsed = !egui::collapsing_header::CollapsingState::load_with_default_open(
                    ctx,
                    self.id.with("collapsing"),
                    true,
                )
                .is_open();
                if panel.collapsed != collapsed {
                    panel.collapsed = collapsed;
                    ctx.request_repaint();
                }
                if let Some(size) = dock_size.filter(|_| collapsed == out.inner.is_none()) {
                    // Measure native window chrome, including font fallback and DPI rounding.
                    let measured = if collapsed {
                        out.response.rect.height()
                    } else {
                        out.response.rect.height() - size.y
                    };
                    let saved = if collapsed {
                        &mut panel.header_height
                    } else {
                        &mut panel.chrome_height
                    };
                    if saved.is_none_or(|height| (height - measured).abs() > 0.5) {
                        *saved = Some(measured);
                        ctx.request_repaint();
                    }
                }
            }
            store_docks(ctx, state);
        }
        out
    }
}

/// Keep floating titles below the ribbon while leaving the other three sides unrestricted.
pub(crate) fn floating_bounds(ctx: &egui::Context) -> egui::Rect {
    let mut bounds = egui::Rect::EVERYTHING;
    bounds.min.y = ctx.available_rect().top();
    bounds
}

/// Compact single-column settings; drag the title into a sidebar or use the panel context menu.
pub(crate) fn window<'open>(ctx: &egui::Context, title: &str) -> SettingsWindow<'open> {
    let id = egui::Id::new(("scicanvas-dialog", title));
    SettingsWindow {
        id,
        window: egui::Window::new(RichText::new(title).strong())
            .id(id)
            .constrain_to(floating_bounds(ctx))
            .collapsible(true)
            .resizable(true)
            .pivot(Align2::LEFT_TOP)
            .default_pos(egui::pos2(
                ctx.screen_rect().center().x - 190.0,
                ctx.screen_rect().top() + 60.0,
            ))
            .default_width(360.0)
            .min_width(280.0)
            .default_height(ctx.screen_rect().height() * 0.75)
            .max_height((ctx.available_rect().height() - 64.0).max(100.0))
            .vscroll(false)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::same(PANEL_MARGIN))),
    }
}

/// 对话框按钮的统一尺寸。
pub(crate) const BUTTON: egui::Vec2 = egui::vec2(84.0, 28.0);

/// 让接下来 `n` 个统一尺寸的按钮在这一行里居中。
pub(crate) fn center_buttons(ui: &mut egui::Ui, n: usize) {
    let gap = ui.spacing().item_spacing.x;
    let used = n as f32 * BUTTON.x + n.saturating_sub(1) as f32 * gap;
    ui.add_space(((ui.available_width() - used) / 2.0).max(0.0));
}

/// 主按钮：强调色底、白字。
pub(crate) fn primary(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(egui::Color32::WHITE).strong())
        .fill(ui.visuals().selection.stroke.color)
        .min_size(BUTTON)
}

/// 普通按钮。
pub(crate) fn plain(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).min_size(BUTTON)
}

fn buttons(ui: &mut egui::Ui, ok: &str) -> (bool, bool) {
    let mut res = (false, false);
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        center_buttons(ui, 2);
        res.0 = ui.add(primary(ui, ok)).clicked();
        res.1 = ui.add(plain("取消")).clicked();
    });
    res
}

pub fn show(ctx: &egui::Context, dialog: &mut Dialog, pal: &Palette, project: &scicanvas_core::Project) -> Action {
    let mut action = Action::None;
    let mut open = true;
    let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    match dialog {
        Dialog::LayerContents(contents) => action = contents.show(ctx, project, pal),
        Dialog::AnalysisCurves { curves, .. } => {
            window(ctx, "分析曲线").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("全选").clicked() {
                        for (_, _, checked) in curves.iter_mut() {
                            *checked = true;
                        }
                    }
                    if ui.button("清空").clicked() {
                        for (_, _, checked) in curves.iter_mut() {
                            *checked = false;
                        }
                    }
                });
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for (_, name, checked) in curves.iter_mut() {
                        ui.checkbox(checked, name.as_str());
                    }
                });
            });
        }
        Dialog::ColumnProps {
            book: _,
            edit,
            sample,
            error,
            ..
        } => {
            use scicanvas_core::format::{DateFormat, NumberFormat};
            use scicanvas_core::{DataKind, Role};
            let title = format!("列属性 — {}", edit.key);
            window(ctx, &title).open(&mut open).show(ctx, |ui| {
                egui::Grid::new("column-props")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("短名");
                        ui.label(RichText::new(edit.key.as_str()).strong())
                            .on_hover_text("图形通过短名引用列，创建后不再改变");
                        ui.end_row();
                        for (label, field) in [
                            ("长名称", &mut edit.long_name),
                            ("单位", &mut edit.units),
                            ("注释", &mut edit.comment),
                        ] {
                            ui.label(label);
                            ui.add(egui::TextEdit::singleline(field).desired_width(200.0));
                            ui.end_row();
                        }
                        ui.label("列属性");
                        let roles = [
                            (Role::X, "X"),
                            (Role::Y, "Y"),
                            (Role::XError, "X 误差"),
                            (Role::YError, "Y 误差"),
                            (Role::Z, "Z"),
                            (Role::Label, "标签"),
                            (Role::Ignore, "忽略"),
                        ];
                        let current = roles.iter().find(|(r, _)| *r == edit.role).map_or("Y", |(_, n)| *n);
                        egui::ComboBox::from_id_salt("column-role")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (role, name) in roles {
                                    ui.selectable_value(&mut edit.role, role, name);
                                }
                            });
                        ui.end_row();
                        ui.label("数据类型");
                        egui::ComboBox::from_id_salt("column-kind")
                            .selected_text(edit.kind.label())
                            .show_ui(ui, |ui| {
                                for kind in DataKind::ALL {
                                    ui.selectable_value(&mut edit.kind, kind, kind.label());
                                }
                            });
                        ui.end_row();
                        if let Some(mode) = &mut edit.recalc {
                            ui.label("重算方式");
                            ui.horizontal_wrapped(|ui| {
                                for m in scicanvas_core::ops::RecalcMode::ALL {
                                    ui.radio_value(mode, m, m.label());
                                }
                            });
                            ui.end_row();
                        }
                        ui.label("显示格式");
                        match edit.kind {
                            DataKind::Num => {
                                ui.horizontal_wrapped(|ui| {
                                    let mut index = edit.format.number.index();
                                    let mut places = edit.format.number.places();
                                    egui::ComboBox::from_id_salt("column-number-format")
                                        .selected_text(NumberFormat::KINDS[index])
                                        .show_ui(ui, |ui| {
                                            for (i, name) in NumberFormat::KINDS.iter().enumerate() {
                                                ui.selectable_value(&mut index, i, *name);
                                            }
                                        });
                                    ui.add_enabled_ui(index != 0, |ui| {
                                        ui.label("小数位");
                                        ui.add(
                                            egui::DragValue::new(&mut places)
                                                .range(0..=scicanvas_core::format::MAX_PLACES)
                                                .speed(0.1),
                                        );
                                    });
                                    edit.format.number = NumberFormat::from_index(index, places);
                                });
                            }
                            DataKind::DateTime => {
                                ui.horizontal_wrapped(|ui| {
                                    egui::ComboBox::from_id_salt("column-date-format")
                                        .selected_text(edit.format.date.label())
                                        .show_ui(ui, |ui| {
                                            for f in DateFormat::ALL {
                                                ui.selectable_value(&mut edit.format.date, f, f.label());
                                            }
                                        });
                                    use scicanvas_core::format::{utc_offset_label, utc_offsets};
                                    egui::ComboBox::from_id_salt("column-utc-offset")
                                        .selected_text(utc_offset_label(edit.format.utc_offset))
                                        .show_ui(ui, |ui| {
                                            for m in utc_offsets() {
                                                ui.selectable_value(
                                                    &mut edit.format.utc_offset,
                                                    m,
                                                    utc_offset_label(m),
                                                );
                                            }
                                        });
                                });
                            }
                            _ => {
                                ui.label(RichText::new("文本和分类列按原样显示").color(pal.weak));
                            }
                        }
                        ui.end_row();
                        let preview = match (edit.kind, sample) {
                            (DataKind::Num, Some(v)) => {
                                Some(edit.format.number.apply(*v).unwrap_or_else(|| fmt_num(*v)))
                            }
                            (DataKind::DateTime, Some(ms)) => {
                                edit.format.date.apply_in(*ms as i64, edit.format.utc_offset)
                            }
                            _ => None,
                        };
                        if let Some(text) = preview {
                            ui.label("第 1 行显示为");
                            ui.label(RichText::new(text).monospace());
                            ui.end_row();
                        }
                    });
                if let Some(e) = error {
                    ui.add_space(4.0);
                    ui.label(RichText::new(e.as_str()).color(ui.visuals().error_fg_color));
                }
                ui.label(
                    RichText::new("显示格式只改变工作表里的显示，存储的数值和原始写法不变；编辑、复制和导出仍用原值。")
                        .small()
                        .color(pal.weak),
                );
            });
        }
        Dialog::ConvertText { book, key, row, value } => {
            window(ctx, "转换为文本列？").open(&mut open).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("列 {key} 第 {} 行的输入与当前列类型不符", *row + 1))
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new("转换后整列已有内容原样保留，再写入本次输入；可以撤销。").color(pal.weak));
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    // 主按钮文字较长，按实际宽度居中
                    let gap = ui.spacing().item_spacing.x;
                    let wide = egui::vec2(132.0, BUTTON.y);
                    ui.add_space(((ui.available_width() - wide.x - BUTTON.x - gap) / 2.0).max(0.0));
                    if ui.add(primary(ui, "转换为文本并写入").min_size(wide)).clicked() {
                        action = Action::ConvertText(*book, key.clone(), *row, value.clone());
                    }
                    if ui.add(plain("取消")).clicked() || esc {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Poly { order } => {
            window(ctx, "多项式拟合").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
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
            window(ctx, "平滑").open(&mut open).show(ctx, |ui| {
                egui::Grid::new("smooth-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("方法");
                        egui::ComboBox::from_id_salt("smooth-method")
                            .selected_text(method.label())
                            .show_ui(ui, |ui| {
                                for m in SmoothMethod::ALL {
                                    ui.selectable_value(method, m, m.label());
                                }
                            });
                        ui.end_row();
                        ui.label("窗口点数");
                        ui.add(egui::DragValue::new(w).range(3..=101).speed(1.0));
                        ui.end_row();
                    });
                ui.label(
                    RichText::new(match method {
                        SmoothMethod::Lowess => "窗口点数即每点参与局部回归的近邻数。",
                        SmoothMethod::FftFilter => "滤掉高于 1/(窗口点数·ΔX) 的频率，需要等间距 X。",
                        _ => "窗口点数取奇数。Savitzky-Golay 能更好地保留峰形。",
                    })
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
        Dialog::CategoryOrder {
            book: _,
            key: _,
            levels,
            error,
        } => {
            window(ctx, "类别顺序").open(&mut open).show(ctx, |ui| {
                let mut swap = None;
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    egui::Grid::new("category-order")
                        .num_columns(3)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            let n = levels.len();
                            for (i, name) in levels.iter().enumerate() {
                                ui.label(RichText::new(format!("{}", i + 1)).color(pal.weak));
                                ui.label(name);
                                ui.horizontal_wrapped(|ui| {
                                    if ui.add_enabled(i > 0, egui::Button::new("↑").small()).clicked() {
                                        swap = Some((i, i - 1));
                                    }
                                    if ui.add_enabled(i + 1 < n, egui::Button::new("↓").small()).clicked() {
                                        swap = Some((i, i + 1));
                                    }
                                });
                                ui.end_row();
                            }
                        });
                });
                if let Some((a, b)) = swap {
                    levels.swap(a, b);
                }
                if ui.add(plain("按名称排序")).clicked() {
                    levels.sort();
                }
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
            });
        }
        Dialog::Sort {
            book,
            columns,
            keys,
            selected,
            only_selected,
            error,
        } => {
            window(ctx, "排序").open(&mut open).show(ctx, |ui| {
                egui::Grid::new("sort-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        for (level, (col, asc)) in keys.iter_mut().enumerate() {
                            ui.label(["主关键字", "次关键字", "第三关键字"][level]);
                            let text = col.map_or("无".to_string(), |i| columns[i].1.clone());
                            egui::ComboBox::from_id_salt(("sort-key", level))
                                .selected_text(text)
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    if level > 0 {
                                        ui.selectable_value(col, None, "无");
                                    }
                                    for (i, (_, label)) in columns.iter().enumerate() {
                                        ui.selectable_value(col, Some(i), label);
                                    }
                                });
                            ui.end_row();
                            ui.label("方向");
                            ui.horizontal(|ui| {
                                ui.radio_value(asc, true, "升序");
                                ui.radio_value(asc, false, "降序");
                            });
                            ui.end_row();
                        }
                    });
                if !selected.is_empty() {
                    ui.checkbox(
                        only_selected,
                        format!("只排选中列（{}），其余列不动", selected.join("、")),
                    );
                }
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                }
                let (ok, cancel) = buttons(ui, "排序");
                if ok {
                    let keys: Vec<(String, bool)> = keys
                        .iter()
                        .filter_map(|(c, asc)| c.map(|i| (columns[i].0.clone(), *asc)))
                        .collect();
                    let within = (*only_selected && !selected.is_empty()).then(|| selected.clone());
                    action = Action::Sort(*book, keys, within);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Ifft {
            book,
            re,
            im,
            rows,
            two_sided,
            odd,
        } => {
            window(ctx, "IFFT").open(&mut open).show(ctx, |ui| {
                egui::Grid::new("ifft-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("实部");
                        ui.label(&re.1);
                        ui.end_row();
                        ui.label("虚部");
                        ui.label(im.as_ref().map_or("无（按 0）", |c| c.1.as_str()));
                        ui.end_row();
                        ui.label("输入");
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(two_sided, false, "单边谱");
                            ui.radio_value(two_sided, true, "双边谱");
                        });
                        ui.end_row();
                        if !*two_sided {
                            use scicanvas_core::fft::InverseInput;
                            ui.label("输出点数");
                            ui.horizontal_wrapped(|ui| {
                                let even = InverseInput::OneSided { odd: false }.points(*rows);
                                let oddn = InverseInput::OneSided { odd: true }.points(*rows);
                                ui.radio_value(odd, false, even.to_string());
                                ui.radio_value(odd, true, oddn.to_string());
                            });
                            ui.end_row();
                        }
                    });
                let (ok, cancel) = buttons(ui, "变换");
                if ok {
                    let input = if *two_sided {
                        scicanvas_core::fft::InverseInput::TwoSided
                    } else {
                        scicanvas_core::fft::InverseInput::OneSided { odd: *odd }
                    };
                    action = Action::Ifft(*book, re.0.clone(), im.as_ref().map(|c| c.0.clone()), input);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::SgDiff {
            window: w,
            order,
            deriv,
        } => {
            window(ctx, "SG 平滑求导").open(&mut open).show(ctx, |ui| {
                egui::Grid::new("sgdiff-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("导数阶数");
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(deriv, 1, "一阶");
                            ui.radio_value(deriv, 2, "二阶");
                        });
                        ui.end_row();
                        ui.label("窗口点数");
                        ui.add(egui::DragValue::new(w).range(3..=101).speed(1.0));
                        ui.end_row();
                        ui.label("多项式阶数");
                        ui.add(egui::DragValue::new(order).range(1..=6).speed(0.1));
                        ui.end_row();
                    });
                // 窗口取奇数；阶数不低于导数阶数，并小于窗口点数 − 1
                *w = (*w).max(3) | 1;
                *order = (*order).clamp((*deriv).max(1), *w - 2);
                let (ok, cancel) = buttons(ui, "求导");
                if ok {
                    action = Action::SgDiff(*w, *order, *deriv);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Normalize { method } => {
            window(ctx, "归一化").open(&mut open).show(ctx, |ui| {
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
            window(ctx, "拟合结果").open(&mut open).show(ctx, |ui| {
                egui::ScrollArea::both()
                    .max_height((ctx.available_rect().height() - 220.0).clamp(100.0, 480.0))
                    .show(ui, |ui| {
                        for (ri, r) in reports.iter().enumerate() {
                            ui.label(RichText::new(format!("{} · {}", r.name, r.kind)).strong());
                            table_grid(ui, ("fit-grid", ri), &fit_param_rows(r, false), pal);
                            ui.add_space(4.0);
                            let mut stats = vec![
                                ("R²".to_string(), fmt_num(r.fit.r2)),
                                ("调整后 R²".to_string(), fmt_num(r.fit.adj_r2)),
                            ];
                            if let Some(pr) = r.fit.pearson_r() {
                                stats.push(("Pearson r".into(), fmt_num(pr)));
                            }
                            stats.push(("残差平方和".into(), fmt_num(r.fit.rss)));
                            stats.push(("数据点数 N".into(), r.fit.n.to_string()));
                            let stats: Vec<Vec<String>> = stats.into_iter().map(|(k, v)| vec![k, v]).collect();
                            table_grid(
                                ui,
                                ("fit-stats", ri),
                                &[vec!["统计量".into(), "值".into()]]
                                    .into_iter()
                                    .chain(stats)
                                    .collect::<Vec<_>>(),
                                pal,
                            );
                            ui.add_space(4.0);
                            ui.label(RichText::new("方差分析").color(pal.weak));
                            table_grid(ui, ("fit-anova", ri), &fit_anova_rows(r), pal);
                            ui.add_space(4.0);
                            ui.label(RichText::new(r.fit.equation()).monospace());
                            ui.add_space(10.0);
                        }
                    });
                ui.label(
                    RichText::new("拟合曲线和逐点残差已写入工作簿，拟合曲线加到图形中。")
                        .small()
                        .color(pal.weak),
                );
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 2);
                    if ui.add(plain("残差图")).clicked() {
                        action = Action::ResidualPlot(reports.clone());
                    }
                    if ui.add(plain("置信/预测带")).on_hover_text("95%").clicked() {
                        action = Action::FitBands(reports.clone());
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        ui.ctx().copy_text(fit_text(reports));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        action = Action::SaveNote("拟合结果".into(), fit_markdown(reports));
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Areas(areas) => {
            window(ctx, "积分结果").open(&mut open).show(ctx, |ui| {
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
            window(ctx, "插值").open(&mut open).show(ctx, |ui| {
                egui::Grid::new("interp-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("方法");
                        ui.horizontal_wrapped(|ui| {
                            for m in InterpMethod::ALL {
                                ui.radio_value(method, m, m.label());
                            }
                        });
                        ui.end_row();
                        ui.label("点数");
                        ui.add(egui::DragValue::new(points).range(2..=100_000).speed(1.0));
                        ui.end_row();
                        ui.label("X 范围");
                        ui.horizontal_wrapped(|ui| {
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
        Dialog::Anova { result, names, tukey } => {
            window(ctx, "单因素方差分析").open(&mut open).show(ctx, |ui| {
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
                        "{verdict}。每个点选的列作为一组，缺失值已跳过。\nTukey HSD 的 p 值已按多重比较校正，组大小不等时为 Tukey–Kramer。"
                    ))
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let (table, groups) = anova_rows(result, names);
                        let pairs = tukey_rows(tukey, names);
                        let text: Vec<String> = table
                            .iter()
                            .chain([vec![]].iter())
                            .chain(groups.iter())
                            .chain([vec![]].iter())
                            .chain(pairs.iter())
                            .map(|r| r.join("\t"))
                            .collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let (table, groups) = anova_rows(result, names);
                        let pairs = tukey_rows(tukey, names);
                        let text = format!(
                            "## 单因素方差分析\n\n{}\n### 各组统计量\n\n{}\n### Tukey HSD 多重比较\n\n{}",
                            md_table(&table),
                            md_table(&groups),
                            md_table(&pairs)
                        );
                        action = Action::SaveNote("方差分析结果".into(), text);
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Correlations(pairs) => {
            window(ctx, "相关系数").open(&mut open).show(ctx, |ui| {
                egui::ScrollArea::both().max_height(420.0).show(ui, |ui| {
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
                ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let mut lines = vec![correlation_header().join("\t")];
                        lines.extend(correlation_rows(pairs).into_iter().map(|r| r.join("\t")));
                        ui.ctx().copy_text(lines.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let mut rows = vec![correlation_header().map(String::from).to_vec()];
                        rows.extend(correlation_rows(pairs));
                        let text = format!("## 相关系数\n\n{}", md_table(&rows));
                        action = Action::SaveNote("相关系数结果".into(), text);
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::NlFit {
            model,
            limits,
            constraints,
        } => {
            window(ctx, "非线性拟合").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("类别");
                    let mut category = model.category();
                    egui::ComboBox::from_id_salt("nlfit-category")
                        .selected_text(category)
                        .show_ui(ui, |ui| {
                            for c in Model::CATEGORIES {
                                ui.selectable_value(&mut category, c, c);
                            }
                        });
                    if category != model.category() {
                        if let Some(m) = Model::ALL.into_iter().find(|m| m.category() == category) {
                            *model = m;
                        }
                    }
                });
                egui::Grid::new("nlfit-models")
                    .num_columns(2)
                    .max_col_width((ui.available_width() - 120.0).max(100.0))
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        let category = model.category();
                        for m in Model::ALL.into_iter().filter(|m| m.category() == category) {
                            ui.radio_value(model, m, m.name());
                            ui.add(egui::Label::new(RichText::new(m.formula()).monospace()).wrap());
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
                for (name, (fix, fv, has_lo, lo, has_hi, hi)) in model.params().iter().zip(limits.iter_mut()) {
                    ui.strong(*name);
                    let free = !*fix;
                    egui::Grid::new(("nlfit-limits", name)).num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        for (label, on, value, enabled) in [("固定", fix, fv, true), ("下限", has_lo, lo, free), ("上限", has_hi, hi, free)] {
                            ui.add_enabled_ui(enabled, |ui| { ui.checkbox(on, label); });
                            ui.add_enabled(enabled && *on, egui::DragValue::new(value).speed(0.01));
                            ui.end_row();
                        }
                    });
                }
                ui.add_space(4.0);
                ui.label(RichText::new("参数约束").color(pal.weak));
                ui.add(
                    egui::TextEdit::multiline(constraints)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("每行一条，如 A1 + A2 = 1、t2 >= t1"),
                );
                let (ok, cancel) = buttons(ui, "拟合");
                if ok {
                    let held = model
                        .params()
                        .iter()
                        .zip(limits.iter())
                        .map(|(_, &(fix, fv, has_lo, lo, has_hi, hi))| scicanvas_core::nlfit::ParamLimits {
                            fixed: fix.then_some(fv),
                            lower: (!fix && has_lo).then_some(lo),
                            upper: (!fix && has_hi).then_some(hi),
                        })
                        .collect();
                    action = Action::NlFit(*model, held, constraints.clone());
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::NlFitReport(reports) => {
            window(ctx, "非线性拟合结果").open(&mut open).show(ctx, |ui| {
                egui::ScrollArea::both()
                    .max_height((ctx.available_rect().height() - 220.0).clamp(100.0, 480.0))
                    .show(ui, |ui| {
                        for (ri, r) in reports.iter().enumerate() {
                            let f = &r.fit;
                            ui.label(RichText::new(format!("{} · {}", r.name, f.model.name())).strong());
                            ui.label(RichText::new(f.model.formula()).monospace().small());
                            for c in &f.constraints {
                                ui.label(RichText::new(format!("约束：{c}")).small().color(pal.weak));
                            }
                            if !f.converged {
                                ui.colored_label(
                                    egui::Color32::from_rgb(182, 67, 66),
                                    "迭代 500 次仍未收敛，结果仅供参考；可换用其他函数或检查数据",
                                );
                            }
                            table_grid(ui, ("nlfit-grid", ri), &nlfit_param_rows(f), pal);
                            ui.add_space(4.0);
                            let stats: Vec<Vec<String>> = [vec!["统计量".to_string(), "值".to_string()]]
                                .into_iter()
                                .chain(nlfit_stats(f).into_iter().map(|(k, v)| vec![k.to_string(), v]))
                                .collect();
                            table_grid(ui, ("nlfit-stats", ri), &stats, pal);
                            ui.add_space(10.0);
                        }
                    });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 1);
                    if ui.add(plain("置信/预测带")).on_hover_text("95%").clicked() {
                        action = Action::NlFitBands(reports.clone());
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        ui.ctx().copy_text(nlfit_text(reports));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        action = Action::SaveNote("非线性拟合结果".into(), nlfit_markdown(reports));
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::FftFilter { kind, lo, hi, nyquist } => {
            window(ctx, "FFT 滤波").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (i, name) in ["低通", "高通", "带通", "带阻"].into_iter().enumerate() {
                        ui.radio_value(kind, i as u8, name);
                    }
                });
                let band = *kind >= 2;
                egui::Grid::new("fft-filter-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(if band { "下限频率" } else { "截止频率" });
                    ui.add(egui::DragValue::new(lo).range(0.0..=f64::MAX).speed(*nyquist / 200.0));
                    ui.end_row();
                    if band {
                        ui.label("上限频率");
                        ui.add(egui::DragValue::new(hi).range(0.0..=f64::MAX).speed(*nyquist / 200.0));
                        ui.end_row();
                    }
                });
                ui.label(
                    RichText::new(format!(
                        "奈奎斯特频率 {}（频率单位为 1/X，X 以 s 为单位时为 Hz）。\n对当前图形的数据曲线，或工作簿里选中的 Y 列做理想滤波：变换后把不通过的频率分量置零再逆变换。\n结果写成新列；来源是图形时也画进图里。X 需要等间距。",
                        fmt_num(*nyquist)
                    ))
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "滤波");
                if ok {
                    let (a, b) = (lo.min(*hi), lo.max(*hi));
                    action = Action::FftFilter(match kind {
                        0 => FilterKind::LowPass(*lo),
                        1 => FilterKind::HighPass(*lo),
                        2 => FilterKind::BandPass(a, b),
                        _ => FilterKind::BandStop(a, b),
                    });
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Fill {
            target,
            kind,
            a,
            b,
            rows,
        } => {
            window(ctx, "填充数据序列").open(&mut open).show(ctx, |ui| {
                ui.label(RichText::new(format!("目标列：{target}")).strong());
                ui.horizontal_wrapped(|ui| {
                    for (i, name) in ["等差序列", "均匀分布随机数", "正态分布随机数"].into_iter().enumerate()
                    {
                        ui.radio_value(kind, i as u8, name);
                    }
                });
                let (la, lb) = match kind {
                    0 => ("首项", "公差"),
                    1 => ("最小值", "最大值"),
                    _ => ("均值", "标准差"),
                };
                egui::Grid::new("fill-grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(la);
                        ui.add(egui::DragValue::new(a).speed(0.1));
                        ui.end_row();
                        ui.label(lb);
                        ui.add(egui::DragValue::new(b).speed(0.1));
                        ui.end_row();
                        ui.label("行数");
                        ui.add(egui::DragValue::new(rows).range(1..=1_000_000).speed(1.0));
                        ui.end_row();
                    });
                ui.label(
                    RichText::new("结果覆盖目标列，可撤销。随机数每次生成都不同。")
                        .small()
                        .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "填充");
                if ok {
                    let seq = match kind {
                        0 => Sequence::Arithmetic { start: *a, step: *b },
                        1 => Sequence::Uniform {
                            min: a.min(*b),
                            max: a.max(*b),
                        },
                        _ => Sequence::Normal { mean: *a, sd: b.abs() },
                    };
                    action = Action::Fill(seq, *rows);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Table { title, rows, note } => {
            window(ctx, title).open(&mut open).show(ctx, |ui| {
                egui::ScrollArea::both().max_height(420.0).show(ui, |ui| {
                    egui::Grid::new("result-table")
                        .striped(true)
                        .num_columns(rows.first().map_or(1, Vec::len))
                        .spacing([16.0, 4.0])
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
                });
                ui.label(RichText::new(note.as_str()).small().color(pal.weak));
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let text: Vec<String> = rows.iter().map(|r| r.join("\t")).collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let text = format!("## {title}\n\n{}\n{note}", md_table(rows));
                        action = Action::SaveNote(title.clone(), text.trim_end().to_string());
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::SetValues {
            target,
            formula,
            keep,
            columns,
            error,
        } => {
            window(ctx, "设置列值").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{target} =")).strong());
                    let edit = ui.add(
                        egui::TextEdit::singleline(formula)
                            .desired_width(340.0)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("例如 A * 2 + sin(B)"),
                    );
                    if edit.changed() {
                        *error = None;
                    }
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        action = Action::SetValues(formula.clone(), *keep);
                    }
                });
                if let Some(e) = error {
                    ui.label(RichText::new(e.as_str()).color(ui.visuals().error_fg_color));
                }
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("插入").small().color(pal.weak));
                    for (key, name) in columns.iter() {
                        if ui.small_button(key.as_str()).on_hover_text(name.as_str()).clicked() {
                            formula.push_str(key);
                        }
                    }
                    // 函数目录：按类别列出，点击插入“函数名(”
                    ui.menu_button("函数", |ui| {
                        use scicanvas_core::formula::{CATEGORIES, FUNCTIONS};
                        for category in CATEGORIES {
                            ui.menu_button(category, |ui| {
                                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                                    for func in FUNCTIONS.iter().filter(|f| f.category == category) {
                                        if ui.button(RichText::new(func.usage).monospace()).clicked() {
                                            formula.push_str(func.name);
                                            formula.push('(');
                                            ui.close_menu();
                                        }
                                    }
                                });
                            });
                        }
                    });
                });
                ui.label(
                    RichText::new(
                        "列用短名（A、B 或 col(A)），i 为行号，常数 pi、e；比较 < > <= >= == != 结果为 1 或 0。\n结果覆盖目标列，无效结果留空，可撤销。",
                    )
                    .small()
                    .color(pal.weak),
                );
                ui.checkbox(keep, "保存公式，引用的列变化时自动重算");
                let (ok, cancel) = buttons(ui, "计算");
                if ok {
                    action = Action::SetValues(formula.clone(), *keep);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Baseline { als, log_lambda, p } => {
            window(ctx, "扣除基线").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.radio_value(als, false, "两端连线");
                    ui.radio_value(als, true, "自动基线（ALS）");
                });
                ui.add_enabled_ui(*als, |ui| {
                    egui::Grid::new("als-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("平滑度 log₁₀ λ");
                        ui.add(egui::DragValue::new(log_lambda).range(0.0..=12.0).speed(0.05).fixed_decimals(1));
                        ui.end_row();
                        ui.label("非对称系数 p");
                        ui.add(egui::DragValue::new(p).range(0.0001..=0.5).speed(0.001).fixed_decimals(4));
                        ui.end_row();
                    });
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列求基线，在 Y 右边写入“基线”和“扣除基线”两列；\n来源是图形时基线也画进图里。ALS（非对称最小二乘）适合光谱等峰朝上的数据：\nλ 越大基线越平直（常用 10²–10⁹），p 越小基线越贴近谷底（常用 0.001–0.05）。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "扣除");
                if ok {
                    action = Action::Baseline(if *als {
                        BaselineMethod::Als {
                            lambda: 10f64.powf(*log_lambda),
                            p: *p,
                        }
                    } else {
                        BaselineMethod::Line
                    });
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Peaks { percent } => {
            window(ctx, "寻峰与峰积分").open(&mut open).show(ctx, |ui| {
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
                    action = Action::Peaks(*percent / 100.0);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Fft { window: w } => {
            window(ctx, "FFT").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
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
            window(ctx, "列运算").open(&mut open).show(ctx, |ui| {
                if b.is_none() {
                    *use_constant = true;
                }
                ui.horizontal_wrapped(|ui| {
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
            window(ctx, "频数统计").open(&mut open).show(ctx, |ui| {
                egui::Grid::new("freq-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("区间数");
                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(auto, "自动（Sturges）");
                            ui.add_enabled(!*auto, egui::DragValue::new(bins).range(1..=10_000));
                        });
                        ui.end_row();
                        ui.label("统计范围");
                        ui.horizontal_wrapped(|ui| {
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
            window(ctx, "t 检验").open(&mut open).show(ctx, |ui| {
                const KINDS: [&str; 3] = ["单样本", "双样本（独立）", "配对"];
                ui.label(format!("参与检验的列：{}", columns.join("、")));
                ui.horizontal_wrapped(|ui| {
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
                        ui.horizontal_wrapped(|ui| {
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
            window(ctx, "t 检验结果").open(&mut open).show(ctx, |ui| {
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
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let text: Vec<String> = rows.iter().map(|(k, v)| format!("{k}\t{v}")).collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let text = format!("## t 检验\n\n{}", md_pairs(["项目", "值"], &rows));
                        action = Action::SaveNote("t 检验结果".into(), text);
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Png { dpi, page_mm } => {
            window(ctx, "导出 PNG").open(&mut open).show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
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
    if !open || (esc && matches!(action, Action::None)) {
        action = Action::Close;
    }
    action
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

/// 标准误文字：固定参数写“固定”，停在上下限上的参数后面注明“（边界）”。
fn nlfit_se(f: &scicanvas_core::nlfit::NlFit, k: usize) -> String {
    if f.fixed[k] {
        "固定".into()
    } else if f.at_bound[k] {
        format!("{}（边界）", fmt_num(f.se[k]))
    } else {
        fmt_num(f.se[k])
    }
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

/// Markdown 表格单元格：竖线转义，换行改成空格。
fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace(['\r', '\n'], " ")
}

/// Markdown 表格（首行为表头），末尾带换行；空表返回空串。
pub(crate) fn md_table(rows: &[Vec<String>]) -> String {
    let Some(head) = rows.first() else { return String::new() };
    let line = |r: &Vec<String>| {
        let cells: Vec<String> = (0..head.len())
            .map(|i| md_cell(r.get(i).map_or("", String::as_str)))
            .collect();
        format!("| {} |\n", cells.join(" | "))
    };
    let mut out = line(head);
    out.push_str(&format!("|{}\n", " --- |".repeat(head.len())));
    for r in &rows[1..] {
        out.push_str(&line(r));
    }
    out
}

/// 两列“名称 | 值”的 Markdown 表格。
fn md_pairs(head: [&str; 2], rows: &[(String, String)]) -> String {
    let mut table = vec![head.map(String::from).to_vec()];
    table.extend(rows.iter().map(|(k, v)| vec![k.clone(), v.clone()]));
    md_table(&table)
}

/// 结果表格：首行为表头（淡色），首列为名称，其余等宽数字。
fn table_grid(ui: &mut egui::Ui, id: impl std::hash::Hash, rows: &[Vec<String>], pal: &Palette) {
    egui::Grid::new(id)
        .striped(true)
        .num_columns(rows.first().map_or(1, Vec::len))
        .spacing([20.0, 4.0])
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
}

/// 线性/多项式参数表：参数、值、标准误、t、p。`exact` 为真时数值保留全部位数（复制到 Excel 用）。
fn fit_param_rows(r: &FitReport, exact: bool) -> Vec<Vec<String>> {
    let num = |v: f64| if exact { format!("{v}") } else { fmt_num(v) };
    let linear = r.fit.coef.len() == 2;
    let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
    for (k, ((c, se), (t, p))) in r.fit.coef.iter().zip(&r.fit.se).zip(r.fit.t_tests()).enumerate() {
        let name = match (linear, k) {
            (true, 0) => "截距 A".to_string(),
            (true, _) => "斜率 B".to_string(),
            (false, 0) => "B0（常数项）".to_string(),
            _ => format!("B{k}"),
        };
        rows.push(vec![name, num(*c), num(*se), num(t), num(p)]);
    }
    rows
}

/// 拟合方差分析表：来源、自由度、平方和、均方、F、p。
fn fit_anova_rows(r: &FitReport) -> Vec<Vec<String>> {
    let a = r.fit.anova();
    vec![
        ["来源", "自由度", "平方和", "均方", "F", "p"]
            .map(String::from)
            .to_vec(),
        vec![
            "回归".into(),
            fmt_num(a.df_reg),
            fmt_num(a.ss_reg),
            fmt_num(a.ms_reg()),
            fmt_num(a.f),
            fmt_num(a.p),
        ],
        vec![
            "残差".into(),
            fmt_num(a.df_err),
            fmt_num(a.ss_err),
            fmt_num(a.ms_err()),
            String::new(),
            String::new(),
        ],
        vec![
            "总计".into(),
            fmt_num(a.df_tot),
            fmt_num(a.ss_tot),
            String::new(),
            String::new(),
            String::new(),
        ],
    ]
}

/// 非线性参数表：参数、值、标准误、t、p（固定参数只写值）。
fn nlfit_param_rows(f: &scicanvas_core::nlfit::NlFit) -> Vec<Vec<String>> {
    let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
    for (k, (name, (t, p))) in f.model.params().iter().zip(f.t_tests()).enumerate() {
        let (t, p) = if f.fixed[k] {
            (String::new(), String::new())
        } else {
            (fmt_num(t), fmt_num(p))
        };
        rows.push(vec![name.to_string(), fmt_num(f.params[k]), nlfit_se(f, k), t, p]);
    }
    rows
}

fn nlfit_markdown(reports: &[NlFitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let f = &r.fit;
            let stats: Vec<(String, String)> = nlfit_stats(f).into_iter().map(|(k, v)| (k.to_string(), v)).collect();
            let constraints: String = f.constraints.iter().map(|c| format!("约束：`{c}`\n\n")).collect();
            format!(
                "## {} — {}\n\n`{}`\n\n{}{}\n{}",
                md_cell(&r.name),
                f.model.name(),
                f.model.formula(),
                constraints,
                md_table(&nlfit_param_rows(f)),
                md_pairs(["统计量", "值"], &stats)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn fit_markdown(reports: &[FitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let stats = [
                ("R²", fmt_num(r.fit.r2)),
                ("调整后 R²", fmt_num(r.fit.adj_r2)),
                ("残差平方和", fmt_num(r.fit.rss)),
                ("N", r.fit.n.to_string()),
            ]
            .map(|(k, v)| (k.to_string(), v));
            format!(
                "## {} — {}\n\n`{}`\n\n{}\n{}\n### 方差分析\n\n{}",
                md_cell(&r.name),
                r.kind,
                r.fit.equation(),
                md_table(&fit_param_rows(r, false)),
                md_pairs(["统计量", "值"], &stats),
                md_table(&fit_anova_rows(r))
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn nlfit_text(reports: &[NlFitReport]) -> String {
    let mut lines = vec![];
    for r in reports {
        let f = &r.fit;
        lines.push(format!("{}\t{}\t{}", r.name, f.model.name(), f.model.formula()));
        lines.extend(f.constraints.iter().map(|c| format!("约束\t{c}")));
        lines.extend(nlfit_param_rows(f).iter().map(|row| row.join("\t")));
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
            let mut lines = vec![format!("{}\t{}", r.name, r.kind), r.fit.equation()];
            lines.extend(fit_param_rows(r, true).iter().map(|row| row.join("\t")));
            lines.push(format!("R²\t{}", r.fit.r2));
            lines.push(format!("调整后 R²\t{}", r.fit.adj_r2));
            lines.push(format!("残差平方和\t{}", r.fit.rss));
            lines.push(format!("N\t{}", r.fit.n));
            lines.push(String::new());
            lines.extend(fit_anova_rows(r).iter().map(|row| row.join("\t")));
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod markdown_tests {
    use super::*;

    #[test]
    fn markdown_tables_escape_cells_and_pad_short_rows() {
        let rows = vec![
            vec!["列 A".to_string(), "p".to_string()],
            vec!["a|b".to_string(), "0.01".to_string()],
            vec!["两\n行".to_string()],
        ];
        assert_eq!(
            md_table(&rows),
            "| 列 A | p |\n| --- | --- |\n| a\\|b | 0.01 |\n| 两 行 |  |\n"
        );
        assert_eq!(md_table(&[]), "");
        assert_eq!(
            md_pairs(["项目", "值"], &[("t".into(), "2".into())]),
            "| 项目 | 值 |\n| --- | --- |\n| t | 2 |\n"
        );
    }
}

#[cfg(test)]
mod docking_tests {
    use super::*;

    struct Host {
        ctx: egui::Context,
        harness: crate::test_ui::Harness,
        second: bool,
        size: egui::Vec2,
    }
    impl Host {
        fn new() -> Self {
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            ctx.style_mut(|s| s.animation_time = 0.0);
            Self {
                harness: crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(1200.0, 900.0)), ()).with_step(0.04),
                ctx,
                second: false,
                size: egui::vec2(1200.0, 900.0),
            }
        }
        fn draw(ctx: &egui::Context, second: bool) {
            begin_docks(ctx);
            egui::TopBottomPanel::top("test-ribbon")
                .exact_height(90.0)
                .show(ctx, |ui| {
                    ui.label("Ribbon");
                });
            egui::SidePanel::left("test-left").exact_width(360.0).show(ctx, |ui| {
                ui.add_space(180.0);
                place_docks(ui, DockSide::Left, ui.available_rect_before_wrap());
            });
            if has_docks(ctx, DockSide::Right) {
                egui::SidePanel::right("test-right").exact_width(360.0).show(ctx, |ui| {
                    place_docks(ui, DockSide::Right, ui.available_rect_before_wrap())
                });
            }
            egui::CentralPanel::default().show(ctx, |_| {});
            for title in if second { vec!["First", "Second"] } else { vec!["First"] } {
                window(ctx, title).show(ctx, |ui| {
                    for i in 0..40 {
                        ui.label(format!("Setting {i}"));
                    }
                });
            }
            end_docks(ctx);
        }
        fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
            self.harness.size = Some(self.size);
            let second = self.second;
            self.harness.run(events, |ctx| Self::draw(ctx, second)).0
        }
        fn settle(&mut self) {
            for _ in 0..6 {
                self.frame(vec![]);
            }
        }
        fn rect(&self, title: &str) -> egui::Rect {
            self.ctx
                .memory(|m| m.area_rect(egui::Id::new(("scicanvas-dialog", title))))
                .unwrap()
        }
        fn drag(&mut self, start: egui::Pos2, end: egui::Pos2) {
            self.harness.size = Some(self.size);
            let second = self.second;
            self.harness
                .drag(start, start + egui::vec2(30.0, 0.0), end, |ctx| Self::draw(ctx, second));
            self.settle();
        }
        fn title_drag(&mut self, title: &str, end: egui::Pos2) {
            let r = self.rect(title);
            self.drag(egui::pos2(r.center().x, r.top() + 12.0), end);
        }
    }

    #[test]
    fn floating_titles_stop_at_the_ribbon_but_other_edges_remain_free() {
        let mut host = Host::new();
        host.settle();
        host.title_drag("First", egui::pos2(700.0, -120.0));
        let rect = host.rect("First");
        assert_eq!(rect.top(), host.ctx.available_rect().top());
        assert!(rect.is_finite());
        let id = egui::Id::new(("scicanvas-dialog", "First"));
        assert_eq!(docks(&host.ctx).panels.iter().find(|p| p.id == id).unwrap().side, None);
        for end in [
            egui::pos2(-180.0, 400.0),
            egui::pos2(1380.0, 400.0),
            egui::pos2(700.0, 1080.0),
        ] {
            dock_panel(&host.ctx, id, None, Some(egui::pos2(450.0, 150.0)));
            host.settle();
            host.title_drag("First", end);
            let rect = host.rect("First");
            if end.x < 0.0 {
                assert!(rect.left() < 0.0);
            } else if end.x > 1200.0 {
                assert!(rect.right() > 1200.0);
            } else {
                assert!(rect.bottom() > 900.0);
            }
            assert!(rect.top() >= host.ctx.available_rect().top());
            assert_eq!(docks(&host.ctx).panels.iter().find(|p| p.id == id).unwrap().side, None);
        }
    }

    #[test]
    fn actual_title_drags_dock_stack_reorder_resize_collapse_and_float() {
        let mut host = Host::new();
        host.settle();
        host.title_drag("First", egui::pos2(160.0, 430.0));
        assert!(has_docks(&host.ctx, DockSide::Left));
        let first = host.rect("First");
        assert!(
            first.left() >= 0.0 && first.right() <= 360.0 && first.top() >= 180.0,
            "{first:?}"
        );
        host.second = true;
        host.settle();
        host.title_drag("Second", egui::pos2(160.0, 800.0));
        let first = host.rect("First");
        let second = host.rect("Second");
        assert!(first.bottom() <= second.top() + 1.0, "{first:?} {second:?}");
        assert!(
            second.right() <= 360.0 && second.bottom() <= 900.0,
            "{second:?} header={} reserved={:?}",
            panel_header_height(&host.ctx),
            docks(&host.ctx).panels.iter().map(|p| p.rect).collect::<Vec<_>>()
        );
        let divider = egui::pos2(170.0, (first.bottom() + second.top()) * 0.5);
        host.drag(divider, divider + egui::vec2(0.0, 60.0));
        assert!(
            host.rect("First").height() > first.height() + 20.0,
            "first={:?} now={:?} layer={:?} response={:?} shares={:?}",
            first,
            host.rect("First"),
            host.ctx.layer_id_at(divider),
            host.ctx
                .read_response(egui::Id::new(("scicanvas-dialog", "First")).with("dock-divider")),
            docks(&host.ctx)
                .panels
                .iter()
                .map(|p| (p.side, p.share))
                .collect::<Vec<_>>()
        );
        host.title_drag("Second", egui::pos2(160.0, 210.0));
        assert!(host.rect("Second").top() < host.rect("First").top());
        host.ctx.style_mut(|style| style.animation_time = 0.2);
        // Use the native collapse arrow; the other expanded panel takes the freed space.
        let second = host.rect("Second");
        let pos = second.min + egui::vec2(12.0, 12.0);
        host.frame(vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            host.frame(vec![egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        host.settle();
        assert!(host.rect("Second").height() < 50.0);
        assert!(host.rect("First").height() > 500.0);
        host.title_drag("First", egui::pos2(780.0, 340.0));
        let state = docks(&host.ctx);
        assert_eq!(
            state
                .panels
                .iter()
                .find(|p| p.id == egui::Id::new(("scicanvas-dialog", "First")))
                .unwrap()
                .side,
            None
        );
        host.title_drag("First", egui::pos2(1180.0, 400.0));
        assert!(has_docks(&host.ctx, DockSide::Right));
        let right = host.rect("First");
        assert!(right.left() >= 840.0 && right.right() <= 1200.0, "{right:?}");
        host.size = egui::vec2(1000.0, 650.0);
        host.settle();
        let right = host.rect("First");
        assert!(right.right() <= 1000.0 && right.bottom() <= 650.0, "{right:?}");
        // Panel context menu is an alternative to dragging.
        let pos = egui::pos2(right.center().x, right.top() + 12.0);
        host.frame(vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            host.frame(vec![egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Secondary,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        let output = host.frame(vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "浮动" => Some(t.pos + egui::vec2(4.0, 4.0)),
                _ => None,
            })
            .expect("panel context menu");
        host.frame(vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            host.frame(vec![egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        host.settle();
        assert!(!has_docks(&host.ctx, DockSide::Right));
        host.second = false;
        host.settle();
        assert!(!has_docks(&host.ctx, DockSide::Left));
        reset_docks(&host.ctx);
        host.settle();
        assert!(!has_docks(&host.ctx, DockSide::Right));
    }
}
