# MovementZone Parser and Numeric Row Mapping -- Ghidra Research Report

**Address(es):** `0x00474E40` (`CCINIClass__ReadMovementZone`), `0x00716065..0x0071608A` (`TechnoTypeClass__ReadINI` consumer), `0x0056C510`, `0x0042C290`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `MovementZone=` string-to-enum parser table, stored numeric values/defaults, row-index relation to the 13x8 passability matrix, and Fly row participation at the matrix/zone-array level.  
**Non-Scope:** matrix contents beyond row-index relation and sentinel risk, exact aircraft/jumpjet runtime pathing, full team compatibility chooser, and exact reduced-zone-type contents.  
**Confidence:** High for parser mapping/default/row relation; Medium for stock-content usage counts from INI text; Partial for runtime Fly locomotor bypass beyond matrix-row existence.  
**Active in YR:** Yes. `TechnoTypeClass__ReadINI` parses the key for normal object type loading, and `Zone_precheck`/zone rebuild paths use the stored field.

## 1. Overview

`MovementZone=` is parsed by a 13-entry string table at `0x0081BA88`; the returned integer is stored directly to `TechnoTypeClass+0x5B4`. The same integer is later used as the row index into the 13x8 `ZonePassabilityMatrix` at `0x0082A594`. There is no SpeedType-derived remap in these verified readers.

The parser is case-insensitive but table-driven. The subterranean spelling in the binary table is `Subterannean`; the common corrected spelling `Subterranean` is not in the table.

## 2. Class Layout / Key Offsets

| Field | Offset | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `MovementZone` | `TechnoTypeClass+0x5B4` (`param_1[0x16D]`) | int | Stored parser result and direct matrix row selector | write at `0x00716081`; ctor default `param_1[0x16D]=0` | Yes |
| `IsSubterranean` derived flag | `TechnoTypeClass+0xD2C` (`param_1[0x34B]`) | bool | Set true only when parser result is `6` | `CMP EAX,0x6` then `SETZ` at `0x0071607E..0x0071608A` | Yes |
| `SpeedType` | `TechnoTypeClass+0x67C` (`param_1[0x19F]`) | int | Separate `SpeedType=` parser result; not this matrix row | `CCINIClass__ReadSpeedType` at `0x007121E0..0x007121E5` in prior matrix report | Yes |

## 3. Core Logic

### 3.1 Parser Algorithm

`CCINIClass__ReadMovementZone(param_1, param_2, default_index)`:

1. Reads a string into a 32-byte local buffer with default `g_MovementZone_NameTable[default_index]`.
2. Scans table pointers from `0x0081BA88` up to, but not including, `0x0081BABC`.
3. Compares the read buffer with each table string using `FUN_007C8D20`.
4. Returns the zero-based table index on equality.
5. Returns `-1` if no table string matches.

`FUN_007C8D20` performs case-insensitive byte comparison; equality returns `0`. Evidence: decompile at `0x007C8D20`, and parser branch returns the current index when compare result is zero at `0x00474E7E..0x00474E99`.

### 3.2 Exact Parser Table

The pointer table is 13 DWORDs from `0x0081BA88` to `0x0081BABC`. Read from `gamemd.exe` image bytes and matched against Ghidra string addresses:

| Value / matrix row | Pointer | Accepted string |
|---:|---:|---|
| 0 | `0x0081BB60` | `Normal` |
| 1 | `0x0081BB58` | `Crusher` |
| 2 | `0x0081BB4C` | `Destroyer` |
| 3 | `0x0081BB38` | `AmphibiousDestroyer` |
| 4 | `0x0081BB24` | `AmphibiousCrusher` |
| 5 | `0x0081BB18` | `Amphibious` |
| 6 | `0x0081BB08` | `Subterannean` |
| 7 | `0x008173D0` | `Infantry` |
| 8 | `0x0081BAF4` | `InfantryDestroyer` |
| 9 | `0x0081BAF0` | `Fly` |
| 10 | `0x0081BAE8` | `Water` |
| 11 | `0x0081BADC` | `WaterBeach` |
| 12 | `0x0081BAD0` | `CrusherAll` |

### 3.3 Defaults and Invalid Values

The constructor initializes `TechnoTypeClass+0x5B4` to `0` (`Normal`) at `TechnoTypeClass__Constructor`, decompile line `param_1[0x16d] = 0`.

