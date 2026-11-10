//! Kontrol S8 catalog, generated Djay mapping and input-to-MIDI bridge.

mod app_events;
pub mod bridge;
pub mod catalog;
mod mapping;
pub mod screens;
mod strip;
mod translator;

use std::path::Path;

use tracing::info;

pub fn export_mapping(path: &Path) -> anyhow::Result<()> {
    mapping::write(path)?;
    for control in catalog::LOCAL_CONTROLS {
        info!(
            input = control.name,
            reason = control.reason,
            "LOCAL_CONTROL"
        );
    }
    for control in catalog::UNASSIGNED_CONTROLS {
        info!(
            input = control.name,
            reason = control.reason,
            "UNASSIGNED_CONTROL"
        );
    }
    info!(path = %path.display(), "DJAY_MAPPING_EXPORTED");
    Ok(())
}
