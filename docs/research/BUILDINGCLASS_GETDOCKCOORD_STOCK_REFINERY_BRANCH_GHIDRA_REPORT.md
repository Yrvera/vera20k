# BuildingClass GetDockCoord Stock Refinery Branch - Ghidra Research Report

**Address(es):** `0x00447B20` primary; support checks at `0x0045FE50`, `0x0043C2D0`, `0x00739EC0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `BuildingClass::GetDockCoord` branch order, branch predicates, returned coordinates, and the branch taken by standard YR `GAREFN`/`NAREFN` when queried with a requesting dock object.  
**Non-Scope:** full refinery unload FSM, far-return `QueueingCell` staging, cargo credit cadence, aircraft/depot pad behavior beyond distinguishing branch order.  
**Confidence:** High  
**Active in YR:** Yes for the helper and for stock `Refinery=yes` branch; `Weeder=yes` branch is conditional and not active for stock `GAREFN`/`NAREFN`.

## 0. Working Notes Contract

**Target question:** Does `BuildingClass::GetDockCoord @ 0x00447B20` use the `NW+(2,1)` branch for stock YR ore refineries, or is that branch `Weeder`/another path?  
**Non-goals:** Do not implement Rust, do not audit the complete miner dock/unload loop, and do not rewrite older docs in this slot.  
**Evidence needed to mark COMPLETE:** Binary branch predicates and return formulas, binary INI reader mapping for `+0x16BC/+0x16BB/+0x16B3`, stock INI values for `GAREFN/NAREFN`, and a Rust-facing handoff for the current `refinery_pad_cell` risk.  
**Stop conditions:** Stop after the `GetDockCoord` branch identity is settled and stock-refinery liveness is proven or ruled out; list wider arrival-path questions as deferred.

## 1. Overview

`BuildingClass::GetDockCoord` is a vtable-dispatched docking coordinate helper. Its first branch checks `BuildingTypeClass+0x16BC` and returns a centered cell at building packed/NW cell `+(2,1)`, but `BuildingTypeClass::ReadINI` maps `+0x16BC` from the literal `Weeder=` key, not `Refinery=`.

Standard YR `GAREFN` and `NAREFN` set `DockUnload=yes` and `Refinery=yes`, and do not set `Weeder=yes`. Therefore a standard stock refinery queried through `GetDockCoord` does not take the `NW+(2,1)` branch; it falls through to the next `+0x16BB` `Refinery=yes` branch.

## 2. Key Offsets / Flags

| Offset / slot | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass vtable+0xA8` | `GetDockCoord -> 0x00447B20` | xref from vtable data `0x007E3F64`; `get_function_xrefs 0x00447B20` | Yes, vtable helper |
| `BuildingClass+0x520` | `BuildingTypeClass*` | `0x00447B27 MOV EAX,[ESI+0x520]`; decompile `0x00447B20` | Yes |
| `BuildingTypeClass+0x16BC` | `Weeder=` | `ReadINI 0x0045FE50` decompile reads `s_Weeder_0081AC50` into `+0x16BC`; ctor/default docs show default false | Conditional; no for stock `GAREFN/NAREFN` |
| `BuildingTypeClass+0x16BB` | `Refinery=` | `ReadINI 0x0045FE50` decompile reads `s_Refinery_0081AA5C` into `+0x16BB`; stock rules set yes | Yes for `GAREFN/NAREFN` |
| `BuildingTypeClass+0x16B3` | `DockUnload=` | `ReadINI 0x0045FE50` decompile reads `s_DockUnload_0081AA94` into `+0x16B3`; `Receive_Radio 0x0043C2D0` uses it for stock admission/handoff | Yes for `GAREFN/NAREFN` |
| `BuildingTypeClass+0x16AB` | `Bunker=` | `ReadINI 0x0045FE50`; branch checked after `Refinery=` in `0x00447B20` | Conditional; not stock refinery |
| `BuildingTypeClass+0x16CB` | `Helipad=` | `ReadINI 0x0045FE50`; dock-offset branch in `0x00447B20` | Conditional; not stock refinery |
| `BuildingTypeClass+0x16A9` | `UnitRepair=` | `ReadINI 0x0045FE50`; dock-offset branch in `0x00447B20` | Conditional; not stock refinery |
| `BuildingTypeClass+0x1780/+0x1788` | `NumberOfDocks` / `DockingOffset%d` array | `0x00464938..0x00464A47`; `0x00447D3D..0x00447D7E` | Yes for dock-offset users; bypassed by stock refinery `+0x16BB` branch |

