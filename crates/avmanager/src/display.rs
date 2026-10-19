use crate::debugfont;
use crate::rect::Rect;
use crate::texture::{Texture, rgb_to_565};

pub const WIDTH: u16 = 640;
pub const HEIGHT: u16 = 480;

const DEBUG_TEXT_COLOR: u16 = rgb_to_565([255, 0, 0]);

// Double buffered 16 bit display with the DirectDraw flip and blit semantics of the engine
#[derive(Clone, Debug)]
pub struct Display {
    width: u16,
    height: u16,
    back: Vec<u16>,
    front: Vec<u16>,
}

impl Display {
    pub fn new(width: u16, height: u16) -> Self {
        let n = width as usize * height as usize;
        Self {
            width,
            height,
            back: vec![0; n],
            front: vec![0; n],
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn front(&self) -> &[u16] {
        &self.front
    }

    pub fn back(&self) -> &[u16] {
        &self.back
    }

    pub fn back_mut(&mut self) -> &mut [u16] {
        &mut self.back
    }

    pub fn front_mut(&mut self) -> &mut [u16] {
        &mut self.front
    }

    // Draws a Polyline style segment of the given pen width, leaving the end point unpainted
    pub fn gdi_draw_line(&mut self, x1: i16, y1: i16, x2: i16, y2: i16, width: i16, color: u32) {
        let value = rgb_to_565([color as u8, (color >> 8) as u8, (color >> 16) as u8]);
        let (mut x, mut y) = (x1 as i32, y1 as i32);
        let (ex, ey) = (x2 as i32, y2 as i32);
        let dx = (ex - x).abs();
        let dy = -(ey - y).abs();
        let sx = if x < ex { 1 } else { -1 };
        let sy = if y < ey { 1 } else { -1 };
        let mut err = dx + dy;
        let half = (width.max(1) as i32) / 2;
        let w = width.max(1) as i32;
        while (x, y) != (ex, ey) {
            for oy in 0..w {
                for ox in 0..w {
                    self.plot(x - half + ox, y - half + oy, value);
                }
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn plot(&mut self, x: i32, y: i32, value: u16) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        self.back[(y * self.width as i32 + x) as usize] = value;
    }

    // Exchanges the buffers so the back buffer now holds the frame shown two swaps ago
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.back, &mut self.front);
    }

    // Fills the back buffer using the integer per channel scaling of the original clear
    pub fn clear(&mut self, r: i32, g: i32, b: i32) {
        let value = ((r / 8) << 11 | (b / 8) | (g / 4) << 5) as u16;
        self.back.fill(value);
    }

    // Blits a source rectangle to a destination rectangle with the engine's edge clipping quirks
    pub fn blit(&mut self, tex: &Texture, dest: &Rect, src: &Rect, keyed: bool) -> bool {
        let w = self.width as i32;
        let h = self.height as i32;
        let mut d = *dest;
        let mut s = *src;
        if d.top >= h || d.left >= w {
            return true;
        }
        if h < d.bottom {
            let k = (h - d.bottom) - 1;
            d.bottom += k;
            s.bottom += k;
            if s.bottom < s.top {
                return true;
            }
        }
        if d.top < 0 {
            s.top -= d.top;
            if s.bottom <= s.top {
                return true;
            }
            d.top = 0;
        }
        if d.left < 0 {
            s.left -= d.left;
            if s.right <= s.left {
                return true;
            }
            d.left = 0;
        }
        if w < d.right {
            let k = w - d.right;
            d.right += k;
            s.right += k;
            if s.right < s.left {
                return true;
            }
        }
        self.copy_rect(tex, &d, &s, keyed)
    }

    // Performs the surface copy itself and reports failure for rectangles DirectDraw rejects
    fn copy_rect(&mut self, tex: &Texture, d: &Rect, s: &Rect, keyed: bool) -> bool {
        let w = self.width as i32;
        let h = self.height as i32;
        let tw = tex.width as i32;
        let th = tex.height as i32;
        let dest_ok = d.left >= 0 && d.top >= 0 && d.right <= w && d.bottom <= h;
        let src_ok = s.left >= 0 && s.top >= 0 && s.right <= tw && s.bottom <= th;
        if !dest_ok || !src_ok || d.width() <= 0 || d.height() <= 0 || s.width() <= 0 || s.height() <= 0 {
            return false;
        }
        let dw = d.width();
        let dh = d.height();
        let sw = s.width();
        let sh = s.height();
        for dy in 0..dh {
            let sy = s.top + dy * sh / dh;
            let src_row = &tex.pixels[(sy * tw) as usize..((sy + 1) * tw) as usize];
            let dst_row_start = ((d.top + dy) * w) as usize;
            let dst_row = &mut self.back[dst_row_start..dst_row_start + w as usize];
            for dx in 0..dw {
                let sx = s.left + dx * sw / dw;
                let p = src_row[sx as usize];
                if keyed && tex.key == Some(p) {
                    continue;
                }
                dst_row[(d.left + dx) as usize] = p;
            }
        }
        true
    }

    // Draws red on black debug text with a cell per character at the given position
    pub fn gdi_text(&mut self, x: i16, y: i16, text: &str) {
        let w = self.width as i32;
        let h = self.height as i32;
        for (i, &ch) in text.as_bytes().iter().enumerate() {
            let cx = x as i32 + i as i32 * debugfont::CELL_WIDTH;
            let cy = y as i32;
            for py in 0..debugfont::CELL_HEIGHT {
                for px in 0..debugfont::CELL_WIDTH {
                    let sx = cx + px;
                    let sy = cy + py;
                    if sx < 0 || sy < 0 || sx >= w || sy >= h {
                        continue;
                    }
                    self.back[(sy * w + sx) as usize] = 0;
                }
            }
            let rows = debugfont::glyph(ch);
            for (ry, bits) in rows.iter().enumerate() {
                for rx in 0..debugfont::GLYPH_WIDTH {
                    if bits & (0x10 >> rx) == 0 {
                        continue;
                    }
                    let sx = cx + 1 + rx as i32;
                    let sy = cy + 4 + ry as i32;
                    if sx < 0 || sy < 0 || sx >= w || sy >= h {
                        continue;
                    }
                    self.back[(sy * w + sx) as usize] = DEBUG_TEXT_COLOR;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex(w: u16, h: u16, v: u16) -> Texture {
        Texture {
            width: w,
            height: h,
            hot_x: 0,
            hot_y: 0,
            pixels: vec![v; w as usize * h as usize],
            key: None,
            source: None,
        }
    }

    #[test]
    fn bottom_edge_clip_drops_one_row() {
        let mut d = Display::new(8, 8);
        let t = tex(4, 4, 1);
        assert!(d.blit(&t, &Rect::new(0, 6, 4, 10), &Rect::new(0, 0, 4, 4), false));
        assert_eq!(d.back()[6 * 8], 1);
        assert_eq!(d.back()[7 * 8], 0);
    }

    #[test]
    fn right_edge_clip_keeps_all_columns() {
        let mut d = Display::new(8, 8);
        let t = tex(4, 4, 1);
        assert!(d.blit(&t, &Rect::new(6, 0, 10, 4), &Rect::new(0, 0, 4, 4), false));
        assert_eq!(d.back()[7], 1);
    }

    #[test]
    fn key_pixels_are_skipped() {
        let mut d = Display::new(4, 4);
        let mut t = tex(2, 1, 5);
        t.pixels[1] = 7;
        t.key = Some(7);
        d.clear(0, 0, 0);
        assert!(d.blit(&t, &Rect::new(0, 0, 2, 1), &Rect::new(0, 0, 2, 1), true));
        assert_eq!(&d.back()[..2], &[5, 0]);
    }

    #[test]
    fn clear_uses_integer_scaling() {
        let mut d = Display::new(1, 1);
        d.clear(255, 255, 255);
        assert_eq!(d.back()[0], 0xffff);
        d.clear(8, 4, 8);
        assert_eq!(d.back()[0], (1 << 11) | (1 << 5) | 1);
    }
}
