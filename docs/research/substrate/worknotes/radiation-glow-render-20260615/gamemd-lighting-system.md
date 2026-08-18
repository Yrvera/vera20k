# gamemd Dynamic-Lighting / LightSource System — Worknote

**Lane:** general dynamic-lighting infrastructure that radiation glow plugs into.
**Goal:** ground the wgpu render design in gamemd's *observable* lighting compositing
(what the player sees), not its C++ plumbing.
**Date:** 2026-06-15
**Authority:** binary → Ghidra → docs/research → ini. Live decompiles cited inline this
session are tagged VERIFIED-FROM-BINARY (2026-06-15). Claims taken from prior verified
docs without a fresh live decompile this session are tagged VERIFIED-FROM-DOC and name
the doc. Inferences are tagged INFERRED.

This extends prior research; it does NOT redo it. Anchor docs (all `[ghidra/verified]`):
- `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md` — the per-cell accumulation formula.
- `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md` — cell light bundle + palette-profile cache.
- `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md` — which draw paths read which cell fields.
- `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md` — toggle → affected-cell dirty scan + per-tick drain.
- `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md` — building-lamp lifecycle.
- `RADIATION_EMP_GHIDRA_REPORT.md` §1.11 — radiation glow uses LightSourceClass.

---

## TL;DR observable model (what we must reproduce in wgpu)

gamemd has ONE dynamic-lighting primitive — `LightSourceClass` — used by **both** map
lamp-post buildings **and** radiation sites (and EMP). A LightSource is a colored point
light (world position + radius + signed intensity + signed RGB tint). When a source is
enabled/disabled/moved/updated, the engine scans the cells in its radius and **recomputes
each cell's lighting from scratch**: it re-sums map ambient + every currently-active
source. The per-cell result is stored on the cell as (a) a small set of integer "light
bundle" fields the renderer reads as brightness/Z-scalars, and (b) a pointer to a cached
**`LightConvertClass`** palette-conversion object keyed by the cell's normalized RGB. The
terrain tile blitter and every sprite draw (overlays, terrain objects, infantry/building
SHPs, some anims) read that cell light state, so a single colored light visibly tints the
ground tile *and* the units standing on it identically. For wgpu we reproduce that
observable result (per-cell RGB tint + brightness applied to terrain and sprites alike),
not the palette-object cache.

---

## 1. The LightSourceClass primitive

**LightSourceClass is an `AbstractClass`-derived object, size `0x4C` bytes** (radiation
path allocates `operator new(0x4c)`, VERIFIED-FROM-BINARY at `0x0065B77C`). Inserted into
a global source vector on construction; iterated each time a cell needs recompute.

### 1.1 Constructor + field layout — VERIFIED-FROM-BINARY (decompile_function `0x00554760`; reads cross-checked via disassemble `0x004842B0`)

`LightSourceClass__Constructor @ 0x00554760` (`__thiscall`, this in ECX, 8 stack args).
Decompiler indexes `param_1` as `undefined4*`, so dword index `[N]` = byte offset `N*4`:

| Byte offset | Field | Set by ctor | Meaning |
|---|---|---|---|
| `+0x24` | intensity | `param_6` (`[9]`) | signed additive brightness contribution, milli-units (1000 = 1.0) |
| `+0x28` | red tint | `param_7` (`[10]`) | signed additive R contribution, milli-units |
| `+0x2C` | green tint | `param_8` (`[0xb]`) | signed additive G contribution, milli-units |
| `+0x30` | blue tint | `param_9` (`[0xc]`) | signed additive B contribution, milli-units |
| `+0x34` | detail threshold | literal `2` (`[0xd]=2`) | source contributes only if `+0x34 <= [Options]DetailLevel` |
| `+0x38` | world X | `param_2` (`[0xe]`) | leptons |
| `+0x3C` | world Y | `param_3` (`[0xf]`) | leptons |
| `+0x40` | world Z | `param_4` (`[0x10]`) | leptons (height; NOT used in the XY radius test) |
| `+0x44` | visibility radius | `param_5` (`[0x11]`) | leptons |
| `+0x48` | active flag (byte) | literal `0` (`[0x12]=0`) | **starts INACTIVE**; a separate enable call turns it on |

