//! `kani-init` — the shipped game binary.
//!
//! The entry script path is read at runtime from a file named `init` located
//! in the same directory as the executable.  The pak file path comes from
//! the first command-line argument, falling back to `game.pak` in the current
//! directory.

fn main() -> anyhow::Result<()> {
    // Locate the `init` file next to the running binary.
    let init_path = std::env::current_exe()?
        .parent()
        .map(|p| p.join("init"))
        .unwrap_or_else(|| std::path::PathBuf::from("init"));

    let entry_script = std::fs::read_to_string(&init_path)
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|_| "scenario/first.ks".to_owned());

    let pak_path = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("game.pak"));

    kani_runtime::run_release(pak_path, &entry_script)
}
