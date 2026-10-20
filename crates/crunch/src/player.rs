use avmanager::crt::ftol;
use avmanager::texture::Palette24;

use crate::host::Host;

pub const SAVE_MAGIC: u32 = 0xfafa_1628;
pub const SAVE_SIZE: u32 = 0x50;
pub const NAME_CAPACITY: usize = 16;

pub const STAT_RUNNING: i32 = 0;
pub const STAT_STRENGTH: i32 = 1;
pub const STAT_JUMPING: i32 = 2;

// The 80 byte player record kept in memory and written verbatim to .crn files
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerData {
    pub name: Vec<u8>,
    pub kind: i16,
    pub stage: u16,
    pub flesh: [u8; 3],
    pub hair: [u8; 3],
    pub pad_1a: [u8; 2],
    pub running: i32,
    pub jumping: i32,
    pub strength: i32,
    pub food: i16,
    pub happiness: i16,
    pub run_best_frac: i16,
    pub run_best_secs: u16,
    pub jump_best_frac: i16,
    pub jump_best_secs: u16,
    pub throw_high_score: u32,
    pub pad_38: [u8; 4],
    pub challenge_active: u8,
    pub challenge_won: u8,
    pub pad_3e: [u8; 2],
    pub transform_pending: i32,
    pub lagging_stat: i32,
    pub first_home_visit: u8,
    pub pad_49: [u8; 3],
    pub tips_index: i32,
}

impl Default for PlayerData {
    fn default() -> Self {
        Self::from_bytes(&[0u8; SAVE_SIZE as usize])
    }
}

impl PlayerData {
    // Decodes the raw record exactly as the game lays it out in memory
    pub fn from_bytes(b: &[u8; 0x50]) -> Self {
        let i16_at = |o: usize| i16::from_le_bytes([b[o], b[o + 1]]);
        let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let i32_at = |o: usize| i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let u32_at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        Self {
            name: b[..NAME_CAPACITY].to_vec(),
            kind: i16_at(0x10),
            stage: u16_at(0x12),
            flesh: [b[0x14], b[0x15], b[0x16]],
            hair: [b[0x17], b[0x18], b[0x19]],
            pad_1a: [b[0x1a], b[0x1b]],
            running: i32_at(0x1c),
            jumping: i32_at(0x20),
            strength: i32_at(0x24),
            food: i16_at(0x28),
            happiness: i16_at(0x2a),
            run_best_frac: i16_at(0x2c),
            run_best_secs: u16_at(0x2e),
            jump_best_frac: i16_at(0x30),
            jump_best_secs: u16_at(0x32),
            throw_high_score: u32_at(0x34),
            pad_38: [b[0x38], b[0x39], b[0x3a], b[0x3b]],
            challenge_active: b[0x3c],
            challenge_won: b[0x3d],
            pad_3e: [b[0x3e], b[0x3f]],
            transform_pending: i32_at(0x40),
            lagging_stat: i32_at(0x44),
            first_home_visit: b[0x48],
            pad_49: [b[0x49], b[0x4a], b[0x4b]],
            tips_index: i32_at(0x4c),
        }
    }

