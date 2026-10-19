use avmanager::anim::{AnimFrame, STATE_RESTART_BACKWARD, STATE_RESTART_FORWARD, STATE_STOPPED};
use avmanager::crt::ftol;
use avmanager::rect::Rect;
use avmanager::texture::Palette24;

use crate::actors::{
    EVENT_ANIM_END, EVENT_INIT_OBJECT, EVENT_MOUSE_HIT, EVENT_TARGET_REACHED, EVENT_TIMER, FACING_FORCE_NONE,
    FACING_NONE, NO_TARGET,
};
use crate::ctx::{
    Ctx, KEY_DOWN, KEY_LCTRL, KEY_LSHIFT, KEY_RCTRL, KEY_RIGHT, KEY_RSHIFT, KEY_SPACE, KEY_UP, MOUSE_LEFT,
    RESULT_CONTINUE, RESULT_QUIT, STATE_CHALLENGE, STATE_HOME_WORLD, STATE_JUMPING, STATE_RUNNING, STATE_THROWING,
    Screen,
};
use crate::host::Host;
use crate::player::PlayerData;
use crate::scene::{HOVER_HIDDEN, HOVER_NORMAL, HOVER_PRESSED};

const ACTOR_CRUNCHLING: i32 = 0;
const ACTOR_CAPN: i32 = 1;
const ACTOR_TUBE: i32 = 2;
const ACTOR_PILE: i32 = 3;
const ACTOR_BOX: i32 = 4;
const ACTOR_DOOR: i32 = 5;
const ACTOR_CASTER: i32 = 6;

const OBJ_K1: i32 = 1;
const OBJ_K2: i32 = 2;
const OBJ_K3: i32 = 3;
const OBJ_NAME: i32 = 4;
const OBJ_SPEED: i32 = 5;
const OBJ_STRENGTH: i32 = 6;
const OBJ_JUMP: i32 = 7;
const OBJ_CHALLENGE: i32 = 8;
const OBJ_TIPS: i32 = 9;
const OBJ_FOOD_GLOW: i32 = 10;
const OBJ_FEEDER_GLOW: i32 = 0xb;
const OBJ_QUIT: i32 = 0xc;
const OBJ_CASTER_GLOW: i32 = 0xd;
const OBJ_ONLINE: i32 = 0xe;

const CL_IDLE: u32 = 0;
const CL_RUN_TO_PILE: u32 = 1;
const CL_RUN_TO_FEEDER: u32 = 2;
const CL_WALK_RIGHT: u32 = 3;
const CL_WALK_LEFT: u32 = 4;
const CL_PLAYING: u32 = 5;
const CL_QTIP_OUT: u32 = 6;
const CL_QTIP_EAR_LOOP: u32 = 7;
const CL_QTIP_DISGUST: u32 = 8;
const CL_QTIP_THROW: u32 = 9;
const CL_TRANSFORM: u32 = 10;
const CL_LEAVING: u32 = 0xb;
const CL_SLEEP: u32 = 0xd;
const CL_SPEECH_RUN: u32 = 0xe;

const SEQ_NONE: u32 = 0;
const SEQ_FIRST_VISIT: u32 = 1;
const SEQ_TRANSFORMED: u32 = 2;
const SEQ_SPEED: u32 = 3;
const SEQ_JUMP: u32 = 4;
const SEQ_STRENGTH: u32 = 5;
const SEQ_CHALLENGE: u32 = 6;

// Indices into the anim handle table that lived at DAT_0042bd60 in the original
const ANIM_BLINK: u16 = 0x00;
const ANIM_WALK_LEFT: u16 = 0x01;
const ANIM_TURN_LEFT: u16 = 0x02;
const ANIM_WALK_RIGHT: u16 = 0x03;
const ANIM_TURN_RIGHT: u16 = 0x04;
const ANIM_RUN_LEFT: u16 = 0x05;
const ANIM_RUN_RIGHT: u16 = 0x06;
const ANIM_EAT: u16 = 0x07;
const ANIM_HUNGRY: u16 = 0x08;
const ANIM_NEED_PRAISE: u16 = 0x09;
const ANIM_NO_FOOD: u16 = 0x0a;
const ANIM_BLINK_PREVIOUS: u16 = 0x0b;
const ANIM_TRANSFORM: u16 = 0x0c;
const ANIM_SLEEP: u16 = 0x0d;
const ANIM_TUBE_LEFT: u16 = 0x0e;
const ANIM_TUBE_MID: u16 = 0x0f;
const ANIM_TUBE_RIGHT: u16 = 0x10;
const ANIM_TICKLE: u16 = 0x11;
const ANIM_DANCE: u16 = 0x12;
const ANIM_QTIP_OUT: u16 = 0x13;
const ANIM_QTIP_EAR_LOOP: u16 = 0x14;
const ANIM_QTIP_DISGUST: u16 = 0x15;
const ANIM_QTIP_THROW: u16 = 0x16;
const ANIM_CRUNCH_PILE: u16 = 0x17;
const ANIM_PEANUT_PILE: u16 = 0x18;
const ANIM_BERRY_PILE: u16 = 0x19;
const ANIM_DOOR: u16 = 0x1a;
const ANIM_BOX_1: u16 = 0x1b;
const ANIM_BOX_2: u16 = 0x1c;
const ANIM_BOX_3: u16 = 0x1d;
const ANIM_CASTER_2: u16 = 0x1e;
const ANIM_CASTER_1: u16 = 0x1f;
const ANIM_CAPN_013: u16 = 0x20;
const ANIM_CAPN_021: u16 = 0x21;
const ANIM_CAPN_008: u16 = 0x22;
const ANIM_CAPN_010: u16 = 0x23;
const ANIM_MOUSE: u16 = 0x24;
const ANIM_MOUSE_FEATHER: u16 = 0x25;
const ANIM_MOUSE_HAND: u16 = 0x26;
const ANIM_MOUSE_QTIP: u16 = 0x27;
const ANIM_SHADOW: u16 = 0x28;
const ANIM_BAR_DARK: u16 = 0x29;
const ANIM_MINIBAR: u16 = 0x2a;
const ANIM_COUNT: usize = 0x2b;

const KEY_7: i32 = 8;
const KEY_0: i32 = 0xb;
const KEY_T: i32 = 0x14;
const KEY_S: i32 = 0x1f;
const KEY_J: i32 = 0x24;

const CASTER_SHOWN_Y: f32 = 30.0;
const CASTER_STEP: f32 = 3.0;
const CASTER_HIDDEN_Y: f32 = -220.0;
const PILE_APPROACH: f32 = 65.0;
const GROUND_Y: f32 = 168.0;

const COL_OFF_X: usize = 0;
const COL_OFF_Y: usize = 1;
const COL_HAND_MIN: usize = 2;
const COL_HAND_MAX: usize = 3;
const COL_FEATHER_MIN: usize = 4;

// Draw offset and tool zones per form, DAT_00429428 dumped from the binary, row is type plus three per stage
const POS_TABLE: [[i32; 5]; 9] = [
    [41, 89, 240, 265, 305],
    [41, 89, 240, 265, 305],
    [41, 89, 240, 265, 305],
    [33, 60, 220, 245, 305],
    [30, 61, 220, 245, 305],
    [25, 62, 220, 245, 305],
    [22, 14, 165, 200, 300],
    [15, 16, 165, 200, 300],
    [18, 2, 165, 200, 300],
];

// Row of the position table, the previous stage while a transform is pending
fn pos_table_row(player: &PlayerData) -> usize {
    player.anim_index(player.transform_pending != 0) as usize
}

// Left edge of the dark overlay on a food or happiness bar, 25 pixels per level capped at 128
fn bar_overlay_left(level: i16) -> u16 {
    let raw = (level as i32 * 0x19) as u16;
    raw.min(0x80)
}

// Clip width of a stat minibar, three pixels per point capped at 60, absent for an empty stat
fn minibar_width(stat: i32) -> Option<u16> {
    let raw = ((stat as i16 as i32) * 3) as u16;
    if raw == 0 {
        return None;
    }
    Some(raw.min(0x3c))
}

