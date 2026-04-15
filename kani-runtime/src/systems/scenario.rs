//! Scenario-file loading helper used by the `poll_interpreter` system.

use anyhow::{Context as _, Result};
use kag_interpreter::HostEvent;
use tokio::sync::mpsc::Sender;

use crate::asset::AssetBackend;

/// Load the `.ks` file named `storage` from `backend` and immediately send a
/// `HostEvent::ScenarioLoaded` back to the interpreter.
///
/// Errors are returned so the caller can log them via Bevy's `error!` macro.
pub fn load_and_send(
    backend: &AssetBackend,
    input_tx: &Sender<HostEvent>,
    storage: &str,
) -> Result<()> {
    let source = backend
        .load_text(storage)
        .with_context(|| format!("reading scenario '{storage}'"))?;

    input_tx
        .try_send(HostEvent::ScenarioLoaded {
            name: storage.to_owned(),
            source,
        })
        .with_context(|| format!("sending ScenarioLoaded for '{storage}'"))?;

    Ok(())
}
