//! Production host for the Jumpjet cruise kernel ([`jumpjet_flight`]).
//!
//! An airborne Jumpjet that VERA's air adapter holds at full height
//! (`AirMovePhase::Hovering` or `Cruising`) with a movement target is flown by
//! the native per-frame body: `Update_Coordinates_And_Altitude @ 0x0054D0F0`
//! then `State3_Translate @ 0x0054BFF0`, the order `Process @ 0x0054AEC0` uses
//! for a moving locomotor. Arrival hands the owner back to the adapter: native
//! state 4 becomes `Descending`, state 2 (a `BalloonHover=` type or one with a
//! target) becomes `Hovering`.
//!
//! Still on the adapter (residuals owned by the Jumpjet state-machine
//! follow-up): takeoff (state 1, which natively translates while climbing),
//! idle hold and its bob (state 2 without a move), descent and landing checks
//! (states 4 and 0), the cell `AltObject` air slot (`0x004135A0`/`0x00487D70`),
//! the `JumpJetTurn=` hold facing, the planning-token arrival notify
//! (`0x00705D60`, which returns early unless `TechnoClass+0x514` is set) and the
//! grounded reset's vtable `+0xF4` call.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on map/, rules/ and sim/ only.

use super::Simulation;
use crate::map::cell_index::NativeCellIdentity;
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::retail_trig::{AtanTable, TrigTable, required_atan_table, required_math_tables};
use crate::rules::locomotor_type::LocomotorKind;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::Position;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::StringInterner;
use crate::sim::movement::air_movement::AirMovementTickStats;
use crate::sim::movement::ground_pose::{ground_surface_z_at, position_world_xy};
use crate::sim::movement::jumpjet_flight::{
    self, FlightOwnerKind, JumpjetFlightHost, STATE_DESCEND, STATE_HOLD, STATE_TRANSLATE,
};
use crate::sim::movement::jumpjet_movement::JumpjetRuntime;
use crate::sim::movement::locomotor::{AirMovePhase, MovementLayer};
use crate::sim::occupancy::OccupancyGrid;
use crate::util::fixed_math::SimFixed;
use crate::util::lepton::{GROUND_LEVEL_HEIGHT_LEPTONS, ground_height_leptons};

/// `(value + ((value >> 31) & 0xFF)) >> 8`, the signed lepton-to-cell step the
/// native cell lookups use.
fn native_cell(value: i32) -> i16 {
    (value.wrapping_add((value >> 31) & 0xFF) >> 8) as i16
}

struct CruiseHost<'a> {
    frame: u32,
    trig: &'a TrigTable,
    atan: &'a AtanTable,
    terrain: Option<&'a ResolvedTerrainGrid>,
    occupancy: &'a OccupancyGrid,
    entities: &'a EntityStore,
    rules: Option<&'a RuleSet>,
    interner: &'a StringInterner,
    kind: FlightOwnerKind,
    location: [i32; 3],
    on_bridge: bool,
    balloon_hover: bool,
    has_target: bool,
    piggyback_active: bool,
    simple_deployer: bool,
    deploy_to_land: bool,
    body_facing: Option<u16>,
    grounded_reset: bool,
}

impl CruiseHost<'_> {
    /// Level and slope of the cell holding `xy`, through the shared dummy cell
    /// for lookups outside the map (`MapClass::Get_CellClass_At_Coord @ 0x00565730`).
    fn cell_terrain(&self, xy: [i32; 2]) -> (u8, u8) {
        let Some(terrain) = self.terrain else {
            return (0, 0);
        };
        match terrain.native_cell_identity((native_cell(xy[0]), native_cell(xy[1]))) {
            NativeCellIdentity::Real(index) => {
                let cell = &terrain.cells()[index];
                (cell.level, cell.slope_type)
            }
            NativeCellIdentity::Dummy => {
                let dummy = terrain.shared_cell_dummy().snapshot();
                (dummy.level as u8, dummy.slope_type)
            }
        }
    }
}

