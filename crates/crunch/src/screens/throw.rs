use avmanager::anim::{STATE_RESTART_BACKWARD, STATE_RESTART_FORWARD, STATE_STOPPED};
use avmanager::texture::Palette24;

use crate::actors::{
    EVENT_ANIM_END, EVENT_GROUND_REACHED, EVENT_INIT_OBJECT, EVENT_TARGET_REACHED, EVENT_TIMER, FACING_NONE, NO_TARGET,
};
use crate::ctx::{
    Ctx, KEY_ESC, KEY_LEFT, KEY_RIGHT, KEY_SPACE, MOUSE_LEFT, RESULT_CONTINUE, STATE_CHALLENGE, STATE_HOME_WORLD,
    STATE_THROWING, Screen,
};
use crate::host::Host;
use crate::player::PlayerData;
use crate::scene::{HOVER_HIDDEN, HOVER_NORMAL};

const A_STANCE: usize = 0;
const A_THROW: usize = 1;
const A_SIDESTEP_LEFT: usize = 2;
const A_SIDESTEP_RIGHT: usize = 3;
const A_THROW_RELOAD: usize = 4;
const A_ROCK: usize = 5;
const A_ROCK_IMPACT: usize = 6;
const A_LAVABALL: usize = 7;
const A_LAVABALL_EXPLODE: usize = 8;
const A_ERUPTION_LEFT: usize = 9;
const A_ERUPTION_RIGHT: usize = 10;
const A_TERRA_FLY: usize = 11;
const A_TERRA_FLY_HIT: usize = 12;
const A_GO: usize = 13;
const A_ONE: usize = 14;
const A_TWO: usize = 15;
const A_THREE: usize = 16;
const A_THIEF_STANCE: usize = 17;
const A_THIEF_THROW: usize = 18;
const A_THIEF_RELOAD: usize = 19;
const A_LIGHTNING: usize = 20;
const A_JURICAR_ZAP: usize = 21;
const A_JURICAR: usize = 22;
const A_CAPN_021: usize = 23;
const A_CAPN_013: usize = 24;
const A_CAPN_029: usize = 25;
const A_CAPN_037: usize = 26;
const A_CAPN_010: usize = 27;
const A_CAP_TV_005: usize = 28;
const A_CAP_TV_006: usize = 29;

const S_ROCKTHROWER: usize = 0;
const S_ROCKTHROWER2: usize = 1;
const S_RIMPACT: usize = 2;
const S_BERRY: usize = 3;
const S_LAVASHOOT: usize = 4;
const S_TERAHIT: usize = 5;
const S_TERADACTYL: usize = 6;
const S_LIGHTNING: usize = 7;
const S_FADEIN: usize = 8;

const ACTOR_PLAYER: i32 = 0;
const ACTOR_THIEF: i32 = 1;
const ACTOR_ROCK0: i32 = 2;
const ACTOR_LAVA0: i32 = 6;
const ACTOR_ERUPT_LEFT: i32 = 10;
const ACTOR_ERUPT_RIGHT: i32 = 0xb;
const ACTOR_PTERO: i32 = 0xc;
const ACTOR_CAPN: i32 = 0xd;
const ACTOR_JURICAR: i32 = 0xe;
const ACTOR_LIGHTNING: i32 = 0xf;
const ACTOR_COUNTDOWN: i32 = 0x10;

const OBJ_SCORE: i32 = 1;
const OBJ_SCORE_TEXT: i32 = 2;
const OBJ_TIME: i32 = 3;
const OBJ_TIME_TEXT: i32 = 4;
const OBJ_DONE: i32 = 5;
const OBJ_GOOD_JOB: i32 = 6;
const OBJ_AGAIN: i32 = 7;

const ROCK_SLOTS: usize = 4;
const LAVA_SLOTS: usize = 4;

const SLOT_FREE: i32 = 0;
const SLOT_FLYING: i32 = 1;
const SLOT_HIT: i32 = 2;

const PS_STANCE: i32 = 1;
const PS_STEP_RIGHT: i32 = 2;
const PS_STEP_LEFT: i32 = 3;
const PS_THROWING: i32 = 4;
const PS_GAME_OVER: i32 = 5;
const PS_RELOADING: i32 = 6;

const CAPN_INTRO_TALK: i32 = 1;
const CAPN_INTRO_END_TALK: i32 = 4;
const CAPN_SKIPPED: i32 = 5;
const CAPN_DEMO_THROW: i32 = 6;
const CAPN_TV_IDLE: i32 = 7;
const CAPN_TV_TALKING: i32 = 8;

const PHASE_NORMAL: i32 = 0;
const PHASE_CHALLENGE_START: i32 = 1;
const PHASE_CHALLENGE_COUNTDOWN: i32 = 3;

const OUTCOME_WIN: i32 = 1;
const OUTCOME_LOSE: i32 = 2;

const COUNTDOWN_START: u32 = 0x83;
const COUNTDOWN_THREE: u32 = 0x82;
const COUNTDOWN_TWO: u32 = 100;
const COUNTDOWN_ONE: u32 = 0x46;
const COUNTDOWN_GO: u32 = 0x28;

const LEFT_CLAMP: f32 = 80.0;
const RIGHT_CLAMP: f32 = 502.0;
const STEP: f32 = 8.0;
const PTERO_SPEED: f32 = 12.0;
const OFF_SCREEN: f32 = -100.0;
const TICKS_PER_SECOND: f32 = 20.0;
const MS_PER_TICK: f32 = 50.0;
const LAVA_GRAVITY: f32 = f32::from_bits(0x3ecc_cccd);
const LAYER_MAIN: u32 = 1;
const LAYER_CAPN: u32 = 4;
const LAYER_TV: u32 = 8;

// Draw offset and rock spawn offset for one stage and type row of the table at 0x42a5e8
#[derive(Clone, Copy)]
struct StageRow {
    draw_off_x: i32,
    draw_off_y: i32,
    rock_dx: i32,
    rock_dy: i32,
}

