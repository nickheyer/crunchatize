use std::fmt;

use crate::crt::{Rand, ftol, stricmp};

pub const INVALID_HANDLE: u32 = 0xffff_ffff;
pub const STREAM_SLOTS: usize = 16;
pub const STREAM_RATE: u32 = 22050;
const SFX_GROW: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WavError {
    NotRiffWave,
    MissingFmt,
    MissingData,
    NotPcm,
    Truncated,
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            WavError::NotRiffWave => "not a RIFF WAVE file",
            WavError::MissingFmt => "no fmt chunk",
            WavError::MissingData => "no data chunk",
            WavError::NotPcm => "not PCM",
            WavError::Truncated => "file shorter than its chunks claim",
        };
        f.write_str(what)
    }
}

impl std::error::Error for WavError {}

// Raw PCM payload plus the format fields the mixer needs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pcm {
    pub data: Vec<u8>,
    pub rate: u32,
    pub channels: u16,
    pub bits: u16,
    pub block_align: u16,
}

impl Pcm {
    pub fn frames(&self) -> usize {
        if self.block_align == 0 { 0 } else { self.data.len() / self.block_align as usize }
    }

    // Reads one channel sample of a frame as a signed 16 bit value
    fn sample(&self, frame: usize, channel: usize) -> i32 {
        let ch = channel.min(self.channels as usize - 1);
        let base = frame * self.block_align as usize;
        if self.bits == 8 {
            (self.data[base + ch] as i32 - 128) << 8
        } else {
            let i = base + ch * 2;
            i16::from_le_bytes([self.data[i], self.data[i + 1]]) as i32
        }
    }
}

// Reads a WAV assuming the fixed 44 byte canonical header the effect loader expects
pub fn parse_canonical_wav(data: &[u8]) -> Result<Pcm, WavError> {
    if data.len() < 44 {
        return Err(WavError::Truncated);
    }
    let channels = u16::from_le_bytes([data[22], data[23]]);
    let rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let block_align = u16::from_le_bytes([data[32], data[33]]);
    let bits = u16::from_le_bytes([data[34], data[35]]);
    let size = u32::from_le_bytes([data[40], data[41], data[42], data[43]]) as usize;
    if data.len() < 44 + size {
        return Err(WavError::Truncated);
    }
    Ok(Pcm { data: data[44..44 + size].to_vec(), rate, channels, bits, block_align })
}

// Reads a WAV by walking its RIFF chunks the way the stream opener does
pub fn parse_riff_wav(data: &[u8]) -> Result<Pcm, WavError> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(WavError::NotRiffWave);
    }
    let mut pos = 12;
    let mut fmt: Option<(u16, u16, u32, u16, u16)> = None;
    let mut pcm: Option<&[u8]> = None;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        let body_start = pos + 8;
        let body_end = body_start.saturating_add(size).min(data.len());
        let body = &data[body_start..body_end];
        if id == b"fmt " {
            if body.len() < 16 {
                return Err(WavError::MissingFmt);
            }
            fmt = Some((
                u16::from_le_bytes([body[0], body[1]]),
                u16::from_le_bytes([body[2], body[3]]),
                u32::from_le_bytes([body[4], body[5], body[6], body[7]]),
                u16::from_le_bytes([body[12], body[13]]),
                u16::from_le_bytes([body[14], body[15]]),
            ));
        } else if id == b"data" && fmt.is_some() {
            pcm = Some(body);
            break;
        }
        pos = body_start + size + (size & 1);
    }
    let (tag, channels, rate, block_align, bits) = fmt.ok_or(WavError::MissingFmt)?;
    if tag != 1 {
        return Err(WavError::NotPcm);
    }
    let body = pcm.ok_or(WavError::MissingData)?;
    Ok(Pcm { data: body.to_vec(), rate, channels, bits, block_align })
}

