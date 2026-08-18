# TechnoClass Set_Destination Preprocessing Flags -- Ghidra Research Report

**Address(es):** `0x00741970` (UnitClass destination preprocessing; Ghidra label `TechnoClass__Set_Destination`), `0x00713FE9`, `0x00714CCF`, `0x00714CF0`, `0x00714D11`, `0x00714D95`, `0x00716065..0x0071608A`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Field identity and branch effects for the offsets named by `NAVCOM_LIFECYCLE` OQ-6: `TechnoTypeClass+0xD6A`, alleged `+0xD28/+0xD29/+0xD2A` in the top stop guard, and `TechnoTypeClass+0xD2C` as consumed by `0x00741970` before the final `FootClass::Set_Destination_Internal` call.
**Non-Scope:** Full dock/radio flow, full `FootClass::Set_Destination_Internal`, `FUN_007447B0` internals, locomotor COM method naming, and the semantic names for Unit/Foot instance bytes `+0x6E0..+0x6E2`.
**Confidence:** High for field identities and read/write addresses; High that `+0xD28/+0xD29/+0xD2A` are not read in the alleged stop guard; Medium-High for the `+0xD2C` branch effect summary because helper internals are intentionally out of scope.
**Active in YR:** Conditional. `0x00741970` is active for UnitClass destination assignments; `+0xD6A` is active for UnitClass types with `BalloonHover=yes`; `+0xD2C` is live code but stock `rulesmd.ini` has no active `MovementZone=Subterannean` UnitClass type found in prior movement-zone audit; `+0xD28/+0xD29/+0xD2A` are parsed and active in crush systems but not consumed as TypeClass reads by this preprocessing stop guard.

## Working Notes Gate

- Target question: Which TypeClass fields behind `+0xD6A`, `+0xD28`, `+0xD29`, `+0xD2A`, and `+0xD2C` actually drive `0x00741970` destination preprocessing, and how do they alter the destination before the final setter?
- Non-goals: Do not re-investigate `FootClass::Set_Destination_Internal`, dock/radio, full teleporter/hover locomotor swapping, or helper internals beyond proving whether `param_2` is rewritten.
- Evidence needed to mark COMPLETE: Binary reads in `0x00741970`, parser/source identity for each offset, final-call evidence, Rust-facing deltas, and stale-doc replacement wording.
- Stop conditions: Stop after all named offsets are either resolved as consumed with branch effects or proven not consumed by the alleged branch; defer helper internals and unrelated flags.

## 1. Overview

The previous navcom open question bundled three different things: real TechnoType fields, decompiler array-index artifacts, and live destination rewrite logic. Fresh binary inspection separates them. `TechnoTypeClass+0xD6A` is `BalloonHover=`, `TechnoTypeClass+0xD2C` is the derived `MovementZone == 6` (`Subterannean`) byte, and `TechnoTypeClass+0xD28/+0xD29/+0xD2A` are the crush hierarchy flags. However, the `0x00741970` early stop guard does **not** read TypeClass `+0xD28/+0xD29/+0xD2A`; it reads Unit/Foot instance bytes at `this+0x6E0/+0x6E1/+0x6E2`.

## 2. Class Layout / Key Offsets

