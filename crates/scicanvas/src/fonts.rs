//! 字体。
//!
//! - 界面字体用微软雅黑（Microsoft YaHei UI），中英文统一；没有时找其他中文字体，
//!   都没有时退回 egui 自带字体。也可以用环境变量 SCICANVAS_FONT 指定字体文件路径。
//! - 图形里的文字用 Arial（SCI 期刊插图常用），找不到时用相近的 Liberation Sans / Helvetica，
//!   再找不到就用界面字体。中文同样落到中文字体上。

use eframe::egui::{self, FontFamily, FontId};

const CJK: &[&str] = &[
    // Windows
    r"C:\Windows\Fonts\msyh.ttc",
    r"C:\Windows\Fonts\msyh.ttf",
    r"C:\Windows\Fonts\simhei.ttf",
    r"C:\Windows\Fonts\simsun.ttc",
    // macOS
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Medium.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/Library/Fonts/Arial Unicode.ttf",
    // Linux
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
];

const CJK_BOLD: &[&str] = &[r"C:\Windows\Fonts\msyhbd.ttc", r"C:\Windows\Fonts\msyhbd.ttf"];

const ARIAL: &[&str] = &[
    r"C:\Windows\Fonts\arial.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/Library/Fonts/Arial.ttf",
    "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
];

const ARIAL_BOLD: &[&str] = &[
    r"C:\Windows\Fonts\arialbd.ttf",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/Library/Fonts/Arial Bold.ttf",
    "/usr/share/fonts/truetype/msttcorefonts/Arial_Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationSans-Bold.ttf",
];

/// 图形文字的字体族。
pub fn plot_font(size: f32, bold: bool) -> FontId {
    FontId::new(size, FontFamily::Name(if bold { "plot-bold" } else { "plot" }.into()))
}

fn load(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| std::fs::read(p).ok())
}

/// 界面字体：微软雅黑字体集里的第 2 个字体是行距更紧凑的 “Microsoft YaHei UI”。
fn ui_font() -> Option<egui::FontData> {
    for p in [r"C:\Windows\Fonts\msyh.ttc", r"C:\Windows\Fonts\msyh.ttf"] {
        if let Ok(bytes) = std::fs::read(p) {
            let mut data = egui::FontData::from_owned(bytes);
            if p.ends_with(".ttc") {
                data.index = 1;
            }
            return Some(data);
        }
    }
    None
}

pub fn setup(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let ui_fonts = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let env = std::env::var("SCICANVAS_FONT").ok();
    let cjk = env.as_deref().and_then(|p| std::fs::read(p).ok()).or_else(|| load(CJK));
    let has_cjk = cjk.is_some();
    if let Some(bytes) = cjk {
        fonts
            .font_data
            .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
        // 界面中英文都用微软雅黑（放在最前）；指定了 SCICANVAS_FONT 时用指定的字体。
        // egui 自带字体留在后面，补齐雅黑里没有的符号。
        let ui = match (env.is_some(), ui_font()) {
            (false, Some(data)) => {
                fonts.font_data.insert("ui".to_owned(), data);
                "ui"
            }
            _ => "cjk",
        };
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().insert(0, ui.to_owned());
        }
    } else {
        eprintln!("SciCanvas: 没有找到中文字体，中文会显示成方框。可以设置环境变量 SCICANVAS_FONT=字体文件路径");
    }
    let has_cjk_bold = match load(CJK_BOLD) {
        Some(bytes) => {
            fonts
                .font_data
                .insert("cjk-bold".to_owned(), egui::FontData::from_owned(bytes));
            true
        }
        None => false,
    };

    for (family, latin, cjk_name) in [
        ("plot", load(ARIAL), if has_cjk { Some("cjk") } else { None }),
        (
            "plot-bold",
            load(ARIAL_BOLD).or_else(|| load(ARIAL)),
            if has_cjk_bold {
                Some("cjk-bold")
            } else if has_cjk {
                Some("cjk")
            } else {
                None
            },
        ),
    ] {
        let mut list = Vec::new();
        if let Some(bytes) = latin {
            fonts
                .font_data
                .insert(family.to_owned(), egui::FontData::from_owned(bytes));
            list.push(family.to_owned());
        }
        list.extend(ui_fonts.iter().cloned());
        if let Some(c) = cjk_name {
            list.push(c.to_owned());
        }
        fonts.families.insert(FontFamily::Name(family.into()), list);
    }
    ctx.set_fonts(fonts);
}
