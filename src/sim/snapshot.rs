//! Simulation snapshot serialization for mid-match save/load.
//!
//! Serializes the full `Simulation` state into a compact binary blob via
//! bincode. Caches and event queues are `#[serde(skip)]`'d on `Simulation`
//! and must be rebuilt by the caller via `rebuild_caches_after_load()`.
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on sim/world (Simulation).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::sim::world::Simulation;

/// Bump this when the snapshot binary format changes in a breaking way.
// Bumped 13 -> 14 for the serialized occupancy entry-order fields used to rebuild
// the skipped CellClass-style occupancy cache after load.
// Bumped 14 -> 15: active-vector order + id/enter-order counters relocated under
// Simulation.substrate (ObjectSubstrate); bincode layout changed (state hash unchanged).
// Bumped 15 -> 16: EntityStore relocated under Simulation.substrate (Slice 1b); bincode
// layout changed (state hash unchanged — world_hash reads the store via the new path).
// Bumped 16 -> 17: MissionCom folded into state_hash (Slice 8); bincode layout
// unchanged (MissionCom already serialized since Slice 6), only the hash changed.
// Bumped 17 -> 18: Factory/Economy authority flip (P5b) — the factory registry +
// the per-house economy statistics are now serialized + hashed; the frames-timer
// per-item field progress_carry is removed from the hash (progress lives in
// Factory; remaining_base_frames stays as the sidebar-ETA mirror); next_insertion_seq
// + seq_carry fields removed (insertion_seq == front enqueue_order); the C1
// factory-step-before-house-tail ordering lock is folded in.
// Bumped 18 -> 19: queue-of-record retirement (P5d) — `queues_by_owner` + `BuildQueueItem`
// are retired; the FIFO queue-of-record moves into the registry (`Factory.queue` of
// `QueueEntry{type_id, enqueue_order, total_base_frames}` + the new active-build
// `Factory.active_total_base_frames`). The per-item `queues_by_owner` hash fold is removed;
// `remaining_base_frames` no longer exists (derived from `progress` at sidebar-view time,
// not hashed). bincode layout changes (the `queues_by_owner` field is gone, the registry
// gains fields), so the version MUST bump.
// Bumped 19 -> 20: S2 — `mission.current`/`substate` authority moves to dispatch time for
// scoped move units (arrival tick hashes Move) and load trusts the serialized MissionCom
// (post-load re-derive deleted). Layout is unchanged, but a pre-S2 save replayed on S2
// logic diverges on arrival ticks, so cross-version restores must be refused.
// Bumped 20 -> 21: per-cell radiation field (substrate Slice 7). `Simulation` gains the
// serialized `radiation` state (cell levels + site registry, both state-hashed) and
// `GameEntity` gains `immune_to_radiation`; RadLevel>0 detonations now deal periodic
// foot-unit damage, so a pre-21 save replayed on 21 logic diverges.
// Bumped 21 -> 22: ScenarioSession (SC-2) — `seed`/frame-clock/`GameOptions` move under
// `Simulation.session` and the session identity fields (map name, theater, bounds, MP
// start waypoints, slot->house) are serialized; bincode layout changes. The move itself
// is hash-neutral (golden baseline unshifted); the identity fields fold into the hash in
// the same slice (documented on the golden-harness constant).
// Bumped 22 -> 23: S3 — Unit barrel destinations are read per-object pre-death (kill-tick
// aim hold changes hashed FacingClass values on kill ticks) and idle machine-less Units
// hash mission Guard(5) instead of the legacy None placeholder. Layout unchanged, but a
// pre-S3 save replayed on S3 logic diverges on the first idle-unit tick, so cross-version
// restores must be refused. (21 and 22 were taken by the parallel radiation and
// ScenarioSession slices; the concurrent bumps merged as 22 -> 23.)
// Bumped 23 -> 24: S4a authoritative flip (Option B) — each live non-miner Unit's
// mission (+0xC4 tick_counter + derived_mission) is now committed at the per-object
// AI host (pre-movement, LogicVector order) instead of in movement_tick (scoped
// movers) / the Phase-9 tail (idle). Commit timing is the gamemd-faithful per-object
// point: a unit that retasks mid-tick (e.g. an idle Guard unit that opportunity-
// acquires a target during combat) now hashes the host-committed mission, not the
// end-of-tick re-derivation. Layout unchanged and the committed goldens are unshifted
// (those scenarios don't exercise mid-tick non-miner retasking), but a pre-S4a save
// replayed on S4a logic diverges on the first such tick, so cross-version restores
// must be refused.
// S4b: GameEntity gains the hashed `damage_particle_live_until` (`+0x308`-
// equivalent) field, folded into the state hash, so 24→25 re-baselines. The new
// field is zero for every entity in stock YR (the AI_Update spark gate is
// Cyborg-only, with no stock users), so the only hash shift is the extra per-
// entity zero in the fold — no behavior change to any committed golden scenario.
// Bumped 25 -> 26: HouseState gains the serialized + hashed native per-house
// difficulty field (Hard=0, Normal=1, Easy=2). A pre-26 save lacks the field and
// cannot preserve mixed-difficulty AI behavior after load.
// Bumped 26 -> 27: ObjectSubstrate gains the serialized AnimStore and GameEntity
// gains the authoritative damage-fire transition cache plus eight animation IDs.
// Bumped 27 -> 28: independent object-alive/limbo/cell lifecycle state,
// lifecycle bookkeeping, and the ordered pending-delete queue are serialized and
// hashed instead of being reconstructed from store/LogicVector presence.
// Bumped 28 -> 29: exact native-width Mission state, category readiness leaves,
// archived target/falling state, and raw locomotor-readiness inputs replace the
// reduced Mission schema and are serialized + hashed.
// Bumped 29 -> 30: the miner FSM cursor (`Miner.state`) retired from the
// serialized Miner component — `MissionCom.handler_state` is the cursor of
// record (Harvest handler absorption / substate-authority flip).
// Bumped 33 -> 34: ParticleSystemStore moved into ObjectSubstrate and became
// serialized; particle systems now share object IDs, LogicVector membership,
// and deferred finalization.
// Bumped 34 -> 35: Phase-0 persistence contract. ScenarioSession gains native
// HouseClass registration order and a wrapping 32-bit frame; frame-anchored
// timers replace reduced countdown state; HouseState gains MapIsClear;
// GameEntity persists MoveSound grace state and HomingTarget's object/cell
// discriminator; PassengerCargo persists one Size value per entry; animation
// membership is rebuilt rather than serialized. Production loading now also
// enforces exact map/rules/session metadata before stable-ID fixup.
// Bumped 35 -> 36: lifecycle target and animation identity state is persisted.
// Bumped 36 -> 37: process-global Main/MapGen RNG cursors are no longer
// serialized; production load retains their live pre-load process state.
// Bumped 37 -> 38: DriveLocomotionRuntime persists the independent head-to
// occupation footprint and whether the current-cell occupation was cleared.
// Bumped 38 -> 39: overlay wall ownership became authoritative persisted state.
// Bumped 39 -> 40: ObjectSubstrate persists the authoritative per-cell raw
// ground/deck occupation bytes instead of reconstructing them from object lists.
// Bumped 40 -> 41: HouseState gains the serialized MultiplayPassive house-type
// fact. Defeat evaluation and the game-over alive scan both skip passive houses,
// so it is an authoritative outcome input and cannot be re-derived on load —
// `rebuild_caches_after_load` takes no RuleSet. Serialized but NOT hashed.
// Bumped 41 -> 42: GameEntity gains the passive target-acquisition bookkeeping
// — `last_target_scan_frame` and `passively_acquired_target` — and its
// `passive_scan_timer` is now armed at the construction frame instead of left
// unarmed. All three are HASHED, so a v41 save written before this change
// restores into a world whose hash differs from a v41 written after it.
// Bumped 42 -> 43: `InfantryRuntime` gains `idle_action_timer` (two u32s), so
// the component grows from 3 to 11 bytes. The encoding is bincode, which is not
// self-describing — the decoder reads the next field's bytes unconditionally and
// a `#[serde(default)]` never fires for a short record. A v42 save read by this
// code would therefore pass the version check and then misread every byte after
// the first infantry entity. The bump turns that silent corruption into a clean
// rejection. The new field is also HASHED.
// Bumped 43 -> 44: `GameEntity` gains the spawn-manager pool
// (`spawn_manager: Option<SpawnManagerState>` — spawn type, missile family,
// regen/reload/kamikaze frames, both manager timers, both targets, manager
// mode, and a variable-length slot vector) and the child back-pointer
// (`spawn_owner_id: Option<u64>`). Same bincode trap as 42 -> 43: the encoding
// is not self-describing, so the decoder reads the next field's bytes
// unconditionally and `#[serde(default)]` never fires for a short record. A v43
// save read by this code would pass the version check and then misread every
// byte from the first entity onward — these two fields are on EVERY entity, not
// just spawner parents, so the corruption starts at entity one. Both are
// HASHED, but only when present (see `world_hash.rs`).
//
// Bumped 44 -> 45: `AnimOverlayState`'s `rate_ms`/`elapsed_ms` became
// `rate_logic_frames`/`elapsed_logic_frames`. Both fields were `u32` before and
// after, so the encoded record is exactly the same width and a v44 save
// deserializes without any error at all — it just means something else. gamemd
// counts an animation's frame delay in logic frames, not wall-clock time, so a
// stored `rate_ms` of 266 comes back as a 266-*frame* delay: the building
// animation is roughly 44x too slow and looks stopped. Identical width is what
// makes this dangerous rather than safe — the test a unit change has to pass is
// whether old bytes still deserialize to the correct meaning, not whether they
// deserialize at all. Presentation state, so NOT hashed.
// (44 is claimed by the in-flight spawner slice, which lands first; this jumps
// over it deliberately.)
//
// FORK WARNING — versions 41 through 46 are AMBIGUOUS. Two branches (this
// repo's dev and the foundations-contracts line merged via PR #109/#110)
// diverged from v40 and independently assigned 41..46 to entirely different
// layout changes. The two lineages are preserved verbatim above and below;
// a version tag in that range does NOT identify a unique layout, so no
// compatibility path may ever key on 41..46. The merge of the two branches
// lands on 47, whose layout is the union of both lineages' fields.
// [foundations lineage] Bumped 40 -> 41: persistent BulletClass-style projectile state is serialized.
// Bumped 41 -> 42: persistent BulletClass collision policy is serialized.
// Bumped 42 -> 43: TriggerRuntime latches now participate in the lockstep hash.
// Bumped 43 -> 44: TeamClass raw actions, deferred advance, timers, attachment
// identities, per-type counts, and non-CRC success state are authoritative.
// Bumped 44 -> 45: piggyback persistence stores one complete nested locomotor runtime.
// Bumped 45 -> 46: Tunnel and DropPod typed special-locomotor runtimes are
// serialized on GameEntity, including their phase and landing state.
// Bumped {41..46 fork} -> 47: merge of the two lineages; layout is the
// union of every field both sides added.
const SNAPSHOT_VERSION: u32 = 47;

