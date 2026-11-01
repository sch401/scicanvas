use super::*;

pub(super) fn types_and_roles(commands: &mut Vec<Command>) {
    commands.extend([
        // 列与工作表
        c("col.type.num", "转换为数值列", "props", "", "列", "numeric type", need_cols, |a| a.set_data_kind(scicanvas_core::DataKind::Num)).modifies(),
        c("col.type.text", "转换为文本列", "props", "", "列", "text type", need_cols, |a| a.set_data_kind(scicanvas_core::DataKind::Text)).modifies(),
        c("col.type.datetime", "转换为日期时间列", "props", "", "列", "datetime UTC type", need_cols, |a| a.set_data_kind(scicanvas_core::DataKind::DateTime)).modifies(),
        c("col.type.category", "转换为分类列", "props", "", "列", "category type", need_cols, |a| a.set_data_kind(scicanvas_core::DataKind::Category)).modifies(),
        c(
            "col.props",
            "列属性",
            "props",
            "",
            "列",
            "column properties format decimal places scientific engineering percent date 列属性 显示格式 小数位 科学计数 工程单位 百分比 日期格式 长名称 单位",
            need_cols,
            |a| a.open_column_props(),
        ),
        c(
            "col.setX",
            "设为 X",
            "setx",
            "",
            "列",
            "set x column",
            need_cols,
            |a| a.set_role(Role::X),
        )
        .modifies(),
        c(
            "col.setY",
            "设为 Y",
            "sety",
            "",
            "列",
            "set y column",
            need_cols,
            |a| a.set_role(Role::Y),
        )
        .modifies(),
        c(
            "col.setIgnore",
            "设为忽略",
            "ignore",
            "",
            "列",
            "disregard",
            need_cols,
            |a| a.set_role(Role::Ignore),
        )
        .modifies(),
        c("col.setZ", "设为 Z", "sety", "", "列", "z column 三维 等高线", need_cols, |a| a.set_role(Role::Z)).modifies(),
        c("col.setLabel", "设为标签", "ignore", "", "列", "label column 标签列 数据标签", need_cols, |a| {
            a.set_role(Role::Label)
        })
        .modifies(),
        c(
            "col.setXError",
            "设为 X 误差",
            "setx",
            "",
            "列",
            "x error",
            need_cols,
            |a| a.set_role(Role::XError),
        )
        .modifies(),
        c(
            "col.setYError",
            "设为 Y 误差",
            "sety",
            "",
            "列",
            "y error",
            need_cols,
            |a| a.set_role(Role::YError),
        )
        .modifies(),
    ]);
}

pub(super) fn rows(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "row.insert",
            "插入行",
            "rowins",
            "",
            "行",
            "insert row blank 插入行 空行",
            need_range,
            |a| a.insert_rows(),
        )
        .modifies(),
        c(
            "row.delete",
            "删除行",
            "rowdel",
            "",
            "行",
            "delete remove row 删除行",
            need_range,
            |a| a.delete_rows(),
        )
        .modifies(),
    ]);
}

fn workbook_commands(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "book.transpose",
            "转置",
            "grid",
            "",
            "工作表",
            "transpose 转置",
            need_book,
            |a| a.transpose_book(),
        )
        .modifies(),
        c(
            "sheet.stack",
            "堆叠列…",
            "grid",
            "",
            "工作表",
            "stack 堆叠",
            need_book,
            |a| a.open_sheet_transform(crate::dialogs::TransformKind::Stack),
        ),
        c(
            "sheet.split",
            "拆分列…",
            "grid",
            "",
            "工作表",
            "split 拆分",
            need_book,
            |a| a.open_sheet_transform(crate::dialogs::TransformKind::Split),
        ),
        c(
            "sheet.reduce",
            "数据精简…",
            "grid",
            "",
            "工作表",
            "reduce decimate 精简",
            need_book,
            |a| a.open_sheet_transform(crate::dialogs::TransformKind::Reduce),
        ),
        c(
            "sheet.freeze",
            "冻结至选中列",
            "grid",
            "",
            "工作表",
            "freeze columns 冻结列",
            need_book,
            |a| a.freeze_selected_columns(),
        ),
        c(
            "sheet.unfreeze",
            "取消冻结",
            "grid",
            "",
            "工作表",
            "unfreeze columns 取消冻结",
            need_book,
            |a| a.unfreeze_columns(),
        ),
        c(
            "book.template.save",
            "保存工作簿模板…",
            "grid",
            "",
            "工作表",
            "workbook template save 模板 保存",
            need_book,
            |a| a.save_workbook_template(),
        ),
        c(
            "book.template.open",
            "从模板新建工作簿…",
            "grid",
            "",
            "工作表",
            "workbook template new 模板 新建",
            always,
            |a| a.open_workbook_template(),
        ),
    ]);
}

