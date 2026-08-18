# Garrison Eject on Destruction Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained. Commit after every task.

**Goal:** When a `CanBeOccupied` building is destroyed in combat, eject its garrison occupants alive at random foundation cells (LIFO order, scatter on placement) instead of killing them with the building. Mirrors gamemd `BuildingClass::SpawnSurvivors` §4a.

**Architecture:** Mirror the established `DestroyedCrewedBuilding` deferred-event pattern. The combat death loop in `combat/mod.rs` collects `DestroyedGarrisonBuilding` events; `world/mod.rs` dispatches them after combat to a new `production::eject_destruction_garrison` helper, which sits next to the existing sell-path eject in `production_sell.rs`.

**Design Doc:** [docs/plans/2026-05-04-garrison-eject-on-destruction-design.md](2026-05-04-garrison-eject-on-destruction-design.md)

---

## Grounding Summary

**Docs (R1):** `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` §4a documents `SpawnSurvivors @ 0x00442D90` — the garrison-occupant eject path on destruction. `GARRISON_SYSTEM_GHIDRA_REPORT.md` §6 / §14c documents the parallel `SellBuilding @ 0x00457DE0` path used by the sell-eject helper we're mirroring. Both reports flag this as YR-active (verified gates `Type+0x16AE = CanBeOccupied`).

