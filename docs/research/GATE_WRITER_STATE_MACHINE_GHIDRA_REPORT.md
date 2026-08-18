# Gate Writer State Machine - Ghidra Research Report

**Address(es):** `0x0044E440` (Building mission `0x18` routine via vtable `+0x254`), `0x00452540` (allied gate opener/check), `0x004A51F0` / `0x004A5240` / `0x004A5290` / `0x004A5360` (`Building+0x350` state writers/finalizer), `0x00578AD0` (live caller)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** writer/state-machine paths that set gate mission `0x18` and the `Building+0x350` helper bytes `+0x18/+0x19` read by `BuildingClass::CanGarrison @ 0x004525F0`.
**Non-Scope:** `CanGarrison` result-code mapping except as context, civilian `CanDock`, full building render composition, vehicle door/helper reuse outside `Building+0x350`, and campaign map fixture runtime tracing.
**Confidence:** High for the static writer/state-machine contract; Medium for stock fixture frequency because no live debugger fixture was run.
**Active in YR:** Conditional. The code is in live YR building/pathing/AI dispatch; the player-visible branch requires a `Gate=yes` building such as stock `[GAGATE_A]`.

## 0. Working Notes Gate

- Target question: Which live writer/state-machine paths set building mission `0x18` and `Building+0x350` bytes `+0x18/+0x19` so infantry gate passability can be modeled without re-studying `CanGarrison` result codes?
- Non-goals: Do not re-study civilian garrison entry, bunker/UnitRepair row helpers, the already settled infantry `3/5/7` mapping, or broader rendering/vehicle helper users.
- Evidence needed to mark COMPLETE: decompile plus assembly for `0x00452540`, `0x0044E440`, `0x004A51F0`, `0x004A5240`, `0x004A5290`, `0x004A5360`; MissionClass dispatch evidence that mission `0x18` reaches building vtable `+0x254`; caller evidence from path/obstacle code; INI/default source for stock gate activation; Rust handoff.
- Stop conditions: write exactly this report, update only `.swarm-claims.md` if completed, leave Rust/INI/in-repo docs untouched, and ship no unresolved in-scope open questions.

## 1. Overview

The helper object at `Building+0x350` is a four-state transition timer. The two bytes are enough for a minimal runtime model: `+0x18` is "transition active", and `+0x19` is target/open-side state (`1` open, `0` closed). Active in YR: Yes as a live helper and building mission path; Conditional for player visibility on `Gate=yes` buildings.

`BuildingClass::CanGarrison`'s passable state is exactly the stable-open state: current mission `0x18` plus `Building+0x350/+0x18 == 0` and `+0x19 == 1`. The writer side reaches that state by assigning mission `0x18`, running the building mission routine at vtable `+0x254`, starting an opening transition with `0x004A51F0`, and later finalizing the timer with `0x004A5360`. Active in YR: Conditional on a live gate object.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x520` | `BuildingClass` | `BuildingTypeClass*` | `0x00452544`; `0x0044E440` reads `param_1[0x148]` | Yes |
| `+0x16B7` | `BuildingTypeClass` | `Gate=yes` flag for this branch | `0x0045254A`, `0x0044E446`; `[GAGATE_A] Gate=yes` in `rulesmd.ini:17186..17206` | Conditional |
| `+0x3C8/+0x3CC` | `BuildingTypeClass` | durations passed to gate state writers | `0x0044E51F..0x0044E52A`, `0x0044E5E3..0x0044E5EE`; writer multiplies second arg by `0x007E27F8` | Conditional |
| `+0xAC` | `MissionClass` | current mission read by `GetCurrentMission` | `MissionClass__GetCurrentMission @ 0x005B3040`; dispatch switch reads `+0xAC` at `0x005B30B2` | Yes |
| `+0xBC` (`param_1[0x2F]`) | `MissionClass` / `BuildingClass` | mission-local gate state `0..5`; reset to `0` on mission assign | assign assembly `0x005B2FFF`; mission routine switch in `0x0044E440` | Yes |
| `+0x350` | `BuildingClass` | transition helper base | `LEA EDI,[ESI+0x350]` in `0x00452568`; helper xrefs from drawing and mission code | Conditional |
| helper `+0x00` | transition helper | stored scaled duration as double | `0x004A5203..0x004A5211`, `0x004A5254..0x004A5262` | Conditional |
| helper `+0x08` | transition helper | start frame | `0x004A5218..0x004A5225`, `0x004A5269..0x004A5276` | Conditional |
| helper `+0x10` | transition helper | remaining/current duration ticks | `0x004A522E`, `0x004A527F`, `0x004A5297..0x004A52BB` | Conditional |
| helper `+0x14` | transition helper | total duration ticks | `0x004A5231`, `0x004A5282`, `0x004A52B5..0x004A52BB` | Conditional |
| helper `+0x18` | transition helper | active transition byte; `1` while opening/closing, `0` stable | predicates `0x004A5110`, `0x004A5130`, `0x004A51B0`, `0x004A51D0`; writers below | Conditional |
| helper `+0x19` | transition helper | open-side byte; `1` open/opening, `0` closed/closing | same helper predicates/writers | Conditional |