pub(super) fn columns(commands: &mut Vec<Command>) {
    workbook_commands(commands);
    commands.extend([
        c(
            "col.fill",
            "填充序列",
            "colins",
            "",
            "列",
            "fill sequence arithmetic random uniform normal 填充 序列 等差 随机数 正态",
            need_book,
            |a| a.open_fill_dialog(),
        ),
        c(
            "col.setvalues",
            "设置列值",
            "colins",
            "Ctrl+Q",
            "列",
            "set column values formula expression calculate 设置列值 公式 计算列 表达式",
            need_book,
            |a| a.open_set_values_dialog(),
        ),
        c(
            "col.insert",
            "插入列",
            "colins",
            "",
            "列",
            "add column",
            need_book,
            |a| a.add_column(),
        )
        .modifies(),
        c(
            "col.delete",
            "删除列",
            "coldel",
            "",
            "列",
            "delete column",
            need_cols,
            |a| a.delete_columns(),
        )
        .modifies(),
        c(
            "col.left",
            "左移列",
            "colleft",
            "",
            "列",
            "move column left 移动列",
            need_cols,
            |a| a.move_columns(scicanvas_core::ColumnMove::Left),
        )
        .modifies(),
        c(
            "col.right",
            "右移列",
            "colright",
            "",
            "列",
            "move column right 移动列",
            need_cols,
            |a| a.move_columns(scicanvas_core::ColumnMove::Right),
        )
        .modifies(),
        c(
            "col.first",
            "列移到最前",
            "colleft",
            "",
            "列",
            "move column first 移动列 最前",
            need_cols,
            |a| a.move_columns(scicanvas_core::ColumnMove::First),
        )
        .modifies(),
        c(
            "col.last",
            "列移到最后",
            "colright",
            "",
            "列",
            "move column last 移动列 最后",
            need_cols,
            |a| a.move_columns(scicanvas_core::ColumnMove::Last),
        )
        .modifies(),
        c(
            "col.categories",
            "类别顺序…",
            "sortasc",
            "",
            "列",
            "category order levels 分类 类别顺序 分类轴",
            need_cols,
            |a| a.open_category_order_dialog(),
        ),
        c(
            "col.clear",
            "清空列",
            "colclear",
            "",
            "列",
            "clear column",
            need_cols,
            |a| a.clear_columns(),
        )
        .modifies(),
    ]);
}

pub(super) fn sort(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "sort.custom",
            "自定义排序…",
            "sortasc",
            "",
            "排序",
            "sort custom multiple keys 多列排序 多关键字 自定义排序 只排选中列",
            need_book,
            |a| a.open_sort_dialog(),
        ),
        c(
            "sort.asc",
            "升序排序",
            "sortasc",
            "",
            "排序",
            "sort ascending",
            need_sortable,
            |a| a.sort_active(true),
        )
        .modifies(),
        c(
            "sort.desc",
            "降序排序",
            "sortdesc",
            "",
            "排序",
            "sort descending",
            need_sortable,
            |a| a.sort_active(false),
        )
        .modifies(),
    ]);
}
