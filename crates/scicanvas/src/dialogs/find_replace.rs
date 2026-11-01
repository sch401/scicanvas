//! Worksheet find and replace controls.
use super::*;
use crate::sheet::{Cell, Range};

#[derive(Clone)]
pub(crate) struct FindReplace {
    pub book: u64,
    pub selection: Option<Range>,
    pub selected_only: bool,
    pub needle: String,
    pub replacement: String,
    pub match_case: bool,
    pub whole_cell: bool,
    pub replace_mode: bool,
    pub cursor: Option<Cell>,
    pub message: String,
}

impl FindReplace {
    pub(crate) fn new(book: u64, selection: Option<Range>, replace_mode: bool) -> Self {
        Self {
            book,
            selection,
            selected_only: selection.is_some_and(|r| r.is_multi()),
            needle: String::new(),
            replacement: String::new(),
            match_case: false,
            whole_cell: false,
            replace_mode,
            cursor: None,
            message: String::new(),
        }
    }

    pub(super) fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "查找和替换")
            .open(open)
            .default_width(430.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("查找");
                    let input = ui.text_edit_singleline(&mut self.needle);
                    if input.changed() {
                        self.cursor = None;
                        self.message.clear();
                    }
                    if input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        action = Action::FindNext;
                    }
                });
                ui.checkbox(&mut self.replace_mode, "显示替换选项");
                if self.replace_mode {
                    ui.horizontal(|ui| {
                        ui.label("替换为");
                        ui.text_edit_singleline(&mut self.replacement);
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.match_case, "区分大小写");
                    ui.checkbox(&mut self.whole_cell, "整格匹配");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("范围");
                    ui.selectable_value(&mut self.selected_only, false, "当前工作表");
                    ui.add_enabled_ui(self.selection.is_some(), |ui| {
                        ui.selectable_value(&mut self.selected_only, true, "所选区域");
                    });
                });
                if !self.message.is_empty() {
                    ui.label(&self.message);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("查找下一个").clicked() {
                        action = Action::FindNext;
                    }
                    if self.replace_mode && ui.button("替换当前").clicked() {
                        action = Action::ReplaceCurrent;
                    }
                    if self.replace_mode && ui.button("全部替换").clicked() {
                        action = Action::ReplaceAll;
                    }
                    if ui.button("关闭").clicked() {
                        action = Action::Close;
                    }
                });
            });
        action
    }
}