## 3. Core Logic

### 3.1 Four State Labels for `Building+0x350`

| Label for Rust model | `+0x18` | `+0x19` | Native predicate | Writer/finalizer evidence | `CanGarrison` passable? | Active in YR |
|---|---:|---:|---|---|---|---|
| `ClosedStable` | `0` | `0` | `FUN_004A51D0 == true` | close finalizer branch `0x004A5375..0x004A5382`; missing-boundary raw setter at `0x004A52E0` has no xrefs | No | Conditional |
| `Opening` | `1` | `1` | `FUN_004A5110 == true` | `StartOpening @ 0x004A51F0` writes `+0x18=1`, `+0x19=1`, timer fields | No | Conditional |
| `OpenStable` | `0` | `1` | `FUN_004A51B0 == true` | finalizer `0x004A5369..0x004A5371`; raw bytes at `0x004A52D0` set same but have no xrefs | Yes, only if mission is `0x18` | Conditional |
| `Closing` | `1` | `0` | `FUN_004A5130 == true` | `StartClosing @ 0x004A5240` writes `+0x18=1`, `+0x19=0`, timer fields; reversal helper can flip into this state | No | Conditional |

Material details:

- The passable state is not "opening"; it is stable open. Evidence: `CanGarrison @ 0x004525F0` calls only `0x004A51B0`, whose assembly requires `+0x18==0` then `+0x19==1` at `0x004A51B0..0x004A51C2`. Active in YR: Conditional.
- `StartOpening @ 0x004A51F0` does nothing if the helper is already stable open (`+0x18==0 && +0x19==1`); otherwise it writes active/opening and seeds timer fields. Evidence: assembly `0x004A51F6..0x004A5234`. Active in YR: Conditional.
- `StartClosing @ 0x004A5240` does nothing if already stable closed (`+0x18==0 && +0x19==0`); otherwise it writes active/closing and seeds timer fields. Evidence: assembly `0x004A5246..0x004A5285`. Active in YR: Conditional.
- `ReverseDirection @ 0x004A5290` only runs when active (`+0x18 != 0`); it rewrites `+0x10 = +0x14 - remaining` and toggles `+0x19`. Evidence: assembly `0x004A5290..0x004A52C9`. Active in YR: Conditional; directly called by `0x0044E440`.
- `FinishTransition @ 0x004A5360` is the only verified xref that writes stable bytes: active/opening becomes stable open; active/closing becomes stable closed. Evidence: decompile/assembly `0x004A5360..0x004A5385`, direct xref from `TechnoClass__AI_Update @ 0x006FA5D1`. Active in YR: Yes for AI update; Conditional for gate objects.
- Raw functions at `0x004A52D0` (`+0x18=0,+0x19=1`) and `0x004A52E0` (`+0x18=0,+0x19=0`) have no Ghidra function boundary and no xrefs found. Per constraint, no function was created. Evidence: raw bytes `0x004A52C9..0x004A52E8`; bulk xrefs empty. Active in YR: No verified active path.

