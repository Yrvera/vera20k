//! Read-only live-world fact adapter for behavior-3 Spark collision.
//!
//! The adapter mirrors the native query order without borrowing mutable
//! simulation state or consuming RNG. It returns an owned fact bundle so the
//! Spark owner can release all world borrows before advancing the Scenario RNG.

use thiserror::Error;

use super::spark::{SparkCollisionFacts, SparkMotionStep, lepton_to_cell_trunc};
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainCell;
use crate::rules::foundation::foundation_dimensions;
use crate::rules::ruleset::RuleSet;
use crate::sim::cell_rect::{CELL_ROW_STRIDE, cell_linear_index};
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::world::Simulation;
use crate::util::lepton::{UnsupportedGroundSlope, ground_height_leptons};
use crate::util::native_x87::NativeF32Bits;

const CELL_AXIS_MASK: i64 = CELL_ROW_STRIDE - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SparkWorldError {
    #[error("Spark collision requires resolved terrain")]
    MissingTerrain,
    #[error("Spark collision requires the mutable overlay grid")]
    MissingOverlayGrid,
    #[error("native cell lookup ({x}, {y}) falls outside the fixed cell array")]
    OutOfRangeCell { x: i32, y: i32 },
    #[error(
        "native cell lookup ({x}, {y}) resolves to unavailable canonical cell ({canonical_x}, {canonical_y})"
    )]
    UnavailableCell {
        x: i32,
        y: i32,
        canonical_x: u16,
        canonical_y: u16,
    },
    #[error("overlay state is unavailable at canonical cell ({rx}, {ry})")]
    UnavailableOverlayCell { rx: u16, ry: u16 },
    #[error("slope type {0} is outside the verified 0..=20 table")]
    UnsupportedSlope(u8),
    #[error("structural bridge cell ({rx}, {ry}) has no live BridgeRuntimeState")]
    MissingBridgeRuntimeState { rx: u16, ry: u16 },
    #[error("structural bridge cell ({rx}, {ry}) has no live runtime cell")]
    MissingBridgeRuntimeCell { rx: u16, ry: u16 },
    #[error("cell occupancy references missing entity {0}")]
    MissingOccupantEntity(u64),
    #[error("building entity {0} has no resolved ObjectType")]
    MissingObjectType(u64),
    #[error("building entity {0} needs unmodelled LaserFence connectivity state")]
    UnsupportedLaserFence(u64),
}

/// Concrete read-only view over the live simulation state Spark collision reads.
pub struct SparkCollisionWorld<'a> {
    sim: &'a Simulation,
    rules: &'a RuleSet,
}

impl<'a> SparkCollisionWorld<'a> {
    pub fn new(sim: &'a Simulation, rules: &'a RuleSet) -> Result<Self, SparkWorldError> {
        if sim.resolved_terrain.is_none() {
            return Err(SparkWorldError::MissingTerrain);
        }
        if sim.overlay_grid.is_none() {
            return Err(SparkWorldError::MissingOverlayGrid);
        }
        Ok(Self { sim, rules })
    }

    /// Gather every native collision input before the owner borrows mutable RNG.
    pub fn query(&self, motion: SparkMotionStep) -> Result<SparkCollisionFacts, SparkWorldError> {
        let (old_rx, old_ry, old_cell) =
            self.cell_for_world_coords(motion.old_coords.x, motion.old_coords.y)?;
        let (candidate_rx, candidate_ry, candidate_cell) =
            self.cell_for_world_coords(motion.candidate_coords.x, motion.candidate_coords.y)?;

        let old_has_structural_bridge = self.live_structural_bridge(old_rx, old_ry, old_cell)?;
        let candidate_has_structural_bridge =
            self.live_structural_bridge(candidate_rx, candidate_ry, candidate_cell)?;
        let accepted_building = self.accepted_building(candidate_rx, candidate_ry)?;
        let overlay = self
            .sim
            .overlay_grid
            .as_ref()
            .ok_or(SparkWorldError::MissingOverlayGrid)?;
        if candidate_rx >= overlay.width() || candidate_ry >= overlay.height() {
            return Err(SparkWorldError::UnavailableOverlayCell {
                rx: candidate_rx,
                ry: candidate_ry,
            });
        }

        Ok(SparkCollisionFacts {
            ground_z: ground_height_leptons(
                candidate_cell.level,
                candidate_cell.slope_type,
                motion.candidate_coords.x,
                motion.candidate_coords.y,
            )
            .map_err(|UnsupportedGroundSlope(slope)| SparkWorldError::UnsupportedSlope(slope))?,
            slope_matrix: slope_matrix(candidate_cell.slope_type)?,
            old_has_structural_bridge,
            candidate_has_structural_bridge,
            accepted_building,
            wall_overlay_id: overlay.cell(candidate_rx, candidate_ry).overlay_id,
        })
    }

