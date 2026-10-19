use avmanager::anim::STATE_RESTART_FORWARD;
use avmanager::input::Input;
use avmanager::texture::Palette24;

use crate::actors::{EVENT_ANIM_END, EVENT_INIT_OBJECT};
use crate::ctx::{
    Ctx, KEY_BACKSPACE, KEY_DEL, KEY_ESC, MOUSE_LEFT, RESULT_CONTINUE, RESULT_QUIT, STATE_HOME_WORLD, Screen,
};
use crate::host::Host;
use crate::scene::{HOVER_NORMAL, HOVER_PRESSED};

const ID_NAME: i32 = 0x10;
const ID_DONE: i32 = 0x11;
const ID_GOBACK: i32 = 0x12;

const SPRITE_PREVIEW: i32 = 3;
const SPRITE_SPRAY_FUR: i32 = 4;
const SPRITE_SPRAY_SKIN: i32 = 5;
const SPRITE_CAPTAIN: i32 = 6;
const SPRITE_FAMILY: i32 = 7;

const ANIM_COUNT: usize = 23;
const ANIM_T_SEL_1: usize = 0;
const ANIM_T_SEL_4: usize = 3;
const ANIM_T2_SEL_2: usize = 7;
const ANIM_T3_SEL_2: usize = 14;
const ANIM_FAMILY_LINES: usize = 18;
const ANIM_SPRAY_R: usize = 19;
const ANIM_SPRAY_L: usize = 20;
const ANIM_CAP006: usize = 22;

const SCROLL_MAX: i16 = 0x280;
const SCROLL_SPEED: i16 = 0x20;

// Scene object names with the ids Init gives them, in the original call order
const SCENE_IDS: [(&str, i32); 18] = [
    ("border1", 1),
    ("border2", 2),
    ("border3", 3),
    ("fur_blue", 4),
    ("fur_pink", 5),
    ("fur_red", 6),
    ("fur_green", 7),
    ("fur_purple", 8),
    ("fur_brown", 9),
    ("skin_blue", 10),
    ("skin_pink", 0xb),
    ("skin_orange", 0xc),
    ("skin_green", 0xd),
    ("skin_purple", 0xe),
    ("skin_yellow", 0xf),
    ("done", ID_DONE),
    ("goback", ID_GOBACK),
    ("name", ID_NAME),
];

// Animation loads in the original call order as handle slot, file name and retain image flag
const ANIM_LOADS: [(usize, &str, bool); ANIM_COUNT] = [
    (0, "t_sel_1", true),
    (1, "t_sel_2", true),
    (2, "t_sel_2", true),
    (3, "t_sel_4", true),
    (4, "t_sel_5", true),
    (5, "t_sel_4", true),
    (6, "t2_sel_1", true),
    (7, "t2_sel_2", true),
    (8, "t2_sel_2", true),
    (9, "t2_sel_4", true),
    (10, "t2_sel_5", true),
    (11, "t2_sel_4", true),
    (12, "t3_sel_1", true),
    (13, "t3_sel_2", true),
    (14, "t3_sel_2", true),
    (15, "t3_sel_4", true),
    (16, "t3_sel_5", true),
    (17, "t3_sel_4", true),
    (18, "Family_lines", true),
    (21, "cap_tv_cap005", false),
    (22, "cap_tv_cap006", false),
    (19, "spray_r", true),
    (20, "spray_l", true),
];

// Which follow up block a clicked scene id falls into
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClickGroup {
    None,
    Type,
    Fur,
    Skin,
}

// The crunchling creator, picking a family and spraying colours before naming it
pub struct CreateScreen {
    mouse_anim: u16,
    scroll_x: i16,
    scroll_speed: i16,
    blink: i16,
    music: u32,
    speech: u32,
    sel_type: i32,
    pending_sel: i32,
    fur_spraying: bool,
    skin_spraying: bool,
    done: bool,
    anims: [u16; ANIM_COUNT],
    sfx_button: u16,
    sfx_yahoo: u16,
    sfx_spray: u16,
    sfx_key: u16,
    sfx_swoosh: u16,
}

impl Default for CreateScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateScreen {
    pub fn new() -> Self {
        Self {
            mouse_anim: 0,
            scroll_x: 0,
            scroll_speed: 0,
            blink: 0,
            music: 0,
            speech: 0,
            sel_type: -1,
            pending_sel: -1,
            fur_spraying: false,
            skin_spraying: false,
            done: false,
            anims: [0; ANIM_COUNT],
            sfx_button: 0,
            sfx_yahoo: 0,
            sfx_spray: 0,
            sfx_key: 0,
            sfx_swoosh: 0,
        }
    }

    // Recolours the t_sel_1 palette with the player colours and installs it on all 19 family anims
    fn apply_player_palette<H: Host>(&self, ctx: &mut Ctx<H>) {
        let mut pal = Palette24::default();
        ctx.av.get_anim_palette(self.anims[ANIM_T_SEL_1], &mut pal);
        ctx.player.recolor_palette(&mut pal);
        for &handle in &self.anims[..=ANIM_FAMILY_LINES] {
            ctx.av.set_anim_palette(handle, &pal);
        }
    }

