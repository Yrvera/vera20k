# PDPLANE Spawner Edge/Silent Path - Ghidra Research Report

**Address(es):** `0x0065E660` primary spawner; `0x0050DA80` invalid-edge fallback; `0x004AA440` map-edge cell finder; `0x004AAB30` edge-candidate predicate; `0x004733A0` cargo add; `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655` paradrop call sites.  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** PDPLANE spawn edge resolution, invalid `House+0x1E0` fallback, `FUN_004AA440` arguments used by the spawner, `g_MapEditorMode` suppression around carrier creation/unlimbo, and passenger cargo loading.  
**Non-Scope:** aircraft mission behavior after spawn, drop cadence, bridge target validation, parachute descent/rendering, and opposite-edge exit/despawn.  
**Confidence:** High for edge fallback, call arguments, carrier silent creation, and passenger cargo-linking; Medium for the full global side-effect inventory of `g_MapEditorMode` because this slice spot-checked the spawner gates but did not re-audit all global xrefs.  
**Active in YR:** Yes. `rulesmd.ini` defines `[ParaDropSpecial] Type=ParaDrop` and `[AmericanParaDropSpecial] Type=AmerParaDrop`; `SuperClass::Launch` cases 5/6 call `FUN_0065E660` through standard YR paths.

## 0. Working Notes Gate

Target question: Does `FUN_0065E660` spawn PDPLANE at the home edge through a fallback/silent path, and how exactly are cargo passengers created and linked?  
Non-goals: Do not investigate the aircraft's approach/overfly/drop missions, bridge target substitution, or parachuted infantry descent/rendering.  
Evidence needed to mark COMPLETE: primary spawner decompile plus assembly ranges for edge fallback, edge finder call args, `g_MapEditorMode` increments, paradrop launch call-site args, and cargo `AddPassenger`; INI/default evidence for stock YR activation; current Rust surface scan.  
Stop conditions: stop after all spawner-path open questions are resolved or explicitly deferred; do not follow post-spawn mission state, generic map-edge helper modes not used by this call, or global side-effect xrefs beyond the spawner's suppression boundary.

## 1. Overview

`FUN_0065E660` is the shared object spawner used by standard paradrop launch paths to create one PDPLANE carrier, place it at a house-selected map edge, set its initial mission/destination, and fill its cargo list with infantry passengers. For stock YR paradrops, callers always pass aircraft count `1`, mission `0x1A`, the validated target cell, no extra target, an infantry-type array index, and a passenger count from the matching `*ParaDropNum` list.

The important verified behaviors are: invalid `WaypointEdge` falls back through secondary edge storage before defaulting to north; the edge finder call uses mode/criterion `4`, whose predicate accepts candidates immediately instead of checking ordinary path-grid walkability; and passengers are created in limbo and linked into cargo, not spawned/unlimbo'd onto the edge cell and then removed from occupancy. Current Rust now matches the passability and limbo-loading shape; the secondary `House+0x577C` fallback remains the main Rust delta from this report.

## 2. Class Layout / Key Offsets

