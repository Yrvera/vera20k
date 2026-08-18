# UnitClass/TechnoClass `+0x418` Dock Flag Lifecycle And Consumers - Ghidra Research Report

**Address(es):** `0x006F2B40`, `0x006F4AB0`, `0x004D8FB0`, `0x0043C2D0`, `0x00737430`, `0x00739EC0`, `0x00741970`, `0x0073D630`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `TechnoClass` byte field `+0x418` as used by stock YR docking/NavCom radio flow: init, `0x18`/`0x19` writes, UnitClass receive/per-cell readers, cancel/break clear paths, and stock DockUnload departure impact.  
**Non-Scope:** every non-dock use of the same byte, save/load serialization, and a full global field-name audit for all Techno-derived classes.  
**Confidence:** High for the dock/NavCom slice.  
**Active in YR:** Yes for GAREFN/NAREFN DockUnload admission and UnitClass per-cell cleanup; Conditional for `0x19` clear paths because they require a break/cancel/cleanup message.

## 1. Overview

`+0x418` is a one-byte Techno-derived radio/contact flag initialized to `0`. In the stock GAREFN/NAREFN admission burst, the refinery sends `0x18` to the miner, which sets the miner's `+0x418 = 1` before propagating `0x18` back to the radio partner.

The field is not a reciprocal dock pointer and is not `+0x2E4`. It gates fallback/cascade radio behavior in `UnitClass::Receive_Radio`, `UnitClass::PerCellProcess`, and `UnitClass::Set_Destination`. Normal DockUnload state-4 exit itself clears `+0x6D1` and queues Harvest; it does not read or clear `+0x418`. However, if `+0x418` survives after unloading, the later UnitClass per-cell cleanup path can consume it and transmit radio `0x08`, which leads the partner-side TechnoClass radio handler to send `0x19` and `0x03` back through the radio link.

## 2. Class Layout / Key Offsets

| Field | Offset | Type | Verified meaning in this slice | Active in YR |
|---|---:|---|---|---|
| Techno/Unit dock/contact flag | `+0x418` | byte | Set by radio `0x18`; cleared by radio `0x19`; read by dock fallback/cleanup branches | Yes / Conditional |
| Adjacent byte | `+0x419` | byte | Constructor initializes to `0`; radio `0x1A/0x1B` toggle it, not part of this dock slice | Conditional, out-of-scope |
| Unit current destination pointer | `+0x5A4` | ptr | Used with `+0x418` to decide whether fallback/cascade should target a building | Yes |
| Unit unload-active latch | `+0x6D1` | byte | DockUnload FSM latch; state-4 exit clears this, not `+0x418` | Yes |
| Unit reciprocal dock link | `+0x2E4` | ptr | Not written by TechnoClass `0x18/0x19`; separate path | Conditional, not stock zero-link DockUnload |

Evidence: constructor `0x006F2B40`; radio writes at `0x006F4B72` and `0x006F4BA6`; per-cell reads at `0x0073A558`, `0x0073A936`; Unit receive read at `0x0073774A`; Set_Destination read at `0x00741B4A`; unload state-4 `+0x6D1` clear at `0x0073E1F6`.

## 3. Core Logic

### 3.1 Initialization

`TechnoClass::Constructor @ 0x006F2B40` initializes byte `+0x418` to `0` via the `param_1[0x106]` byte store. Neighboring bytes `+0x419..+0x41F` are also initialized in the same constructor block, which confirms this is a byte-level flag cluster rather than an `int` pointer.

**Active in YR:** Yes. All UnitClass miners inherit this constructor path.

### 3.2 Standard refinery admission sets the flag

`BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x0E`, is the accepted docking admission path for standard `DockUnload=yes` refineries. For GAREFN/NAREFN it sends:

1. `0x12` with a cell payload at building map cell `+(3,1)`.
2. Directed `0x18` to the miner.
3. Directed `0x16` to the miner.

The directed `0x18` reaches UnitClass through the receive-radio chain. UnitClass has no direct case `0x18`; `FootClass::Receive_Radio @ 0x004D8FB0` also has no direct case `0x18`, so it falls through to `TechnoClass::Receive_Radio @ 0x006F4AB0`.

**Active in YR:** Yes. `rulesmd.ini` has `[GAREFN] DockUnload=yes` at line 11726 and `[NAREFN] DockUnload=yes` at line 12519.

### 3.3 `TechnoClass::Receive_Radio(0x18)` write semantics

For message `0x18`, `TechnoClass::Receive_Radio @ 0x006F4AB0`:

