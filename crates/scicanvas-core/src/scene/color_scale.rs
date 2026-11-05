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
        let manual = layer.color_scale.as_ref().and_then(|c| c.position);
        let labels = [lo, hi]
            .iter()
            .map(|v| crate::text::layout(&crate::fmt_num(*v), TICK_FONT * scale, false).width)
            .fold(0.0, f64::max);
        let original_right = view.bounds[0] + view.bounds[2] - plot[0] - plot[2] - (width + labels + 22.0 * scale);
        let [x, y] = manual.map_or([plot[0] + plot[2] + original_right.max(14.0 * scale), plot[1]], |p| {
            [p[0] * page.w, p[1] * page.h]
        });
        let height = plot[3];
        let start = page.items.len();
        let mut push = |prim| page.items.push(Item { prim, clip: None });
        for step in 0..64 {
            let t = step as f64 / 64.0;
            push(Prim::Rect {
                x,
                y: y + height * (1.0 - t - 1.0 / 64.0),
                w: width,
                h: height / 64.0,
                fill: Some((layer.color_map.sample(if hi > lo { t + 0.5 / 64.0 } else { 0.5 }), 1.0)),
                stroke: None,
            });
        }
        push(Prim::Rect {
            x,
            y,
            w: width,
            h: height,
            fill: None,
            stroke: Some((INK, 0.6)),
        });
        let axis = axis(&[lo, hi], true, lo, hi, false, false);
        for tick in axis.ticks.into_iter().filter(|v| *v >= lo && *v <= hi) {
            let fraction = if hi > lo { (tick - lo) / (hi - lo) } else { 0.5 };
            let yy = y + height * (1.0 - fraction);
            push(Prim::Segments {
                segs: vec![[x + width, yy, x + width + 4.0, yy]],
                color: INK,
                width: 0.6,
                dashed: false,
            });
            push(Prim::Text {
                x: x + width + 7.0,
                y: yy,
                text: crate::fmt_num(tick),
                size: TICK_FONT * scale,
                anchor: Anchor::Start,
                vertical: false,
                bold: false,
                rich: false,
                color: INK,
            });
        }
        push(Prim::Text {
            x,
            y: y - 10.0 * scale,
            text: name,
            size: TICK_FONT * scale,
            anchor: Anchor::Start,
            vertical: false,
            bold: false,
            rich: false,
            color: INK,
        });
        page.objects.push(ObjectLayout {
            target: ObjectTarget::ColorScale(i),
            bounds: [x, y - 18.0 * scale, width + 50.0 * scale, height + 18.0 * scale],
            items: start..page.items.len(),
            overlay: true,
            hit_segment: None,
        });
    }
}
