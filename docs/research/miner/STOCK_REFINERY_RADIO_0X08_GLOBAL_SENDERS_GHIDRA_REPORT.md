# Stock Refinery Radio `0x08` Global Senders - Ghidra Research Report

**Address(es):** sender sites `0x0051A80C`, `0x00522AA2`, `0x0073A93D`, `0x00746142`; receiver `0x0043C2D0`; radio impl `0x0065A970`, `0x0065AD30`, `0x0065AE30`, `0x006F4AB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** bounded constant-xref/caller sweep for literal radio message `0x08` senders through vtable radio slots `+0x274` / `+0x278` and direct receive slot `+0x194`, limited to senders that could reach stock `GAREFN` / `NAREFN` in standard YR or common order/script paths.  
**Non-Scope:** full `BuildingClass::Receive_Radio`, full factory/repair/bunker state machines, every non-literal computed radio message, and runtime replay timing of rare cleanup branches.  
**Confidence:** High for literal sender inventory and stock refinery queue-path exclusion; Medium for semantic names of two boundary-missing helper functions at `0x00522AA2` and `0x00746142`.  
**Active in YR:** Conditional. One live UnitClass cleanup sender can send `0x08` to a stock refinery contact, but stock refineries do not take the `0x17` hidden queue branch.

## Target Question

Does any obscure map/script/control path send radio message `0x08` to stock `GAREFN` / `NAREFN` refineries in a way that activates the `BuildingClass::Receive_Radio(0x08)` queue path, or is `0x08` effectively non-refinery/factory/repair/bunker-only?

Answer: a stock refinery can receive `0x08` from the UnitClass per-cell cleanup path when it is still the sender's radio contact, but this is contact cleanup/break handling. It does not activate the stock refinery queue path because `GAREFN` / `NAREFN` lack `WeaponsFactory=yes`, `UnitRepair=yes`, and `Bunker=yes`, so receiver case `0x08` cannot return `0x17` for stock refineries.

## Non-Goals

- No Rust changes.
- No full re-read of all `BuildingClass::Receive_Radio` cases.
- No investigation of non-stock or modded buildings that combine `Refinery=yes` with `WeaponsFactory=yes`, `UnitRepair=yes`, or `Bunker=yes`.
- No runtime frame trace of exactly when post-unload cleanup fires.

## Evidence Needed To Mark COMPLETE

- Inventory literal `0x08` radio send sites through the radio transmit slots.
- For each literal send site, classify receiver selection and stock refinery reachability.
- Re-check receiver type gates in `BuildingClass::Receive_Radio(0x08)`.
- Check stock `GAREFN` / `NAREFN` INI flags.
- Separate a refinery receiving `0x08` from the hidden queue branch returning `0x17`.

## Stop Conditions

- Stop once all literal `0x08` sender sites through `+0x274` / `+0x278` / `+0x194` have been classified for stock refinery reachability.
- Stop if remaining questions require runtime debugger timing rather than static sender/receiver reachability.
- Stop before broad factory/repair/bunker behavior, because those are receiver-positive controls, not stock refinery paths.

## 1. Sweep Method

Binary byte-pattern sweep of `gamemd.exe` `.text` found:

| Radio entry style | Pattern swept | Hits | Literal `0x08` send sites | Active in YR |
|---|---:|---:|---:|---|
| vtable transmit slot `+0x278` | indirect `CALL [reg+0x278]` | 86 | 0 | Yes; none send `0x08` literally |
| vtable wrapper/default-contact slot `+0x274` | indirect `CALL [reg+0x274]` | 87 | 4 | Yes / Conditional |
| direct receive slot `+0x194` | indirect `CALL [reg+0x194]` | 2 | 0 | Yes; both are inside `RadioClass::Transmit_Radio_Impl` dispatch |

Active in YR: Yes. These are live radio protocol dispatch slots. Evidence: binary pattern scan; Ghidra assembly contexts at all four literal `0x08` sites; `RadioClass::Transmit_Radio_Impl @ 0x0065A970`.

Important limitation: this proves literal sender inventory for these radio slots. It does not prove no computed variable can ever equal `8`, but no material stock-refinery computed path was found in the prior stock harvester/enter reads, and no direct receive literal sender exists outside the radio impl.

## 2. Literal `0x08` Sender Inventory

| Site | Containing function / context | Receiver selection | Stock `GAREFN/NAREFN` reachability | Active in YR |
|---|---|---|---|---|
| `0x0051A80C` | `InfantryClass::PerCellProcess @ 0x00519630`; branch `if this+0x418 != 0` | default radio contact via `vtable+0x274(8)` | No standard stock refinery path found. Infantry can have building contacts in enter/garrison/sabotage paths, but stock refineries are not infantry dock/queue receivers. | Conditional; live for infantry contact cleanup |
| `0x00522AA2` | boundary-missing helper in BuildingClass address range; pre-gated by `HasAnyContact @ 0x0065AE30`, type bytes `+0xD6A/+0xD94`, and control id `0x117B` | default radio contact via `vtable+0x274(8)` | No standard stock refinery reachability established. Preconditions are not the stock CMIN/HARV refinery path; function boundary/name remain uncertain. | Conditional; literal send exists |
| `0x0073A93D` | `UnitClass::PerCellProcess @ 0x00739EC0`; `this+0x418` cleanup branch | default radio contact via `vtable+0x274(8)` | Yes, conditionally. A stock harvester can still have a stock refinery as radio contact after the accepted dock sequence. This is cleanup, not admission. | Yes / Conditional |
| `0x00746142` | boundary-missing UnitClass-range helper; near-identical to `0x00522AA2`, pre-gated by `HasAnyContact @ 0x0065AE30`, type bytes `+0xD6A/+0xD94`, and control id `0x117B` | default radio contact via `vtable+0x274(8)` | No stock CMIN/HARV reachability. The gates are not `Harvester=yes` (`+0xE0E`) and not the stock refinery dock FSM. | Conditional; literal send exists |

### Material site: `0x0073A93D`

`UnitClass::PerCellProcess @ 0x00739EC0` sends `0x08` only after a set of cleanup gates:

- `this+0x418 != 0`.
- Current mission is not the accepted mission-7 destination case.
- Current mission is not `0x10` unload/deploy-building.
- Either no building is under the current cell, or the branch marks the contact state as eligible for cleanup.
- It calls `vtable+0x274(8)`, so the receiver is the sender's default/current radio contact.

Active in YR: Yes / Conditional. The function is live for stock harvesters. The branch can reach a stock refinery only if the refinery remains the unit's radio contact when this cleanup executes. Evidence: decompile `0x00739EC0`; assembly context `0x0073A936..0x0073A943`; prior `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.

