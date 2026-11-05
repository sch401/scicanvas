use super::*;

pub(super) fn append(page: &mut Scene, g: &Graph, books: BookLookup<'_, '_>) {
    for (i, layer) in g.page.layers().iter().enumerate() {
        let Some((name, [lo, hi])) = crate::plotspec::color_scale_data(layer, books) else {
            continue;
        };
        let view = &page.layers[i];
        let plot = view.plot;
        let width = view.bounds[2] * 0.04;
        let scale = g.page.scale();
        let size = TICK_FONT * scale;
        let ticks: Vec<_> = axis(&[lo, hi], true, lo, hi, false, false)
            .ticks
            .into_iter()
            .filter(|v| *v >= lo && *v <= hi)
            .collect();
        let labels: Vec<_> = ticks.iter().map(|v| crate::fmt_num(*v)).collect();
        let label_width = labels
            .iter()
            .map(|s| crate::text::layout(s, size, false).width)
            .fold(0., f64::max);
        let label_height = labels
            .iter()
            .map(|s| crate::text::layout(s, size, false).height)
            .fold(0., f64::max);
        let title = crate::text::layout(&name, size, false);
        let block_width = title.width.max(width + 7. * scale + label_width);
        let manual = layer.color_scale.as_ref().and_then(|c| c.position);
        let original_right = view.bounds[0] + view.bounds[2] - plot[0] - plot[2] - (width + label_width + 22. * scale);
        let [x, y] = manual.map_or_else(
            || {
                let x = plot[0] + plot[2] + original_right.max(14. * scale);
                [
                    x.min(page.w - block_width).max(0.),
                    plot[1].max(title.height + 4. * scale),
                ]
            },
            |p| [p[0] * page.w, p[1] * page.h],
        );
        let bottom = if manual.is_none() {
            (plot[1] + plot[3]).min(page.h - label_height / 2.)
        } else {
            y + plot[3]
        };
        let height = (bottom - y).max(0.);
        let start = page.items.len();
        let mut push = |prim| page.items.push(Item { prim, clip: None });
        for step in 0..64 {
            let t = step as f64 / 64.;
            let overlap = if step < 63 { height / 128. } else { 0. };
            push(Prim::Rect {
                x,
                y: y + height * (1. - t - 1. / 64.) - overlap,
                w: width,
                h: height / 64. + overlap,
                fill: Some((layer.color_map.sample(if hi > lo { t + 0.5 / 64. } else { 0.5 }), 1.)),
                stroke: None,
            });
        }
        push(Prim::Rect {
            x,
            y,
            w: width,
            h: height,
            fill: None,
            stroke: Some((INK, 0.6 * scale)),
        });
        for (tick, label) in ticks.into_iter().zip(labels) {
            let fraction = if hi > lo { (tick - lo) / (hi - lo) } else { 0.5 };
            let yy = y + height * (1. - fraction);
            push(Prim::Segments {
                segs: vec![[x + width, yy, x + width + 4. * scale, yy]],
                color: INK,
                width: 0.6 * scale,
                dashed: false,
            });
            push(text(x + width + 7. * scale, yy, label, size, Anchor::Start));
        }
        push(text(x, y - 4. * scale - title.height / 2., name, size, Anchor::Start));
        let boxes = obstacles(&page.items[start..]);
        let outer = boxes.iter().map(Obstacle::bounds).fold(
            [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
            |a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
        );
        page.objects.push(ObjectLayout {
            target: ObjectTarget::ColorScale(i),
            bounds: [outer[0], outer[1], outer[2] - outer[0], outer[3] - outer[1]],
            items: start..page.items.len(),
            overlay: true,
            hit_segment: None,
        });
    }
}
