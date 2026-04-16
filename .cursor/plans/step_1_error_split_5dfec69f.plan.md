---
name: "Step 1: Error Split"
overview: Split `kag-syntax`'s monolithic `KagError` into a syntax-only `SyntaxError` (in `kag-syntax`) and a new `InterpreterError` (in `kag-interpreter`). Rename `ParseDiagnostic` to `SyntaxWarning`. Remove the `_warning` synthetic tag hack from the lowerer and executor.
todos:
  - id: rewrite-syntax-error
    content: "Rewrite kag-syntax/src/error.rs: rename KagError→SyntaxError (syntax variants only), rename ParseDiagnostic→SyntaxWarning, keep Severity"
    status: completed
  - id: update-syntax-lib
    content: "Update kag-syntax/src/lib.rs exports: SyntaxError, SyntaxWarning, Severity"
    status: completed
  - id: fix-lower-rs
    content: Remove _warning Op::Tag emission from lower.rs duplicate-label branch; update ParseDiagnostic references to SyntaxWarning
    status: completed
  - id: create-interpreter-error
    content: Create kag-interpreter/src/error.rs with InterpreterError owning all 7 runtime variants
    status: completed
  - id: update-interpreter-lib
    content: "Update kag-interpreter/src/lib.rs: add pub mod error; replace kag_syntax::error re-export; export InterpreterError"
    status: completed
  - id: update-runtime-files
    content: "Update all 4 kag-interpreter/src/runtime/ files: swap KagError→InterpreterError imports and usages; delete TAG_WARNING handler from executor.rs"
    status: completed
  - id: update-bridge
    content: Update kani-runtime/src/bridge.rs Severity path
    status: completed
  - id: update-lsp-convert
    content: "Update kag-lsp/src/convert.rs: ParseDiagnostic→SyntaxWarning"
    status: completed
  - id: verify-compile
    content: Run cargo check --workspace and cargo test --workspace; fix any remaining type errors
    status: completed
isProject: false
---

# Step 1: Split Error Types Across Crate Boundaries

## Current state

- [`kag-syntax/src/error.rs`](kag-syntax/src/error.rs) defines `KagError` with **10 variants** — 3 syntax-level (`LexError`, `ParseError`, `UndefinedTag`) and 7 runtime-level (`ScriptError`, `RuntimeError`, `LabelNotFound`, `CallStackUnderflow`, `MacroError`, `ChannelClosed`, `SerializationError`).
- `ParseDiagnostic` (with `Severity`) is a non-fatal message struct, also in `kag-syntax`.
- `kag-interpreter/src/lib.rs` re-exports the whole `kag_syntax::error` module: `pub use kag_syntax::error;`, then `pub use error::KagError`.
- All 4 `kag-interpreter` runtime files import `KagError` as `use crate::error::KagError`.
- `kani-runtime/src/bridge.rs` uses `kag_interpreter::error::Severity`.
- `kag-lsp/src/convert.rs` uses `kag_syntax::error::{ParseDiagnostic, Severity}`.
- [`kag-syntax/src/lower.rs`](kag-syntax/src/lower.rs) emits a synthetic `Op::Tag("_warning", ...)` for duplicate labels (lines 274–286), even though the warning is already pushed to `self.errors`. [`kag-interpreter/src/runtime/executor.rs`](kag-interpreter/src/runtime/executor.rs) has a matching `TAG_WARNING` handler (line 40, 474–478).

## Changes

### 1. `kag-syntax/src/error.rs` — rewrite

- Rename `KagError` → `SyntaxError`. Keep only the 3 syntax variants:
  - `LexError { offset, src, span }` (unchanged)
  - `ParseError { message, src, span }` (unchanged)
  - `InvalidTag { tag_name: String, message: String, span }` (replaces `UndefinedTag`, which will be relevant from Step 4 onward)
- Rename `ParseDiagnostic` → `SyntaxWarning`. Keep the same struct shape (`message: String`, `span: SourceSpan`, `severity: Severity`) for now — typed variants (`DuplicateLabel`, `MissingAttribute`, `BadAttributeType`) are introduced in Step 3.
- Keep `Severity` as-is (still needed by bridge and LSP).
- Remove all 7 runtime variants from the enum. Remove `KagError::parse(...)` constructor.
- Keep `miette::Diagnostic` + `thiserror::Error` derives on `SyntaxError`.

