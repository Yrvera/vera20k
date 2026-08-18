# Skirmish Start Validation Failure UI - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005D3490`, `0x006AE2C0`, `0x006AE3F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish dialog `0x102` Start Game command `0x617` validation failure UI: map capacity, minimum players, same explicit team, selected mode `+0x14` false path, modal helper shape, and Start button disable/re-enable timing.  
**Non-Scope:** successful launch packing after `0x006AD34B`, Choose Map modal behavior, post-shell spawn generation, mode-object construction defaults, and full CSF/localized text extraction.  
**Confidence:** High for binary control flow, string IDs, modal helper behavior, and Rust deltas; Medium for human-readable meaning of numeric string IDs because this slice did not decode the retail string table text.  
**Active in YR:** Yes for ordinary offline Skirmish `0x102` validation; selected-mode false handling is Conditional by selected MPModes object and its return/output contract.

## Working Notes Required By Swarm Slot

- Target question: What does `FUN_006ACEE0` show/do when offline Skirmish Start Game `0x617` validation fails?
- Non-goals: Do not re-investigate successful packing, Choose Map, preview decode, map loader, or spawn generation except as failure-flow boundaries.
- Evidence needed to mark COMPLETE: decompile plus assembly-context evidence for each failure branch, string IDs/keys where binary-visible, modal helper call shape, Start disable/re-enable order, selected-mode false output handling, and Rust handoff tests.
- Stop conditions: stop after every `0x617` validation failure branch in `0x006ACEE0` is either verified or explicitly deferred; do not chase gameplay startup or mode constructors.

## 1. Overview

Pressing Start Game disables button `0x617` before validation. The ordinary validation failures show a shell modal using `FUN_005D3490`, re-enable Start, and return before the dialog result pointer is written. The modal receives two text strings: a formatted/detail string in the first visible text control and a second loaded string in the second visible text control; no explicit button-label strings are passed by these failure paths.

Selected mode rejection has a narrower block condition than prior summaries implied. `0x006ACEE0` blocks and shows string ID `0x469` only when the selected mode `+0x14` returns false **and** the local output dword equals `0x617`. If the false-return output is any other value, the code calls `0x005D5E10` and then continues into launch packing. For stock Battle/ManBattle the mode method accepts; for Siege/Unholy false paths seen in prior work, the output is not proven to be `0x617`.

## 2. Modal Helper And UI Contract

| Behavior | Evidence | Active in YR |
|---|---|---|
| `0x006AE2C0` creates dialog `0x102`, stores a pointer to a local result at `GWL_USERDATA` offset `8`, pumps until result `0x617` or `0x5C0`, and returns true only for `0x617`. | decompile `0x006AE2C0` | Yes, standard offline Skirmish launcher |
| `0x006AE3F0` routes `WM_COMMAND (0x111)` to `0x006ACEE0`; `0x006ACEE0` ignores Start/Back unless notification high word is `0`. | decompile `0x006AE3F0`; Start gate `0x006ACF7B..0x006ACF92` in decompile | Yes |
| Start Game disables button `0x617` immediately before counting rows and validations. | decompile `0x006ACF92..0x006ACF9E` | Yes |
| `FUN_005D3490` constructs a modal shell message dialog, writes `param_1` to child `0x5B0` and `param_2` to child `0x5AE` with message `0x4B2`, optionally writes button text to controls `2` and `0x5AF` only if `param_3/4` are non-empty, then pumps until the modal result changes. | decompile `0x005D3490`; assembly context `0x005D3490..0x005D353A` | Yes |
| These Start failure paths pass only the first two text strings and zeros for `param_3/4`; they do not customize button labels. | assembly contexts `0x006AD0A7..0x006AD0C6`, `0x006AD126..0x006AD145`, `0x006AD274..0x006AD293`, `0x006AD2E3..0x006AD316` | Yes |
| `FUN_007B66C0` initializes the local selected-mode output/string object by setting its first dword to `0`; `FUN_007B6760` frees nonzero first dword and zeros it; `FUN_007B6880` frees/replaces the first dword with an allocated UTF-16 copy of a passed string. | decompile `0x007B66C0`, `0x007B6760`, `0x007B6880` | Yes for this local helper family |

## 3. Failure Branches

### 3.1 Map capacity failure

