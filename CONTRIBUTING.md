# 贡献与维护

当前开发、验收和发布目标为 Windows x64。功能安排见 [技术路线](docs/ROADMAP.md)，维护步骤见 [代码维护重构路线](docs/代码维护重构路线.md)。

## 准备环境

- Git，具备本私密仓库的读取权限。
- Visual Studio Build Tools，安装“使用 C++ 的桌面开发”和 Windows SDK，供 MSVC 编译、链接使用。
- Rust stable，包含 rustfmt、Clippy 和 rust-docs。工作区声明的最低 Rust 版本为 1.88；发布脚本需要 rust-docs 中的标准库版权材料。
- Python 3.11 或更新版本，维护脚本使用标准库 `tomllib`，无需 pip 安装。
- Node.js 20 或更新版本，用于 `node --test` 执行网页清单回归。

在 PowerShell 中运行：

```powershell
git clone https://github.com/sch401/scicanvas.git
cd scicanvas
rustup toolchain install stable --profile minimal --component rustfmt --component clippy --component rust-docs
rustup override set stable
cargo fetch --locked --target x86_64-pc-windows-msvc
python --version
node --version
rustc --version
```

本次维护验证环境为 Python 3.14.3、Node.js 24.19.0、Rust 1.99.0。首次检查需要联网下载固定版本的 cargo-deny 和更新 RustSec 告警数据库；第三方声明检查读取 Cargo 缓存中的原始 crate。

## 运行与完整检查

```powershell
cargo run --release --locked -p scicanvas
python tools/check.py
```

完整检查与 [Windows CI](.github/workflows/ci.yml) 对应，包含依赖安全、许可证与来源，第三方版权材料，维护脚本测试，清单同步与 Python/Node 网页回归，Rust 格式、全部特性测试、核心库无默认特性测试和 Clippy。遇到失败先修复对应问题，再运行完整检查。

开发时可以先运行相应检查：

```powershell
cargo fmt --all
cargo test --profile ci -p scicanvas-core --all-features --locked
cargo test --profile ci -p scicanvas --all-features --locked
```

提交前仍运行 `python tools/check.py`。Windows CI 在 PR 更新、`main` 推送、标签推送及手动触发时执行全部检查。开发分支通过 PR 验证，避免同一提交同时触发 push 和 PR；尚未创建 PR 时可手动运行，连续更新仍会取消该分支上过时的检查。

缓存使用固定提交的 [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache)，按实际 Rust 编译器、环境、各工作区清单和锁文件复用依赖。`target/ci` 属于 Cargo 的 `target` 根目录，缓存仅保留依赖产物，省去本项目测试程序及可重新解压的 crate 源码，同键命中后不再重复上传。工具链或依赖变化时会重建相应缓存，首次运行的耗时包含冷构建，评估提速以同配置第二次运行的日志为准。缓存命中只复用构建产物，每次仍执行所有检查和测试；日常开发和 release 构建使用各自配置。

## 文档与功能清单

以仓库内文件为修改源：

| 内容 | 修改源 | 生成或导出结果 |
| --- | --- | --- |
| 第二阶段技术路线 | `docs/ROADMAP.md` | 可导出为 `ROADMAP.md` 和 `SciCanvas 技术路线（第二阶段）.md` |
| 第三阶段技术路线 | `docs/SciCanvas 功能完备性分析与技术路线.md` | 可导出为同名文件 |
| 维护重构进度 | `docs/代码维护重构路线.md` | 直接维护原文件 |
| 功能状态与子项进度 | `docs/parity/parity.json` | `docs/parity/PARITY.md`、`docs/parity/parity.html` |
| 清单网页样式与交互 | `docs/parity/tracker_template.html` | 同上 |

清单由现有 `docs/parity/build_parity.py` 生成，统一入口只调用它，不另做一套生成逻辑：

```powershell
python tools/sync_docs.py
python tools/sync_docs.py --check
```

默认只生成仓库内的清单文档和网页，`--check` 只核对。功能状态和子项计数写回 JSON，已有编号保持稳定，新增条目追加编号。生成的 Markdown/HTML 通过生成器更新。

需要把路线和清单更新到仓库旁的工作目录时，明确指定目录：

```powershell
python tools/sync_docs.py --desktop-dir ..
python tools/sync_docs.py --check --desktop-dir ..
```

这会导出上表中的两份路线、第二阶段路线的 `ROADMAP.md` 别名，以及 `SciCanvas 对照清单.html`，共四个文件。导出方向为仓库到指定目录；若在副本中编辑过内容，先把改动合回仓库源文件再导出。检查模式报告缺失或过期副本，并保留现有文件。

旧入口 `python docs/parity/build_parity.py --desktop` 继续可用，只更新仓库及仓库旁的网页清单。完整检查和 CI 使用新的统一检查入口，均不导出桌面副本。

## 依赖与第三方版权材料

修改依赖后保留并提交 `Cargo.lock`，运行：

```powershell
cargo fetch --locked --target x86_64-pc-windows-msvc
python tools/third_party.py
python tools/third_party.py --check
python tools/check.py
```

第三方脚本从原始 crate 生成版权声明、字体声明、许可证和离线 MPL 源码，生成结果一起提交。依赖政策与许可例外见 [依赖说明](docs/DEPENDENCIES.md)，材料来源记录见 [版权检查记录](docs/COPYRIGHT_AUDIT.md)。

## Windows 发布候选包

先完成检查并提交所需改动，再用现有发布脚本打包：

```powershell
python tools/package_release.py
```

脚本构建 Windows x64 release exe，检查第三方材料，将 exe、许可证、版权声明、离线 MPL 源码和当前工具链的标准库声明装入 `target/releases/SciCanvas-<版本>-windows-x64.zip`，并输出 ZIP 的 SHA-256。包内 `BUILD_INFO.json` 记录提交、工作区是否有修改、工具链、exe 和锁文件哈希。发布前核对该信息并试运行包内程序；向他人分发时保留完整 ZIP 内容。

此脚本只生成本地候选包。GitHub Release 的发布另行操作。

## 提交约定

- 修改前读取最新文件，保持已有用户改动。维护重构按路线每步单独提交，提交信息用英文，新增注释用中文。
- 保留功能区命令顺序、控件 ID、界面布局和文件格式。绘图继续共用页面版面，图层显式访问，项目修改使用现有撤销事务。
- 新功能按 PR 模板同时提供核心回归测试和真实界面的一步撤销/重做测试；影响绘图时再提供 SVG golden 或 PNG/PDF 导出测试。纯维护改动在不适用项说明原因，并保留原断言和基线。
- 19 份 SVG 基线以及 PNG/PDF、旧项目迁移和往返测试持续验证。改变图形结果时说明原因，单独核对基线。
- 文件格式变更遵循 [项目格式说明](docs/PROJECT_FORMAT.md)，同步版本与迁移测试。功能进度同时更新路线和清单数据源。
- PR 写明具体变化、验证结果和剩余范围，并确认对应提交的 Windows CI 通过。
