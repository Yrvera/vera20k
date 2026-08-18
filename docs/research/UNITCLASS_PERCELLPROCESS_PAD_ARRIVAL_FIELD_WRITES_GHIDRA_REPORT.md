# UnitClass::PerCellProcess Pad Arrival Field Writes - Ghidra Research Report

**Address(es):** `0x00739EC0` primary hot path, with receiver/integration checks at `0x0043C2D0`, `0x006F4AB0`, `0x00737430`, `0x0073D630`, and `0x00458E50`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Stock YR CMIN/HARV -> GAREFN/NAREFN dock-pad arrival field writes/clears around `UnitClass::PerCellProcess @ 0x00739EC0`, specifically `+0x418`, `+0x5A4`, `+0x6D1`, `+0x2E4`, radio `0x15`, and locomotor slot `+0x5C` ordering.
**Non-Scope:** Full refinery unload economics, full mission-10 dump loop, full radio protocol, all non-refinery buildings, and complete writer inventory outside the pad-arrival chain.
**Confidence:** High for this slice.
**Active in YR:** Yes for stock CMIN/HARV docking to GAREFN/NAREFN. `[CMIN]` and `[HARV]` use `Dock=NAREFN,GAREFN` and `Harvester=yes`; `[GAREFN]` and `[NAREFN]` use `DockUnload=yes` and `Refinery=yes`.

## 1. Overview

`UnitClass::PerCellProcess @ 0x00739EC0` does not write reciprocal `unit/building +0x2E4` on the stock ore-refinery pad-arrival path. The hot path compares the arrived building pointer against `unit+0x5A4`, calls `FootClass::PerCellProcess(2)`, sends radio `0x15`, then calls the locomotor vtable slot `+0x5C`.

The field activity is split across nearby stages: radio `0x18` writes `unit+0x418 = 1` before physical arrival; `UnitClass::PerCellProcess` only reads `+0x418` in a later fallback/cascade branch; `+0x6D1` is initialized/cleared later in `UnitClass::Mission_Deploy_Building`; and the only verified reciprocal `+0x2E4` writer in this checked chain is a separate `FUN_00458E50` path, not the stock `DockUnload=yes` refinery pad-arrival branch.

## 2. Key Offsets

