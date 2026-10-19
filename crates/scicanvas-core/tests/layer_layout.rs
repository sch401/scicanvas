use scicanvas_core::{
    page::LayerRect,
    scene::{self, Prim},
    Project,
};

fn inset_project() -> Project {
    let mut p = Project::sample();
    let mut inset = p.graphs[0].first_layer().clone();
    inset.placement = Some(LayerRect {
        x: 0.55,
        y: 0.1,
        width: 0.4,
        height: 0.4,
    });
    inset.transparent = true;
    p.graphs[0].page.add_layer(inset);
    p.graphs[0].page.columns = 2;
    p
}

#[test]
fn inset_preserves_parent_layout_and_top_layer_captures_its_background() {
    let p = inset_project();
    let sc = scene::build_page(&p.graphs[0], &p, 1000.0, 600.0, None);
    assert_eq!(sc.layers[0].bounds, [0.0, 0.0, 1000.0, 600.0]);
    assert_eq!(sc.layers[1].bounds, [550.0, 60.0, 400.0, 240.0]);
    assert_eq!(sc.layer_at(560.0, 70.0), Some(1));
    assert!(!sc.layers[1].contains(560.0, 70.0));
    assert_eq!(sc.layer_at(300.0, 300.0), Some(0));
    // 版面按页宽整体放大后坐标有末位舍入差异
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(sc.items.iter().any(|item| matches!(
        item.prim,
        Prim::Rect { x, y, w, h, fill: None, stroke: None }
            if near(x, 550.0) && near(y, 60.0) && near(w, 400.0) && near(h, 240.0)
    )));
    let view = scene::View {
        x: (2.0, 4.0),
        y: (1.0, 3.0),
    };
    let zoomed = scene::build_page_views(&p.graphs[0], &p, 1000.0, 600.0, &[None, Some(view)]);
    assert_eq!(zoomed.layers[0], sc.layers[0]);
    assert_eq!(zoomed.layers[1].view(), view);
}

#[test]
fn placement_and_page_dimensions_have_valid_numeric_bounds() {
    for rect in [
        LayerRect {
            x: -0.1,
            ..Default::default()
        },
        LayerRect {
            x: 0.8,
            width: 0.3,
            ..Default::default()
        },
        LayerRect {
            height: 0.0,
            ..Default::default()
        },
        LayerRect {
            y: f64::NAN,
            ..Default::default()
        },
        LayerRect {
            x: 1.0,
            width: f64::EPSILON / 2.0,
            ..Default::default()
        },
        LayerRect {
            width: 1.0 + f64::EPSILON,
            ..Default::default()
        },
    ] {
        assert!(rect.validate().is_err());
    }
    let mut p = Project::sample();
    p.graphs[0].page.width_mm = f64::MAX;
    assert!(p.graphs[0].page.validate().is_err());
}

#[cfg(feature = "serde")]
#[test]
fn v2_migrates_and_v3_free_layout_roundtrips() {
    use scicanvas_core::{project_file, svg};
    let p = inset_project();
    let json = project_file::to_json(&p).unwrap();
    let restored = project_file::from_json(&json).unwrap();
    assert_eq!(restored, p);
    assert_eq!(
        svg::render_page(&restored.graphs[0], &restored),
        svg::render_page(&p.graphs[0], &p)
    );
    let mut old = serde_json::to_value(Project::sample()).unwrap();
    old["version"] = 2.into();
    for layer in old["graphs"][0]["page"]["layers"].as_array_mut().unwrap() {
        layer.as_object_mut().unwrap().remove("placement");
        layer.as_object_mut().unwrap().remove("transparent");
    }
    let migrated = project_file::from_json(&old.to_string()).unwrap();
    assert_eq!(migrated.version, project_file::CURRENT_VERSION);
    assert_eq!(migrated.graphs[0].first_layer().placement, None);
    assert!(!migrated.graphs[0].first_layer().transparent);
    let mut bad: serde_json::Value = serde_json::from_str(&json).unwrap();
    bad["graphs"][0]["page"]["layers"][1]["placement"]["width"] = 0.9.into();
    assert!(project_file::from_json(&bad.to_string()).is_err());
}

