//! Decoder for RAD Game Tools Smacker video and audio
#![forbid(unsafe_code)]

mod audio;
mod bits;
mod error;
mod header;
mod huffman;
mod palette;
mod video;

pub use error::Error;
pub use header::{AudioInfo, Flags, Header, Version};

use palette::Palette;
use video::{Canvas, Trees};

/// Smacker stream positioned on the next frame to decode
pub struct Smacker {
    data: Vec<u8>,
    header: Header,
    entries: Vec<FrameEntry>,
    trees: Trees,
    canvas: Canvas,
    cropped: Vec<u8>,
    palette: Palette,
    audio: [Vec<i16>; 7],
    frame_index: usize,
}

/// One decoded frame borrowed from the stream until the next call
pub struct Frame<'a> {
    pub pixels: &'a [u8],
    pub palette: &'a Palette,
    pub palette_changed: bool,
    pub keyframe: bool,
    pub audio: [&'a [i16]; 7],
}

#[derive(Clone, Copy)]
struct FrameEntry {
    offset: usize,
    size: usize,
    types: u8,
    keyframe: bool,
}

fn end_of(start: usize, len: usize, data: &[u8]) -> Result<usize, Error> {
    start
        .checked_add(len)
        .filter(|&end| end <= data.len())
        .ok_or(Error::Truncated)
}

impl Smacker {
    /// Parses the header, frame tables, and Huffman trees without decoding any frame
    pub fn open(data: Vec<u8>) -> Result<Self, Error> {
        let header::Parsed { header, tree_sizes, trees_len } = header::parse(&data)?;
        let count = header.frames as usize + usize::from(header.flags.ring_frame);
        let types_at = end_of(header::SIZE, count.checked_mul(4).ok_or(Error::Truncated)?, &data)?;
        let trees_at = end_of(types_at, count, &data)?;
        let data_at = end_of(trees_at, trees_len, &data)?;
        let mut entries = Vec::with_capacity(count);
        let mut offset = data_at;
        for index in 0..count {
            let raw = header::read_u32(&data, header::SIZE + 4 * index)?;
            let size = (raw & !3) as usize;
            let end = end_of(offset, size, &data)?;
            entries.push(FrameEntry { offset, size, types: data[types_at + index], keyframe: raw & 1 != 0 || index == 0 });
            offset = end;
        }
        let trees = Trees::read(&data[trees_at..data_at], tree_sizes)?;
        let canvas = Canvas::new(header.width, header.height);
        let (width, height) = (header.width as usize, header.height as usize);
        let cropped = if canvas.stride() == width && canvas.blocks_high * 4 == height {
            Vec::new()
        } else {
            vec![0; width * height]
        };
        Ok(Self {
            data,
            header,
            entries,
            trees,
            canvas,
            cropped,
            palette: [[0; 3]; 256],
            audio: Default::default(),
            frame_index: 0,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Frames yielded by a full pass, which includes the ring frame when present
    pub fn frame_count(&self) -> usize {
        self.entries.len()
    }

    pub fn frame_index(&self) -> usize {
        self.frame_index
    }

    /// Decodes the palette record, audio chunks, and video of the next frame, None past the end
    pub fn next_frame(&mut self) -> Result<Option<Frame<'_>>, Error> {
        let Some(entry) = self.entries.get(self.frame_index).copied() else {
            return Ok(None);
        };
        let frame = &self.data[entry.offset..entry.offset + entry.size];
        let mut position = 0;
        let palette_changed = entry.types & 1 != 0;
        if palette_changed {
            position += palette::apply(frame, &mut self.palette)?;
        }
        for (track, samples) in self.audio.iter_mut().enumerate() {
            samples.clear();
            if entry.types & (2 << track) == 0 {
                continue;
            }
            let size = header::read_u32(frame, position).map_err(|_| Error::BadChunk)? as usize;
            let end = position.checked_add(size).filter(|&end| end <= frame.len() && size >= 4).ok_or(Error::BadChunk)?;
            audio::decode(&frame[position + 4..end], self.header.audio[track], samples)?;
            position = end;
        }
        video::decode_frame(&frame[position..], &mut self.trees, &mut self.canvas, self.header.version)?;
        self.frame_index += 1;
        let (width, height) = (self.header.width as usize, self.header.height as usize);
        Ok(Some(Frame {
            pixels: crop(&self.canvas, &mut self.cropped, width, height),
            palette: &self.palette,
            palette_changed,
            keyframe: entry.keyframe,
            audio: std::array::from_fn(|track| self.audio[track].as_slice()),
        }))
    }

    /// Returns to the first frame with a black palette and a blank canvas
    pub fn rewind(&mut self) {
        self.frame_index = 0;
        self.palette = [[0; 3]; 256];
        self.canvas.pixels.fill(0);
        for samples in &mut self.audio {
            samples.clear();
        }
    }
}

/// Copies the visible region out of the canvas only when the size is not block aligned
fn crop<'a>(canvas: &'a Canvas, cropped: &'a mut Vec<u8>, width: usize, height: usize) -> &'a [u8] {
    if cropped.is_empty() {
        return &canvas.pixels;
    }
    let rows = canvas.pixels.chunks_exact(canvas.stride()).take(height);
    for (target, source) in cropped.chunks_exact_mut(width).zip(rows) {
        target.copy_from_slice(&source[..width]);
    }
    cropped
}

impl Frame<'_> {
    /// Expands the indices through the palette into packed RGB triplets
    pub fn rgb24(&self) -> Vec<u8> {
        let mut out = vec![0; self.pixels.len() * 3];
        for (target, &index) in out.chunks_exact_mut(3).zip(self.pixels) {
            target.copy_from_slice(&self.palette[usize::from(index)]);
        }
        out
    }
}
