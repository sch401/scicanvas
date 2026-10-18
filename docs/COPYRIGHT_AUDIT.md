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

## 来源记录与发布边界

2026-10-18 用户确认：图标/网页原型及 Shapiro–Wilk 检验由 AI 生成，用户未向 AI 提供外部代码或素材。以下结合这条开发记录与仓库证据说明范围。

同日用户进一步说明：Origin CHM 仅用于了解和整理需要实现的功能；具体代码和界面结构均为新设计，未参考 Origin 的代码或界面结构。本记录据此将开发方式登记为“功能需求参照、代码与界面独立实现”，与复制原软件的程序、界面或素材分别处理。

### 1. 图标与网页原型

`crates/scicanvas/src/icons.rs` 为内联 SVG 路径，注释表明与网页原型共用。提交线索可追到初始导入 f928fec；本次文件清点未检出 Origin 安装文件、CHM 正文、Origin 标志图片或随包的商业图标集。

图标和原型按“AI 生成、用户未提供外部素材；界面结构独立设计”登记；当前代码和提交线索未指向具体第三方图标集，因此没有编造某个图标库的署名或授权。未来若引入外部图标，需登记真实来源及许可。对外宣传使用 SciCanvas 自身标识。

### 2. Shapiro–Wilk / AS R94 及数值实现

`crates/scicanvas-core/src/stats.rs` 引用 Royston 1995 AS R94；其系数与分段计算流程同公开算法及 SciPy 的 swilk。已对照 SciPy v1.17.1 官方源码，当前 Rust 实现的函数组织、正态分位数求解等与 SciPy 实现有差异。算法/数学系数的共同来源与具体代码改写的来源分别核对。

用户确认未提供 SciPy/R 源码；现有记录为 AI 生成，仓库没有记录具体代码移植来源。当前可确认的是 AS R94 算法引用、共享数值系数及 SciPy 固定参考值测试。本次保守保留 SciPy BSD-3-Clause 原文和来源 URL，作为来源敏感例程的补充声明；若以后发现实际改写自其他代码的证据，应按真实来源处理，SciPy 声明只适用于其许可范围。

其他数值代码中“与 SciPy/NumPy 一致”的注释主要连到测试参考值；这类注释本身不记录是否移植代码。继续维护代码时应记录“参考算法 / 数值对照 / 移植代码”三种不同关系，并保留实际移植源的版权与许可。

### 3. Origin 对照目录、名称及界面表达

根据用户说明，CHM 作为功能需求参考，代码与界面结构独立设计。项目的开发清单仍保留 Origin 帮助章节标题以便查阅；发布脚本不包含完整对照目录、Origin CHM 或截图。若以后公开转载 CHM 原文、截图或完整目录集合，再核对相应文档/汇编授权；以自己的语言说明软件功能与转载原文分别记录。

当前程序使用 SciCanvas 名称。Origin、Nature 等名称在开发资料中说明功能参考对象或样式，不表示授权或关联。独立实现的来源记录与具体第三方库/字体许可分别保留；软件名称的商标使用及具体发行地区规则另行核对。

以中国软件著作权规则为例，《计算机软件保护条例》第六条将思想、处理过程、操作方法和数学概念排除在软件著作权保护范围之外；程序、文档及其他具体表达按各自权利处理。依据见[条例原文](https://www.cac.gov.cn/2013-02/08/c_126468744.htm)。这支持按功能需求独立实现的开发路径，第三方依赖和字体的发行义务继续适用。

## 复现与后续维护

1. `cargo fetch --locked --target x86_64-pc-windows-msvc` 准备原始包缓存。
2. `python tools/third_party.py` 生成声明、原文、字体说明及 MPL 源码归档；`--check` 检查同步。生成过程离线使用原包并验证 SHA-256。
3. `python tools/check.py` 运行依赖审计、声明与清单检查、现有 Rust 检查；`python -m unittest discover -s tools -p "test_*.py"` 验证许可材料。
4. `python tools/package_release.py` 重新构建并生成 `target/releases/SciCanvas-0.1.0-windows-x64.zip`。向他人分发完整包，保留 MPL_SOURCE.md、THIRD_PARTY_NOTICES.md、licenses 和构建信息。

原文依据：[MPL 2.0 第 3 节](https://www.mozilla.org/en-US/MPL/2.0/)、[Mozilla FAQ 第 8–10 问](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)、[OFL 原文](https://openfontlicense.org/open-font-license-official-text/)、[Ubuntu 字体许可](https://canonical.com/legal/font-licence)、[Microsoft 字体分发说明](https://learn.microsoft.com/en-us/typography/fonts/font-faq)、[SciPy v1.17.1 许可](https://github.com/scipy/scipy/blob/v1.17.1/LICENSE.txt)。
