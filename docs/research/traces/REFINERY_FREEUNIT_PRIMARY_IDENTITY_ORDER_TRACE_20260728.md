# Refinery FreeUnit Primary Identity and Simultaneous Order Trace

**Date:** 2026-07-28  
**Rust target:** feature worktree commit `799515ca9867ac189e7c6ea9b03d0d93938d5c6b`  
**Scenario:** stock `GAREFN` NW `(20,20)` is placed/revealed before stock `NAREFN`
NW `(20,35)`; both finish `BuildingUp` on tick T; each primary `Unlimbo` succeeds.
The fixture owners are `Americans` and `Russians`, respectively.

## Verdict

**SCOPED PASS.** Rust and active standard-YR `gamemd.exe` compute the same two
primary cells, primary facing, owner/type identities, fresh Harvest state, and
CMIN-before-HARV creation/registration order for this exact reveal order.
Exact pixels were not captured, so the screen result remains `UNCHECKED`.

**Tally:** PASS: 8 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Scope and stop condition

This trace ends when both successful primary spawns are stored, revealed/registered,
mission-initialized, and handed to rendering. Construction timing and blocked
primary fallback/refund are adjacent findings only; they are owned by the other
trace slots and are not scored here.

## Active-YR inputs

- Merged stock `rulesmd.ini`: `GAREFN` has `Refinery=yes`, `FreeUnit=CMIN`;
  `NAREFN` has `Refinery=yes`, `FreeUnit=HARV`
  (`ini/rulesmd.ini:11722-11736`, `:12515-12530`).
- Stock `artmd.ini`: both foundations are `4x3`
  (`ini/artmd.ini:1706-1709`, `:1763-1766`).
- This is active standard YR, not TS legacy: the live building construction
  handler reaches the FreeUnit field and the stock YR INIs populate both types.

## Pipeline

`BuildingUp completions [GAREFN,NAREFN]` -> `resolve [CMIN,HARV]` ->
`derive [(22,22),(22,37)]` -> `facing 0xC0` ->
`construct with source owner/type` -> `store + reveal/register` ->
`Harvest(10), state 0` -> `CMIN before HARV` -> `unit render handoff`

## Entry-point inventory for this trigger

1. `place_ready_building` creates the structure and installs `BuildingUp`
   (`src/sim/production/production_placement.rs:163-230`).
2. `tick_building_up` is the sole searched Rust completion collector; it walks
   sorted stable IDs and returns finished IDs (`src/sim/world/mod.rs:1801-1822`).
3. Phase 9 passes only those IDs to `spawn_completed_refinery_free_units`
   (`src/sim/world/mod.rs:1985-1996`).
4. The earlier placement-time refinery spawn hook was removed in this commit.
   Map spawns and generic production spawns do not independently trigger this
   completion-owned FreeUnit path.

## Concrete stage trace

