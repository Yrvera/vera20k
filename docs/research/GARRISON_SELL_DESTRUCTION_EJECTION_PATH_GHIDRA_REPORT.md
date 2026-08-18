# Garrison Sell / Destruction Ejection Path -- Ghidra Research Report

**Address(es):** `0x00457DE0` (`BuildingClass::SellBuilding` occupant-eject helper), `0x00449C30` / `0x0044A5CA` (`BuildingClass::Sell` player-sell state machine), `0x00442230` (`BuildingClass::ReceiveDamage` destruction case), `0x0051D0D0` (`InfantryClass::Scatter`)
**Investigation Mode:** exhaustive-slice for garrison occupants leaving a `CanBeOccupied` building due to player sell or building destruction.
**Claimed Scope:** sell/destruction path liveness, occupant order, exit-coordinate selection, failed `Unlimbo`, Scatter handoff, parachute fallback condition, and current Rust deltas.
**Non-Scope:** generic vehicle/building transport unload, open-topped/bunker passenger unload, garrison fire, occupant damage/removal by `PenetratesBunker`, full Infantry scatter branch audit, crew survivor spawning.
**Confidence:** Medium-high. Handoff-critical facts are backed by recent Ghidra reports that cite decompile plus assembly/caller ranges, but this session had no callable Ghidra MCP tool for a fresh cold re-read.
**Active in YR:** Yes for `CanBeOccupied=yes` buildings in standard Yuri's Revenge; `rulesmd.ini` contains many active `CanBeOccupied=yes` civilian structures, and `BuildingClass::ReceiveDamage` gates the destruction branch on `BuildingTypeClass+0x157B`.

## 0. Target Question / Non-Goals / Completion Gate

**Target question:** Verify whether garrison occupant ejection on sell and building destruction share `BuildingClass::SellBuilding @ 0x00457DE0`, and document exact order, placement, failed-placement outcome, Scatter handoff, parachute condition, and Rust-facing consequences.

**Non-goals:** Do not investigate generic transport unload beyond proving it is separate. Do not re-open garrison fire, entry gates, bunker lifecycle, or occupant death-by-warhead.

**Evidence needed to mark COMPLETE:** decompile plus assembly/caller evidence for sell and destruction reaching the ejection helper; Rust line scan proving current sell/destruction deltas; final open-question log with all items resolved or explicitly deferred.

**Stop conditions:** no callable Ghidra MCP for fresh verification; missing function boundary that cannot be inspected read-only; evidence that destruction uses a broad separate owner path outside this slice.

## 1. Overview

Gamemd's player-sell path and `CanBeOccupied` destruction path both reach the same occupant-eject helper, `BuildingClass::SellBuilding @ 0x00457DE0`. Despite the name, that helper ejects/clears garrison occupants; the actual sell state machine and destruction handling live in callers. Active in YR: Yes, because standard YR buildings with `CanBeOccupied=yes` call this path on sell, red-HP emergency eject, and destruction case 4.

The important Rust-facing result is that destruction should not be modeled as a separate random/shuffled foundation-interior placement path. It should follow the sell/eject contract: reset fire index, deterministic foundation-edge exit search, reverse occupant-vector iteration, `Unlimbo(exit_coord, 0)`, destroy the occupant only when that `Unlimbo` fails, then call the occupant Scatter virtual with the building coordinate. For infantry, Scatter can consume `RandomRanged(0,4)`, queue mission `2`, and set a destination. The later mission `0xF` block in the helper is first-argument gated and was not active for the direct callers checked in the 2026-05-27 garrison swarm. Active in YR: Yes for the shared helper and direct Scatter handoff; `0xF` liveness is conditional.

## 2. Class Layout / Key Offsets

