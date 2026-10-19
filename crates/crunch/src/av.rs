use avmanager::anim::{AnimFrame, AnimManager};
use avmanager::bam;
use avmanager::crt::{asset_key, with_extension};
use avmanager::display::Display;
use avmanager::font::{self, FontManager};
use avmanager::input::Input;
use avmanager::rect::Rect;
use avmanager::sound::Sound;
use avmanager::texture::{BitmapManager, Palette24};

use crate::host::Host;

pub const SCREEN_WIDTH: u16 = 640;
pub const SCREEN_HEIGHT: u16 = 480;
pub const SOUND_CHANNELS: u16 = 16;
pub const MOUSE_SENSITIVITY: f32 = 0.5;

// The engine as the game sees it, one struct standing in for the AVManager DLL
pub struct Av {
    pub display: Display,
    pub bitmaps: BitmapManager,
    pub anims: AnimManager,
    pub fonts: FontManager,
    pub sound: Sound,
    pub input: Input,
}

impl Av {
    pub fn new(out_rate: u32) -> Self {
        let mut sound = Sound::new(SOUND_CHANNELS, out_rate);
        sound.set_master_volume(100.0);
        Self {
            display: Display::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            bitmaps: BitmapManager::default(),
            anims: AnimManager::default(),
            fonts: FontManager::default(),
            sound,
            input: Input::new(MOUSE_SENSITIVITY),
        }
    }

    // Loads a BAM file under a key, different from the file name for a second copy with its own colours
    async fn load_anim<H: Host>(
        &mut self,
        host: &mut H,
        key: &str,
        file: &str,
        palette: Option<&Palette24>,
        retain: bool,
    ) -> Option<u16> {
        if let Some(h) = self.anims.find(&asset_key(key)) {
            return Some(h);
        }
        let file = with_extension(file, ".bam");
        let Some(data) = host.read_file(&file).await else {
            host.log(&format!("*Error loading {file}"));
            return None;
        };
        match bam::parse(&data) {
            Ok(parsed) => Some(self.anims.load(key, &parsed, &mut self.bitmaps, palette, retain)),
            Err(e) => {
                host.log(&format!("*Error loading {file}: {e}"));
                None
            }
        }
    }

    pub async fn get_anim<H: Host>(&mut self, host: &mut H, name: &str) -> Option<u16> {
        self.load_anim(host, name, name, None, false).await
    }

    pub async fn get_anim_use_palette<H: Host>(&mut self, host: &mut H, name: &str, palette: &Palette24) -> Option<u16> {
        self.load_anim(host, name, name, Some(palette), false).await
    }

    pub async fn get_anim_retain_image<H: Host>(&mut self, host: &mut H, name: &str) -> Option<u16> {
        self.load_anim(host, name, name, None, true).await
    }

    // Loads a file as a second copy under its own key so another crunchling can wear different colours
    pub async fn get_anim_keyed<H: Host>(
        &mut self,
        host: &mut H,
        key: &str,
        file: &str,
        palette: Option<&Palette24>,
        retain: bool,
    ) -> Option<u16> {
        self.load_anim(host, key, file, palette, retain).await
    }

    // Writes the animation's frame rate into the target when the handle is live
    pub fn get_anim_fps(&self, handle: u16, out: &mut u16) {
        if let Some(f) = self.anims.fps(handle) {
            *out = f;
        }
    }

    pub fn get_hot_spot(&self, handle: u16, x: &mut i32, y: &mut i32) {
        if let Some((hx, hy)) = self.anims.hot_spot(handle) {
            *x = hx;
            *y = hy;
        }
    }

    pub fn get_anim_bbox(&self, handle: u16) -> [i32; 4] {
        self.anims.bbox(handle)
    }

    pub fn get_anim_next_frame(&self, af: &mut AnimFrame) {
        self.anims.next_frame(af);
    }

    pub fn draw_anim(&mut self, af: &mut AnimFrame, x: i32, y: i32) {
        self.anims.draw(af, x, y, &self.bitmaps, &mut self.display);
    }

    pub fn draw_anim_clip(&mut self, af: &mut AnimFrame, x: i16, y: i16, clip: &mut Rect) {
        self.anims.draw_clip(af, x, y, clip, &self.bitmaps, &mut self.display);
    }

