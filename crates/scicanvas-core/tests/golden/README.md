# SVG 回归基线

15 种图库预设（含直方图），以及 X/Y 误差棒、X/Y 数据标签、透明插图、双面板和注释图例，共 20 份 SVG。
数据包含负值、缺失行，组合案例包含特殊字符标题。所有案例调用屏幕与导出共用的
`svg::render_page` / `scene::layout_page`，按文件字节比较；正常测试只读基线。

执行 `cargo test -p scicanvas-core --test golden_svg`。需要有意调整版面时，运行：

```text
cargo test -p scicanvas-core --test golden_svg update_svg_baselines -- --ignored
```

更新后逐份审阅 SVG 差异，再提交基线。重构保持输出时不运行更新命令。
这些文件记录当前实现的回归基准；Origin 目视对照和真实字体排版仍按路线另行验收。
