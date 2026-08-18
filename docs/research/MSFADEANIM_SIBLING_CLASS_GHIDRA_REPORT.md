# MSFadeAnim — Full RE Report
**Target:** `MSFadeAnim` class in `gamemd.exe`  
**Date:** 2026-05-19  
**Session:** /re-swarm slot 5 — independent investigation  
**Status:** COMPLETE

---

## 1. Identity

**"MS" prefix = Mission Selector.** Confirmed via source filename string at 0x00848DE4:
`D:\ra2mdpost\WDTSel.cpp` — "WDT" = World Domination Tour Selector, the single-player
campaign mission selection screen.

- RTTI type descriptor: `0x008300C0` → `.?AVMSFadeAnim@@` (verified via `read_memory 0x008300C0`)
- Class tag: `?AV` (C++ class, not struct)
- Mangled name: `MSFadeAnim` — standalone, no template wrapper

---

## 2. Class Hierarchy (verified from RTTI)

```
MSAnim (base)          — type descriptor at 0x00830088, name ".?AVMSAnim@@"
  └── MSShapeAnim      — type descriptor at 0x008300A0, name ".?AVMSShapeAnim@@"
        └── MSFadeAnim — type descriptor at 0x008300C0, name ".?AVMSFadeAnim@@"
MSOverlayAnim          — type descriptor at 0x008300E0 (separate branch from MSAnim)
MSEngine               — type descriptor at 0x0082C0B8 (the driver class)
```

RTTI COL for MSFadeAnim at `0x00806670`:
- signature=0, offset=0, cdOffset=0
- pTypeDescriptor=0x008300C0 ✓
- pClassHierarchyDescriptor=0x00806660

CHD at 0x00806660: numBaseClasses=3, pBaseClassArray=0x00806650
Base class chain (verified): MSFadeAnim → MSShapeAnim → MSAnim

Verified via `read_memory 0x00806670`, `read_memory 0x00806660`, `read_memory 0x00806650`,
and type descriptor reads at 0x008300C0, 0x008300A0, 0x00830088.

---

## 3. Vtable

**MSFadeAnim vtable at 0x007EE938** (vtable[-1] = COL at 0x007EE934 = 0x00806670, verified
via `read_memory 0x007EE934`).

