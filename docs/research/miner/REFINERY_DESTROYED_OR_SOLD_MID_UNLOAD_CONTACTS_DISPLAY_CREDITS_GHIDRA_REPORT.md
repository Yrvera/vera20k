# Refinery Destroyed/Sold Mid-Unload Contacts, Display, Credits - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x0073CEC0`, `0x00737430`, `0x00442230`, `0x00449C30`, `0x004593A0`, `0x005B35E0`, `0x005B3570`, `0x0047C520`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard YR stock `HARV`/`CMIN` unloading at `GAREFN`/`NAREFN` when the refinery is destroyed or sold during active unload, including contact cleanup, mission/substate, storage/credit cutoff, `UnloadingClass` display flag handling, stale visual risk, and queued/waiting miner handling.
**Non-Scope:** healthy unload cadence, healthy two-miner handoff, normal post-unload Force_Track visuals, slave miner/Yuri deploy-miner path, runtime debugger frame capture.
**Confidence:** High for static branch/order, storage/credit cutoff, contact messages, and display gates; Medium for exact rendered stale-frame count because no live frame capture was performed.
**Active in YR:** Yes. `rulesmd.ini` stock `[HARV]`/`[CMIN]` have `Harvester=yes`, `Dock=NAREFN,GAREFN`, `Storage=40/20`, `UnloadingClass=HORV/CMON`; `[GAREFN]`/`[NAREFN]` have `Refinery=yes`, `NumberOfDocks=1`, `Storage=200`.

## Working Notes Gate

- Target question: exact standard YR behavior when a stock refinery is destroyed/sold while `HARV`/`CMIN` is mid-unload: contacts, mission/substate, storage/credits, `+0x6D1` display, stale render, and queued/waiting miners.
- Non-goals: healthy unload timing, healthy two-miner handoff, and Force_Track post-unload visuals except as comparison.
- Evidence needed to mark COMPLETE: decompile plus assembly context for destroy/sell callers, `UndockUnit`, state-3 missing-building branch, queue/commence, `DrawExtras`, and radio `0x17`; INI proof for stock YR activation; Rust scan for affected surfaces.
- Stop conditions: stop at stock refinery-loss event; record runtime-frame uncertainty instead of mutating Ghidra or expanding into unrelated dock systems.

## 1. Overview

There are two distinct refinery-loss paths. If a real reciprocal `building+0x2E4` link exists, sell/death first call `BuildingClass::UndockUnit`, which clears the bidirectional dock pointer and does not credit or drain cargo. The common stock zero-link state-3 unload path instead rediscovers the adjacent refinery every pass; if lookup fails, it sends optional radio `3`, queues `Mission_Harvest`, and skips the entire storage/credit block.

The display result is branch-specific. Radio `0x17` clears `unit+0x6D1` immediately when delivered to a harvester/weeder, but the state-3 null-refinery branch and `Queue_Mission`/`Commence` do not clear `+0x6D1`; `DrawExtras` still gates `UnloadingClass` only on harvester type, `+0x6D1`, and `Type+0x6B8`.

## 2. Class Layout / Key Offsets

| Field | Owner | Meaning in this slice | Evidence |
|---|---|---|---|
| `+0x2E4` / index `0xB9` | building/unit | Reciprocal dock-link pointer when present | `UndockUnit @ 0x004593A0`; sell/death callers |
| `+0x6D1` | unit byte | Unload-active display flag consumed by `DrawExtras` | set `0x0073DFDA`, normal clear `0x0073E1F6`, radio clear `0x00737AC9` |
| `+0x6C4` | unit | Current `TechnoType/UnitType` pointer temporarily swapped for draw | `0x0073D29C..0x0073D3A2` |
| `+0x6B8` | type | `UnloadingClass` resolved `UnitTypeClass*` | prior parse evidence; draw gate `0x0073D2BA` |
| `+0xF8` / index `0x3E` | unit | Dump-rate accumulator | drain gate `0x0073E355..0x0073E374` |
| `+0xAC/+0xB4/+0xB8/+0xBC` | mission object | current mission, queued mission, queued flag, substate | `Queue_Mission @ 0x005B35E0`, `Commence @ 0x005B3570` |

## 3. Core Logic

