use avmanager::crt::Rand;

use crate::actors::ActorManager;
use crate::av::Av;
use crate::cursor::Cursor;
use crate::host::Host;
use crate::online::Session;
use crate::player::PlayerData;
use crate::scene::Scene;

// Space, right and left as one side of a match holds them this tick
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub space: bool,
    pub right: bool,
    pub left: bool,
}

pub const STATE_BOOT: u32 = 3;
pub const STATE_TITLE: u32 = 4;
pub const STATE_LOAD_CRUNCH: u32 = 5;
pub const STATE_CREDITS: u32 = 6;
pub const STATE_CREATE: u32 = 7;
pub const STATE_HOME_WORLD: u32 = 9;
pub const STATE_RUNNING: u32 = 10;
pub const STATE_THROWING: u32 = 0xb;
pub const STATE_JUMPING: u32 = 0xc;
pub const STATE_CHALLENGE: u32 = 0xd;

pub const RESULT_QUIT: u32 = 2;
pub const RESULT_CONTINUE: u32 = 1;

pub const KEY_ESC: i32 = 1;
pub const KEY_BACKSPACE: i32 = 0x0e;
pub const KEY_LCTRL: i32 = 0x1d;
pub const KEY_LSHIFT: i32 = 0x2a;
pub const KEY_RSHIFT: i32 = 0x36;
pub const KEY_SPACE: i32 = 0x39;
pub const KEY_RCTRL: i32 = 0x57;
pub const KEY_UP: i32 = 0x5c;
pub const KEY_LEFT: i32 = 0x5e;
pub const KEY_RIGHT: i32 = 0x5f;
pub const KEY_DOWN: i32 = 0x61;
pub const KEY_DEL: i32 = 0x64;

pub const MOUSE_LEFT: usize = 0;

// Process wide flags the original kept as static variables
#[derive(Clone, Debug, Default)]
pub struct Globals {
    pub player_loaded: bool,
    pub save_name: String,
    pub debug_overlay: bool,
}

// Everything a screen touches, the engine plus the shared game objects, the host and the online session
pub struct Ctx<H: Host> {
    pub av: Av,
    pub host: H,
    pub scene: Scene,
    pub actors: ActorManager,
    pub cursor: Cursor,
    pub player: PlayerData,
    pub rand: Rand,
    pub globals: Globals,
    pub session: Session,
}

impl<H: Host> Ctx<H> {
    pub fn new(host: H, out_rate: u32) -> Self {
        Self {
            av: Av::new(out_rate),
            host,
            scene: Scene::new(),
            actors: ActorManager::default(),
            cursor: Cursor::default(),
            player: PlayerData::default(),
            rand: Rand::default(),
            globals: Globals::default(),
            session: Session::new(),
        }
    }

    // The game's RandRange helper, an int in lo..=lo+n with lo+n only at rand() == 32767
    pub fn rand_range(&mut self, lo: i32, n: i32) -> i32 {
        self.rand.ranged(lo, n)
    }

    // Clock the simulation sees, frozen for the whole tick and taken from the host's record when replaying
    pub fn time_ms(&self) -> u32 {
        self.session.time_ms(self.host.time_ms())
    }

    // True while a challenge pits the resident crunchling against the visiting friend's
    pub fn versus(&self) -> bool {
        self.player.challenge_active != 0 && self.session.guest().is_some()
    }

    // True while the friend holds a key, always false without a friend in the world
    pub fn helper_key_down(&self, code: i32) -> bool {
        self.session.helper_present() && self.session.helper_keys.is_key_down(code)
    }

    // The race keys one side holds, the friend's relayed keyboard or the host's own
    pub fn held(&self, friend: bool) -> Held {
        if friend {
            Held {
                space: self.helper_key_down(KEY_SPACE),
                right: self.helper_key_down(KEY_RIGHT),
                left: self.helper_key_down(KEY_LEFT),
            }
        } else {
            Held {
                space: self.av.is_key_down(KEY_SPACE),
                right: self.av.is_key_down(KEY_RIGHT),
                left: self.av.is_key_down(KEY_LEFT),
            }
        }
    }

    // Whether a streamed voice finished, answered from the host's record when replaying
    pub fn stream_done(&mut self, channel: u32) -> bool {
        if let Some(done) = self.session.replay_bool() {
            if done {
                self.av.kill_streaming_channel(channel);
            }
            return done;
        }
        let done = self.av.is_stream_sound_done(channel);
        self.session.record_bool(done);
        done
    }
}

// One game state's behaviour, matching the virtual Update, Init and Shutdown slots
pub trait Screen<H: Host> {
    // Loads the screen's assets and objects, false aborts the whole game
    fn init(&mut self, ctx: &mut Ctx<H>) -> impl Future<Output = bool>;

    // Runs one tick and returns 1 to stay, 0 or 2 to quit, or the next state id
    fn update(&mut self, ctx: &mut Ctx<H>) -> impl Future<Output = u32>;

    // Stops streaming sounds the screen owns before it is destroyed
    fn shutdown(&mut self, ctx: &mut Ctx<H>);
}
