//! Worksheet rendering, selection, keyboard editing and column operations.

mod table;

use super::*;
use table::SheetFrame;

impl SciApp {
    pub(crate) fn open_sheet_transform(&mut self, kind: dialogs::TransformKind) {
        let Some(book_id) = self.active_book_id() else { return };
        let Some(book) = self.project.book(book_id) else { return };
        if book.columns.is_empty() {
            self.status = "工作表没有列".into();
            return;
        }
        let selected = self.selected_keys(book_id);
        let columns = book
            .columns
            .iter()
            .map(|c| {
                (
                    c.key.clone(),
                    format!("{}（{}）", c.key, c.display_name()),
                    selected.contains(&c.key),
                )
            })
            .collect();
        let value = self.sort_column(book_id).unwrap_or(0);
        let category = book
            .columns
            .iter()
            .position(|c| c.data.kind() == scicanvas_core::DataKind::Category)
            .unwrap_or(0);
        self.dialog = Some(Dialog::SheetTransform(Box::new(dialogs::SheetTransform {
            book: book_id,
            kind,
            columns,
            value,
            category,
            step: 2,
            average: false,
        })));
    }

    pub(crate) fn freeze_selected_columns(&mut self) {
        let Some(id) = self.active_book_id() else { return };
        let Some(book) = self.project.book(id) else { return };
        let selected = self.selected_keys(id);
        let index = selected
            .iter()
            .filter_map(|key| book.column_index(key))
            .max()
            .or_else(|| self.sort_column(id))
            .unwrap_or(0);
        let count = (index + 1).min(book.columns.len());
        let state = self.sheets.entry(id).or_default();
        if state.viewport_width > 0.0 && 58.0 + 96.0 * count as f32 + 70.0 > state.viewport_width {
            self.status = "窗口宽度不足，请选择更少的列或放大窗口".into();
            return;
        }
        state.frozen_columns = count;
        self.status = format!("已冻结前 {count} 列和行号");
    }

    pub(crate) fn unfreeze_columns(&mut self) {
        let Some(id) = self.active_book_id() else { return };
        self.sheets.entry(id).or_default().frozen_columns = 0;
        self.status = "已取消冻结列".into();
    }

    pub(crate) fn save_workbook_template(&mut self) {
        let Some(id) = self.active_book() else { return };
        if !self.flush_sheet_edits() {
            return;
        }
        let Some(book) = self.project.book(id) else { return };
        let template = scicanvas_core::project_file::workbook_template_from_book(book);
        let Some(path) = self
            .file_dialog()
            .set_title("保存工作簿模板")
            .add_filter("SciCanvas 工作簿模板", &["scbt"])
            .set_file_name(format!("{}.scbt", book.name))
            .save_file()
        else {
            return;
        };
        self.status = match scicanvas_core::project_file::save_workbook_template(&path, &template) {
            Ok(()) => format!("已保存工作簿模板 {}", path.display()),
            Err(error) => format!("保存工作簿模板失败：{error}"),
        };
    }

    pub(crate) fn open_workbook_template(&mut self) {
        let Some(path) = self
            .file_dialog()
            .set_title("从工作簿模板新建")
            .add_filter("SciCanvas 工作簿模板", &["scbt"])
            .pick_file()
        else {
            return;
        };
        match scicanvas_core::project_file::load_workbook_template(&path) {
            Ok(template) => {
                self.create_book_from_template(template);
            }
            Err(error) => self.status = format!("无法打开工作簿模板：{error}"),
        }
    }

    pub(crate) fn create_book_from_template(
        &mut self,
        template: scicanvas_core::project_file::WorkbookTemplate,
    ) -> Option<u64> {
        self.edit_with(
            "从模板新建工作簿",
            |project| scicanvas_core::project_file::create_book_from_template(project, &template).map(Some),
            |app, id| {
                app.show_item(id);
                app.status = "已从模板新建工作簿".into();
                id
            },
        )
    }

