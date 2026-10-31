//! Windows 11 式贴靠助手：子窗口贴到半边或角落后，在剩下的位置列出其他窗口的缩略图，
//! 点击即把该窗口填进去；四分之一贴靠依次填满其余三格。按 Esc 或在缩略图以外按下鼠标取消。

use eframe::egui::{self, pos2, vec2, Color32, Rect, Sense, Stroke};
use scicanvas_core::scene;

use crate::app::SciApp;
use crate::mdi::{self, Geom, Snap, WinState};

pub(crate) struct SnapAssist {
    /// 待填的位置，第一个是当前显示缩略图的位置。
    pub zones: Vec<Snap>,
    /// 本轮已经摆好的窗口（含最先贴靠的那个），不再出现在缩略图里。
    pub placed: Vec<u64>,
    /// 最近一帧各缩略图卡片的位置。
    pub cards: Vec<(u64, Rect)>,
}

const CARD_TITLE_H: f32 = 22.0;
const MAX_CARD_W: f32 = 260.0;
const GAP: f32 = 12.0;

/// 在 `region` 里排 `n` 张卡片（4:3 缩略图加标题栏），选能让卡片最大的列数，整体居中。
pub(crate) fn card_rects(region: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let (mut width, mut cols) = (0.0_f32, 1);
    for c in 1..=n {
        let rows = n.div_ceil(c);
        let cw = (region.width() - GAP * (c - 1) as f32) / c as f32;
        let ch = (region.height() - GAP * (rows - 1) as f32) / rows as f32;
        let w = cw.min((ch - CARD_TITLE_H) / 0.75).min(MAX_CARD_W);
        if w > width {
            (width, cols) = (w, c);
        }
    }
    let width = width.max(40.0);
    let height = width * 0.75 + CARD_TITLE_H;
    let rows = n.div_ceil(cols);
    let grid = vec2(
        cols as f32 * width + (cols - 1) as f32 * GAP,
        rows as f32 * height + (rows - 1) as f32 * GAP,
    );
    let origin = region.center() - grid / 2.0;
    (0..n)
        .map(|i| {
            let (row, col) = (i / cols, i % cols);
            // 最后一行不满时居中
            let in_row = if row == rows - 1 { n - row * cols } else { cols };
            let shift = (cols - in_row) as f32 * (width + GAP) / 2.0;
            Rect::from_min_size(
                origin + vec2(shift + col as f32 * (width + GAP), row as f32 * (height + GAP)),
                vec2(width, height),
            )
        })
        .collect()
}

impl SciApp {
    /// 窗口 `id` 刚贴到 `snap`：有其他打开的窗口时显示贴靠助手。
    pub(crate) fn start_snap_assist(&mut self, id: u64, snap: Snap) {
        let zones = mdi::assist_zones(snap);
        self.snap_assist = (!zones.is_empty()).then(|| SnapAssist {
            zones,
            placed: vec![id],
            cards: Vec::new(),
        });
        if self.assist_candidates().is_empty() {
            self.snap_assist = None;
        }
    }

    /// 可以填进空位的窗口：打开着（含最小化）且本轮还没摆过。
    fn assist_candidates(&self) -> Vec<u64> {
        let Some(assist) = &self.snap_assist else {
            return Vec::new();
        };
        self.all_ids()
            .into_iter()
            .filter(|id| self.open.get(id).copied().unwrap_or(false) && !assist.placed.contains(id))
            .collect()
    }

