# SciCanvas 功能完备性分析与技术路线（第三阶段）

> 分析基准：仓库 `scicanvas` @ `ad11c63`（14 个提交，Rust 13,446 行）；对照清单 `docs/parity/parity.json`（488 条，与 `SciCanvas 对照清单.html` 一致）；Origin 自带帮助 `Origin.chm`（目录 1199 项）、`X-Function.chm`（615 个 X-Function）、内置拟合函数库（239 个显函数 + 5 隐函数 + 22 曲面 + 7 峰函数）。
> 读者：负责写代码的 GPT/Codex。本文的第 7 节是可以直接分派的任务卡。


## 实施记录（2026-10-16）

本文件作为第三阶段的需求与任务卡来源；`docs/ROADMAP.md` 保留历史进度并以本节更新实施顺序。当前只做 Windows。

| 任务 | 状态 | 本轮结果与剩余验收 |
| --- | --- | --- |
| T0.1 SVG 回归基线 | 已实现待验收 | 14 种预设 + X/Y 误差棒 + X/Y 标签 + 透明插图 + 双面板，共 18 份字节基线；正常测试只读，显式忽略测试更新；人工图像审阅与 Origin 对照另行执行 |
| T0.2 cargo deny | 未开始 | 依赖许可与安全检查待接入，不标为已完成 |
| T0.3 清单新增项与子项进度 | 未开始 | 先保留已有 488 条和重新核对后的状态；新增条目另行实现 |
| T0.4 性能基准 | 部分实现 | benches/data_pipeline.rs 已测十万/百万行十列的导入、单曲线排版、整表排序；docs/perf.md 记录结果；完整首帧与峰值内存验收待补 |

统一检查入口为 `python tools/check.py`，调用 Windows CI 的格式、完整测试、无默认特性测试和 Clippy。现有 Windows CI 自动运行新增 golden 测试。基线更新命令和审阅要求见 `crates/scicanvas-core/tests/golden/README.md`。

后续按 T0.2 / T0.3 → A1 类型化列与 v9 迁移 → A3 可组合图型 → A2 同层多曲线/多轴 → A6 字体与出版导出推进。本文列出的 v9–v12 均为规划版本；当前项目格式保持 v8。

---

## 1. 结论

**现在还不完备。代码质量好，但功能覆盖大约只有 14%。**

- 代码健康：core 单元测试 45 个（开/关 feature 都通过），集成测试 59 个，UI 测试 29 个，`clippy -D warnings` 无告警。上次审查提出的问题（WYSIWYG、历史每帧比较、图层只能网格、Deref 到第一层、modifies 分离、clippy）都已修好。
- 覆盖度：488 条中 已完成 27、已实现待验收 24、部分实现 16、未开始 410、不做 11。有进展的只占 14%。
- **P0（87 条，"1.0 必须有"）**：完成 27，待验收 12，部分 13，**未开始 35**。P0 都还没收口。
- **图类型**：235 条中只有 17 条有进展（全部是 2D 线/散点/柱/面积/阶梯/样条），饼图、多轴、多面板、统计图、分组图、3D、等高线、专业图、金融图都是 0。
- **分析功能**：拟合 11%、统计 4%、信号 7%、峰分析 / 图像 / 矩阵 0%。

**最关键的判断：不是"再加功能"就能补齐，而是有 4 个底层结构必须先改（第 5 节 A1–A3、A6）。** 现在的数据模型（`Column.data: Vec<String>`、一个 Layer 只有一个 `PlotSpec`、`scene::build_layer` 按 `PlotType` 分支）撑不起双 Y 轴柱+线、分组统计图、3D、百万行数据和公式列。如果继续在现有结构上堆图类型，后面会大面积返工。

建议顺序：**先收口 P0 + 做 A1/A2/A3/A6 四个重构（约 8 周）→ 再按批次铺图类型和分析功能。**

---

## 2. 分模块现状

| 模块 | 条目 | 已完成 | 待验收 | 部分 | 未开始 | 不做 | 有进展占比 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 工作区与项目 | 22 | 7 | 1 | 2 | 10 | 2 | 50% |
| 工作表 | 31 | 4 | 1 | 4 | 22 | 0 | 29% |
| 矩阵 | 8 | 0 | 0 | 0 | 8 | 0 | 0% |
| 导入导出 | 18 | 3 | 0 | 0 | 14 | 1 | 18% |
| 绘图基础 | 36 | 8 | 4 | 6 | 18 | 0 | 50% |
| 图形探索与小工具 | 14 | 0 | 0 | 1 | 13 | 0 | 7% |
| 回归与拟合 | 28 | 0 | 2 | 1 | 25 | 0 | 11% |
| 数学运算 | 16 | 0 | 2 | 1 | 13 | 0 | 19% |
| 统计 | 27 | 0 | 1 | 0 | 26 | 0 | 4% |
| 信号处理 | 15 | 0 | 0 | 1 | 14 | 0 | 7% |
| 峰分析 | 5 | 0 | 0 | 0 | 5 | 0 | 0% |
| 图像处理 | 5 | 0 | 0 | 0 | 5 | 0 | 0% |
| 导出与发布 | 10 | 1 | 0 | 0 | 9 | 0 | 10% |
| 自动化与编程 | 18 | 0 | 0 | 0 | 10 | 8 | 0% |
| 图类型 · 2D 线图 | 7 | 1 | 3 | 0 | 3 | 0 | 57% |
| 图类型 · 2D 散点图 | 13 | 1 | 3 | 0 | 9 | 0 | 31% |
| 图类型 · 2D 点线图 | 7 | 1 | 1 | 0 | 5 | 0 | 29% |
| 图类型 · 2D 柱状图与条形图 | 14 | 1 | 5 | 0 | 8 | 0 | 43% |
| 图类型 · 面积图 | 6 | 0 | 1 | 0 | 5 | 0 | 17% |
| 图类型 · 饼图 / 多轴 / 多面板 / 瀑布 / 统计 / 分组 / 地图 / 专业 / 金融 / 3D×3 / 等高线 / 图像剖面 | 189 | 0 | 0 | 0 | 189 | 0 | 0% |
| **合计** | 488 | 27 | 24 | 16 | 410 | 11 | 14% |