## 3. Core Logic

Branch order in `BuildingClass::GetDockCoord @ 0x00447B20`:

| Order | Predicate | Return behavior | Evidence | Active in YR |
|---:|---|---|---|---|
| 1 | `Type+0x16BC != 0` | Calls this building vtable `+0x1B8` to get packed/NW cell; returns `(x+2)*256+128`, `(y+1)*256+128`, `this+0xA4` Z | decompile `0x00447B20`; assembly `0x00447B27..0x00447B64` | Conditional; `Weeder=yes`, not stock `GAREFN/NAREFN` |
| 2 | `Type+0x16BB != 0` | Calls `FUN_005F6C80` on the requester object and returns requester X `+0x80`, requester Y, requester Z | decompile `0x00447B20`; assembly `0x00447B9E..0x00447BC4` | Yes when stock refinery helper is queried with sender |
| 3 | `Type+0x16AB != 0 && requester != null` | Computes angle from building center to requester and applies one of four `+/-0x80` lepton side offsets to building coords | decompile `0x00447B20`; assembly starts `0x00447BC9..0x00447BDD` | Conditional; bunker path |
| 4 | `Type+0x16CB != 0 || Type+0x16A9 != 0` | Uses `NumberOfDocks`/`DockingOffset%d`; `0` slots returns building coords, `1` slot uses first offset, `>1` uses `RadioClass::FindDockSlot`; invalid slot falls back to building coords | decompile `0x00447B20`; assembly `0x00447CFC..0x00447D7E` | Conditional; helipad/unit-repair path |
| 5 | fallback | Returns object coords through `FUN_005F6C80` / vtable `+0x48` | decompile `0x00447B20`; assembly `0x00447D10..0x00447D37`, `0x00447DCD..0x00447DD6` | Yes for other building types |

Tiny details:

- The `NW+(2,1)` branch is before `Refinery=yes`, so a modded building with both `Weeder=yes` and `Refinery=yes` would take `Weeder=yes` first. Active in YR: Conditional.
- The `Refinery=yes` branch returns before `Bunker`, `Helipad`, `UnitRepair`, `NumberOfDocks`, or `DockingOffset%d` are examined. Active in YR: Yes for stock refinery helper queries.
- The `Refinery=yes` branch has no null guard for the requester before calling `FUN_005F6C80`; live dock callers must supply a requester. Active in YR: Yes under dock callers; null behavior is not a safe implementation contract.
- `NumberOfDocks=1` on stock `GAREFN/NAREFN` does not make `GetDockCoord` read `DockingOffset0`, because the earlier `+0x16BB` branch already returned. Active in YR: Yes.

## 4. INI Keys

