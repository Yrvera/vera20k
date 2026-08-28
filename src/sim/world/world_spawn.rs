//! Entity spawning for the Simulation.
//!
//! Handles spawning entities from map data (`spawn_from_map`) and from
//! production (`spawn_object`). All entities are stored in EntityStore only
//! (BTreeMap<u64, GameEntity>).
//!
//! Dependency rules: same as sim/ (depends on rules/, map/; never render/ui/audio/net).

use std::collections::BTreeMap;
use std::fmt;

use super::{
    PlacementEvidence, RevealOutcome, RevealPosition, RevealRequest, SimSoundEvent, Simulation,
};
use crate::map::entities::{EntityCategory, MapEntity};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::rules::object_type::{FactoryType, ObjectCategory, ObjectType};
use crate::rules::ruleset::RuleSet;
use crate::sim::animation::{Animation, SequenceKind};
use crate::sim::base_plan::pack_base_plan_cell;
use crate::sim::base_plan_generation::{preflight_recalc, recalc_base_plan};
use crate::sim::components::{
    BridgeOccupancy, BuildingDown, BuildingUp, HarvestOverlay, Health, VoxelAnimation,
};
use crate::sim::game_entity::{
    GameEntity, GeneratedTechnoInit, StructureUpgradeLink, TechnoConstructorInit,
};
use crate::sim::intern::InternedId;
use crate::sim::miner::{Miner, MinerConfig, miner_kind_for_object};
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::production::{ProductionCategory, foundation_dimensions};
use crate::sim::vision::MAX_SIGHT_RANGE;

/// Exact generated-object constructor handoff. The later RMG lifecycle owner
/// supplies this table after replaying all successful and discarded native
/// constructor events on the launch Scenario cursor.
#[derive(Debug, Clone, Default)]
pub(crate) struct GeneratedTechnoInitTable {
    entries: BTreeMap<usize, GeneratedTechnoInit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GeneratedTechnoInitError {
    TraceOrdinalMismatch {
        expected: usize,
        found: usize,
    },
    DuplicateEntityIndex(usize),
    MissingEntityIndex(usize),
    UnexpectedEntityIndex(usize),
    IdentityMismatch {
        entity_index: usize,
        expected_type: String,
        found_type: String,
        expected_cell: (u16, u16),
        found_cell: (u16, u16),
    },
    UnresolvableEntity {
        entity_index: usize,
        techno_type: String,
        owner: String,
    },
}

impl fmt::Display for GeneratedTechnoInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TraceOrdinalMismatch { expected, found } => write!(
                f,
                "generated construction trace expected ordinal {expected}, found {found}"
            ),
            Self::DuplicateEntityIndex(index) => {
                write!(
                    f,
                    "duplicate generated Techno binding for entity index {index}"
                )
            }
            Self::MissingEntityIndex(index) => {
                write!(
                    f,
                    "missing generated Techno binding for entity index {index}"
                )
            }
            Self::UnexpectedEntityIndex(index) => {
                write!(
                    f,
                    "unexpected generated Techno binding for entity index {index}"
                )
            }
            Self::IdentityMismatch {
                entity_index,
                expected_type,
                found_type,
                expected_cell,
                found_cell,
            } => write!(
                f,
                "generated Techno binding {entity_index} expected {expected_type} at {expected_cell:?}, found {found_type} at {found_cell:?}"
            ),
            Self::UnresolvableEntity {
                entity_index,
                techno_type,
                owner,
            } => write!(
                f,
                "generated Techno {entity_index} cannot resolve type {techno_type} or owner {owner}"
            ),
        }
    }
}

impl std::error::Error for GeneratedTechnoInitError {}

impl GeneratedTechnoInitTable {
    pub(crate) fn try_new(
        entries: impl IntoIterator<Item = GeneratedTechnoInit>,
    ) -> Result<Self, GeneratedTechnoInitError> {
        let mut by_index = BTreeMap::new();
        for entry in entries {
            let index = entry.entity_index;
            if by_index.insert(index, entry).is_some() {
                return Err(GeneratedTechnoInitError::DuplicateEntityIndex(index));
            }
        }
        Ok(Self { entries: by_index })
    }

    #[cfg(test)]
    pub(crate) fn entry(&self, entity_index: usize) -> Option<&GeneratedTechnoInit> {
        self.entries.get(&entity_index)
    }

    fn validate<'a>(
        &'a self,
        entities: &[MapEntity],
    ) -> Result<Vec<&'a GeneratedTechnoInit>, GeneratedTechnoInitError> {
        if let Some((&index, _)) = self
            .entries
            .iter()
            .find(|(index, _)| **index >= entities.len())
        {
            return Err(GeneratedTechnoInitError::UnexpectedEntityIndex(index));
        }
        let mut ordered = Vec::with_capacity(entities.len());
        for (entity_index, entity) in entities.iter().enumerate() {
            let init = self
                .entries
                .get(&entity_index)
                .ok_or(GeneratedTechnoInitError::MissingEntityIndex(entity_index))?;
            if !init.techno_type.eq_ignore_ascii_case(&entity.type_id)
                || init.cell != (entity.cell_x, entity.cell_y)
            {
                return Err(GeneratedTechnoInitError::IdentityMismatch {
                    entity_index,
                    expected_type: init.techno_type.clone(),
                    found_type: entity.type_id.clone(),
                    expected_cell: init.cell,
                    found_cell: (entity.cell_x, entity.cell_y),
                });
            }
            ordered.push(init);
        }
        Ok(ordered)
    }
}

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

/// A Foot object owns its configured locomotor independently of the parsed
/// `Speed=` scalar. `DriveLocomotionClass::Process @ 0x004B0500` and
/// `ShipLocomotionClass::Process @ 0x0069FC10` consume their class-local state
/// without a Speed gate. That state is therefore load-bearing at Speed=0,
/// while Structures remain outside Foot and other zero-speed custom
/// locomotors retain the existing inactive compatibility behavior.
fn should_construct_locomotor(category: EntityCategory, object: &ObjectType) -> bool {
    object.speed > 0
        || (matches!(
            category,
            EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft
        ) && matches!(object.locomotor, LocomotorKind::Drive | LocomotorKind::Ship))
}

impl Simulation {
    fn resolve_techno_constructor_word(
        &mut self,
        init: TechnoConstructorInit,
        expected_generated_identity: Option<(usize, &str, (u16, u16))>,
    ) -> Result<u16, GeneratedTechnoInitError> {
        match init {
            TechnoConstructorInit::FreshScenario => {
                Ok((self.scenario_rng.next_u32() & 0xFFFF) as u16)
            }
            TechnoConstructorInit::Restored(word) => Ok(word),
            TechnoConstructorInit::PreconsumedGenerated(generated) => {
                let Some((entity_index, techno_type, cell)) = expected_generated_identity else {
                    return Err(GeneratedTechnoInitError::UnexpectedEntityIndex(
                        generated.entity_index,
                    ));
                };
                if generated.entity_index != entity_index
                    || !generated.techno_type.eq_ignore_ascii_case(techno_type)
                    || generated.cell != cell
                {
                    return Err(GeneratedTechnoInitError::IdentityMismatch {
                        entity_index,
                        expected_type: generated.techno_type,
                        found_type: techno_type.to_string(),
                        expected_cell: generated.cell,
                        found_cell: cell,
                    });
                }
                Ok(generated.techno_ctor_random_word)
            }
        }
    }

