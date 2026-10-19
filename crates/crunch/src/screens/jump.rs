use avmanager::anim::{STATE_RESTART_FORWARD, STATE_STOPPED};
use avmanager::crt::ftol;
use avmanager::texture::Palette24;

use crate::actors::{
    ACTOR_FLAG_FINISH, ACTOR_FLAG_PICKUP, EVENT_ANIM_END, EVENT_FINISH, EVENT_GROUND_REACHED, EVENT_INIT_OBJECT,
    EVENT_PICKUP, EVENT_UNATTACHED, FACING_LEFT, FACING_NONE, FACING_RIGHT,
};
use crate::ctx::{
    Ctx, KEY_ESC, KEY_LEFT, KEY_RIGHT, KEY_SPACE, MOUSE_LEFT, RESULT_CONTINUE, STATE_CHALLENGE, STATE_HOME_WORLD,
    STATE_JUMPING, Screen,
};
use crate::host::Host;
use crate::scene::{FLAG_FINISH, FLAG_PICKUP, FLAG_PLATFORM, HOVER_HIDDEN, HOVER_NORMAL};
use crate::scroll_strip::ScrollStrip;

const ANIM_STANCE: usize = 0;
const ANIM_RUNNING_LEFT: usize = 1;
const ANIM_RUNNING_RIGHT: usize = 2;
const ANIM_JUMPING_LEFT: usize = 3;
const ANIM_JUMPING_RIGHT: usize = 4;
const ANIM_JUMP_TURN_LEFT: usize = 5;
const ANIM_JUMP_TURN_RIGHT: usize = 6;
const ANIM_FALLING_LOOP: usize = 7;
const ANIM_HOVER_FLY: usize = 8;
const ANIM_CAP_TV_IDLE: usize = 9;
const ANIM_CAP_TV_TALK: usize = 10;
const ANIM_CAP_TV_WORRIED: usize = 11;
const ANIM_CAPN_FLY_IDLE: usize = 12;
const ANIM_CAPN_FLY_TALK: usize = 13;
const ANIM_THIEF_CLIMB_A: usize = 14;
const ANIM_THIEF_CLIMB_B: usize = 15;
const ANIM_POLE: usize = 16;
const ANIM_THREE: usize = 17;
const ANIM_TWO: usize = 18;
const ANIM_ONE: usize = 19;
const ANIM_GO: usize = 20;
const ANIM_BERRYBULGE: usize = 21;

const SFX_JUMP: usize = 0;
const SFX_BERRY: usize = 1;
const SFX_LAND: usize = 2;

const ACTOR_THIEF: i32 = 0;
const ACTOR_PLAYER: i32 = 1;
const ACTOR_CAPN: i32 = 2;
const ACTOR_TV: i32 = 3;
const ACTOR_COUNTDOWN: i32 = 4;
const FIRST_PLATFORM_ACTOR: i32 = 5;

const OBJ_BEST_TITLE: i32 = 2;
const OBJ_BEST_TIME: i32 = 3;
const OBJ_TIME: i32 = 4;
const OBJ_TIME_TITLE: i32 = 5;
const OBJ_DONE: i32 = 6;
const OBJ_GOOD_JOB: i32 = 7;
const OBJ_AGAIN: i32 = 8;

const PLAYER_NONE: i32 = 0;
const PLAYER_STANDING: i32 = 1;
const PLAYER_RUNNING: i32 = 2;
const PLAYER_AIRBORNE: i32 = 3;
const PLAYER_FINISHED: i32 = 7;

const CAPN_NONE: i32 = 0;
const CAPN_IDLE: i32 = 0xc;
const CAPN_TALKING: i32 = 0xd;
const CAPN_WORRIED: i32 = 0xe;

const THIEF_STOPPED: i32 = 1;
const THIEF_CLIMBING: i32 = 3;

const FLY_START: i32 = 8;
const FLY_LEAVING: i32 = 9;
const FLY_ARRIVING: i32 = 10;
const FLY_SPEAKING: i32 = 0xb;

const RESULT_UNDECIDED: i32 = 0;
const RESULT_PLAYER_FIRST: i32 = 1;
const RESULT_THIEF_FIRST: i32 = 2;

const COUNTDOWN_START: u32 = 0x83;
const COUNTDOWN_ACTIVE: u32 = 0x28;
const COUNTDOWN_THREE: u32 = 0x82;
const COUNTDOWN_TWO: u32 = 100;
const COUNTDOWN_ONE: u32 = 0x46;

const CHALLENGE_EXIT_FRAMES: u32 = 0x5a;
const FALL_WARNING_FRAMES: u32 = 0x14;
const PRAISE_STREAK: u32 = 3;
const BONUS_BERRIES: u32 = 4;

const JUMP_GRAVITY: f32 = 1.2;
const MAX_FALL_SPEED: f32 = 14.0;
const SCREEN_HEIGHT: i32 = 480;

