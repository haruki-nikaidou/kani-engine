use std::path::Path;

use bytemuck::cast_slice;
use memmap2::Mmap;

use crate::error::PakError;
use crate::format::{
    hash_path, Header, IndexEntry, COMPRESSION_NONE, COMPRESSION_ZSTD, HEADER_SIZE,
    INDEX_ENTRY_SIZE, MAGIC, VERSION,
};

/// Read-only handle to an open `.pak` file.
///
/// The file is memory-mapped for the lifetime of this struct. Uncompressed
/// entries are served as zero-copy slices into the map; zstd-compressed
/// entries are decompressed on demand into a `Vec<u8>`.
pub struct PakReader {
    mmap: Mmap,
    entry_count: usize,
    index_offset: usize,
    strtab_offset: usize,
    data_offset: usize,
}

// SAFETY: Mmap is a read-only mapping; sharing it across threads is safe.
unsafe impl Send for PakReader {}
unsafe impl Sync for PakReader {}

impl PakReader {
    /// Open and memory-map a `.pak` file at `path`.
    pub fn open(path: &Path) -> Result<Self, PakError> {
        let file = std::fs::File::open(path)?;
        // SAFETY: the caller must not truncate the file while this mapping lives.
        let mmap = unsafe { Mmap::map(&file)? };
        Self::from_mmap(mmap)
    }

    /// Construct a reader from an already-created `Mmap` (e.g. from an
    /// anonymous or custom mapping).
    pub fn from_mmap(mmap: Mmap) -> Result<Self, PakError> {
        if mmap.len() < HEADER_SIZE {
            return Err(PakError::Corrupt);
        }
        let header: &Header = bytemuck::from_bytes(&mmap[..HEADER_SIZE]);
        if header.magic != MAGIC {
            return Err(PakError::BadMagic(header.magic));
        }
        if header.version != VERSION {
            return Err(PakError::VersionMismatch(header.version));
        }
        let entry_count = header.entry_count as usize;
        let index_offset = header.index_offset as usize;
        let strtab_offset = header.strtab_offset as usize;
        let data_offset = header.data_offset as usize;

        let index_end = index_offset + entry_count * INDEX_ENTRY_SIZE;
        if index_end > mmap.len() || strtab_offset > mmap.len() || data_offset > mmap.len() {
            return Err(PakError::Corrupt);
        }

        Ok(Self {
            mmap,
            entry_count,
            index_offset,
            strtab_offset,
            data_offset,
        })
    }

    /// Number of entries in the archive.
    #[inline]
    pub fn len(&self) -> usize {
        self.entry_count
    }

    /// `true` if the archive contains no entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entry_count == 0
    }

    /// Iterate over every asset path stored in this pak.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries().iter().map(|e| self.entry_path(e))
    }

    /// Return `true` if `asset_path` exists in this pak.
    pub fn contains(&self, asset_path: &str) -> bool {
        self.find_entry(asset_path).is_some()
    }

    /// Read an entry, decompressing it if necessary.
    ///
    /// For uncompressed entries this still copies the bytes into a `Vec`.
    /// Prefer [`read_raw`](Self::read_raw) when you know the entry is
    /// uncompressed and want a zero-copy slice.
    pub fn read(&self, asset_path: &str) -> Result<Vec<u8>, PakError> {
        let entry = self
            .find_entry(asset_path)
            .ok_or_else(|| PakError::NotFound(asset_path.to_owned()))?;
        let raw = self.entry_data(entry)?;
        match entry.compression {
            COMPRESSION_NONE => Ok(raw.to_vec()),
            COMPRESSION_ZSTD => zstd::decode_all(raw)
                .map_err(|e| PakError::DecompressError(e.to_string())),
            other => Err(PakError::DecompressError(format!(
                "unknown compression type {other:#x}"
            ))),
        }
    }

    /// Return a zero-copy slice into the mapped data for an **uncompressed**
    /// entry.  Returns [`PakError::DecompressError`] if the entry is
    /// compressed — use [`read`](Self::read) instead in that case.
    pub fn read_raw(&self, asset_path: &str) -> Result<&[u8], PakError> {
        let entry = self
            .find_entry(asset_path)
            .ok_or_else(|| PakError::NotFound(asset_path.to_owned()))?;
        if entry.compression != COMPRESSION_NONE {
            return Err(PakError::DecompressError(
                "entry is compressed; use read() instead of read_raw()".to_owned(),
            ));
        }
        self.entry_data(entry)
    }

    // ── private helpers ──────────────────────────────────────────────────────

    fn entries(&self) -> &[IndexEntry] {
        let start = self.index_offset;
        let end = start + self.entry_count * INDEX_ENTRY_SIZE;
        cast_slice(&self.mmap[start..end])
    }

    /// Binary-search the sorted index for `path`.  Handles hash collisions by
    /// scanning forward/backward from the found position.
    fn find_entry(&self, path: &str) -> Option<&IndexEntry> {
        let hash = hash_path(path);
        let entries = self.entries();
        let idx = entries
            .binary_search_by_key(&hash, |e| e.path_hash)
            .ok()?;

        // Walk left to the first entry with this hash.
        let mut lo = idx;
        while lo > 0 && entries[lo - 1].path_hash == hash {
            lo -= 1;
        }
        // Scan forward over all entries sharing the hash.
        for entry in &entries[lo..] {
            if entry.path_hash != hash {
                break;
            }
            if self.entry_path(entry) == path {
                return Some(entry);
            }
        }
        None
    }

    fn entry_path<'a>(&'a self, entry: &IndexEntry) -> &'a str {
        let start = self.strtab_offset + entry.path_offset as usize;
        let end = start + entry.path_len as usize;
        if end > self.mmap.len() {
            return "";
        }
        std::str::from_utf8(&self.mmap[start..end]).unwrap_or("")
    }

    fn entry_data<'a>(&'a self, entry: &IndexEntry) -> Result<&'a [u8], PakError> {
        let start = self.data_offset + entry.data_offset as usize;
        let end = start + entry.compressed_size as usize;
        if end > self.mmap.len() {
            return Err(PakError::Corrupt);
        }
        Ok(&self.mmap[start..end])
    }
}
