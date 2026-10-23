use super::*;

pub(super) fn project(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "book.new",
            "新建工作簿",
            "newbook",
            "Ctrl+N",
            "文件",
            "new workbook sheet",
            always,
            |a| a.new_book(),
        )
        .modifies(),
        c(
            "graph.new",
            "新建空白图形",
            "graph",
            "",
            "文件",
            "new graph 空白图 新建图",
            always,
            |a| a.new_graph(),
        )
        .modifies(),
        c(
            "file.import",
            "导入数据",
            "import",
            "Ctrl+I",
            "文件",
            "import csv txt excel",
            always,
            |a| a.import_csv(),
        )
        .modifies(),
        c(
            "file.open",
            "打开项目",
            "open",
            "Ctrl+O",
            "文件",
            "open project",
            always,
            |a| a.open_project(),
        ),
        c(
            "file.save",
            "保存项目",
            "save",
            "Ctrl+S",
            "文件",
            "save project",
            always,
            |a| a.save_project(false),
        ),
        c(
            "file.saveas",
            "项目另存为",
            "save",
            "",
            "文件",
            "save as",
            always,
            |a| a.save_project(true),
        ),
        c(
            "graph.export",
            "导出 SVG",
            "export",
            "Ctrl+E",
            "文件",
            "export svg image vector 矢量",
            need_any_graph,
            |a| a.export_svg(),
        ),
        c(
            "book.export_csv",
            "导出 CSV",
            "export",
            "",
            "文件",
            "export csv excel table worksheet 表格 工作表",
            need_book,
            |a| a.export_csv(),
        ),
    ]);
}

pub(super) fn graphics(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "graph.export_pdf",
            "导出 PDF",
            "export",
            "",
            "文件",
            "export pdf vector font embed 矢量 字体 嵌入",
            need_any_graph,
            |a| a.export_pdf(),
        ),
        c(
            "graph.export_png",
            "导出 PNG",
            "export",
            "",
            "文件",
            "export png bitmap raster dpi image 位图 分辨率",
            need_any_graph,
            |a| a.export_png_dialog(),
        ),
    ]);
}

pub(super) fn session(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "project.reset",
            "恢复示例项目",
            "reset",
            "",
            "文件",
            "reset sample",
            always,
            |a| a.reset_project(),
        ),
        c("app.quit", "退出", "quit", "", "文件", "quit exit", always, |a| {
            a.quit = true
        }),
    ]);
}

pub(super) fn note(commands: &mut Vec<Command>) {
    commands.push(c(
        "note.export",
        "导出 Note",
        "book",
        "",
        "导出",
        "note txt md markdown 文本",
        |a| {
            a.active
                .and_then(|id| a.project.note(id))
                .map(|_| ())
                .ok_or("请先激活 Note 窗口")
        },
        |a| a.export_note(),
    ));
}
