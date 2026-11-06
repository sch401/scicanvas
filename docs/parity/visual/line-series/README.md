# line-series

覆盖：F275。自建原始数据记录于 scicanvas.json，参考由 NumPy 偏移规则和 matplotlib plot/scatter 生成。

| SciCanvas | matplotlib |
|---|---|
| ![SciCanvas](scicanvas.svg) | ![matplotlib](matplotlib.png) |

数据位置按 1e-6 对照。偏移、原值拾取和实时行连接另有核心回归。

至少两列，不生成隐藏工作表；按同一原始行跨列连接。

每个原始行使用不同颜色；符号统一，图例按行显示。颜色和图例不改变数据点、连接线位置，位置数据与改动前逐项相同；matplotlib.json 另补充按行配色的颜色数组。
