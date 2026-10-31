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
    let mut harness = crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(1200.0, 900.0)), ());
    harness.run(events, |ctx| {
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
    });
    app.finish_frame_edit(app.history_group);
    page
}

fn object_button(at: egui::Pos2, pressed: bool) -> egui::Event {
    crate::test_ui::button(at, pressed)
}

struct ExplorerUi {
    ctx: egui::Context,
    harness: crate::test_ui::Harness,
    // 保留工作表焦点场景的状态；普通项目管理器场景不绘制工作表。
    sheet: Option<SheetState>,
}

fn draw_explorer(app: &mut SciApp, sheet: &mut Option<SheetState>, ctx: &egui::Context) {
    match sheet {
        Some(sheet) => {
            egui::SidePanel::left("explorer")
                .exact_width(190.0)
                .show(ctx, |ui| app.explorer(ui));
            app.sheet_keys(ctx, 1, sheet);
        }
        None => {
            egui::CentralPanel::default().show(ctx, |ui| app.explorer(ui));
        }
    }
}

impl ExplorerUi {
    fn new() -> Self {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        ctx.style_mut(|s| s.animation_time = 0.0);
        let harness = crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(400.0, 600.0)), ()).with_step(0.02);
        Self {
            ctx,
            harness,
            sheet: None,
        }
    }
    fn frame(&mut self, app: &mut SciApp, events: Vec<egui::Event>) -> egui::FullOutput {
        self.harness
            .run(events, |ctx| draw_explorer(app, &mut self.sheet, ctx))
            .0
    }
    fn at(&mut self, app: &mut SciApp, name: &str) -> egui::Pos2 {
        self.frame(app, vec![]);
        crate::test_ui::find_text(&self.frame(app, vec![]), name)
            .map(|pos| pos + egui::vec2(4.0, 4.0))
            .unwrap_or_else(|| panic!("找不到项目标签：{name}"))
    }
    fn button(&mut self, app: &mut SciApp, pos: egui::Pos2, pressed: bool) {
        self.frame(
            app,
            vec![egui::Event::PointerMoved(pos), crate::test_ui::button(pos, pressed)],
        );
    }
    fn click(&mut self, app: &mut SciApp, pos: egui::Pos2) {
        self.button(app, pos, true);
        self.button(app, pos, false);
    }
    fn key(&mut self, app: &mut SciApp, key: egui::Key) {
        self.harness.key(key, |ctx| draw_explorer(app, &mut self.sheet, ctx));
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
    let mut harness = crate::test_ui::Harness::new(
        ctx.clone(),
        Some(egui::vec2(1400.0, 1000.0)),
        |app: &mut SciApp, ctx: &egui::Context| app.dialogs(ctx),
    )
    .with_step(0.02);
    // 列较窄时需经过几帧，才能稳定列宽和换行后的行高。
    for _ in 0..5 {
        harness.frame(app, vec![]);
    }
    let output = harness.frame(app, vec![]);
    if label != "×" {
        harness.click_text(app, &output, label);
        return;
    }
    let pos = {
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
    };
    harness.click(app, pos);
}

fn editor_frame(app: &mut SciApp, ctx: &egui::Context, events: Vec<egui::Event>, props: bool) -> egui::FullOutput {
    app.history_group = None;
    let mut harness = crate::test_ui::Harness::new(ctx.clone(), Some(egui::vec2(1000.0, 900.0)), ());
    let output = harness
        .run(events, |ctx| {
            if props {
                app.props_window(ctx);
            } else {
                egui::CentralPanel::default().show(ctx, |ui| app.book_ui(ui, 1));
            }
        })
        .0;
    app.finish_frame_edit(app.history_group);
    output
}

mod dialog_tests;
mod graph;
mod history;
mod import_export;
mod worksheet;
mod workspace_tests;
