use avmanager::anim::{STATE_RESTART_FORWARD, STATE_STOPPED};
use avmanager::crt::ftol;
use avmanager::sound::INVALID_HANDLE;

use crate::actors::{EVENT_ANIM_END, EVENT_GROUND_REACHED, EVENT_INIT_OBJECT, FACING_NONE, NO_HANDLE};
use crate::ctx::{
    Ctx, Held, KEY_ESC, KEY_SPACE, MOUSE_LEFT, RESULT_CONTINUE, STATE_CHALLENGE, STATE_HOME_WORLD, STATE_RUNNING,
    Screen,
};
use crate::host::Host;
use crate::online::wins_text;
use crate::scene::{HOVER_HIDDEN, HOVER_NORMAL, LAYER_ALL};
use crate::scroll_strip::ScrollStrip;
use crate::skin::Skin;

const A_OPP_RUN: usize = 10;
const A_OPP_WAIT: usize = 11;
const A_OPP_VICTORY: usize = 12;
const A_OPP_LOSE: usize = 13;
const A_CAPN_028: usize = 14;
const A_CAPN_029: usize = 15;
const A_CAPN_013: usize = 16;
const A_CAPN_015: usize = 17;
const A_TV_005: usize = 18;
const A_TV_006: usize = 19;
const A_TV_004: usize = 20;
const A_CAR: usize = 21;
const A_THREE: usize = 22;
const A_TWO: usize = 23;
const A_ONE: usize = 24;
const A_GO: usize = 25;
const A_BERRY: usize = 28;
const A_BISCUIT: usize = 29;
const A_BLOCK: usize = 30;
const A_BLOCK_ANIM: usize = 31;
const A_CART: usize = 32;
const A_CAN: usize = 33;
const A_CAN_ANIM: usize = 34;
const A_LINE: usize = 35;
const A_PLATFORM: usize = 36;
const A_BARRICADE: usize = 37;
const A_BARRICADE_ANIM: usize = 38;
const ANIM_COUNT: usize = 39;

// Handle table of one skater's own crunchling animations
const SK_SKATING: usize = 0;
const SK_JUMP: usize = 1;
const SK_CRASH: usize = 2;
const SK_GETUP: usize = 3;
const SK_VICTORY: usize = 4;
const SK_GOTO_DEFEAT: usize = 5;
const SK_DEFEAT: usize = 6;
const SK_SPIN: usize = 7;
const SK_STRETCH: usize = 8;
const SK_COUNT: usize = 9;

const SFX_LOOP: usize = 0;
const SFX_JUMP: usize = 1;
const SFX_LAND: usize = 2;
const SFX_CRASH: usize = 3;
const SFX_BERRY: usize = 4;
const SFX_COUNT: usize = 5;

const ACTOR_PLAYER: i32 = 0;
const ACTOR_OPPONENT: i32 = 1;
const ACTOR_CAR: i32 = 2;
const ACTOR_CAPN: i32 = 3;
const ACTOR_SIGN: i32 = 0xc;

const PS_SKATING: i32 = 1;
const PS_RIDING: i32 = 2;
const PS_AIRBORNE: i32 = 3;
const PS_CRASH_AIR: i32 = 4;
const PS_CRASH_SLIDE: i32 = 5;
const PS_GETTING_UP: i32 = 6;
const PS_FINISHED: i32 = 7;

const COMM_DRIVE_IN: i32 = 0;
const COMM_STANDING: i32 = 7;
const COMM_DRIVE_OUT: i32 = 8;
const COMM_INTRO_SPEECH: i32 = 9;
const COMM_TV_IDLE: i32 = 10;
const COMM_TV_SPEECH: i32 = 0xb;
const COMM_LAUGH: i32 = 0xc;
const COMM_END_DRIVE_IN: i32 = 0xd;
const COMM_END_SPEECH: i32 = 0xe;

const KIND_CRASH: i32 = 0;
const KIND_BERRY: i32 = 1;
const KIND_PLATFORM: i32 = 2;
const KIND_SLOWDOWN: i32 = 3;
const KIND_FINISH: i32 = 4;

const RACE_NONE: i32 = 0;
const RACE_PLAYER_FIRST: i32 = 1;
const RACE_OPPONENT_FIRST: i32 = 2;

const LAYER_MAIN: u32 = 1;
const LAYER_FRONT: u32 = 4;
const LAYER_TOP: u32 = 8;

const ACCEL: f32 = 0.6;
const BRAKE: f32 = 0.7;
const OBJ_BERRIES: i32 = 1;
const OBJ_TIME: i32 = 2;
const OBJ_GOOD_JOB: i32 = 7;

// Friend match camera, the midpoint of both skaters drawn here and a lead this long ending the race
const VERSUS_CENTER: f32 = 280.0;
const VERSUS_GAP_LIMIT: f32 = 520.0;
const VERSUS_SPAWN_X: f32 = 900.0;
const VERSUS_RIVAL_DEPTH: f32 = 8.0;
const LINE_SCREEN_X: f32 = 700.0;
const CROSS_PAST: f32 = 60.0;

// Table at 0x00429d08 indexed by anim_index, draw offset dx and dy then the platform ride y shift
const PLAYER_OFFSETS: [[i32; 3]; 9] = [
    [0, 11, 22],
    [0, 11, 22],
    [0, 11, 22],
    [0, 0, 16],
    [0, 0, 16],
    [0, 0, 16],
    [0, -24, 30],
    [0, -24, 30],
    [0, -24, 30],
];

// One obstacle actor with its kind, idle and after hit anims and spawn bookkeeping
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObstacleSlot {
    actor_id: i32,
    kind: i32,
    idle: u16,
    alt: u16,
    hit: bool,
    active: bool,
}

// One racing crunchling, the resident one or the visiting friend's, with its anims, speed and distance
#[derive(Clone, Copy, Debug, Default)]
struct Skater {
    actor: i32,
    friend: bool,
    anims: [u16; SK_COUNT],
    bbox: u16,
    state: i32,
    speed: f32,
    max_speed: f32,
    pos: f32,
    berries: u32,
    platform_jumps: u32,
    ride_shift: f32,
    ground_y: f32,
}

impl Skater {
    // Top speed from the running stat, the race track's number
    fn top_speed(running: i32) -> f32 {
        (f64::from(running) * f64::from(0.3f32) + 14.0) as f32
    }

    fn racing(&self) -> bool {
        matches!(self.state, PS_SKATING | PS_RIDING | PS_AIRBORNE)
    }
}

// The skating race against the turtle, the thief through the mine, or a visiting friend's crunchling
#[derive(Debug)]
pub struct RunningScreen {
    strips: [ScrollStrip; 5],
    slots: Vec<ObstacleSlot>,
    anims: [u16; ANIM_COUNT],
    sfx: [u16; SFX_COUNT],
    skaters: Vec<Skater>,
    ground_y: f32,
    opp_speed: f32,
    opp_max_speed: f32,
    opp_accel: f32,
    opp_decel: f32,
    opp_y: f32,
    loop_channel: u32,
    music: u32,
    voice: u32,
    spawn_dist: f32,
    last_tick_ms: u32,
    fraction: f32,
    seconds: u16,
    timer_resync: bool,
    countdown: u32,
    finish_dist: f32,
    half_dist: f32,
    cross_dist: f32,
    opp_finish_dist: f32,
    line_pos: Option<f32>,
    winner: Option<usize>,
    result: i32,
    commentator: i32,
    intro_pending: bool,
    challenge: i32,
    car_speed: f32,
    finish_frames: u32,
}