    pub(crate) fn transpose_book(&mut self) {
        let Some(id) = self.active_book() else { return };
        if !self.flush_sheet_edits() {
            return;
        }
        self.edit_with(
            "转置工作表",
            |project| project.transpose_book(id).map(Some),
            |app, output| {
                app.show_item(output);
                app.status = "已创建转置工作簿".into();
            },
        );
    }

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
            self.pending_copy = Some(b.visible_range_tsv(r.c0(), r.c1(), r.r0(), r.r1()));
            let rows = if b.filter.is_some() {
                b.visible_range_rows(r.r0(), r.r1()).len()
            } else {
                r.rows()
            };
            self.status = format!("已复制 {} 行 × {} 列，可直接粘贴到 Excel", rows, r.cols());
        }
    }

    pub(crate) fn clear_range(&mut self) {
        let Some(bid) = self.active_book() else { return };
        let Some(r) = self.sheets.get(&bid).and_then(|s| s.range) else {
            self.status = "请先选择单元格".into();
            return;
        };
        if let Some(b) = self.project.book_mut(bid) {
            let rows = b.clear_visible_range(r.c0(), r.c1(), r.r0(), r.r1());
            self.status = format!("已清除 {} 行 × {} 列", rows, r.cols());
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
            book: id,
            before: dialogs::ColumnEdit::from(col),
            edit: dialogs::ColumnEdit::from(col),
            sample,
            error: None,
        });
    }

    /// 应用列属性；类型转换失败时不改动任何内容，错误留在对话框里。
    pub(super) fn apply_column_props(&mut self, book: u64, edit: dialogs::ColumnEdit) -> Result<(), String> {
        self.edit_grouped_with(
            "列属性",
            self.history_group,
            |project| edit.apply(project.book_mut(book).ok_or("找不到工作簿")?),
            |app, ()| {
                app.status = format!("已更新列 {} 的属性", edit.key);
            },
        )
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
            Some(levels) => {
                self.dialog = Some(Dialog::CategoryOrder {
                    book: bid,
                    key,
                    levels,
                    error: None,
                })
            }
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
        self.status = format!("已删除 {} 列", sel.join(", "));
        self.edit(&format!("删除列 {}", sel.join(", ")), |project| {
            project.delete_columns(bid, &sel);
            Ok(Some(()))
        });
        self.selection.remove(&bid);
        self.sheets.remove(&bid);
    }

    pub(super) fn track_headers(&mut self, id: u64, edits: &[(usize, usize, String, String)]) {
        if edits.is_empty() {
            return;
        }
        for (col, row, before, _) in edits {
            *header_field(&mut self.project.book_mut(id).unwrap().columns[*col], *row) = before.clone();
        }
        self.edit_grouped("修改列属性", self.history_group, |project| {
            for (col, row, _, after) in edits {
                *header_field(&mut project.book_mut(id).unwrap().columns[*col], *row) = after.clone();
            }
        });
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
            error: None,
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
                    "已按 {} 列{}排序来源数据，保留 {} 个独立采样结果列的原顺序",
                    key,
                    if ascending { "升序" } else { "降序" },
                    self.project
                        .book(bid)
                        .map_or(0, |b| b.columns.iter().filter(|c| c.independent_rows).count())
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
        let result = self.edit_with(
            "设置列值",
            |project| project.set_column_formula(id, &target, formula, keep).map(Some),
            |app, n| {
                app.status = if keep {
                    format!("已按公式设置列 {target}（{n} 行），数据变化时自动重算")
                } else {
                    format!("已按公式设置列 {target}（{n} 行）")
                };
            },
        );
        if result.is_some() {
            true
        } else {
            if let Some(Dialog::SetValues { error, .. }) = &mut self.dialog {
                *error = Some(self.status.clone());
            }
            false
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
        self.edit_with(
            "填充数据序列",
            |project| project.fill_sequence(id, &target, kind, rows, seed).map(Some),
            |app, ()| {
                app.status = format!("已填充列 {target}（{rows} 行）");
            },
        );
    }

    /// 工作表：列标题、长名称 / 单位 / 注释三行和数据区。
    /// 数据区像 Excel 一样：单击选格、拖动选区域、拖动行号选整行、拖动列标题选多列，
    /// 双击或 F2 或直接打字进入编辑。
    #[allow(clippy::too_many_lines)]
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
        let visible_rows = self.project.book(id).and_then(|book| st.filtered_rows(book));
        if let Some(rows) = &visible_rows {
            if st.edit.is_none() && st.range.is_some_and(|range| rows.binary_search(&range.a.r).is_err()) {
                st.range = rows.first().map(|&row| Range::single(st.range.unwrap().a.c, row));
            }
        }
        // 有筛选时在表格上方说明显示了多少行，以及分析是否只用可见行
        let mut visible_toggle = None;
        if let (Some(rows), Some(book)) = (&visible_rows, self.project.book(id)) {
            let mut use_visible = book.uses_visible_rows();
            let total = book.nrows();
            let enabled = !self.modal_active();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("已筛选：显示 {} / {} 行", rows.len(), total)).color(pal.accent));
                if ui
                    .add_enabled(enabled, egui::Checkbox::new(&mut use_visible, "绘图和分析只使用可见行"))
                    .changed()
                {
                    visible_toggle = Some(use_visible);
                }
            });
        }

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
        let has_paste = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))));
        let numeric_editor = st.edit.as_ref().is_some_and(|edit| {
            self.project
                .book(id)
                .and_then(|book| book.columns.get(edit.cell.c))
                .is_some_and(|column| column.data.kind() == scicanvas_core::DataKind::Num)
        });
        let paste_in_editor = st.edit.is_some()
            && ui.memory(|m| m.has_focus(edit_id))
            && ui.input(|i| {
                i.events.iter().any(
                    |e| matches!(e, egui::Event::Paste(text) if numeric_editor || text.contains(['\t', '\r', '\n'])),
                )
            });
        if active
            && (self.dialog.is_none() || has_paste)
            && self.ui.palette.is_none()
            && ((st.edit.is_none() && ui.memory(|m| m.has_focus(focus_id))) || paste_in_editor)
        {
            self.sheet_keys(ui.ctx(), id, &mut st);
        }

        let Some(book) = self.project.book_mut(id) else {
            self.sheets.insert(id, st);
            return;
        };
        let keys: Vec<String> = book.columns.iter().map(|c| c.key.clone()).collect();
        let graphs: Vec<(u64, String)> = self.project.graphs.iter().map(|g| (g.id, g.name.clone())).collect();
        let preview = self
            .book_preview
            .as_ref()
            .and_then(|p| p.result.as_ref().ok())
            .filter(|b| b.id == id);
        let frozen = self.modal_active();
        let Some(book) = self.project.book_mut(id) else {
            self.sheets.insert(id, st);
            return;
        };
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
        let nrows = visible_rows.as_ref().map_or_else(
            || {
                (book.nrows() + 20)
                    .max(40)
                    .max(range.map_or(0, |r| r.r1().max(r.a.r) + 10))
                    .max(st.edit.as_ref().map_or(0, |e| e.cell.r + 10))
            },
            |rows| rows.len(),
        );
        let drag = st.drag;
        let scroll_to = st.scroll_to.take().map(|row| {
            visible_rows.as_ref().map_or(row, |rows| {
                rows.partition_point(|&value| value < row)
                    .min(rows.len().saturating_sub(1))
            })
        });
        let edit = st.edit.take();
        let text_color = ui.visuals().text_color();
        let sel_stroke = egui::Stroke::new(2.0_f32, pal.accent);

        let dragging_cells = matches!(drag, Some(Drag::Cells | Drag::Rows)) && down;
        // 可见的表格区域（拖到上下边缘时自动滚动）
        let view = ui.clip_rect();
        st.viewport_width = ui.available_width();
        let grid = egui::Stroke::new(1.0_f32, pal.meta_bg);
        let mut frame = SheetFrame {
            frozen,
            id,
            pal,
            active,
            modifiers,
            pressed,
            down,
            dbl,
            ptr,
            focus_id,
            edit_id,
            graphs,
            sel_flags,
            ncols,
            nrows,
            frozen_columns: st.frozen_columns.min(ncols),
            frozen_scroll_y: st.frozen_scroll_y,
            visible_rows,
            range,
            multi,
            drag,
            scroll_to,
            edit,
            text_color,
            sel_stroke,
            dragging_cells,
            view,
            grid,
            clicked: None,
            action: None,
            press: None,
            hover: None,
            dbl_cell: None,
            right_cell: None,
            menu_cmd: None,
            edit_end: None,
            autoscrolled: false,
            metadata_edits: Vec::new(),
        };
        frame.draw(ui, book, preview, &mut self.history_group);
        st.frozen_scroll_y = frame.frozen_scroll_y;
        self.finish_sheet_frame(ui, id, st, frame, &keys);
        if let Some(visible_only) = visible_toggle {
            self.set_filter_visible_only(id, visible_only);
        }
    }

    fn finish_sheet_frame(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        mut st: SheetState,
        frame: SheetFrame,
        keys: &[String],
    ) {
        let SheetFrame {
            metadata_edits,
            edit_end,
            press,
            mut edit,
            hover,
            down,
            modifiers,
            focus_id,
            ncols,
            dbl_cell,
            right_cell,
            clicked,
            action,
            menu_cmd,
            ..
        } = frame;
        // ---- 处理这一帧的鼠标和编辑结果
        self.track_headers(id, &metadata_edits);
        let book = self.project.book(id).unwrap();
        let last_col = ncols.saturating_sub(1);
        let mut clear_cols = false;
        let mut want_focus = false;
        let mut cell_write = None;
        let mut conversion = None;
        let mut commit = |book: &scicanvas_core::Book, e: &Edit| {
            if let Some(col) = book.columns.get(e.cell.c) {
                if !col.data.accepts(&e.buf) {
                    conversion = Some(Dialog::ConvertText {
                        book: id,
                        key: col.key.clone(),
                        row: e.cell.r,
                        value: e.buf.clone(),
                    });
                    return;
                }
                cell_write = Some(e.clone());
            }
        };

        if let Some((ok, dc, dr)) = edit_end {
            if let Some(e) = edit.take() {
                if ok {
                    commit(book, &e);
                }
                let c = (e.cell.c as i64 + dc).clamp(0, last_col as i64) as usize;
                let r = st.step_row(e.cell.r, dr);
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
                    want_focus = true;
                    st.drag = Some(Drag::Cols(c));
                    st.moved = false;
                }
                _ => {}
            }
        }

        if let Some(e) = cell_write {
            self.edit_grouped("编辑单元格", self.history_group, |project| {
                project.book_mut(id).unwrap().columns[e.cell.c].set(e.cell.r, e.buf);
            });
        }
        let book = self.project.book(id).unwrap();
        let (new_cols, status) = drag_selection(&mut st, down, hover, last_col, book);
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

        self.select_sheet_header(id, keys, modifiers, clicked, new_cols);
        self.sheet_commands(id, keys, action, menu_cmd);
    }

    fn select_sheet_header(
        &mut self,
        id: u64,
        keys: &[String],
        modifiers: egui::Modifiers,
        clicked: Option<usize>,
        new_cols: Option<(usize, usize)>,
    ) {
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
    }

    fn sheet_commands(
        &mut self,
        id: u64,
        keys: &[String],
        action: Option<(usize, ColAction)>,
        menu_cmd: Option<&'static str>,
    ) {
        // 列标题右键菜单
        if let Some((ci, act)) = action {
            let key = keys[ci].clone();
            let sel = self.selection.entry(id).or_default();
            if !sel.contains(&key) {
                sel.clear();
                sel.push(key.clone());
            }
            self.active = Some(id);
            if matches!(act, ColAction::Filter | ColAction::ClearFilter) {
                if matches!(act, ColAction::Filter) {
                    self.open_filter_dialog(id, &key);
                } else {
                    let visible_only = self.project.book(id).is_some_and(|book| book.uses_visible_rows());
                    self.apply_filter(id, key, None, visible_only);
                }
                return;
            }
            if let ColAction::AddToGraph(gid) = act {
                self.add_to_graph(id, gid);
                return;
            }
            let command = match act {
                ColAction::Props => "col.props",
                ColAction::Filter | ColAction::ClearFilter => unreachable!(),
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
                ColAction::AddToGraph(_) => unreachable!(),
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
        let visible_rows = st.cached_rows();
        if visible_rows.as_ref().is_some_and(|rows| rows.is_empty()) {
            return;
        }
        let has_paste = ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::Paste(_))));
        let anchor = st
            .edit
            .as_ref()
            .filter(|_| has_paste)
            .map(|e| Range::single(e.cell.c, e.cell.r))
            .or(st.range)
            .or_else(|| {
                has_paste
                    .then(|| {
                        self.selected_keys(id)
                            .iter()
                            .filter_map(|key| book.column_index(key))
                            .min()
                            .map(|c| {
                                Range::single(
                                    c,
                                    visible_rows
                                        .as_ref()
                                        .and_then(|rows| rows.first())
                                        .copied()
                                        .unwrap_or(0),
                                )
                            })
                    })
                    .flatten()
            });
        let Some(g) = anchor else { return };
        let events = sheet_events(ctx);
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
        let apply = |project: &mut Project| -> Result<Option<_>, String> {
            let book = project.book_mut(id).unwrap();
            apply_sheet_events(book, st, g, events).map(|result| Some((result.1, result.2, result.3)))
        };
        let show = |app: &mut Self, (status, copy, pasted): (Option<String>, Option<String>, bool)| {
            if pasted {
                if matches!(app.dialog, Some(Dialog::ConvertText { .. })) {
                    app.dialog = None;
                }
                app.selection.remove(&id);
                ctx.memory_mut(|m| m.request_focus(egui::Id::new(("sheet-focus", id))));
            }
            if let Some(t) = copy {
                ctx.copy_text(t);
            }
            if let Some(s) = status {
                app.status = s;
            }
        };
        if writes {
            self.edit_with("编辑工作表", apply, show);
        } else if let Some(result) = apply(&mut self.project).unwrap() {
            show(self, result);
        }
    }
}

