//! Layout regressions for the existing interface.
use super::*;
use crate::ribbon::{show, Ctx, Item::Smalls, UiState, TABS};
use egui::{vec2, Rect};

#[test]
fn short_settings_fit_their_content_and_keep_footer_controls_visible() {
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
    app.active = Some(2);
    app.run_command("fit.poly");
    let mut output = None;
    for _ in 0..4 {
        output = Some(ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0))),
                ..Default::default()
            },
            |ctx| app.ui_frame(ctx),
        ));
    }
    let output = output.unwrap();
    let window = ctx
        .memory(|m| m.area_rect(egui::Id::new(("scicanvas-dialog", "多项式拟合"))))
        .unwrap();
    assert!(
        window.height() < 260.0,
        "short parameter windows fit their contents: {window:?}"
    );
    for label in ["阶数", "拟合", "取消"] {
        let text = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == label => Some((t, s.clip_rect)),
                _ => None,
            })
            .unwrap();
        let bounds = text.0.galley.rect.translate(text.0.pos.to_vec2());
        assert!(
            text.1.contains_rect(bounds) && window.contains_rect(bounds),
            "{label} must remain visible"
        );
    }
}

#[test]
fn every_tab_keeps_three_rows_and_aligned_group_captions() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for width in [800.0, 1280.0, 1440.0] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            ctx.set_theme(theme);
            let pal = crate::theme::palette(theme);
            let mut app = SciApp::initial(pal);
            for tab in TABS {
                for group in tab.groups {
                    for item in group.items {
                        if let Smalls(ids) = item {
                            assert!(ids.len() <= 3, "{} / {} exceeds three rows", tab.id, group.label);
                        }
                    }
                }
                app.active = Some(if tab.ctx == Some(Ctx::Book) { 1 } else { 2 });
                let mut state = UiState::default();
                state.tab = tab.id;
                let mut clicked = None;
                let mut height = 0.0;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 820.0))),
                        ..Default::default()
                    },
                    |ctx| {
                        show(ctx, &app, &mut state, &pal, &mut clicked);
                        height = ctx.available_rect().top();
                    },
                );
                if tab.id == "graph" && width >= 1280.0 {
                    for label in ["坐标轴", "图例与标签", "标注", "设置", "导出"] {
                        assert!(
                            crate::test_ui::find_text(&output, label).is_some(),
                            "{width} / {theme:?} / {label}"
                        );
                    }
                    assert!(crate::test_ui::find_text(&output, "更多功能").is_none());
                }
                let captions: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if t.galley
                                .job
                                .sections
                                .first()
                                .is_some_and(|s| s.format.font_id.size == 11.0)
                                && (tab.groups.iter().any(|g| g.label == t.galley.job.text)
                                    || t.galley.job.text == "更多") =>
                        {
                            Some((t.galley.job.text.clone(), t.galley.rect.translate(t.pos.to_vec2())))
                        }
                        _ => None,
                    })
                    .collect();
                assert!(!captions.is_empty(), "{} captions", tab.id);
                assert!(
                    captions
                        .iter()
                        .all(|(_, rect)| (rect.top() - captions[0].1.top()).abs() <= 0.5),
                    "{}: {captions:?}",
                    tab.id
                );
                assert!(
                    captions
                        .iter()
                        .all(|(_, rect)| rect.left() >= 0.0 && rect.right() <= width),
                    "{}: {captions:?}",
                    tab.id
                );
                let body_top = captions[0].1.top() - 74.0;
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape {
                        let rect = text.galley.rect.translate(text.pos.to_vec2());
                        if rect.top() >= body_top && rect.bottom() <= captions[0].1.top() {
                            assert!(
                                rect.left() >= 0.0 && rect.right() <= width,
                                "{}: {} outside viewport",
                                tab.id,
                                text.galley.job.text
                            );
                        }
                    }
                }
                if captions.len() < tab.groups.len() {
                    assert!(
                        captions.iter().any(|(label, _)| label == "更多"),
                        "{} missing overflow menu",
                        tab.id
                    );
                }
                assert!(height <= 180.0, "{} ribbon is too tall", tab.id);
            }
        }
    }
}

