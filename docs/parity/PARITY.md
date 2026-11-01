# SciCanvas 功能对照清单

条目按 Origin 帮助文档的目录整理，只引用章节标题；实现按公开说明和观察到的行为编写。
数据源为 `parity.json`，页面模板为 `tracker_template.html`。运行 `python docs/parity/build_parity.py` 生成文档与页面；`--check` 检查同步，`--desktop` 同步桌面副本。新增条目追加稳定编号，已有编号保留。

共 514 项：P0 97 项，P1 146 项，P2 259 项，不做 12 项。
状态：已完成 57 项，已实现待验收 66 项，部分实现 17 项，未开始 362 项，不做 12 项。

- **P0 / M1 日常够用**：日常数据处理和论文作图的最小集合。
- **P1 / M2 论文全流程**：统计、多面板、信号处理等。
- **P2 / M3 高级功能**：3D、专业图与高级分析。
- **部分实现**：范围内仍有明确缺口；**已实现待验收**：已接入工作流，专项或参考对照待完成。
- **已完成**：完成当前条目的实现与既定验证；**未开始**按桌面代码判断，旧网页演示单独说明。
- **子项**：`sub_items` 按名称记录 `total`、`done`、可选 `review` / `partial`、`note`。已完成与待验收分别计数；未开始为总数减去以上计数。
- 子项独立于共享存储中的条目状态，修改条目状态不会自动增加子项完成数。规划目标与路线参考基数在备注中标明；重叠目录分别查看，避免重复加总。
- F489 起补充第三阶段需求。F514 为 F148–F156 的目录总览，239 是路线参考数；当前通用线性/多项式回归单独计数。
- Origin 私有格式与脚本兼容保留原范围；扩展包采用 SciCanvas 自有格式。

验证原则：分析与 NIST StRD、SciPy / R 对照；图形与 matplotlib 同数据出图比较数据位置与刻度。

## 工作区与项目

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F001 | 多文档子窗口（拖动、缩放、最小化、最大化） | Basic Child Window Operations | P0 | 已完成 | 自绘 MDI；拖动/缩放、最小化换行、最大化/还原与统一窗口命令；自由拖出工作区，浮动窗口可越过可见区域边缘，支持重置位置；标题栏上边界限制在工作区/功能区下方，左、右、下仍自由移动 | 真实 egui 拖动、缩放、窗口按钮与几何恢复回归；真实向上拖动松开、旧越界位置恢复、非零顶部及另外三边越界回归 | — |
| F002 | 项目管理器（文件夹树、显示 / 隐藏窗口） | Project Explorer | P0 | 已完成 | 真实多级目录；拖放/菜单移动、慢速双击/F2 就地重命名、删除保留内容、显示/隐藏窗口；快捷键按焦点分流；Ctrl 多选、Shift 连续选择、Ctrl+Shift 追加范围；按名称/类型排序，跳过折叠内容 | 项目树、移动/重命名历史、F2 与工作表键盘隔离回归已通过 | — |
| F003 | 项目管理器多级文件夹、拖动整理 | Using Project Explorer to Manage Child Windows | P1 | 已实现待验收 | 真实多级目录；工作簿、图形与文件夹拖入目录/拖回根目录，目标高亮；名称慢速双击或 F2 就地编辑；菜单移动保留，均复用撤销事务；多选批量移动保留内部层级，整批失败回滚；Note 同样支持目录和移动 | 真实 egui 鼠标拖放、根目录移动、自身/下级拒绝、同目录无操作、慢/快速点击、Enter/Esc/失焦与撤销重做回归通过；Windows 手工交互验收待做 | — |
| F004 | 窗口排列：层叠 / 平铺 | Arrange Windows | P1 | 已实现待验收 | 层叠/平铺、边缘及四角吸附、顶部吸附布局与 Snap Assist 候选补位已接入；越界自由移动，贴靠只在实际边缘附近触发；视图菜单或 Ctrl+Shift+R 可重置全部窗口位置；设置面板可左右侧栏吸附、上下堆叠/换序、折叠、调整高度和拖回浮动，左侧与项目管理器共用侧栏，会话内记住布局；标题栏上边界限制在工作区/功能区下方，左、右、下仍自由移动 | 窗口几何、吸附和候选补位回归通过；Windows 不同缩放、窄窗口手工验收待做；真实鼠标左右吸附/换序/分隔条/折叠/拖出/菜单浮动/窄屏/关闭清理及工作区避让回归；真实向上拖动松开、旧越界位置恢复、非零顶部及另外三边越界回归 | — |
| F005 | 项目文件保存 / 打开 | Opening, Closing and Backing Up the Origin Project File | P0 | 已完成 | v21 项目兼容读取 v0–v20，SCV v1。保存点代号跟随撤销重做，项目切换/退出共用保存确认，失败弹窗及 Book/列定位。v19/v20 拟合定义/报告与 v21 稳定指纹、独立行域随项目保存。 | 保存/读入、恢复文件/会话锁、历史保存点、实际确认按钮、最近列表及失败保持回归；Windows 人工交互验收待做 | — |
| F006 | 自动备份与崩溃恢复 | Opening, Closing and Backing Up the Origin Project File | P1 | 已实现待验收 | 未保存项目每五分钟写入会话独立的原子 JSON 恢复文件。Windows 独占锁区分运行/退出会话，启动提供恢复/放弃/稍后，保存或确认放弃后清理本会话备份。panic hook 记录 crash.log，恢复项目保持未保存标记。；第二轮改为单任务后台备份，缓冲流式原子写入，不在备份时重拟合。保存/清理前等待任务；.source 记录来源和清理状态。未确认的旧副本保留，已明确放弃/完成保存的残留自动清理；退出及保存后的清理在后台等待备份完成，界面轮询进度并可返回项目，析构不再同步等待备份；恢复候选信息仅扫描时读盘，确认和较长清理使用独立模态，背景继续绘制并冻结输入 | 保存/读入、恢复文件/会话锁、历史保存点、实际确认按钮、最近列表及失败保持回归；Windows 人工交互验收待做 | — |
| F007 | 最近打开的文件 | Recent Origin Files | P1 | 已实现待验收 | 文件页列出最近十份项目，打开和保存后去重置顶，APPDATA 保存列表与上次目录。点击最近项目共用未保存确认和既有加载入口。；修正旧记录的 Windows 盘符扩展前缀，归一化并去重 | 保存/读入、恢复文件/会话锁、历史保存点、实际确认按钮、最近列表及失败保持回归；Windows 人工交互验收待做 | — |
| F008 | 撤销 / 重做 | — | P0 | 已完成 | 编辑事务；100 步 / 300 MiB；文件夹新建/重命名/移动/删除均可撤销；整列 Arc 写时复制，快照共享未改列并按唯一分配计入预算，保存点代号随撤销重做恢复；重算缓存改用数据版本和计算元数据，连续事务不再每帧复制列，清空保留类型和分类字典 | 项目快照、真实编辑控件与文件夹事务回归 | — |
| F009 | 命令注册表（菜单、工具栏、脚本共用同一套命令） | — | P0 | 已完成 | 统一命令注册表、可用条件、开关状态与修改标记；菜单/按钮/程序调用共用入口，供后续脚本集成复用 | 命令 ID 唯一、状态条件、窗口/文件夹命令与事务回归 | — |
| F010 | 结果日志（分析结果记录） | Analysis Results: The Results Log | P1 | 未开始 | 停靠面板 + 结构化记录 | 手动操作 | — |
| F011 | Notes 文本窗口 / Markdown 报告 | Notes Window | P2 | 已实现待验收 | Note 独立窗口按 Markdown 显示（标题、表格、列表、引用、代码块、行内粗体/斜体/代码），可切换预览/编辑；预览只读但可选择复制，双击表格单元格整格选中，表格右键复制为制表符文本或 Markdown。拟合/非线性拟合/t 检验/方差分析/相关系数/通用结果表“保存为 Note”写成 Markdown 报告；重命名、移动、删除、撤销重做、.md/.txt 导出与 v10 项目保存已接入。富文本编辑与自动结果日志（F010）另行记录。A5 第二批非线性报告窗口跟随重算，“保存为 Note”使用当时数值新建快照，已有 Note 与手写文本保留原文。 | Markdown 解析/表格转义单元测试；预览渲染、预览/编辑切换、拖选复制、双击整格选中与只读界面测试；拟合报告真实按钮写入 Markdown 表格；目录与 ID、JSON/SCV 往返、旧版迁移回归；Windows 人工验收待做；来源编辑、报告刷新与撤销重做后已有 Note 保留回归 | — |
| F012 | 命令窗口（交互式脚本） | The Origin Command Window and Script Window | P1 | 未开始 | 嵌入 Python（PyO3） | 执行脚本修改工作表 | — |
| F013 | 深色模式 | Dark Mode | P0 | 已完成 | 浅色、深色、跟随系统三种选择；统一主题命令；主按钮、错误、提醒和危险动作使用两套语义颜色，文字对比度回归，字号保持原值并统一命名 | 主题切换命令与项目数据不变回归 | — |
| F014 | 快捷键与自定义快捷键 | Hotkeys | P1 | 部分实现 | 固定快捷键与命令面板已实现；用户自定义快捷键与冲突配置待做 | 固定快捷键、文字框撤销回归已通过；自定义部分待实现 | — |
| F015 | 全文搜索（在项目中查找） | Finding in Origin Project | P2 | 未开始 | 遍历项目数据 | 手动操作 | — |
| F016 | 工程保护（密码） | Protecting Origin Projects | P2 | 未开始 | age / chacha20poly1305 加密 | 加密→解密往返 | — |
| F017 | 中文界面 | — | P0 | 已完成 | 简体中文界面，Windows 系统中文字体回退 | 中文菜单与项目树控件回归 | — |
| F018 | 多语言界面（中 / 英） | — | P1 | 未开始 | fluent 或简单字符串表 | 切换语言检查 | — |
| F019 | 选项 / 偏好设置对话框 | Setting Your Preferences with the Options Dialog Box | P1 | 部分实现 | 窗口普通位置/尺寸及最大化、主题、最近项目和上次目录已持久保存。统一选项/偏好设置对话框及更多可配置项继续保留。；按实际显示器工作区和 DPI 恢复标题栏可见位置，保留负坐标副屏 | 保存/读入、恢复文件/会话锁、历史保存点、实际确认按钮、最近列表及失败保持回归；Windows 人工交互验收待做 | — |
| F020 | 自定义菜单与工具栏 | Custom Menus | P2 | 未开始 | 基于命令注册表 | 手动操作 | — |
| F021 | 模板中心 / 应用中心 | Template Center | 不做 | 不做 | — | — | — |
| F022 | 读取 Origin 工程文件 OPJU / OPJ | The Origin Project File | 不做 | 不做 | 私有格式无公开规范；旧 OPJ 可评估 liborigin | — | — |
| F512 | 项目附加文件 | Project / Attach File | P2 | 未开始 | 原生项目附件索引和导入/导出 | 附件往返、重名与损坏输入 | — |
| F513 | 扩展包打包与分发（OPX 用途对照） | Project / OPX Packaging | P2 | 未开始 | SciCanvas 原生扩展包与资源清单；Origin OPX 二进制兼容不在范围内 | 安装/卸载、包版本、资源完整性 | — |

