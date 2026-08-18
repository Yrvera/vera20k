# Refinery FreeUnit Blocked-Primary Fallback and Refund Trace

**Date:** 2026-07-28  
**Rust target:** feature worktree commit `799515ca9867ac189e7c6ea9b03d0d93938d5c6b`  
**Native target:** active standard-YR `gamemd.exe` open in Ghidra (`/gamemd.exe`)  
**Verdict:** **FAIL** — a true third-party dynamic blocker cannot reject Rust's
primary spawn, so Rust overlaps the blocker and never performs native fallback,
cleanup, or refund behavior.

## Exact scenario and stop boundary

Use a stock Allied `GAREFN` at north-west cell `(20,20)`, owned by an ordinary
Americans house, completing construction with no cost modifier. Stock merged data
gives `Refinery=yes`, `FreeUnit=CMIN`, a `4x3` foundation, and `CMIN Cost=1400`
(`ini/rulesmd.ini:11722-11740`, `ini/artmd.ini` `[GAREFN]`,
`ini/rulesmd.ini:7351-7376`). A third-party dynamic mobile object already occupies
the native primary cell `(22,22)`. The bounded failure variant additionally makes
both native nearby-placement attempts return no usable cell or makes their
`Unlimbo` calls fail.

This is not a trace of normal open-ground completion timing or simultaneous
completion identity/order. Those are adjacent findings owned by other trace slots.

## Evidence and active-YR identity

- Live `batch_decompile(0x00445F80,0x00449AD4)` verified the completion function
  body and its active construction-mission caller. At `0x00449AD4`,
  `Mission_Construction` calls vtable slot `+0x4DC` after construction completion.
- `docs/research/BUILDINGCLASS_VTABLE_COMPLETE.md:371` binds BuildingClass slot
  `+0x4DC` to `0x00445F80`; the live body reads BuildingType `+0xEA0`, constructs a
  UnitClass, places it, assigns mission 10, refunds, and cleans up on failure.
- `docs/research/miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md:124-182` was
  checked against the live body and assembly. It identifies the two raw nearby
  option groups without inventing semantic names.
- Stock YR `GAREFN FreeUnit=CMIN` makes this branch active in ordinary YR play.
  It is not a dormant TS-legacy branch.

## Pipeline

`GAREFN buildup completes` → `OnConstructionComplete resolves CMIN` →
`allocate CMIN` → `primary Unlimbo (22,22), facing C0` →
`on rejection: nearby pass 1, then pass 2, facing A0` →
`success: queue/commence Harvest(10)` **or**
`total failure: refund 1400, UnInit CMIN` → `occupancy/render/wallet result`

## Entry-point coverage for this trigger

1. Native stock construction: `Mission_Construction 0x00449AD4` dispatches
   BuildingClass vtable `+0x4DC` to `0x00445F80`.
2. Rust stock construction: `World::run_late_region`
   (`src/sim/world/mod.rs:1985-1995`) takes `tick_building_up()` completions and
   calls `spawn_completed_refinery_free_units`.

The Rust function has one production caller (`rg
spawn_completed_refinery_free_units`). Map/editor and ownership-change native
entry variants are outside this concrete stock-construction scenario.

## Stage trace

