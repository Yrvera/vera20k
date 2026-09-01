# Scenario-Start Crate Post-Map Caller Gate — Ghidra Report

**Date:** 2026-09-01  
**Investigation mode:** exhaustive slice  
**Claimed scope:** the active callers, caller gates, count loop, and relative order of scenario-start random-crate placement around `ScenarioClass__Post_Map_Init @ 0x00686890`.  
**Explicit non-scope:** random-cell placement internals, crate pickup/effects, death drops, regeneration cadence, network-mode implementation, and unrelated post-map house mechanics.  
**Active binary:** installed Yuri's Revenge `gamemd.exe`, PE image base `0x00400000`, Ghidra program `/gamemd.exe`.  
**Confidence:** HIGH for every implementation-facing conclusion below.

## 1. Overview

`ScenarioClass__Post_Map_Init` contains no internal game-mode comparison around its crate loop, but the normal fixed-map caller does. `ScenarioClass__Full_Init @ 0x00686B20` calls the helper only when raw game mode is nonzero and its second control byte is zero. The ordinary fresh fixed-map loader passes that byte as zero. Consequently, campaign mode `0` does **not** receive scenario-start random-crate scatter from this helper, while ordinary offline Skirmish mode `5` does when the lobby `Crates` option is enabled.

Generated random maps have a separate successful-generation path in `ScenarioClass__Read_Scenario @ 0x00684620`; that path calls `ScenarioClass__Post_Map_Init(1)` directly. Synthetic-INI preview initialization does not use the in-game `Full_Init` path and therefore does not seed gameplay crates.

Within `Post_Map_Init`, the initial crate loop completes before AI opening-credit work and before the later selected-mode alliance callback. This is the required scenario-start ordering seam for Rust.

This report corrects the broader wording in `PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md` that treated the helper's missing internal comparison as if the active call path had no game-mode gate.

## 2. Verified fields and globals

| Address / offset | Verified role in this slice | Evidence |
|---|---|---|
| `0x00A8B238` | raw game-mode global read by `Full_Init` | `0x00687BC8..0x00687BCE` |
| `0x00A8B261` | lobby/session `Crates` boolean read by `Post_Map_Init` | `0x0068695C..0x00686963` |
| `0x00A8B54C` | pregame human-node count used as one signed lower-bound candidate | `0x00686965..0x00686972` |
| Rules `+0x1470` | signed `CrateMinimum` | `0x0068696D..0x00686978` |
| Rules `+0x1474` | signed `CrateMaximum` | `0x0068697A..0x00686989` |
| Scenario `+0x34BD` | `IsRandom` branch flag in `Read_Scenario` | branch leading to `0x0068498E` |
| `Full_Init` second control byte | zero admits the fixed-map post-map call; nonzero skips it | `0x00687BD0..0x00687BD6` |

`[MultiplayerDialogSettings] Crates=yes` in the repository retail `ini/rulesmd.ini` supplies the stock launcher default. `[Basic] Official` is read immediately before the fixed-map helper call and passed in `CL`; it is not the crate enable gate and does not change the crate-count loop described here.

## 3. Active call paths

### 3.1 Ordinary fixed-map load

`ScenarioClass__Read_Scenario_INI @ 0x00686730` prepares the fresh-load call with `XOR DL,DL` at `0x0068683A`, loads the scenario object into `ECX`, and calls `ScenarioClass__Full_Init` at `0x00686845`.

Near the end of `Full_Init`:

1. the optional scenario rules (`TMCJ4F`) have already been processed at `0x00687B76`;
2. map-cell attributes and late initialization milestones run through the `90` progress/service point;
3. `CMP [0x00A8B238],ESI` / `JZ 0x00687BF1` at `0x00687BC8..0x00687BCE` skips the helper when raw mode is zero;
4. the second control byte is checked at `0x00687BD0..0x00687BD6`, and a nonzero value also skips the helper;
5. `[Basic] Official` is read at `0x00687BE5`, moved to `CL`, and `ScenarioClass__Post_Map_Init` is called at `0x00687BEC`.

Therefore the active fixed-map rule is:

```text
call Post_Map_Init iff raw_game_mode != 0 and second_control_byte == 0
```

Among Phase 14's admitted ordinary production sessions, this includes offline Skirmish raw mode `5` and excludes campaign raw mode `0`. Raw network modes are outside this slice's production scope even though the native comparison itself admits any nonzero value.

### 3.2 Generated random map

`ScenarioClass__Read_Scenario @ 0x00684620` tests `Scenario+0x34BD`. On the random branch it invokes the random-map generation path; after successful generation, assembly at `0x0068498E..0x00684990` loads `CL=1` and calls `ScenarioClass__Post_Map_Init` directly.

This is a distinct active parent, not an escape around the fixed-map campaign gate: it is reached only by the generated-random-map branch after generation succeeds.

`RandomMapGenerator__InitMapFromSyntheticINI @ 0x00599650` calls `Full_Init` only for its in-game `param_2 == 0` path. Its nonzero preview path follows preview reset/finalization instead and does not seed gameplay crates.

### 3.3 Exhaustive parent check

