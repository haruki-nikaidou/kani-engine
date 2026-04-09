//! Async KAG interpreter actor.
//!
//! `KagInterpreter` runs as a Tokio task and communicates with its host
//! (e.g. a Bevy system) through two async MPSC channels:
//!
//! - **`event_rx`** — the host *receives* `KagEvent`s from the interpreter.
//! - **`input_tx`** — the host *sends* `HostEvent`s back to the interpreter.
//!
//! The interpreter drives the scenario forward op by op.  When an op requires
//! a blocking host response (click wait, timer, choice selection, scenario
//! load), the interpreter awaits the next relevant `HostEvent` before
//! continuing.

pub mod context;
pub mod executor;
pub mod script_engine;

use tokio::sync::mpsc;

use crate::ast::Script;
use crate::error::{KagError, ParseDiagnostic};
use crate::events::{HostEvent, KagEvent};
use crate::parser::parse_script;
use crate::snapshot::InterpreterSnapshot;

use context::RuntimeContext;
use executor::execute_op;

// ─── Channel capacity ─────────────────────────────────────────────────────────

/// Number of `KagEvent`s that can be buffered before the interpreter blocks.
const EVENT_CHANNEL_CAP: usize = 64;
/// Number of `HostEvent`s that can be buffered before the host blocks.
const INPUT_CHANNEL_CAP: usize = 16;

// ─── Public actor handle ──────────────────────────────────────────────────────

/// A handle to a running KAG interpreter task.
///
/// Drop this to shut down the interpreter (the task will exit once both channel
/// ends are dropped).
///
/// ## Thread-locality
///
/// Rhai's `Engine` and `Scope` use `Rc` internally, making them `!Send`.
/// Therefore the interpreter task **must** run on the same thread as its
/// caller.  Use [`KagInterpreter::spawn`] inside a `tokio::task::LocalSet`
/// (e.g. via `LocalSet::run_until` or a `#[tokio::main]` with a current-thread
/// runtime).
#[derive(Debug)]
pub struct KagInterpreter {
    /// Send host events to the interpreter (clicks, timer ticks, choices, …).
    pub input_tx: mpsc::Sender<HostEvent>,
    /// Receive scenario events from the interpreter (text, waits, jumps, …).
    pub event_rx: mpsc::Receiver<KagEvent>,
}

impl KagInterpreter {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Spawn an interpreter from an already-owned `Script<'static>`.
    ///
    /// Returns the actor handle and a `JoinHandle` for the local task.
    /// This must be called from within a `tokio::task::LocalSet` context
    /// because Rhai's internals are `!Send`.
    pub fn spawn(script: Script<'static>) -> (Self, tokio::task::JoinHandle<()>) {
        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAP);
        let (input_tx, input_rx) = mpsc::channel(INPUT_CHANNEL_CAP);

        let storage = script.source_name.clone();
        // spawn_local because Rhai's Engine/Scope are !Send (they use Rc)
        let task = tokio::task::spawn_local(interpreter_task(script, storage, event_tx, input_rx));

