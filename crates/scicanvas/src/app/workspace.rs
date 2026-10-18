//! Project explorer, object operations and workspace child windows.

use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ExplorerSort {
    Name,
    #[default]
    Type,
}

impl SciApp {
    fn range_select_project_item(&mut self, id: u64, append: bool) {
        let anchor = self.explorer_anchor.or(self.explorer_target).unwrap_or(id);
        let a = self.explorer_visible.iter().position(|item| *item == anchor);
        let b = self.explorer_visible.iter().position(|item| *item == id);
        if let (Some(a), Some(b)) = (a, b) {
            if !append {
                self.explorer_selection.clear();
            }
            for item in &self.explorer_visible[a.min(b)..=a.max(b)] {
                if !self.explorer_selection.contains(item) {
                    self.explorer_selection.push(*item);
                }
            }
            self.explorer_target = Some(id);
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            self.explorer_click = None;
        } else {
            self.select_project_item(id, false);
        }
    }

    fn select_project_item(&mut self, id: u64, extend: bool) {
        self.explorer_anchor = Some(id);
        if extend {
            if self.explorer_selection.contains(&id) {
                self.explorer_selection.retain(|selected| *selected != id);
            } else {
                self.explorer_selection.push(id);
            }
            self.explorer_target = self.explorer_selection.last().copied();
        } else {
            self.explorer_selection = vec![id];
            self.explorer_target = Some(id);
        }
        self.project_name = self
            .explorer_target
            .and_then(|id| self.project.name_of(id))
            .unwrap_or("")
            .into();
    }

    /// 同时选中目录及其内容时，只移动最外层对象，保留内部层级。
    fn selected_project_roots(&self, source: u64) -> Vec<u64> {
        let selected = if self.explorer_selection.contains(&source) {
            self.explorer_selection.clone()
        } else {
            vec![source]
        };
        selected
            .iter()
            .copied()
            .filter(|id| {
                let mut parent = self
                    .project
                    .folder(*id)
                    .map_or_else(|| self.project.folder_of(*id), |f| f.parent);
                while let Some(ancestor) = parent {
                    if selected.contains(&ancestor) {
                        return false;
                    }
                    parent = self.project.folder(ancestor).and_then(|f| f.parent);
                }
                true
            })
            .collect()
    }

    pub(crate) fn open_plot_example(&mut self, kind: PlotType) {
        let example = crate::examples::ALL.iter().find(|e| e.kind == kind).unwrap();
        let mut sample = match scicanvas_core::project_file::from_scv(example.bytes) {
            Ok(project) => project,
            Err(error) => {
                self.status = format!("打开示例失败：{error}");
                return;
            }
        };
        let mut book = sample.books.remove(0);
        let mut graph = sample.graphs.remove(0);
        let parent = self.current_folder();
        book.id = self.project.alloc_id();
        book.name = self.project.unique_name(&format!("{}示例数据", kind.label()));
        graph.id = self.project.alloc_id();
        graph.name = self.project.unique_name(&format!("{}示例", kind.label()));
        for layer in graph.page.layers_mut() {
            layer.book_id = Some(book.id);
        }
        let (book_id, graph_id) = (book.id, graph.id);
        self.project.books.push(book);
        self.project.graphs.push(graph);
        for id in [book_id, graph_id] {
            self.project.move_project_item(id, parent).unwrap();
            self.show_item(id);
        }
        self.status = format!(
            "已打开{}示例；查看数据表的注释行了解各列用法，可撤销本次添加",
            kind.label()
        );
    }

    pub(super) fn show_item(&mut self, id: u64) {
        self.explorer_anchor = Some(id);
        self.explorer_selection = vec![id];
        self.explorer_target = Some(id);
        self.project_name = self.project.name_of(id).unwrap_or("").to_string();
        self.open.insert(id, true);
        if let Some(g) = self.geoms.get_mut(&id) {
            if g.state == WinState::Min {
                g.state = WinState::Normal;
            }
        }
        self.active = Some(id);
        self.raise = Some(id);
    }

    pub(crate) fn all_ids(&self) -> Vec<u64> {
        self.project
            .books
            .iter()
            .map(|b| b.id)
            .chain(self.project.graphs.iter().map(|g| g.id))
            .chain(self.project.notes.iter().map(|n| n.id))
            .collect()
    }

