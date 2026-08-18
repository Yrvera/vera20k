# Refinery FreeUnit Completion and Stock Primary Placement Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make stock Allied and Soviet refineries create their configured free
miner exactly once on the existing Rust building-up completion transition, at
the verified stock primary cell, without adding a second scheduler or persistent
one-shot state.

**Architecture:** `Simulation::tick_building_up` remains the deterministic
completion-order owner and returns the stable IDs that transitioned. Phase 9
immediately hands those IDs to a production-owned FreeUnit service, which uses
merged rules/art data and the existing world spawn/lifecycle path. No app,
render, network, snapshot, or command interface changes.

**Design Doc:** `docs/plans/2026-07-28-refinery-freeunit-completion-and-stock-primary-placement-design.md`

---

## Execution Preconditions

The checkout was on `dev` at `55c66381` during planning and contained unrelated
dirty app/RMG/render work. The five Rust files in this plan were clean, and no
Cargo or rustc process was running. Before implementation:

1. Re-run `git status --short`, the per-file logs in the File Map, and the Cargo
   process check from `ENGINE.md`.
2. Do not modify another task's dirty checkout. If another task still owns
   `dev`, Cargo, or the current checkout, execute this plan in an isolated
   `feature/` worktree. If the repo has become sole-owner and clean, follow the
   standing rule and work directly on `dev`.
3. Do not stage, commit, merge, or push unless the user separately authorizes
   those actions.

## Grounding Summary

- The corrected
  `docs/research/miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` is the primary
  verified handoff. It assigns FreeUnit creation to construction completion,
  derives stock `4x3` primary `NW+(2,2)`, preserves `0xC0` primary and `0xA0`
  fallback bytes, and records the undecoded two-pass fallback.
- Live `decompile_function(0x00449A50)` during planning reconfirmed that the
  active Building construction mission waits for completion, invokes vtable
  slot `+0x4DC`, then assigns building mission 5.
- Live `decompile_function(0x00445F80)` during planning reconfirmed the resolved
  `Type+0xEA0` gate, primary Unlimbo attempt with `0xC0`, two fallback attempts
  with `0xA0`, mission 10 plus Commence, and failure refund/destruction.
- Live `decompile_function(0x00447AC0)` during planning reconfirmed building
  coordinates as location plus `foundation*128-128` leptons on each horizontal
  axis. For stock `4x3`, center cell plus south is `NW+(2,2)`.
- `rulesmd.ini` activates `[GAREFN] FreeUnit=CMIN` and
  `[NAREFN] FreeUnit=HARV`; `artmd.ini` gives both `Foundation=4x3`.
- `src/rules/object_type.rs` already parses section-local `FreeUnit`, and
  `RuleSet::refinery_free_unit` already performs the scoped refinery lookup.
- `src/sim/production/production_placement.rs` currently attaches `BuildingUp`
  and immediately creates the miner. `src/sim/world/mod.rs::tick_building_up`
  already gathers completions in stable-ID order but discards that list after
  clearing the components.
- `src/sim/production/production_refinery.rs` currently uses `NW+(width/2,height)`
  and gates primary through `PathGrid`.
- By completion, the rebuilt Rust `PathGrid` normally contains the refinery's
  movement footprint. `building_movement_blocking_cells` relaxes only the
  `Bib=yes` east-edge column; the native FreeUnit primary `NW+(2,2)` remains an
  interior static blocker. The completion service must therefore attempt the
  valid internal primary without treating the source refinery's `PathGrid` mark
  as a rejection.
- Still unknown and excluded here: exact fallback candidate order, independent
  dynamic-blocker rejection, one-object retry/refund cleanup, the
  `BuildingClass+0x300` gate, exact mission assign-plus-Commence equivalence, and
  generic non-refinery mod activation.

## Key Technical Decisions

- **Completion transition owns exactly one attempt:** return the existing
  `finished` vector from `tick_building_up` after clearing each component, then
  consume it immediately in Phase 9. **Confidence: high**
  - **Source:** live `0x00449A50 -> 0x00445F80`; current
    `src/sim/world/mod.rs:1802-1821`.
- **Production policy stays in `production_refinery.rs`:** world code supplies
  ordered IDs and tick inputs but does not inspect FreeUnit data. **Confidence:
  high**
  - **Source:** approved design and current `crate::sim::production` pattern.
- **Stock primary is checked center-plus-south:** use wider checked integer
  intermediates and return `None` if the result cannot be represented as
  `u16`. **Confidence: high**
  - **Source:** live `0x00447AC0`, `0x00445F80`, stock `Foundation=4x3`.
- **Do not pre-reject a valid internal primary through `PathGrid`:** that static
  blocker is the completing source refinery. The foundation was already
  validated at placement. **Confidence: high**
  - **Source:** current `building_movement_blocking_cells`, stock `Bib=yes`, and
    `docs/research/STOCK_REFINERY_ART_REMOVE_OCCUPY_PAD_CELL_GHIDRA_REPORT.md`.