        let handle = Self { input_tx, event_rx };
        (handle, task)
    }

    /// Parse a `.ks` source string and spawn an interpreter in one step.
    ///
    /// The source is borrowed during parsing and then converted to owned data
    /// before the async task starts.
    ///
    /// Any [`ParseDiagnostic`]s produced during parsing are returned alongside
    /// the handle and join-handle so callers can inspect or log them.  A
    /// non-empty diagnostics list does **not** mean the script is unusable —
    /// the interpreter still receives a best-effort op stream.
    pub fn spawn_from_source(
        source: &str,
        source_name: &str,
    ) -> Result<(Self, tokio::task::JoinHandle<()>, Vec<ParseDiagnostic>), KagError> {
        let (script, diags) = parse_script(source, source_name);
        let (handle, task) = Self::spawn(script);
        Ok((handle, task, diags))
    }

    /// Restore a previously saved interpreter from a snapshot and spawn it.
    ///
    /// `source` must be the `.ks` source text of `snapshot.storage` (the
    /// scenario file that was active at save time).  The script is re-parsed
    /// from that source so that op indices are stable — **the source must not
    /// have changed since the snapshot was taken**.
    ///
    /// If the call stack contains cross-file frames (a `[call]` that jumped
    /// into a different file), those files do not need to be supplied here;
    /// they will be requested via the normal `KagEvent::Return` /
    /// `HostEvent::ScenarioLoaded` mechanism when `[return]` is encountered.
    ///
    /// Any parse diagnostics are returned alongside the handle.
    pub fn spawn_from_snapshot(
        snapshot: InterpreterSnapshot,
        source: &str,
    ) -> Result<(Self, tokio::task::JoinHandle<()>, Vec<ParseDiagnostic>), KagError> {
        let source_name = snapshot.storage.clone();
        let (script, diags) = parse_script(source, &source_name);

        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAP);
        let (input_tx, input_rx) = mpsc::channel(INPUT_CHANNEL_CAP);

        let task = tokio::task::spawn_local(interpreter_task_from_snapshot(
            script, snapshot, event_tx, input_rx,
        ));

        let handle = Self { input_tx, event_rx };
        Ok((handle, task, diags))
    }

    // ── Channel convenience ───────────────────────────────────────────────────

    /// Receive the next `KagEvent` from the interpreter, blocking asynchronously.
    pub async fn recv(&mut self) -> Option<KagEvent> {
        self.event_rx.recv().await
    }

    /// Send a `HostEvent` to the interpreter.
    pub async fn send(&self, event: HostEvent) -> Result<(), KagError> {
        self.input_tx
            .send(event)
            .await
            .map_err(|_| KagError::ChannelClosed)
    }
}

// ─── Interpreter task ─────────────────────────────────────────────────────────

// ─── Snapshot helper ──────────────────────────────────────────────────────────

/// Emit a snapshot event if `ctx.to_snapshot()` succeeds, or an error event.
async fn emit_snapshot(ctx: &RuntimeContext, event_tx: &mpsc::Sender<KagEvent>) {
    match ctx.to_snapshot() {
        Ok(snap) => {
            let _ = event_tx.send(KagEvent::Snapshot(Box::new(snap))).await;
        }
        Err(e) => {
            let _ = event_tx
                .send(KagEvent::Error(format!("snapshot error: {e}")))
                .await;
        }
    }
}

// ─── Interpreter tasks ────────────────────────────────────────────────────────

/// Spawn variant: restore an interpreter from a saved snapshot.
async fn interpreter_task_from_snapshot(
    script: Script<'static>,
    snapshot: InterpreterSnapshot,
    event_tx: mpsc::Sender<KagEvent>,
    input_rx: mpsc::Receiver<HostEvent>,
) {
    let storage = snapshot.storage.clone();
    let mut ctx = RuntimeContext::new(storage);
    if let Err(e) = ctx.restore_from_snapshot(&snapshot) {
        let _ = event_tx
            .send(KagEvent::Error(format!("restore error: {e}")))
            .await;
        return;
    }
    run_interpreter(script, ctx, event_tx, input_rx).await;
}

/// The async task that runs the KAG scenario.
///
/// Scenario execution is a simple loop:
///  1. Execute the next op via `execute_op` (synchronous, mutates `ctx`).
///  2. Emit all resulting `KagEvent`s over `event_tx`.
///  3. If the op requires a host response, await the appropriate `HostEvent`.
///  4. Handle scenario-loading when a `Jump`/`Call` targets a different file.
async fn interpreter_task(
    script: Script<'static>,
    storage: String,
    event_tx: mpsc::Sender<KagEvent>,
    input_rx: mpsc::Receiver<HostEvent>,
) {
    let ctx = RuntimeContext::new(storage);
    run_interpreter(script, ctx, event_tx, input_rx).await;
}

