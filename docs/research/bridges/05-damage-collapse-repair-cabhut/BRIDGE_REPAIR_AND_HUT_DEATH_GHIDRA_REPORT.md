# Bridge Repair + Hut-Death-Destroys-Bridge — Ghidra Research Report

**Status:** **Phase 1 + Phase 2 complete, with a targeted Phase 3 extension
on 2026-05-17.** The extension resolves the C4-on-CABHUT action gate and
the destruction-side per-cell bridge-collapse tree. Remaining Phase 3
items (audio callsite details, save/load paths, edge-case enumeration) are
tracked in §19.

This document covers plan functions **#1–#15 plus 12 newly-discovered
functions** that the plan's scoping did not name. Phase 2 fully decompiled
the walker family (#8–#11), the destruction-side dispatchers (#12, #13),
the hut-registry inverse (#15), and traced the C4-Immune keystone for the
project memory bug `project_c4_bridge_hut_followup`.

**Sections at a glance:**
- §1 — Phase 1 conflict verdicts (still authoritative; §14 amends Conflict A)
- §2–§10 — Phase 1 per-function findings and synthesis
- §11 — Phase 2 expanded function inventory (12 new addresses, 4 misnamed,
  1 vanilla-YR copy-paste bug)
- §12 — **Walker bodies — full overlay state machine** (the load-bearing
  Phase 2 deliverable)
- §13 — Destruction-side tree (newly discovered)
- §14 — `field_0x6DF` revised: **DUAL-PURPOSE** flag (C4-plant AND
  Crewed-survivor cooldown share the same byte)
- §15 — vtable[0x160] keystone: **Iron Curtain, NOT Immune** —
  the C4-on-CABHUT bug's true gate lies elsewhere
- §16 — `UnregisterBridgeRepairHut` is a TagClass cleanup, not a hut
  unregister
- §17 — Phase 2 helper reference table
- §18 — Phase-1 open questions: resolved / still open
- §18A — **2026-05-17 Phase 3 extension:** C4 action gate and
  destruction-side per-cell bridge-collapse tree
- §19 — Remaining open questions for Phase 3
- §20 — Next steps

**Primary Addresses (Phase 1):**

| # | Address    | Symbol (current Ghidra)                    | Phase-1 verdict on the name |
|---|------------|--------------------------------------------|------------------------------|
| 1 | `0x519630` | `InfantryClass::PerCellProcess`            | Correct                      |
| 2 | `0x43FB20` | `BuildingClass::Update`                    | Correct                      |
| 3 | `0x57F200` | `MapClass::RepairBridge_Low`               | **Misleading — Low-bridge direction-dispatcher + walker entry; called from engineer-repair path ONLY** |
| 4 | `0x57F440` | `MapClass::RepairBridge_High`              | Misleading — same as #3 for high bridges |
| 5 | `0x575EE0` | `RepairBridgeSegment`                      | **Wrong — fires trigger action 0x1F on segment cells; not a repair function; called from destruction-side endpoint walkers only** |
| 6 | `0x570050` | `ProcessBridgeDestruction_Low`             | **Wrong — engineer-repair entry, NOT destruction** |
|(7)| `0x573540` | `ProcessBridgeDestruction_High`            | **Wrong — same as #6 (only listed via xref check in Phase 1)** |
|(12)| `0x574000`| `MapClass::DestroyBridge_High_MapInit`     | _MapInit suffix is misleading — runtime-called hut-death destruction entry |
|(13)| `0x574C20`| `MapClass::DestroyBridge_Low_MapInit`      | Same as #12 |

**Confidence (overall, Phase 1):** HIGH on call-graph and the three resolved
conflicts; MEDIUM-to-LOW on details that depend on walker bodies (Phase 2).

**Active in YR:** Yes — all six functions are reachable from a normal YR skirmish
(engineer entering a CABHUT cell; C4 placement and detonation on CABHUT). The
`_MapInit` suffix on #12/#13 is a misleading Ghidra label; these functions are
called at runtime (verified by xrefs into `BombClass::Detonate` and
`BuildingClass::Update`'s per-tick body).

---

## 1. Phase 1 verdict — the three pre-flagged conflicts

### Conflict A — `BuildingClass + 0x6DF` semantic

**Resolution:** `field_0x6DF` is a **C4-plant-pending flag**.

- **Set by**: `InfantryClass::PerCellProcess` at `0x519630`, in the **Mission
  0x11 (Sabotage)** branch, when an engineer plants C4 on a building (verified
  in decompilation — see §3.6 below). The same site also writes the timer
  fields `field_0x528` (start frame), `field_0x52C`, `field_0x530` (delay),
  and `field_0x540` (engineer pointer for kill attribution).
- **Read & cleared by**: `BuildingClass::Update` at `0x43FB20`, once
  `(g_CurrentFrameCounter - field_0x528) >= field_0x530` (timer expired). After
  acting, the function writes `this->field_0x6df = 0` and
  `this->field_0x540 = 0` (§3.2 below).
- **NOT touched by**: `BombClass::Detonate` (verified). The demo-truck path
  bypasses `field_0x6DF` entirely and calls `DestroyBridge_*_MapInit`
  directly (§3.7).

**Status of the prior claims:**

| Source                | Prior claim                                          | Phase-1 verdict |
|-----------------------|------------------------------------------------------|------------------|
| BRIDGE_SYSTEM.md      | "repair pending flag (set on engineer enter)"        | **WRONG** — `field_0x6DF` has nothing to do with repair; the bridge-repair path never touches it |
| Plan §"Conflict A" / scoping pass | "self-destruct flag set on hut damage/death" | **PARTIALLY RIGHT** — it is a self-destruct flag, but more specifically a **C4-plant-pending** flag set by the engineer's Mission_Sabotage branch, **not** by a damage handler. |
| MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md | "C4 attached flag" | **RIGHT** for the C4-plant case; this matches the Phase-1 finding exactly |

**Setter completeness (qualifier):** Phase 1 verified the engineer-C4-plant
setter and absence-of-setter in `BombClass::Detonate`. **Phase 2 must still
spot-check** that no other path (e.g., `BuildingClass::ReceiveDamage` at
`0x442230`, building-self-destruct mission state) sets the same byte. The
Phase-1 evidence is consistent with C4-only, but the search was not
exhaustive across all 9849 functions in the binary.

---

### Conflict B — Is `RepairBridge_Low/High` (#3/#4) a direction-agnostic walker driver shared by both paths?

**Resolution:** **NO.** The top-level dispatchers are **separate function
trees**. The engineer-repair path and the hut-destruction path are NOT the
same chain "with different inputs."

**Verified call-graph (Phase 1 xref pass):**

```
ENGINEER REPAIR PATH (engineer steps onto a CABHUT cell):
  InfantryClass::PerCellProcess (0x519630, missions 8/0xB/0x19, Type[0x16B6])
    ├─ Plays EVA + RepairBridgeSound (gated on local human + RulesClass+0x248 != -1)
    ├─ 5×5 scan → decides Low vs High dispatcher
    ├─→ ProcessBridgeDestruction_Low  (0x570050)   ←── MISNAMED. Engineer-repair entry.
    │     ├─ ANOTHER 5×5 scan → if low-bridge overlay [0x4A..0x65] found:
    │     │   └─→ MapClass::RepairBridge_Low (0x57F200)
    │     │         └─→ RepairBridgeWalker_NS_Low / EW_Low  (Phase 2)
    │     └─ Else (no low-bridge overlay): ramp-processing logic
    │         ├─ recursive ProcessBridgeDestruction_Low(moved coord)
    │         ├─ MapClass::ToggleBridgePavement
    │         ├─ MapClass::SetOverlayAndPropagate
    │         ├─ MapClass::ValidateBridgeZones
    │         ├─ MapClass::UpdateBridgeZonesHelper (zones rebuild)
    │         └─ FUN_00569760 (pavement-walker, per LAT_RETRIGGER doc)
    └─→ ProcessBridgeDestruction_High (0x573540) — same shape

HUT DESTRUCTION PATH (C4 timer expires on CABHUT, OR demo-truck on CABHUT):
  BuildingClass::Update (0x43FB20, field_0x6DF + Type[0x16B6] + timer expired)
    ├─ 5×5 scan → decides Low vs High dispatcher
    ├─→ MapClass::DestroyBridge_Low_MapInit  (0x574C20)
    └─→ MapClass::DestroyBridge_High_MapInit (0x574000)
  OR
  BombClass::Detonate (0x438720, target is BuildingClass RTTI=6 AND Type[0x16B6])
    ├─ Apply_area_damage with the bomb warhead
    ├─ Spawn explosion AnimClass
    ├─ 5×5 scan → decides Low vs High dispatcher
    ├─→ MapClass::DestroyBridge_Low_MapInit  (0x574C20)
    └─→ MapClass::DestroyBridge_High_MapInit (0x574000)
```

**Direct xref verification (Phase 1):**

| Function                              | Callers found                                          |
|---------------------------------------|--------------------------------------------------------|
| ProcessBridgeDestruction_Low  (#6)    | `PerCellProcess @0x519cf6`, self×2 (recursive)         |
| ProcessBridgeDestruction_High (#7)    | `PerCellProcess @0x519d12`, self×2 (recursive)         |
| DestroyBridge_High_MapInit    (#12)   | `BombClass::Detonate @0x438982`, `BuildingClass::Update @0x44031b` |
| DestroyBridge_Low_MapInit     (#13)   | `BombClass::Detonate @0x43896a`, `BuildingClass::Update @0x440301` |
| RepairBridge_Low              (#3)    | `ProcessBridgeDestruction_Low @0x5700d6` (ONLY)        |
| RepairBridge_High             (#4)    | `ProcessBridgeDestruction_High @0x5735c8` (ONLY)       |

The convergence (if any) is at the lower level — either the walkers
(Phase 2) or `FUN_00569760` (pavement walker). The dispatcher pairs
**#6/#7 vs #12/#13 are disjoint at the call-graph level**.

**Status of the prior claims:**

| Source                | Prior claim                                          | Phase-1 verdict |
|-----------------------|------------------------------------------------------|------------------|
| BRIDGE_SYSTEM.md      | "FUN_00574C20 / FUN_00574000 are the repair dispatchers" | **WRONG** — they are the **hut-destruction dispatchers**, called from `BombClass::Detonate` and `BuildingClass::Update` (the C4-timer-expired path) |
| Plan §"Conflict B" / scoping pass | "the dispatcher is direction-agnostic; repair vs destruction is decided by current cell state, not by who called" | **PARTIALLY WRONG** — at the **top level**, dispatchers are separate (ProcessBridgeDestruction_* for repair, DestroyBridge_*_MapInit for destruction). The "direction-agnostic" claim may still hold at the **walker level**, but Phase 1 did not decompile the walkers; Phase 2 must verify whether `RepairBridgeWalker_*_*` is used by both directions or only by repair |

**What remains for Phase 2 on this conflict:**
1. Decompile `DestroyBridge_Low_MapInit` (#13) and `_High` (#12) — find where they
   converge with the repair path (if at all).
2. Decompile the four walkers `RepairBridgeWalker_NS_Low/EW_Low/NS_High/EW_High`
   (#8–#11) — determine whether they are called from both sides or only from
   `RepairBridge_Low/High` (and therefore exclusively from the repair path).
3. Decompile `FUN_00569760` (the pavement walker) — confirm its role on both sides.

---

### Conflict C — `RepairBridgeSegment` (#5, `0x575EE0`) semantic

**Resolution:** Both prior claims were partly wrong. The function:

- Does **NOT** "walk 3-wide clearing objects" (BRIDGE_SYSTEM.md's wording).
- Does fire **`TechnoClass::ProcessCellAction(0x1F, 0, DAT_00abd480, 0, 0)`**
  per cell along a segment (gap-scan §D2.5's wording — confirmed).
- The condition for firing on each cell is **`cell + 0x3C != 0`** (i.e., the
  cell has an attached tag/TagClass pointer). Cells without a tag are
  skipped silently. Object clearing is **not** done here.
- It is **NOT called from any repair-side path.** The six callers (Phase-1
  xref result) are all destruction-side:

  | Caller                                        | Address    | Side        |
  |-----------------------------------------------|------------|-------------|
  | `MapClass::FindBridgeEndpoints_EW_Low`        | `0x57c980` | Destruction |
  | `MapClass::FindBridgeEndpoints_NS_Low`        | `0x57caa0` | Destruction |
  | `MapClass::FindBridgeEndpoints_EW_High`       | `0x57dc08` | Destruction |
  | `MapClass::FindBridgeEndpoints_NS_High`       | `0x57dd38` | Destruction |
  | `MapClass::UpdateBridgeEdgeTiles_Low`         | `0x570f00` | Destruction |
  | `MapClass::UpdateBridgeEdgeTiles_High`        | `0x576620` | Destruction |

**The name "Repair" in `RepairBridgeSegment` is wrong**. A better name is
`FireBridgeTriggerActionsOnSegment` — the function's only effect is to fan
out trigger-action `0x1F` to every tagged cell along the destroyed segment.

**Body shape (verified, summarized):**

```
fn FireBridgeTriggerActions(p1: CellCoord, p2: CellCoord) {
    // Normalize p1, p2 so p1 is the smaller endpoint along the
    // varying axis. bVar2 = (p1.y == p2.y) means EW axis, else NS.
    let ew_axis = (p1.y == p2.y);
    let (start, end) = if /* p2 < p1 on the moving axis */ { (p2, p1) } else { (p1, p2) };

    let mut cur = start;
    while cur != end {
        // (a) The cell itself
        let cell = MapClass::Get_CellClass(cur);
        if cell.field_0x3C != 0 { TechnoClass::ProcessCellAction(0x1F, 0, DAT_00abd480, 0, 0); }

        if ew_axis {
            // (b) Three cells along the perpendicular (NS) at this column
            //     using offset table at DAT_0089f698 + g_DirectionOffsets
            for offset in [DAT_0089f698, +DirOff, +DirOff] {
                let c = MapClass::Get_CellClass(cur + offset);
                if c.field_0x3C != 0 { TechnoClass::ProcessCellAction(0x1F, ...); }
            }
            cur.x += 1;
        } else {
            // NS axis: three cells along the perpendicular (EW)
            //         using offset tables DAT_0089f690 / DAT_0089f6a0
            for offset in [DAT_0089f690, DAT_0089f6a0, DAT_0089f6a0_modified] {
                let c = MapClass::Get_CellClass(cur + offset);
                if c.field_0x3C != 0 { TechnoClass::ProcessCellAction(0x1F, ...); }
            }
            cur.y += 1;
        }
    }
}
```

**Tiny details captured during the read (the kind that matter for parity):**

- The loop is `while (cur != end)`, **exclusive** of `end` — the final
  endpoint cell is NOT processed on its own iteration. Whether the endpoint
  is covered by an earlier iteration's perpendicular fan-out depends on the
  segment length and parity; this is a possible off-by-one against the
  cardinality of the trigger fires (parity-relevant). _Phase 2 should verify
  whether `FindBridgeEndpoints` ensures the endpoint is reached via fan-out._
- Per iteration: the segment fires `0x1F` on **4 cells total** (the main
  walker cell + 3 perpendicular cells), not 3. The "3-wide" wording in
  BRIDGE_SYSTEM.md is off by one (it appears the perpendicular fan is 3 cells
  including the main cell's column; depends on whether you count the
  perpendicular row inclusive of cur. Phase 2 should clarify exactly which 3
  the offsets cover.)
- The EW-axis path uses **`DAT_0089f698`** as its perpendicular base offset,
  and the NS-axis path uses **`DAT_0089f690`** and **`DAT_0089f6a0`**. These
  are 4-byte (CellCoord) constants at fixed data addresses — Phase 3 should
  inspect their values to confirm `(+0, -1)`, `(+0, +1)`, etc.
- `DAT_00abd480` is the action-context pointer passed to ProcessCellAction.
  Phase 3: inspect what struct lives there.
- The function operates on `g_CellArray_Base + cell_idx * 4` for cell lookup.
  Out-of-bounds (`idx < 0 || idx > 0x3FFFF`) and null-cell falls back to
  `DAT_00abdc50` (a sentinel/default cell). This is the standard map
  bounds-check pattern — every call site repeats this lookup inline rather
  than using a single helper.

**Status of the prior claims:**

| Source                | Prior claim                                          | Phase-1 verdict |
|-----------------------|------------------------------------------------------|------------------|
| BRIDGE_SYSTEM.md      | "walks 3-wide clearing objects"                      | **WRONG** — no object clearing; only ProcessCellAction(0x1F); fan is 4 cells per step, not 3 |
| gap-scan §D2.5        | "fires ProcessCellAction(0x1F, ...) on cells with non-null TagClass*" | **RIGHT** — confirmed exactly |
| ADDRESS_MAP.md        | "RepairBridgeSegment (walk + repair 3-wide)"         | **NAME WRONG** — function does not repair; called only from destruction-side endpoint walkers |

---

## 2. Class Layout / Key Offsets (Phase 1 — partial)

### BuildingClass (subset relevant to bridge-hut-death)

| Offset    | Type        | Field                                    | Source           |
|-----------|-------------|------------------------------------------|------------------|
| `+0x520`  | TypePtr     | `Type` (BuildingTypeClass*)              | BombClass::Detonate dereference; BuildingClass::Update via `this->Type` |
| `+0x528`  | int         | C4 plant start frame counter             | PerCellProcess sets; Update reads |
| `+0x52C`  | int         | C4 plant aux timestamp (purpose TBD)     | PerCellProcess sets |
| `+0x530`  | int         | C4 plant delay (frames)                  | PerCellProcess sets; Update reads |
| `+0x540`  | TechnoClass*| Engineer who planted C4 (kill attribution) | PerCellProcess sets; Update passes to vtable+0x16c; cleared with field_0x6DF |
| `+0x6df`  | byte (bool) | **C4-plant-pending flag**                | **The flag at the center of Conflict A.** Set by engineer; consumed by Update |

### BuildingTypeClass (subset)

| Offset    | Type   | Field             | Notes |
|-----------|--------|-------------------|-------|
| `+0x16B6` | byte   | `BridgeRepairHut` | Gates both the engineer-repair branch in PerCellProcess (§3.6) AND the hut-destruction branch in BuildingClass::Update (§3.2) and BombClass::Detonate (§3.7) |

### CellClass (subset — fields touched in Phase-1 functions)

| Offset    | Type   | Field / purpose                                    | Where read |
|-----------|--------|----------------------------------------------------|-----------|
| `+0x38`   | int    | IsoTile/TileIndex — bridge ramp tiles are in range `[DAT_00abad1c, DAT_00abad1c + 0x10)` (16 indices) | All three 5×5 scan sites |
| `+0x3C`   | TagClass* | Attached cell tag — non-null triggers ProcessCellAction(0x1F) in #5 | RepairBridgeSegment |
| `+0x44`   | int    | Overlay index. Low-bridge range `[0x4A..0x65]` inclusive (28 indices). High-bridge range `[0xCD..0xE8]` inclusive (28 indices). NS-direction sub-ranges (low): `[0x4A..0x52]`, `[0x5C..0x5F]`, `==0x64` (14 cells total). EW-direction sub-ranges (low): `[0x53..0x5B]`, `[0x60..0x63]`, `==0x65` (14 cells total). High-bridge sub-ranges mirror the same partitioning with offsets in the `0xCD..0xE8` band. | RepairBridge_Low/High, ProcessBridgeDestruction_Low, 5×5 scan sites |
| `+0x11A` | byte   | Ramp transition state byte. Values seen in #6: `5`, `7`, `8`, `12` (`0x05, 0x07, 0x08, 0x0C`) — each gates a different ramp-tile-pattern branch | ProcessBridgeDestruction_Low |
| `+0x11B` | byte   | Ramp transition aux byte. Incremented `+= 4` in #6 across several cells when crossing certain ramp tiles | ProcessBridgeDestruction_Low |
| `+0x140` | uint32 | Cell flags. Bits observed in Phase 1: `0x80`, `0x100`, `0x400`, `0x500` (= 0x100 \| 0x400), `0x800` (some direction-orientation bit, used in #6 to bias `DirectionOffsets[8]` index by ±2) | ProcessBridgeDestruction_Low |
| `+0x24`  | CellCoord  | Adjacent cell coord (when flag 0x80 is clear)   | #6 |
| `+0x2C`  | CellPtr    | Adjacent cell pointer (when flag 0x80 is clear) | #6 |

### InfantryClass (subset — Phase 1)

| Offset (int *)| Byte offset | Type        | Purpose |
|-----|------------|-------------|---------|
| `[0x27]` | `+0x9C` | int    | Location_X (cell-aligned) |
| `[0x28]` | `+0xA0` | int    | Location_Y |
| `[0x29]` | `+0xA4` | int    | Location_Z |
| `[0xD]`  | `+0x34` | TagPtr | Attached tag — fires ProcessCellAction(0x30) on building enter |
| `[0x169]`| `+0x5A4`| Building*| NavTarget (probably "destination building") |
| `[0x1B0]`| `+0x6C0`| Type*  | InfantryTypeClass pointer |

### InfantryTypeClass (subset — bit-flags read in Phase 1)

| Offset    | Purpose (inferred)                                | Read in     |
|-----------|---------------------------------------------------|-------------|
| `+0xEB4`  | Unknown bool A (one of two "is-noisy-on-enter" gates) | PerCellProcess (top of mission-8 branch) |
| `+0xEB5`  | Unknown bool B (companion to +0xEB4)              | PerCellProcess |
| `+0xEC2`  | C4 capability — gates Mission_Sabotage entry      | PerCellProcess |
| `+0xEC3`  | **Engineer flag** — gates "BridgeRepairHut-or-capture branch" vs spy-infiltrate branch (FALSE → spy path). Inferred from the if-statement at LAB_00519b17; should be confirmed against the INI parser | PerCellProcess |
| `+0xEC4`  | Spy/Agent flag (companion to +0xEC3)              | PerCellProcess (spy-infiltrate fallback) |
| `+0xEC6`  | Some prioritization bit for "look up which building" — affects whether `Look_up_building_in_cell` or `CellClass::FindFirstBuilding` is preferred | PerCellProcess (later, outside Phase 1 scope) |

These offsets need explicit confirmation against
`object_type.rs` and/or the binary's INI parser. Phase 1 uses them only to
state the gating logic for the BridgeRepairHut branch.

### Globals touched in Phase 1

| Global             | Type / size | Purpose                                                 |
|--------------------|-------------|---------------------------------------------------------|
| `g_CellArray_Base` | `CellClass**` | Cell pointer array, indexed by `y*0x200 + x`. Map dimensions = 512 × 512 (`0x40000` cells), bounds-check is `idx in [0, 0x3FFFF]`. |
| `DAT_00abdc50`     | CellClass   | Sentinel "default cell" used as fallback on out-of-bounds or null lookups |
| `DAT_00abdc74`     | CellCoord   | Sentinel-cell coord (written by the bounds-check fallback). |
| `DAT_00abad1c`     | int         | Base IsoTile index for low-bridge **ramp tiles**. The 16-tile range `[DAT_00abad1c, +0x10)` is the discriminator in every 5×5 scan in Phase 1 |
| `DAT_00abad30`     | int         | Ramp-tile sub-pattern A discriminator (used in #6, gated on cell+0x11A == 5) |
| `DAT_00abc2b4`     | int         | Ramp-tile sub-pattern B discriminator (gated on cell+0x11A == 8) |
| `DAT_00aa1028`     | int         | Ramp-tile sub-pattern C discriminator (gated on cell+0x11A == 7) |
| `DAT_00aa1130`     | int         | Ramp-tile pattern B alternate (gated on cell+0x11A == 8) |
| `DAT_00aa1548`     | int         | Ramp-tile sub-pattern D discriminator (gated on cell+0x11A == 12) |
| `DAT_00aa0740`     | int         | Ramp-tile pattern D alternate (gated on cell+0x11A == 12) |
| `DAT_0087f8dc`     | int         | Map-bounds X parameter (used in #6 to clip ramp-walking iteration) |
| `DAT_0087f8e0`     | int         | Map-bounds Y parameter (used with `DAT_0087f8dc * 2`) |
| `DAT_0089f690`     | CellCoord (4B) | NS-axis perpendicular offset A (#5, #6) |
| `DAT_0089f698`     | CellCoord   | NS-axis perpendicular offset B / EW-axis base (#5, #6) |
| `DAT_0089f6a0`     | CellCoord   | EW-axis perpendicular offset (#5, #6) |
| `g_DirectionOffsets` | CellCoord[8] | 8-direction step table |
| `DAT_0089f68a`     | int[]       | Y-component of an alternate direction step table (used in #6 ramp branch) |
| `DAT_0089f6dc..0x1c` | CellCoord  | (Plan notes a 16-byte range here; the present functions did not touch it in Phase 1 — keep deferred to Phase 2.) |
| `DAT_00a83dec`     | ptr[]       | "Bridge-repair callback registry" — array of object pointers, vtable+0x28 is invoked on each on bridge repair (#1, after dispatch). Phase 2: identify subscribers. |
| `DAT_00a83df8`     | int         | Count of entries in `DAT_00a83dec` |
| `g_RulesClass_Instance + 0x248` | int | **RepairBridgeSound** sound index. Checked != -1 to gate VocClass playback. Per existing GLOBAL_SOUNDS doc this is sound slot index 0x92 → `BridgeRepaired` from soundmd.ini |
| `g_RulesClass_Instance + 0x1700` / `+0x1708` | double | Damage thresholds (`ConditionRed`/`ConditionYellow`) — referenced in BuildingClass::Update §3.2; not specifically tied to repair |

---

## 3. Per-function findings

### 3.1 InfantryClass::PerCellProcess (0x519630) — engineer-repair entry

This is the on-arrival callback fired when an infantry unit completes a step
into a new cell. Multiple mission codes are handled in series; the
BridgeRepairHut branch lives inside the "capture/enter" outer block
(missions `8`, `0xB`, `0x19`).

**Outer gates (in order, in the BridgeRepairHut path):**

1. **Mission code** ∈ `{8 (Capture), 0xB, 0x19}`. The mission code is read
   via `vtable+0x184` on the engineer (three separate decompiled reads;
   apparently not folded by the compiler — note this is a **micro-detail**
   that matters if you were trying to instrument or hook this site:
   `What_Action` is invoked 3 times in succession).
2. **NavTarget** (`infantry[0x169]`) must not be a `BuildingClass` already in
   mission_capture state — otherwise control falls into the
   garrison/passenger branch at `LAB_00519b17` instead. The full predicate
   is: NavTarget is null, OR `NavTarget.byte[5] & 1 == 0`, OR
   `NavTarget.vtable+0x80() == 0`, OR NavTarget RTTI != 6.
3. **Cell-resident building** is looked up via `Look_up_building_in_cell()`
   and must match either `infantry[0x169]` (NavTarget) or `infantry[0xAD]`
   (some secondary target — possibly the "last building").
4. **InfantryTypeClass+0xEC3** (Engineer flag) must be non-zero. If zero,
   control diverts to `BuildingClass::OnSpyInfiltrate` (the spy path).
5. **Building RTTI** (`vtable+0x2c`) must be `6` (BuildingClass).
6. **Building's `Type[0x16B6]`** (`BridgeRepairHut`) must be non-zero.

**Body of the BridgeRepairHut branch (in execution order):**

```
A. (EVA — local human player only):
   if HouseClass::IsHumanPlayer() {
       puVar = vtable+0x1b8(self)        // get engineer's cell coord
       if CreateRadarEvent(*puVar) {     // creates radar blip; returns whether to play EVA
           VoxClass::PlayEVA(0xFFFFFFFF) // sentinel — Phase 2 needs to verify what 0xFFFFFFFF resolves to in PlayEVA
       }
   }

B. (Sound — global):
   if (g_RulesClass + 0x248) != -1 {    // RepairBridgeSound is set in INI
       local_coord = building.Location;
       VocClass::PlayAt(0)
   }

C. (5×5 scan — outer, deciding low vs high dispatcher):
   for (dy in -2..=+2) {
       for (dx in -2..=+2) {
           tile_idx  = MapClass::Get_CellClass(self.coord + (dx,dy)).field_0x38
           overlay   = MapClass::Get_CellClass(self.coord + (dx,dy)).field_0x44
           if (DAT_00abad1c <= tile_idx < DAT_00abad1c + 0x10)
              || (0x4A <= overlay <= 0x65) {
               low_bridge_found = true;
           }
       }
   }

D. (Dispatch):
   coord = vtable+0x1b8(self)            // engineer's cell coord
   if low_bridge_found {
       ProcessBridgeDestruction_Low(coord)   // (despite the name)
   } else {
       ProcessBridgeDestruction_High(coord)
   }

E. (Bridge-repair callback registry):
   for i in (DAT_00a83df8 - 1) downto 0 {
       cb = DAT_00a83dec[i]
       cb.vtable[0x28](building, 0)
   }
   building.vtable[0x2e0]()              // some post-action call on the hut

F. (Engineer's attached trigger):
   if engineer.field_0x34 != 0 {         // engineer has an attached tag
       TechnoClass::ProcessCellAction(0x30, engineer, DAT_00a8f1e0, 0, 0)
   }

G. (Engineer disposal):
   engineer.vtable[0xF8]()               // Limbo/Destroy — engineer is consumed
```

**Tiny details to capture:**

- The 5×5 scan loop is **inclusive** `[-2..=+2]` on both axes — 25 cells
  total. Verified by the de-compiled loop bounds (`iVar3 = -2; while (iVar3 < 3)`).
- The discriminator OR is short-circuited by C semantics; if a low-bridge
  tile-index match is found early, the overlay check is skipped for that
  cell. This is a parity-relevant ordering detail: scan order matters if
  a cell has BOTH a non-low tile-index AND a low overlay (unlikely but
  technically possible at the boundary between bridge segments).
- `local_3c` low-byte is set to `1` to mark "low bridge found." The
  decompiled output uses `param_2._byte[0]` for this signal — be aware
  the Rust port should not name a struct field `param_2`.
- The function is called via vtable slot `+0x18C` on InfantryClass — i.e.,
  it is virtual-dispatched, but the slot is `Infantry`-specific (other
  unit classes get different `PerCellProcess`).
- Note that the EVA + sound fire **before** the dispatcher is called, and
  **before** any cell state mutation. The order is fixed; a parity port
  must emit the sound event before the bridge-state event.
- The "post-dispatch callback registry" at step E is `vtable+0x28` on each
  registered object. Phase 2 should identify what kind of object subscribes
  (likely HouseClass instances for the "bridge has been repaired" event).
  Until then, it is a `BridgeRepairListener` registry of unknown shape.

**Engineer disposal — semantic check.**
`vtable+0xF8` is the `Object::Limbo` slot in TS/YR conventions. Engineer
entering a CABHUT is **consumed** (despawned). Vanilla YR behavior. Confirmed.

---

### 3.2 BuildingClass::Update (0x43FB20) — hut-destruction entry (C4 timer expired)

This is the per-tick building update function. The bridge-destruction
branch is one of many in the body; it lives near the end and is gated on
`this->field_0x6df != 0`. Phase 1 only annotates the bridge-relevant tail
of this function.

**Bridge-destruction gate:**

```
if (this->field_0x6df != 0) {
    delay_frames = this->field_0x530;
    if (this->field_0x528 != -1) {
        elapsed = g_CurrentFrameCounter - this->field_0x528;
        if (elapsed < delay_frames) {
            delay_frames -= elapsed;
            // timer still ticking — fall through, no bridge action yet
        }
    }
    if (delay_frames == 0 /* timer expired */) {
        if (this->Type[0x16B6] == 0) {
            // NOT a BridgeRepairHut — normal C4 detonation: damage self.
            // NOTE: this branch does NOT clear field_0x6df / field_0x540.
            // Flag persistence is implicit-via-building-death (the
            // ReceiveDamage call typically lethals the building, so the
            // flag never matters again). Verified via decompile_function
            // 0x43FB20 — the clearers in the else-branch (at 0x440320 for
            // field_0x6df and 0x440327 for field_0x540) are not reached on
            // this branch. (corrected 2026-07-18: was "jumps from vtable[0x16C]
            // straight to the function epilogue (LAB_00440378)" — disassembly
            // shows NO jump here; execution falls through linearly from the
            // vtable+0x16C CALL at 0x440358 into the SHARED tail at 0x44035E
            // (`this->field_0x90` check + conditional `vtable[0x124](2)` call),
            // which both the hut and non-hut branches execute identically
            // before reaching LAB_00440378. Verified via
            // disassemble_function(0x43FB20) — MISLEADING, root cause
            // INFERENCE_HARDENED (control-flow summary written as fact without
            // confirming there was no shared fallthrough tail).
            saved_health = this->Health;
            this->vtable[0x16C](&saved_health, 0, RulesClass+0xFA8 /*C4Warhead*/,
                                this->field_0x540 /*engineer*/, 1, 0, 0);
        } else {
            // IS a BridgeRepairHut — destroy the bridge instead
            (5×5 scan, same shape as PerCellProcess §3.1 step C)
            if (low_bridge_found) {
                building_coord = this->vtable[0x1B8](self)
                MapClass::DestroyBridge_Low_MapInit(building_coord)
            } else {
                building_coord = this->vtable[0x1B8](self)
                MapClass::DestroyBridge_High_MapInit(building_coord)
            }
            // Clearer fires ONLY on the BridgeRepairHut branch. Binary
            // places these two writes immediately after the CALL to
            // DestroyBridge_*_OnHutDeath: field_0x6df at 0x440320
            // (`C6 86 DF 06 00 00 00`) and field_0x540 at 0x440327
            // (`C7 86 40 05 00 00 00 00 00 00`), both inside the else-block.
            // Verified via read_memory(0x440320, 18) =
            // `c6 86 df 06 00 00 00 c7 86 40 05 00 00 00 00 00 00 eb`.
            this->field_0x6df = 0;
            this->field_0x540 = 0;
        }
    }
}
```

**Tiny details to capture:**

- The same `vtable[0x1B8]` is called **three times** to read the building's
  coord (once for the inner-loop scan, once for the outer scan, once for the
  final dispatch). The compiler did not fold these. This is irrelevant to
  parity (return value is the same) but matches the BombClass::Detonate
  shape — i.e., this is hand-duplicated code, not a refactored helper.
- **Critical observation for the C4-on-CABHUT bug** (project memory entry
  `project_c4_bridge_hut_followup`): when the gate fires for a BridgeRepairHut,
  the `vtable[0x16C]` damage call is **skipped**. The hut **does not take
  the C4 damage** — only the bridge dies. The hut survives the C4 explosion.
  This is consistent with `Immune=yes` on CABHUT (the hut would refuse the
  damage anyway), but the binary explicitly chooses the bridge-destruction
  branch **before** attempting the damage call, so the Immune flag is not
  what's gating the C4 effect — it's the BridgeRepairHut flag itself.
- `RulesClass+0xFA8` is the **C4Warhead** reference (verified by adjacent
  fields and the call signature). `RulesClass+0xFC8` is a different warhead
  (used in `BombClass::Detonate` §3.7).
- `field_0x540` is passed as the **damage source** so that area-damage
  attribution credits the engineer that planted the C4 (kill-credit). On the
  bridge path, this field is cleared but not used — the bridge "death" has
  no kill-credit recipient.
- The timer check at `field_0x528 == -1` is a "timer not started" sentinel.
  If `-1`, the function falls through to the `delay_frames == 0` test,
  which can only be true if `field_0x530 == 0` — i.e., a zero-delay flag is
  treated as "fire immediately." Otherwise the branch is skipped this tick.
- After firing, both `field_0x6df` and `field_0x540` are cleared. **The
  timer fields `field_0x528`, `field_0x530` are NOT cleared explicitly**
  in this branch — Phase 2 should verify whether they get cleared
  downstream or are left stale (only matters if a second C4 can ever be
  planted on the same building, which would set them again).

---

### 3.3 MapClass::RepairBridge_Low (0x57F200) — direction-detect + walker dispatcher (low bridge)

Despite the name, this is **not** a "repair" function — it is a thin
direction-resolver that:
1. Reads the cell's overlay byte at `cell + 0x44`.
2. Decides whether this cell is NS-oriented or EW-oriented based on the
   sub-range the overlay falls into.
3. Walks **backward by 0 / 1 / 2 cells** along the axis to find a canonical
   "anchor" coordinate.
4. Calls the matching walker (`RepairBridgeWalker_NS_Low` or `RepairBridgeWalker_EW_Low`).

**Sub-range partitioning (low bridge):**

| Overlay range / value | Direction      |
|-----------------------|----------------|
| `[0x4A..0x52]`        | NS             |
| `[0x5C..0x5F]`        | NS             |
| `== 0x64` (100)       | NS             |
| `[0x53..0x5B]`        | EW             |
| `[0x60..0x63]`        | EW             |
| `== 0x65`             | EW             |
| Else                  | Falls through, returns without dispatching |

Total NS cells: 9 + 4 + 1 = 14. Total EW cells: 9 + 4 + 1 = 14. Sum = 28,
which matches the low-bridge overlay band `[0x4A..0x65]`.

**Anchor selection (NS direction, mirrored for EW with `x` instead of `y`):**

```
let cell_n1 = MapClass::Get_CellClass(coord with y-1)
if !is_low_bridge_overlay(cell_n1.field_0x44) {
    // current cell is at the NORTH edge of the segment
    RepairBridgeWalker_NS_Low(coord with y+1)   // start walker one south
} else {
    let cell_n2 = MapClass::Get_CellClass(coord with y-2)
    if is_low_bridge_overlay(cell_n2.field_0x44) {
        // we are >= 2 cells inside from the north edge
        RepairBridgeWalker_NS_Low(coord with y-1)  // start walker one north
    } else {
        // we are exactly 1 cell south of the north edge
        RepairBridgeWalker_NS_Low(coord as-is)
    }
}
```

The "is_low_bridge_overlay" check uses `[0x4A..0x65]` (no NS/EW split — any
low-bridge overlay).

**Tiny details to capture:**

- The function dereferences `*(int *)(puVar3 + 0x44)`. On out-of-bounds or
  null cell, `puVar3` is set to `DAT_00abdc50` (sentinel cell). Reading
  `+0x44` on the sentinel yields whatever was last written there by
  the bounds-check write to `DAT_00abdc74` — i.e., the sentinel cell can
  be polluted between calls. This is unlikely to matter for parity (the
  sentinel's overlay is typically 0, falling outside both sub-ranges),
  but it is a binary-quirk worth knowing about.
- The decompilation uses `CONCAT22(p[1] + (-/+), p[0])` and
  `CONCAT22(p[1], p[0] + (-/+))` to construct the neighbor cell coord
  via 16-bit halves — i.e., (x, y) is packed into a 32-bit value with
  x in the low 16 bits and y in the high 16 bits. This matches what
  `MapClass::Get_CellClass` and `g_CellArray_Base` expect.
- After the walker is called, the function **returns** — there is no
  loop here. The walker itself handles span traversal.
- The function takes `param_1` as `short *` (pointer to two `short`s for
  x, y) — i.e., it accepts a cell-coord pointer, not a value. The decompiler
  spills/loads `psVar1 = param_1` at function entry because `param_1` is
  later overwritten by the CONCAT22 patches.

---

### 3.4 MapClass::RepairBridge_High (0x57F440) — direction-detect + walker dispatcher (high bridge)

**Structurally identical to #3** but with the high-bridge overlay band:

| Overlay range / value | Direction |
|-----------------------|-----------|
| `[0xCD..0xD5]`        | NS        |
| `[0xDF..0xE2]`        | NS        |
| `== 0xE7`             | NS        |
| `[0xD6..0xDE]`        | EW        |
| `[0xE3..0xE6]`        | EW        |
| `== 0xE8`             | EW        |
| Else                  | Returns   |

NS: 9 + 4 + 1 = 14 cells. EW: 9 + 4 + 1 = 14 cells. Sum = 28 cells in the
high-bridge band `[0xCD..0xE8]`.

The two functions are compiled twins — same shape, same offsets, only the
sub-range constants and walker function names differ. They are textbook
copy-paste with constant substitution; not call-into-helper.

(One small caveat: this means a parity port can implement them as a single
generic function parameterized by the band; the binary doesn't, so cycle
counts are technically different, but that's not a parity-observable issue
in a deterministic sim — the same input yields the same dispatch.)

---

### 3.5 RepairBridgeSegment (0x575EE0) — trigger-fire walker, destruction-side only

Already detailed in §1 Conflict C. The function is **misnamed**; it does not
repair. It fires `TechnoClass::ProcessCellAction(0x1F, 0, DAT_00abd480, 0, 0)`
per tagged cell along a segment. Only the destruction-side functions call it.

**Additional Phase-1 detail worth capturing:**

- The axis-decision (`bVar2 = (p1.y == p2.y)`) implies the function only
  handles **axis-aligned** segments (NS or EW). Diagonal segments would
  not work — but bridges in YR are always axis-aligned, so this is fine.
- The endpoint normalization (swap p1/p2 so p1 < p2 along the varying axis)
  ensures the walker always goes "forward." This is a clean parity-port
  point: choose smaller endpoint first.
- ProcessCellAction code `0x1F` (= 31 dec) is the trigger-action ID. In
  TS/RA2 conventions, action 0x1F is typically "Force Trigger" or
  "Destroy Trigger" — Phase 2 should pin this down.
- The `DAT_00abd480` context pointer is shared across all callers — Phase 3
  should inspect its content to know what kind of state the action receives.

---

### 3.6 ProcessBridgeDestruction_Low (0x570050) — engineer-repair entry, MISNAMED

This is the function `PerCellProcess` calls when it has decided
"engineer entered a CABHUT cell, and the 5×5 scan found a low-bridge cell."
The "Destruction" in the name is **wrong** — this function processes the
engineer-trigger action, which is a REPAIR action when the bridge state
permits.

**Body in two phases:**

**Phase A: "Find a low-bridge overlay cell within the engineer's 5×5 neighborhood,
and dispatch to the walker."** This is **a SECOND 5×5 scan**, on top of the one
PerCellProcess already did. The two scans are NOT redundant: the outer scan checks
**either** tile-index `[DAT_00abad1c, +0x10)` OR overlay `[0x4A..0x65]`; this
inner scan checks **only** overlay `[0x4A..0x65]`. If the outer scan matched on
tile-index but not on overlay, Phase A here will find no match → falls through
to Phase B (ramp handling).

```
for (dy in -2..=+2) {
    for (dx in -2..=+2) {
        cell = MapClass::Get_CellClass(engineer.coord + (dx,dy))
        if 0x4A <= cell.field_0x44 <= 0x65 {
            MapClass::RepairBridge_Low(engineer.coord + (dx,dy))   // anchor coord = found cell
            return;
        }
    }
}
```

**Phase B: "Ramp processing — no overlay cell found within 5×5; the engineer is
adjacent to a ramp tile."** This is the multi-branch ramp transition logic.

The decompilation distinguishes 4 ramp sub-patterns based on
`(cell.field_0x38 - DAT_00abad1c) + 1` and `cell.field_0x11A`:

| Tile-index pattern (relative to DAT_00abad1c)           | `cell.field_0x11A` | Branch action |
|---------------------------------------------------------|--------------------|----------------|
| `== DAT_00abc2b4` or `== DAT_00aa1130`                  | `0x08`             | `ToggleBridgePavement(coord, 0, 0)`; then `FUN_00569760(cell, 2, &screen_rect)`; `TacticalClass::DirtyScreenRect` |
| `∈ {DAT_00abad30 .. DAT_00abad30+4}` (5 indices)        | `0x05`             | Same kind of "tile-set + recurse + ToggleBridgePavement" — see below |
| `∈ {DAT_00aa1028 .. DAT_00aa1028+4}` (5 indices)        | `0x07`             | Same shape, different DAT base |
| `== DAT_00aa1548` or `== DAT_00aa0740`                  | `0x0C`             | `ToggleBridgePavement(coord, 0, 0)`; then `FUN_00569760(cell, 4, &screen_rect)`; DirtyScreenRect |
| Anything else                                           | (any)              | Walk one cell along DirectionOffsets[8 negated by 0x800 flag] and retry; bounded by map clip |

For the **`+4` sub-cases** (`DAT_00abad30 + 4` or `DAT_00aa1028 + 4`) the
function does extra setup before the recurse:
- `MapClass::SetOverlayAndPropagate(coord, DAT_*X* - 1 + DAT_00abad1c, -1, -1, 0)`
- Increments **`cell.field_0x11B += 4`** on the current cell AND **two
  neighbor cells** (offsets `+DAT_0089f690`, `+DAT_0089f6a0`, with EW/NS
  flipping depending on which ramp pattern matched).
- Calls `MapClass::ValidateBridgeZones(coord)` → returns a bool that
  gates whether `MapClass::UpdateBridgeZonesHelper()` runs at the bottom.

After ramp-pattern handling, the function:
- Recursively calls `ProcessBridgeDestruction_Low(coord moved by ±2 cells)`
  along the ramp axis.
- Calls `FUN_00569760(cell, dir_code, &screen_rect)` (the pavement walker
  per LAT_RETRIGGER doc).
- Calls `TacticalClass::DirtyScreenRect(...)` to mark the on-screen region
  for redraw.

Finally, **if `MapClass::ValidateBridgeZones` returned true**, it calls
`MapClass::UpdateBridgeZonesHelper()` — i.e., the bridge zone rebuild is
fired once per repair operation when a zone topology change is detected.
**This is the binary's analog of the Rust `zones_dirty` flag.**

**Tiny details to capture (high-parity-impact):**

- The function does its OWN 5×5 scan even after PerCellProcess already did
  one. The two scans use different match conditions — the inner one is
  stricter (overlay only, no tile-index). A Rust port that conflates them
  will get the ramp-handling path wrong.
- The recursive call passes `(coord.x - 2, coord.y)` in one branch and
  `(coord.x, coord.y - 2)` in another — a **2-cell jump**, not 1, along
  the ramp axis. This is because ramps span 2 cells perpendicular to the
  bridge.
- The constant `+4` increment on `cell.field_0x11B` happens on **3 cells**
  for each `+4` ramp case (the current cell, plus two perpendicular
  neighbors via `DAT_0089f690` and `DAT_0089f698`/`DAT_0089f6a0`). This
  is parity-critical: state byte at `+0x11B` accumulates across consecutive
  ramp repairs. If the Rust port writes to only one cell, the next ramp
  operation sees a wrong state.
- Cell-flag bit `0x500` (= `0x100 | 0x400`) is used as an early-exit on the
  initial "find a starting bridge cell" walk: if a cell has either flag
  set, treat it as "found." Bit `0x100` is the bridge-cell layer flag,
  `0x400` looks like the bridgehead/anchor marker.
- The flag bit `0x800` on the first non-null cell biases the
  `DirectionOffsets` index by ±2: `uVar9 = -(uint)((flags & 0x800) != 0) & 2`.
  In effect, the search "direction" flips by 180° when bit `0x800` is set.
  Phase 2 should identify what `0x800` semantically represents (likely an
  orientation/winding bit on the bridge cell).
- The function uses a stack-resident dynamic-vector at `local_18..local_4`
  to **accumulate cells** during a ramp-repair operation (via
  `*(local_14 + local_8 * 4) = local_48`, where `local_8` is a count).
  After the loop, if `local_8 > 0`, it calls `FUN_00586990(&local_18)` —
  a "process pending cells" dispatch. Phase 2 should decompile
  `FUN_00586990` to know what is done with the accumulated cells.
- `MapClass::ValidateBridgeZones` runs `+ propagate` semantics, then
  returns a bool. `MapClass::UpdateBridgeZonesHelper` is invoked
  unconditionally on **success** — i.e., a topology change DID occur.
  This is the Rust `zones_dirty` analog (see §6 below).

---

### 3.7 BombClass::Detonate (0x438720) — hut-destruction entry (demo truck) [Phase 1 spot-check]

Although `BombClass::Detonate` is plan #14 (Phase 2), Phase 1 includes it
because it is the **other half of the destruction call-graph** — without
checking it, the Conflict-B resolution would be incomplete.

**Body summary:**

```
if (this->Target != 0 && !this->AlreadyExploded) {
    this->Target.field_0x38 = 0;
    if (this->Target.field_0x81 == 0) {  // safety/cancel check
        this->Target.field_0x68 = 0;
        this->AlreadyExploded = true;

        coord = this->Target.Location
        Apply_area_damage(this->Damage, RulesClass+0xFC8 /*BombWarhead*/, 1, 0)

        // Spawn explosion animation
        warhead = cell_at(coord).field_0xEC
        sub_anim = FUN_0048ace0(coord)
        anim_type = Warhead::SelectExplosionAnim(warhead, &coord)
        AnimClass::Constructor(anim_type, &coord, 0, 1, 0x2600, sub_anim, 0)

        // Bridge-collapse branch — gated by target being a BridgeRepairHut
        if (this->Target != null
            && this->Target.vtable[0x2C]() == 6   // RTTI == BuildingClass
            && this->Target.Type[0x16B6] != 0) {  // BridgeRepairHut

            (5×5 scan identical to BuildingClass::Update §3.2 and PerCellProcess §3.1)

            if (low_bridge_found) {
                MapClass::DestroyBridge_Low_MapInit(this->Target.coord)
            } else {
                MapClass::DestroyBridge_High_MapInit(this->Target.coord)
            }
        }
    }
    this->Damage = 0;
    this->Target = 0;
    this->Owner  = 0;   // field_0x28
    VocHandle::Stop();
    this->field_0x54 = 0;
}
```

**Key Phase-1 facts from this function:**

- `BombClass::Detonate` **does not touch `field_0x6DF`** on the target.
  This is the conclusive evidence that the C4 flag is exclusively for the
  **planted-C4** delayed path (engineer plant via PerCellProcess), not for
  generic bomb detonation. Demo trucks fire immediately on arrival via
  this function, and bypass the timer machinery entirely.
- The 5×5 scan and dispatch pattern is **byte-for-byte equivalent** to
  the one in `BuildingClass::Update` — same loop bounds, same `+0x38`
  vs `+0x44` tests, same low/high decision. Hand-duplicated code.
- The warhead used for damage is `RulesClass + 0xFC8` (BombWarhead), which
  is different from `RulesClass + 0xFA8` (C4Warhead used by
  `BuildingClass::Update`). Parity-relevant: demo-truck damage vs C4 damage
  apply different warheads to nearby targets.
- The damage application happens **before** the bridge-collapse dispatch.
  Sequence: damage → explosion anim → 5×5 scan → bridge dispatch. A parity
  port must preserve this order.
- `this->field_0x81` (on the target) is a "cancel detonation" flag — if
  non-zero, the bomb runs the else branch (silent fizzle with only state
  cleanup). This is the "defuse" / "vehicle destroyed mid-flight" guard.
- After firing, the bomb clears Damage, Target, Owner, and VocHandle, then
  zeroes `field_0x54`. The bomb is then in a "consumed" state.

---

## 4. INI Keys (Phase-1 relevant)

| Key                          | Section          | Default      | Effect (verified)                                                | Read at binary address (Phase 1) |
|------------------------------|------------------|--------------|------------------------------------------------------------------|------------------------------------|
| `BridgeRepairHut=yes`        | `[CABHUT]`       | yes (CABHUT) | Sets `BuildingTypeClass + 0x16B6`. Gates the bridge branch in PerCellProcess (§3.1), BuildingClass::Update (§3.2), and BombClass::Detonate (§3.7) | `0x519bc0`, `0x440240`, `0x4389A0` |
| `RepairBridgeSound=BridgeRepaired` | `[AudioVisual]` | `BridgeRepaired` | Sets `RulesClass + 0x248`. Gates the sound playback in PerCellProcess (§3.1, step B); if `-1`, no sound | `0x519BD5` |
| `Immune=yes`                 | `[CABHUT]`       | yes (CABHUT) | Sets a bit on TechnoTypeClass (offset not extracted in Phase 1). **Note for the C4 bug:** in `PerCellProcess`'s Mission_Sabotage path, `vtable[0x160]()` (which returns Immune-like state) is checked **before** C4 placement. If non-zero, C4 placement is rejected and the engineer walks away without setting `field_0x6DF`. This is the root cause of "SEAL/Tanya C4 on CABHUT does nothing" — the engineer can't *plant* C4 on an Immune building. **Phase 2 should pin down exactly which TechnoTypeClass field `vtable[0x160]` reads.** | `0x51A580` area (mission-0x11 branch) |
| `DestroyableBridges=yes`     | `[CombatDamage]` | yes          | Suspected master gate; Phase 1 did NOT find an explicit check in #1–#6 — needs Phase 2 to find the gate | (deferred) |
| `BridgeStrength=1500`        | `[CombatDamage]` | 1500         | Bridge tile HP. Does NOT enter Phase 1 functions (relevant only on the damage-side state machine — already covered in HIGH_BRIDGE_DAMAGE_STATE_MACHINE) | n/a here |
| `BridgeExplosions=`          | `[General]`      | 4-anim list  | Not read in Phase 1 functions (used downstream in `BlowUpBridge`, already in BRIDGE_DEFERRED_MECHANICS doc) | n/a here |

**No new INI keys discovered in Phase 1.** All audio/visual hooks reuse
already-parsed keys.

---

## 5. Integration Points (Phase-1 scope)

### Tick-time invocation
Both top-level entry functions run **inside the per-tick update**:
- `InfantryClass::PerCellProcess` is fired by the infantry locomotor when
  it completes a step into a new cell. Frequency: per infantry per move
  step (so for a fast engineer, multiple times per tick during a path).
- `BuildingClass::Update` is fired by the world's building update pass,
  once per building per tick.
- `BombClass::Detonate` is fired by the bomb's own update when its timer
  expires (timer machinery distinct from the C4-plant-on-building timer).

### Convergence
- **Engineer path** converges into the walker family (`RepairBridgeWalker_*`)
  via `RepairBridge_Low/High`. The walker bodies are **Phase 2** work — until
  they are decompiled, "what state the walker writes" remains an open
  question (see §7 below).
- **Hut-destruction path** enters via `DestroyBridge_*_MapInit`; the bodies of
  these two functions are **also Phase 2** work. Phase 1 only verified the
  call-graph entry, not the body.

### Bridge zones rebuild
- `MapClass::UpdateBridgeZonesHelper` is invoked from `ProcessBridgeDestruction_Low`
  (§3.6) only on the **ramp-handling sub-branches** and only when
  `MapClass::ValidateBridgeZones` returns true. This is the Rust
  `zones_dirty` analog: a topology change is detected → zones are
  rebuilt. Phase 1 did NOT inspect what happens inside `UpdateBridgeZonesHelper`
  (already covered by MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md).
- Whether the destruction-side dispatchers `DestroyBridge_*_MapInit` also
  fire `UpdateBridgeZonesHelper` is **Phase 2** work — likely yes, given the
  Rust path requires zones to be rebuilt on both directions.

### Audio / EVA
- EVA_BridgeRepaired plays **only when local player is human** and only
  for the engineer's owning house (gated by `CreateRadarEvent`).
- `RepairBridgeSound` (RulesClass+0x248) plays at the **building's location**
  via `VocClass::PlayAt`. Spatial. Played for everyone in earshot.
- Audio fires **before** the bridge state mutation. Order: EVA → sound →
  scan → walker. A parity-port must match this.

### Listener callback registry
- `DAT_00a83dec[0 .. DAT_00a83df8)` is a "bridge-repair listener" registry,
  iterated **after** the dispatch in PerCellProcess (§3.1, step E). Phase 1
  did not identify the subscribers. Phase 2 should find writers to this
  array (likely in HouseClass init or a similar bridge-aware subsystem).

---

## 6. Current Rust Implementation Status (vs Phase-1 findings)

Pre-existing scan (from parallel agent C) already established that almost
none of the repair-side wiring exists in the Rust codebase. Phase-1 findings
sharpen the spec:

| Subsystem (matched against Phase-1 finding)        | Status        | File(s) / notes |
|-----------------------------------------------------|---------------|------------------|
| `BridgeRepairHut` flag parse (`Type[0x16B6]`)       | Parsed-only   | `object_type.rs:924` — bool, zero consumers |
| `RepairBridgeSound` parse (`RulesClass+0x248`)      | Parsed-only   | `ruleset.rs:736-746` — `BridgeRules.repair_sound: Option<String>`, never emitted |
| `field_0x6DF` C4-plant-pending flag                 | **Not started** | No analogous field on Rust building entities. The C4-on-CABHUT path requires this flag to be plumbed: engineer plants C4 → flag set + timer scheduled → on timer expiry, the building's tick handler reads the flag and either damages self OR destroys bridge based on `BridgeRepairHut`. None of this is implemented. |
| 5×5 scan around CABHUT                              | **Not started** | No per-tick building-neighborhood scan in Rust. |
| Engineer-on-CABHUT trigger (mission 8, Type[0x16B6]) | **Not started** | `world_orders.rs:147-209` is generic capture — has no BridgeRepairHut branch. |
| Reverse bridge state transition (Destroyed → Healthy) | **Not started** | `bridge_state/mod.rs:756-810` is forward-only. |
| `RepairBridgeSound` SimSoundEvent emission          | **Not started** | No `SimSoundEvent::BridgeRepaired` variant exists. |
| `EVA_BridgeRepaired` dispatch                       | **Not started** | No EVA enum entry for this event. |
| Bridge zone rebuild trigger on repair                | **Ready-to-wire** | `bridge_orchestrator.rs:309-324` already supports the `zones_dirty → refresh_bridge_zones_if_dirty` rebuild — the binary fires it conditionally via `ValidateBridgeZones`/`UpdateBridgeZonesHelper`. The Rust trigger can stay as-is; only the **call site** that flips the flag on repair is missing. |
| Overlay grid mutation infrastructure                | **Ready-to-wire** | `overlay_grid.rs:85-172` supports `clear_overlay`/`place_overlay`/`set_overlay_data`. No repair-side caller yet. |
| `field_0x540` (engineer kill-credit attribution)    | Not applicable for parity-port — Rust attribution can use the entity ID directly. |
| Hut registry at `MapClass+0x1160` (DAT_008B41A8)    | **Not started** | Already documented in MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md. Phase-1 did NOT touch this; Phase 2 needs to decompile `UnregisterBridgeRepairHut` (#15). |
| Listener callback registry `DAT_00a83dec`           | **Not started** | Subscribers unknown — Phase 2. |

**Coverage scorecard (refined post-Phase-1):** 2/12 parsed-only,
2/12 ready-to-wire, **8/12 not started.**

---

## 7. Open Questions (resolution status)

1. **What does each `RepairBridgeWalker_*_*` actually write per cell?**
   **RESOLVED 2026-05-18.** Walkers write **only `+0x44` (OverlayTypeIndex)**
   on a 3-wide perpendicular strip per iteration via a 4-case jump table
   indexed by per-walker byte LUTs. They do NOT write `+0x11E`, `+0x11A`,
   `+0x11B`, or `+0x140`. The damage→intact transition on `+0x11E` does NOT
   happen at all on engineer repair — gamemd leaves the damage byte stale.
   See [REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md](REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md)
   + [REPAIRBRIDGEWALKER_FIELD_11E_FOLLOWUP.md](REPAIRBRIDGEWALKER_FIELD_11E_FOLLOWUP.md).

2. **Are the walkers shared between repair and destruction?** **RESOLVED.**
   No. The four repair walkers (`NS_Low @ 0x57F6A0`, `EW_Low @ 0x57FBC0`,
   `NS_High @ 0x5800D0`, `EW_High @ 0x580600`) are each called by exactly
   one dispatcher (`RepairBridge_{Low,High}`), with no caller from the
   destruction-side `DestroyBridge_*_OnHutDeath`. The destruction path uses
   a different family (`UpdateRamp_*_High` per HIGH_BRIDGE_DAMAGE §11.1).

3. **What does `DestroyBridge_*_MapInit` actually do?** **RESOLVED.**
   `_MapInit` suffix confirmed misleading; renamed to `_OnHutDeath` in
   Ghidra. Both are structural twins running a 5×5 inner overlay scan →
   8-direction fallback flag walk → anchor resolution → forward ramp walk;
   per-cell mutation is delegated to `ApplyDamageToCell @ 0x587180`. See
   [DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md](../01-assets-map-load-overlay/DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md).

4. **`InfantryTypeClass + 0xEC3`** is the Engineer-vs-Spy gate in the
   bridge-repair branch. Phase 1 inferred this from control flow; Phase 3
   must confirm against the INI parser (find a function that reads
   `Engineer=` from `[ENGINEER]`). **STILL OPEN.** (Side note:
   C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION §2 verifies `+0xec3 ← Engineer=`
   at `InfantryTypeClass::ReadINI @ 0x524584`. This question is effectively
   closed; mark resolved on next sweep.)

5. **`VoxClass::PlayEVA(0xFFFFFFFF)`** — sentinel meaning. **RESOLVED 2026-05-18.**
   The notation was misleading shorthand. The actual call is
   `PlayEVA("EVA_BridgeRepaired", -1, -1)` (3-arg fastcall). The two `-1`s
   are per-arg "use the EVA INI entry's default priority and voice slot"
   sentinels, resolved in `VoxClass::QueueVoice @ 0x00752480`. Not a
   "use last EVA" or "use radar-event EVA" reference. See
   [VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md](../../VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md).

6. **`DAT_00a83dec` listener registry subscribers.** **RESOLVED 2026-05-18 —
   not a listener registry.** `DAT_00A83DEC` is the `data*` field (+0x04)
   of the global `DynamicVectorClass<InfantryClass*>` at `0x00A83DE8` — the
   `g_InfantryClass_Array` pool. The "callback dispatch" in PerCellProcess
   was a virtual-dispatch broadcast (`each_infantry->vtable[+0x28]`) over
   the live-infantry pool, not a callback registry. No subscribers exist.
   Already correctly identified in `GI_GHIDRA_REPORT.md §3.8`. See
   [DAT_00A83DEC_LISTENER_REGISTRY_GHIDRA_REPORT.md](../../DAT_00A83DEC_LISTENER_REGISTRY_GHIDRA_REPORT.md).

7. **`MapClass::ValidateBridgeZones` and `MapClass::UpdateBridgeZonesHelper`**
   were not re-decompiled in Phase 1 (already covered in
   MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md). Phase 2 should
   verify those reports' findings still match how the repair-side calls
   them (Conflict-aware confidence). **STILL OPEN.**

8. **Trigger-action code `0x1F` (#5) and `0x30` (#1, step F).** **RESOLVED 2026-05-18.**
   Both are `TriggerEvent` enum IDs, not internal switch cases. `0x1F`
   (`BridgeDestroyed`) is fired by `RepairBridgeSegment @ 0x575EE0` from
   destruction-side endpoint walkers; `0x30` (`BridgeRepaired`) is fired
   by `PerCellProcess` engineer-success branches. The "dispatcher" is
   actually `TechnoClass::FireTriggerAction @ 0x006E53A0` — a per-techno
   trigger-event broadcaster, not an action-code switch. Per-occupant
   effect in vanilla skirmish: **none** (no map triggers bind these events).
   See [TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md](../../TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md).

9. **`FUN_00569760`** (pavement walker per LAT_RETRIGGER doc) and
   `FUN_00586990` (cell-list dispatch in #6) — bodies unread in Phase 1.
   **RESOLVED 2026-05-18.** `FUN_00569760` renamed to
   `MapClass__BridgePavementSpanWalker` — linear ≤30-step span walk along
   `g_DirectionOffsets[dir & 7]`. `FUN_00586990` renamed to
   `MapClass__RecalcCellsAndRebuildZones` — two-pass coord-list iterator
   that clears zone_speed_cache + calls RecalcAttributes, then rebuilds
   zone graph via `FUN_00584550`. **Correction to this doc:** `ToggleBridgePavement`
   does NOT call the pavement walker; the arrow is reversed (pavement
   walker calls ToggleBridgePavement on recognised endpoints). See
   [BRIDGE_PAVEMENT_WALKER_AND_CELLLIST_DISPATCH_GHIDRA_REPORT.md](../02-cell-state-layering-zones/BRIDGE_PAVEMENT_WALKER_AND_CELLLIST_DISPATCH_GHIDRA_REPORT.md).

10. **Are there OTHER `field_0x6DF` setters?** Phase 1 found one (engineer
    C4 plant in PerCellProcess) and verified BombClass::Detonate does not
    write it. Phase 2 should spot-check `BuildingClass::ReceiveDamage`
    (#18, `0x442230`) and any building-self-destruct mission handler.
    **STILL OPEN.** (Side note: HIGH_BRIDGE_DAMAGE §13.1 reclassifies
    `field_0x6DF` as the **DelayKill latch**, written by
    `TechnoClass::ReceiveDamage @ 0x701F45` (corrected 2026-07-18: was
    `0x701F47` — that address lands mid-instruction, on the disp32 low byte;
    `read_memory(0x701F40, 16)` = `1f 8b 54 24 4c c6 87 df 06 00 00 01 8b 0d 84
    ed` shows the `MOV byte ptr [EDI+0x6DF],1` opcode (`C6 87`) starts at
    `0x701F45`, matching this doc's own §14.1/§11 table — OFFSET_RETYPED_WRONG,
    internal inconsistency within this doc) when a `CausesDelayKill` warhead
    lands a fatal hit on an `EligibleForDelayKill` building. That likely
    closes this question; mark resolved on next sweep.)

11. **`Immune=yes` enforcement site for C4 placement.** Phase 1 located
    the gate at `PerCellProcess` mission-0x11 branch: `vtable[0x160]()`
    is called on the building, and if non-zero, C4 placement is rejected.
    Phase 2 must identify which TechnoTypeClass field `vtable[0x160]`
    reads, to be certain it is `Immune=yes` and not something adjacent.
    **RESOLVED — `vtable[0x160]` is `IsIronCurtainActive`, NOT `Immune`.**
    Per C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION, gamemd has no upstream
    Immune gate on C4 placement; the original hypothesis that "C4 on CABHUT
    does nothing" was driven by an Immune check is refuted. The Rust port's
    observable symptom is a port-side bug, not parity work.

12. **Walker `+0x11A` / `+0x11B` state-byte semantics.** Phase 1 saw values
    `5/7/8/12` on `+0x11A` and `+= 4` accumulation on `+0x11B`. These bytes
    clearly encode ramp transition state. Phase 2 must enumerate every
    write site to know the complete state space. **STILL OPEN.** (Side note:
    HIGH_BRIDGE_DAMAGE §11.1 enumerates 8 `UpdateRamp_*_High` helpers and
    their state transitions; the `+0x11A`/`+0x11B` writes most likely live
    there. A focused enumeration sweep would close this.)

### Remaining open after 2026-05-18 sweeps

- **#4** (formally) — closeable via cite to C4_ON_BRIDGE_REPAIR_HUT_GATE §2.
- **#7** — `ValidateBridgeZones` audit vs MAPCLASS_ZONES_RAMPS_HUT_REGISTRY.
- **#10** (formally) — closeable via cite to HIGH_BRIDGE_DAMAGE §13.1.
- **#12** — `+0x11A`/`+0x11B` write enumeration; partly covered by
  HIGH_BRIDGE_DAMAGE §11.1 and would benefit from a focused sweep.

---

## 8. Phase-1 corrections to existing docs

Phase 1 has confirmed three corrections to prior research. These are written
up below for Phase-2 propagation; per the plan, the actual amendment of
`BRIDGE_SYSTEM.md` follows after user review.

### Correction A — BRIDGE_SYSTEM.md "Bridge Repair Hut Interaction"

> **Current (wrong):** `field_0x6DF` is a "repair pending flag" set when an
> engineer enters the hut.
>
> **Correction (refined in Phase 2 — see §14):** `field_0x6DF` is a
> **dual-purpose pending-action flag** sharing the same byte and timer triple
> (`+0x528`, `+0x52C`, `+0x530`) for two distinct sub-systems:
> 1. **C4-plant-pending:** set by `InfantryClass::PerCellProcess`
>    (`0x519630`, Mission 0x11) when an engineer with `C4=yes`
>    (`InfantryTypeClass+0xEC2`) plants C4 on a building.
> 2. **Crewed-survivor cooldown:** set by `TechnoClass::ReceiveDamage`
>    (`0x701F45`) when a `Crewed=yes` (`TechnoTypeClass+0x1551`) building
>    would take lethal damage — the building survives at HP=1, the flag
>    suppresses repeat fatal-hit handling during the cooldown window.
>
> Both setters write the same byte; the **clearer** is `BuildingClass::Update`
> (`0x43FB20`) on timer expiration, where the `Type[0x16B6]` (BridgeRepairHut)
> branch decides between bridge-destruction dispatch and self-damage. The
> bridge-repair path (engineer entering CABHUT with `mission=8`) never
> touches `field_0x6DF`.

### Correction B — BRIDGE_SYSTEM.md / ADDRESS_MAP.md "RepairBridgeSegment"

> **Current (wrong):** "Walks 3-wide clearing objects" / "walk + repair 3-wide".
>
> **Correction:** `0x575EE0` does not repair and does not clear objects.
> It is a destruction-side trigger-fire walker that calls
> `TechnoClass::ProcessCellAction(0x1F, 0, DAT_00abd480, 0, 0)` on each
> cell in a segment whose `field_0x3C` (TagClass*) is non-null. It iterates
> 4 cells per step (the current cell + 3 perpendicular cells, exact
> perpendicular set depends on axis). It is called only from the four
> `MapClass::FindBridgeEndpoints_*` functions and the two
> `MapClass::UpdateBridgeEdgeTiles_*` functions — all destruction-side
> end-of-segment processing. A more accurate Ghidra label is
> `FireBridgeTriggerActionsOnSegment` or
> `FireProcessCellAction1F_OnBridgeSegment`.

### Correction C — BRIDGE_SYSTEM.md / ADDRESS_MAP.md dispatcher identity

> **Current (wrong):** `FUN_00574C20` / `FUN_00574000` are the "repair
> dispatchers."
>
> **Correction:**
> - `MapClass::DestroyBridge_Low_MapInit` (`0x574C20`) and
>   `MapClass::DestroyBridge_High_MapInit` (`0x574000`) are the
>   **hut-destruction dispatchers**, called from `BombClass::Detonate`
>   (`0x438720`) on demo-truck explosion and from `BuildingClass::Update`
>   (`0x43FB20`) on C4-timer expiration. The `_MapInit` suffix is a misleading
>   Ghidra label; these functions are runtime-called.
> - The **engineer-repair dispatchers** are
>   `ProcessBridgeDestruction_Low` (`0x570050`) and
>   `ProcessBridgeDestruction_High` (`0x573540`), called from
>   `InfantryClass::PerCellProcess` (`0x519630`) when an engineer
>   (`InfantryTypeClass+0xEC3 != 0`) steps onto a `BridgeRepairHut`
>   building cell. The `ProcessBridgeDestruction_*` name is **wrong** — these
>   functions process the engineer-trigger action, which is a REPAIR.
>   A more accurate Ghidra label is
>   `MapClass::DispatchBridgeRepairFromEngineer_Low/_High`.

---

## 9. Sources

**Ghidra addresses decompiled in Phase 1:**
- `0x519630` — InfantryClass::PerCellProcess (full)
- `0x43FB20` — BuildingClass::Update (full; bridge-relevant tail focused)
- `0x57F200` — MapClass::RepairBridge_Low (full)
- `0x57F440` — MapClass::RepairBridge_High (full)
- `0x575EE0` — RepairBridgeSegment (full)
- `0x570050` — ProcessBridgeDestruction_Low (full)
- `0x438720` — BombClass::Detonate (full; Phase-1 spot-check for Conflict A/B)

**xrefs queried in Phase 1:**
- `0x570050`, `0x573540` (ProcessBridgeDestruction_* callers)
- `0x574000`, `0x574C20` (DestroyBridge_*_MapInit callers)
- `0x57F200`, `0x57F440` (RepairBridge_* callers)
- `0x575EE0` (RepairBridgeSegment callers)

**Prior docs reviewed:**
- `BRIDGE_SYSTEM.md` (§"Bridge Repair Hut Interaction" — has the wrong
  `field_0x6DF` claim and wrong dispatcher identity; see §8 corrections)
- `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` (companion; references repair
  addresses in passing)
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` (UpdateBridgeZonesHelper
  internals — unchanged by Phase 1)
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` (§12.3 mentions the
  5×5 scan; Phase 1 confirmed the dimensions)
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` (FUN_00569760
  noted but not deeply decompiled in Phase 1 — deferred)
- `ENGINEER_CAPTURE_GHIDRA_REPORT.md` (does NOT cover the bridge branch)
- `MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md` (correctly identifies
  `field_0x6DF` as a C4-attached flag in the mission_sabotage context;
  Phase 1 confirms)
- `GLOBAL_SOUNDS_GHIDRA_REPORT.md` (RulesClass+0x248 = RepairBridgeSound;
  Phase 1 confirms)
- `EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md` (EVA_BridgeRepaired call site at
  ~`0x519bc9`; Phase 1 confirms via decompilation context)
- `CELLCLASS_ZONES_SPEED_BRIDGES.md` (RepairBridgeSound string at `0x83A7FC`;
  Phase 1 did not re-verify the string address)

**INI files checked in Phase 1 (via parallel agent):**
- `ini/rulesmd.ini` — `BridgeRepairHut=yes` (line 16348), `RepairBridgeSound=`
  (line 721), CABHUT section (line 16336 ff.), `BridgeStrength=1500`
  (line 816), `DestroyableBridges=yes` (line 804), `BridgeExplosions=`
  (line 529)
- `ini/rules.ini`, `ini/artmd.ini`, `ini/eva.ini`, `ini/evamd.ini`,
  `ini/soundmd.ini` — confirmed identical or absent

**Rust files surveyed (via parallel agent):**
- `src/rules/object_type.rs:924` — `bridge_repair_hut` parse
- `src/rules/ruleset.rs:736-746` — `repair_sound` parse
- `src/sim/bridge_state/mod.rs:756-810` — forward-only transitions
- `src/sim/world/world_orders.rs:147-209` — generic capture path
- `src/sim/world/bridge_orchestrator.rs:309-324` — zones_dirty rebuild
- `src/sim/overlay_grid.rs` — overlay grid mutation primitives

---

## 10. Phase 1 status — concluded; Phase 2 follows in §11+

Phase 1 resolved the three pre-flagged conflicts and produced the
disjoint-dispatchers finding (engineer-repair vs hut-destruction live in
separate function trees). Phase 2 then extended this with the walker bodies,
the destruction-side cell mutators, and the C4-Immune keystone trace.
Phase-1 open questions are revisited in §18 (resolved/still-open).

---

## 11. Phase 2 — expanded function inventory

Phase 2 decompiled or located **15 functions beyond the plan's #1–#15
inventory**. Two function-name corrections, one vanilla-YR copy-paste bug,
and one major flag-semantic refinement (`field_0x6DF` dual-purpose, §14).

| Address    | Symbol (current Ghidra)                                     | Phase-2 finding |
|------------|-------------------------------------------------------------|------------------|
| `0x573540` | `ProcessBridgeDestruction_High`                             | Confirmed compiled twin of `_Low` with high-bridge band `[0xCD..0xE8]` and tile-base `DAT_00aa0e28`. Calls `FUN_00568e40` (high pavement walker, twin of `FUN_00569760`). Misnamed (engineer-repair entry). |
| `0x574000` | `MapClass::DestroyBridge_High_MapInit`                      | Hut-destruction dispatcher. 5×5 scan → `DestroyBridgeFromCell_High` if high-bridge overlay found; else ramp-walk with cell-flag bits 0x500/0x100/0x400/0x80/0x800, calls `ApplyDamageToCell` on each cell, calls `MapClass::UpdateAdjacentBridges_High`, sets `Tactical+0xD7C = 1`, **always** calls `UpdateBridgeZonesHelper` at end. The `_MapInit` suffix is a misleading Ghidra label. |
| `0x574C20` | `MapClass::DestroyBridge_Low_MapInit`                       | Compiled twin of `_High` with low-bridge band `[0x4A..0x65]` and tile-base `DAT_00abad1c`. **Same `UpdateAdjacentBridges_High` call** — see §13.4 for the copy-paste-bug note. |
| `0x57F6A0` | `MapClass::RepairBridgeWalker_NS_Low`                       | The actual repair walker. See §12 for full state-transition table. **Xref-verified to be called only from `RepairBridge_Low`** — no destruction path uses it. |
| `0x57FBC0` | `MapClass::RepairBridgeWalker_EW_Low`                       | Compiled twin of NS_Low for the EW orientation. |
| `0x5800D0` | `MapClass::RepairBridgeWalker_NS_High`                      | Compiled twin of NS_Low for the high-bridge band. |
| `0x580600` | `MapClass::RepairBridgeWalker_EW_High`                      | Compiled twin of NS_Low for high EW. |
| `0x577920` | `MapClass::UnregisterBridgeRepairHut`                       | **Misnamed.** Actually a TagClass-cleanup function. See §16. Removes a tag from per-cell registry + global tag list. Called from `FUN_007258d0` (likely TagClass destructor), NOT from hut destruction. |
| `0x5749C0` | `MapClass::DestroyBridgeFromCell_High` (**new**)            | Destruction-side direction-detect + walker dispatcher. Twin of `RepairBridge_High` but calls `CollapseBridge_*_High` family. |
| `0x574780` | `MapClass::DestroyBridgeFromCell_Low` (**new**)             | Destruction-side direction-detect + walker dispatcher. Twin of `RepairBridge_Low`. |
| `0x5746C0` | `MapClass::IsBridgeRampTile` (**new**)                      | Predicate. Matches tile-relative index against 12 ramp-tile patterns (paired with `+0x11A` sub-index 2/4/8/12). |
| `0x574600` | `MapClass::IsLowBridgeEndpointTile` (**misnamed; new**)     | Predicate. Handles **both** low AND high bridge endpoints despite the `_Low` suffix. Parameterized by direction code (2 = NS, 4 = EW). |
| `0x587180` | `ApplyDamageToCell` (**new**)                               | Per-cell damage destruction. Reads `cell+0x44` (overlay); if low-bridge range → calls `DestroyBridge_Low @0x57BAA0`; if high-bridge range → calls `DestroyBridge_High @0x57CCF0`. For non-overlay (ramp) tiles, dispatches to `ProcessBridgeDamageStateMachine_Low/High` (already documented in HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md). |
| `0x57BAA0` | `DestroyBridge_Low` (**new — third "destroy" family**)      | Direction-detect for the per-cell-damage path. Checks overlay range, walks back ±1/±2 to find anchor, dispatches to `DestroyBridgeWalker_NS_Low` or `DestroyBridgeWalker_EW_Low`. Twin of `RepairBridge_Low` but for destruction. |
| `0x57CCF0` | `DestroyBridge_High` (**new — third "destroy" family**)     | High-bridge twin of `DestroyBridge_Low`. |
| `0x576770` | `MapClass::UpdateAdjacentBridges_High` (**new**)            | Neighbor refresh post-destruction. Walks 8 neighbors of input cell; for any with `cell+0x140 & 0x500`, walks the bridge segment in perpendicular axis and calls `MapClass::UpdateBridgeEdgeTiles_High`. **No `_Low` variant exists in the binary** (see §13.4 copy-paste-bug note). |
| `0x56C510` | `MapClass::UpdateBridgeZonesHelper`                         | Full pathfinding zone rebuild. 13 passability classes × BFS coloring. Already documented in MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md. Called as the **final** step of both `DestroyBridge_*_MapInit` (unconditionally) and the repair walkers (conditionally — only when a main-deck damage variant is repaired). |
| `0x569760` | `FUN_00569760` — low-bridge **pavement walker** (**new**)   | Walks up to 30 cells in given direction; handles ramp transitions via `ToggleBridgePavement` and `SetOverlayAndPropagate`; spawns `OverlayClass` markers from `g_OverlayTypeClass_Array[0xED]` or `[0xEE]`. Called from the ramp-handling paths of `ProcessBridgeDestruction_Low`. |
| `0x568e40` | `FUN_00568e40` — high-bridge pavement walker (**new**)      | Twin of `FUN_00569760` for the high-bridge band. Called from `ProcessBridgeDestruction_High`. |
| `0x598030` | `FUN_00598030` — **random pick with retry** (**new**)       | Loop: `Random_Next() → Math_ftol() → retry while result > limit`. Picks a uniform random integer in `[0, limit]`. **NOT a LAT pattern table.** Called by all 4 walkers to choose a healthy-variant offset 0..3. See §12.4 for parity implications. |
| `0x41BF40` | `TechnoClass::IsIronCurtainActive` (**vtable[0x160] target**) | The vtable[0x160] slot on `BuildingClass`. Reads `+0x18C` (last-curtain frame) and `+0x194` (curtain duration). **Reads NEITHER `ObjectTypeClass+0x233` (Immune; see §15 note) NOR `Type[+0xCB8]` (TargetLaser-like).** The C4-on-CABHUT bug's keystone is therefore NOT here. See §15. |
| `0x701F45` | `TechnoClass::ReceiveDamage` — **new `field_0x6DF` setter** | Sets `field_0x6DF = 1` when a `Crewed=yes` building takes lethal damage; building survives at HP=1, the flag is the cooldown gate. Same field, same timer triple as the C4-plant path. See §14. |

**Naming corrections proposed (4 functions):**

1. `RepairBridgeSegment` (0x575EE0) → `FireBridgeTriggerActionsOnSegment` (Phase 1)
2. `ProcessBridgeDestruction_Low/High` (0x570050/0x573540) → `DispatchBridgeRepairFromEngineer_Low/High` (Phase 1)
3. `DestroyBridge_*_MapInit` (0x574000/0x574C20) → drop the `_MapInit` suffix (Phase 1)
4. `MapClass::UnregisterBridgeRepairHut` (0x577920) → `MapClass::UnregisterTagFromCellAndGlobalList` (Phase 2)
5. `MapClass::IsLowBridgeEndpointTile` (0x574600) → `MapClass::IsBridgeEndpointTile` (Phase 2 — `_Low` suffix is wrong; handles both bands)

---

## 12. Walker bodies — full overlay state machine

The four walkers (`RepairBridgeWalker_NS_Low`, `EW_Low`, `NS_High`, `EW_High`)
are compiled twins differing only in their overlay-band constants and in
which axis they walk. All four implement exactly the same algorithm.

### 12.1 Walking direction — Ghidra naming is reversed-from-intuition

| Walker name (Ghidra) | Find-start direction | Main-loop advance direction | Mutated-cells direction |
|----------------------|----------------------|------------------------------|---------------------------|
| `RepairBridgeWalker_NS_Low`  | X−− (find west edge)  | X++ (walk east)             | 3 cells along Y at fixed X: (X, Y−1), (X, Y), (X, Y+1) |
| `RepairBridgeWalker_EW_Low`  | Y−− (find north edge) | Y++ (walk south)            | 3 cells along X at fixed Y: (X−1, Y), (X, Y), (X+1, Y) |
| `RepairBridgeWalker_NS_High` | X−−                   | X++                          | 3 cells along Y (same as NS_Low) |
| `RepairBridgeWalker_EW_High` | Y−−                   | Y++                          | 3 cells along X (same as EW_Low) |

The Ghidra **`NS/EW` qualifier denotes the perpendicular 3-cell-wide direction**
(NS for `(X, Y±1)` triplets, EW for `(X±1, Y)` triplets), **not the walking
direction**. A `_NS_Low` walker actually walks along the EW axis. This is
counter to natural reading; a parity-port should rename to e.g.
`RepairBridgeWalker_Low_Perpendicular_NS` or
`RepairBridgeWalker_Low_RunsEW_WidthNS`.

### 12.2 Find-start: walk backward until out of band, then step in by 1

```
loop {
    coord.<walking axis> -= 1;
    cell = Get_CellClass(coord);
    if cell.field_0x44 < band_low { break; }    // out of band
} while cell.field_0x44 < band_high;            // continue while still in band
coord.<walking axis> += 1;                      // back into the first in-band cell
```

For NS_Low: `band_low = 0x4A`, `band_high = 0x66` (exclusive upper).
For EW_Low: same band.
For NS_High / EW_High: `band_low = 0xCD`, `band_high = 0xE9`.

This positions the walker at the westernmost (or northernmost) cell of the
bridge segment, ready to walk east (or south).

### 12.3 Main loop — per-step structure

```
loop {
    main_cell  = Get_CellClass(coord)
    perp_cell_a = Get_CellClass(coord with <perp axis> -1)
    perp_cell_b = Get_CellClass(coord with <perp axis> +1)

    # Compute redraw bounding box (FUN_0047fde0, FUN_0047fb90, FUN_00487f40)
    # ... screen-rect math, doesn't affect simulation output

    overlay = main_cell.OverlayTypeIndex      # = cell.field_0x44

    new_overlay = match overlay {
        # Damaged main-deck variants OR destroyed anchor
        <damaged-main-deck-range> | <destroyed-anchor> => {
            bVar1 = true;     # mark "did repair a damaged main-deck cell"
            FUN_00598030() + <healthy-band-base>
        }
        # Bridgehead damaged variants — group A
        <bridgehead-A-damaged-range> => <bridgehead-A-base>
        # Bridgehead damaged variants — group B
        <bridgehead-B-damaged-range> => <bridgehead-B-base>
        # Anything else (already-healthy, out-of-band, etc.)
        _ => SKIP this cell
    }

    if new_overlay != overlay {
        main_cell.OverlayTypeIndex  = new_overlay
        perp_cell_a.OverlayTypeIndex = new_overlay
        perp_cell_b.OverlayTypeIndex = new_overlay

        TacticalClass::DirtyScreenRect(<computed rect>, 0)

        if (overlay == <destroyed-anchor>) {
            # Only fires on Destroyed → Healthy transitions
            RadarClass::MarkTerrainDirty(main_cell.coord)
            RadarClass::MarkTerrainDirty(perp_cell_a.coord)   # via MapCoord_Add
            RadarClass::MarkTerrainDirty(perp_cell_b.coord)   # via FUN_00588c60 (negates offset)
        }

        CellClass::RecalcAttributes(main_cell)
        CellClass::RecalcAttributes(perp_cell_a)
        CellClass::RecalcAttributes(perp_cell_b)
        FUN_00487a10(0)    # called 3 times — likely "mark sprite dirty"
        FUN_00487a10(0)
        FUN_00487a10(0)
    }

    coord.<walking axis> += 1
    if FUN_00580b20() == 0 { break; }   # continuation check (Low) — exit when next cell out of band
                                         # (FUN_00580b70 for High walkers)
}

if bVar1 { MapClass::UpdateBridgeZonesHelper(); }   # zones rebuild only on main-deck repair
if (accumulated_cells > 0) { FUN_005868a0(...); }   # final redraw (unused in this path)
return;
```

### 12.4 Complete overlay state-transition table

For each walker, every overlay byte in the matching band falls into one of
4 buckets: **healthy** (default-case skip, no mutation), **damaged-main**
(restored to a random healthy variant 0..3 picked by FUN_00598030),
**bridgehead-A-damaged** (restored to a fixed base), **bridgehead-B-damaged**
(restored to a fixed base), or **destroyed anchor** (restored to a random
healthy variant, with extra radar-dirty marking).

#### Low-bridge band `[0x4A..0x65]` (28 overlay values)

| Overlay value | Walker variant   | Class             | After repair walker   |
|---------------|------------------|-------------------|------------------------|
| `0x4A..0x4D`  | NS_Low (4 vals)  | Healthy main      | (skip — default case)  |
| `0x4E..0x52`  | NS_Low (5 vals)  | Damaged main      | `FUN_00598030() + 0x4A` (random in 0x4A..0x4D); sets `bVar1=true` |
| `0x53..0x56`  | EW_Low (4 vals)  | Healthy main      | (skip)                 |
| `0x57..0x5B`  | EW_Low (5 vals)  | Damaged main      | `FUN_00598030() + 0x53` (random in 0x53..0x56); sets `bVar1=true` |
| `0x5C`        | NS_Low           | Healthy bridgehead-A | (skip — no-op match) |
| `0x5D`        | NS_Low           | Damaged bridgehead-A | `0x5C`              |
| `0x5E`        | NS_Low           | Healthy bridgehead-B | (skip)               |
| `0x5F`        | NS_Low           | Damaged bridgehead-B | `0x5E`              |
| `0x60`        | EW_Low           | Healthy bridgehead-A | (skip)               |
| `0x61`        | EW_Low           | Damaged bridgehead-A | `0x60`              |
| `0x62`        | EW_Low           | Healthy bridgehead-B | (skip)               |
| `0x63`        | EW_Low           | Damaged bridgehead-B | `0x62`              |
| `0x64`        | NS_Low           | **Destroyed anchor** | `FUN_00598030() + 0x4A` (random); `bVar1=true`; radar terrain dirty for 3 perp cells |
| `0x65`        | EW_Low           | **Destroyed anchor** | `FUN_00598030() + 0x53` (random); `bVar1=true`; radar terrain dirty |

Total: 4 + 5 + 4 + 5 + 4 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = 28 ✓ matches Phase-1 band cardinality.

#### High-bridge band `[0xCD..0xE8]` (28 overlay values)

Identical partition shifted by `+0x83` (= +131):

| Overlay value | Walker variant   | Class                | After repair walker   |
|---------------|------------------|----------------------|------------------------|
| `0xCD..0xD0`  | NS_High (4)      | Healthy main         | (skip)                 |
| `0xD1..0xD5`  | NS_High (5)      | Damaged main         | `FUN_00598030() + 0xCD`; `bVar1=true` |
| `0xD6..0xD9`  | EW_High (4)      | Healthy main         | (skip)                 |
| `0xDA..0xDE`  | EW_High (5)      | Damaged main         | `FUN_00598030() + 0xD6`; `bVar1=true` |
| `0xDF`        | NS_High          | Healthy bridgehead-A | (skip)                 |
| `0xE0`        | NS_High          | Damaged bridgehead-A | `0xDF`                 |
| `0xE1`        | NS_High          | Healthy bridgehead-B | (skip)                 |
| `0xE2`        | NS_High          | Damaged bridgehead-B | `0xE1`                 |
| `0xE3`        | EW_High          | Healthy bridgehead-A | (skip)                 |
| `0xE4`        | EW_High          | Damaged bridgehead-A | `0xE3`                 |
| `0xE5`        | EW_High          | Healthy bridgehead-B | (skip)                 |
| `0xE6`        | EW_High          | Damaged bridgehead-B | `0xE5`                 |
| `0xE7`        | NS_High          | **Destroyed anchor** | `FUN_00598030() + 0xCD`; `bVar1=true`; radar dirty |
| `0xE8`        | EW_High          | **Destroyed anchor** | `FUN_00598030() + 0xD6`; `bVar1=true`; radar dirty |

Total: 28 ✓.

**No PartialCollapseA/B states exist in the overlay encoding.** Phase 1
asked whether walkers handle PartialCollapse → Healthy transitions; Phase 2
confirms there are NO such states in the overlay byte. PartialCollapse
is purely a damage-side concept (cell `+0x11E` `bridge_state` byte, per the
`HIGH_BRIDGE_DAMAGE_STATE_MACHINE` doc); during the destroy walker's progress,
cells transition through PartialCollapse states by writing to the
`bridge_state` byte and CHANGING the overlay byte to a destroyed-anchor
value. The repair walker does NOT see PartialCollapse — by the time the
repair fires, cells are either Healthy, Damaged (`0x4E..0x52` / `0xD1..0xD5`),
or Destroyed (`0x64` / `0xE7` / etc.). The repair "skips" intermediate
states because they don't exist in the overlay layer.

### 12.5 The RNG-based variant selection (FUN_00598030)

The plan hypothesized walkers might use the LAT pattern table at
`DAT_0081CC30` to pick deterministic variants. **They do not.**
`FUN_00598030` is `Random__Next` + `Math__ftol` in a rejection-sampling
loop until the result fits in `[0, limit]`. The limit is `3` (4 healthy
main-deck variants), passed via fastcall register.

**Parity implication:** Bridge repair invokes the game's seeded RNG. The
RNG state advances by 1–N calls per repair (N = number of damaged cells
repaired × number of times the rejection loop retries). For multiplayer
lockstep determinism this is fine — all clients have the same seed. For
**replay** correctness, however, the call sequence must be identical, so
the Rust port must use the same per-tick seeded RNG and call it in the
exact same places.

For bridgehead-damage-only repairs (overlays in `0x5C..0x5F`, `0x60..0x63`,
`0xDF..0xE2`, `0xE3..0xE6`), the walker does **NOT** call `FUN_00598030`
— bridgehead repair uses a fixed restoration value, not a variant. So
bridgehead-only repairs advance the RNG by **0**, while main-deck repairs
advance it by some number per damaged cell.

### 12.6 UpdateBridgeZonesHelper gating

`bVar1` is set true **only** when the walker hits a damaged-main-deck or
destroyed-anchor overlay AND writes a new value. Cases that set bVar1=true:
- Low: `0x4E..0x52`, `0x57..0x5B`, `0x64`, `0x65`
- High: `0xD1..0xD5`, `0xDA..0xDE`, `0xE7`, `0xE8`

Cases that do NOT set bVar1 (and so do NOT trigger zones rebuild):
- All bridgehead repairs (Low `0x5D, 0x5F, 0x61, 0x63`; High `0xE0, 0xE2,
  0xE4, 0xE6`)
- Already-healthy cells (default case, no mutation)

**Parity-critical:** A Rust port that always calls "rebuild zones on any
repair" will over-fire compared to gamemd. The correct gate is: rebuild
only when the repair touched a main-deck (or destroyed-anchor) cell.

### 12.7 RadarClass::MarkTerrainDirty gating

Marked dirty ONLY on Destroyed-anchor → Healthy transitions:
- Low: overlay `0x64` (NS) or `0x65` (EW)
- High: overlay `0xE7` (NS) or `0xE8` (EW)

Marked-dirty cells: main cell + both perpendicular neighbors (3 total). The
offset used for the second perpendicular cell is computed via
`FUN_00588c60(buf, &offset)` which is observed to negate a coord — i.e.,
the two perp cells are `+offset` and `−offset` from the main cell.

Bridgehead and damaged-main-deck repairs do NOT mark radar terrain dirty,
even though the bridge's MAP-visible appearance changes. This is a
parity-relevant minor visual: the minimap will only refresh on the
once-per-bridge-segment Destroyed→Healthy transition.

### 12.8 Per-cell side-effects on every mutated cell (3 cells per step)

For each of the 3 cells written, the walker calls:
1. `CellClass::RecalcAttributes(cell)` — recomputes zones, walkability,
   passability. Documented in BRIDGE_DEFERRED_MECHANICS.
2. `FUN_00487a10(0)` — called 3 times in a row (one per perp cell + main).
   Phase 3 should identify; likely "mark cell sprite dirty for next draw."
3. (Indirectly) `TacticalClass::DirtyScreenRect` for the union of the
   3 cells' screen rects.

Note: the walker does NOT explicitly write `cell.field_0x140` flags
(bridge_walkable bit 0x80, bridge cell bit 0x100, bridgehead bit 0x400).
`RecalcAttributes` is responsible for re-deriving these from the new
overlay byte. **A Rust port must keep this dependency: write overlay
first, then RecalcAttributes which derives the flags.**

### 12.9 Recipe — what a Rust repair function must do per cell

For each cell that transitions from a damaged/destroyed overlay to a healthy one:

1. Pick the healthy variant:
   - **Bridgehead repairs**: target = fixed base (`0x5C`, `0x5E`, `0x60`,
     `0x62` for low; `0xDF`, `0xE1`, `0xE3`, `0xE5` for high). No RNG.
   - **Main-deck or destroyed-anchor**: target = `seeded_rng.next_in(0..=3) +
     band_base` where `band_base` is `0x4A` / `0x53` / `0xCD` / `0xD6`.
2. Write the SAME overlay byte to **3 cells**: the main cell and the two
   perpendicular neighbors. Perpendicular axis is opposite to the walking axis.
3. Recompute `cell.flags` / walkability for all 3 cells (Rust equivalent
   of `RecalcAttributes`).
4. If the original overlay was a destroyed anchor (`0x64`, `0x65`, `0xE7`,
   `0xE8`): mark the radar minimap dirty for all 3 cells.
5. After the entire walker traversal completes, **if any main-deck or
   destroyed cells were repaired**: trigger one `zones_dirty → rebuild_pathgrid`.

The walker advances along the walking axis by 1 cell per step and terminates
when the next cell's overlay is outside the band (`FUN_00580b20` / `FUN_00580b70`).

---

## 13. Destruction-side function tree (newly discovered)

Phase 2 found that the destruction side has its own complete function tree,
parallel to the repair tree, with **no shared walker** at the cell-mutation
level. The two trees diverge from the top-level dispatcher down to the
per-cell mutator.

### 13.1 Repair tree (from Phase 1 + 2)

```
PerCellProcess (mission=8, BridgeRepairHut)
  → 5×5 scan → low or high dispatcher
  ├─→ ProcessBridgeDestruction_Low  (0x570050) ── MISNAMED
  │     ├─ 5×5 scan for low-overlay cell
  │     │   └─→ RepairBridge_Low (0x57F200) ── direction-detect
  │     │         ├─→ RepairBridgeWalker_NS_Low (0x57F6A0) ── overlay state machine
  │     │         └─→ RepairBridgeWalker_EW_Low (0x57FBC0)
  │     └─ Ramp-tile branch (no overlay in 5×5):
  │         ├─ recursive ProcessBridgeDestruction_Low (moved coord)
  │         ├─ ToggleBridgePavement, SetOverlayAndPropagate
  │         ├─ ValidateBridgeZones → UpdateBridgeZonesHelper (conditional)
  │         └─→ FUN_00569760 (low pavement walker; spawns OverlayClass markers)
  └─→ ProcessBridgeDestruction_High (0x573540) ── twin
        ├─→ RepairBridge_High (0x57F440)
        │     ├─→ RepairBridgeWalker_NS_High (0x5800D0)
        │     └─→ RepairBridgeWalker_EW_High (0x580600)
        └─→ FUN_00568e40 (high pavement walker)
```

### 13.2 Destruction tree — hut-death path

```
BombClass::Detonate (0x438720)   [demo truck on CABHUT]
  └─┐
BuildingClass::Update (0x43FB20) [C4 timer expired on CABHUT]
    └─→ 5×5 scan → low or high
        ├─→ MapClass::DestroyBridge_Low_MapInit  (0x574C20)
        │     ├─ 5×5 scan for low-overlay cell
        │     │   └─→ DestroyBridgeFromCell_Low (0x574780) ── direction-detect
        │     │         ├─→ CollapseBridge_NS_Low (addr TBD — Phase 3)
        │     │         └─→ CollapseBridge_EW_Low (addr TBD)
        │     ├─ Ramp-tile walk + ApplyDamageToCell per cell
        │     │   └─→ IsBridgeRampTile (0x5746C0), IsLowBridgeEndpointTile (0x574600)
        │     ├─→ UpdateAdjacentBridges_High (0x576770)   ← see §13.4
        │     ├─ writes Tactical+0xD7C = 1 (renderer-dirty)
        │     └─→ UpdateBridgeZonesHelper (0x56C510) UNCONDITIONAL
        └─→ MapClass::DestroyBridge_High_MapInit (0x574000) — twin
              ├─→ DestroyBridgeFromCell_High (0x5749C0)
              │     ├─→ CollapseBridge_NS_High
              │     └─→ CollapseBridge_EW_High
              └─ rest identical to Low MapInit
```

### 13.3 Destruction tree — per-cell warhead-damage path

```
Apply_area_damage (warhead hits a bridge cell)
  → ApplyDamageToCell (0x587180)
    ├─ if overlay in [0x4A..0x63]:
    │   └─→ DestroyBridge_Low  (0x57BAA0)  ── direction-detect
    │         ├─→ DestroyBridgeWalker_NS_Low (addr TBD)
    │         └─→ DestroyBridgeWalker_EW_Low (addr TBD)
    ├─ if overlay in [0xCD..0xE6]:
    │   └─→ DestroyBridge_High (0x57CCF0)
    └─ if ramp tile (IsBridgeRampTile):
        └─→ ProcessBridgeDamageStateMachine_Low / _High (already documented)
```

The full destruction-walker family (`CollapseBridge_*_*` and
`DestroyBridgeWalker_*_*`) has **6 functions** by Phase-2 count: 2 for
hut-destruction × NS/EW, 2 for warhead-damage × NS/EW, both Low and High.
Their bodies (overlay-mutation pattern, exact bridge_state transitions)
are deferred to Phase 3 unless the user asks earlier.

### 13.4 Copy-paste bug in vanilla gamemd.exe

Both `DestroyBridge_Low_MapInit` (0x574C20) and `DestroyBridge_High_MapInit`
(0x574000) call **`MapClass::UpdateAdjacentBridges_High`** (0x576770) for the
post-destruction neighbor refresh. There is NO `UpdateAdjacentBridges_Low`
in the binary — both code paths reuse the High version.

**This is a vanilla-YR bug.** When a CABHUT serving a LOW bridge is
destroyed, the adjacent-bridge refresh uses the High-bridge edge-tile
logic. In practice this may have minor visible effects (low-bridge
neighbor cells may not get their edge tiles updated correctly), but it's
also possible the inner logic auto-detects bridge type from the cell's
overlay range and is a no-op for low-bridge neighbors. **Phase 3 should
test in-game** whether this manifests as a visible glitch when destroying
a low-bridge CABHUT.

For the Rust port, the parity choice is:
- **Faithful**: replicate the bug — both Low and High destruction paths
  call the same "UpdateAdjacentBridges_High"-equivalent function.
- **Fix-forward**: implement separate low/high neighbor refresh and accept
  a (small) observable-output divergence.

Per the parity bar, **default to faithful** with a code comment flagging
the gamemd.exe bug for future debate.

---

## 14. `field_0x6DF` revised: DUAL-PURPOSE flag (refines Conflict A)

Phase 1 identified one setter (engineer C4 plant). Phase 2 performed an
**exhaustive byte-pattern search** for `mov byte ptr [reg+0x6df], 1` across
all 6 register encodings (ESI/EDI/EAX/ECX/EDX/EBX) and found **one
additional setter** that Phase 1 did not catch.

### 14.1 Complete setter inventory

| Site address | Function                                  | Value | Purpose |
|--------------|-------------------------------------------|-------|---------|
| `0x51A5A7`   | `InfantryClass::PerCellProcess` (Mission_Sabotage branch) | `=1`  | C4-plant-pending: engineer plants C4, building's destruction-timer starts ticking. |
| `0x701F45`   | `TechnoClass::ReceiveDamage` (Crewed=yes branch) | `=1` | Crewed-survivor cooldown: building took lethal damage, was Crewed=yes (TechnoTypeClass+0x1551), HP clamped to 1, flag suppresses repeat fatal hits during cooldown. Also writes timer triple `+0x528 / +0x52C / +0x530`. |
| `0x440320`   | `BuildingClass::Update` (timer-expiry, BridgeRepairHut else-branch only) | `=0`  | Clearer fires ONLY on the BridgeRepairHut branch — immediately after the CALL to `DestroyBridge_*_MapInit`, inside the else-block. The non-hut path (vtable[0x16C] at 0x440358) jumps straight to the function epilogue (0x44035E) and skips this clearer entirely. Verified via `decompile_function 0x43FB20`. Flag persistence on the non-hut path is implicit-via-building-death. |

**No other setters.** Byte-pattern search was exhaustive across all standard
ModRM register encodings.

### 14.2 Implications for the Rust port

- The Rust analog of `field_0x6DF` is a single per-building boolean +
  associated frame timer fields. Both the C4-plant and the
  Crewed-survivor systems must share that state.
- The disambiguation between the two purposes happens in the **clearer**
  (`BuildingClass::Update`): if `Type[0x16B6]` (BridgeRepairHut), the
  expired-timer branch destroys the bridge (and DOES NOT damage the hut
  via `vtable[0x16C]`). Otherwise, it applies area damage via
  `vtable[0x16C]` with the C4 warhead. The Crewed-survivor case
  benefits from the timer expiring at all (the building gets
  "out of cooldown" and can take fatal damage again).
- For CABHUT specifically: `Immune=yes` means `ReceiveDamage` early-outs
  before reaching `0x701F45`, so the Crewed-survivor code path is
  unreachable for CABHUT. Thus for CABHUT, `field_0x6DF` is **exclusively**
  the C4-plant flag.
- For general buildings (e.g., GAREFN, etc.) with both `BridgeRepairHut=yes`
  and a non-immune posture: in theory the Crewed-survivor path could
  collide with C4 placement, but in vanilla YR no such building exists
  (CABHUT is the only `BridgeRepairHut=yes` building and it's `Immune=yes`).

### 14.3 Phase-1 Correction A status — UPDATED

Conflict A's resolution remains: BRIDGE_SYSTEM.md's "repair pending flag"
is wrong. But the **refined** truth is dual-purpose, not single-purpose.
The `BRIDGE_SYSTEM.md` correction in §8 was patched inline to reflect this.

---

## 15. vtable[0x160] keystone — Iron Curtain, NOT Immune

> **Immune-offset correction (2026-05-20 audit):** earlier drafts of this
> doc cited the Immune flag at `TechnoTypeClass+0xC4D` / `Type[+0xC4D]`. The
> correct offset is **`ObjectTypeClass + 0x233`**, verified via
> `read_memory(0x5F9510, 7)` = `88 83 33 02 00 00` = `MOV byte ptr [EBX+0x233], AL`
> inside `ObjectTypeClass::ReadINI @ 0x005F92D0` (PUSH "Immune\0" at `0x832B70`).
> Inline references below use the corrected offset.

### 15.1 Result

- **Address at BuildingClass.vtable[0x160]**: `0x41BF40`
- **Ghidra label**: `TechnoClass::IsIronCurtainActive`
- **Vtable location**: `vtable_BuildingClass @ 0x7E3EBC`, slot `+0x160` at `0x7E401C`

The function reads only instance fields:
- `this + 0x18C` = `IronCurtainLastFrame` (frame counter when curtain was applied)
- `this + 0x194` = `IronCurtainDuration` (frames the curtain lasts)

Returns true while `g_CurrentFrameCounter - this.IronCurtainLastFrame < this.IronCurtainDuration`.
**Does not read `ObjectTypeClass+0x233` (Immune) or any other type-level flag.**

### 15.2 What this means for `project_c4_bridge_hut_followup`

The C4-on-CABHUT bug's gate is **NOT** at `vtable[0x160]` in
`PerCellProcess`. A non-curtained CABHUT passes vtable[0x160] cleanly,
so C4 placement should proceed via PerCellProcess Mission_Sabotage.

The real gate must be upstream of PerCellProcess. The candidates that
match the observed symptom ("clicking C4 on CABHUT does nothing") are:

1. **Click-target validation** — the player's click on a CABHUT cell
   either doesn't generate a valid `Mission_Sabotage` order at all, or
   the engineer's pathing target rejects CABHUT before the engineer reaches
   it. `Immune=yes` may gate selection at the cursor / order-validation
   layer.
2. **Mission/targeting filtering** — there is likely a `WeaponClass::Can_Attack`
   or `Mission::Can_Sabotage` (or similar) predicate that consults the
   target's `ObjectTypeClass+0x233` (Immune). This is a different layer than
   PerCellProcess.
3. **Pathfinding refusal** — Immune buildings may not be valid pathing
   destinations.

**Phase 3 keystone target:** trace the click-to-mission-assignment path
for a SEAL/Tanya holding C4 and a CABHUT under the cursor. The candidate
predicate functions are: `WeaponClass::Can_Attack` (warhead-vs-armor),
`TechnoClass::Owned_By_Player_Or_Allies`, `BuildingClass::Can_Be_Targeted_For_C4`
(if it exists), or any function that reads `ObjectTypeClass+0x233`
(Immune flag).

### 15.3 Lesson learned (Ghidra-trust verification)

Phase 1 assumed `vtable[0x160]` was the Immune check based on the
control-flow position (early-out before C4 placement). Phase 2's live
`read_memory` verification of the vtable proved that assumption WRONG —
the slot resolves to `IsIronCurtainActive`, a runtime-state check, not a
type-flag check.

This is exactly the failure mode flagged by the memory entry
`feedback_vtable_binding_verification.md` ("every vtable-override claim
must be confirmed by live read_memory"). Phase 2 paid this verification
cost and found the keystone in a different place.

---

## 16. `UnregisterBridgeRepairHut` (0x577920) is misnamed

The function's name implies it removes a building from a hut registry on
destruction. Phase 2 decompilation shows otherwise:

```
fn UnregisterBridgeRepairHut(map: &MapClass, param_2: TagClass*):
    // param_2 is a TagClass* (verified by RTTI check 0x2C at top)
    if param_2.vtable[0x2C]() != 0x2C { return; }   // not a TagClass — skip

    // Pass 1: scan MapClass+0x1160 (per-cell tag-coord array, count at +0x116C)
    for entry in map.tag_coords[..count]:
        cell = Get_CellClass(entry)
        if cell.field_0x3C == param_2:    // this cell's tag is the one being unregistered
            FUN_00485250(0)               // some cleanup
            idx = map.tag_vector[+0x115c].Find(cell.field_0x24)
            if idx != -1: remove from MapClass+0x1160 by shifting

    FUN_00485130(param_2)                  // final tag cleanup

    // Pass 2: scan global DAT_008B41AC tag list (count at DAT_008B41B8)
    idx = DAT_008B41A8.vtable[0x10](&param_2)
    if idx != -1 and idx < DAT_008B41B8:
        DAT_008B41B8 -= 1
        shift DAT_008B41AC entries down to fill idx
```

**Findings:**
- The function operates on a **TagClass** (RTTI 0x2C), not a building.
- It removes the tag from both the per-cell registry at `MapClass+0x1160`
  and the global tag list at `DAT_008B41A8` / `DAT_008B41AC`.
- `cell.field_0x3C` is the cell's TagClass pointer (already known from
  RepairBridgeSegment in Phase 1).
- It is called from `FUN_007258d0` at addresses 0x725AE5 and 0x725B57
  — likely the TagClass destructor or scenario cleanup. **NOT** from any
  hut-destruction path.

**Implications for the hut registry mental model:**
- `MapClass+0x1160` is NOT a "list of bridge-repair-hut buildings."
- It is a list of **CELLS that have an attached tag pointer**. Bridge
  repair huts may register here if they have an associated TagClass, but
  the registry is general-purpose tag tracking.
- The Phase-1 plan's interpretation (registry stores `BuildingClass*`
  entries) is wrong. The registry stores `(x, y)` cell coords, and the
  per-cell `cell.field_0x3C` is checked against the tag pointer being
  unregistered. Per
  `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §5, this had
  already been partially corrected — Phase 2 confirms.

**Bridge-collapse-on-hut-destruction is NOT routed through this function.**
The actual hut-destruction path is:
- C4 timer expires → `BuildingClass::Update` → 5×5 scan → `DestroyBridge_*_MapInit`
- Demo-truck → `BombClass::Detonate` → 5×5 scan → `DestroyBridge_*_MapInit`

Neither path invokes `UnregisterBridgeRepairHut`. The hut registry is for
TAG management (scenario-defined triggers), not for hut-destruction
tracking. **Phase 3 should re-verify** that no part of the destruction
cleanup path touches the tag registry.

---

## 17. Phase 2 helper reference table

| Address    | Symbol                              | Brief role                                                                       |
|------------|-------------------------------------|----------------------------------------------------------------------------------|
| `0x42fcb0` | `DynamicVectorClass::Constructor`   | Initializes a stack-resident DynamicVector with vtable pointer, capacity, owned flag. Used throughout the walker bodies. |
| `0x42f7c0` | `DynamicVectorClass::Clear/Free`    | Frees the heap-allocated buffer if owned. Called on max-iteration exit in FUN_00569760. |
| `0x485250` | `DynVec_AddRef` (target assign)     | Generic refcount-with-vector slot setter. Used in UnregisterBridgeRepairHut as part of tag bookkeeping. |
| `0x485130` | `DynVec_RemoveRef` (target unassign) | Inverse of 0x485250. |
| `0x568E40` | High pavement walker                | High-bridge twin of FUN_00569760 — handles ramp tiles for high bridges (called from ProcessBridgeDestruction_High). |
| `0x569760` | Low pavement walker (FUN_00569760)  | Handles ramp tiles for low bridges. Walks up to 30 cells; spawns OverlayClass markers; touches `cell.+0x11B += 4` for transition cells. |
| `0x576770` | `UpdateAdjacentBridges_High`        | Post-destruction neighbor refresh. NO `_Low` variant — both Low and High MapInit call this (gamemd.exe bug — §13.4). |
| `0x580B20` | Walker continuation check (Low)     | Returns true if `cell+0x44` in `(0x49, 0x66)`. Used by Low walkers as the "stay in band" check. |
| `0x580B70` | Walker continuation check (High)    | Returns true if `cell+0x44` in `(0xCC, 0xE9)`. Used by High walkers. |
| `0x586990` | Cell-list zone-marker pass          | Per-cell zone-map clear + RecalcAttributes pass over a coord list. Used after ramp/repair operations. |
| `0x5868a0` | Rectangle-region driver             | Builds a DynamicVector of cells in a rect, calls 0x586990, frees. Used as `FUN_005868a0(&local_18)` at walker end. |
| `0x598030` | Random pick with retry              | `Random_Next + Math_ftol` rejection-loop. Picks uniform `[0, limit]`. Used by walkers for variant selection. NOT a LAT pattern. |
| `0x7C8B3D` | `operator_delete` trampoline        | One-liner free wrapper. |
| `0x46F560` | `Random__Next` (if labeled — else find via call) | The game's seeded RNG. Phase 2 didn't deeply verify; Phase 3 may confirm. |
| `0x4885A0` | `Math__ftol` (if labeled)            | Float-to-long conversion used by FUN_00598030. |

---

## 18. Phase-1 open questions — resolved or still open

| # | Q (from Phase-1 §7)                                                | Status   | Reference / Phase-2 answer |
|---|---------------------------------------------------------------------|----------|----------------------------|
| 1 | Walker per-cell writes (Destroyed→Healthy table)                   | **RESOLVED** | §12.4 full state table |
| 2 | Are walkers shared between repair and destruction?                 | **RESOLVED — NO** | §11 xref check + §13 separate trees |
| 3 | What does `DestroyBridge_*_MapInit` actually do?                   | **RESOLVED** | §11 entry + §13.2 chain |
| 4 | `InfantryTypeClass+0xEC3` = Engineer flag (inferred)              | Still inferred | Phase 3: verify against INI parser for `Engineer=yes` |
| 5 | `VoxClass::PlayEVA(0xFFFFFFFF)` sentinel meaning                   | Open | Phase 3: decompile VoxClass::PlayEVA |
| 6 | `DAT_00a83dec` listener registry subscribers                       | Open | Phase 3: search writers |
| 7 | `MapClass::ValidateBridgeZones` body                                | Partially open | Cross-reference to MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md — needs re-verify of repair-side caller binding |
| 8 | ProcessCellAction codes `0x1F` and `0x30`                          | Open | Phase 3: decompile TechnoClass::ProcessCellAction |
| 9 | `FUN_00569760` body                                                 | **RESOLVED** | Phase 2 decompile complete; pavement walker for ramp tiles |
| 9b | `FUN_00586990` body                                                | **RESOLVED** | §17 — cell-list zone-marker pass |
| 10 | Other `field_0x6DF` setters                                        | **RESOLVED** | §14 — exhaustive byte-pattern search; found Crewed-survivor setter |
| 11 | `vtable[0x160]` Immune-gate keystone                                | **RESOLVED — IT'S IRON CURTAIN, NOT IMMUNE** | §15. C4-on-CABHUT bug's true gate is upstream of PerCellProcess. |
| 12 | Walker `+0x11A` / `+0x11B` state-byte semantics                    | Partially resolved | Phase 1 noted in ramp-handling; full enumeration deferred to Phase 3 (`IsBridgeRampTile` and `IsLowBridgeEndpointTile` use sub-indices 2/4/8/12 on `+0x11A`; `+0x11B` accumulates +4 across ramp transitions) |

---

## 18A. 2026-05-17 Phase 3 extension - C4 action gate and hut-destruction collapse tree

**Scope:** Targeted follow-up for the reported player-visible symptom:
SEAL/Tanya can plant C4 on a bridge repair hut (`CABHUT`), but the bridge
must collapse after the C4 timer expires. This extension answers only the
binary side of that question. Rust implementation differences belong in
the repo fidelity/trace artifacts.

**Active in YR:** Yes. Every function below is on the standard YR runtime
path for a selected C4 infantry unit targeting a BridgeRepairHut and for
the subsequent `BuildingClass::Update` C4-timer expiry.

### 18A.1 C4 action gate: `Immune=yes` does not block the player command

`InfantryClass::What_Action_OnObject` at `0x0051E3B0` returns the C4
action (`0x10`) for a human-selected infantry target when all of these
conditions hold:

- the selected infantry type has `InfantryTypeClass+0xEC2` set
  (`C4=yes`) or its weapon reports ability `0xE`;
- the base action already resolved to attack (`5`);
- the target RTTI is building (`6`);
- the target's virtual destroyed/dead predicate at vtable `+0x80` is
  false;
- the building type's `CanC4` byte at `BuildingTypeClass+0x1577` is true;
- `InvisibleInGame` at `BuildingTypeClass+0x1701` is false.

No branch on the path that returns the C4 action (`0x10`) reads the
`Immune=yes` type byte (corrected 2026-07-18: was "No branch in this action
function reads..." — overbroad. `decompile_function(0x0051E3B0)` shows a
DIFFERENT, unrelated branch late in the same function, reached only when
a generic action code resolves to `5` well after the C4 check has already
returned: `iVar7 = (**param_2->vtable[0x88])(); if (*(char*)(iVar7+0x233)
== '\0') { return 5; } return 2;` — this does read `ObjectTypeClass+0x233`
(Immune, per §15's corrected offset), but it sits on a separate action-code
path the C4 return (`return 0x10;`) never reaches, since that return exits
the function immediately. MISLEADING, root cause INFERENCE_HARDENED — the
narrower, still-true claim is "the C4-action branch specifically does not
consult Immune"). The Phase 2 finding remains correct that vtable `+0x160`
is Iron Curtain state, not the type `Immune` key; this extension further
confirms that the normal action-cursor path for C4 specifically does not
reject CABHUT for `Immune=yes`.

### 18A.2 C4 plant side effect: hut C4 marker and timer

`InfantryClass::PerCellProcess` at `0x00519630`, in mission `0x11`
(sabotage/C4), requires the looked-up building in the current cell to
match the infantry nav target. On success it writes the building C4 marker
byte `BuildingClass+0x6DF = 1`, records attacker/timer fields including
the attacker pointer at `+0x540`, stops movement, and switches out of the
planting state.

The relevant detail for parity is that gamemd claims the plant from the
target building cell lookup, not from a Chebyshev-adjacent attacker cell.
The later bridge decision is entirely driven by the building's update
timer, not by the infantry position.

### 18A.3 Timer expiry: CABHUT does not take normal building damage

`BuildingClass::Update` at `0x0043FB20`, when `BuildingClass+0x6DF != 0`
and the C4 timer has elapsed, computes damage as the building's current
health. If `BuildingTypeClass+0x16B6` (`BridgeRepairHut`) is false, it
passes that health amount to the normal damage vtable with `C4Warhead`.

If `BridgeRepairHut` is true, it does not use normal building damage.
Instead, it scans a hut-centered 5x5 square and selects the bridge class:

- low bridge if any scanned cell has tile index in
  `[DAT_00abad1c, DAT_00abad1c + 0x0F]` or overlay index
  `0x4A..0x65`;
- high bridge otherwise, using the high dispatcher.

After the destruction dispatcher returns, gamemd clears
`BuildingClass+0x6DF = 0` and clears `BuildingClass+0x540 = 0`. This is
true for CABHUT as well as the normal damage path.

### 18A.4 Hut-destruction entry: overlay-first, flags-second

`MapClass::DestroyBridge_Low_MapInit` (`0x00574C20`) and
`MapClass::DestroyBridge_High_MapInit` (`0x00574000`) are runtime
destruction entries despite the `_MapInit` label.

Both functions first perform an inner 5x5 scan around the input hut cell:

- low accepts overlay band `0x4A..0x65`;
- high accepts overlay band `0xCD..0xE8`;
- the first overlay match immediately calls `DestroyBridgeFromCell_*` and
  returns.

Only if no overlay is found does the fallback read cell flags at
`CellClass+0x140`. The fallback accepts `flags & 0x500`, otherwise searches
all 8 directions up to 3 cells outward for such a cell. If neither
`0x100` nor `0x400` is present after that, it returns with no collapse.

The fallback then derives a bridge/ramp anchor from `cell+0x24`,
`cell+0x2C`, and flags `0x80`/`0x800`, walks until
`MapClass::IsBridgeRampTile`, calls `ApplyDamageToCell` up to three times,
continues toward `MapClass::IsLowBridgeEndpointTile`, and finally calls
`MapClass::UpdateAdjacentBridges_High`, sets `g_Tactical+0xD7C = 1`, and
calls `UpdateBridgeZonesHelper`. Both low and high entries call the
`High` adjacent-bridge updater; this copy-paste quirk remains verified.

### 18A.5 `DestroyBridgeFromCell_*`: canonicalize the start cell before collapse

`MapClass::DestroyBridgeFromCell_Low` (`0x00574780`) and
`MapClass::DestroyBridgeFromCell_High` (`0x005749C0`) read the overlay of
the matched bridge cell and dispatch by overlay subrange.

Low subranges:

- NS-oriented set: `0x4A..0x52`, `0x5C..0x5F`, and `0x64`;
- EW-oriented set: `0x53..0x5B`, `0x60..0x63`, and `0x65`.

High subranges:

- NS-oriented set: `0xCD..0xD5`, `0xDF..0xE2`, and `0xE7`;
- EW-oriented set: `0xD6..0xDE`, `0xE3..0xE6`, and `0xE8`.

For either bridge height, the function checks one cell and two cells
behind the matched overlay along the bridge axis. Depending on whether
those cells remain in the bridge overlay band, it calls the matching
`CollapseBridge_*_*` function at either `matched + 1`, `matched`, or a
computed fallback coordinate from `FUN_00588C60`. This means gamemd is not
limited to a precomputed span-anchor tag; it starts from any bridge overlay
the hut scan can see and then recenters to a canonical collapse start.

### 18A.6 `CollapseBridge_*_*`: four-step sweep with explosion anims

The four collapse dispatchers are:

- `0x00575220` - `MapClass::CollapseBridge_EW_Low`
- `0x00575540` - `MapClass::CollapseBridge_NS_Low`
- `0x00575870` - `MapClass::CollapseBridge_EW_High`
- `0x00575BA0` - `MapClass::CollapseBridge_NS_High`

Each one scans both directions along the bridge overlay band to estimate
which side is shorter, chooses sweep direction (`+1` or `-1`) based on
that comparison, computes a midpoint-biased start
`start - (back_count - forward_count) / 2`, and then performs at most
four collapse steps. Each step calls `DestroyBridge_Low` or
`DestroyBridge_High` up to three times until one call returns true, then
advances one cell along the chosen axis and stops early if the next cell
leaves the bridge overlay band.

Before each destroy attempt, the functions spawn bridge explosion anims
on three perpendicular cells unless the current center overlay is already
the terminal cap for that axis (`0x64`/`0x65` low, `0xE7`/`0xE8` high).
Anim type is chosen randomly from the rules explosion vector at
`RulesClass+0x15C` with count `RulesClass+0x168`; frame/random argument is
`RandomRanged(1,5)`.

All four collapse dispatchers call `UpdateBridgeZonesHelper` and set
`g_Tactical+0xD7C = 1` after the sweep.

### 18A.7 `DestroyBridge_*` and walkers: overlay mutation is 3-cell wide

`DestroyBridge_Low` (`0x0057BAA0`) and `DestroyBridge_High`
(`0x0057CCF0`) are per-cell direction dispatchers. They re-check the
current cell overlay subrange, canonicalize one or two cells backward
when needed, and then call one of four walkers:

- `0x0057BCF0` - `MapClass::DestroyBridgeWalker_NS_Low`
- `0x0057C2B0` - `MapClass::DestroyBridgeWalker_EW_Low`
- `0x0057CF60` - `MapClass::DestroyBridgeWalker_NS_High`
- `0x0057D530` - `MapClass::DestroyBridgeWalker_EW_High`

The walkers mutate the current cell plus the two perpendicular neighbor
cells to the same new overlay index, then mark terrain/screen state and
recalculate all three cells. The terminal main-deck state returns true and
also calls the matching `FindBridgeEndpoints_*_*` function:

| Walker | Partial/special cases | Terminal destroyed cap |
|--------|------------------------|------------------------|
| NS Low | `0x5C -> 0x5D`, `0x5E -> 0x5F`, `<0x50 -> 0x50` | `0x50..0x52 -> 0x64` |
| EW Low | `0x60 -> 0x61`, `0x62 -> 0x63`, `<0x59 -> 0x59` | `0x59..0x5B -> 0x65` |
| NS High | `0xDF -> 0xE0`, `0xE1 -> 0xE2`, `<0xD3 -> 0xD3` | `0xD3..0xD5 -> 0xE7` |
| EW High | `0xE3 -> 0xE4`, `0xE5 -> 0xE6`, `<0xDC -> 0xDC` | `0xDC..0xDE -> 0xE8` |

The per-neighbor helper functions
`MapClass::ApplyBridgeDestruction_NS/EW_Low/High` are guarded by the same
bridge overlay band (`0x4A..0x65` or `0xCD..0xE8`). They use
`CheckBridgeNeighbors_*_*` to select the next overlay from a 16-entry
lookup table; if the computed overlay already equals the cell overlay,
they return without rewriting.

### 18A.8 Parity implication for the current bug investigation

For the original executable, a C4 plant on CABHUT should reach bridge
collapse if the hut-centered 5x5 scan sees a bridge overlay, or if the
fallback can reach a bridge/ramp cell through `CellClass+0x140` flags.
There is no evidence that gamemd requires a runtime "anchor span id" on
the scanned cell. The binary uses map tile/overlay/flag evidence first and
then canonicalizes the actual collapse start.

This is the key behavior to compare against the Rust trace: any Rust path
that accepts C4 on CABHUT but only dispatches collapse for 5x5 cells with
a precomputed bridge span/anchor tag can legitimately no-op on topologies
where gamemd would continue through overlay or ramp/flag evidence.

---

## 19. Remaining open questions for Phase 3

**2026-05-17 note:** Items 1 and 2 below were resolved by the targeted
Phase 3 extension in §18A. They are retained here as historical open
questions from Phase 2; the live remaining work starts at item 3.

1. **C4-on-CABHUT bug — locate the real gate.** Trace the click-to-mission
   chain for SEAL/Tanya holding C4, targeting a CABHUT. Suspect:
   `Mission::Can_Sabotage` or `Can_Attack` predicate reading
   `ObjectTypeClass+0x233` (Immune; see §15 note for the prior-draft correction). § 15.2 lists candidates.
2. **`CollapseBridge_*_*` and `DestroyBridgeWalker_*_*` bodies** (6
   functions). Verify that the destruction walker is the inverse of
   the repair walker (overlay → Destroyed/Damaged transitions).
3. **`MapClass::ValidateBridgeZones`** body — confirm that the binary's
   zones-dirty analog is gated on validator return as Phase 1 indicated.
4. **`TechnoClass::ProcessCellAction(0x1F)` and `(0x30)`** — what do
   actions 31 and 48 do? Likely tag-trigger fires (per the gap-scan
   §D2.5 reading), but the exact handlers are unverified.
5. **`VoxClass::PlayEVA(0xFFFFFFFF)`** — what does the sentinel mean?
   Likely "play the most recently scheduled EVA" but unverified.
6. **`DAT_00a83dec` "bridge repair listener" registry** — find writers.
   Currently believed to be a per-instance subscriber array used by
   HouseClass (or similar) for "bridge repaired" notifications.
7. **`Tactical+0xD7C`** — the post-destruction renderer-dirty flag set
   by both `DestroyBridge_*_MapInit`. Likely a global "redraw all bridges"
   refresh trigger. Phase 3 should identify the consumer.
8. **`g_OverlayTypeClass_Array[0xED]` and `[0xEE]`** — the overlay types
   spawned during ramp repair by `FUN_00569760`. Identify by INI name
   (likely something like "BridgeOverlayMarker" or similar).
9. **`InfantryTypeClass+0xEC3` (Engineer flag) and `+0xEC4` (Agent/Spy
   flag)** — confirm against the INI parser.
10. **Audio call-graph completeness** — Phase 1 traced the EVA + Voc
    play sites; Phase 2 didn't re-verify. Phase 3 should sanity-check
    that the audio fires before bridge state mutation.
11. **Multi-hut interaction** — what happens if two CABHUTs serve the
    same bridge and both are damaged/destroyed in the same tick? Both
    have separate timers, so the destruction may fire twice.
12. **Vanilla bug in §13.4** — confirm in-game whether destroying a
    low-bridge CABHUT produces visible glitches due to
    `UpdateAdjacentBridges_High` being called for low bridges.

---

## 20. Next steps

Phase 1 + Phase 2 deliver all five Phase-1 conflict resolutions plus a
complete walker state machine, a destruction-side tree, and the C4
keystone redirect. The plan's success criteria (plan §11) are met:

| Plan §11 criterion                                              | Status |
|------------------------------------------------------------------|--------|
| Resolve Conflicts A, B, C with disassembly citations             | ✅ §1, §8, §14 |
| Walker state-transition table (Destroyed/PartialCollapse/Damaged → Healthy) | ✅ §12.4 (PartialCollapse confirmed not represented in overlay byte) |
| Cell-selection scope (5×5? bridge-group?)                        | ✅ Phase 1 §3.1 + Phase 2 §12 — 5×5 scan + per-band walker traversal |
| Cell-field writes per walker                                     | ✅ §12.3, §12.8 |
| Zones rebuild trigger (`UpdateBridgeZonesHelper`) firing logic   | ✅ §12.6 (conditional on main-deck repair only) |
| EVA + sound dispatch                                              | ✅ Phase 1 §3.1 step A/B (Phase 3 may deepen) |
| All §9 open questions resolved or re-documented                  | ✅ §18 + §19 |
| "Active in YR: yes/no/conditional" per claim                     | ✅ Every function and finding is on an active runtime path; the only TS-legacy risk noted (and dismissed) was the `_MapInit` suffix on #12/#13 |
| BRIDGE_SYSTEM.md correction note                                 | ✅ Already applied in Phase 1; updated inline in §8 Correction A for Phase 2 refinement |

**The paused `/brainstorm` for runtime `bridge_walkable` invalidation can
now resume with the verified spec** (per the plan's "Next Pipeline Step").
Key inputs for the brainstorm:

- **Repair semantics** (§12.9): the cell-level recipe is well-defined —
  pick variant, write overlay to 3 cells, RecalcAttributes, conditionally
  mark radar + rebuild zones. The brainstorm's "what does our trigger fire"
  question is fully answered.
- **Destruction semantics** (§13): the hut-death-destroys-bridge path is
  separate from any repair path; reuses the same `zones_dirty → rebuild_pathgrid`
  infrastructure but with an unconditional rebuild call.
- **The C4-on-CABHUT bug** is not in the bridge code — it's upstream
  in the targeting/mission-assignment chain. The brainstorm should treat
  this as **OUT OF SCOPE** for the bridge-repair feature, and create a
  separate follow-up for the C4 bug (per §15.2).
- **Vanilla copy-paste bug** (§13.4): the Rust port should faithfully
  reproduce the `UpdateAdjacentBridges_High`-only call from both paths
  with a flagged comment.

**2026-05-17 correction:** The earlier "C4 bug is upstream" bullet is
superseded by §18A. The gamemd action and plant gates allow C4 on CABHUT;
the bridge-destruction comparison point is the hut's overlay-first,
flag-second `DestroyBridge_*_MapInit` path.

**Phase 3 is OPTIONAL for the next pipeline step.** The brainstorm can
proceed with Phase-1 + Phase-2 findings; Phase 3's open questions are
refinements rather than blockers.

---

## Sources — Phase 2 additions

**Ghidra addresses decompiled in Phase 2:**
- `0x573540` — ProcessBridgeDestruction_High (full)
- `0x574000` — DestroyBridge_High_MapInit (full)
- `0x574C20` — DestroyBridge_Low_MapInit (full)
- `0x57F6A0` — RepairBridgeWalker_NS_Low (full)
- `0x57FBC0` — RepairBridgeWalker_EW_Low (full)
- `0x5800D0` — RepairBridgeWalker_NS_High (full)
- `0x580600` — RepairBridgeWalker_EW_High (full)
- `0x577920` — UnregisterBridgeRepairHut (full)
- `0x569760` — FUN_00569760 / low pavement walker (full)
- `0x598030` — FUN_00598030 / random pick with retry (full)
- `0x57BAA0` — DestroyBridge_Low (per-cell damage path) (full)
- Helpers via subagent: 0x42fcb0, 0x42f7c0, 0x485130, 0x485250, 0x586990,
  0x5868a0, 0x580b20, 0x580b70, 0x7c8b3d

**Phase 2 destination addresses identified (decompilation may be deferred
to Phase 3):**
- `0x5749C0` — DestroyBridgeFromCell_High
- `0x574780` — DestroyBridgeFromCell_Low
- `0x5746C0` — IsBridgeRampTile
- `0x574600` — IsLowBridgeEndpointTile (misnamed)
- `0x587180` — ApplyDamageToCell
- `0x57CCF0` — DestroyBridge_High (per-cell damage path)
- `0x576770` — UpdateAdjacentBridges_High
- `0x56C510` — UpdateBridgeZonesHelper (already documented)
- `0x568E40` — High pavement walker (FUN_00568e40)
- `0x41BF40` — TechnoClass::IsIronCurtainActive (vtable[0x160] target)
- `0x701F45` — TechnoClass::ReceiveDamage Crewed-survivor setter
  (inside the function at `0x701900` — approximate)

**Phase 2 xrefs queried:**
- `0x57F6A0..0x580600` (walker callers — all 4 confirmed exclusive
  to RepairBridge_Low/High)
- `0x577920` (UnregisterBridgeRepairHut callers — only FUN_007258d0)
- `0x574000`, `0x574C20` (DestroyBridge_*_MapInit callers — re-confirmed
  vs Phase 1)

**Byte-pattern searches (Phase 2):**
- `c6 ?? df 06 00 00 01` and `c6 ?? df 06 00 00 00` across all 6
  ModRM register encodings for setters/clearers of `+0x6DF`
- (Phase 3 may add: `c6 ?? 51 15 00 00 ??` for setters of
  `TechnoTypeClass+0x1551` (Crewed=yes) to cross-check the dual-purpose
  finding)

---

## Sources - 2026-05-17 Phase 3 extension

**Ghidra addresses decompiled or re-decompiled:**
- `0x0051E3B0` - `InfantryClass::What_Action_OnObject` C4 action gate
- `0x00519630` - `InfantryClass::PerCellProcess` mission `0x11` C4 plant
- `0x0043FB20` - `BuildingClass::Update` C4 timer / CABHUT branch
- `0x00574000` - `MapClass::DestroyBridge_High_MapInit`
- `0x00574C20` - `MapClass::DestroyBridge_Low_MapInit`
- `0x005749C0` - `MapClass::DestroyBridgeFromCell_High`
- `0x00574780` - `MapClass::DestroyBridgeFromCell_Low`
- `0x00575220` - `MapClass::CollapseBridge_EW_Low`
- `0x00575540` - `MapClass::CollapseBridge_NS_Low`
- `0x00575870` - `MapClass::CollapseBridge_EW_High`
- `0x00575BA0` - `MapClass::CollapseBridge_NS_High`
- `0x0057BAA0` - `DestroyBridge_Low`
- `0x0057CCF0` - `DestroyBridge_High`
- `0x0057BCF0` - `MapClass::DestroyBridgeWalker_NS_Low`
- `0x0057C2B0` - `MapClass::DestroyBridgeWalker_EW_Low`
- `0x0057CF60` - `MapClass::DestroyBridgeWalker_NS_High`
- `0x0057D530` - `MapClass::DestroyBridgeWalker_EW_High`
- `0x0057DD50` - `MapClass::ApplyBridgeDestruction_NS_Low`
- `0x0057E2A0` - `MapClass::ApplyBridgeDestruction_EW_Low`
- `0x0057E860` - `MapClass::ApplyBridgeDestruction_NS_High`
- `0x0057EDB0` - `MapClass::ApplyBridgeDestruction_EW_High`

**Callee queries:** `0x005749C0`, `0x00574780`, `0x00575220`,
`0x00575540`, `0x0057BAA0`, `0x0057CCF0`, `0x0057BCF0`,
`0x0057C2B0`, `0x0057DD50`, and `0x0057E2A0`.
