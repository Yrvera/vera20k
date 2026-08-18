# Bridge Traversal State Machine — Ground Units Crossing High Bridges — Ghidra Report

**Date:** 2026-07-19
**Scope:** The runtime state machine that carries a GROUND unit (Drive/Walk/Ship locomotor) across a HIGH bridge — the layer field (`OnBridge`), the ramp enter/exit relink, multi-crossing-per-tick state, and segment-exhaustion auto-repath. Targets the 4 `#[ignore]`d WIP tests in `src/sim/movement/movement_tests.rs`.
**Authority order:** binary → Ghidra → docs. Every address cites the Ghidra call that verified it.
**Active in YR:** Confirmed live on standard-skirmish Walk/Drive/Ship locomotor paths (not TS-only — see §7).

> This report EXTENDS existing verified docs. It does not redo them:
> - `bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` (CheckBridgeTraversal @ 0x4D9C60, CellClass offsets, cell-flag bits)
> - `bridges/02-cell-state-layering-zones/HIGH_BRIDGE_ONBRIDGE_OCCUPANCY_TRANSITION_SEQUENCE_GHIDRA_REPORT.md` (walk/drive OnBridge set/clear ordering)
> - `bridges/02-cell-state-layering-zones/BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md` (ObjectClass+0x8C)
> - `GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md` (two-layer occupancy repr)

---

## 0. TL;DR — the state machine

A ground unit's "am I on the bridge deck" state is a **single persistent byte `ObjectClass+0x8C` (`OnBridge`)**, NOT the A* path layer and NOT the locomotor layer. It is updated **only at cell-boundary crossings** by a height-delta predicate, and the object's occupancy list membership (ground list vs bridge list) is selected from that byte at the exact add/remove call site.

- **Set** (`OnBridge=1`) when `dst.Level == src.Level - 4` AND `dst.Flags & 0x100`.
- **Clear** (`OnBridge=0`) when `!(dst.Flags & 0x100)` AND `(src.Flags & 0x100)`.
- **Unchanged** otherwise. Ground→ramp and body→ramp are both **Unchanged** (ramp keeps the pre-step byte).

Path following uses a **24-step (`0x18`) storage buffer** (`FootClass+0x5E0`), refilled by a **fresh full A\* to the persistent final goal** at each segment boundary. The 24 is a storage cap, not a search horizon.

---

## 1. Q1 — How gamemd represents "unit is ON the high-bridge deck"

**Answer: a single persistent 1-byte flag `ObjectClass+0x8C` (`OnBridge`).** It is NOT the A\* path layer, NOT the locomotor layer, and NOT derived from the cell each tick — it is a sticky state byte mutated only at qualifying cell-boundary crossings.

### 1.1 The byte and its readers/writers
- Field: `ObjectClass+0x8C`, one byte. (Cross-doc: `bridges/02-cell-state-layering-zones/BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`.)
- **Written** (walk) at `0x0075C179` (`MOV byte ptr [obj+0x8c], 1`) and `0x0075C193` (`MOV byte ptr [obj+0x8c], 0`) — verified via `disassemble_function 0x0075AEC0` (`WalkLocomotionClass__ProcessMovement`).
- **Written** (drive) at `0x004B1830`/`0x004B184A` and `0x004B2586`/`0x004B25A0` — per the verified transition-sequence doc (`decompile_function 0x004B0F20`).
- **Read as list-selector** at `TechnoClass__EnterCell_AddToMultiCells @ 0x005684B1` and `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005688E1`: they read `byte ptr [obj+0x8C]` and push it as the layer argument to `CellClass::AddContent @ 0x0047E8A0` / `RemoveContent @ 0x0047EA90`, which pick the **ground list `CellClass+0xE4`** when the byte is 0 and the **bridge list `CellClass+0xE8`** when nonzero.
- **Read in path recovery** (`FootClass__Find_Path @ 0x004D3920`) as `(char)this[0x23]` (= byte `+0x8C`) — see §4.3.

### 1.2 The two occupancy layers it selects (already verified elsewhere)
`CellClass` carries two independent per-cell object lists and two occupancy bitfields (verified in `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md §2`):
| Layer | List head | Occupancy bits |
|---|---|---|
| Ground | `+0xE4` (FirstObject) | `+0x124` |
| Bridge deck | `+0xE8` (AltObject) | `+0x128` |

`OnBridge` (`+0x8C`) is what routes a unit between these two lists. A deck vehicle and an under-bridge vehicle coexist in the same cell on different lists.

