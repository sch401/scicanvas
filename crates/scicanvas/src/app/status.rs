use super::*;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatusKind {
    Info,
    Success,
    Warn,
    Error,
}
#[derive(Clone, Debug)]
pub(crate) struct Status {
    pub(crate) kind: StatusKind,
    pub(crate) text: String,
    pub(crate) at: Instant,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            kind: StatusKind::Info,
            text: String::new(),
            at: Instant::now(),
        }
    }
}
impl Status {
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
}
impl std::ops::Deref for Status {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}
impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.text.fmt(f)
    }
}
impl PartialEq<&str> for Status {
    fn eq(&self, text: &&str) -> bool {
        self.text == *text
    }
}
impl StatusKind {
    fn label(self) -> &'static str {
        match self {
            Self::Info => "提示",
            Self::Success => "成功",
            Self::Warn => "警告",
            Self::Error => "错误",
        }
    }
    fn color(self, pal: &Palette) -> egui::Color32 {
        match self {
            Self::Info => pal.ink,
            Self::Success => pal.accent,
            Self::Warn => pal.warn,
            Self::Error => pal.error,
        }
    }
}
impl SciApp {
    pub(super) fn set_status(&mut self, kind: StatusKind, text: String) {
        self.status = Status {
            kind,
            text,
            at: Instant::now(),
        };
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            % 86400;
        let clock = format!("{:02}:{:02}:{:02} UTC", secs / 3600, secs / 60 % 60, secs % 60);
        self.status_messages.push_back((clock, self.status.clone()));
        if self.status_messages.len() > 50 {
            self.status_messages.pop_front();
        }
    }
    pub(crate) fn status_info(&mut self, text: String) {
        self.set_status(StatusKind::Info, text);
    }
    pub(crate) fn status_ok(&mut self, text: String) {
        self.set_status(StatusKind::Success, text);
    }
    pub(crate) fn status_warn(&mut self, text: String) {
        self.set_status(StatusKind::Warn, text);
    }
    pub(crate) fn status_err(&mut self, text: String) {
        self.set_status(StatusKind::Error, text);
    }

