# CliffBackImpassability RecalcAttributes Recheck - Ghidra Research Report

**Address(es):** `0x0047D2B0` (`CellClass::RecalcAttributes`), `0x0066F1D9` (`RulesClass::ReadGeneral` key read)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `[General] CliffBackImpassability`, the three `RecalcAttributes` consumer copies, mode `0/1/2` behavior, six neighbor offsets, height comparison, land-type write gates, standard YR activity, and current Rust status in `src/map/resolved_terrain.rs`.  
**Non-Scope:** Full LAT/tile classification, `CellClass::RecalcZoneType` internals beyond post-write ordering, cliff rendering, bullets, Z-fudge, and A* consumers.  
**Confidence:** High for the scoped branch and stock YR activity; Medium for semantic names of binary `LandType` values because this report relies on existing land-type docs for labels.  
**Active in YR:** Yes. Stock `ini/rulesmd.ini` line 409 sets `CliffBackImpassability=2`.

## Summary

`CliffBackImpassability` is a live YR rule byte at `RulesClass+0x664`. `CellClass::RecalcAttributes` reads it in three repeated six-neighbor checks. Value `0` skips the check, value `1` runs the neighbor scan but never writes `LandType`, and value `2` writes `Cell+0xEC LandType = 3` when a checked neighbor is at least four signed level units above the current cell and the branch-specific land-type gate allows the override.

The current Rust implementation is close for stock output on ordinary clear/water/beach cells, but it is not an exact mechanism. Rust clamps the INI value to `0..2`, runs only a single post-resolve pass for value `2`, and models only the final-style eligible local land buckets. The binary has three branch-specific write sites, no verified clamp in the reader, signed `Level` comparisons, and an earlier overlay branch that can force `LandType=3` without the final land-type set filter.

## Verified Binary Findings

### Rule reader and storage

| Fact | Evidence | Confidence |
|---|---|---|
| The rule is read from `[General] CliffBackImpassability` in `RulesClass::ReadGeneral`. | Assembly at `0x0066F1D9`: `PUSH 0x83c8cc` string, call `0x005276D0`, then `MOV byte ptr [ESI+0x664], AL` at `0x0066F1E6`. Xref to string `0x0083C8CC` only from `0x0066F1D9`. | High |
| The field is one byte at `RulesClass+0x664`. | Reader writes `AL` to `[ESI+0x664]`; consumer reads `byte ptr [Rules+0x664]`. | High |
| The binary reader stores the low byte returned by the INI read; this pass found no clamp at the reader site. | `0x0066F1CB` sign-extends existing byte as default, `0x0066F1E1` calls the integer reader, `0x0066F1E6` stores `AL`. | High for no local clamp at this site |
| Stock YR activates maximal mode. | `ini/rulesmd.ini:409` and `ini/rules.ini:319` both contain `CliffBackImpassability=2`. | High |

### Consumer locations and modes

`CellClass::RecalcAttributes @ 0x0047D2B0` has three inlined copies of the cliff-back scan:

| Copy | Approx range | Branch role | Write gate |
|---|---|---|---|
| Copy 1 | `0x0047D386..0x0047D548` | Early overlay branch after overlay-derived `LandType` and optional slope/overlay cleanup | If any neighbor qualifies and mode is `2`, writes `LandType=3` unconditionally for that branch. |
| Copy 2 | `0x0047D5FF..0x0047D7CB` | Invalid/no usable tile fallback after setting clear defaults | If any neighbor qualifies and mode is `2` and current `LandType==0`, writes `LandType=3`. |
| Copy 3 | `0x0047DB59..0x0047DD34` | Final normal path after tile/LAT/tube/anim side effects | If any neighbor qualifies and mode is `2` and current binary `LandType in {0,2,6,8}`, writes `LandType=3`. |

Mode semantics:

| Mode byte | Behavior | Evidence |
|---|---|---|
| `0` | Skips the neighbor scan entirely. | Each copy tests byte `Rules+0x664` and jumps out on zero, e.g. `0x0047D38B..0x0047D393`, `0x0047DB5E..0x0047DB66`. |
| `1` | Runs the six-neighbor scan, but cannot write `LandType=3` because all write sites compare the byte to `2`. | Copy 3: `0x0047DD02` loads rules pointer, `0x0047DD08` compares `[Rules+0x664]` to `2`, `JNZ 0x0047DD34`; copy 2: `0x0047D7AE` compare to `2`, `JNZ 0x0047D7CB`; copy 1: `0x0047D52F..0x0047D53E`. |
| `2` | Runs the scan and may write `LandType=3`. | Write instructions: `0x0047D53E`, `0x0047D7C1`, `0x0047DD2A`. |
| Other nonzero values | Run the scan but do not write unless the stored byte equals `2`. | Consumer tests `!=0` to enter and `==2` to write. This follows from the same instructions as mode `1`; no clamp was seen at the reader site. |

### Neighbor offsets and height comparison

All three copies use the same six offsets, in this order:

| Order | Offset from current `(X,Y)` | Notes |
|---|---|---|
| 1 | `(X, Y-1)` | North-ish in map coordinates |
| 2 | `(X-1, Y)` | West-ish |
| 3 | `(X+2, Y+2)` | Peculiar two-step SE offset; verified, not a typo in one copy |
| 4 | `(X+1, Y+1)` | SE |
| 5 | `(X-1, Y+1)` | SW |
| 6 | `(X+1, Y-1)` | NE |

The check omits `(X,Y+1)` and `(X-1,Y-1)`.

The height predicate is signed and inclusive at a four-level difference:

```text
qualifies if neighbor.Level >= current.Level + 4
```

Evidence:

- Copy 1 first check: `0x0047D3C3` `MOVSX EDX, byte ptr [neighbor+0x11B]`, `0x0047D3CA` `MOVSX EAX, byte ptr [ESI+0x11B]`, `0x0047D3D1` `ADD EAX,0x4`, `0x0047D3D4` compare, `0x0047D3D6` `JLE` to the write-gate path.
- Copy 1 third check: `0x0047D44F..0x0047D462` has the same signed `current+4 <= neighbor` shape for `(X+2,Y+2)`.
- Copy 3 first check: `0x0047DB96..0x0047DBA9` repeats the same signed pattern.
- Search for `83 C2 04 3B D1` found the six repeated comparison sites in copy 2 at `0x0047D64B`, `0x0047D68E`, `0x0047D6D7`, `0x0047D71C`, `0x0047D761`, `0x0047D7A2`.

### Field writes and ordering

| Write / effect | Evidence | Ordering |
|---|---|---|
| The override writes `Cell+0xEC LandType = 3`. | Copy 1 `0x0047D53E`, copy 2 `0x0047D7C1`, copy 3 `0x0047DD2A`. | Before `CellClass::RecalcZoneType` in each path. |
| Copy 1 calls LAT/slope fixup after possible cliff-back write, then recalculates zone type. | `0x0047D548` target: `CALL 0x0047CA80`, then `0x0047D551` calls `0x00483C80`. | The cliff-back `LandType` can feed LAT/fixup and zone-type recomputation. |
| Copy 2 returns after possible write and directly recalculates zone type; no LAT call in this fallback return path. | `0x0047D7CB` calls `0x00483C80` after the optional write. | Fallback clear/default path. |
| Copy 3 writes after tile/LAT/tube/anim side effects and immediately before zone type. | `0x0047DD2A` write, `0x0047DD36` call `0x00483C80`. | Final normal path. |
| The function mirrors resulting level/zone data after `RecalcZoneType`. | Existing full passability report cites `0x0047D551`, `0x0047D7CD`, `0x0047DD36`; this pass rechecked copy 3 context at `0x0047DD3F..0x0047DD57`. | Always after the optional cliff-back write. |

