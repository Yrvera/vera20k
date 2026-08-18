# Skirmish RandMap.Sed Writer Full Layout - Ghidra Research Report

**Address(es):** `0x00597730`, concrete writer body `0x00597760`, reader wrapper `0x00597A10`, concrete reader body `0x00597A30`, constructor `0x00595680`, normalizer `0x005975E0`, accept caller `0x005E8590`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** full implementation-grade persisted layout for native `RandMap.Sed`: writer entry, direct caller/liveness, input object fields, section/key order, integer/string encoding, defaults, normalizer bounds, reader/default behavior, and current Rust deltas.  
**Non-Scope:** random-map terrain/noise formulas inside `0x00598960`, random-map setup dialog visual layout, exact dialog control geometry, `RandMap.img` pixel format, and malformed external `.SED` runtime UX beyond static reader/no-clamp evidence.  
**Confidence:** High for writer liveness, field layout, key order, value encodings, constructor defaults, normalizer ranges, reader defaults, and current Rust gaps. Medium for malformed external `.SED` downstream behavior because static analysis proves the reader boundary but not every player-visible failure mode.  
**Active in YR:** Conditional. Active in standard YR offline Skirmish when Choose Map command `0x583` opens random-map setup and that setup returns `1`; the reader side is active when selected scenario filename suffix is `.SED`.

## Working Notes Gate

- Target question: What exact `RandMap.Sed` layout does `0x00597730` write, where do values come from, and what must Rust reproduce?
- Non-goals: Do not investigate the random terrain generator, UI art/paint, `RandMap.img` preview pixels, or the whole random-map dialog except input values that feed the writer.
- Evidence needed to mark COMPLETE: direct caller/liveness, writer vtable target, section/key names and order, field offsets, value encoding, defaults, clamping/normalization boundary, reader semantics, Rust touchpoints, and TS-legacy status.
- Stop conditions: stop when a future Rust implementation can write/read a native-compatible seed/options file and knows which surrounding flow must call it; defer only terrain generation and runtime malformed-file UX.

Prior state row: **Partial / high-confidence report exists; proceed to gaps + verification only.** The earlier `SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md` already covered most body-level layout details. This report is the consolidated full-layout handoff requested by the swarm slot, with fresh Ghidra verification of liveness/callers/input sources and stale-doc wording.

## 1. Overview

`RandMap.Sed` is not a normal map file. It is a seed/options INI emitted from the global random-map seed object at `DAT_00ABDFD8` after accepted random-map setup. The writer emits one `[RandomMap]` section, writes `Description` first, then sixteen integer options in native order.

The matching launch reader loads the same key set from `.SED` files and then calls random-map generation. The reader uses the object's current field values as per-key defaults and the static launch path does not call the normalizer before generation.

Every material finding below is **Active in YR: Conditional** unless otherwise stated: live after the user clicks Create Random Map and accepts setup, or on `.SED` launch.

## 2. Class Layout / Persisted Fields

