//! `kani-init` — the shipped game binary.
//!
//! The entry script is baked in at compile time by `kani-bundler` via the
//! `KANI_ENTRY_SCRIPT` environment variable.  The pak file path comes from
//! the first command-line argument, falling back to `game.pak` in the current
//! directory.

fn main() -> anyhow::Result<()> {
    // Baked in at compile time by kani-bundler:
    //   KANI_ENTRY_SCRIPT="scenario/first.ks" cargo build -p kani-init ...
    // Falls back to the conventional default when not set (e.g. during `cargo check`).
    const ENTRY_SCRIPT: &str = match option_env!("KANI_ENTRY_SCRIPT") {
        Some(s) => s,
        None => "scenario/first.ks",
    };

    let pak_path = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("game.pak"));

    kani_runtime::run_release(pak_path, ENTRY_SCRIPT)
}
