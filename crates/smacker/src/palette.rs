use crate::Error;

pub(crate) type Palette = [[u8; 3]; 256];

const LEVELS: [u8; 64] = [
    0x00, 0x04, 0x08, 0x0C, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24, 0x28, 0x2C, 0x30, 0x34, 0x38, 0x3C,
    0x41, 0x45, 0x49, 0x4D, 0x51, 0x55, 0x59, 0x5D, 0x61, 0x65, 0x69, 0x6D, 0x71, 0x75, 0x79, 0x7D,
    0x82, 0x86, 0x8A, 0x8E, 0x92, 0x96, 0x9A, 0x9E, 0xA2, 0xA6, 0xAA, 0xAE, 0xB2, 0xB6, 0xBA, 0xBE,
    0xC3, 0xC7, 0xCB, 0xCF, 0xD3, 0xD7, 0xDB, 0xDF, 0xE3, 0xE7, 0xEB, 0xEF, 0xF3, 0xF7, 0xFB, 0xFF,
];

/// Applies the record at the start of the frame and returns its byte length
pub(crate) fn apply(frame: &[u8], palette: &mut Palette) -> Result<usize, Error> {
    let size = usize::from(*frame.first().ok_or(Error::BadPalette)?) * 4;
    let body = frame.get(1..size).ok_or(Error::BadPalette)?;
    let previous = *palette;
    let mut entry = 0;
    let mut position = 0;
    while entry < 256 && position < body.len() {
        let op = body[position];
        position += 1;
        if op & 0x80 != 0 {
            entry += usize::from(op & 0x7F) + 1;
        } else if op & 0x40 != 0 {
            let start = usize::from(*body.get(position).ok_or(Error::BadPalette)?);
            position += 1;
            let count = usize::from(op & 0x3F) + 1;
            let source = previous.get(start..start + count).ok_or(Error::BadPalette)?;
            for &color in source.iter().take(256 - entry) {
                palette[entry] = color;
                entry += 1;
            }
        } else {
            let rest = body.get(position..position + 2).ok_or(Error::BadPalette)?;
            position += 2;
            palette[entry] = [LEVELS[usize::from(op)], LEVELS[usize::from(rest[0] & 0x3F)], LEVELS[usize::from(rest[1] & 0x3F)]];
            entry += 1;
        }
    }
    Ok(size)
}