/// Core interpreter loop shared by the normal and snapshot-restore paths.
async fn run_interpreter(
    mut script: Script<'static>,
    mut ctx: RuntimeContext,
    event_tx: mpsc::Sender<KagEvent>,
    mut input_rx: mpsc::Receiver<HostEvent>,
) {

    loop {
        // ── Execute the next op ───────────────────────────────────────────
        if ctx.pc >= script.ops.len() {
            let _ = event_tx.send(KagEvent::End).await;
            break;
        }

        let events = match execute_op(&script, &mut ctx) {
            Ok(evs) => evs,
            Err(e) => {
                let _ = event_tx.send(KagEvent::Error(e.to_string())).await;
                break;
            }
        };

        // ── Emit all events and handle blocking ones ───────────────────────
        for event in events {
            match event {
                // ── End of scenario ────────────────────────────────────────
                KagEvent::End => {
                    let _ = event_tx.send(KagEvent::End).await;
                    return;
                }

                // ── Hard stop: wait for explicit Resume or Click ───────────
                KagEvent::Stop => {
                    let _ = event_tx.send(KagEvent::Stop).await;
                    loop {
                        match input_rx.recv().await {
                            Some(HostEvent::Clicked) | Some(HostEvent::Resume) => break,
                            Some(HostEvent::TakeSnapshot) => {
                                emit_snapshot(&ctx, &event_tx).await;
                            }
                            None => return, // channel closed
                            _ => {}         // ignore unrelated events
                        }
                    }
                }

                // ── Click waits ────────────────────────────────────────────
                KagEvent::WaitForClick { clear_after } => {
                    let _ = event_tx.send(KagEvent::WaitForClick { clear_after }).await;
                    loop {
                        match input_rx.recv().await {
                            Some(HostEvent::Clicked) => break,
                            Some(HostEvent::TakeSnapshot) => {
                                emit_snapshot(&ctx, &event_tx).await;
                            }
                            None => return,
                            _ => {}
                        }
                    }
                    if clear_after {
                        let _ = event_tx.send(KagEvent::ClearMessage).await;
                    }
                }

                // ── Timed wait ─────────────────────────────────────────────
                KagEvent::WaitMs(ms) => {
                    let _ = event_tx.send(KagEvent::WaitMs(ms)).await;
                    loop {
                        match input_rx.recv().await {
                            Some(HostEvent::TimerElapsed) | Some(HostEvent::Clicked) => break,
                            Some(HostEvent::TakeSnapshot) => {
                                emit_snapshot(&ctx, &event_tx).await;
                            }
                            None => return,
                            _ => {}
                        }
                    }
                }

                // ── Jump / Call: may require scenario load ─────────────────
                KagEvent::Jump {
                    storage: new_storage,
                    target,
                } => {
                    let needs_load = new_storage
                        .as_deref()
                        .map(|s| s != ctx.current_storage)
                        .unwrap_or(false);

                    let _ = event_tx
                        .send(KagEvent::Jump {
                            storage: new_storage.clone(),
                            target: target.clone(),
                        })
                        .await;

                    if needs_load {
                        // Ask host to load the new scenario file
                        loop {
                            match input_rx.recv().await {
                                Some(HostEvent::ScenarioLoaded { name, source }) => {
                                    let (new_script, diags) = parse_script(&source, &name);
                                    script = new_script;
                                    ctx.current_storage = name.clone();
                                    // Forward any parse-error diagnostics as warnings.
                                    for d in diags {
                                        let _ = event_tx.send(KagEvent::Warning(d.message)).await;
                                    }
                                    // Resolve jump target inside the new script.
                                    let idx = if let Some(ref t) = target {
                                        let key = t.trim_start_matches('*');
                                        match script.label_map.get(key).copied() {
                                            Some(i) => i,
                                            None => {
                                                let _ = event_tx
                                                    .send(KagEvent::Warning(format!(
                                                        "label '{key}' not found in '{}' \
                                                         (script.label_map has {} label(s)); \
                                                         jumping to start",
                                                        ctx.current_storage,
                                                        script.label_map.len(),
                                                    )))
                                                    .await;
                                                0
                                            }
                                        }
                                    } else {
                                        0
                                    };
                                    ctx.jump_to(idx);
                                    break;
                                }
                                None => return,
                                _ => {}
                            }
                        }
                    } else if let Some(ref t) = target {
                        // Same-file jump: resolve the label now
                        let key = t.trim_start_matches('*');
                        if let Some(&idx) = script.label_map.get(key) {
                            ctx.jump_to(idx);
                        } else {
                            let _ = event_tx
                                .send(KagEvent::Error(format!(
                                    "label not found: '{key}' in '{}'",
                                    ctx.current_storage
                                )))
                                .await;
                            return;
                        }
                    }
                }

                // ── Cross-file return: reload caller's script ──────────────
                KagEvent::Return { storage } => {
                    let _ = event_tx
                        .send(KagEvent::Return {
                            storage: storage.clone(),
                        })
                        .await;
                    loop {
                        match input_rx.recv().await {
                            Some(HostEvent::ScenarioLoaded { name, source }) => {
                                let (new_script, diags) = parse_script(&source, &name);
                                script = new_script;
                                ctx.current_storage = name;
                                for d in diags {
                                    let _ = event_tx.send(KagEvent::Warning(d.message)).await;
                                }
                                // ctx.pc was already set to return_pc by the
                                // executor — do NOT override it here.
                                break;
                            }
                            None => return,
                            _ => {}
                        }
                    }
                }

                // ── Choice prompt ──────────────────────────────────────────
                KagEvent::BeginChoices(choices) => {
                    let _ = event_tx.send(KagEvent::BeginChoices(choices)).await;
                    loop {
                        match input_rx.recv().await {
                            Some(HostEvent::ChoiceSelected(idx)) => {
                                ctx.script_engine
                                    .set_f("_last_choice", rhai::Dynamic::from(idx as i64));
                                break;
                            }
                            Some(HostEvent::TakeSnapshot) => {
                                emit_snapshot(&ctx, &event_tx).await;
                            }
                            None => return,
                            _ => {}
                        }
                    }
                }

                // ── Non-blocking events — just forward ─────────────────────
                other => {
                    if event_tx.send(other).await.is_err() {
                        return; // host dropped the receiver
                    }
                }
            }
        }
    }
}

