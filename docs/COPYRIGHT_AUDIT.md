# 代码与素材版权来源检查

检查日期：2026-10-18。源码基准：d928ad3；同时保留本地正在更新的技术路线和清单。
范围为当前 Git 工作区源码、内联 SVG/网页、测试和示例、Cargo.lock 的 Windows x64 normal/build 依赖及实际打包内容。检查方法是文件清点、源码/注释与提交线索核对、依赖原始归档和许可原文检查；这次记录不替代外部授权凭证或发行地区的法律审查。

## 已发现并补齐的发行材料

| 项目 | 检查结果及处理 |
| --- | --- |
| 应用自有部分 | Cargo.toml 原有 MIT OR Apache-2.0；补 LICENSE-MIT / LICENSE-APACHE，第三方部分保留原许可，不以应用许可重新覆盖 |
| Rust 包 | 从实际 Windows 构建树列出 187 个第三方包（含构建期工具）；从 Cargo.lock 校验原始 .crate，保留原始许可、NOTICE 和源码版权头。完整索引与每文件校验和见 licenses/THIRD_PARTY_INDEX.json |
| 未随 crate 提供的许可文本 | 按 .cargo_vcs_info.json 指向的同一提交取得 egui、AccessKit、clipboard-win、enum-map、gl-rs 的原文；URL/SHA-256 见 licenses/supplemental/sources.json，保留 REUSE 文件级版权 |
| MPL 2.0 | resvg、usvg、usvg-parser、usvg-text-layout、usvg-tree 各为 0.37.0；原始完整源码随包提供，校验和与 Cargo.lock 一致；检查本地 MPL 文件与原包相同，见 MPL_SOURCE.md |
| egui 内嵌字体 | Ubuntu Light、Noto Emoji、Hack、emoji-icon-font；保留 UFL、OFL、MIT 原文，以及 Hack 附带的 Bitstream Vera/DejaVu 声明。额外提取字体 name 表原始版权、商标与许可，见 licenses/FONT_NOTICES.md |
| 系统字体 | fonts.rs 和导出模块按路径/系统字体库读取微软雅黑、Arial 等；发布脚本仅复制白名单文件，不复制 Windows 字体、系统 DLL 或外部 SCICANVAS_FONT 文件 |
| Rust 标准库 | 每次构建包附实际 rustc 对应的 COPYRIGHT-library.html 与其引用的许可证目录，构建版本记入 BUILD_INFO.json；该目录包含标准库分发材料的许可集合，不等同每种许可均用于应用业务代码 |
| 示例和测试资产 | 14 份 .scv 的生成代码在 tests/plot_samples.rs，使用公式/常数生成数据和自身 SVG；sample.xlsx 测试断言记录为 openpyxl 生成的短表格。打包不夹带这类开发资产 |

THIRD_PARTY_NOTICES.md 是用户入口；发行候选包同时附许可证和五个 MPL 原始源码归档，接收者离线即可取得源码，私密 GitHub 仓库的权限不影响获取。

## A6 字体复用补充（2026-10-22）

共享度量模块直接复用既有 ab_glyph、ttf-parser 和 epaint_default_fonts 包及原始字体字节。未导入外部字体素材，Ubuntu/Hack/Noto Emoji 的原始声明继续由 third_party.py 提取并随发布包保留。系统 Arial/微软雅黑等只在内存中用于测量、显示及栅格化，SVG 引用字体名称与字宽，未把系统字体嵌入文件或拷入包中。

## A6 PDF 字体嵌入补充（2026-10-23）

PDF 使用 svg2pdf 0.13.0 及其锁定依赖。新增 0.45 系列 resvg/usvg 使用包内声明的 MIT OR Apache-2.0，不扩大 MPL 例外；既有 0.37 MPL 包继续保留完整源码归档。全部许可、原始声明和校验值随实际 Windows 依赖树生成。clipboard-win 5.4.1 被上游撤回后更新为 5.4.3，补充许可来源指向新 crate 的 VCS 提交，BSL-1.0 原文及 SHA-256 已核对。

