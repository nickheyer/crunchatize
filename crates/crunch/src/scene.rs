use avmanager::anim::{AnimFrame, STATE_RESTART_FORWARD};
use avmanager::crt::{stricmp, with_extension};

use crate::av::Av;
use crate::host::Host;

pub const WAM_MAGIC: u32 = 0x7788_acdc;
pub const LAYER_ALL: i32 = 8;

pub const HOVER_NORMAL: i32 = 0;
pub const HOVER_HOT: i32 = 1;
pub const HOVER_PRESSED: i32 = 2;
pub const HOVER_PRESSED_HOT: i32 = 3;
pub const HOVER_HIDDEN: i32 = 4;

pub const KIND_ANIM: u32 = 0;
pub const KIND_TEXT: u32 = 1;

pub const FLAG_HOVER_ONLY: u32 = 1;
pub const FLAG_PLATFORM: u32 = 2;
pub const FLAG_FINISH: u32 = 4;
pub const FLAG_PICKUP: u32 = 8;
pub const FLAG_SCREEN_FIXED: u32 = 0x10;

pub const NO_HANDLE: u16 = 0xffff;
const TEXT_CAPACITY: usize = 32;

// One placed object of a WAM scene, art or text, with its hover state and hit box
#[derive(Clone, Debug, PartialEq)]
pub struct SceneObject {
    pub name: Vec<u8>,
    pub anims: [AnimFrame; 4],
    pub x: i32,
    pub y: i32,
    pub state: i32,
    pub locked: u32,
    pub bbox_y1: i32,
    pub bbox_y2: i32,
    pub bbox_x1: i32,
    pub bbox_x2: i32,
    pub kind: u32,
    pub text: Vec<u8>,
    pub font1: u16,
    pub font2: u16,
    pub text_max_len: i32,
    pub layer: i32,
    pub align: i32,
    pub flags: u32,
    pub extra0: i32,
    pub extra1: i32,
    pub extra2: f32,
    pub extra3: f32,
    pub hover: i32,
    pub id: i32,
    pub text_cursor: i32,
}

impl Default for SceneObject {
    fn default() -> Self {
        Self {
            name: vec![0; 16],
            anims: [AnimFrame { frame: 0.0, anim: NO_HANDLE, state: 0, fps: 0 }; 4],
            x: 0,
            y: 0,
            state: 0,
            locked: 0,
            bbox_y1: 0,
            bbox_y2: 0,
            bbox_x1: 0,
            bbox_x2: 0,
            kind: KIND_ANIM,
            text: Vec::new(),
            font1: NO_HANDLE,
            font2: NO_HANDLE,
            text_max_len: 0,
            layer: 0,
            align: 0,
            flags: 0,
            extra0: 0,
            extra1: 0,
            extra2: 0.0,
            extra3: 0.0,
            hover: HOVER_NORMAL,
            id: 0,
            text_cursor: 0,
        }
    }
}

impl SceneObject {
    pub fn text_str(&self) -> String {
        String::from_utf8_lossy(&self.text).into_owned()
    }

    // Index of the AnimFrame shown for the current hover state with the original fallbacks
    pub fn current_anim_index(&self) -> usize {
        let mut i = match self.hover {
            HOVER_HOT => 1,
            HOVER_PRESSED => 2,
            HOVER_PRESSED_HOT => {
                if self.anims[3].anim == NO_HANDLE { 2 } else { 3 }
            }
            _ => 0,
        };
        if self.anims[i].anim == NO_HANDLE {
            i = 0;
        }
        i
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        let end = (self.pos + n).min(self.data.len());
        let mut out = self.data[self.pos.min(end)..end].to_vec();
        out.resize(n, 0);
        self.pos += n;
        out
    }

    fn u32(&mut self) -> u32 {
        let b = self.bytes(4);
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }

    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }

    fn i16(&mut self) -> i16 {
        let b = self.bytes(2);
        i16::from_le_bytes([b[0], b[1]])
    }

    fn f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }

    fn bool(&mut self) -> bool {
        self.bytes(1)[0] != 0
    }

    // Length prefixed string, copying at most cap bytes while skipping the full length
    fn string(&mut self, cap: usize) -> Vec<u8> {
        let len = self.u32() as usize;
        if len == 0 {
            return Vec::new();
        }
        let take = if cap - 1 < len { cap } else { len };
        let start = self.pos.min(self.data.len());
        let end = (self.pos + take).min(self.data.len());
        let out = self.data[start..end].to_vec();
        self.pos += len;
        out
    }

    fn string_str(&mut self, cap: usize) -> String {
        String::from_utf8_lossy(&self.string(cap)).into_owned()
    }
}

// Scrolling collection of WAM objects with hit testing and layered drawing
#[derive(Debug, Default)]
pub struct Scene {
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub bounds: [i32; 4],
    pub grid_x: i32,
    pub grid_y: i32,
    pub live_count: i32,
    pub objects: Vec<SceneObject>,
}

