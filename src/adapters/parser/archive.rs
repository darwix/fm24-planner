use crate::utils::cancel::CancelFlag;
use crate::{Error, Result};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    time::SystemTime,
};

pub use crate::utils::cancel::check;

/// Archive header: `[magic x6][version] .. [index offset u64 at 9] .. [compression at 25]`.
/// Each member's data follows a header of the same size.
const HEADER: u64 = 26;
const MAGIC: &[u8] = b"\x02\x01fmf.";
const VERSION: usize = 6;
const INDEX_OFFSET: std::ops::Range<usize> = 9..17;
const COMPRESSION: usize = 25;
/// The index starts with the archive magic and version, then a zstd frame.
const INDEX_MAGIC: &[u8] = b"\x02\x01fmf.\x08\0\0";
const ZSTD_MAGIC: &[u8] = b"\x28\xb5\x2f\xfd";

fn manifest(message: &str) -> Error {
    Error::new("INVALID_MANIFEST", message)
}

pub fn decompress(bytes: &[u8], limit: usize, cancel: &CancelFlag) -> Result<Vec<u8>> {
    let mut decoder = zstd::stream::read::Decoder::new(bytes)
        .map_err(|_| Error::new("CORRUPT_COMPRESSION", "The save could not be decompressed."))?;
    let mut output = Vec::new();
    let mut chunk = vec![0; 1024 * 1024];
    loop {
        check(cancel)?;
        let n = decoder.read(&mut chunk).map_err(|_| {
            Error::new("CORRUPT_COMPRESSION", "The save could not be decompressed.")
        })?;
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

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.bytes.len())
            .ok_or_else(|| Error::new("TRUNCATED", "The save contains an incomplete record."))?;
        let b = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn text(&mut self) -> Result<String> {
        let n = self.u32()? as usize;
        if n > 16384 {
            return Err(manifest("An archive name has an invalid length."));
        }
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
}

pub struct Member {
    pub name: String,
    pub offset: u64,
    pub stored: u64,
    pub plain: u64,
}
pub struct Archive {
    file: File,
    size: u64,
    modified: SystemTime,
    compressed: bool,
    pub name: String,
    pub members: Vec<Member>,
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
        let index_start = a.read_header(cancel)?;
        let index = a.read_index(index_start, cancel)?;
        a.read_members(&index, index_start, cancel)?;
        if !a.members.iter().any(|m| m.name == "game_db.dat") {
            return Err(Error::new(
                "MISSING_DATABASE",
                "The save does not contain a player database.",
            ));
        }
        Ok(a)
    }
    /// Checks the archive header, notes the compression and returns where the index starts.
    fn read_header(&mut self, cancel: &CancelFlag) -> Result<u64> {
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
        self.compressed = h[COMPRESSION] == 3;
        // The stored index offset counts from the offset field itself.
        u64::from_le_bytes(h[INDEX_OFFSET].try_into().unwrap())
            .checked_add(INDEX_OFFSET.start as u64)
            .ok_or_else(|| Error::new("INVALID_SIZE", "Invalid archive index offset."))
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
        if length > 32 * 1024 * 1024 {
            return Err(manifest("The save index exceeds the supported size."));
        }
        let compressed = self.read(start + magic, length, cancel)?;
        decompress(&compressed, 64 * 1024 * 1024, cancel)
    }
    /// Reads the archive name and member list: the root group, then named groups.
    fn read_members(&mut self, index: &[u8], index_start: u64, cancel: &CancelFlag) -> Result<()> {
        let mut c = Cursor {
            bytes: index,
            pos: 0,
        };
        self.name = c.text()?;
        self.group(&mut c, "", index_start, cancel)?;
        let groups = c.u32()?;
        if groups > 10000 {
            return Err(manifest("Invalid archive groups."));
        }
        for _ in 0..groups {
            let prefix = c.text()? + "/";
            self.group(&mut c, &prefix, index_start, cancel)?;
        }
        Ok(())
    }
    fn group(
        &mut self,
        c: &mut Cursor<'_>,
        prefix: &str,
        start: u64,
        cancel: &CancelFlag,
    ) -> Result<()> {
        let count = c.u32()?;
        if count > 100000 {
            return Err(manifest("Invalid number of archive members."));
        }
        for _ in 0..count {
            check(cancel)?;
            let mut name = prefix.to_string();
            let mut complete = false;
            for _ in 0..20 {
                let part = c.text()?;
                name.push_str(&part);
                if part.starts_with('.') {
                    complete = true;
                    break;
                }
            }
            if !complete {
                return Err(manifest("Invalid archive member name."));
            }
            let offset = c.u64()?;
            let stored = c.u64()?;
            let plain = c.u64()?;
            c.take(16)?;
            if offset
                .checked_add(stored)
                .and_then(|v| v.checked_add(HEADER))
                .is_none_or(|v| v > start)
                || plain > 2 * 1024u64.pow(3)
            {
                return Err(Error::new(
                    "INVALID_SIZE",
                    "A save member has invalid bounds or exceeds 2 GB.",
                ));
            }
            self.members.push(Member {
                name,
                offset,
                stored,
                plain,
            });
        }
        Ok(())
    }
    fn read(&mut self, position: u64, length: u64, cancel: &CancelFlag) -> Result<Vec<u8>> {
        if position.checked_add(length).is_none_or(|v| v > self.size) {
            return Err(Error::new("TRUNCATED", "The save file is incomplete."));
        }
        let length = usize::try_from(length)
            .map_err(|_| Error::new("INVALID_SIZE", "Archive member is too large."))?;
        let mut b = vec![0; length];
        self.file.seek(SeekFrom::Start(position))?;
        for chunk in b.chunks_mut(1024 * 1024) {
            check(cancel)?;
            self.file.read_exact(chunk)?;
        }
        Ok(b)
    }
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
    pub fn unchanged(&self) -> Result<bool> {
        let stat = self.file.metadata()?;
        Ok(stat.len() == self.size && stat.modified()? == self.modified)
    }
}