    // Handles a click on a border, colour swatch, done or goback object and always returns 1
    async fn on_object_clicked<H: Host>(&mut self, ctx: &mut Ctx<H>, id: i32) -> u32 {
        let group = match id {
            1..=3 => ClickGroup::Type,
            4 => {
                rgb_set(&mut ctx.player.hair, 0, 0, 0xff);
                ClickGroup::Fur
            }
            5 => {
                rgb_set(&mut ctx.player.hair, 0xe0, 0, 0x78);
                ClickGroup::Fur
            }
            6 => {
                rgb_set(&mut ctx.player.hair, 0xff, 0, 0);
                ClickGroup::Fur
            }
            7 => {
                rgb_set(&mut ctx.player.hair, 0x20, 0x6c, 0x38);
                ClickGroup::Fur
            }
            8 => {
                rgb_set(&mut ctx.player.hair, 0x80, 4, 0xd8);
                ClickGroup::Fur
            }
            9 => {
                rgb_set(&mut ctx.player.hair, 0x70, 0x28, 0);
                ClickGroup::Fur
            }
            10 => {
                rgb_set(&mut ctx.player.flesh, 0x10, 0x90, 0xf8);
                ClickGroup::Skin
            }
            0xb => {
                rgb_set(&mut ctx.player.flesh, 0xf8, 0x94, 200);
                ClickGroup::Skin
            }
            0xc => {
                rgb_set(&mut ctx.player.flesh, 0xf8, 0x94, 0x20);
                ClickGroup::Skin
            }
            0xd => {
                rgb_set(&mut ctx.player.flesh, 0x20, 0xe8, 0x20);
                ClickGroup::Skin
            }
            0xe => {
                rgb_set(&mut ctx.player.flesh, 0xc0, 0x80, 0xf8);
                ClickGroup::Skin
            }
            0xf => {
                rgb_set(&mut ctx.player.flesh, 0xf8, 200, 0x30);
                ClickGroup::Skin
            }
            ID_DONE => {
                if !self.done {
                    ctx.av.play_sfx(self.sfx_button, -1);
                    let cap = self.anims[ANIM_CAP006];
                    ctx.actors.set_anims(&ctx.av, SPRITE_CAPTAIN, cap, cap, false, STATE_RESTART_FORWARD);
                    self.speech = ctx.av.play_stream_sound(&mut ctx.host, "002.wav", false, 0).await;
                    ctx.actors.set_visible(SPRITE_CAPTAIN, true);
                    self.done = true;
                }
                ClickGroup::None
            }
            ID_GOBACK => {
                ctx.av.play_sfx(self.sfx_swoosh, -1);
                for border in 1..=3 {
                    ctx.scene.set_hover_by_id(border, HOVER_NORMAL);
                }
                ctx.cursor.set_visible(false);
                ctx.actors.set_visible(SPRITE_CAPTAIN, false);
                self.scroll_speed = -SCROLL_SPEED;
                ClickGroup::None
            }
            _ => ClickGroup::None,
        };
        match group {
            ClickGroup::Type => {
                ctx.av.kill_streaming_channel(self.speech);
                for border in 1..=3 {
                    ctx.scene.set_hover_by_id(border, HOVER_NORMAL);
                }
                ctx.scene.set_hover_by_id(id, HOVER_PRESSED);
                ctx.av.play_sfx(self.sfx_yahoo, -1);
                self.pending_sel = id - 1;
                self.sel_type = self.pending_sel;
                let sel5 = self.anims[sel_5_anim_index(self.pending_sel)];
                ctx.actors.set_anims(&ctx.av, self.pending_sel, sel5, sel5, true, STATE_RESTART_FORWARD);
                ctx.actors.set_anims(&ctx.av, SPRITE_PREVIEW, sel5, sel5, true, STATE_RESTART_FORWARD);
                ctx.cursor.set_visible(false);
            }
            ClickGroup::Fur => {
                if !self.fur_spraying {
                    ctx.av.play_sfx(self.sfx_spray, -1);
                    for fur in 4..=9 {
                        ctx.scene.set_hover_by_id(fur, HOVER_NORMAL);
                    }
                    ctx.scene.set_hover_by_id(id, HOVER_PRESSED);
                    let spray = self.anims[ANIM_SPRAY_R];
                    let mut pal = Palette24::default();
                    ctx.av.get_anim_palette(spray, &mut pal);
                    rgb_set(&mut pal.rgb[0], 0, 0xff, 0);
                    pal.rgb[1] = ctx.player.hair;
                    ctx.av.set_anim_palette(spray, &pal);
                    ctx.actors.set_anims(&ctx.av, SPRITE_SPRAY_FUR, spray, spray, true, STATE_RESTART_FORWARD);
                    ctx.actors.set_visible(SPRITE_SPRAY_FUR, true);
                    self.apply_player_palette(ctx);
                    self.fur_spraying = true;
                }
            }
            ClickGroup::Skin => {
                if !self.skin_spraying {
                    ctx.av.play_sfx(self.sfx_spray, -1);
                    for skin in 10..=0xf {
                        ctx.scene.set_hover_by_id(skin, HOVER_NORMAL);
                    }
                    ctx.scene.set_hover_by_id(id, HOVER_PRESSED);
                    let spray = self.anims[ANIM_SPRAY_L];
                    let mut pal = Palette24::default();
                    ctx.av.get_anim_palette(spray, &mut pal);
                    rgb_set(&mut pal.rgb[0], 0, 0xff, 0);
                    pal.rgb[1] = ctx.player.flesh;
                    ctx.av.set_anim_palette(spray, &pal);
                    ctx.actors.set_anims(&ctx.av, SPRITE_SPRAY_SKIN, spray, spray, true, STATE_RESTART_FORWARD);
                    ctx.actors.set_visible(SPRITE_SPRAY_SKIN, true);
                    self.apply_player_palette(ctx);
                    self.skin_spraying = true;
                }
            }
            ClickGroup::None => {}
        }
        RESULT_CONTINUE
    }