- **Preserve current generic spawn and Harvest result:** this closes the
  ordinary primary-success path without inventing lifecycle/refund behavior.
  **Confidence: medium**
  - **Source:** current `world_spawn.rs:260-430`; native mission 10 is verified,
    but exact assign-plus-Commence state equivalence is deferred.
- **No new persistent state or hash field:** the serialized
  `BuildingUp -> None` transition is the one-shot fact. **Confidence: high**
  - **Source:** approved architecture and current `GameEntity.building_up`.
- **Merged in-repo retail rules/art drive stock tests:** parse base INIs, overlay
  `*md`, then call `RuleSet::merge_art_data`. **Confidence: high**
  - **Source:** current production load order in
    `src/app_init_helpers.rs::compose_rules_layers` and `RuleSet::merge_art_data`.

The medium-confidence mission decision must remain labelled as an
exactification residual in source/test prose. It does not require `/review-plan`
before the scoped primary-success implementation because the current Rust result
already establishes Harvest mission 10 and this plan does not claim exact
Commence timing.

## Open Questions

### Resolved During Planning

- **Will the completion-time `PathGrid` admit `NW+(2,2)`?** No. The source
  refinery normally blocks that interior cell; only the east edge is relaxed by
  `Bib=yes`. The FreeUnit primary must bypass that source-static precheck.
- **Does moving the call require new serialized one-shot state?** No.
  `tick_building_up` already clears a serialized optional component once.
- **Does normal production have rules available?** Yes. The app's production
  tick supplies `Some(&RuleSet)`; no-rules calls found during planning were
  synthetic/headless test and diagnostic paths.
- **Are new parsing tasks needed?** No. `FreeUnit`, refinery identity, target
  resolution, foundation art merge, owner, facing, and miner mission support
  already exist.

### Deferred After This Slice

- Exact candidate sequence and boolean meaning for the two native nearby-cell
  searches.
- Result-bearing primary rejection for an independently injected dynamic
  blocker, retrying the same constructed object, total-failure cleanup, and
  owner-aware refund.
- Semantic producer and active stock trigger for `BuildingClass+0x300`.
- Exact mission current/queued/timer/handler-state equivalence of native
  assign-plus-Commence.
- Native generic BuildingType activation with UnitType-only target restriction.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/production/production_refinery.rs` | Completion-owned FreeUnit service, checked stock-primary derivation, spawn result |
| Modify | `src/sim/production/mod.rs` | Crate-internal re-export of the completion service |
| Modify | `src/sim/production/production_placement.rs` | Remove premature placement-time creation |
| Modify | `src/sim/world/mod.rs` | Return completed IDs and invoke the service in deterministic Phase 9 |
| Modify | `src/sim/production/production_placement_tests.rs` | Retail-data timing, cell, owner, mission, negative, and ordering acceptance tests |

No code file is created. `production_refinery.rs` remains far below the
approximately 600-line split threshold. The existing placement test suite is a
cohesive test-only exception.

No System Map file changes are planned: the frozen map has no existing FreeUnit
mechanism/edge to correct, and creating registry surface for this slice would
violate its no-growth rule.

## Interface Changes

- Private `Simulation::tick_building_up(&mut self)` changes from `()` to
  `Vec<u64>`. Its only production caller is Phase 9 in `run_late_region`.
- Add crate-internal
  `production::spawn_completed_refinery_free_units(&mut Simulation, &[u64], &RuleSet, Option<&PathGrid>, &BTreeMap<...>) -> bool`.
- Replace the private placement-oriented helper with a completion-oriented
  boolean-returning helper.
- No public API, command, snapshot, serialization, INI, network, or app contract
  changes.

## Sim Checklist

- [ ] No `f32`/`f64` added to gameplay logic; cell math uses checked integers.
- [ ] No new deterministic state; no state-hash field or snapshot-version bump.
- [ ] No dependency from `sim/` to render, UI, sidebar, audio, or network.
- [ ] Tick impact is confined to Phase 9: creation occurs same-tick after
      `BuildingUp` clears and before `tick_building_down`.
- [ ] Completing IDs and resulting stable-ID allocation commit in ascending
      `EntityStore` stable-ID order.
- [ ] No RNG calls are added.

## Risk Areas

- Removing the placement call before Phase-9 wiring would temporarily suppress
  all free miners; Task 2 performs both changes as one integration unit.
- Calling production policy while mutably iterating `EntityStore` would violate
  borrow and ordering assumptions; the service consumes the pre-collected IDs
  only after `tick_building_up` ends.
- Consulting `PathGrid` for a valid primary would send every normal completion
  to uncertified fallback because the source refinery blocks its internal cell.
- Failing to fold the service result into `spawned_entities` would leave the app
  atlas stale on the completion tick.
- Full-retail tests must merge `rulesmd.ini` over `rules.ini` and `artmd.ini`
  over `art.ini`; parsing only one variant would test the wrong game data.
- Existing mod-refinery and absent-key tests encode placement-time behavior and
  must be retimed rather than deleted.

## Player-Experience Critical Items

Representative scenario: a human or AI house places a stock GAREFN or NAREFN in
an ordinary YR skirmish, waits for the current Rust build-up transition, and has
the native internal primary cell available except for the source refinery's own
expected static footprint.

| Task # | Class | Item | Why it matters | Verification |
|---|---|---|---|---|
| 2-3 | MILESTONE-BLOCKING | Completion-owned, one-shot timing | Placement-time miners are visible and economically usable early on every refinery build | No unit before transition; exactly one on transition; none later |
| 1,3 | MILESTONE-BLOCKING | Stock `4x3` primary `NW+(2,2)` | Current Rust is one cell too far south | GAREFN/NAREFN at `(20,20)` create at `(22,22)`, never `(22,23)` |
| 1,3 | MILESTONE-BLOCKING | Ignore source refinery's static primary blocker | Rebuilt `PathGrid` otherwise defeats the native primary in ordinary play | Block the full `4x3` foundation in the test grid before completion and still create at `(22,22)` |
| 1-3 | MILESTONE-BLOCKING | Data-driven type and owner | Allied/Soviet stock mappings differ and faction inference would be wrong | Merged retail GAREFN→CMIN and NAREFN→HARV owner assertions |
| 2-3 | COMPOUNDING | Stable completion/spawn order | Stable-ID order affects allocation, mission state, credits, and replay hashes | Simultaneous GAREFN/NAREFN completion produces ordered unit IDs |
| 2-3 | COMPOUNDING | Completion-tick `spawned_entities` | Downstream presentation cache must see the new type immediately | TickResult false before and true on successful completion |
| 1,3 | TEST-ONLY | Preserve `0xC0` primary and Harvest/10 | Correct bytes already exist; stale comments must not drive a byte change | Entity facing and mission assertions |
| — | COMPOUNDING FOLLOW-UP | Independent blocker rejection/refund | Load/editor/injected occupancy can overlap or miss a native refund; uncommon in ordinary placement but lifecycle-sensitive | Explicitly excluded; no test may claim this parity |
| — | EXACTIFICATION-RESIDUAL | Native two-pass fallback order | Trigger is primary rejection; player may see a different fallback cell | Existing fallback retained only for unrepresentable primary and labelled DRIFT |
| — | EXACTIFICATION-RESIDUAL | Assign-plus-Commence equivalence | First eligible Harvest AI tick may differ under expert timing probes | Preserve current Harvest result without exact claim |
| — | EXACTIFICATION-RESIDUAL | `+0x300` gate and generic mod activation | No demonstrated frequent stock trigger; wrong semantics would create speculative state | No implementation in this slice |

---

## Tasks

### Task 1: Add the production-owned completion service and checked primary-cell helper

**Why:** Define and unit-test the production contract before Phase 9 consumes it.

**Files:**

- Modify: `src/sim/production/production_refinery.rs:1-122`

**Pattern:** Follow `EntityStore`'s documented batch pattern: receive stable IDs,
snapshot immutable data, end the borrow, then mutate `Simulation`. Continue using
the existing rules lookup, foundation parser, and generic `spawn_object` path.

**Step 1: Replace stale imports, constants, and comments**

Use these imports and meanings:

```rust
use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

