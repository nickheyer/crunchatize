use std::fmt;

use crate::anim::{AnimFrame, AnimManager, STATE_STOPPED};
use crate::crt::{asset_key, atoi, stricmp};
use crate::display::Display;
use crate::rect::Rect;
use crate::texture::BitmapManager;

pub const ALIGN_LEFT: i32 = 0;
pub const ALIGN_CENTER: i32 = 1;
pub const ALIGN_RIGHT: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    MissingBamLine,
    MissingWidthLine,
    MissingHeightLine,
    NoGlyphs,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Error::MissingBamLine => "first line is not #BAM",
            Error::MissingWidthLine => "second line is not #WIDTH",
            Error::MissingHeightLine => "third line is not #HEIGHT",
            Error::NoGlyphs => "no glyph lines follow the header",
        };
        f.write_str(what)
    }
}

impl std::error::Error for Error {}

// Text form of a .fnt file before its glyph animation has been loaded
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FntFile {
    pub bam_name: String,
    pub width: i16,
    pub height: i16,
    pub glyph_chars: Vec<u8>,
}

// Yields lines the way the engine's buffer reader does, skipping blank separators
fn lines(data: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut pos = 0usize;
    std::iter::from_fn(move || {
        while pos < data.len() && (data[pos] == b'\n' || data[pos] == b'\r') {
            pos += 1;
        }
        if pos >= data.len() {
            return None;
        }
        let start = pos;
        loop {
            pos += 1;
            if pos >= data.len() {
                break;
            }
            let c = data[pos];
            if c == 0 || c == b'\n' || c == b'\r' {
                break;
            }
        }
        Some(&data[start..pos])
    })
}

