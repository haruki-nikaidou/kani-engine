# Refactor Plan: Typed Tags & Error Splitting

## Overview

Replace stringly-typed tag dispatch across the entire engine with a macro-generated
`KnownTag` enum carrying parsed, typed attributes. Split error types by crate
boundary. Propagate typed tags through interpreter, bridge, and LSP.

---

## Step 1 — Split error types across crate boundaries

### Goal

`kag-syntax` should only contain syntax-level errors. Runtime/interpreter errors
move to `kag-interpreter`.

### Changes

**`kag-syntax/src/error.rs`** — keep only:

```rust
/// Syntax-level error (fatal, aborts parse).
pub enum SyntaxError {
    LexError { offset, src, span },
    ParseError { message, src, span },
    /// A known tag failed validation (missing required attribute, wrong type).
    InvalidTag { tag_name: String, message: String, span },
}

/// Syntax-level warning (non-fatal, collected alongside the AST).
/// Replaces the current `ParseDiagnostic`.
pub enum SyntaxWarning {
    DuplicateLabel { name: String, span },
    /// A recommended (but not required) attribute is absent.
    MissingAttribute { tag_name: String, attr: String, span },
    /// Attribute value doesn't parse to the expected type.
    BadAttributeType { tag_name: String, attr: String, expected: String, span },
}
```

**`kag-interpreter/src/error.rs`** — new file, owns runtime errors:

```rust
pub enum InterpreterError {
    /// Wraps a syntax-level error from parsing.
    Syntax(kag_syntax::SyntaxError),
    /// Rhai script evaluation failed.
    ScriptError(String),
    /// Generic runtime failure.
    RuntimeError(String),
    /// `[jump]`/`[call]` target not found.
    LabelNotFound { label: String, storage: String },
    /// `[return]` without matching `[call]`.
    CallStackUnderflow,
    /// Macro expansion error.
    MacroError(String),
    /// Tokio channel closed.
    ChannelClosed,
    /// Save/load serialization failure.
    SerializationError(String),
}
```

### Cleanup

- Remove the 6 runtime variants from `KagError` in `kag-syntax`.
- Remove the `_warning` synthetic tag hack in `lower.rs` — the diagnostic is
  already pushed to `self.errors`, so the `Op::Tag(name: "_warning")` is
  redundant.
- Update all `use crate::error::KagError` in `kag-interpreter` to use the new
  local `InterpreterError`.
- Stop re-exporting `kag_syntax::error` wholesale from `kag-interpreter`; only
  re-export types that are genuinely part of the interpreter's public API.

---

## Step 2 — Typed attribute values & `AttributeString` newtype

### Goal