#[derive(Clone, Debug, Default)]
struct Sfx {
    name: String,
    state: u16,
    priority: i16,
    random_start: bool,
    pcm: Option<Pcm>,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    in_use: bool,
    sfx: u16,
    repeat: i32,
    counter: i32,
    looping: bool,
    generation: u16,
    playing: bool,
    position: f64,
}

// One second ring buffer slot that streams a WAV from memory in refill chunks
#[derive(Clone, Debug)]
struct Stream {
    bits: u16,
    ring: Vec<u8>,
    play_pos: f64,
    playing: bool,
    active: bool,
    has_more: bool,
    looping: bool,
    generation: u32,
    last_write: i64,
    remaining: i64,
    start_countdown: i16,
    refill_timer: i16,
    source: Option<Pcm>,
    source_pos: usize,
}

impl Stream {
    fn new(bits: u16) -> Self {
        let size = (bits as usize * STREAM_RATE as usize) / 8;
        Self {
            bits,
            ring: vec![0; size],
            play_pos: 0.0,
            playing: false,
            active: false,
            has_more: false,
            looping: false,
            generation: 0,
            last_write: -1,
            remaining: 0,
            start_countdown: 0,
            refill_timer: 0,
            source: None,
            source_pos: 0,
        }
    }

    fn size(&self) -> usize {
        self.ring.len()
    }

    fn bytes_per_frame(&self) -> usize {
        self.bits as usize / 8
    }

    fn silence(&self) -> u8 {
        if self.bits == 8 { 0x80 } else { 0 }
    }

    fn play_cursor(&self) -> i64 {
        (self.play_pos as usize * self.bytes_per_frame()) as i64
    }

    // Copies up to `len` bytes of source audio into the ring, returning how many were copied
    fn read_source(&mut self, start: usize, len: usize) -> usize {
        let Some(src) = &self.source else { return 0 };
        let avail = src.data.len().saturating_sub(self.source_pos);
        let n = len.min(avail);
        let size = self.size();
        for i in 0..n {
            self.ring[(start + i) % size] = src.data[self.source_pos + i];
        }
        self.source_pos += n;
        n
    }

    // Fills a locked region, restarting the source when looping or padding silence when not
    fn fill_region(&mut self, start: usize, len: usize) {
        let got = self.read_source(start, len);
        if got < len {
            if self.looping {
                self.source_pos = 0;
                self.read_source(start + got, len - got);
            } else {
                let fill = self.silence();
                let size = self.size();
                for i in got..len {
                    self.ring[(start + i) % size] = fill;
                }
                self.has_more = false;
                self.remaining = size as i64;
            }
        }
    }

    // Refills the ring from the last write point up to the play cursor
    fn refill(&mut self) {
        let play = self.play_cursor();
        if self.last_write == -1 {
            let size = self.size();
            self.last_write = 0;
            self.fill_region(0, size);
        } else {
            let mut bytes = play - self.last_write;
            if bytes < 0 {
                bytes += self.size() as i64;
            }
            if bytes > 0 {
                let start = self.last_write as usize;
                self.fill_region(start, bytes as usize);
            }
            self.last_write = play;
        }
    }

    // Writes silence behind the play cursor while draining and reports when the tail is spent
    fn drain(&mut self) -> bool {
        let play = self.play_cursor();
        let mut bytes = play - self.last_write;
        if bytes < 0 {
            bytes += self.size() as i64;
        }
        if bytes < 1 {
            return false;
        }
        let fill = self.silence();
        let start = self.last_write as usize;
        let size = self.size();
        for i in 0..bytes as usize {
            self.ring[(start + i) % size] = fill;
        }
        self.last_write = play;
        self.remaining -= bytes;
        self.remaining < 1
    }

    fn stop(&mut self) {
        self.playing = false;
        self.play_pos = 0.0;
    }