## 工作表

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F023 | 单元格输入与编辑 | Basic Operations on Worksheet | P0 | 已完成 | egui_extras TableBuilder | 手动操作 | — |
| F024 | 长名称 / 单位 / 注释表头行 | Displaying Supporting Data in Worksheet Header Rows | P0 | 已完成 | 列元数据 | 轴标题自动取“长名称 (单位)” | — |
| F025 | 自定义表头行（参数、采样间隔、迷你图等） | Displaying Supporting Data in Worksheet Header Rows | P1 | 未开始 | 可扩展的表头行定义 | 手动操作 | — |
| F026 | 列属性：X / Y / Z / X误差 / Y误差 / 标签 / 忽略 | Setting Column Designation | P0 | 已实现待验收 | 列角色 X/Y/Z/X误差/Y误差/标签/忽略：功能区、列标题右键和列属性对话框均可设置；误差列自动关联；Z 列和标签列不参与二维作图与分析取列；数据标签选“指定列文本”而未指定列时自动使用标签列；角色随项目保存 | Z/标签列不作为曲线和分析数据、标签列作为数据标签来源、项目往返的核心测试；设为 Z/标签命令撤销/重做回归；Z 列用于等高线/三维图需随对应图型验收 | — |
| F027 | 插入 / 删除 / 移动列 | Arranging the Worksheet: Columns and Rows | P0 | 已实现待验收 | 列插入、删除、清空与稳定列标识；选中列可左移、右移、移到最前/最后（功能区“列”分组、列标题右键“移动列”），多列一起移动保持相对顺序，误差列移动后仍关联原数据列，一步撤销 | 列移动单元测试（相对顺序、边界不动、误差列关联）与列命令撤销/重做回归；Windows 人工验收待做 | — |
| F028 | 插入 / 删除行 | Arranging the Worksheet: Columns and Rows | P0 | 已实现待验收 | 工作表“行”分组与单元格右键菜单：在选区上方插入同数量空行、删除选区覆盖的整行；图形行范围随之平移，整段删除时改回全部行；一步撤销；插删行区分独立采样结果与逐行结果，拟合网格和带保持自身行域；固定独立行范围时按 Book/列完整来源匹配，跨表同名不误固定 | 核心库行操作与行范围回归、命令撤销重做回归；界面手动操作待验收 | — |
| F029 | 从 Excel 复制粘贴（多单元格） | Copying and Pasting Worksheet Data | P0 | 已完成 | 制表符解析 | 单元测试 | — |
| F030 | 鼠标拖动框选单元格、Shift 扩展、复制选区 | Scrolling and Simple Data Selection in the Worksheet | P0 | 已实现待验收 | Rust app.rs/sheet.rs 已接入拖动框选、Shift 扩展、复制/剪切/粘贴与清空选区 | 选区边界、键盘、剪贴板及撤销回归已通过；真实鼠标与长表人工验收待做 | — |
| F031 | 查找与替换 | Finding and Replacing Worksheet Data | P1 | 未开始 | — | 手动操作 | — |
| F032 | 数值显示格式（小数位、科学计数、工程单位） | Controlling How Numeric Data Display in the Worksheet Column | P0 | 已实现待验收 | 工作表 → 列属性（双击列标题或右键“列属性…”）：数值列可选原样、固定小数位、科学计数、工程单位（SI 词头）、百分比及小数位，日期列可选 4 种显示格式；只改变显示，存储值与原始写法不变，格式随项目保存；列属性有效修改即时写入工作表和关联图形，切换窗口仍绑定原表；只用标题栏最小化/还原和叉号，关闭保留修改，撤回使用撤销 | 格式化单元测试（舍入进位、负零、非有限值、SI 词头越界）、项目往返与未设格式时文件不变、列属性对话框一步撤销及转换失败不改动的界面测试；实际工作表/图形绘制、目标表固定、预览数据复用与取消回归；实时设置无底部确认/取消/关闭按钮，标题栏最小化/还原/叉号、关闭保留和开窗撤销重做回归 | — |
| F033 | 日期时间列与计算 | Dates and Times in the Worksheet | P1 | 部分实现 | UTC 毫秒日期类型、ISO8601/常见年月日写法/四位年份输入、缺失位图、排序、类型转换、v9 保存已接入；本地时区/完整日期格式和日历运算待做 | 日期时区等价、行操作、排序/往返、失败转换原子性回归通过；完整日期计算与人工验收待补 | — |
| F034 | 文本列与分类数据 | Setting Column Designation | P1 | 已实现待验收 | Text 与 Category 字典列，列头右键/命令转换；编辑、缺失、复制、排序、保存和撤销已接入；图形的分类轴/分组绘图另按相关条目推进 | 类型、字典、空码、行重排、默认标签排序/有序字典排序、JSON/SCV 与界面转换回归通过；Windows 人工操作待验收 | — |
| F035 | 设置列值（公式列） | Using Formulae to Set or Transform Worksheet Column Values | P0 | 已实现待验收 | Ctrl+Q 设置列值：表达式、列引用/col(A)、行号 i、常数、比较运算与 121 个函数（含整列统计、按行序列）；可勾选“保存公式，引用的列变化时自动重算”，公式随项目保存，列标题带 ƒ 标记，再次打开对话框预填公式；任何编辑提交前（连续输入时每帧）按公式依赖顺序重算，结果并入同一步撤销；自引用与循环依赖拒绝，重算出错在状态栏说明；公式长度/解析递归/树深度在构建时受限，非线性拟合整组输出在所有模式下禁止保存第二份公式；新增输入与文件兼容使用两级解析限额，原旧表达式继续可重算与执行，较长旧表达式使用有界解析栈 | 依赖顺序重算、自引用/循环拒绝、去掉公式后不再重算、项目往返的核心测试；改源单元格后公式列重算且一步撤销的界面测试；设置列值错误保留对话框回归 | — |
| F036 | 单元格公式（=A1+B1） | Using a Formula to Set Cell Values | P2 | 未开始 | 依赖图 + 增量重算 | 单元测试 | — |
| F037 | 生成数据序列（等差、随机数、正态分布） | Generating Simple Worksheet Datasets | P1 | 已实现待验收 | 工作表 → 填充序列：等差序列、均匀分布、正态分布随机数（SplitMix64 + Box–Muller），覆盖目标列，可撤销 | 单元测试：等差值精确；20000 个随机数的均值、标准差落在理论值附近；同种子可复现 | — |
| F038 | 排序（列、工作表、按多列） | Sorting Worksheet Data | P0 | 已实现待验收 | 升序/降序按一列稳定排序整张工作表，同行联动；“自定义排序…”最多三级关键字各自升/降序，可选“只排选中列”（选中列按关键字整体重排，其余列不动）；数值/文本/空值规则一致，一步撤销；自定义排序实时预览到工作表和关联图形，取消恢复原顺序 | 多关键字稳定排序、只排选中列不动其他列、关键字不在范围内拒绝的核心测试；对话框真实点击与撤销/重做回归；百万行排序基准见 docs/perf.md；排序预览、无效关键字、取消与确认后撤销重做回归 | — |
| F039 | 数据筛选 | Data Filter | P1 | 未开始 | — | 手动操作 | — |
| F040 | 条件格式（按值着色） | Conditional Formatting | P2 | 未开始 | — | 目视检查 | — |
| F041 | 转置 | Transposing Worksheets | P1 | 未开始 | — | 单元测试 | — |
| F042 | 堆叠 / 拆分列 | Stacking and Unstacking Worksheet Columns | P1 | 未开始 | — | 与 Origin 结果比较 | — |
| F043 | 合并工作表（按列、按标签） | Join Multiple Sheets by Column | P2 | 未开始 | polars join | 与 Origin 结果比较 | — |
| F044 | 数据精简（隔行抽取、平均合并） | Data Reduction | P1 | 未开始 | — | 与 Origin 结果比较 | — |
| F045 | 删除重复行 | Reduce Duplicated Rows | P2 | 未开始 | — | 单元测试 | — |
| F046 | 数据透视表 | Pivot Table | P2 | 未开始 | polars pivot | 与 Origin 结果比较 | — |
| F047 | 工作表查询（条件提取） | Worksheet Query - Quick Start | P2 | 未开始 | — | 与 Origin 结果比较 | — |
| F048 | 冻结行列 | Freeze Rows Columns | P1 | 未开始 | — | 手动操作 | — |
| F049 | 多工作表（一个工作簿多张表） | Workbooks | 不做 | 不做 | 用户 2026-10-18 决定：每个 Book 只有一张表，多表使用独立窗口；Excel 多工作表导入继续拆成独立 Book，不采用多 Sheet 容器或底部标签页 | 后续保持一表一窗口、独立命名/目录/保存/绘图引用；Excel 拆表导入沿用现有回归 | — |
| F050 | 工作簿模板 | What is Saved with the Workbook Template (*.OTW or *.OTWU) File? | P1 | 未开始 | 保存列定义与格式 | 套模板后结构一致 | — |
| F051 | 选中数据的状态栏统计 | Scrolling and Simple Data Selection in the Worksheet | P0 | 已完成 | — | 与手算比较 | — |
| F052 | 单元格内嵌图片 / 迷你图 | Inserting Graphs Images and Other Objects into Worksheet Cells | P2 | 未开始 | — | 目视检查 | — |
| F053 | 大数据量（百万行流畅滚动） | Origin Limitations | P0 | 已实现待验收 | 类型化数值列、缺失位图和可见行表格；CSV 由文本流式建表（不生成整张字符串表）；折线按像素列保留首/末/最小/最大点抽稀，散点按半像素网格去重；图形场景按图形设置、视图和所引用数据的指纹缓存，数据不变时复用。本机 release 百万行十列：导入 0.81 秒，含完整界面的首帧 246 ms、次帧 12.6 ms，进程峰值内存 494 MB（字体约 56 MB、数值列约 80 MB，原 <150 MB 目标不现实）；设置/图形窗口移动复用矢量绘制指令和文字排版，数据/设置/视图/尺寸/DPI 改变时更新；Windows 使用 wgpu FIFO 呈现 | 流式导入与行表导入结果一致、抽稀保形的核心与界面测试；百万行首帧测量（忽略测试，外部采样峰值内存），数据见 docs/perf.md；拖动修复 4f74ec7 本地完整检查与 Windows CI 通过，用户已确认快速拖动正常；不扩大为整项大数据性能验收 | — |
| F510 | 日期时间列 | Worksheet / Column Properties / Date and Time | P0 | 已实现待验收 | DateTime 列：UTC 毫秒存储、ISO8601/常见年月日写法/四位年份解析、位图/原文保留、转换/排序/v9 保存；列属性可选 4 种日期显示格式和时区（UTC−12…UTC+14 及 +5:30 等），不带时区的输入按列时区理解、带时区（Z/±hh:mm）的保持原时刻，改时区时墙上时间不变；显示、日期刻度（在当地零点取整）按列时区换算；日期列做 X 时自动日期刻度（F096）；时区随项目保存，UTC 时不写入 | 时区换算（不带/带时区输入、新输入、改回 UTC）、时区标签、当地零点刻度与项目往返测试；typed_columns 与界面命令回归；人工验收待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F511 | 分类列及类别排序 | Worksheet / Column Properties / Categorical | P0 | 已完成 | Category 采用 u32 编码和类别字典，支持缺失、编辑、转换、保存与标签排序；分类列右键“类别顺序…”可上移/下移或按名称排序（重排编码并标为有序字典，一步撤销），之后排序与作图按此顺序；分类列做 X 时为分类轴（各类别依次在 1…k，刻度标签为类别名，水平条形图在 Y 轴，缩放保留类别刻度）；类别顺序有效修改即时写入表和图；标题栏最小化/还原和叉号，关闭保留修改，撤回用撤销 | 类别重排、行位置、非法顺序拒绝、分类轴范围与刻度标签顺序、项目往返的核心测试；类别顺序对话框真实点击与撤销/重做回归；类别预览、标签保持与撤销重做回归；实时设置无底部确认/取消/关闭按钮，标题栏最小化/还原/叉号、关闭保留和开窗撤销重做回归；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | — |

## 矩阵

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F054 | 矩阵窗口（二维网格数据） | Basic Matrixbook and Matrixsheet Operations | P1 | 未开始 | ndarray | 手动操作 | — |
| F055 | 矩阵 XY 坐标映射 | Matrixsheet Dimensions and XY Coordinates | P1 | 未开始 | — | 与 Origin 比较 | — |
| F056 | 设置矩阵值（公式） | Setting Matrix Values | P1 | 未开始 | 复用公式解析器 | 与 Origin 比较 | — |
| F057 | 矩阵转置、旋转、翻转、缩放 | Manipulating Matrix Data | P2 | 未开始 | — | 单元测试 | — |
| F058 | 工作表直接转矩阵 | Direct Worksheet to Matrix Conversion | P1 | 未开始 | — | 单元测试 | — |
| F059 | XYZ 网格化：规则数据 | Gridding of Regularly Spaced Data | P1 | 未开始 | — | 与 Origin 比较 | — |
| F060 | XYZ 网格化：随机点（Delaunay 插值、克里金、Renka-Cline） | Gridding Methods for Randomly Spaced Data | P1 | 未开始 | spade（Delaunay）+ 自写克里金 | 与 Origin 网格结果比较 | — |
| F061 | 虚拟矩阵 | Managing Virtual Matrix | P2 | 未开始 | — | — | — |

## 导入导出

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F062 | CSV / TXT 导入（自动识别分隔符） | Importing Simple ASCII Data Files | P0 | 已完成 | csv.rs | 单元测试 | — |
| F063 | GBK / UTF-8 编码识别 | Customizing the ASCII Import Options | P0 | 已完成 | encoding_rs | 单元测试 | — |
| F064 | 表头自动拆分长名称与单位 | Customizing the ASCII Import Options | P0 | 已完成 | — | 单元测试 | — |
| F065 | 导入向导（跳过表头行、指定分隔符、预览） | The Import Wizard | P0 | 未开始 | 导入预览、跳过行与分隔符/表头设置向导未实现；按用户要求暂缓，现有直接导入继续可用 | 手动操作 | — |
| F066 | 导入 Excel（.xlsx / .xls） | Importing from Excel | P0 | 已实现待验收 | 导入 .xlsx/.xlsm（zip + roxmltree）与旧版 .xls（自写 OLE2 复合文档 + BIFF8 读取：FAT/DIFAT/迷你流，共享字符串跨 CONTINUE 记录，NUMBER/RK/MULRK/LABELSST/LABEL/BOOLERR/FORMULA 缓存值）；按单元格数字格式（内置 14–22、27–36、45–47、50–58 含中文日期及自定义格式）把日期序列号写成日期，支持 1904 日期制；合并单元格用左上角的值填满；数值按 15 位有效数字保留；全部为日期写法的列导入为日期时间列；每张非空工作表一个工作簿；加密或 Excel 95 文件给出提示 | Excel 16 另存的同一内容 .xls 与 .xlsx 读取结果逐格一致（含中文日期、日期时间、合并单元格、公式数值与字符串结果、布尔、1200 个跨记录共享字符串、301 字长文本、大数值精度）；原有 openpyxl/手写 xlsx 回归通过；RK 解码单元测试 | — |
| F067 | 导出 Excel | Exporting Worksheets and Matrices | P1 | 未开始 | rust_xlsxwriter | 用 Excel 打开检查 | — |
| F068 | 导出 CSV | Exporting Worksheets and Matrices | P0 | 已实现待验收 | 工作表 → 导出 CSV：UTF-8 BOM、CRLF，表头为“长名称 (单位)”，与 .scv 内的 CSV 同一实现 | 导出后重新导入的往返测试；Excel 打开待人工验收 | — |
| F069 | 拖放文件导入 | Importing Data by Drag-and-Drop | P0 | 已实现待验收 | 把文件拖进窗口：数据文件逐个导入为工作簿并合为一步撤销，项目文件（.scv / .scproj）直接打开；拖动时显示提示层 | 拖放事件回归（导入、撤销、打开项目）；资源管理器实际拖放待人工验收 | — |
| F070 | 批量导入多个文件 | Importing Simple ASCII Data Files | P1 | 已实现待验收 | 已有多文件拖入入口：数据文件逐个创建工作簿并合为一步撤销；混入项目时只打开第一个项目，其余忽略。当前文件对话框为单选，批量入口使用拖放 | 多文件拖入、结果写回、项目混入规则及一步撤销回归通过；Windows 多种格式、大批量文件与部分失败手工验收待做 | — |
| F071 | 从文件名和表头提取变量 | Extracting Variables from the File Name and Header | P2 | 未开始 | — | 单元测试 | — |
| F072 | 二进制数据导入 | Importing Simple Binary Files with the Import Wizard | P2 | 未开始 | — | 单元测试 | — |
| F073 | 第三方格式：MATLAB .mat | Importing Third Party Files | P2 | 未开始 | matfile | 与 MATLAB 比较 | — |
| F074 | 第三方格式：HDF5 / NetCDF | Importing Third Party Files | P2 | 未开始 | hdf5 / netcdf（需 C 库） | 与 h5py 比较 | — |
| F075 | 第三方格式：仪器数据（SPC、JCAMP-DX 等） | Importing Third Party Files | P2 | 未开始 | 逐个格式实现 | 与 Origin 导入结果比较 | — |
| F076 | Parquet / Arrow | — | P2 | 未开始 | polars | 往返测试 | — |
| F077 | 数据库导入（SQL 查询） | Importing Data from a Database | P2 | 未开始 | sqlx / duckdb | 手动操作 | — |
| F078 | 数据连接器（文件更新后自动重新导入） | Data Connector | P2 | 未开始 | notify 监听文件 | 改文件后检查 | — |
| F079 | 导入 Origin 工程中的工作表 | Importing from Origin Project | 不做 | 不做 | 私有格式 | — | — |

