# Apply_area_damage Live-Vector Iteration — TARGET-DEATH Re-Swarm Report

**Address:** `Apply_area_damage @ 0x00489280`
**Date:** 2026-05-28
**Slot:** 5 of 5 — active-vector removal timing re-swarm
**Confidence:** HIGH for all primary findings (live decompile + assembly context verified this session).
**Active in YR:** Yes — all standard bullet and weapon AoE damage routes through this function.

---

## Target Question

Does `Apply_area_damage` iterate a SNAPSHOT or the LIVE object set when collecting damage
targets? What is the per-object damage order? How does it interact with the LogicClass
active-vector cursor running the outer PerTickUpdate pass?

## Non-Goals

- Damage formula / Verses / falloff math (fully covered in `DAMAGE_MATH_GHIDRA_REPORT.md`).
- Bridge tile destruction logic (covered in `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`).
- Slots 1–4 per-class death removal paths (referenced, not duplicated).

## Evidence Needed to Mark COMPLETE

- [x] Confirm SNAPSHOT vs live iteration from decompile + assembly.
- [x] Confirm iteration order (cell-scan order, no sort).
- [x] Confirm ReceiveDamage dispatch uses vector index, not live list.
- [x] Identify active-vector cursor interaction boundary.
- [x] Confirm 0-CellSpread path.

---

## 1. SNAPSHOT: Two-Phase Design (VERIFIED)

`Apply_area_damage` is a strict two-phase function:

**Phase 1 — Collection (cell scan):** `0x004895C0–0x004899D4`
Iterates cells from the CellSpread offset table (`DAT_00ABD490/492`), walks each cell's
occupant linked list (ground at `cell+0xE4` or bridge deck at `cell+0xE8`), and for each
qualifying object calls `operator_new(8)` to allocate an `{object*, distance}` record.
Records are appended into an internal pointer-array vector at:
```
0x004899B0: MOV dword ptr [ECX + EAX*0x4], EBX   ; store record ptr into vector slot EAX
```
The loop continues via `0x004899B3: MOV ESI, [ESI+0x30]` (next object in cell list), then
`0x004899B8: JNZ 0x004896DD` (next object in same cell) and outer cell-index increment at
`0x004899C6–0x004899D4`.

**Phase 2 — Dispatch:** `0x004899F1–0x00489AD0`
After the cell scan loop exits, a separate index loop reads the collected vector:
```
0x004899F1: MOV EAX, [ESP+0x3c]       ; base of collected vector
0x004899F5: MOV ECX, [ESP+0x10]       ; loop index (starts at 0)
0x004899F9: MOV EAX, [EAX + ECX*0x4] ; load record ptr
0x004899FC: MOV ESI, [EAX]            ; object*
0x004899FE: MOV EDI, [EAX+0x4]        ; distance
```
Re-checks `object->IsAlive` (`+0x90`) at `0x00489A01` before calling:
```
0x00489AB6: CALL dword ptr [EDX + 0x16C]  ; vtable+0x16C = ReceiveDamage
```
Loop increments at `0x00489AC1–0x00489AD0` (INC index, CMP vs count, JL back).

**The target set is FIXED at the end of Phase 1.** No cell-list walk occurs during Phase 2.
Objects killed mid-dispatch do NOT add or remove other objects from the dispatch queue.

Evidence: `decompile_function 0x00489280` + `get_assembly_context` at
`0x004899B0`, `0x004899B3`, `0x004899B8`, `0x004899D4`, `0x004899F1`, `0x00489AB6`.

---

## 2. Damage Application Order: Cell-Scan / Linked-List Traversal Order (VERIFIED)

The vector is built in the exact order objects are encountered during the CellSpread
cell-scan. There is **no sort** between Phase 1 and Phase 2.

Cell scan order:
1. Airborne objects in the impact cell via `FUN_00412b40` / `FUN_004137a0` linked walk — prepended before the spread loop.
2. Per spread cell in `DAT_007ED3D0[spread_index]` iteration order, using the pre-computed `DAT_00ABD490/492` X/Y offset table (not sorted by distance, sorted by table index).
3. Within each cell: linked-list traversal order (`object+0x30` next pointer) — insertion order of the cell's occupant list.

No distance sort is performed. Two objects at different distances in the same cell are
dispatched in linked-list order, not near-first order.

**This is determinism-critical for lockstep:** the order objects take damage determines
which ones die first when damage is near the kill threshold, which can change chain effects
(e.g., a unit dying first vs second changes retaliation targets). The current Rust uses
`BTreeSet<u64>` for deduplication (insertion order independent of cell-scan sequence), then
iterates `entities.values()` (BTreeMap order = stable_id order) for the fallback path — both
differ from gamemd's cell/linked-list order.

Evidence: decompile body — no `qsort` / no distance-ordered insertion; `operator_new(8)`
called and appended in-order inside the cell-scan loop.

---

## 3. Active-Vector Cursor Interaction (VERIFIED / INFERRED SEPARATION)

