# Cell_ComputeZAdjust — Ghidra Research Report

**Primary address:** `0x00484680`  
**Supporting function:** `FUN_00483e30` @ `0x00483e30` (ZAdjust initializer / LightConvert setter)  
**Called by batch wrapper:** `FUN_004ae4c0` @ `0x004ae4c0`  
**Confidence:** HIGH (all formulas verified from binary)  
**Active in YR:** Yes — unconditional, runs on every playfield cell during LightningStorm / PsychicDominator transitions and on initial map load

---

## 1. Overview

`Cell_ComputeZAdjust` computes four per-cell Z-sort bias values that renderers use
as the `z_adjust` argument to `CC_Draw_Shape` and `TMP_TileBlitter`. Two fields
(`+0x10A ZAdjust_Ground`, `+0x10E ZAdjust_Bridge`) are unscaled pixel-bias values;
two (`+0x10C ZAdjust_GroundScaled`, `+0x110 ZAdjust_BridgeScaled`) are their
scale-multiplied counterparts. The function does NOT write `+0x104`, `+0x108`,
`+0x112`, or `+0x114` — those are owned by `FUN_00483e30`.

The function selects from four ScenarioClass ZHeight tables depending on which
superweapon (if any) is currently firing, then computes:

```
ZAdjust_Ground  = Base + (PerLevelStep * Level  - Offset)
ZAdjust_Bridge  = Base + (PerLevelStep * (Level+4) - Offset)
```

Both values are then clamped to `[0, 2000]` (inclusive lower, exclusive upper
— the clamp uses `> 1999` for the upper bound, making 2000 reachable only when
the result would exceed 1999). Then:

```
ZAdjust_GroundScaled = (ZAdjust_Ground * Scale) >> 16
ZAdjust_BridgeScaled = (ZAdjust_Bridge  * Scale) >> 16
```

Both scaled values are also clamped to `[0, 2000]`.

---

## 2. Exact Formula (with binary evidence)

### Step 1 — Compute ZAdjust_Base from ScenarioClass

`iVar4 = *(int*)(g_ScenarioClass_Instance + 0x352C) * 1000`

The result is a 32-bit integer. It is then converted to a 16-bit value via
the floor-mod-100 idiom:

```
// Ghidra pseudocode, verified at 0x004846A5-0x004846C0
sVar5 = (short)(iVar4 / 100)
      + (short)(iVar4 >> 31)              // sign-extend for rounding
      - (short)((longlong)iVar4 * 0x51EB851F >> 63)  // mod-100 subtract
      + *(short*)(param_1 + 0x108)        // add ZAdjust_Base from cell
```

This is the **modulo-100** of `ScenarioClass[+0x352C] * 1000`, added to the
cell's stored `ZAdjust_Base` (`+0x108`).

The result `sVar5` is written immediately to **both** `+0x10A` and `+0x10E`
as their starting value:

```c
*(short*)(param_1 + 0x10A) = sVar5;  // ZAdjust_Ground
*(short*)(param_1 + 0x10E) = sVar5;  // ZAdjust_Bridge (same initial value)
```

### Step 2 — Select ZHeight table (superweapon branch)

Three global flag functions are evaluated in cascade:

| Priority | Function | Global | Condition | Active in YR |
|----------|----------|--------|-----------|--------------|
| 1 (highest) | `FUN_0053A100` @ `0x0053A100` | `DAT_00A9FAB4` | `!= 0` → PsychicDominator firing | Yes — fires when PD superweapon is active |
| 2 | `FUN_0053B400` @ `0x0053B400` | `DAT_00A9FAC0` | `!= 0` → PsychicDominator intensity counter | Yes — intermediate PD phase |
| 3 | `FUN_0053A110` @ `0x0053A110` | `DAT_00A9FABC == 1` | LightningStorm active | Yes — fires when LightningStorm superweapon is active |
| 4 (default) | none | — | Normal gameplay | Yes — applies in all normal play |

