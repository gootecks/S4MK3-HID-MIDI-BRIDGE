//! Kontrol S8 catalog and generated Djay mapping.

pub mod catalog;
mod mapping;

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