    // ---------------------------------------------------------------- 操作

    pub(crate) fn new_graph(&mut self) {
        let parent = self.current_folder();
        let id = self.project.alloc_id();
        self.project
            .graphs
            .push(scicanvas_core::Graph::new(id, self.project.unique_name("Graph")));
        self.project.move_project_item(id, parent).unwrap();
        self.show_item(id);
        self.status = "已新建空白图形，可在绘图属性中选择数据".into();
    }

    pub(crate) fn new_book(&mut self) {
        let parent = self.current_folder();
        let id = self.project.new_book();
        self.project.move_project_item(id, parent).unwrap();
        self.show_item(id);
        self.status = format!(
            "已新建 {}，可直接输入数据，或从 Excel 复制一块区域后在单元格里粘贴",
            self.project.name_of(id).unwrap_or("")
        );
    }

    pub(super) fn delete_item(&mut self, id: u64) {
        self.begin_edit("删除对象");
        self.active_layers.remove(&id);
        let name = self.project.name_of(id).unwrap_or("").to_string();
        self.project.remove(id);
        self.explorer_selection.retain(|selected| *selected != id);
        self.open.remove(&id);
        self.selection.remove(&id);
        if self.active == Some(id) {
            self.active = None;
        }
        if self.props_for == Some(id) {
            self.props_for = None;
        }
        self.sheets.remove(&id);
        self.status = format!("已删除 {}", name);
        self.history.changed();
    }

    pub(crate) fn project_target(&self) -> Option<u64> {
        self.explorer_target.filter(|id| self.project.name_of(*id).is_some())
    }

    pub(super) fn current_folder(&self) -> Option<u64> {
        self.explorer_target.and_then(|id| {
            if self.project.folder(id).is_some() {
                Some(id)
            } else {
                self.project.folder_of(id)
            }
        })
    }

