use std::fs::File;
use std::io::{BufWriter, Write};

use smacker::Smacker;

/// Writes every frame as rgb24 and track 0 as s16le to the two output paths
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(rgb_path), Some(pcm_path)) = (args.next(), args.next(), args.next()) else {
        eprintln!("usage: dump <file.smk> <out.rgb> <out.s16>");
        std::process::exit(2);
    };
    let mut smk = Smacker::open(std::fs::read(input)?)?;
    let mut rgb = BufWriter::new(File::create(rgb_path)?);
    let mut pcm = BufWriter::new(File::create(pcm_path)?);
    while let Some(frame) = smk.next_frame()? {
        rgb.write_all(&frame.rgb24())?;
        for sample in frame.audio[0] {
            pcm.write_all(&sample.to_le_bytes())?;
        }
    }
    rgb.flush()?;
    pcm.flush()?;
    Ok(())
}
