//! 界面主题：浅色“石板”和深色“石墨”两套，跟随系统自动切换。
//!
//! egui 的 Style 管控件外观；Palette 放 egui 不认识的、本程序自己用的颜色
//! （工作区背景、表头行、选中列、项目管理器图标等）。

use eframe::egui::{self, vec2, Color32, FontFamily, FontId, Margin, Rounding, Shadow, Stroke, TextStyle, Theme};

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// 本程序自己用到的颜色。
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    /// 强调色：当前窗口标题、选中状态、链接。
    pub accent: Color32,
    /// 多文档工作区的底色。
    pub workspace: Color32,
    /// 工作表“长名称 / 单位 / 注释”三行的底色。
    pub meta_bg: Color32,
    /// 选中列的底色。
    pub sel_bg: Color32,
    pub meta_sel_bg: Color32,
    /// 项目管理器里“表”“图”标记的颜色。
    /// 次要文字。
    pub weak: Color32,
    /// 正文文字。
    pub ink: Color32,
    /// 功能区面板底色。
    pub panel: Color32,
    /// 标题栏和选项卡栏底色。
    pub chrome: Color32,
    /// 上下文选项卡（工作表 / 图形）的标识色。
    pub ctx: Color32,
    pub hover: Color32,
    pub hover_line: Color32,
    pub press: Color32,
    pub line: Color32,
    /// 文件页左侧导航底色。
    pub backstage: Color32,
    /// 子窗口内容底色。
    pub window: Color32,
    /// 当前子窗口标题栏（渐变左、右），其他子窗口标题栏。
    pub title: Color32,
    pub title2: Color32,
    pub title_in: Color32,
}

struct Tokens {
    dark: bool,
    ink: Color32,
    weak: Color32,
    accent: Color32,
    panel: Color32,
    window: Color32,
    workspace: Color32,
    field: Color32,
    stripe: Color32,
    line: Color32,
    line_strong: Color32,
    button: Color32,
    hover: Color32,
    hover_line: Color32,
    press: Color32,
    open: Color32,
    open_line: Color32,
    selection: Color32,
    meta_bg: Color32,
    sel_bg: Color32,
    meta_sel_bg: Color32,
    shadow: u8,
}

/// 浅色“石板”：白色窗口、冷灰面板、蓝色强调。
const LIGHT: Tokens = Tokens {
    dark: false,
    ink: hex(0x1E2633),
    weak: hex(0x6B7685),
    accent: hex(0x2563C9),
    panel: hex(0xF4F6F9),
    window: hex(0xFFFFFF),
    workspace: hex(0xDCE2E9),
    field: hex(0xFFFFFF),
    stripe: hex(0xF6F8FB),
    line: hex(0xD5DCE5),
    line_strong: hex(0xB9C3CF),
    button: hex(0xEBEFF4),
    hover: hex(0xDDE8F9),
    hover_line: hex(0x9DBAE8),
    press: hex(0xC9DBF6),
    open: hex(0xE4ECFA),
    open_line: hex(0xBCCDEA),
    selection: hex(0xCFE0FB),
    meta_bg: hex(0xEEF2F7),
    sel_bg: hex(0xE0EBFC),
    meta_sel_bg: hex(0xD2E1F8),
    shadow: 38,
};

/// 深色“石墨”：深蓝灰底、柔和对比、亮蓝强调。
const DARK: Tokens = Tokens {
    dark: true,
    ink: hex(0xE3E8EF),
    weak: hex(0x9AA4B2),
    accent: hex(0x6EA8FF),
    panel: hex(0x1F232A),
    window: hex(0x262B33),
    workspace: hex(0x15181D),
    field: hex(0x1B1F25),
    stripe: hex(0x2A2F38),
    line: hex(0x39404B),
    line_strong: hex(0x4A5361),
    button: hex(0x313741),
    hover: hex(0x2E4060),
    hover_line: hex(0x4F78B8),
    press: hex(0x34507E),
    open: hex(0x2C3A52),
    open_line: hex(0x45597A),
    selection: hex(0x2B4A7C),
    meta_bg: hex(0x2C323B),
    sel_bg: hex(0x23395C),
    meta_sel_bg: hex(0x2B4670),
    shadow: 110,
};

fn tokens(theme: Theme) -> &'static Tokens {
    match theme {
        Theme::Light => &LIGHT,
        Theme::Dark => &DARK,
    }
}

