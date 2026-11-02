//! 工作表表头、元数据行、单元格和右键菜单的绘制。

use super::*;

/// 工作表一次绘制所需的输入和待处理结果，不跨帧保存。
pub(super) struct SheetFrame {
    pub(super) frozen: bool,
    pub(super) id: u64,
    pub(super) pal: Palette,
    pub(super) active: bool,
    pub(super) modifiers: egui::Modifiers,
    pub(super) pressed: bool,
    pub(super) down: bool,
    pub(super) dbl: bool,
    pub(super) ptr: Option<egui::Pos2>,
    pub(super) focus_id: egui::Id,
    pub(super) edit_id: egui::Id,
    pub(super) graphs: Vec<(u64, String)>,
    pub(super) sel_flags: Vec<bool>,
    pub(super) ncols: usize,
    pub(super) nrows: usize,
    pub(super) frozen_columns: usize,
    pub(super) frozen_scroll_y: f32,
    pub(super) visible_rows: Option<std::sync::Arc<[usize]>>,
    pub(super) range: Option<Range>,
    pub(super) multi: Option<Range>,
    pub(super) drag: Option<Drag>,
    pub(super) scroll_to: Option<usize>,
    pub(super) edit: Option<Edit>,
    pub(super) text_color: egui::Color32,
    pub(super) sel_stroke: egui::Stroke,
    pub(super) dragging_cells: bool,
    pub(super) view: egui::Rect,
    pub(super) grid: egui::Stroke,
    pub(super) clicked: Option<usize>,
    pub(super) action: Option<(usize, ColAction)>,
    pub(super) press: Option<Target>,
    pub(super) hover: Option<Target>,
    pub(super) dbl_cell: Option<(usize, usize)>,
    pub(super) right_cell: Option<(usize, usize)>,
    pub(super) menu_cmd: Option<&'static str>,
    pub(super) edit_end: Option<(bool, i64, i64)>,
    pub(super) autoscrolled: bool,
    pub(super) metadata_edits: Vec<(usize, usize, String, String)>,
}