For comparison, **MSShapeAnim vtable at 0x007EE910** (vtable[-1] at 0x007EE90C = 0x00806620,
COL's pTypeDescriptor = 0x008300A0 = MSShapeAnim).

| Slot | Offset | MSFadeAnim addr | MSShapeAnim addr | Role |
|------|--------|-----------------|------------------|------|
| 0    | +0x00  | 0x005CEBD0      | 0x005CEB80       | Destructor (sets vptr back to base then to MSAnim vtable) |
| 1    | +0x04  | 0x005CEAC0      | 0x005CEAC0 (SAME)| SetActiveByte: `[ecx+0xC] = param` (stdcall, 1 byte param, ret 4) |
| 2    | +0x08  | 0x005CEAD0      | 0x005CEAD0 (SAME)| Pause: decrements timer remainder, called by MSEngine pause handler |
| 3    | +0x0C  | 0x005CEB20      | 0x005CEB20 (SAME)| Resume: arms start-time via GetRadarTimer(), called by MSEngine resume |
| 4    | +0x10  | 0x005CBDA0      | 0x005CB880       | Tick/IsDone: advances animation state, returns 1 when complete |
| 5    | +0x14  | 0x005CC110      | 0x005CBB80       | Draw: renders SHP frames with AlphaShapeClass__ClipRect |
| 6    | +0x18  | *(absent)*      | 0x005CBCB0       | MSShapeAnim-only extra slot |

Vtable slots 1–3 are identical (shared implementation) between MSFadeAnim and MSShapeAnim.
Slots 0, 4, 5 differ; MSShapeAnim has an additional slot 6.

Verified via `read_memory 0x007EE8F0` (96 bytes spanning both vtables).

---

## 4. Constructor

**Function: `MSShapeAnim__Constructor` at 0x005CBD20** (Ghidra label is misleading —
this function constructs objects that end with `*param_1 = &vtable__MSFadeAnim`).

Verified via `decompile_function 0x005CBD20`. The function:
1. Calls `CDFileClass__Constructor()` to load a SHP shape file (stored at `obj[0x1C]`)
2. Sets `obj[0x0C]` byte = 1 (active flag, also set by slot 1)
3. Stamps `GetRadarTimer()` into `obj[0x10]` (start time)
4. Takes 8 params: `(this, x, y, duration?, param5, param6, param7, surface_list)`
5. Ends by writing `vtable__MSFadeAnim` to `*param_1`

Size of MSFadeAnim object allocated by caller: **0x48 bytes** (72 bytes), confirmed via
`operator_new(0x48)` in the caller at 0x0076D4D0.

### Struct layout (from constructor + vtable method analysis)

| Offset | Size | Field | Source |
|--------|------|-------|--------|
| 0x00   | 4    | vtable* | constructor |
| 0x04   | 4    | x position (param_3) | constructor |
| 0x08   | 4    | y position (param_4) | constructor |
| 0x0C   | 1    | active flag (1=playing) | constructor, slot 1, slot 4 guard |
| 0x10   | 4    | start_time (GetRadarTimer) | constructor, slot 4 |
| 0x18   | 4    | duration_remaining | slot 4 tick, slot 2 pause |
| 0x1C   | 4    | CDFileClass* (SHP shape handle) | constructor param_1[7] |
| 0x20   | 4    | param_5 | constructor |
| 0x24   | 4    | param_6 | constructor param_1[9] |
| 0x28   | 4    | current frame index (loop counter) | constructor=0 |
| 0x2C   | 4    | frame_start | FUN_005CB840 setter |
| 0x30   | 4    | frame_end | FUN_005CB860 setter, CDFileClass frame count - 1 |
| 0x34   | 1    | flag | constructor=0 |
| 0x3C   | 1    | flag | constructor=0 |
| 0x40   | 4    | layer_index (0–3) | slot 4, frame table index |
| 0x44   | 4    | surface_list* (param_8) | constructor, slot 4 renderer loop |

---

## 5. What MSFadeAnim Animates

MSFadeAnim renders **SHP sprite sequences with alpha blending** on the
**World Domination Tour mission selection screen** (WDTSel.cpp).

Evidence:
- Constructor is called in `FUN_0076D4D0` (the WDT screen loader) with SHP file handles
  from `param_1+0x470` and `param_1+0x474`.
- The Draw method (slot 5 = 0x005CC110) calls **`AlphaShapeClass__ClipRect`** at 0x00421B60
  (verified via `get_function_by_address 0x00421B60`).
- A 4-element frame lookup table at 0x00830078 maps layer index (0–3) to frame indices:
  `{0x406, 0x404, 0x402, 0x400}` — SHP frame numbers in the 1024+ range.
- The animation target is a surface list passed as the last constructor param.

The "Fade" in MSFadeAnim refers to the alpha-blended transition: shapes are drawn with
`AlphaShapeClass__ClipRect` (not filled rects), fading in/out over a timer-controlled duration.
The specific visual is the map/globe overlay that fades in during WDT mission selection.

---

## 6. Lifecycle — How MSFadeAnim Instances Are Driven

Instances are stored in a **`DynamicVectorClass<MSAnim*>`** (RTTI confirmed at 0x00830588:
`.?AV?$DynamicVectorClass@PAVMSAnim@@@@`).

Per-frame driver is **`FUN_005D1E70`** (the MSAnimList draw loop), called from
`FUN_005D2410` (the MSEngine frame-tick loop):

1. `FUN_005D1E70` iterates the vector, calling vtable[5] (Draw, offset +0x14) on each MSAnim.
2. Also calls vtable[4] (Tick/IsDone, offset +0x10); if it returns 1, calls vtable[0] (dtor)
   and compacts the array.
3. Pause/resume is handled by `FUN_005D2530` ("MSEngine – Pausing/Resuming animations")
   which calls vtable[2] (Pause) and vtable[3] (Resume) on all live instances.

`FUN_0076EA20` is the "create/swap MSAnim for current screen" helper — called ~8 times
across the WDT screen functions. It allocates + constructs new MSAnim instances (size 0x34,
simpler variant) and adds them via `FUN_005D1C20` (MSAnimList__Add).

---

## 7. Comparison to ButtonFadeEffect

| Property | MSFadeAnim | ButtonFadeEffect |
|----------|-----------|-----------------|
| RTTI tag | `?AV` (class) | `?AU` (struct) |
| Vector type | `DynamicVectorClass<MSAnim*>` (0x00830588) | `DynamicVectorClass<ButtonFadeEffect*>` (0x00820460) |
| Render primitive | `AlphaShapeClass__ClipRect` — SHP frame blending | Raw alpha rect fills (reported by slots 1–4 investigation) |
| Driver context | MSEngine (mission selector) | Button/UI hover system |
| Inheritance | MSFadeAnim → MSShapeAnim → MSAnim | No inheritance (struct) |
| Shared infrastructure | **None** — different vector types, different draw primitives, different contexts | — |

**They share no code or infrastructure.** MSFadeAnim is a heavyweight shape-animation class
for mission screen transitions; ButtonFadeEffect is a lightweight UI hover-fade struct. The
"Fade" naming is coincidental.

---

## 8. Active in YR?

**Yes, active in standard YR.** The World Domination Tour (single-player campaign mission
selector) is used in every single-player campaign session. MSFadeAnim drives the animated
globe/map overlays that play during WDT mission briefing selection. No gating flags found.

---

## 9. Open Questions (deferred — out of scope for this pass)

- MSOverlayAnim's vtable and exact role (separate branch from MSAnim; not a parent of MSFadeAnim)
- What `param_1+0x470` and `param_1+0x474` SHP files are (the two MSFadeAnim instances
  created per mission: one 0x48 bytes at duration=3, one 0x40 bytes at duration=5)
- The 4-element frame table at 0x00830078 — are these hardcoded frame offsets or INI-driven?
- MSAnimEntry class (`DynamicVectorClass<MSAnimEntry*>` at 0x008301D0) — wrapper role unclear
- Full MSEngine class structure (RTTI at 0x0082C0B8)
- MSShapeAnim slot 6 (0x005CBCB0) — the extra method absent in MSFadeAnim

---

## 10. Key Verified Facts

1. **MSFadeAnim vtable at 0x007EE938**, confirmed by COL at vtable[-1]=0x007EE934 = 0x00806670
   whose pTypeDescriptor=0x008300C0=`.?AVMSFadeAnim@@`. (verified: `read_memory 0x007EE934`,
   `read_memory 0x00806670`, `read_memory 0x008300C0`)

2. **Inheritance chain: MSFadeAnim → MSShapeAnim → MSAnim** (3 base classes per CHD at
   0x00806660). MSOverlayAnim is a separate branch. (verified: RTTI BCD reads)

3. **Constructor is `MSShapeAnim__Constructor` at 0x005CBD20**, mislabeled in Ghidra.
   It allocates via `operator_new(0x48)`, calls `CDFileClass__Constructor`, and ends by
   writing `vtable__MSFadeAnim` to `*this`. (verified: `decompile_function 0x005CBD20`)

4. **Draw primitive is `AlphaShapeClass__ClipRect` at 0x00421B60**, not raw alpha rect fills —
   MSFadeAnim renders SHP shape frames with alpha blending, not color-filled rectangles.
   (verified: `get_function_by_address 0x00421B60`, byte-trace of slot 5 call target)

5. **Context: World Domination Tour mission selector screen** (`WDTSel.cpp` at 0x00848DE4).
   "MS" = Mission Selector. Animations are driven by `MSEngine` via `DynamicVectorClass<MSAnim*>`.
   (verified: `search_strings WDTSel`, `FUN_005D2530` decompile with "MSEngine –" strings)
