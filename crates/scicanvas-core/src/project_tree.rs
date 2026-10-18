//! 项目中的虚拟文件夹；窗口显示状态仍由桌面会话管理。
use crate::Project;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Folder {
    pub id: u64,
    pub name: String,
    pub parent: Option<u64>,
    pub items: Vec<u64>,
}

impl Project {
    pub fn folder(&self, id: u64) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == id)
    }
    pub fn folder_of(&self, item: u64) -> Option<u64> {
        self.folders.iter().find(|f| f.items.contains(&item)).map(|f| f.id)
    }
    pub fn folder_path(&self, id: u64) -> Option<String> {
        let mut names = Vec::new();
        let mut current = Some(id);
        while let Some(id) = current {
            let folder = self.folder(id)?;
            names.push(folder.name.as_str());
            current = folder.parent;
        }
        names.reverse();
        Some(names.join(" / "))
    }
    pub fn add_folder(&mut self, parent: Option<u64>) -> Result<u64, String> {
        if parent.is_some_and(|id| self.folder(id).is_none()) {
            return Err("找不到目标文件夹".into());
        }
        let name = (1..)
            .map(|i| format!("文件夹{i}"))
            .find(|n| !self.folders.iter().any(|f| f.parent == parent && &f.name == n))
            .unwrap();
        let id = self.alloc_id();
        self.folders.push(Folder {
            id,
            name,
            parent,
            items: Vec::new(),
        });
        if let Err(e) = self.validate_folders() {
            self.folders.pop();
            self.next_id = id;
            return Err(e);
        }
        Ok(id)
    }
    pub fn move_project_item(&mut self, id: u64, destination: Option<u64>) -> Result<(), String> {
        if destination.is_some_and(|d| self.folder(d).is_none()) {
            return Err("找不到目标文件夹".into());
        }
        if let Some(index) = self.folders.iter().position(|f| f.id == id) {
            let before = self.folders[index].parent;
            self.folders[index].parent = destination;
            if let Err(e) = self.validate_folders() {
                self.folders[index].parent = before;
                return Err(e);
            }
        } else {
            if self.book(id).is_none() && self.graph(id).is_none() && self.note(id).is_none() {
                return Err("找不到项目对象".into());
            }
            for f in &mut self.folders {
                f.items.retain(|item| *item != id);
            }
            if let Some(d) = destination {
                self.folders.iter_mut().find(|f| f.id == d).unwrap().items.push(id);
            }
        }
        Ok(())
    }
    /// 只删除组织层级，内容移到上级，数据与图形保持。
    pub fn remove_folder(&mut self, id: u64) -> Result<(), String> {
        let index = self.folders.iter().position(|f| f.id == id).ok_or("找不到文件夹")?;
        let removed = self.folders.remove(index);
        for f in &mut self.folders {
            if f.parent == Some(id) {
                f.parent = removed.parent;
            }
        }
        if let Some(parent) = removed.parent {
            self.folders
                .iter_mut()
                .find(|f| f.id == parent)
                .unwrap()
                .items
                .extend(removed.items);
        }
        Ok(())
    }
    pub fn rename_project_item(&mut self, id: u64, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("名称不能为空".into());
        }
        *self.name_of_mut(id).ok_or("找不到项目对象")? = name.to_string();
        Ok(())
    }
    pub fn validate_folders(&self) -> Result<(), String> {
        use std::collections::BTreeSet;
        let mut items = BTreeSet::new();
        for folder in &self.folders {
            if folder.name.trim().is_empty() {
                return Err("文件夹名称不能为空".into());
            }
            let mut seen = BTreeSet::new();
            let mut current = Some(folder.id);
            while let Some(id) = current {
                if !seen.insert(id) {
                    return Err("文件夹不能移动到自身或下级中".into());
                }
                if seen.len() > 64 {
                    return Err("文件夹最多支持 64 级".into());
                }
                current = self.folder(id).ok_or("文件夹引用了不存在的上级")?.parent;
            }
            for id in &folder.items {
                if (self.book(*id).is_none() && self.graph(*id).is_none() && self.note(*id).is_none())
                    || !items.insert(*id)
                {
                    return Err("文件夹中的对象不存在或重复归属".into());
                }
            }
        }
        Ok(())
    }
}
