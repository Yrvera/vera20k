//! Entity spawning for the Simulation.
//!
//! Handles spawning entities from map data (`spawn_from_map`) and from
//! production (`spawn_object`). All entities are stored in EntityStore only
//! (BTreeMap<u64, GameEntity>).
//!
//! Dependency rules: same as sim/ (depends on rules/, map/; never render/ui/audio/net).

use std::collections::BTreeMap;

use super::{
    PlacementEvidence, RevealOutcome, RevealPosition, RevealRequest, SimSoundEvent, Simulation,
};
use crate::map::entities::{EntityCategory, MapEntity};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::object_type::{FactoryType, ObjectCategory, ObjectType};
use crate::rules::ruleset::RuleSet;
use crate::sim::animation::{Animation, SequenceKind};
use crate::sim::components::{
    BridgeOccupancy, BuildingDown, BuildingUp, HarvestOverlay, Health, VoxelAnimation,
};
use crate::sim::game_entity::GameEntity;
use crate::sim::miner::{Miner, MinerConfig, miner_kind_for_object};
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::production::{ProductionCategory, foundation_dimensions};
use crate::sim::vision::MAX_SIGHT_RANGE;

fn object_uses_voxel(type_id: &str, object: &ObjectType, rules: &RuleSet) -> bool {
    rules
        .art_registry
        .resolve_metadata_entry(type_id, &object.image)
        .map(|entry| entry.voxel)
        .unwrap_or(matches!(
            object.category,
            ObjectCategory::Vehicle | ObjectCategory::Aircraft
        ))
}

impl Simulation {
    /// Spawn entities from parsed map placements into EntityStore.
    pub fn spawn_from_map(
        &mut self,
        entities: &[MapEntity],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
    ) -> u32 {
        self.spawn_from_map_with_resolved(entities, rules, height_map, None)
    }

