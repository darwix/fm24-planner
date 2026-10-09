//! The `.fm` container: a header, the members' data, then a zstd-compressed index.
//!
//! ```text
//! file    [header 26: magic x6, version at 6, index offset u64 at 9, compression at 25]
//!         [member: 26-byte header, then its data] ...
//!         [index: magic x9][zstd frame]
//! index   [archive name][root group][group count u32][group name, group] ...
//! group   [member count u32][member entry] ...
//! entry   [name parts, the last one starts with '.'][offset u64][stored u64][plain u64][16 bytes]
//! text    [len u32][bytes]
//! ```
use crate::utils::cancel::{check, CancelFlag};
use crate::{Error, Result};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    time::SystemTime,
};

/// The archive header; each member's data follows a header of the same size.
const HEADER: u64 = 26;
const MAGIC: &[u8] = b"\x02\x01fmf.";
const VERSION: usize = 6;
const INDEX_OFFSET: std::ops::Range<usize> = 9..17;
const COMPRESSION: usize = 25;
/// The index starts with the archive magic and version, then a zstd frame.
const INDEX_MAGIC: &[u8] = b"\x02\x01fmf.\x08\0\0";
const ZSTD_MAGIC: &[u8] = b"\x28\xb5\x2f\xfd";

const MIB: usize = 1024 * 1024;
/// Reads and decompression go in chunks of this size, checking for cancellation in between.
const CHUNK: usize = MIB;
const MAX_INDEX_STORED: u64 = 32 * MIB as u64;
const MAX_INDEX: usize = 64 * MIB;
const MAX_MEMBER: u64 = 2 * 1024 * MIB as u64;
const MAX_TEXT: usize = 16384;
const MAX_GROUPS: u32 = 10000;
const MAX_MEMBERS: u32 = 100000;
const MAX_NAME_PARTS: usize = 20;
/// Unknown bytes at the end of each member entry.
const ENTRY_PADDING: usize = 16;

pub struct Archive {
    file: File,
    size: u64,
    modified: SystemTime,
    compressed: bool,
    pub name: String,
    members: Vec<Member>,
}

struct Member {
    name: String,
    offset: u64,
    stored: u64,
    plain: u64,
}

impl Archive {
    pub fn open(path: &Path, cancel: &CancelFlag) -> Result<Self> {
        let file = File::open(path)?;
        let stat = file.metadata()?;
        let mut a = Self {
            file,
            size: stat.len(),
            modified: stat.modified()?,
            compressed: false,
            name: String::new(),
            members: vec![],
        };
        let (compressed, index_start) = a.read_header(cancel)?;
        a.compressed = compressed;
        let index = a.read_index(index_start, cancel)?;
        (a.name, a.members) = read_members(&index, index_start, cancel)?;
        if !a.members.iter().any(|m| m.name == "game_db.dat") {
            return Err(Error::new(
                "MISSING_DATABASE",
                "The save does not contain a player database.",
            ));
        }
        Ok(a)
    }

    /// The data of member `name`, decompressed.
    pub fn member(&mut self, name: &str, cancel: &CancelFlag) -> Result<Vec<u8>> {
        let m = self
            .members
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| Error::new("MISSING_MEMBER", format!("The save is missing {name}.")))?;
        let (offset, stored, plain) = (m.offset, m.stored, m.plain);
        let bytes = self.read(offset + HEADER, stored, cancel)?;
        let bytes = if self.compressed {
            decompress(&bytes, plain as usize, cancel)?
        } else {
            bytes
        };
        if bytes.len() as u64 != plain {
            return Err(Error::new(
                "INVALID_SIZE",
                format!("{name} has an unexpected size."),
            ));
        }
        Ok(bytes)
    }

    /// Whether the file still has the size and modification time it had when opened.
    pub fn unchanged(&self) -> Result<bool> {
        let stat = self.file.metadata()?;
        Ok(stat.len() == self.size && stat.modified()? == self.modified)
    }

    /// Checks the archive header. Returns whether members are compressed and where the index starts.
    fn read_header(&mut self, cancel: &CancelFlag) -> Result<(bool, u64)> {
        let h = self.read(0, HEADER, cancel)?;
        if &h[..MAGIC.len()] != MAGIC || h[VERSION] != 8 {
            return Err(Error::new(
                "UNSUPPORTED_FORMAT",
                "This file is not a supported FM24 save archive.",
            ));
        }
        if h[COMPRESSION] != 0 && h[COMPRESSION] != 3 {
            return Err(Error::new(
                "UNSUPPORTED_COMPRESSION",
                "This save uses an unsupported compression method.",
            ));
        }
        // The stored index offset counts from the offset field itself.
        let index_start = u64::from_le_bytes(h[INDEX_OFFSET].try_into().unwrap())
            .checked_add(INDEX_OFFSET.start as u64)
            .ok_or_else(|| Error::new("INVALID_SIZE", "Invalid archive index offset."))?;
        Ok((h[COMPRESSION] == 3, index_start))
    }

    /// The decompressed index: everything from `start` to the end of the file.
    fn read_index(&mut self, start: u64, cancel: &CancelFlag) -> Result<Vec<u8>> {
        let magic = INDEX_MAGIC.len() as u64;
        let header = self.read(start, magic + ZSTD_MAGIC.len() as u64, cancel)?;
        if &header[..INDEX_MAGIC.len()] != INDEX_MAGIC || &header[INDEX_MAGIC.len()..] != ZSTD_MAGIC
        {
            return Err(manifest(
                "The save index is missing or incomplete. Try a completed backup save.",
            ));
        }
        let length = self.size - start - magic;
        if length > MAX_INDEX_STORED {
            return Err(manifest("The save index exceeds the supported size."));
        }
        let compressed = self.read(start + magic, length, cancel)?;
        decompress(&compressed, MAX_INDEX, cancel)
    }

    fn read(&mut self, position: u64, length: u64, cancel: &CancelFlag) -> Result<Vec<u8>> {
        if position.checked_add(length).is_none_or(|v| v > self.size) {
            return Err(Error::new("TRUNCATED", "The save file is incomplete."));
        }
        let length = usize::try_from(length)
            .map_err(|_| Error::new("INVALID_SIZE", "Archive member is too large."))?;
        let mut bytes = vec![0; length];
        self.file.seek(SeekFrom::Start(position))?;
        for chunk in bytes.chunks_mut(CHUNK) {
            check(cancel)?;
            self.file.read_exact(chunk)?;
        }
        Ok(bytes)
    }
}

