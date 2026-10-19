use avmanager::anim::{AnimFrame, STATE_RESTART_BACKWARD, STATE_RESTART_FORWARD};
use avmanager::crt::ftol;

use crate::av::Av;
use crate::scene::Scene;

pub const EVENT_INIT_OBJECT: i32 = 1;
pub const EVENT_ANIM_END: i32 = 2;
pub const EVENT_COLLISION: i32 = 3;
pub const EVENT_TARGET_REACHED: i32 = 4;
pub const EVENT_MOUSE_HIT: i32 = 5;
pub const EVENT_TIMER: i32 = 6;
pub const EVENT_GROUND_REACHED: i32 = 7;
pub const EVENT_UNATTACHED: i32 = 8;
pub const EVENT_PICKUP: i32 = 9;
pub const EVENT_FINISH: i32 = 10;

pub const FACING_LEFT: i32 = 0;
pub const FACING_RIGHT: i32 = 1;
pub const FACING_NONE: i32 = 2;
pub const FACING_FORCE_NONE: i32 = 3;

pub const NO_PLATFORM: i32 = 99999;
pub const NO_TARGET: i32 = 9999;
pub const NO_HANDLE: u16 = 0xffff;

pub const ACTOR_FLAG_PICKUP: u32 = 1;
pub const ACTOR_FLAG_FINISH: u32 = 2;

// A moving sprite with optional shadow, path or gravity motion, timers and platform riding
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub anim: AnimFrame,
    pub shadow: AnimFrame,
    pub id: i32,
    pub x: f32,
    pub y: f32,
    pub path_start_x: f32,
    pub path_start_y: f32,
    pub shadow_off_x: i32,
    pub shadow_off_y: i32,
    pub facing: i32,
    pub pending_facing: i32,
    pub step_x: f32,
    pub t: f32,
    pub steps: f32,
    pub vel_y: f32,
    pub gravity: f32,
    pub max_vel_y: f32,
    pub layer_mask: u32,
    pub target_x: i32,
    pub ground_y: i32,
    pub has_target: bool,
    pub notify_anim_end: bool,
    pub turning: bool,
    pub mouse_hittable: bool,
    pub visible: bool,
    pub has_turn_anims: bool,
    pub shadow_enabled: bool,
    pub screen_fixed: bool,
    pub path_phase: i32,
    pub timer: i32,
    pub platform_id: i32,
    pub flags: u32,
    pub platform_last_x: f32,
    pub platform_last_y: f32,
    pub anim_left: u16,
    pub anim_right: u16,
    pub turn_anim_a: u16,
    pub turn_anim_b: u16,
    pub bbox_anim: u16,
    pub draw_off_x: i32,
    pub draw_off_y: i32,
    pub delayed_count: i16,
    pub delayed_left: u16,
    pub delayed_right: u16,
    pub delayed_mode: i16,
    pub delayed_notify: bool,
}

