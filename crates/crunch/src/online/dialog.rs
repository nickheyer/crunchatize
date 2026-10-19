use avmanager::input::Input;

use super::session::{self, Notice, Phase};
use super::wire::CODE_LEN;
use crate::ctx::{Ctx, KEY_BACKSPACE, KEY_DEL, KEY_ESC, MOUSE_LEFT};
use crate::cursor::Cursor;
use crate::host::Host;
use crate::scene::{HOVER_HIDDEN, HOVER_NORMAL, LAYER_ALL, Scene};

const KEY_RETURN: i32 = 0x1c;
const BLINK_PERIOD: i32 = 0x10;
const BLINK_OFF_AFTER: i32 = 0xb;

const ID_LABEL: i32 = 2;
const ID_CODE: i32 = 3;
const ID_STATUS: i32 = 4;
const ID_M_HOST: i32 = 5;
const ID_M_JOIN: i32 = 6;
const ID_M_BACK: i32 = 7;
const ID_H_BACK: i32 = 8;
const ID_J_JOIN: i32 = 9;
const ID_J_BACK: i32 = 10;
const ID_S_LEAVE: i32 = 11;
const ID_S_BACK: i32 = 12;

const OBJECT_IDS: [(&str, i32); 11] = [
    ("label", ID_LABEL),
    ("code", ID_CODE),
    ("status", ID_STATUS),
    ("m_host", ID_M_HOST),
    ("m_join", ID_M_JOIN),
    ("m_back", ID_M_BACK),
    ("h_back", ID_H_BACK),
    ("j_join", ID_J_JOIN),
    ("j_back", ID_J_BACK),
    ("s_leave", ID_S_LEAVE),
    ("s_back", ID_S_BACK),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Menu,
    Host,
    Join,
    Session,
}

impl Page {
    fn pills(self) -> &'static [i32] {
        match self {
            Page::Menu => &[ID_M_HOST, ID_M_JOIN, ID_M_BACK],
            Page::Host => &[ID_H_BACK],
            Page::Join => &[ID_J_JOIN, ID_J_BACK],
            Page::Session => &[ID_S_LEAVE, ID_S_BACK],
        }
    }
}

// Page the connection phase calls for, keeping a page the player is reading when nothing is happening
fn page_for(phase: &Phase, current: Page) -> Page {
    match phase {
        Phase::Offline => {
            if current == Page::Session {
                Page::Menu
            } else {
                current
            }
        }
        Phase::HostConnecting | Phase::HostWaiting { .. } => Page::Host,
        Phase::JoinConnecting { .. } | Phase::JoinWaiting { .. } => Page::Join,
        Phase::HostPlaying { .. } | Phase::JoinPlaying { .. } => Page::Session,
    }
}

fn notice_text(notice: Notice) -> &'static str {
    match notice {
        Notice::Nothing => "",
        Notice::FriendJoined => "FRIEND JOINED",
        Notice::FriendLeft => "FRIEND LEFT",
        Notice::HostLeft => "HOST LEFT",
        Notice::NoSuchGame => "NO SUCH GAME",
        Notice::GameFull => "GAME IS FULL",
        Notice::CouldNotConnect => "COULD NOT CONNECT",
        Notice::ConnectionLost => "CONNECTION LOST",
    }
}