| # | Stage | Rust result | Active `gamemd` result | Verdict |
|---:|---|---|---|---|
| 1 | Stock binding | `GAREFN -> CMIN`; `NAREFN -> HARV`; both `4x3` | Building type `+0xEA0` supplies the resolved unit type; stock INIs bind the same pair | PASS |
| 2 | Simultaneous selection order | Fixture places GAREFN first; completed IDs are `[1,2]` in ascending stable-ID order (`production_placement_tests.rs:820-860`; `world/mod.rs:1803-1821`) | With the corresponding GAREFN-before-NAREFN successful reveal order, the live Logic vector visits GAREFN first, then NAREFN | PASS |
| 3 | Allied primary cell | `20 + floor(4/2) = 22`; `20 + floor(3/2) + 1 = 22` (`production_refinery.rs:142-155`) | NW cell center -> complete-object center NW `+(2,1)` -> south `(0,+1)` = `(22,22)` | PASS |
| 4 | Soviet primary cell | `20 + 2 = 22`; `35 + 1 + 1 = 37` | Same `4x3` calculation = `(22,37)` | PASS |
| 5 | Primary facing | Both spawn with `0xC0` (`production_refinery.rs:14-16`, `:94-116`) | Primary `Unlimbo` pushes `0xC0` at `0x00446BA5` | PASS |
| 6 | Owner and type identity | Each completion snapshots its building owner/type, resolves that building's `FreeUnit`, and passes both unchanged into `spawn_object` (`production_refinery.rs:32-68`, `:86-116`; `world_spawn.rs:320-337`) | Constructor call receives `BuildingClass+0x21C` owner and `BuildingType+0xEA0` unit type (`0x00446B78-0x00446B8E`); constructor stores the type at `Unit+0x6C4` | PASS |
| 7 | Create/store/reveal order | GAREFN allocates CMIN ID 3, stores and reveals it, then NAREFN allocates HARV ID 4; successful reveal tail-appends in that order (`world_spawn.rs:320-430`, `:569-615`) | GAREFN's inline callback constructs/unlimbos CMIN before NAREFN's later object turn constructs/unlimbos HARV; ordinary reveal uses unsorted tail insertion | PASS |
| 8 | Immediate mission state | Both miners finish spawn with `current=Harvest(10)`, `queued=None`, handler state `0` (`SearchOre`), frame-anchored zero-delay dispatch (`world_spawn.rs:254-276`; `mission/state.rs:139-147`; `miner/mod.rs:77-88`) | After successful primary placement, vtable `+0x1E8` queues `10` and `+0x1EC` commences it (`0x00446E9F-0x00446EB1`): current `10`, queue `-1`, substate `0`, current-frame anchors, zero delay | PASS |
| 9 | Render/screen | Spawn result flags atlas refresh; voxel rendering consumes entity type, owner, cell-derived screen position, and facing (`app_sim_tick.rs:1654-1709`; `app_instances/units.rs:131-243`) | Successful native `Reveal` submits the object and registers eligible logic objects | UNCHECKED |

## Native evidence and identity checks

- `0x00449A50` is confirmed from its body as the active Building construction
  state machine: state 1 tests `Building+0x6DD` and calls receiver vtable `+0x4DC`
  at `0x00449AD4` (fresh read-only `batch_decompile` and
  `disassemble_bytes 0x00449A50..0x00449B20`).
- `0x00445F80` is confirmed from its Building receiver flow and the caller above:
  it reads `Type+0xEA0`, `Owner+0x21C`, calls complete-object slot `+0x48`,
  constructs a unit, unlimbos it, then queues/commences mission 10
  (fresh read-only `batch_decompile 0x00445F80` and
  `disassemble_bytes 0x00446A20..0x00446F00`).
- `0x00447AC0` computes `location + foundation_dimension*128 - 128` on X/Y
  (fresh read-only `batch_decompile 0x00447AC0`).
- `0x007353C0` is confirmed as the Unit constructor from its Foot base call,
  Unit vtable writes, global Unit array append, type store, and locomotor setup;
  `0x00446B8E` is a direct caller (fresh read-only `batch_decompile`,
  `disassemble_bytes`, and `get_function_xrefs`).
- Unit vtable bytes at `0x007F5E58` resolve `+0x1E8 -> 0x005B35E0`
  and `+0x1EC -> 0x005B3570`; their bodies are Queue and Commence field
  transitions (fresh read-only `read_memory` and `batch_decompile`).
- `ObjectClass::Reveal @ 0x005F4EC0` calls registration with unsorted flag `0`;
  `0x0055BAA0 -> 0x005519B0` writes at `items[count]`; the live loop visits
  increasing indices (`0x0055B608-0x0055B619`). All were freshly checked through
  read-only decompile/disassembly.

## Player-visible result

On tick T's successful primary creation path, the Allied player receives a west-facing
CMIN at `(22,22)` and the Soviet player receives a west-facing HARV at `(22,37)`.
CMIN is created and registered before HARV. No scoped player-visible mismatch was found.
Exact voxel frame, depth, pixel position, and same-pass first-AI visual consequences
were not captured, so they do not receive a PASS.

## Adjacent findings (untraced and unscored)

- Construction callback timing and native per-object interleaving versus Rust's Phase-9
  bulk completion owner belong to slot 1.
- Blocked primary placement, both native fallback passes, destruction, and refund
  behavior belong to slot 3.

## Validation note

The exact scenario is encoded by
`simultaneous_refinery_completions_preserve_stable_id_order`
(`src/sim/production/production_placement_tests.rs:816-861`). Commit `799515ca`
records scoped placement/refinery tests as passing. This trace did not rerun Cargo:
processes `cargo` PID 90160 and 93900 already owned Cargo, so the project concurrency
rule required a read-only source/Ghidra trace.
