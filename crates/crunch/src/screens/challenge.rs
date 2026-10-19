use avmanager::anim::STATE_RESTART_FORWARD;
use avmanager::texture::Palette24;

use crate::actors::{EVENT_ANIM_END, EVENT_INIT_OBJECT, EVENT_TIMER, FACING_NONE};
use crate::ctx::{
    Ctx, KEY_ESC, KEY_SPACE, RESULT_CONTINUE, STATE_HOME_WORLD, STATE_JUMPING, STATE_RUNNING, STATE_THROWING, Screen,
};
use crate::host::Host;
use crate::video::play_video_modal;

const ANIM_STANCE: usize = 0;
const ANIM_CAPN_IDLE: usize = 1;
const ANIM_CAPN_TALK_A: usize = 2;
const ANIM_CAPN_TALK_B: usize = 3;
const ANIM_THIEF_STANCE: usize = 4;
const ANIM_THIEF_TALK: usize = 5;

const ACTOR_CRUNCHLING: i32 = 0;
const ACTOR_CAPN: i32 = 1;
const ACTOR_THIEF: i32 = 2;

const GAME_RUNNING: i32 = 0;
const GAME_JUMP: i32 = 1;
const GAME_THROW: i32 = 2;

const SPEAKER_DELAY: i32 = 0x32;
const TALK_ANIM_DELAY: i16 = 0xf;

const THIEF_GROWL: &str = "theif growley3.wav";

// Challenge dialogue, state 0xd, where the Cap'n and the thief pick the next mini game
#[derive(Clone, Debug, Default)]
pub struct ChallengeScreen {
    anims: [u16; 6],
    cursor_anim: u16,
    music: u32,
    voice: u32,
    dialogue_state: i32,
    picked_game: i32,
    after_loss: bool,
}

// State id of the picked mini game, none for the degenerate top value of the random pick
fn game_state(picked: i32) -> Option<u32> {
    match picked {
        GAME_RUNNING => Some(STATE_RUNNING),
        GAME_JUMP => Some(STATE_JUMPING),
        GAME_THROW => Some(STATE_THROWING),
        _ => None,
    }
}

