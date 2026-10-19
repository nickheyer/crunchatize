#![forbid(unsafe_code)]

mod keys;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use avmanager::input::DeviceState;
use avmanager::sound::Sound;
use crunch::host::Host;
use js_sys::{Array, Float32Array, Function, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    AudioContext, AudioContextOptions, AudioContextState, AudioWorkletNode, AudioWorkletNodeOptions,
    CanvasRenderingContext2d, HtmlCanvasElement, ImageData, KeyboardEvent, MouseEvent, Response, Window,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const MOUSE_GAIN: f64 = 2.0;
const SAVE_PREFIX: &str = "crunch.save.";
const OUT_RATE: u32 = 48000;
const TARGET_BUFFER_SECONDS: f64 = 0.12;
const MAX_CATCH_UP_SECONDS: f64 = 1.0;
const GESTURE_EVENTS: [&str; 4] = ["pointerdown", "pointerup", "keydown", "touchend"];

// Input gathered by browser event listeners between game ticks
#[derive(Default)]
struct InputState {
    keys: HashMap<usize, bool>,
    dx: f64,
    dy: f64,
    buttons: [bool; 3],
    debug_toggle: bool,
}

// Speaker output that exists once the browser lets audio start, the node arriving after the worklet loads
struct AudioOut {
    ctx: AudioContext,
    node: Option<AudioWorkletNode>,
}

type SharedAudio = Rc<RefCell<Option<AudioOut>>>;

// Browser shell drawing to a canvas, mixing audio into a worklet, and saving to local storage
pub struct WebHost {
    base_url: String,
    canvas: HtmlCanvasElement,
    ctx2d: CanvasRenderingContext2d,
    rgba: Vec<u8>,
    audio: SharedAudio,
    audio_queued: f64,
    last_audio_time: f64,
    last_feed_ms: f64,
    input: Rc<RefCell<InputState>>,
    start: f64,
}

fn now_ms() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0)
}

impl WebHost {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    // Converts the 5-6-5 frame into RGBA and blits it to the canvas
    fn present(&mut self, frame: &[u16]) {
        for (i, &p) in frame.iter().enumerate() {
            let r = (((p >> 11) & 0x1f) << 3) as u8;
            let g = (((p >> 5) & 0x3f) << 2) as u8;
            let b = ((p & 0x1f) << 3) as u8;
            self.rgba[i * 4] = r;
            self.rgba[i * 4 + 1] = g;
            self.rgba[i * 4 + 2] = b;
            self.rgba[i * 4 + 3] = 255;
        }
        if let Ok(image) = ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&self.rgba), WIDTH, HEIGHT) {
            let _ = self.ctx2d.put_image_data(&image, 0.0, 0.0);
        }
    }

    // Advances the mixer in real time and posts its output to the worklet whenever the context is running
    fn feed_audio(&mut self, sound: &mut Sound) {
        let wall = now_ms();
        let wall_elapsed = ((wall - self.last_feed_ms) / 1000.0).clamp(0.0, MAX_CATCH_UP_SECONDS);
        self.last_feed_ms = wall;
        let audio = self.audio.borrow();
        let live = audio
            .as_ref()
            .filter(|a| a.ctx.state() == AudioContextState::Running)
            .and_then(|a| a.node.as_ref().map(|n| (&a.ctx, n)));
        let Some((ctx, node)) = live else {
            let frames = (wall_elapsed * OUT_RATE as f64) as usize;
            if frames > 0 {
                sound.render(&mut vec![0i16; frames * 2]);
            }
            self.audio_queued = 0.0;
            self.last_audio_time = audio.as_ref().map_or(0.0, |a| a.ctx.current_time());
            return;
        };
        let now = ctx.current_time();
        let elapsed = (now - self.last_audio_time).max(0.0);
        self.last_audio_time = now;
        self.audio_queued = (self.audio_queued - elapsed).max(0.0);
        let frames = ((TARGET_BUFFER_SECONDS - self.audio_queued) * OUT_RATE as f64) as usize;
        if frames == 0 {
            return;
        }
        let mut buf = vec![0i16; frames * 2];
        sound.render(&mut buf);
        let floats: Vec<f32> = buf.iter().map(|&s| s as f32 / 32768.0).collect();
        let array = Float32Array::from(floats.as_slice());
        if let Ok(port) = node.port() {
            let _ = port.post_message_with_transferable(&array, &Array::of1(&array.buffer()));
        }
        self.audio_queued += frames as f64 / OUT_RATE as f64;
    }
}

async fn next_animation_frame() {
    let promise = Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.request_animation_frame(&resolve);
        }
    });
    let _ = JsFuture::from(promise).await;
}

// Downloads a URL, failing on any status outside the 2xx range
async fn fetch_ok(window: &Window, url: &str) -> Result<Response, JsValue> {
    let response: Response = JsFuture::from(window.fetch_with_str(url)).await?.dyn_into()?;
    if !response.ok() {
        return Err(format!("{url}: HTTP {}", response.status()).into());
    }
    Ok(response)
}