---

## 2. Q2 — The RAMP transition: relinking ground↔bridge layer

**Handled inline in each locomotor's per-tick movement body**, at the cell-boundary crossing. There is no separate "ramp handler" function — the relink is a fixed 5-step sequence. Verified in full from the walk locomotor assembly (`disassemble_function 0x0075AEC0`, region `0x0075C117..0x0075C1AE`):

```
0x0075C11C  CALL [obj_vtbl+0x124] (arg 0)   ; REMOVE from OLD cell — observes OLD OnBridge → old list
0x0075C12E  CALL [obj_vtbl+0x1B4]           ; coordinate update (move to new cell)
; --- OnBridge predicate ---
0x0075C154  MOVSX EDX, [src_cell+0x11B]     ; src.Level (signed i8)   (src = previous cell, ESI)
0x0075C15B  MOVSX EDI, [dst_cell+0x11B]     ; dst.Level (signed i8)   (dst = new cell, EAX)
0x0075C162  SUB EDX, 4                       ; src.Level - 4
0x0075C16A  CMP EDI, EDX                     ; dst.Level == src.Level - 4 ?
0x0075C16E  TEST [dst_cell+0x140], 0x100     ; dst is bridge cell ?
0x0075C179  MOV byte [obj+0x8C], 1           ; SET OnBridge=1  (both conditions true)
0x0075C180  TEST [dst_cell+0x140], 0x100     ; if dst IS bridge → skip clear
0x0075C188  TEST [src_cell+0x140], 0x100     ; else if src IS bridge …
0x0075C193  MOV byte [obj+0x8C], 0           ; CLEAR OnBridge=0
; --- end predicate ---
0x0075C1A1  CALL [obj_vtbl+0x1CC]            ; Set_Height_On_Bridge / per-cell process (snaps Z to deck)
0x0075C1AE  CALL [obj_vtbl+0x124] (arg 1)    ; ADD to NEW cell — observes NEW OnBridge → new list
```

### 2.1 The predicate (exact)
- **Set (OnBridge:=1):** `dst.Level == src.Level - 4` **AND** `dst.Flags & 0x100`.
- **Clear (OnBridge:=0):** `!(dst.Flags & 0x100)` **AND** `(src.Flags & 0x100)`. (Set takes priority: if dst is a bridge cell, the clear test is skipped.)
- **Unchanged:** anything else.

Levels are read as **signed i8** (`MOVSX`). `0x100` = the structural on-bridge cell flag (`CellClass+0x140`), **set on ALL bridge cells including ramps/bridgeheads** (verified in `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS §3.2`).

### 2.2 What triggers the height/layer change (the four ramp cases)
On a standard high bridge, body cells store `Level = ground_under_bridge` and the deck sits `+4` above (the `0x100` height bump). The ramp/bridgehead cell's `Level` is `+4` (the approach height). Thus:

| Step | Level relation | dst `0x100`? | src `0x100`? | Result |
|---|---|---|---|---|
| ground→ramp (going up) | dst.Level != src.Level−4 | yes (ramp) | no | **Unchanged** (stays OFF) |
| ramp→body (mount deck) | dst.Level(0) == src.Level(4)−4 | yes | yes | **Set → OnBridge=1** |
| body→ramp (leaving deck) | false | yes (ramp) | yes | **Unchanged** (stays ON) |
| ramp→ground (dismount) | false | no | yes (ramp) | **Clear → OnBridge=0** |

So `OnBridge` flips exactly at the ramp↔body boundary going up and at the ramp↔ground boundary coming down — never at the ground↔ramp boundary. This is the two-step transition the Rust `compute_bridge_transition` already models.

### 2.3 Removal-before-write, add-after ordering (load-bearing)
The old-cell removal (`vtable+0x124` arg 0) runs **before** the `OnBridge` write, so it observes the **pre-transition** list; the new-cell add (`vtable+0x124` arg 1) runs **after**, so it observes the **post-transition** list. This guarantees a mount step removes from the ground list and adds to the bridge list with no window where the unit is on both. (Verified: `0x0075C11C` < `0x0075C179` < `0x0075C1AE`.)

---

## 3. Q3 — Multi-crossing state persistence ("first bridge set")

**`OnBridge` is persistent and only conditionally written.** When a unit consumes several path steps in one movement pass, the locomotor **re-enters `ProcessMovement` (self-call `CALL 0x0075AEC0`)** to process each successive step — verified self-recursion sites at `0x0075B716`, `0x0075BABB`, `0x0075BC04`. Each re-entry re-runs the §2 predicate at the **new** boundary against the fresh src/dst cells.

