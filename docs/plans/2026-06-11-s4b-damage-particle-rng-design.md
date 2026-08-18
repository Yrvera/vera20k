# Slice S4b — Damage-particle RNG (consumption-only / stream-align) — DESIGN

**Status:** ✅ **IMPLEMENTED 2026-06-11 (faithful + dormant).** Draw core, hashed `+0x308`
lifecycle, bit-exact x87 threshold, `SNAPSHOT_VERSION` 24→25, goldens re-baselined (shift proven
to be the field-fold only). **One spec correction landed during implementation — read §0.**

## 0. SPEC CORRECTION (binary-verified during implementation) — the gate is `Cyborg`, not "has-Spark"

The plan's open assumption (§3/§8 step 2 — *"`Type+0xC8F` derives from has ≥1 Spark
`DamageParticleSystems`"*) is **WRONG**. Verified against gamemd:
- `Type+0xC8F` is written in exactly one place — `InfantryTypeClass::ReadINI` sets it from the
  **`Cyborg=`** bool (`ReadBool → +0xEAC`; `if (+0xEAC) +0xC8F = 1`; key string @ `0x825a0c` =
  `"Cyborg"`). `UnitTypeClass::ReadINI` (`0x00747620`, fully decoded) and all other leaves **never
  write it**, so every vehicle/building/aircraft keeps the ctor default `0` (ctor `0x00711395`
  BL=0; byte-sweep for the `0xC8F` displacement finds only 4 sites — ctor-zero, infantry-zero,
  infantry-Cyborg-set, AI_Update-read).
- Stock YR has **zero `Cyborg=yes` units** (`Cyborg=` never appears in `rulesmd.ini`). The 141
  vehicles that set `DamageParticleSystems=SparkSys,…` keep `+0xC8F = 0`.
