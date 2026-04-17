# Implementation Plan — Missing TyranoScript Features

This document covers the implementation of 11 feature groups currently missing from the Kani engine compared to TyranoScript. Each section lists the new tags, the crate-level changes required, and the implementation order within the feature.

> **Notation**: `[tag_defs]` = `kag-syntax/src/tag_defs/mod.rs`, `[events]` = `kag-interpreter/src/events.rs`, `[executor]` = `kag-interpreter/src/runtime/executor.rs`, `[snapshot]` = `kag-interpreter/src/snapshot.rs`, `[rt-events]` = `kani-runtime/src/events.rs`, `[dispatch]` = `kani-runtime/src/systems/tags.rs`.

---

## 1. Save / Load System

**Tags**: `autosave`, `autoload`, `savesnap`, `save_img`, `commit`, `checkpoint`, `clear_checkpoint`, `rollback`, `clearfix`, `sleepgame`, `awakegame`, `breakgame`

The current engine already has `InterpreterSnapshot` and `HostEvent::TakeSnapshot` / `KagEvent::Snapshot`. This feature extends that foundation with script-level save/load control.

### 1a. `autosave` / `autoload`

Trigger a save or load without user interaction.

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Autosave("autosave")` and `Autoload("autoload")` tag definitions (no required attrs). |
| 2 | `kag-interpreter` | `[events]` | Add `KagEvent::AutoSave` and `KagEvent::AutoLoad` variants. |
| 3 | `kag-interpreter` | `[executor]` | Handle `TagName::Autosave` → emit `KagEvent::AutoSave`; `TagName::Autoload` → emit `KagEvent::AutoLoad`. |
| 4 | `kani-runtime` | `[rt-events]` | Add `EvSaveLoad::AutoSave` and `EvSaveLoad::AutoLoad` to a new `EvSaveLoad` event enum. |
| 5 | `kani-runtime` | `[dispatch]` | Route `KagEvent::AutoSave` / `AutoLoad` → `EvSaveLoad`. |

### 1b. `commit`

Flush current variable state to the active save slot.

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Commit("commit")` tag (no attrs). |
| 2 | `kag-interpreter` | `[events]` | Add `KagEvent::CommitSave`. |
| 3 | `kag-interpreter` | `[executor]` | Handle `TagName::Commit` → emit event. |
| 4 | `kani-runtime` | `[rt-events]` | Add `EvSaveLoad::Commit`. |

### 1c. `savesnap` / `save_img`

