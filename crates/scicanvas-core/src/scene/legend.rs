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
                let mapped = p.plot.style.as_ref().and_then(|style| {
                    style
                        .color_column
                        .as_ref()
                        .or(style.size_column.as_ref())
                        .or(style.symbol_column.as_ref())
                });
                let name = mapped.map_or_else(
                    || p.series.name.clone(),
                    |column| format!("{} · {}", p.series.name, column),
                );
                crate::text::fonts().wrap_formatted(&name, LEGEND_FONT, false, (pw - sw - 16.0).max(20.0), g.rich_text)
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
        for (i, plot) in plots.iter().enumerate() {
            let row_height = layouts[i].height + 4.0;
            let cy = row_y + row_height / 2.0;
            row_y += row_height;
            let x0 = lx + 4.0;
            for prim in curve_sample(plot, g.area_opacity, x0, cy, sw) {
                push(prim);
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
            hit_segment: None,
        });
    }

    None
}

fn curve_sample(plot: &crate::page::PreparedPlot, area_opacity: f32, x: f64, y: f64, width: f64) -> Vec<Prim> {
    let mut items = fill_or_line_sample(plot, area_opacity, x, y, width);
    if let Some(pattern) = plot.plot.style.as_ref().and_then(|s| s.pattern.as_ref()) {
        let mut patches: Vec<_> = items.into_iter().map(|prim| Item { prim, clip: None }).collect();
        super::hatch::overlay(&mut patches, 0, pattern);
        items = patches.into_iter().map(|item| item.prim).collect();
    }
    if let Some(symbol) = symbol_sample(plot, x, y, width) {
        items.push(symbol);
    }
    items
}

fn fill_or_line_sample(plot: &crate::page::PreparedPlot, area_opacity: f32, x: f64, y: f64, width: f64) -> Vec<Prim> {
    let style = &plot.style;
    let color = style.color.rgb();
    let has_mapped_color = plot.series.point_styles.iter().any(|point| point.color.is_some());
    let mapped: Vec<_> = if has_mapped_color {
        plot.series
            .point_styles
            .iter()
            .map(|point| point.color.map_or(color, |[r, g, b]| (r, g, b)))
            .collect()
    } else {
        Vec::new()
    };
    let color = mapped.first().copied().unwrap_or(color);
    let varying = mapped
        .first()
        .is_some_and(|first| mapped.iter().any(|value| value != first));
    if style.kind.is_bar() || matches!(style.kind, PlotType::Area | PlotType::Box) {
        if varying {
            let count = mapped.len().min(3);
            return (0..count)
                .map(|index| {
                    let source = [0, mapped.len() / 2, mapped.len() - 1][index];
                    Prim::Rect {
                        x: x + 4.0 + (width - 8.0) * index as f64 / count as f64,
                        y: y - 3.5,
                        w: (width - 8.0) / count as f64,
                        h: 7.0,
                        fill: Some((
                            mapped[source],
                            if style.kind == PlotType::Area {
                                f64::from(area_opacity)
                            } else {
                                1.0
                            },
                        )),
                        stroke: Some(if style.kind == PlotType::Area {
                            (mapped[source], f64::from(style.line_width))
                        } else {
                            (INK, 0.5)
                        }),
                    }
                })
                .collect();
        }
        return vec![Prim::Rect {
            x: x + 4.0,
            y: y - 3.5,
            w: width - 8.0,
            h: 7.0,
            fill: Some((
                color,
                if style.kind == PlotType::Area {
                    f64::from(area_opacity)
                } else {
                    1.0
                },
            )),
            stroke: Some(if style.kind == PlotType::Area && has_mapped_color {
                (color, f64::from(style.line_width))
            } else {
                (INK, 0.5)
            }),
        }];
    }
    if !style.kind.has_line() {
        return Vec::new();
    }
    if varying {
        let count = mapped.len().min(3);
        let segments = (0..count)
            .map(|index| {
                let source = [0, mapped.len() / 2, mapped.len() - 1][index];
                let left = x + width * index as f64 / count as f64;
                let right = x + width * (index + 1) as f64 / count as f64;
                ColoredSegment {
                    points: [left, y, right, y],
                    color: mapped[source],
                }
            })
            .collect();
        vec![Prim::ColoredSegments {
            segments,
            width: f64::from(style.line_width),
        }]
    } else {
        vec![Prim::Polyline {
            pts: vec![[x, y], [x + width, y]],
            color,
            width: f64::from(style.line_width),
        }]
    }
}

