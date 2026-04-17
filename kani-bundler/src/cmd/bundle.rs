//! `kani-bundler bundle` — full release build pipeline.
//!
//! Steps:
//! 1. Walk all non-`.ks` files in the asset base, hash each with BLAKE3.
//! 2. Build a `path_map`: original relative path → `<hex>.<ext>` pak key.
//! 3. Parse every `.ks` script; rewrite asset-reference attribute values using
//!    the path map; serialize the modified source text.
//! 4. Add all hashed assets + rewritten scripts to a [`PakWriter`].
//! 5. Write the `.pak` to `<output>/game.pak`.
//! 6. Install `kani-init` from crates.io via `cargo install` into a temp dir.
//! 7. Write an `init` file containing the entry script name next to the binary.
//! 8. Copy the binary and `init` file to `<output>/`.
//! 9. Zip `<output>/` into `<output>.zip` for distribution.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use kag_syntax::{Op, ParamValue, TextPart, parse_script};
use kani_pak::writer::{Compression, PakWriter};
use tempfile::TempDir;
use walkdir::WalkDir;

use crate::config::{AssetsConfig, asset_base, load_config};


// ─── Entry point ──────────────────────────────────────────────────────────────

pub fn run(project_dir: &Path, target_override: Option<&str>) -> Result<()> {
    let cfg = load_config(project_dir)?;
    let base = asset_base(project_dir, &cfg);
    let output_dir = project_dir.join(&cfg.build.output);

    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("creating output dir '{}'", output_dir.display()))?;

    let compression = parse_compression(&cfg.build.compression);

    println!(
        "bundler: building '{}' v{}...",
        cfg.project.name, cfg.project.version
    );

    // ── Step 1 & 2: hash all non-script assets ────────────────────────────────
    println!("bundler: hashing assets...");
    let path_map = build_path_map(&base)?;
    println!("bundler: {} asset(s) hashed.", path_map.len());

    // ── Step 3 & 4: write pak ─────────────────────────────────────────────────
    println!("bundler: packing assets and scripts...");
    let mut writer = PakWriter::new();

    // Add hashed binary assets.
    for (original_rel, pak_key) in &path_map {
        let full_path = base.join(original_rel);
        let data = std::fs::read(&full_path)
            .with_context(|| format!("reading '{}'", full_path.display()))?;
        writer
            .add(pak_key, &data, compression.clone())
            .with_context(|| format!("adding '{}' to pak", pak_key))?;
    }

    // Add rewritten scripts.
    let scenario_dir = base.join("scenario");
    add_scripts_to_pak(
        &mut writer,
        &scenario_dir,
        &base,
        &path_map,
        &cfg.assets,
        compression.clone(),
    )?;

    // ── Step 5: write .pak file ───────────────────────────────────────────────
    let pak_path = output_dir.join("game.pak");
    let pak_bytes = writer.finish();
    std::fs::write(&pak_path, &pak_bytes)
        .with_context(|| format!("writing pak to '{}'", pak_path.display()))?;
    println!("bundler: wrote '{}' ({} bytes).", pak_path.display(), pak_bytes.len());

    // ── Step 6: install kani-init from crates.io ─────────────────────────────
    let target = target_override
        .map(str::to_owned)
        .or_else(|| {
            if cfg.build.target.is_empty() {
                None
            } else {
                Some(cfg.build.target.clone())
            }
        });

    let (binary_path, _tmp) = install_kani_init(target.as_deref())?;

    // ── Step 7: write `init` file and copy binary ─────────────────────────────
    let dest = output_dir.join(binary_path.file_name().unwrap_or(binary_path.as_os_str()));
    std::fs::copy(&binary_path, &dest)
        .with_context(|| format!("copying binary to '{}'", dest.display()))?;
    // _tmp is dropped here — safe because the binary is already copied.

    let init_path = output_dir.join("init");
    std::fs::write(&init_path, cfg.entry.start.as_bytes())
        .with_context(|| format!("writing init file '{}'", init_path.display()))?;
    println!("bundler: binary at '{}', init file written.", dest.display());

    // ── Step 8: zip the output directory ─────────────────────────────────────
    let zip_path = {
        let mut p = output_dir.clone();
        let stem = p.file_name().map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dist".to_owned());
        p.pop();
        p.join(format!("{stem}.zip"))
    };
    create_zip(&output_dir, &zip_path)
        .with_context(|| format!("creating zip '{}'", zip_path.display()))?;
    println!("bundler: release archive at '{}'.", zip_path.display());

    println!("bundler: done.");
    Ok(())
}

// ─── Asset hashing ────────────────────────────────────────────────────────────

fn build_path_map(asset_base: &Path) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();

    for entry in WalkDir::new(asset_base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().is_some_and(|x| x == "ks") {
            continue; // Scripts are handled separately.
        }

        let data = std::fs::read(path)
            .with_context(|| format!("reading '{}'", path.display()))?;

        let hash = blake3::hash(&data);
        let hex = hash.to_hex();

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let pak_key = if ext.is_empty() {
            hex.to_string()
        } else {
            format!("{hex}.{ext}")
        };

        let rel = relative_path(asset_base, path)?;
        map.insert(rel, pak_key);
    }

    Ok(map)
}

// ─── Script rewriting ─────────────────────────────────────────────────────────

