# SciCanvas 架构说明

本页说明主程序中数据、撤销和绘图的主要流向。开发与验收目标为 Windows。界面运行在 `scicanvas`（eframe/egui），数据模型与计算集中在不依赖界面的 `scicanvas-core`。

## 模块

`scicanvas-core` 的 `model`、`data`、`page` 和 `plotspec` 定义项目、类型化列、工作簿、图形、图层与曲线映射。`ops/` 按来源解析、拟合、平滑、微积分、基线、FFT、重算和结果列拆分分析操作；`nlfit/model` 与 `nlfit/guess` 分别定义非线性模型和自动初值。`csv`、`xls`、`xlsx` 负责文本及 Excel 导入。`project_file` 管迁移和保存，`history` 管撤销重做。

`scene` 生成以页面为单位的图元，`scene/layout` 排列图层并对齐边距，`scene/legend` 计算图例避让，`svg` 序列化图元。图标 SVG 由 `icons/loader` 使用共享 resvg 栈加载。`scicanvas` 的 `app` 管应用状态；`app/analysis_ops`、`app/workspace`、`app/worksheet`、`app/windows` 分担分析命令、项目树、表格和图形窗口。`commands` 与 `ribbon` 接入菜单和功能区，`dialogs` 管设置及报告，`plot` 负责绘图区交互，`export` 负责 PNG/PDF。

## 数据与绘图

CSV、Excel 或剪贴板输入转成 `Book`，每个 `Book` 是一张独立工作表。图层中的 `Plot` 通过 `DataRef` 指向实际工作簿和列。`scene::layout_page` 根据列数据和图形设置生成统一页面图元：界面绘制这些图元，SVG 导出也序列化这些图元；PNG 对 SVG 栅格化，PDF 将 SVG 转成矢量并嵌入获准的字体子集。修改图形前须与升级前后的 SVG/PNG/PDF 基线比较。

## 编辑与撤销

项目编辑经 `SciApp::edit`、`edit_action_with` 或 `edit_grouped_with` 写入 `History`。事务开始时保存一份完整项目快照，连续控件编辑共用快照；只有项目确实变化才提交。失败可回滚，成功编辑通过同一步撤销。窗口几何、当前选择和滚动位置属于界面会话状态，不进入项目快照。

## 文件兼容和检查

当前项目数据版本为 v15，`.scv` 容器版本为 1；读取时迁移 v0–v14，再校验并原子保存。新增持久字段时递增 `PROJECT_VERSION`、提供旧版默认值/迁移，并补旧版读取、当前版本往返及损坏输入测试；不能仅靠更新示例文件。

图形模板使用独立的 `.scgt` JSON 格式，不包含工作簿数据或源编号；套用时按目标工作簿的列短名重绑定并作为新图加入项目。

提交前运行 `python tools/check.py`。Windows CI 同时检查依赖许可/来源、文档同步、格式、全部特性与核心库无默认特性测试、Clippy。格式或渲染修改还须核对 SVG/PNG/PDF 基线；修改事务时核对一步撤销/重做。
