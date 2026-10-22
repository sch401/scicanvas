//! 字体。
//!
//! - 界面字体用微软雅黑（Microsoft YaHei UI），中英文统一；没有时找其他中文字体，
//!   都没有时退回 egui 自带字体。也可以用环境变量 SCICANVAS_FONT 指定字体文件路径。
//! - 图形文字使用核心 text 模块选择的同一字体与 CSS/em 度量，导出复用相同字节。
//!   英文优先 Arial，中文依次回退到系统 CJK 字体，内置 Ubuntu/Hack/Noto Emoji 补齐可用字符。

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

/// egui already converts em size to the font's internal height convention.
pub fn plot_font(size: f32, face: usize) -> FontId {
    FontId::new(size, FontFamily::Name(format!("plot-{face}").into()))
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

    let env = std::env::var("SCICANVAS_FONT").ok();
    let cjk = env.as_deref().and_then(|p| std::fs::read(p).ok()).or_else(|| load(CJK));
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
    for (i, face) in scicanvas_core::text::fonts().faces().iter().enumerate() {
        let name = format!("plot-{i}");
        let mut data = egui::FontData::from_owned(face.data().to_vec());
        data.index = face.index;
        fonts.font_data.insert(name.clone(), data);
        fonts.families.insert(FontFamily::Name(name.clone().into()), vec![name]);
    }
    ctx.set_fonts(fonts);
}