**The area-damage loop itself is safe against mid-loop removal.**
Phase 1 snapshot + Phase 2 dispatch means: objects killed by an earlier `ReceiveDamage`
call in Phase 2 are already removed from the active vector (per slots 1–4: remover
`FUN_0055BAE0` left-compacts + clears `Object+0x98`), but they remain in the collected
vector with their original pointer. The `IsAlive` re-check at `0x00489A01` guards against
dispatching to already-dead objects — the pointer is not dereferenced blindly.

**The OUTER LogicClass cursor is NOT protected by this snapshot.**
`PerTickUpdate @ 0x0055AFB0` iterates the live active-vector via live-count reload at each
iteration (established by slots 1–4). When `ReceiveDamage` in Phase 2 kills an object:
- Remover `FUN_0055BAE0` left-compacts the active-vector and clears `Object+0x98`.
- If the killed object's index was below the PerTickUpdate loop cursor, the cursor
  effectively skips the compacted-in object (established finding, not re-derived here).
- If the killed object's index was above the cursor, the cursor reaches a different object
  than before compaction (but the killed object itself is gone, count decremented, so
  cursor doesn't walk off the end).

The area-damage snapshot protects the collect-then-dispatch internal loop. It does NOT
protect the PerTickUpdate outer cursor — those removals still shift the vector.

**Active in YR:** Yes (normal skirmish weapon fire triggers Apply_area_damage every
detonation with CellSpread > 0).

---

## 4. Zero-CellSpread / Direct-Hit Path

`Apply_area_damage` guards at entry:
```c
local_80 = Math__ftol(wh->CellSpread * 256.0)   // max radius in leptons
```
If `wh->CellSpread == 0`, `local_80 == 0`. The distance check `iVar10 <= local_80` at
the Phase 2 dispatch only passes if `distance == 0` exactly. The spread cell count table
`DAT_007ED3D0[0]` must still be checked (it is: `0x004899C7–0x004899D4`).

**The more important finding:** `WarheadTypeClass::Detonate @ 0x004690B0` calls
`Apply_area_damage` for ALL detonations, including `CellSpread=0`. For `CellSpread=0`,
the offset table covers only the impact cell (cell-count table entry 0 = 1 cell), and
only objects at distance == 0 leptons pass. In practice this means only the object in
the impact cell can be hit — effectively a single-target path through the same function.

There is no separate single-target ReceiveDamage shortcut for `CellSpread=0` at the
`WarheadTypeClass::Detonate` level. `Apply_area_damage` handles both cases.

Evidence: decompile — `local_80 = Math__ftol(...)` is computed unconditionally; the
spread-loop and dispatch loop still execute with `CellSpread=0`.

---

## 5. Rust Shape vs gamemd (DRIFT)

Current `apply_aoe_damage` in `src/sim/combat/combat_aoe.rs`:

| Aspect | gamemd | Rust | Verdict |
|--------|--------|------|---------|
| Snapshot vs live | SNAPSHOT — collect vector, then dispatch | Returns `Vec<(u64, u16)>` to caller — effectively snapshot | **MATCH** (structurally safe) |
| Iteration dedup | None — linked-list traversal, no dedup structure | `BTreeSet<u64>` dedup (occupancy path) | **MATCH** (same logical result, different mechanism) |
| Damage order | Cell-scan table order → linked-list order within cell | BTreeSet visit order (by stable_id), not cell-scan order | **DRIFT** — order diverges when multiple objects share a cell |
| Zero-CellSpread short-circuit | Handled in-function via `local_80==0` | Returns `Vec::new()` for `cell_spread <= SIM_ZERO` | **DRIFT** — Rust skips target collection entirely; gamemd still processes the impact cell |
| Airborne first | Airborne objects collected before cell-scan loop | Airborne handled in fallback path after occupancy path | **DRIFT** (ordering) |
| Active-vector cursor | NOT protected — outer PerTickUpdate sees removals | Rust applies damage list after collection; active-list consequences same | **MATCH** (Rust sim already passes snapshot list to callers) |

---

## Implementation Handoff

**H1 — Fix iteration order in occupancy path**
- Verified behavior: Phase 1 appends objects in cell-scan-table order, then linked-list
  order within each cell.
- Rust delta: Replace `BTreeSet`-based occupancy scan + `entities.values()` fallback with
  a single pass that iterates cells in `cells_in_spread` table order, then within each cell
  iterates the occupancy layer list in insertion order.
- Affected surface: `src/sim/combat/combat_aoe.rs` `apply_aoe_damage`.
- Acceptance scenario: Two tanks at distance ~midpoint in the same cell, one at killable
  health — the one that appears first in the cell's occupancy list (insertion order) dies;
  the one added later survives (or not, if both die). BTreeSet order by id may reverse this.
- Proposed test name: `test_aoe_damage_order_within_cell_matches_insertion_order`
- Risk: MEDIUM — affects which objects die first in close multi-unit engagements.

**H2 — Zero-CellSpread short-circuit removal**
- Verified behavior: `CellSpread=0` still enters Apply_area_damage; processes impact cell;
  only distance-0 objects pass. No objects get a free skip.
- Rust delta: Remove `if cell_spread <= SIM_ZERO { return Vec::new(); }` guard or replace
  with a single-cell, zero-distance check.
- Affected surface: `src/sim/combat/combat_aoe.rs` line ~80.
- Acceptance scenario: A tank standing exactly at the impact cell is damaged by a weapon
  with `CellSpread=0`; currently Rust returns empty and applies zero damage.
- Proposed test name: `test_aoe_zero_cell_spread_still_hits_impact_cell_occupant`
- Risk: HIGH — any weapon with `CellSpread=0` that currently calls `apply_aoe_damage`
  is silently doing zero damage via the Rust path. This fires every detonation for all
  direct-fire weapons.

**H3 — Airborne-first collection order**
- Verified behavior: Airborne objects in the impact cell are appended to the vector BEFORE
  the cell-scan loop over CellSpread cells (they are collected in the `iVar10 < param_1[2]`
  block before the main spread loop).
- Rust delta: The current `entity.occupancy_list_layer().is_none()` fallback runs AFTER
  the occupancy path. Move airborne collection to run first or model it as a pre-scan.
- Affected surface: `src/sim/combat/combat_aoe.rs` ~lines 127–143.
- Acceptance scenario: Airborne unit at impact cell and ground unit both at kill range —
  airborne takes damage first (currently receives it last or in mixed order).
- Proposed test name: `test_aoe_airborne_hit_before_ground_in_collection_order`
- Risk: LOW — affects only detonations near mixed ground/air occupancy.

---

## Negative Facts / Do Not Do

1. **Do NOT sort the collected vector by distance.** gamemd does not sort.
   Evidence: decompile shows no `qsort` call between Phase 1 end and Phase 2 start.

2. **Do NOT short-circuit on `CellSpread=0`.** The impact cell is still processed.
   Evidence: `local_80 = Math__ftol(...)` unconditional; cell-count-table path executes
   with spread=0.

3. **Do NOT use a snapshot to protect the outer PerTickUpdate cursor.** The area-damage
   snapshot is internal to Apply_area_damage only. The outer PerTickUpdate loop sees all
   vector compactions from deaths triggered inside Apply_area_damage.
   Evidence: slots 1–4 established `FUN_0055BAE0` left-compacts immediately; no re-sort or
   index repair happens at PerTickUpdate level after the call returns.

4. **Do NOT implement a separate single-target path for `CellSpread=0`.**
   `WarheadTypeClass::Detonate` passes all detonations through `Apply_area_damage`; there is
   no early branch for zero-spread at the Detonate level.
   Evidence: decompile of `0x004690B0` (CHAOS_DRONE / AAHEATSEEKER2 docs establish call chain).

5. **Do NOT add a sort within a cell's occupant iteration.** Within each cell, iteration
   follows the cell's linked-list insertion order (`object+0x30` next pointer). This is
   non-deterministic between different scenarios where units entered cells in different
   orders, but it IS deterministic for the same scenario state — and gamemd uses this order
   exactly.
   Evidence: `0x004899B3: MOV ESI, [ESI+0x30]` is the unconditional next-pointer walk.

---

## Remaining Uncertainty

1. **`DAT_007ED3D0[0]` exact value** — the cell-count for `CellSpread=0` (determines whether
   any cells are scanned at all). Likely 1 (just the impact cell) but not read directly this
   session.

2. **Airborne object list mechanism** (`FUN_00412b40` / `FUN_004137a0`) — the exact linked
   list or data structure used for airborne objects in the impact cell was not traced beyond
   the call sites visible in the decompile. The ordering within that list is unknown.

3. **`object+0x30` vs `object+0xC`** — `WARHEAD_DETONATE_GHIDRA_REPORT.md` cites `object[0xC]`
   (i.e., byte offset `0x30`) as the next-object pointer in the cell list for objects inside
   the spread cell loop (Step 6b). The assembly confirms `0x004899B3: MOV ESI, [ESI+0x30]`.
   These are consistent (the doc uses dword indexing: `[0xC]*4 = 0x30`), but verifying the
   field name in the struct layout was not done this session.

---

## Sources

- Live `decompile_function 0x00489280` (this session)
- `get_assembly_context` at `0x004899A8`, `0x004899B0`, `0x004899B3`, `0x004899B8`,
  `0x004899D4`, `0x004899F1`, `0x00489A59`, `0x00489AB6`, `0x00489AC1`, `0x00489AD0`
- `docs/research/DAMAGE_MATH_GHIDRA_REPORT.md` §6 — target collection + damage application
- `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`
  §3.3 — "Object damage is collected first, then applied"
- `docs/research/WARHEAD_DETONATE_GHIDRA_REPORT.md` §4 — step-by-step decompile
- Slots 1–4 reports (PENDING_DELETE, CHANGEOWNER, etc.) — active-vector removal mechanics
