# Skirmish Start Validation Modal Text And Mode Rejection - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x00734E60`, `0x005D6310`, `0x005CA6D0`, `0x005CB400`, `0x005C5D40`, `0x005C1D80`, `0x007B6880`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** visible StringTable/CSF text for Start validation string IDs `0x437/0x438/0x43F/0x440/0x457/0x458/0x469`, and whether stock/local retail Skirmish `MPModes` vtable `+0x14` methods can write output dword exactly `0x617` to trigger the generic mode rejection modal.  
**Non-Scope:** successful Start packing, Choose Map preview refresh, listbox/combo paint, trackbar disabled flow, start-marker clipping, and full MPModes UI role editing.  
**Confidence:** High for string key mapping, retail English text, stock/local mode non-`0x617` result, and YR activity; Medium only for non-stock/modded mode objects outside local retail data.  
**Active in YR:** Yes for ordinary Start validation text and stock selectable modes; Conditional for binary Siege/Unholy rejection branches by selected mode/data.

## 0. Working Notes

- Target question: What visible text do Start validation modal string IDs `0x437/0x438/0x43F/0x440/0x457/0x458/0x469` resolve to, and can any retail/local Skirmish `MPModes +0x14` method write output dword `0x617`?
- Non-goals: Do not re-investigate successful packing, Choose Map preview refresh, OwnerDraw listbox, ComboDropWin, trackbar disabled flow, or start-marker clipping.
- Evidence needed to mark COMPLETE: string ID to key mapping from `gamemd.exe`, decoded CSF values from active retail language asset, Start caller branch evidence, concrete stock/local MPModes `+0x14` method evidence, and local `MPModesMD.ini` stock roster check.
- Stop conditions: stop after all seven IDs have visible text/key values and all known stock/local retail mode methods are classified for `0x617` output; leave modded/custom mode behavior as open only.

## 1. Overview

The seven numeric IDs are not the final player-visible text by themselves. In these calls, `StringTable__LoadString @ 0x00734E60` receives the CSF key in `ECX`; the pushed source path `D:\ra2mdpost\Skirmish.cpp` plus numeric ID are debug/missing-string context. The visible text comes from the loaded CSF key.

For standard local YR Skirmish data, no selectable retail `MPModes +0x14` method writes output dword `0x617`. The caller's generic `0x469` modal block is live code, but its blocking condition is not reached by the stock/local exposed roster.

## 2. String ID To Visible Text

Decoded text source: direct read of retail `<ra2-install>/langmd.mix` CSF label entries. Labels are stored as plain ASCII in the CSF body; values were decoded by bitwise-NOT UTF-16LE, matching `src/assets/csf_file.rs`. `language.mix` corroborates `TXT_SCENARIO_TOO_SMALL`, `TXT_NEED_AT_LEAST_TWO_PLAYERS`, and `TXT_OK`; `TXT_CANNOT_ALLY` is YR-only in `langmd.mix`.

| ID | Load-site key evidence | CSF key | Retail English visible text | Modal role | Active in YR |
|---:|---|---|---|---|---|
| `0x437` | `0x006AD073..0x006AD090`, `ECX=0x82C9FC`; PE string at `0x0082C9FC` | `TXT_SCENARIO_TOO_SMALL` | `This map has a %d player max. The max includes human and computer players.` | capacity message, formatted with selected map capacity | Yes: ordinary Start capacity failure |
| `0x438` | `0x006AD0A7..0x006AD0C6`, `ECX=0x825FB0`; PE string at `0x00825FB0` | `TXT_OK` | `OK` | modal OK/control text | Yes: capacity failure modal |
| `0x43F` | `0x006AD0F8..0x006AD10F`, `ECX=0x83FC68`; PE string at `0x0083FC68` | `TXT_NEED_AT_LEAST_TWO_PLAYERS` | `You need at least two players to start the game!` | minimum-player message | Yes: ordinary Start min-player failure |
| `0x440` | `0x006AD126..0x006AD145`, `ECX=0x825FB0`; PE string at `0x00825FB0` | `TXT_OK` | `OK` | modal OK/control text | Yes: min-player failure modal |
| `0x457` | `0x006AD243..0x006AD25D`, `ECX=0x831450`; PE string at `0x00831450` | `TXT_CANNOT_ALLY` | `Must have more than one team to start a game!` | same-explicit-team message | Yes: ordinary Start same-team failure |
| `0x458` | `0x006AD274..0x006AD293`, `ECX=0x825FB0`; PE string at `0x00825FB0` | `TXT_OK` | `OK` | modal OK/control text | Yes: same-team failure modal |
| `0x469` | `0x006AD2E9..0x006AD316`, `ECX=0x825FB0`; PE string at `0x00825FB0` | `TXT_OK` | `OK` | generic selected-mode rejection modal OK/control text; the message body is the mode output string from `FUN_007B7100` | Conditional: only when selected mode returns false and output dword is `0x617` |