/// Binary snapshot envelope — wraps the full `Simulation` state plus
/// compatibility hashes for the map and rules that were active at save time.
#[derive(Serialize, Deserialize)]
pub struct GameSnapshot {
    /// Format version — checked on load to reject incompatible saves.
    pub version: u32,
    /// Hash of the map file — caller verifies on load to ensure same map.
    pub map_hash: u64,
    /// Hash of the merged rules — caller verifies on load to ensure same rules.
    pub rules_hash: u64,
    /// Simulation tick at save time — stored in header for quick preview.
    pub tick: u64,
    /// Unix timestamp (seconds) when the save was created.
    pub save_timestamp: u64,
    /// Map name at save time — stored in header for quick preview.
    pub map_name: String,
    /// The full authoritative simulation state (caches excluded via serde skip).
    pub sim: Simulation,
}

/// Lightweight header extracted from a save file without deserializing the
/// full `Simulation`. Fields are laid out in the same order as `GameSnapshot`
/// so bincode can decode them as a prefix.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameSnapshotHeader {
    pub version: u32,
    pub map_hash: u64,
    pub rules_hash: u64,
    pub tick: u64,
    pub save_timestamp: u64,
    pub map_name: String,
}

/// Errors that can occur during snapshot deserialization.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot version {found} does not match expected {expected}")]
    VersionMismatch { expected: u32, found: u32 },
    #[error("map hash {found:#018x} does not match active map {expected:#018x}")]
    MapMismatch { expected: u64, found: u64 },
    #[error("rules hash {found:#018x} does not match active rules {expected:#018x}")]
    RulesMismatch { expected: u64, found: u64 },
    #[error("snapshot map name {found:?} does not match active map {expected:?}")]
    MapNameMismatch { expected: String, found: String },
    #[error(
        "snapshot header tick {header_tick} does not match serialized simulation tick {simulation_tick}"
    )]
    TickMetadataMismatch {
        header_tick: u64,
        simulation_tick: u64,
    },
    #[error(
        "snapshot header map name {header_map_name:?} does not match serialized session map name {simulation_map_name:?}"
    )]
    MapNameMetadataMismatch {
        header_map_name: String,
        simulation_map_name: String,
    },
    #[error("deserialization failed: {0}")]
    DeserializeFailed(#[from] bincode::Error),
}

/// Structural failures found before a deserialized simulation is admitted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotRestoreError {
    #[error("{registry} contains reserved object id 0")]
    ReservedObjectId { registry: &'static str },
    #[error(
        "{registry} registry key {registry_id} does not match the object's serialized id {object_id}"
    )]
    ObjectIdentityMismatch {
        registry: &'static str,
        registry_id: u64,
        object_id: u64,
    },
    #[error("object id {object_id} is registered by both {first_registry} and {second_registry}")]
    DuplicateObjectIdentity {
        object_id: u64,
        first_registry: &'static str,
        second_registry: &'static str,
    },
    #[error("next object id {next_id} is not after the highest restored object id {highest_id}")]
    ObjectIdCounterBehind { next_id: u64, highest_id: u64 },
    #[error(
        "next occupancy-enter order {next_order} is not after the highest restored order {highest_order}"
    )]
    OccupancyOrderCounterBehind { next_order: u64, highest_order: u64 },
    #[error("LogicVector contains duplicate object id {object_id}")]
    DuplicateLogicIdentity { object_id: u64 },
    #[error("LogicVector object id {object_id} has no restored registry identity")]
    MissingLogicIdentity { object_id: u64 },
    #[error(
        "{source_registry} object {source_id} field {field} references missing {target_registry} object {target_id}"
    )]
    UnresolvedObjectReference {
        source_registry: &'static str,
        source_id: u64,
        field: &'static str,
        target_registry: &'static str,
        target_id: u64,
    },
    #[error(
        "carrier {carrier_id} has {passenger_count} passengers but {size_count} saved Size entries"
    )]
    PassengerSizeCountMismatch {
        carrier_id: u64,
        passenger_count: usize,
        size_count: usize,
    },
    #[error(
        "carrier {carrier_id} saved total Size {saved_total}, but its entry sizes sum to {computed_total}"
    )]
    PassengerSizeTotalMismatch {
        carrier_id: u64,
        saved_total: u32,
        computed_total: u64,
    },
    #[error(
        "entity {object_id} has active MoveSound state but no restorable configured sound identity"
    )]
    ActiveMoveSoundUnresolvable { object_id: u64 },
}

/// Internal borrow-based envelope for serialization (avoids cloning Simulation).
#[derive(Serialize)]
struct GameSnapshotRef<'a> {
    version: u32,
    map_hash: u64,
    rules_hash: u64,
    tick: u64,
    save_timestamp: u64,
    map_name: String,
    sim: &'a Simulation,
}

impl GameSnapshot {
    fn serialize(
        sim: &Simulation,
        map_hash: u64,
        rules_hash: u64,
        map_name: &str,
        save_timestamp: u64,
    ) -> Vec<u8> {
        let snapshot = GameSnapshotRef {
            version: SNAPSHOT_VERSION,
            map_hash,
            rules_hash,
            tick: sim.session.tick,
            save_timestamp,
            map_name: map_name.to_string(),
            sim,
        };
        bincode::serialize(&snapshot).expect("snapshot serialization should not fail")
    }

    /// Serialize a production save with metadata owned by the active session.
    ///
    /// Map/rules digests come from the app's loaded content owners. Tick and map
    /// name are copied from `ScenarioSession`, so the envelope cannot disagree
    /// with the authoritative body. The app supplies the wall-clock timestamp;
    /// sim code remains clock-independent.
    pub fn save_validated(
        sim: &Simulation,
        map_hash: u64,
        rules_hash: u64,
        save_timestamp: u64,
    ) -> Vec<u8> {
        Self::serialize(
            sim,
            map_hash,
            rules_hash,
            &sim.session.map_name,
            save_timestamp,
        )
    }

    /// Test-only constructor for deliberately synthetic envelope metadata.
    #[cfg(test)]
    pub(crate) fn save(
        sim: &Simulation,
        map_hash: u64,
        rules_hash: u64,
        map_name: &str,
        save_timestamp: u64,
    ) -> Vec<u8> {
        Self::serialize(sim, map_hash, rules_hash, map_name, save_timestamp)
    }

    /// Deserialize a current-version snapshot without content validation.
    ///
    /// This exists for internal tests and diagnostics. Production restoration
    /// must use [`Self::load_validated`].
    pub(crate) fn load_unchecked(bytes: &[u8]) -> Result<GameSnapshot, SnapshotError> {
        let _ = Self::read_header(bytes)?;
        Ok(bincode::deserialize(bytes)?)
    }

    /// Compatibility alias for existing unit fixtures that intentionally use
    /// synthetic zero hashes and header-only map names.
    #[cfg(test)]
    pub(crate) fn load(bytes: &[u8]) -> Result<GameSnapshot, SnapshotError> {
        Self::load_unchecked(bytes)
    }

    /// Deserialize only when the save belongs to the exact active content.
    ///
    /// Version and compatibility metadata are rejected before the simulation
    /// body is admitted. The duplicated preview metadata must also agree with
    /// `ScenarioSession`; no warning/continue or zero-hash sentinel exists.
    /// The returned Main/MapGen fields are deserialize placeholders; the app's
    /// production load seam replaces them with the live process cursors.
    pub fn load_validated(
        bytes: &[u8],
        expected_map_hash: u64,
        expected_rules_hash: u64,
        expected_map_name: &str,
    ) -> Result<GameSnapshot, SnapshotError> {
        let header = Self::read_header(bytes)?;
        if header.map_hash != expected_map_hash {
            return Err(SnapshotError::MapMismatch {
                expected: expected_map_hash,
                found: header.map_hash,
            });
        }
        if header.rules_hash != expected_rules_hash {
            return Err(SnapshotError::RulesMismatch {
                expected: expected_rules_hash,
                found: header.rules_hash,
            });
        }
        if header.map_name != expected_map_name {
            return Err(SnapshotError::MapNameMismatch {
                expected: expected_map_name.to_string(),
                found: header.map_name,
            });
        }

        let snapshot: GameSnapshot = bincode::deserialize(bytes)?;
        if snapshot.tick != snapshot.sim.session.tick {
            return Err(SnapshotError::TickMetadataMismatch {
                header_tick: snapshot.tick,
                simulation_tick: snapshot.sim.session.tick,
            });
        }
        if snapshot.map_name != snapshot.sim.session.map_name {
            return Err(SnapshotError::MapNameMetadataMismatch {
                header_map_name: snapshot.map_name,
                simulation_map_name: snapshot.sim.session.map_name,
            });
        }
        Ok(snapshot)
    }

    /// Read only the header fields from a save file without deserializing the
    /// full Simulation. Useful for listing saves in the UI.
    pub fn read_header(bytes: &[u8]) -> Result<GameSnapshotHeader, SnapshotError> {
        let header: GameSnapshotHeader = bincode::deserialize(bytes)?;
        if header.version != SNAPSHOT_VERSION {
            return Err(SnapshotError::VersionMismatch {
                expected: SNAPSHOT_VERSION,
                found: header.version,
            });
        }
        Ok(header)
    }
}

#[derive(Debug, Clone, Copy)]
struct RestoredObjectIndex {
    highest_id: u64,
}

impl RestoredObjectIndex {
    fn build(
        sim: &Simulation,
    ) -> Result<(Self, BTreeMap<u64, &'static str>), SnapshotRestoreError> {
        let mut identities = BTreeMap::new();
        let mut highest_id = 0;

        for (registry_id, entity) in sim.substrate.entities.iter_sorted() {
            Self::register(
                &mut identities,
                "EntityStore",
                registry_id,
                entity.stable_id,
            )?;
            highest_id = highest_id.max(registry_id);
        }
        for (&registry_id, anim) in sim.substrate.anims.iter() {
            Self::register(&mut identities, "AnimStore", registry_id, anim.stable_id)?;
            highest_id = highest_id.max(registry_id);
        }
        for (&registry_id, system) in sim.substrate.particle_systems.iter() {
            Self::register(
                &mut identities,
                "ParticleSystemStore",
                registry_id,
                system.stable_id,
            )?;
            highest_id = highest_id.max(registry_id);
        }

        Ok((Self { highest_id }, identities))
    }

    fn register(
        identities: &mut BTreeMap<u64, &'static str>,
        registry: &'static str,
        registry_id: u64,
        object_id: u64,
    ) -> Result<(), SnapshotRestoreError> {
        if registry_id == 0 {
            return Err(SnapshotRestoreError::ReservedObjectId { registry });
        }
        if registry_id != object_id {
            return Err(SnapshotRestoreError::ObjectIdentityMismatch {
                registry,
                registry_id,
                object_id,
            });
        }
        if let Some(first_registry) = identities.insert(registry_id, registry) {
            return Err(SnapshotRestoreError::DuplicateObjectIdentity {
                object_id: registry_id,
                first_registry,
                second_registry: registry,
            });
        }
        Ok(())
    }
}

fn require_resolved_reference(
    resolves: bool,
    source_registry: &'static str,
    source_id: u64,
    field: &'static str,
    target_registry: &'static str,
    target_id: u64,
) -> Result<(), SnapshotRestoreError> {
    if resolves {
        Ok(())
    } else {
        Err(SnapshotRestoreError::UnresolvedObjectReference {
            source_registry,
            source_id,
            field,
            target_registry,
            target_id,
        })
    }
}