impl SheetFrame {
    pub(super) fn draw(
        &mut self,
        ui: &mut egui::Ui,
        book: &mut scicanvas_core::Book,
        preview: Option<&scicanvas_core::Book>,
        history_group: &mut Option<egui::Id>,
    ) {
        ui.push_id(("sheet", self.id), |ui| {
            if self.frozen_columns > 0 {
                self.draw_frozen(ui, book, preview, history_group);
                return;
            }
            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .drag_to_scroll(false)
                .show(ui, |ui| {
                    // 单元格之间不留空隙，选区和网格线才能连成一片
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    let mut table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .auto_shrink([false, false])
                        .min_scrolled_height(0.0)
                        .drag_to_scroll(false)
                        .column(TCol::exact(58.0))
                        .columns(TCol::initial(96.0).at_least(40.0).clip(true), self.ncols);
                    if let Some(r) = self.scroll_to {
                        table = table.scroll_to_row(r + META_ROWS.len(), None);
                    }
                    table
                        .header(24.0, |header| self.header(header, book, preview, 0, self.ncols, true))
                        .body(|body| self.body(body, book, preview, history_group, (0, self.ncols), true));
                });
        });
    }
    fn draw_frozen(
        &mut self,
        ui: &mut egui::Ui,
        book: &mut scicanvas_core::Book,
        preview: Option<&scicanvas_core::Book>,
        history_group: &mut Option<egui::Id>,
    ) {
        let bounds = ui.available_rect_before_wrap();
        let width = (58.0 + self.frozen_columns as f32 * 96.0).min((bounds.width() - 70.0).max(58.0));
        let split = bounds.left() + width;
        let left = egui::Rect::from_min_max(bounds.min, egui::pos2(split, bounds.bottom()));
        let right = egui::Rect::from_min_max(egui::pos2(split, bounds.top()), bounds.max);
        let wheel = if ui
            .ctx()
            .input(|i| i.pointer.hover_pos().is_some_and(|p| left.contains(p)))
        {
            ui.ctx().input(|i| i.smooth_scroll_delta.y)
        } else {
            0.0
        };
        let forced = (wheel != 0.0).then_some((self.frozen_scroll_y - wheel).max(0.0));
        let start = self.frozen_columns;
        let end = self.ncols;
        let output = ui.allocate_new_ui(
            egui::UiBuilder::new().id_salt("scrolling-region").max_rect(right),
            |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .drag_to_scroll(false)
                    .show(ui, |ui| {
                        self.draw_slice(ui, book, preview, history_group, start, end, false, forced)
                    })
            },
        );
        self.frozen_scroll_y = output.inner.inner.state.offset.y;
        ui.allocate_new_ui(egui::UiBuilder::new().id_salt("frozen-region").max_rect(left), |ui| {
            ui.set_clip_rect(left.intersect(ui.clip_rect()));
            self.draw_slice(
                ui,
                book,
                preview,
                history_group,
                0,
                start,
                true,
                Some(self.frozen_scroll_y),
            );
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_slice(
        &mut self,
        ui: &mut egui::Ui,
        book: &mut scicanvas_core::Book,
        preview: Option<&scicanvas_core::Book>,
        history_group: &mut Option<egui::Id>,
        start: usize,
        end: usize,
        row_header: bool,
        offset: Option<f32>,
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let mut table = TableBuilder::new(ui)
            .id_salt(if row_header { "frozen" } else { "scrolling" })
            .striped(true)
            .resizable(!row_header)
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .drag_to_scroll(false);
        if row_header {
            table = table.column(TCol::exact(58.0));
        }
        if start == end && !row_header {
            table = table.column(TCol::exact(1.0));
        }
        table = table.columns(TCol::initial(96.0).at_least(40.0).clip(true), end - start);
        if let Some(y) = offset {
            table = table.vertical_scroll_offset(y);
        }
        if row_header {
            table = table.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        }
        if let Some(row) = self.scroll_to {
            table = table.scroll_to_row(row + META_ROWS.len(), None);
        }
        table
            .header(24.0, |header| {
                self.header(header, book, preview, start, end, row_header)
            })
            .body(|body| self.body(body, book, preview, history_group, (start, end), row_header))
    }

    fn header(
        &mut self,
        mut header: egui_extras::TableRow<'_, '_>,
        book: &scicanvas_core::Book,
        preview: Option<&scicanvas_core::Book>,
        start: usize,
        end: usize,
        row_header: bool,
    ) {
        if row_header || start == end {
            header.col(|ui| {
                ui.label("");
            });
        }
        for (ci, col) in book.columns.iter().enumerate().skip(start).take(end - start) {
            let col = preview.and_then(|b| b.column(&col.key)).unwrap_or(col);
            header.col(|ui| {
                let rect = ui.max_rect();
                // 带保存公式的列标 ƒ
                let computed = col.formula.is_some()
                    || col
                        .op
                        .as_ref()
                        .is_some_and(|op| op.mode != scicanvas_core::ops::RecalcMode::None);
                let mark = if computed { " ƒ" } else { "" };
                let text = RichText::new(format!("{}({}){mark}", col.key, col.role.label())).strong();
                let resp = ui
                    .centered_and_justified(|ui| ui.add(egui::SelectableLabel::new(self.sel_flags[ci], text)))
                    .inner
                    .on_hover_text("单击选中列，Ctrl / Shift 多选，拖动选多列，右键更多操作");
                if self.multi.is_some_and(|r| ci >= r.c0() && ci <= r.c1()) {
                    ui.painter().line_segment(
                        [
                            rect.left_bottom() + egui::vec2(0.0, -1.0),
                            rect.right_bottom() + egui::vec2(0.0, -1.0),
                        ],
                        self.sel_stroke,
                    );
                }
                if self.pressed && ui.rect_contains_pointer(rect) {
                    self.press = Some(Target::Col(ci));
                }
                if matches!(self.drag, Some(Drag::Cols(_))) && self.down && ui.rect_contains_pointer(rect) {
                    self.hover = Some(Target::Col(ci));
                }
                if resp.clicked() {
                    self.clicked = Some(ci);
                }
                // 双击列标题打开列属性（与 Origin 相同）
                if resp.double_clicked() {
                    self.action = Some((ci, ColAction::Props));
                }
                resp.context_menu(|ui| {
                    self.column_menu(ui, col, ci);
                });
                let active = book
                    .filter
                    .as_ref()
                    .is_some_and(|filter| filter.rules.contains_key(&col.key));
                let button_rect = egui::Rect::from_min_max(egui::pos2(rect.right() - 20.0, rect.top()), rect.max);
                let mut button_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("filter-menu", ci))
                        .max_rect(button_rect),
                );
                button_ui.menu_button(
                    RichText::new("▾").color(if active { self.pal.accent } else { self.pal.weak }),
                    |ui| {
                        if ui.button("筛选…").clicked() {
                            self.action = Some((ci, ColAction::Filter));
                            ui.close_menu();
                        }
                        if active && ui.button("清除筛选").clicked() {
                            self.action = Some((ci, ColAction::ClearFilter));
                            ui.close_menu();
                        }
                    },
                );
            });
        }
    }
    fn column_menu(&mut self, ui: &mut egui::Ui, col: &scicanvas_core::Column, ci: usize) {
        let mut pick = |ui: &mut egui::Ui, label: &str, a: ColAction| {
            if ui.button(label).clicked() {
                self.action = Some((ci, a));
                ui.close_menu();
            }
        };
        pick(ui, "列属性…", ColAction::Props);
        ui.separator();
        pick(ui, "设为 X", ColAction::SetRole(Role::X));
        pick(ui, "设为 Y", ColAction::SetRole(Role::Y));
        pick(ui, "设为 X 误差", ColAction::SetRole(Role::XError));
        pick(ui, "设为 Y 误差", ColAction::SetRole(Role::YError));
        pick(ui, "设为 Z", ColAction::SetRole(Role::Z));
        pick(ui, "设为标签", ColAction::SetRole(Role::Label));
        pick(ui, "设为忽略", ColAction::SetRole(Role::Ignore));
        ui.separator();
        ui.label(format!("数据类型：{}", col.data.kind().label()));
        if col.data.kind() == scicanvas_core::DataKind::Category {
            pick(ui, "类别顺序…", ColAction::CategoryOrder);
        }
        for kind in scicanvas_core::DataKind::ALL {
            pick(ui, &format!("转换为{}列", kind.label()), ColAction::SetDataKind(kind));
        }
        ui.separator();
        for t in PlotType::ALL {
            pick(ui, t.label(), ColAction::Plot(t));
        }
        if !self.graphs.is_empty() {
            ui.menu_button("添加到图形", |ui| {
                for (gid, name) in &self.graphs {
                    pick(ui, name, ColAction::AddToGraph(*gid));
                }
            });
        }
        ui.separator();
        ui.menu_button("移动列", |ui| {
            pick(ui, "左移", ColAction::Move(scicanvas_core::ColumnMove::Left));
            pick(ui, "右移", ColAction::Move(scicanvas_core::ColumnMove::Right));
            pick(ui, "移到最前", ColAction::Move(scicanvas_core::ColumnMove::First));
            pick(ui, "移到最后", ColAction::Move(scicanvas_core::ColumnMove::Last));
        });
        pick(ui, "在右侧插入列", ColAction::InsertRight);
        pick(ui, "清空", ColAction::Clear);
        pick(ui, "删除列", ColAction::Delete);
    }
    fn body(
        &mut self,
        body: egui_extras::TableBody<'_>,
        book: &mut scicanvas_core::Book,
        preview: Option<&scicanvas_core::Book>,
        history_group: &mut Option<egui::Id>,
        columns: (usize, usize),
        row_header: bool,
    ) {
        let (start, end) = columns;
        body.rows(22.0, META_ROWS.len() + self.nrows, |mut row| {
            let ri = row.index();
            if ri < META_ROWS.len() {
                self.metadata_row(row, book, preview, history_group, (start, end), row_header);
                return;
            }
            let visible_row = ri - META_ROWS.len();
            let r = self.visible_rows.as_ref().map_or(visible_row, |rows| rows[visible_row]);
            if row_header {
                row.col(|ui| self.row_header(ui, r));
            } else if start == end {
                row.col(|_| {});
            }
            for (ci, col) in book.columns.iter_mut().enumerate().skip(start).take(end - start) {
                row.col(|ui| self.cell(ui, col, ci, r, preview, history_group));
            }
        });
    }
    fn metadata_row(
        &mut self,
        mut row: egui_extras::TableRow<'_, '_>,
        book: &mut scicanvas_core::Book,
        preview: Option<&scicanvas_core::Book>,
        history_group: &mut Option<egui::Id>,
        columns: (usize, usize),
        row_header: bool,
    ) {
        let (start, end) = columns;
        let ri = row.index();
        if row_header {
            row.col(|ui| {
                ui.painter().rect_filled(ui.max_rect(), 0.0, self.pal.meta_bg);
                ui.centered_and_justified(|ui| {
                    ui.label(META_ROWS[ri]);
                });
            });
        } else if start == end {
            row.col(|_| {});
        }
        for (ci, col) in book.columns.iter_mut().enumerate().skip(start).take(end - start) {
            row.col(|ui| {
                let bg = if self.sel_flags[ci] {
                    self.pal.meta_sel_bg
                } else {
                    self.pal.meta_bg
                };
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, bg);
                ui.painter().line_segment(
                    [rect.right_top(), rect.right_bottom()],
                    egui::Stroke::new(1.0_f32, self.pal.window),
                );
                if let Some(col) = preview.and_then(|b| b.column(&col.key)) {
                    let text = match ri {
                        0 => &col.long_name,
                        1 => &col.units,
                        _ => &col.comment,
                    };
                    ui.centered_and_justified(|ui| {
                        ui.label(text);
                    });
                    return;
                }
                let field = header_field(col, ri);
                let before = field.clone();
                let response = ui.add(
                    egui::TextEdit::singleline(field)
                        .frame(false)
                        .horizontal_align(egui::Align::Center)
                        .margin(egui::vec2(5.0, 3.0))
                        .desired_width(f32::INFINITY),
                );
                if response.changed() {
                    self.metadata_edits.push((ci, ri, before, field.clone()));
                }
                edit_group(&response, history_group);
            });
        }
    }
    fn row_header(&mut self, ui: &mut egui::Ui, r: usize) {
        let rect = ui.max_rect();
        let in_rows = self.multi.is_some_and(|g| r >= g.r0() && r <= g.r1());
        if in_rows {
            ui.painter().rect_filled(rect, 0.0, self.pal.sel_bg);
        }
        let resp = ui.interact(
            rect,
            egui::Id::new(("row-head", self.id, r)),
            egui::Sense::click_and_drag(),
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
        }
        ui.painter().with_clip_rect(rect).text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            (r + 1).to_string(),
            egui::FontId::monospace(crate::theme::size::MONO_SMALL),
            self.pal.weak,
        );
        if self.pressed && ui.rect_contains_pointer(rect) {
            self.press = Some(Target::Row(r));
        }
        if self.dragging_cells && ui.rect_contains_pointer(rect) {
            self.hover = Some(Target::Row(r));
        }
        // 拖到表格上下边缘时自动滚动
        if self.dragging_cells && !self.autoscrolled {
            self.autoscrolled = true;
            if let Some(p) = self.ptr {
                if p.y > self.view.bottom() - 12.0 {
                    ui.scroll_with_delta(egui::vec2(0.0, -16.0));
                } else if p.y < self.view.top() + 24.0 + 6.0 {
                    ui.scroll_with_delta(egui::vec2(0.0, 16.0));
                }
            }
        }
    }
    fn cell(
        &mut self,
        ui: &mut egui::Ui,
        col: &mut scicanvas_core::Column,
        ci: usize,
        r: usize,
        preview: Option<&scicanvas_core::Book>,
        history_group: &mut Option<egui::Id>,
    ) {
        let rect = ui.max_rect();
        let here = Cell { c: ci, r };
        let painter = ui.painter().with_clip_rect(rect.expand(1.0).intersect(ui.clip_rect()));

        if let Some(e) = self.edit.as_mut().filter(|e| e.cell == here) {
            painter.rect_filled(rect, 0.0, self.pal.window);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut e.buf)
                    .id(self.edit_id)
                    .frame(false)
                    .margin(egui::vec2(5.0, 3.0))
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            );
            if !e.focused && !self.frozen {
                resp.request_focus();
                e.focused = true;
            }
            edit_group(&resp, history_group);
            // Tab、上下方向键由表格处理（结束编辑并移动），不让 egui 切换焦点
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    self.edit_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                )
            });
            let (enter, tab, up, dn, esc, shift) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Tab),
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                    i.key_pressed(egui::Key::Escape),
                    i.modifiers.shift,
                )
            });
            if esc {
                self.edit_end = Some((false, 0, 0));
            } else if tab {
                self.edit_end = Some((true, if shift { -1 } else { 1 }, 0));
            } else if up {
                self.edit_end = Some((true, 0, -1));
            } else if dn {
                self.edit_end = Some((true, 0, 1));
            } else if resp.lost_focus() && !self.frozen {
                self.edit_end = Some((
                    true,
                    0,
                    if enter {
                        if shift {
                            -1
                        } else {
                            1
                        }
                    } else {
                        0
                    },
                ));
            }
            painter.rect_stroke(rect.shrink(1.0), 0.0, self.sel_stroke);
            return;
        }

        let in_range = self.multi.is_some_and(|g| g.contains(ci, r));
        if self.sel_flags[ci] || in_range {
            painter.rect_filled(rect, 0.0, self.pal.sel_bg);
        }
        painter.line_segment([rect.right_top(), rect.right_bottom()], self.grid);
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], self.grid);
        let display = preview.and_then(|b| b.column(&col.key)).unwrap_or(col);
        let shown = (r < display.data.len()).then(|| display.display_at(r));
        if let Some(s) = shown.filter(|s| !s.is_empty()) {
            ui.painter().with_clip_rect(rect.intersect(ui.clip_rect())).text(
                rect.right_center() - egui::vec2(5.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                s,
                egui::FontId::monospace(crate::theme::size::MONO),
                self.text_color,
            );
        }
        let resp = ui.interact(
            rect,
            egui::Id::new(("cell", self.id, ci, r)),
            egui::Sense::click_and_drag(),
        );
        if self.pressed && ui.rect_contains_pointer(rect) {
            self.press = Some(Target::Cell(ci, r));
        }
        if self.dbl && ui.rect_contains_pointer(rect) {
            self.dbl_cell = Some((ci, r));
        }
        if self.dragging_cells && ui.rect_contains_pointer(rect) {
            self.hover = Some(Target::Cell(ci, r));
        }
        if resp.secondary_clicked() {
            self.right_cell = Some((ci, r));
        }
        resp.context_menu(|ui| {
            self.cell_menu(ui);
        });

        // 选区外框和活动单元格
        self.cell_selection(&painter, rect, here);
    }
    fn cell_menu(&mut self, ui: &mut egui::Ui) {
        for (cid, label) in [
            ("edit.copy", "复制"),
            ("edit.clear", "清除内容"),
            ("", ""),
            ("row.insert", "插入行"),
            ("row.delete", "删除行"),
            ("", ""),
            ("plot.line", "折线图"),
            ("plot.scatter", "散点图"),
            ("plot.linesym", "点线图"),
            ("plot.bar", "柱状图"),
            ("plot.stacked_bar", "堆叠柱图"),
            ("plot.percent_stacked_bar", "百分比堆叠柱图"),
            ("plot.floating_bar", "浮动柱图"),
            ("plot.horizontal_bar", "条形图"),
            ("plot.area", "面积图"),
            ("plot.spline", "样条曲线图"),
            ("plot.spline_symbol", "样条点线图"),
            ("plot.vertical_drop", "垂线图"),
            ("plot.step_horizontal", "水平阶梯图"),
            ("plot.step_vertical", "垂直阶梯图"),
        ] {
            if cid.is_empty() {
                ui.separator();
            } else if ui.button(label).clicked() {
                self.menu_cmd = Some(cid);
                ui.close_menu();
            }
        }
    }
    fn cell_selection(&self, painter: &egui::Painter, rect: egui::Rect, here: Cell) {
        let Cell { c: ci, r } = here;
        if let Some(g) = self.range {
            if g.is_multi() && g.contains(ci, r) {
                let (lt, rt, lb, rb) = (
                    rect.left_top(),
                    rect.right_top(),
                    rect.left_bottom(),
                    rect.right_bottom(),
                );
                if ci == g.c0() {
                    painter.line_segment([lt, lb], self.sel_stroke);
                }
                if ci == g.c1() {
                    painter.line_segment([rt, rb], self.sel_stroke);
                }
                if r == g.r0() {
                    painter.line_segment([lt, rt], self.sel_stroke);
                }
                if r == g.r1() {
                    painter.line_segment([lb, rb], self.sel_stroke);
                }
            }
            if g.a == here && self.active {
                painter.rect_stroke(rect.shrink(1.0), 0.0, self.sel_stroke);
            }
        }
    }
}
