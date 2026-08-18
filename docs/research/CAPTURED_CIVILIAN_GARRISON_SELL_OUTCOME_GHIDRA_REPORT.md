# Captured Civilian Garrison Sell Outcome - Ghidra Research Report

**Address(es):** `0x004AC660`, `0x006A7780`, `0x00692610`, `0x004C6CB0`, `0x00449C30`, `0x00457DE0`, `0x00458200`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Player sell availability and sell outcome for a `CanBeOccupied=yes` civilian garrison after native reconciliation has made it owned by the human player.
**Non-Scope:** Owner-transfer timing details, garrison entry gates, exact occupant exit-cell order, repair-mode UI, rubble art, MCV/YAREFN undeploy conversion.
**Confidence:** High
**Active in YR:** Yes for standard YR sell-mode UI and `CanBeOccupied=yes` civilian garrisons after ownership reconciliation; conditional on the building being owned by the player and still alive.

## 0. Investigation Contract

**Target question:** Is the player sell command available for captured civilian `CanBeOccupied` garrisons, and if `BuildingClass::Sell` is reached, does native preserve/revert the building or destroy/sell it?

**Non-goals:** Do not re-study `AddGarrisonOccupant` ownership transfer; do not re-study exact edge-cell ejection order; do not implement Rust; do not mutate Ghidra; do not update any docs except this report and the shared claims file.

**Evidence needed to mark COMPLETE:**

1. Prove sell-mode activation reaches a cursor/action path for player-owned building targets.
2. Prove captured civilian garrisons become normal player-owned buildings for sell-mode purposes, without a captured-civilian exception in the sell UI path.
3. Prove the sell event reaches `BuildingClass::Sell` for live building objects.
4. Prove `BuildingClass::Sell` calls `SellBuilding` only as an occupant-ejection stage and then proceeds to final sell/removal logic.
5. Compare that verified behavior to current Rust `production_sell.rs::sell_building`.

**Stop conditions:** Stop after the sell availability/outcome is proven; defer only exact cursor frame names or exact occupant exit ordering if they are not needed to prove the outcome. If the sell event cannot be linked to `BuildingClass::Sell`, status must be PARTIAL.

## 1. Overview

Native does not have a captured-civilian preservation branch in player sell. After `CheckAutoSellOrCivilian` has transferred a civilian garrison to the player, sell mode treats it as a player-owned building target; the queued sell event calls the building sell mission, which ejects occupants if present and then continues into the normal sell destruction/refund path.

The current Rust branch keyed by `garrison_original_owner.is_some()` that ejects/reverts/keeps the building is therefore wrong for player sell. That branch resembles native red-HP/empty-garrison reconciliation, not native player sell.

## 2. Class Layout / Key Offsets

| Offset / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Display/Sidebar `+0x11B1` / `DAT_00880999` | sell-mode active flag | `FUN_004AC660`; `HouseClass__Update` assembly `0x004F8B1C` reads `[0x00880999]` and calls `0x004AC660(0)` if no owned buildings | Yes |
| `House+0x2F0` | owned building count gate for arming sell mode | `FUN_004AC660` checks `0 < *(int *)(g_PlayerPtr+0x2F0)` | Yes |
| `TechnoClass+0x90` | live/in-map object flag checked before sell event dispatch | `EventClass__Execute` case `0x15`, assembly `0x004C6EED..0x004C6EF5` | Yes |
| `Building+0x684/0x688/0x694` | garrison occupant vector/list and count | `SellBuilding @ 0x00457DE0`, prior reports; `CheckAutoSellOrCivilian @ 0x00458200` re-reads occupant count | Yes |
| Building vtable entry data `0x007E4104` | resolves sell mission to `BuildingClass__Sell @ 0x00449C30` | `get_function_xrefs BuildingClass__Sell` -> `From 007e4104 [DATA]`; Event case calls building vtable slot | Yes |

## 3. Core Logic

### 3.1 Sell mode is available while the player owns at least one building

`SidebarClass__Action @ 0x006A7780` handles event `0x8066` (Sell button) by playing the UI click sound and calling `FUN_004AC660(-1)`. Assembly context at `0x006A78C8` shows `PUSH -0x1`, `MOV ECX, ESI`, `CALL 0x004AC660`.

