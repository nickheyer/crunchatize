use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::path::Path;
use std::pin::pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use avmanager::input::DeviceState;
use avmanager::sound::Sound;
use crunch::ctx::STATE_HOME_WORLD;
use crunch::game::Runner;
use crunch::host::{Host, NetEvent};
use crunch::online::dialog::Page;
use crunch::online::hash::state_hash;
use crunch::online::session::CLOSE_HOST_LEFT;
use crunch::online::wire::{self, Message};
use crunch::online::{Notice, Phase};
use crunch::player::PlayerData;

const DATA_DIR: &str = "/home/nick/code/crunchatize/dev/disc/iso/CrunchData";
const ASSETS_DIR: &str = "/home/nick/code/crunchatize/assets";
const FRAME_MS: u32 = 33;
const CODE: &str = "TEST";
const ONLINE_BUTTON: (i16, i16) = (430, 70);
const HOST_PILL: (i16, i16) = (320, 185);
const JOIN_PILL: (i16, i16) = (320, 231);
const QUIT_BUTTON: (i16, i16) = (567, 70);
const SCAN_T: usize = 0x14;
const SCAN_E: usize = 0x12;
const SCAN_S: usize = 0x1f;
const SCAN_RETURN: usize = 0x1c;
const SCAN_SPACE: usize = 0x39;

fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Host,
    Joiner,
}

// An in memory game server joining one host and one joiner with the fixed code
#[derive(Default)]
struct Link {
    host_events: VecDeque<NetEvent>,
    joiner_events: VecDeque<NetEvent>,
    host_up: bool,
    joiner_up: bool,
    host_sent: Vec<Vec<u8>>,
    joiner_sent: Vec<Vec<u8>>,
}

// Script driven host sharing the loopback link, with a virtual clock and in memory saves
struct LoopbackHost {
    side: Side,
    link: Rc<RefCell<Link>>,
    files: HashMap<String, std::path::PathBuf>,
    input: Rc<RefCell<DeviceState>>,
    clock: u32,
    saves: Vec<(String, Vec<u8>)>,
    log: Vec<String>,
}

impl LoopbackHost {
    fn new(side: Side, link: Rc<RefCell<Link>>, input: Rc<RefCell<DeviceState>>) -> Self {
        let mut files = HashMap::new();
        for dir in [DATA_DIR, ASSETS_DIR] {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                files.insert(e.file_name().to_string_lossy().to_lowercase(), e.path());
            }
        }
        Self { side, link, files, input, clock: 0, saves: Vec::new(), log: Vec::new() }
    }
}

impl Host for LoopbackHost {
    async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.files.get(&name.to_lowercase())?).ok()
    }

    fn time_ms(&self) -> u32 {
        self.clock
    }

    fn unix_time(&self) -> u32 {
        if self.side == Side::Host { 11 } else { 22 }
    }

    fn poll_input(&mut self) -> DeviceState {
        self.input.borrow().clone()
    }

    async fn end_frame(&mut self, _frame: Option<&[u16]>, sound: &mut Sound) {
        let mut buf = vec![0i16; 44100 * FRAME_MS as usize / 1000 * 2];
        sound.render(&mut buf);
        self.clock = self.clock.wrapping_add(FRAME_MS);
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
        self.log.push(message.to_string());
    }

    fn net_connect(&mut self, path: &str) {
        let mut link = self.link.borrow_mut();
        match self.side {
            Side::Host => {
                assert_eq!(path, "/online/host");
                link.host_up = true;
                link.host_events.push_back(NetEvent::Open);
                link.host_events.push_back(NetEvent::Message(wire::encode(&Message::Code(CODE.into()))));
            }
            Side::Joiner => {
                assert_eq!(path, format!("/online/join/{CODE}"));
                assert!(link.host_up, "joined before the host opened the room");
                link.joiner_up = true;
                link.joiner_events.push_back(NetEvent::Open);
            }
        }
    }

    fn net_send(&mut self, data: &[u8]) {
        let mut link = self.link.borrow_mut();
        match self.side {
            Side::Host => {
                link.host_sent.push(data.to_vec());
                if link.joiner_up {
                    link.joiner_events.push_back(NetEvent::Message(data.to_vec()));
                }
            }
            Side::Joiner => {
                link.joiner_sent.push(data.to_vec());
                if link.host_up {
                    link.host_events.push_back(NetEvent::Message(data.to_vec()));
                }
            }
        }
    }

    fn net_poll(&mut self) -> Option<NetEvent> {
        let mut link = self.link.borrow_mut();
        match self.side {
            Side::Host => link.host_events.pop_front(),
            Side::Joiner => link.joiner_events.pop_front(),
        }
    }

    fn net_close(&mut self) {
        let mut link = self.link.borrow_mut();
        match self.side {
            Side::Host if link.host_up => {
                link.host_up = false;
                if link.joiner_up {
                    link.joiner_events.push_back(NetEvent::Closed { code: CLOSE_HOST_LEFT, reason: "host left".into() });
                }
            }
            Side::Joiner if link.joiner_up => {
                link.joiner_up = false;
                if link.host_up {
                    link.host_events.push_back(NetEvent::Message(wire::encode(&Message::PeerLeft)));
                }
            }
            _ => {}
        }
    }
}

