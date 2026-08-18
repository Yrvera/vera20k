# CheckBridgeTraversal + CellClass Offsets + Flag-Bit Semantics — Ghidra Research Report

**Phase:** Phase 2 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #15 (CheckBridgeTraversal @ 0x4D9C60), #16 (CellClass offsets), #17 (cell-flag bit semantics)
**Companion doc:** `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` (items #10-#14 — per-class Can_Enter_Cell)
**Date:** 2026-05-13
**Active in YR:** All findings live in retail YR.

> Every claim cites a Ghidra address + decompilation or `read_memory` byte dump.
> Confidence axes: **C**=content / **I**=identity / **B**=binding.

---

## 1. CheckBridgeTraversal (0x4D9C60) — the bridge traversal validator

Function size: 0x4D9C60..0x4D9F? (~480 bytes). Registered at:
- UnitClass vtable+0x1B0 (slot 0x7F5E20)
- InfantryClass vtable+0x1B0 (slot 0x7EB208)
- FootClass vtable+0x1B0 (slot 0x7E8E44)

**NOT registered for** AircraftClass (`vtable+0x1B0 = ObjectClass::DrawIt`) or BuildingClass (`vtable+0x1B0 = AnimClass__Click_stub` returning 0).

### 1.1 Signature

```c
undefined4 CheckBridgeTraversal(
    int param_1,        // CellClass* candidate target cell
    int param_2,        // direction code 0..7 (or -1 for "no direction")
    int *param_3,       // *path_height pointer — INPUT/OUTPUT
    undefined1 *param_4,// *bridge_entered output flag pointer
    int param_5         // CellClass* parent/current predecessor cell; may be 0
);
```

Returns `0` (OK to enter) or `7` (Blocked).

Note: NOT a `__thiscall` despite being invoked via vtable — the args are passed entirely via stack. The vtable slot's "this" pointer becomes `param_1` (cast to int). So the calling convention is **stdcall via vtable dispatch**.

**2026-05-15 audit correction:** older wording in this report named `param_1` "SOURCE" and
`param_5` "DEST". Headless Ghidra re-check plus the focused parent-fallback report confirm the active
mapping above: `param_1` is the candidate target cell; `param_5` is the optional parent/current
predecessor cell. Unit and Infantry pass their `Can_Enter_Cell` arg4 through as this arg5; A*
neighbor expansion supplies an explicit current-node cell to `Can_Enter_Cell`.

### 1.2 Step 0 — compute parent/predecessor cell if not provided

```c
if (param_5 == 0) {
  uint dir_minus_4 = (param_2 - 4) & 7;                                   // 180° rotated
  CoordStruct parent_coord = param_1.coord + g_DirectionOffsets[dir_minus_4];
  param_5 = MapClass::Get_CellClass(&parent_coord);
}
```

**Asymmetry trap:** `(param_2 - 4) & 7` rotates the direction by 180°. This fallback computes a
predecessor cell opposite the supplied direction relative to the candidate cell. For normal adjacent
movement this usually reconstructs the current cell; for lookahead/probe calls it reconstructs the
previous edge cell, not necessarily the object's current occupied cell.

### 1.3 Step 1 — invalid direction handling (param_2 == -1)

```c
if (param_2 == -1) {
  if (*param_3 == -1 && (param_1.Flags & 0x100) != 0) {
    *param_3 = param_1.Level + 4;                                          // bridge deck = ground + 4
  }
  return 0;
}
```

When the caller doesn't have a direction (e.g., teleport-style entry), if `path_height` is unset (-1)
and `param_1` is a bridge cell, **set path_height to `param_1.Level + 4`**. Return OK.

### 1.4 Step 2 — initial path_height assignment from parent/predecessor cell

```c
if (param_5 != 0 && param_1 != 0) {
  if (*param_3 == -1 && (param_5.Flags & 0x100) != 0) {
    *param_3 = param_5.Level + 4;                                          // bridge deck for parent/predecessor
    if ((param_1.Flags & 0x200) == 0) return 7;                            // candidate NOT a bridgehead -> blocked
  }
  ...
}
```

If the parent/predecessor cell is a bridge cell (`param_5.+0x140 & 0x100`) and `path_height` is
unset, set it to `param_5.Level + 4` (bridge deck). **But require the candidate cell to be a
bridgehead** (`0x200`). Otherwise blocked.

**Player effect:** units cannot teleport onto bridge mid-span. They must enter via a bridgehead.

### 1.5 Step 3 — compute height delta and switch on it

```c
int p1_level = (signed char)param_1.Level;                                 // signed!
int p5_height;
if (param_5.Flags & 0x100) {
  p5_height = (signed char)param_5.Level;                                  // parent/predecessor is bridge -> use its Level
} else {
  p5_height = *param_3;                                                    // parent/predecessor not bridge -> use caller's path_height
}
int diff_signed = p5_height - p1_level;
int diff_abs = abs(diff_signed);
```

Now branch on `diff_abs`:

#### Step 3a — diff == 0 (level move)

```c
if (diff_abs == 0) {
  if (((param_1.Flags & 0x100) == 0 ||                                     // candidate NOT bridge
       (param_1.Flags & 0x200) == 0 ||                                     // candidate NOT bridgehead
       (param_5.Flags & 0x100) == 0) &&                                    // OR parent/predecessor NOT bridge
      *param_3 != -1 && *param_3 != p1_level) {
    return 7;
  }
}
```

For a level-height move, if any of `{candidate.bridge, candidate.bridgehead, parent.bridge}` is unset
AND `path_height` does not match `candidate.Level`, return blocked. This is the "level-walk
path_height-consistency" gate.

#### Step 3b — diff_abs == 1 (ramp move)

```c
else if (diff_abs == 1) {
  if (diff_signed < 1) {
    // param_5-derived height is lower than param_1.Level: check param_5.SlopeIndex
    if (param_5.SlopeIndex /* +0x11C */ == 0) return 7;
  } else {
    // param_5-derived height is higher than param_1.Level: check param_1.SlopeIndex
    if (param_1.SlopeIndex /* +0x11C */ == 0) return 7;
  }
}
```

**Verified**: ramp passability uses `cell.+0x11C` (SlopeIndex byte). If 0, blocked. The check is
**asymmetric** by function-local height direction:
- If `p5_height < param_1.Level`, check **param_5** SlopeIndex.
- If `p5_height > param_1.Level`, check **param_1** SlopeIndex.

This is "the cell whose slope you're trying to roll off / roll up" gates the move.

#### Step 3c — diff_abs == 4 (bridge entry/exit)

```c
else if (diff_abs == 4) {
  // Bridge-deck height transition. Two sub-cases:
  if (param_5.Level == p1_level - 4) {                                     // param_1 is HIGH, param_5 is LOW
    if (*param_3 != p1_level) return 7;                                    // path_height must match param_1
    if ((param_5.Flags & 0x100) == 0) return 7;                            // param_5 must be a bridge cell
  }
  if (p1_level == param_5.Level - 4) {                                     // param_1 is LOW, param_5 is HIGH
    if ((param_1.Flags & 0x100) == 0) return 7;                            // param_1 must be bridge cell
    if ((param_1.Flags & 0x200) == 0) return 7;                            // param_1 must be bridgehead
    *param_4 = 1;                                                          // bridge_entered = true
    return 0;
  }
}
```

The 4-level diff handles BOTH:
- **Function-local high-to-low case**: `path_height` must already match `param_1.Level`, and
  `param_5` must be a bridge cell.
- **Function-local low-to-high case**: `param_1` must be a bridge cell AND a bridgehead. Sets
  `*param_4 = 1` to tell the caller "this move just put you on the bridge deck".

**Asymmetry alert**: `*param_4 = 1` is ONLY set in the ascending case. Descending DOES NOT set it. Subtle.

#### Step 3d — diff_abs anything else

```c
else {
  return 7;                                                                // diff is 2, 3, 5, 6, 7, ... → blocked
}
```

Any height delta of 2, 3, 5+ is hard-blocked. Only deltas 0 / 1 / 4 are legal moves.

### 1.6 Step 4 — fallthrough success

```c
return 0;
```

If we got here, the move is OK.

### 1.7 Summary of the 4 valid move shapes

| diff_abs | Required `param_1` state | Required `param_5` state | Additional gate | bridge_entered set? |
|----------|--------------------------|--------------------------|-----------------|---------------------|
| 0 | participates in path-height check | participates in path-height check | `path_height == -1`, `path_height == param_1.Level`, OR all of `{param_1.bridge, param_1.bridgehead, param_5.bridge}` are set | no |
| 1, `p5_height > param_1.Level` | `SlopeIndex != 0` | — | — | no |
| 1, `p5_height < param_1.Level` | — | `SlopeIndex != 0` | — | no |
| 4, `param_1` low / `param_5` high | bridge AND bridgehead | high-side level | — | **YES** (`*param_4 = 1`) |
| 4, `param_1` high / `param_5` low | high-side level | bridge | `path_height == param_1.Level` (= deck) | no |

### 1.8 Caller binding

**2026-05-15 binding correction:** read the table above with `param_1` as the candidate target
cell and `param_5` as the parent/current predecessor cell. `param_5 == 0` is meaningful: with a
valid direction, `CheckBridgeTraversal` reconstructs the predecessor from `candidate +
DirectionOffset[(direction - 4) & 7]`; with `direction == -1`, it uses candidate-only height seeding
and skips directed bridgehead/diff/slope checks.

Callers via vtable dispatch (no static caller list — virtual dispatch). Reached from:
- UnitClass::Can_Enter_Cell @ 0x73F0E0 (call site `(*this->vtable[0x1B0])(...)`)
- InfantryClass::Can_Enter_Cell @ 0x51C... (call site `(*this->vtable[0x1B0])(...)`)
- FootClass-base instances (theoretical — abstract class, never instantiated alone)

Confidence: C=HIGH (full body decompiled), I=HIGH (Ghidra label "CheckBridgeTraversal" matches behaviour), B=HIGH (vtable+0x1B0 occupancy verified for UnitClass/InfantryClass/FootClass via `read_memory`).

Additional 2026-05-15 headless Ghidra evidence:

- UnitClass `Can_Enter_Cell @ 0x0073F0A0` pushes `Can_Enter_Cell` arg4 (`[ESP+0xA0]`) as
  `CheckBridgeTraversal` arg5 at `0x0073F2D6`, then calls `[vtable+0x1B0]` at `0x0073F2EB`.
- InfantryClass `Can_Enter_Cell @ 0x0051BF90` pushes `Can_Enter_Cell` arg4 (`[ESP+0x44]`) as
  `CheckBridgeTraversal` arg5 at `0x0051C0D7`, then calls `[vtable+0x1B0]` at `0x0051C0E6`.
- `AStar_main_loop @ 0x00429A90` calls `[vtable+0x1AC]` at `0x00429F54` with candidate cell,
  neighbor direction, current node/path height, explicit current-node cell, and the low byte of
  `Pathfinder+0x08`. Unit/Infantry then forward that explicit current-node cell into
  `CheckBridgeTraversal` arg5.

Therefore normal A* uses explicit-parent directed traversal. Runtime locomotor probes that pass
parent/current cell `0` rely on the fallback in section 1.2. Direction `-1` calls are a separate
candidate-only height-seeding mode.

---

## 2. CellClass field offsets — verified

`get_struct_layout("CellClass")` returns size 328 (0x148) bytes. Key offsets for pathfinding:

| Offset | Hex | Size | Type | Field | Pathfinding role |
|--------|-----|------|------|-------|------------------|
| 36 | +0x24 | 2 | short | MapCoord_X | Cell's map X (for direction encoding) |
| 38 | +0x26 | 2 | short | MapCoord_Y | Cell's map Y |
| 52 | +0x34 | 4 | ptr | LightConvert | (rendering) |
| 56 | +0x38 | 4 | int | IsoTileTypeIndex | Theater tile index (used by IsBridge check) |
| 60 | +0x3C | 4 | ptr | AttachedTag | (script triggers) |
| 68 | +0x44 | 4 | int | OverlayTypeIndex | Wall / debris overlay (UnitClass::Can_Enter_Cell §9 reads this) |
| 72 | +0x48 | 4 | int | SmudgeTypeIndex | (rendering) |
| 116 | +0x74 | 1 | byte | (used in BuildingClass::Can_Enter_Cell as "active construction" flag) | construction-time gate |
| 118 | +0x76 | — | — | — | — |
| 224 | +0xE0 | 4 | ptr | Jumpjet | **JumpJet occupier list head (separate layer!)** |
| 228 | +0xE4 | 4 | ptr | FirstObject | **GROUND occupancy list head** |
| 232 | +0xE8 | 4 | ptr | AltObject | **BRIDGE occupancy list head** |
| 236 | +0xEC | 4 | int | LandType | LandType enum (Clear/Water/Tunnel/etc.) — used in passability matrix |
| 240 | +0xF0 | 8 | double | RadLevel | (radiation) |
| 248 | +0xF8 | 4 | ptr | RadSite | (radiation) |
| 278 | +0x116 | 2 | short | TubeIndex | **Tube/tunnel index, -1 if no tube.** Used in main A* loop's direction-8 path |
| 282 | +0x11A | 1 | byte | Height | **Dual semantic: terrain height for normal cells, tube sub-direction byte for tube cells (LandType==10), values 2 or 6** |
| 283 | +0x11B | 1 | i8 | Level | **Primary height level used in pathfinding (SIGNED — MOVSX everywhere)** |
| 284 | +0x11C | 1 | byte | SlopeIndex | **Ramp passability — read in CheckBridgeTraversal at diff==1** |
| 285 | +0x11D | 1 | byte | (HeightInPixels — computed in RecalcAttributes as (height_raw-30)/15) | render |
| 292 | +0x124 | 4 | u32 | OccupationFlags | **GROUND occupancy bits** — low byte = "is occupied", bit 5 = "has stationary object" |
| 296 | +0x128 | 4 | u32 | AltOccupationFlags | **BRIDGE occupancy bits** — same layout as +0x124 but for bridge layer |
| 320 | +0x140 | 4 | u32 | Flags | **Cell flags bit field** — see §3 |

### 2.1 Bytes I confirmed by hand

| Offset | Read at | Where | Evidence |
|--------|---------|-------|----------|
| +0x124 (ground occupancy) | UnitClass::Can_Enter_Cell @ 0x73F0F4 | `MOVSX [..., (cell.+0x124 & 0xFF)]` | low byte read as occupancy bit |
| +0x128 (bridge occupancy) | UnitClass::Can_Enter_Cell @ 0x73F303 (post-vtable phase) | similar pattern reading `cell.+0x128` | confirms layer-swap on `path_height == cell.Level + 4` |
| +0xE4 (FirstObject) | UnitClass::Can_Enter_Cell occupier-walk Phase 10 | `piVar15 = cell.+0xE4` then `piVar15 = piVar15.+0x30` (next pointer) | linked list with `next` at object+0x30 |
| +0xE8 (AltObject) | same Phase 10, bridge branch | same pattern | same linked-list semantic |
| +0xEC (LandType) | UnitClass::Can_Enter_Cell @ 0x73F4... | `g_SpeedType_LandType_Table[cell.+0xEC * 9 + locomotor_speed_cat]` | LandType used as passability index |
| +0x140 (Flags) | every bridge-check site | `cell.+0x140 & 0x100` / `& 0x200` / etc. | universal cell-flags slot |
| +0x11B (Level) | CheckBridgeTraversal, A* main loop §3 | `MOVSX EAX, byte ptr [cell + 0x11B]` (signed) | signed byte, height level |
| +0x11C (SlopeIndex) | CheckBridgeTraversal §3b | `if (cell.+0x11C == 0) return 7` | ramp gate |
| +0x116 (TubeIndex) | A* main loop direction-8 path | `g_TubeArray[(short)cell.+0x116 * 4]` | -1 = no tube |
| +0x122 (?) | UnitClass::Can_Enter_Cell @ 0x429EB1 | `*(char *)(cell + 0x122) == '\0'` with `param_7 != '\0'` causes skip | **Open question — likely amphibious-water gate** |

### 2.2 Critical layout shape — what determines GROUND vs BRIDGE

| Layer | Object list head | Occupancy bits | Layer-decision rule |
|-------|------------------|----------------|---------------------|
| **GROUND** | `cell.+0xE4` (FirstObject) | `cell.+0x124` (OccupationFlags) | unit's Z height ≤ cell.Level (or cell is not a bridge cell) |
| **BRIDGE** | `cell.+0xE8` (AltObject) | `cell.+0x128` (AltOccupationFlags) | unit's Z height = cell.Level + 4 AND cell.flags & 0x100 |

These two layers can BOTH be populated simultaneously — a vehicle on the deck and another on the ground below the bridge co-exist in the same cell (different lists).

### 2.3 The `+0x122` byte — open question

Multiple sites in UnitClass::Can_Enter_Cell read `cell.+0x122` as a single byte. Inferred semantics: when `*(char *)(cell + 0x122) == 0` AND a `param_7` flag is set, the cell is skipped. **Hypothesis**: this is a **"can-be-aboard-by-this-amphibious-mode"** gate. Confirmation requires tracing one of the writers (none found in this phase).

---

## 3. Cell-flag (+0x140) bit semantics — pathfinding-relevant bits

`cell.+0x140` is a 32-bit field. Each bit has independent semantics. Below is the **pathfinding-relevant** subset; full enumeration is out of scope for this phase but cross-references the BRIDGE_SYSTEM.md and BRIDGE_DEFERRED_MECHANICS docs.

### 3.1 Bit `0x80` — high-bridge SetBridgeDirection / edge-walk marker

- **Set on**: verified on the `SetBridgeDirection_NESW/NWSE @ 0x47E040/0x47E470` anchor cell when `param_3 & 1` is set. That same anchor stamp can also include `0x100` and `0x200`, so `0x80` is **not** mutually exclusive with bridgehead/transition status.
- **Source**: `SetBridgeDirection_NESW/NWSE`; `MapClass::Resize @ 0x565C10` can later reassert bridge direction for cells that still have `0x80` but lack an anchor pointer. (Per 2026-05-11 / 2026-05-12 audits plus 2026-05-15 re-check.)
- **Read by**: `UpdateBridgeEdgeTiles_High @ 0x576200` (loops while bit 0x80 set to walk body cells from anchor to bridgehead).
- **Pathfinding role**: NOT directly checked in CheckBridgeTraversal or Can_Enter_Cell. Used by the bridge state machine and zone system to delimit body cells.

### 3.2 Bit `0x100` — **on-bridge cell** (structural)

- **Set on**: ALL bridge cells (both bridgeheads AND body cells).
- **Set at**: bridge tile placement (map load) and `SetBridgeDirection`.
- **Read by** (pathfinding):
  - AStar_main_loop §3 (source/dest height bump): `(cell.flags & 0x100) → height += 4`
  - AStar_main_loop §4 (layer decision): `(cell.flags & 0x100) AND abs(height-diff) ≥ 2 → BRIDGE layer`
  - CheckBridgeTraversal §1.4, §1.5, §3a, §3c: gates entry/exit
  - DriveLocomotionClass::Set_Destination @ 0x4AFD40 (Z bump for renderer)
  - compute_edge_cost diagonal-bridge cost (Phase 1 companion doc)
- **Player-observable**: the **primary** "this cell IS a bridge" signal.

### 3.3 Bit `0x200` — **bridgehead / transition** cell

- **Set on**: bridge transition cells used by entry/exit checks. `SetBridgeDirection` can stamp it on
  the anchor-side cell and the opposite transition cell; do not assume it is exclusive with `0x80`.
- **Read by** (pathfinding):
  - CheckBridgeTraversal §1.4 (`param_1` must be bridgehead to set path_height to bridge deck)
  - CheckBridgeTraversal §3a (level-walk path_height-consistency gate)
  - CheckBridgeTraversal §3c (function-local low-to-high 4-level diff requires `param_1` bridgehead)
- **Asymmetry**: A cell can have `0x100` AND `0x200` (= bridgehead/transition). A stamped anchor can
  also have `0x80`. Model these bits independently.

### 3.4 Bit `0x400` — **unknown / not pathfinding-relevant in our scope**

No reads of `0x400` found in pathfinding code paths in this phase. Flagged for follow-up in a non-pathfinding domain phase (Phase 5 edge cases or beyond).

### 3.5 Bit `0x800` — **NS-bridge orientation flag**

- **Set on**: bridge cells whose long axis runs **North-South** (vertical bridges).
- **Read by**: `AStar_compute_edge_cost @ 0x4299F1` to choose between the NS-offset table (`0x7E3710`) and EW-offset table (`0x7E3730`) for diagonal-bridge cost lookup.
- **Player-observable**: subtly different cost weights for diagonal moves around NS vs EW bridges.

(Bit 0x800 is sometimes called the "rotation parity" or "orientation marker" in legacy TS-era code.)

### 3.6 Bit `0x40000` — **BridgeApproach** (PathfinderClass transient marker)

- **Set/cleared by**: `PathfinderClass::UpdateBridgePassability @ 0x42ACF0` via XOR-toggle (twice per A* search → net zero) — see Phase 1 companion doc §5.
- **Read by**: `AStar_compute_edge_cost @ 0x4299BC` → applies `4.0` multiplier (`g_BridgeApproach_CostMult @ 0x7E37BC`).
- **Player-observable**: cells near OTHER moving units' planned paths get 4× cost during this A* run → A* routes around them.
- **Transience**: bit is SET at the start of the A* search and CLEARED at the end. Never persists between searches.

### 3.7 Other flag bits known but not in Phase 2 scope

- `0x2000` (set by `ToggleBridgePavement` for damaged-variant) — see LAT_RETRIGGER doc
- `0x100000` / `0x200000` (zone-system bridge marks) — see CELLCLASS_ZONES_SPEED_BRIDGES doc
- `0x20000` (tube anim placed, sticky) — see BRIDGE_DEFERRED_MECHANICS audit
- `0x1000` (map-edit / fog-restricted, TS-suspect) — separate concern

### 3.8 The TS-legacy bits `0x2000` and `0x4000` (cleared but not read)

Per BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md audit: `cell.Flags` bits 0x2000 and 0x4000 are CLEARED by SetBridgeDirection but no reader exists anywhere in the bridge code path. Safe to ignore in Rust.

Wait — `0x2000` IS used by ToggleBridgePavement per the LAT_RETRIGGER doc. The 2026-05-12 audit must have been scoped narrowly. Conflict — for Phase 2 purposes, **note that 0x2000 IS read by the renderer at CellOverlay_TileDraw @ 0x004803BA reading `(Flags >> 13) & 1`**. So 0x2000 is render-only, NOT pathfinding-relevant.

---

## 4. Cross-doc contradictions resolved

### 4.1 Plan item #15 hypothesis: "CheckBridgeTraversal returns 0=OK / 7=Blocked"

**Confirmed.** Only those two return values exist in the function body. The output parameter `*param_4` separately signals "we just stepped onto the bridge deck" via the diff==4 ascending case.

### 4.2 Plan item #16 hypothesis: "CellClass +0x124 = ground occupancy, +0x128 = bridge"

**Confirmed via struct layout AND read sites.** Names: `OccupationFlags` / `AltOccupationFlags`. Each is a 32-bit field; low byte holds occupancy bits, bit 5 = "stationary object present".

### 4.3 Plan item #17 hypothesis: "0x80 = bridge anchor, 0x100 = on-bridge, 0x200 = bridgehead, 0x800 = orientation"

**Partially corrected by the 2026-05-15 audit.** `0x100`, `0x200`, and `0x800` roles are confirmed
for the inspected pathfinding/cost paths. The older `0x80` wording was too narrow: the verified
`SetBridgeDirection` stamp writes `0x80` on the anchor-side cell together with other bridge bits,
including `0x100` and `0x200`. Do not implement `0x80` as "body-only" or as mutually exclusive with
bridgehead/transition state.

### 4.4 Prior CELLCLASS_ZONES_SPEED_BRIDGES inverted-ternary concern

The 2026-05-13 CELLCLASS audit flagged that doc's pseudocode for `GetZoneID` perpendicular-walk direction was inverted. **Not addressed in this report** — GetZoneID is item #35 in the plan (Phase 4 zone system). Re-affirmed for Phase 4.

---

## 5. The dual occupancy lists — full read map

For Rust port completeness, this is the inventory of reader/writer sites for `cell.+0x124` (OccupationFlags) and `cell.+0xE4` (FirstObject) — and their bridge-layer counterparts at `+0x128` and `+0xE8`.

(Only Phase 2-relevant sites listed. Phase 5 will cover the writers comprehensively.)

| Site | Address | Reads/Writes | Layer |
|------|---------|--------------|-------|
| UnitClass::Can_Enter_Cell pre-vtable | 0x73F0F4 | reads +0x124 | ground (pre-decision) |
| UnitClass::Can_Enter_Cell post-vtable | 0x73F303-0x73F34C | reads +0x128 if path_height==Level+4 | bridge (re-snapshot) |
| UnitClass::Can_Enter_Cell occupier walk | 0x73F4F9 | iterates +0xE4 or +0xE8 by layer flag | both, layer-dependent |
| InfantryClass::Can_Enter_Cell same pattern | 0x51BFC4 / 0x51C2B0 | same as UnitClass | same |
| PathfinderClass::UpdateBridgePassability walk | 0x42AD7? | iterates +0xE4 or +0xE8 by height-diff < 4 | layer-dependent (different threshold!) |
| CheckBridgeTraversal | (no occupancy reads) | — | — |
| compute_edge_cost code-2 blocker prediction | 0x429... | iterates +0xE4 or +0xE8 via prev-cell height-diff | asymmetric < 3 threshold (Phase 1 doc) |

The four threshold values for the layer-decision (already enumerated in Phase 1 doc §4.1) all gate access to the same dual-list structures. A Rust port must match the exact threshold at each site.

---

## 6. Open Questions

1. **`cell.+0x122` byte semantics** — used in UnitClass::Can_Enter_Cell as a gate alongside `param_7`. Hypothesis: water-passable flag for amphibious units. Needs writer trace.
2. **Bit `0x400`** of `cell.+0x140` — not seen in pathfinding reads. May be a non-pathfinding flag (map-edit, rendering). Out of scope.
3. **`cell.+0x74`** byte — used in BuildingClass::Can_Enter_Cell. Likely "construction-in-progress" or "MCV-deploying-here". Not strictly bridge-related.
4. **`OccupationFlags` bit 5 (the >> 5 extraction at `0x73F0F4`)** — likely "this cell has a STATIONARY occupier" (per RA2 convention). Confirmation via writer trace.
5. **Jumpjet occupancy at +0xE0** — third layer (separate from ground/bridge). JumpJet units occupy +0xE0 list. Not explored here — Phase 3 locomotor work covers it.
6. **Tube sub-direction byte at +0x11A == 2 or 6** — per BRIDGE_DEFERRED_MECHANICS audit. Confirms which way the tube faces. Not pathfinding-critical for non-tube cells.
7. **`g_DirectionOffsets @ 0x89F688`** runtime values — BSS-initialised; read as zeros in static dump. Needs runtime trace or boot-time init function discovery.
8. **`DAT_0089F68A`** (the Y component partner of g_DirectionOffsets) — same BSS issue.

---

## 7. Current Rust Implementation Status

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| CheckBridgeTraversal as a callable predicate | [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | `on_bridge` predicate covers the diff==4 entry/exit case but does NOT exactly match the 4-shape table (diff 0/1/4) — see §1.7 |
| Diff==0 path_height-consistency gate | none | **Missing** — Rust assumes contiguous layer. The "you must already be on the deck" gate at diff==0 isn't enforced. |
| Diff==1 SlopeIndex ramp gate | partial | Some slope-cost cell-walkability filter exists, but the asymmetric `p5_height > param_1.Level` checks `param_1.SlopeIndex` / `p5_height < param_1.Level` checks `param_5.SlopeIndex` pattern is not replicated. |
| Diff==4 low-to-high sets bridge_entered output flag | partial | `movement_bridge.rs` toggles `MovementLayer::Bridge` but doesn't return a separate flag to the caller. |
| Diff==4 high-to-low requires `path_height == param_1.Level` | none | **Missing** — Rust doesn't track path_height as a separate state from the unit's actual Z. |
| CellClass +0x124/+0x128 dual occupancy | partial | `movement_occupancy.rs` has occupancy grid but layer split semantics may differ. |
| Cell flag 0x80 (SetBridgeDirection / edge-walk marker) | partial | `PathCell.bridge_walkable` is not semantically equivalent unless it comes from binary-shaped bridge stamping. |
| Cell flag 0x40000 BridgeApproach toggle | none | **Missing** (per Phase 1 doc). |
| Cell flag 0x800 NS-orientation | partial | `BridgeKind::High_NS` vs `High_EW` distinction exists but not stored as a cell-level flag — derived from bridge record. |
| Bit 0x200 bridgehead gate at diff==4 low-to-high | partial | Bridgehead detection exists in `bridge_state` but the predicate-specific gate "`param_1` must be bridgehead" is not enforced. |

---

## 8. Sources

**Ghidra functions decompiled:**
- `CheckBridgeTraversal` @ 0x004D9C60 (~480 bytes body)

**Struct layout:**
- `get_struct_layout("CellClass")` returned 328-byte layout (full table in §2)

**Memory reads:**
- (Indirect — via the Phase 1 + companion Phase 2 doc's vtable verification)

**Cross-references in this phase:**
- vtable+0x1B0 occupancy verified at slot 0x7F5E20 (Unit), 0x7EB208 (Infantry), 0x7E8E44 (Foot) — all → CheckBridgeTraversal
- vtable+0x1B0 NOT CheckBridgeTraversal at slot 0x7E23A8 (Aircraft → DrawIt) and 0x7E406C (Building → stub)

**Companion docs:**
- `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` (the per-class A* entry that calls CheckBridgeTraversal via vtable+0x1B0)
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` (Phase 1 — describes the height-diff ≥ 2 layer-decision that wraps CheckBridgeTraversal)
- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` (Phase 1 — describes the diagonal-bridge cost that consumes cell.flags & 0x800)
- `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` (prior — has the partial CellClass byte-by-byte write map)
- `CELLCLASS_ZONES_SPEED_BRIDGES.md` (prior — has the bit 0x80 / 0x100 / 0x200 origin trace; inverted-ternary in GetZoneID flagged for Phase 4)
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` (prior — bit 0x2000 for damaged-variant render)