Because the predicate leaves the byte **Unchanged** on body→body (both bridge cells, equal Level → neither Set nor Clear fires), the `OnBridge=1` written at the first ramp→body crossing **survives every subsequent body→body crossing** until a genuine Clear condition (ramp→ground) is met. gamemd never recomputes `OnBridge` from scratch per step; it is a sticky byte mutated only on qualifying edges.

**"First bridge set" (Rust test `multi_crossing_preserves_first_bridge_set_update`)** = exactly this: in one tick a fast unit steps ramp→body (Set OnBridge=1, deck_level=4) then body→body (Unchanged); the first Set must not be clobbered by the second crossing's evaluation. The gamemd model: apply Set/Clear/Unchanged **sequentially over the persistent byte**, one per boundary, in path order.

There is no separate "first bridge" object — the persistence is purely the sticky `+0x8C` byte plus `BridgeOccupancy{deck_level}` (which in gamemd is the `Set_Height_On_Bridge` Z snap at `vtable+0x1CC`, recomputed from the deck cell on Set).

---

## 4. Q4 — Segment exhaustion and auto-repath

### 4.1 The 24-step path buffer
`FootClass` stores its active path as a **24-entry (`0x18`) buffer at `+0x5E0`** (24 dwords, `+0x5E0..+0x640`). Verified in `FootClass__Find_Path @ 0x004D3920` (PROOFED, `decompile_function 0x004D3920`):
```c
iVar5 = 0x18 - offset;                       // remaining buffer space
if (astar_result->count < 0x18 - offset)     // clamp store count to buffer space
    iVar5 = astar_result->count;
// copy iVar5 steps into  this + (offset + 0x178)   (int index 0x178 = byte 0x5E0)
```
Consuming a step **shifts the buffer down by one** (`MOVSD.REP` of `0x17`=23 dwords from `+0x5E4`→`+0x5E0`, then `[+0x63C] := -1`) — verified at `0x0075B3D5` and `0x0075BD9A` in the walk assembly. A head value of `-1` (`[+0x5E0] == -1`) means "buffer empty / no path" (`0x0075AF1D CMP [obj+0x5E0], -1`).

### 4.2 Refill = fresh full A\* to the persistent final goal
When the buffer empties, `ProcessMovement` calls `FootClass__Find_Path` (`CALL 0x004D3920` at `0x0075AFC5`), which runs `FootClass__Run_AStar()` from the current cell to the **persistent destination coord** (the NavCom goal, passed in and also cached at `+0x640`). `Run_AStar` returns the **whole route** (`count` may exceed 24); only the first `min(count, 24-offset)` steps are stored. Therefore:

- **The 24-cap is a storage buffer, not a planning horizon.** A\* explores to the real goal; obstacles anywhere on the route (building footprints, other units) are respected on the very first plan and on every refill.
- **Each segment is re-planned from scratch against live cell occupancy.** Buildings occupy cells and are rejected by `Can_Enter_Cell`, so the refill automatically routes around a footprint that lies beyond the previous 24-step segment.
- The final goal and `OnBridge` persist across refills; only the step buffer is regenerated.

Find_Path is invoked from all three ground-mover locomotors — `DriveLocomotionClass__Process_Movement @ 0x004B2630`, `ShipLocomotionClass__Process_Movement @ 0x006A1C80`, `WalkLocomotionClass__ProcessMovement @ 0x0075AEC0` (verified via `get_function_callers 0x004D3920`) — so segment refill is live for every ground unit in a normal YR skirmish.

### 4.3 Does bridge/ramp state affect the repath? — Yes, in the *failure* branch
The normal segment refill is **not** specially bridge-gated (it just re-runs A\*, which honors bridge traversal through `CheckBridgeTraversal @ 0x004D9C60`). But when `Run_AStar` returns **no path**, `Find_Path` enters a recovery branch gated on bridge state (`decompile_function 0x004D3920`):
```c
chebyshev = max(|goal.x - cur.x|, |goal.y - cur.y|);   // in cells
if ( (1 < chebyshev)                                    // not adjacent to goal, OR
     || ( (char)this[0x23] == 0                          //   OnBridge byte (+0x8C) == 0  AND
          && (curcell->Flags & 0x100) != 0 ) )           //   standing on a bridge cell
{
    // recovery: scatter / FindPassableCellNearUnit / Stop_Moving
}
```
So a unit standing on a bridge cell (`0x100`) whose `OnBridge` byte is 0 is forced into recovery **even when adjacent to its goal** — the desync between "on a bridge cell" and "not marked on-bridge" is treated as un-finishable and triggers scatter/re-seek. This is the one place ramp/bridge state changes the repath decision.

