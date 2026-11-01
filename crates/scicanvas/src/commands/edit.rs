use super::*;

pub(super) fn graph_copy(commands: &mut Vec<Command>) {
    commands.extend([c(
        "graph.copy",
        "复制图形",
        "copy",
        "Ctrl+J",
        "编辑",
        "copy graph clipboard image word ppt 剪贴板 粘贴",
        need_any_graph,
        |a| a.copy_graph(),
    )]);
}

pub(super) fn selection(commands: &mut Vec<Command>) {
    commands.extend([
        // 选区
        c(
            "edit.undo",
            "撤销",
            "undo",
            "Ctrl+Z",
            "编辑",
            "undo 撤销",
            need_undo,
            |a| a.undo(),
        ),
        c(
            "edit.redo",
            "重做",
            "redo",
            "Ctrl+Y / Ctrl+Shift+Z",
            "编辑",
            "redo 重做",
            need_redo,
            |a| a.redo(),
        ),
        c(
            "edit.copy",
            "复制",
            "copy",
            "Ctrl+C",
            "选区",
            "copy cells clipboard",
            need_range,
            |a| a.copy_range(),
        ),
        c(
            "edit.find",
            "查找",
            "search",
            "Ctrl+F",
            "选区",
            "find search 查找",
            need_book,
            |a| a.open_find_replace(false),
        ),
        c(
            "edit.replace",
            "替换",
            "search",
            "Ctrl+H",
            "选区",
            "replace 查找替换",
            need_book,
            |a| a.open_find_replace(true),
        ),
        c(
            "edit.clear",
            "清除内容",
            "clear",
            "Delete",
            "选区",
            "clear cells delete",
            need_range,
            |a| a.clear_range(),
        )
        .modifies(),
    ]);
}
