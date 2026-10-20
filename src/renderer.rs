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

    // Premultiplied Porter-Duff Over alpha blend for layered window transparency
    pub fn blend_pixel_premul(
        &mut self,
        x: i32,
        y: i32,
        src_a: f32,
        src_pr: f32,
        src_pg: f32,
        src_pb: f32,
    ) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = y as usize * self.width + x as usize;
            let dst = self.data[idx];
            let dst_a = ((dst >> 24) & 0xFF) as f32 / 255.0;
            let dst_pr = ((dst >> 16) & 0xFF) as f32;
            let dst_pg = ((dst >> 8) & 0xFF) as f32;
            let dst_pb = (dst & 0xFF) as f32;

            let inv_src_a = 1.0 - src_a;
            let out_a = src_a + dst_a * inv_src_a;
            let out_pr = src_pr + dst_pr * inv_src_a;
            let out_pg = src_pg + dst_pg * inv_src_a;
            let out_pb = src_pb + dst_pb * inv_src_a;

            let a_u32 = (out_a * 255.0).round().clamp(0.0, 255.0) as u32;
            let r_u32 = out_pr.round().clamp(0.0, 255.0) as u32;
            let g_u32 = out_pg.round().clamp(0.0, 255.0) as u32;
            let b_u32 = out_pb.round().clamp(0.0, 255.0) as u32;

            self.data[idx] = (a_u32 << 24) | (r_u32 << 16) | (g_u32 << 8) | b_u32;
        }
    }

    // Draws filled rounded rectangle with smooth anti-aliased border and fill
    #[allow(clippy::too_many_arguments)]
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
        let half_w = w as f32 * 0.5;
        let half_h = h as f32 * 0.5;
        let center_x = x as f32 + half_w;
        let center_y = y as f32 + half_h;
        let inner_box_w = (half_w - r as f32).max(0.0);
        let inner_box_h = (half_h - r as f32).max(0.0);
        let bw = border_width as f32;

        let fill_a = ((fill_color >> 24) & 0xFF) as f32 / 255.0;
        let fill_r = ((fill_color >> 16) & 0xFF) as f32;
        let fill_g = ((fill_color >> 8) & 0xFF) as f32;
        let fill_b = (fill_color & 0xFF) as f32;

        let border_a = ((border_color >> 24) & 0xFF) as f32 / 255.0;
        let border_r = ((border_color >> 16) & 0xFF) as f32;
        let border_g = ((border_color >> 8) & 0xFF) as f32;
        let border_b = (border_color & 0xFF) as f32;

        let start_y = (y - 1).max(0);
        let end_y = (y + h + 1).min(self.height as i32);
        let start_x = (x - 1).max(0);
        let end_x = (x + w + 1).min(self.width as i32);

        for py in start_y..end_y {
            for px in start_x..end_x {
                let p_x = px as f32 + 0.5 - center_x;
                let p_y = py as f32 + 0.5 - center_y;

                let qx = p_x.abs() - inner_box_w;
                let qy = p_y.abs() - inner_box_h;

                let ext_x = qx.max(0.0);
                let ext_y = qy.max(0.0);
                let dist = (ext_x * ext_x + ext_y * ext_y).sqrt() + qx.max(qy).min(0.0) - r as f32;

                // Anti-aliased outer coverage over 1-pixel boundary
                let outer_cov = (0.5 - dist).clamp(0.0, 1.0);
                if outer_cov <= 0.0 {
                    continue;
                }

                // Inner coverage for border
                let (fill_cov, border_cov) = if bw > 0.0 {
                    let inner_dist = dist + bw;
                    let inner_cov = (0.5 - inner_dist).clamp(0.0, 1.0);
                    (inner_cov, outer_cov - inner_cov)
                } else {
                    (outer_cov, 0.0)
                };

                let src_a = fill_cov * fill_a + border_cov * border_a;
                if src_a <= 0.0 {
                    continue;
                }

                // Premultiplied RGB for Windows Layered Window blending
                let src_pr = fill_cov * fill_r * fill_a + border_cov * border_r * border_a;
                let src_pg = fill_cov * fill_g * fill_a + border_cov * border_g * border_a;
                let src_pb = fill_cov * fill_b * fill_a + border_cov * border_b * border_a;

                self.blend_pixel_premul(px, py, src_a, src_pr, src_pg, src_pb);
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
            cur_x += 8 * scale + scale; // character width + spacing
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
    let color_outer_bg = 0xFF000000; // Pure Black
    let color_outer_border = 0xFFFFFFFF; // Crisp White
    let color_inner_bg = 0xFF141414; // Deep Charcoal
    let color_inner_border = 0xFF555555; // Muted White border
    let color_text_white = 0xFFFFFFFF;
    let color_text_muted = 0xFF888888;

    // 1. Outer Card: width=266, height=62, r=16, border=2
    buf.draw_rounded_rect(2, 2, 266, 62, 16, color_outer_bg, color_outer_border, 2);

    // 2. Left Pod: Clock Pod: x=10, y=8, width=140, height=50, r=12, border=1
    let pod_x = 10;
    let pod_y = 8;
    let pod_w = 140;
    let pod_h = 50;
    buf.draw_rounded_rect(
        pod_x,
        pod_y,
        pod_w,
        pod_h,
        12,
        color_inner_bg,
        color_inner_border,
        1,
    );

    let time_len = time_str.len() as i32;
    let time_w = (time_len * 18 - 2).max(0);
    let cursor_w = 16;
    let cursor_gap = 4;
    let ampm_gap = 5;
    let ampm_w = if ampm_str.is_empty() {
        0
    } else {
        ampm_str.len() as i32 * 9 - 1
    };

    let total_clock_w =
        cursor_w + cursor_gap + time_w + if ampm_w > 0 { ampm_gap + ampm_w } else { 0 };
    let start_x = pod_x + ((pod_w - total_clock_w) / 2).max(4);

    let cursor_x = start_x;
    let time_x = cursor_x + cursor_w + cursor_gap;
    let ampm_x = time_x + time_w + ampm_gap;

    // Blinking Cursor ► (scale 2)
    if show_cursor {
        buf.draw_text("►", cursor_x, 25, 2, color_text_white);
    }

    // Time text (e.g. 06:45) - scale 2 (16px height)
    buf.draw_text(time_str, time_x, 25, 2, color_text_white);

    // AM/PM text (e.g. PM) - scale 1 (8px height)
    if !ampm_str.is_empty() {
        buf.draw_text(ampm_str, ampm_x, 28, 1, color_text_muted);
    }

    // 3. Right Pod: Session (Balanced and centered in the right half)
    let right_start = pod_x + pod_w;
    let right_end = 268;
    let right_center = right_start + (right_end - right_start) / 2;

    let session_title = "SESSION";
    let title_w = session_title.len() as i32 * 9 - 1;
    let title_x = right_center - title_w / 2;
    buf.draw_text(session_title, title_x, 20, 1, color_text_muted);

    let val_w = session_str.len() as i32 * 9 - 1;
    let val_x = right_center - val_w / 2;
    buf.draw_text(session_str, val_x, 34, 1, color_text_white);

    buf
}