- ⇒ The AI_Update spark draw (this slice's `+0x308` path) **NEVER fires in stock YR** — `Cyborg`
  is a Tiberian Sun ghost. Implementing the plan's has-Spark hypothesis would draw for 141 stock
  vehicles gamemd does NOT draw for → **desync vs gamemd.**
- The visible smoke on damaged vehicles is the *separate* `+0x310` path, not this one.

**Implemented (user chose "faithful + dormant"):** the gate keys off the real
`ObjectType::emits_damage_spark()` = `Cyborg && category==Infantry` (mirrors "only
`InfantryTypeClass::ReadINI` sets `+0xC8F`"). In the current host (`techno_common_post` runs for
the **vehicle** arm only) it is therefore always false → zero draws → the golden shift is purely
the new hashed-field fold (proven by re-running the baselines with the fold line disabled). The
draw becomes live for `Cyborg=yes` infantry once S6 hosts the infantry arm. Everything below is
correct EXCEPT the §3/§8 "emits-damage-particles derives from has-Spark" line.

**Verified spec:** this doc + `docs/research/TECHNOCLASS_AI_UPDATE_BODY_GHIDRA_REPORT.md §4/§7` (live
decode), plus §0 above. **Lockstep-critical:** wrong draw count/stream/position = full-match desync.
**Date:** 2026-06-11
**Parent:** `2026-06-10-s4-techno-common-prepost-design.md` (S4 split; user chose consumption-only).

## 1. Verified gamemd behavior (the spec)

In `TechnoClass::AI_Update`, post-`Mission_Dispatch`, after the anim-stage step
(`0x006facd1–0x006faee0`), per object, per tick:

- **Outer gate:** `Type+0xC8F != 0` (type emits damage particles) AND `HealthRatio < ConditionYellow`
  (`Rules+0x1700`) AND `vtable+0x1c8() > -10` (not in a special damage state).
- Build the **Spark list**: filter `DamageParticleSystems` (`Type+0x77c`/`+0x788`) to those whose
  `ParticleSystemType+0x2b4 == 3` (`BehavesLike == Spark`). **No RNG.**
- **Inner gate:** `this+0x308 == 0` (no live damage-particle system on this object) AND Spark count > 0.
- If inner gate passes:
  - **Draw #1** (always): `RandomRanged(0, 0x7ffffffe)` on **`Scen->Random`** — the spawn-probability
    roll. Compare `roll · 2^-31ish < band`, band = `Rules+0x558/+0x55c` (red, HealthRatio < ConditionRed
    `Rules+0x1708`) or `Rules+0x560/+0x564` (yellow).
  - If the roll **succeeds**: `operator_new`, deterministic offset (no RNG), **Draw #2**:
    `RandomRanged(0, sparkCount-1)` on **`Scen->Random`** (list-pick), spawn the system, store at `+0x308`.

**Draw truth table (Scen->Random):** 0 (gate fail) / 0 (`+0x308`≠0 or no Spark) / 1 (roll fail) /
2 (roll success). `ParticleSystemClass__Constructor` and the offset helper draw **nothing**.

## 2. The `+0x308` lifecycle is load-bearing (the hard part)

The draws live **inside** the `+0x308 == 0` branch. Once a system spawns, `+0x308` is non-null and
the object makes **zero** damage-particle draws until `+0x308` clears (the spawned
`ParticleSystemClass` finishes its lifetime and nulls the owner pointer; AI_Update never clears it).
So the per-object draw cadence is: *while below ConditionYellow and `+0x308` empty → 1 roll/tick; on
success → 0 draws for the spawned system's lifetime.*

**Consequence for consumption-only:** S4b must model a **sim-side `+0x308`-equivalent** — a per-entity
"damage-particle system live until frame X" state — or the draw count drifts (over-drawing every tick
instead of gating on the live system). This is NOT draw-and-discard; it is a faithful gate model.
- Set on roll-success: `live_until = frame + sparkSystemLifetime`.
- While `frame < live_until`: skip the roll (matches `+0x308 != 0`).
- On expiry: clear, resume rolling.
- `sparkSystemLifetime` is data-driven — the chosen Spark `ParticleSystemType`'s lifetime. **Open:**
  confirm how long the gamemd spark system holds `+0x308` (its `Lifetime`/hold duration); the draw
  cadence depends on it. (`+0x310` smoke, destroyed on recovery-above-yellow, is a DIFFERENT pointer —
  do not conflate.)

## 3. VERA infrastructure (confirmed present)

- **Stream:** `sim.scenario_rng` (the `Scen->Random` equivalent; `sim.main_rng` is `g_MainRng`). Both
  in `ScenarioSession`, both hashed (`scenario_session.rs`). S4b draws from `scenario_rng`.
- **Helper:** `SimRng::next_range_u32_inclusive(0, 0x7ffffffe)` and `(0, count-1)` — the mask-and-reject
  variant matching `RandomRanged` (rejects only the `0x7fffffff` aliasing sample; ~1 draw).
- **Parsed (confirmed present):** `ConditionYellow` (`condition_yellow_x1000`) + `ConditionRed`
  (`ruleset.rs:979`); `DamageParticleSystems` CSV (`object_type.damage_particle_systems`);
  **`ParticleSystemBehavesLike` with `Spark = 3`** (`particle_system_type.rs:42`, field `behaves_like`
  at `:71`) — the Spark filter is ready. **Still needed:** the spawn-chance bands (`Rules+0x558/+0x55c`
  red, `+0x560/+0x564` yellow — `[AudioVisual]` damage-particle chances; INI key + parse to identify),
  the spark-system lifetime (`+0x308` hold), and confirming `Type+0xC8F` (emits-damage-particles) is
  derivable from "has ≥1 Spark `DamageParticleSystems`".

## 4. Approach

Reproduce the **consumption** at the native position (post-mission, after the anim-stage step) in the
S4a host bracket's `techno_common_post`, drawing from `sim.scenario_rng`:
1. Per live Unit, evaluate the outer gate (emits-damage-particles + HealthRatio<ConditionYellow + not
   special-state). Below-yellow needs the integer HealthRatio compare (`condition_yellow_x1000`).
2. Gate the Spark list (count of `DamageParticleSystems` with `BehavesLike==Spark`). If zero, no draw.
3. Inner gate on the sim-side `+0x308`-equivalent live-system state. If live, no draw.
4. Draw the prob-roll from `scenario_rng`; pick the band by HealthRatio vs ConditionRed. On success,
   draw the list-pick, set the live-system state (`live_until = frame + lifetime`). **Visual stays
   render-side** — S4b spawns no sim particle, only consumes the draws + tracks the gate state.

`SNAPSHOT_VERSION` bump + golden re-baseline (new `scenario_rng` draws shift the hash). The per-entity
live-system state is hashed (it gates future draws).

## 5. Acceptance tests

- `s4b_no_draw_above_condition_yellow` — a Unit at/above ConditionYellow consumes **zero** scenario_rng draws.
- `s4b_one_draw_when_roll_fails` — below-yellow, `+0x308`-empty, prob-roll fails → exactly 1 draw.
- `s4b_two_draws_when_roll_succeeds` — roll succeeds → 2 draws (roll + list-pick); live-system state set.
- `s4b_no_draw_while_system_live` — after a successful spawn, **zero** draws until `live_until` expires.
- `s4b_draws_from_scenario_not_main` — the draws move `scenario_rng`, not `main_rng`.
- `s4b_zero_draw_without_spark_systems` — a below-yellow Unit whose `DamageParticleSystems` has no
  Spark-`BehavesLike` entry consumes zero draws.
- `s4b_golden_rebaselined` — `SNAPSHOT_VERSION` bumped; replay deterministic; baseline re-measured.

## 6. Spawn-chance band keys (RESOLVED 2026-06-11, disasm-verified)

Both are `[General]` doubles, read in `RulesClass__ReadGeneral` via `CCINIClass__ReadDouble
(0x005283d0)`. Offset↔key mapping **verified from the store sites** (`get_assembly_context`):
- **`ConditionRedSparkingProbability`** → `Rules+0x558` (`FSTP [ESI+0x558]` @ `0x006718d2`) — used
  when `HealthRatio < ConditionRed`.
- **`ConditionYellowSparkingProbability`** → `Rules+0x560` (`FSTP [ESI+0x560]` @ `0x006718ab`) — used
  when `ConditionRed ≤ HealthRatio < ConditionYellow`.

**Default values — VERIFIED 2026-06-11 (my 0.0 hypothesis was WRONG).** Stock INI sets neither key,
so the parity values are the RulesClass ctor defaults (`RulesClass__Constructor 0x00665650`,
construction at `0x0052bac3`):
- **`ConditionRedSparkingProbability` (+0x558) = 0.02** (`0x3f947ae147ae147b`; ctor `param_1[0x156/0x157]`).
- **`ConditionYellowSparkingProbability` (+0x560) = 0.01** (`0x3f847ae147ae147b`; ctor `param_1[0x158/0x159]`).

So the AI_Update Spark effect is **ON by default** (~2% red-band / ~1% yellow-band per tick): a
damaged spark-capable unit below ConditionYellow with `+0x308` empty draws the prob-roll (1 draw)
each tick, and on the ~1–2% it succeeds, draws the list-pick (2nd draw) AND spawns a system (so
`+0x308` then holds for `sparkType.Lifetime` ticks with **zero** draws). The `+0x308` lifecycle is
therefore load-bearing — this is NOT the "always 1 draw" simplification the 0.0 hypothesis implied.

The prob compare is `(double)roll · DAT_007e3570 < band` (`roll` = `RandomRanged(0,0x7ffffffe)`),
where `DAT_007e3570 ≈ 1/INT_MAX`. **Reproduce in fixed-point**, not float (sim determinism). These
keys need parsing into VERA's `RuleSet` (not currently present — grep found neither).

## 7. `+0x308` lifecycle (RESOLVED 2026-06-11, verified)

The spawned spark `ParticleSystemClass` lives for the chosen spark **`ParticleSystemType.Lifetime`**
ticks, then self-removes and the owner's `+0x308` nulls (re-rolling resumes):
- `ParticleSystemClass__AI 0x0062fd60`: `param_1[0x3b]--; if (--==0) vtable+0xf8()` (remove the
  system). The lifetime counter `+0x3b` is set in the ctor from `Type+0x2b8`.
- `ParticleSystemTypeClass__ReadINI 0x006442d0`: `Type+0x2b8 = ReadInt("Lifetime")` (and
  `Type+0x2b4 = BehavesLike` index, `Type+0x29c = SpawnFrames`).
- So **`+0x308`-equivalent: `live_until = spawn_frame + sparkType.Lifetime`**; while
  `frame < live_until` the object makes **zero** damage-particle draws (matches `+0x308 != 0`).
  (`+0x310` smoke is a DIFFERENT pointer — do not conflate.)

## 7b. The bit-exact prob compare (the one delicate part)

gamemd: `if ((double)roll * DAT_007e3570 < band)` where `roll = RandomRanged(0, 0x7ffffffe)`,
`band` ∈ {0.02 red, 0.01 yellow}, and **`DAT_007e3570 = 0x3E00000000400000`** (the exact IEEE-754
double ≈ `2^-31 + 2^-61` ≈ 4.6566e-10; read 2026-06-11 `read_memory 0x007e3570`).

For sim determinism (no per-tick float), reproduce it as an **integer threshold compare**:
- At rules-init (NOT per tick), compute, **per band**, the integer `threshold` = the number of
  `roll` values in `[0, 0x7ffffffe]` for which `(f64)roll * 0x3E00000000400000 < band` holds — i.e.
  the boundary of the gamemd multiply, found by evaluating the EXACT f64 multiply (binary-search the
  flip point; do NOT use `band/scale` division — the divide can round differently from the multiply
  at the boundary, and a 1-off there flips the draw count on that tick → desync).
- Per tick: `let roll = scenario_rng.next_range_u32_inclusive(0, 0x7ffffffe); let spawn = roll < threshold;`
- f64 at init is IEEE-754-deterministic across machines, so the threshold is a deterministic input;
  the per-tick compare is pure integer. Pin the two thresholds (red/yellow) in a test against the
  exact f64 boundary.

## 8. Remaining implementation steps

**Already present:** `ParticleSystemType.lifetime` (parsed, default -1), `BehavesLike (Spark=3)`,
`ConditionYellow/Red` (+x1000), `DamageParticleSystems`, `sim.scenario_rng`.

**One tiny Ghidra item:** `ConditionRed/YellowSparkingProbability` are **NOT in stock INI**, so the
parity value is the RulesClass **ctor default** for `+0x558`/`+0x560` (decode the RulesClass ctor
init). All stock units use that default — it must be exact (it decides the roll-success → 1-vs-2
draw count).

**Rust-side (no Ghidra):**
1. Parse `ConditionRed/YellowSparkingProbability` into `GeneralRules` (fixed-point) with the verified
   ctor default; reproduce the compare `(double)roll · (1/INT_MAX) < band` in fixed-point.
2. `Type+0xC8F` (emits-damage-particles gate) — derive from "has ≥1 Spark `DamageParticleSystems`".
3. Add a per-entity **hashed `damage_particle_live_until`** (the `+0x308`-equivalent;
   `= spawn_frame + sparkType.lifetime`).
4. Draw in `techno_common_post` (S4a host) from `sim.scenario_rng`, 0/1/2 per §1; `SNAPSHOT_VERSION`
   bump + golden re-baseline (new Scen->Random draws shift damaged-unit scenarios — verify the shift
   is only the damage-particle draws before re-baselining).
2. **Parse** `ConditionRed/YellowSparkingProbability` into `RuleSet` (fixed-point) + the
   `DAT_007e3570` scale. `BehavesLike (Spark=3)`, `ConditionYellow/Red`, `DamageParticleSystems`,
   `scenario_rng` are all already present.
3. **`Type+0xC8F`** (emits-damage-particles gate) — confirm it derives from "has ≥1 Spark
   `DamageParticleSystems`" (then no separate parse needed).
4. **Position** — S4b draws inside `techno_common_post` (S4a host); land the S4a flip first, or host
   the draw at the equivalent post-combat point temporarily.
