use super::*;
use crate::mdi::{snap_rect, Snap};

const SCREEN: egui::Vec2 = egui::vec2(1200.0, 800.0);

fn ws() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)
}

fn frame(ctx: &egui::Context, app: &mut SciApp, events: Vec<egui::Event>) {
    let mut harness = crate::test_ui::Harness::new(ctx.clone(), Some(SCREEN), ());
    harness.run(events, |ctx| app.child_windows(ctx));
}

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    crate::test_ui::button(pos, pressed)
}

fn click(ctx: &egui::Context, app: &mut SciApp, pos: egui::Pos2) {
    frame(ctx, app, vec![egui::Event::PointerMoved(pos), button(pos, true)]);
    frame(ctx, app, vec![button(pos, false)]);
    frame(ctx, app, vec![]);
}

fn card(app: &SciApp, id: u64) -> egui::Rect {
    let assist = app.snap_assist.as_ref().expect("snap assist is showing");
    assist
        .cards
        .iter()
        .find(|(c, _)| *c == id)
        .expect("window has a card")
        .1
}

fn setup() -> (egui::Context, SciApp) {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
    // 新的 egui Area 第一帧只量尺寸，第二帧起才可见、可交互。
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

#[test]
fn dragging_a_window_to_an_edge_offers_the_other_windows_for_the_other_half() {
    let (ctx, mut app) = setup();
    let book = app.geoms[&1].rect;
    let start = egui::pos2(book.center().x, book.top() + mdi::TITLE_H / 2.0);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(start), button(start, true)],
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(egui::pos2(150.0, 300.0))],
    );
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(egui::pos2(2.0, 300.0))]);
    frame(&ctx, &mut app, vec![button(egui::pos2(2.0, 300.0), false)]);
    frame(&ctx, &mut app, vec![]);
    assert_eq!(app.geoms[&1].rect, snap_rect(ws(), Snap::Left));
    let assist = app.snap_assist.as_ref().expect("snap assist after snapping");
    assert_eq!(assist.zones, [Snap::Right]);
    let graph = card(&app, 2);
    assert!(snap_rect(ws(), Snap::Right).contains_rect(graph));

    let normal = app.geoms[&2].rect.size();
    click(&ctx, &mut app, graph.center());
    assert!(app.snap_assist.is_none());
    let geom = app.geoms[&2];
    assert_eq!(
        (geom.rect, geom.state),
        (snap_rect(ws(), Snap::Right), WinState::Normal)
    );
    assert_eq!(geom.restore, Some(normal), "the filled window remembers its size");
    assert_eq!(app.active, Some(2));
}

#[test]
fn quarter_snap_fills_the_remaining_quarters_one_by_one() {
    let (ctx, mut app) = setup();
    let book = app.project.new_book();
    app.open.insert(book, true);
    app.geoms.get_mut(&2).unwrap().state = WinState::Min;
    frame(&ctx, &mut app, vec![]);
    app.start_snap_assist(1, Snap::TopLeft);
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);
    assert_eq!(
        app.snap_assist.as_ref().unwrap().cards.len(),
        2,
        "minimized windows are offered too"
    );

    let at = card(&app, book).center();
    click(&ctx, &mut app, at);
    assert_eq!(app.geoms[&book].rect, snap_rect(ws(), Snap::TopRight));
    assert_eq!(
        app.snap_assist.as_ref().unwrap().zones,
        [Snap::BottomLeft, Snap::BottomRight]
    );

    let graph = card(&app, 2);
    assert!(snap_rect(ws(), Snap::BottomLeft).contains_rect(graph));
    click(&ctx, &mut app, graph.center());
    let geom = app.geoms[&2];
    assert_eq!(
        (geom.rect, geom.state),
        (snap_rect(ws(), Snap::BottomLeft), WinState::Normal)
    );
    assert!(app.snap_assist.is_none(), "no windows left to offer");
}

#[test]
fn escape_or_pressing_outside_the_cards_dismisses_without_moving_windows() {
    let (ctx, mut app) = setup();
    let before = app.geoms[&2].rect;
    app.start_snap_assist(1, Snap::Left);
    frame(&ctx, &mut app, vec![]);
    assert!(app.snap_assist.is_some());
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.snap_assist.is_none());

    app.start_snap_assist(1, Snap::Left);
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);
    let outside = snap_rect(ws(), Snap::Right).right_bottom() - egui::vec2(20.0, 20.0);
    assert!(!card(&app, 2).contains(outside));
    click(&ctx, &mut app, outside);
    assert!(app.snap_assist.is_none());
    assert_eq!(app.geoms[&2].rect, before);

    // 只有一个窗口时不显示助手。
    app.open.insert(2, false);
    app.start_snap_assist(1, Snap::Left);
    assert!(app.snap_assist.is_none());
}

#[test]
fn cards_fit_the_zone_and_center_a_short_last_row() {
    let zone = egui::Rect::from_min_size(egui::pos2(600.0, 0.0), egui::vec2(600.0, 800.0));
    for n in 1..=7 {
        let cards = crate::assist::card_rects(zone, n);
        assert_eq!(cards.len(), n);
        for (i, a) in cards.iter().enumerate() {
            assert!(zone.contains_rect(*a), "{n}: {a:?}");
            for b in &cards[i + 1..] {
                assert!(!a.intersects(*b), "{n}: cards overlap");
            }
        }
    }
    let three = crate::assist::card_rects(zone, 3);
    assert!((three[2].center().x - zone.center().x).abs() < 0.5 || three[0].top() != three[2].top());
}
