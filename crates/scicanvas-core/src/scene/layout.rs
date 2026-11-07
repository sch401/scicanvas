use super::*;

#[derive(Clone, Copy)]
struct PageGrid {
    grid_count: usize,
    cols: usize,
    ox: f64,
    oy: f64,
    gap_x: f64,
    gap_y: f64,
    cw: f64,
    ch: f64,
    paper: Rgb,
}

fn page_grid(g: &Graph, w: f64, h: f64) -> PageGrid {
    let [lw0, lh0] = g.page.logical_size();
    let [ox, oy, gw, gh] = g.page.content_rect();
    let (sx, sy) = (w / lw0, h / lh0);
    let (ox, oy) = (ox * sx, oy * sy);
    let (gw, gh) = (w - (lw0 - gw) * sx, h - (lh0 - gh) * sy);
    let paper: Rgb = g.page.background.map_or(PAPER, |[r, gg, b]| (r, gg, b));
    let grid_count = g
        .page
        .layers()
        .iter()
        .filter(|layer| layer.placement.is_none())
        .count()
        .max(1);
    let cols = g.page.columns.min(grid_count).max(1);
    let rows = grid_count.div_ceil(cols);
    let (gap_x, gap_y) = (
        (g.page.gap_mm[0] * 96.0 / 25.4 * sx).min(gw * 0.5 / cols.max(2) as f64),
        (g.page.gap_mm[1] * 96.0 / 25.4 * sy).min(gh * 0.5 / rows.max(2) as f64),
    );
    let (cw, ch) = (
        (gw - gap_x * (cols - 1) as f64) / cols as f64,
        (gh - gap_y * (rows - 1) as f64) / rows as f64,
    );
    PageGrid {
        grid_count,
        cols,
        ox,
        oy,
        gap_x,
        gap_y,
        cw,
        ch,
        paper,
    }
}

fn linked_views<'a>(
    g: &Graph,
    others: BookLookup<'a, '_>,
    views: &[Option<View>],
) -> (Vec<Option<View>>, Vec<crate::coord::AxisSet>) {
    let mut linked: Vec<Option<View>> = (0..g.page.layers().len())
        .map(|i| views.get(i).copied().flatten())
        .collect();
    let own: Vec<_> = if g.page.layers().iter().any(|l| l.link_x.is_some() || l.link_y.is_some()) {
        g.page
            .layers()
            .iter()
            .enumerate()
            .map(|(i, layer)| build_layer(layer, layer.book_id.and_then(others), others, 400., 300., linked[i]).axes)
            .collect()
    } else {
        Vec::new()
    };
    for (i, layer) in g.page.layers().iter().enumerate() {
        if layer.link_x.is_none() && layer.link_y.is_none() {
            continue;
        }
        let x = layer.link_x.and_then(|j| own.get(j)).unwrap_or(&own[i]);
        let y = layer.link_y.and_then(|j| own.get(j)).unwrap_or(&own[i]);
        linked[i] = Some(View {
            x: (x.x.lo, x.x.hi),
            y: (y.left_y.lo, y.left_y.hi),
        });
    }
    (linked, own)
}

