use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use avmanager::input::DeviceState;
use avmanager::sound::Sound;
use crunch::host::{Host, NetEvent};

use crate::files::Files;
use crate::net::NetClient;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const STEP_MS: u32 = 33;
const AUDIO_RATE: u32 = 44100;

// Input for one tick of a scripted run
#[derive(Clone, Debug, Default)]
struct ScriptLine {
    keys: Vec<usize>,
    dx: i32,
    dy: i32,
    buttons: [bool; 3],
}

// Scan code for a key name used in scripts
fn key_by_name(name: &str) -> Option<usize> {
    let upper = name.to_ascii_uppercase();
    let fixed = match upper.as_str() {
        "ESC" => 0x01,
        "SPACE" => 0x39,
        "RETURN" | "ENTER" => 0x1c,
        "BACKSPACE" => 0x0e,
        "LEFT" => 0xcb,
        "RIGHT" => 0xcd,
        "UP" => 0xc8,
        "DOWN" => 0xd0,
        "LSHIFT" => 0x2a,
        "LCTRL" => 0x1d,
        _ => 0,
    };
    if fixed != 0 {
        return Some(fixed);
    }
    if let Some(hex) = upper.strip_prefix("0X") {
        return usize::from_str_radix(hex, 16).ok();
    }
    let bytes = upper.as_bytes();
    if bytes.len() != 1 {
        return None;
    }
    let letters = b"QWERTYUIOPASDFGHJKLZXCVBNM";
    let letter_codes = [
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25,
        0x26, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32,
    ];
    if let Some(i) = letters.iter().position(|&c| c == bytes[0]) {
        return Some(letter_codes[i]);
    }
    if bytes[0].is_ascii_digit() {
        return Some(if bytes[0] == b'0' { 0x0b } else { (bytes[0] - b'1') as usize + 0x02 });
    }
    None
}

// Parses "tick keys dx dy buttons" lines, keys comma separated or "-", buttons as bits
fn parse_script(text: &str) -> Result<HashMap<u64, ScriptLine>, String> {
    let mut out = HashMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(format!("script line {}: expected 5 fields", n + 1));
        }
        let tick: u64 = parts[0].parse().map_err(|_| format!("script line {}: bad tick", n + 1))?;
        let mut keys = Vec::new();
        if parts[1] != "-" {
            for name in parts[1].split(',') {
                keys.push(key_by_name(name).ok_or(format!("script line {}: unknown key {name}", n + 1))?);
            }
        }
        let dx = parts[2].parse().map_err(|_| format!("script line {}: bad dx", n + 1))?;
        let dy = parts[3].parse().map_err(|_| format!("script line {}: bad dy", n + 1))?;
        let bits: u32 = parts[4].parse().map_err(|_| format!("script line {}: bad buttons", n + 1))?;
        out.insert(tick, ScriptLine { keys, dx, dy, buttons: [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0] });
    }
    Ok(out)
}

// Deterministic shell with a virtual clock, scripted input and frame dumps, paced to real time on request
pub struct HeadlessHost {
    files: Files,
    pace: Option<Instant>,
    script: HashMap<u64, ScriptLine>,
    out_dir: Option<PathBuf>,
    dump_every: u64,
    max_ticks: u64,
    ticks: u64,
    frames: u64,
    clock_ms: u32,
    seed: u32,
    wav_path: Option<PathBuf>,
    samples: Vec<i16>,
    online: String,
    net: Option<NetClient>,
}

