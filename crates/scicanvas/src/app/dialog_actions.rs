//! 对话框交互两步：先生成 Action，再由统一分派应用；窗口收尾保持独立。
use super::*;

pub(super) struct DialogFrame {
    pub(super) action: dialogs::Action,
    pub(super) closing: bool,
    pub(super) layer_before: Option<scicanvas_core::page::Layer>,
}

impl SciApp {
    fn sync_dialog_controls(&mut self) {
        let Some(d) = self.dialog.as_mut() else { return };
        // 未改动的实时控件应同步功能区或其他已打开面板中的修改。
        match d {
            Dialog::CustomNlFit(edit) => edit.sync_saved(&self.project.fit_functions),
            Dialog::NlFitReport(_) => d.sync_fit_reports(&self.project),
            Dialog::LayerContents(edit) if edit.draft == edit.before => {
                if let Some(layer) = self
                    .project
                    .graph(edit.graph)
                    .and_then(|g| g.page.layers().get(edit.index))
                {
                    if layer != &edit.before {
                        edit.before = layer.clone();
                        edit.draft = layer.clone();
                        edit.error = None;
                    }
                }
            }
            Dialog::ColumnProps { book, before, edit, .. } if edit == before => {
                if let Some(column) = self.project.book(*book).and_then(|b| b.column(&edit.key)) {
                    let current = dialogs::ColumnEdit::from(column);
                    if &current != before {
                        *before = current.clone();
                        *edit = current;
                        self.book_preview = None;
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn generate_dialog_action(&mut self, ctx: &egui::Context) -> Option<DialogFrame> {
        self.sync_dialog_controls();
        let d = self.dialog.as_mut()?;
        let layer_before = match d {
            Dialog::LayerContents(edit) => Some(edit.draft.clone()),
            _ => None,
        };
        let mut action = dialogs::show(ctx, d, &self.pal, &self.project);
        let closing = matches!(action, dialogs::Action::Close);
        if self.refresh_book_preview() {
            ctx.request_repaint();
        }
        if matches!(action, dialogs::Action::None | dialogs::Action::Close) {
            let live = match &self.dialog {
                Some(Dialog::AnalysisCurves { graph, layer, curves }) => {
                    let chosen: Vec<_> = curves
                        .iter()
                        .filter(|(_, _, on)| *on)
                        .map(|(r, _, _)| r.clone())
                        .collect();
                    (self.analysis_curves.get(&(*graph, *layer)) != Some(&chosen))
                        .then_some(dialogs::Action::AnalysisCurves(*graph, *layer, chosen))
                }
                Some(Dialog::LayerContents(edit)) if edit.draft != edit.before => {
                    Some(dialogs::Action::LayerContents(edit.clone()))
                }
                Some(Dialog::ColumnProps { book, edit, .. })
                    if self
                        .book_preview
                        .as_ref()
                        .filter(|p| !p.applied)
                        .and_then(|p| p.result.as_ref().ok())
                        .is_some_and(|preview| self.project.book(*book) != Some(preview)) =>
                {
                    Some(dialogs::Action::ColumnProps(*book, edit.clone()))
                }
                Some(Dialog::CategoryOrder { book, key, levels, .. })
                    if self
                        .book_preview
                        .as_ref()
                        .filter(|p| !p.applied)
                        .and_then(|p| p.result.as_ref().ok())
                        .is_some_and(|preview| self.project.book(*book) != Some(preview)) =>
                {
                    Some(dialogs::Action::CategoryOrder(*book, key.clone(), levels.clone()))
                }
                _ => None,
            };
            if let Some(live) = live {
                action = live;
            }
        }
        if !matches!(action, dialogs::Action::None) {
            ctx.request_repaint();
        }
        Some(DialogFrame {
            action,
            closing,
            layer_before,
        })
    }

    /// false 表示输入校验提前返回，保留原对话框并跳过本帧收尾。
    pub(super) fn apply_dialog_action(&mut self, ctx: &egui::Context, action: dialogs::Action) -> bool {
        match action {
            dialogs::Action::LayerContents(edit) => self.apply_layer_contents_action(*edit),
            dialogs::Action::AnalysisCurves(graph, layer, curves) => {
                self.analysis_curves.insert((graph, layer), curves);
            }
            dialogs::Action::SaveNote(title, text) => self.apply_save_note_action(title, text),
            dialogs::Action::FitBands(reports) => self.apply_fit_bands_action(reports),
            dialogs::Action::NlFitBands(reports) => self.apply_nlfit_bands_action(reports),
            dialogs::Action::EditNlFit(report) => self.edit_nlfit_report(*report),
            dialogs::Action::Refit(target, recipe) => return self.refit_report(*target, recipe),
            dialogs::Action::ResidualPlot(reports) => self.apply_residual_plot_action(reports),
            dialogs::Action::None => {}
            dialogs::Action::Close => self.dialog = None,
            dialogs::Action::ColumnProps(book, edit) => self.apply_column_props_action(ctx, book, edit),
            dialogs::Action::ConvertText(book, key, row, value) => {
                self.apply_convert_text_action(book, key, row, value)
            }
            dialogs::Action::Fit(order) => {
                self.finish_dialog_input();
                self.fit_order(order, "多项式拟合");
            }
            dialogs::Action::Ifft(book, re, im, input) => self.apply_ifft_action(book, re, im, input),
            dialogs::Action::Sort(book, keys, within) => self.apply_sort_action(book, keys, within),
            dialogs::Action::CategoryOrder(book, key, levels) => self.apply_category_order_action(book, key, levels),
            dialogs::Action::SgDiff(w, order, deriv) => self.apply_sg_diff_action(w, order, deriv),
            dialogs::Action::Smooth(m, w) => {
                self.finish_dialog_input();
                self.smooth(m, w);
            }
            dialogs::Action::Interpolate(method, points, range) => {
                self.finish_dialog_input();
                self.interpolate(method, points, range);
            }
            dialogs::Action::TTest(kind, level) => self.run_ttest(kind, level),
            dialogs::Action::NlFit(model, limits, text) => return self.apply_nlfit_action(model, limits, text),
            dialogs::Action::OpenCustomNlFit => self.open_custom_nlfit_dialog(),
            dialogs::Action::CustomFitLibrary(action) => return self.apply_custom_fit_library_action(*action),
            dialogs::Action::CustomNlFit {
                name,
                formula,
                parameters,
                initial,
                constraints,
            } => {
                return self.apply_custom_nlfit_action(name, formula, parameters, initial, constraints);
            }
            dialogs::Action::SetValues(formula, keep) => self.apply_set_values_action(formula, keep),
            dialogs::Action::Fill(kind, rows) => {
                self.finish_dialog_input();
                self.fill_sequence(kind, rows);
            }
            dialogs::Action::FftFilter(kind) => {
                self.finish_dialog_input();
                self.fft_filter(kind);
            }
            dialogs::Action::Baseline(method) => {
                self.finish_dialog_input();
                self.subtract_baseline(method);
            }
            dialogs::Action::Peaks(min_prominence) => {
                self.finish_dialog_input();
                self.find_peaks(min_prominence);
            }
            dialogs::Action::Fft(window) => {
                self.finish_dialog_input();
                self.fft(window);
            }
            dialogs::Action::ColumnMath(op, constant) => {
                self.finish_dialog_input();
                self.column_math(op, constant);
            }
            dialogs::Action::Frequency(bins, range) => {
                self.finish_dialog_input();
                self.frequency_counts(bins, range);
            }
            dialogs::Action::ExportPng(dpi) => {
                self.dialog = None;
                self.export_png(dpi);
            }
            dialogs::Action::Normalize(m) => {
                self.finish_dialog_input();
                self.normalize(m);
            }
        }
        true
    }

    pub(super) fn finish_dialog_frame(
        &mut self,
        ctx: &egui::Context,
        closing: bool,
        layer_before: Option<scicanvas_core::page::Layer>,
    ) {
        if let Some(Dialog::LayerContents(edit)) = &mut self.dialog {
            if layer_before.as_ref() != Some(&edit.draft) {
                ctx.request_repaint();
            }
            if self
                .project
                .graph(edit.graph)
                .and_then(|g| g.page.layers().get(edit.index))
                != Some(&edit.before)
            {
                edit.error = Some("图层已更新，请重新打开图层内容".into());
            }
        }
        if closing {
            self.finish_edit();
            self.dialog = None;
            self.book_preview = None;
            self.history_group = None;
        } else if self.refresh_book_preview() {
            ctx.request_repaint();
        }
    }

    fn finish_dialog_input(&mut self) {
        self.finish_edit();
        self.dialog = None;
    }

    fn apply_layer_contents_action(&mut self, edit: dialogs::LayerContents) {
        match self.apply_layer_contents(&edit) {
            Ok(()) => {
                if let Some(Dialog::LayerContents(contents)) = &mut self.dialog {
                    contents.before = edit.draft.clone();
                    contents.error = None;
                }
            }
            Err(error) => {
                if let Some(Dialog::LayerContents(contents)) = &mut self.dialog {
                    contents.error = Some(error);
                }
            }
        }
    }

    fn apply_save_note_action(&mut self, title: String, text: String) {
        let parent = self.current_folder();
        self.edit_with(
            "保存结果为 Note",
            |project| {
                let id = project.add_note(&title, text);
                project.move_project_item(id, parent)?;
                Ok(Some(id))
            },
            |app, id| {
                app.show_item(id);
                app.status = "Note 已添加到项目，保存项目即可保留内容".into();
            },
        );
        self.dialog = None;
    }

    fn apply_fit_bands_action(&mut self, reports: Vec<scicanvas_core::ops::FitReport>) {
        self.edit_with(
            "置信/预测带",
            |project| {
                reports
                    .iter()
                    .map(|r| project.poly_bands(r, 0.95))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Some)
            },
            |app, _| {
                app.status = "已写入 95% 置信带和预测带，并加到图中".into();
            },
        );
    }

    fn apply_nlfit_bands_action(&mut self, reports: Vec<scicanvas_core::ops::NlFitReport>) {
        self.edit_with(
            "置信/预测带",
            |project| {
                reports
                    .iter()
                    .map(|r| project.nlfit_bands(r, 0.95))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Some)
            },
            |app, _| {
                app.status = "已写入 95% 置信带和预测带，并加到图中".into();
            },
        );
    }

    fn apply_residual_plot_action(&mut self, reports: Vec<scicanvas_core::ops::FitReport>) {
        self.edit_with(
            "残差图",
            |project| project.residual_plot(&reports).map(Some),
            |app, id| {
                app.show_item(id);
                app.status = "已生成残差图（残差 = Y − 拟合值）".into();
            },
        );
    }

    fn apply_column_props_action(&mut self, ctx: &egui::Context, book: u64, edit: dialogs::ColumnEdit) {
        let mut preview = self.book_preview.take();
        let focus = ctx
            .memory(|m| m.focused())
            .filter(|id| egui::TextEdit::load_state(ctx, *id).is_some());
        if focus.is_some() || ctx.input(|i| i.pointer.primary_down()) {
            self.history_group = Some(focus.unwrap_or_else(|| egui::Id::new("live-column-properties")));
        }
        match self.apply_column_props(book, edit) {
            Ok(()) => {
                if let Some(Dialog::ColumnProps { before, edit, .. }) = &mut self.dialog {
                    *before = edit.clone();
                }
                if let Some(preview) = &mut preview {
                    preview.applied = true;
                }
            }
            Err(e) => {
                if let Some(Dialog::ColumnProps { error, .. }) = &mut self.dialog {
                    *error = Some(e);
                }
            }
        }
        self.book_preview = preview;
    }

    fn apply_convert_text_action(&mut self, book: u64, key: String, row: usize, value: String) {
        self.edit_with(
            "转换为文本列并写入",
            |project| {
                let Some(column) = project
                    .book_mut(book)
                    .and_then(|b| b.columns.iter_mut().find(|c| c.key == key))
                else {
                    return Ok(None);
                };
                column.data = column.data.converted(scicanvas_core::DataKind::Text).unwrap().into();
                column.set(row, value);
                Ok(Some(()))
            },
            |app, ()| app.status = format!("列 {key} 已转换为文本；已有内容保留"),
        );
        self.dialog = None;
    }

    fn apply_ifft_action(
        &mut self,
        book: u64,
        re: String,
        im: Option<String>,
        input: scicanvas_core::fft::InverseInput,
    ) {
        self.finish_edit();
        self.dialog = None;
        self.edit_with(
            "IFFT",
            |project| project.ifft(book, &re, im.as_deref(), input).map(Some),
            |app, (b, g)| {
                app.show_item(g);
                let name = app.project.name_of(b).unwrap_or("").to_string();
                app.status = format!("IFFT 结果在 {name}，实部已画成图");
            },
        );
    }

    fn apply_sort_action(&mut self, book: u64, keys: Vec<(String, bool)>, within: Option<Vec<String>>) {
        self.edit_with(
            "排序",
            |project| project.sort_rows(book, &keys, within.as_deref()).map(Some),
            |app, ()| {
                let names: Vec<String> = keys
                    .iter()
                    .map(|(k, asc)| format!("{k}{}", if *asc { "升序" } else { "降序" }))
                    .collect();
                app.status = match &within {
                    Some(cols) => format!("已按 {} 排序 {} 列，其余列不动", names.join("、"), cols.join("、")),
                    None => format!(
                        "已按 {} 排序整张工作表，保留 {} 个独立采样结果列的原顺序",
                        names.join("、"),
                        app.project
                            .book(book)
                            .map_or(0, |b| b.columns.iter().filter(|c| c.independent_rows).count())
                    ),
                };
                app.dialog = None;
            },
        );
    }

    fn apply_category_order_action(&mut self, book: u64, key: String, levels: Vec<String>) {
        self.edit_with(
            "类别顺序",
            |project| {
                project
                    .book_mut(book)
                    .and_then(|b| b.columns.iter_mut().find(|c| c.key == key))
                    .ok_or_else(|| "找不到这一列".to_string())
                    .and_then(|c| c.data.set_category_order(&levels))
                    .map(Some)
            },
            |app, ()| app.status = format!("已设置 {key} 列的类别顺序"),
        );
    }

    fn apply_sg_diff_action(&mut self, w: usize, order: usize, deriv: usize) {
        self.finish_edit();
        self.dialog = None;
        if let Some(src) = self.analysis_source() {
            self.edit_with(
                "SG 平滑求导",
                |project| project.differentiate_sg(&src, w, order, deriv).map(Some),
                |app, keys| {
                    app.status = format!("已求导数，结果在新列 {}", keys.join(", "));
                },
            );
        }
    }

    fn apply_nlfit_action(
        &mut self,
        model: scicanvas_core::nlfit::Model,
        limits: Vec<scicanvas_core::nlfit::ParamLimits>,
        text: String,
    ) -> bool {
        let constraints = match scicanvas_core::nlfit::parse_constraints(model, &text) {
            Ok(c) => c,
            Err(e) => {
                // 约束写错时保留对话框，方便修改
                self.status = e;
                return false;
            }
        };
        self.finish_edit();
        self.dialog = None;
        self.nlfit(model, &limits, &constraints);
        true
    }

    fn apply_custom_nlfit_action(
        &mut self,
        name: String,
        formula: String,
        parameters: String,
        initial: String,
        constraint_text: String,
    ) -> bool {
        let result = scicanvas_core::nlfit::CustomFitDefinition::from_text(
            &name,
            &formula,
            &parameters,
            &initial,
            &constraint_text,
        )
        .and_then(|definition| {
            let (function, constraints) = definition.resolve()?;
            Ok((function, definition.initial, constraints))
        });
        let (function, values, constraints) = match result {
            Ok(result) => result,
            Err(error) => {
                self.status = error;
                return false;
            }
        };
        let limits = vec![scicanvas_core::nlfit::ParamLimits::default(); function.parameters().len()];
        self.custom_nlfit(function, values, limits, constraints)
    }

    fn apply_set_values_action(&mut self, formula: String, keep: bool) {
        self.finish_edit();
        if self.set_column_values(&formula, keep) {
            self.dialog = None;
        }
    }
}
