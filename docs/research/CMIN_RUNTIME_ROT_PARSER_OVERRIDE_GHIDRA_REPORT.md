# CMIN Runtime ROT Parser Override - Ghidra Research Report

**Address(es):** `0x00712170` (`TechnoTypeClass::ReadINI`), `0x00747620` (`UnitTypeClass::ReadINI`), `0x007353C0` (`UnitClass::Constructor`), `0x004B0F20` prior drive-track report context  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Stock `[CMIN]` runtime ROT value after `TechnoTypeClass` parsing and `UnitTypeClass` harvester/weeder override writes, and the effect on chrono-miner drive-track/turn-cadence claims.  
**Non-Scope:** Full facing interpolation math, full `+0x398` field identity outside the parser/constructor evidence, teleport locomotion timing, and Rust implementation changes.  
**Confidence:** High for the parser write order and the field used by unit facing setup; Medium for negative statements about all possible `+0x398` runtime readers because this pass only used targeted byte-pattern and nearby call-chain searches.  
**Active in YR:** Yes for stock YR unit-type loading and UnitClass construction; Conditional for chrono-miner DriveLocomotion drive-track effects, because stock CMIN uses TeleportLocomotion and only uses drive-track during verified piggyback/dock drive phases.

## 1. Overview

Stock `[CMIN]` has `ROT=5` and `Harvester=yes` in `rulesmd.ini`. The binary does not overwrite the `ROT=`-parsed field to 10. `TechnoTypeClass::ReadINI` parses the `ROT` string into `TechnoTypeClass+0x71C`, and `UnitClass::Constructor` uses that `+0x71C` value to initialize the unit's facing-rate trackers.

The harvester/weeder override does exist, but it writes `UnitTypeClass+0x398`: `15` first, then `10` when `Harvester=yes` or `Weeder=yes`. That write is after `Harvester=` parsing, but it is not the `+0x71C` field consumed by the verified UnitClass facing setup or by the prior drive-track slice.

## 2. Key Offsets / Fields

| Offset | Owner context | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x71C` | `TechnoTypeClass` / unit type object | `ROT=`-parsed facing-rate value; stock CMIN stores `5` here | `TechnoTypeClass::ReadINI @ 0x00714B1B` xref to string `ROT` at `0x0081B164`; `UnitClass::Constructor @ 0x00735570/0x00735584` reads `type+0x71C` | Yes; normal unit type parse and construction path |
| `+0x398` | `TechnoTypeClass`/unit type object, exact semantic outside this slice | Constructor default `15`; `UnitTypeClass::ReadINI` writes `15`, then `10` if `Harvester` or `Weeder` is set | Constructor store `0x00710CAB`; parser stores `0x00747790` and `0x007477B1` | Yes; normal unit type parse path |
| `+0xE0E` | `UnitTypeClass` | `Harvester=` bool | `UnitTypeClass::ReadINI @ 0x007476A6` xref to string `Harvester` at `0x0083D4CC` | Yes; stock `[CMIN] Harvester=yes` |
| `+0xE0F` | `UnitTypeClass` | `Weeder=` bool; shares the `+0x398 = 10` override branch | `UnitTypeClass::ReadINI @ 0x007476BF..0x007477B1` | Yes as parser code; stock CMIN does not set `Weeder=yes` |

## 3. Core Logic

### 3.1 `ROT=` parse

`TechnoTypeClass::ReadINI @ 0x00712170` reads the literal `ROT` key from the unit's INI section. The material xref is `0x00714B1B -> 0x0081B164` and the resulting integer is stored at `this+0x71C`.

Active in YR: Yes. Evidence: this is the standard `TechnoTypeClass::ReadINI` path called by `UnitTypeClass::ReadINI @ 0x0074763D..0x0074763F`; stock `[CMIN] ROT=5` is in `ini/rulesmd.ini:7378`.

### 3.2 Harvester/weeder override write

`UnitTypeClass::ReadINI @ 0x00747620` calls `TechnoTypeClass::ReadINI` first. After that, it reads `CrateGoodie`, `DeployToFire`, `IsSimpleDeployer`, then `Harvester` and `Weeder`. Later it writes `this+0x398 = 15`; if `this+0xE0E` (`Harvester`) or `this+0xE0F` (`Weeder`) is nonzero, it writes `this+0x398 = 10`.

Active in YR: Yes. Evidence: `Harvester` xref `0x007476A6`, `Weeder` read immediately after it, default store `0x00747790`, conditional override store `0x007477B1`; stock `[CMIN] Harvester=yes` is `ini/rulesmd.ini:7364`.

This is a real harvester override, but it is not a write to `this+0x71C`. Therefore the statement "CMIN INI `ROT=5` is overwritten to 10" is false for the `ROT=`-parsed facing field verified in this slice.

### 3.3 Runtime facing setup uses `+0x71C`

`UnitClass::Constructor @ 0x007353C0` reads `unit_type+0x71C` and passes it twice into the small facing-rate setter at `0x004C9680`. The setter clamps values above `0x7F` and stores `(byte)value << 8` into a `FacingClass` field.

Active in YR: Yes. Evidence: `UnitClass::Constructor @ 0x00735570` and `0x00735584` read `param_1[0x1B1]+0x71C`; `0x004C9680` performs the clamp-and-shift. CMIN is a normal UnitClass instance created from the parsed UnitType.

For stock CMIN, the constructor-facing setup therefore receives `5`, not `10`, from the verified `ROT=` path.

### 3.4 Drive-track cadence implication

The prior drive-track report remains correct on the core cadence point: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` does not read the `ROT=` field and advances facing from consumed track-point headings. The parser finding here only corrects the CMIN-specific runtime ROT assumption in adjacent docs.