const GROW: usize = 10;

impl Scene {
    pub fn new() -> Self {
        Self {
            scroll_x: 0,
            scroll_y: 0,
            bounds: [0, 0, 640, 480],
            grid_x: 1,
            grid_y: 1,
            live_count: 0,
            objects: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        for o in &mut self.objects {
            o.state = 0;
        }
        self.live_count = 0;
        self.bounds = [0, 0, 640, 480];
        self.scroll_x = 0;
        self.scroll_y = 0;
    }

    pub fn set_scroll(&mut self, x: i32, y: i32) {
        self.scroll_x = x;
        self.scroll_y = y;
    }

    pub fn to_screen(&self, x: &mut i32, y: &mut i32) {
        *x -= self.scroll_x;
        *y -= self.scroll_y;
    }

    pub fn object_count(&self) -> i32 {
        self.live_count
    }

    pub fn capacity(&self) -> usize {
        self.objects.len()
    }

    // Reads one record, loading its animations and fonts, false on a bad version or load failure
    async fn read_record<H: Host>(r: &mut Reader<'_>, av: &mut Av, host: &mut H, out: &mut SceneObject) -> bool {
        let version = r.i32();
        out.name = r.bytes(16);
        if version >= 3 {
            return false;
        }
        for slot in 0..4 {
            out.anims[slot].anim = NO_HANDLE;
        }
        for slot in 0..4 {
            if r.bool() {
                let name = r.string_str(0x100);
                let Some(h) = av.get_anim(host, &name).await else { return false };
                out.anims[slot].anim = h;
                out.anims[slot].state = r.i16();
                out.anims[slot].fps = r.i16() as u16;
                out.anims[slot].frame = r.f32();
            }
        }
        out.x = r.i32();
        out.y = r.i32();
        out.bbox_y1 = r.i32();
        out.bbox_y2 = r.i32();
        out.bbox_x1 = r.i32();
        out.bbox_x2 = r.i32();
        out.text_max_len = r.i32();
        out.layer = r.i32();
        out.align = r.i32();
        out.locked = r.u32();
        out.kind = r.u32();
        out.flags = r.u32();
        out.text = r.string(TEXT_CAPACITY);
        if version < 2 {
            out.extra0 = 0;
            out.extra1 = 0;
            out.extra2 = 0.0;
            out.extra3 = 0.0;
        } else {
            out.extra0 = r.i32();
            out.extra1 = r.i32();
            out.extra2 = r.f32();
            out.extra3 = r.f32();
        }
        out.hover = HOVER_NORMAL;
        out.font1 = NO_HANDLE;
        out.font2 = NO_HANDLE;
        if r.bool() {
            let name = r.string_str(0x100);
            let Some(h) = av.get_font(host, &name).await else { return false };
            out.font1 = h;
        }
        if r.bool() {
            let name = r.string_str(0x100);
            let Some(h) = av.get_font(host, &name).await else { return false };
            out.font2 = h;
        }
        true
    }

    // Loads "<name>.wam", replacing the current objects, false when the file is missing or malformed
    pub async fn load<H: Host>(&mut self, av: &mut Av, host: &mut H, name: &str) -> bool {
        let file = with_extension(name, ".wam");
        let Some(data) = host.read_file(&file).await else {
            host.log(&format!("*Error loading {name}.wam"));
            return false;
        };
        self.clear();
        let mut r = Reader { data: &data, pos: 0 };
        let version = r.u32();
        let magic = r.u32();
        let count = r.u32();
        if version >= 2 || magic != WAM_MAGIC {
            return false;
        }
        let mut record = SceneObject::default();
        for _ in 0..count {
            if !Self::read_record(&mut r, av, host, &mut record).await {
                return false;
            }
            self.add_object(av, &record);
        }
        true
    }

    // Places a record in the first free slot and computes its hit box, returning the slot index
    pub fn add_object(&mut self, av: &Av, src: &SceneObject) -> usize {
        let index = match self.objects.iter().position(|o| o.state == 0) {
            Some(i) if self.live_count as usize != self.objects.len() => i,
            _ => {
                let i = self.objects.len();
                self.objects.extend((0..GROW).map(|_| SceneObject::default()));
                i
            }
        };
        let mut obj = src.clone();
        obj.state = 1;
        obj.hover = HOVER_NORMAL;
        obj.id = 0;
        obj.text_cursor = 0;
        self.objects[index] = obj;
        self.live_count += 1;
        self.recalc_bbox(av, index);
        index
    }

    // Copies an object under a new id to another spot and alignment, false when the id is unknown
    pub fn clone_object(&mut self, av: &Av, id: i32, new_id: i32, x: i32, y: i32, align: i32) -> bool {
        let Some(i) = self.index_of_id(id) else { return false };
        let mut copy = self.objects[i].clone();
        copy.x = x;
        copy.y = y;
        copy.align = align;
        let index = self.add_object(av, &copy);
        self.objects[index].id = new_id;
        true
    }

    // Moves an object to another spot and refreshes its hit box
    pub fn move_object(&mut self, av: &Av, id: i32, x: i32, y: i32) {
        if let Some(i) = self.index_of_id(id) {
            self.objects[i].x = x;
            self.objects[i].y = y;
            self.recalc_bbox(av, i);
        }
    }

    pub fn get_object(&self, index: usize) -> SceneObject {
        self.objects[index].clone()
    }

    // Recomputes an object's hit box from its art or text and widens the scene bounds
    pub fn recalc_bbox(&mut self, av: &Av, index: usize) {
        let o = &self.objects[index];
        let (mut hot_x, mut hot_y) = (0i32, 0i32);
        let (x1, y1, x2, y2);
        if o.kind == KIND_ANIM {
            av.get_hot_spot(o.anims[0].anim, &mut hot_x, &mut hot_y);
            let b = av.get_anim_bbox(o.anims[0].anim);
            x1 = b[0];
            y1 = b[1];
            x2 = b[2];
            y2 = b[3];
        } else {
            let mut width = av.calc_text_size(&o.text_str(), o.font1 as u32, 0);
            let mut left = 0;
            if o.align == 1 {
                width /= 2;
                left -= width;
            } else if o.align == 2 {
                left = -width;
                width = 0;
            }
            x1 = left;
            y1 = 0;
            x2 = width;
            y2 = av.get_font_height(o.font1 as u32);
        }
        let bx = o.x - hot_x;
        let by = o.y - hot_y;
        let o = &mut self.objects[index];
        o.bbox_x1 = x1 + bx;
        o.bbox_x2 = x2 + bx;
        o.bbox_y1 = y1 + by;
        o.bbox_y2 = y2 + by;
        if o.bbox_y1 < self.bounds[1] {
            self.bounds[1] = o.bbox_y1;
        }
        if self.bounds[3] < o.bbox_y2 {
            self.bounds[3] = o.bbox_y2;
        }
        if o.bbox_x1 < self.bounds[0] {
            self.bounds[0] = o.bbox_x1;
        }
        if self.bounds[2] < o.bbox_x2 {
            self.bounds[2] = o.bbox_x2;
        }
    }

    // Draws every live object on a layer in slot order, 8 meaning all layers
    pub fn draw_layer(&mut self, av: &mut Av, layer: i32) {
        for i in 0..self.objects.len() {
            let o = &self.objects[i];
            if o.state == 0 || o.hover == HOVER_HIDDEN || (layer != LAYER_ALL && o.layer != layer) {
                continue;
            }
            let mut x = o.x - self.scroll_x;
            let mut y = o.y - self.scroll_y;
            if o.state == 2 {
                x = (o.x / self.grid_x) * self.grid_x - self.scroll_x;
                y = (o.y / self.grid_y) * self.grid_y - self.scroll_y;
            }
            if o.flags & FLAG_SCREEN_FIXED != 0 {
                x += self.scroll_x;
                y += self.scroll_y;
            }
            if o.kind == KIND_ANIM {
                if o.flags & FLAG_HOVER_ONLY == 0 || o.hover != HOVER_NORMAL {
                    let slot = o.current_anim_index();
                    let mut af = self.objects[i].anims[slot];
                    av.draw_anim(&mut af, x, y);
                    self.objects[i].anims[slot] = af;
                }
            } else {
                let text = o.text_str();
                av.print_cursor(&text, x, y, o.font1 as u32, o.align, o.text_cursor as u32);
            }
        }
        for o in &self.objects {
            if o.state != 2 {
                continue;
            }
            let mut sx = (o.x / self.grid_x) * self.grid_x - self.scroll_x;
            let mut sy = (o.y / self.grid_y) * self.grid_y - self.scroll_y;
            if o.flags & FLAG_SCREEN_FIXED != 0 {
                sx += self.scroll_x;
                sy += self.scroll_y;
            }
            sx += o.bbox_x1 - o.x;
            sy += o.bbox_y1 - o.y;
            let left = sx as i16;
            let top = sy as i16;
            let right = (sx + (o.bbox_x2 - o.bbox_x1)) as i16;
            let bottom = (sy + (o.bbox_y2 - o.bbox_y1)) as i16;
            av.display.gdi_draw_line(left, top, right, top, 2, 0xffffff);
            av.display.gdi_draw_line(left, top, left, bottom, 2, 0xffffff);
            av.display.gdi_draw_line(left, bottom, right, bottom, 2, 0xffffff);
            av.display.gdi_draw_line(right, top, right, bottom, 2, 0xffffff);
        }
    }

    // Steps the shown animation of every live art object
    pub fn advance_anims(&mut self, av: &Av) -> bool {
        let mut any = false;
        for o in &mut self.objects {
            if o.state != 0 && o.kind == KIND_ANIM {
                let slot = o.current_anim_index();
                av.get_anim_next_frame(&mut o.anims[slot]);
                any = true;
            }
        }
        any
    }

    fn index_of_id(&self, id: i32) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    // Sets an object's hover state by id and restarts the animation that state shows
    pub fn set_hover_by_id(&mut self, id: i32, hover: i32) {
        if let Some(i) = self.index_of_id(id) {
            self.set_hover_by_index(i, hover);
        }
    }

    pub fn set_hover_by_index(&mut self, index: usize, hover: i32) {
        let o = &mut self.objects[index];
        o.hover = hover;
        let slot = o.current_anim_index();
        o.anims[slot].state = STATE_RESTART_FORWARD;
    }

    // Replaces an object's text, cut to one less than its maximum length
    pub fn set_text_by_id(&mut self, id: i32, text: &str) {
        let Some(i) = self.index_of_id(id) else { return };
        let o = &mut self.objects[i];
        let mut cut = o.text_max_len - 1;
        if cut == 0 {
            cut = 0x1f;
        }
        let mut bytes = text.as_bytes().to_vec();
        if cut >= 0 && bytes.len() > cut as usize {
            bytes.truncate(cut as usize);
        }
        o.text = bytes;
    }

    pub fn set_text_cursor_by_id(&mut self, id: i32, cursor: i32) {
        if let Some(i) = self.index_of_id(id) {
            self.objects[i].text_cursor = cursor;
        }
    }

    pub fn text_by_id(&self, id: i32) -> Option<String> {
        self.index_of_id(id).map(|i| self.objects[i].text_str())
    }

    // Gives the first live object with a matching name the interactive id
    pub fn assign_id(&mut self, name: &str, id: i32) {
        let found = self.objects.iter().position(|o| {
            o.state != 0 && {
                let end = o.name.iter().position(|&c| c == 0).unwrap_or(o.name.len());
                stricmp(&o.name[..end], name.as_bytes()) == 0
            }
        });
        if let Some(i) = found {
            self.objects[i].id = id;
        }
    }

    // Reports the topmost interactive object under a screen point without touching hover states
    pub fn probe(&self, x: i32, y: i32) -> Option<i32> {
        self.objects.iter().rev().find_map(|o| {
            if o.state == 0 || o.id == 0 || o.hover == HOVER_HIDDEN {
                return None;
            }
            let (px, py) = if o.flags & FLAG_SCREEN_FIXED != 0 { (x, y) } else { (x + self.scroll_x, y + self.scroll_y) };
            let inside = px >= o.bbox_x1 && px <= o.bbox_x2 && py >= o.bbox_y1 && py <= o.bbox_y2;
            inside.then_some(o.id)
        })
    }

    // Lights an object the way the cursor resting on it would
    pub fn hover_hot_by_id(&mut self, id: i32) {
        if let Some(i) = self.index_of_id(id) {
            let o = &mut self.objects[i];
            if o.hover == HOVER_NORMAL {
                o.hover = HOVER_HOT;
            } else if o.hover == HOVER_PRESSED {
                o.hover = HOVER_PRESSED_HOT;
            }
        }
    }

    // Reports the topmost interactive object under a screen point and updates hover states
    pub fn hit_test(&mut self, x: i32, y: i32) -> Option<i32> {
        let raw_x = x;
        let raw_y = y;
        let mut px = x + self.scroll_x;
        let mut py = y + self.scroll_y;
        let mut hit: Option<i32> = None;
        for i in (0..self.objects.len()).rev() {
            let o = &mut self.objects[i];
            if o.state == 0 || o.id == 0 || o.hover == HOVER_HIDDEN {
                continue;
            }
            let fixed = o.flags & FLAG_SCREEN_FIXED != 0;
            if fixed {
                px = raw_x;
                py = raw_y;
            }
            let inside = px >= o.bbox_x1 && px <= o.bbox_x2 && py >= o.bbox_y1 && py <= o.bbox_y2;
            if hit.is_some() || !inside {
                if o.hover == HOVER_HOT {
                    o.hover = HOVER_NORMAL;
                } else if o.hover == HOVER_PRESSED_HOT {
                    o.hover = HOVER_PRESSED;
                }
            } else {
                if o.hover == HOVER_NORMAL {
                    o.hover = HOVER_HOT;
                } else if o.hover == HOVER_PRESSED {
                    o.hover = HOVER_PRESSED_HOT;
                }
                hit = Some(o.id);
            }
            if fixed {
                px += self.scroll_x;
                py += self.scroll_y;
            }
        }
        hit
    }
}