fn player(name: &str, stage: u16, kind: i16) -> PlayerData {
    let mut p = PlayerData::default();
    p.set_name(name);
    p.stage = stage;
    p.kind = kind;
    p.flesh = [0xf8, 0xc8, 0x30];
    p.hair = [0x70, 0x28, 0x00];
    p.food = 2;
    p.lagging_stat = -1;
    p
}

// One player's runner together with the scripted device its host samples
struct Player {
    runner: Runner<LoopbackHost>,
    input: Rc<RefCell<DeviceState>>,
}

impl Player {
    fn new(side: Side, link: &Rc<RefCell<Link>>, data: PlayerData) -> Self {
        let input = Rc::new(RefCell::new(DeviceState::new()));
        let mut runner = Runner::new(LoopbackHost::new(side, link.clone(), input.clone()), 44100);
        runner.ctx.player = data;
        runner.ctx.globals.player_loaded = true;
        runner.ctx.globals.save_name = format!("{}.crn", if side == Side::Host { "host" } else { "joiner" });
        assert!(block_on(runner.game.switch_state(&mut runner.ctx, STATE_HOME_WORLD)));
        Self { runner, input }
    }

    fn frame(&mut self) -> bool {
        block_on(self.runner.frame())
    }

    fn point(&self, at: (i16, i16), down: bool) {
        let mut input = self.input.borrow_mut();
        input.pointer = Some(at);
        input.buttons = [down, false, false];
    }

    fn keys(&self, keys: &[usize]) {
        let mut input = self.input.borrow_mut();
        input.scan_keys = [false; 256];
        for &k in keys {
            input.scan_keys[k] = true;
        }
    }

    fn phase(&self) -> Phase {
        self.runner.ctx.session.phase().clone()
    }

    fn hash(&self) -> u32 {
        state_hash(&self.runner.ctx, self.runner.game.state)
    }
}

// Steps both players a frame, host first so its records reach the joiner the same frame
fn step(host: &mut Player, joiner: &mut Player) {
    assert!(host.frame(), "host quit unexpectedly");
    assert!(joiner.frame(), "joiner quit unexpectedly");
}

fn steps(host: &mut Player, joiner: &mut Player, n: usize) {
    for _ in 0..n {
        step(host, joiner);
    }
}

// Presses and releases the mouse over a point, leaving the pointer there
fn click(host: &mut Player, joiner: &mut Player, who: Side, at: (i16, i16)) {
    let target = if who == Side::Host { &*host } else { &*joiner };
    target.point(at, false);
    steps(host, joiner, 2);
    let target = if who == Side::Host { &*host } else { &*joiner };
    target.point(at, true);
    steps(host, joiner, 2);
    let target = if who == Side::Host { &*host } else { &*joiner };
    target.point(at, false);
    steps(host, joiner, 2);
}

fn tap(host: &mut Player, joiner: &mut Player, who: Side, key: usize) {
    let target = if who == Side::Host { &*host } else { &*joiner };
    target.keys(&[key]);
    steps(host, joiner, 2);
    let target = if who == Side::Host { &*host } else { &*joiner };
    target.keys(&[]);
    steps(host, joiner, 2);
}

// Centre of a home world object's hit box in the host's scene
fn object_centre(host: &Player, name: &str) -> (i16, i16) {
    let o = host
        .runner
        .ctx
        .scene
        .objects
        .iter()
        .find(|o| o.state != 0 && o.name.iter().take_while(|&&c| c != 0).map(|c| c.to_ascii_lowercase()).eq(name.bytes()))
        .unwrap_or_else(|| panic!("no object {name}"));
    (((o.bbox_x1 + o.bbox_x2) / 2) as i16, ((o.bbox_y1 + o.bbox_y2) / 2) as i16)
}

