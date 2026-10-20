//! 项目快照历史。只保存项目数据，窗口位置和选区由界面维护。

use std::collections::VecDeque;
use std::mem::size_of;

use crate::{Book, Column, Graph, Project};

struct Snapshot {
    project: Project,
    label: String,
    bytes: usize,
}

pub struct History {
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
            self.pending = Some(Pending {
                before: Snapshot::new(current.clone(), label.to_string()),
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

    pub fn commit(&mut self, current: &Project) -> bool {
        let Some(pending) = self.pending.take() else {
            return false;
        };
        if !pending.changed || pending.before.project == *current {
            return false;
        }
        self.redo.clear();
        self.bytes = self.undo.iter().map(|s| s.bytes).sum::<usize>() + pending.before.bytes;
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
        self.bytes -= snapshot.bytes;
        let reverse = Snapshot::new(current.clone(), snapshot.label);
        self.bytes += reverse.bytes;
        self.redo.push_back(reverse);
        self.trim();
        Some(snapshot.project)
    }

    pub fn redo(&mut self, current: &Project) -> Option<Project> {
        self.commit(current);
        let snapshot = self.redo.pop_back()?;
        self.bytes -= snapshot.bytes;
        let reverse = Snapshot::new(current.clone(), snapshot.label);
        self.bytes += reverse.bytes;
        self.undo.push_back(reverse);
        self.trim();
        Some(snapshot.project)
    }

    fn trim(&mut self) {
        while self.undo.len() + self.redo.len() > self.max_steps || self.bytes > self.max_bytes {
            let Some(old) = self.undo.pop_front().or_else(|| self.redo.pop_front()) else {
                break;
            };
            self.bytes -= old.bytes;
        }
    }
}

impl Snapshot {
    fn new(project: Project, label: String) -> Self {
        let bytes = estimated_bytes(&project) + size_of::<Self>() + label.len();
        Self { project, label, bytes }
    }
}

/// 估算快照拥有的堆内存；BTreeMap 按每项 128 字节保守计入节点开销。
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
    for b in &p.books {
        bytes += b.name.capacity() + b.columns.capacity() * size_of::<Column>();
        for c in &b.columns {
            bytes += [&c.key, &c.long_name, &c.units, &c.comment]
                .iter()
                .map(|s| s.capacity())
                .sum::<usize>();
            bytes += c.data.estimated_bytes();
            bytes += c.derived_from.as_ref().map_or(0, String::capacity);
            bytes += c.error_for.as_ref().map_or(0, String::capacity);
        }
    }
    for g in &p.graphs {
        bytes += g.name.capacity() + std::mem::size_of_val(g.page.layers());
        for layer in g.page.layers() {
            bytes += layer.row_by_y.keys().map(|key| 128 + key.capacity()).sum::<usize>();
            bytes += [&layer.title, &layer.x_title, &layer.y_title]
                .iter()
                .map(|s| s.capacity())
                .sum::<usize>();
            bytes += layer.x_key.as_ref().map_or(0, String::capacity);
            bytes += layer.labels.column.as_ref().map_or(0, String::capacity);
            for keys in [&layer.y_keys, &layer.line_only] {
                bytes += keys.capacity() * size_of::<String>() + keys.iter().map(String::capacity).sum::<usize>();
            }
            bytes += layer
                .x_by_y
                .iter()
                .map(|(k, v)| 128 + k.capacity() + v.capacity())
                .sum::<usize>();
            bytes += layer
                .base_by_y
                .iter()
                .map(|(k, v)| 128 + k.capacity() + v.capacity())
                .sum::<usize>();
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
            let crate::ColumnData::Num { values, .. } = &p.books[0].columns[0].data else {
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
}