// Jumping mini game, state 0xc, with its tall scrolling level and the Underground thief race
#[derive(Clone, Debug, Default)]
pub struct JumpScreen {
    cursor_anim: u16,
    anims: [u16; 22],
    sfx: [u16; 3],
    backdrop: ScrollStrip,
    music: u32,
    voice: u32,
    max_scroll_y: i32,
    player_state: i32,
    capn_state: i32,
    thief_state: i32,
    max_run_speed: f32,
    run_accel: f32,
    vx: f32,
    ground_y: f32,
    berries: u32,
    last_time: u32,
    time_frac: f32,
    time_secs: u16,
    timer_reset: bool,
    countdown: u32,
    finished: bool,
    thief_y: f32,
    result: i32,
    intro_pending: bool,
    outro_active: bool,
    challenge: i32,
    capn_fly_state: i32,
    fall_frames: u32,
    fly_y: f32,
    base_y: f32,
    end_delay: u32,
    berry_streak: u32,
}

// Actor flag word for a level object, pickup wins over finish line over plain platform
fn platform_flags(scene_flags: u32) -> u32 {
    let mut flags = 0;
    if scene_flags & FLAG_PICKUP == 0 {
        if scene_flags & FLAG_FINISH != 0 {
            flags = ACTOR_FLAG_FINISH;
        }
    } else {
        flags = ACTOR_FLAG_PICKUP;
    }
    flags
}

// Text the time objects show, seconds then two digit hundredths
fn time_text(secs: u16, hundredths: i32) -> String {
    format!(" {secs}.{hundredths:02}")
}

// Pixels the Cap'n and TV move per frame by how far along the intro flight they are
fn fly_speed(fly_y: f32) -> f32 {
    let mut speed = 12.0;
    if 100.0 <= fly_y {
        if 180.0 <= fly_y {
            if fly_y < 300.0 {
                speed = 2.0;
            }
        } else {
            speed = 4.0;
        }
    } else {
        speed = 8.0;
    }
    speed
}

// Height above the ground the crunchling drops in from, deeper for adults
fn drop_height(stage: u16) -> f32 {
    let mut height = -100.0;
    if stage == 2 {
        height = -150.0;
    }
    height
}

// New scroll y from the player's screen relative y with a dead zone, soft zone and hard snaps
fn camera_scroll_y(rel: f32, mut scroll_y: i32, max_scroll_y: i32) -> i32 {
    if 110.0 <= rel {
        if rel <= 280.0 {
            if 160.0 <= rel {
                if 220.0 < rel && scroll_y < max_scroll_y {
                    scroll_y += 1;
                }
            } else if 0 < scroll_y {
                scroll_y -= 1;
            }
        } else {
            scroll_y += ftol(rel as f64 - 280.0);
            if max_scroll_y < scroll_y {
                scroll_y = max_scroll_y;
            }
        }
    } else {
        scroll_y += ftol(rel as f64 - 110.0);
        if scroll_y < 0 {
            scroll_y = 0;
        }
    }
    scroll_y
}

