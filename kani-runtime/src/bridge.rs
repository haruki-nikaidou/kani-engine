use std::thread;
use std::time::{Duration, Instant};

use bevy::prelude::Resource;
use kag_interpreter::{HostEvent, KagEvent, KagInterpreter};
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::task::LocalSet;

use crate::asset::AssetBackend;

#[derive(Resource)]
pub struct InterpreterBridge {
    pub event_rx: mpsc::Receiver<KagEvent>,
    pub input_tx: mpsc::Sender<HostEvent>,
    pub state: BridgeState,
}

#[derive(Debug, Clone)]
pub enum BridgeState {
    Running,
    WaitingClick {
        clear_after: bool,
    },
    WaitingMs {
        deadline: Instant,
    },
    Stopped,
    WaitingCompletion {
        tag: String,
        params: Vec<(String, String)>,
    },
    WaitingChoice,
    WaitingInput {
        var_name: String,
    },
    WaitingTrigger {
        name: String,
    },
    Ended,
}

pub fn spawn_interpreter(entry_script: String, backend: AssetBackend) -> InterpreterBridge {
    let (event_tx, event_rx) = mpsc::channel(128);
    let (input_tx, input_rx) = mpsc::channel(128);

    thread::spawn(move || interpreter_thread_main(entry_script, backend, event_tx, input_rx));

    InterpreterBridge {
        event_rx,
        input_tx,
        state: BridgeState::Running,
    }
}

fn interpreter_thread_main(
    entry_script: String,
    backend: AssetBackend,
    event_tx: mpsc::Sender<KagEvent>,
    input_rx: mpsc::Receiver<HostEvent>,
) {
    let rt = match Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("failed to build tokio runtime for interpreter thread: {err}");
            return;
        }
    };
    let local = LocalSet::new();

    rt.block_on(local.run_until(run_interpreter_loop(
        entry_script,
        backend,
        event_tx,
        input_rx,
    )));
}

async fn run_interpreter_loop(
    entry_script: String,
    backend: AssetBackend,
    event_tx: mpsc::Sender<KagEvent>,
    mut input_rx: mpsc::Receiver<HostEvent>,
) {
    let source = match backend.load_text(&entry_script) {
        Ok(src) => src,
        Err(err) => {
            let _ = event_tx
                .send(KagEvent::Error(format!(
                    "failed to load entry script: {err:#}"
                )))
                .await;
            return;
        }
    };

    let (mut interp, _task, parse_diags) =
        match KagInterpreter::spawn_from_source(&source, &entry_script) {
            Ok(v) => v,
            Err(err) => {
                let _ = event_tx
                    .send(KagEvent::Error(format!(
                        "failed to start interpreter: {err}"
                    )))
                    .await;
                return;
            }
        };

    for diag in parse_diags {
        let _ = event_tx.send(KagEvent::Warning(format!("{diag:?}"))).await;
    }

    loop {
        tokio::select! {
            maybe_host = input_rx.recv() => {
                let Some(host_event) = maybe_host else { break; };
                if interp.send(host_event).await.is_err() {
                    break;
                }
            }
            maybe_event = interp.recv() => {
                let Some(ev) = maybe_event else { break; };
                let is_end = matches!(ev, KagEvent::End);
                if event_tx.send(ev).await.is_err() {
                    break;
                }
                if is_end {
                    break;
                }
            }
        }
    }
}

impl BridgeState {
    pub fn set_wait_ms(&mut self, ms: u64) {
        let now = Instant::now();
        let deadline = now.checked_add(Duration::from_millis(ms)).unwrap_or(now);
        *self = BridgeState::WaitingMs { deadline };
    }
}