// Formats a race time the way sprintf(" %d.%02d") did
fn time_text(seconds: u16, hundredths: i32) -> String {
    format!(" {seconds}.{hundredths:02}")
}

// Screen x of a skater in a friend match, half the lead over the other away from the camera centre
fn versus_screen_x(pos: f32, other: f32) -> f32 {
    VERSUS_CENTER + (pos - other) / 2.0
}

// Who a lead too long for one screen hands the race to, None while both are still in frame
fn left_behind(lead: f32) -> Option<usize> {
    if lead > VERSUS_GAP_LIMIT {
        Some(0)
    } else if lead < -VERSUS_GAP_LIMIT {
        Some(1)
    } else {
        None
    }
}

impl Default for RunningScreen {
    fn default() -> Self {
        Self {
            strips: Default::default(),
            slots: Vec::new(),
            anims: [0; ANIM_COUNT],
            sfx: [0; SFX_COUNT],
            skaters: Vec::new(),
            ground_y: 0.0,
            opp_speed: 0.0,
            opp_max_speed: 0.0,
            opp_accel: 0.0,
            opp_decel: 0.0,
            opp_y: 0.0,
            loop_channel: INVALID_HANDLE,
            music: 0,
            voice: 0,
            spawn_dist: 0.0,
            last_tick_ms: 0,
            fraction: 0.0,
            seconds: 0,
            timer_resync: false,
            countdown: 0,
            finish_dist: 0.0,
            half_dist: 0.0,
            cross_dist: 0.0,
            opp_finish_dist: 0.0,
            line_pos: None,
            winner: None,
            result: RACE_NONE,
            commentator: COMM_DRIVE_IN,
            intro_pending: false,
            challenge: 0,
            car_speed: 0.0,
            finish_frames: 0,
        }
    }
}

