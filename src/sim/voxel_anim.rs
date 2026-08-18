//! `VoxelAnimClass` — the flying VXL debris a vehicle or building throws when
//! it dies, and the meteors a scenario drops.
//!
//! gamemd-derived: constructor `0x007493B0`, AI `0x00749F30` (vtable `+0x5C`),
//! destructor `0x007499F0`. Each instance carries one `BounceClass` physics body
//! embedded at `+0xB0`; that half lives in [`crate::sim::bounce`].
//!
//! This is NOT `AnimClass` — that draws SHP sprites and is a separate hierarchy
//! sharing only `ObjectClass`. It is also not `sim::components::VoxelAnimation`,
//! which is a per-entity HVA frame cursor.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, map/, util/ and the rest of sim/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::rules::voxel_anim_type::VoxelAnimTypeId;
use crate::sim::bounce::BounceState;
use crate::sim::intern::InternedId;

/// One live `VoxelAnimClass` instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelAnimObject {
    pub stable_id: u64,
    pub type_id: VoxelAnimTypeId,
    /// `+0x10C`. Ticks remaining. The AI decrements it while positive and runs
    /// the expiry arm at zero.
    pub duration: i32,
    /// `+0x110`. Queued for removal; the AI deletes on its next visit.
    pub marked_for_deletion: bool,
    /// Owner house, or `None` — the constructor's fourth parameter, which the
    /// debris path passes as the dying object's house.
    pub owner_house: Option<InternedId>,
    /// The embedded `BounceClass` at `+0xB0`.
    pub bounce: BounceState,
    /// LogicClass membership, reconstructed from the serialized vector.
    #[serde(skip)]
    pub in_logic_vector: bool,
}

impl VoxelAnimObject {
    /// The world coordinate the draw and the damage arms read, in leptons.
    ///
    /// `VoxelAnimClass::AI` refreshes `ObjectClass`'s own coordinate from the
    /// physics body every tick via `CoordStruct::FromDoubles`, so the body is
    /// the authority and this is the conversion.
    pub fn world_coord(&self) -> IVec3 {
        self.bounce.position_leptons()
    }
}

/// Deterministic store, keyed by the shared object id.
///
/// `BTreeMap` for the same reason `EntityStore` uses one: the tick walk and the
/// state hash must see a fixed order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VoxelAnimStore(BTreeMap<u64, VoxelAnimObject>);

impl VoxelAnimStore {
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &VoxelAnimObject)> + '_ {
        self.0.iter()
    }

    pub fn get(&self, id: u64) -> Option<&VoxelAnimObject> {
        self.0.get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: u64) -> Option<&mut VoxelAnimObject> {
        self.0.get_mut(&id)
    }

    pub fn contains_key(&self, id: u64) -> bool {
        self.0.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Insert an object whose identity the shared allocator already assigned.
    pub(crate) fn insert(&mut self, object: VoxelAnimObject) -> u64 {
        let id = object.stable_id;
        debug_assert_ne!(id, 0, "voxel anim requires an assigned stable id");
        self.0.insert(id, object);
        id
    }

    pub(crate) fn remove(&mut self, id: u64) -> Option<VoxelAnimObject> {
        self.0.remove(&id)
    }

    /// Ids in store order, for a walk that mutates the store as it goes.
    pub(crate) fn ids(&self) -> Vec<u64> {
        self.0.keys().copied().collect()
    }
}

