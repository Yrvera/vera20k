# Grizzly BuildTimeMultiplier Consumer - Ghidra Research Report

**Address(es):** `0x006F47A0` (production-time consumer), `0x00711EE0` (`TechnoTypeClass::GetBuildTime`), `0x004C9EA0` / `0x004C9FB0` (factory rate setters), `0x007C5F00` (`Math__ftol`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Stock Yuri's Revenge Grizzly/`MTNK` production timing through `TechnoType+0x608 BuildTimeMultiplier`, including formula order, truncation points, `[General] BuildSpeed` source, and the concrete normal-power one-factory build-time value.  
**Non-Scope:** Placement, prerequisites, sidebar UI drawing, queue cancellation, AI headstart, unrelated factory ownership, and unrelated Grizzly combat/movement behavior.  
**Confidence:** High for direct formula/order/truncation and stock `MTNK` numeric result; Medium for house-type multiplier defaults because this slot used INI absence plus prior reports instead of re-decompiling every `HouseTypeClass` default writer.  
**Active in YR:** Yes.

## Target Question

How does `gamemd.exe` consume `BuildTimeMultiplier=1.5` for stock YR Grizzly/`MTNK` production timing, and does `Cost=700` plus `[General] BuildSpeed=.7` produce a concrete build-time value different from current Rust assumptions?

## Evidence Needed To Mark Complete

- Verify `BuildTimeMultiplier` reader and field offset.
- Verify the production-time consumer reads `TechnoType+0x608`.
- Verify the formula order and each float-to-int conversion point.
- Verify `[General] BuildSpeed` source/default enough to evaluate stock `rulesmd.ini`.
- Verify the factory rate division/clamp that consumes the total time.
- Compare the stock `MTNK` value to current Rust `build_time_base_frames`.

## Stop Conditions

- Stop if no read of `TechnoType+0x608` appears on the live production path.
- Stop if `0x006F47A0` is not reachable from `FactoryClass::SetRate` / rate recalculation.
- Stop after the formula, numeric stock result, and Rust delta are resolved; do not expand into sidebar or production placement.

## 1. Overview

`TechnoTypeClass::GetBuildTime` computes only the base cost/global-speed value. The live production-time consumer at `0x006F47A0` then applies the house/category build-time multiplier, truncates, applies `TechnoType+0x608 BuildTimeMultiplier`, truncates again, and only then applies power/multiple-factory/wall modifiers.

For stock `MTNK` under normal power with one factory and no active house-type build-time override, the build-time value before factory rate division is `661` frames: `trunc(trunc(700 * 0.7 * 0.9) * 1.5)`. The stale `(Cost / BuildSpeed) * BuildTimeMultiplier` wording is wrong.

## 2. Class Layout / Key Offsets

| Offset | Owner | Type | Meaning | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `+0x608` | `TechnoTypeClass` | `float` | `BuildTimeMultiplier` | reader xref `TechnoTypeClass::ReadINI @ 0x00714371`; consumer `FMUL float ptr [EAX + 0x608]` at `0x006F47EE` | Yes |
| `+0x610` | `TechnoTypeClass` | `int32` | `Cost` | `FILD dword ptr [ECX + 0x610]` at `0x00711EE0`; `[MTNK] Cost=700` in `rulesmd.ini:6621` | Yes |
| `+0x1748` | `RulesClass` | `double` | `[General] BuildSpeed` | `FMUL double ptr [EAX + 0x1748]` at `0x00711EEB`; `rulesmd.ini:41` | Yes |
| `+0x57C` | `RulesClass` | `float` | `MultipleFactory` | `MOV EDX,[EAX + 0x57c]` at `0x006F48DB` | Yes |
| `+0x570/+0x574/+0x578` | `RulesClass` | `float` | low-power min/max/modifier | reads at `0x006F4828`, `0x006F4852`, `0x006F481C` | Yes |
| `+0x758` | `RulesClass` | `double` | `WallBuildSpeedCoefficient` | `FMUL double ptr [EDX + 0x758]` at `0x006F493D` | Conditional; wall-only |
| `+0x138` | `HouseTypeClass` | `float` | unit build-time category multiplier | `HouseClass__GetBuildTimeBonus @ 0x0050C0A0`, case `0x28` | Conditional; stock YR data leaves active keys commented/default |

## 3. Core Logic

### Base helper

`TechnoTypeClass::GetBuildTime @ 0x00711EE0`:

```text
FILD [this + 0x610]                 ; Cost
MOV  EAX, [0x008871E0]              ; RulesClass instance
FMUL double ptr [EAX + 0x1748]      ; [General] BuildSpeed
FMUL double ptr [0x007F4E80]        ; 0.9
JMP  0x007C5F00                     ; Math__ftol
```

Verified behavior -> `base = trunc_toward_zero(Cost * Rules.BuildSpeed * 0.9)`. Active in YR: Yes.

### Production-time consumer

`0x006F47A0` consumes the base result:

1. Calls the type build-time vtable path and stores the base at `[ESP+0x8]`.
2. Calls `HouseClass__GetBuildTimeBonus @ 0x0050C0A0`, then `FIMUL [ESP+0x8]`, then `Math__ftol @ 0x006F47D7`.
3. Calls the type getter again, then `FILD [ESP+0x8]`, `FMUL float ptr [EAX + 0x608]`, then `Math__ftol @ 0x006F47F4`.
4. Computes low-power speed from `HouseClass__GetPowerRatio @ 0x004FCE30` and `Rules+0x570/+0x574/+0x578`; divides by speed and truncates at `0x006F4886`.
5. If `Rules.MultipleFactory > 0` and factory count is greater than one, multiplies once for each extra factory and truncates each iteration (`0x006F4900..0x006F4914`).
6. If `WhatAmI()==6` and type `+0x1571` wall flag is set, multiplies by `Rules+0x758` and truncates (`0x006F491B..0x006F4943`).

Pseudocode for the Grizzly-relevant normal path:

```text
base = ftol(Cost * Rules.BuildSpeed * 0.9)
step1 = ftol(base * HouseTypeBuildTimeUnitsMult)   ; stock/default = 1.0 for this case
step2 = ftol(step1 * TechnoType.BuildTimeMultiplier)
step3 = ftol(step2 / low_power_speed)              ; normal power => / 1.0
for each extra matching factory:
    step3 = ftol(step3 * Rules.MultipleFactory)
return step3
```

`Math__ftol @ 0x007C5F00` uses `FISTP` under cached FPU control word `0x0E7F`, i.e. round toward zero. Evidence: current disassembly at `0x007C5F00` shows `FNSTCW`, conditional `FLDCW [0x00822D80]`, then `FISTP`; prior binary-backed report `ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md` read `0x00822D80` as bytes `7F 0E`. Active in YR: Yes.

### Concrete Stock Grizzly Result

Inputs:

- `rulesmd.ini:6603` `[MTNK]`
- `rulesmd.ini:6621` `Cost=700`
- `rulesmd.ini:6648` `BuildTimeMultiplier=1.5`
- `rulesmd.ini:41` `[General] BuildSpeed=.7`

Normal one-factory, normal-power, no house override:

```text
base  = trunc(700 * 0.7 * 0.9) = trunc(441.0) = 441
btm   = trunc(441 * 1.5)       = trunc(661.5) = 661
power = trunc(661 / 1.0)       = 661
multi = no extra factory       = 661
wall  = not a wall             = 661
```

Factory rate consumers divide this value by `54` and clamp to `[1,255]`. `FactoryClass::SetRate @ 0x004C9EA0` and `FactoryClass::CalcRate @ 0x004C9FB0` both call `0x006F47A0`, use the magic signed divide-by-54 sequence (`0x4BDA12F7`), then clamp below `1` and above `255`. For `661`, the rate is `661 / 54 = 12`.

## 4. INI Keys

| Key | Location | Stock value | Effect | Active in YR |
|---|---|---:|---|---|
| `[General] BuildSpeed` | `rulesmd.ini:41`, base `rules.ini:37` | `.7` | Multiplies cost in `TechnoTypeClass::GetBuildTime`; field `Rules+0x1748` | Yes |
| `[MTNK] Cost` | `rulesmd.ini:6621`, base `rules.ini:6587` | `700` | Base production-time input at `TechnoType+0x610` | Yes |
| `[MTNK] BuildTimeMultiplier` | `rulesmd.ini:6648` | `1.5` | Per-type multiplier at `TechnoType+0x608`, applied after base and house multiplier | Yes |
| `BuildTimeUnitsMult` family | only commented examples in `rulesmd.ini:3240/3311` for defenses/infantry; no active MTNK-relevant unit override found | default | House/category production multiplier before `BuildTimeMultiplier` | Conditional; no stock Grizzly override observed |
| `[General] MultipleFactory` | parsed elsewhere; read at `Rules+0x57C` | `0.8` in stock docs | Applies after power and after `BuildTimeMultiplier`, once per extra factory | Conditional |
| `[General] LowPower*` | read at `Rules+0x570/+0x574/+0x578` | stock defaults | Divides time by computed production speed when low power | Conditional |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `TechnoTypeClass::ReadINI @ 0x00714371` | parses `BuildTimeMultiplier` into `TechnoType+0x608` | settled xref from string `0x00843CF0` | Yes |
| `TechnoTypeClass::GetBuildTime @ 0x00711EE0` | computes cost/global base only | disassembly `0x00711EE0..0x00711EF7` | Yes |
| production-time consumer `0x006F47A0` | applies house multiplier, `BuildTimeMultiplier`, power, multi-factory, wall | disassembly `0x006F47A7..0x006F494E` | Yes |
| `HouseClass__GetBuildTimeBonus @ 0x0050C0A0` | category-specific multiplier from `HouseTypeClass` | decompile switch cases `3/7/0x10/0x28` | Yes |
| `HouseClass__GetPowerRatio @ 0x004FCE30` | returns `1.0` when power output covers drain | decompile reads `PowerOutput`/`PowerDrain` | Yes |
| `HouseClass__GetFactoryCount @ 0x00500910` | supplies extra-factory count by category/naval flag | decompile switch | Yes |
| `FactoryClass::SetRate @ 0x004C9EA0` | writes step delay/rate after dividing by 54 and clamping | disassembly `0x004C9EEF..0x004C9F34` | Yes |
| `FactoryClass::CalcRate @ 0x004C9FB0` | same rate calculation helper | disassembly `0x004C9FB9..0x004C9FEA` | Yes |

## 6. Current Rust Implementation Status

Current Rust direct base function:

- `src/sim/production/production_tech.rs:302` `build_time_base_frames`
- comments at `:310-311`: base then `BuildTimeMultiplier`
- integer calculation at `:319-321`: `cost * speed_x1000 * 9 / 10000`, then `base * btm_x1000 / 1000`

For stock Grizzly data this produces `441` then `661`, matching the verified direct base plus `BuildTimeMultiplier` order. `src/sim/production/production_queue.rs:216` stores this as `total_base_frames`.

Current Rust also has later effective-time surfaces for power and multiple factories (`production_tech.rs:366` onward), but this slot did not audit their full end-to-end queue cadence against the factory step-delay model. The Grizzly-specific `BuildTimeMultiplier` gap is not a mismatch in `build_time_base_frames`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildTimeMultiplier` parser | verified | string `0x00843CF0`, reader `0x00714371`, prior MTNK audit | none |
| `TechnoTypeClass::GetBuildTime` | verified | `0x00711EE0..0x00711EF7` | none |
| `0x006F47A0` `BuildTimeMultiplier` consumer | verified | `0x006F47EA..0x006F47F4` | none |
| House/category multiplier before BTM | verified for call/order | `0x006F47C1..0x006F47D7`, `0x0050C0A0` | full HouseType default constructor not re-decompiled in this slot |
| `Math__ftol` truncation | verified | `0x007C5F00`; prior memory read `0x00822D80 = 0x0E7F` | none |
| `[General] BuildSpeed` value source | verified for consumer and stock INI | `0x00711EEB`, `rulesmd.ini:41`, constructor default doc | none for stock YR |
| Factory divide-by-54/clamp | verified | `0x004C9EA0`, `0x004C9FB0` assembly | exact elapsed tick edge from timer start is out-of-scope |
| Sidebar UI progress drawing | deferred | explicit non-goal | separate UI trace |
| AI headstart | deferred | explicit non-goal | separate AI production trace |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Where is `BuildTimeMultiplier` read? -> TechnoType `+0x608` in `TechnoTypeClass::ReadINI`, then production consumer reads `[type + 0x608]`.` (evidence: `0x00714371`, `0x006F47EE`)
- `[RESOLVED] OQ2 - Does `TechnoTypeClass::GetBuildTime` include `BuildTimeMultiplier`? -> No; it only uses `Cost * Rules.BuildSpeed * 0.9`.` (evidence: `0x00711EE0..0x00711EF7`)
- `[RESOLVED] OQ3 - What is the exact truncation order? -> base trunc, house multiplier trunc, `BuildTimeMultiplier` trunc, low-power division trunc, each multi-factory iteration trunc, wall trunc if applicable.` (evidence: `0x006F47D7`, `0x006F47F4`, `0x006F4886`, `0x006F4908`, `0x006F4943`)
- `[RESOLVED] OQ4 - Is the conversion round-nearest or truncation? -> `Math__ftol` uses x87 round-toward-zero control word `0x0E7F`, so positive values truncate/floor.` (evidence: `0x007C5F00`; `ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ5 - What is stock `[General] BuildSpeed`? -> `rulesmd.ini` and base `rules.ini` set `.7`; binary consumes field `Rules+0x1748` as double.` (evidence: `rulesmd.ini:41`, `rules.ini:37`, `0x00711EEB`)
- `[RESOLVED] OQ6 - What concrete Grizzly value results? -> `661` build-time frames before factory rate division; factory rate is `12` ticks per progress step after `/54`.` (evidence: arithmetic from verified formula + INI; `0x004C9EA0`, `0x004C9FB0`)
- `[RESOLVED] OQ7 - Does current Rust direct base disagree for stock Grizzly? -> No; `build_time_base_frames` computes the same `441 -> 661` order.` (evidence: `src/sim/production/production_tech.rs:302-321`)
- `[DEFERRED] OQ8 - What is the exact visible elapsed tick count from rate `12`, including timer initial state?` (category: out-of-scope; reason: target is `BuildTimeMultiplier` consumer, not timer edge cadence; next-step-if-pursued: trace `FactoryClass::AI/Step` timer start and progress increment order)
- `[DEFERRED] OQ9 - Are all HouseType build-time defaults constructor-verified in this slot?` (category: bounded-cost-too-high; reason: stock `MTNK` has no active INI override and prior docs cover default/inert status; next-step-if-pursued: decompile `HouseTypeClass` constructor and parser for the `BuildTime*Mult` family)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Grizzly normal base time is `trunc(trunc(700*.7*.9) * 1.5) = 661`, not `(Cost / BuildSpeed) * BTM`. | `0x00711EE0`, `0x006F47EA..0x006F47F4`, `rulesmd.ini:41/6621/6648` | none observed for direct base | `src/sim/production/production_tech.rs::build_time_base_frames` | Preserve base-then-BTM truncation order | Load stock `rulesmd.ini`, assert `MTNK` base frames `661` | Do not divide by `BuildSpeed`; it is a multiplier on cost. |
| `BuildTimeMultiplier` is applied before low-power and multiple-factory modifiers. | `0x006F47F4` precedes power block `0x006F47F9..0x006F4886` and multi block `0x006F48CF..0x006F4914` | likely mostly represented, but end-to-end cadence unverified | `production_tech.rs::effective_time_to_build_frames_for_object`, queue progress surfaces | Keep BTM in base, then apply power/multi to effective time | Two Grizzly factories reduce time after `661`, not before BTM | Do not fold multi-factory into `BuildTimeMultiplier` or cost. |
| Factory rate consumes total time by integer `/54` and clamps `[1,255]`. | `0x004C9EA0`, `0x004C9FB0` call `0x006F47A0`, divide by `54`, clamp | unchecked against current queue progression model | `src/sim/production/production_queue.rs`, progress tests | If modeling RA2 step cadence, derive step delay from verified total and clamp | `MTNK` stock total `661` gives rate `12` ticks/step | Do not treat 661 as necessarily 661 visible elapsed ticks once emulating factory step bars. |

### Stale Docs / Follow-up Docs

Replace the stale MTNK open item wording:

> `BuildTimeMultiplier` consumer in production-time logic is verified. `TechnoTypeClass::GetBuildTime` computes `trunc(Cost * [General] BuildSpeed * 0.9)`, then the live consumer at `0x006F47A0` applies the house/category build-time multiplier and truncates, then applies `TechnoType+0x608 BuildTimeMultiplier` and truncates. Stock Grizzly/`MTNK` with `Cost=700`, `[General] BuildSpeed=.7`, and `BuildTimeMultiplier=1.5` produces `661` build-time frames before low-power/multiple-factory modifiers and a factory step delay of `12` ticks (`661 / 54`). This is not `(Cost / BuildSpeed) * BuildTimeMultiplier`.

## Negative Facts / Do Not Do

- Do not compute Grizzly time as `Cost / BuildSpeed`; binary multiplies by `[General] BuildSpeed`.
- Do not apply `BuildTimeMultiplier` inside `TechnoTypeClass::GetBuildTime`; the verified base helper does not read `+0x608`.
- Do not round `441 * 1.5` to `662`; `Math__ftol` truncates positive values, so `661.5 -> 661`.
- Do not apply `BuildTimeMultiplier` after low-power or multiple-factory scaling; it is before both.
- Do not add any Grizzly/`MTNK` hardcoded branch; this path is data-driven through `TechnoType`.

## Remaining Uncertainty

None for the requested stock Grizzly `BuildTimeMultiplier` formula and numeric value. Out-of-scope follow-up remains for exact visible elapsed tick count from `FactoryClass::AI/Step` timer initialization if the Rust queue later switches from continuous frame countdown to RA2's 54-step rate model.

## Sources

- Ghidra decompile/disassembly: `0x006F47A0`, `0x00711EE0`, `0x004C9EA0`, `0x004C9FB0`, `0x0050C0A0`, `0x004FCE30`, `0x00500910`, `0x007C5F00`.
- INI files: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/sim/production/production_tech.rs`, `src/sim/production/production_queue.rs`, `src/rules/ruleset.rs`.
- Prior docs cross-checked: `docs/research/units/allied/MTNK.md`, `docs/research/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, `docs/research/POWER_SYSTEM_GHIDRA_REPORT.md`, `docs/research/FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md`, `docs/research/ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md`.
