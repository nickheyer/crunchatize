use avmanager::input::DeviceState;

use crate::ctx::{
    Ctx, RESULT_CONTINUE, RESULT_QUIT, STATE_BOOT, STATE_CHALLENGE, STATE_CREATE, STATE_CREDITS, STATE_HOME_WORLD,
    STATE_JUMPING, STATE_LOAD_CRUNCH, STATE_RUNNING, STATE_THROWING, STATE_TITLE, Screen,
};
use crate::cursor::{Cursor, NO_HANDLE};
use crate::host::Host;
use crate::online::dialog::Dialog;
use crate::online::hash::state_hash;
use crate::online::{Role, session};
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

// Host ticks a lagging helper may catch up on in one frame
const MAX_CATCH_UP_TICKS: usize = 8;
const HELPER_HAND_ART: &str = "mouse_online";

// Opened by the Buy me a coffee button on the save picker
pub const COFFEE_URL: &str = "https://buymeacoffee.com/nickheyer";

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

// The state machine that owns the current screen, the Go Online box and the two online hands
pub struct Game {
    pub state: u32,
    pub screen: Option<ScreenBox>,
    pub dialog: Option<Dialog>,
    hand: Cursor,
    friend_hand: Cursor,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self { state: STATE_BOOT, screen: None, dialog: None, hand: Cursor::default(), friend_hand: Cursor::default() }
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
        self.dialog = None;
        self.hand.anim.anim = NO_HANDLE;
        self.friend_hand.anim.anim = NO_HANDLE;
        ctx.session.on_switched();
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

    // Samples the devices, runs the screen and reports whether the game keeps running
    async fn tick_once<H: Host>(&mut self, ctx: &mut Ctx<H>, device: &DeviceState) -> bool {
        if ctx.host.take_debug_toggle() {
            ctx.globals.debug_overlay = !ctx.globals.debug_overlay;
        }
        ctx.av.input.heartbeat(device, (ctx.cursor.x, ctx.cursor.y));
        ctx.av.sound.heartbeat();
        self.tick(ctx).await
    }

    // A tick of this machine's own world, recorded for the friend while hosting
    async fn live_tick<H: Host>(&mut self, ctx: &mut Ctx<H>, now: u32) -> bool {
        let local = ctx.host.poll_input();
        ctx.session.local.heartbeat(&local, (ctx.cursor.x, ctx.cursor.y));
        let mut device = local;
        if self.dialog.is_some() {
            device.scan_keys = [false; 256];
            device.buttons = [false; 3];
        }
        let device = ctx.session.begin_tick(now, device);
        let alive = self.tick_once(ctx, &device).await;
        let hash = state_hash(ctx, self.state);
        ctx.session.end_tick(&mut ctx.host, hash);
        if !alive {
            return false;
        }
        self.service_dialog(ctx).await;
        true
    }

    // The helper's own input, sent to the host and fed to the box, at the host's tick rate
    async fn local_tick<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let local = ctx.host.poll_input();
        ctx.session.local_hand.apply(&local);
        let pos = ctx.session.local_pos((ctx.cursor.x, ctx.cursor.y));
        ctx.session.local.heartbeat(&local, pos);
        let dialog_open = self.dialog.is_some();
        ctx.session.send_input(&mut ctx.host, &local, dialog_open);
        self.service_dialog(ctx).await;
    }

    // Replays every host tick that arrived, up to the catch up limit, and reports whether any ran
    async fn replay_ticks<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        let mut ran = false;
        for _ in 0..MAX_CATCH_UP_TICKS {
            let Some(record) = ctx.session.next_replay() else { break };
            let device = ctx.session.begin_replay(&record);
            let alive = self.tick_once(ctx, &device).await;
            let hash = state_hash(ctx, self.state);
            ctx.session.end_replay(&mut ctx.host, &record, hash);
            ran = true;
            if !alive {
                session::host_quit(ctx);
                break;
            }
        }
        ran
    }

    // Steps the open box and opens it when a hand clicked Go Online
    async fn service_dialog<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let keep = self.dialog.as_mut().map(|d| d.update(ctx));
        if keep == Some(false) {
            self.dialog = None;
        }
        if ctx.session.take_dialog_request() && self.dialog.is_none() {
            self.dialog = Dialog::open(ctx).await;
            if self.dialog.is_none() {
                ctx.host.log("online: the Go Online box could not be loaded");
            }
        }
    }

    // Picks a hand's art, the screen's chosen tool or the online hat, false while neither is loaded
    async fn dress_hand<H: Host>(hand: &mut Cursor, ctx: &mut Ctx<H>) -> bool {
        let art = ctx.session.helper_art;
        if art != NO_HANDLE {
            hand.set_anim(&ctx.av, art);
        } else if hand.anim.anim == NO_HANDLE
            && let Some(h) = ctx.av.get_anim(&mut ctx.host, HELPER_HAND_ART).await
        {
            hand.set_anim(&ctx.av, h);
        }
        hand.anim.anim != NO_HANDLE
    }

    // Draws the friend's hand on the host, then the box, or the helper's own immediate hand on the helper's machine
    async fn draw_overlay<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let hands_shown = ctx.cursor.visible && ctx.session.helper_present();
        if hands_shown && !ctx.session.local_is_helper() && Self::dress_hand(&mut self.friend_hand, ctx).await {
            self.friend_hand.set_pos(ctx.session.helper.x, ctx.session.helper.y);
            self.friend_hand.set_visible(true);
            self.friend_hand.draw(&mut ctx.av);
        }
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.draw(ctx);
            return;
        }
        if !hands_shown || !ctx.session.local_is_helper() || !Self::dress_hand(&mut self.hand, ctx).await {
            return;
        }
        self.hand.set_pos(ctx.session.local_hand.x, ctx.session.local_hand.y);
        self.hand.set_visible(true);
        self.hand.draw(&mut ctx.av);
    }
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