    /// Build one deliberately lifecycle-free Techno for diagnostic tools.
    ///
    /// This is not a gameplay spawn: it does not reveal the entity, register
    /// occupancy, or update house ownership. It still owns the two native
    /// constructor invariants that apply to every Techno-shaped object:
    /// Simulation allocates the stable identity and consumes one Scenario RNG
    /// draw stored at `TechnoClass +0x3C8` (`0x006F3259`) before the entity
    /// enters its store.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn insert_synthetic_techno_for_diagnostics(
        &mut self,
        rx: u16,
        ry: u16,
        z: u8,
        facing: u8,
        owner: InternedId,
        health: Health,
        type_ref: InternedId,
        category: EntityCategory,
        veterancy: u16,
        vision_range: u16,
        is_voxel: bool,
    ) -> u64 {
        let stable_id = self.allocate_stable_id();
        let techno_ctor_random_word = self
            .resolve_techno_constructor_word(TechnoConstructorInit::FreshScenario, None)
            .expect("fresh Techno constructor initialization cannot fail");
        let entity = GameEntity::new_at_frame_from_constructor_word(
            stable_id,
            rx,
            ry,
            z,
            facing,
            owner,
            health,
            type_ref,
            category,
            veterancy,
            vision_range,
            is_voxel,
            self.session.binary_frame,
            techno_ctor_random_word,
        );
        self.substrate.entities.insert(entity);
        stable_id
    }
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
        self.spawn_from_map_with_resolved_and_overlay_registry(
            entities,
            rules,
            height_map,
            resolved_terrain,
            None,
        )
    }

    pub(crate) fn spawn_from_map_with_resolved_and_overlay_registry(
        &mut self,
        entities: &[MapEntity],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> u32 {
        self.spawn_from_map_with_constructor_inits(
            entities,
            rules,
            height_map,
            resolved_terrain,
            overlay_registry,
            None,
        )
        .expect("fresh fixed-map Techno constructor initialization cannot fail")
    }

    /// Project generated-map Technos using constructor words already consumed
    /// by the launch-time generation cursor. The whole identity table is
    /// validated before the first entity mutates the simulation.
    pub(crate) fn spawn_generated_from_map_with_resolved(
        &mut self,
        entities: &[MapEntity],
        rules: &RuleSet,
        height_map: &BTreeMap<(u16, u16), u8>,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        constructor_inits: &GeneratedTechnoInitTable,
    ) -> Result<u32, GeneratedTechnoInitError> {
        self.spawn_from_map_with_constructor_inits(
            entities,
            Some(rules),
            height_map,
            resolved_terrain,
            None,
            Some(constructor_inits),
        )
    }

    fn spawn_from_map_with_constructor_inits(
        &mut self,
        entities: &[MapEntity],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        constructor_inits: Option<&GeneratedTechnoInitTable>,
    ) -> Result<u32, GeneratedTechnoInitError> {
        let generated_inits = constructor_inits
            .map(|table| table.validate(entities))
            .transpose()?;

        if generated_inits.is_some() {
            let rules = rules.expect("generated-map projection requires rules");
            for (entity_index, entity) in entities.iter().enumerate() {
                if !self.map_entity_resolves_before_constructor(entity, Some(rules), true) {
                    return Err(GeneratedTechnoInitError::UnresolvableEntity {
                        entity_index,
                        techno_type: entity.type_id.clone(),
                        owner: entity.owner.clone(),
                    });
                }
            }
        }

        let mut count: u32 = 0;

        for (entity_index, map_ent) in entities.iter().enumerate() {
            if !self.map_entity_resolves_before_constructor(
                map_ent,
                rules,
                generated_inits.is_some(),
            ) {
                log::warn!(
                    "Skipping map Techno {} owned by {} before constructor resolution",
                    map_ent.type_id,
                    map_ent.owner
                );
                continue;
            }
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
            let constructor_init = generated_inits
                .as_ref()
                .map_or(TechnoConstructorInit::FreshScenario, |inits| {
                    TechnoConstructorInit::PreconsumedGenerated((*inits[entity_index]).clone())
                });
            let techno_ctor_random_word = self.resolve_techno_constructor_word(
                constructor_init,
                generated_inits.as_ref().map(|_| {
                    (
                        entity_index,
                        map_ent.type_id.as_str(),
                        (map_ent.cell_x, map_ent.cell_y),
                    )
                }),
            )?;

            // Build the GameEntity with all required fields.
            let mut ge = GameEntity::new_at_frame_from_constructor_word(
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
                techno_ctor_random_word,
            );
            ge.attached_tag_id = map_ent
                .attached_tag_id
                .as_deref()
                .map(|tag_id| self.interner.intern(tag_id));
            ge.base_defense_response.recruitable_a = map_ent.recruitable_a;
            ge.base_defense_response.recruitable_b = map_ent.recruitable_b;

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
                if should_construct_locomotor(map_ent.category, obj) {
                    let flight_level = rules.map_or(1500, |r| r.general.flight_level);
                    let mut loco = LocomotorState::from_object_type(
                        obj,
                        flight_level,
                        self.session.binary_frame,
                    );
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
                            obj.passengers as u32,
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
            let (stable_id, outcome) =
                self.unlimbo_after_constructor_managers(ge, rules, overlay_registry);
            if !matches!(outcome, RevealOutcome::Revealed { .. }) {
                self.discard_constructed_limbo(stable_id);
                continue;
            }
            if let Some(ruleset) = rules {
                self.initialize_cloak_after_unlimbo(stable_id, ruleset);
                self.add_unit_sensor_after_unlimbo(stable_id, ruleset);
                self.add_building_sensor_array_if_powered(stable_id, ruleset);
            }
            self.commit_map_placement_mission(stable_id, map_ent.mission);
            count += 1;

            if map_ent.category == EntityCategory::Structure
                && let Some(ruleset) = rules
            {
                for (slot, upgrade_type) in map_ent.structure_upgrades.iter().enumerate() {
                    let Some(upgrade_type) = upgrade_type.as_deref() else {
                        continue;
                    };
                    let valid_upgrade = ruleset
                        .object(upgrade_type)
                        .is_some_and(|object| object.category == ObjectCategory::Building);
                    if !valid_upgrade {
                        log::warn!(
                            "Skipping unresolved authored upgrade {} in slot {} on {}",
                            upgrade_type,
                            slot,
                            map_ent.type_id
                        );
                        continue;
                    }
                    if self
                        .spawn_attached_map_upgrade(
                            stable_id,
                            slot as u8,
                            upgrade_type,
                            &map_ent.owner,
                            map_ent.cell_x,
                            map_ent.cell_y,
                            z,
                            map_ent.facing,
                            ruleset,
                        )
                        .is_some()
                    {
                        count += 1;
                    }
                }
            }
        }

        log::info!("Spawned {} entities", count);
        Ok(count)
    }

    fn map_entity_resolves_before_constructor(
        &self,
        map_ent: &MapEntity,
        rules: Option<&RuleSet>,
        require_owner: bool,
    ) -> bool {
        let type_resolves = rules.map_or(true, |rules| {
            rules.object(&map_ent.type_id).is_some_and(|object| {
                matches!(
                    (map_ent.category, object.category),
                    (EntityCategory::Unit, ObjectCategory::Vehicle)
                        | (EntityCategory::Aircraft, ObjectCategory::Aircraft)
                        | (EntityCategory::Infantry, ObjectCategory::Infantry)
                        | (EntityCategory::Structure, ObjectCategory::Building)
                )
            })
        });
        let owner_resolves = (!require_owner && self.houses.is_empty())
            || crate::sim::house_state::house_state_for_owner(
                &self.houses,
                &map_ent.owner,
                &self.interner,
            )
            .is_some();
        type_resolves && owner_resolves
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_attached_map_upgrade(
        &mut self,
        parent_stable_id: u64,
        slot: u8,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        z: u8,
        facing: u8,
        rules: &RuleSet,
    ) -> Option<u64> {
        let stable_id =
            self.construct_object_limbo_at_height(type_id, owner, rx, ry, facing, z, rules)?;
        {
            let upgrade = self.substrate.entities.get_mut(stable_id)?;
            upgrade.structure_upgrade_link = Some(StructureUpgradeLink {
                parent_stable_id,
                slot,
            });
        }
        if self
            .reveal_constructed_object_at_height(
                stable_id,
                rx,
                ry,
                facing,
                z,
                PlacementEvidence::AttachedUpgrade,
                rules,
            )
            .is_none()
        {
            self.discard_constructed_limbo(stable_id);
            return None;
        }
        Some(stable_id)
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

    /// Immediate construction with the match's live OverlayTypeClass table.
    /// Production callers use this boundary whenever the constructed type may
    /// be a Unit, so its virtual Unlimbo sees the same overlay facts as native.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_object_with_overlay_registry(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        rules: &RuleSet,
        height_map: &BTreeMap<(u16, u16), u8>,
        overlay_registry: &crate::map::overlay_types::OverlayTypeRegistry,
    ) -> Option<u64> {
        let z = height_map.get(&(rx, ry)).copied().unwrap_or(0);
        self.spawn_object_at_height_with_overlay_registry(
            type_id,
            owner,
            rx,
            ry,
            facing,
            z,
            rules,
            overlay_registry,
        )
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
        self.spawn_object_at_height_with_overlay_context(
            type_id, owner, rx, ry, facing, z, rules, None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_object_at_height_with_overlay_context(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> Option<u64> {
        self.spawn_object_at_height_with_init(
            type_id,
            owner,
            rx,
            ry,
            facing,
            z,
            rules,
            overlay_registry,
            TechnoConstructorInit::FreshScenario,
        )
        .expect("fresh Techno constructor initialization cannot fail")
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_object_at_height_with_overlay_registry(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
        overlay_registry: &crate::map::overlay_types::OverlayTypeRegistry,
    ) -> Option<u64> {
        self.spawn_object_at_height_with_overlay_context(
            type_id,
            owner,
            rx,
            ry,
            facing,
            z,
            rules,
            Some(overlay_registry),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_object_at_height_with_init(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        init: TechnoConstructorInit,
    ) -> Result<Option<u64>, GeneratedTechnoInitError> {
        let Some(obj) = rules.object(type_id) else {
            return Ok(None);
        };
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
        let techno_ctor_random_word = self.resolve_techno_constructor_word(init, None)?;

        let mut ge = GameEntity::new_at_frame_from_constructor_word(
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
            techno_ctor_random_word,
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
        if should_construct_locomotor(category, obj) {
            ge.locomotor = Some(LocomotorState::from_object_type(
                obj,
                rules.general.flight_level,
                self.session.binary_frame,
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
                cargo: crate::sim::passenger::PassengerCargo::new(
                    obj.passengers as u32,
                    obj.size_limit,
                ),
            };
        } else if obj.can_be_occupied && obj.max_number_occupants > 0 {
            ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                cargo: crate::sim::passenger::PassengerCargo::new(obj.max_number_occupants, 1),
            };
        }

        stamp_building_cell_profile(&mut ge, Some(obj));
        // TechnoClass::Init_Managers — manager-owned children are committed
        // after this parent enters the limbo store and before its Unlimbo.
        ge.capture_manager = crate::sim::capture_manager::init_capture_manager(obj, rules);
        ge.spawn_manager = crate::sim::spawn_manager::init_spawn_manager(
            obj,
            rules,
            &mut self.interner,
            self.session.binary_frame,
        );
        let (stable_id, outcome) =
            self.unlimbo_after_constructor_managers(ge, Some(rules), overlay_registry);
        if !matches!(outcome, RevealOutcome::Revealed { .. }) {
            // This convenience path owns its transient constructor result.
            // Held production objects use the separate limbo/retry boundary.
            self.discard_constructed_limbo(stable_id);
            return Ok(None);
        }
        self.initialize_cloak_after_unlimbo(stable_id, rules);
        self.add_unit_sensor_after_unlimbo(stable_id, rules);
        self.commit_spawn_harvest_mission(stable_id);
        Ok(Some(stable_id))
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
        self.spawn_object_limbo_at_height_with_init(
            type_id,
            owner,
            rx,
            ry,
            facing,
            z,
            rules,
            TechnoConstructorInit::FreshScenario,
        )
        .expect("fresh Techno constructor initialization cannot fail")
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_object_limbo_at_height_with_init(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
        init: TechnoConstructorInit,
    ) -> Result<Option<u64>, GeneratedTechnoInitError> {
        let Some(obj) = rules.object(type_id) else {
            return Ok(None);
        };
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
        let techno_ctor_random_word = self.resolve_techno_constructor_word(init, None)?;

        let mut ge = GameEntity::new_at_frame_from_constructor_word(
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
            techno_ctor_random_word,
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
        if should_construct_locomotor(category, obj) {
            ge.locomotor = Some(LocomotorState::from_object_type(
                obj,
                rules.general.flight_level,
                self.session.binary_frame,
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
                cargo: crate::sim::passenger::PassengerCargo::new(
                    obj.passengers as u32,
                    obj.size_limit,
                ),
            };
        } else if obj.can_be_occupied && obj.max_number_occupants > 0 {
            ge.passenger_role = crate::sim::passenger::PassengerRole::Transport {
                cargo: crate::sim::passenger::PassengerCargo::new(obj.max_number_occupants, 1),
            };
        }

        stamp_building_cell_profile(&mut ge, Some(obj));

        ge.capture_manager = crate::sim::capture_manager::init_capture_manager(obj, rules);
        ge.spawn_manager = crate::sim::spawn_manager::init_spawn_manager(
            obj,
            rules,
            &mut self.interner,
            self.session.binary_frame,
        );

        let stable_id = self.create_limbo(ge);
        self.commit_constructor_owned_techno_children(stable_id, rules);
        self.commit_spawn_harvest_mission(stable_id);
        Ok(Some(stable_id))
    }

    /// Compatibility name for constructing and accounting the Techno identity
    /// held from `FactoryClass::StartProduction @ 0x004C9C70` through delivery.
    /// Production and other limbo constructors share the same manager/child
    /// transaction; later delivery only Unlimbos this retained identity.
    pub(crate) fn create_production_object_limbo_at_height(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
    ) -> Option<u64> {
        self.construct_object_limbo_at_height(type_id, owner, rx, ry, facing, z, rules)
    }

    /// Construct one fully initialized runtime Techno and retain it in limbo
    /// for one or more placement attempts. Starting-force creation and held
    /// production both use this identity-preserving boundary.
    pub(crate) fn construct_object_limbo_at_height(
        &mut self,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        rules: &RuleSet,
    ) -> Option<u64> {
        self.spawn_object_limbo_at_height(type_id, owner, rx, ry, facing, z, rules)
    }

    /// Run one result-bearing Unlimbo transaction against an already stored
    /// production object. Mark failure restores this same identity to limbo;
    /// construction and owned-count accounting are deliberately not repeated.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn unlimbo_held_production_object(
        &mut self,
        stable_id: u64,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        placement: PlacementEvidence,
        rules: &RuleSet,
    ) -> Option<u64> {
        self.reveal_constructed_object_at_height_with_unit_context(
            stable_id, rx, ry, facing, z, placement, rules, None, stable_id,
        )
    }

    /// Production delivery boundary for a held object whose concrete Unit
    /// virtual needs the live overlay table and selected producer identity.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn unlimbo_held_production_object_with_unit_context(
        &mut self,
        stable_id: u64,
        producer_id: u64,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        placement: PlacementEvidence,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> Option<u64> {
        self.reveal_constructed_object_at_height_with_unit_context(
            stable_id,
            rx,
            ry,
            facing,
            z,
            placement,
            rules,
            overlay_registry,
            producer_id,
        )
    }

    /// Place an already constructed limbo Techno without repeating its
    /// constructor draw or manager initialization. Failure restores this same
    /// identity to limbo so a caller may try another coordinate.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reveal_constructed_object_at_height(
        &mut self,
        stable_id: u64,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        placement: PlacementEvidence,
        rules: &RuleSet,
    ) -> Option<u64> {
        self.reveal_constructed_object_at_height_with_unit_context(
            stable_id, rx, ry, facing, z, placement, rules, None, stable_id,
        )
    }

    /// Common concrete Unlimbo boundary. A Unit arriving with ordinary
    /// `EvaluateMark` first executes its exact `+0x1AC` CanEnter predicate;
    /// callers that already proved exact zero carry that evidence instead.
    /// Rejection returns before facing, subcell, bridge, or position mutation.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reveal_constructed_object_at_height_with_unit_context(
        &mut self,
        stable_id: u64,
        rx: u16,
        ry: u16,
        facing: u8,
        z: u8,
        placement: PlacementEvidence,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        producer_id: u64,
    ) -> Option<u64> {
        let requested_position = RevealPosition {
            rx,
            ry,
            z,
            sub_x: self.substrate.entities.get(stable_id)?.position.sub_x,
            sub_y: self.substrate.entities.get(stable_id)?.position.sub_y,
        };
        let placement = if placement == PlacementEvidence::EvaluateMark {
            self.constructor_unlimbo_placement(
                stable_id,
                requested_position,
                rules,
                overlay_registry,
                producer_id,
            )
        } else {
            placement
        };
        if placement == PlacementEvidence::RejectedEarly {
            let _ = self.try_reveal_entity(
                stable_id,
                RevealRequest {
                    position: requested_position,
                    placement,
                    logic_eligible: true,
                },
            );
            return None;
        }

        let z = match placement {
            PlacementEvidence::UnitCanEnterExactZero {
                layer: MovementLayer::Bridge,
            } => self
                .resolved_terrain
                .as_ref()
                .and_then(|terrain| terrain.cell(rx, ry))
                .map_or(z, |cell| cell.bridge_deck_level),
            _ => z,
        };
        if let PlacementEvidence::UnitCanEnterExactZero { layer } = placement
            && let Some(entity) = self.substrate.entities.get_mut(stable_id)
        {
            entity.on_bridge = layer == MovementLayer::Bridge;
        }
        let is_infantry = self
            .substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| entity.category == EntityCategory::Infantry);
        let infantry_sub_cell = is_infantry.then(|| self.allocate_infantry_sub_cell(rx, ry));
        let (sub_x, sub_y) = {
            let entity = self.substrate.entities.get_mut(stable_id)?;
            entity.facing = facing;
            if let Some(sub_cell) = infantry_sub_cell {
                entity.sub_cell = Some(sub_cell);
                let offsets = crate::util::lepton::subcell_lepton_offset(Some(sub_cell));
                entity.position.sub_x = offsets.0;
                entity.position.sub_y = offsets.1;
            }
            (entity.position.sub_x, entity.position.sub_y)
        };
        let outcome = self.try_reveal_entity(
            stable_id,
            RevealRequest {
                position: RevealPosition {
                    rx,
                    ry,
                    z,
                    sub_x,
                    sub_y,
                },
                placement,
                logic_eligible: true,
            },
        );
        if !matches!(outcome, RevealOutcome::Revealed { .. }) {
            return None;
        }
        self.initialize_cloak_after_unlimbo(stable_id, rules);
        self.add_unit_sensor_after_unlimbo(stable_id, rules);
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
        self.add_infantry_tracking_once(stable_id);
        stable_id
    }

    /// Delete a constructor-complete object that never successfully left
    /// limbo. The stable ID and constructor RNG draw stay spent, while the
    /// transient store and owned-count effects are undone exactly once.
    pub(crate) fn discard_constructed_limbo(&mut self, stable_id: u64) -> bool {
        if !self.substrate.entities.contains(stable_id) {
            return false;
        }
        let spawn_children = self
            .substrate
            .entities
            .get(stable_id)
            .and_then(|entity| entity.spawn_manager.as_ref())
            .map(|manager| {
                manager
                    .slots
                    .iter()
                    .filter_map(|slot| slot.spawn)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let slave_children = self
            .production
            .slave_bindings
            .remove(&stable_id)
            .unwrap_or_default();
        let entity = self
            .substrate
            .entities
            .remove(stable_id)
            .expect("constructor object existence checked above");
        debug_assert!(entity.lifecycle.in_limbo && !entity.lifecycle.cell_marked);
        let owner = self.interner.resolve(entity.owner).to_string();
        self.decrement_owned_count(&owner, entity.category);
        for child_id in spawn_children.into_iter().chain(slave_children) {
            if self.substrate.entities.contains(child_id) {
                let discarded = self.discard_constructed_limbo(child_id);
                debug_assert!(discarded, "constructor-owned child must remain in limbo");
            }
        }
        true
    }

    /// Remove only the freshly constructed slave pool owned by `master_id`.
    /// `PowerUp_Cleanup @ 0x006AF580` uses this while transplanting an older
    /// SMIN/YAREFN manager into a newly constructed counterpart: the new
    /// pool's constructor draws remain spent, but none of its children survive.
    pub(crate) fn discard_constructor_owned_slave_pool(&mut self, master_id: u64) -> bool {
        let Some(slave_ids) = self.production.slave_bindings.remove(&master_id) else {
            return false;
        };
        for slave_id in slave_ids {
            if self.substrate.entities.contains(slave_id) {
                let discarded = self.discard_constructed_limbo(slave_id);
                debug_assert!(discarded, "fresh slave-manager child must remain in limbo");
            }
        }
        true
    }

    /// Spawn an object directly into limbo: stored in EntityStore and owner counts
    /// but NOT registered in the active order or map occupancy. Registration
    /// happens later at reveal/landing (e.g. paradrop drop). Returns the stable id.
    pub(crate) fn create_limbo(&mut self, ge: GameEntity) -> u64 {
        self.store_spawned_limbo(ge)
    }

    /// Store a new object, then place it through active
    /// `ObjectClass::Reveal @ 0x005F4EC0`: coordinates commit, the mode-one
    /// playfield gate and Mark(PUT) own the result, and eligible logic
    /// registration happens last. A failed attempt keeps the constructed
    /// identity in limbo for its caller to retain or discard.
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
                placement: PlacementEvidence::EvaluateMark,
                logic_eligible: true,
            },
        );
        (stable_id, outcome)
    }

    /// Complete `TechnoClass::Init_Managers @ 0x006F3F40` while the freshly
    /// constructed parent is still in limbo, then attempt the parent's first
    /// Unlimbo. Both SpawnManagerClass @ 0x006B6C90 and SlaveManagerClass
    /// @ 0x006AF1A0 construct their child Technos before parent placement.
    fn unlimbo_after_constructor_managers(
        &mut self,
        ge: GameEntity,
        rules: Option<&RuleSet>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> (u64, RevealOutcome) {
        let mut position = RevealPosition {
            rx: ge.position.rx,
            ry: ge.position.ry,
            z: ge.position.z,
            sub_x: ge.position.sub_x,
            sub_y: ge.position.sub_y,
        };
        let stable_id = self.store_spawned_limbo(ge);
        if let Some(rules) = rules {
            self.commit_constructor_owned_techno_children(stable_id, rules);
        }
        let placement = rules.map_or(PlacementEvidence::EvaluateMark, |rules| {
            self.constructor_unlimbo_placement(
                stable_id,
                position,
                rules,
                overlay_registry,
                stable_id,
            )
        });
        if let PlacementEvidence::UnitCanEnterExactZero { layer } = placement {
            if layer == MovementLayer::Bridge {
                position.z = self
                    .resolved_terrain
                    .as_ref()
                    .and_then(|terrain| terrain.cell(position.rx, position.ry))
                    .map_or(position.z, |cell| cell.bridge_deck_level);
            }
            if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.on_bridge = layer == MovementLayer::Bridge;
            }
        }
        let outcome = self.try_reveal_entity(
            stable_id,
            RevealRequest {
                position,
                placement,
                logic_eligible: true,
            },
        );
        (stable_id, outcome)
    }

    /// Collapse the concrete Techno `+0x1AC` return code at the same boundary
    /// as `ObjectClass::Unlimbo @ 0x005F4F1B..0x005F4F49`: exact zero admits,
    /// every nonzero code rejects before any object mutation. UnitClass owns
    /// the first active constructor specialization promoted here; the shared
    /// evaluator is the same `(cell,-1,-1,0,0)` body used by production.
    fn constructor_unlimbo_placement(
        &self,
        stable_id: u64,
        position: RevealPosition,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        producer_id: u64,
    ) -> PlacementEvidence {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return PlacementEvidence::RejectedEarly;
        };
        // Active scenario construction installs its CellClass grid before any
        // Techno can reach Unlimbo. Terrain-less diagnostic/unit-test harnesses
        // remain on the generic Mark seam because no native UnitClass cell input
        // exists there to evaluate.
        if entity.category != EntityCategory::Unit || self.resolved_terrain.is_none() {
            return PlacementEvidence::EvaluateMark;
        }
        let owner = self.interner.resolve(entity.owner).to_string();
        let type_id = self.interner.resolve(entity.type_ref).to_string();
        let admission = crate::sim::production::produced_unit_unlimbo_entry_at_resolved_cell(
            self,
            rules,
            &owner,
            &type_id,
            stable_id,
            producer_id,
            (position.rx, position.ry),
            overlay_registry,
        );
        let Some(layer) = admission.exact_zero_layer() else {
            return PlacementEvidence::RejectedEarly;
        };
        PlacementEvidence::UnitCanEnterExactZero { layer }
    }

    /// Materialize constructor-owned Technos in native manager order. The
    /// parent has already consumed its own constructor word; every child uses
    /// the same FreshScenario funnel and remains a stable limbo identity.
    fn commit_constructor_owned_techno_children(&mut self, parent_id: u64, rules: &RuleSet) {
        crate::sim::spawn_manager::commit_spawn_manager_pool(self, parent_id, rules);

        if self.production.slave_bindings.contains_key(&parent_id) {
            return;
        }
        let Some((slave_type, slave_count, owner, rx, ry, z, facing, capacity)) =
            self.substrate.entities.get(parent_id).and_then(|parent| {
                let parent_type = self.interner.resolve(parent.type_ref);
                let object = rules.object_case_insensitive(parent_type)?;
                let slave_type = object.enslaves.as_deref()?;
                let slave_object = rules.object_case_insensitive(slave_type)?;
                if object.slaves_number <= 0 || slave_object.category != ObjectCategory::Infantry {
                    return None;
                }
                Some((
                    slave_type.to_string(),
                    object.slaves_number as usize,
                    self.interner.resolve(parent.owner).to_string(),
                    parent.position.rx,
                    parent.position.ry,
                    parent.position.z,
                    parent.facing,
                    slave_object.storage.max(1) as u16,
                ))
            })
        else {
            return;
        };

        let mut slave_ids = Vec::with_capacity(slave_count);
        for _ in 0..slave_count {
            let Some(slave_id) = self.construct_object_limbo_at_height(
                &slave_type,
                &owner,
                rx,
                ry,
                facing,
                z,
                rules,
            ) else {
                continue;
            };
            if let Some(slave) = self.substrate.entities.get_mut(slave_id) {
                slave.slave_harvester = Some(crate::sim::slave_miner::SlaveHarvester::new(
                    parent_id, capacity,
                ));
            }
            slave_ids.push(slave_id);
        }
        self.production.slave_bindings.insert(parent_id, slave_ids);
    }

    /// Unit/Infantry constructor cloak ability plus UnitClass::Unlimbo's
    /// exceptional state-2 establishment. Active evidence: constructor copy at
    /// 0x007355B6/0x00517D88 and UnitClass::Unlimbo @ 0x00737BA0.
    fn initialize_cloak_after_unlimbo(&mut self, stable_id: u64, rules: &RuleSet) {
        let Some((category, veterancy, in_playfield, type_ref)) =
            self.substrate.entities.get(stable_id).map(|entity| {
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
        let rank_cloak =
            veterancy >= 100 && object.veteran_cloak || veterancy >= 200 && object.elite_cloak;
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
                yard_obj.construction_yard,
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
            is_construction_yard,
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
                        || e.lifecycle.in_limbo
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

        // Native successful deploy transaction:
        // `UnitClass__Deploy @ 0x007393C0`, block `0x00739855..0x00739926`,
        // calls `FUN_00505180 @ 0x00505180` only for a non-controlled
        // ConstructionYard in a nonzero game mode. VERA preflights only the
        // directly indexed Recalc vectors before the destructive MCV removal.
        let recalc_context = self.houses.get(&owner_id).and_then(|house| {
            (is_construction_yard
                && self.session.game_mode_nonzero
                && !house.is_controlled_by_human(true))
            .then(|| {
                let country_name = house
                    .country
                    .map(|country| self.interner.resolve(country).to_owned());
                (
                    country_name,
                    house.side_index,
                    house.difficulty,
                    house.tech_level,
                    house.base_plan.nodes.is_empty(),
                )
            })
        });
        if let Some((country_name, side_index, difficulty, _, true)) = &recalc_context {
            let Some(country_name) = country_name.as_deref() else {
                return false;
            };
            if preflight_recalc(rules, country_name, *side_index, *difficulty).is_err() {
                return false;
            }
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

        if let Some((country_name, side_index, difficulty, tech_level, _)) = recalc_context {
            // The new Building's committed north-west anchor is the native
            // `+0x9C/+0xA0` source. This bounded write order is load-bearing:
            // primary center, optional Recalc, node zero, BasePlan center,
            // then the three independent House AI activation latches.
            let house = self
                .houses
                .get_mut(&owner_id)
                .expect("qualifying deploy owner remains registered");
            house.base_center = Some((rx, ry));
            if house.base_plan.nodes.is_empty() {
                recalc_base_plan(
                    &mut house.base_plan,
                    rules,
                    country_name
                        .as_deref()
                        .expect("empty qualifying plan was preflighted with a country"),
                    side_index,
                    difficulty,
                    tech_level,
                    self.session.game_options.super_weapons,
                    &mut self.scenario_rng,
                );
            }
            if let Some(node_zero) = house.base_plan.nodes.first_mut() {
                node_zero.packed_cell = pack_base_plan_cell(i32::from(rx), i32::from(ry));
            }
            house.base_plan_center = (rx, ry);
            house.enable_ai_deploy_latches();
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
                && !entity.lifecycle.in_limbo
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
        ge.build_const_eligible = obj.build_const_eligible;
        ge.grinding_facility = obj.grinding;
        ge.absorber_facility = obj.unit_absorb || obj.infantry_absorb;
        ge.base_plan_type_index = obj.base_plan_type_index;
        ge.base_plan_is_defense = obj.is_base_defense;
        // Projection of native BuildingType+0x408 after
        // UnitTypeClass__FindOrAllocate @ 0x007480D0 has resolved
        // `none`/`<none>` to a null pointer.
        ge.base_plan_has_undeploy_target = obj.undeploys_into.is_some();
    }
}

#[cfg(test)]
mod techno_constructor_tests {
    use super::*;
    use crate::map::bridge_facts::BridgeCellFacts;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::rng::SimRng;

    fn constructor_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[InfantryTypes]\n0=E1\n1=SLAV\n\n\
             [VehicleTypes]\n0=MTNK\n1=CARRIER\n2=SMIN\n\n\
             [AircraftTypes]\n0=ORCA\n1=HORN\n\n\
             [BuildingTypes]\n0=BASE\n1=UP1\n2=UP2\n3=YAREFN\n\n\
             [E1]\nStrength=100\nSpeed=4\n\n\
             [SLAV]\nStrength=125\nSpeed=4\nStorage=4\n\n\
             [MTNK]\nStrength=300\nSpeed=6\n\n\
             [CARRIER]\nStrength=800\nSpeed=4\nSpawns=HORN\nSpawnsNumber=3\nSpawnRegenRate=600\nSpawnReloadRate=25\n\n\
             [SMIN]\nStrength=2000\nSpeed=3\nEnslaves=SLAV\nSlavesNumber=2\nSlaveRegenRate=500\nSlaveReloadRate=25\n\n\
             [ORCA]\nStrength=200\nSpeed=8\n\n\
             [HORN]\nStrength=75\nSpeed=14\nAmmo=1\n\n\
             [BASE]\nStrength=500\nFoundation=2x2\n\n\
             [UP1]\nStrength=100\nFoundation=1x1\n\n\
             [UP2]\nStrength=100\nFoundation=1x1\n\n\
             [YAREFN]\nStrength=2000\nFoundation=2x2\nEnslaves=SLAV\nSlavesNumber=2\nSlaveRegenRate=500\nSlaveReloadRate=25\n",
        ))
        .expect("constructor fixture rules parse")
    }

    fn map_entity(type_id: &str, category: EntityCategory, cell: (u16, u16)) -> MapEntity {
        MapEntity {
            owner: "Americans".to_string(),
            type_id: type_id.to_string(),
            health: 256,
            cell_x: cell.0,
            cell_y: cell.1,
            facing: 0,
            category,
            sub_cell: 0,
            veterancy: 0,
            high: false,
            mission: None,
            recruitable_a: true,
            recruitable_b: true,
            structure_upgrades: [None, None, None],
        }
    }

    fn install_american_house(sim: &mut Simulation) {
        let owner = sim.interner.intern("Americans");
        sim.houses.insert(
            owner,
            crate::sim::house_state::HouseState::new(owner, 0, None, true, 0, 10),
        );
    }

    fn install_constructor_test_playfield(sim: &mut Simulation) {
        sim.session.map_width = 10;
        sim.session.map_height = 10;
        sim.playfield_bounds = Some(crate::map::playfield::PlayfieldBounds {
            base: 10,
            off_fc: 0,
            off_100: 0,
            off_104: 10,
            off_108: 10,
        });
    }

    fn install_constructor_flat_terrain(sim: &mut Simulation) {
        let speed_costs = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            float: Some(100),
            amphibious: Some(100),
            float_beach: Some(100),
            hover: Some(100),
        };
        let cells = (0..10)
            .flat_map(|ry| {
                (0..10).map(move |rx| ResolvedTerrainCell {
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
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    accepts_smudge: false,
                    allows_tiberium: false,
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
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
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
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                })
            })
            .collect();
        sim.install_resolved_terrain_for_new_map(ResolvedTerrainGrid::from_cells(10, 10, cells));
    }

    fn assert_generated_projection_rejects_before_mutation(
        seed: u64,
        entities: &[MapEntity],
        table: &GeneratedTechnoInitTable,
        expected_error: GeneratedTechnoInitError,
    ) {
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        install_american_house(&mut sim);
        let scenario_before = sim.scenario_rng.logical_state();
        let stable_id_before = sim.substrate.next_stable_object_id;
        let enter_order_before = sim.substrate.next_occupancy_enter_order.current();
        let occupancy_generation_before = sim.substrate.occupancy.generation();
        let raw_occupation_entries_before = sim.substrate.raw_cell_occupation.entry_count();

        assert_eq!(
            sim.spawn_generated_from_map_with_resolved(
                entities,
                &rules,
                &BTreeMap::new(),
                None,
                table,
            ),
            Err(expected_error)
        );
        assert_eq!(sim.scenario_rng.logical_state(), scenario_before);
        assert_eq!(sim.substrate.next_stable_object_id, stable_id_before);
        assert_eq!(
            sim.substrate.next_occupancy_enter_order.current(),
            enter_order_before
        );
        assert_eq!(
            sim.substrate.occupancy.generation(),
            occupancy_generation_before
        );
        assert_eq!(sim.substrate.occupancy.occupied_cell_count(), 0);
        assert_eq!(sim.substrate.logic.len(), 0);
        assert!(sim.substrate.entities.is_empty());
        assert_eq!(
            sim.substrate.raw_cell_occupation.entry_count(),
            raw_occupation_entries_before
        );
        for entity in entities {
            assert!(sim.substrate.occupancy.is_empty_on_layer(
                entity.cell_x,
                entity.cell_y,
                MovementLayer::Ground,
            ));
            assert_eq!(
                sim.substrate
                    .raw_cell_occupation
                    .ground_bits(entity.cell_x, entity.cell_y),
                0
            );
            assert_eq!(
                sim.substrate
                    .raw_cell_occupation
                    .deck_bits(entity.cell_x, entity.cell_y),
                0
            );
        }
    }

    fn constructor_overlay_registry() -> crate::map::overlay_types::OverlayTypeRegistry {
        crate::map::overlay_types::OverlayTypeRegistry::from_ini(
            &IniFile::from_str(
                "[OverlayTypes]\n0=TESTORE\n1=TESTWALL\n\
                 [TESTORE]\nTiberium=yes\nLand=Tiberium\n\
                 [TESTWALL]\nWall=yes\nCrushable=yes\nLand=Wall\n",
            ),
            None,
        )
    }

    #[test]
    fn techno_constructor_live_overlay_context_admits_ore_and_structural_bridge() {
        let seed = 0xC701_0017;
        let rules = constructor_rules();
        let registry = constructor_overlay_registry();
        let mut sim = Simulation::with_seed(seed);
        install_constructor_test_playfield(&mut sim);
        sim.playfield_bounds = Some(crate::map::playfield::PlayfieldBounds {
            base: 0,
            off_fc: -100,
            off_100: -100,
            off_104: 200,
            off_108: 200,
        });
        install_constructor_flat_terrain(&mut sim);

        let ore_authored = (6, 5);
        let ore_runtime = (7, 5);
        let bridge_cell = (8, 5);
        {
            let bridge = sim
                .resolved_terrain
                .as_mut()
                .unwrap()
                .cell_mut(bridge_cell.0, bridge_cell.1)
                .unwrap();
            bridge.level = 3;
            bridge.bridge_deck_level = 7;
            bridge.has_bridge_deck = true;
            bridge.bridge_walkable = true;
            bridge.bridge_facts.raw_flags =
                crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
            bridge.bridge_facts.overlay_id = Some(0);
        }
        sim.bridge_state = Some(
            crate::sim::bridge_state::BridgeRuntimeState::from_resolved_terrain(
                sim.resolved_terrain.as_ref().unwrap(),
                true,
                300,
            ),
        );
        let mut overlays = crate::sim::overlay_grid::OverlayGrid::new(10, 10);
        for cell in [ore_authored, ore_runtime, bridge_cell] {
            overlays.place_overlay(cell.0, cell.1, 0, 0);
        }
        sim.overlay_grid = Some(overlays);

        assert_eq!(
            sim.spawn_from_map_with_resolved_and_overlay_registry(
                &[
                    map_entity("MTNK", EntityCategory::Unit, ore_authored),
                    map_entity("MTNK", EntityCategory::Unit, bridge_cell),
                ],
                Some(&rules),
                &BTreeMap::new(),
                None,
                Some(&registry),
            ),
            2,
        );
        let authored = sim.substrate.entities.get(1).unwrap();
        assert_eq!((authored.position.rx, authored.position.ry), ore_authored);
        assert!(!authored.on_bridge);
        let bridge = sim.substrate.entities.get(2).unwrap();
        assert_eq!((bridge.position.rx, bridge.position.ry), bridge_cell);
        assert_eq!(bridge.position.z, 7);
        assert!(bridge.on_bridge);
        assert_ne!(
            sim.substrate
                .raw_cell_occupation
                .deck_bits(bridge_cell.0, bridge_cell.1)
                & crate::sim::occupancy::VEHICLE_OCCUPATION_BIT,
            0,
        );

        let runtime = sim
            .spawn_object_with_overlay_registry(
                "MTNK",
                "Americans",
                ore_runtime.0,
                ore_runtime.1,
                0,
                &rules,
                &BTreeMap::new(),
                &registry,
            )
            .expect("non-wall overlay must not veto runtime Unit Unlimbo");
        assert_eq!((sim.substrate.entities.get(runtime).unwrap().position.rx, sim.substrate.entities.get(runtime).unwrap().position.ry), ore_runtime);

        let mut expected = SimRng::new(seed);
        for _ in 0..3 {
            let _ = expected.next_u32();
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    }

    #[test]
    fn techno_constructor_wall_rejection_precedes_mutation_and_keeps_graph_draws_spent() {
        let seed = 0xC701_0018;
        let rules = constructor_rules();
        let registry = constructor_overlay_registry();
        let mut sim = Simulation::with_seed(seed);
        install_constructor_test_playfield(&mut sim);
        install_constructor_flat_terrain(&mut sim);
        let wall_cell = (6, 5);
        let mut overlays = crate::sim::overlay_grid::OverlayGrid::new(10, 10);
        overlays.place_overlay(wall_cell.0, wall_cell.1, 1, 0);
        sim.overlay_grid = Some(overlays);

        let parent_id = sim
            .construct_object_limbo_at_height("CARRIER", "Americans", 2, 2, 9, 0, &rules)
            .expect("constructor-complete carrier graph");
        let child_ids = sim
            .substrate
            .entities
            .get(parent_id)
            .unwrap()
            .spawn_manager
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .filter_map(|slot| slot.spawn)
            .collect::<Vec<_>>();
        assert_eq!(child_ids.len(), 3);
        assert!(
            sim.reveal_constructed_object_at_height_with_unit_context(
                parent_id,
                wall_cell.0,
                wall_cell.1,
                0x80,
                7,
                PlacementEvidence::EvaluateMark,
                &rules,
                Some(&registry),
                parent_id,
            )
            .is_none()
        );
        let rejected = sim.substrate.entities.get(parent_id).unwrap();
        assert_eq!((rejected.position.rx, rejected.position.ry, rejected.position.z), (2, 2, 0));
        assert_eq!(rejected.facing, 9);
        assert!(rejected.lifecycle.in_limbo && !rejected.lifecycle.cell_marked);
        assert!(child_ids.iter().all(|id| sim.substrate.entities.contains(*id)));

        let mut expected = SimRng::new(seed);
        for _ in 0..4 {
            let _ = expected.next_u32();
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
        assert!(sim.discard_constructed_limbo(parent_id));
        assert!(sim.substrate.entities.is_empty());
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    }

    #[test]
    fn techno_constructor_raw_entity_constructor_is_world_spawn_only() {
        fn collect_rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("read source directory") {
                let path = entry.expect("read source entry").path();
                if path.is_dir() {
                    collect_rust_files(&path, out);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    out.push(path);
                }
            }
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect_rust_files(&root.join("src"), &mut files);
        let needle = ["GameEntity::", "new_at_frame_from_constructor_word("].concat();
        let mut owners = Vec::new();
        for path in files {
            let source = std::fs::read_to_string(&path).expect("read Rust source");
            let count = source.matches(&needle).count();
            if count != 0 {
                let relative = path
                    .strip_prefix(root)
                    .expect("source under manifest root")
                    .to_string_lossy()
                    .replace('\\', "/");
                owners.push((relative, count));
            }
        }
        owners.sort();
        assert_eq!(
            owners,
            vec![("src/sim/world/world_spawn.rs".to_string(), 4)]
        );

        let production_zero_helper = ["new_at_frame_", "zero_for_diagnostics"].concat();
        let game_entity_source = std::fs::read_to_string(root.join("src/sim/game_entity.rs"))
            .expect("read GameEntity source");
        assert!(
            !game_entity_source.contains(&production_zero_helper),
            "production diagnostics must not synthesize a Techno constructor word"
        );
    }

    #[test]
    fn techno_constructor_diagnostic_path_is_simulation_owned_and_draws_once() {
        let seed = 0xC701_0006;
        let mut sim = Simulation::with_seed(seed);
        let mut expected = SimRng::new(seed);
        let expected_word = (expected.next_u32() & 0xFFFF) as u16;
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("GACNST");

        let stable_id = sim.insert_synthetic_techno_for_diagnostics(
            10,
            12,
            0,
            0,
            owner,
            Health {
                current: 1000,
                max: 1000,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            6,
            false,
        );

        assert_eq!(stable_id, 1);
        assert_eq!(
            sim.substrate
                .entities
                .get(stable_id)
                .expect("diagnostic Techno stored")
                .techno_ctor_random_word,
            expected_word
        );
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    }

    #[test]
    fn techno_constructor_runtime_fresh_paths_draw_once_after_type_resolution() {
        let seed = 0xC701_0001;
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        install_constructor_test_playfield(&mut sim);
        let mut expected = SimRng::new(seed);

        let before_invalid = sim.scenario_rng.logical_state();
        assert!(
            sim.spawn_object("MISSING", "Americans", 1, 1, 0, &rules, &BTreeMap::new())
                .is_none()
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_invalid);

        let placed_word = (expected.next_u32() & 0xFFFF) as u16;
        let placed = sim
            .spawn_object("MTNK", "Americans", 6, 5, 0, &rules, &BTreeMap::new())
            .expect("placed runtime Techno");
        assert_eq!(
            sim.substrate
                .entities
                .get(placed)
                .unwrap()
                .techno_ctor_random_word,
            placed_word
        );

        let _failed_word = (expected.next_u32() & 0xFFFF) as u16;
        assert!(
            sim.spawn_object("BASE", "Americans", 1, 1, 0, &rules, &BTreeMap::new())
                .is_none()
        );
        assert!(sim.substrate.entities.get(2).is_none());

        let limbo_word = (expected.next_u32() & 0xFFFF) as u16;
        let limbo = sim
            .spawn_object_limbo_at_height("E1", "Americans", 6, 6, 0, 0, &rules)
            .expect("limbo runtime Techno");
        let limbo_entity = sim.substrate.entities.get(limbo).unwrap();
        assert_eq!(limbo_entity.techno_ctor_random_word, limbo_word);
        assert!(limbo_entity.lifecycle.in_limbo);
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    }

    #[test]
    fn techno_constructor_spawn_manager_pool_draws_parent_then_children_and_cancels_as_one_graph() {
        let seed = 0xC701_0010;
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        install_american_house(&mut sim);
        let mut expected = SimRng::new(seed);
        let words = (0..4)
            .map(|_| (expected.next_u32() & 0xFFFF) as u16)
            .collect::<Vec<_>>();

        let parent_id = sim
            .construct_object_limbo_at_height("CARRIER", "Americans", 0, 0, 0, 0, &rules)
            .expect("factory-held carrier constructor");
        let parent = sim
            .substrate
            .entities
            .get(parent_id)
            .expect("carrier parent");
        assert_eq!(parent.techno_ctor_random_word, words[0]);
        let child_ids = parent
            .spawn_manager
            .as_ref()
            .expect("constructor spawn manager")
            .slots
            .iter()
            .map(|slot| slot.spawn.expect("constructor-filled spawn slot"))
            .collect::<Vec<_>>();
        assert_eq!(child_ids, vec![2, 3, 4]);
        for (index, child_id) in child_ids.iter().copied().enumerate() {
            let child = sim.substrate.entities.get(child_id).expect("spawn child");
            assert_eq!(child.techno_ctor_random_word, words[index + 1]);
            assert_eq!(child.spawn_owner_id, Some(parent_id));
            assert!(child.lifecycle.in_limbo && !child.lifecycle.cell_marked);
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());

        let after_constructor = sim.scenario_rng.logical_state();
        assert!(sim.discard_constructed_limbo(parent_id));
        assert!(sim.substrate.entities.is_empty());
        assert_eq!(sim.scenario_rng.logical_state(), after_constructor);
    }

    #[test]
    fn techno_constructor_slave_manager_pool_draws_parent_then_children_and_cancels_as_one_graph() {
        let seed = 0xC701_0011;
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        install_american_house(&mut sim);
        let mut expected = SimRng::new(seed);
        let words = (0..3)
            .map(|_| (expected.next_u32() & 0xFFFF) as u16)
            .collect::<Vec<_>>();

        let parent_id = sim
            .construct_object_limbo_at_height("SMIN", "Americans", 0, 0, 0, 0, &rules)
            .expect("factory-held slave miner constructor");
        let slave_ids = sim
            .production
            .slave_bindings
            .get(&parent_id)
            .expect("constructor slave manager")
            .clone();
        assert_eq!(slave_ids, vec![2, 3]);
        assert_eq!(
            sim.substrate
                .entities
                .get(parent_id)
                .unwrap()
                .techno_ctor_random_word,
            words[0]
        );
        for (index, slave_id) in slave_ids.iter().copied().enumerate() {
            let slave = sim.substrate.entities.get(slave_id).expect("slave child");
            assert_eq!(slave.techno_ctor_random_word, words[index + 1]);
            assert_eq!(
                slave.slave_harvester.as_ref().map(|slave| slave.master_id),
                Some(parent_id)
            );
            assert!(slave.lifecycle.in_limbo && !slave.lifecycle.cell_marked);
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());

        let after_constructor = sim.scenario_rng.logical_state();
        assert!(sim.discard_constructed_limbo(parent_id));
        assert!(sim.substrate.entities.is_empty());
        assert!(sim.production.slave_bindings.is_empty());
        assert_eq!(sim.scenario_rng.logical_state(), after_constructor);
    }

    #[test]
    fn techno_constructor_manager_pools_survive_delivery_without_reconstruction() {
        let rules = constructor_rules();
        for (seed, parent_type, expected_child_count) in [
            (0xC701_0012, "CARRIER", 3usize),
            (0xC701_0013, "SMIN", 2usize),
        ] {
            let mut sim = Simulation::with_seed(seed);
            install_constructor_test_playfield(&mut sim);
            let parent_id = sim
                .construct_object_limbo_at_height(parent_type, "Americans", 0, 0, 0, 0, &rules)
                .expect("held manager parent");
            let child_ids = if parent_type == "CARRIER" {
                sim.substrate
                    .entities
                    .get(parent_id)
                    .unwrap()
                    .spawn_manager
                    .as_ref()
                    .unwrap()
                    .slots
                    .iter()
                    .filter_map(|slot| slot.spawn)
                    .collect::<Vec<_>>()
            } else {
                sim.production.slave_bindings[&parent_id].clone()
            };
            assert_eq!(child_ids.len(), expected_child_count);
            let after_constructor = sim.scenario_rng.logical_state();

            assert_eq!(
                sim.unlimbo_held_production_object(
                    parent_id,
                    6,
                    5,
                    0,
                    0,
                    PlacementEvidence::EvaluateMark,
                    &rules,
                ),
                Some(parent_id)
            );
            assert_eq!(sim.scenario_rng.logical_state(), after_constructor);
            let retained_ids = if parent_type == "CARRIER" {
                sim.substrate
                    .entities
                    .get(parent_id)
                    .unwrap()
                    .spawn_manager
                    .as_ref()
                    .unwrap()
                    .slots
                    .iter()
                    .filter_map(|slot| slot.spawn)
                    .collect::<Vec<_>>()
            } else {
                sim.production.slave_bindings[&parent_id].clone()
            };
            assert_eq!(retained_ids, child_ids);
        }
    }

    #[test]
    fn techno_constructor_failed_parent_placement_discards_both_manager_pool_kinds_without_rewind()
    {
        let rules = constructor_rules();
        for (seed, parent_type, draw_count) in [
            (0xC701_0014, "CARRIER", 4usize),
            (0xC701_0015, "SMIN", 3usize),
        ] {
            let mut sim = Simulation::with_seed(seed);
            install_constructor_test_playfield(&mut sim);
            let mut expected = SimRng::new(seed);
            for _ in 0..draw_count {
                let _ = expected.next_u32();
            }

            assert!(
                sim.spawn_object_at_height(parent_type, "Americans", 1, 1, 0, 0, &rules)
                    .is_none()
            );
            assert!(sim.substrate.entities.is_empty());
            assert!(sim.production.slave_bindings.is_empty());
            assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
        }
    }

    #[test]
    fn techno_constructor_unit_can_enter_rejection_discards_eager_pool_without_refunding_draws() {
        let seed = 0xC701_0016;
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        install_constructor_test_playfield(&mut sim);
        install_constructor_flat_terrain(&mut sim);
        let mut expected = SimRng::new(seed);

        let blocker_word = (expected.next_u32() & 0xFFFF) as u16;
        // Parent construction and its three SpawnManager children all happen
        // before ObjectClass::Unlimbo asks UnitClass::Can_Enter_Cell. The first
        // authored Unit is already linked when the CARRIER row reaches that gate.
        for _ in 0..4 {
            let _ = expected.next_u32();
        }
        assert_eq!(
            sim.spawn_from_map(
                &[
                    map_entity("MTNK", EntityCategory::Unit, (6, 5)),
                    map_entity("CARRIER", EntityCategory::Unit, (6, 5)),
                ],
                Some(&rules),
                &BTreeMap::new(),
            ),
            1
        );
        let blocker_id = 1;
        assert_eq!(
            sim.substrate
                .entities
                .get(blocker_id)
                .unwrap()
                .techno_ctor_random_word,
            blocker_word
        );

        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
        assert_eq!(sim.substrate.next_stable_object_id, 6);
        assert_eq!(sim.substrate.entities.len(), 1);
        let blocker = sim.substrate.entities.get(blocker_id).unwrap();
        assert!(blocker.lifecycle.cell_marked && !blocker.lifecycle.in_limbo);
        assert!(sim.substrate.entities.values().all(|entity| {
            !matches!(sim.interner.resolve(entity.type_ref), "CARRIER" | "HORN")
        }));
    }

    #[test]
    fn techno_constructor_fixed_map_uses_native_category_order_after_prior_mark_draw() {
        let seed = 0xC701_0002;
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        install_constructor_test_playfield(&mut sim);
        install_american_house(&mut sim);
        let mut expected = SimRng::new(seed);
        assert_eq!(sim.scenario_rng.next_u32(), expected.next_u32());
        let mut invalid_owner = map_entity("MTNK", EntityCategory::Unit, (3, 3));
        invalid_owner.owner = "UnresolvableHouse".to_string();
        let entities = vec![
            map_entity("MISSING", EntityCategory::Unit, (2, 2)),
            invalid_owner,
            map_entity("MTNK", EntityCategory::Unit, (6, 5)),
            map_entity("ORCA", EntityCategory::Aircraft, (7, 5)),
            map_entity("E1", EntityCategory::Infantry, (8, 5)),
            map_entity("BASE", EntityCategory::Structure, (9, 5)),
            map_entity("BASE", EntityCategory::Structure, (1, 1)),
        ];
        let expected_words: Vec<u16> = (0..5)
            .map(|_| (expected.next_u32() & 0xFFFF) as u16)
            .collect();

        assert_eq!(
            sim.spawn_from_map(&entities, Some(&rules), &BTreeMap::new()),
            4
        );
        let actual_words: Vec<u16> = sim
            .substrate
            .entities
            .values()
            .map(|entity| entity.techno_ctor_random_word)
            .collect();
        assert_eq!(actual_words.as_slice(), &expected_words[..4]);
        assert!(sim.substrate.entities.get(5).is_none());
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    }

    #[test]
    fn techno_constructor_generated_projection_installs_without_a_second_draw() {
        let rules = constructor_rules();
        let entity = map_entity("MTNK", EntityCategory::Unit, (7, 9));
        let table = GeneratedTechnoInitTable::try_new([GeneratedTechnoInit {
            entity_index: 0,
            techno_type: "MTNK".to_string(),
            cell: (7, 9),
            techno_ctor_random_word: 0xA55A,
        }])
        .unwrap();
        let mut sim = Simulation::with_seed(0xC701_0003);
        install_american_house(&mut sim);
        let before = sim.scenario_rng.logical_state();

        assert_eq!(
            sim.spawn_generated_from_map_with_resolved(
                &[entity],
                &rules,
                &BTreeMap::new(),
                None,
                &table,
            )
            .unwrap(),
            1
        );
        assert_eq!(sim.scenario_rng.logical_state(), before);
        assert_eq!(
            sim.substrate
                .entities
                .get(1)
                .unwrap()
                .techno_ctor_random_word,
            0xA55A
        );

        assert!(matches!(
            GeneratedTechnoInitTable::try_new([
                GeneratedTechnoInit {
                    entity_index: 0,
                    techno_type: "MTNK".to_string(),
                    cell: (7, 9),
                    techno_ctor_random_word: 1,
                },
                GeneratedTechnoInit {
                    entity_index: 0,
                    techno_type: "MTNK".to_string(),
                    cell: (7, 9),
                    techno_ctor_random_word: 2,
                },
            ]),
            Err(GeneratedTechnoInitError::DuplicateEntityIndex(0))
        ));
    }

    #[test]
    fn generated_projection_validates_the_whole_table_before_any_mutation() {
        // Every missing/mismatched slot is deliberately after a valid index 0.
        // An inline validator would therefore construct or mark the first map
        // entity before discovering the fault; the shared postconditions below
        // prove the complete table remains a preflight transaction.
        let entities = [
            map_entity("MTNK", EntityCategory::Unit, (7, 9)),
            map_entity("MTNK", EntityCategory::Unit, (8, 9)),
        ];
        let valid_first = || GeneratedTechnoInit {
            entity_index: 0,
            techno_type: "MTNK".to_string(),
            cell: (7, 9),
            techno_ctor_random_word: 0x1111,
        };

        let missing = GeneratedTechnoInitTable::try_new([valid_first()]).unwrap();
        assert_generated_projection_rejects_before_mutation(
            0xC701_0004,
            &entities,
            &missing,
            GeneratedTechnoInitError::MissingEntityIndex(1),
        );

        let unexpected = GeneratedTechnoInitTable::try_new([
            valid_first(),
            GeneratedTechnoInit {
                entity_index: 2,
                techno_type: "MTNK".to_string(),
                cell: (9, 9),
                techno_ctor_random_word: 0x2222,
            },
        ])
        .unwrap();
        assert_generated_projection_rejects_before_mutation(
            0xC701_0005,
            &entities,
            &unexpected,
            GeneratedTechnoInitError::UnexpectedEntityIndex(2),
        );

        let type_mismatch = GeneratedTechnoInitTable::try_new([
            valid_first(),
            GeneratedTechnoInit {
                entity_index: 1,
                techno_type: "ORCA".to_string(),
                cell: (8, 9),
                techno_ctor_random_word: 0x3333,
            },
        ])
        .unwrap();
        assert_generated_projection_rejects_before_mutation(
            0xC701_0006,
            &entities,
            &type_mismatch,
            GeneratedTechnoInitError::IdentityMismatch {
                entity_index: 1,
                expected_type: "ORCA".to_string(),
                found_type: "MTNK".to_string(),
                expected_cell: (8, 9),
                found_cell: (8, 9),
            },
        );

        let cell_mismatch = GeneratedTechnoInitTable::try_new([
            valid_first(),
            GeneratedTechnoInit {
                entity_index: 1,
                techno_type: "MTNK".to_string(),
                cell: (9, 9),
                techno_ctor_random_word: 0x4444,
            },
        ])
        .unwrap();
        assert_generated_projection_rejects_before_mutation(
            0xC701_0007,
            &entities,
            &cell_mismatch,
            GeneratedTechnoInitError::IdentityMismatch {
                entity_index: 1,
                expected_type: "MTNK".to_string(),
                found_type: "MTNK".to_string(),
                expected_cell: (9, 9),
                found_cell: (8, 9),
            },
        );
    }

    #[test]
    fn techno_constructor_authored_upgrades_are_distinct_attached_live_entities() {
        let seed = 0xC701_0005;
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        let mut expected = SimRng::new(seed);
        let mut base = map_entity("BASE", EntityCategory::Structure, (10, 12));
        base.structure_upgrades = [Some("UP1".to_string()), Some("UP2".to_string()), None];
        let words = [
            (expected.next_u32() & 0xFFFF) as u16,
            (expected.next_u32() & 0xFFFF) as u16,
            (expected.next_u32() & 0xFFFF) as u16,
        ];

        assert_eq!(
            sim.spawn_from_map(&[base], Some(&rules), &BTreeMap::new()),
            3
        );
        let parent = sim.substrate.entities.get(1).unwrap();
        assert_eq!(parent.techno_ctor_random_word, words[0]);
        assert!(parent.lifecycle.cell_marked);
        for (stable_id, slot) in [(2, 0), (3, 1)] {
            let upgrade = sim.substrate.entities.get(stable_id).unwrap();
            assert_eq!(upgrade.techno_ctor_random_word, words[slot + 1]);
            assert_eq!(
                upgrade.structure_upgrade_link,
                Some(StructureUpgradeLink {
                    parent_stable_id: 1,
                    slot: slot as u8,
                })
            );
            assert!(!upgrade.lifecycle.in_limbo);
            assert!(!upgrade.lifecycle.cell_marked);
            assert!(upgrade.in_logic_vector);
            assert_eq!((upgrade.position.rx, upgrade.position.ry), (10, 12));
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
    }

    #[test]
    fn techno_constructor_failed_reveal_keeps_one_draw_and_reuses_identity() {
        let seed = 0xC701_0006;
        let rules = constructor_rules();
        let mut sim = Simulation::with_seed(seed);
        let mut expected = SimRng::new(seed);
        let word = (expected.next_u32() & 0xFFFF) as u16;
        let stable_id = sim
            .construct_object_limbo_at_height("MTNK", "Americans", 3, 3, 0, 0, &rules)
            .unwrap();

        assert!(
            sim.reveal_constructed_object_at_height(
                stable_id,
                3,
                3,
                0,
                0,
                PlacementEvidence::MarkFailed,
                &rules,
            )
            .is_none()
        );
        let held = sim.substrate.entities.get(stable_id).unwrap();
        assert!(held.lifecycle.in_limbo);
        assert_eq!(held.techno_ctor_random_word, word);
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
        assert!(sim.discard_constructed_limbo(stable_id));
        assert!(sim.substrate.entities.get(stable_id).is_none());

        let before_restore = sim.scenario_rng.logical_state();
        assert_eq!(
            sim.resolve_techno_constructor_word(TechnoConstructorInit::Restored(0x1357), None)
                .unwrap(),
            0x1357
        );
        assert_eq!(sim.scenario_rng.logical_state(), before_restore);
    }
}
