# PixelFXClass — Ghidra Research Report

Research date: 2026-04-21
Method: RTTI TypeDescriptor, constructor + tick + render-site decompilation, binary
color-table dump at `0x008367C8`, existing-doc cross-reference to
`TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md §2.24` and `SHROUD_FOG_RENDERING_PIPELINE.md`.

---

## Top-line verdict: LIVE

PixelFXClass is **live in every YR skirmish**, gated only on:
- Global flag `DAT_00A8EB78 != 0` (appears to be a user/rules option, likely
  "hardware blit" or "HiColor effects")
- Primary surface being 16-bit color (`PrimarySurface->GetBpp() == 2`)

When both conditions are true, PixelFX generates a per-cell twinkle on ore/gem tiles.
Dormant when run in 32-bit color mode or when the global flag is off.

### Evidence of liveness

- `DrawPixelFXSparkles @ 0x006D7840` is **step #24** of the main tactical render
  pipeline (see `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md §2.24`), running every
  frame after unit/building draws and before the EVA floating-text overlay.
- The pipeline position means it executes unconditionally each frame; the internal
  guards determine whether pixels are actually written.
- Only one caller of the class constructor; only one tick/update site; both inside
  the main render function — so this is a pure visual-only system with zero gameplay
  effect.

---

## Purpose

Per-cell **ore/gem sparkle** animation: writes a single 16-bit RGB pixel at a
jittered position on each ore/tiberium-containing cell per frame, pulsing between two
colors over ~2 seconds to simulate glinting mineral resources. One `PixelFXClass`
instance is stored per cell in `CellClass+0xFC` and persists across frames.

This is **NOT** a generic per-unit spark / muzzle-flash / impact-spark system.
Projectile sparks, weapon flashes, and explosion sparks are handled elsewhere
(ParticleSystemClass, AnimClass, `FUN_0048A620` for IC/ForceShield hits). PixelFX is
exclusively ore/gem twinkle.

---

## RTTI evidence

- TypeDescriptor string `.?AVPixelFXClass@@` at **`0x00836828`** (TD struct at
  `0x00836820`).
- Only one constructor pair labeled in the binary (already named):
  - `PixelFXClass__Constructor @ 0x00631E10` (default, arg=0)
  - `PixelFXClass__Constructor @ 0x00631E30` (arg is table index)
- Vtable symbol `vtable__PixelFXClass` set by ctor (single vtable, non-derived).

---

## Struct layout (VERIFIED — 0x3C bytes)

Verified from `PixelFXClass__Init @ 0x00631D40` and `DrawPixelFXSparkles @ 0x006D7840`.
Allocation size is `operator_new(0x3C)` (60 bytes). `param_1` in Ghidra is `int` so
offsets below are raw byte offsets.

| Offset | Type   | Field                   | Notes                                                    |
|--------|--------|-------------------------|----------------------------------------------------------|
| +0x00  | ptr    | vtable                  | `vtable__PixelFXClass`                                   |
| +0x04  | int    | CurrentR                | Interpolated R (0..255-ish), recomputed per tick         |
| +0x08  | int    | CurrentG                | Interpolated G                                           |
| +0x0C  | int    | CurrentB                | Interpolated B                                           |
| +0x10  | int    | PhaseAccumulator        | 0..0x2000, wraps via bit-12 parity → ping-pong           |
| +0x14  | int    | PhaseStep               | Random in `[min .. max]` from the color table            |
| +0x18  | int    | ColorA_R                | One endpoint of the ping-pong (dimmed by random jitter)  |
| +0x1C  | int    | ColorA_G                |                                                          |
| +0x20  | int    | ColorA_B                |                                                          |
| +0x24  | int    | ColorB_R                | Other endpoint                                           |
| +0x28  | int    | ColorB_G                |                                                          |
| +0x2C  | int    | ColorB_B                |                                                          |
| +0x30  | int    | PixelOffsetX            | Random `[-0x1F .. +0x20]` cell-relative jitter           |
| +0x34  | int    | PixelOffsetY            | Random `[-0x0F .. +0x10]`                                |
| +0x38  | int    | WaitTimer               | Random masked value, ticks down by elapsed ms            |

Field tags: `CurrentR/G/B` and `ColorA/B` are **VERIFIED** by the RGB pack at the end
of DrawPixelFXSparkles. `PhaseAccumulator`, `PhaseStep`, `WaitTimer`, and the two
`PixelOffset*` fields are VERIFIED from `PixelFXClass__Update_Color @ 0x00631E50`
and `PixelFXClass__Tick_Timer @ 0x00631EE0`.

