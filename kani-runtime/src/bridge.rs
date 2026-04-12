//! Interpreter bridge resource.
//!
//! [`InterpreterBridge`] is a Bevy [`Resource`] that lives on the main thread.
//! It owns the communication channels to/from the interpreter task, which runs
//! on a dedicated OS thread with a current-thread Tokio runtime + `LocalSet`
//! (required because Rhai's `Engine`/`Scope` are `!Send`).

use std::future::pending;
use std::thread;
use std::time::Instant;

use anyhow::{Context as _, Result};
use bevy::prelude::Resource;
use kag_interpreter::{HostEvent, KagEvent, KagInterpreter};
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::sync::mpsc;
use tokio::task::LocalSet;

use crate::asset::AssetBackend;

// ─── Bridge state ─────────────────────────────────────────────────────────────

/// What the interpreter is currently waiting for.
///
/// The Bevy input systems read this each frame to decide which `HostEvent` to
/// send back over `input_tx`.
#[derive(Debug, Clone)]
pub enum BridgeState {
    /// Interpreter is running — no host action needed.
    Running,
    /// Interpreter paused at `[l]` or `[p]`.  Mouse-click → `HostEvent::Clicked`.
    WaitingClick { clear_after: bool },
    /// Interpreter paused at `[wait time=…]`.  Check deadline each frame.
    WaitingMs { deadline: Instant },
    /// Interpreter paused at `[s]`.  Click → `HostEvent::Clicked`.
    Stopped,
    /// Interpreter paused at a `[wa]`/`[wt]`/… completion wait.
    WaitingCompletion { tag: String, params: Vec<(String, String)> },
    /// Interpreter waiting for a choice selection.
    WaitingChoice,
    /// Interpreter waiting for text-input from the player.
    WaitingInput { var_name: String },
    /// Interpreter waiting for a named trigger.
    WaitingTrigger { name: String },
    /// Scenario has ended or the interpreter channel was closed.
    Ended,
}

// ─── Bridge resource ──────────────────────────────────────────────────────────

/// Bevy resource that connects the main thread to the interpreter thread.
///
/// `event_rx` and `input_tx` are `Send` even though `KagInterpreter` itself
/// is not, so this resource is safe to keep in the Bevy world.
#[derive(Resource)]
pub struct InterpreterBridge {
    /// Events arriving from the interpreter (non-blocking `try_recv`).
    pub event_rx: mpsc::Receiver<KagEvent>,
    /// Commands sent to the interpreter (non-blocking `try_send`).
    pub input_tx: mpsc::Sender<HostEvent>,
    /// Current wait state — updated by the `poll_interpreter` system.
    pub state: BridgeState,
}

// ─── Spawn ────────────────────────────────────────────────────────────────────

/// Load `entry_script` from `backend` and spawn the interpreter on a dedicated
/// OS thread.  Returns an [`InterpreterBridge`] ready to insert into the Bevy
/// world.
pub fn spawn_interpreter(entry_script: &str, backend: &AssetBackend) -> Result<InterpreterBridge> {
    let source = backend
        .load_text(entry_script)
        .with_context(|| format!("loading entry script '{entry_script}'"))?;
    let script_name = entry_script.to_owned();

    // Synchronous rendezvous channel so we can hand the channel ends back to
    // the Bevy main thread.
    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<InterpreterBridge>>(0);

    thread::Builder::new()
        .name("kani-interp".into())
        .spawn(move || {
            let rt = match RuntimeBuilder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("tokio runtime build failed: {e}")));
                    return;
                }
            };

            let local = LocalSet::new();
            local.block_on(&rt, async move {
                match KagInterpreter::spawn_from_source(&source, &script_name) {
                    Ok((interp, _join, diags)) => {
                        for d in &diags {
                            eprintln!("[kani-runtime] parse warning: {}", d.message);
                        }
                        let bridge = InterpreterBridge {
                            event_rx: interp.event_rx,
                            input_tx: interp.input_tx,
                            state: BridgeState::Running,
                        };
                        let _ = tx.send(Ok(bridge));
                        // Keep the LocalSet alive so the interpreter task can run.
                        pending::<()>().await;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("interpreter spawn failed: {e}")));
                    }
                }
            });
        })
        .context("spawning interpreter thread")?;

    rx.recv().context("interpreter thread did not send a result")?
}
