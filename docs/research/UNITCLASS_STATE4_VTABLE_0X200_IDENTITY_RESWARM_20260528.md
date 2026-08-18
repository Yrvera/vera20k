# UnitClass State-4 Vtable +0x200 Identity - Reswarm Report

**Address(es):** `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `UnitClass::ShouldIdle @ 0x00744270`, `MissionClass::Queue_Mission @ 0x005B35E0`, `MissionClass::Commence @ 0x005B3570`, `UnitClass::AI @ 0x007360C0`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** the concrete UnitClass vtable `+0x200` target and return semantics for the healthy stock `HARV`/`CMIN` `Mission_Deploy_Building` state-4 post-unload exit.
**Non-Scope:** full `Mission_Deploy_Building`, cargo drain, radio protocol internals, war-factory exit mission behavior, non-stock modded refinery art, and runtime same-frame two-miner trace.
**Confidence:** High for vtable identity, branch ordering, predicate semantics, and stock healthy result; Medium for friendly names of `Unit+0x6E1/+0x6E2` and the tiny two-byte helper at `Unit+0x350`.
**Active in YR:** Yes. Stock `HARV`/`CMIN` use `Mission_Deploy_Building` through refinery `DockUnload=yes`; stock `GAREFN/NAREFN` are refineries and are not `WeaponsFactory=yes`.

## 1. Overview

The unresolved `Mission_Deploy_Building` state-4 virtual call at `+0x200` is `UnitClass::ShouldIdle @ 0x00744270`, not the base always-true stub `0x004E0140` and not the InfantryClass override `0x00521B60`.

For the stock healthy `HARV`/`CMIN -> GAREFN/NAREFN` post-unload exit, this predicate should return true: state 4 has just cleared `Unit+0x6D1`, the current mission is `0x10`, the refinery contact is not a `WeaponsFactory=yes` building, and the docked miner is not on the movement-blocking branch. If it returned false, the engine would skip the state-4 direct `BREAK(0x03)` and `MissionClass::Commence` after already clearing the unload-active byte, so this predicate is load-bearing for cleanup/frame ordering.

## 2. Class Layout / Key Offsets

| Offset / field | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| vtable base `0x007F5C70` | UnitClass | primary UnitClass vtable | `ADDRESS_MAP.md`; PE byte read from `gamemd.exe` | Yes |
| vtable `+0x1E8` | UnitClass/MissionClass | `Queue_Mission @ 0x005B35E0` | `gamemd.exe` dword at `0x007F5E58` | Yes |
| vtable `+0x1EC` | UnitClass/MissionClass | `Commence @ 0x005B3570` | `gamemd.exe` dword at `0x007F5E5C` | Yes |
| vtable `+0x200` | UnitClass | `ShouldIdle @ 0x00744270`; readiness-to-commence predicate | `gamemd.exe` dword at `0x007F5E70`; decompile `0x00744270` | Yes |
| vtable `+0x23C` | UnitClass | mission `0x10`, `Mission_Deploy_Building @ 0x0073D630` | `gamemd.exe` dword at `0x007F5EAC` | Yes |
| `Unit+0xAC` | MissionClass | current mission; state-4 path is still mission `0x10` | `0x005B3060`; `0x0073D630` | Yes |
| `Unit+0xB4` | MissionClass | queued mission | `0x005B35E0`; `0x007442B9` | Yes |
| `Unit+0xB8` | MissionClass | queued/current mission start byte used by `ShouldIdle` movement block | `0x005B35E0`; `0x005B3570`; `0x0074431F` | Yes |
| `Unit+0x6D1` | UnitClass | unload-active/dock-render byte; state 4 clears it before `+0x200` | `0x0073E1F6`; `0x007442AB` | Yes |
| `Unit+0x6E1/+0x6E2` | UnitClass | hard blockers for `ShouldIdle` | `0x0074428F..0x007442A5` | Conditional |
| `Unit+0x350+0x18/+0x19` | UnitClass subobject | helper `0x004A51D0` requires both bytes zero | `0x00744329`; `0x004A51D0` | Yes |
| `Radio+0xE4/+0xE8` | RadioClass | contact vector data/count; `0x0065AD40(0)` returns contact slot 0 | `0x0065AD40`; state-4 contact scan `0x0065AE30` | Yes |
| `BuildingType+0x16BD` | BuildingTypeClass | `WeaponsFactory=yes`; special false branch in `ShouldIdle` | `0x0074435D`; `BIB_SYSTEM_GHIDRA_REPORT.md`; INI | Yes for WFs, No for stock refineries |

## 3. Core Logic

### 3.1 Concrete vtable target

The UnitClass vtable base is `0x007F5C70`. Reading the retail `gamemd.exe` PE image directly maps these entries:

| Slot | Vtable address | Concrete pointer |
|---|---:|---:|
| `+0x1E8` | `0x007F5E58` | `0x005B35E0` |
| `+0x1EC` | `0x007F5E5C` | `0x005B3570` |
| `+0x200` | `0x007F5E70` | `0x00744270` |
| `+0x23C` | `0x007F5EAC` | `0x0073D630` |

This resolves the state-4 call to `UnitClass::ShouldIdle @ 0x00744270`. Sibling docs that name base `0x004E0140` or Infantry `0x00521B60` are not valid for UnitClass state 4.

### 3.2 State-4 call order

For the normal non-Weeder stock refinery branch, `Mission_Deploy_Building` state 4 does this order:

1. Rediscover west-cell building.
2. If rediscovered building is a refinery and `building+0x57C` is live, return `1` before cleanup.
3. Clear `Unit+0x6D1 = 0` at `0x0073E1F6`.
4. If normal stock branch condition passes, call `Queue_Mission(10, 0)` at `0x0073E24F..0x0073E254`.
5. Call vtable `+0x200`, now resolved as `UnitClass::ShouldIdle`, at `0x0073E25E`.
6. If false, jump to `0x0073E289` and skip both contact scan/BREAK and `Commence`.
7. If true, call `PathType__Has_Valid_Steps @ 0x0065AE30`; if any contact exists, send `BREAK(0x03)` through vtable `+0x274` at `0x0073E275..0x0073E279`.
8. Call `Commence @ +0x1EC` at `0x0073E27F..0x0073E283`.
9. Return through the mission timer epilogue.

This means `ShouldIdle` is after the unload byte clear but before radio cleanup and mission promotion. It can affect frame ordering even though it does not decide whether `+0x6D1` clears.

### 3.3 `UnitClass::ShouldIdle` predicate

`UnitClass::ShouldIdle @ 0x00744270` returns false immediately if any of these are true:

1. Current mission `Unit+0xAC` is `6`.
2. Current mission `Unit+0xAC` is `0x15`.
3. Byte `Unit+0x6E1` is nonzero.
4. Byte `Unit+0x6E2` is nonzero.
5. Byte `Unit+0x6D1` is nonzero.

If queued mission `Unit+0xB4` is not `7`, it may also return false on a movement/readiness block:

1. Locomotor exists and vtable `+0x80` returns true.
2. Unit vtable `+0x1C8` returns a non-negative value.
3. Current mission is not `5`.
4. Current mission is not `1`, or archive target `Unit+0x2B4` is nonzero.
5. Byte `Unit+0xB8` is zero.

If the early blockers and movement block pass, it calls `FUN_004A51D0(Unit+0x350)`, which returns true only if bytes `+0x18` and `+0x19` of that subobject are both zero. If this helper returns false, `ShouldIdle` returns false.

The final branch is contact/building-sensitive:

- If contact slot 0 exists and is not a building, return true.
- If contact slot 0 is a building but `BuildingType+0x16BD WeaponsFactory` is false, return true.
- If contact slot 0 is a `WeaponsFactory=yes` building, return true only when queued mission is `2` or `7`; otherwise return false.
- If contact slot 0 is null, it looks up a building at the unit's own cell; a `WeaponsFactory=yes` building exactly one cell north blocks and returns false, otherwise return true.

### 3.4 Stock healthy HARV/CMIN state-4 result

For stock `HARV`/`CMIN` completing a healthy unload at stock `GAREFN/NAREFN`, static evidence supports `ShouldIdle == true`:

- Current mission is `0x10`, not `6` or `0x15`.
- State 4 clears `+0x6D1` before the predicate.
- Stock healthy path has no evidence of `+0x6E1/+0x6E2` being set.
- The docked miner should not be in the movement-readiness false branch; the mission is a stationary deploy/unload state.
- Contact slot 0, when present, is a stock refinery. Stock `GAREFN/NAREFN` have `Refinery=yes` and `DockUnload=yes`, but not `WeaponsFactory=yes`, so the contact branch returns true.
- If the contact slot has already been lost, the null-contact fallback only blocks a unit parked in a war-factory exit relation; the stock refinery pad is not a `WeaponsFactory=yes` building.

Therefore, for the scoped healthy stock refinery state-4 exit, `+0x200` does not normally delay cleanup. It does still need to be modeled as a real predicate for exact mechanism, because false returns skip `BREAK(0x03)` and `Commence`.

## 4. INI Keys

| Key | Stock YR value | Effect in this slice | Evidence |
|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | stock chrono miner refinery targets | `ini/rulesmd.ini:7361` |
| `[CMIN] Harvester` | `yes` | selects harvester branch | `ini/rulesmd.ini:7364` |
| `[CMIN] UnloadingClass` | `CMON` | render latch consumer, cleared by `+0x6D1=0` | `ini/rulesmd.ini:7384` |
| `[HARV] Dock` | `NAREFN,GAREFN` | stock war miner refinery targets | `ini/rulesmd.ini:8225` |
| `[HARV] Harvester` | `yes` | selects harvester branch | `ini/rulesmd.ini:8228` |
| `[HARV] UnloadingClass` | `HORV` | render latch consumer, cleared by `+0x6D1=0` | `ini/rulesmd.ini:8246` |
| `[GAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | stock receiver and one contact slot | `ini/rulesmd.ini:11726..11729` |
| `[NAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | stock receiver and one contact slot | `ini/rulesmd.ini:12519..12521` |
| `WeaponsFactory=` | absent/false for `GAREFN/NAREFN`; true for WFs | `ShouldIdle` special blocker uses `BuildingType+0x16BD`, not refinery flags | `0x0074435D`; `BIB_SYSTEM_GHIDRA_REPORT.md` |

## 5. Integration Points

| Function / point | Role | Verified details |
|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | owns state-4 exit | `+0x200` gate is after `+0x6D1` clear and before `BREAK`/`Commence` |
| `UnitClass::ShouldIdle @ 0x00744270` | vtable `+0x200` for UnitClass | real predicate; not always-true |
| `MissionClass::Queue_Mission @ 0x005B35E0` | writes queued mission, optionally calls `+0x200`/`+0x1EC` when force flag nonzero | state 4 calls with force `0`; no immediate internal commence |
| `MissionClass::Commence @ 0x005B3570` | promotes queued mission and resets mission timer fields | skipped if state-4 `ShouldIdle` returns false |
| `UnitClass::AI @ 0x007360C0` | same predicate is used as a general late/early commence gate | confirms `+0x200` is a mission readiness gate, not a refinery-only helper |
| `RadioClass` contact slot helper `0x0065AD40` | returns contact slot by index | `ShouldIdle` reads contact slot 0 |
| `PathType__Has_Valid_Steps @ 0x0065AE30` | contact-present scan in this radio context | called only after `ShouldIdle` true |

## 6. Current Rust Implementation Status

No Rust was edited.

Focused scan:

- `src/sim/miner/mod.rs:130..135` describes `Departing` as stock zero-link cleanup and direct return to SearchOre/Harvest scheduling.
- `src/sim/miner/miner_dock_sequence.rs:1156..1203` releases pad/contact, clears display override, clears mission-deploy delay/unload cluster, and enters `SearchOre`.
- `src/sim/miner/miner_dock.rs:124..130` `release_contact` removes the contact and clears contact-entered state without a `ShouldIdle`-like readiness predicate.
- `src/sim/miner/miner_dock.rs:365..391` has a test proving `release_contact` does not promote a waiter and does not clear pad occupancy.

Current Rust broadly matches the healthy stock outcome because it always performs state-4 cleanup. It does not model the byte-level `ShouldIdle` branch that can skip `BREAK(0x03)`/`Commence` after the unload byte has already cleared. That is acceptable only if the implementation surface remains scoped to healthy stock HARV/CMIN completion. It is drift for exact generalized UnitClass state-4 semantics.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| UnitClass vtable `+0x200` binding | verified | retail PE dword `0x007F5E70 -> 0x00744270` | none |
| `0x00744270` body | verified | decompile plus assembly contexts `0x00744270..0x0074446A` | friendly labels for `+0x6E1/+0x6E2` and `Unit+0x350` subobject bytes |
| State-4 ordering around `+0x200` | verified | decompile `0x0073D630`; assembly contexts `0x0073E23D..0x0073E289` | none |
| Healthy stock refinery result | verified static | `ShouldIdle` predicates plus stock INI and refinery non-WF flag | runtime breakpoint could confirm AL=1 at `0x0073E264` |
| False-return effect | verified | `0x0073E264..0x0073E289`; skips `0x0073E26A`, `0x0073E275`, `0x0073E283` | concrete retail scenario that triggers false is outside scope |
| Base `0x004E0140` and Infantry `0x00521B60` variants | verified negative for Unit state 4 | decompile both; Unit vtable PE dword | none |
| Rust `phase_departing` surface | touched-not-exhausted | `src/sim/miner/miner_dock_sequence.rs:1156` | exact predicate not implemented |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which concrete function backs UnitClass vtable +0x200? -> `UnitClass::ShouldIdle @ 0x00744270`.` (evidence: `gamemd.exe` PE dword `0x007F5E70 -> 0x00744270`; decompile `0x00744270`)
- `[RESOLVED] OQ-02 - Is it the base always-true stub? -> No; `0x004E0140` returns `1`, but UnitClass vtable +0x200 points to `0x00744270`.` (evidence: `0x004E0140`; `0x007F5E70`)
- `[RESOLVED] OQ-03 - Is it the InfantryClass `CanQueueMission_Now` override? -> No; Infantry docs point to `0x00521B60`, but UnitClass vtable uses `0x00744270`.` (evidence: `0x00521B60`; `0x007F5E70`)
- `[RESOLVED] OQ-04 - Where is the state-4 call relative to cleanup? -> After `+0x6D1=0` and `Queue_Mission(10,0)`, before contact scan, `BREAK(0x03)`, and `Commence`.` (evidence: `0x0073E1F6`, `0x0073E24F..0x0073E283`)
- `[RESOLVED] OQ-05 - What happens if it returns false? -> State 4 skips contact scan, direct BREAK, and Commence, then returns through the timer epilogue.` (evidence: `0x0073E264..0x0073E289`)
- `[RESOLVED] OQ-06 - Does stock healthy HARV/CMIN state 4 leave `+0x6D1` blocking the predicate? -> No; state 4 clears it before the call.` (evidence: `0x0073E1F6`; `0x007442AB`)
- `[RESOLVED] OQ-07 - Does the stock refinery contact hit the WeaponsFactory blocker? -> No; `ShouldIdle` blocks only `BuildingType+0x16BD WeaponsFactory`, while stock `GAREFN/NAREFN` are not WFs.` (evidence: `0x0074435D`; `ini/rulesmd.ini:11726..11729`, `12519..12521`; `BIB_SYSTEM_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-08 - Does `Queue_Mission(10,0)` internally call `ShouldIdle`? -> No; `Queue_Mission` only calls `+0x200` when its third argument is nonzero.` (evidence: `0x005B35E0`; state-4 caller `0x0073E24F..0x0073E254`)
- `[RESOLVED] OQ-09 - Is this path active in standard YR? -> Yes for stock harvesters unloading into stock refineries.` (evidence: stock INI lines listed in section 4; `0x0073D630`)
- `[RESOLVED] OQ-10 - Does Rust model this exact predicate? -> No focused scan found a `ShouldIdle` equivalent in miner departing cleanup.` (evidence: `src/sim/miner/miner_dock_sequence.rs:1156..1203`)
- `[DEFERRED] OQ-11 - Friendly semantic names for `Unit+0x6E1/+0x6E2` blockers` (category: `requires-different-system-context`; reason: this slice only needs their false-blocking effect; next-step-if-pursued: trace all writers/consumers of `Unit+0x6E1/+0x6E2`)
- `[DEFERRED] OQ-12 - Concrete runtime false-return scenario for a harvester state-4 edge` (category: `needs-runtime-debugger`; reason: static proof gives predicates, but a replay/debugger is needed to force and observe unusual flag/locomotor combinations; next-step-if-pursued: breakpoint at `0x0073E25E` and log AL plus fields)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| UnitClass state-4 vtable `+0x200` is `UnitClass::ShouldIdle @ 0x00744270`, a real predicate. | `0x007F5E70 -> 0x00744270`; decompile `0x00744270` | missing exact predicate | `src/sim/miner/miner_dock_sequence.rs::phase_departing`; future MissionClass/UnitClass scheduler | Keep healthy stock cleanup behavior, but model predicate before generalized radio BREAK/Commence if byte-level UnitClass mission parity is added. | Stock HARV finishes unload at GAREFN: predicate passes, cleanup sends direct break equivalent and returns to harvest/search. Proposed test: `war_miner_state4_shouldidle_passes_stock_refinery_exit` | Do not replace UnitClass `+0x200` with base `0x004E0140` or Infantry `0x00521B60`. |
| False `ShouldIdle` skips contact scan, direct `BREAK(0x03)`, and `Commence` after `+0x6D1` has already cleared. | `0x0073E1F6`; `0x0073E264..0x0073E289` | missing false branch | same as above; future radio/contact byte model | Preserve order if implementing edge cases: unload visual byte clears even when radio cleanup/commence is skipped. | Forced predicate-false fixture clears unloading display but preserves contact for later cleanup. Proposed test: `state4_shouldidle_false_clears_unload_but_skips_break` | Do not gate `+0x6D1` clear on `ShouldIdle`; the binary clears first. |
| Stock refinery contact does not trigger the `WeaponsFactory=yes` contact blocker. | `0x0074434B..0x00744383`; stock INI; `BuildingType+0x16BD` evidence | Rust does not represent this predicate, but healthy outcome matches | `src/sim/miner/miner_dock.rs`; `src/sim/miner/miner_dock_sequence.rs` | Keep stock refinery exit cleanup unconditional for healthy path unless predicate blockers are explicitly represented. | GAREFN/NAREFN contact at state 4 still releases contact and returns to SearchOre. Proposed test: `stock_refinery_contact_not_weaponsfactory_shouldidle_true` | Do not use `Refinery=yes` or `DockUnload=yes` as the blocker; the blocker is `WeaponsFactory=yes`. |
| `Queue_Mission(10,0)` does not internally commence; the explicit state-4 `ShouldIdle`/`Commence` pair owns promotion. | `0x005B35E0`; `0x0073E24F..0x0073E283` | Rust has phase enum rather than exact queue/current mission promotion | miner dock FSM, future MissionClass fields | If MissionClass fields are added, keep `Queue_Mission(...,0)` and `Commence` separate. | State-4 handoff with force flag `0` does not auto-promote inside queue helper. Proposed test: `queue_mission_force_zero_does_not_commence_until_state4_gate` | Do not collapse `Queue_Mission(10,0)` into immediate `Commence`. |

## Negative Facts / Do Not Do

- Do not label state-4 UnitClass `+0x200` as `ShouldScatter`; that stale label confuses it with vtable `+0x484` scatter/arrival helpers. Evidence: Unit vtable `+0x200 -> 0x00744270`; `+0x484` is a separate slot.
- Do not treat UnitClass `+0x200` as the base always-true stub `0x004E0140`. Evidence: retail vtable dword `0x007F5E70`.
- Do not treat UnitClass `+0x200` as InfantryClass `0x00521B60`. Evidence: Unit vtable dword and decompile of `0x00521B60` are different.
- Do not gate the state-4 unload byte clear on `ShouldIdle`; `+0x6D1` is cleared before the predicate. Evidence: `0x0073E1F6` before `0x0073E25E`.
- Do not implement stock refinery `ShouldIdle` failure using `Refinery=yes`, `DockUnload=yes`, or `NumberOfDocks`; the contact-special blocker is `WeaponsFactory=yes` (`BuildingType+0x16BD`). Evidence: `0x0074435D`.
- Do not assume `Queue_Mission(10,0)` calls `Commence`; the third argument is zero and the explicit state-4 `+0x200`/`+0x1EC` pair follows. Evidence: `0x005B3621..0x005B3641`, `0x0073E24F..0x0073E283`.

## Remaining Uncertainty

- Friendly semantic names for `Unit+0x6E1/+0x6E2` and `Unit+0x350` bytes remain out of scope. Their blocker behavior is verified, but this report does not claim their global lifecycle.
- A runtime debugger trace would be needed to demonstrate a concrete stock-like state-4 false-return edge. Static evidence proves the branch and why healthy stock HARV/CMIN should not take it.

## Stale Docs / Follow-up Docs

- `docs/research/UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`: replace `vtable+0x200 // ShouldScatter` with `vtable+0x200 // UnitClass::ShouldIdle / ready-to-commence predicate`.
- `docs/research/HARV_POST_UNLOAD_RADIO_0X08_FRAME_ORDER_RESWARM_20260528.md`: replace the deferred note "Full vtable +0x200 identity in state 4 remains ordered but not semantically named" with "Resolved by `UNITCLASS_STATE4_VTABLE_0X200_IDENTITY_RESWARM_20260528.md`: UnitClass `+0x200` is `UnitClass::ShouldIdle @ 0x00744270`; healthy stock refinery state-4 should pass it."

## Sources

- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`; `UnitClass::ShouldIdle @ 0x00744270`; `MissionClass::Queue_Mission @ 0x005B35E0`; `MissionClass::Commence @ 0x005B3570`; `UnitClass::AI @ 0x007360C0`; `FUN_004A51D0`; `FUN_0065AD40`; `PathType__Has_Valid_Steps @ 0x0065AE30`; base stub `0x004E0140`; Infantry override `0x00521B60`.
- Ghidra assembly contexts: `0x0073E23D..0x0073E289`; `0x005B3629..0x005B3641`; `0x00744270..0x0074446A`; `0x00736461..0x00736473`; `0x0065AE30..0x0065AE54`.
- Retail binary PE read: `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, vtable dwords at `0x007F5E58`, `0x007F5E5C`, `0x007F5E70`, `0x007F5EAC`.
- Existing docs: `ADDRESS_MAP.md`; `HARV_POST_UNLOAD_RADIO_0X08_FRAME_ORDER_RESWARM_20260528.md`; `miner/CMIN_STATE2_CLOSE_FAR_RETURN_TO_MISSION_ENTER_DISPATCH_GHIDRA_REPORT.md`; `BIB_SYSTEM_GHIDRA_REPORT.md`; `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`.
- Rust scanned: `src/sim/miner/mod.rs`; `src/sim/miner/miner_dock_sequence.rs`; `src/sim/miner/miner_dock.rs`.

**Status:** COMPLETE for the bounded identity, predicate semantics, healthy stock result, and state-4 cleanup-order effect.
