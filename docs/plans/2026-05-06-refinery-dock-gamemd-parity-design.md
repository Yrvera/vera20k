# Refinery Dock — gamemd Parity Rewrite Design

**Date:** 2026-05-06
**Status:** approved by user (Path B), ready for `/write-plan`
**Predecessor research:**
- [REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md](../../ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md) (this session — slot 7/10/8 mapping verified, NEW storage-tier finding)
- [BUILDING_ANIM_STATE_MACHINE.md](../../ra2-rust-game-docs/BUILDING_ANIM_STATE_MACHINE.md) (21-slot table)
- [BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md](../../ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md) (Part 2 needs rewrite — wrong target function)

## Goal

Bring the harvester refinery dock state machine to ≥95% parity with gamemd.exe's `UnitClass::Mission_Deploy_Building` (0x73D630), eliminating the visible "spinning on the pad" rotation phases, fixing the exit cell formula, applying the Ore Purifier bonus per-bale, wiring the per-bale `SpecialAnim` + particle bursts, parsing `AddOccupy/RemoveOccupy` to drop the `bypass_grid` workaround, and (additively) fixing the storage-tier display so refineries show one ore tower instead of four overlapping.

## Architecture Context

### Current implementation

Refinery docking lives in `sim/miner/`. The flow:

```
tick_resource_economy → tick_miners → process_miner
   match miner.state:
     SearchOre  → handle_search_ore
     MoveToOre  → handle_move_to_ore
     Harvest    → handle_harvest
     ReturnToRefinery → handle_return
     Dock       → handle_dock_sequence  ← 7-phase RefineryDockPhase machine
     Unload     → handle_unload         ← DEAD CODE (never reached on production paths)
     WaitNoOre  → handle_wait_no_ore
     ForcedReturn → handle_forced_return
```

The dock state machine in [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) has 7 phases:

```
RefineryDockPhase: Approach → WaitForDock → RotateToPad → EnterPad
                 → TurnOnPad → Unloading → ExitPad → (back to SearchOre)
```

This shape was modeled after the wrong gamemd function (`SlaveManagerClass::AI_Update`, not the harvester dock). Three of the seven phases — `RotateToPad`, `TurnOnPad`, and the explicit `WaitForDock` parking — are fabricated mechanics that don't exist in gamemd's actual dock FSM (`UnitClass::Mission_Deploy_Building`).

Reservations live in `sim::production::dock_reservations` ([src/sim/miner/miner_dock.rs](../../src/sim/miner/miner_dock.rs)) — a `DockReservations` struct keyed by refinery stable ID, single occupant per dock, with a FIFO `VecDeque` for queued miners. **gamemd has no queue** — multiple harvesters loiter and retry linkage informally — but our deterministic FIFO is an intentional scale-exception divergence (per project memory `project_scale_target.md`: 30 players, multiple harvesters per refinery, must replay-deterministically).

INI data flows: `rules/art_data.rs` parses art.ini, `rules/object_type.rs` defines per-object data, `rules/ruleset.rs:1396+` merges art into objects. Foundation is parsed as a `WxH` string and converted via `production::foundation_dimensions(foundation: &str) -> (u16, u16)`. **`AddOccupy*` and `RemoveOccupy*` are not parsed.** The path grid stamps building footprints using only the rectangle. For GAREFN this incorrectly blocks the SE-corner cell (`rx+3, ry+1`) which gamemd's `RemoveOccupy1=3,1` makes walkable — the harvester pad cell. The current code papers over this with `bypass_grid=true` on the harvester's move-to-pad command.

