use super::*;

impl SciApp {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        let frame = if let Some(frame) = self.busy_action.take() {
            if !self.busy_presented {
                self.busy_presented = true;
                self.busy_action = Some(frame);
                ctx.set_cursor_icon(egui::CursorIcon::Progress);
                ctx.request_repaint();
                if let Some(dialog) = &mut self.dialog {
                    let _ = dialogs::show(ctx, dialog, &self.pal, &self.project);
                }
                return;
            }
            self.busy_presented = false;
            frame
        } else {
            let Some(frame) = self.generate_dialog_action(ctx) else {
                return;
            };
            let message = match &frame.action {
                dialogs::Action::GlobalFit(_) | dialogs::Action::PeakFit(_) => Some("正在拟合…"),
                dialogs::Action::BatchExport(..) => Some("正在导出…"),
                _ => None,
            };
            if let Some(message) = message {
                self.status_info(message.into());
                self.busy_action = Some(frame);
                ctx.set_cursor_icon(egui::CursorIcon::Progress);
                ctx.request_repaint();
                return;
            }
            frame
        };
        if !self.apply_dialog_action(ctx, frame.action) {
            return;
        }
        self.finish_dialog_frame(ctx, frame.closing, frame.layer_before);
    }

    // ---------------------------------------------------------------- 输入

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::Key;
        // 正在输入文字时，Ctrl+Z 留给 TextEdit 自己的文字历史。
        let text_focus = ctx
            .memory(|m| m.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some());
        let shifted = ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.modifiers.shift);
        let mut clipboard = None;
        let mut paste = None;
        if self.ui.palette.is_none() {
            ctx.input_mut(|i| {
                i.events.retain(|event| match event {
                    egui::Event::Copy if shifted => {
                        clipboard = Some("graph.style.copy");
                        false
                    }
                    egui::Event::Paste(_) if shifted => {
                        clipboard = Some("graph.style.paste");
                        false
                    }
                    egui::Event::Copy if !text_focus => {
                        clipboard = Some("edit.copy");
                        false
                    }
                    egui::Event::Paste(text) if !text_focus => {
                        paste = Some(text.clone());
                        false
                    }
                    _ => true,
                })
            });
        }
        if let Some(command) = clipboard {
            if !shifted || self.active_graph_id().is_some() {
                self.shortcut_command(ctx, command);
            }
        }
        if let Some(text) = paste {
            #[cfg(test)]
            if ctx.data(|d| d.get_temp::<bool>(egui::Id::new("shortcut-probe"))) == Some(true) {
                self.shortcut_command(ctx, "edit.paste");
            } else {
                self.paste_text(text);
            }
            #[cfg(not(test))]
            self.paste_text(text);
        }
        for command in commands::commands().iter().filter(|c| !c.keys.is_empty()) {
            if self.ui.palette.is_some() && command.id != "win.reset" {
                continue;
            }
            if text_focus
                && matches!(
                    command.id,
                    "edit.undo" | "edit.redo" | "edit.copy" | "edit.paste" | "edit.clear"
                )
            {
                continue;
            }
            let hit = commands::shortcut_bindings(command)
                .into_iter()
                .any(|(m,k)|ctx.input_mut(|i| {
                    let exact=i.events.iter().any(|e|matches!(e,egui::Event::Key{key,pressed:true,modifiers,..} if *key==k && modifiers.matches_exact(m)));
                    exact && i.consume_key(m,k)
                }));
            if hit {
                self.shortcut_command(ctx, command.id);
            }
        }
        if self.ui.backstage && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.ui.backstage = false;
        }
    }

    fn shortcut_command(&mut self, ctx: &egui::Context, id: &'static str) {
        #[cfg(test)]
        if ctx.data(|d| d.get_temp::<bool>(egui::Id::new("shortcut-probe"))) == Some(true) {
            ctx.data_mut(|d| {
                let mut ids = d
                    .get_temp::<Vec<&'static str>>(egui::Id::new("shortcut-dispatched"))
                    .unwrap_or_default();
                ids.push(id);
                d.insert_temp(egui::Id::new("shortcut-dispatched"), ids);
            });
            return;
        }
        let _ = ctx;
        self.run_command(id);
    }

    // ---------------------------------------------------------------- 界面

    pub(super) fn stats_text(&self) -> String {
        let Some(id) = self.active else { return String::new() };
        let Some(b) = self.project.book(id) else {
            return String::new();
        };
        let sel = self.selected_keys(id);
        let v: Vec<f64> = match self.multi_range(id) {
            Some(r) if sel.is_empty() && !b.columns.is_empty() => b.columns[r.c0()..=r.c1().min(b.columns.len() - 1)]
                .iter()
                .flat_map(|c| (r.r0()..=r.r1()).map(move |i| c.data.f64_at(i).unwrap_or(f64::NAN)))
                .filter(|x| x.is_finite())
                .collect(),
            _ => b
                .columns
                .iter()
                .filter(|c| sel.contains(&c.key))
                .flat_map(|c| c.values())
                .filter(|x| x.is_finite())
                .collect(),
        };
        if v.is_empty() {
            return String::new();
        }
        let sum: f64 = v.iter().sum();
        let min = v.iter().copied().fold(f64::INFINITY, f64::min);
        let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        format!(
            "平均值 = {}   总和 = {}   计数 = {}   最小 = {}   最大 = {}",
            scicanvas_core::fmt_num(sum / v.len() as f64),
            scicanvas_core::fmt_num(sum),
            v.len(),
            scicanvas_core::fmt_num(min),
            scicanvas_core::fmt_num(max)
        )
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        let stats = self.stats_text();
        ui.horizontal(|ui| {
            self.status_feedback(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(stats).monospace().weak());
            });
        });
    }

    pub(super) fn refresh_book_preview(&mut self) -> bool {
        let edit = match &self.dialog {
            Some(Dialog::ColumnProps { book, edit, .. }) => BookPreviewEdit::Properties(*book, edit.clone()),
            Some(Dialog::CategoryOrder { book, key, levels, .. }) => {
                BookPreviewEdit::Categories(*book, key.clone(), levels.clone())
            }
            Some(Dialog::Sort {
                book,
                columns,
                keys,
                selected,
                only_selected,
                ..
            }) => BookPreviewEdit::Sort(
                *book,
                keys.iter()
                    .filter_map(|(i, asc)| i.map(|i| (columns[i].0.clone(), *asc)))
                    .collect(),
                (*only_selected && !selected.is_empty()).then(|| selected.clone()),
            ),
            _ => return self.book_preview.take().is_some(),
        };
        if self.book_preview.as_ref().is_some_and(|p| p.edit == edit) {
            return false;
        }
        let id = match &edit {
            BookPreviewEdit::Properties(id, _)
            | BookPreviewEdit::Categories(id, ..)
            | BookPreviewEdit::Sort(id, ..) => *id,
        };
        let reusable = matches!((&self.book_preview, &edit),
            (Some(BookPreview { edit: BookPreviewEdit::Properties(old_id, old), result: Ok(_), .. }),
             BookPreviewEdit::Properties(id, new))
            if old_id == id && old.kind == new.kind && old.role == new.role
                && old.format.utc_offset == new.format.utc_offset);
        // ponytail: type/role/category/sort changes copy one table; shared column storage can reduce this for very large tables.
        // Names, units and display formats reuse the cached data, including during continuous typing.
        let base = if reusable {
            self.book_preview.take().unwrap().result
        } else {
            self.project.book(id).cloned().ok_or_else(|| "找不到工作簿".to_string())
        };
        let result = base.and_then(|mut book| {
            match &edit {
                BookPreviewEdit::Properties(_, edit) => edit.apply(&mut book)?,
                BookPreviewEdit::Categories(_, key, levels) => book
                    .columns
                    .iter_mut()
                    .find(|c| &c.key == key)
                    .ok_or("找不到列")?
                    .data
                    .set_category_order(levels)?,
                BookPreviewEdit::Sort(_, keys, within) => {
                    let mut project = Project::default();
                    project.books.push(book);
                    project.sort_rows(id, keys, within.as_deref())?;
                    book = project.books.remove(0);
                }
            }
            Ok(book)
        });
        if let Some(Dialog::ColumnProps {
            error, sample, edit, ..
        }) = &mut self.dialog
        {
            *error = result.as_ref().err().cloned();
            if let Ok(book) = &result {
                *sample = book
                    .column(&edit.key)
                    .and_then(|c| (0..c.data.len()).find_map(|r| c.data.f64_at(r).filter(|v| v.is_finite())));
            }
        }
        if let Some(Dialog::CategoryOrder { error, .. } | Dialog::Sort { error, .. }) = &mut self.dialog {
            *error = result.as_ref().err().cloned();
        }
        let applied = result.as_ref().is_ok_and(|book| self.project.book(id) == Some(book));
        self.book_preview = Some(BookPreview { edit, result, applied });
        true
    }

    pub(super) fn preview_graph(&self, id: u64) -> Option<std::borrow::Cow<'_, scicanvas_core::Graph>> {
        use std::borrow::Cow;
        let graph = self.project.graph(id)?;
        if let Some(Dialog::LayerContents(edit)) = &self.dialog {
            if edit.graph == id && graph.page.layers().get(edit.index) == Some(&edit.before) {
                let valid = edit.draft.plot_keys(true).is_empty()
                    || edit
                        .draft
                        .book_id
                        .and_then(|id| self.project.book(id))
                        .is_some_and(|book| edit.draft.validate_in(book, &|id| self.project.book(id)).is_ok());
                if valid && edit.draft != edit.before {
                    let mut graph = graph.clone();
                    graph.page.layers_mut()[edit.index] = edit.draft.clone();
                    return Some(Cow::Owned(graph));
                }
            }
        }
        Some(Cow::Borrowed(graph))
    }

    pub(crate) fn start_text_annotation(&mut self) {
        if let Some(id) = self.target_graph() {
            self.finish_edit();
            self.show_item(id);
            self.explorer_focus = false;
            self.adding_text = Some(id);
            self.shape_tool = None;
            self.object_editor = None;
            self.status_info("单击页面放置文字，Esc 取消".into());
        }
    }

    pub(crate) fn manage_annotations(&mut self) {
        if let Some(graph) = self.target_graph() {
            if let Some(text) = self.project.graph(graph).and_then(|g| g.page.annotations.last()) {
                self.object_editor = Some(ObjectEditor {
                    graph,
                    target: plot::ObjectTarget::Text(text.id),
                    focus: true,
                });
            } else {
                self.status_warn("当前图形没有文字标注".into());
            }
        }
    }

    pub(crate) fn auto_legend(&mut self) {
        if let Some(graph) = self.target_graph() {
            self.apply_object_action(graph, plot::ObjectAction::AutoLegend(self.layer_index(graph)));
        }
    }

    pub(super) fn apply_object_action(&mut self, graph: u64, action: plot::ObjectAction) {
        use plot::{ObjectAction as A, ObjectTarget as T};
        if self.project.graph(graph).is_none() {
            return;
        }
        match action {
            A::CancelAdd => {
                self.adding_text = None;
                self.shape_tool = None;
                return;
            }
            A::Edit(target) => {
                self.finish_edit();
                self.object_editor = Some(ObjectEditor {
                    graph,
                    target,
                    focus: true,
                });
                return;
            }
            _ => {}
        }
        let label = match action {
            A::Move(..) => "移动图形对象",
            A::AddText(..) => "添加文字",
            A::Delete(..) => "删除图形对象",
            _ => "图例自动避让",
        };
        if let A::AddShape(kind, a, b) = action {
            self.edit_with(
                "添加绘图对象",
                |project| project.graph_mut(graph).unwrap().page.add_shape(kind, a, b).map(Some),
                |app, id| {
                    app.shape_tool = None;
                    app.object_editor = Some(ObjectEditor {
                        graph,
                        target: T::Shape(id),
                        focus: false,
                    });
                },
            );
            return;
        }
        if let A::ResizeShape(id, a, b) = action {
            if self
                .project
                .graph(graph)
                .unwrap()
                .page
                .shape(id)
                .is_some_and(|s| s.a != a || s.b != b)
            {
                self.edit_grouped(
                    "调整图形端点",
                    Some(egui::Id::new(("object-drag", graph))),
                    |project| {
                        let shape = project.graph_mut(graph).unwrap().page.shape_mut(id).unwrap();
                        shape.a = a;
                        shape.b = b;
                    },
                );
            }
            return;
        }
        if let A::Move(target, position) = action {
            let page = &self.project.graph(graph).unwrap().page;
            let changed = match target {
                T::Text(id) => page.text(id).is_some_and(|t| t.position != position),
                T::Shape(id) => page.shape(id).is_some_and(|s| s.origin() != position),
                T::ColorScale(index) => page
                    .layers()
                    .get(index)
                    .is_some_and(|l| l.color_scale.as_ref().and_then(|c| c.position) != Some(position)),
                T::Legend(index) => page
                    .layers()
                    .get(index)
                    .is_some_and(|l| l.legend_position != Some(position)),
            };
            if changed {
                self.edit_grouped(label, Some(egui::Id::new(("object-drag", graph))), |project| {
                    let page = &mut project.graph_mut(graph).unwrap().page;
                    match target {
                        T::Text(id) => page.text_mut(id).unwrap().position = position,
                        T::Shape(id) => page.shape_mut(id).unwrap().move_to(position),
                        T::Legend(index) => page.layers_mut()[index].legend_position = Some(position),
                        T::ColorScale(index) => {
                            page.layers_mut()[index]
                                .color_scale
                                .get_or_insert_with(Default::default)
                                .position = Some(position)
                        }
                    }
                });
            }
            return;
        }
        self.edit_with(label,|project| {
            let page=&mut project.graph_mut(graph).unwrap().page;
            let text=match action {
                A::AddText(position)=>Some(page.add_text(position)?),
                A::Delete(T::Text(id))=>{page.annotations.retain(|t|t.id!=id);None},
                A::Delete(T::Shape(id))=>{page.remove_shape(id);None},
                A::Delete(T::Legend(index))=>{if let Some(l)=page.layers_mut().get_mut(index){l.legend=false;}None},
                A::Delete(T::ColorScale(index))=>{if let Some(l)=page.layers_mut().get_mut(index){l.color_scale.get_or_insert_with(Default::default).visible=false;}None},
                A::AutoLegend(index)=>{if let Some(l)=page.layers_mut().get_mut(index){l.legend=true;l.legend_position=None;}None},
                _=>None,
            };
            Ok(Some(text))
        },|app,text| {
            if let Some(id)=text {
                app.adding_text=None;app.object_editor=Some(ObjectEditor {graph,target:T::Text(id),focus:true});
            }
            if matches!(action,A::Delete(target) if app.object_editor.is_some_and(|e|e.graph==graph && e.target==target)) {app.object_editor=None;}
        });
    }

    pub(super) fn graph_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        // The plot call is read-only. Temporarily supply the preview table to the shared scene builder,
        // then restore it before processing any commands, exports or history operations.
        let mut preview = self.book_preview.take();
        let preview_book = preview.as_mut().and_then(|p| p.result.as_mut().ok());
        let at = preview_book
            .as_ref()
            .and_then(|b| self.project.books.iter().position(|source| source.id == b.id));
        if let (Some(at), Some(book)) = (at, preview_book) {
            std::mem::swap(&mut self.project.books[at], book);
        }
        let out = self.preview_graph(id).map(|g| {
            plot::show(
                ui,
                &g,
                &self.project,
                self.layer_index(id),
                plot::ObjectUi {
                    adding: self.adding_text == Some(id),
                    adding_shape: self.shape_tool.filter(|(graph, _)| *graph == id).map(|(_, kind)| kind),
                    selected_shape: self.object_editor.filter(|e| e.graph == id).and_then(|e| {
                        if let plot::ObjectTarget::Shape(id) = e.target {
                            Some(id)
                        } else {
                            None
                        }
                    }),
                    epoch: self.plot_epoch,
                    keyboard: self.active == Some(id) && !self.explorer_focus,
                },
            )
        });
        if let Some(at) = at {
            std::mem::swap(
                &mut self.project.books[at],
                preview.as_mut().unwrap().result.as_mut().unwrap(),
            );
        }
        self.book_preview = preview;
        let Some(out) = out else { return };
        if let Some(action) = out.action {
            self.apply_object_action(id, action);
        }
        if out.object_dragging {
            self.history_group = Some(egui::Id::new(("object-drag", id)));
        }
        if out.response.contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
            self.explorer_focus = false;
        }
        if let Some(index) = out.selected_layer {
            self.select_layer(id, index);
        }
        let rect = out.response.rect;

        // 右下角显示鼠标处的坐标
        if let Some((x, y)) = out.hover {
            // 吸附到数据点时显示该点的值，否则显示鼠标处坐标
            let text = match &out.point {
                Some(hit) => format!(
                    "图层 {} · {} · x = {}   y = {}",
                    out.hover_layer.unwrap_or(0) + 1,
                    hit.series,
                    scicanvas_core::fmt_num(hit.x),
                    scicanvas_core::fmt_num(hit.y)
                ),
                None => format!(
                    "图层 {} · x = {}   y = {}",
                    out.hover_layer.unwrap_or(0) + 1,
                    round_sig(x, 4),
                    round_sig(y, 4)
                ),
            };
            ui.painter().text(
                rect.right_bottom() + egui::vec2(-8.0, -6.0),
                egui::Align2::RIGHT_BOTTOM,
                text,
                egui::FontId::proportional(crate::theme::size::LABEL),
                plot::PAGE_HINT,
            );
        } else if out.zoomed {
            ui.painter().text(
                rect.right_bottom() + egui::vec2(-8.0, -6.0),
                egui::Align2::RIGHT_BOTTOM,
                "双击恢复原始范围",
                egui::FontId::proportional(crate::theme::size::LABEL),
                plot::PAGE_HINT_WEAK,
            );
        }

        let (act, remove, object_action) =
            self.graph_context_menu(ui, id, &out.response, out.object, out.point.as_ref(), out.hover_layer);
        if let Some(action) = object_action {
            self.apply_object_action(id, action);
        }
        if let Some((layer, key)) = remove {
            self.remove_curve(id, layer, &key);
        }
        if let Some(cid) = act {
            self.active = Some(id);
            self.run_command(cid);
        }
    }

    fn graph_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        response: &egui::Response,
        object: Option<plot::ObjectTarget>,
        point: Option<&scicanvas_core::scene::PointHit>,
        hover_layer: Option<usize>,
    ) -> (
        Option<&'static str>,
        Option<(usize, String)>,
        Option<plot::ObjectAction>,
    ) {
        let mut act: Option<&'static str> = None;

        // 右键时吸附到的曲线（菜单打开期间保持）
        let curve_id = egui::Id::new(("graph-curve-menu", id));
        let object_id = curve_id.with("object");
        if response.secondary_clicked() {
            self.active = Some(id);
            self.select_layer(id, hover_layer.unwrap_or(0));
            self.style_target = Some((id, hover_layer.unwrap_or(0), point.map(|p| p.key.clone())));
            ui.data_mut(|d| d.insert_temp(object_id, object));
            let hit = point.map(|p| (hover_layer.unwrap_or(0), p.key.clone(), p.series.clone()));
            ui.data_mut(|d| d.insert_temp(curve_id, hit));
        }
        let menu_curve = ui
            .data(|d| d.get_temp::<Option<(usize, String, String)>>(curve_id))
            .flatten();
        let mut remove: Option<(usize, String)> = None;
        let menu_object = ui
            .data(|d| d.get_temp::<Option<plot::ObjectTarget>>(object_id))
            .flatten();
        let mut object_action = None;
        response.context_menu(|ui| {
            if let Some(target) = menu_object {
                if ui
                    .button(if matches!(target, plot::ObjectTarget::Shape(_)) {
                        "编辑图形…"
                    } else {
                        "编辑文字/图例…"
                    })
                    .clicked()
                {
                    object_action = Some(plot::ObjectAction::Edit(target));
                    ui.close_menu();
                }
                if ui.button("删除对象").clicked() {
                    object_action = Some(plot::ObjectAction::Delete(target));
                    ui.close_menu();
                }
                if let plot::ObjectTarget::Legend(index) = target {
                    if ui.button("恢复图例自动位置").clicked() {
                        object_action = Some(plot::ObjectAction::AutoLegend(index));
                        ui.close_menu();
                    }
                }
                ui.separator();
            }
            if let Some((layer, key, name)) = &menu_curve {
                if ui.button(format!("移除曲线 {name}")).clicked() {
                    remove = Some((*layer, key.clone()));
                    ui.close_menu();
                }
                ui.separator();
            }
            let state = crate::menus::MenuState::new(self);
            for cid in ["graph.props", "layer.contents"] {
                if crate::menus::menu_command(ui, &state, cid) {
                    act = Some(cid);
                }
            }
            ui.separator();
            ui.menu_button("坐标轴", |ui| {
                crate::ribbon::scale_menu(ui, true, self, &mut act);
                crate::ribbon::scale_menu(ui, false, self, &mut act);
                if crate::menus::menu_command(ui, &state, "graph.grid") {
                    act = Some("graph.grid");
                }
            });
            for cid in ["graph.legend", "graph.labels"] {
                if crate::menus::menu_command(ui, &state, cid) {
                    act = Some(cid);
                }
            }
            ui.menu_button("添加", |ui| {
                for cid in [
                    "graph.text.add",
                    "graph.reference.vertical",
                    "graph.reference.horizontal",
                    "layer.add",
                ] {
                    if crate::menus::menu_command(ui, &state, cid) {
                        act = Some(cid);
                    }
                }
            });
            ui.menu_button("格式", |ui| {
                for cid in ["graph.style.copy", "graph.style.paste"] {
                    if crate::menus::menu_command(ui, &state, cid) {
                        act = Some(cid);
                    }
                }
                ui.menu_button("应用主题", |ui| {
                    let themes: Vec<_> = std::iter::once(scicanvas_core::graph_theme::GraphTheme::nature())
                        .chain(self.project.graph_themes.iter().cloned())
                        .collect();
                    for theme in themes {
                        if ui.button(&theme.name).clicked() {
                            self.apply_graph_theme(theme);
                            ui.close_menu();
                        }
                    }
                });
                if crate::menus::menu_command(ui, &state, "graph.themes") {
                    act = Some("graph.themes");
                }
            });
            ui.menu_button("导出", |ui| {
                for cid in [
                    "graph.copy",
                    "graph.export",
                    "graph.export_pdf",
                    "graph.export_png",
                    "graph.export_tiff",
                ] {
                    if crate::menus::menu_command(ui, &state, cid) {
                        act = Some(cid);
                    }
                }
            });
            ui.separator();
            if ui.button("恢复原始范围").clicked() {
                plot::reset_view(ui.ctx(), id, self.layer_index(id));
                ui.close_menu();
            }
        });
        (act, remove, object_action)
    }

    /// 把工作簿 `book_id` 里选中的列加到图形当前图层（一步撤销）。
    pub(crate) fn add_to_graph(&mut self, book_id: u64, gid: u64) {
        let keys = self.selected_keys(book_id);
        let layer = self.layer_index(gid);
        self.edit_with(
            "添加曲线",
            |project| project.add_curves(gid, layer, book_id, &keys).map(Some),
            |app, n| {
                {
                    let status_kind = if n > 0 { StatusKind::Success } else { StatusKind::Info };
                    let text: String = if n > 0 {
                        format!("已添加 {n} 条曲线")
                    } else {
                        "这些列已在图层里".into()
                    };
                    app.set_status(status_kind, text);
                };
                app.show_item(gid);
            },
        );
    }

    /// 从图层移除一条曲线（一步撤销）。
    pub(crate) fn remove_curve(&mut self, gid: u64, layer: usize, key: &str) {
        let present = self
            .project
            .graph(gid)
            .and_then(|g| g.page.layers().get(layer))
            .is_some_and(|l| l.plot_keys(true).iter().any(|k| k == key));
        if !present {
            self.finish_edit();
            return;
        }
        self.status_ok("已移除曲线".into());
        self.edit("移除曲线", |project| {
            project.graph_mut(gid).unwrap().page.layers_mut()[layer].remove_curve(key);
            Ok(Some(()))
        });
    }

    pub(super) fn object_window(&mut self, ctx: &egui::Context) {
        use plot::ObjectTarget as T;
        let Some(mut edit) = self.object_editor else { return };
        let Some(before) = self.project.graph(edit.graph).cloned() else {
            self.object_editor = None;
            return;
        };
        let valid = match edit.target {
            T::Text(id) => before.page.text(id).is_some(),
            T::Shape(id) => before.page.shape(id).is_some(),
            T::Legend(i) => before.page.layers().get(i).is_some_and(|l| l.legend),
            T::ColorScale(i) => before
                .page
                .layers()
                .get(i)
                .is_some_and(|l| !l.color_scale.as_ref().is_some_and(|c| !c.visible)),
        };
        if !valid {
            self.object_editor = None;
            return;
        }
        let (open, action) = edit.show(
            ctx,
            self.project.graph_mut(edit.graph).unwrap(),
            &mut self.history_group,
        );
        if self.project.graph(edit.graph) != Some(&before) {
            let after = std::mem::replace(self.project.graph_mut(edit.graph).unwrap(), before);
            self.edit_grouped("编辑图形对象", self.history_group, |project| {
                *project.graph_mut(edit.graph).unwrap() = after;
            });
            ctx.request_repaint();
        }
        self.object_editor = open.then_some(edit);
        if let Some(action) = action {
            self.finish_edit();
            self.apply_object_action(edit.graph, action);
        }
    }

    pub(super) fn about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("关于 SciCanvas")
            .id(egui::Id::new("scicanvas-about"))
            .constrain_to(dialogs::floating_bounds(ctx))
            .open(&mut self.show_about)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!("版本：{}", env!("CARGO_PKG_VERSION")));
                ui.label(format!("提交：{}", env!("SCICANVAS_COMMIT")));
                ui.label(format!("图形后端：{}", self.renderer_info));
            });
    }

    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("快速上手")
            .id(egui::Id::new("scicanvas-help"))
            .constrain_to(dialogs::floating_bounds(ctx))
            .open(&mut self.show_help)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                for text in [
                    "1. 在功能区或 Ctrl+K 搜索命令。",
                    "2. 在工作表中输入数据，或从 Excel 复制粘贴。",
                    "3. 选中数据列后，选择作图或分析命令。",
                    "4. 选中图形可调整坐标轴、曲线和标注。",
                    "5. 在文件菜单保存项目或导出图形。",
                ] {
                    ui.label(text);
                }
            });
    }

    pub(super) fn confirm_window(&mut self, ctx: &egui::Context) {
        if self.pending_discard.is_some()
            || self.save_error.is_some()
            || !self.recovery_candidates.is_empty()
            || self.cleanup_blocks_ui()
        {
            return;
        }
        let Some(id) = self.confirm_delete else { return };
        let name = self.project.name_of(id).unwrap_or("").to_string();
        let choice = dialogs::modal(ctx, "删除", |ui| {
            ui.label(RichText::new(format!("确定删除“{name}”吗？")).strong());
            ui.label(RichText::new("删除后可以用“撤销”（Ctrl+Z）恢复。").color(self.pal.weak));
            dialogs::actions(
                ui,
                &[
                    (dialogs::ActionKind::Danger, "删除"),
                    (dialogs::ActionKind::Plain, "取消"),
                ],
                dialogs::Keys::Modal,
                Ok(()),
            )
        })
        .flatten();
        let decision = choice.map(|index| index == 0);
        match decision {
            Some(true) => {
                self.finish_edit();
                self.confirm_delete = None;
                self.delete_item(id);
                self.finish_edit();
            }
            Some(false) => self.confirm_delete = None,
            None => {}
        }
    }
}

impl SciApp {
    pub(crate) fn start_shape_tool(&mut self, kind: scicanvas_core::page::ShapeKind) {
        let Some(graph) = self.active_graph_id() else { return };
        self.finish_edit();
        self.adding_text = None;
        self.shape_tool = Some((graph, kind));
        self.object_editor = None;
        self.status_info(format!("在页面按下拖动画{}；Esc 取消", kind.label()));
    }
}
