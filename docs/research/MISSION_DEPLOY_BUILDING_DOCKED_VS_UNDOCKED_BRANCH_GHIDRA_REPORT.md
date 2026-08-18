# Mission_Deploy_Building Docked vs Undocked Branch - Ghidra Research Report

**Address(es):** `0x0073D630` primary; `0x004595C0`, `0x004593A0`, `0x00458E50`, `0x0043C2D0`, `0x005B35E0`, `0x0065A970` supporting
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** The `UnitClass::Mission_Deploy_Building` branch split around `unit+0x2E4 == 0` vs nonzero for refinery unload, state-4 zero-link exit, `ReleaseDockedHarvester`/`UndockUnit` reachability, and stale prior-doc implications that stock DockUnload creates a reciprocal `+0x2E4` link.
**Non-Scope:** Exact `DAT_0089F6A0` runtime source/value, full credit math, full `BuildingClass::MissionRepairAndProduce`, and unrelated save/load or non-docking uses of `+0x2E4`.
**Confidence:** High for static branch ordering, active stock path, and teardown reachability. Medium only for "sole caller" inventory where this slot reused prior xref reports rather than re-running a full global xref sweep.
**Active in YR:** Yes for stock CMIN/HARV -> GAREFN/NAREFN zero-link unload. Conditional for reciprocal-link release helpers, because stock DockUnload does not establish that link.

## 1. Overview

`UnitClass::Mission_Deploy_Building @ 0x0073D630` has a real top-level split on `unit+0x2E4` (`param_1[0xB9]` in the decompiler). The old interpretation that stock refinery unloading first establishes this field and then normally exits through `ReleaseDockedHarvester` is wrong for standard YR refineries.

The stock CMIN/HARV -> GAREFN/NAREFN unload path enters and remains in the `unit+0x2E4 == 0` side: it rediscovers the refinery by adjacent-cell lookup, drains cargo in state 3, transitions to state 4, clears `unit+0x6D1`, optionally sends radio `0x03` if a radio contact still exists, queues/continues Harvest, and does not call `ReleaseDockedHarvester` or `UndockUnit`.

## 2. Class Layout / Key Offsets

| Owner | Offset | Purpose in this slice | Active in YR |
|---|---:|---|---|
| Unit/Building | `+0x2E4` (`[0xB9]`) | Reciprocal dock/garrison-style link used by Bunker and release/cleanup helpers, not set by stock DockUnload | Conditional |
| Unit | `+0x6D1` | Dock/unload active latch; set during zero-link unload startup and cleared on state-4 exit | Yes |
| Unit | `+0xBC` (`[0x2F]`) | Mission substate; zero-link harvester unload uses states `3` and `4` here | Yes |
| Unit | `+0x5A4` (`[0x169]`) | Foot/NavCom destination/contact object; state 4 tests it before deciding stop/mission path | Yes |
| Unit | `+0xB4` (`[0x2D]`) | Queued mission; state 4 treats `-1` or `10` differently from other queued missions | Yes |
| Unit | `+0x674` (`[0x19D]`) | Active locomotor pointer; state 4 may call locomotor `Is_Moving` then unit vtable `+0x500` | Yes |
| BuildingType | `+0x16B3` | `DockUnload=yes`; radio `0x15` queues sender mission `0x10` | Yes |
| BuildingType | `+0x16BB` | `Refinery=yes`; zero-link state-4 guard validates located building as refinery | Yes |
| BuildingType | `+0x16AB` | `Bunker=yes`; gates the reciprocal `+0x2E4` writer helper, not standard refinery DockUnload | Conditional |
| Building | `+0x57C` | `Anims_0[8]` / `ProductionAnim` pointer used as state-4 depart wait guard | Yes; usually zero for stock refineries |
| Building | `+0x718` | Bunker/release helper state cleared with `+0x2E4` in linked teardown paths | Conditional |

## 3. Core Logic

### 3.1 Entry split