### 2.1 P0 未开始（35 条，1.0 前必须做）

| 领域 | 条目 |
| --- | --- |
| 工作表 | F028 插入/删除行，F032 数字格式，F035 设置列值（公式），F049 多工作表 |
| 导入导出 | F065 导入向导，F066 Excel 导入，F068 CSV 导出，F069 拖放导入 |
| 绘图 | F083 图层内增删曲线，F089 图形模板，F102 富文本标签 |
| 拟合 | F134–F138 非线性拟合（NLFit、初值、参数约束、报告），F142 用户自定义函数，F148–F152 内置函数族 |
| 数学 | F158 插值，F168 列运算 |
| 统计 | F177 频数统计，F185–F187 t 检验 |
| 信号 | F204 FFT |
| 导出 | F227 PNG，F228 PDF，F231 剪贴板 |
| 图类型 | F305 双 Y 轴，F321–F323 多面板 |

### 2.2 P0 部分实现（13 条，需要补到验收）

F026、F027、F038、F053、F096、F097、F109、F113、F115、F116、F130、F166、F201。每条在 parity.json 里都有说明，验收时请按"缺什么补什么"处理，不要另开新结构。

---

## 3. 对照清单相对 CHM 的遗漏

清单在章节层面基本完整，但下面这些在 CHM 里是独立功能，清单里没有或粒度太粗。**建议把这些新增到 parity.json（新 ID 从 F489 起）**：

| 新增项 | CHM 出处 | 建议优先级 |
| --- | --- | --- |
| 分析报告表 + 重算机制（Auto / Manual / None，源数据变化后自动重算） | Origin.chm › Analysis › Recalculating Analysis Results | **P0**（结构性） |
| 分析主题 / 对话框设置保存（Dialog Theme） | Analysis Themes | P1 |
| 批处理（Batch Processing，用分析模板跑多个数据集） | Batch Processing | P1 |
| 参考线、轴断点、轴上的对象（Reference Lines, Axis Break） | Graphing › Axes | P0（参考线）/ P1（轴断点） |
| 每条曲线独立的图类型（同一图层里柱 + 线 + 散点） | Plot Details › Plot Type | **P0**（结构性） |
| 曲线细节：填充图案、符号边/内填充、连线方式（直线/样条/B 样条/阶梯/Bezier）、按列映射颜色/大小/形状（Modifiers） | Plot Details | P0 / P1 |
| Graph Data Slicer、带浏览面板的图（Browser Graph） | Graph Exploration | P2 |
| Data Extraction / Digitizer 小工具 | Gadgets / Digitizer | P2 |
| 图形摘要（Graphical Summary）、并排统计、双样本 KS、Running 非参数检验 | Statistics | P1 |
| 列公式函数库（Origin 内置函数与 Excel 函数对照表，约 300 个） | Set Column Values › Functions | **P0**（F035 的依赖） |
| 日期时间列、分类列（Categorical）及其排序 | Worksheet › Column Properties | **P0**（结构性） |
| 附加文件、OPX 打包分发 | Project › Attach File | P2 |

**粒度问题**：拟合函数 239 个只归成 10 条，导入格式约 40 种只归成 3 条，X-Function 615 个大部分没有单独对应。验收时不能用"条目已完成"代表"能力已完成"。建议在 parity.json 加一个 `sub_items` 计数字段（如 `"拟合函数库": {"total": 239, "done": 0}`），HTML 清单显示子项进度。

---

## 4. 已经做对的部分（保留，不要改）

- core 不依赖 UI；命令注册表 + 定义处 `.modifies()`。
- `scene::layout_page` 屏幕和导出共用，固定页面画布，屏幕只缩放。保持 WYSIWYG。
- `History::begin / changed / commit` 事务。
- 项目文件 v8 + v0–v7 迁移链。以后每次模型改动都必须加一次迁移和测试（第 5 节的重构会让版本走到 v9–v12）。
- 显式 `first_layer()` / `first_layer_mut()`，不再 Deref。

---

## 5. 结构性缺口（必须先做）

每项给出：为什么、设计、接口、依赖 crate、验收标准。A1、A2、A3、A6 是**前置重构**，必须在铺图类型之前完成。

### A1 列式类型化数据模型 + 多工作表 + 矩阵

**为什么**：`Column.data: Vec<String>` 每次画图都要 parse，百万行目标做不到；日期、分类、缺失值都没法表达；公式列、统计分组、3D 都依赖类型。Origin 的 Book 下面是多个 Sheet，矩阵是独立对象。

**设计**：