The handler counts active AI rows first. Rows whose row-type combo item data is `0`, `1`, or `2` count as active; active count is stored in `DAT_00A8B274`. Then for Start only, the selected map capacity from `0x005E6520(DAT_00A8B254)` is compared to `active_ai_rows + 1`.

If `capacity < active_ai_rows + 1`, the UI path is:

1. Load string ID `0x437` from source string `D:\ra2mdpost\Skirmish.cpp` and format it with the map capacity into the local text buffer.
2. Load string ID `0x438`.
3. Call `FUN_005D3490(formatted_0x437, string_0x438, 0, 0)`.
4. Re-enable Start with `EnableWindow(GetDlgItem(hwnd, 0x617), 1)`.
5. Return before `0x006AD34B` packing and before writing the dialog result pointer.

Evidence: decompile `0x006ACFBD..0x006AD0DA`; assembly contexts `0x006AD073..0x006AD090`, `0x006AD0A7..0x006AD0C6`, `0x006AD0CB..0x006AD0DA`.  
Active in YR: Yes. This is the ordinary offline Skirmish Start validation path.

### 3.2 Fewer than two total players

If `active_ai_rows + 1 <= 1`, the UI path is:

1. Load string ID `0x43F` and format it into the local text buffer.
2. Load string ID `0x440`.
3. Call `FUN_005D3490(formatted_0x43F, string_0x440, 0, 0)`.
4. Re-enable Start `0x617`.
5. Return before packing/result write.

Evidence: decompile `0x006AD0ED..0x006AD159`; assembly contexts `0x006AD0F2..0x006AD10F`, `0x006AD126..0x006AD145`, `0x006AD14A..0x006AD159`.  
Active in YR: Yes.

### 3.3 All active players on the same explicit team

The same-team validation runs only when local team control `0x76D` returns an explicit nonnegative value through `FUN_004E6030(hwnd, 0x76D, -1)`. Team `None` / negative skips this validation entirely.

When local team is explicit, the handler scans active AI rows. For each active AI row, it reads the row's team control `0x76E..0x774` via `FUN_004E5940(row+1)` and `FUN_004E6030(hwnd, team_control, -1)`, then re-reads local `0x76D`. If it finds any active AI team different from the local team, validation passes. If all active AI teams equal the local team, the UI path is:

1. Load string ID `0x457` and format it into the local text buffer.
2. Load string ID `0x458`.
3. Call `FUN_005D3490(formatted_0x457, string_0x458, 0, 0)`.
4. Re-enable Start `0x617`.
5. Return before packing/result write.

Evidence: decompile `0x006AD16C..0x006AD2A7`; assembly contexts `0x006AD16C..0x006AD22A`, `0x006AD242..0x006AD25D`, `0x006AD274..0x006AD293`, `0x006AD298..0x006AD2A7`; helper decompile `0x004E5940`, `0x004E6030`.  
Active in YR: Yes. This is team validation, not start-position collision validation.

### 3.4 Selected MPModes `+0x14` false path

After the three dialog-level validations, `0x006ACEE0` initializes a local output object at stack `local_248`, calls the selected mode object at `DAT_00A8B23C` vtable `+0x14`, and branches on the return byte.

If the mode method returns true, packing begins at `0x006AD34B`.

If the mode method returns false and the local output dword equals `0x617`, the UI path is:

1. Load string ID `0x469`.
2. Resolve the local output object through `FUN_007B7100`; null output becomes the empty string at `0x00887734`.
3. Call `FUN_005D3490(mode_output_string, string_0x469, 0, 0)`.
4. Re-enable Start `0x617`.
5. Free the local output object with `FUN_007B6760`.
6. Return before packing/result write.

If the mode method returns false and the local output dword is not `0x617`, the handler calls `0x005D5E10` and then falls through to packing at `0x006AD34B`; it does not re-enable Start or return on this path.

Evidence: decompile `0x006AD2BA..0x006AD34B`; assembly contexts `0x006AD2BA..0x006AD343` and `0x006AD346..0x006AD34B`; helper decompile `0x007B7100`; `0x005D5E10` decompile shows a global object/list accessor whose return is unused by this caller.  
Active in YR: Conditional. The dispatch is live for offline Skirmish, but the blocking modal subpath requires a selected mode method to return false with output dword exactly `0x617`.

## 4. Selected Mode Rejection Cross-Check