async fn fetch_text(window: &Window, url: &str) -> Result<String, JsValue> {
    let response = fetch_ok(window, url).await?;
    JsFuture::from(response.text()?).await?.as_string().ok_or_else(|| "response body is not text".into())
}

impl Host for WebHost {
    async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
        let file = name.rsplit(['\\', '/']).next().unwrap_or(name).to_lowercase();
        let url = format!("{}{}", self.base_url, file);
        let response = fetch_ok(&web_sys::window()?, &url).await.ok()?;
        let buffer = JsFuture::from(response.array_buffer().ok()?).await.ok()?;
        Some(Uint8Array::new(&buffer).to_vec())
    }

    fn time_ms(&self) -> u32 {
        (now_ms() - self.start) as u32
    }

    fn unix_time(&self) -> u32 {
        (js_sys::Date::now() / 1000.0) as u32
    }

    fn poll_input(&mut self) -> DeviceState {
        let mut input = self.input.borrow_mut();
        let mut state = DeviceState::new();
        for (&code, &down) in &input.keys {
            if down && code < 256 {
                state.scan_keys[code] = true;
            }
        }
        state.mouse_dx = input.dx.round() as i32;
        state.mouse_dy = input.dy.round() as i32;
        input.dx = 0.0;
        input.dy = 0.0;
        state.buttons = input.buttons;
        state
    }

    async fn end_frame(&mut self, frame: Option<&[u16]>, sound: &mut Sound) {
        if let Some(f) = frame {
            self.present(f);
        }
        self.feed_audio(sound);
        next_animation_frame().await;
    }

    fn list_saves(&mut self) -> Vec<String> {
        let Some(storage) = Self::storage() else { return Vec::new() };
        let mut names = Vec::new();
        let count = storage.length().unwrap_or(0);
        for i in 0..count {
            if let Ok(Some(key)) = storage.key(i)
                && let Some(name) = key.strip_prefix(SAVE_PREFIX)
            {
                names.push(name.to_string());
            }
        }
        names.sort();
        names
    }

    fn read_save(&mut self, name: &str) -> Option<Vec<u8>> {
        let storage = Self::storage()?;
        let text = storage.get_item(&format!("{SAVE_PREFIX}{name}")).ok()??;
        decode_hex(&text)
    }

    fn write_save(&mut self, name: &str, data: &[u8]) -> bool {
        let Some(storage) = Self::storage() else { return false };
        storage.set_item(&format!("{SAVE_PREFIX}{name}"), &encode_hex(data)).is_ok()
    }

    fn save_exists(&mut self, name: &str) -> bool {
        Self::storage()
            .and_then(|s| s.get_item(&format!("{SAVE_PREFIX}{name}")).ok().flatten())
            .is_some()
    }

    fn delete_save(&mut self, name: &str) {
        if let Some(storage) = Self::storage() {
            let _ = storage.remove_item(&format!("{SAVE_PREFIX}{name}"));
        }
    }

    fn take_debug_toggle(&mut self) -> bool {
        std::mem::take(&mut self.input.borrow_mut().debug_toggle)
    }

    fn quit_requested(&self) -> bool {
        false
    }

    fn open_url(&mut self, url: &str) {
        if let Some(w) = web_sys::window() {
            let _ = w.open_with_url_and_target(url, "_blank");
        }
    }

    fn launch_setup(&mut self) {
        web_sys::console::log_1(&"Setup.exe launch requested".into());
    }

    fn log(&mut self, message: &str) {
        web_sys::console::log_1(&message.into());
    }
}

fn encode_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok()).collect()
}

// True when the page already holds a user activation or the browser reports audio may start without one
fn autoplay_allowed(window: &Window) -> bool {
    let navigator = window.navigator();
    let activation = Reflect::get(&navigator, &"userActivation".into()).ok();
    let active = activation.and_then(|a| Reflect::get(&a, &"hasBeenActive".into()).ok());
    if active.is_some_and(|v| v.is_truthy()) {
        return true;
    }
    let policy = Reflect::get(&navigator, &"getAutoplayPolicy".into()).ok();
    let Some(query) = policy.and_then(|p| p.dyn_into::<Function>().ok()) else { return false };
    let verdict = query.call1(&navigator, &"audiocontext".into()).ok().and_then(|v| v.as_string());
    verdict.as_deref() == Some("allowed")
}

// Creates the context inside the calling gesture and loads the worklet, after which output begins
fn open_audio(shared: &SharedAudio, worklet_url: &str) -> Result<(), JsValue> {
    let options = AudioContextOptions::new();
    options.set_sample_rate(OUT_RATE as f32);
    let ctx = AudioContext::new_with_context_options(&options)?;
    let _ = ctx.resume();
    *shared.borrow_mut() = Some(AudioOut { ctx: ctx.clone(), node: None });
    let shared = shared.clone();
    let url = worklet_url.to_string();
    spawn_local(async move {
        if let Err(e) = install_worklet(&shared, &ctx, &url).await {
            web_sys::console::error_1(&e);
        }
    });
    Ok(())
}