During `ReadINI`, the current field value is passed as the parser default. Missing `MovementZone=` therefore preserves the already-current value. For stock type construction this means `Normal`; for patched/inherited reads it means "keep prior value" unless an override parses.

Invalid non-empty strings do not default to `Normal` inside `CCINIClass__ReadMovementZone`; the helper returns `-1`, and `TechnoTypeClass__ReadINI` stores that value directly to `+0x5B4` before setting the subterranean bool false. Evidence: parser `return -1` at `0x00474E8E..0x00474E96`; store at `0x00716081`; `CMP EAX,0x6` at `0x0071607E`.

### 3.4 Row-Index Relation

`Zone_precheck @ 0x0042C290` uses the caller-supplied movement zone as `param_4` and tests `(&g_PassabilityMatrix)[param_4 * 8 + neighbor_zone_type] == 1`. Assembly setup at `0x0042C299..0x0042C2B2` computes row base as `0x82A594 + param_4 * 0x20`.

`MapClass__UpdateBridgeZonesHelper @ 0x0056C510` builds one `MapClass+0x18[row]` zone-id array per matrix row. It frees 13 row pointers, starts at `&g_PassabilityMatrix`, advances by 8 DWORDs per pass, and stops at `0x82A734`, proving all 13 rows including Fly are represented in the global row arrays.

## 4. INI Keys

| Key | Binary parser | Default | Stock YR notes | Active in YR |
|---|---|---|---|---|
| `MovementZone=` | `CCINIClass__ReadMovementZone @ 0x00474E40` | current field value; constructor starts at `Normal`/0 | `rulesmd.ini` active values include `Normal`, `Crusher`, `Destroyer`, `Amphibious`, `AmphibiousDestroyer`, `Infantry`, `InfantryDestroyer`, `Fly`, `Water`, `CrusherAll`; no active stock `Subterannean` or `AmphibiousCrusher` lines found | Yes |
| `SpeedType=` | separate parser | unrelated to MovementZone row | Often paired with water/amphibious/fly content, but not this row selector | Yes, outside this slice |
| `[General] AllowShroudedSubteranneanMoves` | not this parser | true in stock files | spelling confirms Westwood's `Subterannean` typo family but is not `MovementZone=` | Conditional, outside this slice |

Stock `rulesmd.ini` active `MovementZone=` counts from text scan: `Amphibious=3`, `AmphibiousDestroyer=4`, `Crusher=6`, `CrusherAll=1`, `Destroyer=13`, `Fly=19`, `Infantry=58`, `InfantryDestroyer=1`, `Normal=35`, `Water=14`.

## 5. Integration Points

`TechnoTypeClass__ReadINI` parses `MovementZone=` late in the type read at `0x00716065..0x00716081`, stores the result to `+0x5B4`, and immediately derives the subterranean bool by equality to value 6.

`Zone_precheck` and matrix readers consume the stored value as a direct matrix row; prior reports verify direct readers at `0x0042C290`, `0x0056C510`, `0x005840C0`, and `0x005889F0`.

Fly is not absent from the binary row/zone-array model: row 9 is in the parser table and the `0x0056C510` rebuild loop covers it. This report does not prove that every stock Fly locomotor runtime path invokes ground FootClass A*; the prior matrix report correctly leaves that as conditional.

## 6. Current Rust Implementation Status