### Eligible land types

Branch-specific gates matter:

- Copy 1: no final land-type set filter after the neighbor scan. The branch itself is entered only for overlay-driven conditions: overlay-derived `LandType == 4`, `LandType == 9`, or `OverlayType+0x2AC != 0`, per decompile at `0x0047D2B0`.
- Copy 2: after invalid/no-tile fallback, writes only when current `LandType == 0`; evidence `0x0047D7B7..0x0047D7C1`.
- Copy 3: writes only when binary `LandType` is `0`, `2`, `6`, or `8`; evidence `0x0047DD11..0x0047DD2A`.

The common older summary "Clear/Water/Beach/Ice" is adequate as a semantic label for copy 3, but it is incomplete for copy 1 because copy 1's eligibility is determined by the early overlay branch rather than the final `{0,2,6,8}` set.

## Active YR Status

Active in standard Yuri's Revenge: **Yes**.

- `ini/rulesmd.ini:409` sets `CliffBackImpassability=2`.
- `CellClass::RecalcAttributes` is called during map/scenario initialization and runtime terrain/overlay mutations. The prior full passability report verified representative active callers: `MapClass::InitCellAttributes @ 0x00568DF4`, `ScenarioClass::Full_Init @ 0x00687A5A`, building placement `0x0044203A`, overlay placement `0x005FC981`, wall/overlay destruction `0x00480F03`, ore removal `0x00480BDC`, and bridge damage/repair callers.
- This is not TS-dead for stock YR. The branch's stock rule value is maximal mode `2`.

## Rust Delta

Current Rust surfaces:

- Rule storage and parsing: `src/rules/ruleset.rs`, field `RuleGeneral::cliff_back_impassability`, default `2`, parsed via `.get_i32("CliffBackImpassability").unwrap_or(2).clamp(0, 2) as u8`.
- Application handoff: `src/app_init.rs` passes the parsed field into terrain build.
- Terrain implementation: `src/map/resolved_terrain.rs:751..804`.
- Tests: `src/map/resolved_terrain.rs:2258..2378`.

Observed status:

| Area | Current Rust | Binary status | Delta |
|---|---|---|---|
| Stock default | Rust default/parses stock `2`. | Stock YR `rulesmd.ini=2`. | No stock-value delta. |
| Nonstock values | Rust clamps to `0..2`. | Reader stores low byte; consumers treat nonzero/non-2 as "scan but no write". | Mechanism drift for modded values above `2` or negative values. |
| Mode `1` | Rust does not run the scan because implementation is gated by `==2`. | Binary runs scan but no write. | State output likely same, exact mechanism/call behavior differs. |
| Neighbor offsets | Rust uses `(0,-1), (-1,0), (2,2), (1,1), (-1,1), (1,-1)`. | Same order and offsets. | Match. |
| Height predicate | Rust uses `cells[nidx].level >= cell_level + 4` with `u8`. | Binary uses signed `MOVSX` byte levels and `neighbor >= current + 4`. | Match for ordinary YR level range; signedness not exact for out-of-range/malformed levels. |
| Branch copies | Rust has one post-resolve pass. | Binary has three branch-specific copies inside `RecalcAttributes`. | Mechanism drift; output drift possible on overlay early-branch cases. |
| Eligible land types | Rust checks local compatibility `Clear`, `Water`, `Beach`. | Binary copy 3 checks binary `LandType in {0,2,6,8}`; copies 1 and 2 have different gates. | Partial/unchecked. Rust may match collapsed Ice-as-Clear output, but branch-specific overlay behavior is not proven. |
| Write effects | Rust sets `land_type=Rock`, `ground_walk_blocked=true`, `is_cliff_like=true`. | Binary writes only `Cell+0xEC LandType=3`; `ZoneType` is then recomputed. | Rust adds derived/cache fields, which is acceptable only if they are binary-equivalent downstream. |