impl JumpjetFlightHost for CruiseHost<'_> {
    fn binary_frame(&self) -> u32 {
        self.frame
    }

    fn trig(&self) -> &TrigTable {
        self.trig
    }

    fn atan(&self) -> &AtanTable {
        self.atan
    }

    fn owner_kind(&self) -> FlightOwnerKind {
        self.kind
    }

    fn location(&self) -> [i32; 3] {
        self.location
    }

    fn set_location(&mut self, coord: [i32; 3]) {
        self.location = coord;
    }

    fn set_z(&mut self, z: i32) {
        self.location[2] = z;
    }

    fn height_above_ground(&self) -> i32 {
        let xy = [self.location[0], self.location[1]];
        self.location[2]
            .wrapping_sub(ground_surface_z_at(xy, self.on_bridge, self.terrain, None).unwrap_or(0))
    }

    fn on_bridge(&self) -> bool {
        self.on_bridge
    }

    fn grounded_reset(&mut self) {
        self.grounded_reset = true;
        self.on_bridge = false;
    }

    fn floor_height(&self, xy: [i32; 2]) -> i32 {
        ground_surface_z_at(xy, false, self.terrain, None).unwrap_or(0)
    }

    fn cell_high_bridge(&self, xy: [i32; 2]) -> bool {
        self.terrain.is_some_and(|terrain| {
            let cell = terrain.native_cell_identity((native_cell(xy[0]), native_cell(xy[1])));
            terrain.native_cell_flags(cell) & 0x100 != 0
        })
    }

    fn cell_top_height(&self, xy: [i32; 2]) -> i32 {
        let (level, slope) = self.cell_terrain(xy);
        let centre = ground_height_leptons(level, slope, 128, 128).unwrap_or(0);
        let (cx, cy) = (native_cell(xy[0]), native_cell(xy[1]));
        if cx < 0 || cy < 0 {
            return jumpjet_flight::cell_top_height(centre, None, false);
        }
        let (rx, ry) = (cx as u16, cy as u16);
        // `BuildingTypeClass::Dimension2 @ 0x00464AF0`: art `Height=` (`+0xEF4`,
        // written at `0x00461101` from the ART key at `0x0081A7A8`) times
        // `g_HeightFactor` (`0x0089DDB8`). The startup chain `0x0045AFA0..
        // 0x0045B070` sets it to 104 under the native control word
        // (`tools/spatial_oracle/height_factor.json`), the same value as the
        // cell level height.
        let building_height = self
            .occupancy
            .first_building_on_layer(rx, ry, MovementLayer::Ground)
            .map(|id| {
                self.entities
                    .get(id)
                    .zip(self.rules)
                    .and_then(|(building, rules)| {
                        rules
                            .object(self.interner.resolve(building.type_ref()))
                            .map(|object| rules.building_launch_height(object))
                    })
                    .unwrap_or(0)
                    .wrapping_mul(GROUND_LEVEL_HEIGHT_LEPTONS)
            });
        let any_techno = self
            .occupancy
            .get(rx, ry)
            .is_some_and(|cell| !cell.is_empty_on(MovementLayer::Ground));
        jumpjet_flight::cell_top_height(centre, building_height, any_techno)
    }

    fn cell_land_type(&self, xy: [i32; 2]) -> u8 {
        self.terrain.map_or(0, |terrain| {
            match terrain.native_cell_identity((native_cell(xy[0]), native_cell(xy[1]))) {
                NativeCellIdentity::Real(index) => terrain.cells()[index].yr_cell_land_type,
                NativeCellIdentity::Dummy => 0,
            }
        })
    }

    fn balloon_hover(&self) -> bool {
        self.balloon_hover
    }

    fn has_target(&self) -> bool {
        self.has_target
    }

    fn piggyback_active(&self) -> bool {
        self.piggyback_active
    }

    fn piggyback_arrival(&mut self) {}

    fn simple_deployer(&self) -> bool {
        self.simple_deployer
    }

    fn type_flag_6ad(&self) -> bool {
        self.deploy_to_land
    }

    fn set_speed_fraction(&mut self, _fraction_bits: u64) {}

    fn arrival_notify(&mut self) {}

    fn air_slot_taken(&mut self) -> bool {
        false
    }

    fn claim_air_slot(&mut self) {}

    fn scatter_to_random_neighbour(&mut self) {}

    fn snap_body_facing(&mut self, facing: u16) {
        self.body_facing = Some(facing);
    }

    fn hold_target_facing(&self) -> Option<u16> {
        None
    }
}

