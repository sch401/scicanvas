# Windows MSI 构建

使用 Windows、Rust 工具链、Python 3、.NET SDK 和 WiX 5.0.2。WiX 可安装在临时工具目录：

```powershell
dotnet tool install --tool-path "$env:TEMP\scicanvas-wix5" wix --version 5.0.2
$env:PATH = "$env:TEMP\scicanvas-wix5;$env:PATH"
python tools/build_msi.py
```

脚本先生成并核对发布 ZIP，再将全部许可、示例项目与程序打入 `target/releases/SciCanvas-<版本>-windows-x64.msi`。安装到当前用户的 `%LOCALAPPDATA%\Programs\SciCanvas`，注册 `.scv`、`.scgt`、`.scfn` 并添加开始菜单快捷方式。升级保留恢复数据。交互式卸载时可选择删除 `%LOCALAPPDATA%\SciCanvas`；默认保留。静默卸载默认保留数据。

安装、双击文件、升级和卸载对话框需要在 Windows 上实际操作检查。运行前请关闭 SciCanvas，并使用单独的测试项目。
