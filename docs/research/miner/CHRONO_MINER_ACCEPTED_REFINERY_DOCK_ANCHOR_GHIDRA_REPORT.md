# Chrono Miner Accepted Refinery Dock Anchor - Ghidra Research Report

**Address(es):** `0x0043C2D0` (`BuildingClass__Receive_Radio`), `0x0041BEA0` (`ObjectClass__Get_Cell_Packed`), `0x004D8FB0` (`FootClass__Receive_Radio`), `0x00447B20` (`BuildingClass__GetDockCoord`), `0x00739EC0` (`UnitClass__PerCellProcess`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Accepted standard YR CMIN/HARV -> GAREFN/NAREFN refinery `CAN_DOCK(0x0E)` move-to anchor, whether it is hardcoded, whether GAREFN/NAREFN differ, and its relation to physical pad/link and `GetDockCoord`.  
**Non-Scope:** Far-return fallback destination, post-unload exit track, busy-refinery queue eviction, unload cadence, and non-refinery docks.  
**Confidence:** High for the `CAN_DOCK` accepted anchor; Medium-High for the later physical arrival relation because this slot rechecked the key functions and reused the fresh `PerCellProcess` report for exact sub-addresses.  
**Active in YR:** Yes. Standard `[CMIN]`/`[HARV]` use `Dock=NAREFN,GAREFN` and `Harvester=yes`; standard `[GAREFN]`/`[NAREFN]` use `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.

> **Correction 2026-05-24 - accepted cell vs dock coord**
>
> The accepted `0x12` target remains `NW+(3,1)`, and it coincides with the
> stock art-opened/passable refinery pad cell. It does not coincide with the
> stock `GetDockCoord` / PerCellProcess equality coordinate, which is
> `NW+(2,1)` for 4x3 GAREFN/NAREFN through the active `Refinery=yes`
> `+0x16BB` branch. Do not use this doc to collapse accepted target,
> `GetDockCoord`, and `QueueingCell`; the current canonical split is in
> `miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.

## 1. Overview

When a standard YR Chrono Miner or War Miner has already contacted a standard ore refinery and `BuildingClass__Receive_Radio` accepts `CAN_DOCK(0x0E)`, the refinery computes the `MOVE_TO_CELL(0x12)` payload inline as the building NW cell plus `(3,1)`. This is not read from art `QueueingCell=`, is not selected by `GetDockCoord`, and does not differ between GAREFN and NAREFN on the stock DockUnload branch.

The later physical-arrival/link step is a separate gate: `UnitClass__PerCellProcess` compares the unit's current cell against the destination building's vtable `+0xA8` dock coordinate and then sends radio `0x15`. For stock GAREFN/NAREFN art, the hardcoded accepted cell `(NW+3,NW+1)` is also an unoccupied/opened refinery pad cell via `RemoveOccupy`, so the accepted radio anchor and the visible dock pad cell coincide in cell-space.

## 2. Key Offsets / Flags

| Class | Offset / slot | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingTypeClass` | `+0x16B3` | `DockUnload=yes`; selects standard refinery `0x0E` branch and `0x15` unload handoff | `0x0043C2D0`; `rulesmd.ini:11726`, `12519` | Yes |
| `BuildingTypeClass` | `+0x16BB` | `Refinery=yes`; later refinery helpers/arrival and unload context | `0x00447B20`; `rulesmd.ini:11727`, `12520` | Yes |
| `BuildingTypeClass` | `+0x16BC` | `Weeder=yes`; shares the hardcoded `0x0E` queue-cell branch, but not standard GAREFN/NAREFN | `0x0043C2D0` | No for standard GAREFN/NAREFN |
| `BuildingTypeClass` | `+0x1618/+0x161C` | parsed `QueueingCell=` storage, not read by accepted `0x0E` DockUnload branch | prior `RECEIVE_RADIO_CASE_0x0E...`; decompile `0x0043C2D0` has no load here | Yes as parsed data, No as accepted anchor input |
| `BuildingTypeClass` | `+0x1780/+0x1788` | `NumberOfDocks` / `DockingOffset[]`; used by `GetDockCoord` helipad/UnitRepair branch, not standard refinery accepted `0x0E` anchor | `0x00447B20` | Conditional; not for GAREFN/NAREFN `0x0E` anchor |
| `ObjectClass` | `+0x9C/+0xA0` | world X/Y; `Get_Cell_Packed` converts to NW cell by sign-correct `>> 8` | `0x0041BEA0` | Yes |

## 3. Core Logic

### 3.1 Accepted `CAN_DOCK(0x0E)` anchor

`BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x0E` first runs the active filter chain: base `TechnoClass__Receive_Radio`, power check, contact/free-slot checks, and `NEED_TO_MOVE(0x13)`.

For standard refinery acceptance, the branch condition is:

```text
if BuildingType.DockUnload || BuildingType.Weeder:
    packed = building.Get_Cell_Packed()
    move_cell = (packed.x + 3, packed.y + 1)
    payload = MapClass__Get_CellClass(move_cell)
    send MOVE_TO_CELL(0x12) to harvester
    if reply == ALREADY_THERE(0x14):
        send ENTER_DOCK(0x18)
        send TIMING_SYNC(0x16)
```

**Active in YR:** Yes. GAREFN/NAREFN both set `DockUnload=yes`, and CMIN/HARV use them through `Dock=NAREFN,GAREFN`.

**Finding:** The move-to cell is hardcoded `building_nw + (3,1)`. It does not read `QueueingCell=4,1`, foundation dimensions, `DockingOffset%d`, `NumberOfDocks`, or the specific refinery art ID.

### 3.2 No GAREFN vs NAREFN branch

There is no stock-refinery type split inside the accepted DockUnload branch. Once `Type+0x16B3` is true, the same `Get_Cell_Packed() + (3,1)` computation runs for both Allied and Soviet refineries.

For a refinery placed with NW cell `(10,10)`:

| Refinery | Accepted `0x12` move cell | Art `QueueingCell=` | Opened pad cell in art |
|---|---:|---:|---:|
| GAREFN | `(13,11)` | `(14,11)` from `QueueingCell=4,1` | `(13,11)` via `RemoveOccupy1=3,1` |
| NAREFN | `(13,11)` | `(14,11)` from `QueueingCell=4,1` | `(13,11)` via `RemoveOccupy8=3,1` |

**Active in YR:** Yes. `rulesmd.ini` and `artmd.ini` stock data set these flags/keys.

### 3.3 Relation to `QueueingCell=`

`QueueingCell=4,1` is present for both stock refineries (`artmd.ini:1716`, `1773`), but the accepted radio path does not consume it. The Westwood comment says it is used when a harvester was not allowed to reserve the docking cell/refinery; that is not the already-accepted `CAN_DOCK` branch being investigated here.

**Active in YR:** Conditional. The data exists in stock YR, but this accepted `0x0E` branch does not read it. Use of `QueueingCell` belongs to non-accepted/waiting or far-staging paths, outside this slot.

### 3.4 Relation to physical pad/link and `GetDockCoord`

`GetDockCoord @ 0x00447B20` is not called by the accepted `CAN_DOCK` move-cell computation. The `0x0E` branch uses only `Get_Cell_Packed`, `MapClass__Get_CellClass`, and radio transmit calls for the anchor.

`GetDockCoord` participates later, through movement/arrival logic. Fresh `UnitClass__PerCellProcess @ 0x00739EC0` evidence says the dock-arrival predicate requires mission `7`/`0x19`, a building destination, and equality between the unit's current cell and the destination building's vtable `+0xA8` dock coordinate; only then does it send radio `0x15` and trigger the unload mission handoff.

For stock GAREFN/NAREFN, the art files explicitly remove occupancy at `(3,1)`, so the accepted hardcoded cell is the physical passable refinery pad cell. This is why the accepted radio target and visible dock/link cell coincide in normal YR play, even though `QueueingCell=4,1` names the adjacent waiting cell.

**Active in YR:** Yes for the arrival predicate and the stock refinery pad opening. Evidence: `UNITCLASS_PERCELLPROCESS...:40-66`, `artmd.ini:1760`, `1795`.

## 4. INI Keys

| File | Section | Key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|---|---|
| `rulesmd.ini` | `[CMIN]` | `Dock` | `NAREFN,GAREFN` | Allows CMIN to target both stock refineries | Yes (`7361`) |
| `rulesmd.ini` | `[CMIN]` | `Harvester` | `yes` | Puts CMIN on harvester dock path | Yes (`7364`) |
| `rulesmd.ini` | `[CMIN]` | `Teleporter` | `yes` | Chrono-specific movement elsewhere; no branch in accepted building `0x0E` anchor | Yes (`7396`) |
| `rulesmd.ini` | `[HARV]` | `Dock` | `NAREFN,GAREFN` | Same stock refinery targets for War Miner | Yes (`8225`) |
| `rulesmd.ini` | `[HARV]` | `Harvester` | `yes` | Standard harvester path | Yes (`8228`) |
| `rulesmd.ini` | `[GAREFN]` | `DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | Selects standard refinery receiver path | Yes (`11726-11729`) |
| `rulesmd.ini` | `[NAREFN]` | `DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | Same as GAREFN | Yes (`12519-12521`) |
| `artmd.ini` | `[GAREFN]` | `QueueingCell` | `4,1` | Not read by accepted `0x0E`; waiting/far-staging data | Conditional (`1773`) |
| `artmd.ini` | `[NAREFN]` | `QueueingCell` | `4,1` | Not read by accepted `0x0E`; waiting/far-staging data | Conditional (`1716`) |
| `artmd.ini` | `[GAREFN]` | `RemoveOccupy1` | `3,1` | Opens the hardcoded accepted pad cell | Yes (`1795`) |
| `artmd.ini` | `[NAREFN]` | `RemoveOccupy8` | `3,1` | Opens the hardcoded accepted pad cell | Yes (`1760`) |
| `artmd.ini` | `[NAREFN]` | `;DockingOffset0` | commented `256,0,0` | Inactive; not read by accepted `0x0E` | No (`1725`) |

## 5. Integration Points

| Function | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | Computes accepted DockUnload/Weeder `MOVE_TO_CELL` as NW `(x+3,y+1)` | direct decompile | Yes |
| `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` | Converts building world X/Y fields to NW cell shorts | direct decompile | Yes |
| `FootClass__Receive_Radio @ 0x004D8FB0` case `0x12` | If already at payload cell returns `0x14`; otherwise sets destination and timestamp fields | direct decompile | Yes |
| `BuildingClass__GetDockCoord @ 0x00447B20` | Later dock-coordinate helper; not source of accepted `0x0E` anchor | direct decompile | Yes, but separate from accepted anchor |
| `UnitClass__PerCellProcess @ 0x00739EC0` | Later physical-arrival/link predicate; sends `0x15` after current cell equals dock coordinate | direct decompile plus `UNITCLASS_PERCELLPROCESS...` | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15` | Receiver of pad-arrival `0x15`; sets sender mission `0x10` for `DockUnload=yes` | direct decompile | Yes |

## 6. Current Rust Implementation Status

The current Rust code already separates accepted CAN_DOCK anchor from art `QueueingCell`.

| Rust item | Status vs this finding |
|---|---|
| `src/sim/miner/miner_dock_sequence.rs:88-90` | Matches accepted binary anchor: `refinery_can_dock_queue_cell(rx, ry) = (rx+3, ry+1)` |
| `src/sim/miner/miner_system.rs:1064-1072` | `refinery_dock_cell` ignores width/height/QueueingCell and delegates to accepted CAN_DOCK cell |
| `src/sim/miner/miner_dock_sequence.rs:70-81` | Separately keeps art `QueueingCell` for waiting/staging paths |
| `src/sim/miner/miner_dock_sequence.rs:333-336` | Computes accepted queue, pad, and exit cells separately, matching the distinction this slot verified |

No Rust files were modified by this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__Receive_Radio` case `0x0E` standard DockUnload branch | verified | `decompile_function 0x0043C2D0` | none for accepted anchor |
| `ObjectClass__Get_Cell_Packed` NW-cell source | verified | `decompile_function 0x0041BEA0` | none |
| `QueueingCell=` exclusion from accepted anchor | verified | `0x0043C2D0` decompile; no `+0x1618/+0x161C` load; prior ReadINI doc | none |
| GAREFN vs NAREFN accepted-anchor equality | verified | shared `DockUnload=yes` branch; `rulesmd.ini:11726`, `12519`; `artmd.ini:1716`, `1773` | none |
| Physical pad opening at `(3,1)` | verified | `artmd.ini:1760`, `1795` | none |
| `GetDockCoord` relation | verified for separation from `0x0E` anchor; touched for exact later coord math | `decompile_function 0x00447B20`; `UNITCLASS_PERCELLPROCESS...:40-66` | no further work needed for accepted anchor; exact lepton-centering belongs to dock-arrival slot |
| Far-return fallback destination | deferred | out-of-scope by swarm assignment | slot 2 |
| Post-unload exit anchor | deferred | out-of-scope by swarm assignment | slot 5 |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Which function computes the accepted move-to cell after `CAN_DOCK` succeeds? `BuildingClass__Receive_Radio @ 0x0043C2D0` computes it inline in case `0x0E`; evidence: direct decompile.

[RESOLVED] OQ-2 - Is the accepted move-to cell hardcoded building anchor `(3,1)`? Yes. It calls `Get_Cell_Packed @ 0x0041BEA0`, then sends `MapClass__Get_CellClass(NW.x+3,NW.y+1)` as the `0x12` payload; evidence: direct decompile.

[RESOLVED] OQ-3 - Does stock GAREFN differ from NAREFN? No for this accepted anchor. Both enter the same `DockUnload=yes` branch and both have `QueueingCell=4,1` only as non-accepted/waiting data; evidence: `rulesmd.ini:11726`, `12519`, `artmd.ini:1716`, `1773`.

[RESOLVED] OQ-4 - Is `QueueingCell=4,1` used by accepted `CAN_DOCK`? No. The accepted branch never reads the parsed fields; evidence: `0x0043C2D0` decompile and prior `RECEIVE_RADIO_CASE_0x0E...` ReadINI mapping.

[RESOLVED] OQ-5 - How does this relate to physical pad/link? The accepted cell `(NW+3,NW+1)` is opened by stock refinery art (`GAREFN RemoveOccupy1=3,1`, `NAREFN RemoveOccupy8=3,1`); later `PerCellProcess` handles arrival/link by comparing current cell to the building dock coordinate and then sending `0x15`; evidence: `artmd.ini:1760`, `1795`, `UNITCLASS_PERCELLPROCESS...:40-66`.

[DEFERRED] OQ-6 - Exact far-return staging when not yet accepted by `CAN_DOCK`. Category: out-of-scope. Reason: assigned to swarm slot 2.

[DEFERRED] OQ-7 - Exact post-unload exit anchor and `Force_Track(0x47)` relation. Category: out-of-scope. Reason: assigned to swarm slot 5.

## Sources

- Ghidra `decompile_function 0x0043C2D0` - `BuildingClass__Receive_Radio`.
- Ghidra `decompile_function 0x0041BEA0` - `ObjectClass__Get_Cell_Packed`.
- Ghidra `decompile_function 0x004D8FB0` - `FootClass__Receive_Radio`.
- Ghidra `decompile_function 0x00447B20` - `BuildingClass__GetDockCoord`.
- Ghidra `decompile_function 0x00739EC0` - `UnitClass__PerCellProcess`.
- `docs/research/RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`.
- `docs/research/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`.
- `docs/research/REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`.
- `docs/research/UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`.
- `docs/research/traces/chrono_miner_return_state_anchor_reserved_refinery_TRACE.md`.
- `ini/rulesmd.ini:7361`, `7364`, `7396`, `8225`, `8228`, `11726-11729`, `12519-12521`.
- `ini/artmd.ini:1716`, `1725`, `1760`, `1773`, `1795`.