| Offset / global | Type | Purpose | Active in YR |
|---|---|---|---|
| `HouseClass+0x1E0` | int | Primary paradrop spawn edge, valid range `0..3`. | Yes; read by `0x0065E6C5`. |
| `HouseClass+0x577C` | int | Fallback edge read by `FUN_0050DA80`; invalid fallback clamps to `0`. | Yes; called at `0x0065E6D6` only when `+0x1E0` is outside `0..3`. |
| `0x0087F7E8` | map singleton / map-class object | Receiver for `FUN_004AA440`; the edge finder is map-bound, not house-bound. | Yes; loaded into ECX before the call at `0x0065E6F0`. |
| `0x00A8E7AC` | int | `g_MapEditorMode`; incremented/decremented around carrier `CreateObject` and carrier `Unlimbo`. | Yes; written at `0x0065E691..0x0065E6B2` and `0x0065E73F..0x0065E78F`. |
| `Aircraft+0x3D4` | byte | Carrier flag set to `1` immediately after carrier object creation. Prior name `IsParachuted` is misleading for aircraft. | Yes; write at `0x0065E6BE`. |
| `Aircraft+0x6C9` | byte | Paradrop-carrying flag set before cargo creation when object `WhatAmI()==2`. | Yes; write at `0x0065E7B8`. |
| `Aircraft+0x114` | `CargoClass` base | Cargo count/head structure passed to `CargoClass::AddPassenger`. | Yes; `LEA ECX,[ESI+0x114]` at `0x0065E7EE`. |
| `CargoClass+0x00` | int | Recomputed cargo count. | Yes; reset/incremented at `0x00473403..0x0047340C`. |
| `CargoClass+0x04` | object pointer | Cargo head pointer. | Yes; written at `0x00473400`. |
| `Object+0x30` | object pointer | Next pointer used by cargo linked list. | Yes; written for the passenger at `0x004733FD`. |

## 3. Core Logic

### 3.1 Paradrop call-site arguments

Active in YR: Yes. Evidence: standard `SuperClass::Launch` cases 5 and 6 call `FUN_0065E660`; stock `rulesmd.ini` exposes both `Type=ParaDrop` and `Type=AmerParaDrop`.

The four relevant call sites all use the same shape:

| Call site | Branch | Stack/register facts | Evidence |
|---|---|---|---|
| `0x006CD421` | generic Allied list | `ECX=Owner House`, `EDX=PDPLANE index`, stack: `1`, `0x1A`, target cell, `0`, infantry `+0xDF8`, `AllyParaDropNum[i]`. | Assembly context `0x006CD3F1..0x006CD421`. |
| `0x006CD493` | generic Yuri list | Same, with `YuriParaDropNum[i]`. | Assembly context `0x006CD463..0x006CD493`. |
| `0x006CD4EB` | generic Soviet fallback list | Same, with `SovParaDropNum[i]`; no list-count assert in this branch. | Assembly context `0x006CD4BB..0x006CD4EB`. |
| `0x006CD655` | American paradrop | Same, with `AmerParaDropNum[i]`. | Assembly context `0x006CD625..0x006CD655`. |

Handoff-critical detail: the first two stack values are always aircraft count `1` and mission `0x1A`; `0x1A` is not cargo count. Passenger count is the final pushed value from the `*ParaDropNum` vector.

### 3.2 Carrier creation and silent boundary

Active in YR: Yes. Evidence: called from live cases 5/6; `g_MapEditorMode` write ranges are inside `FUN_0065E660`.

For each aircraft to spawn, the spawner increments `g_MapEditorMode`, calls aircraft-type vtable `+0x8C` to create the object, then decrements `g_MapEditorMode`. If creation returns null, it returns the count already spawned. The carrier-only flag at `+0x3D4` is then set to `1`.

The same global is incremented again around the carrier's vtable `+0xD8` `Unlimbo` call. If `Unlimbo` fails, the spawner calls carrier vtable `+0x20(1)` to delete it and returns `spawned_count - 1`. That negative return is possible when the first carrier fails unlimbo.

Handoff-critical evidence:

| Behavior | Evidence |
|---|---|
| `g_MapEditorMode++` before carrier `CreateObject`, decrement after | Assembly `0x0065E691..0x0065E6B2`. |
| Carrier flag `+0x3D4 = 1` after creation | Assembly `0x0065E6BE`. |
| `g_MapEditorMode++` around carrier `Unlimbo` | Assembly `0x0065E73F..0x0065E78F`; decompile of `FUN_0065E660`. |
| Failed `Unlimbo` deletes the carrier and returns `spawned_count - 1` | Decompile of `FUN_0065E660`; assembly around `0x0065E77B..0x0065E795`. |

This report verifies the spawner's suppression boundary. The broader claim that `g_MapEditorMode` suppresses construction sounds, AI lifecycle hooks, radar pings, and fog "first seen" events remains consistent with prior `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`, but this slice did not re-audit all 50+ global xrefs.

