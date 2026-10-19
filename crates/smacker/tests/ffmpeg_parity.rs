use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use smacker::{AudioInfo, Flags, Smacker, Version};

const DISC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../dev/disc/iso/CrunchData");

fn ffmpeg(path: &Path, args: &[&str]) -> Child {
    Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args(args)
        .arg("-")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("ffmpeg must be installed to verify decoder output")
}

fn first_difference(ours: &[u8], theirs: &[u8]) -> Option<usize> {
    ours.iter().zip(theirs).position(|(a, b)| a != b)
}

/// Every frame as rgb24 and the whole track 0 stream must equal what ffmpeg decodes
fn compare(name: &str, frames: u32, frame_rate_micros: u32) {
    let path = Path::new(DISC).join(name);
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("skipping {name} because {} is absent", path.display());
        return;
    };
    let mut smk = Smacker::open(data).unwrap();
    let header = smk.header().clone();
    assert_eq!(header.version, Version::Smk2);
    assert_eq!((header.width, header.height), (640, 280));
    assert_eq!(header.frames, frames);
    assert_eq!(header.frame_rate_micros, frame_rate_micros);
    assert_eq!(header.flags, Flags::default());
    assert_eq!(header.audio[0], AudioInfo { present: true, compressed: true, bits16: true, stereo: true, rate: 22050 });
    assert!(header.audio[1..].iter().all(|track| *track == AudioInfo::default()));
    assert_eq!(smk.frame_count(), header.frames as usize + usize::from(header.flags.ring_frame));

    let frame_bytes = header.width as usize * header.height as usize * 3;
    let mut video = ffmpeg(&path, &["-f", "rawvideo", "-pix_fmt", "rgb24"]);
    let mut video_out = video.stdout.take().unwrap();
    let mut reference = vec![0u8; frame_bytes];
    let mut samples = Vec::new();
    let mut first_frame = Vec::new();
    let mut index = 0;
    while let Some(frame) = smk.next_frame().unwrap() {
        let ours = frame.rgb24();
        video_out.read_exact(&mut reference).unwrap_or_else(|_| panic!("ffmpeg produced fewer than {} frames", index + 1));
        if let Some(byte) = first_difference(&ours, &reference) {
            let pixel = byte / 3;
            panic!(
                "frame {index} differs at pixel {pixel} ({}, {}) ours {:?} ffmpeg {:?}",
                pixel % header.width as usize,
                pixel / header.width as usize,
                &ours[pixel * 3..pixel * 3 + 3],
                &reference[pixel * 3..pixel * 3 + 3]
            );
        }
        if index == 0 {
            assert!(frame.keyframe);
            assert!(frame.palette_changed);
            first_frame = ours;
        }
        samples.extend_from_slice(frame.audio[0]);
        index += 1;
    }
    assert_eq!(index, smk.frame_index());
    assert_eq!(video_out.read(&mut reference[..1]).unwrap(), 0, "ffmpeg produced more frames than the decoder");
    assert!(video.wait().unwrap().success());
    assert_eq!(smk.frame_index(), smk.frame_count());

    let mut audio = ffmpeg(&path, &["-vn", "-f", "s16le", "-acodec", "pcm_s16le"]);
    let mut pcm = Vec::new();
    audio.stdout.take().unwrap().read_to_end(&mut pcm).unwrap();
    assert!(audio.wait().unwrap().success());
    let reference: Vec<i16> = pcm.chunks_exact(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect();
    if let Some(sample) = samples.iter().zip(&reference).position(|(a, b)| a != b) {
        panic!("audio differs at sample {sample} ours {} ffmpeg {}", samples[sample], reference[sample]);
    }
    assert_eq!(samples.len(), reference.len(), "audio sample counts differ");

    smk.rewind();
    assert_eq!(smk.frame_index(), 0);
    let again = smk.next_frame().unwrap().unwrap().rgb24();
    assert!(again == first_frame, "frame 0 differs after rewind");
}

#[test]
fn intro_matches_ffmpeg() {
    compare("INTRO.smk", 5084, 41660);
}

#[test]
fn outro_matches_ffmpeg() {
    compare("OUTRO.smk", 765, 41660);
}
