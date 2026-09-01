//! `CellClass::SpreadCellGerminate` without randomization and the generated
//! launch's final `MapClass::InitCellAttributes(1)` ore-density rewrite.
//!
//! Depends on `map::authored_overlay` (native cell-iterator shape),
//! `map::cell_index`, `map::overlay_types`, `map::resolved_terrain`, `rules`,
//! `sim::overlay_grid`, `sim::ore_twinkle`, and `util::direction`; never on
//! render/, ui/, app/, sidebar/, audio/, or net/.

use crate::map::authored_overlay::NativeOverlayMapShape;
use crate::map::cell_index::canonical_cell_coord;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::tiberium_type::TiberiumTypeRegistry;
use crate::sim::ore_twinkle::tiberium_value;
use crate::sim::overlay_grid::OverlayGrid;
use crate::util::direction::DIRECTION_DELTAS;

/// `g_OreDensityByNeighborCount @ 0x0081CD28` (twelve dwords, low bytes):
/// the stored `OverlayData` for a same-class neighbour count modulo
/// `TiberiumClass+0xE4 (MaxDensity)`.
pub(crate) const ORE_DENSITY_BY_NEIGHBOR_COUNT: [u8; 12] = [0, 1, 3, 4, 6, 7, 8, 10, 11, 7, 0, 1];

/// One `SpreadCellGerminate(0)` result for a resource receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GerminatedCell {
    /// New `CellClass+0x11E` (OverlayData) of the receiver.
    pub(crate) density: u8,
    /// Native return `(density + 1) * TiberiumClass+0xB8 (Value)`, signed
    /// wrapping 32-bit.
    pub(crate) value: i32,
}

/// `CellClass::SpreadCellGerminate @ 0x004818E0` with `randomizeType = 0`.
///
/// gamemd-derived (decompiled 2026-09-01): the helper returns 0 without any
/// write when the receiver's `OverlayTypeIndex` (`+0x44`) is -1 or
/// `CellClass::OverlayToTiberiumIndex @ 0x005FDD20` is -1. Otherwise it
/// captures `TiberiumClass+0xB8 (Value)`, resolves all eight
/// `g_DirectionOffsets @ 0x0089F688` neighbours (N, NE, E, SE, S, SW, W, NW;
/// `EDI & 7` at `0x00481968`) through the stamping
/// `MapClass::Get_CellClass @ 0x005657A0` (`0x004819A6`; a miss stamps the
/// shared dummy's coordinate and the read continues on that dummy), counts
/// those whose `OverlayToTiberiumIndex` equals the receiver's, writes
/// `+0x11E = g_OreDensityByNeighborCount[count % MaxDensity]` (`IDIV` on
/// `TiberiumClass+0xE4` at `0x004819CA`), and returns `(data + 1) * Value`.
/// No RNG is drawn for argument 0.
///
/// The caller owns the receiver write and performs each neighbour lookup
/// through `read_neighbor_fields`, including its dummy stamp, so the crate
/// Mark seam and the generated final pass share one helper.
pub(crate) fn spread_cell_germinate_without_randomization(
    tiberium_types: &TiberiumTypeRegistry,
    overlay_registry: &OverlayTypeRegistry,
    receiver_overlay_id: Option<u8>,
    cell: (i16, i16),
    mut read_neighbor_fields: impl FnMut((i16, i16)) -> (Option<u8>, u8),
) -> Option<GerminatedCell> {
    let overlay_id = receiver_overlay_id?;
    let type_id = overlay_registry.tiberium_type_for_overlay(tiberium_types, overlay_id)?;
    let tiberium_type = tiberium_types.get(type_id)?;
    // VERA-internal: the native `IDIV` faults on a zero MaxDensity; no retail
    // TiberiumType sets it to zero.
    if tiberium_type.max_density == 0 {
        return None;
    }
    let mut matching: i32 = 0;
    for (dx, dy) in DIRECTION_DELTAS {
        let neighbor = (
            cell.0.wrapping_add(dx as i16),
            cell.1.wrapping_add(dy as i16),
        );
        let (neighbor_id, _) = read_neighbor_fields(neighbor);
        if neighbor_id
            .and_then(|id| overlay_registry.tiberium_type_for_overlay(tiberium_types, id))
            == Some(type_id)
        {
            matching += 1;
        }
    }
    // At most eight neighbours, so the remainder never leaves the table.
    let index = matching % i32::from(tiberium_type.max_density);
    let density = ORE_DENSITY_BY_NEIGHBOR_COUNT[index as usize];
    Some(GerminatedCell {
        density,
        value: tiberium_value(Some(overlay_id), density, overlay_registry, tiberium_types),
    })
}