```rust
// scicanvas-core/src/data/mod.rs
pub enum ColumnData {
    Num { values: Vec<f64>, missing: BitVec },   // NaN 不表示缺失，缺失用位图
    Text(Vec<String>),
    DateTime { values: Vec<i64>, missing: BitVec }, // 毫秒 UTC，显示时按格式转换
    Category { codes: Vec<u32>, levels: Vec<String>, ordered: bool },
}
pub struct Column {
    pub key: String, pub long_name: String, pub units: String, pub comments: String,
    pub role: Role,          // 新增 Z、Label
    pub format: DisplayFormat, // 小数位、科学计数、日期格式、百分比
    pub formula: Option<String>, // F035，见 A4
    pub data: ColumnData,
}
pub struct Sheet { pub name: String, pub columns: Vec<Column> }
pub struct Book  { pub id: BookId, pub name: String, pub sheets: Vec<Sheet>, pub active: usize }
pub struct Matrix { pub id: MatrixId, pub name: String, pub objects: Vec<MatrixObject> }
pub struct MatrixObject { pub nx: usize, pub ny: usize, pub x: (f64,f64), pub y: (f64,f64), pub z: Vec<f64> }
```

- 新增 `trait ColumnView { fn len(); fn f64_at(i) -> Option<f64>; fn text_at(i) -> Cow<str>; }`，画图和分析只通过它取数据。
- 单元格编辑：输入字符串时按列类型 parse；Text 列允许任意内容；Num 列 parse 失败时提示"转为文本列？"。
- 列类型推断：导入 CSV 时扫描前 1000 行决定类型。
- `PlotSpec` 的列引用从 `book_id + x_key` 改为 `DataRef { book: BookId, sheet: usize, col: ColKey }`。
- 文件格式 v9：迁移时把旧 `Vec<String>` 按推断转换；`serde` 下 Num 列用 base64 编码的 f64 小端字节，体积约为文本的 1/3。

**依赖**：`bitvec`（或自写 `Vec<u64>` 位图）、`chrono`（只用于格式化/解析）。暂不引入 arrow，等 1.0 后需要 Parquet 再说。

**验收**：
- 1,000,000 行 × 10 列 Num 数据：导入 < 2 s，散点图首次渲染 < 500 ms，内存 < 150 MB（Windows release 构建）。
- v8 → v9 迁移测试，旧项目打开后图完全一致（对比 SVG 输出）。
- 工作表 UI 能新建/删除/重命名 Sheet，标签在底部。
- 现有 133 个测试全部通过。

### A2 Layer → 多个 Plot + 轴对象

**为什么**：Origin 每个图层里可以有多条曲线，各自不同类型和样式（F083、双 Y 柱+线、分组图都依赖它）。现在一个 Layer 只有一个 `spec`，这是图类型扩展的最大瓶颈。

**设计**：

```rust
pub struct Layer {
    pub placement: Option<LayerRect>,
    pub transparent: bool,
    pub axes: AxisSet,                 // 多根轴
    pub plots: Vec<Plot>,              // 替代 spec
    pub groups: Vec<PlotGroup>,        // 颜色/符号递增的分组
    pub objects: Vec<GraphObjectId>,   // 文本、箭头、参考线等（A7）
    pub legend: Legend,
}
pub struct Plot {
    pub id: PlotId,
    pub mark: MarkKind,                // 见 A3
    pub stat: StatKind,                // Identity / Mean / Bin / Kde / Box …
    pub mapping: Mapping,              // x, y, z, err_x, err_y, color, size, shape, label, group → DataRef
    pub x_axis: AxisId, pub y_axis: AxisId,
    pub style: PlotStyle,              // 线、符号、填充；None 表示继承分组
    pub rows: RowRange,
    pub visible: bool,
}
pub struct AxisSet { pub axes: Vec<Axis> }   // 每根轴：方向、位置(左/右/上/下/偏移)、刻度、范围、标题、链接
pub struct Axis { pub id: AxisId, pub dir: AxisDir, pub pos: AxisPos, pub scale: Scale, pub range: AxisRange, pub link: Option<AxisLink> }
pub struct AxisLink { pub to: (LayerId, AxisId), pub kind: LinkKind /* Straight | Custom(a,b) */ }
```

- 图层之间轴链接（Origin 的 Layer Link）用 `AxisLink`；多面板共享 X 轴就靠这个。
- 现有 14 种 `PlotType` 变成"预设"：生成一组 `Plot`（如 StackedBar = N 个 Bar mark + Stack stat）。
- 旧 `PlotSpec` 迁移（v10）：一个 spec → N 个 Plot（每个 y_key 一条）。
- 命令：`plot.add_curve`、`plot.remove_curve`、`plot.change_type`、`plot.move_to_axis`、`layer.add_right_y`、`layer.link_axes`。
- UI：图层内容对话框（Layer Contents，左边数据列表，右边图层曲线列表，中间加/删按钮）。

**验收**：
- 同一图层：柱（左 Y）+ 线+符号（右 Y），可以导出 SVG 并与参考图像素差异 < 1%。
- 图层内增删曲线、改单条曲线类型，可撤销。
- 两个图层共享 X 轴，缩放一个另一个跟着变。
- v9 → v10 迁移测试。

### A3 Mark / Stat / Coord 可组合渲染

**为什么**：`scene.rs` 1213 行，`build_layer` 按 `PlotType` 分支。照这个写法加 235 种图会变成上万行的 match。

**设计**：

```rust
pub trait Stat { fn compute(&self, data: &MappedData) -> StatOutput; }          // Identity, Mean, Bin, Kde, BoxSummary, Density2D, Contour, Regression …
pub trait Mark { fn build(&self, ctx: &MarkCtx, data: &StatOutput, out: &mut Vec<Prim>); fn legend(&self) -> LegendSwatch; }
pub trait Coord { fn to_page(&self, p: [f64;2]) -> [f32;2]; fn gridlines(&self) -> …; }  // Cartesian, Polar, Ternary, Smith, Projection3D
pub enum Position { Identity, Dodge{width}, Stack, Fill, Jitter{w,h,seed} }
```