At `0x0073D63B`, the function compares `[ESI+0x2E4]` with zero. The zero case jumps into the normal mission body; the nonzero case falls through to a building lookup and calls `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` at `0x0073D66D` if a building is found.

**Active in YR:** Yes for the function and branch check. The zero branch is the stock refinery path; the nonzero branch is conditional on a reciprocal link already existing.

### 3.2 Stock DockUnload reaches the zero-link side

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` reads `BuildingType+0x16B3`; if `DockUnload=yes`, it calls the sender's mission setter with mission `0x10` and argument `0` (`0x0043C788..0x0043C79E`). `MissionClass::Queue_Mission @ 0x005B35E0` only writes queued/current mission fields (`+0xB4`, mission done byte) for this call; it does not write `+0x2E4`.

Together with prior writer inventory, this means the stock handoff does not create the reciprocal `unit/building +0x2E4` link before `Mission_Deploy_Building` runs.

**Active in YR:** Yes. `rulesmd.ini` has `[CMIN] Dock=NAREFN,GAREFN` and `Harvester=yes` (`7361`, `7364`), `[HARV] Dock=NAREFN,GAREFN` and `Harvester=yes` (`8225`, `8228`), and stock `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes` (`11726-11727`, `12519-12520`).

### 3.3 Zero-link state 4 is the normal unload exit

In the zero-link state-4 branch, the function uses the unit cell plus the global adjacent offset (`DAT_0089F6A0/2`) to find the refinery, checks `BuildingType+0x16BB`, and if `building+0x57C != 0` returns `1` to wait another tick. If the guard is clear, it writes `unit+0x6D1 = 0` at `0x0073E1F6`.

After clearing `+0x6D1`, two paths exist:

- If `+0x5A4` is null, queued mission is `-1`, or queued mission is `10`, it calls `SetMission(10,0)` via vtable `+0x1E8`, may transmit radio `0x03` if `PathType__Has_Valid_Steps` is true, then calls vtable `+0x1EC`.
- Otherwise, it checks locomotor `+0x674`; if moving, it calls unit vtable `+0x500`, then calls vtable `+0x200` and, if true, vtable `+0x1EC`.

No call to `ReleaseDockedHarvester`, no call to `UndockUnit`, and no reciprocal `+0x2E4` clear occurs in this stock zero-link state-4 branch because no reciprocal link exists to clear.

**Active in YR:** Yes. This is the normal stock CMIN/HARV refinery completion path.

### 3.4 Nonzero-link `ReleaseDockedHarvester` path is conditional

If `unit+0x2E4 != 0`, `Mission_Deploy_Building` calls `ReleaseDockedHarvester` after looking up the building in the unit's cell. In `ReleaseDockedHarvester @ 0x004595C0`, the body reads `building+0x2E4`; if null, it clears `building+0x718`, sets building mission `5`, and returns. If non-null and the linked object reports `WhatAmI()==1`, it clears `unit+0x2E4` at `0x004596E6`, commands the locomotor, sets unit speed `1.0`, computes a passable destination, calls unit vtable `+0x480`, sets unit mission `MOVE=2`, then clears `building+0x2E4` and `building+0x718` at `0x00459814..0x0045981A` and sends radio `0x03`.

**Active in YR:** Conditional. The helper is live code, but it requires an existing reciprocal `+0x2E4` link. The checked stock DockUnload path does not create that link.

### 3.5 `UndockUnit` is not called by this stock exit

`BuildingClass::UndockUnit @ 0x004593A0` is a separate linked-unit teardown helper. It reads `building+0x2E4`, requires linked object `WhatAmI()==1`, commands locomotor forced track `0x47`, sets speed `1.0`, then clears `unit+0x2E4` and `building+0x2E4` before sending radio `0x03`.

No call from the stock zero-link state-4 branch to `UndockUnit` was found in this slice. Prior docs place `UndockUnit` on interrupt/destroy/temporal cleanup paths rather than normal stock DockUnload completion.

**Active in YR:** Conditional. Live helper, not the normal stock CMIN/HARV -> GAREFN/NAREFN unload exit.

### 3.6 The reciprocal writer is Bunker-gated

The verified reciprocal writer is `FUN_00458E50` case 5, which stores `building+0x2E4 = unit` at `0x00459301` and `unit+0x2E4 = building` at `0x0045930F`. The caller site at `0x0044B7A3` is immediately gated by a read of `BuildingType+0x16AB` (`Bunker=yes`) at `0x0044B797..0x0044B79F`.

This is the concrete source of the older "building-side linkage" confusion. It is not the stock refinery DockUnload path: GAREFN/NAREFN use `DockUnload=yes` and `Refinery=yes`; the checked stock rules do not mark them as `Bunker=yes`.

**Active in YR:** Conditional. Active for Bunker-capable buildings such as the YR bunker path; not active for stock refinery unload.

## 4. INI Keys

| Key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | Allows chrono miner to target stock refineries | Yes (`rulesmd.ini:7361`) |
| `[CMIN] Harvester` | `yes` | Enables harvester unload mission family | Yes (`rulesmd.ini:7364`) |
| `[HARV] Dock` | `NAREFN,GAREFN` | Allows Soviet harvester to target stock refineries | Yes (`rulesmd.ini:8225`) |
| `[HARV] Harvester` | `yes` | Enables harvester unload mission family | Yes (`rulesmd.ini:8228`) |
| `[GAREFN] DockUnload` / `Refinery` | `yes` / `yes` | Case `0x15` handoff and state-4 refinery guard | Yes (`rulesmd.ini:11726-11727`) |
| `[NAREFN] DockUnload` / `Refinery` | `yes` / `yes` | Case `0x15` handoff and state-4 refinery guard | Yes (`rulesmd.ini:12519-12520`) |
| `[NABNKR] Bunker` | `yes` | Enables `FUN_00458E50` reciprocal writer path | Conditional (`rulesmd.ini:13732`) |

## 5. Integration Points

| Function / branch | Status | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building` entry split | verified | `0x0073D63B` compare, `0x0073D641` zero-branch jump | Yes |
| Zero-link stock unload FSM | verified | decompile `0x0073D630`; state-4 `0x0073E17F..0x0073E289` | Yes |
| `BuildingClass::Receive_Radio` case `0x15` handoff | verified | `0x0043C788..0x0043C79E` | Yes |
| `MissionClass::Queue_Mission` immediate callee | verified | `0x005B35E0`; no `+0x2E4` write | Yes |
| `ReleaseDockedHarvester` branch | verified | `0x0073D66D -> 0x004595C0` | Conditional |
| `UndockUnit` helper | verified | `0x004593A0` body; no zero-link state-4 caller in this slice | Conditional |
| Bunker reciprocal writer | verified | `0x0044B797..0x0044B7A3`, `0x00459301`, `0x0045930F` | Conditional |

