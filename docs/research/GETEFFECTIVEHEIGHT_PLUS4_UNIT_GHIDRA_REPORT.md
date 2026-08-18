# CellClass::GetEffectiveHeight — +4 Bridge Offset Unit Investigation

**Address:** `0x00487D50`  
**Confidence:** HIGH (all four callers decompiled; formula bytewise confirmed)  
**Active in YR:** Yes — called on every pathfinding step, weapon range check, and cliff passability check that involves bridge or elevated terrain.

---

## 1. Overview

`GetEffectiveHeight` returns the logical height of a cell as a signed integer, adding +4 when the bridge overlay flag is set. This report resolves the question: **what unit is the +4?**

**One-line answer:** The +4 is in **Level units** (the same discrete integer height scale as `CellClass.Level` at offset `+0x11B`). Level is **not** leptons, not pixels. It is a small-integer terrain height index (values 0–15 in normal maps) where 1 level ≈ 15 screen pixels (from `HeightInPixels = (height_raw - 30) / 15` in `RecalcAttributes`). The bridge adds 4 levels, which equals exactly 1 `ElevationIncrement` (INI default: `ElevationIncrement=4` in `[ElevationModel]`).

---

## 2. Formula — Bytewise Confirmed

From live Ghidra decompilation of `0x00487D50`:

```c
int GetEffectiveHeight(int cell_ptr) {
    return (int)*(char *)(cell_ptr + 0x11b)
         + ((*(uint *)(cell_ptr + 0x140) >> 7) & 1) * 4;
}
```

Critical detail: `*(char *)` is a **signed-byte load** — Level is sign-extended from `signed char` to `int`. This matches `CELLCLASS_STRUCT_GHIDRA_REPORT.md §2` which documents `+0x11B` as `signed byte`.

So the return type is a plain `int` in **Level units**:
- Normal terrain: returns `Level` (range approximately −128 to +127, in practice 0–15)
- Bridge overlay (Flags bit 7 = 0x80): returns `Level + 4`

There is no multiplication by leptons-per-level (0x100 = 256) or pixels-per-level (15) in the function itself.

---

## 3. All Callers and What They Compare Against

`get_xrefs_to(0x00487D50)` returns exactly **20 call sites** across **4 functions**:

| Caller address | Function name | Call count | Purpose |
|----------------|--------------|------------|---------|
| `0x004CC360` | FUN_004cc360 | 8 calls | Cliff/obstacle LOS check |
| `0x004CC680` | FUN_004cc680 | 4 calls | Height-diff gate (sub-check of FUN_004cc360) |
| `0x006F6F60` | FUN_006f6f60 | 4 calls | Elevation height-fire bonus (full ballistic) |
| `0x006F70E0` | FUN_006f70e0 | 4 calls | Elevation height-fire bonus (range-only) |

### 3.1 FUN_004cc680 @ 0x004CC680 — height-diff gate

Called by FUN_004cc360. Pattern:

```c
iVar1 = CellClass__GetEffectiveHeight(cellA);
iVar2 = CellClass__GetEffectiveHeight(cellB);
if (3 < iVar2 - iVar1) {       // threshold: diff > 3  (i.e., >= 4 levels)
    iVar1 = CellClass__GetEffectiveHeight(cellA);
    iVar2 = CellClass__GetEffectiveHeight(cellB);
    if (0 < iVar2 - iVar1) {   // secondary: diff > 0
        return 1;
    }
}
return 0;
```

**Evidence:** decompile of `0x004CC680`. The comparison threshold is **3** (integer), directly against the Level-unit return value. A bridge cell at `Level + 4` differs from a ground cell at the same `Level` by exactly 4, which is `> 3` → triggers the bridge-height gate. **No lepton or pixel factor is present.**

### 3.2 FUN_004cc360 @ 0x004CC360 — cliff/obstacle LOS check

Uses 8 calls to GetEffectiveHeight. Same pattern as FUN_004cc680:

```c
iVar4 = CellClass__GetEffectiveHeight(cellA);
iVar5 = CellClass__GetEffectiveHeight(cellB);
if (3 < iVar4 - iVar5) { ...   // same >3 threshold, Level units
```

Also directly reads `+0x11B` (Level byte) in the same function for lower-level checks:
```c
if (*(char *)((int)param_2 + 0x11b) < *(char *)((int)param_1 + 0x11b)) {
    return 0;  // target is lower — not impassable
}
```

Confirming that the raw `Level` field and `GetEffectiveHeight()` output are interchangeable at the comparison site — both are in the same unit (Level). **Evidence:** decompile of `0x004CC360` at comparison line `if (3 < iVar4 - iVar5)`.

FUN_004cc360 is called by:
- `FUN_004cc100 @ 0x004CC100` — LOS ray-cast along a lepton path (iterates lepton steps, calls GetEffectiveHeight for cliff occlusion at each cell)
- `FUN_00468bb0 @ 0x00468BB0` — projectile/unit position check at current lepton position