`FUN_004AC660` toggles the sell flag at `this+0x11B1`, clears repair/placement/superweapon modes, and only arms sell when `0 < *(int *)(g_PlayerPtr+0x2F0)`. Active in YR: Yes; this is the standard sidebar sell gadget path.

`HouseClass__Update` later reads the same sell-mode byte as `DAT_00880999`; if the current player has zero owned buildings, it calls `FUN_004AC660(0)` to cancel sell mode. Assembly at `0x004F8B1C..0x004F8B2B` reads `[0x00880999]`, loads `ECX=0x87f7e8`, pushes `0`, then calls `0x004AC660`. Active in YR: Yes.

### 3.2 Sell cursor/action eligibility has no captured-civilian exception

`DisplayClass__DetermineAction @ 0x00692610` has the sell-mode branch under `DAT_00880999`. For an object target, it requires an object, an owner pointer, and `HouseClass__IsHumanPlayer()` on that owner. If the target is a building (`WhatAmI()==6`), it returns sell/no-sell action codes from the generic building checks; there is no branch checking original owner, civilian origin, `CanBeOccupied`, or occupant ownership history. Active in YR: Yes; evidence is decompile of `0x00692610`, raw bytes/read-memory for `0x006929AF..0x00692B05`.

Prerequisite context from `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`: `CheckAutoSellOrCivilian @ 0x00458200`, called from `BuildingClass::Update`, changes occupied Civilian-owned `CanBeOccupied` buildings to the first occupant's owner. Once that has happened, the sell-mode owner check sees the building as human-owned. Active in YR: Yes for standard captured civilian garrisons.

### 3.3 The sell event reaches the building sell mission

`EventClass__Execute @ 0x004C6CB0` case `0x15` fetches an object (`FUN_006E6F20`), validates it is non-null and live (`+0x90 != 0`), then calls a virtual with `PUSH -1`. For a building, the vtable data xref resolves the relevant sell mission to `BuildingClass__Sell @ 0x00449C30`.

Assembly evidence:

- `0x004C6ED5`: `CALL 0x006E6F20`, then `TEST ESI,ESI`, `JZ`.
- `0x004C6EED..0x004C6EF5`: reads and tests `[ESI+0x90]`, aborts if not live.
- `0x004C6EFB..0x004C6F01`: `MOV EAX,[ESI]`, `PUSH -1`, `MOV ECX,ESI`, `CALL dword ptr [EAX+0x19C]`.
- `get_function_xrefs BuildingClass__Sell`: `From 007e4104 [DATA]`.

Active in YR: Yes; event processing is the live command queue consumer.

### 3.4 `BuildingClass::Sell` destroys/sells if reached; `SellBuilding` is only occupant ejection

`BuildingClass__Sell @ 0x00449C30` is a state machine. In state `1`, it calls the occupant-count virtual and, if count is positive, calls `BuildingClass__SellBuilding @ 0x00457DE0`. Assembly context at `0x0044A5BA..0x0044A5CA` shows a vtable occupant-count call, `TEST EAX,EAX`, and `CALL 0x00457DE0`.

After this ejection stage, the function continues into the normal sell path. State `2` handles the final transaction: if no `UndeploysInto` branch applies, it detaches/unlimbos the source building, adds sell credits, processes stored ore, calls final cleanup/uninit (`vtable+0xF8` in decompile), and optionally enters rubble handling. The decompile contains no check for "was originally Civilian" and no `ChangeOwner(Civilian)` preservation branch in player sell.

`BuildingClass__SellBuilding @ 0x00457DE0` clears/ejects occupants and recalculates related state; it does not call `ChangeOwner` and does not destroy the building by itself. Active in YR: Yes, but only as a helper stage in player sell, red-HP ejection, and destruction ejection.

## 4. INI Keys