// ─── Integration tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_script;
    use tokio::task::LocalSet;

    /// Run an async closure inside a `LocalSet` (required because Rhai is `!Send`).
    async fn with_local<F, Fut>(f: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let local = LocalSet::new();
        local.run_until(f()).await;
    }

    async fn collect_events(handle: &mut KagInterpreter, limit: usize) -> Vec<KagEvent> {
        let mut events = Vec::new();
        for _ in 0..limit {
            match handle.recv().await {
                Some(KagEvent::End) => {
                    events.push(KagEvent::End);
                    break;
                }
                Some(e) => events.push(e),
                None => break,
            }
        }
        events
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_simple_text_scenario() {
        with_local(|| async {
            let src = "Hello!\n@l\nWorld!\n";
            let (mut handle, _task, _diags) =
                KagInterpreter::spawn_from_source(src, "test.ks").unwrap();

            let mut events = Vec::new();
            loop {
                match handle.recv().await {
                    Some(KagEvent::WaitForClick { .. }) => {
                        events.push(KagEvent::WaitForClick { clear_after: false });
                        break;
                    }
                    Some(e) => events.push(e),
                    None => break,
                }
            }

            let has_hello = events
                .iter()
                .any(|e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("Hello")));
            assert!(has_hello, "events before [l]: {:?}", events);

            handle.send(HostEvent::Clicked).await.unwrap();

            let mut post = Vec::new();
            loop {
                match handle.recv().await {
                    Some(KagEvent::End) | None => break,
                    Some(e) => post.push(e),
                }
            }

            let has_world = post
                .iter()
                .any(|e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("World")));
            assert!(has_world, "events after click: {:?}", post);
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_end_event_emitted() {
        with_local(|| async {
            let src = "text\n";
            let (script, _diags) = parse_script(src, "t.ks");
            let (mut handle, _) = KagInterpreter::spawn(script);
            let events = collect_events(&mut handle, 10).await;
            assert!(
                events.iter().any(|e| matches!(e, KagEvent::End)),
                "{:?}",
                events
            );
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_stop_unblocks_on_click() {
        with_local(|| async {
            let src = "@s\nafter stop\n";
            let (script, _diags) = parse_script(src, "t.ks");
            let (mut handle, _) = KagInterpreter::spawn(script);

            loop {
                match handle.recv().await {
                    Some(KagEvent::Stop) => break,
                    None => panic!("channel closed before Stop"),
                    _ => {}
                }
            }

            handle.send(HostEvent::Clicked).await.unwrap();

            let mut post = Vec::new();
            loop {
                match handle.recv().await {
                    Some(KagEvent::End) | None => break,
                    Some(e) => post.push(e),
                }
            }

            let has_after = post
                .iter()
                .any(|e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("after")));
            assert!(has_after, "post-stop events: {:?}", post);
        })
        .await;
    }

    /// Verify that `[call storage=sub.ks]` / `[return]` across two files works:
    /// the interpreter should execute the callee, return to the caller, and
    /// emit the text that follows the original `[call]` tag.
    #[tokio::test(flavor = "current_thread")]
    async fn test_cross_file_call_return() {
        with_local(|| async {
            let caller_src = "[call storage=sub.ks target=*fn]\nback\n";
            let sub_src = "*fn\nin sub\n[return]\n";

            let (mut handle, _task, _diags) =
                KagInterpreter::spawn_from_source(caller_src, "caller.ks").unwrap();

            let mut all_events = Vec::<KagEvent>::new();

            loop {
                match handle.recv().await {
                    None => break,
                    Some(KagEvent::End) => {
                        all_events.push(KagEvent::End);
                        break;
                    }
                    // Interpreter crossed into sub.ks — supply its source.
                    Some(KagEvent::Jump {
                        storage: Some(ref s),
                        ..
                    }) if s == "sub.ks" => {
                        handle
                            .send(HostEvent::ScenarioLoaded {
                                name: "sub.ks".into(),
                                source: sub_src.into(),
                            })
                            .await
                            .unwrap();
                    }
                    // Interpreter is returning to caller.ks — supply its source again.
                    Some(KagEvent::Return { ref storage }) if storage == "caller.ks" => {
                        handle
                            .send(HostEvent::ScenarioLoaded {
                                name: "caller.ks".into(),
                                source: caller_src.into(),
                            })
                            .await
                            .unwrap();
                    }
                    Some(e) => all_events.push(e),
                }
            }

            let has_in_sub = all_events.iter().any(
                |e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("in sub")),
            );
            assert!(
                has_in_sub,
                "expected 'in sub' text; events: {:?}",
                all_events
            );

            let has_back = all_events
                .iter()
                .any(|e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("back")));
            assert!(
                has_back,
                "expected 'back' text after return; events: {:?}",
                all_events
            );

            // "back" must come after "in sub"
            let sub_pos = all_events.iter().position(
                |e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("in sub")),
            );
            let back_pos = all_events.iter().position(
                |e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("back")),
            );
            assert!(
                sub_pos < back_pos,
                "'in sub' ({sub_pos:?}) should precede 'back' ({back_pos:?})"
            );
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_wait_ms_unblocks_on_timer_elapsed() {
        with_local(|| async {
            let src = "@wait time=100\ndone\n";
            let (script, _diags) = parse_script(src, "t.ks");
            let (mut handle, _) = KagInterpreter::spawn(script);

            loop {
                match handle.recv().await {
                    Some(KagEvent::WaitMs(_)) => break,
                    None => panic!("channel closed"),
                    _ => {}
                }
            }

            handle.send(HostEvent::TimerElapsed).await.unwrap();

            let mut post = Vec::new();
            loop {
                match handle.recv().await {
                    Some(KagEvent::End) | None => break,
                    Some(e) => post.push(e),
                }
            }

            assert!(
                post.iter().any(
                    |e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("done"))
                ),
                "{:?}",
                post
            );
        })
        .await;
    }

    // ── Snapshot tests ────────────────────────────────────────────────────────

    /// Save state at `[l]`, restore from the snapshot, and verify that the
    /// script resumes and completes from the correct position.
    #[tokio::test(flavor = "current_thread")]
    async fn test_snapshot_round_trip() {
        with_local(|| async {
            // Script: set a variable, display "before", wait for click, display "after".
            let src = "[eval exp=\"f.x = 99;\"]\nbefore\n@l\nafter\n";

            // ── Phase 1: run until the click wait, then snapshot ───────────
            let (mut h1, _t1, _) =
                KagInterpreter::spawn_from_source(src, "test.ks").unwrap();

            // Collect events up to and including WaitForClick
            loop {
                match h1.recv().await {
                    Some(KagEvent::WaitForClick { .. }) | None => break,
                    _ => {}
                }
            }

            // Request a snapshot while paused
            h1.send(HostEvent::TakeSnapshot).await.unwrap();
            let snap = loop {
                match h1.recv().await {
                    Some(KagEvent::Snapshot(s)) => break *s,
                    None => panic!("channel closed before snapshot"),
                    _ => {}
                }
            };

            // Verify the snapshot captured f.x = 99
            assert_eq!(snap.f.get("x").and_then(|v| v.as_i64()), Some(99));

            // ── Phase 2: restore and continue ────────────────────────────────
            let (mut h2, _t2, _) =
                KagInterpreter::spawn_from_snapshot(snap, src).unwrap();

            // Resume by clicking
            h2.send(HostEvent::Clicked).await.unwrap();

            let mut got_after = false;
            loop {
                match h2.recv().await {
                    Some(KagEvent::DisplayText { text, .. }) if text.contains("after") => {
                        got_after = true;
                    }
                    Some(KagEvent::End) | None => break,
                    _ => {}
                }
            }
            assert!(got_after, "expected 'after' text after snapshot restore");
        })
        .await;
    }

    /// Snapshot/restore preserves the `sf` (system) and `f` (game) variable
    /// maps across the round-trip.
    #[tokio::test(flavor = "current_thread")]
    async fn test_snapshot_variables_preserved() {
        with_local(|| async {
            let src = "[eval exp=\"f.score = 42; sf.unlocked = true;\"]\n@l\n";

            let (mut h, _t, _) = KagInterpreter::spawn_from_source(src, "test.ks").unwrap();

            loop {
                match h.recv().await {
                    Some(KagEvent::WaitForClick { .. }) | None => break,
                    _ => {}
                }
            }

            h.send(HostEvent::TakeSnapshot).await.unwrap();
            let snap = loop {
                match h.recv().await {
                    Some(KagEvent::Snapshot(s)) => break *s,
                    None => panic!("no snapshot"),
                    _ => {}
                }
            };

            assert_eq!(snap.f.get("score").and_then(|v| v.as_i64()), Some(42));
            assert_eq!(
                snap.sf.get("unlocked").and_then(|v| v.as_bool()),
                Some(true)
            );

            // Round-trip through JSON
            let json = serde_json::to_string(&snap).expect("serialize");
            let snap2: crate::snapshot::InterpreterSnapshot =
                serde_json::from_str(&json).expect("deserialize");

            assert_eq!(snap2.f.get("score").and_then(|v| v.as_i64()), Some(42));
        })
        .await;
    }
}