- Marks：Line、Point、Bar、Area、Step、Spline、Text、ErrorBar、Drop、Box、Violin、Heatmap、Contour、Surface、Pie、Vector、Candlestick、Polygon。
- 预设（`presets/*.csv` → `presets/*.toml`）：`{ name, category, marks: [...], stat, position, coord, data_shape: "XY" | "XYY" | "XYZ" | "Matrix" | "Groups" }`。新增图类型应该只是加预设文件，少量加 Mark。
- `scene::build_layer` 只做：对每个 Plot → Stat → Position → Coord → Mark，最后统一裁剪和排序。
- `scene.rs` 拆成 `scene/{layout,axis,legend,marks/*,stats/*,coords/*}.rs`，每个文件 < 400 行。

**验收**：
- 现有 14 种图的 SVG 输出与重构前逐字节一致（先把现有输出存成 golden 文件，再重构）。
- 加一个新图类型（如带抖动的点图）只需新增 1 个预设 + 0 个 Mark。
- `scene/` 下没有 `match plot_type` 之类的总分支。

### A4 表达式引擎

**为什么**：设置列值（F035）、用户自定义拟合函数（F142）、函数图、筛选条件、参数约束都要它。

**设计**：
- `scicanvas-core/src/expr/`：词法 → Pratt 解析 → AST → 向量化求值（对整列一次求值，不逐行解释）。
- 语法兼容 Origin 列公式的常用子集：`col(A)`、`A`、`col("Long Name")`、`i`（行号）、`wcol(n)`、`+ - * / ^`、比较和逻辑、`if(c,a,b)`。
- 函数库：数学（sin…atanh、exp、ln、log、sqrt、abs、mod、floor/ceil/round…）、统计（mean、sd、sum、min、max、median、percentile、count、cumsum、diff、rank）、字符串（left、right、mid、len、upper、lower、find、replace、value、text）、日期（date、year、month、day、hour、now）、随机（rnd、normal，指定种子）。目标 1.0 前约 120 个。
- 符号求导：AST 求导 + 化简，供 LM 拟合用解析雅可比；失败时退回数值差分。
- 列公式依赖图：列 → 依赖的列，变化后按拓扑序重算，检测循环引用。

**依赖**：自己写（解析器 ~800 行），不要用 `meval`/`evalexpr`（不支持向量化和求导）。

**验收**：
- 300+ 条公式单元测试，与手算/NumPy 结果比对（相对误差 < 1e-12）。
- 100 万行 `sin(A)*exp(-B/2)` 求值 < 100 ms。
- 公式列在源列编辑后自动更新，循环引用给出明确错误。

### A5 操作对象 + 重算 + 报告表

**为什么**：Origin 的分析结果（拟合、统计、FFT）都是"操作"：记录输入、参数、输出；源数据变化后可以重算（Auto/Manual）；结果写在报告表里，可折叠表格 + 嵌入图。没有这个，后面的批处理、分析模板、Python 录制都没法做。

**设计**：

```rust
pub struct Operation {
    pub id: OpId, pub kind: String,            // "fit.nonlinear", "stats.ttest2" …
    pub inputs: Vec<DataRef>, pub params: serde_json::Value,
    pub outputs: Vec<OutputRef>,               // 写出的列、报告表、图
    pub recalc: RecalcMode,                    // Auto | Manual | None
    pub stale: bool,
}
pub trait Analysis { fn id(&self)->&'static str; fn run(&self, inputs: &[ColumnView], params: &Value) -> Result<AnalysisOutput>; fn dialog(&self) -> DialogSchema; }
pub struct Report { pub title: String, pub sections: Vec<ReportSection> }  // Table / KeyValue / Notes / EmbeddedGraph，可嵌套
```

- 每个分析功能实现 `Analysis`，参数对话框由 `DialogSchema` 自动生成（字段类型、范围、默认值），不要每个分析手写 UI。
- 报告表作为 Book 的一种特殊 Sheet（`SheetKind::Report`），只读，带锁形图标；"Change Parameters" 重新打开对话框。
- 数据编辑时标记依赖 Operation 为 stale；Auto 模式下一帧末尾重算，Manual 模式显示黄色锁。
- 文件格式 v11：存 Operation 列表。

**验收**：
- 线性拟合改为 Operation：改源数据后报告和拟合曲线自动更新；Manual 模式下锁变黄，点击后更新。
- 同一个分析对话框框架至少服务 3 个分析（线性拟合、描述统计、t 检验）。

### A6 发布级导出 + 真实字体度量 + 剪贴板

**为什么**：现在只能导出 SVG，`text_width` 是估算值，导致标签对齐和 PNG/PDF 都没法做准。期刊投稿需要 PNG/TIFF 600 dpi、PDF/EPS 嵌入字体。

**设计**：
- 字体：`fontdb` 加载系统字体（Windows `C:\Windows\Fonts`）+ 内置 1 套开源字体（Noto Sans / Noto Sans CJK SC 子集、STIX Two Math）作为回退；`ttf-parser` + `rustybuzz` 做真实测宽。scene 里所有文本测量走 `FontMetrics` trait，屏幕和导出用同一实现。
- 富文本：Origin 转义 `\b()`、`\i()`、`\+()` 上标、`\-()` 下标、`\g()` 希腊字母、`\f:Font()`。解析成 `RichText { runs: Vec<Run> }`，scene 按 run 排版（F102）。
- PNG/TIFF/JPG：SVG → `resvg` + `tiny-skia` 光栅化；DPI 可选 72–1200；TIFF 用 `tiff` crate，支持 LZW 和 CMYK 选项可后做。
- PDF：`svg2pdf`（或 `krilla`）直接从 scene 生成，字体子集嵌入；EPS 后做（P2）。
- 剪贴板：Windows 下同时放 PNG（CF_DIB）和 EMF；EMF 可先用 PNG 代替，Word/PPT 粘贴可用。用 `arboard` + `clipboard-win`。
- 导出对话框：格式、尺寸（按页面 / 自定义 mm/in）、DPI、背景透明、导出范围（当前图 / 所有图 / 选中图批量）。

