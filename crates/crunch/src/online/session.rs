use std::collections::VecDeque;

use avmanager::crt::ftol;
use avmanager::input::{DeviceState, Input};

use super::wire::{self, Message, PROTOCOL, TickRecord};
use crate::av::MOUSE_SENSITIVITY;
use crate::ctx::{Ctx, Globals, STATE_HOME_WORLD};
use crate::cursor::NO_HANDLE;
use crate::host::{Host, NetEvent};
use crate::player::PlayerData;

pub const CLOSE_NO_SUCH_GAME: u16 = 4404;
pub const CLOSE_GAME_FULL: u16 = 4409;
pub const CLOSE_HOST_LEFT: u16 = 4410;
const HAND_START: (i16, i16) = (320, 240);

// Which of the two hands in the world performed an action
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Primary,
    Helper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Offline,
    Hosting,
    Joined,
}

// The last thing worth telling the player on the dialog's status line
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    Nothing,
    FriendJoined,
    FriendLeft,
    HostLeft,
    NoSuchGame,
    GameFull,
    CouldNotConnect,
    ConnectionLost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Offline,
    HostConnecting,
    HostWaiting { code: String },
    HostPlaying { code: String },
    JoinConnecting { code: String },
    JoinWaiting { code: String },
    JoinPlaying { code: String },
}

// A pointer driven hand kept apart from the engine cursor, clamped to the same screen box
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hand {
    pub x: i16,
    pub y: i16,
    pub down: bool,
}

impl Default for Hand {
    fn default() -> Self {
        Self { x: HAND_START.0, y: HAND_START.1, down: false }
    }
}

impl Hand {
    // Jumps to an absolute pointer or applies scaled deltas, then clamps to 10..630 by 10..470
    pub fn apply(&mut self, device: &DeviceState) {
        match device.pointer {
            Some((px, py)) => {
                self.x = px;
                self.y = py;
            }
            None => {
                let s = f64::from(MOUSE_SENSITIVITY);
                self.x = self.x.wrapping_add(ftol(f64::from(device.mouse_dx) * s) as i16);
                self.y = self.y.wrapping_add(ftol(f64::from(device.mouse_dy) * s) as i16);
            }
        }
        self.x = self.x.clamp(10, 0x276);
        self.y = self.y.clamp(10, 0x1d6);
        self.down = device.buttons[0];
    }
}