| Offset / slot | Behavior in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0x418` | Radio `0x18` dock-admission flag. Written by `TechnoClass::Receive_Radio`, read by `UnitClass::PerCellProcess` and `UnitClass::Receive_Radio`. | `0x006F4B72` writes `1`; `0x006F4BA6` writes `0`; `0x0073A558` reads | Yes |
| Unit `+0x5A4` | Dock/radio destination link compared at pad arrival. Optionally filled in this branch only for `BuildingType+0x16A9` if currently null. | `0x0073A4DF` read, `0x0073A4E9` conditional write, `0x0073A4EF` compare | Yes as link/read; conditional write is not stock refinery |
| Unit `+0x6D1` | Mission-10 unload FSM initialized flag. Not written by pad arrival; set/cleared after receiver sets mission `0x10`. | `0x0073DFDA` set to `1`; `0x0073DEF8` and `0x0073E1F6` clear to `0` | Yes |
| Unit/Building `+0x2E4` | Not written by pad arrival. Reciprocal writer exists in a separate path (`FUN_00458E50`). | No `+0x2E4` access in `0x0073A4D5..0x0073A52B`; writer at `0x00459301`/`0x0045930F` | Conditional outside this slice; no for stock refinery pad arrival |
| Unit locomotor `+0x674` | Reloaded after radio `0x15`; vtable slot `+0x5C` called after radio send. | `0x0073A50D`, `0x0073A521`, `0x0073A52B` | Yes |
| Unit vtable `+0x274` | Sends radio `0x15` to current/first radio contact at pad arrival. | `0x0073A503`, `0x0073A507` | Yes |
| Unit vtable `+0x278` | Directed radio send. Used by nearby `+0x418` cascade branch and by `UnitClass::Receive_Radio(0x16)`, not by the primary arrival send. | `0x0073A5C3..0x0073A5C8`, `0x00737776..0x0073777A` | Conditional |
| BuildingType `+0x16B3` | `DockUnload=yes`; receiver case `0x15` sets sender mission `0x10`. | `BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x15`; INI GAREFN/NAREFN | Yes |

## 3. Core Logic

### 3.1 Pad-arrival predicate

The checked `UnitClass::PerCellProcess` hot path is entered when:

1. The current mission is `7` or `0x19`.
2. `FootClass::GetDestination(0)` returns a non-null destination.
3. The destination's `WhatAmI()` returns `6` (building).
4. The unit's current cell matches the destination building's dock cell after both positions are converted to cell-center coordinates with `cell * 0x100 + 0x80`.
5. The unit has a locomotor pointer and the piggyback CLSID comparison matches `CLSID_WalkLocomotion`.

**Active in YR:** Yes. This is the stock pad-arrival edge for CMIN/HARV once the refinery admission chain has sent the miner to the dock cell.

### 3.2 Exact hot-path field writes and ordering

The immediate arrival block is:

1. Check building type byte `+0x16A9`.
2. If `+0x16A9 != 0` and `unit+0x5A4 == 0`, write `unit+0x5A4 = destination`.
3. Compare `destination == unit+0x5A4`.
4. If equal, call `FootClass::PerCellProcess(2)`.
5. Send radio `0x15` through unit vtable `+0x274`.
6. Reload `unit+0x674`, assert if null.
7. Call locomotor vtable `+0x5C`.
8. Release the piggyback interface if present and return.

Assembly evidence:

| Address | Instruction / effect |
|---|---|
| `0x0073A4D5` | read `byte [ECX + 0x16A9]` from building type |
| `0x0073A4DF` | read `dword [EBP + 0x5A4]` |
| `0x0073A4E9` | conditional write `dword [EBP + 0x5A4] = EAX` |
| `0x0073A4EF` | compare destination `EAX` with `dword [EBP + 0x5A4]` |
| `0x0073A4F7..0x0073A4FB` | push `2`, call `FootClass::PerCellProcess @ 0x004D85D0` |
| `0x0073A503..0x0073A507` | push `0x15`, call `unit.vtable+0x274` |
| `0x0073A50D..0x0073A521` | reload/check `unit+0x674` |
| `0x0073A52B` | call `locomotor.vtable+0x5C` |

**Active in YR:** Yes for the sequence. The `+0x5A4` write is Conditional and appears tied to the `+0x16A9` building-type branch, not stock GAREFN/NAREFN `DockUnload=yes`.

### 3.3 `+0x418` around pad arrival

`UnitClass::PerCellProcess` reads `+0x418` immediately after the primary arrival block returns or falls through:

 ```text
0x0073A558: MOV AL, byte ptr [EBP + 0x418]
0x0073A55E: TEST AL, AL
```

If set, and the destination is a building and mission is `7`, a nearby branch checks the cell one row above the current unit cell and may send directed radio `0x15` through vtable `+0x278`:

```text
0x0073A5C3: PUSH ESI
0x0073A5C4: PUSH 0x15
0x0073A5C8: CALL dword ptr [EDX + 0x278]
```

This is not a write to `+0x418`. The verified writer is `TechnoClass::Receive_Radio @ 0x006F4AB0`: case `0x18` writes `byte [this+0x418] = 1` at `0x006F4B72`; case `0x19` writes `byte [this+0x418] = 0` at `0x006F4BA6`.

**Active in YR:** Yes. `BuildingClass::Receive_Radio(0x0E)` sends `0x18` to accepted DockUnload miners before pad arrival.

### 3.4 `+0x6D1` around pad arrival

`UnitClass::PerCellProcess` does not set or clear `+0x6D1` in the immediate pad-arrival block. `+0x6D1` is part of the subsequent mission `0x10` unload FSM:

| Address | Function | Effect |
|---|---|---|
| `0x0073DEF8` | `UnitClass::Mission_Deploy_Building` | clear `byte [ESI + 0x6D1] = 0` when no valid path exists at mission entry/cleanup |
| `0x0073DFDA` | `UnitClass::Mission_Deploy_Building` | set `byte [ESI + 0x6D1] = 1` when the unload FSM initializes |
| `0x0073E1F6` | `UnitClass::Mission_Deploy_Building` | clear `byte [ESI + 0x6D1] = 0` during later unload/exit cleanup |
| `UnitClass::Receive_Radio(0x17)` | `0x00737430` | if harvester flags and `+0x6D1 != 0`, clear `+0x6D1`, set mission `10`, and redraw |

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` is the bridge from pad arrival into that FSM: for `DockUnload=yes` it calls the sender unit mission setter with mission `0x10`, queued flag `0`, and returns `1`.