use super::production_tech::foundation_dimensions;

/// Native primary FreeUnit facing byte. Under the project facing convention,
/// 0xC0 is west.
const FREE_UNIT_FACING_PRIMARY: u8 = 0xC0;
/// Native fallback FreeUnit facing byte. Under the project facing convention,
/// 0xA0 is southwest.
const FREE_UNIT_FACING_FALLBACK: u8 = 0xA0;
```

Do not put gamemd addresses in Rust comments.

**Step 2: Add the checked primary-cell transform**

```rust
fn primary_free_unit_cell(
    building_rx: u16,
    building_ry: u16,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    let center_x = u32::from(building_rx).checked_add(u32::from(width) / 2)?;
    let center_y = u32::from(building_ry).checked_add(u32::from(height) / 2)?;
    let primary_y = center_y.checked_add(1)?;
    Some((
        u16::try_from(center_x).ok()?,
        u16::try_from(primary_y).ok()?,
    ))
}
```

This names the canonical north-west cell frame and prevents overflow from
clamping or wrapping into another cell.

**Step 3: Add the ordered completion service**

```rust
pub(crate) fn spawn_completed_refinery_free_units(
    sim: &mut Simulation,
    completed_building_ids: &[u64],
    rules: &RuleSet,
    path_grid: Option<&PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    let mut any_spawned = false;

    for &stable_id in completed_building_ids {
        let Some((owner_id, type_ref, rx, ry, width, height)) = ({
            let entity = sim.substrate.entities.get(stable_id);
            entity.and_then(|entity| {
                if entity.category != EntityCategory::Structure {
                    return None;
                }
                let (width, height) = foundation_dimensions(&entity.foundation);
                Some((
                    entity.owner,
                    entity.type_ref,
                    entity.position.rx,
                    entity.position.ry,
                    width,
                    height,
                ))
            })
        }) else {
            continue;
        };

        // These allocations occur only for completed buildings, not every tick.
        // They end immutable interner borrows before spawn_object mutates sim.
        let owner = sim.interner.resolve(owner_id).to_owned();
        let building_type_id = sim.interner.resolve(type_ref).to_owned();
        any_spawned |= try_spawn_refinery_free_unit(
            sim,
            rules,
            &owner,
            &building_type_id,
            rx,
            ry,
            width,
            height,
            path_grid,
            height_map,
        );
    }

    any_spawned
}
```

The service must not sort again. Its input order is the `tick_building_up`
contract.

**Step 4: Replace the placement-oriented helper with a boolean result**

Use this signature and cell selection:

```rust
fn try_spawn_refinery_free_unit(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    building_type_id: &str,
    building_rx: u16,
    building_ry: u16,
    width: u16,
    height: u16,
    path_grid: Option<&PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    if !rules.is_refinery_type(building_type_id) {
        return false;
    }

    let Some(free_unit_type) = rules.refinery_free_unit(building_type_id) else {
        return false;
    };

    let (rx, ry, facing) = if let Some((primary_rx, primary_ry)) =
        primary_free_unit_cell(building_rx, building_ry, width, height)
    {
        // Do not consult PathGrid here. By completion it contains the source
        // refinery's own static blocker over this native internal bay.
        (primary_rx, primary_ry, FREE_UNIT_FACING_PRIMARY)
    } else {
        let Some((fallback_rx, fallback_ry)) =
            find_adjacent_spawn_cell(building_rx, building_ry, width, height, path_grid)
        else {
            log::warn!(
                "No representable cell near completed refinery ({},{}) to spawn {}",
                building_rx,
                building_ry,
                free_unit_type
            );
            return false;
        };
        (
            fallback_rx,
            fallback_ry,
            FREE_UNIT_FACING_FALLBACK,
        )
    };

    let spawned = sim
        .spawn_object(
            free_unit_type,
            owner,
            rx,
            ry,
            facing,
            rules,
            height_map,
        )
        .is_some();

    if spawned {
        log::info!(
            "Completed refinery {} spawned free {} at ({},{}) for {}",
            building_type_id,
            free_unit_type,
            rx,
            ry,
            owner
        );
    } else {
        log::warn!(
            "Completed refinery {} resolved free unit {} but spawn_object failed at ({},{}) for {}",
            building_type_id,
            free_unit_type,
            rx,
            ry,
            owner
        );
    }

    spawned
}
```

Keep `find_adjacent_spawn_cell` mechanically unchanged except for imports and
prose that might imply native parity. It remains an explicit residual.

**Step 5: Keep Task 1 compilable with a temporary placement adapter**

Until Task 2 removes the old placement caller, retain its name and signature as
a narrow adapter:

```rust
pub(super) fn maybe_spawn_refinery_harvester(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    building_type_id: &str,
    building_rx: u16,
    building_ry: u16,
    path_grid: Option<&PathGrid>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> bool {
    let Some(building_type) = rules.object_case_insensitive(building_type_id) else {
        return false;
    };
    let (width, height) = foundation_dimensions(&building_type.foundation);
    try_spawn_refinery_free_unit(
        sim,
        rules,
        owner,
        building_type_id,
        building_rx,
        building_ry,
        width,
        height,
        path_grid,
        height_map,
    )
}
```

This adapter exists only so Task 1 compiles in isolation. Task 2 deletes it
together with the placement import and call; it must not survive the completed
slice.

**Step 6: Add pure transform tests in the same file**

```rust
#[cfg(test)]
mod tests {
    use super::primary_free_unit_cell;

    #[test]
    fn stock_4x3_primary_cell_is_center_plus_south() {
        assert_eq!(primary_free_unit_cell(20, 20, 4, 3), Some((22, 22)));
    }

    #[test]
    fn primary_cell_rejects_u16_overflow() {
        assert_eq!(
            primary_free_unit_cell(u16::MAX, u16::MAX, 4, 3),
            None
        );
    }
}
```

These are Rust transform regressions. The parity premise remains the live binary
evidence in the Sources section.

**Step 7: Format and verify Task 1**

Run:

```text
rustfmt --edition 2024 src/sim/production/production_refinery.rs
cargo test -p vera20k --lib sim::production::production_refinery::tests:: -- --nocapture
```

Expected result line begins:

```text
test result: ok. 2 passed; 0 failed
```

If another session acquires Cargo after the preflight, wait rather than starting
the command.

### Task 2: Move ownership from placement into deterministic Phase 9

**Why:** Make the existing serialized completion transition own the same-tick
FreeUnit attempt and downstream spawn notification.

**Files:**

- Modify: `src/sim/production/mod.rs:10-42`
- Modify: `src/sim/production/production_placement.rs:20,164-253`
- Modify: `src/sim/world/mod.rs:1801-1821,1932-1990`

**Pattern:** Mirror `tick_building_down`: collect completion facts first, then
commit side effects after mutable iteration. Keep domain policy behind the
existing `production` module boundary.

**Step 1: Re-export the completion service**

Add beside the other crate-internal production exports:

```rust
pub(crate) use self::production_refinery::spawn_completed_refinery_free_units;
```

No public export is added.

**Step 2: Return the existing finished vector from `tick_building_up`**

Replace the function with:

```rust
/// Advance build-up animations and return the stable IDs that completed this
/// tick, in ascending EntityStore order.
fn tick_building_up(&mut self) -> Vec<u64> {
    let keys = self.substrate.entities.keys_sorted();
    let mut finished: Vec<u64> = Vec::new();
    for &sid in &keys {
        if let Some(entity) = self.substrate.entities.get_mut(sid) {
            if let Some(ref mut bu) = entity.building_up {
                bu.elapsed_ticks = bu.elapsed_ticks.saturating_add(1);
                if bu.elapsed_ticks >= bu.total_ticks {
                    finished.push(sid);
                }
            }
        }
    }
    for &sid in &finished {
        if let Some(entity) = self.substrate.entities.get_mut(sid) {
            entity.building_up = None;
        }
    }
    finished
}
```

Returning `finished` reuses the vector already allocated by the current code.
Do not add a pending component, queue, hash field, or second scan.

**Step 3: Invoke the production service immediately in Phase 9**

Replace the current `self.tick_building_up();` call with:

```rust
let completed_buildings = self.tick_building_up();
if let Some(rules) = rules {
    *spawned_entities |= production::spawn_completed_refinery_free_units(
        self,
        &completed_buildings,
        rules,
        path_grid,
        height_map,
    );
}
```

Keep `tick_building_down(rules)` immediately after this block. This fixes
same-tick ownership without changing phase order.

**Step 4: Remove placement-time creation**

Delete:

```rust
use super::production_refinery::maybe_spawn_refinery_harvester;
```

and delete the call immediately after `BuildingUp` is attached:

```rust
maybe_spawn_refinery_harvester(sim, rules, owner, type_id, rx, ry, path_grid, height_map);
```

Delete the temporary `maybe_spawn_refinery_harvester` adapter added in Task 1.
After this deletion, `try_spawn_refinery_free_unit` is reached only through the
completion service.

Do not move superweapon refresh or ready-queue consumption; their existing
placement semantics are outside this slice.

**Step 5: Verify interface consumers and architecture**

Run these read-only searches:

```text
rg -n "tick_building_up\(" src
rg -n "maybe_spawn_refinery_harvester|spawn_completed_refinery_free_units" src
rg -n "crate::(render|ui|sidebar|audio|net)|crate::sim::(render|ui|sidebar|audio|net)" src/sim/production/production_refinery.rs src/sim/world/mod.rs
```

Expected:

- one `tick_building_up` definition and one Phase-9 caller;
- no old helper name or placement-time call;
- one production re-export and one Phase-9 completion-service call;
- no new forbidden dependency.

Then run:

```text
cargo check -p vera20k
```

Expected: exit code 0.

Do not run rustfmt on `src/sim/world/mod.rs`; project policy forbids formatting a
`mod.rs` because it recursively formats submodules.

### Task 3: Replace placement-time tests with retail completion acceptance

**Why:** Exercise the real Phase-9 transition, merged stock data, the rebuilt
static-grid condition, one-shot behavior, and deterministic spawn ordering.

**Files:**

- Modify: `src/sim/production/production_placement_tests.rs:1-25,508-681`

**Pattern:** Continue using the production test module's existing
`spawn_structure`, ready-queue, `place_ready_building`, and headless
`advance_tick` helpers. Add one `OnceLock<RuleSet>` so the four in-repo retail
INIs are parsed once for the whole test module.

**Step 1: Add imports**

Add:

```rust
use std::path::Path;
use std::sync::OnceLock;

use crate::map::entities::EntityCategory;
use crate::rules::art_data::ArtRegistry;
use crate::sim::components::BuildingUp;
use crate::sim::mission::MissionType;
```

Retain the existing `Health` import. Add `foundation_dimensions` to the existing
`use super::{...}` list.

**Step 2: Add merged-retail and completion helpers**

```rust
fn merged_retail_refinery_rules() -> &'static RuleSet {
    static RULES: OnceLock<RuleSet> = OnceLock::new();
    RULES.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |relative: &str| {
            std::fs::read_to_string(root.join(relative))
                .unwrap_or_else(|error| panic!("read {relative}: {error}"))
        };

        let mut rules_ini = IniFile::from_str(&read("ini/rules.ini"));
        rules_ini.merge(&IniFile::from_str(&read("ini/rulesmd.ini")));
        let mut rules = RuleSet::from_ini(&rules_ini).expect("merged retail rules parse");

        let mut art_ini = IniFile::from_str(&read("ini/art.ini"));
        art_ini.merge(&IniFile::from_str(&read("ini/artmd.ini")));
        let art = ArtRegistry::from_ini(&art_ini);
        rules.merge_art_data(&art);
        rules
    })
}