#[test]
fn long_settings_fit_the_work_area_and_report_actions_stay_visible() {
    for (width, dpi) in [(800.0, 1.5), (1280.0, 1.0)] {
        for (command, title) in [
            ("graph.props", "绘图属性"),
            ("fit.nonlinear", "非线性拟合"),
            ("fit.linear", "拟合报告"),
        ] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            ctx.set_pixels_per_point(dpi);
            let input = || egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 820.0))),
                ..Default::default()
            };
            for _ in 0..2 {
                let _ = ctx.run(input(), |_| {});
            }
            let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
            app.active = Some(2);
            app.run_command(command);
            let mut output = None;
            for _ in 0..4 {
                output = Some(ctx.run(input(), |ctx| app.ui_frame(ctx)));
            }
            let id = if command == "graph.props" {
                egui::Id::new("scicanvas-props")
            } else {
                egui::Id::new(("scicanvas-dialog", title))
            };
            let window = ctx.memory(|m| m.area_rect(id)).unwrap();
            assert!(
                window.top() >= 160.0 && window.bottom() <= 792.0,
                "{command} {width} {dpi}: {window:?}"
            );
            let output = output.unwrap();
            if command == "fit.linear" {
                for label in ["复制结果", "残差图", "置信/预测带（95%）", "保存为 Note", "关闭"] {
                    let (text, clip) = output
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Text(t) if t.galley.job.text == label => Some((t, s.clip_rect)),
                            _ => None,
                        })
                        .unwrap();
                    assert!(
                        clip.contains_rect(text.galley.rect.translate(text.pos.to_vec2())),
                        "{label} stays visible"
                    );
                }
            }
            if command == "fit.nonlinear" {
                for shape in &output.shapes {
                    if let egui::Shape::Text(t) = &shape.shape {
                        if scicanvas_core::nlfit::Model::ALL
                            .iter()
                            .any(|m| m.formula() == t.galley.job.text)
                        {
                            let rect = t.galley.rect.translate(t.pos.to_vec2());
                            assert!(
                                rect.left() >= window.left() && rect.right() <= window.right(),
                                "formula wraps within panel"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn live_settings_keep_edits_through_minimize_close_and_undo() {
    for command in [
        "graph.props",
        "layer.contents",
        "col.props",
        "col.categories",
        "graph.text.manage",
        "legend",
        "analysis.curves",
    ] {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.active = Some(2);
        let id = match command {
            "graph.props" => egui::Id::new("scicanvas-props"),
            "layer.contents" => egui::Id::new(("scicanvas-dialog", "图层内容")),
            "col.props" => {
                app.active = Some(1);
                app.selection.insert(1, vec!["B".into()]);
                egui::Id::new(("scicanvas-dialog", "列属性 — B"))
            }
            "col.categories" => {
                app.active = Some(1);
                app.selection.insert(1, vec!["C".into()]);
                app.project
                    .book_mut(1)
                    .unwrap()
                    .convert_columns(&["C".into()], scicanvas_core::DataKind::Category)
                    .unwrap();
                egui::Id::new(("scicanvas-dialog", "类别顺序"))
            }
            "analysis.curves" => egui::Id::new(("scicanvas-dialog", "分析曲线")),
            "legend" => egui::Id::new(("scicanvas-dialog", "编辑图例")),
            _ => {
                app.project.graph_mut(2).unwrap().page.add_text([0.2, 0.2]).unwrap();
                egui::Id::new(("scicanvas-dialog", "编辑文字"))
            }
        };
        if command == "legend" {
            app.apply_object_action(2, plot::ObjectAction::Edit(plot::ObjectTarget::Legend(0)));
        } else {
            app.run_command(command);
        }
        let mut harness = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(vec2(1400.0, 1000.0)),
            SciApp::ui_frame as fn(&mut SciApp, &egui::Context),
        );
        for _ in 0..5 {
            harness.frame(&mut app, vec![]);
        }
        let before = app.project.clone();
        match &mut app.dialog {
            Some(Dialog::LayerContents(edit)) => edit.draft.grid = !edit.draft.grid,
            Some(Dialog::ColumnProps { edit, .. }) => edit.long_name = "Live column".into(),
            Some(Dialog::CategoryOrder { levels, .. }) => levels.reverse(),
            Some(Dialog::AnalysisCurves { curves, .. }) => curves[0].2 = false,
            _ if command == "graph.props" => app.run_command("graph.grid"),
            _ => {
                harness.frame(&mut app, vec![egui::Event::Text("Live text".into())]);
            }
        }
        let output = harness.frame(&mut app, vec![]);
        for forbidden in ["确定", "取消", "关闭", "应用"] {
            assert!(
                !output
                    .shapes
                    .iter()
                    .any(|shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.job.text == forbidden)),
                "{command} still has {forbidden}"
            );
        }
        if command != "analysis.curves" {
            assert_ne!(app.project, before, "{command} writes before closing");
            let changed = app.project.clone();
            app.run_command("edit.undo");
            harness.frame(&mut app, vec![]);
            assert_eq!(
                app.project, before,
                "{command}: open controls must not reapply an undone setting"
            );
            app.run_command("edit.redo");
            harness.frame(&mut app, vec![]);
            assert_eq!(app.project, changed);
        }
        if command == "analysis.curves" {
            assert!(matches!(app.analysis_source(), Some(Source::Curves { curves, .. }) if curves.len() == 1));
        }
        let saved = scicanvas_core::project_file::to_scv(&app.project).unwrap();
        assert_eq!(scicanvas_core::project_file::from_scv(&saved).unwrap(), app.project);
        let changed = app.project.clone();
        let click =
            |harness: &mut crate::test_ui::Harness<fn(&mut SciApp, &egui::Context)>, app: &mut SciApp, point| {
                harness.click(app, point);
                for _ in 0..3 {
                    harness.frame(app, vec![]);
                }
            };
        let point = ctx
            .read_response(id.with("minimize"))
            .unwrap_or_else(|| panic!("{command}: missing title control"))
            .rect
            .center();
        click(&mut harness, &mut app, point);
        assert!(
            !egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, id.with("collapsing"), true)
                .is_open(),
            "{command} minimizes from its title bar"
        );
        assert_eq!(app.project, changed);
        let point = ctx
            .read_response(id.with("minimize"))
            .unwrap_or_else(|| panic!("{command}: missing title control"))
            .rect
            .center();
        click(&mut harness, &mut app, point);
        assert!(
            egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, id.with("collapsing"), true)
                .is_open()
        );
        let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
        let output = harness.frame(&mut app, vec![]);
        let close = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::LineSegment { points, .. }
                    if rect.contains(points[0])
                        && (points[0].x - points[1].x).abs() > 3.0
                        && (points[0].y - points[1].y).abs() > 3.0
                        && points[0].y < rect.top() + 30.0
                        && points[0].x > rect.right() - 32.0 =>
                {
                    Some(points[0].lerp(points[1], 0.5))
                }
                _ => None,
            })
            .expect("title-bar X");
        click(&mut harness, &mut app, close);
        assert_eq!(app.project, changed, "{command} keeps changes after X");
        assert!(
            app.dialog.is_none() && app.props_for.is_none() && app.object_editor.is_none(),
            "{command} closes from X"
        );
    }
}

