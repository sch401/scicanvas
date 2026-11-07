use super::docking::{dock_panel, docks, panel_header_height};
use super::*;

struct Host {
    ctx: egui::Context,
    harness: crate::test_ui::Harness,
    second: bool,
    size: egui::Vec2,
}
impl Host {
    fn new() -> Self {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        Self {
            harness: crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(1200.0, 900.0)), ()).with_step(0.04),
            ctx,
            second: false,
            size: egui::vec2(1200.0, 900.0),
        }
    }
    fn draw(ctx: &egui::Context, second: bool) {
        begin_docks(ctx);
        egui::TopBottomPanel::top("test-ribbon")
            .exact_height(90.0)
            .show(ctx, |ui| {
                ui.label("Ribbon");
            });
        egui::SidePanel::left("test-left").exact_width(360.0).show(ctx, |ui| {
            ui.add_space(180.0);
            place_docks(ui, DockSide::Left, ui.available_rect_before_wrap());
        });
        if has_docks(ctx, DockSide::Right) {
            egui::SidePanel::right("test-right").exact_width(360.0).show(ctx, |ui| {
                place_docks(ui, DockSide::Right, ui.available_rect_before_wrap())
            });
        }
        egui::CentralPanel::default().show(ctx, |_| {});
        for title in if second { vec!["First", "Second"] } else { vec!["First"] } {
            window(ctx, title).show(ctx, |ui| {
                for i in 0..40 {
                    ui.label(format!("Setting {i}"));
                }
            });
        }
        end_docks(ctx);
    }
    fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        self.harness.size = Some(self.size);
        let second = self.second;
        self.harness.run(events, |ctx| Self::draw(ctx, second)).0
    }
    fn settle(&mut self) {
        for _ in 0..6 {
            self.frame(vec![]);
        }
    }
    fn rect(&self, title: &str) -> egui::Rect {
        self.ctx
            .memory(|m| m.area_rect(egui::Id::new(("scicanvas-dialog", title))))
            .unwrap()
    }
    fn drag(&mut self, start: egui::Pos2, end: egui::Pos2) {
        self.harness.size = Some(self.size);
        let second = self.second;
        self.harness
            .drag(start, start + egui::vec2(30.0, 0.0), end, |ctx| Self::draw(ctx, second));
        self.settle();
    }
    fn title_drag(&mut self, title: &str, end: egui::Pos2) {
        let r = self.rect(title);
        self.drag(egui::pos2(r.center().x, r.top() + 12.0), end);
    }
}

#[test]
fn floating_windows_move_freely_across_all_edges() {
    let mut host = Host::new();
    host.settle();
    host.title_drag("First", egui::pos2(700.0, -120.0));
    let rect = host.rect("First");
    assert!(rect.top() < 0.0, "{rect:?}");
    assert!(rect.is_finite());
    let id = egui::Id::new(("scicanvas-dialog", "First"));
    assert_eq!(docks(&host.ctx).panels.iter().find(|p| p.id == id).unwrap().side, None);
    for end in [
        egui::pos2(-180.0, 400.0),
        egui::pos2(1380.0, 400.0),
        egui::pos2(700.0, 1080.0),
    ] {
        dock_panel(&host.ctx, id, None, Some(egui::pos2(450.0, 150.0)));
        host.settle();
        host.title_drag("First", end);
        let rect = host.rect("First");
        if end.x < 0.0 {
            assert!(rect.left() < 0.0);
        } else if end.x > 1200.0 {
            assert!(rect.right() > 1200.0);
        } else {
            assert!(rect.bottom() > 900.0);
        }
        assert_eq!(docks(&host.ctx).panels.iter().find(|p| p.id == id).unwrap().side, None);
    }
}

#[test]
fn actual_title_drags_dock_stack_reorder_resize_collapse_and_float() {
    let mut host = Host::new();
    host.settle();
    host.title_drag("First", egui::pos2(160.0, 430.0));
    assert!(has_docks(&host.ctx, DockSide::Left));
    let first = host.rect("First");
    assert!(
        first.left() >= 0.0 && first.right() <= 360.0 && first.top() >= 180.0,
        "{first:?}"
    );
    host.second = true;
    host.settle();
    host.title_drag("Second", egui::pos2(160.0, 800.0));
    let first = host.rect("First");
    let second = host.rect("Second");
    assert!(first.bottom() <= second.top() + 1.0, "{first:?} {second:?}");
    assert!(
        second.right() <= 360.0 && second.bottom() <= 900.0,
        "{second:?} header={} reserved={:?}",
        panel_header_height(&host.ctx),
        docks(&host.ctx).panels.iter().map(|p| p.rect).collect::<Vec<_>>()
    );
    let divider = egui::pos2(170.0, (first.bottom() + second.top()) * 0.5);
    host.drag(divider, divider + egui::vec2(0.0, 60.0));
    assert!(
        host.rect("First").height() > first.height() + 20.0,
        "first={:?} now={:?} layer={:?} response={:?} shares={:?}",
        first,
        host.rect("First"),
        host.ctx.layer_id_at(divider),
        host.ctx
            .read_response(egui::Id::new(("scicanvas-dialog", "First")).with("dock-divider")),
        docks(&host.ctx)
            .panels
            .iter()
            .map(|p| (p.side, p.share))
            .collect::<Vec<_>>()
    );
    host.title_drag("Second", egui::pos2(160.0, 210.0));
    assert!(host.rect("Second").top() < host.rect("First").top());
    host.ctx.style_mut(|style| style.animation_time = 0.2);
    // 使用原生折叠箭头；上方折叠后，另一展开面板占用释放的空间。
    let second = host.rect("Second");
    let pos = second.min + egui::vec2(12.0, 12.0);
    host.frame(vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        host.frame(vec![egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        }]);
    }
    host.settle();
    assert!(host.rect("Second").height() < 50.0);
    assert!(host.rect("First").height() > 500.0);
    host.title_drag("First", egui::pos2(780.0, 340.0));
    let state = docks(&host.ctx);
    assert_eq!(
        state
            .panels
            .iter()
            .find(|p| p.id == egui::Id::new(("scicanvas-dialog", "First")))
            .unwrap()
            .side,
        None
    );
    host.title_drag("First", egui::pos2(1180.0, 400.0));
    assert!(has_docks(&host.ctx, DockSide::Right));
    let right = host.rect("First");
    assert!(right.left() >= 840.0 && right.right() <= 1200.0, "{right:?}");
    host.size = egui::vec2(1000.0, 650.0);
    host.settle();
    let right = host.rect("First");
    assert!(right.right() <= 1000.0 && right.bottom() <= 650.0, "{right:?}");
    // 可用面板右键菜单代替拖动操作。
    let pos = egui::pos2(right.center().x, right.top() + 12.0);
    host.frame(vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        host.frame(vec![egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Secondary,
            modifiers: egui::Modifiers::NONE,
        }]);
    }
    let output = host.frame(vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "浮动" => Some(t.pos + egui::vec2(4.0, 4.0)),
            _ => None,
        })
        .expect("panel context menu");
    host.frame(vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        host.frame(vec![egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        }]);
    }
    host.settle();
    assert!(!has_docks(&host.ctx, DockSide::Right));
    host.second = false;
    host.settle();
    assert!(!has_docks(&host.ctx, DockSide::Left));
    reset_docks(&host.ctx);
    host.settle();
    assert!(!has_docks(&host.ctx, DockSide::Right));
}