    fn sample_at(&self, frame: usize) -> i32 {
        if self.bits == 8 {
            (self.ring[frame] as i32 - 128) << 8
        } else {
            let i = frame * 2;
            i16::from_le_bytes([self.ring[i], self.ring[i + 1]]) as i32
        }
    }
}

// Decoded video audio waiting to be mixed, interleaved stereo at the video's own rate
#[derive(Debug, Default)]
struct VideoAudio {
    queue: std::collections::VecDeque<(i16, i16)>,
    rate: u32,
    position: f64,
}

// Software replacement for the DirectSound based effect and stream player
#[derive(Debug)]
pub struct Sound {
    sfx: Vec<Sfx>,
    sfx_count: u16,
    channels: Vec<Channel>,
    streams: Vec<Stream>,
    generation: u16,
    master_volume: f32,
    out_rate: u32,
    rand: Rand,
    video: VideoAudio,
}

impl Sound {
    // Creates the mixer with the requested effect channel count, capped at 32 like the original
    pub fn new(channel_count: u16, out_rate: u32) -> Self {
        let n = channel_count.min(0x20) as usize;
        let streams = (0..STREAM_SLOTS).map(|i| Stream::new(if i < 8 { 8 } else { 16 })).collect();
        Self {
            sfx: Vec::new(),
            sfx_count: 0,
            channels: vec![Channel::default(); n],
            streams,
            generation: 0,
            master_volume: 100.0,
            out_rate,
            rand: Rand::default(),
            video: VideoAudio::default(),
        }
    }

    // Queues one video frame's worth of decoded samples for mixing
    pub fn push_video_audio(&mut self, samples: &[i16], rate: u32, stereo: bool) {
        self.video.rate = rate;
        if stereo {
            for pair in samples.chunks_exact(2) {
                self.video.queue.push_back((pair[0], pair[1]));
            }
        } else {
            for &s in samples {
                self.video.queue.push_back((s, s));
            }
        }
    }

    pub fn stop_video_audio(&mut self) {
        self.video.queue.clear();
        self.video.position = 0.0;
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume;
    }

    // Hundredths of a decibel the primary buffer would be set to
    pub fn master_attenuation(&self) -> i32 {
        ftol(10000.0 * self.master_volume as f64 / 100.0) - 10000
    }

    fn find_sfx(&mut self, name: &str) -> Option<u16> {
        for (i, s) in self.sfx.iter_mut().enumerate() {
            if s.state != 0 && stricmp(s.name.as_bytes(), name.as_bytes()) == 0 {
                if s.state == 2 {
                    s.state = 1;
                }
                return Some(i as u16);
            }
        }
        None
    }

    // Registers an effect by name, decoding its canonical WAV bytes when first seen
    pub fn get_sfx(&mut self, name: &str, wav: Option<&[u8]>, priority: i16, random_start: bool) -> u16 {
        if let Some(h) = self.find_sfx(name) {
            return h;
        }
        if self.sfx.iter().all(|s| s.state != 0) {
            self.sfx.extend((0..SFX_GROW).map(|_| Sfx::default()));
        }
        let h = self.sfx.iter().position(|s| s.state == 0).unwrap() as u16;
        let pcm = wav.and_then(|w| parse_canonical_wav(w).ok());
        self.sfx[h as usize] = Sfx {
            name: name.to_string(),
            state: if pcm.is_some() { 1 } else { 3 },
            priority,
            random_start,
            pcm,
        };
        self.sfx_count += 1;
        h
    }

    pub fn total_sounds_loaded(&self) -> u16 {
        self.sfx_count
    }

    pub fn sound_ram_usage(&self) -> i32 {
        self.sfx.iter().filter_map(|s| s.pcm.as_ref()).map(|p| p.data.len() as i32).sum()
    }

    fn next_generation(&mut self) -> u16 {
        let g = self.generation;
        self.generation = self.generation.wrapping_add(1);
        g
    }