async fn install_worklet(shared: &SharedAudio, ctx: &AudioContext, url: &str) -> Result<(), JsValue> {
    JsFuture::from(ctx.audio_worklet()?.add_module(url)?).await?;
    let options = AudioWorkletNodeOptions::new();
    options.set_number_of_inputs(0);
    options.set_output_channel_count(&Array::of1(&2.into()));
    let node = AudioWorkletNode::new_with_options(ctx, "crunch-output", &options)?;
    node.connect_with_audio_node(&ctx.destination())?;
    if let Some(out) = shared.borrow_mut().as_mut() {
        out.node = Some(node);
    }
    Ok(())
}

// Creates or resumes the audio context on every input event browsers count as a user gesture
fn attach_audio_unlock(window: &Window, shared: SharedAudio, worklet_url: String) {
    let unlock = Closure::<dyn FnMut()>::new(move || {
        let state = shared.borrow().as_ref().map(|a| a.ctx.state());
        match state {
            None => {
                if let Err(e) = open_audio(&shared, &worklet_url) {
                    web_sys::console::error_1(&e);
                }
            }
            Some(AudioContextState::Running) => {}
            Some(_) => {
                if let Some(out) = shared.borrow().as_ref() {
                    let _ = out.ctx.resume();
                }
            }
        }
    });
    for event in GESTURE_EVENTS {
        let _ = window.add_event_listener_with_callback_and_bool(event, unlock.as_ref().unchecked_ref(), true);
    }
    unlock.forget();
}

// Installs keyboard and mouse listeners on the canvas that accumulate into the shared state
fn attach_input(canvas: &HtmlCanvasElement, input: Rc<RefCell<InputState>>) {
    let down = input.clone();
    let on_key_down = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
        let code = keys::scan_code(&e.code());
        let mut s = down.borrow_mut();
        if e.code() == "F1" && !e.repeat() {
            s.debug_toggle = true;
        }
        if code != 0 {
            s.keys.insert(code, true);
            e.prevent_default();
        }
    });
    let up = input.clone();
    let on_key_up = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
        let code = keys::scan_code(&e.code());
        if code != 0 {
            up.borrow_mut().keys.insert(code, false);
            e.prevent_default();
        }
    });
    let moved = input.clone();
    let scale_canvas = canvas.clone();
    let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
        let rect_w = scale_canvas.client_width().max(1) as f64;
        let scale = WIDTH as f64 / rect_w;
        let mut s = moved.borrow_mut();
        s.dx += e.movement_x() as f64 * scale * MOUSE_GAIN;
        s.dy += e.movement_y() as f64 * scale * MOUSE_GAIN;
    });
    let pressed = input.clone();
    let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
        let b = e.button() as usize;
        if b < 3 {
            let idx = [0usize, 2, 1][b];
            pressed.borrow_mut().buttons[idx] = true;
        }
        e.prevent_default();
    });
    let released = input.clone();
    let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
        let b = e.button() as usize;
        if b < 3 {
            let idx = [0usize, 2, 1][b];
            released.borrow_mut().buttons[idx] = false;
        }
    });
    let window = web_sys::window().expect("window");
    let _ = window.add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref());
    let _ = window.add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref());
    let _ = canvas.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
    let _ = canvas.add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref());
    let _ = window.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
    on_key_down.forget();
    on_key_up.forget();
    on_move.forget();
    on_down.forget();
    on_up.forget();
}

// Boots the game on a canvas, reading files from the pack version the data URL currently serves
#[wasm_bindgen]
pub async fn start(canvas: HtmlCanvasElement, data_url: String, worklet_url: String) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().ok_or("no window")?;
    canvas.set_width(WIDTH);
    canvas.set_height(HEIGHT);
    let ctx2d: CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or("no 2d context")?
        .dyn_into()?;
    let audio: SharedAudio = Rc::new(RefCell::new(None));
    if autoplay_allowed(&window) {
        open_audio(&audio, &worklet_url)?;
    }
    attach_audio_unlock(&window, audio.clone(), worklet_url);
    let version = fetch_text(&window, &format!("{data_url}version")).await?;
    let input = Rc::new(RefCell::new(InputState::default()));
    attach_input(&canvas, input.clone());
    let host = WebHost {
        base_url: format!("{data_url}{}/", version.trim()),
        canvas: canvas.clone(),
        ctx2d,
        rgba: vec![0; (WIDTH * HEIGHT * 4) as usize],
        audio,
        audio_queued: 0.0,
        last_audio_time: 0.0,
        last_feed_ms: now_ms(),
        input,
        start: now_ms(),
    };
    let _ = host.canvas.focus();
    crunch::game::run(host, OUT_RATE).await;
    let _ = window.location().reload();
    Ok(())
}