This is the one important correction to a too-broad negative claim: stock refineries are not globally immune to receiving `0x08`. They can receive it as a cleanup/break message. That does not make case `0x08` the stock refinery queue path.

## 3. Receiver Gate Recheck

`BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x08`:

```text
if Type.UnitRepair or Type.Bunker:
    if distance(sender, building) < 0x180:
        return ROGER

TechnoClass::Receive_Radio(this, sender, 0x08, payload)

if not Type.WeaponsFactory and not Type.UnitRepair and not Type.Bunker:
    return ROGER

return QUEUED(0x17)
```

Active in YR: Conditional. This case is live, but the `0x17` reply is gated to `WeaponsFactory`, `UnitRepair`, or `Bunker`. Evidence: `0x0043C2D0`; prior report `miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`.

Stock YR:

| Building | Stock flags | Missing `0x08` queue flags | Result if it receives `0x08` | Active in YR |
|---|---|---|---|---|
| `GAREFN` | `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` | `WeaponsFactory`, `UnitRepair`, `Bunker` | Base Techno cleanup then `ROGER(1)`, not `0x17` | Yes |
| `NAREFN` | `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` | `WeaponsFactory`, `UnitRepair`, `Bunker` | Base Techno cleanup then `ROGER(1)`, not `0x17` | Yes |

Evidence: `rulesmd.ini:[GAREFN]` lines 11722, 11726, 11727, 11729; `rulesmd.ini:[NAREFN]` lines 12515, 12519, 12520, 12521.

## 4. Protocol Helpers Checked

| Helper | Behavior in this slice | Evidence | Active in YR |
|---|---|---|---|
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | For non-`0x02`/`0x03` messages, forwards to target `Receive_Radio`. For `0x02`, does contact insertion; for `0x03`, removes contact before forwarding. | decompile `0x0065A970` | Yes |
| slot `+0x274` default-contact sender | One-argument send. Literal sites use it as `vtable+0x274(8)`, i.e. send to default/current contact. | call contexts at four sender sites; `RadioClass` helpers | Yes |
| `ContactAtIndex @ 0x0065AD30` | Returns `Contacts[index]` from `this+0xE4`. | assembly `0x0065AD30..0x0065AD3D` | Yes |
| `HasAnyContact @ 0x0065AE30` | Scans `Contacts[]`; returns true if any slot is non-null. | assembly `0x0065AE30..0x0065AE54` | Yes |
| `TechnoClass::Receive_Radio(0x08) @ 0x006F4AB0` | Sends `0x19`, then `0x03`, to the sender/contact. | decompile/contexts `0x006F4C34..0x006F4C41` | Conditional; cleanup only |