- Skips the set/propagate block for AircraftClass objects whose type has `+0xE0D` set.
- Otherwise reads `this+0x418`.
- If the byte is `0`, writes `1` at `0x006F4B72`.
- Calls vtable slot `+0x278` with message `0x18` after the write; the assembly context shows the store before the call.
- Returns `1`.
- If the byte is already nonzero, it does not rewrite or propagate and falls through to base radio handling.

**Active in YR:** Yes for standard UnitClass CMIN/HARV refinery docking. The AircraftClass `+0xE0D` gate is not a CMIN/HARV condition.

### 3.4 `TechnoClass::Receive_Radio(0x19)` clear semantics

For message `0x19`, `TechnoClass::Receive_Radio @ 0x006F4AB0`:

- Reads `this+0x418`.
- If the byte is nonzero, writes `0` at `0x006F4BA6`.
- Calls vtable slot `+0x278` with message `0x19` after the clear; the assembly context shows the clear before the call.
- Returns `1`.
- If the byte is already zero, it does not rewrite or propagate and falls through to base radio handling.

**Active in YR:** Conditional. The code is live, but normal uninterrupted DockUnload state-4 exit does not itself send `0x19`; cancel/break/cleanup paths do.

### 3.5 UnitClass `0x16` reader/cascade

`UnitClass::Receive_Radio @ 0x00737430`, case `0x16`, calls `FootClass::Receive_Radio` first. That fallthrough reaches TechnoClass case `0x16`, which sends `0x18` to the sender without directly writing `+0x418`.

After that base call, UnitClass case `0x16` reads `this+0x418` at `0x0073774A`. If the locomotor is not in its early timing adjustment, the unit is not chrono timer-gated, the destination is a building, `+0x418 != 0`, and mission is `7`, it sends directed `0x15` to the destination at `0x00737776..0x0073777A`.

**Active in YR:** Yes. The standard refinery admission burst sends `0x16` after `0x18`.

### 3.6 UnitClass per-cell readers

`UnitClass::PerCellProcess @ 0x00739EC0` has two relevant `+0x418` reader zones:

1. Pad-arrival fallback/cascade near `0x0073A558`: requires `+0x418 != 0`, a destination building, and mission `7`. It probes the cell one row above the unit and can send directed `0x15` at `0x0073A5C3..0x0073A5C8`. This is not the primary physical pad-arrival branch; the primary branch already sends `0x15` after `FootClass::PerCellProcess(2)`.
2. General cleanup after the arrival block: if `+0x418 != 0` and mission is neither `7` in the accepted-destination case nor mission `0x10`, it can send radio `0x08` to the first radio contact at `0x0073A936..0x0073A93D`.

The `0x08` path matters because `TechnoClass::Receive_Radio(0x08)` sends `0x19` and then `0x03` back through vtable `+0x278`; assembly context at `0x006F4C34..0x006F4C41` confirms the two sends. The receiving TechnoClass `0x19` handler is the verified clear for `+0x418`.

**Active in YR:** Yes for the per-cell function and cleanup branch; Conditional for actual execution after DockUnload because it depends on the unit leaving the building cell with the radio contact still present.

### 3.7 Set_Destination cancel path

`UnitClass` uses the vtable `+0x480` target at `0x00741970` for Set_Destination. In the cancel-dock branch it reads `this+0x418` at `0x00741B4A`. If there is a valid path, mission check passes, this unit is a UnitClass, `+0x418 != 0`, the current destination is an AircraftClass object, and that aircraft type has its `+0xDFC` carryall gate set, the function clears the contact destination and transmits `0x19` then `0x03`.

**Active in YR:** Conditional. The function is live for UnitClass, but this specific `+0xDFC` branch is aircraft/carryall-gated and is not the standard CMIN/HARV-to-refinery path.

### 3.8 DockUnload departure impact

`UnitClass::Mission_Deploy_Building @ 0x0073D630` is the stock DockUnload mission `0x10` unload FSM. On the zero-link stock refinery path it uses adjacent-cell refinery lookup, cargo draining, `+0x6D1`, `+0x5A4`, facing, and anim slots. The state-4 exit clears `+0x6D1` at `0x0073E1F6` and queues Harvest (`mission 10 / 0x0A`), with no `+0x418` read or clear in that state-4 block.

The departure impact is therefore indirect: state-4 does not consume `+0x418`, but subsequent `UnitClass::PerCellProcess` can consume a still-set `+0x418` in the general cleanup branch and emit radio `0x08`, which cascades to `0x19`/`0x03` and clears the radio-tether flag.