#[test]
fn host_and_helper_stay_in_lockstep() {
    if !Path::new(DATA_DIR).is_dir() {
        return;
    }
    let link = Rc::new(RefCell::new(Link::default()));
    let mut host = Player::new(Side::Host, &link, player("CAPN", 0, 0));
    let mut joiner = Player::new(Side::Joiner, &link, player("MATE", 1, 2));
    steps(&mut host, &mut joiner, 5);

    // The host opens the box and starts a room
    click(&mut host, &mut joiner, Side::Host, ONLINE_BUTTON);
    assert!(host.runner.game.dialog.is_some(), "the Go Online box did not open for the host");
    click(&mut host, &mut joiner, Side::Host, HOST_PILL);
    steps(&mut host, &mut joiner, 2);
    assert_eq!(host.phase(), Phase::HostWaiting { code: CODE.into() });
    assert_eq!(host.runner.game.dialog.as_ref().map(|d| d.page()), Some(Page::Host));

    // The joiner types the code
    click(&mut host, &mut joiner, Side::Joiner, ONLINE_BUTTON);
    click(&mut host, &mut joiner, Side::Joiner, JOIN_PILL);
    for key in [SCAN_T, SCAN_E, SCAN_S, SCAN_T] {
        tap(&mut host, &mut joiner, Side::Joiner, key);
    }
    assert_eq!(joiner.runner.game.dialog.as_ref().map(|d| d.entry()), Some(CODE));
    tap(&mut host, &mut joiner, Side::Joiner, SCAN_RETURN);
    steps(&mut host, &mut joiner, 6);
    assert_eq!(host.phase(), Phase::HostPlaying { code: CODE.into() });
    assert_eq!(joiner.phase(), Phase::JoinPlaying { code: CODE.into() });
    assert!(host.runner.game.dialog.is_none() && joiner.runner.game.dialog.is_none(), "the box stayed open");
    assert_eq!(joiner.runner.ctx.player.name_str(), "CAPN", "the helper is not playing the host's crunchling");
    assert_eq!(host.runner.ctx.session.epoch(), joiner.runner.ctx.session.epoch());

    // Both hands wander for a while
    for i in 0..90i16 {
        host.point((100 + i * 4, 200 + (i % 7) * 10), i % 13 == 0);
        joiner.point((600 - i * 4, 300 - (i % 5) * 12), i % 11 == 0);
        step(&mut host, &mut joiner);
    }
    assert_eq!(host.hash(), joiner.hash(), "the worlds drifted apart");
    assert_eq!(joiner.runner.ctx.session.desyncs(), 0);
    assert!(joiner.runner.ctx.session.replayed() >= 90);

    // The helper praises the crunchling with the hand tool
    let happiness_before = host.runner.ctx.player.happiness;
    let mut praised = false;
    for _ in 0..200 {
        let crunchling = host.runner.ctx.actors.get(0);
        if crunchling.anim.anim == avmanager::anim::AnimFrame::default().anim || !crunchling.mouse_hittable {
            step(&mut host, &mut joiner);
            continue;
        }
        let b = host.runner.ctx.actors.bbox(&host.runner.ctx.av, 0);
        let visible = host.runner.ctx.cursor.visible;
        let x = ((b[0] + b[2]) / 2) as i16;
        let y = 250i16.clamp((b[1] + 1) as i16, (b[3] - 1) as i16);
        joiner.point((x, y), visible);
        step(&mut host, &mut joiner);
        if host.runner.ctx.player.happiness > happiness_before {
            praised = true;
            break;
        }
    }
    assert!(praised, "the helper's hand could not praise the crunchling");
    joiner.point((600, 400), false);
    steps(&mut host, &mut joiner, 30);
    assert_eq!(host.hash(), joiner.hash(), "the worlds drifted apart after the praise");

    // The host asks the caster for a tip and the helper's space key skips the speech
    let tips = object_centre(&host, "tips");
    click(&mut host, &mut joiner, Side::Host, tips);
    let mut spoke = false;
    for _ in 0..400 {
        step(&mut host, &mut joiner);
        if host.runner.ctx.av.sound.total_sounds_loaded() > 0 && !host.runner.ctx.cursor.visible {
            spoke = true;
            break;
        }
    }
    assert!(spoke, "the caster never spoke");
    steps(&mut host, &mut joiner, 120);
    joiner.keys(&[SCAN_SPACE]);
    steps(&mut host, &mut joiner, 4);
    joiner.keys(&[]);
    let mut cursor_back = false;
    for _ in 0..200 {
        step(&mut host, &mut joiner);
        if host.runner.ctx.cursor.visible {
            cursor_back = true;
            break;
        }
    }
    assert!(cursor_back, "the helper's space key did not end the speech");
    steps(&mut host, &mut joiner, 60);
    assert_eq!(host.hash(), joiner.hash(), "the worlds drifted apart after the speech");
    assert_eq!(joiner.runner.ctx.session.desyncs(), 0);
    assert!(!host.runner.ctx.host.log.iter().any(|l| l.contains("lost sync")), "{:?}", host.runner.ctx.host.log);

    // The host quits, the helper returns to their own crunchling with the box explaining why
    host.point(QUIT_BUTTON, false);
    steps(&mut host, &mut joiner, 2);
    host.point(QUIT_BUTTON, true);
    let mut quit = false;
    for _ in 0..4 {
        if !host.frame() {
            quit = true;
            break;
        }
        assert!(joiner.frame());
    }
    assert!(quit, "the host's quit click was ignored");
    let saved_host = host.runner.finish();
    assert!(saved_host.saves.iter().any(|(n, _)| n == "host.crn"), "the host did not save");
    for _ in 0..6 {
        assert!(joiner.frame());
    }
    assert_eq!(joiner.phase(), Phase::Offline);
    assert_eq!(joiner.runner.ctx.player.name_str(), "MATE", "the helper did not get their own crunchling back");
    assert_eq!(joiner.runner.ctx.globals.save_name, "joiner.crn");
    assert_eq!(joiner.runner.ctx.session.notice(), Notice::HostLeft);
    assert_eq!(joiner.runner.game.dialog.as_ref().map(|d| d.page()), Some(Page::Join));
    let saved_joiner = joiner.runner.finish();
    assert!(saved_joiner.saves.iter().any(|(n, d)| n == "joiner.crn" && d.windows(4).any(|w| w == b"MATE")));
}
