use crate::Error;

/// Byte length of the fixed file header
pub(crate) const SIZE: usize = 104;

/// Container revision taken from the signature
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    Smk2,
    Smk4,
}

/// Playback hints from the header flag word
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub ring_frame: bool,
    pub y_interlace: bool,
    pub y_double: bool,
}

/// Format of one of the seven audio tracks
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioInfo {
    pub present: bool,
    pub compressed: bool,
    pub bits16: bool,
    pub stereo: bool,
    pub rate: u32,
}

/// Stream description from the fixed file header
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: Version,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub frame_rate_micros: u32,
    pub flags: Flags,
    pub audio: [AudioInfo; 7],
}

pub(crate) struct Parsed {
    pub(crate) header: Header,
    pub(crate) tree_sizes: [u32; 4],
    pub(crate) trees_len: usize,
}

pub(crate) fn read_u32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data
        .get(offset..)
        .and_then(|rest| rest.get(..4))
        .ok_or(Error::Truncated)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub(crate) fn parse(data: &[u8]) -> Result<Parsed, Error> {
    let version = match data.get(..4) {
        Some(b"SMK2") => Version::Smk2,
        Some(b"SMK4") => Version::Smk4,
        _ => return Err(Error::BadSignature),
    };
    if data.len() < SIZE {
        return Err(Error::Truncated);
    }
    let field = |index: usize| read_u32(data, 4 * index);
    let width = field(1)?;
    let height = field(2)?;
    if width > 32768 || height > 32768 {
        return Err(Error::Dimensions);
    }
    let flags = field(5)?;
    let mut audio = [AudioInfo::default(); 7];
    for (track, info) in audio.iter_mut().enumerate() {
        *info = audio_info(field(18 + track)?)?;
    }
    Ok(Parsed {
        header: Header {
            version,
            width,
            height,
            frames: field(3)?,
            frame_rate_micros: frame_micros(field(4)? as i32),
            flags: Flags {
                ring_frame: flags & 1 != 0,
                y_interlace: flags & 2 != 0,
                y_double: flags & 4 != 0,
            },
            audio,
        },
        tree_sizes: [field(14)?, field(15)?, field(16)?, field(17)?],
        trees_len: field(13)? as usize,
    })
}

/// Positive values are milliseconds, negative values are hundredths of a millisecond, zero means 10 fps
fn frame_micros(raw: i32) -> u32 {
    match raw {
        0 => 100_000,
        rate if rate > 0 => (rate as u32).saturating_mul(1000),
        rate => rate.unsigned_abs().saturating_mul(10),
    }
}

fn audio_info(word: u32) -> Result<AudioInfo, Error> {
    if word & 0x0C00_0000 != 0 {
        return Err(Error::BinkAudio);
    }
    Ok(AudioInfo {
        compressed: word & 0x8000_0000 != 0,
        present: word & 0x4000_0000 != 0,
        bits16: word & 0x2000_0000 != 0,
        stereo: word & 0x1000_0000 != 0,
        rate: word & 0x00FF_FFFF,
    })
}