impl RunningScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Appends an obstacle slot with its actor, kind and the idle and after hit anims
    fn add_obstacle_slot(&mut self, idle: u16, alt: u16, actor_id: i32, kind: i32) {
        self.slots.push(ObstacleSlot { actor_id, kind, idle, alt, hit: false, active: false });
    }

    // True while the resident crunchling races the visiting friend's rather than the turtle or thief
    fn versus(&self) -> bool {
        self.skaters.len() == 2
    }

    // Speed the world scrolls at, the resident skater's or the midpoint of both in a friend match
    fn cam_speed(&self) -> f32 {
        if self.versus() { (self.skaters[0].speed + self.skaters[1].speed) / 2.0 } else { self.skaters[0].speed }
    }

    fn skater_index(&self, actor: i32) -> Option<usize> {
        self.skaters.iter().position(|s| s.actor == actor)
    }

    // Runs the turtle or thief on its own until its finish, then brakes it
    fn update_opponent_speed(&mut self) {
        self.opp_finish_dist -= self.opp_speed;
        if 0.0 <= self.opp_finish_dist {
            self.opp_speed += self.opp_accel;
            if self.opp_max_speed < self.opp_speed {
                self.opp_speed = self.opp_max_speed;
            }
        } else {
            self.opp_speed = (f64::from(self.opp_speed) - f64::from(self.opp_decel) * f64::from(0.4f32)) as f32;
            if self.opp_speed < 0.0 {
                self.opp_speed = 0.0;
            }
        }
    }

    // Loads a skater's nine crunchling animations through its own recoloured palette
    async fn load_skater<H: Host>(ctx: &mut Ctx<H>, skin: Skin<'_>, actor: i32, friend: bool) -> Option<Skater> {
        let adult_as_type0 = skin.data.shown_stage() == 1;
        let (_, palette) = skin.palette(&mut ctx.av, &mut ctx.host, "checkPads", false, adult_as_type0).await?;
        let mut skater = Skater { actor, friend, bbox: NO_HANDLE, ..Skater::default() };
        let suffixes = [
            (SK_SKATING, "skating"),
            (SK_JUMP, "jump"),
            (SK_CRASH, "crash"),
            (SK_GETUP, "getup"),
            (SK_VICTORY, "victory"),
            (SK_GOTO_DEFEAT, "gotoDefeat"),
            (SK_DEFEAT, "defeat"),
            (SK_SPIN, "spin"),
            (SK_STRETCH, "stretch"),
        ];
        for (index, suffix) in suffixes {
            skater.anims[index] = skin.anim(&mut ctx.av, &mut ctx.host, suffix, false, adult_as_type0, &palette).await?;
        }
        if skin.data.shown_stage() == 2 {
            skater.bbox = ctx.av.get_anim(&mut ctx.host, "adult_sk8_bbox").await?;
        }
        let row = PLAYER_OFFSETS[skin.data.anim_index(false) as usize];
        ctx.actors.set_draw_offset(actor, row[0], row[1]);
        skater.ride_shift = row[2] as f32;
        skater.max_speed = Skater::top_speed(skin.data.running);
        Some(skater)
    }

    // Rewards the running stat for done or Again and picks the next state
    fn on_mouse_hit<H: Host>(&self, ctx: &mut Ctx<H>, id: i32) -> u32 {
        let mut result = RESULT_CONTINUE;
        let berries = self.skaters[0].berries;
        if id == 5 {
            if ctx.player.food != 0 {
                ctx.player.food -= 1;
            }
            if ctx.player.happiness != 0 {
                ctx.player.happiness -= 1;
            }
            ctx.player.add_stats(1, 0, 0);
            if 4 < berries {
                ctx.player.add_stats(1, 0, 0);
            }
            result = STATE_HOME_WORLD;
        } else if id == 6 {
            ctx.player.add_stats(1, 0, 0);
            if 4 < berries {
                ctx.player.add_stats(1, 0, 0);
            }
            result = STATE_RUNNING;
        }
        result
    }

    // Dispatches INIT_OBJECT and ANIM_END and lands a skater on GROUND_REACHED
    async fn on_actor_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, actor_id: i32) {
        if kind == EVENT_INIT_OBJECT {
            self.on_init_object(ctx, actor_id).await;
        } else if kind == EVENT_ANIM_END {
            self.on_anim_end(ctx, actor_id);
        } else if kind == EVENT_GROUND_REACHED && let Some(i) = self.skater_index(actor_id) {
            let s = &mut self.skaters[i];
            if s.state == PS_AIRBORNE || s.state == PS_RIDING {
                let skating = s.anims[SK_SKATING];
                ctx.actors.set_anims(&ctx.av, s.actor, skating, skating, false, STATE_RESTART_FORWARD);
                s.state = PS_SKATING;
                ctx.av.play_sfx(self.sfx[SFX_LAND], -1);
            } else if s.state == PS_CRASH_AIR {
                s.state = PS_CRASH_SLIDE;
            }
        }
    }

    // Places the skaters, the opponent, car and commentator and hides every other actor
    async fn on_init_object<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        let a = self.anims;
        if let Some(i) = self.skater_index(actor_id) {
            let s = &mut self.skaters[i];
            let stretch = s.anims[SK_STRETCH];
            ctx.actors.set_anims(&ctx.av, actor_id, stretch, stretch, false, STATE_RESTART_FORWARD);
            let x = if i == 0 { 40.0 } else { 90.0 };
            ctx.actors.set_pos(actor_id, x, s.ground_y);
            ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
            if s.bbox != NO_HANDLE {
                ctx.actors.set_bbox_anim(actor_id, s.bbox);
            }
            s.state = PS_SKATING;
            return;
        }
        match actor_id {
            ACTOR_OPPONENT => {
                ctx.actors.set_anims(&ctx.av, actor_id, a[A_OPP_WAIT], a[A_OPP_WAIT], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(actor_id, 90.0, self.opp_y);
                ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
            }
            ACTOR_CAR => {
                if self.challenge == 0 {
                    ctx.actors.set_anims(&ctx.av, actor_id, a[A_CAR], a[A_CAR], false, STATE_RESTART_FORWARD);
                    ctx.actors.set_pos(actor_id, -650.0, 350.0);
                    ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
                    ctx.actors.set_layer_mask(actor_id, LAYER_FRONT);
                    self.car_speed = 42.0;
                    if !self.intro_pending {
                        ctx.actors.set_pos(ACTOR_CAR, 250.0, 350.0);
                        ctx.actors.set_anims(&ctx.av, ACTOR_CAR, a[A_CAR], a[A_CAR], false, STATE_STOPPED);
                    }
                } else {
                    ctx.actors.set_visible(actor_id, false);
                }
            }
            ACTOR_CAPN => {
                ctx.actors.set_anims(&ctx.av, actor_id, a[A_CAPN_028], a[A_CAPN_028], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(actor_id, -500.0, 290.0);
                ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
                ctx.actors.set_layer_mask(actor_id, LAYER_FRONT);
                self.commentator = COMM_DRIVE_IN;
                if self.challenge != 0 {
                    ctx.actors.set_visible(actor_id, false);
                    self.commentator = COMM_INTRO_SPEECH;
                }
                if !self.intro_pending {
                    ctx.actors.set_pos(actor_id, 400.0, 290.0);
                    ctx.actors.set_anims_delayed(
                        &ctx.av,
                        actor_id,
                        0xf,
                        a[A_CAPN_029],
                        a[A_CAPN_029],
                        false,
                        STATE_RESTART_FORWARD,
                    );
                    self.voice = ctx.av.play_stream_sound(&mut ctx.host, "032.wav", false, self.voice).await;
                    self.commentator = COMM_DRIVE_OUT;
                }
            }
            4..=0xc => ctx.actors.set_visible(actor_id, false),
            _ => {}
        }
    }

    // Returns a skater to skating or the defeat hold and ends the commentator laugh
    fn on_anim_end<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        let a = self.anims;
        if let Some(i) = self.skater_index(actor_id) {
            let s = &mut self.skaters[i];
            if s.state == PS_FINISHED {
                let defeat = s.anims[SK_DEFEAT];
                ctx.actors.set_anims(&ctx.av, s.actor, defeat, defeat, false, STATE_RESTART_FORWARD);
            } else {
                let skating = s.anims[SK_SKATING];
                ctx.actors.set_anims(&ctx.av, s.actor, skating, skating, false, STATE_RESTART_FORWARD);
                s.state = PS_SKATING;
            }
        } else if actor_id == ACTOR_CAPN && self.commentator == COMM_LAUGH {
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_005], a[A_TV_005], false, STATE_RESTART_FORWARD);
            self.commentator = COMM_TV_IDLE;
        }
    }

    // Sets every skater's screen x from its speed alone or from its lead in a friend match
    fn place_skaters<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.versus() {
            let (p0, p1) = (self.skaters[0].pos, self.skaters[1].pos);
            ctx.actors.set_x(self.skaters[0].actor, versus_screen_x(p0, p1));
            ctx.actors.set_x(self.skaters[1].actor, versus_screen_x(p1, p0));
        } else {
            let speed = self.skaters[0].speed;
            ctx.actors.set_x(ACTOR_PLAYER, (f64::from(speed) * 6.0 + 50.0) as f32);
            ctx.actors.add_x(ACTOR_OPPONENT, self.opp_speed - speed);
        }
    }

    // Jumps, accelerates and brakes one skater by the keys its side holds, or slides out of a crash
    async fn skater_input<H: Host>(&mut self, ctx: &mut Ctx<H>, i: usize, held: Held) {
        let s = self.sfx;
        let decided = self.winner.is_some();
        let skater = self.skaters[i];
        let held = if decided && skater.racing() { Held::default() } else { held };
        if (skater.state == PS_SKATING || skater.state == PS_RIDING) && held.space {
            ctx.av.play_sfx(s[SFX_JUMP], -1);
            let jump = skater.anims[SK_JUMP];
            ctx.actors.set_anims(&ctx.av, skater.actor, jump, jump, false, STATE_RESTART_FORWARD);
            ctx.actors.set_gravity(skater.actor, -24.0, 1.5, ftol(f64::from(skater.ground_y)), 100.0);
            if skater.state == PS_RIDING {
                let spin = skater.anims[SK_SPIN];
                ctx.actors.set_anims(&ctx.av, skater.actor, spin, spin, false, STATE_RESTART_FORWARD);
                ctx.actors.add_y(skater.actor, -20.0);
                self.skaters[i].platform_jumps += 1;
                if 2 < self.skaters[i].platform_jumps {
                    self.skaters[i].platform_jumps = 0;
                    if self.commentator == COMM_TV_IDLE {
                        let a = self.anims;
                        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_006], a[A_TV_006], false, STATE_RESTART_FORWARD);
                        if ctx.rand_range(0, 2) == 0 {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "052.wav", false, self.voice).await;
                        } else {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "053.wav", false, self.voice).await;
                        }
                        self.commentator = COMM_TV_SPEECH;
                    }
                }
            }
            self.skaters[i].state = PS_AIRBORNE;
        }
        let skater = &mut self.skaters[i];
        if skater.racing() {
            if held.right {
                skater.speed += ACCEL;
                if skater.max_speed < skater.speed {
                    skater.speed = skater.max_speed;
                }
            }
            if held.left {
                skater.speed -= BRAKE;
                if skater.speed < 0.5 {
                    skater.speed = 0.5;
                }
            }
        } else if skater.state == PS_CRASH_SLIDE {
            skater.speed -= 0.25;
            if skater.speed < 0.5 {
                skater.speed = 0.5;
                let getup = skater.anims[SK_GETUP];
                ctx.actors.set_anims(&ctx.av, skater.actor, getup, getup, true, STATE_RESTART_FORWARD);
                skater.state = PS_GETTING_UP;
            }
        } else if skater.state == PS_FINISHED && decided {
            skater.speed = (f64::from(skater.speed) - f64::from(BRAKE) * f64::from(0.8f32)) as f32;
            if skater.speed < 0.0 {
                skater.speed = 0.0;
            }
        }
        self.skaters[i].pos += self.skaters[i].speed;
    }

    // Crashes, rides, slows or scores one skater against every active obstacle it touches
    fn skater_collide<H: Host>(&mut self, ctx: &mut Ctx<H>, i: usize) {
        let a = self.anims;
        let s = self.sfx;
        for slot_index in 0..self.slots.len() {
            let slot = self.slots[slot_index];
            let skater = self.skaters[i];
            if !slot.active || !ctx.actors.overlaps(&ctx.av, skater.actor, slot.actor_id) {
                continue;
            }
            let first = !slot.hit;
            let mut apply_alt = first;
            if slot.kind == KIND_CRASH || slot.kind == KIND_PLATFORM {
                let mut crash = first;
                if slot.kind == KIND_PLATFORM && (skater.state == PS_AIRBORNE || skater.state == PS_RIDING) {
                    let mut vy = ctx.actors.vel_y(skater.actor);
                    let me = ctx.actors.bbox(&ctx.av, skater.actor);
                    let obstacle = ctx.actors.bbox(&ctx.av, slot.actor_id);
                    if vy < 3.0 {
                        vy = 3.0;
                    }
                    if f64::from(me[3]) < f64::from(obstacle[1]) + f64::from(vy) {
                        ctx.actors.add_y(skater.actor, (-(me[3] - obstacle[1])) as f32);
                        ctx.actors.set_gravity(skater.actor, 0.0, 2.0, ftol(f64::from(skater.ground_y)), 100.0);
                        crash = false;
                        apply_alt = false;
                        if skater.state != PS_RIDING {
                            self.skaters[i].state = PS_RIDING;
                            ctx.av.play_sfx(s[SFX_LAND], -1);
                        }
                    }
                }
                if crash && skater.racing() {
                    if self.commentator != COMM_TV_SPEECH && self.commentator != COMM_LAUGH {
                        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_004], a[A_TV_004], true, STATE_RESTART_FORWARD);
                        self.commentator = COMM_LAUGH;
                    }
                    ctx.av.play_sfx(s[SFX_CRASH], -1);
                    let crash_anim = skater.anims[SK_CRASH];
                    ctx.actors.set_anims(&ctx.av, skater.actor, crash_anim, crash_anim, false, STATE_RESTART_FORWARD);
                    ctx.actors.set_gravity(skater.actor, -16.0, 2.0, ftol(f64::from(skater.ground_y)), 100.0);
                    self.skaters[i].state = PS_CRASH_AIR;
                    if self.skaters[i].speed < 7.0 {
                        self.skaters[i].speed = 7.0;
                    }
                }
            } else if first && slot.kind == KIND_BERRY {
                ctx.av.play_sfx(s[SFX_BERRY], -1);
                ctx.actors.set_visible(slot.actor_id, false);
                self.skaters[i].berries += 1;
                if i == 0 {
                    ctx.scene.set_text_by_id(OBJ_BERRIES, &self.skaters[0].berries.to_string());
                }
            } else if first && slot.kind == KIND_FINISH {
                apply_alt = false;
            } else if first {
                if self.skaters[i].speed <= 3.0 {
                    self.skaters[i].speed = 0.5;
                } else {
                    self.skaters[i].speed -= 3.0;
                }
            }
            if apply_alt {
                ctx.actors.set_anims(&ctx.av, slot.actor_id, slot.alt, slot.alt, false, STATE_RESTART_FORWARD);
                ctx.actors.set_layer_mask(slot.actor_id, LAYER_FRONT);
                self.slots[slot_index].hit = true;
            }
        }
    }

    // Poses a skater for the result once it is back on its board
    fn pose_result<H: Host>(&mut self, ctx: &mut Ctx<H>, i: usize, won: bool) {
        let skater = &mut self.skaters[i];
        if skater.state != PS_SKATING {
            return;
        }
        if won {
            let victory = skater.anims[SK_VICTORY];
            ctx.actors.set_anims(&ctx.av, skater.actor, victory, victory, false, STATE_RESTART_FORWARD);
        } else {
            let fall = skater.anims[SK_GOTO_DEFEAT];
            ctx.actors.set_anims(&ctx.av, skater.actor, fall, fall, true, STATE_RESTART_FORWARD);
        }
        ctx.actors.set_layer_mask(skater.actor, LAYER_MAIN);
        skater.state = PS_FINISHED;
    }

    // Sends the Cap'n back to his car for the drive in, or off screen in a challenge
    fn reset_capn<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let a = self.anims;
        self.car_speed = 42.0;
        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_CAPN_028], a[A_CAPN_028], false, STATE_RESTART_FORWARD);
        ctx.actors.set_anims(&ctx.av, ACTOR_CAR, a[A_CAR], a[A_CAR], false, STATE_RESTART_FORWARD);
        ctx.actors.set_pos(ACTOR_CAPN, -500.0, 290.0);
        ctx.actors.set_pos(ACTOR_CAR, -650.0, 350.0);
        ctx.actors.set_layer_mask(ACTOR_CAPN, LAYER_FRONT);
    }

    // Brakes the resident skater over the line, poses both racers and shows the times outside a challenge
    fn solo_finish<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let a = self.anims;
        self.cross_dist -= self.skaters[0].speed;
        if self.cross_dist >= 0.0 {
            return;
        }
        let skater = &mut self.skaters[0];
        skater.speed = (f64::from(skater.speed) - f64::from(BRAKE) * f64::from(0.8f32)) as f32;
        if skater.speed < 0.0 {
            skater.speed = 0.0;
        }
        let mut x = (f64::from(skater.speed) * 6.0 + 50.0 - f64::from(self.cross_dist)) as f32;
        if 500.0 < x {
            x = 500.0;
        }
        ctx.actors.set_x(ACTOR_PLAYER, x);
        if self.result == RACE_NONE {
            self.result = RACE_PLAYER_FIRST;
            if self.opp_finish_dist < 0.0 {
                self.result = RACE_OPPONENT_FIRST;
            }
        }
        if self.skaters[0].state == PS_SKATING {
            let won = self.result == RACE_PLAYER_FIRST;
            self.pose_result(ctx, 0, won);
            if won {
                ctx.actors.set_anims(&ctx.av, ACTOR_OPPONENT, a[A_OPP_LOSE], a[A_OPP_WAIT], false, STATE_RESTART_FORWARD);
                if self.challenge != 0 {
                    ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_NORMAL);
                }
            } else {
                ctx.actors.set_anims(&ctx.av, ACTOR_OPPONENT, a[A_OPP_VICTORY], a[A_OPP_VICTORY], false, STATE_RESTART_FORWARD);
            }
            self.reset_capn(ctx);
            ctx.actors.set_layer_mask(ACTOR_OPPONENT, LAYER_MAIN);
            if self.challenge == 0 {
                self.commentator = COMM_END_DRIVE_IN;
                let best_frac = ctx.player.run_best_frac as u16;
                let previous_best = time_text(ctx.player.run_best_secs, i32::from(best_frac));
                if self.seconds < ctx.player.run_best_secs
                    || (ctx.player.run_best_secs == self.seconds && f64::from(self.fraction) * 5.0 < f64::from(best_frac))
                {
                    ctx.player.run_best_secs = self.seconds;
                    ctx.player.run_best_frac = (ftol(f64::from(self.fraction)) * 5) as i16;
                }
                ctx.scene.set_text_by_id(4, &previous_best);
                ctx.scene.set_hover_by_id(3, HOVER_NORMAL);
                ctx.scene.set_hover_by_id(4, HOVER_NORMAL);
                ctx.scene.set_hover_by_id(5, HOVER_NORMAL);
                ctx.scene.set_hover_by_id(6, HOVER_NORMAL);
                ctx.cursor.set_visible(true);
            }
        }
        self.timer_resync = true;
    }

    // Decides a friend match at the line or a lead of a screen and poses both racers as they settle
    fn versus_race<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.winner.is_none() {
            let (p0, p1) = (self.skaters[0].pos, self.skaters[1].pos);
            if let Some(line) = self.line_pos {
                let crossed: Vec<usize> = (0..2).filter(|&i| self.skaters[i].pos >= line + CROSS_PAST).collect();
                self.winner = crossed.into_iter().max_by(|&x, &y| self.skaters[x].pos.total_cmp(&self.skaters[y].pos));
            }
            if self.winner.is_none() {
                self.winner = left_behind(p0 - p1);
            }
            if let Some(w) = self.winner {
                self.result = if w == 0 { RACE_PLAYER_FIRST } else { RACE_OPPONENT_FIRST };
                let name = if w == 0 { ctx.player.name_str() } else { ctx.session.guest_name() };
                ctx.scene.set_text_by_id(OBJ_GOOD_JOB, &wins_text(&name));
                ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_NORMAL);
                self.reset_capn(ctx);
            }
        }
        if let Some(w) = self.winner {
            for i in 0..2 {
                self.pose_result(ctx, i, i == w);
            }
            self.timer_resync = true;
        }
    }

    // True once every racer whose result the exit waits on has settled into its pose
    fn finished(&self) -> bool {
        if self.versus() {
            self.skaters.iter().all(|s| s.state == PS_FINISHED)
        } else {
            self.skaters[0].state == PS_FINISHED
        }
    }
}