    // Starts an effect, stealing the lowest priority one shot channel when all are busy
    pub fn play_sfx(&mut self, handle: u16, mode: i32) -> u32 {
        let Some(sfx) = self.sfx.get(handle as usize) else { return INVALID_HANDLE };
        let Some(pcm) = sfx.pcm.as_ref() else { return INVALID_HANDLE };
        let frames = pcm.frames();
        let bytes = pcm.data.len();
        let block = pcm.block_align as usize;
        let random_start = sfx.random_start;
        let mut best_priority = sfx.priority;
        let mut best: Option<usize> = None;
        let mut idx = self.channels.len();
        for (i, ch) in self.channels.iter().enumerate() {
            if !ch.in_use {
                idx = i;
                break;
            }
            let p = self.sfx[ch.sfx as usize].priority;
            if p < best_priority && ch.repeat == -1 {
                best_priority = p;
                best = Some(i);
            }
        }
        if idx == self.channels.len()
            && let Some(b) = best {
                let h = (self.channels[b].generation as u32) << 16 | b as u32;
                self.stop_sfx(h);
                idx = b;
            }
        if idx >= self.channels.len() {
            return INVALID_HANDLE;
        }
        let generation = self.next_generation();
        let ch = &mut self.channels[idx];
        ch.repeat = mode;
        ch.counter = mode;
        ch.sfx = handle;
        ch.in_use = true;
        ch.generation = generation;
        ch.position = 0.0;
        ch.looping = mode == 0;
        if mode == 0 && random_start {
            let r = self.rand.ranged(0, 3);
            let byte_pos = (bytes >> 2) * r as usize;
            let frame = byte_pos.checked_div(block).unwrap_or(0);
            self.channels[idx].position = frame.min(frames) as f64;
        }
        self.channels[idx].playing = true;
        (generation as u32) << 16 | idx as u32
    }

    // Stops a channel when the handle generation still matches it
    pub fn stop_sfx(&mut self, handle: u32) -> bool {
        if handle == INVALID_HANDLE {
            return false;
        }
        let idx = (handle & 0xffff) as usize;
        if idx >= self.channels.len() {
            return false;
        }
        let ch = &mut self.channels[idx];
        if (handle >> 16) as u16 != ch.generation || !ch.in_use {
            return false;
        }
        ch.playing = false;
        ch.looping = false;
        ch.in_use = false;
        ch.generation = 0;
        ch.position = 0.0;
        true
    }

    pub fn destroy_all_sounds(&mut self) {
        for ch in &mut self.channels {
            *ch = Channel::default();
        }
        for st in &mut self.streams {
            if st.active {
                st.active = false;
            }
        }
        for s in &mut self.sfx {
            if s.state != 0 {
                *s = Sfx::default();
            }
        }
        self.sfx_count = 0;
    }

    // Opens a stream slot matching the WAV bit depth and primes its ring buffer
    pub fn play_stream_sound(&mut self, wav: &[u8], looping: bool, previous: u32) -> u32 {
        if previous != 0 {
            self.kill_streaming_channel(previous);
        }
        let Ok(pcm) = parse_riff_wav(wav) else { return INVALID_HANDLE };
        let Some(slot) = self.streams.iter().position(|s| !s.active && s.bits == pcm.bits) else {
            return INVALID_HANDLE;
        };
        let generation = self.next_generation() as u32;
        let st = &mut self.streams[slot];
        st.source = Some(pcm);
        st.source_pos = 0;
        st.has_more = true;
        st.last_write = -1;
        st.start_countdown = 6;
        st.looping = looping;
        st.generation = generation;
        st.remaining = 0;
        st.play_pos = 0.0;
        st.playing = false;
        let handle = generation << 16 | slot as u32;
        st.refill();
        st.active = true;
        handle
    }

    fn stream_for(&mut self, handle: u32) -> Option<&mut Stream> {
        if handle == INVALID_HANDLE {
            return None;
        }
        let slot = (handle & 0xffff) as usize;
        let st = self.streams.get_mut(slot)?;
        if st.generation != handle >> 16 {
            return None;
        }
        Some(st)
    }