pub fn palette(theme: Theme) -> Palette {
    let t = tokens(theme);
    Palette {
        dark: t.dark,
        accent: t.accent,
        workspace: t.workspace,
        meta_bg: t.meta_bg,
        sel_bg: t.sel_bg,
        meta_sel_bg: t.meta_sel_bg,
        weak: t.weak,
        ink: t.ink,
        panel: if t.dark { hex(0x2B3039) } else { hex(0xFBFCFE) },
        chrome: if t.dark { hex(0x1B1F25) } else { hex(0xE9EDF2) },
        ctx: if t.dark { hex(0xB79CFF) } else { hex(0x7A4FD6) },
        hover: t.hover,
        hover_line: t.hover_line,
        press: t.press,
        line: t.line,
        backstage: if t.dark { hex(0x2F5F9E) } else { hex(0x2563C9) },
        window: t.window,
        title: if t.dark { hex(0x2F5F9E) } else { hex(0x2C5D9F) },
        title2: if t.dark { hex(0x3A6FB4) } else { hex(0x3F78C2) },
        title_in: if t.dark { hex(0x3B434F) } else { hex(0x9AA5B4) },
    }
}

fn widget(bg: Color32, weak_bg: Color32, line: Stroke, fg: Color32, fg_width: f32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: weak_bg,
        bg_stroke: line,
        rounding: Rounding::same(4.0),
        fg_stroke: Stroke::new(fg_width, fg),
        expansion: 0.0,
    }
}

fn visuals(t: &Tokens) -> egui::Visuals {
    let mut v = if t.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.override_text_color = None;
    v.panel_fill = t.panel;
    v.window_fill = t.window;
    v.window_stroke = Stroke::new(1.0_f32, t.line_strong);
    v.window_rounding = Rounding::same(6.0);
    v.window_shadow = Shadow {
        offset: vec2(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: Color32::from_black_alpha(t.shadow),
    };
    v.popup_shadow = Shadow {
        offset: vec2(0.0, 3.0),
        blur: 10.0,
        spread: 0.0,
        color: Color32::from_black_alpha(t.shadow),
    };
    // 最上层窗口的标题栏用强调色淡底（取自 widgets.open.weak_bg_fill）
    v.window_highlight_topmost = true;
    v.menu_rounding = Rounding::same(5.0);
    v.faint_bg_color = t.stripe;
    v.extreme_bg_color = t.field;
    v.code_bg_color = t.stripe;
    v.hyperlink_color = t.accent;
    v.selection.bg_fill = t.selection;
    v.selection.stroke = Stroke::new(1.0_f32, t.accent);
    v.text_cursor.stroke = Stroke::new(2.0_f32, t.accent);
    v.striped = true;
    v.collapsing_header_frame = false;
    v.indent_has_left_vline = true;
    v.slider_trailing_fill = true;

    v.widgets.noninteractive = widget(t.window, t.panel, Stroke::new(1.0_f32, t.line), t.ink, 1.0);
    v.widgets.inactive = widget(t.button, t.button, Stroke::new(1.0_f32, t.line), t.ink, 1.0);
    v.widgets.hovered = widget(t.hover, t.hover, Stroke::new(1.0_f32, t.hover_line), t.ink, 1.5);
    v.widgets.active = widget(t.press, t.press, Stroke::new(1.0_f32, t.accent), t.ink, 1.5);
    v.widgets.open = widget(t.open, t.open, Stroke::new(1.0_f32, t.open_line), t.ink, 1.0);
    v
}

fn style(theme: Theme) -> egui::Style {
    let t = tokens(theme);
    let mut s = egui::Style {
        visuals: visuals(t),
        text_styles: [
            (TextStyle::Heading, FontId::new(14.5, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        ]
        .into(),
        ..Default::default()
    };
    s.spacing.item_spacing = vec2(6.0, 4.0);
    s.spacing.button_padding = vec2(8.0, 3.0);
    s.spacing.interact_size.y = 22.0;
    s.spacing.menu_margin = Margin::same(6.0);
    s.spacing.window_margin = Margin::same(6.0);
    s.spacing.indent = 14.0;
    s
}

/// 注册两套样式；egui 按系统深浅色自动选用。
pub fn install(ctx: &egui::Context) {
    ctx.set_style_of(Theme::Light, style(Theme::Light));
    ctx.set_style_of(Theme::Dark, style(Theme::Dark));
    ctx.options_mut(|o| o.fallback_theme = Theme::Light);
    ctx.set_theme(egui::ThemePreference::System);
}
