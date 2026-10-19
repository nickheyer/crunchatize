use avmanager::anim::{STATE_RESTART_FORWARD, STATE_STOPPED};
use avmanager::crt::ftol;
use avmanager::sound::INVALID_HANDLE;
use avmanager::texture::Palette24;

use crate::actors::{EVENT_ANIM_END, EVENT_GROUND_REACHED, EVENT_INIT_OBJECT, FACING_NONE};
use crate::ctx::{
    Ctx, KEY_ESC, KEY_LEFT, KEY_RIGHT, KEY_SPACE, MOUSE_LEFT, RESULT_CONTINUE, STATE_CHALLENGE, STATE_HOME_WORLD,
    STATE_RUNNING, Screen,
};
use crate::host::Host;
use crate::scene::{HOVER_HIDDEN, HOVER_NORMAL, LAYER_ALL};
use crate::scroll_strip::ScrollStrip;

const A_CHECK_PADS: usize = 0;
const A_SKATING: usize = 1;
const A_JUMP: usize = 2;
const A_CRASH: usize = 3;
const A_GETUP: usize = 4;
const A_VICTORY: usize = 5;
const A_GOTO_DEFEAT: usize = 6;
const A_DEFEAT: usize = 7;
const A_SPIN: usize = 8;
const A_STRETCH: usize = 9;
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
const A_ADULT_BBOX: usize = 39;
const ANIM_COUNT: usize = 40;

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

const PS_PRE_INIT: i32 = 0;
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

// The skating race against the turtle or, in the challenge, the thief through the mine
#[derive(Debug)]
pub struct RunningScreen {
    strips: [ScrollStrip; 5],
    slots: Vec<ObstacleSlot>,
    anims: [u16; ANIM_COUNT],
    sfx: [u16; SFX_COUNT],
    speed: f32,
    state: i32,
    max_speed: f32,
    accel: f32,
    brake: f32,
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
    berries: u32,
    last_tick_ms: u32,
    fraction: f32,
    seconds: u16,
    timer_resync: bool,
    countdown: u32,
    finish_dist: f32,
    half_dist: f32,
    cross_dist: f32,
    opp_finish_dist: f32,
    result: i32,
    commentator: i32,
    intro_pending: bool,
    challenge: i32,
    car_speed: f32,
    finish_frames: u32,
    platform_jumps: u32,
}

// Formats a race time the way sprintf(" %d.%02d") did
fn time_text(seconds: u16, hundredths: i32) -> String {
    format!(" {seconds}.{hundredths:02}")
}

