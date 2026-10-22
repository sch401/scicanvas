//! Shared plain scientific text metrics, font fallback and baseline placement.
//! ponytail: Latin/CJK/scientific plain runs use advances and kerning; complex-script shaping belongs with the later rich-text implementation.
//! Font sizes are CSS/page pixels per em; egui's height-based sizes are converted explicitly.
use ab_glyph::{Font, FontArc, FontVec};
use std::sync::OnceLock;

static FONTS: OnceLock<Fonts> = OnceLock::new();

pub struct Face {
    font: FontArc,
    pub index: u32,
    pub family: String,
    pub weight: u16,
}
impl Face {
    fn parse(bytes: Vec<u8>, index: u32) -> Option<Self> {
        let face = ttf_parser::Face::parse(&bytes, index).ok()?;
        let family = [16, 1].into_iter().find_map(|kind| {
            face.names()
                .into_iter()
                .filter(|n| n.name_id == kind)
                .filter_map(|n| n.to_string().map(|s| (n.language_id, s)))
                .min_by_key(|(language, _)| if *language == 0x409 { 0 } else { 1 })
                .map(|(_, s)| s)
        })?;
        let weight = face.weight().to_number();
        Some(Self {
            font: FontVec::try_from_vec_and_index(bytes, index).ok()?.into(),
            index,
            family,
            weight,
        })
    }
    pub fn data(&self) -> &[u8] {
        self.font.font_data()
    }
    pub fn height_factor(&self) -> f64 {
        f64::from(self.font.height_unscaled() / self.font.units_per_em().unwrap())
    }
    fn metrics(&self, size: f64) -> (f64, f64) {
        let scale = size / f64::from(self.font.units_per_em().unwrap());
        (
            f64::from(self.font.ascent_unscaled()) * scale,
            -f64::from(self.font.descent_unscaled()) * scale,
        )
    }
}