impl HeadlessHost {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        files: Files,
        script_path: Option<&Path>,
        out_dir: Option<PathBuf>,
        dump_every: u64,
        max_ticks: u64,
        seed: u32,
        wav_path: Option<PathBuf>,
        online: String,
        pace: bool,
    ) -> Result<Self, String> {
        let script = match script_path {
            Some(p) => parse_script(&fs::read_to_string(p).map_err(|e| e.to_string())?)?,
            None => HashMap::new(),
        };
        if let Some(d) = &out_dir {
            fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        Ok(Self {
            files,
            pace: pace.then(Instant::now),
            script,
            out_dir,
            dump_every: dump_every.max(1),
            max_ticks,
            ticks: 0,
            frames: 0,
            clock_ms: 0,
            seed,
            wav_path,
            samples: Vec::new(),
            online,
            net: None,
        })
    }

    pub fn audio_rate() -> u32 {
        AUDIO_RATE
    }

    // Writes every mixed sample as a 16 bit stereo WAV when a path was given
    pub fn write_wav(&self) -> Result<(), String> {
        let Some(path) = &self.wav_path else { return Ok(()) };
        let bytes = self.samples.len() * 2;
        let mut out = Vec::with_capacity(44 + bytes);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&((36 + bytes) as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&AUDIO_RATE.to_le_bytes());
        out.extend_from_slice(&(AUDIO_RATE * 4).to_le_bytes());
        out.extend_from_slice(&4u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(bytes as u32).to_le_bytes());
        for s in &self.samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        fs::write(path, out).map_err(|e| e.to_string())
    }

    // Writes a frame as a binary PPM so any image tool can read it
    fn dump(&self, frame: &[u16]) {
        let Some(dir) = &self.out_dir else { return };
        if !self.frames.is_multiple_of(self.dump_every) {
            return;
        }
        let mut data = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
        for &p in frame {
            data.push((((p >> 11) & 0x1f) << 3) as u8);
            data.push((((p >> 5) & 0x3f) << 2) as u8);
            data.push(((p & 0x1f) << 3) as u8);
        }
        let path = dir.join(format!("frame_{:05}.ppm", self.frames));
        if let Err(e) = fs::write(&path, data) {
            eprintln!("cannot write {}: {e}", path.display());
        }
    }
}

impl Host for HeadlessHost {
    async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
        self.files.read_data(name)
    }

    fn time_ms(&self) -> u32 {
        self.clock_ms
    }

    fn unix_time(&self) -> u32 {
        self.seed
    }

    fn poll_input(&mut self) -> DeviceState {
        let mut state = DeviceState::new();
        if let Some(line) = self.script.get(&self.ticks) {
            for &k in &line.keys {
                if k < 256 {
                    state.scan_keys[k] = true;
                }
            }
            state.mouse_dx = line.dx;
            state.mouse_dy = line.dy;
            state.buttons = line.buttons;
        }
        self.ticks += 1;
        state
    }

    async fn end_frame(&mut self, frame: Option<&[u16]>, sound: &mut Sound) {
        if let Some(f) = frame {
            self.dump(f);
            self.frames += 1;
        }
        let mut buf = vec![0i16; (AUDIO_RATE as usize * STEP_MS as usize / 1000) * 2];
        sound.render(&mut buf);
        if self.wav_path.is_some() {
            self.samples.extend_from_slice(&buf);
        }
        self.clock_ms = self.clock_ms.wrapping_add(STEP_MS);
        if let Some(start) = self.pace {
            let due = start + Duration::from_millis(u64::from(self.clock_ms));
            if let Some(wait) = due.checked_duration_since(Instant::now()) {
                thread::sleep(wait);
            }
        }
    }

    fn list_saves(&mut self) -> Vec<String> {
        self.files.list_saves()
    }

    fn read_save(&mut self, name: &str) -> Option<Vec<u8>> {
        self.files.read_save(name)
    }

    fn write_save(&mut self, name: &str, data: &[u8]) -> bool {
        self.files.write_save(name, data)
    }

    fn save_exists(&mut self, name: &str) -> bool {
        self.files.save_exists(name)
    }

    fn delete_save(&mut self, name: &str) {
        self.files.delete_save(name);
    }

    fn take_debug_toggle(&mut self) -> bool {
        false
    }

    fn quit_requested(&self) -> bool {
        self.ticks >= self.max_ticks
    }

    fn open_url(&mut self, url: &str) {
        println!("open {url}");
    }

    fn show_credits(&mut self, _shown: bool) -> bool {
        false
    }

    fn log(&mut self, message: &str) {
        eprintln!("{message}");
    }

    fn net_connect(&mut self, path: &str) {
        self.net = Some(NetClient::connect(format!("{}{path}", self.online)));
    }

    fn net_send(&mut self, data: &[u8]) {
        if let Some(net) = &self.net {
            net.send(data);
        }
    }

    fn net_poll(&mut self) -> Option<NetEvent> {
        self.net.as_ref()?.poll()
    }

    fn net_close(&mut self) {
        self.net = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_lines_parse() {
        let s = parse_script("# comment\n3 SPACE,A 4 -2 1\n").unwrap();
        let line = &s[&3];
        assert_eq!(line.keys, vec![0x39, 0x1e]);
        assert_eq!((line.dx, line.dy), (4, -2));
        assert_eq!(line.buttons, [true, false, false]);
        assert_eq!(key_by_name("0x5e"), Some(0x5e));
        assert_eq!(key_by_name("7"), Some(0x08));
    }
}