| File | Section / key | Stock value | Binary mapping / effect | Active in YR |
|---|---|---|---|---|
| `ini/rulesmd.ini` | `[GAREFN] DockUnload` | `yes` at line `11726` | `+0x16B3`; used by `Receive_Radio` stock admission and `0x15` handoff | Yes |
| `ini/rulesmd.ini` | `[GAREFN] Refinery` | `yes` at line `11727` | `+0x16BB`; the `GetDockCoord` branch stock GAREFN would take | Yes |
| `ini/rulesmd.ini` | `[GAREFN] Weeder` | absent | `+0x16BC` remains default false | No for stock GAREFN |
| `ini/rulesmd.ini` | `[NAREFN] DockUnload` | `yes` at line `12519` | `+0x16B3`; same as GAREFN | Yes |
| `ini/rulesmd.ini` | `[NAREFN] Refinery` | `yes` at line `12520` | `+0x16BB`; the `GetDockCoord` branch stock NAREFN would take | Yes |
| `ini/rulesmd.ini` | `[NAREFN] Weeder` | absent | `+0x16BC` remains default false | No for stock NAREFN |
| `ini/artmd.ini` | `[NAREFN] QueueingCell` | `4,1` at line `1716` | not read by `GetDockCoord`; separate fallback/wait data | Conditional |
| `ini/artmd.ini` | `[NAREFN] ;DockingOffset0` | commented at line `1725` | inactive; even active `DockingOffset0` would be bypassed by `+0x16BB` branch | No for stock NAREFN |
| `ini/artmd.ini` | `[GAREFN] QueueingCell` | `4,1` at line `1773` | not read by `GetDockCoord`; separate fallback/wait data | Conditional |
| `ini/artmd.ini` | `[GAREFN] RemoveOccupy1` | `3,1` at line `1795` | art passability context, not a `GetDockCoord` input | Yes as art data |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::GetDockCoord @ 0x00447B20` | The scoped helper; branch identity settled here | direct decompile plus assembly contexts listed above | Yes |
| `BuildingTypeClass::ReadINI @ 0x0045FE50` | Maps `Weeder`, `DockUnload`, `Refinery`, `NumberOfDocks`, `DockingOffset%d` to fields | direct decompile; assembly contexts `0x00464938..0x00464A47` for dock offsets | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | Stock `CAN_DOCK(0x0E)` accepted target is separate: `DockUnload || Weeder` sends `NW+(3,1)` cell payload | direct decompile; prior reports confirm | Yes for stock DockUnload |
| `UnitClass::PerCellProcess @ 0x00739EC0` | Contains a dock-arrival path that can call destination building vtable `+0xA8`; also has a separate contact-byte/refinery-adjacent path | direct decompile | Yes, but full path split is outside this slot |

## 6. Current Rust Implementation Status

Current Rust scan found:

- `src/sim/miner/miner_dock_sequence.rs::refinery_can_dock_queue_cell` returns `NW+(3,1)`, matching `BuildingClass::Receive_Radio(0x0E)` accepted stock refinery cell.
- `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell` currently falls back to `NW+(2,1)`, with comments saying this is the retail refinery offset. That is not valid for stock `GAREFN/NAREFN`; the binary ties `NW+(2,1)` to `Weeder=yes`.
- `src/sim/miner/miner_dock_sequence.rs::resolve_refinery_cells` returns `(wait_queue, accepted_cell, pad, dock_capacity)` and feeds `pad` into the later linked phase. That surface now needs a stock-refinery-vs-Weeder distinction if the `pad` value is consumed for state or rendering.
- `src/sim/miner/miner_system.rs::refinery_dock_cell` delegates to `refinery_can_dock_queue_cell`, which is correct for accepted stock `CAN_DOCK` cell and should not be "fixed" to `NW+(2,1)`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `GetDockCoord` vtable binding | verified | `get_function_xrefs 0x00447B20` -> `0x007E3F64` | none |
| `GetDockCoord` `+0x16BC` branch | verified | decompile `0x00447B20`; assembly `0x00447B27..0x00447B64`; `ReadINI` maps `Weeder` | none |
| `GetDockCoord` `+0x16BB` branch | verified | decompile `0x00447B20`; assembly `0x00447B9E..0x00447BC4`; `ReadINI` maps `Refinery` | none |
| `GetDockCoord` dock-offset branch | verified for branch order | decompile `0x00447B20`; assembly `0x00447CFC..0x00447D7E` | exact airfield/depot output not re-audited here |
| Stock `GAREFN/NAREFN` flag values | verified | `rulesmd.ini:11726-11729`, `12519-12521`; `Weeder` absent | none |
| Accepted stock miner target `NW+(3,1)` | verified as supporting fact | `BuildingClass::Receive_Radio @ 0x0043C2D0`; prior `CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md` | none for this slot |
| `UnitClass::PerCellProcess` arrival split | touched-not-exhausted | decompile `0x00739EC0` | full reconciliation of `+0xA8` branch vs `+0x418` adjacent-refinery branch belongs to another slot |
| Current Rust `refinery_pad_cell` | verified scan | Codegraph context and source scan | implementer must decide whether to revert to stock pad/accepted-cell or split Weeder support |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - What is the target question? -> Whether `NW+(2,1)` is stock refinery or `Weeder` in `GetDockCoord`.` (evidence: `0x00447B20`, `0x0045FE50`)
- `[RESOLVED] OQ-2 - What are the non-goals? -> Full unload, far queueing, and implementation are out of scope.` (evidence: user slot instructions)
- `[RESOLVED] OQ-3 - Is `0x00447B20` live? -> Yes, it is vtable slot `+0xA8` via data xref `0x007E3F64`.` (evidence: `get_function_xrefs 0x00447B20`)
- `[RESOLVED] OQ-4 - What branch checks first? -> `Type+0x16BC` before all other type flags.` (evidence: `0x00447B27..0x00447B35`)
- `[RESOLVED] OQ-5 - What does branch 1 return? -> building packed/NW cell `+(2,1)` centered to leptons, plus building Z.` (evidence: `0x00447B40..0x00447B64`; decompile `0x00447B20`)
- `[RESOLVED] OQ-6 - What key maps to `+0x16BC`? -> `Weeder=`.` (evidence: `BuildingTypeClass::ReadINI @ 0x0045FE50`; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`)
- `[RESOLVED] OQ-7 - Do stock `GAREFN/NAREFN` set `Weeder=yes`? -> No; key is absent in those sections and default is false.` (evidence: `rulesmd.ini:11722-11729`, `12515-12521`)
- `[RESOLVED] OQ-8 - What branch does stock `GAREFN/NAREFN` take if `GetDockCoord` is queried? -> `+0x16BB` `Refinery=yes` branch.` (evidence: `0x00447B9E..0x00447BC4`; `rulesmd.ini`)
- `[RESOLVED] OQ-9 - Does stock refinery `GetDockCoord` read `DockingOffset0` because `NumberOfDocks=1`? -> No, the `Refinery=yes` branch returns before `NumberOfDocks`.` (evidence: `0x00447B9E..0x00447BC4` precedes `0x00447D3D`)
- `[RESOLVED] OQ-10 - Is accepted stock miner target sourced from `GetDockCoord`? -> No, `Receive_Radio(0x0E)` computes `NW+(3,1)` inline from packed building cell for `DockUnload || Weeder`.` (evidence: `0x0043C2D0` decompile)
- `[RESOLVED] OQ-11 - Do `QueueingCell` or `RemoveOccupy` drive `GetDockCoord`? -> No; neither appears in `0x00447B20`.` (evidence: decompile `0x00447B20`; `artmd.ini` checked as data context)
- `[RESOLVED] OQ-12 - Null requester edge case? -> The `Refinery=yes` branch has no visible null guard before requester `GetCoords`; live dock callers are expected to pass a sender.` (evidence: `0x00447B9E..0x00447BB4`)
- `[DEFERRED] OQ-13 - Which exact `UnitClass::PerCellProcess` sub-branch starts stock refinery unload in all cases?` (category: `requires-different-system-context`; reason: this slot only needed `GetDockCoord` branch identity; next-step-if-pursued: reconcile `+0xA8` equality branch with `+0x418` adjacent-refinery branch)
- `[RESOLVED] OQ-14 - Current Rust delta? -> `refinery_pad_cell` fallback now encodes the `Weeder` branch as stock refinery behavior.` (evidence: Codegraph and source scan)
- `[RESOLVED] OQ-15 - TS legacy filter? -> `Weeder=yes` path is conditional/legacy-style and not stock `GAREFN/NAREFN`; `Refinery=yes`/`DockUnload=yes` are live YR stock fields.` (evidence: `ReadINI 0x0045FE50`; `rulesmd.ini`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `NW+(2,1)` `GetDockCoord` branch is `Weeder=yes`, not stock `Refinery=yes` | `0x00447B20` first predicate `+0x16BC`; `ReadINI 0x0045FE50` maps `Weeder` to `+0x16BC`; `rulesmd.ini` stock refineries omit `Weeder` | mismatch: `refinery_pad_cell` fallback now treats `NW+(2,1)` as retail refinery | `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell`; `src/sim/miner/miner_tests.rs::refinery_pad_and_conditional_release_cells` | Do not use `NW+(2,1)` as stock `GAREFN/NAREFN` miner deposit/unload pad; either revert stock fallback or split a named Weeder-only helper | `stock_refinery_pad_cell_does_not_use_weeder_getdockcoord_branch` | Do not "fix" stock miner docking from `NW+(3,1)` to `NW+(2,1)` based on stale coord-cell row |
| Stock accepted `CAN_DOCK(0x0E)` target remains building packed/NW `+(3,1)` for `DockUnload=yes` | `BuildingClass::Receive_Radio @ 0x0043C2D0`; stock `DockUnload=yes` at `rulesmd.ini:11726`, `12519` | none observed for `refinery_can_dock_queue_cell`; risky nearby `pad` consumer remains | `src/sim/miner/miner_dock_sequence.rs::refinery_can_dock_queue_cell`; `src/sim/miner/miner_system.rs::refinery_dock_cell` | Preserve accepted stock cell as `NW+(3,1)` and keep it distinct from `QueueingCell=4,1` and `GetDockCoord` Weeder branch | `stock_refinery_can_dock_accepted_cell_stays_nw_plus_3_1` | Do not route `refinery_dock_cell` to `refinery_pad_cell` if that would change accepted stock docking |
| Standard `GAREFN/NAREFN` queried through `GetDockCoord` take `Refinery=yes +0x16BB` branch, returning requester coords with X `+0x80`, not `DockingOffset0` | `0x00447B9E..0x00447BC4`; `ReadINI 0x0045FE50`; `rulesmd.ini:11727`, `12520` | unchecked for any Rust surface that models `GetDockCoord` generically | future building dock-coordinate helper, if implemented | Model `GetDockCoord` separately from stock `CAN_DOCK` accepted cell; do not collapse both into one "refinery pad" concept | `getdockcoord_stock_refinery_uses_requester_x_plus_half_cell_not_docking_offset` | Do not use `NumberOfDocks=1` or commented `DockingOffset0` to override stock refinery `GetDockCoord` |

### Negative Facts / Do Not Do

- Do not label `BuildingTypeClass+0x16BC` as stock refinery. It is `Weeder=`.
- Do not claim `GetDockCoord` branch 1 is active for standard `GAREFN/NAREFN`; stock sections do not set `Weeder=yes`.
- Do not change stock `CAN_DOCK` accepted movement from `NW+(3,1)` to `NW+(2,1)`.
- Do not read `QueueingCell=4,1` or `DockingOffset0` as the stock accepted refinery pad.
- Do not fold `GetDockCoord`, `Receive_Radio(0x0E)` accepted cell, `QueueingCell`, and `RemoveOccupy` into one Rust helper; they are separate reference points.

### Stale Docs / Follow-up Docs

- `docs/research/coord-cell-conversions/fn-building-getdockcoord.md`: replace "Refinery pad branch (`BuildingTypeClass+0x16bc != 0`)" with "Weeder branch (`BuildingTypeClass+0x16BC != 0`, parsed from `Weeder=`): returns building NW `+(2,1)`; stock `GAREFN/NAREFN` do not set this flag."
- `docs/research/coord-cell-conversions/fn-building-getdockcoord.md`: replace "Only branch 1 is active for refineries in standard YR" with "For standard YR `GAREFN/NAREFN`, branch 1 is not active; a `GetDockCoord` query reaches the `Refinery=yes` `+0x16BB` branch. The stock accepted miner `CAN_DOCK` cell is computed separately in `BuildingClass::Receive_Radio(0x0E)` as NW `+(3,1)`."
- `docs/research/coord-cell-conversions/_parity.md`: replace row 35's "FIXED" claim with "DRIFT / STALE EVIDENCE: the `NW+(2,1)` change matches the `Weeder=yes` GetDockCoord branch, not standard stock refinery docking. Stock `GAREFN/NAREFN` accepted miner target remains NW `+(3,1)`."
- `docs/research/coord-cell-conversions/_system.md`: remove "Refinery dock pad - NW+3 -> NW+2 (every miner deposit)" from the fix list; replace with "Reconcile stale `GetDockCoord` row: `NW+(2,1)` is Weeder-only; preserve stock miner accepted cell NW `+(3,1)`."

## Sources

- Ghidra decompile: `0x00447B20`, `0x0045FE50`, `0x0043C2D0`, `0x00739EC0`.
- Ghidra assembly contexts: `0x00447B27..0x00447B64`, `0x00447B9E..0x00447BC4`, `0x00447BC9..0x00447BDD`, `0x00447CFC..0x00447D7E`, `0x00464938..0x00464A47`.
- `ini/rulesmd.ini` lines `7351-7364`, `8215-8228`, `11722-11729`, `12515-12521`.
- `ini/artmd.ini` lines `1706-1725`, `1763-1795`.
- Prior docs checked: `coord-cell-conversions/fn-building-getdockcoord.md`, `miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`, `DOCKING_QUEUE_EXIT_REFERENCE_POINTS_GHIDRA_REPORT.md`.