    // Reports completion once the stream stopped on its own and releases the slot
    pub fn is_stream_sound_done(&mut self, handle: u32) -> bool {
        let Some(st) = self.stream_for(handle) else { return false };
        if st.active && (st.start_countdown > 0 || st.playing) {
            return false;
        }
        if st.active {
            self.kill_streaming_channel(handle);
        }
        true
    }

    pub fn kill_streaming_channel(&mut self, handle: u32) -> bool {
        let Some(st) = self.stream_for(handle) else { return false };
        if !st.active {
            return false;
        }
        st.active = false;
        st.stop();
        st.source = None;
        true
    }

    // Services finished one shot channels, repeat timers, and stream refills once per tick
    pub fn heartbeat(&mut self) {
        for i in 0..self.channels.len() {
            let ch = &self.channels[i];
            if !ch.in_use || ch.playing {
                continue;
            }
            if ch.repeat == 0 {
                continue;
            }
            if ch.repeat > 0 {
                let ch = &mut self.channels[i];
                ch.counter -= 1;
                if ch.counter == 0 {
                    ch.position = 0.0;
                    ch.playing = true;
                    ch.counter = ch.repeat;
                }
                continue;
            }
            let h = (ch.generation as u32) << 16 | i as u32;
            self.stop_sfx(h);
        }
        for st in &mut self.streams {
            if !st.active {
                continue;
            }
            if st.start_countdown < 1 {
                if st.has_more {
                    st.refill_timer -= 1;
                    if st.refill_timer < 1 {
                        st.refill();
                        st.refill_timer = 5;
                    }
                } else if st.drain() {
                    st.stop();
                    st.active = false;
                }
            } else {
                st.start_countdown -= 1;
                if st.start_countdown < 1 {
                    st.playing = true;
                    st.refill_timer = 5;
                }
            }
        }
    }

