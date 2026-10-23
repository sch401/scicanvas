//! Shared scientific text metrics, font fallback and baseline placement.
//! ponytail: Latin/CJK/scientific plain runs use advances and kerning; complex-script shaping belongs with the later rich-text implementation.
//! Font sizes are CSS/page pixels per em, also used directly by egui.
use ab_glyph::{Font, FontArc, FontVec};
use std::sync::OnceLock;

static FONTS: OnceLock<Fonts> = OnceLock::new();

pub struct Face {
    font: FontArc,
    pub index: u32,
    pub family: String,
    pub weight: u16,
    pub italic: bool,
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
        let italic = face.is_italic();
        Some(Self {
            font: FontVec::try_from_vec_and_index(bytes, index).ok()?.into(),
            index,
            family,
            weight,
            italic,
        })
    }
    pub fn data(&self) -> &[u8] {
        self.font.font_data()
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
    italic: Vec<usize>,
    bold_italic: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct Run {
    pub font: usize,
    pub text: String,
    pub x: f64,
    pub width: f64,
    pub line: usize,
    pub size: f64,
    /// Baseline displacement, positive downwards, in page units.
    pub dy: f64,
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

/// Symbols are stored as Unicode, so copy, save and export retain the same characters.
pub const GREEK: &[(&str, &str)] = &[
    ("alpha", "α"),
    ("beta", "β"),
    ("gamma", "γ"),
    ("delta", "δ"),
    ("epsilon", "ε"),
    ("zeta", "ζ"),
    ("eta", "η"),
    ("theta", "θ"),
    ("iota", "ι"),
    ("kappa", "κ"),
    ("lambda", "λ"),
    ("mu", "μ"),
    ("nu", "ν"),
    ("xi", "ξ"),
    ("omicron", "ο"),
    ("pi", "π"),
    ("rho", "ρ"),
    ("sigma", "σ"),
    ("tau", "τ"),
    ("upsilon", "υ"),
    ("phi", "φ"),
    ("chi", "χ"),
    ("psi", "ψ"),
    ("omega", "ω"),
    ("Alpha", "Α"),
    ("Beta", "Β"),
    ("Gamma", "Γ"),
    ("Delta", "Δ"),
    ("Epsilon", "Ε"),
    ("Zeta", "Ζ"),
    ("Eta", "Η"),
    ("Theta", "Θ"),
    ("Iota", "Ι"),
    ("Kappa", "Κ"),
    ("Lambda", "Λ"),
    ("Mu", "Μ"),
    ("Nu", "Ν"),
    ("Xi", "Ξ"),
    ("Omicron", "Ο"),
    ("Pi", "Π"),
    ("Rho", "Ρ"),
    ("Sigma", "Σ"),
    ("Tau", "Τ"),
    ("Upsilon", "Υ"),
    ("Phi", "Φ"),
    ("Chi", "Χ"),
    ("Psi", "Ψ"),
    ("Omega", "Ω"),
];

#[derive(Clone, Default, PartialEq, Eq)]
struct Style {
    bold: bool,
    italic: bool,
    scripts: Vec<bool>,
}
struct Span {
    text: String,
    style: Style,
}
fn append(spans: &mut Vec<Span>, text: &str, style: &Style) {
    if let Some(last) = spans.last_mut().filter(|last| last.style == *style) {
        last.text.push_str(text);
    } else {
        spans.push(Span {
            text: text.into(),
            style: style.clone(),
        });
    }
}

// Braced commands keep nested formats unambiguous: ^{…}, _{…}, \b{…}, \i{…}.
// Only opted-in layers parse them. Unfinished/unknown commands remain literal while editing.
fn parse(text: &str, bold: bool, rich: bool) -> Vec<Span> {
    fn body(text: &str, style: &Style, depth: usize, out: &mut Vec<Span>) {
        let mut rest = text;
        while !rest.is_empty() {
            let command = ["^{", "_{", "\\b{", "\\i{"].into_iter().find(|p| rest.starts_with(p));
            if let Some(command) = command.filter(|_| depth < 32) {
                let content = &rest[command.len()..];
                let (mut braces, mut escaped, mut end) = (1, false, None);
                for (i, c) in content.char_indices() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    match c {
                        '\\' => escaped = true,
                        '{' => braces += 1,
                        '}' => {
                            braces -= 1;
                            if braces == 0 {
                                end = Some(i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(end) = end {
                    let mut nested = style.clone();
                    match command {
                        "^{" => nested.scripts.push(true),
                        "_{" => nested.scripts.push(false),
                        "\\b{" => nested.bold = !nested.bold,
                        "\\i{" => nested.italic = !nested.italic,
                        _ => unreachable!(),
                    }
                    body(&content[..end], &nested, depth + 1, out);
                    rest = &content[end + 1..];
                    continue;
                }
            }
            if let Some(next) = rest.strip_prefix('\\') {
                if let Some(c) = next.chars().next().filter(|c| "\\^_{}".contains(*c)) {
                    append(out, &c.to_string(), style);
                    rest = &next[c.len_utf8()..];
                    continue;
                }
                let end = next.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(next.len());
                if let Some((_, symbol)) = GREEK.iter().find(|(name, _)| *name == &next[..end]) {
                    append(out, symbol, style);
                    rest = &next[end..];
                    continue;
                }
            }
            let c = rest.chars().next().unwrap();
            append(out, &c.to_string(), style);
            rest = &rest[c.len_utf8()..];
        }
    }
    let style = Style {
        bold,
        ..Default::default()
    };
    let mut spans = Vec::new();
    if rich {
        body(text, &style, 0, &mut spans);
    } else {
        append(&mut spans, text, &style);
    }
    spans
}

pub fn plain_text(text: &str, rich: bool) -> String {
    parse(text, false, rich).into_iter().map(|s| s.text).collect()
}

fn spans_from_chars(chars: &[(char, Style)]) -> Vec<Span> {
    let mut out = Vec::new();
    for (c, style) in chars {
        append(&mut out, &c.to_string(), style);
    }
    out
}
fn encode(chars: &[(char, Style)], bold: bool, rich: bool) -> String {
    if !rich {
        return chars.iter().map(|(c, _)| *c).collect();
    }
    let mut out = String::new();
    for span in spans_from_chars(chars) {
        let mut s = String::new();
        for c in span.text.chars() {
            if "\\^_{}".contains(c) {
                s.push('\\');
            }
            s.push(c);
        }
        for sup in span.style.scripts.iter().rev() {
            s = format!("{}{{{s}}}", if *sup { '^' } else { '_' });
        }
        if span.style.italic {
            s = format!("\\i{{{s}}}");
        }
        if span.style.bold != bold {
            s = format!("\\b{{{s}}}");
        }
        out.push_str(&s);
    }
    out
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
            italic: vec![0, 1, 2],
            bold_italic: vec![0, 1, 2],
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
        let italic = fonts.load(&[r"C:\Windows\Fonts\ariali.ttf"]);
        let bold_italic = fonts.load(&[r"C:\Windows\Fonts\arialbi.ttf"]);
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
        fonts.italic = italic.or(latin).into_iter().chain(cjk).chain([0, 1, 2]).collect();
        fonts.bold_italic = bold_italic
            .or(bold)
            .or(italic)
            .or(latin)
            .into_iter()
            .chain(cjk_bold)
            .chain([0, 1, 2])
            .collect();
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
        self.layout_formatted(text, size, bold, false)
    }
    pub fn layout_formatted(&self, text: &str, size: f64, bold: bool, rich: bool) -> Layout {
        self.layout_spans(&parse(text, bold, rich), size, bold)
    }
    fn layout_spans(&self, spans: &[Span], size: f64, bold: bool) -> Layout {
        let chain = if bold { &self.bold } else { &self.regular };
        let (mut ascent, mut descent) = self.faces[chain[0]].metrics(size);
        let mut runs: Vec<Run> = Vec::new();
        let mut widths = vec![0.0];
        let mut previous = None;
        for span in spans {
            let chain = match (span.style.bold, span.style.italic) {
                (false, false) => &self.regular,
                (true, false) => &self.bold,
                (false, true) => &self.italic,
                (true, true) => &self.bold_italic,
            };
            let (mut em, mut dy) = (size, 0.0);
            for sup in &span.style.scripts {
                dy += em * if *sup { -0.4 } else { 0.25 };
                em *= 0.7;
            }
            for original in span.text.chars() {
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
                let scale = em / f64::from(face.font.units_per_em().unwrap());
                let line = widths.len() - 1;
                if let Some((last_font, last_glyph, last_em, last_dy)) = previous {
                    if last_font == index && last_em == em && last_dy == dy {
                        widths[line] += f64::from(face.font.kern_unscaled(last_glyph, glyph)) * scale;
                    }
                }
                let advance = f64::from(face.font.h_advance_unscaled(glyph)) * scale;
                if runs
                    .last()
                    .is_none_or(|r| r.font != index || r.line != line || r.size != em || r.dy != dy)
                {
                    runs.push(Run {
                        font: index,
                        text: String::new(),
                        x: widths[line],
                        width: 0.0,
                        line,
                        size: em,
                        dy,
                    });
                }
                let run = runs.last_mut().unwrap();
                run.text.push(c);
                widths[line] += advance;
                run.width = widths[line] - run.x;
                let (a, d) = face.metrics(em);
                ascent = ascent.max(a - dy);
                descent = descent.max(d + dy);
                previous = Some((index, glyph, em, dy));
            }
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
        self.wrap_formatted(text, size, bold, max_width, false)
    }
    pub fn wrap_formatted(&self, text: &str, size: f64, bold: bool, max_width: f64, rich: bool) -> String {
        let mut out = String::new();
        let mut line: Vec<(char, Style)> = Vec::new();
        let chars = parse(text, bold, rich)
            .into_iter()
            .flat_map(|span| span.text.chars().map(|c| (c, span.style.clone())).collect::<Vec<_>>());
        for (c, style) in chars {
            if c == '\r' {
                continue;
            }
            if c == '\n' {
                while line.last().is_some_and(|(c, _)| c.is_whitespace()) {
                    line.pop();
                }
                out.push_str(&encode(&line, bold, rich));
                out.push('\n');
                line.clear();
                continue;
            }
            let mut next = line.clone();
            next.push((c, style.clone()));
            while !line.is_empty() && self.layout_spans(&spans_from_chars(&next), size, bold).width > max_width {
                if let Some(split) = line.iter().rposition(|(c, _)| c.is_whitespace()) {
                    let mut prefix = line[..split].to_vec();
                    while prefix.last().is_some_and(|(c, _)| c.is_whitespace()) {
                        prefix.pop();
                    }
                    out.push_str(&encode(&prefix, bold, rich));
                    out.push('\n');
                    line = line[split..]
                        .iter()
                        .skip_while(|(c, _)| c.is_whitespace())
                        .cloned()
                        .collect();
                } else {
                    out.push_str(&encode(&line, bold, rich));
                    out.push('\n');
                    line.clear();
                }
                next = line.clone();
                next.push((c, style.clone()));
            }
            if !line.is_empty() || !c.is_whitespace() {
                line.push((c, style));
            }
        }
        out.push_str(&encode(&line, bold, rich));
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
pub fn layout_formatted(text: &str, size: f64, bold: bool, rich: bool) -> Layout {
    fonts().layout_formatted(text, size, bold, rich)
}

/// Fit a plain label to its allotted axis/page span without changing project settings.
pub fn fit_size(text: &str, size: f64, bold: bool, width: f64) -> f64 {
    fit_size_formatted(text, size, bold, width, false)
}
pub fn fit_size_formatted(text: &str, size: f64, bold: bool, width: f64, rich: bool) -> f64 {
    let measured = layout_formatted(text, size, bold, rich).width;
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
    fn scientific_spans_preserve_plain_input_nesting_and_script_metrics() {
        let f = Fonts::bundled();
        let source = r"CO_{2} R^{2} \b{bold \i{mix}} \alpha 中文";
        assert_eq!(plain_text(source, false), source);
        assert_eq!(plain_text(source, true), "CO2 R2 bold mix α 中文");
        assert_eq!(
            plain_text(r"x\_{literal} \b{unfinished \unknown", true),
            r"x_{literal} \b{unfinished \unknown"
        );
        let spans = parse(r"a\b{b\i{c}}d", false, true);
        assert_eq!(
            spans
                .iter()
                .map(|s| (s.text.as_str(), s.style.bold, s.style.italic))
                .collect::<Vec<_>>(),
            [
                ("a", false, false),
                ("b", true, false),
                ("c", true, true),
                ("d", false, false)
            ]
        );
        assert!(!parse(r"\b{normal}", true, true)[0].style.bold);
        let rich = f.layout_formatted(source, 20.0, false, true);
        assert!(rich.runs.iter().any(|r| r.dy > 0.0 && (r.size - 14.0).abs() < 1e-9));
        assert!(rich.runs.iter().any(|r| r.dy < 0.0 && (r.size - 14.0).abs() < 1e-9));
        let doubled = f.layout_formatted(source, 40.0, false, true);
        assert!((doubled.width - rich.width * 2.0).abs() < 1e-8);
        assert!((doubled.height - rich.height * 2.0).abs() < 1e-8);
        for r in &rich.runs {
            let (a, d) = f.faces[r.font].metrics(r.size);
            assert!(rich.ascent >= a - r.dy && rich.height - rich.ascent >= d + r.dy - 1e-8);
        }
        let plain = f.layout(source, 20.0, false);
        assert!(plain.width > rich.width);
        assert!(plain.runs.iter().all(|r| r.size == 20.0 && r.dy == 0.0));
        let nested = format!("{}x{}", "^{".repeat(500), "}".repeat(500));
        assert!(f.layout_formatted(&nested, 20.0, false, true).width.is_finite());
    }

    #[test]
    fn rich_wrap_keeps_formats_balanced_and_measures_visible_runs() {
        let f = Fonts::bundled();
        let source = r"\b{WWW \i{CO_{2} 中文} R^{2} \mu}";
        for bold in [false, true] {
            let wrapped = f.wrap_formatted(source, 12.0, bold, 42.0, true);
            assert!(wrapped.contains('\n'));
            assert_eq!(
                plain_text(&wrapped, true).split_whitespace().collect::<String>(),
                plain_text(source, true).split_whitespace().collect::<String>()
            );
            let original = parse(source, bold, true);
            let after = parse(&wrapped, bold, true);
            let chars = |spans: Vec<Span>| {
                spans
                    .into_iter()
                    .flat_map(|s| {
                        s.text
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .map(|c| (c, s.style.clone()))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            };
            assert!(chars(original) == chars(after));
            assert!(f.layout_formatted(&wrapped, 12.0, bold, true).width <= 42.0 + 1e-8);
        }
    }

    #[test]
    fn windows_styles_use_real_font_faces_when_available() {
        let f = Fonts::system();
        for (source, path, bold, italic) in [
            (r"\b{A}", r"C:\Windows\Fonts\arialbd.ttf", true, false),
            (r"\i{A}", r"C:\Windows\Fonts\ariali.ttf", false, true),
            (r"\b{\i{A}}", r"C:\Windows\Fonts\arialbi.ttf", true, true),
        ] {
            if std::path::Path::new(path).exists() {
                let l = f.layout_formatted(source, 12.0, false, true);
                let face = &f.faces[l.runs[0].font];
                assert_eq!(face.italic, italic);
                assert_eq!(face.weight >= 600, bold);
            }
        }
    }
    #[test]
    fn wrapping_rechecks_a_wide_glyph_after_a_word_break() {
        let fonts = Fonts::bundled();
        let width = fonts.layout("i WW", 10.0, false).width + 0.1;
        assert!(fonts.layout("WWW", 10.0, false).width > width);
        let wrapped = fonts.wrap("i WWW", 10.0, false, width);
        for line in wrapped.lines() {
            assert!(fonts.layout(line, 10.0, false).width <= width, "overflow: {wrapped:?}");
        }
        assert_eq!(wrapped.replace([' ', '\n'], ""), "iWWW");
    }

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
