//! Live CellClass overlay identity/state surface for fresh map finalization.
//!
//! The authored reader mutates this surface synchronously in native fixed-grid
//! lookup order. Only after OverlayData, the shared drain, and the first live
//! Recalc sweep does it move a linear payload into the simulation OverlayGrid.

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::{ResolvedTerrainGrid, SharedCellDummy};

pub(crate) const NO_OVERLAY_IDENTITY: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FinalizedOverlayCell {
    identity: i32,
    state: u8,
}

impl Default for FinalizedOverlayCell {
    fn default() -> Self {
        Self {
            identity: NO_OVERLAY_IDENTITY,
            state: 0,
        }
    }
}

impl FinalizedOverlayCell {
    pub(crate) const fn identity(self) -> i32 {
        self.identity
    }

    pub(crate) const fn overlay_id(self) -> Option<u8> {
        if self.identity >= 0 && self.identity <= u8::MAX as i32 - 1 {
            Some(self.identity as u8)
        } else {
            None
        }
    }

    pub(crate) const fn state(self) -> u8 {
        self.state
    }
}

/// Consumed-once final identity/state/authored-wall-count authority.
/// Deliberately not `Clone`.
#[derive(Debug)]
pub(crate) struct FinalizedOverlayPayload {
    width: u16,
    height: u16,
    cells: Vec<FinalizedOverlayCell>,
    authored_wall_neighbor_counts: Vec<u8>,
}

impl FinalizedOverlayPayload {
    pub(crate) fn into_parts(self) -> (u16, u16, Vec<FinalizedOverlayCell>, Vec<u8>) {
        (
            self.width,
            self.height,
            self.cells,
            self.authored_wall_neighbor_counts,
        )
    }