| Offset | Key | Type / encoding | Fresh constructor default | Writer evidence | Reader evidence | Active in YR |
|---|---|---:|---:|---|---|---|
| `+0x38` | `Theater` | signed decimal int | `0` | `0x0059789D..0x005978B0` | `0x00597B90..0x00597BAA` | Conditional |
| `+0x3C` | `MapType` | signed decimal int | `1` | `0x00597885..0x00597898` | `0x00597B76..0x00597BA2` | Conditional |
| `+0x40` | `Resources` | signed decimal int | `1` | `0x0059798D..0x005979A0` | `0x00597C97..0x00597CB2` | Conditional |
| `+0x44` | `Ruggedness` | signed decimal int | `0` | `0x005978E5..0x005978F8` | `0x00597BDE..0x00597BF8` | Conditional |
| `+0x48` | `Time` | signed decimal int | `1` | `0x005978B5..0x005978C8` | `0x00597BAD..0x00597BC7` | Conditional |
| `+0x4C` | `WaterAmount` | signed decimal int | `0` | `0x00597915..0x00597928` | `0x00597C12..0x00597C3E` | Conditional |
| `+0x50` | `NumPlayers` | signed decimal int | `2` | `0x00597855..0x00597868` | `0x00597B42..0x00597B5C` | Conditional |
| `+0x54` | `Tiberium` | signed decimal int | `0` before normalization | `0x0059792D..0x00597940` | `0x00597C2C..0x00597C46` | Conditional |
| `+0x58` | `TiberiumLayout` | signed decimal int | `0` | `0x00597945..0x00597958` | `0x00597C49..0x00597C63` | Conditional |
| `+0x5C` | `Vegetation` | signed decimal int | `0` | `0x0059795D..0x00597970` | `0x00597C60..0x00597C8C` | Conditional |
| `+0x60` | `UrbanPresence` | signed decimal int | `0` | `0x00597975..0x00597988` | `0x00597C7A..0x00597C94` | Conditional |
| `+0x64` | `Width` | signed decimal int, size bucket | `0` | `0x00597825..0x00597838` | `0x00597B11..0x00597B2B` | Conditional |
| `+0x68` | `Height` | signed decimal int, size bucket | `0` | `0x0059783D..0x00597850` | `0x00597B28..0x00597B54` | Conditional |
| `+0x6C` | `Accessibility` | signed decimal int | `0` | `0x005978FD..0x00597910` | `0x00597BFB..0x00597C15` | Conditional |
| `+0x70` | `RegionSize` | signed decimal int | `0` | `0x005978CD..0x005978E0` | `0x00597BC4..0x00597BF0` | Conditional |
| `+0x74` | `Seed` | signed decimal int | `-1` | `0x0059786D..0x00597880` | `0x00597B5F..0x00597B79` | Conditional |
| `+0x78` | `Description` | comma-separated hex UTF-16 code units | localized `TXT_RANDOM_MAP_DESCRIPTION`, else empty | `0x00597804..0x00597820`, helper `0x00528E00` | `0x00597ADE..0x00597B0C`, helper `0x00528F00` | Conditional |

Fresh constructor defaults are from constructor body `0x00595680` / prior body evidence `0x00595693..0x005956C4`; fresh Ghidra could not decompile the uncreated concrete body by address but disassembly range `0x00595680..0x00595710` is readable and matches the prior report. Active in YR: Conditional, because accepted setup and `.SED` launch both use this class/object.

## 3. Core Logic

### 3.1 Liveness and Entry Points

Active in YR: Conditional, not TS-only. Fresh Ghidra decompile of `FUN_005E8590` verifies the standard Choose Map random-map accept path:

1. Calls `FUN_00595BC0()`.
2. If result is not `1`, returns `-1`.
3. Sets `DAT_008316D4 = 1`.
4. Calls `FUN_00597730(s_RandMap_Sed_0082BC30)`.
5. Rebuilds preview wrapper from `RandMap.img`.
6. Updates or appends one `RandMap.Sed` scenario record.

Evidence: fresh `FUN_005E8590` decompile; direct xref to `FUN_00597730` from `0x005E85E2`; prior command branch evidence `0x005E69D3..0x005E6A1F`. This proves writer behavior is live in standard YR, gated by user path and setup result.

Fresh Ghidra xrefs show `FUN_00597730` has exactly one direct call in this target path: `0x005E85E2` in `FUN_005E8590`. Active in YR: Conditional.

### 3.2 Writer Wrapper and Vtable Target

Active in YR: Conditional. Fresh decompile of `FUN_00597730`:

- If filename argument is non-null, it dispatches `(**(code **)(*this + 8))(filename, this + 0x78)`.
- If filename argument is null, it calls `FUN_00558810(this + 0x78)` and returns whether the result is nonzero.

Evidence: fresh `FUN_00597730` decompile. Prior vtable evidence in `SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md` resolves vtable `+0x8` to body `0x00597760` via `MapSeedClass` vtable bytes at `0x007ED8E4`. The non-null filename path is the live accepted setup path because `FUN_005E8590` passes `RandMap.Sed`.

### 3.3 Writer Body and Key Order

Active in YR: Conditional. Concrete writer body `0x00597760` returns false on null filename, logs `"Saving random map: %s - %ls\n"`, initializes a file/INI object, optionally copies the supplied description pointer to `this+0x78`, and writes one `[RandomMap]` section.