impl<H: Host> Screen<H> for RunningScreen {
    // Loads Running.wam, the strips, actors, anims, sounds and obstacle slots for the variant
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.challenge = i32::from(ctx.player.challenge_active);
        let guest = ctx.session.guest().filter(|_| self.challenge != 0).cloned();
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "Running").await;
        ctx.scene.assign_id("berries", OBJ_BERRIES);
        ctx.scene.assign_id("time", OBJ_TIME);
        ctx.scene.assign_id("BestTitle", 3);
        ctx.scene.assign_id("BestTime", 4);
        ctx.scene.assign_id("done", 5);
        ctx.scene.assign_id("Again", 6);
        ctx.scene.set_text_by_id(OBJ_TIME, "0.00");
        ctx.scene.set_text_by_id(OBJ_BERRIES, "0");
        ctx.scene.set_text_by_id(4, "0.00");
        ctx.scene.set_hover_by_id(3, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(4, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(5, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(6, HOVER_HIDDEN);
        ctx.scene.assign_id("GoodJob", OBJ_GOOD_JOB);
        ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_HIDDEN);
        if self.challenge != 0 {
            ctx.scene.set_text_by_id(OBJ_GOOD_JOB, "You Win!");
        }
        let Some(mouse) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        ctx.cursor.set_anim(&ctx.av, mouse);
        ctx.cursor.set_pos(0x140, 400);
        ctx.cursor.set_visible(true);
        let sounds = [
            (SFX_LOOP, "skateloop.wav"),
            (SFX_CRASH, "crash.wav"),
            (SFX_JUMP, "skatejump.wav"),
            (SFX_BERRY, "cs_berry.wav"),
            (SFX_LAND, "skateland.wav"),
        ];
        for (index, name) in sounds {
            let Some(h) = ctx.av.get_sfx(&mut ctx.host, name, 0, false).await else { return false };
            self.sfx[index] = h;
        }
        let [sky, track, road, mid, front] = &mut self.strips;
        if self.challenge == 0 {
            sky.add_tile(&mut ctx.av, &mut ctx.host, "sk8bw8sky00.bam", 0, 0).await;
            sky.set_speed_factor(0.05);
            sky.set_y(0.0);
            for name in ["sk8bw7trak00.bam", "sk8bw7trak01.bam", "sk8bw7trak02.bam", "sk8bw7trak03.bam", "sk8bw7trak04.bam"] {
                track.add_tile(&mut ctx.av, &mut ctx.host, name, 0, 0).await;
            }
            track.set_speed_factor(0.2);
            track.set_y(0.0);
            road.add_tile(&mut ctx.av, &mut ctx.host, "sk8bw6road01.bam", 0, 3).await;
            road.add_tile(&mut ctx.av, &mut ctx.host, "sk8bw6road02.bam", 0, 0).await;
            road.set_speed_factor(1.0);
            road.set_y(240.0);
            let mid_tiles = [
                ("welcomesign.bam", 0, 0),
                ("sk8bw5gate00.bam", 0, 2),
                ("sk8bw5gate00.bam", 0, 2),
                ("sk8bw5gate00.bam", 0, 4),
                ("sk8bw5shak00.bam", 0, 0),
                ("sk8bw5shak01.bam", 0, 0),
                ("boothanim.bam", 0, 0),
                ("sk8bw5shak00.bam", 0x140, 0),
                ("sk8bw5shak01.bam", 0x140, 0),
                ("boothanim.bam", 0x140, 0),
                ("sk8bw5gate00.bam", 0, 6),
            ];
            for (name, width, repeat) in mid_tiles {
                mid.add_tile(&mut ctx.av, &mut ctx.host, name, width, repeat).await;
            }
            mid.set_random_start(1);
            mid.set_speed_factor(1.0);
            mid.set_y(260.0);
            front.add_tile(&mut ctx.av, &mut ctx.host, "sk8bw3lamp00.bam", 0x280, 0).await;
            front.add_tile(&mut ctx.av, &mut ctx.host, "sk8bw3bush00.bam", 0x140, 0).await;
            front.set_speed_factor(2.0);
            front.set_y(490.0);
            front.spawn(&mut ctx.rand, 2000.0);
        } else {
            sky.add_tile(&mut ctx.av, &mut ctx.host, "sk8ug9backdrop.bam", 0, 0).await;
            sky.set_speed_factor(0.05);
            sky.set_y(0.0);
            let track_tiles = [
                "sk8ug7baktrak00.bam",
                "sk8ug7baktrak01.bam",
                "sk8ug7baktrak02.bam",
                "sk8ug7baktrak03.bam",
                "sk8ug7baktrak04.bam",
            ];
            for name in track_tiles {
                track.add_tile(&mut ctx.av, &mut ctx.host, name, 0, 0).await;
            }
            track.set_speed_factor(0.2);
            track.set_y(0.0);
            road.add_tile(&mut ctx.av, &mut ctx.host, "sk8ug6road.bam", 0, 0).await;
            road.set_speed_factor(1.0);
            road.set_y(240.0);
            let mid_tiles = [
                ("sk8ug5dwelling.bam", 700),
                ("sk8ug5stalagtite.bam", 0x168),
                ("sk8ug5column.bam", 0x168),
                ("sk8ug5minecar.bam", 500),
                ("sk8ug5shack.bam", 300),
            ];
            for (name, width) in mid_tiles {
                mid.add_tile(&mut ctx.av, &mut ctx.host, name, width, 0).await;
            }
            mid.set_speed_factor(1.0);
            mid.set_y(260.0);
            front.add_tile(&mut ctx.av, &mut ctx.host, "sk8ug1column.bam", 0x3c0, 0).await;
            front.add_tile(&mut ctx.av, &mut ctx.host, "sk8ug1stalagtite.bam", 0x280, 0).await;
            front.set_speed_factor(2.0);
            front.set_y(0.0);
            front.spawn(&mut ctx.rand, 2000.0);
        }
        ctx.actors.clear();
        for id in [1, 4, 5, 6, 7, 8, 9, 10, 0xb, 3, 2, 0, 0xc] {
            ctx.actors.add_actor(id, true);
        }
        if self.challenge == 0 {
            self.opp_y = 280.0;
            self.ground_y = 280.0;
        } else {
            self.opp_y = 205.0;
            self.ground_y = 320.0;
        }
        self.skaters.clear();
        let resident = ctx.player.clone();
        let Some(mut player) = Self::load_skater(ctx, Skin::host(&resident), ACTOR_PLAYER, false).await else {
            return false;
        };
        player.ground_y = self.ground_y;
        self.skaters.push(player);
        if let Some(g) = &guest {
            let Some(mut rival) = Self::load_skater(ctx, Skin::guest(g), ACTOR_OPPONENT, true).await else {
                return false;
            };
            rival.ground_y = self.ground_y - VERSUS_RIVAL_DEPTH;
            self.skaters.push(rival);
        }
        let shared_anims = [
            (A_LINE, "sk8bw7line"),
            (A_BERRY, "berrybulge"),
            (A_BISCUIT, "biscuit"),
            (A_CAPN_028, "capn_cap028"),
            (A_CAPN_029, "capn_cap029"),
            (A_CAPN_013, "capn_cap013"),
            (A_CAPN_015, "capn_cap015"),
            (A_TV_004, "cap_tv_cap004"),
            (A_TV_005, "cap_tv_cap005"),
            (A_TV_006, "cap_tv_cap006"),
            (A_CAR, "backwardia_car"),
            (A_ONE, "one"),
            (A_TWO, "two"),
            (A_THREE, "three"),
            (A_GO, "go"),
        ];
        for (index, name) in shared_anims {
            let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
            self.anims[index] = h;
        }
        if self.challenge == 0 {
            let variant_anims = [
                (A_CART, "hot_dog_cart"),
                (A_BLOCK, "cone"),
                (A_BLOCK_ANIM, "coneanim"),
                (A_CAN, "canTrash"),
                (A_CAN_ANIM, "canTrashanim"),
                (A_PLATFORM, "cratesx4"),
                (A_BARRICADE, "barricade"),
                (A_BARRICADE_ANIM, "FlashFallAnim"),
                (A_OPP_RUN, "turtle_running"),
                (A_OPP_WAIT, "turtle_warmup"),
                (A_OPP_VICTORY, "turtle_victory"),
                (A_OPP_LOSE, "turtle_warmup"),
            ];
            for (index, name) in variant_anims {
                let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
                self.anims[index] = h;
            }
        } else {
            let mut variant_anims = vec![
                (A_BLOCK, "minecarblock"),
                (A_BLOCK_ANIM, "minecarfallanim"),
                (A_PLATFORM, "sk8ug3platform"),
            ];
            if guest.is_none() {
                variant_anims.extend([
                    (A_OPP_RUN, "thief_sk8_sk8_loop"),
                    (A_OPP_WAIT, "thief_sk8_sk8_wait"),
                    (A_OPP_LOSE, "thief_sk8_sk8_lose"),
                    (A_OPP_VICTORY, "thief_sk8_sk8_victory"),
                ]);
            }
            for (index, name) in variant_anims {
                let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
                self.anims[index] = h;
            }
        }
        let a = self.anims;
        self.opp_max_speed = 14.0;
        self.opp_accel = 0.7;
        self.opp_decel = 1.2;
        self.opp_speed = 0.0;
        self.loop_channel = INVALID_HANDLE;
        self.slots.clear();
        self.add_obstacle_slot(a[A_BERRY], a[A_BERRY], 6, KIND_BERRY);
        self.add_obstacle_slot(a[A_BISCUIT], a[A_BISCUIT], 10, KIND_BERRY);
        if self.challenge == 0 {
            if ctx.player.stage == 0 {
                self.add_obstacle_slot(a[A_BLOCK], a[A_BLOCK_ANIM], 4, KIND_SLOWDOWN);
            } else {
                self.add_obstacle_slot(a[A_CART], a[A_CART], 4, KIND_PLATFORM);
            }
            self.add_obstacle_slot(a[A_PLATFORM], a[A_PLATFORM], 5, KIND_PLATFORM);
            self.add_obstacle_slot(a[A_CAN], a[A_CAN_ANIM], 7, KIND_CRASH);
            self.add_obstacle_slot(a[A_PLATFORM], a[A_PLATFORM], 8, KIND_PLATFORM);
            self.add_obstacle_slot(a[A_BARRICADE], a[A_BARRICADE_ANIM], 9, KIND_CRASH);
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "tg52Skating.wav", true, 0).await;
        } else {
            self.add_obstacle_slot(a[A_BLOCK], a[A_BLOCK_ANIM], 4, KIND_CRASH);
            self.add_obstacle_slot(a[A_PLATFORM], a[A_PLATFORM], 5, KIND_PLATFORM);
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "ps57Challenge.wav", true, 0).await;
            self.opp_max_speed = 17.0;
        }
        self.add_obstacle_slot(a[A_LINE], a[A_LINE], 0xb, KIND_FINISH);
        self.finish_dist = 22000.0;
        self.half_dist = self.finish_dist / 2.0;
        self.spawn_dist = 700.0;
        self.cross_dist = self.finish_dist + 620.0;
        self.opp_finish_dist = self.cross_dist + 150.0;
        self.line_pos = None;
        self.winner = None;
        self.result = RACE_NONE;
        self.seconds = 0;
        self.fraction = 0.0;
        self.timer_resync = true;
        self.countdown = 0x83;
        ctx.cursor.set_visible(false);
        self.intro_pending = self.challenge == 0;
        if 0 < ctx.player.running {
            self.intro_pending = false;
        }
        self.commentator = COMM_DRIVE_IN;
        self.voice = 0;
        self.finish_frames = 0;
        true
    }

    // Scrolls, reads input, runs the countdown, race, commentator, collisions, spawning and drawing
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let a = self.anims;
        let s = self.sfx;
        let mut result = RESULT_CONTINUE;
        let cam = self.cam_speed();
        for strip in &mut self.strips {
            strip.scroll(&ctx.av, cam);
        }
        for strip in &mut self.strips[..4] {
            strip.fill_and_draw(&mut ctx.av, &mut ctx.rand);
        }
        if ctx.cursor.visible {
            let (dx, dy) = ctx.av.get_mouse_delta();
            ctx.cursor.move_by(dx, dy);
            let (mx, my) = (ctx.cursor.x, ctx.cursor.y);
            if let Some(id) = ctx.scene.hit_test(i32::from(mx), i32::from(my))
                && ctx.av.is_mouse_button_down(MOUSE_LEFT)
            {
                result = self.on_mouse_hit(ctx, id);
            }
        }
        ctx.actors.update(&ctx.av);
        while let Some(ev) = ctx.actors.pop_event() {
            self.on_actor_event(ctx, ev.kind, ev.actor_id).await;
        }
        self.place_skaters(ctx);
        if self.commentator == COMM_DRIVE_OUT {
            ctx.actors.add_x(ACTOR_CAPN, -cam);
            ctx.actors.add_x(ACTOR_CAR, -cam);
        }
        for slot in &self.slots {
            if slot.active {
                ctx.actors.add_x(slot.actor_id, -cam);
            }
        }
        if self.countdown < 0x28 {
            if self.countdown != 0 {
                self.countdown -= 1;
                if self.countdown == 1 {
                    ctx.actors.set_visible(ACTOR_SIGN, false);
                }
            }
            if self.versus() {
                self.versus_race(ctx);
            } else {
                self.update_opponent_speed();
                self.solo_finish(ctx);
            }
            for i in 0..self.skaters.len() {
                let held = ctx.held(self.skaters[i].friend);
                self.skater_input(ctx, i, held).await;
            }
            let now = ctx.time_ms();
            let mut dt = now.wrapping_sub(self.last_tick_ms) as f32;
            if self.timer_resync {
                dt = 0.0;
                self.timer_resync = false;
            }
            self.last_tick_ms = now;
            self.fraction = (f64::from(dt) / 50.0 + f64::from(self.fraction)) as f32;
            while 20.0 <= self.fraction {
                self.fraction -= 20.0;
                self.seconds = self.seconds.wrapping_add(1);
            }
            let hundredths = ftol(f64::from(self.fraction)) * 5;
            ctx.scene.set_text_by_id(OBJ_TIME, &time_text(self.seconds, hundredths));
        } else if !self.intro_pending {
            if self.challenge == 0 || self.challenge == 3 {
                self.countdown -= 1;
            } else if self.challenge == 1 {
                ctx.actors.set_visible(ACTOR_CAPN, true);
                ctx.actors.set_pos(ACTOR_CAPN, 512.0, 4.0);
                ctx.actors.set_screen_fixed(ACTOR_CAPN, true);
                ctx.actors.set_draw_offset(ACTOR_CAPN, 0, 0);
                ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_006], a[A_TV_006], false, STATE_RESTART_FORWARD);
                ctx.actors.set_layer_mask(ACTOR_CAPN, LAYER_TOP);
                self.commentator = COMM_TV_IDLE;
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, "049.wav", false, self.voice).await;
                self.challenge += 1;
            } else if ctx.stream_done(self.voice) {
                self.challenge += 1;
            }
            if self.countdown == 0x28 {
                ctx.actors.set_anims(&ctx.av, ACTOR_SIGN, a[A_GO], a[A_GO], false, STATE_STOPPED);
                for skater in &self.skaters {
                    let skating = skater.anims[SK_SKATING];
                    ctx.actors.set_anims(&ctx.av, skater.actor, skating, skating, false, STATE_RESTART_FORWARD);
                }
                if !self.versus() {
                    ctx.actors.set_anims(&ctx.av, ACTOR_OPPONENT, a[A_OPP_RUN], a[A_OPP_RUN], false, STATE_RESTART_FORWARD);
                }
                if self.challenge == 0 {
                    ctx.actors.set_anims_delayed(
                        &ctx.av,
                        ACTOR_CAPN,
                        0x19,
                        a[A_CAPN_028],
                        a[A_CAPN_028],
                        false,
                        STATE_RESTART_FORWARD,
                    );
                } else {
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_005], a[A_TV_005], false, STATE_RESTART_FORWARD);
                }
            } else if self.countdown == 0x46 {
                ctx.actors.set_anims(&ctx.av, ACTOR_SIGN, a[A_ONE], a[A_ONE], false, STATE_STOPPED);
            } else if self.countdown == 100 {
                ctx.actors.set_anims(&ctx.av, ACTOR_SIGN, a[A_TWO], a[A_TWO], false, STATE_STOPPED);
            } else if self.countdown == 0x82 {
                ctx.actors.set_anims(&ctx.av, ACTOR_SIGN, a[A_THREE], a[A_THREE], false, STATE_STOPPED);
                ctx.actors.set_visible(ACTOR_SIGN, true);
                ctx.actors.set_layer_mask(ACTOR_SIGN, LAYER_TOP);
                ctx.actors.set_pos(ACTOR_SIGN, 320.0, 180.0);
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, "032.wav", false, self.voice).await;
            }
        }
        match self.commentator {
            COMM_DRIVE_IN => {
                let (x, _) = ctx.actors.pos(ACTOR_CAPN);
                if 400.0 <= x || ctx.av.is_key_down(KEY_SPACE) {
                    ctx.actors.set_pos(ACTOR_CAPN, 400.0, 290.0);
                    ctx.actors.set_pos(ACTOR_CAR, 250.0, 350.0);
                    ctx.actors.set_anims_delayed(
                        &ctx.av,
                        ACTOR_CAPN,
                        0xf,
                        a[A_CAPN_029],
                        a[A_CAPN_029],
                        false,
                        STATE_RESTART_FORWARD,
                    );
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAR, a[A_CAR], a[A_CAR], false, STATE_STOPPED);
                    self.voice = ctx.av.play_stream_sound(&mut ctx.host, "031.wav", false, self.voice).await;
                    self.commentator = COMM_INTRO_SPEECH;
                } else {
                    if 4.0 < self.car_speed {
                        self.car_speed -= 1.0;
                    }
                    ctx.actors.add_x(ACTOR_CAPN, self.car_speed);
                    ctx.actors.add_x(ACTOR_CAR, self.car_speed);
                }
            }
            COMM_DRIVE_OUT => {
                let (x, _) = ctx.actors.pos(ACTOR_CAPN);
                if x < -300.0 {
                    ctx.actors.set_pos(ACTOR_CAPN, 512.0, 4.0);
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_005], a[A_TV_005], false, STATE_RESTART_FORWARD);
                    ctx.actors.set_layer_mask(ACTOR_CAPN, LAYER_TOP);
                    ctx.actors.set_visible(ACTOR_CAPN, true);
                    self.commentator = COMM_TV_IDLE;
                }
            }
            COMM_INTRO_SPEECH if ctx.stream_done(self.voice) || ctx.av.is_key_down(KEY_SPACE) => {
                ctx.av.kill_streaming_channel(self.voice);
                self.commentator = COMM_DRIVE_OUT;
                self.intro_pending = false;
            }
            COMM_TV_SPEECH if ctx.stream_done(self.voice) => {
                ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_005], a[A_TV_005], false, STATE_RESTART_FORWARD);
                self.commentator = COMM_TV_IDLE;
            }
            COMM_END_DRIVE_IN => {
                let (x, _) = ctx.actors.pos(ACTOR_CAPN);
                if 400.0 <= x {
                    ctx.actors.set_pos(ACTOR_CAPN, 400.0, 290.0);
                    ctx.actors.set_pos(ACTOR_CAR, 250.0, 350.0);
                    ctx.actors.set_anims_delayed(
                        &ctx.av,
                        ACTOR_CAPN,
                        0xf,
                        a[A_CAPN_029],
                        a[A_CAPN_029],
                        false,
                        STATE_RESTART_FORWARD,
                    );
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAR, a[A_CAR], a[A_CAR], false, STATE_STOPPED);
                    self.commentator = COMM_END_SPEECH;
                    if self.challenge == 0 {
                        if self.result == RACE_PLAYER_FIRST {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "033.wav", false, self.voice).await;
                        } else {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "034.wav", false, self.voice).await;
                        }
                    }
                } else {
                    if 4.0 < self.car_speed {
                        self.car_speed -= 1.0;
                    }
                    ctx.actors.add_x(ACTOR_CAPN, self.car_speed);
                    ctx.actors.add_x(ACTOR_CAR, self.car_speed);
                }
            }
            COMM_END_SPEECH if ctx.stream_done(self.voice) => {
                self.commentator = COMM_STANDING;
                ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_CAPN_028], a[A_CAPN_028], false, STATE_RESTART_FORWARD);
            }
            _ => {}
        }
        for i in 0..self.skaters.len() {
            self.skater_collide(ctx, i);
        }
        let rolling = self.skaters.iter().any(|k| k.speed > 1.0 && (k.state == PS_SKATING || k.state == PS_RIDING));
        if !rolling {
            if self.loop_channel != INVALID_HANDLE {
                ctx.av.stop_sfx(self.loop_channel);
                self.loop_channel = INVALID_HANDLE;
            }
        } else if self.loop_channel == INVALID_HANDLE {
            self.loop_channel = ctx.av.play_sfx(s[SFX_LOOP], 1);
        }
        for slot in &mut self.slots {
            if slot.active && ctx.actors.bbox(&ctx.av, slot.actor_id)[2] < 0 {
                ctx.actors.set_visible(slot.actor_id, false);
                slot.active = false;
            }
        }
        if 0.0 <= self.spawn_dist || self.finish_dist <= 1000.0 {
            self.spawn_dist -= cam;
        } else {
            let i = ctx.rand_range(0, self.slots.len() as i32 - 1) as usize;
            let slot = self.slots[i];
            if !slot.active {
                let y = self.ground_y + 85.0;
                let ahead = if self.versus() { VERSUS_SPAWN_X } else { 800.0 };
                let x = ctx.rand_range(0, 150) as f32 + ahead;
                ctx.actors.set_pos(slot.actor_id, x, y);
                if slot.kind == KIND_BERRY || slot.idle != slot.alt {
                    ctx.actors.set_anims(&ctx.av, slot.actor_id, slot.idle, slot.idle, false, STATE_RESTART_FORWARD);
                } else {
                    ctx.actors.set_anims(&ctx.av, slot.actor_id, slot.idle, slot.idle, false, STATE_STOPPED);
                }
                if slot.kind == KIND_BERRY {
                    ctx.actors.set_y(slot.actor_id, self.ground_y + 40.0);
                }
                ctx.actors.set_visible(slot.actor_id, true);
                ctx.actors.set_layer_mask(slot.actor_id, LAYER_MAIN);
                self.slots[i].hit = false;
                self.slots[i].active = true;
                self.spawn_dist = 700.0;
            }
        }
        if self.finish_dist < 1000.0 {
            self.strips[4].set_stopped(true);
        }
        if 0.0 <= self.finish_dist {
            self.finish_dist -= cam;
            if self.finish_dist < 0.0 {
                let last = self.slots.len() - 1;
                let line = self.slots[last];
                ctx.actors.set_pos(line.actor_id, LINE_SCREEN_X, 367.0);
                ctx.actors.set_anims(&ctx.av, line.actor_id, line.idle, line.idle, false, STATE_RESTART_FORWARD);
                ctx.actors.set_visible(line.actor_id, true);
                self.slots[last].active = true;
                self.finish_dist = -1.0;
                if self.versus() {
                    let cam_pos = (self.skaters[0].pos + self.skaters[1].pos) / 2.0;
                    self.line_pos = Some(cam_pos + LINE_SCREEN_X - VERSUS_CENTER);
                }
                for skater in &self.skaters {
                    ctx.actors.set_layer_mask(skater.actor, LAYER_FRONT);
                }
                ctx.actors.set_layer_mask(ACTOR_OPPONENT, LAYER_FRONT);
            }
        }
        if self.half_dist != 0.0 && self.finish_dist < self.half_dist {
            self.half_dist = 0.0;
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_006], a[A_TV_006], false, STATE_RESTART_FORWARD);
            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "058.wav", false, self.voice).await;
            self.commentator = COMM_TV_SPEECH;
        }
        for skater in &self.skaters {
            if skater.state == PS_RIDING {
                ctx.actors.add_y(skater.actor, skater.ride_shift);
            }
        }
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_MAIN);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_FRONT);
        for skater in &self.skaters {
            if skater.state == PS_RIDING {
                ctx.actors.add_y(skater.actor, -skater.ride_shift);
            }
        }
        if ctx.av.is_key_down(KEY_ESC) {
            result = STATE_HOME_WORLD;
        }
        if self.challenge != 0 && self.finished() {
            self.finish_frames += 1;
            if 0x78 < self.finish_frames {
                ctx.player.challenge_won = u8::from(self.result == RACE_PLAYER_FIRST);
                result = if self.versus() { STATE_HOME_WORLD } else { STATE_CHALLENGE };
            }
        }
        self.strips[4].fill_and_draw(&mut ctx.av, &mut ctx.rand);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_TOP);
        ctx.scene.draw_layer(&mut ctx.av, LAYER_ALL);
        ctx.cursor.draw(&mut ctx.av);
        ctx.scene.advance_anims(&ctx.av);
        result
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
    fn opponent_ramps_to_cap_then_brakes_to_zero() {
        let mut g = RunningScreen::new();
        g.opp_finish_dist = 20.0;
        g.opp_max_speed = 14.0;
        g.opp_accel = 0.7;
        g.opp_decel = 1.2;
        g.opp_speed = 13.8;
        g.update_opponent_speed();
        assert_eq!(g.opp_speed, 14.0);
        g.update_opponent_speed();
        assert_eq!(g.opp_speed, (14.0f64 - f64::from(1.2f32) * f64::from(0.4f32)) as f32);
        g.opp_speed = 0.1;
        g.update_opponent_speed();
        assert_eq!(g.opp_speed, 0.0);
    }

    #[test]
    fn obstacle_slots_keep_insertion_order() {
        let mut g = RunningScreen::new();
        g.add_obstacle_slot(7, 8, 6, KIND_BERRY);
        g.add_obstacle_slot(9, 9, 0xb, KIND_FINISH);
        assert_eq!(g.slots.len(), 2);
        assert_eq!(g.slots[0].actor_id, 6);
        assert_eq!(
            g.slots[1],
            ObstacleSlot { actor_id: 0xb, kind: KIND_FINISH, idle: 9, alt: 9, hit: false, active: false }
        );
    }

    #[test]
    fn time_text_pads_hundredths() {
        assert_eq!(time_text(3, 5), " 3.05");
        assert_eq!(time_text(100, 0), " 100.00");
        assert_eq!(time_text(12, 95), " 12.95");
    }

    #[test]
    fn friend_match_camera_splits_the_lead() {
        assert_eq!(versus_screen_x(0.0, 0.0), VERSUS_CENTER);
        assert_eq!(versus_screen_x(300.0, 100.0), VERSUS_CENTER + 100.0);
        assert_eq!(versus_screen_x(100.0, 300.0), VERSUS_CENTER - 100.0);
        assert_eq!(left_behind(VERSUS_GAP_LIMIT), None);
        assert_eq!(left_behind(VERSUS_GAP_LIMIT + 1.0), Some(0));
        assert_eq!(left_behind(-VERSUS_GAP_LIMIT - 1.0), Some(1));
    }

    #[test]
    fn top_speed_follows_the_running_stat() {
        assert_eq!(Skater::top_speed(0), 14.0);
        assert_eq!(Skater::top_speed(20), (20.0f64 * f64::from(0.3f32) + 14.0) as f32);
        let cam = RunningScreen { skaters: vec![Skater { speed: 10.0, ..Skater::default() }], ..RunningScreen::default() };
        assert_eq!(cam.cam_speed(), 10.0);
        let both = RunningScreen {
            skaters: vec![Skater { speed: 10.0, ..Skater::default() }, Skater { speed: 4.0, ..Skater::default() }],
            ..RunningScreen::default()
        };
        assert!(both.versus());
        assert_eq!(both.cam_speed(), 7.0);
    }
}