**Active in YR:** Yes. GAREFN/NAREFN have `DockUnload=yes`; CMIN/HARV reach mission `0x10` via this handoff.

### 3.5 `+0x2E4` around pad arrival

No reciprocal `+0x2E4` write occurs in the `0x0073A4D5..0x0073A52B` pad-arrival block. The direct field writes in that block are to `+0x5A4` only; `+0x674` is read for the locomotor call.

The separate reciprocal writer verified in this investigation is:

```text
0x00459301: MOV dword ptr [ESI + 0x2E4], EBP
0x0045930F: MOV dword ptr [EBP + 0x2E4], ESI
0x00459327: MOV dword ptr [ESI + 0x718], 0x6
0x00459331..0x00459337: SetMission(5, queued=1) on the linked unit
```

That writer is in `FUN_00458E50`, reached from `BuildingClass::MissionRepairAndProduce @ 0x0044B780` under a building-type `+0x16AB` gate. It is outside the stock `DockUnload=yes` refinery pad-arrival branch checked here.

**Active in YR:** Conditional. The reciprocal writer is real binary code, but this report found no evidence that stock CMIN/HARV -> GAREFN/NAREFN pad arrival uses it. For the requested stock refinery path, Active in YR: No.

## 4. INI Keys

| INI path | Value | Effect for this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN]` | `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Teleporter=yes` | CMIN uses the stock harvester refinery docking chain. | Yes (`rulesmd.ini:7361`, `7364`, `7396`) |
| `rulesmd.ini:[HARV]` | `Dock=NAREFN,GAREFN`, `Harvester=yes` | Regular miner uses the same pad-arrival handoff. | Yes (`rulesmd.ini:8225`, `8228`) |
| `rulesmd.ini:[GAREFN]` | `DockUnload=yes`, `Refinery=yes` | Receiver case `0x15` sets sender mission `0x10`. | Yes (`rulesmd.ini:11726`, `11727`) |
| `rulesmd.ini:[NAREFN]` | `DockUnload=yes`, `Refinery=yes` | Same as GAREFN. | Yes (`rulesmd.ini:12519`, `12520`) |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | Radio `0x18`/`0x19` writes/clears `+0x418`. | `0x006F4B72`, `0x006F4BA6` | Yes |
| `UnitClass::Receive_Radio @ 0x00737430` | Radio `0x16` can send directed `0x15` after checking destination building, `+0x418`, and mission `7`. | `0x00737776..0x0073777A` | Yes |
| `UnitClass::PerCellProcess @ 0x00739EC0` | Primary physical pad-arrival sequence: `PerCellProcess(2)` -> radio `0x15` -> locomotor `+0x5C`. | `0x0073A4F7..0x0073A52B` | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | Receiver case `0x15` sets sender mission `0x10` for `DockUnload=yes`. | decompile case `0x15` | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | Mission `0x10` owns `+0x6D1` initialization/cleanup. | `0x0073DEF8`, `0x0073DFDA`, `0x0073E1F6` | Yes |
| `FUN_00458E50` | Separate reciprocal `+0x2E4` writer. | `0x00459301`, `0x0045930F` | Conditional, not stock refinery pad arrival |

## 6. Current Rust Implementation Status

Not scanned in this subagent pass. The requested scope was binary field-write tracing only, and no Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::PerCellProcess @ 0x00739EC0` pad-arrival hot path | verified | `0x0073A4D5..0x0073A52B` | none for requested fields/order |
| `+0x5A4` write in the hot path | verified | `0x0073A4E9`, gated by `+0x16A9` and null `+0x5A4` | exact INI semantic of `+0x16A9` outside stock refinery scope |
| `+0x418` behavior around arrival | verified | read at `0x0073A558`; writes at `0x006F4B72`/`0x006F4BA6` | none for requested scope |
| `+0x6D1` writes/clears | verified | `0x0073DEF8`, `0x0073DFDA`, `0x0073E1F6` | full unload FSM economics out-of-scope |
| `+0x2E4` in pad-arrival hot path | verified absent | decompile/assembly of `0x0073A4D5..0x0073A52B`; separate writer at `0x00459301`/`0x0045930F` | full writer inventory assigned to another swarm slot |
| Radio `0x15` and locomotor `+0x5C` ordering | verified | `0x0073A4F7`, `0x0073A503`, `0x0073A52B` | none |
| Receiver mission handoff after `0x15` | verified | `BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x15` | none |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does pad arrival write reciprocal `+0x2E4`? No. The immediate `UnitClass::PerCellProcess` block writes only conditional `+0x5A4`; reciprocal `+0x2E4` writes are in `FUN_00458E50` at `0x00459301`/`0x0045930F`, outside this stock refinery pad-arrival slice.