Tiny but load-bearing detail: the visible message for the generic selected-mode rejection modal is not ID `0x469`; `0x469` resolves to `TXT_OK`. The message body is the output object dereferenced by `FUN_007B7100`, which returns an empty string when the output pointer is zero.

## 3. Mode `+0x14` Rejection Sweep

The Start caller initializes the output object with `FUN_007B66C0`, dispatches selected mode vtable `+0x14`, and checks `CMP [ESP+0x1C], 0x617` only on a false return. Evidence: `0x006AD2BA..0x006AD2E1`; Active in YR: Yes for offline Skirmish Start.

| Mode / category | Stock/local exposure | `+0x14` evidence | Can write output dword `0x617`? | Active in YR |
|---|---|---|---|---|
| `Battle` | yes, ids `1` and `9` in `ini/mpmodesmd.ini` | vtable target `0x005D6310`; decompile returns `1` | No; accepts and does not touch output | Yes |
| `ManBattle` | yes, ids `5..8` in `ini/mpmodesmd.ini` | vtable target `0x005D6310`; decompile returns `1` | No; accepts and does not touch output | Yes |
| `FreeForAll` | yes, id `2` in `ini/mpmodesmd.ini` | `0x005C5D40` assembly delegates to `0x005D6310`, then rewrites node `+0x6B`, returns `1` | No; accepts after side effects | Conditional by selected mode |
| `Cooperative` | yes, id `3` in `ini/mpmodesmd.ini` | `0x005C1D80` optionally calls `0x0049B760`, then delegates to `0x005D6310` | No; accepts via base method | Conditional by selected mode |
| `Unholy` | yes, id `4` in `ini/mpmodesmd.ini` | `0x005CB400` returns `0` when `DAT_00A8B258 == 0`, otherwise delegates to `0x005D6310` | No; false branch writes no output, so initialized dword remains `0` | Conditional by selected mode/global byte |
| `Siege` | no stock/local `ini/mpmodesmd.ini` roster row; binary category exists | `0x005CA6D0` false branches call `0x007B6880` with Siege message strings | No immediate `0x617`; `0x007B6880` stores an allocated wide-string pointer into the first output dword | No for stock local roster; Conditional if data supplies Siege |

`FUN_007B6880` frees any prior output pointer, allocates `(len + 1) * 2` bytes, stores that pointer as the first output dword, then copies the wide string. Active in YR: Yes where a mode uses it. This proves Siege-style failures do not intentionally write the literal command code `0x617`.

## 4. Current Rust Implementation Status

