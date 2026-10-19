use avmanager::crt::Rand;

use crate::actors::ActorManager;
use crate::av::Av;
use crate::cursor::Cursor;
use crate::host::Host;
use crate::player::PlayerData;
use crate::scene::Scene;

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
    pub launch_setup: bool,
    pub open_url: bool,
    pub debug_overlay: bool,
}

// Everything a screen touches, the engine plus the shared game objects and the host
pub struct Ctx<H: Host> {
    pub av: Av,
    pub host: H,
    pub scene: Scene,
    pub actors: ActorManager,
    pub cursor: Cursor,
    pub player: PlayerData,
    pub rand: Rand,
    pub globals: Globals,
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
        }
    }

    // The game's RandRange helper, an int in lo..=lo+n with lo+n only at rand() == 32767
    pub fn rand_range(&mut self, lo: i32, n: i32) -> i32 {
        self.rand.ranged(lo, n)
    }

    pub fn time_ms(&self) -> u32 {
        self.host.time_ms()
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
