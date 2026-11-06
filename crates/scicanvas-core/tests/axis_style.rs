use scicanvas_core::{
    page::{AxisStyle, TickSpacing},
    scene::{self, Anchor, Prim},
    ticks::{self, AxisScale},
    Project,
};

#[test]
fn custom_spacing_and_density_guard_cover_linear_and_log_axes() {
    let spacing = TickSpacing {
        major_step: Some(2.5),
        minor_count: Some(4),
    };
    let a = ticks::axis(&[0., 10.], true, 0., 10., false, false, &spacing);
    assert_eq!((a.lo, a.hi), (0., 10.));
    assert_eq!(a.ticks, [0., 2.5, 5., 7.5, 10.]);
    assert_eq!(&a.minor[..4], &[0.5, 1., 1.5, 2.]);
    let a = ticks::axis(
        &[],
        false,
        1.,
        9.,
        false,
        false,
        &TickSpacing {
            major_step: Some(2.),
            ..Default::default()
        },
    );
    assert_eq!((a.lo, a.hi), (1., 9.));
    assert_eq!(a.ticks, [2., 4., 6., 8.]);
    let a = ticks::axis(
        &[0., 10.],
        true,
        0.,
        10.,
        false,
        false,
        &TickSpacing {
            major_step: Some(1e-9),
            ..Default::default()
        },
    );
    assert!(a.ticks.len() <= 200);
    assert_eq!(a.ticks, [0., 2., 4., 6., 8., 10.]);
    let a = ticks::axis_scaled(
        &[],
        false,
        1.,
        1e6,
        AxisScale::Log10,
        false,
        &TickSpacing {
            major_step: Some(2.),
            minor_count: Some(0),
        },
    );
    assert_eq!(a.ticks, [1., 100., 1e4, 1e6]);
    assert!(a.minor.is_empty());
    let mut layer = scicanvas_core::page::Layer {
        x_ticks: TickSpacing {
            major_step: Some(0.5),
            minor_count: Some(4),
        },
        ..Default::default()
    };
    layer.set_axis_scale(true, AxisScale::Log10).unwrap();
    assert!(layer.x_ticks.is_default());
    for scale in [AxisScale::Probability, AxisScale::Logit, AxisScale::Reciprocal] {
        let a = ticks::axis_scaled(&[0.1, 0.9], true, 0., 1., scale, false, &Default::default());
        let b = ticks::axis_scaled(&[0.1, 0.9], true, 0., 1., scale, false, &spacing);
        assert_eq!(a, b);
    }
}

#[test]
fn export_pt_sizes_ignore_page_scale_and_large_text_stays_outside_plot_inside_page() {
    for width in [89., 210.] {
        let mut p = Project::sample();
        let g = p.graph_mut(2).unwrap();
        g.page.width_mm = width;
        g.first_layer_mut().axis_style.tick_font_pt = Some(8.);
        let s = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
        let numeric: Vec<_> = s
            .items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Text { text, size, .. } if text.parse::<f64>().is_ok() => Some(*size),
                _ => None,
            })
            .collect();
        assert!(!numeric.is_empty());
        assert!(numeric.iter().all(|s| (s - 10.666666666666666).abs() < 1e-12));
    }
    let mut p = Project::sample();
    let g = p.graph_mut(2).unwrap();
    g.first_layer_mut().legend = false;
    g.first_layer_mut().x_title = "Measured time with controlled temperature (seconds)".into();
    g.first_layer_mut().y_title = "Measured response amplitude (millivolts)".into();
    g.first_layer_mut().axis_style = AxisStyle {
        axis_width_pt: Some(8.),
        major_len_pt: Some(0.),
        minor_len_pt: Some(0.),
        tick_font_pt: Some(24.),
        label_font_pt: Some(24.),
        title_font_pt: Some(24.),
        ..Default::default()
    };
    let s = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
    let [left, top, w, h] = s.plot;
    let half_stroke = scene::pt_to_units(8.) / 2.;
    for item in &s.items {
        if let Prim::Text {
            x,
            y,
            text,
            size,
            anchor,
            vertical,
            bold,
            rich,
            ..
        } = &item.prim
        {
            if text.is_empty() {
                continue;
            }
            assert!(
                (*size - scene::pt_to_units(24.)).abs() < 1e-12,
                "explicit text size changed: {text}"
            );
            let l = scicanvas_core::text::layout_formatted(text, *size, *bold, *rich);
            let (tw, th) = if *vertical {
                (l.height, l.width)
            } else {
                (l.width, l.height)
            };
            let factor = match anchor {
                Anchor::Start => 0.,
                Anchor::Middle => 0.5,
                Anchor::End => 1.,
            };
            let (x0, y0) = (x - factor * tw, y - th / 2.);
            assert!(
                x0 >= -1e-6 && y0 >= -1e-6 && x0 + tw <= s.w + 1e-6 && y0 + th <= s.h + 1e-6,
                "{text}: {x0},{y0},{tw},{th}"
            );
            assert!(
                x0 + tw <= left - half_stroke + 1e-6
                    || x0 >= left + w + half_stroke - 1e-6
                    || y0 + th <= top - half_stroke + 1e-6
                    || y0 >= top + h + half_stroke - 1e-6,
                "{text}: overlaps plot"
            );
        }
    }
}

