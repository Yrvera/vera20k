# ReleaseDockedHarvester Exit Anchor Recheck - Ghidra Research Report

**Address(es):** `0x004595C0` primary, `0x0041BEA0`, `0x0056DC20`, `0x0073D66D`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** only the post-dump/release-path exit anchor computation inside `BuildingClass__ReleaseDockedHarvester`, the function behind building vtable `+0x1B8`, and the inputs handed to `FootClass__Find_Nearby_Passable_Cell`.
**Non-Scope:** full unload cadence, stock refinery dump state machine, animation semantics beyond ordering around the anchor, full `Find_Nearby_Passable_Cell` ranking, runtime frame capture.
**Confidence:** High for static binary formulas and call ordering; Medium for stock-path reachability because sibling reports already show the ordinary zero-link refinery exit bypasses this helper.
**Active in YR:** Conditional. The function exists in the live YR binary and is called from `UnitClass__Mission_Deploy_Building @ 0x0073D66D`, but that caller reaches it only when `unit+0x2E4 != 0`; the verified stock `DockUnload` zero-link path exits through Mission_Deploy_Building state 4 without this anchor.

## 1. Overview

`BuildingClass__ReleaseDockedHarvester @ 0x004595C0`, when reached with a non-null building dock link and a Drive-type docked unit, computes its passable-cell search seed from the building's packed cell, not from `QueueingCell` and not from the foundation center. The exact seed is `building.Get_Cell_Packed() + (-1,+1)`.

For a `GAREFN` placed with NW/origin cell `(10,10)`, the concrete seed passed into `FootClass__Find_Nearby_Passable_Cell` is `(9,11)`. This report does not claim the full stock chrono-miner post-dump route uses this helper in the ordinary zero-link path; it records the helper's anchor behavior when the helper is executed.

## 2. Key Offsets / Functions

| Item | Offset / address | Finding | Evidence | Active in YR |
|---|---:|---|---|---|
| Building dock link | building `+0x2E4` | Null path returns before destination computation; non-null path may compute the exit seed. | `0x004595C0` decompile reads `param_1->field_0x2e4`. | Conditional: only non-null link path. |
| Unit dock link | unit `+0x2E4` (`piVar1[0xB9]`) | Cleared before the anchor computation. | `0x004595C0`: `piVar1[0xb9] = 0`. | Conditional: only non-null link path. |
| Building vtable `+0x1B8` | data xref `0x007E4074 -> 0x0041BEA0` | This is the cell-producing helper used by the anchor computation. | `0x004595C0` calls `param_1->vtable+0x1B8`; `get_xrefs_to 0x0041BEA0` includes `0x007E4074 [DATA]`. | Yes as a live object helper; Conditional in this release branch. |
| `ObjectClass__Get_Cell_Packed` | `0x0041BEA0` | Converts object location leptons at `+0x9C/+0xA0` into packed cell shorts by signed divide-by-256. | decompile `0x0041BEA0`. | Yes. |
| `FootClass__Find_Nearby_Passable_Cell` | `0x0056DC20` | Receives the packed seed after the `(-1,+1)` edit. | call xref from `0x004597E3` in `BuildingClass__ReleaseDockedHarvester`. | Yes globally; Conditional for this caller. |

## 3. Core Logic

Relevant `0x004595C0` ordering:

1. Clears building animation slots `0xA` and `0xB`, then may play the exit sound and create slots `0xC`/`0xD`. These happen before the anchor.
2. Reads `building+0x2E4`. If null, clears `building+0x718`, sets building mission `5`, and returns; no anchor is computed.
3. Calls the docked unit vtable `+0x2C` and continues only when that returns `1`.
4. Clears `unit+0x2E4`.
5. Calls the active locomotor and force-track setup first.
6. Calls building vtable `+0x1B8`; the vtable target for BuildingClass is `ObjectClass__Get_Cell_Packed @ 0x0041BEA0`.
7. Builds the packed anchor with `x - 1`, `y + 1`:

```text
packed = building.vtable+0x1B8(...)
anchor.x = packed.x - 1
anchor.y = packed.y + 1
```

