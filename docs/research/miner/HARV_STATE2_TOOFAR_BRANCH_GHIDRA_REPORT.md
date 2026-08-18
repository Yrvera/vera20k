# HARV Mission_Harvest State 2 Too-Far Branch - Ghidra Research Report

**Address(es):** `0x0073E5E0` primary (`UnitClass__Mission_Harvest`), `0x004DF040` (`FootClass__Find_Docking_Bay`), `0x0056DC20` (`FootClass__Find_Nearby_Passable_Cell`), `0x005657A0` (`MapClass__Get_CellClass`), `0x0066D530` (`RulesClass__ReadGeneral`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard YR non-teleporter War Miner (`[HARV]`) state-2 return-to-refinery close/far branch: coordinate inputs, threshold inclusivity for `[General] HarvesterTooFarDistance=5`, close-enough behavior, and too-far fallback target.
**Non-Scope:** full dock radio sequence after state 3, unload/departure, chrono teleport timing, full `Find_Nearby_Passable_Cell` across all callers, A* internals, ore scan behavior, and Rust implementation changes.
**Confidence:** High for state-2 branch, threshold inclusivity, coordinate sources, and fallback seed/dispatch; Medium for exact passable-cell helper human parameter names beyond the already-verified callsite matrix.
**Active in YR:** Yes. Stock `[HARV]` has `Harvester=yes`, `Dock=NAREFN,GAREFN`, no `Teleporter=yes`, Drive locomotor, and standard stock refineries have `DockUnload=yes`/`Refinery=yes`.

## Working Notes

**Target question:** For stock non-teleporter HARV in `Mission_Harvest` state 2, what exact distance compare decides close docking vs too-far fallback, what coordinates feed it, is the threshold inclusive, and what target is assigned when the fallback runs?

**Non-goals:** Do not re-open CMIN teleport, dock unload timing, post-unload exit, armed behavior, ore scan occupancy, or Rust code changes.

**Evidence needed to mark COMPLETE:** direct `Mission_Harvest` decompile and assembly around the compare; INI reader/default/value evidence for `HarvesterTooFarDistance`; stock HARV/refinery INI liveness; fallback `Find_Docking_Bay` arg order; fallback seed and `Find_Nearby_Passable_Cell` callsite stack/ECX evidence; destination handoff evidence; Rust surface scan and concrete tests.

**Stop conditions:** Stop after state 2 either sends radio `0x02`, assigns the nearby fallback `CellClass*`, clears destination on null fallback, or returns with no state change. Do not mutate Ghidra, Rust, INI, or older docs.

## 1. Overview

In `Mission_Harvest` state 2, stock HARV first refuses to select a new target while it already has a destination. That owner-gated branch is not a no-effect early return: it joins the shared mission-delay epilogue, consumes `Scenario+0x218.RandomRanged(0,2)`, and returns the current mission's base delay plus the random jitter. If HARV is idle, it finds a dockable refinery and measures 3D lepton distance between the miner object coordinate and the refinery object coordinate. For non-teleporter HARV, `distance <= Rules.HarvesterTooFarDistance * 0x100` is the close branch; with stock rules this is inclusive at `5 * 256 = 1280` leptons.

When the close branch passes, HARV sends radio message `0x02` to the refinery and advances harvest substate to `3` only if the reply is `1`. When the close branch does not fire, state 2 performs a second fallback dock search (`arg3=1`) and, for normal HARV, assigns a fallback movement target only if the recomputed distance to that fallback refinery is strictly greater than `0x300` leptons (3 cells). That fallback target is not the accepted dock pad; it is a nearby passable `CellClass*` seeded from `refinery_anchor + BuildingType.QueueingCell`.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `+0xBC` (`param_1[0x2F]`) | Unit | harvest substate; `2` return, `3` enter handoff | `0x0073E5E0`, writes `+0xBC=3` at `0x0073EE68` | Yes |
| `+0x5A4` (`param_1[0x169]`) | Foot/Unit | existing destination/NavCom pointer; nonzero causes state 2 to skip new selection | `0x0073E5E0` state 2 | Yes |
| `+0x6C4` (`param_1[0x1B1]`) | Unit | type pointer | `0x0073E5E0` entry and dock-list access | Yes |
| `+0x3F8` | TechnoType | `Dock=` vector count | `0x0073E5E0`; stock `[HARV] Dock=NAREFN,GAREFN` | Yes |
| `+0xCD4` | TechnoType | `Teleporter=yes` flag; zero selects HARV non-chrono branch | `0x0073E5E0`; `[HARV]` section has no `Teleporter=yes` | Yes for distinction; false on HARV |
| `+0xE0E` | TechnoType | `Harvester=yes` gate | `0x0073E5E0`; `rulesmd.ini:8228` | Yes |
| `Rules+0xD78` | RulesClass | `HarvesterTooFarDistance` in cells | `0x0073EC0E`; reader at `0x0066FFE3..0x0066FFF0`; `rulesmd.ini:293` | Yes |
| `Rules+0xD7C` | RulesClass | `ChronoHarvTooFarDistance`; not used by HARV | `0x0073EE40`; reader at `0x00670003..0x0067001B`; `rulesmd.ini:294` | Yes for CMIN, negative fact for HARV |
| `BuildingType+0x1618/+0x161C` | BuildingType | `QueueingCell` X/Y offsets for fallback seed | `0x0073ED25`, `0x0073ED34`; `artmd.ini:1716,1773` | Yes |
| unit vtable `+0x528` | Unit/Foot | `Find_Docking_Bay` | callsites `0x0073EC41` and state-2 decompile | Yes |
| unit vtable `+0x278` | Radio transmit | close branch sends message `2` to refinery object | `0x0073EE54..0x0073EE59` | Yes |
| unit vtable `+0x480` | destination setter | fallback sets/clears destination | `0x0073EDB5`, `0x0073EE7F` | Yes |
| `Scenario+0x218` | `Random` object | shared state-2 mission-delay jitter source, inclusive range `0..2` | `0x0073EF8E..0x0073EFA2`; `Random__RandomRanged @ 0x0065C7E0` | Yes |

## 3. Core Logic

### 3.1 HARV Close/Far Branch

Verified binary behavior, Active in YR: Yes.

1. State 2 first checks `param_1[0x169]`. If the unit already has a destination, normal HARV does not select a new refinery target on this tick. Assembly `0x0073EB5A..0x0073EB62` (`MOV EAX,[EBP+0x5A4]`; `TEST EAX,EAX`; `JNZ 0x0073EF77`) proves that it joins the shared mission-delay epilogue described in §3.7 rather than returning without RNG/timing effects.
2. If idle, it calls vtable `+0x528` (`Find_Docking_Bay`) with:
   - dock-list pointer: `Type + 0x3E8` / `param_1[0x1B1] + 1000`
   - `arg2 = 0`
   - `arg3 = 0`
3. If the unit type `+0xCD4` teleporter flag is zero, the HARV branch runs.
4. The branch gets coordinates by virtual slot `+0x48` from the found refinery object and from the miner object. It computes:

```text
dx = miner_coord.x - refinery_coord.x
dy = miner_coord.y - refinery_coord.y
dz = miner_coord.z - refinery_coord.z
distance = ftol(sqrt(dx*dx + dy*dy + dz*dz))
```

5. It compares `distance` against `Rules+0xD78 << 8`.

Assembly evidence:

```text
0x0073EC08  MOV ECX,[g_RulesClass_Instance]
0x0073EC0E  MOV EDX,[ECX+0xD78]
0x0073EC14  SHL EDX,0x8
0x0073EC17  CMP EAX,EDX
0x0073EC19  JLE 0x0073EE51
```

The `JLE` proves the close branch is inclusive: equality at exactly `HarvesterTooFarDistance * 256` leptons is close, not too far.

Stock YR value: `rulesmd.ini:293` and `rules.ini:234` both set `HarvesterTooFarDistance=5`, so stock close threshold is `1280` leptons.

### 3.2 Close Branch Output

Verified binary behavior, Active in YR: Yes.

When `distance <= Rules+0xD78 * 0x100`, the branch jumps to `0x0073EE51`:

```text
0x0073EE54  PUSH ESI          ; refinery BuildingClass*
0x0073EE55  PUSH 0x2          ; radio message
0x0073EE57  MOV ECX,EBP       ; HARV
0x0073EE59  CALL [unit.vtable+0x278]
0x0073EE5F  CMP EAX,0x1
0x0073EE62  JNZ 0x0073EC1F
0x0073EE68  MOV [EBP+0xBC],0x3
```

The close branch does not set a cell destination and does not compute `QueueingCell`. It sends radio `0x02` to the refinery object. Only reply `1` advances harvest substate to `3`; substate `3` then queues mission `7` (`Mission_Enter`) at `0x0073EE8D..0x0073EE93`.

Player-visible effect: a HARV close enough to its chosen refinery begins the dock contact path without first driving to the `QueueingCell` staging target.

### 3.3 Fallback Dock Search And HARV 0x300 Gate

Verified binary behavior, Active in YR: Yes, conditional when the close branch fails or radio reply is not `1`.

If the close branch does not advance to substate `3`, state 2 brackets a second dock search:

```text
0x0073EC1F  MOV EDX,[g_MapEditorMode]
0x0073EC25  PUSH 0x1          ; fallback arg3
0x0073EC28  PUSH 0x0
0x0073EC27  INC EDX
0x0073EC41  CALL [unit.vtable+0x528]
0x0073EC4F  DEC ECX
0x0073EC52  MOV [g_MapEditorMode],ECX
```

This is the same fallback `Find_Docking_Bay(..., 0, 1)` path described by prior dock reports: it can return a refinery even when normal reservation/contact filtering prevented the close path.

After it finds a fallback refinery, state 2 recomputes the same miner-object to refinery-object 3D lepton distance. For normal HARV (`Teleporter=no`), the fallback movement branch is gated by a separate hardcoded threshold:

```text
0x0073ECD0  CMP EAX,0x300
0x0073ECD5  JG 0x0073ECDF
0x0073ECD7  TEST BL,BL        ; teleporter flag
0x0073ECD9  JZ 0x0073EF77
```

So for non-teleporter HARV:

- `distance > 0x300` (strictly greater than 768 leptons) -> compute fallback target.
- `distance <= 0x300` -> do not set a fallback target; state remains `2` and joins the shared mission-delay epilogue, including its scenario RNG draw.

This 3-cell gate is separate from `HarvesterTooFarDistance=5`. A normal HARV can fail the close/radio path, then still skip fallback movement if the fallback refinery is within 3 cells.

### 3.4 Fallback Target Formula

Verified binary behavior, Active in YR: Yes.

When the HARV fallback movement branch runs, it converts the fallback refinery object coordinate to a building anchor cell, then adds `BuildingType.QueueingCell`:

```text
0x0073ECE5  MOV ESI,[ESI+0x520]       ; BuildingType*
...
0x0073ED0D  SAR ECX,0x8               ; refinery cell X from lepton coord
0x0073ED10  SAR EAX,0x8               ; refinery cell Y from lepton coord
0x0073ED25  MOV DX,[ESI+0x1618]       ; QueueingCell.X
0x0073ED2C  ADD DX,CX
0x0073ED34  MOV AX,[ESI+0x161C]       ; QueueingCell.Y
0x0073ED3B  ADD [ESP+0x12],AX
```

The signed cell conversion is the usual `coord + (coord >> 31 & 0xFF) >> 8` form seen in the decompile, so negative lepton coordinates round toward zero in this helper path.

For stock refineries:

- `[NAREFN] QueueingCell=4,1` at `artmd.ini:1716`
- `[GAREFN] QueueingCell=4,1` at `artmd.ini:1773`

Therefore a GAREFN at anchor `(rx, ry)` seeds fallback from `(rx+4, ry+1)`. This is distinct from the close/Mission_Enter accepted CAN_DOCK cell `(rx+3, ry+1)` documented in the radio reports and matching `GAREFN RemoveOccupy1=3,1` at `artmd.ini:1795`.

### 3.5 Fallback `Find_Nearby_Passable_Cell` Arguments

Verified binary behavior, Active in YR: Yes.

The call at `0x0073ED75` uses ECX=`0x0087F7E8`, the `MapClass` singleton, not the HARV unit. The stack push order at `0x0073ED42..0x0073ED75` matches the newer `FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`:

| Logical input | Verified value | Evidence |
|---|---:|---|
| receiver / ECX | `MapClass` singleton `0x0087F7E8` | `0x0073ED66 MOV ECX,0x87f7e8` |
| output cell | local packed-cell out buffer | final two pushes before call |
| origin cell | `refinery_anchor + QueueingCell` | `0x0073ED25`, `0x0073ED34` |
| speed/passability type | `2` | `0x0073ED5E PUSH 0x2` |
| zone id | `-1` | `0x0073ED58 PUSH -0x1` |
| movement / bridge height args | `0,0` | `0x0073ED56..0x0073ED57` |
| width / height | `1,1` | `0x0073ED54..0x0073ED55` |
| reject overlay / height / object checks | `0,0,0` | zero pushes around `0x0073ED4F..0x0073ED53` |
| bridge cells allowed | `1` | `0x0073ED4E PUSH EAX`, with `EAX=1` |
| target preference cell | `{0,0}` | zeroed target words at `0x0073ED6B..0x0073ED70` |
| skip-first-quadrant | `0` | `0x0073ED4C PUSH ESI`, `ESI=0` |
| occupancy-rect check | `0` | `0x0073ED42 PUSH ESI`, `ESI=0` |

Important correction: the pushed literal `2` is not a search radius. It is the speed/passability argument. The helper's effective ring limit comes from receiver `+0xF4 + +0xF8`, capped at `0x20`; because the receiver is MapClass here, ordinary maps search up to the 32-ring cap.

### 3.6 Fallback Destination Handoff

Verified binary behavior, Active in YR: Yes.

After `Find_Nearby_Passable_Cell`, state 2 compares the packed result against the null-cell sentinel. If null, it clears destination:

```text
0x0073ED83  CMP AX,[0x00B1CFB8]
0x0073ED98  JZ 0x0073EE77
0x0073EE77  MOV EAX,[EBP]
0x0073EE7A  PUSH 0x1
0x0073EE7C  PUSH ESI          ; null
0x0073EE7F  CALL [unit.vtable+0x480]
```

If valid, it converts the packed cell through `MapClass__Get_CellClass @ 0x005657A0` and calls unit vtable `+0x480`:

```text
0x0073EDA8  MOV ECX,0x87f7e8
0x0073EDAD  CALL 0x005657A0   ; MapClass__Get_CellClass
0x0073EDB2  PUSH EAX          ; CellClass*
0x0073EDB3  MOV ECX,EBP       ; HARV
0x0073EDB5  CALL [unit.vtable+0x480]
0x0073EDBB  JMP 0x0073EF77
```

State remains `2`; there is no `+0xBC=3` write in the fallback destination branch. The valid-destination call then jumps to the same shared mission-delay epilogue as the existing-destination owner gate. The HARV drives toward this intermediate `CellClass*` and re-enters state-2 evaluation later.

### 3.7 Shared Mission-Delay RNG Tail

Verified binary behavior, Active in YR: Yes.

The owner-gated state-2 branch at `0x0073EB62` and the valid far-destination branch at `0x0073EDBB` both jump to `0x0073EF77`. That epilogue obtains the current mission-control entry, converts its delay field to the base integer delay, advances the scenario-owned RNG with an inclusive `RandomRanged(0,2)`, and returns `base_delay + jitter`:

```text
0x0073EF77  MOV ECX,EBP
0x0073EF79  CALL 0x005B3A00        ; current MissionControl entry
0x0073EF7E  FLD qword ptr [EAX+0x10]
0x0073EF81  FMUL qword ptr [0x007E27F8]
0x0073EF87  CALL 0x007C5F00        ; ftol
0x0073EF8C  MOV ESI,EAX             ; base mission delay
0x0073EF8E  MOV EAX,[0x00A8B230]    ; ScenarioClass*
0x0073EF93  PUSH 0x2
0x0073EF95  PUSH 0x0
0x0073EF97  LEA ECX,[EAX+0x218]     ; scenario Random
0x0073EF9D  CALL 0x0065C7E0         ; RandomRanged(0,2)
0x0073EFA2  ADD EAX,ESI
```

Evidence was rechecked on 2026-07-25 against the live `gamemd.exe` program in Ghidra using `disassemble_bytes(start_address=0x0073EB5A,length=32)`, `disassemble_bytes(start_address=0x0073EDB0,length=24)`, `disassemble_bytes(start_address=0x0073EF77,length=48)`, and `batch_decompile(functions=0x005B3A00,0x0065C7E0)`. `0x005B3A00` returns the current `g_MissionControl_Array` entry; `0x0065C7E0` is the inclusive, rejection-sampled `RandomRanged` helper. Because the bounds differ, the normal scenario RNG advances by at least one raw draw; rejection can consume more.

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `[General] HarvesterTooFarDistance` | `5` | Inclusive close-radio threshold for HARV: `5 * 256 = 1280` leptons | `rulesmd.ini:293`; `0x0073EC0E..0x0073EC19`; reader `0x0066FFE3..0x0066FFF0` | Yes |
| `[General] ChronoHarvTooFarDistance` | `50` | CMIN threshold; not selected for HARV because `Teleporter=no` | `rulesmd.ini:294`; `0x0073EE40..0x0073EE4B` | Yes for CMIN, no for HARV branch |
| `[HARV] Harvester` | `yes` | reaches harvester mission path | `rulesmd.ini:8228`; `0x0073E5E0` type flag `+0xE0E` | Yes |
| `[HARV] Dock` | `NAREFN,GAREFN` | dock list searched by state 2 | `rulesmd.ini:8225`; `Find_Docking_Bay` input `Type+0x3E8` | Yes |
| `[HARV] Locomotor` | Drive CLSID | confirms HARV is not a teleporter | `rulesmd.ini` HARV section; `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` | Yes |
| `[HARV] Teleporter` | absent / false | selects non-teleporter branch using `Rules+0xD78` | absence in `[HARV]` section; `0x0073E5E0` tests `Type+0xCD4` | Yes as false |
| `[NAREFN]/[GAREFN] QueueingCell` | `4,1` | fallback seed only | `artmd.ini:1716`, `artmd.ini:1773`; binary reads `+0x1618/+0x161C` | Conditional |
| `[GAREFN] RemoveOccupy1` | `3,1` | matches accepted dock pad, not fallback seed | `artmd.ini:1795`; prior radio reports | Yes in dock path |
| `[GAREFN]/[NAREFN] DockUnload` | `yes` | enables refinery dock radio path after close branch | `rulesmd.ini:11726`, `12519` | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | refinery identity for docking/unload | `rulesmd.ini:11727`, `12520` | Yes |

`RulesClass__ReadGeneral` uses current field values as `ReadInt` defaults. Assembly evidence for HARV:

```text
0x0066FFD7  MOV EDX,[ESI+0xD78]       ; current default
0x0066FFE3  PUSH 0x83C480             ; "HarvesterTooFarDistance"
0x0066FFEB  CALL 0x005276D0           ; CCINIClass__ReadInt
0x0066FFF0  MOV [ESI+0xD78],EAX
```

Chrono equivalent:

```text
0x0066FFF6  MOV ECX,[ESI+0xD7C]
0x00670003  PUSH 0x83C464             ; "ChronoHarvTooFarDistance"
0x0067000B  CALL 0x005276D0
0x0067001B  MOV [ESI+0xD7C],EAX
```

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass__Mission_Harvest @ 0x0073E5E0` | owns state-2 close/far decision and fallback target assignment | direct decompile and assembly | Yes |
| `FootClass__Find_Docking_Bay @ 0x004DF040` | iterates type `Dock=` vector and chooses a candidate dock object | direct decompile; state-2 vtable `+0x528` calls | Yes |
| radio transmit vtable `+0x278` | close branch sends `0x02` to refinery | `0x0073EE54..0x0073EE59` | Yes |
| `Find_Nearby_Passable_Cell @ 0x0056DC20` | fallback search from `QueueingCell` seed | `0x0073ED75` call; helper report | Yes |
| `MapClass__Get_CellClass @ 0x005657A0` | converts packed fallback cell to `CellClass*` | `0x0073EDA8..0x0073EDAD` | Yes |
| unit destination vtable `+0x480` | receives fallback cell or null clear | `0x0073EDB5`, `0x0073EE7F` | Yes |
| `MissionClass__GetMissionTimerEntry @ 0x005B3A00` | supplies the current mission-control delay field for the shared state-2 return | `0x0073EF77..0x0073EF8C`; direct decompile of `0x005B3A00` | Yes |
| `Random__RandomRanged @ 0x0065C7E0` | advances `Scenario+0x218` for inclusive `0..2` mission-delay jitter | `0x0073EF8E..0x0073EFA2`; direct decompile of `0x0065C7E0` | Yes |
| `RulesClass__ReadGeneral @ 0x0066D530` | reads `HarvesterTooFarDistance` into `Rules+0xD78` | `0x0066FFE3..0x0066FFF0` | Yes |

## 6. Current Rust Implementation Status

No Rust was edited by this research correction. The source comparison below
was refreshed on 2026-07-25 against `dev` commit
`613b25969933ab43b1d3a074fc4f9467575a8920`; it supersedes the report's
earlier pre-implementation scan.

| Rust surface | Current status vs this slice |
|---|---|
| `src/rules/ruleset.rs:587..588`, `:957..958` | Defaults and parses both harvester thresholds (`5`, `50`). |
| `src/sim/miner/mod.rs:176..230` | `MinerConfig` carries both `too_far_threshold_standard` and `too_far_threshold_chrono`; both now have callers. |
| `src/sim/miner/miner_system.rs:39..68` | `return_exceeds_too_far_threshold` reconstructs 3D lepton coordinates and compares squared distance for both miner kinds. This is closer than the stale chrono-only scan, but exact equivalence to native `ftol(sqrt(...))` at fractional boundary values remains UNCHECKED. |
| `src/sim/miner/miner_system.rs:693..767` | `handle_return` invokes standard far-return staging after refinery selection, but at this baseline it lacks the native top-of-state existing-NavCom owner gate. |
| `src/sim/miner/miner_system.rs:1093..1133` | `try_issue_standard_far_return_drive` applies `too_far_threshold_standard`, resolves the shared QueueingCell passable staging cell, and keeps `ReturnToRefinery`; it then uses metadata-free `issue_move_if_idle`, omitting merged acceleration/deceleration/slowdown authority. |
| `src/sim/miner/miner_system.rs:1458..1536` | `issue_outbound_ore_move` already owns the full merged-rule Drive profile for outbound ore; the far-return caller does not yet reuse it at this baseline. |
| `src/sim/miner/miner_tests.rs:505` | `return_close_enough_to_refinery_enters_dock` covers a close HARV behavior, but not exact inclusive threshold to refinery object coordinate and not the too-far fallback seed. |
| `src/sim/miner/miner_tests.rs:477` | `war_miner_does_not_teleport` covers negative chrono behavior, not standard HARV too-far movement. |
| `Simulation` scheduler / `handle_return` dispatch | Current Rust source scan does not show the state-2 shared `base mission delay + RandomRanged(0,2)` return contract. A movement/destination test must not certify unchanged RNG for either an existing destination or a valid far fallback. |

Primary refreshed Rust-facing gaps: the standard threshold and QueueingCell
staging path now exist, but the state-2 owner gate is missing, far-return
movement bypasses the full merged Drive profile, the native second-search plus
strict `0x300` fallback mechanism remains unproven, and the shared
mission-delay RNG/scheduler tail is absent/unchecked.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| HARV YR liveness | verified | `[HARV]` stock INI; `0x0073E5E0` harvester and teleporter gates | none |
| HARV close distance coordinate inputs | verified | `0x0073EDC8..0x0073EDE0` plus decompile | none |
| HARV threshold offset and inclusive compare | verified | `0x0073EC0E..0x0073EC19`; `rulesmd.ini:293` | none |
| `RulesClass__ReadGeneral` default/read/store | verified | `0x0066FFE3..0x0066FFF0`; `0x00670003..0x0067001B` | constructor seed outside scope; retail INI overrides stock |
| Close branch radio output and state write | verified | `0x0073EE51..0x0073EE68` | downstream Mission_Enter timing out-of-scope |
| Fallback second `Find_Docking_Bay` call | verified | `0x0073EC1F..0x0073EC52`; `0x004DF040` | exact internal reservation filtering covered by prior report |
| HARV hardcoded fallback `0x300` gate | verified | `0x0073ECD0..0x0073ECD9` | none |
| Fallback QueueingCell seed | verified | `0x0073ED0D..0x0073ED3B`; `artmd.ini:1716,1773` | none |
| Fallback helper argument order | verified | `0x0073ED42..0x0073ED75`; helper report | full helper semantics not re-opened except needed facts |
| Fallback destination handoff | verified | `0x0073ED83..0x0073EDB5`, `0x0073EE77..0x0073EE7F` | locomotor path execution out-of-scope |
| Existing-destination and valid-fallback shared delay/RNG epilogue | verified | `0x0073EB5A..0x0073EB62`, `0x0073EDB5..0x0073EDBB`, `0x0073EF77..0x0073EFA2`; `batch_decompile(0x005B3A00,0x0065C7E0)` | Rust mission-dispatch delay integration remains a separate implementation slice. |
| Accepted CAN_DOCK pad vs fallback seed | verified by cross-doc and INI | radio reports; `artmd.ini:1773,1795` | none for this slice |
| Rust implementation comparison | refreshed/touched-not-exhausted | direct source read at `dev` `613b2596` on 2026-07-25 | owner gate/profile are addressed by the active prerequisite; exact threshold rounding, second-search/`0x300`, and scheduler RNG remain separate residuals |

## 8. Open Questions - Final State Of The Investigation Log

- `[RESOLVED] OQ-1 - Is this code active for stock YR HARV? -> Yes. Stock HARV has `Harvester=yes`, `Dock=NAREFN,GAREFN`, no `Teleporter=yes`, and Drive locomotor; state 2 reaches the non-teleporter branch.` (evidence: `rulesmd.ini:8215..8270`, `0x0073E5E0`)
- `[RESOLVED] OQ-2 - What coordinates feed the close/far distance? -> Miner object coord and refinery object coord via vtable `+0x48`; not pad, center, accepted CAN_DOCK cell, or QueueingCell.` (evidence: `0x0073EDC8..0x0073EDE0`, `0x0073E5E0`)
- `[RESOLVED] OQ-3 - What threshold does HARV use? -> `Rules+0xD78` (`HarvesterTooFarDistance`) shifted left 8, stock `5*256=1280` leptons.` (evidence: `0x0073EC0E..0x0073EC19`, `rulesmd.ini:293`)
- `[RESOLVED] OQ-4 - Is the threshold inclusive? -> Yes. The branch uses `JLE`; equality takes the close radio path.` (evidence: `0x0073EC17..0x0073EC19`)
- `[RESOLVED] OQ-5 - What happens on close pass? -> HARV sends radio `0x02` to the refinery object and writes state `3` only on reply `1`.` (evidence: `0x0073EE54..0x0073EE68`)
- `[RESOLVED] OQ-6 - Does close pass use QueueingCell? -> No. QueueingCell is read only in the fallback movement branch after the second dock search.` (evidence: `0x0073ED25`, `0x0073ED34`; no QueueingCell read before `0x0073EE51`)
- `[RESOLVED] OQ-7 - What second dock search runs after close failure? -> `Find_Docking_Bay(Type+0x3E8,0,1)` bracketed by `g_MapEditorMode++/--`.` (evidence: `0x0073EC1F..0x0073EC52`)
- `[RESOLVED] OQ-8 - What additional HARV fallback gate exists? -> Normal HARV only assigns fallback movement when recomputed distance is strictly `>0x300` leptons; `<=0x300` returns with no target/state change.` (evidence: `0x0073ECD0..0x0073ECD9`)
- `[RESOLVED] OQ-9 - What is the fallback seed? -> refinery anchor cell plus `BuildingType+0x1618/+0x161C` (`QueueingCell`), stock `(4,1)`.` (evidence: `0x0073ED0D..0x0073ED3B`, `artmd.ini:1716,1773`)
- `[RESOLVED] OQ-10 - Is pushed literal `2` the fallback radius? -> No. It is speed/passability type; helper receiver is MapClass and ring limit is receiver size fields capped at 32.` (evidence: `0x0073ED5E`, `0x0073ED66`; `FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-11 - What does the fallback hand to movement? -> Null clears destination with force arg `1`; valid result becomes `CellClass*` via `MapClass__Get_CellClass` and is passed to unit vtable `+0x480`.` (evidence: `0x0073ED83..0x0073EDB5`, `0x0073EE77..0x0073EE7F`)
- `[RESOLVED] OQ-12 - Does fallback advance state to 3? -> No. There is no `+0xBC=3` write in fallback destination branch; state stays `2`.` (evidence: `0x0073ED75..0x0073EDBB`, decompile)
- `[RESOLVED] OQ-13 - What INI reader stores the threshold? -> `RulesClass__ReadGeneral` calls `CCINIClass__ReadInt` for `HarvesterTooFarDistance`, using existing `Rules+0xD78` as default and storing result back to `+0xD78`.` (evidence: `0x0066FFD7..0x0066FFF0`)
- `[RESOLVED, REFRESHED 2026-07-25] OQ-14 - What current Rust surfaces are affected? -> Standard HARV now applies `too_far_threshold_standard` and resolves QueueingCell staging in `try_issue_standard_far_return_drive`, but issues it through metadata-free `issue_move_if_idle`; `handle_return` also lacks the native existing-NavCom gate before refinery selection. Exact native threshold rounding, second-search/strict-`0x300` behavior, and the shared scheduler RNG tail remain unclosed.` (evidence: direct reads of `src/sim/miner/miner_system.rs:39..68,693..767,1093..1133,1458..1536` at `dev` `613b2596`)
- `[RESOLVED] OQ-17 - Is the state-2 owner/far-destination return RNG-neutral? -> No. Both the non-null destination jump and the valid fallback destination jump reach `0x0073EF77`, which returns the current mission's base delay plus `Scenario+0x218.RandomRanged(0,2)`.` (evidence: `0x0073EB5A..0x0073EB62`, `0x0073EDB5..0x0073EDBB`, `0x0073EF77..0x0073EFA2`; live `disassemble_bytes` and `batch_decompile` calls on 2026-07-25)
- `[DEFERRED] OQ-15 - Exact runtime frame count from close state-3 write to Mission_Enter dock admission.` (category: out-of-scope; reason: covered by close radio timing report and needs runtime scheduler trace for exact frame count; next-step-if-pursued: trace MissionClass scheduler around substate 3)
- `[DEFERRED] OQ-16 - Full helper internals for direct/indirect `FUN_006D6410` classification.` (category: out-of-scope; reason: newer helper report already covers the fallback call enough for this slot; next-step-if-pursued: dedicated `Find_Nearby_Passable_Cell` visual-height projection audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| HARV close/far decision uses miner object coordinate to refinery object coordinate, not accepted pad or QueueingCell, and close is inclusive at `HarvesterTooFarDistance*256`. | `0x0073EDC8..0x0073EE19`; `rulesmd.ini:293` | partial/unchecked: current standard HARV applies the threshold to reconstructed 3D lepton coordinates, but compares squared distance rather than native `ftol(sqrt(...))`; boundary equivalence is unproven | `src/sim/miner/miner_system.rs:39..68,1093..1115`, `src/sim/miner/mod.rs:176` | Preserve the standard split; separately prove or repair the exact native rounding boundary. | `war_return_at_exact_harvester_too_far_threshold_enters_close_radio_path`: HARV exactly 5 cells/lepton-equivalent from refinery object enters Dock/Approach path and does not stage at QueueingCell. | Do not certify squared-distance equivalence without covering subcell/Z fixtures around the truncation boundary. |
| HARV beyond the close threshold does a second fallback dock search and only assigns fallback movement if fallback refinery distance is `>0x300` leptons. | `0x0073EC1F..0x0073ECD9` | partial/unchecked: standard far staging exists, but exact native second-search and strict post-fallback `0x300` ownership are not closed by the refreshed source scan | `src/sim/miner/miner_system.rs:693..767,1093..1133` and refinery selection helper | Preserve the strict `>3 cells` secondary gate for non-teleporter fallback movement after reproducing the second-search authority. | `war_return_close_failure_within_three_cells_waits_without_queueingcell_move`: simulate close radio/reservation failure at <=3 cells and assert no QueueingCell movement is issued, state remains return/retry. | Do not treat mere presence of standard staging code as proof of the native two-search mechanism. |
| HARV too-far fallback seed is `refinery_anchor + QueueingCell`; valid result is converted to `CellClass*` and passed to `Set_Destination`; state remains `2`. | `0x0073ED0D..0x0073EDB5`; `artmd.ini:1716,1773` | partial: current HARV resolves QueueingCell staging and retains ReturnToRefinery, but metadata-free `issue_move_if_idle` omits the merged Drive profile | `src/sim/miner/miner_system.rs:1093..1133,1458..1536`, `src/sim/miner/miner_dock_sequence.rs` helper reuse | Reuse the full stock-miner Drive command authority for the existing staging target, while preserving state/tick behavior. | `war_return_far_uses_queueingcell_staging_not_can_dock_pad`: refinery `(10,10)`, HARV far away, movement final goal is `(14,11)`, not `(13,11)`, with merged Drive acceleration/deceleration/slowdown. | Do not use accepted CAN_DOCK pad `(rx+3,ry+1)` as the far fallback target or leave the Drive profile zeroed. |
| Existing-destination and valid far-destination state-2 exits share a mission-delay tail that returns `base_delay + RandomRanged(0,2)` from `Scenario+0x218`. | `0x0073EB5A..0x0073EB62`, `0x0073EDB5..0x0073EDBB`, `0x0073EF77..0x0073EFA2`; `0x005B3A00`, `0x0065C7E0` decompiles | missing/unchecked: current miner dispatch does not visibly consume this shared jitter or schedule the next mission call from the returned delay | mission scheduler and `src/sim/miner/miner_system.rs` dispatch integration | Trace the production mission scheduler, then consume the shared scenario RNG in the verified call order and apply the returned base-plus-jitter delay. | A binary-derived dispatch oracle should show the state-2 call advances the scenario RNG and delays the next harvest mission dispatch by the returned value for both an existing NavCom and a valid far fallback. | Do not add an isolated RNG draw without the associated scheduler-delay contract; do not require unchanged RNG in a state-2 full-dispatch oracle. |
| The fallback helper literal `2` is speed/passability type, not radius; ECX is MapClass, normal cap is up to 32 rings, with width/height 1 and occupancy-rect check disabled. | `0x0073ED42..0x0073ED75`; `FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md` | existing helper may use simplified radius and occupancy behavior | `src/sim/miner/miner_dock_sequence.rs` nearby-passable helpers | Reuse or extend the binary-equivalent fallback helper; do not create a radius-2 search for HARV. | `war_return_far_blocked_queueingcell_searches_beyond_radius_two`: block seed/ring 1/2 and leave later valid cell; fallback still finds according to binary cap. | Do not implement the stale "radius=2 around QueueingCell" wording. |

Concrete Rust test-name proposals:

- `war_return_at_exact_harvester_too_far_threshold_enters_close_radio_path`
- `war_return_one_lepton_beyond_threshold_uses_queueingcell_fallback`
- `war_return_far_uses_refinery_object_anchor_not_can_dock_pad_for_threshold`
- `war_return_close_failure_within_three_cells_waits_without_queueingcell_move`
- `war_return_far_blocked_queueingcell_uses_mapclass_fnpc_cap_not_radius_two`
- `war_return_far_valid_fallback_keeps_return_state_until_recheck`

## Negative Facts / Do Not Do

- Do not use `ChronoHarvTooFarDistance` for HARV. HARV has no `Teleporter=yes`, so the branch reads `Rules+0xD78`, not `Rules+0xD7C`.
- Do not measure the HARV close/far threshold to the accepted dock pad, refinery center, `RemoveOccupy1`, or `QueueingCell`. The binary measures object coordinate to object coordinate.
- Do not make equality too far. `distance == HarvesterTooFarDistance*256` is close (`JLE`).
- Do not advance state to `3` after assigning the fallback staging destination. Fallback keeps state `2`.
- Do not implement the fallback search as radius `2`; the literal `2` is speed/passability type.
- Do not treat `QueueingCell` as the accepted dock pad. Stock GAREFN fallback seed is `(rx+4,ry+1)` while accepted dock cell is `(rx+3,ry+1)`.
- Do not add chrono teleport/piggyback behavior to HARV. The stock `[HARV]` path is Drive locomotion and non-teleporter.
- Do not call the existing-destination or valid far-destination state-2 branch RNG-neutral. Both reach `RandomRanged(0,2)` in the shared mission-delay epilogue.

## Remaining Uncertainty

- Exact live frame separation after state `3` is queued remains runtime-scheduler work and belongs to the close radio timing trace, not this branch slice.
- The fallback helper parameter names beyond the callsite matrix are inherited from recent helper reports. The values and stack order are verified here; broader helper semantics were not re-opened.
- Rust comparison is a source scan, not an executed parity test. The report identifies affected surfaces and proposed tests but does not modify or run code.
- Exact Rust scheduler ownership for the returned mission delay is not closed here. The RNG call is verified, but implementing it safely requires tracing how the production Rust mission dispatcher stores/applies per-object delays.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_HARVEST_STATE2_TOOFAR_PATHFIND_BRANCH_GHIDRA_REPORT.md`: replace "Find_Nearby_Passable_Cell search radius = 2 cells" with "the state-2 fallback passes `speed_type=2`; effective search limit comes from the `MapClass` receiver `+0xF4 + +0xF8`, capped at 32 rings. The seed is still `refinery_anchor + QueueingCell`."
- Same file: replace "`param_1 (this) = harvester unit instance`" for the fallback helper call with "`ECX/receiver = MapClass singleton `0x0087F7E8`; the harvester is not the helper receiver in this callsite.`"
- Same file: replace "require occupancy-clear" for the scoped fallback call with "the final occupancy-rect check argument is `0`; this call does not invoke `CellRect__CheckOccupancy`, though `CheckPassability` still filters passability."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md`: replace "Find_Nearby_Passable_Cell(target_cell, ...)" if interpreted as a small local radius with "fallback seeds `refinery_anchor + QueueingCell`, calls MapClass receiver `Find_Nearby_Passable_Cell` with `speed_type=2`, width/height `1`, target `{0,0}`, and binary helper cap (normally 32 rings)."
- Any in-repo fidelity note saying "radius=2 around QueueingCell" should become "QueueingCell seed; literal `2` is SpeedType/passability, not radius."

## Sources

- Ghidra read-only decompile: `UnitClass__Mission_Harvest @ 0x0073E5E0`.
- Ghidra read-only assembly context: HARV close compare `0x0073EC08..0x0073EC19`.
- Ghidra read-only assembly context: close radio/state write `0x0073EE51..0x0073EE68`.
- Ghidra read-only assembly context: fallback search/gate/seed/call `0x0073EC1F..0x0073ED75`.
- Ghidra read-only assembly contexts rechecked 2026-07-25: existing-destination jump `0x0073EB5A..0x0073EB62`, valid fallback handoff/jump `0x0073EDB0..0x0073EDBB`, and shared delay/RNG epilogue `0x0073EF77..0x0073EFA2` via `disassemble_bytes`.
- Ghidra read-only batch decompile rechecked 2026-07-25: `MissionClass__GetMissionTimerEntry @ 0x005B3A00`, `Random__RandomRanged @ 0x0065C7E0`.
- Ghidra read-only decompile: `FootClass__Find_Docking_Bay @ 0x004DF040`.
- Ghidra read-only decompile: `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.
- Ghidra read-only decompile: `MapClass__Get_CellClass @ 0x005657A0`.
- Ghidra read-only assembly context: `RulesClass__ReadGeneral` harvester keys `0x0066FFE3..0x0067001B`.
- Prior reports read: `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md`, `CHRONO_MINER_MISSION_HARVEST_STATE2_RETURN_BRANCH_COORDS_GHIDRA_REPORT.md`, `MISSION_HARVEST_STATE2_TOOFAR_PATHFIND_BRANCH_GHIDRA_REPORT.md`, `MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md`, `FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan only: `src/sim/miner/mod.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs`, `src/rules/ruleset.rs`, `src/rules/art_data.rs`.
