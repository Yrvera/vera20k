# UnitClass PerCellProcess Contact-Flag Adjacent-Building 0x15 Branch - Ghidra Report

**Address(es):** `0x00739EC0` primary, with support from `0x006F4AB0`, `0x0047C520`, `0x005657A0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** The later non-GetDockCoord `0x15` branch in `UnitClass::PerCellProcess`, specifically the branch gated by `Techno+0x418`, a building destination, mission `7`, and a building lookup in the cell one row north of the unit's current cell.
**Non-Scope:** Accepted refinery target `NW+(3,1)`, stock `GetDockCoord` `NW+(2,1)`, `QueueingCell` `NW+(4,1)`, full `0x16` scheduling, and radio `0x15` receiver side effects beyond the branch's send/return behavior.
**Confidence:** High for predicates, cell math, branch outcome, and Rust-facing delta; Medium for exact live-frame trigger frequency because static evidence proves the active branch and gates but not which `0x15` source wins in every runtime frame.
**Active in YR:** Conditional yes. The code is live in standard YR UnitClass per-cell processing and the `+0x418` flag is set by the standard refinery `0x18` handshake; this exact branch fires only when its destination/building/current-cell predicates line up.

## 0. Working Notes

Target question: What exact predicates and effects does the later non-GetDockCoord `0x15` branch in `UnitClass::PerCellProcess @ 0x00739EC0` use, and is it active for stock YR refinery docking?

Non-goals: Do not re-investigate the settled accepted `NW+(3,1)`, stock `GetDockCoord` `NW+(2,1)`, or `QueueingCell` `NW+(4,1)` split; do not decode all of `UnitClass::PerCellProcess`; do not write Rust.

Evidence needed to mark COMPLETE: Decompile plus assembly context for the `+0x418` branch, decompile for the building lookup helper, evidence for the `+0x418` writer, stock INI gates for stock refinery docking, current Rust surfaces that would need the handoff, and stale-doc replacement wording.

Stop conditions: Stop once the branch predicates, one-row-north cell math, `0x15` return handling, active-YR gate, Rust delta, negative facts, and remaining uncertainty are all documented without expanding into other radio/facing/unload slots.

## 1. Overview

`UnitClass::PerCellProcess` has a later `0x15` sender that is separate from the earlier `GetDockCoord` equality branch. This later branch does not call `GetDockCoord` and does not compare against the dock coord. It only runs when the unit's contact flag `+0x418` is set, the current destination is a building, the unit's current mission is `7`, and the cell one row north of the unit's current occupied cell contains that same destination building.

If those gates pass, the unit transmits directed radio `0x15` to the destination building. If the receiver returns neither `1` nor `5`, the unit queues/sets a fallback mission through vtable `+0x174` with `DAT_00B1CFE8, 1, 0`. If the receiver returns `1` or `5`, this branch has no fallback write and falls through to later per-cell logic.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Verified meaning in this branch | Evidence | Active in YR |
|---|---:|---|---|---|
| `+0x418` | Techno/Unit instance | Contact-entered radio flag. Must be nonzero before this branch continues. | `0x0073A558`; writer `0x006F4AB0` case `0x18` | Conditional yes |
| `+0x5A4` / decompiler `param_1[1].field_0x84` | Foot/Unit | Current destination/NavCom pointer; this branch requires it to be non-null and a building. | Decompile `0x00739EC0`; assembly loads destination from stack after `FootClass__GetDestination` | Yes |
| vtable `+0x2C` | Abstract object | `WhatAmI()` / abstract type check; branch requires destination type `6` (building). | `0x0073A572..0x0073A578` | Yes |
| vtable `+0x184` | Unit/Foo mission getter | Current mission check; branch requires mission `7`. | `0x0073A57D..0x0073A588` | Yes |
| vtable `+0x1B8` | Object occupied-cell getter | Called at function entry to get current cell; saved as local low word X / high word Y. | `0x00739ECD..0x00739EE2` | Yes |
| `Look_up_building_in_cell @ 0x0047C520` | Cell helper | Scans `CellClass+0xE4` and returns first object whose `WhatAmI()==6`. | Decompile `0x0047C520` | Yes |
| vtable `+0x278` | Radio transmit/receive routing | Sends directed radio message `0x15` with destination building argument. | `0x0073A5C0..0x0073A5C8` | Yes |
| vtable `+0x174` | Mission/action fallback | Called only if `0x15` return is neither `1` nor `5`. | `0x0073A5D6..0x0073A5E4` | Conditional |

## 3. Core Logic

### 3.1 Branch predicates

The relevant decompile block is:

```text
if (unit.+0x418 != 0 &&
    destination != NULL &&
    destination.WhatAmI() == 6 &&
    unit.CurrentMission() == 7) {
    check_cell = (current_cell.x, current_cell.y - 1);
    cell = MapClass::Get_CellClass(check_cell);
    building = Look_up_building_in_cell(cell);
    if (destination == building) {
        result = unit.Radio(0x15, destination);
        if (result != 1 && result != 5) {
            unit.Set_Mission_Or_Action(DAT_00B1CFE8, 1, 0);
        }
    }
}
```

Assembly confirms the exact early gates:

- `0x0073A558`: `MOV AL, byte ptr [EBP + 0x418]`
- `0x0073A55E..0x0073A560`: zero `+0x418` jumps out to `0x0073A5EA`
- `0x0073A566..0x0073A56C`: null destination jumps out
- `0x0073A572..0x0073A578`: destination vtable `+0x2C`, require return `0x6`
- `0x0073A57D..0x0073A588`: unit vtable `+0x184`, require return `0x7`

This branch is therefore not a generic "any adjacent building" trigger. It is a contact-flagged, mission-7, destination-building trigger.

### 3.2 Adjacent-building lookup cell math

At function entry, `UnitClass::PerCellProcess` obtains the unit's current occupied cell:

- `0x00739ECD..0x00739ED3`: load vtable and call vtable `+0x1B8`
- `0x00739EDD..0x00739EE2`: copy the returned packed cell into locals

In the branch, the decompiler reconstructs:

```text
check_cell = CONCAT22(current_y - 1, current_x)
```

Assembly confirms the `Y-1` and same-X shape:

- `0x0073A58A`: move current high word / Y source into `ECX`
- `0x0073A58E`: move current low word / X source into `AX`
- `0x0073A593`: `DEC ECX`
- `0x0073A594`: store low word X into stack cell
- `0x0073A599`: store decremented high word Y into stack cell
- `0x0073A5A2..0x0073A5B0`: call `MapClass__Get_CellClass @ 0x005657A0`
- `0x0073A5B7`: call `Look_up_building_in_cell @ 0x0047C520`

`MapClass__Get_CellClass @ 0x005657A0` indexes `y * 0x200 + x`, with a fallback dummy cell for out-of-range or null backing entries. It does not clamp the supplied `Y-1`; it returns the dummy cell if the linear index is invalid.

`Look_up_building_in_cell @ 0x0047C520` is a pure building lookup over the cell object list while `g_GameActive != 0`: it walks `CellClass+0xE4`, follows next pointer `+0x30`, calls each object's vtable `+0x2C`, and returns the first object whose type is `6`. It does not verify refinery flags, health, DockUnload, or ownership in this helper.

### 3.3 `0x15` send and return handling

If the destination pointer equals the building found in `(current_x, current_y - 1)`, the branch sends directed radio `0x15`:

- `0x0073A5BC`: compare destination pointer in `ESI` with lookup result in `EAX`
- `0x0073A5BE`: mismatch jumps out
- `0x0073A5C3`: push destination building
- `0x0073A5C4`: push `0x15`
- `0x0073A5C8`: call unit vtable `+0x278`

The return is treated as follows:

- `0x0073A5CE`: `DEC EAX`; original return `1` jumps out at `0x0073A5CF`
- `0x0073A5D1`: `SUB EAX, 0x4`; original return `5` jumps out at `0x0073A5D4`
- otherwise `0x0073A5D9..0x0073A5E4` calls vtable `+0x174` with `DAT_00B1CFE8`, `1`, `0`

So the branch tolerates exactly receiver return `1` and `5` as non-fallback results. Any other return queues/sets the fallback action.

### 3.4 Activity for stock YR refinery docking

The branch is active code for standard YR units. The stock refinery path can set the branch's radio/contact flag:

- `TechnoClass::Receive_Radio @ 0x006F4AB0`, case `0x18`, writes `+0x418 = 1` when the byte was previously zero, then propagates `0x18`.
- Existing verified refinery reports show `BuildingClass::Receive_Radio(0x0E)` can send `0x18`/`0x16` after an already-there `0x12` result in the stock DockUnload path.
- `rulesmd.ini:[GAREFN] DockUnload=yes` at line `11726`; `rulesmd.ini:[GAREFN] Refinery=yes` at line `11727`; `rulesmd.ini:[NAREFN] DockUnload=yes` at line `12519`; `rulesmd.ini:[NAREFN] Refinery=yes` at line `12520`.
- `rulesmd.ini:[CMIN] Dock=NAREFN,GAREFN` at line `7361`; `rulesmd.ini:[HARV] Dock=NAREFN,GAREFN` at line `8225`.

The branch is conditional, not guaranteed on every miner arrival frame. It requires the unit to already have `+0x418`, mission `7`, a building destination, and the destination building present in the cell one row north of the current occupied cell. Static evidence proves the active path and all gates; exact first-winner timing versus `UnitClass::Receive_Radio(0x16)` and the earlier `GetDockCoord` equality branch remains runtime-order sensitive.

## 4. INI Keys

| Key | Stock YR value | Role in this slice | Evidence |
|---|---|---|---|
| `[GAREFN] DockUnload` | `yes` | Enables stock refinery dock-unload radio path that can establish contact state. | `rulesmd.ini:11726` |
| `[GAREFN] Refinery` | `yes` | Makes the building a stock refinery/dock destination. | `rulesmd.ini:11727` |
| `[NAREFN] DockUnload` | `yes` | Same for Soviet refinery. | `rulesmd.ini:12519` |
| `[NAREFN] Refinery` | `yes` | Same for Soviet refinery. | `rulesmd.ini:12520` |
| `[CMIN] Dock` | `NAREFN,GAREFN` | Lets Chrono Miner target stock refineries. | `rulesmd.ini:7361` |
| `[HARV] Dock` | `NAREFN,GAREFN` | Lets War Miner target stock refineries. | `rulesmd.ini:8225` |

There is no INI key for `+0x418` itself; it is runtime radio/contact state.

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::PerCellProcess @ 0x00739EC0` | Owns this branch and sends `0x15` on the adjacent-building condition. | decompile and assembly `0x0073A558..0x0073A5E4` | Yes / conditional |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | `0x18` writer for `+0x418`; `0x19` clearer. | decompile case `0x18`/`0x19` | Yes / conditional |
| `MapClass::Get_CellClass @ 0x005657A0` | Converts `(x,y-1)` into a `CellClass*`; invalid index returns dummy cell. | decompile | Yes |
| `Look_up_building_in_cell @ 0x0047C520` | Returns the first building object in a cell object list. | decompile | Yes |
| `BuildingClass::Receive_Radio(0x0E)` | Stock path that can cause `0x18`/`0x16` after already-there admission. | prior reports and stock INI gates | Yes |