// Connection phase, the lockstep record or replay of the current tick, the friend's hand and crunchling
pub struct Session {
    phase: Phase,
    epoch: u32,
    seq: u32,
    tick_time: Option<u32>,
    recording: Option<TickRecord>,
    replay: Option<(Vec<bool>, usize)>,
    replay_overrun: bool,
    queue: VecDeque<TickRecord>,
    helper_input: DeviceState,
    last_sent: Option<DeviceState>,
    pub helper: Hand,
    pub helper_keys: Input,
    pub local_hand: Hand,
    pub local: Input,
    pub helper_art: u16,
    guest: Option<PlayerData>,
    stash: Option<Globals>,
    pending_switch: Option<u32>,
    dialog_request: bool,
    notice: Notice,
    desync_sent: bool,
    desyncs: u32,
    replayed: u32,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            phase: Phase::Offline,
            epoch: 0,
            seq: 0,
            tick_time: None,
            recording: None,
            replay: None,
            replay_overrun: false,
            queue: VecDeque::new(),
            helper_input: DeviceState::new(),
            last_sent: None,
            helper: Hand::default(),
            helper_keys: Input::new(MOUSE_SENSITIVITY),
            local_hand: Hand::default(),
            local: Input::new(MOUSE_SENSITIVITY),
            helper_art: NO_HANDLE,
            guest: None,
            stash: None,
            pending_switch: None,
            dialog_request: false,
            notice: Notice::Nothing,
            desync_sent: false,
            desyncs: 0,
            replayed: 0,
        }
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn role(&self) -> Role {
        match self.phase {
            Phase::Offline => Role::Offline,
            Phase::HostConnecting | Phase::HostWaiting { .. } | Phase::HostPlaying { .. } => Role::Hosting,
            Phase::JoinConnecting { .. } | Phase::JoinWaiting { .. } | Phase::JoinPlaying { .. } => Role::Joined,
        }
    }

    // True while ticks flow between the two players
    pub fn is_playing(&self) -> bool {
        matches!(self.phase, Phase::HostPlaying { .. } | Phase::JoinPlaying { .. })
    }

    pub fn helper_present(&self) -> bool {
        self.is_playing()
    }

    // True on the machine whose player is the helper
    pub fn local_is_helper(&self) -> bool {
        matches!(self.phase, Phase::JoinPlaying { .. })
    }

    // The visiting friend's own crunchling, present on both machines while they play together
    pub fn guest(&self) -> Option<&PlayerData> {
        self.guest.as_ref()
    }

    pub fn guest_mut(&mut self) -> Option<&mut PlayerData> {
        self.guest.as_mut()
    }

    pub fn guest_name(&self) -> String {
        self.guest.as_ref().map(PlayerData::name_str).unwrap_or_default()
    }

    pub fn code(&self) -> Option<&str> {
        match &self.phase {
            Phase::HostWaiting { code }
            | Phase::HostPlaying { code }
            | Phase::JoinConnecting { code }
            | Phase::JoinWaiting { code }
            | Phase::JoinPlaying { code } => Some(code),
            _ => None,
        }
    }

    pub fn notice(&self) -> Notice {
        self.notice
    }

    pub fn clear_notice(&mut self) {
        self.notice = Notice::Nothing;
    }

    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    pub fn desyncs(&self) -> u32 {
        self.desyncs
    }

    pub fn replayed(&self) -> u32 {
        self.replayed
    }

    // Opens the dialog on the machine whose own hand clicked the Go Online button
    pub fn request_dialog(&mut self, who: Who) {
        if (who == Who::Helper) == self.local_is_helper() {
            self.dialog_request = true;
        }
    }

    pub fn take_dialog_request(&mut self) -> bool {
        std::mem::take(&mut self.dialog_request)
    }

    pub fn take_pending_switch(&mut self) -> Option<u32> {
        self.pending_switch.take()
    }

    // Forgets screen bound handles after a state switch
    pub fn on_switched(&mut self) {
        self.helper_art = NO_HANDLE;
    }

    // Screen position of the hand this machine's player moves
    pub fn local_pos(&self, cursor: (i16, i16)) -> (i16, i16) {
        if self.local_is_helper() { (self.local_hand.x, self.local_hand.y) } else { cursor }
    }

    pub fn time_ms(&self, real: u32) -> u32 {
        self.tick_time.unwrap_or(real)
    }

    // Freezes the clock for a tick and, while hosting with a friend, samples their hand and starts a record
    pub fn begin_tick(&mut self, now: u32, device: DeviceState) -> DeviceState {
        self.tick_time = Some(now);
        if !matches!(self.phase, Phase::HostPlaying { .. }) {
            return device;
        }
        let helper = self.helper_input.clone();
        self.helper.apply(&helper);
        self.helper_keys.heartbeat(&helper, (self.helper.x, self.helper.y));
        self.recording = Some(TickRecord {
            epoch: self.epoch,
            seq: self.seq,
            time_ms: now,
            host: device.clone(),
            helper,
            log: Vec::new(),
            hash: 0,
        });
        device
    }

    // Sends the finished record with the state hash the friend must reproduce
    pub fn end_tick<H: Host>(&mut self, host: &mut H, hash: u32) {
        self.tick_time = None;
        if let Some(mut record) = self.recording.take() {
            record.hash = hash;
            host.net_send(&wire::encode(&Message::Tick(Box::new(record))));
            self.seq += 1;
        }
    }

    pub fn next_replay(&mut self) -> Option<TickRecord> {
        if matches!(self.phase, Phase::JoinPlaying { .. }) { self.queue.pop_front() } else { None }
    }

    // Installs a host record as this tick's clock, sound answers and both players' inputs
    pub fn begin_replay(&mut self, record: &TickRecord) -> DeviceState {
        self.tick_time = Some(record.time_ms);
        self.replay = Some((record.log.clone(), 0));
        self.replay_overrun = false;
        self.helper.apply(&record.helper);
        self.helper_keys.heartbeat(&record.helper, (self.helper.x, self.helper.y));
        record.host.clone()
    }

    // Compares the state after a replayed tick with the host's hash and reports the first mismatch
    pub fn end_replay<H: Host>(&mut self, host: &mut H, record: &TickRecord, hash: u32) {
        self.tick_time = None;
        let consumed_all = self.replay.as_ref().is_none_or(|(log, pos)| *pos == log.len());
        self.replay = None;
        self.replayed += 1;
        self.seq = record.seq + 1;
        if (hash != record.hash || !consumed_all || self.replay_overrun) && !self.desync_sent {
            self.desync_sent = true;
            self.desyncs += 1;
            host.log(&format!("online: lost sync at tick {}, asking the host to restart", record.seq));
            host.net_send(&wire::encode(&Message::Desync { epoch: self.epoch, seq: record.seq }));
        }
    }

    // Next recorded sound answer while replaying, None when the tick is live
    pub fn replay_bool(&mut self) -> Option<bool> {
        let (log, pos) = self.replay.as_mut()?;
        match log.get(*pos) {
            Some(&done) => {
                *pos += 1;
                Some(done)
            }
            None => {
                self.replay_overrun = true;
                None
            }
        }
    }

    pub fn record_bool(&mut self, done: bool) {
        if let Some(record) = self.recording.as_mut() {
            record.log.push(done);
        }
    }

    // Sends the helper's input to the host whenever it changed, only the pointer while the dialog is up
    pub fn send_input<H: Host>(&mut self, host: &mut H, local: &DeviceState, dialog_open: bool) {
        if !matches!(self.phase, Phase::JoinPlaying { .. }) {
            return;
        }
        let device = if dialog_open { DeviceState { pointer: local.pointer, ..DeviceState::new() } } else { local.clone() };
        if self.last_sent.as_ref() != Some(&device) {
            host.net_send(&wire::encode(&Message::Input(device.clone())));
            self.last_sent = Some(device);
        }
    }
}

