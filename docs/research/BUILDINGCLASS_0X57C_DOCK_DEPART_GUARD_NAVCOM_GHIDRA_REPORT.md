# BuildingClass+0x57C Dock-Depart Guard NavCom - Ghidra Research Report

**Address(es):** `0x0073D630` primary; helpers `0x00451750`, `0x00451890`, `0x00451E40`, `0x004509D0`, `0x0043FB20`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `BuildingClass+0x57C` as the state-4 refinery unload depart guard, its slot-8 writer/clearer path, and stock GAREFN/NAREFN activity.
**Non-Scope:** full chrono miner exit track math, radio message inventory, all non-refinery production anim consumers, and exact AnimClass per-frame destruction timing.
**Confidence:** High for the guard and stock no-delay result; Medium for generic slot-8 lifecycle beyond the refinery path.
**Active in YR:** Conditional. The state-4 code path is active for standard YR harvesters and refineries; the actual wait branch requires a non-null slot-8 `ProductionAnim` pointer, which stock GAREFN/NAREFN do not create.

## 1. Overview

`BuildingClass+0x57C` is the building animation pointer array entry for slot 8, mapped to `ProductionAnim`. `UnitClass::Mission_Deploy_Building` state 4 checks this pointer before releasing the harvester from the unit-side unload mission handoff. In stock YR GAREFN/NAREFN, the code reaches the guard, but the pointer is normally zero because both stock refineries have no active `ProductionAnim` art key.

Player-visible consequence: stock chrono miners and war miners should not pause at refinery departure for a ProductionAnim. A modded refinery that defines `ProductionAnim` can create a real departure wait until the slot-8 anim pointer is cleared.

## 2. Key Offsets

| Field | Offset | Meaning | Active in YR |
|---|---:|---|---|
| `BuildingClass::Anims_0` | `+0x55C` | Base of 21 animation slot pointers | Yes; verified by helper iteration over `0x15` slots at `0x00451890`/`0x00451E40`. |
| `BuildingClass::Anims_0[8]` | `+0x57C` | Slot 8 live `AnimClass*`; `ProductionAnim` | Conditional; field is live, but stock refinery unload normally leaves it null. |
| `BuildingClass::Anims_0[10]` | `+0x584` | Slot 10 live `AnimClass*`; `SpecialAnim` | Yes for stock GAREFN/NAREFN per-bale ore anim. |
| `BuildingTypeClass + 0xF4C + slot*0x44` | slot table | Normal anim type name selected by `SetAnimSlotImage` | Conditional per key presence. |
| Slot-8 normal key | `Type+0x116C` | `ProductionAnim` name | No for stock GAREFN/NAREFN. |
| Slot-8 damaged key | `Type+0x117C` | `ProductionAnimDamaged` name | No for stock GAREFN/NAREFN. |
| Slot-8 alternate key | `Type+0x118C` | alternate selected when `param_4 != 0` | Not used by the verified refinery calls, which pass `0`. |

## 3. Core Logic

### Slot-8 creation before state 4

`UnitClass::Mission_Deploy_Building @ 0x0073D630` can request slot 8 in two refinery-unload state-3 exits:

| Call site | Trigger | Call | Active in YR |
|---|---|---|---|
| `0x0073E517` | `StorageClass::FindFirstNonEmptySlot` returns `-1`, and `building->Type+0x16BB` (`Refinery=yes`) is true | `BuildingClass::SetAnimSlotImage(slot=8, low_health, 0, 0)` | Yes for standard unload; effective only if art key exists. |
| `0x0073E58F` | queued mission override mid-unload, again gated by `Refinery=yes` | same slot-8 call | Conditional; active only if the override branch triggers. |

`BuildingClass::SetAnimSlotImage @ 0x00451750` computes the slot art pointer as `Type + slot*0x44 + 0xF4C` for normal health, `+0xF5C` for damaged, or `+0xF6C` for the alternate selector. It calls `CreateAnimForSlot` only when the chosen string is non-empty. Therefore the state-3 slot-8 request is a no-op for stock refineries whose `ProductionAnim` is absent/commented.

`BuildingClass::CreateAnimForSlot @ 0x00451890` is the writer. It destroys any old slot occupant, then writes the new `AnimClass*` to `(&this->Anims_0)[slot]`. For slot 8, that write is `building+0x57C`.

### State-4 departure guard

In `UnitClass::Mission_Deploy_Building` state 4, the non-Weeder path:

1. Recomputes the adjacent refinery cell from the harvester cell and the unload offset.
2. Looks up the building in that cell.
3. Requires `building != null`.
4. Requires `building->Type+0x16BB != 0` (`Refinery=yes`).
5. Reads `building+0x57C`.
6. If `building+0x57C != 0`, returns `1` immediately.

This return happens before `unit+0x6D1` is cleared and before the normal state-4 `SetMission(Harvest=10)` / queue handoff. It is a pure animation-slot occupancy guard. It does not read locomotor readiness, does not check a navcom status field, and does not send or wait for radio `0x07`, `0x10`, or `0x19`.

**Active in YR:** Yes for the state-4 path; Conditional for the wait branch. Standard YR GAREFN/NAREFN pass the `Refinery=yes` gate but normally fail the pointer-nonzero condition, so no ProductionAnim departure delay occurs.

### Slot clearing / lifetime

`BuildingClass::ClearAnimSlot @ 0x00451E40` is the generic clearer. For a specific slot it reads `(&this->Anims_0)[slot]`, writes zero to that entry, then calls the old anim's vtable `+0x20` destroy/release method. For slot 8, this clears `building+0x57C`.

`BuildingClass::UpdateAnimation @ 0x004509D0`, called from `BuildingClass::Update @ 0x0043FB20` at `0x0043FE22`, reads `building+0x57C` alongside a sibling slot. If the type/mission gates pass, it creates a later slot and clears the involved slots. This verifies `+0x57C` participates in the normal building animation lifecycle, not in movement/nav state.

**Active in YR:** The update function is live for buildings. For stock GAREFN/NAREFN unload, this slot-8 subpath is normally inactive because slot 8 was never created.

## 4. INI Keys

| File | Section | Key | Value | Effect |
|---|---|---|---|---|
| `ini/rulesmd.ini:11722` | `[GAREFN]` | `DockUnload` | `yes` | Standard Allied refinery accepts dock unload. Active in YR: Yes. |
| `ini/rulesmd.ini:11727` | `[GAREFN]` | `Refinery` | `yes` | Satisfies state-4 `Type+0x16BB` guard. Active in YR: Yes. |
| `ini/rulesmd.ini:11736` | `[GAREFN]` | `FreeUnit` | `CMIN` | Standard Allied refinery creates chrono miner. Active in YR: Yes. |
| `ini/artmd.ini:1763-1789` | `[GAREFN]` | `ProductionAnim` | absent | Slot-8 creation request is no-op. Active in YR: No stock delay. |
| `ini/rulesmd.ini:12515` | `[NAREFN]` | `DockUnload` | `yes` | Standard Soviet refinery accepts dock unload. Active in YR: Yes. |
| `ini/rulesmd.ini:12520` | `[NAREFN]` | `Refinery` | `yes` | Satisfies state-4 `Type+0x16BB` guard. Active in YR: Yes. |
| `ini/rulesmd.ini:12530` | `[NAREFN]` | `FreeUnit` | `HARV` | Standard Soviet refinery creates war miner. Active in YR: Yes. |
| `ini/artmd.ini:1749` | `[NAREFN]` | `;ProductionAnim=NAREFN_AR` | commented | Slot-8 creation request is no-op in stock. Active in YR: No stock delay. |

## 5. Integration Points

| Function | Verified role | Active in YR |
|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | Emits slot-8 request at unload completion/override and reads `building+0x57C` in state 4. | Yes for HARV/CMIN unload. |
| `BuildingClass::SetAnimSlotImage @ 0x00451750` | Selects slot art key and short-circuits on empty strings. | Yes; stock slot-8 refinery calls no-op. |
| `BuildingClass::CreateAnimForSlot @ 0x00451890` | Writes `AnimClass*` into `Anims_0[slot]`; slot 8 is `+0x57C`. | Conditional per non-empty art key. |
| `BuildingClass::ClearAnimSlot @ 0x00451E40` | Clears `Anims_0[slot]`; slot 8 clears `+0x57C`. | Yes as generic lifecycle. |
| `BuildingClass::UpdateAnimation @ 0x004509D0` | Reads slot-8 pointer and clears animation slots under type/mission gates. | Conditional for this slot; live function. |
| `BuildingClass::Update @ 0x0043FB20` | Calls `UpdateAnimation` at `0x0043FE22`. | Yes for buildings. |

## 6. Reconciliation With Prior Reports

`BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_GHIDRA_REPORT.md` is confirmed: `+0x57C` is `Anims_0[8]` / `ProductionAnim`, not locomotor readiness, not a building-ready flag, and not a dock latch.

`MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` is confirmed on the state-4 read: the guard checks `Refinery=yes` and `building+0x57C != 0`, then returns `1`. This follow-up resolves that report's older "likely anim-playing or loco-busy" wording to the narrower slot-8 `ProductionAnim` pointer.

The stock-YR activity statement is: standard refineries reach the guard but do not wait on it because slot 8 is empty. The behavior becomes active for mods that define `ProductionAnim`/damaged slot-8 art for a dock-unload refinery.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| State-4 guard read of `+0x57C` | verified | `decompile_function 0x0073D630`; prior disassembly cites `0x0073E1D5`, `0x0073E1DF`, `0x0073E1EA` | none for this slice |
| Slot-8 creation at unload complete | verified | `0x0073E517` xref to `0x00451750`; decompile shows `SetAnimSlotImage(8, ...)` | none |
| Slot-8 creation at mission override | verified | `0x0073E58F` xref to `0x00451750`; decompile shows second `SetAnimSlotImage(8, ...)` | none |
| `SetAnimSlotImage` empty-key no-op | verified | `0x00451750` checks chosen string before `CreateAnimForSlot` | none |
| `CreateAnimForSlot` slot pointer write | verified | `0x00451890` writes `(&this->Anims_0)[slot] = puVar4` | none |
| `ClearAnimSlot` slot pointer zeroing | verified | `0x00451E40` writes `(&this->Anims_0)[slot] = 0` before anim release | none |
| `UpdateAnimation` slot-8 lifecycle read | touched-not-exhausted | `0x004509D0`; prior report assembly for slot-8/slot-11 clears | exact non-refinery slot gates deferred |
| Stock GAREFN activity | verified | `rulesmd.ini:11726-11736`; `artmd.ini:1763-1789` | none |
| Stock NAREFN activity | verified | `rulesmd.ini:12519-12530`; `artmd.ini:1748-1749` | none |
| Exact duration of modded `ProductionAnim` wait | deferred | requires per-AnimType frame/lifetime trace | next slot should investigate AnimClass lifecycle if modded parity is needed |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What is `BuildingClass+0x57C`? It is `Anims_0[8]`, the live slot-8 `ProductionAnim` pointer. Evidence: `0x00451750`, `0x00451890`, `0x00451E40`, prior slot table in `REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`.

[RESOLVED] OQ-2 - Does state 4 wait on stock GAREFN/NAREFN? No. The guard is reached, but stock GAREFN has no `ProductionAnim`, and stock NAREFN's `ProductionAnim=NAREFN_AR` is commented. Evidence: `rulesmd.ini:11726-11736`, `rulesmd.ini:12519-12530`, `artmd.ini:1749`, `artmd.ini:1763-1789`.

[RESOLVED] OQ-3 - Is the guard a navcom/locomotor release field? No. It is a pointer read before the mission handoff. Evidence: `0x0073D630` state-4 branch reads `building+0x57C`; writer/clearer are animation slot helpers at `0x00451890`/`0x00451E40`.

[RESOLVED] OQ-4 - What writes `+0x57C` in this dock-depart slice? `CreateAnimForSlot` writes the new `AnimClass*` when called with slot 8, reached from `Mission_Deploy_Building` at `0x0073E517` and `0x0073E58F` only if the selected `ProductionAnim` key is non-empty.

[RESOLVED] OQ-5 - What clears `+0x57C`? `ClearAnimSlot` is the generic direct clearer; `UpdateAnimation` also participates in slot lifecycle. Evidence: `0x00451E40`, `0x004509D0`. Exact modded visible duration is deferred.

[DEFERRED] OQ-6 - Exact frame count for a modded refinery `ProductionAnim` wait. Reason: requires `AnimClass` tick/destruction and the chosen mod art's loop metadata, outside this slot. Category: out-of-scope.

## Sources

- Ghidra MCP: `decompile_function 0x0073D630`, `0x00451750`, `0x00451890`, `0x00451E40`, `0x004509D0`, `0x0043FB20`.
- Ghidra MCP xrefs: `get_function_xrefs 0x00451750`, `0x00451890`, `0x00451E40`, `0x004509D0`.
- Prior reports: `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_GHIDRA_REPORT.md`; `miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`; `miner/REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`; `miner/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini:11722-11736`, `ini/rulesmd.ini:12515-12530`, `ini/artmd.ini:1706-1749`, `ini/artmd.ini:1763-1789`.