| Finding | Active in YR | Evidence | Confidence |
|---|---|---|---|
| Destroyed/sold linked refinery calls `UndockUnit` only when `building+0x2E4 != 0`. | Conditional | death case checks `+0x2E4` then calls `0x004593A0` at `0x004424A2..0x004424EA`; sell state 0 checks `+0x2E4` and calls at `0x0044AAA4..0x0044AAB0` | High |
| `UndockUnit` clears unit then building `+0x2E4`, sends radio `3`, and contains no storage/credit calls. | Conditional | decompile `0x004593A0`; assembly `0x00459450..0x00459462` from prior report; this pass re-decompiled body | High |
| `UndockUnit` itself does not clear `unit+0x6D1`. | Conditional | decompile `0x004593A0` has locomotor stop/head-to/speed, link clears, radio `3`; no `+0x6D1` write | High |
| Sell broadcasts radio `0x17` after the linked-undock check and before clearing building anim slots. | Yes for sell state 0 | `0x0044AB5A MOV vtable`, `0x0044AB5D PUSH 0x17`, `0x0044AB61 field_0x6DD=0`, `0x0044AB68 CALL [vtable+0x280]` | High |
| Death-result damage sends radio `0x17` to collected destination/contact units outside the close-damage case; the linked `+0x2E4` unit is removed from that list before `UndockUnit`. | Conditional | list removal around `0x004424A8..0x004424E8`; later loop sends `0x17` at `0x0044259D..0x004425A4` and clears target field at `0x004425AA` | High |
| Radio `0x17` clears `+0x6D1` immediately if current type is harvester/weeder and `+0x6D1 != 0`, then scatters and sets mission Harvest. | Conditional on receiving radio `0x17` | `UnitClass::Receive_Radio @ 0x00737430`; assembly `0x00737A98..0x00737AF6`, clear at `0x00737AC9`, mission `10` at `0x00737ADE` | High |
| State-3 zero-link missing-refinery branch cannot drain cargo or credit money. | Yes | `Look_up_building_in_cell` at `0x0073E306`; null branch `0x0073E311..0x0073E350`; drain starts only on non-null branch at `0x0073E355+` | High |
| Missing-refinery abort order is optional radio `3`, then `Queue_Mission(10,1)`, then mission-timer/random return. | Yes | assembly `0x0073E313..0x0073E350`; `PUSH 3` before vtable `+0x274`, then `PUSH 1`, `PUSH 0xA`, vtable `+0x1E8` | High |
| Missing-refinery branch, `Queue_Mission`, and `Commence` do not clear `+0x6D1`. | Yes | no `+0x6D1` write in `0x0073E306..0x0073E350`; `0x005B35E0` writes queued mission fields; `0x005B3570` writes current mission/substate/timers only | High |
| `DrawExtras` will still choose `UnloadingClass` while `Harvester=yes`, `unit+0x6D1 != 0`, and `Type+0x6B8 != 0`; no mission/substate gate is present. | Yes | assembly `0x0073D29C..0x0073D2C4` swap; restore original type at `0x0073D3A2` | High |
| Full miners re-entering `Mission_Harvest` after refinery loss go back to return/refinery selection; partial miners can resume ore-search logic first. | Yes | `Mission_Harvest @ 0x0073E5E0` state 0 full check transitions to substate 2; partial path falls through search/harvest logic | High |
| Waiting/queued miners are aborted, not promoted, when the refinery itself is sold/destroyed. | Yes for loss event | sell broadcasts `0x17`; death loop sends `0x17` or close-damage side effect; no `0x0E`/`0x15` accept/promote path in sell/death loss code | High |

## 4. INI Keys

| Key | Stock YR value | Effect in this slice | Evidence |
|---|---|---|---|
| `[HARV] Harvester` | `yes` | Enables harvester/unload visual logic | `ini/rulesmd.ini:8228`; draw gate `Type+0xE0E` |
| `[HARV] Storage` | `40` | Cargo fullness and drain source | `ini/rulesmd.ini:8236` |
| `[HARV] UnloadingClass` | `HORV` | Alternate draw type while `+0x6D1` set | `ini/rulesmd.ini:8246`; draw gate `Type+0x6B8` |
| `[CMIN] Harvester` | `yes` | Enables chrono miner unload path | `ini/rulesmd.ini:7364` |
| `[CMIN] Storage` | `20` | Cargo fullness and drain source | `ini/rulesmd.ini:7374` |
| `[CMIN] UnloadingClass` | `CMON` | Alternate draw type while `+0x6D1` set | `ini/rulesmd.ini:7384` |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | Valid refinery target | `ini/rulesmd.ini:11727`, `12520` |
| `[GAREFN]/[NAREFN] NumberOfDocks` | `1` | Stock single dock capacity | `ini/rulesmd.ini:11729`, `12521` |
| `[GAREFN]/[NAREFN] Storage` | `200` | Building storage, not a fallback credit source after null lookup | `ini/rulesmd.ini:11744`, `12538` |