| Key | Defaults / examples | Effect in this slice | Binary reader / use | Active in YR |
|---|---|---|---|---|
| `CanBeOccupied` | many civilian buildings in `rulesmd.ini` set `yes`; base comments define UC buildings | Enables garrison/civilian reconciliation path; not a sell preservation flag | `BuildingClass::Update` prior report: `Type+0x157B` guard before `CheckAutoSellOrCivilian`; sell UI does not read it | Yes |
| `MaxNumberOccupants` | usually `5`, `6`, `8`, or `10` on UC buildings | Gives garrison capacity; not a sell preservation flag | garrison reports; occupant count is runtime vector count at sell | Yes |
| `CanOccupyFire` | usually `yes` with UC buildings | Garrison firing ability; irrelevant to sell outcome | Not read by sell path in this slice | Conditional; not material here |
| `Unsellable` | tech buildings often set `yes`; ordinary civilian UC buildings searched here do not rely on this key | If set and represented in type data, can produce a no-sell outcome; not tied to captured civilian state | Cursor/sell action uses generic target checks, not captured-origin checks | Conditional |
| `[Selling]` `Recruitable=no`, `NoThreat=yes` | `rulesmd.ini:30571..30573` | Mission/control metadata for selling objects, not a preservation rule | State machine mission path | Yes |

## 5. Integration Points

| Point | Evidence | Active in YR |
|---|---|---|
| Sell gadget click -> sell mode toggle | `SidebarClass__Action @ 0x006A7780`; assembly `0x006A78C8` calls `0x004AC660(-1)` | Yes |
| Sell mode target action | `DisplayClass__DetermineAction @ 0x00692610`; `DAT_00880999` branch | Yes |
| Sell command execution | `EventClass__Execute @ 0x004C6CB0` case `0x15`; assembly `0x004C6ED5..0x004C6F11` | Yes |
| Sell mission | `BuildingClass__Sell @ 0x00449C30`; data xref `0x007E4104` | Yes |
| Garrison ejection helper | `BuildingClass__SellBuilding @ 0x00457DE0`; call from `0x0044A5CA` | Yes |
| Civilian ownership prerequisite | `CheckAutoSellOrCivilian @ 0x00458200`; prior report and fresh decompile | Yes |

## 6. Current Rust Implementation Status

Rust currently diverges from native for this target:

- `src/sim/production/production_sell.rs::sell_building` treats `garrison_original_owner.is_some()` as "captured civilian" and returns early after ejection/revert/no refund, keeping the building.
- `src/app_commands.rs::sell_selected_buildings` queues `Command::SellBuilding` for selected structures owned by the local player, with no captured-civilian exclusion.
- `src/sim/world/world_commands.rs` checks command-owner ownership, then calls `production::sell_building`.

