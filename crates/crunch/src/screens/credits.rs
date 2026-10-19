use crate::ctx::{Ctx, KEY_ESC, MOUSE_LEFT, RESULT_CONTINUE, STATE_LOAD_CRUNCH, Screen};
use crate::host::Host;

const ID_DONE: i32 = 1;

// Credits roll with looping music and a done button back to the save picker
#[derive(Default)]
pub struct CreditsScreen {
    music_channel: u32,
    mouse_anim: u16,
    button_sfx: u16,
}

impl CreditsScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Plays the button sound and picks LoadCrunch when the done object was hit
    fn on_object_clicked<H: Host>(&self, ctx: &mut Ctx<H>, object_id: i32) -> u32 {
        let mut result = RESULT_CONTINUE;
        ctx.av.play_sfx(self.button_sfx, -1);
        if object_id == ID_DONE {
            result = STATE_LOAD_CRUNCH;
        }
        result
    }
}

impl<H: Host> Screen<H> for CreditsScreen {
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        let Some(h) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        self.mouse_anim = h;
        ctx.cursor.set_anim(&ctx.av, self.mouse_anim);
        ctx.cursor.set_pos(320, 400);
        ctx.cursor.set_visible(true);
        let Some(h) = ctx.av.get_sfx(&mut ctx.host, "Button1.wav", 0, false).await else { return false };
        self.button_sfx = h;
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "credits").await;
        ctx.scene.assign_id("done", ID_DONE);
        self.music_channel = ctx.av.play_stream_sound(&mut ctx.host, "crunchsong3.wav", true, 0).await;
        true
    }

    // Clicks re-fire every tick the button stays down and ESC also leaves for LoadCrunch
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut result = RESULT_CONTINUE;
        ctx.scene.draw_layer(&mut ctx.av, 0);
        ctx.scene.draw_layer(&mut ctx.av, 1);
        let (dx, dy) = ctx.av.get_mouse_delta();
        ctx.cursor.move_by(dx, dy);
        let (x, y) = (ctx.cursor.x, ctx.cursor.y);
        ctx.cursor.draw(&mut ctx.av);
        if let Some(id) = ctx.scene.hit_test(x as i32, y as i32)
            && ctx.av.is_mouse_button_down(MOUSE_LEFT)
        {
            result = self.on_object_clicked(ctx, id);
        }
        ctx.scene.advance_anims(&ctx.av);
        if ctx.av.is_key_down(KEY_ESC) {
            result = STATE_LOAD_CRUNCH;
        }
        result
    }

    fn shutdown(&mut self, ctx: &mut Ctx<H>) {
        ctx.av.kill_streaming_channel(self.music_channel);
    }
}