/// Logging/test receipt of one generated final `InitCellAttributes(1)` pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GeneratedCellAttributesReceipt {
    /// Real cells the native `CellIterator` visited.
    pub(crate) real_cells: u32,
    /// Iterator coordinates with no allocated cell (never expected on a
    /// generated map; logged so a shape mismatch cannot pass silently).
    pub(crate) unallocated_cells: u32,
    /// Real cells whose overlay resolved to a TiberiumClass and whose density
    /// was rewritten.
    pub(crate) germinated_cells: u32,
    /// The native caller-local wrapping sum of every `SpreadCellGerminate`
    /// return; `RandomMapGenerator::Generate` discards it.
    pub(crate) tiberium_value_total: i32,
}

/// Resolve one native fixed-stride lookup to an allocated real cell of the
/// generated grid; `None` is a `Get_CellClass` miss (the shared dummy).
fn resolve_real_cell(terrain: &ResolvedTerrainGrid, x: i16, y: i16) -> Option<(u16, u16)> {
    terrain
        .native_fixed_cell_index(x, y)
        .and_then(|_| canonical_cell_coord(i32::from(x), i32::from(y)))
}

/// Generated launch tail of `RandomMapGenerator::Generate @ 0x00598960`.
///
/// gamemd-derived: after the generator constructors, its final whole-map
/// `RecalcAttributes(-1)` loop (`0x0059937D`), and the growth-then-spread
/// queue initialization (`TiberiumClass::InitGrowthQueues_All @ 0x00722D00`,
/// `InitSpreadQueues_All @ 0x00722240`), `Generate` calls
/// `MapClass::InitCellAttributes(1) @ 0x00568BB0` (`push 1` at `0x0059943F`,
/// call at `0x0059944C`). For every real cell in `CellIterator` order that
/// pass calls `SpreadCellGerminate(0)` before the cell's own
/// `RecalcAttributes(-1)` and adds the return to a caller-local wrapping
/// total; the return is not stored (the `MapClass+0x134` store belongs to
/// `Full_Init`'s argument-0 call only) and the already initialized queues are
/// not rebuilt.
///
/// The per-cell Recalc after the rewrite is not repeated here:
/// `CellClass::RecalcAttributes @ 0x0047D2B0` never reads `+0x11E`, and the
/// identities the germination reads are already final after the generator's
/// whole-map Recalc, so no attribute changes. The pass's terrain-Anim
/// scalar-delete/recreate is the eager tile-anim set (native ID chronology
/// remains the G10 phase-journal residual). Germination reads only overlay
/// identity and writes only the receiver's density, so its result does not
/// depend on the visiting order; the order still fixes which missing
/// neighbour the shared dummy's coordinate retains last.
pub(crate) fn run_generated_final_cell_attributes(
    terrain: &mut ResolvedTerrainGrid,
    overlay_grid: &mut OverlayGrid,
    tiberium_types: &TiberiumTypeRegistry,
    overlay_registry: &OverlayTypeRegistry,
    map_width: u16,
    map_height: u16,
) -> GeneratedCellAttributesReceipt {
    let dummy = terrain.shared_cell_dummy();
    let mut receipt = GeneratedCellAttributesReceipt::default();
    let shape = NativeOverlayMapShape::new(i32::from(map_width), i32::from(map_height));
    for (x, y) in shape.recalc_cells() {
        let Some((rx, ry)) = resolve_real_cell(terrain, x, y) else {
            receipt.unallocated_cells += 1;
            continue;
        };
        receipt.real_cells += 1;
        let receiver_overlay_id = overlay_grid.cell(rx, ry).overlay_id;
        let germinated = {
            let terrain: &ResolvedTerrainGrid = terrain;
            let overlay_grid: &OverlayGrid = overlay_grid;
            spread_cell_germinate_without_randomization(
                tiberium_types,
                overlay_registry,
                receiver_overlay_id,
                (x, y),
                |(nx, ny)| match resolve_real_cell(terrain, nx, ny) {
                    Some((nrx, nry)) => {
                        let neighbor = overlay_grid.cell(nrx, nry);
                        (neighbor.overlay_id, neighbor.overlay_data)
                    }
                    None => {
                        dummy.stamp_coord(i32::from(nx), i32::from(ny));
                        dummy.overlay_fields()
                    }
                },
            )
        };
        let Some(GerminatedCell { density, value }) = germinated else {
            continue;
        };
        // Direct `CellClass+0x11E` write: no runtime dirtiness on a fresh load.
        overlay_grid.cell_mut(rx, ry).overlay_data = density;
        let _ = terrain.set_runtime_overlay_bridge_state_byte(rx, ry, density);
        receipt.germinated_cells += 1;
        receipt.tiberium_value_total = receipt.tiberium_value_total.wrapping_add(value);
    }
    receipt
}
