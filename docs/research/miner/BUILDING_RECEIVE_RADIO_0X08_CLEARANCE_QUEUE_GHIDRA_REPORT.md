# BuildingClass::Receive_Radio 0x08 Clearance / Queue - Ghidra Research Report

**Address(es):** `0x0043C2D0` primary, `0x006F4AB0`, `0x0065A820`, `0x0065A970`, `0x0065ADF0`, `0x0065AD90`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `BuildingClass::Receive_Radio` case `0x08` for refinery relevance, `0x0B` emission/non-emission from that path, `0x17` queue reply conditions, and minimal contact-slot helper behavior needed for stock refinery docking.  
**Non-Scope:** full BuildingClass radio switch, factory/repair/bunker state machines beyond the `0x08` branch guard, full `Mission_Harvest`, full `Mission_Enter`, and all non-refinery radio cases.  
**Confidence:** High  
**Active in YR:** Conditional. The case is live in the BuildingClass receiver, but stock GAREFN/NAREFN do not use its queue return path because they lack the flags tested by case `0x08`.

## 1. Overview

`BuildingClass::Receive_Radio(0x08)` is not the stock refinery dock queue path. For stock `[GAREFN]` and `[NAREFN]`, the case calls the TechnoClass base `0x08` cleanup behavior and then returns `ROGER(1)` because the refinery's active flags are `DockUnload=yes` / `Refinery=yes`, not `WeaponsFactory=yes`, `UnitRepair=yes`, or `Bunker=yes`.

The player-visible busy-refinery behavior is instead controlled by HELLO/contact capacity and repeated `CAN_DOCK(0x0E)` admission, not by a `QUEUED(0x17)` reply from case `0x08`.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `+0xE4` | RadioClass/TechnoClass | Contacts data pointer | `RadioClass::Receive_Radio @ 0x0065A820`, `FindFreeContactSlot @ 0x0065ADF0` | Yes |
| `+0xE8` | RadioClass/TechnoClass | Contacts capacity | `0x0065ADF0`, `0x0065AE60` | Yes |
| `+0x16A9` | BuildingTypeClass | `UnitRepair=yes`; case `0x08` near-distance and queue flag | `0x0043C2D0`; docs map to UnitRepair; `rulesmd.ini` service depots | Conditional; not GAREFN/NAREFN |
| `+0x16AB` | BuildingTypeClass | `Bunker=yes`; case `0x08` near-distance and queue flag | `0x0043C2D0`; docs map to Bunker | Conditional; not GAREFN/NAREFN |
| `+0x16B3` | BuildingTypeClass | `DockUnload=yes`; stock refinery unload branch, not read by case `0x08` | `rulesmd.ini:11726`, `12519`; `0x0043C2D0` case `0x0E`/`0x15` | Yes |
| `+0x16BB` | BuildingTypeClass | `Refinery=yes`; stock refinery type flag, not read by case `0x08` | `rulesmd.ini:11727`, `12520`; prior flag correction docs | Yes |
| `+0x16BD` | BuildingTypeClass | `WeaponsFactory=yes`; case `0x08` returns `0x17` when set | `0x0043C2D0`; `rulesmd.ini` factory entries | Conditional; not GAREFN/NAREFN |
| `+0x418` | TechnoClass/ObjectClass view | dock/contact-entered flag toggled by radio `0x18`/`0x19`; base `0x08` sends `0x19` then `0x03` | `TechnoClass::Receive_Radio @ 0x006F4AB0` | Conditional; only meaningful if already dock-linked |

## 3. Core Logic

### 3.1 Case `0x08` branch order

Pseudocode from `BuildingClass::Receive_Radio @ 0x0043C2D0`:

```text
if Type.UnitRepair or Type.Bunker:
    distance = |sender.coords - building.coords|
    if distance < 0x180:
        return ROGER

TechnoClass::Receive_Radio(this, sender, 0x08, payload)

if not Type.WeaponsFactory and not Type.UnitRepair and not Type.Bunker:
    return ROGER

return QUEUED(0x17)
```

