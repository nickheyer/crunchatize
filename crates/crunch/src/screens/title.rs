use crate::ctx::{Ctx, KEY_ESC, MOUSE_LEFT, RESULT_CONTINUE, RESULT_QUIT, STATE_LOAD_CRUNCH, Screen};
use crate::host::Host;

const ID_PLAY: i32 = 1;

// Opening screen whose single play button leads to the save picker
#[derive(Default)]
pub struct TitleScreen {
    mouse_anim: u16,
    button_sfx: u16,
}

impl TitleScreen {
    pub fn new() -> Self {
        Self::default()
    }

    // Plays the button sound and picks LoadCrunch when the play object was hit
    fn on_click<H: Host>(&self, ctx: &mut Ctx<H>, object_id: i32) -> u32 {
        let mut result = RESULT_CONTINUE;
        ctx.av.play_sfx(self.button_sfx, -1);
        if object_id == ID_PLAY {
            result = STATE_LOAD_CRUNCH;
        }
        result
    }
}

impl<H: Host> Screen<H> for TitleScreen {
    async fn init(&mut self, ctx: &mut Ctx<H>) -> bool {
        let Some(h) = ctx.av.get_anim(&mut ctx.host, "mouse").await else { return false };
        self.mouse_anim = h;
        ctx.cursor.set_anim(&ctx.av, self.mouse_anim);
        ctx.cursor.set_pos(320, 400);
        ctx.cursor.set_visible(true);
        let Some(h) = ctx.av.get_sfx(&mut ctx.host, "Button1.wav", 0, false).await else { return false };
        self.button_sfx = h;
        ctx.scene.load(&mut ctx.av, &mut ctx.host, "title").await;
        ctx.scene.assign_id("play", ID_PLAY);
        true
    }

    // Clicks re-fire every tick the button stays down over the play object
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
            result = self.on_click(ctx, id);
        }
        ctx.scene.advance_anims(&ctx.av);
        if ctx.av.is_key_down(KEY_ESC) {
            result = RESULT_QUIT;
        }
        result
    }

    fn shutdown(&mut self, _ctx: &mut Ctx<H>) {}
}
