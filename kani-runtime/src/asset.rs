use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use bevy::asset::AssetApp;
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::prelude::App;
use kani_pak::{PakAssetReader, PakReader};

#[derive(Clone, bevy::prelude::Resource)]
pub enum AssetBackend {
    FileSystem { base: PathBuf },
    Pak { reader: Arc<PakReader> },
}

impl AssetBackend {
    pub fn file_system(base: impl Into<PathBuf>) -> Self {
        Self::FileSystem { base: base.into() }
    }

    pub fn pak(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::Pak {
            reader: Arc::new(PakReader::open(path.as_ref())?),
        })
    }

    pub fn load_text(&self, path: &str) -> Result<String> {
        let bytes = self.load_bytes(path)?;
        String::from_utf8(bytes).context("asset is not valid UTF-8 text")
    }

    pub fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        match self {
            AssetBackend::FileSystem { base } => {
                let full = base.join(path);
                std::fs::read(&full).with_context(|| {
                    format!("failed to read asset from filesystem: {}", full.display())
                })
            }
            AssetBackend::Pak { reader } => reader
                .read(path)
                .with_context(|| format!("failed to read asset from pak: {path}")),
        }
    }

    pub fn register_bevy_source(&self, app: &mut App) {
        if let AssetBackend::Pak { reader } = self {
            app.register_asset_source(
                AssetSourceId::Name("pak".into()),
                AssetSourceBuilder::new({
                    let reader = reader.clone();
                    move || Box::new(PakAssetReader::from_reader(reader.clone()))
                }),
            );
        }
    }
}
