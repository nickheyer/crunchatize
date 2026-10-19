use crate::ctx::Ctx;
use crate::host::Host;

const FNV_OFFSET: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

// FNV-1a accumulator over the fields that must match on both sides of a session
struct Digest(u32);

impl Digest {
    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u32::from(b)).wrapping_mul(FNV_PRIME);
        }
    }

    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }

    fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    fn flag(&mut self, v: bool) {
        self.bytes(&[v as u8]);
    }
}

// Hash of the simulated state after a tick, excluding handles and anything only drawn
pub fn state_hash<H: Host>(ctx: &Ctx<H>, state: u32) -> u32 {
    let mut d = Digest(FNV_OFFSET);
    d.u32(state);
    d.bytes(&ctx.player.to_bytes());
    d.u32(ctx.rand.state());
    d.i32(i32::from(ctx.cursor.x));
    d.i32(i32::from(ctx.cursor.y));
    d.flag(ctx.cursor.visible);
    d.i32(i32::from(ctx.session.helper.x));
    d.i32(i32::from(ctx.session.helper.y));
    d.flag(ctx.session.helper.down);
    d.i32(ctx.scene.scroll_x);
    d.i32(ctx.scene.scroll_y);
    for o in ctx.scene.objects.iter().filter(|o| o.state != 0) {
        d.i32(o.hover);
        d.bytes(&o.text);
        let af = o.anims[o.current_anim_index()];
        d.f32(af.frame);
        d.i32(i32::from(af.state));
    }
    for a in &ctx.actors.actors {
        d.i32(a.id);
        d.f32(a.x);
        d.f32(a.y);
        d.flag(a.visible);
        d.f32(a.anim.frame);
        d.i32(i32::from(a.anim.state));
        d.i32(a.timer);
        d.i32(a.facing);
        d.flag(a.has_target);
        d.i32(a.target_x);
        d.i32(i32::from(a.delayed_count));
        d.f32(a.vel_y);
        d.f32(a.t);
    }
    d.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_matches_fnv1a_reference() {
        let mut d = Digest(FNV_OFFSET);
        d.bytes(b"a");
        assert_eq!(d.0, 0xe40c_292c);
        let mut d = Digest(FNV_OFFSET);
        d.bytes(b"foobar");
        assert_eq!(d.0, 0xbf9c_f968);
    }
}
