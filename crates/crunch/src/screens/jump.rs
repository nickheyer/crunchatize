use avmanager::anim::{STATE_RESTART_FORWARD, STATE_STOPPED};
use avmanager::crt::ftol;

use crate::actors::{
    ACTOR_FLAG_FINISH, ACTOR_FLAG_PICKUP, EVENT_ANIM_END, EVENT_FINISH, EVENT_GROUND_REACHED, EVENT_INIT_OBJECT,
    EVENT_PICKUP, EVENT_UNATTACHED, FACING_LEFT, FACING_NONE, FACING_RIGHT,
};
use crate::ctx::{
    Ctx, Held, KEY_ESC, KEY_SPACE, RESULT_CONTINUE, STATE_CHALLENGE, STATE_HOME_WORLD, STATE_JUMPING, Screen,
};
use crate::host::Host;
use crate::online::wins_text;
use crate::player::{PlayerData, STAT_JUMPING};
use crate::scene::{FLAG_FINISH, FLAG_PICKUP, FLAG_PLATFORM, HOVER_HIDDEN, HOVER_NORMAL};
use crate::scroll_strip::ScrollStrip;
use crate::skin::Skin;

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

// Handle table of one jumper's own crunchling animations
const J_STANCE: usize = 0;
const J_RUN_LEFT: usize = 1;
const J_RUN_RIGHT: usize = 2;
const J_JUMP_LEFT: usize = 3;
const J_JUMP_RIGHT: usize = 4;
const J_TURN_LEFT: usize = 5;
const J_TURN_RIGHT: usize = 6;
const J_FALLING: usize = 7;
const J_COUNT: usize = 8;

const SFX_JUMP: usize = 0;
const SFX_BERRY: usize = 1;
const SFX_LAND: usize = 2;

// Actor 0 is the thief in his arena and the visiting friend's crunchling in a friend climb
const ACTOR_RIVAL: i32 = 0;
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
const PLAYER_START_X: f32 = 280.0;
const GUEST_START_X: f32 = 360.0;
const LEFT_EDGE: f32 = 30.0;
const RIGHT_EDGE: f32 = 560.0;

// Height gap that leaves one friend out of frame and hands the climb to the higher one
const VERSUS_GAP_LIMIT: f32 = 380.0;

// One climbing crunchling, the resident one or the visiting friend's, with its anims and run state
#[derive(Clone, Copy, Debug, Default)]
struct Jumper {
    actor: i32,
    friend: bool,
    anims: [u16; J_COUNT],
    state: i32,
    vx: f32,
    max_run_speed: f32,
    jump_power: f32,
    drop: f32,
    start_x: f32,
    fall_frames: u32,
    berries: u32,
    berry_streak: u32,
    finished: bool,
}

impl Jumper {
    // Top running speed of a form, six for babies, eight for teens and ten for adults
    fn run_speed(stage: u16) -> f32 {
        match stage {
            0 => 6.0,
            1 => 8.0,
            _ => 10.0,
        }
    }

    // Upward launch speed from the jumping stat
    fn launch(jumping: i32) -> f32 {
        (-(jumping as f64 * 0.5 + 15.0)) as f32
    }

    fn racing(&self) -> bool {
        matches!(self.state, PLAYER_STANDING | PLAYER_RUNNING | PLAYER_AIRBORNE)
    }
}