fn aligned_margins<'a>(
    g: &Graph,
    others: BookLookup<'a, '_>,
    views: &[Option<View>],
    grid: &PageGrid,
    k: f64,
    linked: &[crate::coord::AxisSet],
    styles: &[ResolvedAxisStyle],
) -> Vec<[f64; 4]> {
    let mut cells = Vec::new();
    let mut aligned = vec![[0.0; 4]; g.page.layers().len()];
    if grid.grid_count > 1 {
        let mut natural = Vec::new();
        for (i, layer) in g.page.layers().iter().enumerate() {
            if layer.placement.is_none() {
                let (rw, rh) = (grid.cw / k, grid.ch / k);
                let sc = build_layer_full(LayerContext {
                    layer,
                    book: layer.book_id.and_then(others),
                    others,
                    w: rw,
                    h: rh,
                    view: views.get(i).copied().flatten(),
                    min: [0.; 4],
                    legend_outside: false,
                    style: styles[i],
                    linked_x: layer.link_x.and_then(|j| linked.get(j)).map(|a| &a.x),
                    linked_y: layer.link_y.and_then(|j| linked.get(j)).map(|a| &a.left_y),
                });
                let [left, top, pw, ph] = sc.plot;
                let index = cells.len();
                cells.push((i, index / grid.cols, index % grid.cols));
                natural.push([left, top, rw - left - pw, rh - top - ph]);
            }
        }
        for &(i, row, col) in &cells {
            let same = |f: &dyn Fn(usize, usize) -> bool, index: usize| {
                cells
                    .iter()
                    .zip(&natural)
                    .filter(|((_, r, c), _)| f(*r, *c))
                    .map(|(_, margin)| margin[index])
                    .fold(0.0, f64::max)
            };
            aligned[i] = [
                same(&|_, c| c == col, 0),
                same(&|r, _| r == row, 1),
                same(&|_, c| c == col, 2),
                same(&|r, _| r == row, 3),
            ];
        }
    }
    aligned
}

pub(super) fn compose<'a>(
    g: &Graph,
    book: impl Fn(u64) -> Option<&'a Book>,
    w: f64,
    h: f64,
    views: &[Option<View>],
) -> Scene {
    let grid = page_grid(g, w, h);
    let paper = grid.paper;
    let book = book;
    let others: BookLookup<'a, '_> = &book;
    let (linked, linked_axes) = linked_views(g, others, views);
    let views = &linked[..];
    let k = g.page.scale();
    let styles: Vec<_> = g
        .page
        .layers()
        .iter()
        .map(|layer| ResolvedAxisStyle::new(&layer.axis_style, k))
        .collect();
    let aligned = aligned_margins(g, others, views, &grid, k, &linked_axes, &styles);
    let mut grid_index = 0;
    let mut manual_legends = Vec::new();
    let scenes = g
        .page
        .layers()
        .iter()
        .enumerate()
        .map(|(i, layer)| {
            let [dx, dy, lw, lh] = match layer.placement {
                Some(rect) => [rect.x * w, rect.y * h, rect.width * w, rect.height * h],
                None => {
                    let index = grid_index;
                    grid_index += 1;
                    [
                        grid.ox + (index % grid.cols) as f64 * (grid.cw + grid.gap_x),
                        grid.oy + (index / grid.cols) as f64 * (grid.ch + grid.gap_y),
                        grid.cw,
                        grid.ch,
                    ]
                }
            };
            let mut scene = build_layer_full(LayerContext {
                layer,
                book: layer.book_id.and_then(others),
                others,
                w: lw / k,
                h: lh / k,
                view: views.get(i).copied().flatten(),
                min: aligned[i],
                legend_outside: false,
                style: styles[i],
                linked_x: layer.link_x.and_then(|j| linked_axes.get(j)).map(|a| &a.x),
                linked_y: layer.link_y.and_then(|j| linked_axes.get(j)).map(|a| &a.left_y),
            });
            for item in &mut scene.items {
                scale_prim(&mut item.prim, k);
                translate(&mut item.prim, dx, dy);
                if let Some(rect) = &mut item.clip {
                    *rect = rect.map(|value| value * k);
                    rect[0] += dx;
                    rect[1] += dy;
                }
            }
            scene.plot = scene.plot.map(|value| value * k);
            scene.plot[0] += dx;
            scene.plot[1] += dy;
            scene.layers[0].plot = scene.plot;
            scene.layers[0].bounds = [dx, dy, lw, lh];
            for object in &mut scene.objects {
                object.target = ObjectTarget::Legend(i);
                object.bounds = object.bounds.map(|v| v * k);
                object.bounds[0] += dx;
                object.bounds[1] += dy;
                if let Some(line) = &mut object.hit_segment {
                    for p in line {
                        p[0] = p[0] * k + dx;
                        p[1] = p[1] * k + dy;
                    }
                }
            }
            if let Some(position) = layer.legend_position {
                if let Some(mut object) = scene.objects.pop() {
                    // 手动定位只改变整体偏移，自动计算的绘图区矩形保持不变。
                    let x = (position[0] * w).clamp(0.0, (w - object.bounds[2]).max(0.0));
                    let y = (position[1] * h).clamp(0.0, (h - object.bounds[3]).max(0.0));
                    let mut items: Vec<_> = scene.items.drain(object.items.clone()).collect();
                    for item in &mut items {
                        translate(&mut item.prim, x - object.bounds[0], y - object.bounds[1]);
                        item.clip = Some([0.0, 0.0, w, h]);
                    }
                    object.bounds[0] = x;
                    object.bounds[1] = y;
                    object.overlay = true;
                    manual_legends.push((object, items));
                }
            }
            scene
        })
        .collect();
    let mut page = compose_page(g, scenes, manual_legends, w, h, paper);
    super::color_scale::append(&mut page, g, others, &styles);
    page
}

