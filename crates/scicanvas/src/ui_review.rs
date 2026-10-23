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
        for width in [800.0, 1440.0] {
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
                let ys: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if t.galley
                                .job
                                .sections
                                .first()
                                .is_some_and(|s| s.format.font_id.size == 11.0)
                                && tab.groups.iter().any(|g| g.label == t.galley.job.text) =>
                        {
                            Some(t.pos.y)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(ys.len(), tab.groups.len(), "{} captions", tab.id);
                assert!(ys.iter().all(|y| (y - ys[0]).abs() <= 0.5), "{}: {ys:?}", tab.id);
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
            ("fit.linear", "拟合结果"),
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
                for label in ["复制结果", "残差图", "置信/预测带", "保存为 Note", "关闭"] {
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