## 6. Current Rust Implementation Status

Not audited in this slot. Implementation-facing consequence: stock refinery unload should not require or synthesize a reciprocal `unit/building +0x2E4` link for normal CMIN/HARV -> GAREFN/NAREFN completion. If a Bunker/garrison-like path is modeled later, that is the path requiring reciprocal-link semantics and linked-unit teardown helpers.

## 7. Cross-Doc Reconciliation

`MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` correctly identified the top-level split and the `ReleaseDockedHarvester` call site, but its section 7 overreached by saying the first tick after arrival normally sees `param_1[0xB9] != 0`, with the link established by a building-side linkage call from `PerCellProcess`. Current evidence supersedes that: stock pad arrival and radio `0x15` do not write reciprocal `+0x2E4`; normal DockUnload runs the zero-link FSM.

`UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` uses broad names like `DockedTo` / `DockedUnit` for `[0xB9]`. Those names are acceptable for the conditional reciprocal-link helpers, but misleading for stock refinery DockUnload if read as the normal ore-refinery docking flag.

`STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`, and the 2026-05-21 correction in `miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` match this slot's findings: stock DockUnload uses the zero-link unload FSM; reciprocal release/Force_Track paths are conditional.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Entry branch on `unit+0x2E4` | verified | `0x0073D63B`, `0x0073D641` | none |
| Zero-link stock state-4 exit | verified | `0x0073E17F..0x0073E289` | none for branch split |
| `ReleaseDockedHarvester` reachability from `Mission_Deploy_Building` | verified | call at `0x0073D66D` | full global caller inventory reused from prior docs |
| `ReleaseDockedHarvester` body | verified | decompile/assembly `0x004595C0`, `0x004596E6`, `0x00459814` | none for branch split |
| `UndockUnit` body and stock non-reachability | verified for body, touched for caller context | decompile `0x004593A0`; no state-4 call | full caller inventory outside scope |
| Stock radio `0x15` handoff | verified | `0x0043C788..0x0043C79E`, `0x005B35E0` | none |
| Reciprocal writer identity | verified | `0x00459301`, `0x0045930F`; `0x0044B797..0x0044B7A3` Bunker gate | none for stock-refinery exclusion |
| Exact `DAT_0089F6A0` source/value | deferred | sibling swarm slot owns it | next slot/result |
| Full `BuildingClass::MissionRepairAndProduce` behavior | deferred | only Bunker gate inspected | separate slot owns DockUnload reachability |

