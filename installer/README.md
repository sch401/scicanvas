# Windows MSI 构建

使用 Windows、Rust 工具链、Python 3、.NET SDK 和 WiX 5.0.2。WiX 可安装在临时工具目录：

```powershell
dotnet tool install --tool-path "$env:TEMP\scicanvas-wix5" wix --version 5.0.2
$env:PATH = "$env:TEMP\scicanvas-wix5;$env:PATH"
python tools/build_msi.py
```

脚本先生成并核对发布 ZIP，再将全部许可、示例项目与程序打入 `target/releases/SciCanvas-<版本>-windows-x64.msi`。安装界面使用固定版本的 WiX UI 标准中文向导，提供欢迎、许可、路径选择、安装确认、进度、完成及取消确认页面，采用 SciCanvas 自主品牌图像。点击“完成”关闭向导。同版本重新打包可更新已安装的候选包。安装到当前用户的 `%LOCALAPPDATA%\Programs\SciCanvas`，注册 `.scv`、`.scgt`、`.scfn` 并添加开始菜单快捷方式。升级保留恢复数据。交互式卸载时可选择删除 `%LOCALAPPDATA%\SciCanvas`；默认保留。静默卸载默认保留数据。

安装、双击文件、升级和卸载对话框按 [MSI 人工验收](manual-acceptance.md) 操作。使用独立 Windows 测试账户或虚拟机，并在运行前关闭 SciCanvas。

首次 MSI 构建会把 UI 扩展及对应源码下载到忽略的 `target/tools/wix-ui`。界面组件的原始许可证和对应源码随 MSI 保存在 `licenses/wix-ui`；便携 ZIP 不包含该安装界面。
