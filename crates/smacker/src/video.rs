use crate::Error;
use crate::bits::BitReader;
use crate::header::Version;
use crate::huffman::BigTree;

const RUN_LENGTHS: [u16; 64] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 128, 256, 512, 1024, 2048,
];

/// The four shared 16-bit trees read from the file header
pub(crate) struct Trees {
    mmap: BigTree,
    mclr: BigTree,
    full: BigTree,
    types: BigTree,
}

/// Block-aligned index image whose rows span a whole number of four-pixel blocks
pub(crate) struct Canvas {
    pub(crate) blocks_wide: usize,
    pub(crate) blocks_high: usize,
    pub(crate) pixels: Vec<u8>,
}

#[derive(Clone, Copy)]
enum FullMode {
    Plain,
    Double,
    Half,
}

impl Trees {
    pub(crate) fn read(data: &[u8], sizes: [u32; 4]) -> Result<Self, Error> {
        let mut bits = BitReader::new(data);
        Ok(Self {
            mmap: BigTree::read(&mut bits, sizes[0])?,
            mclr: BigTree::read(&mut bits, sizes[1])?,
            full: BigTree::read(&mut bits, sizes[2])?,
            types: BigTree::read(&mut bits, sizes[3])?,
        })
    }

    fn reset(&mut self) {
        self.mmap.reset();
        self.mclr.reset();
        self.full.reset();
        self.types.reset();
    }
}

impl Canvas {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        let blocks_wide = width.div_ceil(4) as usize;
        let blocks_high = height.div_ceil(4) as usize;
        Self { blocks_wide, blocks_high, pixels: vec![0; blocks_wide * blocks_high * 16] }
    }

    pub(crate) fn stride(&self) -> usize {
        self.blocks_wide * 4
    }

    fn row(&mut self, block: usize, row: usize) -> &mut [u8] {
        let stride = self.stride();
        let start = (block / self.blocks_wide) * 4 * stride + (block % self.blocks_wide) * 4;
        &mut self.pixels[start + row * stride..][..4]
    }
}

/// Paints every block chain of one frame onto the canvas
pub(crate) fn decode_frame(data: &[u8], trees: &mut Trees, canvas: &mut Canvas, version: Version) -> Result<(), Error> {
    trees.reset();
    let bits = &mut BitReader::new(data);
    let total = canvas.blocks_wide * canvas.blocks_high;
    let mut block = 0;
    while block < total {
        let descriptor = trees.types.decode(bits)?;
        let run = usize::from(RUN_LENGTHS[usize::from((descriptor >> 2) & 0x3F)]);
        let end = total.min(block + run);
        match descriptor & 3 {
            0 => {
                for block in block..end {
                    let colors = trees.mclr.decode(bits)?;
                    let map = trees.mmap.decode(bits)?;
                    paint_mono(canvas, block, colors, map);
                }
            }
            1 => {
                let mode = full_mode(bits, version)?;
                for block in block..end {
                    paint_full(canvas, block, &mut trees.full, bits, mode)?;
                }
            }
            2 => {}
            _ => {
                let color = (descriptor >> 8) as u8;
                for block in block..end {
                    for row in 0..4 {
                        canvas.row(block, row).fill(color);
                    }
                }
            }
        }
        block = end;
    }
    Ok(())
}

/// Only SMK4 streams carry the mode bits ahead of each full block chain
fn full_mode(bits: &mut BitReader, version: Version) -> Result<FullMode, Error> {
    if version == Version::Smk2 {
        return Ok(FullMode::Plain);
    }
    if bits.bit()? {
        Ok(FullMode::Double)
    } else if bits.bit()? {
        Ok(FullMode::Half)
    } else {
        Ok(FullMode::Plain)
    }
}

/// Map bits select the high color byte when set and the low byte when clear
fn paint_mono(canvas: &mut Canvas, block: usize, colors: u16, mut map: u16) {
    let [low, high] = colors.to_le_bytes();
    for row in 0..4 {
        for (x, pixel) in canvas.row(block, row).iter_mut().enumerate() {
            *pixel = if (map >> x) & 1 == 1 { high } else { low };
        }
        map >>= 4;
    }
}

fn paint_full(canvas: &mut Canvas, block: usize, full: &mut BigTree, bits: &mut BitReader, mode: FullMode) -> Result<(), Error> {
    match mode {
        FullMode::Plain => {
            for row in 0..4 {
                let right = full.decode(bits)?.to_le_bytes();
                let left = full.decode(bits)?.to_le_bytes();
                let line = canvas.row(block, row);
                line[..2].copy_from_slice(&left);
                line[2..].copy_from_slice(&right);
            }
        }
        FullMode::Double => {
            for half in 0..2 {
                let [left, right] = full.decode(bits)?.to_le_bytes();
                for row in 0..2 {
                    canvas.row(block, half * 2 + row).copy_from_slice(&[left, left, right, right]);
                }
            }
        }
        FullMode::Half => {
            for half in 0..2 {
                let right = full.decode(bits)?.to_le_bytes();
                let left = full.decode(bits)?.to_le_bytes();
                for row in 0..2 {
                    let line = canvas.row(block, half * 2 + row);
                    line[..2].copy_from_slice(&left);
                    line[2..].copy_from_slice(&right);
                }
            }
        }
    }
    Ok(())
}
