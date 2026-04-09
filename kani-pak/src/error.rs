use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PakError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bad magic bytes: expected b\"KANIPAK\\0\", got {0:?}")]
    BadMagic([u8; 8]),

    #[error("unsupported pak version {0} (this build supports version 1)")]
    VersionMismatch(u32),

    #[error("asset not found in pak: {0}")]
    NotFound(String),

    #[error("decompression failed: {0}")]
    DecompressError(String),

    #[error("pak file is truncated or corrupt")]
    Corrupt,

    #[error("path is too long: {0} bytes (maximum is 65535)")]
    PathTooLong(usize),

    #[error("write error for path {path:?}: {source}")]
    WriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