8. Calls `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.
9. Converts the result to `CellClass*`, calls unit vtable `+0x480(dest,1)`, then unit vtable `+0x1E8(MOVE=2,0)`.

`ObjectClass__Get_Cell_Packed @ 0x0041BEA0`:

```text
cell.x = (Location_X + ((Location_X >> 31) & 0xFF)) >> 8
cell.y = (Location_Y + ((Location_Y >> 31) & 0xFF)) >> 8
```

For a placed building whose origin/NW cell is `(10,10)`, this helper produces `(10,10)`. The release helper then changes that to `(9,11)`.

## 4. Find_Nearby_Passable_Cell Inputs

At the release call site (`0x004597E3` xref to `0x0056DC20`), the material inputs visible in the decompile are:

| Input | Value in this caller | Evidence | Active in YR |
|---|---|---|---|
| Seed cell | `Get_Cell_Packed() + (-1,+1)` | `0x004595C0`: `CONCAT22(psVar6[1] + 1, *psVar6 + -1)` before the call. | Conditional. |
| Speed type | docked unit type field `+0x67C` | `0x004595C0`: type query via unit vtable `+0x84`, then `*(iVar4 + 0x67c)` passed to `Find_Nearby_Passable_Cell`. | Conditional. |
| Zone / special index | `-1` after local `uVar8 = 0xffffffff` | immediate setup before the call. | Conditional. |
| Boolean flags | displayed decompile passes `0,0,1,1,0,0,0,1,...,0,0` after the `-1` value | `0x004595C0` decompile at the `FootClass__Find_Nearby_Passable_Cell` call. | Conditional. |
| Preferred/reference local | local pointer initialized around the call | `puVar17 = &uStack_40`; `uStack_40` is reset before call. Exact semantic name deferred. | Conditional. |

The full candidate ordering/ranking inside `0x0056DC20` was intentionally not expanded. The function does begin by reading the seed's `x/y` from its seed pointer and searches rings up to a capped mover-dependent range (`+0xF4 + +0xF8`, capped at `0x20`), but this slot only needed the caller's seed and parameter source.

## 5. Concrete GAREFN Seed

Retail YR `GAREFN` has `Foundation=4x3` in `ini/artmd.ini:1766`, and `QueueingCell=4,1` in `ini/artmd.ini:1773`. `ReleaseDockedHarvester` does not read `QueueingCell` for this anchor.

For a `GAREFN` placed with NW/origin cell `(10,10)`:

```text
Get_Cell_Packed() = (10,10)
ReleaseDockedHarvester anchor = (10 - 1, 10 + 1)
Find_Nearby_Passable_Cell seed = (9,11)
```

`(9,11)` is one cell west of the 4x3 foundation's left edge and one row down from the NW row. The queue cell would be `(14,11)`, but that is not this helper's seed.

## 6. Integration / Activity Notes

The only function xref to `0x004595C0` returned by Ghidra is `UnitClass__Mission_Deploy_Building @ 0x0073D66D`. In that caller, the call sits in the `else` branch for `unit+0x2E4 != 0`; the main stock refinery unload path when `unit+0x2E4 == 0` has separate state-3/state-4 handling and does not call this helper.

Active in YR: Conditional. Evidence: `UnitClass__Mission_Deploy_Building` is live for units; `[CMIN]` has `Dock=NAREFN,GAREFN` and `Harvester=yes` in `ini/rulesmd.ini:7361` and `ini/rulesmd.ini:7364`; `[GAREFN]` is a live refinery with `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, and `FreeUnit=CMIN` in `ini/rulesmd.ini:11726-11736`. However, the binary call to this helper is gated by the nonzero reciprocal link, so this report must not be read as proof that ordinary stock `CMIN -> GAREFN` zero-link dump completion uses `(9,11)`.

TS legacy check: no TS-only global or INI gate was found inside this narrow anchor sequence. The path is YR-live code, but conditional on runtime link state rather than a TS/YR mode flag.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` anchor sequence | verified | decompile `0x004595C0`; call to `0x0056DC20` from `0x004597E3` | none for anchor slice |
| Building vtable `+0x1B8` target | verified | `0x004595C0` indirect call; xref `0x007E4074 -> 0x0041BEA0` | none |
| `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` formula | verified | decompile `0x0041BEA0` | none |
| Concrete `GAREFN` `(10,10)` seed | verified/inferred from binary formula plus INI foundation | `0x004595C0`, `0x0041BEA0`, `ini/artmd.ini:1766` | runtime map obstacle result not checked |
| `Find_Nearby_Passable_Cell @ 0x0056DC20` full ranking | touched-not-exhausted | decompile `0x0056DC20` | out of scope |
| Ordinary stock zero-link refinery exit | touched-not-exhausted | decompile `0x0073D66D` shows separate branch | sibling reports cover it; not expanded here |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Is the anchor `GetCellLocation()+(-1,+1)`? Yes. Evidence: `0x004595C0` calls vtable `+0x1B8`, then constructs `CONCAT22(y+1,x-1)` before `0x0056DC20`.

[RESOLVED] OQ-2 - What function produces `GetCellLocation` here? `ObjectClass__Get_Cell_Packed @ 0x0041BEA0`, reached through BuildingClass vtable `+0x1B8` (`0x007E4074` data xref). Evidence: decompile and xrefs.

[RESOLVED] OQ-3 - What seed is produced for `GAREFN` NW `(10,10)`? `(9,11)`. Evidence: `0x0041BEA0` returns the origin packed cell; `0x004595C0` subtracts one from x and adds one to y.

[RESOLVED] OQ-4 - Is `QueueingCell=4,1` used by this anchor? No. Evidence: `0x004595C0` reads only the vtable `+0x1B8` packed cell before the seed edit; `ini/artmd.ini:1773` is data for other refinery queue logic, not read here.

[DEFERRED] OQ-5 - Does an open clean map always return `(9,11)` as the final `Find_Nearby_Passable_Cell` result? Category: out-of-scope. This slot verifies the seed and inputs; full candidate acceptance/ranking and runtime occupancy were not expanded.

## Sources

- Ghidra read-only decompiled: `0x004595C0`, `0x0041BEA0`, `0x0056DC20`, `0x0073D66D`.
- Ghidra read-only xrefs: `0x004595C0 <- 0x0073D66D`; `0x0056DC20 <- 0x004597E3`; `0x0041BEA0 <- 0x007E4074 [DATA]`.
- INI checked: `ini/artmd.ini:1766`, `ini/artmd.ini:1773`; `ini/rulesmd.ini:7361`, `ini/rulesmd.ini:7364`, `ini/rulesmd.ini:11726-11736`.
- Prior docs checked: `docs/traces/2026-05-21-trace-chrono-miner-post-dump-exit.md`, `docs/research/miner/CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md`, `docs/research/CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`, `docs/research/miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`.
