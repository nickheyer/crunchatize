use std::fmt;

pub const MAGIC: u32 = 0x1234_ACDC;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadMagic(u32),
    BadVersion(u32),
    Truncated(usize),
    BadMarker { frame: usize, byte: u8 },
    BadFormat { frame: usize, byte: u8 },
    PaletteTooLarge(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic(m) => write!(f, "bad BAM magic {m:#x}"),
            Error::BadVersion(v) => write!(f, "unsupported BAM version {v}"),
            Error::Truncated(at) => write!(f, "BAM data ends early at byte {at}"),
            Error::BadMarker { frame, byte } => {
                write!(f, "frame {frame} starts with {byte:#x} instead of 'z'")
            }
            Error::BadFormat { frame, byte } => {
                write!(f, "frame {frame} uses unknown pixel format {byte}")
            }
            Error::PaletteTooLarge(n) => write!(f, "palette has {n} entries"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct Bam {
    pub version: u32,
    pub transparent: bool,
    pub color_key: u32,
    pub loop_mode: i16,
    pub fps: u16,
    pub bbox: [i32; 4],
    pub palette: Vec<[u8; 3]>,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub x: i16,
    pub y: i16,
    pub bitmap: Bitmap,
}

#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    pub hot_x: i16,
    pub hot_y: i16,
    pub pixels: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pixels {
    Rgb565(Vec<u16>),
    Indexed(Vec<u8>),
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos + n;
        if end > self.data.len() {
            return Err(Error::Truncated(self.pos));
        }
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn u16le(&mut self) -> Result<u16, Error> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32le(&mut self) -> Result<u32, Error> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u16be(&mut self) -> Result<u16, Error> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
}

// Parses a whole BAM file into frames with their raw pixel payloads
pub fn parse(data: &[u8]) -> Result<Bam, Error> {
    let mut c = Cursor { data, pos: 0 };
    let magic = c.u32le()?;
    let version = c.u32le()?;
    let frame_count = c.u16le()?;
    let transparent = c.u16le()? != 0;
    let color_key = c.u32le()?;
    let loop_mode = c.u16le()? as i16;
    let fps = c.u16le()?;
    let bbox = [
        c.u32le()? as i32,
        c.u32le()? as i32,
        c.u32le()? as i32,
        c.u32le()? as i32,
    ];
    if magic != MAGIC {
        return Err(Error::BadMagic(magic));
    }
    if version >= 3 {
        return Err(Error::BadVersion(version));
    }
    let mut palette = Vec::new();
    if version > 1 {
        let n = c.u32le()?;
        if n > 256 {
            return Err(Error::PaletteTooLarge(n));
        }
        for _ in 0..n {
            palette.push([c.u8()?, c.u8()?, c.u8()?]);
        }
    }
    let mut frames = Vec::with_capacity(frame_count as usize);
    for i in 0..frame_count as usize {
        let x = c.u16le()? as i16;
        let y = c.u16le()? as i16;
        let bitmap = parse_bitmap(&mut c, i)?;
        frames.push(Frame { x, y, bitmap });
    }
    Ok(Bam {
        version,
        transparent,
        color_key,
        loop_mode,
        fps,
        bbox,
        palette,
        frames,
    })
}

// Decodes one 'z' bitmap record in raw, run length 16 bit, or run length 8 bit form
fn parse_bitmap(c: &mut Cursor<'_>, frame: usize) -> Result<Bitmap, Error> {
    let marker = c.u8()?;
    if marker != b'z' {
        return Err(Error::BadMarker { frame, byte: marker });
    }
    let format = c.u8()?;
    let width = c.u16be()?;
    let height = c.u16be()?;
    let hot_x = c.u16be()? as i16;
    let hot_y = c.u16be()? as i16;
    let count = (width as i16 as i32 * height as i16 as i32).max(0) as usize;
    let pixels = match format {
        0 => {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                v.push(c.u16be()?);
            }
            Pixels::Rgb565(v)
        }
        1 => {
            let mut v = Vec::with_capacity(count);
            let mut run = 0u32;
            let mut value = 0u16;
            for _ in 0..count {
                if run == 0 {
                    value = c.u16be()?;
                    if value == 0 {
                        run = c.u16be()? as u32;
                    }
                }
                v.push(value);
                run = run.saturating_sub(1);
            }
            Pixels::Rgb565(v)
        }
        2 => {
            let mut v = Vec::with_capacity(count);
            let mut run = 0u32;
            let mut value = 0u8;
            for _ in 0..count {
                if run == 0 {
                    value = c.u8()?;
                    if value == 0 {
                        run = c.u8()? as u32;
                    }
                }
                v.push(value);
                run = run.saturating_sub(1);
            }
            Pixels::Indexed(v)
        }
        other => return Err(Error::BadFormat { frame, byte: other }),
    };
    Ok(Bitmap {
        width,
        height,
        hot_x,
        hot_y,
        pixels,
    })
}

// Returns how many bytes a parse consumes so callers can compare with the file length
pub fn consumed_length(data: &[u8]) -> Result<usize, Error> {
    let mut c = Cursor { data, pos: 0 };
    let bam = parse(data)?;
    c.pos = 8 + 2 + 2 + 4 + 2 + 2 + 16;
    if bam.version > 1 {
        c.pos += 4 + bam.palette.len() * 3;
    }
    for (i, _) in bam.frames.iter().enumerate() {
        c.pos += 4;
        parse_bitmap(&mut c, i)?;
    }
    Ok(c.pos)
}
