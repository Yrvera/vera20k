# Radiation Green Glow (render) — Existing Research & Current Rust State

**Lane:** inventory of prior research + current Rust radiation state the render layer can read.
**Date:** 2026-06-15 · **Mode:** read-only (no Rust, no Ghidra writes). Authority order binary → Ghidra → docs → ini.
**Feature:** Cell/Map substrate open item #4 — `SUBSTRATE_OPEN_ITEMS_20260610.md:15`. Sim core landed `86b0d4bf`; the per-site dynamic LightSource glow is the residual.

Verification tags: **VERIFIED-DOC** = sourced from a `[ghidra/verified]` research doc (not re-decompiled this lane); **VERIFIED-CODE** = read directly from current Rust this lane; **INFERRED** = reasoned, not directly confirmed; **UNKNOWN** = explicitly unverified.

---

## 1. Prior research inventory (radiation + LightSource)

Found via research-index FTS (`research_search "radiation RadSite Desolator RadLight glow light"`). Docs touching the glow, by relevance:

| Doc | What it carries for the glow |
|---|---|
| `RADIATION_EMP_GHIDRA_REPORT.md` | §1.6 activation, §1.8 AI per-tick light step, §1.11 Visual Effects, §1.2/§1.12 INI offsets + globals. The primary visual-effects source. |
| `CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | §2.6 (NEW LIVE-0610 full decode) the single best table; the "Green glow" row has the load-bearing intensity/tint formula. §4.2 #12 dispositions the residual. R16 row (§2 cell map). CLOSED — evidence archive, do not re-derive. |
| `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md` | LightSource field offsets (`+0x24` intensity, `+0x28/2C/30` RGB tint, all ×1000 units), the dirty/affected-cell recompute machinery, and an explicit "radiation uses the SAME LightSource update machinery, immediate mode, zero queue flag" handoff + a named acceptance test. |
| `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md` | Negative fact: radiation glow updates do **NOT** enqueue delayed lighting records — both activation and AI update callers pass zero (immediate mode). |
| `TERRAIN_OBJECT_LIGHT_KEYS_AND_LIGHTSOURCE_OWNERSHIP_GHIDRA_REPORT.md`, `BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md` | LightSourceClass ctor (`0x00554760`) has exactly 3 callers: `BuildingClass::Unlimbo`, `BuildingClass::OnConstructionComplete`, `RadSiteClass::Activate @ 0x0065B580`. Confirms radiation is one of only three dynamic-light producers. |
| `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md` | Parser defaults/ownership for the static building-light keys (LightVisibility/Intensity/RedTint…). Context for the existing static-light primitive, not radiation itself. |
| `RULESCLASS_GHIDRA_REPORT.md` §`[Radiation]`, `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` | INI offsets and stock frame-delay values (RadLevelDelay/RadLightDelay = 90; RadApplicationDelay = 16). |
| `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md` | RadSite AI is iterated **backward** over the global vector, right after LightningStorm; appends mid-loop are not visited same pass. Tick-ordering context. |

### 1a. Radiation sim model — verified facts pulled from study §2.6 + RADIATION_EMP §1.x

All **VERIFIED-DOC** (`[ghidra/verified]`; re-verified live in the study's LIVE-0610 pass — not re-decompiled this lane):

- **Trigger / gating:** any weapon with `RadLevel > 0` (`WeaponType+0x158`); site created in `WarheadTypeClass::Detonate 0x004690B0`. **Active in stock YR — no SpecialFlags / TS gate anywhere in the path** (study §2.6 explicit). Stock producers: `[RadEruptionWeapon]=500` (Desolator deploy), `[NukePayload]/[CRNuke]/[Nukebomb]=500`, `[Demobomb]=100` (`ini/rulesmd.ini`, `[Radiation]` at line 913).
- **RadSiteClass:** 0x74 B, vtable `0x007F0810`. Fields: `+0x40/+0x42` center X/Y, `+0x44` spread cells, `+0x48` radius leptons = `spread*256+128`, `+0x4C` level, `+0x6C` duration = `RadDurationMultiple × RadLevel`, `+0x70` remaining frames, **`+0x24` = `LightSourceClass*`** (the glow handle).
- **Spread** (`SetCellRadLevels 0x0065B9C0`): (2·spread+1)² square; per cell 3D lepton distance (incl. height); `cell.RadLevel += (radius−dist)/radius × level` when `dist ≤ radius`. Additive across different-center sites.
- **Decay** (`RadSiteClass::AI 0x0065B800`, per tick per site): `remaining -= 1`/tick; every `RadLevelDelay` frames a per-cell decay step subtracts `falloff/levelSteps`; self-deletes at `remaining < 1` (dtor clears center `+0xF8`). Ghidra label "ApplyRadDamage" on `0x0065BD00` is WRONG — it is the decay step (noted in study).
- **Damage** (`FootClass::AI 0x004DA530`, every `frame % RadApplicationDelay == 0`): FootClass-only, buildings never; `ImmuneToRadiation` (`TechnoType+0xD37`) exempt.

### 1b. Render / LightSource glow facts (the render TODO)

The single load-bearing source is **study §2.6 "Green glow" row** + **RADIATION_EMP §1.6/§1.8/§1.11**. All **VERIFIED-DOC**:

- **LightSource creation:** in `RadSiteClass::Activate 0x0065B580` step 5 — creates a `LightSourceClass` (ctor `0x00554760`) at the center cell's 3D coords on first activation; on a merge it updates the existing light's intensity/tint instead.
- **Initial intensity (study §2.6 — the formula the open-items doc quotes):**
  `intensity = ftol(min(level × RadLightFactor, 2000.0))`.
  RADIATION_EMP §1.6 states the un-clamped form `LightIntensity = ftol(RadLevel × RadLightFactor)`; the study's LIVE-0610 decode adds the **`min(…, 2000.0)` clamp** — treat 2000 as the verified cap (matches `LIGHT_CLAMP_MAX = 2000` already in `map/lighting.rs`). Stock: `level=500 × RadLightFactor` — see §3 caveat on the stock factor value.
- **Initial tint (study §2.6):** per channel `tint = min(ch × 1000/255 × RadTintFactor, 2000)` where `ch` = the RadColor R/G/B byte. RADIATION_EMP §1.6 gives the cruder `TintR/G/B = ftol(RadColor.ch × RadTintFactor)`; the study form (the `×1000/255` normalize into the engine's `1000==1.0` light units, then the 2000 clamp) is the more precise one and matches the LightSource tint-unit convention (`+0x28/2C/30` are ×1000).
- **Per-tick fade** (`RadSiteClass::AI 0x0065B800`, every `RadLightDelay` frames — RADIATION_EMP §1.8):
  - tint fades **linearly with remaining lifetime**: `newCh = (TintCh × RemainingDuration) / TotalDuration`.
  - intensity steps down by a precomputed decrement: `LightIntensityPerStep = TotalDuration / RadLightDelay`; `LightDecrement = LightIntensity / LightIntensityPerStep`; each light-step does `newIntensity = intensity − LightDecrement`.
  - Update is **immediate mode** (`0x00554AA0(...,0)` — queue flag zero; confirmed by `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS`). So the glow recomputes affected cells the same tick the parameters change.
- **Tick cadence:** intensity step is keyed on `RadLightDelay` (stock 90), independent of the `RadLevelDelay` (stock 90) field-decay timer — they happen to share the stock value but are separate timers (study §2.6 + GLOBAL_TIMING).
- **EMP sparkle anim:** RADIATION_EMP §1.11 — `EMPulseSparkles` (`RulesClass+0x17F4`) is also played; shared with EMP. INFERRED relevance: a secondary visual the render design may want, separate from the light/tint glow itself.
- **SNOW-theater channel forcing (study §2.6 "Green glow" row):** on SNOW theater (`Scen+0x1258==1`) the R/B channels are force-adjusted — **exact intent UNKNOWN** (flagged unverified in the study; do not invent the formula).
- **LightSource field units** (`LIGHTSOURCE_DIRTY_SCHEDULING` §): `+0x24` intensity, `+0x28/+0x2C/+0x30` R/G/B tint, all integer-scaled `1000 == 1.0`; affected-cell radius lives at `+0x44` (`LightVisibility` in leptons for building lamps). Same struct radiation reuses.

---

## 2. Current Rust radiation state — full API surface the render layer can read

File `src/sim/radiation.rs` (**VERIFIED-CODE**, read this lane). Sim-owned, in `sim/` (render may read it, never the reverse — the #1 invariant).

### 2a. `RadiationState` public API (`src/sim/radiation.rs`)

```rust
pub struct RadiationState { /* private: cells: BTreeMap<(u16,u16),f64>, sites: BTreeMap<(u16,u16),RadSite> */ }   // :67-74
```
Public methods (signatures verbatim):
- `pub fn cell_level(&self, cell: (u16, u16)) -> f64`  — `:145` raw, un-clamped level.
- `pub fn damaging_level(&self, cell: (u16, u16), level_max: i32) -> i32`  — `:151` `trunc(min(level, RadLevelMax))` (damage clamp only).
- `pub fn site_at(&self, center: (u16, u16)) -> Option<&RadSite>`  — `:158`.
- `pub fn sites(&self) -> impl Iterator<Item = &RadSite>`  — `:162` (sorted by center, BTreeMap order).
- `pub fn iter_cells(&self) -> impl Iterator<Item = (&(u16, u16), &f64)>`  — `:168` deterministic, sorted by coord; doc-comment already says "for state hashing **and the render glow layer**".
- `pub fn is_empty(&self) -> bool`  — `:173`.
- `pub fn current_site_level(site: &RadSite) -> i32`  — `:180` `remaining × level / duration` (int); the deployed-Desolator re-fire gate.
- Mutators (sim-only, render must not call): `apply_detonation` `:259`, `tick_decay` `:309`.

### 2b. `RadSite` struct — public fields (`src/sim/radiation.rs:37-64`, all `pub`)

```rust
pub struct RadSite {
    pub center: (u16, u16),         // center cell
    pub spread: i32,                // whole cells; affected square = (2·spread+1)²
    pub radius_leptons: i32,        // = spread × 256 + 128
    pub level: i32,                 // level at last (re)activation
    pub level_steps: i32,           // duration / level_delay (int div)
    pub duration: i32,              // total lifetime frames = duration_multiple × level
    pub remaining: i32,             // frames left; site dies at remaining < 1
    pub level_timer_start: u32,     // frame the decay countdown last (re)started
    pub level_timer_duration: i32,  // countdown length (level_delay at activation)
}
```

### 2c. Verdict on the open-items claim "`sites()/iter_cells()` already expose everything render needs"

**Partially TRUE — with two concrete gaps.** What render CAN read today:
- **Per-cell field** for a tint-by-cell-level approach: `iter_cells()` gives every irradiated cell + its `f64` level, deterministic order. (VERIFIED-CODE.)
- **Per-site state** for a native-faithful per-site LightSource approach: `sites()` gives `center`, `spread`, `radius_leptons`, `level`, **`remaining` and `duration`** — i.e. everything to compute native intensity `min(level × RadLightFactor, 2000)` and the linear tint fade `(TintCh × remaining) / duration`, plus the falloff radius. (VERIFIED-CODE.)

**Gaps — what `sites()`/`iter_cells()` do NOT expose (so the claim is not 100%):**
1. **No glow rules on the site.** `RadSite` stores **no** `RadColor`, `RadLightFactor`, `RadTintFactor`, or `RadLightDelay`. Those live on `RuleSet.radiation: RadiationRules` (see §2d), which the render layer must thread in separately (it is NOT carried by `RadiationState`). The struct comment at `radiation.rs:37` and `ruleset.rs:951` ("kept here so the render layer can pick them up later") both assume render reaches into `RadiationRules`.
2. **No `RadLightDelay` light-timer on the site.** `RadSite` carries only the **level** timer (`level_timer_start` / `level_timer_duration`, armed from `RadLevelDelay`). The native glow steps on a **separate `RadLightDelay` timer** (§1b). Stock RadLightDelay==RadLevelDelay==90 so they coincide, but there is no independent light-timer field — a faithful per-site intensity step-down would need either a new sim field or the render layer to derive the light step itself from `remaining`/`duration`/`light_delay`. INFERRED: render can fully derive the fade from `remaining/duration` + the rules without new sim state (the tint fade is a pure function of `remaining/duration`; the intensity step is `level×factor` scaled the same way), so a new field is likely avoidable — but this is a design decision, not a current capability.

Net: `iter_cells()` is sufficient for a **per-cell additive-tint** render (no per-site light object), and `sites()`+`RadiationRules` is sufficient for a **per-site LightSource** render of intensity+tint — but the glow color/factor constants are NOT on the sim state and must come from `RuleSet`.

### 2d. Glow constants — already parsed, sitting in `RadiationRules` (`src/rules/ruleset.rs`)

`RuleSet.radiation: RadiationRules` (`ruleset.rs:1633`, parsed `:1699`). Render-relevant fields (**VERIFIED-CODE**, `ruleset.rs:953-979`), all flagged "Render-only" in their doc-comments:
- `pub light_delay: i32`        (`RadLightDelay`) — `:964`
- `pub light_factor: SimFixed`  (`RadLightFactor`) — `:972`
- `pub tint_factor: SimFixed`   (`RadTintFactor`) — `:974`
- `pub color: (u8, u8, u8)`     (`RadColor=R,G,B`) — `:976`
- plus `duration_multiple`, `level_delay`, `level_max`, `level_factor`, `site_warhead` (sim-side).
Defaults (`:981-996`): `light_delay 90`, `light_factor sim_from_f32(0.1)`, `tint_factor sim_from_f32(1.0)`, `color (0,255,0)` — **pure green**, the visible glow color. `level_max 500`.

> **CAVEAT (must verify against `ini/rulesmd.ini` `[Radiation]` in the design pass):** the test fixture in `radiation.rs:367-380` uses `light_factor = 0.1`, `tint_factor = 1.0`, `color = (0,255,0)`, `level_max = 500`. With `level 500 × light_factor 0.1 = 50` intensity (well under the 2000 clamp). These are the **Rust defaults**, not confirmed-from-ini values this lane — the design must read the actual stock `[Radiation]` keys before pinning numbers. (Flagged because the open-items doc's "intensity `min(level×RadLightFactor,2000)`" example implies the clamp matters; at stock factor 0.1 it does not bind.)

### 2e. Render-layer radiation consumers today: NONE

`grep -i "radiation|RadiationState|rad_glow|RadColor|rad_light" src/render/` → **no matches** (VERIFIED-CODE). The render layer reads zero radiation state today. Confirms the boundary is clean: sim produces, render consumes-nothing-yet.

### 2f. Closest existing render-light primitive (reuse candidate, NOT dynamic)

`src/map/lighting.rs` (**VERIFIED-CODE**) already has a CPU linear-falloff point-light system:
- `pub struct PointLight { rx, ry, center_x, center_y, radius_leptons, intensity, tint: [i32;3], active, detail }` — `:485`. Intensity/tint in `1000 == 1.0` units (same convention as the native LightSource `+0x24/28/2C/30`).
- `pub fn accumulate_point_lights(grid: &mut CellLightGrid, lights: &[PointLight])` — `:574`; contribution `((range − distance)/range) × intensity`, signed-summed then one clamp/channel. `LIGHT_CLAMP_MAX = 2000` (`:32`) — **matches the native glow's 2000 intensity/tint cap**.
- `point_light_from_object(...)` `:539`, `collect_building_lights(...)` `:509`.

This is a **map-load-time / static bake into `CellLightGrid`** (building lamps), driven by `MapEntity`, not a per-tick dynamic recompute. INFERRED: the radiation glow is the **dynamic, per-tick analog** of exactly this — same falloff math, same units, same 2000 cap, but the source set changes every tick (sites spawn/decay/die) and must be rebuilt/applied each frame from `RadiationState`, not baked once at load. The `LIGHTSOURCE_DIRTY_SCHEDULING` doc's whole "affected-cell recompute on toggle" is the native machinery that this dynamic rebuild stands in for. No `src/render/**/*light*` file exists; there is no existing dynamic-light-per-tick path to slot into — that infrastructure is the actual TODO.

---

## 3. Commit `86b0d4bf` — exactly what sim-core landed (the done/todo boundary)

`git show --stat 86b0d4bf` (**VERIFIED-CODE**). Title: "sim: substrate Slice 7 — per-cell radiation field service (closes study §4.2 #12)". 14 files, +1139/−9.

**DONE (sim, this commit):**
- `RadiationState` (`sim/radiation.rs`, new, 579 lines): site registry + sparse per-cell `f64` field; (2·spread+1)² additive linear falloff over 3D lepton distance (level×104 Z); radius = `CellSpread×256+128`; per-site activation-anchored countdown decay (`falloff/level_steps` per `RadLevelDelay`); same-center merge / different-center stack; self-delete < remaining 1.
- `[Radiation]` parsed into `RadiationRules` (`rules/ruleset.rs`, +105) — **including all four render-only glow keys** (`RadColor`, `RadLightFactor`, `RadTintFactor`, `RadLightDelay`); `RadSiteWarhead` added to the referenced-warhead set.
- `ImmuneToRadiation` on `ObjectType` → copied to `GameEntity` at spawn (`object_type.rs`, `game_entity.rs`, `world_spawn.rs`).
- Combat (`combat/mod.rs` +170, `combat_weapon.rs`): `RadLevel>0` detonations emit `RadDetonation`; field folds in before the damage phase; periodic foot-unit damage `trunc(trunc(min(level, RadLevelMax)) × RadLevelFactor) × Verses/100` every `RadApplicationDelay` frames, sourceless (no retaliation); deployed-Desolator self-fire below `RadLevel/3`.
- World (`world/mod.rs`): field persisted + state-hashed (zero folds while empty — golden baselines unshifted); decay runs after the combat phase. `world_hash.rs` (+22). `SNAPSHOT_VERSION 20 → 21` (`snapshot.rs`).
- Tests: falloff exactness, application-delay boundary, verses scaling, building/immunity exemptions, merge-vs-stack, lifetime/residue, activation-anchored countdown, re-fire loop, serde round-trip.

**NOT done (the render TODO this design covers):**
- **No LightSource / dynamic glow.** Nothing in this commit touches `render/`, `map/lighting.rs`, or any light path. The render-only glow keys are parsed and **sit unused** in `RadiationRules` (their doc-comments literally say "Render-only" / "kept here so the render layer can pick them up later").
- No EMPulseSparkles anim spawn.
- No SNOW-theater channel handling.

**Precise boundary:** sim owns the per-cell `f64` field + per-site lifecycle/timers and exposes them read-only via `RadiationState::sites()` / `iter_cells()` / `current_site_level()`. The render layer must (a) read that state each tick, (b) thread in the four glow constants from `RuleSet.radiation`, (c) compute native intensity `min(level×RadLightFactor, 2000)` and tint `min(ch×1000/255×RadTintFactor, 2000)` faded by `remaining/duration`, and (d) apply that as a **dynamic** per-tick light contribution onto the cell tint — the dynamic analog of the static `accumulate_point_lights` bake in `map/lighting.rs`, which does not yet exist as per-tick infrastructure.

---

## Open questions for the design pass (flagged, not resolved here)
1. **Per-cell tint vs per-site LightSource.** Both are feasible from current state. Per-cell (`iter_cells()`) is simpler and matches the falloff that the sim already baked into the field; per-site (`sites()` + falloff recompute) is closer to the native LightSource object and gets the intensity-vs-tint separation (intensity is `level×factor`, tint is `color×remaining/duration`) for free. The native engine uses **one LightSource per site**, not per-cell. UNKNOWN which produces the bit-identical visible result — needs a design decision + parity check.
2. **Stock factor values** — confirm `[Radiation]` `RadLightFactor`/`RadTintFactor`/`RadColor`/`RadLevelMax` from `ini/rulesmd.ini` (§2d caveat); the 2000 clamp may or may not bind at stock factors.
3. **Two timers (`RadLevelDelay` vs `RadLightDelay`).** `RadSite` only stores the level timer. Decide whether render derives the light step from `remaining/duration` (no new sim field) or a `RadLightDelay` field is added to `RadSite` (sim change → snapshot bump). Tint fade is a pure function of `remaining/duration` so it needs no new state; the intensity step-down likely doesn't either. (INFERRED.)
4. **SNOW-theater R/B forcing** — intent UNKNOWN per study §2.6; needs a fresh Ghidra read before implementing (do not invent).
5. **EMPulseSparkles** anim — secondary visual, separate from the light glow; in scope? (RADIATION_EMP §1.11.)

---

## Addendum — live-binary re-verification of `RadSiteClass::Activate` (2026-06-15)

The §1b glow facts above are tagged VERIFIED-DOC (not re-decompiled in the doc's own lane). One
live re-verification was run this pass to promote the load-bearing activation flow to
**VERIFIED-FROM-BINARY** (`mcp__ghidra-mcp__decompile_function 0x0065B580`, `RadSiteClass__Activate`):

- Reads RadLevelDelay (`g_RulesClass+0x1810`) → stores `+0x30`; RadLightDelay (`+0x1814`) → `+0x3c`.
  Confirms the **two separate timers** (level vs light) — matches §2c gap #2 and §1b.
- Four `Math__ftol()` results → intensity `+0x54` and tint `+0x58/+0x5C/+0x60`. Confirms the
  `ftol(...)` integer-truncation of the intensity/tint products in §1b.
- Derives `+0x50 = duration(+0x6c) / RadLevelDelay`, `+0x64 = duration / RadLightDelay`,
  `+0x68 = intensity / lightSteps(+0x64)` — confirms `LightDecrement = LightIntensity /
  LightIntensityPerStep` (§1b per-tick fade) directly.
- `MapClass__Get_CellClass(this+0x40)` → center-cell 3D coord via cell vtable slot `+0x48`, then:
  **first activation (`+0x24 == 0`)**: `operator_new(0x4c)` + `LightSourceClass__Constructor(...)`
  (LightSource is **0x4C bytes**), store `+0x24`, zero `light+0x34`, `CreateProductionAnim(0)`,
  `SetCellRadLevels`; **else (merge)**: `FUN_00554aa0(intensity, …, tint…, 0)` updates the existing
  light in place — the trailing `0` confirms **immediate (un-queued) mode** (§1b / queued-mode census).

Not re-verified this pass (still VERIFIED-DOC): the AI per-tick fade body (`0x0065B800`), the
`×1000/255` tint channel scaling, the 2000 caps, and the SNOW-theater R/B force. Re-read
`LightSourceClass__Constructor 0x00554760` + the cell-light apply `0x00484180` before pinning
exact glow integers.
