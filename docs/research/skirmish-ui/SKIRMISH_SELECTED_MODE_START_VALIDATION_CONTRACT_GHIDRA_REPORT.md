# Skirmish Selected-Mode Start Validation Contract - Ghidra Research Report

**Address(es):** `0x006ACEE0`, selected-mode vtable `+0x14` targets `0x005D6310`, `0x005C5D40`, `0x005C1D80`, `0x005CB400`, `0x005CA6D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish Start click selected-mode acceptance/rejection contract: caller ordering, output/result semantics, stock selected-mode callback results, visible rejection text source, and direct Rust validation handoff.  
**Non-Scope:** Start button pre-disabled visuals, WOL/network game startup, successful post-shell spawn/mission init, broad Choose Map UI behavior, custom/modded MPModes beyond stock local data, and native modal pixel reconstruction.  
**Confidence:** High for the stock/local selected-mode contract and ordinary Rust deltas; Medium for non-stock mode objects because they are outside local retail data.  
**Active in YR:** Yes for the offline Skirmish Start caller and stock local selectable MPModes; Conditional for binary-only Siege/Unholy rejection branches by selected mode/data.

## 0. Working Notes

- Target question: What is the selected-mode Start acceptance/rejection contract for offline Skirmish, including output/result semantics and visible rejection behavior?
- Non-goals: Start-button visual enablement, networking/WOL, full session spawn after shell exit, and broad map/setup UI behavior outside the Start click path.
- Evidence needed to mark COMPLETE: Ghidra decompile plus assembly for `0x006ACEE0` selected-mode dispatch and concrete stock `+0x14` implementations, xref/loader evidence that the modes are live in YR, prior CSF/modal text confirmation, and current Rust validation surface scan.
- Stop conditions: Stop once all selected-mode Start result cases are resolved or explicitly deferred, no new branches appear in a final pass over the dispatcher and concrete methods, and the report includes Implementation Handoff plus Negative Facts / Do Not Do.

Note on this pass: current Ghidra MCP `batch_decompile` returned "Function not found" for the raw addresses and `FUN_` names. Per read-only constraints, no function boundaries were created. Handoff-critical claims therefore cite existing decompile-backed reports plus fresh assembly contexts from this slot.

## 1. Overview

Offline Skirmish Start validates ordinary setup constraints first, then initializes a small output/string object and calls the selected MPModes object's vtable `+0x14`. A selected-mode callback returning nonzero accepts and Start falls through into launch packing; a false return blocks only if the output object's first dword equals literal `0x617`.

For stock local YR data, Battle/Team Game/ManBattle, Free For All, and Cooperative accept through this callback. Unholy can return false when its global enable byte is clear, but that false result writes no `0x617` output, so the generic selected-mode modal is not shown and Start continues past the selected-mode gate. Siege has binary rejection logic and localized role error strings, but no stock local `[Siege]` row exists in `ini/mpmodesmd.ini`; if supplied by data, its failures write string pointers, not literal `0x617`.

## 2. Class Layout / Key Offsets

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B23C` | selected MPModes object pointer used by Start | assembly `0x006AD2BA`; prior `SKIRMISH_MPMODES_SESSION_PACKING_BROAD_RECHECK_GHIDRA_REPORT.md` | Yes |
| selected object vtable `+0x14` | selected-mode Start acceptance callback | assembly `0x006AD2C9..0x006AD2D2`; prior vtable report | Yes/Conditional by selected mode |
| local output object at `ESP+0x18` / first dword checked at `ESP+0x1C` after push adjustment | mode output/result buffer initialized before vtable call | assembly `0x006AD2C0..0x006AD2E1`; prior text report | Yes |
| `DAT_00A8B250` | selected mode id mirror copied after acceptance | assembly `0x006AD34B..0x006AD364`; prior packing report | Yes |
| `DAT_00A8B254` | selected scenario/map index mirror copied after acceptance, clamped to `0` if out of range | assembly `0x006AD34B..0x006AD36B`; prior packing report | Yes |
| `DAT_00A8DA78`, `DAT_00A8DA84` | player/node pointer array and count consumed by FFA/Coop/Siege callbacks | assembly `0x005C5D51`, `0x005C1D80`, `0x005CA6E9`; prior vtable report | Yes/Conditional |
| node `+0x6B` | mode-role/team-adjacent value used by FFA rewrite and Siege validation | assembly `0x005C5D6A..0x005C5D71`, `0x005CA701`; prior role report | Conditional |
| `DAT_00A8B258` | Unholy enable byte | assembly `0x005CB400`, writer at `0x005CB3F0..0x005CB3F2`; prior vtable report | Conditional |