| # | Stage | Native computed result | Rust at `799515ca` | Verdict |
|---:|---|---|---|---|
| 1 | Completion trigger and data | Active completion callback resolves `GAREFN +0xEA0 -> CMIN`; stock cost is 1400. | Completion list resolves `GAREFN -> CMIN` through `RuleSet::refinery_free_unit` (`ruleset.rs:2345-2354`). | **PASS** |
| 2 | Primary coordinate and facing | 4x3 center cell is NW `+(2,1)`; south table entry makes primary NW `+(2,2)` = `(22,22)`; facing `0xC0` at `0x00446BA5-0x00446BD2`. | `primary_free_unit_cell` computes `(20+4/2,20+3/2+1)=(22,22)` with facing `0xC0` (`production_refinery.rs:94-100,142-155`). | **PASS** |
| 3 | Primary placement rejection | Constructed CMIN calls virtual `Unlimbo`; the true third-party blocker makes the call at `0x00446BD2` return false, entering fallback. | `spawn_object` supplies `PlacementEvidence::MarkSucceeded` unconditionally and adds another occupancy entry (`world_spawn.rs:598-615`; `lifecycle.rs:190-220`). The blocker cannot reject it. | **FAIL** |
| 4 | Nearby fallback attempts | Exactly two sequential FNPC calls, first `0x00446CCD`, then `0x00446E10`; each valid result gets one `Unlimbo`, at `0x00446D24` then `0x00446E67`, both facing `0xA0`. | Zero fallback attempts in this scenario. Rust's box-perimeter helper is reached only if primary coordinate arithmetic is unrepresentable, not if placement fails (`production_refinery.rs:94-113,157-190`). | **NOT-IMPLEMENTED** |
| 5 | Successful-fallback mission | After either fallback succeeds, native queues mission `10` then commences it at `0x00446EA7/0x00446EB1`. Mission 10 is Harvest. | The wrongly primary-overlapped miner is nevertheless assigned `MissionType::Harvest = 10` (`world_spawn.rs:254-276`; `mission/mod.rs:65`). | **PASS** |
| 6 | Total-failure object cleanup | After both passes fail, native calls constructed CMIN vtable `+0x20(1)` at `0x00446E94-0x00446E9A`, uninitializing/destroying it. No CMIN remains. | This branch is unreachable; the primary CMIN remains alive, active, and cell-marked (`production_refinery.rs:115-139`). | **FAIL** |
| 7 | Total-failure owner refund | Native obtains owner-adjusted CMIN cost through UnitType vtable `+0xB8(owner,1)` and calls `HouseClass::Add_Credits` at `0x00446E71-0x00446E8F`. For this stock Americans case: `C -> C+1400`. Allocation failure has the same refund at `0x00446EB9-0x00446EDD`. | No credit mutation exists in the completion FreeUnit path: `C -> C` (`production_refinery.rs:115-139`). | **FAIL** |
| 8 | Player-visible terminal state | Primary-only obstruction: CMIN appears at the first usable nearby cell facing `0xA0`. Total failure: no CMIN, owner has 1400 more credits. | Both variants show a CMIN overlapping the blocker at `(22,22)`, facing `0xC0`; no refund. | **FAIL** |
| 9 | Exact cell returned within each FNPC pass | Attempt order and raw option values are computed, but this trace did not instantiate the full native candidate pool/frame state needed to name the returned cell. | Rust never calls FNPC for this placement failure. | **UNCHECKED** |

## Native attempt order and raw options

After primary failure, native seeds both searches from the building's world
location. The first result is fully consumed before the second search begins:

1. FNPC call `0x00446CCD`, raw post-zone options
   `0,1,1,1,1,0,0,scratch,0,0`; if non-invalid, one `Unlimbo` at
   `0x00446D24` with facing `0xA0`.
2. Only after result-invalid or `Unlimbo=false`, FNPC call `0x00446E10`, raw
   post-zone options `0,1,1,0,1,0,0,scratch,0,0`; if non-invalid, one
   `Unlimbo` at `0x00446E67` with facing `0xA0`.
3. Only after the second result/placement fails does refund precede object
   uninitialization.

The unnamed option that changes from `1` to `0` remains deliberately unnamed.
The exact internal candidate visitation/result cell is **UNCHECKED**, not inferred.

## Static foundation blocker versus the scenario blocker

Rust's `PathGrid` tests mark the entire completed `4x3` refinery foundation
blocked (`production_placement_tests.rs:136-151,696-735`). The implementation
intentionally ignores that static foundation mark for the native internal primary
bay (`production_refinery.rs:97-99`); otherwise ordinary stock primary placement
would always be rejected.

That does not justify ignoring the independent dynamic occupancy list. The scenario
uses a different object's live occupancy at `(22,22)`. Native `Unlimbo` observes
that placement failure and falls back; Rust bypasses all placement admission and
blindly inserts a second object into the same cell. The Rust comment correctly
distinguishes the refinery's own static blocker, but the code collapses both cases
into unconditional success.

## Milestone findings

1. **High when the primary bay is dynamically occupied:** Rust produces an
   overlapping miner instead of using nearby placement.
2. **High under local congestion:** the native two-pass recovery path is absent,
   so attempt count, option split, and fallback facing differ.
3. **High when all placements fail:** Rust retains a live miner where native
   destroys the constructed object.
4. **High when all placements fail:** Rust withholds the stock 1400-credit owner
   refund.

The trigger is bounded to a refinery completing while its native bay is occupied;
it is not an every-refinery-completion failure.

## Adjacent findings, intentionally untraced

- Normal open-ground FreeUnit appearance timing relative to the buildup frame.
- Simultaneous refinery completion stable identity/allocation order.

## Validation and verdict tally

No Cargo command was run: this trace is read-only apart from this required report,
and compiling would create additional files outside the one-file write contract.
Evidence is direct source inspection at commit `799515ca`, stock merged INI values,
and live read-only decompile/disassembly of active `gamemd.exe`.

**PASS: 3 | FAIL: 4 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1**