Important mapping note: Rust's local `LandType` compatibility enum is not byte-identical to the binary `Cell+0xEC LandType` labels used in the decompile. For example, current Rust uses `Water=4` and `Beach=3`, while the verified binary copy-3 comparisons use values `{0,2,6,8}`. Future implementation should compare against the canonical binary land-type model or prove the compatibility mapping preserves all outputs.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `CliffBackImpassability` is a byte read from `[General]`, stock YR value `2`; consumer mode is `0` skip, nonzero scan, `2` write. | `0x0066F1D9..0x0066F1E6`, `0x0047D386`, `0x0047D52F`, `0x0047D7AE`, `0x0047DB59`, `0x0047DD08`; `ini/rulesmd.ini:409`. | Rust clamps to `0..2` and scans only for `2`. | `src/rules/ruleset.rs`, `src/map/resolved_terrain.rs`. | Preserve stock `2` behavior; decide whether modded nonstock values should keep binary low-byte semantics rather than clamp. | INI values `0`, `1`, `2`, `3`, `258`, and `-1` on the same cliff fixture: only values whose stored byte equals `2` should write; all nonzero values would scan in gamemd but only `2` writes. | Do not assume "greater than 1" means enabled. Consumer checks exactly `==2`. |
| The six offsets are exactly `(0,-1), (-1,0), (2,2), (1,1), (-1,1), (1,-1)`; `(X+2,Y+2)` is real and `S`/`NW` are omitted. | Copy ranges `0x0047D386..0x0047D548`, `0x0047D5FF..0x0047D7CB`, `0x0047DB59..0x0047DD34`; assembly contexts around `0x0047D3C3`, `0x0047D44F`, `0x0047DB96`. | Rust matches the offset list. | `src/map/resolved_terrain.rs:758..761`. | Keep exact asymmetric offsets. | Unit test each of the six offsets independently, plus negative tests for omitted `(0,1)` and `(-1,-1)`. | Do not "fix" the asymmetric list to a normal 8-neighbor ring. |
| Height comparison is signed `neighbor.Level >= current.Level + 4`. | `MOVSX`/`ADD 4`/`CMP`/`JLE` sequence at `0x0047D3C3..0x0047D3D6`, `0x0047D44F..0x0047D462`, `0x0047DB96..0x0047DBA9`. | Rust matches ordinary range but uses unsigned `u8`. | `src/map/resolved_terrain.rs:783..785`. | Use signed level semantics if Rust ever preserves raw `Cell+0x11B` beyond normal `0..14` map levels. | Boundary test with current level `0`, neighbor `3` no write; neighbor `4` write; if malformed raw levels are supported, add signed byte boundary tests. | Do not use absolute height difference; only higher neighbor by at least four qualifies. |
| Copy 3 writes only for binary `LandType in {0,2,6,8}`; copy 2 only for `0`; copy 1 has no final set filter after the overlay branch gate. | Copy 3 `0x0047DD11..0x0047DD2A`; copy 2 `0x0047D7B7..0x0047D7C1`; copy 1 `0x0047D52F..0x0047D53E` plus decompile branch condition. | Rust uses one local `Clear/Water/Beach` gate after terrain resolve. | `src/map/resolved_terrain.rs:762..771`, overlay/terrain metadata path. | Model branch-equivalent eligibility or prove current post-resolve compatibility mapping gives identical `ZoneType` for all affected overlay/tile cases. | Fixtures: no-tile clear fallback; normal clear/water/beach/ice-equivalent cells; early overlay branch with overlay land `4`; early overlay branch with overlay land `9`; noneligible rock/rough/tiberium cells. | Do not collapse this to "all passable cells" without proving overlay and ice cases. |
| The `LandType=3` override happens before `RecalcZoneType`, so the forced land type feeds reduced zone classification in the same call. | Copy 1 `0x0047D53E` then `0x0047D548..0x0047D551`; copy 2 `0x0047D7C1` then `0x0047D7CB`; copy 3 `0x0047DD2A` then `0x0047DD36`. | Rust directly sets blocking/cache fields in a full terrain-grid build. | `src/map/resolved_terrain.rs`, `src/sim/pathfinding/zone_build.rs`. | Ensure the forced rock/impassable classification is visible to zone/path construction in the same logical update. | After forcing a cell, its reduced zone type/path-grid passability must be impassable before any path query can consume it. | Do not only set a visual `is_cliff_like` flag; pathing must see the land/zone effect. |

