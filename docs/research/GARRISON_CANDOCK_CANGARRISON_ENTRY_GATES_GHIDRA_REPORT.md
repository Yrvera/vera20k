# Garrison CanDock / CanGarrison Entry Gates - Ghidra Research Report

**Address(es):** `0x00457CE0` (`BuildingClass::CanDock`), `0x004525F0` (`BuildingClass::CanGarrison`), scoped caller `0x0051BF90` (`InfantryClass::Can_Enter_Cell`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** player-visible garrison entry gates in `CanDock`, gate-style passability in `CanGarrison`, and directly coupled infantry `Can_Enter_Cell` result codes.
**Non-Scope:** bunker entry/exit lifecycle, garrison fire, occupant death/removal, sell/destruction ejection, full gate animation/state machine.
**Confidence:** High for the two target helper bodies and directly coupled callers; Medium for human-readable names of mission `0x18` and helper `0x004A51B0`.
**Active in YR:** Yes for normal infantry order/cell-arrival paths; Conditional for individual INI flags and gate state.

## 0. Working Notes Gate

- Target question: Which exact native gates decide whether infantry may enter a civilian garrison building, and how does that differ from the misleading `CanGarrison` helper?
- Non-goals: no bunker entry investigation except separation; no Rust edits; no re-opening fire/ejection/occupant-death systems.
- Evidence needed to mark COMPLETE: decompile plus assembly/xref evidence for both helpers, caller evidence for live YR paths, INI/default evidence for critical keys, and an implementation handoff.
- Stop conditions: every scoped gate resolved or deferred, zero-add Ghidra pass over the helpers/callers, and only this report plus `.swarm-claims.md` written.

## 1. Overview

`BuildingClass::CanDock @ 0x00457CE0` is the real civilian-garrison entry validator. It is reached by human cursor/action logic, infantry cell-arrival processing, AI target maintenance, and nearest-dock searches.

`BuildingClass::CanGarrison @ 0x004525F0` is not the civilian garrison validator. It is a gate-style building passability helper: non-`Gate=` buildings return true immediately; `Gate=yes` buildings only return true when the current mission is `0x18` and the `+0x350` gate-state helper reports open/enterable.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `BuildingType+0x157B` | building type | `CanBeOccupied=` required by `CanDock` | `0x00457CF3..0x00457D01`; parser/Rust key `CanBeOccupied` | Conditional on INI |
| `BuildingType+0x1580` | building type | `MaxNumberOccupants`; full check is equality, not `>=` | `0x00457D79..0x00457D8B`; `rulesmd.ini` many values | Conditional on INI |
| `InfantryType+0xEB4` | infantry type | `Occupier=` branch | `0x00457D48..0x00457D56`; `rulesmd.ini:3720`, `4335`, `4877` examples | Conditional on infantry type |
| `InfantryType+0xEB5` | infantry type | `Assaulter=` alternate path | `0x00457DAD..0x00457DD3`; `rulesmd.ini` has stock `Assaulter=no` lines | Conditional / normally false in checked stock examples |
| `Object+0xAC` | building instance | current mission; `0x12` and `0x13` reject `CanDock` | `0x00457D07..0x00457D18` | Yes |
| `Techno+0x21C` | object instance | owner/house pointer | owner comparisons at `0x00457D58..0x00457D73`, result-code path `0x0051C504..0x0051C516` | Yes |
| `House+0x34 -> Country+0x1A6` | owner country | `MultiplayPassive=true` allows neutral/special entry | `0x00457D68..0x00457D73`; `rulesmd.ini:3343`, `3351` | Yes for neutral/special houses |
| vtable `+0x408` | building | occupant count | `0x00457D75..0x00457D85`; caller `AddGarrisonOccupant` updates vector | Yes |
| `Techno+0x2C0/+0x2C4` | target object | mind-control link/flag checked by `0x007105E0` | `0x007105E0..0x007105FC`, call at `0x00457D9A` | Yes |
| `BuildingType+0x16B7` | building type | `Gate=` flag read by `CanGarrison` and infantry cell entry | `0x004525F3..0x00452601`, `0x0051C4EB..0x0051C4F7`; `rulesmd.ini:17204` | Conditional on INI |
| `Building+0x350` | building instance | gate-state object/helper input | `0x00452616..0x00452623`, helper `0x004A51B0` reads `+0x18/+0x19` | Conditional on gate state |

## 3. Core Logic

### `BuildingClass::CanDock @ 0x00457CE0`

The function rejects immediately unless all top-level structural gates pass: non-null infantry argument, `CanBeOccupied` set, mission is neither `0x12` nor `0x13`, building coordinates are in the playfield, and vtable `+0x1D4` reports false. Active in YR: Yes; caller `InfantryClass::PerCellProcess @ 0x00519630` calls it when an infantry reaches its target building. Evidence: decompile `0x00457CE0`; assembly contexts `0x00457CEB..0x00457D46`, `0x005196C8..0x0051972D`.

If `Occupier` is false, `CanDock` does not run the normal entry rules. It only returns true for the `Assaulter` path when the infantry is not allied with the building and the building already has at least one occupant. Active in YR: Conditional on `Assaulter=yes`; stock checked lines show several `Assaulter=no` defaults. Evidence: `0x00457DAD..0x00457DD3`.

If `Occupier` is true, the garrison entry rules are: same owner OR target owner country has `MultiplayPassive`, current occupant count is not exactly equal to `MaxNumberOccupants`, target is not red HP, and target is not mind-controlled. Active in YR: Yes for GI/Conscript-style stock occupiers and neutral/special buildings. Evidence: `0x00457D58..0x00457DA1`; `rulesmd.ini:3343`, `3351`, `3720`, `4335`, `4877`.

Important bounds detail: capacity rejection is equality (`count == max`) rather than `count >= max`. Active in YR: Yes. Evidence: vtable `+0x408` call and compare at `0x00457D79..0x00457D8B`. A corrupted over-capacity building would not be rejected by this helper on capacity alone.

Red health uses `ObjectClass::IsRedHP @ 0x005F5CD0`, which returns true when `Health / MaxStrength <= Rules+0x1708` and health is positive. Active in YR: Yes; standard YR `ConditionRed=25%` is in `[AudioVisual]` at `rulesmd.ini:752`. Evidence: decompile `0x005F5CD0`, call at `0x00457D8F`.

The last rejection is mind-control, not chrono warp. `TechnoClass::IsMindControlled @ 0x007105E0` reads `+0x2C0` pointer and `+0x2C4` byte; it does not read Rust's `being_warped_ticks`-style chrono state. Active in YR: Yes. Evidence: decompile and assembly `0x007105E0..0x007105FC`, caller `0x00457D98..0x00457DA1`.

`CanDock` does not transfer ownership. `InfantryClass::PerCellProcess` calls `BuildingClass::AddGarrisonOccupant @ 0x00522910` after `CanDock` succeeds; `AddGarrisonOccupant` inserts the infantry and plays first-occupant events but has no `ChangeOwner` call in its body. Active in YR: Yes. Evidence: caller sequence `0x005196D4..0x0051972D`; decompile `0x00522910`.

### `BuildingClass::CanGarrison @ 0x004525F0`

This helper is a gate passability predicate, not the main garrison-entry validator. If `BuildingType+0x16B7` (`Gate=`) is false, it returns true immediately. Active in YR: Yes as code; effect is conditional on caller using it for a building object. Evidence: decompile `0x004525F0`; assembly `0x004525F3..0x00452606`.

If `Gate=` is true, it calls vtable `+0x184` and requires current mission `0x18`. Only then does it call `FUN_004A51B0` on `Building+0x350`; that helper returns true only when byte `+0x18 == 0` and byte `+0x19 == 1`. Active in YR: Conditional on gate data/state. Evidence: `0x00452607..0x00452628`, helper `0x004A51B0..0x004A51C2`, caller list includes draw/body and mission-move users.

`InfantryClass::Can_Enter_Cell @ 0x0051BF90` calls `CanGarrison` only from its building-object branch after reading `BuildingType+0x16B7`. If `CanGarrison` succeeds, this branch does not upgrade the result code. If it fails, allied buildings upgrade to at least code `3`; enemy buildings require infantry action ability or return hard block `7`, otherwise upgrade to at least code `5`. Active in YR: Yes for infantry pathing against gate-style buildings. Evidence: decompile `0x0051BF90`; assembly `0x0051C4EB..0x0051C549`.

## 4. INI Keys

| Key | Stock evidence | Binary read/effect | Active in YR |
|---|---|---|---|
| `CanBeOccupied=` | many stock civilian buildings; e.g. `rulesmd.ini:13002`, `14108` | read at `BuildingType+0x157B`; required by `CanDock` | Conditional |
| `MaxNumberOccupants=` | common values 1/3/4/5/6/8/10; examples `rulesmd.ini:13003`, `18480`, `21240` | compared by equality against vtable `+0x408` occupant count | Conditional |
| `Occupier=` | `E1` `rulesmd.ini:3720`; `CONS`/other examples at `4335`, `4877`; `GGI` override `Occupier=no;yes` at `3870` | `InfantryType+0xEB4` selects normal garrison branch | Conditional |
| `Assaulter=` | stock checked examples are `no`, e.g. `rulesmd.ini:4028`, `4079`, `4513`, `4604` | `InfantryType+0xEB5`; only true when not allied and building already occupied | Conditional |
| `MultiplayPassive=` | `[Special] rulesmd.ini:3343`, `[Neutral] rulesmd.ini:3351` | owner country `+0x1A6`; allows different-owner entry | Yes for neutral/special |
| `Gate=` | `rulesmd.ini:17204` | `BuildingType+0x16B7`; gates `CanGarrison` | Conditional |
| `ConditionRed=` | `[AudioVisual] rulesmd.ini:752` = `25%` | `Rules+0x1708`; red HP blocks `CanDock` | Yes |

## 5. Integration Points

`InfantryClass::What_Action_OnObject @ 0x0051E3B0` calls `CanDock` for human infantry targeting building objects and returns action `9` when it succeeds. Active in YR: Yes for cursor/order feedback. Evidence: decompile `0x0051E3B0`, block calling `0x00457CE0`.

`InfantryClass::PerCellProcess @ 0x00519630` is the actual entry commit path. For mission `8`, it checks `Occupier` or `Assaulter`, validates the current cell's building equals the target, calls `CanDock`, redirects if false, and calls `AddGarrisonOccupant` if true. Active in YR: Yes. Evidence: `0x0051968E..0x0051972D`.

`InfantryClass::Can_Enter_Cell @ 0x0051BF90` uses `CanGarrison` only for passability/result-code classification around `Gate=` buildings. This is separate from actual garrison boarding. Active in YR: Yes for infantry pathing. Evidence: `0x0051C4EB..0x0051C549`; caller list for `0x004525F0`.

`CanDock` also has non-player automation callers (`FootClass::Find_Nearest_Dock @ 0x004DFCB0`, `FUN_004DFE00`, `TechnoClass::AI_Update @ 0x006F9E50`). Active in YR: Yes. Evidence: Ghidra caller list for `0x00457CE0`.

## 6. Current Rust Implementation Status

Rust already has first-class garrison flags and capacity parsing: `src/rules/object_type.rs` parses `CanBeOccupied`, `MaxNumberOccupants`, `Occupier`, and `Assaulter`.

Entry validation lives mostly in `src/sim/passenger.rs::can_enter_transport` and `src/sim/world/world_commands.rs::Command::EnterTransport`. It checks alive/dying state, same owner or owner string `neutral`/`special`, `Occupier`, red HP, and cargo capacity. Current deltas: no building mission `0x12/0x13` equivalent gate, no target `vtable+0x1D4` deploying gate, no mind-control gate on the target building, and neutral permission is hardcoded by owner name rather than `MultiplayPassive`.

Cursor/order selection in `src/app_cursor.rs` and `src/app_context_order.rs` uses `CanBeOccupied`, selected `Occupier`, friendly structure or neutral/special owner. Current delta: it does not include full `CanDock` gates, so it can show/issue enter for red/full/building-state/mind-controlled targets until the sim rejects or accepts differently.

Pathing result-code logic is in `src/sim/pathfinding/cell_entry.rs` and `src/sim/movement/movement_occupancy.rs`. Vehicle row-helper exceptions are already category-gated to `EntityCategory::Unit`, matching the "do not apply bunker/row helper to infantry" constraint. Current delta: no obvious modeled `Gate=`/`CanGarrison` passability branch producing infantry codes `3`, `5`, `7` for gate-style buildings.

Rust currently transfers neutral garrison ownership immediately in `src/sim/passenger.rs` after boarding. Binary evidence in this slice only proves `CanDock`/`AddGarrisonOccupant` do not transfer immediately; prior garrison docs say ownership is reconciled later. Treat exact timing as a follow-up if visible owner-color or sidebar timing matters.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::CanDock @ 0x00457CE0` | verified | decompile; assembly contexts `0x00457CEB..0x00457DD5` | none for scoped gates |
| `Occupier` normal branch | verified | `0x00457D48..0x00457DA1`; INI examples | none |
| `Assaulter` alternate branch | verified | `0x00457DAD..0x00457DD3`; INI examples | exact stock unit activation beyond checked no-default examples deferred |
| Red HP gate | verified | `0x005F5CD0`, call `0x00457D8F`; `rulesmd.ini:752` | none |
| Mind-control vs warp gate | verified | `0x007105E0..0x007105FC`, call `0x00457D9A` | no chrono-warp reader found in scoped helper |
| `CanGarrison @ 0x004525F0` | verified | decompile; assembly `0x004525F3..0x00452628` | full gate mission state-machine not in scope |
| `InfantryClass::Can_Enter_Cell` result-code coupling | verified | `0x0051C4EB..0x0051C549` | runtime gate-open/closed fixture not traced |
| Bunker entry | deferred | user non-scope; infantry branch does not use bunker row helper here | slot 5 / separate bunker lifecycle report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is CanDock the civilian garrison entry validator? -> Yes, live order/cell-arrival callers invoke it before AddGarrisonOccupant.` (evidence: `0x00457CE0`, `0x005196D4..0x0051972D`)
- `[RESOLVED] OQ-2 - Is CanGarrison the civilian garrison entry validator? -> No; it checks `Gate=` and gate state only.` (evidence: `0x004525F0`, `0x004525F3..0x00452628`)
- `[RESOLVED] OQ-3 - What capacity comparison is used? -> equality, `count == MaxNumberOccupants`.` (evidence: `0x00457D79..0x00457D8B`)
- `[RESOLVED] OQ-4 - Does red HP block entry? -> Yes, via `ObjectClass::IsRedHP` and `Rules+0x1708`.` (evidence: `0x00457D8F`, `0x005F5CD0`, `rulesmd.ini:752`)
- `[RESOLVED] OQ-5 - Does CanDock check chrono warp? -> No scoped evidence; it checks mind-control `+0x2C0/+0x2C4` via `0x007105E0`.` (evidence: `0x00457D9A`, `0x007105E0..0x007105FC`)
- `[RESOLVED] OQ-6 - Are neutral/civilian buildings allowed by owner name or a house flag? -> Binary uses owner country `MultiplayPassive +0x1A6`.` (evidence: `0x00457D68..0x00457D73`, `rulesmd.ini:3343`, `3351`)
- `[RESOLVED] OQ-7 - Does AddGarrisonOccupant transfer owner immediately? -> No in its body; it inserts occupant and first-occupant events only.` (evidence: `0x00522910`, call `0x0051972D`)
- `[RESOLVED] OQ-8 - What result codes couple to CanGarrison? -> true does not upgrade; false gives allied >=3, enemy no-action 7, enemy action >=5.` (evidence: `0x0051C4F7..0x0051C549`)
- `[DEFERRED] OQ-9 - What is the exact human-readable name and transition contract for mission `0x18` on gate buildings?` (category: requires-different-system-context; reason: this slot verifies `CanGarrison` gates, not full gate animation/state machine; next-step-if-pursued: trace stock gate open/close missions and `Building+0x350` writers)
- `[DEFERRED] OQ-10 - Is any stock YR infantry `Assaulter=yes` after all md/base merges?` (category: bounded-cost-too-high; reason: scoped binary path is verified and checked examples are `no`, but full merged object audit is outside this slot; next-step-if-pursued: parse merged rules for every InfantryType)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `CanDock` accepts normal garrison entry only for `CanBeOccupied`, not construction/selling/deploying, same owner or `MultiplayPassive`, not exactly full, not red HP, not mind-controlled. | `0x00457CE0`; assembly `0x00457CF3..0x00457DA1`; `0x007105E0`; INI `rulesmd.ini:3343/3351/752` | partial mismatch | `src/sim/passenger.rs::can_enter_transport`, `src/sim/world/world_commands.rs`, `src/app_cursor.rs`, `src/app_context_order.rs` | Centralize a CanDock-equivalent predicate and use it for cursor/order and command validation; replace owner-name neutral shortcut with house/country `MultiplayPassive`; add mission/deploying/mind-control gates when state exists. | GI can enter neutral CAGAS at yellow health with free capacity; cannot enter same building when full, red HP, selling/building, or mind-controlled; proposed test `garrison_candock_rejects_full_red_state_and_mind_controlled_targets` | Do not let cursor/order use a looser predicate than command execution. |
| Capacity rejection is `count == MaxNumberOccupants`, not `>=`; normal valid state never overfills because AddGarrisonOccupant inserts one after CanDock. | `0x00457D79..0x00457D8B`, `0x00522910` | likely stricter (`< capacity`) | `src/sim/passenger.rs::PassengerCargo::can_accept`, garrison validator tests | Keep ordinary Rust capacity safe, but document/test the native equality boundary if exact corrupted-state replay/parity is modeled. | Synthetic over-capacity garrison fixture documents chosen Rust behavior vs native equality; proposed test `garrison_capacity_boundary_documents_native_equality_check` | Do not claim native uses `>=` without noting the assembly compare is equality. |
| `CanGarrison` is gate passability: non-`Gate=` returns true; `Gate=yes` requires mission `0x18` and gate helper true; `Can_Enter_Cell` maps failed allied/enemy cases to codes 3/5/7. | `0x004525F0`; helper `0x004A51B0`; `0x0051C4EB..0x0051C549` | missing/unchecked | `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs`, building gate-state surface | Add an infantry-specific gate-building passability branch separate from civilian garrison boarding and vehicle bunker/row-helper logic. | Infantry pathing against closed allied gate returns scatter/soft-block code 3; closed enemy gate with weapon/action ability returns code 5; no-action infantry returns 7; proposed test `infantry_gate_can_garrison_result_codes_match_native` | Do not use `CanGarrison` to decide civilian building entry or bunker entry. |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_SYSTEM_GHIDRA_REPORT.md` replacement wording: "`BuildingClass::CanDock @ 0x00457CE0` is the main civilian-garrison entry validator. `BuildingClass::CanGarrison @ 0x004525F0` is a gate-style passability helper used by `Can_Enter_Cell`: non-`Gate=` buildings return true, while `Gate=yes` buildings require mission `0x18` and the `Building+0x350` gate helper to be open/enterable. Do not describe `CanGarrison` as the garrison entry validator."
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_SYSTEM_GHIDRA_REPORT.md` replacement wording for warp note: "`CanDock` calls `TechnoClass::IsMindControlled @ 0x007105E0`, which reads `Techno+0x2C0/+0x2C4`. This scoped helper does not prove a chrono-warp/being-warped gate."
- `C:/Users/enok/Documents/ra2-rust-game-docs/INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md` remains directionally correct; add precision: "`CanGarrison` true for non-`Gate=` is a helper default, but the infantry caller only reaches it under its `BuildingType+0x16B7` branch in the verified result-code path."

## Negative Facts / Do Not Do

- Do not use `CanGarrison` as the player command validator for entering civilian buildings. Evidence: `CanDock` callers include `InfantryClass::What_Action_OnObject` and `PerCellProcess`; `CanGarrison` body is `Gate=`/mission-state only. Active in YR: Yes.
- Do not treat Rust owner strings `neutral`/`special` as the binary rule. Evidence: native reads `building.Owner->CountryType+0x1A6` (`MultiplayPassive`) at `0x00457D68..0x00457D73`. Active in YR: Yes.
- Do not add a chrono-warp rejection to `CanDock` based on this evidence. Evidence: scoped call is `TechnoClass::IsMindControlled @ 0x007105E0`, reading `+0x2C0/+0x2C4`. Active in YR: Yes for mind-control, not verified for warp.
- Do not merge bunker/unit-repair row-helper rules into infantry garrison entry. Evidence: `CanDock` reads `CanBeOccupied`/`Occupier`; `Can_Enter_Cell` gate path reads `+0x16B7`; bunker row-helper is absent from this branch per prior infantry report. Active in YR: No for scoped infantry entry gates.
- Do not assume `AddGarrisonOccupant` immediately transfers neutral building ownership. Evidence: decompile `0x00522910` has no `ChangeOwner` call; transfer timing belongs to the separate occupant reconciliation path. Active in YR: Yes.

## Remaining Uncertainty

- Exact mission `0x18` label and all writers/transitions for `Building+0x350` gate state were not traced; only the `CanGarrison` read-side contract is verified here.
- Full merged-stock audit of `Assaulter=yes` infantry was not performed; the binary path is verified and checked stock examples are `Assaulter=no`.
- Exact later ownership-transfer tick for neutral civilian garrisons remains sourced from prior docs, not re-proven in this slot.

## Sources

- Ghidra decompile/read-only: `0x00457CE0`, `0x004525F0`, `0x0051BF90`, `0x0051E3B0`, `0x00519630`, `0x00522910`, `0x005F5CD0`, `0x007105E0`, `0x004A51B0`.
- Ghidra assembly contexts/read-only: `0x00457CEB..0x00457DD5`, `0x004525F3..0x00452628`, `0x0051C4EB..0x0051C549`, `0x0051968E..0x0051972D`, `0x004A51B0..0x004A51C2`, `0x007105E0..0x007105FC`.
- Existing docs referenced: `GARRISON_SYSTEM_GHIDRA_REPORT.md`, `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`, `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md`, `BUNKER_SYSTEM_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned only: `src/rules/object_type.rs`, `src/sim/passenger.rs`, `src/sim/world/world_commands.rs`, `src/app_cursor.rs`, `src/app_context_order.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs`.