**Ghidra (R2):** Behavior cited in design doc is sourced from the verified report above; key facts: LIFO occupant iteration, `building.center + random_foundation_offset` cell selection (interior, NOT perimeter — that's the sell path), owner = building's current owner (`field_0x8C`), unlimbo failure → Destroy (no parachute on destruction; that's only sell). IC-killed branch (`field_0x6E0 != 0`) is explicitly out of scope.

**Repo pattern (R3):** Mirror `DestroyedCrewedBuilding` end-to-end:
- Struct definition: [src/sim/combat/mod.rs:282-290](../../src/sim/combat/mod.rs#L282-L290)
- `CombatTickResult` field: [src/sim/combat/mod.rs:313-314](../../src/sim/combat/mod.rs#L313-L314)
- `DeathEffects` field: [src/sim/combat/mod.rs:345](../../src/sim/combat/mod.rs#L345)
- Death-loop population: [src/sim/combat/mod.rs:403-412](../../src/sim/combat/mod.rs#L403-L412)
- Result construction: [src/sim/combat/mod.rs:610](../../src/sim/combat/mod.rs#L610) and [src/sim/combat/mod.rs:1334](../../src/sim/combat/mod.rs#L1334)
- World dispatch: [src/sim/world/mod.rs:1194-1205](../../src/sim/world/mod.rs#L1194-L1205)
- Helper module: [src/sim/production/production_sell.rs:184-227](../../src/sim/production/production_sell.rs#L184-L227)
- Re-export: [src/sim/production/mod.rs:33-35](../../src/sim/production/mod.rs#L33-L35)

The helper itself mirrors `eject_garrison_occupants` at [src/sim/production/production_sell.rs:246-373](../../src/sim/production/production_sell.rs#L246-L373) — same LIFO iteration, same `used_cells` accounting, same scatter via `issue_direct_move` + `NEIGHBORS`. Diverges on cell strategy (interior vs perimeter), fallback (kill vs parachute), ownership (inherit vs revert), per design doc.

**INI keys (R4):** No new INI parsing. The feature reads only `CanBeOccupied=yes` and the `Foundation=NxM` art entry, both already parsed:
- [src/rules/object_type.rs:412](../../src/rules/object_type.rs#L412) — `can_be_occupied`
- `Foundation=` consumed via `foundation_dimensions()` at [src/sim/production/production_tech.rs:562](../../src/sim/production/production_tech.rs#L562)

**Still unknown:** Nothing material. Design fixed all open questions.

## Key Technical Decisions

- **Deferred event mirroring `DestroyedCrewedBuilding`** — **Confidence:** high. **Source:** repo pattern at [src/sim/combat/mod.rs:282-314](../../src/sim/combat/mod.rs#L282-L314) and design doc Approach B. Avoids polluting combat module with spawn/scatter knowledge.
- **Branch on `obj.can_be_occupied && category == Structure`** — **Confidence:** high. **Source:** transports use the same `PassengerRole::Transport` cargo enum, so a category-only check would mis-fire on transports. The combination uniquely identifies garrison buildings.
- **Cell selection: shuffle foundation interior offsets via Fisher-Yates over `sim.rng`** — **Confidence:** high. **Source:** Ghidra report §4a — `random_foundation_offset` per occupant. We pre-shuffle once per building (cheaper, deterministic) instead of drawing per occupant; the visible result is identical.
- **Owner = building.owner at time of death (NOT `garrison_original_owner`)** — **Confidence:** high. **Source:** Ghidra report §4a `occupant->OwnerIndex = this->field_0x8C`. The garrisoning player retains their infantry; revert semantics don't apply when the building is destroyed.
- **Failure fallback = kill (no parachute)** — **Confidence:** high. **Source:** Ghidra report §4a explicit `infantry->Destroy()` on unlimbo failure. Parachute is sell-path only.
- **Scatter via `issue_direct_move` + `NEIGHBORS` random direction** — **Confidence:** high. **Source:** mirrors sell-path scatter at [production_sell.rs:331-356](../../src/sim/production/production_sell.rs#L331-L356). We don't have AI Mission_Hunt yet; we use scatter for everyone (deviation noted in design doc Out of Scope).

## Open Questions

### Resolved During Planning

- **Make helper `pub` or `pub(crate)`?** `pub` and re-exported through `production::eject_destruction_garrison`, mirroring `eject_destruction_survivors`. The crewed-survivor helper sets the precedent at [src/sim/production/mod.rs:34](../../src/sim/production/mod.rs#L34).
- **Where does `issue_direct_move` come from in the new helper?** `crate::sim::movement::issue_direct_move` (already used by sell-path at [production_sell.rs:352](../../src/sim/production/production_sell.rs#L352)).
- **Does `OccupancyGrid::add` need the `sub_cell` argument?** Yes — same call shape as the sell path at [production_sell.rs:323-329](../../src/sim/production/production_sell.rs#L323-L329).

### Deferred to Implementation

- None.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Add `DestroyedGarrisonBuilding` struct, `CombatTickResult` field, `DeathEffects` field, branch in death loop |
| Modify | [src/sim/world/mod.rs](../../src/sim/world/mod.rs) | Dispatch loop after combat, calling `production::eject_destruction_garrison` |
| Modify | [src/sim/production/production_sell.rs](../../src/sim/production/production_sell.rs) | New `eject_destruction_garrison` helper |
| Modify | [src/sim/production/mod.rs](../../src/sim/production/mod.rs) | Re-export new helper |
| Modify | [src/sim/passenger.rs](../../src/sim/passenger.rs) | Add tests in existing `mod tests` block |

No new files. No deletions.

## Interface Changes

- `combat::DestroyedGarrisonBuilding` — new public struct in `combat/mod.rs`. Peer of `DestroyedCrewedBuilding`. No external consumers besides the new dispatch in `world/mod.rs`.
- `CombatTickResult.destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding>` — new field. Existing constructors at [combat/mod.rs:603-612](../../src/sim/combat/mod.rs#L603-L612) and [combat/mod.rs:1327-1336](../../src/sim/combat/mod.rs#L1327-L1336) need update.
- `production::eject_destruction_garrison` — new public function. Mirrors `eject_destruction_survivors` shape.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (no math added; only event collection + cell-list iteration)
- [x] New state included in deterministic state hash — N/A. The event itself is ephemeral per-tick. Ejected entity state (`position`, `owner`, `passenger_role`, `movement_target`) is already hashed.
- [x] No dependencies on render/ui/sidebar/audio/net — verified. Combat module gains no new imports outside `sim/`. Helper uses `sim::movement` and `sim::passenger`, both already in-bounds.
- [x] Tick ordering impact noted — none. Eject runs in the existing post-combat block in `World::advance_tick` immediately after the crewed-building dispatch.
- [x] BTreeMap iteration order considered — N/A. Eject iterates `event.passenger_ids: Vec<u64>` which is captured in cargo-insertion order; `sim.entities.values()` for the `occupied_cells` collection is already sorted via BTreeMap.

## Risk Areas

- **Branch correctness in death loop:** the new branch must be placed before the existing kill-passengers loop AND skip the kill block on match. A naive `if`-with-`push` (without skipping the kill loop) would push the event AND kill the passengers, producing dead occupants in `passenger_ids` at eject time. Task 4 spells out the structure with an explicit `if-else` arm.
- **Transport regression:** transports must continue to kill-all-riders. The negative regression test in Task 7 guards this.
- **Capture timing:** `passenger_ids: Vec<u64>` must be cloned from cargo *before* anything mutates the building entity (despawn). Captured during `dead_info` extraction in the death loop.
- **Determinism — RNG draw order:** Fisher-Yates over `w*h` cells (one `next_u32` per swap), then per occupant one `next_u32` for scatter direction. Order documented in helper doc-comment. Two runs from the same snapshot must produce identical outputs.
- **Multiple destroyed garrisons in one tick:** dispatch order matters for RNG. The dispatch loop iterates `combat_result.destroyed_garrison_buildings` in insertion order, which mirrors the BTreeMap-sorted `dead_entities` iteration in the death loop. Deterministic.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | Garrison occupants survive building destruction | gamemd ejects them at foundation cells; current code kills them. Player garrisoning a building expects to lose the building but keep the infantry — silent loss is immediately noticeable when the player counts surviving units. | Unit test `test_garrison_eject_on_destruction_happy_path` — assert all occupants alive post-destruction. |
| Task 5 | Cell strategy: random INTERIOR foundation cells, not PERIMETER | gamemd places ejected infantry inside the foundation footprint (rubble cells), not on the edge. Sell-path uses perimeter; reusing it would visually drift the spawn point by 1-2 tiles. | Test asserts ejected positions are within `[rx, rx+w) × [ry, ry+h)`. |
| Task 5 | LIFO iteration over `passenger_ids` | gamemd iterates high→low index. Affects which occupant gets the first cell when foundation is partially blocked — visible if e.g. an Engineer (last to board) is the one that survives a 2-cell-only-free foundation. | Test asserts last-boarded occupant is placed first (gets the first shuffled cell). |
| Task 5 | Owner = building's current owner | If a Civilian Hospital is garrisoned by Player A and destroyed, the Conscripts should pop out as Player A's units, not as Civilian. | Test sets pre-destruction owner = Player A, asserts ejected infantry owner == Player A. |
| Task 5 | All-cells-blocked → kill, no parachute | Matches gamemd "Destroy on unlimbo failure". Diverges from sell-path (which would parachute, but our parachute system isn't implemented). | Test pre-occupies all foundation cells, asserts occupants `dying = true`. |
| Task 8 | In-game observable parity | Player destroys a garrisoned hospital with 5 GIs inside; sees 5 GIs spawn at rubble cells, scatter outward. Match against gamemd visually. | Manual: in skirmish, garrison `CAGAS01` with 5 conscripts, attack with a Rhino, observe 5 conscripts emerging on rubble cells. |

---

## Tasks

### Task 1: Add `DestroyedGarrisonBuilding` struct in `combat/mod.rs`

**Why:** Define the deferred-event type first. Everything downstream (collection, dispatch, helper) consumes this struct.

**Files:**
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — insert after the existing `DestroyedCrewedBuilding` struct.

**Pattern:** Mirror `DestroyedCrewedBuilding` at [combat/mod.rs:282-290](../../src/sim/combat/mod.rs#L282-L290) verbatim.

**Step 1: Insert struct definition**

Add after line 290 (after the closing `}` of `DestroyedCrewedBuilding`):

```rust
/// A `CanBeOccupied` building destroyed in combat with live occupants —
/// garrison ejection is deferred to the caller (which has access to
/// `Simulation` for repositioning, occupancy registration, and scatter).
///
/// Mirrors gamemd `BuildingClass::SpawnSurvivors` §4a — occupants are placed
/// at random cells within the building's foundation footprint, in LIFO order,
/// inheriting the building's current owner (the garrisoning player).
pub struct DestroyedGarrisonBuilding {
    pub building_id: u64,
    pub type_id: InternedId,
    /// Building's owner at time of death — ejected infantry inherit this.
    pub owner: InternedId,
    pub rx: u16,
    pub ry: u16,
    pub z: u8,
    pub foundation_w: u16,
    pub foundation_h: u16,
    /// Snapshot of `cargo.passengers` at time of death. LIFO order preserved
    /// (eject helper iterates in reverse).
    pub passenger_ids: Vec<u64>,
}
```

**Step 2: Verify**

Run: `cargo check`
Expected: PASS (no consumers yet, just the new struct).

**Step 3: Commit**

Commit message: `combat: add DestroyedGarrisonBuilding struct for deferred eject`

---

### Task 2: Add `destroyed_garrison_buildings` field to `CombatTickResult` and `DeathEffects`

**Why:** Plumb the collection through the same channels as `destroyed_crewed_buildings`. Adding the field before populating it ensures the build stays green between tasks.

**Files:**
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — three insertion sites.

**Pattern:** Mirror `destroyed_crewed_buildings` at the same locations.

**Step 1: Add to `CombatTickResult`**

In `CombatTickResult` (struct starts at line 302), add after the existing `destroyed_crewed_buildings` field at line 314:

```rust
    /// Garrisoned buildings destroyed this tick — occupants should be ejected
    /// by the caller via `production::eject_destruction_garrison`.
    pub destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding>,
```

**Step 2: Add to `DeathEffects`**

In the `DeathEffects` struct (line 341-349), add after the existing `destroyed_crewed_buildings` field at line 345:

```rust
    destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding>,
```

**Step 3: Initialize the local in `handle_entity_deaths`**

At line 369 (after the existing `destroyed_crewed_buildings` initializer):

```rust
    let mut destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding> = Vec::new();
```

**Step 4: Populate the `DeathEffects` return value**

In the `DeathEffects { ... }` return struct at line 532-540, add after `destroyed_crewed_buildings`:

```rust
        destroyed_garrison_buildings,
```

**Step 5: Update the early-return `CombatTickResult` constructor**

At [combat/mod.rs:603-612](../../src/sim/combat/mod.rs#L603-L612) (the `tick_ms == 0` early-return path), add after `destroyed_crewed_buildings: Vec::new(),`:

```rust
            destroyed_garrison_buildings: Vec::new(),
```

**Step 6: Update the main-path `CombatTickResult` constructor**

At [combat/mod.rs:1327-1336](../../src/sim/combat/mod.rs#L1327-L1336), add after `destroyed_crewed_buildings: death.destroyed_crewed_buildings,`:

```rust
        destroyed_garrison_buildings: death.destroyed_garrison_buildings,
```

**Step 7: Verify**

Run: `cargo check`
Expected: PASS — no consumers yet, just the empty plumbing.

**Step 8: Commit**

Commit message: `combat: plumb destroyed_garrison_buildings through CombatTickResult`

---

### Task 3: Branch the death loop on `CanBeOccupied`

**Why:** This is the actual behavior change in the combat module — push the event AND skip the kill-passengers loop for garrison buildings.

**Files:**
- Modify: [src/sim/combat/mod.rs:391-430](../../src/sim/combat/mod.rs#L391-L430) — restructure the existing kill loop.

**Pattern:** New branch sits inside the existing `if let Some((type_id, ...)) = dead_info` block, between the object-data lookup (lines 393-413) and the existing kill-passengers loop (lines 415-430).

**Step 1: Restructure the kill-passengers block**

Replace the existing block at lines 415-430 (the `// Kill all passengers inside a destroyed transport/garrison.` comment through the closing `}` of the for-loop) with:

```rust
            // Snapshot cargo before the building entity is despawned. Used to
            // either eject garrison occupants alive (CanBeOccupied buildings)
            // or kill all riders (transports — current behavior).
            let passenger_ids: Vec<u64> = entities
                .get(dead_id)
                .and_then(|e| e.passenger_role.cargo())
                .map(|c| c.passengers.clone())
                .unwrap_or_default();

            // Branch: garrisoned CanBeOccupied buildings eject occupants at
            // random foundation cells (handled post-combat by the world layer
            // via production::eject_destruction_garrison). Transports continue
            // to kill all riders — that's a separate parity gap to fix later.
            let is_garrison_building = rules
                .object(type_id_str)
                .map(|obj| obj.can_be_occupied)
                .unwrap_or(false)
                && category == EntityCategory::Structure
                && !passenger_ids.is_empty();

            if is_garrison_building {
                let (foundation_w, foundation_h) = rules
                    .object(type_id_str)
                    .map(|obj| {
                        crate::sim::production::foundation_dimensions(&obj.foundation)
                    })
                    .unwrap_or((1, 1));
                destroyed_garrison_buildings.push(DestroyedGarrisonBuilding {
                    building_id: dead_id,
                    type_id,
                    owner,
                    rx,
                    ry,
                    z,
                    foundation_w,
                    foundation_h,
                    passenger_ids,
                });
            } else {
                // Existing transport / non-garrison cargo behavior: kill riders.
                for &pid in &passenger_ids {
                    if let Some(pax) = entities.get_mut(pid) {
                        pax.health.current = 0;
                        pax.dying = true;
                        pax.passenger_role = PassengerRole::None;
                        pax.attack_target = None;
                        pax.movement_target = None;
                        pax.selected = false;
                    }
                }
            }
```

Note: `type_id_str` is already in scope from line 392. `category` is destructured at line 391. `EntityCategory` is already imported in this file at the top (line 5 of `production_sell.rs`'s use list, but verify combat/mod.rs separately — it's referenced elsewhere in this file e.g. line 377).

**Step 2: Verify imports**

Confirm these are in scope at the top of `combat/mod.rs`:
- `EntityCategory` — already used at line 377, confirmed in scope.
- `PassengerRole` — already used in the existing kill loop at line 425, confirmed in scope.

If `crate::sim::production::foundation_dimensions` import is missing at the top, add it. Check via:

Run: `cargo check`

If the build fails on `foundation_dimensions`, add to the imports section at the top of `combat/mod.rs`:

```rust
use crate::sim::production::foundation_dimensions;
```

…and replace the qualified path in the new code with a bare `foundation_dimensions(&obj.foundation)`.

**Step 3: Verify build**

Run: `cargo check`
Expected: PASS.

**Step 4: Run existing tests**

Run: `cargo test --lib sim::combat`
Expected: PASS — transport tests still pass since the `else` arm preserves their kill-all-riders behavior.

**Step 5: Commit**

Commit message: `combat: collect garrison buildings on destruction instead of killing occupants`

---

### Task 4: Add `eject_destruction_garrison` helper in `production_sell.rs`

**Why:** The actual eject logic — cell selection, placement, scatter, kill-on-fail. Defined before the world-layer dispatch consumes it.

**Files:**
- Modify: [src/sim/production/production_sell.rs](../../src/sim/production/production_sell.rs) — add new helper after `eject_garrison_occupants` (after line 373).

**Pattern:** Mirrors `eject_garrison_occupants` (LIFO iteration, `used_cells` accounting, scatter via `issue_direct_move`). Diverges on cell strategy (interior shuffle, not perimeter), fallback (kill, not parachute), ownership (event.owner inherited, no revert).

**Step 1: Add the import for the new struct**

At the top of `production_sell.rs`, alongside the existing combat-related imports, add:

```rust
use crate::sim::combat::DestroyedGarrisonBuilding;
```

(Insert near the other `crate::sim::*` use statements at lines 5-14.)

**Step 2: Add the helper function**

Insert after the closing `}` of `eject_garrison_occupants` at line 373:

```rust
/// Eject garrison occupants from a building destroyed in combat.
///
/// Mirrors gamemd `BuildingClass::SpawnSurvivors` §4a (0x00442D90):
/// - Iterates occupants in LIFO order (matches gamemd's high→low index loop).
/// - Places each at a random cell within the building's foundation footprint
///   (interior cells — perimeter is the sell path's strategy).
/// - Owner = building's current owner at time of death (no revert semantics —
///   the building is gone).
/// - Successful placement issues a scatter move to a random adjacent cell.
/// - Placement failure (no free foundation cell) marks the occupant dying;
///   no parachute fallback (that's sell-only in gamemd, and our parachute
///   system isn't implemented).
///
/// **Determinism — RNG draw order:**
/// 1. Fisher-Yates shuffle of foundation cell offsets (one `next_u32` per swap,
///    `(w*h - 1)` swaps total).
/// 2. Per occupant (in LIFO): one `next_u32` for scatter direction.
///
/// Returns the count of occupants successfully ejected (excludes those killed
/// by full-foundation fallback).
pub fn eject_destruction_garrison(
    sim: &mut Simulation,
    rules: &RuleSet,
    event: &DestroyedGarrisonBuilding,
) -> usize {
    if event.passenger_ids.is_empty() || event.foundation_w == 0 || event.foundation_h == 0 {
        return 0;
    }

    // Build interior-foundation cell list, then Fisher-Yates shuffle it.
    let mut cells: Vec<(u16, u16)> =
        Vec::with_capacity(event.foundation_w as usize * event.foundation_h as usize);
    for dy in 0..event.foundation_h {
        for dx in 0..event.foundation_w {
            cells.push((event.rx + dx, event.ry + dy));
        }
    }
    // Fisher-Yates: for i from len-1 down to 1, swap cells[i] with cells[rng % (i+1)].
    for i in (1..cells.len()).rev() {
        let j = (sim.rng.next_u32() as usize) % (i + 1);
        cells.swap(i, j);
    }

    // Collect currently occupied cells to avoid stacking on top of other entities.
    let occupied_cells: Vec<(u16, u16)> = sim
        .entities
        .values()
        .filter(|e| !e.passenger_role.is_inside_transport() && !e.dying && e.is_alive())
        .map(|e| (e.position.rx, e.position.ry))
        .collect();

    let mut ejected: usize = 0;
    let mut used_cells: Vec<(u16, u16)> = Vec::new();

    // LIFO: iterate passengers in reverse — matches gamemd high→low index loop.
    for &pax_id in event.passenger_ids.iter().rev() {
        // Find first shuffled cell not already occupied or used by this batch.
        let placement = cells.iter().find(|&&(cx, cy)| {
            !occupied_cells.iter().any(|&(ox, oy)| ox == cx && oy == cy)
                && !used_cells.iter().any(|&(ux, uy)| ux == cx && uy == cy)
        });

        let Some(&(spawn_rx, spawn_ry)) = placement else {
            // No free cell — kill the occupant. Matches gamemd Destroy on
            // unlimbo failure. Still advance scatter RNG to keep draw order
            // stable regardless of failure rate? No — gamemd doesn't issue
            // scatter on failed eject either. Skip the RNG draw here.
            if let Some(pax) = sim.entities.get_mut(pax_id) {
                pax.health.current = 0;
                pax.dying = true;
                pax.passenger_role = PassengerRole::None;
                pax.attack_target = None;
                pax.movement_target = None;
                pax.selected = false;
            }
            continue;
        };
        used_cells.push((spawn_rx, spawn_ry));

        // Place infantry on the map.
        let pax_sub_cell = if let Some(pax) = sim.entities.get_mut(pax_id) {
            pax.passenger_role = PassengerRole::None;
            pax.owner = event.owner;
            pax.position.rx = spawn_rx;
            pax.position.ry = spawn_ry;
            pax.position.z = event.z;
            let (sub_x, sub_y) = lepton::subcell_lepton_offset(pax.sub_cell);
            pax.position.sub_x = sub_x;
            pax.position.sub_y = sub_y;
            pax.position.refresh_screen_coords();
            pax.sub_cell
        } else {
            // Passenger missing (already despawned by another system). Skip.
            continue;
        };

        // Register in occupancy grid.
        sim.occupancy.add(
            spawn_rx,
            spawn_ry,
            pax_id,
            crate::sim::movement::locomotor::MovementLayer::Ground,
            pax_sub_cell,
        );

        // Scatter: short move to a random adjacent cell (matches sell-path
        // scatter; gamemd's Mission_Scatter has the same observable effect).
        let scatter_speed = sim
            .entities
            .get(pax_id)
            .and_then(|e| rules.object(sim.interner.resolve(e.type_ref)))
            .map(|obj| ra2_speed_to_leptons_per_second(obj.speed))
            .unwrap_or(ra2_speed_to_leptons_per_second(4));
        let start_dir = sim.rng.next_u32() as usize % 8;
        for i in 0..8 {
            let (dx, dy) = NEIGHBORS[(start_dir + i) % 8];
            let sx = spawn_rx as i32 + dx as i32;
            let sy = spawn_ry as i32 + dy as i32;
            if sx >= 0 && sy >= 0 {
                let dest = (sx as u16, sy as u16);
                let blocked = occupied_cells
                    .iter()
                    .any(|&(ox, oy)| ox == dest.0 && oy == dest.1)
                    || used_cells
                        .iter()
                        .any(|&(ux, uy)| ux == dest.0 && uy == dest.1);
                if !blocked {
                    movement::issue_direct_move(&mut sim.entities, pax_id, dest, scatter_speed);
                    break;
                }
            }
        }
        ejected += 1;
    }

    ejected
}
```

**Step 3: Verify**

Run: `cargo check`
Expected: PASS.

**Step 4: Commit**

Commit message: `production: add eject_destruction_garrison helper for combat-destroyed garrisons`

---

### Task 5: Re-export `eject_destruction_garrison` from `production` module

**Why:** Match the export pattern of `eject_destruction_survivors` so `world/mod.rs` can call it as `production::eject_destruction_garrison`.

**Files:**
- Modify: [src/sim/production/mod.rs:33-35](../../src/sim/production/mod.rs#L33-L35).

**Step 1: Update the re-export line**

Change line 33-35 from:

```rust
pub use self::production_sell::{
    eject_destruction_survivors, sell_building, tick_repairs, toggle_repair,
};
```

…to:

```rust
pub use self::production_sell::{
    eject_destruction_garrison, eject_destruction_survivors, sell_building, tick_repairs,
    toggle_repair,
};
```

**Step 2: Verify**

Run: `cargo check`
Expected: PASS.

**Step 3: Commit**

Commit message: `production: re-export eject_destruction_garrison`

---

### Task 6: Wire dispatch in `world/mod.rs` after combat

**Why:** Connect the deferred event collection to the helper. This is the actual behavioral hook — without it, the event sits in `CombatTickResult` unread.

**Files:**
- Modify: [src/sim/world/mod.rs:1194-1205](../../src/sim/world/mod.rs#L1194-L1205) — add a new dispatch loop adjacent to the existing crewed-building dispatch.

**Pattern:** Mirror the crewed-building dispatch loop directly above it.

**Step 1: Insert the dispatch loop**

Immediately after the closing `}` of the crewed-building dispatch loop at line 1205, insert:

```rust
            // Eject garrison occupants from CanBeOccupied buildings destroyed in combat.
            for ev in &combat_result.destroyed_garrison_buildings {
                production::eject_destruction_garrison(self, rules, ev);
            }
```

**Step 2: Verify build**

Run: `cargo check`
Expected: PASS.

**Step 3: Run all sim tests**

Run: `cargo test --lib sim`
Expected: PASS. Existing combat / passenger / production tests should remain green; the new path doesn't affect them yet (no garrison-destruction test until Task 7).

**Step 4: Commit**

Commit message: `world: dispatch garrison eject after combat tick`

---

### Task 7: Add unit tests in `passenger.rs`

**Why:** Verify the end-to-end behavior — building destruction triggers eject, occupants survive at foundation cells, blocked-foundation falls back to kill, transports remain unchanged.

**Files:**
- Modify: [src/sim/passenger.rs](../../src/sim/passenger.rs) — add tests at the end of the existing `mod tests` block (which already has garrison test fixtures `garrison_test_rules`, `spawn_garrison_building`, `spawn_boarding_occupier`).

**Pattern:** Reuse the existing fixtures. Combat is exercised via `tick_combat_with_fog` directly with a minimal scaffold (matches existing combat tests in [src/sim/combat/combat_tests.rs](../../src/sim/combat/combat_tests.rs)).

**Step 1: Add the happy-path test**

At the bottom of `mod tests` (before the closing `}` of the module), add:

```rust
    /// Helper: insert an Occupier infantry directly into a garrison building's
    /// cargo (skipping the boarding flow). Used by destruction-eject tests.
    fn place_inside_garrison(
        sim: &mut Simulation,
        rules: &RuleSet,
        building_id: u64,
        type_ref: &str,
        owner_str: &str,
    ) -> u64 {
        let stable_id = sim.allocate_stable_id();
        let owner_id = sim.interner.intern(owner_str);
        let type_id = sim.interner.intern(type_ref);
        let mut ge = GameEntity::test_default(stable_id, type_ref, owner_str, 0, 0);
        ge.owner = owner_id;
        ge.type_ref = type_id;
        ge.passenger_role = PassengerRole::Inside {
            transport_id: building_id,
        };
        sim.entities.insert(ge);
        // Add to building's cargo.
        if let Some(bldg) = sim.entities.get_mut(building_id) {
            if let Some(cargo) = bldg.passenger_role.cargo_mut() {
                let obj = rules.object(type_ref).expect("type exists");
                cargo.board(stable_id, obj.size.max(1));
            }
        }
        // Building inherits garrisoning player's ownership (sim does this on
        // first board). For destruction tests we set it explicitly here, and
        // also set category=Structure since GameEntity::test_default leaves it
        // as Unit — the death-loop branch keys on Structure.
        if let Some(bldg) = sim.entities.get_mut(building_id) {
            if bldg.garrison_original_owner.is_none() {
                bldg.garrison_original_owner = Some(bldg.owner);
            }
            bldg.owner = owner_id;
            bldg.category = crate::map::entities::EntityCategory::Structure;
        }
        stable_id
    }

    /// Drive `tick_combat_with_fog` once with the building marked dead, then
    /// run the world-layer eject dispatch manually (mirrors what
    /// `World::advance_tick` does after combat).
    fn destroy_and_eject(
        sim: &mut Simulation,
        rules: &RuleSet,
        building_id: u64,
    ) -> Vec<u64> {
        // Mark building as dead by zeroing health.
        if let Some(bldg) = sim.entities.get_mut(building_id) {
            bldg.health.current = 0;
        }
        // Run combat tick — collects DestroyedGarrisonBuilding and skips the
        // kill-passengers branch.
        let mut sound_sink: Vec<crate::sim::world::SimSoundEvent> = Vec::new();
        let combat_result = crate::sim::combat::tick_combat_with_fog(
            &mut sim.entities,
            &mut sim.occupancy,
            rules,
            &mut sim.interner,
            None,
            &std::collections::BTreeMap::new(),
            Some(&mut sound_sink),
            &mut sim.production.resource_nodes,
            sim.tick,
            16,
        );
        // Manual dispatch (mirrors world/mod.rs:1194-1208).
        let mut survivor_ids = Vec::new();
        for ev in &combat_result.destroyed_garrison_buildings {
            survivor_ids.extend(ev.passenger_ids.iter().copied());
            crate::sim::production::eject_destruction_garrison(sim, rules, ev);
        }
        survivor_ids
    }

    #[test]
    fn test_garrison_eject_on_destruction_happy_path() {
        let rules = garrison_test_rules();
        let mut sim = Simulation::new();
        let building_id =
            spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Allied", 10, 10);
        let pax1 = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");
        let pax2 = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");
        let pax3 = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");

        let survivor_ids = destroy_and_eject(&mut sim, &rules, building_id);
        assert_eq!(survivor_ids.len(), 3);

        // Building gone.
        assert!(sim.entities.get(building_id).is_none());

        // All three occupants alive, on the map, owned by Allied, with
        // PassengerRole::None.
        for pid in [pax1, pax2, pax3] {
            let pax = sim.entities.get(pid).expect("survivor present");
            assert!(pax.is_alive(), "occupant {pid} should be alive");
            assert!(!pax.dying, "occupant {pid} should not be dying");
            assert!(matches!(pax.passenger_role, PassengerRole::None));
            assert_eq!(
                sim.interner.resolve(pax.owner),
                "Allied",
                "occupant {pid} should retain garrisoning owner"
            );
            // Position within foundation footprint (1x1 for CAGAS01 default).
            assert_eq!(pax.position.rx, 10);
            assert_eq!(pax.position.ry, 10);
        }
    }
```

**Step 2: Add the blocked-foundation test**

After the happy-path test:

```rust
    #[test]
    fn test_garrison_eject_blocked_foundation_kills_occupants() {
        let rules = garrison_test_rules();
        let mut sim = Simulation::new();
        // CAGAS01 is 1x1 — single foundation cell. Block it with another entity.
        let building_id =
            spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Allied", 10, 10);
        let pax = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");

        // Spawn a blocking entity on the (single) foundation cell.
        let blocker_id = sim.allocate_stable_id();
        let owner_id = sim.interner.intern("Allied");
        let mut blocker =
            GameEntity::test_default(blocker_id, "E1", "Allied", 10, 10);
        blocker.owner = owner_id;
        blocker.type_ref = sim.interner.intern("E1");
        sim.entities.insert(blocker);

        destroy_and_eject(&mut sim, &rules, building_id);

        // Occupant should be marked dying (no free cell to place them).
        let pax_entity = sim.entities.get(pax).expect("entity present");
        assert!(pax_entity.dying, "occupant should be marked dying");
        assert_eq!(pax_entity.health.current, 0);
        assert!(matches!(pax_entity.passenger_role, PassengerRole::None));
    }
```

**Step 3: Add the transport-regression test**

After the blocked-foundation test:

```rust
    #[test]
    fn test_transport_destruction_still_kills_riders() {
        // Regression: the new branch must not fire for transports (CanBeOccupied=no).
        // Use a minimal APC-like rule with Passengers>0 but CanBeOccupied=no.
        let ini_str = "\
[InfantryTypes]
0=E1
[VehicleTypes]
0=APC
[BuildingTypes]

[E1]
Name=Conscript
Cost=100
Strength=125
Armor=none
Speed=4

[APC]
Name=APC
Cost=600
Strength=200
Armor=light
Speed=8
Passengers=5

[General]
[AudioVisual]
ConditionRed=25%
ConditionYellow=50%
";
        let ini = IniFile::from_str(ini_str);
        let rules = RuleSet::from_ini(&ini).expect("parse APC rules");

        let mut sim = Simulation::new();
        let stable_id = sim.allocate_stable_id();
        let owner_id = sim.interner.intern("Allied");
        let type_id = sim.interner.intern("APC");
        let mut transport =
            GameEntity::test_default(stable_id, "APC", "Allied", 10, 10);
        transport.owner = owner_id;
        transport.type_ref = type_id;
        let obj = rules.object("APC").expect("APC exists");
        transport.passenger_role = PassengerRole::Transport {
            cargo: PassengerCargo::new(obj.passengers, 0),
        };
        sim.entities.insert(transport);

        let pax = place_inside_garrison(&mut sim, &rules, stable_id, "E1", "Allied");

        destroy_and_eject(&mut sim, &rules, stable_id);

        // Rider should be dead — transport keeps the kill-all behavior.
        let pax_entity = sim.entities.get(pax).expect("rider entity exists");
        assert!(pax_entity.dying, "transport rider should be killed");
        assert_eq!(pax_entity.health.current, 0);
    }
```

**Step 4: Verify imports for the test module**

The new tests reference `Simulation::new()`, `tick_combat_with_fog`, and `eject_destruction_garrison`. The existing imports at the top of `mod tests` are:

```rust
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
```

`Simulation::new()` is the constructor used by all existing tests in this module (e.g., line 805 of `passenger.rs`). No new imports needed beyond what's already present — `super::*` brings in `PassengerCargo`, `PassengerRole`, and `BoardingPhase`; the `IniFile`/`RuleSet` imports cover the APC test's INI parsing.

**Step 5: Run tests**

Run: `cargo test --lib sim::passenger::tests::test_garrison_eject -- --nocapture`
Expected: 2 tests pass.

Run: `cargo test --lib sim::passenger::tests::test_transport_destruction -- --nocapture`
Expected: 1 test passes.

Run: `cargo test --lib sim::passenger`
Expected: ALL tests pass — no regressions in the existing 30+ passenger tests.

**Step 6: Commit**

Commit message: `passenger: add tests for garrison eject on destruction`

---

### Task 8: Manual in-game parity verification

**Why:** Unit tests cover the mechanic in isolation; the parity bar is "indistinguishable from gamemd.exe in a single skirmish." This step confirms the player-visible result matches.

**Verify:**

Run the engine in skirmish mode. Place or find a `CanBeOccupied` civilian building (e.g., gas station / hospital). Garrison it with 5 conscripts. Attack with a Rhino tank until destroyed.

Expected (matches gamemd):
- Building's death animation plays.
- 5 conscripts spawn at the rubble cells (within the foundation footprint).
- Conscripts immediately scatter outward to adjacent cells.
- Conscripts' owner = the player who garrisoned (selectable, controllable).
- No conscripts are silently lost.

Run the same scenario in `gamemd.exe` (original engine) for visual comparison. Differences to check:
- Spawn positions: should be near-identical relative to building footprint.
- Scatter timing: occupants should start moving on the same tick they appear.
- Owner: clicking an ejected conscript should select them as your unit.

If the spawn positions look noticeably different (e.g., on the wrong side of the foundation), revisit Task 4's cell-shuffle logic — gamemd's `random_foundation_offset` may pick differently.

**Step 1: Build release**

Run: `cargo build --release`

**Step 2: Run skirmish, perform the scenario above.**

**Step 3: If mismatches, file a follow-up task. Otherwise, commit any cleanup.**

No commit if no changes.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-04-garrison-eject-on-destruction-design.md](2026-05-04-garrison-eject-on-destruction-design.md)
- **Ghidra reports:**
  - `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` §4a — `BuildingClass::SpawnSurvivors @ 0x00442D90` garrison occupant ejection on destruction. LIFO iteration, random foundation cell offset, owner = `field_0x8C`, unlimbo failure → Destroy.
  - `GARRISON_SYSTEM_GHIDRA_REPORT.md` §6 / §14c — parallel `BuildingClass::SellBuilding @ 0x00457DE0` flow (perimeter cells, parachute fallback) — used as the mirror for the existing `eject_garrison_occupants` sell helper.
- **gamemd.exe addresses (kept here, not in code comments):**
  - `0x00442D90` `BuildingClass::SpawnSurvivors` — destruction eject path
  - `0x00457DE0` `BuildingClass::SellBuilding` — sell eject path
  - `field_0x8C` — building owner (HouseClass index)
  - `field_0x6E0` — IC-killed flag (out of scope this plan)
  - `field_0x16AE` `CanBeOccupied`
- **INI keys:** `CanBeOccupied=yes` (rules*.ini, parsed at [src/rules/object_type.rs:412](../../src/rules/object_type.rs#L412)); `Foundation=NxM` (art*.ini, parsed via [foundation_dimensions](../../src/sim/production/production_tech.rs#L562)).
- **Related code:**
  - [src/sim/combat/mod.rs:282-314](../../src/sim/combat/mod.rs#L282-L314) — `DestroyedCrewedBuilding` pattern this mirrors
  - [src/sim/world/mod.rs:1194-1205](../../src/sim/world/mod.rs#L1194-L1205) — crewed dispatch hook this duplicates
  - [src/sim/production/production_sell.rs:184-227](../../src/sim/production/production_sell.rs#L184-L227) — `eject_destruction_survivors` shape
  - [src/sim/production/production_sell.rs:246-373](../../src/sim/production/production_sell.rs#L246-L373) — `eject_garrison_occupants` (sell-eject sibling)
- **Recent related commits:**
  - `65f0b1d garrison: left-click on selected garrisoned building unloads occupants`
  - `78ab9c4 garrison: emit StructureAbandoned on last-occupant unload (pre-revert owner)`