    fn cell_for_world_coords(
        &self,
        world_x: i32,
        world_y: i32,
    ) -> Result<(u16, u16, &ResolvedTerrainCell), SparkWorldError> {
        let x = lepton_to_cell_trunc(world_x);
        let y = lepton_to_cell_trunc(world_y);
        let Some((canonical_x, canonical_y)) = canonical_cell(x, y) else {
            return Err(SparkWorldError::OutOfRangeCell { x, y });
        };
        let terrain = self
            .sim
            .resolved_terrain
            .as_ref()
            .ok_or(SparkWorldError::MissingTerrain)?;
        let cell =
            terrain
                .cell(canonical_x, canonical_y)
                .ok_or(SparkWorldError::UnavailableCell {
                    x,
                    y,
                    canonical_x,
                    canonical_y,
                })?;
        Ok((canonical_x, canonical_y, cell))
    }

    fn live_structural_bridge(
        &self,
        rx: u16,
        ry: u16,
        cell: &ResolvedTerrainCell,
    ) -> Result<bool, SparkWorldError> {
        if !cell.bridge_facts.has_structural_bridge() {
            return Ok(false);
        }
        let state = self
            .sim
            .bridge_state
            .as_ref()
            .ok_or(SparkWorldError::MissingBridgeRuntimeState { rx, ry })?;
        let runtime = state
            .cell(rx, ry)
            .ok_or(SparkWorldError::MissingBridgeRuntimeCell { rx, ry })?;
        Ok(runtime.deck_present)
    }

    fn accepted_building(&self, rx: u16, ry: u16) -> Result<bool, SparkWorldError> {
        let Some(occupancy) = self.sim.occupancy().get(rx, ry) else {
            return Ok(false);
        };
        for occupant in occupancy.iter_layer(MovementLayer::Ground) {
            let entity = self
                .sim
                .entities()
                .get(occupant.entity_id)
                .ok_or(SparkWorldError::MissingOccupantEntity(occupant.entity_id))?;
            if entity.category != EntityCategory::Structure {
                continue;
            }

            // Native stops at the first building in the selected object list.
            let object = self
                .sim
                .object_type(entity.type_ref, self.rules)
                .ok_or(SparkWorldError::MissingObjectType(entity.stable_id))?;
            if object.laser_fence {
                return Err(SparkWorldError::UnsupportedLaserFence(entity.stable_id));
            }
            let undeploys_from_one_by_one = object
                .undeploys_into
                .as_deref()
                .and_then(|target| self.rules.object(target))
                .is_some()
                && foundation_dimensions(&object.foundation) == (1, 1);
            return Ok(!undeploys_from_one_by_one);
        }
        Ok(false)
    }
}

fn canonical_cell(x: i32, y: i32) -> Option<(u16, u16)> {
    let index = cell_linear_index(x, y)?;
    Some((
        (index & CELL_AXIS_MASK) as u16,
        (index / CELL_ROW_STRIDE) as u16,
    ))
}

macro_rules! matrix {
    ($($bits:expr),+ $(,)?) => {
        [$(NativeF32Bits::from_bits($bits)),+]
    };
}

const ZERO_MATRIX: [NativeF32Bits; 12] = matrix![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,];