const STAGE_TABLE: [StageRow; 9] = [
    StageRow { draw_off_x: 0, draw_off_y: 50, rock_dx: 3, rock_dy: 35 },
    StageRow { draw_off_x: 0, draw_off_y: 50, rock_dx: 3, rock_dy: 35 },
    StageRow { draw_off_x: 0, draw_off_y: 50, rock_dx: 3, rock_dy: 35 },
    StageRow { draw_off_x: 0, draw_off_y: 0, rock_dx: 12, rock_dy: 5 },
    StageRow { draw_off_x: 0, draw_off_y: 0, rock_dx: 12, rock_dy: 5 },
    StageRow { draw_off_x: 0, draw_off_y: 0, rock_dx: 12, rock_dy: 5 },
    StageRow { draw_off_x: 0, draw_off_y: -31, rock_dx: 0, rock_dy: 0 },
    StageRow { draw_off_x: 0, draw_off_y: -31, rock_dx: 0, rock_dy: 0 },
    StageRow { draw_off_x: 0, draw_off_y: -31, rock_dx: 0, rock_dy: 0 },
];

// One thrown rock, slot i drives actor 2 + i
#[derive(Clone, Copy, Default)]
struct RockSlot {
    state: i32,
    y: f32,
    speed: f32,
    owner: i32,
}

// The rock throwing mini-game, state 0xb, with its tutorial, countdown and challenge variant
#[derive(Default)]
pub struct ThrowScreen {
    anims: [u16; 30],
    sfx: [u16; 9],
    cursor_anim: u16,
    music: u32,
    voice: u32,
    score: u32,
    thief_score: u32,
    countdown: u32,
    player_state: i32,
    thief_state: i32,
    capn_state: i32,
    intro_step: i32,
    intro_waiting_voice: bool,
    intro_active: bool,
    challenge_phase: i32,
    exit_frames: u32,
    outcome: i32,
    timer_accum: f32,
    seconds_left: i16,
    first_tick: bool,
    last_time_ms: u32,
    rocks: [RockSlot; ROCK_SLOTS],
    left_erupting: bool,
    right_erupting: bool,
    erupt_cooldown: i32,
    lava_count: u32,
    lava: [i32; LAVA_SLOTS],
    ptero_respawn: i32,
    ptero_x: f32,
    ptero_stun: i32,
    commentary_counter: u32,
}

// Installs the same animation for both facings and restarts it in the given mode
fn set_actor_anim<H: Host>(ctx: &mut Ctx<H>, id: i32, anim: u16, notify: bool, mode: i16) {
    ctx.actors.set_anims(&ctx.av, id, anim, anim, notify, mode);
}