Tag attributes should carry parsed types (`u64`, `f32`, `bool`, typed string)
instead of raw `Option<ParamValue<'src>>`. Attributes containing `&expr` or
`%key` (which can't be parsed at syntax time) are represented as a
`MaybeResolved` enum.

### New types in `kag-syntax/src/tag_defs/mod.rs`

```rust
/// A string attribute value — newtype to distinguish from arbitrary strings.
/// Wraps the raw text of a `storage=`, `target=`, `exp=`, etc. attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeString<'src>(pub Cow<'src, str>);

/// An attribute value that may or may not be statically known.
///
/// `Literal(T)` — the value was a plain string and parsed successfully.
/// `Dynamic(ParamValue)` — the value is an `&expr` or `%key` that can only
/// be resolved at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum MaybeResolved<'src, T> {
    Literal(T),
    Dynamic(ParamValue<'src>),
}
```

### Attribute field types by kind

| KAG type | Rust field type |
|----------|----------------|
| `storage=`, `target=`, `exp=`, `name=`, `method=`, `face=`, … | `Option<MaybeResolved<'src, AttributeString<'src>>>` |
| `time=`, `speed=`, `fadetime=`, `buf=` | `Option<MaybeResolved<'src, u64>>` |
| `x=`, `y=`, `opacity=`, `volume=`, `hmax=`, `vmax=` | `Option<MaybeResolved<'src, f32>>` |
| `visible=`, `loop=`, `bold=`, `italic=`, `join=` | `Option<MaybeResolved<'src, bool>>` |

### Parsing

During `from_tag()`, each attribute is parsed from `ParamValue`:

```rust
fn parse_attr<T: FromStr>(pv: &ParamValue<'src>) -> MaybeResolved<'src, T> {
    match pv {
        ParamValue::Literal(s) => match s.parse::<T>() {
            Ok(v) => MaybeResolved::Literal(v),
            Err(_) => /* emit SyntaxWarning::BadAttributeType, use default */
        },
        other @ (ParamValue::Entity(_) | ParamValue::MacroParam { .. }) => {
            MaybeResolved::Dynamic(other.clone())
        }
        ParamValue::MacroSplat => /* skip */
    }
}
```

---

## Step 3 — Merge validation into `from_tag()` via `TryFrom`-style API

### Goal

Parsing + validation happen in one pass. `from_tag()` both constructs the
`KnownTag` and emits diagnostics for missing/bad attributes.

### New signature

```rust
impl<'src> KnownTag<'src> {
    /// Parse and validate a raw `Tag` into a typed `KnownTag`.
    ///
    /// Always returns a `KnownTag` (using `Extension` for unrecognised names).
    /// Diagnostics for missing required/recommended attributes or type errors
    /// are appended to `diagnostics`.
    pub fn from_tag(
        tag: &Tag<'src>,
        diagnostics: &mut Vec<SyntaxWarning>,
    ) -> Self { ... }
}
```

### Validation rules encoded per-attribute

- **`Required<T>`** — absent → push `SyntaxError::InvalidTag`, field is `None`.
- **`Recommended<T>`** — absent → push `SyntaxWarning::MissingAttribute`, field
  is `None`.
- **`Optional<T>`** — absent → field is `None`, no diagnostic.

These annotations are metadata consumed by the `define_tags!` macro (step 5).

### Delete

- `validate.rs`'s standalone `validate_tag()`, `require()`, `recommend()`,
  `recommend_any_of()` — all absorbed into the macro-generated `from_tag()`.

---

## Step 4 — Add `Extension` variant to `KnownTag`

### Goal

`from_tag()` never returns `None` — every tag becomes a `KnownTag`. Unknown
tags are captured as `Extension`, eliminating the need for a separate stringly-
typed passthrough.

### Change

```rust
pub enum KnownTag<'src> {
    // ...all existing variants...

    /// A tag not recognised by the engine. Carries the raw name and params
    /// so game-specific / plugin code can handle it.
    Extension {
        name: Cow<'src, str>,
        params: Vec<Param<'src>>,
    },
}
```

`TagName` gets a corresponding treatment — either add `TagName::Extension` or
have `tag_name()` return `Option<TagName>` (returning `None` for extensions).

---

## Step 5 — `define_tags!` declarative macro

### Goal

Single source of truth for all tag metadata. One edit to add a tag, with
compile-time guarantee of completeness.

### Input syntax

```rust
define_tags! {
    // variant    "kag_name"    [alias "other_name"]   { attributes }

    // ── Control flow ──
    If("if") {
        exp: Required<AttributeString>,
    },
    Elsif("elsif") {
        exp: Required<AttributeString>,
    },
    Else("else") {},
    Endif("endif") {},
    Ignore("ignore") {
        exp: Required<AttributeString>,
    },
    Endignore("endignore") {},

    // ── Navigation ──
    Jump("jump") {
        storage: RecommendedAnyOf["storage","target"]<AttributeString>,
        target:  RecommendedAnyOf["storage","target"]<AttributeString>,
    },
    Call("call") {
        storage: RecommendedAnyOf["storage","target"]<AttributeString>,
        target:  RecommendedAnyOf["storage","target"]<AttributeString>,
    },
    Return("return") {},

    // ── Audio ──
    Bgm("bgm") {
        storage:  Required<AttributeString>,
        r#loop:   Optional<bool>,
        volume:   Optional<f32>,
        fadetime: Optional<u64>,
    },
    Se("se", alias "playSe") {
        storage: Required<AttributeString>,
        buf:     Optional<u32>,
        volume:  Optional<f32>,
        r#loop:  Optional<bool>,
    },
    Vo("vo", alias "voice") {
        storage: Required<AttributeString>,
        buf:     Optional<u32>,
    },

    // ── Image ──
    Bg("bg") {
        storage: Required<AttributeString>,
        time:    Optional<u64>,
        method:  Optional<AttributeString>,
    },
    Image("image") {
        storage: Required<AttributeString>,
        layer:   Optional<AttributeString>,
        x:       Optional<f32>,
        y:       Optional<f32>,
        visible: Optional<bool>,
    },

    // ... (all other tags follow the same pattern)
}
```

### Generated output

The macro expands to:

1. **`pub enum TagName { If, Elsif, ..., PlaySe, Voice }`**
   - `TagName::from_name(&str) -> Option<TagName>` — handles aliases
   - `TagName::as_str(self) -> &'static str`
   - `TagName::all() -> impl Iterator<Item = TagName>` — for LSP
   - `TagName::canonical(self) -> TagName` — `PlaySe → Se`, `Voice → Vo`

2. **`pub enum KnownTag<'src> { If { exp: Option<MaybeResolved<..>> }, ..., Extension { name, params } }`**
   - `KnownTag::from_tag(&Tag, &mut Vec<SyntaxWarning>) -> Self`
   - `KnownTag::tag_name(&self) -> Option<TagName>` (`None` for `Extension`)

3. **Validation logic** — baked into `from_tag()`:
   - `Required` → error diagnostic if absent
   - `Recommended` → warning diagnostic if absent
   - `RecommendedAnyOf` → warning if *none* of the group is present
   - Type parsing errors → `BadAttributeType` warning

4. **Metadata accessors** (for LSP):
   - `TagName::param_names(self) -> &'static [&'static str]`
   - `TagName::doc_summary(self) -> &'static str` (via doc attribute on the
     macro input)

### File changes

- **Delete** `kag-syntax/src/tag_defs/names.rs` — absorbed into macro output.
- **Delete** most of `kag-syntax/src/tag_defs/validate.rs` — absorbed into
  macro output. Keep the file only if there are complex validation rules that
  can't be expressed as `Required`/`Recommended` annotations.
- **Rewrite** `kag-syntax/src/tag_defs/mod.rs` — contains only the macro
  invocation, `AttributeString`, `MaybeResolved`, and the `Extension` variant
  logic.

---

## Step 6 — Propagate `KnownTag` through interpreter, bridge, and LSP

### 6a. Interpreter (`kag-interpreter/src/runtime/executor.rs`)

**Before:**
```rust
const TAG_BG: &str = "bg";
match name {
    TAG_BG => {
        let storage = resolved_str(ctx, tag, "storage");
        Ok(vec![build_generic_event(ctx, tag)])
    }
}
```

**After:**
```rust
match KnownTag::from_tag(tag, &mut diags) {
    KnownTag::Bg { storage, time, method } => {
        let storage = resolve(ctx, &storage);
        Ok(vec![KagEvent::Tag(ResolvedTag::Bg { storage, time, method })])
    }
    KnownTag::Extension { name, params } => {
        Ok(vec![KagEvent::Tag(ResolvedTag::Extension { name, params })])
    }
}
```

**Delete:**
- All 40+ `const TAG_*: &str` constants.
- `build_generic_event()`, `build_resolved_params()` — replaced by typed
  construction.
- `resolved_str()`, `resolve_u64()` — replaced by a generic
  `resolve<T>(ctx, &MaybeResolved<T>) -> T` helper.

### 6b. Event channel (`kag-interpreter/src/events.rs`)

**Replace:**
```rust
// Old
Tag { name: String, params: Vec<(String, String)> },
WaitForCompletion { tag: String, params: Vec<(String, String)> },
```

**With:**
```rust
// New
Tag(ResolvedTag),
WaitForCompletion { which: TagName, canskip: Option<bool>, buf: Option<u32> },
```

Where `ResolvedTag` is a parallel enum with all `MaybeResolved<T>` collapsed
to concrete `Option<T>`:

```rust
/// A `KnownTag` with all dynamic values resolved to concrete types.
/// Lives in `kag-interpreter` because resolution requires `RuntimeContext`.
pub enum ResolvedTag {
    Bg { storage: String, time: Option<u64>, method: Option<String> },
    Se { storage: String, buf: Option<u32>, volume: Option<f32>, looping: bool },
    Extension { name: String, params: Vec<(String, String)> },
    // ...
}
```

Alternatively, make `KnownTag` generic over a value-wrapper type so both
`kag-syntax` and `kag-interpreter` share the same enum shape:

```rust
// In kag-syntax (unresolved):
type SyntaxTag<'src> = KnownTag<'src, MaybeResolved<'src>>;

// In kag-interpreter (resolved):
type ResolvedTag = KnownTag<'static, Resolved>;
```

The `define_tags!` macro can generate both instantiations.

### 6c. Bridge tag handlers (`kani-runtime/src/systems/tags/`)

**Replace `EvTagRouted`:**
```rust
// Old
pub struct EvTagRouted { pub name: String, pub params: Vec<(String, String)> }

// New
pub struct EvTagRouted(pub ResolvedTag);
```

**Simplify handlers** — e.g. `image.rs`:

```rust
// Old
match tag.name.as_str() {
    "bg" => {
        if let Some(storage) = param(p, "storage") {
            ev_bg.write(EvSetBackground { storage, time: param_u64(p, "time"), ... });
        }
    }
}

// New
match &tag.0 {
    ResolvedTag::Bg { storage, time, method } => {
        ev_bg.write(EvSetBackground {
            storage: storage.clone(),
            time: *time,
            method: method.clone(),
        });
    }
}
```

**Delete:**
- `param()`, `param_f32()`, `param_bool()`, `param_u32()`, `param_u64()`
  helpers in `tags/mod.rs`.
- `is_known_tag()` — replaced by `ResolvedTag` variant matching.
- `EvUnknownTag` — replaced by matching `ResolvedTag::Extension`.

### 6d. LSP (`kag-lsp/src/analysis/`)

**`completion.rs`** — replace hand-maintained list:
```rust
// Old
const BUILTIN_TAG_NAMES: &[&str] = &["r", "p", "l", "jump", ...];

// New
let builtin_names = TagName::all().map(|t| t.as_str());
```

Add parameter-name completions:
```rust
// When cursor is inside a tag, offer its known param names
if let Some(tag_name) = TagName::from_name(current_tag) {
    for &param in tag_name.param_names() {
        items.push(CompletionItem { label: param.to_owned(), ... });
    }
}
```

**`hover.rs`** — replace hand-maintained docs:
```rust
// Old
const BUILTIN_TAGS: &[(&str, &str)] = &[("r", "Insert a line break..."), ...];

// New — doc strings from the macro metadata
if let Some(tag_name) = TagName::from_name(text) {
    let desc = tag_name.doc_summary();
    let params = tag_name.param_names().join(", ");
    format!("**tag** `{text}`\n\n{desc}\n\nParams: {params}")
}
```

---

## Migration order

```
Step 1  (error split)          — compiles independently
  ↓
Step 2  (AttributeString,      — compiles independently, KnownTag fields change
         MaybeResolved)           but no consumers yet
  ↓
Step 4  (Extension variant)    — compiles with step 2
  ↓
Step 5  (define_tags! macro)   — replaces manual enum/impl code from steps 2+4
  ↓
Step 3  (validation in         — absorbed into macro, delete validate.rs
         from_tag)
  ↓
Step 6a (interpreter)          — migrate executor.rs to use KnownTag
  ↓
Step 6b (event channel)        — replace KagEvent::Tag with ResolvedTag
  ↓
Step 6c (bridge handlers)      — simplify tag systems
  ↓
Step 6d (LSP)                  — use TagName metadata for completions/hover
```

Each step should result in a compilable, testable state. Existing tests in
`kag-syntax/src/tag_defs/mod.rs` (round-trip, validation) and
`kag-interpreter/src/runtime/executor.rs` (execution) must pass at every step.

