use avmanager::anim::{AnimFrame, STATE_RESTART_FORWARD};
use avmanager::crt::{Rand, ftol};

use crate::av::Av;
use crate::host::Host;

// One background tile template or active tile of a parallax strip
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tile {
    pub anim: AnimFrame,
    pub x: f32,
    pub y: f32,
    pub width: i32,
    pub height: i32,
    pub repeat: i32,
    pub index: i32,
    pub hotspot_x: i32,
}

// Endless horizontally scrolling band of tiles chosen randomly or in sequence
#[derive(Clone, Debug)]
pub struct ScrollStrip {
    pub factor: f32,
    pub tiles: Vec<Tile>,
    pub active: Vec<Tile>,
    pub y: f32,
    pub stopped: bool,
    pub seq_index: i16,
    pub rand_start: i32,
}

impl Default for ScrollStrip {
    fn default() -> Self {
        Self {
            factor: 1.0,
            tiles: Vec::new(),
            active: Vec::new(),
            y: 0.0,
            stopped: false,
            seq_index: 0,
            rand_start: 0,
        }
    }
}

impl ScrollStrip {
    pub fn set_speed_factor(&mut self, factor: f32) {
        self.factor = factor;
    }

    pub fn set_y(&mut self, y: f32) {
        self.y = y;
    }

    pub fn set_random_start(&mut self, first_random_index: i32) {
        self.rand_start = first_random_index;
    }

    pub fn set_stopped(&mut self, stopped: bool) {
        self.stopped = stopped;
    }

    // Registers a tile template from a BAM, false when the art is missing
    pub async fn add_tile<H: Host>(&mut self, av: &mut Av, host: &mut H, name: &str, width_override: i32, repeat: i32) -> bool {
        let Some(handle) = av.get_anim(host, name).await else { return false };
        let mut anim = AnimFrame { frame: 0.0, anim: handle, state: STATE_RESTART_FORWARD, fps: 0 };
        av.get_anim_fps(handle, &mut anim.fps);
        let (mut hx, mut hy) = (0i32, 0i32);
        av.get_hot_spot(handle, &mut hx, &mut hy);
        let b = av.get_anim_bbox(handle);
        let mut width = b[2] - b[0];
        if width_override != 0 {
            width = width_override;
        }
        let index = self.tiles.len() as i32;
        self.tiles.push(Tile { anim, x: 0.0, y: 0.0, width, height: b[3] - b[1], repeat, index, hotspot_x: hx });
        true
    }

    // Appends one tile to the right end, consuming one random number every call
    pub fn spawn(&mut self, rand: &mut Rand, mut start_x: f32) {
        let count = self.tiles.len() as i32;
        let mut r = rand.ranged(self.rand_start, count - self.rand_start) as usize;
        let mut rep = self.tiles[r].repeat;
        if self.seq_index >= 0 {
            r = self.seq_index as usize;
            self.seq_index += 1;
            if self.seq_index as i32 >= count {
                self.seq_index = -1;
            }
        }
        if let Some(last) = self.active.last() {
            start_x = ((last.width - last.hotspot_x) as u32) as f32 + last.x;
            if last.repeat != 0 {
                rep = last.repeat - 1;
                r = last.index as usize;
            }
        }
        let mut tile = self.tiles[r];
        tile.repeat = rep;
        tile.x = tile.hotspot_x as f32 + start_x;
        tile.y = self.y;
        self.active.push(tile);
    }

    fn right_edge(&self) -> f32 {
        match self.active.last() {
            Some(last) => (last.width as u32 as f32 + last.x) - last.hotspot_x as f32,
            None => 0.0,
        }
    }

    // Fills the strip to the screen edge, drops tiles that scrolled off, and draws the rest
    pub fn fill_and_draw(&mut self, av: &mut Av, rand: &mut Rand) {
        while !self.stopped {
            if 640.0 <= self.right_edge() {
                self.seq_index = -1;
                break;
            }
            self.spawn(rand, 0.0);
        }
        while let Some(first) = self.active.first() {
            if first.width as u32 as f32 + first.x < 0.0 {
                self.active.remove(0);
            } else {
                break;
            }
        }
        for tile in &mut self.active {
            let y = ftol(tile.y as f64);
            let x = ftol(tile.x as f64);
            av.draw_anim(&mut tile.anim, x, y);
        }
    }

    // Scrolls every tile by the integer displacement of the first tile and steps their frames
    pub fn scroll(&mut self, av: &Av, dx: f32) {
        let mut d = dx * self.factor;
        if let Some(first) = self.active.first_mut() {
            let before = ftol(first.x as f64);
            first.x -= d;
            d = (before - ftol(first.x as f64)) as f32;
            av.get_anim_next_frame(&mut first.anim);
        }
        for tile in self.active.iter_mut().skip(1) {
            tile.x -= d;
            av.get_anim_next_frame(&mut tile.anim);
        }
    }
}
