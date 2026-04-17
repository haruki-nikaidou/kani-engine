//! `kani-bundler run` — launch the game in developer mode.

use std::path::Path;

use anyhow::Result;

use crate::config::{asset_base, load_config};

pub fn run(project_dir: &Path) -> Result<()> {
    let cfg = load_config(project_dir)?;
    let base = asset_base(project_dir, &cfg);
    kani_runtime::run_develop(base, &cfg.entry.start)
}