impl ChallengeScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Advances the dialogue on INIT_OBJECT and TIMER events, ANIM_END is ignored
    async fn on_actor_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, actor: i32) {
        if kind == EVENT_INIT_OBJECT {
            self.init_actor(ctx, actor);
        } else if kind == EVENT_ANIM_END {
            self.on_anim_end();
        } else if kind == EVENT_TIMER {
            if actor == ACTOR_CAPN {
                if self.dialogue_state == 0 {
                    let talk = self.anims[ANIM_CAPN_TALK_A];
                    ctx.actors.set_anims_delayed(
                        &ctx.av,
                        ACTOR_CAPN,
                        TALK_ANIM_DELAY,
                        talk,
                        talk,
                        false,
                        STATE_RESTART_FORWARD,
                    );
                    self.voice = ctx.av.play_stream_sound(&mut ctx.host, "045.wav", false, self.voice).await;
                    self.dialogue_state = 1;
                } else if self.dialogue_state == 3 {
                    if !self.after_loss {
                        let talk = self.anims[ANIM_CAPN_TALK_A];
                        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
                        if self.picked_game == GAME_RUNNING {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "046.wav", false, self.voice).await;
                        } else if self.picked_game == GAME_JUMP {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "047.wav", false, self.voice).await;
                        } else if self.picked_game == GAME_THROW {
                            self.voice = ctx.av.play_stream_sound(&mut ctx.host, "048.wav", false, self.voice).await;
                        }
                    } else {
                        let talk = self.anims[ANIM_CAPN_TALK_B];
                        ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
                        self.voice = ctx.av.play_stream_sound(&mut ctx.host, "050.wav", false, self.voice).await;
                    }
                    self.dialogue_state = 4;
                } else if self.dialogue_state == 6 {
                    self.dialogue_state = 7;
                }
            } else if actor == ACTOR_THIEF {
                let talk = self.anims[ANIM_THIEF_TALK];
                ctx.actors.set_anims(&ctx.av, ACTOR_THIEF, talk, talk, false, STATE_RESTART_FORWARD);
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, THIEF_GROWL, false, self.voice).await;
                self.dialogue_state = 2;
            }
        }
    }

    // Poses the crunchling, Cap'n and thief and starts the first speaker's timer
    fn init_actor<H: Host>(&mut self, ctx: &mut Ctx<H>, actor: i32) {
        if actor == ACTOR_CRUNCHLING {
            let stance = self.anims[ANIM_STANCE];
            ctx.actors.set_anims(&ctx.av, ACTOR_CRUNCHLING, stance, stance, false, STATE_RESTART_FORWARD);
            if ctx.player.stage == 0 {
                ctx.actors.set_pos(ACTOR_CRUNCHLING, 110.0, 320.0);
            } else if ctx.player.stage == 1 {
                ctx.actors.set_pos(ACTOR_CRUNCHLING, 100.0, 300.0);
            } else if ctx.player.stage == 2 {
                ctx.actors.set_pos(ACTOR_CRUNCHLING, 80.0, 270.0);
            }
            ctx.actors.face(&ctx.av, ACTOR_CRUNCHLING, FACING_NONE);
        } else if actor == ACTOR_CAPN {
            let idle = self.anims[ANIM_CAPN_IDLE];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
            ctx.actors.set_pos(ACTOR_CAPN, 180.0, 220.0);
            ctx.actors.face(&ctx.av, ACTOR_CAPN, FACING_NONE);
            if !self.after_loss {
                ctx.actors.set_timer(ACTOR_CAPN, SPEAKER_DELAY);
            }
        } else if actor == ACTOR_THIEF {
            let stance = self.anims[ANIM_THIEF_STANCE];
            ctx.actors.set_anims(&ctx.av, ACTOR_THIEF, stance, stance, false, STATE_RESTART_FORWARD);
            ctx.actors.set_pos(ACTOR_THIEF, 361.0, 180.0);
            ctx.actors.face(&ctx.av, ACTOR_THIEF, FACING_NONE);
            if self.after_loss {
                ctx.actors.set_timer(ACTOR_THIEF, SPEAKER_DELAY);
            }
        }
    }

    // Leaves every actor untouched, no animation on this screen asks for an end notice
    fn on_anim_end(&self) {}
}