## 6. Current Rust Implementation Status

Rust currently has a contact model but still compresses this branch into the broader dock FSM:

| Surface | Current behavior | Delta / risk |
|---|---|---|
| `src/sim/miner/miner_dock.rs:31` `RefineryDockContacts` | Models `contact_entered` as a `+0x418`-like state, separate from `on_pad`. | Good surface for the flag, but branch-specific adjacent-building lookup is not modeled as its own `0x15` source. |
| `src/sim/miner/miner_dock_sequence.rs:613` `phase_mission_enter` | Marks contact entered and moves to `Linked` when idle at accepted cell. | Compresses `0x12 already-there`, `0x18`, `0x16`, and `0x15` source handling into one phase transition. |
| `src/sim/miner/miner_dock_sequence.rs:700` `phase_linked` | Mutates snapshot position to `pad` and calls `link_on_pad`, then starts pivot/display state. | High risk if `pad` means stock `GetDockCoord` `NW+(2,1)`; this branch proves another source can attempt `0x15` from an adjacent-building/contact state without proving physical `GetDockCoord` arrival. |
| `src/sim/miner/miner_dock_sequence.rs:719..722` | Marks contact entered and on-pad together. | Binary separates `+0x418` contact state from any physical/pad occupancy model. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `+0x418` branch entry predicates | verified | `0x0073A558..0x0073A588` | none |
| Adjacent cell math `(x, y-1)` | verified | `0x0073A58A..0x0073A599` | none |
| `MapClass::Get_CellClass` invalid-cell behavior | verified | `0x005657A0` | none for this branch |
| `Look_up_building_in_cell` semantics | verified | `0x0047C520` | helper does not prove building is a refinery; caller destination does |
| `0x15` send and return handling | verified | `0x0073A5BC..0x0073A5E4` | receiver-side effects belong to slot 5 |
| `+0x418` writer | verified | `0x006F4AB0` case `0x18`; prior `+0x418` report | exact second-call scheduling belongs to slot 4 |
| Stock YR activity gates | verified | `rulesmd.ini` stock Dock/DockUnload/Refinery lines plus prior Building `0x0E` reports | exact first-winner runtime source remains unresolved |
| Current Rust surfaces | verified | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs` scans | implementation patch not part of this report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-CF-001 - What mode is this investigation? -> exhaustive-slice for the later `+0x418` adjacent-building `0x15` branch only.` (evidence: user scope)
- `[RESOLVED] OQ-CF-002 - What are the exact branch entry gates? -> `+0x418 != 0`, non-null destination, destination `WhatAmI()==6`, and unit mission `7`.` (evidence: `0x0073A558..0x0073A588`)
- `[RESOLVED] OQ-CF-003 - Does this branch call `GetDockCoord`? -> No; no vtable `+0xA8` call occurs in this branch, unlike the earlier equality branch.` (evidence: `0x0073A558..0x0073A5E4`)
- `[RESOLVED] OQ-CF-004 - What cell does it inspect? -> Same current X, current Y minus one.` (evidence: `0x0073A58A..0x0073A599`)
- `[RESOLVED] OQ-CF-005 - What does the cell lookup return? -> First building object in the cell object list, type `6`, while `g_GameActive != 0`.` (evidence: `0x0047C520`)
- `[RESOLVED] OQ-CF-006 - What if the inspected cell is out of range? -> `MapClass::Get_CellClass` returns the dummy cell, and building lookup returns null unless that dummy has an object list.` (evidence: `0x005657A0`, `0x0047C520`)
- `[RESOLVED] OQ-CF-007 - Does the branch require the looked-up building to equal the current destination? -> Yes, pointer equality between destination and lookup result.` (evidence: `0x0073A5BC..0x0073A5BE`)
- `[RESOLVED] OQ-CF-008 - What radio message is sent? -> Directed message `0x15` to the destination building.` (evidence: `0x0073A5C3..0x0073A5C8`)
- `[RESOLVED] OQ-CF-009 - Which return values avoid fallback? -> Receiver returns `1` or `5`.` (evidence: `0x0073A5CE..0x0073A5D4`)
- `[RESOLVED] OQ-CF-010 - What happens on other return values? -> The unit calls vtable `+0x174` with `DAT_00B1CFE8`, `1`, `0`.` (evidence: `0x0073A5D6..0x0073A5E4`)
- `[RESOLVED] OQ-CF-011 - Is `+0x418` active for stock YR refinery docking? -> Yes conditionally; standard `0x18` writes it and stock refinery DockUnload paths can send `0x18`.` (evidence: `0x006F4AB0`; prior Building `0x0E` reports; `rulesmd.ini:11726,12519`)
- `[RESOLVED] OQ-CF-012 - Is this branch the same as PerCellProcess GetDockCoord equality? -> No; it is a separate later source with different gates and no `GetDockCoord` compare.` (evidence: `0x0073A558..0x0073A5E4` versus prior `0x0073A391..0x0073A52B`)
- `[RESOLVED] OQ-CF-013 - Which Rust surfaces are affected? -> `RefineryDockContacts`, `phase_mission_enter`, and `phase_linked` in miner dock code.` (evidence: source scan)
- `[DEFERRED] OQ-CF-014 - Does this adjacent-building branch win before later/aligned `0x16` in a concrete stock replay?` (category: needs-runtime-debugger; reason: static code proves both active sources and gates, not the exact first source in every live frame; next-step-if-pursued: runtime trace a miner at accepted cell through `0x18`, `0x16`, and next `PerCellProcess`)
- `[DEFERRED] OQ-CF-015 - What exact receiver-side writes does BuildingClass `0x15` perform?` (category: out-of-scope; reason: assigned to slot 5; next-step-if-pursued: read `Radio_0x15_start_unload_side_effects` report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `+0x418` contact flag gates a separate PerCellProcess `0x15` source; it is not pad occupancy. | `0x0073A558`; writer `0x006F4AB0` case `0x18` | Partially modeled as `contact_entered`, but `phase_linked` currently couples contact with `on_pad`. | `src/sim/miner/miner_dock.rs`; `src/sim/miner/miner_dock_sequence.rs` | Keep contact-entered state distinct from physical/on-pad or snapshot-position state. | Miner has contact flag after `0x18` while still physically at accepted cell; this alone must not imply `on_pad`. | Do not mark pad occupancy just because `+0x418`/contact-entered is set. |
| Branch checks destination building in cell `(current_x, current_y-1)` and sends `0x15` only if pointer-equal to destination. | `0x0073A58A..0x0073A5C8`; `0x0047C520` | Missing as a source-aware handoff. | Future miner per-cell/contact handling; currently around `phase_mission_enter` and `phase_linked`. | Add/model a branch-specific trigger that looks one row north from current cell and compares to the active refinery destination before starting unload. | Miner at a cell whose north neighbor contains the destination refinery can attempt `0x15` without current cell equaling `GetDockCoord`. | Do not substitute `refinery_pad_cell` or `GetDockCoord` equality for this branch. |
| Receiver return `1` or `5` are accepted non-fallback results; other returns call vtable `+0x174` with `DAT_00B1CFE8,1,0`. | `0x0073A5CE..0x0073A5E4` | Unchecked; current Rust likely treats dock handoff as a phase transition without return-code distinction. | Miner dock FSM / radio-result modeling. | Preserve result-sensitive handling if Rust models this source; only successful/accepted return codes should continue normal handoff. | Stub building receiver returning a refusal does not start unload and sends miner into the fallback state. | Do not treat every attempted `0x15` as unload start. |

Concrete Rust test names:

- `miner_dock_contact_flag_does_not_imply_pad_occupancy`
- `miner_dock_percell_contact_flag_north_building_sends_0x15`
- `miner_dock_percell_contact_flag_requires_destination_pointer_match`
- `miner_dock_percell_contact_flag_0x15_refusal_falls_back`
- `miner_dock_getdockcoord_and_contact_flag_sources_are_distinct`

## 10. Negative Facts / Do Not Do

- Do not say PerCellProcess has only the `GetDockCoord` equality `0x15` source. Evidence: separate branch `0x0073A558..0x0073A5E4`.
- Do not make this branch call or depend on `GetDockCoord`; this branch contains no vtable `+0xA8` call. Evidence: decompile and assembly for `0x0073A558..0x0073A5E4`.
- Do not make `+0x418` equal pad occupancy or reciprocal dock link; it is a byte contact flag set by radio `0x18`. Evidence: `0x006F4AB0`, prior `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.
- Do not send `0x15` merely because any building is north of the unit; the looked-up building must pointer-equal the current destination. Evidence: `0x0073A5BC..0x0073A5BE`.
- Do not treat every `0x15` return as success; only returns `1` and `5` avoid the fallback call. Evidence: `0x0073A5CE..0x0073A5E4`.

## 11. Stale Docs / Follow-up Docs

Known stale or too-narrow wording:

- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_GETDOCKCOORD_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/_system.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/_parity.md`

Replacement wording:

> `UnitClass::PerCellProcess @ 0x00739EC0` has at least two refinery-relevant `0x15` sources. The earlier `GetDockCoord` equality branch sends `0x15` only when current cell equals the destination building's `GetDockCoord`. A later separate branch requires `Techno+0x418`, a non-null building destination, mission `7`, and the destination building found in the cell one row north of the unit's current occupied cell; that branch sends `0x15` without calling `GetDockCoord`.

## 12. Remaining Uncertainty

- Exact live-frame source winner among later/aligned `0x16`, PerCellProcess `GetDockCoord` equality, and this contact-flag adjacent-building branch remains runtime-sensitive.
- Slot 5 owns the receiver-side `0x15` start-unload side effects; this report only proves the send and branch-local return handling.

## Sources

- Ghidra decompile `UnitClass::PerCellProcess @ 0x00739EC0`.
- Ghidra assembly contexts: `0x00739ECD..0x00739EE2`, `0x0073A558..0x0073A5E4`.
- Ghidra decompile `TechnoClass::Receive_Radio @ 0x006F4AB0`.
- Ghidra decompile `Look_up_building_in_cell @ 0x0047C520`.
- Ghidra decompile `MapClass::Get_CellClass @ 0x005657A0`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_PERCELLPROCESS_GETDOCKCOORD_VS_0X16_RECONCILIATION_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`.
- Current Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`.