// Applies every connection event that arrived since the previous frame
pub fn poll_net<H: Host>(ctx: &mut Ctx<H>) {
    while let Some(event) = ctx.host.net_poll() {
        match event {
            NetEvent::Open => on_open(ctx),
            NetEvent::Message(bytes) => match wire::decode(&bytes) {
                Some(message) => on_message(ctx, message),
                None => ctx.host.log("online: ignored a malformed message"),
            },
            NetEvent::Closed { code, reason } => on_closed(ctx, code, &reason),
        }
    }
}

// Introduces this player's crunchling to the host once the join connection is up
fn on_open<H: Host>(ctx: &mut Ctx<H>) {
    if let Phase::JoinConnecting { code } = &ctx.session.phase {
        let code = code.clone();
        let hello = Message::Hello { protocol: PROTOCOL, player: ctx.player.clone() };
        ctx.host.net_send(&wire::encode(&hello));
        ctx.session.phase = Phase::JoinWaiting { code };
    }
}

fn on_message<H: Host>(ctx: &mut Ctx<H>, message: Message) {
    match message {
        Message::Code(code) => {
            if ctx.session.phase == Phase::HostConnecting {
                ctx.host.log(&format!("online: hosting game {code}"));
                ctx.session.phase = Phase::HostWaiting { code };
            }
        }
        Message::Hello { protocol, player } => match &ctx.session.phase {
            Phase::HostWaiting { .. } | Phase::HostPlaying { .. } if protocol == PROTOCOL => {
                ctx.session.guest = Some(player);
                start_as_host(ctx);
            }
            Phase::HostWaiting { .. } | Phase::HostPlaying { .. } => {
                ctx.host.log(&format!("online: a friend with protocol {protocol} tried to join, this build speaks {PROTOCOL}"));
            }
            _ => {}
        },
        Message::Start { epoch, seed, host, guest } => {
            if matches!(ctx.session.phase, Phase::JoinWaiting { .. } | Phase::JoinPlaying { .. }) {
                begin_joined(ctx, epoch, seed, host, guest);
            }
        }
        Message::Tick(record) => {
            if matches!(ctx.session.phase, Phase::JoinPlaying { .. }) && record.epoch == ctx.session.epoch {
                ctx.session.queue.push_back(*record);
            }
        }
        Message::Input(device) => {
            if matches!(ctx.session.phase, Phase::HostPlaying { .. }) {
                ctx.session.helper_input = device;
            }
        }
        Message::Leave | Message::PeerLeft => match &ctx.session.phase {
            Phase::HostWaiting { code } | Phase::HostPlaying { code } => {
                let code = code.clone();
                ctx.host.log("online: the friend left");
                friend_left(ctx, Phase::HostWaiting { code }, Notice::FriendLeft);
            }
            Phase::JoinWaiting { .. } | Phase::JoinPlaying { .. } => end_joined(ctx, Notice::HostLeft, true),
            _ => {}
        },
        Message::Desync { epoch, seq } => {
            if matches!(ctx.session.phase, Phase::HostPlaying { .. }) && epoch == ctx.session.epoch {
                ctx.host.log(&format!("online: the friend lost sync at tick {seq}, restarting the world"));
                start_as_host(ctx);
            }
        }
    }
}