## 5. Integration Points

- `BuildingClass::Sell @ 0x00449C30`: state 0 linked interrupt, then broadcast radio `0x17`, then removal/refund work.
- `BuildingClass::ReceiveDamage @ 0x00442230`: death result case removes linked unit from local contact list, calls `UndockUnit`, then handles other contact/destination units.
- `BuildingClass::UndockUnit @ 0x004593A0`: linked interrupt helper only; no cargo/credit/display cleanup.
- `UnitClass::Mission_Deploy_Building @ 0x0073D630`: zero-link stock unload FSM; state 3 performs refinery rediscovery and aborts on null lookup.
- `UnitClass::Receive_Radio @ 0x00737430`: radio `0x17` is the proven immediate `+0x6D1` clear path.
- `UnitClass::DrawExtras @ 0x0073CEC0`: render-time `UnloadingClass` swap and restore.

## 6. Current Rust Implementation Status

Rust scan only, no edits:

- `src/sim/miner/miner_dock_sequence.rs:317` `resolve_refinery_cells` rejects missing, `dying`, and zero-health refineries.
- `src/sim/miner/miner_dock_sequence.rs:471` `abort_invalid_refinery` clears reservation/contact, movement/facing/track state, unload timers, and `display_type_override`.
- `src/sim/miner/miner_dock_sequence.rs:791` `phase_unloading` awards credits only after a live `ref_sid` has survived phase dispatch.
- `src/sim/miner/miner_dock_sequence.rs:400` `interrupt_refinery_docked_miners` approximates linked sell/removal cleanup.
- `src/sim/production/production_sell.rs:459` calls `interrupt_refinery_docked_miners` before removing the building.
- `src/sim/miner/miner_tests.rs:4409` and `:4470` cover full refinery-loss return and no-credit/no-stuck-visual behavior.

