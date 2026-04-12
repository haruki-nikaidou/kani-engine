//! Asset abstraction layer.
//!
//! [`AssetBackend`] hides whether assets live on the raw filesystem (dev) or
//! inside a `.pak` archive (release).  Scenario `.ks` files are loaded
//! synchronously via [`AssetBackend::load_text`].  Binary assets (images,
//! audio) go through Bevy's `AssetServer` after the backend has registered
//! itself as a named asset source.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::asset::AssetApp;
use bevy::prelude::App;
use kani_pak::{PakAssetReader, PakReader};

/// Switchable asset storage: raw filesystem (dev) or `.pak` archive (release).
#[derive(Clone)]
pub enum AssetBackend {
    /// Assets are read directly from the filesystem under `base`.
    FileSystem { base: PathBuf },
    /// Assets are read from a memory-mapped `.pak` archive.
    Pak { reader: Arc<PakReader> },
}

impl AssetBackend {
    /// Convenience constructor — open a `.pak` file and wrap it.
    pub fn from_pak(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let reader =
            PakReader::open(path).with_context(|| format!("opening pak '{}'", path.display()))?;
        Ok(Self::Pak { reader: Arc::new(reader) })
    }

    /// Synchronously load a UTF-8 text file (used for `.ks` scenario files).
    pub fn load_text(&self, path: &str) -> Result<String> {
        let bytes = self.load_bytes(path)?;
        String::from_utf8(bytes)
            .map_err(|e| anyhow!("asset '{path}' is not valid UTF-8: {e}"))
    }

    /// Synchronously load raw bytes.
    pub fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        match self {
            Self::FileSystem { base } => {
                let full = base.join(path);
                std::fs::read(&full)
                    .with_context(|| format!("reading '{}'", full.display()))
            }
            Self::Pak { reader } => {
                reader.read(path).with_context(|| format!("reading '{path}' from pak"))
            }
        }
    }

    /// Register this backend as a Bevy asset source.
    ///
    /// In `Pak` mode a source named `"pak"` is registered; assets are then
    /// addressed as `pak://path/to/asset.png`.  In `FileSystem` mode no extra
    /// source is registered because Bevy's default source already reads from
    /// the project directory.
    pub fn register_bevy_source(self, app: &mut App) {
        if let Self::Pak { reader } = self {
            let reader = Arc::clone(&reader);
            app.register_asset_source(
                AssetSourceId::Name("pak".into()),
                AssetSourceBuilder::new(move || Box::new(PakAssetReader::from_reader(Arc::clone(&reader)))),
            );
        }
    }

    /// Return the URL prefix used when building asset paths for `AssetServer`.
    ///
    /// `""` for filesystem mode, `"pak://"` for pak mode.
    pub fn asset_prefix(&self) -> &'static str {
        match self {
            Self::FileSystem { .. } => "",
            Self::Pak { .. } => "pak://",
        }
    }
}
