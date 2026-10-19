use avmanager::anim::{AnimFrame, STATE_RESTART_FORWARD};

use crate::av::Av;

pub const NO_HANDLE: u16 = 0xffff;

// The single mouse cursor sprite with its clamped position and animation
#[derive(Clone, Debug)]
pub struct Cursor {
    pub x: i16,
    pub y: i16,
    pub visible: bool,
    pub anim: AnimFrame,
}

impl Default for Cursor {
    fn default() -> Self {
        Self { x: 0, y: 0, visible: false, anim: AnimFrame { frame: 0.0, anim: NO_HANDLE, state: 0, fps: 0 } }
    }
}

impl Cursor {
    // Moves by a delta and clamps to the 10..630 by 10..470 box
    pub fn move_by(&mut self, dx: i16, dy: i16) {
        self.x = self.x.wrapping_add(dx);
        if self.x < 0x277 {
            if self.x < 10 {
                self.x = 10;
            }
        } else {
            self.x = 0x276;
        }
        self.y = self.y.wrapping_add(dy);
        if self.y < 0x1d7 {
            if self.y < 10 {
                self.y = 10;
            }
        } else {
            self.y = 0x1d6;
        }
    }

    // Switches the cursor art, restarting it from the first frame when it changes
    pub fn set_anim(&mut self, av: &Av, handle: u16) {
        if self.anim.anim != handle {
            self.anim.anim = handle;
            self.anim.state = STATE_RESTART_FORWARD;
            av.get_anim_fps(handle, &mut self.anim.fps);
        }
    }

    pub fn set_pos(&mut self, x: i16, y: i16) {
        self.x = x;
        self.y = y;
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    // Draws the cursor at its position then steps its animation
    pub fn draw(&mut self, av: &mut Av) {
        if self.visible {
            av.draw_anim(&mut self.anim, self.x as i32, self.y as i32);
            av.get_anim_next_frame(&mut self.anim);
        }
    }
}
