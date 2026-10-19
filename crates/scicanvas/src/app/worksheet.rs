//! Worksheet rendering, selection, keyboard editing and column operations.

use super::*;

impl SciApp {
    pub(crate) fn selected_keys(&self, book_id: u64) -> Vec<String> {
        self.selection.get(&book_id).cloned().unwrap_or_default()
    }

    /// 单元格选区（一个格子不算）。
    pub(crate) fn multi_range(&self, book_id: u64) -> Option<Range> {
        self.sheets.get(&book_id).and_then(|s| s.range).filter(|r| r.is_multi())
    }

    /// 要处理的列和行：优先用列标题选择；没有选列时用单元格选区覆盖的列和行。
    pub(crate) fn picked(&self, book_id: u64) -> (Vec<String>, Option<(usize, usize)>) {
        let sel = self.selected_keys(book_id);
        if !sel.is_empty() {
            return (sel, None);
        }
        match (self.multi_range(book_id), self.project.book(book_id)) {
            (Some(r), Some(b)) if !b.columns.is_empty() => {
                let keys = b.columns[r.c0()..=r.c1().min(b.columns.len() - 1)]
                    .iter()
                    .map(|c| c.key.clone())
                    .collect();
                let rows = if r.r1() > r.r0() { Some((r.r0(), r.r1())) } else { None };
                (keys, rows)
            }
            _ => (sel, None),
        }
    }

    pub(crate) fn copy_range(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        if let Some(b) = self.project.book(bid) {
            self.pending_copy = Some(b.range_tsv(r.c0(), r.c1(), r.r0(), r.r1()));
            self.status = format!("已复制 {} 行 × {} 列，可直接粘贴到 Excel", r.rows(), r.cols());
        }
    }

    pub(crate) fn clear_range(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        if let Some(b) = self.project.book_mut(bid) {
            b.clear_range(r.c0(), r.c1(), r.r0(), r.r1());
            self.status = format!("已清除 {} 行 × {} 列", r.rows(), r.cols());
        }
    }

    /// 在选区上方插入与选中行数相同的空行。
    pub(crate) fn insert_rows(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        self.project.insert_rows(bid, r.r0(), r.rows());
        self.status = format!("已在第 {} 行上方插入 {} 行", r.r0() + 1, r.rows());
    }

    /// 删除选区覆盖的整行（所有列）。
    pub(crate) fn delete_rows(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        self.project.delete_rows(bid, r.r0(), r.r1());
        if let Some(st) = self.sheets.get_mut(&bid) {
            st.range = Some(Range::single(r.c0(), r.r0()));
        }
        self.status = if r.rows() == 1 {
            format!("已删除第 {} 行", r.r0() + 1)
        } else {
            format!("已删除第 {}–{} 行", r.r0() + 1, r.r1() + 1)
        };
    }

