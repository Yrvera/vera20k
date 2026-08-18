# HARV / GAREFN West-Cell Unload Identity Trace

**Scenario:** Stock YR `HARV` unloads one Ore storage slot at stock `GAREFN`.
**Date:** 2026-05-27
**Slot:** trace-swarm production slot 1.
**Scope:** `UnitClass::Mission_Deploy_Building` state 3 unload identity and effects: west-cell building lookup, owner credits, purifier context, `BaleDepositEvent` building identity, and Rust `reserved_refinery` as dock/contact bookkeeping only.
**Out of scope:** radio `0x16`, far-return fallback search, teleport lifecycle, null west-cell branch, state-4 non-refinery cases, live render-frame latch timing.

## Verdict Tally

PASS: 10 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Sources Read

- `docs/research/miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`
- `docs/research/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`
- `docs/research/miner/EMPTY_SLOT_UNLOAD_GATE_TO_STATE4_RELEASE_TIMING_GHIDRA_REPORT.md`
- `docs/research/ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md`
- Current Rust: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/components.rs`, `src/sim/miner/miner_tests.rs`
- Stock INI: `ini/rulesmd.ini`, `ini/artmd.ini`

No live Ghidra instance was available (`list_instances` returned none), so no fresh Ghidra query was made. gamemd evidence below comes from existing verified Ghidra reports that directly decompiled `UnitClass::Mission_Deploy_Building @ 0x0073D630` and related helpers. No Ghidra mutation, Rust edit, INI edit, or non-trace doc edit was performed.

Active-YR status: verified for stock `HARV` because `[HARV]` has `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=40`; stock `[GAREFN]` has `DockUnload=yes` and `Refinery=yes`. This is the Allied/Soviet harvester unload path in `UnitClass::Mission_Deploy_Building`, not dormant TS legacy and not Yuri slave-miner deposit.

## Pipeline

`HARV already in unload state 3 at accepted pad cell` -> `current cell lookup` -> `lookup cell = miner cell + (-1,0)` -> `first BuildingClass in lookup cell object list` -> `whole first non-empty ore slot drains` -> `building owner receives credits` -> `same owner supplies real/AI purifier context` -> `one deposit event uses rediscovered building ID` -> `reserved_refinery remains release/contact bookkeeping`.

## Concrete Values

- Rust concrete regression setup: miner stable ID `1` at cell `(13,11)`; `reserved_refinery=Some(2)`; west-cell stock `GAREFN` stable ID `3` at `(12,11)`.
- Lookup cell: `(13 - 1, 11) = (12,11)`.
- Cargo slot: one Ore slot represented by one bale with value `100` credits in the focused Rust test.
- No purifier in this concrete setup: `effective_purifier_count = 0`; bonus `100 * 0 * 25 / 100 = 0`.
- Expected spendable credit delta: `+100` to the rediscovered building owner.
- Expected deposit events: `1`, with `building_id = 3`.

## Stage Results

| # | Stage | gamemd output for this scenario | Current Rust output | Verdict |
|---:|---|---|---|---|
| 1 | Stock data / active path | `HARV` reaches standard harvester state-3 unload; `GAREFN` accepts `DockUnload=yes` and is `Refinery=yes`. | Rules objects expose harvester/refinery flags; miner dock FSM handles `HARV` unload at `GAREFN`. | PASS |
| 2 | State-3 authority lookup | State 3 uses `current_cell + DAT_0089F6A0/DAT_0089F6A2`, verified as the adjacent refinery lookup in active YR; helper scans the cell object list and returns the first building. | `mission_deploy_unload_building` computes `lookup_rx = miner.position.rx - 1`, `lookup_ry = miner.position.ry`, then returns the first live structure from that occupancy layer. | PASS |
| 3 | Lookup concrete cell | For a miner on the accepted stock pad, the lookup cell is west of the miner and contains the refinery object. | Miner at `(13,11)` looks up `(12,11)` and finds stable ID `3`. | PASS |
| 4 | Reserved refinery independence | The zero-link state-3 unload path does not depend on `unit+0x2E4`; the rediscovered building drives the unload effects. | In the positive-drain branch, `reserved_refinery` is not used for credit owner, purifier owner, or event identity; `ref_sid` is only carried for abort/depart cleanup paths. | PASS |
| 5 | Slot selection and grain | `FindFirstNonEmptySlot` returns the first occupied storage slot; `GetAmount` and `RemoveAmount` drain the whole slot on the dump gate. | `SLOT_ORDER=[Ore,Gem]`; all cargo bales of the selected resource type are retained out in one atomic drain. | PASS |
| 6 | Credit owner identity | gamemd reads owner from the rediscovered `BuildingClass` via `GetOwner`; miner controller is not the recipient. | Rust resolves owner from `unload_building_id` and adds `slot_value` to that owner. Focused test shows `Americans` unchanged and west-cell `Germans` +100 when `reserved_refinery` points elsewhere. | PASS |
| 7 | Purifier context identity | gamemd uses the same building-owner context for real purifier count and AI virtual purifiers before calling credit add. | `effective_purifier_count(sim, rules, &refinery_owner)` uses the rediscovered building owner string. | PASS |
| 8 | Credit arithmetic in concrete setup | One 100-credit ore slot, no purifier: base `+100`, bonus `0`, total `+100`. | Focused test asserts west-cell owner delta `+100` and reserved/refinery owner delta `0`. | PASS |
| 9 | BaleDepositEvent identity | State-3 visible side effects belong to the rediscovered refinery object for this dump. | Rust emits exactly one `BaleDepositEvent { building_id: unload_building_id, tick }`; focused test asserts `building_id == 3`. | PASS |
| 10 | Bookkeeping cleanup role | gamemd's state-3 positive drain does not use the release helper branch; dock/contact teardown is separate from deposit identity. | Rust keeps `reserved_refinery` for contact/reservation cleanup in later or abort paths; focused mismatch test confirms cleanup can release `2` while credit/event identity remains `3`. | PASS |
| 11 | Exact object-list ordering | Ghidra report verifies "first building in cell object list"; it does not provide a live runtime object-list ordering sample for this exact stock HARV/GAREFN test. | Rust uses occupancy iteration order for the selected movement layer. Exact equivalence of insertion/order semantics against `CellClass+0xE4` for all stacked objects was not recomputed here. | UNCHECKED |
| 12 | Live frame timing of event consumption | gamemd fires state-3 side effects inside the dump-gate mission call. | Rust emits the event during miner tick; renderer consumes later in app tick. Exact same-frame render/audio parity was not computed for this identity-only slot. | UNCHECKED |

## Current Rust Evidence

- `src/sim/miner/miner_dock_sequence.rs:432` defines `mission_deploy_unload_building`.
- `src/sim/miner/miner_dock_sequence.rs:437` computes `lookup_rx = miner.position.rx - 1`; `:438` keeps the same Y cell.
- `src/sim/miner/miner_dock_sequence.rs:442` reads the lookup cell occupancy; `:445` returns the first live structure.
- `src/sim/miner/miner_dock_sequence.rs:1046` requires the rediscovered building before a positive drain.
- `src/sim/miner/miner_dock_sequence.rs:1068` resolves `refinery_owner` from `unload_building_id`.
- `src/sim/miner/miner_dock_sequence.rs:1075` credits that owner; `:1083` uses that owner for `effective_purifier_count`.
- `src/sim/miner/miner_dock_sequence.rs:1098` emits `BaleDepositEvent` with `building_id: unload_building_id`.
- `src/sim/miner/miner_tests.rs:4632` covers west-cell building identity versus `reserved_refinery`.
- `src/sim/miner/miner_tests.rs:4759` covers reserved-refinery cleanup versus unload credit/event identity.

## Failures

None confirmed for this concrete stock `HARV`/`GAREFN` state-3 unload identity slice.

## Adjacent Findings

- Exact `CellClass+0xE4` object-list ordering versus Rust occupancy-layer iteration remains `UNCHECKED` for unusual stacked-object cases. The concrete stock one-refinery cell is covered.
- Exact same-render-frame consumption of the deposit event is outside this slot and remains `UNCHECKED`.
- Null west-cell lookup and state-4 non-refinery/refinery guard behavior are assigned to other trace-swarm slots and were not traced here.

## Status

COMPLETE for slot 1. No confirmed FAIL or NOT-IMPLEMENTED finding in the requested stock `HARV`/`GAREFN` state-3 west-cell unload identity scenario.