---

## 5. YR-active confirmation (not TS-legacy)

All of the above is live in a standard YR skirmish:
- The `OnBridge` predicate is inline in `WalkLocomotionClass__ProcessMovement`, `DriveLocomotionClass` drive-track, and `ShipLocomotionClass__Process_Movement`, each called unconditionally by their `Process` entry — no `SpecialFlags` / TS gate wraps the branch (per transition-sequence doc OQ9, re-confirmed here from the walk body).
- `Find_Path`'s 24-buffer refill is on the mainline movement path for all ground movers.
- No INI key controls the sequencing (searched `rulesmd.ini`/`artmd.ini`; only bridge strength/repair/`TooBigToFitUnderBridge`/`ZFudgeBridge` exist — none touch `+0x8C`, the dual lists, or the `-4` predicate).
- This is high-bridge (`0x100` + signed `Level-4`) behavior — **distinct from** low-bridge/tube (`TubeClass`), which is out of scope and does not set `OnBridge`.

---

## 6. Current Rust surface (what this corrects/enables)

The predicate itself is already correct and tested (non-ignored):
- `src/sim/movement/movement_bridge.rs::compute_bridge_transition` implements exactly the §2.1 predicate (signed i8, `dst_h == src_h-4 && dst.has_structural_bridge()` for Set; `!dst.structural && src.structural` for Clear).
- `resolve_cell_transition_bridge_state` + `apply_pending_bridge_render_state` drive `on_bridge`/`bridge_occupancy` independently of `loco.layer`, matching the "sticky byte, not the path layer" model.
- `PathGrid::set_cell_for_test` sets `bridge_structural = bridge_walkable` (true on ramps), and the real grid (`core.rs:1885-1896`) gives ramp cells `bridge_structural=true` AND `transition=true` — so `has_structural_bridge()` == gamemd `0x100` (set on ALL bridge cells incl. ramps). **Guardrail:** the predicate's "structural bridge flag" must stay true on ramp/bridgehead cells, or Set-on-ramp→body and Clear-on-ramp→ground break.
- Non-ignored tests already pin single-step ramp→body Set and ramp→ground Clear (`on_bridge_fires_at_ramp_to_body_only`, `on_bridge_clears_at_ramp_to_ground_only`, `no_bridge_lookahead_pre_claim`).

The **four `#[ignore]`d tests** name machinery that is not yet landed: the ship-locomotor relink variant, the multi-crossing-per-tick persistence, and the segment-exhaustion repath. None require changing the predicate.

---

## 7. Rust Implementation Handoff

### 7.1 `ship_high_bridge_ramp_to_body_relinks_after_on_bridge_update` (movement_tests.rs ~:2144)
- **Binary contract:** the relink sequence in §2 is locomotor-agnostic. Ship (`0x006A1C80`) uses the same `Get_Effective_Height`/`GetGroundHeight` + `vtable+0x124` remove/add machinery (verified `get_function_callees 0x006A1C80`) and the same `+0x8C` selector; a ship crossing ramp→body must Set OnBridge=1 and move from the ground object list at the old cell to the bridge object list at the new cell.
- **Required Rust effect:** the Ship locomotor branch in `movement_tick`/`movement_step` must run the same `resolve_cell_transition_bridge_state` → `OccupancyGrid` remove(old, pre-transition layer) → add(new, post-transition layer) as Drive/Walk. No ship-specific bypass of the bridge predicate.
- **Acceptance:** after one 500ms tick over path `(1,1)ramp→(2,1)body`: `entity.on_bridge == true`, `bridge_occupancy.deck_level == 4`, old cell `(1,1)` has 0 occupants on both layers, body cell `(2,1)` has `count_on(Bridge)==1` and `count_on(Ground)==0`.

### 7.2 `on_bridge_fires_at_ramp_to_body_only` (movement_tests.rs ~:2218)
- **Binary contract:** §2.1 predicate + §2.3 ordering. Set fires only at ramp→body; the bridge-list insert happens **after** the OnBridge write.
- **Required Rust effect:** projected `on_bridge` (post-transition) selects the insertion layer; ground→ramp must NOT pre-claim the bridge list.
- **Acceptance:** pre-tick `on_bridge==false`; after ramp→body tick: `on_bridge==true`, `bridge_occupancy.deck_level==4`, `(2,1).count_on(Bridge)==1`, `count_on(Ground)==0`.

