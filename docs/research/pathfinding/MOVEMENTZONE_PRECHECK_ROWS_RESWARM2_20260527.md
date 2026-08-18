# MovementZone Precheck Rows Reswarm2 - 2026-05-27

**Slot:** 4, second pathgrid re-swarm  
**Target:** MovementZone matrix row coverage and reduced-zone precheck requirements for pathgrid water/pier implementation.  
**Scope:** `ZonePassabilityMatrix @ 0x0082A594`, `MovementZone=` row mapping, `Zone_precheck @ 0x0042C290`, `AStar_pathfind_search @ 0x0042C900`, and Rust `zone_search`/`passability`/`zone_build`/`zone_map`.  
**Non-scope:** exact cell-level `Can_Enter_Cell`, exact WaterBridge TMP data, and code changes.  
**Status:** COMPLETE for row coverage and precheck gating. Rust code was not edited.

## Executive Verdict

There is no verified binary MovementZone-row whitelist for `Zone_precheck`. Active YR passes the current or overridden MovementZone row into `Zone_precheck`, and `Zone_precheck` gates graph edges with `ZonePassabilityMatrix[row][neighbor_zone_type] == 1`. Therefore the Rust gate in `zone_search.rs` that enables reduced-zone precheck only for `Normal`, `Amphibious`, `Infantry`, and `Fly` is a parity approximation, not a gamemd rule.

For the pathgrid water/pier issue, Rust can and should enable reduced-zone precheck for all valid MovementZone rows once the row matrix used by the precheck path is the binary reduced-zone matrix. That includes `Crusher`, `Destroyer`, `CrusherAll`, `AmphibiousDestroyer`, `AmphibiousCrusher`, `Subterranean`, `InfantryDestroyer`, `Water`, and `WaterBeach`. None of those rows intentionally bypass precheck in `AStar_pathfind_search`; ordinary Fly/Jumpjet locomotor paths may bypass A* entirely, but if a row reaches A*, the row is valid input.

Important Rust caveat: `src/sim/pathfinding/zone_build.rs` contains the verified binary-shaped `MOVEMENT_CLASS_PASSABILITY` table, but `src/sim/pathfinding/passability.rs` still exposes a stale/wrong `PASSABILITY_MATRIX` for several rows/columns and is used by `zone_hierarchy.rs::zone_precheck_flat`. Do not enable all rows against the stale `passability.rs` matrix without first unifying it with the verified table.

## Binary Evidence

### MovementZone rows

`CCINIClass__ReadMovementZone @ 0x00474E40` parses a 13-entry table and `TechnoTypeClass__ReadINI` stores the parser result to `TechnoTypeClass+0x5B4`. Existing verified report `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md` identifies the rows:

| Row | Name |
|---:|---|
| 0 | `Normal` |
| 1 | `Crusher` |
| 2 | `Destroyer` |
| 3 | `AmphibiousDestroyer` |
| 4 | `AmphibiousCrusher` |
| 5 | `Amphibious` |
| 6 | `Subterannean` |
| 7 | `Infantry` |
| 8 | `InfantryDestroyer` |
| 9 | `Fly` |
| 10 | `Water` |
| 11 | `WaterBeach` |
| 12 | `CrusherAll` |

Fresh Ghidra spot-check of `AStar_pathfind_search @ 0x0042C900` confirms the ordinary row source: assembly around `0x0042CA39` reads `TechnoType+0x5B4` into the local movement row. The JumpJet-infantry branch at `0x0042CA43..0x0042CA69` is the known exception: if `WhatAmI()==0xF` and `JumpJet=yes`, it overwrites the local row with constant `7` (`Infantry`).

### Matrix rows and water/beach/outside legality

`ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` verifies `ZonePassabilityMatrix` as `int[13][8]` at `0x0082A594`, ending at `0x0082A734`; only value `1` passes. Fresh Ghidra spot-checks match:

- `Zone_precheck @ 0x0042C299..0x0042C2B2`: computes row base as `0x82A594 + movement_zone * 0x20`.
- `Zone_precheck @ 0x0042C60A..0x0042C612`: compares `row[neighbor_zone_type] == 1`; non-`1` skips the edge.

The binary reduced zone-type columns are:

| Column | Meaning |
|---:|---|
| 0 | Ground/default |
| 1 | Crushable/road-like overlay class |
| 2 | Wall/destroyer class |
| 3 | Beach |
| 4 | Water |
| 5 | Building/object branch |
| 6 | Impassable/gate/zero-speed class |
| 7 | Outside/sentinel |

Row-by-row legality for the columns relevant to water/pier implementation:

| Row | Name | Beach col 3 | Water col 4 | Outside col 7 | Notes |
|---:|---|---|---|---|---|
| 0 | `Normal` | blocked | blocked | blocked | Standard land. |
| 1 | `Crusher` | blocked | blocked | blocked | Can use col 1 only in addition to ground. |
| 2 | `Destroyer` | blocked | blocked | blocked | Can use cols 1 and 2 in addition to ground. |
| 3 | `AmphibiousDestroyer` | pass | pass | blocked | Amphibious plus destroyer/building-class passability. |
| 4 | `AmphibiousCrusher` | pass | pass | blocked | Binary row exists; no active stock YR line found. |
| 5 | `Amphibious` | pass | pass | blocked | Standard amphibious row. |
| 6 | `Subterannean` | blocked | blocked | blocked | Binary row exists; no active stock YR line found. |
| 7 | `Infantry` | blocked | blocked | blocked | Can use col 5; cannot enter water/beach by matrix. |
| 8 | `InfantryDestroyer` | blocked | blocked | blocked | Binary row exists; one active stock YR line found. |
| 9 | `Fly` | pass | pass | blocked | Matrix row exists and blocks outside/sentinel; runtime use is locomotor-conditional. |
| 10 | `Water` | blocked | pass | blocked | Ships/water-only. |
| 11 | `WaterBeach` | pass | pass | blocked | Binary row exists; no active stock YR line found. |
| 12 | `CrusherAll` | blocked | blocked | blocked | Same matrix profile as `Destroyer`; one active stock YR line found. |

### A* precheck row gating

Fresh Ghidra spot-check of `AStar_pathfind_search @ 0x0042C900` confirms:

- The function derives the row from caller override or `TechnoType+0x5B4`; JumpJet infantry can coerce it to row 7.
- It calls `Zone_precheck` at `0x0042CB58` and again at `0x0042CCB3` on retry when the hierarchy flag remains enabled.
- The hierarchy flag is controlled by object/state/playfield conditions, not by a MovementZone-row whitelist.
- Cross-zone mismatch with hierarchy enabled returns failure before cell A*. Same-zone `Zone_precheck` failure disables hierarchy and can still run cell A*.

Verified inference: rows can be inactive because no standard unit type uses them, or because a locomotor path bypasses A*. That is not the same as a row-specific `Zone_precheck` bypass.

## Stock YR Row Activity

Text scan of active `MovementZone=` lines in `ini/rulesmd.ini`:

| Row | Name | Active stock YR explicit count | Standard YR content status |
|---:|---|---:|---|
| 0 | `Normal` | 35 | Active; also constructor/default row for missing key. |
| 1 | `Crusher` | 6 | Active. |
| 2 | `Destroyer` | 13 | Active. |
| 3 | `AmphibiousDestroyer` | 4 | Active. |
| 4 | `AmphibiousCrusher` | 0 | Binary-active row; no active stock YR line found. |
| 5 | `Amphibious` | 3 | Active. |
| 6 | `Subterannean` | 0 | Binary-active row; no active stock YR line found. |
| 7 | `Infantry` | 58 | Active. |
| 8 | `InfantryDestroyer` | 1 | Active. |
| 9 | `Fly` | 19 | Active in data; A* use is locomotor-conditional. |
| 10 | `Water` | 14 | Active. |
| 11 | `WaterBeach` | 0 | Binary-active row; no active stock YR line found. |
| 12 | `CrusherAll` | 1 | Active. |

