use avmanager::anim::STATE_RESTART_FORWARD;
use avmanager::texture::Palette24;

use crate::actors::EVENT_INIT_OBJECT;
use crate::ctx::{
    Ctx, KEY_ESC, MOUSE_LEFT, RESULT_CONTINUE, RESULT_QUIT, STATE_CREATE, STATE_CREDITS, STATE_HOME_WORLD, Screen,
};
use crate::host::Host;
use crate::player::{PlayerData, load_player};
use crate::scene::{HOVER_HIDDEN, HOVER_NORMAL};
use crate::video::play_video_modal;

const ID_CREATE: i32 = 1;
const ID_QUIT: i32 = 2;
const ID_RIGHT: i32 = 3;
const ID_LEFT: i32 = 4;
const ID_DONE: i32 = 5;
const ID_NAME: i32 = 6;
const ID_CREDITS: i32 = 7;
const ID_VIEW_STORY: i32 = 8;
const ID_AOL: i32 = 9;
const ID_DELETE: i32 = 10;
const ID_DELETE_BOX: i32 = 11;
const ID_YES: i32 = 12;
const ID_NO: i32 = 13;

const CRUNCHLING_ACTOR: i32 = 0;
const ACTOR_LAYER_MASK: u32 = 0xf;

// Blink animation per form, baby then the three teen types then the three adult types
const BLINK_ANIM_NAMES: [&str; 7] = ["b_blink", "t_blink", "t2_blink", "t3_blink", "a_blink", "a2_blink", "a3_blink"];

// Save picker listing every .crn file with a blinking preview of the selected crunchling
#[derive(Default)]
pub struct LoadCrunchScreen {
    music_channel: u32,
    blink_anims: [u16; 7],
    selected_index: i32,
    entries: Vec<(String, PlayerData)>,
    mouse_anim: u16,
    button_sfx: u16,
    click_latch: bool,
    delete_confirm: bool,
}

impl LoadCrunchScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Rebuilds the entry list from every readable save and reports whether any exist
    fn scan_save_files<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.entries.clear();
        for name in ctx.host.list_saves() {
            self.add_entry(ctx, &name);
        }
        !self.entries.is_empty()
    }

    // Appends the save as an entry when its magic and size check out
    fn add_entry<H: Host>(&mut self, ctx: &mut Ctx<H>, filename: &str) -> bool {
        let Some(player) = load_player(&mut ctx.host, filename) else { return false };
        self.entries.push((filename.to_string(), player));
        true
    }

    // Loads the selected save into the live player, recolours the blink anims and places actor 0
    fn select_entry<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let (filename, player) = &self.entries[self.selected_index as usize];
        ctx.player = player.clone();
        ctx.globals.save_name = filename.clone();
        ctx.globals.player_loaded = true;
        let mut pal = Palette24::default();
        ctx.av.get_anim_palette(self.blink_anims[0], &mut pal);
        ctx.player.recolor_palette(&mut pal);
        for &h in &self.blink_anims {
            ctx.av.set_anim_palette(h, &pal);
        }
        ctx.scene.set_text_by_id(ID_NAME, &ctx.player.name_str());
        let (pos, index) = blink_placement(ctx.player.stage, ctx.player.kind);
        if let Some((x, y)) = pos {
            ctx.actors.set_pos(CRUNCHLING_ACTOR, x, y);
        }
        let blink = self.blink_anims[index];
        ctx.actors.set_anims(&ctx.av, CRUNCHLING_ACTOR, blink, blink, false, STATE_RESTART_FORWARD);
    }

    // Plays the button sound then acts on the clicked object and returns the tick result
    async fn on_object_clicked<H: Host>(&mut self, ctx: &mut Ctx<H>, object_id: i32) -> u32 {
        let mut result = RESULT_CONTINUE;
        ctx.av.play_sfx(self.button_sfx, -1);
        match object_id {
            ID_CREATE => result = STATE_CREATE,
            ID_QUIT => result = RESULT_QUIT,
            ID_RIGHT => {
                self.selected_index = next_index(self.selected_index, self.entries.len());
                self.select_entry(ctx);
            }
            ID_LEFT => {
                self.selected_index = prev_index(self.selected_index, self.entries.len());
                self.select_entry(ctx);
            }
            ID_DONE => result = STATE_HOME_WORLD,
            ID_CREDITS => result = STATE_CREDITS,
            ID_VIEW_STORY => play_video_modal(&mut ctx.av, &mut ctx.host, "Intro.smk").await,
            ID_AOL => {
                result = RESULT_QUIT;
                ctx.globals.launch_setup = true;
            }
            ID_DELETE => {
                self.delete_confirm = true;
                for id in [ID_DELETE_BOX, ID_YES, ID_NO] {
                    ctx.scene.set_hover_by_id(id, HOVER_NORMAL);
                }
                for id in ID_CREATE..=ID_DELETE {
                    ctx.scene.set_hover_by_id(id, HOVER_HIDDEN);
                }
            }
            ID_YES => {
                result = STATE_CREATE;
                delete_all_save_files(&mut ctx.host);
            }
            ID_NO => {
                self.delete_confirm = false;
                for id in [ID_DELETE_BOX, ID_YES, ID_NO] {
                    ctx.scene.set_hover_by_id(id, HOVER_HIDDEN);
                }
                for id in ID_CREATE..=ID_DELETE {
                    ctx.scene.set_hover_by_id(id, HOVER_NORMAL);
                }
            }
            _ => {}
        }
        result
    }

    fn on_actor_event<H: Host>(&mut self, ctx: &mut Ctx<H>, kind: i32, actor_id: i32) {
        if kind == EVENT_INIT_OBJECT {
            self.on_actor_init(ctx, actor_id);
        }
    }

    fn on_actor_init<H: Host>(&mut self, ctx: &mut Ctx<H>, actor_id: i32) {
        if actor_id == CRUNCHLING_ACTOR {
            self.select_entry(ctx);
        }
    }
}