The emitted order is:

1. `Description`
2. `Width`
3. `Height`
4. `NumPlayers`
5. `Seed`
6. `MapType`
7. `Theater`
8. `Time`
9. `RegionSize`
10. `Ruggedness`
11. `Accessibility`
12. `WaterAmount`
13. `Tiberium`
14. `TiberiumLayout`
15. `Vegetation`
16. `UrbanPresence`
17. `Resources`

Evidence: prior body-level disassembly/decompile-backed report `0x00597760..0x005979F4`, with writer call sequence `0x00597811..0x005979A0`; fresh Ghidra disassembly range `0x00597730..0x005979F8` is readable and no contradictory target was found. String/key addresses from prior report: section `RandomMap` at `0x0082BB24`; keys at `0x0081B1A4`, `0x0082BBE4`, `0x0081A7A8`, `0x0082BBD8`, `0x0082BBD0`, `0x0082BBC8`, `0x00818658`, `0x0081F11C`, `0x0082BBBC`, `0x0082BBB0`, `0x0082BBA0`, `0x0082BB94`, `0x00817278`, `0x0082BB84`, `0x0082BB78`, `0x0082BB68`, `0x0082BB5C`.

### 3.4 Integer Encoding

Active in YR: Conditional. All sixteen integer fields are serialized through the integer INI writer with format selector `0`. Fresh decompile of `FUN_005275C0` shows selector `0` falls to `DAT_00817F6C`, while selector `1` and `2` choose the hex formats at `DAT_00825BAC` and `DAT_00825BB0`. Therefore this writer emits signed decimal text via native `%d`.

Evidence: fresh `FUN_005275C0` decompile; prior writer body shows each integer write pushes selector `0`. Active in YR: Conditional.

### 3.5 Description Encoding

Active in YR: Conditional. `Description` is not plain text. Fresh decompile of `FUN_00528E00` shows it walks a `short *` UTF-16 buffer until zero, converts each code unit to base-16 text with radix `0x10`, appends delimiter `","` from `DAT_00817F70`, then writes the resulting ASCII string to the INI entry.

Fresh decompile of `FUN_00528F00` shows the reader locates the INI entry, copies/trims the ASCII value, tokenizes it by `","`, parses each token using `"%x"` at `DAT_00825BD4`, writes each parsed value as a 16-bit code unit to the output buffer, and appends a UTF-16 zero terminator.

Example native-compatible value for `Random Map` is `52,61,6e,64,6f,6d,20,4d,61,70,`. Active in YR: Conditional.

### 3.6 Input Value Sources Before Write

Active in YR: Conditional. Fresh decompile of random-map setup callback `FUN_00596300` shows:

- Command `0x621` randomizes object fields: `Theater` from `RandomRanged(0,100) > 0x31`, `MapType` from `RandomRanged(1,4)`, `Time/Resources/Width/Height` from `RandomRanged(0,3)`, `Description` from string table id `0xF5E`, and `Seed` from `RandomRanged(0,0xFFFF)`, then calls `FUN_005975E0`.
- Command/control update path `FUN_00596C70` reads combo/listbox controls into fields `+0x3C`, `+0x38`, `+0x40`, `+0x48`, `+0x64/+0x68`, and `+0x50`, then calls `FUN_005975E0`.
- `FUN_00596E50` also calls `FUN_005975E0` before repopulating/applying dialog controls.
- Generate command `0x620` calls `FUN_00596C70`, runs `FUN_00598960(1, hwnd)`, generates preview, and snapshots the current `DAT_00ABDFD8` seed object into `DAT_00ABE150`.

Evidence: fresh `FUN_00596300`, `FUN_00597380`, `FUN_00596C70`, and `FUN_00596E50` decompiles. This report does not claim full UI-control layout, only that accepted writer input is a normalizer-constrained `MapSeedClass` object mutated by the random-map setup dialog.

### 3.7 Normalizer Bounds

Active in YR: Conditional. Fresh decompile of `FUN_005975E0` verifies these clamps:

| Offset / key | Clamp |
|---|---|
| `+0x40 Resources` | `0..3` |
| `+0x3C MapType` | `0..4` |
| `+0x48 Time` | `0..3` |
| `+0x44 Ruggedness` | `0..100` |
| `+0x4C WaterAmount` | `0..100` |
| `+0x50 NumPlayers` | `2..8` |
| `+0x54 Tiberium` | `1..100` |
| `+0x58 TiberiumLayout` | `0..100` |
| `+0x5C Vegetation` | `0..100` |
| `+0x60 UrbanPresence` | `0..100` |
| `+0x64 Width` | `0..3` |
| `+0x68 Height` | `0..3` |
| `+0x6C Accessibility` | `0..100` |
| `+0x70 RegionSize` | `0..100` |
| `+0x74 Seed` | `0..65535` |

No clamp for `+0x38 Theater` was found in `FUN_005975E0`; standard setup paths constrain it through UI/randomization. Active in YR: Conditional.

### 3.8 Reader Wrapper, Defaults, and Launch Boundary

Active in YR: Conditional. Fresh decompile of `FUN_00597A10`:

- If filename argument is non-null, it dispatches `(**(code **)(*this + 4))(filename)`.
- If filename argument is null, it calls `FUN_005587F0()` and returns whether the result is nonzero.

Prior vtable evidence resolves vtable `+0x4` to concrete reader `0x00597A30`. The reader returns false for null/failed load, reads the same `[RandomMap]` keys, and for each integer key passes the current field value as the default into the integer reader before storing the result back. Missing keys therefore preserve object state; for a fresh object, Section 2 constructor defaults apply.

Fresh xrefs show `FUN_00597A10` has a direct call at `0x00684975` in `ScenarioClass__Read_Scenario`. Prior launch evidence `0x0068496B..0x00684989` shows `.SED` launch calls reader on `DAT_00ABDFD8`; if the reader succeeds, launch calls `FUN_00598960(0,0)`. No reader-side call to `FUN_005975E0` was found in the reader body or launch boundary. Active in YR: Conditional.

### 3.9 Generator Seed Consumption Sanity Check

Active in YR: Conditional. Fresh decompile of `FUN_00598960` confirms the generator immediately uses `param_1 + 0x74` as the seed passed to `FUN_0065C6D0` and copies the RNG state into `DAT_00ABE890`. This makes the exact `Seed` field encoding/default/clamping load-bearing. It also confirms `param_2` distinguishes preview mode: preview calls pass nonzero and repaint/generate previews; launch passes `0,0`.

Evidence: fresh `FUN_00598960` decompile; launch caller xref at `0x00684975`. Active in YR: Conditional.

## 4. INI Keys

These are not `rules*.ini` or `art*.ini` keys. They are keys in generated/read `RandMap.Sed`.