    #[cfg(test)]
    pub(crate) fn from_cells_for_test(
        width: u16,
        height: u16,
        cells: Vec<(i32, u8)>,
        authored_wall_neighbor_counts: Vec<u8>,
    ) -> Self {
        let expected = usize::from(width) * usize::from(height);
        assert_eq!(cells.len(), expected);
        assert_eq!(authored_wall_neighbor_counts.len(), expected);
        Self {
            width,
            height,
            cells: cells
                .into_iter()
                .map(|(identity, state)| FinalizedOverlayCell { identity, state })
                .collect(),
            authored_wall_neighbor_counts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeOverlayCellTarget {
    Real(usize),
    Dummy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuthoredOverlayCellRef {
    pub(crate) target: NativeOverlayCellTarget,
    pub(crate) coord: (i16, i16),
}

/// Synchronous wall-local effects in native execution order. Callers must
/// apply each effect before this transaction advances to the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthoredWallEffect {
    TacticalRadarDirty(AuthoredOverlayCellRef),
    CleanupRecalcAndZone(AuthoredOverlayCellRef),
    BlockerCountIncrement(AuthoredOverlayCellRef),
    CommonAnchorRecalc(AuthoredOverlayCellRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthoredWallMarkResult {
    Completed,
    RejectedUnallocatedAnchor,
    RejectedSteepSlope,
    RejectedNonWallType,
}

/// Mutable, load-local native overlay cell surface. It cannot escape except by
/// moving its real-cell values into `FinalizedOverlayPayload`.
#[derive(Debug)]
pub(crate) struct LiveOverlayCells {
    width: u16,
    height: u16,
    cells: Vec<FinalizedOverlayCell>,
    /// Exact real-cell `CellClass+0x122` contribution made by authored walls.
    /// True-dummy increments are output-inert and deliberately not exported.
    authored_wall_neighbor_counts: Vec<u8>,
    shared_dummy: SharedCellDummy,
}

impl LiveOverlayCells {
    pub(crate) fn empty_for_terrain(terrain: &ResolvedTerrainGrid) -> Self {
        let width = terrain.width();
        let height = terrain.height();
        Self {
            width,
            height,
            cells: vec![FinalizedOverlayCell::default();
                usize::from(width) * usize::from(height)],
            authored_wall_neighbor_counts: vec![0; usize::from(width) * usize::from(height)],
            shared_dummy: terrain.shared_cell_dummy(),
        }
    }

    /// `MapClass::Get_CellClass` narrows both operands to signed words before
    /// sign-extending `y * 512 + x`. A true miss stamps only dummy coordinates.
    pub(crate) fn target(
        &self,
        terrain: &ResolvedTerrainGrid,
        x: i16,
        y: i16,
    ) -> NativeOverlayCellTarget {
        if let Some(index) = terrain.native_fixed_cell_index(x, y) {
            NativeOverlayCellTarget::Real(index)
        } else {
            self.shared_dummy.stamp_coord(i32::from(x), i32::from(y));
            NativeOverlayCellTarget::Dummy
        }
    }

    pub(crate) fn read(&self, target: NativeOverlayCellTarget) -> FinalizedOverlayCell {
        match target {
            NativeOverlayCellTarget::Real(index) => self.cells[index],
            NativeOverlayCellTarget::Dummy => {
                let (identity, state) = self.shared_dummy.overlay_identity_state();
                FinalizedOverlayCell { identity, state }
            }
        }
    }

    pub(crate) fn write_identity(
        &mut self,
        target: NativeOverlayCellTarget,
        identity: i32,
    ) {
        match target {
            NativeOverlayCellTarget::Real(index) => self.cells[index].identity = identity,
            NativeOverlayCellTarget::Dummy => {
                self.shared_dummy.write_overlay_identity(identity);
            }
        }
    }

    pub(crate) fn write_state(&mut self, target: NativeOverlayCellTarget, state: u8) {
        match target {
            NativeOverlayCellTarget::Real(index) => self.cells[index].state = state,
            NativeOverlayCellTarget::Dummy => self.shared_dummy.write_overlay_state(state),
        }
    }

    pub(crate) fn write(
        &mut self,
        target: NativeOverlayCellTarget,
        identity: i32,
        state: u8,
    ) {
        match target {
            NativeOverlayCellTarget::Real(index) => {
                self.cells[index] = FinalizedOverlayCell { identity, state };
            }
            NativeOverlayCellTarget::Dummy => {
                self.shared_dummy
                    .write_overlay_identity_state(identity, state);
            }
        }
    }

    fn cell_ref(&self, terrain: &ResolvedTerrainGrid, x: i16, y: i16) -> AuthoredOverlayCellRef {
        let target = self.target(terrain, x, y);
        let coord = self.coord_for_target(terrain, target, (x, y));
        AuthoredOverlayCellRef { target, coord }
    }

    fn coord_for_target(
        &self,
        terrain: &ResolvedTerrainGrid,
        target: NativeOverlayCellTarget,
        dummy_fallback: (i16, i16),
    ) -> (i16, i16) {
        match target {
            NativeOverlayCellTarget::Real(index) => terrain
                .cells
                .get(index)
                .map(|cell| (cell.rx as i16, cell.ry as i16))
                .expect("native fixed-cell index must address resolved terrain"),
            NativeOverlayCellTarget::Dummy => {
                let coord = self.shared_dummy.snapshot().coord;
                if coord == (i32::from(dummy_fallback.0), i32::from(dummy_fallback.1)) {
                    dummy_fallback
                } else {
                    (coord.0 as i16, coord.1 as i16)
                }
            }
        }
    }

    fn wrapping_increment_authored_wall_count(&mut self, target: NativeOverlayCellTarget) {
        if let NativeOverlayCellTarget::Real(index) = target {
            self.authored_wall_neighbor_counts[index] =
                self.authored_wall_neighbor_counts[index].wrapping_add(1);
        }
    }

    /// Complete the authored `Wall=yes` Mark arm after reader admission and
    /// allocation. Successful Full_Init keeps ScenarioInit nonzero, so there is
    /// intentionally no Rust approximation of the counter-zero build predicate.
    ///
    /// Native evidence: `OverlayClass::Mark @ 0x005FC570`, wall success corridor
    /// `0x005FC6F4..0x005FC775`, and common tail `0x005FD1FA..0x005FD227`.
    pub(crate) fn mark_authored_wall(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        registry: &OverlayTypeRegistry,
        x: i16,
        y: i16,
        overlay_id: u8,
        mut apply_effect: impl FnMut(AuthoredWallEffect),
    ) -> AuthoredWallMarkResult {
        const CARDINAL: [(i16, i16); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        const CLEANUP_CROSS: [(i16, i16); 5] = [(0, -1), (1, 0), (0, 1), (-1, 0), (0, 0)];
        const ADJACENT_8: [(i16, i16); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let anchor = self.cell_ref(terrain, x, y);
        let NativeOverlayCellTarget::Real(anchor_index) = anchor.target else {
            return AuthoredWallMarkResult::RejectedUnallocatedAnchor;
        };
        let anchor_coord = self.coord_for_target(terrain, anchor.target, anchor.coord);
        if terrain.cells[anchor_index].slope_type > 4 && overlay_id != 0xB2 {
            return AuthoredWallMarkResult::RejectedSteepSlope;
        }
        if !registry.flags(overlay_id).is_some_and(|flags| flags.wall) {
            return AuthoredWallMarkResult::RejectedNonWallType;
        }

        // Native writes state before compact identity.
        self.write_state(anchor.target, 0);
        self.write_identity(anchor.target, i32::from(overlay_id));

        for (dx, dy) in CLEANUP_CROSS {
            let visit = self.cell_ref(
                terrain,
                anchor_coord.0.wrapping_add(dx),
                anchor_coord.1.wrapping_add(dy),
            );
            apply_effect(AuthoredWallEffect::TacticalRadarDirty(visit));

            let current = self.read(visit.target);
            let Some(current_id) = current.overlay_id() else {
                continue;
            };
            if !registry.flags(current_id).is_some_and(|flags| flags.wall) {
                continue;
            }

            let mut connectivity = 0u8;
            for (bit, (neighbor_dx, neighbor_dy)) in CARDINAL.into_iter().enumerate() {
                // A real CellClass keeps its own packed coordinate. The shared
                // dummy does not: every miss restamps that same object's +0x24,
                // so re-read its coordinate before each native Adjacent_Cell.
                let base = self.coord_for_target(terrain, visit.target, visit.coord);
                let neighbor = self.cell_ref(
                    terrain,
                    base.0.wrapping_add(neighbor_dx),
                    base.1.wrapping_add(neighbor_dy),
                );
                if current.identity != NO_OVERLAY_IDENTITY
                    && self.read(neighbor.target).identity == current.identity
                {
                    connectivity |= 1 << bit;
                }
            }
            self.write_state(visit.target, (current.state & 0xF0) | connectivity);
            let recalc = AuthoredOverlayCellRef {
                target: visit.target,
                coord: self.coord_for_target(terrain, visit.target, visit.coord),
            };
            apply_effect(AuthoredWallEffect::CleanupRecalcAndZone(recalc));
        }

        for (dx, dy) in ADJACENT_8 {
            let neighbor = self.cell_ref(
                terrain,
                anchor_coord.0.wrapping_add(dx),
                anchor_coord.1.wrapping_add(dy),
            );
            self.wrapping_increment_authored_wall_count(neighbor.target);
            apply_effect(AuthoredWallEffect::BlockerCountIncrement(neighbor));
        }

        apply_effect(AuthoredWallEffect::CommonAnchorRecalc(anchor));
        AuthoredWallMarkResult::Completed
    }

    pub(crate) fn finish(self) -> FinalizedOverlayPayload {
        FinalizedOverlayPayload {
            width: self.width,
            height: self.height,
            cells: self.cells,
            authored_wall_neighbor_counts: self.authored_wall_neighbor_counts,
        }
    }

    #[cfg(test)]
    pub(crate) fn real_cell(&self, index: usize) -> FinalizedOverlayCell {
        self.cells[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, SharedCellDummy, zone_class};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};

    fn flat_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        let land = LandType::Clear.as_index();
        let speed_costs = SpeedCostProfile::default();
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: true,
            tileset_index: None,
            land_type: land,
            yr_cell_land_type: land,
            slope_type: 0,
            template_height: 0,
            height_in_pixels: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs,
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: true,
            allows_tiberium: false,
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
            base_land_type: land,
            base_yr_cell_land_type: land,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: speed_costs,
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn flat_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let cells = (0..height)
            .flat_map(|ry| (0..width).map(move |rx| flat_cell(rx, ry)))
            .collect();
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    fn wall_registry() -> OverlayTypeRegistry {
        let ini = IniFile::from_str(
            "[OverlayTypes]\n0=WALLA\n1=OTHER\n2=WALLB\n\
             [WALLA]\nWall=yes\n\
             [OTHER]\nIsARock=yes\n\
             [WALLB]\nWall=yes\n",
        );
        OverlayTypeRegistry::from_ini(&ini, None)
    }

    fn real(index: usize, coord: (i16, i16)) -> AuthoredOverlayCellRef {
        AuthoredOverlayCellRef {
            target: NativeOverlayCellTarget::Real(index),
            coord,
        }
    }

    #[test]
    fn shared_dummy_overlay_pair_persists_across_coordinate_misses_and_resets_on_resize() {
        let dummy = SharedCellDummy::fresh();
        let retained = dummy.clone();
        dummy.write_overlay_identity_state(0x5c, 2);
        dummy.stamp_coord(-510, 2);

        assert!(dummy.same_identity(&retained));
        assert_eq!(retained.overlay_identity_state(), (0x5c, 2));
        assert_eq!(retained.snapshot().coord, (-510, 2));

        dummy.reconstruct_for_map_resize();
        assert_eq!(retained.overlay_identity_state(), (NO_OVERLAY_IDENTITY, 0));
        assert_eq!(retained.snapshot().coord, (0, 0));
    }

    #[test]
    fn payload_retains_signed_identity_and_independent_state_until_consumed() {
        let payload = FinalizedOverlayPayload::from_cells_for_test(
            2,
            1,
            vec![(NO_OVERLAY_IDENTITY, 41), (0xee, 9)],
            vec![7, 11],
        );
        let (width, height, cells, counts) = payload.into_parts();

        assert_eq!((width, height), (2, 1));
        assert_eq!(cells[0].overlay_id(), None);
        assert_eq!(cells[0].state(), 41);
        assert_eq!(cells[1].overlay_id(), Some(0xee));
        assert_eq!(cells[1].state(), 9);
        assert_eq!(counts, vec![7, 11]);
    }

    #[test]
    fn authored_wall_runs_cleanup_counts_and_common_recalc_in_native_order() {
        let terrain = flat_terrain(5, 5);
        let registry = wall_registry();
        let mut live = LiveOverlayCells::empty_for_terrain(&terrain);
        let north = live.target(&terrain, 2, 1);
        let east = live.target(&terrain, 3, 2);
        live.write(north, 0, 0);
        live.write(east, 2, 0);

        let mut effects = Vec::new();
        assert_eq!(
            live.mark_authored_wall(&terrain, &registry, 2, 2, 0, |effect| {
                effects.push(effect)
            }),
            AuthoredWallMarkResult::Completed
        );

        assert_eq!(
            effects,
            vec![
                AuthoredWallEffect::TacticalRadarDirty(real(7, (2, 1))),
                AuthoredWallEffect::CleanupRecalcAndZone(real(7, (2, 1))),
                AuthoredWallEffect::TacticalRadarDirty(real(13, (3, 2))),
                AuthoredWallEffect::CleanupRecalcAndZone(real(13, (3, 2))),
                AuthoredWallEffect::TacticalRadarDirty(real(17, (2, 3))),
                AuthoredWallEffect::TacticalRadarDirty(real(11, (1, 2))),
                AuthoredWallEffect::TacticalRadarDirty(real(12, (2, 2))),
                AuthoredWallEffect::CleanupRecalcAndZone(real(12, (2, 2))),
                AuthoredWallEffect::BlockerCountIncrement(real(7, (2, 1))),
                AuthoredWallEffect::BlockerCountIncrement(real(8, (3, 1))),
                AuthoredWallEffect::BlockerCountIncrement(real(13, (3, 2))),
                AuthoredWallEffect::BlockerCountIncrement(real(18, (3, 3))),
                AuthoredWallEffect::BlockerCountIncrement(real(17, (2, 3))),
                AuthoredWallEffect::BlockerCountIncrement(real(16, (1, 3))),
                AuthoredWallEffect::BlockerCountIncrement(real(11, (1, 2))),
                AuthoredWallEffect::BlockerCountIncrement(real(6, (1, 1))),
                AuthoredWallEffect::CommonAnchorRecalc(real(12, (2, 2))),
            ]
        );
        assert_eq!(live.read(north).state(), 0x04, "north connects south");
        assert_eq!(
            live.read(east).state(),
            0,
            "different wall id stays isolated"
        );
        let anchor = live.target(&terrain, 2, 2);
        assert_eq!(live.read(anchor).state(), 0x01, "anchor connects north");
        for index in [6usize, 7, 8, 11, 13, 16, 17, 18] {
            assert_eq!(live.authored_wall_neighbor_counts[index], 1);
        }
        assert_eq!(live.authored_wall_neighbor_counts[12], 0);
    }

    #[test]
    fn authored_wall_slope_rejects_before_stamp_or_effect() {
        let mut terrain = flat_terrain(3, 3);
        terrain.cell_mut(1, 1).expect("anchor").slope_type = 5;
        let registry = wall_registry();
        let mut live = LiveOverlayCells::empty_for_terrain(&terrain);
        let mut effects = Vec::new();

        assert_eq!(
            live.mark_authored_wall(&terrain, &registry, 1, 1, 0, |effect| {
                effects.push(effect)
            }),
            AuthoredWallMarkResult::RejectedSteepSlope
        );
        assert!(effects.is_empty());
        assert_eq!(live.real_cell(4), FinalizedOverlayCell::default());
        assert!(
            live.authored_wall_neighbor_counts
                .iter()
                .all(|&count| count == 0)
        );
    }

    #[test]
    fn authored_wall_retains_wrapping_alias_counts_across_data_and_low_body_overwrite() {
        let terrain = flat_terrain(512, 2);
        let registry = wall_registry();
        let mut live = LiveOverlayCells::empty_for_terrain(&terrain);
        // West of (0,1) linearizes to fixed slot 511 -> real (511,0).
        live.authored_wall_neighbor_counts[511] = u8::MAX;
        let mut effects = Vec::new();
        assert_eq!(
            live.mark_authored_wall(&terrain, &registry, 0, 1, 0, |effect| {
                effects.push(effect)
            }),
            AuthoredWallMarkResult::Completed
        );
        assert_eq!(live.authored_wall_neighbor_counts[511], 0);
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(
                    effect,
                    AuthoredWallEffect::BlockerCountIncrement(AuthoredOverlayCellRef {
                        target: NativeOverlayCellTarget::Dummy,
                        ..
                    })
                ))
                .count(),
            3,
            "SE, S, and NW are true dummy targets in this fixed-grid fixture"
        );

        let anchor = live.target(&terrain, 0, 1);
        assert_eq!(
            live.read(anchor).state(),
            0,
            "absent data retains Mark state"
        );
        let retained = live.authored_wall_neighbor_counts.clone();
        live.write_state(anchor, 0xA7);
        assert_eq!(live.authored_wall_neighbor_counts, retained);
        live.write(anchor, 0x7A, 2);
        assert_eq!(live.authored_wall_neighbor_counts, retained);

        let (_, _, cells, counts) = live.finish().into_parts();
        assert_eq!(cells[512].overlay_id(), Some(0x7A));
        assert_eq!(cells[512].state(), 2);
        assert_eq!(
            counts, retained,
            "low body overwrite cannot reverse wall counts"
        );
        assert_eq!(
            counts.iter().map(|&count| u32::from(count)).sum::<u32>(),
            4,
            "only five real targets exist and the aliased 255 increment wraps to zero"
        );
    }
}