impl<H: Host> Screen<H> for LoadCrunchScreen {
    // Without any save plays the intro and leaves Update to move on to Create
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        self.selected_index = 0;
        self.music_channel = 0;
        if !self.scan_save_files(ctx) {
            play_video_modal(&mut ctx.av, &mut ctx.host, "Intro.smk").await;
            return true;
        }
        let Some(h) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        self.mouse_anim = h;
        ctx.cursor.set_anim(&ctx.av, self.mouse_anim);
        ctx.cursor.set_pos(320, 400);
        ctx.cursor.set_visible(true);
        let Some(h) = ctx.av.get_sfx(&mut ctx.host, "Button1.wav", 0, false).await else { return false };
        self.button_sfx = h;
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "LoadCrunch").await;
        ctx.scene.assign_id("Create", ID_CREATE);
        ctx.scene.assign_id("quit", ID_QUIT);
        ctx.scene.assign_id("right", ID_RIGHT);
        ctx.scene.assign_id("left", ID_LEFT);
        ctx.scene.assign_id("done", ID_DONE);
        ctx.scene.assign_id("name", ID_NAME);
        ctx.scene.assign_id("credits", ID_CREDITS);
        ctx.scene.assign_id("ViewStory", ID_VIEW_STORY);
        ctx.scene.assign_id("AOL", ID_AOL);
        ctx.scene.assign_id("Delete", ID_DELETE);
        ctx.scene.assign_id("DeleteBox", ID_DELETE_BOX);
        ctx.scene.assign_id("Yes", ID_YES);
        ctx.scene.assign_id("No", ID_NO);
        ctx.scene.set_hover_by_id(ID_DELETE_BOX, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(ID_YES, HOVER_HIDDEN);
        ctx.scene.set_hover_by_id(ID_NO, HOVER_HIDDEN);
        ctx.scene.set_text_by_id(ID_NAME, "");
        if self.entries.len() == 1 {
            ctx.scene.set_hover_by_id(ID_RIGHT, HOVER_HIDDEN);
            ctx.scene.set_hover_by_id(ID_LEFT, HOVER_HIDDEN);
        }
        for (slot, name) in self.blink_anims.iter_mut().zip(BLINK_ANIM_NAMES) {
            let Some(h) = ctx.av.get_anim_retain_image(&mut ctx.host, name).await else { return false };
            *slot = h;
        }
        self.music_channel = ctx.av.play_stream_sound(&mut ctx.host, "crunchsong3.wav", true, 0).await;
        ctx.actors.clear();
        ctx.actors.add_actor(CRUNCHLING_ACTOR, true);
        self.click_latch = false;
        self.delete_confirm = false;
        true
    }

    // Clicks latch until the cursor rests on an object with the button released
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        if self.entries.is_empty() {
            return STATE_CREATE;
        }
        let mut result = RESULT_CONTINUE;
        ctx.scene.draw_layer(&mut ctx.av, 0);
        ctx.actors.update(&ctx.av);
        while let Some(ev) = ctx.actors.pop_event() {
            self.on_actor_event(ctx, ev.kind, ev.actor_id);
        }
        if !self.delete_confirm {
            ctx.actors.draw(&mut ctx.av, &ctx.scene, ACTOR_LAYER_MASK);
        }
        ctx.scene.draw_layer(&mut ctx.av, 1);
        let (dx, dy) = ctx.av.get_mouse_delta();
        ctx.cursor.move_by(dx, dy);
        let (x, y) = (ctx.cursor.x, ctx.cursor.y);
        ctx.cursor.draw(&mut ctx.av);
        if let Some(id) = ctx.scene.hit_test(x as i32, y as i32) {
            if ctx.av.is_mouse_button_down(MOUSE_LEFT) {
                if !self.click_latch {
                    result = self.on_object_clicked(ctx, id).await;
                    self.click_latch = true;
                }
            } else {
                self.click_latch = false;
            }
        }
        ctx.scene.advance_anims(&ctx.av);
        if ctx.av.is_key_down(KEY_ESC) {
            result = RESULT_QUIT;
        }
        result
    }

    fn shutdown(&mut self, ctx: &mut Ctx<H>) {
        ctx.av.kill_streaming_channel(self.music_channel);
    }
}