### 2. `kag-syntax/src/lib.rs` — update exports

```rust
// Before
pub use error::{KagError, ParseDiagnostic, Severity};

// After
pub use error::{SyntaxError, SyntaxWarning, Severity};
```

### 3. `kag-syntax/src/lower.rs` — remove `_warning` synthetic tag

In the duplicate-label branch (lines 274–286), remove the `self.emit(Op::Tag { name: "_warning", ... })` call. The `self.push_warning(...)` that precedes it already records the diagnostic. The remaining code just emits nothing and falls through.

Also update the types in `push_error` / `push_warning` to use `SyntaxWarning` instead of `ParseDiagnostic`.

### 4. `kag-interpreter/src/error.rs` — new file

Create this file with `InterpreterError`, owning all the runtime variants:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InterpreterError {
    #[error(transparent)]
    Syntax(#[from] kag_syntax::SyntaxError),
    #[error("script evaluation error: {0}")]
    ScriptError(String),
    #[error("runtime error: {0}")]
    RuntimeError(String),
    #[error("label not found: '{label}' in '{storage}'")]
    LabelNotFound { label: String, storage: String },
    #[error("call stack underflow: [return] without matching [call]")]
    CallStackUnderflow,
    #[error("macro parameter error: {0}")]
    MacroError(String),
    #[error("channel closed unexpectedly")]
    ChannelClosed,
    #[error("serialization error: {0}")]
    SerializationError(String),
}
```

### 5. `kag-interpreter/src/lib.rs` — own the error module

- Add `pub mod error;` for the new local module.
- Replace `pub use kag_syntax::error;` with only what's genuinely public API:
  ```rust
  pub use kag_syntax::error::SyntaxWarning;   // returned by parse_script
  pub use kag_syntax::error::Severity;         // used by kani-runtime bridge
  pub use error::InterpreterError;
  ```
- Replace `pub use error::KagError;` and `pub use error::ParseDiagnostic;` with the new names.

### 6. `kag-interpreter/src/runtime/` — update all 4 files

| File | Change |
|------|--------|
| `mod.rs` | `use crate::error::{InterpreterError, SyntaxWarning};`; change all `-> Result<..., KagError>` to `InterpreterError` |
| `executor.rs` | `use crate::error::InterpreterError;`; replace all `KagError::*` constructors; delete `TAG_WARNING` constant and its match arm |
| `context.rs` | `use crate::error::InterpreterError;`; update `Result` return types |
| `script_engine.rs` | `use crate::error::InterpreterError;`; replace `KagError::ScriptError`, `KagError::SerializationError` |

Specific replacements in `executor.rs`:
- `KagError::CallStackUnderflow` → `InterpreterError::CallStackUnderflow`
- `KagError::MacroError(...)` → `InterpreterError::MacroError(...)`
- Remove `const TAG_WARNING: &str = "_warning";` and the `TAG_WARNING => { ... }` match arm

### 7. `kani-runtime/src/bridge.rs` — update Severity path

```rust
// Before
kag_interpreter::error::Severity::Error => { ... }
kag_interpreter::error::Severity::Warning => { ... }

// After — Severity is still re-exported at kag_interpreter::Severity
kag_interpreter::Severity::Error => { ... }
kag_interpreter::Severity::Warning => { ... }
```

### 8. `kag-lsp/src/convert.rs` — update import

```rust
// Before
use kag_syntax::error::{ParseDiagnostic, Severity};

// After
use kag_syntax::error::{SyntaxWarning, Severity};
```

Update any local usage of `ParseDiagnostic` → `SyntaxWarning` in that file.

## File change summary

- Rewrite: `kag-syntax/src/error.rs`, `kag-interpreter/src/lib.rs`
- New file: `kag-interpreter/src/error.rs`
- Edit: `kag-syntax/src/lib.rs`, `kag-syntax/src/lower.rs`
- Edit (imports + type names): `kag-interpreter/src/runtime/mod.rs`, `executor.rs`, `context.rs`, `script_engine.rs`
- Edit (path update): `kani-runtime/src/bridge.rs`, `kag-lsp/src/convert.rs`

## Validation

Run `cargo check --workspace` after each sub-step. The existing test suite in `kag-syntax` and `kag-interpreter` must pass with `cargo test --workspace`.
