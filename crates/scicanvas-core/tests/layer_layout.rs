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
    assert!(sc.items.iter().any(|item| matches!(
        item.prim,
        Prim::Rect {
            x: 550.0,
            y: 60.0,
            w: 400.0,
            h: 240.0,
            fill: None,
            stroke: None
        }
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
    assert_eq!(migrated.version, 3);
    assert_eq!(migrated.graphs[0].first_layer().placement, None);
    assert!(!migrated.graphs[0].first_layer().transparent);
    let mut bad: serde_json::Value = serde_json::from_str(&json).unwrap();
    bad["graphs"][0]["page"]["layers"][1]["placement"]["width"] = 0.9.into();
    assert!(project_file::from_json(&bad.to_string()).is_err());
}
