use crate::bam::Bam;
use crate::crt::{asset_key, stricmp};
use crate::display::Display;
use crate::rect::Rect;
use crate::texture::{BitmapManager, Palette24, Texture, color_key_565};

// Playback cursor the game keeps per on screen animation instance
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimFrame {
    pub frame: f32,
    pub anim: u16,
    pub state: i16,
    pub fps: u16,
}

pub const STATE_STOPPED: i16 = 0;
pub const STATE_FORWARD: i16 = 1;
pub const STATE_BACKWARD: i16 = 2;
pub const STATE_RESTART_FORWARD: i16 = 3;
pub const STATE_RESTART_BACKWARD: i16 = 4;

pub const LOOP_WRAP: i16 = 0;
pub const LOOP_PINGPONG: i16 = 1;
pub const LOOP_ONCE: i16 = 2;

#[derive(Clone, Copy, Debug)]
pub struct FrameRef {
    pub x: i16,
    pub y: i16,
    pub bitmap: u16,
}

#[derive(Clone, Debug, Default)]
pub struct AnimRecord {
    pub name: String,
    pub in_use: bool,
    pub loop_mode: i16,
    pub fps: u16,
    pub frames: Vec<FrameRef>,
    pub color_key: u32,
    pub transparent: bool,
    pub bbox: [i32; 4],
    pub first: u16,
    pub last: u16,
}

#[derive(Default, Debug)]
pub struct AnimManager {
    slots: Vec<AnimRecord>,
    count: i16,
}

const GROW: usize = 25;

impl AnimManager {
    // Finds a loaded animation by its case insensitive key name
    pub fn find(&self, key: &str) -> Option<u16> {
        self.slots
            .iter()
            .position(|s| s.in_use && stricmp(s.name.as_bytes(), key.as_bytes()) == 0)
            .map(|i| i as u16)
    }

    // Picks the lowest free slot, growing the table in blocks when none is free
    fn allocate(&mut self) -> u16 {
        if let Some(i) = self.slots.iter().position(|s| !s.in_use) {
            return i as u16;
        }
        let i = self.slots.len();
        self.slots.extend((0..GROW).map(|_| AnimRecord::default()));
        i as u16
    }

    // Registers a parsed BAM and its textures and returns the new handle
    pub fn load(
        &mut self,
        name: &str,
        bam: &Bam,
        bitmaps: &mut BitmapManager,
        use_palette: Option<&Palette24>,
        retain: bool,
    ) -> u16 {
        let key = asset_key(name);
        if let Some(existing) = self.find(&key) {
            self.release(existing);
        }
        let handle = self.allocate();
        let mut palette = bam.palette.clone();
        if let Some(p) = use_palette {
            palette.resize(256, [0; 3]);
            palette[..255].copy_from_slice(&p.rgb[..255]);
        }
        let key_value = color_key_565(bam.color_key);
        let mut frames = Vec::with_capacity(bam.frames.len());
        for f in &bam.frames {
            let mut tex = Texture::from_bitmap(&f.bitmap, &palette, retain);
            if bam.transparent {
                tex.key = Some(key_value);
            }
            frames.push(FrameRef {
                x: f.x,
                y: f.y,
                bitmap: bitmaps.add(tex),
            });
        }
        let last = frames.len().saturating_sub(1) as u16;
        self.slots[handle as usize] = AnimRecord {
            name: key,
            in_use: true,
            loop_mode: bam.loop_mode,
            fps: bam.fps,
            frames,
            color_key: bam.color_key,
            transparent: bam.transparent,
            bbox: bam.bbox,
            first: 0,
            last,
        };
        self.count += 1;
        handle
    }

    pub fn release(&mut self, handle: u16) {
        if let Some(s) = self.slots.get_mut(handle as usize)
            && s.in_use {
                s.in_use = false;
                s.frames = Vec::new();
                self.count -= 1;
            }
    }

    pub fn release_all(&mut self) {
        for i in 0..self.slots.len() {
            self.release(i as u16);
        }
    }

    pub fn count(&self) -> i16 {
        self.count
    }

    fn live(&self, handle: u16) -> Option<&AnimRecord> {
        self.slots.get(handle as usize).filter(|s| s.in_use)
    }

    pub fn fps(&self, handle: u16) -> Option<u16> {
        self.live(handle).map(|s| s.fps)
    }

    // First frame offset pair, absent when the animation has no frames
    pub fn hot_spot(&self, handle: u16) -> Option<(i32, i32)> {
        let s = self.live(handle)?;
        let f = s.frames.first()?;
        Some((f.x as i32, f.y as i32))
    }

    pub fn bbox(&self, handle: u16) -> [i32; 4] {
        self.slots[handle as usize].bbox
    }

