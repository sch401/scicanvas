# MSI 人工验收

使用独立 Windows 测试账户或虚拟机。两个包分别是 `SciCanvas-0.1.0-windows-x64.msi` 和 `SciCanvas-0.1.1-upgrade-test.msi`；后者只更改安装包版本，用于检查升级流程，程序内容与候选版相同。

| 步骤 | 操作 | 预期结果 | 结果 |
|---|---|---|---|
| 1 | 在测试账户的 `%LOCALAPPDATA%\SciCanvas\recovery` 创建 `msi-test.txt`，写入 `keep` | 标记文件存在 | |
| 2 | 安装 0.1.0 候选包 | 程序安装到当前用户的 `%LOCALAPPDATA%\Programs\SciCanvas`，开始菜单有 SciCanvas | |
| 3 | 双击安装目录 `samples` 中的一个 `.scv` 项目，打开“帮助 → 关于 SciCanvas” | 项目打开；关于窗口显示版本、提交和图形后端 | |
| 4 | 安装 0.1.1 升级测试包 | 安装完成，`msi-test.txt` 内容仍为 `keep` | |
| 5 | 卸载时保持“删除本机恢复数据”未勾选 | 程序卸载，`msi-test.txt` 保留 | |
| 6 | 再安装候选包，卸载时勾选“删除本机恢复数据” | 测试账户的 `%LOCALAPPDATA%\SciCanvas` 被删除 | |

记录 Windows 版本、两份安装包的 SHA-256、日期、通过或未通过的步骤，并保留安装和卸载窗口截图。使用测试账户，避免影响日常账户中的恢复文件。