| Section | Key | Type | Fresh default if missing | Writer emits | Reader consumes | Active in YR |
|---|---|---:|---:|---|---|---|
| `RandomMap` | `Description` | comma-hex UTF-16 CSV | localized string-table default or empty fallback | Yes | Yes | Conditional |
| `RandomMap` | `Width` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Height` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `NumPlayers` | signed decimal int | `2` | Yes | Yes | Conditional |
| `RandomMap` | `Seed` | signed decimal int | `-1` before normalizer/dialog randomization | Yes | Yes | Conditional |
| `RandomMap` | `MapType` | signed decimal int | `1` | Yes | Yes | Conditional |
| `RandomMap` | `Theater` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Time` | signed decimal int | `1` | Yes | Yes | Conditional |
| `RandomMap` | `RegionSize` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Ruggedness` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Accessibility` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `WaterAmount` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Tiberium` | signed decimal int | `0` before normalizer; native normalizer min is `1` | Yes | Yes | Conditional |
| `RandomMap` | `TiberiumLayout` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Vegetation` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `UrbanPresence` | signed decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Resources` | signed decimal int | `1` | Yes | Yes | Conditional |

Repo `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, and `ini/artmd.ini` do not define this persisted layout. Theater files may contain `RequiredForRMG=true` terrain-template availability flags, but those are generator inputs, not `.SED` writer fields. Active in YR: Conditional for `.SED` layout; generator terrain template consumption is out of scope.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map command `0x583` | Enters random-map setup and calls `0x005E8590`; `-1` result skips accept side effects | prior command assembly `0x005E69D3..0x005E6A1F`; fresh `FUN_005E8590` decompile | Conditional |
| Accepted setup gate | Only result `1` from `FUN_00595BC0` causes writer call | fresh `FUN_005E8590` and `FUN_00595BC0` decompiles | Conditional |
| Writer call | `DAT_00ABDFD8` seed object is saved to `RandMap.Sed` via `FUN_00597730` | fresh `FUN_005E8590`; xref `0x005E85E2` | Conditional |
| Dialog input normalization | Randomize/control-read/display paths call `FUN_005975E0` before accepted state can be written | fresh `FUN_00596300`, `FUN_00597380`, `FUN_00596C70`, `FUN_00596E50` | Conditional |
| Reader launch branch | `.SED` launch calls `FUN_00597A10`; success calls `FUN_00598960(0,0)` | xref `0x00684975`; prior launch range `0x0068496B..0x00684989` | Conditional |
| Generator consumer | `FUN_00598960` consumes seed at `+0x74` immediately | fresh `FUN_00598960` decompile | Conditional |

## 6. Current Rust Implementation Status

Rust currently has sentinel/list support but no native `.SED` seed/options model.