The live Ghidra `get_function_callers` results were re-read cold after the branch analysis:

- `ScenarioClass__Post_Map_Init @ 0x00686890` has exactly two function parents: `ScenarioClass__Full_Init @ 0x00686B20` and `ScenarioClass__Read_Scenario @ 0x00684620`.
- `ScenarioClass__Full_Init @ 0x00686B20` has exactly two function parents: `ScenarioClass__Read_Scenario_INI @ 0x00686730` and `RandomMapGenerator__InitMapFromSyntheticINI @ 0x00599650`.

No third startup caller was found.

## 4. Crate loop and ordering

The crate bootstrap begins after the earlier selected-mode/start-unit portion of `Post_Map_Init`:

1. `0x0068695C` loads `DAT_00A8B261`; zero jumps past the entire loop at `0x00686963`.
2. Signed comparisons select `max(CrateMinimum, pregame_human_node_count)` and then cap it by `CrateMaximum` at `0x00686965..0x00686989`.
3. `TEST EAX,EAX` / `JLE` skips placement for a nonpositive final count.
4. Each positive iteration calls the random-cell crate placer at `0x00686994`.
5. The outer counter is decremented regardless of the placer's return value. This is a fixed number of attempts, not a top-up-until-success loop.
6. The helper then reaches its service call at `0x0068699C` and only afterward enters house/AI preparation.
7. `HouseClass__Add_Credits @ 0x004F9950` is called later at `0x00686A73`.
8. The selected-mode alliance callback runs later still in the `0x00686AD4..0x00686AF2` region.

The signed count is therefore:

```text
attempts = min(CrateMaximum, max(CrateMinimum, pregame_human_node_count))
if attempts <= 0: make zero attempts
otherwise: make exactly attempts calls, without retrying failed calls
```

For ordinary Skirmish startup, Rust must preserve the native relative order: initial crate attempts, then AI opening credits, then post-start alliance work.

## 5. INI and option authority

| Input | Authority in this slice | Verified effect |
|---|---|---|
| `[MultiplayerDialogSettings] Crates=` | launcher/session option, stock `yes` | gates the entire initial attempt loop through `0x00A8B261` |
| `[CrateRules] CrateMinimum=` | finalized Rules object | signed lower bound for attempts |
| `[CrateRules] CrateMaximum=` | finalized Rules object | signed upper cap for attempts |
| `[Basic] Official=` | scenario input passed to `Post_Map_Init` | not the crate gate or count source |

Scenario-specific rule overrides are finalized before the fixed-map caller decides whether to invoke `Post_Map_Init`, so startup crate bounds must be read from the same final layered rules authority used by the launched scenario.

## 6. Rust status at investigation time

Direct inspection of `src/sim/scenario_post_map.rs` found:

- `skirmish_session` already distinguishes the ordinary fixed-map Skirmish path from campaign and returns `ScenarioPostMapOutput.crates: Option<_>` accordingly. That gate is directionally correct and must remain.
- the function applies Skirmish AI opening credits before initial crate placement. That order contradicts active retail and is the implementation-facing defect found by this slice.
- a pending Phase 14 design draft proposed moving startup crates outside the Skirmish branch because `Post_Map_Init` has no internal game-mode check. This investigation falsifies that proposal before code was written.

No Rust file was changed during this investigation.

## 7. Coverage ledger

| Sub-area | Status | Evidence / boundary |
|---|---|---|
| fixed-map active parent | verified | `Read_Scenario_INI -> Full_Init`, `0x0068683A..0x00686845` |
| fixed-map raw-mode gate | verified | `0x00687BC8..0x00687BCE` |
| fixed-map second-byte gate | verified | `0x00687BD0..0x00687BD6` |
| scenario-rules finalization before post-map | verified | `0x00687B76` before `0x00687BEC` |
| generated-map direct parent | verified | `Read_Scenario`, `0x0068498E..0x00684990` |
| synthetic-INI preview exclusion | verified | `InitMapFromSyntheticINI` branch split |
| exhaustive function-parent set | verified | cold `get_function_callers` for `0x00686890` and `0x00686B20` |
| lobby crate option | verified | `0x0068695C..0x00686963`; retail `Crates=yes` |
| signed count selection | verified | `0x00686965..0x00686989` |
| nonpositive-count skip | verified | `TEST` / `JLE` before placer loop |
| attempt versus success ownership | verified | unconditional outer decrement after placer call |
| crate/AI-credit/alliance order | verified | placer `0x00686994`, credits `0x00686A73`, alliance callback later |
| random-cell placement semantics | excluded | owned by the active retail crate-runtime report |
| pickup/effect/regeneration semantics | excluded | separate Phase 14 mechanisms |
| network-mode Rust support | excluded | not admitted by the current production launch path |

## 8. Open Questions Log

