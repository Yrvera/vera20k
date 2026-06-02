//! BTreeMap-backed entity storage with deterministic sorted iteration.
//!
//! `EntityStore` replaces `hecs::World` as the container for all game entities.
//! Entities are keyed by their stable_id (u64) for O(log n) lookup. BTreeMap
//! provides deterministic sorted iteration natively — no manual cache needed.
//!
//! ## Borrow patterns
//! - Single entity mutation: `store.get_mut(id)` borrows only that entry
//! - Cross-entity reads during mutation: read target first (clone needed data),
//!   then get_mut on the other entity
//! - Batch iteration with mutation: collect `keys_sorted()`, loop with `get_mut()`
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on sim/game_entity.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::sim::game_entity::GameEntity;

/// Container for all game entities, keyed by stable_id.
///
/// Uses `BTreeMap<u64, GameEntity>` for deterministic sorted iteration
/// and O(log n) lookup. All iteration methods return entities in
/// ascending stable_id order, which is critical for lockstep multiplayer.
///
/// Maintains a secondary per-owner index (`by_owner`) so that queries like
/// "all buildings owned by house X" are O(that house's entities) instead of
/// O(total entities). The index is maintained **incrementally**: `insert`,
/// `remove`, and `change_owner` keep it in sync, so `ids_for_owner()` is always
/// current with no rebuild needed. `rebuild_owner_index()` exists only for the
/// deserialize finalizer (the primary map is bulk-loaded, bypassing `insert`).
#[derive(Debug, Clone)]
pub struct EntityStore {
    /// Primary storage: stable_id -> GameEntity.
    entities: BTreeMap<u64, GameEntity>,
    /// Per-owner index: owner InternedId -> ascending-stable_id Vec of ids.
    /// Maintained incrementally by `insert`/`remove`/`change_owner`. Emptied
    /// owners are dropped from the map so a wiped-out house's `ids_for_owner`
    /// returns `&[]`, identical to a fresh rebuild. Deterministic iteration via
    /// BTreeMap key order + sorted Vecs.
    by_owner: BTreeMap<crate::sim::intern::InternedId, Vec<u64>>,
}

