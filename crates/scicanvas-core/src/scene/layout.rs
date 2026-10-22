use super::*;

pub(super) fn compose<'a>(
    g: &Graph,
    book: impl Fn(u64) -> Option<&'a Book>,
    w: f64,
    h: f64,
    views: &[Option<View>],
) -> Scene {
    let others: BookLookup<'a, '_> = &book;
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
    let mut linked: Vec<Option<View>> = (0..g.page.layers().len())
        .map(|i| views.get(i).copied().flatten())
        .collect();
    if g.page
        .layers()
        .iter()
        .any(|layer| layer.link_x.is_some() || layer.link_y.is_some())
    {
        let own: Vec<View> = g
            .page
            .layers()
            .iter()
            .enumerate()
            .map(|(i, layer)| {
                build_layer(layer, layer.book_id.and_then(&book), others, 400.0, 300.0, linked[i]).layers[0].view()
            })
            .collect();
        for (i, layer) in g.page.layers().iter().enumerate() {
            if layer.link_x.is_none() && layer.link_y.is_none() {
                continue;
            }
            let pick = |link: Option<usize>, axis: fn(&View) -> (f64, f64)| {
                link.and_then(|j| own.get(j)).map_or(axis(&own[i]), axis)
            };
            linked[i] = Some(View {
                x: pick(layer.link_x, |view| view.x),
                y: pick(layer.link_y, |view| view.y),
            });
        }
    }
    let views = &linked[..];
    let k = g.page.scale();
    let mut cells = Vec::new();
    let mut aligned = vec![[0.0; 4]; g.page.layers().len()];
    if grid_count > 1 {
        let mut natural = Vec::new();
        for (i, layer) in g.page.layers().iter().enumerate() {
            if layer.placement.is_none() {
                let (rw, rh) = (cw / k, ch / k);
                let sc = build_layer(
                    layer,
                    layer.book_id.and_then(&book),
                    others,
                    rw,
                    rh,
                    views.get(i).copied().flatten(),
                );
                let [left, top, pw, ph] = sc.plot;
                let index = cells.len();
                cells.push((i, index / cols, index % cols));
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
    let mut grid_index = 0;
    let mut scenes = g.page.layers().iter().enumerate().map(|(i, layer)| {
        let [dx, dy, lw, lh] = match layer.placement {
            Some(rect) => [rect.x * w, rect.y * h, rect.width * w, rect.height * h],
            None => {
                let index = grid_index;
                grid_index += 1;
                [
                    ox + (index % cols) as f64 * (cw + gap_x),
                    oy + (index / cols) as f64 * (ch + gap_y),
                    cw,
                    ch,
                ]
            }
        };
        let mut scene = build_layer_min(
            layer,
            layer.book_id.and_then(&book),
            others,
            lw / k,
            lh / k,
            views.get(i).copied().flatten(),
            aligned[i],
        );
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
        scene
    });
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
    }
    page.layer_plots = vec![page.plot];
    for scene in scenes {
        page.layer_plots.push(scene.plot);
        page.layers.extend(scene.layers);
        page.items.extend(scene.items);
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
    page
}
