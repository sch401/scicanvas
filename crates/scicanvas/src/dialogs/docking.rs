use super::window::PANEL_MARGIN;
use super::*;

/// Settings share one docking implementation and keep their existing widget IDs and drafts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DockSide {
    Left,
    Right,
}

#[derive(Clone)]
pub(super) struct DockPanel {
    pub(super) id: egui::Id,
    pub(super) side: Option<DockSide>,
    pub(super) visible: bool,
    pub(super) seen: bool,
    pub(super) rect: Option<egui::Rect>,
    pub(super) share: f32,
    pub(super) float_pos: Option<egui::Pos2>,
    pub(super) collapsed: bool,
    pub(super) chrome_height: Option<f32>,
    pub(super) header_height: Option<f32>,
}

#[derive(Clone, Default)]
pub(super) struct Docks {
    pub(super) panels: Vec<DockPanel>,
    pub(super) left: Option<egui::Rect>,
    pub(super) right: Option<egui::Rect>,
}

pub(super) fn docks(ctx: &egui::Context) -> Docks {
    ctx.data(|d| d.get_temp(egui::Id::new("settings-docks")))
        .unwrap_or_default()
}

pub(super) fn store_docks(ctx: &egui::Context, state: Docks) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("settings-docks"), state));
}

pub(crate) fn reset_docks(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<Docks>(egui::Id::new("settings-docks")));
}

pub(crate) fn begin_docks(ctx: &egui::Context) {
    let mut state = docks(ctx);
    state.left = None;
    state.right = None;
    for panel in &mut state.panels {
        panel.seen = false;
        panel.rect = None;
    }
    store_docks(ctx, state);
}

pub(crate) fn end_docks(ctx: &egui::Context) {
    let mut state = docks(ctx);
    let mut changed = false;
    for panel in &mut state.panels {
        if panel.visible && !panel.seen {
            panel.visible = false;
            changed = true;
        }
    }
    store_docks(ctx, state);
    if changed {
        ctx.request_repaint();
    }
}

pub(crate) fn has_docks(ctx: &egui::Context, side: DockSide) -> bool {
    docks(ctx).panels.iter().any(|p| p.visible && p.side == Some(side))
}

pub(super) fn panel_header_height(ctx: &egui::Context) -> f32 {
    let title = RichText::new("面板").strong();
    let style = ctx.style();
    ctx.fonts(|fonts| title.font_height(fonts, &style))
        .max(style.spacing.interact_size.y)
        + 2.0 * PANEL_MARGIN
}

/// Reserve sidebar space before drawing document windows; each expanded panel gets a scrollable row.
pub(crate) fn place_docks(ui: &mut egui::Ui, side: DockSide, rect: egui::Rect) {
    let ctx = ui.ctx().clone();
    let mut state = docks(&ctx);
    match side {
        DockSide::Left => state.left = Some(rect),
        DockSide::Right => state.right = Some(rect),
    }
    let indices: Vec<_> = state
        .panels
        .iter()
        .enumerate()
        .filter(|(_, p)| p.visible && p.side == Some(side))
        .map(|(i, _)| i)
        .collect();
    let header = panel_header_height(&ctx);
    let expanded: Vec<_> = indices
        .iter()
        .map(|&i| {
            let id = state.panels[i].id.with("collapsing");
            let open = egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, id, true).is_open();
            // Docked rows redistribute space immediately, without animated bodies overlapping their neighbours.
            ctx.animate_bool_with_time(id, open, 0.0);
            open
        })
        .collect();
    let shares: f32 = indices
        .iter()
        .zip(&expanded)
        .filter(|(_, open)| **open)
        .map(|(&i, _)| state.panels[i].share)
        .sum();
    let headers: Vec<_> = indices
        .iter()
        .zip(&expanded)
        .map(|(&i, open)| {
            if *open {
                state.panels[i].chrome_height.unwrap_or(header + 8.0) + 1.0
            } else {
                state.panels[i].header_height.unwrap_or(header)
            }
        })
        .collect();
    let free = (rect.height() - headers.iter().sum::<f32>() - 4.0 * indices.len().saturating_sub(1) as f32).max(0.0);
    let mut y = rect.top();
    for (at, &i) in indices.iter().enumerate() {
        let h = headers[at]
            + if expanded[at] {
                free * state.panels[i].share / shares.max(0.01)
            } else {
                0.0
            };
        let panel_rect = egui::Rect::from_min_size(egui::pos2(rect.left(), y), egui::vec2(rect.width(), h));
        state.panels[i].rect = Some(panel_rect);
        y += h + 4.0;
        if at + 1 < indices.len() && expanded[at] && expanded[at + 1] {
            let divider = egui::Rect::from_min_size(egui::pos2(rect.left(), y - 4.0), egui::vec2(rect.width(), 4.0));
            let response = ui
                .interact(divider, state.panels[i].id.with("dock-divider"), egui::Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
            ui.painter().hline(
                divider.x_range(),
                divider.center().y,
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            if response.dragged() && !response.drag_started() {
                let j = indices[at + 1];
                let pair = state.panels[i].share + state.panels[j].share;
                let delta = response.drag_delta().y * shares / free.max(1.0);
                state.panels[i].share = (state.panels[i].share + delta).clamp(pair * 0.1, pair * 0.9);
                state.panels[j].share = pair - state.panels[i].share;
                ctx.request_repaint();
            }
        }
    }
    store_docks(&ctx, state);
}

pub(super) fn dock_target(ctx: &egui::Context, state: &Docks, pos: egui::Pos2) -> Option<(DockSide, egui::Rect)> {
    let rect = ctx.available_rect();
    let left = state
        .left
        .unwrap_or_else(|| egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + 64.0, rect.bottom())));
    let right = state
        .right
        .unwrap_or_else(|| egui::Rect::from_min_max(egui::pos2(rect.right() - 64.0, rect.top()), rect.max));
    if left.contains(pos) {
        Some((DockSide::Left, left))
    } else if right.contains(pos) {
        Some((DockSide::Right, right))
    } else {
        None
    }
}

pub(super) fn dock_panel(ctx: &egui::Context, id: egui::Id, side: Option<DockSide>, pos: Option<egui::Pos2>) {
    let mut state = docks(ctx);
    let Some(at) = state.panels.iter().position(|p| p.id == id) else {
        return;
    };
    let mut panel = state.panels.remove(at);
    panel.side = side;
    panel.rect = None;
    if side.is_none() {
        panel.float_pos = pos;
    }
    let at = side
        .and_then(|side| {
            pos.and_then(|pos| {
                state
                    .panels
                    .iter()
                    .position(|p| p.visible && p.side == Some(side) && p.rect.is_some_and(|r| pos.y < r.center().y))
            })
        })
        .unwrap_or(state.panels.len());
    state.panels.insert(at, panel);
    store_docks(ctx, state);
    ctx.request_repaint();
}