impl Default for RunningScreen {
    fn default() -> Self {
        Self {
            strips: Default::default(),
            slots: Vec::new(),
            anims: [0; ANIM_COUNT],
            sfx: [0; SFX_COUNT],
            speed: 0.0,
            state: PS_PRE_INIT,
            max_speed: 0.0,
            accel: 0.0,
            brake: 0.0,
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
            berries: 0,
            last_tick_ms: 0,
            fraction: 0.0,
            seconds: 0,
            timer_resync: false,
            countdown: 0,
            finish_dist: 0.0,
            half_dist: 0.0,
            cross_dist: 0.0,
            opp_finish_dist: 0.0,
            result: RACE_NONE,
            commentator: COMM_DRIVE_IN,
            intro_pending: false,
            challenge: 0,
            car_speed: 0.0,
            finish_frames: 0,
            platform_jumps: 0,
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

    // Runs the opponent on its own until its finish, or by the friend's right and left keys, then brakes it
    fn update_opponent_speed(&mut self, friend: Option<(bool, bool)>) {
        self.opp_finish_dist -= self.opp_speed;
        if 0.0 <= self.opp_finish_dist {
            match friend {
                None | Some((true, _)) => {
                    self.opp_speed += self.opp_accel;
                    if self.opp_max_speed < self.opp_speed {
                        self.opp_speed = self.opp_max_speed;
                    }
                }
                Some((false, true)) => {
                    self.opp_speed -= self.opp_decel;
                    if self.opp_speed < 0.0 {
                        self.opp_speed = 0.0;
                    }
                }
                Some((false, false)) => {}
            }
        } else {
            self.opp_speed = (f64::from(self.opp_speed) - f64::from(self.opp_decel) * f64::from(0.4f32)) as f32;
            if self.opp_speed < 0.0 {
                self.opp_speed = 0.0;
            }
        }
    }

    // Rewards the running stat for done or Again and picks the next state
    fn on_mouse_hit<H: Host>(&self, ctx: &mut Ctx<H>, id: i32) -> u32 {
        let mut result = RESULT_CONTINUE;
        if id == 5 {
            if ctx.player.food != 0 {
                ctx.player.food -= 1;
            }
            if ctx.player.happiness != 0 {
                ctx.player.happiness -= 1;
            }
            ctx.player.add_stats(1, 0, 0);
            if 4 < self.berries {
                ctx.player.add_stats(1, 0, 0);
            }
            result = STATE_HOME_WORLD;
        } else if id == 6 {
            ctx.player.add_stats(1, 0, 0);
            if 4 < self.berries {
                ctx.player.add_stats(1, 0, 0);
            }
            result = STATE_RUNNING;
        }
        result
    }

    // Dispatches INIT_OBJECT and ANIM_END and lands the player on GROUND_REACHED
    async fn on_actor_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, actor_id: i32) {
        let a = self.anims;
        if kind == EVENT_INIT_OBJECT {
            self.on_init_object(ctx, actor_id).await;
        } else if kind == EVENT_ANIM_END {
            self.on_anim_end(ctx, actor_id);
        } else if kind == EVENT_GROUND_REACHED && actor_id == ACTOR_PLAYER {
            if self.state == PS_AIRBORNE || self.state == PS_RIDING {
                ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_SKATING], a[A_SKATING], false, STATE_RESTART_FORWARD);
                self.state = PS_SKATING;
                ctx.av.play_sfx(self.sfx[SFX_LAND], -1);
            } else if self.state == PS_CRASH_AIR {
                self.state = PS_CRASH_SLIDE;
            }
        }
    }

    // Places the player, opponent, car and commentator and hides every other actor
    async fn on_init_object<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        let a = self.anims;
        match actor_id {
            0 => {
                ctx.actors.set_anims(&ctx.av, actor_id, a[A_STRETCH], a[A_STRETCH], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(actor_id, 40.0, self.ground_y);
                ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
                if ctx.player.stage == 2 {
                    ctx.actors.set_bbox_anim(actor_id, a[A_ADULT_BBOX]);
                }
                self.state = PS_SKATING;
            }
            1 => {
                ctx.actors.set_anims(&ctx.av, actor_id, a[A_OPP_WAIT], a[A_OPP_WAIT], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(actor_id, 90.0, self.opp_y);
                ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
            }
            2 => {
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
            3 => {
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

    // Returns the player to skating or the defeat hold and ends the commentator laugh
    fn on_anim_end<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        let a = self.anims;
        if actor_id == ACTOR_PLAYER {
            if self.state == PS_FINISHED {
                ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_DEFEAT], a[A_DEFEAT], false, STATE_RESTART_FORWARD);
            } else {
                ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_SKATING], a[A_SKATING], false, STATE_RESTART_FORWARD);
                self.state = PS_SKATING;
            }
        } else if actor_id == ACTOR_CAPN && self.commentator == COMM_LAUGH {
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_005], a[A_TV_005], false, STATE_RESTART_FORWARD);
            self.commentator = COMM_TV_IDLE;
        }
    }
}

