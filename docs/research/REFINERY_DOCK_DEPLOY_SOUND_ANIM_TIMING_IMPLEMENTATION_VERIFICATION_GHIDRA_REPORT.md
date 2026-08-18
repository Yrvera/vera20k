# Refinery Dock Deploy Sound / Anim Timing - Ghidra Research Report

**Address(es):** `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x00451750` (`BuildingClass::SetAnimSlotImage`), `0x00451890` (`BuildingClass::CreateAnimForSlot`), `0x00427D00` (`AnimTypeClass::ReadINI`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact timing/source of stock refinery unload-start sound and building animation slot 7 around `Mission_Deploy_Building`
**Non-Scope:** full cargo drain economics, refinery occupancy/contact lifetime, all building anim slot parser variants outside the slots touched by refinery unload
**Confidence:** High for stock YR; Medium for modded `PreProductionAnim` sound behavior because the generic AnimClass playback path is sourced from prior sound report plus reader decompile, not runtime audio capture
**Active in YR:** Yes for the mission path; stock slot-7 visual/sound is inactive because stock refinery art leaves `PreProductionAnim` unset

## Working Notes

- Target question: Does stock refinery unload play a DockDeploy-like sound and when does slot 7 animation start?
- Non-goals: Do not rediscover `0x15`/`0x16` handoff, accepted cell, contact flag, or cargo-credit formulas unless this evidence directly contradicts them.
- Evidence needed to mark COMPLETE: decompile plus assembly range for unload-start, animation slot calls, sound call inventory, and stock INI/art evidence for slot/sound fields.
- Stop conditions: stop after the stock unload-start sound/slot-7 timing is proven and Rust handoff is stated; defer runtime audio capture only if binary+INI cannot distinguish an actual sound.

## 1. Overview

Stock refinery unloading starts in `UnitClass::Mission_Deploy_Building`, not at radio `0x15`. The first-entry unload latch sets unit fields, optionally calls building anim slot 7, then writes substate `+0xBC = 3` and returns through the mission timer epilogue. In stock `GAREFN/NAREFN`, slot 7 is called but has no configured `PreProductionAnim`, so it creates no visible anim and no anim sound.

## 2. Class Layout / Key Offsets

| Field / slot | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `Unit+0x6D1` | unload first-entry/draw latch | `0x0073DFBD..0x0073DFDA` reads zero then writes `1` | Yes |
| `Unit+0xF8` | bale-rate accumulator | `0x0073DFD0` writes zero before latch | Yes |
| `Unit+0x100..0x10C` | periodic accumulator | `0x0073DFE0..0x0073DFFC` writes current frame/start data and enable | Yes |
| `Unit+0xBC` | deploy/unload substate | `0x0073E093` writes `3` after slot-7 call | Yes |
| Building anim slot `7` | `PreProductionAnim` family | `0x0073E08A` pushes `7`; `0x00451750` computes type slot entry | Conditional: engine-live, stock refineries unset |
| Building anim slot `10` / `0xA` | `SpecialAnim` family | `0x0073E3B6` pushes `0xA`; artmd has `GAREFNOR/NAREFNOR` | Yes |
| Building anim slot `8` | `ProductionAnim` family | `0x0073E513` pushes `8`; stock refineries unset/commented | Conditional: engine-live, stock refineries unset |
| AnimType `+0x2F8` | `StartSound=` / `Report=` voc index | `AnimTypeClass::ReadINI @ 0x00427D00` stores index `0xBE` | Conditional: only if anim type defines sound |
| AnimType `+0x2FC` | `StopSound=` voc index | `AnimTypeClass::ReadINI @ 0x00427D00` stores index `0xBF` | Conditional |

## 3. Core Logic

### Unload latch and slot 7 timing

`Mission_Deploy_Building` reaches the stock harvester branch after `PathType::Has_Valid_Steps()` succeeds and the facing/rate timer check accepts the east-facing slot. Evidence: `0x0073DF56..0x0073DF72` computes `((timer >> 7) + 1) & 0x1FE` and compares with `0x80`; the non-accepted branch calls locomotor/facing and returns `5`.

When `Unit+0x6D1 == 0`, the first-entry block runs:

| Order | Operation | Evidence | Active in YR |
|---|---|---|---|
| 1 | reset `Unit+0xF8 = 0` | `0x0073DFD0` | Yes |
| 2 | set `Unit+0x6D1 = 1` | `0x0073DFDA` | Yes |
| 3 | initialize `+0x100..+0x10C` | `0x0073DFE0..0x0073DFFC` | Yes |
| 4 | if unit type `Harvester=yes`, look up adjacent refinery building | decompile `0x0073D630`, block before `0x0073E05F` | Yes for `HARV/CMIN` |
| 5 | call `SetAnimSlotImage(slot=7, damaged_flag, 0, 0)` | `0x0073E08A PUSH 0x7`; `0x0073E08E CALL 0x00451750` | Conditional visual |
| 6 | write `Unit+0xBC = 3` | `0x0073E093` | Yes |
| 7 | jump to mission timer epilogue | `0x0073E09D -> 0x0073E289` | Yes |

Slot 7 therefore occurs at mission `0x10` unload latch/facing acceptance, after `+0x6D1` is set and before substate `3`, not at `0x15`, not at first `0x16`, and not at first cargo drain.

### Why stock slot 7 is a no-op

`BuildingClass::SetAnimSlotImage @ 0x00451750` selects a per-slot art entry, then returns if the string pointer is empty. Assembly shows the empty gate: `0x004517AB TEST EDX,EDX`, `0x004517AF CMP byte ptr [EDX],0`, `0x004517B2 JZ 0x004517C4`; only non-empty entries call `BuildingClass::CreateAnimForSlot @ 0x004517BF`.

Stock art leaves the relevant refinery slot entries commented:

| Building | Stock art evidence | Active in YR |
|---|---|---|
| `NAREFN` | `artmd.ini:1748 ;PreProductionAnim=NAREFN_A`, `artmd.ini:1749 ;ProductionAnim=NAREFN_AR` | Slot-7 engine call yes; visual no |
| `GAREFN` | `artmd.ini:1763..1798` has `SpecialAnim=GAREFNOR` but no `PreProductionAnim` or `ProductionAnim` | Slot-7 engine call yes; visual no |
| base RA2 `NAREFN` | `art.ini:1139 ;PreProductionAnim=NAREFN_A`, `art.ini:1140 ;ProductionAnim=NAREFN_AR` | fallback also unset |
| base RA2 `GAREFN` | `art.ini:1154..1188` has `SpecialAnim=GAREFNOR` but no `PreProductionAnim` | fallback also unset |

### Per-bale visual starts later, not at unload latch

The visible stock ore-dump building animation is slot 10 (`SpecialAnim`), not slot 7. The per-bale threshold block does:

| Order | Operation | Evidence | Active in YR |
|---|---|---|---|
| 1 | compare `HarvesterDumpRate * 900.0 <= Unit+0xF8` | `0x0073E355..0x0073E374` | Yes |
| 2 | call building vtable `+0x468` particle/smoke effect | `0x0073E37A..0x0073E37E` | Yes |
| 3 | if slot-10 pointer is empty, call `SetAnimSlotImage(10, ...)` | `0x0073E384..0x0073E3BA` | Yes |
| 4 | then `StorageClass::FindFirstNonEmptySlot` | `0x0073E3BF..0x0073E3C5` | Yes |

Stock art defines `GAREFNOR` and `NAREFNOR` as one-shot ore-transfer animations with no `StartSound=`, `Report=`, or `StopSound=` lines in their sections (`artmd.ini:17473..17480`, `artmd.ini:17575..17582`). Active in YR: Yes for visual, No for sound by INI.

## 4. Sound Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| Radio `0x15` has no sound or anim side effects | Parent report `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`; this report did not re-open that settled fact | Yes, negative |
| First `0x16` has no sound | Parent report `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`; this report did not re-open that settled fact | Yes, negative |
| Stock unload latch has no direct `VocClass`/deploy sound call in the harvester branch | `UnitClass::Mission_Deploy_Building @ 0x0073D630` decompile; assembly around `0x0073DFBD..0x0073E09D` contains field writes and `SetAnimSlotImage(7)`, no `VocClass__PlayAtCoord` call | Yes, negative |
| The only direct `VocClass__PlayAtCoord` observed in `Mission_Deploy_Building` is outside the stock harvester refinery branch | decompile shows `VocClass__PlayAtCoord` after non-harvester deploy/passenger work; assembly `0x0073DC50..0x0073DC67` calls `0x00750E20` after type sound check | No for stock `HARV/CMIN -> GAREFN/NAREFN`; Conditional for other deploy-like paths |
| Anim-created sounds would come from the spawned `AnimType` `StartSound=`/`Report=` field, not from a building DockDeploy event | `AnimTypeClass::ReadINI @ 0x00427D00` reads `StartSound` then `Report` into `+0x2F8`; `ANIMATION_SOUNDS_GHIDRA_REPORT.md` verifies playback via `AnimClass::AI/Middle` | Conditional |
| Stock refinery unload anim types have no configured sound | `artmd.ini:17473..17480` (`GAREFNOR`) and `artmd.ini:17575..17582` (`NAREFNOR`) contain no `StartSound=`, `Report=`, or `StopSound=`; stock slot 7 unset | Yes, negative |
| `[AudioVisual] DeploySound=` is not a stock refinery unload sound source | `rulesmd.ini:709 DeploySound=` blank; no `Mission_Deploy_Building` stock harvester branch read of this key was found | Yes, negative |

Conclusion: **stock `GAREFN/NAREFN` refinery unloading should not emit a `DockDeploy`/deploy sound at unload latch**. A modded refinery `PreProductionAnim` or `SpecialAnim` could produce anim-type sounds only if the spawned anim section defines `StartSound=`/`Report=`/`StopSound=`.

## 5. INI Keys

| Key / section | Stock value | Binary use | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[GAREFN] DockUnload` | `yes` | admits stock refinery unload path in receiver reports | Yes |
| `rulesmd.ini:[NAREFN] DockUnload` | `yes` | same | Yes |
| `rulesmd.ini:[GAREFN]/[NAREFN] Refinery` | `yes` | state-3/state-4 anim/exit checks read `Type+0x16BB` | Yes |
| `rules.ini:[HARV]/[CMIN] UnloadingClass` | `HORV` / `CMON` | unit draw swap keyed by `Unit+0x6D1`, per prior display report | Yes |
| `artmd.ini:[GAREFN] SpecialAnim` | `GAREFNOR` | slot 10 per-bale visual | Yes |
| `artmd.ini:[NAREFN] SpecialAnim` | `NAREFNOR` | slot 10 per-bale visual | Yes |
| `artmd.ini:[NAREFN] ;PreProductionAnim` | commented `NAREFN_A` | slot 7 call finds empty entry | No for stock visual; Conditional for mods |
| `artmd.ini:[GAREFN] PreProductionAnim` | absent | slot 7 call finds empty entry | No for stock visual; Conditional for mods |
| `artmd.ini:[GAREFNOR]/[NAREFNOR] StartSound/Report/StopSound` | absent | no anim sound is configured | No for stock sound |
| `rulesmd.ini:[AudioVisual] DeploySound` | blank | not read by stock refinery unload branch | No for this path |

## 6. Integration Points

`Mission_Deploy_Building` is reached after the building receives `0x15` and queues sender mission `0x10`. This report verifies only the later mission handler surface. The slot-7 call is synchronous inside mission `0x10` first-entry after facing/rate acceptance. Slot-10 `SpecialAnim` is later and tied to dump-rate threshold crossings.

The generic anim sound system is separate: `BuildingClass::CreateAnimForSlot` constructs an `AnimClass`, and `AnimTypeClass::ReadINI` supplies optional `StartSound`/`Report`/`StopSound`. Stock refinery slot 7 does not instantiate an anim, so there is no chance for an anim-type sound there.

## 7. Current Rust Implementation Status

Current Rust in `src/sim/miner/miner_dock_sequence.rs:805..830` does:

- `link_on_pad`
- set `display_type_override = UnloadingClass`
- force east facing
- push `SimSoundEvent::DockDeploy`
- set `unload_timer`
- transition to `RefineryDockPhase::Unloading`

Current Rust in `src/sim/miner/mod.rs:103..115` correctly documents that `FaceSync` and `MissionQueued` are not `0x15` unload side effects. That split is aligned with the settled parent model.

Delta from this report:

- `display_type_override` at unload latch is broadly aligned with `Unit+0x6D1` display swap timing, assuming it remains tied to mission `0x10` first-entry and not `0x15`.
- `SimSoundEvent::DockDeploy` at unload latch is a stock mismatch. No stock `GAREFN/NAREFN` unload-start sound is proven; the stock visible sound surface is absent unless an anim type defines sound, and stock refinery anim types do not.
- Rust does not appear to model slot 7 `PreProductionAnim` as a distinct conditional building anim trigger. For stock assets this is not visible because slot 7 is unset; for modded art it is an engine-supported conditional gap.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building` first-entry harvester branch | verified | decompile `0x0073D630`; assembly `0x0073DFBD..0x0073E09D` | none for slot-7/sound timing |
| `SetAnimSlotImage(slot=7)` call timing | verified | `0x0073E08A PUSH 0x7`, `0x0073E08E CALL 0x00451750` | none |
| `SetAnimSlotImage` empty-entry behavior | verified | decompile `0x00451750`; assembly `0x004517AB..0x004517BF` | none |
| stock refinery `PreProductionAnim` data | verified | `artmd.ini:1706..1763`, `art.ini:1097..1154` | none |
| stock refinery `SpecialAnim` data | verified | `artmd.ini:1739`, `1787`, `17473..17582` | none |
| direct sound calls in stock harvester unload branch | verified-negative | decompile `0x0073D630`; no direct sound call in `0x0073DFBD..0x0073E09D` | runtime audio capture unnecessary for direct call proof |
| non-harvester direct sound call in same function | touched-not-exhausted | assembly `0x0073DC50..0x0073DC67` | out-of-scope path; not stock refinery |
| generic AnimClass sound playback | touched-not-exhausted | `AnimTypeClass::ReadINI @ 0x00427D00`; prior `ANIMATION_SOUNDS_GHIDRA_REPORT.md` | full playback stack not re-decompiled in this narrow slot |
| current Rust `start_unload_deploy` | verified for comparison | `src/sim/miner/miner_dock_sequence.rs:805..830` | exact future patch out-of-scope |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does stock refinery unload-start play sound at radio 0x15? -> No; settled parent report says 0x15 only queues mission 0x10.` (evidence: `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-2 - Does first ordinary 0x16 play sound? -> No; settled parent report says first 0x16 only syncs locomotor/facing/rate.` (evidence: `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-3 - Does mission 0x10 first-entry play a direct deploy sound? -> No direct sound call in the stock harvester branch.` (evidence: `0x0073DFBD..0x0073E09D`)
- `[RESOLVED] OQ-4 - When is slot 7 called? -> At mission 0x10 first-entry after facing/rate acceptance, after +0x6D1 is set, before +0xBC=3.` (evidence: `0x0073DFBD..0x0073E093`)
- `[RESOLVED] OQ-5 - Is stock slot 7 visible? -> No, because stock refinery art has no active PreProductionAnim.` (evidence: `artmd.ini:1706..1763`, `art.ini:1097..1154`, `0x004517AF`)
- `[RESOLVED] OQ-6 - Does stock slot 7 create an anim sound? -> No, because no slot-7 anim is created.` (evidence: `0x004517AF..0x004517BF`, stock art lines above)
- `[RESOLVED] OQ-7 - Is stock per-bale SpecialAnim sound-bearing? -> No stock StartSound/Report/StopSound on GAREFNOR/NAREFNOR.` (evidence: `artmd.ini:17473..17582`, `0x00427D00`)
- `[RESOLVED] OQ-8 - Does `[AudioVisual] DeploySound=` feed this path? -> No evidence in mission 0x10 harvester branch, and stock key is blank.` (evidence: `rulesmd.ini:709`, `0x0073D630`)
- `[RESOLVED] OQ-9 - Is `UnloadingClass` display override timing compatible with unload latch? -> Broadly yes: prior display report ties it to +0x6D1, which is set at first-entry before state 3.` (evidence: `.swarm-claims.md` row for `HARV_UNLOADING_CLASS_DISPLAY_TIMING`, `0x0073DFDA`)
- `[DEFERRED] OQ-10 - Exact runtime mixer behavior for a modded PreProductionAnim with StartSound.` (category: out-of-scope; reason: stock assets do not configure this; next-step-if-pursued: runtime-capture modded slot-7 anim with StartSound)

## 10. Visual / Audio Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `SetAnimSlotImage(7)` at `0x0073E08E` | `UnitType+0xE0E Harvester=yes`, adjacent refinery found | stock none; modded `PreProductionAnim` | building slot placement | `CreateAnimForSlot` path if non-empty | No visible stock; Conditional mod | pre-production/unload latch |
| 2 | building vtable `+0x468` at `0x0073E37E` | dump-rate threshold crossed | refinery smoke/particle surface | building/refinery-local | particle path | Yes | per-bale effect |
| 3 | `SetAnimSlotImage(10)` at `0x0073E3BA` | threshold crossed and slot-10 pointer empty | `GAREFNOR` / `NAREFNOR` | building special anim slot | normal anim path | Yes | ore-transfer visual |
| 4 | `SetAnimSlotImage(8)` at `0x0073E517` | empty cargo and `Refinery=yes` | stock none; `ProductionAnim` unset/commented | building slot placement | normal anim path | No visible stock; Conditional mod | completion/depart guard |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `GAREFNOR` | yes | conditional per-bale | yes | no | no | yes | yes | no | `artmd.ini:17473..17480`, `0x0073E3BA` |
| `NAREFNOR` | yes | conditional per-bale | yes | no | no | yes | yes | no | `artmd.ini:17575..17582`, `0x0073E3BA` |
| `NAREFN_A` | no stock section active; commented | no stock | no stock | no | no | no | conditional mod | yes stock | `artmd.ini:1748`, `17585..17593` commented |
| `NAREFN_AR` | no stock section active; commented | no stock | no stock | no | no | no | conditional mod | yes stock | `artmd.ini:1749`, `17595..17605` commented |

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock unload latch does not emit DockDeploy/deploy sound | `0x0073DFBD..0x0073E09D`; no direct sound call; stock slot 7 unset | mismatch: `start_unload_deploy` pushes `SimSoundEvent::DockDeploy` | `src/sim/miner/miner_dock_sequence.rs:start_unload_deploy`; `src/app_sim_tick.rs` DockDeploy mapping | remove/suppress stock `DockDeploy` for `GAREFN/NAREFN` unload latch; only play anim-type sounds if a real spawned anim defines them | HARV/CMIN starts unloading at stock refinery; no deploy sound event is queued before first bale drain | Do not use radio `0x15`, first `0x16`, or mission `0x10` latch as a generic DockDeploy sound site |
| Slot 7 call occurs at mission `0x10` first-entry after facing/rate acceptance and before state 3 | `0x0073DFD0..0x0073E093` | missing/unchecked: no distinct slot-7 building anim trigger seen | building anim/event surface around refinery unload start | if modeling modded art, trigger `PreProductionAnim` slot 7 at unload latch; for stock, no visible result because entry empty | modded refinery with `PreProductionAnim=...` starts slot 7 once before first dump threshold; stock refinery shows no slot-7 visual | Do not delay stock unload or show `NAREFN_A`; it is commented out |
| Stock visible ore-transfer anim is slot 10 per dump threshold, before storage drain | `0x0073E355..0x0073E3C5`; `artmd.ini:GAREFNOR/NAREFNOR` | partially implemented through bale events/app building anim per search | `src/app_building_anim.rs` and `BaleDepositEvent` pipeline | keep/refine slot-10 SpecialAnim as per-bale/slot-drain event, not unload-start event | full ore-only harvester triggers one `GAREFNOR/NAREFNOR` pulse at dump threshold, before credit/drain effect is observed | Do not move `SpecialAnim` to `0x15` or unload latch |
| `UnloadingClass` display swap is tied to `Unit+0x6D1` unload-active latch | `0x0073DFDA`; prior `HARV_UNLOADING_CLASS_DISPLAY_TIMING_GHIDRA_REPORT` claim | broadly aligned if current Rust starts override only at `start_unload_deploy` | `src/sim/miner/miner_dock_sequence.rs:start_unload_deploy`, unit draw/render override | keep display override at mission `0x10` unload latch, not `0x15`; clear at stock state-4 equivalent | first unload-active frame uses `HORV/CMON`; `0x15`/`MissionQueued` frame does not | Do not use display override as evidence for sound or building slot-7 visual |

## 12. Negative Facts / Do Not Do

- Do not play `DockDeploy` at radio `0x15`. Active in YR: No.
- Do not play `DockDeploy` at first ordinary `0x16`. Active in YR: No.
- Do not play a generic deploy sound at stock mission `0x10` unload latch. Active in YR: No evidence; stock INI/art gives no source.
- Do not treat slot 7 as the stock ore-transfer visual. Active in YR: No visible stock slot-7 because `PreProductionAnim` is unset/commented.
- Do not treat `NAREFN_A` or `NAREFN_AR` as active stock refinery animations. Active in YR: No; both are commented in stock art.
- Do not source refinery unload sound from `[AudioVisual] DeploySound=`. Active in YR: No for this path, and the stock key is blank.
- Do not merge slot 7 and slot 10 timing. Slot 7 is first-entry/unload latch; slot 10 is later per-bale threshold.

## Sources

- Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra assembly: `0x0073DF56..0x0073E09D`, `0x0073E355..0x0073E3C5`
- Ghidra decompile/assembly: `BuildingClass::SetAnimSlotImage @ 0x00451750`, `0x004517AB..0x004517BF`
- Ghidra decompile: `BuildingClass::CreateAnimForSlot @ 0x00451890`
- Ghidra decompile: `AnimTypeClass::ReadINI @ 0x00427D00`
- INI/art: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`, `ini/rules.ini`
- Referenced reports: `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`, `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`, `ANIMATION_SOUNDS_GHIDRA_REPORT.md`, `miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`