/// Write a world-lepton coordinate back into the cell/sub-cell position and
/// the exact Z the renderer and range checks read.
fn commit_world_location(position: &mut Position, location: [i32; 3]) {
    let cell_x = i32::from(native_cell(location[0])).max(0);
    let cell_y = i32::from(native_cell(location[1])).max(0);
    position.rx = cell_x as u16;
    position.ry = cell_y as u16;
    position.sub_x = SimFixed::from_num((location[0] - cell_x * 256).clamp(0, 255));
    position.sub_y = SimFixed::from_num((location[1] - cell_y * 256).clamp(0, 255));
    position.exact_z_leptons = Some(location[2]);
}

/// A Jumpjet the native cruise flies this turn: airborne at the adapter's full
/// height with a movement target.
fn cruise_eligible(entity: &crate::sim::game_entity::GameEntity) -> bool {
    entity.locomotor.as_ref().is_some_and(|locomotor| {
        locomotor.kind == LocomotorKind::Jumpjet
            && locomotor.layer == MovementLayer::Air
            && matches!(
                locomotor.air_phase,
                AirMovePhase::Hovering | AirMovePhase::Cruising
            )
            && locomotor.jumpjet_runtime().is_some()
    }) && entity
        .movement_target
        .as_ref()
        .is_some_and(|target| target.final_goal.is_some())
}

