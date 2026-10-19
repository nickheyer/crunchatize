use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use avmanager::input::DeviceState;
use avmanager::sound::Sound;
use crunch::ctx::{
    Ctx, STATE_CHALLENGE, STATE_CREATE, STATE_CREDITS, STATE_HOME_WORLD, STATE_JUMPING, STATE_LOAD_CRUNCH,
    STATE_RUNNING, STATE_THROWING, STATE_TITLE,
};
use crunch::game::Game;
use crunch::host::Host;
use crunch::player::{PlayerData, encode_save};

const DATA_DIR: &str = "/home/nick/code/crunchatize/dev/disc/iso/CrunchData";
const TICK_MS: u32 = 33;

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(fut: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

// Disc backed host with a virtual clock, in memory saves and a count of lit frames
struct SmokeHost {
    files: HashMap<String, std::path::PathBuf>,
    saves: Vec<(String, Vec<u8>)>,
    clock: u32,
    lit_frames: usize,
    frames: usize,
}

impl SmokeHost {
    fn new(dir: &Path) -> Self {
        let mut files = HashMap::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            files.insert(e.file_name().to_string_lossy().to_lowercase(), e.path());
        }
        Self { files, saves: Vec::new(), clock: 0, lit_frames: 0, frames: 0 }
    }
}

impl Host for SmokeHost {
    async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.files.get(&name.to_lowercase())?).ok()
    }

    fn time_ms(&self) -> u32 {
        self.clock
    }

    fn unix_time(&self) -> u32 {
        7
    }

    fn poll_input(&mut self) -> DeviceState {
        DeviceState::new()
    }

    async fn end_frame(&mut self, frame: Option<&[u16]>, sound: &mut Sound) {
        if let Some(f) = frame {
            self.frames += 1;
            if f.iter().any(|&p| p != 0) {
                self.lit_frames += 1;
            }
        }
        let mut buf = vec![0i16; 44100 * TICK_MS as usize / 1000 * 2];
        sound.render(&mut buf);
        self.clock = self.clock.wrapping_add(TICK_MS);
    }

    fn list_saves(&mut self) -> Vec<String> {
        self.saves.iter().map(|(n, _)| n.clone()).collect()
    }

    fn read_save(&mut self, name: &str) -> Option<Vec<u8>> {
        self.saves.iter().find(|(n, _)| n == name).map(|(_, d)| d.clone())
    }

    fn write_save(&mut self, name: &str, data: &[u8]) -> bool {
        self.saves.retain(|(n, _)| n != name);
        self.saves.push((name.to_string(), data.to_vec()));
        true
    }

    fn save_exists(&mut self, name: &str) -> bool {
        self.saves.iter().any(|(n, _)| n == name)
    }

    fn delete_save(&mut self, name: &str) {
        self.saves.retain(|(n, _)| n != name);
    }

    fn take_debug_toggle(&mut self) -> bool {
        false
    }

    fn quit_requested(&self) -> bool {
        false
    }

    fn open_url(&mut self, _url: &str) {}

    fn show_credits(&mut self, _shown: bool) -> bool {
        false
    }

    fn log(&mut self, message: &str) {
        eprintln!("{message}");
    }
}

fn player(stage: u16, kind: i16) -> PlayerData {
    let mut p = PlayerData::default();
    p.set_name("spaz");
    p.stage = stage;
    p.kind = kind;
    p.flesh = [0xf8, 0xc8, 0x30];
    p.hair = [0x70, 0x28, 0x00];
    p.run_best_secs = 100;
    p.jump_best_secs = 100;
    p.throw_high_score = 10;
    p.lagging_stat = -1;
    p
}

// Boots a state and runs ticks, returning the number of frames with any lit pixel
fn boot_and_run(state: u32, p: PlayerData, ticks: usize) -> usize {
    let dir = Path::new(DATA_DIR);
    let mut host = SmokeHost::new(dir);
    host.saves.push(("Player_1.crn".to_string(), encode_save(&p)));
    let mut ctx = Ctx::new(host, 44100);
    ctx.player = p;
    let mut game = Game::new();
    assert!(block_on(game.switch_state(&mut ctx, state)), "state {state:#x} failed to initialise");
    for _ in 0..ticks {
        let device = ctx.host.poll_input();
        ctx.av.input.heartbeat(&device, (ctx.cursor.x, ctx.cursor.y));
        ctx.av.sound.heartbeat();
        assert!(block_on(game.tick(&mut ctx)), "state {state:#x} asked to quit");
        block_on(ctx.av.swap_buffers(&mut ctx.host));
    }
    ctx.host.lit_frames
}

#[test]
fn every_screen_boots_and_runs() {
    if !Path::new(DATA_DIR).is_dir() {
        return;
    }
    for (state, stage) in [
        (STATE_TITLE, 0),
        (STATE_LOAD_CRUNCH, 0),
        (STATE_CREDITS, 0),
        (STATE_CREATE, 0),
        (STATE_HOME_WORLD, 0),
        (STATE_HOME_WORLD, 1),
        (STATE_HOME_WORLD, 2),
        (STATE_RUNNING, 0),
        (STATE_RUNNING, 2),
        (STATE_THROWING, 1),
        (STATE_JUMPING, 0),
        (STATE_JUMPING, 2),
        (STATE_CHALLENGE, 2),
    ] {
        let lit = boot_and_run(state, player(stage, (stage % 3) as i16), 240);
        assert!(lit > 200, "state {state:#x} stage {stage} drew only {lit} lit frames");
    }
}

// The three challenge variants reach their games and the win path plays the outro
#[test]
fn challenge_variants_run() {
    if !Path::new(DATA_DIR).is_dir() {
        return;
    }
    for state in [STATE_RUNNING, STATE_THROWING, STATE_JUMPING] {
        let mut p = player(2, 0);
        p.challenge_active = 1;
        let lit = boot_and_run(state, p, 240);
        assert!(lit > 200, "challenge state {state:#x} drew only {lit} lit frames");
    }
    let mut p = player(2, 1);
    p.challenge_active = 1;
    p.challenge_won = 1;
    let lit = boot_and_run(STATE_CHALLENGE, p, 30);
    assert!(lit > 0, "won challenge drew nothing");
}