与此前 SVG/PNG 路径不同，PDF 文档会嵌入实际使用字体的子集。程序先读取字体 OpenType OS/2 fsType，通过 ttf-parser 检查权限、轮廓嵌入与子集化限制，拒绝受限、仅位图、禁止子集或无效权限的字体。程序不修改系统字体文件；exe 和发布包仍不复制 Windows 或用户外部字体。检查依据见 [Microsoft OpenType fsType 规范](https://learn.microsoft.com/typography/opentype/spec/os2#fstype)。实际导出已检查 Arial 四种样式、微软雅黑常规/粗体的字体流及 Unicode 映射，系统字体对应的使用条款继续适用。

## 来源记录与发布边界

2026-10-18 用户确认：图标/网页原型及 Shapiro–Wilk 检验由 AI 生成，用户未向 AI 提供外部代码或素材。以下结合这条开发记录与仓库证据说明范围。

### 1. 图标与网页原型

`crates/scicanvas/src/icons.rs` 为内联 SVG 路径，注释表明与网页原型共用。提交线索可追到初始导入 f928fec。

图标和原型按“AI 生成、用户未提供外部素材；界面结构独立设计”登记；当前代码和提交线索未指向具体第三方图标集，因此没有编造某个图标库的署名或授权。未来若引入外部图标，需登记真实来源及许可。对外宣传使用 SciCanvas 自身标识。

### 2. Shapiro–Wilk / AS R94 及数值实现

`crates/scicanvas-core/src/stats.rs` 引用 Royston 1995 AS R94；其系数与分段计算流程同公开算法及 SciPy 的 swilk。已对照 SciPy v1.17.1 官方源码，当前 Rust 实现的函数组织、正态分位数求解等与 SciPy 实现有差异。算法/数学系数的共同来源与具体代码改写的来源分别核对。

用户确认未提供 SciPy/R 源码；现有记录为 AI 生成，仓库没有记录具体代码移植来源。当前可确认的是 AS R94 算法引用、共享数值系数及 SciPy 固定参考值测试。本次保守保留 SciPy BSD-3-Clause 原文和来源 URL，作为来源敏感例程的补充声明；若以后发现实际改写自其他代码的证据，应按真实来源处理，SciPy 声明只适用于其许可范围。

其他数值代码中“与 SciPy/NumPy 一致”的注释主要连到测试参考值；这类注释本身不记录是否移植代码。继续维护代码时应记录“参考算法 / 数值对照 / 移植代码”三种不同关系，并保留实际移植源的版权与许可。

### 3. 产品名称与界面表达

项目使用 SciCanvas 自身名称。色表和内置主题采用中性名称，内部旧标识仅用于文件兼容。引入第三方代码、字体和素材时，按实际来源保留版权、许可证与发行材料。

## 复现与后续维护

1. `cargo fetch --locked --target x86_64-pc-windows-msvc` 准备原始包缓存。
2. `python tools/third_party.py` 生成声明、原文、字体说明及 MPL 源码归档；`--check` 检查同步。生成过程离线使用原包并验证 SHA-256。
3. `python tools/check.py` 运行依赖审计、声明与清单检查、现有 Rust 检查；`python -m unittest discover -s tools -p "test_*.py"` 验证许可材料。
4. `python tools/package_release.py` 重新构建并生成 `target/releases/SciCanvas-0.1.0-windows-x64.zip`。向他人分发完整包，保留 MPL_SOURCE.md、THIRD_PARTY_NOTICES.md、licenses 和构建信息。

原文依据：[MPL 2.0 第 3 节](https://www.mozilla.org/en-US/MPL/2.0/)、[Mozilla FAQ 第 8–10 问](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)、[OFL 原文](https://openfontlicense.org/open-font-license-official-text/)、[Ubuntu 字体许可](https://canonical.com/legal/font-licence)、[Microsoft 字体分发说明](https://learn.microsoft.com/en-us/typography/fonts/font-faq)、[SciPy v1.17.1 许可](https://github.com/scipy/scipy/blob/v1.17.1/LICENSE.txt)。


## 依赖与图标栈升级补充（2026-10-25）

PNG、PDF 与 SVG 图标现共用 resvg/usvg 0.45.1，rustybuzz 与 fontdb 各只有当前单一版本。Cargo.lock 不再包含 0.37 解析器，也不含 rustybuzz 0.12.1。该版渲染库声明为 MIT OR Apache-2.0，按原始包分别保留许可证与源码版权头，索引由 Cargo.lock 生成。

更新后的 Windows 完整特性依赖索引中没有 MPL-2.0 包，MPL 源码归档不会打入本版发布包。仓库保留的 0.37 源码和声明是历史版本档案，说明见 MPL_SOURCE.md。发布脚本仅在当前锁定依赖含 MPL 包时才附源码说明和对应完整归档。

图标 loader 保留 egui_extras 0.29.1 的 URI、尺寸提示、缓存和颜色换算行为，原 MIT 声明已随依赖记录；全套图标/缩略图逐像素比对一致。16份图型/科学文字/多面板PNG逐像素与升级前一致；屏幕字形位置和PDF矢量/字体子集回归均通过。

## A1 统计检验算法与表值（2026-11-04）

Lilliefors 正态临界表和渐近参数采用 statsmodels 0.14.6 的 `stats/_lilliefors_critical_values.py`，表插值规则对照 `_lilliefors.py` 和 `tabledist.py`。版权原文保留于 `licenses/supplemental/statsmodels-0.14.6/LICENSE.txt` 并加入分发声明。AD 的 p 值使用 D’Agostino–Stephens 公式，对照 statsmodels `normal_ad`。KS 双样本精确概率及有限样本 kstwo 的 Durbin、Pelz–Good 分支对照 SciPy 1.17.1，沿用仓库保留的 SciPy BSD 许可。参考库仅用于开发机生成测试数据。

Dixon 临界值取自 [Rorabacher 原文](https://community.jmp.com/kvoqx44227/attachments/kvoqx44227/discussions/48839/1/Rorabacher%201991%20-%20Critical%20Values%20of%20Dixon%20Test.pdf)，Analytical Chemistry 63 (1991), 139–146，表 I、II、V、VI 的推荐 n 区间。数值表与算法代码区分，未复制第三方 Dixon 实现。

独立抽查使用 [CRAN outliers 的 qdixon 原始表](https://github.com/cran/outliers/blob/master/R/qdixon.R)。该表为单侧概率，双侧 α 对应 α/2。下列八个值与原文逐项一致，并写入回归测试：

| n | 比值 | 双侧 α | 临界值 |
|---|---|---|---|
| 3 | r10 | 0.10 | 0.941 |
| 5 | r10 | 0.05 | 0.710 |
| 7 | r10 | 0.01 | 0.680 |
| 8 | r11 | 0.05 | 0.615 |
| 11 | r21 | 0.05 | 0.625 |
| 14 | r22 | 0.01 | 0.674 |
| 20 | r22 | 0.05 | 0.491 |
| 30 | r22 | 0.01 | 0.483 |