    // Draws a freshly reset animation at a point the way stack built AnimFrames are used
    pub fn draw_anim_reset(&mut self, handle: u16, x: i32, y: i32) {
        let mut af = AnimFrame { frame: 0.0, anim: handle, state: avmanager::anim::STATE_RESTART_FORWARD, fps: 0 };
        self.draw_anim(&mut af, x, y);
    }

    pub fn set_anim_palette(&mut self, handle: u16, palette: &Palette24) {
        self.anims.set_palette(handle, palette, &mut self.bitmaps);
    }

    pub fn get_anim_palette(&self, handle: u16, out: &mut Palette24) {
        if let Some(p) = self.anims.palette(handle, &self.bitmaps) {
            *out = p;
        }
    }

    pub async fn get_font<H: Host>(&mut self, host: &mut H, name: &str) -> Option<u16> {
        if let Some(h) = self.fonts.find(&asset_key(name)) {
            return Some(h);
        }
        let file = with_extension(name, ".fnt");
        let data = host.read_file(&file).await?;
        let parsed = match font::parse_fnt(&data) {
            Ok(p) => p,
            Err(e) => {
                host.log(&format!("*Error loading {name}: {e}"));
                return None;
            }
        };
        let anim = self.get_anim(host, &parsed.bam_name).await?;
        let bbox = self.anims.bbox(anim);
        Some(self.fonts.install(name, &parsed, anim, bbox))
    }

    pub fn print(&mut self, text: &str, x: i32, y: i32, font: u32, align: i32) {
        self.fonts.print(text, x, y as i16, font, align, 0, &self.anims, &self.bitmaps, &mut self.display);
    }

    pub fn print_cursor(&mut self, text: &str, x: i32, y: i32, font: u32, align: i32, cursor: u32) {
        self.fonts.print(text, x, y as i16, font, align, cursor, &self.anims, &self.bitmaps, &mut self.display);
    }

    pub fn calc_text_size(&self, text: &str, font: u32, cursor: u32) -> i32 {
        self.fonts.text_width(text, font, cursor)
    }

    pub fn get_font_height(&self, font: u32) -> i32 {
        self.fonts.height(font)
    }

    pub async fn get_sfx<H: Host>(&mut self, host: &mut H, name: &str, priority: i16, random_start: bool) -> Option<u16> {
        let data = host.read_file(name).await;
        if data.is_none() {
            host.log(&format!("*Error loading {name}"));
            return None;
        }
        Some(self.sound.get_sfx(name, data.as_deref(), priority, random_start))
    }

    pub fn play_sfx(&mut self, handle: u16, mode: i32) -> u32 {
        self.sound.play_sfx(handle, mode)
    }

    pub fn stop_sfx(&mut self, channel: u32) {
        self.sound.stop_sfx(channel);
    }

    pub async fn play_stream_sound<H: Host>(&mut self, host: &mut H, name: &str, looping: bool, previous: u32) -> u32 {
        let Some(data) = host.read_file(name).await else {
            host.log(&format!("*Error loading {name}"));
            if previous != 0 {
                self.sound.kill_streaming_channel(previous);
            }
            return avmanager::sound::INVALID_HANDLE;
        };
        self.sound.play_stream_sound(&data, looping, previous)
    }

    pub fn is_stream_sound_done(&mut self, channel: u32) -> bool {
        self.sound.is_stream_sound_done(channel)
    }

    pub fn kill_streaming_channel(&mut self, channel: u32) {
        self.sound.kill_streaming_channel(channel);
    }

    pub fn is_key_down(&self, code: i32) -> bool {
        self.input.is_key_down(code)
    }

    pub fn key_input(&self) -> i32 {
        self.input.key_input()
    }

    pub fn is_mouse_button_down(&self, button: usize) -> bool {
        self.input.is_mouse_button_down(button)
    }

    pub fn get_mouse_delta(&self) -> (i16, i16) {
        self.input.mouse_delta()
    }

    pub fn clear_buffer(&mut self, r: i32, g: i32, b: i32) {
        self.display.clear(r, g, b);
    }

    // Flips the buffers and hands the finished frame to the host until the next refresh
    pub async fn swap_buffers<H: Host>(&mut self, host: &mut H) {
        self.display.swap();
        host.end_frame(Some(self.display.front()), &mut self.sound).await;
    }

    // Releases every animation, bitmap, font and sound like a state switch does
    pub fn release_all(&mut self) {
        self.anims.release_all();
        self.bitmaps.flush_all();
        self.fonts.release_all();
        self.sound.destroy_all_sounds();
    }
}
