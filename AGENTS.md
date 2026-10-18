# SciCanvas 项目安排

- 当前阶段只以 Windows 为开发、测试、性能验收和发布目标。
- CI 使用 Windows runner；Python 集成采用 Windows 嵌入式运行时；安装包优先 Windows。
- macOS、Linux 适配、验证和发布延期到后续阶段，启用时间由用户决定。
- 保持核心库与界面解耦，保留现有跨平台代码；后续平台任务按需启动。
- 工作表按独立窗口组织：一个 Book 对应一张表，不做一个 Book 多张 Sheet 或底部 Sheet 标签页；Excel 多工作表导入时拆成多个独立 Book。（用户 2026-10-18 明确要求）
- 实施顺序以 `docs/ROADMAP.md` 为准。当前重点是作图底座和 Windows 科研工作流。
- 屏幕、图库与出版导出使用 `scene::layout_page` 的固定页面画布，显示层只做整体缩放。
- 项目修改使用 `History::begin` / `changed` / `commit` 事务；命令在定义旁标记 `.modifies()`，连续编辑复用快照。
- 图层访问显式指定目标；需要第一层时使用命名明确的 `first_layer()` / `first_layer_mut()`。