Constructor also appends `this` to the global source vector at `DAT_00ABCA14` with count
`DAT_00ABCA20` (verified: tail of `0x00554760` writes `*(DAT_00abca14 + count*4) = this`,
increments count). `0x00484180` iterates exactly this vector. VERIFIED-FROM-BINARY.

> `+0x38/+0x3C/+0x40` are read as a contiguous 3-int coord block at `0x004842E2`
> (`LEA EAX,[ESI+0x38]` then reads `[ECX]`,`[ECX+4]`,`[ECX+8]`). The radius test uses only
> X and Y; Z is not part of the falloff distance. VERIFIED-FROM-BINARY.

### 1.2 Enable / disable / update wrappers — VERIFIED-FROM-BINARY (decompile `0x00554A60`, `0x00554AA0`)

These are the *only* way a source becomes visible — and each one **immediately schedules
the affected cells dirty** so the change shows the same tick.

- **Enable `0x00554A60`** (Ghidra label "CreateProductionAnim" is STALE/WRONG — it is the
  LightSource enable wrapper): if `+0x48 == 0`, set `+0x48 = 1`, then call dirty-scan
  `0x00554AF0(mode)`. VERIFIED-FROM-BINARY.
- **Disable `0x00554A80`**: if `+0x48 != 0`, set `+0x48 = 0`, then call `0x00554AF0(mode)`.
  VERIFIED-FROM-DOC (`LIGHTSOURCE_DIRTY_SCHEDULING`, `LIFECYCLE` reports; symmetric to enable).
- **Update `0x00554AA0`**: writes new intensity (`+0x24`) and RGB (`+0x28/+0x2C/+0x30`),
  and **only if already active (`+0x48 != 0`)** calls `0x00554AF0(mode)`.
  VERIFIED-FROM-BINARY (decompile `0x00554AA0`). This is the per-frame radiation-fade path.

The active flag is flipped **before** the dirty-scan, so recompute reads the new state.
VERIFIED-FROM-DOC (`LIGHTSOURCE_DIRTY_SCHEDULING` OQ-09).

### 1.3 Lifecycle / registration (building lamps) — VERIFIED-FROM-DOC (`LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD`)