The selected ScenarioClass offsets for each branch (verified from decompile):

| Branch | PerLevelStep offset | Offset offset |
|--------|---------------------|---------------|
| PsychicDominator full (`FUN_0053A100 != 0`) | `+0x355C` | `+0x3558` |
| PsychicDominator intensity (`FUN_0053B400 != 0`) | `+0x3590` | `+0x358C` |
| LightningStorm (`FUN_0053A110 == 1`) | `+0x3574` | `+0x3570` |
| Normal game | `+0x3544` | `+0x3540` |

### Step 3 — Add Level-based height bias

Let `S` = `*(short*)(g_ScenarioClass_Instance + PerLevelStep_offset)`,
`O` = `*(short*)(g_ScenarioClass_Instance + Offset_offset)`,
`L` = `*(signed char*)(param_1 + 0x11B)` (CellClass Level, read as **signed char**).

```
// ZAdjust_Ground (verified at corresponding branch in 0x00484680)
*(short*)(param_1 + 0x10A) += (S * L) - O

// ZAdjust_Bridge — computed locally as sVar5 (not yet written):
sVar5 = (S * (L + 4)) - O
```

**Critical detail:** `Level` is read as `signed char` (`*(char*)(param_1 + 0x11B)`).
This means negative Level values (underground/subterranean cells, if any) produce
negative contributions. In practice Level is 0–14 for visible terrain.

**Critical detail:** The Bridge variant uses `(L + 4)` not `L`. The +4 matches
`GetEffectiveHeight` (`0x00487D50`) which adds exactly 4 to Level when flag 0x80
(HasBridgeOverlay) is set. This means `ZAdjust_Bridge` represents the Z-adjust
for an object sitting *on top of a bridge* over this cell.

After the branch:
```c
*(short*)(param_1 + 0x10E) += sVar5;  // ZAdjust_Bridge finalized
```

### Step 4 — Apply scale multiplier and compute Scaled variants

```c
*(short*)(param_1 + 0x10C) = (short)((int)(*(short*)(param_1+0x10A)) * *(int*)(param_1+0x104)) >> 16)
*(short*)(param_1 + 0x10E) = (short)((int)(*(short*)(param_1+0x10E)) * *(int*)(param_1+0x104)) >> 16)
```

`param_1 + 0x104` = `ZAdjust_Scale` (int32, default 0x10000 = 1.0 in 16.16 fixed-point).
The `>> 16` with `(short)` cast is a 16.16 fixed-point multiply: result = `(A * Scale) / 65536`.
At the default scale of `0x10000`, `ZAdjust_GroundScaled == ZAdjust_Ground` exactly.

### Step 5 — Clamp all four values to [0, 2000]

```c
// Upper clamp (verified at 0x004847D0-0x004847F0):
if (ZAdjust_Ground  > 1999) ZAdjust_Ground  = 2000;   // +0x10A
if (ZAdjust_GroundScaled > 1999) ZAdjust_GroundScaled = 2000;  // +0x10C
if (ZAdjust_Bridge  > 1999) ZAdjust_Bridge  = 2000;   // +0x10E

// Lower clamp (verified at 0x00484800-0x00484830):
if (ZAdjust_Ground  < 1) ZAdjust_Ground  = 0;
if (ZAdjust_GroundScaled < 1) ZAdjust_GroundScaled = 0;
if (ZAdjust_Bridge  < 1) ZAdjust_Bridge  = 0;   // written as undefined2=0
```

**Critical detail:** Upper clamp threshold is `> 1999` (strictly greater), so value
2000 is achievable when input would have been 2000 or more. Lower clamp threshold
is `< 1` (strictly less than 1), so 0 results only when value ≤ 0; value 1 is NOT
zeroed out.

**Fields NOT written by Cell_ComputeZAdjust:** `+0x104` (Scale), `+0x108` (Base),
`+0x112` (ZAdjust_5), `+0x114` (ZAdjust_6). These 4 are managed exclusively by
`FUN_00483e30`.