#[test]
fn p13_drag_guide_remains_visible_above_middle_settings() {
    let mut host = Host::new();
    host.settle();
    let rect = host.rect("First");
    let start = egui::pos2(rect.center().x, rect.top() + 12.);
    host.frame(vec![
        egui::Event::PointerMoved(start),
        crate::test_ui::button(start, true),
    ]);
    host.frame(vec![egui::Event::PointerMoved(start + egui::vec2(30., 0.))]);
    let end = egui::pos2(160., 430.);
    let out = host.frame(vec![egui::Event::PointerMoved(end)]);
    let id = egui::Id::new(("scicanvas-dialog", "First"));
    let layer = host.ctx.layer_id_at(host.rect("First").center()).unwrap();
    assert_eq!(layer.order, egui::Order::Middle);
    let (_, guide) = super::docking::dock_target(&host.ctx, &docks(&host.ctx), end).unwrap();
    let color = host.ctx.style().visuals.selection.bg_fill;
    assert!(out.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Rect(rect)
            if rect.rect == guide && rect.fill == color.gamma_multiply(0.25))
    }));
    assert!(egui::Order::Foreground > layer.order);
    assert_eq!(layer.id, id);
}

#[test]
fn floating_panel_uses_a_desktop_viewport_with_independent_input_and_close() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.set_embed_viewports(false);
    let closing = Rc::new(Cell::new(false));
    let typing = Rc::new(Cell::new(false));
    let descriptors = Rc::new(RefCell::new(Vec::new()));
    egui::Context::set_immediate_viewport_renderer({
        let closing = closing.clone();
        let typing = typing.clone();
        let descriptors = descriptors.clone();
        move |ctx, mut viewport| {
            descriptors.borrow_mut().push((viewport.ids, viewport.builder.clone()));
            let mut input = egui::RawInput {
                viewport_id: viewport.ids.this,
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420., 560.))),
                ..Default::default()
            };
            input.viewports.insert(
                viewport.ids.this,
                egui::ViewportInfo {
                    parent: Some(viewport.ids.parent),
                    outer_rect: Some(egui::Rect::from_min_size(
                        egui::pos2(1500., -200.),
                        egui::vec2(440., 600.),
                    )),
                    events: if closing.get() {
                        vec![egui::ViewportEvent::Close]
                    } else {
                        vec![]
                    },
                    focused: Some(true),
                    ..Default::default()
                },
            );
            if typing.get() {
                input.events.push(egui::Event::Text("updated".into()));
            }
            let _ = ctx.run(input, |child| (viewport.viewport_ui_cb)(child));
        }
    });
    let id = egui::Id::new("desktop-panel-test");
    let mut open = true;
    let mut text = String::new();
    for step in 0..3 {
        typing.set(step == 1);
        closing.set(step == 2);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            window(ctx, "Desktop panel")
                .id(id)
                .open(&mut open)
                .default_width(420.)
                .show(ctx, |ui| {
                    assert_eq!(ui.ctx().viewport_id(), egui::ViewportId(id));
                    assert!(ui.ctx().input(|i| i.viewport().outer_rect.unwrap().left() > 1000.));
                    ui.text_edit_singleline(&mut text).request_focus();
                });
        });
    }
    assert_eq!(text, "updated");
    assert!(!open);
    for (ids, builder) in descriptors.borrow().iter() {
        assert_eq!(ids.this, egui::ViewportId(id));
        assert_eq!(ids.parent, egui::ViewportId::ROOT);
        assert_eq!(builder.decorations, Some(true));
        assert_eq!(builder.resizable, Some(true));
        assert_eq!(builder.inner_size.unwrap().x, 420.);
    }
    assert_eq!(docks(&ctx).panels.iter().find(|p| p.id == id).unwrap().side, None);
}