    pub(crate) fn snap_assist_ui(&mut self, ctx: &egui::Context, ws: Rect) {
        let Some(zone) = self.snap_assist.as_ref().and_then(|a| a.zones.first().copied()) else {
            return;
        };
        let candidates = self.assist_candidates();
        if candidates.is_empty() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.snap_assist = None;
            return;
        }
        let area = mdi::snap_rect(ws, zone).translate(ws.min.to_vec2());
        let panel = area.shrink(4.0);
        let cards = card_rects(panel.shrink(GAP), candidates.len());
        let press = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten());
        if press.is_some_and(|p| !cards.iter().any(|r| r.contains(p))) {
            self.snap_assist = None;
            return;
        }
        let pal = self.pal;
        let mut chosen = None;
        egui::Area::new(egui::Id::new("snap-assist"))
            .order(egui::Order::Foreground)
            .fixed_pos(area.min)
            .constrain(false)
            .show(ctx, |ui| {
                ui.set_clip_rect(ws);
                ui.allocate_exact_size(area.size(), Sense::hover());
                let painter = ui.painter().clone();
                let backdrop = pal.hud_bg;
                painter.rect_filled(panel, 8.0, backdrop);
                painter.rect_stroke(panel, 8.0, Stroke::new(1.0_f32, pal.line));
                for (&id, &rect) in candidates.iter().zip(&cards) {
                    let response = ui.interact(rect, egui::Id::new(("snap-assist-card", id)), Sense::click());
                    self.paint_card(ui, id, rect, response.hovered());
                    if response.clicked() {
                        chosen = Some(id);
                    }
                }
            });
        if let Some(assist) = &mut self.snap_assist {
            assist.cards = candidates.iter().copied().zip(cards).collect();
        }
        if let Some(id) = chosen {
            self.assist_place(id, ws, zone);
        }
    }

    fn paint_card(&self, ui: &egui::Ui, id: u64, rect: Rect, hovered: bool) {
        let pal = self.pal;
        let painter = ui.painter_at(rect.expand(3.0));
        painter.rect_filled(rect.translate(vec2(2.0, 3.0)), 4.0, Color32::from_black_alpha(40));
        painter.rect_filled(rect, 4.0, pal.window);
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), CARD_TITLE_H));
        painter.rect_filled(bar, 4.0, if hovered { pal.title } else { pal.title_in });
        let is_book = self.project.book(id).is_some();
        let icon = Rect::from_center_size(pos2(bar.left() + 12.0, bar.center().y), vec2(13.0, 13.0));
        crate::icons::icon(
            if is_book {
                "book"
            } else if self.project.note(id).is_some() {
                "note"
            } else {
                "graph"
            },
            true,
        )
        .paint_at(ui, icon);
        painter.with_clip_rect(bar.shrink(2.0)).text(
            pos2(icon.right() + 6.0, bar.center().y),
            egui::Align2::LEFT_CENTER,
            self.project.name_of(id).unwrap_or(""),
            egui::FontId::proportional(crate::theme::size::HUD),
            Color32::WHITE,
        );
        let thumb = Rect::from_min_max(pos2(rect.left(), bar.bottom()), rect.max).shrink(4.0);
        painter.rect_filled(thumb, 0.0, Color32::WHITE);
        if let Some(graph) = self.project.graph(id) {
            let sc = scene::layout_page(graph, &self.project, &[]);
            crate::plot::paint(ui, thumb, &sc);
        } else if let Some(note) = self.project.note(id) {
            for (row, text) in note.text.lines().take(8).enumerate() {
                painter.with_clip_rect(thumb).text(
                    thumb.left_top() + vec2(3.0, row as f32 * 14.0),
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::monospace(crate::theme::size::MICRO),
                    Color32::BLACK,
                );
            }
        } else if let Some(book) = self.project.book(id) {
            // 工作簿：表头加前几行数据。
            let p = painter.with_clip_rect(thumb);
            let font = egui::FontId::proportional(crate::theme::size::MICRO);
            let row_h = 14.0;
            let col_w = (thumb.width() / book.columns.len().clamp(1, 4) as f32).max(36.0);
            p.rect_filled(
                Rect::from_min_size(thumb.min, vec2(thumb.width(), row_h)),
                0.0,
                pal.meta_bg,
            );
            for (c, col) in book.columns.iter().enumerate() {
                let x = thumb.left() + c as f32 * col_w;
                if x > thumb.right() {
                    break;
                }
                let cell =
                    |row: usize| Rect::from_min_size(pos2(x, thumb.top() + row as f32 * row_h), vec2(col_w, row_h));
                let header = if col.long_name.is_empty() {
                    &col.key
                } else {
                    &col.long_name
                };
                p.with_clip_rect(cell(0).intersect(thumb)).text(
                    cell(0).left_center() + vec2(3.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    header,
                    font.clone(),
                    pal.ink,
                );
                for (r, value) in col.data.iter().enumerate() {
                    let rect = cell(r + 1);
                    if rect.top() > thumb.bottom() {
                        break;
                    }
                    p.with_clip_rect(rect.intersect(thumb)).text(
                        rect.right_center() - vec2(3.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        value,
                        font.clone(),
                        pal.ink,
                    );
                }
                p.line_segment(
                    [pos2(x + col_w, thumb.top()), pos2(x + col_w, thumb.bottom())],
                    Stroke::new(1.0_f32, pal.line),
                );
            }
        }
        if hovered {
            painter.rect_stroke(rect, 4.0, Stroke::new(2.0_f32, pal.accent));
        }
    }

    /// 把窗口 `id` 放进 `zone`，记下原来的大小，再转到下一个空位。
    fn assist_place(&mut self, id: u64, ws: Rect, zone: Snap) {
        let rect = mdi::snap_rect(ws, zone);
        let geom = self.geoms.entry(id).or_insert_with(|| Geom::new(rect.min, rect.size()));
        geom.restore.get_or_insert(geom.rect.size());
        geom.rect = rect;
        geom.state = WinState::Normal;
        self.open.insert(id, true);
        self.active = Some(id);
        self.raise = Some(id);
        self.explorer_target = Some(id);
        self.project_name = self.project.name_of(id).unwrap_or("").to_string();
        if let Some(assist) = &mut self.snap_assist {
            assist.placed.push(id);
            assist.zones.remove(0);
            if assist.zones.is_empty() {
                self.snap_assist = None;
            }
        }
        if self.assist_candidates().is_empty() {
            self.snap_assist = None;
        }
    }
}