Building lamp source lives at `BuildingClass+0x614` (nullable cache, not a save pointer):
allocated on `Unlimbo`/`OnConstructionComplete` only when `BuildingTypeClass+0xE34`
(`LightIntensity`) is nonzero, then immediately enabled with mode 0. Online/offline/capture
toggle it; damage-case-4 and sell disable it before teardown; destructor deletes + zeros
it; `Load` zeroes it (rebuild after load, don't serialize the handle). **Not in scope for
radiation glow**, but it proves the primitive is shared and the enable/disable cadence is
event-driven and immediate.

---

## 2. Radiation glow IS this primitive — VERIFIED-FROM-BINARY (decompile + disassemble `0x0065B580`)

This is the crux of the lane: `RadSiteClass__Activate @ 0x0065B580` builds and drives a
`LightSourceClass` exactly like a lamp, with two distinctions.

Verified flow (disassemble `0x0065B580`):
1. Compute rad world coords: `0x0065B750 CALL 0x005657a0` (Get_CellClass for rad cell at
   `RadSite+0x40`), then `0x0065B75E CALL [cell_vtable+0x48]` → cell world X/Y/Z onto stack.
2. Intensity + RGB computed by FPU math reading `RulesClass` fields `+0x1820`/`+0x1828` and
   color bytes `+0x1830..+0x1832`: i.e. `RadColor` scaled by `RadTintFactor`, intensity from
   `RadLevel * RadLightFactor`. FPU block `0x0065B5E2..0x0065B70F` VERIFIED-FROM-BINARY;
   semantic INI key names VERIFIED-FROM-DOC (`RADIATION_EMP` §1.11).
3. If `RadSite+0x24` (light ptr) is null: `0x0065B77C operator new(0x4c)` then `0x0065B7B2
   CALL 0x00554760` (LightSource ctor) with those coords/intensity/RGB; store ptr to `+0x24`.
4. **`0x0065B7BE MOV dword [light+0x34],0x0`** — overrides the detail threshold to **0**.
   *Distinction 1:* radiation glow is NEVER culled by `[Options]DetailLevel` (building lamps
   default to threshold 2, so a lamp is culled at DetailLevel 0/1 — radiation is not).
   VERIFIED-FROM-BINARY.
5. `0x0065B7C8 PUSH 0; 0x0065B7CA CALL 0x00554a60` — enable wrapper, **immediate mode (0)**.
6. *Distinction 2 (else branch `0x0065B7DE`):* if the light already exists, it calls
   `0x00554aa0` (update) instead of constructing — the per-`RadLightDelay` intensity/tint
   fade as the RadSite decays. VERIFIED-FROM-BINARY.

So radiation has no bespoke render path. **If we implement the general
LightSource→cell→draw pipeline, the radiation green glow falls out for free**: emit a
LightSource at the rad cell with intensity = `RadLevel*RadLightFactor`, RGB = `RadColor *
RadTintFactor`, detail-threshold 0, refreshed every `RadLightDelay` ticks and faded over the
site lifetime. INFERRED (design implication, well-grounded in §2 binary flow).

---

## 3. The per-cell lighting model (the key observable)

**Yes — gamemd keeps a per-cell light state that sources accumulate into, and that terrain
+ object drawing reads.** It is stored on `CellClass`, computed by `FUN_00484180`,
committed by `FUN_00483E30`, read by every draw path.

### 3.1 Cell light fields — VERIFIED-FROM-DOC (`MAP_LIGHTCONVERT_CACHE`, `LIGHTING_DRAW_CONSUMERS`); offsets cross-checked against this session's `0x00484180` disassembly

| `CellClass` offset | Role |
|---|---|
| `+0x34` | cached `LightConvertClass*` — palette-conversion profile for this cell's RGB; null triggers lazy recompute in draw paths |
| `+0x104` | 16.16 brightness scale (default `0x10000` = 1.0) carrying "excess" brightness factored out of RGB |
| `+0x108` | auxiliary light metadata word |
| `+0x10A` | "top"/unscaled ground brightness scalar |
| `+0x10C` | **common** brightness scalar — the one most draw paths pass to the blitter |
| `+0x10E` | "bottom"/alternate brightness scalar |
| `+0x110/+0x112/+0x114` | normalized R/G/B (0..1000) — cache-key mirror for `+0x34` |

NOTE: a *different* function `Cell_ComputeZAdjust @ 0x00484680` writes the overlapping set
`+0x10A/+0x10C/+0x10E` as **Z-sort depth bias** during superweapon transitions — that is
depth, not color. For ordinary play the lighting writer `0x00483E30` owns the bundle. Do
not conflate the two. VERIFIED-FROM-DOC (`CELL_COMPUTE_ZADJUST` vs `LIGHTING_DRAW_CONSUMERS`).

### 3.2 Per-cell compute formula `FUN_00484180` — VERIFIED-FROM-BINARY (disassemble this session, full body `0x00484180..0x00484675`)

For a real (non-sentinel) cell, milli-units (1000 = 1.0):

1. **Base ambient from scenario `[Lighting]`** (`0x004841AF`): `ambient = Scenario+0x352C *
   1000 / 100`; `red = +0x3534*1000/100`; `green = +0x3538*…`; `blue = +0x353C*…`. Point-light
   additive intensity starts at `0`.
2. **Loop every source** in `DAT_00ABCA14[0..DAT_00ABCA20)`. Skip unless BOTH:
   - `source+0x48 != 0` (active) — `0x00484294 TEST/JZ`.
   - `source+0x34 <= [0x00A8EB78]` (DetailLevel) — `0x004842A5 CMP EDX,EAX; JL skip`.
3. **Radius test** (leptons): cell center = `(cx*256+128, cy*256+128)` (`0x004842C1 SHL 8;
   ADD 0x80`). Reject if `dx*dx+dy*dy > radius*radius` (`0x0048432B JA`). Then real distance
   via `Sqrt_Approx(0x4CAC40)`+`ftol(0x7C5F00)`; inclusive guard `distance > radius` skips.
4. **Linear falloff** (`0x0048439A..0x004843B8`): `factor = ((radius-distance)*1000)/radius`
   (the `*1000` is `LEA *5*5*5; SHL 3`, then `DIV radius`). At `distance == radius`, factor = 0.
5. **Additive accumulation** (`0x004843BA..0x0048443E`): for each of intensity `+0x24`,
   R `+0x28`, G `+0x2C`, B `+0x30`: `add = source_field * factor / 1000` (`/1000` = magic
   `0x10624DD3`, `SAR 6`, sign-correct — **truncates toward zero, supports negative lamps**),
   then `ADD` into the running channel accumulator. **All active sources sum in before any
   clamp.**
6. **Ambient + point intensity** (`0x00484467 ADD EDX,EAX`): base ambient + accumulated
   additive intensity → ground brightness term; then add height term `level*height - ground`
   (top) / `level*(height+4) - ground` (bottom), `[Lighting] Ground`/`Level` selected per
   lighting mode (normal branch `Scenario+0x3540/+0x3544`).
7. **Normalize + clamp** (`0x004845A2..0x00484615`): high-clamp ground term to `2000`; call
   `FUN_005558E0` to clamp RGB `0..2000`, normalize the max channel to `1000`, scale the
   others proportionally, fold "excess" into the 16.16 scale (`+0x104`); scale bottom by that
   factor (`>>16`); final low-clamp all to `0`.
8. **Sentinel cells** `(0,0)`/`(-1,-1)` (`0x00484621`): return neutral — scale `0x10000`,
   intensity `0`, all RGB/ambient `1000` (`0x3e8`). VERIFIED-FROM-BINARY.

### 3.3 Commit + palette cache `FUN_00483E30` / `FUN_00544E70` — VERIFIED-FROM-DOC (`MAP_LIGHTCONVERT_CACHE`)

`0x00483E30` writes the 8-field bundle to the cell and maintains `+0x34`: normalizes the new
RGB (clamp 0..1000, quantize low bits by DetailLevel — masks `&~0x7F / &~0x3F / &~0x1F` for
detail 0/1/2), and if it differs from the cell's current convert key, releases the old
profile (refcount `+0x194`, gated by `g_GameActive`) and looks up/creates a new one via
`0x00544E70`. The cache (`DAT_0087F69C` vector) is keyed by the normalized RGB triple only;
`(1000,1000,1000)` returns the shared default profile at index 0 (unlit cells share one
profile). VERIFIED-FROM-DOC.

### 3.4 Draw consumers (why ground AND units tint together) — VERIFIED-FROM-DOC (`LIGHTING_DRAW_CONSUMERS`)

Every visible draw path lazy-inits `cell+0x34` if null, then reads the bundle:
- **Terrain TMP tile** `CellOverlay_TileDraw @ 0x00480350`: passes `cell+0x34` (palette
  profile) AND `cell+0x10C` (brightness scalar) into `TMP_TileBlitter`.
- **Overlays** (ore/walls/bridges) `0x0047F6A0`: `+0x10C` common, `+0x10A`/`+0x10E` branches.
- **Terrain objects** (trees/rocks/lamp art) `0x0071C250`: `+0x10C` (or `+0x10A` for a flag).
- **Techno SHPs** (infantry/buildings/units) `0x00705E00`: `+0x10C`.
- **Anims** `0x00423200`: per-type — cell convert+`+0x10C`, or fixed/global convert.

So a colored light at a cell visibly tints the terrain tile, the ore/overlay on it, the tree
on it, AND the infantry/tank standing on it — all from the same cell state. **This uniform
ground+sprite tint is the single most important observable to reproduce.**

---

## 4. Dirty scheduling (how a toggle becomes visible) — VERIFIED-FROM-DOC (`LIGHTSOURCE_DIRTY_SCHEDULING`)

`0x00554AF0(mode)` scans the source's affected cells (inclusive square `floor(radius/256)+1`
then circular `center-distance <= radius`, same lepton math as §3.2 step 3):
- **mode 0 (immediate)** — every standard lamp AND radiation caller uses this — calls
  `Get_CellClass` + `0x00483E30` per cell, recomputing from all currently-active sources NOW.
