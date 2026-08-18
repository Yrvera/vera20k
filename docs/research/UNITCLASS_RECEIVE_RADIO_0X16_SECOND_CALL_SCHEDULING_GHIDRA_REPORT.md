# UnitClass::Receive_Radio(0x16) Second-Call Scheduling - Ghidra Research Report

**Address(es):** `UnitClass::Receive_Radio @ 0x00737430`; supporting `BuildingClass::Receive_Radio @ 0x0043C2D0`, `FootClass::Mission_Enter @ 0x004D9290`, `FootClass::Receive_Radio @ 0x004D8FB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** source and effect of a later/already-synchronized `0x16` after the first ordinary `0x16` sets locomotor/facing rate and returns.  
**Non-Scope:** accepted-cell/GetDockCoord coordinate split, full `0x18` contact flag lifecycle, full `0x15` unload side effects, all non-refinery dock systems.  
**Confidence:** High for the scheduling source inside this slice; Medium for exact timer storage outside `Mission_Enter` return consumption.  
**Active in YR:** Yes. Stock `GAREFN/NAREFN` have `DockUnload=yes` and `Refinery=yes`; stock `[Enter] Rate=.016`.

## 0. Investigation Contract

**Target question:** What causes the later/already-synchronized `0x16` call after first ordinary `0x16` sets locomotor/facing rate and returns? Is it MissionEnter retry, explicit self-schedule, locomotor callback, building resend, or another mechanism? Does first `0x16` return affect building-side sound/retry/contact state?

**Non-goals:** Do not re-open the settled `NW+(3,1)` accepted target, `NW+(2,1)` `GetDockCoord`, `QueueingCell`, or physical bridge question. Do not decode all `0x18`/`0x15` side effects.

**Evidence needed to mark COMPLETE:** decompile plus assembly for `UnitClass::Receive_Radio(0x16)`; decompile plus assembly for building's `0x12 -> 0x18 -> 0x16` sender; decompile plus assembly for `FootClass::Mission_Enter` retry and `FootClass::Receive_Radio(0x12)` already-there return; INI proof for `[Enter] Rate`; current Rust surface scan.

**Stop conditions:** stop after proving the later `0x16` source for the stock refinery path and recording negative facts for self-schedule/locomotor callback/immediate building retry. Defer exact `0x18` field writes and `0x15` side effects to their own slots.

## 1. Overview

`UnitClass::Receive_Radio(0x16)` does not schedule another `0x16`. The first ordinary `0x16` calls the locomotor/facing vtable `+0x4C` with `0x4000` and returns `1`; the building treats that as success and performs no immediate retry.

The later `0x16` comes from the same building-side `CAN_DOCK(0x0E)` path being reached again on a later `FootClass::Mission_Enter` dispatch. On that later dispatch, `FootClass::Receive_Radio(0x12)` returns `0x14` if the unit is already at the sent cell, so `BuildingClass::Receive_Radio(0x0E)` sends `0x18` and then sends `0x16` again.

## 2. Class Layout / Key Offsets

| Owner | Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| Unit/Foot | `+0x674` | locomotor pointer; null asserts before vtable calls | `0x007376E0`, `0x0073771B` | Yes |
| Unit/Foot | `+0x388` | rate/facing timer object read by `RateTimer::Current` | `0x007376CE..0x007376D9` | Yes |
| Unit/Foot | `+0x6AF` | early gate; if nonzero, skip first-`0x16` rate setup | `0x007376BF..0x007376C7` | Yes |
| Unit/Foot | `+0x418` | destination/active-enter flag used before `0x15` send | `0x0073774A..0x00737752` | Yes |
| Unit/Foot | vtable `+0x278` | send/receive radio to another object | `0x00737773..0x0073777A`, `0x0043CACE`, `0x0043CADB` | Yes |
| Unit/Foot | vtable `+0x274` | radio self/event dispatch; not used by `0x16` to self-schedule `0x16` | no `0x274(0x16)` in case; `0x274(0x15)` exists in PerCellProcess | Yes |
| Locomotor | vtable `+0x10` | `Is_Moving` gate before later `0x15` | `0x00737735..0x0073773D` | Yes |
| Locomotor | vtable `+0x4C` | first `0x16` turn/sync command with `0x4000` | `0x00737705..0x00737709` | Yes |
| BuildingType | `+0x16B3` | `DockUnload=yes` branch, stock GAREFN/NAREFN | `0x0043C8E8..0x0043C8FA`; `rulesmd.ini` | Yes |
| BuildingType | `+0x16BC` | `Weeder=yes` branch, not stock refinery | `0x0043C8F2..0x0043C8FA` | Conditional |

## 3. Core Logic

### First `0x16` on ordinary approach

Verified binary flow:

1. `UnitClass::Receive_Radio(0x16)` first calls `FootClass::Receive_Radio(sender, 0x16, payload)`. Evidence: decompile `0x00737430`, case `0x16`; assembly `0x007376AD..0x007376BA`.
2. It reads `byte +0x6AF`; if nonzero, it skips the first-turn block. Evidence: `0x007376BF..0x007376C7`.
3. It calls `RateTimer::Current` on `+0x388` and compares returned word to `0x4000`. Evidence: `0x007376CE..0x007376D9`.
4. If current is not `0x4000`, it calls locomotor vtable `+0x4C(0x4000)` and returns `1`. Evidence: `0x007376E0..0x00737718`.
5. No `0x15` is attempted in that early-return block.
6. No `Set_Destination`, no `Mission_Enter` timer write, no self `0x16`, and no building resend is present in the `0x16` case.

### Later/already-synchronized `0x16`

If the first-turn early return does not fire, the case continues:

1. Assert/check locomotor pointer `+0x674`.
2. Call locomotor vtable `+0x10`; if moving, skip handoff.
3. Call `FootClass::GetDestination(0)`; require non-null.
4. Require unit byte `+0x418` nonzero.
5. Require destination `WhatAmI()==6` (building).
6. Require unit mission from vtable `+0x184` equals `7`.
7. Send radio `0x15` to the destination via vtable `+0x278`.

Evidence: decompile `0x00737430` case `0x16`; assembly `0x0073771B..0x00737780`, especially `0x00737738` (`Is_Moving`), `0x00737743` (`GetDestination`), `0x0073774A` (`+0x418`), `0x0073775C` (`WhatAmI`), `0x00737768` (`Mission`), `0x00737775..0x0073777A` (`push 0x15`, vtable `+0x278`).

### Source of the later `0x16`

`BuildingClass::Receive_Radio(0x0E)` is the verified resend source in this slice:

1. It sends `0x12` with the accepted cell payload. Evidence: `0x0043CAAE..0x0043CABE`.
2. It only enters the handoff block if `0x12` returns `0x14`. Evidence: `0x0043CABE..0x0043CAC1`.
3. It sends `0x18` to the requester. Evidence: `0x0043CAC7..0x0043CACE`.
4. It then sends `0x16` to the requester. Evidence: `0x0043CAD4..0x0043CADB`.
5. If `0x16` returns `1`, it jumps to normal success return; no sound and no retry are performed. Evidence: `0x0043CAE1..0x0043CAE4`.
6. If `0x16` returns anything other than `1`, it plays/requester event at `DAT_0089C848` through requester vtable `+0x174`. Evidence: `0x0043CAEA..0x0043CAF7`.

`FootClass::Receive_Radio(0x12)` is what flips the building path from "move accepted" to "already there":

1. If payload pointer is non-null, it calls payload object's vtable `+0x48` to get coordinates.
2. It converts payload lepton X/Y to signed cell X/Y by adding sign correction and shifting by 8.
3. It calls this unit's occupied-cell vtable `+0x1B8`.
4. If the unit occupied cell matches payload cell, it returns `0x14`.

Evidence: decompile `FootClass::Receive_Radio @ 0x004D8FB0`, case `0x12`; assembly `0x004D9140..0x004D9197`, especially the cell compare at `0x004D9180..0x004D9189` and return `EAX=0x14` at `0x004D918B..0x004D9197`.

`FootClass::Mission_Enter` is the repeat driver:

1. Each dispatch calls the destination/request target with radio `0x0E`. Evidence: `0x004D92B2..0x004D92BF`.
2. After the dispatch body, it fetches the mission timer entry, loads `Rate`, multiplies by `900`, converts with `Math::ftol`, and adds `RandomRanged(0,2)`. Evidence: `0x004D946C..0x004D9497`.
3. Stock `[Enter] Rate=.016`, so stock retry cadence is `ftol(.016 * 900) + RandomRanged(0,2)` = `14..16` frames. Evidence: `ini/rulesmd.ini:[Enter] Rate=.016`; same value in base `rules.ini`.

Therefore the later `0x16` is not an immediate self-schedule. It is a building resend on a later `Mission_Enter -> 0x0E` pass after `0x12` reports "already at the target cell."

## 4. INI Keys

| INI key | Stock value | Role in this slice | Evidence |
|---|---:|---|---|
| `[Enter] Rate` | `.016` | Mission retry delay base; `Rate * 900 + RandomRanged(0,2)` | `rulesmd.ini:30507..30510`; `0x004D946C..0x004D9497` |
| `[GAREFN] DockUnload` | `yes` | Enables stock refinery `0x0E -> 0x12 -> 0x18 -> 0x16` branch | `rulesmd.ini:11726`; `0x0043C8E8..0x0043C8FA` |
| `[NAREFN] DockUnload` | `yes` | Same for Soviet refinery | `rulesmd.ini:12519`; `0x0043C8E8..0x0043C8FA` |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | Stock refinery identity, but not the `0x16` scheduling source itself | `rulesmd.ini:11727`, `12520` |

## 5. Integration Points

| Integration point | Verified behavior | Evidence |
|---|---|---|
| `Mission_Enter -> Building 0x0E` | one `0x0E` per mission dispatch | `0x004D92B2..0x004D92BF` |
| `Building 0x0E -> Unit 0x12` | sends accepted cell payload and checks for `0x14` | `0x0043CAAE..0x0043CAC1` |
| `Unit 0x12 already-there` | returns `0x14` only when occupied cell equals payload cell | `0x004D9140..0x004D9197` |
| `Building 0x0E -> Unit 0x18 -> Unit 0x16` | sends `0x18`, then `0x16`, synchronously after `0x12 == 0x14` | `0x0043CAC7..0x0043CADB` |
| `Unit 0x16 first-turn` | calls locomotor `+0x4C(0x4000)` and returns `1` | `0x007376BF..0x00737718` |
| `Unit 0x16 later/aligned` | if idle + destination building + mission 7 + flag, sends `0x15` | `0x0073771B..0x00737780` |
| Building after `0x16 == 1` | success return only; no sound/retry | `0x0043CAE1..0x0043CAE4` |
| Building after `0x16 != 1` | plays requester event/sound at `DAT_0089C848` | `0x0043CAEA..0x0043CAF7` |

## 6. Current Rust Implementation Status

Rust currently has several good pieces but still compresses the binary's radio timing:

| Rust surface | Current behavior observed | Delta vs verified binary |
|---|---|---|
| `src/sim/miner/miner_dock_sequence.rs:598` `phase_mission_enter` | when already at accepted cell and gates pass, marks `contact_entered` and goes directly to `Linked` | Missing explicit `0x12 == 0x14 -> 0x18 -> first 0x16 returns after turn command -> later retry `0x16` can send `0x15` split |
| `src/sim/miner/miner_dock_sequence.rs:686` `phase_awaiting_accepted_cell` | after movement arrival, immediately returns to `MissionEnter` | Correct directionally, but no stock 14..16 frame MissionEnter timer is visible in this local phase |
| `src/sim/miner/miner_dock_sequence.rs:700` `phase_linked` | snaps snapshot to `pad`, marks contact/on-pad, starts pivot and sound | Too early for first `0x16`; building's first `0x16 == 1` should not itself start unload handoff, and building sound is not from `0x16 == 1` path |
| `src/sim/miner/miner_dock_sequence.rs:746` `phase_pivoting` | models a facing timer using `FacingClass`, then starts `Unloading` | Useful mechanism, but should map to first `0x16` turn/sync phase before `0x15`/unload, not after a collapsed `Linked` handoff |
| `src/sim/miner/miner_dock.rs:20` `RefineryDockContacts` | has `contact_entered` and `on_pad` split | Good surface for modeling `0x18` contact separate from physical/on-pad unload |
| `src/sim/miner/mod.rs:86` `RefineryDockPhase` | has `MissionEnter`, `AwaitingAcceptedCell`, `Linked`, `Pivoting`, `Unloading` | Needs an explicit first-`0x16` synchronized-turn phase or redefinition of `Pivoting` before `Linked`/`0x15` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Receive_Radio(0x16)` first-turn branch | verified | `0x007376BF..0x00737718` | none for scheduling |
| `UnitClass::Receive_Radio(0x16)` later `0x15` branch | verified | `0x0073771B..0x00737780` | full `0x15` receiver side effects are another slot |
| `UnitClass::Receive_Radio(0x16)` self-schedule search inside case | verified negative | decompile case `0x16`; no self `0x16`, no timer enqueue | global binary-wide xref search not available in this slot, but owner case is decisive |
| `BuildingClass::Receive_Radio(0x0E)` `0x12 -> 0x18 -> 0x16` sender | verified | `0x0043CAAE..0x0043CADB` | exact `0x18` field writes deferred |
| Building response to `0x16` return value | verified | `0x0043CAE1..0x0043CAF7` | identify `DAT_0089C848` sound/event name if needed |
| `FootClass::Receive_Radio(0x12)` already-there return | verified | `0x004D9140..0x004D9197` | none for scheduling |
| `FootClass::Mission_Enter` dispatch and delay return | verified | `0x004D92B2..0x004D92BF`, `0x004D946C..0x004D9497` | exact mission timer storage/decrement is outside this slice |
| Current Rust phase surfaces | touched-not-exhausted | Codegraph + `rg` + file read | no Rust patch in this report |
| Full `0x18` contact lifecycle | deferred | scoped out | separate slot/report |
| Full `0x15` unload side effects | deferred | scoped out | separate slot/report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does `0x16` self-schedule another `0x16`? -> No within the owner case; first-turn branch calls locomotor `+0x4C(0x4000)` and returns `1`, later branch can send only `0x15`.` (evidence: `0x007376BF..0x00737780`)
- `[RESOLVED] OQ-02 - Does first ordinary `0x16` cause building to retry immediately? -> No; building treats return `1` as success and returns.` (evidence: `0x0043CAE1..0x0043CAE4`)
- `[RESOLVED] OQ-03 - What building-side condition sends `0x16`? -> `BuildingClass::Receive_Radio(0x0E)` sends it only after `0x12` returns `0x14`, and after sending `0x18`.` (evidence: `0x0043CAAE..0x0043CADB`)
- `[RESOLVED] OQ-04 - What makes `0x12` return `0x14`? -> `FootClass::Receive_Radio(0x12)` compares occupied cell to payload object's cell and returns `0x14` on equality.` (evidence: `0x004D9140..0x004D9197`)
- `[RESOLVED] OQ-05 - What repeats the building `0x0E` path? -> `FootClass::Mission_Enter` sends one `0x0E` per dispatch and returns the mission timer delay.` (evidence: `0x004D92B2..0x004D92BF`, `0x004D946C..0x004D9497`)
- `[RESOLVED] OQ-06 - What is stock Enter retry cadence? -> `ftol(.016 * 900) + RandomRanged(0,2)`, i.e. `14..16` frames for stock `[Enter]`.` (evidence: `rulesmd.ini:[Enter] Rate=.016`, `0x004D946C..0x004D9497`)
- `[RESOLVED] OQ-07 - Does a locomotor callback send the second `0x16`? -> No evidence in this slice; locomotor `+0x4C` is called and then the owner case returns. The verified later call source is MissionEnter/building resend.` (evidence: `0x007376E0..0x00737718`, `0x004D92B2..0x0043CADB`)
- `[RESOLVED] OQ-08 - Does first `0x16` affect building sound? -> Yes by return value only: return `1` suppresses the fallback requester event/sound; non-`1` plays `DAT_0089C848`.` (evidence: `0x0043CAE1..0x0043CAF7`)
- `[RESOLVED] OQ-09 - Does first `0x16` write building contact state? -> Not in the `0x16` owner case; building has already sent `0x18` before `0x16`.` (evidence: `0x0043CAC7..0x0043CADB`, `0x007376AD..0x00737780`)
- `[RESOLVED] OQ-10 - Is this active for stock YR refineries? -> Yes for GAREFN/NAREFN through `DockUnload=yes`; Yuri YAREFN slave-miner building is outside this stock miner path.` (evidence: `rulesmd.ini:11726`, `12519`; `0x0043C8E8..0x0043C8FA`)
- `[DEFERRED] OQ-11 - Exact field writes performed by `0x18`.` (category: out-of-scope; reason: this slot only needed whether first `0x16` schedules/resends; next-step-if-pursued: use the dedicated `0x18` lifecycle slot)
- `[DEFERRED] OQ-12 - Exact unload state writes performed by building/unit `0x15`.` (category: out-of-scope; reason: this slot only needed second-call scheduling; next-step-if-pursued: use the dedicated `0x15` side-effects slot)
- `[DEFERRED] OQ-13 - Exact mission timer storage/decrement site after `Mission_Enter` return.` (category: requires-different-system-context; reason: `Mission_Enter` return calculation is proven, but dispatcher storage/decrement is broader MissionClass scheduling; next-step-if-pursued: decode mission dispatch timer owner)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| First ordinary `0x16` sets locomotor/facing target `0x4000` and returns `1`, without sending `0x15` | `0x007376BF..0x00737718` | `phase_linked` currently starts link/pivot/sound after accepted-cell MissionEnter pass | `src/sim/miner/miner_dock_sequence.rs:700`, `:746`; `src/sim/miner/mod.rs:86` | model a first-`0x16` sync/turn phase that does not start unload or mark on-pad unload | `first_0x16_sets_pivot_but_does_not_start_unload` | Do not collapse first `0x16` into `Linked`/`Unloading` |
| Later `0x16` comes from later `Mission_Enter -> Building 0x0E` resend after `0x12 == 0x14` | `0x004D92B2..0x004D92BF`; `0x0043CAAE..0x0043CADB`; `0x004D9140..0x004D9197` | Rust returns from accepted-cell arrival to `MissionEnter`, but no visible 14..16 frame retry gate in local phase | `src/sim/miner/miner_dock_sequence.rs:598`, `:686` | gate the post-arrival MissionEnter retry by stock `[Enter]` delay before the building can resend `0x18/0x16` | `accepted_cell_arrival_waits_enter_rate_before_resend_0x16` | Do not resend `0x16` next tick unless the stock mission timer is due |
| Building sends `0x18` before each qualifying `0x16` resend | `0x0043CAC7..0x0043CADB` | Rust has `contact_entered` but currently marks in `phase_mission_enter`/`phase_linked` before a distinct first-vs-later `0x16` split | `src/sim/miner/miner_dock.rs:20`, `src/sim/miner/miner_dock_sequence.rs:598` | keep `contact_entered` as a separate `0x18` state; do not equate it with physical on-pad unload | `building_resend_marks_contact_entered_before_0x16_without_on_pad_link` | Do not use `on_pad` as a proxy for `0x18` |
| Building return handling: `0x16 == 1` means success/no fallback sound; non-`1` plays requester event/sound | `0x0043CAE1..0x0043CAF7` | Rust emits `DockDeploy` in `phase_linked`, regardless of binary `0x16` return distinction | `src/sim/miner/miner_dock_sequence.rs:700`; `src/sim/world/mod.rs` sound event | move/guard sound emission to the verified `0x15`/unload-start side, not first `0x16 == 1` | `first_0x16_return_1_does_not_emit_dock_deploy_sound` | Do not play deploy sound merely because first turn sync started |
| Later/aligned `0x16` can send `0x15` only if not moving, destination exists and is building, `+0x418` is set, mission is 7 | `0x0073771B..0x00737780` | Rust should check state source and movement idleness before unload start | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs` | require idle + destination/refinery + contact-entered + MissionEnter/mission-7 equivalent before starting unload | `second_0x16_requires_idle_destination_building_contact_and_mission_enter` | Do not let pivot completion alone start unload if the radio gates have not been met |

### Negative Facts / Do Not Do

- Do not implement `0x16` as self-scheduling another `0x16`.
- Do not implement the locomotor turn command as a callback that sends `0x16`.
- Do not retry `0x16` immediately inside building code when first `0x16` returns `1`.
- Do not treat first `0x16 == 1` as unload start.
- Do not play the fallback requester event/sound on first `0x16 == 1`; that path is explicitly skipped.
- Do not use `GetDockCoord` movement or physical NW+2 movement to explain second-call scheduling; this report did not re-open coordinates and the resend source is radio/timer based.

### Stale Docs / Follow-up Docs

Replace any wording like "`0x16` schedules its own later retry" with:

> `0x16` does not self-schedule. First ordinary `0x16` can set the locomotor/facing target to `0x4000` and return `1`; the later `0x16` is produced by a later `Mission_Enter -> BuildingClass::Receive_Radio(0x0E)` pass when the unit-side `0x12` reply is already-there (`0x14`), so the building sends `0x18` then `0x16` again.

## Sources

- Ghidra decompile: `UnitClass::Receive_Radio @ 0x00737430`
- Ghidra assembly: `0x007376AD..0x00737780`
- Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Ghidra assembly: `0x0043CAAE..0x0043CAF7`
- Ghidra decompile: `FootClass::Receive_Radio @ 0x004D8FB0`
- Ghidra assembly: `0x004D9140..0x004D9197`
- Ghidra decompile: `FootClass::Mission_Enter @ 0x004D9290`
- Ghidra assembly: `0x004D92B2..0x004D92BF`, `0x004D946C..0x004D9497`
- INI: `ini/rulesmd.ini`
- Rust scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/mod.rs`
