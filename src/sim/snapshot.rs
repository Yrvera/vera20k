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
// Bumped 11 -> 12 for the two-stream RNG split: `rng` (one field) became
// `scenario_rng` + `main_rng` + `seed`, changing the positional bincode layout.
// Old single-`rng` blobs must be rejected, not mis-deserialized.
const SNAPSHOT_VERSION: u32 = 12;

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
            tick: sim.tick,
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
    use crate::sim::world::Simulation;
    use std::collections::BTreeMap;

    /// Helper: advance a sim by one tick with empty inputs.
    fn tick(sim: &mut Simulation) {
        let height_map = BTreeMap::new();
        sim.advance_tick(&[], None, &height_map, None, None, 67);
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

    /// `AttackTarget::for_cell` survives serialize → deserialize as the same
    /// `TargetKind::Cell` variant (regression for SNAPSHOT_VERSION 4 → 5).
    #[test]
    fn cell_attack_target_round_trips_through_snapshot() {
        use crate::sim::combat::{AttackTarget, TargetKind};
        use crate::sim::game_entity::GameEntity;

        let mut sim = Simulation::new();
        let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        entity.attack_target = Some(AttackTarget::for_cell(50, 50));
        sim.entities.insert(entity);

        let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0);
        let loaded = GameSnapshot::load(&bytes).expect("load should succeed");
        let restored = loaded
            .sim
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
        sim.entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        assert!(sim.entities.contains(1));
        assert!(!sim.live_object_order_snapshot().contains(&1));
        // Reveal both: tail-append in reveal order, not sorted.
        sim.entities
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
            sim.entities
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
        sim.entities
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

    // --- Slice 1: reveal/conceal/unlimbo/uninit lifecycle chokepoint ---

    /// `reveal` adds a member; `conceal` removes it from the order but keeps the
    /// store slot (limbo).
    #[test]
    fn reveal_then_conceal_roundtrips_membership() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        sim.entities
            .insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        sim.reveal(1);
        assert!(sim.entities.get(1).unwrap().in_logic_vector);
        assert_eq!(sim.live_object_order_snapshot(), vec![1]);
        sim.conceal(1);
        assert!(!sim.entities.get(1).unwrap().in_logic_vector);
        assert!(sim.live_object_order_snapshot().is_empty());
        assert!(sim.entities.get(1).is_some()); // conceal keeps the store slot
    }

    /// `unlimbo` is `reveal`: a stored limbo object joins the active order.
    #[test]
    fn unlimbo_equals_reveal_appends_member() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        sim.entities
            .insert(GameEntity::test_default(7, "E1", "Americans", 3, 3));
        sim.unlimbo(7);
        assert!(sim.entities.get(7).unwrap().in_logic_vector);
        assert_eq!(sim.live_object_order_snapshot(), vec![7]);
    }

    /// `uninit` conceals then frees the store slot.
    #[test]
    fn uninit_conceals_then_frees_store_slot() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let mut ge = GameEntity::test_default(2, "MTNK", "Americans", 4, 4);
        ge.owner = owner;
        sim.entities.insert(ge);
        sim.reveal(2);
        sim.uninit(2);
        assert!(sim.entities.get(2).is_none());
        assert!(sim.live_object_order_snapshot().is_empty());
    }

    /// `despawn_entity` is retained and delegates to `uninit`.
    #[test]
    fn despawn_entity_delegates_to_uninit() {
        use crate::sim::game_entity::GameEntity;
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let mut ge = GameEntity::test_default(3, "MTNK", "Americans", 6, 6);
        ge.owner = owner;
        sim.entities.insert(ge);
        sim.reveal(3);
        sim.despawn_entity(3);
        assert!(sim.entities.get(3).is_none());
        assert!(sim.live_object_order_snapshot().is_empty());
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
            sim.entities.insert(ge);
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
        sim.entities
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
        sim.entities
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
        sim.entities
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
        sim2.entities
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
}