**验收**：
- 同一图的 SVG、PNG(300 dpi)、PDF 三者对比，文本位置偏差 < 0.5 px（300 dpi 下）。
- 中文标签、希腊字母、上下标在 PDF 中可选中复制，字体已嵌入。
- 粘贴到 Word / PowerPoint 正常显示。

### A7 图形对象 + 拾取编辑

**设计**：`GraphObject { id, kind: Text(RichText) | Line | Arrow | Rect | Ellipse | Polygon | Image | RefLine{axis,value}, anchor: Anchor(Page|Layer|Data), style, z }`；scene 输出 `Prim` 时带 `HitId`（Plot / Axis / Legend / Object / Layer），屏幕点击通过命中测试选中对象，双击打开对应属性面板，拖动移动并写入历史。

**验收**：点击曲线 → 打开该曲线样式；点击轴 → 轴对话框；拖动图例和文本，可撤销；参考线随数据坐标移动。

### A8 数值基础库

| 用途 | 方案 | 说明 |
| --- | --- | --- |
| 线性代数 | `faer`（首选）或 `nalgebra` | QR/SVD 用于线性拟合和协方差 |
| 分布与特殊函数 | `statrs` + 自写补充（非中心 t/F 等） | 所有 p 值与 R 对比到 1e-10 |
| FFT | `rustfft` + `realfft` | |
| 三角剖分 | `spade` | 等高线、XYZ 网格化 |
| 等值线 | 自写 Marching Squares（或 `contour` crate） | |
| 优化 | 自写 Levenberg–Marquardt（带边界与线性约束），参考 MINPACK | 不要依赖 `argmin` 的 LM，控制力不足 |
| 样条/插值 | 已有 `spline.rs`，扩展 Akima、PCHIP、B 样条、平滑样条 | |

全部依赖用 `cargo deny` 检查许可证，禁止 GPL/LGPL。

---

## 6. 分模块缺口与路线

下面每个模块列出"缺什么 → 怎么做 → 依赖哪个 A"。

### 6.1 工作区与项目（50%）

- 缺：项目搜索、项目内对象重命名冲突处理、自动保存与崩溃恢复、最近文件、附加文件、窗口状态持久化。
- 做法：自动保存每 5 分钟写 `.sccproj.autosave`，启动时检测；最近文件存在 `%APPDATA%\SciCanvas\recent.json`。
- 依赖：无。

### 6.2 工作表（29%）

- 缺：F028 插入/删除行、F032 数字格式、F035 设置列值、F049 多工作表、列属性对话框（长名、单位、注释、类型、格式）、筛选、查找替换、冻结、条件格式（P2）、大数据虚拟滚动。
- 做法：基于 A1；表格绘制只渲染可见行（egui `ScrollArea::show_rows`）；列头三行（长名/单位/注释）可开关；F035 基于 A4 的公式对话框（左侧函数树，右侧编辑框，下方预览前 10 行）。
- 依赖：A1、A4。

### 6.3 矩阵与网格化（0%）

- 缺：Matrix 对象、矩阵编辑、XYZ → 矩阵网格化（规则网格、Kriging 可后做、Delaunay 线性、Shepard、薄板样条）、矩阵运算（转置、翻转、缩放、扩展）、矩阵 ↔ 图像。
- 依赖：A1、A8（spade）。

### 6.4 导入导出（18%）

| 优先级 | 格式 | crate |
| --- | --- | --- |
| P0 | CSV/TXT 导入向导（分隔符、编码、表头行、跳过行、预览）；CSV 导出；拖放导入 | 已有 csv + `encoding_rs` |
| P0 | Excel xlsx/xls 导入 | `calamine` |
| P1 | Excel xlsx 导出 | `rust_xlsxwriter` |
| P1 | 图片导入（作为矩阵 / 图层背景） | `image` |
| P1 | MATLAB .mat v5 | 自写或 `matfile` |
| P2 | HDF5 / NetCDF / Parquet | `hdf5`（需要 C 库，考虑后做）、`netcdf`、`parquet` |
| P2 | 仪器格式（JCAMP-DX、SPC、pCLAMP…） | 自写，按用户需求 |
| 不做 | Origin .opju 读写（专有格式） | |

导入统一实现 `trait Importer { fn sniff(&[u8]) -> f32; fn preview(...) -> Preview; fn import(...) -> Vec<Sheet>; }`，导入设置可保存为"导入过滤器"复用。

### 6.5 绘图基础（50%）

- 缺：F083 图层内增删曲线（A2）、F089 图形模板、F102 富文本（A6）、曲线样式细节（填充图案、符号边/内、连线方式、Modifiers 按列映射颜色/大小）、颜色递增分组、轴断点、参考线、轴刻度标签格式（日期/分类/自定义）、图例自动更新与编辑、对象拾取（A7）。
- 图形模板：模板 = `Page` 去掉 DataRef 的序列化（保留样式、轴、图层布局、对象），另存为 `.sctpl`；套用时按 data_shape 把新数据绑定进去。"克隆图并换数据"同理。
- 依赖：A2、A3、A6、A7。