#[test]
fn live_panels_follow_external_changes_without_reverting_them() {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
    let mut harness = crate::test_ui::Harness::new(ctx.clone(), None, SciApp::ui_frame);
    app.active = Some(1);
    app.selection.insert(1, vec!["B".into()]);
    app.run_command("col.props");
    harness.frame(&mut app, vec![]);
    app.run_command("col.setX");
    let changed = app.project.clone();
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    assert_eq!(app.project, changed);
    assert!(matches!(&app.dialog, Some(Dialog::ColumnProps { edit, .. }) if edit.role == scicanvas_core::Role::X));
    app.active = Some(2);
    app.run_command("layer.contents");
    harness.frame(&mut app, vec![]);
    app.run_command("graph.grid");
    let changed = app.project.clone();
    harness.frame(&mut app, vec![]);
    harness.frame(&mut app, vec![]);
    assert_eq!(app.project, changed);
    assert!(
        matches!(&app.dialog, Some(Dialog::LayerContents(edit)) if edit.draft == *app.project.graph(2).unwrap().first_layer())
    );
}

#[test]
fn title_buttons_share_native_geometry_at_each_dpi() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for dpi in [1.0, 1.25, 1.5, 2.0] {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            crate::fonts::setup(&ctx);
            ctx.set_theme(theme);
            ctx.set_pixels_per_point(dpi);
            ctx.style_mut(|s| s.animation_time = 0.0);
            let input = || egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 820.0))),
                ..Default::default()
            };
            for _ in 0..2 {
                let _ = ctx.run(input(), |_| {});
            }
            let mut app = SciApp::initial(crate::theme::palette(theme));
            app.active = Some(2);
            app.run_command("graph.props");
            let id = egui::Id::new("scicanvas-props");
            for collapsed in [false, true] {
                if collapsed {
                    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                        &ctx,
                        id.with("collapsing"),
                        true,
                    );
                    state.set_open(false);
                    state.store(&ctx);
                }
                let mut output = None;
                for _ in 0..4 {
                    output = Some(ctx.run(input(), |ctx| app.ui_frame(ctx)));
                }
                let panel = ctx.memory(|m| m.area_rect(id)).unwrap();
                let min = ctx.read_response(id.with("minimize")).unwrap().rect;
                let close_icon = output
                    .unwrap()
                    .shapes
                    .into_iter()
                    .find_map(|shape| match shape.shape {
                        egui::Shape::LineSegment { points, stroke }
                            if panel.contains(points[0])
                                && points[0].x > panel.right() - 32.0
                                && points[0].y < panel.top() + 40.0
                                && (points[0].x - points[1].x).abs() > 3.0
                                && (points[0].y - points[1].y).abs() > 3.0 =>
                        {
                            Some((Rect::from_two_pos(points[0], points[1]), stroke))
                        }
                        _ => None,
                    })
                    .expect("native X");
                let close = close_icon.0.expand(2.0);
                assert!(
                    (min.center().y - close.center().y).abs() < 0.1,
                    "{theme:?} {dpi} collapsed={collapsed}: {min:?} {close:?}"
                );
                assert!((min.width() - close.width()).abs() < 0.1 && (min.height() - close.height()).abs() < 0.1);
                assert!((close.left() - min.right() - ctx.style().spacing.item_spacing.x).abs() < 0.1);
            }
        }
    }
}