    async fn on_sprite_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, id: i32) {
        if kind == EVENT_INIT_OBJECT {
            self.on_sprite_init(ctx, id).await;
        } else if kind == EVENT_ANIM_END {
            self.on_sprite_anim_end(ctx, id);
        }
    }

    // Places each sprite with its first animation and starts the captain's 001.wav intro
    async fn on_sprite_init<H: Host>(&mut self, ctx: &mut Ctx<H>, id: i32) {
        match id {
            0 => {
                let idle = self.anims[ANIM_T_SEL_1];
                ctx.actors.set_anims(&ctx.av, id, idle, idle, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(id, 65.0, 280.0);
            }
            1 => {
                let idle = self.anims[ANIM_T2_SEL_2];
                ctx.actors.set_anims(&ctx.av, id, idle, idle, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(id, 265.0, 280.0);
            }
            2 => {
                let idle = self.anims[ANIM_T3_SEL_2];
                ctx.actors.set_anims(&ctx.av, id, idle, idle, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(id, 465.0, 280.0);
            }
            SPRITE_PREVIEW => {
                let idle = self.anims[ANIM_T_SEL_4];
                ctx.actors.set_anims(&ctx.av, id, idle, idle, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(id, 700.0, 220.0);
            }
            SPRITE_SPRAY_FUR => {
                let spray = self.anims[ANIM_SPRAY_R];
                ctx.actors.set_anims(&ctx.av, id, spray, spray, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(id, 702.0, 164.0);
                ctx.actors.set_visible(id, false);
            }
            SPRITE_SPRAY_SKIN => {
                let spray = self.anims[ANIM_SPRAY_L];
                ctx.actors.set_anims(&ctx.av, id, spray, spray, true, STATE_RESTART_FORWARD);
                ctx.actors.set_pos(id, 684.0, 164.0);
                ctx.actors.set_visible(id, false);
            }
            SPRITE_CAPTAIN => {
                ctx.actors.set_pos(id, 495.0, 15.0);
                ctx.actors.set_screen_fixed(id, true);
                let cap = self.anims[ANIM_CAP006];
                ctx.actors.set_anims(&ctx.av, id, cap, cap, false, STATE_RESTART_FORWARD);
                self.speech = ctx.av.play_stream_sound(&mut ctx.host, "001.wav", false, 0).await;
            }
            SPRITE_FAMILY => {
                ctx.actors.set_pos(id, 57.0, 124.0);
                let lines = self.anims[ANIM_FAMILY_LINES];
                ctx.actors.set_anims(&ctx.av, id, lines, lines, false, STATE_RESTART_FORWARD);
            }
            _ => {}
        }
    }

    // Rolls the next idle pose for a family sprite and starts the scroll once the picked one finishes
    fn on_sprite_anim_end<H: Host>(&mut self, ctx: &mut Ctx<H>, id: i32) {
        let base = match id {
            0 => Some(0),
            1 => Some(6),
            2 => Some(12),
            SPRITE_PREVIEW => Some(idle_base(self.sel_type)),
            SPRITE_SPRAY_FUR => {
                self.fur_spraying = false;
                ctx.actors.set_visible(id, false);
                None
            }
            SPRITE_SPRAY_SKIN => {
                self.skin_spraying = false;
                ctx.actors.set_visible(id, false);
                None
            }
            _ => None,
        };
        if let Some(base) = base {
            let idle = self.anims[base + idle_anim_offset(ctx.rand_range(0, 99))];
            ctx.actors.set_anims(&ctx.av, id, idle, idle, true, STATE_RESTART_FORWARD);
            if self.pending_sel == id {
                self.pending_sel = -1;
                self.scroll_speed = SCROLL_SPEED;
                ctx.av.play_sfx(self.sfx_swoosh, -1);
                ctx.actors.set_visible(SPRITE_CAPTAIN, false);
            }
        }
    }
}

impl<H: Host> Screen<H> for CreateScreen {
    // Loads select.wam and every asset, resets the player record and builds the eight sprites
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        let Some(mouse) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        self.mouse_anim = mouse;
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "select").await;
        ctx.globals.save_name.clear();
        let player = &mut ctx.player;
        player.running = 0;
        player.jumping = 0;
        player.strength = 0;
        player.food = 0;
        player.happiness = 0;
        player.stage = 0;
        player.kind = 0;
        player.transform_pending = 0;
        player.lagging_stat = -1;
        player.first_home_visit = 1;
        player.tips_index = 0;
        player.run_best_frac = 0;
        player.run_best_secs = 100;
        player.jump_best_frac = 0;
        player.jump_best_secs = 100;
        player.throw_high_score = 10;
        player.challenge_active = 0;
        player.challenge_won = 0;
        rgb_set(&mut player.flesh, 0xf8, 200, 0x30);
        rgb_set(&mut player.hair, 0x70, 0x28, 0);
        player.set_name("spaz");
        for (name, id) in SCENE_IDS {
            ctx.scene.assign_id(name, id);
        }
        ctx.scene.set_text_by_id(ID_NAME, "");
        ctx.cursor.set_anim(&ctx.av, self.mouse_anim);
        ctx.cursor.set_visible(true);
        ctx.cursor.set_pos(0x140, 100);
        let Some(button) = ctx.av.get_sfx(&mut ctx.host, "CS_Button.wav", 0, false).await else { return false };
        self.sfx_button = button;
        let Some(yahoo) = ctx.av.get_sfx(&mut ctx.host, "CS_yahoo.wav", 0, false).await else { return false };
        self.sfx_yahoo = yahoo;
        let Some(spray) = ctx.av.get_sfx(&mut ctx.host, "CS_spray.wav", 0, false).await else { return false };
        self.sfx_spray = spray;
        let Some(key) = ctx.av.get_sfx(&mut ctx.host, "CS_key.wav", 0, false).await else { return false };
        self.sfx_key = key;
        let Some(swoosh) = ctx.av.get_sfx(&mut ctx.host, "CS_swoosh.wav", 0, false).await else { return false };
        self.sfx_swoosh = swoosh;
        self.music = ctx.av.play_stream_sound(&mut ctx.host, "crunchsong3.wav", true, 0).await;
        for (slot, name, retain) in ANIM_LOADS {
            let loaded = if retain {
                ctx.av.get_anim_retain_image(&mut ctx.host, name).await
            } else {
                ctx.av.get_anim(&mut ctx.host, name).await
            };
            let Some(handle) = loaded else { return false };
            self.anims[slot] = handle;
        }
        ctx.actors.clear();
        for id in [0, 1, 2, SPRITE_PREVIEW, SPRITE_CAPTAIN, SPRITE_FAMILY, SPRITE_SPRAY_FUR, SPRITE_SPRAY_SKIN] {
            ctx.actors.add_actor(id, true);
        }
        self.apply_player_palette(ctx);
        ctx.scene.set_hover_by_id(9, HOVER_PRESSED);
        ctx.scene.set_hover_by_id(0xf, HOVER_PRESSED);
        self.fur_spraying = false;
        self.skin_spraying = false;
        self.scroll_x = 0;
        self.scroll_speed = 0;
        self.blink = 0;
        self.done = false;
        self.pending_sel = -1;
        true
    }

    // Draws, scrolls or takes mouse and name input, then leaves for HomeWorld once the done speech ends
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut result = RESULT_CONTINUE;
        ctx.scene.draw_layer(&mut ctx.av, 0);
        ctx.actors.update(&ctx.av);
        while let Some(event) = ctx.actors.pop_event() {
            self.on_sprite_event(ctx, event.kind, event.actor_id).await;
        }
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 0xf);
        ctx.actors.draw(&mut ctx.av, &ctx.scene, 4);
        if !ctx.cursor.visible {
            self.scroll_x = self.scroll_x.wrapping_add(self.scroll_speed);
            if self.scroll_x < SCROLL_MAX + 1 {
                if self.scroll_x < 0 {
                    ctx.cursor.set_visible(true);
                    if self.speech != 0 {
                        ctx.actors.set_visible(SPRITE_CAPTAIN, true);
                    }
                    self.scroll_speed = 0;
                    self.scroll_x = 0;
                }
            } else {
                ctx.cursor.set_visible(true);
                if self.speech != 0 {
                    ctx.actors.set_visible(SPRITE_CAPTAIN, true);
                }
                self.scroll_speed = 0;
                self.scroll_x = SCROLL_MAX;
            }
            ctx.scene.set_scroll(self.scroll_x as i32, 0);
        } else {
            let (dx, dy) = ctx.av.get_mouse_delta();
            ctx.cursor.move_by(dx, dy);
            let (mouse_x, mouse_y) = (ctx.cursor.x, ctx.cursor.y);
            ctx.cursor.draw(&mut ctx.av);
            if let Some(id) = ctx.scene.hit_test(mouse_x as i32, mouse_y as i32)
                && ctx.av.is_mouse_button_down(MOUSE_LEFT)
            {
                result = self.on_object_clicked(ctx, id).await;
            }
            let key = ctx.av.key_input();
            if key != 0 {
                ctx.av.play_sfx(self.sfx_key, -1);
                if key == KEY_BACKSPACE || key == KEY_DEL {
                    if let Some(mut text) = ctx.scene.text_by_id(ID_NAME)
                        && !text.is_empty()
                    {
                        text.pop();
                        ctx.scene.set_text_by_id(ID_NAME, &text);
                    }
                } else if let Some(name) = Input::translate_key_def(key)
                    && name.len() == 1
                    && accepts_name_char(name.as_bytes()[0])
                    && let Some(mut text) = ctx.scene.text_by_id(ID_NAME)
                {
                    text.push_str(name);
                    ctx.scene.set_text_by_id(ID_NAME, &text);
                }
            }
            self.blink += 1;
            if self.blink > 0x10 {
                self.blink = 0;
            }
            ctx.scene.set_text_cursor_by_id(ID_NAME, if self.blink > 0xb { 2 } else { 1 });
        }
        if self.speech != 0 && ctx.av.is_stream_sound_done(self.speech) {
            self.speech = 0;
            ctx.actors.set_visible(SPRITE_CAPTAIN, false);
            if self.done {
                match ctx.scene.text_by_id(ID_NAME) {
                    Some(text) if !text.is_empty() => ctx.player.set_name(&text),
                    _ => ctx.player.set_name("spaz"),
                }
                ctx.player.kind = self.sel_type as i16;
                ctx.globals.player_loaded = true;
                result = STATE_HOME_WORLD;
            }
        }
        ctx.scene.advance_anims(&ctx.av);
        if ctx.av.is_key_down(KEY_ESC) {
            result = RESULT_QUIT;
        }
        result
    }

    fn shutdown(&mut self, ctx: &mut Ctx<H>) {
        ctx.av.kill_streaming_channel(self.music);
        ctx.av.kill_streaming_channel(self.speech);
    }
}