impl EntityStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
            by_owner: BTreeMap::new(),
        }
    }

    /// Insert an entity. Returns its stable_id. Maintains the `by_owner` index.
    /// If an entity with the same id already existed (rare — stable_ids are
    /// monotonic), its old owner entry is removed first.
    pub fn insert(&mut self, entity: GameEntity) -> u64 {
        let id = entity.stable_id;
        let owner = entity.owner;
        if let Some(old) = self.entities.insert(id, entity) {
            self.index_remove(old.owner, id);
        }
        self.index_add(owner, id);
        id
    }

    /// Remove an entity by stable_id. Returns the removed entity if it existed.
    /// Maintains the `by_owner` index.
    pub fn remove(&mut self, stable_id: u64) -> Option<GameEntity> {
        let removed = self.entities.remove(&stable_id);
        if let Some(ref e) = removed {
            self.index_remove(e.owner, stable_id);
        }
        removed
    }

    /// Clear all RadioClass-style live contacts involving `stable_id`.
    ///
    /// Idempotent. Safe if `stable_id` is absent.
    pub fn clear_radio_contacts_for(&mut self, stable_id: u64) {
        for entity in self.entities.values_mut() {
            entity.clear_live_contact_with(stable_id);
            // Drop a dangling dock-entered link pointing at the departing entity
            // (the BREAK cascade a limbo'd dock partner would otherwise miss).
            if entity.dock_entered_with == Some(stable_id) {
                entity.dock_entered_with = None;
            }
            if entity.stable_id == stable_id {
                entity.radio_contacts.clear_all();
                entity.dock_entered_with = None;
            }
        }
    }

    /// Look up an entity by stable_id (immutable).
    pub fn get(&self, stable_id: u64) -> Option<&GameEntity> {
        self.entities.get(&stable_id)
    }

    /// Look up an entity by stable_id (mutable).
    pub fn get_mut(&mut self, stable_id: u64) -> Option<&mut GameEntity> {
        self.entities.get_mut(&stable_id)
    }

    /// Check if an entity exists.
    pub fn contains(&self, stable_id: u64) -> bool {
        self.entities.contains_key(&stable_id)
    }

    /// Number of entities in the store.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get sorted keys for deterministic iteration.
    ///
    /// Callers typically iterate with `get()` or `get_mut()`:
    /// ```ignore
    /// let keys = store.keys_sorted();
    /// for &id in &keys {
    ///     if let Some(entity) = store.get_mut(id) { ... }
    /// }
    /// ```
    pub fn keys_sorted(&self) -> Vec<u64> {
        self.entities.keys().copied().collect()
    }

    /// Iterate all entities in deterministic stable_id order (immutable).
    pub fn iter_sorted(&self) -> impl Iterator<Item = (u64, &GameEntity)> {
        self.entities.iter().map(|(&k, v)| (k, v))
    }

    /// Iterate all entity values in deterministic stable_id order (immutable).
    pub fn values_sorted(&self) -> impl Iterator<Item = &GameEntity> {
        self.entities.values()
    }

    /// Iterate all entities in stable_id order (immutable).
    /// With BTreeMap, this is always deterministic.
    pub fn values(&self) -> impl Iterator<Item = &GameEntity> {
        self.entities.values()
    }

    /// Iterate all entities mutably in stable_id order.
    /// With BTreeMap, this is always deterministic.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut GameEntity> {
        self.entities.values_mut()
    }

    /// Stable IDs owned by the given owner, in sorted order.
    /// Returns an empty slice if the owner has no entities.
    /// O(1) lookup + O(n) iteration where n = that owner's entity count.
    pub fn ids_for_owner(&self, owner: crate::sim::intern::InternedId) -> &[u64] {
        self.by_owner.get(&owner).map_or(&[], |ids| ids.as_slice())
    }

    /// Move an entity to a new owner: updates `entity.owner` AND the `by_owner`
    /// index together. Index only — does NOT touch HouseState owned-counts
    /// (callers own that, because count semantics differ by transfer kind).
    /// No-op if the entity is absent or already owned by `new_owner`.
    pub fn change_owner(&mut self, stable_id: u64, new_owner: crate::sim::intern::InternedId) {
        let old_owner = match self.entities.get_mut(&stable_id) {
            Some(e) if e.owner != new_owner => {
                let old = e.owner;
                e.owner = new_owner;
                old
            }
            _ => return,
        };
        self.index_remove(old_owner, stable_id);
        self.index_add(new_owner, stable_id);
    }

    /// Insert `id` into its owner bucket at the sorted (ascending) position.
    fn index_add(&mut self, owner: crate::sim::intern::InternedId, id: u64) {
        let v = self.by_owner.entry(owner).or_default();
        let pos = v.partition_point(|&x| x < id);
        v.insert(pos, id);
    }

    /// Remove `id` from its owner bucket; drop the bucket if it empties (so the
    /// map matches a fresh rebuild, which never stores empty owners).
    fn index_remove(&mut self, owner: crate::sim::intern::InternedId, id: u64) {
        if let Some(v) = self.by_owner.get_mut(&owner) {
            if let Ok(pos) = v.binary_search(&id) {
                v.remove(pos);
            }
            if v.is_empty() {
                self.by_owner.remove(&owner);
            }
        }
    }

    /// Rebuild the per-owner index from primary storage.
    /// Called after deserialization or any bulk mutation that bypasses insert/remove.
    pub fn rebuild_owner_index(&mut self) {
        self.by_owner.clear();
        for (&id, entity) in &self.entities {
            self.by_owner.entry(entity.owner).or_default().push(id);
        }
        // BTreeMap iteration is already sorted by key; Vecs are sorted because
        // entities BTreeMap iterates in ascending stable_id order.
    }
}

impl serde::Serialize for EntityStore {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.entities.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for EntityStore {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entities = BTreeMap::<u64, GameEntity>::deserialize(deserializer)?;
        let mut store = Self {
            entities,
            by_owner: BTreeMap::new(),
        };
        store.rebuild_owner_index();
        Ok(store)
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::game_entity::GameEntity;

