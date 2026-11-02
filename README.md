# SciCanvas

SciCanvas 是一款面向科研数据处理、拟合和绘图的桌面软件，使用 Rust 开发。当前处于 0.1 开发阶段，以 Windows x64 为开发和验收平台。

## 主要功能

- **工作表**：导入 CSV、文本和 Excel 数据，编辑列属性、公式、排序、查找替换，支持转置、堆叠、拆分和抽取。筛选可按数值范围、文字或类别隐藏行，源数据保留。一个工作表对应一个独立窗口，Excel 多工作表导入后分别成为独立工作表。
- **科研绘图**：提供折线、散点、柱状、堆叠柱、条形、面积、阶梯、样条和垂线等 14 种图型，支持误差棒、多图层、双 Y 轴、文本标注和图形模板。图型预览随窗口宽度调整，展开菜单可查看全部图型。
- **数据分析**：提供线性和多项式拟合、非线性拟合、平滑、插值、微分、积分、FFT 和常用统计分析。非线性拟合支持参数约束和自定义函数，报告展示参数、标准误、拟合状态和相关提示。
- **项目与结果**：保存 `.scv` 项目，在项目中管理数据表、图形和 Note。拟合结果可以保存为报告或 Note，支持撤销与重做。
- **导出与批量操作**：工作表可导出 CSV 和 Excel，图形可导出 SVG、PDF、PNG 和 TIFF。支持从模板批量绘图和批量导出图形。

功能进度与验收状态见 [功能对照清单](docs/parity/PARITY.md)，也可打开 [离线清单网页](docs/parity/parity.html)。

## 快速上手

1. 打开 SciCanvas，导入数据或创建工作表。
2. 检查 X、Y 列及单位，选中需要绘图的列。
3. 在“绘图”中选择图型，或通过“作图面板与图库”设置数据映射。
4. 在“分析”中选择拟合或其他计算，在图形窗口调整样式。
5. 保存 `.scv` 项目，或导出所需的数据和图片。

[samples](samples/README.md) 提供各图型的示例项目和数据排列说明。常用快捷键包括 `Ctrl+S` 保存项目、`Ctrl+Z` 撤销、`Ctrl+K` 搜索命令，以及 `Ctrl+F1` 折叠功能区。

## 构建和运行

需要 Windows x64、Rust 1.88 或更新版本，以及带 Windows SDK 的 Visual Studio C++ 构建工具。首次构建需要下载依赖。

```powershell
git clone https://github.com/sch401/scicanvas.git
cd scicanvas
cargo run --release --locked -p scicanvas
```

也可以先编译，再双击运行 `target/release/scicanvas.exe`：

```powershell
cargo build --release --locked -p scicanvas
```

程序自动加载 Windows 中文字体。如需指定字体，可在运行前设置 `SCICANVAS_FONT`，例如 `C:\Windows\Fonts\msyh.ttc`。

Windows MSI 安装包的构建方法见 [安装包说明](installer/README.md)。安装、升级、文件关联及卸载界面仍待人工验收，操作步骤见 [MSI 人工验收清单](installer/manual-acceptance.md)。

## 文件格式

`.scv` 是项目容器，包含项目数据、表格和图形预览。当前项目数据格式为 v23，可读取 v0 至 v23 的项目；旧版程序无法打开新保存的 v23 项目。`.scfn` 用于交换自定义拟合函数，`.scgt` 用于图形模板。格式说明和迁移记录见 [项目文件格式](docs/PROJECT_FORMAT.md)。

## 开发与验证

代码分为 `crates/scicanvas-core` 核心库和 `crates/scicanvas` 桌面应用。核心库负责数据、计算和图形场景，桌面应用负责界面与交互。

```powershell
cargo test --profile ci -p scicanvas-core --all-features --locked
cargo test --profile ci -p scicanvas --all-features --locked
python tools/check.py
```

开发环境、完整检查和发布步骤见 [贡献说明](CONTRIBUTING.md)。数值测试结果见 [NIST 拟合对照](docs/parity/nist.md)，人工验收步骤见 [验收清单](docs/acceptance/README.md)，后续安排见 [技术路线](docs/ROADMAP.md)。

## 许可证

项目代码采用 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE) 双许可证。第三方依赖和素材保留各自许可证，详见 [第三方版权与许可证](THIRD_PARTY_NOTICES.md)。