impl ThrowScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Loads the WAM, sounds, actors and animations, false when any asset is missing
    async fn init_screen<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.challenge_phase = ctx.player.challenge_active as i32;
        self.intro_active = self.challenge_phase == PHASE_NORMAL;
        if self.challenge_phase == PHASE_NORMAL {
            ctx.scene.load(&mut ctx.av, &mut ctx.host, "throw").await;
            ctx.scene.assign_id("time", OBJ_TIME);
            ctx.scene.assign_id("timeText", OBJ_TIME_TEXT);
            ctx.scene.assign_id("GoodJob", OBJ_GOOD_JOB);
            ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_HIDDEN);
            ctx.scene.set_text_by_id(OBJ_TIME, "60");
            ctx.scene.assign_id("Again", OBJ_AGAIN);
            ctx.scene.set_hover_by_id(OBJ_AGAIN, HOVER_HIDDEN);
        } else {
            ctx.scene.load(&mut ctx.av, &mut ctx.host, "throw_challenge").await;
            ctx.scene.assign_id("time", OBJ_TIME);
            ctx.scene.assign_id("timeText", OBJ_TIME_TEXT);
            ctx.scene.assign_id("scoreText", OBJ_SCORE_TEXT);
            ctx.scene.assign_id("GoodJob", OBJ_GOOD_JOB);
            ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_HIDDEN);
            ctx.scene.set_text_by_id(OBJ_TIME, "0");
            ctx.scene.set_text_by_id(OBJ_TIME_TEXT, "Thief");
            let name = ctx.player.name_str();
            ctx.scene.set_text_by_id(OBJ_SCORE_TEXT, &name);
        }
        ctx.scene.assign_id("score", OBJ_SCORE);
        ctx.scene.assign_id("done", OBJ_DONE);
        ctx.scene.set_text_by_id(OBJ_SCORE, "0");
        ctx.scene.set_hover_by_id(OBJ_DONE, HOVER_HIDDEN);
        let Some(h) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        self.cursor_anim = h;
        ctx.cursor.set_anim(&ctx.av, self.cursor_anim);
        ctx.cursor.set_pos(0x140, 400);
        ctx.cursor.set_visible(true);
        let sounds = [
            ("cs_rimpact.wav", S_RIMPACT),
            ("cs_rockthrower.wav", S_ROCKTHROWER),
            ("cs_rockthrower2.wav", S_ROCKTHROWER2),
            ("cs_berry.wav", S_BERRY),
            ("cs_terahit.wav", S_TERAHIT),
            ("cs_teradactyl.wav", S_TERADACTYL),
            ("cs_fadein.wav", S_FADEIN),
            ("cs_lightning.wav", S_LIGHTNING),
            ("cs_lavashoot4.wav", S_LAVASHOOT),
        ];
        for (name, slot) in sounds {
            let Some(h) = ctx.av.get_sfx(&mut ctx.host, name, 0, false).await else { return false };
            self.sfx[slot] = h;
        }
        ctx.actors.clear();
        for id in [ACTOR_ERUPT_LEFT, ACTOR_ERUPT_RIGHT, 6, 7, 8, 9, ACTOR_PTERO, ACTOR_CAPN] {
            ctx.actors.add_actor(id, true);
        }
        if self.challenge_phase == PHASE_NORMAL {
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "tg56Throwing.wav", true, 0).await;
        } else {
            ctx.actors.add_actor(ACTOR_THIEF, true);
            self.music = ctx.av.play_stream_sound(&mut ctx.host, "tg57Challenge.wav", true, 0).await;
        }
        for id in [ACTOR_JURICAR, ACTOR_LIGHTNING, ACTOR_PLAYER, 2, 3, 4, 5, ACTOR_COUNTDOWN] {
            ctx.actors.add_actor(id, true);
        }
        let row = STAGE_TABLE[ctx.player.anim_index(false) as usize];
        ctx.actors.set_draw_offset(ACTOR_PLAYER, row.draw_off_x, row.draw_off_y);
        let name = ctx.player.anim_name("throw_stance", false, false);
        let Some(h) = ctx.av.get_anim_retain_image(&mut ctx.host, &name).await else { return false };
        self.anims[A_STANCE] = h;
        let mut pal = Palette24::default();
        ctx.av.get_anim_palette(self.anims[A_STANCE], &mut pal);
        ctx.player.recolor_palette(&mut pal);
        ctx.av.set_anim_palette(self.anims[A_STANCE], &pal);
        let recolored = [
            ("throw", A_THROW),
            ("sidestep", A_SIDESTEP_LEFT),
            ("sidestep", A_SIDESTEP_RIGHT),
            ("throwReload", A_THROW_RELOAD),
        ];
        for (suffix, slot) in recolored {
            let name = ctx.player.anim_name(suffix, false, false);
            let Some(h) = ctx.av.get_anim_use_palette(&mut ctx.host, &name, &pal).await else { return false };
            self.anims[slot] = h;
        }
        let shared = [
            ("rock", A_ROCK),
            ("rockimpact", A_ROCK_IMPACT),
            ("lavaballanim", A_LAVABALL),
            ("lavaballexplode", A_LAVABALL_EXPLODE),
            ("eruption", A_ERUPTION_LEFT),
            ("eruption", A_ERUPTION_RIGHT),
            ("terra_fly", A_TERRA_FLY),
            ("terra_fly_hit", A_TERRA_FLY_HIT),
            ("capn_cap021", A_CAPN_021),
            ("capn_cap013", A_CAPN_013),
            ("capn_cap037", A_CAPN_037),
            ("capn_cap010", A_CAPN_010),
            ("capn_cap029", A_CAPN_029),
            ("cap_tv_cap005", A_CAP_TV_005),
            ("cap_tv_cap006", A_CAP_TV_006),
            ("juricar_zap", A_JURICAR_ZAP),
            ("juricar", A_JURICAR),
            ("lightning", A_LIGHTNING),
            ("one", A_ONE),
            ("two", A_TWO),
            ("three", A_THREE),
            ("go", A_GO),
        ];
        for (name, slot) in shared {
            let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
            self.anims[slot] = h;
        }
        if self.challenge_phase != PHASE_NORMAL {
            let thief = [
                ("thief_throw_stance", A_THIEF_STANCE),
                ("thief_throw_throw", A_THIEF_THROW),
                ("thief_throw_throw_reload", A_THIEF_RELOAD),
            ];
            for (name, slot) in thief {
                let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
                self.anims[slot] = h;
            }
        }
        for rock in &mut self.rocks {
            rock.state = SLOT_FREE;
            rock.y = OFF_SCREEN;
        }
        self.player_state = 0;
        self.capn_state = 0;
        self.intro_step = 0;
        self.intro_waiting_voice = false;
        self.voice = 0;
        self.score = 0;
        self.thief_score = 0;
        self.seconds_left = 0x3c;
        self.timer_accum = 0.0;
        self.first_tick = true;
        self.countdown = COUNTDOWN_START;
        ctx.cursor.set_visible(false);
        self.erupt_cooldown = 0;
        self.lava_count = 0;
        self.lava = [SLOT_FREE; LAVA_SLOTS];
        self.right_erupting = false;
        self.left_erupting = false;
        self.ptero_respawn = 0x168;
        self.ptero_x = 0.0;
        self.exit_frames = 0;
        self.commentary_counter = 0;
        if ctx.player.strength > 0 {
            self.intro_active = false;
        }
        true
    }

    // Runs one frame of input, motion, scoring and drawing and returns the next state
    async fn update_screen<H: Host>(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut ret = RESULT_CONTINUE;
        if ctx.cursor.visible {
            let (dx, dy) = ctx.av.get_mouse_delta();
            ctx.cursor.move_by(dx, dy);
            let (mx, my) = (ctx.cursor.x, ctx.cursor.y);
            if let Some(id) = ctx.scene.hit_test(mx as i32, my as i32)
                && ctx.av.is_mouse_button_down(MOUSE_LEFT)
            {
                ret = Self::on_scene_click(&mut ctx.player, id);
            }
        }
        ctx.actors.update(&ctx.av);
        while let Some(ev) = ctx.actors.pop_event() {
            self.on_actor_event(ctx, ev.kind, ev.actor_id).await;
        }
        for (slot, rock) in self.rocks.iter_mut().enumerate() {
            if rock.state != SLOT_FREE {
                rock.y -= rock.speed;
                ctx.actors.set_y(ACTOR_ROCK0 + slot as i32, rock.y);
                if rock.y < OFF_SCREEN {
                    rock.state = SLOT_FREE;
                    ctx.actors.set_visible(ACTOR_ROCK0 + slot as i32, false);
                }
            }
        }
        if self.ptero_respawn == 0 {
            if self.ptero_stun == 0 {
                self.ptero_x -= PTERO_SPEED;
                ctx.actors.set_x(ACTOR_PTERO, self.ptero_x);
            } else {
                self.ptero_stun -= 1;
                if self.ptero_stun == 0 {
                    set_actor_anim(ctx, ACTOR_PTERO, self.anims[A_TERRA_FLY], false, STATE_RESTART_FORWARD);
                }
            }
            if self.ptero_x < OFF_SCREEN {
                self.ptero_respawn = 0x168;
                ctx.actors.set_visible(ACTOR_PTERO, false);
            }
        }
        if self.countdown < COUNTDOWN_GO {
            if self.countdown != 0 {
                self.countdown -= 1;
                if self.countdown == 1 {
                    ctx.actors.set_visible(ACTOR_COUNTDOWN, false);
                }
            }
            let over = (self.challenge_phase == PHASE_NORMAL && self.seconds_left == 0)
                || (self.challenge_phase != PHASE_NORMAL && (self.score > 9 || self.thief_score > 9));
            if over {
                self.outcome = if self.score < 10 || self.thief_score > 9 { OUTCOME_LOSE } else { OUTCOME_WIN };
                if self.player_state != PS_GAME_OVER {
                    if self.player_state != PS_STANCE {
                        set_actor_anim(ctx, ACTOR_PLAYER, self.anims[A_STANCE], false, STATE_RESTART_FORWARD);
                    }
                    if self.challenge_phase != PHASE_NORMAL && self.thief_state != PS_STANCE {
                        set_actor_anim(ctx, ACTOR_THIEF, self.anims[A_THIEF_STANCE], false, STATE_RESTART_FORWARD);
                    }
                    self.player_state = PS_GAME_OVER;
                    if self.challenge_phase == PHASE_NORMAL {
                        let old_high = format!("{}", ctx.player.throw_high_score);
                        if ctx.player.throw_high_score < self.score {
                            ctx.player.throw_high_score = self.score;
                        }
                        ctx.scene.set_text_by_id(OBJ_TIME, &old_high);
                        ctx.scene.set_text_by_id(OBJ_TIME_TEXT, "High Score");
                        self.voice = ctx.av.play_stream_sound(&mut ctx.host, "041.wav", false, self.voice).await;
                        set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAP_TV_006], false, STATE_RESTART_FORWARD);
                        self.capn_state = CAPN_TV_TALKING;
                        ctx.scene.set_hover_by_id(OBJ_DONE, HOVER_NORMAL);
                        ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_NORMAL);
                        ctx.scene.set_hover_by_id(OBJ_AGAIN, HOVER_NORMAL);
                        ctx.cursor.set_visible(true);
                    } else if self.outcome == OUTCOME_WIN {
                        ctx.scene.set_hover_by_id(OBJ_GOOD_JOB, HOVER_NORMAL);
                    }
                }
            } else {
                if self.player_state != PS_THROWING && self.player_state != PS_RELOADING && ctx.av.is_key_down(KEY_SPACE) {
                    if ctx.rand_range(0, 2) == 0 {
                        ctx.av.play_sfx(self.sfx[S_ROCKTHROWER2], -1);
                    } else {
                        ctx.av.play_sfx(self.sfx[S_ROCKTHROWER], -1);
                    }
                    set_actor_anim(ctx, ACTOR_PLAYER, self.anims[A_THROW], true, STATE_RESTART_FORWARD);
                    self.player_state = PS_THROWING;
                    let speed = ctx.player.strength as f32 + 12.0;
                    let dy = STAGE_TABLE[ctx.player.anim_index(false) as usize].rock_dy as f32;
                    let dx = STAGE_TABLE[ctx.player.anim_index(false) as usize].rock_dx as f32;
                    self.spawn_rock(ctx, ACTOR_PLAYER, dx, dy, speed);
                }
                if matches!(self.player_state, PS_RELOADING | PS_STANCE | PS_STEP_RIGHT | PS_STEP_LEFT) {
                    let mut new_state = PS_STANCE;
                    if ctx.av.is_key_down(KEY_RIGHT) {
                        let (mut x, _) = ctx.actors.pos(ACTOR_PLAYER);
                        x += STEP;
                        if x > RIGHT_CLAMP {
                            x = RIGHT_CLAMP;
                        }
                        ctx.actors.set_x(ACTOR_PLAYER, x);
                        if self.player_state != PS_STEP_RIGHT && self.player_state != PS_RELOADING {
                            set_actor_anim(ctx, ACTOR_PLAYER, self.anims[A_SIDESTEP_RIGHT], false, STATE_RESTART_FORWARD);
                        }
                        new_state = PS_STEP_RIGHT;
                    }
                    if ctx.av.is_key_down(KEY_LEFT) {
                        let (mut x, _) = ctx.actors.pos(ACTOR_PLAYER);
                        x -= STEP;
                        if x < LEFT_CLAMP {
                            x = LEFT_CLAMP;
                        }
                        ctx.actors.set_x(ACTOR_PLAYER, x);
                        if self.player_state != PS_STEP_LEFT && self.player_state != PS_RELOADING {
                            set_actor_anim(ctx, ACTOR_PLAYER, self.anims[A_SIDESTEP_LEFT], false, STATE_RESTART_FORWARD);
                        }
                        new_state = PS_STEP_LEFT;
                    }
                    if self.player_state != PS_STANCE && self.player_state != PS_RELOADING && new_state == PS_STANCE {
                        set_actor_anim(ctx, ACTOR_PLAYER, self.anims[A_STANCE], false, STATE_RESTART_FORWARD);
                    }
                    if self.player_state != PS_RELOADING {
                        self.player_state = new_state;
                    }
                }
                if self.challenge_phase == PHASE_NORMAL {
                    let now = ctx.time_ms();
                    let mut dt = now.wrapping_sub(self.last_time_ms) as f32;
                    if self.first_tick {
                        dt = 0.0;
                        self.first_tick = false;
                    }
                    self.last_time_ms = now;
                    self.timer_accum += dt / MS_PER_TICK;
                    while self.timer_accum >= TICKS_PER_SECOND {
                        self.timer_accum -= TICKS_PER_SECOND;
                        self.seconds_left = self.seconds_left.wrapping_sub(1);
                    }
                    let text = format!(" {}", self.seconds_left as u16);
                    ctx.scene.set_text_by_id(OBJ_TIME, &text);
                } else if self.thief_state != PS_THROWING && self.thief_state != PS_RELOADING {
                    ctx.av.play_sfx(self.sfx[S_ROCKTHROWER2], -1);
                    set_actor_anim(ctx, ACTOR_THIEF, self.anims[A_THIEF_THROW], true, STATE_RESTART_FORWARD);
                    self.thief_state = PS_THROWING;
                }
                if self.challenge_phase == PHASE_NORMAL && self.ptero_respawn != 0 {
                    if self.ptero_respawn == 0x14 {
                        ctx.av.play_sfx(self.sfx[S_TERADACTYL], -1);
                    }
                    if self.ptero_respawn == 1 {
                        set_actor_anim(ctx, ACTOR_PTERO, self.anims[A_TERRA_FLY], false, STATE_RESTART_FORWARD);
                        ctx.actors.set_visible(ACTOR_PTERO, true);
                        self.ptero_x = 700.0;
                        ctx.actors.set_pos(ACTOR_PTERO, self.ptero_x, -10.0);
                        self.ptero_stun = 0;
                    }
                    self.ptero_respawn -= 1;
                }
            }
        } else if !self.intro_active {
            if self.challenge_phase == PHASE_NORMAL || self.challenge_phase == PHASE_CHALLENGE_COUNTDOWN {
                self.countdown -= 1;
            } else if self.challenge_phase == PHASE_CHALLENGE_START {
                ctx.actors.set_visible(ACTOR_CAPN, true);
                ctx.actors.set_pos(ACTOR_CAPN, 512.0, 4.0);
                ctx.actors.set_screen_fixed(ACTOR_CAPN, true);
                ctx.actors.set_draw_offset(ACTOR_CAPN, 0, 0);
                set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAP_TV_006], false, STATE_RESTART_FORWARD);
                ctx.actors.set_layer_mask(ACTOR_CAPN, LAYER_TV);
                self.capn_state = CAPN_TV_TALKING;
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, "049.wav", false, self.voice).await;
                self.challenge_phase += 1;
            } else if ctx.stream_done(self.voice) {
                self.challenge_phase += 1;
            }
            if self.countdown == COUNTDOWN_GO {
                set_actor_anim(ctx, ACTOR_COUNTDOWN, self.anims[A_GO], false, STATE_STOPPED);
            } else if self.countdown == COUNTDOWN_ONE {
                set_actor_anim(ctx, ACTOR_COUNTDOWN, self.anims[A_ONE], false, STATE_STOPPED);
            } else if self.countdown == COUNTDOWN_TWO {
                set_actor_anim(ctx, ACTOR_COUNTDOWN, self.anims[A_TWO], false, STATE_STOPPED);
            } else if self.countdown == COUNTDOWN_THREE {
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, "032.wav", false, self.voice).await;
                set_actor_anim(ctx, ACTOR_COUNTDOWN, self.anims[A_THREE], false, STATE_STOPPED);
                ctx.actors.set_visible(ACTOR_COUNTDOWN, true);
                ctx.actors.set_layer_mask(ACTOR_COUNTDOWN, LAYER_TV);
                ctx.actors.set_pos(ACTOR_COUNTDOWN, 320.0, 180.0);
                set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAP_TV_006], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(ACTOR_CAPN, 512.0, 4.0);
                ctx.actors.set_visible(ACTOR_CAPN, true);
                self.capn_state = CAPN_TV_TALKING;
            }
        } else if ctx.av.is_key_down(KEY_SPACE) {
            ctx.av.kill_streaming_channel(self.voice);
            ctx.actors.walk_to(&ctx.av, ACTOR_CAPN, NO_TARGET);
            ctx.actors.set_visible(ACTOR_JURICAR, false);
            ctx.actors.set_visible(ACTOR_LIGHTNING, false);
            ctx.actors.set_visible(ACTOR_PLAYER, true);
            ctx.actors.set_timer(ACTOR_CAPN, 0);
            self.capn_state = CAPN_SKIPPED;
            self.intro_active = false;
        } else if self.intro_waiting_voice && ctx.stream_done(self.voice) {
            self.intro_waiting_voice = false;
            self.advance_intro(ctx).await;
        }
        if self.capn_state == CAPN_TV_TALKING && ctx.stream_done(self.voice) {
            self.capn_state = CAPN_TV_IDLE;
            set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAP_TV_005], false, STATE_RESTART_FORWARD);
        }
        if self.player_state != PS_GAME_OVER {
            if self.erupt_cooldown == 0 {
                self.try_erupt(ctx);
            } else {
                self.erupt_cooldown -= 1;
            }
        }
        ctx.scene.draw_layer(&mut ctx.av, 0);
        let old_score = self.score;
        let old_thief_score = self.thief_score;
        for slot in 0..ROCK_SLOTS {
            if self.rocks[slot].state != SLOT_FLYING {
                continue;
            }
            let rock_id = ACTOR_ROCK0 + slot as i32;
            for lava_slot in 0..LAVA_SLOTS {
                let lava_id = ACTOR_LAVA0 + lava_slot as i32;
                if self.lava[lava_slot] == SLOT_FLYING && ctx.actors.overlaps(&ctx.av, rock_id, lava_id) {
                    if self.capn_state == CAPN_DEMO_THROW {
                        self.advance_intro(ctx).await;
                    } else if self.rocks[slot].owner == ACTOR_PLAYER {
                        self.score += 1;
                    } else if self.rocks[slot].owner == ACTOR_THIEF {
                        self.thief_score += 1;
                    }
                    ctx.av.play_sfx(self.sfx[S_RIMPACT], -1);
                    self.rocks[slot].state = SLOT_HIT;
                    self.lava[lava_slot] = SLOT_HIT;
                    set_actor_anim(ctx, rock_id, self.anims[A_ROCK_IMPACT], true, STATE_RESTART_FORWARD);
                    set_actor_anim(ctx, lava_id, self.anims[A_LAVABALL_EXPLODE], true, STATE_RESTART_FORWARD);
                    break;
                }
                if self.ptero_respawn == 0 && self.ptero_stun == 0 && ctx.actors.overlaps(&ctx.av, rock_id, ACTOR_PTERO) {
                    ctx.av.play_sfx(self.sfx[S_TERAHIT], -1);
                    if self.rocks[slot].owner == ACTOR_PLAYER {
                        self.score += 3;
                    } else if self.rocks[slot].owner == ACTOR_THIEF {
                        self.thief_score += 3;
                    }
                    self.rocks[slot].state = SLOT_HIT;
                    set_actor_anim(ctx, rock_id, self.anims[A_ROCK_IMPACT], true, STATE_RESTART_FORWARD);
                    set_actor_anim(ctx, ACTOR_PTERO, self.anims[A_TERRA_FLY_HIT], false, STATE_RESTART_FORWARD);
                    self.ptero_stun = 4;
                }
            }
        }
        if old_score != self.score {
            let text = format!("{}", self.score);
            ctx.scene.set_text_by_id(OBJ_SCORE, &text);
            self.commentary_counter += 1;
            if self.commentary_counter > 10 {
                self.commentary_counter = 0;
                if self.capn_state == CAPN_TV_IDLE {
                    set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAP_TV_006], false, STATE_RESTART_FORWARD);
                    if ctx.rand_range(0, 2) == 0 {
                        self.voice = ctx.av.play_stream_sound(&mut ctx.host, "052.wav", false, self.voice).await;
                    } else {
                        self.voice = ctx.av.play_stream_sound(&mut ctx.host, "053.wav", false, self.voice).await;
                    }
                    self.capn_state = CAPN_TV_TALKING;
                }
            }
        }
        if self.challenge_phase != PHASE_NORMAL && old_thief_score != self.thief_score {
            let text = format!("{}", self.thief_score);
            ctx.scene.set_text_by_id(OBJ_TIME, &text);
        }
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_MAIN);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_CAPN);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, LAYER_TV);
        ctx.scene.draw_layer(&mut ctx.av, 1);
        ctx.cursor.draw(&mut ctx.av);
        ctx.scene.advance_anims(&ctx.av);
        if ctx.av.is_key_down(KEY_ESC) {
            ret = STATE_HOME_WORLD;
        }
        if self.challenge_phase != PHASE_NORMAL && self.player_state == PS_GAME_OVER {
            self.exit_frames += 1;
            if self.exit_frames > 0x78 {
                ctx.player.challenge_won = (self.outcome == OUTCOME_WIN) as u8;
                ret = STATE_CHALLENGE;
            }
        }
        ret
    }

    // Steps the first time tutorial cutscene through lightning, juricar, Cap'n talk and demo throw
    async fn advance_intro<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        match self.intro_step {
            0 => {
                ctx.actors.set_visible(ACTOR_PLAYER, false);
                ctx.av.play_sfx(self.sfx[S_LIGHTNING], -1);
                ctx.actors.set_visible(ACTOR_LIGHTNING, true);
                set_actor_anim(ctx, ACTOR_LIGHTNING, self.anims[A_LIGHTNING], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(ACTOR_LIGHTNING, 220.0, -20.0);
                ctx.actors.set_timer(ACTOR_LIGHTNING, 0x1e);
                self.intro_step += 1;
            }
            1 => {
                ctx.actors.set_visible(ACTOR_JURICAR, true);
                ctx.av.play_sfx(self.sfx[S_FADEIN], -1);
                set_actor_anim(ctx, ACTOR_JURICAR, self.anims[A_JURICAR_ZAP], true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(ACTOR_JURICAR, 140.0, 170.0);
                self.intro_step += 1;
            }
            2 => {
                ctx.actors.set_visible(ACTOR_LIGHTNING, false);
                set_actor_anim(ctx, ACTOR_JURICAR, self.anims[A_JURICAR], false, STATE_RESTART_FORWARD);
                ctx.actors.set_visible(ACTOR_CAPN, true);
                set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAPN_029], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(ACTOR_CAPN, 300.0, 200.0);
                ctx.actors.face(&ctx.av, ACTOR_CAPN, FACING_NONE);
                ctx.actors.set_timer(ACTOR_CAPN, 0x96);
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, "039.wav", false, self.voice).await;
                self.intro_waiting_voice = true;
                self.capn_state = CAPN_INTRO_TALK;
                self.intro_step += 1;
            }
            3 => {
                ctx.actors.set_layer_mask(ACTOR_CAPN, LAYER_CAPN);
                ctx.actors.set_pos(ACTOR_CAPN, 300.0, 250.0);
                set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAPN_013], false, STATE_RESTART_FORWARD);
                self.intro_step += 1;
            }
            4 => {
                set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAPN_037], true, STATE_RESTART_FORWARD);
                self.capn_state = CAPN_DEMO_THROW;
                self.intro_step += 1;
            }
            5 => {
                set_actor_anim(ctx, ACTOR_CAPN, self.anims[A_CAPN_013], false, STATE_RESTART_FORWARD);
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, "040.wav", false, self.voice).await;
                self.intro_waiting_voice = true;
                self.capn_state = CAPN_INTRO_END_TALK;
                self.intro_step += 1;
            }
            6 => {
                ctx.actors.set_visible(ACTOR_PLAYER, false);
                ctx.actors.set_visible(ACTOR_CAPN, false);
                ctx.av.play_sfx(self.sfx[S_FADEIN], -1);
                set_actor_anim(ctx, ACTOR_JURICAR, self.anims[A_JURICAR_ZAP], true, STATE_RESTART_BACKWARD);
                self.intro_step += 1;
            }
            7 => {
                ctx.actors.set_visible(ACTOR_JURICAR, false);
                ctx.actors.set_visible(ACTOR_LIGHTNING, false);
                ctx.actors.set_visible(ACTOR_PLAYER, true);
                self.intro_active = false;
            }
            _ => {}
        }
    }

    // Launches a rock from the first free slot at the owner's position plus an offset
    fn spawn_rock<H: Host>(&mut self, ctx: &mut Ctx<H>, owner: i32, dx: f32, dy: f32, speed: f32) {
        let mut slot = 0;
        while slot < ROCK_SLOTS && self.rocks[slot].state != SLOT_FREE {
            slot += 1;
        }
        if slot != ROCK_SLOTS {
            let rock_id = ACTOR_ROCK0 + slot as i32;
            let (ox, oy) = ctx.actors.pos(owner);
            self.rocks[slot].y = oy + dy;
            ctx.actors.set_pos(rock_id, ox + dx, self.rocks[slot].y);
            ctx.actors.set_visible(rock_id, true);
            set_actor_anim(ctx, rock_id, self.anims[A_ROCK], false, STATE_RESTART_FORWARD);
            self.rocks[slot].state = SLOT_FLYING;
            self.rocks[slot].speed = speed;
            self.rocks[slot].owner = owner;
        }
    }

    // Picks a random side and starts its eruption when that side is idle
    fn try_erupt<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if ctx.rand_range(0, 2) == 0 {
            if !self.right_erupting {
                ctx.av.play_sfx(self.sfx[S_LAVASHOOT], -1);
                set_actor_anim(ctx, ACTOR_ERUPT_RIGHT, self.anims[A_ERUPTION_RIGHT], true, STATE_RESTART_FORWARD);
                ctx.actors.set_timer(ACTOR_ERUPT_RIGHT, 0xf);
                ctx.actors.set_visible(ACTOR_ERUPT_RIGHT, true);
                self.right_erupting = true;
                self.erupt_cooldown = 0x1e;
            }
        } else if !self.left_erupting {
            ctx.av.play_sfx(self.sfx[S_LAVASHOOT], -1);
            set_actor_anim(ctx, ACTOR_ERUPT_LEFT, self.anims[A_ERUPTION_LEFT], true, STATE_RESTART_FORWARD);
            ctx.actors.set_timer(ACTOR_ERUPT_LEFT, 0xf);
            ctx.actors.set_visible(ACTOR_ERUPT_LEFT, true);
            self.left_erupting = true;
            self.erupt_cooldown = 0x1e;
        }
    }

    // Throws a lavaball from the left or right eruption with random lift and speed
    fn spawn_lavaball<H: Host>(&mut self, ctx: &mut Ctx<H>, side: i32) {
        if self.lava_count < LAVA_SLOTS as u32 {
            let mut slot = 0;
            while slot < LAVA_SLOTS && self.lava[slot] != SLOT_FREE {
                slot += 1;
            }
            if slot != LAVA_SLOTS {
                self.lava[slot] = SLOT_FLYING;
                let lava_id = ACTOR_LAVA0 + slot as i32;
                set_actor_anim(ctx, lava_id, self.anims[A_LAVABALL], false, STATE_RESTART_FORWARD);
                let lift = ctx.rand_range(2, 6);
                ctx.actors.set_gravity(lava_id, (-7 - lift) as f32, LAVA_GRAVITY, 0x14a, 100.0);
                let hspeed = ctx.rand_range(6, 10);
                ctx.actors.set_arc(lava_id, hspeed as f32, 12.0);
                if side == 0 {
                    ctx.actors.set_pos(lava_id, 50.0, 254.0);
                    ctx.actors.walk_to(&ctx.av, lava_id, 700);
                } else {
                    ctx.actors.set_pos(lava_id, 550.0, 254.0);
                    ctx.actors.walk_to(&ctx.av, lava_id, -100);
                }
                ctx.actors.set_visible(lava_id, true);
                self.lava_count += 1;
            }
        }
    }

    // Hides a lavaball actor and releases its slot
    fn free_lavaball<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        ctx.actors.set_visible(actor_id, false);
        let slot = (actor_id - ACTOR_LAVA0) as usize;
        if self.lava[slot] != SLOT_FREE {
            self.lava[slot] = SLOT_FREE;
            self.lava_count -= 1;
        }
    }

    // Done spends food and happiness and trains strength, Again only trains, both leave the screen
    fn on_scene_click(player: &mut PlayerData, id: i32) -> u32 {
        let mut ret = RESULT_CONTINUE;
        if id == OBJ_DONE {
            if player.food != 0 {
                player.food = player.food.wrapping_sub(1);
            }
            if player.happiness != 0 {
                player.happiness = player.happiness.wrapping_sub(1);
            }
            player.add_stats(0, 1, 0);
            ret = STATE_HOME_WORLD;
        } else if id == OBJ_AGAIN {
            player.add_stats(0, 1, 0);
            ret = STATE_THROWING;
        }
        ret
    }

    // Routes actor events to the init, anim end, timer and ground handlers
    async fn on_actor_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, actor_id: i32) {
        match kind {
            EVENT_INIT_OBJECT => self.on_init_object(ctx, actor_id).await,
            EVENT_ANIM_END => self.on_anim_end(ctx, actor_id).await,
            EVENT_TARGET_REACHED
                if !(ACTOR_LAVA0..=ACTOR_LAVA0 + 3).contains(&actor_id) && actor_id == ACTOR_CAPN =>
            {
                self.advance_intro(ctx).await;
            }
            EVENT_TIMER => match actor_id {
                ACTOR_ERUPT_LEFT => self.spawn_lavaball(ctx, 0),
                ACTOR_ERUPT_RIGHT => self.spawn_lavaball(ctx, 1),
                ACTOR_LIGHTNING | ACTOR_CAPN => self.advance_intro(ctx).await,
                _ => {}
            },
            EVENT_GROUND_REACHED if (ACTOR_LAVA0..ACTOR_LAVA0 + 4).contains(&actor_id) => {
                self.free_lavaball(ctx, actor_id);
            }
            _ => {}
        }
    }

    // Gives each actor its starting animation, position and visibility on its INIT_OBJECT event
    async fn on_init_object<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        match actor_id {
            ACTOR_PLAYER => {
                set_actor_anim(ctx, actor_id, self.anims[A_STANCE], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(actor_id, 300.0, 320.0);
                ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
                self.player_state = PS_STANCE;
            }
            ACTOR_THIEF => {
                set_actor_anim(ctx, actor_id, self.anims[A_THIEF_STANCE], false, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(actor_id, 440.0, 232.0);
                ctx.actors.face(&ctx.av, actor_id, FACING_NONE);
                self.thief_state = PS_STANCE;
            }
            2..=5 => {
                set_actor_anim(ctx, actor_id, self.anims[A_ROCK], false, STATE_RESTART_FORWARD);
                ctx.actors.set_visible(actor_id, false);
            }
            6..=9 => {
                set_actor_anim(ctx, actor_id, self.anims[A_LAVABALL], false, STATE_RESTART_FORWARD);
                ctx.actors.set_visible(actor_id, false);
            }
            ACTOR_ERUPT_LEFT => {
                ctx.actors.set_visible(actor_id, false);
                set_actor_anim(ctx, actor_id, self.anims[A_ERUPTION_LEFT], false, STATE_STOPPED);
                ctx.actors.set_pos(actor_id, -22.0, 156.0);
            }
            ACTOR_ERUPT_RIGHT => {
                ctx.actors.set_visible(actor_id, false);
                set_actor_anim(ctx, actor_id, self.anims[A_ERUPTION_RIGHT], false, STATE_STOPPED);
                ctx.actors.set_pos(actor_id, 532.0, 164.0);
            }
            ACTOR_PTERO => {
                set_actor_anim(ctx, actor_id, self.anims[A_TERRA_FLY], false, STATE_RESTART_FORWARD);
                ctx.actors.set_visible(actor_id, false);
            }
            ACTOR_CAPN => {
                ctx.actors.set_visible(actor_id, false);
                if self.intro_active {
                    self.advance_intro(ctx).await;
                }
            }
            ACTOR_JURICAR | ACTOR_LIGHTNING | ACTOR_COUNTDOWN => {
                ctx.actors.set_visible(actor_id, false);
            }
            _ => {}
        }
    }

    // Finishes a reload, clears a spent rock or lavaball, ends an eruption or steps the intro
    async fn on_anim_end<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        match actor_id {
            ACTOR_PLAYER => {
                if self.player_state == PS_THROWING {
                    set_actor_anim(ctx, actor_id, self.anims[A_THROW_RELOAD], true, STATE_RESTART_FORWARD);
                    self.player_state = PS_RELOADING;
                } else {
                    self.player_state = PS_STANCE;
                    set_actor_anim(ctx, actor_id, self.anims[A_STANCE], false, STATE_RESTART_FORWARD);
                }
            }
            ACTOR_THIEF => {
                if self.thief_state == PS_THROWING {
                    self.spawn_rock(ctx, ACTOR_THIEF, 30.0, -30.0, 12.0);
                    set_actor_anim(ctx, actor_id, self.anims[A_THIEF_RELOAD], true, STATE_RESTART_FORWARD);
                    self.thief_state = PS_RELOADING;
                } else {
                    self.thief_state = PS_STANCE;
                }
            }
            2..=5 => {
                self.rocks[(actor_id - ACTOR_ROCK0) as usize].state = SLOT_FREE;
                ctx.actors.set_visible(actor_id, false);
            }
            6..=9 => self.free_lavaball(ctx, actor_id),
            ACTOR_ERUPT_LEFT => {
                ctx.actors.set_visible(actor_id, false);
                self.left_erupting = false;
            }
            ACTOR_ERUPT_RIGHT => {
                ctx.actors.set_visible(actor_id, false);
                self.right_erupting = false;
            }
            ACTOR_CAPN if self.capn_state == CAPN_DEMO_THROW => {
                self.spawn_rock(ctx, ACTOR_CAPN, 17.0, -25.0, 16.0);
            }
            ACTOR_JURICAR => self.advance_intro(ctx).await,
            _ => {}
        }
    }
}