Active in YR: Conditional. `WeaponsFactory`, `UnitRepair`, and `Bunker` are live YR concepts, but stock ore refineries do not set any of those flags. Evidence: `0x0043C2D0`; `rulesmd.ini:11722-11729`, `12515-12521`.

Tiny details:

- The near-distance shortcut only runs for `UnitRepair`/`Bunker`, not for `DockUnload`/`Refinery`.
- The distance constant is `0x180` leptons, i.e. 384 leptons or 1.5 cells, not the 3-cell wording in some older docs.
- The TechnoClass base handler is called before the final `ROGER`/`0x17` flag test unless the near-distance shortcut returns early.
- `DockUnload=yes` and `Refinery=yes` are not tested in case `0x08`.

### 3.2 TechnoClass base side effect

`TechnoClass::Receive_Radio @ 0x006F4AB0` handles message `0x08` by sending message `0x19` to the sender, then message `0x03` to the sender, and returning the `0x03` result.

Active in YR: Conditional. The handler is live, but its visible effect depends on an existing contact/dock flag state. Evidence: `0x006F4AB0`, case `0x08`; `0x18` and `0x19` cases toggle `+0x418`.

For a stock refinery receiving `0x08`, this base cleanup happens, then BuildingClass returns `1` because the stock refinery is not a factory, repair depot, or bunker.

### 3.3 `0x17 QUEUED`

Case `0x08` returns `0x17` only after the base cleanup call and only when at least one of these receiver type flags is set:

- `WeaponsFactory=yes` (`+0x16BD`)
- `UnitRepair=yes` (`+0x16A9`)
- `Bunker=yes` (`+0x16AB`)

Active in YR: Yes for those systems; No for stock GAREFN/NAREFN. Evidence: `0x0043C2D0`; stock refinery INI only sets `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.

### 3.4 `0x0B DOCK_APPROACH`

Case `0x08` does not transmit `0x0B`. The only `0x0B` code inside `BuildingClass::Receive_Radio @ 0x0043C2D0` is the receiver-side case `0x0B`, which queues mission `0x14` on the building itself and then falls through to the shared `0x0C` tail.

Active in YR: Conditional. The receiver case is live if some sender sends `0x0B`, but this slice found no `0x0B` emission from `0x08` and no stock harvester-refinery need for it. Evidence: `0x0043C2D0`; stock harvester path decompiles use `0x02` HELLO and `0x0E` CAN_DOCK.

### 3.5 Contact-slot full behavior and eviction

There are two different contact paths:

1. `RadioClass::Transmit_Radio_Impl @ 0x0065A970` handles outgoing `HELLO(0x02)`. If the sender's own contact array is full and the target is not already present, it sends `BREAK(0x03)` to `Contacts[0]`, then tries the new target.
2. `RadioClass::Receive_Radio @ 0x0065A820` handles incoming `HELLO(0x02)`. If the receiver's contact array has no free slot, it returns `NEGATORY(10)` and does not evict `Contacts[0]`.

Active in YR: Yes. Stock `Mission_Harvest` sends `HELLO(0x02)` to the refinery in state 2, and the stock refinery contact array is sized by `NumberOfDocks=1`. Evidence: `0x0073E5E0`, `0x0065A970`, `0x0065A820`, `rulesmd.ini:11729`, `12521`.

Consequence: a busy stock refinery does not evict its current harvester from its own Contacts[] merely because another harvester asks for access. The older broad "full slot evicts Contacts[0]" rule applies to the sender-side outgoing helper, not to the receiver-side full-refinery HELLO path.

## 4. INI Keys

| INI path | Value | Use in this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[GAREFN] DockUnload` | `yes` at line 11726 | Stock refinery DockUnload branch; not read by case `0x08` | Yes |
| `rulesmd.ini:[GAREFN] Refinery` | `yes` at line 11727 | Stock refinery role; not read by case `0x08` | Yes |
| `rulesmd.ini:[GAREFN] NumberOfDocks` | `1` at line 11729 | Contacts capacity for standard Allied refinery | Yes |
| `rulesmd.ini:[NAREFN] DockUnload` | `yes` at line 12519 | Stock refinery DockUnload branch; not read by case `0x08` | Yes |
| `rulesmd.ini:[NAREFN] Refinery` | `yes` at line 12520 | Stock refinery role; not read by case `0x08` | Yes |
| `rulesmd.ini:[NAREFN] NumberOfDocks` | `1` at line 12521 | Contacts capacity for standard Soviet refinery | Yes |
| `WeaponsFactory=yes` | factory entries | Enables `0x17` reply in case `0x08` | Conditional; not refineries |
| `UnitRepair=yes` | service depots | Enables near-distance shortcut and `0x17` reply in case `0x08` | Conditional; not refineries |
| `Bunker=yes` | bunker types | Enables near-distance shortcut and `0x17` reply in case `0x08` | Conditional; not refineries |