#[test]
fn linked_axes_inherit_exact_tick_arrays() {
    let mut p = Project::sample();
    let g = p.graph_mut(2).unwrap();
    g.first_layer_mut().x_ticks = TickSpacing {
        major_step: Some(2.5),
        minor_count: Some(4),
    };
    let mut second = g.first_layer().clone();
    second.x_ticks = TickSpacing {
        major_step: Some(1.),
        minor_count: Some(0),
    };
    second.link_x = Some(0);
    g.page.add_layer(second);
    g.page.columns = 2;
    let s = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
    assert_eq!(s.layers[0].axes.x, s.layers[1].axes.x);
}

#[cfg(feature = "serde")]
#[test]
fn v39_projects_default_fields_theme_capture_and_validation() {
    use scicanvas_core::{graph_theme::GraphTheme, project_file};
    let p = Project::sample();
    let text = project_file::to_json(&p).unwrap();
    for key in ["axis_style", "x_ticks", "y_ticks", "y2_ticks"] {
        assert!(!text.contains(&format!("\"{key}\"")));
    }
    let mut old: serde_json::Value = serde_json::from_str(&text).unwrap();
    old["version"] = 39.into();
    let mut p = project_file::from_json(&old.to_string()).unwrap();
    assert!(p.graph(2).unwrap().first_layer().axis_style.is_default());
    let l = p.graph_mut(2).unwrap().first_layer_mut();
    l.axis_style.axis_width_pt = Some(2.);
    l.x_ticks.major_step = Some(0.5);
    let theme = GraphTheme::capture(l);
    let mut target = scicanvas_core::page::Layer::default();
    theme.apply(&mut target);
    assert_eq!(target.axis_style, l.axis_style);
    assert!(target.x_ticks.is_default());
    let mut old_theme: serde_json::Value = serde_json::from_str(&theme.to_json().unwrap()).unwrap();
    old_theme.as_object_mut().unwrap().remove("axis_style");
    assert!(GraphTheme::from_json(&old_theme.to_string())
        .unwrap()
        .axis_style
        .is_default());
    assert_eq!(p, project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap());
    let page = &mut p.graph_mut(2).unwrap().page;
    for style in [
        AxisStyle {
            axis_width_pt: Some(0.),
            ..Default::default()
        },
        AxisStyle {
            tick_font_pt: Some(73.),
            ..Default::default()
        },
        AxisStyle {
            minor_len_pt: Some(-1.),
            ..Default::default()
        },
    ] {
        page.layers_mut()[0].axis_style = style;
        assert!(page.validate().is_err());
    }
    page.layers_mut()[0].axis_style = AxisStyle::default();
    for spacing in [
        TickSpacing {
            major_step: Some(f64::NAN),
            ..Default::default()
        },
        TickSpacing {
            major_step: Some(0.),
            ..Default::default()
        },
        TickSpacing {
            minor_count: Some(10),
            ..Default::default()
        },
    ] {
        page.layers_mut()[0].x_ticks = spacing;
        assert!(page.validate().is_err());
    }
}

#[test]
fn frame_and_right_axis_share_tick_geometry_and_independent_stroke_widths() {
    use scicanvas_core::page::TickDir;
    for frame in [false, true] {
        for dir in TickDir::ALL {
            for length in [8., 0.] {
                let mut p = Project::sample();
                let layer = p.graph_mut(2).unwrap().first_layer_mut();
                layer.frame = frame;
                layer.tick_dir = dir;
                layer.legend = false;
                layer.edit_styles(|styles| styles.entry("C".into()).or_default().right_axis = true);
                layer.axis_style = AxisStyle {
                    axis_width_pt: Some(2.),
                    tick_width_pt: Some(0.5),
                    major_len_pt: Some(length),
                    minor_len_pt: Some(length / 2.),
                    ..Default::default()
                };
                let s = scene::layout_page(p.graph(2).unwrap(), &p, &[]);
                let ticks: Vec<_> = s
                    .items
                    .iter()
                    .filter_map(|item| match &item.prim {
                        Prim::Segments { segs, width, .. } if (*width - scene::pt_to_units(0.5)).abs() < 1e-8 => {
                            Some(segs)
                        }
                        _ => None,
                    })
                    .flatten()
                    .collect();
                if length == 0. {
                    assert!(ticks.is_empty());
                    continue;
                }
                let axes = &s.layers[0].axes;
                let right = axes.right_y.as_ref().expect("right Y axis");
                let expected = (axes.x.ticks.len() + axes.x.minor.len()) * if frame { 2 } else { 1 }
                    + axes.left_y.ticks.len()
                    + axes.left_y.minor.len()
                    + right.ticks.len()
                    + right.minor.len();
                assert_eq!(ticks.len(), expected, "mirror/right-axis tick count");
                let (inside, outside) = dir.factors();
                for segment in ticks {
                    let extent = (segment[2] - segment[0]).hypot(segment[3] - segment[1]);
                    assert!(
                        (extent - scene::pt_to_units(f64::from(length)) * (inside + outside)).abs() < 1e-8
                            || (extent - scene::pt_to_units(f64::from(length / 2.)) * (inside + outside)).abs() < 1e-8
                    );
                }
                assert!(s.items.iter().any(
                    |item| matches!(&item.prim,Prim::Polyline{width,..} if (*width-scene::pt_to_units(2.)).abs()<1e-8)
                ));
            }
        }
    }
}
