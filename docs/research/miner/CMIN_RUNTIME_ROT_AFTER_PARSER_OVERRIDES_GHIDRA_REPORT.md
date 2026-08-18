# CMIN Runtime ROT After Parser Overrides - Ghidra Research Report

**Address(es):** `0x00712170` (`TechnoTypeClass::ReadINI`), `0x00747620` (`UnitTypeClass::ReadINI`), `0x007353C0` (`UnitClass::Constructor`), consumer context `0x004B0500` / `0x004B04D0`, prior drive-track context `0x004B0F20`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Exact effective runtime `ROT=`/turn-rate value for stock `[CMIN]` after `TechnoTypeClass` and `UnitTypeClass` parser writes, plus the minimum DriveLocomotion/process-drive-track consumer connection needed for a Rust handoff.  
**Non-Scope:** Full drive locomotion state machine, full `Process_Drive_Track` re-analysis, teleport timing, refinery docking protocol, and the complete gameplay meaning of the separate `+0x398` UnitType field.  
**Confidence:** High for parser write order, field offsets, stock CMIN effective `ROT=` value, and constructor/DriveLocomotion consumer field identity. Medium for broader `+0x398` semantics because that field is only identified negatively here.  
**Active in YR:** Yes for stock unit-type parsing and UnitClass construction. Conditional for CMIN DriveLocomotion consumption because stock CMIN normally uses TeleportLocomotion and only reaches DriveLocomotion consumers during piggyback/dock drive phases.

## 0. Investigation Contract

**Target question:** After all relevant parser defaults and overrides, what exact runtime `ROT=`/turn-rate value should stock `[CMIN]` expose to unit/drive facing consumers: INI `5`, harvester override `10`, or some other value?

**Non-goals:** Do not re-investigate the full drive locomotion state machine; do not rediscover settled docking facts; do not modify Rust, INI, or older docs; do not resolve the full gameplay meaning of `UnitTypeClass+0x398` beyond whether it is the `ROT=` consumer field.

**Evidence needed to mark COMPLETE:** INI evidence for `[CMIN] ROT` and `Harvester`; binary evidence for the `ROT` key reader and destination offset; binary evidence for UnitType parser order and harvester/weeder override destination; binary evidence for at least one runtime consumer of the parsed ROT field; a Rust-facing handoff and concrete test name.

**Stop conditions:** Stop after the parser field identity and immediate facing consumers are proven; stop if `+0x398` is shown not to be the `ROT=` consumer field; defer any broader `+0x398` semantic or non-CMIN locomotion paths.

## 1. Overview

Stock `[CMIN]` has `ROT=5` and `Harvester=yes` in `rulesmd.ini`. The effective runtime value of the `ROT=`-parsed facing-rate field is `5`, not `10`. Active in YR: Yes. Evidence: `ini/rulesmd.ini:7364`, `ini/rulesmd.ini:7378`; `TechnoTypeClass::ReadINI @ 0x00714B1B..0x00714B2F`.

The harvester/weeder parser override is real, but it writes `UnitTypeClass+0x398 = 10`, not `TechnoTypeClass+0x71C`, the `ROT=` field consumed by verified UnitClass facing setup and DriveLocomotion ROT refresh. Active in YR: Yes. Evidence: `UnitTypeClass::ReadINI @ 0x00747790..0x007477B1`; `UnitClass::Constructor @ 0x00735570..0x0073558D`; `DriveLocomotionClass::Process @ 0x004B0500`, `DriveLocomotionClass__Update_Facing_From_Type @ 0x004B04D0`.

## 2. Key Offsets