impl JumpScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Loads the crunchling strip through the stance palette and every shared animation
    async fn load_anims<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        let name = ctx.player.anim_name("stance", false, false);
        let Some(stance) = ctx.av.get_anim_retain_image(&mut ctx.host, &name).await else { return false };
        self.anims[ANIM_STANCE] = stance;
        let mut pal = Palette24::default();
        ctx.av.get_anim_palette(stance, &mut pal);
        ctx.player.recolor_palette(&mut pal);
        ctx.av.set_anim_palette(stance, &pal);
        let crunchling = [
            (ANIM_RUNNING_LEFT, "running_left"),
            (ANIM_RUNNING_RIGHT, "running_right"),
            (ANIM_JUMPING_LEFT, "jumping_left"),
            (ANIM_JUMPING_RIGHT, "jumping_right"),
            (ANIM_JUMP_TURN_LEFT, "jump_turn_left"),
            (ANIM_JUMP_TURN_RIGHT, "jump_turn_right"),
            (ANIM_FALLING_LOOP, "falling_loop"),
        ];
        for (slot, suffix) in crunchling {
            let name = ctx.player.anim_name(suffix, false, false);
            let Some(h) = ctx.av.get_anim_use_palette(&mut ctx.host, &name, &pal).await else { return false };
            self.anims[slot] = h;
        }
        let shared = [
            (ANIM_BERRYBULGE, "berrybulge"),
            (ANIM_HOVER_FLY, "hover_fly"),
            (ANIM_CAP_TV_IDLE, "cap_tv_cap005"),
            (ANIM_CAP_TV_TALK, "cap_tv_cap006"),
            (ANIM_CAP_TV_WORRIED, "cap_tv_cap004"),
            (ANIM_CAPN_FLY_IDLE, "capn_cap028"),
            (ANIM_CAPN_FLY_TALK, "capn_cap029"),
            (ANIM_ONE, "one"),
            (ANIM_TWO, "two"),
            (ANIM_THREE, "three"),
            (ANIM_GO, "go"),
        ];
        for (slot, name) in shared {
            let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
            self.anims[slot] = h;
        }
        true
    }

    // Handles a click on a scene object, done rewards and leaves, Again rewards and restarts
    fn on_mouse_hit<H: Host>(&mut self, ctx: &mut Ctx<H>, id: i32) -> u32 {
        let mut ret = RESULT_CONTINUE;
        if id == OBJ_DONE {
            if ctx.player.food != 0 {
                ctx.player.food -= 1;
            }
            if ctx.player.happiness != 0 {
                ctx.player.happiness -= 1;
            }
            ctx.player.add_stats(0, 0, 1);
            if BONUS_BERRIES < self.berries {
                ctx.player.add_stats(0, 0, 1);
            }
            ret = STATE_HOME_WORLD;
        } else if id == OBJ_AGAIN {
            ctx.player.add_stats(0, 0, 1);
            if BONUS_BERRIES < self.berries {
                ctx.player.add_stats(0, 0, 1);
            }
            ret = STATE_JUMPING;
        }
        ret
    }

    // Dispatches one actor event to landing, falling, berry, finish and init handling
    async fn on_actor_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, actor: i32) {
        match kind {
            EVENT_INIT_OBJECT => self.on_init_object(ctx, actor),
            EVENT_ANIM_END => self.on_anim_end(ctx, actor),
            EVENT_GROUND_REACHED if actor == ACTOR_PLAYER && self.player_state == PLAYER_AIRBORNE => {
                let stance = self.anims[ANIM_STANCE];
                ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, stance, stance, false, STATE_RESTART_FORWARD);
                self.player_state = PLAYER_STANDING;
                ctx.av.play_sfx(self.sfx[SFX_LAND], -1);
            }
            EVENT_UNATTACHED if actor == ACTOR_PLAYER => {
                let falling = self.anims[ANIM_FALLING_LOOP];
                ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, falling, falling, false, STATE_RESTART_FORWARD);
                let ground = ftol(self.ground_y as f64);
                ctx.actors.set_gravity(ACTOR_PLAYER, 0.0, JUMP_GRAVITY, ground, MAX_FALL_SPEED);
                self.player_state = PLAYER_AIRBORNE;
            }
            EVENT_PICKUP => {
                self.berries += 1;
                self.berry_streak += 1;
                if PRAISE_STREAK < self.berry_streak {
                    self.berry_streak = 0;
                    if self.capn_state == CAPN_IDLE {
                        let talk = self.anims[ANIM_CAP_TV_TALK];
                        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
                        if ctx.rand_range(0, 2) == 0 {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "052.wav", false, self.voice).await;
                        } else {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "053.wav", false, self.voice).await;
                        }
                        self.capn_state = CAPN_TALKING;
                    }
                }
                ctx.av.play_sfx(self.sfx[SFX_BERRY], -1);
            }
            EVENT_FINISH => self.finished = true,
            _ => {}
        }
    }

    // Places the thief, player, Cap'n, TV and countdown digits on their first frame
    fn on_init_object<H: Host>(&mut self, ctx: &mut Ctx<H>, actor: i32) {
        match actor {
            ACTOR_THIEF => {
                self.thief_state = THIEF_STOPPED;
                let climb = self.anims[ANIM_THIEF_CLIMB_A];
                ctx.actors.set_anims(&ctx.av, actor, climb, climb, false, STATE_STOPPED);
                ctx.actors.set_pos(actor, 312.0, self.thief_y);
            }
            ACTOR_PLAYER => {
                self.player_state = PLAYER_STANDING;
                let stance = self.anims[ANIM_STANCE];
                ctx.actors.set_anims(&ctx.av, actor, stance, stance, false, STATE_RESTART_FORWARD);
                ctx.actors.set_turn_anims(actor, self.anims[ANIM_JUMP_TURN_RIGHT], self.anims[ANIM_JUMP_TURN_LEFT]);
                ctx.actors.set_pos(actor, 280.0, self.ground_y);
                ctx.actors.face(&ctx.av, actor, FACING_NONE);
                ctx.actors.set_bbox_anim(actor, stance);
                if !self.intro_pending {
                    let drop = drop_height(ctx.player.stage);
                    self.player_state = PLAYER_AIRBORNE;
                    ctx.actors.set_visible(actor, true);
                    ctx.actors.set_pos(actor, 280.0, drop + self.ground_y);
                    let falling = self.anims[ANIM_FALLING_LOOP];
                    ctx.actors.set_anims(&ctx.av, actor, falling, falling, false, STATE_RESTART_FORWARD);
                    let ground = ftol(self.ground_y as f64);
                    ctx.actors.set_gravity(actor, 1.0, 1.0, ground, MAX_FALL_SPEED);
                } else {
                    ctx.actors.set_visible(actor, false);
                }
            }
            ACTOR_CAPN => {
                if !self.intro_pending {
                    ctx.actors.set_pos(ACTOR_CAPN, 512.0, 4.0);
                    let idle = self.anims[ANIM_CAP_TV_IDLE];
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
                    ctx.actors.set_screen_fixed(ACTOR_CAPN, true);
                    ctx.actors.set_draw_offset(ACTOR_CAPN, 0, 0);
                } else {
                    ctx.actors.set_visible(actor, false);
                }
            }
            ACTOR_TV => {
                let fly = self.anims[ANIM_HOVER_FLY];
                ctx.actors.set_anims(&ctx.av, actor, fly, fly, false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(actor, 100.0, self.fly_y);
            }
            ACTOR_COUNTDOWN => ctx.actors.set_visible(actor, false),
            _ => {}
        }
    }

    // Returns the Cap'n to idle once his worried animation has played out
    fn on_anim_end<H: Host>(&mut self, ctx: &mut Ctx<H>, actor: i32) {
        if actor == ACTOR_CAPN && self.capn_state == CAPN_WORRIED {
            self.capn_state = CAPN_IDLE;
            let idle = self.anims[ANIM_CAP_TV_IDLE];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
        }
    }

    // Finish bookkeeping, thief climb, jump, steering, movement and the clock while the race runs
    fn update_race<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.countdown != 0 {
            self.countdown -= 1;
            if self.countdown == 1 {
                ctx.actors.set_visible(ACTOR_COUNTDOWN, false);
            }
        }
        if self.finished {
            if self.result == RESULT_UNDECIDED {
                self.result = RESULT_PLAYER_FIRST;
                if self.thief_y <= 100.0 {
                    self.result = RESULT_THIEF_FIRST;
                }
            }
            if self.player_state == PLAYER_STANDING {
                self.player_state = PLAYER_FINISHED;
                if ctx.player.challenge_active == 0 {
                    ctx.actors.set_visible(ACTOR_CAPN, false);
                    self.capn_fly_state = FLY_START;
                    self.outro_active = true;
                    let old_best = time_text(ctx.player.jump_best_secs, ctx.player.jump_best_frac as u16 as i32);
                    if self.time_secs < ctx.player.jump_best_secs
                        || (ctx.player.jump_best_secs == self.time_secs
                            && self.time_frac as f64 * 5.0 < (ctx.player.jump_best_frac as u16) as f64)
                    {
                        ctx.player.jump_best_secs = self.time_secs;
                        ctx.player.jump_best_frac = (ftol(self.time_frac as f64) * 5) as i16;
                    }
                    ctx.scene.set_text_by_id(OBJ_BEST_TIME, &old_best);
                    ctx.scene.set_hover_by_id(OBJ_BEST_TITLE, HOVER_NORMAL);
                    ctx.scene.set_hover_by_id(OBJ_BEST_TIME, HOVER_NORMAL);
                    ctx.scene.set_hover_by_id(OBJ_DONE, HOVER_NORMAL);
                    ctx.scene.set_hover_by_id(OBJ_AGAIN, HOVER_NORMAL);
                    self.vx = 0.0;
                    ctx.cursor.set_visible(true);
                } else if self.result == RESULT_PLAYER_FIRST {
                    ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_NORMAL);
                }
            }
            self.timer_reset = true;
        }
        if self.challenge != 0 {
            let climb = self.anims[ANIM_THIEF_CLIMB_B];
            if self.thief_state == THIEF_STOPPED {
                ctx.actors.set_anims(&ctx.av, ACTOR_THIEF, climb, climb, false, STATE_RESTART_FORWARD);
                self.thief_state = THIEF_CLIMBING;
            } else if self.thief_y <= 100.0 {
                if self.thief_state == THIEF_CLIMBING {
                    ctx.actors.set_anims(&ctx.av, ACTOR_THIEF, climb, climb, false, STATE_STOPPED);
                    self.thief_state = THIEF_STOPPED;
                }
            } else {
                self.thief_y -= 3.0;
                ctx.actors.set_y(ACTOR_THIEF, self.thief_y);
            }
        }
        if (self.player_state == PLAYER_STANDING || self.player_state == PLAYER_RUNNING) && ctx.av.is_key_down(KEY_SPACE) {
            ctx.av.play_sfx(self.sfx[SFX_JUMP], -1);
            ctx.actors.set_anims(
                &ctx.av,
                ACTOR_PLAYER,
                self.anims[ANIM_JUMPING_RIGHT],
                self.anims[ANIM_JUMPING_LEFT],
                false,
                STATE_RESTART_FORWARD,
            );
            let ground = ftol(self.ground_y as f64);
            let vy = (-(ctx.player.jumping as f64 * 0.5 + 15.0)) as f32;
            ctx.actors.set_gravity(ACTOR_PLAYER, vy, JUMP_GRAVITY, ground, MAX_FALL_SPEED);
            self.player_state = PLAYER_AIRBORNE;
        }
        if self.player_state == PLAYER_STANDING || self.player_state == PLAYER_AIRBORNE || self.player_state == PLAYER_RUNNING {
            if ctx.av.is_key_down(KEY_RIGHT) {
                ctx.actors.face(&ctx.av, ACTOR_PLAYER, FACING_RIGHT);
                if self.vx < 0.0 && self.player_state != PLAYER_AIRBORNE {
                    self.vx = 0.0;
                }
                self.vx += self.run_accel;
                if self.max_run_speed < self.vx {
                    self.vx = self.max_run_speed;
                }
            } else if ctx.av.is_key_down(KEY_LEFT) {
                ctx.actors.face(&ctx.av, ACTOR_PLAYER, FACING_LEFT);
                if 0.0 < self.vx && self.player_state != PLAYER_AIRBORNE {
                    self.vx = 0.0;
                }
                self.vx -= self.run_accel;
                if self.vx < -self.max_run_speed {
                    self.vx = -self.max_run_speed;
                }
            } else if self.player_state == PLAYER_STANDING || self.player_state == PLAYER_RUNNING {
                if self.player_state != PLAYER_STANDING {
                    let stance = self.anims[ANIM_STANCE];
                    ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, stance, stance, false, STATE_RESTART_FORWARD);
                    ctx.actors.face(&ctx.av, ACTOR_PLAYER, FACING_NONE);
                    self.player_state = PLAYER_STANDING;
                }
                self.vx = 0.0;
            }
            if self.vx != 0.0 && self.player_state == PLAYER_STANDING {
                ctx.actors.set_anims(
                    &ctx.av,
                    ACTOR_PLAYER,
                    self.anims[ANIM_RUNNING_RIGHT],
                    self.anims[ANIM_RUNNING_LEFT],
                    false,
                    STATE_RESTART_FORWARD,
                );
                self.player_state = PLAYER_RUNNING;
            }
            let vy = ctx.actors.vel_y(ACTOR_PLAYER);
            if vy < MAX_FALL_SPEED {
                self.fall_frames = 0;
            } else {
                self.fall_frames += 1;
                if FALL_WARNING_FRAMES < self.fall_frames && self.capn_state != CAPN_TALKING && self.capn_state != CAPN_WORRIED {
                    let worried = self.anims[ANIM_CAP_TV_WORRIED];
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, worried, worried, true, STATE_RESTART_FORWARD);
                    self.capn_state = CAPN_WORRIED;
                }
            }
        }
        if self.vx != 0.0 && !self.finished {
            let (mut x, _y) = ctx.actors.pos(ACTOR_PLAYER);
            x += self.vx;
            if x <= 560.0 {
                if x < 30.0 {
                    x = 30.0;
                }
            } else {
                x = 560.0;
            }
            ctx.actors.set_x(ACTOR_PLAYER, x);
        }
        let now = ctx.time_ms();
        let mut dt = now.wrapping_sub(self.last_time) as f32;
        if self.timer_reset {
            dt = 0.0;
            self.timer_reset = false;
        }
        self.last_time = now;
        self.time_frac = (dt as f64 / 50.0 + self.time_frac as f64) as f32;
        while 20.0 <= self.time_frac {
            self.time_frac -= 20.0;
            self.time_secs = self.time_secs.wrapping_add(1);
        }
        let text = time_text(self.time_secs, ftol(self.time_frac as f64) * 5);
        ctx.scene.set_text_by_id(OBJ_TIME, &text);
    }

    // Counts down through three, two, one, go with the Cap'n's challenge speech first
    async fn update_countdown<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.challenge == 0 || self.challenge == 3 {
            self.countdown -= 1;
        } else if self.challenge == 1 {
            ctx.actors.set_visible(ACTOR_CAPN, true);
            ctx.actors.set_pos(ACTOR_CAPN, 512.0, 4.0);
            ctx.actors.set_screen_fixed(ACTOR_CAPN, true);
            ctx.actors.set_draw_offset(ACTOR_CAPN, 0, 0);
            let talk = self.anims[ANIM_CAP_TV_TALK];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "049.wav", false, self.voice).await;
            self.challenge += 1;
        } else if ctx.stream_done(self.voice) {
            self.challenge += 1;
        }
        if self.countdown == COUNTDOWN_ACTIVE {
            let go = self.anims[ANIM_GO];
            ctx.actors.set_anims(&ctx.av, ACTOR_COUNTDOWN, go, go, false, STATE_STOPPED);
            let stance = self.anims[ANIM_STANCE];
            ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, stance, stance, false, STATE_RESTART_FORWARD);
            ctx.scene.set_hover_by_id(OBJ_TIME, HOVER_NORMAL);
            ctx.scene.set_hover_by_id(OBJ_TIME_TITLE, HOVER_NORMAL);
        } else if self.countdown == COUNTDOWN_ONE {
            let one = self.anims[ANIM_ONE];
            ctx.actors.set_anims(&ctx.av, ACTOR_COUNTDOWN, one, one, false, STATE_STOPPED);
        } else if self.countdown == COUNTDOWN_TWO {
            let two = self.anims[ANIM_TWO];
            ctx.actors.set_anims(&ctx.av, ACTOR_COUNTDOWN, two, two, false, STATE_STOPPED);
        } else if self.countdown == COUNTDOWN_THREE {
            let three = self.anims[ANIM_THREE];
            ctx.actors.set_anims(&ctx.av, ACTOR_COUNTDOWN, three, three, false, STATE_STOPPED);
            ctx.actors.set_visible(ACTOR_COUNTDOWN, true);
            ctx.actors.set_screen_fixed(ACTOR_COUNTDOWN, true);
            ctx.actors.set_layer_mask(ACTOR_COUNTDOWN, 8);
            ctx.actors.set_pos(ACTOR_COUNTDOWN, 320.0, 180.0);
            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "032.wav", false, self.voice).await;
            let talk = self.anims[ANIM_CAP_TV_TALK];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
            self.capn_state = CAPN_TALKING;
        }
    }

    // Flies the Cap'n and TV in, speaks, drops the crunchling in and flies back out
    async fn update_intro<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.capn_fly_state == FLY_START {
            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "036.wav", false, self.voice).await;
            ctx.actors.set_visible(ACTOR_CAPN, true);
            let talk = self.anims[ANIM_CAPN_FLY_TALK];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
            ctx.actors.set_draw_offset(ACTOR_CAPN, 0xa6, -0x52);
            self.capn_fly_state = FLY_ARRIVING;
        }
        let speed = fly_speed(self.fly_y);
        let mut done = false;
        let mut arrived = false;
        if self.capn_fly_state == FLY_LEAVING {
            self.fly_y -= speed;
            done = self.fly_y < -400.0;
        } else if self.capn_fly_state == FLY_ARRIVING {
            arrived = 240.0 <= self.fly_y;
            if !arrived {
                self.fly_y += speed;
            }
        } else if self.capn_fly_state == FLY_SPEAKING && ctx.stream_done(self.voice) {
            let idle = self.anims[ANIM_CAPN_FLY_IDLE];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
            self.capn_fly_state = FLY_LEAVING;
        }
        if ctx.av.is_key_down(KEY_SPACE) {
            done = true;
            if self.capn_fly_state == FLY_ARRIVING {
                arrived = true;
            }
            ctx.av.kill_streaming_channel(self.voice);
        }
        ctx.actors.set_pos(ACTOR_TV, 100.0, self.base_y + self.fly_y);
        ctx.actors.set_pos(ACTOR_CAPN, 100.0, self.base_y + self.fly_y);
        if done {
            ctx.actors.set_visible(ACTOR_TV, false);
            self.intro_pending = false;
            ctx.actors.set_pos(ACTOR_CAPN, 512.0, 4.0);
            let idle = self.anims[ANIM_CAP_TV_IDLE];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
            ctx.actors.set_screen_fixed(ACTOR_CAPN, true);
            ctx.actors.set_draw_offset(ACTOR_CAPN, 0, 0);
        }
        if arrived {
            let drop = drop_height(ctx.player.stage);
            self.player_state = PLAYER_AIRBORNE;
            ctx.actors.set_visible(ACTOR_PLAYER, true);
            ctx.actors.set_pos(ACTOR_PLAYER, 280.0, drop + self.ground_y);
            let falling = self.anims[ANIM_FALLING_LOOP];
            ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, falling, falling, false, STATE_RESTART_FORWARD);
            let ground = ftol(self.ground_y as f64);
            ctx.actors.set_gravity(ACTOR_PLAYER, 1.0, 1.0, ground, MAX_FALL_SPEED);
            self.capn_fly_state = FLY_SPEAKING;
        }
    }

    // Flies the Cap'n and TV down after the finish, speaks, and flies back up
    async fn update_outro<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        match self.capn_fly_state {
            FLY_START => {
                ctx.actors.set_layer_mask(ACTOR_CAPN, 8);
                ctx.actors.set_layer_mask(ACTOR_TV, 8);
                ctx.actors.set_visible(ACTOR_CAPN, true);
                ctx.actors.set_visible(ACTOR_TV, true);
                let idle = self.anims[ANIM_CAPN_FLY_IDLE];
                ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
                ctx.actors.set_draw_offset(ACTOR_CAPN, 0xa6, -0x52);
                self.capn_fly_state = FLY_ARRIVING;
                self.fly_y = -300.0;
            }
            FLY_LEAVING if -350.0 < self.fly_y => {
                self.fly_y -= 6.0;
            }
            FLY_ARRIVING => {
                if 200.0 <= self.fly_y {
                    let talk = self.anims[ANIM_CAPN_FLY_TALK];
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
                    self.capn_fly_state = FLY_SPEAKING;
                    self.voice = ctx.av.play_stream_sound(&mut ctx.host, "037.wav", false, self.voice).await;
                } else {
                    self.fly_y += 6.0;
                }
            }
            FLY_SPEAKING if ctx.stream_done(self.voice) => {
                let idle = self.anims[ANIM_CAPN_FLY_IDLE];
                ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
                self.capn_fly_state = FLY_LEAVING;
            }
            _ => {}
        }
        ctx.actors.set_pos(ACTOR_TV, 100.0, self.fly_y);
        ctx.actors.set_pos(ACTOR_CAPN, 100.0, self.fly_y);
    }
}