    pub fn to_bytes(&self) -> [u8; 0x50] {
        let mut b = [0u8; 0x50];
        b[..NAME_CAPACITY].copy_from_slice(&self.name);
        b[0x10..0x12].copy_from_slice(&self.kind.to_le_bytes());
        b[0x12..0x14].copy_from_slice(&self.stage.to_le_bytes());
        b[0x14..0x17].copy_from_slice(&self.flesh);
        b[0x17..0x1a].copy_from_slice(&self.hair);
        b[0x1a..0x1c].copy_from_slice(&self.pad_1a);
        b[0x1c..0x20].copy_from_slice(&self.running.to_le_bytes());
        b[0x20..0x24].copy_from_slice(&self.jumping.to_le_bytes());
        b[0x24..0x28].copy_from_slice(&self.strength.to_le_bytes());
        b[0x28..0x2a].copy_from_slice(&self.food.to_le_bytes());
        b[0x2a..0x2c].copy_from_slice(&self.happiness.to_le_bytes());
        b[0x2c..0x2e].copy_from_slice(&self.run_best_frac.to_le_bytes());
        b[0x2e..0x30].copy_from_slice(&self.run_best_secs.to_le_bytes());
        b[0x30..0x32].copy_from_slice(&self.jump_best_frac.to_le_bytes());
        b[0x32..0x34].copy_from_slice(&self.jump_best_secs.to_le_bytes());
        b[0x34..0x38].copy_from_slice(&self.throw_high_score.to_le_bytes());
        b[0x38..0x3c].copy_from_slice(&self.pad_38);
        b[0x3c] = self.challenge_active;
        b[0x3d] = self.challenge_won;
        b[0x3e..0x40].copy_from_slice(&self.pad_3e);
        b[0x40..0x44].copy_from_slice(&self.transform_pending.to_le_bytes());
        b[0x44..0x48].copy_from_slice(&self.lagging_stat.to_le_bytes());
        b[0x48] = self.first_home_visit;
        b[0x49..0x4c].copy_from_slice(&self.pad_49);
        b[0x4c..0x50].copy_from_slice(&self.tips_index.to_le_bytes());
        b
    }

    // Name as text up to the first NUL
    pub fn name_str(&self) -> String {
        let end = self.name.iter().position(|&c| c == 0).unwrap_or(self.name.len());
        String::from_utf8_lossy(&self.name[..end]).into_owned()
    }

    // Copies a name in with strcpy semantics into the 16 byte field
    pub fn set_name(&mut self, name: &str) {
        let mut buf = vec![0u8; NAME_CAPACITY];
        let bytes = name.as_bytes();
        let n = bytes.len().min(NAME_CAPACITY - 1);
        buf[..n].copy_from_slice(&bytes[..n]);
        self.name = buf;
    }

    // Stage prefix and type infix followed by the suffix, with the pre transform stage while pending
    pub fn anim_name(&self, suffix: &str, keep_baby_type: bool, adult_as_type0: bool) -> String {
        let mut stage = self.stage as u32;
        if self.transform_pending != 0 && stage != 0 {
            stage -= 1;
        }
        let mut out = String::new();
        match stage {
            0 => out.push('b'),
            1 => out.push('t'),
            2 => out.push('a'),
            _ => {}
        }
        if stage == 0 && !keep_baby_type {
            out.push('_');
        } else {
            let mut kind = self.kind;
            if adult_as_type0 && kind == 2 {
                kind = 0;
            }
            match kind {
                0 => out.push('_'),
                1 => out.push_str("2_"),
                2 => out.push_str("3_"),
                _ => {}
            }
        }
        out.push_str(suffix);
        out
    }

    // Stage the crunchling still looks like, one back while a transform waits for its home world
    pub fn shown_stage(&self) -> u16 {
        if self.transform_pending != 0 && self.stage != 0 { self.stage - 1 } else { self.stage }
    }

    // Row index into the per form tables, type plus three per stage
    pub fn anim_index(&self, ignore_transform: bool) -> u32 {
        let kind = self.kind as i32 as u32;
        let stage = self.stage as u32;
        if !ignore_transform {
            if self.transform_pending != 0 && stage != 0 {
                return kind.wrapping_add((stage - 1) * 3);
            }
            return kind.wrapping_add(stage * 3);
        }
        if stage == 0 { kind } else { kind.wrapping_add((stage - 1) * 3) }
    }

    fn stat_cap(&self) -> Option<i32> {
        match self.stage {
            0 => Some(8),
            1 => Some(15),
            2 => Some(20),
            _ => None,
        }
    }

    // Adds to a stat with the stage cap and reports whether the cap was hit
    fn add_stat_clamped(stat: &mut i32, delta: i32, cap: Option<i32>) -> bool {
        if *stat + delta < 0 {
            *stat = 0;
            return false;
        }
        *stat += delta;
        if let Some(c) = cap
            && *stat > c
        {
            *stat = c;
            return true;
        }
        false
    }

