use std::path::Path;

use avmanager::bam;
use avmanager::texture::Texture;

const DATA_DIR: &str = "/home/nick/code/crunchatize/dev/disc/iso/CrunchData";

// Every shipped BAM must parse and be consumed to the last byte
#[test]
fn every_disc_bam_parses_exactly() {
    let dir = Path::new(DATA_DIR);
    if !dir.is_dir() {
        return;
    }
    let mut count = 0;
    let mut v1 = 0;
    let mut mismatches = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("bam")) != Some(true) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let used = bam::consumed_length(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if used != data.len() {
            mismatches.push(format!("{}: parsed {used} of {} bytes", path.display(), data.len()));
        }
        let parsed = bam::parse(&data).unwrap();
        if parsed.version == 1 {
            v1 += 1;
            continue;
        }
        let indexed = parsed.frames.iter().any(|f| matches!(f.bitmap.pixels, bam::Pixels::Indexed(_)));
        if indexed {
            assert_eq!(parsed.palette.len(), 256, "{}", path.display());
        } else {
            assert!(parsed.palette.len() == 256 || parsed.palette.is_empty(), "{}", path.display());
        }
        for f in &parsed.frames {
            let t = Texture::from_bitmap(&f.bitmap, &parsed.palette, false);
            assert_eq!(t.pixels.len(), f.bitmap.width as usize * f.bitmap.height as usize);
        }
        count += 1;
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(count + v1, 703);
    assert_eq!(v1, 44);
}

// Palette entry 255 of transparent files converts to the green key value
#[test]
fn transparent_bams_key_on_their_palette() {
    let path = Path::new(DATA_DIR).join("Minibar.bam");
    if !path.is_file() {
        return;
    }
    let parsed = bam::parse(&std::fs::read(path).unwrap()).unwrap();
    assert!(parsed.transparent);
    assert_eq!(parsed.color_key, 0x0000_ff00);
    assert_eq!(parsed.palette[255], [0, 255, 0]);
    assert_eq!(parsed.frames.len(), 1);
    assert_eq!((parsed.frames[0].bitmap.width, parsed.frames[0].bitmap.height), (56, 9));
}
