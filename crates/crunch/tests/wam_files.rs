use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use avmanager::input::DeviceState;
use avmanager::sound::Sound;
use crunch::av::Av;
use crunch::host::Host;
use crunch::scene::{KIND_TEXT, Scene};

const DATA_DIR: &str = "/home/nick/code/crunchatize/dev/disc/iso/CrunchData";

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(fut: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

// Reads game files straight from the extracted disc folder
struct DirHost {
    files: HashMap<String, std::path::PathBuf>,
    frames: usize,
}

impl DirHost {
    fn new(dir: &Path) -> Self {
        let mut files = HashMap::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            files.insert(e.file_name().to_string_lossy().to_lowercase(), e.path());
        }
        Self { files, frames: 0 }
    }
}

impl Host for DirHost {
    async fn read_file(&mut self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.files.get(&name.to_lowercase())?).ok()
    }

    fn time_ms(&self) -> u32 {
        0
    }

    fn unix_time(&self) -> u32 {
        0
    }

    fn poll_input(&mut self) -> DeviceState {
        DeviceState::new()
    }

    async fn end_frame(&mut self, _frame: Option<&[u16]>, _sound: &mut Sound) {
        self.frames += 1;
    }

    fn list_saves(&mut self) -> Vec<String> {
        Vec::new()
    }

    fn read_save(&mut self, _name: &str) -> Option<Vec<u8>> {
        None
    }

    fn write_save(&mut self, _name: &str, _data: &[u8]) -> bool {
        false
    }

    fn save_exists(&mut self, _name: &str) -> bool {
        false
    }

    fn delete_save(&mut self, _name: &str) {}

    fn take_debug_toggle(&mut self) -> bool {
        false
    }

    fn quit_requested(&self) -> bool {
        false
    }

    fn open_url(&mut self, _url: &str) {}

    fn show_credits(&mut self, _shown: bool) -> bool {
        false
    }

    fn launch_setup(&mut self) {}

    fn log(&mut self, message: &str) {
        eprintln!("{message}");
    }
}

const GAME_SCENES: [&str; 13] = [
    "title",
    "LoadCrunch",
    "credits",
    "select",
    "HomeWorld",
    "Running",
    "throw",
    "throw_challenge",
    "jumpcomlev01",
    "jumpcomlev02",
    "jumpcomlev03",
    "jumpcomlev04",
    "challenge",
];

// Every scene file the game opens must load with all its animations and fonts resolved
#[test]
fn every_game_wam_loads() {
    let dir = Path::new(DATA_DIR);
    if !dir.is_dir() {
        return;
    }
    let mut host = DirHost::new(dir);
    for name in GAME_SCENES {
        let mut av = Av::new(44100);
        let mut scene = Scene::new();
        let ok = block_on(scene.load(&mut av, &mut host, name));
        assert!(ok, "{name}.wam failed to load");
        assert!(scene.object_count() > 0, "{name}.wam has no objects");
        for o in &scene.objects {
            if o.state == 0 {
                continue;
            }
            if o.kind == KIND_TEXT {
                assert_ne!(o.font1, 0xffff, "{name}.wam text object without a font");
            }
            assert!(o.bbox_x2 >= o.bbox_x1 && o.bbox_y2 >= o.bbox_y1, "{name}.wam has an inverted box");
        }
    }
}

// The home world scene must expose the interactive objects by their names
#[test]
fn home_world_scene_names_resolve() {
    let dir = Path::new(DATA_DIR);
    if !dir.is_dir() {
        return;
    }
    let mut host = DirHost::new(dir);
    let mut av = Av::new(44100);
    let mut scene = Scene::new();
    assert!(block_on(scene.load(&mut av, &mut host, "HomeWorld")));
    for (i, n) in ["k1", "k2", "k3", "name", "speed", "strength", "jump", "challenge", "tips", "quit", "online"].iter().enumerate() {
        scene.assign_id(n, i as i32 + 1);
    }
    assert_eq!(scene.objects.iter().filter(|o| o.state != 0 && o.id != 0).count(), 11);
    assert!(scene.text_by_id(4).is_some());
}