impl<H: Host> Screen<H> for ChallengeScreen {
    // Loads the dialogue scene and cast, starts the music and picks the mini game
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "challenge").await;
        let Some(mouse) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        self.cursor_anim = mouse;
        ctx.cursor.set_anim(&ctx.av, self.cursor_anim);
        ctx.cursor.set_pos(0x140, 400);
        ctx.cursor.set_visible(false);
        ctx.actors.clear();
        ctx.actors.add_actor(ACTOR_CAPN, true);
        ctx.actors.add_actor(ACTOR_CRUNCHLING, true);
        ctx.actors.add_actor(ACTOR_THIEF, true);
        let name = ctx.player.anim_name("stance", false, false);
        let Some(stance) = ctx.av.get_anim_retain_image(&mut ctx.host, &name).await else { return false };
        self.anims[ANIM_STANCE] = stance;
        let mut pal = Palette24::default();
        ctx.av.get_anim_palette(stance, &mut pal);
        ctx.player.recolor_palette(&mut pal);
        ctx.av.set_anim_palette(stance, &pal);
        let shared = [
            (ANIM_CAPN_IDLE, "capn_cap008"),
            (ANIM_CAPN_TALK_A, "capn_cap013"),
            (ANIM_CAPN_TALK_B, "capn_cap013"),
            (ANIM_THIEF_STANCE, "thief_challenge_stance"),
            (ANIM_THIEF_TALK, "thief_challenge_talk"),
        ];
        for (slot, name) in shared {
            let Some(h) = ctx.av.get_anim(&mut ctx.host, name).await else { return false };
            self.anims[slot] = h;
        }
        self.music = ctx.av.play_stream_sound(&mut ctx.host, "ps57challenge.wav", true, 0).await;
        self.voice = 0;
        self.picked_game = ctx.rand_range(0, 3);
        self.dialogue_state = 0;
        if ctx.player.challenge_active == 0 {
            self.after_loss = false;
        } else {
            self.after_loss = ctx.player.challenge_won == 0;
        }
        true
    }

    // Plays the outro after a win, steps the dialogue, runs the actors and draws the scene
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut ret = RESULT_CONTINUE;
        if ctx.player.challenge_active != 0 && ctx.player.challenge_won != 0 {
            play_video_modal(&mut ctx.av, &mut ctx.host, "Outro.smk").await;
            ret = STATE_HOME_WORLD;
        }
        if self.dialogue_state == 7 || ctx.av.is_key_down(KEY_SPACE) {
            if let Some(state) = game_state(self.picked_game) {
                ret = state;
            }
            if ctx.player.challenge_active == 0 {
                ctx.player.challenge_active = 1;
            } else {
                ret = STATE_HOME_WORLD;
            }
        } else if self.dialogue_state == 1 {
            if ctx.av.is_stream_sound_done(self.voice) {
                let idle = self.anims[ANIM_CAPN_IDLE];
                ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
                let talk = self.anims[ANIM_THIEF_TALK];
                ctx.actors.set_anims(&ctx.av, ACTOR_THIEF, talk, talk, false, STATE_RESTART_FORWARD);
                self.voice = ctx.av.play_stream_sound(&mut ctx.host, THIEF_GROWL, false, self.voice).await;
                self.dialogue_state = 2;
            }
        } else if self.dialogue_state == 2 {
            if ctx.av.is_stream_sound_done(self.voice) {
                let stance = self.anims[ANIM_THIEF_STANCE];
                ctx.actors.set_anims(&ctx.av, ACTOR_THIEF, stance, stance, false, STATE_RESTART_FORWARD);
                ctx.actors.set_timer(ACTOR_CAPN, SPEAKER_DELAY);
                self.dialogue_state = 3;
            }
        } else if self.dialogue_state == 4 {
            if ctx.av.is_stream_sound_done(self.voice) {
                if !self.after_loss {
                    let talk = self.anims[ANIM_CAPN_TALK_B];
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
                    if self.picked_game == GAME_RUNNING {
                        self.voice = ctx.av.play_stream_sound(&mut ctx.host, "046b.wav", false, self.voice).await;
                    } else if self.picked_game == GAME_JUMP {
                        self.voice = ctx.av.play_stream_sound(&mut ctx.host, "047b.wav", false, self.voice).await;
                    } else if self.picked_game == GAME_THROW {
                        self.voice = ctx.av.play_stream_sound(&mut ctx.host, "048b.wav", false, self.voice).await;
                    }
                } else {
                    let talk = self.anims[ANIM_CAPN_TALK_A];
                    ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, talk, talk, false, STATE_RESTART_FORWARD);
                    self.voice = ctx.av.play_stream_sound(&mut ctx.host, "051.wav", false, self.voice).await;
                }
                self.dialogue_state = 5;
            }
        } else if self.dialogue_state == 5 && ctx.av.is_stream_sound_done(self.voice) {
            let idle = self.anims[ANIM_CAPN_IDLE];
            ctx.actors.set_anims(&ctx.av, ACTOR_CAPN, idle, idle, false, STATE_RESTART_FORWARD);
            ctx.actors.set_timer(ACTOR_CAPN, SPEAKER_DELAY);
            self.dialogue_state = 6;
        }
        ctx.actors.update(&ctx.av);
        while let Some(ev) = ctx.actors.pop_event() {
            self.on_actor_event(ctx, ev.kind, ev.actor_id).await;
        }
        ctx.scene.draw_layer(&mut ctx.av, 0);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 1);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 4);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 8);
        ctx.scene.draw_layer(&mut ctx.av, 1);
        ctx.cursor.draw(&mut ctx.av);
        ctx.scene.advance_anims(&ctx.av);
        if ctx.av.is_key_down(KEY_ESC) {
            ret = STATE_HOME_WORLD;
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
    fn picked_game_maps_to_states() {
        assert_eq!(game_state(0), Some(STATE_RUNNING));
        assert_eq!(game_state(1), Some(STATE_JUMPING));
        assert_eq!(game_state(2), Some(STATE_THROWING));
        assert_eq!(game_state(3), None);
    }
}