## 绘图基础

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F080 | 按选列自动配对 X / Y 绘图 | Creating Graphs from Graph Templates | P0 | 已完成 | plot_from_selection | 单元测试 | — |
| F081 | 按选区（部分行）绘图 | Creating Graphs from Graph Templates | P0 | 已实现待验收 | 工作表矩形选区经 picked/PlotSpec.rows 接入作图、图库与保存，按原始行截取 | 选区行范围、无效行、项目往返与 SVG 回归已通过；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F082 | 页面-图层-曲线 三层结构 | The Page-Layer-Plot Hierarchy | P0 | 已实现待验收 | 图层以有序 Vec<Plot> 唯一保存曲线；绘图/分析/保存共用 Plot 描述与 DataRef/CurveMapping，场景以 PreparedPlot 和 AxisSet/YAxis 共用当前 X/左 Y/右 Y 标尺及定位。PlotSpec 仅作编辑草稿和旧格式边界转换；v15 保存右 Y 独立范围/格式，读取 v0–v14；三轴以上仍待推进 | 跨表同名列、行范围、混合图型、独立左右 Y 范围/格式、v14 默认外观迁移、v15 JSON/SVG 字节往返、排序/移除/撤销重做及来源解析回归；14 图型 SVG 黄金回归；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F083 | 往图层添加 / 移除曲线 | Adding or Swapping Data Plots in the Graph Layer | P0 | 已实现待验收 | 图层内容窗口以单列纵向呈现可用工作簿/列及当前图层曲线和 X/Y 来源；支持添加、移除、上移/下移、名称排序和左右 Y 轴选择。修改即时写入并支持撤销重做；标题栏最小化/还原与叉号关闭，关闭保留已生效修改；侧栏吸附、右键快捷入口和浮动柱上下端配对保留。清空后换表清理旧标签列，同表加回保留 X、行范围和标签设置 | 真实界面添加/移除/排序/换轴/来源显示、目标图层隔离、即时预览、关窗保留与开窗撤销重做、换表清理及同表设置保留、浮动柱配对和 v15 保存重开回归；matplotlib 同数据视觉对照待做，Windows 人工验收待做 | — |
| F084 | 多图层图形 | Creating Multi-Layer Graphs | P0 | 已实现待验收 | 多图层增删、跨工作簿数据、独立属性/裁剪/视图、分析写回目标层已实现 | 跨工作簿、布局、裁剪、视图、保存与撤销回归已通过；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F085 | 函数绘图（y = f(x)） | Plotting Mathematical Functions | P1 | 未开始 | 复用公式解析器 | 与解析值比较 | — |
| F086 | 分组绘图（按分类列分组） | Plotting Data by Groups | P1 | 未开始 | — | 与 Origin 比较 | — |
| F087 | 图形缩放、平移 | Zooming on the Graph Page and inside Layer | P0 | 已完成 | 自绘 plot.rs/coord.rs：图层独立缩放、平移和视图复位；固定页面画布整体缩放 | 手动操作 | — |
| F088 | 批量绘图（同一模板换数据） | Batch Plotting | P1 | 未开始 | — | 手动操作 | — |
| F089 | 图形模板（保存样式，换数据一键出图） | Creating Graphs from Graph Templates | P0 | 已实现待验收 | 在“绘图 → 模板”中将当前页面/图层/轴设置/曲线样式保存为独立 `.scgt` 模板；不保存源工作簿编号或数据，曲线按列短名重绑定到选中的目标 Book，缺少所需列时拒绝应用；模板可跨项目复用并生成新图 | 模板文件保存/加载；套用到列标识匹配的不同工作簿后核对图型、图层、曲线样式、轴设置和数据来源；缺列/损坏模板不改项目；单步撤销重做；Windows 人工验收待做 | — |
| F090 | 格式复制粘贴 | Copying and Pasting Formats | P1 | 未开始 | — | 手动操作 | — |
| F091 | 主题（图形样式主题） | Graph Formats and Themes | P1 | 未开始 | 已有 Nature 默认配色与版式；图形主题的保存、切换和应用入口待做，默认样式单独计入现有绘图能力 | 目视检查 | — |
| F092 | 坐标轴范围与刻度 | Controlling the Graph Axis Display Range | P0 | 已完成 | ticks.rs（1/2/5 步长）；左 Y 和右 Y 分别支持自动或手动设定范围 | 自动/手动范围和独立左右 Y 范围场景回归；Origin 刻度与范围对照 | — |
| F093 | 对数轴 | Axes Dialog Reference | P0 | 已完成 | — | 与 Origin 比较 | — |
| F094 | 其他坐标类型：倒数、概率、log2、ln、Logit | Axes Dialog Reference | P1 | 未开始 | — | 与 Origin 比较 | — |
| F095 | 坐标轴断点 | Axes Dialog Reference | P1 | 未开始 | — | 目视比较 | — |
| F096 | 刻度标签格式（科学计数、日期、自定义） | Axes Dialog Reference | P0 | 已完成 | ticks.rs：普通数值、a×10ⁿ 科学计数、对数刻度；日期时间列做 X 时自动用日期刻度（秒、分、时、日按 UTC 整步长，月、年按日历取整）；属性窗口可为 X、左 Y、右 Y 分别设数字刻度格式（固定小数位、科学计数、工程单位、百分比），右 Y 缺省跟随左 Y；刻度标签按共享实际字宽排版 | 数值/对数刻度回归；日期轴步长与标签测试；左/右 Y 独立范围和刻度格式、v14 默认值迁移、不同缩放/DPI 字宽回归；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | — |
| F097 | 次刻度、刻度方向、四边框线 | Axes Dialog Reference | P0 | 已完成 | 屏幕/SVG 共用主次刻度；默认 Nature 风格只画左轴和下轴、刻度朝外；属性窗口可开“四边框”（闭合坐标框，顶边和右边画无标签的镜像主次刻度）并选刻度方向朝外/朝内/内外，朝内时刻度标签贴近坐标轴、坐标框相应放大；未设置时项目文件与旧图不变 | 默认朝外两轴回归与 golden SVG 不变；四边框闭合折线、朝内刻度全部在框内、四边均有刻度、坐标框放大的单元测试；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | — |
| F098 | 网格线 | Axes Dialog Reference | P0 | 已完成 | — | 目视检查 | — |
| F099 | 图例（自动更新） | Graph Legends | P0 | 已完成 | 图例按曲线名称和当前图层更新，并按实际图元避让；四角都会遮挡数据时放到绘图区右侧外部；图例手动编辑/拖动另见 F100；图例宽度和每行高度使用真实字体度量，长名称自动换行 | legend_placement 回归与目视检查；实际图元避让、换行及页面边界回归 | — |
| F100 | 图例手动编辑与位置拖动 | Manual Control of the Graph Legend | P1 | 已实现待验收 | 图例可拖到页面任意位置、双击逐曲线编辑、Delete/右键删除、恢复自动避让；页面比例定位，缩放与保存重开保持位置，支持撤销重做；屏幕和 SVG/PNG/PDF 复用同一版面 | 真实鼠标拖动/双击、一步撤销重做、自动布局还原、保存往返、插图覆盖顺序与屏幕/导出排版回归；Windows 人工验收待做；实时设置无底部确认/取消/关闭按钮，标题栏最小化/还原/叉号、关闭保留和开窗撤销重做回归 | 图例操作：已完成 0/2，待验收 2，部分实现 0；文字编辑、手动位置拖动与恢复自动避让均已接入待验收 |
| F101 | 色阶条（Color Scale） | Color Scales | P1 | 未开始 | — | 目视检查 | — |
| F102 | 文字标签（上下标、希腊字母、富文本） | Creating Text Labels | P0 | 已实现待验收 | 标题、轴标题、图例、指定列标签及自由文字支持上下标/希腊字母/局部粗斜体；自由文字可添加、拖动、双击编辑、删除，编辑即时刷新；页面比例位置和 pt 字号由 v13 引入，当前 v18 兼容读取 v0–v17，支持撤销重做，屏幕/SVG/PNG/PDF 共用版面 | 真实格式按钮/希腊字母菜单、添加/编辑/拖动/删除与一步撤销重做、焦点隔离、JSON/SCV 保存重开、旧项目默认值；屏幕/SVG 多缩放/DPI 原生字形，PDF 字体与独立渲染对照；Windows 人工验收待做；实时设置无底部确认/取消/关闭按钮，标题栏最小化/还原/叉号、关闭保留和开窗撤销重做回归 | 科学文字：已完成 0/4，待验收 4，部分实现 0；上下标、希腊字母、局部粗斜体、自由文字对象均已接入待验收。复杂整形和完整数学公式另计 |
| F103 | LaTeX 公式 | Creating Text Labels | P1 | 未开始 | 评估 typst 或 MathJax 转 SVG | 目视比较 | — |
| F104 | 绘图对象（直线、箭头、矩形、圆） | Creating and Editing Drawing Objects | P1 | 未开始 | — | 手动操作 | — |
| F105 | 数据标签 | Adding Labels to Plots | P1 | 已实现待验收 | 数值/X-Y/原始行号/指定列文本，字号/偏移、边缘定位、屏幕/SVG 和撤销均已接入；文字边界使用共享真实度量，过长文本按绘图区适配；启用科学文字后，指定列文本支持上下标与局部粗斜体，共用格式排版 | 行对齐、文本转义、柱段值、图层与真实开关回归已通过；matplotlib 同数据视觉对照待做；真实字宽/行高、屏幕缩放及 SVG 文字边界回归 | 标签来源：已完成 0/4，待验收 4，部分实现 0；数值、X/Y、行号、指定列；自动化回归已通过，Origin 对照待验收 |
| F106 | 误差棒 | Adding Error Bars to Your Graph | P0 | 已完成 | 对称 X/Y 误差列、稳定自动关联、端帽/线宽、分组定位与屏幕/SVG 已实现；本条当前范围为对称误差 | 行对齐、对数裁剪、分组柱、往返与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F107 | 插入图片 / 表格到图中 | Inserting Image into Graph | P2 | 未开始 | — | 目视检查 | — |
| F108 | 合并多张图 / 拆分图层 | The Merge Graph Dialog Box | P1 | 未开始 | — | 目视检查 | — |
| F109 | 图层排列（行列网格、间距） | The Arrange Layers Dialog Box | P0 | 已完成 | 上下/左右/四面板快捷创建、网格列数、自由矩形与透明叠放；网格面板按行列统一边距对齐坐标框；属性窗口可设网格横向/纵向间距（mm）；图层的 X、Y 轴可分别联动到另一图层（范围与刻度跟随目标图层，目标缩放时一起变化，删除目标图层时联动取消，联动到自身或不存在的图层被拒绝）；未设置时项目文件不变 | 间距平移面板与缩小宽度、联动范围与刻度相同、目标缩放跟随、无效联动拒绝、删除图层后联动清理的布局测试；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | — |
| F110 | 布局页（多图拼版） | Using the Layout Page Window | P1 | 未开始 | — | 目视检查 | — |
| F111 | 曲线颜色、线型、符号递增 | Customizing Data Plot Colors | P0 | 已完成 | Nature 风格默认色系（深蓝、砖红、青、紫……）与圆点优先的符号顺序 | 目视比较 | — |
| F112 | 按数据集控制颜色 / 大小（颜色映射） | Using a Dataset to Control Plot Color | P1 | 未开始 | — | 目视比较 | — |
| F113 | 绘图属性对话框（Plot Details） | Plot Details Controls | P0 | 已实现待验收 | 属性窗口：页面尺寸/边距/背景/间距、图层位置与透明、图型、列映射、左/右轴标题、范围与数字格式、对数/刻度方向、四边框、线宽/符号、误差棒和数据标签；图层有右轴曲线时可设右 Y 自动/手动范围和独立数字格式（缺省跟随左 Y）；“曲线样式”可逐条设置图型、颜色、符号、线宽、符号大小和右轴；全部修改一步撤销并即时刷新；紧凑单列，可吸附/堆叠/折叠/浮动；当前另有独立右 Y 范围/格式、参考线、符号独立边框/填充和颜色/大小/形状列映射，修改即时更新 | 真实属性控件与撤销回归；逐曲线覆盖（散点、颜色、线宽、右轴）与项目往返的场景测试；Windows 人工验收待做；重绘请求检查；绘图属性与图层内容同时停靠、工作区避让和布局操作不修改项目的回归 | — |
| F114 | 显示范围（只画部分行） | Editing the Plots Display Range | P1 | 部分实现 | 从工作表选区创建并保存部分行绘图已实现；图形中的独立行范围编辑入口待做 | PlotSpec.rows 截取/保存回归已通过；补齐独立编辑入口后验收 | — |
| F115 | 页面尺寸、背景、边距 | The (Plot Details) Print/Dimensions Tab | P0 | 已实现待验收 | 页面毫米尺寸、固定逻辑画布、图层透明背景；新建图默认 A4 宽 210×157.5 mm，字号、点和线宽按页宽（扣除左右边距）相对 89 mm 单栏基准等比放大；属性窗口可设四边页边距（网格图层排在边距以内，自定义位置图层按整页定位）和页面背景色（图层与图例底色跟随），屏幕、SVG、PNG 共用；未设置时项目文件不变 | 边距平移网格图层、背景色替换全部白底与整页背景、项目往返、未设置时不写字段、负数/过大边距拒绝的核心测试；屏幕截图目视核对；Windows 人工验收待做 | — |

