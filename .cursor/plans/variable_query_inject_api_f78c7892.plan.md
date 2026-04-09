---
name: Variable query/inject API
overview: "Add two-way variable access to `KagInterpreter` without shared state: a `snapshot()` pull query and a `set_variable()` push command, both tunneled through the existing `HostEvent` channel using a side-band helper so every blocking wait loop handles them transparently."
todos:
  - id: events-types
    content: Add `VariableSnapshot` struct and two new `HostEvent` variants (`SetVariable`, `QueryVariables`) to events.rs
    status: pending
  - id: side-band-helper
    content: Add `try_side_band()` helper function in mod.rs above `interpreter_task`
    status: pending
  - id: patch-loops
    content: Patch all five blocking `loop` bodies in `interpreter_task` to route events through `try_side_band` before the loop-specific match arms
    status: pending
  - id: public-api
    content: Add `set_variable()` and `snapshot()` async methods to the `KagInterpreter` impl block
    status: pending
isProject: false
---

# Variable Query and Injection API

## Design summary

```mermaid
sequenceDiagram
    participant Bridge as BevyBridge
    participant Handle as KagInterpreter
    participant Task as interpreter_task

    Bridge->>Handle: set_variable(F, "player_name", "\"Alice\"")
    Handle->>Task: HostEvent::SetVariable{...}
    Task->>Task: exec("f.player_name = \"Alice\";")
    Note over Task: handled by side-band helper inside any blocking loop

    Bridge->>Handle: snapshot()
    Handle->>Task: HostEvent::QueryVariables(oneshot_tx)
    Task->>Bridge: oneshot: VariableSnapshot { f: {...}, sf: {...}, tf: {...} }
```



## Files changed

### 1. `[kag-interpreter/src/events.rs](kag-interpreter/src/events.rs)`

**Add `VariableSnapshot` type** (all values stringified — consistent with existing `VariableChanged` design note):

```rust
#[derive(Debug, Clone)]
pub struct VariableSnapshot {
    pub f: std::collections::HashMap<String, String>,
    pub sf: std::collections::HashMap<String, String>,
    pub tf: std::collections::HashMap<String, String>,
}
```

**Add two new `HostEvent` variants** after the existing `Resume` arm:

```rust
/// Set a single variable. `value_expr` is evaluated as a Rhai expression
/// (e.g. `"42"`, `"true"`, `"\"Alice\""`).
SetVariable {
    scope: VarScope,
    key: String,
    value_expr: String,
},

/// Request a point-in-time snapshot of all variable scopes.
/// The reply arrives through the oneshot channel — valid to call
/// whenever the interpreter is blocked at any pause point.
QueryVariables(tokio::sync::oneshot::Sender<VariableSnapshot>),
```

`oneshot::Sender<VariableSnapshot>` is `Send` so the `HostEvent` enum remains `Send`.

---

### 2. `[kag-interpreter/src/runtime/mod.rs](kag-interpreter/src/runtime/mod.rs)`

#### 2a. Side-band helper (new private function)

Placed above `interpreter_task`. Returns `None` when the event was consumed, `Some(event)` when it should still be matched by the caller's loop:

```rust
fn try_side_band(ctx: &mut RuntimeContext, event: HostEvent) -> Option<HostEvent> {
    match event {
        HostEvent::SetVariable { scope, key, value_expr } => {
            let prefix = match scope {
                VarScope::F  => "f",
                VarScope::Sf => "sf",
                VarScope::Tf => "tf",
                VarScope::Mp => "mp",
            };
            // Mirrors [eval] — errors become warnings rather than panics.
            let _ = ctx.script_engine.exec(&format!("{prefix}.{key} = {value_expr};"));
            None
        }
        HostEvent::QueryVariables(tx) => {
            let snap = VariableSnapshot {
                f:  ctx.script_engine.f().into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                sf: ctx.script_engine.sf().into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                tf: ctx.script_engine.tf().into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            };
            let _ = tx.send(snap);
            None
        }
        other => Some(other),
    }
}
```

#### 2b. Patch every blocking `loop` in `interpreter_task`

There are five blocking loops (`Stop`, `WaitForClick`, `WaitMs`, cross-file `Jump`, `Return`, `BeginChoices`). Each currently has `_ => {}`. Replace that pattern uniformly:

```rust
// before (example from Stop loop):
Some(HostEvent::Clicked) | Some(HostEvent::Resume) => break,
None => return,
_ => {}

// after:
Some(event) => {
    if let Some(event) = try_side_band(&mut ctx, event) {
        match event {
            HostEvent::Clicked | HostEvent::Resume => break,  // loop-specific arms
            _ => {}
        }
    }
}
None => return,
```

The `Jump`/`Return` load loops use `ScenarioLoaded` as their terminal arm — same pattern applies.

#### 2c. New public methods on `KagInterpreter`

```rust
/// Inject a variable value before resuming. `value_expr` is a Rhai literal
/// or expression (e.g. `"42"`, `"\"Alice\""`, `"f.count + 1"`).
pub async fn set_variable(
    &self,
    scope: VarScope,
    key: impl Into<String>,
    value_expr: impl Into<String>,
) -> Result<(), KagError> {
    self.send(HostEvent::SetVariable {
        scope,
        key: key.into(),
        value_expr: value_expr.into(),
    }).await
}

/// Return a point-in-time copy of all variable scopes.
/// Call only when the interpreter is paused (after any blocking KagEvent).
pub async fn snapshot(&self) -> Result<VariableSnapshot, KagError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    self.send(HostEvent::QueryVariables(tx)).await?;
    rx.await.map_err(|_| KagError::ChannelClosed)
}
```

---

## What is not changed

- `script_engine.rs` — no new methods needed; `exec` already exists and is reused.
- `executor.rs` — the existing `TAG_EVAL` path is the model; nothing changes there.
- `VariableChanged` enum variant — left in place as-is (unused, harmless).

## Usage contract (for bridge documentation)

- `set_variable` is safe to call any time the bridge has received a blocking `KagEvent` (`WaitForClick`, `Stop`, `WaitMs`, `BeginChoices`) and has not yet sent the corresponding resume event. Also safe to call before the interpreter starts.
- `snapshot` follows the same rule. Calling either while the interpreter is actively executing (between events) risks the `HostEvent` sitting in the channel buffer until the next pause — it will still be processed correctly, just not immediately.