// Jumping mini game, state 0xc, with its tall scrolling level, the thief race and the friend climb
#[derive(Clone, Debug, Default)]
pub struct JumpScreen {
    cursor_anim: u16,
    anims: [u16; 22],
    sfx: [u16; 3],
    jumpers: Vec<Jumper>,
    backdrop: ScrollStrip,
    music: u32,
    voice: u32,
    max_scroll_y: i32,
    capn_state: i32,
    thief_state: i32,
    ground_y: f32,
    last_time: u32,
    time_frac: f32,
    time_secs: u16,
    timer_reset: bool,
    countdown: u32,
    thief_y: f32,
    result: i32,
    winner: Option<usize>,
    intro_pending: bool,
    outro_active: bool,
    challenge: i32,
    capn_fly_state: i32,
    fly_y: f32,
    base_y: f32,
    end_delay: u32,
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

// Who a height gap too tall for one screen hands the climb to, None while both are in frame
fn left_behind(y0: f32, y1: f32) -> Option<usize> {
    if y1 - y0 > VERSUS_GAP_LIMIT {
        Some(0)
    } else if y0 - y1 > VERSUS_GAP_LIMIT {
        Some(1)
    } else {
        None
    }
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

// Stores a climb time as a crunchling's best when it beats the one on record
fn record_best(player: &mut PlayerData, secs: u16, frac: f32) {
    if secs < player.jump_best_secs
        || (player.jump_best_secs == secs && frac as f64 * 5.0 < (player.jump_best_frac as u16) as f64)
    {
        player.jump_best_secs = secs;
        player.jump_best_frac = (ftol(frac as f64) * 5) as i16;
    }
}

// Level file for a form, the friend climb taking the lower form of the two so both can finish it
fn level_name(stage: u16) -> &'static str {
    match stage {
        1 => "jumpcomlev02",
        2 => "jumpcomlev03",
        _ => "jumpcomlev01",
    }
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

    // True while the resident crunchling climbs against the visiting friend's rather than the thief
    fn versus(&self) -> bool {
        self.jumpers.len() == 2
    }

    fn jumper_index(&self, actor: i32) -> Option<usize> {
        self.jumpers.iter().position(|j| j.actor == actor)
    }

    // Loads a jumper's eight crunchling animations through its own recoloured palette
    async fn load_jumper<H: Host>(ctx: &mut Ctx<H>, skin: Skin<'_>, actor: i32, friend: bool, start_x: f32) -> Option<Jumper> {
        let (stance, pal) = skin.palette(&mut ctx.av, &mut ctx.host, "stance", false, false).await?;
        let mut jumper = Jumper { actor, friend, start_x, ..Jumper::default() };
        jumper.anims[J_STANCE] = stance;
        let suffixes = [
            (J_RUN_LEFT, "running_left"),
            (J_RUN_RIGHT, "running_right"),
            (J_JUMP_LEFT, "jumping_left"),
            (J_JUMP_RIGHT, "jumping_right"),
            (J_TURN_LEFT, "jump_turn_left"),
            (J_TURN_RIGHT, "jump_turn_right"),
            (J_FALLING, "falling_loop"),
        ];
        for (slot, suffix) in suffixes {
            jumper.anims[slot] = skin.anim(&mut ctx.av, &mut ctx.host, suffix, false, false, &pal).await?;
        }
        jumper.max_run_speed = Jumper::run_speed(skin.data.shown_stage());
        jumper.jump_power = Jumper::launch(skin.data.jumping);
        jumper.drop = drop_height(skin.data.shown_stage());
        Some(jumper)
    }

    // Loads every shared animation
    async fn load_anims<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
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

    // Pays every climber's jumping training, done leaving for home and Again restarting the climb
    fn on_mouse_hit<H: Host>(&mut self, ctx: &mut Ctx<H>, id: i32) -> u32 {
        let going_home = match id {
            OBJ_DONE => true,
            OBJ_AGAIN => false,
            _ => return RESULT_CONTINUE,
        };
        for j in &self.jumpers {
            let bonus = BONUS_BERRIES < j.berries;
            match (j.friend, ctx.session.guest_mut()) {
                (true, Some(guest)) => guest.train(STAT_JUMPING, bonus, going_home),
                (true, None) => {}
                (false, _) => ctx.player.train(STAT_JUMPING, bonus, going_home),
            }
        }
        if going_home { STATE_HOME_WORLD } else { STATE_JUMPING }
    }

    // Dispatches one actor event to landing, falling, berry, finish and init handling
    async fn on_actor_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, actor: i32) {
        match kind {
            EVENT_INIT_OBJECT => self.on_init_object(ctx, actor),
            EVENT_ANIM_END => self.on_anim_end(ctx, actor),
            EVENT_GROUND_REACHED => {
                if let Some(i) = self.jumper_index(actor)
                    && self.jumpers[i].state == PLAYER_AIRBORNE
                {
                    let j = &mut self.jumpers[i];
                    let stance = j.anims[J_STANCE];
                    ctx.actors.set_anims(&ctx.av, actor, stance, stance, false, STATE_RESTART_FORWARD);
                    j.state = PLAYER_STANDING;
                    ctx.av.play_sfx(self.sfx[SFX_LAND], -1);
                }
            }
            EVENT_UNATTACHED => {
                if let Some(i) = self.jumper_index(actor) {
                    let j = &mut self.jumpers[i];
                    let falling = j.anims[J_FALLING];
                    ctx.actors.set_anims(&ctx.av, actor, falling, falling, false, STATE_RESTART_FORWARD);
                    let ground = ftol(self.ground_y as f64);
                    ctx.actors.set_gravity(actor, 0.0, JUMP_GRAVITY, ground, MAX_FALL_SPEED);
                    j.state = PLAYER_AIRBORNE;
                }
            }
            EVENT_PICKUP => {
                if let Some(i) = self.jumper_index(actor) {
                    self.jumpers[i].berries += 1;
                    self.jumpers[i].berry_streak += 1;
                    if PRAISE_STREAK < self.jumpers[i].berry_streak {
                        self.jumpers[i].berry_streak = 0;
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
            }
            EVENT_FINISH => {
                if let Some(i) = self.jumper_index(actor) {
                    self.jumpers[i].finished = true;
                }
            }
            _ => {}
        }
    }

    // Stands a jumper on its mark, or drops it in from above when no intro flight precedes the climb
    fn init_jumper<H: Host>(&mut self, ctx: &mut Ctx<H>, i: usize) {
        let j = &mut self.jumpers[i];
        let actor = j.actor;
        j.state = PLAYER_STANDING;
        let stance = j.anims[J_STANCE];
        ctx.actors.set_anims(&ctx.av, actor, stance, stance, false, STATE_RESTART_FORWARD);
        ctx.actors.set_turn_anims(actor, j.anims[J_TURN_RIGHT], j.anims[J_TURN_LEFT]);
        ctx.actors.set_pos(actor, j.start_x, self.ground_y);
        ctx.actors.face(&ctx.av, actor, FACING_NONE);
        ctx.actors.set_bbox_anim(actor, stance);
        if !self.intro_pending {
            j.state = PLAYER_AIRBORNE;
            ctx.actors.set_visible(actor, true);
            ctx.actors.set_pos(actor, j.start_x, j.drop + self.ground_y);
            let falling = j.anims[J_FALLING];
            ctx.actors.set_anims(&ctx.av, actor, falling, falling, false, STATE_RESTART_FORWARD);
            let ground = ftol(self.ground_y as f64);
            ctx.actors.set_gravity(actor, 1.0, 1.0, ground, MAX_FALL_SPEED);
        } else {
            ctx.actors.set_visible(actor, false);
        }
    }

    // Places the thief or guest, player, Cap'n, TV and countdown digits on their first frame
    fn on_init_object<H: Host>(&mut self, ctx: &mut Ctx<H>, actor: i32) {
        if let Some(i) = self.jumper_index(actor) {
            self.init_jumper(ctx, i);
            return;
        }
        match actor {
            ACTOR_RIVAL => {
                self.thief_state = THIEF_STOPPED;
                let climb = self.anims[ANIM_THIEF_CLIMB_A];
                ctx.actors.set_anims(&ctx.av, actor, climb, climb, false, STATE_STOPPED);
                ctx.actors.set_pos(actor, 312.0, self.thief_y);
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

    // Settles the resident climber's finish against the thief, with times and buttons outside a challenge
    fn solo_finish<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if !self.jumpers[0].finished {
            return;
        }
        if self.result == RESULT_UNDECIDED {
            self.result = RESULT_PLAYER_FIRST;
            if self.thief_y <= 100.0 {
                self.result = RESULT_THIEF_FIRST;
            }
        }
        if self.jumpers[0].state == PLAYER_STANDING {
            self.jumpers[0].state = PLAYER_FINISHED;
            if ctx.player.challenge_active == 0 {
                ctx.actors.set_visible(ACTOR_CAPN, false);
                self.capn_fly_state = FLY_START;
                self.outro_active = true;
                let old_best = time_text(ctx.player.jump_best_secs, ctx.player.jump_best_frac as u16 as i32);
                record_best(&mut ctx.player, self.time_secs, self.time_frac);
                ctx.scene.set_text_by_id(OBJ_BEST_TIME, &old_best);
                ctx.scene.set_hover_by_id(OBJ_BEST_TITLE, HOVER_NORMAL);
                ctx.scene.set_hover_by_id(OBJ_BEST_TIME, HOVER_NORMAL);
                ctx.scene.set_hover_by_id(OBJ_DONE, HOVER_NORMAL);
                ctx.scene.set_hover_by_id(OBJ_AGAIN, HOVER_NORMAL);
                self.jumpers[0].vx = 0.0;
                ctx.cursor.set_visible(true);
            } else if self.result == RESULT_PLAYER_FIRST {
                ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_NORMAL);
            }
        }
        self.timer_reset = true;
    }

    // Decides a friend climb at the top platform or a lead of a screen and holds both once they land
    fn versus_finish<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.winner.is_none() {
            let y0 = ctx.actors.pos(self.jumpers[0].actor).1;
            let y1 = ctx.actors.pos(self.jumpers[1].actor).1;
            let heights = [y0, y1];
            let done: Vec<usize> = (0..2).filter(|&i| self.jumpers[i].finished).collect();
            self.winner = done.into_iter().min_by(|&x, &y| heights[x].total_cmp(&heights[y]));
            if self.winner.is_none() {
                self.winner = left_behind(y0, y1);
            }
            if let Some(w) = self.winner {
                self.result = if w == 0 { RESULT_PLAYER_FIRST } else { RESULT_THIEF_FIRST };
                let name = if w == 0 { ctx.player.name_str() } else { ctx.session.guest_name() };
                ctx.scene.set_text_by_id(OBJ_GOOD_JOB, &wins_text(&name));
                ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_NORMAL);
                if self.challenge == 0 {
                    if self.jumpers[w].finished {
                        match (w, ctx.session.guest_mut()) {
                            (1, Some(guest)) => record_best(guest, self.time_secs, self.time_frac),
                            (1, None) => {}
                            _ => record_best(&mut ctx.player, self.time_secs, self.time_frac),
                        }
                    }
                    ctx.scene.set_hover_by_id(OBJ_DONE, HOVER_NORMAL);
                    ctx.scene.set_hover_by_id(OBJ_AGAIN, HOVER_NORMAL);
                    ctx.cursor.set_visible(true);
                }
            }
        }
        if self.winner.is_some() {
            for j in &mut self.jumpers {
                if j.state == PLAYER_STANDING {
                    j.state = PLAYER_FINISHED;
                    j.vx = 0.0;
                }
            }
            self.timer_reset = true;
        }
    }

    // Jumps, runs and steers one climber by the keys its side holds, and worries the Cap'n on long falls
    fn jumper_input<H: Host>(&mut self, ctx: &mut Ctx<H>, i: usize, held: Held) {
        let decided = self.winner.is_some();
        let held = if decided && self.jumpers[i].racing() { Held::default() } else { held };
        let j = &mut self.jumpers[i];
        let actor = j.actor;
        if (j.state == PLAYER_STANDING || j.state == PLAYER_RUNNING) && held.space {
            ctx.av.play_sfx(self.sfx[SFX_JUMP], -1);
            ctx.actors.set_anims(&ctx.av, actor, j.anims[J_JUMP_RIGHT], j.anims[J_JUMP_LEFT], false, STATE_RESTART_FORWARD);
            let ground = ftol(self.ground_y as f64);
            ctx.actors.set_gravity(actor, j.jump_power, JUMP_GRAVITY, ground, MAX_FALL_SPEED);
            j.state = PLAYER_AIRBORNE;
        }
        if j.racing() {
            if held.right {
                ctx.actors.face(&ctx.av, actor, FACING_RIGHT);
                if j.vx < 0.0 && j.state != PLAYER_AIRBORNE {
                    j.vx = 0.0;
                }
                j.vx += 1.0;
                if j.max_run_speed < j.vx {
                    j.vx = j.max_run_speed;
                }
            } else if held.left {
                ctx.actors.face(&ctx.av, actor, FACING_LEFT);
                if 0.0 < j.vx && j.state != PLAYER_AIRBORNE {
                    j.vx = 0.0;
                }
                j.vx -= 1.0;
                if j.vx < -j.max_run_speed {
                    j.vx = -j.max_run_speed;
                }
            } else if j.state == PLAYER_STANDING || j.state == PLAYER_RUNNING {
                if j.state != PLAYER_STANDING {
                    let stance = j.anims[J_STANCE];
                    ctx.actors.set_anims(&ctx.av, actor, stance, stance, false, STATE_RESTART_FORWARD);
                    ctx.actors.face(&ctx.av, actor, FACING_NONE);
                    j.state = PLAYER_STANDING;
                }
                j.vx = 0.0;
            }
            if j.vx != 0.0 && j.state == PLAYER_STANDING {
                ctx.actors.set_anims(&ctx.av, actor, j.anims[J_RUN_RIGHT], j.anims[J_RUN_LEFT], false, STATE_RESTART_FORWARD);
                j.state = PLAYER_RUNNING;
            }
            let vy = ctx.actors.vel_y(actor);
            if vy < MAX_FALL_SPEED {
                j.fall_frames = 0;
            } else {
                j.fall_frames += 1;
                if FALL_WARNING_FRAMES < j.fall_frames && self.capn_state != CAPN_TALKING && self.capn_state != CAPN_WORRIED {
                    let worried = self.anims[ANIM_CAP_TV_WORRIED];
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, worried, worried, true, STATE_RESTART_FORWARD);
                    self.capn_state = CAPN_WORRIED;
                }
            }
        }
        if j.vx != 0.0 && !j.finished {
            let (mut x, _y) = ctx.actors.pos(actor);
            x += j.vx;
            x = x.clamp(LEFT_EDGE, RIGHT_EDGE);
            ctx.actors.set_x(actor, x);
        }
    }