### 3.2 Mission `0x18` Dispatch and Routine

`MissionClass::Mission_Dispatch @ 0x005B3060` reads current mission from `+0xAC`; case `0x18` calls vtable `+0x254`. For BuildingClass, data xref `0x007E4110` points to `0x0044E440`, so building mission `0x18` runs `FUN_0044E440`. Evidence: dispatch decompile case `0x18`; assembly `0x005B32A4..0x005B32C5`; `get_function_xrefs(0x0044E440)` reports data xref `0x007E4110`. Active in YR: Yes for MissionClass dispatch; Conditional for buildings assigned mission `0x18`.

`MissionClass::Assign_Mission @ 0x005B2FD0` resets mission-local state on any mission assignment except the special `current==0x1C && new==5` case. It writes `+0xAC=mission`, `+0xB4=-1`, byte `+0xB8=0`, dword `+0xBC=0`, timing fields `+0xC0/+0xC8`, and clears `+0xD0`. Evidence: assembly `0x005B2FE7..0x005B302A`. Active in YR: Yes.

The building mission `0x18` routine uses `+0xBC` as a local state:

| Local state | Verified behavior | Evidence | Active in YR |
|---:|---|---|---|
| `0` | Entry/setup. If stable open, goes to state `2` and seeds a hold timer. If not open/opening/closing, calls `StartOpening`. If already closing, calls `ReverseDirection`. Then moves to state `1`. | `0x0044E46A..0x0044E579`; calls `0x004A51B0`, `0x004A5110`, `0x004A5130`, `0x004A51F0`, `0x004A5290` | Conditional |
| `1` | Opening wait. If stable open, changes to state `2`; then shares frame/closed checks with state `4`. | `0x0044E5BC..0x0044E5CA`; fallthrough in decompile | Conditional |
| `2` | Open hold. Calls obstruction/occupant scan `0x0044E3A0`; if obstruction is present, re-seeds timer. If no obstruction and hold progress reaches 1.0, changes to state `3`. | `0x0044E5FE..0x0044E697`; helper `0x0044E3A0` scans occupied cells in building footprint | Conditional |
| `3` | Begin closing. Calls `StartClosing`, sets state `4`, plays configured sound. | `0x0044E6BB..0x0044E6E6`; `0x004A5240` call at `0x0044E5EE` in decompile/`0x0044E5EE` and case-3 xref `0x0044E5EE` | Conditional |
| `4` | Closing wait/frame update. If stable closed, calls vtable `+0x484(0,1)`, sets state `5`, dirty byte `+0x80=1`. Also updates byte `+0x703` from transition progress while any helper predicate is true. | `0x0044E6F6..0x0044E768`; assembly/decompile calls `0x004A51D0`, `0x004A5110`, `0x004A5130`, `0x004A51B0`, `0x004A52F0` | Conditional |
| `5` | No explicit switch case in `0x0044E440`; routine falls through to normal mission timer return. This is post-close idle while mission remains assigned until another mission change. | no `case 5` body in decompile; switch default common return | Conditional |

### 3.3 Live Entry That Sets Mission `0x18`

`MapClass__Check_Crushable_Obstacle @ 0x00578AD0` is a live path/obstacle check. When it finds a building occupant with `BuildingType+0x16B7 Gate=yes`, it branches on ownership:

- Allied gate: calls `FUN_00452540`. Evidence: decompile and assembly `0x00578B32..0x00578B5B`. Active in YR: Conditional on allied `Gate=yes` object in the checked cell.
- Enemy gate: calls the read-side `BuildingClass::CanGarrison @ 0x004525F0` and returns passable only if it is already open. Evidence: `0x00578B40..0x00578B4B`. Active in YR: Conditional.

`FUN_00452540` is the allied opener/check:

```text
if !Gate: return true
if mission == 0x18 and not Closing and not ClosedStable:
    return CanGarrison()  // true only stable-open
clear/retarget via vtable +0x1F0(-1)
Assign_Mission(0x18, 0)
Commence/Reset via vtable +0x1EC()
return false
```

Evidence: decompile and assembly `0x00452540..0x004525E0`; direct caller `0x00578B5B`. Active in YR: Conditional.

Important details:

- The opener deliberately does not return passable on the same call that assigns mission `0x18`; after assignment it returns false. Evidence: `0x004525C4..0x004525DD` calls assign/commence then returns `AL=0`. Active in YR: Conditional.
- If the gate is already mission `0x18` and in `Opening` (`+0x18=1,+0x19=1`), the opener avoids reassigning mission but still returns false because `CanGarrison` is false until stable open. Evidence: first branch only forces mission when `Closing` or `ClosedStable` or non-`0x18`; second branch calls `0x004A51B0`. Active in YR: Conditional.
- If the gate is `Closing` or `ClosedStable`, allied contact reassigns mission `0x18`, clearing local state to `0`; state `0` then reverses active closing or starts opening. Evidence: `0x00452570..0x00452582` jumps to assignment when `Closing`/`ClosedStable`; assign reset evidence above; state `0` evidence above. Active in YR: Conditional.

## 4. INI Keys / Stock Data

| Key | Stock evidence | Binary read/effect in this slice | Active in YR |
|---|---|---|---|
| `Gate=` | `[GAGATE_A] Gate=yes` in `ini/rulesmd.ini:17186..17204` and `ini/rules.ini:9394..9412` | consumed as `BuildingType+0x16B7` by `0x00452540`, `0x004525F0`, and `0x00578AD0` | Conditional on map-placed gate |
| `DeployTime=` | `[GAGATE_A] DeployTime=.044` at `rulesmd.ini:17205` | not directly named in scoped decompile, but `BuildingType+0x3C8/+0x3CC` durations are passed to the transition writers | Conditional |
| `GateCloseDelay=` | `[GAGATE_A] GateCloseDelay=.2` at `rulesmd.ini:17206` | likely source for the state-2 open-hold timer; exact parse offset not re-verified in this writer slice | Conditional |

## 5. Integration Points

| Integration point | Evidence | Active in YR |
|---|---|---|
| `TechnoClass__AI_Update` finalizes all active transition timers through `0x004A5150` then `0x004A5360` | decompile `0x006FA550`; xrefs `0x006FA5C6` and `0x006FA5D1`; assembly of helper/finalizer | Yes |
| Mission dispatch calls building mission `0x18` via vtable `+0x254` | `MissionClass__Mission_Dispatch @ 0x005B3060`; BuildingClass vtable data xref `0x007E4110 -> 0x0044E440` | Yes |
| Allied path/obstacle gate assigns mission `0x18` through `0x00452540` | `MapClass__Check_Crushable_Obstacle @ 0x00578AD0` direct call `0x00578B5B`; assign call `0x004525C8..0x004525CC` | Conditional |
| Read-side `CanGarrison` consumes only stable open while mission `0x18` | prior report plus decompile `0x004525F0`; helper `0x004A51B0` | Conditional |
| Building draw reads helper state to choose gate animation frame | `BuildingClass_DrawBody @ 0x0043D350` checks mission `0x18` and helper predicates; not expanded here | Conditional |

## 6. Current Rust Implementation Status

Current Rust has native cell-entry result names in `src/sim/pathfinding/cell_entry.rs`, including code `3` as `ScatterRequired`, code `5` as `OccupiedEnemy`, and code `7` as `Impassable`. The scanned surfaces do not contain a building `Gate=yes` runtime mission/state model or a building gate transition helper equivalent. Active Rust delta: missing.

