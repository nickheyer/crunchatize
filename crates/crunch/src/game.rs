use crate::ctx::{
    Ctx, RESULT_CONTINUE, RESULT_QUIT, STATE_BOOT, STATE_CHALLENGE, STATE_CREATE, STATE_CREDITS, STATE_HOME_WORLD,
    STATE_JUMPING, STATE_LOAD_CRUNCH, STATE_RUNNING, STATE_THROWING, STATE_TITLE, Screen,
};
use crate::host::Host;
use crate::player::save_player;
use crate::screens::challenge::ChallengeScreen;
use crate::screens::create::CreateScreen;
use crate::screens::credits::CreditsScreen;
use crate::screens::home_world::HomeWorldScreen;
use crate::screens::jump::JumpScreen;
use crate::screens::load_crunch::LoadCrunchScreen;
use crate::screens::running::RunningScreen;
use crate::screens::throw::ThrowScreen;
use crate::screens::title::TitleScreen;

pub const TICK_INTERVAL_MS: i32 = 0x20;
const LOADING_X: i32 = 0x10c;
const LOADING_Y: i32 = 0xd6;

const HAIR_TABLE: [[u8; 3]; 6] = [
    [0x00, 0x00, 0xff],
    [0xe0, 0x00, 0x78],
    [0xff, 0x00, 0x00],
    [0x20, 0x6c, 0x38],
    [0x80, 0x04, 0xd8],
    [0x70, 0x28, 0x00],
];

const FLESH_TABLE: [[u8; 3]; 6] = [
    [0x10, 0x90, 0xf8],
    [0xf8, 0x94, 0xc8],
    [0xf8, 0x94, 0x20],
    [0x20, 0xe8, 0x20],
    [0xc0, 0x80, 0xf8],
    [0xf8, 0xc8, 0x30],
];

// One of the nine concrete screens, dispatched by game state id
pub enum ScreenBox {
    Title(TitleScreen),
    LoadCrunch(Box<LoadCrunchScreen>),
    Credits(CreditsScreen),
    Create(Box<CreateScreen>),
    HomeWorld(Box<HomeWorldScreen>),
    Running(Box<RunningScreen>),
    Throw(Box<ThrowScreen>),
    Jump(Box<JumpScreen>),
    Challenge(Box<ChallengeScreen>),
}

impl ScreenBox {
    // Allocates the screen for a state id, None for ids with no screen class
    pub fn for_state(state: u32) -> Option<Self> {
        Some(match state {
            STATE_TITLE => Self::Title(TitleScreen::new()),
            STATE_LOAD_CRUNCH => Self::LoadCrunch(Box::new(LoadCrunchScreen::new())),
            STATE_CREDITS => Self::Credits(CreditsScreen::new()),
            STATE_CREATE => Self::Create(Box::default()),
            STATE_HOME_WORLD => Self::HomeWorld(Box::default()),
            STATE_RUNNING => Self::Running(Box::new(RunningScreen::new())),
            STATE_THROWING => Self::Throw(Box::new(ThrowScreen::new())),
            STATE_JUMPING => Self::Jump(Box::new(JumpScreen::new())),
            STATE_CHALLENGE => Self::Challenge(Box::new(ChallengeScreen::new())),
            _ => return None,
        })
    }

    async fn init<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        match self {
            Self::Title(s) => s.init(ctx).await,
            Self::LoadCrunch(s) => s.init(ctx).await,
            Self::Credits(s) => s.init(ctx).await,
            Self::Create(s) => s.init(ctx).await,
            Self::HomeWorld(s) => s.init(ctx).await,
            Self::Running(s) => s.init(ctx).await,
            Self::Throw(s) => s.init(ctx).await,
            Self::Jump(s) => s.init(ctx).await,
            Self::Challenge(s) => s.init(ctx).await,
        }
    }

    async fn update<H: Host>(&mut self, ctx: &mut Ctx<H>) -> u32 {
        match self {
            Self::Title(s) => s.update(ctx).await,
            Self::LoadCrunch(s) => s.update(ctx).await,
            Self::Credits(s) => s.update(ctx).await,
            Self::Create(s) => s.update(ctx).await,
            Self::HomeWorld(s) => s.update(ctx).await,
            Self::Running(s) => s.update(ctx).await,
            Self::Throw(s) => s.update(ctx).await,
            Self::Jump(s) => s.update(ctx).await,
            Self::Challenge(s) => s.update(ctx).await,
        }
    }

    fn shutdown<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        match self {
            Self::Title(s) => s.shutdown(ctx),
            Self::LoadCrunch(s) => s.shutdown(ctx),
            Self::Credits(s) => s.shutdown(ctx),
            Self::Create(s) => s.shutdown(ctx),
            Self::HomeWorld(s) => s.shutdown(ctx),
            Self::Running(s) => s.shutdown(ctx),
            Self::Throw(s) => s.shutdown(ctx),
            Self::Jump(s) => s.shutdown(ctx),
            Self::Challenge(s) => s.shutdown(ctx),
        }
    }
}