### 3.3 Invalid edge fallback

Active in YR: Yes. Evidence: live spawner reads `House+0x1E0` and calls fallback helper before every carrier edge lookup.

The spawner reads `House+0x1E0`. If the value is less than `0` or greater than `3`, it calls `FUN_0050DA80(House)`. The fallback helper reads `House+0x577C`; if that value is also outside `0..3`, it returns `0`.

Verified behavior:

| Input state | Result | Evidence |
|---|---|---|
| `House+0x1E0` in `0..3` | Use that edge directly. | Assembly `0x0065E6C5..0x0065E6D2`. |
| `House+0x1E0 < 0` or `> 3` and `House+0x577C` in `0..3` | Use `House+0x577C`. | `FUN_0050DA80` decompile; call at `0x0065E6D6`. |
| Both fields invalid | Use edge `0`. | `FUN_0050DA80` decompile. |

Do not implement invalid primary `waypoint_edge` as a launch abort.

### 3.4 Edge finder call and passability behavior

Active in YR: Yes. Evidence: `FUN_0065E660` always calls `FUN_004AA440` after resolving the edge.

The spawner calls `FUN_004AA440` on the map singleton (`ECX=0x0087F7E8`) with:

| Parameter | Value in spawner | Evidence |
|---|---|---|
| output cell pointer | local stack coord | `LEA EAX,[ESP+0x38]`, pushed before call at `0x0065E6EC..0x0065E6F5`. |
| edge | resolved `0..3` edge value in `EAX` | pushed at `0x0065E6EB`. |
| preferred / alternate cells | both sentinel `0x00B04C38` | pushes at `0x0065E6E1` and `0x0065E6E6`. |
| criterion / mode passed to candidate predicate | `4` | push at `0x0065E6DF`; consumed by `FUN_004AAB30`. |
| zone flag | `1` | push at `0x0065E6DD`; no zone is computed because the preferred/fallback cell is sentinel. |
| final flag | `0` | push at `0x0065E6DB`. |

Important correction: `FUN_004AAB30` immediately returns true when its criterion parameter is `4`. Assembly at `0x004AAB3D..0x004AAB4B` compares the parameter to `4`, sets `AL=1`, and returns before ordinary cell passability/object checks. Therefore this spawner call is an edge-cell finder, not a normal "find walkable edge cell" passability search.

Implications:

- The old Rust path/name `find_passable_at_edge` was misleading for this call path; current Rust uses a paradrop-specific carrier edge helper.
- Path-grid walkability is stricter than the binary for paradrop carrier spawn.
- The binary does not abort just because no path-grid-walkable edge cell exists; in this call, candidate acceptance is driven by in-playfield/perimeter iteration and the criterion-4 fast accept.
- The spawner does not check the returned cell against sentinel before making center coords and calling `Unlimbo`; any final failure is handled by the carrier `Unlimbo` result.

### 3.5 Mission/destination set before unlimbo

Active in YR: Yes. Evidence: inside live spawner before carrier `Unlimbo`.

After the edge cell is found, the spawner calls carrier vtable `+0x1E8` with mission `0x1A` and arg `0`. If the target cell argument is nonzero, it calls carrier vtable `+0x480(target, 1)`. The extra target argument from paradrop call sites is `0`, so vtable `+0x3C8` is skipped for standard paradrops.

Order matters: mission and destination are set before the carrier is unlimbo'd at the edge coordinate.

### 3.6 Passenger creation and cargo loading

Active in YR: Yes. Evidence: live spawner branch after carrier `WhatAmI()==2`, infantry index not `-1`, passenger count nonzero.

After successful carrier unlimbo, the spawner calls carrier vtable `+0x2C` and requires `WhatAmI()==2`. For paradrop, it also requires infantry index not `-1` and passenger count not zero. Then it sets carrier `+0x6C9=1`, looks up `g_InfantryTypeClass_Array[infantry_index]`, and loops exactly `passenger_count` times.

