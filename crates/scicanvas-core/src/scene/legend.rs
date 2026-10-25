//! 逐曲线图例排版和重叠避让。
use super::*;

pub(super) fn draw_layer_legend(
    ctx: &LayerContext<'_, '_, '_>,
    data: &LayerData,
    layout: &AxisLayout,
    items: &mut Vec<Item>,
    objects: &mut Vec<ObjectLayout>,
    data_start: usize,
    data_end: usize,
) -> Option<Scene> {
    let g = ctx.layer;
    let book = ctx.book;
    let others = ctx.others;
    let w = ctx.w;
    let h = ctx.h;
    let min = ctx.min;
    let view = ctx.view;
    let legend_outside = ctx.legend_outside;
    let plots = &data.plots;
    let [left, top, pw, ph] = layout.rect;
    // 图例：坐标框内，白底无边框
    if g.legend && !plots.is_empty() {
        let sw = 18.0;
        let names: Vec<_> = plots
            .iter()
            .map(|p| {
                crate::text::fonts().wrap_formatted(
                    &p.series.name,
                    LEGEND_FONT,
                    false,
                    (pw - sw - 16.0).max(20.0),
                    g.rich_text,
                )
            })
            .collect();
        let layouts: Vec<_> = names
            .iter()
            .map(|name| crate::text::layout_formatted(name, LEGEND_FONT, false, g.rich_text))
            .collect();
        let lw = layouts.iter().map(|l| l.width).fold(0.0, f64::max) + sw + 16.0;
        let lhh = layouts.iter().map(|l| l.height + 4.0).sum::<f64>() + 4.0;
        // 自动放在四个角里数据最少的一个，尽量不遮挡曲线
        let pad_x = MAJOR_TICK + 6.0;
        let pad_y = MAJOR_TICK + 4.0;
        let corners = [
            (left + pad_x, top + pad_y),
            (left + pw - pad_x - lw, top + pad_y),
            (left + pw - pad_x - lw, top + ph - pad_y - lhh),
            (left + pad_x, top + ph - pad_y - lhh),
        ];
        // 按本图层实际画出的几何（柱、线、面积、误差棒、符号、标签）给四个角打分
        let obstacles = obstacles(&items[data_start..data_end]);
        let score = |(x, y): (f64, f64)| coverage(&obstacles, [x - 4.0, y - 4.0, lw + 8.0, lhh + 8.0]);
        let (mut lx, mut ly) = corners.iter().copied().min_by_key(|c| score(*c)).unwrap_or(corners[0]);
        // 四个角都会压住数据（如百分比堆叠柱占满整个高度）：重排，图例放到坐标框右侧
        let gap = 10.0;
        if legend_outside {
            (lx, ly) = (left + pw + gap, top);
        } else if score((lx, ly)) > 0 && lw + gap + 14.0 < w * 0.35 {
            let mut wider = min;
            wider[2] = wider[2].max(lw + gap + 14.0);
            return Some(build_layer_full(LayerContext {
                layer: g,
                book,
                others,
                w,
                h,
                view,
                min: wider,
                legend_outside: true,
            }));
        }
        // 上面借用过 items，图例图元（都不裁剪）改用新的 push 推入
        let start = items.len();
        let mut push = |prim: Prim| items.push(Item { prim, clip: None });
        push(Prim::Rect {
            x: lx,
            y: ly,
            w: lw,
            h: lhh,
            fill: Some((PAPER, 1.0)),
            stroke: None,
        });
        let mut row_y = ly + 2.0;
        for (i, st) in plots.iter().map(|p| &p.style).enumerate() {
            let color = st.color.rgb();
            let row_height = layouts[i].height + 4.0;
            let cy = row_y + row_height / 2.0;
            row_y += row_height;
            let x0 = lx + 4.0;
            if st.kind.is_bar() || st.kind == PlotType::Area {
                push(Prim::Rect {
                    x: x0 + 4.0,
                    y: cy - 3.5,
                    w: sw - 8.0,
                    h: 7.0,
                    fill: Some((
                        color,
                        if st.kind == PlotType::Area {
                            f64::from(g.area_opacity)
                        } else {
                            1.0
                        },
                    )),
                    stroke: Some((INK, 0.5)),
                });
            } else {
                if st.kind.has_line() {
                    push(Prim::Polyline {
                        pts: vec![[x0, cy], [x0 + sw, cy]],
                        color,
                        width: f64::from(st.line_width),
                    });
                }
                if st.kind.has_symbol() {
                    push(Prim::Symbols {
                        sym: st.symbol,
                        pts: vec![[x0 + sw / 2.0, cy]],
                        r: f64::from(st.symbol_size) / 2.0,
                        color,
                    });
                }
            }
            push(formatted_text(
                x0 + sw + 4.0,
                cy,
                names[i].clone(),
                LEGEND_FONT,
                Anchor::Start,
                g.rich_text,
            ));
        }
        objects.push(ObjectLayout {
            target: ObjectTarget::Legend(0),
            bounds: [lx, ly, lw, lhh],
            items: start..items.len(),
            overlay: false,
        });
    }

    None
}
