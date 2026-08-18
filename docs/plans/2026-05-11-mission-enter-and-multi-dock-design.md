# Multi-Pad DockingOffset + Pad-Aware Airfield Reservation Design

## Goal

Close the player-visible parity gap where multi-pad airfields (GAAIRC = 4 pads, AMRADR = 4 pads) currently park every docking aircraft on the same offset, by parsing all `DockingOffset%d` entries and threading pad-index assignment through the aircraft dock state machine. Refinery / service depot / single-pad helipad consumers gain the same multi-pad-capable infrastructure but only ever use pad 0.

## Architecture Context

### Current state (pre-change)

Six entry-into-building FSMs exist in the codebase, each with its own `GameEntity` field and tick function. The three that touch dock geometry today:

1. **Harvester ↔ Refinery** ([src/sim/miner/](src/sim/miner/)): `MinerState::Dock` + `RefineryDockPhase` (Approach / Linked / Unloading / Departing). Reservation through single-slot [`DockReservations`](src/sim/miner/miner_dock.rs#L18). Reads single [`ObjectType.docking_offset`](src/rules/object_type.rs#L305-L308). NumberOfDocks=1 in retail.
2. **Vehicle ↔ Service Depot** ([src/sim/docking/building_dock.rs](src/sim/docking/building_dock.rs)): `DockState` + `DockPhase` (Approach / WaitForDock / EnterDock / Servicing / ExitDock). Single-slot via `depot_dock_reservations`. Uses building center, not `docking_offset` — NumberOfDocks=1.
3. **Aircraft ↔ Airfield / Helipad** ([src/sim/docking/aircraft_dock.rs](src/sim/docking/aircraft_dock.rs)): `AircraftMission::Docking { sub_state, airfield_id }` + `AircraftAmmo.dock_phase` (ReturnToBase / WaitForDock / Descending / Reloading / Launching). Multi-slot reservation via [`AirfieldDocks`](src/sim/docking/aircraft_dock.rs#L85-L204) tracking a *count* of occupants per airfield but **not which pad each occupies**. NumberOfDocks=4 for GAAIRC and AMRADR.

The art.ini parser at [src/rules/art_data.rs:272-279](src/rules/art_data.rs#L272-L279) reads only `DockingOffset0`. The art→rules merge at [src/rules/ruleset.rs:1630-1640](src/rules/ruleset.rs#L1630-L1640) propagates that single field. `number_of_docks: u8` is parsed at [object_type.rs:950](src/rules/object_type.rs#L950) but only the airfield path consumes it.

### gamemd behavior the design must reproduce

From the verified research:

- `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` loops `NumberOfDocks` times reading `DockingOffset%d` into `BuildingTypeClass+0x1788` (stride 12 bytes, 3 × int32 per entry). Verified live, Stage 1 audit.
- `RadioClass::Transmit_Radio_Impl @ 0x0065A970` cmd 2 (HELLO) does first-empty-slot scan of `Contacts[]@+0xE4`, writes target into the slot, returns the slot index implicitly via array position. Verified Stage 2.
- Aircraft sends radio cmd 0xE (CAN_DOCK?) → building writes the aircraft into `Contacts[]` and returns the pad cell coords via the building's vtable. Verified Stage 2 ([MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md) §3).
- Each pad's cell = building origin + lepton offset, converted into cell space with the standard `+128` half-cell rounding (matches existing [`refinery_pad_cell`](src/sim/miner/miner_dock_sequence.rs#L57-L74)).

Per CLAUDE.md "internals are not the spec — outputs are", we do not mirror gamemd's RadioClass + Mission_Enter + PerCellProcess split. We reproduce the observable outputs: visually distinct pad positions, correct queue/promote ordering, no two aircraft on the same pad.

## Impact Analysis

| File | Change | Risk |
|---|---|---|
| [src/rules/art_data.rs](src/rules/art_data.rs#L92-L95) | Replace `docking_offset: Option<(i32,i32,i32)>` with `pads: Vec<DockPad>` field. Parser loops 0..N reading `DockingOffset%d`. | LOW — single function |
| [src/rules/object_type.rs](src/rules/object_type.rs#L305-L308) | Same field swap. Default `Vec::new()`. | LOW |
| [src/rules/shp_vehicle_sequence.rs](src/rules/shp_vehicle_sequence.rs#L129) | Test fixture initializer — swap default. | TRIVIAL |
| [src/rules/ruleset.rs](src/rules/ruleset.rs#L1634-L1637) | Art→rules merge swaps to whole-vec merge. | LOW |
| [src/sim/miner/miner_dock_sequence.rs:57-74](src/sim/miner/miner_dock_sequence.rs#L57-L74) | `refinery_pad_cell()` reads `pads.first()`. | TRIVIAL |
| [src/sim/docking/aircraft_dock.rs:85-204](src/sim/docking/aircraft_dock.rs#L85-L204) | **`AirfieldDocks` becomes pad-keyed.** `try_reserve` returns `Option<u8>` (pad_index) instead of `bool`. Queue/promote/cancel logic updated. | MEDIUM |
| [src/sim/docking/aircraft_dock.rs:32-45](src/sim/docking/aircraft_dock.rs#L32-L45) | `AircraftAmmo.target_pad: Option<u8>` added. Serde derive picks it up. | LOW |
| [src/sim/docking/aircraft_dock.rs:290+](src/sim/docking/aircraft_dock.rs#L290) | `tick_aircraft_docks` uses per-pad cell for descent/launch via new helper. | MEDIUM |
| [src/sim/aircraft/mod.rs](src/sim/aircraft/mod.rs) | `AircraftMission::Docking { sub_state, airfield_id, pad_index: u8 }` adds `pad_index`. | LOW |
| **NEW** [src/sim/docking/pad_geometry.rs](src/sim/docking/pad_geometry.rs) | `pad_cell_for(origin, &DockPad) -> (u16, u16)` helper extracted from `refinery_pad_cell`. | LOW |
| Test suites | API-driven mechanical updates: `pads: vec![...]` instead of `docking_offset: Some(...)`. `AirfieldDocks` tests assert pad_index. | LOW |

**Save-game compatibility:** new field on `AircraftAmmo` and `AircraftMission::Docking` → save format changes. Pre-1.0 project; old saves don't need to load.

**Determinism:** `BTreeMap<u64, Vec<Option<u64>>>` keyed iteration is deterministic. First-empty-slot linear scan over a small vec is deterministic. No new floats, no new RNG draws, no HashMap.

**Blast radius:** confined to `sim/docking/aircraft_dock.rs`, two `rules/` parser files, one tiny touch on `miner_dock_sequence.rs`, and one new tiny module. No other `sim/` modules touched.

## Chosen Approach

Replace `Option<(i32,i32,i32)>` on `ObjectType` with `Vec<DockPad>`; extend `AirfieldDocks` to be pad-keyed (returning pad_index from `try_reserve`); thread pad_index through aircraft dock state machine. Refinery and service depot keep their existing FSMs unchanged — they read `pads.first()` for their (always single) pad. Miner FSM is not refactored; aircraft FSM gains one new field.

This is the **parity-only** scope. The user explicitly chose not to unify the six entry-into-building FSMs in this work, citing the parity bar (outputs not internals). The unification effort is logged as a deferred follow-up.

## Tiny-Detail Ledger

The implementation must reproduce each of these output-driving details. Implementation reviews and the eventual `/write-plan` should cross-check against this list.

**Multi-pad parser:**
- DockingOffset%d keys in art.ini, 0-indexed (`DockingOffset0` … `DockingOffset<NumberOfDocks-1>`). `[doc: BUILDING_DOCKING_SYSTEM §1.1]`
- Stride 12 bytes per entry, 3 × int32 (x, y, z) in leptons. `[doc: BUILDING_DOCKING_SYSTEM §1.1, verified Stage 1]`
- NumberOfDocks=N but K<N offsets specified: bytes are zero-initialized (not garbage). Vec entries for missing indices use `lepton_offset = (0,0,0)`. `[doc: BUILDING_DOCKING_SYSTEM verification audit 2026-05-11]`
- NumberOfDocks=0 or missing key: default to 1. `[code: src/rules/object_type.rs:950 unwrap_or(1).max(1)]`
- Pad lepton coordinate space: 256 leptons per cell.
- **Pad offset is BUILDING-CENTER-RELATIVE, not origin-relative.** Verified live in `BuildingClass::GetDockCoord @ 0x00447B20`: pad cell = `GetCoords() + DockingOffset[i]` where `GetCoords()` returns building geometric center. Cell conversion then uses `+128` half-cell rounding. `[Ghidra 0x00447B20 multi-pad branch, verified 2026-05-11]`
- A previous "+128 half-cell off origin" formula in our `refinery_pad_cell` was bugged but never fired in retail (all retail refineries have `DockingOffset0` commented out; NADEPT depot uses a different code path). Commit 3 fixes the formula. `[code: src/sim/miner/miner_dock_sequence.rs:64-66]`

**Pad-index allocation:**
- First-empty-slot scan across `Contacts[]`. Two simultaneous arrivals: the one processed first in iteration order (BTreeMap, stable) gets pad 0, the second gets pad 1. `[doc: MISSION_ENTER_CROSSWALK §1, verified Stage 2]`
- Allocation is per-building, independent across buildings. Two airfields' pad 0s do not collide.
- gamemd's "evict slot 0 on full" path is intentionally NOT implemented — our FIFO queue handles overflow more gracefully and matches user expectation. Documented divergence; player-invisible.

**Aircraft path:**
- GAAIRC = 4 pads; AMRADR = 4 pads; NAHPAD = GAHPAD = 1 pad. `[ini: rulesmd.ini]`
- After `try_reserve` returns `Some(pad_index)`, aircraft's `air_move_to` target = `pad_cell_for(building_origin, &pads[pad_index])`. NOT the building center.
- Descent triggered at distance ≤ 2 (existing `cell_distance` ≤ 2 check). `[code: src/sim/docking/aircraft_dock.rs:421]`
- Reload trigger: `AirMovePhase::Landed` AND ammo < max. `[code: src/sim/docking/aircraft_dock.rs:478-484]`
- `ReloadRate` ticks between ammo restores. `[ini: rules.ini ReloadRate → general.reload_rate_ticks]`
- Launch when ammo full: release pad via `AirfieldDocks::release(ac_sid)`, ascend. Existing behavior, threaded through new pad-aware release. `[code: src/sim/docking/aircraft_dock.rs:493-497]`

**Refinery / depot (pad 0 only, multi-pad-capable but not exercised in retail):**
- `pads.first().map(|p| p.lepton_offset)` replaces existing `docking_offset` reads. Output cell identical.
- Refinery exit facing 0x47, exit cell offset (-0x80, +0x80) leptons — unchanged. `[doc: HARVESTER_DOCK_UNLOAD §4]`
- HarvesterDumpRate 14.4 ticks/bale — unchanged. `[doc: HARVESTER_DOCK_UNLOAD §2.1]`
- HarvesterLoadRate 18 frames/bale — unchanged.

**Edge cases:**
- Airfield destroyed mid-dock → `AirfieldDocks::cleanup_dead` releases all pads, promotes queue. (Existing logic, extended to pad-aware.)
- Aircraft destroyed mid-dock → same.
- All N pads occupied + new aircraft → FIFO queue (existing behavior, kept).
- Pad cell inside foundation footprint → existing `AddOccupy` / `RemoveOccupy` art parsing handles cell occupancy; aircraft movement bypasses occupied cells during dock approach. `[code: src/rules/art_data.rs add_occupy / remove_occupy parsing]`
- Single-pad airfield (helipad) → `try_reserve` returns `Some(0)` for the one slot; old test cases pass.
- **Destination-must-be-a-building guard.** gamemd's `AircraftClass::Mission_Enter` state 7 checks `destination->vtable[+0x2C] == 6` (Abstract_Building) before calling the per-pad cell lookup; non-building destinations fall through to a generic GetCoords path. Our Rust equivalent: `find_nearest_airfield` already filters to `entity.category == EntityCategory::Structure` ([aircraft_dock.rs:238-240](src/sim/docking/aircraft_dock.rs#L238-L240)), so the multi-pad path only fires for structures. No design change needed; ledger entry only. `[live decompile of 0x00419C80 case 7, 2026-05-11]`

## Design

### Components

```
src/rules/object_type.rs
    pub struct DockPad {
        pub lepton_offset: (i32, i32, i32),
    }
    impl ObjectType {
        pub pads: Vec<DockPad>,        // replaces docking_offset
        pub number_of_docks: u8,       // unchanged
    }

src/rules/art_data.rs
    // Parser loop reads DockingOffset0..N-1, pushing to Vec.

src/rules/ruleset.rs
    // Art→rules merge takes whole vec instead of Option.

src/sim/docking/pad_geometry.rs (NEW, small)
    pub fn pad_cell_for(origin: (u16, u16), pad: &DockPad) -> (u16, u16)
    // Lepton→cell conversion with +128 half-cell rounding.
    // Single implementation, used by refinery_pad_cell and aircraft tick.

src/sim/docking/aircraft_dock.rs
    pub struct AirfieldDocks {
        slots: BTreeMap<u64, Vec<Option<u64>>>,        // af_sid → [pad_0_occ, pad_1_occ, ...]
        queues: BTreeMap<u64, VecDeque<u64>>,
        aircraft_to_pad: BTreeMap<u64, (u64, u8)>,     // ac → (af_sid, pad_index)
    }
    pub struct AircraftAmmo {
        // ...existing fields...
        pub target_pad: Option<u8>,                    // NEW
    }

src/sim/aircraft/mod.rs
    pub enum AircraftMission {
        // ...
        Docking { sub_state, airfield_id, pad_index: u8 },  // pad_index NEW
        DockedIdle { airfield_id, pad_index: u8 },          // pad_index NEW (for consistency)
    }
```

### Interfaces / Contracts

`AirfieldDocks` new public surface:

```rust
impl AirfieldDocks {
    /// Try to claim a pad. Returns Some(pad_index) on success, None if all pads full (aircraft is enqueued).
    pub fn try_reserve(&mut self, af_sid: u64, ac_sid: u64, num_pads: u8) -> Option<u8>;

    /// Look up which pad an aircraft is parked on, if any.
    pub fn pad_for(&self, ac_sid: u64) -> Option<(u64, u8)>;

    /// Release an aircraft's pad. Returns the next aircraft promoted from queue, if any.
    pub fn release(&mut self, ac_sid: u64) -> Option<u64>;

    /// Cancel a reservation or queue position.
    pub fn cancel(&mut self, ac_sid: u64);

    /// Remove dead entities (aircraft or airfields). Promotes queue as pads free.
    pub fn cleanup_dead(&mut self, alive: &BTreeSet<u64>);
}
```

`pad_cell_for` helper (single function):

```rust
/// Convert a building origin + foundation + DockPad's lepton offset into a cell (rx, ry).
///
/// CRITICAL: DockingOffset is **building-center-relative**, not origin-relative.
/// Mirrors `BuildingClass::GetDockCoord @ 0x00447B20` which adds the pad offset to
/// `GetCoords()` (building geometric center) before lepton→cell conversion. The
/// center is `origin + ((W-1)*128, (H-1)*128)` leptons.
///
/// Verified via live decompile 2026-05-11: a previous "+128 half-cell off origin"
/// formula in `refinery_pad_cell` was bugged but never fired in retail (all retail
/// refineries have `DockingOffset0` commented out; NADEPT is depot which doesn't
/// use this helper). Fixing it here for correctness.
pub fn pad_cell_for(origin: (u16, u16), foundation: (u16, u16), pad: &DockPad) -> (u16, u16) {
    let (rx, ry) = origin;
    let (w, h) = foundation;
    // Building geometric center offset (in leptons) from origin top-left cell.
    let center_off_x = (w as i32 - 1) * 128;
    let center_off_y = (h as i32 - 1) * 128;
    let (dx, dy, _) = pad.lepton_offset;
    // Lepton offset from origin cell's top-left → cell with +128 half-cell rounding.
    let cx = (center_off_x + dx + 128).div_euclid(256);
    let cy = (center_off_y + dy + 128).div_euclid(256);
    (
        (rx as i32 + cx).max(0) as u16,
        (ry as i32 + cy).max(0) as u16,
    )
}
```

### Data Flow

**Aircraft docks at airfield, multi-pad:**

```
Aircraft (ammo=0) → tick_aircraft_docks → find_nearest_airfield → got (af_sid, _, _)
    │
    ├─→ AircraftMission::Docking { sub_state: ReturnToBase, airfield_id, pad_index: ? }
    │   (pad_index temporarily undefined until reservation acquired)
    │
    ├─→ At distance ≤ 2: try_reserve(af_sid, ac_sid, num_pads)
    │   ├─→ Some(pad_idx): set target_pad = pad_idx, transition to Descending
    │   │   air_move_to = pad_cell_for(building_origin, &obj.pads[pad_idx])
    │   └─→ None: enqueued, sub_state stays WaitForDock
    │
    ├─→ Descending → Landed → Reloading (per-tick ammo restore)
    │
    └─→ ammo == max: release(ac_sid) → ascend, AircraftMission::None
```

**Refinery dock, single-pad (unchanged output):**

```
Miner → MinerState::Dock → RefineryDockPhase::Approach
    │
    ├─→ DockReservations::try_reserve (single-slot, unchanged)
    │
    ├─→ pad_cell_for(refinery_origin, &obj.pads[0]) → (rx, ry)
    │   (called via existing refinery_pad_cell helper, now thin wrapper around pad_cell_for)
    │
    └─→ rest of FSM unchanged
```

### Error Handling

- Building has `NumberOfDocks > 0` but `pads.is_empty()`: treat as 0 pads, can't dock. Aircraft falls through to "no airfield" path (existing `RESCAN_COOLDOWN_TICKS`). Log warning at parse time.
- `try_reserve` called with `num_pads = 0`: returns `None`. Aircraft enqueues and never gets promoted; cleanup_dead eventually removes from queue when the building dies.
- Aircraft's `target_pad` set but the airfield was destroyed before descent: existing `ReturnToBase` branch already re-resolves. Extend to also clear `target_pad`.
- Determinism check: BTreeMap iteration order is documented stable. Vec linear scan is deterministic. The first-empty-slot allocation gives reproducible pad-index assignments across replays.

No `Result` returns introduced; the design uses `Option` throughout to match the existing `aircraft_dock.rs` style. No new error types in `thiserror`.

### Testing Strategy

**Unit tests in `src/rules/art_data.rs`:**
- Parse GAAIRC art entry with `NumberOfDocks=4` + `DockingOffset0..3` → `pads.len() == 4`, each lepton_offset correct.
- Parse with `NumberOfDocks=4` but only `DockingOffset0` specified → `pads.len() == 4`, indices 1..3 have `(0,0,0)` (zero-initialized).
- Parse with `NumberOfDocks=0` (or missing) → `pads.len() == 0` or 1 default-empty entry (match retail behavior).

**Unit tests in `src/sim/docking/aircraft_dock.rs`:**
- 4-pad airfield: 4 aircraft, each gets a distinct pad_index in deterministic order (0, 1, 2, 3).
- 5th aircraft on 4-pad airfield queues; release of pad 1 promotes queue to pad 1.
- Single-pad helipad: 1 aircraft gets pad 0; 2nd queues.
- `cleanup_dead` for an airfield with 3 pads occupied → all 3 released, all queue entries cleared.

**Determinism replay test (new):**
- Spawn 2 aircraft simultaneously, run for N ticks, capture `pad_for` results.
- Re-run with same seed; assert pad assignments identical.

**Integration test (existing tests updated, not net-new):**
- `src/sim/miner/miner_tests.rs` — refinery harvest cycle still passes with `pads.first()` substitution.

**Manual smoke test:**
- Build GAAIRC, train 4 aircraft, watch each land on a visibly distinct pad.

### Migration order (5 commits)

| # | Commit | What | Test gate |
|---|---|---|---|
| 1 | Parse multi-pad alongside single | Add `pads: Vec<DockPad>` to ObjectType + parser; keep `docking_offset` field temporarily populated from `pads.first()` | All existing tests pass |
| 2 | Drop single-pad field | Remove `docking_offset`; consumers read `pads.first()`. Refinery + depot updated. | All existing tests pass with mechanical updates |
| 3 | Extract `pad_geometry::pad_cell_for` | Move the lepton→cell conversion into the new module. Refinery uses it. | Refinery tests still pass with identical output |
| 4 | Pad-keyed `AirfieldDocks` | `try_reserve` returns `Option<u8>`; `AircraftAmmo.target_pad` threaded; aircraft tick uses per-pad cell. AircraftMission::Docking gains pad_index. | New AirfieldDocks tests + updated aircraft_dock tests pass |
| 5 | Documentation + dead-code cleanup | Update module doc comments; remove any transitional code | All tests pass; `cargo clippy` clean |

Each commit independently passes `cargo test`. Each can be reverted without breaking the previous.

## Architectural Decisions

**Patterns followed:**
- INI keys parsed in `rules/`, merged via `art_data.rs` → `ObjectType` (same pattern as `add_occupy`, `damage_fire_offsets`, etc.).
- Reservation state lives in a coordinator (`AirfieldDocks`), not on `GameEntity` — matches existing project style for shared state.
- `BTreeMap` for determinism (no HashMap), `serde::Serialize/Deserialize` derives for save-game.
- File size discipline: aircraft_dock.rs already at ~650 lines; will grow ~50 lines. Acceptable. pad_geometry.rs is a new ~30-line file.
- `Option<u8>` instead of `bool` from `try_reserve` follows the existing pattern of using `Option<EntityId>` to mean "succeeded with value" elsewhere.

**Patterns deviated from:**
- None significant. The new module `sim/docking/pad_geometry.rs` is a small new file but matches the existing `sim/docking/` layout (sibling to `aircraft_dock.rs` and `building_dock.rs`).

**Tech debt introduced:**
- The 6 fragmented FSMs (passenger, miner refinery, service depot, aircraft, capture, c4) remain unchanged. The maintenance smell persists. Logged as deferred follow-up.
- `AircraftMission::Docking` and `AircraftAmmo.dock_phase` continue to coexist (same dual state-tracking pattern as today). Not unified in this work.

**Determinism contract:**
- BTreeMap keys (u64) — deterministic iteration.
- `Vec<Option<u64>>` first-empty linear scan — deterministic.
- No new RNG draws, no new floating-point math, no `HashMap`.
- `pad_cell_for` is pure integer arithmetic (+128 half-cell rounding).

## Alternatives Considered

**Option A1 — Full `Mission` enum on GameEntity mirroring gamemd's vtable dispatch.** Rejected. Per CLAUDE.md "internals are not the spec — outputs are." Adding a Mission enum reproduces gamemd's internal dispatch pattern with no parity gain. Anti-pattern: "new pattern for no reason."

**Option A2 — Trait-driven `DockIntent` component shared across consumers.** Rejected. The user explicitly chose parity-only scope, leaving FSMs alone. A shared trait would force every existing FSM to refactor. Logged as deferred follow-up.

**Option B2 — Keep single `docking_offset` field, add `extra_docking_offsets: Vec` for indices 1+.** Rejected. Two parallel data paths for the same concept; smell. The chosen `Vec<DockPad>` is one source of truth.

**Option B3 — `HashMap<u8, (i32,i32,i32)>` for pads.** Rejected. Determinism concern (HashMap iteration order). Vec is fine for a 4-pad ceiling.

**Option C2 — Unified `PadManager` replacing both `DockReservations` and `AirfieldDocks`.** Rejected. Conflicts with the "leave FSMs alone" scope; every consumer's call sites would change, widening the test surface unnecessarily. Logged as deferred follow-up.

**Option C3 — Per-entity `dock_pad_occupants: Vec<Option<EntityId>>` on `GameEntity`.** Rejected. Mirrors gamemd's `Contacts[]@+0xE4` layout most closely, but bloats GameEntity for the 99% of buildings that aren't dockable and forces every consumer to mutate the building entity to acquire a pad. The coordinator pattern is cleaner.

**Combined commit 1+2.** Considered. The user asked but ultimately approved the 5-step migration as-is. The no-op duplication step gives us a clean fallback if step 2 turns up unexpected consumers.

---

## Deferred follow-ups (logged but out of scope)

1. **FSM unification.** Sharing the "approach + dock-link + release" primitive across miner / aircraft / depot via a `DockIntent` trait or substrate module. The user explicitly excluded this from the current work; it's a clean-up, not a parity item.
2. **C4 scatter operator-precedence bug.** Stage 2 found that `world_orders.rs::queue_c4_post_detonation_scatter` uses `(tick >> 12 + 1) >> 1 & 7` which Rust parses as `(tick >> 13) >> 1 & 7 = (tick >> 14) & 7` because `+` binds tighter than `>>`. The binary formula is `((value >> 12) + 1) >> 1 & 7` where `value` comes from `this->RateTimer (+0x388).Current()`, not necessarily our `sim.tick`. Both the parens and the input value need a fix in a separate small commit.
3. **AI retreat-to-repair** triggered by `UnitClass::ReceiveDamage @ 0x00738664` (Mission 7 queued when HP ≤ ConditionRed). Deferred per the no-AI memory.
4. **Stale doc patches.** `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` still refers to `UnitClass::Mission_Enter` for `0x00739EC0` even though Ghidra has been renamed to `UnitClass::PerCellProcess`. Stage 1 audit notes are appended but the original sections aren't rewritten. Optional cleanup pass.
5. **`BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` Section 7 still describes `0x006AF6C0` as a refinery dock-queue processor.** Stage 2 confirmed there's no separate processor; refineries use Mission_Deploy_Building + FUN_004595C0 (now renamed `BuildingClass::ReleaseDockedHarvester`). Optional doc rewrite.
6. **Carryall passenger-pickup path.** `AircraftClass::Mission_Enter @ 0x00419C80` *also* handles Carryall passenger pickup via a separate `Type+0xDFC` (Carryall) branch in state 7. That branch calls `CargoClass::AddPassenger` + `AircraftClass::Carryall_Pickup` and does NOT interact with the multi-pad data (Carryalls airlift units, they don't dock at airfields for that). Live-decompile-verified 2026-05-11. The multi-pad design leaves this branch alone; Carryall pickup stays in whatever Rust path implements it (currently TBD), orthogonal to this work.

## Verified research used to derive this design

- [docs/plans/2026-05-11-mission-enter-and-multi-dock-investigation-plan.md](docs/plans/2026-05-11-mission-enter-and-multi-dock-investigation-plan.md)
- [MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md) (new this session)
- [BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md](docs/research/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md) (verification audit appended 2026-05-11)
- [MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md) (verification audit appended 2026-05-11)
- [FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md](docs/research/FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md) (verification audit appended 2026-05-11)
- [HARVESTER_DOCK_UNLOAD.md](docs/research/HARVESTER_DOCK_UNLOAD.md)
- [HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](docs/research/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md)

## Parity-gap reality checks (2026-05-11)

Six live checks were run to confirm the parity gap is real and the design closes it correctly:

1. **GAAIRC docking offsets in `ini/artmd.ini`** — 4 pads with offsets (0,-128,0), (0,128,0), (256,-128,0), (256,128,0). Without multi-pad parsing, 4 aircraft target the same cell → visible stacking. AMRADR inherits via `Image=GAAIRC`. Verdict: ✓ visual gap is significant.
2. **AircraftClass vtable +0x240 wiring** — `read_memory(0x007E24E4, 4) = 0x00419C80` little-endian. ✓ dispatch wiring confirmed live.
3. **`AircraftClass::Mission_Enter @ 0x00419C80` live re-decompile** — states 0-5 are dormant TS holdovers; state 6 = approach + radio cmd 0xE; state 7 = `(*(building+0xA8))(coords_out, aircraft)` per-pad cell lookup. Carryall branch (`Type+0xDFC`) orthogonal. Verdict: ✓ Stage 2 doc's claims match.
4. **Air-layer occupancy** — read [air_movement.rs:8](src/sim/movement/air_movement.rs#L8): aircraft don't use ground occupancy. AddOccupy/RemoveOccupy interaction with pad cells is moot. Verdict: ✓ not a problem.
5. **Air-move target flow** — traced `tick_aircraft_docks → m.air_move_to → issue_air_move_command → entity.movement_target.final_goal → tick_air_movement → position → renderer`. Per-pad cell does drive descent. Verdict: ✓ chain works as designed.
6. **`BuildingClass::GetDockCoord @ 0x00447B20` live decompile** — revealed that gamemd adds DockingOffset to **building geometric CENTER**, not origin top-left. Our existing `refinery_pad_cell` formula was origin-relative — bugged but never fired in retail (all refineries have commented-out DockingOffset0). Design's `pad_cell_for` formula corrected (see Interfaces section). Verdict: ✓ formula bug caught and patched before implementation.