#[test]
fn chart_type_menus_open_and_dispatch_in_both_tabs() {
    for (tab, active, theme) in [
        ("plot", 1, egui::Theme::Light),
        ("graph", 2, egui::Theme::Light),
        ("plot", 1, egui::Theme::Dark),
        ("graph", 2, egui::Theme::Dark),
    ] {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        ctx.set_theme(theme);
        let pal = theme::palette(theme);
        let mut app = SciApp::initial(pal);
        app.active = Some(active);
        let mut state = UiState::default();
        state.tab = tab;
        let mut harness = crate::test_ui::Harness::new(
            ctx,
            Some(vec2(if tab == "graph" { 1800.0 } else { 800.0 }, 820.0)),
            |state: &mut UiState, ctx: &egui::Context| {
                let mut clicked = None;
                show(ctx, &app, state, &pal, &mut clicked);
                clicked
            },
        );
        let mut output = harness.frame(&mut state, vec![]);
        for _ in 0..3 {
            output = harness.frame(&mut state, vec![]);
        }
        for label in ["折线图", "散点图", "点线图"] {
            assert!(
                crate::test_ui::find_text(&output, label).is_some(),
                "{tab} should show {label}"
            );
        }
        let direct = crate::test_ui::find_text(&output, "散点图").unwrap() + vec2(5.0, 5.0);
        harness.frame_value(&mut state, vec![egui::Event::PointerMoved(direct)]);
        harness.frame_value(&mut state, vec![crate::test_ui::button(direct, true)]);
        let clicked = harness.frame_value(&mut state, vec![crate::test_ui::button(direct, false)]);
        assert_eq!(clicked, Some("plot.scatter"), "{tab} preview");
        harness.frame(&mut state, vec![]);
        let menu = harness
            .ctx
            .data(|data| data.get_temp::<Rect>(egui::Id::new("ribbon-gallery-menu-rect")))
            .unwrap()
            .center();
        harness.click(&mut state, menu);
        output = harness.frame(&mut state, vec![]);
        for _ in 0..3 {
            output = harness.frame(&mut state, vec![]);
        }
        let rect = |id: &str| {
            harness
                .ctx
                .data(|data| data.get_temp::<Rect>(egui::Id::new(("gallery-grid-tile", id))))
                .unwrap()
        };
        let first = rect("plot.line");
        let columns = [
            rect("plot.line").center().x,
            rect("plot.linesym").center().x,
            rect("plot.spline").center().x,
        ];
        for (_, ids) in crate::ribbon::PLOT_CATEGORIES {
            for (index, id) in ids.iter().enumerate() {
                let tile = rect(id);
                assert!((tile.width() - first.width()).abs() < 0.1, "{tab} / {theme:?} / {id}");
                assert!(
                    (tile.center().x - columns[index % 3]).abs() < 0.1,
                    "{tab} / {theme:?} / {id}: {tile:?}"
                );
            }
        }
        let scroll = crate::test_ui::find_text(&output, "线").expect("gallery category") + vec2(80., 80.);
        let mut position = None;
        for _ in 0..12 {
            position = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape {
                        let at = text.pos + vec2(5., 5.);
                        if text.galley.job.text == "柱状图" && at.y > 180. && shape.clip_rect.contains(at) {
                            return Some(at);
                        }
                    }
                    None
                })
                .max_by(|a, b| a.y.total_cmp(&b.y));
            if position.is_some() {
                break;
            }
            harness.frame(
                &mut state,
                vec![
                    egui::Event::PointerMoved(scroll),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0., -180.),
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            for _ in 0..4 {
                output = harness.frame(&mut state, vec![]);
            }
        }
        let position = position.expect("bar chart reachable in grouped gallery");
        harness.frame_value(&mut state, vec![egui::Event::PointerMoved(position)]);
        harness.frame_value(&mut state, vec![crate::test_ui::button(position, true)]);
        let clicked = harness.frame_value(&mut state, vec![crate::test_ui::button(position, false)]);
        assert_eq!(clicked, Some("plot.bar"), "{tab}");
    }
}

