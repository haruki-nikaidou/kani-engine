//! `kani.toml` schema and loader.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::Deserialize;

// ─── Top-level config ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct KaniConfig {
    pub project: ProjectConfig,
    pub entry: EntryConfig,
    pub assets: AssetsConfig,
    pub build: BuildConfig,
}

// ─── [project] ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// Optional metadata; not used by the tooling but preserved in kani.toml.
    #[serde(default)]
    #[allow(dead_code)]
    pub author: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub description: String,
}

fn default_version() -> String {
    "0.1.0".to_owned()
}

// ─── [entry] ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EntryConfig {
    /// Path to the starting scenario file, relative to `assets.base`.
    pub start: String,
}

// ─── [assets] ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AssetsConfig {
    /// Root asset directory, relative to the project root.
    #[serde(default = "default_assets_base")]
    pub base: String,

    /// `(tag_name, attribute_name)` pairs whose values are asset file paths.
    /// Used by `check` (missing-file validation) and `bundle` (path rewriting).
    #[serde(default = "default_asset_attrs")]
    pub asset_attrs: Vec<[String; 2]>,
}

fn default_assets_base() -> String {
    "src".to_owned()
}

fn default_asset_attrs() -> Vec<[String; 2]> {
    [
        ["bg", "storage"],
        ["image", "storage"],
        ["chara", "storage"],
        ["bgm", "storage"],
        ["playse", "storage"],
        ["voice", "storage"],
        ["video", "storage"],
    ]
    .into_iter()
    .map(|[t, a]| [t.to_owned(), a.to_owned()])
    .collect()
}

impl AssetsConfig {
    /// Returns `true` if `(tag_name, attr_name)` is an asset reference pair.
    pub fn is_asset_attr(&self, tag: &str, attr: &str) -> bool {
        self.asset_attrs.iter().any(|[t, a]| t == tag && a == attr)
    }
}

// ─── [build] ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    /// Rust target triple, e.g. `"x86_64-unknown-linux-gnu"`.
    /// Empty string means the host target.
    #[serde(default)]
    pub target: String,

    /// Output directory for bundled artefacts, relative to the project root.
    #[serde(default = "default_output")]
    pub output: String,

    /// Compression strategy: `"none"` or `"zstd"`.
    #[serde(default = "default_compression")]
    pub compression: String,
}

fn default_output() -> String {
    "dist".to_owned()
}

fn default_compression() -> String {
    "zstd".to_owned()
}

// ─── Loader ───────────────────────────────────────────────────────────────────

/// Load and parse the `kani.toml` in `project_dir`.
pub fn load_config(project_dir: &Path) -> Result<KaniConfig> {
    let path = project_dir.join("kani.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading '{}'", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing '{}'", path.display()))
}

/// Resolve the absolute path to the asset base directory.
pub fn asset_base(project_dir: &Path, cfg: &KaniConfig) -> PathBuf {
    project_dir.join(&cfg.assets.base)
}