    fn make_entity(id: u64) -> GameEntity {
        GameEntity::test_default(id, "HTNK", "Americans", 10, 10)
    }

    #[test]
    fn test_insert_and_get() {
        let mut store = EntityStore::new();
        store.insert(make_entity(1));
        store.insert(make_entity(2));

        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
        assert!(store.contains(1));
        assert!(store.contains(2));
        assert!(!store.contains(3));

        let e = store.get(1).expect("entity 1 should exist");
        assert_eq!(e.stable_id, 1);
    }

    #[test]
    fn test_get_mut() {
        let mut store = EntityStore::new();
        store.insert(make_entity(1));

        let e = store.get_mut(1).expect("entity 1 should exist");
        e.health.current = 50;

        let e = store.get(1).expect("entity 1 should exist");
        assert_eq!(e.health.current, 50);
    }

    #[test]
    fn test_remove() {
        let mut store = EntityStore::new();
        store.insert(make_entity(1));
        store.insert(make_entity(2));

        let removed = store.remove(1);
        assert!(removed.is_some());
        assert_eq!(removed.expect("should be Some").stable_id, 1);
        assert_eq!(store.len(), 1);
        assert!(!store.contains(1));
        assert!(store.contains(2));

        // Removing non-existent ID returns None.
        assert!(store.remove(99).is_none());
    }

    #[test]
    fn clear_radio_contacts_removes_one_sided_peer_contact() {
        let mut store = EntityStore::new();
        let mut entity = make_entity(1);
        entity.mark_live_contact_with(2);
        store.insert(entity);
        store.insert(make_entity(2));

        store.clear_radio_contacts_for(2);

        assert!(store.get(1).unwrap().radio_contacts.is_empty());
        assert!(store.get(2).unwrap().radio_contacts.is_empty());
    }

    #[test]
    fn clear_radio_contacts_removes_reciprocal_contacts() {
        let mut store = EntityStore::new();
        let mut first = make_entity(1);
        let mut second = make_entity(2);
        first.mark_live_contact_with(2);
        second.mark_live_contact_with(1);
        store.insert(first);
        store.insert(second);

        store.clear_radio_contacts_for(1);

        assert!(store.get(1).unwrap().radio_contacts.is_empty());
        assert!(store.get(2).unwrap().radio_contacts.is_empty());
    }

    #[test]
    fn clear_radio_contacts_missing_id_preserves_unrelated_contacts() {
        let mut store = EntityStore::new();
        let mut entity = make_entity(1);
        entity.radio_contacts.set_capacity(4); // hold more than one contact
        entity.mark_live_contact_with(2);
        entity.mark_live_contact_with(3);
        store.insert(entity);

        store.clear_radio_contacts_for(99);

        let contacts = &store.get(1).unwrap().radio_contacts;
        assert_eq!(contacts.len(), 2);
        assert!(contacts.contains(2) && contacts.contains(3));
    }

    #[test]
    fn clear_radio_contacts_preserves_remaining_order() {
        let mut store = EntityStore::new();
        let mut entity = make_entity(1);
        entity.radio_contacts.set_capacity(4); // hold more than one contact
        entity.mark_live_contact_with(2);
        entity.mark_live_contact_with(3);
        entity.mark_live_contact_with(4);
        store.insert(entity);
        store.insert(make_entity(3));

        store.clear_radio_contacts_for(3);

        // Removal nulls slot 1 in place (no compaction): 2 and 4 keep their slots.
        let contacts = &store.get(1).unwrap().radio_contacts;
        assert_eq!(contacts.len(), 2);
        assert!(contacts.contains(2) && contacts.contains(4) && !contacts.contains(3));
        assert_eq!(contacts.find_slot(2), Some(0));
        assert_eq!(contacts.find_slot(4), Some(2));
    }