    // Advances a playback cursor one tick using the animation's loop mode and fps
    pub fn next_frame(&self, af: &mut AnimFrame) {
        let Some(s) = self.live(af.anim) else { return };
        let first = s.first as u32;
        let last = s.last as u32;
        if (last as i32 - first as i32) < 1 {
            af.frame = first as f32;
            return;
        }
        if af.state == STATE_STOPPED {
            return;
        }
        if af.state == STATE_RESTART_FORWARD {
            af.state = STATE_FORWARD;
            af.frame = first as f32;
            return;
        }
        if af.state == STATE_RESTART_BACKWARD {
            af.state = STATE_BACKWARD;
            af.frame = last as f32;
            return;
        }
        let step: f32 = if af.fps != 0 { (af.fps as f64 / 30.0) as f32 } else { 1.0 };
        let end = (last + 1) as f32;
        let first_f = first as f32;
        match s.loop_mode {
            LOOP_PINGPONG => {
                if af.state == STATE_FORWARD {
                    af.frame += step;
                    if end <= af.frame {
                        af.state = STATE_BACKWARD;
                        af.frame = (last as f32 as f64 + 0.99) as f32;
                    }
                } else {
                    af.frame -= step;
                    if af.frame < first_f || end <= af.frame {
                        af.state = STATE_FORWARD;
                        af.frame = first_f;
                    }
                }
            }
            LOOP_ONCE => {
                if af.state == STATE_FORWARD {
                    af.frame += step;
                    if end <= af.frame {
                        af.frame = last as f32;
                        af.state = STATE_STOPPED;
                    }
                } else {
                    af.frame -= step;
                    if af.frame < first_f {
                        af.frame = first_f;
                        af.state = STATE_STOPPED;
                    }
                }
            }
            _ => {
                if af.state == STATE_FORWARD {
                    af.frame += step;
                    if end <= af.frame {
                        af.frame = first_f;
                    }
                } else {
                    af.frame -= step;
                    if af.frame < first_f {
                        af.frame = last as f32;
                    }
                }
            }
        }
    }

    fn current_frame(&self, af: &mut AnimFrame) -> FrameRef {
        if af.state == STATE_RESTART_FORWARD || af.state == STATE_RESTART_BACKWARD {
            self.next_frame(af);
        }
        let s = &self.slots[af.anim as usize];
        s.frames[af.frame as i32 as usize]
    }

    // Draws the current frame with its hotspot so that (x, y) lands on the frame origin
    pub fn draw(&self, af: &mut AnimFrame, x: i32, y: i32, bitmaps: &BitmapManager, display: &mut Display) {
        let f = self.current_frame(af);
        let tex = bitmaps.get(f.bitmap);
        let left = x - f.x as i32 + tex.hot_x;
        let top = y - f.y as i32 + tex.hot_y;
        let dest = Rect::new(left, top, left + tex.width as i16 as i32, top + tex.height as i16 as i32);
        let src = Rect::new(0, 0, tex.width as i16 as i32, tex.height as i16 as i32);
        display.blit(tex, &dest, &src, tex.key.is_some());
    }

    // Draws a sub rectangle of the current frame, narrowing the clip to the bitmap width first
    pub fn draw_clip(
        &self,
        af: &mut AnimFrame,
        x: i16,
        y: i16,
        clip: &mut Rect,
        bitmaps: &BitmapManager,
        display: &mut Display,
    ) {
        let f = self.current_frame(af);
        let tex = bitmaps.get(f.bitmap);
        let width = tex.width as i16 as i32;
        if width < clip.right {
            clip.right = width;
        }
        let dest = Rect::new(
            x as i32,
            y as i32,
            x as i32 + clip.width(),
            y as i32 + clip.height(),
        );
        display.blit(tex, &dest, clip, tex.key.is_some());
    }

    // Replaces the palette of every retained frame texture of an animation
    pub fn set_palette(&self, handle: u16, palette: &Palette24, bitmaps: &mut BitmapManager) {
        for f in &self.slots[handle as usize].frames {
            bitmaps.get_mut(f.bitmap).set_palette(palette);
        }
    }

    // Reads the retained palette of the first frame texture
    pub fn palette(&self, handle: u16, bitmaps: &BitmapManager) -> Option<Palette24> {
        let f = self.slots[handle as usize].frames.first()?;
        bitmaps.get(f.bitmap).palette()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_with(loop_mode: i16, frames: usize) -> AnimManager {
        let mut m = AnimManager::default();
        m.slots.push(AnimRecord {
            name: "t".into(),
            in_use: true,
            loop_mode,
            fps: 30,
            frames: (0..frames).map(|_| FrameRef { x: 0, y: 0, bitmap: 0 }).collect(),
            color_key: 0,
            transparent: false,
            bbox: [0; 4],
            first: 0,
            last: (frames - 1) as u16,
        });
        m
    }

    #[test]
    fn once_stops_on_last_frame() {
        let m = manager_with(LOOP_ONCE, 3);
        let mut af = AnimFrame { frame: 0.0, anim: 0, state: STATE_FORWARD, fps: 0 };
        m.next_frame(&mut af);
        m.next_frame(&mut af);
        assert_eq!((af.frame, af.state), (2.0, STATE_FORWARD));
        m.next_frame(&mut af);
        assert_eq!((af.frame, af.state), (2.0, STATE_STOPPED));
    }

    #[test]
    fn pingpong_turns_with_fraction() {
        let m = manager_with(LOOP_PINGPONG, 2);
        let mut af = AnimFrame { frame: 1.0, anim: 0, state: STATE_FORWARD, fps: 0 };
        m.next_frame(&mut af);
        assert_eq!(af.state, STATE_BACKWARD);
        assert_eq!(af.frame, (1.0f64 + 0.99) as f32);
    }

    #[test]
    fn wrap_loops_back_to_first() {
        let m = manager_with(LOOP_WRAP, 2);
        let mut af = AnimFrame { frame: 1.5, anim: 0, state: STATE_FORWARD, fps: 15 };
        m.next_frame(&mut af);
        assert_eq!(af.frame, 0.0);
    }

    #[test]
    fn restart_states_resolve_before_stepping() {
        let m = manager_with(LOOP_WRAP, 4);
        let mut af = AnimFrame { frame: 2.0, anim: 0, state: STATE_RESTART_BACKWARD, fps: 0 };
        m.next_frame(&mut af);
        assert_eq!((af.frame, af.state), (3.0, STATE_BACKWARD));
    }
}