Because native sell-mode eligibility sees the post-reconciliation owner and native `BuildingClass::Sell` has no captured-civilian preservation branch, the Rust early-return preserve branch is a mismatch when the player sell command is issued.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / complete evidence / stop conditions | verified | Section 0 | none |
| Sell gadget event `0x8066` | verified | `0x006A7780`, assembly `0x006A78C8` | none |
| Sell-mode active flag `DAT_00880999` / `+0x11B1` | verified | `0x004AC660`, `0x004F8B1C..0x004F8B2B` | none |
| Sell-mode cursor/action branch | verified for outcome | `0x00692610`, bytes `0x006929AF..0x00692B05` | exact cursor frame naming out of scope |
| Captured civilian owner prerequisite | verified-from-prior, spot-checked | `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `0x00458200` fresh decompile | none for this target |
| Event sell execution | verified | `0x004C6CB0`, assembly `0x004C6ED5..0x004C6F11` | none |
| `BuildingClass::Sell` occupant stage | verified | `0x00449C30`, assembly `0x0044A5BA..0x0044A5CA` | exact ejection placement out of scope |
| `BuildingClass::Sell` final outcome | verified | `0x00449C30` decompile, disassembly/read range `0x00449C30..0x0044A9FF` from prior report plus fresh decompile | rubble art and storage credit details out of scope |
| `SellBuilding` owner/destruction effect | verified | `0x00457DE0` decompile | exact exit-cell order covered by other reports |
| Rust `production_sell.rs` comparison | verified | local source scan lines around `sell_building` | implementation not changed in this report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this an exhaustive-slice or coverage-map? -> exhaustive-slice for captured civilian garrison player-sell availability/outcome only.` (evidence: user scope; Section 0)
- `[RESOLVED] OQ-02 - What arms sell mode? -> Sidebar event 0x8066 calls FUN_004AC660(-1), which toggles +0x11B1/DAT_00880999 if the player owns at least one building.` (evidence: `0x006A7780`, `0x004AC660`, `0x006A78C8`)
- `[RESOLVED] OQ-03 - Does sell mode remain active with no buildings? -> No; HouseClass__Update cancels it when owned building count is zero and DAT_00880999 is set.` (evidence: `0x004F8B1C..0x004F8B2B`)
- `[RESOLVED] OQ-04 - Does sell cursor eligibility inspect captured-civilian origin? -> No branch was found in DisplayClass__DetermineAction sell-mode path; it uses current owner/human ownership and generic building action checks.` (evidence: `0x00692610`, bytes `0x006929AF..0x00692B05`)
- `[RESOLVED] OQ-05 - Does CanBeOccupied itself block sell? -> No sell-specific `CanBeOccupied` preservation/block branch exists in the sell UI or sell mission; it matters only because the prior reconciliation can change the current owner.` (evidence: `0x00692610`, `0x00449C30`, `0x00458200`)
- `[RESOLVED] OQ-06 - Does the sell event call the building sell mission? -> Yes; EventClass case 0x15 validates a live object and calls the virtual sell mission; building data xref resolves to BuildingClass__Sell.` (evidence: `0x004C6ED5..0x004C6F11`, data xref `0x007E4104`)
- `[RESOLVED] OQ-07 - What does SellBuilding do if reached from sell? -> It ejects/clears occupants; it is not final sell/destruction and does not ChangeOwner.` (evidence: `0x00457DE0`)
- `[RESOLVED] OQ-08 - Does BuildingClass::Sell stop after SellBuilding for occupied garrisons? -> No; it calls SellBuilding from state 1 and continues to state 2 final sell logic.` (evidence: `0x00449C30`, `0x0044A5BA..0x0044A5CA`)
- `[RESOLVED] OQ-09 - Does BuildingClass::Sell preserve/revert captured civilian garrisons? -> No captured-civilian or original-owner branch was found; final sell path removes/sells the building if reached.` (evidence: `0x00449C30` decompile)
- `[RESOLVED] OQ-10 - Does current Rust match? -> No; Rust keeps/reverts captured civilian garrisons on sell via `garrison_original_owner.is_some()` early return.` (evidence: `src/sim/production/production_sell.rs::sell_building`)
- `[DEFERRED] OQ-11 - Exact sell cursor frame ID and mouse.shp frame sequence?` (category: out-of-scope; reason: not needed to prove command availability/outcome; next-step-if-pursued: UI cursor parity report)
- `[DEFERRED] OQ-12 - Exact ejection cell order and scatter RNG?` (category: out-of-scope; reason: already covered by adjacent garrison ejection reports and not needed for sell preservation question; next-step-if-pursued: use `GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-13 - Null/dead target edge case? -> Event case 0x15 aborts if object lookup returns null or live byte +0x90 is zero.` (evidence: `0x004C6ED5..0x004C6EF5`)
- `[RESOLVED] OQ-14 - Empty garrison edge case? -> Sell skips SellBuilding if occupant count is zero, then still proceeds to final sell logic.` (evidence: `0x00449C30`, `0x0044A5BA..0x0044A5CA`)
- `[RESOLVED] OQ-15 - TS legacy filter? -> The sell UI/event/sell mission path is standard YR; no TS-only gate controls the target outcome.` (evidence: live sidebar/event/building mission paths above)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Player-owned captured civilian `CanBeOccupied` garrisons are sell-command eligible through normal sell-mode ownership checks; no captured-origin exception exists. | `0x004AC660`; `0x00692610`; prior `0x00458200` ownership transfer | mismatch risk: Rust command can reach sell, but implementation preserves building | `src/app_commands.rs::sell_selected_buildings`; `src/sim/world/world_commands.rs`; `src/sim/production/production_sell.rs::sell_building` | Let sell command for post-reconciliation owned garrison enter normal building sell behavior. | Player captures civilian UC building, waits one update for ownership, arms sell, clicks building; command is accepted. Proposed test: `captured_civilian_garrison_sell_command_is_available_after_reconciliation` | Do not block sell just because the building was originally civilian. |
| If `BuildingClass::Sell` is reached, occupied garrisons call `SellBuilding` for occupant ejection and then proceed to normal sell/removal/refund logic. | `0x00449C30`; assembly `0x0044A5BA..0x0044A5CA`; `0x00457DE0` | mismatch: Rust early-returns for `garrison_original_owner.is_some()` and keeps/reverts building | `src/sim/production/production_sell.rs::sell_building` | Remove/disable the captured-civilian preserve branch for player sell; use normal sell path after ejection. | Captured civilian garrison with occupants is sold; occupants eject and building is removed. Proposed test: `captured_civilian_garrison_sell_removes_building_after_ejecting_occupants` | Do not cite `SellBuilding` helper alone as proof of preservation. |
| `SellBuilding` does not ChangeOwner; ownership revert is a separate `CheckAutoSellOrCivilian` reconciliation behavior for empty/red-HP garrisons, not the player sell transaction. | `0x00457DE0`; `0x00458200`; `0x00449C30` | mismatch: Rust uses sell ejection helper to revert owner and keep building | `production_sell.rs::eject_garrison_occupants`; `passenger.rs` reconciliation surfaces | Keep revert behavior in abandon/red-HP reconciliation, but not in player sell final outcome. | Empty-abandon still reverts, while player sell removes; both scenarios remain distinct. Proposed test: `abandon_reverts_but_player_sell_removes_captured_civilian_garrison` | Do not merge abandon/revert semantics into sell semantics. |

## 10. Negative Facts / Do Not Do

- Do not preserve a captured civilian garrison on player sell. Active in YR: No; evidence `BuildingClass__Sell @ 0x00449C30` proceeds to final sell/removal once reached.
- Do not use `garrison_original_owner.is_some()` as a native sell-outcome discriminator. Active in YR: No; evidence native sell UI/mission has no original-owner branch.
- Do not treat `BuildingClass__SellBuilding @ 0x00457DE0` as the complete sell transaction. Active in YR: No; evidence it is called from state 1 at `0x0044A5CA`, before final sell state logic.
- Do not block sell solely because the building is `CanBeOccupied=yes`. Active in YR: No; evidence sell-mode path uses current ownership/generic building checks, not `CanBeOccupied` as a no-sell gate.
- Do not move the player-sell preservation behavior into `CheckAutoSellOrCivilian`; that helper is the update/reconciliation path, not command execution. Active in YR: No for player sell; evidence `EventClass__Execute` case `0x15` calls sell mission.

## 11. Remaining Uncertainty

None for the target outcome. Exact cursor artwork/frame and exact occupant exit ordering were intentionally out of scope and do not affect the sell-preserve-vs-remove conclusion.

## 12. Stale Docs / Follow-up Docs

- `src/sim/production/production_sell.rs` lines documenting the captured-civilian sell branch should be replaced with: "Native player sell does not preserve/revert captured civilian `CanBeOccupied` garrisons. Once the building is owned by the player, sell uses the normal `BuildingClass::Sell` state machine: eject occupants via `SellBuilding`, then remove/sell the building. Revert-to-civilian belongs to `CheckAutoSellOrCivilian` abandon/red-HP reconciliation, not player sell."
- `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md` Section 10 handoff row should be updated from "Re-check captured-civilian sell UX..." to: "Captured-civilian sell UX is now verified by `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md`: player sell is available after ownership reconciliation and `BuildingClass::Sell` removes/sells the building if reached; Rust's captured-civilian preserve branch is stale."
- `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md` should append: "Follow-up verified: captured civilian `CanBeOccupied` garrisons do not have a player-sell preservation exception; `SellBuilding` remains eject-only, while `BuildingClass::Sell` supplies the final removal."

## Sources

- Ghidra read-only decompile: `FUN_004AC660`, `SidebarClass__Action @ 0x006A7780`, `DisplayClass__DetermineAction @ 0x00692610`, `EventClass__Execute @ 0x004C6CB0`, `BuildingClass__Sell @ 0x00449C30`, `BuildingClass__SellBuilding @ 0x00457DE0`, `BuildingClass__CheckAutoSellOrCivilian @ 0x00458200`.
- Ghidra read-only assembly/context: `0x006A78C8`, `0x004F8B1C..0x004F8B2B`, `0x004C6ED5..0x004C6F11`, `0x0044A5BA..0x0044A5CA`.
- Ghidra read-only bytes/ranges: `0x006929AF..0x00692B05`, `0x004C6E80..0x004C6F1F`, prior/fresh `0x00449C30..0x0044A9FF`.
- Docs referenced: `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`, `SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md`, `MouseClass_research.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, especially `CanBeOccupied`, `MaxNumberOccupants`, `CanOccupyFire`, `Unsellable`, `[Selling]`.