### Color table @ `0x008367C8` — 2 × 0x28-byte entries (VERIFIED by memory dump)

```
Entry 0 (index 0, passed as arg=0 from ctor):           // "pure tiberium" entry?
  +0x00  ColorA_R         = 0x9E  (158)
  +0x04  ColorA_G         = 0x9E  (158)
  +0x08  ColorA_B         = 0xE0  (224)   → light cyan/blue
  +0x0C  ColorB_R         = 0x28  (40)
  +0x10  ColorB_G         = 0x28  (40)
  +0x14  ColorB_B         = 0x50  (80)    → dark blue
  +0x18  JitterShift      = 0x05             ((1<<5)-1 = 0x1F per-channel darken mask)
  +0x1C  WaitMask         = 0x0FFF           (AND-mask for initial WaitTimer)
  +0x20  PhaseStepMax     = 0x0C  (12)
  +0x24  PhaseStepMin     = 0x03  (3)

Entry 1 (index 1, passed as arg=1 from ctor):           // "ore" entry
  +0x00  ColorA_R         = 0xFF  (255)
  +0x04  ColorA_G         = 0xFF  (255)
  +0x08  ColorA_B         = 0xF0  (240)   → bright near-white / yellow
  +0x0C  ColorB_R         = 0xB0  (176)
  +0x10  ColorB_G         = 0x90  (144)
  +0x14  ColorB_B         = 0x00  (0)     → dark gold
  +0x18  JitterShift      = 0x00             (no per-channel darkening)
  +0x1C  WaitMask         = 0x0FFF
  +0x20  PhaseStepMax     = 0x1E  (30)
  +0x24  PhaseStepMin     = 0x0F  (15)
```

The caller `DrawPixelFXSparkles` selects the table entry with
`PixelFXClass__Constructor(cell_tiberium_value != 0)`, i.e., it passes 1 for any
tiberium-valued cell and 0 otherwise.

**Interpretation notes:**
- RA2 ore (`[Tiberium Type 1 "Ore"]` = yellow) would match entry 1 (yellow gold
  ColorA, darker gold ColorB).
- Gems (`[Tiberium Type 2 "Gems"]` = blue/purple) are typed as `value != 0`, so they
  ALSO use entry 1 — which is wrong for gem color. Therefore **entry 0 must be
  something else** (possibly empty/unmarked tiberium cells, or a fallback for cells
  with type==2 but value==0). See open question below.

---

## Constructor / initialization

Two entry points share the same init:

```
PixelFXClass__Constructor(tableIndex) {
    this->vtable = &vtable__PixelFXClass;
    PixelFXClass__Init(this, tableIndex);
}
```

`PixelFXClass__Init @ 0x00631D40` (renamed this pass):
```c
void Init(PixelFXClass* p, int tableIndex) {
    const int* row = &g_PixelFxColorTable[tableIndex * 0x28];  // = 0x008367C8 + i*0x28

    uint r = rand();
    p->PixelOffsetX = (r & 0x3F) - 0x1F;    // [-0x1F .. +0x20]
    p->PixelOffsetY = ((r >> 5) & 0x1F) - 0x0F;  // [-0x0F .. +0x10]

    p->ColorA_R = row[0];                   // base color A
    p->ColorA_G = row[1];
    p->ColorA_B = row[2];

    if (row[6] /* JitterShift */ != 0) {
        uint mask = (1 << row[6]) - 1;      // e.g., (1<<5)-1 = 0x1F
        uint rj = rand();
        p->ColorA_R -= (mask & rj);
        rj >>= row[6];
        p->ColorA_G -= (mask & rj);
        rj >>= row[6];
        p->ColorA_B -= (mask & rj);
    }

    p->ColorB_R = row[3];  p->CurrentR = row[3];   // seed Current = ColorB
    p->ColorB_G = row[4];  p->CurrentG = row[4];
    p->PhaseAccumulator = 0;
    p->ColorB_B = row[5];  p->CurrentB = row[5];

    p->PhaseStep = row[9] + (rand() % (row[8] - row[9] + 1));  // random [min..max]
    p->WaitTimer = rand() & row[7];                             // random & mask
}
```

---

## Vtable