- `[RESOLVED] Q1 — Does fixed-map campaign raw mode 0 reach Post_Map_Init?` No. `Full_Init` jumps over the call when `g_GameMode == 0`.
- `[RESOLVED] Q2 — What does the ordinary fresh fixed-map loader pass as the second control byte?` Zero, via `XOR DL,DL` before the call to `Full_Init`.
- `[RESOLVED] Q3 — What does a nonzero second control byte do?` It skips the `Post_Map_Init` call. A broader semantic name for the byte is unnecessary for this slice.
- `[RESOLVED] Q4 — Is there a separate generated-map caller?` Yes. Successful random generation calls `Post_Map_Init(1)` directly from `Read_Scenario`.
- `[RESOLVED] Q5 — Does synthetic-INI preview initialization seed crates?` No. The preview branch does not enter the in-game `Full_Init` path.
- `[RESOLVED] Q6 — What gates the crate loop and what is the stock setting?` `DAT_00A8B261`, sourced from the lobby/session `Crates` option; retail rules specify `Crates=yes`.
- `[RESOLVED] Q7 — Are the count comparisons signed?` Yes, including the final nonpositive skip.
- `[RESOLVED] Q8 — What is the order relative to AI credits and alliances?` Crate attempts first, AI credits second, alliance callback later.
- `[RESOLVED] Q9 — Are scenario rule overrides finalized first?` Yes, the optional scenario rules process occurs before the fixed-map post-map gate and call.
- `[RESOLVED] Q10 — Does the native gate admit nonzero raw modes other than offline Skirmish?` Yes, subject to the second byte, but current production network admission is outside scope.
- `[RESOLVED] Q11 — Does a failed random-cell placement get retried?` No. The requested attempt is consumed.
- `[RESOLVED] Q12 — Does `[Basic] Official` own the crate gate/count?` No. It is passed into the helper for other post-map behavior.

## 9. Adversarial checks

- **Crates disabled:** the global option jump bypasses the loop completely.
- **Campaign fixed map:** raw mode zero bypasses the entire helper at the caller.
- **Inverted or negative bounds:** the signed native comparisons and final `<= 0` skip are normative; Rust must not silently reinterpret them as unsigned.
- **Preview versus playable generated map:** preview does not seed; successful playable random generation does.
- **Placement failure:** one call still consumes one outer attempt; no hidden top-up loop exists.

The zero-add pass re-read both caller sets after these checks and produced no new unresolved question.

## 10. Implementation handoff

### Exact Rust obligations for the scenario-start seam

1. Preserve the existing campaign/Skirmish distinction: do not create scenario-start random crates for fixed-map campaign mode `0` from this helper.
2. Keep the lobby `Crates` option as the top-level bootstrap gate.
3. Derive signed attempts from the finalized layered Rules authority using `min(max(CrateMinimum, human_count), CrateMaximum)`, with `<= 0` making zero calls.
4. Make exactly that many placement calls; do not retry a failed/ghost attempt merely to reach a successful-overlay count.
5. In ordinary Skirmish post-map work, execute initial crate attempts before AI opening credits and before alliance application.
6. Admit the successful generated-random-map path once that production path supplies the same finalized rules/session inputs; do not seed crate state during preview generation.

### Required focused regressions

- fixed-map campaign produces no scenario-start crate bootstrap output;
- fixed-map Skirmish with `Crates=false` makes zero attempts;
- fixed-map Skirmish with `Crates=true` uses signed min/human/max attempt ownership;
- failed placement consumes one attempt rather than triggering top-up;
- an observable order test proves crates precede AI credits and alliances;
- preview random-map initialization has no gameplay crate side effect, while successful playable generation enters the post-map seam.

### Stale-document correction wording

Replace the `PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md` statement that `Post_Map_Init` has no game-mode gate with:

> `Post_Map_Init` itself has no internal mode comparison, but ordinary fixed-map activation is caller-gated: `Full_Init` skips it for raw mode 0 and when its second control byte is nonzero; fresh fixed-map nonzero modes call it. Generated random maps have a separate successful-generation caller. Therefore fixed-map campaign mode 0 does not receive startup scatter from this helper.

The placement, pickup, effect, and regeneration findings in that report remain authoritative unless separately contradicted.

## 11. Ghidra annotation candidates

None. Existing function names are adequate, and no metadata synchronization was requested. This investigation was read-only.

## Sources

- Live Ghidra disassembly/call graph for `ScenarioClass__Read_Scenario @ 0x00684620`, `ScenarioClass__Read_Scenario_INI @ 0x00686730`, `ScenarioClass__Post_Map_Init @ 0x00686890`, `ScenarioClass__Full_Init @ 0x00686B20`, and `RandomMapGenerator__InitMapFromSyntheticINI @ 0x00599650`.
- Live Ghidra caller queries for `0x00686890` and `0x00686B20`, repeated during the zero-add pass.
- `docs/research/PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md` for the previously verified placement/runtime mechanisms and the statement corrected here.
- `docs/research/SCENARIO_INIT_DEEP_DIVE.md` and `docs/research/LOADING_FUN_0069AE90_SKIRMISH_CALLERS_AFTER_FIRST_RENDERER_GHIDRA_REPORT.md` for corroborating fixed-map mode-gate context.
- `ini/rulesmd.ini` for active repository retail `CrateMinimum`, `CrateMaximum`, `CrateRegen`, and `Crates=yes` values.
- Direct Rust inspection of `src/sim/scenario_post_map.rs` at the Phase 14 frontier.
