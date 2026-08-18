# Garrison CanDock Entry Gates Reswarm - 2026-05-27

**Address(es):** `0x00457CE0` (`BuildingClass::CanDock`), live callers `0x00519630` (`InfantryClass::PerCellProcess`) and `0x0051E3B0` (`InfantryClass::What_Action_OnObject`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active Yuri's Revenge civilian occupied-building entry predicates in `BuildingClass::CanDock` and current Rust comparison against `src/sim/passenger.rs::can_enter_transport` / `can_dock_occupier_garrison`.
**Non-Scope:** garrison ejection, fire, render, bunker, `CanGarrison` gate passability except stale-doc wording, and broad passenger/transport behavior.
**Confidence:** High for `CanDock` gates and Rust comparison; Medium for exact Rust state-name equivalence to native mission/state fields.
**Active in YR:** Yes for the function and live player/order arrival paths; Conditional for INI/type-controlled flags such as `CanBeOccupied`, `Occupier`, `Assaulter`, and `MultiplayPassive`.

## Working Notes Gate

- Target question: Which active `BuildingClass::CanDock` predicates decide whether infantry may enter `CanBeOccupied` buildings, and does current Rust `src/sim/passenger.rs` match them?
- Non-goals: no ejection, fire, render, bunker, broad passenger transport, or `CanGarrison` gate-passability re-study.
- Evidence needed to mark COMPLETE: fresh read-only Ghidra decompile plus assembly/range evidence for `CanDock` and adjacent live callers, INI/default evidence for entry keys, current Rust scan with line references, and implementation handoff.
- Stop conditions: no new scoped open questions after a final pass over `CanDock`, live callers, `AddGarrisonOccupant`, red-health, mind-control, and current Rust entry helpers; write only this report plus shared claims file.

## 1. Overview

`BuildingClass::CanDock @ 0x00457CE0` is the active normal occupied-building entry predicate. It is reached before the actual insertion path when infantry arrives at its target building and is also consulted by human cursor/action code for building targets.

The current Rust entry helper now matches many important `CanDock` gates: `CanBeOccupied`, building-up/down state proxy, playfield bounds when a grid is available, `Occupier`, same-owner or owner-country `MultiplayPassive`, equality capacity rejection, red-health rejection, and target mind-control rejection. The remaining Rust-facing gaps in this slice are the native `Assaulter` alternate branch and the exact identity/equivalence of the native vtable `+0x1D4` state predicate.

## 2. Key Fields And Predicates

| Predicate / field | Native behavior | Evidence | Active in YR |
|---|---|---|---|
| Non-null infantry argument | Null passenger argument returns false before type reads | decompile `0x00457CE0`; assembly context `0x00457CEB..0x00457D01` | Yes |
| `CanBeOccupied` | `BuildingType+0x157B` must be nonzero | decompile `0x00457CE0`; `rulesmd.ini` has stock occupied buildings such as `CAGAS01` with `CanBeOccupied=yes` | Conditional on INI |
| Construction/selling missions | building mission values `0x12` and `0x13` reject before owner/capacity | decompile `0x00457CE0`; assembly range `0x00457D01..0x00457D18` | Yes |
| In-playfield coordinate | building coordinate from vtable `+0x48` must pass `MapClass::IsCoordsInPlayfield` | decompile `0x00457CE0`; assembly range `0x00457D1A..0x00457D34` | Yes |
| vtable `+0x1D4` state gate | `CanDock` requires this virtual to return false | decompile `0x00457CE0`; assembly range `0x00457D36..0x00457D46` | Yes, exact state name not re-proven here |
| `Occupier` branch | normal entry requires `InfantryType+0xEB4 != 0` | decompile `0x00457CE0`; `rulesmd.ini:3720`, `4335`, `4877` | Conditional on infantry type |
| Owner permission | same house OR target owner country `+0x1A6` (`MultiplayPassive`) | decompile `0x00457CE0`; assembly `0x00457D58..0x00457D73`; `rulesmd.ini:3343`, `3351` | Yes for neutral/special stock houses |
| Capacity bound | rejects only when occupant count equals `MaxNumberOccupants`; comparison is equality, not `>=` | decompile `0x00457CE0`; assembly `0x00457D79..0x00457D8B` | Yes |
| Red-health gate | `ObjectClass::IsRedHP` blocks entry; helper returns true only for positive health and `health / max <= ConditionRed` | decompile `0x005F5CD0`; call at `0x00457D8F`; `rulesmd.ini:752` | Yes |
| Mind-control gate | `TechnoClass::IsMindControlled` blocks if `Techno+0x2C0` pointer or `+0x2C4` byte is nonzero | decompile `0x007105E0`; call at `0x00457D9A`; assembly `0x007105E0..0x007105FC` | Yes |
| `Assaulter` alternate branch | non-`Occupier` infantry may return true only if `Assaulter`, not allied to building, and building already has occupants | decompile `0x00457CE0`; assembly `0x00457DAD..0x00457DD3`; stock checked examples are `Assaulter=no` | Conditional |

## 3. Core Logic

`CanDock` first proves the target is a valid, active `CanBeOccupied` building: passenger pointer non-null, building type occupiable, mission not `0x12` or `0x13`, coordinates in playfield, and vtable `+0x1D4` false. Active in YR: Yes. Evidence: decompile `0x00457CE0`; assembly ranges `0x00457CEB..0x00457D46`; live arrival caller `0x00519630`.

For normal garrison entry, `Occupier=yes` selects the main branch. That branch accepts only if the infantry owns the building or the building owner's country has `MultiplayPassive`, occupant count is not exactly equal to `MaxNumberOccupants`, the building is not red HP, and the building is not mind-controlled. Active in YR: Yes for stock occupier infantry and neutral/special stock civilian owners. Evidence: decompile `0x00457CE0`; assembly ranges `0x00457D48..0x00457DA1`; INI `rulesmd.ini:3343`, `3351`, `3720`, `4335`, `4877`, `752`.

The capacity detail is load-bearing: the native compare is equality. A valid normal state reaches equality when full, but a corrupted over-capacity state would not be rejected by capacity alone in this helper. Active in YR: Yes. Evidence: vtable `+0x408` occupant-count call and compare against `BuildingType+0x1580` at `0x00457D79..0x00457D8B`.

`Assaulter=yes` is not the normal entry path. If `Occupier` is false, `CanDock` can still return true only when `Assaulter` is true, the infantry is not allied with the building, and the building has at least one occupant. Active in YR: Conditional; branch is live code, stock checked examples remain `Assaulter=no`. Evidence: decompile `0x00457CE0`; assembly `0x00457DAD..0x00457DD3`; INI examples `rulesmd.ini:4028`, `4079`, `4513`, `4604`.

`InfantryClass::PerCellProcess @ 0x00519630` is the commit-side live caller for the normal arrival path. It requires mission `8`, `Occupier` or `Assaulter`, a building target of type `6`, current cell's building equal to target, calls `CanDock`, redirects if false, and calls `BuildingClass::AddGarrisonOccupant @ 0x00522910` if true. Active in YR: Yes. Evidence: decompile `0x00519630`; assembly range `0x00519680..0x00519734`.

`AddGarrisonOccupant` is after the gate and does not re-run every `CanDock` predicate. For `Occupier`, it limbos the infantry, appends it to the building occupant vector, recalculates power, and plays first-occupant events. For `Assaulter`, it takes a different parachute/redirect branch. Active in YR: Yes. Evidence: decompile `0x00522910`; assembly range `0x00522910..0x00522A0F`.

## 4. INI Keys

| Key | Default / stock evidence | Binary effect | Active in YR |
|---|---|---|---|
| `CanBeOccupied=` | stock civilian buildings set it, e.g. `rulesmd.ini:19322` | required by `CanDock` at `BuildingType+0x157B` | Conditional |
| `MaxNumberOccupants=` | stock values include `10`, `5`, `3`, `1`; e.g. `rulesmd.ini:19323`, `19350`, `19377`, `21240` | equality compare against occupant count | Conditional |
| `Occupier=` | stock infantry examples at `rulesmd.ini:3720`, `4335`, `4877` | selects normal entry branch | Conditional |
| `Assaulter=` | checked stock examples are `no`, e.g. `rulesmd.ini:4028`, `4079`, `4513`, `4604` | alternate non-allied occupied-building branch | Conditional |
| `MultiplayPassive=` | `[Neutral]` and `[Special]` stock entries at `rulesmd.ini:3343`, `3351` | allows different-owner entry through owner country | Yes for standard neutral/special houses |
| `ConditionRed=` | `rulesmd.ini:752` is `25%` | `ObjectClass::IsRedHP` threshold via rules singleton | Yes |

## 5. Current Rust Implementation Status

Current Rust entry points scanned with Codegraph and file reads:

- `src/sim/passenger.rs:188` `can_enter_transport`
- `src/sim/passenger.rs:233` `can_dock_occupier_garrison`
- `src/sim/passenger.rs:278` `owner_country_multiplay_passive`
- `src/sim/passenger.rs:528` `is_at_or_below_red_hp`
- tests at `src/sim/passenger.rs:1476`, `1495`, `1525`, `1547`, `1575`, `1601`, `1615`, `1629`

Matched or mostly matched:

- `CanBeOccupied` gate: Rust checks `building_obj.can_be_occupied` before garrison gates. Active in YR: Yes; evidence `0x00457CF3..0x00457D01`.
- Construction/sell-like proxy: Rust rejects `building.building_up` or `building_down`; this is directionally aligned with native mission `0x12`/`0x13`, but exact equivalence of the Rust states to the mission field remains a handoff risk. Active in YR: Yes; evidence `0x00457D01..0x00457D18`.
- Playfield: Rust rejects out-of-grid only when a `PathGrid` is supplied; native always checks global map playfield. Active in YR: Yes; evidence `0x00457D1A..0x00457D34`.
- `Occupier`: Rust requires `passenger_obj.occupier`. Active in YR: Yes/Conditional by type; evidence `0x00457D48..0x00457D56`.
- Owner: Rust now uses the building owner's country `MultiplayPassive`, not hardcoded owner strings. Active in YR: Yes; evidence `0x00457D58..0x00457D73`.
- Capacity: Rust `can_dock_occupier_garrison` checks `cargo.count() == max_number_occupants`, matching the native equality boundary for this helper. Active in YR: Yes; evidence `0x00457D79..0x00457D8B`.
- Red HP: Rust rejects at or below `ConditionRed`; the outer `can_enter_transport` alive check prevents dead-building entry, which preserves the native positive-health condition for this entry path. Active in YR: Yes; evidence `0x005F5CD0`.
- Mind-control: Rust rejects `building.mind_controlled`, matching the binary's conceptual gate if the field is maintained from `Techno+0x2C0/+0x2C4` equivalents. Active in YR: Yes; evidence `0x007105E0..0x007105FC`.

Observed deltas / risks:

- Rust does not implement the `Assaulter` alternate `CanDock` true branch. This may be acceptable if stock merged YR data keeps it inactive, but the native branch is live for mods or any active `Assaulter=yes` infantry.
- Rust has no clearly named equivalent for native vtable `+0x1D4` in `CanDock`; current `building_up/building_down` does not prove this gate is covered.
- Rust's playfield check is optional by `path_grid: Option<&PathGrid>`; native `CanDock` always consults the playfield.
- Rust comments above `can_enter_transport` are stale/narrow: they mention red-health but omit current implemented gates such as building-up/down, `MultiplayPassive`, capacity equality, playfield, and mind-control.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::CanDock @ 0x00457CE0` top-level gates | verified | decompile `0x00457CE0`; assembly `0x00457CEB..0x00457D46` | exact name of vtable `+0x1D4` gate |
| `Occupier` normal branch | verified | decompile `0x00457CE0`; assembly `0x00457D48..0x00457DA1`; INI lines above | none for branch predicates |
| Capacity equality | verified | `0x00457D79..0x00457D8B` | none |
| Red HP helper | verified | decompile `0x005F5CD0`; call `0x00457D8F`; `rulesmd.ini:752` | none for entry path |
| Mind-control helper | verified | decompile `0x007105E0`; call `0x00457D9A`; assembly `0x007105E0..0x007105FC` | exact Rust maintenance of `mind_controlled` out-of-scope |
| `Assaulter` alternate branch | verified | `0x00457DAD..0x00457DD3`; INI examples | full merged stock audit of active `Assaulter=yes` |
| Arrival caller `InfantryClass::PerCellProcess` | verified | decompile `0x00519630`; assembly `0x00519680..0x00519734` | no broader mission system claims |
| Cursor/action caller `What_Action_OnObject` | touched-not-exhausted | decompile `0x0051E3B0` contains `CanDock` call and action `9` return | full cursor action matrix out-of-scope |
| Current Rust `src/sim/passenger.rs` | verified for scoped comparison | Codegraph context and file lines listed above | no Rust edits made |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `CanDock` active in YR entry flow? -> Yes, `PerCellProcess` calls it before `AddGarrisonOccupant` on the mission-8 building-arrival path.` (evidence: `0x00519680..0x00519734`)
- `[RESOLVED] OQ-2 - Does `CanDock` require `CanBeOccupied`? -> Yes, it reads `BuildingType+0x157B` and rejects false.` (evidence: `0x00457CF3..0x00457D01`)
- `[RESOLVED] OQ-3 - Does construction/selling state block entry? -> Yes, mission values `0x12` and `0x13` reject before owner/capacity checks.` (evidence: `0x00457D01..0x00457D18`)
- `[RESOLVED] OQ-4 - Is playfield checked? -> Yes, building coords must pass `MapClass::IsCoordsInPlayfield`.` (evidence: `0x00457D1A..0x00457D34`)
- `[RESOLVED] OQ-5 - Is the owner exception hardcoded Neutral/Special? -> No, native reads target owner country `MultiplayPassive`.` (evidence: `0x00457D58..0x00457D73`; `rulesmd.ini:3343`, `3351`)
- `[RESOLVED] OQ-6 - Is capacity `< max` or `!= max`? -> Native rejects only `count == max`.` (evidence: `0x00457D79..0x00457D8B`)
- `[RESOLVED] OQ-7 - Does red health block entry? -> Yes, via `ObjectClass::IsRedHP`, positive health and ratio `<= ConditionRed`.` (evidence: `0x005F5CD0`; `rulesmd.ini:752`)
- `[RESOLVED] OQ-8 - Does mind-control block entry? -> Yes, target `Techno+0x2C0` or `+0x2C4` nonzero blocks.` (evidence: `0x007105E0..0x007105FC`)
- `[RESOLVED] OQ-9 - Does Rust currently include the main normal-branch gates? -> Mostly yes: see `can_dock_occupier_garrison` lines `233..275`.` (evidence: `src/sim/passenger.rs:233`)
- `[DEFERRED] OQ-10 - What exact semantic name and all callers/writers define native vtable `+0x1D4`?` (category: requires-different-system-context; reason: this slot only needs to prove `CanDock` reads it; next-step-if-pursued: trace vtable binding and compare to Rust building state fields)
- `[DEFERRED] OQ-11 - Does fully merged stock YR contain any active `Assaulter=yes` infantry?` (category: bounded-cost-too-high; reason: branch is verified and checked examples are false, but a full merged object audit is outside this slot; next-step-if-pursued: parse all merged InfantryTypes)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Normal `Occupier` entry requires `CanBeOccupied`, not mission `0x12/0x13`, in-playfield, vtable `+0x1D4` false, same owner or `MultiplayPassive`, `count != max`, not red HP, not mind-controlled. | `0x00457CE0`; assembly `0x00457CEB..0x00457DA1`; `0x005F5CD0`; `0x007105E0`; `rulesmd.ini:3343/3351/752` | mostly matched, but vtable `+0x1D4` exact equivalent and always-on playfield check are not proven | `src/sim/passenger.rs::can_dock_occupier_garrison`; command/cursor callers that reuse it | Keep one CanDock-equivalent predicate and prove/encode the missing native state gate; ensure entry validation always has map/playfield context when the map exists. | GI enters healthy neutral `CAGAS01`; same fixture rejects full, red, mind-controlled, out-of-playfield, building-up, and building-down targets. | `garrison_candock_normal_branch_matches_native_gates` | Medium: a loose cursor/command predicate can issue impossible native orders. |
| `Assaulter` is a separate non-`Occupier` true branch requiring not allied and existing occupants. | `0x00457DAD..0x00457DD3`; `AddGarrisonOccupant @ 0x00522910` assaulter branch | missing | `src/sim/passenger.rs::can_dock_occupier_garrison`; future assault/clearing behavior surface | Either implement the branch if stock/mod data can activate it, or explicitly gate/document unsupported `Assaulter` parity. | Non-occupier `Assaulter=yes` infantry cannot enter empty enemy building but passes `CanDock` against occupied non-allied building. | `garrison_candock_assaulter_requires_enemy_occupied_building` | High for mods or if stock audit finds activation; do not collapse into normal `Occupier` storage. |
| Capacity rejection is equality, not `>=`; current normal-state fill still prevents overfill elsewhere. | `0x00457D79..0x00457D8B`; `0x00522910` appends after success | matched in `can_dock_occupier_garrison`; broader cargo helpers use `< capacity` | `src/sim/passenger.rs::PassengerCargo::can_accept`, `can_dock_occupier_garrison` tests | Preserve the CanDock equality check in the garrison helper; document any deliberate Rust safety clamp outside exact native corrupted-state behavior. | Synthetic over-capacity fixture records whether Rust intentionally differs outside valid native state. | `garrison_candock_capacity_uses_native_equality_boundary` | Low for normal play, but parity reports must not claim native uses `>=`. |

## 9. Negative Facts / Do Not Do

- Do not use `CanGarrison` as the civilian occupied-building entry validator. Active in YR: No for normal entry validation; evidence: normal arrival path calls `CanDock @ 0x00457CE0`, while prior verified report shows `CanGarrison @ 0x004525F0` is gate passability.
- Do not implement entry permission as literal owner names `Neutral`/`Special`. Active in YR: Yes; evidence: native reads target owner `House+0x34 -> Country+0x1A6` at `0x00457D58..0x00457D73`.
- Do not change the capacity gate to `count >= max` while claiming exact `CanDock` parity. Active in YR: Yes; evidence: equality compare at `0x00457D79..0x00457D8B`.
- Do not treat `Assaulter` as ordinary garrison storage. Active in YR: Conditional; evidence: `CanDock` has a separate branch at `0x00457DAD..0x00457DD3`, and `AddGarrisonOccupant @ 0x00522910` handles non-`Occupier`/`Assaulter` via a different parachute/redirect path.
- Do not add a chrono/warp rejection based on this `CanDock` slot. Active in YR: No evidence in scoped helper; evidence: the scoped rejection calls `TechnoClass::IsMindControlled @ 0x007105E0`, reading `+0x2C0/+0x2C4`.

## 10. Stale Docs / Follow-up Wording

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_SYSTEM_GHIDRA_REPORT.md` replacement wording for `0x007105E0` summary: "`0x007105E0` is used here as `TechnoClass::IsMindControlled`; it reads `Techno+0x2C0` and byte `+0x2C4`. Do not describe the `CanDock` rejection as a being-warped/chrono gate without separate evidence."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_SYSTEM_GHIDRA_REPORT.md` replacement wording for entry summary: "`BuildingClass::CanDock @ 0x00457CE0` requires `CanBeOccupied`, mission not `0x12/0x13`, in-playfield coordinates, vtable `+0x1D4` false, then either the normal `Occupier` branch or the separate `Assaulter` branch. Normal `Occupier` entry uses same owner or owner-country `MultiplayPassive`, capacity equality (`count != MaxNumberOccupants`), not red HP, and not mind-controlled."

## 11. Remaining Uncertainty

- Exact semantic name, writers, and Rust equivalent of native vtable `+0x1D4` remain deferred.
- Full merged-stock audit for active `Assaulter=yes` infantry remains deferred.
- Exact equivalence between Rust `building_up`/`building_down` and native mission values `0x12`/`0x13` is not proven in this slot.

## Sources

- Ghidra read-only decompile: `0x00457CE0`, `0x00519630`, `0x0051E3B0`, `0x00522910`, `0x005F5CD0`, `0x007105E0`.
- Ghidra read-only assembly/disassembly ranges: `0x00457CEB..0x00457DD3`, `0x00519680..0x00519734`, `0x00522910..0x00522A0F`, `0x005F5CD0..0x005F5D14`, `0x007105E0..0x007105FC`.
- Existing docs: `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`, `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md`, `GARRISON_SYSTEM_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/passenger.rs`, `src/rules/object_type.rs`.