// Removes every save file, not only the selected one
fn delete_all_save_files<H: Host>(host: &mut H) {
    for name in host.list_saves() {
        host.delete_save(&name);
    }
}

// Index after a right click, wrapping to the first entry past the last one
fn next_index(selected: i32, count: usize) -> i32 {
    if (selected as u32) < (count as u32).wrapping_sub(1) { selected + 1 } else { 0 }
}

// Index after a left click, wrapping to the last entry before the first one
fn prev_index(selected: i32, count: usize) -> i32 {
    if selected == 0 { count as i32 - 1 } else { selected - 1 }
}

// Actor 0 position and blink slot for a stage and type, unknown stages leave the position alone
fn blink_placement(stage: u16, kind: i16) -> (Option<(f32, f32)>, usize) {
    match stage {
        0 => (Some((155.0, 280.0)), 0),
        1 => (Some((145.0, 250.0)), match kind { 0 => 1, 1 => 2, 2 => 3, _ => 0 }),
        2 => (Some((125.0, 205.0)), match kind { 0 => 4, 1 => 5, 2 => 6, _ => 0 }),
        _ => (None, 0),
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context as TaskContext, Poll, Waker};

    use avmanager::input::DeviceState;
    use avmanager::sound::Sound;

    use super::*;
    use crate::player::encode_save;

    // In memory save directory that also records every data file the game asked for
    struct SaveHost {
        files: Vec<(String, Vec<u8>)>,
        reads: Vec<String>,
    }

    impl Host for SaveHost {
        async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
            self.reads.push(name.to_string());
            None
        }

        fn time_ms(&self) -> u32 {
            0
        }

        fn unix_time(&self) -> u32 {
            0
        }

        fn poll_input(&mut self) -> DeviceState {
            DeviceState::default()
        }

        async fn end_frame(&mut self, _frame: Option<&[u16]>, _sound: &mut Sound) {}

        fn list_saves(&mut self) -> Vec<String> {
            self.files.iter().map(|(n, _)| n.clone()).collect()
        }

        fn read_save(&mut self, name: &str) -> Option<Vec<u8>> {
            self.files.iter().find(|(n, _)| n == name).map(|(_, d)| d.clone())
        }

        fn write_save(&mut self, name: &str, data: &[u8]) -> bool {
            self.files.push((name.to_string(), data.to_vec()));
            true
        }

        fn save_exists(&mut self, name: &str) -> bool {
            self.files.iter().any(|(n, _)| n == name)
        }

        fn delete_save(&mut self, name: &str) {
            self.files.retain(|(n, _)| n != name);
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

        fn launch_setup(&mut self) {}

        fn log(&mut self, _message: &str) {}
    }

    fn block_on<F: Future>(fut: F) -> F::Output {
        let mut fut = pin!(fut);
        let mut cx = TaskContext::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    fn save_named(name: &str) -> Vec<u8> {
        let mut p = PlayerData::default();
        p.set_name(name);
        encode_save(&p)
    }

    fn ctx_with_saves(files: Vec<(String, Vec<u8>)>) -> Ctx<SaveHost> {
        Ctx::new(SaveHost { files, reads: Vec::new() }, 22050)
    }

    #[test]
    fn scan_keeps_listing_order_and_skips_bad_files() {
        let mut ctx = ctx_with_saves(vec![
            ("Player_2.crn".into(), save_named("two")),
            ("junk.crn".into(), vec![1, 2, 3]),
            ("Player_1.crn".into(), save_named("one")),
        ]);
        let mut screen = LoadCrunchScreen::new();
        assert!(screen.scan_save_files(&mut ctx));
        let names: Vec<&str> = screen.entries.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Player_2.crn", "Player_1.crn"]);
        assert_eq!(screen.entries[1].1.name_str(), "one");
        assert!(!screen.add_entry(&mut ctx, "junk.crn"));
    }

    #[test]
    fn no_saves_plays_intro_then_goes_to_create() {
        let mut ctx = ctx_with_saves(Vec::new());
        let mut screen = LoadCrunchScreen::new();
        assert!(block_on(screen.init(&mut ctx)));
        assert_eq!(ctx.host.reads, ["Intro.smk"]);
        assert_eq!(block_on(screen.update(&mut ctx)), STATE_CREATE);
    }

    #[test]
    fn delete_all_removes_every_save() {
        let mut ctx = ctx_with_saves(vec![
            ("Player_1.crn".into(), save_named("one")),
            ("Player_2.crn".into(), save_named("two")),
        ]);
        delete_all_save_files(&mut ctx.host);
        assert!(ctx.host.list_saves().is_empty());
    }

    #[test]
    fn right_and_left_wrap_around() {
        assert_eq!(next_index(0, 3), 1);
        assert_eq!(next_index(2, 3), 0);
        assert_eq!(next_index(0, 1), 0);
        assert_eq!(prev_index(0, 3), 2);
        assert_eq!(prev_index(2, 3), 1);
        assert_eq!(prev_index(0, 1), 0);
    }

    #[test]
    fn placement_follows_stage_and_type() {
        assert_eq!(blink_placement(0, 2), (Some((155.0, 280.0)), 0));
        assert_eq!(blink_placement(1, 0), (Some((145.0, 250.0)), 1));
        assert_eq!(blink_placement(1, 2), (Some((145.0, 250.0)), 3));
        assert_eq!(blink_placement(2, 1), (Some((125.0, 205.0)), 5));
        assert_eq!(blink_placement(2, 7), (Some((125.0, 205.0)), 0));
        assert_eq!(blink_placement(3, 0), (None, 0));
    }
}