## 图形探索与小工具

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F116 | 读取坐标 / 数据点提示 | Reading Coordinate Values in the Graph | P0 | 已实现待验收 | 自绘 plot.rs：悬停坐标、十字光标与图层定位；鼠标 8 像素内吸附到最近的数据点（不含拟合等结果曲线），画出高亮圈并弹出提示（曲线名、原始行号、X、Y），右下角坐标栏同步显示该点的值；堆叠柱显示本段值，水平条形按交换后的轴定位，统计变换图层不显示行号；读取共用 PreparedPlot 与 AxisSet，分组柱/条形柱按实际柱位中心定位，保留原始行和堆叠段值 | 坐标逆变换、图层命中与缩放回归；真实指针事件吸附到指定行、远离数据不吸附；堆叠柱段值与水平条形定位的核心测试；Windows 人工验收待做；实际渲染矩形端点拾取回归覆盖分组柱、堆叠柱和条形图 | — |
| F117 | 数据高亮联动（图 ↔ 工作表） | Data Highlighter: Selecting Points in both Graph and Source Worksheet | P1 | 未开始 | — | 手动操作 | — |
| F118 | 屏蔽数据点（Mask） | Masking Data Points in the Graph or Worksheet | P1 | 未开始 | 掩码位图 | 屏蔽后拟合结果与 Origin 比较 | — |
| F119 | 框选 / 删除 / 移动数据点 | Drawing or Moving, or Deleting Data Points | P2 | 未开始 | — | 手动操作 | — |
| F120 | 异常值检测 | Detecting Outliers | P2 | 部分实现 | 统计菜单已有 Grubbs 检验，报告最远点及显著性；图内异常点标记、交互筛选及更多检测方法待做 | Grubbs 数值与界面输出回归通过；图内标记、联动与范围交互待实现 | — |
| F121 | 局部放大视图 | Magnifying or Zooming In on a Portion of the Graph | P2 | 未开始 | — | 手动操作 | — |
| F122 | 快速拟合小工具（框选范围实时拟合） | The Quick Fit Gadget | P1 | 未开始 | 复用拟合引擎 | 与 Origin 比较 | — |
| F123 | 快速寻峰小工具 | The Quick Peaks Gadget | P1 | 未开始 | — | 与 Origin 比较 | — |
| F124 | 积分 / 微分 / 插值小工具 | The Integrate Gadget | P1 | 未开始 | — | 与 Origin 比较 | — |
| F125 | 交点小工具 | The Intersect Gadget | P2 | 未开始 | — | 与 Origin 比较 | — |
| F126 | FFT 小工具 | The FFT Gadget | P2 | 未开始 | — | 与 Origin 比较 | — |
| F127 | 统计小工具 | The Statistics Gadget | P2 | 未开始 | — | 与 Origin 比较 | — |
| F128 | 垂直游标 | The Vertical Cursor Gadget | P2 | 未开始 | — | 手动操作 | — |
| F129 | 曲线数字化（从图片取点） | Digitizer | P1 | 未开始 | image + 坐标标定 | 用已知图片检验 | — |
| F502 | 图形数据切片器 | Graph Exploration / Graph Data Slicer | P2 | 未开始 | A1 数据筛选视图驱动图形和分析 | 筛选、复位、源数据更新与保存 | — |
| F503 | 带浏览面板的图形 | Graph Exploration / Browser Graph | P2 | 未开始 | 图形浏览面板切换数据集与范围 | 数据切换、图形状态和保存 | — |
| F504 | 图像曲线数字化与数据提取 | Gadgets / Digitizer | P2 | 未开始 | 导入图片、坐标标定、点选与曲线采样 | 已知坐标图片的恢复误差和输出 | — |

## 回归与拟合

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F130 | 线性回归（斜率、截距、标准误、R²、残差） | The Linear Regression Dialog Box | P0 | 已实现待验收 | 斜率/截距、标准误、R²、残差平方和及报告；拟合时为每条数据写入逐点残差列（Y − 拟合值，与原数据同行，未参与拟合的行留空），拟合结果对话框“残差图”生成残差对 X 的散点图，一步撤销 | 残差逐行等于 Y − 拟合值且和为 0、残差图曲线映射的核心测试；残差图按钮的真实界面点击与撤销/重做回归；SciPy/NIST 参考值对照待做；NoInt1/NoInt2 过原点回归尚不支持，登记 S2-5 | — |
| F131 | 多项式回归 | The Polynomial Regression Dialog Box | P0 | 已完成 | X 标准化后使用列主元 Householder QR 和残差修正；2–6 阶拟合、参数标准误、R²、曲线及撤销已接入 | 解析/仿真回归已通过；NIST StRD 认证对照及已知失败见 docs/parity/nist.md；NoInt1/NoInt2 过原点回归尚不支持，登记 S2-5；可复现 NumPy/SciPy 多组参考见 tests/reference 与 tools/reference/README.md（六阶容差原因单列），Windows 人工结果另记；2026-11 验收：自动测试 + 参考值 | 多项式阶次：已完成 0/5，待验收 5，部分实现 0；2–6 阶；认证数据与稳定性验收待完成 |
| F132 | 多元线性回归 | The Multiple Linear Regression Dialog Box | P1 | 已完成 | 分析 → 多元线性回归：先点选自变量、最后点选因变量；参数表（值、标准误、t、p）与 R²、调整 R²、F 检验、残差标准差 | 与 numpy.linalg.lstsq 及 scipy.stats.t / f 计算的参数、标准误、p 值一致（1e-9），含缺失行与共线提示；NIST StRD 认证对照及已知失败见 docs/parity/nist.md；2026-11 验收：自动测试 + 参考值 | — |
| F133 | 含 X 误差的线性拟合（York 法） | The Linear Fit with X Error Dialog (Pro Only) | P2 | 未开始 | — | 与 Origin 比较 | — |
| F134 | 非线性拟合引擎（Levenberg–Marquardt） | Theory of Nonlinear Curve Fitting | P0 | 已完成 | Levenberg–Marquardt（中心差分雅可比、自适应阻尼），参数固定、上下界与线性约束（活动集），标准误 (JᵀJ)⁻¹·RSS/(n−p)（有约束时在零空间计算），28 种内置函数与自动初值；用户自定义函数见 F142；自定义标量函数复用同一求解器，用户函数库范围见 F142；新结果保存完整重算定义，支持自动/手动/不重算，报告与曲线/带同次求解同步。 | 原 24 种函数的 SciPy 1.17.1 curve_fit 参数/标准误参考回归、上下界与线性约束回归通过；新增 Rational1 自动初值和自定义公式合成数据回归通过；NIST StRD 认证对照及已知失败见 docs/parity/nist.md；Nelson 双自变量尚不支持，登记 S2-5；Gauss1–3/ENSO 已纳入正式测试；Gauss1–3 双起点达标，ENSO 两起点符合原始 LRE 或认证标准误尺度的验收条件；Lanczos1 残差与标准误的浮点限制继续记录；2026-11 验收：自动测试 + 参考值 | — |
| F135 | 参数初值自动估计 | Fitting Functions | P0 | 已实现待验收 | 当前 28 种内置非线性模型均有数据驱动初值估计，含 Rational1/12/22/33；支持固定参数并报告未收敛状态。自定义函数当前由用户提供初值 | 原模型初值/SciPy 拟合回归及 Rational1 自动初值合成数据回归通过；退化数据与复杂初值实例验收待做；NIST StRD 认证对照及已知失败见 docs/parity/nist.md；StRD 使用官方给定初值，自动初值本身继续单独验收 | 当前模型自动初值：已完成 0/28，待验收 28，部分实现 0；28 为当前 Model::ALL 数量，含四种有理模型。完整函数库见 F514；自定义函数由用户输入初值，复杂数据与 Origin 实例待验收。 |
| F136 | 参数固定、上下界、线性约束 | Parameters,Bounds,Constraints and Weighting | P0 | 已实现待验收 | 非线性拟合对话框每个参数可设固定值、下限、上限，并可写参数间线性约束（每行一条，=、<=、>=，如 A1 + A2 = 1、t2 >= t1 + 3）：固定参数不计入自由度；上下界与约束用活动集方法——等式与贴边且下降方向朝外的不等式在零空间求步，其余不等式和上下界截断步长，挡路的约束加入活动集重新求步；初值先投影到可行域，约束互相矛盾时报错；标准误按停止点约束零空间计算，自由度扣除约束个数；报告列出约束并标出停在边界的参数；A5 第二批内置/自定义报告重新载入原初值、固定参数、上下界及线性约束，执行后原位更新结果组，一步撤销重做。 | 上下界与 SciPy curve_fit(bounds, trf) 一致；等式约束、生效与不生效的不等式与 SciPy 代入消元拟合的参数（1e-5）、标准误（1e-3）、RSS 一致；约束解析与矛盾约束报错测试；对话框约束生效与写错保留对话框的界面回归 | — |
| F137 | 加权拟合（误差列权重等） | Parameters,Bounds,Constraints and Weighting | P1 | 未开始 | — | 与 Origin 比较 | — |
| F138 | 拟合报告（参数表、ANOVA、置信带、预测带） | Interpreting Regression Results | P0 | 已实现待验收 | 线性/多项式报告：参数、标准误、t、双侧 p，R²/调整 R²/Pearson r/RSS/N，以及方差分析表（回归、残差、总计的自由度、平方和、均方、F、p）；非线性报告：参数、标准误、t、p 与统计量；两类报告的“置信/预测带”按钮按 95% 写出置信下/上限、预测下/上限四列并加到拟合曲线所在图层（多项式用参数协方差，非线性用 Δ 法），一步撤销；报告可复制或保存为 Markdown Note 持久保存；多元回归另有 t/p 与整体 F 检验。A5 第二批非线性数值报告随结果列持久保存，通过“分析 → 拟合报告…”重开，使用 Book/结果列/独立报告编号隔离多个拟合。报告与曲线同次重算，失败保留上次结果并标记失败，来源删除保留快照；原位重编辑完整设置，保留样式，报告跟随撤销重做。Note 为用户快照。统一报告表及线性/其他统计联动仍待推进。；重算保留用户备注及跨 Book 名称，草稿比较排除重算缓存，手动重算恢复改动过的输出；报告关联显示实际工作簿名，重编辑按钮集中在报告块末尾。拟合报告区分收敛、浮点精度极限和失败，状态随 v22 项目保存。线性和多项式回归使用列主元 QR，NIST Longley 及 Wampler1–5 达到原有精度阈值。非线性拟合支持自适应差分和停滞精修。拟合结果的高相对标准误、高协方差条件数或负调整 R² 会触发局部极小或不可辨识提示 | 多项式置信/预测带、方差分析、t/p 与 NumPy 范德蒙德最小二乘 + scipy.stats 一致（1e-9，p 值 1e-6，极小 p 值不下溢）；非线性置信/预测带与 scipy curve_fit pcov + 解析梯度 Δ 法一致；按钮真实点击与撤销/重做回归；报告同步/失败保留/重编辑/跨表隔离/删除与标识复用/v19 兼容/v20 往返/撤销重做/Note 保留回归，Windows 人工验收待做；NIST MGH17 和 BoxBOD 双起点提示检查，界面、复制、Note、重拟合和新旧项目往返回归 | — |
| F139 | 残差图（残差-自变量、正态概率图） | Residual Plot Analysis | P1 | 未开始 | — | 与 Origin 比较 | — |
| F140 | 多峰拟合 | Multiple Curves/Multiple Peaks | P1 | 未开始 | — | 与 Origin 比较 | — |
| F141 | 全局拟合（多组数据共享参数） | Multiple Curves/Multiple Peaks | P1 | 未开始 | — | 与 Origin 比较 | — |
| F142 | 用户自定义拟合函数 | Fitting Functions | P0 | 已实现待验收 | 自定义标量拟合支持 1–7 个命名参数、手动初值和线性等式/不等式约束，复用公式解析器与 LM。项目函数库可保存、载入、重命名及重新编辑；.scfn v1 文件供跨项目复用。新增函数文件载入直接进入当前项目，同名文件先载入编辑器，点击保存函数后更新；拟合报告可载入原函数快照、原初值及完整限制，原位更新结果组和报告。函数库新增/修改与拟合结果均支持一步撤销重做，项目 v20 保存完整定义和数值报告 | 7 项新增回归覆盖稳定编号/重命名/过期草稿、函数库内存预算、v17 空库与 v18 JSON/SCV 往返、独立函数文件载入后实际拟合、损坏/新版/重复输入和保存失败保留原文件；原生文件选择、Windows 人工验收及完整 NIST 对照待做；NIST StRD 认证对照及已知失败见 docs/parity/nist.md；Nelson 双自变量尚不支持，登记 S2-5；Gauss1–3/ENSO 已纳入正式测试；Gauss1–3 双起点达标，ENSO 参数 LRE 约 6.03/5.93 < 7，登记已知失败，RSS/SE 达标；自定义参数上限 12，8–12 参数文件往返、12 参数拟合、13 参数拒绝与 9 参数真实编辑/滚动/报告/撤销回归；含 8–12 参数函数的文件需本批程序读取；正常 1229×706/900×650 窗口下 9/12 参数末行完全可见且可编辑的回归；参数与表单统一滚动 | 自定义拟合工作流：已完成 0/5，待验收 5，部分实现 0；表达式/参数、初值/约束、求解与报告、非法输入/撤销、函数库及 .scfn 保存/载入/复用已接入待验收。A5 已接入非线性重算与持久报告、原设置重编辑；函数库定义批量联动和统一报告表仍待做。 |
| F143 | 隐函数拟合（圆、椭圆等 5 个） | Nonlinear Implicit Curve Fitting | P2 | 未开始 | 正交距离回归 | 与 Origin 比较 | 隐函数：已完成 0/5，待验收 0，部分实现 0；第三阶段路线参考数量，目录待逐项核对 |
| F144 | 曲面拟合（22 个内置函数） | Non-linear Surface Fitting | P2 | 未开始 | — | 与 Origin 比较 | 曲面函数：已完成 0/22，待验收 0，部分实现 0；第三阶段路线参考数量，目录待逐项核对 |
| F145 | 拟合比较（F 检验、AIC、BIC） | Fit Comparison | P1 | 未开始 | — | 与 Origin 比较 | — |
| F146 | 标准曲线反查 X / Y | Finding Y/X from X/Y Standard Curves | P1 | 部分实现 | 分析 → 标准曲线反查：按点选顺序取标准浓度、标准响应、样品响应，线性标准曲线反查浓度并给出 95% 置信区间半宽（Miller & Miller 公式）；由 X 求 Y、非线性标准曲线待做 | 与 numpy.polyfit 和 scipy.stats.t 计算的反查值、标准差、置信区间一致（1e-9） | — |
| F147 | 快速拟合（菜单一键） | Quick Fit | P0 | 已实现待验收 | 线性拟合一键命令、多项式阶数对话框、报告和结果写回已接入 | 命令、对话框确认/取消、目标图层与撤销回归已通过；完整人工工作流验收待做 | — |
| F148 | 内置拟合函数：峰函数（Gauss、Lorentz、Voigt、PsdVoigt、LogNormal 等） | Curve Fitting Functions（共 239 个） | P0 | 已实现待验收 | 峰函数：Gauss、Lorentz、Voigt（Gauss wG 与 Lorentz wL 卷积，面积 A；Faddeeva 函数用 Weideman N=64 有理展开）、PsdVoigt1、LogNormal，公式与参数名同 Origin，均有数据驱动初值 | 从自动初值收敛到 SciPy 1.17.1 curve_fit（p0 取真值）的参数（1e-5）、标准误（1e-3）与 RSS；Voigt 线形与 scipy.special.voigt_profile 一致（< 1e-12） | — |
| F149 | 内置拟合函数：指数类（ExpDec1-3、ExpGrow、ExpAssoc 等） | Curve Fitting Functions（共 239 个） | P0 | 已实现待验收 | 指数类：ExpDec1/2/3、ExpGro1、ExpAssoc，公式与参数名同 Origin，均有数据驱动初值（多指数由单指数估计拆分） | 从自动初值收敛到 SciPy 1.17.1 curve_fit 的参数（1e-5）、标准误（1e-3）与 RSS | — |
| F150 | 内置拟合函数：S 形与生长（Boltzmann、Logistic、DoseResp、Hill、Gompertz 等） | Curve Fitting Functions（共 239 个） | P0 | 已实现待验收 | S 形与生长：Boltzmann、Logistic、DoseResp、Hill、Gompertz，公式与参数名同 Origin，均有数据驱动初值 | 从自动初值收敛到 SciPy 1.17.1 curve_fit 的参数（1e-5）、标准误（1e-3）与 RSS | — |
| F151 | 内置拟合函数：幂函数与对数（Power、Allometric、Log 等） | Curve Fitting Functions（共 239 个） | P0 | 已实现待验收 | 幂函数与对数：Allometric1、Allometric2、Power1、Log3P1，公式与参数名同 Origin，均有数据驱动初值 | 从自动初值收敛到 SciPy 1.17.1 curve_fit 的参数（1e-5）、标准误（1e-3）与 RSS | — |
| F152 | 内置拟合函数：多项式与有理函数（Poly、Rational 等） | Curve Fitting Functions（共 239 个） | P0 | 部分实现 | 线性入口提供一阶，多项式入口提供 2–6 阶；有理函数提供 Rational1（一阶/一阶）、Rational12（一阶/二阶）、Rational22（二阶/二阶）、Rational33（三阶/三阶），新增名按阶数定义，不作 Origin 别名。复用自动初值、解析导数、固定/上下界/线性约束、报告和结果曲线；分母极点处断线，项目保存和撤销重做沿用现有流程。更多参数化、多项式变体和数据驱动目录待做 | 四个有理模型与 SciPy 1.17.1 参数/标准误/RSS 对照；Rational33 的 NIST Thurber 37 点、自动初值与两组官方起点对照；完整 NIST 参考值对照待做 | 当前多项式阶数：已完成 0/6，待验收 6，部分实现 0；一阶走线性拟合入口，2–6 阶走多项式入口；按支持阶数计数，不折算为 239 项参考函数目录进度。；当前有理模型：已完成 0/4，待验收 4，部分实现 0；Rational1/12/22/33 四个模型已接入，数字描述阶数，独立数值回归通过，人工操作与 Origin 对照待验收；不折算为 239 项目录已完成数。 |
| F153 | 内置拟合函数：波形（Sine、SineDamp、Pulse、SquareWave 等） | Curve Fitting Functions（共 239 个） | P1 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 | — |
| F154 | 内置拟合函数：酶动力学（MichaelisMenten、各类抑制模型等） | Curve Fitting Functions（共 239 个） | P1 | 部分实现 | 酶动力学：MichaelisMenten（Hanes–Woolf 线性化估计初值）；各类抑制模型待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值（线性化回归）收敛 | — |
| F155 | 内置拟合函数：吸附与结合（Langmuir、Freundlich、BET、OneSiteBind 等） | Curve Fitting Functions（共 239 个） | P1 | 部分实现 | 吸附：Langmuir、Freundlich 等温线，准一级、准二级吸附动力学（线性化估计初值）；BET、OneSiteBind 等待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值（线性化回归）收敛 | — |
| F156 | 内置拟合函数：其余专业函数（流变、热力学、统计分布等） | Curve Fitting Functions（共 239 个） | P2 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 | — |
| F157 | 寻峰分析专用函数（Bigaussian、FraserSuzuki 等 7 个） | Peak Analyzer Functions | P2 | 未开始 | — | 与 Origin 比较 | 寻峰函数：已完成 0/7，待验收 0，部分实现 0；第三阶段路线参考数量，目录待逐项核对 |
| F514 | 内置拟合函数库总览 | Fitting Function Library | P1 | 部分实现 | 当前 Model::ALL 为 28 个内置非线性模型（原 24 个加四个有理模型）；另有线性及 2–6 阶多项式入口和支持项目保存/重编辑和 .scfn 跨项目复用的自定义标量拟合。内置模型：Gauss、Lorentz、Voigt、PsdVoigt1、LogNormal、ExpDec1、ExpDec2、ExpDec3、ExpGro1、ExpAssoc、Boltzmann、Logistic、DoseResp、Hill、Gompertz、Allometric1、Allometric2、Power1、Log3P1、Langmuir、Freundlich、PseudoFirstOrder、PseudoSecondOrder、MichaelisMenten、Rational1、Rational12、Rational22、Rational33。239 项参考目录逐项对应、更多函数族与数据驱动函数库仍待补 | 原 24 模型已有参考回归；四个有理模型通过 SciPy 参数/标准误/RSS 对照，Rational33 通过 NIST Thurber 专项；完整 NIST 参考值对照待做 | 显函数参考目录：已完成 0/239，待验收 0，部分实现 0；239 为路线参考基数；当前 28 个模型的逐项目录归属仍待确认，与 F148–F156 重叠，不直接折算为目录完成数。；当前内置模型：已完成 0/28，待验收 28，部分实现 0；当前 Model::ALL 的 28 个模型已有各自回归，均待完整参考/人工验收；不含线性与多项式阶数，也不含用户自定义表达式。 |

