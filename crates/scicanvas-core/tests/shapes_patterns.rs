#![cfg(feature = "serde")]
use scicanvas_core::{
    page::{CurveStyle, FillPattern, PatternKind, ShapeKind, ShapeObject},
    project_file,
    scene::{self, ObjectTarget, Prim},
    svg, PlotType, Project,
};
#[test]
fn b7_shapes_geometry_precise_hit_and_stable_save_ids() {
    let mut p = Project::sample();
    let page = &mut p.graph_mut(2).unwrap().page;
    let id = page.add_shape(ShapeKind::Arrow, [0.1, 0.1], [0.3, 0.3]).unwrap();
    page.shape_mut(id).unwrap().stroke = [12, 34, 56];
    for (kind, a, b) in [
        (ShapeKind::Rect, [0.4, 0.1], [0.6, 0.3]),
        (ShapeKind::Ellipse, [0.7, 0.1], [0.9, 0.3]),
        (ShapeKind::Line, [0.1, 0.5], [0.3, 0.5]),
    ] {
        let id = page.add_shape(kind, a, b).unwrap();
        let shape = page.shape_mut(id).unwrap();
        shape.dashed = true;
        shape.fill = Some(([100, 150, 200], 0.25));
    }
    let sc = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
    assert_eq!(
        sc.objects
            .iter()
            .filter(|o| matches!(o.target, ObjectTarget::Shape(_)))
            .count(),
        4
    );
    let point = [sc.w * 0.2, sc.h * 0.2];
    assert_eq!(sc.object_at(point[0], point[1]), Some(ObjectTarget::Shape(id)));
    assert_ne!(sc.object_at(sc.w * 0.1, sc.h * 0.3), Some(ObjectTarget::Shape(id)));
    let line = sc
        .objects
        .iter()
        .find(|o| o.target == ObjectTarget::Shape(4))
        .unwrap()
        .hit_segment
        .unwrap();
    let center = [(line[0][0] + line[1][0]) / 2., line[0][1]];
    assert_eq!(
        sc.object_at_tolerance(center[0], center[1] + 3., 4.),
        Some(ObjectTarget::Shape(4))
    );
    assert_ne!(
        sc.object_at_tolerance(center[0], center[1] + 5., 4.),
        Some(ObjectTarget::Shape(4))
    );
    let rendered = svg::render_page(p.graph(2).unwrap(), &p);
    assert!(rendered.contains("#0c2238"));
    let json = project_file::to_json(&p).unwrap();
    assert_eq!(p, project_file::from_json(&json).unwrap());
    p.graph_mut(2).unwrap().page.remove_shape(4);
    let next = p
        .graph_mut(2)
        .unwrap()
        .page
        .add_shape(ShapeKind::Line, [0.2, 0.6], [0.5, 0.6])
        .unwrap();
    assert_eq!(next, 5);
    let ellipse = ShapeObject::new(ShapeKind::Ellipse, [0.1, 0.1], [0.5, 0.5]);
    assert!(scene::shape_primitives(&ellipse, 800., 600.)
        .iter()
        .any(|p| matches!(p,Prim::Polyline{pts,..} if pts.len()==97)));
    let mut reversed = ShapeObject::new(ShapeKind::Arrow, [0.8, 0.2], [0.2, 0.8]);
    reversed.move_to([0.9, 0.9]);
    assert!((reversed.origin()[0] - 0.4).abs() < 1e-12);
    assert!(reversed.a[0] > reversed.b[0]);
    reversed.validate().unwrap();
}
fn in_u(p: [f64; 2]) -> bool {
    p[0] >= -1e-8
        && p[0] <= 40. + 1e-8
        && p[1] >= -1e-8
        && p[1] <= 24. + 1e-8
        && !(p[0] > 16. + 1e-8 && p[0] < 24. - 1e-8 && p[1] > 8. + 1e-8)
}
#[test]
fn b7_hatches_even_odd_clipping_and_dot_grid() {
    let poly = [
        [0., 0.],
        [40., 0.],
        [40., 24.],
        [24., 24.],
        [24., 8.],
        [16., 8.],
        [16., 24.],
        [0., 24.],
    ];
    for kind in PatternKind::ALL {
        let pattern = FillPattern {
            kind,
            color: Some([20, 30, 40]),
            ..Default::default()
        };
        let prim = scene::hatch::hatch_polygon(&poly, &pattern, (0, 0, 0));
        match prim {
            Prim::Segments { segs, color, .. } => {
                assert!(!segs.is_empty());
                assert_eq!(color, (20, 30, 40));
                for s in segs {
                    for t in [0., 0.25, 0.5, 0.75, 1.] {
                        assert!(
                            in_u([s[0] + (s[2] - s[0]) * t, s[1] + (s[3] - s[1]) * t]),
                            "{kind:?}: {s:?}"
                        );
                    }
                }
            }
            Prim::Symbols { pts, .. } => {
                assert!(!pts.is_empty());
                assert!(pts.into_iter().all(in_u));
            }
            _ => panic!(),
        }
    }
    let pattern = FillPattern {
        kind: PatternKind::Dots,
        ..Default::default()
    };
    assert!(
        matches!(scene::hatch::hatch_rect([2.,3.,40.,24.],&pattern,(0,0,0)),Prim::Symbols{pts,..} if pts.len()==15)
    );
}
#[test]
fn b7_pattern_geometry_is_bounded_when_zoomed_and_none_is_byte_identical() {
    let mut p = Project::sample();
    let old = svg::render_page(p.graph(2).unwrap(), &p);
    p.graph_mut(2).unwrap().first_layer_mut().set_plot_style(
        "B",
        Some(CurveStyle {
            pattern: None,
            ..Default::default()
        }),
    );
    assert_eq!(old, svg::render_page(p.graph(2).unwrap(), &p));
    for (key, values) in [("A", vec![0., 1e6]), ("B", vec![1., 2.])] {
        p.book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == key)
            .unwrap()
            .data = values.iter().map(ToString::to_string).collect();
    }
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.edit_draft(|d| d.y_keys = vec!["B".into()]);
    layer.set_plot_type(PlotType::Bar).unwrap();
    layer.x_auto = false;
    layer.x_min = 0.;
    layer.x_max = 1.;
    layer.set_plot_style(
        "B",
        Some(CurveStyle {
            pattern: Some(FillPattern {
                kind: PatternKind::Cross,
                color: Some([201, 33, 66]),
                ..Default::default()
            }),
            ..Default::default()
        }),
    );
    let sc = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
    let mut count = 0;
    for item in &sc.items {
        if let Prim::Segments {
            segs,
            color: (201, 33, 66),
            ..
        } = &item.prim
        {
            count += segs.len();
            if let Some([x, y, w, h]) = item.clip {
                for s in segs {
                    for p in [[s[0], s[1]], [s[2], s[3]]] {
                        assert!(p[0] >= x - 1e-7 && p[0] <= x + w + 1e-7 && p[1] >= y - 1e-7 && p[1] <= y + h + 1e-7);
                    }
                }
            }
        }
    }
    assert!(count > 0 && count < 1000, "{count}");
    let rendered = svg::render_page(p.graph(2).unwrap(), &p);
    assert!(!rendered.contains("<pattern"));
    assert!(rendered.contains("#c92142"));
    let json = project_file::to_json(&p).unwrap();
    assert_eq!(p, project_file::from_json(&json).unwrap());
}