/// How many debris objects of each `DebrisTypes=` entry a death throws.
///
/// gamemd-derived: the debris block of `TechnoClass::ReceiveDamage @
/// 0x00701900`, read from disassembly at `0x0070226F`..`0x007023B3`. The
/// arithmetic is small and the RNG order is the whole parity contract, so it is
/// pinned here as a pure function rather than buried in the spawn walk.
///
/// The gates, in native order:
/// 1. `MapClass::Get_CellClass_At_Coord` on the death cell, then
///    `CMP [cell+0xEC], 2 / JZ 0x00702672` at `0x00702274` — a unit dying on
///    WATER throws no debris at all, and takes no draw. That is the caller's
///    gate, not this function's.
/// 2. `TechnoType+0x5BC` (`MaxDebris`) must be positive (`0x00702291`).
/// 3. `RandomRanged(MinDebris, MaxDebris - 1)` at `0x007022C8`. Note the `DEC
///    EDI` at `0x007022AD`: the range's top is one BELOW `MaxDebris`, not equal
///    to it, so `MaxDebris=1` can only ever yield the `MinDebris` end.
/// 4. The `DebrisTypes` vector count (`TechnoType+0x324`) must be positive, and
///    the budget must be positive (`0x007022EA`, `0x007022F8`).
/// 5. For each entry, in list order: one `Random__Next()`, then
///    `count = min(|next| % (DebrisMaximums[i] + 1), budget)`.
///
/// The budget is a PER-ENTRY CAP, not a pool that drains. `CMP EDX,EBX` at
/// `0x0070233B` compares against the same `EBX` every iteration, and the
/// `[ESP+0x14]` save/restore around the loop exists only because
/// `MOV EBX,EAX` at `0x0070235B` clobbers the register with the freshly
/// allocated object — nothing ever subtracts from it. A type listing two
/// debris entries can therefore throw up to twice the budget.
///
/// `debris_maximums` is positionally paired with `debris_types`.
pub fn debris_spawn_counts(
    debris_type_count: usize,
    debris_maximums: &[i32],
    min_debris: i32,
    max_debris: i32,
    rng: &mut crate::sim::rng::SimRng,
) -> Vec<i32> {
    if max_debris <= 0 || debris_type_count == 0 {
        return Vec::new();
    }
    // `RandomRanged(MinDebris, MaxDebris - 1)`. The helper sorts reversed
    // bounds and consumes no draw when they are equal, matching native's own
    // `RandomRanged`.
    let low = min_debris.max(0) as u32;
    let high = (max_debris - 1).max(0) as u32;
    let budget = rng.next_range_u32_inclusive(low, high) as i32;
    if budget <= 0 {
        return Vec::new();
    }

    (0..debris_type_count)
        .map(|index| {
            // VERA-internal, gamemd equivalent UNCHECKED: a `DebrisMaximums`
            // entry this list does not have is treated as 0, where native reads
            // past the vector's end. Stock authors the two lists at equal
            // length — 36 sections, each `DebrisTypes=TIRE` with one maximum —
            // so this is unreachable in stock.
            let maximum = debris_maximums.get(index).copied().unwrap_or(0);
            let divisor = maximum.saturating_add(1).max(1) as u32;
            let drawn = rng.next_raw_abs_modulo(divisor) as i32;
            drawn.min(budget)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::rng::SimRng;

    #[test]
    fn gsi_05_14_no_debris_without_a_positive_maxdebris_and_no_draw_taken() {
        // `TEST ECX,ECX / JLE` at `0x00702291` sits BEFORE the budget draw, so
        // a type with no `MaxDebris=` costs the shared stream nothing.
        let mut rng = SimRng::new(11);
        let counts = debris_spawn_counts(1, &[3], 0, 0, &mut rng);
        assert!(counts.is_empty());
        assert_eq!(rng.logical_view(), SimRng::new(11).logical_view());

        // Same for a type that authors debris counts but lists no types.
        let mut rng = SimRng::new(11);
        let counts = debris_spawn_counts(0, &[], 2, 9, &mut rng);
        assert!(counts.is_empty());
        assert_eq!(rng.logical_view(), SimRng::new(11).logical_view());
    }

    #[test]
    fn gsi_05_14_budget_range_stops_one_below_maxdebris() {
        // `DEC EDI` at `0x007022AD` — the range is [MinDebris, MaxDebris - 1].
        // With MinDebris == MaxDebris - 1 the two bounds coincide, and the
        // helper (like native's RandomRanged) then consumes no draw at all, so
        // the budget is exactly that value and every later draw is the entry's.
        let mut rng = SimRng::new(5);
        let counts = debris_spawn_counts(1, &[0], 4, 5, &mut rng);
        // DebrisMaximums[0] = 0 gives a divisor of 1, so the entry draw is
        // taken and always yields 0.
        assert_eq!(counts, vec![0]);
        let mut expected = SimRng::new(5);
        expected.next_raw_abs_modulo(1);
        assert_eq!(rng.logical_view(), expected.logical_view());
    }

    #[test]
    fn gsi_05_14_budget_caps_each_entry_independently_rather_than_draining() {
        // The load-bearing correction: `CMP EDX,EBX` at `0x0070233B` compares
        // against the same budget every iteration. Two entries can therefore
        // each reach the cap, for twice the budget in total — a pool that
        // drained would give at most the budget across both.
        //
        // MinDebris == MaxDebris - 1 == 1 pins the budget at 1 with no draw;
        // each entry then draws `|next| % 6` and clamps to 1.
        let mut rng = SimRng::new(2024);
        let counts = debris_spawn_counts(2, &[5, 5], 1, 2, &mut rng);
        assert_eq!(counts.len(), 2);
        assert!(counts.iter().all(|&n| (0..=1).contains(&n)));

        // Exactly two draws, one per entry — the second is taken even if the
        // first already consumed the whole budget.
        let mut expected = SimRng::new(2024);
        expected.next_raw_abs_modulo(6);
        expected.next_raw_abs_modulo(6);
        assert_eq!(rng.logical_view(), expected.logical_view());

        // And the pool reading would be observably different: search seeds for
        // a case where both entries want the cap. With the pool reading the
        // second would be forced to 0.
        let both_at_cap = (0..500u64).any(|seed| {
            let mut rng = SimRng::new(seed);
            debris_spawn_counts(2, &[5, 5], 1, 2, &mut rng) == vec![1, 1]
        });
        assert!(
            both_at_cap,
            "a per-entry cap must be able to produce [1, 1]; a draining pool never could"
        );
    }

    #[test]
    fn gsi_05_14_entry_count_is_bounded_by_its_own_debris_maximum() {
        // `|next| % (DebrisMaximums[i] + 1)` — inclusive of the maximum.
        for seed in 0..200u64 {
            let mut rng = SimRng::new(seed);
            let counts = debris_spawn_counts(1, &[3], 9, 10, &mut rng);
            assert_eq!(counts.len(), 1);
            assert!(
                (0..=3).contains(&counts[0]),
                "seed {seed} produced {} for DebrisMaximums=3",
                counts[0]
            );
        }
    }
}