    pub(crate) fn new_folder(&mut self) {
        match self.project.add_folder(self.current_folder()) {
            Ok(id) => {
                self.select_project_item(id, false);
                self.status = "已新建文件夹，可在右键菜单中重命名".into();
                self.project_name = self.project.name_of(id).unwrap_or("").to_string();
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn rename_project_target(&mut self) {
        if let Some(id) = self.project_target() {
            self.status = match self.project.rename_project_item(id, &self.project_name) {
                Ok(()) => "已重命名".into(),
                Err(e) => e,
            };
        }
    }

    pub(crate) fn move_project_target(&mut self) {
        if let Some(id) = self.project_target() {
            let roots = self.selected_project_roots(id);
            // 移动只改变目录结构，保留目录副本即可整批回滚，无需复制表格数据。
            let before = self.project.folders.clone();
            for source in roots {
                if let Err(error) = self.project.move_project_item(source, self.folder_destination) {
                    self.project.folders = before;
                    self.status = error;
                    return;
                }
            }
            self.status = "已移动所选项目对象".into();
        }
    }

    pub(crate) fn remove_project_folder(&mut self) {
        if let Some(id) = self.project_target() {
            let parent = self.project.folder(id).and_then(|f| f.parent);
            self.status = match self.project.remove_folder(id) {
                Ok(()) => {
                    self.explorer_target = parent;
                    self.explorer_selection = parent.into_iter().collect();
                    self.project_name = parent.and_then(|id| self.project.name_of(id)).unwrap_or("").to_string();
                    "已删除文件夹，内容已移到上级".into()
                }
                Err(e) => e,
            };
        }
    }

    pub(crate) fn hide_project_window(&mut self) {
        if let Some(id) = self.project_target() {
            self.open.insert(id, false);
            if self.active == Some(id) {
                self.active = None;
            }
            if self.props_for == Some(id) {
                self.props_for = None;
            }
            self.status = "窗口已隐藏，可从项目管理器重新打开".into();
        }
    }

    pub(crate) fn show_project_window(&mut self) {
        if let Some(id) = self.project_target() {
            self.show_item(id);
        }
    }

    pub(crate) fn set_project_window_state(&mut self, state: WinState) {
        if let Some(id) = self.project_target() {
            self.open.insert(id, true);
            self.geoms
                .entry(id)
                .or_insert_with(|| Geom::new(egui::pos2(20.0, 20.0), egui::vec2(420.0, 440.0)))
                .state = state;
            self.active = Some(id);
            self.raise = Some(id);
        }
    }

    pub(crate) fn show_all(&mut self) {
        for id in self.all_ids() {
            self.open.insert(id, true);
        }
    }

    /// 层叠（false）或平铺（true）所有打开的窗口。
    pub(crate) fn arrange_windows(&mut self, tile: bool) {
        let ids: Vec<u64> = self
            .all_ids()
            .into_iter()
            .filter(|id| self.open.get(id).copied().unwrap_or(false))
            .collect();
        if ids.is_empty() {
            return;
        }
        let (w, h) = (self.ws.width(), self.ws.height());
        let cols = (ids.len() as f32).sqrt().ceil() as usize;
        let rows = ids.len().div_ceil(cols);
        for (i, id) in ids.iter().enumerate() {
            let rect = if tile {
                let (cw, ch) = (w / cols as f32, h / rows as f32);
                egui::Rect::from_min_size(
                    egui::pos2((i % cols) as f32 * cw, (i / cols) as f32 * ch),
                    egui::vec2(cw, ch),
                )
            } else {
                let off = 26.0 * i as f32;
                egui::Rect::from_min_size(
                    egui::pos2(12.0 + off, 10.0 + off),
                    egui::vec2((w * 0.62).clamp(360.0, 760.0), (h * 0.72).clamp(260.0, 560.0)),
                )
            };
            self.geoms.insert(
                *id,
                Geom {
                    rect,
                    state: WinState::Normal,
                    restore: None,
                },
            );
        }
        if let Some(last) = ids.last() {
            self.active = Some(*last);
            self.raise = Some(*last);
        }
    }

    pub(crate) fn minimize_all(&mut self) {
        for (id, open) in &self.open {
            if *open {
                if let Some(g) = self.geoms.get_mut(id) {
                    g.state = WinState::Min;
                }
            }
        }
    }

    pub(crate) fn hide_all(&mut self) {
        for v in self.open.values_mut() {
            *v = false;
        }
        self.active = None;
    }

    pub(super) fn project_item_menu(&mut self, ui: &mut egui::Ui, id: u64) {
        self.explorer_target = Some(id);
        let folder = self.project.folder(id).is_some();
        if folder {
            if ui.button("新建子文件夹").clicked() {
                self.run_command("project.folder.new");
                ui.close_menu();
            }
        } else {
            for (command, label) in [
                ("project.show", "显示窗口"),
                ("project.hide", "隐藏窗口"),
                ("window.minimize", "最小化"),
                ("window.maximize", "最大化"),
                ("window.restore", "还原"),
            ] {
                if ui.button(label).clicked() {
                    self.run_command(command);
                    ui.close_menu();
                }
            }
        }
        ui.separator();
        if self.explorer_selection.len() > 1 {
            ui.label(format!(
                "已选 {} 项，移动将应用于全部所选项",
                self.explorer_selection.len()
            ));
            ui.label("重命名、显示、隐藏及删除针对当前项");
        }
        ui.label("名称");
        let response = ui.text_edit_singleline(&mut self.project_name);
        if ui.button("重命名").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            self.run_command("project.rename");
            ui.close_menu();
        }
        ui.separator();
        egui::ComboBox::from_id_salt(("move-project", id))
            .selected_text(
                self.folder_destination
                    .and_then(|d| self.project.folder_path(d))
                    .unwrap_or_else(|| "项目根目录".into()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.folder_destination, None, "项目根目录");
                for f in &self.project.folders {
                    if f.id != id {
                        ui.selectable_value(
                            &mut self.folder_destination,
                            Some(f.id),
                            self.project.folder_path(f.id).unwrap(),
                        );
                    }
                }
            });
        if ui.button("移动到所选位置").clicked() {
            self.run_command("project.move");
            ui.close_menu();
        }
        ui.separator();
        if folder {
            if ui.button("删除文件夹（保留内容）").clicked() {
                self.run_command("project.folder.remove");
                ui.close_menu();
            }
        } else if ui.button("删除对象…").clicked() {
            self.run_command("project.delete");
            ui.close_menu();
        }
    }

    pub(super) fn start_explorer_rename(&mut self, id: u64) {
        self.select_project_item(id, false);
        self.explorer_target = Some(id);
        self.project_name = self.project.name_of(id).unwrap_or("").into();
        self.explorer_click = None;
        self.explorer_rename = Some((id, true));
    }

    pub(super) fn finish_explorer_rename(&mut self, cancel: bool) {
        if let Some((id, _)) = self.explorer_rename.take() {
            if !cancel {
                self.explorer_target = Some(id);
                self.run_command("project.rename");
            }
            self.project_name = self.project.name_of(id).unwrap_or("").into();
        }
    }

    pub(super) fn explorer_label(&mut self, ui: &mut egui::Ui, id: u64, text: RichText) -> egui::Response {
        if let Some((editing, focus)) = self.explorer_rename {
            if editing == id {
                let mut output = egui::TextEdit::singleline(&mut self.project_name)
                    .id_salt(("project-rename", id))
                    .desired_width(ui.available_width())
                    .show(ui);
                if focus {
                    output.response.request_focus();
                    output.state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.project_name.chars().count()),
                    )));
                    output.state.store(ui.ctx(), output.response.id);
                    self.explorer_rename = Some((id, false));
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.finish_explorer_rename(true);
                } else if output.response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.finish_explorer_rename(false);
                }
                return output.response;
            }
        }
        let response = ui.add(
            egui::Button::new(text)
                .frame(false)
                .selected(self.explorer_selection.contains(&id) || self.explorer_target == Some(id))
                .sense(egui::Sense::click_and_drag()),
        );
        response.dnd_set_drag_payload(ProjectDrag(id));
        if response.drag_started() {
            self.finish_explorer_rename(false);
            if !self.explorer_selection.contains(&id) {
                self.select_project_item(id, false);
            }
            self.explorer_target = Some(id);
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            self.explorer_click = None;
        }
        if response.clicked() {
            self.finish_explorer_rename(false);
            let modifiers = ui.input(|i| i.modifiers);
            if modifiers.shift {
                self.range_select_project_item(id, modifiers.ctrl);
                return response;
            }
            let extend = modifiers.ctrl;
            let now = ui.input(|i| i.time);
            let slow_second_click = !extend
                && self.explorer_selection.len() <= 1
                && self.explorer_target == Some(id)
                && self
                    .explorer_click
                    .is_some_and(|(last, time)| last == id && (0.5..=2.0).contains(&(now - time)));
            self.select_project_item(id, extend);
            if !extend && self.project.folder(id).is_none() {
                self.run_command("project.show");
            }
            if slow_second_click {
                self.start_explorer_rename(id);
            } else if !extend {
                self.explorer_click = Some((id, now));
            } else {
                self.explorer_click = None;
            }
        }
        if response.secondary_clicked() {
            self.finish_explorer_rename(false);
            if !self.explorer_selection.contains(&id) {
                self.select_project_item(id, false);
            }
            self.explorer_click = None;
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            self.folder_destination = self
                .project
                .folder(id)
                .map_or_else(|| self.project.folder_of(id), |f| f.parent);
        }
        response.context_menu(|ui| self.project_item_menu(ui, id));
        response.on_hover_text(
            "Ctrl+单击添加或取消选择；拖到文件夹或根目录移动所选项；单选后隔 0.5–2 秒再单击名称重命名，也可按 F2",
        )
    }

    pub(super) fn explorer_drop(&mut self, ui: &mut egui::Ui, response: &egui::Response, destination: Option<u64>) {
        if let Some(payload) = response.dnd_hover_payload::<ProjectDrag>() {
            let mut ancestor = destination;
            let mut valid = true;
            while let Some(id) = ancestor {
                if self.selected_project_roots(payload.0).contains(&id) {
                    valid = false;
                    break;
                }
                ancestor = self.project.folder(id).and_then(|f| f.parent);
            }
            ui.ctx().set_cursor_icon(if valid {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::NotAllowed
            });
            ui.painter()
                .rect_stroke(response.rect.expand(2.0), 3.0, ui.visuals().selection.stroke);
        }
        if let Some(payload) = response.dnd_release_payload::<ProjectDrag>() {
            self.finish_explorer_rename(false);
            self.explorer_click = None;
            let source = payload.0;
            let changed = self.selected_project_roots(source).iter().any(|id| {
                self.project
                    .folder(*id)
                    .map_or_else(|| self.project.folder_of(*id), |f| f.parent)
                    != destination
            });
            if changed {
                self.explorer_target = Some(source);
                self.folder_destination = destination;
                self.run_command("project.move");
                if let Some(id) = destination {
                    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        egui::Id::new(("project-folder", id)),
                        true,
                    );
                    state.set_open(true);
                    state.store(ui.ctx());
                }
            }
        }
    }

    pub(super) fn explorer_folder(&mut self, ui: &mut egui::Ui, parent: Option<u64>, visible: &mut Vec<u64>) {
        let mut folders: Vec<_> = self
            .project
            .folders
            .iter()
            .filter(|f| f.parent == parent)
            .map(|f| (f.id, f.name.clone()))
            .collect();
        folders.sort_by_key(|(_, name)| name.to_lowercase());
        for (id, name) in folders {
            visible.push(id);
            let (arrow, header, _) = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                egui::Id::new(("project-folder", id)),
                true,
            )
            .show_header(ui, |ui| self.explorer_label(ui, id, RichText::new(name)))
            .body(|ui| self.explorer_folder(ui, Some(id), visible));
            if arrow.clicked() {
                self.explorer_click = None;
            }
            self.explorer_drop(ui, &header.inner.union(arrow), Some(id));
        }
        let mut items: Vec<_> = self
            .project
            .books
            .iter()
            .map(|b| (b.id, 0, b.name.clone()))
            .chain(self.project.graphs.iter().map(|g| (g.id, 1, g.name.clone())))
            .chain(self.project.notes.iter().map(|n| (n.id, 2, n.name.clone())))
            .filter(|(id, _, _)| self.project.folder_of(*id) == parent)
            .collect();
        items.sort_by_key(|(_, kind, name)| {
            (
                if self.explorer_sort == ExplorerSort::Type {
                    *kind
                } else {
                    0
                },
                name.to_lowercase(),
            )
        });
        for (id, kind, name) in items {
            visible.push(id);
            let shown = self.open.get(&id).copied().unwrap_or(false);
            ui.horizontal(|ui| {
                ui.add(
                    crate::icons::icon(["book", "graph", "note"][kind], self.pal.dark)
                        .fit_to_exact_size(egui::vec2(16.0, 16.0)),
                )
                .on_hover_text(["数据表", "图形", "Note 笔记"][kind]);
                self.explorer_label(
                    ui,
                    id,
                    if shown {
                        RichText::new(&name)
                    } else {
                        RichText::new(&name).weak()
                    },
                )
            });
        }
    }

    pub(super) fn explorer(&mut self, ui: &mut egui::Ui) {
        if let Some(pos) = ui.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten()) {
            self.explorer_focus = ui.max_rect().contains(pos) && ui.ctx().layer_id_at(pos) == Some(ui.layer_id());
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("项目管理器").strong());
            if self.explorer_selection.len() > 1 {
                ui.weak(format!("{} 项", self.explorer_selection.len()));
            }
        });
        ui.horizontal(|ui| {
            let root = ui.button("项目根目录");
            if root.clicked() {
                self.finish_explorer_rename(false);
                self.explorer_target = None;
                self.explorer_selection.clear();
                self.explorer_anchor = None;
                self.project_name.clear();
                self.explorer_click = None;
            }
            self.explorer_drop(ui, &root, None);
            if ui.button("新建文件夹").clicked() {
                self.run_command("project.folder.new");
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("新建表").clicked() {
                self.run_command("book.new");
            }
            if ui
                .button("新建图")
                .on_hover_text("新建空白图形，随后在绘图属性中选择数据")
                .clicked()
            {
                self.run_command("graph.new");
            }
            if ui.button("新建 Note").clicked() {
                self.run_command("note.new");
            }
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("explorer-sort")
                .selected_text(if self.explorer_sort == ExplorerSort::Type {
                    "按类型排序"
                } else {
                    "按名称排序"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.explorer_sort, ExplorerSort::Type, "按类型排序");
                    ui.selectable_value(&mut self.explorer_sort, ExplorerSort::Name, "按名称排序");
                });
        });
        // 面板先于子窗口绘制：只在项目管理器有焦点时处理 F2，并消费按键，避免工作表同时开始编辑单元格。
        if self.explorer_rename.is_none() && self.explorer_focus && !ui.ctx().wants_keyboard_input() {
            if let Some(id) = self.project_target() {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
                    self.start_explorer_rename(id);
                }
            }
        }
        ui.separator();
        let mut visible = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| self.explorer_folder(ui, None, &mut visible));
        self.explorer_visible = visible;
    }

    pub(super) fn child_windows(&mut self, ctx: &egui::Context) {
        let ws = ctx.available_rect();
        self.ws = ws;
        let ids = self.all_ids();

        // 鼠标按下时，按下位置所在的窗口成为当前窗口
        let press = ctx.input(|i| {
            if i.pointer.any_pressed() {
                i.pointer.interact_pos()
            } else {
                None
            }
        });
        if let Some(pos) = press {
            if let Some(layer) = ctx.layer_id_at(pos) {
                if let Some(id) = ids.iter().copied().find(|id| win_id(*id) == layer.id) {
                    self.active = Some(id);
                    if self.explorer_target != Some(id) {
                        self.project_name = self.project.name_of(id).unwrap_or("").to_string();
                    }
                    self.explorer_target = Some(id);
                }
            }
        }

        let mut min_slot = 0;
        let mut snapped = None;
        for id in ids {
            if !self.open.get(&id).copied().unwrap_or(false) {
                continue;
            }
            let is_book = self.project.book(id).is_some();
            let name = self.project.name_of(id).unwrap_or("").to_string();
            if self.raise == Some(id) {
                ctx.move_to_top(mdi::layer_id(win_id(id)));
            }
            let slot = (id.saturating_sub(1) % 6) as f32;
            let mut geom = *self.geoms.entry(id).or_insert_with(|| {
                if is_book {
                    Geom::new(
                        egui::pos2(12.0 + 26.0 * slot, 10.0 + 24.0 * slot),
                        egui::vec2(420.0, 440.0),
                    )
                } else {
                    Geom::new(
                        egui::pos2(450.0 + 26.0 * slot, 10.0 + 24.0 * slot),
                        egui::vec2(560.0, 440.0),
                    )
                }
            });
            let this_slot = min_slot;
            if geom.state == WinState::Min {
                min_slot += 1;
            }
            let active = self.active == Some(id);
            let pal = self.pal;
            let resp = mdi::window(
                ctx,
                win_id(id),
                ws,
                &mut geom,
                this_slot,
                &name,
                if is_book {
                    "book"
                } else if self.project.note(id).is_some() {
                    "note"
                } else {
                    "graph"
                },
                active,
                &pal,
                |ui| {
                    if is_book {
                        self.book_ui(ui, id);
                    } else if self.project.note(id).is_some() {
                        self.note_ui(ui, id);
                    } else {
                        self.graph_ui(ui, id);
                    }
                },
            );
            self.geoms.insert(id, geom);
            if let Some(snap) = resp.snapped {
                snapped = Some((id, snap));
            }
            if resp.pressed {
                self.explorer_anchor = Some(id);
                self.explorer_selection = vec![id];
                self.active = Some(id);
                if self.explorer_target != Some(id) {
                    self.project_name = self.project.name_of(id).unwrap_or("").to_string();
                }
                self.explorer_target = Some(id);
            }
            if let Some(state) = resp.state {
                self.explorer_target = Some(id);
                self.run_command(match state {
                    WinState::Min => "window.minimize",
                    WinState::Max => "window.maximize",
                    WinState::Normal => "window.restore",
                });
            }
            if resp.close {
                self.explorer_target = Some(id);
                self.run_command("project.hide");
            }
        }
        self.raise = None;
        if let Some((id, snap)) = snapped {
            self.start_snap_assist(id, snap);
        }
        self.snap_assist_ui(ctx, ws);
    }
}
