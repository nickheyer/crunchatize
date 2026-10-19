#![forbid(unsafe_code)]

mod keys;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use avmanager::input::DeviceState;
use avmanager::sound::Sound;
use crunch::host::Host;
use crunch_pak::Index;
use js_sys::{Array, Float32Array, Function, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    AudioContext, AudioContextOptions, AudioContextState, AudioWorkletNode, AudioWorkletNodeOptions, Cache,
    CanvasRenderingContext2d, HtmlCanvasElement, ImageData, KeyboardEvent, PointerEvent, ReadableStreamDefaultReader,
    ReadableStreamReadResult, Request, Response, Url, Window,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const SAVE_PREFIX: &str = "crunch.save.";
const OUT_RATE: u32 = 48000;
const TARGET_BUFFER_SECONDS: f64 = 0.12;
const MAX_CATCH_UP_SECONDS: f64 = 1.0;
const GESTURE_EVENTS: [&str; 4] = ["pointerdown", "pointerup", "keydown", "touchend"];
const PACK: &str = "crunch.pak";
const CACHE_NAME: &str = "crunch";
const BAR_WIDTH: f64 = 400.0;

// Input gathered by browser event listeners between game ticks
#[derive(Default)]
struct InputState {
    keys: HashMap<usize, bool>,
    pointer: Option<(i16, i16)>,
    buttons: [bool; 3],
    debug_toggle: bool,
}

// Speaker output that exists once the browser lets audio start, the node arriving after the worklet loads
struct AudioOut {
    ctx: AudioContext,
    node: Option<AudioWorkletNode>,
}

type SharedAudio = Rc<RefCell<Option<AudioOut>>>;

// The whole data pack held in memory with its parsed index
struct Pack {
    bytes: Vec<u8>,
    index: Index,
}

impl Pack {
    fn parse(bytes: Vec<u8>) -> Result<Self, JsValue> {
        let index = crunch_pak::read_index(&mut bytes.as_slice()).map_err(|e| JsValue::from(e.to_string()))?;
        Ok(Self { bytes, index })
    }

    // Bytes of one file by lower case name, None when the pack has no such file
    fn file(&self, name: &str) -> Option<&[u8]> {
        let entry = self.index.files.get(name)?;
        let start = usize::try_from(self.index.body_start + entry.offset).ok()?;
        let end = start.checked_add(usize::try_from(entry.size).ok()?)?;
        self.bytes.get(start..end)
    }
}

// Browser shell drawing to a canvas, mixing audio into a worklet, and saving to local storage
pub struct WebHost {
    pack: Pack,
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
        self.pack.file(&file).map(<[u8]>::to_vec)
    }

    fn time_ms(&self) -> u32 {
        (now_ms() - self.start) as u32
    }

    fn unix_time(&self) -> u32 {
        (js_sys::Date::now() / 1000.0) as u32
    }

    fn poll_input(&mut self) -> DeviceState {
        let input = self.input.borrow();
        let mut state = DeviceState::new();
        for (&code, &down) in &input.keys {
            if down && code < 256 {
                state.scan_keys[code] = true;
            }
        }
        state.pointer = input.pointer;
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

// Paints a caption and a progress bar on the black canvas while the pack loads
fn draw_progress(ctx2d: &CanvasRenderingContext2d, caption: &str, fraction: f64) {
    let (w, h) = (f64::from(WIDTH), f64::from(HEIGHT));
    let left = (w - BAR_WIDTH) / 2.0;
    ctx2d.set_fill_style_str("#000");
    ctx2d.fill_rect(0.0, 0.0, w, h);
    ctx2d.set_fill_style_str("#ddd");
    ctx2d.set_stroke_style_str("#ddd");
    ctx2d.set_font("16px sans-serif");
    ctx2d.set_text_align("center");
    let _ = ctx2d.fill_text(caption, w / 2.0, h / 2.0 - 12.0);
    ctx2d.fill_rect(left, h / 2.0, BAR_WIDTH * fraction, 10.0);
    ctx2d.stroke_rect(left + 0.5, h / 2.0 + 0.5, BAR_WIDTH, 10.0);
}

// Message text of a thrown value, the string itself or an Error's message
fn describe(e: &JsValue) -> String {
    e.as_string()
        .or_else(|| e.dyn_ref::<js_sys::Error>().map(|err| String::from(err.message())))
        .unwrap_or_else(|| format!("{e:?}"))
}

// Opens the cache that keeps the pack between visits, None with a console warning where the browser has none
async fn open_cache(window: &Window) -> Option<Cache> {
    let opened: Result<Cache, JsValue> = async {
        let caches = window.caches()?;
        JsFuture::from(caches.open(CACHE_NAME)).await?.dyn_into()
    }
    .await;
    match opened {
        Ok(cache) => Some(cache),
        Err(e) => {
            web_sys::console::warn_1(&e);
            None
        }
    }
}

// Deletes every cached pack except the one at the given address
async fn prune_cache(cache: &Cache, keep: &str) -> Result<(), JsValue> {
    let keys: Array = JsFuture::from(cache.keys()).await?.dyn_into()?;
    for key in keys.iter() {
        let request: Request = key.dyn_into()?;
        if request.url() != keep {
            JsFuture::from(cache.delete_with_request(&request)).await?;
        }
    }
    Ok(())
}

// Downloads the pack and stores a copy in the cache while the download streams on to the caller
async fn fetch_and_keep(window: &Window, cache: Option<&Cache>, url: &str) -> Result<Response, JsValue> {
    let response = fetch_ok(window, url).await?;
    let Some(cache) = cache else { return Ok(response) };
    if let Err(e) = prune_cache(cache, url).await {
        web_sys::console::warn_1(&e);
    }
    let copy = response.clone()?;
    let cache = cache.clone();
    let key = url.to_string();
    spawn_local(async move {
        if let Err(e) = JsFuture::from(cache.put_with_str(&key, &copy)).await {
            web_sys::console::warn_1(&e);
        }
    });
    Ok(response)
}

// Streams a response body into memory, reporting bytes received and the total at every whole percent
async fn read_body(response: &Response, mut progress: impl FnMut(usize, usize)) -> Result<Vec<u8>, JsValue> {
    let total: usize = response
        .headers()
        .get("content-length")?
        .and_then(|v| v.parse().ok())
        .ok_or("pack response has no content-length")?;
    let body = response.body().ok_or("pack response has no body")?;
    let reader: ReadableStreamDefaultReader = body.get_reader().unchecked_into();
    let mut bytes = Vec::with_capacity(total);
    let mut shown = usize::MAX;
    loop {
        let result: ReadableStreamReadResult = JsFuture::from(reader.read()).await?.unchecked_into();
        if result.get_done().unwrap_or(false) {
            break;
        }
        let chunk: Uint8Array = result.get_value().dyn_into()?;
        let start = bytes.len();
        bytes.resize(start + chunk.length() as usize, 0);
        chunk.copy_to(&mut bytes[start..]);
        let percent = (bytes.len() as f64 * 100.0 / total.max(1) as f64) as usize;
        if percent != shown {
            shown = percent;
            progress(bytes.len(), total);
        }
    }
    if bytes.len() != total {
        return Err(format!("pack is {} bytes but {total} were announced", bytes.len()).into());
    }
    Ok(bytes)
}

// Finds the current pack version and brings that pack into memory from the cache or the network
async fn load_pack(window: &Window, ctx2d: &CanvasRenderingContext2d, data_url: &str) -> Result<Pack, JsValue> {
    draw_progress(ctx2d, "Checking game data", 0.0);
    let version = fetch_text(window, &format!("{data_url}version")).await?;
    let url = Url::new_with_base(&format!("{data_url}{}/{PACK}", version.trim()), &window.location().href()?)?.href();
    let cache = open_cache(window).await;
    let cached = match &cache {
        Some(c) => JsFuture::from(c.match_with_str(&url)).await?.dyn_into::<Response>().ok(),
        None => None,
    };
    let (response, verb) = match cached {
        Some(r) => (r, "Loading"),
        None => (fetch_and_keep(window, cache.as_ref(), &url).await?, "Downloading"),
    };
    let bytes = read_body(&response, |done, total| {
        let fraction = done as f64 / total as f64;
        let caption = format!("{verb} game data {:.0}% of {:.0} MB", fraction * 100.0, total as f64 / 1e6);
        draw_progress(ctx2d, &caption, fraction);
    })
    .await?;
    Pack::parse(bytes)
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

// Pointer position in game pixels clamped to the screen, from an event anywhere on the page
fn pointer_at(canvas: &HtmlCanvasElement, e: &PointerEvent) -> (i16, i16) {
    let rect = canvas.get_bounding_client_rect();
    let x = (f64::from(e.client_x()) - rect.left()) * f64::from(WIDTH) / rect.width().max(1.0);
    let y = (f64::from(e.client_y()) - rect.top()) * f64::from(HEIGHT) / rect.height().max(1.0);
    (x.floor().clamp(0.0, f64::from(WIDTH - 1)) as i16, y.floor().clamp(0.0, f64::from(HEIGHT - 1)) as i16)
}

// Game button slot for a browser button, left then right then middle
fn button_slot(e: &PointerEvent) -> Option<usize> {
    [0usize, 2, 1].get(usize::try_from(e.button()).ok()?).copied()
}

// Installs keyboard and pointer listeners that accumulate into the shared state
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
    let move_canvas = canvas.clone();
    let on_move = Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
        moved.borrow_mut().pointer = Some(pointer_at(&move_canvas, &e));
    });
    let pressed = input.clone();
    let press_canvas = canvas.clone();
    let on_down = Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
        let mut s = pressed.borrow_mut();
        s.pointer = Some(pointer_at(&press_canvas, &e));
        if let Some(slot) = button_slot(&e) {
            s.buttons[slot] = true;
        }
        e.prevent_default();
    });
    let released = input.clone();
    let on_up = Closure::<dyn FnMut(PointerEvent)>::new(move |e: PointerEvent| {
        if let Some(slot) = button_slot(&e) {
            released.borrow_mut().buttons[slot] = false;
        }
    });
    let window = web_sys::window().expect("window");
    let _ = window.add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref());
    let _ = window.add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref());
    let _ = window.add_event_listener_with_callback("pointermove", on_move.as_ref().unchecked_ref());
    let _ = canvas.add_event_listener_with_callback("pointerdown", on_down.as_ref().unchecked_ref());
    let _ = window.add_event_listener_with_callback("pointerup", on_up.as_ref().unchecked_ref());
    on_key_down.forget();
    on_key_up.forget();
    on_move.forget();
    on_down.forget();
    on_up.forget();
}

// Boots the game on a canvas after loading the pack version the data URL currently serves
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
    let pack = match load_pack(&window, &ctx2d, &data_url).await {
        Ok(pack) => pack,
        Err(e) => {
            draw_progress(&ctx2d, &format!("Could not load game data, {}", describe(&e)), 0.0);
            return Err(e);
        }
    };
    let input = Rc::new(RefCell::new(InputState::default()));
    attach_input(&canvas, input.clone());
    let host = WebHost {
        pack,
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