impl Simulation {
    /// One native cruise frame for an eligible Jumpjet, or `None` when the
    /// owner is not cruising and the adapter keeps it this turn.
    pub(crate) fn tick_jumpjet_cruise_one(
        &mut self,
        stable_id: u64,
        rules: Option<&RuleSet>,
    ) -> Option<AirMovementTickStats> {
        let frame = self.session.binary_frame;
        let entity = self.substrate.entities.get(stable_id)?;
        if !cruise_eligible(entity) {
            // An interrupted cruise (its move target dropped, or the adapter
            // moved the owner out of the air phases) leaves state 3. Native
            // `Stop_Moving 0x0054B4D0` re-targets a nearby cell and keeps
            // flying; VERA holds at once with no speed, which the adapter then
            // owns.
            let interrupted = entity
                .locomotor
                .as_ref()
                .and_then(|locomotor| locomotor.jumpjet_runtime())
                .is_some_and(|runtime| runtime.phase == STATE_TRANSLATE);
            if interrupted
                && let Some(runtime) = self
                    .substrate
                    .entities
                    .get_mut(stable_id)
                    .and_then(|entity| entity.locomotor.as_mut())
                    .and_then(|locomotor| locomotor.jumpjet_runtime_mut())
            {
                runtime.phase = STATE_HOLD;
                runtime.flight.current_speed_bits = 0;
                runtime.flight.target_speed_bits = 0;
            }
            return None;
        }
        let (phase, flight, location, body_facing, grounded_reset, height) = {
            let entity = self.substrate.entities.get(stable_id)?;
            let locomotor = entity.locomotor.as_ref()?;
            if locomotor.kind != LocomotorKind::Jumpjet
                || locomotor.layer != MovementLayer::Air
                || !matches!(
                    locomotor.air_phase,
                    AirMovePhase::Hovering | AirMovePhase::Cruising
                )
            {
                return None;
            }
            let goal = entity.movement_target.as_ref()?.final_goal?;
            let runtime = locomotor.jumpjet_runtime()?;
            // Infantry Move_To (`0x0054B415`) installs its selected subcell;
            // other owners fly to the cell's centre.
            let destination = if entity.category == EntityCategory::Infantry
                && runtime.moving
                && runtime.destination != JumpjetRuntime::NULL
            {
                [
                    runtime.destination.x,
                    runtime.destination.y,
                    runtime.destination.z,
                ]
            } else {
                [
                    i32::from(goal.0) * 256 + 128,
                    i32::from(goal.1) * 256 + 128,
                    0,
                ]
            };
            let terrain = self.resolved_terrain.as_ref();
            let xy = position_world_xy(&entity.position);
            let z = entity.position.exact_z_leptons.unwrap_or_else(|| {
                ground_surface_z_at(xy, entity.on_bridge, terrain, None)
                    .unwrap_or(0)
                    .wrapping_add(locomotor.altitude.to_num::<i32>())
            });
            let object =
                rules.and_then(|rules| rules.object(self.interner.resolve(entity.type_ref())));
            let (trig, _) = required_math_tables();
            let mut host = CruiseHost {
                frame,
                trig,
                atan: required_atan_table(),
                terrain,
                occupancy: &self.substrate.occupancy,
                entities: &self.substrate.entities,
                rules,
                interner: &self.interner,
                kind: match entity.category {
                    EntityCategory::Unit => FlightOwnerKind::Unit,
                    EntityCategory::Infantry => FlightOwnerKind::Infantry,
                    _ => FlightOwnerKind::Other,
                },
                location: [xy[0], xy[1], z],
                on_bridge: entity.on_bridge,
                balloon_hover: locomotor.balloon_hover,
                has_target: entity.attack_target.is_some(),
                piggyback_active: entity.foot_locomotor_swap_active,
                simple_deployer: object.is_some_and(|object| object.is_simple_deployer),
                deploy_to_land: object.is_some_and(|object| object.deploy_to_land),
                body_facing: None,
                grounded_reset: false,
            };
            let params = runtime.params;
            let mut flight = runtime.flight;
            if !matches!(runtime.phase, STATE_TRANSLATE | STATE_HOLD) {
                // The adapter's takeoff stands in for State 0 (`0x0054B9AA..B9D8`):
                // snap the locomotor facing to the body facing (`+0x388`), zero
                // both speed doubles and seed `+0x80 = Height`. Update outside
                // states 2 and 3 zeroes the bob (`0x0054D22C`). A re-order from a
                // hold keeps them: its speeds were zeroed at arrival and Update
                // copied the locomotor facing to the body every frame.
                let body = entity
                    .body_facing
                    .as_ref()
                    .map_or(u16::from(entity.facing) << 8, |body| body.current(frame));
                flight.facing.snap(body, frame);
                flight.current_speed_bits = 0;
                flight.target_speed_bits = 0;
                flight.bob_phase_bits = 0;
                flight.target_height = params.height;
            }
            jumpjet_flight::update_coordinates_and_altitude(
                STATE_TRANSLATE,
                destination,
                &params,
                &mut flight,
                &mut host,
            );
            let phase =
                jumpjet_flight::state3_translate(destination, &params, &mut flight, &mut host);
            (
                phase,
                flight,
                host.location,
                host.body_facing,
                host.grounded_reset,
                host.height_above_ground(),
            )
        };

        let entity = self.substrate.entities.get_mut(stable_id)?;
        commit_world_location(&mut entity.position, location);
        if grounded_reset {
            entity.on_bridge = false;
        }
        if let Some(facing) = body_facing {
            entity.facing = (facing >> 8) as u8;
            if let Some(body) = entity.body_facing.as_mut() {
                body.snap(facing, frame);
            }
        }
        let arrived = phase != STATE_TRANSLATE;
        if arrived {
            entity.movement_target = None;
        }
        let locomotor = entity.locomotor.as_mut()?;
        locomotor.altitude = SimFixed::from_num(height);
        if arrived {
            locomotor.air_phase = if phase == STATE_DESCEND {
                AirMovePhase::Descending
            } else {
                AirMovePhase::Hovering
            };
        }
        if let Some(runtime) = locomotor.jumpjet_runtime_mut() {
            runtime.flight = flight;
            runtime.phase = phase;
        }
        Some(AirMovementTickStats {
            air_movers: 1,
            arrivals: u32::from(arrived),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::jumpjet_params::JumpjetParams;
    use crate::sim::components::MovementTarget;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::ground_pose::position_world_coord;
    use crate::sim::movement::locomotor::LocomotorState;
    use serde_json::Value;

    fn retail_tables_present() -> bool {
        let (trig, _) = required_math_tables();
        if !trig.matches_retail() || !required_atan_table().matches_retail() {
            // With RA2_DIR set, a mismatched table is a failure, not a skip.
            assert!(
                std::env::var_os("RA2_DIR").is_none(),
                "RA2_DIR is set but the retail sine or atan table does not match"
            );
            eprintln!("skipped: set RA2_DIR to the retail install to run this");
            return false;
        }
        true
    }

    fn native_row(name: &str) -> Value {
        let rows: Value = serde_json::from_str(include_str!(
            "../../../tools/spatial_oracle/jumpjet_flight.json"
        ))
        .expect("corpus parses");
        rows.as_array()
            .expect("rows")
            .iter()
            .find(|row| row["name"] == name)
            .cloned()
            .expect("named row")
    }

    /// A Unit Jumpjet with the corpus BASE type block, fresh from `link`
    /// (state 0, locomotor facing `0x4000`), hovering at 500 at cell (10,10)
    /// with a move to cell (16,10). The terrain is the oracle fixture: level 0
    /// and flat, except cell (9,10) at level 2 with slope 1.
    fn hovering_jumpjet(body_facing: u8) -> Simulation {
        let mut sim = Simulation::new();
        super::super::lifecycle_tests::install_common_raw_terrain(&mut sim, 24, 16, 0, None);
        {
            let cell = sim
                .resolved_terrain
                .as_mut()
                .and_then(|terrain| terrain.cell_mut(9, 10))
                .expect("fixture cell");
            cell.level = 2;
            cell.slope_type = 1;
        }
        let mut entity = GameEntity::test_default(1, "JUMPJETUNIT", "Americans", 10, 10);
        entity.category = EntityCategory::Unit;
        entity.facing = body_facing;
        entity.position.sub_x = SimFixed::from_num(128);
        entity.position.sub_y = SimFixed::from_num(128);
        let mut locomotor = LocomotorState::for_test_kind(LocomotorKind::Jumpjet);
        locomotor
            .jumpjet_runtime_mut()
            .expect("jumpjet runtime")
            .link(&JumpjetParams {
                turn_rate: 4,
                speed: SimFixed::from_num(14),
                climb: 5.0,
                crash: 5.0,
                height: 500,
                accel: 2.0,
                wobbles: 0.15,
                deviation: 40,
                no_wobbles: false,
            });
        locomotor.air_phase = AirMovePhase::Hovering;
        locomotor.altitude = SimFixed::from_num(500);
        locomotor.target_altitude = SimFixed::from_num(500);
        entity.locomotor = Some(locomotor);
        entity.movement_target = Some(MovementTarget {
            path: vec![(16, 10)],
            path_layers: vec![MovementLayer::Air],
            next_index: 0,
            speed: SimFixed::from_num(14),
            final_goal: Some((16, 10)),
            ..Default::default()
        });
        sim.substrate.entities.insert(entity);
        sim
    }

    /// Fly a native row through the production air tick, comparing the owner
    /// position and exact Z every frame, until arrival hands it to descent.
    fn fly_native_row(name: &str, body_facing: u8) {
        let row = native_row(name);
        let frames = row["output"]["frames"].as_array().expect("frames");
        let mut sim = hovering_jumpjet(body_facing);
        for (index, expected) in frames.iter().enumerate() {
            sim.session.binary_frame = 1001 + index as u32;
            let stats = sim.tick_air_movement_with_cell_lists_one(1, None);
            let entity = sim.substrate.entities.get(1).expect("jumpjet");
            let coord = position_world_coord(&entity.position);
            let native: Vec<i64> = expected["coord"]
                .as_array()
                .expect("coord")
                .iter()
                .map(|value| value.as_i64().expect("int"))
                .collect();
            assert_eq!(
                vec![i64::from(coord.x), i64::from(coord.y), i64::from(coord.z)],
                native,
                "{name}: frame {index}"
            );
            assert_eq!(
                u32::from(entity.facing),
                (expected["body_facing"].as_u64().expect("body facing") >> 8) as u32,
                "{name}: body facing, frame {index}"
            );
            let last = index + 1 == frames.len();
            assert_eq!(stats.arrivals, u32::from(last), "{name}: frame {index}");
        }
        let entity = sim.substrate.entities.get(1).expect("jumpjet");
        assert!(entity.movement_target.is_none());
        let locomotor = entity.locomotor.as_ref().expect("locomotor");
        assert_eq!(locomotor.air_phase, AirMovePhase::Descending);
        assert_eq!(
            locomotor.jumpjet_runtime().map(|runtime| runtime.phase),
            Some(STATE_DESCEND)
        );
    }

    /// Production parity for the native `east_cruise` row, owner already facing
    /// east.
    #[test]
    fn production_cruise_flies_the_native_east_cruise_frames() {
        if retail_tables_present() {
            fly_native_row("east_cruise", 0x40);
        }
    }

    /// Production parity for `east_from_west_facing`: the takeoff seed snaps the
    /// locomotor facing to the body facing (west), so the cruise turns around
    /// from there instead of starting from the linked `0x4000`.
    #[test]
    fn production_cruise_turns_from_the_body_facing() {
        if retail_tables_present() {
            fly_native_row("east_from_west_facing", 0xC0);
        }
    }

    /// Dropping the move target mid-cruise leaves state 3 for a held, stopped
    /// locomotor instead of carrying speed into the next order.
    #[test]
    fn an_interrupted_cruise_holds_with_no_speed() {
        let mut sim = hovering_jumpjet(0x40);
        for frame in 0..20 {
            sim.session.binary_frame = 1001 + frame;
            sim.tick_air_movement_with_cell_lists_one(1, None);
        }
        let runtime = |sim: &Simulation| {
            sim.substrate
                .entities
                .get(1)
                .and_then(|entity| entity.locomotor.as_ref())
                .and_then(|locomotor| locomotor.jumpjet_runtime())
                .cloned()
                .expect("runtime")
        };
        assert_eq!(runtime(&sim).phase, STATE_TRANSLATE);
        assert!(runtime(&sim).flight.current_speed() > 0.0);
        sim.substrate
            .entities
            .get_mut(1)
            .expect("jumpjet")
            .movement_target = None;
        sim.session.binary_frame = 1021;
        sim.tick_air_movement_with_cell_lists_one(1, None);
        let held = runtime(&sim);
        assert_eq!(held.phase, STATE_HOLD);
        assert_eq!(held.flight.current_speed_bits, 0);
        assert_eq!(held.flight.target_speed_bits, 0);
    }

    /// `g_HeightFactor`, read from the native startup chain, is the multiplier
    /// the cell top height applies to a building's art `Height=`.
    #[test]
    fn building_height_factor_matches_the_native_startup_value() {
        let native: Value = serde_json::from_str(include_str!(
            "../../../tools/spatial_oracle/height_factor.json"
        ))
        .expect("height factor parses");
        assert_eq!(
            native["height_factor"].as_i64(),
            Some(i64::from(GROUND_LEVEL_HEIGHT_LEPTONS))
        );
    }
}
