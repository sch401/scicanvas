use super::*;

pub(super) fn objects(commands: &mut Vec<Command>) {
    commands.extend([
        // 文件
        c(
            "project.folder.new",
            "新建文件夹",
            "open",
            "",
            "工作区",
            "folder new 文件夹",
            always,
            |a| a.new_folder(),
        )
        .modifies(),
        c(
            "project.folder.remove",
            "删除文件夹（保留内容）",
            "coldel",
            "",
            "工作区",
            "folder remove",
            need_folder,
            |a| a.remove_project_folder(),
        )
        .modifies(),
        c(
            "project.rename",
            "重命名项目对象",
            "props",
            "",
            "工作区",
            "rename folder book graph",
            need_project_item,
            |a| a.rename_project_target(),
        )
        .modifies(),
        c(
            "project.move",
            "移动项目对象",
            "open",
            "",
            "工作区",
            "move folder",
            need_project_item,
            |a| a.move_project_target(),
        )
        .modifies(),
        c(
            "project.show",
            "显示窗口",
            "book",
            "",
            "工作区",
            "show window",
            need_window,
            |a| a.show_project_window(),
        ),
        c(
            "project.hide",
            "隐藏窗口",
            "book",
            "",
            "工作区",
            "hide window",
            need_window,
            |a| a.hide_project_window(),
        ),
        c(
            "project.delete",
            "删除项目对象…",
            "coldel",
            "",
            "工作区",
            "delete item",
            need_window,
            |a| a.confirm_delete = a.project_target(),
        ),
    ]);
}

pub(super) fn note(commands: &mut Vec<Command>) {
    commands.push(
        c(
            "note.new",
            "新建 Note",
            "book",
            "",
            "工作区",
            "note 笔记 计算结果",
            always,
            |a| a.new_note(),
        )
        .modifies(),
    );
}