| Mode family | `+0x14` behavior relevant to this UI slice | Blocks in `0x006ACEE0`? | Evidence | Active in YR |
|---|---|---|---|---|
| Battle / ManBattle | shared method `0x005D6310` returns true unconditionally | No failure | prior `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`; vtable call site rechecked at `0x006AD2D2` | Yes for standard modes |
| Siege | method at `0x005CA6D0` can return false and writes localized `MP:NoDefender`, `MP:OnlyOneBeseiged`, `MP:IllegalTeam`, or `MP:NoAttackers` into the output object using `0x007B6880` | Not proven to block; written output is a heap string pointer, not `0x617`, so caller's observed branch would fall through unless another unverified wrapper changes the output contract | assembly contexts `0x005CA720..0x005CA7B6`, `0x006AD2D9..0x006AD346`; prior mode report | Conditional; binary support exists, exposed local `mpmodesmd.ini` lacks `[Siege]` |
| Unholy | method at `0x005CB400` returns false when `DAT_00A8B258 == 0`, with no output write | Not via generic `0x469` modal because output remains `0`, not `0x617`; caller falls through to `0x005D5E10` then packing | assembly context `0x005CB400..0x005CB421`, caller `0x006AD2D9..0x006AD346`; prior mode report | Conditional by selected Unholy and global byte |
| FreeForAll / Cooperative | accept in the prior report, with side effects only | No failure | prior mode report | Conditional by selected mode |

The practical implementation point is that Rust should implement the ordinary three validation modals now. For selected-mode rejection UI, do not assume every false `+0x14` return blocks the Start button; the verified blocking condition is stricter.

## 5. Current Rust Implementation Status

Rust now builds a `SkirmishLaunchSession` in `src/ui/skirmish_shell/state.rs`.
As of 2026-05-23, current Rust has data-level capacity, no-opponent, and
same-explicit-team validation, maps those failures to
`SkirmishValidationModalState` in `src/app.rs`, consumes OK clicks, and renders
a primitive validation modal. Remaining deltas are native modal art/text/layout
parity, the non-native capacity text suffix, Start disable/re-enable timing, and
the future selected-mode `+0x14` output contract.

