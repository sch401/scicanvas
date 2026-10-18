//! Project explorer, object operations and workspace child windows.

use super::*;

impl SciApp {
    pub(super) fn show_item(&mut self, id: u64) {
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
            .collect()
    }

    // ---------------------------------------------------------------- 操作

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
                self.explorer_target = Some(id);
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
            self.status = match self.project.move_project_item(id, self.folder_destination) {
                Ok(()) => "已移动项目对象".into(),
                Err(e) => e,
            };
        }
    }

    pub(crate) fn remove_project_folder(&mut self) {
        if let Some(id) = self.project_target() {
            let parent = self.project.folder(id).and_then(|f| f.parent);
            self.status = match self.project.remove_folder(id) {
                Ok(()) => {
                    self.explorer_target = parent;
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
                .selected(self.explorer_target == Some(id))
                .sense(egui::Sense::click_and_drag()),
        );
        response.dnd_set_drag_payload(ProjectDrag(id));
        if response.drag_started() {
            self.finish_explorer_rename(false);
            self.explorer_target = Some(id);
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            self.explorer_click = None;
        }
        if response.clicked() {
            self.finish_explorer_rename(false);
            let now = ui.input(|i| i.time);
            let slow_second_click = self.explorer_target == Some(id)
                && self
                    .explorer_click
                    .is_some_and(|(last, time)| last == id && (0.5..=2.0).contains(&(now - time)));
            self.explorer_target = Some(id);
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            if self.project.folder(id).is_none() {
                self.run_command("project.show");
            }
            if slow_second_click {
                self.start_explorer_rename(id);
            } else {
                self.explorer_click = Some((id, now));
            }
        }
        if response.secondary_clicked() {
            self.finish_explorer_rename(false);
            self.explorer_click = None;
            self.project_name = self.project.name_of(id).unwrap_or("").into();
            self.folder_destination = self
                .project
                .folder(id)
                .map_or_else(|| self.project.folder_of(id), |f| f.parent);
        }
        response.context_menu(|ui| self.project_item_menu(ui, id));
        response.on_hover_text("拖到文件夹或项目根目录移动；选中后隔 0.5–2 秒再单击名称重命名，也可按 F2")
    }

    pub(super) fn explorer_drop(&mut self, ui: &mut egui::Ui, response: &egui::Response, destination: Option<u64>) {
        if let Some(payload) = response.dnd_hover_payload::<ProjectDrag>() {
            let mut ancestor = destination;
            let mut valid = true;
            while let Some(id) = ancestor {
                if id == payload.0 {
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
            let current = self
                .project
                .folder(source)
                .map_or_else(|| self.project.folder_of(source), |f| f.parent);
            if current != destination {
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

    pub(super) fn explorer_folder(&mut self, ui: &mut egui::Ui, parent: Option<u64>) {
        let folders: Vec<_> = self
            .project
            .folders
            .iter()
            .filter(|f| f.parent == parent)
            .map(|f| (f.id, f.name.clone()))
            .collect();
        for (id, name) in folders {
            let (arrow, header, _) = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                egui::Id::new(("project-folder", id)),
                true,
            )
            .show_header(ui, |ui| self.explorer_label(ui, id, RichText::new(name)))
            .body(|ui| self.explorer_folder(ui, Some(id)));
            if arrow.clicked() {
                self.explorer_click = None;
            }
            self.explorer_drop(ui, &header.inner.union(arrow), Some(id));
        }
        let items: Vec<_> = self
            .project
            .books
            .iter()
            .map(|b| (b.id, true, b.name.clone()))
            .chain(self.project.graphs.iter().map(|g| (g.id, false, g.name.clone())))
            .filter(|(id, _, _)| self.project.folder_of(*id) == parent)
            .collect();
        for (id, is_book, name) in items {
            let shown = self.open.get(&id).copied().unwrap_or(false);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_book { "表" } else { "图" })
                        .small()
                        .strong()
                        .color(if is_book { self.pal.book } else { self.pal.graph }),
                );
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
        ui.label(RichText::new("项目管理器").strong());
        ui.horizontal(|ui| {
            let root = ui.button("项目根目录");
            if root.clicked() {
                self.finish_explorer_rename(false);
                self.explorer_target = None;
                self.project_name.clear();
                self.explorer_click = None;
            }
            self.explorer_drop(ui, &root, None);
            if ui.button("新建文件夹").clicked() {
                self.run_command("project.folder.new");
            }
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
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| self.explorer_folder(ui, None));
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
                if is_book { "book" } else { "graph" },
                active,
                &pal,
                |ui| {
                    if is_book {
                        self.book_ui(ui, id);
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