// The game loop stepped one frame at a time so a host and a helper can run side by side
pub struct Runner<H: Host> {
    pub ctx: Ctx<H>,
    pub game: Game,
    last_tick: u32,
    delta: i32,
    world: Vec<u16>,
}

impl<H: Host> Runner<H> {
    pub fn new(host: H, out_rate: u32) -> Self {
        let mut ctx = Ctx::new(host, out_rate);
        let seed = ctx.host.unix_time();
        ctx.rand.srand(seed);
        Self { ctx, game: Game::new(), last_tick: 0, delta: 0, world: Vec::new() }
    }

    // Runs one frame and reports whether the game keeps running
    pub async fn frame(&mut self) -> bool {
        let ctx = &mut self.ctx;
        if ctx.host.quit_requested() {
            return false;
        }
        session::poll_net(ctx);
        if let Some(state) = ctx.session.take_pending_switch()
            && !self.game.switch_state(ctx, state).await
        {
            return false;
        }
        let now = ctx.host.time_ms();
        let due = now.wrapping_sub(self.last_tick) as i32 > TICK_INTERVAL_MS;
        if ctx.session.role() == Role::Joined {
            if due {
                self.delta = now.wrapping_sub(self.last_tick) as i32;
                self.last_tick = now;
                self.game.local_tick(ctx).await;
            }
            if self.game.replay_ticks(ctx).await {
                self.world.clear();
                self.world.extend_from_slice(ctx.av.display.back());
            } else if self.world.len() == ctx.av.display.back().len() {
                ctx.av.display.back_mut().copy_from_slice(&self.world);
            }
            self.game.draw_overlay(ctx).await;
            if ctx.globals.debug_overlay {
                draw_debug_overlay(ctx, self.delta);
            }
            ctx.av.swap_buffers(&mut ctx.host).await;
        } else if due {
            self.delta = now.wrapping_sub(self.last_tick) as i32;
            self.last_tick = now;
            if !self.game.live_tick(ctx, now).await {
                return false;
            }
            self.game.draw_overlay(ctx).await;
            if ctx.globals.debug_overlay {
                draw_debug_overlay(ctx, self.delta);
            }
            ctx.av.swap_buffers(&mut ctx.host).await;
        } else {
            ctx.host.end_frame(None, &mut ctx.av.sound).await;
        }
        true
    }

    // Tears the game down, says goodbye online, saves the loaded player and hands the host back
    pub fn finish(mut self) -> H {
        let ctx = &mut self.ctx;
        self.game.destroy_screen(ctx);
        ctx.av.release_all();
        session::shutdown(ctx);
        if ctx.globals.player_loaded {
            let mut name = ctx.globals.save_name.clone();
            save_player(&mut ctx.host, &ctx.player, &mut name);
            ctx.globals.save_name = name;
        }
        self.ctx.host
    }
}

// Runs the whole game from boot to quit, saves the loaded player, and hands the host back
pub async fn run<H: Host>(host: H, out_rate: u32) -> H {
    let mut runner = Runner::new(host, out_rate);
    while runner.frame().await {}
    runner.finish()
}