fn ready_and_place(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
    rx: u16,
    ry: u16,
    path_grid: &PathGrid,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> u64 {
    let owner_id = sim.interner.intern(owner);
    let type_ref = sim.interner.intern(type_id);
    sim.production
        .ready_by_owner
        .entry(owner_id)
        .or_default()
        .push_back(type_ref);
    assert!(place_ready_building(
        sim,
        rules,
        owner,
        type_id,
        rx,
        ry,
        Some(path_grid),
        height_map,
    ));
    sim.substrate
        .entities
        .values()
        .find(|entity| {
            entity.category == EntityCategory::Structure
                && entity.owner == owner_id
                && entity.type_ref == type_ref
                && (entity.position.rx, entity.position.ry) == (rx, ry)
        })
        .map(|entity| entity.stable_id)
        .expect("placed building entity")
}

fn set_ticks_until_completion(sim: &mut Simulation, stable_id: u64, ticks: u16) {
    assert!(ticks > 0);
    let buildup = sim
        .substrate
        .entities
        .get_mut(stable_id)
        .and_then(|entity| entity.building_up.as_mut())
        .expect("placed building has BuildingUp");
    buildup.elapsed_ticks = buildup.total_ticks.saturating_sub(ticks);
}

fn block_building_foundation(
    grid: &mut PathGrid,
    rules: &RuleSet,
    type_id: &str,
    rx: u16,
    ry: u16,
) {
    let object = rules.object(type_id).expect("stock building type");
    let (width, height) = foundation_dimensions(&object.foundation);
    for dy in 0..height {
        for dx in 0..width {
            grid.set_blocked(rx + dx, ry + dy, true);
        }
    }
}

fn unit_ids(sim: &Simulation, owner: &str, type_id: &str) -> Vec<u64> {
    sim.substrate
        .entities
        .values()
        .filter(|entity| {
            entity.category == EntityCategory::Unit
                && sim.interner.resolve(entity.owner).eq_ignore_ascii_case(owner)
                && sim
                    .interner
                    .resolve(entity.type_ref)
                    .eq_ignore_ascii_case(type_id)
        })
        .map(|entity| entity.stable_id)
        .collect()
}
```

The test-only filesystem reads use committed in-repo INIs, not an external
retail installation. `OnceLock` prevents repeated multi-megabyte parsing.

**Step 3: Replace `refinery_placement_spawns_one_starter_harvester`**

```rust
#[test]
fn stock_refinery_free_unit_spawns_on_building_up_completion_once() {
    let rules = merged_retail_refinery_rules();
    let mut sim = Simulation::new();
    let height_map = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    let refinery = ready_and_place(
        &mut sim,
        rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, rules, "GAREFN", 20, 20);
    set_ticks_until_completion(&mut sim, refinery, 2);

    assert!(unit_ids(&sim, "Americans", "CMIN").is_empty());
    let before = sim.advance_tick(&[], Some(rules), &height_map, Some(&grid), None, 33);
    assert!(!before.spawned_entities);
    assert!(sim.substrate.entities.get(refinery).unwrap().building_up.is_some());
    assert!(unit_ids(&sim, "Americans", "CMIN").is_empty());

    let completion =
        sim.advance_tick(&[], Some(rules), &height_map, Some(&grid), None, 33);
    assert!(completion.spawned_entities);
    assert!(sim.substrate.entities.get(refinery).unwrap().building_up.is_none());
    assert_eq!(unit_ids(&sim, "Americans", "CMIN").len(), 1);

    let later = sim.advance_tick(&[], Some(rules), &height_map, Some(&grid), None, 33);
    assert!(!later.spawned_entities);
    assert_eq!(unit_ids(&sim, "Americans", "CMIN").len(), 1);
}
```

This single test covers pre-completion absence, same-tick notification, component
clear order, and no duplication.

**Step 4: Add the Allied primary-cell assertion**

```rust
#[test]
fn stock_4x3_refinery_free_unit_uses_native_primary_cell() {
    let rules = merged_retail_refinery_rules();
    let mut sim = Simulation::new();
    let height_map = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    let refinery = ready_and_place(
        &mut sim,
        rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, rules, "GAREFN", 20, 20);
    set_ticks_until_completion(&mut sim, refinery, 1);

    let result = sim.advance_tick(&[], Some(rules), &height_map, Some(&grid), None, 33);
    assert!(result.spawned_entities);
    let ids = unit_ids(&sim, "Americans", "CMIN");
    assert_eq!(ids.len(), 1);
    let miner = sim.substrate.entities.get(ids[0]).unwrap();
    assert_eq!((miner.position.rx, miner.position.ry), (22, 22));
    assert_ne!((miner.position.rx, miner.position.ry), (22, 23));
    assert_eq!(miner.facing, 0xC0);
    assert_eq!(miner.mission.current().known(), Some(MissionType::Harvest));
}
```

The blocked `4x3` test grid proves the completing source structure cannot divert
the internal primary.

**Step 5: Add the Soviet stock-data variant**

```rust
#[test]
fn stock_soviet_refinery_completion_spawns_harv() {
    let rules = merged_retail_refinery_rules();
    let mut sim = Simulation::new();
    let height_map = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Russians", "NACNST", 14, 20);
    let refinery = ready_and_place(
        &mut sim,
        rules,
        "Russians",
        "NAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    block_building_foundation(&mut grid, rules, "NAREFN", 20, 20);
    set_ticks_until_completion(&mut sim, refinery, 1);

    sim.advance_tick(&[], Some(rules), &height_map, Some(&grid), None, 33);
    let ids = unit_ids(&sim, "Russians", "HARV");
    assert_eq!(ids.len(), 1);
    let miner = sim.substrate.entities.get(ids[0]).unwrap();
    assert_eq!((miner.position.rx, miner.position.ry), (22, 22));
    assert_eq!(miner.facing, 0xC0);
    assert_eq!(miner.mission.current().known(), Some(MissionType::Harvest));
    assert!(unit_ids(&sim, "Russians", "CMIN").is_empty());
}
```

**Step 6: Add the ConYard absent-key negative**

```rust
#[test]
fn gacnst_completion_has_no_free_unit() {
    let rules = merged_retail_refinery_rules();
    let mut sim = Simulation::new();
    let height_map = BTreeMap::new();
    let conyard = sim
        .spawn_object(
            "GACNST",
            "Americans",
            20,
            20,
            0,
            rules,
            &height_map,
        )
        .expect("spawn stock ConYard");
    sim.substrate.entities.get_mut(conyard).unwrap().building_up = Some(BuildingUp {
        elapsed_ticks: 0,
        total_ticks: 1,
    });
    let credits_before = credits_for_owner(&sim, "Americans");

    let result = sim.advance_tick(&[], Some(rules), &height_map, None, None, 33);
    assert!(!result.spawned_entities);
    assert!(unit_ids(&sim, "Americans", "CMIN").is_empty());
    assert!(unit_ids(&sim, "Americans", "HARV").is_empty());
    assert_eq!(credits_for_owner(&sim, "Americans"), credits_before);
}
```

This proves absent-key behavior without adding a faction-derived miner.

**Step 7: Add simultaneous-completion ordering**

```rust
#[test]
fn simultaneous_refinery_completions_preserve_stable_id_order() {
    let rules = merged_retail_refinery_rules();
    let mut sim = Simulation::new();
    let height_map = BTreeMap::new();
    let mut grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 14, 20);
    spawn_structure(&mut sim, 2, "Russians", "NACNST", 14, 35);
    let allied = ready_and_place(
        &mut sim,
        rules,
        "Americans",
        "GAREFN",
        20,
        20,
        &grid,
        &height_map,
    );
    let soviet = ready_and_place(
        &mut sim,
        rules,
        "Russians",
        "NAREFN",
        20,
        35,
        &grid,
        &height_map,
    );
    assert!(allied < soviet);
    block_building_foundation(&mut grid, rules, "GAREFN", 20, 20);
    block_building_foundation(&mut grid, rules, "NAREFN", 20, 35);
    set_ticks_until_completion(&mut sim, allied, 1);
    set_ticks_until_completion(&mut sim, soviet, 1);

    let result = sim.advance_tick(&[], Some(rules), &height_map, Some(&grid), None, 33);
    assert!(result.spawned_entities);
    let cmin = unit_ids(&sim, "Americans", "CMIN");
    let harv = unit_ids(&sim, "Russians", "HARV");
    assert_eq!(cmin.len(), 1);
    assert_eq!(harv.len(), 1);
    assert!(
        cmin[0] < harv[0],
        "lower completing building stable ID must allocate its FreeUnit first"
    );
}
```

**Step 8: Retime the two retained mod/refinery tests**

Rename:

- `modded_refinery_placement_uses_free_unit_from_rules` to
  `modded_refinery_completion_uses_free_unit_from_rules`;
- `refinery_without_free_unit_spawns_nothing` to
  `refinery_without_free_unit_completion_spawns_nothing`.

Replace the modded test body with:

```rust
#[test]
fn modded_refinery_completion_uses_free_unit_from_rules() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=MODHARV\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=MODPROC\n\
         [GACNST]\n\
         Foundation=2x2\n\
         [MODPROC]\n\
         Refinery=yes\n\
         FreeUnit=MODHARV\n\
         Foundation=3x3\n\
         [MODHARV]\n\
         Harvester=yes\n\
         Dock=MODPROC\n\
         Speed=4\n",
    ))
    .expect("rules should parse");
    let mut sim = Simulation::new();
    let height_map = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 18, 18);
    let refinery = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "MODPROC",
        20,
        20,
        &grid,
        &height_map,
    );

    assert!(unit_ids(&sim, "Americans", "MODHARV").is_empty());
    set_ticks_until_completion(&mut sim, refinery, 1);
    let result =
        sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 33);
    assert!(result.spawned_entities);
    assert_eq!(unit_ids(&sim, "Americans", "MODHARV").len(), 1);
}
```

Replace the absent-key test body with:

```rust
#[test]
fn refinery_without_free_unit_completion_spawns_nothing() {
    let rules = RuleSet::from_ini(&IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=MODHARV\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         0=GACNST\n\
         1=MODPROC\n\
         [GACNST]\n\
         Foundation=2x2\n\
         [MODPROC]\n\
         Refinery=yes\n\
         Foundation=3x3\n\
         [MODHARV]\n\
         Harvester=yes\n\
         Dock=MODPROC\n\
         Speed=4\n",
    ))
    .expect("rules should parse");
    let mut sim = Simulation::new();
    let height_map = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    spawn_structure(&mut sim, 1, "Americans", "GACNST", 18, 18);
    let refinery = ready_and_place(
        &mut sim,
        &rules,
        "Americans",
        "MODPROC",
        20,
        20,
        &grid,
        &height_map,
    );

    set_ticks_until_completion(&mut sim, refinery, 1);
    let result =
        sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 33);
    assert!(!result.spawned_entities);
    assert!(unit_ids(&sim, "Americans", "MODHARV").is_empty());
}
```

These remain Rust regression tests; they do not broaden the stock plan into
native generic non-refinery mod parity.

**Step 9: Format the leaf test file**

Run:

```text
rustfmt --edition 2024 src/sim/production/production_placement_tests.rs
```

Do not format either `mod.rs`.

### Task 4: Run proportional validation

**Why:** Confirm the interface wiring, real completion path, stock data, and
deterministic order without consuming the project-wide full-suite budget more
than once.

**Files:** No source changes unless a failure points into a file modified by
Tasks 1-3. Do not fix unrelated dirty-worktree failures.

**Pattern:** Follow the `ENGINE.md` test tiers and literal-result reporting.

**Step 1: Re-check Cargo ownership**

Run:

```text
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU,StartTime
```

Expected: no process owned by another session. Wait if one exists.

**Step 2: Run the scoped production placement module**

Run:

```text
cargo test -p vera20k --lib sim::production::placement_tests:: -- --nocapture
```

Expected result line begins:

```text
test result: ok.
```

Record the complete literal result line, including passed/failed/ignored counts.

**Step 3: Run the pure helper module if the scoped filter did not include it**

Run:

```text
cargo test -p vera20k --lib sim::production::production_refinery::tests:: -- --nocapture
```

Expected result line begins:

```text
test result: ok. 2 passed; 0 failed
```

**Step 4: Review the behavioral fence**

Run:

```text
rg -n "when a refinery is placed|south of the foundation|0xC0 = .*south|maybe_spawn_refinery_harvester" src/sim/production
rg -n "Find_Nearby_Passable_Cell|fallback|refund|Commence|BuildingClass\\+0x300" docs/plans/2026-07-28-refinery-freeunit-completion-and-stock-primary-placement-{design,plan}.md
```

Expected:

- no stale placement-time or incorrect facing prose in the modified Rust owner;
- plan/design still name fallback, refund, Commence, and `+0x300` as residuals.

**Step 5: Run the full library suite exactly once at merge to `dev`**

The session that actually merges this slice to `dev` runs:

```text
cargo test -p vera20k --lib
```

Expected result line begins:

```text
test result: ok.
```

If this plan is executed on an isolated feature worktree, do not spend this
full-suite run before merge; hand the scoped literal outputs to the merge owner.
If execution occurs directly on sole-owner `dev`, this is the one allowed
merge-point full-suite run.

## Sources & References

- **Approved and amended design:**
  `docs/plans/2026-07-28-refinery-freeunit-completion-and-stock-primary-placement-design.md`
- **Implementation contract:**
  `docs/contracts/2026-07-28-refinery-freeunit-completion-implementation-contract.md`
- **Primary corrected Ghidra report:**
  `docs/research/miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`
- **Stock source/negative controls:**
  `docs/research/FIRST_ALLIED_MINER_SOURCE_GHIDRA_REPORT.md`;
  `docs/research/GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`
- **Static footprint integration:**
  `docs/research/STOCK_REFINERY_ART_REMOVE_OCCUPY_PAD_CELL_GHIDRA_REPORT.md`
- **Live gamemd.exe verification during planning:**
  - `decompile_function(0x00445F80)` — completion FreeUnit gate, primary,
    fallbacks, mission, cleanup/refund;
  - `decompile_function(0x00449A50)` — active construction completion caller;
  - `decompile_function(0x00447AC0)` — building center-coordinate formula.
- **Retail INI:**
  - `ini/rulesmd.ini:11722-11740` — GAREFN, `FreeUnit=CMIN`;
  - `ini/rulesmd.ini:12515-12534` — NAREFN, `FreeUnit=HARV`;
  - `ini/artmd.ini:1706-1716` — NAREFN `Foundation=4x3`;
  - `ini/artmd.ini:1763-1773` — GAREFN `Foundation=4x3`.
- **Current Rust anchors:**
  - `src/rules/object_type.rs:373,1031`;
  - `src/rules/ruleset.rs:2334-2354,2376+`;
  - `src/sim/production/production_placement.rs:164-253`;
  - `src/sim/production/production_refinery.rs:1-122`;
  - `src/sim/world/mod.rs:1802-1821,1932-1990`;
  - `src/sim/world/world_spawn.rs:260-430,594-615`;
  - `src/sim/production/production_tech.rs:688-830`;
  - `src/sim/production/production_placement_tests.rs:508-681`.
- **Relevant recent commits:**
  - `593b06ce` — original placement-time refinery harvester helper;
  - `12948d89` — occupied-building-foundation placement rejection;
  - `95bef99d` — ordered lifecycle authority;
  - `3cf53da6` — current Harvest mission authority.
