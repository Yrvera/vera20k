# Scenario Lighting Default Reset Path - Ghidra Report

**Target question:** For map lighting fields owned by `ScenarioClass`, where are defaults reset, what are the ordinary `[Lighting]` defaults, how do missing keys behave in `ScenarioClass::Read_INI_Basic`, and what adjacent Ion/Nuke/Dominator defaults are established by the same reset path?

**Status:** COMPLETE for the scoped constructor/reset/default path.

## Non-goals

- Do not re-investigate the cell lighting formula, LightSource falloff, LightConvert cache, dirty scheduling, or renderer consumers.
- Do not resolve the complete Lightning Storm, Nuke flash, Ion, or Psychic Dominator transition timelines.
- Do not inspect INI template files or FinalAlert defaults as behavioral sources.
- Do not mutate Ghidra labels, Rust code, INI data, existing docs, or `.swarm-claims.md`.

## Evidence Needed To Mark COMPLETE

- Verify the `ScenarioClass` constructor/reset writer that initializes the lighting fields.
- Verify that normal scenario load calls that reset writer before `[Lighting]` parsing.
- Verify the exact ordinary lighting default integers for Ambient/Red/Green/Blue/Ground/Level.
- Verify directly adjacent Ion/Nuke/Dominator default integers where written by the same reset helper.
- Verify missing-key behavior in `CCINIClass::ReadDouble` and the default arguments passed by `ScenarioClass::Read_INI_Basic`.

## Stop Conditions

- Stop after the reset/default writer and missing-key behavior are proven.
- Stop if exact dynamic superweapon transition semantics require following consumers outside the adjacent reset/parser slice.
- Stop without writing anything except this report.

## Verified Findings

1. `ScenarioClass__Constructor @ 0x006832C0` calls `FUN_00683610`, which is the reset/default writer for the relevant lighting fields. Active in YR: Yes.

2. `FUN_006851F0` is the scenario clear/reset path used during scenario initialization; it writes a few pre-clear fields, calls `FUN_00683610`, and then clears map/object/system state. Active in YR: Yes for standard fresh scenario/map loads.

3. `ScenarioClass__Full_Init @ 0x00686B20` calls `FUN_006851F0` near the start when its internal skip-clear flag is not set, before the later call to `ScenarioClass__Read_INI_Basic @ 0x00689E90`. Active in YR: Yes for normal campaign/skirmish map load; Conditional for special/retry paths that pass the skip-clear flag.

4. Ordinary map lighting defaults written by `FUN_00683610` are:

| Field | Default internal integer | Public-unit equivalent | Active in YR |
|---|---:|---:|---|
| `ScenarioClass+0x3528` ordinary/base ambient | `100` | `1.00` | Yes |
| `ScenarioClass+0x352C` current/working ambient | `100` | `1.00` | Yes |
| `ScenarioClass+0x3530` bottom/current ambient slot | `100` | `1.00` | Yes |
| `ScenarioClass+0x3534` red | `100` | `1.00` | Yes |
| `ScenarioClass+0x3538` green | `100` | `1.00` | Yes |
| `ScenarioClass+0x353C` blue | `100` | `1.00` | Yes |
| `ScenarioClass+0x3540` ground | `50` | `0.20` | Yes |
| `ScenarioClass+0x3544` level | `8` | `0.032` | Yes |

5. `ScenarioClass__Read_INI_Basic @ 0x00689E90` reads ordinary `[Lighting]` keys using the current ScenarioClass field as the default argument. Missing `Ambient/Red/Green/Blue` preserve reset values via `field * 0.01 -> ReadDouble -> ftol(value * 100 + 0.01)`. Missing `Ground/Level` preserve reset values via `field * 0.004 -> ReadDouble -> ftol(value * 250 + 0.01)`. Active in YR: Yes.

6. `CCINIClass__ReadDouble @ 0x005283D0` returns its `param_4` default if the section/key is null, the section is absent, the entry is absent, or the entry has no value. Therefore missing map `[Lighting]` keys preserve the freshly reset `ScenarioClass` defaults, not FinalAlert/template defaults. Active in YR: Yes.

7. `Ambient` is special on read: the parsed value is written to `+0x3528`, `+0x352C`, and `+0x3530`. Missing `Ambient` therefore preserves all three at `100`. Active in YR: Yes.

8. Adjacent Ion/Lightning-storm lighting defaults written by `FUN_00683610` are:

| Field | Reset value | Parser key in `0x00689E90` | Active in YR |
|---|---:|---|---|
| `+0x3548` Ion/Lightning ambient | `87` | `IonAmbient` | Conditional: dynamic Ion/Lightning branch |
| `+0x354C` Ion red | `30` | `IonRed` | Conditional: dynamic Ion/Lightning branch |
| `+0x3550` Ion green | `40` | `IonGreen` | Conditional: dynamic Ion/Lightning branch |
| `+0x3554` Ion blue | `75` | `IonBlue` | Conditional: dynamic Ion/Lightning branch |
| `+0x3558` Ion ground | `0` | `IonGround` | Conditional: dynamic Ion/Lightning branch |
| `+0x355C` Ion level | `0` | `IonLevel` | Conditional: dynamic Ion/Lightning branch |

9. Adjacent Nuke/flash-style defaults written by `FUN_00683610` are `+0x3560=200`, `+0x3564=175`, `+0x3568=150`, `+0x356C=125`, `+0x3570=100`, `+0x3574=100`, and `+0x3578=1`. In this scoped parser, only `NukeAmbientChangeRate` at `+0x3578` is read from `[Lighting]`; the other fields are reset here but not map-key parsed by `0x00689E90`. Active in YR: Conditional: dynamic Nuke/Lightning/superweapon lighting branches.

10. Adjacent Dominator defaults written by `FUN_00683610` and read by `0x00689E90` are:

| Field | Reset value | Parser key | Active in YR |
|---|---:|---|---|
| `+0x357C` | `150` | `DominatorAmbient` | Conditional: Psychic Dominator branch |
| `+0x3580` | `85` | `DominatorRed` | Conditional: Psychic Dominator branch |
| `+0x3584` | `20` | `DominatorGreen` | Conditional: Psychic Dominator branch |
| `+0x3588` | `30` | `DominatorBlue` | Conditional: Psychic Dominator branch |
| `+0x358C` | `0` | `DominatorGround` | Conditional: Psychic Dominator branch |
| `+0x3590` | `0` | `DominatorLevel` | Conditional: Psychic Dominator branch |
| `+0x3594` | `1` | `DominatorAmbientChangeRate` | Conditional: Psychic Dominator branch |

## Implementation Handoff

- Initialize ordinary map lighting from the binary reset defaults, not from FinalAlert/template text. Active in YR: Yes.
- Missing ordinary `[Lighting]` keys should preserve `Ambient/R/G/B=1.00`, `Ground=0.20`, and `Level=0.032` in public units. Active in YR: Yes.
- Preserve the integer scaling split: ordinary/RGB values use a 100 scale, while Ground/Level use a 250 scale. Active in YR: Yes.
- Treat Ion, Nuke/flash, and Dominator fields as separate dynamic-lighting profiles. They share the reset helper and `[Lighting]` section, but ordinary map ambience should not use them unless the relevant dynamic branch is active. Active in YR: Conditional.

## Negative Facts / Do Not Do

- Do not use FinalAlert template defaults as the engine default for missing map lighting keys. Active in YR: No.
- Do not default ordinary `Ground` to `0.0`; reset writes internal `50`, equivalent to public `0.20`. Active in YR: No.
- Do not treat ordinary missing-key behavior as "zero if absent"; `ReadDouble` returns the supplied current-field default. Active in YR: No.
- Do not merge ordinary ambience with Ion/Nuke/Dominator lighting. Active in YR: No for ordinary map lighting; Conditional for their dedicated dynamic branches.

## Remaining Uncertainty

- Exact semantic names for `+0x3560..+0x3574` were not fully re-proven here beyond reset values and adjacent dynamic-lighting classification. Active in YR: Conditional.
- Special paths that intentionally call `ScenarioClass__Full_Init` with the skip-clear flag may preserve previous ScenarioClass state before INI reads; this is not the standard fresh map-load path. Active in YR: Conditional.

## Ghidra Evidence

- `ScenarioClass__Constructor @ 0x006832C0`
- `FUN_00683610 @ 0x00683610`
- `FUN_006851F0 @ 0x006851F0`
- `ScenarioClass__Full_Init @ 0x00686B20`
- `ScenarioClass__Read_Scenario @ 0x00684620`
- `ScenarioClass__Read_Scenario_INI @ 0x00686730`
- `ScenarioClass__Read_INI_Basic @ 0x00689E90`
- `CCINIClass__ReadDouble @ 0x005283D0`
