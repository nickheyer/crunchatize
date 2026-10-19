#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

// Packs a game data folder into one file and confirms the written index reads back
fn run(src: &Path, dst: &Path) -> io::Result<()> {
    let mut out = BufWriter::new(File::create(dst)?);
    let count = crunch_pak::pack_to(src, &mut out)?;
    out.flush()?;
    let mut pack = File::open(dst)?;
    let index = crunch_pak::read_index(&mut pack)?;
    if index.files.len() != count {
        return Err(io::Error::other(format!("wrote {count} files but read back {}", index.files.len())));
    }
    println!("{count} files, {} bytes -> {}", pack.metadata()?.len(), dst.display());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let [_, src, dst] = args.as_slice() else {
        eprintln!("usage: crunch-pak <CrunchData dir> <out.pak>");
        return ExitCode::FAILURE;
    };
    match run(Path::new(src), Path::new(dst)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("crunch-pak: {e}");
            ExitCode::FAILURE
        }
    }
}