### 7.3 `on_bridge fires/clears` multi-tick (already covered — clear at ramp→ground)
Covered by non-ignored `on_bridge_clears_at_ramp_to_ground_only`. Contract: body→ramp keeps ON (Unchanged), ramp→ground clears (src `0x100`, dst not) → `bridge_occupancy=None`, ground-list insert.

### 7.4 `multi_crossing_preserves_first_bridge_set_update` (movement_tests.rs ~:2503)
- **Binary contract:** §3. OnBridge is sticky; per-boundary predicate is applied **sequentially** over the persistent state within a single movement pass (gamemd re-enters ProcessMovement per step). First Set survives later Unchanged.
- **Required Rust effect:** when a tick advances the unit across >1 cell boundary, iterate boundaries in path order, threading the running `on_bridge`/`bridge_occupancy` through each `resolve_cell_transition_bridge_state`; do NOT recompute from the current cell alone or reset between crossings. The final committed state and the final cell's occupancy layer must reflect the last boundary, but a mid-sequence Set must persist through subsequent Unchanged crossings.
- **Acceptance:** speed 1024 lep/s over `(1,1)ramp→(2,1)body→(3,1)body` in one 500ms tick: final cell `(3,1)`, `on_bridge==true`, `bridge_occupancy.deck_level==4`, `(3,1).count_on(Bridge)==1`, `count_on(Ground)==0`.

### 7.5 `test_segment_exhaustion_repath_avoids_friendly_building_footprint` (movement_tests.rs ~:1976)
- **Binary contract:** §4. On buffer exhaustion (`next_index >= path.len()` and not at `final_goal`), re-plan a fresh A\* from the current cell to the persistent `final_goal` against **live occupancy** (building footprints block). The 24-cap is storage-only; the re-plan sees obstacles beyond the prior segment. Bridge traversal in the re-plan is via the normal legality gate (`CheckBridgeTraversal`), not a special case. Speed-ramp state is preserved across the refill (gamemd keeps the unit moving; it does not reset velocity at a segment seam).
- **Required Rust effect:** `handle_path_exhaustion` must (a) fire when the segment is consumed and `final_goal` not reached, (b) build the block set from the live `EntityStore` (`build_entity_block_set`) so Structures block, (c) re-plan to `final_goal`, (d) preserve `speed`/`current_speed`/accel/decel/`final_goal`. The current implementation at `movement_tick.rs:227` (`handle_path_exhaustion`) already does all four; landing = removing `#[ignore]` once the call path is wired for the test's entity-block threading.
- **Acceptance:** with a 2x2 Structure footprint at cell 30 (beyond the first 24-step segment) and a very fast tank moving `(1,2)→(40,2)`, the first post-exhaustion path (first cell != start) contains **no** foundation cell.
- **Note / possible divergence to watch:** gamemd's *initial* A\* already sees the building (full-route plan), so in gamemd the unit never heads into the footprint at all; the Rust test only exercises the repath because its initial command was intentionally obstacle-blind (`entity_blocks=None`). Parity requires that **every** A\* invocation (initial and each refill) is planned against live occupancy — if the Rust initial-command path ever plans without the block set, it will visibly aim at obstacles until the next segment, which gamemd never does.

---

## 8. Remaining uncertainty

- **Ship OnBridge inline writer not byte-verified this pass.** The relink machinery (shared `Get_Effective_Height`/`GetGroundHeight`/`vtable+0x124` + `+0x8C` selector) is confirmed by callee list; the exact `MOV byte [obj+0x8C]` site inside `0x006A1C80` was not disassembled (function is ~50KB). Drive and Walk writers are byte-verified. Confidence the ship mirrors them: HIGH (shared selector + prior-doc corroboration), but not PROOFED. Next step: disassemble `0x006A1C80` and locate the `[obj+0x11B]-4` compare + `[obj+0x8C]` write.
- **Multi-cell-per-frame in gamemd vs multi-crossing-per-tick in Rust.** gamemd advances by unit speed per frame and re-enters ProcessMovement to consume consecutive steps; Rust advances by variable `ms` per tick. The *state contract* (sticky byte, sequential per-boundary Set/Clear/Unchanged) is identical; the exact number of boundaries crossed per unit-of-time is a speed/timestep concern, not a state-machine concern.
- **`+0x640` cached goal coord** is used by the buffer-refill clamp; its precise writer wasn't fully traced (not needed for the state contract).