#[test]
fn page_margins_shift_grid_layers_and_background_colors_the_page() {
    let mut p = Project::sample();
    let page = &mut p.graphs[0].page;
    let [w, h] = page.logical_size();
    let mm = 96.0 / 25.4;
    page.margin_mm = [10.0, 5.0, 20.0, 15.0];
    page.background = Some([250, 240, 230]);
    page.validate().unwrap();
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let [x, y, bw, bh] = sc.layers[0].bounds;
    assert!(near(x, 10.0 * mm) && near(y, 5.0 * mm), "{:?}", sc.layers[0].bounds);
    assert!(near(bw, w - 30.0 * mm) && near(bh, h - 20.0 * mm));
    // 第一个图元是整页背景；不再有白色填充
    assert!(matches!(
        sc.items[0].prim,
        Prim::Rect {
            x: 0.0,
            y: 0.0,
            fill: Some(((250, 240, 230), _)),
            ..
        }
    ));
    assert!(!sc.items.iter().any(|i| matches!(
        i.prim,
        Prim::Rect {
            fill: Some(((255, 255, 255), _)),
            ..
        }
    )));
    // 保存后读回不变；没设边距和背景的页面不写这两个字段
    #[cfg(feature = "serde")]
    {
        let saved = scicanvas_core::project_file::to_json(&p).unwrap();
        assert_eq!(
            scicanvas_core::project_file::from_json(&saved).unwrap().graphs[0].page,
            p.graphs[0].page
        );
        let plain = scicanvas_core::project_file::to_json(&Project::sample()).unwrap();
        assert!(!plain.contains("margin_mm") && !plain.contains("background"));
    }
    // 边距过大或为负时拒绝
    let page = &mut p.graphs[0].page;
    page.margin_mm = [150.0, 0.0, 60.0, 0.0];
    assert!(page.validate().is_err());
    page.margin_mm = [-1.0, 0.0, 0.0, 0.0];
    assert!(page.validate().is_err());
}

#[test]
fn grid_gaps_separate_panels_and_linked_axes_follow_their_target() {
    let mut p = Project::sample();
    // 两个网格图层：第 2 层只画 C 列（范围不同）
    let mut second = p.graphs[0].first_layer().clone();
    second.y_keys = vec!["C".into()];
    second.x_auto = false;
    second.x_min = 5.0;
    second.x_max = 12.0;
    p.graphs[0].page.add_layer(second);
    p.graphs[0].page.columns = 2;
    let mm = 96.0 / 25.4;
    let layout = |p: &Project, views: &[Option<scene::View>]| scene::layout_page(&p.graphs[0], p, views);
    let before = layout(&p, &[]);
    p.graphs[0].page.gap_mm = [10.0, 0.0];
    p.graphs[0].page.validate().unwrap();
    let sc = layout(&p, &[]);
    let [x0, _, w0, _] = sc.layers[0].bounds;
    let [x1, _, _, _] = sc.layers[1].bounds;
    assert!((x1 - (x0 + w0) - 10.0 * mm).abs() < 1e-9);
    assert!(w0 < before.layers[0].bounds[2]);
    // 第 2 层 X 轴联动到第 1 层：范围相同，第 1 层缩放时跟着变
    p.graphs[0].page.layers_mut()[1].link_x = Some(0);
    p.graphs[0].page.validate().unwrap();
    let sc = layout(&p, &[]);
    assert_eq!(
        (sc.layers[1].xa.lo, sc.layers[1].xa.hi),
        (sc.layers[0].xa.lo, sc.layers[0].xa.hi)
    );
    assert_eq!(sc.layers[1].xa.ticks, sc.layers[0].xa.ticks);
    let zoom = scene::View {
        x: (2.0, 6.0),
        y: (sc.layers[0].ya.lo, sc.layers[0].ya.hi),
    };
    let sc = layout(&p, &[Some(zoom), None]);
    assert_eq!((sc.layers[1].xa.lo, sc.layers[1].xa.hi), (2.0, 6.0));
    // 联动到自身或不存在的图层：拒绝
    p.graphs[0].page.layers_mut()[1].link_x = Some(1);
    assert!(p.graphs[0].page.validate().is_err());
    p.graphs[0].page.layers_mut()[1].link_x = Some(5);
    assert!(p.graphs[0].page.validate().is_err());
    // 删除目标图层后联动取消
    p.graphs[0].page.layers_mut()[1].link_x = Some(0);
    p.graphs[0].page.remove_layer(0).unwrap();
    assert_eq!(p.graphs[0].page.layers()[0].link_x, None);
    p.graphs[0].page.validate().unwrap();
}