| Offset / field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass+0x684` | DynamicVectorClass header for garrison occupants | `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`, constructor/add/eject traces | Yes |
| `BuildingClass+0x688` | occupant pointer array | `0x00457DE0` loop; prior report cites assembly `0x004580B1` load | Yes |
| `BuildingClass+0x694` | occupant count | `0x004581F0` getter; `0x00457DE0` loop setup | Yes |
| `BuildingClass+0x69C` | garrison fire index reset before ejection | `0x00457DE0` first write per prior report | Yes |
| `BuildingTypeClass+0x157B` | `CanBeOccupied` destruction-path gate | `0x00442230` case 4; `rulesmd.ini` active values | Yes |

## 3. Core Logic

### 3.1 Shared ejection helper

`BuildingClass::SellBuilding @ 0x00457DE0` is the garrison occupant-eject helper, not the whole player-sell transaction. Active in YR: Yes. Evidence: `GARRISON_SYSTEM_GHIDRA_REPORT.md` Section 14c and `PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md` Section 3.1 cite decompile plus assembly around `0x00458098..0x00458138`.

Verified behavior:

| Behavior | Evidence | Active in YR |
|---|---|---|
| Fire index reset happens before occupant ejection. | `0x00457DE0`, first write to `Building+0x69C` in prior Ghidra reports | Yes |
| Empty occupant vector returns without placement/RNG. | `0x00457DE0` `GetOccupantCount` gate | Yes |
| Exit cell search is deterministic around foundation edges; no Fisher-Yates and no raw `% 8` draw in the helper. | prior report decompile `0x00457E35..0x00458060`, assembly spot-check around `0x00458060` / `0x00458098` | Yes |
| Occupants iterate high-to-low from `count - 1` down to `0`. | prior report assembly `0x00458098 MOV EBP,[ESI+0x694]`, `0x0045809E DEC EBP`, `0x004580B1 MOV EDI,[EAX+EBP*4]` | Yes |
| Each occupant attempts `Unlimbo(exit_coord, 0)`. | prior report assembly `0x004580BD CALL [EDX+0xD8]` | Yes |
| Failed `Unlimbo` calls the occupant vtable `+0xF8`, destroying/removing that occupant. | prior report assembly `0x004580C3 TEST AL,AL` then failure branch | Yes |
| Successful `Unlimbo` clears the archive target, calls occupant Scatter with the building coordinate and two true flags, and does not draw raw `% 8` RNG inside the ejection helper. | `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`; assembly `0x004580E9..0x0045810A` | Yes |
| The later `+0x1E8(0xF,0)` block exists but is gated on `GetFoundationWidth() != 0`; for any real building (width > 0) this block fires for every successfully ejected occupant. (corrected 2026-05-29: was "first-argument gated; direct callers pass first argument zero" — WRONG; `BuildingClass__SellBuilding` takes only `this`, no extra arg; the gate at `0x00458110` reads `cVar4` = the `GetFoundationWidth()` return at `0x00457E35`; any building with nonzero width enters the block — via `decompile_function 0x00457DE0` + `disassemble_function 0x00457DE0` — ROOT_CAUSE: INFERENCE_HARDENED) | assembly `0x00458110..0x00458138`; `decompile_function 0x00457DE0` confirmed `cVar4 = BuildingTypeClass__GetFoundationWidth()` gate | Yes for real buildings |
| If no exit coordinate can be found after all four edge scans, the fallback branches on `GetFoundationHeight() == 0`: a zero-height building calls `SpawnUnitsWithParachute(0)` and returns; a nonzero-height building computes an inside-foundation fallback coordinate from the foundation bounds. The branching is internal to `SellBuilding` — it is NOT caller-dependent. (corrected 2026-05-29: was "caller-argument dependent: destruction/red-HP callers take SpawnUnitsWithParachute(0)'s null remove branch; normal player sell uses inside-foundation fallback" — WRONG; all callers invoke the same void-thiscall `SellBuilding(this)` with no extra arg; the `cVar5 = GetFoundationHeight()` test at `0x00458140..0x00458146` is what controls the branch — via `decompile_function 0x00457DE0` + `disassemble_function 0x00457DE0` — ROOT_CAUSE: INFERENCE_HARDENED) | `disassemble_function 0x00457DE0` confirms `0x00458140: MOV AL,[ESP+0x44]` (cVar5 = height); `0x00458146: JZ 0x00458180` → `SpawnUnitsWithParachute(0)`; else falls through to inside-foundation coord | Yes |

### 3.2 Sell caller

Player sell uses `BuildingClass::Sell`, a state machine, and calls `SellBuilding` from the occupant-ejection stage before the final building destroy/refund stage. Active in YR: Yes. Evidence: `docs/gap-scans/2026-05-04b-disparity-scan-garrison.md:78..83` cites `BuildingClass::Sell @ 0x0044A5CA`; `PASSENGER...` lists `0x00449C30` state 1 as the sell mission caller.

This means `SellBuilding` should be described as occupant ejection only. It does not by itself prove the building survives; the player-sell state machine later destroys the building for the normal sell flow. Active in YR: Yes.

### 3.3 Destruction caller

`BuildingClass::ReceiveDamage @ 0x00442230` destruction result case 4 calls the same `SellBuilding` helper if `BuildingTypeClass+0x157B CanBeOccupied` is true. Active in YR: Yes. Evidence: `PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md` Section 3.2, plus `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` `BuildingClass__ReceiveDamage` entry.

No separate "destroyed garrison shuffled foundation interior" path is verified for standard YR. Active in YR: No for that proposed shortcut; evidence is absence in the verified destruction caller and current Rust-only comments at `src/sim/production/production_sell.rs:380..399`.

### 3.4 Scatter and RNG

The ejection helper does not choose a random adjacent destination. It calls the occupant Scatter virtual after successful `Unlimbo`; for infantry, that Scatter call uses scenario `RandomRanged(0,4)` jitter around a computed direction, not raw `next_u32() % 8`. Active in YR: Yes. Evidence: `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md` cites `0x0045810A`, `0x0051D2AC..0x0051D2BA`, and `0x0051D36D..0x0051D385`.

## 4. INI Keys

| Key | Default / role | Evidence | Active in YR |
|---|---|---|---|
| `CanBeOccupied` | default false, many YR civilian buildings override yes; gates destruction ejection at `+0x157B` | `ini/rulesmd.ini` active occurrences; `0x00442230` gate | Yes |
| `MaxNumberOccupants` | capacity for garrison vector; not part of sell/destruction ejection placement | `ini/rulesmd.ini`; `GARRISON_SYSTEM_GHIDRA_REPORT.md` | Yes |
| `Occupier` | infantry entry eligibility; not part of ejection | `ini/rules.ini` / `rulesmd.ini`; prior garrison docs | Yes for entry, out-of-scope here |
| `Parachuted` / parachute rules | generic parachute capability/rules exist, but this slice only proves `SpawnUnitsWithParachute(0)` fallback when no exit coord exists | `ini/rulesmd.ini:25332`; `0x00457DE0` prior report | Conditional |

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::Sell @ 0x0044A5CA` / `0x00449C30` | player-sell state machine calls occupant-eject helper | prior reports and gap scan | Yes |
| `BuildingClass::ReceiveDamage @ 0x00442230` | destruction case calls same helper for `CanBeOccupied` | prior reports | Yes |
| `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200` | red-HP emergency ejection without building destruction; related but not the sell/destruction focus | gap scan `2026-05-04b` | Yes, separate follow-up |
| Generic transport unload | separate from garrison vector and not proven by this helper | `PASSENGER...` generic unload deferral | Conditional/out-of-scope |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Match status | Evidence |
|---|---|---|---|
| `src/sim/production/production_sell.rs:248..378` | sell garrison ejection snapshots cargo, uses perimeter helper, iterates reverse, kills if no free selected cell, issues immediate random adjacent move | partial | LIFO mostly matches; immediate raw scatter and no parachute fallback mismatch |
| `src/sim/production/production_sell.rs:380..511` | destruction ejection shuffles foundation interior cells with Fisher-Yates, places in foundation footprint, then immediate raw scatter | mismatch | binary destruction path calls same sell/eject helper |
| `src/sim/combat/mod.rs:600..619`, `872..903` | `DestroyedGarrisonBuilding` event documents random foundation placement and collects foundation dimensions | mismatch in contract/comments | should represent shared sell-style ejection contract or reuse sell helper |
| `src/sim/world/mod.rs:1398..1400` | world drains destroyed garrison events through `production::eject_destruction_garrison` | affected surface | call target likely needs sell-parity behavior |
| `src/sim/passenger.rs:1083..1111` | test helper removes building then calls destruction ejection helper | test surface affected | existing tests encode current random-foundation behavior |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::SellBuilding @ 0x00457DE0` ejection contract | verified-from-prior-ghidra | prior decompile + assembly ranges in `PASSENGER...` | fresh Ghidra MCP cold re-read unavailable |
| player-sell caller | verified-from-prior-ghidra | `0x0044A5CA` / `0x00449C30` prior caller evidence | none for this slice |
| destruction caller | verified-from-prior-ghidra | `0x00442230` case 4 prior caller evidence | none for `CanBeOccupied` buildings |
| exit scan exact directional order | touched-not-exhausted | prior docs say deterministic four foundation edges | exact edge-order pseudocode should be re-read in Ghidra before implementing pixel-perfect scan |
| failed `Unlimbo` behavior | verified-from-prior-ghidra | vtable `+0xF8` failure branch cited at `0x004580BD..0x004580C3` | none |
| parachute fallback | verified-from-prior-ghidra | `SpawnUnitsWithParachute(0)` branch cited in prior docs | exact spawn ordering after parachute helper not explored |
| generic transport unload | deferred | prior report marks generic unload out-of-scope | separate investigation |
| current Rust sell/destruction surfaces | verified | file scan line references above | implementation not performed |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Do sell and destruction share the same garrison ejection helper? -> Yes for `CanBeOccupied` destruction and player sell; both reach `SellBuilding @ 0x00457DE0`.` (evidence: `0x00449C30` / `0x0044A5CA`, `0x00442230`, `0x00457DE0` prior reports)
- `[RESOLVED] OQ-02 -- Is destruction a separate random foundation-interior placement path? -> No verified standard-YR path; that behavior exists in current Rust, not in the cited binary path.` (evidence: `PASSENGER...` Section 3.2; Rust `production_sell.rs:380..420`)
- `[RESOLVED] OQ-03 -- What is occupant order? -> reverse/high-to-low from `count - 1` to `0`.` (evidence: assembly `0x00458098..0x004580B1` cited in prior report)
- `[RESOLVED] OQ-04 -- What happens on failed `Unlimbo`? -> occupant vtable `+0xF8` is called, so that occupant is destroyed/removed; later occupants continue.` (evidence: `0x004580BD..0x004580C3` cited in prior report)
- `[RESOLVED] OQ-05 -- Is Scatter immediate raw `% 8`? -> No; helper calls the occupant Scatter virtual after successful `Unlimbo`; Infantry scatter uses scenario `RandomRanged(0,4)` jitter after its own gates.` (evidence: `0x0045810A`, `0x0051D2AC..0x0051D385`, `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-06 -- When is parachute used? -> When all four foundation-edge scans find no free cell, SellBuilding checks GetFoundationHeight(): if zero, calls SpawnUnitsWithParachute(0); if nonzero, computes inside-foundation fallback coordinate. Branching is internal to SellBuilding — not caller-dependent.` (corrected 2026-05-29: was "caller-argument dependent" — WRONG; no extra arg exists; gate is cVar5=GetFoundationHeight() at 0x00458140; via disassemble_function 0x00457DE0 ROOT_CAUSE: INFERENCE_HARDENED; evidence: `disassemble_function 0x00457DE0` assembly `0x00458140..0x00458180`)
- `[RESOLVED] OQ-07 -- Is the path active in stock YR? -> Yes; `CanBeOccupied=yes` is present in `rulesmd.ini`, and destruction caller gates on `BuildingTypeClass+0x157B`.` (evidence: `ini/rulesmd.ini`; `0x00442230`)
- `[DEFERRED] OQ-08 -- Exact foundation-edge directional scan pseudocode and coordinate bounds.` (category: `needs-runtime-debugger`; reason: no callable Ghidra MCP in this session for fresh disassembly/decompile pass; next-step-if-pursued: cold re-read `0x00457E35..0x00458060` before final implementation)
- `[DEFERRED] OQ-09 -- Exact `SpawnUnitsWithParachute(0)` passenger ordering/landing positions.` (category: `out-of-scope`; reason: fallback helper internals are parachute-system work; next-step-if-pursued: targeted parachute fallback investigation)
- `[DEFERRED] OQ-10 -- Generic transport unload parity.` (category: `out-of-scope`; reason: garrison vector path is separate; next-step-if-pursued: generic transport unload re-investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Destruction of `CanBeOccupied` garrisons calls the same `SellBuilding` ejection helper as sell. | `0x00442230` caller + `0x00457DE0` helper in prior Ghidra reports | mismatch: Rust destruction uses shuffled foundation interiors | `src/sim/production/production_sell.rs::eject_destruction_garrison`, `src/sim/combat/mod.rs::DestroyedGarrisonBuilding` | Make destruction use the sell-style deterministic foundation-edge ejection contract. | Destroy a 2x2 occupied civilian building with a fixed map and assert occupants appear at the same first free edge cells as sell, in reverse occupant order. | Do not preserve the Fisher-Yates foundation shuffle. |
| Occupants are processed reverse/high-to-low and failed `Unlimbo` destroys only that occupant. | assembly ranges `0x00458098..0x004580C3` cited in prior report | partial: Rust uses reverse order but kills on no free cell before modeling parachute/actual `Unlimbo` failure | `production_sell.rs` garrison helpers and tests | Preserve LIFO; distinguish "no exit coord" parachute fallback from "chosen Unlimbo failed" destruction. | Block the selected exit cell after an exit coordinate exists and assert that occupant is destroyed while later loop behavior remains deterministic. | Do not turn all blocked-edge cases into blanket kill-all. |
| Successful ejection calls direct Scatter; the helper does not draw raw `% 8` scatter direction. | `0x004580E9..0x0045810A`, Infantry scatter `0x0051D2AC..0x0051D385` | mismatch: Rust sell/destruction immediately issues random adjacent move using `next_u32() % 8` | future scatter/movement surface plus `production_sell.rs` | Route through a parity Scatter operation so RNG order matches Scatter's `RandomRanged(0,4)` gates, not immediate raw modulo. Keep the later `0xF` queue gated unless a live nonzero-first-argument caller is proven. | Compare RNG state before/after ejection with blocked-free map: ejection should not consume per-occupant `% 8` draws before Scatter runs. | Do not replace `% 8` with `RandomRanged(0,7)`; the call timing and bounds are both wrong. |

Acceptance test-name proposals:

- `destroyed_garrison_uses_sell_edge_scan_and_lifo_order`
- `garrison_ejection_failed_unlimbo_kills_only_failed_occupant`
- `garrison_ejection_queues_scatter_without_immediate_raw_direction_rng`

### Negative Facts / Do Not Do

- Do not implement destruction garrison ejection as random or shuffled foundation-interior placement. Evidence: destruction `CanBeOccupied` caller reaches `SellBuilding @ 0x00457DE0`, while current shuffle exists only in Rust comments/code at `production_sell.rs:380..420`. Active in YR: No.
- Do not treat `BuildingClass::SellBuilding` as the whole sell transaction. Evidence: player sell state machine `0x0044A5CA` / `0x00449C30` calls it as one stage; final destruction/refund is caller work. Active in YR: Yes.
- Do not treat all no-edge cases the same. The fallback when no exit cell is found is determined by `GetFoundationHeight()` inside `SellBuilding` itself — not by which caller invoked it. If height == 0, `SpawnUnitsWithParachute(0)` runs; otherwise an inside-foundation fallback coordinate is computed. Active in YR: Yes. (corrected 2026-05-29: was "destruction/red-HP callers pass zero" — WRONG; no caller-specific argument exists; see `disassemble_function 0x00457DE0` `0x00458140..0x00458180` — ROOT_CAUSE: INFERENCE_HARDENED)
- Do not consume raw `next_u32() % 8` inside the ejection helper for Scatter. Evidence: `0x0045810A` calls the Scatter virtual; `0x0051D2AC..0x0051D385` shows Infantry Scatter uses later `RandomRanged(0,4)`. Active in YR: Yes.
- Do not use garrison ejection evidence to rewrite generic transport unload. Evidence: prior `PASSENGER...` report explicitly left generic transport unload out of scope. Active in YR: Conditional and separate.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_sell.rs:380..399` comment should be replaced with: "Destroyed `CanBeOccupied` garrisons use the same gamemd `SellBuilding @ 0x00457DE0` occupant-eject contract as sell: deterministic foundation-edge exit search, reverse occupant order, `Unlimbo(exit, 0)`, failed `Unlimbo` destroys that occupant, no immediate raw scatter-direction RNG; if no exit coord exists, gamemd calls `SpawnUnitsWithParachute(0)`."
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/mod.rs:600..606` comment should be replaced with: "A `CanBeOccupied` building destroyed in combat with live occupants; gamemd routes this through `BuildingClass::SellBuilding @ 0x00457DE0`, the same occupant-eject helper used by sell, rather than random foundation-footprint placement."
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_SYSTEM_GHIDRA_REPORT.md` Section 16 row for `0x00457DE0` should say: "Occupant ejection only -- does NOT destroy the building; called by player-sell state and `CanBeOccupied` destruction paths."
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` `BuildingClass__CheckAutoSellOrCivilian` bullet should say: "Empty + non-civilian owner -> revert to civilian house. Red-HP -> eject occupants via `SellBuilding` without destroying the building."

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_SYSTEM_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/gap-scans/2026-05-04b-disparity-scan-garrison.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_sell.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/passenger.rs`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`

## Status

PARTIAL: the behavioral answer and Rust handoff are strong enough to act on, because the key claims cite recent decompile plus assembly/caller evidence from existing reports. It is not marked COMPLETE because this session did not expose a callable Ghidra MCP tool, so the required fresh cold re-read / zero-add Ghidra pass could not be performed.
