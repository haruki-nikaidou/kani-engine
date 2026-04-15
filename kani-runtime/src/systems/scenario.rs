//! Scenario-file loading helper used by the `poll_interpreter` system.

use anyhow::{Context as _, Result, anyhow};
use bevy::log::warn;
use kag_interpreter::HostEvent;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;

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

    match input_tx.try_send(HostEvent::ScenarioLoaded {
        name: storage.to_owned(),
        source,
    }) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            warn!("[kani-runtime] input channel full; ScenarioLoaded for '{storage}' dropped");
        }
        Err(TrySendError::Closed(_)) => {
            return Err(anyhow!(
                "interpreter channel closed while sending ScenarioLoaded for '{storage}'"
            ));
        }
    }

    Ok(())
}