fn validate_nav_reference(
    target: &crate::sim::components::NavTargetRef,
    identities: &BTreeMap<u64, &'static str>,
    entity_ids: &BTreeSet<u64>,
    source_id: u64,
    field: &'static str,
) -> Result<(), SnapshotRestoreError> {
    match target {
        crate::sim::components::NavTargetRef::Cell { .. } => Ok(()),
        crate::sim::components::NavTargetRef::Entity { id }
        | crate::sim::components::NavTargetRef::Building { id } => require_resolved_reference(
            entity_ids.contains(id),
            "EntityStore",
            source_id,
            field,
            "EntityStore",
            *id,
        ),
        crate::sim::components::NavTargetRef::Object { id } => require_resolved_reference(
            identities.contains_key(id),
            "EntityStore",
            source_id,
            field,
            "object namespace",
            *id,
        ),
    }
}

fn validate_passenger_size_tables(sim: &Simulation) -> Result<(), SnapshotRestoreError> {
    for (carrier_id, entity) in sim.substrate.entities.iter_sorted() {
        let crate::sim::passenger::PassengerRole::Transport { cargo } = &entity.passenger_role
        else {
            continue;
        };
        if cargo.passengers.len() != cargo.passenger_sizes.len() {
            return Err(SnapshotRestoreError::PassengerSizeCountMismatch {
                carrier_id,
                passenger_count: cargo.passengers.len(),
                size_count: cargo.passenger_sizes.len(),
            });
        }
        let computed_total: u64 = cargo
            .passenger_sizes
            .iter()
            .map(|&size| u64::from(size))
            .sum();
        if computed_total != u64::from(cargo.total_size) {
            return Err(SnapshotRestoreError::PassengerSizeTotalMismatch {
                carrier_id,
                saved_total: cargo.total_size,
                computed_total,
            });
        }
    }
    Ok(())
}

fn restore_object_references(
    sim: &mut Simulation,
    identities: &BTreeMap<u64, &'static str>,
) -> Result<(), SnapshotRestoreError> {
    use crate::sim::combat::TargetKind;
    use crate::sim::game_entity::BunkerLink;
    use crate::sim::movement::homing_movement::HomingTarget;
    use crate::sim::passenger::PassengerRole;

    let entity_ids: BTreeSet<u64> = sim.substrate.entities.keys_sorted().into_iter().collect();
    let anim_ids: BTreeSet<u64> = sim.substrate.anims.iter().map(|(&id, _)| id).collect();

    // Swizzle::Apply has no unmatched-reference recovery path. Validate the
    // complete modeled pointer graph before mutating even weak references or
    // derived caches, so a failed restore remains an atomic rejection.
    for (entity_id, entity) in sim.substrate.entities.iter_sorted() {
        for contact_id in entity.radio_contacts.iter_live() {
            require_resolved_reference(
                entity_ids.contains(&contact_id),
                "EntityStore",
                entity_id,
                "radio_contacts",
                "EntityStore",
                contact_id,
            )?;
        }

        if let PassengerRole::Transport { cargo } = &entity.passenger_role {
            for &passenger_id in &cargo.passengers {
                require_resolved_reference(
                    entity_ids.contains(&passenger_id),
                    "EntityStore",
                    entity_id,
                    "passenger_role.cargo.passengers",
                    "EntityStore",
                    passenger_id,
                )?;
            }
        }

        if let Some(TargetKind::Entity(target_id)) =
            entity.attack_target.as_ref().map(|target| target.target)
        {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "attack_target",
                "EntityStore",
                target_id,
            )?;
        }
        if let Some(TargetKind::Entity(target_id)) = entity.suspended_attack_target {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "suspended_attack_target",
                "EntityStore",
                target_id,
            )?;
        }

        if let Some(target) = entity.navigation.suspended_nav_com.as_ref() {
            validate_nav_reference(
                target,
                identities,
                &entity_ids,
                entity_id,
                "navigation.suspended_nav_com",
            )?;
        }
        if let Some(target) = entity.navigation.nav_com_aux.as_ref() {
            validate_nav_reference(
                target,
                identities,
                &entity_ids,
                entity_id,
                "navigation.nav_com_aux",
            )?;
        }
        if let Some(target) = entity.navigation.nav_com.as_ref() {
            validate_nav_reference(
                target,
                identities,
                &entity_ids,
                entity_id,
                "navigation.nav_com",
            )?;
        }
        for target in &entity.navigation.nav_queue {
            validate_nav_reference(
                target,
                identities,
                &entity_ids,
                entity_id,
                "navigation.nav_queue",
            )?;
        }

        if let Some(target_id) = entity.dock_entered_with {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "dock_entered_with",
                "EntityStore",
                target_id,
            )?;
        }
        if let Some(target_id) = entity.capture_target {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "capture_target",
                "EntityStore",
                target_id,
            )?;
        }
        if let Some(plant) = entity.c4_plant.as_ref() {
            require_resolved_reference(
                entity_ids.contains(&plant.target_building_id),
                "EntityStore",
                entity_id,
                "c4_plant.target_building_id",
                "EntityStore",
                plant.target_building_id,
            )?;
        }

        if let Some(dock) = entity.dock_state.as_ref() {
            require_resolved_reference(
                entity_ids.contains(&dock.dock_building_id),
                "EntityStore",
                entity_id,
                "dock_state.dock_building_id",
                "EntityStore",
                dock.dock_building_id,
            )?;
        }
        if let Some(ammo) = entity.aircraft_ammo.as_ref()
            && let Some(target_id) = ammo.target_airfield
        {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "aircraft_ammo.target_airfield",
                "EntityStore",
                target_id,
            )?;
        }
        if let Some(miner) = entity.miner.as_ref() {
            if let Some(target_id) = miner.home_refinery {
                require_resolved_reference(
                    entity_ids.contains(&target_id),
                    "EntityStore",
                    entity_id,
                    "miner.home_refinery",
                    "EntityStore",
                    target_id,
                )?;
            }
            if let Some(target_id) = miner.reserved_refinery {
                require_resolved_reference(
                    entity_ids.contains(&target_id),
                    "EntityStore",
                    entity_id,
                    "miner.reserved_refinery",
                    "EntityStore",
                    target_id,
                )?;
            }
        }
        if let Some(slave) = entity.slave_harvester.as_ref() {
            require_resolved_reference(
                entity_ids.contains(&slave.master_id),
                "EntityStore",
                entity_id,
                "slave_harvester.master_id",
                "EntityStore",
                slave.master_id,
            )?;
        }

        let passenger_partner = match &entity.passenger_role {
            PassengerRole::Boarding {
                target_transport_id,
                ..
            } => Some((*target_transport_id, "passenger_role.boarding")),
            PassengerRole::Inside { transport_id } => {
                Some((*transport_id, "passenger_role.inside"))
            }
            PassengerRole::None | PassengerRole::Transport { .. } => None,
        };
        if let Some((target_id, field)) = passenger_partner {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                field,
                "EntityStore",
                target_id,
            )?;
        }

        if let Some(homing) = entity.homing_state.as_ref()
            && let Some(HomingTarget::Object(target_id)) = homing.target
        {
            require_resolved_reference(
                identities.contains_key(&target_id),
                "EntityStore",
                entity_id,
                "homing_state.target",
                "object namespace",
                target_id,
            )?;
        }

        for &anim_id in entity.damage_fire_anim_ids.iter().flatten() {
            require_resolved_reference(
                anim_ids.contains(&anim_id),
                "EntityStore",
                entity_id,
                "damage_fire_anim_ids",
                "AnimStore",
                anim_id,
            )?;
        }
        if let Some(target_id) = entity.bunker_occupant {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "bunker_occupant",
                "EntityStore",
                target_id,
            )?;
        }
        if let BunkerLink::Approaching(target_id) | BunkerLink::Installed(target_id) =
            entity.bunker_link
        {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "bunker_link",
                "EntityStore",
                target_id,
            )?;
        }
        if let Some(runtime) = entity.bunker_runtime.as_ref()
            && let Some(target_id) = runtime.installing_unit
        {
            require_resolved_reference(
                entity_ids.contains(&target_id),
                "EntityStore",
                entity_id,
                "bunker_runtime.installing_unit",
                "EntityStore",
                target_id,
            )?;
        }
    }

    for (&anim_id, anim) in sim.substrate.anims.iter() {
        if let Some(owner_id) = anim.owner_entity {
            require_resolved_reference(
                entity_ids.contains(&owner_id),
                "AnimStore",
                anim_id,
                "owner_entity",
                "EntityStore",
                owner_id,
            )?;
        }
    }

    for (&system_id, system) in sim.substrate.particle_systems.iter() {
        if let Some(owner_id) = system.owner_entity {
            require_resolved_reference(
                entity_ids.contains(&owner_id),
                "ParticleSystemStore",
                system_id,
                "owner_entity",
                "EntityStore",
                owner_id,
            )?;
        }
        if let Some(attached_id) = system.attached_entity {
            require_resolved_reference(
                entity_ids.contains(&attached_id),
                "ParticleSystemStore",
                system_id,
                "attached_entity",
                "EntityStore",
                attached_id,
            )?;
        }
    }

    for &object_id in &sim.substrate.pending_delete {
        require_resolved_reference(
            identities.contains_key(&object_id),
            "PendingDeleteList",
            object_id,
            "entry",
            "object namespace",
            object_id,
        )?;
    }

    for factory in sim.production.factory_shadow.iter_insertion_ordered() {
        if let Some(object_id) = factory.object.as_ref().and_then(|object| object.entity_id) {
            require_resolved_reference(
                entity_ids.contains(&object_id),
                "FactoryRegistry",
                u64::from(factory.owner.index()),
                "object.entity_id",
                "EntityStore",
                object_id,
            )?;
        }
    }

    // These are deliberate weak identities. Native expiry can leave the last
    // attacker dangling, and C4 kill credit may outlive its attacker.
    for entity in sim.substrate.entities.values_mut() {
        if entity
            .last_attacker_id
            .is_some_and(|id| !entity_ids.contains(&id))
        {
            entity.last_attacker_id = None;
        }
        if let Some(pending) = entity.pending_c4_detonation.as_mut()
            && !entity_ids.contains(&pending.attacker_id)
        {
            pending.attacker_id = 0;
        }
    }

    // These manager maps are derived/transitional mirrors rather than modeled
    // native pointer slots. Restore prunes them only after the authoritative
    // object graph has passed validation.
    for producers in sim.production.active_producer_by_owner.values_mut() {
        producers.retain(|_, id| entity_ids.contains(id));
    }
    sim.production
        .slave_bindings
        .retain(|master_id, slave_ids| {
            slave_ids.retain(|id| entity_ids.contains(id));
            entity_ids.contains(master_id)
        });
    sim.production.dock_reservations.cleanup_dead(&entity_ids);
    sim.production
        .depot_dock_reservations
        .cleanup_dead(&entity_ids);
    sim.production.airfield_docks.cleanup_dead(&entity_ids);

    // The produced-object link was validated above, so this legacy helper is
    // intentionally a no-op for every admitted snapshot.
    sim.production
        .factory_shadow
        .fixup_object_references(&entity_ids);

    Ok(())
}