fn apply_sheet_events(
    book: &mut scicanvas_core::Book,
    st: &mut SheetState,
    mut g: Range,
    events: Vec<egui::Event>,
) -> Result<(Range, Option<String>, Option<String>, bool), String> {
    use egui::{Event, Key};
    let visible_rows = st.cached_rows();
    let mut pasted = false;
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
                    r: crate::sheet::step_visible_row(visible_rows.as_deref(), cell.r, dr),
                };
                match key {
                    Key::A => {
                        g = Range {
                            a: Cell {
                                c: 0,
                                r: visible_rows
                                    .as_ref()
                                    .and_then(|rows| rows.first())
                                    .copied()
                                    .unwrap_or(0),
                            },
                            b: Cell {
                                c: last_col,
                                r: visible_rows
                                    .as_ref()
                                    .and_then(|rows| rows.last())
                                    .copied()
                                    .unwrap_or_else(|| book.nrows().saturating_sub(1)),
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
                        let rows = book.clear_visible_range(g.c0(), g.c1(), g.r0(), g.r1());
                        status = Some(format!("已清除 {} 行 × {} 列", rows, g.cols()));
                    }
                    Key::Escape => g = Range { a: g.a, b: g.a },
                    Key::Home => {
                        let r = if modifiers.command {
                            visible_rows
                                .as_ref()
                                .and_then(|rows| rows.first())
                                .copied()
                                .unwrap_or(0)
                        } else {
                            g.a.r
                        };
                        g = Range::single(0, r);
                        st.scroll_to = Some(r);
                    }
                    Key::End => {
                        let r = if modifiers.command {
                            visible_rows
                                .as_ref()
                                .and_then(|rows| rows.last())
                                .copied()
                                .unwrap_or_else(|| book.nrows().saturating_sub(1))
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
                copy = Some(book.visible_range_tsv(g.c0(), g.c1(), g.r0(), g.r1()));
                let rows = if book.filter.is_some() {
                    book.visible_range_rows(g.r0(), g.r1()).len()
                } else {
                    g.rows()
                };
                if matches!(e, Event::Cut) {
                    book.clear_visible_range(g.c0(), g.c1(), g.r0(), g.r1());
                    status = Some(format!("已剪切 {} 行 × {} 列", rows, g.cols()));
                } else {
                    status = Some(format!("已复制 {} 行 × {} 列，可直接粘贴到 Excel", rows, g.cols()));
                }
            }
            Event::Paste(text) => {
                let (range, message, changed) = paste_sheet_cells(book, g, &text, visible_rows.as_deref())?;
                g = range;
                status = Some(message);
                if changed {
                    st.edit = None;
                    pasted = true;
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
    Ok((g, status, copy, pasted))
}

fn paste_sheet_cells(
    book: &mut scicanvas_core::Book,
    mut range: Range,
    text: &str,
    visible_rows: Option<&[usize]>,
) -> Result<(Range, String, bool), String> {
    let conflicts: Vec<String> = if csv::is_block_paste(text) {
        csv::paste_conflicts(book, range.c0(), range.r0(), text)
    } else {
        book.columns[range.c0()..=range.c1()]
            .iter()
            .filter(|c| !c.data.accepts_paste(text.trim()))
            .map(|c| c.key.clone())
            .collect()
    };
    if !conflicts.is_empty() {
        return Ok((
            range,
            format!(
                "列 {} 与粘贴内容类型不符，数据保持原样。请从列头右键转换为文本列后再粘贴。",
                conflicts.join(", ")
            ),
            false,
        ));
    }
    let message = if csv::is_block_paste(text) {
        let (c0, r0) = (range.c0(), range.r0());
        let (nr, nc, last_row) = if let Some(visible) = visible_rows {
            csv::paste_into_visible(book, c0, visible, r0, text)?
        } else {
            let (nr, nc) = csv::paste_into(book, c0, r0, text);
            (nr, nc, r0 + nr.max(1) - 1)
        };
        range = Range {
            a: Cell { c: c0, r: r0 },
            b: Cell {
                c: c0 + nc.max(1) - 1,
                r: last_row,
            },
        };
        format!("已粘贴 {} 行 × {} 列", nr, nc)
    } else {
        let value = text.trim();
        let rows = book.fill_visible_range(range.c0(), range.c1(), range.r0(), range.r1(), value);
        if range.is_multi() {
            format!("已把 {} 填入 {} 个单元格", value, rows * range.cols())
        } else {
            "已粘贴".to_string()
        }
    };
    Ok((range, message, true))
}

#[derive(Clone, Copy)]
enum Target {
    Cell(usize, usize),
    Row(usize),
    Col(usize),
}

fn drag_selection(
    st: &mut SheetState,
    down: bool,
    hover: Option<Target>,
    last_col: usize,
    book: &scicanvas_core::Book,
) -> (Option<(usize, usize)>, Option<String>) {
    let mut status = None;
    let mut new_cols = None;
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

    (new_cols, status)
}

fn sheet_events(ctx: &egui::Context) -> Vec<egui::Event> {
    use egui::{Event, Key};
    ctx.input_mut(|i| {
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
    })
}