pub struct Fonts {
    faces: Vec<Face>,
    regular: Vec<usize>,
    bold: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct Run {
    pub font: usize,
    pub text: String,
    pub x: f64,
    pub width: f64,
    pub line: usize,
}
#[derive(Clone, Debug)]
pub struct Layout {
    pub runs: Vec<Run>,
    pub widths: Vec<f64>,
    pub width: f64,
    pub height: f64,
    pub ascent: f64,
    pub line_height: f64,
}
impl Fonts {
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
    /// Embedded licensed fonts also provide deterministic metrics for headless consumers and tests.
    pub fn bundled() -> Self {
        let faces = [
            epaint_default_fonts::UBUNTU_LIGHT,
            epaint_default_fonts::HACK_REGULAR,
            epaint_default_fonts::NOTO_EMOJI_REGULAR,
        ]
        .into_iter()
        .map(|bytes| Face::parse(bytes.to_vec(), 0).expect("valid embedded font"))
        .collect();
        Self {
            faces,
            regular: vec![0, 1, 2],
            bold: vec![0, 1, 2],
        }
    }
    fn load(&mut self, paths: &[&str]) -> Option<usize> {
        paths.iter().find_map(|path| {
            let face = Face::parse(std::fs::read(path).ok()?, 0)?;
            let index = self.faces.len();
            self.faces.push(face);
            Some(index)
        })
    }
    pub fn system() -> Self {
        let mut fonts = Self::bundled();
        let latin = fonts.load(&[
            r"C:\Windows\Fonts\arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        ]);
        let bold = fonts.load(&[
            r"C:\Windows\Fonts\arialbd.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Bold.ttf",
        ]);
        let custom = std::env::var("SCICANVAS_FONT")
            .ok()
            .and_then(|path| fonts.load(&[&path]));
        let cjk = custom.or_else(|| {
            fonts.load(&[
                r"C:\Windows\Fonts\msyh.ttc",
                r"C:\Windows\Fonts\msyh.ttf",
                r"C:\Windows\Fonts\simhei.ttf",
                r"C:\Windows\Fonts\simsun.ttc",
                "/System/Library/Fonts/PingFang.ttc",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            ])
        });
        let cjk_bold = if custom.is_some() {
            custom
        } else {
            fonts
                .load(&[r"C:\Windows\Fonts\msyhbd.ttc", r"C:\Windows\Fonts\msyhbd.ttf"])
                .or(cjk)
        };
        fonts.regular = latin.into_iter().chain(cjk).chain([0, 1, 2]).collect();
        fonts.bold = bold.or(latin).into_iter().chain(cjk_bold).chain([0, 1, 2]).collect();
        fonts
    }
    pub fn primary(&self, bold: bool) -> usize {
        if bold {
            self.bold[0]
        } else {
            self.regular[0]
        }
    }
    pub fn layout(&self, text: &str, size: f64, bold: bool) -> Layout {
        let chain = if bold { &self.bold } else { &self.regular };
        let (mut ascent, mut descent) = self.faces[chain[0]].metrics(size);
        let mut runs: Vec<Run> = Vec::new();
        let mut widths = vec![0.0];
        let mut previous = None;
        for original in text.chars() {
            if original == '\r' {
                continue;
            }
            if original == '\n' {
                widths.push(0.0);
                previous = None;
                continue;
            }
            let original = if original == '\t' { ' ' } else { original };
            let selected = chain
                .iter()
                .copied()
                .find(|i| self.faces[*i].font.glyph_id(original).0 != 0);
            let (index, c) = selected.map(|i| (i, original)).unwrap_or_else(|| {
                for replacement in ['\u{fffd}', '?'] {
                    if let Some(i) = chain
                        .iter()
                        .copied()
                        .find(|i| self.faces[*i].font.glyph_id(replacement).0 != 0)
                    {
                        return (i, replacement);
                    }
                }
                (chain[0], '?')
            });
            let face = &self.faces[index];
            let glyph = face.font.glyph_id(c);
            let scale = size / f64::from(face.font.units_per_em().unwrap());
            let line = widths.len() - 1;
            if let Some((last_font, last_glyph)) = previous {
                if last_font == index {
                    widths[line] += f64::from(face.font.kern_unscaled(last_glyph, glyph)) * scale;
                }
            }
            let advance = f64::from(face.font.h_advance_unscaled(glyph)) * scale;
            if runs.last().is_none_or(|r| r.font != index || r.line != line) {
                runs.push(Run {
                    font: index,
                    text: String::new(),
                    x: widths[line],
                    width: 0.0,
                    line,
                });
            }
            let run = runs.last_mut().unwrap();
            run.text.push(c);
            widths[line] += advance;
            run.width = widths[line] - run.x;
            let (a, d) = face.metrics(size);
            ascent = ascent.max(a);
            descent = descent.max(d);
            previous = Some((index, glyph));
        }
        let line_height = ascent + descent;
        let width = widths.iter().copied().fold(0.0, f64::max);
        Layout {
            height: line_height * widths.len() as f64,
            width,
            widths,
            runs,
            ascent,
            line_height,
        }
    }
    pub fn wrap(&self, text: &str, size: f64, bold: bool, max_width: f64) -> String {
        let mut out = String::new();
        let mut line = String::new();
        for c in text.chars() {
            if c == '\r' {
                continue;
            }
            if c == '\n' {
                out.push_str(line.trim_end());
                out.push('\n');
                line.clear();
                continue;
            }
            let mut next = line.clone();
            next.push(c);
            if !line.is_empty() && self.layout(&next, size, bold).width > max_width {
                if let Some((split, _)) = line.char_indices().rev().find(|(_, c)| c.is_whitespace()) {
                    out.push_str(line[..split].trim_end());
                    out.push('\n');
                    line = line[split..].trim_start().to_string();
                } else {
                    out.push_str(&line);
                    out.push('\n');
                    line.clear();
                }
            }
            if !line.is_empty() || !c.is_whitespace() {
                line.push(c);
            }
        }
        out.push_str(&line);
        out
    }
}

pub fn fonts() -> &'static Fonts {
    FONTS.get_or_init(Fonts::system)
}
/// Configure an immutable font set before any layout (e.g. deterministic server-side export).
pub fn initialize(fonts: Fonts) -> Result<(), Fonts> {
    FONTS.set(fonts)
}
pub fn layout(text: &str, size: f64, bold: bool) -> Layout {
    fonts().layout(text, size, bold)
}

/// Fit a plain label to its allotted axis/page span without changing project settings.
pub fn fit_size(text: &str, size: f64, bold: bool, width: f64) -> f64 {
    let measured = layout(text, size, bold).width;
    if measured > width && measured > 0.0 {
        size * width.max(1.0) / measured
    } else {
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_advances_kerning_lines_and_fallback() {
        let f = Fonts::bundled();
        assert!(f.layout("WWW", 10.0, false).width > 2.0 * f.layout("iii", 10.0, false).width);
        let face = &f.faces[0];
        let units = f64::from(face.font.units_per_em().unwrap());
        let expected = (f64::from(face.font.h_advance_unscaled(face.font.glyph_id('A')))
            + f64::from(
                face.font
                    .kern_unscaled(face.font.glyph_id('A'), face.font.glyph_id('V')),
            )
            + f64::from(face.font.h_advance_unscaled(face.font.glyph_id('V'))))
            * 10.0
            / units;
        assert!((f.layout("AV", 10.0, false).width - expected).abs() < 1e-9);
        let a = f.layout("AV\nμ ± ∞", 10.0, false);
        assert_eq!(a.widths.len(), 2);
        assert!(a.height > 10.0);
        assert!((f.layout("AV", 20.0, false).width - 2.0 * expected).abs() < 1e-9);
        assert!(f
            .layout("中文", 10.0, false)
            .runs
            .iter()
            .all(|r| r.text.chars().all(|c| f.faces[r.font].font.glyph_id(c).0 != 0)));
        assert!(f.wrap("A very long title 中文", 10.0, true, 40.0).contains('\n'));
    }
}