impl Simulation {
    /// Validate and re-register a deserialized simulation before it can resume.
    ///
    /// Native load registers old identities and pointer slots, resolves them,
    /// then restores global active/cell membership. Rust stores stable IDs
    /// directly, so the equivalent transaction builds one global namespace,
    /// rejects any unmatched modeled native pointer, cleans only deliberate
    /// weak/derived identities, and reconstructs skipped indexes in dependency
    /// order.
    pub(crate) fn restore_after_snapshot_load(&mut self) -> Result<(), SnapshotRestoreError> {
        let (index, identities) = RestoredObjectIndex::build(self)?;
        if self.substrate.next_stable_object_id <= index.highest_id {
            return Err(SnapshotRestoreError::ObjectIdCounterBehind {
                next_id: self.substrate.next_stable_object_id,
                highest_id: index.highest_id,
            });
        }

        let highest_order = self
            .substrate
            .entities
            .values()
            .filter(|entity| entity.lifecycle.cell_marked)
            .map(|entity| entity.occupancy_enter_order)
            .max()
            .unwrap_or(0);
        let next_order = self.substrate.next_occupancy_enter_order.current();
        if next_order <= highest_order {
            return Err(SnapshotRestoreError::OccupancyOrderCounterBehind {
                next_order,
                highest_order,
            });
        }

        let mut seen_logic = BTreeSet::new();
        for &object_id in self.substrate.logic.as_slice() {
            if !seen_logic.insert(object_id) {
                return Err(SnapshotRestoreError::DuplicateLogicIdentity { object_id });
            }
            if !identities.contains_key(&object_id) {
                return Err(SnapshotRestoreError::MissingLogicIdentity { object_id });
            }
        }

        validate_passenger_size_tables(self)?;
        restore_object_references(self, &identities)?;

        // Rust's native-shaped re-registration order:
        // 1. class registry indexes, 2. Logic slots (including ParticleSystem),
        // 3. CellClass-style lists.
        self.substrate.entities.rebuild_owner_index();
        self.rebuild_logic_membership();
        self.substrate.occupancy =
            crate::sim::occupancy::OccupancyGrid::rebuild(&self.substrate.entities);
        self.substrate.cell_occupation =
            crate::sim::occupancy::CellOccupationGrid::rebuild(&self.substrate.entities);
        Ok(())
    }

    /// Recreate app-owned loop handles for serialized active MoveSound state.
    ///
    /// The configured identity is re-emitted as a transient sound event while
    /// the authoritative active/countdown bytes remain untouched. The local
    /// audio owner performs any process-global selection after its acceptance
    /// gates; snapshot restoration never advances an RNG itself.
    pub(crate) fn restore_move_sound_handles_after_load(
        &mut self,
        rules: &crate::rules::ruleset::RuleSet,
    ) -> Result<(), SnapshotRestoreError> {
        let object_ids = self.substrate.entities.keys_sorted();
        for object_id in object_ids {
            let Some(entity) = self.substrate.entities.get(object_id) else {
                continue;
            };
            if !entity.move_sound_active {
                continue;
            }

            let type_ref = entity.type_ref;
            let world = Self::movement_sound_world(entity);
            let Some(configured_sound) = self
                .object_type(type_ref, rules)
                .and_then(|object| object.move_sound.as_deref())
                .map(str::trim)
                .filter(|sound| !sound.is_empty() && !sound.eq_ignore_ascii_case("none"))
                .map(str::to_owned)
            else {
                return Err(SnapshotRestoreError::ActiveMoveSoundUnresolvable { object_id });
            };
            let Some(sound_id) = self.interner.get(&configured_sound) else {
                return Err(SnapshotRestoreError::ActiveMoveSoundUnresolvable { object_id });
            };

            self.sound_events
                .push(crate::sim::world::SimSoundEvent::AnimationStarted {
                    anim_id: object_id,
                    sound_id,
                    world,
                });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::locomotor_type::{MovementZone, SpeedType};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use crate::sim::world::{RevealOutcome, Simulation};
    use std::collections::BTreeMap;

    /// Helper: advance a sim by one tick with empty inputs.
    fn tick(sim: &mut Simulation) {
        let height_map = BTreeMap::new();
        sim.advance_tick(&[], None, &height_map, None, None, 67);
    }

    fn clear_terrain_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
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
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: crate::map::resolved_terrain::zone_class::GROUND,
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
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn flat_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(width as usize * height as usize);
        for ry in 0..height {
            for rx in 0..width {
                cells.push(clear_terrain_cell(rx, ry));
            }
        }
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    fn all_terrain_costs(terrain: &ResolvedTerrainGrid) -> BTreeMap<SpeedType, TerrainCostGrid> {
        let mut costs = BTreeMap::new();
        for speed_type in [
            SpeedType::Foot,
            SpeedType::Track,
            SpeedType::Wheel,
            SpeedType::Hover,
            SpeedType::Winged,
            SpeedType::Float,
            SpeedType::Amphibious,
            SpeedType::FloatBeach,
        ] {
            costs.insert(
                speed_type,
                TerrainCostGrid::from_resolved_terrain(terrain, speed_type),
            );
        }
        costs
    }

    fn rebuild_load_caches(sim: &mut Simulation, terrain: ResolvedTerrainGrid) {
        let terrain_costs = all_terrain_costs(&terrain);
        // Synthetic fixtures use unchecked snapshots and often bypass the
        // monotonic allocators. Rebuild their substrate caches directly; the
        // production path uses `restore_after_snapshot_load`.
        sim.substrate.entities.rebuild_owner_index();
        sim.rebuild_logic_membership();
        sim.substrate.occupancy =
            crate::sim::occupancy::OccupancyGrid::rebuild(&sim.substrate.entities);
        sim.substrate.cell_occupation =
            crate::sim::occupancy::CellOccupationGrid::rebuild(&sim.substrate.entities);
        sim.rebuild_caches_after_load(
            terrain,
            crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig::default(),
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            terrain_costs,
        );
    }

    fn cell_order(sim: &Simulation, rx: u16, ry: u16, layer: MovementLayer) -> Vec<u64> {
        sim.substrate
            .occupancy
            .get(rx, ry)
            .map(|occ| occ.iter_layer(layer).map(|o| o.entity_id).collect())
            .unwrap_or_default()
    }

    fn assert_zone_grids_equivalent(a: &ZoneGrid, b: &ZoneGrid) {
        assert_eq!(a.width, b.width);
        assert_eq!(a.height, b.height);
        for &mz in MovementZone::all_ground() {
            let map_a = a.map_for(mz).expect("zone map exists for movement zone");
            let map_b = b.map_for(mz).expect("zone map exists for movement zone");
            assert_eq!(map_a.zone_count, map_b.zone_count);
            for y in 0..a.height {
                for x in 0..a.width {
                    assert_eq!(
                        map_a.zone_at(x, y, MovementLayer::Ground),
                        map_b.zone_at(x, y, MovementLayer::Ground),
                        "ground zone mismatch for {mz:?} at ({x},{y})"
                    );
                    assert_eq!(
                        map_a.zone_at(x, y, MovementLayer::Bridge),
                        map_b.zone_at(x, y, MovementLayer::Bridge),
                        "bridge zone mismatch for {mz:?} at ({x},{y})"
                    );
                }
            }
            let adj_a = a
                .adjacency_for(mz)
                .expect("zone adjacency exists for movement zone");
            let adj_b = b
                .adjacency_for(mz)
                .expect("zone adjacency exists for movement zone");
            for zone in 0..=map_a.zone_count {
                assert_eq!(
                    adj_a.neighbors_of(zone),
                    adj_b.neighbors_of(zone),
                    "adjacency mismatch for {mz:?} zone {zone}"
                );
            }
        }
    }

    /// Prove snapshot round-trip preserves all authoritative state.
    ///
    /// 1. Create a Simulation, advance N ticks
    /// 2. Save snapshot -> bytes -> load snapshot
    /// 3. Advance both the loaded sim and a reference sim for M more ticks
    /// 4. Assert both reach the same state hash
    #[test]
    fn round_trip_preserves_state_hash() {
        // Create two identical simulations from the same seed.
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();

        // Advance both for 50 ticks to build up some state.
        for _ in 0..50 {
            tick(&mut sim_a);
            tick(&mut sim_b);
        }

        // Snapshot sim_a at tick 50.
        let hash_at_50 = sim_a.state_hash();
        let bytes = GameSnapshot::save(&sim_a, 0, 0, "test_map", 0);

        // Load the snapshot.
        let snapshot = GameSnapshot::load(&bytes).expect("load should succeed");
        let mut sim_loaded = snapshot.sim;

        // Verify the loaded sim has the same state hash as the original at tick 50.
        assert_eq!(
            sim_loaded.state_hash(),
            hash_at_50,
            "loaded snapshot must match original state hash at save point"
        );

        // Advance both the original and loaded sims for 50 more ticks.
        for _ in 0..50 {
            tick(&mut sim_a);
            tick(&mut sim_loaded);
        }

        // Both must reach the same state hash at tick 100.
        assert_eq!(
            sim_a.state_hash(),
            sim_loaded.state_hash(),
            "original and loaded sim must reach identical state after continued ticking"
        );

        // The reference sim (never serialized) must also match.
        for _ in 0..50 {
            tick(&mut sim_b);
        }
        assert_eq!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "reference sim (never serialized) must match serialized sim"
        );
    }

    #[test]
    fn version_mismatch_is_rejected() {
        let sim = Simulation::new();
        let mut bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);

        // Corrupt the version field (first 4 bytes in bincode little-endian).
        bytes[0] = 255;

        assert!(matches!(
            GameSnapshot::load(&bytes),
            Err(SnapshotError::VersionMismatch {
                expected: SNAPSHOT_VERSION,
                found: 255,
            })
        ));
    }

    #[test]
    fn gsi_04_07_v38_header_is_rejected_before_wall_owner_decode() {
        let bytes = bincode::serialize(&GameSnapshotHeader {
            version: 38,
            map_hash: 1,
            rules_hash: 2,
            tick: 3,
            save_timestamp: 4,
            map_name: "v38-layout".to_string(),
        })
        .expect("serialize v38 header only");

        assert!(matches!(
            GameSnapshot::load(&bytes),
            Err(SnapshotError::VersionMismatch {
                expected: SNAPSHOT_VERSION,
                found: 38,
            })
        ));
    }

    #[test]
    fn current_header_with_missing_body_reports_deserialization_failure() {
        let bytes = bincode::serialize(&GameSnapshotHeader {
            version: SNAPSHOT_VERSION,
            map_hash: 1,
            rules_hash: 2,
            tick: 3,
            save_timestamp: 4,
            map_name: "current-layout".to_string(),
        })
        .expect("serialize current header only");

        assert!(matches!(
            GameSnapshot::load(&bytes),
            Err(SnapshotError::DeserializeFailed(_))
        ));
    }