    // Finish bookkeeping, thief climb, both climbers' input and the clock while the race runs
    fn update_race<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if self.countdown != 0 {
            self.countdown -= 1;
            if self.countdown == 1 {
                ctx.actors.set_visible(ACTOR_COUNTDOWN, false);
            }
        }
        if self.versus() {
            self.versus_finish(ctx);
        } else {
            self.solo_finish(ctx);
        }
        if self.challenge != 0 && !self.versus() {
            let climb = self.anims[ANIM_THIEF_CLIMB_B];
            if self.thief_y > 100.0 {
                if self.thief_state == THIEF_STOPPED {
                    ctx.actors.set_anims(&ctx.av, ACTOR_RIVAL, climb, climb, false, STATE_RESTART_FORWARD);
                    self.thief_state = THIEF_CLIMBING;
                } else {
                    self.thief_y -= 3.0;
                    ctx.actors.set_y(ACTOR_RIVAL, self.thief_y);
                }
            } else if self.thief_state == THIEF_CLIMBING {
                ctx.actors.set_anims(&ctx.av, ACTOR_RIVAL, climb, climb, false, STATE_STOPPED);
                self.thief_state = THIEF_STOPPED;
            }
        }
        for i in 0..self.jumpers.len() {
            let held = ctx.held(self.jumpers[i].friend);
            self.jumper_input(ctx, i, held);
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
            for j in &self.jumpers {
                let stance = j.anims[J_STANCE];
                ctx.actors.set_anims(&ctx.av, j.actor, stance, stance, false, STATE_RESTART_FORWARD);
            }
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
        if ctx.either_key_down(KEY_SPACE) {
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
            for j in &mut self.jumpers {
                j.state = PLAYER_AIRBORNE;
                ctx.actors.set_visible(j.actor, true);
                ctx.actors.set_pos(j.actor, j.start_x, j.drop + self.ground_y);
                let falling = j.anims[J_FALLING];
                ctx.actors.set_anims(&ctx.av, j.actor, falling, falling, false, STATE_RESTART_FORWARD);
                let ground = ftol(self.ground_y as f64);
                ctx.actors.set_gravity(j.actor, 1.0, 1.0, ground, MAX_FALL_SPEED);
            }
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

    // World y the camera tracks, the resident climber's or the midpoint of both friends
    fn camera_target<H: Host>(&self, ctx: &Ctx<H>) -> f32 {
        let (_, y0) = ctx.actors.pos(self.jumpers[0].actor);
        if self.versus() {
            let (_, y1) = ctx.actors.pos(self.jumpers[1].actor);
            (y0 + y1) / 2.0
        } else {
            y0
        }
    }

    // True once every climber whose result the exit waits on has come to rest
    fn finished(&self) -> bool {
        self.jumpers.iter().all(|j| j.state == PLAYER_FINISHED)
    }
}

impl<H: Host> Screen<H> for JumpScreen {
    // Loads the level for the forms or the challenge, builds its actors and resets every field
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.challenge = ctx.player.challenge_active as i32;
        let guest = ctx.session.guest().cloned();
        let untrained = ctx.player.jumping <= 0 || guest.as_ref().is_some_and(|g| g.jumping <= 0);
        self.intro_pending = self.challenge == 0 && untrained;
        ctx.actors.clear();
        if self.challenge == 0 {
            let stage = guest.as_ref().map_or(ctx.player.stage, |g| ctx.player.stage.min(g.shown_stage()));
            ctx.scene.load(&mut ctx.av, &mut ctx.host, level_name(stage)).await;
        } else {
            ctx.scene.load(&mut ctx.av, &mut ctx.host, "jumpcomlev04").await;
        }
        if self.challenge != 0 || guest.is_some() {
            ctx.actors.add_actor(ACTOR_RIVAL, true);
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
        self.ground_y = (bottom - 0x14) as f32;
        self.thief_y = (bottom - 0x140) as f32;
        self.jumpers.clear();
        let resident = ctx.player.clone();
        let Some(player) = Self::load_jumper(ctx, Skin::host(&resident), ACTOR_PLAYER, false, PLAYER_START_X).await
        else {
            return false;
        };
        self.jumpers.push(player);
        if let Some(g) = &guest {
            let Some(rival) = Self::load_jumper(ctx, Skin::guest(g), ACTOR_RIVAL, true, GUEST_START_X).await else {
                return false;
            };
            self.jumpers.push(rival);
        }
        if !self.load_anims(ctx).await {
            return false;
        }
        if self.challenge == 0 {
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "ps56Jumping.wav", true, 0).await;
        } else {
            if guest.is_none() {
                let Some(h) = ctx.av.get_anim(&mut ctx.host, "thief_climb_climb").await else { return false };
                self.anims[ANIM_THIEF_CLIMB_A] = h;
                let Some(h) = ctx.av.get_anim(&mut ctx.host, "ugjump4pole").await else { return false };
                self.anims[ANIM_POLE] = h;
                let Some(h) = ctx.av.get_anim(&mut ctx.host, "thief_climb_climb").await else { return false };
                self.anims[ANIM_THIEF_CLIMB_B] = h;
            }
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "ps57Challenge.wav", true, 0).await;
        }
        self.time_secs = 0;
        self.time_frac = 0.0;
        self.timer_reset = true;
        self.countdown = COUNTDOWN_START;
        ctx.cursor.set_visible(false);
        self.fly_y = -400.0;
        self.base_y = bottom as f32 - 480.0;
        self.capn_fly_state = FLY_START;
        self.voice = 0;
        self.capn_state = CAPN_NONE;
        self.outro_active = false;
        self.end_delay = 0;
        self.result = RESULT_UNDECIDED;
        self.winner = None;
        true
    }