fn compose_page(
    g: &Graph,
    scenes: Vec<Scene>,
    manual_legends: Vec<(ObjectLayout, Vec<Item>)>,
    w: f64,
    h: f64,
    paper: Rgb,
) -> Scene {
    let mut scenes = scenes.into_iter();
    let mut page = scenes
        .next()
        .unwrap_or_else(|| build_layer(&Layer::default(), None, &|_| None, w, h, None));
    page.w = w;
    page.h = h;
    if g.page.layers().len() > 1
        || g.page
            .layers()
            .iter()
            .any(|layer| layer.transparent || layer.placement.is_some())
        || g.page.background.is_some()
        || g.page.margin_mm != [0.0; 4]
    {
        page.items.insert(
            0,
            Item {
                prim: Prim::Rect {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                    fill: Some((PAPER, 1.0)),
                    stroke: None,
                },
                clip: None,
            },
        );
        for object in &mut page.objects {
            object.items.start += 1;
            object.items.end += 1;
        }
    }
    page.layer_plots = vec![page.plot];
    for mut scene in scenes {
        page.layer_plots.push(scene.plot);
        page.layers.extend(scene.layers);
        let offset = page.items.len();
        for object in &mut scene.objects {
            object.items.start += offset;
            object.items.end += offset;
        }
        page.objects.extend(scene.objects);
        page.items.extend(scene.items);
    }
    for (mut object, items) in manual_legends {
        object.items = page.items.len()..page.items.len() + items.len();
        page.items.extend(items);
        page.objects.push(object);
    }
    if paper != PAPER {
        for item in &mut page.items {
            if let Prim::Rect {
                fill: Some((color, _)), ..
            } = &mut item.prim
            {
                if *color == PAPER {
                    *color = paper;
                }
            }
        }
    }
    super::shapes::append(&mut page, g);
    for annotation in &g.page.annotations {
        let size = annotation.size_pt * 96.0 / 72.0;
        let text =
            crate::text::fonts().wrap_formatted(&annotation.text, size, false, (w - 4.0).max(1.0), annotation.rich);
        let layout = crate::text::layout_formatted(&text, size, false, annotation.rich);
        let width = layout.width.max(8.0);
        let height = layout.height;
        let x = (annotation.position[0] * w).clamp(0.0, (w - width).max(0.0));
        let y = (annotation.position[1] * h).clamp(0.0, (h - height).max(0.0));
        let start = page.items.len();
        page.items.push(Item {
            prim: Prim::Text {
                x,
                y: y + height / 2.0,
                text,
                size,
                anchor: Anchor::Start,
                vertical: false,
                bold: false,
                rich: annotation.rich,
                color: (annotation.color[0], annotation.color[1], annotation.color[2]),
            },
            clip: Some([0.0, 0.0, w, h]),
        });
        page.objects.push(ObjectLayout {
            target: ObjectTarget::Text(annotation.id),
            bounds: [x, y, width, height],
            items: start..page.items.len(),
            overlay: true,
            hit_segment: None,
        });
    }
    page
}