    /// Concurrent-slice ladder: radiation took 20 -> 21, ScenarioSession (SC-2)
    /// took 21 -> 22, S3 (per-object pre-death facing read + idle-Guard authority)
    /// took 22 -> 23, the S4a authoritative flip (per-object mission commit
    /// relocated to the AI host) took 23 -> 24, and S4b (the hashed
    /// `damage_particle_live_until` `+0x308`-equivalent field) took 24 -> 25,
    /// per-house native AI difficulty took 25 -> 26, and scheduler-owned
    /// animation persistence took 26 -> 27, and independent serialized lifecycle
    /// axes plus the pending-delete boundary took 27 -> 28, and exact Mission
    /// state/readiness schema took 28 -> 29, and the Harvest handler
    /// absorption (the miner FSM cursor retired into
    /// `MissionCom.handler_state`) took 29 -> 30. Particle systems reached 34;
    /// the consolidated Phase-0 persistence schema took 34 -> 35, and
    /// lifecycle target/animation identity state took 35 -> 36, and omission
    /// of process-global Main/MapGen RNG state took 36 -> 37, and serialized
    /// Drive occupation footprints took 37 -> 38, and authoritative wall
    /// ownership took 38 -> 39, and raw occupation bytes took 39 -> 40. Versions
    /// 41..46 are a FORK: dev assigned them to MultiplayPassive, passive-acquire
    /// bookkeeping, the infantry idle timer, the spawn-manager pool, and the
    /// anim-overlay unit change, while the foundations line assigned the same
    /// numbers to projectile state, trigger hashing, TeamClass state, piggyback
    /// persistence, and typed special locomotors. The merge unified both as 47.
    /// This pins it so a later accidental bump is caught.
    #[test]
    fn snapshot_version_is_47() {
        assert_eq!(super::SNAPSHOT_VERSION, 47);
    }

    #[test]
    fn gsi_04_12_raw_occupation_snapshot_roundtrip_preserves_both_planes() {
        let mut sim = Simulation::new();
        sim.substrate.raw_cell_occupation.mark_ground(17, 23, 0x23);
        sim.substrate.raw_cell_occupation.mark_deck(17, 23, 0xC4);
        sim.substrate.raw_cell_occupation.mark_ground(2, 31, 0x02);
        let expected_hash = sim.state_hash();

        let bytes = GameSnapshot::save(&sim, 0, 0, "gsi_04_12_raw_occupation", 0);
        assert_eq!(
            GameSnapshot::read_header(&bytes)
                .expect("current snapshot header")
                .version,
            47
        );

        let mut restored = GameSnapshot::load(&bytes).expect("current snapshot").sim;
        restored
            .restore_after_snapshot_load()
            .expect("restore transient caches without replacing raw bytes");

        assert_eq!(
            restored.substrate.raw_cell_occupation.ground_bits(17, 23),
            0x23
        );
        assert_eq!(
            restored.substrate.raw_cell_occupation.deck_bits(17, 23),
            0xC4
        );
        assert_eq!(
            restored.substrate.raw_cell_occupation.ground_bits(2, 31),
            0x02
        );
        assert_eq!(restored.state_hash(), expected_hash);
    }

    #[test]
    fn gsi_04_05_v40_roundtrip_restores_drive_footprint_and_cell_occupation() {
        use crate::sim::components::{DriveLocomotionRuntime, DriveOccupationFootprint};
        use crate::sim::game_entity::GameEntity;
        use crate::sim::occupancy::{CellOccupationGrid, VEHICLE_OCCUPATION_BIT};

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("AMERICANS");
        let type_ref = sim.interner.intern("MTNK");
        let entity_id = sim.allocate_stable_id();
        let mut entity = GameEntity::test_default(entity_id, "MTNK", "AMERICANS", 2, 2);
        entity.owner = owner;
        entity.type_ref = type_ref;
        sim.substrate.entities.insert(entity);
        sim.add_entity_occupancy(entity_id);

        let footprint = DriveOccupationFootprint {
            rx: 3,
            ry: 2,
            layer: MovementLayer::Ground,
        };
        sim.substrate
            .entities
            .get_mut(entity_id)
            .expect("Drive unit")
            .drive_locomotion = Some(DriveLocomotionRuntime {
            occupation_head_to: Some(footprint),
            current_occupation_cleared: true,
            ..Default::default()
        });
        sim.substrate.cell_occupation = CellOccupationGrid::rebuild(&sim.substrate.entities);
        let expected_hash = sim.state_hash();

        let bytes = GameSnapshot::save(&sim, 0, 0, "gsi_04_05", 0);
        assert_eq!(
            GameSnapshot::read_header(&bytes)
                .expect("current snapshot header")
                .version,
            47
        );

        let mut restored_a = GameSnapshot::load(&bytes).expect("current snapshot").sim;
        let mut restored_b = GameSnapshot::load(&bytes).expect("current snapshot").sim;
        restored_a
            .restore_after_snapshot_load()
            .expect("first deterministic rebuild");
        restored_b
            .restore_after_snapshot_load()
            .expect("second deterministic rebuild");

        for restored in [&restored_a, &restored_b] {
            let drive = restored
                .substrate
                .entities
                .get(entity_id)
                .expect("restored Drive unit")
                .drive_locomotion
                .as_ref()
                .expect("restored Drive runtime");
            assert_eq!(drive.occupation_head_to, Some(footprint));
            assert!(drive.current_occupation_cleared);
            assert_eq!(restored.state_hash(), expected_hash);
            assert_eq!(
                restored
                    .substrate
                    .cell_occupation
                    .vehicle_bits(2, 2, MovementLayer::Ground),
                0
            );
            assert_eq!(
                restored.substrate.cell_occupation.vehicle_bits(
                    footprint.rx,
                    footprint.ry,
                    footprint.layer
                ),
                VEHICLE_OCCUPATION_BIT
            );
        }
    }

    #[test]
    fn gsi_04_07_v40_roundtrip_restores_wall_owner() {
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("AMERICANS");
        let mut overlays = crate::sim::overlay_grid::OverlayGrid::new(8, 8);
        overlays.place_owned_wall(3, 4, 2, 0x1A, owner);
        sim.overlay_grid = Some(overlays);
        let expected_hash = sim.state_hash();
        let mut unowned = Simulation::new();
        let _ = unowned.interner.intern("AMERICANS");
        let mut unowned_overlays = crate::sim::overlay_grid::OverlayGrid::new(8, 8);
        unowned_overlays.place_overlay(3, 4, 2, 0x1A);
        unowned.overlay_grid = Some(unowned_overlays);
        assert_ne!(
            unowned.state_hash(),
            expected_hash,
            "wall ownership participates in deterministic state"
        );

        let bytes = GameSnapshot::save(&sim, 0, 0, "gsi_04_07", 0);
        assert_eq!(
            GameSnapshot::read_header(&bytes)
                .expect("current snapshot header")
                .version,
            47
        );
        let restored = GameSnapshot::load(&bytes).expect("current snapshot").sim;
        let cell = restored
            .overlay_grid
            .as_ref()
            .expect("overlay grid")
            .cell(3, 4);
        assert_eq!(cell.overlay_id, Some(2));
        assert_eq!(cell.overlay_data, 0x1A);
        assert_eq!(cell.wall_owner, Some(owner));
        assert_eq!(restored.state_hash(), expected_hash);
    }

    #[test]
    fn validated_load_requires_exact_content_and_session_metadata() {
        use crate::sim::command::{Command, CommandEnvelope};

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("AMERICANS");
        sim.session.map_name = "MAP01.MAP".to_string();
        sim.session.tick = 0x1_0000_0007;
        sim.session.binary_frame = u32::MAX - 2;
        sim.session.total_sim_ms = 12_345;
        sim.session.house_order.push(owner);
        sim.pending_commands.push(CommandEnvelope::new(
            owner,
            sim.session.tick + 3,
            Command::Stop { entity_id: 71 },
        ));
        sim.scatter_rng().next_u32();
        sim.weapon_spread_rng().next_u32();
        sim.mapgen_rng.next_u32();
        let rng_before = sim.rng_state();
        let process_default = crate::sim::rng::SimRng::new(0).logical_state();

        let bytes = GameSnapshot::save_validated(&sim, 0x11, 0x22, 0x33);
        let restored =
            GameSnapshot::load_validated(&bytes, 0x11, 0x22, "MAP01.MAP").expect("exact metadata");
        assert_eq!(restored.tick, sim.session.tick);
        assert_eq!(restored.map_name, sim.session.map_name);
        assert_eq!(restored.sim.session.binary_frame, u32::MAX - 2);
        assert_eq!(restored.sim.session.total_sim_ms, 12_345);
        assert_eq!(restored.sim.session.house_order, vec![owner]);
        assert_eq!(restored.sim.pending_commands, sim.pending_commands);
        assert_eq!(
            restored.sim.scenario_rng.logical_state(),
            rng_before.scenario
        );
        assert_eq!(restored.sim.main_rng.logical_state(), process_default);
        assert_eq!(restored.sim.mapgen_rng.logical_state(), process_default);

        assert!(matches!(
            GameSnapshot::load_validated(&bytes, 0x12, 0x22, "MAP01.MAP"),
            Err(SnapshotError::MapMismatch { .. })
        ));
        assert!(matches!(
            GameSnapshot::load_validated(&bytes, 0x11, 0x23, "MAP01.MAP"),
            Err(SnapshotError::RulesMismatch { .. })
        ));
        assert!(matches!(
            GameSnapshot::load_validated(&bytes, 0x11, 0x22, "MAP02.MAP"),
            Err(SnapshotError::MapNameMismatch { .. })
        ));

        let mut inconsistent = GameSnapshot::load_unchecked(&bytes).expect("current snapshot");
        inconsistent.tick = inconsistent.tick.wrapping_add(1);
        let inconsistent_bytes = bincode::serialize(&inconsistent).expect("corrupt tick metadata");
        assert!(matches!(
            GameSnapshot::load_validated(&inconsistent_bytes, 0x11, 0x22, "MAP01.MAP"),
            Err(SnapshotError::TickMetadataMismatch { .. })
        ));

        inconsistent.tick = inconsistent.sim.session.tick;
        inconsistent.sim.session.map_name = "OTHER.MAP".to_string();
        let inconsistent_bytes = bincode::serialize(&inconsistent).expect("corrupt map metadata");
        assert!(matches!(
            GameSnapshot::load_validated(&inconsistent_bytes, 0x11, 0x22, "MAP01.MAP"),
            Err(SnapshotError::MapNameMetadataMismatch { .. })
        ));
    }