The only visible `Gate` parser surfaced in the quick scan is overlay-oriented (`src/map/overlay_types.rs` / `src/map/resolved_terrain.rs`), not the scoped `BuildingType+0x16B7` building gate runtime. Current Rust delta: missing/unchecked for building type parsing.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Building+0x350` four helper predicates | verified | `0x004A5110`, `0x004A5130`, `0x004A51B0`, `0x004A51D0` decompile+assembly | none |
| `StartOpening @ 0x004A51F0` | verified | decompile+assembly `0x004A51F0..0x004A5238` | exact INI field source for duration |
| `StartClosing @ 0x004A5240` | verified | decompile+assembly `0x004A5240..0x004A5289` | exact INI field source for duration |
| `ReverseDirection @ 0x004A5290` | verified | decompile+assembly `0x004A5290..0x004A52C9`; xref from `0x0044E440` | none |
| `FinishTransition @ 0x004A5360` | verified | decompile+assembly `0x004A5360..0x004A5385`; xref from `TechnoClass__AI_Update` | none |
| raw setters `0x004A52D0/0x004A52E0` | touched-not-exhausted | raw bytes and empty xrefs | no function boundaries; no active path found |
| `MissionClass::Mission_Dispatch` case `0x18` | verified | `0x005B3060` decompile; assembly `0x005B32A4..0x005B32C5` | none |
| Building mission `0x18` routine `0x0044E440` | verified | vtable data xref `0x007E4110`; decompile | none for gate state slice |
| `FUN_00452540` mission writer | verified | decompile+assembly; caller `0x00578B5B` | none |
| `MapClass__Check_Crushable_Obstacle` caller | verified | `0x00578AD0` decompile+assembly | exact calling frequency/map fixture runtime trace |
| Unit/vehicle reuse of helper functions | deferred | examples: `UnitClass__Mission_Move @ 0x0073B0B0`, `UnitClass__Mission_Guard @ 0x00740A90` | out-of-scope; not `Building+0x350` |
| full gate render frame selection | deferred | `BuildingClass_DrawBody @ 0x0043D350` touched | out-of-scope visual frame parity |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-001 - What mode applies? -> exhaustive-slice for writer paths to mission 0x18 and Building+0x350 bytes.` (evidence: user target and bounded address set)
- `[RESOLVED] OQ-002 - What byte states exist? -> four states: closed stable, opening, open stable, closing.` (evidence: `0x004A5110`, `0x004A5130`, `0x004A51B0`, `0x004A51D0`)
- `[RESOLVED] OQ-003 - Which state is passable to CanGarrison? -> mission 0x18 plus stable open only.` (evidence: `0x004525F0`, `0x004A51B0`)
- `[RESOLVED] OQ-004 - Which writer starts opening? -> `0x004A51F0` writes active/opening and timer fields.` (evidence: `0x004A51F0..0x004A5238`)
- `[RESOLVED] OQ-005 - Which writer starts closing? -> `0x004A5240` writes active/closing and timer fields.` (evidence: `0x004A5240..0x004A5289`)
- `[RESOLVED] OQ-006 - Which helper reverses an active transition? -> `0x004A5290` toggles byte `+0x19` and adjusts remaining progress.` (evidence: `0x004A5290..0x004A52C9`)
- `[RESOLVED] OQ-007 - Who finalizes stable states? -> `TechnoClass__AI_Update` calls `0x004A5150`; when done it calls `0x004A5360`.` (evidence: `0x006FA5C6`, `0x006FA5D1`, `0x004A5150`, `0x004A5360`)
- `[RESOLVED] OQ-008 - Does mission 0x18 dispatch to the building routine? -> yes, case 0x18 calls vtable +0x254, BuildingClass vtable data xref points to 0x0044E440.` (evidence: `0x005B32A4..0x005B32C5`, `0x007E4110`)
- `[RESOLVED] OQ-009 - What assigns mission 0x18 for allied gate passability? -> `FUN_00452540` through vtable +0x1E8 after clearing via +0x1F0(-1).` (evidence: `0x004525B8..0x004525D6`)
- `[RESOLVED] OQ-010 - Is `FUN_00452540` live? -> yes, `MapClass__Check_Crushable_Obstacle` calls it for allied Gate=yes buildings.` (evidence: `0x00578B32..0x00578B5B`)
- `[RESOLVED] OQ-011 - Does mission assignment reset local state? -> yes, `Assign_Mission` resets +0xBC to 0 and timers.` (evidence: `0x005B2FE7..0x005B302A`)
- `[RESOLVED] OQ-012 - Does the opener allow same-call pass-through? -> no, mission assignment path returns false; passability only after stable open.` (evidence: `0x004525C4..0x004525DD`)
- `[RESOLVED] OQ-013 - Is stock data present? -> yes, `[GAGATE_A] Gate=yes` exists in base and md rules, map-placed only by TechLevel.` (evidence: `rulesmd.ini:17186..17206`, `rules.ini:9394..9414`)
- `[DEFERRED] OQ-014 - Which exact INI key maps to Type+0x3C8/+0x3CC?` (category: bounded-cost-too-high; reason: writer slice proved duration consumption but not parser origin; next-step-if-pursued: trace BuildingType parser offsets for DeployTime/GateCloseDelay)
- `[DEFERRED] OQ-015 - Which stock campaign/skirmish map exercises this path at runtime?` (category: needs-runtime-debugger; reason: static liveness and stock INI are proven, fixture frequency is not; next-step-if-pursued: breakpoint `0x00578B5B`/`0x0044E440` on a map with GAGATE_A)
- `[DEFERRED] OQ-016 - What is the exact semantic name of vtable +0x484 in state 4?` (category: requires-different-system-context; reason: not needed for read-side passability model; next-step-if-pursued: trace occupancy/visibility mutation side effects after stable close)
- `[DEFERRED] OQ-017 - Do raw setters at 0x004A52D0/0x004A52E0 have hidden callers?` (category: bounded-cost-too-high; reason: no function boundary and no xrefs found; next-step-if-pursued: runtime watch helper bytes or full code-pattern sweep)

