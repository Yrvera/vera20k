# Infantry-From-Barracks Exit Fix Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Stop the Rust port from teleporting barracks-produced infantry to an
`ExitCoord`-derived cell. Spawn them at the building's foundation-center cell
(matching gamemd's `GetCoord()` fall-through) so the existing pathfinder +
rally MoveTo walks them out through the foundation cells.

**Architecture:** Split the spawn-cell dispatch in
[src/sim/production/production_spawn.rs](src/sim/production/production_spawn.rs)
at `find_spawn_cell_for_owner` by `produced_category`. Infantry routes to a
new dedicated helper that returns the foundation-center cell unconditionally
(no `ExitCoord`, no passability check, no fallback). Vehicle/aircraft path
stays on the existing `find_spawn_cell_near_structure` (known parity-wrong,
scope-deferred).

**Design Doc:** [docs/plans/2026-05-17-infantry-from-barracks-exit-fix-design.md](docs/plans/2026-05-17-infantry-from-barracks-exit-fix-design.md)

---

## Grounding Summary

- **Docs:** RALLY_POINTS_AND_UNIT_SPAWNING.md (GREEN, 2026-05-16) §6 covers
  the infantry alt-path Unlimbo-at-`GetCoord()`; §7/§7a cover sub-cell tables
  and GetExitCoord (NOT used for infantry). INFANTRY_SUBCELL_POSITIONING.md
  (GREEN, 2026-05-17, 15/15 claims confirmed) covers PlaceInfantryInCell /
  WalkLoco integration. INFANTRYCLASS_GHIDRA_REPORT.md (GREEN, 2026-05-17)
  confirms constructor SubCell=2 at +0x6E8.
- **Ghidra verified:** ExitObject_Main alt path at `0x443F54` → GetDockCoord
  (`0x00447B20`) → fall-through `FUN_005F6C80` → `building->GetCoord()` =
  building center lepton at `+0x9C..+0xA4`. ExitCoord
  (`BuildingTypeClass+0xEC8/ECC/ED0`) never read for infantry. RallyTarget
  (`building+0x86`) gated branch issues `MoveTo(rally, 1)` + `SetMission(MOVE = 2)`.
- **Repo pattern:** [find_helipad_for_aircraft in production_spawn.rs:386-425](src/sim/production/production_spawn.rs#L386-L425)
  computes the building's center cell as `(entity.position.rx + fw/2,
  entity.position.ry + fh/2)`. The new infantry helper mirrors this idiom.
- **INI:** GAPILE 3x2 ExitCoord=-64,64,0 GDIBarracks=yes; NAHAND 2x2
  ExitCoord=0,0,0 NODBarracks=yes; YABRCK 2x3 ExitCoord=-64,64,0
  YuriBarracks=yes — confirmed in [ini/rulesmd.ini](ini/rulesmd.ini) and
  [ini/artmd.ini](ini/artmd.ini). All three barracks set ExitCoord; the
  Rust port's `lepton_to_cell` rounds all of these to `(0, 0)`, so the
  bug currently fires for every YR barracks unit.
- **Unknowns after grounding:** the exact facing arg passed by ExitObject's
  alt path to Unlimbo (ledger item #13) — deferred follow-up per design,
  not in scope here.

## Key Technical Decisions

- **Foundation-center cell formula** `(rx + foundation_w / 2, ry + foundation_h / 2)`
  — **Confidence:** high. **Source:** RALLY_POINTS §6 step 4 + INFANTRY_SUBCELL
  GREEN audit + repo pattern at [production_spawn.rs:418-422](src/sim/production/production_spawn.rs#L418-L422).
- **No `cell_available_for_spawn` check at infantry spawn step** — **Confidence:** high.
  **Source:** RALLY_POINTS §6 step 4 (infantry occupy own-building foundation cells
  in gamemd); INFANTRY_SUBCELL §"Occupancy Byte Bit Field" (0x40 building bit triggers
  garrison check, not a hard block, and producer-building's own cell is the canonical
  spawn site).
- **No `nearest_walkable_around` fallback for infantry** — **Confidence:** high.
  **Source:** RALLY_POINTS §6 alt path has no fallback; gamemd's `Unlimbo` returns
  failure (no fallback search) if its own CanEnterCell rejects.
- **No-rally case stays put** — **Confidence:** high. **Source:** RALLY_POINTS §6
  step 4 last paragraph; absence of else branch in alt path. User confirmed in
  brainstorm.
- **Keep `facing = 64` at the spawn_object call site** — **Confidence:** low.
  **Source:** current Rust code [production_queue.rs:521](src/sim/production/production_queue.rs#L521).
  Gamemd's facing arg was not RE'd. Flagged as deferred follow-up #4 in design doc.
  **/review-plan should flag this.**

## Open Questions

### Resolved During Planning

- **Are there existing tests that assert a specific spawn cell for infantry?**
  No. Grep confirmed `find_spawn_cell_for_owner` is called for `ObjectCategory::Infantry`
  only in two locations (`is_matching_factory` tests at production_tests.rs:631/642),
  neither of which asserts a spawn position. Adding new tests is purely additive.
- **Does adding `Foundation=3x2` to GAPILE in `factory_rules()` break any existing
  test?** Searched all GAPILE usages in production_tests.rs and production_queue_tests.rs.
  No test relies on GAPILE defaulting to 1x1 foundation, and no test asserts an
  infantry-spawn cell position. Safe to add.

### Deferred to Implementation

- None. All open design questions were resolved in the brainstorm.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/production/production_spawn.rs` | Add `find_infantry_spawn_cell_near_structure`; branch `find_spawn_cell_for_owner` by category |
| Modify | `src/sim/production/production_tests.rs` | Add `Foundation=` to GAPILE+MYBARR fixtures; add 4 new infantry-spawn unit tests |

No other files touched. `production_queue.rs`, `world_spawn.rs`, and `bump_crush.rs`
are read-only references.

## Interface Changes

- `find_spawn_cell_for_owner` — **signature unchanged.** Behavior changes only
  for `produced_category = ObjectCategory::Infantry`: returns the producing
  barracks's foundation-center cell instead of ExitCoord-derived cell.
- `find_infantry_spawn_cell_near_structure` — **new module-private function.**
  No external consumers.
- `preferred_exit_offsets` — **unchanged.** Still called for Vehicle/Aircraft.
  Still parity-wrong for those categories (scope-deferred).

## Sim Checklist

- [x] All math uses integer cell coords (`u16`) — no f32/f64 in game logic.
- [x] No new state added; deterministic state hash unaffected.
- [x] No dependencies on render/ui/sidebar/audio/net — change is isolated to
  `sim/production/`.
- [x] Tick ordering: unchanged. Production-spawn slot in `World::advance_tick`
  is unchanged.
- [x] BTreeMap iteration order: `producer_candidates_for_owner_category`
  iteration is unchanged; we only change what happens after a candidate is
  chosen.

## Risk Areas

- **Regression risk for vehicle/aircraft path:** zero — the new code adds a
  branch BEFORE the existing call to `find_spawn_cell_near_structure`. The
  existing path is structurally untouched.
- **Foundation occupancy collision:** new infantry-spawn cell falls INSIDE the
  producer building's foundation. Today that cell would be rejected by
  `cell_available_for_spawn`'s building-bit check; the new path bypasses
  that check. Mitigated by Task 6 regression test that explicitly proves
  the spawn succeeds with foundation occupancy present.
- **Test fixture mutation:** adding `Foundation=3x2` to GAPILE and
  `Foundation=2x2` to MYBARR in `factory_rules()` affects all tests that
  use that fixture. Verified by grep that no test depends on GAPILE/MYBARR
  defaulting to 1x1. Mitigated by Task 7's full `cargo test` run.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Infantry spawns at foundation-center cell of producing barracks | Player sees infantry emerge from the building's footprint, then walk out via pathfinding (gamemd's "exits through the door" visual). Fires every time the player builds an infantry unit. | RALLY_POINTS §6 alt path at `0x443F54`; Task 4 unit test |
| Task 1 | `ExitCoord` is NOT read for infantry spawn | Per RALLY_POINTS §6 "What does NOT happen". Fires every time a barracks with ExitCoord (= all YR barracks) produces infantry. | Ghidra trace of `ExitObject_Main` alt path; Task 5 unit test (MYBARR ExitCoord=-64,64,0 → still center cell) |
| Task 1 | No `nearest_walkable_around` fallback for infantry | gamemd's alt path has no fallback; if Unlimbo fails, the unit is refunded. Player observes consistent spawn behavior — no "infantry teleported 12 cells away" surprise when the foundation is unusually placed. | Task 6 unit test (succeeds with foundation occupancy present) |
| Task 2 | No-rally infantry stays at spawn cell | gamemd issues no MoveTo when rally is unset (RALLY_POINTS §6 step 4, absence of else). Player sees infantry stand at the building until ordered. User-confirmed in brainstorm. | Existing `production_queue.rs:552-587` rally-MoveTo guard, unchanged. No new test needed (existing rally-set test path exercises the positive case). |

## Deferred Follow-Ups (NOT in this plan)

Carried from the design doc — these are tracked separately and must NOT be
silently included in this implementation:

1. Vehicle/aircraft `ExitCoord` semantics rewrite (`GetDockCellForObject` +
   barracks-flag-gated lepton add).
2. Walk-loco `PlaceInfantryInCell` port (preference table + random rotation
   + Mark/Unmark virtual call sequence).
3. Amphibious infantry exit gate (`Type+0xE0D`).
4. Initial body facing RE for ExitObject alt path.

---

## Tasks

### Task 1: Add `find_infantry_spawn_cell_near_structure` helper

**Why:** Introduces the new gamemd-faithful infantry-spawn-cell helper.
Self-contained — only defines the function; no caller change yet so the
existing test suite still passes after this task.

**Files:**
- Modify: `src/sim/production/production_spawn.rs` (add new function)

**Pattern:** Mirrors [find_helipad_for_aircraft in production_spawn.rs:386-425](src/sim/production/production_spawn.rs#L386-L425),
which computes a building center cell from `entity.position.rx + fw/2,
entity.position.ry + fh/2`. The new function is simpler (one cell, no dock
slot scan).

**Step 1: Add the helper function**

Insert this function in `production_spawn.rs` immediately after
`find_spawn_cell_near_structure` (i.e., after line 164):

```rust
/// Infantry-specific spawn cell: the foundation-center cell of the producing
/// barracks. Matches the original engine's alt-path Unlimbo at the building's
/// center lepton coord; `ExitCoord` is intentionally ignored, no passability
/// check is performed, and there is no fallback to a nearby cell — if the
/// foundation has no recognizable size the caller refunds the unit.
///
/// The infantry then walks out of the foundation via the existing pathfinder
/// once the rally MoveTo is issued; the foundation cells are passable to
/// infantry (only vehicles are hard-blocked).
fn find_infantry_spawn_cell_near_structure(
    rules: &RuleSet,
    base_rx: u16,
    base_ry: u16,
    structure_id: &str,
) -> Option<(u16, u16)> {
    let obj = rules.object(structure_id)?;
    let (w, h) = super::production_tech::foundation_dimensions(&obj.foundation);
    Some((base_rx.saturating_add(w / 2), base_ry.saturating_add(h / 2)))
}
```

**Step 2: Verify it builds**

Run: `cargo check -p ra2_rust_game`
Expected: clean build (function is currently unused — `#[allow(dead_code)]`
is NOT needed because Task 2 will wire it up before the next commit).

Note: do NOT commit yet. Task 2 wires this function up and commits both
changes atomically.

---

### Task 2: Dispatch infantry category through the new helper

**Why:** Wires `find_infantry_spawn_cell_near_structure` into
`find_spawn_cell_for_owner` so infantry production actually uses the new
path. Vehicle/aircraft path is preserved unchanged.

**Files:**
- Modify: `src/sim/production/production_spawn.rs:77-91` (the
  `for (_sid, bx, by, structure_id) in bases` loop)

**Pattern:** Existing module-internal dispatch idiom — match on
`ObjectCategory` and route to the appropriate helper.

**Step 1: Replace the loop body**

Find the existing loop in `find_spawn_cell_for_owner` at lines 77-91:

```rust
    let resolved_terrain = sim.resolved_terrain.as_ref();
    for (_sid, bx, by, structure_id) in bases {
        if let Some(cell) = find_spawn_cell_near_structure(
            *bx,
            *by,
            structure_id,
            produced_category,
            rules,
            path_grid,
            &sim.occupancy,
            resolved_terrain,
            require_water,
        ) {
            return Some(cell);
        }
    }
    None
```

Replace it with:

```rust
    let resolved_terrain = sim.resolved_terrain.as_ref();
    for (_sid, bx, by, structure_id) in bases {
        let cell = match produced_category {
            ObjectCategory::Infantry => {
                find_infantry_spawn_cell_near_structure(rules, *bx, *by, structure_id)
            }
            _ => find_spawn_cell_near_structure(
                *bx,
                *by,
                structure_id,
                produced_category,
                rules,
                path_grid,
                &sim.occupancy,
                resolved_terrain,
                require_water,
            ),
        };
        if let Some(cell) = cell {
            return Some(cell);
        }
    }
    None
```

**Step 2: Verify it builds**

Run: `cargo check -p ra2_rust_game`
Expected: clean build, no warnings about unused parameters (`path_grid`,
`require_water` etc. are still used by the non-Infantry branch).

**Step 3: Run the existing production test suite**

Run: `cargo test -p ra2_rust_game --lib sim::production`
Expected: all tests pass. The new branch only triggers for
`ObjectCategory::Infantry`, and no existing test asserts an infantry
spawn-cell position, so the suite must stay green.

**Step 4: Commit**

```
sim/production: route infantry spawn to building-center cell

ExitCoord is a vehicle/aircraft concept in gamemd; the alt path for
infantry Unlimbos at building->GetCoord() = the foundation center
lepton, then issues MoveTo(rally) so pathfinding walks the unit out
through the foundation cells. Add find_infantry_spawn_cell_near_structure
and dispatch by ObjectCategory in find_spawn_cell_for_owner.

The vehicle/aircraft branch is unchanged (still parity-wrong on
ExitCoord semantics, scope-deferred).
```

Verify with `git status` and `git log -1` after commit.

---

### Task 3: Add `Foundation=` to GAPILE and MYBARR in `factory_rules()`

**Why:** The new infantry-spawn tests need realistic foundation sizes so
the foundation-center cell math is meaningful. GAPILE in the real INI is
3x2; MYBARR is a modded fixture, so we add 2x2 (matches NAHAND-style even
dim) for the ExitCoord-ignored test case.

**Files:**
- Modify: `src/sim/production/production_tests.rs:436-454` (the `factory_rules`
  fixture's ini string)

**Pattern:** Existing test fixture style — INI key=value lines inside the
multi-line ini string.

**Step 1: Add Foundation lines**

Find the `[GAPILE]` and `[MYBARR]` blocks in `factory_rules()` at lines
438-439 and 452-454, and add `Foundation=` keys:

For `[GAPILE]` (currently lines 438-439):
```rust
             [GAPILE]\n\
             Factory=InfantryType\n\
```

Change to:
```rust
             [GAPILE]\n\
             Factory=InfantryType\n\
             Foundation=3x2\n\
```

For `[MYBARR]` (currently lines 452-454):
```rust
             [MYBARR]\n\
             Factory=InfantryType\n\
             ExitCoord=-64,64,0\n\
```

Change to:
```rust
             [MYBARR]\n\
             Factory=InfantryType\n\
             ExitCoord=-64,64,0\n\
             Foundation=2x2\n\
```

**Step 2: Verify the existing suite still passes**

Run: `cargo test -p ra2_rust_game --lib sim::production`
Expected: all tests pass. No existing test asserts GAPILE or MYBARR's
foundation size (grep confirmed in grounding).

No commit yet — the next task adds the tests that use these foundations.

---

### Task 4: Add `infantry_spawn_uses_foundation_center_cell` test

**Why:** Locks in the new behavior: GAPILE (3x2, odd width × even height)
at (20, 20) produces infantry at foundation-center cell (21, 21).

**Files:**
- Modify: `src/sim/production/production_tests.rs` (add new test function;
  insert after `exit_coord_parsed_and_used_for_spawn` at line 689)

**Pattern:** Existing `exit_coord_parsed_and_used_for_spawn` test structure
— set up Simulation, spawn_structure, call `find_spawn_cell_for_owner`,
assert the returned cell.

**Step 1: Add the test**

Insert this `#[test]` function after the `exit_coord_parsed_and_used_for_spawn`
function:

```rust
#[test]
fn infantry_spawn_uses_foundation_center_cell() {
    let rules = factory_rules();
    // GAPILE has Foundation=3x2 in the fixture, no ExitCoord.
    // Foundation-center cell of a building at (20, 20) is (20 + 3/2, 20 + 2/2)
    // = (21, 21) — the cell inside the foundation that gamemd's
    // building->GetCoord() lepton lands in.
    let mut sim = Simulation::new();
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 20, 20);
    let spawn = find_spawn_cell_for_owner(
        &mut sim,
        &rules,
        "Americans",
        ObjectCategory::Infantry,
        None,
        false,
    )
    .expect("infantry spawn from GAPILE should succeed");
    assert_eq!(
        spawn,
        (21, 21),
        "infantry spawns at foundation-center cell of 3x2 GAPILE at (20, 20)"
    );
}
```

**Step 2: Run the test**

Run: `cargo test -p ra2_rust_game --lib infantry_spawn_uses_foundation_center_cell -- --nocapture`
Expected: PASS.

No commit yet — Task 5 and Task 6 add more tests against the same module.

---

### Task 5: Add `infantry_spawn_ignores_exit_coord` test

**Why:** Proves ledger item #2 (`ExitCoord` is never read for infantry,
even when the INI sets it). Uses MYBARR which has `ExitCoord=-64,64,0`
— before this fix, that ExitCoord would have produced cell offset (0, 0)
and the spawn cell would have been the anchor (10, 10). After fix: still
foundation center (11, 11).

**Files:**
- Modify: `src/sim/production/production_tests.rs` (add new test after Task 4's
  test)

**Pattern:** Same as Task 4.

**Step 1: Add the test**

Insert this `#[test]` function after `infantry_spawn_uses_foundation_center_cell`:

```rust
#[test]
fn infantry_spawn_ignores_exit_coord() {
    let rules = factory_rules();
    // MYBARR has ExitCoord=-64,64,0 AND Foundation=2x2.
    // gamemd's infantry alt path NEVER reads ExitCoord; the unit Unlimbos at
    // building->GetCoord() = foundation center. For a 2x2 barracks at (10, 10)
    // that's (10 + 2/2, 10 + 2/2) = (11, 11). The (-64, 64) ExitCoord must
    // have zero effect.
    let mut sim = Simulation::new();
    spawn_structure(&mut sim, 1, "Americans", "MYBARR", 10, 10);
    let spawn = find_spawn_cell_for_owner(
        &mut sim,
        &rules,
        "Americans",
        ObjectCategory::Infantry,
        None,
        false,
    )
    .expect("infantry spawn from MYBARR should succeed");
    assert_eq!(
        spawn,
        (11, 11),
        "infantry spawn ignores ExitCoord=-64,64,0; uses foundation-center cell"
    );
}
```

**Step 2: Run the test**

Run: `cargo test -p ra2_rust_game --lib infantry_spawn_ignores_exit_coord -- --nocapture`
Expected: PASS.

No commit yet — Task 6 adds the foundation-occupancy regression test.

---

### Task 6: Add `infantry_spawn_succeeds_when_center_cell_blocked` test

**Why:** Proves ledger item #14 — the new infantry path bypasses
`cell_available_for_spawn` so the spawn succeeds even when the foundation
center cell is occupied (which it always is — the producer building owns
it). Distinguishes this path from the vehicle/aircraft path which DOES
honor that check.

**Files:**
- Modify: `src/sim/production/production_tests.rs` (add new test after Task 5)

**Pattern:** Same as Task 4 — plus a deliberate occupancy entry at the
spawn cell.

**Step 1: Add the test**

Insert this `#[test]` function after `infantry_spawn_ignores_exit_coord`:

```rust
#[test]
fn infantry_spawn_succeeds_when_center_cell_blocked() {
    let rules = factory_rules();
    // The producing GAPILE itself occupies (20, 20) via spawn_structure's
    // single-cell registration. The new foundation-center cell (21, 21) is
    // inside the building's footprint. gamemd's infantry alt path performs
    // no passability check at the spawn step — only vehicles are
    // hard-blocked by building cells. Infantry succeed.
    let mut sim = Simulation::new();
    spawn_structure(&mut sim, 1, "Americans", "GAPILE", 20, 20);
    // Also occupy the foundation-center cell explicitly to make the test
    // robust against future changes to spawn_structure's occupancy footprint.
    sim.occupancy.add(
        21,
        21,
        1,
        crate::sim::movement::locomotor::MovementLayer::Ground,
        None,
        crate::sim::occupancy::CellListInsertion::AppendBuilding,
    );
    let spawn = find_spawn_cell_for_owner(
        &mut sim,
        &rules,
        "Americans",
        ObjectCategory::Infantry,
        None,
        false,
    )
    .expect("infantry spawn should succeed even with building bit on center cell");
    assert_eq!(
        spawn,
        (21, 21),
        "infantry spawn ignores foundation occupancy and lands at center cell"
    );
}
```

**Step 2: Run all three new tests together**

Run: `cargo test -p ra2_rust_game --lib infantry_spawn -- --nocapture`
Expected: all 3 tests PASS.

No commit yet — Task 7 runs the full module test before committing.

---

### Task 7: Run full production module test + commit tests

**Why:** Final regression check before committing the test changes. Confirms
the new fixture additions (Foundation= keys) and the new tests don't break
anything in `production_queue_tests` or `production_placement_tests`.

**Files:**
- None (verification + commit only)

**Step 1: Run the full production-module test suite**

Run: `cargo test -p ra2_rust_game --lib sim::production`
Expected: all tests PASS, including the new infantry tests, the existing
ExitCoord parser tests (vehicle path unchanged), and all queue / placement
tests.

If any test fails:
- If it's an existing test that depends on GAPILE being 1x1 foundation,
  revisit Task 3 — that should not happen per grounding, but if it does,
  add a new infantry-specific fixture instead of mutating `factory_rules`.
- If it's the new infantry test, re-read the diff against the design doc
  before patching the assertion.

**Step 2: Commit**

```
sim/production: cover infantry foundation-center spawn

Add Foundation=3x2 to GAPILE and Foundation=2x2 to MYBARR in
factory_rules so the foundation-center math has a meaningful target.

New tests:
- infantry_spawn_uses_foundation_center_cell (GAPILE 3x2 odd-x-even)
- infantry_spawn_ignores_exit_coord (MYBARR ExitCoord=-64,64,0 ignored)
- infantry_spawn_succeeds_when_center_cell_blocked (no passability check
  at spawn step; matches gamemd's CanEnterCell behavior for infantry on
  own-building foundation cells)
```

Verify with `git status` and `git log -1` after commit.

---

### Task 8: End-to-end verification against gamemd.exe

**Why:** Confirm the implementation matches the original engine's
observable behavior in a real skirmish, not just unit tests.

**Files:**
- None (manual verification)

**Verify:**
- Launch the Rust client (`cargo run`), start a skirmish on any map with
  an Allied/Soviet/Yuri start.
- Build a barracks, build a GI (or Conscript, Initiate).
- **Expected (matches gamemd):** the infantry sprite appears inside or
  at the edge of the barracks footprint, then walks out through the
  foundation cells toward the rally point (if set) or stands at the
  spawn cell (if no rally).
- **Regression watch:** previously, the infantry would teleport to the
  barracks' anchor cell or to a cell scanned outward by
  `nearest_walkable_around`. Confirm that is no longer the case.

Cross-check the binary behavior (Ghidra MCP):
- Decompile `BuildingClass::ExitObject_Main` at `0x00443C60` and find the
  alt path at `0x443F54`. Confirm the call chain is `GetDockCoord` →
  fall-through `FUN_005F6C80` → returns `GetCoord()` = building+0x9C..0xA4.
  This is RALLY_POINTS §6 step 4 verbatim — the implementation must produce
  observably equivalent output.

If a difference is observed, file it under the design's "Deferred Follow-Ups"
section and bring it back to `/brainstorm` before patching.

**No commit needed.** This task is verification, not modification.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-17-infantry-from-barracks-exit-fix-design.md](docs/plans/2026-05-17-infantry-from-barracks-exit-fix-design.md)
- **Ghidra reports:**
  - `ra2-rust-game-docs/RALLY_POINTS_AND_UNIT_SPAWNING.md` §6 (alt path),
    §7 (sub-cell tables), §7a (GetExitCoord — *not* used for infantry),
    §18 (Unlimbo chain) — GREEN-audited 2026-05-16
  - `ra2-rust-game-docs/INFANTRY_SUBCELL_POSITIONING.md` §"Placement Function"
    + §"Walk Locomotor Integration" + §"Occupancy Byte Bit Field" — GREEN-audited
    2026-05-17 (15/15 claims confirmed)
  - `ra2-rust-game-docs/INFANTRYCLASS_GHIDRA_REPORT.md` §2 constructor SubCell=2
    — GREEN-audited 2026-05-17
- **gamemd.exe addresses (kept here, NOT in Rust code comments per CLAUDE.md):**
  - `BuildingClass::ExitObject_Main` @ `0x00443C60` — main spawn dispatch
  - Alt path entry @ `0x443F54` — infantry-from-barracks branch
  - `BuildingClass::GetDockCoord` @ `0x00447B20` (vtable +0xA8)
  - Fall-through `FUN_005F6C80` — returns `building->GetCoord()`
  - `BuildingClass::GetExitCoord` @ `0x0044F640` (vtable +0xB4) — *not* used for infantry
  - `InfantryClass` constructor @ `0x00517A50` — sets `param_1[0x1BA]=2` (SubCell field at +0x6E8)
  - `InfantryClass` vtable @ `0x007EB058` — Mark @ slot 60 (`0x005217C0`), Unmark @ slot 61 (`0x00521850`)
- **INI keys:**
  - `rulesmd.ini [GAPILE]` ExitCoord=-64,64,0, GDIBarracks=yes
  - `rulesmd.ini [NAHAND]` ExitCoord=0,0,0, NODBarracks=yes
  - `rulesmd.ini [YABRCK]` ExitCoord=-64,64,0, YuriBarracks=yes
  - `artmd.ini [GAPILE]` Foundation=3x2
  - `artmd.ini [NAHAND]` Foundation=2x2
  - `artmd.ini [YABRCK]` Foundation=2x3
- **Related code:**
  - [src/sim/production/production_spawn.rs:300-314](src/sim/production/production_spawn.rs#L300-L314) — current `preferred_exit_offsets` (vehicle/aircraft scope-deferred)
  - [src/sim/production/production_spawn.rs:386-425](src/sim/production/production_spawn.rs#L386-L425) — `find_helipad_for_aircraft` (pattern to mirror)
  - [src/sim/production/production_tech.rs:562-573](src/sim/production/production_tech.rs#L562-L573) — `foundation_dimensions`
  - [src/sim/production/production_queue.rs:506-595](src/sim/production/production_queue.rs#L506-L595) — spawn pipeline that calls `find_spawn_cell_for_owner` and issues rally MoveTo
  - [src/sim/world/world_spawn.rs:280-357](src/sim/world/world_spawn.rs#L280-L357) — `spawn_object` infantry block (sub_cell + sub-x/sub-y assignment, unchanged)
