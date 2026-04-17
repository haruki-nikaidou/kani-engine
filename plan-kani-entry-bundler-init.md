# Plan: Implement `kani-entry`, `kani-bundler`, and `kani-init`

## Overview

Three new crates that complete the engine's distribution pipeline:
- **`kani-entry`** abstracts dev/release execution modes
- **`kani-bundler`** provides a CLI project manager
- **`kani-init`** is the thin shipped binary

---

## Step 1: Create `kani-entry` (library crate)

**Path**: `kani-entry/`

### Cargo.toml

- Add a `develop` feature flag.
- Depend on `kani-runtime`, `kani-pak` (with `bevy` feature), and `bevy`.
- When `develop` is enabled, additionally pull in `notify` (file watcher) for hot-reload.

### lib.rs

Expose two public functions:

- **`run_develop(base_path: PathBuf, entry_script: &str)`**
  - Guarded by `#[cfg(feature = "develop")]`
  - Constructs `AssetBackend::FileSystem`
  - Configures Bevy with dev features
  - Sets up file-watching/hot-reload via `bevy_asset`'s built-in file watcher
  - Builds and runs the `App` with `KaniRuntimePlugin`

- **`run_release(pak_path: PathBuf, entry_script: &str)`**
  - Constructs `AssetBackend::from_pak(pak_path)`
  - Builds a minimal `App` (no dev tools)
  - Runs it with `KaniRuntimePlugin`

### Workspace

- Register the crate in workspace `Cargo.toml` `members`.

---

## Step 2: Create `kani-init` (binary crate)

**Path**: `kani-init/`

### Cargo.toml

- Depend on `kani-entry` **without** the `develop` feature.

### main.rs

- Call `kani_entry::run_release()` with a compile-time-embedded pak path and entry script.
- These are injected via `env!()` or constants set by `kani-bundler` at compile time using `--cfg` / env vars.
- Fall back to CLI args or a convention like `game.pak` next to the binary.

---

## Step 3: Define `kani.toml` schema

Design a TOML config struct (parsed with `serde` + `toml`) containing:

```toml
[project]
name = "my-game"
version = "0.1.0"
author = "..."

[entry]
start = "scenario/first.ks"

[assets]
base = "data"
include = ["**/*"]
exclude = []

[build]
targets = ["x86_64-unknown-linux-gnu"]
output = "dist"
compression = "default"
```

---

## Step 4: Create `kani-bundler` (binary crate)

**Path**: `kani-bundler/`

### Cargo.toml

- Depend on `kani-entry` with `develop` feature
- Depend on `kani-pak` with `write` feature
- Depend on `kag-syntax`, `kag-interpreter`
- Depend on `toml`, `clap`, `blake3`, `walkdir`

### CLI subcommands (via `clap`)

#### `new <name>`
- Scaffold a new project directory with:
  - A minimal `kani.toml`
  - A `scenario/first.ks` starter script
  - Placeholder asset folders (`data/bgimage/`, `data/fgimage/`, `data/bgm/`, etc.)

#### `run`
- Parse `kani.toml`
- Call `kani_entry::run_develop()` with the configured base dir and entry script

#### `check`
- Parse all `.ks` scripts with `kag_syntax`, collect diagnostics
- Resolve all asset references (e.g. `storage=` attributes in tags) against the filesystem
- Report missing assets and script errors

#### `fmt`
- Walk `.ks` files
- Parse with `kag_syntax::parser::parse_script`
- Re-emit formatted output (requires a formatter pass over the CST)
- **Note**: This is significant effort — consider deferring to a later milestone

#### `bundle`
1. Walk the asset directory
2. Rename each file to its `blake3` hash
3. Rewrite asset references in scripts to hashed names
4. Pack everything into a `.pak` via `PakWriter`
5. Compile `kani-init` for the target platform:
   - Invoke `cargo build --release -p kani-init` with env vars for pak path/entry script
   - e.g. `KANI_PAK_PATH` and `KANI_ENTRY_SCRIPT`

---

## Step 5: Update workspace `Cargo.toml`

- Add `kani-entry`, `kani-bundler`, `kani-init` to `members`.
- Add new workspace dependencies:
  - `clap` (CLI framework)
  - `toml` (config parsing)
  - `blake3` (asset hashing)
  - `walkdir` (directory traversal)
  - `notify` (file watching, if used for hot-reload outside Bevy's watcher)

---

## Step 6: Wire asset hashing into the bundle pipeline

In `kani-bundler`'s `bundle` command:

1. After hashing and renaming assets, generate a mapping file (original path → hashed name).
2. Rewrite `storage=` params in parsed KAG scripts using `kag-syntax` AST.
3. Serialize the rewritten scripts into the `.pak`.
4. This ensures `kani-init` can look up assets by their hashed names transparently.

---

## Considerations

### Hot-reload strategy
- Bevy's `AssetServer` has built-in file watching.
- `run_develop` should rely on that for binary assets.
- Add `notify` only for `.ks` script-level reload (scripts aren't Bevy assets).

### Cross-compilation in `bundle`
- Compiling `kani-init` for other platforms requires cross-compilation toolchains.
- Initially support only the host target.
- Add `cross` integration later.

### Formatter scope
- A full KAG formatter is significant effort.
- Defer `fmt` to a later milestone.
- Focus on `run`, `check`, and `bundle` first.

---

## Implementation Order

1. `kani-entry` — foundation, needed by both other crates
2. `kani-init` — simple, validates `kani-entry`'s `run_release` API
3. `kani.toml` schema — needed before `kani-bundler`
4. `kani-bundler` subcommands in order:
   1. `new` — easiest, scaffolding only
   2. `run` — thin wrapper on `run_develop`
   3. `check` — script/asset validation
   4. `bundle` — heaviest, full pipeline
   5. `fmt` — deferred

