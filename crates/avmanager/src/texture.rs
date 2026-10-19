use crate::bam::{Bitmap, Pixels};

// Palette as passed around by the game, a count followed by 256 RGB triplets
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette24 {
    pub count: u32,
    pub rgb: [[u8; 3]; 256],
}

impl Default for Palette24 {
    fn default() -> Self {
        Self { count: 0, rgb: [[0; 3]; 256] }
    }
}

// Converts 8 bit RGB to the 5-6-5 display format by truncation like the DirectDraw path
pub const fn rgb_to_565(rgb: [u8; 3]) -> u16 {
    ((rgb[0] as u16 >> 3) << 11) | ((rgb[1] as u16 >> 2) << 5) | (rgb[2] as u16 >> 3)
}

// Converts a 0x00RRGGBB key color into the surface pixel value it keys on
pub const fn color_key_565(color: u32) -> u16 {
    rgb_to_565([(color >> 16) as u8, (color >> 8) as u8, color as u8])
}

#[derive(Clone, Debug)]
pub struct IndexedSource {
    pub indices: Vec<u8>,
    pub palette: Vec<[u8; 3]>,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub width: u16,
    pub height: u16,
    pub hot_x: i32,
    pub hot_y: i32,
    pub pixels: Vec<u16>,
    pub key: Option<u16>,
    pub source: Option<IndexedSource>,
}

impl Texture {
    // Builds the 16 bit surface for a frame bitmap using the animation palette for indexed data
    pub fn from_bitmap(bitmap: &Bitmap, palette: &[[u8; 3]], retain: bool) -> Texture {
        let (pixels, source) = match &bitmap.pixels {
            Pixels::Rgb565(v) => (v.clone(), None),
            Pixels::Indexed(idx) => {
                let lut = build_lut(palette);
                let px = idx.iter().map(|&i| lut[i as usize]).collect();
                let source = retain.then(|| IndexedSource {
                    indices: idx.clone(),
                    palette: palette.to_vec(),
                });
                (px, source)
            }
        };
        Texture {
            width: bitmap.width,
            height: bitmap.height,
            hot_x: bitmap.hot_x as i32,
            hot_y: bitmap.hot_y as i32,
            pixels,
            key: None,
            source,
        }
    }

    // Reconverts retained indexed pixels through a replacement palette
    pub fn set_palette(&mut self, palette: &Palette24) {
        let source = self
            .source
            .as_mut()
            .expect("palette change on a texture loaded without a retained image");
        source.palette = palette.rgb[..palette.count as usize].to_vec();
        let lut = build_lut(&source.palette);
        self.pixels = source.indices.iter().map(|&i| lut[i as usize]).collect();
    }

    // Reads back the retained palette of an indexed texture
    pub fn palette(&self) -> Option<Palette24> {
        let source = self.source.as_ref()?;
        let mut out = Palette24 { count: source.palette.len() as u32, ..Default::default() };
        for (dst, src) in out.rgb.iter_mut().zip(&source.palette) {
            *dst = *src;
        }
        Some(out)
    }

    pub fn ram_bytes(&self) -> i32 {
        self.width as i16 as i32 * self.height as i16 as i32 * 2
    }
}

// Expands a palette into the 256 entry index to pixel table the blitter uses
fn build_lut(palette: &[[u8; 3]]) -> [u16; 256] {
    assert_eq!(palette.len(), 256, "indexed textures need a full 256 entry palette");
    let mut lut = [0u16; 256];
    for (dst, &rgb) in lut.iter_mut().zip(palette) {
        *dst = rgb_to_565(rgb);
    }
    lut
}

#[derive(Default, Debug)]
pub struct BitmapManager {
    slots: Vec<Option<Texture>>,
    ram_usage: i32,
    total: i16,
}

impl BitmapManager {
    // Stores a texture in the lowest free slot and returns its handle
    pub fn add(&mut self, texture: Texture) -> u16 {
        self.ram_usage += texture.ram_bytes();
        self.total += 1;
        if let Some(i) = self.slots.iter().position(Option::is_none) {
            self.slots[i] = Some(texture);
            return i as u16;
        }
        self.slots.push(Some(texture));
        (self.slots.len() - 1) as u16
    }

    pub fn get(&self, id: u16) -> &Texture {
        self.slots[id as usize].as_ref().expect("bitmap handle refers to a freed slot")
    }

    pub fn get_mut(&mut self, id: u16) -> &mut Texture {
        self.slots[id as usize].as_mut().expect("bitmap handle refers to a freed slot")
    }

    pub fn flush_all(&mut self) {
        self.slots.clear();
        self.ram_usage = 0;
        self.total = 0;
    }

    pub fn ram_usage(&self) -> i32 {
        self.ram_usage
    }

    pub fn total(&self) -> i16 {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_key_matches_palette_entry() {
        assert_eq!(color_key_565(0x0000_ff00), 0x07e0);
        assert_eq!(rgb_to_565([0, 255, 0]), 0x07e0);
        assert_eq!(rgb_to_565([255, 255, 255]), 0xffff);
    }
}
