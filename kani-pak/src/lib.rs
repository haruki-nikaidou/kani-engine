//! `kani-pak` — custom binary package format for kani-engine.
//!
//! A `.pak` file is a flat archive with a memory-mappable, sorted index.
//! Reading is zero-copy for uncompressed entries and on-demand for zstd-compressed ones.
//!
//! # Feature flags
//!
//! | Feature | What it enables |
//! |---------|-----------------|
//! | `bevy`  | [`PakAssetReader`] implementing bevy's `AssetReader` |
//! | `write` | [`PakWriter`] for creating `.pak` files (bundler use) |
//!
//! The `write` feature has no dependency on bevy; the bundler decides what to pack
//! and how — `kani-pak` only serialises the format.

pub mod error;
pub mod format;
pub mod reader;

#[cfg(feature = "write")]
pub mod writer;

#[cfg(feature = "bevy")]
pub mod asset_reader;

pub use error::PakError;
pub use reader::PakReader;

#[cfg(feature = "write")]
pub use writer::{Compression, PakWriter};

#[cfg(feature = "bevy")]
pub use asset_reader::PakAssetReader;