fn symbol_sample(plot: &crate::page::PreparedPlot, x: f64, y: f64, width: f64) -> Option<Prim> {
    let style = &plot.style;
    if !style.kind.has_symbol() {
        return None;
    }
    let curve_style = plot.plot.style.as_ref();
    let mapped = plot
        .series
        .point_styles
        .iter()
        .any(|point| point.color.is_some() || point.size.is_some() || point.symbol.is_some());
    let custom = curve_style.is_some_and(|style| {
        style.symbol_fill.is_some() || style.symbol_border_color.is_some() || style.symbol_border_width.is_some()
    });
    if !mapped && !custom {
        return Some(Prim::Symbols {
            sym: style.symbol,
            pts: vec![[x + width / 2.0, y]],
            r: f64::from(style.symbol_size) / 2.0,
            color: style.color.rgb(),
        });
    }
    let count = plot.series.point_styles.len();
    let sample = if count == 0 {
        vec![0]
    } else {
        let mut indices = vec![0, count / 2, count - 1];
        indices.dedup();
        indices
    };
    let fill_style = curve_style
        .and_then(|style| style.symbol_fill)
        .unwrap_or(crate::page::SymbolFill::MatchCurve);
    let border_color = curve_style.and_then(|style| style.symbol_border_color);
    let border_width = curve_style.and_then(|style| style.symbol_border_width);
    let symbols = sample
        .iter()
        .enumerate()
        .map(|(index, source)| {
            let point = plot.series.point_styles.get(*source).copied().unwrap_or_default();
            let fill = match fill_style {
                crate::page::SymbolFill::MatchCurve => {
                    Some(point.color.map_or(style.color.rgb(), |[r, g, b]| (r, g, b)))
                }
                crate::page::SymbolFill::Hollow => None,
                crate::page::SymbolFill::Color([r, g, b]) => Some((r, g, b)),
            };
            let stroke = if border_color.is_some() || border_width.is_some() {
                Some((
                    border_color.map_or((0, 0, 0), |[r, g, b]| (r, g, b)),
                    f64::from(border_width.unwrap_or(0.6)),
                ))
            } else {
                None
            };
            StyledSymbol {
                at: [x + width * (index as f64 + 0.5) / sample.len() as f64, y],
                symbol: point.symbol.unwrap_or(style.symbol),
                radius: f64::from(point.size.unwrap_or(style.symbol_size)) / 2.0,
                fill,
                stroke,
            }
        })
        .collect();
    Some(Prim::StyledSymbols { symbols })
}

pub(super) fn draw_row_legend(
    entries: &[(String, Rgb)],
    at: [f64; 2],
    items: &mut Vec<Item>,
    objects: &mut Vec<ObjectLayout>,
) {
    let start = items.len();
    let width = entries
        .iter()
        .map(|(s, _)| text_width(s, LEGEND_FONT))
        .fold(0., f64::max)
        + 24.;
    for (i, (name, color)) in entries.iter().enumerate() {
        let y = at[1] + i as f64 * 16.;
        items.push(Item {
            prim: Prim::Rect {
                x: at[0],
                y: y - 4.,
                w: 12.,
                h: 8.,
                fill: Some((*color, 1.)),
                stroke: None,
            },
            clip: None,
        });
        items.push(Item {
            prim: text(at[0] + 18., y, name.clone(), LEGEND_FONT, Anchor::Start),
            clip: None,
        });
    }
    objects.push(ObjectLayout {
        target: ObjectTarget::Legend(0),
        bounds: [at[0], at[1] - 8., width, entries.len() as f64 * 16.],
        items: start..items.len(),
        overlay: false,
        hit_segment: None,
    });
}