| Surface | Current status | Evidence |
|---|---|---|
| Button command | Recognized but log-only | `src/app.rs:941..944` |
| Synthetic sentinel record | Present | `src/skirmish_scenarios.rs:14`, `src/skirmish_scenarios.rs:82` |
| Sentinel min/max/official | Present and native-shaped: min `2`, max `4`, official `true` | `src/skirmish_scenarios.rs:14..16`, `src/skirmish_scenarios.rs:82..104` |
| Upsert single sentinel | Present | `src/skirmish_scenarios.rs:212..241` |
| Mode random flag filter | Present | `src/skirmish_scenarios.rs:195..207` |
| `.SED` seed/options parser/writer | Missing | `rg RandMap/SED src`; only sentinel/preview strings found |
| Accepted setup state model | Missing | `src/app.rs:941..944` log-only branch |
| Launch `.sed` generation branch | Missing | `src/app_init.rs:324..327` routes requested map through normal loader; `src/app_list_maps.rs:396..404` searches only `.mmx/.yro/.map/.mpr/.yrm` variants |
| Preview branch | Partial: sentinel preview constants exist, but no accepted setup lifecycle writes/refreshes `RandMap.img` | `src/app_skirmish_shell_render.rs:73..74` and preview tests |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / completion criteria | verified | Working Notes Gate | none |
| `FUN_00597730` wrapper | verified | fresh decompile | none |
| Direct writer caller | verified | xref `0x005E85E2`; fresh `FUN_005E8590` | none |
| Writer body `0x00597760` key order | verified | prior body report plus fresh readable disassembly range `0x00597730..0x005979F8` | none for layout |
| Integer formatter | verified | fresh `FUN_005275C0`; prior writer selector `0` evidence | none |
| UTF-16 hex CSV writer/reader | verified | fresh `FUN_00528E00`, `FUN_00528F00` | none |
| Constructor defaults | verified | prior constructor body `0x00595693..0x005956C4`; fresh readable range `0x00595680..0x00595710` | none |
| Dialog input sources | verified for writer-relevant fields | fresh `FUN_00596300`, `FUN_00597380`, `FUN_00596C70`, `FUN_00596E50` | exact visual/control layout out of scope |
| Normalizer `FUN_005975E0` | verified | fresh decompile | none |
| Reader wrapper and direct launch xref | verified | fresh `FUN_00597A10`; xref `0x00684975` | none |
| Reader body defaults/no-clamp boundary | verified for static layout | prior body report plus launch xref; no normalizer xref on reader path | malformed external file UX deferred |
| Generator seed consumption | verified | fresh `FUN_00598960` | terrain formulas out of scope |
| TS legacy / liveness filter | verified | standard Choose Map caller and `.SED` launch branch | none |
| Current Rust status | verified | Codegraph `RandMap` search; `rg`/file reads | implementation not performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x00597730` live in standard YR? -> Yes, conditionally after Create Random Map setup returns `1`.` (evidence: fresh `FUN_005E8590`; xref `0x005E85E2`)
- `[RESOLVED] OQ-02 - Is this TS-only legacy? -> No; it is reached from standard YR offline Skirmish Choose Map and `.SED` launch.` (evidence: prior command branch `0x005E69D3..0x005E6A1F`; fresh `FUN_005E8590`; xref `0x00684975`)
- `[RESOLVED] OQ-03 - What concrete method does writer wrapper call? -> vtable `+0x8`, concrete writer body `0x00597760`.` (evidence: fresh `FUN_00597730`; prior vtable evidence `0x007ED8E4+8`)
- `[RESOLVED] OQ-04 - What section/key layout is emitted? -> One `[RandomMap]` section with `Description` then sixteen integer keys listed in Section 3.3.` (evidence: prior writer body `0x00597811..0x005979A0`; fresh readable disassembly range)
- `[RESOLVED] OQ-05 - Are integers decimal or hex? -> Signed decimal via selector `0` / `%d`.` (evidence: fresh `FUN_005275C0`; prior writer selector `0`)
- `[RESOLVED] OQ-06 - Is `Description` plain text? -> No, comma-separated hex UTF-16 code units.` (evidence: fresh `FUN_00528E00`, `FUN_00528F00`)
- `[RESOLVED] OQ-07 - Where do writer input values come from? -> The setup dialog mutates `DAT_00ABDFD8`; randomize/control-read paths call `FUN_005975E0` before saveable state.` (evidence: fresh `FUN_00596300`, `FUN_00597380`, `FUN_00596C70`, `FUN_00596E50`)
- `[RESOLVED] OQ-08 - What are normalizer ranges? -> Listed in Section 3.7; notably `Seed=0..65535`, `NumPlayers=2..8`, size buckets `0..3`.` (evidence: fresh `FUN_005975E0`)
- `[RESOLVED] OQ-09 - Does normalizer clamp `Theater`? -> No clamp observed in `FUN_005975E0`; setup UI/randomization constrains it.` (evidence: fresh `FUN_005975E0`)
- `[RESOLVED] OQ-10 - Which reader wrapper is used at launch? -> `FUN_00597A10`, vtable `+0x4` concrete reader `0x00597A30`.` (evidence: fresh `FUN_00597A10`; xref `0x00684975`; prior vtable evidence)
- `[RESOLVED] OQ-11 - Are missing keys defaulted from constants or current fields? -> Current object fields; fresh object defaults come from constructor.` (evidence: prior reader body `0x00597B11..0x00597CB2`; prior constructor body)
- `[RESOLVED] OQ-12 - Does `.SED` launch reader clamp before generation? -> No static reader-side normalizer call was found; launch proceeds from reader success to `FUN_00598960(0,0)`.` (evidence: prior reader body and launch range `0x0068496B..0x00684989`; fresh normalizer xrefs do not include reader)
- `[RESOLVED] OQ-13 - Does the generator consume the persisted seed? -> Yes, `FUN_00598960` immediately reads `+0x74`.` (evidence: fresh `FUN_00598960`)
- `[RESOLVED] OQ-14 - Does current Rust implement the layout? -> No; only sentinel/list/preview constants exist.` (evidence: Codegraph `RandMap` search; `src/app.rs`, `src/skirmish_scenarios.rs`, `src/app_init.rs`, `src/app_list_maps.rs`)
- `[DEFERRED] OQ-15 - What is exact player-visible behavior for malformed external `.SED` values?` (category: needs-runtime-debugger; reason: static evidence proves defaults/no reader clamp, but not every downstream bad-value UX; next-step-if-pursued: run native with crafted `.SED` variants)
- `[DEFERRED] OQ-16 - Full random terrain meaning of each option field.` (category: out-of-scope; reason: this slot is writer layout only; next-step-if-pursued: generator formula investigation over `0x00598960` callees)
- `[DEFERRED] OQ-17 - Exact setup dialog visual/control geometry.` (category: out-of-scope; reason: only writer input fields were needed; next-step-if-pursued: separate random-map setup dialog report)