    pub fn spawn_from_map_with_resolved(
        &mut self,
        entities: &[MapEntity],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
    ) -> u32 {
        let mut count: u32 = 0;

        for map_ent in entities {
            let bridge_spawn = map_ent
                .high
                .then(|| {
                    resolved_terrain
                        .and_then(|terrain| terrain.cell(map_ent.cell_x, map_ent.cell_y))
                        .filter(|cell| cell.bridge_walkable)
                        .map(|cell| cell.bridge_deck_level)
                })
                .flatten();
            if map_ent.high && bridge_spawn.is_none() {
                log::warn!(
                    "Map entity {} at ({},{}) requested HIGH spawn but no bridge deck was resolved; falling back to ground",
                    map_ent.type_id,
                    map_ent.cell_x,
                    map_ent.cell_y
                );
            }
            let z: u8 = bridge_spawn.unwrap_or_else(|| {
                height_map
                    .get(&(map_ent.cell_x, map_ent.cell_y))
                    .copied()
                    .unwrap_or(0)
            });

            let max_health: u16 = rules
                .and_then(|r| r.object(&map_ent.type_id))
                .map(|obj| obj.strength as u16)
                .unwrap_or(map_ent.health);

            let health = Health {
                current: if max_health > 0 {
                    // map_ent.health is 0-256 where 256 = 100%. Convert to absolute HP.
                    ((map_ent.health as u32 * max_health as u32) / 256) as u16
                } else {
                    map_ent.health
                },
                max: if max_health > 0 {
                    max_health
                } else {
                    map_ent.health
                },
            };

            let uses_voxel_default: bool = match map_ent.category {
                EntityCategory::Unit | EntityCategory::Aircraft => true,
                EntityCategory::Infantry | EntityCategory::Structure => false,
            };
            let uses_voxel: bool = rules
                .and_then(|rules| {
                    rules
                        .object(&map_ent.type_id)
                        .map(|object| object_uses_voxel(&map_ent.type_id, object, rules))
                })
                .unwrap_or(uses_voxel_default);

            let sight_range = rules
                .and_then(|r| r.object(&map_ent.type_id))
                .map(|obj| (obj.sight.max(0) as u16).min(MAX_SIGHT_RANGE))
                .unwrap_or_else(|| Self::default_vision_range_for_category(map_ent.category));

            let stable_id = self.allocate_stable_id();
            let owner_id = self.interner.intern(&map_ent.owner);
            let type_id = self.interner.intern(&map_ent.type_id);

            // Build the GameEntity with all required fields.
            let mut ge = GameEntity::new_at_frame(
                stable_id,
                map_ent.cell_x,
                map_ent.cell_y,
                z,
                map_ent.facing,
                owner_id,
                health,
                type_id,
                map_ent.category,
                map_ent.veterancy,
                sight_range,
                uses_voxel,
                self.session.binary_frame,
            );

            if self.debug_event_logging {
                ge.debug_log = Some(crate::sim::debug_event_log::DebugEventLog::new());
            }

            // Turret facing for voxel units with Turret=yes.
            let obj = rules.and_then(|r| r.object(&map_ent.type_id));
            stamp_scoring_flags(&mut ge, obj);
            let has_turret = obj.map(|o| o.has_turret).unwrap_or(false);
            if has_turret {
                let initial = crate::sim::movement::turret::body_facing_to_turret(map_ent.facing);
                let rot_byte = obj.map(|o| o.turret_rot.clamp(0, 0xFF) as u8).unwrap_or(5);
                ge.barrel_facing = Some(crate::sim::movement::FacingClass::new(initial, rot_byte));
            }
            // The effective art metadata, not the rules category, selects the
            // mutually exclusive VXL or SHP animation carrier when available.
            if uses_voxel {
                ge.voxel_animation = Some(VoxelAnimation::new(1, 1));
            }
            // Infantry animation and sub-cell position.
            if map_ent.category == EntityCategory::Infantry {
                ge.animation = Some(Animation::new(SequenceKind::Stand));
                ge.sub_cell = Some(map_ent.sub_cell);
                let (lx, ly) = crate::util::lepton::subcell_lepton_offset(Some(map_ent.sub_cell));
                ge.position.sub_x = lx;
                ge.position.sub_y = ly;
            }
            // SHP vehicles (Voxel=no non-infantry units like Dolphin, Terror Drone, Squid)
            // also need Animation for walk/attack frame cycling.
            if !uses_voxel
                && (map_ent.category == EntityCategory::Unit
                    || map_ent.category == EntityCategory::Aircraft)
            {
                ge.animation = Some(Animation::new(SequenceKind::Stand));
            }
            // Crush properties from rules.ini.
            if let Some(obj) = rules.and_then(|r| r.object(&map_ent.type_id)) {
                ge.crushable = obj.crushable;
                ge.deployed_crushable = obj.deployed_crushable;
                ge.omni_crusher = obj.omni_crusher;
                ge.regular_crusher = obj.crusher;
                ge.drive_accelerates = obj.accelerates;
                ge.omni_crush_resistant = obj.omni_crush_resistant;
                ge.immune_to_radiation = obj.immune_to_radiation;
                ge.occupier = obj.occupier;
                if map_ent.category == EntityCategory::Structure && obj.gate {
                    ge.building_gate =
                        Some(crate::sim::game_entity::BuildingGateRuntime::default());
                }
                if map_ent.category == EntityCategory::Structure && obj.bunker {
                    ge.bunker_runtime =
                        Some(crate::sim::docking::bunker_install::BunkerRuntime::idle());
                }
                ge.zfudge_bridge = obj.zfudge_bridge;
                ge.too_big_to_fit_under_bridge = obj.too_big_to_fit_under_bridge;
            }
            // Locomotor for movable entities.
            if let Some(obj) = rules.and_then(|r| r.object(&map_ent.type_id)) {
                if obj.speed > 0 {
                    let flight_level = rules.map_or(1500, |r| r.general.flight_level);
                    let mut loco = LocomotorState::from_object_type(obj, flight_level);
                    if bridge_spawn.is_some() {
                        loco.layer = MovementLayer::Bridge;
                    }
                    ge.locomotor = Some(loco);
                }
            }
            // Bridge occupancy.
            if let Some(deck_level) = bridge_spawn {
                ge.bridge_occupancy = Some(BridgeOccupancy { deck_level });
                ge.on_bridge = true;
            }
            // Miner + harvest overlay.
            let miner_obj = rules.and_then(|r| r.object(&map_ent.type_id));
            let miner_kind = miner_obj.and_then(miner_kind_for_object);
            if let Some(kind) = miner_kind {
                let mcfg: MinerConfig = rules.map(MinerConfig::from_rules).unwrap_or_default();
                let storage = miner_obj.map(|o| o.storage.max(0) as u16).unwrap_or(0);
                ge.miner = Some(Miner::new(kind, &mcfg, storage));
                ge.harvest_overlay = Some(HarvestOverlay {
                    frame: 0,
                    visible: false,
                    elapsed_frames: 0,
                });
            }
            // Passenger cargo for transports and garrisonable buildings.
            if let Some(obj) = rules.and_then(|r| r.object(&map_ent.type_id)) {
                if obj.passengers > 0 {
                    ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                        cargo: crate::sim::passenger::PassengerCargo::new(
                            obj.passengers,
                            obj.size_limit,
                        ),
                    };
                } else if obj.can_be_occupied && obj.max_number_occupants > 0 {
                    // Garrisonable buildings: capacity = MaxNumberOccupants, SizeLimit = 1 (infantry only).
                    ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                        cargo: crate::sim::passenger::PassengerCargo::new(
                            obj.max_number_occupants,
                            1,
                        ),
                    };
                }
            }