## 5. Conclusions

1. There is no literal `0x08` sender through the directed transmit slot `+0x278`.
   Active in YR: Yes. Evidence: 86-site call-pattern sweep, zero literal `0x08` pushes.

2. There are exactly four literal `0x08` senders through the default-contact slot `+0x274`.
   Active in YR: Conditional. Evidence: assembly contexts at `0x0051A80C`, `0x00522AA2`, `0x0073A93D`, `0x00746142`.

3. The only stock-refinery-reachable sender found is `UnitClass::PerCellProcess @ 0x0073A93D`, and it is a cleanup/break path from an existing contact.
   Active in YR: Yes / Conditional. Evidence: decompile `0x00739EC0`; prior `+0x418` lifecycle report.

4. Stock `GAREFN/NAREFN` receiving `0x08` does not imply queue admission. The stock receiver returns `ROGER(1)` because it lacks `WeaponsFactory`, `UnitRepair`, and `Bunker`.
   Active in YR: Yes. Evidence: `0x0043C2D0`; `rulesmd.ini` stock refinery flags.

5. The hidden `0x17` reply after `0x08` remains factory/repair/bunker-only for stock YR content. A modded refinery with one of those flags would be a different, conditional case.
   Active in YR: Yes for those systems; No for stock `GAREFN/NAREFN`. Evidence: `0x0043C2D0`.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `+0x278` directed transmit literal sweep | verified | 86 call-pattern contexts; no `PUSH 0x8` | none |
