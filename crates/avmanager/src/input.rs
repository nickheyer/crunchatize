use crate::crt::ftol;

// Engine key codes are compact indexes mapped onto DirectInput scan codes by this table
const CODE_TO_SCAN: [i32; 0x68] = {
    let mut t = [0i32; 0x68];
    let mut i = 1;
    while i <= 0x53 {
        t[i] = i as i32;
        i += 1;
    }
    t[0x54] = 0x57;
    t[0x55] = 0x58;
    t[0x56] = 0x9c;
    t[0x57] = 0x9d;
    t[0x58] = 0xb5;
    t[0x59] = 0xb7;
    t[0x5a] = 0xb8;
    t[0x5b] = 0xc7;
    t[0x5c] = 0xc8;
    t[0x5d] = 0xc9;
    t[0x5e] = 0xcb;
    t[0x5f] = 0xcd;
    t[0x60] = 0xcf;
    t[0x61] = 0xd0;
    t[0x62] = 0xd1;
    t[0x63] = 0xd2;
    t[0x64] = 0xd3;
    t[0x65] = 0xdb;
    t[0x66] = 0xdc;
    t[0x67] = 0xdd;
    t
};

const KEY_NAMES: [&str; 0x68] = [
    "", "ESC", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", "BCKSP", "TAB", "Q", "W",
    "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]", "RET", "LCTRL", "A", "S", "D", "F", "G", "H",
    "J", "K", "L", ";", "'", "`", "LSHFT", "\\", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/",
    "RSHFT", "MULT", "LALT", "SP", "CAPS", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9",
    "F10", "NUM", "SCRL", "PAD7", "PAD8", "PAD9", "SUB", "PAD4", "PAD5", "PAD6", "ADD", "PAD1",
    "PAD2", "PAD3", "PAD0", "DEC", "F11", "F12", "ENT", "RCTRL", "DIV", "SYS", "RALT", "HOME", "UP",
    "PGUP", "LEFT", "RIGHT", "END", "DOWN", "PGDN", "INS", "DEL", "LWIN", "RWIN", "APPS",
];

// Raw device state the host samples once per tick, pointer set by hosts that know the absolute mouse spot
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceState {
    pub scan_keys: [bool; 256],
    pub mouse_dx: i32,
    pub mouse_dy: i32,
    pub pointer: Option<(i16, i16)>,
    pub buttons: [bool; 3],
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceState {
    pub fn new() -> Self {
        Self { scan_keys: [false; 256], mouse_dx: 0, mouse_dy: 0, pointer: None, buttons: [false; 3] }
    }
}

#[derive(Clone, Debug)]
pub struct Input {
    keys: [[u8; 256]; 2],
    buttons: [u32; 3],
    current: usize,
    mouse: [i16; 2],
    sensitivity: f32,
}

impl Input {
    pub fn new(sensitivity: f32) -> Self {
        Self { keys: [[0; 256]; 2], buttons: [0; 3], current: 0, mouse: [0; 2], sensitivity }
    }

    // Flips the key snapshot pair and samples the devices, a pointer becoming the delta from the cursor onto it
    pub fn heartbeat(&mut self, device: &DeviceState, cursor: (i16, i16)) {
        self.current ^= 1;
        let cur = &mut self.keys[self.current];
        for (dst, &down) in cur.iter_mut().zip(device.scan_keys.iter()) {
            *dst = if down { 0x80 } else { 0 };
        }
        self.buttons = [0; 3];
        for (i, &down) in device.buttons.iter().enumerate() {
            if down {
                self.buttons[i] |= 0x80;
            }
        }
        self.mouse = match device.pointer {
            Some((px, py)) => [px.saturating_sub(cursor.0), py.saturating_sub(cursor.1)],
            None => {
                let s = self.sensitivity as f64;
                [ftol(device.mouse_dx as f64 * s) as i16, ftol(device.mouse_dy as f64 * s) as i16]
            }
        };
    }

    fn scan_for(code: i32) -> Option<usize> {
        CODE_TO_SCAN.get(usize::try_from(code).ok()?).map(|&s| s as usize)
    }

    pub fn is_key_down(&self, code: i32) -> bool {
        match Self::scan_for(code) {
            Some(scan) => self.keys[self.current][scan] & 0x80 != 0,
            None => false,
        }
    }

    // True when the key is down now and was up in the previous snapshot
    fn newly_pressed(&self, code: i32) -> bool {
        let Some(scan) = Self::scan_for(code) else { return false };
        let now = self.keys[self.current][scan] & 0x80 != 0;
        let before = self.keys[self.current ^ 1][scan] & 0x80 != 0;
        now && !before
    }

    // Returns the lowest newly pressed key code ignoring the shift keys, or 0
    pub fn key_input(&self) -> i32 {
        (0..256)
            .filter(|&c| c != 0x2a && c != 0x36)
            .find(|&c| self.newly_pressed(c))
            .unwrap_or(0)
    }

    pub fn is_mouse_button_down(&self, button: usize) -> bool {
        self.buttons[button] & 0x80 != 0
    }

    pub fn mouse_delta(&self) -> (i16, i16) {
        (self.mouse[0], self.mouse[1])
    }

    pub fn translate_key_def(code: i32) -> Option<&'static str> {
        if !(0..=0x100).contains(&code) {
            return None;
        }
        KEY_NAMES.get(code as usize).copied().filter(|n| !n.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_codes_map_to_extended_scancodes() {
        assert_eq!(CODE_TO_SCAN[0x5e], 0xcb);
        assert_eq!(Input::translate_key_def(0x5e), Some("LEFT"));
        assert_eq!(Input::translate_key_def(0x61), Some("DOWN"));
        assert_eq!(Input::translate_key_def(0), None);
    }

    #[test]
    fn key_input_reports_only_new_presses() {
        let mut input = Input::new(0.5);
        let mut dev = DeviceState::new();
        dev.scan_keys[0x39] = true;
        input.heartbeat(&dev, (0, 0));
        assert_eq!(input.key_input(), 0x39);
        input.heartbeat(&dev, (0, 0));
        assert_eq!(input.key_input(), 0);
        assert!(input.is_key_down(0x39));
    }

    #[test]
    fn mouse_delta_is_scaled_and_truncated() {
        let mut input = Input::new(0.5);
        let dev = DeviceState { mouse_dx: 7, mouse_dy: -3, ..DeviceState::new() };
        input.heartbeat(&dev, (0, 0));
        assert_eq!(input.mouse_delta(), (3, -1));
    }

    #[test]
    fn pointer_overrides_the_delta_with_the_move_onto_it() {
        let mut input = Input::new(0.5);
        let dev = DeviceState { pointer: Some((100, 40)), mouse_dx: 7, ..DeviceState::new() };
        input.heartbeat(&dev, (60, 50));
        assert_eq!(input.mouse_delta(), (40, -10));
    }
}
