//! Binary format definitions for `.pak` files.
//!
//! ## Layout
//!
//! ```text
//! [Header          – 48 bytes]
//! [Index Table     – entry_count × 32 bytes, sorted by path_hash]
//! [String Table    – packed UTF-8 path strings]
//! [Data Section    – raw or zstd-compressed blobs]
//! ```
//!
//! All multi-byte integers are little-endian.

use bytemuck::{Pod, Zeroable};
use rustc_hash::FxHasher;
use std::hash::Hasher;

/// Magic bytes at the start of every `.pak` file.
pub const MAGIC: [u8; 8] = *b"KANIPAK\0";

/// Current format version.
pub const VERSION: u32 = 1;

/// Size of the serialised [`Header`] in bytes.
pub const HEADER_SIZE: usize = std::mem::size_of::<Header>();

/// Size of one serialised [`IndexEntry`] in bytes.
pub const INDEX_ENTRY_SIZE: usize = std::mem::size_of::<IndexEntry>();

/// Compression discriminant: no compression.
pub const COMPRESSION_NONE: u8 = 0;

/// Compression discriminant: zstd.
pub const COMPRESSION_ZSTD: u8 = 1;

/// File header — always at byte offset 0.
///
/// ```text
/// offset  size  field
///      0     8  magic
///      8     4  version
///     12     4  flags       (reserved, must be 0)
///     16     4  entry_count
///     20     4  _pad
///     24     8  index_offset
///     32     8  strtab_offset
///     40     8  data_offset
/// total: 48 bytes
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Header {
    pub magic: [u8; 8],
    pub version: u32,
    pub flags: u32,
    pub entry_count: u32,
    pub _pad: u32,
    pub index_offset: u64,
    pub strtab_offset: u64,
    pub data_offset: u64,
}

/// One entry in the index table.
///
/// Entries are sorted by `path_hash` to allow O(log n) binary search.
///
/// ```text
/// offset  size  field
///      0     8  path_hash
///      8     4  path_offset   (byte offset into the String Table)
///     12     2  path_len      (byte length of the path string)
///     14     1  compression   (COMPRESSION_NONE or COMPRESSION_ZSTD)
///     15     1  _pad
///     16     8  data_offset   (byte offset relative to data_offset in Header)
///     24     4  compressed_size
///     28     4  uncompressed_size
/// total: 32 bytes
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct IndexEntry {
    pub path_hash: u64,
    pub path_offset: u32,
    pub path_len: u16,
    pub compression: u8,
    pub _pad: u8,
    pub data_offset: u64,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
}

/// Hash an asset path to a `u64` using FxHash.
///
/// Both [`crate::reader::PakReader`] and [`crate::writer::PakWriter`] must use
/// this function so the index remains consistent.
#[inline]
pub fn hash_path(path: &str) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(path.as_bytes());
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(HEADER_SIZE, 48);
        assert_eq!(INDEX_ENTRY_SIZE, 32);
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(hash_path("assets/bg.png"), hash_path("assets/bg.png"));
        assert_ne!(hash_path("a"), hash_path("b"));
    }
}
