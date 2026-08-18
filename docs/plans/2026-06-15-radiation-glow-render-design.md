# Radiation Green Glow (Render) — Design Spec

**STATUS: DESIGN — awaiting user approval, no code yet.**

**Feature:** Cell/Map substrate open item #4 (`docs/research/SUBSTRATE_OPEN_ITEMS_20260610.md:15`).
**Date:** 2026-06-15 · **Authority order:** binary → Ghidra → docs/research → ini.
**Scope of this doc:** render-layer design only. The sim core landed in commit `86b0d4bf`
(Slice 7); nothing here touches sim state, the snapshot, or the state hash.

Confidence tags used below: **VERIFIED-FROM-BINARY** (live Ghidra decompile/disasm this
worknote round, call cited), **VERIFIED-CODE** (read from current Rust this round, file:line),
**VERIFIED-DOC** (from a prior `[ghidra/verified]` research doc, not re-decompiled), **INFERRED**
(reasoned design implication), **UNKNOWN/UNCHECKED** (explicitly unverified — do not invent).

> **Verifier note.** The radiation-light formula lane shipped a doc-only first draft that
> concluded the per-site `2000` clamp does *not* exist. A second pass with a live Ghidra
> connection **REFUTED** that draft (supersession banner in
> `docs/research/substrate/worknotes/radiation-glow-render-20260615/gamemd-radsite-light.md`).
> This spec uses the corrected, binary-verified values: the per-site `min(…, 2000.0)` clamp on
> both intensity and each tint channel **is real**, and the tint channel is pre-scaled
> `byte × 1000 / 255` before the `× RadTintFactor` and the clamp. Those corrected values
> override any contradicting earlier claim.
>
> **Independent re-verify (2026-06-15): all three load-bearing claims CONFIRMED from the binary**
> (`verification.md`) — intensity (`read_memory 0x007edae0` = `2000.0`), the dual-curve tint/intensity
> decay + `RadLightDelay` cadence, and the additive→normalize→clamp `0..2000` per-cell compositing
> read by both terrain (`0x00480350`) and techno/sprite (`0x00705E00`) draws via `CellClass+0x34/+0x10c`.
> No corrections needed.

---

## 1. Intent / symptom

Irradiated ground must visibly **glow green on every Desolator deploy** (and every other
`RadLevel>0` detonation: nukes, demo bombs). The sim core that tracks the radiation field, sites,
decay, merge, and damage is **done** (`86b0d4bf`); the **render-layer dynamic light/glow is
missing** — the four glow constants are parsed into `RuleSet.radiation` but no render code consumes
them, so the field is currently invisible.

---

## 2. Verified gamemd behavior

gamemd has **one** dynamic-light primitive, `LightSourceClass` (a `0x4C`-byte object holding world
X/Y/Z leptons, a radius in leptons, a signed milli-unit intensity, and a signed RGB tint, all in
`1000 == 1.0` units). The radiation glow is **not a bespoke green overlay** — `RadSiteClass`
constructs one ordinary `LightSourceClass` per site and drives it; the same per-cell lighting
pipeline that handles building lamps then tints both the ground tile and the units standing on it.
Reproducing the general primitive's *observable* result makes the green glow fall out.

### 2.1 Trigger & gating — VERIFIED, not gated

Site creation fires for any weapon with `RadLevel>0` in `WarheadTypeClass::Detonate`
(`0x004690B0`, VERIFIED-DOC) — stock `[RadEruptionWeapon]=500` (Desolator deploy),
nukes/`[Demobomb]=100`, etc. The per-tick RadSite update loop and its light flush run
**unconditionally every logic tick** — no `SpecialFlags` guard, no TS-legacy gate
(VERIFIED-FROM-BINARY, `decompile_function 0x0055AFB0`: the RadSite reverse-loop + trailing
`FUN_00554d50()` flush have no scenario-flag guard, in contrast to the lightning/ion-storm blocks
earlier in the same function which *are* gated on `*g_ScenarioClass & 0x1000`). The glow is live in
stock YR.

### 2.2 Intensity formula — VERIFIED-FROM-BINARY (with the 2000 clamp)

Computed once at activation in `RadSiteClass__Activate` (stored at `RadSite+0x54`):

```
LightIntensity = ftol( min( RadSite.RadLevel × RadLightFactor , 2000.0 ) )
```

