# Low Bridge TubeMovement Final Z / Interpolation -- Ghidra Research Report

**Address(es):** `0x007359F0` (`UnitClass::TubeMovement`), `0x0051B350` (infantry tube movement), `0x004B0F20` (drive producer), `0x0075AEC0` / `0x0075B3FC..0x0075B54A` (walk producer), `0x007363B0` (`UnitClass::AI`), `0x0051BF00` (`InfantryClass::AI`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** low-bridge/tunnel TubeClass active traversal final Z, per-tick interpolation cadence, `TubeClass+0x1C0` consumer semantics, destination coordinate writes, and unit-vs-infantry differences.
**Non-Scope:** bridge SHP filename resolution, high-bridge occupancy except negative comparison, full pathfinder direction-8 emission, retail MIX map coverage for `[Tubes]`.
**Confidence:** High for the decompiled unit/infantry producer/consumer behavior; Medium for retail-map prevalence of explicit low-bridge tubes because this pass did not extract MIX-packed maps.
**Active in YR:** Yes for the producer/consumer paths when an object has active tube state (`object+0x684 >= 0`). The same active fields are set by standard drive and walk locomotor direction-8 producers.

## 1. Overview

Active TubeMovement is a per-object movement state driven by `object+0x684` (signed tube index) and `object+0x685` (tube path cursor). Units and infantry share the same step-buffer traversal shape, but they do not share final landing semantics: units snap X/Y to `TubeClass+0x28` and keep the accumulated tube Z, while infantry uses infantry placement and snaps final Z to ground height.

The most important final-Z result is that unit TubeMovement does not recompute or clamp the final Z from the exit cell when traversal finishes. Unit final Z is whatever accumulated value is stored at `object+0x570`, seeded by the producer and incremented by a signed integer quotient derived from `TubeClass+0x1C0`.

## 2. Class Layout / Key Offsets

| Field | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `ObjectClass` active tube index | `+0x684` signed byte | `0xFF` inactive; non-negative indexes `g_TubeArray` | `UnitClass::AI @ 0x007363B0`, `InfantryClass::AI @ 0x0051BF00`, producers `0x004B1380`, `0x0075B3FC` | Yes, conditional on direction-8 tube traversal |
| Tube cursor | `+0x685` byte | Current path-step index; incremented after reaching the current in-tube target | `UnitClass::TubeMovement @ 0x00735B36..0x00735B4C`, infantry `0x0051B491..0x0051B4AE` | Yes |
| Tube movement target coord | `+0x568/+0x56C/+0x570` | Next in-tube target center; `+0x570` is also the Z accumulator used by unit final landing | drive producer `0x004B0F20`, walk producer `0x0075B45E..0x0075B54A`, unit final `0x00735FB4..0x00735FEC` | Yes |
| Current object coord | `+0x9C/+0xA0/+0xA4` | Current world coord passed to virtual coord setter/getter | unit and infantry tube decompiles | Yes |
| Tube entry coord | `TubeClass+0x24` | Start cell used for initial target and ground-height delta base | unit `0x00735AB2..0x00735AF7`, infantry `0x0051B3FE..0x0051B44D` | Yes |
| Tube exit coord | `TubeClass+0x28` | Producer destination and unit final X/Y snap target | drive `0x004B0F20`, unit final `0x00735FB1..0x00735FDB` | Yes |
| Tube direction | `TubeClass+0x2C` | Final facing source if exit cell has a tube | unit `0x0073607A..0x007360A5`; prior low-bridge reports | Yes |
| Tube path steps | `TubeClass+0x30..` | Direction entries; `-1` sentinel means no more path steps | unit first check `0x00735A05..0x00735A20`, infantry `0x0051B368..0x0051B383` | Yes |
| Tube path count | `TubeClass+0x1C0` | Signed divisor for per-step Z delta; not a remaining counter | unit `0x00735AE1..0x00735B26`, walk producer `0x0075B4FC..0x0075B544` | Yes |
| Cell ground object list | `CellClass+0xE4` | Unit/infantry final-exit blocker list for tube exit cell | unit `0x00735E5F..0x00735E6E`, infantry `0x0051B798` branch | Yes |

## 3. Core Logic

### 3.1 Producer seeding and `TubeClass+0x1C0`

Drive and walk producers enter tube movement only from path direction `8` and a valid current-cell tube index. They write `object+0x684 = tube_index`, clear `object+0x685 = 0`, copy the following path buffer forward, clear `object+0x63C`, and seed the first tube target from `TubeClass+0x24` plus `path[0]`.

Both producer families compute:

```text
z_delta = (exit_ground_height - current_ground_height) / tube.path_len
object+0x570 = current_ground_height + z_delta
```

This is signed integer division (`IDIV`) with no zero guard in the checked drive and walk producer paths. Active in YR: Yes for drive/walk direction-8 traversal; zero-step auto shells are real data but are not valid visible producer inputs without another path avoiding this division.

Evidence:

- Drive producer branch in `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` writes `+0x684/+0x685`, reads `TubeClass+0x1C0`, and writes `object+0x570 = (exit-current)/path_len + current`.
- Walk producer assembly `0x0075B3FC..0x0075B54A` writes the same active fields and performs `IDIV EDI` where `EDI = TubeClass+0x1C0`, then writes `object+0x570`.
- Prior `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md` records the same no-zero-guard producer behavior.

### 3.2 Per-tick interpolation / timing

When active, both UnitClass and InfantryClass check the current `TubeClass+0x30[cursor]` entry first. If it is `-1`, they skip straight to final-exit handling.

If the step is valid:

1. Compute distance from current object coord to `object+0x568`.
2. Compute a movement budget:
   - Unit path uses `TechnoTypeClass+0x678` multiplied by the double at `0x007E48F0` before `Math::ftol` (`0x00735A76..0x00735A93`). Existing timing docs identify this constant as `1.5`.
   - Infantry tube movement reads `TechnoTypeClass+0x678` directly (`0x0051B3D9..0x0051B3EB`).
3. If `distance > budget`, move partially toward the current target and return. Cursor is not incremented.
4. If `distance <= budget`, call the coord setter to reach the current target, increment `+0x685` by one, and read the next path entry.
5. If the next entry is valid, compute the next target and add one signed `z_delta` to `object+0x570`.
6. Use only the leftover budget from this tick to move partially toward the next target, then return.

There is no loop that consumes multiple tube path entries in one AI call. One tick can finish the current segment and optionally spend leftover budget into the next segment, but it cannot advance through an arbitrary number of tube cells in one call. Active in YR: Yes, because `UnitClass::AI @ 0x007363B0` and `InfantryClass::AI @ 0x0051BF00` dispatch to these functions before normal AI when `+0x684 >= 0`.

Evidence:

- Unit cursor increment and single-next-step branch: `0x00735B36..0x00735B64`, next-target setup `0x00735B6A..0x00735C16`, residual interpolation return `0x00735C30..0x00735D34`.
- Unit partial branch when current distance exceeds budget: `0x00735D35..0x00735E28`.
- Infantry parallel cursor/residual structure: `0x0051B491..0x0051B58A`, residual interpolation `0x0051B599..0x0051B78F`.

### 3.3 Unit final landing

Unit final landing checks the current/exit cell ground object list at `CellClass+0xE4`. If the list is empty, it writes:

```text
current.x = TubeClass+0x28.x * 256 + 128
current.y = TubeClass+0x28.y * 256 + 128
current.z = object+0x570
object+0x684 = 0xFF
object+0x68B = 1
movement/facing state updates
if GetTubeAtCell(final_cell) != null:
    facing = (tube.direction << 13) - 0x6001, masked to facing quantum
```

Active in YR: Yes for units with active tube state and an empty ground object list at exit. Evidence: `0x00735FA1..0x00735FEC` (Tube+0x28 X/Y plus `ESI+0x570` Z), clear at `0x00735FF8`, facing source at `0x0073607A..0x007360A5`.

Important tiny detail: the final Z write uses `object+0x570`; it does not call `CellClass::GetGroundHeight` for the exit cell in the final branch and does not force the coordinate to `TubeClass+0x28` ground height. If `(exit_ground - entry_ground)` is not exactly divisible by `TubeClass+0x1C0`, the signed-division remainder is not corrected at final landing.

If `CellClass+0xE4` is nonempty, unit final landing does not clear `+0x684`; instead it builds a small list of blockers and can stop/scatter units or infantry occupying the ground list. Active in YR: Conditional on blocked exit. Evidence: unit final else branch starts at `0x00735E6E`.

### 3.4 Infantry final landing

Infantry final landing is not a direct `TubeClass+0x28` snap. After the active traversal has reached the current target cell, infantry calls its cell-entry virtual (`vtable+0x1AC`) and only finishes when that returns clear. It then calls `CellClass::PlaceInfantryInCell`, calls `CellClass::GetGroundHeight` for the placement coordinate, writes that coordinate through the object coord setter, marks cell occupancy, and clears `+0x684`.

Active in YR: Yes for infantry with active tube state. Evidence: `InfantryClass::AI @ 0x0051BF00` dispatches to `0x0051B350`; final placement path at `0x0051B936..0x0051BA8D`.

Important difference from units:

- Unit final X/Y uses `TubeClass+0x28` directly.
- Infantry final placement uses the current reached cell plus infantry subcell placement, not a direct final `TubeClass+0x28` coordinate write.
- Unit final Z uses accumulated `object+0x570`.
- Infantry final Z uses `CellClass::GetGroundHeight` after placement.

## 4. INI Keys

| Key / source | Value / source line | Relevance | Active in YR |
|---|---|---|---|
| `[General] TunnelSpeed` | `ini/rulesmd.ini:365`, `ini/rules.ini:276` = `1` | Present for tunnel movement broadly; not observed as a direct read in the final-Z unit/infantry functions covered here. | Conditional; not material to this slice's verified final-Z math |
| `[General] DestroyableBridges` | `ini/rulesmd.ini:804`, `ini/rules.ini:664` = `yes` | Confirms bridge collapse paths are standard YR, but final-Z TubeMovement math itself is not gated by this key. | Yes for bridge damage system; not a TubeMovement final-Z gate |
| `[General] BridgeStrength` | `ini/rulesmd.ini:816`, `ini/rules.ini:676` = `1500` | Damage strength, non-scope for active traversal math. | Yes for damage; no direct final-Z effect found |
| `[General] BridgeDestruction` | `ini/rulesmd.ini:3029`, `ini/rules.ini:2509` = `yes` | Damage/destruction rule, non-scope for active traversal math. | Yes for damage; no direct final-Z effect found |
| `TooBigToFitUnderBridge` | multiple UnitType entries in `ini/rulesmd.ini`; e.g. `:5549` | Related low-bridge pathing gate in other reports; not read by TubeMovement consumer final-Z path. | Conditional on unit type in pathing, not final interpolation |
| Map `[Tubes]` | parsed by `FUN_007283C0` in prior low-bridge reports | Supplies nonzero `TubeClass+0x1C0` and path steps for visible tube traversal. | Conditional on map data |

## 5. Integration Points

| Integration point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Unit AI dispatch | If `(char)Unit+0x684 >= 0`, call `UnitClass::TubeMovement`, then vtable `+0x4A0(0)`, and return before normal AI. | `0x007363B0` | Yes |
| Infantry AI dispatch | If `(char)Infantry+0x684 >= 0`, call infantry tube movement, then vtable `+0x4A0(0)`, and return before normal AI. | `0x0051BF00` | Yes |
| Drive producer | Direction `8` and valid current-cell tube index set active tube state and first target. | `0x004B0F20`; write site `0x004B1380` from prior producer report | Yes for drive locomotion |
| Walk producer | Direction `8` and valid current-cell tube index set active tube state and first target. | `0x0075B3FC..0x0075B54A` | Yes for infantry/walk locomotion |
| Low/high bridge object lists | Tube final blocker checks use ground `CellClass+0xE4`; this is not high-bridge `AltObject`/`OnBridge` landing. | unit `0x00735E5F..0x00735E6E`; `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` | Yes |

## 6. Current Rust Implementation Status

Current Rust already represents `TubeFact` fields and explicit-vs-auto sources:

- `src/map/tube_facts.rs:30` defines `TubeFact { entry, exit, direction, path_steps, source }`.
- `src/map/tube_facts.rs:66` uses `path_steps.len()` as `path_len`.
- `src/map/resolved_terrain.rs:155` stores a `tube_index` equivalent.
- `src/map/resolved_terrain.rs:298` implements direction `8` stepping to `tube.exit` or `(0,0)`.
- `src/sim/pathfinding/core.rs:382` only exposes direction-8 A* edges for `TubeSource::ExplicitMap` with `path_len > 0`.

Current Rust tube movement does not match the verified final-Z/timing behavior:

- `src/sim/movement/tube_movement.rs:124-144` advances one tube path cell per Rust tick rather than using a speed budget and residual interpolation.
- `src/sim/movement/tube_movement.rs:124-125` immediately finishes when `cursor >= path_len`; `begin_low_bridge_tube_movement` still does not reject zero-step tubes despite the declared `ZeroLengthTube` error.
- `src/sim/movement/tube_movement.rs:159` snaps to `state.exit`.
- `src/sim/movement/tube_movement.rs:262-269` treats low-bridge tube cells as bridge-walkable landing and writes deck-level Z, `on_bridge`, and bridge occupancy. That is not what the checked unit/infantry TubeMovement final branches do.
- No scanned Rust state stores the equivalent of `object+0x570` tube-Z accumulator or per-class unit-vs-infantry final landing difference.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::TubeMovement` active entry and step-valid branch | verified | `0x007359F0`, especially `0x00735A05..0x00735E28` | none for requested slice |
| Unit final empty-exit branch | verified | `0x00735FA1..0x007360A5` | none |
| Unit final blocked-exit branch | verified for branch role | `0x00735E6E..0x00735F9A` | Exact blocker side effects are outside final-Z scope |
| Infantry tube movement step-valid branch | verified | `0x0051B350`, especially `0x0051B368..0x0051B78F` | none for requested slice |
| Infantry final placement branch | verified | `0x0051B936..0x0051BA8D` | none |
| Unit AI active dispatch | verified | `0x007363B0` | none |
| Infantry AI active dispatch | verified | `0x0051BF00` | none |
| Drive producer Z seed | verified | `0x004B0F20`, active write site `0x004B1380` from prior producer report | none |
| Walk producer Z seed | verified | `0x0075B3FC..0x0075B54A` | none |
| `TubeClass+0x1C0` parser source | verified by prior doc, not re-expanded here | `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`, `[Tubes] parser @ 0x007283C0` | full parser is prior-covered |
| Retail map prevalence of explicit low-bridge `[Tubes]` | deferred | prior plain-text search only | MIX extraction/runtime map dump |
| High-bridge occupancy comparison | verified negative comparison only | `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`; unit tube final reads `+0xE4` | high-bridge occupancy not expanded |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-001 -- What enters unit TubeMovement? -> Unit AI calls it when signed byte +0x684 is non-negative and returns before normal AI.` (evidence: `0x007363B0`)
- `[RESOLVED] OQ-002 -- What enters infantry TubeMovement? -> Infantry AI calls `0x0051B350` when signed byte +0x684 is non-negative and returns before normal AI.` (evidence: `0x0051BF00`)
- `[RESOLVED] OQ-003 -- Is TubeClass+0x1C0 a timer, cursor, or path count? -> It is a signed path-step count used as a divisor for Z delta; cursor is object+0x685.` (evidence: unit `0x00735AE1..0x00735B26`, walk `0x0075B4FC..0x0075B544`)
- `[RESOLVED] OQ-004 -- Is there a zero guard before dividing by TubeClass+0x1C0? -> No guard in checked drive/walk producers.` (evidence: `0x004B0F20`, `0x0075B4FC..0x0075B544`)
- `[RESOLVED] OQ-005 -- Does unit final Z snap to exit ground height? -> No; final branch writes current X/Y from Tube+0x28 and Z from object+0x570.` (evidence: `0x00735FA1..0x00735FEC`)
- `[RESOLVED] OQ-006 -- Does infantry final Z match unit final Z? -> No; infantry final placement calls PlaceInfantryInCell and GetGroundHeight before clearing tube state.` (evidence: `0x0051B936..0x0051BA8D`)
- `[RESOLVED] OQ-007 -- Can one tick consume multiple tube cells? -> Not arbitrarily; the function can complete current target, increment cursor once, optionally move into the next target using leftover budget, then returns.` (evidence: unit `0x00735B36..0x00735D34`, infantry `0x0051B491..0x0051B78F`)
- `[RESOLVED] OQ-008 -- Which object list gates final tube exit? -> Ground list `CellClass+0xE4`, not high-bridge `AltObject +0xE8`.` (evidence: unit `0x00735E5F..0x00735E6E`; bridge occupancy report)
- `[RESOLVED] OQ-009 -- Are zero-step auto shells valid visible traversal inputs? -> Producer evidence says no for standard drive/walk direction-8 traversal because path_len is divided by without a guard.` (evidence: drive/walk producers; prior low-bridge producer report)
- `[RESOLVED] OQ-010 -- Is `TunnelSpeed` directly used in final-Z TubeMovement? -> Not in the checked final-Z consumer/producers; speed budget comes from type speed fields in this slice.` (evidence: unit `0x00735A76..0x00735A93`, infantry `0x0051B3D9..0x0051B3EB`; INI `TunnelSpeed=1`)
- `[RESOLVED] OQ-011 -- Does low-bridge TubeMovement set high-bridge OnBridge? -> No matching final branch write was found in unit/infantry tube movement; final blocker list is ground list.` (evidence: unit/infantry decompiles; negative comparison to high-bridge `ObjectClass+0x8C` docs)
- `[RESOLVED] OQ-012 -- Does final unit facing use tube direction? -> Yes, after final coordinate write it calls GetTubeAtCell on final cell and updates facing from `TubeClass+0x2C` if present.` (evidence: `0x0073607A..0x007360A5`)
- `[RESOLVED] OQ-013 -- Does Rust currently represent the Z accumulator? -> No scanned Rust tube state contains a `+0x570` equivalent.` (evidence: `src/sim/movement/tube_movement.rs:24`, `src/map/tube_facts.rs:30`)
- `[DEFERRED] OQ-014 -- Which stock retail maps use explicit low-bridge `[Tubes]`?` (category: `requires-different-system-context`; reason: requires MIX extraction or runtime map dump; next-step-if-pursued: extract stock maps and inspect `[Tubes]` entries with low-bridge cells)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Unit tube final Z is accumulated `object+0x570`, seeded and stepped by signed `(exit_ground-entry_ground)/TubeClass+0x1C0`, not recomputed from exit ground height. | `0x004B0F20`, `0x00735AE1..0x00735B26`, `0x00735FA1..0x00735FEC` | missing/mismatch | `src/sim/movement/tube_movement.rs`, entity movement state | Store and update a tube-Z accumulator for unit-class tube traversal; preserve signed integer truncation remainder at final landing. | A two-step explicit tube where ground delta is not divisible by path_len finishes at `entry_ground + path_len * trunc(delta/path_len)`, not exact exit ground. Proposed test: `test_low_bridge_tube_unit_final_z_preserves_gamemd_truncation_remainder`. | Do not snap unit final Z to `bridge_deck_level` or exit-cell ground height. |
| Tube timing is speed-budget based: partial move if distance exceeds budget; otherwise finish current target, increment cursor once, optionally spend leftover budget into next target, then return. | Unit `0x00735A76..0x00735D34`; infantry `0x0051B3D9..0x0051B78F` | mismatch | `src/sim/movement/tube_movement.rs`, movement tick scheduling | Replace one-path-step-per-tick tube advance with per-tick lepton interpolation and at-most-one-cursor-increment behavior. | A fast unit reaching a tube node with leftover budget moves partway into the next tube segment in the same tick, but does not consume two cursor increments. Proposed test: `test_low_bridge_tube_interpolates_one_cursor_increment_with_residual_budget_like_gamemd`. | Do not model tube traversal as fixed one cell per tick or as a loop that drains all available speed across many tube steps. |
| Infantry final landing differs from units: final placement uses infantry cell placement and ground height; units snap X/Y to `TubeClass+0x28` and keep accumulated Z. | Unit `0x00735FA1..0x00735FEC`; infantry `0x0051B936..0x0051BA8D` | missing distinction | `src/sim/movement/tube_movement.rs`, infantry/subcell placement surfaces | Add class/category-specific final landing: infantry must run placement/subcell-style landing and final ground-height Z, while units use tube exit X/Y plus accumulator Z. | Vehicle and infantry traverse the same explicit tube; vehicle final Z keeps accumulator truncation, infantry final Z equals placement ground height and occupies infantry subcell. Proposed test: `test_low_bridge_tube_infantry_final_z_uses_ground_height_not_unit_accumulator`. | Do not share one final snap routine for all FootClass objects. |

### Negative Facts / Do Not Do

- Do not consume automatic zero-step low-bridge shells as visible direction-8 traversal inputs. Active in YR: Yes for producer paths; evidence drive/walk producers divide by `TubeClass+0x1C0` with no zero guard, while auto shells have `+0x1C0=0` per prior low-bridge constructor reports.
- Do not implement low-bridge TubeMovement final landing as high-bridge `OnBridge` / bridge occupancy. Active in YR: Yes for final tube path; evidence unit final branch checks `CellClass+0xE4` and no matching `ObjectClass+0x8C` final write was found in `0x007359F0` / `0x0051B350`.
- Do not snap unit final Z to `TubeClass+0x28` ground height or bridge deck level. Active in YR: Yes for unit tube exit; evidence `0x00735FB4..0x00735FEC` writes Z from `object+0x570`.
- Do not assume `TubeClass+0x1C0` is remaining time. Active in YR: Yes; evidence object cursor is `+0x685`, while `TubeClass+0x1C0` is read as a divisor and parser-populated path count in prior reports.
- Do not assume infantry and units differ only by animation. Active in YR: Yes; evidence infantry final path calls `CellClass::PlaceInfantryInCell` and `CellClass::GetGroundHeight`, while unit final path directly writes tube exit X/Y.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/IMPLEMENTATION_LOW_BRIDGE_TUBECLASS_HEIGHT_LAYER_TRACE.md`: replace Stage 5/Stage 8 unresolved wording with: "UnitClass TubeMovement final Z is accumulated at object `+0x570`: producers seed it as `entry_ground + signed_trunc((exit_ground-entry_ground)/TubeClass+0x1C0)`, each completed in-tube step adds the same signed quotient, and unit final landing writes `TubeClass+0x28` X/Y with this accumulated Z. Infantry TubeMovement shares the traversal cadence but final landing calls infantry placement and uses `CellClass::GetGroundHeight`, so infantry final Z differs from units."

## Sources

- Ghidra decompile: `UnitClass::TubeMovement @ 0x007359F0`
- Ghidra assembly contexts: `0x00735A05..0x007360A5`
- Ghidra decompile: infantry tube movement `0x0051B350`
- Ghidra assembly contexts: `0x0051B368..0x0051BA8D`
- Ghidra decompile: `UnitClass::AI @ 0x007363B0`
- Ghidra decompile: `InfantryClass::AI @ 0x0051BF00`
- Ghidra decompile: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- Ghidra assembly contexts: `WalkLocomotionClass::ProcessMovement @ 0x0075B3FC..0x0075B54A`
- `C:/Users/enok/Documents/ra2-rust-game-docs/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/IMPLEMENTATION_LOW_BRIDGE_TUBECLASS_HEIGHT_LAYER_TRACE.md`
- Rust scan: `src/sim/movement/tube_movement.rs`, `src/map/tube_facts.rs`, `src/map/resolved_terrain.rs`, `src/sim/pathfinding/core.rs`
- INI scan: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