Capture a thumbnail for the save screen.

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Savesnap("savesnap")` (no attrs) and `SaveImg("save_img") { storage: optional<str> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::Savesnap` and `ResolvedTag::SaveImg { storage }`. |
| 3 | `kag-interpreter` | `[executor]` | Emit `KagEvent::Tag(ResolvedTag::Savesnap)` / `SaveImg`. |
| 4 | `kani-runtime` | `[rt-events]` | Add variants to `EvSaveLoad`. |

### 1d. `checkpoint` / `clear_checkpoint` / `rollback`

Mark undo points for the player to roll back to.

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Checkpoint("checkpoint")`, `ClearCheckpoint("clear_checkpoint")`, `Rollback("rollback")`. |
| 2 | `kag-interpreter` | `[snapshot]` | Add a `checkpoint_stack: Vec<InterpreterSnapshot>` field to `RuntimeContext` (not serialised — checkpoints are session-local). |
| 3 | `kag-interpreter` | `[executor]` | `checkpoint` → push a snapshot onto the stack. `rollback` → pop & restore. `clear_checkpoint` → clear the stack. |
| 4 | `kag-interpreter` | `[events]` | Add `KagEvent::Rollback` so the host can also restore visual state. |

### 1e. `clearfix`

Clear persistent (`fix`) variables that survive across save slots.

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Clearfix("clearfix")`. |
| 2 | `kag-interpreter` | `[events]` | Add `KagEvent::ClearFix`. |
| 3 | `kag-interpreter` | `[executor]` | Emit event; actual clearing is host-side since `fix` lives outside the interpreter. |

### 1f. `sleepgame` / `awakegame` / `breakgame`

Push/pop entire game state for nested sub-games (e.g. mini-game inside a VN).

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Sleepgame("sleepgame") { storage: optional<str> }`, `Awakegame("awakegame")`, `Breakgame("breakgame")`. |
| 2 | `kag-interpreter` | `[snapshot]` | Add `game_stack: Vec<InterpreterSnapshot>` to `RuntimeContext`. |
| 3 | `kag-interpreter` | `[executor]` | `sleepgame` → snapshot current state, push to stack, jump to new storage. `awakegame` → pop & restore. `breakgame` → pop & discard. |
| 4 | `kag-interpreter` | `[events]` | Add `KagEvent::SleepGame` / `AwakeGame` for host visual restore. |

---

## 2. Camera System

**Tags**: `camera`, `reset_camera`, `wait_camera`

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add: `Camera("camera") { x: optional<f32>, y: optional<f32>, zoom: optional<f32>, time: optional<u64>, from_x: optional<f32>, from_y: optional<f32>, from_zoom: optional<f32> }`, `ResetCamera("reset_camera") { time: optional<u64> }`, `WaitCamera("wait_camera")` (add to `@wait_group`). |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::Camera { x, y, zoom, time, from_x, from_y, from_zoom }`, `ResolvedTag::ResetCamera { time }`. Add `TagName::WaitCamera` to the wait-group handling. |
| 3 | `kag-interpreter` | `[executor]` | Resolve attrs and emit `KagEvent::Tag`. |
| 4 | `kani-runtime` | `[rt-events]` | New `EvCameraTag` enum: `MoveCamera { … }`, `ResetCamera { time }`. |
| 5 | `kani-runtime` | `[dispatch]` | Route `ResolvedTag::Camera` → `EvCameraTag::MoveCamera`, etc. |
| 6 | `kani-runtime` | `lib.rs` | Register `EvCameraTag` event + add to update systems. |

---

## 3. Speech-Bubble (Fuki) Mode

**Tags**: `fuki_start`, `fuki_stop`, `fuki_chara`

Switches the message display from a fixed window to per-character speech bubbles.

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `FukiStart("fuki_start")`, `FukiStop("fuki_stop")`, `FukiChara("fuki_chara") { name: recommended<str>, left: optional<f32>, top: optional<f32>, width: optional<f32>, height: optional<f32> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::FukiStart`, `ResolvedTag::FukiStop`, `ResolvedTag::FukiChara { name, left, top, width, height }`. |
| 3 | `kag-interpreter` | `[executor]` | Emit `KagEvent::Tag` for each. |
| 4 | `kag-interpreter` | `[snapshot]` | Add `fuki_mode: bool` flag to `InterpreterSnapshot` for save/load. |
| 5 | `kani-runtime` | `[rt-events]` | Add variants to `EvMessageWindowTag` or create new `EvFukiTag`. |
| 6 | `kani-runtime` | `[dispatch]` | Route the new `ResolvedTag` variants. |

---

## 4. Auto-Mode Control

**Tags**: `autostart`, `autostop`, `autoconfig`

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Autostart("autostart")`, `Autostop("autostop")`, `Autoconfig("autoconfig") { speed: optional<u64>, page_wait: optional<u64> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::AutoMode { enabled: bool }` and `ResolvedTag::AutoConfig { speed, page_wait }`. |
| 3 | `kag-interpreter` | `[executor]` | `autostart` → emit `AutoMode { enabled: true }`. `autostop` → `{ enabled: false }`. `autoconfig` → `AutoConfig`. |
| 4 | `kag-interpreter` | `[snapshot]` | Add `auto_mode: bool` to `InterpreterSnapshot`. |
| 5 | `kani-runtime` | `[rt-events]` | Add `EvControlTag::AutoMode { enabled }` and `EvControlTag::AutoConfig { speed, page_wait }`. |
| 6 | `kani-runtime` | `[dispatch]` | Route accordingly. |

---

## 7. Voice Configuration

**Tags**: `voconfig`, `vostart`, `vostop`

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Voconfig("voconfig") { name: optional<str>, buf: optional<u32>, vosf: optional<bool> }`, `Vostart("vostart")`, `Vostop("vostop")`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::Voconfig { name, buf, vosf }`, `ResolvedTag::Vostart`, `ResolvedTag::Vostop`. |
| 3 | `kag-interpreter` | `[executor]` | Emit `KagEvent::Tag` variants. |
| 4 | `kani-runtime` | `[rt-events]` | Add `EvAudioTag::SetVoiceConfig { … }`, `EvAudioTag::StartVoice`, `EvAudioTag::StopVoice`. |
| 5 | `kani-runtime` | `[dispatch]` | Route the new resolved tags. |

---

## 8. SE Fade

**Tags**: `fadeinse`, `fadeoutse`

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Fadeinse("fadeinse") { storage: recommended<str>, time: optional<u64>, buf: optional<u32> }`, `Fadeoutse("fadeoutse") { time: optional<u64>, buf: optional<u32> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::FadeinSe { storage, time, buf }`, `ResolvedTag::FadeoutSe { time, buf }`. |
| 3 | `kag-interpreter` | `[executor]` | Resolve and emit. |
| 4 | `kani-runtime` | `[rt-events]` | Add `EvAudioTag::FadeInSe { storage, time, buf }`, `EvAudioTag::FadeOutSe { time, buf }`. |
| 5 | `kani-runtime` | `[dispatch]` | Route the new resolved tags. |

---

## 10. Positioned Text (`ptext` / `mtext`)

**Tags**: `ptext`, `mtext`

Absolute-positioned text labels and multi-line text areas, independent of the message window.

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Ptext("ptext") { name: recommended<str>, x: optional<f32>, y: optional<f32>, width: optional<f32>, face: optional<str>, size: optional<f32>, color: optional<str>, text: optional<str>, overwrite: optional<bool>, layer: optional<str> }`, `Mtext("mtext") { x: optional<f32>, y: optional<f32>, width: optional<f32>, height: optional<f32>, text: optional<str>, face: optional<str>, size: optional<f32>, color: optional<str>, layer: optional<str> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::Ptext { name, x, y, width, face, size, color, text, overwrite, layer }`, `ResolvedTag::Mtext { x, y, width, height, text, face, size, color, layer }`. |
| 3 | `kag-interpreter` | `[executor]` | Resolve and emit `KagEvent::Tag`. |
| 4 | `kani-runtime` | `[rt-events]` | Add `EvMessageWindowTag::SpawnPtext { … }` and `EvMessageWindowTag::SpawnMtext { … }`. |
| 5 | `kani-runtime` | `[dispatch]` | Route accordingly. |

---

## 11. Close Confirmation

**Tags**: `closeconfirm_on`, `closeconfirm_off`

Enable/disable a "Are you sure you want to quit?" dialog when the user tries to close the window.

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `CloseconfirmOn("closeconfirm_on")`, `CloseconfirmOff("closeconfirm_off")`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::CloseConfirm { enabled: bool }`. |
| 3 | `kag-interpreter` | `[executor]` | `closeconfirm_on` → `CloseConfirm { enabled: true }`, `_off` → `false`. |
| 4 | `kani-runtime` | `[rt-events]` | Add `EvUiTag::SetCloseConfirm { enabled: bool }`. |
| 5 | `kani-runtime` | `[dispatch]` | Route. |

---

## 13. Dialog Configuration

**Tags**: `dialog_config`, `dialog_config_ok`, `dialog_config_ng`, `dialog_config_filter`

Customise the appearance and button text of the built-in dialog box.

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `DialogConfig("dialog_config") { bg: optional<str>, color: optional<str>, width: optional<f32>, height: optional<f32> }`, `DialogConfigOk("dialog_config_ok") { text: optional<str>, bg: optional<str>, color: optional<str> }`, `DialogConfigNg("dialog_config_ng") { text: optional<str>, bg: optional<str>, color: optional<str> }`, `DialogConfigFilter("dialog_config_filter") { color: optional<str>, opacity: optional<f32> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::DialogConfig { … }`, `ResolvedTag::DialogConfigOk { … }`, `ResolvedTag::DialogConfigNg { … }`, `ResolvedTag::DialogConfigFilter { … }`. |
| 3 | `kag-interpreter` | `[executor]` | Resolve and emit. |
| 4 | `kani-runtime` | `[rt-events]` | Add variants to `EvUiTag`: `ConfigureDialog`, `ConfigureDialogOk`, `ConfigureDialogNg`, `ConfigureDialogFilter`. |
| 5 | `kani-runtime` | `[dispatch]` | Route accordingly. |

---

## 14. Default Font (`deffont`)

**Tag**: `deffont`

Set the default font properties that `[resetfont]` restores to.

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Deffont("deffont") { face: optional<str>, size: optional<f32>, color: optional<str>, bold: optional<bool>, italic: optional<bool>, shadow: optional<bool>, edge: optional<bool> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::Deffont { face, size, color, bold, italic, shadow, edge }`. |
| 3 | `kag-interpreter` | `[executor]` | Resolve and emit `KagEvent::Tag`. |
| 4 | `kag-interpreter` | `[snapshot]` | Add `deffont: Option<DeffontSnap>` to `InterpreterSnapshot` to persist defaults across saves. Define `DeffontSnap` as a serialisable struct with the same fields. |
| 5 | `kani-runtime` | `[rt-events]` | Add `EvMessageWindowTag::SetDefaultFont { face, size, color, bold, italic, shadow, edge }`. |
| 6 | `kani-runtime` | `[dispatch]` | Route `ResolvedTag::Deffont` → `EvMessageWindowTag::SetDefaultFont`. |

---

## 15. Particle Effects (`popopo`)

**Tag**: `popopo`

Spawn a particle/bubble effect overlay (e.g. falling petals, floating bubbles).

### Steps

| Step | Crate | File | Change |
|------|-------|------|--------|
| 1 | `kag-syntax` | `[tag_defs]` | Add `Popopo("popopo") { storage: recommended<str>, count: optional<u64>, x: optional<f32>, y: optional<f32>, width: optional<f32>, height: optional<f32>, time: optional<u64>, layer: optional<str> }`. |
| 2 | `kag-interpreter` | `[events]` | Add `ResolvedTag::Popopo { storage, count, x, y, width, height, time, layer }`. |
| 3 | `kag-interpreter` | `[executor]` | Resolve attrs and emit `KagEvent::Tag`. |
| 4 | `kani-runtime` | `[rt-events]` | Add `EvEffectTag::Popopo { storage, count, x, y, width, height, time, layer }`. |
| 5 | `kani-runtime` | `[dispatch]` | Route `ResolvedTag::Popopo` → `EvEffectTag::Popopo`. |

---

## Implementation Order

Suggested priority based on dependency and user impact:

1. **Save/Load** (§1) — foundation for all persistence; `autosave`/`autoload` first, then `checkpoint`/`rollback`, then `sleepgame`
2. **Default Font** (§14) — small, self-contained, unblocks `resetfont` correctness
3. **Close Confirmation** (§11) — small, high UX impact
4. **Auto-Mode Control** (§4) — small, completes skip/auto pair
5. **SE Fade** (§8) — small, extends existing audio infrastructure
6. **Voice Configuration** (§7) — small, extends existing audio infrastructure
7. **Camera System** (§2) — medium scope, high visual impact
8. **Positioned Text** (§10) — medium scope, needed for UI-heavy games
9. **Dialog Configuration** (§13) — cosmetic, can ship with defaults
10. **Speech-Bubble Mode** (§3) — largest scope, needs rendering work
11. **Particle Effects** (§15) — visual polish, lowest priority

---

## Cross-Cutting Concerns

- **Snapshot compatibility**: Any new field added to `InterpreterSnapshot` must use `#[serde(default)]` to remain backward-compatible with existing save files.
- **Tag name registration**: Every new tag added to `define_tags!` automatically gets a `TagName` variant, `from_name()` entry, and LSP completion. No manual wiring needed.
- **Testing**: Each new tag should have at minimum:
  - A `tag_defs` unit test (required/optional attr diagnostics)
  - An executor unit test (correct event emission)
  - An integration `.ks` scenario in `kag-interpreter/tests/`