- `RadLevel` = the **per-site peak level** (`RadSite+0x4C`), summed on overlap — **NOT** the
  per-cell decayed level. VERIFIED-FROM-BINARY (`disassemble_function 0x0065B580`: `FILD [ESI+0x4c]`
  → `FMUL [EDI+0x1820]` (`RadLightFactor`) → `FCOMP` vs `0x007edae0` → `ftol 0x007c5f00` →
  `MOV [ESI+0x54]`).
- The clamp ceiling `0x007edae0` decodes to exactly `2000.0` (VERIFIED-FROM-BINARY,
  `read_memory 0x007edae0` = `00 00 00 00 00 40 9F 40`).
- `ftol` (`0x007c5f00`, MSVC `_ftol`) truncates toward zero — rounding mode INFERRED-standard,
  the arithmetic VERIFIED.

### 2.3 Color (tint) — VERIFIED-FROM-BINARY (with the ×1000/255 rescale correction)

Each tint channel computed at activation (`RadSite+0x58/+0x5C/+0x60`):

```
Tint_c = ftol( min( (RadColor_c × 1000 / 255) × RadTintFactor , 2000.0 ) )    c ∈ {R,G,B}
```

VERIFIED-FROM-BINARY (`disassemble_function 0x0065B580`): RadColor bytes read from
`RulesClass+0x1830/+0x1831/+0x1832`; integer pre-scale `× 1000` (`LEA ×5 ×5 ×5; SHL 3`) then
signed `÷255` (`IMUL 0x80808081; SAR 7` — `convert_number 0x80808081` = the ÷255 magic); then
`FMUL [EDI+0x1828]` (`RadTintFactor`); then the same `FCOMP` vs `0x007edae0` (2000.0); then `ftol`.
The `×1000/255` byte rescale and the per-channel 2000 clamp are both **corrections** to
`RADIATION_EMP_GHIDRA_REPORT.md §1.6`, which gives the cruder un-rescaled, un-clamped form. The
`1000` base is the engine's `1.0` light unit, so the green channel pre-scales to exactly `1000`.

### 2.4 Decay / fade — VERIFIED-FROM-BINARY (two different curves)

`RadSiteClass__AI` (`0x0065B800`) decrements `RemainingDuration` (`RadSite+0x70`) every tick and
self-destructs at `< 1` (tearing down the LightSource with it). On each **light-step**
(`decompile_function 0x0065B800`):

```
tint_c'      = Tint_c × RemainingDuration / TotalDuration      (integer; multiplicative ratio fade)
intensity'   = LightSource.intensity − LightIntensityDecrement (fixed per-step subtraction)
FUN_00554aa0(intensity', tintR', tintG', tintB', 0)            (trailing 0 = immediate mode)
```

with the per-step decrement precomputed at activation (VERIFIED-FROM-BINARY, integer `CDQ/IDIV` in
`0x0065B580`):

```
TotalDuration           = RadSite+0x6C = RadLevel × RadDurationMultiple
LightIntensityPerStep   = RadSite+0x64 = TotalDuration / RadLightDelay
LightIntensityDecrement = RadSite+0x68 = LightIntensity / LightIntensityPerStep
```

**Critical: tint and intensity decay on different curves.** Tint is the `remaining/total` *ratio*
(full at spawn → 0 at expiry); intensity is a *fixed per-step subtraction*. The render must
reproduce both — not one shared scalar.

### 2.5 Update cadence — VERIFIED-FROM-BINARY

Two separate timers, both off `RulesClass` `[Radiation]`: `RadLevelDelay` (`+0x1810`) drives the
per-cell level decay/damage; `RadLightDelay` (`+0x1814`) drives the **light** update. Both are
stock `90` so they coincide, but they are independent fields (VERIFIED-FROM-BINARY,
`decompile_function 0x0065B580`: `+0x1810`→`RadSite+0x30`, `+0x1814`→`RadSite+0x3c`). The light
parameters change once per `RadLightDelay` (~6 frames per fade step over a 500-frame Desolator
lifetime), **stepwise, not continuous**.

### 2.6 RadSite → LightSource → per-cell compositing — VERIFIED