fn on_closed<H: Host>(ctx: &mut Ctx<H>, code: u16, reason: &str) {
    let by_code = |fallback: Notice| match code {
        CLOSE_NO_SUCH_GAME => Notice::NoSuchGame,
        CLOSE_GAME_FULL => Notice::GameFull,
        CLOSE_HOST_LEFT => Notice::HostLeft,
        _ => fallback,
    };
    let phase = ctx.session.phase.clone();
    if phase == Phase::Offline {
        return;
    }
    ctx.host.log(&format!("online: connection closed with code {code} {reason}"));
    match phase {
        Phase::Offline => {}
        Phase::HostConnecting => go_offline(ctx, Notice::CouldNotConnect),
        Phase::HostWaiting { .. } | Phase::HostPlaying { .. } => go_offline(ctx, Notice::ConnectionLost),
        Phase::JoinConnecting { .. } => go_offline(ctx, by_code(Notice::CouldNotConnect)),
        Phase::JoinWaiting { .. } => go_offline(ctx, by_code(Notice::ConnectionLost)),
        Phase::JoinPlaying { .. } => end_joined(ctx, by_code(Notice::ConnectionLost), true),
    }
}

// Sends the friend's crunchling home and restarts the host's world alone, keeping or dropping the room
fn friend_left<H: Host>(ctx: &mut Ctx<H>, phase: Phase, notice: Notice) {
    let had_guest = ctx.session.guest.take().is_some();
    let session = &mut ctx.session;
    session.phase = phase;
    session.recording = None;
    session.helper_input = DeviceState::new();
    session.last_sent = None;
    session.notice = notice;
    if had_guest {
        session.pending_switch = Some(STATE_HOME_WORLD);
    }
}

fn go_offline<H: Host>(ctx: &mut Ctx<H>, notice: Notice) {
    ctx.host.net_close();
    friend_left(ctx, Phase::Offline, notice);
}