| Field | Offset | Type | Identity | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `BalloonHover` | `TechnoTypeClass+0xD6A` | byte bool | INI `BalloonHover=` | string `0x00843838`; xref `0x00714D95`; assembly writes `[EBP+0xD6A]` at `0x00714DA9`; `0x00741983..0x00741991` reads `[this+0x6C4]+0xD6A` | Yes, conditional by UnitClass type data; stock `DISK` has `BalloonHover=yes` |
| `Crusher` | `TechnoTypeClass+0xD28` | byte bool | INI `Crusher=` | string `0x0081BB58`; xref `0x00714CCF`; assembly writes `[EBP+0xD28]` at `0x00714CE3` | Yes in crush/path systems; not read in the alleged `0x00741970` stop guard |
| `OmniCrusher` | `TechnoTypeClass+0xD29` | byte bool | INI `OmniCrusher=` | string `0x0084387C`; xref `0x00714CF0`; assembly writes `[EBP+0xD29]` at `0x00714D04` | Yes in crush systems; not read in the alleged `0x00741970` stop guard |
| `OmniCrushResistant` | `TechnoTypeClass+0xD2A` | byte bool | INI `OmniCrushResistant=` | string `0x00843868`; xref `0x00714D11`; assembly writes `[EBP+0xD2A]` at `0x00714D25` | Yes in crush systems; not read in the alleged `0x00741970` stop guard |
| derived `IsSubterranean` | `TechnoTypeClass+0xD2C` | byte bool | `MovementZone == 6`, where row 6 is binary spelling `Subterannean` | `MovementZone` string xref `0x00716065`; `ReadMovementZone` call `0x00716079`; `CMP EAX,0x6`, store `+0x5B4`, `SETZ`, write `[EBP+0xD2C]` at `0x0071607E..0x0071608A`; reads in `0x00741970` at `0x00741EA1` and `0x0074209D` | Conditional; live engine path, no active stock `rulesmd.ini` `Subterannean` type found by prior audit |
| stop-guard byte A | `Unit/Foot instance +0x6E0` | byte | unresolved instance movement-state flag, **not** `TechnoType+0xD28` | `0x00741A96 MOV AL,[EBP+0x6E0]` | Yes, conditional |
| stop-guard byte B | `Unit/Foot instance +0x6E1` | byte | unresolved instance movement-state flag, **not** `TechnoType+0xD29` | `0x00741AA7 MOV CL,[EBP+0x6E1]` | Yes, conditional |
| stop-guard byte C | `Unit/Foot instance +0x6E2` | byte | unresolved instance movement-state flag, **not** `TechnoType+0xD2A` | `0x00741AB1 MOV CL,[EBP+0x6E2]` | Yes, conditional |

## 3. Core Logic

### 3.1 `+0xD6A` / `BalloonHover=` null-destination intercept

At the top of `0x00741970`, the function loads `this+0x6C4` as the type pointer, then reads byte `type+0xD6A`. If `BalloonHover` is false, or the requested destination is non-null, the block is skipped.

When active, all of these must also be true: requested destination is null, current `NavCom` at `this+0x5A4` is non-null, and instance field `this+0x2B4` is non-null. The block then compares `NavCom` directly to `this+0x2B4`, or if `NavCom` has RTTI `0x0B`, compares `NavCom+0xE4` to `this+0x2B4`; either match reaches the early return/mission-clear path. If those direct tests fail, it computes a timer delta through `RateTimer__Current`, `0x005F3DB0`, and `0x004D03D0`; a nonzero helper result reaches the same path.

Effect on destination: this branch never rewrites `param_2` to another target and never reaches the final `FootClass::Set_Destination_Internal` call. It consumes a null-destination request by either returning immediately when vtable `+0x184` returns mission `1`, or calling vtable `+0x1F0` with argument `1` and returning. Active in YR: Conditional; stock UnitClass `DISK` has `BalloonHover=yes`, and `0x00741970` is the UnitClass destination entry, but the branch additionally requires the runtime `NavCom/+0x2B4` state.

### 3.2 Alleged `+0xD28/+0xD29/+0xD2A` stop guard is not a TypeClass read

The stop guard immediately after the same-destination early return clears `this+0x1F8`, then reads:

- `this+0x6E0` at `0x00741A96`
- `this+0x6E1` at `0x00741AA7`
- `this+0x6E2` at `0x00741AB1`
- `this+0x2B0` at `0x00741AC3`

Effect on destination: if `+0x6E0 == 0` and either `+0x6E1` or `+0x6E2` is nonzero, the function calls `FootClass::Stop_Moving @ 0x004DF0D0` and returns. If `+0x6E0 != 0` but `this+0x2B0` is null, it also calls `Stop_Moving` and returns. No new destination is committed and no TypeClass crush flag participates in this branch.

This resolves the NAVCOM_LIFECYCLE OQ-6 confusion: the decompiler expression names looked like `param_1[10].field_0x28/0x29/0x2A`, but assembly proves those are byte offsets from `this`, not reads through the type pointer at `this+0x6C4`.