#[test]
fn b7_drawing_geometry_matches_independent_numpy_and_matplotlib() {
    let doc: serde_json::Value = serde_json::from_str(include_str!("reference/drawing_geometry.json")).unwrap();
    let points = |value: &serde_json::Value| {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()])
            .collect::<Vec<_>>()
    };
    for case in doc["cases"].as_array().unwrap() {
        if case["type"] == "shape" {
            let kind: ShapeKind = serde_json::from_value(case["kind"].clone()).unwrap();
            let a = points(&serde_json::json!([case["a"], case["b"]]));
            let shape = ShapeObject::new(kind, a[0], a[1]);
            let prims = scene::shape_primitives(&shape, 800., 600.);
            if kind == ShapeKind::Rect {
                let Prim::Rect { x, y, w, h, .. } = prims[0] else {
                    panic!()
                };
                for (a, b) in [x, y, w, h].iter().zip(case["bounds"].as_array().unwrap()) {
                    assert!((a - b.as_f64().unwrap()).abs() < 1e-9);
                }
            } else {
                let Prim::Polyline { pts, .. } = &prims[0] else {
                    panic!()
                };
                for (a, b) in pts.iter().zip(points(&case["points"])) {
                    for i in 0..2 {
                        assert!((a[i] - b[i]).abs() < 1e-9);
                    }
                }
                if kind == ShapeKind::Arrow {
                    let Prim::Triangles { pts, .. } = &prims[1] else {
                        panic!()
                    };
                    for (a, b) in pts.iter().zip(points(&case["head"])) {
                        for i in 0..2 {
                            assert!((a[i] - b[i]).abs() < 1e-9);
                        }
                    }
                }
            }
        } else if case["type"] == "dots" {
            let poly = points(&case["poly"]);
            let pattern = FillPattern {
                kind: PatternKind::Dots,
                ..Default::default()
            };
            let Prim::Symbols { pts, .. } = scene::hatch::hatch_polygon(&poly, &pattern, (0, 0, 0)) else {
                panic!()
            };
            assert_eq!(pts, points(&case["points"]));
        } else {
            let kind = serde_json::from_value(case["kind"].clone()).unwrap();
            let pattern = FillPattern {
                kind,
                ..Default::default()
            };
            let rect: Vec<_> = case["rect"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            let Prim::Segments { mut segs, .. } =
                scene::hatch::hatch_rect(rect.try_into().unwrap(), &pattern, (0, 0, 0))
            else {
                panic!()
            };
            segs.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
            let expected: Vec<Vec<f64>> = case["segments"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect())
                .collect();
            assert_eq!(segs.len(), expected.len());
            for (a, b) in segs.iter().zip(expected) {
                for i in 0..4 {
                    assert!((a[i] - b[i]).abs() < 1e-9);
                }
            }
        }
    }
}

