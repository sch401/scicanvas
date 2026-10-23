// 应用级测试的公共辅助函数，各主题文件保留原测试逻辑与断言。
use super::*;

fn app() -> SciApp {
    SciApp::initial(theme::palette(egui::Theme::Light))
}

fn roundtrip(app: &mut SciApp, before: &Project) {
    let after = app.project.clone();
    assert_ne!(&after, before);
    app.run_command("edit.undo");
    assert_eq!(&app.project, before);
    app.run_command("edit.redo");
    assert_eq!(app.project, after);
}

fn object_frame(app: &mut SciApp, ctx: &egui::Context, events: Vec<egui::Event>, editor: bool) -> egui::Rect {
    app.history_group = None;
    let mut page = egui::Rect::NOTHING;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                page = plot::page_rect(
                    ui.available_rect_before_wrap(),
                    app.project.graph(2).unwrap().page.logical_size(),
                );
                app.graph_ui(ui, 2);
            });
            if editor {
                app.object_window(ctx);
            }
        },
    );
    app.finish_frame_edit(app.history_group);
    page
}

fn object_button(at: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

struct ExplorerUi {
    ctx: egui::Context,
    time: f64,
    modifiers: egui::Modifiers,
    /// Draws the explorer as a side panel followed by this active sheet, as the app does.
    sheet: Option<SheetState>,
}

impl ExplorerUi {
    fn new() -> Self {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        Self {
            ctx,
            time: 0.0,
            modifiers: egui::Modifiers::NONE,
            sheet: None,
        }
    }

    fn frame(&mut self, app: &mut SciApp, events: Vec<egui::Event>) -> egui::FullOutput {
        self.time += 0.02;
        self.ctx.run(
            egui::RawInput {
                time: Some(self.time),
                modifiers: self.modifiers,
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 600.0))),
                events,
                ..Default::default()
            },
            |ctx| match &mut self.sheet {
                Some(sheet) => {
                    egui::SidePanel::left("explorer")
                        .exact_width(190.0)
                        .show(ctx, |ui| app.explorer(ui));
                    app.sheet_keys(ctx, 1, sheet);
                }
                None => {
                    egui::CentralPanel::default().show(ctx, |ui| app.explorer(ui));
                }
            },
        )
    }

    fn at(&mut self, app: &mut SciApp, name: &str) -> egui::Pos2 {
        self.frame(app, vec![]);
        self.frame(app, vec![])
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(t) if t.galley.job.text == name => Some(t.pos + egui::vec2(4.0, 4.0)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing explorer label: {name}"))
    }

    fn button(&mut self, app: &mut SciApp, pos: egui::Pos2, pressed: bool) {
        self.frame(
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }

    fn click(&mut self, app: &mut SciApp, pos: egui::Pos2) {
        self.button(app, pos, true);
        self.button(app, pos, false);
    }

    fn key(&mut self, app: &mut SciApp, key: egui::Key) {
        self.frame(
            app,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        self.frame(
            app,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }

    fn drag(&mut self, app: &mut SciApp, source: &str, destination: &str) {
        let from = self.at(app, source);
        let to = self.at(app, destination);
        self.frame(app, vec![egui::Event::PointerMoved(from)]);
        self.button(app, from, true);
        self.frame(app, vec![egui::Event::PointerMoved(from + egui::vec2(12.0, 0.0))]);
        self.frame(app, vec![egui::Event::PointerMoved(to)]);
        self.button(app, to, false);
        self.frame(app, vec![]);
    }
}

/// 渲染当前对话框并点击文字为 `label` 的按钮。
fn press_dialog_button(app: &mut SciApp, label: &str) {
    let ctx = egui::Context::default();
    crate::fonts::setup(&ctx);
    ctx.style_mut(|style| style.animation_time = 0.0);
    let mut time = 0.0;
    let mut frame = |app: &mut SciApp, events| {
        time += 0.02;
        ctx.run(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1000.0))),
                events,
                ..Default::default()
            },
            |ctx| app.dialogs(ctx),
        )
    };
    // Narrow grids need several frames to settle their column widths and wrapped rows.
    for _ in 0..5 {
        frame(app, vec![]);
    }
    let output = frame(app, vec![]);
    let pos = if label == "×" {
        let id = match app.dialog.as_ref().unwrap() {
            Dialog::LayerContents(_) => egui::Id::new(("scicanvas-dialog", "图层内容")),
            Dialog::ColumnProps { edit, .. } => egui::Id::new(("scicanvas-dialog", format!("列属性 — {}", edit.key))),
            Dialog::CategoryOrder { .. } => egui::Id::new(("scicanvas-dialog", "类别顺序")),
            _ => panic!("unsupported close target"),
        };
        let rect = ctx.memory(|m| m.area_rect(id)).unwrap();
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
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
            .expect("title-bar close glyph")
    } else {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + egui::vec2(5.0, 5.0)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing button {label}"))
    };
    frame(app, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

fn editor_frame(app: &mut SciApp, ctx: &egui::Context, events: Vec<egui::Event>, props: bool) -> egui::FullOutput {
    app.history_group = None;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 900.0))),
            events,
            ..Default::default()
        },
        |ctx| {
            if props {
                app.props_window(ctx);
            } else {
                egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
            }
        },
    );
    app.finish_frame_edit(app.history_group);
    output
}

mod dialog_tests;
mod graph;
mod history;
mod import_export;
mod worksheet;
mod workspace_tests;