---

## 9. Docs reviewed / relationship
- Extends (no contradictions found): `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`, `HIGH_BRIDGE_ONBRIDGE_OCCUPANCY_TRANSITION_SEQUENCE_GHIDRA_REPORT.md`, `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`, `GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md`.
- New contribution: the `FootClass+0x5E0` 24-step buffer, `Find_Path 0x004D3920` refill/failure model, the bridge-gated recovery branch, and the walk-locomotor byte-level relink ordering tied directly to the 4 ignored tests.

## Sources (Ghidra calls this session)
- `list_instances` — bridge up, `gamemd.exe` open (testProsjekt).
- `decompile_function 0x004D3920` (`FootClass__Find_Path`, PROOFED) — 24-buffer clamp, refill, bridge-gated recovery.
- `decompile_function 0x004DC810` (`FootClass__SetPathIndex`) — path-cursor fields `+0x520/+0x524/+0x528`, `-1`=no-path.
- `disassemble_function 0x0075AEC0` (`WalkLocomotionClass__ProcessMovement`) — OnBridge predicate `0x0075C154..0x0075C193`, remove/add ordering `0x0075C11C`/`0x0075C1AE`, buffer shift `0x0075B3D5`/`0x0075BD9A`, self-recursion `0x0075B716`/`0x0075BABB`/`0x0075BC04`.
- `get_function_callers 0x004D3920` — Drive/Ship/Walk Process_Movement.
- `get_function_callees 0x006A1C80` (`ShipLocomotionClass__Process_Movement`) — shared height/relink machinery + `FootClass__Find_Path`.
- `search_functions "Path"` — function inventory.
- Cross-checked INI: `ini/rulesmd.ini`, `ini/artmd.ini` (no key governs the sequence).

---

## 10. Ship locomotor OnBridge writer (follow-up) — 2026-07-20

Closes §8's open item ("Ship OnBridge inline writer not byte-verified"). One correction to §8's
premise: the ship's writer is **NOT inside `Process_Movement @ 0x006A1C80`** — that function only
READS `+0x8C`. The writers live in **`ShipLocomotionClass__Process_Drive_Track @ 0x006A05F0`**
(body `0x006A05F0..0x006A1C58`, role verified from the body: drive-track step loop over
`g_DriveTrackDescriptors`/`g_DriveTrackStepArrays`, 7-lepton step consumption, track-terminator
jump, fractional tail step — `decompile_function 0x006A05F0`; called only by
`ShipLocomotionClass__Process @ 0x0069FC10`, `get_function_callers 0x006A05F0`).

### 10.1 Writer inventory (exhaustive, PROOFED)
Binary-wide exact-byte scans for `MOV byte [reg+0x8C], imm8` (`C6 80..87 8C 00 00 00 00/01`, 12
`search_byte_patterns` runs — the tool's mask arg is ignored, so each ModRM variant was scanned
exactly) plus all-mnemonic `search_instructions` operand scans (`0x8c]`) inside
`ShipLocomotionClass__Process` (1 read @ `0x0069FE79`, CMP), `__Process_Movement` (2 reads, §10.5),
and `__Process_Drive_Track` (2 writer pairs + 1 read @ `0x006A0F07`, nothing else):

| Site | SET (`:=1`) | CLEAR (`:=0`) | Role (from `decompile_function 0x006A05F0`) |
|---|---|---|---|
| A — mid-track cell crossing (per-step loop, new cell ≠ old cell) | `0x006A0EBC` (`C6 82 8C 00 00 00 01`) | `0x006A0ED6` (`C6 80 ... 00`) | normal boundary crossing while consuming 7-lepton track steps |
| B — fractional tail step (leftover `remaining/7` via `CoordStruct__ScaleByFactor(…, 0.14285715)`) | `0x006A1BD7` (`C6 86 ... 01`) | `0x006A1BF1` (`C6 80 ... 00`) | sub-step move at end of the movement pass that still crosses a cell |

Bytes read back via `disassemble_bytes 0x006A0E50..0x006A0F30` and `0x006A1B60..0x006A1C58`.