impl<H: Host> Screen<H> for RunningScreen {
    // Loads Running.wam, the strips, actors, anims, sounds and obstacle slots for the variant
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.challenge = i32::from(ctx.player.challenge_active);
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "Running").await;
        ctx.scene.assign_id("berries", 1);
        ctx.scene.assign_id("time", 2);
        ctx.scene.assign_id("BestTitle", 3);
        ctx.scene.assign_id("BestTime", 4);
        ctx.scene.assign_id("done", 5);
        ctx.scene.assign_id("Again", 6);
        ctx.scene.set_text_by_id(2, "0.00");
        ctx.scene.set_text_by_id(1, "0");
        ctx.scene.set_text_by_id(4, "0.00");
        ctx.scene.set_hover_by_id(3, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(4, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(5, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(6, HOVER_HIDDEN);
        ctx.scene.assign_id("GoodJob", 7);
        ctx.scene.set_hover_by_id(7, HOVER_HIDDEN);
        if self.challenge != 0 {
            ctx.scene.set_text_by_id(7, "You Win!");
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
        let dy = PLAYER_OFFSETS[ctx.player.anim_index(false) as usize][1];
        let dx = PLAYER_OFFSETS[ctx.player.anim_index(false) as usize][0];
        ctx.actors.set_draw_offset(ACTOR_PLAYER, dx, dy);
        let adult_as_type0 = ctx.player.stage == 1;
        let name = ctx.player.anim_name("checkPads", false, adult_as_type0);
        let Some(pads) = ctx.av.get_anim_retain_image(&mut ctx.host, &name).await else { return false };
        self.anims[A_CHECK_PADS] = pads;
        let mut palette = Palette24::default();
        ctx.av.get_anim_palette(pads, &mut palette);
        ctx.player.recolor_palette(&mut palette);
        ctx.av.set_anim_palette(pads, &palette);
        let player_anims = [
            (A_SKATING, "skating"),
            (A_JUMP, "jump"),
            (A_CRASH, "crash"),
            (A_GETUP, "getup"),
            (A_VICTORY, "victory"),
            (A_GOTO_DEFEAT, "gotoDefeat"),
            (A_DEFEAT, "defeat"),
            (A_SPIN, "spin"),
            (A_STRETCH, "stretch"),
        ];
        for (index, suffix) in player_anims {
            let name = ctx.player.anim_name(suffix, false, adult_as_type0);
            let Some(h) = ctx.av.get_anim_use_palette(&mut ctx.host, &name, &palette).await else { return false };
            self.anims[index] = h;
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
        if ctx.player.stage == 2 {
            let Some(h) = ctx.av.get_anim(&mut ctx.host, "adult_sk8_bbox").await else { return false };
            self.anims[A_ADULT_BBOX] = h;
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
            self.opp_y = 280.0;
            self.ground_y = 280.0;
        } else {
            let variant_anims = [
                (A_BLOCK, "minecarblock"),
                (A_BLOCK_ANIM, "minecarfallanim"),
                (A_PLATFORM, "sk8ug3platform"),
                (A_OPP_RUN, "thief_sk8_sk8_loop"),
                (A_OPP_WAIT, "thief_sk8_sk8_wait"),
                (A_OPP_LOSE, "thief_sk8_sk8_lose"),
                (A_OPP_VICTORY, "thief_sk8_sk8_victory"),
            ];
            for (index, name) in variant_anims {
                let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
                self.anims[index] = h;
            }
            self.opp_y = 205.0;
            self.ground_y = 320.0;
        }
        let a = self.anims;
        self.speed = 0.0;
        self.max_speed = (f64::from(ctx.player.running) * f64::from(0.3f32) + 14.0) as f32;
        self.accel = 0.6;
        self.brake = 0.7;
        self.state = PS_PRE_INIT;
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
        self.result = RACE_NONE;
        self.berries = 0;
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
        self.platform_jumps = 0;
        true
    }

    // Scrolls, reads input, runs the countdown, race, commentator, collisions, spawning and drawing
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let a = self.anims;
        let s = self.sfx;
        let mut result = RESULT_CONTINUE;
        for strip in &mut self.strips {
            strip.scroll(&ctx.av, self.speed);
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
        ctx.actors.set_x(ACTOR_PLAYER, (f64::from(self.speed) * 6.0 + 50.0) as f32);
        ctx.actors.add_x(ACTOR_OPPONENT, self.opp_speed - self.speed);
        if self.commentator == COMM_DRIVE_OUT {
            ctx.actors.add_x(ACTOR_CAPN, -self.speed);
            ctx.actors.add_x(ACTOR_CAR, -self.speed);
        }
        for slot in &self.slots {
            if slot.active {
                ctx.actors.add_x(slot.actor_id, -self.speed);
            }
        }
        if self.countdown < 0x28 {
            if self.countdown != 0 {
                self.countdown -= 1;
                if self.countdown == 1 {
                    ctx.actors.set_visible(ACTOR_SIGN, false);
                }
            }
            let friend = ctx
                .session
                .helper_present()
                .then(|| (ctx.helper_key_down(KEY_RIGHT), ctx.helper_key_down(KEY_LEFT)));
            self.update_opponent_speed(friend);
            self.cross_dist -= self.speed;
            if self.cross_dist < 0.0 {
                self.speed = (f64::from(self.speed) - f64::from(self.brake) * f64::from(0.8f32)) as f32;
                if self.speed < 0.0 {
                    self.speed = 0.0;
                }
                let mut x = (f64::from(self.speed) * 6.0 + 50.0 - f64::from(self.cross_dist)) as f32;
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
                if self.state == PS_SKATING {
                    if self.result == RACE_PLAYER_FIRST {
                        ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_VICTORY], a[A_VICTORY], false, STATE_RESTART_FORWARD);
                        ctx.actors.set_anims(
                            &ctx.av,
                            ACTOR_OPPONENT,
                            a[A_OPP_LOSE],
                            a[A_OPP_WAIT],
                            false,
                            STATE_RESTART_FORWARD,
                        );
                        if self.challenge != 0 {
                            ctx.scene.set_hover_by_id(7, HOVER_NORMAL);
                        }
                    } else {
                        ctx.actors.set_anims(
                            &ctx.av,
                            ACTOR_PLAYER,
                            a[A_GOTO_DEFEAT],
                            a[A_GOTO_DEFEAT],
                            true,
                            STATE_RESTART_FORWARD,
                        );
                        ctx.actors.set_anims(
                            &ctx.av,
                            ACTOR_OPPONENT,
                            a[A_OPP_VICTORY],
                            a[A_OPP_VICTORY],
                            false,
                            STATE_RESTART_FORWARD,
                        );
                    }
                    self.car_speed = 42.0;
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_CAPN_028], a[A_CAPN_028], false, STATE_RESTART_FORWARD);
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAR, a[A_CAR], a[A_CAR], false, STATE_RESTART_FORWARD);
                    ctx.actors.set_pos(ACTOR_CAPN, -500.0, 290.0);
                    ctx.actors.set_pos(ACTOR_CAR, -650.0, 350.0);
                    ctx.actors.set_layer_mask(ACTOR_CAPN, LAYER_FRONT);
                    ctx.actors.set_layer_mask(ACTOR_PLAYER, LAYER_MAIN);
                    ctx.actors.set_layer_mask(ACTOR_OPPONENT, LAYER_MAIN);
                    self.state = PS_FINISHED;
                    if self.challenge == 0 {
                        self.commentator = COMM_END_DRIVE_IN;
                        let best_frac = ctx.player.run_best_frac as u16;
                        let previous_best = time_text(ctx.player.run_best_secs, i32::from(best_frac));
                        if self.seconds < ctx.player.run_best_secs
                            || (ctx.player.run_best_secs == self.seconds
                                && f64::from(self.fraction) * 5.0 < f64::from(best_frac))
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
            if (self.state == PS_SKATING || self.state == PS_RIDING) && ctx.av.is_key_down(KEY_SPACE) {
                ctx.av.play_sfx(s[SFX_JUMP], -1);
                ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_JUMP], a[A_JUMP], false, STATE_RESTART_FORWARD);
                ctx.actors.set_gravity(ACTOR_PLAYER, -24.0, 1.5, ftol(f64::from(self.ground_y)), 100.0);
                if self.state == PS_RIDING {
                    ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_SPIN], a[A_SPIN], false, STATE_RESTART_FORWARD);
                    ctx.actors.add_y(ACTOR_PLAYER, -20.0);
                    self.platform_jumps += 1;
                    if 2 < self.platform_jumps {
                        self.platform_jumps = 0;
                        if self.commentator == COMM_TV_IDLE {
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
                self.state = PS_AIRBORNE;
            }
            if self.state == PS_SKATING || self.state == PS_RIDING || self.state == PS_AIRBORNE {
                if ctx.av.is_key_down(KEY_RIGHT) {
                    self.speed += self.accel;
                    if self.max_speed < self.speed {
                        self.speed = self.max_speed;
                    }
                }
                if ctx.av.is_key_down(KEY_LEFT) {
                    self.speed -= self.brake;
                    if self.speed < 0.5 {
                        self.speed = 0.5;
                    }
                }
            } else if self.state == PS_CRASH_SLIDE {
                self.speed -= 0.25;
                if self.speed < 0.5 {
                    self.speed = 0.5;
                    ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_GETUP], a[A_GETUP], true, STATE_RESTART_FORWARD);
                    self.state = PS_GETTING_UP;
                }
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
            ctx.scene.set_text_by_id(2, &time_text(self.seconds, hundredths));
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
                ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_SKATING], a[A_SKATING], false, STATE_RESTART_FORWARD);
                ctx.actors.set_anims(&ctx.av, ACTOR_OPPONENT, a[A_OPP_RUN], a[A_OPP_RUN], false, STATE_RESTART_FORWARD);
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
        for i in 0..self.slots.len() {
            let slot = self.slots[i];
            if !slot.active || !ctx.actors.overlaps(&ctx.av, ACTOR_PLAYER, slot.actor_id) {
                continue;
            }
            let first = !slot.hit;
            let mut apply_alt = first;
            if slot.kind == KIND_CRASH || slot.kind == KIND_PLATFORM {
                let mut crash = first;
                if slot.kind == KIND_PLATFORM && (self.state == PS_AIRBORNE || self.state == PS_RIDING) {
                    let mut vy = ctx.actors.vel_y(ACTOR_PLAYER);
                    let player = ctx.actors.bbox(&ctx.av, ACTOR_PLAYER);
                    let obstacle = ctx.actors.bbox(&ctx.av, slot.actor_id);
                    if vy < 3.0 {
                        vy = 3.0;
                    }
                    if f64::from(player[3]) < f64::from(obstacle[1]) + f64::from(vy) {
                        ctx.actors.add_y(ACTOR_PLAYER, (-(player[3] - obstacle[1])) as f32);
                        ctx.actors.set_gravity(ACTOR_PLAYER, 0.0, 2.0, ftol(f64::from(self.ground_y)), 100.0);
                        crash = false;
                        apply_alt = false;
                        if self.state != PS_RIDING {
                            self.state = PS_RIDING;
                            ctx.av.play_sfx(s[SFX_LAND], -1);
                        }
                    }
                }
                if crash && (self.state == PS_SKATING || self.state == PS_AIRBORNE || self.state == PS_RIDING) {
                    if self.commentator != COMM_TV_SPEECH && self.commentator != COMM_LAUGH {
                        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_004], a[A_TV_004], true, STATE_RESTART_FORWARD);
                        self.commentator = COMM_LAUGH;
                    }
                    ctx.av.play_sfx(s[SFX_CRASH], -1);
                    ctx.actors.set_anims(&ctx.av, ACTOR_PLAYER, a[A_CRASH], a[A_CRASH], false, STATE_RESTART_FORWARD);
                    ctx.actors.set_gravity(ACTOR_PLAYER, -16.0, 2.0, ftol(f64::from(self.ground_y)), 100.0);
                    self.state = PS_CRASH_AIR;
                    if self.speed < 7.0 {
                        self.speed = 7.0;
                    }
                }
            } else if first && slot.kind == KIND_BERRY {
                ctx.av.play_sfx(s[SFX_BERRY], -1);
                ctx.actors.set_visible(slot.actor_id, false);
                self.berries += 1;
                ctx.scene.set_text_by_id(1, &self.berries.to_string());
            } else if first && slot.kind == KIND_FINISH {
                apply_alt = false;
            } else if first {
                if self.speed <= 3.0 {
                    self.speed = 0.5;
                } else {
                    self.speed -= 3.0;
                }
            }
            if apply_alt {
                ctx.actors.set_anims(&ctx.av, slot.actor_id, slot.alt, slot.alt, false, STATE_RESTART_FORWARD);
                ctx.actors.set_layer_mask(slot.actor_id, LAYER_FRONT);
                self.slots[i].hit = true;
            }
        }
        if self.speed <= 1.0 || (self.state != PS_SKATING && self.state != PS_RIDING) {
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
            self.spawn_dist -= self.speed;
        } else {
            let i = ctx.rand_range(0, self.slots.len() as i32 - 1) as usize;
            let slot = self.slots[i];
            if !slot.active {
                let y = self.ground_y + 85.0;
                let x = ctx.rand_range(0, 150) as f32 + 800.0;
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
            self.finish_dist -= self.speed;
            if self.finish_dist < 0.0 {
                let last = self.slots.len() - 1;
                let line = self.slots[last];
                ctx.actors.set_pos(line.actor_id, 700.0, 367.0);
                ctx.actors.set_anims(&ctx.av, line.actor_id, line.idle, line.idle, false, STATE_RESTART_FORWARD);
                ctx.actors.set_visible(line.actor_id, true);
                self.slots[last].active = true;
                self.finish_dist = -1.0;
                ctx.actors.set_layer_mask(ACTOR_PLAYER, LAYER_FRONT);
                ctx.actors.set_layer_mask(ACTOR_OPPONENT, LAYER_FRONT);
            }
        }
        if self.half_dist != 0.0 && self.finish_dist < self.half_dist {
            self.half_dist = 0.0;
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, a[A_TV_006], a[A_TV_006], false, STATE_RESTART_FORWARD);
            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "058.wav", false, self.voice).await;
            self.commentator = COMM_TV_SPEECH;
        }
        if self.state == PS_RIDING {
            let row = ctx.player.anim_index(false) as usize;
            ctx.actors.add_y(ACTOR_PLAYER, PLAYER_OFFSETS[row][2] as f32);
        }
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_MAIN);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_FRONT);
        if self.state == PS_RIDING {
            let row = ctx.player.anim_index(false) as usize;
            ctx.actors.add_y(ACTOR_PLAYER, (-PLAYER_OFFSETS[row][2]) as f32);
        }
        if ctx.av.is_key_down(KEY_ESC) {
            result = STATE_HOME_WORLD;
        }
        if self.challenge != 0 && self.state == PS_FINISHED {
            self.finish_frames += 1;
            if 0x78 < self.finish_frames {
                ctx.player.challenge_won = u8::from(self.result == RACE_PLAYER_FIRST);
                result = STATE_CHALLENGE;
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
        g.update_opponent_speed(None);
        assert_eq!(g.opp_speed, 14.0);
        g.update_opponent_speed(None);
        assert_eq!(g.opp_speed, (14.0f64 - f64::from(1.2f32) * f64::from(0.4f32)) as f32);
        g.opp_speed = 0.1;
        g.update_opponent_speed(None);
        assert_eq!(g.opp_speed, 0.0);
    }

    #[test]
    fn friend_keys_drive_the_opponent() {
        let mut g = RunningScreen::new();
        g.opp_finish_dist = 1000.0;
        g.opp_max_speed = 14.0;
        g.opp_accel = 0.7;
        g.opp_decel = 1.2;
        g.update_opponent_speed(Some((false, false)));
        assert_eq!(g.opp_speed, 0.0);
        g.update_opponent_speed(Some((true, false)));
        assert_eq!(g.opp_speed, 0.7);
        g.update_opponent_speed(Some((false, true)));
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
}