Zero-add pass: re-read `0x00452540`, `0x0044E440`, `0x004A51F0`, `0x004A5240`, `0x004A5290`, `0x004A5360`, and `0x00578AD0`; no new in-scope branch was added.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Gate passability needs a runtime building gate state with four labels: `ClosedStable`, `Opening`, `OpenStable`, `Closing`; only `OpenStable` (`+0x18=0,+0x19=1`) satisfies `CanGarrison`, and only while mission is `0x18`. | `0x004A5110/5130/51B0/51D0`; `0x004525F0`; `0x0044E440` | missing | building runtime state plus `src/sim/pathfinding/cell_entry.rs` gate branch | Add a minimal building gate state machine separate from civilian garrison and overlay gates. | Set up a `Gate=yes` building in mission `0x18`; assert closed/opening/closing block but stable-open permits infantry branch continuation. Proposed test: `infantry_gate_passability_requires_mission_18_and_stable_open_state` | Do not treat "opening" as passable; `+0x18=1,+0x19=1` is still false for `CanGarrison`. |
| Allied obstacle checks assign mission `0x18` but return blocked on that same check; passability arrives after the opening transition finalizes. | `0x00578B5B -> 0x00452540`; assignment/false return `0x004525C4..0x004525DD`; finalizer `0x004A5360` | missing | movement/pathing gate-open request surface | When allied infantry/ground logic encounters a closed allied gate, request/open the gate and classify current entry as blocked until stable open. | First check against closed allied gate queues/sets opening but returns code `3`; after transition completion, same check returns clear/passable. Proposed test: `allied_closed_gate_request_opens_but_first_entry_check_still_blocks` | Do not make gate-open request instantly clear the cell in the same tick/check. |
| Mission `0x18` local state controls open/hold/close: state 0 starts/reverses opening, state 2 holds open while obstruction helper finds occupants, state 3 starts closing, state 4 waits for stable closed, state 5 post-close idle. | `0x0044E440` decompile; dispatch `0x005B3060`; assign reset `0x005B2FD0` | missing | building mission tick system | Add enough mission tick behavior to preserve passability timing and auto-close after no obstruction. | Gate opens, remains open while an object occupies the gate footprint, then starts closing after the hold timer when clear. Proposed test: `gate_mission_18_holds_open_while_footprint_occupied_then_closes` | Do not encode this as a static passability flag; the auto-close/obstruction hold affects pathing. |