## 3. Core Logic

### 3.1 Start caller ordering

Start command `0x617` enters `0x006ACEE0` and performs ordinary setup validation before the selected-mode callback. The selected-mode dispatch begins only after capacity, minimum-player, and same-explicit-team checks have passed. Active in YR: Yes.

Verified selected-mode sequence:

1. Load selected mode pointer from `DAT_00A8B23C`.
2. Initialize local output object with `0x007B66C0`.
3. Load selected object's vtable and call `vtable+0x14`, passing the output object pointer.
4. If `AL != 0`, jump directly to accepted packing at `0x006AD34B`.
5. If `AL == 0`, compare output first dword against literal `0x617`.
6. If output dword is not `0x617`, call `0x005D5E10` then continue into accepted packing at `0x006AD34B`.
7. If output dword is `0x617`, show the generic selected-mode modal, re-enable Start, clean up the output object, and return before packing.

Evidence: assembly `0x006AD2BA..0x006AD34B`; prior decompile-backed reports `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` and `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`. Active in YR: Yes.

Tiny detail: false return alone is not a blocking result. The blocking condition is `false && output_dword == 0x617`. This matters because stock Unholy can return false with output zero, and that does not show the generic mode rejection modal.

### 3.2 Visible selected-mode rejection text

The generic selected-mode rejection branch pushes string id `0x469`, but that id resolves to `TXT_OK`. The modal body is not `0x469`; it is the mode output string returned by `0x007B7100` from the initialized output object. Evidence: assembly `0x006AD2E9..0x006AD316`; prior CSF decode in `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`. Active in YR: Conditional, only if `false && output_dword == 0x617`.

### 3.3 Concrete stock/local callback results

| Mode / class | Stock local row(s) | `+0x14` target | Result contract | Output dword `0x617`? | Active in YR |
|---|---|---:|---|---|---|
| Battle / Team Game | `[Battle]` ids `1`, `9` | `0x005D6310` | `MOV AL,1; RET 4`; unconditional accept | No | Yes |
| ManBattle | `[ManBattle]` ids `5..8` | `0x005D6310` | same unconditional accept | No | Yes |
| FreeForAll | `[FreeForAll]` id `2` | `0x005C5D40` | delegates to base accept; if accepted, iterates node array and rewrites any node `+0x6B != -1` to that node's index; returns accept | No | Conditional by selected mode |
| Cooperative | `[Cooperative]` id `3` | `0x005C1D80` | if node count is exactly `2` and `this+0x40` is non-null, calls `0x0049B760(node0,node1)`, then delegates to base accept | No | Conditional by selected mode |
| Unholy | `[Unholy]` id `4` | `0x005CB400` | if `DAT_00A8B258 == 0`, returns false without touching output; otherwise delegates to base accept | No; false branch leaves initialized output non-`0x617` | Conditional by selected mode/global byte |
| Siege | no stock local row | `0x005CA6D0` | validates node `+0x6B` roles: exactly one defender value `1`, at least one attacker value `2`, values outside `0..2` reject | No; failures call `0x007B6880` with localized strings, which stores an allocated string pointer | No for stock local roster; Conditional if data supplies Siege |

