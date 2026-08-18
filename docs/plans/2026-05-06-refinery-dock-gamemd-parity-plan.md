# Refinery Dock — gamemd Parity Rewrite Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Bring the harvester refinery dock state machine to ≥95% parity with gamemd.exe `UnitClass::Mission_Deploy_Building` (0x73D630) — collapse 7 phases to 4, fix exit cell, per-bale purifier, slot 7/10/8 + particle wiring, AddOccupy/RemoveOccupy parsing.

**Architecture:** Sim-only state machine + new bale event queue feeding the renderer. Builds on existing `tick_miners` two-phase snapshot pattern, existing `sound_events` queue pattern, and existing INI merge through `ruleset.rs::merge_art_data`. No new module boundaries.

**Design Doc:** [docs/plans/2026-05-06-refinery-dock-gamemd-parity-design.md](2026-05-06-refinery-dock-gamemd-parity-design.md)

---

## Revision history

- **2026-05-06 v1**: initial plan (25 tasks)
- **2026-05-06 v2**: post-`/review-plan` corrections (this version, 21 tasks):
  - Storage-tier ActiveAnim selection deferred to a separate follow-up PR (depends on per-refinery ore storage state we don't have yet — out of scope for the dock state machine work)
  - Dropped Tasks 1 (storage-tier formula spot-check) and 2 (Type+0x16A8 identity) — neither blocks the remaining work
  - Dropped Tasks 7-8 (storage_tier field + tick computation) — same reason
  - Rewrote Task 6 to modify `PathGrid::block_building_footprint` directly (via concrete callsite list) and also handle the OccupancyGrid stamping in `world_spawn.rs`
  - Fixed Task 4 test (use `ArtRegistry::from_ini` wrapper, not raw `IniFile`)
  - Fixed Task 19 field shapes (`refinery_smoke_offsets: [IVec3; 4]` array, not 4 separate Options) and added parsing for `RefinerySmokeFrames`
  - Updated File Map: `Simulation` lives in `src/sim/world/mod.rs`, not `world_orders.rs`
  - Renamed `BuildingAnim` → `BuildingAnimConfig` references
  - Tasks 6-11 (FSM rewrite) marked as a single atomic commit unit (build broken between them; no per-task commits)

---

## Grounding Summary

- **Docs:** [REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md) (this session) verifies slot 7/10/8 mapping in `BuildingTypeClass+0xF4C` table. [BUILDING_ANIM_STATE_MACHINE.md](../../../ra2-rust-game-docs/BUILDING_ANIM_STATE_MACHINE.md) covers the 21-slot system. [BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md](../../../ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md) Part 2 is INCORRECT (wrong target function); rewriting it is Task 21.
- **Ghidra confirmed:** Mission_Deploy_Building at 0x73D630 has 4 cases (0/1/3/4); per-bale gate is `HarvesterDumpRate × 900.0` frames; SpecialAnim (slot 10) plays per bale; 4 particles spawn per bale via vtable+0x468 → `FUN_00459900` at offsets `Type+0x7CC/0x7D8/0x7E4/0x7F0`; UndockUnit (0x4593A0) sets exit facing 0x47 and offset `(-0x80, +0x80)` from origin; single-slot dock at `BuildingClass+0x2E4`.
- **Repo pattern:** sim → render event queues already exist via `Simulation::sound_events: Vec<SimSoundEvent>` and `Simulation::fire_events: Vec<SimFireEvent>` (both `#[serde(skip)]`). New `bale_events: Vec<BaleDepositEvent>` mirrors this exactly. Two-phase snapshot pattern in `tick_miners` preserved.
- **INI keys driving:** `[GAREFN]` rules: `DockUnload`, `Refinery`, `Storage=200`, `RefinerySmokeOffsetOne/Two/Three/Four`, `RefinerySmokeFrames=50`, `RefinerySmokeParticleSystem=SmallGreySSys`. `[GAREFN]` art: `Foundation=4x3`, `QueueingCell=4,1`, `AddOccupy1=-1,0`, `AddOccupy2=-1,-1`, `RemoveOccupy1=3,1`, `ActiveAnim/Two/Three/Four=GAREFNL1..L4`, `SpecialAnim=GAREFNOR`. `[General]`: `HarvesterDumpRate` (defaulted to 0.016 in ruleset.rs when absent), `PurifierBonus=.25`, `ConditionYellow=50%`.
- **Unknown after grounding:** none blocking. The deferred storage-tier work has its own follow-up; this plan can land independently.

---

## Key Technical Decisions

- **Collapse `RefineryDockPhase` 7→4 variants (Approach/Linked/Unloading/Departing)** — matches gamemd's actual FSM shape. **Confidence:** high. **Source:** Ghidra 0x73D630
- **Keep deterministic FIFO `DockReservations`** — gamemd has no queue; we replace with FIFO as scale-exception for 30-player target. **Confidence:** high. **Source:** project memory `project_scale_target.md`, CLAUDE.md scale-exception clause
- **Per-bale Ore Purifier bonus inline** (replace existing end-of-load batching) — matches gamemd. **Confidence:** high. **Source:** Ghidra 0x73D630 state 3
- **Exit cell = building_origin + (-0x80, +0x80) leptons** (replace foundation-derived formula). **Confidence:** high. **Source:** Ghidra UndockUnit 0x4593A0
- **Bale events emitted via new `bale_events` queue** (replace `dock_active_anim: bool`). **Confidence:** high. **Source:** repo pattern (existing `sound_events` and `fire_events` at `src/sim/world/mod.rs:208,212`)
- **Parse `AddOccupy*` and `RemoveOccupy*`** to compute correct footprint. **Confidence:** high. **Source:** ini/artmd.ini GAREFN
- **Drop `bypass_grid=true` workaround** once correct footprint stamps both grids. **Confidence:** high. **Source:** ini + Ghidra
- **`MinerState::Unload` enum variant + `handle_unload` function are dead code** — production paths run through `Dock` only. **Confidence:** high (verified via grep). **Source:** [src/sim/miner/miner_system.rs:188](../../src/sim/miner/miner_system.rs#L188), [src/sim/ai.rs:1071](../../src/sim/ai.rs#L1071)

---

## Open Questions

### Resolved during planning

- **Q: Does `BuildingClass::Update` independently fire slot 10 for refineries?** → No. Unit FSM in Mission_Deploy_Building is sole driver.
- **Q: Is `[ESI+0xBC]=3` a building or unit field write?** → Unit. ESI is `param_1` (the unit) throughout.
- **Q: Where is `unit+0x3E` incremented?** → Doesn't matter for us — our `unload_timer` produces the correct cadence.

### Deferred to a follow-up PR

- **Storage-tier ActiveAnim display** (slots 3-6 selecting one of `GAREFNL1/L2/L3/L4` based on stored ore level): requires per-refinery ore storage state that doesn't exist in our engine today (bales currently convert directly to player credits in [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) `phase_unloading`). Adding per-refinery storage is a separate scope. Until then, this PR loops ALL four ActiveAnim variants continuously (which is wrong but no worse than current behavior — current behavior gates them on `dock_active_anim` which is also wrong).
- **Storage-tier formula spot-check** (Ghidra 0x4509D0 sites 0x450E0D, 0x450F99) — only relevant when the per-refinery storage feature lands.
- **`Type+0x16A8` identity (HasStorage vs HasTurretAnim)** — gates whether slot 10 fires unsuppressed. Visible behavior implies refineries don't set it (slot 10 fires); not blocking this PR.

### Deferred to implementation (real unknowns surfaced during execution)

- **Q3: Should we replicate gamemd's first-bale jitter (`Random(0,2) × 30` frames seeded by Unlimbo)?** Probably no — minor cosmetic drift, hard to verify visibly. Decide during Task 10 review.
- **Q4: `bypass_grid` removal safety** — the actual blast radius of changing path-grid stamping is unknown until Task 4 lands. Keep `bypass_grid=true` through Tasks 8-11, then remove in Task 20 once integration tests pass.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs) | Collapse `RefineryDockPhase` enum 7→4 |
| Modify | [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) | Full rewrite: 4 phase handlers; new exit cell; emit bale events |
| Modify | [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs) | Drop dead `MinerState::Unload` dispatch arm |
| Modify | [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) | Mechanical rename, drop fabricated-phase tests, add new tests |
| Modify | [src/sim/components.rs](../../src/sim/components.rs) | Define `BaleDepositEvent` |
| Modify | [src/sim/game_entity.rs](../../src/sim/game_entity.rs) | Drop `dock_active_anim` field |
| Modify | [src/sim/world/mod.rs](../../src/sim/world/mod.rs) | Add `bale_events: Vec<BaleDepositEvent>` to `Simulation` (next to `sound_events` at line 208 and `fire_events` at line 212) |
| Modify | [src/sim/ai.rs](../../src/sim/ai.rs) at line 1071 | Drop `MinerState::Unload` arm in test scaffolding |
| Modify | [src/rules/art_data.rs](../../src/rules/art_data.rs) | Parse `AddOccupy1..N`, `RemoveOccupy1..N` |
| Modify | [src/rules/object_type.rs](../../src/rules/object_type.rs) | Add `add_occupy`, `remove_occupy`, `refinery_smoke_frames` fields |
| Modify | [src/rules/ruleset.rs](../../src/rules/ruleset.rs) | Merge add/remove from art entry into ObjectType |
| Modify | [src/sim/production/production_tech.rs](../../src/sim/production/production_tech.rs) | New `building_footprint_cells()` helper |
| Modify | [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) at line 1041 | Extend `PathGrid::block_building_footprint` to take add/remove occupy slices |
| Modify | [src/app_init.rs](../../src/app_init.rs) at line 622 | Update `block_building_footprint` call to pass add/remove occupy |
| Modify | [src/app_sim_tick.rs](../../src/app_sim_tick.rs) at lines 729 + 740 | Same call-site update |
| Modify | [src/sim/world/world_spawn.rs](../../src/sim/world/world_spawn.rs) at lines 240-261 | OccupancyGrid stamping uses `building_footprint_cells` |
| Modify | [src/app_instances/shp.rs](../../src/app_instances/shp.rs) | Drop `dock_active_anim` parameter; consume bale events for SpecialAnim + particles |
| Modify | [ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md](../../../ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md) | Rewrite Part 2 (currently wrong target function) |

---

## Interface Changes

**Added (public):**
- `RefineryDockPhase::Linked`, `RefineryDockPhase::Departing` (new variants)
- `BaleDepositEvent { building_id: u64, tick: u64 }` (new struct in `sim/components.rs`)
- `Simulation::bale_events: Vec<BaleDepositEvent>` (new field, `#[serde(skip)]`)
- `ObjectType::add_occupy: Vec<(i16, i16)>`, `ObjectType::remove_occupy: Vec<(i16, i16)>`
- `ObjectType::refinery_smoke_frames: u16` (new field, defaults to 0 if INI key absent)
- `building_footprint_cells(origin_rx, origin_ry, foundation, add_occupy, remove_occupy) -> Vec<(u16, u16)>` (new helper)
- `PathGrid::block_building_footprint` signature extended with `add_occupy: &[(i16, i16)], remove_occupy: &[(i16, i16)]`

**Removed:**
- `RefineryDockPhase::{WaitForDock, RotateToPad, EnterPad, TurnOnPad, ExitPad}` (5 variants)
- `GameEntity::dock_active_anim: bool` (replaced by event queue)
- `handle_unload` function in `miner_system.rs` (dead code)
- `MinerState::Unload` dispatch arm in `process_miner` (variant kept in enum to avoid serialization break)
- `unload_base_total: u32` field on `Miner` (no longer needed since purifier is per-bale)

**Modified semantics:**
- `RefineryDockPhase::Approach` now ALSO includes the dock-reservation poll loop (formerly `WaitForDock`'s job)
- `RefineryDockPhase::Unloading` now emits `BaleDepositEvent` per bale and applies purifier bonus per-bale (was end-of-load)

---

## Sim Checklist

- [ ] All math integer / fixed-point — no f32/f64 in game logic ✅ (cell math is u16; lepton offsets are i32)
- [ ] New state included in deterministic state hash ✅ (new `RefineryDockPhase` variants — Task 6 note; `bale_events` excluded as transient per `#[serde(skip)]`)
- [ ] No dependencies on render/ui/sidebar/audio/net ✅ (sim emits events; render reads)
- [ ] Tick ordering impact noted ✅ (bale events emit during `tick_miners`; renderer drains after sim tick)
- [ ] BTreeMap iteration order considered ✅ (`DockReservations` and `EntityStore` already use stable_id keys)

---

## Risk Areas

1. **Path-grid + occupancy-grid stamping change (Task 4)** — affects EVERY building placement. If `building_footprint_cells()` returns wrong cells, building placement breaks game-wide. Mitigation: Task 3 has comprehensive unit tests; Task 4 has a GAREFN-specific integration test.
2. **`RefineryDockPhase` enum change is not snapshot-backwards-compatible** (Task 6) — saved replays using old variants fail to deserialize. Acceptable; replay system is in-flight.
3. **Atomic commit window for Tasks 6-11** — these collectively rewrite the FSM. Build is broken between them. Implementer must complete all six before committing (CLAUDE.md prohibits `--no-verify`).
4. **Renderer hot-path change** — Tasks 12-15 modify the per-frame building render loop. Performance regression risk; profile before/after if numbers feel slow.

---

## Parity-Critical Items

The player-visible details that must match gamemd exactly:

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 8 | Approach phase: no body rotation before driving onto pad | Player sees "spinning at queue cell" today; gamemd doesn't | Visual A/B vs gamemd, confirm no pivot |
| 9 | Linked phase: no 180° pivot on pad | Player sees on-pad spin today; gamemd doesn't | Visual A/B vs gamemd |
| 10 | Per-bale purifier credits inline (not end-of-load) | Credits ticker shows lump sum at end today; gamemd ticks per bale | Watch credits during dock w/ Purifier built |
| 11 | Exit cell at building_origin + (-0x80, +0x80) leptons | Harvester exits in different direction than gamemd | Visual A/B comparison of exit path |
| 11 | Exit facing snap to 0x47 | Wrong heading after dock = different SearchOre target | Inspect facing in entity debug view |
| 14 | Per-bale SpecialAnim trigger (GAREFNOR) | We never trigger this; gamemd plays it every bale | Visual: GAREFNOR pulse per bale |
| 15 | Per-bale particle bursts at RefinerySmokeOffsetOne/Two | We never trigger these; gamemd emits 4 per bale | Visual: smoke puffs every bale |
| 10 | Per-bale interval = 14.4 frames (HarvesterDumpRate × 900) | Total unload time visible | Frame-counter check during full unload (already correct in `unload_timer`) |
| 4 | AddOccupy/RemoveOccupy correct stamping | Pad cell currently blocked → bypass_grid hack; mod compatibility | Visual: build refinery, walk through (rx-1, ry) and (rx-1, ry-1) cells; verify (rx+3, ry+1) walkable |

---

## Tasks

### Task 1: Parse `AddOccupy*` and `RemoveOccupy*` in art_data.rs

**Why:** Foundation for the `building_footprint_cells` helper. INI parsing is the lowest-risk addition — pure additive, no consumers yet.

**Files:**
- Modify: [src/rules/art_data.rs](../../src/rules/art_data.rs)

**Pattern:** Mirror existing INI parsing patterns in `art_data.rs::ArtEntry::from_ini_section` — see how `damage_fire_offsets` is parsed at line 262-279 (loop with index, break on missing key).

**Step 1: Add fields to `ArtEntry`.**

Add to the struct definition (search for `pub struct ArtEntry`):

```rust
/// Cells added to the rectangular foundation (AddOccupy1..N from art.ini).
/// Signed offsets from the building's origin (rx, ry) — negative = west/north.
pub add_occupy: Vec<(i16, i16)>,
/// Cells removed from the rectangular foundation (RemoveOccupy1..N from art.ini).
pub remove_occupy: Vec<(i16, i16)>,
```

**Step 2: Add parsing in `ArtEntry::from_ini_section`.**

After the `damage_fire_offsets` block (around line 279), add:

```rust
let add_occupy: Vec<(i16, i16)> = {
    let mut offsets = Vec::new();
    for i in 1..=8 {
        let key = format!("AddOccupy{}", i);
        if let Some(val) = section.get(&key) {
            let mut parts = val.split(',');
            if let (Some(x), Some(y)) = (
                parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
                parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
            ) {
                offsets.push((x, y));
            }
        } else {
            break;
        }
    }
    offsets
};
let remove_occupy: Vec<(i16, i16)> = {
    let mut offsets = Vec::new();
    for i in 1..=8 {
        let key = format!("RemoveOccupy{}", i);
        if let Some(val) = section.get(&key) {
            let mut parts = val.split(',');
            if let (Some(x), Some(y)) = (
                parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
                parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
            ) {
                offsets.push((x, y));
            }
        } else {
            break;
        }
    }
    offsets
};
```

Add `add_occupy` and `remove_occupy` to the `ArtEntry { ... }` construction at line 302+.

**Step 3: Defaults in other `ArtEntry` construction sites.**

Search for `ArtEntry {` outside `from_ini_section`. Likely sites: [src/rules/shp_vehicle_sequence.rs](../../src/rules/shp_vehicle_sequence.rs) (around line 123). Add `add_occupy: Vec::new(), remove_occupy: Vec::new()` to each.

**Step 4: Add tests at bottom of art_data.rs.**

```rust
#[cfg(test)]
mod add_remove_occupy_tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn parses_add_occupy_from_ini() {
        let ini = IniFile::from_str(
            "[GAREFN]\nAddOccupy1=-1,0\nAddOccupy2=-1,-1\nRemoveOccupy1=3,1\n"
        );
        let registry = ArtRegistry::from_ini(&ini);
        let entry = registry.get("GAREFN").expect("GAREFN");
        assert_eq!(entry.add_occupy, vec![(-1, 0), (-1, -1)]);
        assert_eq!(entry.remove_occupy, vec![(3, 1)]);
    }

    #[test]
    fn empty_when_no_keys() {
        let ini = IniFile::from_str("[FOO]\nFoundation=2x2\n");
        let registry = ArtRegistry::from_ini(&ini);
        let entry = registry.get("FOO").expect("FOO");
        assert!(entry.add_occupy.is_empty());
        assert!(entry.remove_occupy.is_empty());
    }

    #[test]
    fn skips_malformed_entries() {
        let ini = IniFile::from_str(
            "[FOO]\nAddOccupy1=not_a_pair\nAddOccupy2=1,2\n"
        );
        let registry = ArtRegistry::from_ini(&ini);
        let entry = registry.get("FOO").expect("FOO");
        // The loop breaks when it can't parse; malformed first entry stops the loop.
        // (Behavior matches damage_fire_offsets pattern.)
        assert!(entry.add_occupy.is_empty());
    }
}
```

**Step 5: Verify**

Run: `cargo test -p ra2 --lib add_remove_occupy_tests`
Expected: 3 tests pass.

**Step 6: Commit.**
Commit message: `rules: parse AddOccupy/RemoveOccupy from art.ini`

---

### Task 2: Add `add_occupy`/`remove_occupy`/`refinery_smoke_frames` to ObjectType + merge from art

**Why:** Carries parsed values from ArtEntry into the ObjectType used by sim. Also adds the missing `RefinerySmokeFrames` parsing needed by Task 15.

**Files:**
- Modify: [src/rules/object_type.rs](../../src/rules/object_type.rs)
- Modify: [src/rules/ruleset.rs](../../src/rules/ruleset.rs)

**Pattern:** Same as `queueing_cell` and `docking_offset` merge already in `ruleset.rs::merge_art_data` (around line 1430-1437). For `refinery_smoke_frames`, mirror the existing TechnoType particle-field parsing (commit 0101a64 added `refinery_smoke_offsets` and `refinery_smoke_particle_system`).

**Step 1: Add fields to `ObjectType` struct.**

Near line 305 (alongside `queueing_cell`/`docking_offset`):

```rust
/// Cells added to the rectangular foundation (from art.ini AddOccupy1..N).
pub add_occupy: Vec<(i16, i16)>,
/// Cells removed from the rectangular foundation (from art.ini RemoveOccupy1..N).
pub remove_occupy: Vec<(i16, i16)>,
```

Near line 627 (alongside `refinery_smoke_offsets`):

```rust
/// `RefinerySmokeFrames=` — frame count for the smoke particle system.
pub refinery_smoke_frames: u16,
```

**Step 2: Default initialization.**

In `ObjectType::from_ini_section` (around line 775), add:

```rust
add_occupy: Vec::new(),
remove_occupy: Vec::new(),
```

In the same constructor near the existing `refinery_smoke_*` reads (line ~948-980), add:

```rust
refinery_smoke_frames: section.get_i32("RefinerySmokeFrames").unwrap_or(0).max(0) as u16,
```

**Step 3: Merge from ArtEntry in ruleset.rs.**

After the existing `docking_offset` merge (around line 1437):

```rust
// Merge AddOccupy/RemoveOccupy from art.ini.
if !entry.add_occupy.is_empty() {
    obj.add_occupy = entry.add_occupy.clone();
}
if !entry.remove_occupy.is_empty() {
    obj.remove_occupy = entry.remove_occupy.clone();
}
```

Note: `refinery_smoke_frames` comes from rulesmd.ini directly (not art), so no merge needed — it's parsed in step 2.

**Step 4: Add tests.**

In ruleset.rs tests:

```rust
#[test]
fn merge_art_propagates_add_remove_occupy() {
    let rules_ini = "[BuildingTypes]\n0=GAREFN\n[GAREFN]\nFoundation=4x3\n";
    let art_ini = "[GAREFN]\nAddOccupy1=-1,0\nRemoveOccupy1=3,1\n";
    let mut ruleset = RuleSet::from_str(rules_ini).expect("rules");
    let art = crate::rules::art_data::ArtRegistry::from_ini(&IniFile::from_str(art_ini));
    ruleset.merge_art_data(&art);
    let obj = ruleset.object("GAREFN").expect("GAREFN");
    assert_eq!(obj.add_occupy, vec![(-1, 0)]);
    assert_eq!(obj.remove_occupy, vec![(3, 1)]);
}
```

In object_type.rs tests, near `techno_type_parses_refinery_smoke_offsets`:

```rust
#[test]
fn techno_type_parses_refinery_smoke_frames() {
    let ini = IniFile::from_str(
        "[FOO]\nRefinerySmokeFrames=50\n"
    );
    let section = ini.section("FOO").expect("section");
    let obj = ObjectType::from_ini_section("FOO", &section, ObjectCategory::Building);
    assert_eq!(obj.refinery_smoke_frames, 50);
}
```

**Step 5: Verify**

Run: `cargo test -p ra2 --lib merge_art_propagates_add_remove_occupy refinery_smoke_frames`
Expected: PASS.

Run: `cargo build` to confirm no other ObjectType construction sites broke.

**Step 6: Commit.**
Commit message: `rules: carry AddOccupy/RemoveOccupy through art→ObjectType merge; parse RefinerySmokeFrames`

---

### Task 3: `building_footprint_cells` helper + tests

**Why:** Pure logic helper. Replaces ad-hoc rectangle stamping. High-confidence unit-testable; lands before any consumer changes.

**Files:**
- Modify: [src/sim/production/production_tech.rs](../../src/sim/production/production_tech.rs)

**Pattern:** Sits next to `foundation_dimensions` (line 562) — same module, same data-driven approach.

**Step 1: Add the helper function.**

Append after `foundation_dimensions`:

```rust
/// Returns the actual occupied cells for a building, applying AddOccupy and
/// RemoveOccupy to the rectangular foundation. Cells outside [0, u16::MAX]
/// after offset application are dropped.
///
/// Order of operations:
/// 1. Generate rectangle cells (rx..rx+w) × (ry..ry+h)
/// 2. Add cells from add_occupy (deltas relative to origin)
/// 3. Remove cells listed in remove_occupy (deltas relative to origin)
///
/// Returns sorted, deduplicated cells.
pub fn building_footprint_cells(
    origin_rx: u16,
    origin_ry: u16,
    foundation: &str,
    add_occupy: &[(i16, i16)],
    remove_occupy: &[(i16, i16)],
) -> Vec<(u16, u16)> {
    use std::collections::BTreeSet;
    let (w, h) = foundation_dimensions(foundation);
    let mut cells: BTreeSet<(u16, u16)> = BTreeSet::new();

    for dx in 0..w {
        for dy in 0..h {
            let rx = origin_rx as i32 + dx as i32;
            let ry = origin_ry as i32 + dy as i32;
            if rx >= 0 && rx <= u16::MAX as i32 && ry >= 0 && ry <= u16::MAX as i32 {
                cells.insert((rx as u16, ry as u16));
            }
        }
    }

    for &(dx, dy) in add_occupy {
        let rx = origin_rx as i32 + dx as i32;
        let ry = origin_ry as i32 + dy as i32;
        if rx >= 0 && rx <= u16::MAX as i32 && ry >= 0 && ry <= u16::MAX as i32 {
            cells.insert((rx as u16, ry as u16));
        }
    }

    for &(dx, dy) in remove_occupy {
        let rx = origin_rx as i32 + dx as i32;
        let ry = origin_ry as i32 + dy as i32;
        if rx >= 0 && rx <= u16::MAX as i32 && ry >= 0 && ry <= u16::MAX as i32 {
            cells.remove(&(rx as u16, ry as u16));
        }
    }

    cells.into_iter().collect()
}
```

**Step 2: Add tests.**

```rust
#[cfg(test)]
mod footprint_tests {
    use super::*;

    #[test]
    fn rectangle_only_4x3() {
        let cells = building_footprint_cells(10, 20, "4x3", &[], &[]);
        assert_eq!(cells.len(), 12);
        assert!(cells.contains(&(10, 20)));
        assert!(cells.contains(&(13, 22)));
    }

    #[test]
    fn garefn_real_footprint() {
        // GAREFN: Foundation=4x3, AddOccupy1=-1,0, AddOccupy2=-1,-1, RemoveOccupy1=3,1
        let cells = building_footprint_cells(
            10, 20, "4x3",
            &[(-1, 0), (-1, -1)],
            &[(3, 1)],
        );
        assert_eq!(cells.len(), 13); // 12 base + 2 added - 1 removed
        assert!(cells.contains(&(9, 19)));   // (-1,-1) added
        assert!(cells.contains(&(9, 20)));   // (-1, 0) added
        assert!(!cells.contains(&(13, 21))); // (3, 1) removed (the dock pad)
    }

    #[test]
    fn add_then_remove_overlap() {
        let cells = building_footprint_cells(10, 20, "1x1", &[(2, 0)], &[(2, 0)]);
        assert_eq!(cells.len(), 1);
        assert!(cells.contains(&(10, 20)));
    }

    #[test]
    fn negative_offset_clamping() {
        let cells = building_footprint_cells(0, 0, "1x1", &[(-1, 0), (-1, -1)], &[]);
        assert_eq!(cells.len(), 1);
    }

    #[test]
    fn deduplication() {
        let cells = building_footprint_cells(10, 20, "2x2", &[(0, 0)], &[]);
        assert_eq!(cells.len(), 4);
    }
}
```

**Step 3: Verify**

Run: `cargo test -p ra2 --lib footprint_tests`
Expected: 5 tests pass.

**Step 4: Commit.**
Commit message: `production: add building_footprint_cells helper with AddOccupy/RemoveOccupy support`

---

### Task 4: Wire footprint helper into PathGrid + OccupancyGrid stamping

**Why:** Replaces the rectangle-only stamping that incorrectly blocks GAREFN's pad cell (rx+3, ry+1). Affects both grids — pathfinding (PathGrid) and per-cell entity ownership (OccupancyGrid).

**Files:**
- Modify: [src/sim/pathfinding/core.rs:1041](../../src/sim/pathfinding/core.rs#L1041) — extend `PathGrid::block_building_footprint` signature
- Modify: [src/app_init.rs:622](../../src/app_init.rs#L622) — caller update
- Modify: [src/app_sim_tick.rs:729](../../src/app_sim_tick.rs#L729) and line 740 — caller updates
- Modify: [src/sim/world/world_spawn.rs:240-261](../../src/sim/world/world_spawn.rs#L240-L261) — OccupancyGrid stamping
- Modify: [src/sim/miner/miner_tests.rs:1602](../../src/sim/miner/miner_tests.rs#L1602) and line 1722 — test callers
- Modify: [src/sim/pathfinding/core_tests.rs:301](../../src/sim/pathfinding/core_tests.rs#L301) — test caller

**Pattern:** Method signature extension + call-site updates. Use `building_footprint_cells` from Task 3.

**Step 1: Extend `PathGrid::block_building_footprint` signature.**

Current:
```rust
pub fn block_building_footprint(&mut self, cell_rx: u16, cell_ry: u16, foundation: &str) {
    let (fw, fh): (u16, u16) = parse_foundation(foundation);
    for dy in 0..fh {
        for dx in 0..fw {
            let bx = cell_rx.wrapping_add(dx);
            let by = cell_ry.wrapping_add(dy);
            self.set_blocked(bx, by, true);
        }
    }
}
```

New (extends signature with two slices):
```rust
pub fn block_building_footprint(
    &mut self,
    cell_rx: u16,
    cell_ry: u16,
    foundation: &str,
    add_occupy: &[(i16, i16)],
    remove_occupy: &[(i16, i16)],
) {
    let cells = crate::sim::production::production_tech::building_footprint_cells(
        cell_rx, cell_ry, foundation, add_occupy, remove_occupy,
    );
    for (rx, ry) in cells {
        self.set_blocked(rx, ry, true);
    }
}
```

**Step 2: Update non-test callers.**

[src/app_init.rs:622](../../src/app_init.rs#L622) — needs the obj reference for add/remove occupy. Read the surrounding code to find where `obj` is available; pass `&obj.add_occupy, &obj.remove_occupy`. If obj isn't available at that callsite, look up via `rules.object(building_type_id)`.

[src/app_sim_tick.rs:729](../../src/app_sim_tick.rs#L729): same pattern.

[src/app_sim_tick.rs:740](../../src/app_sim_tick.rs#L740): this caller passes hardcoded `"1x1"` (probably a wall or filler). Pass empty slices: `&[], &[]`.

**Step 3: Update test callers.**

[src/sim/miner/miner_tests.rs:1602](../../src/sim/miner/miner_tests.rs#L1602) and line 1722:
```rust
// Before:
path_grid.block_building_footprint(10, 10, "4x3");
// After:
path_grid.block_building_footprint(10, 10, "4x3", &[], &[]);
```

[src/sim/pathfinding/core_tests.rs:301](../../src/sim/pathfinding/core_tests.rs#L301): same pattern.

**Step 4: Update OccupancyGrid stamping in world_spawn.rs.**

Current code at [src/sim/world/world_spawn.rs:240-261](../../src/sim/world/world_spawn.rs#L240-L261):

```rust
let spawn_foundation = if category == EntityCategory::Structure {
    rules
        .and_then(|r| r.object(&map_ent.type_id))
        .map(|obj| foundation_dimensions(&obj.foundation))
} else {
    None
};
self.entities.insert(ge);
self.increment_owned_count(&owner_str, category);
if let Some((fw, fh)) = spawn_foundation {
    for dy in 0..fh {
        for dx in 0..fw {
            self.occupancy.add(spawn_rx + dx, spawn_ry + dy, spawn_sid, spawn_layer, None);
        }
    }
} else {
    self.occupancy.add(spawn_rx, spawn_ry, spawn_sid, spawn_layer, spawn_sub_cell);
}
```

Replace with:

```rust
let spawn_cells: Option<Vec<(u16, u16)>> = if category == EntityCategory::Structure {
    rules
        .and_then(|r| r.object(&map_ent.type_id))
        .map(|obj| crate::sim::production::production_tech::building_footprint_cells(
            spawn_rx,
            spawn_ry,
            &obj.foundation,
            &obj.add_occupy,
            &obj.remove_occupy,
        ))
} else {
    None
};
self.entities.insert(ge);
self.increment_owned_count(&owner_str, category);
if let Some(cells) = spawn_cells {
    for (rx, ry) in cells {
        self.occupancy.add(rx, ry, spawn_sid, spawn_layer, None);
    }
} else {
    self.occupancy.add(spawn_rx, spawn_ry, spawn_sid, spawn_layer, spawn_sub_cell);
}
```

**Step 5: Add a GAREFN-specific integration test.**

In [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs):

```rust
#[test]
fn garefn_footprint_leaves_dock_pad_walkable() {
    let mut grid = PathGrid::new(32, 32);
    grid.block_building_footprint(
        10, 10, "4x3",
        &[(-1, 0), (-1, -1)],
        &[(3, 1)],
    );
    // RemoveOccupy1=3,1 → cell (13, 11) should be walkable (the dock pad)
    assert!(!grid.is_blocked(13, 11), "RemoveOccupy1=3,1 should leave (rx+3,ry+1) walkable");
    // AddOccupy1=-1,0 → cell (9, 10) should be blocked
    assert!(grid.is_blocked(9, 10), "AddOccupy1=-1,0 should block (rx-1, ry+0)");
    // AddOccupy2=-1,-1 → cell (9, 9) should be blocked
    assert!(grid.is_blocked(9, 9), "AddOccupy2=-1,-1 should block (rx-1, ry-1)");
    // Standard rectangle cells still blocked
    assert!(grid.is_blocked(10, 10));
    assert!(grid.is_blocked(13, 12));
}
```

**Step 6: Verify**

Run: `cargo test -p ra2 garefn_footprint`
Expected: PASS.
Run: `cargo build` — expect no compile errors after all callsite updates.
Run: `cargo test -p ra2 --lib` — expect no regressions.

**Step 7: Commit.**
Commit message: `pathfinding+occupancy: stamp building footprints via building_footprint_cells (respects AddOccupy/RemoveOccupy)`

---

### Task 5: Define `BaleDepositEvent` + add `bale_events` queue to Simulation

**Why:** Event channel from sim to renderer. Mirrors existing `sound_events` and `fire_events` patterns.

**Files:**
- Modify: [src/sim/components.rs](../../src/sim/components.rs)
- Modify: [src/sim/world/mod.rs](../../src/sim/world/mod.rs) at lines 188-340

**Pattern:** Identical to `SimSoundEvent` + `Simulation::sound_events: Vec<SimSoundEvent>` (lines 207-208 of mod.rs).

**Step 1: Define event type.**

In [src/sim/components.rs](../../src/sim/components.rs) (near other event types):

```rust
/// Emitted by the refinery dock state machine each time a harvester deposits
/// one bale. Renderer consumes it to fire SpecialAnim (slot 10) and spawn
/// 4 particle bursts at RefinerySmokeOffset positions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BaleDepositEvent {
    /// Refinery stable_id where the bale was deposited.
    pub building_id: u64,
    /// Sim tick when this event was emitted (for ordering / debugging).
    pub tick: u64,
}
```

**Step 2: Add field to `Simulation`.**

In [src/sim/world/mod.rs:188-220](../../src/sim/world/mod.rs#L188), after `pub fire_events: Vec<SimFireEvent>` (line 212):

```rust
/// Bale deposit events emitted during refinery dock unloading — drained
/// by the app layer for SpecialAnim trigger and particle bursts.
#[serde(skip)]
pub bale_events: Vec<crate::sim::components::BaleDepositEvent>,
```

**Step 3: Initialize in `Simulation::new()` (or wherever the struct is constructed).**

Add `bale_events: Vec::new(),` near where `sound_events: Vec::new()` is at line 336.

**Step 4: Drain at end of frame.**

Find where `sim.sound_events.clear()` happens (renderer or app glue, likely in `app_sim_tick.rs` post-render). Add a parallel `sim.bale_events.clear()` after the renderer drains it. Confirm this during Tasks 14-15 implementation.

**Step 5: Verify**

Run: `cargo build`
Expected: compile success.

**Step 6: Commit.**
Commit message: `sim: add BaleDepositEvent + bale_events queue on Simulation`

---

### Task 6: Collapse `RefineryDockPhase` enum 7→4 (START OF ATOMIC COMMIT WINDOW)

> **⚠️ ATOMIC COMMIT WARNING ⚠️**
> Tasks 6 through 11 collectively rewrite the dock state machine. The build is broken between them. **DO NOT commit until Task 11 completes.** Pre-commit hooks will reject. This is a single logical change broken into reviewable steps; commit once at the end.

**Why:** Pure type-level change. Defines the contract every subsequent task implements against.

**Files:**
- Modify: [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs) — replace enum at line 85

**Step 1: Replace the enum definition.**

```rust
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum RefineryDockPhase {
    /// Pathing toward QueueingCell while polling DockReservations each tick.
    /// On reservation grant: re-target pad cell, transition to Linked.
    #[default]
    Approach,
    /// Reservation granted; driving onto pad cell. On arrival: emit DockArrival
    /// sound, set display_type_override = UnloadingClass, init unload_timer,
    /// transition to Unloading.
    Linked,
    /// Per-bale deposit pulse. Each bale emits BaleDepositEvent.
    /// On cargo empty: release reservation, transition to Departing.
    Unloading,
    /// Drive to exit cell at building_origin + (-0x80, +0x80) leptons.
    /// On arrival: snap facing 0x47, return to SearchOre.
    Departing,
}
```

**Step 2: Run cargo check (expect errors).**

Run: `cargo check 2>&1 | head -100`
Capture the error list — these are the files Tasks 7-11 must fix. Expected breakage in `miner_dock_sequence.rs`, `miner_system.rs`, `miner_tests.rs`, and `ai.rs:1067`.

**Do NOT commit yet.** Move to Task 7.

---

### Task 7: Drop dead `MinerState::Unload` dispatch arm

**Why:** Dead code path; simplifies the FSM dispatcher. Surfaced during the trace investigation.

**Files:**
- Modify: [src/sim/miner/miner_system.rs:188](../../src/sim/miner/miner_system.rs#L188)
- Modify: [src/sim/ai.rs:1071](../../src/sim/ai.rs#L1071)

**Step 1: Replace the dispatch arm.**

[src/sim/miner/miner_system.rs:188](../../src/sim/miner/miner_system.rs#L188):

```rust
// Before:
//   MinerState::Unload => handle_unload(sim, rules, config, snap),
// After:
MinerState::Unload => {
    // Legacy state — production code never enters this path. If we encounter
    // it (e.g., loaded from an old save), fall through to SearchOre.
    snap.miner.state = MinerState::SearchOre;
}
```

**Step 2: Delete `handle_unload` function.**

In [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs) around lines 511-562 — delete the entire `fn handle_unload(...)` block.

**Step 3: Update `ai.rs` test scaffolding.**

[src/sim/ai.rs:1071-1074](../../src/sim/ai.rs#L1071):

```rust
// Before:
//   MinerState::Unload => {
//       saw_dock_or_unload = true;
//       saw_unload = true;
//   }
// After: remove the arm entirely; the Dock arm at line 1065 already detects unload via dock_phase.
```

**Step 4: Verify**

Run: `cargo check`
Expected: errors only from Task 6's enum change (in miner_dock_sequence.rs and miner_tests.rs).

**Do NOT commit yet.** Move to Task 8.

---

### Task 8: Rewrite `phase_approach` handler

**Why:** Absorbs old `WaitForDock` and `RotateToPad` mechanics. Polls reservation each tick; on grant, re-target pad with bypass_grid (workaround retained until Task 20).

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs)

**Pattern:** Existing two-phase snapshot pattern from `tick_miners`. Existing helpers (`is_adjacent_or_at`, `issue_move_if_idle`, `movement::issue_direct_move`) reused.

**Step 1: Replace `phase_approach`.**

```rust
fn phase_approach(
    sim: &mut Simulation,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
    queue: (u16, u16),
    pad: (u16, u16),
    ref_sid: u64,
) {
    // Try to acquire the dock reservation. If granted, immediately re-target
    // the pad cell with bypass_grid and transition to Linked.
    if sim.production.dock_reservations.try_reserve(ref_sid, snap.entity_id) {
        snap.miner.dock_queued = false;
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, pad, snap.speed);
        if let Some(entity) = sim.entities.get_mut(snap.entity_id)
            && let Some(ref mut mt) = entity.movement_target
        {
            mt.bypass_grid = true;
        }
        snap.miner.dock_phase = RefineryDockPhase::Linked;
        return;
    }
    snap.miner.dock_queued = true;

    // Reservation not granted — keep heading toward QueueingCell.
    if !is_adjacent_or_at((snap.rx, snap.ry), queue) {
        if let Some(grid) = path_grid {
            issue_move_if_idle(&mut sim.entities, grid, snap.entity_id, queue, snap.speed);
        }
    }
    // If adjacent, just loiter.
}
```

**Step 2: Delete `phase_wait_for_dock` and `phase_rotate_to_pad` functions.**

**Step 3: Update `handle_dock_sequence` dispatch to match new variants.**

```rust
match snap.miner.dock_phase {
    RefineryDockPhase::Approach => {
        phase_approach(sim, path_grid, snap, queue, pad, ref_sid);
    }
    RefineryDockPhase::Linked => {
        phase_linked(sim, rules, snap, pad, ref_sid);  // Task 9
    }
    RefineryDockPhase::Unloading => {
        phase_unloading(sim, rules, config, snap, ref_sid);  // Task 10
    }
    RefineryDockPhase::Departing => {
        phase_departing(sim, snap, exit);  // Task 11
    }
}
```

**Do NOT commit yet.** Move to Task 9.

---

### Task 9: Rewrite `phase_linked` (replaces `phase_enter_pad` and `phase_turn_on_pad`)

**Why:** Drives onto pad, sets up display_type_override and dock anim, transitions to Unloading. No body rotation step.

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs)

**Step 1: Add `phase_linked`.**

```rust
fn phase_linked(
    sim: &mut Simulation,
    rules: &RuleSet,
    snap: &mut MinerSnapshot,
    pad: (u16, u16),
    ref_sid: u64,
) {
    let arrived = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_none());
    if !arrived {
        return;
    }

    snap.rx = pad.0;
    snap.ry = pad.1;

    // Apply UnloadingClass override.
    if let Some(uc) = unloading_class(rules, sim.interner.resolve(snap.type_id))
        && let Some(entity) = sim.entities.get_mut(snap.entity_id)
    {
        entity.display_type_override = Some(sim.interner.intern(&uc));
    }

    // Emit DockDeploy sound.
    sim.sound_events.push(SimSoundEvent::DockDeploy {
        building_id: ref_sid,
    });

    // Initialize unload_timer to 0 — first bale fires after one full
    // unload_tick_interval, matching gamemd's per-bale gate.
    snap.miner.unload_timer = 0;

    snap.miner.dock_phase = RefineryDockPhase::Unloading;
}
```

**Step 2: Delete `phase_enter_pad` and `phase_turn_on_pad`.**

**Do NOT commit yet.** Move to Task 10.

---

### Task 10: Rewrite `phase_unloading` (per-bale purifier inline + emit events)

**Why:** Two parity fixes: (a) purifier bonus per-bale, (b) emit `BaleDepositEvent` per pulse.

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs)
- Modify: [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs) — drop `unload_base_total` from `Miner` struct

**Step 1: Replace `phase_unloading`.**

```rust
fn phase_unloading(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    snap: &mut MinerSnapshot,
    ref_sid: u64,
) {
    use crate::sim::components::BaleDepositEvent;

    if snap.miner.unload_timer > 0 {
        snap.miner.unload_timer -= 10;
        return;
    }

    if let Some(bale) = snap.miner.cargo.pop() {
        let value: i32 = i32::from(bale.value);
        let owner_str = sim.interner.resolve(snap.owner).to_string();

        {
            let credits = credits_entry_for_owner(sim, &owner_str);
            *credits = credits.saturating_add(value);
        }

        // Per-bale purifier bonus (matches gamemd).
        if player_has_purifier(sim, rules, sim.interner.resolve(snap.owner)) {
            let bonus_pct: i32 = rules.general.purifier_bonus_pct;
            let bonus: i32 = value * bonus_pct / 100;
            if bonus > 0 {
                let credits = credits_entry_for_owner(sim, &owner_str);
                *credits = credits.saturating_add(bonus);
            }
        }

        sim.bale_events.push(BaleDepositEvent {
            building_id: ref_sid,
            tick: sim.tick,
        });

        snap.miner.unload_timer = snap
            .miner
            .unload_timer
            .saturating_add(config.unload_tick_interval as i16);
        return;
    }

    // Cargo empty — release dock and depart.
    sim.production.dock_reservations.release(ref_sid);
    snap.miner.home_refinery = Some(ref_sid);

    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.display_type_override = None;
    }

    snap.miner.dock_phase = RefineryDockPhase::Departing;
}
```

**Step 2: Remove `unload_base_total` field from `Miner` struct.**

In [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs) — delete `pub unload_base_total: u32` from the `Miner` struct definition + its initialization in `Miner::new`.

**Step 3: Remove all references to `unload_base_total`.**

Run: `grep -rn 'unload_base_total' src/`. Delete every reference. Most are in `miner_system.rs`'s old `handle_unload` (already deleted in Task 7) and `phase_unloading` (just rewrote).

**Do NOT commit yet.** Move to Task 11.

---

### Task 11: Rewrite `phase_departing` (new exit cell formula) + ATOMIC COMMIT

**Why:** Fixes exit cell to gamemd's hardcoded `(-0x80, +0x80)` lepton offset from origin. Snaps facing to 0x47 on arrival. **End of atomic commit window — commit after this task.**

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs)

**Step 1: Replace `refinery_exit_cell` helper.**

```rust
/// Exit cell — where the miner drives after undocking.
///
/// gamemd-correct formula: building_origin_lepton + (-0x80, +0x80) leptons.
/// Origin in leptons = (rx * 256, ry * 256). Add offset, then floor-divide
/// by 256 for cell coordinates. Foundation dimensions are NOT used.
pub(super) fn refinery_exit_cell(rx: u16, ry: u16) -> (u16, u16) {
    let exit_x = (rx as i32 * 256 - 0x80) / 256;
    let exit_y = (ry as i32 * 256 + 0x80) / 256;
    (exit_x.max(0) as u16, exit_y.max(0) as u16)
}
```

Update `resolve_refinery_cells` to use the new signature.

**Step 2: Rename `phase_exit_pad` to `phase_departing` and clean up.**

```rust
fn phase_departing(
    sim: &mut Simulation,
    snap: &mut MinerSnapshot,
    exit: (u16, u16),
) {
    let moving = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_some());
    let at_exit = (snap.rx, snap.ry) == exit;
    let teleporting = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());

    if !moving && !at_exit {
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
        if let Some(entity) = sim.entities.get_mut(snap.entity_id)
            && let Some(ref mut mt) = entity.movement_target
        {
            mt.bypass_grid = true;
        }
        return;
    }

    if !moving && at_exit && !teleporting {
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.facing = 0x47;
        }
        snap.miner.reserved_refinery = None;
        snap.miner.dock_queued = false;
        snap.miner.forced_return = false;
        snap.miner.target_ore_cell = None;
        snap.miner.last_harvest_cell = None;
        snap.miner.dock_phase = RefineryDockPhase::Approach;
        snap.miner.state = MinerState::SearchOre;
        return;
    }

    if let Some(entity) = sim.entities.get(snap.entity_id) {
        snap.rx = entity.position.rx;
        snap.ry = entity.position.ry;
    }
}
```

**Step 3: Verify**

Run: `cargo check`
Expected: compiles. Tests will still fail until Task 16-17 (handled later).

Run: `cargo test -p ra2 --lib miner_dock 2>&1 | head -20`
Expected: many test failures referencing old enum variants. That's normal — Tasks 16-17 fix them.

**Step 4: COMMIT (atomic — all of Tasks 6-11 in one).**

Commit message:
```
miner_dock: collapse RefineryDockPhase 7→4 (Approach/Linked/Unloading/Departing)

- Drop fabricated WaitForDock/RotateToPad/EnterPad/TurnOnPad/ExitPad phases
- Match gamemd Mission_Deploy_Building 4-state inner FSM
- Per-bale purifier bonus inline (was end-of-load batch)
- Exit cell at origin + (-0x80, +0x80) leptons (was foundation-derived)
- Drop dead handle_unload + MinerState::Unload dispatch
- Emit BaleDepositEvent per bale for renderer

Tests in miner_tests.rs broken until Task 16/17 mechanical rename.
```

(Pre-commit hook will run `cargo check` — that should pass. `cargo test` may fail; the test fixes are coming in Tasks 16-17. If your hook runs `cargo test`, complete Tasks 16-17 BEFORE attempting commit.)

---

### Task 12: Remove `dock_active_anim` field from GameEntity

**Why:** Replaced by `bale_events` queue. Field has no remaining writers after Task 10, no remaining readers after Task 13.

**Files:**
- Modify: [src/sim/game_entity.rs:202](../../src/sim/game_entity.rs#L202) — delete field
- Modify: [src/sim/game_entity.rs](../../src/sim/game_entity.rs) Default impl — delete init
- Modify: [src/app_instances/shp.rs:303](../../src/app_instances/shp.rs#L303) — drop arg
- Modify: [src/app_instances/shp.rs:492](../../src/app_instances/shp.rs#L492) — drop param

**Step 1: Search for all references.**

Run: `grep -rn 'dock_active_anim' src/`. Expected sites:
- `src/sim/game_entity.rs:202` — field definition
- `src/sim/game_entity.rs` — Default init
- `src/app_instances/shp.rs:303, 492` — renderer
- `src/sim/miner/miner_dock_sequence.rs` — any leftover writes (should be zero after Task 9)

**Step 2: Delete the field + Default init.**

**Step 3: Update the renderer to drop the parameter.**

Function at [src/app_instances/shp.rs:492](../../src/app_instances/shp.rs#L492) — remove `dock_active_anim: bool,` parameter.
Caller at line 303 — remove `entity.dock_active_anim,` argument.

**Step 4: Verify**

Run: `cargo check`
Expected: some renderer-side errors related to the dock_active_anim conditional (line 529). Task 13 fixes those.

**Step 5: Commit.**
Commit message: `entities: remove dock_active_anim (replaced by bale_events queue)`

---

### Task 13: Renderer — drop `dock_active_anim` gating, leave ActiveAnim looping

**Why:** Without `dock_active_anim`, the renderer must decide what to do with the Active branch unconditionally. Per gamemd, ActiveAnim/Two/Three/Four loop continuously regardless of dock state.

> **Note:** The full storage-tier display fix (selecting ONE of slots 3-6 by storage level) is deferred to a follow-up PR per the Open Questions section. This task just removes the broken dock_active_anim gating; the four ActiveAnim variants will still loop simultaneously (visibly wrong, but no worse than current broken behavior — current code force-loops them when `dock_active_anim=true` and uses fallback logic otherwise).

**Files:**
- Modify: [src/app_instances/shp.rs:524-555](../../src/app_instances/shp.rs#L524-L555)

**Step 1: Simplify the Active branch.**

Replace the existing `dock_active_anim` block with a clean "always loop unless capturable+enemy" decision tree:

```rust
} else if matches!(
    anim.kind,
    crate::rules::art_data::BuildingAnimKind::Active
        | crate::rules::art_data::BuildingAnimKind::Production
) {
    if anim.loop_count < 0 {
        // Infinite-loop ActiveAnim. Capturable tech buildings (Oil Derrick,
        // Airport, etc.): primary slot only plays after capture.
        let is_capturable: bool = rules
            .and_then(|r| r.object(building_type))
            .map(|o| o.capturable)
            .unwrap_or(false);
        if anim.is_primary && is_capturable && !is_player_owned {
            anim.start_frame
        } else {
            looping_frame(anim, idle_anim_elapsed_ms)
        }
    } else {
        // One-shot ActiveAnim/Production driven by ECS overlays.
        overlays
            .and_then(|o| o.anims.iter().find(|a| anim_upper_id == Some(a.anim_type)))
            .map(|a| a.frame)
            .unwrap_or_else(|| resting_building_anim_frame(anim))
    }
}
```

(This is equivalent to the existing code with the `dock_active_anim` early-branch removed — the `else if anim.loop_count < 0` and `else` arms stay intact.)

**Step 2: Verify**

Run: `cargo build` — expect compile success.
Run game smoke test: build a refinery, observe ActiveAnim loops continuously regardless of harvester state.

**Step 3: Commit.**
Commit message: `render: drop dock_active_anim gating (ActiveAnim loops unconditionally)`

---

### Task 14: Renderer — consume bale events for SpecialAnim trigger

**Why:** Plays GAREFNOR (slot 10) one-shot per bale, matching gamemd.

**Files:**
- Modify: renderer code (find via `grep -rn 'BuildingAnimOverlays' src/`)

**Pattern:** Existing one-shot anim overlay machinery. Bale events trigger anims via the existing `BuildingAnimOverlays` API.

**Step 1: Locate the per-tick building overlay update.**

Run: `grep -rn 'BuildingAnimOverlays' src/` and inspect callers. Find where overlays are mutated each tick.

**Step 2: Drain bale events and trigger SpecialAnim.**

Insert into the per-frame setup pass (in app_instances/shp.rs or wherever `BuildingAnimOverlays` is updated):

```rust
for event in &sim.bale_events {
    if let Some(building) = sim.entities.get_mut(event.building_id) {
        if let Some(ref mut overlays) = building.building_anim_overlays {
            // Find the SpecialAnim entry in the building's art and (re)trigger it.
            // Use existing one-shot trigger API on BuildingAnimOverlays — refer
            // to existing callers in `sim/production/` for the exact method.
            overlays.trigger_one_shot_for_kind(BuildingAnimKind::Special);
        }
    }
}
```

(The exact API name depends on the existing one-shot machinery. If no `trigger_one_shot_for_kind` method exists, add one that finds the matching anim and resets its frame counter — pattern lives in `BuildingAnimOverlays`.)

**Step 3: Verify**

Run game, dock a harvester at a refinery, observe GAREFNOR plays at the building per bale.

**Step 4: Commit.**
Commit message: `render: trigger SpecialAnim (slot 10) one-shot per BaleDepositEvent`

---

### Task 15: Renderer — consume bale events for particle bursts

**Why:** Spawns up to 4 particles per bale at refinery's RefinerySmokeOffset offsets. Matches gamemd's vtable+0x468 emitter.

**Files:**
- Modify: renderer particle-spawning code (search via `grep -rn 'particle_system' src/sim/particles/`)

**Pattern:** Existing particle spawn API in `sim/particles/`. ObjectType already has `refinery_smoke_offsets: [IVec3; 4]` (commit 0101a64) and `refinery_smoke_particle_system: Option<String>`. Task 2 added `refinery_smoke_frames: u16`.

**Step 1: Add a particle-spawn driver in the bale event consumer.**

In the same place that drains bale_events for SpecialAnim (Task 14):

```rust
for event in &sim.bale_events {
    let Some(building) = sim.entities.get(event.building_id) else { continue };
    let Some(obj) = rules.object(sim.interner.resolve(building.type_ref)) else { continue };

    let Some(particle_type_name) = obj.refinery_smoke_particle_system.as_deref() else {
        continue;  // No particle system configured
    };
    let frame_count = obj.refinery_smoke_frames;

    // Iterate the fixed array; skip zero-vector entries (treated as "unset").
    for offset in obj.refinery_smoke_offsets.iter() {
        if *offset == IVec3::ZERO {
            continue;  // Unset slot — skip
        }
        let spawn_x: i32 = building.position.rx as i32 * 256 + offset.x;
        let spawn_y: i32 = building.position.ry as i32 * 256 + offset.y;
        let spawn_z: i32 = offset.z;
        // Use existing particle spawn API. Exact function depends on `sim/particles/spawn.rs`.
        crate::sim::particles::spawn::spawn_particle_at(
            particle_type_name,
            spawn_x,
            spawn_y,
            spawn_z,
            frame_count,
        );
    }
}
sim.bale_events.clear();
```

(Verify exact `spawn_particle_at` signature against `src/sim/particles/spawn.rs` during implementation.)

**Note on tiny-detail #26:** The Allied refinery defines `RefinerySmokeOffsetOne` and `Two` only (`Three` and `Four` not specified → default to `IVec3::ZERO`). The skip-zero loop correctly suppresses those, matching gamemd's behavior of overlapping invisible particles at origin.

**Step 2: Verify**

Run game; dock harvester; observe 2 visible smoke puffs per bale (One + Two) at correct offsets.

**Step 3: Commit.**
Commit message: `render: spawn particles per BaleDepositEvent at RefinerySmokeOffset positions`

---

### Task 16: Mechanical test rename — collapse old phase variants

**Why:** Existing dock tests reference old phase variants. Mechanical rename so they compile against the new enum.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs)

**Step 1: Rename mappings.**

| Old variant | New variant |
|---|---|
| `RefineryDockPhase::WaitForDock` | `RefineryDockPhase::Approach` |
| `RefineryDockPhase::RotateToPad` | `RefineryDockPhase::Approach` |
| `RefineryDockPhase::EnterPad` | `RefineryDockPhase::Linked` |
| `RefineryDockPhase::TurnOnPad` | `RefineryDockPhase::Linked` |
| `RefineryDockPhase::ExitPad` | `RefineryDockPhase::Departing` |

**Step 2: Mechanical replace.**

PowerShell (Windows):
```powershell
$path = "src\sim\miner\miner_tests.rs"
(Get-Content $path) `
  -replace 'RefineryDockPhase::WaitForDock', 'RefineryDockPhase::Approach' `
  -replace 'RefineryDockPhase::RotateToPad', 'RefineryDockPhase::Approach' `
  -replace 'RefineryDockPhase::EnterPad', 'RefineryDockPhase::Linked' `
  -replace 'RefineryDockPhase::TurnOnPad', 'RefineryDockPhase::Linked' `
  -replace 'RefineryDockPhase::ExitPad', 'RefineryDockPhase::Departing' `
  | Set-Content $path
```

Bash:
```bash
sed -i 's/RefineryDockPhase::WaitForDock/RefineryDockPhase::Approach/g;
        s/RefineryDockPhase::RotateToPad/RefineryDockPhase::Approach/g;
        s/RefineryDockPhase::EnterPad/RefineryDockPhase::Linked/g;
        s/RefineryDockPhase::TurnOnPad/RefineryDockPhase::Linked/g;
        s/RefineryDockPhase::ExitPad/RefineryDockPhase::Departing/g' \
        src/sim/miner/miner_tests.rs
```

Also rename calls to `block_building_footprint` to pass the two new slice args (lines 1602, 1722):

```bash
sed -i 's/block_building_footprint(\([^,]*\), \([^,]*\), "\([^"]*\)")/block_building_footprint(\1, \2, "\3", \&[], \&[])/g' \
        src/sim/miner/miner_tests.rs
```

**Step 3: Run tests.**

Run: `cargo test -p ra2 --lib miner`
Expected: most pass; some fail because they assert on phase-specific behavior that no longer exists. Those drop to Task 17.

**Step 4: Commit.**
Commit message: `miner_tests: mechanical rename for RefineryDockPhase 7→4 collapse`

---

### Task 17: Delete tests of fabricated mechanics

**Why:** Tests asserting on RotateToPad/TurnOnPad behavior — fabricated mechanics that no longer exist.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs)

**Step 1: Identify tests to delete.**

Run: `cargo test -p ra2 --lib miner 2>&1 | grep -E 'FAIL|panicked'` to find failing tests after Task 16.

Likely candidates:
- Tests asserting facing changes during the old RotateToPad/TurnOnPad
- Tests asserting `display_type_override` is set in TurnOnPad specifically
- Tests checking body rotation rate during dock

**Step 2: Delete each failing test that asserts on fabricated mechanics.**

For each: delete the entire `#[test] fn name() { ... }` block.

**Step 3: Verify**

Run: `cargo test -p ra2 --lib miner`
Expected: all remaining tests pass.

**Step 4: Commit.**
Commit message: `miner_tests: drop tests of fabricated RotateToPad/TurnOnPad mechanics`

---

### Task 18: Add new tests for collapsed FSM and bale events

**Why:** Cover the new behaviors: per-bale event emission, exit cell formula, phase transitions.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs)

**Step 1: Test phase transitions.**

```rust
#[test]
fn approach_to_linked_on_reservation_grant() {
    // Setup: harvester near queue cell, refinery has open dock.
    // Tick: try_reserve succeeds; phase becomes Linked.
    // Assert: dock_phase == Linked, movement_target.bypass_grid == true.
}

#[test]
fn linked_to_unloading_on_pad_arrival() {
    // Setup: harvester at pad cell, dock_phase = Linked, movement_target = None.
    // Tick: phase_linked sees arrival; transitions to Unloading.
    // Assert: dock_phase == Unloading, display_type_override == Some(UnloadingClass).
}
```

**Step 2: Test bale event emission.**

```rust
#[test]
fn unloading_emits_bale_event_per_bale() {
    // Setup: harvester on pad with 5 bales, unload_timer = 0.
    // Tick 5 times.
    // Assert: sim.bale_events.len() == 5.
}
```

**Step 3: Test per-bale purifier.**

```rust
#[test]
fn unloading_applies_per_bale_purifier_bonus() {
    // Setup: harvester with 1 bale value=100, owner has Purifier (bonus 25%).
    // Tick until bale deposits.
    // Assert: credits increased by 100 + 25 = 125.
}
```

**Step 4: Test exit cell formula.**

```rust
#[test]
fn departing_uses_gamemd_exit_cell_formula() {
    // refinery at (10, 20). Expected exit:
    //   (10*256 - 0x80) / 256 = (2560 - 128) / 256 = 9
    //   (20*256 + 0x80) / 256 = (5120 + 128) / 256 = 20
    let cell = refinery_exit_cell(10, 20);
    assert_eq!(cell, (9, 20));
}
```

**Step 5: Test exit facing.**

```rust
#[test]
fn departing_snaps_facing_to_0x47() {
    // Setup: harvester at exit cell, dock_phase = Departing, no movement.
    // Tick: phase_departing detects arrival; facing = 0x47.
    // Assert: entity.facing == 0x47, miner.state == SearchOre.
}
```

**Step 6: Verify**

Run: `cargo test -p ra2 --lib miner_tests`
Expected: all 5 new tests pass.

**Step 7: Commit.**
Commit message: `miner_tests: add coverage for 4-phase FSM, bale events, exit cell`

---

### Task 19: Integration test — full dock cycle

**Why:** End-to-end test exercising the entire collapsed FSM. Catches inter-task wiring bugs.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs)

**Step 1: Write the integration test.**

```rust
#[test]
fn full_dock_cycle_war_miner() {
    // 1. Spawn refinery at (10, 20) with Storage=200.
    // 2. Spawn War Miner near refinery with cargo_full = 40 bales × 25.
    // 3. Issue ForcedReturn on the miner.
    // 4. Tick simulation until miner is back in SearchOre state.
    // 5. Assert:
    //    - sim.bale_events count == 40 (one per bale)
    //    - house credits increased by exactly 1000 (no purifier)
    //    - miner final position is at exit cell (origin + (-0x80, +0x80) leptons)
    //    - miner final facing == 0x47
    //    - dock_reservations is empty (released)
}
```

**Step 2: Verify**

Run: `cargo test -p ra2 --lib full_dock_cycle_war_miner`
Expected: PASS.

**Step 3: Commit.**
Commit message: `miner_tests: full dock cycle integration test`

---

### Task 20: Remove `bypass_grid` workaround

**Why:** Once Tasks 3-4 land, the dock pad cell (rx+3, ry+1) is no longer in the path/occupancy grid (RemoveOccupy1=3,1). `bypass_grid=true` is no longer needed.

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs)

**Step 1: Verify path-grid stamping is correct.**

In-game test: build a refinery, observe in debug overlay that (rx+3, ry+1) is walkable, (rx-1, ry) and (rx-1, ry-1) are blocked.

**Step 2: Remove `bypass_grid = true` writes.**

Two sites in `miner_dock_sequence.rs`:
- In `phase_approach` after `try_reserve` grant (just `issue_direct_move` remains)
- In `phase_departing` (just `issue_direct_move` remains)

**Step 3: Run integration test.**

Run: `cargo test -p ra2 --lib full_dock_cycle_war_miner`
Expected: still PASS.

**Step 4: Visual smoke test.**

Run game, dock a harvester. Confirm it drives onto and off the pad correctly.

**Step 5: Commit.**
Commit message: `miner_dock: drop bypass_grid workaround (RemoveOccupy makes pad cell walkable)`

---

### Task 21: Rewrite Part 2 of `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md`

**Why:** Existing Part 2 documents the wrong gamemd function (slave manager FSM, not refinery dock). Misleads future readers.

**Files:**
- Modify: [ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md](../../../ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md)

**Step 1: Replace Part 2 entirely.**

The existing Part 2 ("Refinery Dock Queue System") incorrectly attributes `FUN_006AF6C0` to refinery docking. Replace with:

- Identifies the correct entry point: `UnitClass::Mission_Deploy_Building` at 0x73D630
- Summarizes the 4-state inner FSM (cases 0/1/3/4)
- References [REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md) for full details
- Adds a header at the top: "**CORRECTION 2026-05-06: Original Part 2 conflated this with SlaveManagerClass::AI_Update. Rewritten.**"

**Step 2: Keep Part 1 (Hospital/Armory/RepairPad timer FSM) unchanged.**

Part 1 covers different building types and is verified correct.

**Step 3: Commit.**
Commit message: `docs: rewrite refinery dock section in BUILDING_DOCK_AND_HEAL_STATE_MACHINES (was wrong target function)`

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-06-refinery-dock-gamemd-parity-design.md](2026-05-06-refinery-dock-gamemd-parity-design.md)
- **Ghidra reports cited:**
  - [ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md)
  - [ra2-rust-game-docs/BUILDING_ANIM_STATE_MACHINE.md](../../../ra2-rust-game-docs/BUILDING_ANIM_STATE_MACHINE.md)
  - [ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md](../../../ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md) (Part 2 to rewrite — Task 21)
- **gamemd.exe addresses (kept here, not in code comments):**
  - `0x73D630` — `UnitClass::Mission_Deploy_Building` (harvester dock FSM)
  - `0x73E5E0` — `Mission_Harvest`
  - `0x451750` — `BuildingClass::SetAnimSlotImage`
  - `0x451890` — `BuildingClass::CreateAnimForSlot`
  - `0x4509D0` — `BuildingClass::UpdateAnimation`
  - `0x4593A0` — `BuildingClass::UndockUnit`
  - `0x459900` — vtable+0x468 particle emitter
  - `0x460A6C` — Refinery flag write (`+0x16BB`)
  - `0x45FE50` — `BuildingTypeClass::ReadINI`
  - Const at `0x007E27F8` — IEEE-754 `900.0`
- **INI keys consumed:**
  - rulesmd.ini `[GAREFN]`: DockUnload, Refinery, Storage, RefinerySmokeOffsetOne/Two, RefinerySmokeFrames, RefinerySmokeParticleSystem
  - artmd.ini `[GAREFN]`: Foundation, QueueingCell, AddOccupy1/2, RemoveOccupy1, ActiveAnim/Two/Three/Four, SpecialAnim
  - artmd.ini `[GAREFNOR]`: LoopCount=1, LoopEnd=19, Rate=200
  - rulesmd.ini `[General]`: HarvesterDumpRate (defaulted), PurifierBonus, ConditionYellow
- **Repo patterns mirrored:**
  - [src/sim/world/mod.rs:188-340](../../src/sim/world/mod.rs#L188) — `Simulation` struct + `sound_events`/`fire_events` queues
  - [src/sim/miner/miner_system.rs::tick_miners](../../src/sim/miner/miner_system.rs) — two-phase snapshot pattern
  - [src/rules/ruleset.rs::merge_art_data](../../src/rules/ruleset.rs) — INI merge pattern
  - [src/sim/production/production_tech.rs::foundation_dimensions](../../src/sim/production/production_tech.rs) — neighbor for `building_footprint_cells`
  - [src/sim/pathfinding/core.rs::block_building_footprint](../../src/sim/pathfinding/core.rs) — extended in Task 4
- **Project memory referenced:**
  - `feedback_brainstorm_before_implement.md`
  - `project_scale_target.md` (FIFO scale-exception)
  - `feedback_parity_bar.md` (drives Path B)
  - `feedback_no_engine_refs_in_comments.md` (Ghidra addresses stay here)
- **Recent relevant commits:**
  - `0101a64` — particles: TechnoType particle fields to ObjectType (powers Task 15)
  - `1f6a1bc` — miner: per-unit Storage + fractional HarvesterDumpRate (Task 10 builds on this)