#[test]
fn b7_default_pattern_ink_contrasts_with_solid_bar_fill() {
    let mut p = Project::sample();
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.edit_draft(|d| d.y_keys = vec!["B".into()]);
    layer.set_plot_type(PlotType::Bar).unwrap();
    layer.set_plot_style(
        "B",
        Some(CurveStyle {
            color: Some([15, 77, 146]),
            pattern: Some(FillPattern::default()),
            ..Default::default()
        }),
    );
    let sc = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
    assert!(sc.items.iter().any(|i| matches!(
        &i.prim,
        Prim::Rect {
            fill: Some(((15, 77, 146), _)),
            ..
        }
    )));
    assert!(sc
        .items
        .iter()
        .any(|i| matches!(&i.prim, Prim::Segments { color: (255, 255, 255), segs, .. } if !segs.is_empty())));
}

#[test]
fn arrow_shaft_stops_inside_head_for_thin_and_thick_lines() {
    use scicanvas_core::page::{ShapeKind, ShapeObject};
    for width in [1., 8., 20.] {
        for (a, b) in [
            ([0.1, 0.5], [0.8, 0.5]),
            ([0.8, 0.5], [0.1, 0.5]),
            ([0.1, 0.2], [0.8, 0.8]),
        ] {
            let mut shape = ShapeObject::new(ShapeKind::Arrow, a, b);
            shape.width_pt = width;
            let prims = scene::shape_primitives(&shape, 800., 600.);
            let Prim::Polyline { pts: shaft, .. } = &prims[0] else {
                panic!("shaft")
            };
            let Prim::Triangles { pts: head, .. } = &prims[1] else {
                panic!("head")
            };
            let center = [
                (head[0][0] + head[1][0] + head[2][0]) / 3.,
                (head[0][1] + head[1][1] + head[2][1]) / 3.,
            ];
            assert!((shaft[1][0] - center[0]).abs() < 1e-9 && (shaft[1][1] - center[1]).abs() < 1e-9);
            let distance = (head[0][0] - center[0]).hypot(head[0][1] - center[1]);
            assert!(distance > width * 96. / 72. / 2.);
        }
    }
}