Deferred pile is 3 of 17 and all are outside the writer-layout slice. This report is COMPLETE for persisted layout and writer/reader handoff.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native `RandMap.Sed` is a `[RandomMap]` seed/options file with exact key order: `Description`, `Width`, `Height`, `NumPlayers`, `Seed`, `MapType`, `Theater`, `Time`, `RegionSize`, `Ruggedness`, `Accessibility`, `WaterAmount`, `Tiberium`, `TiberiumLayout`, `Vegetation`, `UrbanPresence`, `Resources`. | prior writer body `0x00597811..0x005979A0`; fresh caller/wrapper verification | missing | new random-map seed/options model plus `src/app.rs` accepted setup path and launch/map-load surface | Write/read the exact native key set and order; store fields as seed/options, not as concrete map data. | A native-style `RandMap.Sed` round-trips with same key order and values. Proposed test: `skirmish_randmap_sed_round_trips_native_key_order` | Do not treat `RandMap.Sed` as a loose map or omit fields because the chooser sentinel exists. |
| All integer fields are signed decimal `%d`; `Description` is hex CSV of UTF-16 code units. | fresh `FUN_005275C0`, `FUN_00528E00`, `FUN_00528F00`; prior writer selector `0` | missing | seed/options parser/writer; shell display-name bridge | Encode integer values as decimal text and description as comma-delimited hex UTF-16 code units. | `Description=52,61,6e,64,6f,6d,20,4d,61,70,` decodes to `Random Map`; writing the same string reproduces hex CSV. Proposed test: `skirmish_randmap_sed_description_uses_hex_utf16_csv` | Do not write `Description=Random Map`; native helper does not use plain text for this field. |
| Accepted setup normalizes object values before saveable state reaches writer; writer serializes fields as-is. | fresh `FUN_00596300`, `FUN_00596C70`, `FUN_00596E50`, `FUN_005975E0`; fresh `FUN_005E8590` | missing | future random-map setup state and accepted `0x583` path | Clamp setup-originated values with native ranges before persisting accepted random-map state. | Out-of-range setup values serialize as native clamps: `NumPlayers=2..8`, `Seed=0..65535`, `Width/Height=0..3`, `Tiberium=1..100`. Proposed test: `skirmish_randmap_create_dialog_clamps_before_sed_write` | Do not postpone all normalization until generator launch and call that parity; native setup normalizes before writer use. |
| Reader defaults are current object fields/fresh constructor defaults, and static `.SED` launch does not call the normalizer before `FUN_00598960(0,0)`. | prior reader body `0x00597A30`; fresh `FUN_00597A10`; xref `0x00684975`; prior launch range `0x0068496B..0x00684989` | missing | seed/options parser/default policy and launch branch | Use native constructor defaults for missing keys; keep parser/default behavior distinct from setup-origin normalization. | Missing `Seed`, `NumPlayers`, `MapType`, and `Resources` produce defaults `-1`, `2`, `1`, `1` before any explicit Rust validation layer. Proposed test: `skirmish_randmap_sed_missing_keys_preserve_native_defaults` | Do not silently clamp every externally loaded `.SED` and claim native reader parity. |
| `.SED` launch branch consumes seed/options, not `RandMap.img` or normal map INI. | fresh `FUN_00598960`; xref `0x00684975`; prior launch `.SED` branch | missing | `src/app_init.rs`, `src/app_list_maps.rs`, random-map generator/load branch | Preempt normal map lookup for `.sed` tokens, parse seed/options, then route to random-map generation or explicit generator blocker. | Selecting `RandMap.Sed` does not produce the ordinary "map not found" path and does not parse `[PreviewPack]`. Proposed test: `skirmish_launch_sed_branch_preempts_normal_map_lookup` | Do not use `RandMap.img` as gameplay terrain and do not invent a generated `.map` filename. |

## 10. Negative Facts / Do Not Do