    pub(super) fn status_feedback(&mut self, ui: &mut egui::Ui) {
        let elapsed = self.status.at.elapsed();
        let color = if self.status.kind == StatusKind::Success && elapsed >= Duration::from_secs(8) {
            self.pal.weak
        } else {
            self.status.kind.color(&self.pal)
        };
        if self.status.kind == StatusKind::Success && elapsed < Duration::from_secs(8) {
            ui.ctx().request_repaint_after(Duration::from_secs(8) - elapsed);
        }
        if self.busy_action.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Progress);
        }
        let width = if self.stats_text().is_empty() {
            ui.available_width()
        } else {
            ui.available_width() * 0.55
        };
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(crate::theme::space::L, crate::theme::space::L),
                egui::Sense::hover(),
            );
            status_icon(ui.painter(), rect, self.status.kind, color);
            let response = ui
                .allocate_ui_with_layout(
                    egui::vec2((width - rect.width()).max(0.0), ui.spacing().interact_size.y),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(self.status.as_str()).color(color))
                                .truncate()
                                .sense(egui::Sense::click()),
                        )
                    },
                )
                .inner
                .on_hover_text(format!(
                    "{}：{}\n单击查看消息记录",
                    self.status.kind.label(),
                    self.status
                ));
            if response.clicked() {
                self.show_status_messages = true;
            }
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("status-message-rect"), response.rect));
        });
    }

    pub(super) fn status_messages_window(&mut self, ctx: &egui::Context) {
        if !self.show_status_messages {
            return;
        }
        let mut open = true;
        dialogs::window(ctx, "消息记录")
            .centered()
            .default_width(theme::dialog::LARGE)
            .open(&mut open)
            .show(ctx, |ui| {
                if ui.button("复制全部").clicked() {
                    let text = self
                        .status_messages
                        .iter()
                        .map(|(clock, s)| format!("{clock} [{}] {}", s.kind.label(), s.text))
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.ctx().copy_text(text);
                }
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for (clock, status) in self.status_messages.iter().rev() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(clock).small().color(self.pal.weak));
                            ui.label(RichText::new(status.kind.label()).color(status.kind.color(&self.pal)));
                            ui.add(egui::Label::new(&status.text).wrap().selectable(true));
                        });
                    }
                });
            });
        self.show_status_messages = open;
    }
}
fn status_icon(painter: &egui::Painter, rect: egui::Rect, kind: StatusKind, color: egui::Color32) {
    let c = rect.center();
    let r = rect.width() * 0.4;
    let stroke = egui::Stroke::new(1.5_f32, color);
    painter.circle_stroke(c, r, stroke);
    let p = |x: f32, y: f32| c + egui::vec2(x * r, y * r);
    match kind {
        StatusKind::Success => {
            painter.line_segment([p(-0.5, 0.0), p(-0.1, 0.4)], stroke);
            painter.line_segment([p(-0.1, 0.4), p(0.6, -0.4)], stroke);
        }
        StatusKind::Error => {
            painter.line_segment([p(-0.4, -0.4), p(0.4, 0.4)], stroke);
            painter.line_segment([p(-0.4, 0.4), p(0.4, -0.4)], stroke);
        }
        StatusKind::Info => {
            painter.circle_filled(p(0.0, -0.5), 1.0, color);
            painter.line_segment([p(0.0, -0.1), p(0.0, 0.6)], stroke);
        }
        StatusKind::Warn => {
            painter.line_segment([p(0.0, -0.5), p(0.0, 0.2)], stroke);
            painter.circle_filled(p(0.0, 0.55), 1.0, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn u4_operations_set_severity_and_history_keeps_latest_fifty() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into()]);
        app.fit_order(1, "线性拟合").unwrap();
        assert_eq!(app.status.kind, StatusKind::Success);
        app.project
            .book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == "B")
            .unwrap()
            .data = vec!["1".into()].into();
        let before = app.project.clone();
        assert!(app.fft(scicanvas_core::fft::Window::Hamming).is_err());
        assert_eq!(app.status.kind, StatusKind::Error);
        assert_eq!(app.project, before);
        let message = app.status.text.clone();
        app.status.at -= Duration::from_secs(9);
        assert_eq!(app.status.kind, StatusKind::Error);
        assert_eq!(app.status.text, message);
        for i in 0..55 {
            app.status_info(format!("消息 {i}"));
        }
        assert_eq!(app.status_messages.len(), 50);
        assert_eq!(app.status_messages.front().unwrap().1.text, "消息 5");
        assert_eq!(app.status_messages.back().unwrap().1.text, "消息 54");
    }
    #[test]
    fn u4_status_click_opens_history_and_copy_keeps_messages() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.status_err("拟合失败：有效点不足".into());
        let mut h = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(1280., 820.)),
            |a: &mut SciApp, ctx: &egui::Context| {
                egui::TopBottomPanel::bottom("status-test").show(ctx, |ui| a.status_bar(ui));
                a.status_messages_window(ctx);
            },
        );
        for _ in 0..3 {
            h.frame(&mut app, vec![]);
        }
        let at = ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("status-message-rect")))
            .unwrap()
            .center();
        h.click(&mut app, at);
        assert!(app.show_status_messages);
        for _ in 0..3 {
            h.frame(&mut app, vec![]);
        }
        let out = h.frame(&mut app, vec![]);
        let out = h.click_text(&mut app, &out, "复制全部");
        assert!(out.platform_output.copied_text.contains("[错误] 拟合失败：有效点不足"));
        assert!(out.platform_output.copied_text.contains("UTC"));
    }
    #[test]
    fn u4_long_fit_presents_busy_feedback_before_mutating_project() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.active = Some(1);
        app.selection.insert(1, vec!["B".into(), "C".into()]);
        app.run_command("fit.global");
        let before = app.project.clone();
        let mut h = crate::test_ui::Harness::new(
            ctx,
            Some(egui::vec2(1280., 820.)),
            |a: &mut SciApp, c: &egui::Context| a.dialogs(c),
        );
        for _ in 0..5 {
            h.frame(&mut app, vec![]);
        }
        let out = h.frame(&mut app, vec![]);
        h.click_text(&mut app, &out, "拟合");
        assert_eq!(app.status.text, "正在拟合…");
        assert!(app.busy_action.is_some());
        assert_eq!(app.project, before);
        let out = h.frame(&mut app, vec![]);
        assert_eq!(out.platform_output.cursor_icon, egui::CursorIcon::Progress);
        assert_eq!(app.project, before);
        h.frame(&mut app, vec![]);
        assert!(app.busy_action.is_none());
        assert_ne!(app.status.text, "正在拟合…");
    }
}