| Rust surface | Current status |
|---|---|
| `src/rules/locomotor_type.rs` | Numeric discriminants match rows 0..12, including Fly row 9 and CrusherAll row 12. Parser accepts corrected `Subterranean` and `Subterrannean`, but not the binary table spelling `Subterannean`; unknown values warn and default to `Normal` instead of producing/storing `-1`. |
| `src/rules/object_type.rs` | Missing key uses `MovementZone::default()` (`Normal`), matching stock constructor default but not necessarily "preserve prior parsed value" semantics for multi-pass inherited reads. |
| `src/sim/pathfinding/passability.rs` / `zone_build.rs` | Direct `movement_zone as usize` row usage matches the binary row-index contract. Some comments still describe local `LandType` columns rather than reduced binary zone-type columns. |
| `src/sim/pathfinding/zone_map.rs` / `zone_incremental.rs` | `MovementZone::all_ground()` excludes `Fly`; binary `0x0056C510` builds all 13 row arrays including Fly. |
| `src/sim/pathfinding/zone_search.rs` | Reduced precheck currently allows only selected movement zones, including Fly; binary `Zone_precheck` itself accepts whichever row the caller passes. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CCINIClass__ReadMovementZone @ 0x00474E40` | verified | decompile and assembly context | none |
| MovementZone table `0x0081BA88..0x0081BABC` | verified | Ghidra parser addresses plus PE image table read | none |
| `TechnoTypeClass__ReadINI` MovementZone consumer | verified | decompile `0x00716065..0x0071608A` | none |
| Constructor default | verified | `TechnoTypeClass__Constructor`: `param_1[0x16d]=0` | none |
| Invalid-string behavior | verified | parser returns `-1`, caller stores EAX | runtime consequences of later row `-1` are out-of-scope |
| Direct matrix row relation | verified | `Zone_precheck` row base `0x82A594 + param_4*0x20`; prior matrix readers report | none |
| Fly zone-array participation | verified at rebuild-row level | `0x0056C510` loops rows until `0x82A734` | exact Fly locomotor runtime path remains conditional |
| Stock YR active values | verified from repo INI text | `ini/rulesmd.ini` scan | exact expansion after mix overlay order not reloaded in engine |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- What function reads MovementZone? -> `CCINIClass__ReadMovementZone`, called once from `TechnoTypeClass__ReadINI`.` (evidence: string xref `0x008431C8`, call `0x00716079`)
- `[RESOLVED] OQ-2 -- What exact strings map to enum rows? -> 13-entry table listed in Section 3.2.` (evidence: table `0x0081BA88..0x0081BABC`)
- `[RESOLVED] OQ-3 -- Is matching case-sensitive? -> No, comparison helper folds ASCII/case-equivalent bytes.` (evidence: `FUN_007C8D20`)
- `[RESOLVED] OQ-4 -- What is the default when key is missing? -> Current stored value; constructor value is row 0 Normal.` (evidence: parser default table lookup; ctor write)
- `[RESOLVED] OQ-5 -- What happens on unknown string? -> parser returns `-1`, caller stores it.` (evidence: `0x00474E8E..0x00474E96`; `0x00716081`)
- `[RESOLVED] OQ-6 -- Does Subterranean parse by corrected spelling? -> Not by table evidence; binary string is `Subterannean`.` (evidence: `0x0081BB08`)
- `[RESOLVED] OQ-7 -- Is Fly row 9 in the parser? -> Yes.` (evidence: table pointer `0x0081BAF0` at index 9)
- `[RESOLVED] OQ-8 -- Does row number directly index matrix readers? -> Yes, direct `row*8` / `row*0x20` indexing.` (evidence: `0x0042C299..0x0042C2B2`, `0x0042C60A..0x0042C612`)
- `[RESOLVED] OQ-9 -- Does binary rebuild include Fly row arrays? -> Yes, all 13 rows are built by `0x0056C510`.` (evidence: row loop to `0x82A734`)
- `[RESOLVED] OQ-10 -- Are Amphibious variants distinct parser rows? -> Yes, rows 3/4/5 are `AmphibiousDestroyer`, `AmphibiousCrusher`, `Amphibious`.` (evidence: table entries)
- `[RESOLVED] OQ-11 -- Does stock YR use all scoped variants? -> Stock text uses Fly, CrusherAll, InfantryDestroyer, Amphibious, AmphibiousDestroyer; no active `Subterannean` or `AmphibiousCrusher` line found.` (evidence: `ini/rulesmd.ini` scan)
- `[DEFERRED] OQ-12 -- Do all stock Fly locomotor movement orders enter FootClass A*?` (category: out-of-scope; reason: target is parser/row mapping; next-step-if-pursued: trace Jumpjet/FlyLocomotion move command into path request)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Binary parser table accepts `Subterannean` row 6, not the corrected spelling by table evidence | `0x0081BB08`; parser `0x00474E40` | mismatch: Rust accepts `Subterranean`/`Subterrannean`, not `Subterannean` | `src/rules/locomotor_type.rs::MovementZone::from_ini` | Add the binary typo as the canonical accepted spelling; decide whether to keep forgiving aliases as non-binary extensions | `MovementZone=Subterannean` parses to row 6 and sets subterranean movement behavior | Do not make `Subterranean` the only accepted spelling |
| Unknown MovementZone strings return and store `-1`; missing key defaults through current value, constructor value 0 | `0x00474E8E..0x00474E96`, `0x00716081`, ctor `param_1[0x16d]=0` | mismatch: Rust unknown defaults to `Normal`; missing key always default-normal in object parser | `src/rules/locomotor_type.rs`, `src/rules/object_type.rs` | Preserve stock missing-key Normal for fresh objects, but add tests/documentation for unknown-string behavior before choosing whether to model invalid row | Invalid value does not silently become row 0 in a parser parity test | Do not hide invalid content as Normal if later parser parity requires exact `-1` behavior |
| Binary creates row arrays for all 13 movement-zone rows, including Fly row 9; row users directly index by MovementZone | `0x0056C510` loop to `0x82A734`; `0x0042C290` row math | mismatch/intentional shortcut: Rust `all_ground()` excludes Fly and `ZoneGrid::can_reach(Fly)` returns true | `src/rules/locomotor_type.rs::all_ground`, `src/sim/pathfinding/zone_map.rs`, `zone_search.rs` | Decide explicitly whether Fly row 9 gets built or remains a verified shortcut; if shortcut remains, preserve sentinel/out-of-playfield blocking in cell legality | Fly row 9 is not treated as "all cells including OOB" and tests document why no Fly map exists | Do not describe Fly as absent from binary zone maps |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/ZONE_PASSABILITY_VERIFIED.md`: replace any wording that says `MovementZone=Subterranean` is the binary parser string with: "`CCINIClass__ReadMovementZone` accepts the Westwood table spelling `Subterannean` for row 6; `TechnoTypeClass__ReadINI` stores row 6 to `+0x5B4` and sets the derived subterranean bool by `value == 6`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/ZONE_PASSABILITY_VERIFIED.md`: replace row-9 wording "Fly passes everything except rock/OoB sentinel" with: "Fly is parser/matrix row 9. Prior matrix-reader evidence shows row 9 participates in the 13-row matrix and row-array rebuild; exact runtime use by stock aircraft/jumpjet locomotors is conditional and requires a separate locomotor trace."
- `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: if MovementZone parser behavior is summarized, add: "`AStar_pathfind_search` consumes the stored numeric `MovementZone` row; parser defaults and invalid-string behavior are owned by `CCINIClass__ReadMovementZone @ 0x00474E40`, not by A*."

## Negative Facts / Do Not Do

- Do not map MovementZone rows from `SpeedType`; direct readers consume `TechnoTypeClass+0x5B4` as row. Evidence: `0x00716081`, `0x0042C290`.
- Do not assume the binary accepts only corrected `Subterranean`; the table string is `Subterannean`. Evidence: table entry pointer `0x0081BB08`.
- Do not say unknown MovementZone strings default to Normal in the binary helper; no-match returns `-1`. Evidence: `0x00474E8E..0x00474E96`.
- Do not say Fly is absent from binary zone arrays; `0x0056C510` builds all 13 rows including row 9. Evidence: loop until `0x82A734`.
- Do not collapse Amphibious rows; `AmphibiousDestroyer`, `AmphibiousCrusher`, and `Amphibious` are separate parser values 3/4/5. Evidence: table entries `0x0081BB38`, `0x0081BB24`, `0x0081BB18`.

## Remaining Uncertainty

- Exact stock Fly/Jumpjet/FlyLocomotion runtime path into, or around, FootClass A* remains out-of-scope. The parser row and zone-array existence are verified; whether every Fly move uses those arrays is not.
- Runtime consequences of an invalid stored row `-1` are not traced. The parser/store behavior is verified, but later negative-index safety/failure mode was outside this slice.

## Sources

- Ghidra: `CCINIClass__ReadMovementZone @ 0x00474E40`; `FUN_007C8D20 @ 0x007C8D20`; `TechnoTypeClass__ReadINI @ 0x00716065..0x0071608A`; `TechnoTypeClass__Constructor`; `Zone_precheck @ 0x0042C290`; `MapClass__UpdateBridgeZonesHelper @ 0x0056C510`.
- Binary image bytes: `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, table `0x0081BA88..0x0081BABC`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `ini/rules.ini`.
- Prior reports: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`.