Active in YR: Yes, conditional. The branch is on the active UnitClass destination path. Exact semantic names for instance bytes `+0x6E0..+0x6E2` are outside this slot.

### 3.3 `+0xD2C` / derived `MovementZone == Subterannean` destination preprocessing

`TechnoTypeClass+0xD2C` is set only by the MovementZone parser: read `MovementZone=`, store row to `+0x5B4`, then set `+0xD2C = (row == 6)`. The row-6 accepted string is `Subterannean`, preserving Westwood's spelling.

`0x00741970` reads `type+0xD2C` twice:

1. First branch at `0x00741E8E..0x0074208A`: if `param_2` is non-null and `type+0xD2C` is true, it checks the active locomotor pointer at `this+0x674`, calls locomotor slot `+0x10`, queries `IPiggyback`, compares the current class ID to `CLSID_DriveLocomotion`, and only then enters the Drive-specific rewrite/queue path. When the Drive predicate passes, the original requested destination can be inserted into the dynamic vector rooted at `this+0x5AC`, another vector rooted at `this+0x588` is cleared, the function asks self vtable `+0x1C4` for a cell destination, and if that cell is a bridge cell (`cell+0x140 & 0x100`) and the unit is not in the vtable `+0xBC` state, it calls `FootClass__Find_Nearby_Passable_Cell` using `type+0x5B4` MovementZone. If a valid cell is returned, `param_2` is replaced with `MapClass__Get_CellClass(returned_cell)`.
2. Second branch at `0x0074208A..0x007422F7`: if `param_2` is non-null, `type+0xD2C` is true, and the force flag (`param_3`) is nonzero, it runs current-cell/destination-cell bridge and obstacle tests. If the combined tests allow correction, it calls `FUN_007447B0` and assigns its return value to `param_2`, then continues to the later queue/current-cell block. If the tests reject correction, it falls through without the `FUN_007447B0` rewrite.

Effect on destination: `+0xD2C` is the only named OQ-6 field that can actually replace `param_2` with a different `CellClass*` in this slice. The replacement is subterranean/bridge/passability-specific and uses the stored `MovementZone` row. Active in YR: Conditional; code is live in the active UnitClass function, but prior MovementZone audit found no active stock `rulesmd.ini` `MovementZone=Subterannean` lines.

### 3.4 Final call evidence

The normal tail call to `FootClass::Set_Destination_Internal @ 0x004D94B0` occurs at `0x0074315F..0x00743161`, with the current `param_2` value after all preprocessing. A special UnitRepair/UnitReload accepted path also calls the same internal setter at `0x00742D09..0x00742D0B`, then writes `this+0x5E0 = -1` and returns. This report does not re-open the internal setter semantics.

## 4. INI Keys

| Key | Binary reader | Stored field | Default/source | Effect in this slice | Active in YR |
|---|---|---:|---|---|---|
| `BalloonHover=` | `TechnoTypeClass::ReadINI @ 0x00714D95` | `+0xD6A` | existing field passed as default; constructor default zero per prior TechnoType base docs | Gates null-destination intercept before same-destination/stop guards | Yes, conditional; stock `DISK`, `JUMPJET`, `LUNR`, `ZEP` examples set it, but this function is UnitClass-specific |
| `Crusher=` | `0x00714CCF` | `+0xD28` | default false | No direct effect in `0x00741970` stop guard; active elsewhere for crushing/pathing | Yes elsewhere; not this branch |
| `OmniCrusher=` | `0x00714CF0` | `+0xD29` | default false | No direct effect in `0x00741970` stop guard; active elsewhere for crush override | Yes elsewhere; not this branch |
| `OmniCrushResistant=` | `0x00714D11` | `+0xD2A` | default false | No direct effect in `0x00741970` stop guard; active elsewhere as target resistance | Yes elsewhere; not this branch |
| `MovementZone=` | `CCINIClass__ReadMovementZone @ 0x00474E40`, consumed at `0x00716065..0x0071608A` | `+0x5B4` and derived `+0xD2C` | missing key preserves current field; constructor default `Normal`/0; invalid string stores `-1` per prior audit | `+0xD2C` enables subterranean destination queue/rewrite blocks | Conditional; stock no active `Subterannean` rows found by prior audit |