    #[test]
    fn test_deterministic_iteration_order() {
        let mut store = EntityStore::new();
        // Insert in non-sorted order.
        store.insert(make_entity(5));
        store.insert(make_entity(1));
        store.insert(make_entity(3));
        store.insert(make_entity(2));
        store.insert(make_entity(4));

        let keys: Vec<u64> = store.keys_sorted();
        assert_eq!(keys, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_iter_sorted() {
        let mut store = EntityStore::new();
        store.insert(make_entity(3));
        store.insert(make_entity(1));
        store.insert(make_entity(2));

        let ids: Vec<u64> = store.iter_sorted().map(|(id, _)| id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_values_sorted() {
        let mut store = EntityStore::new();
        store.insert(make_entity(3));
        store.insert(make_entity(1));
        store.insert(make_entity(2));

        let ids: Vec<u64> = store.values_sorted().map(|e| e.stable_id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_sorted_after_mutation() {
        let mut store = EntityStore::new();
        store.insert(make_entity(1));
        store.insert(make_entity(3));

        let keys: Vec<u64> = store.keys_sorted();
        assert_eq!(keys, vec![1, 3]);

        // Insert maintains order.
        store.insert(make_entity(2));
        let keys: Vec<u64> = store.keys_sorted();
        assert_eq!(keys, vec![1, 2, 3]);

        // Remove maintains order.
        store.remove(1);
        let keys: Vec<u64> = store.keys_sorted();
        assert_eq!(keys, vec![2, 3]);
    }

    #[test]
    fn test_empty_store() {
        let store = EntityStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.get(1).is_none());

        let keys: Vec<u64> = store.keys_sorted();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_mutable_iteration_pattern() {
        let mut store = EntityStore::new();
        store.insert(make_entity(1));
        store.insert(make_entity(2));
        store.insert(make_entity(3));

        // The canonical pattern for mutating during iteration:
        // collect keys, then get_mut each entity.
        let keys = store.keys_sorted();
        for &id in &keys {
            if let Some(entity) = store.get_mut(id) {
                entity.health.current = entity.health.current.saturating_sub(10);
            }
        }

        // Verify all were mutated.
        for &id in &[1u64, 2, 3] {
            let e = store.get(id).expect("should exist");
            assert_eq!(e.health.current, 90);
        }
    }

    #[test]
    fn test_cross_entity_read_pattern() {
        let mut store = EntityStore::new();
        let mut e1 = make_entity(1);
        e1.position.rx = 10;
        e1.position.ry = 20;
        store.insert(e1);

        let mut e2 = make_entity(2);
        e2.position.rx = 30;
        e2.position.ry = 40;
        store.insert(e2);

        // Read target position first (immutable borrow ends).
        let target_pos = store.get(2).map(|e| e.position.clone());
        // Then mutate attacker (no conflict).
        if let (Some(attacker), Some(pos)) = (store.get_mut(1), target_pos) {
            // In real code: compute firing direction, apply cooldown, etc.
            assert_eq!(pos.rx, 30);
            attacker.facing = 128; // face toward target
        }

        assert_eq!(store.get(1).expect("should exist").facing, 128);
    }

    #[test]
    fn test_per_owner_index() {
        use crate::sim::intern::StringInterner;

        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");
        let soviets = interner.intern("Russians");

        let mut store = EntityStore::new();

        let mut e1 = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e1.owner = americans;
        let mut e2 = GameEntity::test_default(2, "MTNK", "Americans", 6, 6);
        e2.owner = americans;
        let mut e3 = GameEntity::test_default(3, "RHNO", "Russians", 10, 10);
        e3.owner = soviets;

        store.insert(e1);
        store.insert(e3);
        store.insert(e2);
        store.rebuild_owner_index();

        // Americans should have [1, 2] sorted.
        assert_eq!(store.ids_for_owner(americans), &[1, 2]);
        // Russians should have [3].
        assert_eq!(store.ids_for_owner(soviets), &[3]);

        // Remove one American entity, rebuild.
        store.remove(1);
        store.rebuild_owner_index();
        assert_eq!(store.ids_for_owner(americans), &[2]);
        assert_eq!(store.ids_for_owner(soviets), &[3]);

        // Remove all American entities, rebuild.
        store.remove(2);
        store.rebuild_owner_index();
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);

        // Unknown owner returns empty slice.
        let unknown = interner.intern("Yuri");
        assert_eq!(store.ids_for_owner(unknown), &[] as &[u64]);
    }

    #[test]
    fn insert_indexes_immediately() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");
        let mut store = EntityStore::new();
        let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e.owner = americans;
        store.insert(e);
        // No rebuild: the index is current right after insert.
        assert_eq!(store.ids_for_owner(americans), &[1]);
    }

    #[test]
    fn remove_deindexes_immediately() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");
        let mut store = EntityStore::new();
        let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e.owner = americans;
        store.insert(e);
        store.remove(1);
        // Bucket emptied → owner dropped, identical to a fresh rebuild.
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);
    }

    #[test]
    fn change_owner_moves_entry_immediately_and_is_idempotent() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");
        let soviets = interner.intern("Russians");
        let mut store = EntityStore::new();
        let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e.owner = americans;
        store.insert(e);

        store.change_owner(1, soviets);
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);
        assert_eq!(store.ids_for_owner(soviets), &[1]);
        assert_eq!(store.get(1).unwrap().owner, soviets);

        // Same-owner call is a no-op (no duplicate in the bucket).
        store.change_owner(1, soviets);
        assert_eq!(store.ids_for_owner(soviets), &[1]);

        // Missing id is a no-op.
        store.change_owner(999, americans);
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);
    }

