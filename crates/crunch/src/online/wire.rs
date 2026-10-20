use avmanager::input::DeviceState;

use crate::player::{PlayerData, SAVE_SIZE};

pub const PROTOCOL: u8 = 3;
pub const CODE_LEN: usize = 4;

const TAG_HELLO: u8 = 1;
const TAG_START: u8 = 2;
const TAG_TICK: u8 = 3;
const TAG_INPUT: u8 = 4;
const TAG_LEAVE: u8 = 5;
const TAG_DESYNC: u8 = 6;
const TAG_CODE: u8 = 0x80;
const TAG_PEER_LEFT: u8 = 0x81;

const KEY_BYTES: usize = 32;
const DEVICE_LEN: usize = KEY_BYTES + 4 + 4 + 1 + 4 + 1;
const RECORD_LEN: usize = SAVE_SIZE as usize;

// Everything one side of a session says to the other, plus the two notices the server adds
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Hello { protocol: u8, player: PlayerData },
    Start { epoch: u32, seed: u32, host: PlayerData, guest: PlayerData },
    Tick(Box<TickRecord>),
    Input(DeviceState),
    Leave,
    Desync { epoch: u32, seq: u32 },
    Code(String),
    PeerLeft,
}

// Inputs, clock and replayed sound answers of one host tick, with the state hash after it
#[derive(Clone, Debug, PartialEq)]
pub struct TickRecord {
    pub epoch: u32,
    pub seq: u32,
    pub time_ms: u32,
    pub host: DeviceState,
    pub helper: DeviceState,
    pub log: Vec<bool>,
    pub hash: u32,
}

fn put_device(out: &mut Vec<u8>, d: &DeviceState) {
    let mut keys = [0u8; KEY_BYTES];
    for (i, &down) in d.scan_keys.iter().enumerate() {
        if down {
            keys[i / 8] |= 1 << (i % 8);
        }
    }
    out.extend_from_slice(&keys);
    out.extend_from_slice(&d.mouse_dx.to_le_bytes());
    out.extend_from_slice(&d.mouse_dy.to_le_bytes());
    match d.pointer {
        Some((x, y)) => {
            out.push(1);
            out.extend_from_slice(&x.to_le_bytes());
            out.extend_from_slice(&y.to_le_bytes());
        }
        None => out.extend_from_slice(&[0; 5]),
    }
    out.push(d.buttons.iter().enumerate().map(|(i, &b)| (b as u8) << i).sum());
}

fn get_device(bytes: &[u8]) -> Option<DeviceState> {
    let b = bytes.get(..DEVICE_LEN)?;
    let mut d = DeviceState::new();
    for (i, key) in d.scan_keys.iter_mut().enumerate() {
        *key = b[i / 8] & (1 << (i % 8)) != 0;
    }
    d.mouse_dx = i32::from_le_bytes([b[32], b[33], b[34], b[35]]);
    d.mouse_dy = i32::from_le_bytes([b[36], b[37], b[38], b[39]]);
    if b[40] != 0 {
        d.pointer = Some((i16::from_le_bytes([b[41], b[42]]), i16::from_le_bytes([b[43], b[44]])));
    }
    for (i, button) in d.buttons.iter_mut().enumerate() {
        *button = b[45] & (1 << i) != 0;
    }
    Some(d)
}

fn get_record(bytes: &[u8], at: usize) -> Option<PlayerData> {
    let raw: [u8; RECORD_LEN] = bytes.get(at..at + RECORD_LEN)?.try_into().ok()?;
    Some(PlayerData::from_bytes(&raw))
}

fn get_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let b = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn encode(msg: &Message) -> Vec<u8> {
    let mut out = Vec::new();
    match msg {
        Message::Hello { protocol, player } => {
            out.extend_from_slice(&[TAG_HELLO, *protocol]);
            out.extend_from_slice(&player.to_bytes());
        }
        Message::Start { epoch, seed, host, guest } => {
            out.push(TAG_START);
            out.extend_from_slice(&epoch.to_le_bytes());
            out.extend_from_slice(&seed.to_le_bytes());
            out.extend_from_slice(&host.to_bytes());
            out.extend_from_slice(&guest.to_bytes());
        }
        Message::Tick(t) => {
            out.push(TAG_TICK);
            out.extend_from_slice(&t.epoch.to_le_bytes());
            out.extend_from_slice(&t.seq.to_le_bytes());
            out.extend_from_slice(&t.time_ms.to_le_bytes());
            put_device(&mut out, &t.host);
            put_device(&mut out, &t.helper);
            out.extend_from_slice(&(t.log.len() as u16).to_le_bytes());
            let mut bits = vec![0u8; t.log.len().div_ceil(8)];
            for (i, &done) in t.log.iter().enumerate() {
                if done {
                    bits[i / 8] |= 1 << (i % 8);
                }
            }
            out.extend_from_slice(&bits);
            out.extend_from_slice(&t.hash.to_le_bytes());
        }
        Message::Input(d) => {
            out.push(TAG_INPUT);
            put_device(&mut out, d);
        }
        Message::Leave => out.push(TAG_LEAVE),
        Message::Desync { epoch, seq } => {
            out.push(TAG_DESYNC);
            out.extend_from_slice(&epoch.to_le_bytes());
            out.extend_from_slice(&seq.to_le_bytes());
        }
        Message::Code(code) => {
            out.push(TAG_CODE);
            out.extend_from_slice(code.as_bytes());
        }
        Message::PeerLeft => out.push(TAG_PEER_LEFT),
    }
    out
}

