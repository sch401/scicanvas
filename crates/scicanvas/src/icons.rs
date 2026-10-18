//! 功能区图标和图类型缩略图。
//!
//! 与网页原型共用同一套 SVG，由 egui_extras 的 SVG 加载器渲染。
//! SVG 里用占位色（--ic-b 等），按浅色 / 深色主题替换成实际颜色后缓存。
//! 注意：egui 的 SVG 渲染器不带字体，图标里不能用 <text>。

use std::collections::HashMap;
use std::sync::OnceLock;

use eframe::egui;

const AX: &str = r#"<path d="M4 3.5V20h16.5"/>"#;

fn wrap(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
        body
    )
}

/// (名称, SVG 主体)
fn sources() -> Vec<(&'static str, String)> {
    vec![
        ("note", wrap(r#"<path d="M6 3h9l4 4v14H6zM15 3v5h4M9 12h7M9 16h7"/>"#)),
        (
            "steph",
            wrap(&format!(
                r##"{AX}<path d="M6 16h5V11h4V6h5" stroke="IC_R" stroke-width="1.9"/>"##
            )),
        ),
        (
            "stepv",
            wrap(&format!(
                r##"{AX}<path d="M6 16v-5h5V6h9" stroke="IC_R" stroke-width="1.9"/>"##
            )),
        ),
        ("undo", wrap(r#"<path d="M9 5L4 10l5 5M4 10h9a6 6 0 0 1 0 12"/>"#)),
        ("redo", wrap(r#"<path d="M15 5l5 5-5 5M20 10h-9a6 6 0 0 0 0 12"/>"#)),
        (
            "newbook",
            wrap(
                r##"<rect x="3" y="3.5" width="14" height="16" rx="1.5" fill="IC_PAPER" stroke="IC_B"/><path d="M3 8h14M8 3.5v16M12.5 3.5v16M3 12h14M3 16h14" stroke="IC_B" stroke-width="1" opacity=".75"/><circle cx="18.5" cy="17.5" r="4.3" fill="IC_G" stroke="none"/><path d="M18.5 15.3v4.4M16.3 17.5h4.4" stroke="#ffffff" stroke-width="1.7"/>"##,
            ),
        ),
        (
            "import",
            wrap(
                r##"<path d="M4 14v5.5h16V14"/><path d="M12 3.5v11M7.5 10l4.5 4.5 4.5-4.5" stroke="IC_B" stroke-width="1.9"/>"##,
            ),
        ),
        (
            "open",
            wrap(
                r##"<path d="M3 7.5V18.5a1 1 0 0 0 1 1h14.5l2.5-8H7l-2.5 8" /><path d="M3 7.5V5.5a1 1 0 0 1 1-1h4l2 2h7a1 1 0 0 1 1 1v4" stroke="IC_O"/>"##,
            ),
        ),
        (
            "save",
            wrap(
                r##"<path d="M5 4h11l3 3v12a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z" fill="IC_PAPER"/><rect x="7.5" y="4" width="8" height="5" stroke="IC_B"/><rect x="7" y="13" width="10" height="7" stroke="IC_B"/>"##,
            ),
        ),
        (
            "export",
            wrap(
                r##"<rect x="3" y="5" width="13" height="14" rx="1.5" fill="IC_PAPER"/><path d="M5.5 15l3-3.5 2.2 1.8 3-4" stroke="IC_R"/><path d="M14 10l7-7M16 3h5v5" stroke="IC_B" stroke-width="1.9"/>"##,
            ),
        ),
        (
            "line",
            wrap(&format!(
                r##"{AX}<path d="M6 16l4-6 3.5 3.5L20 6" stroke="IC_R" stroke-width="2"/>"##
            )),
        ),
        (
            "scatter",
            wrap(&format!(
                r##"{AX}<g fill="currentColor" stroke="none"><rect x="6.3" y="13.3" width="3" height="3"/><rect x="10.5" y="9.5" width="3" height="3"/><rect x="15.5" y="5.5" width="3" height="3"/><rect x="14" y="12.5" width="3" height="3"/></g>"##
            )),
        ),
        (
            "linesym",
            wrap(&format!(
                r##"{AX}<path d="M7.5 15l4.5-5 6-3.5" stroke="currentColor"/><g fill="IC_R" stroke="none"><rect x="6" y="13.5" width="3" height="3"/><rect x="10.5" y="8.5" width="3" height="3"/><rect x="16.5" y="5" width="3" height="3"/></g>"##
            )),
        ),
        (
            "bar",
            wrap(&format!(
                r##"{AX}<g fill="IC_R" stroke="none" opacity=".9"><rect x="6.5" y="11" width="3" height="9"/><rect x="11.5" y="6" width="3" height="14"/><rect x="16.5" y="9" width="3" height="11"/></g>"##
            )),
        ),
        (
            "props",
            wrap(
                r##"<path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1"/><circle cx="15" cy="6" r="2" fill="IC_PAPER" stroke="IC_B"/><circle cx="9" cy="12" r="2" fill="IC_PAPER" stroke="IC_B"/><circle cx="17" cy="18" r="2" fill="IC_PAPER" stroke="IC_B"/>"##,
            ),
        ),
        (
            "showall",
            wrap(
                r##"<rect x="3" y="3" width="11" height="8" rx="1" fill="IC_PAPER"/><rect x="6.5" y="7.5" width="11" height="8" rx="1" fill="IC_PAPER"/><rect x="10" y="12" width="11" height="8" rx="1" fill="IC_PAPER" stroke="IC_B"/>"##,
            ),
        ),
        (
            "copy",
            wrap(
                r##"<rect x="8" y="8" width="12" height="13" rx="1.5" fill="IC_PAPER" stroke="IC_B"/><path d="M16 8V4.5A1.5 1.5 0 0 0 14.5 3h-9A1.5 1.5 0 0 0 4 4.5v11A1.5 1.5 0 0 0 5.5 17H8"/>"##,
            ),
        ),
        (
            "clear",
            wrap(
                r##"<rect x="3" y="4" width="13" height="12" rx="1" fill="IC_PAPER"/><path d="M3 8h13M7.5 4v12"/><path d="M14 20h7M14.6 17.2l4.6-4.6 2 2-4.6 4.6z" stroke="IC_O" fill="IC_PAPER"/>"##,
            ),
        ),
        (
            "cascade",
            wrap(
                r##"<rect x="3" y="3" width="12" height="9" rx="1" fill="IC_PAPER"/><rect x="6" y="6.5" width="12" height="9" rx="1" fill="IC_PAPER"/><rect x="9" y="10" width="12" height="9" rx="1" fill="IC_PAPER" stroke="IC_B"/><path d="M9 12.5h12" stroke="IC_B"/>"##,
            ),
        ),
        (
            "tile",
            wrap(
                r##"<rect x="3" y="4" width="8" height="7" rx="1" fill="IC_PAPER" stroke="IC_B"/><rect x="13" y="4" width="8" height="7" rx="1" fill="IC_PAPER"/><rect x="3" y="13" width="8" height="7" rx="1" fill="IC_PAPER"/><rect x="13" y="13" width="8" height="7" rx="1" fill="IC_PAPER"/>"##,
            ),
        ),
        (
            "minall",
            wrap(
                r##"<rect x="3" y="4" width="18" height="11" rx="1.5" opacity=".5"/><rect x="3" y="17" width="7" height="3" rx="1" fill="IC_B" stroke="IC_B"/><rect x="12" y="17" width="7" height="3" rx="1" fill="IC_B" stroke="IC_B"/>"##,
            ),
        ),
        (
            "hideall",
            wrap(
                r##"<rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M7 20h10" stroke="IC_B" stroke-width="2"/>"##,
            ),
        ),
        (
            "fitlin",
            wrap(&format!(
                r##"{AX}<g fill="currentColor" stroke="none"><circle cx="7.5" cy="15.5" r="1.3"/><circle cx="10.5" cy="14" r="1.3"/><circle cx="13" cy="10.5" r="1.3"/><circle cx="16" cy="9.5" r="1.3"/><circle cx="18.5" cy="6.5" r="1.3"/></g><path d="M5.5 17.5L20.5 5" stroke="IC_R" stroke-width="1.9"/>"##
            )),
        ),
        (
            "fitpoly",
            wrap(&format!(
                r##"{AX}<g fill="currentColor" stroke="none"><circle cx="6.5" cy="8" r="1.3"/><circle cx="9" cy="13" r="1.3"/><circle cx="12" cy="15.5" r="1.3"/><circle cx="15.5" cy="13.5" r="1.3"/><circle cx="18.5" cy="7.5" r="1.3"/></g><path d="M5.5 6.5C9 18 15 18 19.5 5.5" stroke="IC_R" stroke-width="1.9"/>"##
            )),
        ),
        (
            "smooth",
            wrap(
                r##"<path d="M3 14l1.5-4 1.5 6 1.5-7 1.5 5 1.5-3 1.5 4 1.5-6 1.5 5 1.5-4 1.5 3 1.5-2" stroke-width="1.1" opacity=".55"/><path d="M3 12.5c4-2.5 6 2 9 0s5-2 9 0" stroke="IC_B" stroke-width="2"/>"##,
            ),
        ),
        (
            "diff",
            wrap(&format!(
                r##"{AX}<path d="M6 17c4-1 7-6 9-11"/><path d="M8 15.5L18.5 6" stroke="IC_R" stroke-width="1.8"/><circle cx="12.8" cy="11" r="1.6" fill="IC_R" stroke="none"/>"##
            )),
        ),
        (
            "integ",
            wrap(
                r##"<path d="M4 20h16.5M4 3.5V20"/><path d="M6 20V15c3-6 6-8 12-9v14z" fill="IC_B" stroke="none" opacity=".28"/><path d="M6 15c3-6 6-8 12-9" stroke="IC_B" stroke-width="1.9"/>"##,
            ),
        ),
        (
            "norm",
            wrap(
                r##"<path d="M4 20h16.5"/><path d="M4 6h16.5" stroke="IC_G" stroke-dasharray="2 2"/><g fill="IC_G" stroke="none" opacity=".85"><rect x="6" y="12" width="3" height="8"/><rect x="10.5" y="6" width="3" height="14"/><rect x="15" y="9" width="3" height="11"/></g>"##,
            ),
        ),
        (
            "stats",
            wrap(
                r##"<path d="M4 20h16.5"/><rect x="7" y="8" width="10" height="7" rx="1" fill="IC_PAPER" stroke="IC_B"/><path d="M12 3.5V8M12 15v3.5M9.5 3.5h5M9.5 18.5h5M7 11.5h10" stroke="IC_B"/>"##,
            ),
        ),
        (
            "setx",
            wrap(
                r##"<rect x="3.5" y="3.5" width="17" height="17" rx="3" fill="IC_PAPER" stroke="IC_B"/><path d="M8.5 8l7 8M15.5 8l-7 8" stroke="IC_B" stroke-width="2"/>"##,
            ),
        ),
        (
            "sety",
            wrap(
                r##"<rect x="3.5" y="3.5" width="17" height="17" rx="3" fill="IC_PAPER" stroke="IC_R"/><path d="M8.5 7.5l3.5 4.5 3.5-4.5M12 12v5" stroke="IC_R" stroke-width="2"/>"##,
            ),
        ),
        (
            "ignore",
            wrap(r##"<rect x="3.5" y="3.5" width="17" height="17" rx="3" fill="IC_PAPER"/><path d="M8 12h8"/>"##),
        ),
        (
            "colins",
            wrap(
                r##"<rect x="3.5" y="3.5" width="8" height="17" rx="1" fill="IC_PAPER"/><path d="M3.5 8h8"/><path d="M17 8.5v7M13.5 12h7" stroke="IC_G" stroke-width="2"/>"##,
            ),
        ),
        (
            "rowins",
            wrap(
                r##"<rect x="3.5" y="3.5" width="17" height="8" rx="1" fill="IC_PAPER"/><path d="M8 3.5v8"/><path d="M12 13.5v7M8.5 17h7" stroke="IC_G" stroke-width="2"/>"##,
            ),
        ),
        (
            "rowdel",
            wrap(
                r##"<rect x="3.5" y="3.5" width="17" height="8" rx="1" fill="IC_PAPER"/><path d="M8 3.5v8"/><path d="M9.5 14.5l5 5M14.5 14.5l-5 5" stroke="IC_R" stroke-width="2"/>"##,
            ),
        ),
        (
            "coldel",
            wrap(
                r##"<rect x="3.5" y="3.5" width="8" height="17" rx="1" fill="IC_PAPER"/><path d="M3.5 8h8"/><path d="M14.5 9.5l5 5M19.5 9.5l-5 5" stroke="IC_R" stroke-width="2"/>"##,
            ),
        ),
        (
            "colclear",
            wrap(
                r##"<rect x="3.5" y="3.5" width="8" height="17" rx="1" fill="IC_PAPER"/><path d="M3.5 8h8"/><path d="M14 18h7M14.5 15l3.5-3.5 2 2-3.5 3.5z" stroke="IC_O"/>"##,
            ),
        ),
        (
            "sortasc",
            wrap(
                r##"<path d="M7 4v16M3.5 7.5L7 4l3.5 3.5" stroke="IC_B" stroke-width="1.8"/><path d="M13 6h3M13 11h5M13 16h7"/>"##,
            ),
        ),
        (
            "sortdesc",
            wrap(
                r##"<path d="M7 4v16M3.5 16.5L7 20l3.5-3.5" stroke="IC_B" stroke-width="1.8"/><path d="M13 6h7M13 11h5M13 16h3"/>"##,
            ),
        ),
        // 对数轴：刻度间距按 log 变化
        (
            "logx",
            wrap(
                r##"<path d="M3.5 17h17"/><path d="M5 17v-4M11 17v-4M14.5 17v-2.5M17 17v-2M18.8 17v-1.5M20.3 17v-1.2" stroke="IC_B" stroke-width="1.8"/><path d="M5 8h15M16.5 5l3.5 3-3.5 3" stroke="IC_B"/>"##,
            ),
        ),
        (
            "logy",
            wrap(
                r##"<path d="M7 3.5v17"/><path d="M7 19h4M7 13h4M7 9.5h2.5M7 7h2M7 5.2h1.5M7 3.7h1.2" stroke="IC_R" stroke-width="1.8"/><path d="M16 19V4M13 7.5l3-3.5 3 3.5" stroke="IC_R"/>"##,
            ),
        ),
        (
            "grid",
            wrap(
                r##"<rect x="3.5" y="3.5" width="17" height="17" rx="1"/><path d="M3.5 9.2h17M3.5 14.8h17M9.2 3.5v17M14.8 3.5v17" stroke-dasharray="1.5 1.5" opacity=".7"/>"##,
            ),
        ),
        (
            "legend",
            wrap(
                r##"<rect x="3" y="5" width="18" height="14" rx="1.5" fill="IC_PAPER"/><path d="M6 10h3" stroke="currentColor" stroke-width="2"/><path d="M6 14h3" stroke="IC_R" stroke-width="2"/><path d="M11 10h7M11 14h5" opacity=".6"/>"##,
            ),
        ),
        (
            "sun",
            wrap(
                r##"<circle cx="12" cy="12" r="4" fill="IC_O" stroke="IC_O"/><path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4" stroke="IC_O"/>"##,
            ),
        ),
        (
            "moon",
            wrap(
                r##"<path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z" fill="IC_B" stroke="IC_B" opacity=".85"/>"##,
            ),
        ),
        (
            "system",
            wrap(
                r##"<rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M12 4h7.5A1.5 1.5 0 0 1 21 5.5v9a1.5 1.5 0 0 1-1.5 1.5H12z" fill="currentColor" opacity=".25" stroke="none"/><path d="M12 4v12M8 20h8M12 16v4"/>"##,
            ),
        ),
        (
            "explorer",
            wrap(
                r##"<rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M9 4v16"/><path d="M5 8h2M5 11h2M5 14h2" stroke="IC_B"/>"##,
            ),
        ),
        (
            "help",
            wrap(
                r##"<circle cx="12" cy="12" r="8.5"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 .9-1 1.6v.6" stroke="IC_B" stroke-width="1.8"/><circle cx="12" cy="17" r="1" fill="IC_B" stroke="none"/>"##,
            ),
        ),
        (
            "search",
            wrap(r##"<circle cx="10.5" cy="10.5" r="6"/><path d="M15 15l5 5"/>"##),
        ),
        (
            "reset",
            wrap(r##"<path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3"/><path d="M4 4.5v4h4" stroke="IC_B"/>"##),
        ),
        ("chevron", wrap(r##"<path d="M6 15l6-6 6 6"/>"##)),
        ("chevrondown", wrap(r##"<path d="M6 9l6 6 6-6"/>"##)),
        (
            "back",
            wrap(r##"<circle cx="12" cy="12" r="9"/><path d="M13.5 8l-4 4 4 4M9.5 12h6"/>"##),
        ),
        (
            "book",
            wrap(
                r##"<rect x="3" y="4" width="18" height="16" rx="1.5" fill="IC_PAPER" stroke="IC_B"/><path d="M3 8.5h18M9 4v16M15 4v16M3 13h18" stroke="IC_B" stroke-width="1"/>"##,
            ),
        ),
        (
            "graph",
            wrap(
                r##"<rect x="3" y="3" width="18" height="18" rx="1.5" fill="IC_PAPER" stroke="IC_R"/><path d="M6.5 17.5v-11M6.5 17.5h11"/><path d="M8 15l3-4 2.5 2 4-5" stroke="IC_R" stroke-width="1.8"/>"##,
            ),
        ),
        (
            "quit",
            wrap(
                r##"<path d="M14 4h4a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-4"/><path d="M10 8l-4 4 4 4M6 12h9" stroke="IC_R"/>"##,
            ),
        ),
    ]
}

/// 图类型画廊缩略图：白底，Origin 默认配色，两种主题通用。
/// 图型缩略图：功能区画廊与悬停说明共用。配色与实际出图一致（深蓝、砖红）。
fn thumbs() -> Vec<(&'static str, String)> {
    let th = |b: &str| {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 58 40" width="58" height="40"><rect width="58" height="40" rx="3" fill="#ffffff" stroke="#d6dbe2"/><path d="M8 4v30h46" fill="none" stroke="#333333" stroke-width="1"/>{}</svg>"##,
            b
        )
    };
    vec![
        (
            "line",
            th(
                r##"<path d="M11 27L19 18L27 22L35 12L43 16L51 9" fill="none" stroke="#0f4d92" stroke-width="1.3" stroke-linejoin="round"/><path d="M11 32L19 29L27 30L35 25L43 26L51 22" fill="none" stroke="#b64342" stroke-width="1.3" stroke-linejoin="round"/>"##,
            ),
        ),
        (
            "scatter",
            th(
                r##"<circle cx="11" cy="27" r="1.7" fill="#0f4d92"/><circle cx="19" cy="18" r="1.7" fill="#0f4d92"/><circle cx="27" cy="22" r="1.7" fill="#0f4d92"/><circle cx="35" cy="12" r="1.7" fill="#0f4d92"/><circle cx="43" cy="16" r="1.7" fill="#0f4d92"/><circle cx="51" cy="9" r="1.7" fill="#0f4d92"/><rect x="9.5" y="30.5" width="3" height="3" fill="#b64342"/><rect x="17.5" y="27.5" width="3" height="3" fill="#b64342"/><rect x="25.5" y="28.5" width="3" height="3" fill="#b64342"/><rect x="33.5" y="23.5" width="3" height="3" fill="#b64342"/><rect x="41.5" y="24.5" width="3" height="3" fill="#b64342"/><rect x="49.5" y="20.5" width="3" height="3" fill="#b64342"/>"##,
            ),
        ),
        (
            "linesym",
            th(
                r##"<path d="M11 27L19 18L27 22L35 12L43 16L51 9" fill="none" stroke="#0f4d92" stroke-width="1.1" stroke-linejoin="round"/><circle cx="11" cy="27" r="1.7" fill="#0f4d92"/><circle cx="19" cy="18" r="1.7" fill="#0f4d92"/><circle cx="27" cy="22" r="1.7" fill="#0f4d92"/><circle cx="35" cy="12" r="1.7" fill="#0f4d92"/><circle cx="43" cy="16" r="1.7" fill="#0f4d92"/><circle cx="51" cy="9" r="1.7" fill="#0f4d92"/><path d="M11 32L19 29L27 30L35 25L43 26L51 22" fill="none" stroke="#b64342" stroke-width="1.1" stroke-linejoin="round"/><rect x="9.5" y="30.5" width="3" height="3" fill="#b64342"/><rect x="17.5" y="27.5" width="3" height="3" fill="#b64342"/><rect x="25.5" y="28.5" width="3" height="3" fill="#b64342"/><rect x="33.5" y="23.5" width="3" height="3" fill="#b64342"/><rect x="41.5" y="24.5" width="3" height="3" fill="#b64342"/><rect x="49.5" y="20.5" width="3" height="3" fill="#b64342"/>"##,
            ),
        ),
        (
            "bar",
            th(
                r##"<rect x="12" y="20" width="5" height="14" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="27" y="11" width="5" height="23" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="42" y="16" width="5" height="18" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="17" y="25" width="5" height="9" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="32" y="18" width="5" height="16" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="47" y="22" width="5" height="12" fill="#b64342" stroke="#222222" stroke-width=".5"/>"##,
            ),
        ),
        (
            "stacked_bar",
            th(
                r##"<rect x="13" y="24" width="9" height="10" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="27" y="20" width="9" height="14" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="41" y="26" width="9" height="8" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="13" y="16" width="9" height="8" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="27" y="9" width="9" height="11" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="41" y="13" width="9" height="13" fill="#b64342" stroke="#222222" stroke-width=".5"/>"##,
            ),
        ),
        (
            "percent_stacked_bar",
            th(
                r##"<rect x="13" y="20" width="9" height="14" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="27" y="14" width="9" height="20" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="41" y="24" width="9" height="10" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="13" y="6" width="9" height="14" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="27" y="6" width="9" height="8" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="41" y="6" width="9" height="18" fill="#b64342" stroke="#222222" stroke-width=".5"/>"##,
            ),
        ),
        (
            "floating_bar",
            th(
                r##"<rect x="13" y="16" width="9" height="12" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="27" y="9" width="9" height="13" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="41" y="18" width="9" height="11" fill="#0f4d92" stroke="#222222" stroke-width=".5"/>"##,
            ),
        ),
        (
            "horizontal_bar",
            th(
                r##"<rect x="8" y="7" width="30" height="4" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="8" y="17" width="40" height="4" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="8" y="27" width="22" height="3" fill="#0f4d92" stroke="#222222" stroke-width=".5"/><rect x="8" y="11" width="22" height="4" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="8" y="21" width="28" height="4" fill="#b64342" stroke="#222222" stroke-width=".5"/><rect x="8" y="30" width="34" height="4" fill="#b64342" stroke="#222222" stroke-width=".5"/>"##,
            ),
        ),
        (
            "area",
            th(
                r##"<path d="M8 34L8 20L18 14L28 18L38 9L48 13L54 11L54 34Z" fill="#b64342" fill-opacity=".3"/><path d="M8 20L18 14L28 18L38 9L48 13L54 11" fill="none" stroke="#b64342" stroke-width="1.1" stroke-linejoin="round"/><path d="M8 34L8 28L18 24L28 27L38 20L48 23L54 21L54 34Z" fill="#0f4d92" fill-opacity=".35"/><path d="M8 28L18 24L28 27L38 20L48 23L54 21" fill="none" stroke="#0f4d92" stroke-width="1.1" stroke-linejoin="round"/>"##,
            ),
        ),
        (
            "spline",
            th(
                r##"<path d="M11 27C15 20 17 17 21 18S29 25 33 18 41 9 51 11" fill="none" stroke="#0f4d92" stroke-width="1.3"/><path d="M11 32C16 29 20 28 24 30S32 28 36 26 46 22 51 23" fill="none" stroke="#b64342" stroke-width="1.3"/>"##,
            ),
        ),
        (
            "spline_symbol",
            th(
                r##"<path d="M11 27C15 20 17 17 21 18S29 25 33 18 41 9 51 11" fill="none" stroke="#0f4d92" stroke-width="1.1"/><path d="M11 32C16 29 20 28 24 30S32 28 36 26 46 22 51 23" fill="none" stroke="#b64342" stroke-width="1.1"/><circle cx="11" cy="27" r="1.7" fill="#0f4d92"/><circle cx="21" cy="18" r="1.7" fill="#0f4d92"/><circle cx="33" cy="18" r="1.7" fill="#0f4d92"/><circle cx="51" cy="11" r="1.7" fill="#0f4d92"/><rect x="9.5" y="30.5" width="3" height="3" fill="#b64342"/><rect x="22.5" y="28.5" width="3" height="3" fill="#b64342"/><rect x="34.5" y="24.5" width="3" height="3" fill="#b64342"/><rect x="49.5" y="21.5" width="3" height="3" fill="#b64342"/>"##,
            ),
        ),
        (
            "vertical_drop",
            th(
                r##"<path d="M12 34V22" stroke="#0f4d92" stroke-width="1.1"/><path d="M24 34V12" stroke="#0f4d92" stroke-width="1.1"/><path d="M36 34V18" stroke="#0f4d92" stroke-width="1.1"/><path d="M48 34V9" stroke="#0f4d92" stroke-width="1.1"/><circle cx="12" cy="22" r="1.7" fill="#0f4d92"/><circle cx="24" cy="12" r="1.7" fill="#0f4d92"/><circle cx="36" cy="18" r="1.7" fill="#0f4d92"/><circle cx="48" cy="9" r="1.7" fill="#0f4d92"/><path d="M17 34V28" stroke="#b64342" stroke-width="1.1"/><path d="M29 34V24" stroke="#b64342" stroke-width="1.1"/><path d="M41 34V27" stroke="#b64342" stroke-width="1.1"/><path d="M52 34V20" stroke="#b64342" stroke-width="1.1"/><rect x="15.5" y="26.5" width="3" height="3" fill="#b64342"/><rect x="27.5" y="22.5" width="3" height="3" fill="#b64342"/><rect x="39.5" y="25.5" width="3" height="3" fill="#b64342"/><rect x="50.5" y="18.5" width="3" height="3" fill="#b64342"/>"##,
            ),
        ),
        (
            "step_horizontal",
            th(
                r##"<path d="M9 27H17V20H26V24H35V13H44V17H53" fill="none" stroke="#0f4d92" stroke-width="1.3"/><path d="M9 32H17V29H26V30H35V25H44V27H53" fill="none" stroke="#b64342" stroke-width="1.3"/>"##,
            ),
        ),
        (
            "step_vertical",
            th(
                r##"<path d="M9 27V20H17V24H26V13H35V17H44V10H53" fill="none" stroke="#0f4d92" stroke-width="1.3"/><path d="M9 32V29H17V30H26V25H35V27H44V22H53" fill="none" stroke="#b64342" stroke-width="1.3"/>"##,
            ),
        ),
    ]
}

struct ThemeColors {
    ink: &'static str,
    b: &'static str,
    r: &'static str,
    g: &'static str,
    o: &'static str,
    paper: &'static str,
}

const LIGHT: ThemeColors = ThemeColors {
    ink: "#1e2633",
    b: "#2563c9",
    r: "#d9372b",
    g: "#2f9e44",
    o: "#e08a00",
    paper: "#ffffff",
};
const DARK: ThemeColors = ThemeColors {
    ink: "#e3e8ef",
    b: "#6ea8ff",
    r: "#ff6b5e",
    g: "#5ccf8b",
    o: "#f0b45a",
    paper: "#1b1f25",
};

fn colorize(svg: &str, c: &ThemeColors) -> String {
    svg.replace("currentColor", c.ink)
        .replace("IC_PAPER", c.paper)
        .replace("IC_B", c.b)
        .replace("IC_R", c.r)
        .replace("IC_G", c.g)
        .replace("IC_O", c.o)
}

type Cache = HashMap<(String, bool), &'static [u8]>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(|| {
        let mut m = HashMap::new();
        for (name, svg) in sources() {
            for (dark, colors) in [(false, &LIGHT), (true, &DARK)] {
                let bytes: &'static [u8] = Box::leak(colorize(&svg, colors).into_bytes().into_boxed_slice());
                m.insert((name.to_string(), dark), bytes);
            }
        }
        for (name, svg) in thumbs() {
            let bytes: &'static [u8] = Box::leak(svg.into_bytes().into_boxed_slice());
            m.insert((format!("thumb-{}", name), false), bytes);
        }
        m
    })
}

/// 功能区图标。名称不存在时退回“帮助”图标。
pub fn icon(name: &str, dark: bool) -> egui::Image<'static> {
    let key = (name.to_string(), dark);
    let (name, bytes) = match cache().get(&key) {
        Some(b) => (name, *b),
        None => ("help", *cache().get(&("help".to_string(), dark)).unwrap()),
    };
    egui::Image::from_bytes(
        format!(
            "bytes://scicanvas/icon/{}/{}.svg",
            if dark { "dark" } else { "light" },
            name
        ),
        bytes,
    )
}

/// 图类型缩略图，名称为命令 id 中 plot. 之后的部分（line、stacked_bar……）。
pub fn thumb(kind: &str) -> egui::Image<'static> {
    let bytes = *cache()
        .get(&(format!("thumb-{}", kind), false))
        .unwrap_or_else(|| cache().get(&("thumb-line".to_string(), false)).unwrap());
    egui::Image::from_bytes(format!("bytes://scicanvas/thumb/{}.svg", kind), bytes)
}