### 6.6 图类型（7%）

**按 A3 完成后，以"预设 + 少量新 Mark/Stat/Coord"批量铺。**沿用上一版路线中的 6 批，修订如下：

| 批次 | 内容 | 新增 Mark / Stat / Coord | 约数量 |
| --- | --- | --- | --- |
| B1 | 剩余 2D：点线变体、散点变体（气泡、颜色映射、带标签）、柱状变体（分组、带误差、条形堆叠）、面积变体、高低线、Drop line、多轴（双 Y、多 Y、双 X、Y 偏移） | Bubble 映射、Dodge | ~45 |
| B2 | 多面板（垂直、水平、4/9 面板、堆叠、合并图）、瀑布、插图（Inset）、饼图/环图 | Pie Mark、Polar 布局 | ~30 |
| B3 | 统计图：直方图、带分布曲线直方图、箱线图、小提琴图、QQ/PP、Pareto、散点矩阵、边际图、平均值条、Raincloud、Beeswarm、Ridgeline、热图 | Bin、Kde、BoxSummary、Jitter；Box/Violin Mark | ~42 + 分组 14 |
| B4 | 等高线与热图（XYZ 和矩阵）、彩色填充、Profile 图、图像图 | Contour Stat、Heatmap Mark、颜色条 | ~17 |
| B5 | 专业图：极坐标、三元图、Smith 圆图、风玫瑰、雷达、向量场、Polar 等高线、Piper/Durov（P2）、金融（K 线、OHLC、成交量） | Polar/Ternary/Smith Coord、Vector、Candlestick | ~30 |
| B6 | 3D：散点、柱、曲面（彩色、线框、带投影）、XYY 3D 柱/墙、3D 向量 | Projection3D Coord（CPU 投影 + 深度排序先做；wgpu 交互旋转后做） | ~37 |
| — | 地图（6 条） | 建议标"不做"或 P2，依赖 Shapefile/GeoJSON | — |

每批验收：每种图一个 golden SVG + 一张 Origin 同数据截图的人工对照；"统计图"另外要求统计量与 R 结果一致。

### 6.7 图形探索与小工具（7%）

- 缺：数据读取器（Data Reader/屏幕读数）、数据选择器（Data Selector，选范围）、刷选（Data Highlighter）、缩放平移已有部分；Gadgets：积分、统计、拟合、FFT、微分、基线、四分位、聚类（P2）。
- 做法：Gadget = 一个"感兴趣区域（ROI 矩形）+ 一个 Analysis"，结果显示在 ROI 旁边，可输出到报告表。基于 A5 + A7，所有 Gadget 共享一个框架。

### 6.8 回归与拟合（11%）

这是 Origin 用户最看重的模块，建议投入最大。

- 线性：简单线性、带权重、多元线性（MLR）、多项式、过原点、固定斜率、比较两条直线；输出 R²、调整 R²、ANOVA、参数 SE/t/p/置信区间、残差图、置信带和预测带。
- 非线性（NLFit）：
  - 算法：Levenberg–Marquardt（带参数上下界）、线性约束（投影或罚函数）、Orthogonal Distance Regression（P1）、单纯形（P2）。
  - 初值：每个内置函数带"参数初始化代码"（Origin 每个 FDF 都有），用 Rust 闭包实现，如高斯峰从最大值和半高宽估计。
  - 权重：无、仪器（误差列）、统计、任意列。
  - 共享参数 / 全局拟合（多数据集共享部分参数）：P1。
  - 报告：参数表、统计量（χ²、R²、RSS、AIC/BIC）、协方差/相关矩阵、残差分析、拟合曲线。
- 函数库：内置 239 个函数分 10 个族（Exponential、Growth/Sigmoidal、Peak、Power、Rational、Polynomial、Waveform、Pharmacology、Statistics、Hyperbola…）。**以数据形式**存 `fitfuncs/*.toml`：`{ name, category, formula, params, bounds, init: "builtin:gauss" | 表达式 }`，不要每个函数写一个 Rust 结构。P0 先做最常用 40 个（Gauss、Lorentz、Voigt、ExpDec1/2/3、ExpGrow、Boltzmann、Logistic、Hill、Allometric、Poly、Sine…），其余按族逐批补。
- 用户自定义函数（F142）：基于 A4，函数编辑器带参数表、初值、导数自动推导，保存到用户目录。
- 隐函数、曲面拟合（XYZ / 矩阵）：P1。
- 依赖：A4、A5、A8。
- **验收：NIST StRD 非线性回归全部 27 个数据集**，参数与认证值至少 6 位有效数字一致（低难度 / 中难度要求 7 位）；线性回归 11 个数据集到 1e-10。

### 6.9 数学运算（19%）

- 缺：F158 插值/外推（线性、样条、Akima、PCHIP，1D 和 3D 散点插值）、F168 列/行运算（基于 A4）、多条曲线平均、简单曲线运算（加减曲线）、数值积分（梯形/Simpson、曲线下面积）、微分（含 Savitzky–Golay 导数）、归一化（已有，补多种方式）、基线扣除（P1，与峰分析共享）。

### 6.10 统计（4%）