[RESOLVED] OQ-2 - What is the exact radio/locomotor order? `FootClass::PerCellProcess(2)` at `0x0073A4F7..0x0073A4FB`, then radio `0x15` at `0x0073A503..0x0073A507`, then locomotor `+0x5C` at `0x0073A52B`.

[RESOLVED] OQ-3 - Does pad arrival write `+0x418`? No. It reads `+0x418` at `0x0073A558`; radio `0x18` writes it at `0x006F4B72`, and radio `0x19` clears it at `0x006F4BA6`.

[RESOLVED] OQ-4 - Does pad arrival write `+0x6D1`? No. `+0x6D1` is written in `UnitClass::Mission_Deploy_Building` after receiver-side mission `0x10` handoff.

[RESOLVED] OQ-5 - Does stock GAREFN/NAREFN require the conditional `+0x5A4` write at `0x0073A4E9`? Not on the checked stock DockUnload path. That write is gated by building type byte `+0x16A9`; stock refineries are verified through `DockUnload=yes` receiver case `0x15`.

## Sources

- Ghidra `decompile_function 0x00739EC0` - `UnitClass::PerCellProcess`.
- Ghidra `get_assembly_context` around `0x0073A4D5`, `0x0073A4E9`, `0x0073A4F7`, `0x0073A503`, `0x0073A50D`, `0x0073A521`, `0x0073A52B`.
- Ghidra `decompile_function 0x006F4AB0` and assembly at `0x006F4B72`, `0x006F4BA6` - `TechnoClass::Receive_Radio`.
- Ghidra `decompile_function 0x00737430` and assembly at `0x00737776..0x0073777A` - `UnitClass::Receive_Radio`.
- Ghidra `decompile_function 0x0043C2D0` - `BuildingClass::Receive_Radio`.
- Ghidra `decompile_function 0x0073D630` and assembly at `0x0073DEF8`, `0x0073DFDA`, `0x0073E1F6` - `UnitClass::Mission_Deploy_Building`.
- Ghidra assembly at `0x00459301`, `0x0045930F` - separate reciprocal `+0x2E4` writer in `FUN_00458E50`.
- Ghidra `decompile_function 0x0044B780` - `BuildingClass::MissionRepairAndProduce` context for `FUN_00458E50`.
- `ini/rulesmd.ini` lines for CMIN/HARV/GAREFN/NAREFN activation.
- Prior docs: `UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`, `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`, `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`.