            stamp_building_cell_profile(&mut ge, obj);
            // TechnoClass::Init_Managers for map-placed parents.
            if let Some(ruleset) = rules
                && let Some(obj) = ruleset.object(&map_ent.type_id)
            {
                ge.capture_manager =
                    crate::sim::capture_manager::init_capture_manager(obj, ruleset);
                ge.spawn_manager = crate::sim::spawn_manager::init_spawn_manager(
                    obj,
                    ruleset,
                    &mut self.interner,
                    self.session.binary_frame,
                );
            }
            let has_spawn_manager = ge.spawn_manager.is_some();
            let (stable_id, outcome) = self.unlimbo(ge);
            debug_assert!(matches!(outcome, RevealOutcome::Revealed { .. }));
            if let Some(ruleset) = rules {
                self.initialize_cloak_after_unlimbo(stable_id, ruleset);
                self.add_unit_sensor_after_unlimbo(stable_id, ruleset);
                self.add_building_sensor_array_if_powered(stable_id, ruleset);
            }
            if has_spawn_manager && let Some(ruleset) = rules {
                crate::sim::spawn_manager::commit_spawn_manager_pool(self, stable_id, ruleset);
            }
            self.commit_map_placement_mission(stable_id, map_ent.mission);
            count += 1;
        }

        log::info!("Spawned {} entities", count);
        count
    }

    /// Commit the `MISSION=` column a map placement authored, at the position
    /// the scenario reader does it: immediately after the object is unlimboed
    /// onto the map.
    ///
    /// Retail runs `Queue_Mission(<name>, 0)` and then its readiness check plus
    /// `Commence` on the same line, and `Queue` refuses the `-1` sentinel — so
    /// an absent or unrecognised name leaves the object on the mission it was
    /// born with. Because the queue slot is empty at birth, `Commence`'s field
    /// writes are exactly `Assign`'s, which is the verb used here; the residual
    /// is that retail's Unit path can leave the promotion one tick late when
    /// its readiness predicate says no, and this cannot.
    ///
    /// Dispatchable miners keep their spawn-time Harvest override — the native
    /// creation-mission family is its own recorded UNCHECKED and the harvest
    /// FSM needs a truthful `current` from birth.
    fn commit_map_placement_mission(
        &mut self,
        stable_id: u64,
        authored: Option<crate::sim::mission::MissionType>,
    ) {
        if self.commit_spawn_harvest_mission(stable_id) {
            return;
        }
        let Some(mission) = authored else {
            return;
        };
        let now = self.session.binary_frame;
        let _ = self.mission_assign_exact(
            stable_id,
            crate::sim::mission::MissionId::from_known(mission),
            now,
        );
    }

    /// Commit the spawn-time Harvest mission for a freshly stored miner so the
    /// host's mission dispatch has a truthful `current` from birth (cursor
    /// `SearchOre` == the zeroed handler state a fresh Assign writes).
    /// UNCHECKED: the native creation-mission family (Enter_Idle_Mode /
    /// initial-unit assigns, roadmap Track B1) is unverified — this preserves
    /// the legacy spawn-into-SearchOre behavior. Slave Miners are excluded
    /// (their own system drives them; never Harvest-dispatched).
    ///
    /// Returns whether the object was a dispatchable miner, i.e. whether this
    /// call owns its spawn mission.
    fn commit_spawn_harvest_mission(&mut self, stable_id: u64) -> bool {
        let is_dispatchable_miner = self
            .substrate
            .entities
            .get(stable_id)
            .and_then(|e| e.miner.as_ref())
            .is_some_and(|m| m.kind != crate::sim::miner::MinerKind::Slave);
        if !is_dispatchable_miner {
            return false;
        }
        let now = self.session.binary_frame;
        let _ = self.mission_assign_exact(
            stable_id,
            crate::sim::mission::MissionId::from_known(crate::sim::mission::MissionType::Harvest),
            now,
        );
        true
    }

    /// Spawn one object instance (used by production). Returns the stable_id on success.
    pub fn spawn_object(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        rules: &RuleSet,
        height_map: &BTreeMap<(u16, u16), u8>,
    ) -> Option<u64> {
        let z: u8 = height_map.get(&(rx, ry)).copied().unwrap_or(0);
        self.spawn_object_at_height(type_id, owner, rx, ry, facing, z, rules)
    }

    pub(crate) fn spawn_object_at_height(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
    ) -> Option<u64> {
        let obj = rules.object(type_id)?;
        let health = Health {
            current: obj.strength.max(1) as u16,
            max: obj.strength.max(1) as u16,
        };
        let category = match obj.category {
            ObjectCategory::Infantry => EntityCategory::Infantry,
            ObjectCategory::Vehicle => EntityCategory::Unit,
            ObjectCategory::Aircraft => EntityCategory::Aircraft,
            ObjectCategory::Building => EntityCategory::Structure,
        };
        let uses_voxel = object_uses_voxel(type_id, obj, rules);
        let sight_range = (obj.sight.max(0) as u16).min(MAX_SIGHT_RANGE);
        let stable_id = self.allocate_stable_id();
        let owner_iid = self.interner.intern(owner);
        let type_iid = self.interner.intern(type_id);

        let mut ge = GameEntity::new_at_frame(
            stable_id,
            rx,
            ry,
            z,
            facing,
            owner_iid,
            health,
            type_iid,
            category,
            0, // veterancy = rookie for production spawns
            sight_range,
            uses_voxel,
            self.session.binary_frame,
        );

        if self.debug_event_logging {
            ge.debug_log = Some(crate::sim::debug_event_log::DebugEventLog::new());
        }

        stamp_scoring_flags(&mut ge, Some(obj));
        if obj.has_turret {
            let initial = crate::sim::movement::turret::body_facing_to_turret(facing);
            let rot_byte = obj.turret_rot.clamp(0, 0xFF) as u8;
            ge.barrel_facing = Some(crate::sim::movement::FacingClass::new(initial, rot_byte));
        }
        if uses_voxel {
            ge.voxel_animation = Some(VoxelAnimation::new(1, 1));
        }
        if category == EntityCategory::Infantry {
            ge.animation = Some(Animation::new(SequenceKind::Stand));
            ge.sub_cell = Some(self.allocate_infantry_sub_cell(rx, ry));
            let (lx, ly) = crate::util::lepton::subcell_lepton_offset(ge.sub_cell);
            ge.position.sub_x = lx;
            ge.position.sub_y = ly;
        }
        // SHP vehicles also need animation for walk/attack frame cycling.
        if !uses_voxel && (category == EntityCategory::Unit || category == EntityCategory::Aircraft)
        {
            ge.animation = Some(Animation::new(SequenceKind::Stand));
        }
        ge.crushable = obj.crushable;
        ge.deployed_crushable = obj.deployed_crushable;
        ge.omni_crusher = obj.omni_crusher;
        ge.regular_crusher = obj.crusher;
        ge.drive_accelerates = obj.accelerates;
        ge.omni_crush_resistant = obj.omni_crush_resistant;
        ge.immune_to_radiation = obj.immune_to_radiation;
        ge.occupier = obj.occupier;
        if category == EntityCategory::Structure && obj.gate {
            ge.building_gate = Some(crate::sim::game_entity::BuildingGateRuntime::default());
        }
        if category == EntityCategory::Structure && obj.bunker {
            ge.bunker_runtime = Some(crate::sim::docking::bunker_install::BunkerRuntime::idle());
        }
        ge.zfudge_bridge = obj.zfudge_bridge;
        ge.too_big_to_fit_under_bridge = obj.too_big_to_fit_under_bridge;
        if obj.speed > 0 {
            ge.locomotor = Some(LocomotorState::from_object_type(
                obj,
                rules.general.flight_level,
            ));
            if ge.locomotor.as_ref().is_some_and(|locomotor| {
                locomotor.kind == crate::rules::locomotor_type::LocomotorKind::Ship
            }) {
                ge.ship_locomotion = Some(Default::default());
            }
        }
        // Aircraft ammo: set up ammo tracking for aircraft with finite Ammo=.
        if obj.ammo >= 0 && category == EntityCategory::Aircraft {
            ge.aircraft_ammo = Some(crate::sim::docking::aircraft_dock::AircraftAmmo::new(
                obj.ammo,
            ));
        }
        // Initialize aircraft mission for Fly-locomotor aircraft.
        if ge
            .locomotor
            .as_ref()
            .is_some_and(|l| l.kind == crate::rules::locomotor_type::LocomotorKind::Fly)
        {
            ge.aircraft_mission = Some(crate::sim::aircraft::AircraftMission::Idle);
        }

        if let Some(kind) = miner_kind_for_object(obj) {
            let mcfg: MinerConfig = MinerConfig::from_rules(rules);
            let storage = obj.storage.max(0) as u16;
            ge.miner = Some(Miner::new(kind, &mcfg, storage));
            ge.harvest_overlay = Some(HarvestOverlay {
                frame: 0,
                visible: false,
                elapsed_frames: 0,
            });
        }
        // Passenger cargo for transports and garrisonable buildings.
        if obj.passengers > 0 {
            ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                cargo: crate::sim::passenger::PassengerCargo::new(obj.passengers, obj.size_limit),
            };
        } else if obj.can_be_occupied && obj.max_number_occupants > 0 {
            ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                cargo: crate::sim::passenger::PassengerCargo::new(obj.max_number_occupants, 1),
            };
        }

        stamp_building_cell_profile(&mut ge, Some(obj));
        // TechnoClass::Init_Managers — the spawn pool exists iff `Spawns=`
        // resolves. Children are created right after placement, below.
        ge.capture_manager = crate::sim::capture_manager::init_capture_manager(obj, rules);
        ge.spawn_manager = crate::sim::spawn_manager::init_spawn_manager(
            obj,
            rules,
            &mut self.interner,
            self.session.binary_frame,
        );
        let has_spawn_manager = ge.spawn_manager.is_some();
        let (stable_id, outcome) = self.unlimbo(ge);
        debug_assert!(matches!(outcome, RevealOutcome::Revealed { .. }));
        self.initialize_cloak_after_unlimbo(stable_id, rules);
        self.add_unit_sensor_after_unlimbo(stable_id, rules);
        if has_spawn_manager {
            crate::sim::spawn_manager::commit_spawn_manager_pool(self, stable_id, rules);
        }
        self.commit_spawn_harvest_mission(stable_id);
        Some(stable_id)
    }

    /// Create an object in limbo: stored in EntityStore and owner counts, but
    /// not registered in map occupancy. Used by paradrop cargo loading, where
    /// gamemd creates passengers directly into CargoClass without Unlimbo.
    pub(crate) fn spawn_object_limbo_at_height(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
    ) -> Option<u64> {
        let obj = rules.object(type_id)?;
        let health = Health {
            current: obj.strength.max(1) as u16,
            max: obj.strength.max(1) as u16,
        };
        let category = match obj.category {
            ObjectCategory::Infantry => EntityCategory::Infantry,
            ObjectCategory::Vehicle => EntityCategory::Unit,
            ObjectCategory::Aircraft => EntityCategory::Aircraft,
            ObjectCategory::Building => EntityCategory::Structure,
        };
        let uses_voxel = object_uses_voxel(type_id, obj, rules);
        let sight_range = (obj.sight.max(0) as u16).min(MAX_SIGHT_RANGE);
        let stable_id = self.allocate_stable_id();
        let owner_iid = self.interner.intern(owner);
        let type_iid = self.interner.intern(type_id);

        let mut ge = GameEntity::new_at_frame(
            stable_id,
            rx,
            ry,
            z,
            facing,
            owner_iid,
            health,
            type_iid,
            category,
            0,
            sight_range,
            uses_voxel,
            self.session.binary_frame,
        );

        if self.debug_event_logging {
            ge.debug_log = Some(crate::sim::debug_event_log::DebugEventLog::new());
        }

        stamp_scoring_flags(&mut ge, Some(obj));
        if obj.has_turret {
            let initial = crate::sim::movement::turret::body_facing_to_turret(facing);
            let rot_byte = obj.turret_rot.clamp(0, 0xFF) as u8;
            ge.barrel_facing = Some(crate::sim::movement::FacingClass::new(initial, rot_byte));
        }
        if uses_voxel {
            ge.voxel_animation = Some(VoxelAnimation::new(1, 1));
        }
        if category == EntityCategory::Infantry {
            ge.animation = Some(Animation::new(SequenceKind::Stand));
            ge.sub_cell = Some(self.allocate_infantry_sub_cell(rx, ry));
            let (lx, ly) = crate::util::lepton::subcell_lepton_offset(ge.sub_cell);
            ge.position.sub_x = lx;
            ge.position.sub_y = ly;
        }
        if !uses_voxel && (category == EntityCategory::Unit || category == EntityCategory::Aircraft)
        {
            ge.animation = Some(Animation::new(SequenceKind::Stand));
        }
        ge.crushable = obj.crushable;
        ge.deployed_crushable = obj.deployed_crushable;
        ge.omni_crusher = obj.omni_crusher;
        ge.regular_crusher = obj.crusher;
        ge.drive_accelerates = obj.accelerates;
        ge.omni_crush_resistant = obj.omni_crush_resistant;
        ge.immune_to_radiation = obj.immune_to_radiation;
        ge.occupier = obj.occupier;
        if category == EntityCategory::Structure && obj.gate {
            ge.building_gate = Some(crate::sim::game_entity::BuildingGateRuntime::default());
        }
        if category == EntityCategory::Structure && obj.bunker {
            ge.bunker_runtime = Some(crate::sim::docking::bunker_install::BunkerRuntime::idle());
        }
        ge.zfudge_bridge = obj.zfudge_bridge;
        ge.too_big_to_fit_under_bridge = obj.too_big_to_fit_under_bridge;
        if obj.speed > 0 {
            ge.locomotor = Some(LocomotorState::from_object_type(
                obj,
                rules.general.flight_level,
            ));
            if ge.locomotor.as_ref().is_some_and(|locomotor| {
                locomotor.kind == crate::rules::locomotor_type::LocomotorKind::Ship
            }) {
                ge.ship_locomotion = Some(Default::default());
            }
        }
        if obj.ammo >= 0 && category == EntityCategory::Aircraft {
            ge.aircraft_ammo = Some(crate::sim::docking::aircraft_dock::AircraftAmmo::new(
                obj.ammo,
            ));
        }
        if ge
            .locomotor
            .as_ref()
            .is_some_and(|l| l.kind == crate::rules::locomotor_type::LocomotorKind::Fly)
        {
            ge.aircraft_mission = Some(crate::sim::aircraft::AircraftMission::Idle);
        }
        if let Some(kind) = miner_kind_for_object(obj) {
            let mcfg: MinerConfig = MinerConfig::from_rules(rules);
            let storage = obj.storage.max(0) as u16;
            ge.miner = Some(Miner::new(kind, &mcfg, storage));
            ge.harvest_overlay = Some(HarvestOverlay {
                frame: 0,
                visible: false,
                elapsed_frames: 0,
            });
        }
        if obj.passengers > 0 {
            ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                cargo: crate::sim::passenger::PassengerCargo::new(obj.passengers, obj.size_limit),
            };
        } else if obj.can_be_occupied && obj.max_number_occupants > 0 {
            ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                cargo: crate::sim::passenger::PassengerCargo::new(obj.max_number_occupants, 1),
            };
        }

        stamp_building_cell_profile(&mut ge, Some(obj));

        ge.capture_manager = crate::sim::capture_manager::init_capture_manager(obj, rules);

        let stable_id = self.create_limbo(ge);
        self.commit_spawn_harvest_mission(stable_id);
        Some(stable_id)
    }

    /// Store a freshly constructed object in native-style limbo and account for
    /// its owner. Placement is a separate, result-bearing Reveal transaction.
    fn store_spawned_limbo(&mut self, mut ge: GameEntity) -> u64 {
        let stable_id = ge.stable_id;
        let owner = self.interner.resolve(ge.owner).to_string();
        let category = ge.category;

        // This boundary receives newly constructed objects. Make those constructor
        // facts explicit so storage can never imply cell or logic presence.
        ge.lifecycle.object_alive = true;
        ge.lifecycle.in_limbo = true;
        ge.lifecycle.cell_marked = false;
        ge.in_logic_vector = false;
        ge.owned_count_released = false;

        self.substrate.entities.insert(ge);
        self.increment_owned_count(&owner, category);
        stable_id
    }

    /// Spawn an object directly into limbo: stored in EntityStore and owner counts
    /// but NOT registered in the active order or map occupancy. Registration
    /// happens later at reveal/landing (e.g. paradrop drop). Returns the stable id.
    pub(crate) fn create_limbo(&mut self, ge: GameEntity) -> u64 {
        self.store_spawned_limbo(ge)
    }

    /// Store a new object, then place it through ObjectClass-style Reveal:
    /// coordinates commit, Mark(PUT) owns occupancy, and eligible logic
    /// registration happens last. The stored object remains addressable if a
    /// future caller-supplied placement result makes Reveal fail.
    pub(crate) fn unlimbo(&mut self, ge: GameEntity) -> (u64, RevealOutcome) {
        let position = RevealPosition {
            rx: ge.position.rx,
            ry: ge.position.ry,
            z: ge.position.z,
            sub_x: ge.position.sub_x,
            sub_y: ge.position.sub_y,
        };
        let stable_id = self.store_spawned_limbo(ge);
        let outcome = self.try_reveal_entity(
            stable_id,
            RevealRequest {
                position,
                placement: PlacementEvidence::MarkSucceeded,
                logic_eligible: true,
            },
        );
        (stable_id, outcome)
    }

    /// Unit/Infantry constructor cloak ability plus UnitClass::Unlimbo's
    /// exceptional state-2 establishment. Active evidence: constructor copy at
    /// 0x007355B6/0x00517D88 and UnitClass::Unlimbo @ 0x00737BA0.
    fn initialize_cloak_after_unlimbo(&mut self, stable_id: u64, rules: &RuleSet) {
        let Some((category, veterancy, in_playfield, type_ref)) = self
            .substrate
            .entities
            .get(stable_id)
            .map(|entity| {
                (
                    entity.category,
                    entity.veterancy,
                    entity.in_playfield,
                    entity.type_ref,
                )
            })
        else {
            return;
        };
        if !matches!(category, EntityCategory::Unit | EntityCategory::Infantry) {
            return;
        }
        let Some(object) = rules.object(self.interner.resolve(type_ref)) else {
            return;
        };
        let rank_cloak = veterancy >= 100 && object.veteran_cloak
            || veterancy >= 200 && object.elite_cloak;
        if !object.cloakable && !rank_cloak {
            return;
        }
        let mut cloak = crate::sim::cloak_disguise::CloakRuntime::new(
            self.session.binary_frame as i32,
            rules.general.cloaking_stages,
        );
        // Only UnitClass owns the direct Unlimbo state write, and it tests the
        // copied runtime Cloakable byte rather than rank-granted CLOAK.
        if category == EntityCategory::Unit && object.cloakable && !in_playfield {
            cloak.establish_unlimbo_fully_cloaked();
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.cloak = Some(cloak);
        }
    }

    /// Update VoxelAnimation frame_counts for all voxel entities from atlas data.
    ///
    /// Called after the unit atlas is built, since frame counts are only known after
    /// loading HVA files.
    pub fn update_voxel_anim_frame_counts(
        &mut self,
        frame_counts: &std::collections::BTreeMap<(String, crate::sim::components::VxlLayer), u32>,
    ) {
        use crate::sim::components::VxlLayer;

        let keys = self.substrate.entities.keys_sorted();
        let mut updated: u32 = 0;
        for &sid in &keys {
            let Some(entity) = self.substrate.entities.get_mut(sid) else {
                continue;
            };
            let Some(ref mut va) = entity.voxel_animation else {
                continue;
            };
            let max_fc: u32 = [
                VxlLayer::Composite,
                VxlLayer::Body,
                VxlLayer::Turret,
                VxlLayer::Barrel,
            ]
            .iter()
            .filter_map(|layer| {
                frame_counts.get(&(self.interner.resolve(entity.type_ref).to_string(), *layer))
            })
            .copied()
            .max()
            .unwrap_or(1);

            if max_fc > 1 && va.frame_count != max_fc {
                va.frame_count = max_fc;
                updated += 1;
            }
        }
        if updated > 0 {
            log::info!(
                "Updated VoxelAnimation frame_count for {} entities",
                updated
            );
        }
    }

    /// Deploy an MCV entity: despawn it and spawn a construction yard in its place.
    /// Checks that the footprint area is free of other structures and passable terrain
    /// before deploying. Returns false if deployment is blocked.
    pub(crate) fn deploy_mcv(
        &mut self,
        stable_id: u64,
        rules: &RuleSet,
        _height_map: &BTreeMap<(u16, u16), u8>,
    ) -> bool {
        // Read deploy data from EntityStore before mutating.
        let deploy_data = self.substrate.entities.get(stable_id).and_then(|entity| {
            let type_str = self.interner.resolve(entity.type_ref);
            let yard_type = construction_yard_type_for_mcv(type_str, rules)?;
            let yard_obj = rules.object(&yard_type)?;
            let (spawn_rx, spawn_ry) = deploy_origin_from_unit_cell(
                entity.position.rx,
                entity.position.ry,
                &yard_obj.foundation,
            );
            Some((
                entity.owner,
                spawn_rx,
                spawn_ry,
                entity.position.z,
                yard_type.clone(),
                yard_obj.deploy_facing,
                entity.selected,
                yard_obj.foundation.clone(),
                entity.facing,
            ))
        });
        let Some((
            owner_id,
            rx,
            ry,
            z,
            yard_type,
            deploy_facing,
            was_selected,
            foundation,
            source_facing,
        )) = deploy_data
        else {
            return false;
        };

        // Check that all footprint cells are free before deploying.
        let (fw, fh) = foundation_dimensions(&foundation);
        for dy in 0..fh {
            for dx in 0..fw {
                let cell_x = rx.saturating_add(dx);
                let cell_y = ry.saturating_add(dy);
                // Check for existing structures (excluding the MCV itself).
                let occupied = self.substrate.entities.values().any(|e| {
                    // A Dying structure corpse (sold/destroyed earlier in this
                    // command batch) no longer blocks an MCV deploy footprint.
                    if e.dying
                        || e.stable_id == stable_id
                        || e.category != EntityCategory::Structure
                    {
                        return false;
                    }
                    let Some(existing) = self.object_type(e.type_ref, rules) else {
                        return false;
                    };
                    if existing.wall {
                        return false;
                    }
                    let (ew, eh) = foundation_dimensions(&existing.foundation);
                    cell_x >= e.position.rx
                        && cell_x < e.position.rx.saturating_add(ew)
                        && cell_y >= e.position.ry
                        && cell_y < e.position.ry.saturating_add(eh)
                });
                if occupied {
                    log::info!("MCV deploy blocked: structure at ({},{})", cell_x, cell_y,);
                    self.sound_events
                        .push(SimSoundEvent::CannotDeployHere { owner: owner_id });
                    return false;
                }
                // Check terrain build-blocked.
                if self
                    .effective_build_blocked(cell_x, cell_y)
                    .unwrap_or(false)
                {
                    log::info!("MCV deploy blocked: terrain at ({},{})", cell_x, cell_y,);
                    self.sound_events
                        .push(SimSoundEvent::CannotDeployHere { owner: owner_id });
                    return false;
                }
            }
        }

        if source_facing != deploy_facing {
            if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.facing_target = Some(deploy_facing);
                entity.facing = deploy_facing;
                entity.movement_target = None;
            }
            return true;
        }

        // Despawn the MCV.
        self.uninit_with_rules(stable_id, rules);

        // Spawn the construction yard.
        let owner_str = self.interner.resolve(owner_id).to_string();
        let Some(new_sid) =
            self.spawn_object_at_height(&yard_type, &owner_str, rx, ry, 0, z, rules)
        else {
            return false;
        };

        // Set selected and building-up state on the new entity.
        if let Some(ge) = self.substrate.entities.get_mut(new_sid) {
            ge.selected = was_selected;
            ge.building_up = Some(BuildingUp {
                elapsed_ticks: 0,
                total_ticks: 30,
            });
        }

        true
    }

    /// Undeploy a structure back into its mobile unit (e.g. ConYard → MCV).
    /// Reads `UndeploysInto` from rules.ini to determine the spawned unit type.
    /// Starts a reverse build-up animation (`BuildingDown`); the actual unit
    /// spawn happens when the animation completes (see `tick_building_down`).
    pub(crate) fn undeploy_building(&mut self, stable_id: u64, rules: &RuleSet) -> bool {
        // Read undeploy data before mutating.
        let undeploy_data = self.substrate.entities.get(stable_id).and_then(|entity| {
            if !self.can_undeploy_building_runtime(stable_id, rules) {
                return None;
            }
            let type_str = self.interner.resolve(entity.type_ref);
            let unit_type = undeploy_target_for_building(type_str, rules)?;
            let obj = rules.object(type_str)?;
            let (center_rx, center_ry) =
                undeploy_unit_cell(entity.position.rx, entity.position.ry, &obj.foundation);
            Some((
                entity.owner,
                center_rx,
                center_ry,
                entity.position.z,
                unit_type,
                entity.selected,
            ))
        });
        let Some((owner_id, rx, ry, z, unit_type, was_selected)) = undeploy_data else {
            return false;
        };

        // Start the reverse build-up animation instead of instant despawn.
        let unit_type_id = self.interner.intern(&unit_type);
        if let Some(ge) = self.substrate.entities.get_mut(stable_id) {
            ge.building_down = Some(BuildingDown {
                elapsed_ticks: 0,
                total_ticks: 30,
                spawn_type: unit_type_id,
                spawn_owner: owner_id,
                spawn_rx: rx,
                spawn_ry: ry,
                spawn_z: z,
                was_selected,
            });
        }
        true
    }

    pub(crate) fn should_show_undeploy_building_command(
        &self,
        stable_id: u64,
        rules: &RuleSet,
    ) -> bool {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return false;
        };
        let Some(obj) = self.object_type(entity.type_ref, rules) else {
            return false;
        };
        if obj.construction_yard && self.owner_has_building_production_busy(entity.owner) {
            return false;
        }
        self.can_undeploy_building_runtime(stable_id, rules)
    }

    pub(crate) fn can_undeploy_building_runtime(&self, stable_id: u64, rules: &RuleSet) -> bool {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return false;
        };
        if entity.category != EntityCategory::Structure
            || entity.building_up.is_some()
            || entity.building_down.is_some()
        {
            return false;
        }
        let type_str = self.interner.resolve(entity.type_ref);
        let Some(obj) = rules.object(type_str) else {
            return false;
        };
        let Some(target) = obj.undeploys_into.as_deref() else {
            return false;
        };
        if rules.object(target).is_none() {
            return false;
        }
        if !obj.construction_yard {
            return true;
        }
        self.construction_yard_redeploy_core_gate(entity)
    }

    fn construction_yard_redeploy_core_gate(&self, entity: &GameEntity) -> bool {
        if !self.session.game_options.mcv_redeploy || !entity.radio_contacts.is_empty() {
            return false;
        }
        self.houses
            .get(&entity.owner)
            .is_some_and(|house| house.is_human)
    }

    fn owner_has_building_production_busy(&self, owner: crate::sim::intern::InternedId) -> bool {
        // P5d: the registry is the queue-of-record. Busy = an active Building build held OR
        // a non-empty Building tail.
        self.production
            .factory_shadow
            .view(owner, ProductionCategory::Building)
            .is_some_and(|v| v.object.is_some() || !v.queue.is_empty())
    }

    /// Find the next available infantry sub-cell at a given cell position.
    /// Scans existing infantry entities at (rx, ry) and returns the first unused
    /// spot from FUNCTIONAL_SUB_CELLS. Falls back to the first entry if all taken
    /// (caller should have avoided full cells via spawn cell selection).
    fn allocate_infantry_sub_cell(&self, rx: u16, ry: u16) -> u8 {
        let mut occupied: [bool; 5] = [false; 5];
        for entity in self.substrate.entities.values() {
            if !entity.dying
                && entity.position.rx == rx
                && entity.position.ry == ry
                && entity.category == EntityCategory::Infantry
            {
                if let Some(sub) = entity.sub_cell {
                    if (sub as usize) < occupied.len() {
                        occupied[sub as usize] = true;
                    }
                }
            }
        }
        for &spot in &crate::sim::movement::bump_crush::FUNCTIONAL_SUB_CELLS {
            if !occupied[spot as usize] {
                return spot;
            }
        }
        crate::sim::movement::bump_crush::FUNCTIONAL_SUB_CELLS[0]
    }
}