- 描述统计：按列/按行/按组、频数分布（F177）、离散频数、交叉表、相关系数（Pearson/Spearman/Kendall）、正态性检验（Shapiro-Wilk、KS、Lilliefors、Anderson-Darling、D'Agostino）。
- 假设检验：单样本/双样本/配对 t（F185–F187）、方差齐性（Levene、Brown-Forsythe、F）、单/双因素 ANOVA、重复测量 ANOVA、均值比较（Tukey、Bonferroni、Scheffe、Dunnett…）。
- 非参数：Mann-Whitney、Wilcoxon 符号秩、Kruskal-Wallis、Friedman、Mood 中位数、双样本 KS。
- 多元（P2）：PCA、聚类（K-means、层次）、判别分析、PLS。
- 生存（P2）：Kaplan-Meier、Cox、Weibull。
- 功效与样本量（P2）。
- 输出全部走 A5 报告表。
- **验收**：每个检验对照 R（`t.test`、`aov`、`TukeyHSD`、`shapiro.test` 等）生成的参考值，p 值相对误差 < 1e-8。参考值由 `tests/reference/gen_r.R` 脚本生成并提交到仓库。

### 6.11 信号处理（7%）

- 平滑（已有部分：补 Savitzky-Golay、Percentile、FFT 滤波、LOWESS/LOESS）、FFT（F204，振幅/相位/功率谱、窗函数、单边/双边）、IFFT、STFT（P1）、相关与卷积、去卷积（P2）、FFT 滤波器（低/高/带通/带阻/阈值）、IIR/FIR 滤波器设计（P1）、包络、Hilbert 变换、小波（P2）。
- 依赖：A8（rustfft）。
- 验收：与 SciPy `numpy.fft`、`scipy.signal.savgol_filter`、`scipy.signal.butter`+`filtfilt` 输出对比到 1e-10。

### 6.12 峰分析（0%）

- 基线（锚点 + 插值、ALS、多项式拟合、Asymmetric Least Squares）、寻峰（局部最大、二阶导、高度/宽度/距离阈值）、峰积分（面积、中心、FWHM）、多峰拟合（基于 NLFit 的复合函数 Gauss/Lorentz/Voigt/PearsonVII，共享基线）。
- UI：Peak Analyzer 向导（基线 → 寻峰 → 积分 / 拟合 → 报告），可视化拖动锚点。
- 依赖：A5、A7、6.8 NLFit。

### 6.13 图像处理（0%）

- 1.0 只做：导入图像为矩阵、灰度/伪彩色显示、剖面（Profile）、裁剪/旋转/缩放、阈值二值化、直方图。其余 52 个 Image X-Function 和 13 个 Vision X-Function 标为 P2。
- 依赖：A1 矩阵、`image`、`imageproc`。

### 6.14 导出与发布（10%）

- 见 A6。另外：批量导出所有图、按期刊预设尺寸（单栏 85 mm / 双栏 175 mm）导出、字体统一替换、"出版样式"一键套用（SCI 风格模板已在默认样式中，扩展为可选主题）。

### 6.15 自动化与编程（0%）

保持原路线：内置 Python 运行时（PyO3 + 嵌入 CPython）、`scicanvas` Python 包、脚本窗口、操作录制、包管理。前置条件是 A5（所有操作都是可序列化的 Operation + 命令），否则录制做不了。建议放在 1.0 之后的 1.1。

---

## 7. 分阶段任务卡（给 GPT 的工作单）

**规则（每张卡都适用）**：
1. 一张卡一个 PR，PR 内必须包含测试，`tools/check.py` 和 `cargo clippy -D warnings` 全通过。
2. 改了模型就要加文件格式迁移和迁移测试。
3. 改了 parity 状态就同时更新 `docs/parity/parity.json` 和 HTML 清单。
4. 不要在 UI 里写业务逻辑；core 先实现、先测试，UI 只调命令。
5. 渲染相关改动必须有 golden SVG 测试。

### 阶段 0：重构前准备（第 1 周）

| 卡 | 内容 | 验收 |
| --- | --- | --- |
| T0.1 | 为现有 14 种图 + 误差棒 + 标签 + 多层生成 golden SVG（`tests/golden/*.svg`），加 `cargo test` 比较 | golden 测试稳定通过 |
| T0.2 | 加 `cargo deny` 配置和 CI 步骤 | CI 绿 |
| T0.3 | 加 parity 新增条目（第 3 节表格）和 `sub_items` 字段，HTML 同步 | 清单可显示子项进度 |
| T0.4 | 性能基准 `benches/`：10 万 / 100 万行导入、渲染、排序 | 输出记录到 docs/perf.md |

### 阶段 1：结构重构（第 2–8 周）

| 卡 | 内容 | 依赖 | 周 |
| --- | --- | --- | --- |
| T1.1 | A1-a：`ColumnData` 类型化列 + `ColumnView` + v9 迁移 | T0.1 | 2 |
| T1.2 | A1-b：Book → Sheet 多工作表（F049）+ 列属性对话框 + 数字格式（F032） | T1.1 | 1 |
| T1.3 | A3：scene 拆分为 Mark/Stat/Position/Coord，14 种图迁移到预设，golden 不变 | T0.1 | 2 |
| T1.4 | A2：Layer → Vec\<Plot\> + AxisSet + v10 迁移；图层内容对话框（F083）；双 Y（F305） | T1.3 | 2 |
| T1.5 | A6-a：fontdb + rustybuzz 真实测宽，替换 `text_width`；内置回退字体 | T1.3 | 1 |

> T1.1 与 T1.3 可并行。

### 阶段 2：P0 收口（第 9–16 周）