Evidence: assembly contexts `0x005D6310`, `0x005C5D40..0x005C5D86`, `0x005C1D80..0x005C1DB2`, `0x005CB400..0x005CB421`, `0x005CA6D0..0x005CA7BF`; prior decompile-backed vtable report. Active in YR: as listed.

### 3.4 Siege role rejection details

Siege's binary callback is useful as a negative fact even though stock local data does not expose it. It scans each node value at `+0x6B`:

| Node `+0x6B` value | Effect | Evidence | Active in YR |
|---:|---|---|---|
| `0` | accepted by scan; no counter change | `0x005CA704..0x005CA718` | Conditional, Siege selected |
| `1` | defender/besieged slot; a second `1` rejects with `MP:OnlyOneBeseiged` | `0x005CA709..0x005CA716`, `0x005CA76C..0x005CA782` | Conditional |
| `2` | attacker count increments | `0x005CA70C..0x005CA710` | Conditional |
| other | rejects with `MP:IllegalTeam` | `0x005CA70C..0x005CA75E` | Conditional |
| no defender | rejects with `MP:NoDefender` | `0x005CA720..0x005CA73A` | Conditional |
| defender but no attackers | rejects with `MP:NoAttackers` | `0x005CA790..0x005CA7AB` | Conditional |

The string keys are present at `0x0082FEE8` (`MP:NoAttackers`), `0x0082FEF8` (`MP:OnlyOneBeseiged`), `0x0082FF0C` (`MP:IllegalTeam`), and `0x0082FF1C` (`MP:NoDefender`), with source file string `0x0082FF2C` (`D:\ra2mdpost\MPSiege.cpp`). Active in YR: Conditional if a Siege object is selected; no stock local row.

## 4. INI Keys / Data

No `rulesmd.ini` or `artmd.ini` key directly controls this selected-mode Start callback gate. The relevant stock local data is `ini/mpmodesmd.ini`, which defines the selectable mode rows and omits `[Siege]`.

| INI section | Rows | Mode class | Start callback effect | Active in YR |
|---|---|---|---|---|
| `[Battle]` | `1=GUI:Battle,...,standard,true`; `9=GUI:TeamGame,...,teamgame,false` | Battle | unconditional accept | Yes |
| `[ManBattle]` | ids `5..8` | ManBattle | unconditional accept | Yes |
| `[FreeForAll]` | id `2` | FreeForAll | accept after node role rewrite | Yes when selected |
| `[Unholy]` | id `4` | Unholy | conditional false without `0x617`; otherwise accept | Yes when selected |
| `[Cooperative]` | id `3` | Cooperative | optional pre-call, then accept | Yes when selected |
| `[Siege]` | absent | Siege | binary support only, not stock local selectable | No for stock local roster |

Binary loader activity evidence: strings `MPModesMD.ini` at `0x00830A18`, mode class strings `Cooperative` `0x00830BCC`, `FreeForAll` `0x00830BD8`, `Unholy` `0x00830BE4`, `Siege` `0x00830BEC`, `ManBattle` `0x00830BF4`, `Battle` `0x00830C00`; factory/constructor addresses in prior vtable report. Active in YR: Yes for `MPModesMD.ini` local roster.

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map selected mode commit | selected mode pointer/id and selected map are committed together before Start | prior `SKIRMISH_MPMODES_SESSION_PACKING_BROAD_RECHECK_GHIDRA_REPORT.md`, `0x005E7160`, `0x005E71E5..0x005E7382` | Yes |
| Start ordinary validation before mode callback | capacity, minimum-player, and same-explicit-team failures return before selected-mode dispatch | assembly `0x006AD05B..0x006AD2BA`; prior modal recheck | Yes |
| Selected-mode callback | selected object from `DAT_00A8B23C` receives output buffer via vtable `+0x14` | assembly `0x006AD2BA..0x006AD2D2` | Yes |
| Generic selected-mode rejection modal | body comes from mode output; OK text from `TXT_OK`; only blocks on output dword `0x617` | assembly `0x006AD2D9..0x006AD316`; prior text report | Conditional |
| Accepted path | copies selected mode id/map mirrors and continues packing | assembly `0x006AD34B..0x006AD36B`; prior packing report | Yes |