/// Where a deploying unit's building lands, given the cell the unit is standing on.
///
/// gamemd takes a single step north-west, gated on the foundation being larger
/// than 2 in either axis — the dimensions are read separately but only feed one
/// OR, so a 3x3, a 4x4 and a 6x4 all get the same one-cell step and nothing is
/// ever halved. The unit's cell is therefore NOT the footprint's centre for an
/// even-sized building: a 4x4 Construction Yard puts it at local index (1,1),
/// the north-west one of the four middle cells, leaving one cell of yard to the
/// north-west and two to the south-east. That lopsidedness is authentic — it is
/// what a 4x4 with a one-cell step has to look like.
///
/// Verified against gamemd 2026-08-05. See [`undeploy_unit_cell`] for the
/// mirror; the two must stay inverses.
fn deploy_origin_from_unit_cell(unit_rx: u16, unit_ry: u16, foundation: &str) -> (u16, u16) {
    let (width, height) = foundation_dimensions(foundation);
    if width > 2 || height > 2 {
        (unit_rx.saturating_sub(1), unit_ry.saturating_sub(1))
    } else {
        (unit_rx, unit_ry)
    }
}

/// Resolve the deploy target for an MCV-like unit via rules.ini `DeploysInto=`.
fn construction_yard_type_for_mcv(type_id: &str, rules: &RuleSet) -> Option<String> {
    let obj = rules.object(type_id)?;
    let target: &str = obj.deploys_into.as_deref()?;
    rules.object(target)?;
    Some(target.to_string())
}