    pub(crate) fn set_role(&mut self, role: Role) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.picked(bid).0;
        if sel.is_empty() {
            self.status = "请先单击列标题选中列".into();
            return;
        }
        if let Some(b) = self.project.book_mut(bid) {
            b.set_roles(&sel, role);
        }
        let name = match role {
            Role::X => "X",
            Role::Y => "Y",
            Role::XError => "X 误差",
            Role::YError => "Y 误差",
            Role::Ignore => "忽略",
            Role::Z => "Z",
            Role::Label => "标签",
        };
        self.status = format!("已将 {} 列设为 {}", sel.join(", "), name);
    }

    pub(crate) fn add_column(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.selected_keys(bid);
        if let Some(b) = self.project.book_mut(bid) {
            let at = sel
                .last()
                .and_then(|k| b.column_index(k))
                .map(|i| i + 1)
                .unwrap_or(b.columns.len());
            let key = b.insert_column(at, Role::Y);
            self.status = format!("已添加列 {}", key);
        }
    }

    /// 打开第一个选中列的属性对话框。
    pub(crate) fn open_column_props(&mut self) {
        let Some(id) = self.active_book() else { return };
        let Some(key) = self.selected_keys(id).into_iter().next() else {
            self.status = "请先单击列标题选中列".into();
            return;
        };
        let Some(col) = self.project.book(id).and_then(|b| b.column(&key)) else {
            return;
        };
        let sample = (0..col.data.len()).find_map(|r| col.data.f64_at(r).filter(|v| v.is_finite()));
        self.dialog = Some(Dialog::ColumnProps {
            edit: dialogs::ColumnEdit {
                key,
                long_name: col.long_name.clone(),
                units: col.units.clone(),
                comment: col.comment.clone(),
                role: col.role,
                kind: col.data.kind(),
                format: col.format,
                recalc: col.op.as_ref().map(|op| op.mode),
            },
            sample,
            error: None,
        });
    }

    /// 应用列属性；类型转换失败时不改动任何内容，错误留在对话框里。
    pub(super) fn apply_column_props(&mut self, edit: dialogs::ColumnEdit) -> Result<(), String> {
        let id = self.active_book().ok_or("找不到工作簿")?;
        let book = self.project.book_mut(id).ok_or("找不到工作簿")?;
        let col = book.column(&edit.key).ok_or("找不到列")?;
        if col.data.kind() != edit.kind {
            book.convert_columns(std::slice::from_ref(&edit.key), edit.kind)?;
        }
        if book.column(&edit.key).is_some_and(|c| c.role != edit.role) {
            book.set_roles(std::slice::from_ref(&edit.key), edit.role);
        }
        let col = book.columns.iter_mut().find(|c| c.key == edit.key).unwrap();
        col.long_name = edit.long_name;
        col.units = edit.units;
        col.comment = edit.comment;
        if let (Some(op), Some(mode)) = (col.op.as_mut(), edit.recalc) {
            op.mode = mode;
        }
        // 时区改变：不带时区写的日期按新时区重新换算，显示的墙上时间不变
        let offset_changed = col.format.utc_offset != edit.format.utc_offset;
        col.format = edit.format;
        if offset_changed && col.data.kind() == scicanvas_core::DataKind::DateTime {
            col.set_utc_offset(edit.format.utc_offset);
        }
        self.status = format!("已更新列 {} 的属性", edit.key);
        Ok(())
    }

    pub(crate) fn set_data_kind(&mut self, kind: scicanvas_core::DataKind) {
        let Some(id) = self.active_book() else { return };
        let keys = self.selected_keys(id);
        self.status = match self.project.book_mut(id).unwrap().convert_columns(&keys, kind) {
            Ok(()) => format!("已将 {} 设为{}；原始写法保留", keys.join(", "), kind.label()),
            Err(error) => error,
        };
    }

    pub(crate) fn clear_columns(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.selected_keys(bid);
        if sel.is_empty() {
            self.status = "请先单击列标题选中列".into();
            return;
        }
        if let Some(b) = self.project.book_mut(bid) {
            for c in &mut b.columns {
                if sel.contains(&c.key) {
                    c.data.clear();
                }
            }
        }
        self.status = format!("已清空 {} 列", sel.join(", "));
    }

    /// 类别顺序对话框：选中的第一列须是分类列。
    pub(crate) fn open_category_order_dialog(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(key) = self.selected_keys(bid).first().cloned() else {
            self.status = "请先单击分类列的列标题".into();
            return;
        };
        let levels = self
            .project
            .book(bid)
            .and_then(|b| b.column(&key))
            .and_then(|c| c.data.category_levels());
        match levels {
            Some(levels) => self.dialog = Some(Dialog::CategoryOrder { book: bid, key, levels }),
            None => self.status = format!("{key} 列不是分类列，可先“转换为分类列”"),
        }
    }

    pub(crate) fn move_columns(&mut self, to: scicanvas_core::ColumnMove) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.selected_keys(bid);
        if sel.is_empty() {
            self.status = "请先单击列标题选中要移动的列".into();
            return;
        }
        let moved = self.project.book_mut(bid).is_some_and(|b| b.move_columns(&sel, to));
        if moved {
            // 选中的列不变，单元格选区按列号记录，移动后清掉
            if let Some(s) = self.sheets.get_mut(&bid) {
                s.range = None;
            }
            self.status = format!("已移动 {} 列", sel.join(", "));
        } else {
            self.status = "已在边上，无法再移动".into();
        }
    }

    pub(crate) fn delete_columns(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let sel = self.selected_keys(bid);
        if sel.is_empty() {
            self.status = "请先单击列标题选中要删除的列".into();
            return;
        }
        self.project.delete_columns(bid, &sel);
        self.selection.remove(&bid);
        self.sheets.remove(&bid);
        self.status = format!("已删除 {} 列", sel.join(", "));
    }

    pub(super) fn track_headers(&mut self, id: u64, edits: &[(usize, usize, String, String)]) {
        if edits.is_empty() {
            return;
        }
        if !self.history.is_pending() {
            for (col, row, before, _) in edits {
                *header_field(&mut self.project.book_mut(id).unwrap().columns[*col], *row) = before.clone();
            }
            self.begin_edit("修改列属性");
            for (col, row, _, after) in edits {
                *header_field(&mut self.project.book_mut(id).unwrap().columns[*col], *row) = after.clone();
            }
        }
        self.history.changed();
    }

    /// 排序依据的列：选中的第一列，或当前单元格所在列。
    pub(crate) fn sort_column(&self, book_id: u64) -> Option<usize> {
        let book = self.project.book(book_id)?;
        if let Some(k) = self.selected_keys(book_id).first() {
            return book.column_index(k);
        }
        self.sheets
            .get(&book_id)
            .and_then(|s| s.range)
            .map(|r| r.a.c)
            .filter(|c| *c < book.columns.len())
    }

    /// 自定义排序对话框：主关键字默认为选中的第一列或当前单元格所在列。
    pub(crate) fn open_sort_dialog(&mut self) {
        let Some(bid) = self.active_book_id() else { return };
        let Some(book) = self.project.book(bid) else { return };
        let columns: Vec<(String, String)> = book
            .columns
            .iter()
            .map(|c| (c.key.clone(), format!("{}（{}）", c.key, c.display_name())))
            .collect();
        if columns.is_empty() {
            self.status = "工作表没有列".into();
            return;
        }
        let first = self.sort_column(bid).unwrap_or(0);
        self.dialog = Some(Dialog::Sort {
            book: bid,
            columns,
            keys: [(Some(first), true), (None, true), (None, true)],
            selected: self.selected_keys(bid),
            only_selected: false,
        });
    }

    pub(crate) fn sort_active(&mut self, ascending: bool) {
        let Some(bid) = self.active_book_id() else { return };
        let Some(col) = self.sort_column(bid) else { return };
        match self.project.sort_book(bid, col, ascending) {
            Ok(()) => {
                let key = self
                    .project
                    .book(bid)
                    .map(|b| b.columns[col].key.clone())
                    .unwrap_or_default();
                self.status = format!(
                    "已按 {} 列{}排序整张工作表，各行数据一起移动",
                    key,
                    if ascending { "升序" } else { "降序" }
                );
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_set_values_dialog(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(book) = self.project.book(id) else { return };
        let Some(target) = keys.first().and_then(|k| book.column(k)) else {
            self.status = "请先单击列标题选中要设置的列".into();
            return;
        };
        self.dialog = Some(Dialog::SetValues {
            target: format!("{}（{}）", target.key, target.display_name()),
            formula: target.formula.clone().unwrap_or_default(),
            keep: target.formula.is_some(),
            columns: book
                .columns
                .iter()
                .map(|c| (c.key.clone(), c.display_name().to_owned()))
                .collect(),
            error: None,
        });
    }

    /// 按公式设置第一个点选列的值；公式有误时留在对话框里显示错误。
    pub(super) fn set_column_values(&mut self, formula: &str, keep: bool) -> bool {
        let Some((id, keys)) = self.ttest_columns() else {
            return false;
        };
        let Some(target) = keys.first().cloned() else {
            return false;
        };
        match self.project.set_column_formula(id, &target, formula, keep) {
            Ok(n) => {
                self.status = if keep {
                    format!("已按公式设置列 {target}（{n} 行），数据变化时自动重算")
                } else {
                    format!("已按公式设置列 {target}（{n} 行）")
                };
                true
            }
            Err(e) => {
                if let Some(Dialog::SetValues { error, .. }) = &mut self.dialog {
                    *error = Some(e.clone());
                }
                self.status = e;
                false
            }
        }
    }

    pub(crate) fn open_fill_dialog(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(book) = self.project.book(id) else { return };
        let Some(target) = keys.first().and_then(|k| book.column(k)) else {
            self.status = "请先单击列标题选中要填充的列".into();
            return;
        };
        self.dialog = Some(Dialog::Fill {
            target: format!("{}（{}）", target.key, target.display_name()),
            kind: 0,
            a: 1.0,
            b: 1.0,
            rows: if book.nrows() > 0 { book.nrows() } else { 10 },
        });
    }

    pub(super) fn fill_sequence(&mut self, kind: scicanvas_core::stats::Sequence, rows: usize) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(target) = keys.first().cloned() else { return };
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        match self.project.fill_sequence(id, &target, kind, rows, seed) {
            Ok(()) => self.status = format!("已填充列 {target}（{rows} 行）"),
            Err(e) => self.status = e,
        }
    }

    /// 工作表：列标题、长名称 / 单位 / 注释三行和数据区。
    /// 数据区像 Excel 一样：单击选格、拖动选区域、拖动行号选整行、拖动列标题选多列，
    /// 双击或 F2 或直接打字进入编辑。
    pub(super) fn book_ui(&mut self, ui: &mut egui::Ui, id: u64) {
        let pal = self.pal;
        let active = self.active == Some(id);
        let (modifiers, pressed, down, dbl) = ui.input(|i| {
            (
                i.modifiers,
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.button_double_clicked(egui::PointerButton::Primary),
            )
        });
        let ptr = ui.input(|i| i.pointer.interact_pos());
        let focus_id = egui::Id::new(("sheet-focus", id));
        let edit_id = egui::Id::new(("sheet-edit", id));
        let selected = self.selected_keys(id);
        let mut st = self.sheets.remove(&id).unwrap_or_default();

        // 表格本身是一个可获得键盘焦点的部件：方向键、Tab、Esc 留给表格，不让 egui 拿去切换焦点
        ui.interact(ui.max_rect(), focus_id, egui::Sense::focusable_noninteractive());
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                focus_id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            )
        });
        if active
            && st.edit.is_none()
            && self.dialog.is_none()
            && self.ui.palette.is_none()
            && ui.memory(|m| m.has_focus(focus_id))
        {
            self.sheet_keys(ui.ctx(), id, &mut st);
        }

        #[derive(Clone, Copy)]
        enum Target {
            Cell(usize, usize),
            Row(usize),
            Col(usize),
        }

        let Some(book) = self.project.book_mut(id) else {
            self.sheets.insert(id, st);
            return;
        };
        let keys: Vec<String> = book.columns.iter().map(|c| c.key.clone()).collect();
        let sel_flags: Vec<bool> = keys.iter().map(|k| selected.contains(k)).collect();
        let ncols = book.columns.len();
        if ncols == 0 {
            st.range = None;
            st.edit = None;
        }
        if let Some(r) = st.range.as_mut() {
            r.clamp(ncols);
        }
        let range = st.range;
        let multi = range.filter(|r| r.is_multi());
        let nrows = (book.nrows() + 20)
            .max(40)
            .max(range.map_or(0, |r| r.r1().max(r.a.r) + 10))
            .max(st.edit.as_ref().map_or(0, |e| e.cell.r + 10));
        let drag = st.drag;
        let scroll_to = st.scroll_to.take();
        let mut edit = st.edit.take();
        let text_color = ui.visuals().text_color();
        let sel_stroke = egui::Stroke::new(2.0_f32, pal.accent);

        let mut clicked: Option<usize> = None;
        let mut action: Option<(usize, ColAction)> = None;
        let mut press: Option<Target> = None;
        let mut hover: Option<Target> = None;
        let mut dbl_cell: Option<(usize, usize)> = None;
        let mut right_cell: Option<(usize, usize)> = None;
        let mut menu_cmd: Option<&'static str> = None;
        let mut edit_end: Option<(bool, i64, i64)> = None;
        let mut autoscrolled = false;
        let mut metadata_edits = Vec::new();
        let dragging_cells = matches!(drag, Some(Drag::Cells | Drag::Rows)) && down;
        // 可见的表格区域（拖到上下边缘时自动滚动）
        let view = ui.clip_rect();
        let grid = egui::Stroke::new(1.0_f32, pal.meta_bg);

        ui.push_id(("sheet", id), |ui| {
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
                        .columns(TCol::initial(96.0).at_least(40.0).clip(true), ncols);
                    if let Some(r) = scroll_to {
                        table = table.scroll_to_row(r + META_ROWS.len(), None);
                    }
                    table
                        .header(24.0, |mut header| {
                            header.col(|ui| {
                                ui.label("");
                            });
                            for (ci, col) in book.columns.iter().enumerate() {
                                header.col(|ui| {
                                    let rect = ui.max_rect();
                                    // 带保存公式的列标 ƒ
                                    let computed = col.formula.is_some()
                                        || col
                                            .op
                                            .as_ref()
                                            .is_some_and(|op| op.mode != scicanvas_core::ops::RecalcMode::None);
                                    let mark = if computed { " ƒ" } else { "" };
                                    let text =
                                        RichText::new(format!("{}({}){mark}", col.key, col.role.label())).strong();
                                    let resp = ui
                                        .centered_and_justified(|ui| {
                                            ui.add(egui::SelectableLabel::new(sel_flags[ci], text))
                                        })
                                        .inner
                                        .on_hover_text("单击选中列，Ctrl / Shift 多选，拖动选多列，右键更多操作");
                                    if multi.is_some_and(|r| ci >= r.c0() && ci <= r.c1()) {
                                        ui.painter().line_segment(
                                            [
                                                rect.left_bottom() + egui::vec2(0.0, -1.0),
                                                rect.right_bottom() + egui::vec2(0.0, -1.0),
                                            ],
                                            sel_stroke,
                                        );
                                    }
                                    if pressed && ui.rect_contains_pointer(rect) {
                                        press = Some(Target::Col(ci));
                                    }
                                    if matches!(drag, Some(Drag::Cols(_))) && down && ui.rect_contains_pointer(rect) {
                                        hover = Some(Target::Col(ci));
                                    }
                                    if resp.clicked() {
                                        clicked = Some(ci);
                                    }
                                    // 双击列标题打开列属性（与 Origin 相同）
                                    if resp.double_clicked() {
                                        action = Some((ci, ColAction::Props));
                                    }
                                    resp.context_menu(|ui| {
                                        let mut pick = |ui: &mut egui::Ui, label: &str, a: ColAction| {
                                            if ui.button(label).clicked() {
                                                action = Some((ci, a));
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
                                            pick(
                                                ui,
                                                &format!("转换为{}列", kind.label()),
                                                ColAction::SetDataKind(kind),
                                            );
                                        }
                                        ui.separator();
                                        for t in PlotType::ALL {
                                            pick(ui, t.label(), ColAction::Plot(t));
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
                                    });
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(22.0, META_ROWS.len() + nrows, |mut row| {
                                let ri = row.index();
                                if ri < META_ROWS.len() {
                                    row.col(|ui| {
                                        ui.painter().rect_filled(ui.max_rect(), 0.0, pal.meta_bg);
                                        ui.centered_and_justified(|ui| {
                                            ui.label(META_ROWS[ri]);
                                        });
                                    });
                                    for (ci, col) in book.columns.iter_mut().enumerate() {
                                        row.col(|ui| {
                                            let bg = if sel_flags[ci] { pal.meta_sel_bg } else { pal.meta_bg };
                                            let rect = ui.max_rect();
                                            ui.painter().rect_filled(rect, 0.0, bg);
                                            ui.painter().line_segment(
                                                [rect.right_top(), rect.right_bottom()],
                                                egui::Stroke::new(1.0_f32, pal.window),
                                            );
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
                                                metadata_edits.push((ci, ri, before, field.clone()));
                                            }
                                            edit_group(&response, &mut self.history_group);
                                        });
                                    }
                                    return;
                                }

                                let r = ri - META_ROWS.len();
                                // 行号：拖动选整行
                                row.col(|ui| {
                                    let rect = ui.max_rect();
                                    let in_rows = multi.is_some_and(|g| r >= g.r0() && r <= g.r1());
                                    if in_rows {
                                        ui.painter().rect_filled(rect, 0.0, pal.sel_bg);
                                    }
                                    let resp = ui.interact(
                                        rect,
                                        egui::Id::new(("row-head", id, r)),
                                        egui::Sense::click_and_drag(),
                                    );
                                    if resp.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
                                    }
                                    ui.painter().with_clip_rect(rect).text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        (r + 1).to_string(),
                                        egui::FontId::monospace(12.0),
                                        pal.weak,
                                    );
                                    if pressed && ui.rect_contains_pointer(rect) {
                                        press = Some(Target::Row(r));
                                    }
                                    if dragging_cells && ui.rect_contains_pointer(rect) {
                                        hover = Some(Target::Row(r));
                                    }
                                    // 拖到表格上下边缘时自动滚动
                                    if dragging_cells && !autoscrolled {
                                        autoscrolled = true;
                                        if let Some(p) = ptr {
                                            if p.y > view.bottom() - 12.0 {
                                                ui.scroll_with_delta(egui::vec2(0.0, -16.0));
                                            } else if p.y < view.top() + 24.0 + 6.0 {
                                                ui.scroll_with_delta(egui::vec2(0.0, 16.0));
                                            }
                                        }
                                    }
                                });

                                for (ci, col) in book.columns.iter_mut().enumerate() {
                                    row.col(|ui| {
                                        let rect = ui.max_rect();
                                        let here = Cell { c: ci, r };
                                        let painter =
                                            ui.painter().with_clip_rect(rect.expand(1.0).intersect(ui.clip_rect()));

                                        if let Some(e) = edit.as_mut().filter(|e| e.cell == here) {
                                            painter.rect_filled(rect, 0.0, pal.window);
                                            let resp = ui.add(
                                                egui::TextEdit::singleline(&mut e.buf)
                                                    .id(edit_id)
                                                    .frame(false)
                                                    .margin(egui::vec2(5.0, 3.0))
                                                    .desired_width(f32::INFINITY)
                                                    .font(egui::TextStyle::Monospace),
                                            );
                                            if !e.focused {
                                                resp.request_focus();
                                                e.focused = true;
                                            }
                                            edit_group(&resp, &mut self.history_group);
                                            // Tab、上下方向键由表格处理（结束编辑并移动），不让 egui 切换焦点
                                            ui.memory_mut(|m| {
                                                m.set_focus_lock_filter(
                                                    edit_id,
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
                                                edit_end = Some((false, 0, 0));
                                            } else if tab {
                                                edit_end = Some((true, if shift { -1 } else { 1 }, 0));
                                            } else if up {
                                                edit_end = Some((true, 0, -1));
                                            } else if dn {
                                                edit_end = Some((true, 0, 1));
                                            } else if resp.lost_focus() {
                                                edit_end = Some((
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
                                            painter.rect_stroke(rect.shrink(1.0), 0.0, sel_stroke);
                                            return;
                                        }

                                        let in_range = multi.is_some_and(|g| g.contains(ci, r));
                                        if sel_flags[ci] || in_range {
                                            painter.rect_filled(rect, 0.0, pal.sel_bg);
                                        }
                                        painter.line_segment([rect.right_top(), rect.right_bottom()], grid);
                                        painter.line_segment([rect.left_bottom(), rect.right_bottom()], grid);
                                        let shown = (r < col.data.len()).then(|| col.display_at(r));
                                        if let Some(s) = shown.filter(|s| !s.is_empty()) {
                                            ui.painter().with_clip_rect(rect.intersect(ui.clip_rect())).text(
                                                rect.right_center() - egui::vec2(5.0, 0.0),
                                                egui::Align2::RIGHT_CENTER,
                                                s,
                                                egui::FontId::monospace(12.5),
                                                text_color,
                                            );
                                        }
                                        let resp = ui.interact(
                                            rect,
                                            egui::Id::new(("cell", id, ci, r)),
                                            egui::Sense::click_and_drag(),
                                        );
                                        if pressed && ui.rect_contains_pointer(rect) {
                                            press = Some(Target::Cell(ci, r));
                                        }
                                        if dbl && ui.rect_contains_pointer(rect) {
                                            dbl_cell = Some((ci, r));
                                        }
                                        if dragging_cells && ui.rect_contains_pointer(rect) {
                                            hover = Some(Target::Cell(ci, r));
                                        }
                                        if resp.secondary_clicked() {
                                            right_cell = Some((ci, r));
                                        }
                                        resp.context_menu(|ui| {
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
                                                    menu_cmd = Some(cid);
                                                    ui.close_menu();
                                                }
                                            }
                                        });

                                        // 选区外框和活动单元格
                                        if let Some(g) = range {
                                            if g.is_multi() && g.contains(ci, r) {
                                                let (lt, rt, lb, rb) = (
                                                    rect.left_top(),
                                                    rect.right_top(),
                                                    rect.left_bottom(),
                                                    rect.right_bottom(),
                                                );
                                                if ci == g.c0() {
                                                    painter.line_segment([lt, lb], sel_stroke);
                                                }
                                                if ci == g.c1() {
                                                    painter.line_segment([rt, rb], sel_stroke);
                                                }
                                                if r == g.r0() {
                                                    painter.line_segment([lt, rt], sel_stroke);
                                                }
                                                if r == g.r1() {
                                                    painter.line_segment([lb, rb], sel_stroke);
                                                }
                                            }
                                            if g.a == here && active {
                                                painter.rect_stroke(rect.shrink(1.0), 0.0, sel_stroke);
                                            }
                                        }
                                    });
                                }
                            });
                        });
                });
        });

        // ---- 处理这一帧的鼠标和编辑结果
        self.track_headers(id, &metadata_edits);
        if edit_end.is_some_and(|(ok, _, _)| ok) || (press.is_some() && edit.is_some()) {
            self.begin_edit("编辑单元格");
        }
        let book = self.project.book_mut(id).unwrap();
        let last_col = ncols.saturating_sub(1);
        let mut status: Option<String> = None;
        let mut clear_cols = false;
        let mut new_cols: Option<(usize, usize)> = None;
        let mut want_focus = false;
        let mut cell_changed = false;
        let mut conversion = None;
        let mut commit = |book: &mut scicanvas_core::Book, e: &Edit| {
            if let Some(col) = book.columns.get_mut(e.cell.c) {
                if !col.data.accepts(&e.buf) {
                    conversion = Some(Dialog::ConvertText {
                        book: id,
                        key: col.key.clone(),
                        row: e.cell.r,
                        value: e.buf.clone(),
                    });
                    return;
                }
                cell_changed = true;
                col.set(e.cell.r, e.buf.clone());
            }
        };

        if let Some((ok, dc, dr)) = edit_end {
            if let Some(e) = edit.take() {
                if ok {
                    commit(book, &e);
                }
                let c = (e.cell.c as i64 + dc).clamp(0, last_col as i64) as usize;
                let r = (e.cell.r as i64 + dr).max(0) as usize;
                st.range = Some(Range::single(c, r));
                st.scroll_to = Some(r);
                want_focus = true;
            }
        }

        if let Some(t) = press {
            // 正在编辑别的格子：先保存
            let same = matches!((t, &edit), (Target::Cell(c, r), Some(e)) if e.cell == Cell { c, r });
            if !same {
                if let Some(e) = edit.take() {
                    commit(book, &e);
                }
            }
            match t {
                Target::Cell(c, r) if !same => {
                    clear_cols = true;
                    match st.range.as_mut() {
                        Some(g) if modifiers.shift => g.b = Cell { c, r },
                        _ => st.range = Some(Range::single(c, r)),
                    }
                    st.drag = Some(Drag::Cells);
                    st.moved = false;
                    want_focus = true;
                }
                Target::Row(r) if ncols > 0 => {
                    clear_cols = true;
                    let a = match st.range {
                        Some(g) if modifiers.shift => Cell { c: 0, r: g.a.r },
                        _ => Cell { c: 0, r },
                    };
                    st.range = Some(Range {
                        a,
                        b: Cell { c: last_col, r },
                    });
                    st.drag = Some(Drag::Rows);
                    st.moved = true;
                    want_focus = true;
                }
                Target::Col(c) => {
                    st.drag = Some(Drag::Cols(c));
                    st.moved = false;
                }
                _ => {}
            }
        }

        if down {
            match (st.drag, hover) {
                (Some(Drag::Cells), Some(Target::Cell(c, r))) => {
                    if let Some(g) = st.range.as_mut() {
                        if g.b != (Cell { c, r }) {
                            g.b = Cell { c, r };
                            st.moved = true;
                        }
                    }
                }
                (Some(Drag::Cells), Some(Target::Row(r))) => {
                    if let Some(g) = st.range.as_mut() {
                        g.b.r = r;
                        st.moved = true;
                    }
                }
                (Some(Drag::Rows), Some(Target::Cell(_, r) | Target::Row(r))) => {
                    if let Some(g) = st.range.as_mut() {
                        g.b = Cell { c: last_col, r };
                    }
                }
                (Some(Drag::Cols(start)), Some(Target::Col(c))) if c != start || st.moved => {
                    st.moved = true;
                    new_cols = Some((start.min(c), start.max(c)));
                }
                _ => {}
            }
        } else if let Some(d) = st.drag.take() {
            if st.moved && !matches!(d, Drag::Cols(_)) {
                if let Some(g) = st.range.filter(|g| g.is_multi()) {
                    status = Some(range_status(book, &g));
                }
            }
        }

        if let Some((c, r)) = dbl_cell {
            let buf = book
                .columns
                .get(c)
                .and_then(|col| col.data.get(r))
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default();
            edit = Some(Edit {
                cell: Cell { c, r },
                buf,
                focused: false,
            });
            st.range = Some(Range::single(c, r));
            st.drag = None;
        }
        if let Some((c, r)) = right_cell {
            if !st.range.is_some_and(|g| g.contains(c, r)) {
                st.range = Some(Range::single(c, r));
                clear_cols = true;
            }
        }
        st.edit = edit;
        if let Some(dialog) = conversion {
            self.dialog = Some(dialog);
        }
        if cell_changed {
            self.history.changed();
        }
        self.sheets.insert(id, st);

        if want_focus || dbl_cell.is_some() || right_cell.is_some() || press.is_some() {
            self.active = Some(id);
        }
        if want_focus {
            ui.memory_mut(|m| m.request_focus(focus_id));
        }
        if clear_cols {
            self.selection.remove(&id);
        }
        if let Some((a, b)) = new_cols {
            self.selection.insert(id, keys[a..=b].to_vec());
            if let Some(s) = self.sheets.get_mut(&id) {
                s.range = None;
            }
            self.status = format!("已选中 {} 列", keys[a..=b].join(", "));
        }
        if let Some(s) = status {
            self.status = s;
        }

        // 单击列标题：选择列
        if let Some(ci) = clicked.filter(|_| new_cols.is_none()) {
            let key = keys[ci].clone();
            let sel = self.selection.entry(id).or_default();
            if modifiers.command {
                if let Some(p) = sel.iter().position(|k| *k == key) {
                    sel.remove(p);
                } else {
                    sel.push(key);
                }
            } else if modifiers.shift && !sel.is_empty() {
                let last = sel.last().cloned().unwrap_or_default();
                let a = keys.iter().position(|k| *k == last).unwrap_or(ci);
                for k in &keys[a.min(ci)..=a.max(ci)] {
                    if !sel.contains(k) {
                        sel.push(k.clone());
                    }
                }
            } else {
                sel.clear();
                sel.push(key);
            }
            self.active = Some(id);
            self.status = format!("已选中 {} 列", sel.join(", "));
            if let Some(s) = self.sheets.get_mut(&id) {
                s.range = None;
            }
        }

        // 列标题右键菜单
        if let Some((ci, act)) = action {
            let key = keys[ci].clone();
            let sel = self.selection.entry(id).or_default();
            if !sel.contains(&key) {
                sel.clear();
                sel.push(key);
            }
            self.active = Some(id);
            let command = match act {
                ColAction::Props => "col.props",
                ColAction::SetDataKind(scicanvas_core::DataKind::Num) => "col.type.num",
                ColAction::SetDataKind(scicanvas_core::DataKind::Text) => "col.type.text",
                ColAction::SetDataKind(scicanvas_core::DataKind::DateTime) => "col.type.datetime",
                ColAction::SetDataKind(scicanvas_core::DataKind::Category) => "col.type.category",
                ColAction::SetRole(Role::X) => "col.setX",
                ColAction::SetRole(Role::Y) => "col.setY",
                ColAction::SetRole(Role::XError) => "col.setXError",
                ColAction::SetRole(Role::YError) => "col.setYError",
                ColAction::SetRole(Role::Ignore) => "col.setIgnore",
                ColAction::SetRole(Role::Z) => "col.setZ",
                ColAction::SetRole(Role::Label) => "col.setLabel",
                ColAction::InsertRight => "col.insert",
                ColAction::CategoryOrder => "col.categories",
                ColAction::Move(scicanvas_core::ColumnMove::Left) => "col.left",
                ColAction::Move(scicanvas_core::ColumnMove::Right) => "col.right",
                ColAction::Move(scicanvas_core::ColumnMove::First) => "col.first",
                ColAction::Move(scicanvas_core::ColumnMove::Last) => "col.last",
                ColAction::Clear => "col.clear",
                ColAction::Delete => "col.delete",
                ColAction::Plot(PlotType::Line) => "plot.line",
                ColAction::Plot(PlotType::Scatter) => "plot.scatter",
                ColAction::Plot(PlotType::LineSymbol) => "plot.linesym",
                ColAction::Plot(PlotType::Bar) => "plot.bar",
                ColAction::Plot(PlotType::StackedBar) => "plot.stacked_bar",
                ColAction::Plot(PlotType::PercentStackedBar) => "plot.percent_stacked_bar",
                ColAction::Plot(PlotType::FloatingBar) => "plot.floating_bar",
                ColAction::Plot(PlotType::HorizontalBar) => "plot.horizontal_bar",
                ColAction::Plot(PlotType::Area) => "plot.area",
                ColAction::Plot(PlotType::Spline) => "plot.spline",
                ColAction::Plot(PlotType::SplineSymbol) => "plot.spline_symbol",
                ColAction::Plot(PlotType::VerticalDrop) => "plot.vertical_drop",
                ColAction::Plot(PlotType::StepHorizontal) => "plot.step_horizontal",
                ColAction::Plot(PlotType::StepVertical) => "plot.step_vertical",
            };
            self.run_command(command);
        }

        // 单元格右键菜单
        if let Some(cmd) = menu_cmd {
            self.active = Some(id);
            self.run_command(cmd);
        }
    }

    /// 表格有键盘焦点时的按键：方向键移动（Shift 扩展选区）、Tab / 回车移动、
    /// Delete 清除、Ctrl+C / Ctrl+X / Ctrl+V、Ctrl+A 全选、Esc 取消选区、F2 或直接打字开始编辑。
    pub(super) fn sheet_keys(&mut self, ctx: &egui::Context, id: u64, st: &mut SheetState) {
        use egui::{Event, Key};
        // 项目管理器先于子窗口绘制：最后一次点击落在项目管理器时，按键不属于工作表。
        if self.explorer_focus {
            return;
        }
        let Some(book) = self.project.book(id) else { return };
        if book.columns.is_empty() {
            return;
        }
        let Some(mut g) = st.range else { return };
        let events: Vec<Event> = ctx.input_mut(|i| {
            let mut mine = Vec::new();
            i.events.retain(|e| {
                let take = match e {
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        matches!(
                            key,
                            Key::ArrowUp
                                | Key::ArrowDown
                                | Key::ArrowLeft
                                | Key::ArrowRight
                                | Key::Enter
                                | Key::Tab
                                | Key::F2
                                | Key::Delete
                                | Key::Backspace
                                | Key::Escape
                                | Key::Home
                                | Key::End
                                | Key::PageUp
                                | Key::PageDown
                        ) || (modifiers.command && *key == Key::A)
                    }
                    Event::Copy | Event::Cut | Event::Paste(_) => true,
                    Event::Text(t) => !t.chars().all(char::is_control),
                    _ => false,
                };
                if take {
                    mine.push(e.clone());
                }
                !take
            });
            mine
        });
        if events.is_empty() {
            return;
        }
        let writes = events.iter().any(|e| {
            matches!(
                e,
                Event::Cut
                    | Event::Paste(_)
                    | Event::Key {
                        key: Key::Delete | Key::Backspace,
                        ..
                    }
            )
        });
        if writes {
            self.begin_edit("编辑工作表");
        }
        let book = self.project.book_mut(id).unwrap();

        let mut status: Option<String> = None;
        let mut start_edit: Option<String> = None;
        let mut copy: Option<String> = None;
        for e in events {
            let last_col = book.columns.len() - 1;
            match e {
                Event::Key { key, modifiers, .. } => {
                    let (dc, dr): (i64, i64) = match key {
                        Key::ArrowUp => (0, -1),
                        Key::ArrowDown => (0, 1),
                        Key::ArrowLeft => (-1, 0),
                        Key::ArrowRight => (1, 0),
                        Key::PageUp => (0, -20),
                        Key::PageDown => (0, 20),
                        Key::Enter => (0, if modifiers.shift { -1 } else { 1 }),
                        Key::Tab => (if modifiers.shift { -1 } else { 1 }, 0),
                        _ => (0, 0),
                    };
                    let step = |cell: Cell| Cell {
                        c: (cell.c as i64 + dc).clamp(0, last_col as i64) as usize,
                        r: (cell.r as i64 + dr).max(0) as usize,
                    };
                    match key {
                        Key::A => {
                            g = Range {
                                a: Cell { c: 0, r: 0 },
                                b: Cell {
                                    c: last_col,
                                    r: book.nrows().saturating_sub(1),
                                },
                            };
                            status = Some(range_status(book, &g));
                        }
                        Key::F2 => {
                            start_edit = Some(
                                book.columns[g.a.c]
                                    .data
                                    .get(g.a.r)
                                    .map(std::borrow::Cow::into_owned)
                                    .unwrap_or_default(),
                            );
                        }
                        Key::Delete | Key::Backspace => {
                            book.clear_range(g.c0(), g.c1(), g.r0(), g.r1());
                            status = Some(format!("已清除 {} 行 × {} 列", g.rows(), g.cols()));
                        }
                        Key::Escape => g = Range { a: g.a, b: g.a },
                        Key::Home => {
                            let r = if modifiers.command { 0 } else { g.a.r };
                            g = Range::single(0, r);
                            st.scroll_to = Some(r);
                        }
                        Key::End => {
                            let r = if modifiers.command {
                                book.nrows().saturating_sub(1)
                            } else {
                                g.a.r
                            };
                            g = Range::single(last_col, r);
                            st.scroll_to = Some(r);
                        }
                        _ if dc != 0 || dr != 0 => {
                            let extend = modifiers.shift && !matches!(key, Key::Enter | Key::Tab);
                            if extend {
                                g.b = step(g.b);
                                st.scroll_to = Some(g.b.r);
                            } else {
                                let n = step(g.a);
                                g = Range::single(n.c, n.r);
                                st.scroll_to = Some(n.r);
                            }
                        }
                        _ => {}
                    }
                }
                Event::Copy | Event::Cut => {
                    copy = Some(book.range_tsv(g.c0(), g.c1(), g.r0(), g.r1()));
                    if matches!(e, Event::Cut) {
                        book.clear_range(g.c0(), g.c1(), g.r0(), g.r1());
                        status = Some(format!("已剪切 {} 行 × {} 列", g.rows(), g.cols()));
                    } else {
                        status = Some(format!("已复制 {} 行 × {} 列，可直接粘贴到 Excel", g.rows(), g.cols()));
                    }
                }
                Event::Paste(text) => {
                    let conflicts: Vec<String> = if csv::is_block_paste(&text) {
                        csv::paste_conflicts(book, g.c0(), g.r0(), &text)
                    } else {
                        book.columns[g.c0()..=g.c1()]
                            .iter()
                            .filter(|c| !c.data.accepts(text.trim()))
                            .map(|c| c.key.clone())
                            .collect()
                    };
                    if !conflicts.is_empty() {
                        status = Some(format!(
                            "列 {} 与粘贴内容类型不符，数据保持原样。请从列头右键转换为文本列后再粘贴。",
                            conflicts.join(", ")
                        ));
                        continue;
                    }
                    if csv::is_block_paste(&text) {
                        let (c0, r0) = (g.c0(), g.r0());
                        let (nr, nc) = csv::paste_into(book, c0, r0, &text);
                        g = Range {
                            a: Cell { c: c0, r: r0 },
                            b: Cell {
                                c: c0 + nc.max(1) - 1,
                                r: r0 + nr.max(1) - 1,
                            },
                        };
                        status = Some(format!("已粘贴 {} 行 × {} 列", nr, nc));
                    } else {
                        let v = text.trim();
                        book.fill_range(g.c0(), g.c1(), g.r0(), g.r1(), v);
                        status = Some(if g.is_multi() {
                            format!("已把 {} 填入 {} 个单元格", v, g.rows() * g.cols())
                        } else {
                            "已粘贴".to_string()
                        });
                    }
                }
                Event::Text(t) => {
                    start_edit.get_or_insert_with(String::new).push_str(&t);
                }
                _ => {}
            }
        }
        g.clamp(book.columns.len());
        st.range = Some(g);
        if let Some(buf) = start_edit {
            st.edit = Some(Edit {
                cell: g.a,
                buf,
                focused: false,
            });
            st.range = Some(Range::single(g.a.c, g.a.r));
        }
        if let Some(t) = copy {
            ctx.copy_text(t);
        }
        if let Some(s) = status {
            self.status = s;
        }
        if writes {
            self.history.changed();
        }
    }
}