impl Actor {
    fn new(id: i32) -> Self {
        Self {
            anim: AnimFrame { frame: 0.0, anim: NO_HANDLE, state: 0, fps: 0 },
            shadow: AnimFrame { frame: 0.0, anim: NO_HANDLE, state: 0, fps: 0 },
            id,
            x: 0.0,
            y: 0.0,
            path_start_x: 0.0,
            path_start_y: 0.0,
            shadow_off_x: 0,
            shadow_off_y: 0,
            facing: FACING_NONE,
            pending_facing: 0,
            step_x: 0.0,
            t: 0.0,
            steps: 0.0,
            vel_y: 0.0,
            gravity: 0.0,
            max_vel_y: 0.0,
            layer_mask: 1,
            target_x: 0,
            ground_y: 0,
            has_target: false,
            notify_anim_end: false,
            turning: false,
            mouse_hittable: false,
            visible: true,
            has_turn_anims: false,
            shadow_enabled: false,
            screen_fixed: false,
            path_phase: 0,
            timer: 0,
            platform_id: NO_PLATFORM,
            flags: 0,
            platform_last_x: 0.0,
            platform_last_y: 0.0,
            anim_left: NO_HANDLE,
            anim_right: NO_HANDLE,
            turn_anim_a: 0,
            turn_anim_b: 0,
            bbox_anim: NO_HANDLE,
            draw_off_x: 0,
            draw_off_y: 0,
            delayed_count: 0,
            delayed_left: 0,
            delayed_right: 0,
            delayed_mode: 0,
            delayed_notify: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub actor_id: i32,
    pub kind: i32,
}

#[derive(Debug, Default)]
pub struct ActorManager {
    pub actors: Vec<Actor>,
    pub events: Vec<Event>,
}

impl ActorManager {
    pub fn clear(&mut self) {
        self.actors.clear();
        self.events.clear();
    }

    // Appends a fresh actor and optionally queues its INIT_OBJECT event
    pub fn add_actor(&mut self, id: i32, post_init: bool) -> usize {
        self.actors.push(Actor::new(id));
        if post_init {
            self.push_event(EVENT_INIT_OBJECT, id);
        }
        self.actors.len() - 1
    }

    pub fn find(&self, id: i32) -> Option<usize> {
        self.actors.iter().position(|a| a.id == id)
    }

    fn idx(&self, id: i32) -> usize {
        self.find(id).unwrap_or_else(|| panic!("actor id {id} does not exist"))
    }

    pub fn get(&self, id: i32) -> &Actor {
        &self.actors[self.idx(id)]
    }

    pub fn get_mut(&mut self, id: i32) -> &mut Actor {
        let i = self.idx(id);
        &mut self.actors[i]
    }

    pub fn push_event(&mut self, kind: i32, actor_id: i32) {
        self.events.push(Event { actor_id, kind });
    }

    // Pops the most recently queued event
    pub fn pop_event(&mut self) -> Option<Event> {
        self.events.pop()
    }

    pub fn set_pos(&mut self, id: i32, x: f32, y: f32) {
        let a = self.get_mut(id);
        a.x = x;
        a.y = y;
    }

    pub fn add_x(&mut self, id: i32, dx: f32) {
        let a = self.get_mut(id);
        a.x += dx;
    }

    pub fn add_y(&mut self, id: i32, dy: f32) {
        let a = self.get_mut(id);
        a.y += dy;
    }

    pub fn set_x(&mut self, id: i32, x: f32) {
        self.get_mut(id).x = x;
    }

    pub fn set_y(&mut self, id: i32, y: f32) {
        self.get_mut(id).y = y;
    }

    pub fn pos(&self, id: i32) -> (f32, f32) {
        let a = self.get(id);
        (a.x, a.y)
    }

    pub fn vel_y(&self, id: i32) -> f32 {
        self.get(id).vel_y
    }

    pub fn set_draw_offset(&mut self, id: i32, ox: i32, oy: i32) {
        let a = self.get_mut(id);
        a.draw_off_x = ox;
        a.draw_off_y = oy;
    }

    pub fn set_bbox_anim(&mut self, id: i32, anim: u16) {
        self.get_mut(id).bbox_anim = anim;
    }

    pub fn set_shadow(&mut self, id: i32, anim: u16, ox: i32, oy: i32) {
        let a = self.get_mut(id);
        a.shadow.anim = anim;
        a.shadow.state = STATE_RESTART_FORWARD;
        a.shadow_off_x = ox;
        a.shadow_off_y = oy;
    }

    pub fn set_shadow_enabled(&mut self, id: i32, enabled: bool) {
        self.get_mut(id).shadow_enabled = enabled;
    }

    pub fn set_flags(&mut self, id: i32, flags: u32) {
        self.get_mut(id).flags = flags;
    }

    pub fn set_screen_fixed(&mut self, id: i32, fixed: bool) {
        self.get_mut(id).screen_fixed = fixed;
    }

    pub fn set_timer(&mut self, id: i32, ticks: i32) {
        self.get_mut(id).timer = ticks;
    }

    pub fn set_mouse_hittable(&mut self, id: i32, hittable: bool) {
        self.get_mut(id).mouse_hittable = hittable;
    }

    pub fn set_layer_mask(&mut self, id: i32, mask: u32) {
        self.get_mut(id).layer_mask = mask;
    }

    pub fn set_visible(&mut self, id: i32, visible: bool) {
        self.get_mut(id).visible = visible;
    }

    pub fn set_fps(&mut self, id: i32, fps: u16) {
        self.get_mut(id).anim.fps = fps;
    }

    pub fn target(&self, id: i32) -> i32 {
        let a = self.get(id);
        if a.has_target { a.target_x } else { -1 }
    }

    // Starts walking toward an x position, 9999 cancelling the walk
    pub fn walk_to(&mut self, av: &Av, id: i32, target_x: i32) {
        let a = self.get_mut(id);
        if target_x == NO_TARGET {
            a.has_target = false;
            return;
        }
        a.target_x = target_x;
        a.has_target = true;
        let dir = if a.x < target_x as f32 { FACING_RIGHT } else { FACING_LEFT };
        self.face(av, id, dir);
    }

    // Configures the back and forth path motion from a displacement and step length
    pub fn set_path(&mut self, id: i32, dx: i32, dy: i32, phase: f32, step_len: f32) {
        let a = self.get_mut(id);
        a.path_phase = 1;
        if step_len == 0.0 {
            a.steps = 0.0;
            return;
        }
        let (steps, step_x, step_y);
        if dy.abs() < dx.abs() {
            steps = ftol((dx.abs() as f32 / step_len) as f64);
            step_x = if dx < 0 { -step_len } else { step_len };
            let major = (dx as f32 / step_len).abs() as f64;
            step_y = (dy as f64 / major) as f32;
        } else {
            steps = ftol((dy.abs() as f32 / step_len) as f64);
            step_y = if dy < 0 { -step_len } else { step_len };
            let major = (dy as f32 / step_len).abs() as f64;
            step_x = (dx as f64 / major) as f32;
        }
        a.path_start_x = a.x;
        a.path_start_y = a.y;
        a.step_x = step_x;
        a.vel_y = step_y;
        a.steps = steps as f32;
        a.t = steps as f32 * phase;
    }

    pub fn set_gravity(&mut self, id: i32, vel_y: f32, gravity: f32, ground_y: i32, max_vel_y: f32) {
        let a = self.get_mut(id);
        a.vel_y = vel_y;
        a.ground_y = ground_y;
        a.gravity = gravity;
        a.max_vel_y = max_vel_y;
        a.platform_id = NO_PLATFORM;
    }

    // Restarts path motion with a new per tick speed and step count
    pub fn set_arc(&mut self, id: i32, t: f32, steps: f32) {
        let a = self.get_mut(id);
        a.step_x = 0.0;
        a.t = t;
        a.steps = steps;
    }

    // Installs the facing dependent animations and restarts playback in the given mode
    pub fn set_anims(&mut self, av: &Av, id: i32, right: u16, left: u16, notify_end: bool, mode: i16) {
        let a = self.get_mut(id);
        a.anim_left = left;
        a.anim_right = right;
        a.notify_anim_end = notify_end;
        let chosen = if a.facing == FACING_LEFT { left } else { right };
        a.anim.anim = chosen;
        a.anim.state = mode;
        a.anim.frame = 0.0;
        a.delayed_count = 0;
        av.get_anim_fps(chosen, &mut a.anim.fps);
    }

    // Applies the animation change now or after a countdown of ticks
    #[allow(clippy::too_many_arguments)]
    pub fn set_anims_delayed(&mut self, av: &Av, id: i32, delay: i16, right: u16, left: u16, notify_end: bool, mode: i16) {
        if delay == 0 {
            self.set_anims(av, id, right, left, notify_end, mode);
            return;
        }
        let a = self.get_mut(id);
        a.delayed_left = left;
        a.delayed_right = right;
        a.delayed_notify = notify_end;
        a.delayed_mode = mode;
        a.delayed_count = delay;
    }

    pub fn set_turn_anims(&mut self, id: i32, turn_to_right: u16, turn_to_left: u16) {
        let a = self.get_mut(id);
        a.turn_anim_a = turn_to_left;
        a.turn_anim_b = turn_to_right;
        a.has_turn_anims = true;
    }

    // Changes facing, playing a turn animation first when the actor has them
    pub fn face(&mut self, av: &Av, id: i32, dir: i32) {
        let a = self.get_mut(id);
        if !a.has_turn_anims || dir == FACING_FORCE_NONE {
            let dir = if dir == FACING_FORCE_NONE { FACING_NONE } else { dir };
            a.facing = dir;
            let chosen = if a.facing == FACING_LEFT { a.anim_left } else { a.anim_right };
            a.anim.anim = chosen;
            a.anim.state = STATE_RESTART_FORWARD;
            av.get_anim_fps(chosen, &mut a.anim.fps);
            a.turning = false;
            return;
        }
        if dir == a.facing {
            return;
        }
        a.pending_facing = dir;
        let mut mode = STATE_RESTART_FORWARD;
        let chosen = match a.facing {
            FACING_LEFT => {
                a.facing = FACING_NONE;
                mode = STATE_RESTART_BACKWARD;
                a.turn_anim_a
            }
            FACING_RIGHT => {
                a.facing = FACING_NONE;
                mode = STATE_RESTART_BACKWARD;
                a.turn_anim_b
            }
            _ => {
                let c = if a.pending_facing == FACING_RIGHT { a.turn_anim_b } else { a.turn_anim_a };
                a.facing = a.pending_facing;
                c
            }
        };
        a.anim.anim = chosen;
        a.anim.state = mode;
        a.step_x = 0.0;
        av.get_anim_fps(chosen, &mut a.anim.fps);
        a.turning = true;
    }

    // World space hit box of an actor from its bbox animation, hotspot and draw offset
    pub fn bbox(&self, av: &Av, id: i32) -> [i32; 4] {
        let a = self.get(id);
        let handle = if a.bbox_anim != NO_HANDLE { a.bbox_anim } else { a.anim.anim };
        let (mut hx, mut hy) = (0i32, 0i32);
        av.get_hot_spot(handle, &mut hx, &mut hy);
        let b = av.get_anim_bbox(handle);
        let ox = (ftol(a.x as f64) - hx) + a.draw_off_x;
        let oy = (ftol(a.y as f64) - hy) + a.draw_off_y;
        [b[0] + ox, b[1] + oy, b[2] + ox, b[3] + oy]
    }

    // True when two different actors' hit boxes touch, edges inclusive
    pub fn overlaps(&self, av: &Av, a: i32, b: i32) -> bool {
        if a == b {
            return false;
        }
        let ra = self.bbox(av, a);
        let rb = self.bbox(av, b);
        rb[0] <= ra[2] && ra[0] <= rb[2] && rb[1] <= ra[3] && ra[1] <= rb[3]
    }

    // Lands a falling actor on an overlapping moving platform, returning whether it landed
    pub fn try_land(&mut self, av: &Av, id: i32) -> bool {
        for i in 0..self.actors.len() {
            let p = &self.actors[i];
            if !p.visible || p.path_phase == 0 || p.flags & ACTOR_FLAG_PICKUP != 0 {
                continue;
            }
            let pid = p.id;
            if !self.overlaps(av, id, pid) {
                continue;
            }
            let mut vy = self.vel_y(id);
            let me = self.bbox(av, id);
            let plat = self.bbox(av, pid);
            if vy < 25.0 {
                vy = 25.0;
            }
            if plat[0] <= me[2] - 15 && me[0] + 15 <= plat[2] && (plat[1] as f32) < me[3] as f32 + vy {
                let (px, py) = (self.actors[i].x, self.actors[i].y);
                let flags = self.actors[i].flags;
                let a = self.get_mut(id);
                a.platform_id = pid;
                a.platform_last_x = px;
                a.platform_last_y = py;
                self.add_y(id, -((me[3] - plat[1]) as f32) + 10.0);
                if flags & ACTOR_FLAG_FINISH != 0 {
                    self.push_event(EVENT_FINISH, id);
                }
                return true;
            }
        }
        false
    }

    // Hides any pickup touching the actor and queues a PICKUP event for it
    pub fn collect_pickups(&mut self, av: &Av, id: i32) {
        for i in 0..self.actors.len() {
            let p = &self.actors[i];
            if !p.visible || p.path_phase == 0 || p.flags & ACTOR_FLAG_PICKUP == 0 {
                continue;
            }
            let pid = p.id;
            if self.overlaps(av, id, pid) {
                self.actors[i].visible = false;
                self.push_event(EVENT_PICKUP, id);
            }
        }
    }

    // Queues MOUSE_HIT for every visible hittable actor whose box contains the point
    pub fn mouse_hit_test(&mut self, av: &Av, x: i16, y: i16) {
        for i in 0..self.actors.len() {
            let a = &self.actors[i];
            if !a.visible || !a.mouse_hittable {
                continue;
            }
            let id = a.id;
            let b = self.bbox(av, id);
            let (x, y) = (x as i32, y as i32);
            if b[0] <= x && x <= b[2] && b[1] <= y && y <= b[3] {
                self.push_event(EVENT_MOUSE_HIT, id);
            }
        }
    }

    // Draws visible actors whose layer mask matches, shadows first, in creation order
    pub fn draw(&mut self, av: &mut Av, scene: &Scene, mask: u32) {
        for i in 0..self.actors.len() {
            let a = &self.actors[i];
            if !a.visible || a.layer_mask & mask == 0 {
                continue;
            }
            let mut x = a.draw_off_x + ftol(a.x as f64);
            let mut y = a.draw_off_y + ftol(a.y as f64);
            if !a.screen_fixed {
                scene.to_screen(&mut x, &mut y);
            }
            if a.shadow_enabled {
                let sx = (x + a.shadow_off_x) - a.draw_off_x;
                let sy = (y + a.shadow_off_y) - a.draw_off_y;
                let mut shadow = a.shadow;
                av.draw_anim(&mut shadow, sx, sy);
                self.actors[i].shadow = shadow;
            }
            let mut main = self.actors[i].anim;
            av.draw_anim(&mut main, x, y);
            self.actors[i].anim = main;
        }
    }

    // Advances every actor one tick with the original ordering, early return and rider pass
    pub fn update(&mut self, av: &Av) {
        for i in 0..self.actors.len() {
            if !self.actors[i].visible || self.actors[i].anim.anim == NO_HANDLE {
                continue;
            }
            let before = self.actors[i].anim;
            av.get_anim_next_frame(&mut self.actors[i].anim);
            let after = self.actors[i].anim;
            let old_mode = before.state;
            let ended = old_mode != STATE_RESTART_FORWARD
                && old_mode != STATE_RESTART_BACKWARD
                && ((after.state == 0 && old_mode != 0)
                    || after.state != old_mode
                    || (after.state == 1 && after.frame < before.frame)
                    || (after.state == 2 && before.frame < after.frame));
            if ended {
                let id = self.actors[i].id;
                if !self.actors[i].turning {
                    if self.actors[i].notify_anim_end {
                        self.push_event(EVENT_ANIM_END, id);
                    }
                } else if self.actors[i].pending_facing == self.actors[i].facing {
                    let a = &mut self.actors[i];
                    let chosen = if a.facing == FACING_LEFT { a.anim_left } else { a.anim_right };
                    a.anim.anim = chosen;
                    a.anim.state = STATE_RESTART_FORWARD;
                    av.get_anim_fps(chosen, &mut a.anim.fps);
                    a.turning = false;
                } else {
                    let pending = self.actors[i].pending_facing;
                    self.face(av, id, pending);
                }
            }
            if self.actors[i].delayed_count != 0 {
                self.actors[i].delayed_count -= 1;
                if self.actors[i].delayed_count == 0 {
                    let a = &self.actors[i];
                    let (id, r, l, n, m) = (a.id, a.delayed_right, a.delayed_left, a.delayed_notify, a.delayed_mode);
                    self.set_anims(av, id, r, l, n, m);
                }
            }
            if self.actors[i].timer != 0 {
                if self.actors[i].timer == 1 {
                    let id = self.actors[i].id;
                    self.push_event(EVENT_TIMER, id);
                }
                self.actors[i].timer -= 1;
            }
            {
                let a = &mut self.actors[i];
                if a.has_target && !a.turning {
                    if a.facing == FACING_LEFT {
                        a.x -= a.t;
                        if a.x <= a.target_x as f32 {
                            a.x = a.target_x as f32;
                            let id = a.id;
                            a.has_target = false;
                            self.push_event(EVENT_TARGET_REACHED, id);
                        }
                    } else {
                        a.x += a.t;
                        if a.target_x as f32 <= a.x {
                            a.x = a.target_x as f32;
                            let id = a.id;
                            a.has_target = false;
                            self.push_event(EVENT_TARGET_REACHED, id);
                        }
                    }
                }
            }
            let a = &self.actors[i];
            if a.path_phase == 0 || a.steps == 0.0 {
                let id = a.id;
                if a.gravity != 0.0 {
                    let a = &mut self.actors[i];
                    a.vel_y += a.gravity;
                    if a.max_vel_y < a.vel_y {
                        a.vel_y = a.max_vel_y;
                    }
                    if 0.0 < a.vel_y && self.try_land(av, id) {
                        let a = &mut self.actors[i];
                        a.gravity = 0.0;
                        a.vel_y = 0.0;
                        self.push_event(EVENT_GROUND_REACHED, id);
                    }
                }
                let a = &mut self.actors[i];
                if a.vel_y != 0.0 {
                    a.y += a.vel_y;
                    if (a.ground_y as f32) < a.y {
                        a.y = a.ground_y as f32;
                        a.gravity = 0.0;
                        a.vel_y = 0.0;
                        self.push_event(EVENT_GROUND_REACHED, id);
                        return;
                    }
                }
            } else {
                let a = &mut self.actors[i];
                if a.path_phase == 1 {
                    a.t += 1.0;
                    if a.steps < a.t {
                        a.t = a.steps;
                        a.path_phase = 2;
                    }
                } else {
                    a.t -= 1.0;
                    if a.t < 0.0 {
                        a.t = 0.0;
                        a.path_phase = 1;
                    }
                }
                a.x = a.step_x * a.t + a.path_start_x;
                a.y = a.vel_y * a.t + a.path_start_y;
            }
        }
        for i in 0..self.actors.len() {
            let a = &self.actors[i];
            if !a.visible || a.platform_id == NO_PLATFORM {
                continue;
            }
            let p = self.idx(a.platform_id);
            let dx = self.actors[p].x - a.platform_last_x;
            let dy = self.actors[p].y - a.platform_last_y;
            let (px, py) = (self.actors[p].x, self.actors[p].y);
            let a = &mut self.actors[i];
            a.x += dx;
            a.y += dy;
            a.platform_last_x = px;
            a.platform_last_y = py;
            let id = a.id;
            let pid = a.platform_id;
            let me = self.bbox(av, id);
            let plat = self.bbox(av, pid);
            if me[2] - 15 < plat[0] || plat[2] < me[0] + 15 {
                self.actors[i].platform_id = NO_PLATFORM;
                self.push_event(EVENT_UNATTACHED, id);
            }
        }
    }
}
