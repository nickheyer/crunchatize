use avmanager::font::{ALIGN_CENTER, Glyph};
use avmanager::texture::Texture;

use crate::ctx::{Ctx, KEY_ESC, MOUSE_LEFT, RESULT_CONTINUE, STATE_LOAD_CRUNCH, Screen};
use crate::host::Host;
use crate::scene::{HOVER_HIDDEN, KIND_TEXT, SceneObject};

const ID_DONE: i32 = 1;
const ID_ART: i32 = 2;

// The replacement credits, one entry per line, centred below the avatar
const QUOTE_LINES: [&str; 3] = ["\"I f*cking love", "cap'n crunch\"", "- Nick (1999)"];
const QUOTE_X: i32 = 320;
const QUOTE_TOP: i32 = 208;
const QUOTE_LINE_STEP: i32 = 32;
const HYPHEN_END_WIDTH: i32 = 8;
const EXTRA_GLYPHS: [u8; 3] = [b'\'', b'-', b'*'];

// Credits roll with looping music and a done button back to the save picker
#[derive(Default)]
pub struct CreditsScreen {
    music_channel: u32,
    mouse_anim: u16,
    button_sfx: u16,
    host_credits: bool,
}

// Copies a w by h block of pixels between textures
fn copy_rect(src: &Texture, dst: &mut Texture, from: (i32, i32), size: (i32, i32), to: (i32, i32)) {
    for y in 0..size.1 {
        for x in 0..size.0 {
            dst.put(to.0 + x, to.1 + y, src.pixel(from.0 + x, from.1 + y));
        }
    }
}

// Bounds of the non key pixels inside a glyph cell as left, top, right, bottom
fn ink_bounds(sheet: &Texture, cell: Glyph, key: u16) -> Option<(i32, i32, i32, i32)> {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for y in i32::from(cell.top)..=i32::from(cell.bottom) {
        for x in i32::from(cell.left)..=i32::from(cell.right) {
            if sheet.pixel(x, y) != key {
                bounds = Some(match bounds {
                    None => (x, y, x, y),
                    Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
                });
            }
        }
    }
    bounds
}

// Adds apostrophe, hyphen and asterisk cells derived from the font's double quote, cursor bar and x
fn extend_font<H: Host>(ctx: &mut Ctx<H>, font: u16) -> bool {
    let f = u32::from(font);
    let (Some(quote), Some(bar)) = (ctx.av.fonts.glyph(f, b'"'), ctx.av.fonts.glyph(f, b'|')) else { return false };
    let cell_w = i32::from(quote.right) - i32::from(quote.left) + 1;
    let cell_h = ctx.av.get_font_height(f);
    let sheet_id = ctx.av.anims.frame_bitmap(ctx.av.fonts.anim(f), 0);
    let old = ctx.av.bitmaps.get(sheet_id);
    let Some(key) = old.key else { return false };
    let Some((bx1, by1, bx2, by2)) = ink_bounds(old, bar, key) else { return false };
    let top = i32::from(old.height);
    let mut sheet = Texture {
        width: old.width,
        height: old.height + cell_h as u16,
        hot_x: old.hot_x,
        hot_y: old.hot_y,
        pixels: old.pixels.clone(),
        key: old.key,
        source: None,
    };
    sheet.pixels.resize(usize::from(old.width) * usize::from(sheet.height), key);
    copy_rect(old, &mut sheet, (i32::from(quote.left), i32::from(quote.top)), (cell_w / 2, cell_h), (0, top));
    let bar_h = by2 - by1 + 1;
    let hyphen_y = top + (cell_h - bar_h) / 2;
    let hyphen_mid = cell_w + cell_w / 2;
    copy_rect(old, &mut sheet, (bx1, by1), (HYPHEN_END_WIDTH, bar_h), (hyphen_mid - HYPHEN_END_WIDTH, hyphen_y));
    copy_rect(old, &mut sheet, (bx2 + 1 - HYPHEN_END_WIDTH, by1), (HYPHEN_END_WIDTH, bar_h), (hyphen_mid, hyphen_y));
    ctx.av.fonts.stamp_half("x", f, 2 * cell_w + cell_w / 4, top, &ctx.av.anims, &ctx.av.bitmaps, &mut sheet);
    *ctx.av.bitmaps.get_mut(sheet_id) = sheet;
    for (i, ch) in EXTRA_GLYPHS.into_iter().enumerate() {
        let left = i as i32 * cell_w;
        let glyph = Glyph {
            left: left as i16,
            top: top as i16,
            right: (left + cell_w - 1) as i16,
            bottom: (top + cell_h - 1) as i16,
        };
        ctx.av.fonts.add_glyph(f, ch, glyph);
    }
    true
}

// Places the quote lines as centred text objects drawn with the game font
fn add_quote<H: Host>(ctx: &mut Ctx<H>, font: u16) {
    for (i, line) in QUOTE_LINES.iter().enumerate() {
        let object = SceneObject {
            kind: KIND_TEXT,
            text: line.as_bytes().to_vec(),
            font1: font,
            x: QUOTE_X,
            y: QUOTE_TOP + i as i32 * QUOTE_LINE_STEP,
            align: ALIGN_CENTER,
            ..SceneObject::default()
        };
        ctx.scene.add_object(&ctx.av, &object);
    }
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
    // Loads the scene and swaps the original art for the quote when the host shows credits of its own
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
        self.host_credits = ctx.host.show_credits(true);
        if self.host_credits {
            ctx.scene.assign_id("new", ID_ART);
            ctx.scene.set_hover_by_id(ID_ART, HOVER_HIDDEN);
            let Some(font) = ctx.av.get_font(&mut ctx.host, "font").await else { return false };
            if !extend_font(ctx, font) {
                return false;
            }
            add_quote(ctx, font);
        }
        self.music_channel = ctx.av.play_stream_sound(&mut ctx.host, "crunchsong3.wav", true, 0).await;
        true
    }

    // Clicks re-fire every tick the button stays down and ESC also leaves for LoadCrunch
    async fn update(&mut self, ctx: &mut Ctx<H>) -> u32 {
        let mut result = RESULT_CONTINUE;
        if self.host_credits {
            ctx.av.clear_buffer(0, 0, 0);
        }
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
        ctx.host.show_credits(false);
        ctx.av.kill_streaming_channel(self.music_channel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texture(width: u16, height: u16, pixels: Vec<u16>) -> Texture {
        Texture { width, height, hot_x: 0, hot_y: 0, pixels, key: Some(0), source: None }
    }

    #[test]
    fn ink_bounds_frames_the_non_key_pixels_of_a_cell() {
        let mut t = texture(6, 4, vec![0; 24]);
        t.put(2, 1, 7);
        t.put(4, 2, 7);
        let cell = Glyph { left: 0, top: 0, right: 5, bottom: 3 };
        assert_eq!(ink_bounds(&t, cell, 0), Some((2, 1, 4, 2)));
        assert_eq!(ink_bounds(&texture(6, 4, vec![0; 24]), cell, 0), None);
    }

    #[test]
    fn copy_rect_moves_a_block_between_textures() {
        let src = texture(3, 2, vec![1, 2, 3, 4, 5, 6]);
        let mut dst = texture(4, 4, vec![0; 16]);
        copy_rect(&src, &mut dst, (1, 0), (2, 2), (2, 1));
        assert_eq!(dst.pixels, vec![0, 0, 0, 0, 0, 0, 2, 3, 0, 0, 5, 6, 0, 0, 0, 0]);
    }
}
