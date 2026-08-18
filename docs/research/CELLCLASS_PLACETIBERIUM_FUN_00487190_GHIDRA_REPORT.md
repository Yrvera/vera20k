# CellClass::PlaceTiberium — Full Decompilation & Verification Report

**Date:** 2026-05-20
**Binary:** gamemd.exe (Yuri's Revenge)
**Target:** FUN_00487190 (`CellClass::PlaceTiberium`)
**Purpose:** Verify §10 pseudocode in ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md
**Active in YR:** Yes — called every spread and growth tick; also from BuildingClass destruction and crate pickup

---

## 1. Function Signature

```
undefined4 __thiscall CellClass__PlaceTiberium(int param_1, int param_2, int param_3)
```

- `param_1` (`this`) = CellClass pointer (direct byte offsets apply directly)
- `param_2` = TiberiumClass index (int, not pointer)
- `param_3` = density amount to add / initial density to set

Verified via `decompile_function 0x00487190`.

---

## 2. Full Verified Decompilation Walkthrough

### 2.1 Entry Guard: MaxDensity Check

```c
iVar13 = *(int *)(g_TiberiumClass_Array + param_2 * 4);  // load TiberiumClass* by index
if (*(int *)(iVar13 + 0xe4) <= param_3) {               // TiberiumClass+0xE4 = MaxDensity (12)
    return 0;
}
```

If `param_3 >= MaxDensity` (12), the function returns 0 immediately.
**This gate fires before either branch** — the density argument is rejected at entry, not per-path.

### 2.2 Branch Selector: `CanPlaceTiberium`

```c
cVar2 = CellClass__CanPlaceTiberium(iVar13);  // pass TiberiumClass*
if (cVar2 == '\0') {
    // Branch A: cell already has tiberium (or blocked) — GROW
} else {
    // Branch B: cell is free for new tiberium — GERMINATE
}
```

`CanPlaceTiberium` is called with the `TiberiumClass*` (not `this`). The return value gates the path.

---

### 2.3 Branch A: Grow Existing Ore (CanPlaceTiberium == false)

This path is entered when the cell already has a tiberium overlay or placement is blocked for some reason. There is a **multi-gate pre-check** before density is modified:

```c
// Gate 1: TiberiumGrowthEnabled flag
if (*(char *)(g_ScenarioClass_Instance + 0x34a6) == '\0') return 0;

// Gate 2: cell's overlay must map to a valid tiberium index
iVar5 = CellClass__OverlayToTiberiumIndex();
if (iVar5 == -1) return 0;

// Gate 3: cell must be flat (no slope/damage)
// CellClass+0x11C read as char — must be '\0'
if (*(char *)(param_1 + 0x11c) != '\0') return 0;

// Gate 4: current density must be < MaxDensity-1
if (*(int *)(iVar5 + 0xe4) + -1 <= (int)(uint)*(byte *)(param_1 + 0x11e)) return 0;

// Gate 5: TiberiumClass+0xB0 (GrowthPercentage, double) must be >= _DAT_007e3810 (0.0)
if (*(double *)(iVar5 + 0xb0) < _DAT_007e3810) return 0;

// Gate 6: overlay's tiberium index must match the requested param_2
iVar5 = CellClass__OverlayToTiberiumIndex();
if (iVar5 != param_2) return 0;
```

**DISCREPANCY vs §10:** §10 says Branch A is simply `CanPlaceTiberium == false` (cell already has tiberium). The actual code has **6 additional gates** including the `TiberiumGrowthEnabled` flag, a GrowthPercentage check, and a tib-type-match check. None of these are in the §10 pseudocode.

After all gates pass, density is modified:

```c
bVar3 = *(char *)(param_1 + 0x11e) + (char)param_3;   // read OverlayData, add param_3
*(byte *)(param_1 + 0x11e) = bVar3;                    // write back
iVar13 = *(int *)(iVar13 + 0xe4) + -1;                 // MaxDensity - 1 = 11
if (iVar13 <= (int)(uint)bVar3) {                       // clamp to 11
    bVar3 = (byte)iVar13;
}
*(byte *)(param_1 + 0x11e) = bVar3;                    // write clamped value
```

Then a screen rect is dirtied (viewport calculation), and:

```c
TiberiumClass__AddToSpreadQueue(param_1 + 0x24);       // param_1+0x24 = cell's MapCoord
return 1;
```

**Branch A writes: CellClass+0x11E (OverlayData), then calls AddToSpreadQueue.**

No `RecalcAttributes`, no `RadarClass::MarkTerrainDirty` in Branch A.

---

### 2.4 Branch B: Germinate New Ore (CanPlaceTiberium == true)

#### Sub-branch B1: Flat cell (`CellClass+0x11C == '\0'`)

```c
pvVar4 = operator_new(0xb0);          // allocate OverlayClass (size 0xB0)
local_84 = *(int *)(param_1 + 0x24); // cell's MapCoord
iVar5 = Random__RandomRanged(0, 0xb); // random 0–11

// Select overlay type:
// TiberiumClass+0xE0 = Image (OverlayTypeClass*), +0x294 = ArrayIndex
uVar6 = *(undefined4 *)(g_OverlayTypeClass_Array +
         (*(int *)(*(int *)(iVar13 + 0xe0) + 0x294) + iVar5) * 4);
```

**Confirmed: `Random__RandomRanged(0, 0xb)` = range [0, 11]. Matches §10.**

The overlay array index is: `TibImage->ArrayIndex + random_variant`.

#### Sub-branch B2: Sloped cell (`CellClass+0x11C != '\0'`)

```c
pvVar4 = operator_new(0xb0);
local_84 = *(int *)(param_1 + 0x24);
iVar5 = Random__RandomRanged(0, 1);   // random 0 or 1

uVar6 = *(undefined4 *)(g_OverlayTypeClass_Array + -8 +
         (*(int *)(*(int *)(iVar13 + 0xe0) + 0x294)    // Image->ArrayIndex
          + (uint)*(byte *)(param_1 + 0x11c) * 2       // slopeIdx * 2
          + *(int *)(iVar13 + 0xe8)                    // TibClass+0xE8 = NumImages (12)
          + iVar5) * 4);                               // + random 0..1
```

Formula: `ArrayIndex + NumImages + (slopeIdx * 2) + random - 2`
The `-8` in the pointer arithmetic = `-2` elements in the 4-byte array (i.e. `-2 * 4 = -8`).
**Matches §10 exactly.**

#### Common tail of Branch B

After either sub-branch constructs the OverlayClass:

```c
OverlayClass__Constructor(uVar6, &local_84, 0xffffffff);
// Falls through to LAB_00487291 (whether allocation succeeded or not!)

LAB_00487291:
TiberiumClass__AddToGrowthQueue(param_1 + 0x24);  // add cell to growth queue
*(char *)(param_1 + 0x11e) = (char)param_3;       // write OverlayData = param_3
```

**DISCREPANCY vs §10:** §10 says "Add to growth queue, set OverlayData = densityAmount" with AddToGrowthQueue = `FUN_007235a0`. The actual add-to-growth-queue function is **`TiberiumClass__AddToGrowthQueue` at `0x007235a0`** — confirmed by `decompile_function 0x007235a0` which shows it manipulating `TiberiumClass+0x110` (GrowthQueue heap) and `TiberiumClass+0x10C` (GrowthQueue count). Address matches §10.

After setting OverlayData, there is a screen rect dirty + RadarClass call:

```c
RadarClass__MarkTerrainDirty(param_1 + 0x24);
return 1;
```

**Branch B writes: CellClass+0x11E (OverlayData = param_3), calls AddToGrowthQueue, calls RadarClass::MarkTerrainDirty.**

---

## 3. Field Accesses — Complete Table

Verified via `decompile_function 0x00487190`.

| Offset | Field | Read/Write | Branch | Notes |
|--------|-------|-----------|--------|-------|
| `param_1+0x24` | MapCoord (short[2]) | Read | Both | Passed to queue helpers and OverlayClass ctor |
| `param_1+0x11C` | SlopeIndex (byte, read as char) | Read | Both | `== '\0'` = flat; `!= '\0'` = sloped. Also used as slope variant index in B2 |
| `param_1+0x11E` | OverlayData (density, byte) | Read+Write | A | Read to check current density, written after add+clamp |
| `param_1+0x11E` | OverlayData (density, byte) | Write | B | Written with `param_3` after OverlayClass ctor |
| `param_1+0x44` | OverlayTypeIndex | NOT directly written | — | Written implicitly by `OverlayClass::Constructor` stamping into the cell |
| `TibClass+0xE4` | MaxDensity (int, = 12) | Read | Entry + A | Gate and clamp |
| `TibClass+0xE0` | Image (OverlayTypeClass*) | Read | B | Dereferenced to +0x294 (ArrayIndex) |
| `TibClass+0xE8` | NumImages (int, = 12) | Read | B2 | Sloped variant formula |
| `TibClass+0xB0` | GrowthPercentage (double) | Read | A | Must be >= 0.0 (gate 5) |
| `ScenarioClass+0x34A6` | TiberiumGrowthEnabled (bool) | Read | A | Gate 1 — only in Branch A |

### Fields NOT directly written by PlaceTiberium

- **CellClass+0x140 Flags bit 7 (0x80):** NOT written by PlaceTiberium. The bit is set by `OverlayClass::Constructor` or `CellClass::RecalcAttributes`, not directly here.
- **CellClass+0xEC LandType:** NOT written by PlaceTiberium. Set by `CellClass::RecalcAttributes` when it reads OverlayTypeClass+0x298. RecalcAttributes is NOT called by PlaceTiberium directly.

Verified: the decompilation of `CellClass__RecalcAttributes` at `0x0047d2b0` (via `decompile_function 0x0047d2b0`) shows: when `OverlayToTiberiumIndex != -1` and `SlopeIndex < 5` and `OverlayTypeClass+0x298 == 0`, it sets `this->LandType = 5`. This is the actual site for the LandType=5 write.

**DISCREPANCY vs §10 / prior doc:** §10 lists "CellClass+0x140 bit 7" and "CellClass+0xEC LandType" as fields PlaceTiberium writes. They are NOT directly written by FUN_00487190. They are set by RecalcAttributes (which is called elsewhere after placement). PlaceTiberium does NOT call RecalcAttributes.

---

## 4. Density Argument Behavior

| Branch | How param_3 is used |
|--------|-------------------|
| Branch A (grow) | Added to current OverlayData (`cell->OverlayData += param_3`), then clamped to MaxDensity-1 (11) |
| Branch B (germinate) | Directly assigned: `cell->OverlayData = param_3` |

Clamping in Branch A is a two-step write: first write the unclamped sum, then check and write again if over limit. Branch B has no clamp — callers are expected to pass a valid value.

---

## 5. RecalcAttributes and RadarMarkDirty

| Call | Branch A | Branch B |
|------|---------|---------|
| `CellClass::RecalcAttributes` | NOT called | NOT called |
| `RadarClass::MarkTerrainDirty` | NOT called | Called (`RadarClass__MarkTerrainDirty(param_1 + 0x24)`) |
| `TacticalClass::DirtyScreenRect` | Called | Called |

**DISCREPANCY vs §10:** §10 does not mention `RadarClass::MarkTerrainDirty`. It is only called in Branch B (germinate), not Branch A (grow). `RecalcAttributes` is not called at all by PlaceTiberium — the LandType and flag updates happen through the OverlayClass constructor chain or via external RecalcAttributes calls from other code.

---

## 6. Caller Table

Verified via `get_function_callers 0x00487190` and `get_xrefs_to 0x00487190`.

| Caller Address | Named Function | density passed | Context |
|----------------|----------------|----------------|---------|
| `0x00483775` | `CellClass__GrowTiberium` | `1` | Growth queue tick — grow density by 1 |
| `0x004838c5` | `CellClass__SpreadTiberium` | `3` | Spread queue tick — germinate neighbor at density 3 |
| `0x00441bed` | `BuildingClass__DestructionEffects` | unknown | Building destruction — scatters tiberium on destruction |
| `0x00481ed8` | (within `CrateClass__PickupDispatch`) | `1` | Tiberium crate pickup — places 1 level on the crate cell |
| `0x00481f60` | (within `CrateClass__PickupDispatch`) | from `CONCAT44(1,iVar3)` | Tiberium crate — places 10–20 random cells around crate |

There is **no direct call from a map-load seeding function** at `FUN_004818e0`. That address (`0x004818e0`) decompiles to `CellClass::SpreadCellGerminate`, not a map-loader. Map-load tiberium comes from the `[OverlayPack]`/`[OverlayDataPack]` data parsed directly into cell arrays without calling PlaceTiberium.

The claim in §10 context that `FUN_004818e0` is the "map-load seeding" caller is incorrect. Verified: `decompile_function 0x004818e0` shows `CellClass__SpreadCellGerminate`, a spread-queue post-germination step.

---

## 7. Queue Helper Address Confirmation

| Helper | Called As | Address | Confirmed |
|--------|-----------|---------|-----------|
| AddToGrowthQueue | `TiberiumClass__AddToGrowthQueue(param_1 + 0x24)` | `0x007235a0` | Yes — decompiled; manipulates `TibClass+0x110` heap and `+0x10C` count |
| AddToSpreadQueue | `TiberiumClass__AddToSpreadQueue(param_1 + 0x24)` | `0x00722af0` | Yes — decompiled; manipulates `TibClass+0xF4` heap and `+0xF0` count |

Both helpers use `Random__Next() % 50` for jitter (0–49 frames added to current frame counter). Confirmed by decompilation of both.

§10 had the addresses **reversed**: it listed `FUN_007235a0` for growth queue (Branch B new ore) and `FUN_00722af0` for spread queue (Branch A grow). This matches the actual binary — the naming is correct, just the §10 prose was slightly ambiguous about which branch calls which.

---

## 8. Discrepancies vs §10 Pseudocode

| # | §10 Claim | Actual Binary | Severity |
|---|-----------|--------------|---------|
| 1 | Branch A has no guard beyond `CanPlaceTiberium == false` | Branch A has **6 gates**: TiberiumGrowthEnabled, valid overlay tib index, flat cell, density < max-1, GrowthPercentage >= 0, tib-type match | HIGH — the TiberiumGrowthEnabled gate means growth is completely suppressed when the flag is off |
| 2 | `RadarClass::MarkTerrainDirty` not mentioned | It IS called — in Branch B only | MEDIUM — affects minimap update on new ore placement |
| 3 | `RecalcAttributes` — not mentioned, implied absent | Confirmed absent from PlaceTiberium | Matches existing doc — no discrepancy |
| 4 | CellClass+0x140 bit 7 and +0xEC LandType listed as written by PlaceTiberium | NOT written directly; set by OverlayClass ctor / RecalcAttributes elsewhere | HIGH — the doc table is wrong about who sets these fields |
| 5 | `FUN_004818e0` cited as map-load seed caller | That address is `CellClass__SpreadCellGerminate`, not a map loader | LOW — caller table note only, no code impact |
| 6 | Branch A path: "CanPlaceTiberium is FALSE" = "cell already has tiberium" | Correct — but the 6 early-return gates mean PlaceTiberium in grow mode is NOT equivalent to "just grow any tib cell" | MEDIUM |

---

## 9. Active in YR

- **Yes, unconditionally active.** Both spread and growth paths call PlaceTiberium every ore tick.
- Branch A's TiberiumGrowthEnabled gate (`ScenarioClass+0x34A6`) uses the same flag documented in §6 of the main report. In standard YR skirmish this is true.
- The GrowthPercentage gate (`TibClass+0xB0`) filters out Cruentus (gems) which have `GrowthPercentage=0` — confirmed by INI data.

---

## 10. Summary

FUN_00487190 is a two-path function: germinate (new ore) or grow (existing ore). The §10 pseudocode is broadly correct about the two paths, queue addresses, and random-variant range, but has three significant errors:

1. Branch A has 6 pre-flight guards not in §10, most importantly `TiberiumGrowthEnabled` and a GrowthPercentage > 0 requirement (which suppresses gem growth).
2. `RadarClass::MarkTerrainDirty` is called in Branch B but missing from §10.
3. CellClass+0x140 and +0xEC are NOT written by PlaceTiberium — those are set by RecalcAttributes, called from outside this function.