---

## 3. Fields Written — Complete Picture

| Offset | Name | Written by | Formula summary |
|--------|------|------------|-----------------|
| +0x104 | ZAdjust_Scale | FUN_00483e30 only | 16.16 fixed-point multiplier, default 0x10000 |
| +0x108 | ZAdjust_Base | FUN_00483e30 only | Base pixel bias added before level calc |
| +0x10A | ZAdjust_Ground | Cell_ComputeZAdjust | `Base + (S*L - O)`, clamped [0,2000] |
| +0x10C | ZAdjust_GroundScaled | Cell_ComputeZAdjust | `(ZAdjust_Ground * Scale) >> 16`, clamped [0,2000] |
| +0x10E | ZAdjust_Bridge | Cell_ComputeZAdjust | `Base + (S*(L+4) - O)`, clamped [0,2000] |
| +0x110 | ZAdjust_BridgeScaled | FUN_00483e30 (param_2) | Written by FUN_00483e30 at LAB_00483F7A |
| +0x112 | ZAdjust_5 | FUN_00483e30 only | default 1000 |
| +0x114 | ZAdjust_6 | FUN_00483e30 only | default 1000 |

**CORRECTION to CELLCLASS_STRUCT_GHIDRA_REPORT:** `+0x110` is NOT written by
`Cell_ComputeZAdjust`. The function at `0x00484680` writes only `+0x10A`, `+0x10C`,
and `+0x10E`. The `+0x110` field is written at `LAB_00483F7A` in `FUN_00483e30`
(`*(short*)(param_1+0x110) = (undefined2)param_2`) where `param_2` holds the
post-light-source-accumulation Z value.

---

## 4. Caller Chain and Invocation Context

```
LogicClass__PerTickUpdate (0x0055AFB0)
  └─ FUN_004AE4C0 (0x004AE4C0)          ← triggered when LightningStorm level changes
       └─ Cell_ComputeZAdjust (0x00484680)  ← called for EVERY playfield cell

FUN_0053AD00 (0x0053AD00)               ← LightConvert propagation dispatcher
  └─ FUN_004AE4C0                       ← same batch: recomputes all cells
       └─ Cell_ComputeZAdjust

LogicClass__PerTickUpdate               ← ALSO calls FUN_004AE4C0 directly
  (from 0x0055B4C6, inside LightningStorm level counter update block)
```

`FUN_004AE4C0` iterates all cells via `MapClass__CellIterator_Init` /
`MapClass__CellIterator_Next` and calls `Cell_ComputeZAdjust` on each.

**This function is NOT called at map load.** Map load uses `FUN_004AE450` which
calls `FUN_00483e30` (not `Cell_ComputeZAdjust`) on every cell — that is the
**initialization pass** that writes all 8 ZAdjust fields to their initial values.
`Cell_ComputeZAdjust` runs only on **per-tick updates** triggered by active
superweapons.

**Active in YR: Conditional** — runs only when:
- LightningStorm is active (`DAT_00A9FABC == 1`), OR
- PsychicDominator is firing (`DAT_00A9FAB4 != 0` or `DAT_00A9FAC0 != 0`)

In normal gameplay (no superweapons), ZAdjust values remain at the values set by
`FUN_00483e30` during initialization. `Cell_ComputeZAdjust` is never called.

---

## 5. Rendering Consumers (Verified)

### +0x10C (ZAdjust_GroundScaled) — consumed by TMP_TileBlitter and CC_Draw_Shape

**CellOverlay_TileDraw @ `0x00480350`** (verified at `0x00480431`):
```c
TMP_TileBlitter(..., (int)*(short*)(param_1 + 0x10C), 1, ...);
```
This is the terrain tile draw path. `+0x10C` is the Z-sort bias passed to the
tile blitter for **ground-level tiles**.