| `+0x274` default-contact literal sweep | verified | 87 call-pattern contexts; four `PUSH 0x8` | none for literal sites |
| direct `+0x194` receive literal sweep | verified | two calls, both internal variable dispatch in `0x0065A970` | none |
| `0x0051A80C` infantry sender | verified for send and non-stock classification | `0x00519630`; context `0x0051A806..0x0051A812` | exact infantry mod edge cases out of scope |
| `0x00522AA2` boundary-missing sender | touched-not-exhausted | context `0x00522A91..0x00522AA8` | exact function name; not needed for stock refinery conclusion |
| `0x0073A93D` UnitClass cleanup sender | verified | `0x00739EC0`; context `0x0073A936..0x0073A943` | runtime frame timing deferred |
| `0x00746142` boundary-missing sender | touched-not-exhausted | context `0x00746131..0x00746148` | exact function name; not needed for stock refinery conclusion |
| Stock receiver `0x08 -> 0x17` gate | verified | `0x0043C2D0`; `rulesmd.ini` | none |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-011-001 - Are there literal directed `+0x278` sends of `0x08`? -> No.` (evidence: 86-site pattern sweep; Ghidra contexts)
- `[RESOLVED] OQ-011-002 - Are there literal default-contact `+0x274` sends of `0x08`? -> Yes, four sites: `0x0051A80C`, `0x00522AA2`, `0x0073A93D`, `0x00746142`.` (evidence: Ghidra contexts)
- `[RESOLVED] OQ-011-003 - Can any verified sender reach stock `GAREFN/NAREFN`? -> Yes, conditionally: UnitClass per-cell cleanup can send to the unit's current/default contact, which can be the stock refinery after docking contact setup.` (evidence: `0x00739EC0`, `0x0073A93D`)
- `[RESOLVED] OQ-011-004 - Does a stock refinery receiving `0x08` return hidden queue `0x17`? -> No. `0x17` requires `WeaponsFactory`, `UnitRepair`, or `Bunker`; stock `GAREFN/NAREFN` do not set these.` (evidence: `0x0043C2D0`; `rulesmd.ini`)
- `[RESOLVED] OQ-011-005 - Is `0x08` a stock refinery admission/queue mechanism? -> No. Stock queue/admission remains HELLO/CAN_DOCK/contact capacity; `0x08` is cleanup/break handling when it reaches a refinery.` (evidence: `0x006F4AB0`; `0x00739EC0`; prior OQ-011 report)
- `[DEFERRED] OQ-011-006 - Exact function names for `0x00522AA2` and `0x00746142`.` (category: `requires-different-system-context`; reason: Ghidra has missing/unclear boundaries, and pre-call gates are enough to exclude stock CMIN/HARV refinery queue reachability; next-step-if-pursued: boundary repair or surrounding class audit)
- `[DEFERRED] OQ-011-007 - Exact frame when stock harvester cleanup `0x08` fires after unload.` (category: `needs-runtime-debugger`; reason: static code proves branch and gates, not the exact replay frame; next-step-if-pursued: runtime trace from accepted dock through first post-unload cell process)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock refinery can receive `0x08` only as cleanup; it must not use that as queue admission or return `0x17`. | `0x0043C2D0`, `0x0073A93D`, `rulesmd.ini` | likely matches queue model; exact cleanup path unchecked | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs` | Keep queue admission on HELLO/CAN_DOCK/contact capacity, not `0x08`. | `test_stock_refinery_does_not_use_radio_0x08_queue_path` | Do not implement a refinery wait queue by simulating `Receive_Radio(0x08) -> 0x17` for `GAREFN/NAREFN`. |
| A lingering contact-entered (`+0x418`-like) state can trigger `0x08 -> 0x19 -> 0x03` cleanup after non-mission-7/non-mission-0x10 gates. | `0x00739EC0`; `0x006F4AB0` | unchecked for post-unload cleanup timing | `RefineryDockContacts::clear_contact_entered`, `release_contact`, departure code in `miner_dock_sequence.rs` | Ensure cleanup/release clears contact-entered and contact state without treating miner as queued. | unload completion with lingering contact releases contact and next miner may retry by normal contact path | Do not call ReleaseDockedHarvester/Force_Track(0x47) for stock zero-link refinery cleanup. |
| Non-refinery positive `0x17` behavior belongs to factory/repair/bunker receivers, not stock refineries. | `0x0043C2D0`; INI flags | out of current miner scope | future factory/repair/bunker systems | Preserve separate receiver gates if those systems are implemented. | factory/repair/bunker-specific radio tests, not miner tests | Do not generalize stock refinery behavior from factory/repair/bunker `0x08` responses. |

## Negative Facts / Do Not Do

- Do not treat `0x08` as the stock refinery hidden queue path.
- Do not return `0x17` from stock `GAREFN/NAREFN` on `0x08`.
- Do not model a second stock refinery queue sourced from `0x08`; visible waiting is still admission/contact retry.
- Do not say "stock refineries never receive `0x08`"; the precise statement is that receiving it is cleanup-only and non-queue.
- Do not use `ReleaseDockedHarvester` / `Force_Track(0x47)` as a consequence of the stock zero-link cleanup path.

## Remaining Uncertainty

- Exact names and full semantics of the two boundary-missing literal senders at `0x00522AA2` and `0x00746142`.
- Exact runtime frame/tick when `UnitClass::PerCellProcess` cleanup `0x08` fires after a stock unload, if it fires in a given replay.
- Modded/non-stock buildings that combine refinery behavior with factory/repair/bunker flags were not classified.

## Stale Docs / Follow-up Wording

Recommended replacement wording for older docking/unload docs:

- Replace "stock refineries do not use radio `0x08`" with: "stock refinery queue/admission does not use radio `0x08`; however a stock refinery can receive `0x08` from UnitClass contact cleanup, where it performs base cleanup and returns `ROGER`, not `0x17`."
- Replace "radio `0x08` is the hidden refinery queue path" with: "radio `0x08 -> 0x17` is factory/repair/bunker-gated. Stock `GAREFN/NAREFN` lack those flags, so their busy-dock behavior remains HELLO/CAN_DOCK/contact-capacity retry."
- Replace "no global sender can reach a stock refinery" with: "the only stock-reachable literal sender found is UnitClass per-cell cleanup at `0x0073A93D`; it is a break/cleanup path, not a queue path."

## Sources

- Ghidra `decompile_function 00519630` - `InfantryClass::PerCellProcess`.
- Ghidra `decompile_function 00739EC0` - `UnitClass::PerCellProcess`.
- Ghidra `decompile_function 0065A970` - `RadioClass::Transmit_Radio_Impl`.
- Ghidra assembly contexts for all `+0x274` and `+0x278` call-pattern sites; literal `0x08` sites at `0x0051A80C`, `0x00522AA2`, `0x0073A93D`, `0x00746142`.
- Ghidra assembly contexts `0x0065AD30..0x0065AD3D` and `0x0065AE30..0x0065AE54`.
- Prior report: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`.
- Prior report: `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.
- `ini/rulesmd.ini`.
- Rust scan: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`.
