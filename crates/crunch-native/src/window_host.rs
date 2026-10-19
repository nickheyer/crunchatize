use std::time::Instant;

use avmanager::input::DeviceState;
use avmanager::sound::Sound;
use crunch::host::Host;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

use crate::audio::Audio;
use crate::files::Files;
use crate::keys::scan_code;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

// Desktop shell with a scaled window, the window mouse as the pointer and the audio queue
pub struct WindowHost {
    window: Window,
    files: Files,
    audio: Audio,
    scale: usize,
    start: Instant,
    last_audio: Instant,
    rgb: Vec<u32>,
    debug_toggle: bool,
    quit_requested: bool,
}

impl WindowHost {
    pub fn new(files: Files, audio: Audio, scale: usize) -> Result<Self, String> {
        let mut window = Window::new(
            "Crunchling Adventure",
            WIDTH * scale,
            HEIGHT * scale,
            WindowOptions::default(),
        )
        .map_err(|e| e.to_string())?;
        window.set_target_fps(60);
        Ok(Self {
            window,
            files,
            audio,
            scale,
            start: Instant::now(),
            last_audio: Instant::now(),
            rgb: vec![0; WIDTH * HEIGHT * scale * scale],
            debug_toggle: false,
            quit_requested: false,
        })
    }

    // Expands a 5-6-5 frame to the window's 0RGB pixels with integer scaling
    fn convert(&mut self, frame: &[u16]) {
        let s = self.scale;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let p = frame[y * WIDTH + x] as u32;
                let r = ((p >> 11) & 0x1f) << 3;
                let g = ((p >> 5) & 0x3f) << 2;
                let b = (p & 0x1f) << 3;
                let v = (r << 16) | (g << 8) | b;
                for sy in 0..s {
                    let row = (y * s + sy) * WIDTH * s;
                    for sx in 0..s {
                        self.rgb[row + x * s + sx] = v;
                    }
                }
            }
        }
    }

    fn render_audio(&mut self, sound: &mut Sound) {
        let wanted = self.audio.frames_wanted();
        if wanted == 0 {
            return;
        }
        let elapsed = self.last_audio.elapsed().as_secs_f64();
        self.last_audio = Instant::now();
        let by_time = (elapsed * self.audio.rate() as f64) as usize;
        let frames = wanted.min(by_time.max(1));
        let mut buf = vec![0i16; frames * 2];
        sound.render(&mut buf);
        self.audio.push(&buf);
    }
}

impl Host for WindowHost {
    async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
        self.files.read_data(name)
    }

    fn time_ms(&self) -> u32 {
        self.start.elapsed().as_millis() as u32
    }

    fn unix_time(&self) -> u32 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0)
    }

    fn poll_input(&mut self) -> DeviceState {
        let mut state = DeviceState::new();
        if !self.window.is_open() {
            self.quit_requested = true;
        }
        for key in self.window.get_keys() {
            let code = scan_code(key);
            if code != 0 {
                state.scan_keys[code] = true;
            }
        }
        if self.window.is_key_pressed(Key::F1, KeyRepeat::No) {
            self.debug_toggle = true;
        }
        if let Some((x, y)) = self.window.get_mouse_pos(MouseMode::Clamp) {
            state.pointer = Some(((x / self.scale as f32) as i16, (y / self.scale as f32) as i16));
        }
        state.buttons = [
            self.window.get_mouse_down(MouseButton::Left),
            self.window.get_mouse_down(MouseButton::Right),
            self.window.get_mouse_down(MouseButton::Middle),
        ];
        state
    }

    async fn end_frame(&mut self, frame: Option<&[u16]>, sound: &mut Sound) {
        if let Some(f) = frame {
            self.convert(f);
        }
        self.render_audio(sound);
        let s = self.scale;
        if self.window.update_with_buffer(&self.rgb, WIDTH * s, HEIGHT * s).is_err() {
            self.quit_requested = true;
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
        std::mem::take(&mut self.debug_toggle)
    }

    fn quit_requested(&self) -> bool {
        self.quit_requested
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
}
