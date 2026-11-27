//! Read-only package views for PlayStation button prompts. The only artwork
//! built into the port is our own. The rest comes from the user's package.

mod glyphs;
mod movie;

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use zlib_rs::{DeflateConfig, Inflate, InflateFlush, Status};

const DATA: &str = "data.rpf";
const CONTROLLER: &str = "data/ttdb/ui/platform.ttpf";
const DIAGRAM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dualsense.bc7"));
const MAX_MOVIE: usize = 1 << 20;

struct Patch {
    offset: u64,
    bytes: Vec<u8>,
}

fn unpack(source: &[u8], size: usize) -> Option<(Vec<u8>, usize)> {
    let mut raw = vec![0; size + 1];
    let mut state = Inflate::new(true, 15);
    if state
        .decompress(source, &mut raw, InflateFlush::NoFlush)
        .ok()?
        != Status::StreamEnd
        || state.total_out() != size as u64
    {
        return None;
    }
    raw.truncate(size);
    Some((raw, state.total_in() as usize))
}

/// Keep the compressed span exact as well as the decoded length. Prefix
/// empty non-final deflate blocks at byte boundaries after the zlib header.
fn pack(raw: &[u8], capacity: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0; capacity];
    let (stream, rc) = zlib_rs::compress_slice(
        &mut buffer,
        raw,
        DeflateConfig {
            level: 9,
            ..Default::default()
        },
    );
    if rc != zlib_rs::ReturnCode::Ok {
        return None;
    }
    let extra = capacity.checked_sub(stream.len())?;
    let sixes = extra % 5;
    let fives = extra.checked_sub(sixes * 6)? / 5;
    let mut result = Vec::with_capacity(capacity);
    result.extend(stream.get(..2)?);
    for _ in 0..fives {
        result.extend([0, 0, 0, 255, 255]);
    }
    // Empty fixed-code block followed by an empty stored block.
    for _ in 0..sixes {
        result.extend([2, 0, 0, 0, 255, 255]);
    }
    result.extend(&stream[2..]);
    Some(result)
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..)?.get(..4)?.try_into().ok()?,
    ))
}

fn controller_patch(file: &[u8]) -> Option<Patch> {
    // One texture in the RDS5 package. Entries give 256-byte disk sectors.
    if file.get(..4)? != b"RDS5" || word(file, 4)? != 3 {
        return None;
    }
    let sector =
        |index: usize| word(file, 16 + 8 * index).map(|v| ((v & 0x03ff_ffff) as usize) * 256);
    let header = sector(0)?;
    let start = sector(1)?;
    let end = sector(2)?;
    let table = file.get(header..start)?;
    if table.get(..4)? != b"ETPF" || word(table, 8)? != 1 {
        return None;
    }
    let entry = table.get(word(table, 12)? as usize..)?;
    let size = word(entry, 0)?;
    if size & 0x3fff != 1023 || (size >> 14) & 0x3fff != 511 || word(entry, 4)? != 0x20_0311 {
        return None;
    }
    let (_, capacity) = unpack(file.get(start..end)?, DIAGRAM.len())?;
    Some(Patch {
        offset: start as u64,
        bytes: pack(DIAGRAM, capacity)?,
    })
}

/// RPF stream starts are 16-byte aligned. Inflate just the prefix to
/// identify a movie, then check its export and sprite tables before use.
fn button_patch(file: &mut File) -> Result<Option<Patch>> {
    file.rewind()?;
    let length = file.metadata()?.len();
    let mut magic = [0; 4];
    if length < 4 {
        return Ok(None);
    }
    file.read_exact(&mut magic)?;
    if magic != *b"9FPR" {
        return Ok(None);
    }
    const WINDOW: usize = 1024 * 1024;
    const AHEAD: usize = 2048;
    let mut buf = vec![0; WINDOW + AHEAD];
    let mut state = Inflate::new(true, 15);
    let mut prefix = [0; 128];
    let mut base = 0;
    while base < length {
        file.seek(SeekFrom::Start(base))?;
        let size = (length - base).min(buf.len() as u64) as usize;
        file.read_exact(&mut buf[..size])?;
        for pos in (0..size.min(WINDOW)).step_by(16) {
            let source = &buf[pos..size];
            if !matches!(source, [0x78, 0x01 | 0x5e | 0x9c | 0xda, ..]) {
                continue;
            }
            state.reset(true);
            if state
                .decompress(source, &mut prefix, InflateFlush::NoFlush)
                .is_err()
                || state.total_out() != prefix.len() as u64
                || !movie::is_input_movie(&prefix)
            {
                continue;
            }
            let output = word(&prefix, 4).unwrap() as usize;
            if !(128..=MAX_MOVIE).contains(&output) {
                continue;
            }
            let offset = base + pos as u64;
            file.seek(SeekFrom::Start(offset))?;
            let mut packed = vec![0; (length - offset).min(MAX_MOVIE as u64) as usize];
            file.read_exact(&mut packed)?;
            if let Some((raw, capacity)) = unpack(&packed, output)
                && let Some(replacement) = movie::replace(&raw)
                && let Some(bytes) = pack(&replacement, capacity)
            {
                return Ok(Some(Patch { offset, bytes }));
            }
        }
        base += WINDOW as u64;
    }
    Ok(None)
}

fn read_only(path: &Path) -> io::Result<File> {
    let mut options = File::options();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Hold the package stable until both views have been published.
        options.share_mode(1);
    }
    options.open(path)
}