#[test]
fn narrow_ribbon_exposes_overflow_without_horizontal_scrolling() {
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    let pal = theme::palette(egui::Theme::Light);
    let mut app = SciApp::initial(pal);
    app.active = Some(1);
    let mut state = UiState::default();
    state.tab = "analysis";
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(vec2(800.0, 820.0)),
        |state: &mut UiState, ctx: &egui::Context| {
            let mut clicked = None;
            show(ctx, &app, state, &pal, &mut clicked);
            clicked
        },
    );
    let mut output = harness.frame(&mut state, vec![]);
    for _ in 0..3 {
        output = harness.frame(&mut state, vec![]);
    }
    let before = crate::test_ui::find_text(&output, "更多功能").expect("overflow menu");
    output = harness.frame(
        &mut state,
        vec![
            egui::Event::PointerMoved(egui::pos2(400.0, 140.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(-300.0, 0.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let after = crate::test_ui::find_text(&output, "更多功能").expect("overflow remains visible");
    assert!((before.x - after.x).abs() < 0.5);
    harness.click_text(&mut state, &output, "更多功能");
    output = harness.frame(&mut state, vec![]);
    assert!(
        crate::test_ui::find_text(&output, "信号处理").is_some(),
        "overflow contains hidden groups"
    );
    harness.click_text(&mut state, &output, "信号处理");
    output = harness.frame(&mut state, vec![]);
    let position = crate::test_ui::find_text(&output, crate::commands::find("signal.fft").unwrap().label)
        .expect("overflow submenu contains commands")
        + vec2(4.0, 4.0);
    harness.frame_value(&mut state, vec![egui::Event::PointerMoved(position)]);
    harness.frame_value(&mut state, vec![crate::test_ui::button(position, true)]);
    let clicked = harness.frame_value(&mut state, vec![crate::test_ui::button(position, false)]);
    assert_eq!(clicked, Some("signal.fft"));
}

#[test]
fn chart_previews_expand_with_window_width_and_resize_consistently() {
    let ids = crate::ribbon::PLOT_TYPES;
    for (tab, active) in [("plot", 1), ("graph", 2)] {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        let pal = theme::palette(egui::Theme::Light);
        let mut app = SciApp::initial(pal);
        app.active = Some(active);
        let mut state = UiState::default();
        state.tab = tab;
        let mut harness = crate::test_ui::Harness::new(
            ctx,
            Some(vec2(800.0, 820.0)),
            |state: &mut UiState, ctx: &egui::Context| {
                let mut clicked = None;
                show(ctx, &app, state, &pal, &mut clicked);
            },
        );
        let mut counts = Vec::new();
        for width in [640.0, 800.0, 1800.0, 640.0] {
            harness.size = Some(vec2(width, 820.0));
            let mut output = harness.frame(&mut state, vec![]);
            for _ in 0..2 {
                output = harness.frame(&mut state, vec![]);
            }
            let count = ids
                .iter()
                .filter_map(|id| commands::find(id))
                .filter(|command| crate::test_ui::find_text(&output, command.label).is_some())
                .count();
            counts.push(count);
        }
        if tab == "graph" {
            assert_eq!(&counts[..2], &[0, 0], "compact graph keeps commands ahead of previews");
            assert!(harness
                .ctx
                .data(|data| data.get_temp::<Rect>(egui::Id::new("ribbon-gallery-menu-rect")))
                .is_some());
        } else {
            assert!(counts[0] >= 1 && counts[1] >= 3, "{tab}: {counts:?}");
        }
        assert!(counts[2] > 4 && counts[2] > counts[1], "{tab}: {counts:?}");
        assert!(counts.iter().all(|count| *count <= ids.len()));
        assert_eq!(counts[3], counts[0], "{tab}: {counts:?}");
    }
}

#[path = "ui_capture.rs"]
mod capture;

fn rust_ui_literals(source: &str) -> Vec<(usize, String)> {
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut result = Vec::new();
    while i < bytes.len() {
        if bytes[i..].starts_with(b"//") {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i + 1 < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'"' {
            let start = i;
            i += 1;
            let mut text = Vec::new();
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    text.push(bytes[i]);
                    i += 1;
                }
                text.push(bytes[i]);
                i += 1;
            }
            result.push((start, String::from_utf8(text).unwrap()));
            i += 1;
        } else {
            i += 1;
        }
    }
    result
}

#[test]
fn ui_copy_uses_short_user_facing_hints() {
    fn scan(path: &std::path::Path) {
        for item in std::fs::read_dir(path).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                if path.file_name().unwrap() != "tests" {
                    scan(&path);
                }
                continue;
            }
            if path.extension().is_none_or(|s| s != "rs")
                || path
                    .file_name()
                    .is_some_and(|s| s == "markdown.rs" || s == "ui_review.rs" || s == "ui_capture.rs")
            {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let literals = rust_ui_literals(&source);
            for (offset, text) in &literals {
                // The sixth string argument to command c(...) is searchable metadata, not displayed copy.
                let keywords = path.components().any(|s| s.as_os_str() == "commands")
                    && source[..*offset]
                        .rfind("c(")
                        .is_some_and(|start| literals.iter().filter(|(i, _)| *i > start && *i < *offset).count() == 5);
                if keywords {
                    continue;
                }
                for word in [
                    "NumPy",
                    "numpy",
                    "SciPy",
                    "matplotlib",
                    "statsmodels",
                    "对齐",
                    "快照",
                    "一次性",
                    "算法同",
                    "可撤销",
                    "可以撤销",
                    "暂不支持",
                ] {
                    if text == "修改即时生效，可撤销（Ctrl+Z）" {
                        continue;
                    }
                    if matches!(word, "可撤销" | "可以撤销") && !path.components().any(|p| p.as_os_str() == "dialogs")
                    {
                        continue;
                    }
                    assert!(!text.contains(word), "{}: {text}", path.display());
                }
                if path.parent().is_some_and(|p| p.ends_with("dialogs"))
                    && source[..*offset]
                        .rsplit_once("hint(")
                        .is_some_and(|(_, tail)| !tail.contains(')'))
                {
                    assert!(
                        !text.contains("\\n")
                            && text.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)).count() <= 30,
                        "{}: {text}",
                        path.display()
                    );
                }
            }
        }
    }
    scan(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
}

#[test]
fn visible_plot_and_analysis_commands_have_ribbon_entries() {
    use crate::ribbon::Item;
    let mut visible = std::collections::HashSet::new();
    for tab in TABS {
        for group in tab.groups {
            for item in group.items {
                match item {
                    Item::Big(id) => {
                        visible.insert(*id);
                    }
                    Item::Smalls(ids) | Item::Gallery(ids) => visible.extend(ids.iter().copied()),
                    Item::Scales => {
                        for x in [true, false] {
                            for kind in scicanvas_core::ticks::AxisScale::ALL {
                                visible.insert(crate::commands::axis_scale_command(x, kind));
                            }
                        }
                    }
                }
            }
        }
    }
    for id in crate::ribbon::PLOT_TYPES.iter().copied().chain([
        "fit.linear",
        "fit.poly",
        "fit.nonlinear",
        "fit.global",
        "fit.peaks",
        "fit.compare",
        "fit.calibration",
        "fit.reports",
        "math.smooth",
        "math.diff",
        "math.integ",
        "math.norm",
        "math.interp",
        "math.colmath",
        "math.average",
        "math.diff_sg",
        "signal.baseline",
        "signal.peaks",
        "signal.fft",
        "signal.ifft",
        "signal.filter",
        "signal.convolve",
        "signal.correlate",
        "stats.columns",
        "stats.rows",
        "stats.frequency",
        "stats.whole",
        "stats.compare",
        "stats.summary",
        "stats.ttest",
        "stats.anova",
        "stats.anova2",
        "stats.correlation",
        "stats.normality",
        "stats.grubbs",
        "stats.dixon",
        "stats.ks2",
        "graph.style.copy",
        "graph.style.paste",
        "graph.new",
        "project.folder.new",
        "plot.builder",
        "plot.panels_stacked",
        "plot.panels_side",
        "plot.panels_grid4",
    ]) {
        assert!(crate::commands::find(id).is_some(), "{id}");
        assert!(visible.contains(id), "missing ribbon entry: {id}");
    }
    let mut grouped = Vec::new();
    for (_, ids) in crate::ribbon::PLOT_CATEGORIES {
        grouped.extend(ids.iter().copied());
    }
    grouped.sort();
    assert!(grouped.windows(2).all(|p| p[0] != p[1]));
    let mut all = crate::ribbon::PLOT_TYPES.to_vec();
    all.sort();
    assert_eq!(grouped, all);
}

#[test]
fn command_labels_use_dialog_ellipsis_and_readable_spacing() {
    for id in [
        "fit.poly",
        "fit.nonlinear",
        "fit.global",
        "fit.peaks",
        "fit.compare",
        "fit.calibration",
        "fit.reports",
        "stats.whole",
        "signal.convolve",
        "math.interp",
        "plot.function",
    ] {
        assert!(crate::commands::find(id).unwrap().label.ends_with('…'), "{id}");
    }
    for id in [
        "book.new",
        "graph.new",
        "edit.undo",
        "plot.line",
        "graph.style.copy",
        "graph.style.paste",
    ] {
        assert!(!crate::commands::find(id).unwrap().label.ends_with('…'), "{id}");
    }
    for command in crate::commands::commands() {
        let chars = command.label.chars().collect::<Vec<_>>();
        for pair in chars.windows(2) {
            let cjk = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
            assert!(
                !(pair[0].is_ascii_alphabetic() && cjk(pair[1]) || cjk(pair[0]) && pair[1].is_ascii_alphabetic()),
                "{}: {}",
                command.id,
                command.label
            );
        }
    }
}

#[test]
fn axis_scale_commands_use_one_history_step() {
    let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
    app.active = Some(2);
    let before = app.project.clone();
    app.run_command("graph.scale.x.log2");
    assert_eq!(
        app.project.graph(2).unwrap().first_layer().effective_x_scale(),
        scicanvas_core::ticks::AxisScale::Log2
    );
    app.run_command("edit.undo");
    assert_eq!(app.project, before);
}

#[test]
fn axis_scale_menu_dispatches_registered_choice() {
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    crate::fonts::setup(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.);
    let pal = theme::palette(egui::Theme::Light);
    let mut app = SciApp::initial(pal);
    app.active = Some(2);
    let mut state = UiState::default();
    state.tab = "graph";
    let mut harness = crate::test_ui::Harness::new(
        ctx,
        Some(vec2(1800., 820.)),
        |state: &mut UiState, ctx: &egui::Context| {
            let mut clicked = None;
            show(ctx, &app, state, &pal, &mut clicked);
            clicked
        },
    );
    for _ in 0..4 {
        harness.frame(&mut state, vec![]);
    }
    let output = harness.frame(&mut state, vec![]);
    harness.click_text(&mut state, &output, "X：线性");
    let output = harness.frame(&mut state, vec![]);
    let at = crate::test_ui::text_rect(&output, scicanvas_core::ticks::AxisScale::Log2.label())
        .expect("scale option")
        .center();
    harness.frame_value(
        &mut state,
        vec![egui::Event::PointerMoved(at), crate::test_ui::button(at, true)],
    );
    let clicked = harness.frame_value(&mut state, vec![crate::test_ui::button(at, false)]);
    assert_eq!(clicked, Some("graph.scale.x.log2"));
}

#[test]
fn dialog_code_lines_remain_formatter_friendly() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dialogs");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (line, source) in text.lines().enumerate() {
            assert!(
                source.chars().count() <= 140 || !rust_ui_literals(source).is_empty(),
                "{}:{}",
                path.display(),
                line + 1
            );
        }
    }
}