### 10.2 Predicate — EXACTLY the Walk predicate (PROOFED at instruction level)
Site A (`disassemble_bytes 0x006A0E50..0x006A0F30`):
```
0x006A0E58  CALL [vtbl+0x124] (arg 0)      ; REMOVE from OLD cell (pre-write)
0x006A0E68  CALL [vtbl+0x1B4]              ; coordinate commit to new coord
0x006A0E7B  CALL 0x005657A0                ; MapClass__Get_CellClass (ECX=g_Map 0x87F7E8) → ESI = src cell
0x006A0E8C  CALL 0x005657A0                ; → EBX = dst cell
0x006A0E93  MOVSX EAX, [ESI+0x11B]         ; src.Level (signed i8)
0x006A0E9A  MOVSX ECX, [EBX+0x11B]         ; dst.Level (signed i8)
0x006A0EA1  SUB EAX, 4
0x006A0EA8  CMP ECX, EAX                   ; dst.Level == src.Level - 4 ?
0x006A0EB1  TEST [EBX+0x140], 0x100        ; dst has structural bridge flag ?
0x006A0EBC  MOV byte [obj+0x8C], 1         ; SET (both true)
0x006A0EC3  TEST [EBX+0x140], 0x100        ; dst IS bridge → skip clear
0x006A0ECB  TEST [ESI+0x140], 0x100        ; else src IS bridge …
0x006A0ED6  MOV byte [obj+0x8C], 0         ; CLEAR
```
Site B is the same sequence on `[ECX+0x11B]`(src)/`[EAX+0x11B]`(dst) at `0x006A1BB2..0x006A1BF1`.
**Set:** `dst.Level == src.Level − 4 && dst.Flags&0x100`. **Clear:** `!(dst.0x100) && src.0x100`.
Set priority, signed `MOVSX`, `0x100` flag, remove-before-write / add-after — all identical to
Walk §2.1/§2.3 and Drive. src/dst binding: PROOFED for site A (the SET-tested cell pointer is
reused immediately after as the crush-victim cell of the just-entered cell, and `+0x124(1)` adds
into it); HIGH for site B (identical instruction shape + add-order; per-register coord binding not
separately traced).