**CellClass__DrawOverlay_Body @ `0x0047F6A0`** (verified at `0x0047FA3E`, `0x0047FA8C`):
```c
iVar9 = (int)*(short*)(param_1 + 0x10C);   // used as z_adjust to CC_Draw_Shape
```
Used for overlays that are NOT flagged as bridge surface — wall overlays, ore
overlays on slopes (SlopeIndex != 0), gate overlays.

**TechnoClass_DrawSHP @ `0x00705F42`** (verified at `0x00705FCA`):
```c
param_8 = (int)*(short*)(iVar4 + 0x10C);
```
Used for building SHP drawing when building is flagged `field_0x1702 != 0`
(fire-port/garrison condition). Gets the Z-adjust from the building's current cell.

### +0x10A (ZAdjust_Ground, unscaled) — consumed by CC_Draw_Shape for bridge overlay pass

**CellClass__DrawOverlay_Body** (verified at `0x0047F9B4`):
```c
iVar9 = (int)*(short*)(param_1 + 0x10A);
```
This branch fires when overlay type has flag `+0x2A8` (isCrawlable/bridge-surface
overlay, Flags bit 7 = `0x80` set). Bridge surface overlays use the **unscaled**
ground Z-adjust.

### +0x10E (ZAdjust_Bridge) — consumed by CC_Draw_Shape for bridge objects

**CellClass__DrawOverlay_Body** (verified at `0x0047F8B4`):
```c
CC_Draw_Shape(..., (int)*(short*)(param_1 + 0x10E), ...);
```
This branch fires when `Flags & 0x80` (HasBridgeOverlay). Overlays drawn on cells
with bridge overlay get the **bridge** Z-adjust (Level+4 formula), making them
sort higher than ground-level objects.

---

## 6. Meaning of ZAdjust — Not a Sort Key, a Brightness/Depth Bias

The 6th argument to `CC_Draw_Shape` (the z_adjust parameter) is a **Z-sort depth
bias** used by the isometric renderer to adjust where in the Z-buffer a sprite
lands. Value 1000 = neutral (no modification); values above 1000 push the sprite
toward the viewer (draws on top); values below 1000 push it away. The formula
maps level 0 to a neutral value and shifts up/down with terrain height, so a unit
on a higher cell sorts correctly above one on a lower cell.

`ZAdjust_Ground` is used for objects/tiles sitting on the ground surface.
`ZAdjust_Bridge` (using `Level+4`) ensures objects on bridge surfaces sort above
the ground below them, consistent with the +4 level delta used throughout the
engine (see `GetEffectiveHeight @ 0x00487D50`).

---

## 7. FUN_00483e30 — The Initializer / Full ZAdjust Setter

Address: `0x00483E30`. Called at map load, during LightConvert cache invalidation,
and as a fallback guard before drawing when `param_1 + 0x34` (LightConvert pointer)
is null.

When called with `param_2 == 0` (no explicit LightConvert):
- Calls `FUN_00484180` to compute the full 8-field set including light source contributions
- Writes all 8 fields: `+0x104` through `+0x114`

When called with `param_2 == 0x10000` (default scale, dummy cell):
- Writes all ZAdjust fields to defaults: 1000, 1000, 1000, 1000, 1000, 1000 and Scale=0x10000, Base=0

`FUN_00484180` mirrors `Cell_ComputeZAdjust`'s formula but uses **int** operands
(not short) and incorporates local light source contributions from the
`DAT_00ABCA14` light object array before computing the ZHeight bias.

---

## 8. Open Questions — Final State

