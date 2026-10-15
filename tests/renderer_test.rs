#[test]
fn test_clock_formatting_12h_pm() {
    let (time_str, ampm) = time_reminder::renderer::format_clock(14, 30, "12h");
    assert_eq!(time_str, "02:30");
    assert_eq!(ampm, "PM");
}

#[test]
fn test_clock_formatting_12h_am() {
    let (time_str, ampm) = time_reminder::renderer::format_clock(0, 15, "12h");
    assert_eq!(time_str, "12:15");
    assert_eq!(ampm, "AM");
}

#[test]
fn test_clock_formatting_24h() {
    let (time_str, ampm) = time_reminder::renderer::format_clock(14, 30, "24h");
    assert_eq!(time_str, "14:30");
    assert_eq!(ampm, "");
}

#[test]
fn test_font_glyph_retrieval() {
    let glyph_0 = time_reminder::font_8bit::get_glyph('0');
    assert_eq!(glyph_0.len(), 8);
    // '0' has top and bottom solid lines
    assert!(glyph_0[0] != 0);
}