Renderer uses `dock_active_anim: bool` on `GameEntity` ([src/sim/game_entity.rs:202](../../src/sim/game_entity.rs#L202)) to gate ActiveAnim looping in [src/app_instances/shp.rs:529](../../src/app_instances/shp.rs#L529). gamemd does not gate ActiveAnim on dock state — it loops unconditionally while the building is alive. The boolean's purpose is misconceived.

### Existing patterns this design follows

- **Two-phase snapshot pattern** for sim ticks (snapshot all miners → process deterministically → write back) — already used in `tick_miners`. New code preserves this.
- **Event queues for sim → render communication** — `sim.sound_events: Vec<SimSoundEvent>` is the existing pattern for emitting one-shot events from sim to render/audio. New `bale_events` follows the same shape.
- **Per-tick reservation cleanup** — `DockReservations::cleanup_dead(&alive_set)` runs at the top of `tick_miners`. Unchanged.
- **Art.ini → ObjectType merge** — `rules/ruleset.rs::merge_art_data` runs once at load. New keys (`AddOccupy*`, `RemoveOccupy*`) merge through this same path.

### Sim layering check

All sim changes stay in `sim/` and `rules/`. Renderer changes in `app_instances/shp.rs` consume sim-emitted events; no upstream dependency. The sim/ → render/ direction is preserved. ✅

## Impact Analysis

### Files modified

| File | Change | Risk | LOC delta |
|---|---|---|---|
| [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs) | Collapse `RefineryDockPhase` enum 7→4 variants | Med | -10 |
| [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) | Full rewrite: 4 phase handlers, fix exit cell, emit bale events | High | -50 net |
| [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs) | Remove dead `MinerState::Unload` branch | Low | -60 |
| [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) | Mechanical rename + delete fabricated-phase tests + add new ones | Med | ±100 |
| [src/sim/components.rs](../../src/sim/components.rs) | Remove `dock_active_anim`, add bale event queue type | Low | ±20 |
| [src/sim/world/mod.rs](../../src/sim/world/mod.rs) (or wherever events live) | Add `bale_events: Vec<BaleDepositEvent>` | Low | +30 |
| [src/sim/ai.rs:1071](../../src/sim/ai.rs#L1071) | Remove `MinerState::Unload` arm in test scaffolding | Low | -5 |
| [src/rules/art_data.rs:245+](../../src/rules/art_data.rs) | Parse `AddOccupy1..N`, `RemoveOccupy1..N` | Low | +40 |
| [src/rules/object_type.rs:295](../../src/rules/object_type.rs) | Add `add_occupy: Vec<(i16,i16)>`, `remove_occupy: Vec<(i16,i16)>` | Low | +10 |
| [src/rules/ruleset.rs:1396+](../../src/rules/ruleset.rs) | Merge add/remove from art entry into object | Low | +10 |
| [src/sim/production/production_tech.rs:562](../../src/sim/production/production_tech.rs) | New `building_footprint_cells()` helper | Med | +30 |
| Path-grid stamping (callers of `foundation_dimensions`) | Use `building_footprint_cells()` for stamping | High | varies |
| [src/app_instances/shp.rs:529](../../src/app_instances/shp.rs#L529) | Remove `dock_active_anim` gating; storage-tier select for ActiveAnim*; consume bale events for SpecialAnim + particles | Med | +60 |

Total: ~7-9 hours of work, ~400 lines net change.

### Risk areas

1. **Path-grid stamping change** has the highest blast radius — every building placement uses it. If we get the new footprint helper wrong, building placement breaks for ALL buildings, not just refineries. Mitigation: extensive unit tests on `building_footprint_cells()` covering rectangle-only, AddOccupy-only, RemoveOccupy-only, and combined cases.
2. **`RefineryDockPhase` enum change is not snapshot-backwards-compatible.** Any saved replays referencing the old variants will fail to deserialize. Acceptable — replay system is in-flight and we own the format.
3. **Storage-tier formula is UNVERIFIED.** The design includes the storage-tier display fix but the exact tier formula (`floor(stored × 4 / max_storage)` vs threshold table) needs Ghidra spot-check before implementation. If the formula is wrong, the visual will switch tiers at the wrong levels.
4. **`Type+0x16A8` identity is UNVERIFIED.** This gates whether slot 10 fires for refineries. If it turns out refineries DO set this flag, our per-bale SpecialAnim trigger may need extra handling.
5. **First-bale jitter (`Random(0,2) × 30`)** — gamemd seeds the bale gate with random jitter on Unlimbo. Skipping this introduces a tiny parity drift (multiple simultaneous arrivals would pulse-sync). We're not implementing it; flagged as known minor drift.

### Tick-ordering impact

`bale_events` are emitted during `tick_miners` (currently called inside `tick_resource_economy`). Renderer reads them after sim tick completes. No re-ordering of sim phases needed.

### Determinism / state hash impact

- `RefineryDockPhase` change: state hash inputs include miner phase. New variants must be added to the hash function. ✅ noted in checklist.
- `bale_events` are transient (cleared each frame after render); not in state hash.
- `add_occupy`/`remove_occupy` on ObjectType are static rules data; not in state hash.
- `dock_active_anim` removal: was in state hash inputs (it's on GameEntity). Removal is safe — no behavioral observers in sim/.

## Chosen Approach

**Approach 2 (Path B):** scoped parity rewrite. Collapse 7 phases → 4, parse AddOccupy/RemoveOccupy, fix exit cell formula, per-bale purifier, wire slot 10 + particle bursts via bale events, fix storage-tier display, drop `bypass_grid` workaround. Keep deterministic FIFO `DockReservations` as scale-exception divergence.

Rejected alternatives:
- **Approach 1 (minimal collapse):** only collapses enum, leaves all other bugs. Fails the 99% parity bar — multiple visible bugs unfixed.
- **Approach 3 (full gamemd literal, drop FIFO):** replicates informal loiter-and-retry queueing. Fails determinism + scale targets. CLAUDE.md scale-exception clause applies.

## Tiny-Detail Ledger

The parity-relevant details the implementation must preserve. Each cites source.

### FSM behavior

| # | Detail | Source |
|---|---|---|
| 1 | gamemd has 4 distinct dock states (cases 0/1/3/4 in `Mission_Deploy_Building`), not 7 | [GHIDRA 0x73D630] |
| 2 | No body rotation step before driving onto pad — locomotor handles facing implicitly | [GHIDRA 0x73D630, no Set_Facing calls in approach path] |
| 3 | No 180° pivot on pad — orientation is whatever the path left it | [GHIDRA 0x73D630] |
| 4 | Linkage (`unit+0xB9 = building`) is set by `FUN_004595C0` when unit physically reaches pad cell, NOT when approach begins | [GHIDRA 0x73D630 inner-branch prelude] |
| 5 | Single-slot dock: building has one pointer at `+0x2E4`, no queue array on building side | [GHIDRA UndockUnit 0x4593A0] |

### Per-bale timing

| # | Detail | Source |
|---|---|---|
| 6 | Per-bale gate: `HarvesterDumpRate × 900.0` frames | [GHIDRA: const 0x408C200000000000d at 0x007E27F8] |
| 7 | Default `HarvesterDumpRate = 0.016` minutes → 14.4 frames per bale at 15 fps | [ini: rulesmd.ini General + GHIDRA] |
| 8 | Per-bale order within a tick: particle emit → SetAnimSlotImage(10) → Storage RemoveAmount → Add_Tiberium_Credits(base) → Add_Tiberium_Credits(purifier bonus) → reset gate | [GHIDRA 0x73D630 state 3] |
| 9 | Purifier bonus is per-bale, inline, NOT batched at end-of-load | [GHIDRA 0x73D630] |
| 10 | First-bale jitter `Random(0,2) × 30` frames seeded by Unlimbo — UNVERIFIED whether it affects regular dock cycles. **Not implementing; flagged as known minor drift.** | [GHIDRA 0x737C4C, 0x7371E3] |

### Cell math

| # | Detail | Source |
|---|---|---|
| 11 | Pad cell from art.ini `DockingOffset0=` if defined; else fallback `(rx+w-1, ry+h/2)` | [doc REFINERY_DOCK_ANIM_SLOTS §1, ini artmd.ini GAREFN] |
| 12 | Lepton→cell rounding: `(coord + 128) / 256` (toward nearest cell center) | [src/sim/miner/miner_dock_sequence.rs:71] |
| 13 | Exit cell = `building_origin_lepton + (-0x80, +0x80)` — NOT foundation-derived | [GHIDRA UndockUnit 0x4593A0] |
| 14 | Exit facing snap = `0x47` on undock arrival | [GHIDRA UndockUnit] |
| 15 | Undock locomotor speed set to `1.0` via vtable+0x544 | [GHIDRA UndockUnit] |
| 16 | GAREFN footprint: 4×3 rectangle + AddOccupy(-1,0) + AddOccupy(-1,-1) + RemoveOccupy(3,1). The (3,1) cell is the dock pad — must be walkable in the path grid. | [ini: artmd.ini GAREFN] |

### Animation

| # | Detail | Source |
|---|---|---|
| 17 | Slot 7 (PreProductionAnim) call on dock arrival fires unconditionally for refineries; no-op on stock GAREFN/NAREFN (empty art name short-circuits SetAnimSlotImage) | [GHIDRA 0x451750] |
| 18 | Slot 8 (ProductionAnim) call on completion fires unconditionally for refineries; no-op on stock GAREFN/NAREFN | [GHIDRA 0x451750] |
| 19 | Slot 10 (SpecialAnim = GAREFNOR) fires per bale, **destroyed and recreated** every pulse | [GHIDRA CreateAnimForSlot 0x451890 step 8] |
| 20 | GAREFNOR is 20 frames @ 200ms = 4s nominal. Per-bale interval is 14.4 frames = 0.96s. Anim never plays past frame ~5 during continuous unload. | [ini artmd.ini GAREFNOR] |
| 21 | ActiveAnim/Two/Three/Four (slots 3-6) loop continuously, INDEPENDENT of dock state | [GHIDRA BuildingClass::UpdateAnimation 0x4509D0] |
| 22 | Storage-tier display: only ONE of slots 3-6 is active at any moment, indexed by storage fill level. Formula UNVERIFIED — suspected `floor(stored × 4 / max_storage)`. **To verify before implementation.** | [GHIDRA 0x4509D0 sites 0x450E0D, 0x450F99] |
| 23 | Slot-7 second arg = `health <= Rules+0x1700` (ConditionYellow as double, default 0.5) → selects damaged variant | [GHIDRA 0x73E08E] |

### Particles

| # | Detail | Source |
|---|---|---|
| 24 | 4 particles spawn per bale at offsets from `Type+0x7CC/0x7D8/0x7E4/0x7F0` | [GHIDRA FUN_00459900] |
| 25 | INI keys: `RefinerySmokeOffsetOne/Two/Three/Four` | [ini rulesmd.ini GAREFN] |
| 26 | Allied refinery defines One+Two only; Three+Four default to (0,0,0) and overlap with origin | [ini rulesmd.ini GAREFN] |
| 27 | Particle system: `RefinerySmokeParticleSystem=SmallGreySSys` (Allied) | [ini] |
| 28 | Particle frame count: `RefinerySmokeFrames=50` (Allied) | [ini] |
| 29 | Particle spawn order: emitter fires BEFORE SetAnimSlotImage in per-bale block | [GHIDRA 0x73D630 state 3 ordering] |

### INI defaults

| # | Detail | Source |
|---|---|---|
| 30 | `Refinery=yes` sets `BuildingTypeClass+0x16BB`; gates slot-3-6 storage-tier and slot-8 calls | [GHIDRA 0x460A6C] |
| 31 | `DockUnload=yes` sets `BuildingTypeClass+0x16B3`; gates dock acceptance | [doc BUILDING_DOCK_AND_HEAL_STATE_MACHINES §0] |
| 32 | `ConditionYellow=0.5` (Rules+0x1700) — damage threshold for damaged-variant anim | [GHIDRA 0x451750] |
| 33 | `PurifierBonus` parsed at Rules+0xF3C; multiplier applied per bale | [GHIDRA 0x73D630] |

### UNVERIFIED items (must resolve before implementation)

- **Storage-tier formula** (#22) — quick `/re-investigate storage-tier formula` before implementation Task 1.
- **`Type+0x16A8` identity** — sources conflict (HasStorage vs HasTurretAnim). Determine via `/re-investigate Type+0x16A8 INI key`.

### Known intentional divergences (NOT bugs)

- **FIFO dock reservations** vs gamemd's informal loiter-retry. Scale-exception per CLAUDE.md.
- **First-bale jitter not implemented.** Cosmetic, hard to verify, minor drift.
- **Building-side undock vs unit-side undock.** We trigger the exit move from the unit's FSM; gamemd fires `UndockUnit` from the building. No player-visible difference; saves a building-side observer.

## Design

### Components

#### 1. Phase enum collapse

```rust
// src/sim/miner/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
         serde::Serialize, serde::Deserialize)]
pub enum RefineryDockPhase {
    /// Pathing toward QueueingCell while polling DockReservations.try_reserve()
    /// each tick. On grant: immediately re-target pad cell, transition to Linked.
    /// Maps to gamemd outer FSM (states 0/1/3, no link yet).
    #[default]
    Approach,

    /// Reservation granted; driving onto pad cell with bypass_grid (until #34
    /// AddOccupy/RemoveOccupy parsing lands; then bypass_grid removed). On
    /// arrival: emit dock-arrival sound, set display_type_override = UnloadingClass,
    /// init unload_timer. Maps to gamemd inner FSM states 0+1.
    Linked,

    /// Per-bale deposit pulse. Each bale emits BaleDepositEvent. On cargo empty:
    /// release reservation, clear display_type_override, transition to Departing.
    /// Maps to gamemd inner FSM state 3.
    Unloading,

    /// Drive to exit cell at building_origin + (-0x80, +0x80) leptons.
    /// On arrival: snap facing 0x47, return to SearchOre. Maps to gamemd inner
    /// FSM state 4 + UndockUnit.
    Departing,
}
```

Removed variants: `WaitForDock`, `RotateToPad`, `EnterPad`, `TurnOnPad`, `ExitPad`. (Net: 7 variants → 4.)

#### 2. Bale event queue

```rust
// src/sim/components.rs (or new src/sim/bale_event.rs if it grows)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BaleDepositEvent {
    /// Refinery stable_id. Renderer uses to look up building position + INI data.
    pub building_id: u64,
    /// Tick when emitted (for ordering / debugging).
    pub tick: u64,
}

// Added to Simulation struct alongside existing `sound_events: Vec<SimSoundEvent>`:
//   pub bale_events: Vec<BaleDepositEvent>,
// Cleared each frame after renderer drains it.
```

Renderer consumption ([src/app_instances/shp.rs](../../src/app_instances/shp.rs)) iterates `bale_events` to:
1. Trigger `SpecialAnim` (slot 10) on the building — adds to existing `BuildingAnimOverlays` one-shot system
2. Spawn 4 particles at `RefinerySmokeOffsetOne/Two/Three/Four` from rules

#### 3. Footprint helper

```rust
// src/sim/production/production_tech.rs (extension)
/// Returns the actual occupied cells for a building, applying AddOccupy and
/// RemoveOccupy to the rectangular foundation. Cells outside [0, u16::MAX]
/// after offset application are dropped.
///
/// `add_occupy` / `remove_occupy` are signed cell deltas from the building's
/// origin (rx, ry). Negative values extend left/up.
pub fn building_footprint_cells(
    origin_rx: u16,
    origin_ry: u16,
    foundation: &str,
    add_occupy: &[(i16, i16)],
    remove_occupy: &[(i16, i16)],
) -> Vec<(u16, u16)> { ... }
```

Replaces direct usage of `foundation_dimensions()` for path-grid stamping. The dimensions function stays — still used by callers that genuinely want the rectangle (e.g., placement preview UI).

#### 4. ObjectType extension

```rust
// src/rules/object_type.rs
pub struct ObjectType {
    // ... existing fields ...
    /// Cells added to the rectangular foundation. From art.ini AddOccupy1..N.
    pub add_occupy: Vec<(i16, i16)>,
    /// Cells removed from the rectangular foundation. From art.ini RemoveOccupy1..N.
    pub remove_occupy: Vec<(i16, i16)>,
}
```

Parsed in `art_data.rs::ArtEntry` and merged in `ruleset.rs::merge_art_data`.

#### 5. Renderer changes

```rust
// src/app_instances/shp.rs (around line 529)
//
// CURRENT:
//   if dock_active_anim && matches!(anim.kind, BuildingAnimKind::Active) {
//       looping_frame(anim, idle_anim_elapsed_ms)  // force-loop ActiveAnim
//   } else if anim.loop_count < 0 { ... }
//
// NEW:
//   - Drop dock_active_anim gating entirely.
//   - For BuildingAnimKind::Active on refineries: select ONE of the four
//     ActiveAnim variants by storage tier index (0..3). Other three skip render.
//   - SpecialAnim driven by BaleDepositEvent (already exists in
//     BuildingAnimOverlays one-shot machinery — just feed it from the new queue).
```

Storage tier index is computed by sim and stored on the building entity (e.g., `building.storage_tier: u8`). Renderer reads, picks `is_primary` for tier 0, `ActiveAnimTwo` for tier 1, etc. (UNVERIFIED formula — see ledger #22.)

### Interfaces / Contracts

**New public APIs:**
- `building_footprint_cells(origin_rx, origin_ry, foundation, add_occupy, remove_occupy) -> Vec<(u16, u16)>` — replaces ad-hoc footprint stamping
- `BaleDepositEvent { building_id, tick }` — new event type
- `Simulation::bale_events: Vec<BaleDepositEvent>` — new event queue
- `ObjectType::add_occupy`, `ObjectType::remove_occupy` — new fields
- `GameEntity::storage_tier: u8` (refineries only) — new field

**Removed APIs:**
- `RefineryDockPhase::{WaitForDock, RotateToPad, EnterPad, TurnOnPad, ExitPad}` (5 variants)
- `GameEntity::dock_active_anim` field
- `MinerState::Unload` arm in `process_miner` dispatch (state still exists in enum for `ai.rs` reference; mark for future removal but don't break the variant)

**Modified APIs:**
- `RefineryDockPhase` (4 variants total, 3 with same names but new semantics: Approach absorbs WaitForDock+RotateToPad logic; Linked absorbs EnterPad+TurnOnPad; Departing replaces ExitPad)
- Path-grid stamping callers now consume `building_footprint_cells()`

### Data Flow

```
INI load:
  art.ini AddOccupy/RemoveOccupy → ArtEntry → merge_art_data → ObjectType.add_occupy/remove_occupy

Tick (per miner, in Dock state):
  handle_dock_sequence(snap):
    match snap.miner.dock_phase:
      Approach:
        if !is_adjacent_or_at(queue_cell) && !has_movement_target:
          issue_path_to(queue_cell)
        if try_reserve(refinery_sid, miner_sid):
          issue_path_to(pad_cell)  [bypass_grid removed when AddOccupy/RemoveOccupy lands]
          dock_phase = Linked

      Linked:
        if at_pad_cell:
          emit DockArrivalSound
          set display_type_override = UnloadingClass
          init unload_timer
          dock_phase = Unloading

      Unloading:
        if cargo.empty():
          release_reservation()
          clear display_type_override
          dock_phase = Departing
          return
        if unload_timer expired:
          bale = cargo.pop()
          credits += bale.value
          if owner has Purifier: credits += bale.value * purifier_bonus_pct / 100  ← per-bale
          push BaleDepositEvent { building_id, tick }
          unload_timer += unload_tick_interval

      Departing:
        if !has_movement_target:
          exit_cell = building_origin_lepton + (-0x80, +0x80) leptons → cell
          issue_path_to(exit_cell)
        if at_exit_cell:
          facing = 0x47
          dock_phase = Approach (reset for next cycle)
          state = SearchOre

Renderer (per frame, after sim tick):
  for event in sim.bale_events:
    building = entity_lookup(event.building_id)
    trigger SpecialAnim slot on building.overlays
    for offset in building.refinery_smoke_offsets (One..Four):
      spawn particle at building.position + offset
  sim.bale_events.clear()

  for building in render_pass:
    if building.is_refinery:
      tier = building.storage_tier  ← computed each tick from current vs max storage
      render only ActiveAnim[tier] (skip the other three)
```

Path grid (build / rebuild — typically at world load, plus on building placement):
```
for building in entities (structures):
  cells = building_footprint_cells(building.rx, building.ry,
                                    building.foundation,
                                    building.add_occupy,
                                    building.remove_occupy)
  for cell in cells:
    path_grid.mark_blocked(cell)
```

### Error Handling

- **Refinery destroyed mid-dock:** existing path — `dock_state.dock_building_id` resolves to None → abort to SearchOre. Same for the new 4-phase machine.
- **Reservation grant race:** `try_reserve()` is idempotent for the current occupant. If a miner's reservation is canceled mid-Approach (e.g., refinery sold), the next Approach tick re-tries; if still no refinery available, transitions to SearchOre.
- **Exit cell unreachable:** if the computed exit cell is off-map (negative result from origin + (-0x80, +0x80)), clamp to (0, 0). Should never happen for legitimate map placements.
- **Bale event queue overflow:** `Vec` grows. Unlikely to be a problem at 30 players (bale rate is ~one per 14 frames per harvester; bounded by harvester count × bales/load).
- **Storage tier formula UNVERIFIED:** mark renderer to fall back to `ActiveAnim` (tier 0) if the formula returns out-of-range. Better than crashing.

### Testing Strategy

**Unit tests (pure logic):**
- `building_footprint_cells()` — 4 cases: rectangle only, +AddOccupy, +RemoveOccupy, both. Edge case: AddOccupy with negative deltas.
- Phase transitions in `handle_dock_sequence` — happy path Approach → Linked → Unloading → Departing → SearchOre.
- Per-bale purifier bonus arithmetic (current end-of-load vs new per-bale; verify total credits unchanged for default values).
- Exit cell formula: given building at (10, 20), exit cell = `((10*256 - 0x80) / 256, (20*256 + 0x80) / 256)` → (9, 20).

**Integration tests:**
- Full dock cycle: spawn refinery + harvester with full cargo → tick until cargo empty → verify exit move issued.
- Bale event emission: verify `bale_events` count matches cargo size after a full unload.
- Storage tier transitions: spawn refinery, deposit ore in chunks, verify `storage_tier` field updates correctly.

**RE-driven verification:**
- Compare against gamemd in-game: side-by-side video of harvester docking. Confirm no visible spin-on-pad. Confirm GAREFNOR plays per bale. Confirm smoke puffs visible at correct offsets. Confirm exit direction matches.

**Test count delta:**
- ~25 existing tests rename mechanically (WaitForDock → Approach, etc.)
- ~5 existing tests delete (assertions on rotation phases)
- ~6 new tests (footprint helper, exit cell, bale event emission, storage tier, per-bale purifier, AddOccupy parsing)

### Determinism considerations

- All sim math stays integer / fixed-point. New `bale_events` use `u64` IDs and `u64` ticks. ✅
- BTreeMap iteration order: `DockReservations` and `EntityStore` already use BTreeMap with stable_id keys. ✅
- New random sources: NONE. (First-bale jitter from gamemd is not implemented.)
- State hash: `RefineryDockPhase` enum included; `dock_active_anim` removed; `storage_tier` added; `bale_events` excluded (transient).
- Tick ordering: bale events emit during `tick_miners`. Renderer drains after sim tick completes. No re-ordering. ✅

## Architectural Decisions

### Patterns followed

- **Two-phase snapshot pattern** for sim ticks (existing in `tick_miners`).
- **Event queue for sim → render** (existing `sound_events` pattern).
- **INI merge through ruleset** (existing `merge_art_data` pattern).
- **Footprint helper instead of inline rectangle** (new pattern, but follows the same data-driven approach as `foundation_dimensions`).

### Patterns deviated from

- **Per-bale event emission** is new for this system. Previously, animation triggers came from one-shot overlays driven by component flags (`dock_active_anim`). Switching to event queue is more idiomatic for fire-and-forget pulses (matches gamemd's "do this once per pulse" pattern more naturally).

### Tech debt introduced

- **`MinerState::Unload` enum variant remains** even though no production code path leads to it. Removing it touches `ai.rs` test scaffolding and `Miner` serialization. Deferred — flagged for cleanup pass.
- **Storage tier formula remains UNVERIFIED at design time.** Plan task #1 should be a targeted Ghidra spot-check before any code is written.
- **`Type+0x16A8` identity unresolved.** Same — Ghidra spot-check first.
- **`bypass_grid` removal blocks on AddOccupy/RemoveOccupy task landing first.** Plan must order these correctly.

## Alternatives Considered

### Approach 1 — Minimal collapse

Just rename the enum variants. Leave purifier timing, cell formulas, anim handling, occupy parsing as-is.

**Rejected because:** fails 99% parity bar. The "spinning on pad" goes away but every other identified divergence stays. Path A from the original brainstorm round.

### Approach 3 — Full literal gamemd replication, drop FIFO

Drop `DockReservations` FIFO. Replicate gamemd's outer state 3 "loiter and retry adjacent cells" exactly.

**Rejected because:**
- Determinism harder (probe order matters; multiple harvesters race informally).
- O(N²) at scale — fails project memory `project_scale_target` (30 players, multiple harvesters per refinery, replay-deterministic).
- CLAUDE.md scale-exception clause explicitly authorizes this divergence.

### Approach 4 — Defer storage-tier fix to follow-up

Only fix the dock state machine + exit cell + per-bale purifier + AddOccupy/RemoveOccupy. Leave the four-overlapping-towers visual bug.

**Rejected because:** the storage-tier fix touches the same renderer code as the slot 10 SpecialAnim wiring. Doing them in one PR is cleaner. User chose "include" in clarifying-question A.

## Sources & References

- **Ghidra functions decompiled (this session):**
  - `0x73D630` (Mission_Deploy_Building, full FSM)
  - `0x451750` (SetAnimSlotImage)
  - `0x451890` (CreateAnimForSlot)
  - `0x4509D0` (BuildingClass::UpdateAnimation, partial — slots 3/4/5/6/10 paths)
  - `0x4593A0` (UndockUnit)
  - `0x459900` (vtable+0x468 particle emitter)
  - `0x460A6C` (Refinery flag write)
  - `0x45FE50` (BuildingTypeClass::ReadINI, slot table verification)
  - `0x73E5E0` (Mission_Harvest, dock entry)
- **Memory addresses verified:**
  - `0x007E27F8` = IEEE-754 double `900.0` (frame-per-minute constant)
  - Const `(-0x80, +0x80)` lepton offset for exit cell
  - `(rx, ry) → leptons → cell` rounding `(coord + 128) / 256`
- **Research docs:**
  - `ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md` (this session)
  - `ra2-rust-game-docs/BUILDING_ANIM_STATE_MACHINE.md` (parent doc, 21-slot table)
  - `ra2-rust-game-docs/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md` (Part 2 INCORRECT — to rewrite as separate task)
- **INI keys:**
  - `[GAREFN]` rules: DockUnload, Refinery, NumberOfDocks, Storage, RefinerySmokeOffsetOne/Two/Three/Four, RefinerySmokeFrames, RefinerySmokeParticleSystem
  - `[GAREFN]` art: Foundation, QueueingCell, AddOccupy1/2, RemoveOccupy1, ActiveAnim/Two/Three/Four, SpecialAnim
  - `[GAREFNOR]`, `[GAREFNL1..L4]` art (anim entries)
  - `[General]`: HarvesterDumpRate=0.016, HarvesterLoadRate, PurifierBonus, ConditionYellow=0.5
- **Repo code patterns:**
  - `src/sim/miner/miner_dock_sequence.rs` (current 7-phase impl)
  - `src/sim/world/sound_events.rs` (event queue pattern to mirror)
  - `src/rules/ruleset.rs::merge_art_data` (INI merge pattern)
  - `src/sim/production/production_tech.rs::foundation_dimensions` (helper to extend)
  - `src/app_instances/shp.rs:529` (renderer gating site)
- **Project memory:**
  - `feedback_brainstorm_before_implement.md` — this brainstorm satisfies the requirement
  - `project_scale_target.md` — justifies FIFO scale-exception
  - `feedback_parity_bar.md` — 99% parity goal drives Path B over Path A
  - `feedback_no_engine_refs_in_comments.md` — Ghidra addresses stay in this doc, NOT in Rust code comments

---

**Next step:** `/write-plan` to break this into ~15-20 implementation tasks with verification steps. Plan should order tasks: Ghidra spot-checks first (storage-tier formula, Type+0x16A8) → INI parsing (AddOccupy/RemoveOccupy) → footprint helper → path-grid wiring → enum collapse → phase handler rewrite → bale event queue → renderer changes → test cleanup → integration verification.