Each passenger loop:

1. Calls infantry-type vtable `+0x8C` with the owner house pointer as an argument.
2. If creation succeeds, calls passenger vtable `+0xD4`.
3. Calls `CargoClass::AddPassenger` with `ECX=carrier+0x114` and the passenger pointer.
4. Decrements the remaining passenger loop counter whether or not passenger creation succeeded.

Negative facts:

- Passenger objects are not `Unlimbo`'d here.
- The passenger loop is not wrapped in `g_MapEditorMode`.
- Passengers do not get transient edge-cell occupancy from this function.
- Cargo full/size-limit checks do not exist on this path; `CargoClass::AddPassenger` links the passenger into the list and recomputes cargo count.

`CargoClass::AddPassenger` itself calls passenger vtable `+0xD4`, writes the old cargo head into passenger `+0x30` unless it appends through a nested/open-transport branch, stores the passenger as the new cargo head at `Cargo+0x04`, resets `Cargo+0x00`, and walks `Object+0x30` links while bit 2 of `Object+0x14` remains set to recompute count.

Handoff-critical evidence:

| Behavior | Evidence |
|---|---|
| Branch gate `WhatAmI()==2`, index != `-1`, count != 0 | Assembly `0x0065E79B..0x0065E7B8`. |
| Set carrier `+0x6C9=1` | Assembly `0x0065E7B8`. |
| Infantry type lookup from `0x00A8E34C` | Assembly `0x0065E7BF..0x0065E7C5`. |
| Create passenger with owner-house argument | Assembly `0x0065E7CE..0x0065E7D8`. |
| No passenger `Unlimbo`; direct add to cargo | Assembly `0x0065E7E4..0x0065E7F5`. |
| Cargo head/count linking | `CargoClass__AddPassenger` decompile and assembly `0x004733FA..0x0047342C`. |

## 4. INI Keys

| INI key / section | Stock YR value | Effect in this slice | Active in YR |
|---|---|---|---|
| `[ParaDropSpecial] Type` | `ParaDrop` | Enables generic launch case 5. | Yes; `rulesmd.ini:30952..30962`. |
| `[AmericanParaDropSpecial] Type` | `AmerParaDrop` | Enables American launch case 6. | Yes; `rulesmd.ini:30967..30977`. |
| `[General] AmerParaDropInf/Num` | `E1` / `8` | Passenger type/count for case 6. | Yes; `rulesmd.ini:241..242`. |
| `[General] AllyParaDropInf/Num` | `E1` / `6` | Allied generic passenger type/count. | Yes; `rulesmd.ini:244..245`. |
| `[General] SovParaDropInf/Num` | `E2` / `9` | Soviet generic passenger type/count. | Yes; `rulesmd.ini:247..248`. |
| `[General] YuriParaDropInf/Num` | `INIT` / `6` | Yuri generic passenger type/count. | Yes; `rulesmd.ini:250..251`. |
| `[AircraftTypes] 7` | `PDPLANE` | `FUN_0041CAA0` name lookup finds the aircraft type index used as `EDX`. | Yes; `rulesmd.ini:1166`. |
| `[PDPLANE] Landable` | `no` | Relevant after spawn, but outside this slice. | Yes; `rulesmd.ini:11549`; post-spawn behavior not investigated here. |
| `[PDPLANE] Spawned` | `yes` | Content flag; the spawner still explicitly sets runtime carrier byte `+0x3D4=1`. | Yes; `rulesmd.ini:11544`; binary write at `0x0065E6BE`. |

## 5. Integration Points

The live entry points are `SuperClass::Launch` cases 5 and 6. Case 5 selects the generic per-side list by `House+0x1E8`; case 6 always uses the American list. Both require the PDPLANE type lookup to succeed and the infantry type's `+0xDF8` value to be non-`-1` before calling the spawner.