## 9. Open Questions - Final State

[RESOLVED] OQ-1 - Does normal stock CMIN/HARV refinery unload enter `Mission_Deploy_Building` with `unit+0x2E4 != 0`? No. Stock DockUnload handoff and immediate callees do not set reciprocal `+0x2E4`; the stock unload FSM is the zero-link branch. Evidence: `0x0043C788..0x0043C79E`, `0x005B35E0`, `0x0073D63B`, prior writer inventory.

[RESOLVED] OQ-2 - What happens on the nonzero branch? It looks up the building and calls `ReleaseDockedHarvester @ 0x004595C0` at `0x0073D66D`, then continues into common mission logic. Active in YR: Conditional, only if a reciprocal link already exists.

[RESOLVED] OQ-3 - Does zero-link state 4 call `ReleaseDockedHarvester` or `UndockUnit`? No. It checks the refinery/slot-8 guard, clears `+0x6D1`, optionally stops moving, may send radio `0x03`, and queues/continues Harvest. Evidence: `0x0073E17F..0x0073E289`.

[RESOLVED] OQ-4 - Where did the old building-side `param_1[0xB9]` linkage idea come from? The reciprocal writer exists in `FUN_00458E50` case 5, but the caller is gated by `BuildingType+0x16AB` (`Bunker=yes`), not stock `DockUnload=yes` refineries. Evidence: `0x0044B797..0x0044B7A3`, `0x00459301`, `0x0045930F`.

[RESOLVED] OQ-5 - Is `UndockUnit` the normal stock state-4 exit? No. Its body is a conditional linked-unit teardown helper; no call from stock zero-link state 4 was found in this slice. Evidence: `0x004593A0` and `0x0073E17F..0x0073E289`.

[DEFERRED] OQ-6 - Exact runtime value/source of `DAT_0089F6A0`. Category: assigned-to-sibling-slot. This slot only needed to verify the zero-link branch uses it for refinery rediscovery.

[DEFERRED] OQ-7 - Full `BuildingClass::MissionRepairAndProduce` DockUnload reachability. Category: assigned-to-sibling-slot. This slot inspected only the Bunker gate needed to exclude the reciprocal writer from stock refinery DockUnload.

## Sources

- Ghidra read-only decompile: `0x0073D630`, `0x004595C0`, `0x004593A0`, `0x00458E50`, `0x0043C2D0`, `0x005B35E0`, `0x0065A970`, `0x0044B000`.
- Ghidra read-only assembly context: `0x0073D63B`, `0x0073D641`, `0x0073D66D`, `0x0073E17F..0x0073E289`, `0x004596E6`, `0x00459814`, `0x00459301`, `0x0045930F`, `0x0043C788`, `0x0044B797..0x0044B7A3`.
- Prior docs checked: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