## Negative Facts / Do Not Do

- Do not use mission `0x13` as the infantry gate passability mission. Evidence: `CanGarrison` compares current mission to literal `0x18` at `0x00452611`; mission dispatch case `0x18` reaches the gate state routine. Active in YR: Yes/Conditional.
- Do not treat `Opening` (`+0x18=1,+0x19=1`) as passable. Evidence: `0x004A51B0` rejects any nonzero `+0x18`; `0x004A5110` is a different predicate. Active in YR: Conditional.
- Do not merge building `Gate=yes` with overlay `Gate=yes`. Evidence: scoped binary reads `BuildingType+0x16B7` from `BuildingClass` objects and runs `Building+0x350` state; Rust overlay gate parsing is a separate map overlay surface. Active in YR: Conditional.
- Do not make allied gate opening instantaneous. Evidence: `0x00452540` assigns mission `0x18` then returns false; `0x004A5360` later stabilizes open after timer completion. Active in YR: Conditional.
- Do not create or depend on synthetic Ghidra functions at `0x004A52D0/0x004A52E0`. Evidence: raw bytes exist but no function boundary/xrefs were found; the verified live finalizer is `0x004A5360`. Active in YR: No verified active path.

## Remaining Uncertainty

- Exact parser source for `BuildingType+0x3C8/+0x3CC` durations was not re-verified in this writer slice.
- Exact stock campaign/skirmish frequency for `GAGATE_A` was not runtime-traced.
- Exact side effects of vtable `+0x484` after stable close were not expanded because they are not required for the minimal read-side passability model.

## Stale Docs / Follow-up Docs

- `docs/research/GATE_MECHANIC_BUILDING_GATE_PASSABILITY_GHIDRA_REPORT.md` replacement wording: "For infantry/building gate passability, use mission `0x18`, not mission `0x13`. `BuildingClass::CanGarrison @ 0x004525F0` requires `Gate=yes`, current mission `0x18`, and `Building+0x350` stable-open bytes `+0x18==0` and `+0x19==1`. Allied closed-gate contact assigns mission `0x18` through `FUN_00452540` but still returns blocked until the `Building+0x350` transition finalizes; do not describe infantry passability as simply 'open gate removes occupation' without this state-machine predicate."
- `docs/research/INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md` optional addendum wording: "The deferred writer names are now: `StartOpening @ 0x004A51F0`, `StartClosing @ 0x004A5240`, `ReverseDirection @ 0x004A5290`, `FinishTransition @ 0x004A5360`, and building mission `0x18` routine `0x0044E440`; helper states are `ClosedStable (0,0)`, `Opening (1,1)`, `OpenStable (0,1)`, `Closing (1,0)`."

## Sources

- Ghidra decompile/read-only: `0x0044E440`, `0x00452540`, `0x004525F0`, `0x004A5110`, `0x004A5130`, `0x004A5150`, `0x004A51B0`, `0x004A51D0`, `0x004A51F0`, `0x004A5240`, `0x004A5290`, `0x004A5360`, `0x00578AD0`, `0x005B2FD0`, `0x005B3060`, `0x006FA550`.
- Ghidra assembly/read-only: `0x00452540..0x004525E0`, `0x004A51F0..0x004A5238`, `0x004A5240..0x004A5289`, `0x004A5290..0x004A52C9`, `0x004A5360..0x004A5385`, `0x00578AD0..0x00578B64`, `0x005B2FD0..0x005B3030`, `0x005B32A4..0x005B32C5`.
- Raw bytes inspected read-only: `0x004A52C9..0x004A52E8`.
- Prior docs used for duplication/context: `INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md`, `GATE_MECHANIC_BUILDING_GATE_PASSABILITY_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/sim/pathfinding/cell_entry.rs`, `src/map/overlay_types.rs`, `src/map/resolved_terrain.rs`.
