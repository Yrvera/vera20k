# Accepted Cell / GetDockCoord / QueueingCell Doc-Cluster Audit - Ghidra Report

**Date:** 2026-05-24T20:51+02:00  
**Slot:** re-swarm subagent slot 4  
**Scope:** exact coordinate wording only for the accepted stock refinery cell, stock `GetDockCoord`, `QueueingCell`, stock `RemoveOccupy` art pad opening, `+0x16B3/+0x16BB/+0x16BC` flags, and whether the accepted radio anchor and visible dock/link cell "coincide".  
**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN` docking, stock refinery art, and the referenced radio/per-cell functions.  
**Confidence:** High for coordinate wording; Medium for any phrase that tries to name the player-visible "link" point, because multiple source-specific `0x15` paths exist.

## Working Notes

**Target question:** Do the five listed docs still use exact, non-contradictory wording for stock accepted NW+(3,1), stock `GetDockCoord` NW+(2,1), `QueueingCell` NW+(4,1), stock `RemoveOccupy` pad opening, `+0x16B3/+0x16BB/+0x16BC`, and the "coincide" claim?  
**Non-goals:** Do not re-open far-return staging, post-unload exit, two-miner runtime order, Rust implementation, or non-refinery dock behavior.  
**Evidence needed to mark COMPLETE:** Fresh read-only Ghidra decompile of `BuildingClass::Receive_Radio`, `BuildingClass::GetDockCoord`, `UnitClass::PerCellProcess`, `UnitClass::Receive_Radio(0x16)`, and `FootClass::Mission_Enter`; stock INI/art lines for `DockUnload`, `Refinery`, `QueueingCell`, and `RemoveOccupy`; comparison against the canonical synthesis.  
**Stop conditions:** Stop after stale coordinate wording is identified and exact replacement text is provided per listed doc, with no source/Rust edits.

## 1. Verdict

The canonical model is consistent and should be the wording source: for stock 4x3 `GAREFN/NAREFN`, accepted `Receive_Radio(0x0E)` movement target is `NW+(3,1)`, stock `GetDockCoord` is `NW+(2,1)` through `+0x16BB Refinery=yes`, and art `QueueingCell=4,1` is `NW+(4,1)`.

The only material stale wording in the audited cluster is in the two older accepted-cell reports. They correctly prove accepted NW+(3,1), but some sentences still imply either that `NW+(2,1)` belongs only to the `+0x16BC` branch, or that accepted radio anchor and visible dock/link cell simply "coincide." The corrected wording is: accepted NW+(3,1) coincides with the stock art-opened/passable `RemoveOccupy` pad cell, but it does not coincide with stock `GetDockCoord`/PerCellProcess equality coordinate NW+(2,1).

## 2. Verified Binary Facts

- `BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x0E`, uses `Type+0x16B3 DockUnload` or `Type+0x16BC Weeder` for the accepted stock branch. It calls `Get_Cell_Packed`, forms packed `CONCAT22(y+1, x+3)`, converts that through `MapClass::Get_CellClass`, and sends radio `0x12`. This is accepted `NW+(3,1)`.
- The same accepted `0x0E` payload computation does not call `BuildingClass::GetDockCoord`, does not read `QueueingCell`, and does not read `DockingOffset`/`NumberOfDocks` for the stock accepted movement target.
- `BuildingClass::GetDockCoord @ 0x00447B20` first checks `Type+0x16BC` (Weeder), which returns centered `NW+(2,1)` from packed NW cell. Stock GAREFN/NAREFN are not this branch.
- `BuildingClass::GetDockCoord @ 0x00447B20` then checks `Type+0x16BB` (Refinery). For a 4x3 building, `BuildingClass::GetCoords @ 0x00447AC0` returns `NW + ((W-1)*0x80, (H-1)*0x80)`, and the refinery branch adds `+0x80` X. For W=4/H=3 this converts to cell `NW+(2,1)`.
- `UnitClass::PerCellProcess @ 0x00739EC0` has a `GetDockCoord` equality branch that can send radio `0x15`, and also has a contact-flag/adjacent-building `0x15` branch. The equality branch is a possible source, not the definition of the accepted movement cell.
- `UnitClass::Receive_Radio @ 0x00737430`, case `0x16`, does not call `GetDockCoord` and does not set a new move destination. It can return after setting locomotor/rate state, or later send `0x15` from a stopped unit with a live building destination and mission 7.
- `FootClass::Mission_Enter @ 0x004D9290` sends one `0x0E` per mission dispatch and returns the `[Enter]` timer delay plus `RandomRanged(0,2)`, preserving the staged retry model.

## 3. Verified INI / Art Facts

- `rulesmd.ini:[CMIN]` and `[HARV]` target `Dock=NAREFN,GAREFN` and set `Harvester=yes`.
- `rulesmd.ini:[GAREFN]` has `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`; same for `[NAREFN]`.
- Stock `GAREFN/NAREFN` have no stock `Weeder=yes` key in their sections; `+0x16BC` is not the stock refinery flag.
- `artmd.ini:[GAREFN] QueueingCell=4,1` and `[NAREFN] QueueingCell=4,1` are stock art fallback/waiting data, not the accepted `0x0E` payload.
- `artmd.ini:[GAREFN] RemoveOccupy1=3,1` and `[NAREFN] RemoveOccupy8=3,1` open the accepted `NW+(3,1)` pad/passable cell. This art fact does not make that cell `GetDockCoord`.

## 4. Doc Reconciliation

| Doc | Status | Exact audit result |
|---|---|---|
| `miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md` | YELLOW stale wording | Correct for accepted NW+(3,1), QueueingCell exclusion, and flags. Stale where "visible dock pad/link cell coincide" can be read as `GetDockCoord`/link equality. |
| `BUILDING_RECEIVE_RADIO_0E_STOCK_REFINERY_CANDOCK_CELL_GHIDRA_REPORT.md` | YELLOW stale wording | Correct for accepted NW+(3,1). Stale where it says NW+(2,1) "belongs to" `GetDockCoord +0x16BC`; stock `GetDockCoord` NW+(2,1) is `+0x16BB Refinery=yes`, while `+0x16BC Weeder` is separate. |
| `coord-cell-conversions/fn-building-getdockcoord.md` | GREEN with two wording tightenings | Current summary correctly names `+0x16BB` stock refinery and says no NW+3 -> NW+2 physical bridge. Tighten any "every harvester deposit"/"arrival coordinate" phrase to avoid implying `GetDockCoord` is required for every unload. |
| `coord-cell-conversions/_system.md` | GREEN | Already reconciles accepted NW+(3,1), stock `GetDockCoord` NW+(2,1), QueueingCell NW+(4,1), no physical bridge, and `+0x16BC = Weeder`. |
| `coord-cell-conversions/_parity.md` | GREEN | Row 35 is already reopened as DRIFT and correctly separates accepted target, stock GetDockCoord, QueueingCell, no physical bridge, and source-aware radio/timer handoff. |

## 5. Exact Stale-Doc Replacement Wording

### `miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`

Replace the Overview paragraph that starts "The later physical-arrival/link step is a separate gate" with:

```markdown
Later unload/link handoff is source-specific and must not be collapsed into the accepted movement anchor. `UnitClass::PerCellProcess` has a `GetDockCoord` equality branch that can send radio `0x15`, but stock `GetDockCoord` for a 4x3 refinery is `NW+(2,1)`, while the accepted `0x12` target is `NW+(3,1)`. `UnitClass::Receive_Radio(0x16)` can also send `0x15` from the stopped accepted-cell state without a physical move to `GetDockCoord`.
```

Replace the paragraph that starts "For stock GAREFN/NAREFN, the art files explicitly remove occupancy" with:

```markdown
For stock GAREFN/NAREFN, art `RemoveOccupy` opens `(3,1)`, so the accepted hardcoded `NW+(3,1)` cell is a passable/opened refinery pad cell. That means the accepted radio anchor coincides with the stock art-opened pad cell, but it does not coincide with stock `GetDockCoord`/PerCellProcess equality coordinate `NW+(2,1)`, and `QueueingCell=4,1` remains the adjacent waiting/fallback cell.
```

Replace OQ-5 with:

```markdown
[RESOLVED] OQ-5 - How does this relate to physical pad/link? The accepted cell `(NW+3,NW+1)` is opened by stock refinery art (`GAREFN RemoveOccupy1=3,1`, `NAREFN RemoveOccupy8=3,1`). This proves passability/art-pad opening only. Later handoff can come from source-specific `0x16` or `PerCellProcess` branches; stock `GetDockCoord` equality is `NW+(2,1)` and is not the accepted anchor.
```

### `BUILDING_RECEIVE_RADIO_0E_STOCK_REFINERY_CANDOCK_CELL_GHIDRA_REPORT.md`

Replace the sentence "NW+(2,1) belongs to the BuildingClass::GetDockCoord +0x16BC branch..." with:

```markdown
`NW+(2,1)` is the stock 4x3 `BuildingClass::GetDockCoord` cell through the `+0x16BB Refinery=yes` branch; the `+0x16BC Weeder=yes` branch also returns `NW+(2,1)` but is not stock GAREFN/NAREFN. Neither `GetDockCoord` branch is the accepted `0x0E` `MOVE_TO_CELL` payload, which remains `NW+(3,1)`.
```

Replace the negative-input paragraph that starts "`BuildingClass::GetDockCoord` is a real function..." with:

```markdown
`BuildingClass::GetDockCoord` is a real function. For stock GAREFN/NAREFN it reaches the `+0x16BB Refinery=yes` branch and, for a 4x3 foundation, returns centered coordinates in cell `NW+(2,1)`. The `+0x16BC Weeder=yes` branch also returns packed `NW+(2,1)` but is not stock GAREFN/NAREFN. This function is not invoked by the standard accepted `CAN_DOCK(0x0E)` payload computation.
```

Replace OQ-09 with:

```markdown
- `[RESOLVED] OQ-09 - Is `NW+(2,1)` active for stock accepted GAREFN/NAREFN admission? -> No for accepted admission; yes as the separate stock 4x3 `GetDockCoord` cell via `+0x16BB Refinery=yes`.` (evidence: `0x0043C2D0`, `0x00447B20`, stock `Refinery=yes`)
```

Replace the `_system.md` follow-up wording line that says "`NW+(2,1)` is a separate `GetDockCoord +0x16BC` branch until proven live..." with:

```markdown
stock accepted miner admission remains `NW+(3,1)`; stock 4x3 `GetDockCoord` is `NW+(2,1)` via `+0x16BB Refinery=yes`; `+0x16BC` is the separate Weeder flag; `QueueingCell=4,1` remains `NW+(4,1)`.
```

### `coord-cell-conversions/fn-building-getdockcoord.md`

Replace the "Active in YR" paragraph after the vtable slot proof with:

```markdown
**Yes.** Bound to BuildingClass vtable slot `0xA8`. In stock refinery docking, calls to this function resolve through `+0x16BB Refinery=yes` and return the 4x3 `NW+(2,1)` coordinate. It is active in side-check/equality paths, but it is not the accepted `BuildingClass::Receive_Radio(0x0E)` movement target and is not required for every unload handoff because `UnitClass::Receive_Radio(0x16)` can send `0x15` from stopped accepted `NW+(3,1)` state.
```

Replace "This is the later dock-arrival coordinate checked by `UnitClass::PerCellProcess` before radio 0x15" with:

```markdown
This is the coordinate used by the `UnitClass::PerCellProcess` `GetDockCoord` equality branch before one possible radio `0x15` handoff. It is not required before every `0x15`, because the `0x16` path can hand off from the stopped accepted cell.
```

### `coord-cell-conversions/_system.md`

No stale replacement is required. If tightening is desired, replace "`GetDockCoord arrival cell`" with "`GetDockCoord equality cell`" where the text discusses `NW+(2,1)`, because `0x16` can hand off without an arrival at that cell.

### `coord-cell-conversions/_parity.md`

No stale replacement is required. Row 35 already uses correct active-YR wording: `+0x16BB Refinery=yes`, `+0x16BC Weeder=yes` distinct, accepted `NW+(3,1)`, stock `GetDockCoord` `NW+(2,1)`, `QueueingCell` `NW+(4,1)`, and no physical NW+3 -> NW+2 bridge.

## 6. Implementation / Doc Handoff

- Preserve three names in implementation/docs: accepted `0x0E` target `NW+(3,1)`, stock 4x3 `GetDockCoord` equality cell `NW+(2,1)`, and `QueueingCell` wait/fallback `NW+(4,1)`.
- When docs mention "pad", qualify it: "art-opened/passable pad cell" may be NW+(3,1), while "`GetDockCoord`/PerCellProcess equality cell" is NW+(2,1). Do not use "pad/link cell" unqualified.
- When docs mention flags, use `+0x16B3 = DockUnload`, `+0x16BB = Refinery`, `+0x16BC = Weeder`. Do not describe `+0x16BC` as the stock refinery flag.
- Future doc edits should patch the two YELLOW docs above with the exact replacement text, then leave `_system.md` and `_parity.md` as the current canonical wording unless new evidence appears.

## 7. Negative Facts

- Accepted stock `CAN_DOCK(0x0E)` does not use `GetDockCoord`, `QueueingCell`, `DockingOffset`, `NumberOfDocks`, or foundation dimensions for its `0x12` payload.
- `RemoveOccupy=3,1` opens/passability-clears the accepted cell; it does not redefine `GetDockCoord`.
- Stock GAREFN/NAREFN are `DockUnload=yes` and `Refinery=yes`, not `Weeder=yes`.
- Accepted radio anchor and stock `GetDockCoord` do not coincide for 4x3 stock refineries: `NW+(3,1)` vs `NW+(2,1)`.
- Accepted radio anchor and stock art-opened passable pad cell do coincide at `NW+(3,1)` for GAREFN/NAREFN. This is an art/passability statement only.
- `QueueingCell=4,1` is staging/fallback/wait data, not the accepted target.
- There is no proven physical bridge/move from accepted `NW+(3,1)` to `GetDockCoord` `NW+(2,1)` in `gamemd`.

## 8. Remaining Uncertainty

- Exact first rendered frame/source of every `0x15` in a concrete retail replay remains runtime-sensitive. Static docs prove the coordinate split and source possibilities, not every replay's winning source.
- The best short label for the player's visible "dock/link cell" remains ambiguous. Use explicit labels instead: "art-opened/passable accepted cell" for NW+(3,1), and "`GetDockCoord` equality cell" for NW+(2,1).
- This audit did not inspect or modify Rust and did not patch the stale docs directly by assignment.

## Sources

- Fresh read-only Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Fresh read-only Ghidra decompile: `BuildingClass::GetDockCoord @ 0x00447B20`.
- Fresh read-only Ghidra decompile: `BuildingClass::GetCoords @ 0x00447AC0`.
- Fresh read-only Ghidra decompile: `UnitClass::PerCellProcess @ 0x00739EC0`.
- Fresh read-only Ghidra decompile: `UnitClass::Receive_Radio @ 0x00737430`.
- Fresh read-only Ghidra decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`: `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`: `[GAREFN]`, `[NAREFN]` `QueueingCell` and `RemoveOccupy` lines.
