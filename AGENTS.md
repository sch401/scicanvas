# SciCanvas 项目安排

- 当前阶段只以 Windows 为开发、测试、性能验收和发布目标。
- CI 使用 Windows runner；Python 集成采用 Windows 嵌入式运行时；安装包优先 Windows。
- macOS、Linux 适配、验证和发布延期到后续阶段，启用时间由用户决定。
- 保持核心库与界面解耦，保留现有跨平台代码；后续平台任务按需启动。
- 实施顺序以 `docs/ROADMAP.md` 为准。当前重点是作图底座和 Windows 科研工作流。