`FUN_0065E660` calls:

- aircraft type vtable `+0x8C` for carrier creation.
- `FUN_0050DA80` only for invalid primary edge.
- `FUN_004AA440` for the carrier spawn edge cell.
- carrier vtable `+0x1E8`, `+0x480`, optionally `+0x3C8`, and `+0xD8`.
- carrier vtable `+0x2C` for class check.
- infantry type vtable `+0x8C` for passenger creation.
- passenger vtable `+0xD4`.
- `CargoClass__AddPassenger`.
- carrier vtable `+0x1EC` after cargo loading.

Tick-cycle integration after `+0x1EC` is outside this report.

## 6. Current Rust Implementation Status

Current Rust surfaces after the 2026-05-22 paradrop fixes:

- `src/sim/superweapon/paradrop.rs` no longer aborts on invalid primary `waypoint_edge`; it currently falls back to north. This is closer than the old abort behavior, but still does not model the verified secondary `House+0x577C` fallback before north.
- `src/sim/superweapon/paradrop.rs` routes carrier edge selection through `find_paradrop_carrier_edge_cell`, a paradrop-specific helper that does not use ordinary ground path-grid passability as the spawn oracle.
- Carrier creation still does not explicitly model every `g_MapEditorMode` side effect; a full silent-spawn side-effect audit remains separate.
- Passengers are now created in limbo for paradrop cargo loading rather than spawned/unlimbo'd on the edge cell and then removed from occupancy.
- `src/sim/passenger.rs` now exposes forced paradrop loading; standard `PassengerCargo::board` capacity/size limits are not used for this SW payload path.
- Passenger loading now avoids transient map occupancy during cargo setup, matching the binary finding that only the carrier is unlimbo'd by this spawner.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SuperClass::Launch` cases 5/6 -> `FUN_0065E660` call sites | verified | `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655`; `rulesmd.ini` active SW definitions | none for spawner args |
| `FUN_0065E660` carrier create/unlimbo | verified | decompile; assembly `0x0065E691..0x0065E795` | full carrier class internals outside scope |
| Invalid `House+0x1E0` edge fallback | verified | decompile `FUN_0050DA80`; assembly `0x0065E6C5..0x0065E6D6` | none |
| `FUN_004AA440` spawner call arguments | verified | assembly `0x0065E6DB..0x0065E6F6` | generic modes not used by spawner not exhaustively covered |
| `FUN_004AAB30` criterion-4 fast accept | verified | decompile; assembly `0x004AAB3D..0x004AAB4B` | none for this call |
| Passenger creation and cargo linking | verified | assembly `0x0065E7A7..0x0065E7F5`; `CargoClass__AddPassenger` decompile/assembly | exact semantic name of vtable `+0xD4` deferred |
| `CargoClass__AddPassenger` list/count behavior | verified | decompile; assembly `0x004733A0..0x0047342C` | nested/open-transport branch semantics out of scope |
| Global side-effect inventory of `g_MapEditorMode` | touched-not-exhausted | spawner writes verified; prior report lines 824..845 | full 50+ xref audit would require separate verify-doc/re-investigate |
| Aircraft mission behavior after spawn | deferred | explicit non-scope | covered by other paradrop slots |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `FUN_0065E660` on a live YR paradrop path? -> Yes; cases 5/6 in `SuperClass::Launch` call it and stock `rulesmd.ini` defines both superweapon types.` (evidence: `0x006CD421`, `0x006CD655`, `rulesmd.ini:30952..30977`)
- `[RESOLVED] OQ-2 - What are the exact paradrop call-site arguments? -> `ECX=House`, `EDX=PDPLANE index`, stack `1, 0x1A, target, 0, infantry_index, passenger_count`.` (evidence: `0x006CD3F1..0x006CD421`, `0x006CD625..0x006CD655`)
- `[RESOLVED] OQ-3 - Does invalid primary `WaypointEdge` abort? -> No; it calls `FUN_0050DA80` and falls back through `House+0x577C`, then `0`.` (evidence: `0x0065E6C5..0x0065E6D6`, `0x0050DA80`)
- `[RESOLVED] OQ-4 - Is the edge finder called on House or Map? -> Map singleton `0x0087F7E8`; house only supplies the edge.` (evidence: `0x0065E6F0`)
- `[RESOLVED] OQ-5 - What edge-finder arguments are used? -> output local, edge, sentinel, sentinel, `4`, `1`, `0`.` (evidence: `0x0065E6DB..0x0065E6F6`)
- `[RESOLVED] OQ-6 - Does this edge-finder call require ordinary passability? -> No; `FUN_004AAB30` returns true immediately for criterion `4`.` (evidence: `0x004AAB3D..0x004AAB4B`)
- `[RESOLVED] OQ-7 - Does the spawner check edge finder result for sentinel before unlimbo? -> No explicit sentinel abort in the spawner; failure is handled by carrier `Unlimbo`.` (evidence: `FUN_0065E660` decompile; `0x0065E6F6..0x0065E77B`)
- `[RESOLVED] OQ-8 - Which operations are wrapped in `g_MapEditorMode`? -> Carrier `CreateObject` and carrier `Unlimbo` only in this function.` (evidence: `0x0065E691..0x0065E6B2`, `0x0065E73F..0x0065E78F`)
- `[RESOLVED] OQ-9 - Are passengers Unlimbo'd? -> No; the passenger path creates objects and immediately links them into cargo.` (evidence: `0x0065E7D8..0x0065E7F5`)
- `[RESOLVED] OQ-10 - Are passengers created with the owning house? -> Yes; the owner house pointer saved from `ECX` is pushed into infantry type vtable `+0x8C`.` (evidence: `0x0065E7CE..0x0065E7D8`)
- `[RESOLVED] OQ-11 - Does cargo loading enforce Rust-style capacity/size? -> Not in this path; `CargoClass::AddPassenger` links and recomputes count, with no `Passengers=`/`SizeLimit=` gate visible.` (evidence: `0x004733A0..0x0047342C`)
- `[RESOLVED] OQ-12 - What happens if carrier creation fails? -> Return current spawned count.` (evidence: `FUN_0065E660` decompile; null branch after `0x0065E6B0`)
- `[RESOLVED] OQ-13 - What happens if carrier unlimbo fails? -> Delete carrier and return `spawned_count - 1`.` (evidence: `FUN_0065E660` decompile; branch around `0x0065E77B..0x0065E795`)
- `[RESOLVED] OQ-14 - Does zero passenger count load cargo? -> No; branch requires passenger count nonzero.` (evidence: `0x0065E7B0..0x0065E7B6`)
- `[RESOLVED] OQ-15 - Does invalid infantry index load cargo? -> No; branch requires index != `-1`.` (evidence: `0x0065E7A7..0x0065E7AE`)
- `[RESOLVED] OQ-16 - Is this TS legacy? -> No for stock paradrop; active stock YR superweapons and buildings grant these paths.` (evidence: `rulesmd.ini:12362`, `rulesmd.ini:13924`, `rulesmd.ini:30952..30977`)
- `[DEFERRED] OQ-17 - Exactly which non-audio side effects does `g_MapEditorMode` suppress globally?` (category: `requires-different-system-context`; reason: this slice verified the spawner's write windows but not all global xrefs; next-step-if-pursued: run a dedicated global `MapEditorMode` xref audit or verify-doc pass on prior report section 24)
- `[DEFERRED] OQ-18 - Exact semantic name of passenger vtable `+0xD4`.` (category: `bounded-cost-too-high`; reason: ownership is proven by the creation argument and the call is not needed to implement cargo-link behavior; next-step-if-pursued: trace Object/Techno vtable slot map)

Adversarial corner cases answered:

- Invalid primary edge but valid fallback: use fallback, do not abort.
- Invalid primary and invalid fallback: use edge 0.
- No path-grid-walkable edge cells: binary spawner's criterion-4 call is not a path-grid-walkable filter.
- Passenger creation failure mid-loop: the loop counter still decrements; successful prior passengers remain cargo.
- First carrier `Unlimbo` failure: return can be `-1`; caller does not use this as a success count in the inspected paradrop path.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Invalid `House+0x1E0` falls back to `House+0x577C`, then `0`; launch does not abort. Active in YR: Yes. | `0x0065E6C5..0x0065E6D6`; `FUN_0050DA80` | partial: Rust now falls back invalid `waypoint_edge` to north, but does not model secondary `+0x577C` | `src/sim/superweapon/paradrop.rs`; likely `HouseState` needs fallback edge field or computed substitute | Resolve invalid waypoint edge through the verified fallback chain. | `paradrop_invalid_waypoint_edge_uses_secondary_edge`; `paradrop_invalid_waypoint_and_secondary_edge_defaults_north` | Do not treat invalid edge as launch failure. |
| Spawner edge lookup calls map-edge helper with criterion `4`, which bypasses ordinary passability/object checks. Active in YR: Yes. | `0x0065E6DB..0x0065E6F6`; `0x004AAB3D..0x004AAB4B` | mostly matched: Rust now routes to `find_paradrop_carrier_edge_cell`; exact south-edge RNG/candidate behavior remains separately tracked | `src/sim/world/edge_cell.rs`; `src/sim/superweapon/paradrop.rs` | Keep using the paradrop carrier edge-cell resolver instead of normal passability. | `paradrop_edge_spawn_ignores_ground_pathgrid_blockers`; `paradrop_edge_spawn_does_not_abort_when_edge_ground_cells_blocked` | Do not reuse ground-unit passability as the PDPLANE spawn oracle. |
| Carrier `CreateObject` and carrier `Unlimbo` are wrapped in `g_MapEditorMode`; passengers are not unlimbo'd. Active in YR: Yes. | `0x0065E691..0x0065E6B2`; `0x0065E73F..0x0065E78F`; passenger path `0x0065E7D8..0x0065E7F5` | partial/matched for passengers: Rust now creates passengers in limbo; full carrier silent lifecycle parity remains separate | `src/sim/world/world_spawn.rs`; `src/sim/superweapon/paradrop.rs` | Keep limbo-only passenger creation/cargo-link path; audit carrier spawn side effects separately if needed. | `paradrop_carrier_spawn_emits_no_lifecycle_audio_or_radar_event`; `paradrop_passengers_loaded_without_edge_occupancy` | Do not create passengers on the map even briefly; it can perturb occupancy, subcell allocation, reveal/audio hooks, and deterministic event order. |
| Passenger count is exactly the `*ParaDropNum[i]` loop count; cargo add does not enforce `Passengers=` capacity or `SizeLimit=`. Active in YR: Yes. | call-site count pushes; `CargoClass__AddPassenger` `0x004733A0..0x0047342C` | matched by forced paradrop loading | `src/sim/passenger.rs`; `src/sim/superweapon/paradrop.rs` | Paradrop cargo loading should link exactly N passengers for the list entry, independent of PDPLANE `Passengers=`. | `paradrop_loads_num_passengers_even_when_pdplane_has_no_passengers_key`; `paradrop_cargo_size_limit_not_applied_to_superweapon_payload` | Do not model paradrop loading as normal transport boarding. |
| Mission `0x1A` and destination are set before carrier unlimbo. Active in YR: Yes. | `0x0065E6FF..0x0065E721`; `0x0065E73A..0x0065E77B` | partial: Rust sets mission/move after spawn | `src/sim/superweapon/paradrop.rs:228..245` | If lifecycle hooks ever observe spawn state, set mission/destination before publishing/unlimbo equivalent. | `paradrop_carrier_has_approach_target_when_spawn_lifecycle_runs` | Do not let a visible spawned carrier spend a tick in idle/no-destination state. |

Stale Docs / Follow-up Docs:

- Replace "edge_cell = FUN_004AA440(House, edge, sentinel, sentinel, 4, 1, 0)" with "edge_cell = FUN_004AA440(MapClass singleton `0x0087F7E8`, edge, sentinel, sentinel, 4, 1, 0); the house supplies only the edge."
- Add: "For this spawner call, criterion `4` makes `FUN_004AAB30` immediately accept candidates, so this is not a normal path-grid passability search."
- Replace any implication that paradrop passengers are unlimbo'd at the carrier edge with: "passengers are created in limbo and linked directly into `CargoClass`; only the carrier is unlimbo'd."

## 10. Negative Facts / Do Not Do

- Do not abort paradrop launch on invalid `waypoint_edge`.
- Do not use ground pathfinding walkability as the PDPLANE carrier edge-spawn acceptance rule for this call.
- Do not spawn passenger infantry into map occupancy and then remove them; the binary never unlimbo's them during loading.
- Do not enforce PDPLANE `Passengers=` or normal transport `SizeLimit=` while loading superweapon paradrop cargo.
- Do not assume the `+0x3D4` byte means the carrier is literally parachuting; it is set on the aircraft immediately after creation.
- Do not extend this report to drop cadence, overfly exit, or parachute descent; those are separate slices.

## 11. Remaining Uncertainty

- Full `g_MapEditorMode` side-effect inventory was not re-audited here. The spawner's suppression windows are verified; the complete global meaning should be covered by a dedicated verify-doc pass if implementation needs exact event suppression categories.
- The exact semantic name of passenger vtable slot `+0xD4` remains unresolved. It is called both just before `AddPassenger` and inside `AddPassenger`; implementation should preserve the observable cargo-link/ownership result rather than copy a speculative name.
- Generic `FUN_004AA440` modes outside the spawner's `edge, sentinel, sentinel, 4, 1, 0` call remain out of scope.

## 12. Concrete Rust Test Name Proposals

- `paradrop_invalid_waypoint_edge_uses_secondary_edge`
- `paradrop_invalid_waypoint_and_secondary_edge_defaults_north`
- `paradrop_edge_spawn_ignores_ground_pathgrid_blockers`
- `paradrop_edge_spawn_does_not_abort_when_edge_ground_cells_blocked`
- `paradrop_passengers_loaded_without_edge_occupancy`
- `paradrop_loads_num_passengers_even_when_pdplane_has_no_passengers_key`
- `paradrop_cargo_size_limit_not_applied_to_superweapon_payload`
- `paradrop_carrier_spawn_emits_no_lifecycle_audio_or_radar_event`
- `paradrop_carrier_has_approach_target_when_spawn_lifecycle_runs`

## Sources

- Ghidra decompiled/read-only: `FUN_0065E660`, `FUN_0050DA80`, `FUN_004AA440`, `FUN_004AAB30`, `CargoClass__AddPassenger`, `SuperClass__Launch`, `FUN_0041CAA0`.
- Ghidra assembly contexts: `0x0065E691..0x0065E6B2`, `0x0065E6C5..0x0065E6F6`, `0x0065E73F..0x0065E7F5`, `0x004AAB3D..0x004AAB4B`, `0x004733A0..0x0047342C`, `0x006CD3F1..0x006CD421`, `0x006CD463..0x006CD493`, `0x006CD4BB..0x006CD4EB`, `0x006CD625..0x006CD655`.
- Prior report referenced: `docs/research/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`.
- INI sources checked: `ini/rulesmd.ini` `[General]` paradrop lists, `[PDPLANE]`, `[ParaDropSpecial]`, `[AmericanParaDropSpecial]`; `ini/artmd.ini` `[PDPLANE]`.
- Rust surfaces scanned: `src/sim/superweapon/paradrop.rs`, `src/sim/world/edge_cell.rs`, `src/sim/passenger.rs`, `src/sim/world/world_spawn.rs`.
