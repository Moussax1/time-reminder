use crate::font_8bit::get_glyph;

pub fn format_clock(hour: u32, minute: u32, format: &str) -> (String, String) {
    if format == "24h" {
        (format!("{:02}:{:02}", hour, minute), String::new())
    } else {
        let ampm = if hour >= 12 { "PM" } else { "AM" };
        let h12 = if hour == 0 {
            12
        } else if hour > 12 {
            hour - 12
        } else {
            hour
        };
        (format!("{:02}:{:02}", h12, minute), ampm.to_string())
    }
}

// 32-bit ARGB Pixel Buffer
pub struct PixelBuffer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u32>, // 0xAARRGGBB
}

impl PixelBuffer {
    pub fn new(width: usize, height: usize, fill_color: u32) -> Self {
        Self {
            width,
            height,
            data: vec![fill_color; width * height],
        }
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.data[y as usize * self.width + x as usize] = color;
        }
    }

    // Draws filled rounded rectangle with border
    pub fn draw_rounded_rect(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        fill_color: u32,
        border_color: u32,
        border_width: i32,
    ) {
        for py in y..y + h {
            for px in x..x + w {
                // Determine if inside rounded rectangle
                let dx = if px < x + r {
                    x + r - px
                } else if px >= x + w - r {
                    px - (x + w - r - 1)
                } else {
                    0
                };

                let dy = if py < y + r {
                    y + r - py
                } else if py >= y + h - r {
                    py - (y + h - r - 1)
                } else {
                    0
                };

                let dist_sq = dx * dx + dy * dy;
                let r_sq = r * r;

                if dist_sq <= r_sq {
                    // Check if on border
                    let inner_r = (r - border_width).max(0);
                    let inner_dx = if px < x + border_width + inner_r {
                        x + border_width + inner_r - px
                    } else if px >= x + w - border_width - inner_r {
                        px - (x + w - border_width - inner_r - 1)
                    } else {
                        0
                    };

                    let inner_dy = if py < y + border_width + inner_r {
                        y + border_width + inner_r - py
                    } else if py >= y + h - border_width - inner_r {
                        py - (y + h - border_width - inner_r - 1)
                    } else {
                        0
                    };

                    let inner_dist_sq = inner_dx * inner_dx + inner_dy * inner_dy;
                    let is_inner = px >= x + border_width
                        && px < x + w - border_width
                        && py >= y + border_width
                        && py < y + h - border_width
                        && inner_dist_sq <= inner_r * inner_r;

                    if !is_inner && border_width > 0 {
                        self.set_pixel(px, py, border_color);
                    } else {
                        self.set_pixel(px, py, fill_color);
                    }
                }
            }
        }
    }

    // Draws 8-bit text
    pub fn draw_text(&mut self, text: &str, start_x: i32, start_y: i32, scale: i32, color: u32) {
        let mut cur_x = start_x;
        for ch in text.chars() {
            let glyph = get_glyph(ch.to_ascii_uppercase());
            for (row_idx, &row) in glyph.iter().enumerate() {
                for col_idx in 0..8 {
                    if (row & (0x80 >> col_idx)) != 0 {
                        // Draw scaled pixel
                        for sy in 0..scale {
                            for sx in 0..scale {
                                self.set_pixel(
                                    cur_x + col_idx * scale + sx,
                                    start_y + row_idx as i32 * scale + sy,
                                    color,
                                );
                            }
                        }
                    }
                }
            }
            cur_x += 8 * scale + 1 * scale; // character width + spacing
        }
    }
}

// Renders the exact Split Pod Card
pub fn render_split_pod_card(
    time_str: &str,
    ampm_str: &str,
    session_str: &str,
    show_cursor: bool,
) -> PixelBuffer {
    let width = 270;
    let height = 66;
    let mut buf = PixelBuffer::new(width, height, 0x00000000); // Transparent background

    // Colors
    let color_outer_bg = 0xFF000000;      // Pure Black
    let color_outer_border = 0xFFFFFFFF;  // Crisp White
    let color_inner_bg = 0xFF141414;      // Deep Charcoal
    let color_inner_border = 0xFF555555;  // Muted White border
    let color_text_white = 0xFFFFFFFF;
    let color_text_muted = 0xFF888888;

    // 1. Outer Card: width=266, height=62, r=16, border=2
    buf.draw_rounded_rect(
        2, 2, 266, 62, 16, color_outer_bg, color_outer_border, 2,
    );

    // 2. Left Pod: Clock Pod: x=8, y=8, width=135, height=50, r=12, border=1
    buf.draw_rounded_rect(
        8, 8, 135, 50, 12, color_inner_bg, color_inner_border, 1,
    );

    // Blinking Cursor ►
    if show_cursor {
        buf.draw_text("►", 16, 25, 2, color_text_white);
    }

    // Time text (e.g. 10:45) - scale 2 (16px height)
    buf.draw_text(time_str, 34, 25, 2, color_text_white);

    // AM/PM text (e.g. PM) - scale 1 (8px height)
    if !ampm_str.is_empty() {
        buf.draw_text(ampm_str, 114, 28, 1, color_text_muted);
    }

    // 3. Right Pod: Session
    buf.draw_text("SESSION", 152, 20, 1, color_text_muted);
    buf.draw_text(session_str, 152, 34, 1, color_text_white);

    buf
}