const SLOPE_MATRICES: [[NativeF32Bits; 12]; 21] = [
    matrix![
        0x3f800000, 0, 0, 0, 0, 0x3f800000, 0, 0, 0, 0, 0x3f800000, 0
    ],
    matrix![
        0x3f5f3969, 0xbaaf51ef, 0xbefaa753, 0, 0x3ac90fd5, 0x3f7fffec, 0x250a3d28, 0, 0x3efaa73f,
        0xba44dce1, 0x3f5f397a, 0
    ],
    matrix![
        0x3f7fffec, 0xbac90fd5, 0x248a3d28, 0, 0x3aaf51ef, 0x3f5f3969, 0x3efaa753, 0, 0xba44dce1,
        0xbefaa73f, 0x3f5f397a, 0
    ],
    matrix![
        0x3f5f3969, 0xbaaf51ef, 0x3efaa753, 0, 0x3ac90fd5, 0x3f7fffec, 0xa48a3d28, 0, 0xbefaa73f,
        0x3a44dce1, 0x3f5f397a, 0
    ],
    matrix![
        0x3f800000, 0, 0, 0, 0, 0x3f5f397a, 0xbefaa753, 0, 0, 0x3efaa753, 0x3f5f397a, 0
    ],
    matrix![
        0x3f773916, 0x3d06934b, 0xbe83d956, 0, 0x3d12b5d0, 0x3f77322f, 0x3e83d956, 0, 0x3e83a585,
        0xbe840d13, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f77322f, 0xbd12b5d0, 0x3e83d956, 0, 0xbd06934b, 0x3f773916, 0x3e83d956, 0, 0xbe840d13,
        0xbe83a585, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f773916, 0x3d06934b, 0x3e83d956, 0, 0x3d12b5d0, 0x3f77322f, 0xbe83d956, 0, 0xbe83a585,
        0x3e840d13, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f77322f, 0xbd12b5d0, 0xbe83d956, 0, 0xbd06934b, 0x3f773916, 0xbe83d956, 0, 0x3e840d13,
        0x3e83a585, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f773916, 0x3d06934b, 0xbe83d956, 0, 0x3d12b5d0, 0x3f77322f, 0x3e83d956, 0, 0x3e83a585,
        0xbe840d13, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f77322f, 0xbd12b5d0, 0x3e83d956, 0, 0xbd06934b, 0x3f773916, 0x3e83d956, 0, 0xbe840d13,
        0xbe83a585, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f773916, 0x3d06934b, 0x3e83d956, 0, 0x3d12b5d0, 0x3f77322f, 0xbe83d956, 0, 0xbe83a585,
        0x3e840d13, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f77322f, 0xbd12b5d0, 0xbe83d956, 0, 0xbd06934b, 0x3f773916, 0xbe83d956, 0, 0x3e840d13,
        0x3e83a585, 0x3f6e6b6d, 0
    ],
    matrix![
        0x3f6fa319, 0x3d80294b, 0xbeb13d26, 0, 0x3d860ad1, 0x3f6f963a, 0x3eb13d26, 0, 0x3eb0f77f,
        0xbeb182b2, 0x3f5f397a, 0
    ],
    matrix![
        0x3f6f963a, 0xbd860ad1, 0x3eb13d26, 0, 0xbd80294b, 0x3f6fa319, 0x3eb13d26, 0, 0xbeb182b2,
        0xbeb0f77f, 0x3f5f397a, 0
    ],
    matrix![
        0x3f6fa319, 0x3d80294b, 0x3eb13d26, 0, 0x3d860ad1, 0x3f6f963a, 0xbeb13d26, 0, 0xbeb0f77f,
        0x3eb182b2, 0x3f5f397a, 0
    ],
    matrix![
        0x3f6f963a, 0xbd860ad1, 0xbeb13d26, 0, 0xbd80294b, 0x3f6fa319, 0xbeb13d26, 0, 0x3eb182b2,
        0x3eb0f77f, 0x3f5f397a, 0
    ],
    ZERO_MATRIX,
    ZERO_MATRIX,
    ZERO_MATRIX,
    ZERO_MATRIX,
];