| Rust surface | Current status | Delta vs binary |
|---|---|---|
| `src/ui/skirmish_shell/state.rs::launch_session` | rejects missing selected map, no enabled opponent, map capacity overflow, same explicit team, invalid color, and invalid start | still lacks selected-mode `+0x14` output contract |
| `src/app.rs::handle_skirmish_shell_action` | maps launch validation errors to app-level `SkirmishValidationModalState` | modal is primitive/non-native; Start pressed state is cleared by mouse-up rather than the native disable/re-enable timing |
| `src/app_skirmish_shell_render.rs` | renders validation modal instances and text | native `0x005D3490` art/text/layout parity and exact CSF formatting remain incomplete |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x006AE2C0` modal loop/result contract | verified | decompile `0x006AE2C0` | none |
| `0x006AE3F0` `WM_COMMAND` dispatch | verified | decompile `0x006AE3F0` | none |
| Start notification gate and initial disable | verified | decompile `0x006ACF7B..0x006ACF9E` | none |
| active AI row count | verified | decompile `0x006ACFBD..0x006AD052` | row label display already covered elsewhere |
| map capacity failure UI | verified | `0x006AD05B..0x006AD0DA`, assembly contexts listed above | CSF text content not decoded |
| minimum players failure UI | verified | `0x006AD0ED..0x006AD159`, assembly contexts listed above | CSF text content not decoded |
| same explicit team failure UI | verified | `0x006AD16C..0x006AD2A7`, helpers `0x004E5940`, `0x004E6030` | CSF text content not decoded |
| selected mode `+0x14` blocking modal condition | verified | `0x006AD2BA..0x006AD343`, `0x007B66C0`, `0x007B6760`, `0x007B7100` | concrete non-stock mode that writes output dword `0x617`, if any |
| selected mode false fallthrough | verified | `0x006AD346..0x006AD34B`; `0x005D5E10` decompile | whether this is reachable in retail UI without custom mode data |
| successful packing | not-touched | out of scope; prior packing report | separate reports own this |
| full localized text strings | deferred | string IDs verified, text table not decoded in this slice | dedicated CSF/StringTable extraction if human text is needed |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the exact target slice?` -> Start Game `0x617` failure UI in offline Skirmish `0x102`, not successful packing. (evidence: user scope; `0x006ACEE0`)
- `[RESOLVED] OQ-02 - Is this path live in YR?` -> Yes, `0x006AE2C0` creates/pumps the offline Skirmish dialog and `0x006AE3F0` sends `WM_COMMAND` into `0x006ACEE0`. (evidence: decompile `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-03 - What disables Start?` -> Start `0x617` with notification `0` calls `EnableWindow(GetDlgItem(hwnd,0x617),0)` before validations. (evidence: `0x006ACF92..0x006ACF9E`)
- `[RESOLVED] OQ-04 - Which rows count as active opponents?` -> row item data `0`, `1`, or `2` from controls `0x50B/0x50E/0x516/0x51A/0x51B/0x51C/0x51D`. (evidence: `0x006ACFBD..0x006AD052`)
- `[RESOLVED] OQ-05 - What happens on map capacity failure?` -> modal strings `0x437/0x438`, Start re-enabled, return before packing. (evidence: `0x006AD05B..0x006AD0DA`)
- `[RESOLVED] OQ-06 - What happens on fewer than two players?` -> modal strings `0x43F/0x440`, Start re-enabled, return before packing. (evidence: `0x006AD0ED..0x006AD159`)
- `[RESOLVED] OQ-07 - What happens when all active explicit teams match?` -> modal strings `0x457/0x458`, Start re-enabled, return before packing. (evidence: `0x006AD16C..0x006AD2A7`)
- `[RESOLVED] OQ-08 - Does Team None participate in same-team failure?` -> No; local team `< 0` skips the same-team scan. (evidence: `0x006AD16C..0x006AD17C`)
- `[RESOLVED] OQ-09 - What modal helper is used?` -> `FUN_005D3490`, with first two text controls `0x5B0` and `0x5AE`. (evidence: decompile `0x005D3490`)
- `[RESOLVED] OQ-10 - Are explicit button labels passed?` -> No for scoped failure paths; `param_3/4` are zero. (evidence: failure call assembly contexts)
- `[RESOLVED] OQ-11 - What is the selected-mode blocking condition?` -> false return plus local output dword exactly `0x617`. (evidence: `0x006AD2D5..0x006AD2E1`)
- `[RESOLVED] OQ-12 - What does selected-mode generic modal show?` -> mode output string from `FUN_007B7100(local)` and string ID `0x469`, then re-enables Start and frees local output. (evidence: `0x006AD2E3..0x006AD334`, `0x007B7100`)
- `[RESOLVED] OQ-13 - What happens when mode false output is not `0x617`?` -> `0x005D5E10` is called and packing continues. (evidence: `0x006AD346..0x006AD34B`)
- `[RESOLVED] OQ-14 - Do stock Battle/ManBattle reject?` -> No, their `+0x14` accepts unconditionally. (evidence: prior mode report plus caller recheck)
- `[RESOLVED] OQ-15 - Is same-team validation start-position collision?` -> No, it reads team controls `0x76D..0x774`. (evidence: `0x004E5940`, `0x006AD16C..0x006AD2A7`)
- `[DEFERRED] OQ-16 - What is the English/German/etc text for string IDs `0x437/0x438/0x43F/0x440/0x457/0x458/0x469`?` (category: bounded-cost-too-high; reason: binary IDs are verified but this slot did not decode the retail StringTable/CSF text; next-step-if-pursued: run a focused string-table extraction against `ra2md.csf`/internal StringTable)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Map capacity failure blocks Start, shows modal string pair `0x437/0x438`, re-enables Start, and stays in shell | `0x006AD05B..0x006AD0DA`; `0x005D3490` | partially implemented: Rust has validation and primitive modal, but capacity text formatting/art/layout are not native | `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | Preserve no-launch behavior and replace primitive/non-native modal formatting with native-visible text/layout | selected map capacity `2`, local + 2 enabled AIs: shell remains visible, no pending launch session, Start becomes usable again | Do not treat invalid map capacity as `NoSelectedMap`; proposed test `skirmish_start_rejects_map_capacity_with_native_modal` |
| Fewer than two total players blocks with string pair `0x43F/0x440` | `0x006AD0ED..0x006AD159` | partially implemented: `NoEnabledOpponent` maps to validation modal state, but native modal parity remains incomplete | same | Preserve visible modal behavior and native CSF/body text | default map, all AI rows disabled: no loading transition, visible error category is min players, Start usable after dismissal | Do not reduce this to a warning log; proposed test `skirmish_start_no_opponent_shows_min_players_modal` |
| Local explicit team and all active AI on same team blocks with string pair `0x457/0x458`; local Team None skips the check | `0x006AD16C..0x006AD2A7`; helpers `0x004E5940`, `0x004E6030` | partially implemented: `SameExplicitTeam` exists and maps to validation modal state, but native modal parity remains incomplete | `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | Preserve same-explicit-team validation and native-visible modal text/layout | local team 1, two enabled AIs team 1: blocked; local Team None with AIs team 1: not blocked by this check | Do not confuse this with start-position collision; proposed test `skirmish_start_rejects_all_players_same_explicit_team` |
| Selected-mode false return blocks only if output dword is `0x617`; otherwise caller falls through after `0x005D5E10` | `0x006AD2BA..0x006AD34B`; `0x007B66C0/6760/6880/7100` | unchecked/missing mode model | future MPModes model plus `launch_session` validation | Preserve the stricter native condition when adding mode acceptance UI | synthetic mode acceptance returning false with output `0x617` blocks and shows mode-output + `0x469`; false with non-`0x617` must not be modeled as the same blocking UI without more evidence | Do not make every mode false return a blocking modal; proposed test `skirmish_mode_rejection_blocks_only_native_start_code` |

## 9. Negative Facts / Do Not Do

- Do not implement the same-team branch as start-position collision. Active in YR: Yes; evidence `0x006AD16C..0x006AD2A7` reads team controls `0x76D..0x774`.
- Do not transition to loading on ordinary validation failures. Active in YR: Yes; evidence all three ordinary failures return before `0x006AD8D3` result write.
- Do not leave Start disabled after a blocking failure. Active in YR: Yes; evidence re-enable calls at `0x006AD0CB..0x006AD0DA`, `0x006AD14A..0x006AD159`, `0x006AD298..0x006AD2A7`, `0x006AD31B..0x006AD32A`.
- Do not reduce validation errors to logs. Active in YR: Yes; evidence `0x005D3490` modal calls in every ordinary failure branch.
- Do not assume all selected mode false returns block with `0x469`. Active in YR: Conditional; evidence `0x006AD2D9..0x006AD346` gates the modal on local output dword `0x617`.

## 10. Remaining Uncertainty

- Exact localized text for numeric string IDs `0x437/0x438/0x43F/0x440/0x457/0x458/0x469` was not decoded; IDs and call sites are verified.
- Whether any retail/custom selected mode `+0x14` method writes output dword exactly `0x617` remains open; stock Battle/ManBattle do not reject, and checked Siege/Unholy false paths do not prove that output.

## Stale Docs / Replacement Wording

- Replace "If the selected MPModes `+0x14` returns false, Start is re-enabled after a string ID `0x469` modal" with: "If selected MPModes `+0x14` returns false, `0x006ACEE0` shows the `0x469` modal and re-enables Start only when the local output dword equals `0x617`; false returns with other output fall through through `0x005D5E10` into packing."
- Keep the prior correction that `0x006AD16C..0x006AD2A7` is same explicit team validation, not start-position collision validation.

## Sources

- Ghidra decompiled/read: `0x006ACEE0`, `0x006AE2C0`, `0x006AE3F0`, `0x005D3490`, `0x005D5E10`, `0x005E6520`, `0x004E5940`, `0x004E6030`, `0x007B66C0`, `0x007B6760`, `0x007B6880`, `0x007B7100`, `0x005CA630`, disassembly contexts for `0x005CA6D0..0x005CA7BF` and `0x005CB400..0x005CB421`.
- Ghidra assembly contexts: `0x006AD05B..0x006AD0DA`, `0x006AD0ED..0x006AD159`, `0x006AD16C..0x006AD2A7`, `0x006AD2BA..0x006AD34B`, `0x006AD8C7..0x006AD8D5`, `0x005D3490..0x005D353A`, `0x005CA720..0x005CA7B6`, `0x005CB400..0x005CB421`.
- Prior reports referenced for context and contradiction checks: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`, `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`, `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`.