Set in the constructor — `vtable__PixelFXClass` is the single assigned vtable, but
this report did not enumerate individual slots because the render path reads the POD
fields directly (no virtual calls). The vtable exists only to satisfy RTTI; no
virtual dispatch was observed in the use sites.

---

## Lifecycle

1. **Creation (lazy, per-cell).** Inside `DrawPixelFXSparkles`, for each cell within
   the diamond-pattern viewport sweep that is (a) ore/tiberium and (b) has
   `CellClass+0xFC == NULL`, `new PixelFXClass(isOre)` is allocated and stored in
   `CellClass+0xFC`. The sparkle lives for the life of the cell (tied to the cell,
   not per-frame).
2. **Expiry / re-seed.** If `PhaseAccumulator > 0x1FFF`, the tick code re-invokes
   `PixelFXClass__Init` (re-roll colors + timer) in place. Instances are never
   explicitly freed during gameplay — they live until the cell/map is destroyed.
3. **Per-frame tick + draw.** Every frame (if render gates pass):
   - `PixelFXClass__Tick_Timer(p, elapsed_ms)` — decrements `WaitTimer` by `elapsed_ms`.
     Returns true when `WaitTimer` goes non-positive, meaning "animate this frame".
   - When true, `PixelFXClass__Update_Color(p, elapsed_ms)` advances
     `PhaseAccumulator` and recomputes `CurrentR/G/B` as a linear interpolation
     between `ColorA` and `ColorB` using a triangle wave derived from bits of the
     accumulator.
   - One pixel is written to the primary surface at
     `(cellScreenX + p->PixelOffsetX, cellScreenY + p->PixelOffsetY)` using the
     current 16-bit RGB packed via `g_DD_R/G/B Loss/Shift`.

---

## Rendering path (VERIFIED)

This is **per-pixel direct surface writing**, not a sprite/quad draw. See
`DrawPixelFXSparkles @ 0x006D7840`:

1. **Render gates** (all must pass):
   - `FUN_0055AF60() <= DAT_00ABCD44` — a shroud-level check (probably "only run when
     player can see map", i.e., not during shroud-full blackness)
   - `(*g_PrimarySurface)->GetBpp() == 2` — **16-bit color surface only** (dormant in
     32-bit mode)
   - `DAT_00A8EB78 != 0` — global "enable pixel effects" flag (see open questions)
   - `(*g_PrimarySurface)->Lock(0, 0)` succeeds
2. **Elapsed time** is computed via `timeGetTime()` minus last-call timestamp,
   clamped to 1000 ms.
3. **Viewport diamond sweep.** Starting from the viewport center cell, iterates a
   diamond region of size `(viewport_height/0xF + 0x11) x (viewport_width/0x3C + 4)`
   cells.
4. **Per-cell filter.** Only draws for cells where:
   - `cell->flags & 0x10` (has overlay)
   - `FUN_00487950() == 0` (not in shroud / not some mask)
   - `(cell->TerrainType == 2 || TiberiumValue != 0)` — ore or gem cell
   - `cell->field_0xE4 == 0` (no building on cell)
   - `(cell->flags_0x140 & 0x1000) == 0` (some "no pixel" flag)
5. **Instance lifecycle.** If `cell->field_0xFC == NULL`, allocate a new
   `PixelFXClass` and store it. Else if that instance's `PhaseAccumulator > 0x1FFF`,
   re-init in place.
6. **Tick + Draw.**
   ```c
   if (PixelFXClass__Tick_Timer(fx, elapsed_ms)) {
       ComputeScreenPos(&screen);
       int x = screen.cx + fx->PixelOffsetX;
       int y = screen.cy + fx->PixelOffsetY;
       if (x,y in viewport) {
           PixelFXClass__Update_Color(fx, elapsed_ms);
           ushort rgb16 = ((fx->CurrentB >> DD_BLoss) << DD_BShift)
                        | ((fx->CurrentG >> DD_GLoss) << DD_GShift)
                        | ((fx->CurrentR >> DD_RLoss) << DD_RShift);
           *((ushort*)(surface_ptr + y*pitch + x*2)) = rgb16;
       }
   }
   ```
7. **Unlock** primary surface.

This is a **separate render pass** that runs **after** normal sprite drawing and
**before** floating-text overlays. It bypasses the sprite/quad/palette systems
entirely and writes raw 16-bit RGB values.

---

## Numeric constants extracted

| Constant                           | Value       | Source                                          |
|------------------------------------|-------------|-------------------------------------------------|
| Struct size                        | 0x3C (60 B) | `operator_new(0x3C)` in `DrawPixelFXSparkles`   |
| Color table entries                | 2           | Array at `0x008367C8`, size 0x28 each           |
| PixelOffsetX range                 | -31..+32    | Init jitter mask                                |
| PixelOffsetY range                 | -15..+16    | Init jitter mask                                |
| Phase accumulator max              | 0x2000      | Clamp in `Update_Color`                         |
| Phase re-init threshold            | 0x1FFF      | Caller decision in `DrawPixelFXSparkles`        |
| Interpolation resolution           | 0x1000      | Triangle-wave period in `Update_Color`          |
| Elapsed-time clamp                 | 1000 ms     | `DrawPixelFXSparkles`                           |
| Tiberium color A (pure)            | `#9E9EE0`   | Table entry 0                                    |
| Tiberium color B (pure)            | `#282850`   | Table entry 0                                    |
| Ore color A                        | `#FFFFF0`   | Table entry 1                                    |
| Ore color B                        | `#B09000`   | Table entry 1                                    |

---

## Call graph

```
DrawPixelFXSparkles @ 0x006D7840
  ├─ PixelFXClass__Constructor(0 or 1)  @ 0x00631E30  (size 0x3C alloc)
  │    └─ PixelFXClass__Init           @ 0x00631D40   (reads table @ 0x008367C8)
  ├─ PixelFXClass__Init                @ 0x00631D40   (re-init in place)
  ├─ PixelFXClass__Tick_Timer          @ 0x00631EE0   (returns bool)
  ├─ PixelFXClass__Update_Color        @ 0x00631E50   (recomputes Current RGB)
  └─ direct 16-bit surface write (no CC_Draw_Shape call)
```

**Single-caller chain.** No other function in the binary constructs, ticks, or reads a
PixelFXClass instance. No callers from weapon fire, hit impact, muzzle flash, or
unit draw.

---

## INI keys

**None directly.** PixelFXClass has no ReadINI function and no associated INI keys.
The global enable flag `DAT_00A8EB78` is likely set from a rules/options mechanism
but is not one of the flash/FX-named INI strings searched. Related surface flags
observed in the binary (not verified this pass):
- `DAT_00A8EB78` is also checked by:
  - `FUN_0048A620` (IC/ForceShield spark anim)
  - Several other render-gating sites (28 xrefs total)

Candidate: this appears to be `[General] PixelFX` or a similar options flag. A
future pass should walk back from `DAT_00A8EB78` writes to the ReadBool site.

The color table at `0x008367C8` contains **hardcoded RGB values** — no INI-driven
color override.

---

## Open questions

1. **Why does the table have 2 entries when all tiberium-valued cells pass
   `iVar11 != 0`?** The caller condition is
   `(cell->type == 2 || tiberiumValue != 0)` AND the ctor arg is `(tiberiumValue != 0)`.
   For a gem cell with type=2, if its `Get_Tiberium_Value()` returns non-zero (which
   it should, since gems have value), both conditions route to entry 1 (yellow/gold).
   Entry 0 (blue) is reached only when `type==2 && value==0` — which suggests
   "temporarily-valueless tiberium cell" (e.g., harvested-dry gem tile that's still
   marked). Worth verifying by stepping through or checking the OverlayToTiberiumIndex
   map.
2. **What is `DAT_00A8EB78`?** Likely `[General] TiberiumGrowthEffect`, `[General]
   PixelFX`, or an options-dialog setting. 28 xrefs — worth a dedicated mini-pass.
3. **Are the RGB values color-depth accurate?** The surface-write path shifts by
   `g_DD_R/G/B_Loss` / `Shift`, which are DirectDraw surface masks. Values are correct
   on 16-bit 565/555 surfaces; 32-bit is gated out. Rust wgpu engine should
   pre-compute the 8-bit RGB output before shift.
4. **Z-ordering.** The pass runs AFTER units/buildings draw, so sparkles render on
   top of foreground sprites — this may differ from the intuitive "sparkles should
   be underneath units on the same cell" expectation. Check against stock YR
   behavior.
5. **Lifetime of cell-owned instances.** Never observed freed in code walked this
   pass. If `cell->field_0xFC` is reset during cell cleanup (map teardown, ore
   depletion → no overlay) it would leak. Worth a follow-up xref sweep on
   `cell + 0xFC` writes.

---

## Ghidra functions labeled this pass

| Address     | Old name              | New name                      | Purpose                                           |
|-------------|-----------------------|-------------------------------|---------------------------------------------------|
| `0x00631D40`| `FUN_00631d40`        | `PixelFXClass__Init`          | Zero-seed struct from color table entry           |
| `0x00631E50`| `FUN_00631e50`        | `PixelFXClass__Update_Color`  | Triangle-wave interpolation between ColorA/B      |
| `0x00631EE0`| `FUN_00631ee0`        | `PixelFXClass__Tick_Timer`    | Decrement WaitTimer by elapsed ms, return bool    |
| `0x006D7840`| `FUN_006d7840`        | `DrawPixelFXSparkles`         | Per-frame diamond sweep + 16-bpp pixel write      |

Already-labeled functions (pre-existing):
- `0x00631E10`, `0x00631E30` — `PixelFXClass__Constructor` (two entry points)

Program saved after renames.

---

## Confidence summary

| Claim                                                                     | Confidence | Evidence                        |
|---------------------------------------------------------------------------|------------|---------------------------------|
| Live in YR, runs every frame (pre-gate)                                   | VERIFIED   | Tactical render pipeline §2.24  |
| Exclusively ore/gem twinkle — NOT a generic spark system                  | VERIFIED   | Single caller, cell-overlay gate|
| Struct size = 0x3C, field layout                                          | VERIFIED   | `new(0x3C)` + Init + Draw       |
| Color table at `0x008367C8`, 2 × 0x28-byte rows                           | VERIFIED   | Memory dump + Init offsets      |
| Per-pixel 16-bit surface write, not a sprite/quad draw                    | VERIFIED   | Direct surface-pointer write    |
| Separate render pass, post-unit/pre-text                                  | VERIFIED   | Pipeline step 24 position       |
| One instance per ore cell, stored in `CellClass+0xFC`                     | VERIFIED   | Field write in Draw             |
| Dormant on 32-bit color surfaces                                          | VERIFIED   | `GetBpp() == 2` gate            |
| Entry 0 vs Entry 1 semantic mapping (pure vs ore)                         | MEDIUM     | See open question 1             |
| `DAT_00A8EB78` is the `[Options] DetailLevel` knob (0..2)                 | VERIFIED   | Round 2 — see below             |

---

## Follow-up investigation (round 2) — 2026-04-21

### Q2: What writes to the gate flag `DAT_00A8EB78`? — RESOLVED

`DAT_00A8EB78` is the **`[Options] DetailLevel` knob** — the classic RA2 "Detail
Level" option from the in-game Options menu. It is **NOT** a color-depth-derived
flag and **NOT** a DirectDraw init flag.

**Evidence chain.**

1. **`get_xrefs_to DAT_00A8EB78`** returns 28 sites. Of these, only **two are
   WRITEs**:
   - `0055fae0 in OptionsClass__ApplyFromLauncherDialog` (formerly `FUN_0055faa0`)
   - `004e1f10 in OptionsClass__ApplyFromInGameDialog`   (formerly `FUN_004e1de0`)
   (The third "WRITE" at `0x005fa370` turned out to be unrelated — that address
    falls inside `OptionsClass__SetDefaults @ 0x005fa350` which writes 800/600/etc.
    to nearby fields, not to DAT_00A8EB78 itself. Ghidra's xref listing appears to
    have mis-categorised it.)

2. **Both writer functions are dialog-box handlers.** Both query
   `GetDlgItem(hwnd, 0x52B)` (the Detail Level combo box / slider control) and both
   apply the identical formula:
   ```c
   LVar2 = SendMessageA(hwnd_0x52B, 0x400, 0, 0);   // CB_GETCURSEL or TBM_GETPOS
   uVar3 = -(uint)(LVar2 != 0) & 2;                 // → 0 or 2
   if (DAT_00A8EB78 != uVar3) {
       DAT_00A8EB78 = uVar3;
       FUN_004AE450();                              // map-wide cell refresh
   }
   ```
   So the dialog stores **only `0` or `2`** (two-state toggle), but the INI
   parser accepts the full `0..2` range.

3. **Caller chain confirms these are user-facing dialogs.** Both parent functions
   also call `OptionsClass__WriteToINI` at the end — so the value is persisted to
   `RA2MD.INI` whenever the user clicks OK.
   - `OptionsClass__ApplyFromLauncherDialog` is called from
     `OptionsClass__ShowLauncherDialog @ 0x0055FC80` (pre-game launcher options)
   - `OptionsClass__ApplyFromInGameDialog` is called from
     `OptionsClass__ShowInGameDialog @ 0x004E1D00` (in-game ESC → Options)

4. **INI key confirmed as `DetailLevel`.** String `DetailLevel` is at
   `0x0081855C`. `get_xrefs_to 0x0081855C` returns three readers, the most
   important being:
   ```
   From 005fa782 in OptionsClass__ReadFromINI [DATA]     — reads the knob
   From 005fadd3 in OptionsClass__WriteToINI [DATA]      — writes the knob
   From 00428081 in AnimTypeClass__ReadINI  [DATA]       — per-anim threshold
   ```
   `OptionsClass__ReadFromINI @ 0x005FA780` reads:
   ```c
   uVar6 = CCINIClass__ReadInt("Options", "DetailLevel", param_1[6]);
   if (1 < (int)uVar6) uVar6 = 2;                // clamp high
   uVar6 = uVar6 & ((int)uVar6 < 1) - 1;         // clamp negative to 0
   param_1[6] = uVar6;                           // OptionsClass+0x18
   Register_heap_pool("DetailLevel = %d\n", uVar6);   // log
   ```
   Section is **`[Options]`** (string `s_Options_008254dc`), key is **`DetailLevel`**,
   and values are clamped to **`[0, 2]`**. Stored in `OptionsClass` at byte offset
   `0x18` (param_1[6]).

5. **`Register_heap_pool` debug trace confirms semantics.** In
   `OptionsClass__ShowInGameDialog` the function logs:
   ```
   "GameControls: GameSpeed = %d, ScrollRate = %d, Detail = %d"
   ```
   passing `DAT_00A8EB60` (GameSpeed), `DAT_00A8EB70` (ScrollRate), `DAT_00A8EB78`
   (Detail). This is the third parameter: **Detail** = `DAT_00A8EB78`.

**Semantics.**

- `DetailLevel=0` (the default in YR) → minimal visual effects. `DrawPixelFXSparkles`
  is GATED OUT (so no ore/gem twinkle). `LineTrail__SetColorDecrement` DOUBLES the
  decrement rate (so line trails fade faster). Similar gating in 25 other render
  sites.
- `DetailLevel=2` (maximum) → full visual effects enabled. PixelFX runs; LineTrail
  decrement is stored as-received.
- `DetailLevel=1` is accepted by the INI parser but the in-game dialog only produces
  `0` or `2`. Value 1 behaves identically to 0 for the flag-is-nonzero check in
  `DrawPixelFXSparkles`, but may differ in other sites that do strict equality.

**Side-effect.** Changing the Detail Level calls `FUN_004AE450`, which iterates every
cell on the map (`MapClass::CellIterator_Next`) and calls `FUN_00483E30` on each
(likely a lighting/color-buffer refresh: `0, 0x10000, 0, 1000, 1000, 1000`). This
is why the game briefly stutters when you change the setting mid-game.

**Implication for our Rust engine.**

- The Pixel FX sparkle system should be gated on a rules/options setting equivalent
  to `DetailLevel != 0`, NOT on hardware color depth.
- We control the rendering surface format — if we want parity with vanilla YR, we
  should add an in-engine `[Options] DetailLevel` knob (0..2) and:
  - Gate PixelFX sparkle drawing on `DetailLevel >= 1` (conservatively — the YR
    default of `0` hides it)
  - Skip the 16-bpp `GetBpp() == 2` check — our wgpu output is always 32-bit, so
    color-depth gating is meaningless.
- Also gate AnimTypeClass animations with their per-anim `DetailLevel` threshold
  (from `art.ini`) against the global `DetailLevel` setting.

### Ghidra labels applied (round 2)

| Address     | Old name              | New name                               |
|-------------|-----------------------|----------------------------------------|
| `0x0055FAA0`| `FUN_0055faa0`        | `OptionsClass__ApplyFromLauncherDialog`|
| `0x0055FC80`| `FUN_0055fc80`        | `OptionsClass__ShowLauncherDialog`     |
| `0x004E1DE0`| `FUN_004e1de0`        | `OptionsClass__ApplyFromInGameDialog`  |
| `0x004E1D00`| `FUN_004e1d00`        | `OptionsClass__ShowInGameDialog`       |
| `0x005FA350`| `FUN_005fa350`        | `OptionsClass__SetDefaults`            |

Program saved.
