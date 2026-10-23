use super::scientific_text::format_selection;
#[test]
fn selected_unicode_formats_toggle_nest_clear_and_insert_symbols() {
    let mut text = "电压CO2".to_string();
    let (a, b) = format_selection(&mut text, 4..5, "_{", true);
    assert_eq!(text, "电压CO_{2}");
    assert_eq!(&text.chars().skip(a).take(b - a).collect::<String>(), "2");
    let (a, b) = format_selection(&mut text, a..b, "\\b{", true);
    let (a, b) = format_selection(&mut text, a..b, "\\i{", true);
    assert_eq!(text, r"电压CO_{\b{\i{2}}}");
    format_selection(&mut text, a..b, "", true);
    assert_eq!(text, "电压CO2");
    let (a, b) = format_selection(&mut text, 4..5, "^{", true);
    format_selection(&mut text, a..b, "^{", true);
    assert_eq!(text, "电压CO2");
    let (a, b) = format_selection(&mut text, 0..2, "α", false);
    assert_eq!(text, "αCO2");
    assert_eq!((a, b), (1, 1));
    let (a, b) = format_selection(&mut text, 1..1, "\\i{", true);
    assert_eq!(a, b);
    assert_eq!(text, r"α\i{}CO2");
}