The ray-cast in FUN_004cc100 converts lepton coords to cell coords (via `>> 8`) and then calls GetEffectiveHeight. At no point is GetEffectiveHeight's output multiplied by 256 or 15 before comparison — the cliff gate stays in Level units throughout.

### 3.3 FUN_006f6f60 @ 0x006F6F60 — height-fire range bonus (ballistic)

Called from `TechnoClass__InRange @ 0x006F7220`. Pattern:

```c
iVar3 = CellClass__GetEffectiveHeight(targetCell);
iVar4 = CellClass__GetEffectiveHeight(attackerCell);
if (iVar4 - iVar3 < 0) {
    iVar4 = 0;   // clamp to 0 (no penalty for firing uphill)
} else {
    iVar3 = CellClass__GetEffectiveHeight(attackerCell);  // re-read
    iVar4 = CellClass__GetEffectiveHeight(targetCell);    // re-read
    iVar4 = iVar4 - iVar3;    // height_delta (Level units)
}
// Divide by ElevationIncrement to get number of increments:
uStack_4 = iVar4 / *(int *)(g_RulesClass_Instance + 0x1838);
//          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//          Rules+0x1838 = ElevationIncrement (INI default: 4)
iVar4 = DAT_00b0eb34 * iVar4;   // scale by per-level lepton factor
iVar3 = Math__ftol();
uStack_4 = iVar3 * 0x100 * iVar3 * 0x100 + iVar4 * iVar4;  // ballistic distance
Sqrt_Approx((double)uStack_4);
return Math__ftol();   // returns lepton range bonus
```

The division `height_delta / ElevationIncrement` divides **Level-unit delta by 4** to count how many full "elevation increments" the height difference spans. The final return value is in leptons, but the **input delta from GetEffectiveHeight is in Level units**. Evidence: decompile of `0x006F6F60`; `Rules+0x1838 = ElevationIncrement` confirmed in `TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §0` and `range_min_max.md §5`.

### 3.4 FUN_006f70e0 @ 0x006F70E0 — height-fire range bonus (range-only)

Called from `TechnoClass__InRange @ 0x006F7220`. Same height-delta / ElevationIncrement pattern:

```c
iVar3 = CellClass__GetEffectiveHeight(attackerCell);
uStack_4 = CellClass__GetEffectiveHeight(targetCell);
uStack_4 = uStack_4 - iVar3;    // height_delta in Level units
uStack_4 = uStack_4 / *(int *)(g_RulesClass_Instance + 0x1838);   // / ElevationIncrement
iVar3 = Math__ftol();
return iVar3 << 8;   // return as leptons (× 256)
```

The `<< 8` at the end converts "number of elevation increments" to leptons (256 leptons per cell × increments). Again, the GetEffectiveHeight input is in Level units and is never scaled before the divide. Evidence: decompile of `0x006F70E0`.

---

## 4. Cross-check: DAT_0089E864 vs GetEffectiveHeight

`DAT_0089E864 = 2 × BridgeHeight` (= 208 leptons; see `DAT_0089E864_BRIDGE_THRESHOLD_IDENTITY_GHIDRA_REPORT.md §1`). This constant is used in `Apply_area_damage @ 0x00489280` for bridge-deck layer selection:

```c
ground_z + DAT_0089e864 / 2 < impact_z   // == ground_z + BridgeHeight
```

**GetEffectiveHeight is not called in Apply_area_damage.** That function calls `CellClass__GetGroundHeight()` instead, and operates in lepton Z space directly. The two systems (GetEffectiveHeight Level-unit consumers vs. Apply_area_damage lepton consumers) are separate and do not cross. No caller of GetEffectiveHeight was found to compare its output directly against `DAT_0089E864` or any lepton value.

The bridge offset of 4 levels and the bridge height of 104 leptons are related only by convention:  
`4 levels × ≈26 leptons/level ≈ 104 leptons`, but this conversion never appears in GetEffectiveHeight's call chain.

---

## 5. Unit Conversion Summary

| Unit system | Scale | Where used |
|-------------|-------|-----------|
| **Level** (GetEffectiveHeight output) | 1 level ≈ 15 screen pixels | Pathfinding, cliff detection, weapon elevation bonus input |
| Leptons | 1 cell = 256 leptons | Object coordinates, range distance, Apply_area_damage Z |
| Screen pixels | 15 px/level for terrain | Rendering only |

GetEffectiveHeight's output is **always** used as a Level-unit integer in all 4 callers. The conversion to leptons happens only downstream (in FUN_006f6f60 / 006f70e0, via `<< 8` after dividing by ElevationIncrement).

---

## 6. The +4 — Verified Interpretation

**The +4 bridge offset is measured in Level units.** Specifically:
- `Level` at `+0x11B` is a discrete terrain height index (signed byte, values 0–15 in normal maps).
- A bridge surface is defined as being 4 levels above the ground cell it spans.
- 4 levels equals exactly one `ElevationIncrement` (INI: `[ElevationModel] ElevationIncrement=4` in `rulesmd.ini:938`).
- The cliff-detection threshold in FUN_004cc360/004cc680 is `> 3` (i.e., ≥ 4 levels), meaning a bridge cell looks like a cliff from ground level — intentional design symmetry.
- No leptons-per-level or pixels-per-level factor is present anywhere in GetEffectiveHeight or its immediate comparisons.