// Parses one frame from the wire, None for anything malformed or unknown
pub fn decode(bytes: &[u8]) -> Option<Message> {
    let (&tag, body) = bytes.split_first()?;
    match tag {
        TAG_HELLO => Some(Message::Hello { protocol: *body.first()?, player: get_record(body, 1)? }),
        TAG_START => Some(Message::Start {
            epoch: get_u32(body, 0)?,
            seed: get_u32(body, 4)?,
            host: get_record(body, 8)?,
            guest: get_record(body, 8 + RECORD_LEN)?,
        }),
        TAG_TICK => {
            let epoch = get_u32(body, 0)?;
            let seq = get_u32(body, 4)?;
            let time_ms = get_u32(body, 8)?;
            let host = get_device(body.get(12..)?)?;
            let helper = get_device(body.get(12 + DEVICE_LEN..)?)?;
            let at = 12 + 2 * DEVICE_LEN;
            let count = u16::from_le_bytes([*body.get(at)?, *body.get(at + 1)?]) as usize;
            let bits = body.get(at + 2..at + 2 + count.div_ceil(8))?;
            let log = (0..count).map(|i| bits[i / 8] & (1 << (i % 8)) != 0).collect();
            let hash = get_u32(body, at + 2 + count.div_ceil(8))?;
            Some(Message::Tick(Box::new(TickRecord { epoch, seq, time_ms, host, helper, log, hash })))
        }
        TAG_INPUT => Some(Message::Input(get_device(body)?)),
        TAG_LEAVE => Some(Message::Leave),
        TAG_DESYNC => Some(Message::Desync { epoch: get_u32(body, 0)?, seq: get_u32(body, 4)? }),
        TAG_CODE => {
            let code = std::str::from_utf8(body.get(..CODE_LEN)?).ok()?;
            Some(Message::Code(code.to_string()))
        }
        TAG_PEER_LEFT => Some(Message::PeerLeft),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> DeviceState {
        let mut d = DeviceState::new();
        d.scan_keys[0x39] = true;
        d.scan_keys[0xcb] = true;
        d.mouse_dx = -7;
        d.mouse_dy = 1234;
        d.pointer = Some((-3, 479));
        d.buttons = [true, false, true];
        d
    }

    fn record(name: &str, stage: u16) -> PlayerData {
        let mut player = PlayerData::default();
        player.set_name(name);
        player.stage = stage;
        player.food = 3;
        player.flesh = [1, 2, 3];
        player
    }

    #[test]
    fn every_message_round_trips() {
        let tick = TickRecord {
            epoch: 2,
            seq: 77,
            time_ms: 0xdead_beef,
            host: device(),
            helper: DeviceState::new(),
            log: vec![true, false, false, true, true, false, false, false, false, true],
            hash: 0x1234_5678,
        };
        for msg in [
            Message::Hello { protocol: PROTOCOL, player: record("MATE", 1) },
            Message::Start { epoch: 1, seed: 99, host: record("spaz", 2), guest: record("MATEY", 0) },
            Message::Tick(Box::new(tick)),
            Message::Input(device()),
            Message::Leave,
            Message::Desync { epoch: 3, seq: 4 },
            Message::Code("KWZB".into()),
            Message::PeerLeft,
        ] {
            assert_eq!(decode(&encode(&msg)), Some(msg));
        }
    }

    #[test]
    fn short_and_unknown_frames_are_rejected() {
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[TAG_HELLO, PROTOCOL, 1, 2]), None);
        assert_eq!(decode(&[TAG_START, 1, 2]), None);
        let one_record = encode(&Message::Hello { protocol: PROTOCOL, player: record("A", 0) });
        assert_eq!(decode(&[&[TAG_START, 0, 0, 0, 0, 0, 0, 0, 0][..], &one_record[2..]].concat()), None);
        assert_eq!(decode(&[TAG_TICK]), None);
        assert_eq!(decode(&[0x7f]), None);
        assert_eq!(decode(&[TAG_CODE, b'A', b'B']), None);
    }
}