## 5. Integration Points

| Function | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | Receiver switch; case `0x08`, `0x0B`, `0x0E`, `0x15` | decompiled | Yes |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | Base `0x08` sends `0x19` then `0x03`; `0x18`/`0x19` toggle dock flag | decompiled | Conditional |
| `RadioClass::Receive_Radio @ 0x0065A820` | Incoming HELLO full receiver returns `10`, no eviction | decompiled | Yes |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | Outgoing HELLO may evict sender's old `Contacts[0]` before target receive | decompiled | Yes |
| `FindFreeContactSlot @ 0x0065ADF0` | Scans receiver contacts and returns true for null slot or matching target | decompiled | Yes |
| `RadioClass::FindDockSlot @ 0x0065AD90` | Scans contacts for exact target and returns slot index or `-1` | decompiled | Yes |
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | Stock harvester state 2 uses `HELLO(0x02)`, not `0x08` | decompiled relevant call site | Yes |
| `FootClass::Mission_Enter @ 0x004D9290` | Per-tick admission uses `CAN_DOCK(0x0E)`, not `0x08` | decompiled relevant call site | Yes |
| `TechnoClass::Set_Destination @ 0x00741970` | Real UnitClass vtable+0x480 dock sender; uses `0x02`/`0x0E` in the stock building-destination path | decompiled relevant calls | Yes |

## 6. Current Rust Implementation Status

Rust now has a dedicated `RefineryDockContacts` structure in `src/sim/miner/miner_dock.rs`. It models accepted contacts and a deterministic retry queue; `hello_or_wait` rejects when contact count is at capacity and queues the miner rather than evicting the current contact.

That matches the receiver-side stock-refinery HELLO finding from this report. The Rust queue is still an explicit internal ordering aid, because the binary's visible stock refinery path is not a `0x17` return from case `0x08`; it is repeated admission attempts plus contact capacity.

Relevant implementation references:

- `src/sim/miner/miner_dock.rs:38` `hello_or_wait`
- `src/sim/miner/miner_dock.rs:52` capacity-full wait path
- `src/sim/miner/miner_dock_sequence.rs:584` dock sequence admission call
- `src/sim/miner/miner_dock_sequence.rs:603` sets `dock_queued=true`
- `src/sim/production/production_types.rs:206` stores `RefineryDockContacts`

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Building case `0x08` branch order | verified | `0x0043C2D0` | none |
| Stock refinery flag exclusion from `0x17` | verified | `0x0043C2D0`; `rulesmd.ini:11726-11729`, `12519-12521` | none |
| Case `0x08` does not send `0x0B` | verified | `0x0043C2D0` decompile | none |
| Receiver-side case `0x0B` self-queues building mission `0x14` | verified | `0x0043C2D0` case `0x0B` | exact non-refinery senders out of scope |
| TechnoClass base `0x08` cleanup | verified | `0x006F4AB0` | none |
| Incoming HELLO full-refinery behavior | verified | `0x0065A820` | none |
| Outgoing HELLO sender-side eviction | verified | `0x0065A970` | none |
| `FindFreeContactSlot` semantics | verified | `0x0065ADF0` | none |
| `FindDockSlot` semantics | verified | `0x0065AD90` | none |
| Stock harvester sender path avoids `0x08` | verified for sampled stock path | `0x0073E5E0`, `0x004D9290`, `0x00741970` | full global constant-xref sweep deferred |
| Factory/repair/bunker use of `0x08` | touched-not-exhausted | `0x0043C2D0` flag branches | out of scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Does case 0x08 return QUEUED for stock GAREFN/NAREFN? -> No. It returns 1 after base cleanup because stock refineries lack +0x16BD/+0x16A9/+0x16AB.` (evidence: `0x0043C2D0`; `rulesmd.ini:11726-11729`, `12519-12521`)
- `[RESOLVED] OQ-002 - Does case 0x08 emit radio 0x0B? -> No. No `0x0B` transmit appears in the case; `0x0B` is only a receiver case in this function.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-003 - What does case 0x0B do when received by a building? -> It queues mission 0x14 on the receiver building and then takes the shared TechnoClass tail.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-004 - Does receiver-side full Contacts[] evict slot 0? -> No. Incoming HELLO scans for a null slot and returns 10 when full.` (evidence: `0x0065A820`)
- `[RESOLVED] OQ-005 - Where does the documented slot-0 eviction happen? -> In outgoing `Transmit_Radio_Impl(HELLO)` on the sender's own Contacts[] before receiver dispatch.` (evidence: `0x0065A970`)
- `[RESOLVED] OQ-006 - Does stock Mission_Harvest use 0x08 to reserve the refinery? -> No. State 2 sends HELLO(0x02) when close enough.` (evidence: `0x0073E5E0`)
- `[RESOLVED] OQ-007 - Does per-tick Mission_Enter use 0x08 while approaching? -> No. The visible retry sends CAN_DOCK(0x0E).` (evidence: `0x004D9290`)
- `[RESOLVED] OQ-008 - What does FindFreeContactSlot accept? -> Any null slot or a slot already equal to the queried target; false only when capacity is exhausted by different contacts.` (evidence: `0x0065ADF0`)
- `[RESOLVED] OQ-009 - What does FindDockSlot return for null target? -> It skips the scan and returns -1.` (evidence: `0x0065AD90`)
- `[DEFERRED] OQ-010 - Which non-refinery systems send 0x0B?` (category: `out-of-scope`; reason: this slot is limited to stock refinery relevance; next-step-if-pursued: trace factory/repair building mission senders listed in `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-011 - Is there any obscure scenario/map script that sends 0x08 to a stock refinery?` (category: `requires-different-system-context`; reason: the stock CMIN/HARV path sampled here does not, but global command/script senders were not the target; next-step-if-pursued: run a constant-xref sweep for radio `0x08` send sites)

## Sources

- Ghidra `decompile_function 0x0043C2D0` - `BuildingClass::Receive_Radio`
- Ghidra `decompile_function 0x006F4AB0` - `TechnoClass::Receive_Radio`
- Ghidra `decompile_function 0x0065A820` - `RadioClass::Receive_Radio`
- Ghidra `decompile_function 0x0065A970` - `RadioClass::Transmit_Radio_Impl`
- Ghidra `decompile_function 0x0065ADF0` - free/matching contact-slot probe
- Ghidra `decompile_function 0x0065AD90` - `RadioClass::FindDockSlot`
- Ghidra `decompile_function 0x0073E5E0` - `UnitClass::Mission_Harvest`
- Ghidra `decompile_function 0x004D9290` - `FootClass::Mission_Enter`
- Ghidra `decompile_function 0x00741970` - `TechnoClass::Set_Destination` / UnitClass vtable+0x480 path
- `docs/plans/2026-05-20-radio-link-refinery-dock-investigation-plan.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `src/sim/miner/miner_dock.rs`
- `src/sim/miner/miner_dock_sequence.rs`