// The crunchling's home with feeding, tools, training doors and the Cap'n sequences
pub struct HomeWorldScreen {
    mouse_latched: bool,
    loop_channel: u32,
    voice_channel: u32,
    last_walk_state: u32,
    crunchling_state: u32,
    ear_loop_count: i32,
    pile_x: f32,
    cursor_index: u16,
    pile_index: u16,
    box_index: u16,
    tube_index: u16,
    box_opening: bool,
    door_open: bool,
    result: u32,
    pending_state: u32,
    caster_wav: String,
    caster_dir: i32,
    caster_y: f32,
    hungry_nags: u32,
    praise_nags: u32,
    capn_sequence: u32,
    capn_step: u32,
    waiting_for_voice: bool,
    sfx_lever: u16,
    sfx_eat: u16,
    sfx_pipe: u16,
    sfx_giggle: u16,
    sfx_uhuh: u16,
    sfx_qtip: u16,
    sfx_door_open: u16,
    sfx_door_close: u16,
    sfx_sad: u16,
    sfx_hungry: u16,
    sfx_praise: u16,
    sfx_transform: u16,
    sfx_run_fast: u16,
    sfx_yahoo: u16,
    anims: [u16; ANIM_COUNT],
}

impl Default for HomeWorldScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl HomeWorldScreen {
    pub fn new() -> Self {
        Self {
            mouse_latched: false,
            loop_channel: 0,
            voice_channel: 0,
            last_walk_state: 0,
            crunchling_state: 0,
            ear_loop_count: 0,
            pile_x: 0.0,
            cursor_index: 0,
            pile_index: 0,
            box_index: 0,
            tube_index: 0,
            box_opening: false,
            door_open: false,
            result: 0,
            pending_state: 0,
            caster_wav: String::new(),
            caster_dir: 0,
            caster_y: 0.0,
            hungry_nags: 0,
            praise_nags: 0,
            capn_sequence: 0,
            capn_step: 0,
            waiting_for_voice: false,
            sfx_lever: 0,
            sfx_eat: 0,
            sfx_pipe: 0,
            sfx_giggle: 0,
            sfx_uhuh: 0,
            sfx_qtip: 0,
            sfx_door_open: 0,
            sfx_door_close: 0,
            sfx_sad: 0,
            sfx_hungry: 0,
            sfx_praise: 0,
            sfx_transform: 0,
            sfx_run_fast: 0,
            sfx_yahoo: 0,
            anims: [0; ANIM_COUNT],
        }
    }

    fn anim(&self, index: u16) -> u16 {
        self.anims[index as usize]
    }

    async fn load_sfx<H: Host>(ctx: &mut Ctx<H>, name: &str) -> Option<u16> {
        ctx.av.get_sfx(&mut ctx.host, name, 0, false).await
    }

    // Stores a plain animation in the handle table, false when the file is missing
    async fn load_anim<H: Host>(&mut self, ctx: &mut Ctx<H>, index: u16, name: &str) -> bool {
        match ctx.av.get_anim(&mut ctx.host, name).await {
            Some(h) => {
                self.anims[index as usize] = h;
                true
            }
            None => false,
        }
    }

    // Stores a stage and type named crunchling animation drawn with the recoloured palette
    async fn load_crunchling_anim<H: Host>(
        &mut self,
        ctx: &mut Ctx<H>,
        index: u16,
        suffix: &str,
        keep_baby_type: bool,
        palette: &Palette24,
    ) -> bool {
        let name = ctx.player.anim_name(suffix, keep_baby_type, false);
        match ctx.av.get_anim_use_palette(&mut ctx.host, &name, palette).await {
            Some(h) => {
                self.anims[index as usize] = h;
                true
            }
            None => false,
        }
    }