// True for the characters a game code is made of
fn accepts_code_char(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

// The Go Online box drawn over any screen, driven by this machine's own hand and keyboard
pub struct Dialog {
    scene: Scene,
    click: u16,
    page: Page,
    entry: String,
    blink: i32,
    latch: bool,
    cursor: Cursor,
}

impl Dialog {
    // Loads the box scene and opens it on the page matching the connection phase
    pub async fn open<H: Host>(ctx: &mut Ctx<H>) -> Option<Self> {
        let mut scene = Scene::new();
        if !scene.load(&mut ctx.av, &mut ctx.host, "online").await {
            return None;
        }
        for (name, id) in OBJECT_IDS {
            scene.assign_id(name, id);
        }
        let click = ctx.av.get_sfx(&mut ctx.host, "Button1.wav", 0, false).await?;
        let art = if ctx.session.local_is_helper() { "mouse_online" } else { "mouse" };
        let hand = ctx.av.get_anim(&mut ctx.host, art).await?;
        let mut cursor = Cursor::default();
        cursor.set_anim(&ctx.av, hand);
        cursor.set_visible(true);
        let mut dialog = Self { scene, click, page: Page::Menu, entry: String::new(), blink: 0, latch: true, cursor };
        let page = page_for(ctx.session.phase(), Page::Menu);
        dialog.show(if page == Page::Menu && ctx.session.notice() != Notice::Nothing { Page::Join } else { page });
        if let Some(code) = ctx.session.code() {
            dialog.entry = code.to_string();
        }
        Some(dialog)
    }

    pub fn page(&self) -> Page {
        self.page
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    fn show(&mut self, page: Page) {
        for id in ID_M_HOST..=ID_S_BACK {
            self.scene.set_hover_by_id(id, HOVER_HIDDEN);
        }
        for &id in page.pills() {
            self.scene.set_hover_by_id(id, HOVER_NORMAL);
        }
        self.page = page;
        self.latch = true;
    }

    // Runs one tick of clicks and typing, false once the box should close
    pub fn update<H: Host>(&mut self, ctx: &mut Ctx<H>) -> bool {
        let wanted = page_for(ctx.session.phase(), self.page);
        if wanted != self.page {
            self.show(wanted);
        }
        let (mx, my) = ctx.session.local_pos((ctx.cursor.x, ctx.cursor.y));
        self.cursor.set_pos(mx, my);
        let down = ctx.session.local.is_mouse_button_down(MOUSE_LEFT);
        let mut action = None;
        if let Some(id) = self.scene.hit_test(i32::from(mx), i32::from(my))
            && down
            && !self.latch
        {
            action = Some(id);
        }
        self.latch = down;
        let key = ctx.session.local.key_input();
        if key == KEY_ESC {
            action = Some(match self.page {
                Page::Menu => ID_M_BACK,
                Page::Host => ID_H_BACK,
                Page::Join => ID_J_BACK,
                Page::Session => ID_S_BACK,
            });
        } else if key != 0 && self.page == Page::Join {
            self.type_key(ctx, key, &mut action);
        }
        if let Some(id) = action {
            ctx.av.play_sfx(self.click, -1);
            if !self.click(ctx, id) {
                return false;
            }
        }
        self.blink += 1;
        if self.blink > BLINK_PERIOD {
            self.blink = 0;
        }
        self.refresh_text(ctx);
        true
    }

    // Edits the code entry with letters, digits, backspace and return
    fn type_key<H: Host>(&mut self, ctx: &mut Ctx<H>, key: i32, action: &mut Option<i32>) {
        if *ctx.session.phase() != Phase::Offline {
            return;
        }
        if key == KEY_RETURN {
            *action = Some(ID_J_JOIN);
        } else if key == KEY_BACKSPACE || key == KEY_DEL {
            self.entry.pop();
            ctx.session.clear_notice();
        } else if let Some(name) = Input::translate_key_def(key)
            && name.len() == 1
            && accepts_code_char(name.as_bytes()[0])
            && self.entry.len() < CODE_LEN
        {
            self.entry.push_str(name);
            ctx.session.clear_notice();
        }
    }

    // Acts on a pill by id, false when the box closes
    fn click<H: Host>(&mut self, ctx: &mut Ctx<H>, id: i32) -> bool {
        match id {
            ID_M_HOST => {
                session::start_hosting(ctx);
                self.show(Page::Host);
            }
            ID_M_JOIN => {
                ctx.session.clear_notice();
                self.show(Page::Join);
            }
            ID_M_BACK | ID_S_BACK => return false,
            ID_H_BACK | ID_J_BACK => {
                session::cancel(ctx);
                ctx.session.clear_notice();
                self.show(Page::Menu);
            }
            ID_J_JOIN if self.entry.len() == CODE_LEN => session::join(ctx, &self.entry.clone()),
            ID_S_LEAVE => {
                let helper = ctx.session.local_is_helper();
                session::leave(ctx);
                if helper {
                    return false;
                }
                self.show(Page::Menu);
            }
            _ => {}
        }
        true
    }

    // Fills the label, code and status lines from the page and connection phase
    fn refresh_text<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        let phase = ctx.session.phase().clone();
        let notice = notice_text(ctx.session.notice());
        let code = ctx.session.code().unwrap_or("").to_string();
        let (label, code_line, status): (String, String, String) = match self.page {
            Page::Menu => (String::new(), String::new(), String::new()),
            Page::Host => {
                let status = match phase {
                    Phase::HostConnecting => "CONNECTING",
                    Phase::HostWaiting { .. } if notice.is_empty() => "WAITING FOR A FRIEND",
                    _ => notice,
                };
                ("YOUR CODE".into(), code, status.into())
            }
            Page::Join => {
                let status = match phase {
                    Phase::JoinConnecting { .. } | Phase::JoinWaiting { .. } => "CONNECTING",
                    _ => notice,
                };
                ("ENTER A CODE".into(), self.entry.clone(), status.into())
            }
            Page::Session => {
                if ctx.session.local_is_helper() {
                    ("PLAYING ONLINE".into(), String::new(), String::new())
                } else {
                    (format!("CODE {code}"), String::new(), "FRIEND JOINED".into())
                }
            }
        };
        self.scene.set_text_by_id(ID_LABEL, &label);
        self.scene.set_text_by_id(ID_CODE, &code_line);
        self.scene.set_text_by_id(ID_STATUS, &status);
        let typing = self.page == Page::Join && phase == Phase::Offline;
        let cursor = if typing && self.blink <= BLINK_OFF_AFTER { 1 } else { 0 };
        self.scene.set_text_cursor_by_id(ID_CODE, cursor);
    }

    pub fn draw<H: Host>(&mut self, ctx: &mut Ctx<H>) {
        self.scene.draw_layer(&mut ctx.av, LAYER_ALL);
        self.scene.advance_anims(&ctx.av);
        self.cursor.draw(&mut ctx.av);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_follow_the_phase() {
        assert_eq!(page_for(&Phase::Offline, Page::Join), Page::Join);
        assert_eq!(page_for(&Phase::Offline, Page::Session), Page::Menu);
        assert_eq!(page_for(&Phase::HostConnecting, Page::Menu), Page::Host);
        assert_eq!(page_for(&Phase::JoinWaiting { code: "ABCD".into() }, Page::Menu), Page::Join);
        assert_eq!(page_for(&Phase::HostPlaying { code: "ABCD".into() }, Page::Host), Page::Session);
    }

    #[test]
    fn code_entry_takes_capitals_and_digits_only() {
        assert!(accepts_code_char(b'A') && accepts_code_char(b'Z') && accepts_code_char(b'7'));
        assert!(!accepts_code_char(b'a') && !accepts_code_char(b' ') && !accepts_code_char(b'-'));
    }
}