- `[RESOLVED]` Q1 — Does Cell_ComputeZAdjust write +0x110? → No. Only +0x10A, +0x10C, +0x10E. Evidence: full decompile at `0x00484680`.
- `[RESOLVED]` Q2 — Bridge variant is Level+4 (not some other offset)? → Yes, exactly +4. Evidence: `sVar5 = S * (*(char*)(param_1+0x11B) + 4) - O` at bridge branch in decompile.
- `[RESOLVED]` Q3 — What are the three flag functions? → PsychicDominator firing (DAT_00A9FAB4), PD intensity (DAT_00A9FAC0), LightningStorm (DAT_00A9FABC==1). Evidence: xrefs to those globals show PsychicDominator__Process writes/reads them.
- `[RESOLVED]` Q4 — Which renderers consume which ZAdjust field? → +0x10C: TMP_TileBlitter + CC_Draw_Shape for ground overlays/tiles; +0x10A: CC_Draw_Shape for bridge-surface overlays; +0x10E: CC_Draw_Shape for objects on bridge overlay cells. Evidence: decompiles of CellOverlay_TileDraw, CellClass__DrawOverlay_Body, TechnoClass_DrawSHP.
- `[RESOLVED]` Q5 — Is the function called at map load? → No. Map load uses FUN_004AE450 which calls FUN_00483e30. Cell_ComputeZAdjust is only called per-tick by FUN_004AE4C0. Evidence: xrefs to 0x00484680 show only one caller (FUN_004AE4C0).
- `[RESOLVED]` Q6 — What is the upper clamp value? → 2000 (threshold `> 1999`). Lower clamp: 0 (threshold `< 1`). Evidence: decompile lines at 0x004847D0–0x00484830.
- `[RESOLVED]` Q7 — How is Level read (signed or unsigned)? → Signed char `*(char*)(param_1+0x11B)`. Evidence: Ghidra cast `(short)*(char*)`.
- `[DEFERRED]` Q8 — What exact INI keys set the ScenarioClass ZHeight table values (+0x3540, +0x3558, +0x3570, +0x358C, +0x3544, +0x355C, +0x3574, +0x3590)? (category: `requires-different-system-context`; reason: requires reading ScenarioClass INI parser which is a separate large function; next-step: search for `ZHeightAdjust` or similar key in ReadScenario).
- `[DEFERRED]` Q9 — Does +0x110 (ZAdjust_BridgeScaled) have direct rendering consumers beyond FUN_00483e30 writes? (category: `bounded-cost-too-high`; reason: no function named for it appeared in function search; would require xref scan of every call site that reads cell+0x110 offset).

---

## Sources

### Ghidra functions decompiled
- `0x00484680` — Cell_ComputeZAdjust (primary target, full decompile)
- `0x004AE4C0` — batch iterator wrapper that calls Cell_ComputeZAdjust for all cells
- `0x00483E30` — FUN_00483e30 (full ZAdjust setter / LightConvert initializer)
- `0x00484180` — FUN_00484180 (8-field ZAdjust computer with light sources)
- `0x0053A100` — FUN_0053A100 (reads DAT_00A9FAB4 — PD firing flag)
- `0x0053B400` — FUN_0053B400 (reads DAT_00A9FAC0 — PD intensity counter)
- `0x0053A110` — FUN_0053A110 (reads DAT_00A9FABC == 1 — LightningStorm flag)
- `0x0055AFB0` — LogicClass__PerTickUpdate (caller context)
- `0x0053AD00` — FUN_0053AD00 (alternate caller via LightConvert propagation)
- `0x00480350` — CellOverlay_TileDraw (consumer of +0x10C via TMP_TileBlitter)
- `0x0047F6A0` — CellClass__DrawOverlay_Body (consumer of +0x10A, +0x10C, +0x10E)
- `0x0047F510` — CellClass__DrawOverlay_Shadow (does NOT read ZAdjust fields)
- `0x00705E00` — TechnoClass_DrawSHP (consumer of +0x10C for building SHP)
- `0x006D20E0` — Tactical__AdjustForZ (uses separate Z calculation, not cell ZAdjust)
- `0x00487D50` — GetEffectiveHeight (confirms +4 level delta for bridges)
- `0x00568CB9` — MapClass__InitCellAttributes (calls FUN_00483e30 at load, not Cell_ComputeZAdjust)

### Doc files referenced
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — base CellClass struct, field names/offsets confirmed