    // Draws the frame, runs input, physics, the active phase and the camera, then picks the result
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut ret = RESULT_CONTINUE;
        self.backdrop.scroll(&ctx.av, 0.0);
        self.backdrop.fill_and_draw(&mut ctx.av, &mut ctx.rand);
        ctx.scene.draw_layer(&mut ctx.av, 0);
        ctx.scene.draw_layer(&mut ctx.av, 2);
        if self.challenge != 0 && !self.versus() {
            ctx.av.draw_anim_reset(self.anims[ANIM_POLE], 0x141, 0);
        }
        if ctx.cursor.visible {
            let (dx, dy) = ctx.av.get_mouse_delta();
            ctx.cursor.move_by(dx, dy);
            if let Some(id) = ctx.pressed_object() {
                ret = self.on_mouse_hit(ctx, id);
            }
        }
        ctx.actors.update(&ctx.av);
        while let Some(ev) = ctx.actors.pop_event() {
            self.on_actor_event(ctx, ev.kind, ev.actor_id).await;
        }
        for j in &self.jumpers {
            ctx.actors.collect_pickups(&ctx.av, j.actor);
        }
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
        let py = self.camera_target(ctx);
        let (sx, sy) = (ctx.scene.scroll_x, ctx.scene.scroll_y);
        let rel = (py as f64 - sy as f64) as f32;
        let sy = camera_scroll_y(rel, sy, self.max_scroll_y);
        if ctx.either_key_down(KEY_ESC) {
            ret = STATE_HOME_WORLD;
        }
        ctx.scene.set_scroll(sx, sy);
        if self.challenge != 0 && self.finished() {
            self.end_delay += 1;
            if CHALLENGE_EXIT_FRAMES < self.end_delay {
                ctx.player.challenge_won = (self.result == RESULT_PLAYER_FIRST) as u8;
                ret = if self.versus() { STATE_HOME_WORLD } else { STATE_CHALLENGE };
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

    #[test]
    fn levels_follow_the_form_and_bests_only_improve() {
        assert_eq!(level_name(0), "jumpcomlev01");
        assert_eq!(level_name(1), "jumpcomlev02");
        assert_eq!(level_name(2), "jumpcomlev03");
        let mut p = PlayerData { jump_best_secs: 20, jump_best_frac: 50, ..Default::default() };
        record_best(&mut p, 21, 0.0);
        assert_eq!((p.jump_best_secs, p.jump_best_frac), (20, 50));
        record_best(&mut p, 20, 9.0);
        assert_eq!((p.jump_best_secs, p.jump_best_frac), (20, 45));
    }

    #[test]
    fn jumper_numbers_follow_the_record() {
        assert_eq!(Jumper::run_speed(0), 6.0);
        assert_eq!(Jumper::run_speed(1), 8.0);
        assert_eq!(Jumper::run_speed(2), 10.0);
        assert_eq!(Jumper::launch(0), -15.0);
        assert_eq!(Jumper::launch(10), -20.0);
    }

    #[test]
    fn a_screen_of_height_decides_a_friend_climb() {
        assert_eq!(left_behind(1000.0, 1000.0), None);
        assert_eq!(left_behind(600.0, 1000.0), Some(0));
        assert_eq!(left_behind(1000.0, 600.0), Some(1));
        assert_eq!(left_behind(700.0, 1000.0), None);
    }
}