Comments in `rulesmd.ini` contain inactive `AmphibiousCrusher`/`WaterBeach` lines; those do not make the rows stock-active. Binary still builds and reads all 13 rows.

## Current Rust Findings

### Correct or mostly correct

- `src/rules/locomotor_type.rs` declares all 13 rows plus an invalid `-1` sentinel and `MovementZone::all_ground()` currently includes every valid row, including `Fly`, `Water`, `WaterBeach`, and `CrusherAll`.
- `src/sim/pathfinding/zone_build.rs` contains `MOVEMENT_CLASS_PASSABILITY`, which matches the verified binary matrix rows for reduced zone types.
- `src/sim/pathfinding/zone_map.rs` iterates `MovementZone::all_ground()`, so the current full build can construct maps for all valid rows.
- `src/sim/pathfinding/zone_hierarchy.rs` has a binary-shaped consumer scaffold: three levels, parent gating, edge exclusions, insertion-order sequence, and MovementZone matrix lookup.

### Drift / implementation blockers

| Rust surface | Finding |
|---|---|
| `src/sim/pathfinding/zone_search.rs:62` | `can_use_reduced_zone_precheck()` only enables `Normal`, `Amphibious`, `Infantry`, and `Fly`; binary has no such row whitelist. |
| `src/sim/pathfinding/zone_search.rs:227` and `:530` | Rows excluded by the whitelist fall straight to cell A*, bypassing cross-zone rejection. This is directly relevant to `Crusher`, `Destroyer`, `CrusherAll`, `Water`, and `WaterBeach`. |
| `src/sim/pathfinding/passability.rs:115` | `PASSABILITY_MATRIX` is stale/wrong versus the verified binary matrix. Examples: `Normal` incorrectly passes col 2 and 6 and uses `2` for outside instead of `3`; `Fly` incorrectly passes outside; `Subterranean` incorrectly passes col 5 and outside; `Water` uses `2` for outside instead of `3`. |
| `src/sim/pathfinding/zone_hierarchy.rs:389` and `:433` | `zone_precheck_flat` calls `passability::is_passable_for_zone()`, so it inherits the stale `passability.rs` matrix instead of the verified `zone_build.rs` row table. |
| `src/sim/pathfinding/passability.rs:149` | `zone_layer_for_speed_type()` remains a SpeedType-to-row helper. It is acceptable only for legacy speed/cost compatibility, not for MovementZone reachability. |

## Row-by-Row Implementation Handoff

| Row | Name | Enable reduced-zone precheck? | Handoff |
|---:|---|---|---|
| 0 | `Normal` | Already enabled; keep enabled. | Must reject beach/water/outside via matrix before cell A*. Confirm stale `passability.rs` row is fixed to `[1,2,2,2,2,2,2,3]`. |
| 1 | `Crusher` | Yes. | Enable precheck. It passes only cols 0 and 1; water and beach must be cross-zone unreachable. |
| 2 | `Destroyer` | Yes. | Enable precheck. It passes cols 0,1,2; not water/beach. This row is active and important for wall/destroyer-class vehicles. |
| 3 | `AmphibiousDestroyer` | Yes. | Enable precheck. It can pass beach/water; ensure it still blocks col 6 and outside. |
| 4 | `AmphibiousCrusher` | Yes for binary parity; stock content inactive. | Build/precheck row anyway because gamemd builds all rows. It can pass beach/water but not walls/buildings/impassable/outside. |
| 5 | `Amphibious` | Already enabled; keep enabled. | Pass beach/water; block outside. |
| 6 | `Subterannean` | Yes for binary parity; stock content inactive. | Row must pass cols 0,1,2,6 only. Do not let it pass water/beach/outside. |
| 7 | `Infantry` | Already enabled; keep enabled. | Block beach/water/outside; pass col 5. |
| 8 | `InfantryDestroyer` | Yes. | Enable precheck. Active stock row; blocks water/beach/outside. |
| 9 | `Fly` | Keep row available; runtime A* use conditional. | If a Fly row enters this path, matrix blocks outside/sentinel. Do not model Fly as all-cells including OOB. |
| 10 | `Water` | Yes. | Enable precheck. Only water col 4 passes; beach/land/outside must hard reject before cell A*. This is important for water/shore path parity. |
| 11 | `WaterBeach` | Yes for binary parity; stock content inactive. | Enable when row exists. It can pass beach/water only; do not use as a generic hover/amphibious fallback. |
| 12 | `CrusherAll` | Yes. | Enable precheck. Active stock row; matrix equals `Destroyer` for zone passability, so it does not allow water/beach. Crushing behavior remains outside this matrix. |