Current Rust intentionally clears `display_type_override` on invalid-refinery abort. Static gamemd evidence does not show a `+0x6D1` clear in the state-3 null-refinery branch; keep that as a deliberate implementation choice until a runtime capture proves exact stock visible restoration.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Sell linked-undock check | verified | `0x0044AAA4..0x0044AAB0` | none |
| Sell radio `0x17` broadcast | verified | `0x0044AB5A..0x0044AB68` | exact recipient list inside generic transmit helper not reopened |
| Damage death linked-undock order | verified | `0x004424A8..0x004424EA` | exact death animation frame ordering requires runtime |
| Damage contact/radio `0x17` loop | verified | `0x0044259D..0x004425AA` | close-damage side effects for units within `0x100` out of scope |
| `UndockUnit` cargo/display behavior | verified | `0x004593A0` | none |
| State-3 missing-building branch | verified | `0x0073E306..0x0073E350` | runtime stale-frame count |
| `Look_up_building_in_cell` null condition | verified | `0x0047C520` scans cell object list for `WhatAmI()==6` | none |
| `Queue_Mission`/`Commence` writes | verified | `0x005B35E0`, `0x005B3570` | none for `+0x6D1` |
| `DrawExtras` unloading type gate | verified | `0x0073D29C..0x0073D3A2` | runtime frame capture |
| Current Rust surfaces | touched-not-exhausted | source scan paths above | focused tests not run in this research pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the target active in standard YR? -> Yes, stock HARV/CMIN and GAREFN/NAREFN carry the needed INI gates.` (evidence: `ini/rulesmd.ini` keys above; `0x0073D630`)
- `[RESOLVED] OQ-02 - Does sell call `UndockUnit` before removal when linked? -> Yes, guarded by `building+0x2E4 != 0`.` (evidence: `0x0044AAA4..0x0044AAB0`)
- `[RESOLVED] OQ-03 - Does death call `UndockUnit` before teardown when linked? -> Yes in damage result case 4.` (evidence: `0x004424A8..0x004424EA`)
- `[RESOLVED] OQ-04 - Does `UndockUnit` drain or credit cargo? -> No.` (evidence: `0x004593A0`)
- `[RESOLVED] OQ-05 - Does `UndockUnit` clear `+0x6D1`? -> No static write in the helper.` (evidence: `0x004593A0`)
- `[RESOLVED] OQ-06 - What clears `+0x6D1` immediately? -> Unit radio `0x17`, when delivered to a harvester/weeder with flag set.` (evidence: `0x00737A98..0x00737AF6`)
- `[RESOLVED] OQ-07 - Does state-3 missing-building abort drain or credit? -> No, null lookup skips all storage/credit calls.` (evidence: `0x0073E306..0x0073E350`)
- `[RESOLVED] OQ-08 - What mission/substate follows state-3 null lookup? -> optional radio `3`, then queued Harvest `10` with immediate commence.` (evidence: `0x0073E313..0x0073E350`; `0x005B35E0`; `0x005B3570`)
- `[RESOLVED] OQ-09 - Does that queue/commence clear display? -> No, neither function writes `+0x6D1`.` (evidence: `0x005B35E0`, `0x005B3570`)
- `[RESOLVED] OQ-10 - Are waiting miners promoted when the refinery is sold/destroyed? -> No promotion path was found; sell/death use abort/contact-break radio paths, not accept/handoff.` (evidence: `0x0044AB5A..0x0044AB68`, `0x0044259D..0x004425AA`)
- `[DEFERRED] OQ-11 - Exact count of rendered stale `HORV`/`CMON` frames after zero-link missing-refinery abort.` (category: `needs-runtime-debugger`; reason: static code proves no immediate clear but not render-loop frame count; next-step-if-pursued: capture consecutive frames while selling/killing refinery during state-3 unload)
- `[DEFERRED] OQ-12 - Exact same-tick ordering between death animation map removal and the next miner mission dispatch.` (category: `needs-runtime-debugger`; reason: static caller order is known, tick/frame interleaving is not captured; next-step-if-pursued: live trace `ReceiveDamage -> UndockUnit -> unit AI tick`)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `UnitClass::DrawExtras @ 0x0073CEC0` | `Type+0xE0E != 0`, `unit+0x6D1 != 0`, `Type+0x6B8 != 0` at `0x0073D2A6..0x0073D2C4` | `HORV` or `CMON` unit type | normal unit draw anchor | normal unit body draw path | yes while flag remains set | unload visual override |
| 2 | `UnitClass::DrawExtras @ 0x0073CEC0` | original type saved at `0x0073D29C`, restored at `0x0073D3A2` | `HARV` or `CMIN` gameplay type | same | same | yes | restore real type after draw |

Asset role matrix:

| Asset/type | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `HARV` | yes | yes when `+0x6D1 == 0` or no `HORV` | normal miner | content | no | no | no | no | rules/art INI; draw gate |
| `HORV` | yes | yes while `HARV +0x6D1` gate passes | possible stale after null abort | content | no | no | no | no | `UnloadingClass=HORV`; `0x0073D2A6..0x0073D2C4` |
| `CMIN` | yes | yes when `+0x6D1 == 0` or no `CMON` | normal miner | content | no | no | no | no | rules/art INI; draw gate |
| `CMON` | yes | yes while `CMIN +0x6D1` gate passes | possible stale after null abort | content | no | no | no | no | `UnloadingClass=CMON`; `0x0073D2A6..0x0073D2C4` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Missing/destroyed zero-link refinery during state-3 unload skips all storage/credit calls and queues Harvest; full cargo returns to refinery selection. | `0x0073E306..0x0073E350`; `0x0073E5E0` | mostly aligned; tests exist | `miner_dock_sequence::resolve_refinery_cells`, `abort_invalid_refinery`, `phase_unloading`; `miner_system::handle_return` | Abort before any slot drain when refinery lookup is missing/dying/zero-health; preserve cargo and credits. | `war_miner_mid_unload_refinery_destroyed_preserves_cargo_no_credit_returns_to_refinery` | Do not credit the removed refinery owner or clear cargo after null lookup. |
| Sold/destroyed linked refinery clears dock/contact state through interrupt/radio, not through normal completion promotion. | `0x0044AAA4..0x0044AB68`; `0x004424A8..0x004425AA`; `0x004593A0` | partial model present | `interrupt_refinery_docked_miners`, `RefineryDockContacts`, sell/destruction hooks | Cancel on-pad and waiting contacts; do not promote another miner into a refinery that is being removed. | `war_miner_waiting_on_destroyed_refinery_aborts_not_promoted` | Do not run healthy two-miner handoff after sell/death. |
| Static binary does not clear `+0x6D1` on state-3 null-refinery abort; radio `0x17` does clear it immediately when delivered. | `0x0073E306..0x0073E350`; `0x005B35E0`; `0x005B3570`; `0x00737AC9`; `0x0073D29C..0x0073D3A2` | Rust currently clears `display_type_override` immediately in `abort_invalid_refinery` | `display_type_override`, `abort_invalid_refinery`, render selection | Keep the current immediate clear only as an intentional/runtime-verified choice, not as a static-binary claim. | `war_miner_mid_unload_refinery_loss_display_restore_matches_runtime_capture` | Do not assert gamemd state-3 null abort clears `HORV`/`CMON`; it does not in static code. |

Stale Docs / Follow-up Docs:

- `docs/research/miner/HARV_DESTROYED_REFINERY_UNLOAD_ABORT_GHIDRA_REPORT.md`: replace "unload visual must end on abort" with "Static binary evidence shows the zero-link state-3 null-refinery branch does not clear `unit+0x6D1`; radio `0x17` clears it when delivered, and Rust's immediate `display_type_override` clear requires runtime visual confirmation as a deliberate parity choice."
- `docs/research/miner/MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`: keep the cargo/no-credit/order claims, but replace any acceptance wording that assumes immediate visual restoration with "exact stale `HORV`/`CMON` rendered frame count remains runtime-only; static code proves no clear in the state-3 null branch."
- `docs/research/miner/HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md`: replace broad "lost refinery immediately picks another refinery" wording with "after loss, full cargo enters Harvest state 0 full-check and then return/refinery selection; partial cargo can resume ore-search logic first."
- `docs/research/miner/HARV_UNLOADING_CLASS_DISPLAY_TIMING_GHIDRA_REPORT.md`: no replacement needed; this report confirms its static `+0x6D1` null-branch caveat.

## Negative Facts / Do Not Do

- Do not award credits or drain storage after `Look_up_building_in_cell` returns null in state 3.
- Do not promote queued/waiting miners into a refinery that is being sold or destroyed.
- Do not treat `UndockUnit` as normal zero-link completion or as proof of Force_Track behavior for this loss branch.
- Do not claim the state-3 missing-refinery branch sends radio `0x07`, `0x19`, or `0x15`; verified branch sends optional radio `3` and queues Harvest.
- Do not claim static gamemd clears `+0x6D1` on state-3 null-refinery abort; only radio `0x17` and normal state 4 are proven clear paths in this slice.

## Remaining Uncertainty

- Exact number of player-visible stale `HORV`/`CMON` frames after zero-link state-3 null-refinery abort needs runtime debugger or screenshot capture.
- Exact same-tick ordering between combat death animation/removal and the next unit mission dispatch needs live trace if a future bug depends on one-frame visibility.

## Sources

- Ghidra read-only decompile: `0x0073D630`, `0x0073CEC0`, `0x00737430`, `0x00442230`, `0x00449C30`, `0x004593A0`, `0x005B35E0`, `0x005B3570`, `0x0047C520`, `0x0073E5E0`.
- Ghidra read-only assembly context: `0x0073E306`, `0x0073E311`, `0x0073E33D`, `0x0073E1F6`, `0x0073DFDA`, `0x0073E355`, `0x0073D29C`, `0x0073D2A6`, `0x0073D2B0`, `0x0073D2BA`, `0x0073D2C4`, `0x0073D3A2`, `0x00737A98`, `0x00737AC9`, `0x004424A8`, `0x004424EA`, `0x0044259D`, `0x004425AA`, `0x0044AAA4`, `0x0044AAB0`, `0x0044AB5A`, `0x0044AB68`.
- Prior docs checked: `HARV_DESTROYED_REFINERY_UNLOAD_ABORT_GHIDRA_REPORT.md`, `MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`, `HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md`, `HARV_UNLOADING_CLASS_DISPLAY_TIMING_GHIDRA_REPORT.md`, `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan only: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/production/production_sell.rs`, `src/sim/miner/miner_tests.rs`, `src/app_instances/units.rs`.
