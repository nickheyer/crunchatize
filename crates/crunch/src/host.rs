use std::future::Future;

use avmanager::input::DeviceState;
use avmanager::sound::Sound;

// Everything the game needs from the machine it runs on, supplied by a native or web shell
pub trait Host {
    // Reads a data file by name, matching the name without regard to letter case
    fn read_file(&mut self, name: &str) -> impl Future<Output = Option<Vec<u8>>>;

    // Milliseconds since an arbitrary start, the timeGetTime equivalent
    fn time_ms(&self) -> u32;

    // Seconds since the Unix epoch, used once to seed the random generator
    fn unix_time(&self) -> u32;

    // Samples keyboard, mouse deltas and buttons accumulated since the previous call
    fn poll_input(&mut self) -> DeviceState;

    // Shows a finished frame, renders the audio for the interval, and resolves at the next refresh
    fn end_frame(&mut self, frame: Option<&[u16]>, sound: &mut Sound) -> impl Future<Output = ()>;

    // Save file names ending in .crn in directory listing order
    fn list_saves(&mut self) -> Vec<String>;

    fn read_save(&mut self, name: &str) -> Option<Vec<u8>>;

    fn write_save(&mut self, name: &str, data: &[u8]) -> bool;

    fn save_exists(&mut self, name: &str) -> bool;

    fn delete_save(&mut self, name: &str);

    // True once per press of the F1 key that toggles the statistics overlay
    fn take_debug_toggle(&mut self) -> bool;

    // True when the window was closed, ending the message loop like WM_QUIT
    fn quit_requested(&self) -> bool;

    fn open_url(&mut self, url: &str);

    // Shows or hides host provided credits content and returns true when the host has any
    fn show_credits(&mut self, shown: bool) -> bool;

    fn log(&mut self, message: &str);
}