// Seeds a fresh world holding both crunchlings and sends the friend everything they need to follow it
fn start_as_host<H: Host>(ctx: &mut Ctx<H>) {
    let Some(code) = ctx.session.code().map(str::to_string) else { return };
    let Some(guest) = ctx.session.guest.clone() else { return };
    let session = &mut ctx.session;
    session.epoch += 1;
    session.seq = 0;
    let seed = ctx.host.unix_time().wrapping_mul(0x9e37_79b1) ^ ctx.host.time_ms().rotate_left(7) ^ session.epoch;
    let start = Message::Start { epoch: session.epoch, seed, host: ctx.player.clone(), guest };
    ctx.host.net_send(&wire::encode(&start));
    ctx.rand.srand(seed);
    session.phase = Phase::HostPlaying { code };
    session.helper = Hand::default();
    session.helper_keys = Input::new(MOUSE_SENSITIVITY);
    session.helper_input = DeviceState::new();
    session.recording = None;
    session.notice = Notice::FriendJoined;
    session.pending_switch = Some(STATE_HOME_WORLD);
}

// Simulates the host's world with the host's crunchling resident and this player's own as the guest
fn begin_joined<H: Host>(ctx: &mut Ctx<H>, epoch: u32, seed: u32, host: PlayerData, guest: PlayerData) {
    let Some(code) = ctx.session.code().map(str::to_string) else { return };
    if ctx.session.stash.is_none() {
        ctx.session.stash = Some(ctx.globals.clone());
    }
    ctx.player = host;
    ctx.globals.player_loaded = true;
    ctx.rand.srand(seed);
    let session = &mut ctx.session;
    session.guest = Some(guest);
    session.epoch = epoch;
    session.seq = 0;
    session.queue.clear();
    session.replay = None;
    session.tick_time = None;
    session.helper = Hand::default();
    session.helper_keys = Input::new(MOUSE_SENSITIVITY);
    session.last_sent = None;
    session.desync_sent = false;
    session.notice = Notice::FriendJoined;
    session.phase = Phase::JoinPlaying { code };
    session.pending_switch = Some(STATE_HOME_WORLD);
}

// Brings this player's crunchling home with everything that happened to it and shows why when asked
fn end_joined<H: Host>(ctx: &mut Ctx<H>, notice: Notice, show: bool) {
    ctx.host.net_close();
    if let Some(own) = ctx.session.guest.take() {
        ctx.player = own;
    }
    if let Some(stash) = ctx.session.stash.take() {
        ctx.globals.player_loaded = stash.player_loaded;
        ctx.globals.save_name = stash.save_name;
    }
    let session = &mut ctx.session;
    session.phase = Phase::Offline;
    session.queue.clear();
    session.replay = None;
    session.tick_time = None;
    session.last_sent = None;
    session.helper_input = DeviceState::new();
    session.notice = notice;
    session.pending_switch = Some(STATE_HOME_WORLD);
    session.dialog_request = show;
}

pub fn start_hosting<H: Host>(ctx: &mut Ctx<H>) {
    if ctx.session.phase == Phase::Offline {
        ctx.host.net_connect("/online/host");
        ctx.session.phase = Phase::HostConnecting;
        ctx.session.notice = Notice::Nothing;
    }
}

pub fn join<H: Host>(ctx: &mut Ctx<H>, code: &str) {
    if ctx.session.phase == Phase::Offline {
        ctx.host.net_connect(&format!("/online/join/{code}"));
        ctx.session.phase = Phase::JoinConnecting { code: code.to_string() };
        ctx.session.notice = Notice::Nothing;
    }
}

// Abandons a connection attempt or an empty room
pub fn cancel<H: Host>(ctx: &mut Ctx<H>) {
    if matches!(
        ctx.session.phase,
        Phase::HostConnecting | Phase::HostWaiting { .. } | Phase::JoinConnecting { .. } | Phase::JoinWaiting { .. }
    ) {
        go_offline(ctx, Notice::Nothing);
    }
}