## Acceptance Tests

1. `cliff_back_offsets_exact_six`: for each verified offset, place one neighbor exactly four levels above the current cell and assert the current cell becomes impassable in mode `2`.
2. `cliff_back_omitted_offsets_do_not_trigger`: place high neighbors only at `(0,1)` and `(-1,-1)` and assert no cliff-back write.
3. `cliff_back_height_threshold_inclusive`: diff `3` does not write; diff `4` writes.
4. `cliff_back_mode_values`: stock `2` writes, `0` and `1` do not write. If modded parity is in scope, include `3`, `258`, and `-1` using byte-storage semantics.
5. `cliff_back_binary_landtype_gate`: copy-3-equivalent fixture covers binary land values `0`, `2`, `6`, `8` as eligible and representative noneligible values as no-write.
6. `cliff_back_overlay_branch_gate`: overlay-derived early branch cases for land `4`, land `9`, and `OverlayType+0x2AC` force `LandType=3` when behind a cliff in mode `2`.
7. `cliff_back_zone_visible_same_recalc`: the forced land type feeds reduced zone/path-grid classification before the next path query.

## Remaining Uncertainty

- Exact semantic labels for binary `LandType` values `2`, `6`, and `8` are inherited from prior docs in this report. The numeric gate is verified here; a separate land-type parser/table audit should own the final labels.
- Copy 1 overlay branch labels are only bounded to overlay `LandType == 4`, `LandType == 9`, or `OverlayType+0x2AC != 0` from decompilation. This report did not audit every stock overlay type that reaches that branch.
- This pass did not prove whether `CCINIClass__ReadInt @ 0x005276D0` internally clamps or transforms arbitrary text values before returning. The local reader site stores `AL`, and the consumers check byte semantics.
- Runtime side effects of mode `1` scanning are believed to be state-output neutral because `MapClass__Get_CellClass` is a lookup, but this pass did not runtime-watch for incidental cache effects. For stock YR, mode `1` is not used.

## Sources

- Ghidra decompile: `CellClass::RecalcAttributes @ 0x0047D2B0`.
- Ghidra assembly contexts: `0x0047D386`, `0x0047D3C3..0x0047D3D6`, `0x0047D44F..0x0047D462`, `0x0047D52F..0x0047D548`, `0x0047D5FF`, `0x0047D64B`, `0x0047D68E`, `0x0047D6D7`, `0x0047D71C`, `0x0047D761`, `0x0047D7A2`, `0x0047D7AE..0x0047D7C1`, `0x0047DB59`, `0x0047DB96..0x0047DBA9`, `0x0047DD02..0x0047DD36`.
- Ghidra reader evidence: `RulesClass::ReadGeneral @ 0x0066D530`, key site `0x0066F1D9..0x0066F1E6`, string `0x0083C8CC`.
- Xrefs: `CellClass::RecalcAttributes @ 0x0047D2B0` caller list from Ghidra MCP; string xref for `0x0083C8CC`.
- INI: `ini/rulesmd.ini:409`, `ini/rules.ini:319`.
- Existing docs checked: `CLIFF_OBJECTS_GHIDRA_REPORT.md`, `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`, `docs/research/bridges/00-system-models/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`, `RULESCLASS_FIELDS.csv`.
- Rust surfaces checked: `src/map/resolved_terrain.rs`, `src/rules/ruleset.rs`, `src/app_init.rs`, `src/sim/pathfinding/passability.rs`.
