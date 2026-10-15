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
    present: Project,
    undo: VecDeque<Snapshot>,
    redo: VecDeque<Snapshot>,
    bytes: usize,
    max_steps: usize,
    max_bytes: usize,
    mergeable: bool,
}

impl History {
    pub fn new(project: &Project) -> Self {
        Self::with_limits(project, 100, 300 * 1024 * 1024)
    }

    pub fn with_limits(project: &Project, max_steps: usize, max_bytes: usize) -> Self {
        Self {
            present: project.clone(),
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            bytes: 0,
            max_steps,
            max_bytes,
            mergeable: false,
        }
    }

    /// 在实际修改后记录；无变化时保留重做栈。同一控件的一次编辑可合并。
    pub fn record(&mut self, current: &Project, label: &str, merge: bool) -> bool {
        // ponytail: 比较项目需要 O(数据量)，大项目改用修改代数和 Arc 列数据。
        if self.present == *current {
            return false;
        }
        let before = std::mem::replace(&mut self.present, current.clone());
        let merge = merge && self.mergeable;
        self.mergeable = true;
        self.redo.clear();
        self.bytes = self.undo.iter().map(|s| s.bytes).sum();
        if !(merge && !self.undo.is_empty()) {
            let snapshot = Snapshot::new(before, label.to_string());
            self.bytes += snapshot.bytes;
            self.undo.push_back(snapshot);
        } else if self.undo.back().is_some_and(|s| s.project == *current) {
            // 一次连续编辑又回到起点，不留空记录。
            self.bytes -= self.undo.pop_back().unwrap().bytes;
            self.mergeable = false;
        }
        self.trim();
        true
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.back().map(|s| s.label.as_str())
    }

    /// 调用前先 record，保证当前编辑已提交到历史。
    pub fn undo(&mut self, current: &Project) -> Option<Project> {
        self.mergeable = false;
        let snapshot = self.undo.pop_back()?;
        self.bytes -= snapshot.bytes;
        let reverse = Snapshot::new(current.clone(), snapshot.label);
        self.bytes += reverse.bytes;
        self.redo.push_back(reverse);
        self.present = snapshot.project.clone();
        self.trim();
        Some(snapshot.project)
    }

    pub fn redo(&mut self, current: &Project) -> Option<Project> {
        self.mergeable = false;
        let snapshot = self.redo.pop_back()?;
        self.bytes -= snapshot.bytes;
        let reverse = Snapshot::new(current.clone(), snapshot.label);
        self.bytes += reverse.bytes;
        self.undo.push_back(reverse);
        self.present = snapshot.project.clone();
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
        if self.undo.is_empty() {
            self.mergeable = false;
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
    for b in &p.books {
        bytes += b.name.capacity() + b.columns.capacity() * size_of::<Column>();
        for c in &b.columns {
            bytes += [&c.key, &c.long_name, &c.units, &c.comment]
                .iter()
                .map(|s| s.capacity())
                .sum::<usize>();
            bytes += c.data.capacity() * size_of::<String>() + c.data.iter().map(String::capacity).sum::<usize>();
            bytes += c.derived_from.as_ref().map_or(0, String::capacity);
        }
    }
    for g in &p.graphs {
        bytes += g.name.capacity() + std::mem::size_of_val(g.page.layers());
        for layer in g.page.layers() {
            bytes += [&layer.title, &layer.x_title, &layer.y_title]
                .iter()
                .map(|s| s.capacity())
                .sum::<usize>();
            bytes += layer.x_key.as_ref().map_or(0, String::capacity);
            for keys in [&layer.y_keys, &layer.line_only] {
                bytes += keys.capacity() * size_of::<String>() + keys.iter().map(String::capacity).sum::<usize>();
            }
            bytes += layer
                .x_by_y
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
    fn roundtrip_and_branch() {
        let mut p = Project::default();
        let initial = p.clone();
        let mut h = History::new(&p);
        let mut states = vec![p.clone()];
        for _ in 0..5 {
            p.new_book();
            assert!(h.record(&p, "新建工作簿", false));
            states.push(p.clone());
        }
        for state in states[..5].iter().rev() {
            p = h.undo(&p).unwrap();
            assert_eq!(&p, state);
        }
        assert_eq!(p, initial);
        assert!(h.undo(&p).is_none());
        assert!(!h.record(&p, "无变化", false));
        for state in states.iter().skip(1) {
            p = h.redo(&p).unwrap();
            assert_eq!(&p, state);
        }
        p = h.undo(&p).unwrap();
        p.books[0].name = "分支".into();
        h.record(&p, "重命名", false);
        assert!(h.redo(&p).is_none());
        assert_eq!(h.undo_label(), Some("重命名"));
    }

    #[test]
    fn edits_merge_and_return_to_start() {
        let mut p = Project::sample();
        let initial = p.clone();
        let mut h = History::new(&p);
        p.books[0].name = "a".into();
        h.record(&p, "重命名", false);
        p.books[0].name = "ab".into();
        h.record(&p, "重命名", true);
        p = h.undo(&p).unwrap();
        assert_eq!(p, initial);
        assert_eq!(h.redo_label(), Some("重命名"));
        p = h.redo(&p).unwrap();
        p.books[0].name = initial.books[0].name.clone();
        h.record(&p, "重命名", false);
        let before = p.clone();
        p.books[0].name = "临时".into();
        h.record(&p, "连续编辑", false);
        p = before.clone();
        h.record(&p, "连续编辑", true);
        p.books[0].name = "重新输入".into();
        h.record(&p, "连续编辑", true);
        p = h.undo(&p).unwrap();
        assert_eq!(p, before);
        assert_eq!(h.undo_label(), Some("重命名"));
    }

    #[test]
    fn limits_evict_oldest_and_bound_redo() {
        let mut p = Project::default();
        let mut h = History::with_limits(&p, 2, usize::MAX);
        for _ in 0..3 {
            p.new_book();
            h.record(&p, "新建", false);
        }
        p = h.undo(&p).unwrap();
        p = h.undo(&p).unwrap();
        assert_eq!(p.books.len(), 1);
        assert!(h.undo(&p).is_none());

        let mut h = History::with_limits(&p, 100, 4096);
        p.books[0].columns[0].set(0, "x".repeat(8192));
        h.record(&p, "大数据", false);
        p = h.undo(&p).unwrap();
        assert!(h.bytes <= 4096);
        assert!(h.redo(&p).is_none());
        p.books[0].columns[0].set(0, "x".repeat(8192));
        let mut h = History::with_limits(&p, 100, 4096);
        p.books[0].name = "修改".into();
        h.record(&p, "重命名", false);
        assert!(h.undo_label().is_none());
    }
}