/// Both paths are installed together. Only the cache is ever opened for
/// writing. A content-addressed directory can serve concurrent launches
/// and automatically changes when either the input package or the art does.
pub fn prepare(package: &Path, cache: &Path) -> Result<Option<[(PathBuf, PathBuf); 2]>> {
    let package = if package.as_os_str().is_empty() {
        Path::new(".")
    } else {
        package
    };
    let diagram_path = package.join(CONTROLLER);
    let mut diagram = match read_only(&diagram_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        file => file?,
    };
    if diagram.metadata()?.len() > 2 << 20 {
        return Ok(None);
    }
    let mut original_diagram = Vec::new();
    diagram.read_to_end(&mut original_diagram)?;
    let Some(drawing) = controller_patch(&original_diagram) else {
        return Ok(None);
    };
    let data_path = package.join(DATA);
    let mut archive = read_only(&data_path)?;

    let cache = cache.canonicalize()?;
    ensure!(
        !cache.starts_with(package.canonicalize()?),
        "UI cache is inside the game directory"
    );
    victory_core::ensure_outside_repo(&cache)?;
    let mut key = blake3::Hasher::new();
    key.update(b"victory-controller-ui-v1");
    key.update(DIAGRAM);
    for button in glyphs::Button::ALL {
        key.update(&glyphs::artwork(0, button));
    }
    key.update(&original_diagram);
    key.update_reader(&mut archive)?;
    let dest = cache.join(format!("dualsense-{}", key.finalize().to_hex()));
    let expected = [
        (data_path, dest.join("buttons.rpf")),
        (diagram_path, dest.join("controller.ttpf")),
    ];
    let lengths = [archive.metadata()?.len(), original_diagram.len() as u64];
    let complete = || {
        expected
            .iter()
            .zip(lengths)
            .all(|((_, p), n)| p.metadata().is_ok_and(|m| m.is_file() && m.len() == n))
    };
    if complete() {
        return Ok(Some(expected));
    }
    let Some(buttons) = button_patch(&mut archive)? else {
        return Ok(None);
    };
    let stage = tempfile::Builder::new()
        .prefix(".dualsense-")
        .tempdir_in(&cache)?;
    victory_core::ensure_outside_repo(stage.path())?;
    let mut data = File::create_new(stage.path().join("buttons.rpf"))?;
    archive.rewind()?;
    io::copy(&mut archive, &mut data)?;
    data.seek(SeekFrom::Start(buttons.offset))?;
    data.write_all(&buttons.bytes)?;
    data.sync_all()?;
    drop(data);
    let at = drawing.offset as usize;
    original_diagram[at..at + drawing.bytes.len()].copy_from_slice(&drawing.bytes);
    let mut image = File::create_new(stage.path().join("controller.ttpf"))?;
    image.write_all(&original_diagram)?;
    image.sync_all()?;
    drop(image);
    // Only a complete pair becomes visible. Another process may win the race.
    if let Err(error) = fs::rename(stage.path(), &dest)
        && !complete()
    {
        return Err(error).context("publishing the controller UI cache");
    }
    Ok(Some(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_length_streams_consume_every_byte() {
        let raw = b"0123456789".repeat(150);
        for capacity in 200..210 {
            let packed = pack(&raw, capacity).unwrap();
            assert_eq!(packed.len(), capacity);
            assert_eq!(unpack(&packed, raw.len()), Some((raw.clone(), capacity)));
        }
        assert!(pack(&raw, 4).is_none());
    }

    #[test]
    fn unknown_assets_are_left_alone() {
        assert!(controller_patch(b"RDS5").is_none());
        assert!(movie::replace(&[0; 256]).is_none());
        let temp = tempfile::tempdir().unwrap();
        assert!(prepare(temp.path(), temp.path()).unwrap().is_none());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    /// Check the whole view, including blocks outside the two substitutions.
    /// Game data and the temporary cache stay outside the repository.
    #[test]
    fn supplied_package_is_read_only_and_the_views_are_reusable() {
        let Some(path) = std::env::var_os("VICTORY_TEST_UI_DIR") else {
            return;
        };
        let package = Path::new(&path);
        let cache = tempfile::tempdir().unwrap();
        let digest = |path: &Path| {
            let mut h = blake3::Hasher::new();
            h.update_reader(File::open(path).unwrap()).unwrap();
            h.finalize()
        };
        let originals = [package.join(DATA), package.join(CONTROLLER)];
        let before = originals.each_ref().map(|p| digest(p));
        let views = prepare(package, cache.path())
            .unwrap()
            .expect("supported UI");
        assert_eq!(prepare(package, cache.path()).unwrap(), Some(views.clone()));
        let patches = [
            button_patch(&mut File::open(&originals[0]).unwrap())
                .unwrap()
                .unwrap(),
            controller_patch(&fs::read(&originals[1]).unwrap()).unwrap(),
        ];
        for ((source, view), patch) in views.iter().zip(patches) {
            let mut f = File::open(source).unwrap();
            let mut expected = blake3::Hasher::new();
            expected.update_reader((&mut f).take(patch.offset)).unwrap();
            expected.update(&patch.bytes);
            f.seek_relative(patch.bytes.len() as i64).unwrap();
            expected.update_reader(f).unwrap();
            assert_eq!(digest(view), expected.finalize());
        }
        assert_eq!(before, originals.each_ref().map(|p| digest(p)));
    }
}