Rust now has data-level capacity, no-opponent, and same-explicit-team validation in `src/ui/skirmish_shell/state.rs::launch_session`, returning `LaunchValidationError::MapCapacityExceeded`, `NoEnabledOpponent`, and `SameExplicitTeam`. The app surface maps these failures to `SkirmishValidationModalState` and current rendering draws a primitive validation modal. Remaining deltas are native-shaped shell modal text/layout/art, exact capacity `%d` formatting, and a full MPModes object model for the selected-mode `+0x14` output contract.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `StringTable__LoadString` call convention | verified | decompile/assembly `0x00734E60`, calls from `0x006AD073..0x006AD316` | none |
| seven Start validation IDs | verified | table in section 2 | none |
| retail English CSF text | verified | `langmd.mix` direct CSF entry parse offsets: `TXT_OK` `132774`, `TXT_SCENARIO_TOO_SMALL` `176497`, `TXT_NEED_AT_LEAST_TWO_PLAYERS` `176687`, `TXT_CANNOT_ALLY` `602509` | non-English/localized installs outside this retail path |
| generic mode rejection caller gate | verified | `0x006AD2BA..0x006AD346` | none |
| stock/local selectable modes can write `0x617` | verified negative | `ini/mpmodesmd.ini`, mode methods listed in section 3 | modded custom MPModes outside local retail data |
| binary Siege support | verified conditional | `0x005CA6D0`, `0x007B6880`, no `[Siege]` in `ini/mpmodesmd.ini` | no stock UI route in exposed roster |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - What do IDs 0x437/0x438/0x43F/0x440/0x457/0x458/0x469 display? -> They resolve to the CSF keys/text in section 2; three IDs are `TXT_OK`.` (evidence: `0x006AD073..0x006AD316`, `langmd.mix` CSF entries)
- `[RESOLVED] OQ-02 - Is this active in YR? -> Yes for ordinary Start validation; the mode rejection text is conditional on the stricter false+0x617 gate.` (evidence: `FUN_006ACEE0`, `FUN_006AE2C0` inherited from parent report)
- `[RESOLVED] OQ-03 - Can Battle/ManBattle write 0x617? -> No; `0x005D6310` returns true unconditionally.` (evidence: decompile `0x005D6310`)
- `[RESOLVED] OQ-04 - Can FreeForAll/Cooperative write 0x617? -> No; both accept through the base method after side effects.` (evidence: assembly `0x005C5D40`, `0x005C1D80`)
- `[RESOLVED] OQ-05 - Can stock Unholy write 0x617? -> No; its false branch writes no output, leaving the initialized dword zero.` (evidence: assembly `0x005CB400..0x005CB421`, `0x007B66C0`)
- `[RESOLVED] OQ-06 - Can local retail Siege trigger 0x617? -> No for stock local roster because no `[Siege]` row exists; binary Siege failures write allocated string pointers, not literal `0x617`.` (evidence: `ini/mpmodesmd.ini`, `0x005CA6D0`, `0x007B6880`)
- `[DEFERRED] OQ-07 - Could a modded/custom MPModes object intentionally write literal `0x617`?` (category: out-of-scope; reason: user asked retail/local Skirmish MPModes; next-step-if-pursued: investigate mod extension hooks or non-retail objects)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Capacity failure shows `TXT_SCENARIO_TOO_SMALL` formatted with capacity and `TXT_OK` | `0x006AD073..0x006AD0C6`; `langmd.mix` CSF decode | partially implemented: validation and primitive modal exist, but native text formatting/layout/art remain incomplete | `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`, shell modal renderer/state | surface native-visible error text/layout instead of primitive/non-native formatting | map capacity `2` with local+2 AIs stays in shell and modal body is `This map has a 2 player max...`, OK text is `OK` | Do not use raw numeric ID or append non-native `(requested/capacity)` text; proposed test `skirmish_start_capacity_modal_uses_retail_csf_text` |
| Min-player failure shows `TXT_NEED_AT_LEAST_TWO_PLAYERS` and `TXT_OK` | `0x006AD0F8..0x006AD145`; `langmd.mix` CSF decode | partially implemented: `NoEnabledOpponent` maps to primitive modal state, but native modal parity remains incomplete | same | classify no-opponent failure as native min-player modal text | all AI rows disabled stays in shell and modal body is `You need at least two players to start the game!` | Do not leave this as a warning log; proposed test `skirmish_start_no_opponent_modal_uses_retail_csf_text` |
| Same-explicit-team failure shows `TXT_CANNOT_ALLY` and `TXT_OK` | `0x006AD243..0x006AD293`; `langmd.mix` CSF decode | partially implemented: `SameExplicitTeam` maps to primitive modal state, but native modal parity remains incomplete | same | classify all active players on one explicit team as native same-team modal text | local team 1 and all active AIs team 1 blocks with `Must have more than one team to start a game!` | Do not call this start-position collision; proposed test `skirmish_start_same_team_modal_uses_cannot_ally_text` |
| Generic mode rejection `0x469` is only OK text and stock/local modes do not produce output dword `0x617` | `0x006AD2D9..0x006AD316`; mode sweep section 3 | missing MPModes model; no immediate stock Rust delta for generic modal | future MPModes model plus `launch_session` validation | keep generic `0x469` modal gated on literal output dword `0x617`; stock modes should not show it | synthetic test mode false+`0x617` blocks with mode output body and OK text; stock Battle/FFA/Coop/Unholy paths do not | Do not treat every false mode return as a `0x469` modal; proposed test `skirmish_mode_rejection_ok_text_requires_native_start_code` |

## 8. Negative Facts / Do Not Do

- Do not display `0x438`, `0x440`, `0x458`, or `0x469` as unique message bodies; all four resolve to `TXT_OK` in this slice. Active in YR: Yes/Conditional by call site; evidence `0x006AD0A7..0x006AD316`, PE string `0x00825FB0`, `langmd.mix` `TXT_OK`.
- Do not treat `0x469` as the generic mode rejection body text. Active in YR: Conditional; evidence `0x006AD302..0x006AD316` passes the mode output from `FUN_007B7100` as first modal argument and `TXT_OK` as the second.
- Do not implement a stock/local generic mode rejection modal for Battle, ManBattle, FreeForAll, Cooperative, or Unholy. Active in YR: Yes/Conditional; evidence `0x005D6310`, `0x005C5D40`, `0x005C1D80`, `0x005CB400`.
- Do not expose Siege in stock offline Skirmish just to exercise this modal. Active in YR: No for stock local roster; evidence no `[Siege]` in `ini/mpmodesmd.ini`, though binary category support exists.
- Do not model Siege false output as literal `0x617`; its failure paths call `FUN_007B6880`, which writes an allocated string pointer. Active in YR: Conditional; evidence `0x005CA720..0x005CA7AB`, `0x007B6880`.

## 9. Remaining Uncertainty

- Modded/custom non-retail MPModes objects could intentionally write output dword `0x617`; this report only claims stock/local retail YR data and binary-known mode classes.
- Non-English installs may have different localized values for the same CSF keys; the decoded retail path here is the active local English install.

## Stale Docs / Replacement Wording

- `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`: replace "Exact localized text for numeric string IDs `0x437/0x438/0x43F/0x440/0x457/0x458/0x469` was not decoded; IDs and call sites are verified." with "The Start validation IDs resolve through CSF keys, not directly to unique message strings: `0x437 -> TXT_SCENARIO_TOO_SMALL` (`This map has a %d player max. The max includes human and computer players.`), `0x43F -> TXT_NEED_AT_LEAST_TWO_PLAYERS` (`You need at least two players to start the game!`), `0x457 -> TXT_CANNOT_ALLY` (`Must have more than one team to start a game!`), and `0x438/0x440/0x458/0x469 -> TXT_OK` (`OK`). The generic mode rejection message body comes from the mode output object; `0x469` is the OK/control text."
- `docs/research/skirmish-ui/SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`: replace "Whether any retail/custom selected mode `+0x14` method writes output dword exactly `0x617` remains open" with "No stock/local retail selectable MPModes `+0x14` method writes output dword exactly `0x617`: Battle/ManBattle accept, FreeForAll/Cooperative accept after side effects, Unholy false leaves output zero, and Siege is not in stock `MPModesMD.ini` while its binary false paths write allocated string pointers via `FUN_007B6880`, not literal `0x617`. Modded/custom mode objects remain out of scope."

## Sources

- Ghidra read-only decompile/assembly: `0x006ACEE0`, `0x00734E60`, `0x005D6310`, `0x005CA6D0`, `0x005CB400`, `0x005C5D40`, `0x005C1D80`, `0x007B66C0`, `0x007B6760`, `0x007B6880`, `0x007B7100`.
- Ghidra assembly contexts: `0x006AD073..0x006AD316`, `0x005CA701..0x005CA7BF`, `0x006AD2BA..0x006AD346`.
- Binary PE string reads from local retail `gamemd.exe`: `0x0082C9FC`, `0x0083FC68`, `0x00831450`, `0x00825FB0`, `0x0083FC4C`, Siege keys at `0x0082FEE8..0x0082FF2C`.
- Asset text decoded from local retail `<ra2-install>/langmd.mix`; method: locate CSF label entry by ASCII key, decode value as bitwise-NOT UTF-16LE.
- INI checked: `ini/mpmodesmd.ini`.
- Prior reports referenced: `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`, `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`.
