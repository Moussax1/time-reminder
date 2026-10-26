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

    // Verify 'V' has pointed bottom pixels and no flat U-like bottom (0x3C)
    let glyph_v = time_reminder::font_8bit::get_glyph('V');
    assert_eq!(glyph_v[4], 0x24, "Row 4 should taper diagonally inward");
    assert_eq!(
        glyph_v[5], 0x24,
        "Row 5 should maintain diagonal taper without flat bottom"
    );
    assert_eq!(
        glyph_v[6], 0x18,
        "Row 6 should converge to a centered pointed tip"
    );
    assert_ne!(glyph_v[5], 0x3C, "Row 5 must not have flat bottom bar");

    // Verify static header symbol '■'
    let glyph_sq = time_reminder::font_8bit::get_glyph('■');
    assert_eq!(glyph_sq[1], 0x7E);
    assert_eq!(glyph_sq[6], 0x7E);
}

#[test]
fn test_split_pod_card_dimensions_and_antialiased_corners() {
    let buf = time_reminder::renderer::render_split_pod_card("06:45", "PM", "00H 08M", true);
    assert_eq!(buf.width, 270);
    assert_eq!(buf.height, 66);

    // Verify smooth anti-aliasing on outer card corners:
    // With smooth SDF anti-aliasing, pixels along the curved corner boundary have
    // fractional alpha (between 0 and 255) rather than raw jagged 0 or 255 steps.
    let mut has_fractional_alpha = false;
    for py in 0..16 {
        for px in 0..16 {
            let pixel = buf.data[py * buf.width + px];
            let alpha = (pixel >> 24) & 0xFF;
            if alpha > 0 && alpha < 255 {
                has_fractional_alpha = true;
                break;
            }
        }
        if has_fractional_alpha {
            break;
        }
    }
    assert!(
        has_fractional_alpha,
        "Corner pixels should exhibit sub-pixel anti-aliasing (fractional alpha)"
    );
}

#[test]
fn test_split_pod_card_ampm_spacing() {
    let buf_pm = time_reminder::renderer::render_split_pod_card("06:45", "PM", "00H 08M", false);

    // In 12h mode, content is centered with time ending around x = 122 and AM/PM starting at x >= 127.
    // Verify that the 5-pixel separation gap between time and AM/PM (x = 122..127, y = 28..36)
    // contains only the pod background, proving zero collision or overlap.
    let col_inner_bg = 0xFF141414;
    for y in 28..36 {
        for x in 123..127 {
            let p = buf_pm.data[y * buf_pm.width + x];
            assert_eq!(
                p, col_inner_bg,
                "Pixel at ({}, {}) in the separation gap was not empty background! Found 0x{:08X}",
                x, y, p
            );
        }
    }

    // AM/PM starts at x >= 127 and must have rendered glyph pixels
    let mut ampm_drawn = false;
    for y in 28..36 {
        for x in 127..145 {
            let p = buf_pm.data[y * buf_pm.width + x];
            if p != col_inner_bg && (p >> 24) & 0xFF > 0 {
                ampm_drawn = true;
                break;
            }
        }
    }
    assert!(ampm_drawn, "AM/PM glyphs should be rendered after x >= 127");
}

#[test]
fn test_draw_text_tall_dimensions_and_session_spacing() {
    let buf = time_reminder::renderer::render_split_pod_card("06:45", "PM", "00H 08M", true);

    // 1. Verify "SESSION" title starts at y = 18
    let col_muted = 0xFF888888;
    let mut found_title_row18 = false;
    for x in 150..268 {
        if buf.data[18 * buf.width + x] == col_muted {
            found_title_row18 = true;
            break;
        }
    }
    assert!(found_title_row18, "SESSION title should render at y = 18");

    // 2. Verify increased breathing gap below SESSION: rows 25..33 should have no title or session text
    for y in 26..33 {
        for x in 160..250 {
            let p = buf.data[y * buf.width + x];
            assert_eq!(
                p, 0xFF000000,
                "Row {} in vertical space below SESSION should be empty black background",
                y
            );
        }
    }

    // 3. Verify "00H 08M" starts at y = 34 and reaches y = 43 (10px active height, +3px taller)
    let col_white = 0xFFFFFFFF;
    let mut found_session_row34 = false;
    let mut found_session_row43 = false;
    for x in 150..268 {
        if buf.data[34 * buf.width + x] == col_white {
            found_session_row34 = true;
        }
        if buf.data[43 * buf.width + x] == col_white {
            found_session_row43 = true;
        }
    }
    assert!(
        found_session_row34,
        "Session time should render top row at y = 34"
    );
    assert!(
        found_session_row43,
        "Session time should reach row y = 43 (+3px taller)"
    );
}

#[test]
fn test_export_sample_artifact_image() {
    let buf = time_reminder::renderer::render_split_pod_card("06:45", "PM", "00H 08M", true);
    let path = std::env::temp_dir().join("time_reminder_rendered_pod.bmp");
    use std::io::Write;
    if let Ok(mut file) = std::fs::File::create(path) {
        let width = buf.width as u32;
        let height = buf.height as u32;
        let file_size = 54 + width * height * 4;
        let mut header = [0u8; 54];
        header[0] = b'B';
        header[1] = b'M';
        header[2..6].copy_from_slice(&file_size.to_le_bytes());
        header[10..14].copy_from_slice(&54u32.to_le_bytes());
        header[14..18].copy_from_slice(&40u32.to_le_bytes());
        header[18..22].copy_from_slice(&(width as i32).to_le_bytes());
        header[22..26].copy_from_slice(&(-(height as i32)).to_le_bytes());
        header[26..28].copy_from_slice(&1u16.to_le_bytes());
        header[28..30].copy_from_slice(&32u16.to_le_bytes());
        let _ = file.write_all(&header);
        for &px in &buf.data {
            let b = (px & 0xFF) as u8;
            let g = ((px >> 8) & 0xFF) as u8;
            let r = ((px >> 16) & 0xFF) as u8;
            let a = ((px >> 24) & 0xFF) as u8;
            let _ = file.write_all(&[b, g, r, a]);
        }
    }
}
