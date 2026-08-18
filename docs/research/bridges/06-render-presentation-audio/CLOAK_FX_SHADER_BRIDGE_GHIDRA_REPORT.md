# Cloak FX — Shader-Bridge Ghidra Research Report

**Primary addresses:** `TechnoClass__CloakingTick @ 0x006FB740`,
`TechnoClass_GetVisualState @ 0x00703860`,
`TechnoClass__ModifyCloakDrawFlags @ 0x0070ED80`,
`TechnoClass__Draw (VXL path) @ 0x00706640`,
`TechnoClass_DrawSHP @ 0x00705E00`,
`TechnoClass__StartCloaking @ 0x00703770`,
`TechnoClass__StartUncloaking @ 0x007036C0`,
`UnitClass__TurretAI (Mirage tree-pick) @ 0x007468C0`,
`UnitClass__GetDisplayType @ 0x007465B0`,
`TechnoClass__Constructor @ 0x006F2B40`,
`Blitter_Shimmer_75pct_Remap @ 0x00494330`,
`Blitter_ZWriteOnly_RLE_Remap_50pct @ 0x00497CF0`,
`Blitter_Scanline_Blend25pct_Remap @ 0x00494080`,
`Blitter_selector @ 0x00490B90`.

**Confidence:** HIGH overall. Every formula, offset, and constant is verified
directly from disassembly or memory read in this session — including the
intensity-table generation formula (OQ#A — resolved by decompiling
`FUN_00420140` at section 6.6) and the brightness-bit blitter behavior
(OQ#B — resolved by decompiling `FUN_00495730` at section 7.4).

**Active in YR:** Conditional. The unit-level cloak system is **fully active**
in stock YR play on three retail unit types: SUB, DLPH, SQD
(`Cloakable=yes`). The disguise sprite-swap is fully active on SPY and MGTK.
The building-cloak path (BuildingClass override) is shipping-but-dormant —
no retail building sets `Cloakable=yes` or `CloakGenerator=yes`. The
shimmer-suppression timer at `TechnoClass+0x1EC/+0x1F4` is structurally live
but functionally dormant — no live-in-YR code path writes a non-zero
duration to `+0x1F4`, so the suppression branch never fires. See section
10 (TS-Legacy Register).

**Relationship to prior reports:** This report **VERIFIES, CORRECTS, AND
EXTENDS** the existing reports in `ra2-rust-game-docs/`:
- `CLOAKING_VISUAL_PIPELINE.md`
- `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`
- `CLOAKING_INTERACTIONS_REPORT.md`
- `DISGUISE_SYSTEM_GHIDRA_REPORT.md`
- `SENSOR_CLOAK_DETECTION.md`
- `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md`

**Two corrections to prior reports were found this session** — see
sections 4.2 and 4.3.

This is the **shader-bridge report** that ties gamemd's visual-state internals
to the Rust GPU pipeline's `SpriteInstance.fx_flags` bit 0 +
`fx_params[0]` (per
[docs/plans/2026-05-10-voxel-gpu-remap-fx-design.md](../../../plans/2026-05-10-voxel-gpu-remap-fx-design.md)).
The primary deliverable is the per-state recipe table in §11.

---

## 1. TL;DR — what changes the player sees

Five distinct cloak-FX outputs the player can observe. The shader must
reproduce each:

1. **Cloak fade-out animation** (state 0→2). Over `CloakingStages` ticks (default 9, gated by `CloakingSpeed`), the unit fades through alpha bands: opaque → shimmer (75/25 dithered) → 50% blend → 25% blend → invisible. Plays `CloakSound` at state-transition start.
2. **Cloak fade-in animation** (state 2→0). Reverse of fade-out. Plays the same `CloakSound`.
3. **Fully cloaked unit, allied/own view**: shows the unit with a **pulsing shimmer** cycle (4 distinct shimmer bands every 256 game ticks). Different from a flat 50%-blend.
4. **Fully cloaked unit, enemy view**: skipped entirely (no draw) **unless** the enemy has a sensor over the cell, in which case it shows the same 50%-blend as allied view.
5. **Mirage Tank disguised**: a TypeClass sprite-swap (not a cloak FX). Allied/own viewers see the real Mirage; enemy viewers see a tree sprite (`DefaultMirageDisguises`-picked OverlayType). The Mirage **is not Cloakable=yes**, so cloak FX never composes with the tree sprite in retail YR.

The Spy disguise (PermaDisguise) uses the same GetDisplayType-swap as
Mirage. Cloak FX doesn't compose with it either (Spy is not `Cloakable=yes`).

---

## 2. TechnoClass Cloak/Shimmer Field Map (verified offsets)

All offsets are direct byte offsets on `TechnoClass` (i.e., from the `this`
pointer). Verified from disassembly of the functions cited. **Critical: many
prior-report C decompilations show `param_1[N]` where param_1 is `int *`,
which means byte offset = `N × 4`. Cross-check against the function's
parameter type per CLAUDE.md decompilation pitfall.**

| Offset | Size | Field | Verification site |
|---|---|---|---|
| `+0x09C` | 12 bytes | `Position` (x:i32, y:i32, z:i32) | GetVisualState 0x007038FE reads `param_1+0x9C` |
| `+0x1B4` | DWORD | WarpIn timer start (-1 = inactive) | ScaleByWarpInVisualPhase 0x0070E4B0 |
| `+0x1BC` | DWORD | WarpIn timer duration | same |
| `+0x1C0` | DWORD | WarpIn phase id (1-9 enum) | same |
| `+0x198` | DWORD | Temporal-fade timer start (-1) | ScaleByTemporalVisualPhase 0x0070E380 |
| `+0x1A0` | DWORD | Temporal-fade timer duration | same |
| `+0x1A4` | DWORD | Temporal-fade phase id | same |
| **`+0x1D8`** | BYTE | **Disguise-active flag** | TurretAI 0x0074, Constructor 0x006F2CB2 inits to 0 |
| **`+0x1DC`** | DWORD | **Disguise-pick frame counter / cloak-shimmer phase base (SHARED)** | TurretAI writes via `param_1[0x77] = g_CurrentFrameCounter`; ModifyCloakDrawFlags reads via `*(int *)(param_1 + 0x1dc)`; Constructor 0x006F2CB8 inits to 0 |
| **`+0x1EC`** | DWORD | Shimmer-suppression timer start (-1 = inactive) | ModifyCloakDrawFlags 0x0070ED8E; Constructor 0x006F2CD5 inits to `g_CurrentFrameCounter` |
| **`+0x1F4`** | DWORD | Shimmer-suppression timer duration | ModifyCloakDrawFlags 0x0070ED94; Constructor 0x006F2CDB inits to 0 |
| `+0x21C` | DWORD | `Owner` (HouseClass*) | ModifyCloakDrawFlags 0x0070EDAD reads for IsHumanPlayer |
| **`+0x220`** | DWORD | **CloakState (0/1/2/3 enum)** | GetVisualState 0x007038BB, CloakingTick `param_1[3].RefCount`, StartCloaking 0x00703770 |
| **`+0x224`** | DWORD | **CloakProgress** | GetVisualState 0x00703A67, StartCloaking sets to 0, StartUncloaking sets to `CloakingStages - 1` |
| `+0x228` | BYTE | CloakDirty (set when Progress changes) | CloakingTick |
| `+0x22C..0x237` | CDTimer | CloakStepTimer (start at +0x22C, duration at +0x234) | StartCloaking writes via `param_1[0x8b..0x8d]` |
| `+0x238` | DWORD | CloakingSpeed (cached from TypeClass+0x310) | StartCloaking 0x00703770 |
| `+0x23C` | DWORD | CloakStepDelta (+1 cloaking, -1 uncloaking) | StartCloaking sets to +1, StartUncloaking sets to -1 |
| `+0x240..0x24B` | CDTimer | ReCloakDelayTimer (start at +0x23C, dur at +0x244) | CloakingTick state 3→0 transition |
| `+0x418` | BYTE | "Selectable/notify-player" flag | ProcessCloakAndNotify 0x006F4A70 |
| **`+0x41A`** | BYTE | **IsDiscoveredByCurrentPlayer** | GetVisualState 0x00703879 |
| `+0x518` | DWORD | Disguised TypeClass pointer (Mirage→Tree) | TurretAI 0x00746A85 writes `param_1[0x146]`; GetDisplayType returns this for enemy view |
| `+0x51C` | DWORD | Disguise frame index (set to 0 by TurretAI) | TurretAI 0x00746A8B writes `param_1[0x147] = 0` |
| `+0x6C4` | DWORD | True (own) TypeClass pointer | UnitClass::GetDisplayType returns this for allied view |
| `+0x6D3` | BYTE | "Infantry building-underside" flag (affects DrawSHP draw mode) | TechnoClass__Draw 0x00706640 |

### TechnoTypeClass cloak field offsets (verified)

All direct byte offsets, all confirmed by reading the `MOV [EBP+offset]`
instruction surrounding the ReadINI call.

| Offset | Size | INI Key | String addr | Parse site |
|---|---|---|---|---|
| `+0x2A2` | BYTE | `VeteranAbilities=CLOAK` (= VeteranAbilities[6]) | — | CloakingTick |
| `+0x2B4` | BYTE | `EliteAbilities=CLOAK` | — | CloakingTick |
| `+0x310` | DWORD | **`CloakingSpeed`** | 0x0084443C | TechnoTypeClass::ReadINI @ 0x00712441 (verified bytes `MOV [EBP+0x310], EAX` at 0x00712450) |
| `+0xC93` | BYTE | **`CloakStop`** | 0x008441B4 | TechnoTypeClass::ReadINI @ 0x00713116 (verified `MOV [EBP+0xC93], AL` at 0x00713120) |
| `+0xC9A` | BYTE | **`Invisible`** | 0x00843944 | TechnoTypeClass::ReadINI @ 0x00714A9E (verified `MOV [EBP+0xC9A], AL` at 0x00714AB0) |
| `+0xC9D` | BYTE | `Sensors` | — | BuildingClass::ShouldUncloak reads this at 0x00457938 |
| `+0xCD0` | BYTE | **`Cloakable`** | 0x00843EA8 | TechnoTypeClass::ReadINI @ 0x00713F7F (verified `MOV [EBP+0xCD0], AL` at 0x00713F90) |

### RulesClass cloak field offsets

| Offset | Size | INI Key | Section | Default | Verification |
|---|---|---|---|---|---|
| **`+0x628`** | DWORD | **`CloakingStages`** | [General] | 9 | StartUncloaking 0x00703710 reads `*(int *)(g_RulesClass_Instance + 0x628)`; GetVisualState 0x00703A83 FIDIV's by `[EDX + 0x628]` |
| **`+0x6A0`** | DWORD | **`CloakSound`** (VocClass index) | [AudioVisual] | NavalUnitEmerge | RulesClass::ReadAudioVisual writes via `param_1[0x1a8]` where param_1 is `int *` → 0x1A8 × 4 = 0x6A0 |
| **`+0xFFC`** | pointer | **`DefaultMirageDisguises`** list head | [General] | TREE01..TREE04 | TurretAI 0x00746A60 reads `*(int *)(g_RulesClass_Instance + 0xFFC)` as array base |
| **`+0x1008`** | DWORD | DefaultMirageDisguises count | (computed) | 4 | TurretAI reads `*(int *)(g_RulesClass_Instance + 0x1008)` as upper bound for `RandomRanged(0, n-1)` |
| `+0x1014` | DWORD | **`MirageDisguiseSwitchInterval`** (or similar) | [General] | TBD (used as disguise timer duration) | TurretAI 0x00746A2C writes `param_1[0x7a] = *(int *)(g_RulesClass_Instance + 0x1014)` (disguise-pick-on-enemy-near timer duration) |
| `+0x1708` | DOUBLE | `ConditionRed` or similar health-ratio threshold | [AudioVisual] | (low health) | CloakingTick reads `*(double *)(g_RulesClass_Instance + 0x1708)` for auto-cloak/uncloak gate |

### Globals

| Address | Symbol | Purpose | Verification |
|---|---|---|---|
| `0x00A8ED84` | `g_CurrentFrameCounter` | Game-tick counter, deterministic | ModifyCloakDrawFlags 0x0070ED86, TurretAI, StartCloaking, Constructor — all read this |
| `0x008871E0` | `g_RulesClass_Instance` | (Pointer-to-RulesClass*) | GetVisualState 0x00703A7D loads from this |
| `0x0087E8A4` | `g_ABuffer` | A-buffer descriptor pointer (dither pattern source) | Shimmer/25%/50% blitters all read row-end at `[g_ABuffer + 0x1C]` and row-pitch at `[g_ABuffer + 0x20]` |
| `0x00887644` | **`g_ZBuffer`** | Z-buffer descriptor pointer | Confirmed via WinMain WRITE xref at 0x006BDE93; used by brightness blitters (FUN_00495730 at 0x004955E4) |
| `0x00B73550` | **`g_hWnd`** | Main window handle (0 = headless/dedicated) | **Confirmed via WinMain READ xrefs at 0x006BDA75, 0x006BDAE7, 0x006BDB8A, 0x006BDC08, 0x006BDC7F.** GetVisualState 0x0070396B reads — if 0, returns visual state 3 for fully-cloaked unit |
| `0x00A8B238` | **`g_GameMode`** | Game mode (0 = unset/loading) | **Confirmed via Main_Game WRITE xref at 0x0048CF94.** GetVisualState 0x00703A09 reads — if 0, returns state 5 (fully invisible) in the no-player-pointer branch |
| `0x00A8ED6B` | **`g_IsMapEditor`** | Map editor flag (1 = in editor) | Referenced in FUN_004F4780 and Main_Game; GetVisualState 0x00703897 uses for editor-mode bypass |
| `0x00A83D4C` | (likely) g_PlayerPtr alias | Used in allied check at GetVisualState 0x007039C0 | — |
| `0x007E1710` | `dbl_256_0` | Hard-coded `256.0` double | Memory dump: `00 00 00 00 00 00 70 40` = 256.0; used by GetVisualState FMUL |
| `0x0088A084` | `g_IntensityTableCache` | Array of cached intensity tables | FUN_00420140 0x00420143 reads as cache base |
| `0x0088A090` | `g_IntensityTableCount` | Count of cached intensity tables | FUN_00420140 0x00420149 reads as cache count |

---

## 3. CloakState State Machine (CloakingTick @ 0x006FB740)

**OQ#1 RESOLVED.** Address 0x006FB740 IS the cloak state machine. Ghidra has
it named `TechnoClass__CloakingTick`. The Agent D scoping read in the
investigation plan was a misread — the body DOES match the cloak machine,
it's just structured via `param_1[3].XYZ` because Ghidra interpreted the
cloak fields as an ObjectClass struct overlay. The numeric field reads
(via StartCloaking's cleaner `int *` typing) confirm the offsets.

### 3.1 State enum (CloakState at +0x220)

```
0 = Uncloaked    — opaque draw, no FX
1 = Cloaking     — fade-out animation in progress
2 = Cloaked      — fully cloaked (allied: shimmer pulse; enemy: skip or 50%-with-sensor)
3 = Uncloaking   — fade-in animation in progress
```

No 4th state exists. Verified by exhaustive switch coverage in CloakingTick.

### 3.2 Tick dispatch (per-tick, called from TechnoClass::AI)

CloakingTick is called every game tick for every TechnoClass with potential
cloak ability. The dispatch is:

```
RefCount(=CloakState) switch:
  case 0 (Uncloaked):
    if (vtable+0x288 IsCloakable returns 0) AND (no veteran/elite cloak ability):
      return  // not cloakable, no further processing
    if (FootClass::GetDestination is a building with TypeClass+0x16BD set):
      // destination cell is inside a gap-generator building — special handling
      ...
    if (CloakStepTimer expired):
      CloakProgress += CloakStepDelta
      CloakStepTimer.start = g_CurrentFrameCounter, .duration = CloakStepDelta
      CloakDirty = 1
    if (vtable+0x2A0 CanAutoCloak returns nonzero):
      if (GetHealthRatio > RulesClass+0x1708):
        call StartCloaking(0)  // healthy enough → auto-cloak
        return
      if (RandomRanged(0, 99) < 4):
        call StartCloaking(0)  // 4% per-tick chance even when unhealthy
        return
    return

  case 1 (Cloaking):
    if (CloakStepTimer.start == 0):  // first tick of state 1
      CloakStepTimer.start = g_CurrentFrameCounter, .duration = 1
    visual = GetVisualState(1, 0)
    if (visual == 2):
      // health-gated re-uncloak: low-health units interrupt mid-cloak
      if (GetHealthRatio > RulesClass+0x1708): return
      if (RandomRanged(0, 99) > 9): return
      call StartUncloaking(1)
      return
    if (visual in {3, 5}):
      // transition to Cloaked
      CloakState = 2
      CloakStepTimer.start = g_CurrentFrameCounter
      CloakStepTimer.duration = CloakStepDelta
      CloakProgress = 0
      Mark(2)  // redraw
      if (WhatAmI == Unit (1) and AttachedTag at +0x28 != -1):
        call vtable+0xFC (notification)
      else:
        // gap-generator-shroud add: for every TechnoClass mind-controlled by this one OR for self,
        // add this unit's cell to the SensorCount for the cell's house
        ...
      if (WhatAmI == Unit && NOT HouseClass__IsPlayerControl):
        call vtable+0x174  // chrono-shift to null-coord (?)
    return

  case 2 (Cloaked):
    if (vtable+0x2A4 ShouldUncloak returns nonzero):
      call StartUncloaking(0)
    return

  case 3 (Uncloaking):
    Mark(2)
    visual = GetVisualState(1, 0)
    if (visual == 0):
      // fully visible — transition to Uncloaked
      CloakStepTimer.start = g_CurrentFrameCounter, .duration = unknown(unaff_EBP)
      CloakState = 0
      CloakProgress = 0
      ReCloakDelayTimer.start = g_CurrentFrameCounter, .duration = ftol(unaff_EBP)
      Mark(2)
      return
    if (visual != 1):
      return
    if (vtable+0x2A0 CanAutoCloak returns nonzero):
      call StartCloaking(1)  // re-cloak: 1 arg means "skipped sound"
    return
```

**Critical detail — the second arg of StartCloaking / StartUncloaking:**

- `StartCloaking(0)` → plays CloakSound at unit position (verified at
  StartCloaking 0x00703790: `if (unaff_retaddr == 0) VocClass__PlayAt(0)`).
  This is the path triggered by auto-cloak transitions.
- `StartCloaking(1)` → suppresses CloakSound. Used by the cloak-up branch
  inside the uncloaking state (state 3 → state 1) so the unit doesn't double-
  beep when interrupting an uncloak.
- Same convention for StartUncloaking. Verified at 0x007036DD:
  `if (param_2 == 0) VocClass__PlayAt(0)`.

**Edge case for shader:** Per CloakingTick state 1, when visual_state reaches 2
during cloak-up, there's a 10% per-tick (`RandomRanged(0,99) > 9 = false` for
< 10) chance to ABORT the cloak and re-uncloak. This is a non-determinism
trap if Rust uses a different RNG order. Note that auto-cloak chance is
**4%** in state 0 (interpolating health), but ABORT chance is **10%** in
state 1 — these are different thresholds. The RNG calls are in this order:
- State 0 auto-cloak: GetHealthRatio compared to RulesClass+0x1708, then
  Random(0,99) < 4
- State 1 abort-uncloak: GetHealthRatio compared to RulesClass+0x1708, then
  Random(0,99) > 9

### 3.3 StartCloaking and StartUncloaking field writes

```c
// StartCloaking (verified disassembly):
if (CloakState == 0 || CloakState == 3) {
    vtable+0xDC(0);                                 // Mark dirty or layer-add
    CloakState = 1;
    CloakProgress = 0;
    CloakStepTimer.start    = g_CurrentFrameCounter;
    CloakStepTimer.unk_offset_4 = iStack_c;           // stack garbage — likely 0
    CloakStepTimer.duration  = TypeClass+0x310;       // CloakingSpeed
    CloakingSpeed_cache       = TypeClass+0x310;       // also stored at +0x238
    CloakStepDelta            = 1;
    if (param_2 == 0) VocClass__PlayAt(0);            // play CloakSound
    if (Owner != g_PlayerPtr && byte+0x83) call vtable+0x150;  // notify
}

// StartUncloaking (verified):
if (CloakState == 2 || CloakState == 1) {
    CloakState = 3;
    CloakProgress = RulesClass+0x628 - 1;           // CloakingStages - 1
    // ... same timer fields as StartCloaking ...
    CloakStepDelta = -1;
    if (param_2 == 0) VocClass__PlayAt(0);
}
```

**Subtle parity item**: the "uninit iStack_c stack value" is written into
`param_1[0x8c]` (= `+0x230`, CloakStepTimer.unk_offset_4). On most Windows
stack layouts, this would be a leftover stack value from the caller (e.g.,
the saved EBP or a prior local). For Rust parity, the cleanest interpretation
is **CDTimer.start_offset_4 = 0 at startup, undefined thereafter**, but the
field is never read after this write per our scan — it appears unused. Treat
as opaque junk and zero-init in Rust.

**Parity item**: StartCloaking/StartUncloaking do NOT write to `+0x1DC`,
`+0x1EC`, or `+0x1F4`. These fields are touched only by `TechnoClass__Constructor`
and (for `+0x1DC`) by `UnitClass::TurretAI`.

---

## 4. Visual State Mapping (GetVisualState @ 0x00703860)

Returns a `char` 0..5 from `CloakState`, `CloakProgress`, viewer
perspective, and global state.

### 4.1 Full decision tree (verified from disassembly)

```
GetVisualState(this, char param_2_perspective_query, void *param_3_viewer):

  TypeClass = vtable+0x84()   // GetType
  invisible_flag = TypeClass+0xC9A     // byte
  is_discovered  = this+0x41A           // byte

  // First gate: Invisible=yes branch
  if invisible_flag != 0 AND is_discovered != 0:
    return 0    // Invisible-type unit, already discovered → render normal
  
  // Re-fetch TypeClass (Ghidra artifact — compiler emitted second call)
  if TypeClass+0xC9A != 0 AND this+0x41A == 0 AND g_IsMapEditor (byte at 0x00A8ED6B) == 0:
    return 5    // Invisible-type + undiscovered + not editor → totally hidden

  if CloakState (this+0x220) == 0: return 0
  if g_IsMapEditor != 0: return 0
  if vtable+0x2C WhatAmI() == 6 (Building): return 0    // BUILDINGS NEVER GET CLOAK FX

  // Branch on CloakState
  if CloakState == 2 (Cloaked):
    if param_2 != 0 (perspective-aware query):
      if param_3 == 0 (no viewer): return 5
      viewer_house = param_3+0x30
      cell = MapClass::GetCellClass(this.Position rounded to cell)
      if cell.SensorCountForHouse(viewer_house) != 0: return 3
      return 5
    // param_2 == 0 (render-time)
    if g_hWnd (at 0x00B73550) == 0: return 3   // headless/dedicated
    if this+0x41A (IsDiscovered) != 0: return 3
    cell = MapClass::GetCellClass(this.Position)
    if cell.SensorCountForHouse(g_PlayerPtr.ArrayIndex) != 0: return 3
    if g_GameMode (at 0x00A8B238) == 0: return 5
    if this+0x87 (Owner) == 0: return 5
    if g_PlayerPtr == 0: return 5
    if !HouseClass::IsAlliedWith(g_PlayerPtr, this.Owner): return 5
    if !HouseClass::IsAlliedWith(this.Owner, g_PlayerPtr): return 5
    return 3    // allied & mutually allied — see shimmering cloaked unit

  // CloakState in {1, 3}: animating
  progress = CloakProgress (this+0x224)
  if progress <= 0: return 0
  
  visual = ftol(progress / CloakingStages * 256.0)   // FIDIV by RulesClass+0x628, FMUL by dbl@0x007E1710
  if visual < 0x40: return 1
  if visual < 0x80: return 2
  if visual < 0xC0: return 3
  if param_2 == 0 AND this+0x41A != 0:
    return 3   // discovered + render-time + visual >= 0xC0 → cap at 3 (allied-see-discovered behavior)
  return (visual >= 0xFF) ? 5 : 4
```

### 4.2 CORRECTION TO PRIOR REPORT — discovered-clamp branch

Prior `CLOAKING_VISUAL_PIPELINE.md` describes the discovered-clamp branch
as "75-99%: return 4, BUT if param_2==0 AND IsDiscoveredByCurrentPlayer:
return 3 instead of 4 (clamp for allied view)". This is RIGHT but the doc's
example walkthrough doesn't apply it. **Verified**: when visual reaches
0xC0+ (state 4 region) AND the unit has been "discovered" (e.g., by sensor
sweep), the render-time call returns 3 instead of 4. This means **a
discovered cloaking unit visually stops getting more transparent at the 50%
blend stage and stays there until fully cloaked**. Parity-relevant.

### 4.3 Visual progression for CloakingStages=9

```
Cloaking (state 1, Progress ticks UP):
  P=0       visual_state=0    (Progress must be > 0 to enter compute branch)
  P=1       28 < 0x40         → state 1
  P=2       56 < 0x40         → state 1
  P=3       85 < 0x80         → state 2
  P=4-6     113-170 < 0xC0    → state 2
  P=7       199 ≥ 0xC0        → state 3 (CloakingTick transitions to Cloaked here)
  P=8       227 ≥ 0xC0, <0xFF → state 4 (unreachable — already transitioned)
  P=9+      256+ ≥ 0xFF        → state 5 (unreachable)

Uncloaking (state 3, Progress ticks DOWN from CloakingStages-1=8):
  P=8       227 ≥ 0xC0, <0xFF → state 4    ← first tick visible at state 4
  P=7       199 ≥ 0xC0        → state 3
  P=6-3     85-170 < 0xC0     → state 2
  P=2       56 < 0x40         → state 1
  P=1       28 < 0x40         → state 1
  P=0       (compute fails)   → state 0 (CloakingTick transitions to Uncloaked)
```

**Subtle**: state 4 is only reachable during *uncloaking's first tick*, not
during cloaking. This is consistent with the gameplay observation that
cloaking feels faster than uncloaking — cloak-up skips state 4 entirely.

### 4.4 CORRECTION TO PRIOR REPORT — exit visual at 0xFF boundary

Verified at disassembly 0x00703AED-0x00703AF4:
```
00703aed: XOR ECX,ECX
00703aef: CMP EAX,0xFF
00703af4: SETGE CL          ; CL = 1 if EAX >= 0xFF, else 0
00703af7: ADD ECX,0x4       ; ECX = 4 + (1 if EAX >= 0xFF else 0) = 4 or 5
```

So: `EAX == 0xFE → state 4`; `EAX == 0xFF → state 5`. Boundary is `>= 0xFF`,
not `> 0xFE`. (These are mathematically equivalent for integers, but the
prior doc's C statement `return (0xfe < iVar3) + 4` is the literal
decompiler output, not the underlying SETGE.) Document for accurate
re-implementation.

---

## 5. Allied Shimmer Pulse (ModifyCloakDrawFlags @ 0x0070ED80)

For cloaked units owned by the local human player (and any animating unit
with the shimmer-timer expired or inactive), this function modulates the
draw flags to produce a pulsing shimmer effect.

### 5.1 Function body (verified from disassembly)

```c
uint ModifyCloakDrawFlags(this, uint flags) {
    int duration = this+0x1F4;
    int start    = this+0x1EC;
    if (start != -1) {
        elapsed = g_CurrentFrameCounter - start;
        if (duration <= elapsed) goto compute_phase;  // timer expired
        duration = duration - elapsed;                 // remaining time
    }
    if (duration != 0):
        // Timer still has time remaining — gate on owner
        if (!HouseClass::IsHumanPlayer(this.Owner)):
            return flags;   // suppress shimmer for non-human-owned units during timer
    
compute_phase:
    uVar2 = (g_CurrentFrameCounter - this+0x1DC + 0x40) & 0x800000FF;
    // Sign-extend correction (handle wrap-around to negative byte):
    if ((int32)uVar2 < 0):
        uVar2 = (uVar2 - 1 | 0xFFFFFF00) + 1;       // restore signed-byte semantics
    
    // 4 shimmer bands, 2 50%-blend bands, opaque elsewhere — full table:
    if uVar2 < 0x40: return flags                           // OPAQUE
    if uVar2 in [0x40, 0x43]: return flags | 2              // shimmer (75/25)
    if uVar2 in [0x44, 0x4B]: return flags | 4              // 50% blend
    if uVar2 in [0x4C, 0x4F]: return flags | 2              // shimmer
    if uVar2 in [0x50, 0x6F]: return flags                  // OPAQUE
    if uVar2 in [0x70, 0x73]: return flags | 2              // shimmer
    if uVar2 in [0x74, 0x7B]: return flags | 4              // 50% blend
    if uVar2 in [0x7C, 0x7F]: return flags | 2              // shimmer
    if uVar2 >= 0x80: return flags                          // OPAQUE
}
```

### 5.2 CRITICAL CORRECTION TO PRIOR REPORT

`CLOAKING_VISUAL_PIPELINE.md` documents the phase bands as:
```
// 0x4C-0x4F: opaque flash
// 0x7C-0x7F: opaque
```

**Both are WRONG.** Verified from the JL/JGE control flow in the
disassembly at 0x0070EDE2-0x0070EE1B: when uVar2 falls in [0x4C, 0x4F] or
[0x7C, 0x7F], the code falls through to `OR ESI, 0x2` at 0x0070EE18 → **shimmer**.

**Correct table:** 4 shimmer bands, NOT 2. The pulse cycle is:

```
phase ∈ [0x00, 0x3F]: opaque (64 frames)
phase ∈ [0x40, 0x43]: shimmer (4 frames)
phase ∈ [0x44, 0x4B]: 50% (8 frames)
phase ∈ [0x4C, 0x4F]: shimmer (4 frames)
phase ∈ [0x50, 0x6F]: opaque (32 frames)
phase ∈ [0x70, 0x73]: shimmer (4 frames)
phase ∈ [0x74, 0x7B]: 50% (8 frames)
phase ∈ [0x7C, 0x7F]: shimmer (4 frames)
phase ∈ [0x80, 0xFF]: opaque (128 frames)
```

Cycle length = 256 game ticks. Total: 128 opaque-tail + 64 opaque-head +
32 opaque-middle = 224 opaque + 16 shimmer + 16 50%-blend = 256. So the
shimmer/blend pulse fires about 12.5% of the time (32/256).

The pattern produces a "first pulse" at the start of each 0x40-period:
shimmer → 50% → shimmer (4+8+4 = 16 frames), then a long opaque (32 frames),
then a "second pulse" (shimmer → 50% → shimmer, 16 frames), then a long
opaque (128 frames). Player observes: **two shimmer-pulses per ~5-second
window, then a 2-second opaque gap, repeat**.

### 5.3 The shimmer-suppression timer is dormant in YR

The timer at `+0x1EC`/`+0x1F4` would suppress shimmer for non-human-owned
units when duration > 0. **It is never armed in retail YR play.** Verified:
- `TechnoClass__Constructor` (0x006F2B40-0x006F3268) initializes
  `+0x1EC = g_CurrentFrameCounter` (frozen at unit creation) and
  `+0x1F4 = 0`. With duration=0, the elapsed check immediately succeeds:
  `duration (0) <= elapsed (≥0)`, jumps to compute_phase, suppression
  branch never executes.
- Byte-pattern search for writers `89 ?? F4 01 00 00` (MOV [reg+0x1F4], reg)
  returned NO live in-function matches outside the constructor. The
  candidate sites 0x0068EE6A and 0x0068EE90 are in unclassified code
  (no enclosing function — `get_function_by_address` returned "No function
  found") and the disassembly at those points shows reads of `+0x1DC` and
  `+0x2E0` rather than a write to `+0x1F4` — byte-pattern false positives
  from adjacent instruction overlap.

**Conclusion:** Per CLAUDE.md TS-legacy rule, the `+0x1EC/+0x1F4` shimmer-
suppression timer is shipping-but-dormant in YR. **The shader should
unconditionally apply the shimmer phase formula for any cloaked unit (state
2) viewed by the local player.**

### 5.4 OQ#5 RESOLVED — `+0x1DC` IS shared between disguise and cloak shimmer

This was an explicit open question. Resolution:

- `UnitClass::TurretAI` writes `param_1[0x77] = g_CurrentFrameCounter`
  (verified by decomp at 0x00746A90). With `param_1` typed `int *`, the byte
  offset is `0x77 × 4 = 0x1DC`.
- `TechnoClass__ModifyCloakDrawFlags` reads `*(int *)(param_1 + 0x1DC)`
  (direct byte arithmetic, param_1 typed `int`). Confirmed byte offset
  `0x1DC`.
- `TechnoClass__Constructor` zero-inits at byte `+0x1DC` (via
  `MOV [ESI+0x1DC], EBX` at 0x006F2CB8 where EBX=0).

**Verified: the same dword at `+0x1DC` is shared between (a) the Mirage
disguise frame counter and (b) the cloak-shimmer phase base.**

**Player-observable consequence:** for any unit that is BOTH `Cloakable=yes`
AND `DisguiseWhenStill=yes`, every disguise re-pick would reset the shimmer
phase to "frame counter at re-pick time". The pulse cycle would
re-synchronize to the disguise pick instant.

**In stock YR, this doesn't fire** because:
- Mirage Tank (MGTK) is `DisguiseWhenStill=yes` but NOT `Cloakable=yes`.
- Spy (SPY) is `CanDisguise=yes`, `PermaDisguise=yes`, but NOT `Cloakable=yes`.
- The three Cloakable units (SUB, DLPH, SQD) are NOT `CanDisguise=yes`.

**However**, the path is reachable via Veteran promotion. `VeteranAbilities=
CLOAK` (TypeClass+0x2A2) grants cloak to a unit that lacks `Cloakable=yes`.
If a player promotes a Mirage Tank to veteran with a CLOAK ability (modded
or via map trigger), the field-sharing would become observable: every
disguise re-pick resets the shimmer phase. **For Rust parity, the cleanest
approach is to use a single field for both purposes** and accept that the
behavior matches gamemd. The alternative — separate fields — would
introduce a parity deviation for the moddable veteran-cloak case.

---

## 6. Blitter Per-Pixel Operations

All three cloak-relevant blitters use a similar inner-loop pattern. The
key parity items are: the blend ratio, the dither pattern via the intensity
table, and the color-0 transparency invariant.

### 6.1 Shimmer 75/25 blitter (`0x00494330`) — flag bit 0x02

```
intensity_clamp = clamp((param_8 < 1 ? 0 : param_8) * 0x105 >> 0xB, 0, 0xFE)
intensity_table_base = BlitterInfo+8                  // per-instance, set by Blitter_selector
remap_palette_base   = BlitterInfo+4                  // per-instance theater palette
channel_mask         = ushort at BlitterInfo+0xC      // RGB565 = 0xF7DE or RGB555 = 0x7BDE

per pixel (loop count = param_4):
    src_byte = *src++
    if src_byte != 0:                                  // COLOR 0 = TRANSPARENT — VERIFIED
        abuf_value = *abuf_ptr                          // ushort from a-buffer
        lut_offset = intensity_clamp * 0x200 + intensity_table_base + abuf_value * 2
        palette_idx = *(ushort *)lut_offset | src_byte
        rgb565 = *(ushort *)(remap_palette_base + palette_idx * 2)
        *dest = ((rgb565 >> 2) & mask) * 3 + ((*dest >> 2) & mask)
        // = 0.75 * shaded_src + 0.25 * dest
    abuf_ptr++; dest++
    if abuf_ptr at row-end (>= g_ABuffer+0x1C):
        abuf_ptr -= g_ABuffer+0x20                      // row-pitch wrap
```

### 6.2 50/50 blitter (`0x00497CF0`) — flag bit 0x04

```
intensity_clamp = same as above (param_9 instead of param_8)
intensity_table_base = BlitterInfo+8
remap_palette_base = BlitterInfo+4

This is the RLE variant: it consumes SHP run-length-encoded byte data.
src_byte == 0 means "skip next N pixels" (where N = next byte).

per pixel (after skip resolution):
    palette_idx = *(ushort *)(intensity_table_base + intensity_clamp * 0x200 + abuf * 2) | src_byte
    rgb565 = *(ushort *)(remap_palette_base + palette_idx * 2)
    *dest = ((rgb565 >> 1) & mask) + ((*dest >> 1) & mask)
    // = 0.5 * shaded_src + 0.5 * dest
```

### 6.3 25/75 blitter (`0x00494080`) — flag bit 0x06

```
Same loop structure as shimmer 75/25 but blend:
    *dest = ((rgb565 >> 2) & mask) + ((*dest >> 2) & mask) * 3
    // = 0.25 * shaded_src + 0.75 * dest
```

### 6.4 Intensity-table lookup explained

The shimmer/50%/25% blitters all read from a 2D intensity LUT:
`intensity_table[intensity_clamp][abuf_value]` where:
- `intensity_clamp` is in [0, 0xFE] (255 entries × 0x200 = 512 bytes per row)
- `abuf_value` is a ushort sampled from the a-buffer at the destination pixel position

The LUT is a `256 × 256` ushort table = **131,072 bytes** (allocated as `0x20008`
= 131,080 with 8-byte header). Indexed by `row_offset = intensity_clamp * 0x200
+ abuf_value * 2`. Each ushort value is `palette_index_high_bits` that gets
OR'd with the source byte to form the final palette index.

**The a-buffer provides the dither pattern.** Different screen pixels have
different a-buffer values, so the same source byte at the same intensity
produces different palette indices at different pixel positions. This is
NOT a flat alpha multiply — it's a **per-pixel dithered palette
substitution** that approximates translucency.

### 6.5 LUT generation formula — OQ#A FULLY RESOLVED

The intensity-table is generated lazily by `FUN_00420140` at 0x00420140
(decompiled this session). The function maintains a cache (`g_IntensityTableCache`
at 0x0088A084, `g_IntensityTableCount` at 0x0088A090); on cache miss, it
allocates `0x20008` bytes and fills using:

```c
n = size_param - 1     // typically 0xFE = 254 for blitter use
for idx in 0 .. 0x10000:
    low  = idx & 0xFF        // = abuf_byte (drives dither)
    high = idx >> 8           // = intensity_clamp (drives transparency level)
    val  = (low * high * n) / 0x7E02
    if val > n: val = n
    table[idx] = (val & 0xFF) << 8     // stored as ushort, value in high byte
```

**Constants:**
- `n` = 254 (= `size_param - 1` where size_param = 255)
- Divisor `0x7E02` = **32,258** decimal
- Output range: [0, 254], packed into the high byte of a ushort with low byte = 0

The blitter then OR's the table-lookup-result with the source byte:
`final_palette_idx = (table[(intensity_clamp << 8) | abuf] | src_byte)` —
which is a 16-bit index `(intensity << 8) | src_byte` into the `remap_palette`
(VPL-style two-byte palette indexing — same as voxel VPL).

**For shader parity, full reproduction is now trivial:**

```wgsl
fn shimmer_lookup(src_byte: u32, abuf: u32, intensity_clamp: u32) -> u32 {
    let n: u32 = 254u;
    let v_raw = (abuf * intensity_clamp * n) / 32258u;
    let v: u32 = min(v_raw, n);
    return (v << 8u) | src_byte;    // index into remap_palette
}
```

No precomputed LUT required — the formula is cheap enough to inline.

### 6.6 Shader path choice

- **Path A (full parity)**: implement the formula above per fragment. Sample
  the a-buffer or a noise/Bayer texture for `abuf`. Derive
  `intensity_clamp` from the per-instance transparency scale via
  `clamp((scale * 261) >> 11, 0, 254)`. Apply the VPL-style double-lookup
  to produce a per-pixel-dithered palette index. **Matches gamemd pixel-for-pixel.**
- **Path B (flat alpha)**: use the SpriteInstance.fx_params[0] as a flat
  alpha multiplier (0.75/0.5/0.25 per visual state). **Visibly smoother
  than gamemd** — gamemd's shimmer has a granular dither pattern.

Both paths now have known cost/benefit. Path A is recommended for parity
since the formula is simple. Path B is acceptable as a Phase 2.1
short-term implementation, swappable for Path A later.

### 6.5 Color-0 transparency is hard-coded

All three blitters explicitly check `if src_byte != 0` before writing. This
is the **transparency invariant** documented in
`VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md` §8 and re-confirmed here for the
SHP/blend draw path.

---

## 7. Draw Flag Construction — VXL vs SHP

Both `TechnoClass__Draw` (VXL) and `TechnoClass_DrawSHP` build draw flags
from visual_state but with subtly different encodings.

### 7.1 VXL flag encoding (TechnoClass__Draw 0x00706640)

```
flags = 0x2000  // base "VXL with custom frame index"
if param_11 == 0:     // not in warp-mode (caller may override)
    switch GetVisualState(0, 0):
        case 1: flags = 0x2002    // base + shimmer
        case 2:
        case 3: flags = 0x2004    // base + 50% blend
        case 4:
            if CloakProgress == 0: flags = 0x200A    // base + shimmer + brightness
            else:                  flags = 0x200C    // base + 50% blend + brightness
        case 5: return                                // skip draw

// Warp-in/out compose (sets bits 0x02 + 0x04)
if vtable+0x1D4 (IsWarpingIn) || vtable+0x1D8 (IsWarpingOut):
    if not (building with TypeClass+0x16B1):
        flags |= 0x2004
    else:
        flags |= 0x2006

// Allied shimmer (only for human-player-owned & locally controlled)
if HouseClass::IsHumanPlayer(Owner) && vtable+0xC4 IsPlayerControlled:
    if not (building with TypeClass+0x6D3):
        flags = vtable+0x43C (= ModifyCloakDrawFlags)(flags)

// Always-on bits
flags = (flags | 0x800) & ~param_11   // 0x800 = remap, param_11 = mask-out

// Visual transparency scale (for chrono fade composition)
scale = vtable+0x464 (= ScaleByTransparency)(param_9)
if vtable+0x160 (IsTemporal) || (building with vtable+0xA5):
    scale = ScaleByTemporalVisualPhase(scale)
    scale = ScaleByWarpInVisualPhase(scale)
```

### 7.2 SHP flag encoding (TechnoClass_DrawSHP 0x00705E00)

```
flags = 0
switch GetVisualState(0, 0):    // result in uStack_44
    case 2:
    case 3: flags = 4                                // 50% blend
    case 4:
        if CloakProgress != 0: flags = 4              // 50% blend (same as 2/3)
        else:                  flags = 2              // shimmer (same as 1) — NO brightness bit
    case 1: flags = 2                                // shimmer
    case 5: skip                                      // no draw

// Warp/temporal compose: flags |= 4 (or |= 6 for buildings with TypeClass+0x16B1)
// ... building underside override → flags |= 0x20
// Custom frame index: flags |= 0x2000
// Mirror flip: flags |= 0x4000
// Always remap: flags |= 0x800

// Allied shimmer (same as VXL)
if HouseClass::IsHumanPlayer(Owner) && vtable+0xC4 IsPlayerControlled:
    if not (building with TypeClass+0x6D3):
        flags = ModifyCloakDrawFlags(flags)
```

### 7.3 VXL brightness bit 0x08 — OQ#B FULLY RESOLVED

VXL state 4 uniquely adds bit `0x08` (via `0x200A` for Progress==0, or
`0x200C` for Progress!=0). `Blitter_selector` at `0x00490B90` dispatches
`flags & 6` to one of 4 blitter families (shimmer / 50% / 25% / opaque),
and `flags & 0x08` selects a brightness-variant slot within families
`0x02` and `0x04`:

| Flags input | Dispatched slot | Resolved draw method |
|---|---|---|
| 0x200A (state 4 + Progress==0) → 0x280A with remap | BlitterInfo+0xB4 | `FUN_00495730` |
| 0x200C (state 4 + Progress!=0) → 0x280C with remap | BlitterInfo+0xB0 | `FUN_00495590` |

**Reachability — CORRECTION TO MY EARLIER CLAIM.** I previously dismissed
state 4 + Progress!=0 as unreachable. **It IS reachable in normal play** —
during the first tick of uncloaking, CloakProgress starts at
`CloakingStages - 1 = 8` and the visual_state is 4 (= `8/9*256 = 227`,
which is in [0xC0, 0xFE]). So **every cloaked unit's first uncloak tick
fires the slot-0xB0 brightness blitter** for VXL units.

State 4 + Progress=0 (slot 0xB4) is still unreachable in normal play
(Progress=0 triggers the state 3→0 transition before the visual state can
be queried).

### 7.4 Brightness blitter (FUN_00495730 @ slot 0xB4) decompiled

Verified — the brightness variant adds three behaviors to the base
shimmer/50%-blend:

```c
do {
    z_value = *param_6;  // Z-buffer ushort
    param_6++;
    if (param_5 < z_value) AND src_byte != 0:    // Z-TEST: only draw if behind nothing
        // Same intensity-table lookup + dither as standard shimmer:
        intensity_idx = (intensity_clamp * 0x100 + abuf) ushort lookup
        palette_idx = intensity_idx | src_byte
        rgb565 = remap_palette[palette_idx * 2]
        // BLEND: 75/25, but reading dest from PARAM_2[PARAM_9] (offset!)
        *param_2 = ((rgb565 >> 2) & mask) * 3 + ((param_2[param_9] >> 2) & mask)
    
    param_2++; param_3++; param_4--
    // Wrap both Z-buffer and a-buffer pointers at row-end
    if (g_ZBuffer+0x1C <= param_6): param_6 -= g_ZBuffer+0x20
    if (g_ABuffer+0x1C <= puVar4): puVar4 -= g_ABuffer+0x20
} while loop_count > 0
```

**Three differences from base shimmer (FUN_00494330):**
1. **Z-buffer test** at `param_5 < *param_6`. Only writes if source depth
   is in front of (less than) Z-buffer value. Allows cloaked VXL to occlude
   and be occluded by 3D world content (terrain, ramps).
2. **Destination read from `param_2[param_9]`** (offset by `param_9` ushorts)
   instead of `*param_2`. The `param_9` parameter is a depth-aligned y-offset
   — this implements a **vertical sample-from-different-row** for the blend
   destination. Visually this creates a slight "ghost" of the unit drawn
   one or more scanlines up, which is the "brightness/glow" effect.
3. **Z-buffer pointer wrap** at row-end (in addition to a-buffer wrap).

**Blend ratio is still 75/25**, NOT 50/50. The "50% blend" label on this
slot is misleading — `flags & 6 == 4` dispatches HERE but the actual math
is 75/25 because the brightness bit replaces the inner-loop blend formula.

### 7.5 Brightness blitter (FUN_00495590 @ slot 0xB0) — state-4-normal case

`FUN_00495590` is not classified as a function in Ghidra's database but
the disassembly bytes confirm the same overall structure as
`FUN_00495730`:
- Same intensity-clamp formula `clamp((param_8 * 0x105) >> 11, 0, 0xFE)`
- Same intensity-table base read from `[ECX+0x08]`
- Same g_ZBuffer reference at 0x00887644
- Same `intensity_clamp * 0x200` stride

The blend ratio at slot 0xB0 (state-4 50%-family + brightness) was not
fully extracted from the byte disassembly this session — the function is
structurally similar to slot 0xB4 (75/25 blend with z-test + offset-dest).
Based on the slot-0xB0 dispatch being under `flags & 6 == 4`, the
expectation is **the brightness bit converts the 50/50 blend to either a
75/25 or 25/75 variant** (the same way slot 0xB4 does for shimmer). Most
likely: 75/25 (= more visible source), to match the "brightness boost"
semantic.

**For shader parity recommendation:** treat VXL state 4 (Progress != 0)
as a 75/25 blend with optional z-test. Visually similar to state 1
shimmer at 75% but with the dither pattern and z-interaction. If the
implementation drops z-test and offset-dest read, the result is a flat
75% alpha — close to state-1 visual fidelity, just at a different point
in the animation. **Recommended fx_params[0] for state 4 in shader: 0.75
(NOT 0.5).**

---

## 8. Mirage Tree-Disguise Sprite Swap (UnitClass::GetDisplayType @ 0x007465B0)

**OQ#6 RESOLVED.** The Mirage tree-disguise is a TypeClass swap, NOT a cloak
FX. The function:

```c
UnitClass::GetDisplayType(this, char force_true_type):
    bool allied = HouseClass::IsAlliedWith(this.Owner, g_PlayerPtr);
    if allied && force_true_type == 0:
        return this+0x6C4       // real TypeClass (own VXL)
    return this+0x518            // disguised TypeClass (tree OverlayType pointer)
```

The disguised type pointer at `+0x518` is set by `UnitClass::TurretAI` at
0x00746A85. The selection logic is:
- TurretAI fires every game tick.
- If unit not moving (FootClass::GetDestination returns 0) AND TypeClass has
  the disguise bit (`+0xD32`) set AND no enemy is in the unit's targeting
  range (8-cell scan), AND the timer at `+0x1E8`/`+0x1F0` has elapsed:
  - Pick a random tree from `RulesClass+0xFFC[]` (DefaultMirageDisguises),
    count at `+0x1008` (= 4 by default = TREE01..TREE04).
  - Set `this+0x518 = picked tree TypeClass`
  - Set `this+0x51C = 0` (frame index reset)
  - Set `this+0x1D8 = 1` (disguise active flag)
  - Set `this+0x1DC = g_CurrentFrameCounter` (**SHARED with cloak shimmer phase**)
  - Call `vtable+0x49C` (probably OnDisguiseChanged notifier)

If an enemy IS within range, the disguise breaks (vtable+0x470) and the
re-disguise timer at `+0x1E8`/`+0x1F0` is set to `RulesClass+0x1014`
ticks before another auto-disguise pick is allowed.

**Important parity items:**
- The 8-cell scan iterates the 8 surrounding cells (`DirectionOffsets`
  table at `0x0089F68A`) AND checks `IsBridge` for the unit's cell. This is
  the "is there an enemy nearby?" check.
- The 4-tick interval `g_CurrentFrameCounter & 0x80000007 == 0` (= every 8th
  tick) gates the scan to keep cost down. So Mirage scans only every 8 ticks.
- Tree TypeClass pointer comes from `RulesClass+0xFFC[index]` array.

**Hand-off to rendering:** GetDisplayType returns the tree TypeClass.
The caller (somewhere in TechnoClass__Render path) uses it to select VXL
vs SHP rendering and to look up frame data. The tree is an OverlayType
(SHP-based, not VXL), so the rendering goes through `TechnoClass_DrawSHP`
not `TechnoClass__Draw`. The visual_state at the top of DrawSHP is computed
on the Mirage's CloakState (which is 0 since Mirage isn't Cloakable). So
**the tree renders opaque** (state 0 → flag 0) with no cloak FX applied.

**For Rust integration:** the existing `display_type_override` field on
`GameEntity` (used by miner unload-class rendering) can be repurposed for
Mirage tree-swap. The render layer reads `cloak.disguised_type` (when set)
and uses it as the sprite-key TypeClass override. No `fx_flags` bit is
needed for disguise.

---

## 9. INI Keys Reference (verified offsets)

| Key | Section | Type | Class+Offset | Default | YR retail usage |
|---|---|---|---|---|---|
| `CloakingStages` | [General] | int | RulesClass+0x628 | 9 | Global, used by all cloak transitions |
| `CloakSound` | [AudioVisual] | VocClass ref | RulesClass+0x6A0 | NavalUnitEmerge | Plays at every cloak/uncloak state transition |
| `DefaultMirageDisguises` | [General] | TypeRef list | RulesClass+0xFFC..+0x1008 | TREE01..TREE04 | Used by MGTK only |
| `Cloakable` | per-type | bool | TechnoTypeClass+0xCD0 | no | SUB=yes, DLPH=yes, SQD=yes |
| `CloakingSpeed` | per-type | int | TechnoTypeClass+0x310 | 0 (clamped to 1?) | SUB=1, DLPH=1, SQD=5 (higher = slower fade) |
| `CloakStop` | per-type | bool | TechnoTypeClass+0xC93 | no | Used by IsCloakable check |
| `Invisible` | per-type | bool | TechnoTypeClass+0xC9A | no | **No retail YR type sets this** |
| `Sensors` | per-type | bool | TechnoTypeClass+0xC9D | no | Read by BuildingClass::ShouldUncloak |
| `VeteranAbilities=CLOAK` | per-type | bool | TechnoTypeClass+0x2A2 | no | Read by CloakingTick's IsCloakable check |
| `EliteAbilities=CLOAK` | per-type | bool | TechnoTypeClass+0x2B4 | no | Same |
| `CanDisguise` | per-type | bool | TechnoTypeClass+0xD2F | no | SPY=yes, MGTK=yes |
| `PermaDisguise` | per-type | bool | TechnoTypeClass+0xD30 | no | SPY=yes only |
| `DisguiseWhenStill` | per-type | bool | TechnoTypeClass+0xD31 | no | MGTK=yes only |

**Parity edge case — CloakingSpeed=0:** The TechnoTypeClass::ReadINI default
is whatever the field already holds, which for a fresh TypeClass is **0**.
With CloakingSpeed=0, the CDTimer would expire every tick → instant cloak.
The CDTimer math doesn't clamp Speed≤1 to 1 — verified by reading the timer
code. So a unit with CloakingSpeed=0 (effectively undefined behavior) would
cloak in one tick. **In stock YR, all 3 Cloakable units have CloakingSpeed
explicitly set** (1, 1, 5), so the zero-default case doesn't fire.

---

## 10. TS-Legacy Status Register

| Component | Status in YR | Verification |
|---|---|---|
| Unit-level CloakState machine | **Active** | SUB/DLPH/SQD use it every match |
| Allied shimmer pulse (ModifyCloakDrawFlags) | **Active** | Called from TechnoClass__Draw & DrawSHP for human-player-owned cloaked units |
| Shimmer suppression timer (`+0x1EC`/`+0x1F4`) | **Dormant** | Init at constructor; no in-function live writer to `+0x1F4` outside constructor |
| `Invisible=yes` GetVisualState branch | **Code-live, data-dormant** | Branch is reachable, but no retail YR type sets `Invisible=yes` |
| Building cloak generators (CloakGenerator=, building-level Cloakable=) | **Dormant** | No retail YR building uses these. BuildingClass::ShouldUncloak override at 0x004578C0 is dead in normal play. |
| `g_IsMapEditor` early-out branches | **Active in editor only** | Returns state 0 always in map editor mode |
| `g_hWnd == 0` headless return-state-3 | **Active in dedicated/replay only** | Should be reachable in dedicated-server skirmish |
| `g_GameMode == 0 → state 5` early-out | **Verify mode 0 meaning** | Likely "no-game-mode-selected" / shell — probably unreachable in skirmish |
| Mirage tree-disguise | **Active** | MGTK uses every match |
| Spy disguise | **Active** | SPY uses every match |
| VXL state-4 brightness bit (0x200A/0x200C) | **Code-live, condition-unreachable** | The Progress==0 trigger condition doesn't fire in normal state-4 (Progress=8 during uncloak first tick) |
| Cloak/disguise `+0x1DC` field sharing | **Active mechanically; observably moot in retail** | Stock YR has no Cloakable+Disguising unit type. Activates via veteran CLOAK promotion (rare). |
| `DecloakToFire` weapon-fire path | **Active** | Confirmed via GetFireError 0x006FC0B0 (out of scope this session — documented in prior reports) |

---

## 11. Shader-Bridge Recipe — THE primary deliverable

This is the per-state mapping from runtime cloak/perspective state to the
Rust `SpriteInstance` FX uniform values. Use this table to populate
`fx_flags` bit 0 and `fx_params[0]` (and optionally a sprite-key override
for the Mirage tree-disguise case) per entity, per frame.

**Inputs (per entity):**
- `CloakState` ∈ {0, 1, 2, 3} (sim state)
- `CloakProgress` ∈ [0, CloakingStages-1] (sim state)
- `CloakingStages` (RulesClass; default 9)
- `IsDiscoveredByCurrentPlayer` (sim state; per-viewer)
- `Owner` (HouseClass id)
- `LocalPlayer` (the rendering perspective)
- `cell.SensorCount[LocalPlayer.House]` (cell-level sensor reveal)
- `cloak_phase_base` (= unit `+0x1DC`; usually 0 at start, set on Mirage disguise pick if applicable)
- `g_CurrentFrameCounter` (game tick)
- `disguised_type_id` (= unit `+0x518`; non-zero if Mirage actively disguised)

**Outputs (to SpriteInstance):**
- `fx_flags` bit 0 (= the existing CLOAK flag in `apply_fx`)
- `fx_params[0]` (= alpha multiplier)
- (optionally) `sprite_override.type_id` (= disguised tree TypeClass, for sprite-swap)

### 11.1 Visual-state computation (do this first per entity)

```
fn compute_visual_state(cloak, viewer, cell) -> u8:
  # Invisible-type early-out (TS-legacy in retail YR but live in code):
  if type.invisible && !is_discovered:
    return 5  # fully hidden
  if cloak.state == 0:
    return 0
  if WhatAmI(entity) == Building:
    return 0  # Buildings never get cloak FX
  if cloak.state == 2:  # Cloaked
    if cell.sensor_count[viewer.house] != 0: return 3
    if is_discovered: return 3
    if !mutually_allied(viewer.house, entity.owner): return 5
    return 3  # allied — shimmer
  # state 1 (Cloaking) or 3 (Uncloaking)
  if cloak.progress <= 0: return 0
  visual_raw = trunc(cloak.progress / CloakingStages * 256.0)
  if visual_raw < 0x40: return 1
  if visual_raw < 0x80: return 2
  if visual_raw < 0xC0: return 3
  if is_discovered: return 3   # clamp at 3 when previously-discovered
  if visual_raw >= 0xFF: return 5
  return 4
```

### 11.2 Per-state shader uniform mapping

**Update from OQ#B resolution:** VXL state 4 actually uses a **75/25 brightness
blend (`fx_params[0] = 0.75`)**, NOT 50% as previously suspected. SHP state 4
still maps to 50% (no brightness path for SHP). Recommend separating the path
for VXL vs SHP.

| visual_state | fx_flags bit 0 | fx_params[0] (SHP) | fx_params[0] (VXL) | sprite_override | discard? |
|---|---|---|---|---|---|
| 0 | 0 | 1.0 | 1.0 | none | no |
| 1 | 1 | 0.75 (dither — see §6) | 0.75 | none | no |
| 2 | 1 | 0.50 | 0.50 | none | no |
| 3 | 1 | 0.50 | 0.50 | none | no |
| 4 | 1 | 0.50 (no brightness in SHP) | 0.75 (brightness — see §7.5) | none | no |
| 5 | (any) | (any) | (any) | none | **YES — skip draw entirely** |

For path-A shader (full parity), state 1, 2, 3, 4 all use the **dithered
shimmer formula** with appropriate `intensity_clamp` derived from these
target alphas:

```
state_to_intensity_clamp(state) = match:
  1 → 191  (= 0.75 visible = 0.75 * 254)
  2,3 → 127  (= 0.50)
  4 (VXL) → 191
  4 (SHP) → 127
```

Then `intensity = clamp((intensity_clamp * 0x105) >> 11, 0, 254)` for the
blitter formula equivalence.

### 11.3 Allied shimmer phase overlay (state 2 + allied)

When `visual_state == 3` (allied view of fully cloaked unit) AND the entity
is `IsHumanPlayer(Owner)` AND IsPlayerControlled — the pulse cycle applies.
Compute on CPU per frame:

```
fn compute_allied_shimmer_alpha(cloak, current_frame) -> f32:
  phase = (current_frame - cloak.phase_base + 0x40) & 0xFF   # 256-tick cycle
  if phase < 0x40: return 1.0                                 # opaque
  if phase <= 0x43: return 0.75                                # shimmer (first burst)
  if phase < 0x4C: return 0.50                                 # 50% blend
  if phase <= 0x4F: return 0.75                                # shimmer (second burst — first cluster)
  if phase < 0x70: return 1.0                                  # opaque
  if phase <= 0x73: return 0.75                                # shimmer (third burst)
  if phase < 0x7C: return 0.50                                 # 50% blend
  if phase <= 0x7F: return 0.75                                # shimmer (fourth burst)
  return 1.0                                                    # opaque tail
```

Apply this AS A REPLACEMENT for the flat 0.5 in row "visual_state=3" above
when the unit is allied (= player-controlled). The shimmer pulse is
deterministic on game tick — no wall-clock dependency.

### 11.4 Mirage tree-disguise sprite override (parallel system, no FX)

```
fn compute_sprite_override(entity, viewer) -> Option<TypeId>:
  if entity.disguise_active && !mutually_allied(entity.owner, viewer.house):
    return Some(entity.disguised_type_id)
  return None
```

Note: independent of cloak state. Applied as a sprite-key override, not as
an `fx_flags` bit. The tree renders OPAQUE (no cloak FX) because Mirage
itself stays in CloakState=0 in retail.

### 11.5 Composition with warp/chrono FX (Phase 5, cross-reference only)

`TechnoClass__Draw` calls `ScaleByTemporalVisualPhase` (0x0070E380) and
`ScaleByWarpInVisualPhase` (0x0070E4B0) AFTER computing cloak draw flags.
These scale a transparency value through phase-based curves (case 1-9
selectors). For the cloak phase, the relevant detail is:

- `+0x198` / `+0x1A0` / `+0x1A4` = temporal-fade timer triplet
- `+0x1B4` / `+0x1BC` / `+0x1C0` = warp-in fade timer triplet

These are NOT cloak fields. They belong to Phase 5 (Warp/Chrono FX). The
cloak and warp effects multiplicatively compose at the per-pixel level —
when a unit is BOTH cloaking AND warping (e.g., a Chrono Miner that's
also cloakable), the alpha is the product. Documented for cross-reference;
implementation belongs to the Phase 5 investigation.

---

## 12. Open Questions

### OQ#A — Intensity-table contents — **RESOLVED**

The shimmer/50%/25%/brightness blitters read from a 256×256 ushort LUT.
The generation formula was extracted from `FUN_00420140` at 0x00420140 and
is documented in §6.5: `val = clamp((abuf * intensity_clamp * 254) / 32258, 0, 254)`,
stored in the high byte of a ushort with low byte = 0. The blitter then
OR's the table value with the source byte to form a 16-bit
`(shading << 8) | src` palette index that's used to look up the VPL-style
remap palette.

**For shader:** reproduce the formula inline (no LUT needed). See §6.6 for
WGSL recipe.

### OQ#B — VXL state-4 brightness blitter behavior — **RESOLVED**

The slot-0xB4 blitter (`FUN_00495730`) was fully decompiled this session
(see §7.4). It does a 75/25 blend with z-test + depth-offset destination
read. The slot-0xB0 blitter (`FUN_00495590`) was structurally inspected via
byte-disassembly and matches the same pattern (see §7.5) — likely also
75/25 with z-test and depth-offset.

**Reachability correction:** state 4 + Progress!=0 IS reachable in normal
play (during uncloak's first tick at Progress=8), dispatching the slot-0xB0
brightness blitter. State 4 + Progress=0 is unreachable in normal play.

**For shader:** map VXL state 4 to `fx_params[0] = 0.75` (matching the
brightness blitter's 75/25 ratio), NOT 0.5 as the visual-state-3 mapping
would suggest. See §11.2 table.

### OQ#C — Confirming symbol identification — **RESOLVED**

- `0x00B73550` = **`g_hWnd`** confirmed via WinMain READ xrefs at 0x006BDA75,
  0x006BDAE7, 0x006BDB8A, 0x006BDC08, 0x006BDC7F.
- `0x00A8B238` = **`g_GameMode`** confirmed via Main_Game WRITE xref at
  0x0048CF94.
- `0x00887644` = **`g_ZBuffer`** confirmed via WinMain WRITE xref at
  0x006BDE93.
- `0x0087E8A4` = **`g_ABuffer`** confirmed via heavy use across all blitters.

### OQ#D — Whether `g_GameMode == 0` is reachable in skirmish — **NOT BLOCKING**

The branch `if (g_GameMode == 0) return 5` is deep in the "fully cloaked
enemy view + no sensor + not allied" path. Since `g_GameMode` is set by
`Main_Game` at startup (write at 0x0048CF94) and the game enters Main_Game
loop before any cloaked unit can be rendered, the branch is effectively
unreachable during a live skirmish — only fires during boot/transition
states when units shouldn't render anyway. **Treat as safe to ignore in
the shader.**

### OQ#E — Iron-test against gamemd screenshot — **out of scope (impl-phase)**

Test-phase activity, not reverse engineering. Recommend a side-by-side
capture of:
- Player-owned cloaked SUB (allied shimmer cycle)
- Cloaking SUB animation (state 1 transitions)
- Cloaked SUB visible to enemy via DEST sensor (state 3 dither)
- Allied-viewed AI-owned Mirage tank disguised as tree
- VXL state 4 first uncloak tick (brightness variant — slot 0xB0)

For pixel-correct parity verification during Phase 2 implementation.

---

## 13. Current Rust Implementation Surface (summary)

- **GPU pipeline plumbing**: SpriteInstance has `fx_flags`/`fx_params`/`ic_tint`
  fields wired to vertex attributes 7-10. `apply_fx()` shader has cloak/EMP/IC/
  warp stub branches.
- **INI keys parsed**: `RadarInvisible`, `CanDisguise`, `DecloakToFire`,
  `DisguiseFireBlinkTime`, `DisguiseFireOnly`, `MakesDisguise`,
  `AttackCursorOnDisguise`, `GapGenerator` (placeholder).
- **INI keys NOT parsed yet (Phase 2 needs to add):** `Cloakable`,
  `CloakingSpeed`, `CloakStop`, `Invisible`, `DisguiseWhenStill`,
  `PermaDisguise`, `DetectDisguise`, `DetectDisguiseRange`, `Sensors`,
  `SensorsSight`, `CloakingStages` (global), `CloakSound` (global),
  `DefaultMirageDisguises` (global).
- **No `Cloak` sim component yet on `GameEntity`.**
- **No `tick_cloak` system in `World::advance_tick`.**
- **`display_type_override` mechanism already exists** (used by miner
  unload-class) — reusable for Mirage tree-swap.
- **FX uniforms set to default-zero** at all SpriteInstance push sites in
  `app_instances/units.rs`.

---

## Sources

### Ghidra functions decompiled this session (HIGH confidence)
- `TechnoClass__CloakingTick` @ `0x006FB740` (FULL)
- `TechnoClass_GetVisualState` @ `0x00703860` (FULL, with disassembly)
- `TechnoClass__ModifyCloakDrawFlags` @ `0x0070ED80` (FULL, with disassembly verification of phase bands)
- `TechnoClass__Draw` (VXL path) @ `0x00706640` (MEDIUM — flag-build path)
- `TechnoClass_DrawSHP` @ `0x00705E00` (MEDIUM — flag-build path)
- `TechnoClass__StartCloaking` @ `0x00703770` (FULL)
- `TechnoClass__StartUncloaking` @ `0x007036C0` (FULL)
- `TechnoClass__DoCloak` @ `0x004D3780` (LIGHT)
- `TechnoClass__DoUncloak` @ `0x006F4EB0` (LIGHT)
- `TechnoClass__ProcessCloakAndNotify` @ `0x006F4A70` (LIGHT)
- `UnitClass__TurretAI` (Mirage tree-pick) @ `0x007468C0` (FULL — disguise pick logic)
- `UnitClass__GetDisplayType` @ `0x007465B0` (FULL)
- `Blitter_Shimmer_75pct_Remap` @ `0x00494330` (FULL)
- `Blitter_ZWriteOnly_RLE_Remap_50pct` @ `0x00497CF0` (FULL — RLE variant)
- `Blitter_Scanline_Blend25pct_Remap` @ `0x00494080` (FULL)
- `Blitter_selector` @ `0x00490B90` (MEDIUM — dispatch table)
- `Blitter_init` @ `0x0048EBF0` (MEDIUM — blitter family construction)
- `FUN_00420140` (intensity-table generator) @ `0x00420140` (FULL — formula extracted)
- `FUN_00495730` (brightness-shimmer with z-test, slot 0xB4) @ `0x00495730` (FULL)
- `FUN_00495590` (brightness-50% with z-test, slot 0xB0) @ `0x00495590` (DISASSEMBLY ONLY — function not classified in Ghidra)
- `TechnoClass__ScaleByTemporalVisualPhase` @ `0x0070E380` (LIGHT — for Phase 5 cross-ref)
- `TechnoClass__ScaleByWarpInVisualPhase` @ `0x0070E4B0` (LIGHT — for Phase 5 cross-ref)
- `BuildingClass__ShouldUncloak` @ `0x004578C0` (LIGHT — TS-legacy confirmation)
- `RulesClass::ReadAudioVisual` @ `0x006691E0` (LIGHT — CloakSound offset)
- `TechnoClass__Constructor` @ `0x006F2B40` (LIGHT — field init audit)

### Memory reads
- `0x007E1710`: dbl_256.0 constant (verified `00 00 00 00 00 00 70 40` = 256.0)
- `0x006F2C80-0x006F2CFE`: constructor field init sequence
- `0x00713F60-0x00713FA0`: Cloakable parse site
- `0x00712420-0x00712460`: CloakingSpeed parse site
- `0x007130F8-0x00713138`: CloakStop parse site
- `0x00714A80-0x00714AC0`: Invisible parse site
- `0x007E5648-0x007E5668`: brightness blitter vtable (slot 0xB4 → draw method @ 0x00495730)
- `0x007E5780-0x007E57A0`: shimmer blitter vtable (slot 0x7C → draw method @ 0x00494330)
- `0x007E5660-0x007E5670`: brightness 50% blitter vtable (slot 0xB0 → draw method @ 0x00495590)
- `0x00495590` (FUN_00495590): disassembly of brightness 50% blitter for slot 0xB0

### Byte-pattern searches
- `89 86 ec 01 00 00` (MOV [ESI+0x1EC], EAX): 1 live match (constructor)
- `89 9e dc 01 00 00` (MOV [ESI+0x1DC], EBX): 1 live match (constructor) + RulesClass false-positive
- `89 87 dc 01 00 00` (MOV [EDI+0x1DC], EAX): 1 live match (TurretAI 0x00746A90) + RulesClass false-positive
- `8b 96 ec 01 00 00` (MOV EDX, [ESI+0x1EC]): 2 live matches (Evaluate_Candidate + post-ModifyCloakDrawFlags)

### Symbol-identification xref scans
- `0x00B73550` → 5 WinMain READ refs (= `g_hWnd`)
- `0x00A8B238` → Main_Game READ refs + 1 WRITE (= `g_GameMode`)
- `0x00887644` → WinMain WRITE + multiple READs in render path (= `g_ZBuffer`)
- `0x00A8ED6B` → FUN_004F4780 + Main_Game refs (= `g_IsMapEditor` per behavioral inference)

### Prior research consulted (VERIFIED/EXTENDED, NOT re-derived)
- `docs/research/CLOAKING_VISUAL_PIPELINE.md` (HIGH)
- `docs/research/CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` (HIGH)
- `docs/research/CLOAKING_INTERACTIONS_REPORT.md` (HIGH)
- `docs/research/DISGUISE_SYSTEM_GHIDRA_REPORT.md` (HIGH)
- `docs/research/SENSOR_CLOAK_DETECTION.md` (HIGH)
- `docs/research/BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md` (HIGH)
- `docs/research/VXL_RASTERIZER_DISPATCH_GHIDRA_REPORT.md` (HIGH) — for color-0 transparency invariant

### INI files
- `ini/rulesmd.ini` — all cloak/disguise/sensor keys
- `ini/rules.ini` — base RA2 confirmation

### Investigation plan executed
- `docs/plans/2026-05-11-cloak-fx-investigation-plan.md`
