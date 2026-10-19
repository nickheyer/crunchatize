#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const MAGIC: &[u8; 4] = b"CRPK";
const HEADER_LEN: u64 = 8;

// Byte position of one file relative to the start of the pack body, and its length
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub offset: u64,
    pub size: u64,
}

// Parsed pack index with the absolute position where the body begins
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub body_start: u64,
    pub files: BTreeMap<String, Entry>,
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

// Lists the regular files of a folder by lower case name, rejecting names that collide or break the index
fn list_files(dir: &Path) -> io::Result<BTreeMap<String, (PathBuf, u64)>> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.contains(['\t', '\n']) {
            return Err(invalid(format!("{name:?} contains a tab or newline")));
        }
        let size = entry.metadata()?.len();
        if files.insert(name.clone(), (entry.path(), size)).is_some() {
            return Err(invalid(format!("{name} appears twice ignoring case")));
        }
    }
    Ok(files)
}

// Writes the magic, index and every file body in name order, returning the file count
pub fn pack_to(dir: &Path, out: &mut impl Write) -> io::Result<usize> {
    let files = list_files(dir)?;
    let mut index = String::new();
    let mut offset = 0u64;
    for (name, (_, size)) in &files {
        index.push_str(&format!("{name}\t{offset}\t{size}\n"));
        offset += size;
    }
    let index_len = u32::try_from(index.len()).map_err(|_| invalid("index exceeds 4 GiB".into()))?;
    out.write_all(MAGIC)?;
    out.write_all(&index_len.to_le_bytes())?;
    out.write_all(index.as_bytes())?;
    for (name, (path, size)) in &files {
        let copied = io::copy(&mut fs::File::open(path)?, out)?;
        if copied != *size {
            return Err(invalid(format!("{name} changed size while packing")));
        }
    }
    Ok(files.len())
}

// Reads the header and index from the start of a pack
pub fn read_index(pack: &mut impl Read) -> io::Result<Index> {
    let mut header = [0u8; HEADER_LEN as usize];
    pack.read_exact(&mut header)?;
    if &header[..4] != MAGIC {
        return Err(invalid("not a CRPK pack".into()));
    }
    let index_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let mut text = vec![0u8; index_len];
    pack.read_exact(&mut text)?;
    let text = String::from_utf8(text).map_err(|e| invalid(e.to_string()))?;
    let mut files = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(name), Some(offset), Some(size), None) = (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(invalid(format!("malformed index line {line:?}")));
        };
        let offset = offset.parse().map_err(|_| invalid(format!("bad offset in {line:?}")))?;
        let size = size.parse().map_err(|_| invalid(format!("bad size in {line:?}")))?;
        files.insert(name.to_string(), Entry { offset, size });
    }
    Ok(Index { body_start: HEADER_LEN + index_len as u64, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("crunch-pak-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn pack_round_trips_lower_cased_names_in_order() {
        let dir = scratch_dir("roundtrip");
        fs::write(dir.join("Mouse.BAM"), b"bam bytes").unwrap();
        fs::write(dir.join("001.wav"), b"wav").unwrap();
        fs::create_dir(dir.join("sub")).unwrap();
        let mut pack = Vec::new();
        assert_eq!(pack_to(&dir, &mut pack).unwrap(), 2);
        let index = read_index(&mut pack.as_slice()).unwrap();
        let names: Vec<&String> = index.files.keys().collect();
        assert_eq!(names, ["001.wav", "mouse.bam"]);
        let wav = index.files["001.wav"];
        let bam = index.files["mouse.bam"];
        assert_eq!((wav.offset, wav.size), (0, 3));
        assert_eq!((bam.offset, bam.size), (3, 9));
        let body = &pack[index.body_start as usize..];
        assert_eq!(&body[wav.offset as usize..][..wav.size as usize], b"wav");
        assert_eq!(&body[bam.offset as usize..][..bam.size as usize], b"bam bytes");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn names_that_collide_ignoring_case_are_rejected() {
        let dir = scratch_dir("collide");
        fs::write(dir.join("a.wav"), b"1").unwrap();
        fs::write(dir.join("A.WAV"), b"2").unwrap();
        let err = pack_to(&dir, &mut Vec::new()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn wrong_magic_is_rejected() {
        let err = read_index(&mut &b"RIFF\0\0\0\0"[..]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
