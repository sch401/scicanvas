# matplotlib 同数据位置对照

本组比较数据位置与刻度映射，不比较默认页边距、字号、图例或线宽。47 个样例由测试中的 Project + Graph 生成；单层使用 Scene.plot，多层逐层使用 Scene.layer_plots。SciCanvas 页面坐标换算成框内 (fx, fy)，Y 轴朝上。JSON 记录原始列数据、类别顺序 / UTC 毫秒 / 时区、轴范围和图元。

```powershell
python -m venv .venv-ref
.venv-ref/Scripts/python -m pip install -r tools/reference/requirements-reference.txt
cargo test -p scicanvas-core --features serde --test visual_cases -- --ignored
.venv-ref/Scripts/python tools/reference/gen_matplotlib.py
cargo test --profile ci -p scicanvas-core --features serde --test visual_cases --test visual_compare
```

CI 仅读取提交的 JSON，并检查当前场景仍与已提交 JSON 一致。场景变化后，维护者显式重生成并复核，不自动更新基线。

数据来源：自建最小样例。line/scatter/linesym 的 X=0..4、Y=-2,3,缺失,5,-1 沿用 golden_svg 的负值/缺失行；scatter 另固定第 2–4 行选区。其他图型增加明确的第二列、误差、日期、类别或密集样条控制点，完整输入在各 scicanvas.json 中。

Matplotlib 使用同一数据和 SciCanvas 的轴范围，通过 transData / transAxes 计算相对位置。柱子只比较中心及上下端，堆叠按正负分别累计，百分比按每种符号归一到 ±100。误差棒仅比较端点，端帽设为 0。面积比较填充多边形的上下边界。自然样条在 SciCanvas 的同一组采样 X 上用 CubicSpline(bc_type=natural) 独立计算 Y；B 样条使用 SciPy BSpline 同节点，Bezier 使用 BPoly。日期用 UTC 毫秒转换 date2num，类别显式保留字典顺序。颜色用 Normalize 和现有五节点 Viridis 插值规则，比对 RGB 字节；大小/形状只比较位置，并排图展示映射。

位置绝对容差 1e-6，自然样条另加相对 Y 误差 1e-9。参考 JSON 记录 Python/NumPy/SciPy/matplotlib 版本；PNG 为 300 DPI 透明背景，字体 Arial → Microsoft YaHei。SciCanvas SVG 保留现有固定出版画布行为。

每个目录包含 scicanvas.json、scicanvas.svg、matplotlib.json、matplotlib.png 和并排说明。独立环境重生成的 JSON 应逐字节一致，PNG 不要求字节一致。科学文字排版、图例位置和字号按本轮指南留在本组数值验收范围之外。
