// Reproduces the MSVC 6 C runtime pieces the original binaries depend on

pub const RAND_MAX: i32 = 0x7fff;

#[derive(Clone, Debug)]
pub struct Rand {
    seed: u32,
}

impl Default for Rand {
    fn default() -> Self {
        Self { seed: 1 }
    }
}

impl Rand {
    pub fn srand(&mut self, seed: u32) {
        self.seed = seed;
    }

    pub fn state(&self) -> u32 {
        self.seed
    }

    // Linear congruential generator identical to msvcrt rand
    pub fn rand(&mut self) -> i32 {
        self.seed = self.seed.wrapping_mul(214013).wrapping_add(2531011);
        ((self.seed >> 16) & 0x7fff) as i32
    }

    // Matches the engine helper that scales rand over [0, 32767] into base..=base+span
    pub fn ranged(&mut self, base: i32, span: i32) -> i32 {
        ftol(self.rand() as f64 / 32767.0 * span as f64 + base as f64)
    }
}

// Truncates toward zero like the __ftol helper used for every float to int store
pub fn ftol(value: f64) -> i32 {
    value as i32
}

// Case insensitive compare with the same ordering result as _stricmp
pub fn stricmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let ca = a.get(i).copied().unwrap_or(0);
        let cb = b.get(i).copied().unwrap_or(0);
        let la = ca.to_ascii_lowercase();
        let lb = cb.to_ascii_lowercase();
        if la != lb {
            return if la < lb { -1 } else { 1 };
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

// Parses leading whitespace, sign and decimal digits exactly like atoi
pub fn atoi(text: &[u8]) -> i32 {
    let mut i = 0;
    while i < text.len() && matches!(text[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let mut negative = false;
    if i < text.len() && (text[i] == b'-' || text[i] == b'+') {
        negative = text[i] == b'-';
        i += 1;
    }
    let mut value: i32 = 0;
    while i < text.len() && text[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((text[i] - b'0') as i32);
        i += 1;
    }
    if negative { value.wrapping_neg() } else { value }
}

// Strips any directory prefix and the final extension the way the asset managers key names
pub fn asset_key(name: &str) -> String {
    let base = match name.rfind('\\') {
        Some(p) => &name[p + 1..],
        None => name,
    };
    let mut key: String = base.chars().take(0x3f).collect();
    if let Some(dot) = key.rfind('.') {
        key.truncate(dot);
    }
    key
}

// Appends the extension unless the name already ends with it ignoring case
pub fn with_extension(name: &str, ext: &str) -> String {
    let n = name.as_bytes();
    let e = ext.as_bytes();
    if n.len() >= e.len() && stricmp(&n[n.len() - e.len()..], e) == 0 {
        return name.to_string();
    }
    format!("{name}{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rand_matches_msvc_sequence() {
        let mut r = Rand::default();
        assert_eq!([r.rand(), r.rand(), r.rand()], [41, 18467, 6334]);
    }

    #[test]
    fn asset_key_strips_dir_and_extension() {
        assert_eq!(asset_key("foo\\Bar.baz.bam"), "Bar.baz");
        assert_eq!(asset_key("Speed_cold"), "Speed_cold");
    }

    #[test]
    fn extension_is_not_doubled() {
        assert_eq!(with_extension("a.BAM", ".bam"), "a.BAM");
        assert_eq!(with_extension("a", ".bam"), "a.bam");
    }

    #[test]
    fn atoi_skips_space() {
        assert_eq!(atoi(b" 40\r"), 40);
        assert_eq!(atoi(b"-7x"), -7);
    }
}