    // Mixes every playing channel and stream into interleaved stereo output
    pub fn render(&mut self, out: &mut [i16]) {
        out.fill(0);
        let out_rate = self.out_rate as f64;
        let mut acc = vec![0i32; out.len()];
        for ch in &mut self.channels {
            if !ch.playing {
                continue;
            }
            let pcm = match self.sfx[ch.sfx as usize].pcm.as_ref() {
                Some(p) => p,
                None => continue,
            };
            let frames = pcm.frames();
            if frames == 0 {
                ch.playing = false;
                continue;
            }
            let step = pcm.rate as f64 / out_rate;
            for pair in acc.chunks_exact_mut(2) {
                if ch.position >= frames as f64 {
                    if ch.looping {
                        ch.position -= frames as f64;
                    } else {
                        ch.playing = false;
                        ch.position = 0.0;
                        break;
                    }
                }
                let f0 = ch.position as usize;
                let frac = ch.position - f0 as f64;
                let f1 = if f0 + 1 < frames { f0 + 1 } else if ch.looping { 0 } else { f0 };
                for (c, slot) in pair.iter_mut().enumerate() {
                    let a = pcm.sample(f0, c) as f64;
                    let b = pcm.sample(f1, c) as f64;
                    *slot += (a + (b - a) * frac) as i32;
                }
                ch.position += step;
            }
        }
        for st in &mut self.streams {
            if !st.playing {
                continue;
            }
            let frames = st.size() / st.bytes_per_frame();
            let step = STREAM_RATE as f64 / out_rate;
            for pair in acc.chunks_exact_mut(2) {
                while st.play_pos >= frames as f64 {
                    st.play_pos -= frames as f64;
                }
                let f0 = st.play_pos as usize;
                let frac = st.play_pos - f0 as f64;
                let f1 = (f0 + 1) % frames;
                let a = st.sample_at(f0) as f64;
                let b = st.sample_at(f1) as f64;
                let v = (a + (b - a) * frac) as i32;
                pair[0] += v;
                pair[1] += v;
                st.play_pos += step;
            }
        }
        if self.video.rate != 0 && !self.video.queue.is_empty() {
            let step = self.video.rate as f64 / out_rate;
            for pair in acc.chunks_exact_mut(2) {
                while self.video.position >= 1.0 {
                    self.video.queue.pop_front();
                    self.video.position -= 1.0;
                }
                let Some(&(l0, r0)) = self.video.queue.front() else { break };
                let (l1, r1) = self.video.queue.get(1).copied().unwrap_or((l0, r0));
                let frac = self.video.position;
                pair[0] += (l0 as f64 + (l1 as f64 - l0 as f64) * frac) as i32;
                pair[1] += (r0 as f64 + (r1 as f64 - r0 as f64) * frac) as i32;
                self.video.position += step;
            }
        }
        let gain = 10f64.powf(self.master_attenuation() as f64 / 2000.0);
        for (dst, &v) in out.iter_mut().zip(&acc) {
            *dst = ((v as f64 * gain) as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(frames: usize) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"RIFF");
        d.extend_from_slice(&((36 + frames * 2) as u32).to_le_bytes());
        d.extend_from_slice(b"WAVEfmt ");
        d.extend_from_slice(&16u32.to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&22050u32.to_le_bytes());
        d.extend_from_slice(&44100u32.to_le_bytes());
        d.extend_from_slice(&2u16.to_le_bytes());
        d.extend_from_slice(&16u16.to_le_bytes());
        d.extend_from_slice(b"data");
        d.extend_from_slice(&((frames * 2) as u32).to_le_bytes());
        for i in 0..frames {
            d.extend_from_slice(&(i as i16).wrapping_mul(100).to_le_bytes());
        }
        d
    }

    #[test]
    fn canonical_and_riff_parsers_agree() {
        let w = wav(10);
        assert_eq!(parse_canonical_wav(&w).unwrap(), parse_riff_wav(&w).unwrap());
    }

    #[test]
    fn one_shot_channel_frees_after_finishing() {
        let mut s = Sound::new(16, 22050);
        let w = wav(4);
        let h = s.get_sfx("a.wav", Some(&w), 0, false);
        let ch = s.play_sfx(h, -1);
        assert_eq!(ch, 0);
        let mut out = vec![0i16; 2 * 10];
        s.render(&mut out);
        assert!(!s.channels[0].playing);
        s.heartbeat();
        assert!(!s.channels[0].in_use);
    }

    #[test]
    fn lower_priority_one_shot_is_stolen() {
        let mut s = Sound::new(1, 22050);
        let w = wav(4000);
        let low = s.get_sfx("low.wav", Some(&w), 1, false);
        let high = s.get_sfx("high.wav", Some(&w), 5, false);
        let first = s.play_sfx(low, -1);
        let second = s.play_sfx(high, -1);
        assert_eq!(first & 0xffff, 0);
        assert_eq!(second & 0xffff, 0);
        assert_ne!(first >> 16, second >> 16);
        assert_eq!(s.channels[0].sfx, high);
    }

    #[test]
    fn stream_starts_after_six_ticks_and_finishes() {
        let mut s = Sound::new(16, 22050);
        let w = wav(100);
        let h = s.play_stream_sound(&w, false, 0);
        assert_eq!(h & 0xffff, 8);
        for _ in 0..5 {
            s.heartbeat();
            assert!(!s.is_stream_sound_done(h));
        }
        s.heartbeat();
        assert!(s.streams[8].playing);
        let mut out = vec![0i16; 2 * 22050];
        s.render(&mut out);
        for _ in 0..40 {
            s.heartbeat();
            s.render(&mut out);
        }
        assert!(s.is_stream_sound_done(h));
        assert!(!s.streams[8].active);
    }
}