No simulation tick-cycle path is claimed here; this is shell/setup validation before shell exit.

## 6. Current Rust Implementation Status

| Rust surface | Current status | Delta vs selected-mode contract |
|---|---|---|
| `src/ui/skirmish_shell/state.rs:1914..2002` | `launch_session` validates selected map existence, map capacity, no enabled opponent, and same explicit team, then returns `SkirmishLaunchSession`. | Missing selected-mode callback/result model; always packs `mode: SkirmishLaunchMode::Battle` at line `1995`. |
| `src/app.rs:622..637` | Start action calls `launch_session`; successful session starts; errors are mapped to a validation modal only for native ordinary failures. | No selected-mode false/output-dword handling surface yet. |
| `src/app.rs:664..697` | ordinary native validation errors map to `TXT_SCENARIO_TOO_SMALL`, `TXT_NEED_AT_LEAST_TWO_PLAYERS`, `TXT_CANNOT_ALLY`, and `TXT_OK`. | Correct for ordinary failures; selected-mode generic modal body/output contract is not represented. |
| `src/skirmish_launch.rs:14`, `src/skirmish_launch.rs:201`, `src/skirmish_launch.rs:210` | launch mode enum/session/error model exists, but launch mode is currently only `Battle` in the Start packing surface. | Needs selected mode id/class behavior before callback parity can be represented. |
| `src/skirmish_modes.rs:21..109` | parsed/stock `SkirmishGameMode` rows include `mode_id`, `display_label_key`, `rules_override`, `map_filter`, and known stock defaults. | Data model is enough to select/filter maps, but not enough to express mode class callback semantics or `false + 0x617` output. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes and target scope | verified | section 0 | none |
| Start selected-mode dispatch gate | verified | assembly `0x006AD2BA..0x006AD34B`; prior decompile reports | none |
| Generic selected-mode modal text source | verified | assembly `0x006AD2E9..0x006AD316`; prior CSF report | none |
| Battle/ManBattle callback | verified | assembly `0x005D6310`; prior vtable report | none |
| FreeForAll callback | verified | assembly `0x005C5D40..0x005C5D86`; prior vtable report | downstream consumer of rewritten node `+0x6B` is outside this target |
| Cooperative callback | verified | assembly `0x005C1D80..0x005C1DB2`; prior cooperative pre-call report | cooperative campaign/save internals are outside this target |
| Unholy callback | verified | assembly `0x005CB400..0x005CB421`; prior vtable report | exact user path for `DAT_00A8B258 == 0` remains adjacent |
| Siege callback | verified conditional | assembly `0x005CA6D0..0x005CA7BF`; string search `0x0082FEE8..0x0082FF2C`; prior vtable report | no stock local `[Siege]`; modded exposure out of scope |
| Stock local MPModes roster | verified | `ini/mpmodesmd.ini`; strings `0x00830A18..0x00830C00` | custom/modded MPModes out of scope |
| Current Rust ordinary validation mapping | verified | `src/ui/skirmish_shell/state.rs:1914..2002`; `src/app.rs:664..697` | add selected-mode acceptance surface/tests later |
| Current Rust selected-mode callback parity | touched-not-exhausted | Rust scan listed in section 6 | future implementation work, not research blocker |
| Network/WOL selected-mode behavior | not-touched | out of scope | separate network investigation |
| Native validation modal pixels | not-touched | out of scope | separate visual contract |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the selected mode object used by Start? -> `DAT_00A8B23C`, committed by Choose Map and loaded immediately before the vtable call.` (evidence: `0x006AD2BA`; prior `0x005E7160` report)
- `[RESOLVED] OQ-02 - Is the path live in YR? -> Yes for offline Skirmish; the data source is `MPModesMD.ini` and the Start command is `0x617`.` (evidence: `0x00830A18`; `0x006AD2BA`; `ini/mpmodesmd.ini`)
- `[RESOLVED] OQ-03 - What does the callback receive? -> One argument, the initialized output object pointer, after `0x007B66C0` initializes it.` (evidence: `0x006AD2C0..0x006AD2D2`; prior text report)
- `[RESOLVED] OQ-04 - Does false return always reject Start? -> No; rejection modal/return requires false plus output dword exactly `0x617`.` (evidence: `0x006AD2D5..0x006AD346`)
- `[RESOLVED] OQ-05 - What happens on false plus non-`0x617` output? -> Native calls `0x005D5E10` and continues into accepted packing at `0x006AD34B`.` (evidence: `0x006AD2D9..0x006AD34B`)
- `[RESOLVED] OQ-06 - What is visible text for the generic selected-mode rejection? -> Body comes from the output object via `0x007B7100`; `0x469` is `TXT_OK`.` (evidence: `0x006AD2E9..0x006AD316`; prior CSF report)
- `[RESOLVED] OQ-07 - Do Battle/Team Game/ManBattle reject? -> No; their `+0x14` is unconditional accept `0x005D6310`.` (evidence: `0x005D6310`; `ini/mpmodesmd.ini`)
- `[RESOLVED] OQ-08 - Does FreeForAll reject? -> No in stock path; it delegates to base accept and rewrites node `+0x6B` values that are not `-1`.` (evidence: `0x005C5D40..0x005C5D86`)
- `[RESOLVED] OQ-09 - Does Cooperative reject? -> No in this callback; it optionally calls `0x0049B760` for exactly two nodes and `this+0x40 != 0`, then delegates to base accept.` (evidence: `0x005C1D80..0x005C1DB2`)
- `[RESOLVED] OQ-10 - Does Unholy trigger the generic modal? -> No for known stock callback behavior; false when `DAT_00A8B258 == 0` writes no output, so no `0x617` modal gate.` (evidence: `0x005CB400..0x005CB421`; `0x006AD2D9..0x006AD346`)
- `[RESOLVED] OQ-11 - Does stock local Siege trigger this contract? -> No stock local `[Siege]` row exists; binary Siege support is conditional if data supplies it.` (evidence: `ini/mpmodesmd.ini`; string `0x00830BEC`)
- `[RESOLVED] OQ-12 - If Siege is selected by custom data, does it write literal `0x617`? -> No evidence of that; its failures call `0x007B6880` with localized strings, which prior report verified stores a string pointer.` (evidence: `0x005CA720..0x005CA7AB`; prior `0x007B6880` decompile report)
- `[RESOLVED] OQ-13 - Which ordinary Start failures precede the selected-mode callback? -> map capacity, fewer than two players, same explicit team.` (evidence: `0x006AD05B..0x006AD2A7`; prior modal recheck)
- `[RESOLVED] OQ-14 - Does current Rust carry selected mode into launch session? -> Not yet; `launch_session` always writes `SkirmishLaunchMode::Battle`.` (evidence: `src/ui/skirmish_shell/state.rs:1994..1996`)
- `[RESOLVED] OQ-15 - Does current Rust already map ordinary native validation modals? -> Yes for capacity, no enabled opponent, and same explicit team.` (evidence: `src/app.rs:664..697`)
- `[DEFERRED] OQ-16 - Can custom/modded non-stock MPModes write output dword `0x617` intentionally?` (category: out-of-scope; reason: target is stock offline Skirmish selected modes; next-step-if-pursued: investigate mod extension/custom factory paths)
- `[DEFERRED] OQ-17 - Exact user path that leaves Unholy `DAT_00A8B258 == 0`.` (category: requires-different-system-context; reason: callback result is enough for this contract, but the UI path for the byte belongs to mode setup/session flow; next-step-if-pursued: trace Unholy setup hooks `0x005CB3F0` and `0x005CB430`)
- `[DEFERRED] OQ-18 - Native modal pixel/resource geometry for generic selected-mode rejection.` (category: out-of-scope; reason: this target is validation contract, not modal art; next-step-if-pursued: modal resource/layout investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Start calls selected mode `+0x14` only after ordinary validation passes | `0x006AD05B..0x006AD2D2`; prior decompile reports | missing selected-mode callback model | `src/ui/skirmish_shell/state.rs:1914..2002`, future mode/session validation surface | keep ordinary capacity/min-player/same-team checks before selected-mode acceptance | invalid capacity setup should show capacity modal and never evaluate selected-mode rejection | Do not let selected-mode callbacks mask ordinary native validation failures |
| Selected-mode accept is `AL != 0`; false alone is not blocking | `0x006AD2D5..0x006AD346` | missing; no selected-mode error enum/result yet | `src/skirmish_launch.rs:210..231`, `src/app.rs:664..697` | represent native result as accepted, false-nonblocking, or false-blocking-with-output rather than a simple bool | synthetic callback false with output `0` should not show the generic selected-mode modal | Do not map every false callback to a modal; that would reject stock Unholy incorrectly |
| Generic selected-mode modal requires false plus output dword `0x617`; body is output string, OK is `TXT_OK` | `0x006AD2D9..0x006AD316`; prior CSF report | missing selected-mode modal result type | future launch/session validation and app modal mapping | when implemented, gate the modal on literal native Start code and use output string as body | synthetic false+`0x617` result stays in shell, body equals supplied output string, OK text resolves from `TXT_OK` | Do not use string id `0x469` as the body; it is OK/control text |
| Stock Battle, Team Game, and ManBattle callbacks always accept | `0x005D6310`; `ini/mpmodesmd.ini` rows `1`, `5..9` | current Rust happens to launch as Battle-only, but selected mode id is not carried | `src/ui/skirmish_shell/state.rs:1994..1996`, `src/skirmish_launch.rs:14..201` | selected stock Battle-like modes should not add mode-specific Start rejections | selecting Battle, Team Game, Duel, Megawealth, Meat Grinder, Naval War with otherwise valid setup launches instead of showing generic mode modal | Do not invent selected-mode setup constraints for these modes |
| FreeForAll accepts and rewrites node `+0x6B` values that are not `-1` to node index | `0x005C5D40..0x005C5D86`; prior vtable report | missing selected-mode side effect/session behavior | future selected-mode launch session packing | implement equivalent deterministic session effect when FFA mode semantics are added | valid FFA setup launches and records per-node FFA role/index behavior equivalent to native | Do not treat FFA as a Start validation rejection mode |
| Cooperative accepts after optional two-node pre-call | `0x005C1D80..0x005C1DB2`; prior cooperative pre-call report | missing cooperative-specific side effect | future selected-mode launch/session packing | preserve optional pre-call semantics if Cooperative becomes supported | two-node Cooperative setup records the native cooperative pre-call data and still accepts | Do not add player/team rejection in this callback |
| Unholy false with output zero does not show generic modal and continues past the selected-mode gate | `0x005CB400..0x005CB421`; caller gate `0x006AD2D9..0x006AD346` | missing selected-mode model; current stock UI may not expose the edge | future selected-mode validation | distinguish false-non`0x617` from false+`0x617` | forced Unholy disabled-byte state should not show the generic selected-mode rejection modal from this gate | Do not implement `false == block` |
| Stock local roster omits Siege; binary Siege failures write localized strings, not literal `0x617` | `ini/mpmodesmd.ini`; `0x005CA720..0x005CA7AB`; prior `0x007B6880` decompile | current Rust does not model Siege, which is fine for stock local roster | `src/skirmish_modes.rs`, future custom mode support | keep Siege out of stock offline Skirmish unless data supplies it; if added, model its role checks separately | stock mode list has no Siege row; custom Siege invalid roles produce Siege-specific message handling, not the generic `0x617` modal unless a separate output code proves it | Do not expose Siege just to exercise generic rejection; do not model Siege output as literal `0x617` |

## 10. Negative Facts / Do Not Do

- Do not pre-disable Start for invalid selected-mode or ordinary setup. Active in YR: Yes; Start is click-to-validate. Evidence: this report's scope inherits settled button-state reports; selected-mode validation only runs after command `0x617`.
- Do not treat every selected-mode false return as a rejection modal. Active in YR: Yes/Conditional. Evidence: `0x006AD2D9..0x006AD346`.
- Do not use `0x469` as a modal body string. Active in YR: Conditional. Evidence: `0x006AD2E9..0x006AD316`; prior CSF report shows `0x469 -> TXT_OK`.
- Do not add selected-mode Start rejections for stock Battle, Team Game, ManBattle, FreeForAll, or Cooperative. Active in YR: Yes/Conditional by selected row. Evidence: `0x005D6310`, `0x005C5D40`, `0x005C1D80`, `ini/mpmodesmd.ini`.
- Do not reject stock Unholy solely because its callback can return false; the caller's blocking gate also requires output dword `0x617`, and Unholy false writes no output. Active in YR: Conditional. Evidence: `0x005CB400..0x005CB421`, `0x006AD2D9..0x006AD346`.
- Do not expose Siege in stock local offline Skirmish without data evidence. Active in YR: No for stock local roster. Evidence: no `[Siege]` section in `ini/mpmodesmd.ini`.
- Do not model Siege role failures as literal `0x617`. Active in YR: Conditional. Evidence: `0x005CA720..0x005CA7AB` calls `0x007B6880` with message strings.
- Do not move selected-mode callback before ordinary capacity/min-player/same-team checks. Active in YR: Yes. Evidence: ordinary branches `0x006AD05B..0x006AD2A7` precede `0x006AD2BA`.

## 11. Remaining Uncertainty

- Current Ghidra MCP did not expose direct decompile bodies in this pass; no function boundaries were created because the session is read-only. The report uses fresh assembly contexts plus existing decompile-backed reports for the same addresses.
- Custom/modded MPModes objects could intentionally write output dword `0x617`; this is out of scope for stock offline Skirmish.
- The exact UI path that sets/clears Unholy `DAT_00A8B258` is adjacent setup/session behavior, not needed to prove the callback result contract.
- Native modal pixels/resource rectangles for the generic selected-mode rejection remain visual parity work.

## Sources

- Fresh Ghidra read-only assembly contexts from this pass: `0x006AD073`, `0x006AD0A7`, `0x006AD0F8`, `0x006AD126`, `0x006AD243`, `0x006AD274`, `0x006AD2BA`, `0x006AD2D2`, `0x006AD2E1`, `0x006AD2E9`, `0x006AD302`, `0x006AD30C`, `0x006AD316`, `0x006AD346`, `0x006AD34B`, `0x005D6310`, `0x005C5D40`, `0x005C5D51`, `0x005C1D80`, `0x005C1D8D`, `0x005C1DA5`, `0x005CB400`, `0x005CB417`, `0x005CB41F`, `0x005CA6D0`, `0x005CA6E9`, `0x005CA6FF`, `0x005CA70C`, `0x005CA712`, `0x005CA73A`, `0x005CA748`, `0x005CA76C`, `0x005CA782`, `0x005CA790`, `0x005CA7A9`, `0x005CA7B9`.
- Fresh Ghidra string search: `MPModesMD.ini` `0x00830A18`; mode strings `0x00830BCC..0x00830C00`; Siege message strings `0x0082FEE8`, `0x0082FEF8`, `0x0082FF0C`, `0x0082FF1C`, source path `0x0082FF2C`; `TXT_OK` `0x00825FB0`.
- Existing decompile-backed reports referenced: `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_SESSION_PACKING_BROAD_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_COOPERATIVE_PRECALL_0049B760_GHIDRA_REPORT.md`, `SKIRMISH_MODE_ROLE_UI_NODE_0X6B_GHIDRA_REPORT.md`.
- INI checked: `ini/mpmodesmd.ini`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/skirmish_launch.rs`, `src/skirmish_modes.rs`.