// The state machine that owns the current screen
pub struct Game {
    pub state: u32,
    pub screen: Option<ScreenBox>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self { state: STATE_BOOT, screen: None }
    }

    pub fn destroy_screen<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        if let Some(mut s) = self.screen.take() {
            s.shutdown(ctx);
        }
    }

    // Runs one tick and reports whether the game keeps running
    pub async fn tick<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        if self.state == STATE_BOOT {
            return self.switch_state(ctx, STATE_TITLE).await;
        }
        let result = match self.screen.as_mut() {
            Some(s) => s.update(ctx).await,
            None => 0,
        };
        match result {
            0 | RESULT_QUIT => false,
            RESULT_CONTINUE => true,
            next => self.switch_state(ctx, next).await,
        }
    }

    // Tears down the old screen, shows the Loading art for one frame, and starts the new screen
    pub async fn switch_state<H: Host>(&mut self, ctx: &mut Ctx<H>, state: u32) -> bool {
        if self.screen.is_some() {
            self.destroy_screen(ctx);
            ctx.av.release_all();
        }
        ctx.av.clear_buffer(0, 0, 0);
        ctx.av.swap_buffers(&mut ctx.host).await;
        ctx.av.clear_buffer(0, 0, 0);
        if let Some(h) = ctx.av.get_anim(&mut ctx.host, "Loading").await {
            ctx.av.draw_anim_reset(h, LOADING_X, LOADING_Y);
        }
        ctx.av.swap_buffers(&mut ctx.host).await;
        let Some(mut screen) = ScreenBox::for_state(state) else {
            return false;
        };
        self.state = state;
        let ok = screen.init(ctx).await;
        self.screen = Some(screen);
        ok
    }
}

// Index of a colour in a six entry table, 6 when absent
fn color_index(table: &[[u8; 3]; 6], rgb: [u8; 3]) -> usize {
    table.iter().position(|&c| c == rgb).unwrap_or(6)
}

// The connect.php address opened when the player chose to go online
pub fn online_url<H: Host>(ctx: &Ctx<H>) -> String {
    let p = &ctx.player;
    format!(
        "http://www.capncrunch.com/games_online/connect.php?name={}&stage={}&hair={}&flesh={}&type={}&strength={}&jumping={}&running={}",
        p.name_str(),
        p.stage,
        color_index(&HAIR_TABLE, p.hair),
        color_index(&FLESH_TABLE, p.flesh),
        p.kind,
        p.strength,
        p.jumping,
        p.running
    )
}

// Draws the F1 statistics overlay the original rendered with GDI text
fn draw_debug_overlay<H: Host>(ctx: &mut Ctx<H>, delta_ms: i32) {
    let fps = if delta_ms != 0 { 1000 / delta_ms } else { 0 };
    let text = format!("FPS: {fps}");
    ctx.av.display.gdi_text(0, 0, &text);
    let bitmaps = format!("Bitmaps:{:11} ({:4})", ctx.av.bitmaps.ram_usage(), ctx.av.bitmaps.total());
    ctx.av.display.gdi_text(0, 0x10, &bitmaps);
    let sound = format!("  Sound:{:11} ({:4})", ctx.av.sound.sound_ram_usage(), ctx.av.sound.total_sounds_loaded());
    ctx.av.display.gdi_text(0, 0x20, &sound);
}

// Runs the whole game from boot to quit, performs the exit actions, and hands the host back
pub async fn run<H: Host>(host: H, out_rate: u32) -> H {
    let mut ctx = Ctx::new(host, out_rate);
    let seed = ctx.host.unix_time();
    ctx.rand.srand(seed);
    let mut game = Game::new();
    let mut last_tick: u32 = 0;
    loop {
        if ctx.host.quit_requested() {
            break;
        }
        let now = ctx.host.time_ms();
        let delta = now.wrapping_sub(last_tick) as i32;
        if delta > TICK_INTERVAL_MS {
            last_tick = now;
            let device = ctx.host.poll_input();
            if ctx.host.take_debug_toggle() {
                ctx.globals.debug_overlay = !ctx.globals.debug_overlay;
            }
            ctx.av.input.heartbeat(&device);
            ctx.av.sound.heartbeat();
            if !game.tick(&mut ctx).await {
                break;
            }
            if ctx.globals.debug_overlay {
                draw_debug_overlay(&mut ctx, delta);
            }
            ctx.av.swap_buffers(&mut ctx.host).await;
        } else {
            ctx.host.end_frame(None, &mut ctx.av.sound).await;
        }
    }
    game.destroy_screen(&mut ctx);
    ctx.av.release_all();
    if ctx.globals.player_loaded {
        let mut name = ctx.globals.save_name.clone();
        save_player(&mut ctx.host, &ctx.player, &mut name);
        ctx.globals.save_name = name;
    }
    if ctx.globals.launch_setup {
        ctx.host.launch_setup();
    } else if ctx.globals.open_url {
        let url = online_url(&ctx);
        ctx.host.open_url(&url);
    }
    ctx.host
}