- **Activation** (`0x0065B580`, VERIFIED-FROM-BINARY): on first activation `operator_new(0x4c)` +
  `LightSourceClass__Constructor 0x00554760` at the center cell's 3D coords with
  `(spreadLeptons, intensity, tintR/G/B)`; **forces detail threshold `light+0x34 = 0`** (radiation
  is never culled by `[Options]DetailLevel`, unlike building lamps which default to threshold 2);
  enables it immediate-mode (`0x00554a60(0)`). On a merge it instead calls `0x00554aa0(…, 0)` to
  update the existing light in place.
- **Per-cell accumulation** (`FUN_00484180`, VERIFIED-FROM-BINARY/DOC): for each active source
  within radius, `add = source_field × ((radius − distance) × 1000 / radius) / 1000`, **summed
  additively** onto the scenario `[Lighting]` ambient base across *all* sources, then the
  accumulated RGB is normalized (max channel → 1000, excess into a 16.16 scale) and **clamped
  `0..2000`** (the *second*, downstream 2000 clamp; both clamps exist). Falloff uses lepton cell
  centers `(cx×256+128, cy×256+128)`, inclusive edge → factor 0. Distance uses X/Y only; Z is not
  in the radius test.
- **Draw consumers** (VERIFIED-DOC, `LIGHTING_DRAW_CONSUMERS`): terrain TMP blit, overlays, terrain
  objects, AND techno SHPs all read the same per-cell brightness scalar + palette profile — so one
  green light tints the ground tile and the unit on it identically. **This uniform ground+sprite
  tint is the single most important observable to reproduce.**
- **Per-frame flush** (`0x0055AFB0`, VERIFIED-FROM-BINARY): a reverse loop over the live RadSite
  array (`0x00B04BD4`/count `0x00B04BE0`) calls each RadSite's AI (vtable `+0x5C`), then
  `FUN_00554d50()` batch-flushes dirty cells.

### 2.7 Blend rule — VERIFIED-FROM-BINARY

Lighting is **additive, then normalized + clamped** — not multiplicative, not per-source clamped,
not max-blend. All active sources sum into the ambient base first; only then is the result
normalized and clamped `0..2000`. (Note: the existing Rust `accumulate_point_lights` clamps
per-light per-channel and uses f32 cell-space distance — a pre-existing DRIFT noted in
`MAP_LIGHTING_CELL_COMPUTE`; see §8.)

### 2.8 Stock numeric walkthrough (Desolator) — VERIFIED formulas, stock INI inputs

`RadDurationMultiple=1`, `RadLevelDelay=RadLightDelay=90`, `RadLightFactor=0.1`,
`RadTintFactor=1.0`, `RadColor=0,255,0`; Desolator `RadLevel=500`:
- `TotalDuration = 500 × 1 = 500` frames (~33 s @ 15 fps), light steps every 90 frames (~5–6 steps).
- `LightIntensity = ftol(min(500 × 0.1, 2000)) = 50` — a single site stays far under the 2000 cap.
- `LightIntensityPerStep = 500/90 = 5`; `LightIntensityDecrement = 50/5 = 10` → intensity ≈ `50−10k`.
- tint at spawn `(0, min(255×1000/255 × 1.0, 2000), 0) = (0, 1000, 0)`; then `(0, 1000×rem/500, 0)`.
- The **per-site** 2000 clamp only bites when stacked levels reach `RadLevel ≥ 20000`
  (`× 0.1 ≥ 2000`); the **per-cell** accumulation clamp bites sooner on overlapping sites.

---

## 3. Current Rust state

### 3.1 Sim API the render layer can read (VERIFIED-CODE, `src/sim/radiation.rs`)

