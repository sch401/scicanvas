# Windows 依赖检查

`python tools/check.py` 会安装并运行固定的 cargo-deny 0.20.2，随后检查清单同步、子项显示和 Rust 代码。
安装包来自 [官方 release](https://github.com/EmbarkStudios/cargo-deny/releases/tag/0.20.2)，SHA-256 固定在安装脚本中；工具和下载缓存位于忽略的 `target/tools/cargo-deny`。校验失败立即停止。
Windows CI 使用同一脚本和 `deny.toml`，审计失败会使检查失败。

审计范围是当前发布目标 `x86_64-pc-windows-msvc` 的完整特性依赖图，使用 `Cargo.lock`。
检查 RustSec 告警、撤回版本、许可证、通配符依赖和未知 Git/注册表来源。
已有依赖多版本作为警告列出；漏洞、未允许的许可证与未知来源为错误。
GPL/LGPL 未列入允许列表。新增直接依赖继续优先 MIT / Apache-2.0 / BSD。

以下是现有界面的限定包版本许可例外，升级版本时重新核对。许可文本可在对应 crates.io 包中查阅，发行打包时需保留适用的许可与字体声明。

| 包与版本 | 例外许可 | 当前用途 |
| --- | --- | --- |
| clipboard-win 5.4.1、error-code 3.4.0 | BSL-1.0 | Windows 剪贴板 |
| epaint_default_fonts 0.29.1 | OFL-1.1、LicenseRef-UFL-1.0 | egui 自带字体 |
| resvg / usvg / usvg-parser / usvg-tree 0.37.0 | MPL-2.0 | egui SVG 图标 |
| usvg-text-layout 0.37.0 | MPL-2.0 | PNG 导出 / 复制图形的文字排版 |

核心库的 `serde` 特性使用 zip 9.x（MIT；4.x 有 RUSTSEC-2026-0369 等告警，因此工作区最低 Rust 版本升到 1.88）读写 `.scv`，只启用 deflate，并通过 flate2 选用纯 Rust 的 miniz_oxide 后端（与界面依赖相同），不引入加密或其他压缩算法。

界面层的 PNG 导出和复制图形直接依赖 resvg 0.37（与 egui 图标同一版本，MPL-2.0 例外已覆盖），额外开启 `text` 与 `system-fonts`，引入 fontdb、rustybuzz、ttf-parser 等 MIT / Apache-2.0 排版库；剪贴板位图用 arboard 3.6 的 `image-data`，复用依赖树里已有的 image 0.25（Windows 只编译 PNG / BMP 编码）。

基础白名单还包含现有传递依赖使用的 ISC、Zlib、Unicode-3.0。
上述字体和 MPL 例外按包及精确版本配置，其他包使用这些许可时仍需单独核对。

现有依赖有三条停止维护通知，`deny.toml` 按告警编号列明原因；漏洞告警的忽略列表为空。

- [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html)：paste 1.0.15，经 accesskit_windows 0.23.2 引入；保留 egui 无障碍支持，在 eframe 升级时复查替代。
- [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html)：ttf-parser 0.25.1，经 owned_ttf_parser / ab_glyph 引入；A6 字体迁移时评估 skrifa。
- [RUSTSEC-2026-0206](https://rustsec.org/advisories/RUSTSEC-2026-0206.html)：rustybuzz 0.12.1，经 usvg-text-layout 0.37 引入，只用于 PNG 导出和复制图形时的文字排版；随 T1.5 / A6 字体栈迁移一起复查。

这三条通知均无已修复版本。新告警仍会触发审计；已失效的告警例外也会触发检查，提醒清理配置。
配置语义见 [cargo-deny 官方文档](https://embarkstudios.github.io/cargo-deny/checks/cfg.html)。