## 数学运算

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F158 | 插值 / 外推（线性、三次样条、Akima） | Interpolate/Extrapolate | P0 | 已实现待验收 | 分析 → 插值：线性、自然三次样条、Akima，等间距 X 网格、可自定义范围并按端点段外推；结果写成新 X/Y 列并加到图上，一步撤销 | 与 SciPy 1.17.1 interp1d / CubicSpline / Akima1DInterpolator 的区间内与外推值一致（1e-12） | 插值方法：已完成 0/3，待验收 3，部分实现 0；线性、自然三次样条、Akima；均匀输出网格入口已接入 |
| F159 | 按 X 求 Y 插值 | Interpolate/Extrapolate Y from X | P1 | 部分实现 | interpolate.rs 已可在指定 X 数组计算 Y；桌面插值对话框提供均匀网格及范围。按用户指定 X 列或单点查询的独立入口待做 | 线性/样条/Akima 参考值和外推回归通过；用户 X 列查询入口待实现 | — |
| F160 | 轨迹插值 | Trace Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F161 | XYZ 轨迹插值 | XYZ Trace Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F162 | 二维插值 | 2D Interpolate/Extrapolate | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F163 | 三维插值 | 3D Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F164 | 数值积分（梯形法、曲线下面积） | Integrate | P0 | 已完成 | analysis.rs/ops.rs 已实现梯形积分、累计积分结果与绝对面积选项 | 解析积分、行对齐与结果写回回归已通过；可复现 NumPy/SciPy 多组参考见 tests/reference 与 tools/reference/README.md（六阶容差原因单列），Windows 人工结果另记；2026-11 验收：自动测试 + 参考值 | — |
| F165 | 二维体积积分 | 2D Volume Integrate (Pro Only) | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F166 | 数值微分（含 Savitzky-Golay 平滑求导） | Differentiate | P0 | 已实现待验收 | 有限差分微分和独立 Savitzky–Golay 平滑；“SG 平滑求导”对话框可选一阶/二阶导数、窗口点数和多项式阶数，按实际 X 在窗口内做最小二乘多项式并对其求导，两端平移窗口，结果写成新列（单位按 Y/X 或 Y/X² 推导），一步撤销 | 与 SciPy 1.17.1 savgol_filter(deriv, delta, mode=interp) 三组参数逐点一致（相对误差 < 1e-9，含两端）；二次函数二阶导为常数；对话框真实点击与撤销/重做回归 | — |
| F167 | 多条曲线平均 / 合并 | Average/Merge Multiple Curves | P1 | 部分实现 | 已把多曲线线性插值到共同 X 范围求均值/标准差，输出结果表、Y 误差及线图；通用曲线合并工作流待做 | 平均结果、缺失值、结果输出和一步撤销回归通过；通用合并入口与参考软件验收待做 | — |
| F168 | 简单列运算（加减乘除、两曲线相减） | Simple Column/Curve Math | P0 | 已完成 | 分析 → 列运算：两列或列与常数的加减乘除，结果新列与单位推导；不同 X 网格的曲线先线性插值后相减，范围外留空；一步撤销 | 逐行、常数、单位、除零与“原始减拟合”残差回归；可复现 NumPy/SciPy 多组参考见 tests/reference 与 tools/reference/README.md（六阶容差原因单列），Windows 人工结果另记；2026-11 验收：自动测试 + 数值参考 | — |
| F169 | 归一化（最大值、Z 分数、区间缩放） | Normalize | P0 | 已完成 | 已接入除以最大值、Z 分数、区间缩放，结果写入派生列并支持撤销 | 算法参考值、写回及撤销回归已通过；可复现 NumPy/SciPy 多组参考见 tests/reference 与 tools/reference/README.md（六阶容差原因单列），Windows 人工结果另记；2026-11 验收：自动测试 + 参考值 | — |
| F170 | 多边形面积 | Polygon Area | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F171 | 反函数（交换 X、Y 求逆） | Inverse | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F172 | 自定义滤波 / 中值滤波 | Median Filter | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F173 | 扣除直线基线 | Subtract Straight Line | P1 | 已实现待验收 | 扣除基线对话框选“两端连线”：首末两点连线作为基线 | 单元测试；SciPy/NIST 参考值对照待做 | — |
| F509 | 列公式函数库 | Set Column Values / Functions | P0 | 已实现待验收 | 表达式引擎 121 个函数，分 8 类：数学、三角与双曲、特殊函数（Γ、B、erf/erfinv、不完全 B/Γ 等）、逻辑与比较（if/and/or/not/xor/between，另有比较运算符 < > <= >= == !=）、概率分布（正态/t/F/χ²/二项/泊松/指数的密度、分布函数与分位数）、列统计（sum mean median std var sem count range rms percentile skewness kurtosis，min/max 单参数为整列）、按行序列（cumsum cumprod diff rank zscore lag lead）、日期时间（年月日时分秒、星期、日序、date/time）；设置列值对话框“函数”菜单按类别插入 | 全部标量函数与 Python math / SciPy 1.17.1 / NumPy 2.4.4 参考值一致（相对 1e-9）；偏度、峰度、百分位数、标准差、标准误与 scipy.stats 一致；列统计与按行序列的整列计算及缺失值处理测试；目录名称唯一、≥120 个 | 列公式函数：已完成 0/120，待验收 120，部分实现 0；当前 121 个函数名，首批 120 个目标已达到，待人工与 Origin 对照 |