- **mode != 0 (queued)** — enqueues 0x14-byte records into `DAT_00ABCA44`, drained over ticks
  by `0x00554D50` (6 ms prep budget, atomic commit) in the logic tick after RadSite AI /
  before EMPulse AI. Real path but no standard caller decompiled passes nonzero.
Gated by master logic flag `DAT_00829AE4` (off during load/clear). VERIFIED-FROM-DOC.

---

## 5. Ambient-vs-dynamic blend rule (answer)

**ADDITIVE, then normalized + clamped — VERIFIED-FROM-BINARY (§3.2 steps 5–7).**

- Each active source contributes `field * linear_falloff_factor / 1000` (truncate-toward-zero),
  summed across all sources (negative lamps subtract).
- Source intensity sum is **added** to the scenario ambient base.
- Source RGB sums are **added** to the scenario RGB tint base.
- Only AFTER all sources are summed: RGB clamped `0..2000`, max channel normalized to 1000
  with excess brightness pushed into a separate 16.16 scale, ambient high-clamped to 2000,
  everything low-clamped to 0.
- It is NOT per-source clamped, NOT multiplicative, NOT max-blend. (Current Rust
  `accumulate_point_lights` clamps per channel per light and uses f32 cell-space distance —
  a known DRIFT, per `MAP_LIGHTING_CELL_COMPUTE`.)