## Required Tests

Suggested test names and intent:

| Test | Intent |
|---|---|
| `movement_zone_matrix_matches_binary_13x8` | Assert the single Rust matrix used by zone precheck equals the verified rows from `0x0082A594`, including value `3` for outside. |
| `reduced_zone_precheck_enabled_for_all_valid_rows` | Assert `can_use_reduced_zone_precheck(Some(row))` is true for rows 0..12 and false only for `Invalid`/explicit unsupported state. |
| `crusher_destroyer_crusherall_reject_water_and_beach_precheck` | Build a tiny land/water/beach grid and assert rows 1,2,12 fail reachability from land to water/beach before cell A*. |
| `water_row_only_reaches_water` | Assert row 10 reaches water-to-water but rejects water-to-beach and water-to-ground cross-zone targets. |
| `waterbeach_row_reaches_beach_and_water_only` | Assert row 11 can connect beach/water where graph adjacency exists and rejects ground/outside. |
| `fly_row_blocks_outside_sentinel` | Assert row 9 does not treat reduced zone type 7 as passable. |
| `zone_precheck_uses_movement_zone_not_speed_type` | A unit with `MovementZone=Water` and `SpeedType=Float` must use row 10, not a SpeedType-derived hover/fly row. |
| `jumpjet_infantry_astar_uses_infantry_row` | If JumpJet infantry reaches A*, row is coerced to Infantry row 7, not Fly row 9. |

## Negative Facts / Do Not Do

- Do not keep a hard-coded row whitelist for reduced-zone precheck and call it parity.
- Do not treat `Crusher`, `Destroyer`, or `CrusherAll` as unsafe to precheck merely because they were skipped in current Rust.
- Do not use `SpeedType` as the `ZonePassabilityMatrix` row.
- Do not treat `Fly` as passable outside the playfield; row 9 blocks column 7.
- Do not use the stale `passability.rs` matrix as the authoritative binary matrix until it is reconciled with `zone_build.rs::MOVEMENT_CLASS_PASSABILITY`.
- Do not infer a MovementZone-specific bypass from normal aircraft/jumpjet locomotor behavior. That is an entry-path issue, not a matrix/precheck row rule.

## Remaining Uncertainty

- Runtime frequency of row 9 (`Fly`) entering `FootClass::Find_Path -> AStar_pathfind_search` remains locomotor-conditional and should stay documented separately.
- Exact invalid row `-1` runtime consequences are not traced here. Parser/store behavior is verified, but this report does not recommend enabling precheck for `MovementZone::Invalid`.
- Exact WaterBridge TMP movement bytes remain outside this slot; this report only proves the matrix/precheck row contract once a cell is classified into reduced zone type 0..7.

## Sources

- Ghidra fresh spot-checks: `Zone_precheck @ 0x0042C290`, row-base setup `0x0042C299..0x0042C2B2`, matrix compare `0x0042C60A..0x0042C612`; `AStar_pathfind_search @ 0x0042C900`, calls `0x0042CB58` and `0x0042CCB3`, JumpJet-infantry row override `0x0042CA43..0x0042CA69`.
- Existing verified reports: `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_TECHNOTYPE_0XD94_COM_PATH_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini` active `MovementZone=` text scan.
- Rust read-only scan: `src/rules/locomotor_type.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_hierarchy.rs`.