impl<H: Host> Screen<H> for ThrowScreen {
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.init_screen(ctx).await
    }

    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        self.update_screen(ctx).await
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
    fn done_spends_food_and_happiness_and_trains_strength() {
        let mut p = PlayerData { food: 3, happiness: 0, ..Default::default() };
        assert_eq!(ThrowScreen::on_scene_click(&mut p, OBJ_DONE), STATE_HOME_WORLD);
        assert_eq!((p.food, p.happiness, p.strength), (2, 0, 1));
    }

    #[test]
    fn again_trains_strength_and_other_ids_do_nothing() {
        let mut p = PlayerData::default();
        assert_eq!(ThrowScreen::on_scene_click(&mut p, OBJ_AGAIN), STATE_THROWING);
        assert_eq!(p.strength, 1);
        assert_eq!(ThrowScreen::on_scene_click(&mut p, OBJ_SCORE), RESULT_CONTINUE);
        assert_eq!(p.strength, 1);
    }

    #[test]
    fn stage_table_rows_follow_anim_index() {
        let mut p = PlayerData::default();
        let baby = STAGE_TABLE[p.anim_index(false) as usize];
        assert_eq!((baby.draw_off_y, baby.rock_dx, baby.rock_dy), (50, 3, 35));
        p.stage = 1;
        p.kind = 2;
        let teen = STAGE_TABLE[p.anim_index(false) as usize];
        assert_eq!((teen.draw_off_y, teen.rock_dx, teen.rock_dy), (0, 12, 5));
        p.stage = 2;
        let adult = STAGE_TABLE[p.anim_index(false) as usize];
        assert_eq!((adult.draw_off_x, adult.draw_off_y, adult.rock_dx, adult.rock_dy), (0, -31, 0, 0));
    }

    #[test]
    fn lava_gravity_matches_binary_constant() {
        assert_eq!(LAVA_GRAVITY.to_bits(), 0x3ecc_cccd);
        assert_eq!(LAVA_GRAVITY, 0.4f32);
    }
}