// Ends the session on purpose, the host playing on alone and the guest returning to their own world
pub fn leave<H: Host>(ctx: &mut Ctx<H>) {
    match ctx.session.phase {
        Phase::HostWaiting { .. } | Phase::HostPlaying { .. } => {
            ctx.host.net_send(&wire::encode(&Message::Leave));
            go_offline(ctx, Notice::Nothing);
        }
        Phase::JoinPlaying { .. } => {
            ctx.host.net_send(&wire::encode(&Message::Leave));
            end_joined(ctx, Notice::Nothing, false);
        }
        _ => cancel(ctx),
    }
}

// Treats a replayed quit as the host leaving, since only the host's own hand may end the game
pub fn host_quit<H: Host>(ctx: &mut Ctx<H>) {
    ctx.host.log("online: the host quit");
    end_joined(ctx, Notice::HostLeft, true);
}

// Tells the friend goodbye and restores this player's own crunchling before the game saves it
pub fn shutdown<H: Host>(ctx: &mut Ctx<H>) {
    match ctx.session.phase {
        Phase::Offline => {}
        Phase::JoinPlaying { .. } => {
            ctx.host.net_send(&wire::encode(&Message::Leave));
            end_joined(ctx, Notice::Nothing, false);
        }
        Phase::HostWaiting { .. } | Phase::HostPlaying { .. } => {
            ctx.host.net_send(&wire::encode(&Message::Leave));
            go_offline(ctx, Notice::Nothing);
        }
        _ => go_offline(ctx, Notice::Nothing),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_follows_pointer_and_clamps() {
        let mut hand = Hand::default();
        let mut device = DeviceState::new();
        device.pointer = Some((5, 700));
        device.buttons[0] = true;
        hand.apply(&device);
        assert_eq!((hand.x, hand.y, hand.down), (10, 0x1d6, true));
        device.pointer = None;
        device.mouse_dx = 21;
        device.mouse_dy = -3;
        device.buttons[0] = false;
        hand.apply(&device);
        assert_eq!((hand.x, hand.y, hand.down), (20, 0x1d5, false));
    }

    #[test]
    fn dialog_requests_follow_the_local_hand() {
        let mut session = Session::new();
        session.request_dialog(Who::Helper);
        assert!(!session.take_dialog_request());
        session.request_dialog(Who::Primary);
        assert!(session.take_dialog_request());
        session.phase = Phase::JoinPlaying { code: "ABCD".into() };
        session.request_dialog(Who::Primary);
        assert!(!session.take_dialog_request());
        session.request_dialog(Who::Helper);
        assert!(session.take_dialog_request());
    }

    #[test]
    fn replay_log_is_consumed_in_order_and_overrun_is_flagged() {
        let mut session = Session::new();
        session.phase = Phase::JoinPlaying { code: "ABCD".into() };
        let record = TickRecord {
            epoch: 0,
            seq: 0,
            time_ms: 5,
            host: DeviceState::new(),
            helper: DeviceState::new(),
            log: vec![false, true],
            hash: 0,
        };
        session.begin_replay(&record);
        assert_eq!(session.time_ms(99), 5);
        assert_eq!(session.replay_bool(), Some(false));
        assert_eq!(session.replay_bool(), Some(true));
        assert_eq!(session.replay_bool(), None);
        assert!(session.replay_overrun);
        assert_eq!(session.time_ms(99), 5);
    }

    #[test]
    fn guest_name_is_empty_without_a_friend() {
        let mut session = Session::new();
        assert_eq!(session.guest_name(), "");
        let mut mate = PlayerData::default();
        mate.set_name("MATE");
        session.guest = Some(mate);
        assert_eq!(session.guest_name(), "MATE");
        assert_eq!(session.guest_mut().map(|g| g.happiness += 1), Some(()));
        assert_eq!(session.guest().map(|g| g.happiness), Some(1));
    }
}