    // Loads the eight shared effects then the stage specific voice set, false on any miss
    async fn load_sfx_set<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        if Self::load_sfx(ctx, "CS_Button.wav").await.is_none() {
            return false;
        }
        let Some(h) = Self::load_sfx(ctx, "CS_lever.wav").await else { return false };
        self.sfx_lever = h;
        let Some(h) = Self::load_sfx(ctx, "CS_pipe.wav").await else { return false };
        self.sfx_pipe = h;
        let Some(h) = Self::load_sfx(ctx, "cs_uhuh.wav").await else { return false };
        self.sfx_uhuh = h;
        let Some(h) = Self::load_sfx(ctx, "CS_yahoo.wav").await else { return false };
        self.sfx_yahoo = h;
        let Some(h) = Self::load_sfx(ctx, "cs_dooropen.wav").await else { return false };
        self.sfx_door_open = h;
        let Some(h) = Self::load_sfx(ctx, "cs_runfast.wav").await else { return false };
        self.sfx_run_fast = h;
        let Some(h) = Self::load_sfx(ctx, "cs_doorclose.wav").await else { return false };
        self.sfx_door_close = h;
        if ctx.player.stage == 0 {
            let Some(h) = Self::load_sfx(ctx, "CS_b_giggle.wav").await else { return false };
            self.sfx_giggle = h;
            let Some(h) = Self::load_sfx(ctx, "CS_b_hungry.wav").await else { return false };
            self.sfx_hungry = h;
            let Some(h) = Self::load_sfx(ctx, "CS_b_sad.wav").await else { return false };
            self.sfx_sad = h;
            let Some(h) = Self::load_sfx(ctx, "CS_b_eat.wav").await else { return false };
            self.sfx_eat = h;
            let Some(h) = Self::load_sfx(ctx, "CS_b_praise.wav").await else { return false };
            self.sfx_praise = h;
        } else if ctx.player.stage == 1 {
            let Some(h) = Self::load_sfx(ctx, "CS_t_hungry.wav").await else { return false };
            self.sfx_hungry = h;
            let Some(h) = Self::load_sfx(ctx, "CS_t_sad.wav").await else { return false };
            self.sfx_sad = h;
            let Some(h) = Self::load_sfx(ctx, "CS_t_eat.wav").await else { return false };
            self.sfx_eat = h;
            let Some(h) = Self::load_sfx(ctx, "CS_transform1.wav").await else { return false };
            self.sfx_transform = h;
            let Some(h) = Self::load_sfx(ctx, "CS_t_praise.wav").await else { return false };
            self.sfx_praise = h;
        } else if ctx.player.stage == 2 {
            let Some(h) = Self::load_sfx(ctx, "CS_a_hungry.wav").await else { return false };
            self.sfx_hungry = h;
            let Some(h) = Self::load_sfx(ctx, "CS_a_sad.wav").await else { return false };
            self.sfx_sad = h;
            let Some(h) = Self::load_sfx(ctx, "CS_a_eat.wav").await else { return false };
            self.sfx_eat = h;
            let Some(h) = Self::load_sfx(ctx, "CS_transform2.wav").await else { return false };
            self.sfx_transform = h;
            let Some(h) = Self::load_sfx(ctx, "CS_a_praise.wav").await else { return false };
            self.sfx_praise = h;
        }
        true
    }

    // Loads every animation after the blink palette is known, false on any miss
    async fn load_anim_set<H: Host>(&mut self, ctx: &mut Ctx<H>, palette: &Palette24) -> bool {
        if !self.load_crunchling_anim(ctx, ANIM_WALK_LEFT, "hm_walk_left", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_TURN_LEFT, "hm_turn_left", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_WALK_RIGHT, "hm_walk_right", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_TURN_RIGHT, "hm_turn_right", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_RUN_RIGHT, "hm_run_right", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_RUN_LEFT, "hm_run_left", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_NO_FOOD, "no_food", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_HUNGRY, "hungry", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_EAT, "eat", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_NEED_PRAISE, "need_praise", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_SLEEP, "sleep", false, palette).await {
            return false;
        }
        if !self.load_crunchling_anim(ctx, ANIM_DANCE, "dance", false, palette).await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_TUBE_LEFT, "tube_left").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_TUBE_MID, "tube_mid").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_TUBE_RIGHT, "tube_right").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_CRUNCH_PILE, "crunch_pile").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_PEANUT_PILE, "peanut_pile").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_BERRY_PILE, "berry_pile").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_BOX_1, "box_1").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_BOX_2, "box_2").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_BOX_3, "box_3").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_DOOR, "door").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_BAR_DARK, "f&hBar_dark").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_MINIBAR, "minibar").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_CASTER_2, "hw_caster_2").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_CASTER_1, "hw_caster_1").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_MOUSE, "mouse").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_MOUSE_HAND, "mouse_hand").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_MOUSE_QTIP, "mouse_qtip").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_SHADOW, "shadow").await {
            return false;
        }
        if ctx.player.stage == 0 {
            if !self.load_crunchling_anim(ctx, ANIM_TICKLE, "tickle", false, palette).await {
                return false;
            }
            if !self.load_anim(ctx, ANIM_MOUSE_FEATHER, "mouse_feather").await {
                return false;
            }
        } else if ctx.player.stage == 1 {
            let Some(h) = Self::load_sfx(ctx, "cs_qtip.wav").await else { return false };
            self.sfx_qtip = h;
            if !self.load_crunchling_anim(ctx, ANIM_QTIP_OUT, "qtip_out", false, palette).await {
                return false;
            }
            if !self.load_crunchling_anim(ctx, ANIM_QTIP_EAR_LOOP, "qtip_ear_loop", false, palette).await {
                return false;
            }
            if !self.load_crunchling_anim(ctx, ANIM_QTIP_DISGUST, "qtip_disgust", false, palette).await {
                return false;
            }
            if !self.load_crunchling_anim(ctx, ANIM_QTIP_THROW, "qtip_throw", false, palette).await {
                return false;
            }
        }
        true
    }

    fn set_crunchling_anim<H: Host>(&self, ctx: &mut Ctx<H>, index: u16, notify: bool, mode: i16) {
        let h = self.anim(index);
        ctx.actors.set_anims(&ctx.av, ACTOR_CRUNCHLING, h, h, notify, mode);
    }

    fn set_crunchling_run<H: Host>(&self, ctx: &mut Ctx<H>) {
        let right = self.anim(ANIM_RUN_RIGHT);
        let left = self.anim(ANIM_RUN_LEFT);
        ctx.actors.set_anims(&ctx.av, ACTOR_CRUNCHLING, right, left, false, STATE_RESTART_FORWARD);
    }

    fn set_capn_anim<H: Host>(&self, ctx: &mut Ctx<H>, index: u16) {
        let h = self.anim(index);
        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, h, h, false, STATE_RESTART_FORWARD);
    }

    async fn play_voice<H: Host>(&mut self, ctx: &mut Ctx<H>, name: &str) {
        self.voice_channel = ctx.av.play_stream_sound(&mut ctx.host, name, false, self.voice_channel).await;
    }

    // Advances the scripted Cap'n visit one step for the running sequence id
    async fn capn_sequence_step<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        match self.capn_sequence {
            SEQ_FIRST_VISIT => match self.capn_step {
                0 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_008);
                    ctx.actors.set_pos(ACTOR_CAPN, 90.0, 145.0);
                    ctx.actors.set_timer(ACTOR_CAPN, 0x1e);
                    self.capn_step += 1;
                }
                1 => {
                    self.play_voice(ctx, "060.wav").await;
                    self.set_capn_anim(ctx, ANIM_CAPN_013);
                    self.waiting_for_voice = true;
                    self.capn_step += 1;
                }
                2 => {
                    ctx.scene.set_hover_by_id(OBJ_FEEDER_GLOW, HOVER_PRESSED);
                    self.play_voice(ctx, "060a.wav").await;
                    self.waiting_for_voice = true;
                    self.capn_step += 1;
                }
                3 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_008);
                    ctx.actors.set_timer(ACTOR_CAPN, 0x1e);
                    self.capn_step += 1;
                }
                4 => {
                    ctx.scene.set_hover_by_id(OBJ_FEEDER_GLOW, HOVER_HIDDEN);
                    self.play_voice(ctx, "066.wav").await;
                    self.waiting_for_voice = true;
                    self.set_capn_anim(ctx, ANIM_CAPN_021);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, 300);
                    ctx.actors.set_arc(ACTOR_CAPN, 4.0, 4.0);
                    self.capn_step += 1;
                }
                5 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_013);
                    self.capn_step += 1;
                }
                6 => {
                    ctx.scene.set_hover_by_id(OBJ_FOOD_GLOW, HOVER_PRESSED);
                    self.play_voice(ctx, "066a.wav").await;
                    self.waiting_for_voice = true;
                    self.capn_step += 1;
                }
                7 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_008);
                    ctx.actors.set_timer(ACTOR_CAPN, 0x3c);
                    self.capn_step += 1;
                }
                8 => {
                    ctx.scene.set_hover_by_id(OBJ_FOOD_GLOW, HOVER_HIDDEN);
                    ctx.scene.set_hover_by_id(OBJ_CASTER_GLOW, HOVER_PRESSED);
                    self.play_voice(ctx, "066b.wav").await;
                    self.waiting_for_voice = true;
                    self.set_capn_anim(ctx, ANIM_CAPN_013);
                    self.capn_step += 1;
                }
                9 => {
                    ctx.scene.set_hover_by_id(OBJ_CASTER_GLOW, HOVER_HIDDEN);
                    self.set_capn_anim(ctx, ANIM_CAPN_010);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, -0x3c);
                    ctx.actors.set_arc(ACTOR_CAPN, 7.0, 6.0);
                    self.set_door(ctx, true);
                    self.capn_step += 1;
                }
                10 => {
                    ctx.actors.set_visible(ACTOR_CAPN, false);
                    self.set_door(ctx, false);
                    self.capn_step += 1;
                    ctx.cursor.set_visible(true);
                    self.capn_sequence = SEQ_NONE;
                    self.idle(ctx);
                    ctx.scene.set_hover_by_id(OBJ_FOOD_GLOW, HOVER_HIDDEN);
                    ctx.scene.set_hover_by_id(OBJ_FEEDER_GLOW, HOVER_HIDDEN);
                }
                _ => {}
            },
            SEQ_TRANSFORMED => match self.capn_step {
                0 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_008);
                    ctx.actors.set_pos(ACTOR_CAPN, -60.0, 145.0);
                    ctx.actors.set_timer(ACTOR_CAPN, 0x1e);
                    self.capn_step += 1;
                }
                1 => {
                    self.set_door(ctx, true);
                    if ctx.player.stage == 1 {
                        self.play_voice(ctx, "003.wav").await;
                    } else {
                        self.play_voice(ctx, "005.wav").await;
                    }
                    self.waiting_for_voice = true;
                    self.set_capn_anim(ctx, ANIM_CAPN_021);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, 200);
                    ctx.actors.set_arc(ACTOR_CAPN, 4.0, 4.0);
                    self.capn_step += 1;
                }
                2 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_013);
                    self.capn_step += 1;
                }
                3 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_008);
                    ctx.actors.set_timer(ACTOR_CRUNCHLING, 0x1e);
                    self.capn_step += 1;
                }
                4 => {
                    self.play_voice(ctx, "004.wav").await;
                    self.waiting_for_voice = true;
                    self.set_capn_anim(ctx, ANIM_CAPN_013);
                    self.capn_step += 1;
                }
                5 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_010);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, -0x3c);
                    ctx.actors.set_arc(ACTOR_CAPN, 4.0, 4.0);
                    self.set_door(ctx, true);
                    self.capn_step += 1;
                }
                6 => {
                    ctx.actors.set_visible(ACTOR_CAPN, false);
                    self.set_door(ctx, false);
                    self.capn_step += 1;
                    ctx.cursor.set_visible(true);
                    self.capn_sequence = SEQ_NONE;
                    self.idle(ctx);
                }
                _ => {}
            },
            SEQ_SPEED | SEQ_JUMP | SEQ_STRENGTH | SEQ_CHALLENGE => match self.capn_step {
                0 => {
                    ctx.cursor.set_visible(false);
                    ctx.actors.set_visible(ACTOR_CAPN, true);
                    self.set_capn_anim(ctx, ANIM_CAPN_008);
                    ctx.actors.set_pos(ACTOR_CAPN, -60.0, 145.0);
                    ctx.actors.set_timer(ACTOR_CAPN, 0x14);
                    self.set_door(ctx, true);
                    self.set_crunchling_run(ctx);
                    ctx.actors.set_y(ACTOR_CRUNCHLING, GROUND_Y);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, 300);
                    ctx.actors.set_arc(ACTOR_CRUNCHLING, 12.0, 8.0);
                    ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                    self.crunchling_state = CL_SPEECH_RUN;
                    self.capn_step += 1;
                }
                1 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_021);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, 0x46);
                    ctx.actors.set_arc(ACTOR_CAPN, 6.0, 4.0);
                    self.capn_step += 1;
                }
                2 => {
                    match self.capn_sequence {
                        SEQ_SPEED => self.play_voice(ctx, "030.wav").await,
                        SEQ_JUMP => self.play_voice(ctx, "035.wav").await,
                        SEQ_STRENGTH => self.play_voice(ctx, "038.wav").await,
                        SEQ_CHALLENGE => self.play_voice(ctx, "044.wav").await,
                        _ => {}
                    }
                    self.waiting_for_voice = true;
                    self.set_capn_anim(ctx, ANIM_CAPN_021);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, 0xa0);
                    ctx.actors.set_arc(ACTOR_CAPN, 5.0, 4.0);
                    self.capn_step += 1;
                }
                3 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_013);
                    self.capn_step += 1;
                }
                4 => {
                    self.set_capn_anim(ctx, ANIM_CAPN_010);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, -0x3c);
                    ctx.actors.set_arc(ACTOR_CAPN, 6.0, 4.0);
                    self.capn_step += 1;
                    match self.capn_sequence {
                        SEQ_SPEED => self.leave_to_game(ctx, STATE_RUNNING),
                        SEQ_JUMP => self.leave_to_game(ctx, STATE_JUMPING),
                        SEQ_STRENGTH => self.leave_to_game(ctx, STATE_THROWING),
                        SEQ_CHALLENGE => self.leave_to_game(ctx, STATE_CHALLENGE),
                        _ => {}
                    }
                }
                _ => self.capn_sequence = SEQ_NONE,
            },
            _ => {}
        }
    }

    // Reacts to a click on a WAM object, returning 2 for Quit and online and 1 otherwise
    async fn on_object_clicked<H: Host>(&mut self, ctx: &mut Ctx<H>, obj: i32) -> u32 {
        let mut result = RESULT_CONTINUE;
        let mut fed = false;
        match obj {
            OBJ_K1 => {
                self.pile_index = ANIM_CRUNCH_PILE;
                self.box_index = ANIM_BOX_1;
                self.tube_index = ANIM_TUBE_LEFT;
                fed = true;
                let h = self.anim(self.pile_index);
                ctx.actors.set_anims(&ctx.av, ACTOR_PILE, h, h, true, STATE_RESTART_BACKWARD);
                self.pile_x = 233.0;
                ctx.actors.set_pos(ACTOR_PILE, self.pile_x, 375.0);
            }
            OBJ_K2 => {
                self.pile_index = ANIM_PEANUT_PILE;
                self.box_index = ANIM_BOX_2;
                self.tube_index = ANIM_TUBE_MID;
                fed = true;
                let h = self.anim(self.pile_index);
                ctx.actors.set_anims(&ctx.av, ACTOR_PILE, h, h, true, STATE_RESTART_BACKWARD);
                self.pile_x = 269.0;
                ctx.actors.set_pos(ACTOR_PILE, self.pile_x, 375.0);
            }
            OBJ_K3 => {
                self.pile_index = ANIM_BERRY_PILE;
                self.box_index = ANIM_BOX_3;
                self.tube_index = ANIM_TUBE_RIGHT;
                fed = true;
                let h = self.anim(self.pile_index);
                ctx.actors.set_anims(&ctx.av, ACTOR_PILE, h, h, true, STATE_RESTART_BACKWARD);
                self.pile_x = 311.0;
                ctx.actors.set_pos(ACTOR_PILE, self.pile_x, 375.0);
            }
            OBJ_SPEED => self.capn_sequence = SEQ_SPEED,
            OBJ_STRENGTH => self.capn_sequence = SEQ_STRENGTH,
            OBJ_JUMP => self.capn_sequence = SEQ_JUMP,
            OBJ_CHALLENGE if ctx.player.stage > 1 => self.capn_sequence = SEQ_CHALLENGE,
            OBJ_TIPS => {
                if ctx.player.tips_index == 0 {
                    self.caster_speak(ctx, "014.wav");
                } else if ctx.player.tips_index == 1 {
                    self.caster_speak(ctx, "013.wav");
                } else if ctx.player.tips_index == 2 {
                    self.caster_speak(ctx, "012.wav");
                }
                ctx.player.tips_index += 1;
                if ctx.player.tips_index > 2 {
                    ctx.player.tips_index = 0;
                }
            }
            OBJ_QUIT => result = RESULT_QUIT,
            OBJ_ONLINE => {
                result = RESULT_QUIT;
                ctx.globals.open_url = true;
            }
            _ => {}
        }
        if self.capn_sequence != SEQ_NONE {
            if ctx.player.food == 0 {
                self.set_crunchling_anim(ctx, ANIM_HUNGRY, false, STATE_RESTART_FORWARD);
                ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
                ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
                ctx.actors.set_timer(ACTOR_CRUNCHLING, 0x5a);
                ctx.av.play_sfx(self.sfx_hungry, -1);
                self.hungry_nags += 1;
                if self.hungry_nags > 3 && ctx.cursor.visible {
                    self.caster_speak(ctx, "008.wav");
                    self.hungry_nags = 0;
                }
                self.capn_sequence = SEQ_NONE;
            } else {
                self.capn_step = 0;
                self.capn_sequence_step(ctx).await;
            }
        }
        if fed {
            ctx.av.play_sfx(self.sfx_lever, -1);
            if ctx.player.food < 5 {
                ctx.av.play_sfx(self.sfx_yahoo, -1);
            }
            ctx.scene.set_hover_by_id(OBJ_K1, HOVER_HIDDEN);
            ctx.scene.set_hover_by_id(OBJ_K2, HOVER_HIDDEN);
            ctx.scene.set_hover_by_id(OBJ_K3, HOVER_HIDDEN);
            let lid = self.anim(self.box_index);
            ctx.actors.set_anims(&ctx.av, ACTOR_BOX, lid, lid, true, STATE_RESTART_FORWARD);
            self.box_opening = true;
            ctx.actors.set_visible(ACTOR_PILE, false);
            ctx.actors.set_y(ACTOR_CRUNCHLING, GROUND_Y);
            if ctx.player.food < 5 {
                self.set_crunchling_run(ctx);
                ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, 400);
                ctx.actors.set_arc(ACTOR_CRUNCHLING, 12.0, 8.0);
                ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                self.crunchling_state = CL_RUN_TO_FEEDER;
            } else {
                ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
                ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                self.set_crunchling_anim(ctx, ANIM_NO_FOOD, true, STATE_RESTART_FORWARD);
                ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
            }
            ctx.cursor.set_visible(false);
        }
        result
    }

    // Swings the door open on layer 4 or closed on layer 1 with its sound when the state changes
    fn set_door<H: Host>(&mut self, ctx: &mut Ctx<H>, open: bool) {
        if self.door_open != open {
            self.door_open = open;
            let door = self.anim(ANIM_DOOR);
            if !open {
                ctx.av.play_sfx(self.sfx_door_close, -1);
                ctx.actors.set_anims(&ctx.av, ACTOR_DOOR, door, door, true, STATE_RESTART_BACKWARD);
                ctx.actors.set_layer_mask(ACTOR_DOOR, 1);
            } else {
                ctx.av.play_sfx(self.sfx_door_open, -1);
                ctx.actors.set_anims(&ctx.av, ACTOR_DOOR, door, door, true, STATE_RESTART_FORWARD);
                ctx.actors.set_layer_mask(ACTOR_DOOR, 4);
            }
        }
    }

    // Opens the door and parks the crunchling for twenty ticks before it runs off to the next state
    fn leave_to_game<H: Host>(&mut self, ctx: &mut Ctx<H>, next_state: u32) {
        self.pending_state = next_state;
        self.set_door(ctx, true);
        self.crunchling_state = CL_LEAVING;
        self.set_crunchling_anim(ctx, ANIM_BLINK, false, STATE_RESTART_FORWARD);
        ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
        ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
        ctx.actors.set_timer(ACTOR_CRUNCHLING, 0x14);
        ctx.actors.set_y(ACTOR_CRUNCHLING, GROUND_Y);
        ctx.cursor.set_visible(false);
    }

    // Queues a caster speech, hides the cursor and sends the crunchling running to x 300
    fn caster_speak<H: Host>(&mut self, ctx: &mut Ctx<H>, wav: &str) {
        self.caster_wav = wav.to_string();
        ctx.cursor.set_visible(false);
        self.caster_dir = 1;
        self.set_crunchling_run(ctx);
        ctx.actors.set_y(ACTOR_CRUNCHLING, GROUND_Y);
        ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, 300);
        ctx.actors.set_arc(ACTOR_CRUNCHLING, 12.0, 8.0);
        ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
        self.crunchling_state = CL_SPEECH_RUN;
    }

    // Dispatches actor events to init, anim end, target reached, mouse hit and timer handlers
    async fn handle_event<H: Host>(&mut self, ctx: &mut Ctx<H>, event: i32, actor: i32) {
        match event {
            EVENT_INIT_OBJECT => self.on_init_object(ctx, actor).await,
            EVENT_ANIM_END => self.on_anim_end(ctx, actor).await,
            EVENT_TARGET_REACHED => {
                if actor == ACTOR_CAPN {
                    self.capn_sequence_step(ctx).await;
                } else {
                    match self.crunchling_state {
                        CL_IDLE => self.idle(ctx),
                        CL_RUN_TO_PILE => {
                            let target = ctx.actors.target(ACTOR_CRUNCHLING);
                            if target < 0 || target == ftol((self.pile_x - PILE_APPROACH) as f64) {
                                self.set_crunchling_anim(ctx, ANIM_EAT, false, STATE_RESTART_FORWARD);
                                ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_FORCE_NONE);
                                let pile = self.anim(self.pile_index);
                                ctx.actors.set_anims(&ctx.av, ACTOR_PILE, pile, pile, true, STATE_RESTART_FORWARD);
                                ctx.actors.set_fps(ACTOR_PILE, 8);
                                ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
                                ctx.av.play_sfx(self.sfx_eat, -1);
                            }
                        }
                        CL_RUN_TO_FEEDER => {
                            self.idle(ctx);
                            ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                        }
                        CL_WALK_RIGHT => {
                            ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, 0x78);
                            self.crunchling_state = CL_WALK_LEFT;
                        }
                        CL_WALK_LEFT => {
                            ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, 400);
                            self.crunchling_state = CL_WALK_RIGHT;
                        }
                        CL_LEAVING => self.set_door(ctx, false),
                        CL_SPEECH_RUN => {
                            self.set_crunchling_anim(ctx, ANIM_BLINK, false, STATE_RESTART_FORWARD);
                            ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
                            ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
                            ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                        }
                        _ => {}
                    }
                }
            }
            EVENT_MOUSE_HIT => self.on_mouse_hit(ctx),
            EVENT_TIMER => self.on_timer(ctx, actor).await,
            _ => {}
        }
    }

    // Picks the tool cursor from the mouse height zone and starts tickle, dance or qtip on a click
    fn on_mouse_hit<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.cursor_index != ANIM_MOUSE {
            return;
        }
        let my = ctx.cursor.y as i32;
        if self.crunchling_state == CL_SLEEP {
            self.cursor_index = ANIM_MOUSE_HAND;
        } else {
            let mut stage = ctx.player.stage as u32;
            if stage != 0 && ctx.player.transform_pending != 0 {
                stage -= 1;
            }
            let row = &POS_TABLE[pos_table_row(&ctx.player)];
            if stage == 0 && row[COL_FEATHER_MIN] < my {
                self.cursor_index = ANIM_MOUSE_FEATHER;
            } else if row[COL_HAND_MIN] < my && my <= row[COL_HAND_MAX] {
                self.cursor_index = ANIM_MOUSE_HAND;
            } else if stage == 1 && row[COL_HAND_MAX] < my && my < row[COL_HAND_MAX] + 0x14 {
                self.cursor_index = ANIM_MOUSE_QTIP;
            }
        }
        if ctx.av.is_mouse_button_down(MOUSE_LEFT)
            && self.crunchling_state != CL_PLAYING
            && self.crunchling_state != CL_QTIP_OUT
            && self.crunchling_state != CL_QTIP_EAR_LOOP
            && self.crunchling_state != CL_QTIP_DISGUST
        {
            if self.cursor_index == ANIM_MOUSE_FEATHER {
                ctx.av.play_sfx(self.sfx_giggle, -1);
                self.set_crunchling_anim(ctx, ANIM_TICKLE, true, STATE_RESTART_FORWARD);
                ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_FORCE_NONE);
                ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
                ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                self.crunchling_state = CL_PLAYING;
            } else if self.cursor_index == ANIM_MOUSE_HAND {
                ctx.av.play_sfx(self.sfx_praise, -1);
                self.set_crunchling_anim(ctx, ANIM_DANCE, true, STATE_RESTART_FORWARD);
                ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_FORCE_NONE);
                ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
                ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                ctx.actors.set_y(ACTOR_CRUNCHLING, GROUND_Y);
                self.crunchling_state = CL_PLAYING;
            } else if self.cursor_index == ANIM_MOUSE_QTIP {
                self.set_crunchling_anim(ctx, ANIM_QTIP_OUT, true, STATE_RESTART_FORWARD);
                ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_FORCE_NONE);
                ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
                ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                self.crunchling_state = CL_QTIP_OUT;
                ctx.av.play_sfx(self.sfx_qtip, -1);
            } else {
                self.idle(ctx);
            }
            if self.cursor_index != ANIM_MOUSE && ctx.player.happiness < 5 {
                ctx.player.happiness += 1;
            }
        }
    }

    // Gives each of the seven actors its first animation, position, layer and visibility
    async fn on_init_object<H: Host>(&mut self, ctx: &mut Ctx<H>, actor: i32) {
        match actor {
            ACTOR_CRUNCHLING => {
                if self.crunchling_state == CL_TRANSFORM {
                    self.set_crunchling_anim(ctx, ANIM_BLINK_PREVIOUS, false, STATE_RESTART_FORWARD);
                    ctx.actors.set_pos(ACTOR_CRUNCHLING, 350.0, GROUND_Y);
                } else if self.crunchling_state == CL_SLEEP {
                    self.set_crunchling_anim(ctx, ANIM_SLEEP, false, STATE_RESTART_FORWARD);
                    let mut lift: f32 = 30.0;
                    if ctx.player.stage == 1 {
                        lift = 50.0;
                    } else if ctx.player.stage == 2 {
                        lift = 70.0;
                    }
                    ctx.actors.set_pos(ACTOR_CRUNCHLING, 500.0, lift + GROUND_Y);
                } else {
                    self.set_crunchling_anim(ctx, ANIM_BLINK, false, STATE_RESTART_FORWARD);
                    if self.capn_sequence == SEQ_NONE {
                        ctx.actors.set_timer(ACTOR_CRUNCHLING, 0x5a);
                    }
                    ctx.actors.set_pos(ACTOR_CRUNCHLING, 350.0, GROUND_Y);
                }
                ctx.actors.set_shadow(ACTOR_CRUNCHLING, self.anim(ANIM_SHADOW), 0x10, 0x82);
                ctx.actors.set_shadow_enabled(ACTOR_CRUNCHLING, true);
                ctx.actors.set_turn_anims(ACTOR_CRUNCHLING, self.anim(ANIM_TURN_RIGHT), self.anim(ANIM_TURN_LEFT));
                ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
                ctx.actors.set_mouse_hittable(ACTOR_CRUNCHLING, true);
            }
            ACTOR_CAPN => {
                if self.capn_sequence == SEQ_NONE {
                    ctx.actors.set_visible(ACTOR_CAPN, false);
                } else {
                    self.capn_sequence_step(ctx).await;
                }
            }
            ACTOR_TUBE => {
                let h = self.anim(ANIM_TUBE_LEFT);
                ctx.actors.set_anims(&ctx.av, ACTOR_TUBE, h, h, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(ACTOR_TUBE, 0.0, 0.0);
                ctx.actors.set_visible(ACTOR_TUBE, false);
            }
            ACTOR_PILE => {
                let h = self.anim(ANIM_CRUNCH_PILE);
                ctx.actors.set_anims(&ctx.av, ACTOR_PILE, h, h, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(ACTOR_PILE, 233.0, 348.0);
                ctx.actors.set_visible(ACTOR_PILE, false);
            }
            ACTOR_BOX => {
                let h = self.anim(ANIM_BOX_1);
                ctx.actors.set_anims(&ctx.av, ACTOR_BOX, h, h, false, STATE_STOPPED);
                ctx.actors.set_pos(ACTOR_BOX, 0.0, 0.0);
                ctx.actors.set_visible(ACTOR_BOX, true);
                ctx.actors.set_layer_mask(ACTOR_BOX, 4);
            }
            ACTOR_DOOR => {
                let h = self.anim(ANIM_DOOR);
                ctx.actors.set_anims(&ctx.av, ACTOR_DOOR, h, h, true, STATE_STOPPED);
                self.door_open = false;
                ctx.actors.set_pos(ACTOR_DOOR, 0.0, 0.0);
                ctx.actors.set_visible(ACTOR_DOOR, true);
                ctx.actors.set_layer_mask(ACTOR_DOOR, 2);
            }
            ACTOR_CASTER => {
                let h = self.anim(ANIM_CASTER_2);
                ctx.actors.set_anims(&ctx.av, ACTOR_CASTER, h, h, false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(ACTOR_CASTER, -20.0, self.caster_y);
            }
            _ => {}
        }
    }

    // Chains the qtip anims, finishes the transform, feeds, cycles the box and tube, and exits by the door
    async fn on_anim_end<H: Host>(&mut self, ctx: &mut Ctx<H>, actor: i32) {
        match actor {
            ACTOR_CRUNCHLING => match self.crunchling_state {
                CL_QTIP_OUT => {
                    self.set_crunchling_anim(ctx, ANIM_QTIP_EAR_LOOP, true, STATE_RESTART_FORWARD);
                    self.crunchling_state = CL_QTIP_EAR_LOOP;
                    self.ear_loop_count = 0;
                }
                CL_QTIP_EAR_LOOP => {
                    let loops = self.ear_loop_count;
                    self.ear_loop_count += 1;
                    if loops != 0 {
                        self.set_crunchling_anim(ctx, ANIM_QTIP_DISGUST, true, STATE_RESTART_FORWARD);
                        self.crunchling_state = CL_QTIP_DISGUST;
                    }
                }
                CL_QTIP_DISGUST => {
                    self.set_crunchling_anim(ctx, ANIM_QTIP_THROW, true, STATE_RESTART_FORWARD);
                    self.crunchling_state = CL_QTIP_THROW;
                }
                CL_TRANSFORM => {
                    let row = &POS_TABLE[pos_table_row(&ctx.player)];
                    ctx.actors.set_draw_offset(ACTOR_CRUNCHLING, row[COL_OFF_X], row[COL_OFF_Y]);
                    self.set_crunchling_anim(ctx, ANIM_BLINK, false, STATE_RESTART_FORWARD);
                    self.capn_sequence_step(ctx).await;
                }
                _ => self.idle(ctx),
            },
            ACTOR_CAPN => self.capn_sequence_step(ctx).await,
            ACTOR_TUBE => ctx.actors.set_visible(ACTOR_TUBE, false),
            ACTOR_PILE => {
                if self.crunchling_state == CL_RUN_TO_PILE {
                    if ctx.player.food < 5 {
                        ctx.player.food += 1;
                    }
                    self.idle(ctx);
                    ctx.actors.set_visible(ACTOR_PILE, false);
                    let lid = self.anim(self.box_index);
                    ctx.actors.set_anims(&ctx.av, ACTOR_BOX, lid, lid, true, STATE_RESTART_BACKWARD);
                } else {
                    self.set_crunchling_run(ctx);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, ftol((self.pile_x - PILE_APPROACH) as f64));
                    ctx.actors.set_arc(ACTOR_CRUNCHLING, 12.0, 8.0);
                    ctx.actors.set_timer(ACTOR_CRUNCHLING, 0);
                    self.crunchling_state = CL_RUN_TO_PILE;
                }
            }
            ACTOR_BOX => {
                if self.box_opening {
                    if ctx.player.food < 5 {
                        ctx.actors.set_timer(ACTOR_TUBE, 0xf);
                        let tube = self.anim(self.tube_index);
                        ctx.actors.set_anims(&ctx.av, ACTOR_TUBE, tube, tube, true, STATE_RESTART_FORWARD);
                        ctx.actors.set_visible(ACTOR_TUBE, true);
                        ctx.av.play_sfx(self.sfx_pipe, -1);
                    } else {
                        let lid = self.anim(self.box_index);
                        ctx.actors.set_anims(&ctx.av, ACTOR_BOX, lid, lid, true, STATE_RESTART_BACKWARD);
                        ctx.av.play_sfx(self.sfx_uhuh, -1);
                    }
                    self.box_opening = false;
                } else {
                    ctx.scene.set_hover_by_id(OBJ_K1, HOVER_NORMAL);
                    ctx.scene.set_hover_by_id(OBJ_K2, HOVER_NORMAL);
                    ctx.scene.set_hover_by_id(OBJ_K3, HOVER_NORMAL);
                    ctx.cursor.set_visible(true);
                    let lid = self.anim(self.box_index);
                    ctx.actors.set_anims(&ctx.av, ACTOR_BOX, lid, lid, false, STATE_STOPPED);
                }
            }
            ACTOR_DOOR if !self.door_open && self.crunchling_state == CL_LEAVING => {
                self.result = self.pending_state;
            }
            _ => {}
        }
    }

    // Starts a walk, plays the transform, runs the crunchling out the door, or blinks on a timer
    async fn on_timer<H: Host>(&mut self, ctx: &mut Ctx<H>, actor: i32) {
        if actor == ACTOR_CRUNCHLING {
            match self.crunchling_state {
                CL_IDLE => {
                    let right = self.anim(ANIM_WALK_RIGHT);
                    let left = self.anim(ANIM_WALK_LEFT);
                    ctx.actors.set_anims(&ctx.av, ACTOR_CRUNCHLING, right, left, false, STATE_RESTART_FORWARD);
                    ctx.actors.set_arc(ACTOR_CRUNCHLING, 4.0, 8.0);
                    ctx.actors.set_timer(ACTOR_CRUNCHLING, 0x32);
                    self.crunchling_state = self.last_walk_state;
                    if self.crunchling_state == CL_WALK_RIGHT {
                        ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, 400);
                    } else {
                        ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, 0x78);
                    }
                }
                CL_PLAYING => {}
                CL_TRANSFORM => {
                    self.set_crunchling_anim(ctx, ANIM_TRANSFORM, true, STATE_RESTART_FORWARD);
                    ctx.av.play_sfx(self.sfx_transform, -1);
                }
                CL_LEAVING => {
                    self.set_crunchling_run(ctx);
                    ctx.actors.set_arc(ACTOR_CRUNCHLING, 12.0, 8.0);
                    ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, -0x7d);
                    ctx.av.play_sfx(self.sfx_run_fast, -1);
                }
                CL_SPEECH_RUN => {
                    self.set_crunchling_anim(ctx, ANIM_BLINK, false, STATE_RESTART_FORWARD);
                    ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
                }
                _ => {
                    if self.crunchling_state == CL_WALK_RIGHT || self.crunchling_state == CL_WALK_LEFT {
                        self.last_walk_state = self.crunchling_state;
                    }
                    self.idle(ctx);
                }
            }
        } else if actor == ACTOR_CAPN {
            self.capn_sequence_step(ctx).await;
        } else if actor == ACTOR_TUBE {
            ctx.actors.set_visible(ACTOR_PILE, true);
        }
    }

    // Picks a hungry, need praise or blink pose for ninety ticks and nags through the caster after four
    fn idle<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let mut roll = ctx.rand_range(0, 4);
        if self.crunchling_state == CL_RUN_TO_FEEDER || self.crunchling_state == CL_RUN_TO_PILE {
            roll = 3;
            if self.crunchling_state == CL_RUN_TO_PILE {
                self.crunchling_state = CL_IDLE;
            }
        } else {
            self.crunchling_state = CL_IDLE;
        }
        ctx.actors.walk_to(&ctx.av, ACTOR_CRUNCHLING, NO_TARGET);
        ctx.actors.set_timer(ACTOR_CRUNCHLING, 0x5a);
        if roll == 0 && ctx.player.food == 0 {
            self.set_crunchling_anim(ctx, ANIM_HUNGRY, false, STATE_RESTART_FORWARD);
            ctx.av.play_sfx(self.sfx_hungry, -1);
            self.hungry_nags += 1;
            if self.hungry_nags > 3 && ctx.cursor.visible {
                self.caster_speak(ctx, "008.wav");
                self.hungry_nags = 0;
            }
        } else if roll == 1 && ctx.player.happiness == 0 {
            self.set_crunchling_anim(ctx, ANIM_NEED_PRAISE, false, STATE_RESTART_FORWARD);
            ctx.av.play_sfx(self.sfx_sad, -1);
            self.praise_nags += 1;
            if self.praise_nags > 3 && ctx.cursor.visible {
                self.caster_speak(ctx, "007.wav");
                self.praise_nags = 0;
            }
        } else {
            self.set_crunchling_anim(ctx, ANIM_BLINK, false, STATE_RESTART_FORWARD);
        }
        ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
    }

    // Applies the shift and ctrl training cheats, the 7 stage cheat and the 0 reload cheat
    fn cheat_keys<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let shift = ctx.av.is_key_down(KEY_LSHIFT) || ctx.av.is_key_down(KEY_RSHIFT);
        let ctrl = ctx.av.is_key_down(KEY_LCTRL) || ctx.av.is_key_down(KEY_RCTRL);
        if shift || ctrl {
            let key = ctx.av.key_input();
            if ctx.player.transform_pending == 0 {
                if shift && ctx.av.is_key_down(KEY_DOWN) && key == KEY_S {
                    ctx.player.add_stats(1, 0, 0);
                }
                if shift && ctx.av.is_key_down(KEY_UP) && key == KEY_J {
                    ctx.player.add_stats(0, 0, 1);
                }
                if ctrl && ctx.av.is_key_down(KEY_RIGHT) && key == KEY_T {
                    ctx.player.add_stats(0, 1, 0);
                }
                if key == KEY_7 {
                    ctx.player.transform_pending = 1;
                    ctx.player.stage += 1;
                    if ctx.player.stage > 2 {
                        ctx.player.stage = 2;
                    }
                }
            }
            if key == KEY_0 {
                self.result = STATE_HOME_WORLD;
            }
            ctx.player.lagging_stat = -1;
        }
    }

    // Draws the food and happiness dark overlays then the three stat minibars with one shared frame
    fn draw_bars<H: Host>(&self, ctx: &mut Ctx<H>) {
        let mut af = AnimFrame { frame: 0.0, anim: self.anim(ANIM_BAR_DARK), state: STATE_RESTART_FORWARD, fps: 0 };
        let food_left = bar_overlay_left(ctx.player.food);
        let mut clip = Rect::new(food_left as i32, 0, 0x80, 0xb);
        ctx.av.draw_anim_clip(&mut af, (food_left as i32 + 5) as i16, 0x32, &mut clip);
        let happy_left = bar_overlay_left(ctx.player.happiness);
        clip.left = happy_left as i32;
        ctx.av.draw_anim_clip(&mut af, (happy_left as i32 + 5) as i16, 0x46, &mut clip);
        af.anim = self.anim(ANIM_MINIBAR);
        clip.left = 0;
        clip.top = 0;
        clip.bottom = 9;
        if let Some(width) = minibar_width(ctx.player.running) {
            clip.right = width as i32;
            ctx.av.draw_anim_clip(&mut af, 0x194, 0x1ae, &mut clip);
        }
        if let Some(width) = minibar_width(ctx.player.jumping) {
            clip.right = width as i32;
            ctx.av.draw_anim_clip(&mut af, 0x1e2, 0x1ae, &mut clip);
        }
        if let Some(width) = minibar_width(ctx.player.strength) {
            clip.right = width as i32;
            ctx.av.draw_anim_clip(&mut af, 0x22f, 0x1ae, &mut clip);
        }
    }

    // Raises the caster three pixels a tick, plays its speech at the top, and lowers it when done
    async fn update_caster<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.caster_dir < 0 {
            if CASTER_HIDDEN_Y < self.caster_y {
                self.caster_y -= CASTER_STEP;
            }
        } else if CASTER_SHOWN_Y <= self.caster_y {
            if ctx.av.is_key_down(KEY_SPACE) || ctx.av.is_stream_sound_done(self.voice_channel) {
                ctx.av.kill_streaming_channel(self.voice_channel);
                ctx.cursor.set_visible(true);
                self.idle(ctx);
                let h = self.anim(ANIM_CASTER_2);
                ctx.actors.set_anims(&ctx.av, ACTOR_CASTER, h, h, false, STATE_RESTART_FORWARD);
                self.caster_dir = -1;
            }
        } else {
            self.caster_y += CASTER_STEP;
            if CASTER_SHOWN_Y <= self.caster_y {
                let h = self.anim(ANIM_CASTER_1);
                ctx.actors.set_anims_delayed(&ctx.av, ACTOR_CASTER, 0x14, h, h, false, STATE_RESTART_FORWARD);
                self.voice_channel =
                    ctx.av.play_stream_sound(&mut ctx.host, &self.caster_wav, false, self.voice_channel).await;
            }
        }
        ctx.actors.set_y(ACTOR_CASTER, self.caster_y);
    }
}