Drive re-verified at byte level this pass (upgrades §1.1's doc-cited claim to PROOFED):
`0x004B1830`/`0x004B184A` (predicate `0x004B1807..0x004B1845`) and `0x004B2586`/`0x004B25A0`
(predicate `0x004B2561..0x004B259B`), both inside `DriveLocomotionClass__Process_Drive_Track
@ 0x004B0F20` — instruction-identical to the ship sites (`disassemble_bytes 0x004B17F0..0x004B1858`,
`0x004B2546..0x004B25B0`). Ship's Process_Drive_Track is a near-clone of Drive's.

### 10.3 Ship-under-bridge guard: emergent, not special-cased
There is **no ship-specific check**. A ship sailing under a high bridge crosses water→water bridge
cells: `dst.Level == src.Level` (flat water), so the Set's `−4` delta test fails; the Clear is
skipped because dst has `0x100`. The byte stays 0 (Unchanged) and the ship remains on the
ground/water list (`CellClass+0xE4`) — the deck stays independently occupiable. The Level−4 delta
IS the guard. (A ship could only ever Set OnBridge by dropping 4 levels into a `0x100` cell, which
water topology never produces.)

### 10.4 `vtable+0x1CC` (Set_Height_On_Bridge / per-cell Z snap) ordering — differs from Walk
- **Site A:** predicate → crusher pass → **ADD(1) @ `0x006A101F`** → `+0x1CC(0)` @ `0x006A10D8`
  (with `obj+0x74` temporarily zeroed; call shared with the same-cell step path). Walk does
  `+0x1CC` BEFORE the add (`0x0075C1A1` < `0x0075C1AE`); ship site A does it AFTER.
- **Site B (tail):** predicate → ADD(1) @ `0x006A1C02` → return. **No `+0x1CC` at all** — the Z
  snap waits for the next pass.
- **Track-terminator jump** (step array hits its 0,0 terminator; ship snaps to the stored exit
  coord `this+0x40`): remove(0) @ `0x006A16B4` → `+0x1B4` → `+0x1CC(0)` @ `0x006A16CD` →
  ADD(1) @ `0x006A16DA` — a genuine cell relink that **does NOT touch OnBridge** (sticky byte
  keeps the layer on both ends). `disassemble_bytes 0x006A1570..0x006A1715` +
  `decompile_function 0x006A05F0` (branch `iStack_58==0 && iStack_54==0 && step!=0`).

### 10.5 Ship-side readers (context, verified this pass)
- `0x006A0F07` (Process_Drive_Track, right after site A's write): crusher pass picks the victim
  list from the fresh byte — OnBridge=1 → bridge list `cell+0xE8`; OnBridge=0 → `cell+0xE4` unless
  the unit's Z ≥ ground + `g_BridgeZ_Offset` (then `+0xE8`).
- `0x006A296C` (Process_Movement): height base = `cell.Level + (OnBridge ? 4 : 0)` (NEG/SBB/AND-4
  idiom, `disassemble_bytes 0x006A2940..0x006A29F0`).
- `0x006A29D6` (Process_Movement): if `OnBridge XOR ((cell.Flags>>8)&1)` — byte disagrees with the
  cell's `0x100` flag — sets `obj+0x68B := 1` (desync marker byte; role of `+0x68B` UNVERIFIED).
- `0x0069FE79` (Process): CMP read only.

### 10.6 Ship locomotor CLSID (verified from binary data)
GUID bytes `E1 74 EA 2B CA 7C D3 11 BE 14 00 10 4B 62 A1 6C` = `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}`
found at `0x007E9AB0` (`search_byte_patterns`), referenced from WinMain `0x006BD3BD` (factory
registration), `BuildingClass__MissionRepairAndProduce 0x0044C780/0x0044C78B` (creates the ship
locomotor for produced naval units), and code at `0x006A3E70` in the ship-locomotor region (no
function defined there; presumed GetClassID — role UNVERIFIED, left undisturbed). Confidence:
HIGH the GUID is the ship locomotor CLSID (usage-site corroboration).

### 10.7 Rust handoff (movement_bridge.rs ship path)
The ship path must run the **same** `compute_bridge_transition` predicate as Drive/Walk at every
cell boundary — no ship special case — because the water-level guard is emergent from the shared
`−4` delta; plus: (a) boundary order remove(old, pre-write layer) → commit coord → predicate →
add(new, post-write layer); (b) relinks that aren't level-qualifying boundary crossings (track-jump
style teleports to a same-purpose coord) keep the sticky `on_bridge` unchanged while still
re-linking occupancy; (c) deck-Z snap after the add on normal crossings and skipped entirely on the
fractional tail crossing (§10.4) — only relevant once ship Z rendering is tied to the snap.

### 10.8 Sources (Ghidra calls, 2026-07-20 session)
`get_function_by_address 0x006A1C80 / 0x006A05F0 / 0x004B1830 / 0x004B2586 / 0x005657A0`;
`search_instructions` (operand `0x8c]`) in `ShipLocomotionClass__Process{,_Movement,_Drive_Track}`;
`search_byte_patterns` ×12 (`C6 8x 8C 00 00 00 00/01`) + GUID bytes; `read_memory 0x0075C179 /
0x0075C193`; `disassemble_bytes 0x006A0E50..0x006A0F30, 0x006A0FF5..0x006A10F5,
0x006A1570..0x006A1715, 0x006A1B60..0x006A1C58, 0x004B17F0..0x004B1858, 0x004B2546..0x004B25B0,
0x006A2940..0x006A29F0`; `decompile_function 0x006A05F0`; `get_function_callers 0x006A05F0`;
`get_xrefs_to 0x007E9AB0`.

---

## 11. Implementation correction note (2026-07-20, Rust-side landing)

All four §7 tests are now landed and passing. Two corrections to §7's Rust guidance
discovered during landing (binary model unchanged):

1. **§7.1/§7.2/§7.4 fixture flag semantics were WRONG for Rust.** The fixtures were
   prescribed with body cells `transition=false`. In this port, `PathCell.transition`
   models the `0x200` lane flag, which the CERTIFIED bridge stamping
   (`src/map/bridge_facts.rs` stamp slots) sets on **Anchor + Forward1 — every
   crossable deck lane cell** — and strips only from the Forward2 edge lane. A
   `transition=false` structural cell is therefore a NON-crossable edge cell, not a
   normal body cell; the fixtures modeled an unreal bridge. Corrected fixtures stamp
   body lane cells `transition=true`.
2. **Runtime-vs-plan gate divergence (real engine fix).** The runtime crossing
   evaluation (`evaluate_runtime_can_enter_cell`) ran `check_bridge_traversal`
   unconditionally, while the A* neighbor expansion gates it behind
   `needs_bridge_traversal_for_edge` — so the runtime rejected deck edges the plan
   legally produced (consistent with §2: the original's locomotor relink re-checks
   NOTHING at the boundary). Fixed by mirroring the A* gate at the runtime call site.