Active in YR: Conditional. Evidence: `DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md` verifies `0x004B0F20` for DriveLocomotion phases; stock CMIN uses TeleportLocomotion at `ini/rulesmd.ini:7398`, so drive-track applies only during piggyback/dock drive phases.

The claim "CMIN drive-track/turn cadence uses effective ROT=10" should not be used. For drive-track curves, cadence is governed by track budget and point consumption; for constructor-facing rate, stock CMIN's verified `ROT=` field is `5`.

## 4. INI Keys

| Section / key | Stock value | Effect in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `[CMIN] ROT` | `5` | Parsed by `TechnoTypeClass::ReadINI` into `+0x71C`; used by UnitClass facing-rate setup | `ini/rulesmd.ini:7378`, `0x00714B1B`, `0x00735570/0x00735584` | Yes |
| `[CMIN] Harvester` | `yes` | Triggers `UnitTypeClass+0x398 = 10`, not `+0x71C = 10` | `ini/rulesmd.ini:7364`, `0x007476A6`, `0x007477B1` | Yes |
| `[CMIN] Locomotor` | Teleport CLSID | Makes drive-track impact conditional to piggyback/dock drive phases | `ini/rulesmd.ini:7398`, prior drive-track report | Yes |

## 5. Integration Points

`UnitTypeClass::ReadINI @ 0x00747620` is the immediate parser wrapper for vehicle/unit types. It calls `TechnoTypeClass::ReadINI @ 0x00712170` before reading `Harvester=` and before the `+0x398` harvester/weeder override.

`UnitClass::Constructor @ 0x007353C0` initializes the live UnitClass facing trackers from `unit_type+0x71C`. This ties the `ROT=` parse to a runtime object and resolves the material "which field is used?" question for normal unit-facing setup.

`DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` is not re-covered here. Its prior report's OQ-7 should be closed by this report: stock CMIN's `ROT=`-parsed field remains `5`; the harvester override-to-10 write exists but targets a different offset.

## 6. Current Rust Implementation Status