/// Resolve the undeploy target for a building via rules.ini `UndeploysInto=`.
fn undeploy_target_for_building(type_id: &str, rules: &RuleSet) -> Option<String> {
    let obj = rules.object(type_id)?;
    let target: &str = obj.undeploys_into.as_deref()?;
    rules.object(target)?;
    Some(target.to_string())
}

/// Where the unit reappears when a building undeploys, given the building's
/// north-west footprint cell.
///
/// The exact mirror of [`deploy_origin_from_unit_cell`]: gamemd steps one cell
/// south-east behind the same `> 2` foundation gate, so deploy-then-undeploy
/// returns the vehicle to the cell it started on, for every foundation size.
///
/// This used to add `width / 2`, which is `+2` on the 4x4 Construction Yard
/// against gamemd's `+1`. The two halves were not inverses, so every
/// deploy/undeploy cycle walked the MCV one cell east and one cell south, and
/// the error compounded across cycles until a redeploy could fail on terrain
/// gamemd would never have put the vehicle on. The old name asserted the
/// footprint had a centre cell; a 4x4 does not, and believing it did is what
/// produced the wrong inverse.
///
/// Verified against gamemd 2026-08-05.
fn undeploy_unit_cell(origin_rx: u16, origin_ry: u16, foundation: &str) -> (u16, u16) {
    let (width, height) = foundation_dimensions(foundation);
    if width > 2 || height > 2 {
        (origin_rx.saturating_add(1), origin_ry.saturating_add(1))
    } else {
        (origin_rx, origin_ry)
    }
}

/// Copy the rules-derived scoring flags onto a freshly built entity.
///
/// Every spawn path calls this, so a type that must not appear on the score
/// screen is honored no matter how the object came into the world. The flag is
/// copied rather than looked up later because the score bookkeeping runs in the
/// lifecycle authority, which deliberately holds no `RuleSet` borrow.
fn stamp_scoring_flags(ge: &mut GameEntity, obj: Option<&crate::rules::object_type::ObjectType>) {
    ge.dont_score = obj.is_some_and(|o| o.dont_score);
}

fn stamp_building_cell_profile(
    ge: &mut GameEntity,
    obj: Option<&crate::rules::object_type::ObjectType>,
) {
    let Some(obj) = obj else {
        return;
    };
    ge.foundation = obj.foundation.clone();
    ge.spotlight_capable = obj.has_spotlight;
    if ge.category == EntityCategory::Structure {
        ge.building_hidden_occupancy = Some(obj.hidden_occupancy);
        ge.base_reservation_spacing = obj.base_reservation_spacing;
        ge.determines_waypoint_edge = obj.factory == Some(FactoryType::BuildingType);
    }
}