#[test]
fn u6_group_icons_are_distinct() {
    let mut duplicates = Vec::new();
    for tab in TABS {
        for group in tab.groups {
            let mut seen = std::collections::HashMap::new();
            for item in group.items {
                let ids = match item {
                    crate::ribbon::Item::Big(id) => vec![*id],
                    crate::ribbon::Item::Smalls(ids) => ids.to_vec(),
                    _ => vec![],
                };
                for id in ids {
                    let c = crate::commands::find(id).unwrap();
                    if let Some(other) = seen.insert(c.icon, id) {
                        duplicates.push((tab.id, group.label, other, id, c.icon));
                    }
                }
            }
        }
    }
    assert!(duplicates.is_empty(), "{duplicates:?}");
}

#[test]
fn product_source_literals_use_neutral_wording() {
    fn scan(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().unwrap() != "tests" {
                    scan(&path);
                }
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs")
                || path.file_name().is_some_and(|name| name == "ui_review.rs")
            {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for (offset, text) in rust_ui_literals(&source) {
                let legacy_wire_name = path.file_name().unwrap() == "colormap.rs"
                    && source[..offset].trim_end().ends_with("serde(rename =");
                if legacy_wire_name {
                    continue;
                }
                for word in text.split(|character: char| !character.is_ascii_alphabetic()) {
                    assert!(
                        !["Origin", "Nature", "CHM", "LabTalk"].contains(&word),
                        "product wording at {}:{}",
                        path.display(),
                        offset
                    );
                }
            }
        }
    }
    let native = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    scan(&native.join("src"));
    scan(&native.parent().unwrap().join("scicanvas-core/src"));
}