## 5. Integration Points

`0x00741970` is a one-shot UnitClass destination preprocessing entry. For this slice, the relevant integration is:

- Input: caller supplies `param_2` destination pointer and force flag `param_3`.
- Early null path: `BalloonHover` can consume a null destination before the internal setter.
- Stop path: instance bytes `+0x6E0..+0x6E2`, not TypeClass crush bytes, can call `Stop_Moving` and return.
- Subterranean path: `type+0xD2C` can queue the original destination and/or replace `param_2` with a passable `CellClass*`.
- Output: the current `param_2` reaches `FootClass::Set_Destination_Internal` at `0x0074315F`, unless an early return path consumed it.

## 6. Current Rust Implementation Status

Rust currently has the individual parsed type fields: `teleporter`, `balloon_hover`, `crusher`, `omni_crusher`, `omni_crush_resistant`, and `movement_zone` in `src/rules/object_type.rs`. It also has a `MovementZone::Subterranean` enum that accepts the binary spelling `Subterannean` in `src/rules/locomotor_type.rs`.

The movement issuing path in `src/sim/movement/movement_commands.rs::issue_move_command_with_layered` currently resolves blocked goals generically, computes a path, attaches `MovementTarget`, and for Drive locomotors calls `navcom::set_destination_internal_cell` then clears `navigation.nav_queue`. It has a manual queue append path, but no `TechnoClass::Set_Destination` preprocessing equivalent for `BalloonHover` null-destination interception or the `MovementZone==Subterannean` original-destination queue/passable-cell rewrite. `src/sim/movement/navcom.rs` models only the owner destination/null path for the normal cell-target slice.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00741970` type-pointer reads | verified | decompile plus assembly `0x00741983..0x00741991`, `0x00741E9B..0x00741EA9`, `0x00742097..0x007420A5` | helper names only |
| `+0xD6A` identity | verified | string `0x00843838`, xref `0x00714D95`, write `0x00714DA9`, read `0x00741989` | exact meaning of runtime `this+0x2B4` relation |
| alleged `+0xD28/+0xD29/+0xD2A` stop guard | verified-negative | parser writes at `0x00714CE3/0x00714D04/0x00714D25`; stop guard reads `this+0x6E0/+0x6E1/+0x6E2` at `0x00741A96..0x00741AB1` | semantic names of `+0x6E0..+0x6E2` |
| `+0xD2C` identity | verified | `0x00716065..0x0071608A`; prior MovementZone parser table `0x0081BA88..0x0081BABC` | none for identity |
| `+0xD2C` branch 1 destination effect | verified | `0x00741E8E..0x0074208A` decompile/disassembly | exact names of vectors at `+0x5AC` and `+0x588` |
| `+0xD2C` branch 2 destination effect | touched-not-exhausted | `0x0074208A..0x007422F7` decompile/disassembly | `FUN_007447B0` internals intentionally out-of-scope |
| final internal setter call | verified | `0x0074315F..0x00743161`; special path `0x00742D09..0x00742D0B` | internal setter behavior covered by newer navcom reports |
| Rust surface scan | verified | `src/rules/object_type.rs`, `src/rules/locomotor_type.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/navcom.rs` | no code changes in this slot |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Is `0x00741970` the active target function? -> Yes for UnitClass destination preprocessing; prior vtable docs and fresh decompile/disassembly identify the body.` (evidence: `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`; `0x00741970`)
- `[RESOLVED] OQ-02 -- What is `TechnoType+0xD6A`? -> `BalloonHover=`, parsed at `0x00714D95` and read at `0x00741989`.` (evidence: `0x00843838`, `0x00714D95`, `0x00741983..0x00741991`)
- `[RESOLVED] OQ-03 -- What are `TechnoType+0xD28/+0xD29/+0xD2A`? -> `Crusher=`, `OmniCrusher=`, `OmniCrushResistant=`, respectively.` (evidence: `0x00714CCF`, `0x00714CF0`, `0x00714D11`; `CRUSH_SYSTEM_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-04 -- Are `TechnoType+0xD28/+0xD29/+0xD2A` read by the top stop guard in `0x00741970`? -> No; the guard reads instance bytes `this+0x6E0/+0x6E1/+0x6E2`.` (evidence: `0x00741A96..0x00741AB1`)
- `[RESOLVED] OQ-05 -- What is `TechnoType+0xD2C`? -> Derived bool set to true only when parsed `MovementZone` row equals 6 (`Subterannean`).` (evidence: `0x00716065..0x0071608A`; `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-06 -- Does `+0xD6A` rewrite the destination? -> No; it can consume a null destination by returning before the internal setter.` (evidence: `0x00741983..0x00741A7D`)
- `[RESOLVED] OQ-07 -- Does `+0xD2C` rewrite the destination? -> Yes conditionally; it can replace `param_2` with a passable/current-cell-derived `CellClass*`.` (evidence: `0x00741E8E..0x0074208A`, `0x0074208A..0x007422F7`)
- `[RESOLVED] OQ-08 -- Is `Teleporter=` one of these OQ-6 offsets? -> No; `Teleporter=` is `TechnoType+0xCD4`, parsed at `0x00713FE9/0x00713FF6` and read by the later locomotor swap, outside the named set.` (evidence: `0x00843E60`, `0x00713FE9`, `0x007423CD`)
- `[DEFERRED] OQ-09 -- What are the exact names of instance bytes `+0x6E0..+0x6E2`?` (category: `requires-different-system-context`; reason: target was the TypeClass-field audit and branch effect, not instance state-field ownership; next-step-if-pursued: trace writers/readers of `this+0x6E0..+0x6E2`)
- `[DEFERRED] OQ-10 -- What are the exact semantic names of vectors at `+0x5AC` and `+0x588` in the `+0xD2C` branch?` (category: `requires-different-system-context`; reason: navqueue producer slot owns queue layout; next-step-if-pursued: reconcile with slot-1 NavQueue producers and action-line report)
- `[DEFERRED] OQ-11 -- What does `FUN_007447B0` do internally?` (category: `out-of-scope`; reason: this slot only needed to prove call/rewrite effect; next-step-if-pursued: targeted bridge/passable-cell helper investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `BalloonHover= yes` on a UnitClass can intercept `Set_Destination(NULL, force)` before `FootClass::Set_Destination_Internal`; it does not clear/write a new `NavCom` on that branch. | `0x00741983..0x00741A7D`; parser `0x00714D95`; stock `DISK BalloonHover=yes` | missing/unchecked | `src/sim/movement/movement_commands.rs`, future public Set_Destination wrapper | Model a pre-internal-setter null-destination intercept for BalloonHover UnitClass runtime state, preserving old destination when the binary returns early. | Issue null destination to a BalloonHover UnitClass with existing `nav_com` and matching runtime relation; old `nav_com` remains and no new movement target is installed. Proposed test: `balloon_hover_null_destination_intercept_preserves_navcom` | Do not implement `BalloonHover` as just a jumpjet idle/landing flag if UnitClass `Set_Destination(NULL)` parity is in scope. |
| The top stop guard is not driven by `Crusher/OmniCrusher/OmniCrushResistant`; it reads instance bytes `+0x6E0..+0x6E2` and `+0x2B0`. | `0x00741A96..0x00741ACB`; parser writes for real crush fields `0x00714CE3/0x00714D04/0x00714D25` | docs stale; Rust should not add crush-field stop logic from OQ-6 | `src/sim/movement/movement_commands.rs`; navcom docs | Keep crush flags for crush/path systems. If modeling this stop guard later, add instance-state fields, not type crush predicates. | Unit with `OmniCrusher=yes` but valid movement state should not be stopped solely because the type has OmniCrusher. Proposed test: `set_destination_does_not_stop_for_omnicrusher_type_flag` | Do not copy old pseudocode that branches on `type.crusher/omni_crusher/omni_crush_resistant` in Set_Destination. |
| `MovementZone=Subterannean` sets `TechnoType+0xD2C`; this can queue/retain the original requested target and replace `param_2` with a passable/current-cell-derived `CellClass*` before final setter. | parser `0x00716065..0x0071608A`; reads/effects `0x00741E8E..0x0074208A`, `0x0074208A..0x007422F7`; final call `0x0074315F` | missing | `src/rules/locomotor_type.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/navcom.rs`, navigation queue state | Add a dedicated subterranean Set_Destination preprocessing path before generic pathfinding: preserve/queue original target when the Drive/current-loco predicates match and write effective `nav_com` to the binary-chosen cell target. | Subterranean unit commanded from a bridge/obstructed current cell rewrites destination to the binary nearest passable cell while action-line/queue can still expose the original endpoint as appropriate. Proposed test: `subterranean_set_destination_rewrites_bridge_target_before_navcom_commit` | Do not treat `MovementZone::Subterranean` as ordinary Track movement with only A* row differences. |

### Negative Facts / Do Not Do

- Do not label `TechnoType+0xD6A` as AutoAttackMove or chrono-specific. Evidence: parser xref to `BalloonHover` string at `0x00714D95`, write `[EBP+0xD6A]`, and read at `0x00741989`.
- Do not label `TechnoType+0xD2C` as Teleporter. Evidence: `Teleporter` is `+0xCD4` (`0x00713FE9/0x00713FF6`); `+0xD2C` is written from `MovementZone == 6` at `0x0071607E..0x0071608A`.
- Do not implement the `0x00741A96` stop guard using `Crusher/OmniCrusher/OmniCrushResistant` type fields. Evidence: the guard reads `[EBP+0x6E0]`, `[EBP+0x6E1]`, `[EBP+0x6E2]`, not `[[EBP+0x6C4]+0xD28...]`.
- Do not collapse `Subterannean` parser behavior into corrected spelling only. Evidence: MovementZone table row 6 is the binary spelling per `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md`.
- Do not assume every parsed TypeClass flag in the OQ has a Set_Destination effect. `+0xD28/+0xD29/+0xD2A` are real fields but their effects are in crush systems, not this preprocessing branch.

### Stale Docs / Follow-up Docs

- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`: replace OQ-6 with:
  > `TypeClass+0xD6A` is `BalloonHover=` and gates a null-destination intercept in UnitClass `Set_Destination`. `TypeClass+0xD2C` is the derived `MovementZone == Subterannean` byte and gates subterranean destination queue/passable-cell rewrite branches. The older `+0xD28/+0xD29/+0xD2A` stop-guard wording was a decompiler indexing mistake: the branch reads Unit/Foot instance bytes `+0x6E0/+0x6E1/+0x6E2`, not the TechnoType crush fields `Crusher/OmniCrusher/OmniCrushResistant`.
- `docs/research/UNITCLASS_SET_DESTINATION_NORMAL_DRIVE_CELL_GHIDRA_REPORT.md`: replace "Type movement flag set: `Type+0xD28 == 0`..." with:
  > Instance movement-state stop guard: `this+0x6E0 == 0` and (`this+0x6E1 != 0` or `this+0x6E2 != 0`) calls `FootClass::Stop_Moving` and returns; if `this+0x6E0 != 0` but `this+0x2B0 == 0`, it also calls `Stop_Moving` and returns. These are not `TechnoTypeClass+0xD28/+0xD29/+0xD2A`.
- `docs/research/TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`: replace "TypeClass[0xD6A] = IsChronoMiner flag (or similar)" with:
  > `TypeClass+0xD6A` is `BalloonHover=`, parsed at `TechnoTypeClass::ReadINI @ 0x00714D95`; the top block is a BalloonHover null-destination intercept, not a chrono-miner type check.

## Sources

- Ghidra read-only: `decompile_function 0x00741970`; `disassemble_function 0x00741970`; `decompile_function 0x00713FE9`; `get_bulk_xrefs` for strings `0x0081BB58`, `0x0084387C`, `0x00843868`, `0x00843838`, `0x008431C8`, `0x00843E60`; `get_assembly_context` for parser xrefs and `0x00716065`.
- Prior docs checked: `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`, `UNITCLASS_SET_DESTINATION_NORMAL_DRIVE_CELL_GHIDRA_REPORT.md`, `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`, `CRUSH_SYSTEM_GHIDRA_REPORT.md`, `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, `units/allied/JUMPJET.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