fn add_scripts_to_pak(
    writer: &mut PakWriter,
    scenario_dir: &Path,
    asset_base: &Path,
    path_map: &HashMap<String, String>,
    assets_cfg: &AssetsConfig,
    compression: Compression,
) -> Result<()> {
    for entry in WalkDir::new(scenario_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "ks"))
    {
        let path = entry.path();
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("reading script '{}'", path.display()))?;

        let rewritten = rewrite_script(&source, path, path_map, assets_cfg);

        let rel = relative_path(asset_base, path)?;
        writer
            .add(&rel, rewritten.as_bytes(), compression.clone())
            .with_context(|| format!("adding script '{}' to pak", rel))?;
    }
    Ok(())
}

/// Rewrite asset-reference attribute values in `source` using `path_map`.
///
/// Uses the semantic AST from `kag_syntax` to find every tag param that is
/// an asset reference, then performs a string replacement on the original
/// source text (avoiding full CST surgery while still being precise — each
/// replacement target is the literal value text from the span).
fn rewrite_script(
    source: &str,
    script_path: &Path,
    path_map: &HashMap<String, String>,
    assets_cfg: &AssetsConfig,
) -> String {
    let source_name = script_path.display().to_string();
    let (script, _diagnostics) = parse_script(source, &source_name);

    // Collect (byte_offset, byte_len, new_value) replacements.
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    for op in &script.ops {
        match op {
            Op::Tag(tag) => {
                collect_tag_replacements(tag, source, path_map, assets_cfg, &mut replacements);
            }
            Op::Text { parts, .. } => {
                for part in parts {
                    if let TextPart::InlineTag(tag) = part {
                        collect_tag_replacements(
                            tag,
                            source,
                            path_map,
                            assets_cfg,
                            &mut replacements,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    if replacements.is_empty() {
        return source.to_owned();
    }

    // Sort by offset descending so applying from the end preserves earlier offsets.
    replacements.sort_unstable_by(|a, b| b.0.cmp(&a.0));

    let mut out = source.to_owned();
    for (offset, len, new_val) in replacements {
        out.replace_range(offset..offset + len, &new_val);
    }
    out
}

fn collect_tag_replacements(
    tag: &kag_syntax::Tag<'_>,
    source: &str,
    path_map: &HashMap<String, String>,
    assets_cfg: &AssetsConfig,
    replacements: &mut Vec<(usize, usize, String)>,
) {
    for param in &tag.params {
        let Some(key) = param.key.as_deref() else {
            continue;
        };
        if !assets_cfg.is_asset_attr(&tag.name, key) {
            continue;
        }
        let ParamValue::Literal(ref val) = param.value else {
            continue;
        };
        let Some(pak_key) = path_map.get(val.as_ref()) else {
            continue;
        };

        // Find the literal value within the param's span in the source.
        let param_span: miette::SourceSpan = param.span;
        let p_start: usize = param_span.offset();
        let p_len: usize = param_span.len();
        let param_src = &source[p_start..p_start + p_len];

        // Locate the old value inside the param source text.
        if let Some(val_pos) = param_src.find(val.as_ref()) {
            replacements.push((p_start + val_pos, val.len(), pak_key.clone()));
        }
    }
}

// ─── Install kani-init ────────────────────────────────────────────────────────

/// Install `kani-init` from crates.io into a temporary directory and return
/// `(path_to_binary, temp_dir)`.
///
/// The caller must keep `TempDir` alive until the binary has been copied.
fn install_kani_init(target: Option<&str>) -> Result<(PathBuf, TempDir)> {
    println!("bundler: installing kani-init from crates.io...");

    let tmp = TempDir::new().context("creating temp install dir")?;
    let tmp_path = tmp.path();

    let mut cmd = Command::new("cargo");
    cmd.args(["install", "kani-init", "--root"])
        .arg(tmp_path);

    if let Some(t) = target {
        cmd.args(["--target", t]);
    }

    let status = cmd.status().context("failed to invoke cargo — is it in PATH?")?;
    if !status.success() {
        bail!("cargo install kani-init exited with status {status}");
    }

    // `cargo install --root <dir>` places binaries in `<dir>/bin/`.
    let binary_name = {
        let is_windows = target
            .map(|t| t.contains("windows"))
            .unwrap_or(cfg!(target_os = "windows"));
        if is_windows { "kani-init.exe" } else { "kani-init" }
    };
    let binary_path = tmp_path.join("bin").join(binary_name);

    if !binary_path.exists() {
        bail!(
            "expected binary at '{}' but it was not found",
            binary_path.display()
        );
    }

    Ok((binary_path, tmp))
}


// ─── Distribution zip ─────────────────────────────────────────────────────────

/// Zip every file in `dir` (non-recursively for now) into `zip_path`.
fn create_zip(dir: &Path, zip_path: &Path) -> Result<()> {
    let file = std::fs::File::create(zip_path)
        .with_context(|| format!("creating '{}'", zip_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(dir)
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");

        zip.start_file(&rel, options)
            .with_context(|| format!("adding '{rel}' to zip"))?;
        let data = std::fs::read(path)
            .with_context(|| format!("reading '{}'", path.display()))?;
        zip.write_all(&data)
            .with_context(|| format!("writing '{rel}' to zip"))?;
    }

    zip.finish().context("finalising zip")?;
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn relative_path(base: &Path, path: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(base)
        .with_context(|| format!("'{}' is not under '{}'", path.display(), base.display()))?;
    // Always use forward slashes (pak format convention).
    let s = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Ok(s)
}

fn parse_compression(s: &str) -> Compression {
    match s {
        "zstd" => Compression::Zstd { level: 3 },
        _ => Compression::None,
    }
}