// Stores a colour triple the way the RGB_Set helper did
fn rgb_set(rgb: &mut [u8; 3], r: u8, g: u8, b: u8) {
    rgb[0] = r;
    rgb[1] = g;
    rgb[2] = b;
}

// Handle slot of a family's sel_5 pose, 4, 10 or 16
fn sel_5_anim_index(sel_type: i32) -> usize {
    match sel_type {
        1 => 10,
        2 => 0x10,
        _ => 4,
    }
}

// Handle slot where a family's six poses start, 0, 6 or 12
fn idle_base(sel_type: i32) -> usize {
    match sel_type {
        1 => 6,
        2 => 12,
        _ => 0,
    }
}

// Idle pose offset for a 0 to 99 roll, thresholds at 30, 45, 60 and 75
fn idle_anim_offset(roll: i32) -> usize {
    if roll < 0x1e {
        3
    } else if roll < 0x2d {
        0
    } else if roll < 0x3c {
        1
    } else if roll < 0x4b {
        2
    } else if roll < 100 {
        5
    } else {
        unreachable!("RandRange(0, 99) never reaches 100")
    }
}

// True for the single characters the name field accepts, space, capital letters and digits
fn accepts_name_char(c: u8) -> bool {
    c == b' ' || c.is_ascii_uppercase() || c.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use avmanager::input::DeviceState;
    use avmanager::sound::{INVALID_HANDLE, Sound};

    use super::*;

    const DATA_DIR: &str = "/home/nick/code/crunchatize/dev/disc/iso/CrunchData";
    const SCAN_ESC: usize = 1;
    const SCAN_1: usize = 0x02;
    const SCAN_BACKSPACE: usize = 0x0e;
    const SCAN_A: usize = 0x1e;
    const SCAN_Z: usize = 0x2c;
    const SCAN_B: usize = 0x30;
    const SCAN_SPACE: usize = 0x39;
    const SCAN_DEL: usize = 0xd3;
    const SAMPLES_PER_TICK: usize = 2 * 22050 / 30;

    // Host serving the disc data directory with in memory saves and audio rendered per frame
    #[derive(Default)]
    struct DiscHost {
        ticks: u32,
        device: DeviceState,
        saves: HashMap<String, Vec<u8>>,
    }

    impl Host for DiscHost {
        async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
            let entry = std::fs::read_dir(DATA_DIR)
                .unwrap()
                .filter_map(Result::ok)
                .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))?;
            std::fs::read(entry.path()).ok()
        }

        fn time_ms(&self) -> u32 {
            self.ticks * 33
        }

        fn unix_time(&self) -> u32 {
            0
        }

        fn poll_input(&mut self) -> DeviceState {
            let state = self.device.clone();
            self.device.mouse_dx = 0;
            self.device.mouse_dy = 0;
            state
        }

        async fn end_frame(&mut self, _frame: Option<&[u16]>, sound: &mut Sound) {
            self.ticks += 1;
            sound.heartbeat();
            let mut out = vec![0i16; SAMPLES_PER_TICK];
            sound.render(&mut out);
        }

        fn list_saves(&mut self) -> Vec<String> {
            self.saves.keys().cloned().collect()
        }

        fn read_save(&mut self, name: &str) -> Option<Vec<u8>> {
            self.saves.get(name).cloned()
        }

        fn write_save(&mut self, name: &str, data: &[u8]) -> bool {
            self.saves.insert(name.to_string(), data.to_vec());
            true
        }

        fn save_exists(&mut self, name: &str) -> bool {
            self.saves.contains_key(name)
        }

        fn delete_save(&mut self, name: &str) {
            self.saves.remove(name);
        }

        fn take_debug_toggle(&mut self) -> bool {
            false
        }

        fn quit_requested(&self) -> bool {
            false
        }

        fn open_url(&mut self, _url: &str) {}

        fn launch_setup(&mut self) {}

        fn log(&mut self, _message: &str) {}
    }

    fn block_on<F: Future>(fut: F) -> F::Output {
        let mut fut = pin!(fut);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    fn start() -> (CreateScreen, Ctx<DiscHost>) {
        assert!(Path::new(DATA_DIR).is_dir(), "{DATA_DIR} must hold the CrunchData files");
        let mut ctx = Ctx::new(DiscHost::default(), 22050);
        let mut screen = CreateScreen::new();
        assert!(block_on(screen.init(&mut ctx)));
        (screen, ctx)
    }

    // One game tick the way the main loop runs it, input, update, then the frame flip
    fn tick(screen: &mut CreateScreen, ctx: &mut Ctx<DiscHost>) -> u32 {
        block_on(async {
            let device = ctx.host.poll_input();
            ctx.av.input.heartbeat(&device);
            let result = screen.update(ctx).await;
            ctx.av.swap_buffers(&mut ctx.host).await;
            result
        })
    }

    fn move_cursor(ctx: &mut Ctx<DiscHost>, x: i16, y: i16) {
        ctx.host.device.mouse_dx = (x - ctx.cursor.x) as i32 * 2;
        ctx.host.device.mouse_dy = (y - ctx.cursor.y) as i32 * 2;
    }

    fn click(screen: &mut CreateScreen, ctx: &mut Ctx<DiscHost>, x: i16, y: i16) -> u32 {
        move_cursor(ctx, x, y);
        ctx.host.device.buttons[0] = true;
        let result = tick(screen, ctx);
        ctx.host.device.buttons[0] = false;
        result
    }

    fn press(screen: &mut CreateScreen, ctx: &mut Ctx<DiscHost>, scan: usize) {
        ctx.host.device.scan_keys[scan] = true;
        assert_eq!(tick(screen, ctx), RESULT_CONTINUE);
        ctx.host.device.scan_keys[scan] = false;
        assert_eq!(tick(screen, ctx), RESULT_CONTINUE);
    }

    fn run_until_cursor_visible(screen: &mut CreateScreen, ctx: &mut Ctx<DiscHost>) {
        let mut ticks = 0;
        while !ctx.cursor.visible {
            assert_eq!(tick(screen, ctx), RESULT_CONTINUE);
            ticks += 1;
            assert!(ticks < 2000, "the scroll never finished");
        }
    }

    fn hover_of(ctx: &Ctx<DiscHost>, id: i32) -> i32 {
        ctx.scene.objects.iter().find(|o| o.id == id).expect("scene id missing").hover
    }

    fn name_text(ctx: &Ctx<DiscHost>) -> String {
        ctx.scene.text_by_id(ID_NAME).expect("name object missing")
    }

    #[test]
    fn init_resets_the_player_and_prepares_the_scene() {
        let (screen, ctx) = start();
        let p = &ctx.player;
        assert_eq!(p.name_str(), "spaz");
        assert_eq!((p.kind, p.stage), (0, 0));
        assert_eq!(p.flesh, [0xf8, 0xc8, 0x30]);
        assert_eq!(p.hair, [0x70, 0x28, 0x00]);
        assert_eq!((p.running, p.jumping, p.strength), (0, 0, 0));
        assert_eq!((p.food, p.happiness), (0, 0));
        assert_eq!((p.run_best_frac, p.run_best_secs), (0, 100));
        assert_eq!((p.jump_best_frac, p.jump_best_secs), (0, 100));
        assert_eq!(p.throw_high_score, 10);
        assert_eq!((p.challenge_active, p.challenge_won), (0, 0));
        assert_eq!((p.transform_pending, p.lagging_stat), (0, -1));
        assert_eq!((p.first_home_visit, p.tips_index), (1, 0));
        assert!(ctx.globals.save_name.is_empty());
        assert!(!ctx.globals.player_loaded);
        for (_, id) in SCENE_IDS {
            assert!(ctx.scene.objects.iter().any(|o| o.id == id), "scene id {id} unassigned");
        }
        assert_eq!(name_text(&ctx), "");
        assert_eq!((hover_of(&ctx, 9), hover_of(&ctx, 0xf)), (HOVER_PRESSED, HOVER_PRESSED));
        assert_eq!(hover_of(&ctx, 4), HOVER_NORMAL);
        assert!(ctx.cursor.visible);
        assert_eq!((ctx.cursor.x, ctx.cursor.y), (0x140, 100));
        assert_eq!(ctx.cursor.anim.anim, screen.mouse_anim);
        assert_ne!(screen.music, INVALID_HANDLE);
        assert_eq!(ctx.actors.actors.iter().map(|a| a.id).collect::<Vec<_>>(), vec![0, 1, 2, 3, 6, 7, 4, 5]);
        assert_eq!(ctx.actors.events.len(), 8);
        for pair in [(1, 2), (3, 5), (7, 8), (9, 11), (13, 14), (15, 17)] {
            assert_eq!(screen.anims[pair.0], screen.anims[pair.1]);
        }
        assert_ne!(screen.anims[ANIM_T_SEL_1], screen.anims[ANIM_T2_SEL_2]);
        assert_eq!((screen.scroll_x, screen.scroll_speed, screen.blink), (0, 0, 0));
        assert_eq!((screen.sel_type, screen.pending_sel), (-1, -1));
        assert!(!screen.fur_spraying && !screen.skin_spraying && !screen.done);
    }

    #[test]
    fn picking_colouring_naming_and_done_reach_home_world() {
        let (mut screen, mut ctx) = start();
        assert_eq!(tick(&mut screen, &mut ctx), RESULT_CONTINUE);
        assert!(screen.speech != 0 && screen.speech != INVALID_HANDLE);
        let captain = ctx.actors.get(SPRITE_CAPTAIN);
        assert!(captain.screen_fixed && captain.visible);
        assert_eq!((captain.x, captain.y), (495.0, 15.0));
        assert_eq!(captain.anim.anim, screen.anims[ANIM_CAP006]);
        assert_eq!(ctx.actors.pos(0), (65.0, 280.0));
        assert_eq!(ctx.actors.pos(1), (265.0, 280.0));
        assert_eq!(ctx.actors.pos(2), (465.0, 280.0));
        assert_eq!(ctx.actors.pos(SPRITE_PREVIEW), (700.0, 220.0));
        assert_eq!(ctx.actors.pos(SPRITE_SPRAY_FUR), (702.0, 164.0));
        assert_eq!(ctx.actors.pos(SPRITE_SPRAY_SKIN), (684.0, 164.0));
        assert_eq!(ctx.actors.pos(SPRITE_FAMILY), (57.0, 124.0));
        assert!(!ctx.actors.get(SPRITE_SPRAY_FUR).visible && !ctx.actors.get(SPRITE_SPRAY_SKIN).visible);
        assert_eq!(ctx.actors.get(0).anim.anim, screen.anims[ANIM_T_SEL_1]);
        assert_eq!(ctx.actors.get(1).anim.anim, screen.anims[ANIM_T2_SEL_2]);
        assert_eq!(ctx.actors.get(2).anim.anim, screen.anims[ANIM_T3_SEL_2]);
        assert_eq!(ctx.actors.get(SPRITE_PREVIEW).anim.anim, screen.anims[ANIM_T_SEL_4]);
        assert_eq!(ctx.actors.get(SPRITE_FAMILY).anim.anim, screen.anims[ANIM_FAMILY_LINES]);

        assert_eq!(click(&mut screen, &mut ctx, 300, 300), RESULT_CONTINUE);
        assert!(!ctx.cursor.visible);
        assert_eq!((screen.sel_type, screen.pending_sel), (1, 1));
        assert_eq!(hover_of(&ctx, 2), HOVER_PRESSED);
        assert_eq!(ctx.actors.get(1).anim.anim, screen.anims[10]);
        assert_eq!(ctx.actors.get(SPRITE_PREVIEW).anim.anim, screen.anims[10]);
        assert!(ctx.actors.get(1).notify_anim_end);
        assert_eq!(screen.speech, 0);
        assert!(!ctx.actors.get(SPRITE_CAPTAIN).visible);
        run_until_cursor_visible(&mut screen, &mut ctx);
        assert_eq!((screen.scroll_x, screen.scroll_speed), (SCROLL_MAX, 0));
        assert_eq!((ctx.scene.scroll_x, ctx.scene.scroll_y), (640, 0));
        assert_eq!(screen.pending_sel, -1);
        assert!(screen.anims[6..12].contains(&ctx.actors.get(1).anim.anim));
        assert!(!ctx.actors.get(SPRITE_CAPTAIN).visible);

        assert_eq!(click(&mut screen, &mut ctx, 290, 287), RESULT_CONTINUE);
        assert_eq!(ctx.player.hair, [0, 0, 0xff]);
        assert!(screen.fur_spraying);
        assert_eq!((hover_of(&ctx, 4), hover_of(&ctx, 9)), (HOVER_PRESSED, HOVER_NORMAL));
        let spray = ctx.actors.get(SPRITE_SPRAY_FUR);
        assert!(spray.visible && spray.notify_anim_end);
        assert_eq!(spray.anim.anim, screen.anims[ANIM_SPRAY_R]);
        let mut pal = Palette24::default();
        ctx.av.get_anim_palette(screen.anims[ANIM_SPRAY_R], &mut pal);
        assert_eq!((pal.rgb[0], pal.rgb[1]), ([0, 0xff, 0], [0, 0, 0xff]));
        ctx.av.get_anim_palette(screen.anims[ANIM_FAMILY_LINES], &mut pal);
        assert_eq!((pal.rgb[1], pal.rgb[0x51]), ([0, 0, 0], [0, 0, 0]));
        assert_eq!((pal.rgb[0xa0][0], pal.rgb[0xa0][1]), (0, 0));
        assert!(pal.rgb[0xa0][2] >= 0xfe);
        assert!(pal.rgb[0x50][0] >= 0xf7 && pal.rgb[0x50][1] >= 0xc7);

        assert_eq!(click(&mut screen, &mut ctx, 400, 380), RESULT_CONTINUE);
        assert_eq!(ctx.player.flesh, [0xf8, 0x94, 0x20]);
        assert!(screen.skin_spraying);
        assert_eq!((hover_of(&ctx, 0xc), hover_of(&ctx, 0xf)), (HOVER_PRESSED, HOVER_NORMAL));
        assert_eq!(ctx.actors.get(SPRITE_SPRAY_SKIN).anim.anim, screen.anims[ANIM_SPRAY_L]);
        ctx.av.get_anim_palette(screen.anims[ANIM_SPRAY_L], &mut pal);
        assert_eq!((pal.rgb[0], pal.rgb[1]), ([0, 0xff, 0], [0xf8, 0x94, 0x20]));

        move_cursor(&mut ctx, 320, 100);
        press(&mut screen, &mut ctx, SCAN_A);
        press(&mut screen, &mut ctx, SCAN_B);
        press(&mut screen, &mut ctx, SCAN_SPACE);
        press(&mut screen, &mut ctx, SCAN_1);
        assert_eq!(name_text(&ctx), "AB1");
        for _ in 0..12 {
            press(&mut screen, &mut ctx, SCAN_Z);
        }
        assert_eq!(name_text(&ctx), "AB1ZZZZZZZ");
        press(&mut screen, &mut ctx, SCAN_BACKSPACE);
        assert_eq!(name_text(&ctx), "AB1ZZZZZZ");
        press(&mut screen, &mut ctx, SCAN_DEL);
        assert_eq!(name_text(&ctx), "AB1ZZZZZ");

        assert_eq!(click(&mut screen, &mut ctx, 520, 440), RESULT_CONTINUE);
        assert!(screen.done);
        assert!(screen.speech != 0 && screen.speech != INVALID_HANDLE);
        let captain = ctx.actors.get(SPRITE_CAPTAIN);
        assert!(captain.visible);
        assert_eq!(captain.anim.anim, screen.anims[ANIM_CAP006]);
        assert_eq!(click(&mut screen, &mut ctx, 520, 440), RESULT_CONTINUE);
        let mut result = RESULT_CONTINUE;
        let mut ticks = 0;
        while result == RESULT_CONTINUE {
            result = tick(&mut screen, &mut ctx);
            ticks += 1;
            assert!(ticks < 3000, "the done speech never finished");
        }
        assert_eq!(result, STATE_HOME_WORLD);
        assert_eq!(screen.speech, 0);
        assert!(!ctx.actors.get(SPRITE_CAPTAIN).visible);
        assert_eq!(ctx.player.kind, 1);
        assert_eq!(ctx.player.name_str(), "AB1ZZZZZ");
        assert!(ctx.globals.player_loaded);
        screen.shutdown(&mut ctx);
    }

    #[test]
    fn goback_scrolls_home_and_escape_quits() {
        let (mut screen, mut ctx) = start();
        assert_eq!(tick(&mut screen, &mut ctx), RESULT_CONTINUE);
        assert_eq!(click(&mut screen, &mut ctx, 100, 300), RESULT_CONTINUE);
        assert_eq!((screen.sel_type, screen.pending_sel), (0, 0));
        assert_eq!(ctx.actors.get(0).anim.anim, screen.anims[4]);
        run_until_cursor_visible(&mut screen, &mut ctx);
        assert_eq!(hover_of(&ctx, 1), HOVER_PRESSED);
        assert_eq!(click(&mut screen, &mut ctx, 120, 440), RESULT_CONTINUE);
        assert!(!ctx.cursor.visible);
        assert_eq!(screen.scroll_speed, -SCROLL_SPEED);
        for border in 1..=3 {
            assert_eq!(hover_of(&ctx, border), HOVER_NORMAL);
        }
        assert!(!ctx.actors.get(SPRITE_CAPTAIN).visible);
        run_until_cursor_visible(&mut screen, &mut ctx);
        assert_eq!((screen.scroll_x, screen.scroll_speed), (0, 0));
        assert_eq!(ctx.scene.scroll_x, 0);
        assert!(!screen.done && !ctx.globals.player_loaded);
        ctx.host.device.scan_keys[SCAN_ESC] = true;
        assert_eq!(tick(&mut screen, &mut ctx), RESULT_QUIT);
        screen.shutdown(&mut ctx);
    }

    #[test]
    fn empty_name_falls_back_to_spaz() {
        let (mut screen, mut ctx) = start();
        assert_eq!(tick(&mut screen, &mut ctx), RESULT_CONTINUE);
        assert_eq!(click(&mut screen, &mut ctx, 500, 300), RESULT_CONTINUE);
        run_until_cursor_visible(&mut screen, &mut ctx);
        assert_eq!(click(&mut screen, &mut ctx, 520, 440), RESULT_CONTINUE);
        let mut result = RESULT_CONTINUE;
        let mut ticks = 0;
        while result == RESULT_CONTINUE {
            result = tick(&mut screen, &mut ctx);
            ticks += 1;
            assert!(ticks < 3000, "the done speech never finished");
        }
        assert_eq!(result, STATE_HOME_WORLD);
        assert_eq!(ctx.player.kind, 2);
        assert_eq!(ctx.player.name_str(), "spaz");
    }

    #[test]
    fn text_cursor_blinks_on_a_seventeen_tick_cycle() {
        let (mut screen, mut ctx) = start();
        let mut seen = Vec::new();
        for _ in 0..17 {
            tick(&mut screen, &mut ctx);
            let cursor = ctx.scene.objects.iter().find(|o| o.id == ID_NAME).unwrap().text_cursor;
            seen.push((screen.blink, cursor));
        }
        assert_eq!(seen[0], (1, 1));
        assert_eq!(seen[10], (11, 1));
        assert_eq!(seen[11], (12, 2));
        assert_eq!(seen[15], (16, 2));
        assert_eq!(seen[16], (0, 1));
    }

    #[test]
    fn idle_offsets_follow_the_roll_thresholds() {
        assert_eq!(idle_anim_offset(0), 3);
        assert_eq!(idle_anim_offset(29), 3);
        assert_eq!(idle_anim_offset(30), 0);
        assert_eq!(idle_anim_offset(44), 0);
        assert_eq!(idle_anim_offset(45), 1);
        assert_eq!(idle_anim_offset(59), 1);
        assert_eq!(idle_anim_offset(60), 2);
        assert_eq!(idle_anim_offset(74), 2);
        assert_eq!(idle_anim_offset(75), 5);
        assert_eq!(idle_anim_offset(99), 5);
    }

    #[test]
    fn family_slots_follow_the_type() {
        assert_eq!((sel_5_anim_index(0), sel_5_anim_index(1), sel_5_anim_index(2)), (4, 10, 16));
        assert_eq!(sel_5_anim_index(-1), 4);
        assert_eq!((idle_base(0), idle_base(1), idle_base(2)), (0, 6, 12));
        assert_eq!(idle_base(-1), 0);
    }

    #[test]
    fn name_accepts_space_capitals_and_digits_only() {
        assert!(accepts_name_char(b' '));
        assert!(accepts_name_char(b'A') && accepts_name_char(b'Z'));
        assert!(accepts_name_char(b'0') && accepts_name_char(b'9'));
        assert!(!accepts_name_char(b'a') && !accepts_name_char(b'@') && !accepts_name_char(b'['));
        assert!(!accepts_name_char(b'/') && !accepts_name_char(b':') && !accepts_name_char(0xc1));
        let mut rgb = [1, 2, 3];
        rgb_set(&mut rgb, 0xf8, 200, 0x30);
        assert_eq!(rgb, [0xf8, 0xc8, 0x30]);
    }
}