/// The archive name and member list: the root group, then named groups.
/// Every member must end before the index at `index_start`.
fn read_members(
    index: &[u8],
    index_start: u64,
    cancel: &CancelFlag,
) -> Result<(String, Vec<Member>)> {
    let mut c = Cursor::new(index);
    let name = c.text()?;
    let mut members = vec![];
    read_group(&mut c, "", index_start, &mut members, cancel)?;
    let groups = c.u32()?;
    if groups > MAX_GROUPS {
        return Err(manifest("Invalid archive groups."));
    }
    for _ in 0..groups {
        let prefix = c.text()? + "/";
        read_group(&mut c, &prefix, index_start, &mut members, cancel)?;
    }
    Ok((name, members))
}

fn read_group(
    c: &mut Cursor<'_>,
    prefix: &str,
    index_start: u64,
    members: &mut Vec<Member>,
    cancel: &CancelFlag,
) -> Result<()> {
    let count = c.u32()?;
    if count > MAX_MEMBERS {
        return Err(manifest("Invalid number of archive members."));
    }
    for _ in 0..count {
        check(cancel)?;
        let name = member_name(c, prefix)?;
        let (offset, stored, plain) = (c.u64()?, c.u64()?, c.u64()?);
        c.field(ENTRY_PADDING)?;
        if offset
            .checked_add(stored)
            .and_then(|v| v.checked_add(HEADER))
            .is_none_or(|v| v > index_start)
            || plain > MAX_MEMBER
        {
            return Err(Error::new(
                "INVALID_SIZE",
                "A save member has invalid bounds or exceeds 2 GB.",
            ));
        }
        members.push(Member {
            name,
            offset,
            stored,
            plain,
        });
    }
    Ok(())
}

/// A member name is stored in parts; the last part is the extension, starting with '.'.
fn member_name(c: &mut Cursor<'_>, prefix: &str) -> Result<String> {
    let mut name = prefix.to_string();
    for _ in 0..MAX_NAME_PARTS {
        let part = c.text()?;
        name.push_str(&part);
        if part.starts_with('.') {
            return Ok(name);
        }
    }
    Err(manifest("Invalid archive member name."))
}

fn manifest(message: &str) -> Error {
    Error::new("INVALID_MANIFEST", message)
}

fn corrupt() -> Error {
    Error::new("CORRUPT_COMPRESSION", "The save could not be decompressed.")
}

/// Decompresses a zstd frame, failing once the output would pass `limit` bytes.
fn decompress(bytes: &[u8], limit: usize, cancel: &CancelFlag) -> Result<Vec<u8>> {
    let mut decoder = zstd::stream::read::Decoder::new(bytes).map_err(|_| corrupt())?;
    let mut output = Vec::new();
    let mut chunk = vec![0; CHUNK];
    loop {
        check(cancel)?;
        let n = decoder.read(&mut chunk).map_err(|_| corrupt())?;
        if n == 0 {
            break;
        }
        if n > limit.saturating_sub(output.len()) {
            return Err(Error::new(
                "INVALID_SIZE",
                "Decompressed data exceeds the supported size.",
            ));
        }
        output.extend_from_slice(&chunk[..n]);
    }
    Ok(output)
}

/// The index, read front to back.
type Cursor<'a> = std::io::Cursor<&'a [u8]>;

/// Little-endian fields of the index, failing with `TRUNCATED` past its end.
trait IndexRead {
    fn field(&mut self, n: usize) -> Result<Vec<u8>>;
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.field(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.field(8)?.try_into().unwrap()))
    }
    fn text(&mut self) -> Result<String> {
        let n = self.u32()? as usize;
        if n > MAX_TEXT {
            return Err(manifest("An archive name has an invalid length."));
        }
        Ok(String::from_utf8_lossy(&self.field(n)?).into_owned())
    }
}
impl IndexRead for Cursor<'_> {
    fn field(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0; n];
        self.read_exact(&mut bytes)
            .map_err(|_| Error::new("TRUNCATED", "The save contains an incomplete record."))?;
        Ok(bytes)
    }
}
