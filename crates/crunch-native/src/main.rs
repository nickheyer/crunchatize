#![forbid(unsafe_code)]

mod audio;
mod executor;
mod files;
mod headless_host;
mod keys;
mod net;
mod window_host;

use std::path::PathBuf;
use std::process::ExitCode;

use crate::headless_host::HeadlessHost;
use crate::window_host::WindowHost;

const USAGE: &str = "usage: crunch --data <CrunchData dir> [--data <more dir>] [--saves <dir>] [--scale <n>] [--online <wss url>]\n       crunch --data <dir> --headless --ticks <n> [--script <file>] [--out <dir>] [--dump-every <n>] [--seed <n>] [--wav <file>] [--online <url>] [--pace]";
const DEFAULT_ONLINE: &str = "wss://crunchatize.app";

struct Args {
    data: Vec<PathBuf>,
    saves: PathBuf,
    online: String,
    scale: usize,
    headless: bool,
    ticks: u64,
    script: Option<PathBuf>,
    out: Option<PathBuf>,
    dump_every: u64,
    seed: u32,
    wav: Option<PathBuf>,
    pace: bool,
}

// Parses the command line, returning the usage text on any mistake
fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        data: Vec::new(),
        saves: PathBuf::new(),
        online: DEFAULT_ONLINE.to_string(),
        scale: 1,
        headless: false,
        ticks: 0,
        script: None,
        out: None,
        dump_every: 1,
        seed: 1,
        wav: None,
        pace: false,
    };
    let mut saves: Option<PathBuf> = None;
    let mut it = std::env::args().skip(1);
    let bad = |_| USAGE.to_string();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => args.data.push(it.next().ok_or(USAGE)?.into()),
            "--online" => args.online = it.next().ok_or(USAGE)?,
            "--saves" => saves = Some(it.next().ok_or(USAGE)?.into()),
            "--scale" => args.scale = it.next().ok_or(USAGE)?.parse().map_err(bad)?,
            "--headless" => args.headless = true,
            "--ticks" => args.ticks = it.next().ok_or(USAGE)?.parse().map_err(bad)?,
            "--script" => args.script = Some(it.next().ok_or(USAGE)?.into()),
            "--out" => args.out = Some(it.next().ok_or(USAGE)?.into()),
            "--dump-every" => args.dump_every = it.next().ok_or(USAGE)?.parse().map_err(bad)?,
            "--seed" => args.seed = it.next().ok_or(USAGE)?.parse().map_err(bad)?,
            "--wav" => args.wav = Some(it.next().ok_or(USAGE)?.into()),
            "--pace" => args.pace = true,
            _ => return Err(USAGE.to_string()),
        }
    }
    let first = args.data.first().ok_or(USAGE)?;
    args.saves = saves.unwrap_or_else(|| first.join("saves"));
    args.scale = args.scale.clamp(1, 4);
    if args.headless && args.ticks == 0 {
        return Err(USAGE.to_string());
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let files = match files::Files::new(&args.data, &args.saves) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open data folder {}: {e}", args.data[0].display());
            return ExitCode::from(1);
        }
    };
    if args.headless {
        let host = match HeadlessHost::new(files, args.script.as_deref(), args.out, args.dump_every, args.ticks, args.seed, args.wav, args.online, args.pace) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        let host = executor::block_on(crunch::game::run(host, HeadlessHost::audio_rate()));
        if let Err(e) = host.write_wav() {
            eprintln!("cannot write wav: {e}");
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }
    let audio = match audio::Audio::open() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("cannot open audio output: {e}");
            return ExitCode::from(1);
        }
    };
    let rate = audio.rate();
    let host = match WindowHost::new(files, audio, args.scale, args.online) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("cannot open window: {e}");
            return ExitCode::from(1);
        }
    };
    executor::block_on(crunch::game::run(host, rate));
    ExitCode::SUCCESS
}
