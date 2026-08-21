//! Current CellClass facts consumed by retained radar terrain reconstruction.
//!
//! This adapter keeps full surface rebuilds and incremental dirty-cell updates
//! on one presentation-owned read contract. It never owns or serializes map
//! state; successful save load supplies the restored simulation components.

use std::collections::HashMap;

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::BridgeRuntimeState;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::runtime::SimRuntime;

use super::minimap_helpers::{
    OverlayClassification, current_cell_radar_source, minimap_overlay_datum,
    radar_colors_for_tmp_metadata,
};

/// Read-only live CellClass authority for one retained-radar reconstruction.
///
/// An outer `Option` at the projection call distinguishes an initial
/// map/presentation fallback from an installed simulation authority. Missing
/// components inside an installed authority fail absent instead of reviving
/// stale presentation entries.
#[derive(Clone, Copy)]
pub(crate) struct CurrentRadarCellAuthority<'a> {
    resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    bridge_state: Option<&'a BridgeRuntimeState>,
    overlay_grid: Option<&'a OverlayGrid>,
    overlay_registry: Option<&'a OverlayTypeRegistry>,
    rules: Option<&'a RuleSet>,
}

impl<'a> CurrentRadarCellAuthority<'a> {
    pub(crate) fn new(
        resolved_terrain: Option<&'a ResolvedTerrainGrid>,
        bridge_state: Option<&'a BridgeRuntimeState>,
        overlay_grid: Option<&'a OverlayGrid>,
        overlay_registry: Option<&'a OverlayTypeRegistry>,
        rules: Option<&'a RuleSet>,
    ) -> Self {
        Self {
            resolved_terrain,
            bridge_state,
            overlay_grid,
            overlay_registry,
            rules,
        }
    }

    /// Production presentation wiring after the restored simulation commits.
    pub(crate) fn from_runtime(runtime: &'a SimRuntime) -> Self {
        Self::new(
            runtime.simulation.resolved_terrain.as_ref(),
            runtime.simulation.bridge_state.as_ref(),
            runtime.simulation.overlay_grid.as_ref(),
            Some(&runtime.resources.overlay_registry),
            Some(&runtime.resources.rules),
        )
    }

    /// Current tile branch of `CellClass::GetRadarColor @ 0x0047C060`.
    /// Bit 0x2000 selects the first sibling TMP only when the pristine
    /// subimage advertised damaged data and the sibling entered the native
    /// variant chain. Both metadata triples remain retained even though this
    /// active function reads RadarLeft and duplicates it into the raw pair.
    pub(crate) fn tile_radar_colors(
        self,
        rx: u16,
        ry: u16,
        terrain_brightness: f32,
    ) -> Option<([u8; 3], [u8; 3])> {
        let damaged_variant = self
            .bridge_state
            .and_then(|state| state.cell(rx, ry))
            .is_some_and(|cell| cell.damaged_variant);
        let metadata = self
            .resolved_terrain?
            .current_tile_radar_metadata(rx, ry, damaged_variant)?;
        Some(radar_colors_for_tmp_metadata(
            metadata.left,
            metadata.right,
            metadata.valid,
            terrain_brightness,
        ))
    }

    /// Resolve the current source branch for `CellClass::GetRadarColor`.
    ///
    /// Verified active YR sources: `CellClass::GetRadarColor @ 0x0047C060`
    /// reads current terrain occupation, structural bridge state, and current
    /// overlay identity/data. Successful load rebuilds the whole authority via
    /// `FUN_00685120 -> RadarClass::Init @ 0x00655B20` after swizzling current
    /// CellClass state; no saved radar pixels or dirty queue participate.
    pub(super) fn source(
        self,
        rx: u16,
        ry: u16,
        structural_bridge_color: [u8; 3],
        overlay_radar_colors: &HashMap<(u8, u8), [u8; 3]>,
    ) -> Option<([u8; 3], OverlayClassification)> {
        let terrain_object_present = self
            .resolved_terrain
            .and_then(|terrain| terrain.cell(rx, ry))
            .is_some_and(|cell| cell.terrain_object_occupation.is_some());
        // `CellClass+0x140 & 0x100` is the structural-high branch. The
        // restored resolved fact supplies only that bridge-family identity;
        // restored BridgeRuntimeState supplies its current live state. This
        // deliberately excludes generic/low decks, while a destroyed saved
        // high bridge cannot be revived by immutable source-map facts alone.
        let structural_high_identity = self
            .resolved_terrain
            .and_then(|terrain| terrain.cell(rx, ry))
            .is_some_and(|cell| cell.bridge_facts.has_structural_bridge());
        let runtime_bridge_cell = self.bridge_state.and_then(|state| state.cell(rx, ry));
        let structural_bridge_present = structural_high_identity
            && runtime_bridge_cell.is_some_and(|cell| {
                cell.deck_present && BridgeRuntimeState::effective_render_state(cell).is_some()
            });
        let overlay = if structural_high_identity {
            // High walkers keep their current Cell+0x44 identity only in
            // BridgeRuntimeState. OverlayGrid intentionally mirrors low
            // surfaces, so consulting it here can revive stale 0xCD after a
            // restored 0xE7/0xE8 collapse. Native -1 is Rust 0xFF.
            runtime_bridge_cell.and_then(|cell| {
                (cell.overlay_byte != u8::MAX).then(|| {
                    minimap_overlay_datum(
                        rx,
                        ry,
                        cell.overlay_byte,
                        0,
                        self.overlay_registry,
                        self.rules,
                    )
                })
            })
        } else {
            self.overlay_grid.and_then(|grid| {
                let cell = grid.cell(rx, ry);
                cell.overlay_id.map(|overlay_id| {
                    minimap_overlay_datum(
                        rx,
                        ry,
                        overlay_id,
                        cell.overlay_data,
                        self.overlay_registry,
                        self.rules,
                    )
                })
            })
        };
        current_cell_radar_source(
            terrain_object_present,
            structural_bridge_present,
            overlay,
            structural_bridge_color,
            overlay_radar_colors,
        )
    }
}

#[cfg(test)]
#[path = "current_radar_cell_tests.rs"]
mod tests;
