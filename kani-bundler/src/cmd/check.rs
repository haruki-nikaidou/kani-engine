//! `kani-bundler check` — validate scripts and asset references.
//!
//! Reports:
//! - Syntax errors in every `.ks` file under `data/scenario/`
//! - Asset references (via configured `asset_attrs`) that point to missing files

use std::path::Path;

use anyhow::Result;
use kag_syntax::{Op, TextPart, parse_script};
use walkdir::WalkDir;

use crate::config::{AssetsConfig, asset_base, load_config};

pub fn run(project_dir: &Path) -> Result<()> {
    let cfg = load_config(project_dir)?;
    let base = asset_base(project_dir, &cfg);
    let scenario_dir = base.join("scenario");

    let mut error_count: usize = 0;

    for entry in WalkDir::new(&scenario_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "ks"))
    {
        let path = entry.path();
        let source_name = path.display().to_string();

        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: cannot read '{}': {e}", path.display());
                error_count += 1;
                continue;
            }
        };

        let (script, diagnostics) = parse_script(&text, &source_name);

        // Report syntax diagnostics.
        for diag in &diagnostics {
            eprintln!("{diag:?}");
            error_count += 1;
        }

        // Walk ops for asset references.
        error_count += check_asset_refs(&script, path, &base, &cfg.assets);
    }

    if error_count == 0 {
        println!("check: no issues found.");
    } else {
        eprintln!("check: {error_count} issue(s) found.");
        std::process::exit(1);
    }

    Ok(())
}

fn check_asset_refs(
    script: &kag_syntax::Script<'_>,
    script_path: &Path,
    asset_base: &Path,
    assets_cfg: &AssetsConfig,
) -> usize {
    let mut count = 0;

    for op in &script.ops {
        match op {
            Op::Tag(tag) => {
                count += check_tag(tag, script_path, asset_base, assets_cfg);
            }
            Op::Text { parts, .. } => {
                for part in parts {
                    if let TextPart::InlineTag(tag) = part {
                        count += check_tag(tag, script_path, asset_base, assets_cfg);
                    }
                }
            }
            _ => {}
        }
    }

    count
}

fn check_tag(
    tag: &kag_syntax::Tag<'_>,
    script_path: &Path,
    asset_base: &Path,
    assets_cfg: &AssetsConfig,
) -> usize {
    let mut count = 0;
    for param in &tag.params {
        let Some(key) = param.key.as_deref() else {
            continue;
        };
        if !assets_cfg.is_asset_attr(&tag.name, key) {
            continue;
        }
        let kag_syntax::ParamValue::Literal(ref val) = param.value else {
            continue;
        };
        let asset_path = asset_base.join(val.as_ref());
        if !asset_path.exists() {
            eprintln!(
                "missing asset: '{}' referenced in '{}' (tag [{}], attr {}=)",
                asset_path.display(),
                script_path.display(),
                tag.name,
                key,
            );
            count += 1;
        }
    }
    count
}
