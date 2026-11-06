//! 项目快照历史。只保存项目数据，窗口位置和选区由界面维护。

use std::collections::VecDeque;
use std::mem::size_of;

use crate::{Book, Column, Graph, Project};

struct Snapshot {
    project: Project,
    label: String,
    bytes: usize,
    generation: u64,
}

pub struct History {
    current: u64,
    next: u64,
    pending: Option<Pending>,
    undo: VecDeque<Snapshot>,
    redo: VecDeque<Snapshot>,
    bytes: usize,
    max_steps: usize,
    max_bytes: usize,
}

struct Pending {
    before: Snapshot,
    changed: bool,
}

impl Default for History {
    fn default() -> Self {
        Self::with_limits(100, 300 * 1024 * 1024)
    }
}

impl History {
    pub fn with_limits(max_steps: usize, max_bytes: usize) -> Self {
        Self {
            current: 0,
            next: 1,
            pending: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            bytes: 0,
            max_steps,
            max_bytes,
        }
    }

    /// 修改前开启事务；同一事务的后续输入复用第一份快照。
    pub fn begin(&mut self, current: &Project, label: &str) {
        if self.pending.is_none() {
            let mut snapshot = Snapshot::new(current.clone(), label.to_string());
            snapshot.generation = self.current;
            self.pending = Some(Pending {
                before: snapshot,
                changed: false,
            });
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// 当前事务里已有写入（用于提交前做公式重算等收尾）。
    pub fn has_changes(&self) -> bool {
        self.pending.as_ref().is_some_and(|p| p.changed)
    }

    /// 实际写入后设置修改标记。空闲帧不比较或复制项目。
    pub fn changed(&mut self) {
        self.pending.as_mut().expect("写入前应开启撤销事务").changed = true;
    }

    /// 一次性编辑收尾时与起点比较；空闲帧和旧连续编辑仍使用 changed 标记。
    pub fn changed_if_different(&mut self, current: &Project) -> bool {
        let pending = self.pending.as_mut().expect("写入前应开启撤销事务");
        pending.changed = pending.before.project != *current;
        pending.changed
    }

    /// 放弃当前事务并移回原快照，不改动已提交的撤销/重做记录及预算。
    pub fn rollback(&mut self) -> Option<Project> {
        self.pending.take().map(|pending| pending.before.project)
    }

    pub fn commit(&mut self, current: &Project) -> bool {
        let Some(pending) = self.pending.take() else {
            return false;
        };
        if !pending.changed || pending.before.project == *current {
            return false;
        }
        self.redo.clear();
        self.current = self.next;
        self.next += 1;
        self.undo.push_back(pending.before);
        self.trim();
        true
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.pending
            .as_ref()
            .filter(|p| p.changed)
            .map(|p| p.before.label.as_str())
            .or_else(|| self.undo.back().map(|s| s.label.as_str()))
    }

    pub fn redo_label(&self) -> Option<&str> {
        if self.pending.as_ref().is_some_and(|p| p.changed) {
            None
        } else {
            self.redo.back().map(|s| s.label.as_str())
        }
    }

    /// 先提交当前事务，再撤销；每次只克隆一次当前项目供重做。
    pub fn undo(&mut self, current: &Project) -> Option<Project> {
        self.commit(current);
        let snapshot = self.undo.pop_back()?;
        let mut reverse = Snapshot::new(current.clone(), snapshot.label);
        reverse.generation = self.current;
        self.current = snapshot.generation;
        self.redo.push_back(reverse);
        self.trim();
        Some(snapshot.project)
    }

    pub fn redo(&mut self, current: &Project) -> Option<Project> {
        self.commit(current);
        let snapshot = self.redo.pop_back()?;
        let mut reverse = Snapshot::new(current.clone(), snapshot.label);
        reverse.generation = self.current;
        self.current = snapshot.generation;
        self.undo.push_back(reverse);
        self.trim();
        Some(snapshot.project)
    }

    pub fn generation(&self) -> u64 {
        self.current
    }

    fn recount(&mut self) {
        let mut allocations = std::collections::HashSet::new();
        self.bytes = self
            .undo
            .iter()
            .chain(&self.redo)
            .map(|s| {
                let mut bytes = s.bytes;
                for col in s.project.books.iter().flat_map(|b| &b.columns) {
                    bytes -= col.data.estimated_bytes();
                    if allocations.insert(col.data.identity()) {
                        bytes += col.data.estimated_bytes();
                    }
                }
                bytes
            })
            .sum();
    }
    fn trim(&mut self) {
        self.recount();
        while self.undo.len() + self.redo.len() > self.max_steps || self.bytes > self.max_bytes {
            let Some(old) = self.undo.pop_front().or_else(|| self.redo.pop_front()) else {
                break;
            };
            drop(old);
            self.recount();
        }
    }
}

impl Snapshot {
    fn new(project: Project, label: String) -> Self {
        let bytes = estimated_bytes(&project) + size_of::<Self>() + label.len();
        Self {
            project,
            label,
            bytes,
            generation: 0,
        }
    }
}

/// 估算快照拥有的堆内存，曲线按 Vec 容量及对象拥有的字符串计入。
fn estimated_bytes(p: &Project) -> usize {
    let mut bytes = p.books.capacity() * size_of::<Book>() + p.graphs.capacity() * size_of::<Graph>();
    bytes += p.notes.capacity() * size_of::<crate::Note>()
        + p.notes
            .iter()
            .map(|n| n.name.capacity() + n.text.capacity())
            .sum::<usize>();
    bytes += p.folders.capacity() * size_of::<crate::project_tree::Folder>()
        + p.folders
            .iter()
            .map(|f| f.name.capacity() + f.items.capacity() * size_of::<u64>())
            .sum::<usize>();
    bytes += p.graph_themes.capacity() * size_of::<crate::graph_theme::GraphTheme>()
        + p.graph_themes.iter().map(|t| t.name.capacity()).sum::<usize>();
    bytes += p.fit_functions.capacity() * size_of::<crate::nlfit::SavedFitFunction>()
        + p.fit_functions
            .iter()
            .map(|saved| {
                let f = &saved.definition;
                f.name.capacity()
                    + f.formula.capacity()
                    + f.constraints.capacity()
                    + f.parameters.capacity() * size_of::<String>()
                    + f.parameters.iter().map(String::capacity).sum::<usize>()
                    + f.initial.capacity() * size_of::<f64>()
            })
            .sum::<usize>();
    for b in &p.books {
        bytes += b.name.capacity() + b.columns.capacity() * size_of::<Column>();
        for c in &b.columns {
            bytes += [&c.key, &c.long_name, &c.units, &c.comment]
                .iter()
                .map(|s| s.capacity())
                .sum::<usize>();
            bytes += c.data.estimated_bytes();
            bytes += c.fit_report.as_ref().map_or(0, |r| r.estimated_bytes());
            bytes += c.derived_from.as_ref().map_or(0, String::capacity);
            bytes += c.error_for.as_ref().map_or(0, String::capacity);
            if let Some(crate::ops::Operation {
                kind:
                    crate::ops::OpKind::Baseline { anchors, .. } | crate::ops::OpKind::BaselineSubtracted { anchors, .. },
                ..
            }) = &c.op
            {
                bytes += anchors.capacity() * size_of::<f64>();
            }
            if let Some(crate::ops::Operation {
                kind: crate::ops::OpKind::Nonlinear(recipe),
                y,
                x,
                ..
            }) = &c.op
            {
                bytes += recipe.estimated_bytes() + y.capacity() + x.as_ref().map_or(0, String::capacity);
            }
        }
    }
    for g in &p.graphs {
        bytes += g.name.capacity() + std::mem::size_of_val(g.page.layers());
        bytes += g.page.shapes.capacity() * size_of::<crate::page::ShapeObject>();
        for layer in g.page.layers() {
            bytes += layer.estimated_plot_bytes();
            bytes += [&layer.title, &layer.x_title, &layer.y_title]
                .iter()
                .map(|s| s.capacity())
                .sum::<usize>();
            bytes += layer.x_key.as_ref().map_or(0, String::capacity);
            bytes += layer.labels.column.as_ref().map_or(0, String::capacity);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_branch_and_noop_preserve_redo() {
        let mut p = Project::default();
        let mut h = History::default();
        let mut states = vec![p.clone()];
        for _ in 0..5 {
            h.begin(&p, "新建");
            p.new_book();
            h.changed();
            assert!(h.commit(&p));
            states.push(p.clone());
        }
        for state in states[..5].iter().rev() {
            p = h.undo(&p).unwrap();
            assert_eq!(&p, state);
        }
        assert!(h.undo(&p).is_none());
        h.begin(&p, "空操作");
        assert!(!h.commit(&p));
        assert_eq!(h.redo_label(), Some("新建"));
        for state in states.iter().skip(1) {
            p = h.redo(&p).unwrap();
            assert_eq!(&p, state);
        }
        p = h.undo(&p).unwrap();
        h.begin(&p, "重命名");
        p.books[0].name = "分支".into();
        h.changed();
        h.commit(&p);
        assert!(h.redo(&p).is_none());
        assert_eq!(h.undo_label(), Some("重命名"));
    }

    #[test]
    fn continuous_large_edit_reuses_snapshot_and_return_to_start_is_noop() {
        let mut p = Project::sample();
        p.books[0].columns[0].data = vec!["123".into(); 100_000].into();
        let initial = p.clone();
        let mut h = History::default();
        h.begin(&p, "列名称");
        let allocation = |p: &Project| {
            let crate::ColumnData::Num { values, .. } = &*p.books[0].columns[0].data else {
                panic!("numeric column expected")
            };
            values.as_ptr()
        };
        let data = allocation(&h.pending.as_ref().unwrap().before.project);
        for _ in 0..50 {
            h.begin(&p, "列名称");
            p.books[0].columns[0].long_name.push('a');
            h.changed();
            assert_eq!(allocation(&h.pending.as_ref().unwrap().before.project), data);
        }
        assert_eq!(h.undo_label(), Some("列名称"));
        let edited = p.clone();
        p = h.undo(&p).unwrap();
        assert_eq!(p, initial);
        p = h.redo(&p).unwrap();
        assert_eq!(p, edited);
        h.begin(&p, "往返编辑");
        p.books[0].name.push('x');
        h.changed();
        p.books[0].name.pop();
        assert!(!h.commit(&p));
        assert_eq!(h.undo_label(), Some("列名称"));
    }

    #[test]
    fn limits_evict_oldest_and_bound_redo() {
        let mut p = Project::default();
        let mut h = History::with_limits(2, usize::MAX);
        for _ in 0..3 {
            h.begin(&p, "新建");
            p.new_book();
            h.changed();
            h.commit(&p);
        }
        p = h.undo(&p).unwrap();
        p = h.undo(&p).unwrap();
        assert_eq!(p.books.len(), 1);
        assert!(h.undo(&p).is_none());
        let mut h = History::with_limits(100, 4096);
        h.begin(&p, "大数据");
        p.books[0].columns[0].set(0, "x".repeat(8192));
        h.changed();
        h.commit(&p);
        p = h.undo(&p).unwrap();
        assert!(h.bytes <= 4096);
        assert!(h.redo(&p).is_none());
    }
    #[test]
    fn fit_report_memory_is_included_in_snapshot_budget() {
        let mut p = Project::sample();
        p.nlfit(
            &crate::ops::Source::Book {
                id: 1,
                selected: vec!["C".into()],
                rows: None,
            },
            crate::nlfit::Model::ExpDec1,
            &[],
            &[],
        )
        .unwrap();
        let before = estimated_bytes(&p);
        p.books[0]
            .columns
            .iter_mut()
            .find_map(|c| c.fit_report.as_mut())
            .unwrap()
            .error = Some("x".repeat(8192));
        assert!(estimated_bytes(&p) >= before + 8192);
    }

    #[test]
    fn custom_fit_library_memory_is_included_in_snapshot_budget() {
        let mut project = Project::default();
        let empty = estimated_bytes(&project);
        let definition = crate::nlfit::CustomFitDefinition::from_text("Line", "a*x+b", "a,b", "2,1", "a >= 0").unwrap();
        project.save_fit_function(None, definition).unwrap();
        project.fit_functions[0].definition.formula.reserve(4096);
        assert!(estimated_bytes(&project) > empty + 4096);
        assert_eq!(estimated_bytes(&project), reference_estimated_bytes(&project));
        let snapshot = Snapshot::new(project.clone(), "function".into());
        assert_eq!(
            snapshot.bytes,
            estimated_bytes(&snapshot.project) + size_of::<Snapshot>() + "function".len()
        );
    }

    fn reference_estimated_bytes(p: &Project) -> usize {
        let mut bytes = p.books.capacity() * size_of::<Book>() + p.graphs.capacity() * size_of::<Graph>();
        bytes += p.notes.capacity() * size_of::<crate::Note>()
            + p.notes
                .iter()
                .map(|n| n.name.capacity() + n.text.capacity())
                .sum::<usize>();
        bytes += p.folders.capacity() * size_of::<crate::project_tree::Folder>()
            + p.folders
                .iter()
                .map(|f| f.name.capacity() + f.items.capacity() * size_of::<u64>())
                .sum::<usize>();
        bytes += p.graph_themes.capacity() * size_of::<crate::graph_theme::GraphTheme>()
            + p.graph_themes.iter().map(|t| t.name.capacity()).sum::<usize>();
        bytes += p.fit_functions.capacity() * size_of::<crate::nlfit::SavedFitFunction>()
            + p.fit_functions
                .iter()
                .map(|saved| {
                    let f = &saved.definition;
                    f.name.capacity()
                        + f.formula.capacity()
                        + f.constraints.capacity()
                        + f.parameters.capacity() * size_of::<String>()
                        + f.parameters.iter().map(String::capacity).sum::<usize>()
                        + f.initial.capacity() * size_of::<f64>()
                })
                .sum::<usize>();
        for b in &p.books {
            bytes += b.name.capacity() + b.columns.capacity() * size_of::<Column>();
            for c in &b.columns {
                bytes += [&c.key, &c.long_name, &c.units, &c.comment]
                    .iter()
                    .map(|s| s.capacity())
                    .sum::<usize>();
                bytes += c.data.estimated_bytes();
                bytes += c.fit_report.as_ref().map_or(0, |r| r.estimated_bytes());
                bytes += c.derived_from.as_ref().map_or(0, String::capacity);
                bytes += c.error_for.as_ref().map_or(0, String::capacity);
                if let Some(crate::ops::Operation {
                    kind:
                        crate::ops::OpKind::Baseline { anchors, .. }
                        | crate::ops::OpKind::BaselineSubtracted { anchors, .. },
                    ..
                }) = &c.op
                {
                    bytes += anchors.capacity() * size_of::<f64>();
                }
                if let Some(crate::ops::Operation {
                    kind: crate::ops::OpKind::Nonlinear(recipe),
                    y,
                    x,
                    ..
                }) = &c.op
                {
                    bytes += recipe.estimated_bytes() + y.capacity() + x.as_ref().map_or(0, String::capacity);
                }
            }
        }
        for g in &p.graphs {
            bytes += g.name.capacity() + std::mem::size_of_val(g.page.layers());
            bytes += g.page.shapes.capacity() * size_of::<crate::page::ShapeObject>();
            for layer in g.page.layers() {
                bytes += layer.functions.capacity() * size_of::<crate::page::FunctionCurve>();
                for function in &layer.functions {
                    bytes += function.expr.capacity() + function.style.legend_label.capacity();
                    for column in [
                        &function.style.color_column,
                        &function.style.size_column,
                        &function.style.symbol_column,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        bytes += column.capacity();
                    }
                }
                bytes += layer.spec.plots.capacity() * size_of::<crate::page::plots::Plot>();
                for plot in &layer.spec.plots {
                    bytes += plot.source.col.capacity();
                    bytes += plot.x.as_ref().map_or(0, |r| r.col.capacity());
                    bytes += plot.base.as_ref().map_or(0, |r| r.col.capacity());
                    bytes += plot.style.as_ref().map_or(0, |style| style.legend_label.capacity());
                }
                bytes += [&layer.title, &layer.x_title, &layer.y_title]
                    .iter()
                    .map(|s| s.capacity())
                    .sum::<usize>();
                bytes += layer.x_key.as_ref().map_or(0, String::capacity);
                bytes += layer.labels.column.as_ref().map_or(0, String::capacity);
            }
        }
        bytes
    }

    fn curve_budget_fixture(unbound: bool, empty: bool) -> Project {
        fn reserved(text: &str, capacity: usize) -> String {
            let mut value = String::with_capacity(capacity);
            value.push_str(text);
            value
        }
        let mut p = Project::sample();
        p.books.reserve(3);
        p.graphs.reserve(2);
        let mut theme = crate::graph_theme::GraphTheme::builtin();
        theme.name = reserved("Science", 512);
        p.graph_themes.push(theme);
        let layer = p.graphs[0].first_layer_mut();
        if unbound {
            layer.book_id = None;
        }
        let mut function = crate::page::FunctionCurve::new(reserved("sin(x)", 4096));
        function.style.legend_label = reserved("function", 128);
        layer.functions.push(function);
        layer.edit_draft(|draft| {
            draft.y_keys.push(reserved("中文列@2", 96));
            draft.line_only.push(reserved("派生线@2", 128));
            draft.x_by_y.insert(reserved("hidden@2", 80), reserved("A@2", 192));
            draft.base_by_y.insert(reserved("B", 64), reserved("", 96));
            draft
                .base_by_y
                .insert(reserved("hidden@2", 160), reserved("lower@2", 320));
            draft.row_by_y.insert(reserved("hidden", 144), None);
            draft.row_by_y.insert(reserved("中文列@2", 240), Some((1, 3)));
            if empty {
                draft.y_keys.clear();
                draft.line_only.clear();
            }
        });
        layer.spec.plots.reserve(37);
        layer.spec.plots[0].source.col.reserve(96);
        layer.spec.x_key = Some(reserved("A", 64));
        layer.labels.column = Some(reserved("标签列", 128));
        layer.title = reserved("预算回归", 256);
        let second = layer.clone();
        p.graphs[0].page.add_layer(second);
        p
    }

    #[test]
    fn curve_budget_counts_vec_capacity_hidden_and_unbound_entries() {
        for unbound in [false, true] {
            for empty in [false, true] {
                let p = curve_budget_fixture(unbound, empty);
                // 原项目的预留容量和快照克隆后的容量分别核对。
                assert_eq!(estimated_bytes(&p), reference_estimated_bytes(&p));
                let mut snapshot = p.clone();
                assert_eq!(estimated_bytes(&snapshot), reference_estimated_bytes(&snapshot));
                let before = estimated_bytes(&snapshot);
                snapshot.graphs[0].first_layer_mut().set_plot_style(
                    "style-only",
                    Some(crate::page::CurveStyle {
                        legend_label: "曲线对象拥有的标签".repeat(100),
                        right_axis: true,
                        ..Default::default()
                    }),
                );
                assert!(estimated_bytes(&snapshot) > before, "逐项计入 Vec 对象及标签的实际分配");
                assert_eq!(estimated_bytes(&snapshot), reference_estimated_bytes(&snapshot));
            }
        }
    }

    #[test]
    fn curve_budget_preserves_exact_limits_eviction_redo_and_branching() {
        let initial = curve_budget_fixture(true, true);
        let label = "曲线改动😀";
        let cost = reference_estimated_bytes(&initial.clone()) + size_of::<Snapshot>() + label.len();
        let data: usize = initial
            .books
            .iter()
            .flat_map(|b| &b.columns)
            .map(|c| c.data.estimated_bytes())
            .sum();
        let charge = |n: usize| if n == 0 { 0 } else { data + n * (cost - data) };
        for (max_steps, max_bytes, retained) in [
            (2, usize::MAX, 2),
            (100, charge(2), 2),
            (100, charge(2) - 1, 1),
            (100, charge(1) - 1, 0),
        ] {
            let mut p = initial.clone();
            let mut states = vec![p.clone()];
            let mut h = History::with_limits(max_steps, max_bytes);
            for rows in [Some((0, 3)), Some((1, 4))] {
                h.begin(&p, label);
                assert_eq!(h.pending.as_ref().unwrap().before.bytes, cost);
                p.graphs[0].first_layer_mut().spec.rows = rows;
                h.changed();
                assert!(h.commit(&p));
                states.push(p.clone());
            }
            assert_eq!(h.undo.len(), retained);
            assert_eq!(h.bytes, charge(retained));
            for i in (2 - retained..2).rev() {
                assert_eq!(h.undo_label(), Some(label));
                p = h.undo(&p).unwrap();
                assert_eq!(p, states[i]);
                assert_eq!(h.bytes, charge(retained));
            }
            assert!(h.undo(&p).is_none());
            for state in states.iter().skip(3 - retained) {
                assert_eq!(h.redo_label(), Some(label));
                p = h.redo(&p).unwrap();
                assert_eq!(&p, state);
                assert_eq!(h.bytes, charge(retained));
            }
            assert!(h.redo(&p).is_none());
            if retained > 0 {
                p = h.undo(&p).unwrap();
                h.begin(&p, label);
                assert!(!h.commit(&p));
                assert_eq!(h.redo_label(), Some(label));
                h.begin(&p, label);
                p.graphs[0].first_layer_mut().spec.rows = Some((2, 5));
                h.changed();
                assert!(h.commit(&p));
                assert_eq!(h.bytes, charge(retained));
                assert!(h.redo(&p).is_none());
            }
        }
    }

    #[test]
    fn rollback_moves_pending_snapshot_and_preserves_history_budget_and_redo() {
        let mut p = Project::sample();
        let initial = p.clone();
        let mut h = History::default();
        h.begin(&p, "已提交");
        p.new_book();
        h.changed();
        assert!(h.commit(&p));
        let committed = p.clone();
        p = h.undo(&p).unwrap();
        assert_eq!(p, initial);
        let bytes = h.bytes;
        h.begin(&p, "放弃");
        let crate::ColumnData::Num { values, .. } =
            &*h.pending.as_ref().unwrap().before.project.books[0].columns[0].data
        else {
            panic!("numeric");
        };
        let allocation = values.as_ptr();
        p.books[0].columns[0].set(0, "临时文字".into());
        p.new_book();
        assert!(h.changed_if_different(&p));
        assert!(h.has_changes());
        p = h.rollback().unwrap();
        assert_eq!(p, initial);
        let crate::ColumnData::Num { values, .. } = &*p.books[0].columns[0].data else {
            panic!("numeric");
        };
        assert_eq!(values.as_ptr(), allocation, "回滚移动原快照而非再次克隆");
        assert!(!h.is_pending());
        assert_eq!(h.bytes, bytes);
        assert!(h.undo_label().is_none());
        assert_eq!(h.redo_label(), Some("已提交"));
        assert!(h.rollback().is_none());
        assert_eq!(h.bytes, bytes);
        assert_eq!(h.redo(&p).unwrap(), committed);
    }

    #[test]
    fn actual_change_detection_can_return_to_start_without_consuming_redo() {
        let mut p = Project::sample();
        let mut h = History::default();
        h.begin(&p, "已提交");
        p.new_book();
        h.changed();
        assert!(h.commit(&p));
        p = h.undo(&p).unwrap();
        let before = p.clone();
        let bytes = h.bytes;
        h.begin(&p, "无改动");
        assert!(!h.changed_if_different(&p));
        assert!(!h.has_changes());
        p.books[0].name.push('!');
        assert!(h.changed_if_different(&p));
        p.books[0].name.pop();
        assert!(!h.changed_if_different(&p));
        assert!(!h.has_changes());
        assert!(!h.commit(&p));
        assert_eq!(p, before);
        assert_eq!(h.bytes, bytes);
        assert_eq!(h.redo_label(), Some("已提交"));
        assert!(h.undo_label().is_none());
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn save_generations_follow_undo_redo_and_branch_without_reusing_ids() {
        let mut p = Project::sample();
        let mut h = History::default();
        let saved = h.generation();
        h.begin(&p, "编辑");
        p.books[0].columns[0].set(0, "123".into());
        h.changed();
        h.commit(&p);
        let changed = h.generation();
        assert_ne!(saved, changed);
        p = h.undo(&p).unwrap();
        assert_eq!(h.generation(), saved);
        p = h.redo(&p).unwrap();
        assert_eq!(h.generation(), changed);
        p = h.undo(&p).unwrap();
        h.begin(&p, "分支");
        p.books[0].name.push('x');
        h.changed();
        h.commit(&p);
        assert_ne!(h.generation(), changed);
        let generation = h.generation();
        h.begin(&p, "无变化");
        h.changed();
        assert!(!h.commit(&p));
        assert_eq!(h.generation(), generation);
    }
    #[test]
    fn snapshots_share_columns_and_only_changed_column_is_copied() {
        let mut p = Project::sample();
        let before = p.clone();
        let mut h = History::default();
        assert_eq!(
            p.books[0].columns[0].data.identity(),
            before.books[0].columns[0].data.identity()
        );
        h.begin(&p, "单元格");
        p.books[0].columns[0].set(0, "99".into());
        h.changed();
        h.commit(&p);
        assert_ne!(
            p.books[0].columns[0].data.identity(),
            before.books[0].columns[0].data.identity()
        );
        assert_eq!(
            p.books[0].columns[1].data.identity(),
            before.books[0].columns[1].data.identity()
        );
        assert_eq!(h.undo(&p).unwrap(), before);
    }
}