    #[test]
    fn restore_validates_references_then_rebuilds_logic_particles_and_cells() {
        use crate::rules::particle_system_type::ParticleSystemTypeId;
        use crate::sim::game_entity::GameEntity;
        use crate::sim::particles::ParticleSystem;
        use crate::util::fixed_math::SimFixed;
        use glam::IVec3;

        let mut sim = Simulation::new();
        sim.session.map_name = "RESTORE.MAP".to_string();
        let owner = sim.interner.intern("AMERICANS");
        let type_ref = sim.interner.intern("MTNK");

        let entity_id = sim.allocate_stable_id();
        let mut entity = GameEntity::test_default(entity_id, "MTNK", "AMERICANS", 5, 6);
        entity.owner = owner;
        entity.type_ref = type_ref;
        entity.last_attacker_id = Some(999);
        sim.substrate.entities.insert(entity);
        sim.add_entity_occupancy(entity_id);

        let particle_id = sim.allocate_stable_id();
        sim.substrate.particle_systems.insert(ParticleSystem {
            stable_id: particle_id,
            in_logic_vector: false,
            type_id: ParticleSystemTypeId(0),
            coords: IVec3::ZERO,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(0),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1d,
            marked_for_deletion: false,
            directionless: false,
            attached_entity: Some(entity_id),
            owner_entity: Some(entity_id),
            target_coords: IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        });
        sim.set_logic_order_for_test(vec![particle_id, entity_id]);

        let bytes = GameSnapshot::save_validated(&sim, 7, 8, 9);
        let mut restored = GameSnapshot::load_validated(&bytes, 7, 8, "RESTORE.MAP")
            .expect("strict snapshot")
            .sim;
        restored
            .restore_after_snapshot_load()
            .expect("valid restored object graph");

        assert_eq!(
            restored.live_object_order_snapshot(),
            vec![particle_id, entity_id]
        );
        assert!(
            restored
                .substrate
                .particle_systems
                .get(particle_id)
                .expect("particle system")
                .in_logic_vector
        );
        assert!(
            restored
                .substrate
                .entities
                .get(entity_id)
                .expect("entity")
                .in_logic_vector
        );
        assert!(
            restored
                .substrate
                .occupancy
                .contains_entity(5, 6, entity_id)
        );
        assert_eq!(
            restored
                .substrate
                .entities
                .get(entity_id)
                .expect("entity")
                .last_attacker_id,
            None
        );
        let system = restored
            .substrate
            .particle_systems
            .get(particle_id)
            .expect("particle system");
        assert_eq!(system.owner_entity, Some(entity_id));
        assert_eq!(system.attached_entity, Some(entity_id));
    }

    #[test]
    fn restore_rejects_unresolved_native_pointer_before_weak_cleanup() {
        use crate::rules::particle_system_type::ParticleSystemTypeId;
        use crate::sim::game_entity::GameEntity;
        use crate::sim::particles::ParticleSystem;
        use crate::util::fixed_math::SimFixed;
        use glam::IVec3;

        let mut sim = Simulation::new();
        let entity_id = sim.allocate_stable_id();
        let mut entity = GameEntity::test_default(entity_id, "MTNK", "AMERICANS", 5, 6);
        entity.last_attacker_id = Some(999);
        sim.substrate.entities.insert(entity);

        let particle_id = sim.allocate_stable_id();
        sim.substrate.particle_systems.insert(ParticleSystem {
            stable_id: particle_id,
            in_logic_vector: false,
            type_id: ParticleSystemTypeId(0),
            coords: IVec3::ZERO,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(0),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1d,
            marked_for_deletion: false,
            directionless: false,
            attached_entity: Some(999),
            owner_entity: Some(entity_id),
            target_coords: IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        });

        assert_eq!(
            sim.restore_after_snapshot_load(),
            Err(SnapshotRestoreError::UnresolvedObjectReference {
                source_registry: "ParticleSystemStore",
                source_id: particle_id,
                field: "attached_entity",
                target_registry: "EntityStore",
                target_id: 999,
            })
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(entity_id)
                .expect("entity")
                .last_attacker_id,
            Some(999),
            "failed restoration must not clean a later weak reference"
        );
        assert_eq!(
            sim.substrate
                .particle_systems
                .get(particle_id)
                .expect("particle system")
                .attached_entity,
            Some(999),
            "failed restoration must not sanitize the unresolved strong reference"
        );
    }