`RadiationState` is pure sim data, in `sim/`, serialized + state-hashed; render may read it via `&`,
never write. Public surface:
- `sites() -> impl Iterator<&RadSite>` (`:162`, deterministic BTreeMap order by center).
- `iter_cells() -> impl Iterator<(&(u16,u16), &f64)>` (`:168`; doc-comment already says "for state
  hashing **and the render glow layer**").
- `current_site_level(site) -> i32` = `remaining × level / duration` (`:180`).
- `cell_level` (`:145`), `site_at` (`:158`), `is_empty` (`:173`).
- `RadSite` public fields (`:37-64`): `center`, `spread`, `radius_leptons` (= `spread×256+128`),
  `level` (peak at last activation), `level_steps`, `duration`, `remaining`, `level_timer_start`,
  `level_timer_duration`.

**Two gaps in the "exposes everything render needs" claim** (VERIFIED-CODE):
1. **No glow rules on the site.** `RadSite` carries no `RadColor/RadLightFactor/RadTintFactor/
   RadLightDelay`; those live on `RuleSet.radiation: RadiationRules` and must be threaded into the
   render adapter separately.
2. **No `RadLightDelay` light timer on the site** — `RadSite` carries only the *level* timer.
   Stock `RadLightDelay == RadLevelDelay == 90` so they coincide. INFERRED: render can derive the
   fade purely from `remaining/duration` + the rules (tint fade is a pure ratio; intensity step is
   `level×factor` scaled the same way), so **no new sim field / no snapshot bump is required**.

### 3.2 Render-side glow constants — already parsed (VERIFIED-CODE, `src/rules/ruleset.rs`)

All four are parsed into `RadiationRules` and flagged "Render-only" in their doc-comments:
`light_delay: i32` (`:964`), `light_factor: SimFixed` (`:972`), `tint_factor: SimFixed` (`:974`),
`color: (u8,u8,u8)` (`:976`). Defaults match stock exactly (`light_delay 90`, `light_factor 0.1`,
`tint_factor 1.0`, `color (0,255,0)`). **No new INI parsing required** (see §4).

### 3.3 Render infra gap — is there any dynamic light today? NO

VERIFIED-CODE (rust-render-architecture worknote, evidence ledger):
- The tactical scene is lit by a **single static per-cell RGB multiplier** — `CellLightGrid` in
  `src/map/lighting.rs`, baked at map load, folded into every sprite's per-instance `tint: vec3f`
  (`src/render/batch.rs:42,54`), applied in every fragment shader as `color.rgb * input.tint`
  (`batch_shader.wgsl:85`). Terrain reads it via `terrain_tile_tint_at()` into `SpriteInstance.tint`
  (`map/terrain.rs:840`); units/overlays read the same grid through the category accessors.
- Building lamps accumulate into that grid via `accumulate_point_lights()` (`lighting.rs:574`):
  signed linear falloff `(radius−dist)/radius × intensity`, summed into raw RGB + additive
  accumulators, with `LIGHT_CLAMP_MAX = 2000` (`:32`) — **the exact shape and 2000 cap the
  radiation glow needs.**
- **There is NO additive blend pass, no light-accumulation buffer, no light volume, no
  post-process** — verified across `render/mod.rs` and all 7 `.wgsl` shaders. No `render/**/*light*`
  file exists.
- The grid is rebuilt **event-driven** — `rebuild_lighting_grid_from_sim` (`app_init.rs:171`) on map
  load (`:1069`), building placement/removal (`app_input.rs:827`), transitions
  (`app_transitions.rs:151`) — **never per-frame.** Radiation decays every tick, so a per-frame /
  per-step refresh trigger is the one genuinely new piece of plumbing.
- Render reads zero radiation state today (`grep -i radiation src/render/` → no matches). The
  boundary is clean: sim produces, render consumes-nothing-yet.

The radiation glow is therefore a **data-feed problem, not a new-pipeline problem.** The tint
*consumption* path is 100% built; the gap is the *feed + update cadence*.

---

## 4. INI surface

All ten `[Radiation]` keys are parsed and stock-correct in Rust today; **nothing is missing**, no
new parsing is required. RA2 `rules.ini` and YR `rulesmd.ini` are byte-identical here (no YR patch).
The four lighting-relevant keys (VERIFIED-CODE / VERIFIED file reads, ini-keys worknote):

| Key | Stock value | Parsed in Rust today | To add |
|---|---|---|---|
| `RadColor` | `0,255,0` (pure green) | YES — `ruleset.rs:1026-1035` → `color: (u8,u8,u8)` | — |
| `RadLightFactor` | `0.1` | YES — `ruleset.rs:1018-1021` → `light_factor: SimFixed` | — |
| `RadTintFactor` | `1.0` | YES — `ruleset.rs:1022-1025` → `tint_factor: SimFixed` | — |
| `RadLightDelay` | `90` | YES — `ruleset.rs:1013` → `light_delay: i32` | — |
| (context) `RadDurationMultiple` | `1` | YES — `ruleset.rs:1007` | — |
| (context) `RadLevelMax` | `500` | YES — `ruleset.rs:1011` | — |

- **Do NOT add `RadSiteColor`** — it does not exist in stock; it is an Ares extension (VERIFIED file
  read: absent from both `rules.ini` and `rulesmd.ini`).
- Never hardcode the color/factors — the adapter reads them from `RuleSet.radiation`.
- Minor flags for the implementer (do not block the design): the `RadSiteWarhead` doc-comment
  (`ruleset.rs:977`) says "uppercased" but the code only `trim()`s (cosmetic); and `light_factor`/
  `tint_factor` are stored `SimFixed` whereas gamemd does the `ftol` math in `double` — fine at
  stock values, a potential render drift **only** at exotic non-stock factors (see §8).

---

## 5. Proposed architecture

### 5.1 Chosen seam — **SEAM A: accumulate radiation into the existing `CellLightGrid`**

A render/app-layer light service walks `RadiationState` each frame (when non-empty), derives the
green contribution (intensity/tint/fade per §2), and accumulates it into the existing
`CellLightGrid` — exactly as `accumulate_point_lights` already does for building lamps — *before*
the tint accessors read it. The tint then flows through the existing `SpriteInstance.tint` →
`color.rgb * tint` path with **zero GPU/pipeline/bind-group/shader/atlas change.**

**Rationale (matches the verified gamemd primitive):** gamemd's radiation glow *is* a
`LightSourceClass` feeding per-cell ambient color, composited through the same per-cell pipeline as
building lamps. Folding a green source into `CellLightGrid` reproduces that observable result —
tinting terrain AND the units/buildings/overlays on irradiated cells in one shot — satisfying "model
the gamemd primitive, don't approximate." It reuses the verified additive falloff math, `1000==1.0`
units, and the matching `LIGHT_CLAMP_MAX = 2000`.

**Rejected alternatives** (rust-render-architecture worknote §3):
- **SEAM B (separate additive glow pass):** duplicates the lighting model gamemd folds into ambient,
  risks double-counting against the cap, composites differently from the original (screen overlay vs
  per-cell ambient). More GPU work, lower parity. Reconsider only if SEAM A shows a frame-cadence
  problem.
- **SEAM C (per-sprite tint only):** parity miss — the *ground* must glow, not just the units. (SEAM
  A subsumes it for free.)
- **SEAM D (post-process):** out of scope, least faithful, no per-cell post-process exists.

### 5.2 Components

1. **`RadiationState → light` adapter** (app/render layer): reads `sites()` (+ `radius_leptons`,
   `level`, `remaining`, `duration`) and the four `RuleSet.radiation` constants; emits a
   `PointLight`-shaped green source per active site (position = center cell, radius =
   `radius_leptons`), applying `intensity = min(level × RadLightFactor, 2000)` and
   `tint = (RadColor × 1000/255 × RadTintFactor) × (remaining/duration)` clamped at 2000, then routes
   through the **additive** `accumulate_point_lights` path (never overwriting the multiplicative
   base tint). Detail-threshold is forced on (radiation ignores DetailLevel, §2.6). Lives next to
   `collect_live_building_lights` / `rebuild_lighting_grid_from_sim`.
2. **Per-frame / per-step refresh trigger** (app layer): when `!sim.radiation.is_empty()`, refresh
   the radiation contribution on top of the cached base+building grid — the one new piece of
   plumbing. Idle matches (`is_empty()`) pay nothing. Cadence options in §6 / §8.
3. **Rules plumbing:** confirm `rules.radiation.{color,light_factor,tint_factor,light_delay}` reach
   the adapter (already on `rules.radiation`, threaded into the sim tick at `world/mod.rs:2305`;
   the render side just needs the same `&RuleSet`).

### 5.3 #1-invariant compliance (sim never depends on render)

Preserved **by construction**: `RadiationState` stays pure serialized sim data (it knows nothing of
lights/tint/RGB and evolves only inside `World::advance_tick`); the light service lives in app/render
and **READS** `RadiationState` by `&`-borrow, writing only into the app-owned `CellLightGrid` that
`render/` already consumes. Dependency direction is **render → sim (read-only)**, never the reverse.
The tint is a pure function of serialized sim state, so it is replay/lockstep-safe and never feeds
back into the deterministic hash. No game-logic float leaks into `sim/` — the adapter's RGB/falloff
math is render-side and may use `glam`/`f32`; the sim `f64` field and `SimFixed` factors stay in
`sim/`/`rules/`.

---

## 6. Slice breakdown

Each slice is render-only and independently verifiable. **No sim state, snapshot version, or state
hash changes in any slice** — call that out at review: the golden replay baselines must be byte-for-
byte unshifted (radiation already folds zero into the hash while present per `86b0d4bf`; the render
glow never touches the hash). Sim determinism is unaffected.

- **Slice 1 — static green tint, no fade.** Adapter emits one green source per active site at full
  spawn intensity/tint into `CellLightGrid`; refresh trigger gated on `!radiation.is_empty()`.
  *Verify:* deploy a Desolator → irradiated cells (ground + any unit on them) turn green; expires
  when the site dies. Confirms the feed + seam + invariant before adding curves.
- **Slice 2 — correct intensity + tint formulas with the 2000 clamps.** Apply
  `intensity = min(level × RadLightFactor, 2000)` and `tint = min(ch × 1000/255 × RadTintFactor,
  2000)`; route additively; apply the downstream per-cell `0..2000` clamp. *Verify:* stock Desolator
  reads intensity 50 / green tint 1000 (§2.8); stacked deploys approach the cap.
- **Slice 3 — time fade on the two curves.** Tint fades by `remaining/duration` (ratio); intensity
  steps down by the fixed per-step decrement; step on the `RadLightDelay` cadence (derived from
  `remaining/duration` + rules, no new sim field). *Verify:* glow visibly dims in ~5–6 steps over the
  ~33 s lifetime and is gone at expiry; side-by-side step cadence matches gamemd.
- **Slice 4 — stacking / overlap parity.** Confirm overlapping sites sum additively before the
  per-cell clamp (multiple Desolators / a nuke over a deploy). *Verify:* overlap is brighter, capped
  at 2000, matching gamemd's additive-then-clamp.
- **Slice 5 (optional, scope-gated) — secondary visuals.** `EMPulseSparkles` anim and any SNOW-
  theater channel forcing — **deferred / out of scope** pending the §8 UNKNOWNs; not required for the
  core green-glow parity.

---

## 7. Acceptance criteria (player-visible, side-by-side with gamemd)

1. **Appears on deploy:** every Desolator deploy (and every `RadLevel>0` detonation) produces a green
   glow on the irradiated cells, on the **ground tile and on units/buildings standing on them**
   (uniform, §2.6).
2. **Correct color:** pure green at stock `RadColor=0,255,0` / `RadTintFactor=1.0` (tint base 1000,
   intensity 50 for a stock Desolator).
3. **Correct footprint:** glow covers the `(2·spread+1)²` affected square with linear falloff to the
   edge (lepton-center distance, inclusive edge → 0), matching the sim field extent.
4. **Decays/steps over lifetime:** dims **stepwise** on the `RadLightDelay` cadence (~5–6 steps over
   ~33 s for stock Desolator), with tint fading on the `remaining/duration` ratio and intensity on the
   fixed per-step subtraction (two curves, §2.4).
5. **Vanishes on expiry:** glow disappears exactly when the site self-destructs (`remaining < 1`); the
   contribution is removed from the grid the same frame.
6. **Stacking:** overlapping sites read brighter (additive), clamped at 2000.
7. **Not detail-gated:** glow shows at every `[Options] DetailLevel` (radiation forces threshold 0).
8. **Determinism:** golden replay baselines and the state hash are unchanged (render-only).

Side-by-side method: deploy a Desolator in both gamemd.exe and this engine on the same map/cell;
compare onset frame, color, footprint, the fade-step cadence and per-step brightness, and the expiry
frame.

---

## 8. OPEN / UNKNOWN / UNCHECKED

1. **Refresh cadence — RESOLVED 2026-06-15 → per-step (match gamemd).** The radiation contribution is
   rebuilt only on `RadLightDelay` step boundaries, so the dim happens in ~5–6 discrete steps exactly
   like gamemd — not smoothly interpolated. (Rejected: per-frame, simpler but produces a smooth fade
   where gamemd's is stepwise, a subtle observable drift.) The refresh trigger fires on the step
   boundary while `!radiation.is_empty()`.
2. **No independent light timer in sim (resolved as INFERRED, confirm at review).** Render derives the
   fade from `remaining/duration` + rules with no new sim field / no snapshot bump. INFERRED from the
   formula structure; confirm it reproduces gamemd's step boundaries before code lands. If a faithful
   per-step intensity ledger turns out to need sim state, that would be a sim change (snapshot bump) —
   currently judged unnecessary.
3. **`SimFixed` vs `double` factor math (UNCHECKED at non-stock factors).** gamemd does the
   intensity/tint `ftol` math in `double`; Rust stores `light_factor`/`tint_factor` as `SimFixed`.
   Bit-identical at stock `0.1`/`1.0`; a potential render drift only at exotic non-stock factors. Not
   a blocker for stock parity; flag if the engine ever ships non-stock `[Radiation]`.
4. **SNOW-theater R/B channel forcing (UNKNOWN — do not invent).** On SNOW theater (`Scen+0x1258==1`)
   the study flags an R/B channel adjustment whose exact intent is unverified. Needs a fresh Ghidra
   read before implementing; out of scope for the core slices (Slice 5).
5. **`EMPulseSparkles` anim (scope question).** A secondary visual (shared with EMP,
   `RulesClass+0x17F4`) played alongside the light. Separate from the green-glow light itself; in
   scope or not? (Slice 5.)
6. **Per-site vs per-cell light model — RESOLVED 2026-06-15 → per-site (native-faithful).** The
   adapter emits **one green source per site** from `sites()` (recomputing falloff in the adapter),
   matching gamemd's one-LightSource-per-site object model and getting the intensity-vs-tint curve
   separation naturally. Slice 2 must still empirically confirm the visible result is pixel-identical
   to a per-cell feed, but per-site is the chosen model.
7. **Pre-existing `accumulate_point_lights` DRIFT (inherited, not introduced).** The existing point-
   light accumulator clamps per-light per-channel and uses f32 cell-space distance, whereas gamemd
   sums all sources before one clamp using lepton-center distance (VERIFIED-FROM-BINARY §2.7;
   `MAP_LIGHTING_CELL_COMPUTE`). Radiation routing through it inherits that DRIFT. Out of scope to fix
   here, but flagged: it may slightly affect overlap/falloff exactness. Decide whether to fix the
   shared accumulator or accept the inherited drift for radiation.
8. **`Math__ftol` rounding (DEFERRED, ±1-unit).** `0x007c5f00` rounding mode is black-box;
   ±1-unit boundary effects on intensity/tint. Negligible at stock values.

---

## 9. References

Worknotes (`docs/research/substrate/worknotes/radiation-glow-render-20260615/`):
- `existing-research-and-rust-state.md` — prior research inventory + current Rust API surface +
  the `86b0d4bf` done/todo boundary + live re-verification addendum.
- `gamemd-radsite-light.md` — the binary-verified intensity/tint/decay/cadence formulas; **carries
  the verifier supersession banner** (refutes the doc-only "no 2000 clamp" draft with live evidence).
- `gamemd-lighting-system.md` — the general `LightSourceClass` primitive, per-cell compute formula,
  additive blend rule, draw consumers, DetailLevel gate.
- `rust-render-architecture.md` — current Rust render/lighting architecture, the SEAM A/B/C/D
  analysis, the infra gap, and the evidence ledger.
- `ini-keys.md` — stock `[Radiation]` values, RA2=YR identity, full Rust parse table, the
  `RadSiteColor` non-existence, the trigger chain.

Source study / open-items:
- `docs/research/SUBSTRATE_OPEN_ITEMS_20260610.md` #4 — the open item this closes.
- `docs/research/CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §2.6 — the load-bearing
  "Green glow" decode row.
- `docs/research/RADIATION_EMP_GHIDRA_REPORT.md` §1.6/§1.8/§1.11 — radiation activation / per-tick
  light step / visual effects.
- `docs/research/MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`,
  `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`,
  `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md` — the per-cell lighting pipeline.

Key sim/render files (read-only this pass): `src/sim/radiation.rs`, `src/rules/ruleset.rs`
(`RadiationRules`), `src/map/lighting.rs` (`CellLightGrid`, `accumulate_point_lights`),
`src/render/batch.rs` (`SpriteInstance.tint`), shaders `batch_shader.wgsl`.
