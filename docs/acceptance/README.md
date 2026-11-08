# Windows 人工验收清单

**验收程序**：[main 的 Windows CI 构建](https://github.com/sch401/scicanvas/actions/runs/42410257779)，提交 `bbbb400cca07280f7e1c66fa9d0a6b5d7a3ce0a3`，产物 `SciCanvas-Windows-x64`，到期时间 **2026-11-21 20:22 UTC**。解压后运行其中的 SciCanvas.exe。此构建包含 #101、#102；v0.1.0 安装包不用于本次验收。 从每项指定的数据或示例开始，把修改后的项目和导出文件放在临时验收目录。

每个条目的表格由验收人记录结果，并在条目末尾填写日期、提交号和结论。未实际执行的步骤留空。导出和剪贴板部分需要电子表格、文字处理和演示软件，恢复测试需单独的验收实例。

| 顺序 | 清单 | 条目数 | 预计时间 |
|---|---|---:|---:|
| 1 | [工作表](worksheet.md) | 11 | 45–80 分钟 |
| 2 | [拟合和报告](fitting.md) | 4 | 20–30 分钟 |
| 3 | [工作区和项目](workspace.md) | 5 | 25–35 分钟 |
| 4 | [导入导出](import_export.md) | 10 | 55–85 分钟 |
| 5 | [绘图](plotting.md) | 15 | 60–85 分钟 |

第一批的 68 项及用户已有记录继续保留；本页原有五份清单的时间估计为 205–315 分钟，可分次执行。F006 需等待至少 5 分钟自动备份。所有条目统一使用页首构建。


## 第二批（待做）

本批共 25 个功能条目（21 项模板重写、单列 F153，加 F094、F275、F515），另含 U0–U8。第一批用户已有的 68 项验收记录沿用。下面每组可独立安排，功能和界面均在页首同一个构建上操作。

先按 [数据说明](data/README.md) 导入 CSV，首行是列名；空白保持为空。A=X/B=Y 的表需要确认列类型；多组数据按各条目指定设置。Ctrl+K 搜索命令名执行。已提交 CSV 和参考值，验收机器运行程序即可；只有重新生成参考数据时才需要 Python、NumPy 和 SciPy。每条结果仍为待做，数值容差写在对应清单。

| 组 | 数据或示例 | 验收项与清单 | 预计时间 | 需要的软件 |
|---|---|---|---|---|
| 1 信号与插值 | convolution、deconvolution、interpolation、curve_merge.csv | [分析](analysis.md)：F206、F159、F167 | 60–90 分钟 | SciCanvas；文本编辑器用于查看 CSV |
| 2 基线与寻峰 | baseline_peaks、baseline_polynomial、peak_detection.csv | [分析](analysis.md)：F216、F218 | 45–60 分钟 | SciCanvas |
| 3 统计 | statistics.csv | [分析](analysis.md)：F176、F506、F505 | 45–60 分钟 | SciCanvas |
| 4 全局抑制与吸附 | inhibition_*.csv、binding_*.csv | [拟合](fitting.md)：F154、F155 | 60–90 分钟 | SciCanvas |
| 5 波形与多峰 | wave_*.csv、multipeak、overlapping_peaks.csv | [拟合](fitting.md)：F153、F140、F220 | 60–90 分钟 | SciCanvas |
| 6 映射与特殊图 | 25、30、31、32、33 号 .scv 示例 | [绘图](plotting.md)：F101、F269、F295、F363、F413 | 60–90 分钟 | SciCanvas；SVG 浏览器 |
| 7 格式、主题与坐标 | 01、29 号示例；axis_*.csv | [绘图](plotting.md)：F085、F090、F091、F114、F094、F275、F515 | 60–90 分钟 | SciCanvas；SVG 浏览器 |
| 8 功能区与参数窗口 | 01、25 号示例；上列分析 CSV | [界面](ui.md)：U0–U4（浅色/深色），各行“打开方式”照做 | 60–90 分钟 | SciCanvas |
| 9 菜单、键盘与浮动面板 | 01 号示例及 U8 表内操作 | [界面](ui.md)：U5–U8，含 #101 面板尺寸/位置、#102 图库示例按钮/折叠功能区 | 60–90 分钟 | SciCanvas；双显示器可补测跨屏放置 |

合计约 8.5–12.5 小时，可分九次完成。U5 并入界面表中的 U1（含 U5）行。遇到问题记录条目号、步骤、实际值和截图，再继续下一独立项。修改后的项目、主题和导出文件都保存到个人临时验收目录。
