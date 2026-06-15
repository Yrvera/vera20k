//! Simulation snapshot serialization for mid-match save/load.
//!
//! Serializes the full `Simulation` state into a compact binary blob via
//! bincode. Caches and event queues are `#[serde(skip)]`'d on `Simulation`
//! and must be rebuilt by the caller via `rebuild_caches_after_load()`.
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on sim/world (Simulation).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

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
const SNAPSHOT_VERSION: u32 = 25;

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
    #[error("map hash mismatch — save was made on a different map")]
    MapMismatch,
    #[error("rules hash mismatch — save was made with different rules")]
    RulesMismatch,
    #[error("deserialization failed: {0}")]
    DeserializeFailed(#[from] bincode::Error),
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
    /// Serialize the current simulation state into a binary save blob.
    ///
    /// The caller provides hashes of the current map and rules, the current
    /// tick, the map name, and the wall-clock save timestamp (seconds since
    /// UNIX epoch) for header metadata. The timestamp is taken at the app
    /// layer — sim/ must not read the system clock so headless/replay builds
    /// stay clock-independent.
    pub fn save(
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

    /// Deserialize a snapshot from bytes.
    ///
    /// Checks the version field but NOT map/rules hashes — the caller decides
    /// policy on hash mismatches (warn vs reject).
    pub fn load(bytes: &[u8]) -> Result<GameSnapshot, SnapshotError> {
        let snapshot: GameSnapshot = bincode::deserialize(bytes)?;
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(SnapshotError::VersionMismatch {
                expected: SNAPSHOT_VERSION,
                found: snapshot.version,
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
    use crate::sim::world::Simulation;
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
            overlay_blocks: false,
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
        sim.substrate.occupancy
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

        let result = GameSnapshot::load(&bytes);
        assert!(result.is_err(), "mismatched version should fail");
    }

    /// Concurrent-slice ladder: radiation took 20 -> 21, ScenarioSession (SC-2)
    /// took 21 -> 22, S3 (per-object pre-death facing read + idle-Guard authority)
    /// took 22 -> 23, the S4a authoritative flip (per-object mission commit
    /// relocated to the AI host) took 23 -> 24, and S4b (the hashed
    /// `damage_particle_live_until` `+0x308`-equivalent field) took 24 -> 25. This
    /// pins it so a later accidental bump is caught.
    #[test]
    fn snapshot_version_is_25() {
        assert_eq!(super::SNAPSHOT_VERSION, 25);
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
            .substrate.entities
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
        sim.substrate.entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        assert!(sim.substrate.entities.contains(1));
        assert!(!sim.live_object_order_snapshot().contains(&1));
        // Reveal both: tail-append in reveal order, not sorted.
        sim.substrate.entities
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
            sim.substrate.entities
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
        sim.substrate.entities
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

    /// Slice 2 acceptance: save/load restores identical `presence` for an active
    /// unit (InCell), a never-revealed limbo object (Limbo), and a boarded/cargo
    /// unit (Limbo) — and the state hash is unchanged by `presence` (it is
    /// serde-skip and not hashed).
    #[test]
    fn saveload_restores_presence_for_active_limbo_and_cargo() {
        use crate::sim::game_entity::{GameEntity, Presence};
        use crate::sim::passenger::PassengerRole;

        let mut sim = Simulation::new();
        // (1) Active unit on the playfield → InCell.
        sim.substrate
            .entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        sim.reveal(1);
        // (2) Never-revealed limbo object → Limbo (default, never joined active set).
        sim.substrate
            .entities
            .insert(GameEntity::test_default(2, "E1", "Americans", 0, 0));
        // (3) Transport-loaded infantry: revealed, then concealed while boarding → Limbo.
        let mut pax = GameEntity::test_default(3, "E1", "Americans", 6, 6);
        pax.passenger_role = PassengerRole::Inside { transport_id: 1 };
        sim.substrate.entities.insert(pax);
        sim.reveal(3);
        sim.conceal(3); // boards: leaves the active order → Limbo

        // Pre-save expectations.
        assert_eq!(sim.substrate.entities.get(1).unwrap().presence, Presence::InCell);
        assert_eq!(sim.substrate.entities.get(2).unwrap().presence, Presence::Limbo);
        assert_eq!(sim.substrate.entities.get(3).unwrap().presence, Presence::Limbo);
        let hash_before = sim.state_hash();

        // Round-trip + the real load-path membership rebuild.
        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let mut restored = GameSnapshot::load(&bytes).expect("load should succeed").sim;
        restored.rebuild_logic_membership();

        // Presence restored identically.
        assert_eq!(restored.substrate.entities.get(1).unwrap().presence, Presence::InCell);
        assert_eq!(restored.substrate.entities.get(2).unwrap().presence, Presence::Limbo);
        assert_eq!(restored.substrate.entities.get(3).unwrap().presence, Presence::Limbo);

        // Hash is unaffected by presence (serde-skip + not hashed).
        assert_eq!(restored.state_hash(), hash_before);

        // The reconciled shadow agrees with the derivation everywhere.
        #[cfg(debug_assertions)]
        restored.debug_assert_presence_consistent();
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
        sim.substrate.entities
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
        use crate::sim::game_entity::{GameEntity, Presence};
        let mut sim = Simulation::new();
        let mut ge = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        // `place_spawned` resolves the owner against `sim.interner`; re-intern so
        // the id is valid there (test_default uses the thread-local test interner).
        ge.owner = sim.interner.intern("Americans");
        let id = sim.unlimbo(ge);

        let e = sim.substrate.entities.get(id).expect("entity in store");
        assert!(e.in_logic_vector, "must be in the active order");
        assert_eq!(e.presence, Presence::InCell);
        assert_eq!(sim.live_object_order_snapshot(), vec![id]);
        assert!(
            sim.substrate.occupancy.contains_entity(5, 5, id),
            "must be registered in its foundation cell",
        );
        #[cfg(debug_assertions)]
        sim.debug_assert_presence_consistent();
    }

    /// Slice 3: `create_limbo(ge)` stores the entity and increments owner counts
    /// but leaves it OUT of the active order and OUT of occupancy (born InLimbo).
    #[test]
    fn create_limbo_leaves_entity_out_of_logic_and_occupancy() {
        use crate::sim::game_entity::{GameEntity, Presence};
        let mut sim = Simulation::new();
        let mut ge = GameEntity::test_default(2, "E1", "Americans", 6, 6);
        // `place_spawned` resolves the owner against `sim.interner`; re-intern so
        // the id is valid there (test_default uses the thread-local test interner).
        ge.owner = sim.interner.intern("Americans");
        let id = sim.create_limbo(ge);

        let e = sim.substrate.entities.get(id).expect("entity in store");
        assert!(!e.in_logic_vector, "limbo object is not an active member");
        assert_eq!(e.presence, Presence::Limbo);
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
        sim.substrate.entities
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
        sim.substrate.entities
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
        sim.substrate.entities
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
        sim2.substrate.entities
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