impl<H: Host> Screen<H> for HomeWorldScreen {
    // Loads the scene, sounds, anims and actors and arms the first visit or transform sequence
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.last_walk_state = CL_WALK_RIGHT;
        self.caster_dir = -1;
        self.caster_y = CASTER_HIDDEN_Y;
        self.crunchling_state = CL_IDLE;
        if ctx.rand_range(0, 6) > 1 {
            self.crunchling_state = CL_SLEEP;
        }
        self.capn_step = 0;
        self.waiting_for_voice = false;
        self.capn_sequence = SEQ_NONE;
        if ctx.player.first_home_visit != 0 {
            self.capn_sequence = SEQ_FIRST_VISIT;
            ctx.player.first_home_visit = 0;
        }
        if ctx.player.transform_pending != 0 {
            self.capn_sequence = SEQ_TRANSFORMED;
            ctx.player.transform_pending = 0;
        }
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "HomeWorld").await;
        ctx.scene.assign_id("k1", OBJ_K1);
        ctx.scene.assign_id("k2", OBJ_K2);
        ctx.scene.assign_id("k3", OBJ_K3);
        ctx.scene.assign_id("name", OBJ_NAME);
        ctx.scene.assign_id("speed", OBJ_SPEED);
        ctx.scene.assign_id("strength", OBJ_STRENGTH);
        ctx.scene.assign_id("jump", OBJ_JUMP);
        ctx.scene.assign_id("challenge", OBJ_CHALLENGE);
        ctx.scene.assign_id("tips", OBJ_TIPS);
        ctx.scene.assign_id("FoodGlow", OBJ_FOOD_GLOW);
        ctx.scene.assign_id("feederGlow", OBJ_FEEDER_GLOW);
        ctx.scene.assign_id("CasterGlow", OBJ_CASTER_GLOW);
        ctx.scene.assign_id("Quit", OBJ_QUIT);
        ctx.scene.assign_id("online", OBJ_ONLINE);
        ctx.scene.set_hover_by_id(OBJ_FOOD_GLOW, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(OBJ_FEEDER_GLOW, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(OBJ_CASTER_GLOW, HOVER_HIDDEN);
        if ctx.player.stage < 2 {
            ctx.scene.set_hover_by_id(OBJ_CHALLENGE, HOVER_PRESSED);
        }
        let name = ctx.player.name_str();
        ctx.scene.set_text_by_id(OBJ_NAME, &name);
        if !self.load_sfx_set(ctx).await {
            return false;
        }
        self.loop_channel = ctx.av.play_stream_sound(&mut ctx.host, "cs_homeloop.wav", true, 0).await;
        ctx.actors.clear();
        ctx.actors.add_actor(ACTOR_TUBE, true);
        ctx.actors.add_actor(ACTOR_DOOR, true);
        ctx.actors.add_actor(ACTOR_CASTER, true);
        ctx.actors.add_actor(ACTOR_CRUNCHLING, true);
        ctx.actors.add_actor(ACTOR_CAPN, true);
        ctx.actors.add_actor(ACTOR_PILE, true);
        ctx.actors.add_actor(ACTOR_BOX, true);
        let blink_name = ctx.player.anim_name("blink", false, false);
        let Some(blink) = ctx.av.get_anim_retain_image(&mut ctx.host, &blink_name).await else { return false };
        self.anims[ANIM_BLINK as usize] = blink;
        let mut palette = Palette24::default();
        ctx.av.get_anim_palette(blink, &mut palette);
        ctx.player.recolor_palette(&mut palette);
        ctx.av.set_anim_palette(blink, &palette);
        if self.capn_sequence == SEQ_TRANSFORMED && ctx.player.stage != 0 {
            ctx.player.stage -= 1;
            if !self.load_crunchling_anim(ctx, ANIM_BLINK_PREVIOUS, "blink", false, &palette).await {
                return false;
            }
            if !self.load_crunchling_anim(ctx, ANIM_TRANSFORM, "transform", true, &palette).await {
                return false;
            }
            ctx.player.stage += 1;
            self.crunchling_state = CL_TRANSFORM;
            let row = &POS_TABLE[ctx.player.anim_index(true) as usize];
            ctx.actors.set_draw_offset(ACTOR_CRUNCHLING, row[COL_OFF_X], row[COL_OFF_Y]);
        } else {
            let row = &POS_TABLE[ctx.player.anim_index(false) as usize];
            ctx.actors.set_draw_offset(ACTOR_CRUNCHLING, row[COL_OFF_X], row[COL_OFF_Y]);
        }
        if !self.load_anim_set(ctx, &palette).await {
            return false;
        }
        ctx.cursor.set_anim(&ctx.av, self.anim(ANIM_MOUSE));
        ctx.cursor.set_visible(true);
        ctx.cursor.set_pos(0x140, 200);
        if self.capn_sequence != SEQ_TRANSFORMED {
            self.crunchling_state = CL_IDLE;
        }
        if !self.load_anim(ctx, ANIM_CAPN_013, "capn_cap013").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_CAPN_021, "capn_cap021").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_CAPN_010, "capn_cap010").await {
            return false;
        }
        if !self.load_anim(ctx, ANIM_CAPN_008, "capn_cap008").await {
            return false;
        }
        if self.capn_sequence != SEQ_NONE {
            ctx.cursor.set_visible(false);
        }
        self.box_index = ANIM_BOX_1;
        self.box_opening = false;
        self.tube_index = ANIM_TUBE_LEFT;
        self.voice_channel = 0;
        ctx.player.challenge_active = 0;
        self.hungry_nags = 0;
        self.praise_nags = 0;
        true
    }

    // Runs one frame of drawing, input, cheats, actor events, bars and the caster, returning the next state
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut result = RESULT_CONTINUE;
        self.result = RESULT_CONTINUE;
        self.cursor_index = ANIM_MOUSE;
        ctx.scene.draw_layer(&mut ctx.av, 0);
        if ctx.cursor.visible {
            let (dx, dy) = ctx.av.get_mouse_delta();
            ctx.cursor.move_by(dx, dy);
            let (mx, my) = (ctx.cursor.x, ctx.cursor.y);
            if let Some(obj) = ctx.scene.hit_test(mx as i32, my as i32) {
                if obj == OBJ_K1 || obj == OBJ_K2 || obj == OBJ_K3 {
                    self.cursor_index = ANIM_MOUSE_HAND;
                    if obj == OBJ_K1 {
                        let h = self.anim(ANIM_BOX_1);
                        ctx.actors.set_anims(&ctx.av, ACTOR_BOX, h, h, false, STATE_STOPPED);
                    } else if obj == OBJ_K2 {
                        let h = self.anim(ANIM_BOX_2);
                        ctx.actors.set_anims(&ctx.av, ACTOR_BOX, h, h, false, STATE_STOPPED);
                    } else if obj == OBJ_K3 {
                        let h = self.anim(ANIM_BOX_3);
                        ctx.actors.set_anims(&ctx.av, ACTOR_BOX, h, h, false, STATE_STOPPED);
                    }
                }
                if ctx.av.is_mouse_button_down(MOUSE_LEFT) {
                    if !self.mouse_latched {
                        result = self.on_object_clicked(ctx, obj).await;
                        self.mouse_latched = true;
                    }
                } else {
                    self.mouse_latched = false;
                }
            }
            ctx.actors.mouse_hit_test(&ctx.av, mx, my);
        }
        self.cheat_keys(ctx);
        ctx.actors.update(&ctx.av);
        while let Some(ev) = ctx.actors.pop_event() {
            self.handle_event(ctx, ev.kind, ev.actor_id).await;
        }
        ctx.cursor.set_anim(&ctx.av, self.anim(self.cursor_index));
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 2);
        ctx.scene.draw_layer(&mut ctx.av, 2);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 1);
        ctx.scene.draw_layer(&mut ctx.av, 1);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 4);
        ctx.scene.draw_layer(&mut ctx.av, 4);
        self.draw_bars(ctx);
        self.update_caster(ctx).await;
        if ctx.player.lagging_stat > -1 {
            if ctx.player.lagging_stat == 0 {
                self.caster_speak(ctx, "009.wav");
            } else if ctx.player.lagging_stat == 1 {
                self.caster_speak(ctx, "011.wav");
            } else if ctx.player.lagging_stat == 2 {
                self.caster_speak(ctx, "010.wav");
            }
            ctx.player.lagging_stat = -1;
        }
        if self.waiting_for_voice && ctx.av.is_stream_sound_done(self.voice_channel) {
            self.waiting_for_voice = false;
            self.capn_sequence_step(ctx).await;
        }
        ctx.cursor.draw(&mut ctx.av);
        ctx.scene.advance_anims(&ctx.av);
        if self.result != RESULT_CONTINUE {
            result = self.result;
        }
        result
    }

    fn shutdown(&mut self, ctx: &mut Ctx<H>) {
        ctx.av.kill_streaming_channel(self.loop_channel);
        ctx.av.kill_streaming_channel(self.voice_channel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pos_table_matches_binary_dump() {
        assert_eq!(POS_TABLE[0], [41, 89, 240, 265, 305]);
        assert_eq!(POS_TABLE[3], [33, 60, 220, 245, 305]);
        assert_eq!(POS_TABLE[5], [25, 62, 220, 245, 305]);
        assert_eq!(POS_TABLE[8], [18, 2, 165, 200, 300]);
    }

    #[test]
    fn pos_table_row_uses_previous_stage_while_transforming() {
        let mut p = PlayerData { kind: 1, stage: 2, ..Default::default() };
        assert_eq!(pos_table_row(&p), 7);
        p.transform_pending = 1;
        assert_eq!(pos_table_row(&p), 4);
        p.stage = 0;
        assert_eq!(pos_table_row(&p), 1);
    }

    #[test]
    fn bar_overlay_caps_at_full_width() {
        assert_eq!(bar_overlay_left(0), 0);
        assert_eq!(bar_overlay_left(3), 75);
        assert_eq!(bar_overlay_left(5), 125);
        assert_eq!(bar_overlay_left(6), 128);
    }

    #[test]
    fn minibar_skips_empty_and_caps() {
        assert_eq!(minibar_width(0), None);
        assert_eq!(minibar_width(7), Some(21));
        assert_eq!(minibar_width(20), Some(60));
        assert_eq!(minibar_width(25), Some(60));
    }
}
