use avmanager::texture::Palette24;

use crate::av::Av;
use crate::host::Host;
use crate::player::PlayerData;

// Key prefix keeping the visiting crunchling's recoloured animations apart from the host's copies
pub const GUEST_PREFIX: &str = "guest:";

// A crunchling record paired with the key prefix its animations load under
#[derive(Clone, Copy)]
pub struct Skin<'a> {
    pub data: &'a PlayerData,
    pub prefix: &'a str,
}

impl<'a> Skin<'a> {
    pub fn host(data: &'a PlayerData) -> Self {
        Self { data, prefix: "" }
    }

    pub fn guest(data: &'a PlayerData) -> Self {
        Self { data, prefix: GUEST_PREFIX }
    }

    fn key(&self, name: &str) -> String {
        format!("{}{name}", self.prefix)
    }

    // Loads the retained palette source for a suffix and returns it with the record's colours applied
    pub async fn palette<H: Host>(
        &self,
        av: &mut Av,
        host: &mut H,
        suffix: &str,
        keep_baby_type: bool,
        adult_as_type0: bool,
    ) -> Option<(u16, Palette24)> {
        let name = self.data.anim_name(suffix, keep_baby_type, adult_as_type0);
        let handle = av.get_anim_keyed(host, &self.key(&name), &name, None, true).await?;
        let mut palette = Palette24::default();
        av.get_anim_palette(handle, &mut palette);
        self.data.recolor_palette(&mut palette);
        av.set_anim_palette(handle, &palette);
        Some((handle, palette))
    }

    // Loads a suffix drawn through an already recoloured palette
    pub async fn anim<H: Host>(
        &self,
        av: &mut Av,
        host: &mut H,
        suffix: &str,
        keep_baby_type: bool,
        adult_as_type0: bool,
        palette: &Palette24,
    ) -> Option<u16> {
        let name = self.data.anim_name(suffix, keep_baby_type, adult_as_type0);
        av.get_anim_keyed(host, &self.key(&name), &name, Some(palette), false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_keys_carry_the_prefix_and_host_keys_do_not() {
        let data = PlayerData::default();
        assert_eq!(Skin::host(&data).key("b_blink"), "b_blink");
        assert_eq!(Skin::guest(&data).key("b_blink"), "guest:b_blink");
    }
}