| 卡 | 内容 | 依赖 |
| --- | --- | --- |
| T2.1 | A4 表达式引擎 + 120 函数 + 设置列值对话框（F035、F168） | T1.1 |
| T2.2 | 插入/删除行（F028）、虚拟滚动、查找替换 | T1.1 |
| T2.3 | 导入向导 + Excel 导入 + CSV 导出 + 拖放（F065、F066、F068、F069） | T1.2 |
| T2.4 | A6-b：PNG/TIFF/PDF 导出 + 剪贴板 + 导出对话框（F227、F228、F231） | T1.5 |
| T2.5 | A6-c：富文本标签（F102） | T1.5 |
| T2.6 | A5：Operation + 报告表 + 重算 + 对话框 Schema；把线性拟合、多项式拟合改成 Operation | T1.2 |
| T2.7 | NLFit：LM + 约束 + 初值 + 报告（F134–F138）；NIST StRD 测试 | T2.1、T2.6 |
| T2.8 | 拟合函数库 TOML + 前 40 个函数（F148–F152）+ 用户自定义函数（F142） | T2.7 |
| T2.9 | 统计：描述统计报告、频数（F177）、t 检验三种（F185–F187）、R 参考值脚本 | T2.6 |
| T2.10 | 插值（F158）、FFT（F204） | T2.6 |
| T2.11 | 多面板预设（F321–F323）+ 轴链接 | T1.4 |
| T2.12 | 图形模板（F089） | T1.4 |
| T2.13 | 13 条 P0 部分实现项逐条补完 | — |

**阶段 2 结束 = 公测（约第 16–20 周）**：P0 87 条全部"已完成"。

### 阶段 3：图类型批次 + 分析扩展（第 17–36 周）

| 卡 | 内容 |
| --- | --- |
| T3.1 | A7 图形对象 + 拾取编辑 + 参考线 |
| T3.2 | 图类型 B1（剩余 2D + 多轴） |
| T3.3 | 图类型 B2（多面板全集、瀑布、插图、饼图） |
| T3.4 | 图类型 B3（统计图 + 分组图）+ ANOVA、非参数、正态性检验 |
| T3.5 | 矩阵对象 + 网格化 + 图类型 B4（等高线/热图） |
| T3.6 | 拟合函数库补全到 239 + 全局拟合 + 曲面拟合 |
| T3.7 | 信号处理（滤波、相关、STFT）+ 峰分析器 |
| T3.8 | 图类型 B5（专业图、金融） |
| T3.9 | 图形探索：数据读取器、选择器、刷选、Gadget 框架 + 5 个 Gadget |

### 阶段 4：1.0（第 37–44 周）

| 卡 | 内容 |
| --- | --- |
| T4.1 | 图类型 B6（3D，CPU 投影版） |
| T4.2 | 批处理、分析模板、对话框主题 |
| T4.3 | 图像处理 1.0 子集 |
| T4.4 | 性能与稳定性：百万行目标、崩溃恢复、内存泄漏检查 |
| T4.5 | 安装包、文档、示例项目 |

相比上一版时间线（41 周到 1.0），因为插入了阶段 1 的结构重构，**1.0 推迟到约第 44 周，公测仍在约第 20 周**。重构多花 3 周，但后面每个图类型和分析功能都会更快，整体不亏。

---

## 8. 验证体系

| 类别 | 方法 | 位置 |
| --- | --- | --- |
| 渲染 | golden SVG 逐字节比较；PNG 用像素差 < 1% | `tests/golden/` |
| 拟合 | NIST StRD（线性 11 + 非线性 27 数据集） | `tests/nist/` |
| 统计 | R 脚本生成参考值（JSON），测试比对 | `tests/reference/gen_r.R` |
| 信号/数学 | SciPy 脚本生成参考值 | `tests/reference/gen_scipy.py` |
| 与 Origin 对照 | 同一数据在 Origin 中出图、截图，人工比对，记录在 `docs/parity/visual/` | 每批图类型 |
| 性能 | `benches/` + 记录表 `docs/perf.md`，回归超过 20% CI 告警 | |
| 文件兼容 | 每个历史版本保存一个示例项目，迁移测试读取并对比输出 | `tests/fixtures/v*/` |

参考值脚本只在开发者机器上运行，生成的 JSON 提交到仓库，CI 不需要装 R/Python。

---

## 9. 给 GPT 的提示词模板

```
你在 SciCanvas 仓库（Rust，scicanvas-core + scicanvas 两个 crate，egui 0.29，Windows 优先）中工作。
请先阅读 AGENTS.md、docs/ROADMAP.md、docs/SciCanvas 功能完备性分析与技术路线.md 中的任务卡 {卡号}。

任务：{卡号} {标题}
范围：{涉及的文件/模块}
接口：{按文档第 5 节中对应的结构体/trait 定义实现；如需偏离，在 PR 描述中说明理由}
约束：
- core 不得依赖 UI；UI 只通过命令注册表调用 core。
- 模型改动必须提升文件格式版本并添加迁移和迁移测试。
- 渲染改动必须更新或新增 golden SVG 测试；未改变的图必须保持逐字节一致。
- 新增依赖必须是 MIT/Apache-2.0/BSD 许可，并通过 cargo deny。
- 更新 docs/parity/parity.json 和对照清单 HTML 中相关条目的状态。
验收：{复制任务卡的验收标准}
完成后运行 cargo fmt、cargo clippy --all-targets -- -D warnings、cargo test --all-features，全部通过再提交。
```

---

## 附：数据来源

- 代码：`scicanvas` @ ad11c63，测试 45 + 59 + 29 = 133，clippy 干净。
- 清单：`docs/parity/parity.json` 488 条。
- Origin.chm 目录 1199 项；X-Function.chm 615 个函数（数据操作 161、导入导出 71、实用工具 61、图像 52、统计 47、图形操作 44、绘图 44、数学 35、信号处理 30、拟合 25、数据探索 16、视觉 13、其他 6、光谱 6、数据库 4）；拟合函数 239 + 5 + 22 + 7。