- Do not serialize `Description` as plain text. Active in YR: No for this writer. Evidence: fresh `FUN_00528E00` and `FUN_00528F00`.
- Do not serialize integers as hex. Active in YR: No for this writer. Evidence: fresh `FUN_005275C0`; selector `0` is `%d`.
- Do not create or update `RandMap.Sed` on mere `0x583` button click. Active in YR: No; side effects require `FUN_00595BC0` result `1`. Evidence: fresh `FUN_005E8590`.
- Do not treat `Width` and `Height` as literal cell dimensions in the `.SED` layout. Active in YR: No; they are buckets clamped `0..3`. Evidence: fresh `FUN_005975E0`.
- Do not claim reader-side clamping for external `.SED` load. Active in YR: No static evidence; launch proceeds reader success -> generator. Evidence: prior reader/launch body, fresh normalizer xrefs exclude reader.
- Do not parse `RandMap.Sed` as ordinary map INI or `[PreviewPack]`. Active in YR: No; `.SED` launch reads seed/options and calls generator. Evidence: xref `0x00684975`; fresh `FUN_00598960`.
- Do not use `RandMap.img` as gameplay terrain. Active in YR: No; it is preview-side output, while launch calls `FUN_00598960(0,0)`. Evidence: fresh `FUN_005E8590`, `FUN_00598960`.

## 11. Remaining Uncertainty

- Malformed external `.SED` runtime UX remains unverified by native runtime. Static evidence is enough for parser/default/no-reader-clamp implementation, but not for every downstream bad-value error or crash behavior.
- Exact random terrain interpretation of each persisted option is outside this writer report and belongs to a generator formula report.
- Exact random-map setup dialog visual/control layout is outside this writer report. This report only verifies the value sources needed to feed the writer.

## 12. Stale Docs / Follow-up Docs

Replacement wording for any stale or vague Create Random Map docs:

> `RandMap.Sed` is a native `[RandomMap]` seed/options file written only after accepted random-map setup. Writer `0x00597730` dispatches through `MapSeedClass` vtable `+0x8` to body `0x00597760`, which emits `Description`, `Width`, `Height`, `NumPlayers`, `Seed`, `MapType`, `Theater`, `Time`, `RegionSize`, `Ruggedness`, `Accessibility`, `WaterAmount`, `Tiberium`, `TiberiumLayout`, `Vegetation`, `UrbanPresence`, and `Resources` in that order. Integer fields are signed decimal `%d`; `Description` is comma-separated hex UTF-16 code units. Reader `0x00597A30` uses current object fields as per-key defaults, and the `.SED` launch path was not observed to call the normalizer before `FUN_00598960(0,0)`.

Replacement wording for any stale current-Rust docs:

> Current Rust has a `RandMap.Sed` sentinel record with native min/max/official metadata and mode filtering, but lacks the native `.SED` seed/options parser/writer, accepted setup state, writer call, and launch-time `.sed` branch. The `0x583` app branch is still log-only.

## Sources

- Fresh read-only Ghidra: `FUN_00597730`, `FUN_00597A10`, `FUN_005975E0`, `FUN_005E8590`, `FUN_00595BC0`, `FUN_00596300`, `FUN_00597380`, `FUN_00596C70`, `FUN_00596E50`, `FUN_00598960`, `FUN_00528E00`, `FUN_00528F00`, `FUN_005275C0`.
- Fresh Ghidra xrefs: `FUN_00597730` called from `0x005E85E2`; `FUN_00597A10` called from `0x00684975`; `FUN_005975E0` called from `0x00597425`, `0x00596841`, `0x00596E3A`, `0x00596E59`; UTF-16 helper xrefs include writer/reader body call sites `0x00597820` and `0x00597B0C`.
- Prior verified body report used for concrete uncreated body ranges: `docs/research/skirmish-ui/SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md`.
- Related prior docs: `docs/research/skirmish-ui/SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`.
- Rust scan: Codegraph `RandMap` search; `src/app.rs`, `src/skirmish_scenarios.rs`, `src/app_init.rs`, `src/app_list_maps.rs`, `src/app_skirmish_shell_render.rs`.