impl<H: Host> Screen<H> for JumpScreen {
    // Loads the level for the stage or the challenge, builds its actors and resets every field
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.challenge = ctx.player.challenge_active as i32;
        self.intro_pending = self.challenge == 0;
        ctx.actors.clear();
        if self.challenge == 0 {
            if ctx.player.stage == 1 {
                ctx.scene.load(&mut ctx.av, &mut ctx.host, "jumpcomlev02").await;
            } else if ctx.player.stage == 2 {
                ctx.scene.load(&mut ctx.av, &mut ctx.host, "jumpcomlev03").await;
            } else {
                ctx.scene.load(&mut ctx.av, &mut ctx.host, "jumpcomlev01").await;
            }
        } else {
            ctx.scene.load(&mut ctx.av, &mut ctx.host, "jumpcomlev04").await;
            ctx.actors.add_actor(ACTOR_THIEF, true);
        }
        let bottom = ctx.scene.bounds[3];
        self.max_scroll_y = bottom - SCREEN_HEIGHT;
        ctx.scene.set_scroll(0, self.max_scroll_y);
        let mut next_actor = FIRST_PLATFORM_ACTOR;
        for i in 0..ctx.scene.object_count() as usize {
            let obj = ctx.scene.get_object(i);
            if obj.flags & FLAG_PLATFORM != 0 || obj.flags & FLAG_PICKUP != 0 {
                ctx.scene.set_hover_by_index(i, HOVER_HIDDEN);
                ctx.actors.add_actor(next_actor, false);
                ctx.actors.set_pos(next_actor, obj.x as f32, obj.y as f32);
                let anim = obj.anims[0].anim;
                ctx.actors.set_anims(&ctx.av, next_actor, anim, anim, false, STATE_RESTART_FORWARD);
                ctx.actors.set_path(next_actor, obj.extra0, obj.extra1, obj.extra2, obj.extra3);
                ctx.actors.set_flags(next_actor, platform_flags(obj.flags));
                next_actor += 1;
            }
        }
        ctx.scene.assign_id("time", OBJ_TIME);
        ctx.scene.assign_id("time Title", OBJ_TIME_TITLE);
        ctx.scene.assign_id("done", OBJ_DONE);
        ctx.scene.assign_id("BestTitle", OBJ_BEST_TITLE);
        ctx.scene.assign_id("BestTime", OBJ_BEST_TIME);
        ctx.scene.set_text_by_id(OBJ_TIME, "0.00");
        ctx.scene.set_hover_by_id(OBJ_TIME, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(OBJ_TIME_TITLE, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(OBJ_DONE, HOVER_HIDDEN);
        ctx.scene.set_text_by_id(OBJ_BEST_TIME, "0.00");
        ctx.scene.set_hover_by_id(OBJ_BEST_TITLE, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(OBJ_BEST_TIME, HOVER_HIDDEN);
        ctx.scene.assign_id("GoodJob", OBJ_GOOD_JOB);
        ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_HIDDEN);
        if self.challenge == 0 {
            ctx.scene.assign_id("Again", OBJ_AGAIN);
            ctx.scene.set_hover_by_id(OBJ_AGAIN, HOVER_HIDDEN);
        } else {
            ctx.scene.set_text_by_id(OBJ_GOOD_JOB, "You Win!");
        }
        let Some(mouse) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        self.cursor_anim = mouse;
        ctx.cursor.set_anim(&ctx.av, self.cursor_anim);
        ctx.cursor.set_pos(0x140, 400);
        ctx.cursor.set_visible(true);
        let Some(h) = ctx.av.get_sfx(&mut ctx.host, "skatejump.wav", 0, false).await else { return false };
        self.sfx[SFX_JUMP] = h;
        let Some(h) = ctx.av.get_sfx(&mut ctx.host, "cs_berry.wav", 0, false).await else { return false };
        self.sfx[SFX_BERRY] = h;
        let Some(h) = ctx.av.get_sfx(&mut ctx.host, "jumpland.wav", 0, false).await else { return false };
        self.sfx[SFX_LAND] = h;
        self.backdrop.add_tile(&mut ctx.av, &mut ctx.host, "jumpc9bakdrop.bam", 0, 0).await;
        self.backdrop.set_speed_factor(0.05);
        self.backdrop.set_y(0.0);
        ctx.actors.add_actor(ACTOR_COUNTDOWN, true);
        ctx.actors.add_actor(ACTOR_CAPN, true);
        ctx.actors.add_actor(ACTOR_TV, true);
        ctx.actors.add_actor(ACTOR_PLAYER, true);
        if !self.load_anims(ctx).await {
            return false;
        }
        if self.challenge == 0 {
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "ps56Jumping.wav", true, 0).await;
        } else {
            let Some(h) = ctx.av.get_anim(&mut ctx.host, "thief_climb_climb").await else { return false };
            self.anims[ANIM_THIEF_CLIMB_A] = h;
            let Some(h) = ctx.av.get_anim(&mut ctx.host, "ugjump4pole").await else { return false };
            self.anims[ANIM_POLE] = h;
            let Some(h) = ctx.av.get_anim(&mut ctx.host, "thief_climb_climb").await else { return false };
            self.anims[ANIM_THIEF_CLIMB_B] = h;
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "ps57Challenge.wav", true, 0).await;
        }
        if ctx.player.stage == 0 {
            self.max_run_speed = 6.0;
        } else if ctx.player.stage == 1 {
            self.max_run_speed = 8.0;
        } else if ctx.player.stage == 2 {
            self.max_run_speed = 10.0;
        }
        self.run_accel = 1.0;
        self.vx = 0.0;
        self.ground_y = (bottom - 0x14) as f32;
        self.thief_y = (bottom - 0x140) as f32;
        self.player_state = PLAYER_NONE;
        self.berries = 0;
        self.time_secs = 0;
        self.time_frac = 0.0;
        self.timer_reset = true;
        self.countdown = COUNTDOWN_START;
        self.finished = false;
        ctx.cursor.set_visible(false);
        if 0 < ctx.player.jumping {
            self.intro_pending = false;
        }
        self.fly_y = -400.0;
        self.base_y = bottom as f32 - 480.0;
        self.capn_fly_state = FLY_START;
        self.voice = 0;
        self.capn_state = CAPN_NONE;
        self.outro_active = false;
        self.fall_frames = 0;
        self.end_delay = 0;
        self.result = RESULT_UNDECIDED;
        self.berry_streak = 0;
        true
    }

    // Draws the frame, runs input, physics, the active phase and the camera, then picks the result
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut ret = RESULT_CONTINUE;
        self.backdrop.scroll(&ctx.av, 0.0);
        self.backdrop.fill_and_draw(&mut ctx.av, &mut ctx.rand);
        ctx.scene.draw_layer(&mut ctx.av, 0);
        ctx.scene.draw_layer(&mut ctx.av, 2);
        if self.challenge != 0 {
            ctx.av.draw_anim_reset(self.anims[ANIM_POLE], 0x141, 0);
        }
        if ctx.cursor.visible {
            let (dx, dy) = ctx.av.get_mouse_delta();
            ctx.cursor.move_by(dx, dy);
            let (mx, my) = (ctx.cursor.x, ctx.cursor.y);
            if let Some(id) = ctx.scene.hit_test(mx as i32, my as i32)
                && ctx.av.is_mouse_button_down(MOUSE_LEFT)
            {
                ret = self.on_mouse_hit(ctx, id);
            }
        }
        ctx.actors.update(&ctx.av);
        while let Some(ev) = ctx.actors.pop_event() {
            self.on_actor_event(ctx, ev.kind, ev.actor_id).await;
        }
        ctx.actors.collect_pickups(&ctx.av, ACTOR_PLAYER);
        if self.countdown < COUNTDOWN_ACTIVE {
            self.update_race(ctx);
        } else if !self.intro_pending {
            self.update_countdown(ctx).await;
        } else {
            self.update_intro(ctx).await;
        }
        if self.outro_active {
            self.update_outro(ctx).await;
        }
        if self.capn_state == CAPN_TALKING && ctx.stream_done(self.voice) {
            self.capn_state = CAPN_IDLE;
            let idle = self.anims[ANIM_CAP_TV_IDLE];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
        }
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 1);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 4);
        ctx.scene.draw_layer(&mut ctx.av, 1);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 8);
        ctx.cursor.draw(&mut ctx.av);
        ctx.scene.advance_anims(&ctx.av);
        let (_px, py) = ctx.actors.pos(ACTOR_PLAYER);
        let (sx, sy) = (ctx.scene.scroll_x, ctx.scene.scroll_y);
        let rel = (py as f64 - sy as f64) as f32;
        let sy = camera_scroll_y(rel, sy, self.max_scroll_y);
        if ctx.av.is_key_down(KEY_ESC) {
            ret = STATE_HOME_WORLD;
        }
        ctx.scene.set_scroll(sx, sy);
        if self.challenge != 0 && self.player_state == PLAYER_FINISHED {
            self.end_delay += 1;
            if CHALLENGE_EXIT_FRAMES < self.end_delay {
                ctx.player.challenge_won = (self.result == RESULT_PLAYER_FIRST) as u8;
                ret = STATE_CHALLENGE;
            }
        }
        ret
    }

    fn shutdown(&mut self, ctx: &mut Ctx<H>) {
        ctx.av.kill_streaming_channel(self.music);
        ctx.av.kill_streaming_channel(self.voice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_flags_prefer_pickup_then_finish() {
        assert_eq!(platform_flags(FLAG_PLATFORM), 0);
        assert_eq!(platform_flags(FLAG_PLATFORM | FLAG_FINISH), ACTOR_FLAG_FINISH);
        assert_eq!(platform_flags(FLAG_PICKUP | FLAG_FINISH), ACTOR_FLAG_PICKUP);
    }

    #[test]
    fn time_text_pads_hundredths() {
        assert_eq!(time_text(100, 0), " 100.00");
        assert_eq!(time_text(3, 5), " 3.05");
        assert_eq!(time_text(12, 95), " 12.95");
    }

    #[test]
    fn fly_speed_slows_near_the_stop() {
        assert_eq!(fly_speed(-400.0), 8.0);
        assert_eq!(fly_speed(100.0), 4.0);
        assert_eq!(fly_speed(180.0), 2.0);
        assert_eq!(fly_speed(300.0), 12.0);
    }

    #[test]
    fn drop_height_deepens_for_adults() {
        assert_eq!(drop_height(0), -100.0);
        assert_eq!(drop_height(1), -100.0);
        assert_eq!(drop_height(2), -150.0);
    }

    #[test]
    fn camera_zones_match_the_original() {
        assert_eq!(camera_scroll_y(190.0, 50, 400), 50);
        assert_eq!(camera_scroll_y(150.0, 50, 400), 49);
        assert_eq!(camera_scroll_y(150.0, 0, 400), 0);
        assert_eq!(camera_scroll_y(250.0, 50, 400), 51);
        assert_eq!(camera_scroll_y(250.0, 400, 400), 400);
        assert_eq!(camera_scroll_y(300.5, 390, 400), 400);
        assert_eq!(camera_scroll_y(290.5, 50, 400), 60);
        assert_eq!(camera_scroll_y(50.5, 100, 400), 41);
        assert_eq!(camera_scroll_y(10.0, 50, 400), 0);
    }
}
