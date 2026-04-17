//! [`PakWriter`] — creates `.pak` archives.
//!
//! The bundler drives all decisions (which files to include, at what
//! compression level). `PakWriter` only serialises the format.
//!
//! # Example
//!
//! ```rust,no_run
//! use kani_pak::writer::{Compression, PakWriter};
//!
//! let mut w = PakWriter::new();
//! w.add("script/intro.ks", b"[wait time=1]", Compression::None).unwrap();
//! w.add("bg/forest.png",   &std::fs::read("bg/forest.png").unwrap(),
//!       Compression::Zstd { level: 3 }).unwrap();
//! let pak_bytes = w.finish();
//! std::fs::write("game.pak", pak_bytes).unwrap();
//! ```

use std::io::Write;

use bytemuck::bytes_of;

use crate::error::PakError;
use crate::format::{
    COMPRESSION_NONE, COMPRESSION_ZSTD, HEADER_SIZE, Header, INDEX_ENTRY_SIZE, IndexEntry, MAGIC,
    VERSION, hash_path,
};

/// How to compress an entry when adding it to the archive.
#[derive(Clone, Debug)]
pub enum Compression {
    /// Store verbatim — no compression overhead.
    None,
    /// Compress with zstd at the given level (`1`–`22`; `3` is a good default).
    Zstd { level: i32 },
}

// Internal representation of a staged entry before serialisation.
struct StagedEntry {
    #[allow(dead_code)]
    path: String,
    path_hash: u64,
    /// Byte offset of this path in the string table accumulated so far.
    path_offset: u32,
    path_len: u16,
    compression: u8,
    /// Compressed (or raw) payload bytes.
    data: Vec<u8>,
    uncompressed_size: u32,
}

/// Builds a `.pak` archive in memory, then writes it all at once.
///
/// The bundler is responsible for deciding what to include and which
/// compression to apply.  [`PakWriter`] handles only the binary layout.
#[derive(Default)]
pub struct PakWriter {
    staged: Vec<StagedEntry>,
    string_table: Vec<u8>,
}

impl PakWriter {
    /// Create an empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage `data` under `path` with the given compression strategy.
    ///
    /// Paths are stored verbatim — use forward slashes and no leading slash,
    /// e.g. `"assets/bg/forest.png"`.
    ///
    /// Returns [`PakError::PathTooLong`] if `path` exceeds 65535 bytes.
    pub fn add(
        &mut self,
        path: &str,
        data: &[u8],
        compression: Compression,
    ) -> Result<(), PakError> {
        let path_bytes = path.as_bytes();
        let path_len =
            u16::try_from(path_bytes.len()).map_err(|_| PakError::PathTooLong(path_bytes.len()))?;

        let path_hash = hash_path(path);
        let uncompressed_size = data.len() as u32;

        let compressed = match &compression {
            Compression::None => data.to_vec(),
            Compression::Zstd { level } => zstd::encode_all(data, *level)?,
        };
        let compression_byte = match compression {
            Compression::None => COMPRESSION_NONE,
            Compression::Zstd { .. } => COMPRESSION_ZSTD,
        };

        let path_offset = self.string_table.len() as u32;
        self.string_table.extend_from_slice(path_bytes);

        self.staged.push(StagedEntry {
            path: path.to_owned(),
            path_hash,
            path_offset,
            path_len,
            compression: compression_byte,
            data: compressed,
            uncompressed_size,
        });
        Ok(())
    }

    /// Serialise the archive into a `Vec<u8>`.
    ///
    /// Panics only if the internal `Vec` write fails, which cannot happen.
    pub fn finish(self) -> Vec<u8> {
        let mut buf = Vec::new();

        #[allow(clippy::expect_used)]
        self.write_to(&mut buf)
            .expect("in-memory write must not fail");
        buf
    }

    /// Serialise the archive by writing to `w`.
    ///
    /// This consumes the writer. Entries are sorted by `path_hash` before
    /// being written so the reader can binary-search the index.
    pub fn write_to(mut self, w: &mut impl Write) -> Result<(), PakError> {
        // Sort by hash so the reader can binary-search.
        self.staged.sort_unstable_by_key(|e| e.path_hash);

        let entry_count = self.staged.len() as u32;
        let index_offset = HEADER_SIZE as u64;
        let strtab_offset = index_offset + (entry_count as u64) * (INDEX_ENTRY_SIZE as u64);
        let data_offset = strtab_offset + self.string_table.len() as u64;

        // Header
        let header = Header {
            magic: MAGIC,
            version: VERSION,
            flags: 0,
            entry_count,
            _pad: 0,
            index_offset,
            strtab_offset,
            data_offset,
        };
        w.write_all(bytes_of(&header))?;

        // Index table — track the running data offset as we go
        let mut current_data_offset: u64 = 0;
        for entry in &self.staged {
            let ie = IndexEntry {
                path_hash: entry.path_hash,
                path_offset: entry.path_offset,
                path_len: entry.path_len,
                compression: entry.compression,
                _pad: 0,
                data_offset: current_data_offset,
                compressed_size: entry.data.len() as u32,
                uncompressed_size: entry.uncompressed_size,
            };
            w.write_all(bytes_of(&ie))?;
            current_data_offset += entry.data.len() as u64;
        }

        // String table
        w.write_all(&self.string_table)?;

        // Data blobs
        for entry in &self.staged {
            w.write_all(&entry.data)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::PakReader;

    fn roundtrip(entries: &[(&str, &[u8], Compression)]) -> PakReader {
        let mut w = PakWriter::new();
        for (path, data, comp) in entries {
            w.add(path, data, comp.clone()).unwrap();
        }
        let bytes = w.finish();
        // Wrap in an anonymous mmap via a temp file.
        use std::io::Write as _;
        let mut tmp = tempfile::tempfile().unwrap();
        tmp.write_all(&bytes).unwrap();
        let mmap = unsafe { memmap2::Mmap::map(&tmp).unwrap() };
        PakReader::from_mmap(mmap).unwrap()
    }

    #[test]
    fn uncompressed_roundtrip() {
        let reader = roundtrip(&[("hello.txt", b"world", Compression::None)]);
        assert!(reader.contains("hello.txt"));
        assert_eq!(reader.read("hello.txt").unwrap(), b"world");
    }

    #[test]
    fn zstd_roundtrip() {
        let data = b"the quick brown fox jumps over the lazy dog ".repeat(100);
        let reader = roundtrip(&[("big.bin", &data, Compression::Zstd { level: 3 })]);
        assert_eq!(reader.read("big.bin").unwrap(), data);
    }

    #[test]
    fn path_too_long_is_rejected() {
        let long_path = "a".repeat(65536);
        let mut w = PakWriter::new();
        assert!(matches!(
            w.add(&long_path, b"data", Compression::None),
            Err(crate::error::PakError::PathTooLong(65536))
        ));
    }

    #[test]
    fn multiple_entries() {
        let reader = roundtrip(&[
            ("a.txt", b"alpha", Compression::None),
            ("b.txt", b"beta", Compression::None),
            ("c.txt", b"gamma", Compression::Zstd { level: 1 }),
        ]);
        assert_eq!(reader.read("a.txt").unwrap(), b"alpha");
        assert_eq!(reader.read("b.txt").unwrap(), b"beta");
        assert_eq!(reader.read("c.txt").unwrap(), b"gamma");
        assert!(!reader.contains("d.txt"));
    }
}