## 统计

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F174 | 列统计（均值、标准差、中位数、分位数等） | Statistics on Columns | P0 | 已完成 | 已接入 N、均值、样本标准差、标准误、中位数、线性插值分位数、极值与总和，生成结果工作簿 | 描述统计与结果表回归已通过；可复现 NumPy/SciPy 多组参考见 tests/reference 与 tools/reference/README.md（六阶容差原因单列），Windows 人工结果另记；2026-11 验收：自动测试 + 参考值 | — |
| F175 | 行统计 | Statistics on Rows | P1 | 已实现待验收 | 统计 → 行统计：对点选的各列逐行求均值、标准差、标准误、N，写在最后一列右边；标准差设为 Y 误差列，可直接画误差棒 | 单元测试覆盖缺失值、单值行与列属性 | — |
| F176 | 整表统计 | Statistics on Whole Sheet | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F177 | 频数统计 / 分箱 | Frequency Counts | P0 | 已完成 | 统计 → 频数统计：等宽分箱（Sturges 或指定区间数、可选范围），每列生成区间、频数、累积与相对频率表，一步撤销 | 与 numpy.histogram 的边界与计数一致；2026-11：可复现 SciPy 参考数据和直接测试通过。 | — |
| F178 | 二维频数统计 | 2D Frequency Counts/Binning | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F179 | 正态性检验（Shapiro-Wilk、KS、Anderson-Darling 等） | Normality Test | P1 | 部分实现 | 统计 → 正态性检验：Shapiro–Wilk（Royston AS R94，3 ≤ N ≤ 5000），每个点选列一行；KS、Anderson–Darling 待做 | 与 SciPy 1.17.1 shapiro 一致（W 1e-6，SciPy 内部为单精度；p 1e-4） | 列明的正态性方法：已完成 0/3，待验收 1，部分实现 0；Shapiro–Wilk 已接入；KS、Anderson–Darling 未开始；仅计本条列明方法 |
| F180 | 相关系数（Pearson、Spearman、Kendall） | Correlation Coefficient (Pro Only) | P1 | 已完成 | 统计 → 相关系数：点选列两两计算 Pearson、Spearman（平均秩）、Kendall τ-b（并列校正）及双侧 p 值 | 与 SciPy 1.17.1 pearsonr、spearmanr、kendalltau(method="asymptotic") 一致（1e-9），含并列值与负相关；2026-11：可复现 SciPy 参考数据和直接测试通过。 | — |
| F181 | 偏相关系数 | Partial Correlation Coefficient (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F182 | 分布拟合 | Distribution Fit (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F183 | 列联表与卡方检验 | Cross Tabulation and Chi-square (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F184 | Grubbs / Dixon Q 异常值检验 | Grubbs' Test | P1 | 部分实现 | 统计 → Grubbs 异常值：双侧检验离均值最远的点，给出 G、临界值 G(0.05) 与 p 值；Dixon Q 待做 | 临界值与 p 值与按 scipy.stats.t 计算的 Grubbs 公式一致（1e-9） | 列明的异常值方法：已完成 0/2，待验收 1，部分实现 0；Grubbs 已接入；Dixon Q 未开始 |
| F185 | 单样本 t 检验 | One-Sample T-Test | P0 | 已完成 | 统计 → t 检验（单样本）：自写正则化不完全 Beta 求 t 分布，报告 t、自由度、双侧 p、置信区间 | 与 SciPy 1.17.1 ttest_1samp 一致（1e-9）；2026-11：可复现 SciPy 参考数据和直接测试通过。 | — |
| F186 | 双样本 t 检验（含 Welch） | Two-Sample T-Test | P0 | 已完成 | 统计 → t 检验（双样本）：方差相等的 Student 检验与 Welch 检验，置信区间针对均值差 | 与 SciPy 1.17.1 ttest_ind（equal_var 真/假）一致（1e-9）；2026-11：可复现 SciPy 参考数据和直接测试通过。 | — |
| F187 | 配对 t 检验 | Pair-Sample T-Test | P0 | 已完成 | 统计 → t 检验（配对）：按行配对，只用两列都有数值的行 | 与 SciPy 1.17.1 ttest_rel 一致（1e-9）；2026-11：可复现 SciPy 参考数据和直接测试通过。 | — |
| F188 | 方差检验（单样本、F 检验） | Two Sample Test for Variances (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F189 | 比例检验、泊松率检验 | One-Sample Proportion Test (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F190 | 单因素 / 双因素 / 三因素方差分析（含多重比较） | One, Two, and Three Way ANOVA | P1 | 部分实现 | 统计 → 单因素方差分析：每列一组，ANOVA 表、各组描述统计与 Tukey HSD 多重比较（组大小不等时为 Tukey–Kramer）；双因素、三因素待做 | 与 SciPy 1.17.1 f_oneway、f.sf（1e-9）及 tukey_hsd（置信限 1e-7、p 值 1e-5）、studentized_range 一致 | 列明的 ANOVA 维数：已完成 0/3，待验收 1，部分实现 0；单因素及 Tukey/Tukey–Kramer 已接入；双因素、三因素未开始 |
| F191 | 重复测量方差分析 | Repeated Measure ANOVA (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F192 | 非参数检验（Wilcoxon、Mann-Whitney、Kruskal-Wallis、Friedman 等） | Nonparametric Tests (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F193 | 生存分析（Kaplan-Meier、Cox 回归、Weibull） | Survival Analysis (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F194 | 主成分分析 PCA | Principal Component Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F195 | 偏最小二乘 PLS | Partial Least Squares | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F196 | 聚类分析 | Cluster Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F197 | 判别分析 | Discriminant Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F198 | 机器学习（分类、回归） | Machine Learning (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F199 | 功效与样本量计算 | Power and Sample Size (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F200 | ROC 曲线 | ROC Curve (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F505 | 图形摘要报告 | Statistics / Graphical Summary | P1 | 未开始 | 描述统计与分布图组合报告 | 参考统计值与图形、报告往返 | — |
| F506 | 并排统计比较 | Statistics / Side-by-Side Statistics | P1 | 未开始 | 多组数据统计与并列结果 | SciPy/R 数值对照与缺失值处理 | — |
| F507 | 双样本 Kolmogorov–Smirnov 检验 | Statistics / Two-Sample KS Test | P1 | 未开始 | A8 双样本统计量与 p 值 | SciPy/R 对照、重复值和小样本 | — |
| F508 | 滑动窗口非参数检验 | Statistics / Running Nonparametric Tests | P1 | 未开始 | A8 窗口范围、方法选择和结果列 | 窗口边界、缺失值、SciPy/R 对照 | — |

## 信号处理

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F201 | 平滑（移动平均、Savitzky-Golay、FFT 平滑、LOWESS） | Smoothing | P0 | 已实现待验收 | 平滑对话框四种方法：移动平均、二阶 Savitzky–Golay、LOWESS（每点取最近的窗口点数个近邻，tricube 权重局部加权线性回归，X 可无序、不等间距）、FFT 滤波（去掉首尾连线趋势后低通，截止频率 1/(窗口点数·ΔX)，再加回趋势；X 不等间距时提示先插值）；结果写新列，来源是图形时加为曲线，一步撤销 | LOWESS 与 NumPy 逐点暴力实现（k=5/9，无序不等间距 X）一致、直线不变；FFT 平滑与 numpy rfft 掩模参考一致（误差 < 1e-9）；不等间距 X 拒绝 FFT 平滑；LOWESS 稳健迭代验收待做 | 平滑方法：已完成 0/4，待验收 2，部分实现 0；移动平均、Savitzky–Golay 已实现待验收；FFT、LOWESS 未开始 |
| F202 | FFT 滤波（低通、高通、带通、带阻） | FFT Filters | P1 | 已实现待验收 | 分析 → 信号处理 → FFT 滤波：低通、高通、带通、带阻理想滤波（频域置零后逆变换），结果写成新列，来源是图形时画进图里 | 与 numpy rfft / irfft 掩模滤波逐点一致（1e-12）；带通 + 带阻 = 原信号 | 频域滤波类型：已完成 0/4，待验收 4，部分实现 0；低通、高通、带通、带阻理想滤波；不代表 IIR 或其他滤波器实现 |
| F203 | IIR 滤波（巴特沃斯、切比雪夫等） | IIR Filters (Pro Only) | P2 | 未开始 | 自写或 biquad | 与 Origin 及 SciPy.signal 结果比较 | — |
| F204 | 傅里叶变换 FFT / IFFT（幅值、相位、功率谱、窗函数） | Fast Fourier Transform (FFT) | P0 | 已实现待验收 | 任意点数 FFT（基 2/Bluestein）、矩形/Hann/Hamming 窗、单边幅值/相位/功率谱及幅值图；独立 IFFT 命令：选中实部/虚部列或在 FFT 结果表直接运行，单边谱按实数信号逆变换（可选偶/奇输出点数，同 numpy.fft.irfft），双边谱做复数逆变换（同 numpy.fft.ifft，输出实部和虚部），时间间隔按 1/(N·df) 换算，生成信号表与曲线图 | FFT 与 numpy.fft 一致；irfft（8/9/7 点）与 ifft 与 NumPy 2.4.4 参考值误差 < 1e-12；任意长度往返还原；FFT→IFFT 项目往返、对话框真实点击与撤销/重做回归 | — |
| F205 | 短时傅里叶变换 STFT | Short-Time Fourier Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F206 | 卷积 / 反卷积 | Convolution | P1 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F207 | 相关（自相关、互相关） | Correlation | P1 | 已实现待验收 | 分析 → 信号处理 → 相关函数：选一列求自相关、两列求互相关（归一化，滞后以点计），结果表与线图 | 与 numpy.correlate 归一化结果一致（1e-12） | — |
| F208 | 相干性 | Coherence (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F209 | 希尔伯特变换、包络 | Hilbert Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F210 | 二维 FFT 与二维 FFT 滤波 | 2D Fourier Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F211 | 小波变换（连续、分解、重构、去噪） | Wavelet Transforms (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F212 | 二维小波 | 2D Wavelet Transforms | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F213 | 二维相关 | 2D Correlation (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F214 | 矩阵平滑 | Matrix Smoothing | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F215 | 抽取 / 降采样 | Decimation (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |

## 峰分析

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F216 | 基线：锚点法、自动基线（ALS、多项式等） | Peak Analyzer | P1 | 部分实现 | 分析 → 信号处理 → 扣除基线：两端连线与 ALS 自动基线（非对称最小二乘，五对角 LDLᵀ 求解）；锚点法、多项式基线待做 | ALS 与 numpy 稠密求解结果一致（1e-9） | — |
| F217 | 扣除基线 | Peak Analyzer | P1 | 已实现待验收 | 每条曲线在 Y 右边写入“基线”和“扣除基线”两列；来源是图形时基线画进图里，一步撤销 | 单元测试覆盖缺失行、列位置与图层曲线 | — |
| F218 | 寻峰（局部极值、导数法、阈值） | Peak Analyzer | P1 | 部分实现 | 分析 → 信号处理 → 寻峰：局部极大值（平顶取中点）+ 最小突出度阈值（占 Y 范围百分比），算法同 scipy.signal.find_peaks；导数法待做 | 单元测试与 SciPy 1.17.1 find_peaks / peak_prominences 逐值一致（含平顶峰） | — |
| F219 | 峰积分（面积、半峰宽、质心） | Peak Analyzer | P1 | 已实现待验收 | 峰表给出峰高、突出度、半峰宽（半突出度处，同 peak_widths）、线性基线以上的面积与质心；积分区间到相邻峰之间的最低点 | 单元测试与 SciPy peak_widths、numpy.trapezoid 一致；高斯峰面积与 A·s·√π 吻合 | — |
| F220 | 多峰拟合（重叠峰分峰） | Peak Analyzer | P1 | 未开始 | 复用拟合引擎 | 与 Origin Peak Analyzer 结果比较 | — |

## 图像处理

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F221 | 导入图片到矩阵 | Importing Images and Videos | P2 | 未开始 | image | 手动操作 | — |
| F222 | 图像调整（亮度、对比度、伽马） | Image Adjustment | P2 | 未开始 | image / imageproc | 与 Origin 比较 | — |
| F223 | 几何变换（旋转、裁剪、缩放） | Geometric Transformations | P2 | 未开始 | imageproc | 与 Origin 比较 | — |
| F224 | 空间滤波（边缘检测、锐化等） | Spatial Filters | P2 | 未开始 | imageproc | 与 Origin 比较 | — |
| F225 | 图像分析（粒子计数、剖面） | Image Analysis | P2 | 未开始 | imageproc | 与 Origin 比较 | — |

## 导出与发布

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F226 | 导出 SVG | Exporting Your Graph or Layout Page to an Image File | P0 | 已完成 | svg.rs | 目视比较 Origin 导出图 | — |
| F227 | 导出 PNG（指定 DPI） | The Export Graph as Image Dialog Box | P0 | 已实现待验收 | 图形 → 导出 PNG：SVG 版面经 resvg 栅格化，可选 150/300/600/1200 dpi 或自定义；写入 pHYs 物理分辨率，像素尺寸按页面毫米换算；中文回退到系统字体；字体集与屏幕/核心度量共用同一字节，SVG 使用自然字宽和明确基线，支持科学文字分段字号/上下标偏移/粗斜体 | 自动化回归核对像素尺寸、pHYs、可解码和绘制内容；插入 Word 的实际尺寸待人工验收；字体实际字节匹配及 96/300/600 DPI 标题栅格边界回归；科学文字真实字形、屏幕/SVG 基线对照与 96/300/600 DPI 边界回归 | — |
| F228 | 导出 PDF（矢量，嵌入字体） | Which Export Format Should I Choose | P0 | 已实现待验收 | 文件页、功能区、图形右键和命令搜索导出 PDF；复用页面 SVG 为矢量，保持毫米尺寸、裁剪、透明面积、双轴、多图层与科学文字；嵌入实际字体子集并保留 Unicode 文本。按实际使用字体的 fsType 拒绝受限/仅位图/禁止子集等权限，导出不改项目；自由文字和手动图例沿用相同页面比例位置 | 14 图型+科学文字/中文/误差棒/双轴+多面板/插图共 16 样例回归；独立 pypdf 检查字体流、Unicode、页面尺寸和无页面位图，Poppler 渲染与 PNG 对照并逐图查看；实际打印与 Windows 文件对话框人工验收待做；新增自由文字和手动图例的 PDF/PNG 独立渲染对照 | PDF 导出：已完成 0/4，待验收 4，部分实现 0；矢量/裁剪/透明度、毫米页面尺寸、字体子集/Unicode、科学文字排版均已接入并通过专项核对；最终人工验收待做 |
| F229 | 导出 EPS | Which Export Format Should I Choose | P1 | 未开始 | 自写 PostScript 输出 | LaTeX 编译检查 | — |
| F230 | 导出 TIFF（期刊要求） | Which Export Format Should I Choose | P1 | 未开始 | image | 尺寸与 DPI 检查 | — |
| F231 | 复制图形到剪贴板（粘贴到 Word / PPT） | Pasting or Embedding Graphs in Other Applications | P0 | 已实现待验收 | 复制图形（Ctrl+J）：按 300 dpi 栅格化后经 arboard 写入剪贴板位图；矢量 EMF 待评估 | 与 PNG 共用栅格化回归；粘贴到 Word / PPT 待人工验收 | — |
| F232 | 导出 EMF（Office 可编辑矢量） | Which Export Format Should I Choose | P1 | 未开始 | 自写 EMF 写出器 | 插入 Word 检查 | — |
| F233 | 批量导出多张图 | The Export Graph Dialog Box | P1 | 未开始 | — | 手动操作 | — |
| F234 | 打印 | Printing Graphs, Layout Pages, and Analytical Results | P2 | 未开始 | 导出 PDF 后交给系统打印 | 手动操作 | — |
| F235 | 图形动画（导出视频 / GIF） | Creating Graph Animation | P2 | 未开始 | gif / ffmpeg | 手动操作 | — |

## 自动化与编程

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F236 | 内置 Python 运行时（预装 numpy、scipy、pandas） | Python Console | P1 | 未开始 | Windows 用官方嵌入版，macOS / Linux 用 python-build-standalone，随安装包分发；脚本在独立进程中运行，崩溃不影响软件 | 在没装 Python 的干净系统上安装后运行全部示例脚本 | — |
| F237 | scicanvas Python 包（连接运行中的软件；离线模式直接调用核心库） | Python Packages | P1 | 未开始 | 本机控制通道（带令牌）+ Arrow 批量传数据；离线模式用 PyO3 编译核心库 | 脚本示例回归测试，与界面操作结果比较 | — |
| F238 | 脚本窗口（编辑器、运行 / 停止、输出面板、报错行高亮） | The Origin Command Window and Script Window | P1 | 未开始 | egui 代码编辑面板，调用内置 python.exe 子进程，实时显示输出 | 运行、中途停止、报错定位到行 | — |
| F239 | 操作录制（把界面操作生成 Python 脚本，可回放） | Batch Processing | P1 | 未开始 | 命令注册表记录命令序列，翻译成 scicanvas Python 调用 | 录制 → 回放，结果与手动操作一致 | — |
| F240 | Python 包管理（装包到内置环境，可切换到系统 Python / conda） | Python Packages | P1 | 未开始 | 调用内置 pip；设置里选择解释器，脚本窗口标题显示当前环境 | 安装、卸载、切换环境后运行示例 | — |
| F241 | 命令行批处理（无界面） | Batch Processing | P1 | 未开始 | scicanvas-cli，调用核心库 | 脚本示例回归测试 | — |
| F242 | 分析模板（换数据自动重算） | Analysis Templates | P1 | 未开始 | 分析操作记录为命令序列 | 与 Origin 比较 | — |
| F243 | MCP 服务器（让 AI 代理驱动软件） | — | P1 | 未开始 | 参照 PhotoCraft：命令注册表暴露为 MCP 工具 | 代理完成示例任务 | — |
| F244 | 插件系统 | Apps | P2 | 未开始 | WebAssembly 插件（wasmtime）沙盒运行 | 示例插件 | — |
| F245 | R 控制台 | R Console and Rserve Console | P2 | 未开始 | — | — | — |
| F246 | MATLAB 控制台 | MATLAB Console | 不做 | 不做 | — | — | — |
| F247 | LabTalk 脚本兼容 | LabTalk Programming（1,018 页） | 不做 | 不做 | 改用 Python | — | — |
| F248 | Origin C 兼容 | Origin C（3,862 页） | 不做 | 不做 | 改用 Python / Rust 插件 | — | — |
| F249 | X-Function 兼容（约 680 个） | X-Function Reference | 不做 | 不做 | 功能按需在核心库实现，不做接口兼容 | — | — |
| F250 | COM 自动化服务器 | Automation Server | 不做 | 不做 | — | — | — |
| F251 | LabVIEW 虚拟仪器 | Using Origin from LabVIEW | 不做 | 不做 | — | — | — |
| F252 | Mathematica Link | Mathematica Link | 不做 | 不做 | — | — | — |
| F253 | Code Builder 集成开发环境 | The Code Builder | 不做 | 不做 | — | — | — |

## 图类型 · 2D 线图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F254 | 折线图 | Line Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F255 | 水平阶梯图 | Horizontal Step Graph | P1 | 已完成 | 水平阶梯：先横后竖，共享折线图元与 SVG | 拐点几何、拟合线、保存与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F256 | 垂直阶梯图 | Vertical Step Graph | P1 | 已完成 | 垂直阶梯：先竖后横，共享折线图元与 SVG | 拐点几何、拟合线、保存与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F257 | Y 偏移堆叠线图 | Stacked Lines by Y Offsets Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F258 | 棒棒糖图 | Lollipop Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F259 | 颜色映射线系列 | Colormapped Line Series | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F260 | 样条曲线图 | Spline Graph | P1 | 已实现待验收 | 自然三次样条、缺失/重复 X 分段、自动范围与共享折线图元 | 解析值和 SciPy 1.17.1 自然边界对照、分段及 SVG 回归已通过；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |

## 图类型 · 2D 散点图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F261 | 散点图 | 2D Scatter Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F262 | 中心坐标散点图 | 2D Scatter Central Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F263 | 列散点图 | Column Scatter | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F264 | Y 误差棒图 | Y Error Bar Graph | P0 | 已完成 | Y 对称误差列自动关联、端帽/线宽、裁剪与共享线段图元 | 端点几何、无效值、保存与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F265 | XY 误差棒图 | XY Error Bar Graph | P0 | 已完成 | 对称 X/Y 误差列自动关联、方向转换、端帽/线宽与共享图元 | 关联、端点几何、裁剪及撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F266 | 散点 + 地毯图 | Scatter+Rug Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F267 | 垂线图 | Vertical Drop Line | P1 | 已完成 | 原始点到零基线的垂线，对数轴使用显示下界；符号/线宽与 SVG 共用 | 正负值、对数基线、SVG 线段和撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F268 | 气泡图 | Indexed Size Bubble Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F269 | 颜色映射散点图 | Color Mapped Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F270 | 气泡 + 颜色映射图 | Indexed Size (Bubble) and Color Map Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F271 | 误差带图 | Error Band Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F272 | 密度点图 | Density Dots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F273 | 彩色点图 | Color Dots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 2D 点线图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F274 | 点线图 | Line+Symbol Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F275 | 线系列图 | Line Series Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F276 | 两点线段图 | 2 Point Segment Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F277 | 三点线段图 | 3 Point Segment Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F278 | 样条连接点线图 | Spline Connected Graph | P1 | 已实现待验收 | 自然三次样条连接，符号与误差保持原始点；派生线独立绘制 | SciPy 数值、符号数、分段、自动范围及撤销回归已通过；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F279 | 按行绘图 | Row Wise Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F280 | 前后对比图 | Before and After Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 2D 柱状图与条形图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F281 | 柱状图 | Column Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F282 | 带标签柱状图 | Column + Label Graph | P1 | 已实现待验收 | 柱图配合数值/X-Y/行号/指定列数据标签，字号与偏移可调 | 标签行对齐、段值、边缘定位、SVG 与撤销回归已通过；matplotlib 同数据视觉对照待做 | — |
| F283 | 条形图 | Bar Graph | P0 | 已完成 | 数值型条形图、并排分组、误差换向、物理轴范围与共享矩形 | 轴方向、正负值、分组误差、SVG 与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F284 | 堆叠柱状图 | Stacked Column Graph | P1 | 已完成 | 按原始行与 X 对齐，正负柱段分别累加，共享矩形与 SVG | 缺失行、正负值、段几何、误差与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F285 | 浮动条形图 | Floating Bar Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F286 | 堆叠条形图 | Stacked Bar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F287 | 百分比堆叠柱状图 | 100% Stacked Column Graph | P1 | 已实现待验收 | 按行正负组分别归一至 ±100%，零总量与极端尺度处理，共享 SVG | 归一化参考值、极端数值、误差缩放及撤销回归已通过；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F288 | 百分比堆叠条形图 | 100% Stacked Bar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F289 | 浮动柱状图 | Floating Column Graph | P2 | 已完成 | 上下端列稳定配对、完整行过滤、反向区间、多组排列与共享 SVG | 配对、删列/复用、矩形、对数裁剪与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F290 | 百分比丝带图 | 100% Ribbon Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F291 | 丝带图 | Ribbon Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F292 | 圆堆积图 | Circular Packing Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F293 | 螺旋条形图 | Spiral Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F294 | 频数图 | Frequency Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 饼图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F295 | 2D 饼图 | 2D Color Pie Chart | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F296 | 3D 饼图 | 3D Color Pie Chart | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F297 | 环形图 | Doughnut Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F298 | 复合饼图 | Pie Of Pie Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F299 | 复合条饼图 | Bar Of Pie Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F300 | 复合环形图 | Doughnut Of Pie | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F301 | 百分比环形图 | Doughnut with Total Wedges = 100 | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F302 | 不等半径饼图 | Pie with Different Radius Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F303 | 不等半径环形图 | Doughnut with Different Radius Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F304 | 旭日图 | Sunburst Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 多轴图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F305 | 双 Y 轴图 | Double Y Graph | P0 | 已完成 | 曲线样式里勾选“右侧 Y 轴”：右轴曲线独立自动/手动定范围，可单独设刻度数字格式（缺省跟随左 Y）；存在右轴曲线时绘图属性显示右轴设置；右侧显示轴线、刻度、标签和标题，符号自动范围端点留 4%；对数开关仍左右共用；屏幕、SVG、PNG、PDF 和数据点吸附共用 AxisSet/YAxis，拟合/平滑结果线保留来源轴 | 左右轴范围互不影响、独立刻度格式、v14 默认外观迁移、右轴坐标框/标题/边距、数据拾取、保存重开和导出一致性回归；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F306 | 双 Y 轴柱状图 | Double-Y Column Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F307 | 双 Y 轴柱状 + 点线图 | Double-Y Column Line+Symbol Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F308 | 双 X 轴独立图 | Double X - Independent XY Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F309 | 双 X 轴公式换算图 | Double X - Custom Formula Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F310 | 共享 Y 的双 X 轴图 | Y-Shared Double X Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F311 | 阿伦尼乌斯双 X 轴图 | Double X - Arrhenius Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F312 | 2Ys Y-Y 图 | 2Ys Y-Y Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F313 | 2Ys 柱状图 | 2Ys Column | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F314 | 2Ys 柱状-点线图 | 2Ys Column-Line Symbol | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F315 | 3Ys Y-YY 图 | 3Ys Y-YY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F316 | 3Ys Y-Y-Y 图 | 3Ys Y-Y-Y | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F317 | 4Ys Y-YYY 图 | 4Ys Y-YYY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F318 | 4Ys YY-YY 图 | 4Ys YY-YY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F319 | 多 Y 轴图 | Multiple Y Axes Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F320 | 放大图 | Zoom Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 多面板图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F321 | 上下两面板图 | Vertical 2 Panel Graph | P0 | 已完成 | 绘图 → 多面板 → 上下两面板：Y 列按顺序分到两个图层，单列网格，页面 210 × 280 mm（A4 纸宽） | 核心库分配与网格排版回归；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | — |
| F322 | 左右两面板图 | Horizontal 2 Panel Graph | P0 | 已完成 | 绘图 → 多面板 → 左右两面板：Y 列按顺序分到两个图层，两列网格，页面 210 × 105 mm（A4 纸宽） | 核心库分配与网格排版回归；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | — |
| F323 | 四面板图 | 4 Panel Graph | P0 | 已完成 | 绘图 → 多面板 → 四面板：Y 列按顺序分到 2 × 2 网格的四个图层，页面 210 × 157.5 mm（A4 纸宽） | 核心库分配与网格排版回归；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | — |
| F324 | 九面板图 | 9 Panel Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F325 | 堆叠面板图 | Stack Panels Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F326 | 多面板图 | Multiple Panels Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F327 | 插图（图中图） | Inset Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F328 | 网格分面图 | Trellis | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F329 | 双 Y 网格分面图 | Trellis Double Y | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F330 | 簇图 | Cluster Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 瀑布图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F331 | 2D 瀑布图 | 2D Waterfall Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F332 | 2D 瀑布图（Y 颜色映射） | 2D Waterfall Y Color Mapping | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F333 | 2D 瀑布图（Z 颜色映射） | 2D Waterfall Z Color Mapping | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F334 | 3D 瀑布图 | 3D Waterfall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F335 | 3D 瀑布图（Y 颜色映射） | 3D Waterfall Y:Color Mapping | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F336 | 3D 瀑布图（Z 颜色映射） | 3D Waterfall Z:Color Mapping | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 统计图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F337 | 二维核密度图 | 2D Kernel Density Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F338 | 均值条形图 | Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F339 | 蜂群图 | Beeswarm Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F340 | Bland-Altman 图 | Bland-Altman Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F341 | 箱线图 | Box Chart Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F342 | 双 Y 箱线图 | 2Ys Box Chart Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F343 | 直方图 / 分布图 | Histogram/Distribution Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F344 | 概率直方图 | Histogram Probabilities Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F345 | 直方图 + 地毯图 | Histogram + Rug/Distribution + Rug Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F346 | 区间图 | Interval Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F347 | 多面板直方图 | Multi-Panel Histogram Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F348 | 平行索引图 | Parallel Index Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F349 | 平行坐标图 | Parallel Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F350 | 平行集图 | Parallel Sets Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F351 | 帕累托图（分箱数据） | Pareto Chart - Binned Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F352 | 帕累托图（原始数据） | Pareto Chart - Raw Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F353 | 人口金字塔图（龙卷风图） | Population Pyramid Graph(Tornado Chart) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F354 | 概率图 | Probability Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F355 | Q-Q 图 | Q-Q Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F356 | 质量控制图（X̄-R） | QC(x bar R) Chart Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F357 | 点图 | Dot Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F358 | 堆叠点图 | Stacked Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F359 | 散点矩阵图 | Scatter Matrix Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F360 | 堆叠直方图 | Stacked Histograms Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F361 | 多变异图 | Multi-Vari Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F362 | 变异性图 | Variability Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F363 | 小提琴图 | Violin Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F364 | 桑基图 | Sankey Diagrams | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F365 | 冲积图 | Alluvial Diagrams | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F366 | 网络图 | Network Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F367 | 和弦图 | Chord Diagram/Chord Diagram with Ratio Layout | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F368 | 层次边捆绑图 | Hierarchical Edge Bundling Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F369 | 边际箱线图 | Marginal Box Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F370 | 边际直方图 | Marginal Histograms Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F371 | 边际图 | Marginal Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F372 | 山脊图 | Ridgeline Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F373 | 对称图 | Symmetry Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F374 | 运行图 | Run Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F375 | 累积分布图 | CDF Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F376 | 交互作用图 | Interaction Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F377 | 主效应图 | Main Effects Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F378 | 因果图（鱼骨图） | Cause-and-Effect Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 分组图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F379 | 分组散点图 | Grouped Scatter | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F380 | 分组柱状图 | Grouped Columns | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F381 | 分组直方图 | Grouped Histogram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F382 | 分组箱线图（索引数据） | Grouped Box Charts-Indexed Data | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F383 | 分组箱线图（原始数据） | Grouped Box Charts-Raw Data | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F384 | 分组列散点图 | Grouped Column Scatter-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F385 | 分组区间图 | Grouped Interval Plot-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F386 | 分组均值条形图 | Grouped Mean Bar-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F387 | 分组点图 | Grouped Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F388 | 分组堆叠点图 | Grouped Stacked Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F389 | 分组小提琴图 | Grouped Violin-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F390 | 分组半小提琴图 | Grouped Half Violin-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F391 | 分组半箱线图 | Grouped Half Box-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F392 | 分组浮动条形图 | Grouped Floating Bar | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 地图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F393 | 条形地图 | Bar Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F394 | 饼图地图 | Pie Map Chart | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F395 | 方格地图 | Tile Grid Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F396 | 桑基地图 | Sankey Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F397 | Stiff 地图 | Stiff Map Diagram | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F398 | 矩形树图 | Treemap | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 面积图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F399 | 面积图 | Area Graph | P1 | 已完成 | 按连续有效行分段、跨零三角剖分、透明度、边界线与共享 SVG | 面积积分、缺口、对数裁剪、网格缩放与撤销回归已通过；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；2026-11 验收：自动测试 + 视觉对照 | — |
| F400 | 堆叠面积图 | Stacked Area Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F401 | 百分比堆叠面积图 | 100% Stacked Area Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F402 | 填充面积图 | Fill Area Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F403 | 双色填充图 | Fill Two Colors Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F404 | 风筝图 | Kite Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 专业图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F405 | 窗口浏览图 | Window-view Browser Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F406 | 子弹图 | Bullet Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F407 | 极坐标等高线 + 梯度矢量 | Polar Contour + Gradient Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F408 | 等高线 + 梯度矢量 | Contour Line + Gradient Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F409 | 等高线 + 流线 | Contour + Streamline | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F410 | Piper 三线图 | Piper/Trilinear Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F411 | Durov 图 | Durov Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F412 | Stiff 图 | Stiff Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F413 | 极坐标图 | Polar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F414 | 极坐标矢量图（θrθr） | Polar Vector theta r theta r graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F415 | 罗盘图 | Compass Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F416 | 极坐标矢量图（θrAM） | Polar Vector ?RAM graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F417 | 雷达图 | Radar Chart (Spider Plot) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F418 | 径向条形图 | Radial Bar and Radial Stacked Bar Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F419 | 直角三元图 | Right-Triangle Ternary | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F420 | 史密斯圆图 | Smith Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F421 | 堆叠径向图 | Stacked Radial Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F422 | 流线图 | Streamline Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F423 | 三元图 | Ternary | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F424 | 三元矢量图 | Ternary Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F425 | 风玫瑰图（分箱数据） | Wind Rose-Binned Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F426 | 风玫瑰图（原始数据） | Wind Rose-Raw Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F427 | 矢量图（XY + 角度 + 大小） | X, Y, Angle, Magnitude Vector Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F428 | 矢量图（起点 XY + 终点 XY） | X, Y, X, Y Vector Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 金融图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F429 | 桥图（瀑布式） | Bridge Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F430 | 高低收图 | High-Low-Close Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F431 | K 线图 | Japanese_Candlestick | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F432 | OHLC 条形图 | OHLC Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F433 | OHLC + 成交量图 | OHLC-Volume | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F434 | 股票折线图 | Line (Stock) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 3D XYY 图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F435 | XYY 3D 柱状图 | XYY 3D Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F436 | XYY 3D 百分比堆叠柱 | XYY 3D 100% Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F437 | XYY 3D 并列柱 | XYY 3D Side by Side Bars | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F438 | XYY 3D 堆叠柱 | XYY 3D Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F439 | XYY 3D 柱（带误差棒） | XYY 3D Bar Graph with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F440 | 3D 丝带图 | 3D Ribbons Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F441 | 3D 百分比堆叠墙图 | 3D 100% Stacked Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F442 | 3D 堆叠墙图 | 3D Stacked Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F443 | 3D 墙图 | 3D Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 3D 曲面图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F444 | 填色曲面图 | Color Fill Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F445 | X 恒定带底曲面 | X Constant with Base Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F446 | Y 恒定带底曲面 | Y Constant with Base Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F447 | 颜色映射曲面图 | Color Map Surface | P1 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F448 | 颜色映射曲面（带投影） | Color Map Surface with Projection | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F449 | 3D 三元颜色映射曲面 | 3D Ternary Colormap Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F450 | 线框图 | Wire Frame | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F451 | 线框曲面 | Wire Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F452 | 填色曲面（带误差棒） | Color Fill Surface with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F453 | 颜色映射曲面（带误差棒） | Colormap Surface with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F454 | 多个填色曲面 | Multiple Color Fill Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F455 | 多个颜色映射曲面 | Multiple Color Map Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F456 | 参数曲面 | Parametric Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F457 | 3D 堆叠曲面 | 3D Stacked Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F458 | 3D 堆叠热图 | 3D Stacked Heatmaps | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 3D 散点 / 柱 / 矢量图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F459 | 3D 散点图 | 3D Scatter Graph | P1 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F460 | 3D 散点（Z 误差棒） | 3D Scatter with Z Error Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F461 | 3D 线图 | 3D Line Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F462 | 轨迹图 | Trajectory Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F463 | 3D 柱状图 | 3D Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F464 | 3D 柱状图（分类 XY） | 3D Bar Graph with Categorical/Text XY | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F465 | 3D 堆叠柱状图 | 3D Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F466 | 3D 百分比堆叠柱状图 | 3D 100% Stacked Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F467 | 3D 浮动柱状图 | 3D Floating Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F468 | 3D 矢量（XYZ XYZ） | 3D Vector XYZ XYZ | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F469 | 3D 矢量（XYZ dXdYdZ） | 3D Vector XYZ dXdYdZ | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F470 | 3D 三元散点 | 3D Ternary Symbol | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F471 | 四面体图（四元相图） | 3D Tetrahedral(Quaternary Phase Diagram) | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 等高线与热图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F472 | 填色等高线图 | Color Fill Contour | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F473 | 黑白等高线图（带标签） | Black and White Lines with Labels Contour | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F474 | 灰度等高线图 | Gray Scale Map Contour | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F475 | 分类 Z 等高线图 | Contour - Categorical Z | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F476 | 热图 | Heatmap | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F477 | 极坐标热图 | Polar Heatmap | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F478 | 带标签热图 | Heatmap with Labels | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F479 | 分割热图 | Split Heatmap | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F480 | 分块热图 | Heatmap with Split Tiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F481 | 极坐标等高线（XYZ 数据） | Polar Contour from XYZ Data | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F482 | 极坐标等高线（矩阵数据） | Polar Contour from Matrix Data | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F483 | 三元等高线图 | Ternary Contour | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F484 | 叠加等高线图 | Overlaid Contour Plot | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 图像与剖面

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F485 | 等高线剖面图 | Contour Profiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F486 | 图片背景 | Image as Background | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F487 | 图像图 | Image Plot | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F488 | 图像剖面图 | Image Profiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |

## 分析工作流

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F489 | 分析报告表与重算（自动 / 手动 / 不重算） | Recalculating Analysis Results | P0 | 已实现待验收 | 既有逐列分析与线性/多项式重算保持。新建内置/自定义非线性拟合在结果 Y 列保存函数快照、初值、固定/上下界/线性约束、来源 X/Y 与行范围，列属性选择自动/手动/不重算；原位更新拟合 X/Y 与已有置信/预测带并供下游公式使用，来源未变跳过求解。失败保留整组旧结果，跨表各写各的来源表；源数据编辑与自动重算同一步撤销，手动重算单独一步。v19 保存定义并兼容 v0–v18，旧非线性快照保持。报告与 Note 仍为当次快照，统一联动报告表和函数库编辑后重拟合待做 | 自动/手动/不重算、无变化跳过、失败整组保留、依赖顺序、跨表隔离、行范围、删除解绑及标识复用、参数约束、置信带、v18 快照读取与 v19 往返回归；界面列属性切换及源编辑/手动重算一步撤销重做；Windows 人工操作和联动报告后续验收 | 非线性结果重算方式：已完成 0/3，待验收 3，部分实现 0；自动、手动、不重算已接入结果 Y 列，原 X/Y 与已添加的置信/预测带整组更新，人工验收待做。；统一可编辑联动报告表：已完成 0/1，待验收 0，部分实现 0；后续范围；现有报告窗口和 Markdown Note 保留当次快照，不随重算改写。 |
| F490 | 分析主题与对话框设置保存 | Analysis Themes | P1 | 未开始 | A5 保存分析参数主题，按操作恢复设置 | 跨会话恢复参数并验证默认值 | — |
| F491 | 分析模板批处理多个数据集 | Batch Processing | P1 | 未开始 | A5 模板输入映射、批量执行与错误报告 | 多文件运行、部分失败、结果关联 | — |

## 作图核心

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F492 | 参考线（水平 / 垂直） | Graphing / Axes / Reference Lines | P0 | 已实现待验收 | 绘图属性可添加/编辑/删除水平与垂直参考线，按 X、左 Y 或右 Y 轴值定位；支持颜色、线宽和实/虚线，参考线在当前图层数据之下绘制，并随轴范围和对数刻度重新映射；旧项目默认无参考线 | 线性/对数轴及右 Y 轴的位置映射、非正对数值、坐标范围变化、屏幕/SVG 共用图元、项目 v15 默认值/当前 v18 往返（参考线字段始于 v16）和添加/编辑/删除撤销重做；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | — |
| F493 | 坐标轴断点 | Graphing / Axes / Axis Break | P1 | 未开始 | A2 分段坐标映射、断点标记与拾取逆变换 | 断点两侧几何、导出与交互 | — |
| F494 | 附着坐标轴的对象 | Graphing / Axes / Objects | P1 | 未开始 | A7 对象锚点引用轴与数值位置 | 缩放后定位、保存和撤销 | — |
| F495 | 同一图层每条曲线独立图类型（柱 / 线 / 散点混合） | Plot Details / Plot Type | P0 | 已实现待验收 | 曲线样式里可为每条曲线单独选图型（折线、散点、点线、柱、面积、样条、样条点线、垂线、水平/垂直阶梯），同层混合绘制，柱子按改后的柱曲线重新分组；堆叠类与条形图按整层设置；一步撤销、随项目保存 | 散点与折线混合、柱子重新分组、项目往返的场景测试；SVG 与屏幕共用场景 | — |
| F496 | 曲线填充图案 | Plot Details / Pattern | P1 | 未开始 | A3 共用填充图元，区分实色与图案 | 屏幕与 SVG 图案一致、保存和撤销 | — |
| F497 | 符号边框与内部填充独立设置 | Plot Details / Symbol | P0 | 已实现待验收 | 绘图属性的曲线样式中，可独立选择跟随曲线颜色、空心或自定义内部填充；边框可单独启用并设置颜色/线宽，适用于已有符号图型。屏幕和 SVG/PNG/PDF 使用共享场景与符号几何，设置随 v17 及以后项目保存并可撤销重做 | 空心符号/边框共享场景与 SVG 回归、真实属性控件撤销重做、v16 默认值和 v17 JSON/SCV 往返通过；matplotlib 同数据视觉对照待做，Windows 人工验收待做 | 填充模式：已完成 0/3，待验收 3，部分实现 0；跟随曲线颜色、空心、自定义颜色已接入，均待人工视觉验收。；边框设置：已完成 0/2，待验收 2，部分实现 0；边框颜色、线宽独立设置；人工验收待做。 |
| F498 | 曲线连接方式（直线 / 样条 / B 样条 / 阶梯 / Bezier） | Plot Details / Line / Connect | P0 | 已完成 | 直线、自然样条（样条/样条点线图型）、水平/垂直阶梯（阶梯图型）；折线与点线图在属性窗口可选连接方式：B 样条（两端钳位均匀三次 B 样条，数据点为控制点，经过首尾点）、Bezier（每 4 点一段三次 Bezier，经过第 1、4、7… 点，余下点降阶）；拟合等结果曲线不受影响，未设置时项目文件不变 | B 样条与 scipy.interpolate.BSpline 同节点向量逐点一致（< 1e-12）；Bezier 端点、分段与二次尾段的解析值一致；场景折线点数回归；matplotlib 同数据视觉对照待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收）；已核对主要行为自动测试和视觉证据。 | 连接方式：已完成 0/6，待验收 6，部分实现 0；直线、自然样条、水平/垂直阶梯、B 样条、Bezier 均已接入，待人工与 Origin 对照 |
| F499 | 按列映射曲线或符号颜色 | Plot Details / Modifiers / Color | P0 | 已实现待验收 | 按曲线所属工作簿的数值/日期/分类列映射颜色，复用图层色板，当前 14 种基础图型渲染路径均已接入。面积图填充和曲线轮廓使用同一颜色：按 X 排序后每个点覆盖到相邻边的投影中点，保留透明度和缺失行断开；缺失颜色回退曲线颜色。图例显示代表样例与透明度，支持实时编辑、撤销、保存和删列清理 | 面积图排序/跨零面积守恒、缺失颜色与断行、外部分类列/均值/对数轴、常值图例、实际属性控件和一步撤销/重做、当前 v18 SCV 往返（映射字段始于 v17）回归通过；屏幕图元与 SVG/96及192 DPI PNG 颜色和透明混合回归通过；matplotlib 同数据视觉对照待做，Windows 人工验收待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | 当前基础图型颜色映射：已完成 0/14，待验收 14，部分实现 0；现有 14 个 PlotType 的颜色映射渲染路径均已接入待验收，含面积填充/轮廓。面积颜色按相邻点投影中点分段；图例为代表样例，连续渐变填充、自定义映射规则和完整分类键图例为后续扩展。 |
| F500 | 按列映射符号大小 | Plot Details / Modifiers / Size | P0 | 已实现待验收 | 按曲线所属工作簿的数值列线性映射符号大小，可设置最小/最大尺寸（1–20 页面逻辑单位，默认 2–12）；缺失/无效值沿用原符号大小，极端范围稳定归一化。适用于带符号图型，保存/导出/撤销及删列清理已接入 | 缺失值、正负 1e308 极端范围、逐点尺寸共享场景、实际下拉控件与撤销重做、v17 往返回归通过；matplotlib 同数据视觉对照待做，Windows 人工验收待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | 数值列大小映射：已完成 0/1，待验收 1，部分实现 0；数值列、尺寸范围及缺失值回退已接入待验收；当前按符号直径线性映射，面积比例与非线性尺寸映射为后续扩展。 |
| F501 | 按列映射符号形状 | Plot Details / Modifiers / Shape | P0 | 已实现待验收 | 按曲线所属工作簿的分类列顺序映射圆、方、上/下/左/右三角、菱形、六边形；超过 8 类循环使用符号，缺失类别回退曲线符号。图例显示代表样例，屏幕/导出共享几何，支持保存、删列清理和撤销重做 | 分类顺序取值/符号映射、共享场景/SVG、真实属性控件和撤销重做及项目往返回归通过；matplotlib 同数据视觉对照待做，Windows 人工验收待做；matplotlib 同数据位置/刻度对照见 evidence.visual（样式与人工交互另验收） | 可映射符号：已完成 0/8，待验收 8，部分实现 0；8 种现有符号按类别显示顺序固定循环；逐类任意指定符号、完整分类键图例尚未提供，当前图例仅为代表样例。 |