    // Applies training rewards, advances the stage, and records which stat lags behind
    pub fn add_stats(&mut self, d_running: i32, d_strength: i32, d_jumping: i32) {
        self.lagging_stat = -1;
        let cap = self.stat_cap();
        let hit_run = Self::add_stat_clamped(&mut self.running, d_running, cap);
        let hit_str = Self::add_stat_clamped(&mut self.strength, d_strength, cap);
        let hit_jmp = Self::add_stat_clamped(&mut self.jumping, d_jumping, cap);
        if self.stage == 0 {
            if self.running > 6 && self.strength > 6 && self.jumping > 6 {
                self.stage = 1;
                self.transform_pending = 1;
                return;
            }
        } else if self.stage == 1 && self.running > 13 && self.strength > 13 && self.jumping > 13 {
            self.stage = 2;
            self.transform_pending = 1;
            return;
        }
        if (hit_run || hit_str || hit_jmp) && self.stage < 2 {
            let threshold = if self.stage == 1 { 14 } else { 7 };
            if self.running < threshold {
                self.lagging_stat = STAT_RUNNING;
            } else if self.strength < threshold {
                self.lagging_stat = STAT_STRENGTH;
            } else if self.jumping < threshold {
                self.lagging_stat = STAT_JUMPING;
            }
        }
    }

    // Pays for one training game, food and happiness spent when going home, the stat raised twice with a berry bonus
    pub fn train(&mut self, stat: i32, bonus: bool, going_home: bool) {
        if going_home {
            if self.food != 0 {
                self.food = self.food.wrapping_sub(1);
            }
            if self.happiness != 0 {
                self.happiness = self.happiness.wrapping_sub(1);
            }
        }
        let steps = if bonus { 2 } else { 1 };
        for _ in 0..steps {
            match stat {
                STAT_RUNNING => self.add_stats(1, 0, 0),
                STAT_STRENGTH => self.add_stats(0, 1, 0),
                _ => self.add_stats(0, 0, 1),
            }
        }
    }

    // Recolours the two skin and fur ramps of a palette from black toward the chosen colours
    pub fn recolor_palette(&self, pal: &mut Palette24) {
        palette_gradient(pal, 1, 0x50, [0, 0, 0], self.flesh);
        palette_gradient(pal, 0x51, 0xa0, [0, 0, 0], self.hair);
    }
}

// Fills palette entries first..=last with a single precision ramp truncated per step
pub fn palette_gradient(pal: &mut Palette24, first: u16, last: u16, from: [u8; 3], to: [u8; 3]) {
    let n = (last as i32 - first as i32) as f32;
    let step: [f32; 3] = [
        ((to[0] as i32 - from[0] as i32) as f64 / n as f64) as f32,
        ((to[1] as i32 - from[1] as i32) as f64 / n as f64) as f32,
        ((to[2] as i32 - from[2] as i32) as f64 / n as f64) as f32,
    ];
    let mut cur = [from[0] as f32, from[1] as f32, from[2] as f32];
    for i in first as usize..=last as usize {
        pal.rgb[i] = [ftol(cur[0] as f64) as u8, ftol(cur[1] as f64) as u8, ftol(cur[2] as f64) as u8];
        cur[0] += step[0];
        cur[1] += step[1];
        cur[2] += step[2];
    }
}

// Encodes the record with its magic and size header for a .crn file
pub fn encode_save(player: &PlayerData) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + SAVE_SIZE as usize);
    out.extend_from_slice(&SAVE_MAGIC.to_le_bytes());
    out.extend_from_slice(&SAVE_SIZE.to_le_bytes());
    out.extend_from_slice(&player.to_bytes());
    out
}