Not audited and not modified in this slot. Existing trace text that says "ROT=10 IS applied at parse time for Harvester=yes" should be treated as stale for gamemd parity unless it is explicitly referring to Rust's own model rather than the verified binary field.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `[CMIN]` INI values | verified | `ini/rulesmd.ini:7364`, `7378`, `7398` | none for this slice |
| `ROT` string xref | verified | string `0x0081B164`; xrefs `0x00714B1B` and bullet parser `0x0046BF2E` | none for unit ROT parse |
| `TechnoTypeClass::ReadINI` `ROT=` write | verified | `0x00714B1B..0x00714B2F`, store to `+0x71C` | none |
| `UnitTypeClass::ReadINI` `Harvester=` read | verified | `0x007476A6`, store to `+0xE0E` | none |
| `UnitTypeClass::ReadINI` harvester/weeder override | verified | `0x00747790` default `+0x398=15`; `0x007477B1` conditional `+0x398=10` | exact gameplay meaning of `+0x398` outside this parser slice |
| `UnitClass::Constructor` facing-rate source | verified | `0x00735570`, `0x00735584`, callee `0x004C9680` | full facing interpolation math remains outside scope |
| Drive-track cadence effect | verified for no ROT override dependency | prior `0x004B0F20` report plus parser findings here | no re-trace of full drive-track body in this slot |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does stock `[CMIN]` define `ROT=5` and `Harvester=yes`? Yes. Evidence: `ini/rulesmd.ini:7364`, `7378`.

[RESOLVED] OQ-2 - Where is `ROT=` parsed for unit types? `TechnoTypeClass::ReadINI @ 0x00714B1B` reads string `0x0081B164` and stores the result to `+0x71C`. Active in YR: Yes.

[RESOLVED] OQ-3 - Does the UnitType harvester branch overwrite the `ROT=`-parsed `+0x71C` field? No. It writes `+0x398`, not `+0x71C`. Active in YR: Yes.

[RESOLVED] OQ-4 - Is there a real write of `10` for harvesters? Yes. `UnitTypeClass::ReadINI @ 0x007477B1` writes `+0x398 = 10` if `Harvester` or `Weeder` is true. Active in YR: Yes.

[RESOLVED] OQ-5 - Which verified field does UnitClass facing setup consume? `UnitClass::Constructor` consumes `type+0x71C`, then `0x004C9680` clamps and shifts it into a facing-rate field. Active in YR: Yes.

[RESOLVED] OQ-6 - Does this change the prior drive-track cadence finding? It corrects only the adjacent CMIN-specific "effective ROT=10" assumption. `Process_Drive_Track @ 0x004B0F20` cadence remains track-budget/track-point driven and not a direct ROT formula. Active in YR: Conditional for CMIN drive phases.

[DEFERRED] OQ-7 - What is the full gameplay semantic of `TechnoTypeClass/UnitTypeClass+0x398`? Category: out-of-scope. This report only proves it is not the `ROT=`-parsed field consumed by verified UnitClass facing setup.

## Sources

- Ghidra decompiled / assembly context: `TechnoTypeClass::ReadINI @ 0x00712170`, `ROT` xref at `0x00714B1B`, string `0x0081B164`.
- Ghidra decompiled / assembly context: `UnitTypeClass::ReadINI @ 0x00747620`, `Harvester` xref `0x007476A6`, default store `0x00747790`, override store `0x007477B1`.
- Ghidra decompiled / assembly context: `TechnoTypeClass::Constructor @ 0x00710AF0`, default store `+0x398=15` at `0x00710CAB`.
- Ghidra decompiled: `UnitClass::Constructor @ 0x007353C0`, `+0x71C` reads at `0x00735570` and `0x00735584`.
- Ghidra decompiled: `FUN_004C9680 @ 0x004C9680`, facing-rate clamp/shift setter.
- INI checked: `ini/rulesmd.ini` `[CMIN]` lines `7364`, `7378`, `7398`.
- Prior report referenced: `DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md` OQ-7.