// Parses the three header lines and the glyph character list of a .fnt file
pub fn parse_fnt(data: &[u8]) -> Result<FntFile, Error> {
    let mut it = lines(data);
    let bam = it.next().filter(|l| l.starts_with(b"#BAM")).ok_or(Error::MissingBamLine)?;
    let bam_name = String::from_utf8_lossy(&bam[5.min(bam.len())..]).into_owned();
    let w = it.next().filter(|l| l.starts_with(b"#WIDTH")).ok_or(Error::MissingWidthLine)?;
    let width = atoi(&w[6..]) as i16;
    let h = it.next().filter(|l| l.starts_with(b"#HEIGHT")).ok_or(Error::MissingHeightLine)?;
    let height = atoi(&h[7..]) as i16;
    let glyph_chars: Vec<u8> = it.map(|l| l[0]).collect();
    if glyph_chars.is_empty() {
        return Err(Error::NoGlyphs);
    }
    Ok(FntFile { bam_name, width, height, glyph_chars })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Glyph {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
}

#[derive(Clone, Debug)]
pub struct FontRecord {
    pub name: String,
    pub in_use: bool,
    pub height: i16,
    pub width: i16,
    pub anim: u16,
    pub map: [u8; 128],
    pub glyphs: Vec<Glyph>,
}

impl Default for FontRecord {
    fn default() -> Self {
        Self {
            name: String::new(),
            in_use: false,
            height: 0,
            width: 0,
            anim: 0,
            map: [0xff; 128],
            glyphs: Vec::new(),
        }
    }
}

#[derive(Default, Debug)]
pub struct FontManager {
    slots: Vec<FontRecord>,
    count: i16,
}

impl FontManager {
    pub fn find(&self, key: &str) -> Option<u16> {
        self.slots
            .iter()
            .position(|s| s.in_use && stricmp(s.name.as_bytes(), key.as_bytes()) == 0)
            .map(|i| i as u16)
    }

    fn allocate(&mut self) -> u16 {
        if let Some(i) = self.slots.iter().position(|s| !s.in_use) {
            return i as u16;
        }
        self.slots.push(FontRecord::default());
        (self.slots.len() - 1) as u16
    }

    // Lays glyph cells across the font animation's bounding box and registers the font
    pub fn install(&mut self, name: &str, fnt: &FntFile, anim: u16, bbox: [i32; 4]) -> u16 {
        let handle = self.allocate();
        let mut map = [0xffu8; 128];
        let mut glyphs = Vec::with_capacity(fnt.glyph_chars.len());
        let mut cx: i16 = 0;
        let mut cy: i16 = 0;
        for (k, &ch) in fnt.glyph_chars.iter().enumerate() {
            let mut bottom = cy as i32 + fnt.height as i32 - 1;
            if bbox[3] < bottom {
                bottom = bbox[3];
            }
            if ch < 128 {
                map[ch as usize] = k as u8;
            }
            glyphs.push(Glyph {
                left: cx,
                top: cy,
                right: cx - 1 + fnt.width,
                bottom: bottom as i16,
            });
            cx += fnt.width;
            if bbox[2] - 6 <= cx as i32 {
                cx = 0;
                cy += fnt.height;
            }
        }
        for i in 0..26 {
            let upper = (b'A' + i) as usize;
            let lower = (b'a' + i) as usize;
            if map[upper] == 0xff {
                map[upper] = map[lower];
            } else if map[lower] == 0xff {
                map[lower] = map[upper];
            }
        }
        self.slots[handle as usize] = FontRecord {
            name: asset_key(name),
            in_use: true,
            height: fnt.height,
            width: fnt.width,
            anim,
            map,
            glyphs,
        };
        self.count += 1;
        handle
    }

    pub fn release_all(&mut self) {
        for s in &mut self.slots {
            if s.in_use {
                s.in_use = false;
                s.glyphs = Vec::new();
                self.count -= 1;
            }
        }
    }

    pub fn height(&self, font: u32) -> i32 {
        self.slots[font as usize].height as i32
    }

    fn glyph_of(rec: &FontRecord, ch: u8) -> Option<Glyph> {
        if ch >= 128 {
            return None;
        }
        let idx = rec.map[ch as usize] as i8;
        if idx < 0 { None } else { Some(rec.glyphs[idx as usize]) }
    }

    // Measures text width, including the cursor glyph when a cursor is requested
    pub fn text_width(&self, text: &str, font: u32, cursor: u32) -> i32 {
        let rec = &self.slots[font as usize];
        let mut width = 0;
        for &ch in text.as_bytes() {
            if ch == b' ' {
                width += rec.width as i32;
            } else if let Some(g) = Self::glyph_of(rec, ch) {
                width += (g.right as i32 - g.left as i32) - 1;
            }
        }
        if cursor != 0
            && let Some(g) = Self::glyph_of(rec, b'|') {
                width += (g.right as i32 - g.left as i32) - 1;
            }
        width
    }

    // Draws text glyph by glyph with alignment and an optional trailing cursor bar
    #[allow(clippy::too_many_arguments)]
    pub fn print(
        &self,
        text: &str,
        mut x: i32,
        y: i16,
        font: u32,
        align: i32,
        cursor: u32,
        anims: &AnimManager,
        bitmaps: &BitmapManager,
        display: &mut Display,
    ) {
        let rec = &self.slots[font as usize];
        if align == ALIGN_RIGHT {
            x -= self.text_width(text, font, cursor);
        } else if align == ALIGN_CENTER {
            x -= self.text_width(text, font, cursor) / 2;
        }
        let mut af = AnimFrame { frame: 0.0, anim: rec.anim, state: STATE_STOPPED, fps: 0 };
        for &ch in text.as_bytes() {
            if ch == b' ' {
                x += rec.width as i32;
                continue;
            }
            let Some(g) = Self::glyph_of(rec, ch) else { continue };
            let mut clip = Rect::new(g.left as i32, g.top as i32, g.right as i32, g.bottom as i32);
            anims.draw_clip(&mut af, x as i16, y, &mut clip, bitmaps, display);
            x += (g.right as i32 - g.left as i32) - 1;
        }
        if cursor == 1
            && let Some(g) = Self::glyph_of(rec, b'|') {
                let mut clip = Rect::new(g.left as i32, g.top as i32, g.right as i32, g.bottom as i32);
                anims.draw_clip(&mut af, x as i16, y, &mut clip, bitmaps, display);
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_and_glyph_list() {
        let f = parse_fnt(b"#BAM KahunaFont_yellow.bam\r\n#WIDTH 24\r\n#HEIGHT 30\r\nA \r\nB\r\n|\r\n").unwrap();
        assert_eq!(f.bam_name, "KahunaFont_yellow.bam");
        assert_eq!((f.width, f.height), (24, 30));
        assert_eq!(f.glyph_chars, b"AB|".to_vec());
    }

    #[test]
    fn glyph_grid_wraps_near_right_edge() {
        let f = parse_fnt(b"#BAM x.bam\r\n#WIDTH 10\r\n#HEIGHT 10\r\nA\r\nB\r\nC\r\n").unwrap();
        let mut m = FontManager::default();
        let h = m.install("font.fnt", &f, 0, [0, 0, 25, 100]);
        let rec = &m.slots[h as usize];
        assert_eq!((rec.glyphs[1].left, rec.glyphs[1].top), (10, 0));
        assert_eq!((rec.glyphs[2].left, rec.glyphs[2].top), (0, 10));
        assert_eq!(rec.map[b'a' as usize], 0);
        assert_eq!(m.text_width("AB C", h as u32, 0), 8 + 8 + 10 + 8);
    }
}
