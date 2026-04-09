//! Bevy [`AssetReader`] implementation backed by a `.pak` file.
//!
//! # Registration
//!
//! ```rust,no_run
//! use bevy_asset::io::{AssetSource, AssetSourceId};
//! use kani_pak::PakAssetReader;
//! use std::path::Path;
//!
//! // In your Bevy App setup:
//! // app.register_asset_source(
//! //     AssetSourceId::Name("pak".into()),
//! //     AssetSource::build()
//! //         .with_reader(|| Box::new(
//! //             PakAssetReader::open(Path::new("game.pak")).unwrap()
//! //         )),
//! // );
//! ```
//!
//! Asset paths are then addressed as `pak://assets/bg/forest.png`.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bevy_asset::io::{AssetReader, AssetReaderError, PathStream, Reader, VecReader};
use futures_core::stream::Stream;

use crate::error::PakError;
use crate::reader::PakReader;

/// Bevy [`AssetReader`] that reads assets from a `.pak` archive.
///
/// Wrap this in an [`Arc`] if you want to share it across multiple
/// asset sources; otherwise construct one per source registration.
pub struct PakAssetReader(Arc<PakReader>);

impl PakAssetReader {
    /// Open the `.pak` file at `path` and return a reader backed by it.
    pub fn open(path: &Path) -> Result<Self, PakError> {
        let reader = PakReader::open(path)?;
        Ok(Self(Arc::new(reader)))
    }

    /// Construct from an existing `Arc<PakReader>` (useful when the reader is
    /// shared between the asset source and the interpreter bridge).
    pub fn from_reader(reader: Arc<PakReader>) -> Self {
        Self(reader)
    }

    /// Access the underlying reader, e.g. to call [`PakReader::read`] from the
    /// interpreter bridge without going through the Bevy asset pipeline.
    pub fn reader(&self) -> &Arc<PakReader> {
        &self.0
    }
}

impl AssetReader for PakAssetReader {
    /// Return the decompressed bytes for `path`.
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let path_str = path_to_str(path)?;
        let data = self
            .0
            .read(path_str)
            .map_err(|e| pak_err_to_asset_err(e, path))?;
        Ok(VecReader::new(data))
    }

    /// Return the bytes of the `.meta` sidecar file for `path`, if present.
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let path_str = path_to_str(path)?;
        let meta_key = format!("{path_str}.meta");
        let data = self
            .0
            .read(&meta_key)
            .map_err(|e| pak_err_to_asset_err(e, path))?;
        Ok(VecReader::new(data))
    }

    /// Return a stream of direct children of the virtual directory `path`.
    ///
    /// Because `.pak` files have a flat layout, "directories" are inferred
    /// from path prefixes.  Only **immediate** children are yielded.
    ///
    /// An empty `path` (the root) lists every top-level entry.
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let prefix = path_to_str(path)?;

        let children: Vec<PathBuf> = if prefix.is_empty() {
            // Root directory: yield every entry that has no '/' (top-level).
            self.0
                .paths()
                .filter(|p| !p.contains('/'))
                .map(PathBuf::from)
                .collect()
        } else {
            // Non-root: strip the prefix (with trailing slash) and keep only
            // immediate children — those whose remainder has no further '/'.
            let prefix_with_slash = if prefix.ends_with('/') {
                prefix.to_owned()
            } else {
                format!("{prefix}/")
            };
            self.0
                .paths()
                .filter_map(|p| {
                    let rest = p.strip_prefix(&prefix_with_slash)?;
                    if rest.contains('/') {
                        None
                    } else {
                        Some(PathBuf::from(p))
                    }
                })
                .collect()
        };

        if children.is_empty() && !self.0.is_empty() {
            // Verify that at least one entry lives anywhere under this prefix.
            let has_any = if prefix.is_empty() {
                !self.0.is_empty()
            } else {
                let prefix_with_slash = if prefix.ends_with('/') {
                    prefix.to_owned()
                } else {
                    format!("{prefix}/")
                };
                self.0.paths().any(|p| p.starts_with(&prefix_with_slash))
            };
            if !has_any {
                return Err(AssetReaderError::NotFound(path.to_owned()));
            }
        }

        Ok(Box::new(OwnedPathStream::new(children)))
    }

    /// Return `true` if `path` is a virtual directory (i.e. any entry has it
    /// as a path prefix, or `path` is empty and the archive is non-empty).
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let prefix = path_to_str(path)?;
        if prefix.is_empty() {
            return Ok(!self.0.is_empty());
        }
        let prefix_with_slash = if prefix.ends_with('/') {
            prefix.to_owned()
        } else {
            format!("{prefix}/")
        };
        Ok(self.0.paths().any(|p| p.starts_with(&prefix_with_slash)))
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn path_to_str(path: &Path) -> Result<&str, AssetReaderError> {
    path.to_str()
        .ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))
}

fn pak_err_to_asset_err(err: PakError, path: &Path) -> AssetReaderError {
    match err {
        PakError::NotFound(_) => AssetReaderError::NotFound(path.to_owned()),
        other => AssetReaderError::Io(Arc::new(std::io::Error::other(other.to_string()))),
    }
}

// ── OwnedPathStream ───────────────────────────────────────────────────────────

/// A simple `Stream<Item = PathBuf>` that drains a `Vec`.
struct OwnedPathStream {
    iter: std::vec::IntoIter<PathBuf>,
}

impl OwnedPathStream {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            iter: paths.into_iter(),
        }
    }
}

impl Stream for OwnedPathStream {
    type Item = PathBuf;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<PathBuf>> {
        Poll::Ready(self.iter.next())
    }
}

impl Unpin for OwnedPathStream {}
