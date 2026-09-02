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
/// `AND EDX,0x7` on a copy of the loop counter at `0x00481966..0x00481968`) through the stamping
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
        if neighbor_id.and_then(|id| overlay_registry.tiberium_type_for_overlay(tiberium_types, id))
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
/// neighbour the shared dummy's coordinate retains last. The density lives in
/// the overlay grid alone, as every runtime density write does.
pub(crate) fn run_generated_final_cell_attributes(
    terrain: &ResolvedTerrainGrid,
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
        receipt.germinated_cells += 1;
        receipt.tiberium_value_total = receipt.tiberium_value_total.wrapping_add(value);
    }
    receipt
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::map::basic::{BasicSection, SpecialFlagsSection};
    use crate::map::bridge_facts::BridgeCellFacts;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, zone_class};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
    use crate::sim::world::Simulation;

    /// Storage covering the whole `(8, 8)` native diamond (`x`, `y` in
    /// `1..=15`).
    const STORAGE: u16 = 16;
    const MAP_WIDTH: u16 = 8;
    const MAP_HEIGHT: u16 = 8;

    fn flat_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        let land_type = LandType::Clear.as_index();
        let speed_costs = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            float: Some(100),
            amphibious: Some(100),
            float_beach: Some(100),
            hover: Some(100),
        };
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type,
            yr_cell_land_type: land_type,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs,
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: true,
            allows_tiberium: true,
            height_in_pixels: 0,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: zone_class::GROUND,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: land_type,
            base_yr_cell_land_type: land_type,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: speed_costs,
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn flat_terrain() -> ResolvedTerrainGrid {
        let cells = (0..STORAGE)
            .flat_map(|ry| (0..STORAGE).map(move |rx| flat_cell(rx, ry)))
            .collect();
        ResolvedTerrainGrid::from_cells(STORAGE, STORAGE, cells)
    }

    /// Overlay ids: `ORE` = 0 (outside every native image range, so it falls
    /// back to the first TiberiumClass), `GEM` = 27 (the native Cruentus
    /// `Image=2` range `27..=38`), `ROCK` = 28.
    const ORE: u8 = 0;
    const GEM: u8 = 27;
    const ROCK: u8 = 28;

    fn two_class_rules() -> (RuleSet, OverlayTypeRegistry) {
        let mut ini = String::from(
            "[General]\nTiberiumGrows=yes\nTiberiumSpreads=yes\n\
             [InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
             [Tiberiums]\n0=Riparius\n1=Cruentus\n\
             [Riparius]\nImage=1\nValue=25\nGrowth=2200\nGrowthPercentage=.06\n\
             Spread=2200\nSpreadPercentage=.06\n\
             [Cruentus]\nImage=2\nValue=50\nGrowth=2200\nGrowthPercentage=.06\n\
             Spread=2200\nSpreadPercentage=.06\n\
             [OverlayTypes]\n0=ORE\n",
        );
        for index in 1..GEM {
            ini.push_str(&format!("{index}=FILL{index}\n"));
        }
        ini.push_str(&format!(
            "{GEM}=GEM\n{ROCK}=ROCK\n[ORE]\nTiberium=yes\n[GEM]\nTiberium=yes\n[ROCK]\nIsARock=yes\n"
        ));
        let ini = IniFile::from_str(&ini);
        let rules = RuleSet::from_ini(&ini).expect("germination rules");
        let registry = OverlayTypeRegistry::from_ini(&ini, None);
        assert_eq!(
            registry
                .tiberium_type_for_overlay(&rules.tiberium_types, GEM)
                .map(|id| id.0),
            Some(1),
            "the gem overlay must resolve to the second TiberiumClass"
        );
        (rules, registry)
    }

    /// One 4x4 ore field inside the diamond, a second-class gem beside it, a
    /// rock beside that, and an isolated ore cell.
    fn painted_grid() -> OverlayGrid {
        let mut grid = OverlayGrid::new(STORAGE, STORAGE);
        for ry in 6..=9 {
            for rx in 6..=9 {
                grid.place_overlay(rx, ry, ORE, 5);
            }
        }
        grid.place_overlay(10, 7, GEM, 5);
        grid.place_overlay(10, 8, ROCK, 0);
        grid.place_overlay(12, 12, ORE, 5);
        // The fixture placements are not the pass's runtime dirtiness.
        grid.take_dirty_cells();
        grid
    }

    /// `SpreadCellGerminate(0)`: density from the same-class neighbour count
    /// through `g_OreDensityByNeighborCount`; a different TiberiumClass or a
    /// non-resource overlay never counts; the return sums `(state + 1) * Value`.
    #[test]
    fn generated_pass_rewrites_every_resource_density_from_same_class_neighbours() {
        let (rules, registry) = two_class_rules();
        let terrain = flat_terrain();
        let mut grid = painted_grid();

        let receipt = run_generated_final_cell_attributes(
            &terrain,
            &mut grid,
            &rules.tiberium_types,
            &registry,
            MAP_WIDTH,
            MAP_HEIGHT,
        );

        let mut expected_total = 0i32;
        for ry in 6..=9u16 {
            for rx in 6..=9u16 {
                let on_x_edge = rx == 6 || rx == 9;
                let on_y_edge = ry == 6 || ry == 9;
                let expected = match (on_x_edge, on_y_edge) {
                    (true, true) => 4,
                    (true, false) | (false, true) => 7,
                    (false, false) => 11,
                };
                assert_eq!(
                    grid.cell(rx, ry).overlay_data,
                    expected,
                    "field cell ({rx},{ry})"
                );
                expected_total += (i32::from(expected) + 1) * 25;
            }
        }
        assert_eq!(
            grid.cell(10, 7).overlay_data,
            0,
            "the gem counts no ore neighbours"
        );
        expected_total += 50;
        assert_eq!(grid.cell(10, 8).overlay_data, 0, "the rock is untouched");
        assert_eq!(grid.cell(12, 12).overlay_data, 0, "isolated ore");
        expected_total += 25;
        assert_eq!(
            receipt,
            GeneratedCellAttributesReceipt {
                real_cells: u32::from(MAP_HEIGHT) * (2 * u32::from(MAP_WIDTH) - 1),
                unallocated_cells: 0,
                germinated_cells: 18,
                tiberium_value_total: expected_total,
            }
        );
        assert!(
            grid.take_dirty_cells().is_empty(),
            "a fresh-load density rewrite emits no runtime dirtiness"
        );
    }

    /// A `Get_CellClass` miss stamps the shared dummy and reads its retained
    /// overlay identity, so one same-class dummy identity counts once per
    /// missing direction; the last miss of the last visited receiver is the
    /// coordinate the dummy keeps.
    #[test]
    fn generated_pass_counts_the_shared_dummy_for_missing_neighbours_in_native_order() {
        let (rules, registry) = two_class_rules();
        let terrain = flat_terrain();
        let mut grid = OverlayGrid::new(STORAGE, STORAGE);
        // Last cell of the (8, 8) `CellIterator`: sum 24, x 9..=15 -> (15, 9).
        grid.place_overlay(15, 9, ORE, 5);
        let dummy = terrain.shared_cell_dummy();
        dummy.set_overlay_fields(Some(ORE), 3);

        let receipt = run_generated_final_cell_attributes(
            &terrain,
            &mut grid,
            &rules.tiberium_types,
            &registry,
            MAP_WIDTH,
            MAP_HEIGHT,
        );

        // NE (16,8), E (16,9), SE (16,10) miss the 16-wide storage; the five
        // other neighbours are real, empty cells.
        assert_eq!(
            grid.cell(15, 9).overlay_data,
            ORE_DENSITY_BY_NEIGHBOR_COUNT[3]
        );
        assert_eq!(receipt.germinated_cells, 1);
        assert_eq!(
            receipt.tiberium_value_total,
            (i32::from(ORE_DENSITY_BY_NEIGHBOR_COUNT[3]) + 1) * 25
        );
        assert_eq!(
            terrain.dummy_cell_requested_coord(),
            (16, 10),
            "the SE miss is the last stamp in N, NE, E, SE, S, SW, W, NW order"
        );
        assert_eq!(
            dummy.overlay_fields(),
            (Some(0), 3),
            "germination never writes the dummy's identity or state"
        );
    }

    /// Generator tail order: the growth-then-spread queues are seeded from the
    /// painted densities before `InitCellAttributes(1)` rewrites them, and
    /// nothing rebuilds them afterwards.
    #[test]
    fn generated_queues_are_seeded_before_germination_and_left_alone() {
        let (rules, registry) = two_class_rules();
        let mut sim = Simulation::with_seed(0x0C_0001);
        sim.install_resolved_terrain_for_new_map(flat_terrain());
        let terrain = flat_terrain();
        let mut grid = painted_grid();
        // Painted 0 qualifies for growth (below MaxDensity - 1) and not for
        // spread; the interior germinates to 11, which would invert both.
        grid.cell_mut(7, 7).overlay_data = 0;
        let scenario_before = sim.scenario_rng.state();
        let main_before = sim.main_rng.state();
        let basic = BasicSection::default();
        let special_flags = SpecialFlagsSection::default();

        let stats = crate::sim::runtime::initialize_native_tiberium_queues(
            &mut sim,
            &basic,
            &special_flags,
            &rules,
            &registry,
            Some(&grid),
        )
        .expect("a grid seeds the native queues");
        let queue_bitmaps = |sim: &Simulation| {
            sim.production
                .ore_growth_state
                .native_tiberium_state()
                .classes
                .iter()
                .map(|class| (class.growth_bitmap.clone(), class.spread_bitmap.clone()))
                .collect::<Vec<_>>()
        };
        let bitmaps_before = queue_bitmaps(&sim);
        let receipt = run_generated_final_cell_attributes(
            &terrain,
            &mut grid,
            &rules.tiberium_types,
            &registry,
            MAP_WIDTH,
            MAP_HEIGHT,
        );

        assert_eq!(grid.cell(7, 7).overlay_data, 11);
        assert_eq!(receipt.germinated_cells, 18);
        let (ore_growth, ore_spread) = &bitmaps_before[0];
        assert!(
            ore_growth.contains(&(7, 7)),
            "seeded from the painted density: {stats:?}"
        );
        assert!(!ore_spread.contains(&(7, 7)));
        assert_eq!(
            queue_bitmaps(&sim),
            bitmaps_before,
            "germination rebuilds no queue"
        );
        assert_eq!(sim.scenario_rng.state(), scenario_before);
        assert_eq!(sim.main_rng.state(), main_before);
    }
}
