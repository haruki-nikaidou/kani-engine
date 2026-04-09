---
name: kani-pak crate
overview: "Add a new `kani-pak` crate that defines a custom binary package format for kani-engine: fast reading via memmap2 + zstd, an optional Bevy `AssetReader` impl, and a write API that the bundler drives entirely — the crate itself is format-only with no bundling strategy."
todos:
  - id: workspace
    content: Add kani-pak to workspace Cargo.toml members and workspace.dependencies (memmap2, zstd, bytemuck, rustc-hash, bevy_asset optional, futures-lite optional)
    status: completed
  - id: scaffold
    content: Create kani-pak/Cargo.toml and src/lib.rs skeleton with feature flags (bevy, write)
    status: completed
  - id: format
    content: "Implement src/format.rs: Header and IndexEntry as bytemuck::Pod structs, magic/version constants"
    status: completed
  - id: error
    content: "Implement src/error.rs: PakError variants (NotFound, BadMagic, VersionMismatch, DecompressError, Io)"
    status: completed
  - id: reader
    content: "Implement src/reader.rs: PakReader with memmap2 open, binary-search lookup by FxHash, read() / read_raw() / contains() / paths()"
    status: completed
  - id: writer
    content: "Implement src/writer.rs (write feature): PakWriter with add(), finish(), write_to(); sorts entries by path_hash before serialising"
    status: completed
  - id: asset-reader
    content: "Implement src/asset_reader.rs (bevy feature): PakAssetReader wrapping Arc<PakReader>, full AssetReader impl including read_directory and is_directory"
    status: completed
isProject: false
---

# kani-pak Crate Design

## Crate responsibilities

- Define the `.pak` binary format (header, sorted index, string table, data blocks).
- Provide a zero-copy reader (`PakReader`) using `memmap2`; decompress on-demand with `zstd`.
- Expose an **optional** Bevy `AssetReader` impl behind the `bevy` feature (for the frontend/bridge crate).
- Expose an **optional** `PakWriter` behind the `write` feature (for the bundler). The bundler calls `writer.add("path", bytes, compression)` and decides what goes in — `kani-pak` only serialises the format.

## Feature flags

```
default = []
bevy    = ["dep:bevy_asset", "dep:bevy_utils"]
write   = []          # no extra dep; just enables PakWriter
```

- Bevy frontend crate: enables `bevy`.
- Bundler crate: enables `write`. No `bevy` dependency needed.

## Binary format

```
[Header – 40 bytes]
  magic:           [u8; 8]  = b"KANIPAK\0"
  version:         u32 LE   = 1
  flags:           u32 LE   (reserved)
  entry_count:     u32 LE
  _pad:            u32
  index_offset:    u64 LE   → start of Index Table
  strtab_offset:   u64 LE   → start of String Table
  data_offset:     u64 LE   → start of Data Section

[Index Table] – entry_count × IndexEntry (32 bytes each, sorted by path_hash)
  path_hash:       u64 LE   (FxHash of the UTF-8 path)
  path_offset:     u32 LE   (byte offset into String Table)
  path_len:        u16 LE
  compression:     u8       (0 = none, 1 = zstd)
  _pad:            u8
  data_offset:     u64 LE   (relative to data_offset in header)
  compressed_size: u32 LE
  uncompressed_size: u32 LE

[String Table] – packed UTF-8 paths (no null terminators needed; lengths in index)

[Data Section] – raw or zstd-compressed blobs
```

Index is sorted by `path_hash` → binary search gives O(log n) lookup purely on the mmap'd slice.

## Source layout

```
kani-pak/
  Cargo.toml
  src/
    lib.rs          — public re-exports, doc
    format.rs       — Header, IndexEntry (bytemuck Pod structs), constants
    reader.rs       — PakReader (memmap2 Mmap, binary-search lookup)
    error.rs        — PakError (thiserror)
    asset_reader.rs — bevy AssetReader impl  [cfg(feature="bevy")]
    writer.rs       — PakWriter              [cfg(feature="write")]
```

## Key types

```rust
// reader.rs
pub struct PakReader { mmap: Mmap, entries: &[IndexEntry], ... }

impl PakReader {
    pub fn open(path: &Path) -> Result<Self, PakError>
    /// Returns decompressed bytes for `asset_path`.
    pub fn read(&self, asset_path: &str) -> Result<Vec<u8>, PakError>
    /// Zero-copy slice for uncompressed entries.
    pub fn read_raw(&self, asset_path: &str) -> Result<&[u8], PakError>
    pub fn contains(&self, asset_path: &str) -> bool
    pub fn paths(&self) -> impl Iterator<Item = &str>
}

// writer.rs  (write feature)
pub struct PakWriter { entries: Vec<...>, string_table: Vec<u8>, data: Vec<u8> }

impl PakWriter {
    pub fn new() -> Self
    pub fn add(&mut self, path: &str, data: &[u8], compression: Compression)
    pub fn finish(self) -> Vec<u8>            // in-memory pak
    pub fn write_to(self, w: impl Write) -> Result<(), PakError>
}

pub enum Compression { None, Zstd { level: i32 } }
```

## Bevy AssetReader

The `bevy` feature implements `[AssetReader](https://docs.rs/bevy_asset/latest/bevy_asset/io/trait.AssetReader.html)`:

```rust
// asset_reader.rs
pub struct PakAssetReader(Arc<PakReader>);

impl AssetReader for PakAssetReader {
    async fn read(&self, path: &Path) -> Result<impl AsyncRead + ..., AssetReaderError>
    async fn read_meta(...) -> ...  // look for "<path>.meta"
    async fn read_directory(...) -> ...
    async fn is_directory(...) -> ...
}
```

`read` resolves to: lookup in pak → decompress if needed → wrap `Vec<u8>` in `std::io::Cursor` → adapt to `AsyncRead` via `futures_lite`.

The bevy frontend/bridge registers it with:

```rust
app.register_asset_source(
    "pak",
    AssetSource::build().with_reader(|| Box::new(PakAssetReader::open("game.pak").unwrap()))
);
```

## Workspace changes

- Add `kani-pak` to `workspace.members` in root `Cargo.toml`.
- Add to `[workspace.dependencies]`: `memmap2`, `zstd`, `bytemuck` (for Pod index casting), `rustc-hash` (FxHash), `bevy_asset` (optional), `futures-lite` (for `AsyncRead` bridge under `bevy` feature).
- `kani-pak/Cargo.toml` inherits workspace deps; bevy and futures-lite are optional.

## What `kani-pak` deliberately does NOT contain

- Which files to bundle (bundler's concern).
- Compression level choice at the format level — `Compression::Zstd { level }` is bundler-supplied.
- Hot-reload, watch, or VFS layering (bridge/frontend concern).
- Any async executor or Tokio dependency (async in `AssetReader` is driven by Bevy's executor).