/// Exact behavior-3 matrix bits selected by CellClass slope type.
pub fn slope_matrix(slope: u8) -> Result<[NativeF32Bits; 12], SparkWorldError> {
    SLOPE_MATRICES
        .get(slope as usize)
        .copied()
        .ok_or(SparkWorldError::UnsupportedSlope(slope))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, BridgeheadAnchorClass,
        DamageState,
    };
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::occupancy::CellListInsertion;
    use crate::sim::overlay_grid::OverlayGrid;

    fn terrain_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
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
            tileset_index: None,
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            is_cliff_redraw: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            overlay_blocks: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: SpeedCostProfile::default(),
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

    fn motion_at(x: i32, y: i32) -> SparkMotionStep {
        SparkMotionStep {
            old_coords: IVec3::new(x, y, 100),
            candidate_coords: IVec3::new(x, y, 100),
            candidate_f32: [NativeF32Bits::POSITIVE_ZERO; 3],
            persistent_velocity: [NativeF32Bits::POSITIVE_ZERO; 3],
            probe_velocity: [NativeF32Bits::POSITIVE_ZERO; 3],
        }
    }

    fn one_cell_sim(cell: ResolvedTerrainCell) -> Simulation {
        let mut sim = Simulation::new();
        sim.resolved_terrain = Some(ResolvedTerrainGrid::from_cells(1, 1, vec![cell]));
        sim.overlay_grid = Some(OverlayGrid::new(1, 1));
        sim
    }

    fn empty_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str("")).unwrap()
    }

    fn add_building(sim: &mut Simulation, id: u64, type_name: &str) {
        let owner = sim.interner.intern("Neutral");
        let type_ref = sim.interner.intern(type_name);
        let entity = GameEntity::new_at_frame_zero_for_test(
            id,
            0,
            0,
            0,
            0,
            owner,
            Health {
                current: 100,
                max: 100,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            0,
            false,
        );
        sim.entities_mut().insert(entity);
        sim.occupancy_mut().add(
            0,
            0,
            id,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
    }

    #[test]
    fn fixed_stride_lookup_preserves_native_aliasing() {
        assert_eq!(canonical_cell(0, 0), Some((0, 0)));
        assert_eq!(canonical_cell(512, -1), Some((0, 0)));
        assert_eq!(canonical_cell(-1, 1), Some((511, 0)));
        assert_eq!(canonical_cell(-1, 0), None);
    }

    #[test]
    fn matrix_table_preserves_exact_aliases_and_zero_rows() {
        assert_eq!(slope_matrix(5), slope_matrix(9));
        assert_eq!(slope_matrix(6), slope_matrix(10));
        assert_eq!(slope_matrix(7), slope_matrix(11));
        assert_eq!(slope_matrix(8), slope_matrix(12));
        for slope in 17..=20 {
            assert_eq!(slope_matrix(slope), Ok(ZERO_MATRIX));
        }
        assert_eq!(slope_matrix(1).unwrap()[0].bits(), 0x3f5f3969);
        assert_eq!(slope_matrix(16).unwrap()[10].bits(), 0x3f5f397a);
        assert_eq!(slope_matrix(21), Err(SparkWorldError::UnsupportedSlope(21)));
    }

    #[test]
    fn live_bridge_fact_is_static_stamp_and_runtime_deck_state() {
        let mut cell = terrain_cell(0, 0);
        cell.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
        let mut sim = one_cell_sim(cell);
        let mut bridge = BridgeRuntimeState::default();
        bridge.test_seed_cell(
            0,
            0,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 4,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0x18,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
        sim.bridge_state = Some(bridge);
        let rules = empty_rules();

        let intact = SparkCollisionWorld::new(&sim, &rules)
            .unwrap()
            .query(motion_at(0, 0))
            .unwrap();
        assert!(intact.old_has_structural_bridge);
        assert!(intact.candidate_has_structural_bridge);

        sim.bridge_state
            .as_mut()
            .unwrap()
            .cell_mut(0, 0)
            .unwrap()
            .deck_present = false;
        let collapsed = SparkCollisionWorld::new(&sim, &rules)
            .unwrap()
            .query(motion_at(0, 0))
            .unwrap();
        assert!(!collapsed.old_has_structural_bridge);
        assert!(!collapsed.candidate_has_structural_bridge);
    }

    #[test]
    fn world_query_uses_fixed_stride_alias_for_real_cell_data() {
        let mut cell = terrain_cell(0, 0);
        cell.level = 2;
        let sim = one_cell_sim(cell);
        let rules = empty_rules();
        let aliased = SparkCollisionWorld::new(&sim, &rules)
            .unwrap()
            .query(motion_at(512 * 256, -256))
            .unwrap();
        assert_eq!(aliased.ground_z, 208);
    }

    #[test]
    fn first_building_exclusion_does_not_scan_later_buildings() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n1=Vehicle\n\
             [BuildingTypes]\n1=Deployable\n2=Solid\n3=Fence\n\
             [Vehicle]\n\
             [Deployable]\nUndeploysInto=Vehicle\nFoundation=1x1\n\
             [Solid]\nFoundation=2x2\n\
             [Fence]\nLaserFence=yes\nFoundation=1x1\n",
        ))
        .unwrap();

        let mut ordered = one_cell_sim(terrain_cell(0, 0));
        add_building(&mut ordered, 1, "Deployable");
        add_building(&mut ordered, 2, "Solid");
        let world = SparkCollisionWorld::new(&ordered, &rules).unwrap();
        assert_eq!(world.accepted_building(0, 0), Ok(false));

        let mut solid = one_cell_sim(terrain_cell(0, 0));
        add_building(&mut solid, 2, "Solid");
        let world = SparkCollisionWorld::new(&solid, &rules).unwrap();
        assert_eq!(world.accepted_building(0, 0), Ok(true));

        let mut fence = one_cell_sim(terrain_cell(0, 0));
        add_building(&mut fence, 3, "Fence");
        let world = SparkCollisionWorld::new(&fence, &rules).unwrap();
        assert_eq!(
            world.accepted_building(0, 0),
            Err(SparkWorldError::UnsupportedLaserFence(3))
        );
    }
}