**Active in YR:** Yes for state-4 DockUnload exit; Conditional for the later cleanup cascade depending on remaining contact state and cell/building conditions.

## 4. INI Keys

| Key | Path | Value in stock YR | Effect in this slice |
|---|---|---|---|
| `DockUnload` | `rulesmd.ini:[GAREFN]`, `[NAREFN]` | `yes` | Enables the building case `0x15` handoff to sender mission `0x10` and the standard refinery admission route |
| `Refinery` | `rulesmd.ini:[GAREFN]`, `[NAREFN]` | `yes` | Marks the building as a refinery; adjacent lookup and unload behavior depend on refinery type flags |
| `Dock` | `rulesmd.ini:[CMIN]`, `[HARV]` | `NAREFN,GAREFN` | Lets harvesters target stock refineries |
| `QueueingCell` | `artmd.ini:[GAREFN]`, `[NAREFN]` | `4,1` | Not the accepted radio `0x0E` cell; that path uses hardcoded `+(3,1)` |

No INI key was found that directly maps to Techno instance byte `+0x418`; it is runtime radio state.

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass::Constructor` | Initializes `+0x418 = 0` | `0x006F2B40`, `param_1[0x106] = 0` | Yes |
| `BuildingClass::Receive_Radio` | Sends `0x12`, `0x18`, `0x16` on accepted refinery docking | `0x0043C2D0` case `0x0E`; `rulesmd.ini` DockUnload lines | Yes |
| `FootClass::Receive_Radio` | Falls through unhandled `0x18/0x19` to TechnoClass | `0x004D8FB0` switch lacks `0x18/0x19` direct cases | Yes |
| `TechnoClass::Receive_Radio` | Writes and clears `+0x418`; propagates `0x18/0x19` | `0x006F4B72`, `0x006F4BA6` | Yes / Conditional |
| `UnitClass::Receive_Radio` | Reads `+0x418` in case `0x16` directed `0x15` cascade | `0x0073774A`, `0x00737776` | Yes |
| `UnitClass::PerCellProcess` | Reads `+0x418` for fallback arrival and post-contact cleanup | `0x0073A558`, `0x0073A936` | Yes / Conditional |
| `UnitClass::Set_Destination` | Reads `+0x418` in aircraft/carryall leave-dock cancel branch | `0x00741B4A`, `0x00741B97` | Conditional |
| `UnitClass::Mission_Deploy_Building` | Stock unload exit does not read/clear `+0x418`; clears `+0x6D1` | `0x0073D630`, `0x0073E1F6` | Yes |

## 6. Current Rust Implementation Status

Rust currently models refinery contact/pad state with `RefineryDockContacts` and `Miner::dock_phase`, not a direct `+0x418` field:

- `src/sim/miner/miner_dock.rs` has `contacts`, `waiting_retry_queue`, and `on_pad`.
- `src/sim/miner/miner_dock_sequence.rs` has `Approach -> Linked -> Pivoting -> Unloading -> DepositCooldown -> Departing`.
- `src/sim/world/world_hash.rs` hashes dock contacts, wait queues, and pad occupancy.

This is a clean-Rust equivalent surface, but the binary slice requires three observable/ordering constraints for parity: `0x18` admission state is established before `0x16` timing sync, `0x15` can be cascaded only after the `+0x418`/destination/mission gates, and the post-unload cleanup must be able to break a lingering radio contact after the unload FSM itself has already queued Harvest.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Constructor init of `+0x418` | verified | `0x006F2B40` | none for dock slice |
| Building admission sends `0x18`/`0x16` | verified | `0x0043C2D0`; `rulesmd.ini` GAREFN/NAREFN | none |
| FootClass fallback for `0x18/0x19` | verified | `0x004D8FB0` | none |
| TechnoClass `0x18` set | verified | `0x006F4B72`, call after store at `0x006F4B79` | none |
| TechnoClass `0x19` clear | verified | `0x006F4BA6`, call after clear at `0x006F4BAD` | none |
| TechnoClass `0x08` break cascade | verified | `0x006F4C34..0x006F4C41` | exact sender inventory belongs to radio `0x08` slot |
| UnitClass `0x16` `+0x418` cascade | verified | `0x0073774A`, `0x00737776..0x0073777A` | none |
| PerCellProcess fallback `0x15` reader | verified | `0x0073A558`, `0x0073A5C3..0x0073A5C8` | rare trigger frequency needs runtime trace |
| PerCellProcess cleanup `0x08` reader | verified | `0x0073A936..0x0073A93D` | exact normal-cycle timing needs runtime trace |
| Set_Destination cancel branch | verified | `0x00741B4A`, `0x00741B97` | not stock CMIN refinery |
| Mission_Deploy_Building state-4 exit | verified | `0x0073D630`; `+0x6D1` clear at `0x0073E1F6` | no `+0x418` direct role found |
| Non-dock consumers of `+0x418` | deferred | out-of-scope | global struct-layout audit |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-418-001 - Is +0x418 initialized before radio use? -> Yes, `TechnoClass::Constructor` initializes byte `+0x418` to `0`.` (evidence: `0x006F2B40`)
- `[RESOLVED] OQ-418-002 - Does radio 0x18 write +0x418 or +0x2E4? -> It writes byte `+0x418 = 1`, not `+0x2E4`.` (evidence: `0x006F4B72`)
- `[RESOLVED] OQ-418-003 - Does radio 0x19 clear +0x418? -> Yes, but only if the byte is already nonzero.` (evidence: `0x006F4BA6`)
- `[RESOLVED] OQ-418-004 - Does the write happen before propagation? -> Yes; the store precedes the vtable `+0x278` call.` (evidence: `0x006F4B72` then `0x006F4B79`)
- `[RESOLVED] OQ-418-005 - Does FootClass intercept 0x18/0x19 before TechnoClass? -> No direct FootClass cases; they fall through to TechnoClass.` (evidence: `0x004D8FB0`)
- `[RESOLVED] OQ-418-006 - Does UnitClass case 0x16 read +0x418? -> Yes, it gates a directed `0x15` cascade.` (evidence: `0x0073774A`, `0x00737776`)
- `[RESOLVED] OQ-418-007 - Does PerCellProcess read +0x418 on pad-arrival fallback? -> Yes, it gates a mission-7/destination-building `0x15` fallback.` (evidence: `0x0073A558`, `0x0073A5C3`)
- `[RESOLVED] OQ-418-008 - Does PerCellProcess use +0x418 after unload? -> It can; the cleanup branch sends radio `0x08` when `+0x418` remains set and mission/cell gates pass.` (evidence: `0x0073A936..0x0073A93D`)
- `[RESOLVED] OQ-418-009 - Does DockUnload state-4 clear +0x418? -> No direct clear found; it clears `+0x6D1` and queues Harvest.` (evidence: `0x0073D630`, `0x0073E1F6`)
- `[RESOLVED] OQ-418-010 - Does Set_Destination use +0x418 for stock refinery departure? -> No; its `+0x418` branch is aircraft/carryall-gated, not standard refinery.` (evidence: `0x00741B4A`, `TYPECLASS_0XDFC_LEAVE_DOCK_GATE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-418-011 - Is there an INI key for +0x418? -> No dock-specific INI reader found; it is runtime radio state.` (evidence: constructor and radio decompiles; INI grep)
- `[DEFERRED] OQ-418-012 - What is the exact semantic name across all non-dock consumers?` (category: out-of-scope; reason: this slot is bounded to docking/NavCom; next-step-if-pursued: run a global TechnoClass field audit)
- `[DEFERRED] OQ-418-013 - What exact frame after state-4 exit does the cleanup `0x08 -> 0x19/0x03` cascade fire in a live stock replay?` (category: needs-runtime-debugger; reason: static code proves the branch and gates, not exact runtime frame with live contact state; next-step-if-pursued: runtime trace CMIN unloading through first post-exit cell crossing)

## Sources

- Ghidra `decompile_function 006F2B40` - `TechnoClass::Constructor`.
- Ghidra `decompile_function 006F4AB0` plus assembly contexts at `0x006F4B72`, `0x006F4BA6`, `0x006F4C34..0x006F4C41`, `0x006F4C50..0x006F4C6B`.
- Ghidra `decompile_function 004D8FB0` - `FootClass::Receive_Radio`.
- Ghidra `decompile_function 0043C2D0` - `BuildingClass::Receive_Radio`.
- Ghidra `decompile_function 00737430` plus assembly contexts at `0x0073774A`, `0x00737776`.
- Ghidra `decompile_function 00739EC0` plus assembly contexts at `0x0073A558`, `0x0073A5C3`, `0x0073A936`.
- Ghidra `decompile_function 00741970` plus assembly contexts at `0x00741B4A`, `0x00741B97`.
- Ghidra `decompile_function 0073D630` plus assembly context at `0x0073E1F6`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_00739EC0_NAVCOM_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_PERCELLPROCESS_PAD_ARRIVAL_FIELD_WRITES_GHIDRA_REPORT.md`.
- `ini/rulesmd.ini`, `ini/artmd.ini`.
