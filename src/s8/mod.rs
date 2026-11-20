//! Kontrol S8 catalog, generated Djay mapping and input-to-MIDI bridge.

mod app_events;
pub mod bridge;
pub mod catalog;
mod feedback;
mod led_output;
mod led_palette;
mod led_state;
pub mod leds;
mod mapping;
pub mod output;
pub mod screens;
mod strip;
mod translator;

use std::path::Path;

use tracing::info;

pub fn export_mapping(path: &Path, layout: catalog::Layout) -> anyhow::Result<()> {
    mapping::write(path, layout)?;
    if layout == catalog::Layout::Default {
        explain_default_mapping();
    }
    info!(path = %path.display(), ?layout, "DJAY_MAPPING_EXPORTED");
    Ok(())
}

fn explain_default_mapping() {
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
}