| Offset | Owner context | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x71C` | `TechnoTypeClass` / unit type object | `ROT=`-parsed facing-rate value; stock CMIN stores `5` here | `0x00714B1B` pushes string `0x0081B164` (`ROT`); `0x00714B2F` stores `EAX` to `[EBP+0x71C]` | Yes |
| `+0x398` | `UnitTypeClass`/TechnoType-derived object | Separate int field set to `15`, then `10` for `Harvester`/`Weeder`; not the `ROT=` field | `0x00747790` stores `15`; `0x007477B1` stores `10` after `+0xE0E/+0xE0F` checks | Yes |
| `+0xE0E` | `UnitTypeClass` | `Harvester=` bool | `0x0074769F..0x007476B9` reads string `0x0083D4CC` and stores to `[EDI+0xE0E]` | Yes; stock CMIN sets it |
| `+0xE0F` | `UnitTypeClass` | `Weeder=` bool; shares the `+0x398=10` branch | `0x007476BF..0x007476CD`, branch check `0x007477A7..0x007477AF` | Yes as parser path; not set by stock CMIN |

## 3. Parser Findings

`UnitTypeClass::ReadINI @ 0x00747620` calls `TechnoTypeClass::ReadINI @ 0x00712170` first, before `Harvester=` is read and before the `+0x398` override branch. Active in YR: Yes. Evidence: assembly `0x0074763D MOV ECX,EDI`, `0x0074763F CALL 0x00712170`, `0x00747644 TEST AL,AL`, then UnitType-specific reads.

`TechnoTypeClass::ReadINI` parses `ROT` by pushing string address `0x0081B164`; memory at that address is `52 4F 54 00` (`ROT\0`). It reads the previous default from `[EBP+0x71C]`, calls the integer reader, and writes the returned value to `[EBP+0x71C]`. Active in YR: Yes. Evidence: assembly context `0x00714B14 MOV ECX,[EBP+0x71C]`, `0x00714B1B PUSH 0x81B164`, `0x00714B2A CALL 0x005276D0`, `0x00714B2F MOV [EBP+0x71C],EAX`.

`UnitTypeClass::ReadINI` reads `Harvester=` into `[EDI+0xE0E]`, reads `Weeder=` into `[EDI+0xE0F]`, then later writes `[EDI+0x398]=15`. If either byte is nonzero, it writes `[EDI+0x398]=10`. Active in YR: Yes. Evidence: assembly `0x007476A6 PUSH 0x83D4CC`, `0x007476B9 MOV [EDI+0xE0E],AL`; `0x00747790 MOV [EDI+0x398],0xF`; `0x0074779D..0x007477B1` checks `+0xE0E/+0xE0F` and conditionally stores `0xA`.

Negative parser result: there is no observed UnitType harvester branch write to `+0x71C`. The known `Harvester=yes` override writes `+0x398`, so stock CMIN's `ROT=`-parsed runtime facing-rate remains `5`. Active in YR: Yes. Evidence: same parser range above plus stock `[CMIN] ROT=5` at `ini/rulesmd.ini:7378`.

## 4. Consumer Findings

`UnitClass::Constructor @ 0x007353C0` consumes `type+0x71C` twice to initialize facing-rate trackers, then calls `FUN_004C9680`. Active in YR: Yes for UnitClass instances including CMIN. Evidence: `0x00735570 MOV EAX,[EAX+0x71C]` / `CALL 0x004C9680`; `0x00735584 MOV EDX,[ECX+0x71C]` / `CALL 0x004C9680`.

The facing-rate setter clamps values greater than `0x7E` to `0x7F` and stores the byte value shifted left by 8 at `FacingClass+0x14`. Active in YR: Yes. Evidence: `FUN_004C9680 @ 0x004C9680`: decompile and assembly `0x004C9684 CMP EAX,0x7F`, `0x004C9689 MOV EAX,0x7F`, `0x004C9690 MOV DH,AL`, `0x004C9692 MOV [ECX+0x14],DX`.

DriveLocomotion consumers read the same `type+0x71C` byte through a type pointer returned by the owner virtual. Because Ghidra decompiles the returned pointer as a narrower base, this appears as `iVar2 + 0x11C`; the UnitClass constructor and parser prove the unit-type ROT field is `+0x71C` on the full type object. Active in YR: Conditional for stock CMIN drive phases. Evidence: `DriveLocomotionClass__Update_Facing_From_Type @ 0x004B04D0` calls owner vtable `+0x1BC` then passes `*(byte *)(type + 0x11C)` to vtable `+0x7C`; `DriveLocomotionClass::Process @ 0x004B0500` reads the same offset at tick start. This consumer context is used only when the CMIN is in a DriveLocomotion/piggyback drive phase.

`Process_Drive_Track @ 0x004B0F20` remains outside this slice except for the prior OQ-7 closure: its track-point cadence should not be described as using "CMIN effective ROT=10." Active in YR: Conditional for CMIN drive phases. Evidence: prior report `miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`; parser proof in this report.

## 5. INI Keys

| Section / key | Stock value | Effect in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `[CMIN] ROT` | `5` | Parsed into the effective facing-rate `ROT=` field `+0x71C` | `ini/rulesmd.ini:7378`; `0x00714B1B..0x00714B2F` | Yes |
| `[CMIN] Harvester` | `yes` | Triggers separate `+0x398=10`, not `+0x71C=10` | `ini/rulesmd.ini:7364`; `0x007476A6`; `0x007477B1` | Yes |
| `[CMIN] Locomotor` | Teleport CLSID | Makes DriveLocomotion consumer relevance conditional to piggyback/dock drive phases | `ini/rulesmd.ini:7398`; prior drive-track report | Yes |

## 6. Current Rust Implementation Status

Rust currently models `ObjectType::turret_rot` as the `ROT` value used for body/facing movement, but `ObjectType::from_section` overrides it to `10` whenever `Harvester=yes`. Active Rust surface: `src/rules/object_type.rs:897` through `:905`.

That is a binary mismatch for the `ROT=` field verified here: stock CMIN/HARV should keep `turret_rot = 5` if this Rust field represents `TechnoTypeClass+0x71C`. A distinct field should be introduced only if `UnitTypeClass+0x398` is later needed for NaturalMission/sequence behavior. Active in YR: Yes for parser behavior; Rust delta inferred from source scan, not binary.

Rust miner pivot currently reads `obj.turret_rot` in `dock_pivot_rot_byte` and therefore inherits the parser mismatch. Active Rust surface: `src/sim/miner/miner_dock_sequence.rs:71` through `:75`, `:694` through `:702`, and `:723` through `:742`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `[CMIN] ROT` / `Harvester` stock values | verified | `ini/rulesmd.ini:7364`, `7378` | none |
| `TechnoTypeClass::ReadINI` `ROT=` parse | verified | `0x00714B14..0x00714B2F`, string `0x0081B164` | none |
| `UnitTypeClass::ReadINI` parent-call order | verified | `0x0074763D..0x00747646` | none |
| `UnitTypeClass::ReadINI` `Harvester=` read | verified | `0x0074769F..0x007476B9`, string `0x0083D4CC` | none |
| `UnitTypeClass::ReadINI` `+0x398` override | verified | `0x00747790..0x007477B1` | full semantic of `+0x398` deferred |
| `UnitClass::Constructor` facing-rate source | verified | `0x00735570..0x0073558D`, `0x004C9680` | full facing interpolation math out of scope |
| DriveLocomotion ROT consumer connection | touched-not-exhausted | `0x004B04D0`, `0x004B0500`, prior `0x004B0F20` report | no full state-machine re-trace by design |
| Current Rust parser surface | verified by source scan | `src/rules/object_type.rs:897..905` | implementation change not made in this research slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - What is stock CMIN's declared `ROT` and `Harvester` data? -> `ROT=5`, `Harvester=yes`.` (evidence: `ini/rulesmd.ini:7364`, `7378`)
- `[RESOLVED] OQ-2 - Where does `ROT=` parse for UnitTypes? -> Parent `TechnoTypeClass::ReadINI` reads `ROT` and writes `+0x71C`.` (evidence: `0x00714B1B..0x00714B2F`, string `0x0081B164`)
- `[RESOLVED] OQ-3 - Does `UnitTypeClass::ReadINI` call parent parsing before or after its harvester override? -> Parent parse happens first.` (evidence: `0x0074763D..0x00747646`)
- `[RESOLVED] OQ-4 - Does `Harvester=yes` overwrite the `ROT=` field? -> No; it writes separate `+0x398=10`, not `+0x71C`.` (evidence: `0x00747790..0x007477B1`)
- `[RESOLVED] OQ-5 - Which field does runtime unit facing setup consume? -> `type+0x71C`.` (evidence: `0x00735570`, `0x00735584`, `0x004C9680`)
- `[RESOLVED] OQ-6 - What exact stock CMIN effective `ROT=` value should Rust expose to ROT/facing consumers? -> `5`.` (evidence: `ini/rulesmd.ini:7378`, `0x00714B2F`, `0x00735570..0x0073558D`)
- `[RESOLVED] OQ-7 - How does this close the prior drive-track OQ-7? -> Do not feed "effective ROT=10" into DriveLocomotion/drive-track claims; CMIN `ROT=` remains `5`, while `Process_Drive_Track` cadence remains track-budget/track-point driven.` (evidence: this report plus prior `miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-8 - What is the full player-visible semantic of `+0x398`?` (category: `out-of-scope`; reason: this slice only needed to prove it is not the `ROT=` field; next-step-if-pursued: investigate NaturalMission/sequence readers of `+0x398`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock CMIN effective `ROT=` field remains `5`; `Harvester=yes` does not make it `10` | `ini/rulesmd.ini:7378`; `0x00714B1B..0x00714B2F`; `0x00747790..0x007477B1` | mismatch: Rust overrides harvester `turret_rot` to `10` | `src/rules/object_type.rs` `ObjectType::from_section` | Parse `ROT` into the ROT/facing field without harvester override; reserve `+0x398` for a distinct field only after its semantic is implemented | Loading stock rules gives CMIN and HARV `turret_rot == 5` while any future NaturalMission/aux field can still be `10` | Do not model `UnitTypeClass+0x398=10` as `ROT=10` |
| Unit/facing setup consumes `type+0x71C`, not `+0x398` | `0x00735570`, `0x00735584`, `0x004C9680` | mismatch propagates to locomotor/facing caches because Rust copies `obj.turret_rot` into locomotor `rot` | `src/sim/movement/locomotor.rs`; CMIN spawn/init surfaces | CMIN locomotor/body-facing ROT should use parsed `ROT=5` for stock rules | `test_cmin_parser_rot_effective_value_matches_gamemd` should assert spawned CMIN locomotor/body ROT is `5` | Do not compensate in movement code; fix the parser/source value |
| Miner dock pivot uses the same Rust `turret_rot` source and should not assume harvester ROT=10 | `src/sim/miner/miner_dock_sequence.rs:71..75`; binary parser evidence above | mismatch for pivot duration comments/tests that assume override to `10` | `src/sim/miner/miner_dock_sequence.rs`; `src/sim/miner/miner_tests.rs` | Dock pivot should use stock CMIN/HARV ROT `5` unless a separate proven binary field changes that specific behavior | Acceptance scenario: CMIN dock pivot uses ROT 5 timing; proposed test `test_cmin_parser_rot_effective_value_matches_gamemd` plus a focused dock-pivot timing assertion | Do not preserve stale "Harvester=yes ROT=10" comments as parity claims |

### Stale Docs / Follow-up Docs

- `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`: replace "prior docs report harvester parse override to 10" / OQ-7 wording with: "Parser follow-up verifies stock CMIN `ROT=` remains `5`; the harvester/weeder write to `UnitTypeClass+0x398=10` is not the `ROT=` field consumed by UnitClass/DriveLocomotion facing setup."
- `docs/research/miner/traces/CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md`: replace "writes 10 to TypeClass+0x398 after standard ReadInt" if used as a ROT claim with: "`UnitTypeClass+0x398=10` is a separate harvester/weeder field; stock CMIN effective `ROT=` remains `5`."
- `src/rules/object_type.rs:897`: comment is stale if it claims gamemd forces `ROT=10`; replace with: "gamemd writes a separate UnitType `+0x398=10` for Harvester/Weeder, but `ROT=` remains the parsed `TechnoType+0x71C` value."
- `src/sim/miner/miner_tests.rs:3486`: replace "With Harvester=yes ROT=10 override" with: "Stock harvester `ROT=` remains 5; any `+0x398=10` behavior is not the ROT/facing field."

## Negative Facts / Do Not Do

- Do not implement `Harvester=yes` as an override from `ROT=5` to `ROT=10`. Evidence: `0x007477B1` writes `+0x398`, not `+0x71C`. Active in YR: Yes.
- Do not treat the DriveLocomotion decompiler's `type+0x11C` display as contradicting `+0x71C`; it is a narrower returned type/base view, while the parser and UnitClass constructor prove the full UnitType `ROT=` field is `+0x71C`. Evidence: `0x00714B2F`, `0x00735570`, `0x004B04D0`. Active in YR: Conditional for CMIN drive phases.
- Do not change `Process_Drive_Track` cadence based on "effective ROT=10"; the prior drive-track cadence remains track-budget/track-point driven. Evidence: prior `0x004B0F20` report plus no parser override to `+0x71C`. Active in YR: Conditional.
- Do not delete or ignore the `+0x398=10` write; it is real, but belongs to a separate future field/semantic. Evidence: `0x00747790..0x007477B1`. Active in YR: Yes.
- Do not update stale research docs in-place from this swarm slot; use the replacement wording above unless the parent authorizes a doc patch. Evidence: user scope allowed only this report path. Active in YR: process constraint, not binary behavior.

## Sources

- Ghidra decompiled / assembly context: `TechnoTypeClass::ReadINI @ 0x00712170`, `ROT` string `0x0081B164`, write range `0x00714B14..0x00714B2F`.
- Ghidra decompiled / assembly context: `UnitTypeClass::ReadINI @ 0x00747620`, parent call `0x0074763D..0x00747646`, `Harvester` read `0x0074769F..0x007476B9`, override `0x00747790..0x007477B1`.
- Ghidra decompiled / assembly context: `UnitClass::Constructor @ 0x007353C0`, `+0x71C` reads `0x00735570` and `0x00735584`.
- Ghidra decompiled / assembly context: `FUN_004C9680 @ 0x004C9680`, clamp/shift range `0x004C9680..0x004C9696`.
- Ghidra decompiled: `DriveLocomotionClass__Update_Facing_From_Type @ 0x004B04D0`, `DriveLocomotionClass::Process @ 0x004B0500`.
- INI checked: `ini/rulesmd.ini` `[CMIN]`.
- Prior docs referenced: `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`; `docs/research/CMIN_RUNTIME_ROT_PARSER_OVERRIDE_GHIDRA_REPORT.md`.