    #[test]
    fn restore_recreates_active_move_sound_without_rng_or_countdown_mutation() {
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        use crate::sim::game_entity::GameEntity;
        use crate::sim::world::SimSoundEvent;

        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n0=TEST\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TEST]\nMoveSound=VMove\n",
        ))
        .expect("move-sound rules");
        let mut sim = Simulation::new();
        sim.session.map_name = "MOVESOUND.MAP".to_string();
        let owner = sim.interner.intern("AMERICANS");
        let type_ref = sim.interner.intern("TEST");
        let sound_id = sim.interner.intern("VMove");
        let entity_id = sim.allocate_stable_id();
        let mut entity = GameEntity::test_default(entity_id, "TEST", "AMERICANS", 7, 9);
        entity.owner = owner;
        entity.type_ref = type_ref;
        entity.move_sound_active = true;
        entity.move_sound_countdown = 2;
        sim.substrate.entities.insert(entity);

        let bytes = GameSnapshot::save_validated(&sim, 17, 18, 19);
        let mut restored = GameSnapshot::load_validated(&bytes, 17, 18, "MOVESOUND.MAP")
            .expect("strict snapshot")
            .sim;
        restored
            .restore_after_snapshot_load()
            .expect("valid restored object graph");
        restored.resolve_type_handles(&rules);
        let rng_before = restored.rng_state();
        restored
            .restore_move_sound_handles_after_load(&rules)
            .expect("active move sound resolves");

        assert_eq!(restored.rng_state(), rng_before);
        let entity = restored.substrate.entities.get(entity_id).expect("entity");
        assert!(entity.move_sound_active);
        assert_eq!(entity.move_sound_countdown, 2);
        assert!(matches!(
            restored.sound_events.as_slice(),
            [SimSoundEvent::AnimationStarted {
                anim_id,
                sound_id: restored_sound,
                ..
            }] if *anim_id == entity_id && *restored_sound == sound_id
        ));
    }

    #[test]
    fn exact_mission_schema_round_trips_raw_ids_leaves_archives_and_locomotors() {
        use crate::rules::locomotor_type::LocomotorKind;
        use crate::sim::combat::TargetKind;
        use crate::sim::game_entity::GameEntity;
        use crate::sim::mission::state::MissionTestFixture;
        use crate::sim::mission::{MissionDispatchTimer, MissionId, MissionLeafState};
        use crate::sim::movement::locomotor::LocomotorState;
        use crate::sim::movement::locomotor_ready::LocomotorReadyState;

        let leaves = [
            MissionLeafState::unit_raw_for_test(1, 2, 3, 4),
            MissionLeafState::infantry_raw_for_test(5, 41),
            MissionLeafState::aircraft_raw_for_test(6, 7, true),
            MissionLeafState::building_raw_for_test(8),
            MissionLeafState::unit_raw_for_test(9, 10, 11, 12),
            MissionLeafState::infantry_raw_for_test(13, -1),
        ];

        let mut sim = Simulation::new();
        for index in 0..6 {
            let id = index as u64 + 1;
            let mut entity = GameEntity::test_default(id, "MTNK", "Americans", 5, 5);
            entity.mission_leaf = leaves[index];
            entity.suspended_attack_target = Some(if index & 1 == 0 {
                TargetKind::Entity(100 + id)
            } else {
                TargetKind::Cell(index as u16, (index + 1) as u16)
            });
            entity.set_object_is_falling_down_for_test(index as u8 + 1);
            entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
            if index == 0 {
                entity.mission.apply_test_fixture(MissionTestFixture {
                    current: MissionId::from_raw(i32::MIN),
                    suspended: MissionId::from_raw(0x1234_5678),
                    queued: MissionId::from_raw(i32::MAX),
                    movement_bypass_latch: 0xa5,
                    handler_state: 0x1122_3344,
                    mission_start_frame: 0x5566_7788,
                    ai_counter: 0x99aa_bbcc,
                    dispatch_timer: MissionDispatchTimer::from_raw(-17, -29),
                });
            }
            sim.substrate.entities.insert(entity);
        }

        let bytes = GameSnapshot::save(&sim, 1, 2, "mission-schema", 3);
        let loaded = GameSnapshot::load(&bytes).expect("load exact Mission schema");

        for index in 0..6 {
            let entity = loaded
                .sim
                .substrate
                .entities
                .get(index as u64 + 1)
                .expect("restored Mission fixture");
            let id = index as u64 + 1;
            let expected_suspended_target = Some(if index & 1 == 0 {
                TargetKind::Entity(100 + id)
            } else {
                TargetKind::Cell(index as u16, (index + 1) as u16)
            });
            assert_eq!(entity.mission_leaf, leaves[index]);
            assert_eq!(
                entity.suspended_attack_target, expected_suspended_target,
                "suspended TargetKind variant and payload must round-trip"
            );
            assert_eq!(entity.object_is_falling_down, index as u8 + 1);
        }

        let first = loaded.sim.substrate.entities.get(1).unwrap();
        assert_eq!(first.mission.current(), MissionId::from_raw(i32::MIN));
        assert_eq!(first.mission.suspended(), MissionId::from_raw(0x1234_5678));
        assert_eq!(first.mission.queued(), MissionId::from_raw(i32::MAX));
        assert_eq!(
            first.mission.dispatch_timer(),
            MissionDispatchTimer::from_raw(-17, -29)
        );
    }

    #[test]
    fn entity_construction_frame_round_trips_as_dispatch_start() {
        use crate::map::entities::EntityCategory;
        use crate::sim::components::Health;
        use crate::sim::game_entity::GameEntity;

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("MTNK");
        let entity = GameEntity::new_at_frame(
            1,
            5,
            5,
            0,
            0,
            owner,
            Health {
                current: 100,
                max: 100,
            },
            type_ref,
            EntityCategory::Unit,
            0,
            5,
            true,
            37,
        );
        sim.substrate.entities.insert(entity);

        let bytes = GameSnapshot::save(&sim, 0, 0, "frame-37", 0);
        let loaded = GameSnapshot::load(&bytes).expect("load frame-37 entity");
        assert_eq!(
            loaded
                .sim
                .substrate
                .entities
                .get(1)
                .unwrap()
                .mission
                .dispatch_timer()
                .start_frame(),
            37
        );
    }

    #[test]
    fn house_difficulty_round_trips_through_snapshot() {
        use crate::sim::house_state::{HouseDifficulty, HouseState};

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Computer1");
        let mut house = HouseState::new(owner, 0, None, false, 0, 10);
        house.difficulty = HouseDifficulty::Easy;
        sim.houses.insert(owner, house);
        let expected_hash = sim.state_hash();

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let loaded = GameSnapshot::load(&bytes).expect("load should succeed");

        assert_eq!(
            loaded.sim.houses.get(&owner).map(|house| house.difficulty),
            Some(HouseDifficulty::Easy),
        );
        assert_eq!(loaded.sim.state_hash(), expected_hash);
    }

    /// `AttackTarget::for_cell` survives serialize → deserialize as the same
    /// `TargetKind::Cell` variant (regression for SNAPSHOT_VERSION 4 → 5).
    #[test]
    fn cell_attack_target_round_trips_through_snapshot() {
        use crate::sim::combat::{AttackTarget, TargetKind};
        use crate::sim::game_entity::GameEntity;

        let mut sim = Simulation::new();
        let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        entity.attack_target = Some(AttackTarget::for_cell(50, 50));
        sim.substrate.entities.insert(entity);

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let loaded = GameSnapshot::load(&bytes).expect("load should succeed");
        let restored = loaded
            .sim
            .substrate
            .entities
            .get(1)
            .expect("entity should be restored")
            .attack_target
            .as_ref()
            .expect("attack_target should be restored");
        assert!(matches!(restored.target, TargetKind::Cell(50, 50)));
    }

    /// Reveal registers at the tail; a stored-but-unrevealed (limbo) object is
    /// absent from the active order until revealed. (DRIFT 2 / ledger 9)
    #[test]
    fn limbo_object_registers_only_on_reveal() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        // Stored but not revealed: present in the store, absent from the order.
        sim.substrate
            .entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        assert!(sim.substrate.entities.contains(1));
        assert!(!sim.live_object_order_snapshot().contains(&1));
        // Reveal both: tail-append in reveal order, not sorted.
        sim.substrate
            .entities
            .insert(GameEntity::test_default(2, "MTNK", "Americans", 6, 6));
        sim.register_live_object(2);
        sim.register_live_object(1);
        assert_eq!(sim.live_object_order_snapshot(), vec![2, 1]);
    }

    /// The active order is serialized directly and restored verbatim — not
    /// re-derived, not sorted. (ledger 13)
    #[test]
    fn saveload_restores_live_object_order_verbatim() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        for id in [10u64, 20, 30] {
            sim.substrate
                .entities
                .insert(GameEntity::test_default(id, "MTNK", "Americans", 5, 5));
            sim.register_live_object(id);
        }
        // Force an order whose sequence differs from stable-id order.
        sim.set_logic_order_for_test(vec![20, 10, 30]);

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        assert_eq!(restored.live_object_order_snapshot(), vec![20, 10, 30]);
    }

    /// After load, membership is rebuilt from the order; a restored member
    /// unregisters exactly once (no stale entry) and re-registers without
    /// duplicating (no double-add). Avoids the §3.4 hazard. (ledger 14)
    #[test]
    fn saveload_restored_member_removes_cleanly() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        sim.register_live_object(1);

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let mut restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        // Real load-path step: membership flags are false straight after deserialize.
        restored.rebuild_logic_membership();

        // Unregister removes exactly once — no stale entry left behind.
        restored.unregister_live_object(1);
        assert!(!restored.live_object_order_snapshot().contains(&1));
        // Re-register appends once — no double-add.
        restored.register_live_object(1);
        assert_eq!(
            restored
                .live_object_order_snapshot()
                .iter()
                .filter(|&&x| x == 1)
                .count(),
            1
        );
    }

    /// Rust snapshots preserve a dead-limbo object's independent state and the
    /// ordered pending-delete boundary instead of reconstructing either from
    /// global storage or LogicVector membership.
    #[test]
    fn lifecycle_authority_pending_boundary_roundtrips_queue_and_state() {
        use crate::sim::game_entity::GameEntity;

        let mut sim = Simulation::new();
        let mut entity = GameEntity::test_default(7, "MTNK", "Americans", 5, 5);
        entity.lifecycle.object_alive = false;
        entity.lifecycle.in_limbo = true;
        entity.lifecycle.cell_marked = false;
        entity.in_logic_vector = false;
        entity.dying = true;
        entity.dirty_rect_eligible = true;
        entity.owned_count_released = true;
        sim.substrate.entities.insert(entity);
        sim.substrate.pending_delete.extend([7, 3, 7]);
        let hash_before = sim.state_hash();

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        let restored_entity = restored.substrate.entities.get(7).expect("entity restored");

        assert!(!restored_entity.lifecycle.object_alive);
        assert!(restored_entity.lifecycle.in_limbo);
        assert!(!restored_entity.lifecycle.cell_marked);
        assert!(!restored_entity.in_logic_vector);
        assert!(restored_entity.dying);
        assert!(restored_entity.dirty_rect_eligible);
        assert!(restored_entity.owned_count_released);
        assert_eq!(restored.substrate.pending_delete, vec![7, 3, 7]);
        assert_eq!(restored.state_hash(), hash_before);
    }

    #[test]
    fn lifecycle_authority_logic_rebuild_does_not_rederive_limbo_or_mark() {
        use crate::sim::game_entity::GameEntity;

        let mut sim = Simulation::new();
        let mut off_cell_member = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        off_cell_member.lifecycle.in_limbo = false;
        off_cell_member.lifecycle.cell_marked = false;
        sim.substrate.entities.insert(off_cell_member);
        sim.substrate
            .logic
            .try_push(1)
            .expect("logic fixture append");

        let mut marked_non_member = GameEntity::test_default(2, "MTNK", "Americans", 6, 6);
        marked_non_member.lifecycle.in_limbo = false;
        marked_non_member.lifecycle.cell_marked = true;
        sim.substrate.entities.insert(marked_non_member);

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let mut restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        restored.rebuild_logic_membership();

        let member = restored.substrate.entities.get(1).expect("member restored");
        assert!(member.in_logic_vector);
        assert!(!member.lifecycle.in_limbo);
        assert!(!member.lifecycle.cell_marked);

        let non_member = restored
            .substrate
            .entities
            .get(2)
            .expect("non-member restored");
        assert!(!non_member.in_logic_vector);
        assert!(!non_member.lifecycle.in_limbo);
        assert!(non_member.lifecycle.cell_marked);
    }

    #[test]
    fn lifecycle_authority_bookkeeping_facts_roundtrip_and_change_state_hash() {
        use crate::sim::game_entity::GameEntity;

        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        let default_hash = sim.state_hash();

        let entity = sim.substrate.entities.get_mut(1).expect("fixture entity");
        entity.dirty_rect_eligible = true;
        entity.owned_count_released = true;
        let changed_hash = sim.state_hash();
        assert_ne!(default_hash, changed_hash);

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        let restored_entity = restored.substrate.entities.get(1).expect("entity restored");
        assert!(restored_entity.dirty_rect_eligible);
        assert!(restored_entity.owned_count_released);
        assert_eq!(restored.state_hash(), changed_hash);
    }

    #[test]
    fn saveload_occupancy_list_order_matches_incremental() {
        use crate::map::entities::EntityCategory;
        use crate::sim::game_entity::GameEntity;

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");

        let mut structure = GameEntity::test_default(100, "GAPOWR", "Americans", 5, 5);
        structure.owner = owner;
        structure.category = EntityCategory::Structure;
        sim.substrate.entities.insert(structure);
        sim.add_entity_occupancy(100);

        let mut older_mobile = GameEntity::test_default(50, "MTNK", "Americans", 5, 5);
        older_mobile.owner = owner;
        older_mobile.category = EntityCategory::Unit;
        sim.substrate.entities.insert(older_mobile);
        sim.add_entity_occupancy(50);

        let mut newer_mobile = GameEntity::test_default(10, "HTNK", "Americans", 5, 5);
        newer_mobile.owner = owner;
        newer_mobile.category = EntityCategory::Unit;
        sim.substrate.entities.insert(newer_mobile);
        sim.add_entity_occupancy(10);

        let incremental = cell_order(&sim, 5, 5, MovementLayer::Ground);
        assert_eq!(incremental, vec![10, 50, 100]);
        let hash_at_save = sim.state_hash();

        let bytes = GameSnapshot::save(&sim, 0, 0, "order_test", 0);
        let mut restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        rebuild_load_caches(&mut restored, flat_terrain(8, 8));

        assert_eq!(
            cell_order(&restored, 5, 5, MovementLayer::Ground),
            incremental,
            "rebuilt occupancy cache must match the incremental CellClass list order"
        );
        assert_eq!(
            restored.state_hash(),
            hash_at_save,
            "cache rebuild must not change authoritative save state"
        );
    }

    #[test]
    fn saveload_rebuild_is_deterministic() {
        use crate::map::entities::EntityCategory;
        use crate::sim::game_entity::GameEntity;

        let terrain = flat_terrain(8, 8);
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        for (stable_id, type_id, category, rx, ry) in [
            (3, "GAPOWR", EntityCategory::Structure, 2, 2),
            (1, "MTNK", EntityCategory::Unit, 3, 2),
            (2, "E1", EntityCategory::Infantry, 3, 2),
        ] {
            let mut entity = GameEntity::test_default(stable_id, type_id, "Americans", rx, ry);
            entity.owner = owner;
            entity.category = category;
            if category == EntityCategory::Infantry {
                entity.sub_cell = Some(2);
            }
            sim.substrate.entities.insert(entity);
            sim.add_entity_occupancy(stable_id);
        }
        let bytes = GameSnapshot::save(&sim, 0, 0, "deterministic_rebuild", 0);

        let mut a = GameSnapshot::load(&bytes)
            .expect("first load should succeed")
            .sim;
        let mut b = GameSnapshot::load(&bytes)
            .expect("second load should succeed")
            .sim;
        rebuild_load_caches(&mut a, terrain.clone());
        rebuild_load_caches(&mut b, terrain);

        assert_eq!(a.terrain_costs, b.terrain_costs);
        assert_eq!(cell_order(&a, 3, 2, MovementLayer::Ground), vec![2, 1]);
        assert_eq!(
            cell_order(&a, 3, 2, MovementLayer::Ground),
            cell_order(&b, 3, 2, MovementLayer::Ground)
        );

        let path_a = PathGrid::from_resolved_terrain_with_bridges(
            a.resolved_terrain.as_ref().expect("terrain restored"),
            a.bridge_state.as_ref(),
        );
        let path_b = PathGrid::from_resolved_terrain_with_bridges(
            b.resolved_terrain.as_ref().expect("terrain restored"),
            b.bridge_state.as_ref(),
        );
        assert_eq!(path_a, path_b);

        a.rebuild_zone_grid(&path_a);
        b.rebuild_zone_grid(&path_b);
        assert_zone_grids_equivalent(
            a.zone_grid.as_ref().expect("zone grid rebuilt"),
            b.zone_grid.as_ref().expect("zone grid rebuilt"),
        );
        assert_eq!(a.state_hash(), b.state_hash());
    }

    // --- Slice 1: reveal/conceal/unlimbo/uninit lifecycle chokepoint ---

    /// `reveal` adds a member; `conceal` removes it from the order but keeps the
    /// store slot (limbo).
    #[test]
    fn reveal_then_conceal_roundtrips_membership() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        sim.substrate
            .entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        sim.reveal(1);
        assert!(sim.substrate.entities.get(1).unwrap().in_logic_vector);
        assert_eq!(sim.live_object_order_snapshot(), vec![1]);
        sim.conceal(1);
        assert!(!sim.substrate.entities.get(1).unwrap().in_logic_vector);
        assert!(sim.live_object_order_snapshot().is_empty());
        assert!(sim.substrate.entities.get(1).is_some()); // conceal keeps the store slot
    }

    /// Slice 3: `unlimbo(ge)` places the entity into BOTH the active order and
    /// occupancy in one atomic call — a caller can never observe it in `logic`
    /// without occupancy, because the method returns only after both. Owner count
    /// is incremented. (No-op collapse: same end state as the old 4-step.)
    #[test]
    fn unlimbo_ge_places_into_logic_and_occupancy_atomically() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        let mut ge = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        // `place_spawned` resolves the owner against `sim.interner`; re-intern so
        // the id is valid there (test_default uses the thread-local test interner).
        ge.owner = sim.interner.intern("Americans");
        let (id, outcome) = sim.unlimbo(ge);
        assert!(matches!(outcome, RevealOutcome::Revealed { .. }));

        let e = sim.substrate.entities.get(id).expect("entity in store");
        assert!(e.in_logic_vector, "must be in the active order");
        assert!(!e.lifecycle.in_limbo);
        assert!(e.lifecycle.cell_marked);
        assert_eq!(sim.live_object_order_snapshot(), vec![id]);
        assert!(
            sim.substrate.occupancy.contains_entity(5, 5, id),
            "must be registered in its foundation cell",
        );
        #[cfg(debug_assertions)]
        sim.debug_assert_lifecycle_consistent();
    }

    /// Slice 3: `create_limbo(ge)` stores the entity and increments owner counts
    /// but leaves it OUT of the active order and OUT of occupancy (born InLimbo).
    #[test]
    fn create_limbo_leaves_entity_out_of_logic_and_occupancy() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        let mut ge = GameEntity::test_default(2, "E1", "Americans", 6, 6);
        // `place_spawned` resolves the owner against `sim.interner`; re-intern so
        // the id is valid there (test_default uses the thread-local test interner).
        ge.owner = sim.interner.intern("Americans");
        let id = sim.create_limbo(ge);

        let e = sim.substrate.entities.get(id).expect("entity in store");
        assert!(!e.in_logic_vector, "limbo object is not an active member");
        assert!(e.lifecycle.in_limbo);
        assert!(!e.lifecycle.cell_marked);
        assert!(sim.live_object_order_snapshot().is_empty());
        assert!(
            !sim.substrate.occupancy.contains_entity(6, 6, id),
            "limbo object must not occupy a cell",
        );
    }

    /// `uninit` conceals then frees the store slot.
    #[test]
    fn uninit_conceals_then_frees_store_slot() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let mut ge = GameEntity::test_default(2, "MTNK", "Americans", 4, 4);
        ge.owner = owner;
        sim.substrate.entities.insert(ge);
        sim.reveal(2);
        sim.uninit(2);
        // Two-phase: resolvable-but-Dying until the drain, off the logic order now.
        assert!(sim.substrate.entities.get(2).is_some_and(|e| e.dying));
        assert!(sim.live_object_order_snapshot().is_empty());
        sim.flush_pending_delete();
        assert!(sim.substrate.entities.get(2).is_none());
    }

    /// `despawn_entity` is retained and delegates to `uninit`.
    #[test]
    fn despawn_entity_delegates_to_uninit() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let mut ge = GameEntity::test_default(3, "MTNK", "Americans", 6, 6);
        ge.owner = owner;
        sim.substrate.entities.insert(ge);
        sim.reveal(3);
        sim.despawn_entity(3);
        // Two-phase: resolvable-but-Dying until the drain, off the logic order now.
        assert!(sim.substrate.entities.get(3).is_some_and(|e| e.dying));
        assert!(sim.live_object_order_snapshot().is_empty());
        sim.flush_pending_delete();
        assert!(sim.substrate.entities.get(3).is_none());
    }

    /// The membership invariant holds across a mix of reveal/conceal/uninit.
    #[test]
    #[cfg(debug_assertions)]
    fn lifecycle_keeps_membership_invariant() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        for id in [1u64, 2, 3] {
            let mut ge = GameEntity::test_default(id, "MTNK", "Americans", 5, 5);
            ge.owner = owner;
            sim.substrate.entities.insert(ge);
            sim.reveal(id);
        }
        sim.conceal(2);
        sim.uninit(1);
        sim.debug_assert_logic_membership_consistent();
        assert_eq!(sim.live_object_order_snapshot(), vec![3]);
    }

    // --- LogicClass live count-reload pass (scheduler contract) ---

    /// Insert an entity into the store and append it to the active order.
    fn spawn_and_register(sim: &mut Simulation, id: u64) {
        use crate::sim::game_entity::GameEntity;
        sim.substrate
            .entities
            .insert(GameEntity::test_default(id, "MTNK", "Americans", 5, 5));
        sim.register_live_object(id);
    }

    /// An object the body tail-appends during the pass is ticked later in the
    /// SAME pass, because the live length is re-read after each body call.
    #[test]
    fn logic_scheduler_append_during_pass_ticks_new_tail_same_tick() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        spawn_and_register(&mut sim, 1); // A
        spawn_and_register(&mut sim, 2); // B
        // C exists in the store but is NOT yet in the active order.
        sim.substrate
            .entities
            .insert(GameEntity::test_default(3, "MTNK", "Americans", 6, 6));
        assert!(!sim.live_object_order_snapshot().contains(&3));

        let mut visited = Vec::new();
        sim.for_each_live_object(|sim, id| {
            visited.push(id);
            if id == 1 {
                // A's body reveals C at the tail.
                sim.register_live_object(3);
            }
        });

        // C ran in the same pass, after the old tail.
        assert_eq!(visited, vec![1, 2, 3]);
        assert_eq!(sim.live_object_order_snapshot(), vec![1, 2, 3]);
    }

    /// Registering the same object twice is a no-op: the order keeps one entry
    /// and the body runs for it exactly once.
    #[test]
    fn logic_scheduler_duplicate_registration_is_idempotent() {
        let mut sim = Simulation::new();
        spawn_and_register(&mut sim, 1);
        sim.register_live_object(1); // duplicate
        assert_eq!(sim.live_object_order_snapshot(), vec![1]);

        let mut visits = 0;
        sim.for_each_live_object(|_, id| {
            if id == 1 {
                visits += 1;
            }
        });
        assert_eq!(visits, 1);
    }

    /// When the current object unregisters itself, compaction shifts its
    /// successor into the just-processed slot; the cursor still advances, so
    /// that successor is skipped this pass (no index repair).
    #[test]
    fn logic_scheduler_self_unregister_uses_compacting_index_semantics() {
        let mut sim = Simulation::new();
        spawn_and_register(&mut sim, 1); // A
        spawn_and_register(&mut sim, 2); // B
        spawn_and_register(&mut sim, 3); // C

        let mut visited = Vec::new();
        sim.for_each_live_object(|sim, id| {
            visited.push(id);
            if id == 2 {
                sim.unregister_live_object(2); // B removes itself
            }
        });

        // A and B were visited; C (shifted into B's slot) is skipped this pass.
        assert_eq!(visited, vec![1, 2]);
        // Order is compacted, order-preserving — B gone, C retained.
        assert_eq!(sim.live_object_order_snapshot(), vec![1, 3]);
    }

    /// Premise: a snapshot walk MISSES a same-pass append that the live pass
    /// catches. This is the drift the live pass exists to remove.
    #[test]
    fn logic_scheduler_snapshot_walk_misses_same_pass_append() {
        use crate::sim::game_entity::GameEntity;

        // Snapshot path: appended object is invisible to this pass.
        let mut sim = Simulation::new();
        spawn_and_register(&mut sim, 1);
        spawn_and_register(&mut sim, 2);
        sim.substrate
            .entities
            .insert(GameEntity::test_default(3, "MTNK", "Americans", 6, 6));
        let order = sim.live_object_order_snapshot();
        let mut snapshot_visited = Vec::new();
        for &id in &order {
            snapshot_visited.push(id);
            if id == 1 {
                sim.register_live_object(3);
            }
        }
        assert_eq!(snapshot_visited, vec![1, 2]); // C missed

        // Live path on an equivalent setup: appended object is visited.
        let mut sim2 = Simulation::new();
        spawn_and_register(&mut sim2, 1);
        spawn_and_register(&mut sim2, 2);
        sim2.substrate
            .entities
            .insert(GameEntity::test_default(3, "MTNK", "Americans", 6, 6));
        let mut live_visited = Vec::new();
        sim2.for_each_live_object(|sim, id| {
            live_visited.push(id);
            if id == 1 {
                sim.register_live_object(3);
            }
        });
        assert_eq!(live_visited, vec![1, 2, 3]); // C caught

        assert_ne!(snapshot_visited, live_visited);
    }

    /// `Command::ForceAttackCell` is serializable (replay/snapshot back-compat).
    #[test]
    fn force_attack_cell_command_serializes() {
        use crate::sim::command::Command;
        let cmd = Command::ForceAttackCell {
            attacker_id: 7,
            target_rx: 100,
            target_ry: 200,
        };
        let bytes = bincode::serialize(&cmd).expect("serialize should succeed");
        let restored: Command = bincode::deserialize(&bytes).expect("deserialize should succeed");
        assert!(matches!(
            restored,
            Command::ForceAttackCell {
                attacker_id: 7,
                target_rx: 100,
                target_ry: 200
            }
        ));
    }

    /// Substrate Slice 5 (#8) re-entry case: when an entity LEAVES a cell and
    /// re-enters it, it takes a fresh (newest) enter order while keeping its
    /// (lowest) stable id — the one ordering the base
    /// `saveload_occupancy_list_order_matches_incremental` fixture cannot
    /// produce. The post-load rebuild must reproduce the re-entered list
    /// exactly and deterministically.
    #[test]
    fn saveload_occupancy_list_order_survives_reentry() {
        use crate::map::entities::EntityCategory;
        use crate::sim::game_entity::GameEntity;
        use crate::sim::occupancy::OccupancyGrid;

        let mut sim = Simulation::new();
        for id in 1u64..=3 {
            let mut e = GameEntity::test_default(id, "E1", "Americans", 5, 5);
            e.category = EntityCategory::Infantry;
            sim.substrate.entities.insert(e);
            sim.add_entity_occupancy(id);
        }
        // Re-entry: pop entity 1 out and back in. Its enter order is now the
        // NEWEST while its stable id stays the LOWEST — an id-sorted rebuild
        // would produce a different list, so this discriminates the
        // (enter_order, id) contract from a naive id sort.
        sim.remove_entity_occupancy(1);
        sim.add_entity_occupancy(1);

        let live: Vec<(u64, MovementLayer)> = sim
            .substrate
            .occupancy
            .get(5, 5)
            .expect("occupied cell")
            .iter_layer(MovementLayer::Ground)
            .map(|o| (o.entity_id, o.layer))
            .collect();
        // Non-buildings PREPEND, so after the re-entry the live list is
        // [1 (re-entered, newest), 3, 2].
        assert_eq!(
            live.iter().map(|(id, _)| *id).collect::<Vec<u64>>(),
            vec![1, 3, 2],
            "incremental list order (prepend + re-entry) is the fixture premise"
        );

        // Serde round trip (the snapshot path), then the post-load rebuild
        // (`rebuild_caches_after_load` delegates occupancy to exactly this).
        let bytes = bincode::serialize(&sim).expect("sim serializes");
        let restored: Simulation = bincode::deserialize(&bytes).expect("sim deserializes");
        let rebuilt = OccupancyGrid::rebuild(&restored.substrate.entities);
        let rebuilt_list: Vec<(u64, MovementLayer)> = rebuilt
            .get(5, 5)
            .expect("rebuilt cell")
            .iter_layer(MovementLayer::Ground)
            .map(|o| (o.entity_id, o.layer))
            .collect();
        assert_eq!(
            rebuilt_list, live,
            "post-load rebuild must reproduce the incremental occupant list exactly"
        );

        // Determinism: a second rebuild from the same store is identical.
        let rebuilt_again = OccupancyGrid::rebuild(&restored.substrate.entities);
        let rebuilt_again_list: Vec<(u64, MovementLayer)> = rebuilt_again
            .get(5, 5)
            .expect("rebuilt cell")
            .iter_layer(MovementLayer::Ground)
            .map(|o| (o.entity_id, o.layer))
            .collect();
        assert_eq!(rebuilt_again_list, rebuilt_list, "rebuild is deterministic");
    }
}