// Decodes a .crn file, rejecting a bad magic or size like the original loader
pub fn decode_save(data: &[u8]) -> Option<PlayerData> {
    if data.len() < 8 {
        return None;
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if magic != SAVE_MAGIC || size != SAVE_SIZE {
        return None;
    }
    let body: [u8; 0x50] = data.get(8..8 + SAVE_SIZE as usize)?.try_into().ok()?;
    Some(PlayerData::from_bytes(&body))
}

// Writes the player to its file, allocating the first free Player_N.crn when none is named
pub fn save_player<H: Host>(host: &mut H, player: &PlayerData, save_name: &mut String) -> bool {
    if save_name.is_empty() {
        let mut n = 1;
        loop {
            let candidate = format!("Player_{n}.crn");
            if !host.save_exists(&candidate) {
                *save_name = candidate;
                break;
            }
            n += 1;
        }
    }
    let name = avmanager::crt::with_extension(save_name, ".crn");
    host.write_save(&name, &encode_save(player))
}

pub fn load_player<H: Host>(host: &mut H, name: &str) -> Option<PlayerData> {
    let file = avmanager::crt::with_extension(name, ".crn");
    decode_save(&host.read_save(&file)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_round_trip() {
        let mut p = PlayerData::default();
        p.set_name("spaz");
        p.stage = 1;
        p.kind = 2;
        p.flesh = [0xf8, 0xc8, 0x30];
        p.hair = [0x70, 0x28, 0x00];
        p.running = 5;
        p.throw_high_score = 10;
        p.run_best_secs = 100;
        p.lagging_stat = -1;
        let b = p.to_bytes();
        assert_eq!(PlayerData::from_bytes(&b), p);
        assert_eq!(decode_save(&encode_save(&p)), Some(p));
    }

    #[test]
    fn anim_names_follow_stage_and_type() {
        let mut p = PlayerData::default();
        assert_eq!(p.anim_name("blink", false, false), "b_blink");
        p.stage = 1;
        p.kind = 1;
        assert_eq!(p.anim_name("blink", false, false), "t2_blink");
        p.transform_pending = 1;
        assert_eq!(p.anim_name("transform", true, false), "b2_transform");
        assert_eq!(p.anim_index(false), 1);
        p.transform_pending = 0;
        p.stage = 2;
        p.kind = 2;
        assert_eq!(p.anim_name("stance", false, true), "a_stance");
        assert_eq!(p.anim_index(false), 8);
    }

    #[test]
    fn shown_stage_lags_a_pending_transform() {
        let mut p = PlayerData { stage: 2, ..Default::default() };
        assert_eq!(p.shown_stage(), 2);
        p.transform_pending = 1;
        assert_eq!(p.shown_stage(), 1);
        p.stage = 0;
        assert_eq!(p.shown_stage(), 0);
    }

    #[test]
    fn stats_cap_and_evolve() {
        let mut p = PlayerData::default();
        p.add_stats(9, 0, 0);
        assert_eq!((p.running, p.lagging_stat), (8, STAT_STRENGTH));
        p.strength = 7;
        p.jumping = 7;
        p.add_stats(0, 0, 0);
        assert_eq!((p.stage, p.transform_pending), (1, 1));
    }

    #[test]
    fn training_pays_the_stat_and_spends_only_on_the_way_home() {
        let mut p = PlayerData { food: 3, happiness: 0, ..Default::default() };
        p.train(STAT_RUNNING, true, true);
        assert_eq!((p.food, p.happiness, p.running), (2, 0, 2));
        p.train(STAT_STRENGTH, false, false);
        assert_eq!((p.food, p.strength), (2, 1));
        p.train(STAT_JUMPING, false, true);
        assert_eq!((p.food, p.jumping), (1, 1));
    }

    #[test]
    fn gradient_truncates_toward_zero() {
        let mut pal = Palette24::default();
        palette_gradient(&mut pal, 1, 0x50, [0, 0, 0], [0xf8, 0xc8, 0x30]);
        assert_eq!(pal.rgb[1], [0, 0, 0]);
        assert_eq!(pal.rgb[0x50][0], 247);
        assert_eq!(pal.rgb[0], [0, 0, 0]);
        assert_eq!(pal.rgb[0x51], [0, 0, 0]);
    }
}
