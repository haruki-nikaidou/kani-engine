//! `kani-bundler new <name>` — scaffold a new project from the bundled template.

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use include_dir::{Dir, include_dir};

/// The entire `init/` directory tree, embedded at compile time.
static INIT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/init");

pub fn run(name: &str) -> Result<()> {
    let dest = Path::new(name);
    if dest.exists() {
        bail!("'{}' already exists", dest.display());
    }

    // Extract every embedded file, preserving the directory tree.
    extract_dir(&INIT_DIR, dest)?;

    // Write the project manifest (kani.toml) at the project root.
    let kani_toml = format!(
        r#"[project]
name    = "{name}"
version = "0.1.0"
author  = ""

[entry]
start = "first.ks"

[assets]
base = "src"

[build]
target      = ""
output      = "dist"
compression = "zstd"
"#
    );
    write_file(&dest.join("kani.toml"), &kani_toml)?;

    println!("Created project '{name}'.");
    println!("  Assets and starter scripts are in src/");
    println!("  Entry scenario: src/first.ks");
    println!("  Run with:  kani-bundler run --project {name}");

    Ok(())
}

/// Recursively extract the contents of an embedded [`Dir`] into `dest`.
fn extract_dir(dir: &Dir<'_>, dest: &Path) -> Result<()> {
    for sub in dir.dirs() {
        let target = dest.join(sub.path());
        std::fs::create_dir_all(&target)
            .with_context(|| format!("creating directory '{}'", target.display()))?;
        extract_dir(sub, dest)?;
    }

    for file in dir.files() {
        let target = dest.join(file.path());
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory '{}'", parent.display()))?;
        }
        std::fs::write(&target, file.contents())
            .with_context(|| format!("writing '{}'", target.display()))?;
    }

    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content).with_context(|| format!("writing '{}'", path.display()))
}
