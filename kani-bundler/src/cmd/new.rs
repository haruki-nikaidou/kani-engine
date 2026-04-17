//! `kani-bundler new <name>` — scaffold a new project.

use std::path::Path;

use anyhow::{Context as _, Result, bail};

pub fn run(name: &str) -> Result<()> {
    let dest = Path::new(name);
    if dest.exists() {
        bail!("'{}' already exists", dest.display());
    }

    let dirs = [
        "data/scenario",
        "data/bgimage",
        "data/fgimage",
        "data/bgm",
        "data/image",
    ];
    for dir in &dirs {
        let path = dest.join(dir);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("creating directory '{}'", path.display()))?;
    }

    // kani.toml
    let kani_toml = format!(
        r#"[project]
name    = "{name}"
version = "0.1.0"
author  = ""

[entry]
start = "scenario/first.ks"

[assets]
base = "data"

[build]
target      = ""
output      = "dist"
compression = "zstd"
"#
    );
    write_file(&dest.join("kani.toml"), &kani_toml)?;

    // Minimal starter script
    let first_ks = r#"; first.ks — entry scenario
[bg storage="bgimage/placeholder.jpg"]
[l]
Here is the first line of your story.[l]
[end]
"#;
    write_file(&dest.join("data/scenario/first.ks"), first_ks)?;

    println!("Created project '{name}'.");
    println!("  Edit data/scenario/first.ks to get started.");
    println!("  Run with:  kani-bundler run --project {name}");

    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)
        .with_context(|| format!("writing '{}'", path.display()))
}