For the wgpu radiation glow we don't need byte-exact palette quantization, but to look right
we must: (a) accumulate the green source additively onto ambient before clamp; (b) use
lepton-center linear falloff with the inclusive-edge=0 rule; (c) apply the resulting per-cell
RGB to terrain AND sprites in the cell uniformly; (d) refresh on a `RadLightDelay` cadence and
fade with the site. INFERRED (design, grounded in §2–§4).

---

## 6. TS-legacy / gating notes

- **No TS-only gate on the core path.** `0x00554AF0`/`0x00484180` are live in standard YR;
  the active gates are the scenario logic flag (`DAT_00829AE4`) and `[Options]DetailLevel`,
  not a SpecialFlags dead path. VERIFIED-FROM-DOC (`LIGHTSOURCE_DIRTY_SCHEDULING` OQ-15).
- **DetailLevel gate is real and player-facing for lamps** (threshold 2) but **does NOT
  affect radiation** (threshold forced to 0). Radiation glow shows at every detail level.
  VERIFIED-FROM-BINARY (`0x0065B7BE`).
- The superweapon/Ion/PsychicDominator height-mode branches in `0x00484180` (§3.2 step 6) are
  live conditional code, not the radiation path — leave them out of the radiation glow feature.
- `ExtraLight=` is NOT RGB ambience — it is a building draw-depth/Z scalar; do not route it
  through the radiation tint grid. VERIFIED-FROM-DOC (`LIGHTING_DRAW_CONSUMERS` finding 7).

---

## 7. Open / not-this-session

- Exact `LightConvertClass` palette-table generation (`0x00555DA0`→blitter) — out of scope;
  wgpu reproduces the observable tint, not the palette object. DEFERRED (per cache doc).
- `Math__ftol` rounding mode (`0x007C5F00`) — black-box; ±1-unit boundary effects. DEFERRED.
- Whether any rare caller passes queued mode (nonzero) — all decompiled standard callers use
  immediate mode 0. DEFERRED (`LIGHTSOURCE_DIRTY_SCHEDULING` OQ-16).

## Sources (this session)

- Live Ghidra decompiles (2026-06-15): `decompile_function 0x00554760` (LightSource ctor),
  `0x00554A60` (enable), `0x00554AA0` (update), `0x0065B580` (RadSite activate).
- Live Ghidra disassembly (2026-06-15): `disassemble_function 0x004842B0`
  (full `0x00484180` body) and `0x0065B580` (full RadSite activate body).
- Prior verified docs (named inline above), all `[ghidra/verified]`.
