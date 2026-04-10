use anyhow::Result;
use kag_interpreter::HostEvent;
use tokio::sync::mpsc;

use crate::asset::AssetBackend;

pub fn load_and_send_scenario(
    backend: &AssetBackend,
    input_tx: &mpsc::Sender<HostEvent>,
    storage: &str,
) -> Result<()> {
    let source = backend.load_text(storage)?;
    let _ = input_tx.try_send(HostEvent::ScenarioLoaded {
        name: storage.to_string(),
        source,
    });
    Ok(())
}