    #[test]
    fn change_owner_preserves_sorted_order_in_both_buckets() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let a = interner.intern("Americans");
        let b = interner.intern("Russians");
        let mut store = EntityStore::new();
        for id in [10u64, 20, 30] {
            let mut e = GameEntity::test_default(id, "HTNK", "Americans", 5, 5);
            e.owner = a;
            store.insert(e);
        }
        let mut e = GameEntity::test_default(15, "RHNO", "Russians", 6, 6);
        e.owner = b;
        store.insert(e);
        // Move 20 from a→b; both buckets must stay ascending.
        store.change_owner(20, b);
        assert_eq!(store.ids_for_owner(a), &[10, 30]);
        assert_eq!(store.ids_for_owner(b), &[15, 20]);
    }

    /// Acceptance: a store built purely by incremental ops has a `by_owner`
    /// byte-identical to one produced by a full rebuild — proving
    /// deserialize-rebuild ≡ incremental.
    #[test]
    fn incremental_index_matches_rebuild() {
        use crate::sim::intern::StringInterner;
        let mut interner = StringInterner::new();
        let a = interner.intern("Americans");
        let b = interner.intern("Russians");
        let c = interner.intern("Yuri");
        let mut store = EntityStore::new();
        for (id, owner) in [(5u64, a), (1, b), (3, a), (2, c), (4, b)] {
            let mut e = GameEntity::test_default(id, "HTNK", "Americans", 5, 5);
            e.owner = owner;
            store.insert(e);
        }
        store.change_owner(3, b); // a→b
        store.change_owner(2, a); // c→a (empties c)
        store.remove(5); // drops from a
        let incremental = store.by_owner.clone();
        store.rebuild_owner_index();
        assert_eq!(incremental, store.by_owner);
    }

    #[test]
    fn test_rebuild_owner_index() {
        use crate::sim::intern::StringInterner;

        let mut interner = StringInterner::new();
        let americans = interner.intern("Americans");

        let mut store = EntityStore::new();
        let mut e1 = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        e1.owner = americans;
        let mut e2 = GameEntity::test_default(2, "MTNK", "Americans", 6, 6);
        e2.owner = americans;
        store.insert(e1);
        store.insert(e2);

        // Manually clear the index to simulate deserialization state.
        store.by_owner.clear();
        assert_eq!(store.ids_for_owner(americans), &[] as &[u64]);

        // Rebuild should restore the index.
        store.rebuild_owner_index();
        assert_eq!(store.ids_for_owner(americans), &[1, 2]);
    }
}