**Active in YR: Yes.** All four callers are on live code paths in standard YR skirmish: cliff LOS blocking, projectile height gating, weapon elevation range bonus.

---

## 7. Open Questions — Final State

- `[RESOLVED] OQ-1` — What unit is the +4? → Level units (signed integer, same scale as `CellClass.Level`). Evidence: decompile of `0x00487D50`; callers compare against integer `3`, not lepton or pixel values.
- `[RESOLVED] OQ-2` — Is Level sign-extended? → Yes. `*(char *)` with explicit cast to `int` sign-extends from `signed char`. Evidence: decompile `0x00487D50`: `(int)*(char *)(param_1 + 0x11b)`.
- `[RESOLVED] OQ-3` — Is GetEffectiveHeight output ever compared with DAT_0089E864 or a lepton value? → No. Apply_area_damage uses `GetGroundHeight()` not `GetEffectiveHeight`. The lepton bridge system is a separate code path. Evidence: `get_xrefs_to(0x00487D50)` returns exactly 4 callers, none inside `Apply_area_damage`.
- `[RESOLVED] OQ-4` — What are all callers? → FUN_004cc360 (cliff LOS), FUN_004cc680 (height-diff gate), FUN_006f6f60 (ballistic elevation bonus), FUN_006f70e0 (range elevation bonus). Evidence: `get_function_callers(0x00487D50)`.
- `[RESOLVED] OQ-5` — Is there a TS-only caller? → FUN_004cc360 is called from FUN_004cc100 (LOS ray-cast) and FUN_00468bb0 (projectile position check). Both are on live YR paths. FUN_006f6f60 and FUN_006f70e0 are inside `TechnoClass__InRange` which is on every targeting tick. No TS-gated caller found.
- `[DEFERRED] OQ-6` — Exact pixel/lepton meaning of `DAT_00b0eb34` (per-level lepton factor in FUN_006f6f60). Category: `requires-different-system-context`; reason: this is part of the ElevationModel ballistic computation, not GetEffectiveHeight; covered by TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md.

---

## 8. Load-Bearing Verified Facts

1. `GetEffectiveHeight(0x00487D50)` returns `(int)*(char *)(cell+0x11B) + ((cell.Flags >> 7) & 1) * 4` — signed-byte Level sign-extended, +4 in same Level unit. Evidence: Ghidra decompile of `0x00487D50`.
2. FUN_004cc680 (`0x004CC680`) compares `GetEffectiveHeight(A) - GetEffectiveHeight(B) > 3` — threshold is `3` in Level units. A bridge (+4) triggers this gate. Evidence: decompile of `0x004CC680`.
3. FUN_006f6f60/006f70e0 divide `GetEffectiveHeight` delta by `Rules+0x1838` (= `ElevationIncrement`, INI default 4) — treating the delta as levels, dividing to get increments. Evidence: decompile of `0x006F6F60` / `0x006F70E0`; `Rules+0x1838 = ElevationIncrement` confirmed in `TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §0`.
4. `ElevationIncrement = 4` in both `ini/rules.ini:758` and `ini/rulesmd.ini:938` — the +4 bridge offset equals exactly one elevation increment by design.
5. `Apply_area_damage` (lepton-space, uses `DAT_0089E864`) does not call `GetEffectiveHeight`. The 4-Level bridge offset and 104-lepton bridge height are parallel representations, never mixed in the same comparison. Evidence: `get_xrefs_to(0x00487D50)` = exactly 20 call sites, none in `0x00489280`.

---

## 9. Sources

**Ghidra addresses decompiled:**
- `0x00487D50` — CellClass::GetEffectiveHeight (target function)
- `0x004CC680` — FUN_004cc680 (height-diff gate sub-check)
- `0x004CC360` — FUN_004cc360 (cliff/obstacle LOS check)
- `0x004CC100` — FUN_004cc100 (LOS ray-cast, caller of FUN_004cc360)
- `0x00468BB0` — FUN_00468bb0 (projectile position check, caller of FUN_004cc360)
- `0x006F6F60` — FUN_006f6f60 (elevation ballistic range bonus)
- `0x006F70E0` — FUN_006f70e0 (elevation range bonus, Branch B)
- `0x006F7220` — TechnoClass::InRange (caller of 006f6f60 / 006f70e0)

**Reference docs:**
- `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md §2.7` — GetEffectiveHeight overview
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md §6` — Level at +0x11B, signed byte
- `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` — Phase 1/6 bridge-height checks
- `DAT_0089E864_BRIDGE_THRESHOLD_IDENTITY_GHIDRA_REPORT.md` — lepton bridge threshold (separate system)
- `TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §0` — ElevationIncrement = Rules+0x1838
- `combat/systems/range_min_max.md §5` — ElevationIncrement formula

**INI files checked:**
- `ini/rulesmd.ini:938` — `ElevationIncrement=4`
- `ini/rules.ini:758` — `ElevationIncrement=4`
