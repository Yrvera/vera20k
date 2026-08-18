# In-Game Options Dialog Proc 0x004E1FE0 Init/Persist Path - Ghidra Research Report

**Address(es):** `0x004E1D00` (`OptionsClass::ShowInGameDialog`), `0x004E1FE0` (own dialog proc), `0x004E1DE0` (`ApplyFromInGameDialog`), `0x005FAD10` (`WriteToINI`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** in-game/shell Options own dialog launch, proc messages, init, slider/check apply, result codes, and persistence path.  
**Non-Scope:** owner-draw paint assets/chrome composition; generic message-box modal family; launcher options proc `0x0055FDB0` except as prior context.  
**Confidence:** High for launch/proc/apply/write; Medium for resource-layout transcription where raw templates were parsed from bytes but not re-rendered.  
**Active in YR:** Conditional. Active in standard YR when `State_Machine` enters game state 5; active-game template only when `g_GameActive byte == 1`, shell template otherwise.

## Working Notes Required Before Investigation

- Target question: What exact behavior does the in-game Options own dialog path implement from `ShowInGameDialog` through proc init/commands/scrolls, apply, and INI persistence?
- Non-goals: Do not study full chrome paint assets, do not implement Rust, do not mutate Ghidra, do not rewrite other docs outside the allowed report/claims file.
- Evidence needed to mark COMPLETE: decompile plus disassembly for `0x004E1D00`, `0x004E1FE0`, `0x004E1DE0`, `0x005FAD10`; caller/xref proof of liveness; template byte proof for `0xBBB` and `0xF5`; Rust scan and concrete acceptance handoff.
- Stop conditions: Stop when every scoped proc branch, affected control, active-game gate, slider range/inversion, checkbox global, result code, and persistence key is resolved or explicitly deferred.

## 1. Overview

The in-game Options path is not the generic CSF message-box helper and not the main-menu launcher options proc. `State_Machine @ 0x0048C8B0` enters case 5 and calls `OptionsClass::ShowInGameDialog @ 0x004E1D00`, which creates either RT_DIALOG `0xBBB` or `0xF5` with own proc `0x004E1FE0`.

Result handling is the key implementation boundary: the caller initializes a result slot to `-1`, stores its address in `GWLP_USER` offset 8, pumps `0x00623120` until the slot changes, and persists only when the final result equals `1`. Result `2` is caller-generated when the pump reports game end; it skips `ApplyFromInGameDialog` and `WriteToINI`.

## 2. Entry/Liveness

| Area | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| State-machine entry | case 5 calls `0x004E1D00`; if state remains 5 after return, state is reset to 1 | decompile `0x0048C8B0`; disasm `0x0048C9C9..0x0048C9E6`; caller xref to `0x004E1D00` | Yes, when in-game state machine reaches state 5 |
| Template selection | reads byte `0x00A8E9A0`; exactly `== 1` selects `0xBBB`, else `0xF5`; proc pointer is `0x004E1FE0` | decompile `0x004E1D00`; disasm `0x004E1D2A..0x004E1D47` | Conditional: active-game byte `1` vs all other values |
| Result storage | `SetWindowLongA(hwnd, 8, &local_4)` after create/show; `local_4` starts `-1` | decompile `0x004E1D00`; disasm `0x004E1D52..0x004E1D62` | Yes |
| Pump result 2 | while result is `-1`, `0x00623120` returning `1` writes result `2` | decompile `0x004E1D00`; disasm `0x004E1D70..0x004E1D81`; pump return proof `0x0062314E..0x00623159` | Conditional: only when `Main_Tick` returns nonzero |
| Persist path | result `1` calls `ApplyFromInGameDialog`, then `WriteToINI` with ECX `0x00A8EB60` | decompile `0x004E1D00`; disasm `0x004E1D9A..0x004E1DAB`; xrefs to `0x004E1DE0` and `0x005FAD10` | Yes |

## 3. Dialog Resources And Affected Controls

Template bytes were read from `0x00C01B18` (`0xBBB`, plain DLGTEMPLATE, 17 controls) and `0x00BF9F58` (`0xF5`, DLGTEMPLATEEX, 20 controls). This report uses the templates only to identify controls and rect differences; paint assets remain out of scope.

### `0xBBB` active-game template

Affected controls in the proc/apply path:

| ID | Resource role | DLU rect | Proc/apply behavior | Active in YR |
|---|---|---:|---|---|
| `0x686` | Back button | `(425,346,108,23)` | WM_COMMAND sets result `1` unconditionally | Yes |
| `0x52C` | Keyboard button | `(425,149,108,23)` | WM_COMMAND hiword `0` and active byte `==1`: set `g_GameState=4`, result `1` | Conditional: active-game only |
| `0x52D` | Sound button | `(425,122,108,23)` | WM_COMMAND hiword `0` and active byte `==1`: set `g_GameState=6`, result `1`; init enables it via `0x00407000` | Conditional: active-game only |
| `0x529` | GameSpeed trackbar | `(144,100,128,13)` | range `0..6`, init/apply inverted as `6 - field/pos`; label `0x671` | Yes, unless hidden by gates |
| `0x52A` | ScrollRate trackbar | `(144,131,128,13)` | range `0..6`, init/apply inverted as `6 - field/pos`; label `0x672` | Yes |
| `0x52B` | VisualDetails trackbar | `(144,162,128,13)` | range `0..2`, direct field/pos; label `0x673` | Yes |
| `0x601` | TargetLines checkbox | `(89,206,119,10)` | `BM_GETCHECK/SETCHECK`; writes Options `+0x1E` (`UnitActionLines`) | Yes |
| `0x604` | ShowHidden checkbox | `(89,224,119,10)` | `BM_GETCHECK/SETCHECK`; writes Options `+0x1F` (`ShowHidden`) | Yes |
| `0x602` | Tooltips checkbox | `(214,206,127,10)` | `BM_GETCHECK/SETCHECK`; writes Options `+0x20` (`ToolTips`) and updates tooltip manager if active | Yes |

Other visible statics used by proc: `0x714` GameSpeed caption, `0x671/0x672/0x673` dynamic labels, `0x694` title, `0x695` blank footer.

### `0xF5` shell/inactive template

`0xF5` is not a derived copy of `0xBBB`. It has wider and repositioned sliders (`cx=148`), plus shell-only controls:

| ID | Resource role | DLU rect | Proc/apply behavior | Active in YR |
|---|---|---:|---|---|
| `0x686` | Back button | `(425,346,108,23)` | WM_COMMAND sets result `1` unconditionally | Conditional: shell/inactive dialog |
| `0x529` | GameSpeed trackbar | `(138,82,148,13)` | same range/inversion as active template | Conditional |
| `0x52A` | ScrollRate trackbar | `(138,109,148,13)` | same range/inversion as active template | Conditional |
| `0x52B` | VisualDetails trackbar | `(138,163,148,13)` | same range/direct mapping as active template | Conditional |
| `0x50F` | Difficulty trackbar | `(138,136,148,13)` | range `0..2`, direct field/pos, initialized/applied only when active byte is zero/non-`==1` as described below | Conditional: shell/inactive only |
| `0x51A` | ScrollCoasting checkbox | `(204,209,128,10)` | present in template but not referenced by `0x004E1FE0` or `0x004E1DE0` in this slice | Conditional resource-only in this path |
| `0x71C` | static/bitmap placeholder | `(448,38,61,33)` | present in template; no proc/apply reference in this slice | Conditional resource-only in this path |

The `0xF5` template does not contain `0x52C` or `0x52D`; if those command IDs were sent anyway, the own proc also gates them on `g_GameActive == 1`.

## 4. Proc Message Semantics

The proc first delegates to the common shell dialog proc `0x00622B50` and returns that nonzero result. Its own dispatch then handles only `WM_COMMAND (0x111)`, `WM_HSCROLL (0x114)`, and custom init `0x497`. Evidence: decompile `0x004E1FE0`; disasm `0x004E1FF4..0x004E2022`.

### `WM_COMMAND (0x111)`

| Control | Predicate | Side effects | Evidence | Active in YR |
|---|---|---|---|---|
| `0x52C` | hiword `0`, active byte `==1` | `g_GameState = 4`; result slot `=1` | disasm `0x004E233A..0x004E23AD` | Conditional: active-game |
| `0x52D` | hiword `0`, active byte `==1` | `g_GameState = 6`; result slot `=1`; returns immediately | disasm `0x004E2370..0x004E2393` | Conditional: active-game |
| `0x686` | no hiword or active-byte test | result slot `=1`; returns immediately | disasm `0x004E2359..0x004E236D` | Yes for both templates |

No button in this proc writes a non-persist close result. Result `2` is not a button result; it is written by the caller when the pump reports game end.

### `WM_HSCROLL (0x114)`

The proc responds only when the low word of `wParam` equals `5` (`CMP DI,0x5`). It then shifts the high word into an index and uses control-specific CSF pointer tables.

| Sender HWND | Label control | String table base | Label sequence by index | Evidence | Active in YR |
|---|---|---|---|---|---|
| `0x529` GameSpeed | `0x671` | `0x00822730` | visual `0..6`: `TXT_SLOWEST`, `TXT_SLOWER`, `TXT_SLOW`, `TXT_MEDIUM`, `TXT_FAST`, `TXT_FASTER`, `TXT_FASTEST` | disasm `0x004E2288..0x004E22A3`; memory `0x00822730`, strings `0x008227B8..0x00822800` | Yes |
| `0x52A` ScrollRate | `0x672` | `0x0082274C` | same `SLOWEST..FASTEST` order | disasm `0x004E22A5..0x004E22BD`; memory `0x0082274C` | Yes |
| `0x52B` VisualDetails | `0x673` | `0x00822768` | `TXT_LOW`, `TXT_MEDIUM`, `TXT_HIGH` | disasm `0x004E22BF..0x004E22D7`; strings `0x008227A4`, `0x008227B0`, `0x008227DC` | Yes |
| `0x50F` Difficulty | `0x670` | `0x00822774` | `TXT_EASY`, `TXT_NORMAL`, `TXT_HARD` | disasm `0x004E22D9..0x004E2302`; strings `0x00822780`, `0x0082278C`, `0x00822798` | Conditional: skipped when active byte is nonzero |

The selected CSF key pointer is passed to `StringTable::LoadString` with source string `D:\ra2mdpost\GameDlg.CPP` and id `0x1B2`, then the target static receives message `0x4B2`. Evidence: disasm `0x004E230F..0x004E232B`; source string memory `0x00822848`.

### Custom init `0x497`

| Control(s) | Init behavior | Evidence | Active in YR |
|---|---|---|---|
| `0x529`, `0x714`, `0x671` | if `g_GameMode == 0` and byte `0x00A8EDDC == 0`, hide all three | disasm `0x004E2028..0x004E2079` | Conditional |
| `0x529` | otherwise send `0x4AC(0,0)`, `TBM_SETRANGE 0x406(w=1,l=0x00060000)`, `TBM_SETPOS 0x405(w=1,l=6-GameSpeed)` | disasm `0x004E207B..0x004E20B8` | Conditional |
| `0x529`, `0x714`, `0x671` | if `0x00A8B538 != 0`, hide all three; the binary repeats the same hide trio twice | disasm `0x004E20BA..0x004E211A` | Conditional |
| `0x52A` | range `0..6`, pos `6-ScrollRate` | disasm `0x004E211A..0x004E2159` | Yes |
| `0x52B` | range `0..2`, pos `DetailLevel` | disasm `0x004E215B..0x004E2193` | Yes |
| `0x601`, `0x604`, `0x602` | `BM_SETCHECK (0xF1)` to nonzero value of Options bytes `+0x1E/+0x1F/+0x20` | disasm `0x004E2195..0x004E2201` | Yes |
| `0x52D` | when active byte `==1`, `EnableWindow(0x52D, FUN_00407000())`; returns after this if the control exists | disasm `0x004E2201..0x004E222F` | Conditional: active-game |
| `0x50F` | otherwise range `0..2`, pos `Difficulty` | disasm `0x004E2232..0x004E226F` | Conditional: shell/inactive |

Note the active discriminator is not uniformly expressed: template selection uses `== 1`; difficulty scroll/apply uses zero/nonzero tests in places; init uses `==1` for the sound-button branch and falls to difficulty otherwise.

## 5. ApplyFromInGameDialog

`0x004E1DE0` reads controls by `GetDlgItem`, ignores absent controls, and uses `SendMessageA` for `TBM_GETPOS (0x400)` or `BM_GETCHECK (0xF0)`.

| Control | Field/global written | Conversion/side effects | Evidence | Active in YR |
|---|---|---|---|---|
| `0x529` | Options `+0x00` / `0x00A8EB60` (`GameSpeed`) | desired internal value is `6 - pos`; if changed and active byte `==1` and `g_GameMode` is neither `0` nor `5`, queue event `0x0D` if queue count `<0x80` and do not immediately store; otherwise store directly | decompile `0x004E1DE0`; disasm `0x004E1DEF..0x004E1EC0` | Yes; network branch conditional |
| `0x52A` | Options `+0x10` / `0x00A8EB70` (`ScrollRate`) | direct store `6 - pos` | disasm `0x004E1EC0..0x004E1EE2` | Yes |
| `0x52B` | Options `+0x18` / `0x00A8EB78` (`DetailLevel`) | direct store `pos`; if changed, call `0x004AE450` | disasm `0x004E1EE8..0x004E1F15` | Yes |
| `0x601` | Options `+0x1E` / `0x00A8EB7E` (`UnitActionLines`) | set byte to `(BM_GETCHECK == 1)`; call `0x0070D180` | disasm `0x004E1F1A..0x004E1F41`; corroborates `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md` | Yes |
| `0x604` | Options `+0x1F` / `0x00A8EB7F` (`ShowHidden`) | set byte to `(BM_GETCHECK == 1)` | disasm `0x004E1F46..0x004E1F67` | Yes, debug/option byte |
| `0x602` | Options `+0x20` / `0x00A8EB80` (`ToolTips`) | set byte to `(BM_GETCHECK == 1)`; if tooltip manager pointer non-null and active byte `==1`, call `0x007241A0` with the new byte | disasm `0x004E1F6D..0x004E1FA9` | Yes |
| `0x50F` | Options `+0x04` / `0x00A8EB64` (`Difficulty`) | direct store `pos`, only when active byte is zero and the control exists | disasm `0x004E1FAE..0x004E1FD2` | Conditional: shell/inactive only |

No read/write of `0x51A` (`ScrollCoasting`) was found in `0x004E1FE0` or `0x004E1DE0`.

## 6. INI Read/Write Fields

`WriteToINI @ 0x005FAD10` constructs `RA2MD.INI` using string `0x00826444`, then writes the full Options object, not only the controls changed by this dialog. Evidence: disasm `0x005FAD19..0x005FB00E`, file string memory `0x00826444`, section/key string memory `0x00833000..0x00833328`. `ReadFromINI @ 0x005FA620` reads the paired keys and clamps some fields; `SetDefaults @ 0x005FA350` seeds defaults before user INI is read.

### Touched by this dialog and persisted

| Section/key | Options offset | Type/range in this path | Read evidence | Write evidence | Active in YR |
|---|---:|---|---|---|---|
| `[Options] GameSpeed` | `+0x00` | int; dialog range `0..6`, internal inverted | `0x005FA620` key read; default `3` at `0x005FA350` | disasm `0x005FAD31..0x005FAD48` | Yes |
| `[Options] Difficulty` | `+0x04` | int clamped by reader to `0..4`; in this dialog slider `0..2` shell-only | `0x005FA620`; default `1` | disasm `0x005FAD4D..0x005FAD62` | Conditional in this dialog |
| `[Options] ScrollRate` | `+0x10` | int; dialog range `0..6`, internal inverted | `0x005FA620`; default `3` | disasm `0x005FAD9B..0x005FADB0` | Yes |
| `[Options] DetailLevel` | `+0x18` | int clamped/read and dialog range `0..2` | `0x005FA620`; default `2` | disasm `0x005FADCD..0x005FADE2` | Yes |
| `[Options] UnitActionLines` | `+0x1E` | bool byte from `0x601` | `0x005FA620`; default `1` | disasm `0x005FADFF..0x005FAE12` | Yes |
| `[Options] ShowHidden` | `+0x1F` | bool byte from `0x604` | `0x005FA620`; default `0` | disasm `0x005FAE17..0x005FAE2A` | Yes |
| `[Options] ToolTips` | `+0x20` | bool byte from `0x602` | `0x005FA620`; default `1` | disasm `0x005FAE2F..0x005FAE42` | Yes |

### Also written by `WriteToINI` from the same object

`WriteToINI` also writes `[Options] CampDifficulty`, `ScrollMethod`, `AutoScroll`, `SidebarCameoText`; `[Video] ScreenWidth`, `ScreenHeight`, `StretchMovies`; `[Audio] SoundVolume`, `VoiceVolume`, `ScoreVolume`, `IsScoreRepeat`, `IsScoreShuffle`, `SoundLatency`, `InGameMusic`; `[Network] Socket`, `NetCard`, `DestNet`, and encoded `NetID` from offsets `+0x4C/+0x50`. Evidence: disasm `0x005FAD67..0x005FAFF6`; key strings in memory `0x00833000..0x00833328`.

`ReadFromINI` additionally reads `[Video] AllowHiResModes`, `[Video] AllowModeToggle` (global), and `[Video] AllowVRAMSidebar`; these were not found in `WriteToINI` in this function. Evidence: decompile `0x005FA620` and write disasm absence across `0x005FAD10..0x005FB044`.

## 7. Current Rust Implementation Status

| Surface | Current Rust status | Delta vs verified binary |
|---|---|---|
| `src/ui/shell/modal.rs` | Has `ModalKind::InGameOptions`, `template_id(true)=0xBBB`, `template_id(false)=0xF5`, and result-convention tests around result `1`/`2` | Matches the high-level id/result convention, but has no full descriptor/control state, no init/apply logic, no labels, no command routing |
| `src/app.rs` | Main-menu Options opens an egui placeholder; pause menu is a separate egui overlay with `PauseMenuAction::SetGameSpeed(tps)` | No in-game full-shell Options dialog, no `0x004E1FE0`-equivalent proc, no `0x52C/0x52D/0x686` result routing |
| `src/app_sim_tick.rs` | `advance_in_game_runtime` advances sim when `!state.paused`; no `service_tick`/session-mode modal pump helper found in focused grep | Does not model `0x00623120` modal pump/result-2 contract for this dialog |
| `src/audio/music.rs`, `src/util/ini_writer.rs` | Reads/writes only `[Audio] ScoreVolume` through a single-key helper | Missing whole-object `OptionsClass::WriteToINI` equivalent and result==1 apply-then-write path for all persisted keys |
| `src/ui/main_menu_dialogs.rs` | Launcher options dialog is documented as open-level placeholder, widgets/write-back not decoded | Not this target; do not reuse it for in-game Options |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `State_Machine` case 5 liveness | verified | `0x0048C9C9..0x0048C9E6`, caller xref | none |
| `ShowInGameDialog` template/result/persist path | verified | `0x004E1D2A..0x004E1DAB`; callees/xrefs | none |
| `0x004E1FE0` WM_COMMAND | verified | `0x004E233A..0x004E23AD` | none |
| `0x004E1FE0` WM_HSCROLL | verified | `0x004E2278..0x004E232B`; string tables `0x00822730..0x00822800` | exact localized text values not decoded from CSF |
| `0x004E1FE0` custom init `0x497` | verified | `0x004E2028..0x004E226F` | none |
| `ApplyFromInGameDialog` | verified | `0x004E1DEF..0x004E1FD2` | downstream meaning of states 4/6 outside scope |
| `WriteToINI` field/key writes | verified | `0x005FAD10..0x005FB044`; strings `0x00826444`, `0x00833000..0x00833328` | exact helper formatting for every key not re-derived |
| `0xBBB`/`0xF5` resource controls | verified | `read_memory 0x00C01B18 len 1000`; `read_memory 0x00BF9F58 len 1300` | pixel projection and owner-draw assets deferred to chrome slot |
| Rust implementation scan | verified | focused `rg`/file reads over listed source files | exact future patch structure deferred |
| Chrome paint assets | deferred | user scope; separate slot owns chrome/assets | investigate owner-draw assets if implementation needs pixel proof |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - Which template is selected for active game? -> byte `0x00A8E9A0 == 1` selects `0xBBB`, else `0xF5`.` (evidence: `0x004E1D2A..0x004E1D47`)
- `[RESOLVED] OQ-2 - Which controls set persist result? -> `0x52C` active hiword-zero sets state 4/result 1, `0x52D` active hiword-zero sets state 6/result 1, `0x686` sets result 1 unconditionally.` (evidence: `0x004E233A..0x004E23AD`)
- `[RESOLVED] OQ-3 - Is there a cancel-without-save close button? -> No scoped button writes any non-persist close result; result 2 is caller-generated on pump game-end.` (evidence: `0x004E1D70..0x004E1D9D`, `0x004E233A..0x004E23AD`)
- `[RESOLVED] OQ-4 - What are slider ranges/inversions? -> `0x529`/`0x52A` use range 0..6 and `6-x`; `0x52B`/`0x50F` use range 0..2 direct.` (evidence: `0x004E207F..0x004E226D`, `0x004E1DEF..0x004E1FD2`)
- `[RESOLVED] OQ-5 - What checkbox globals are affected? -> `0x601/+0x1E`, `0x604/+0x1F`, `0x602/+0x20`, each set from `BM_GETCHECK == 1`.` (evidence: `0x004E2195..0x004E2201`, `0x004E1F1A..0x004E1FA9`)
- `[RESOLVED] OQ-6 - Which INI file and keys are written? -> `RA2MD.INI`; full Options object written by `0x005FAD10`.` (evidence: `0x005FAD19..0x005FB00E`, string `0x00826444`)
- `[RESOLVED] OQ-7 - Does `0x51A` ScrollCoasting apply in this path? -> No reference found in scoped proc/apply; it is resource-present in `0xF5` only here.` (evidence: `0x004E1FE0`, `0x004E1DE0`, resource bytes `0x00BF9F58`)
- `[DEFERRED] OQ-8 - Which exact SHP/PAL assets draw `0xBBB`/`0xF5` controls?` (category: out-of-scope; reason: assigned to chrome/ownerdraw slot; next-step-if-pursued: trace common owner-draw setup and paint callbacks for these templates)
- `[DEFERRED] OQ-9 - Exact downstream player-facing meaning of `g_GameState=4` vs `6`.` (category: out-of-scope; reason: this slice proves command side effects and persistence; next-step-if-pursued: trace state-machine cases 4 and 6 interaction flows)

## 10. UI Message/Control Ledger

Paint composition is intentionally not claimed. Message/control participation is:

| Order | Function/address | Condition | Control/message | Active for target? | Role |
|---|---|---|---|---|---|
| 1 | `0x004E1D00` | `g_GameActive byte == 1` else not | create `0xBBB` or `0xF5`, proc `0x004E1FE0` | Yes/conditional | dialog selection |
| 2 | `0x004E1FE0` via common proc | common proc nonzero result returns first | `0x00622B50` | Yes | framework processing |
| 3 | `0x004E1FE0` custom init | message `0x497` | slider ranges/positions, checkbox states, active/shell gate | Yes | init |
| 4 | `0x004E1FE0` scroll | `WM_HSCROLL` low word `5` | label update by sender HWND and high-word index | Yes | live label update |
| 5 | `0x004E1FE0` command | `WM_COMMAND` | `0x52C`, `0x52D`, `0x686` result writes | Yes/conditional | close/next-dialog |
| 6 | `0x004E1D00` after result | result `1` | apply then write INI | Yes | persistence |

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0xBBB`/`0xF5` selected by active byte equality `==1`; `0xF5` has distinct wider slider rects and shell-only `0x50F/0x51A/0x71C` | `0x004E1D2A..0x004E1D47`; resource reads `0x00C01B18`, `0x00BF9F58` | partial stub only | `src/ui/shell/modal.rs`, shell descriptor/layout surfaces, app modal router | Build full owner-draw descriptors from separate resource tables; do not derive `0xF5` from `0xBBB` | Active dialog has `0xBBB` controls incl. `0x52C/0x52D`; shell dialog has `0x50F/0x51A/0x71C` and 148-wide sliders | `test_ingame_options_template_selection_and_control_sets`; risk: resource/layout drift |
| Result `1` applies and writes `RA2MD.INI`; result `2` skips persistence; no cancel-without-save button | `0x004E1D70..0x004E1DAB`, `0x004E233A..0x004E23AD`; file string `0x00826444` | Rust only tests convention and writes ScoreVolume on quit | `src/app.rs`, `src/audio/music.rs`, `src/util/ini_writer.rs`, needed Options state/writer | Route `0x52C/0x52D/0x686` to result `1`; caller runs apply then whole-object write; game-ended result `2` does neither | Click Back in active dialog after toggling ToolTips updates Options state and writes all persisted sections; synthetic result `2` leaves file unchanged | `test_ingame_options_result_one_applies_then_writes_all_options`; risk: confusing message-box result convention with own-proc convention |
| GameSpeed `0x529` is inverted and in active network mode queues event `0x0D` without immediate field store; offline/inactive stores directly | `0x004E1DEF..0x004E1EC0` | pause menu sets app TPS directly; no command queue semantics | app options controller, simulation command queue/event surface | Use `internal = 6 - pos`; in network modes 3/4 enqueue speed event when queue count `<0x80`; in modes 0/5 or inactive store local field | Network-mode changed slider queues exactly one speed event and keeps local field old until command execution; skirmish/campaign direct-stores | `test_ingame_options_gamespeed_network_queues_without_immediate_store`; risk: direct assignment in network lockstep |
| Checkbox writes are `BM_GETCHECK == 1` to offsets `+0x1E/+0x1F/+0x20`; `ToolTips` also updates tooltip manager only if active and manager exists | `0x004E1F1A..0x004E1FA9`; WriteToINI `0x005FADFF..0x005FAE42` | no in-game options state; no UnitActionLines/ShowHidden/ToolTips persistence | options state, render/action-line gates, tooltip service | Treat resource label `GUI:TargetLines` as `[Options] UnitActionLines`; do not use nonzero check result values other than `==1` | Toggle each checkbox and assert exact Options bytes plus write keys; ToolTips manager update only when active | `test_ingame_options_checkboxes_map_to_options_bytes`; risk: label/key confusion |

## 12. Negative Facts / Do Not Do

- Do not implement this as a generic message-box modal or `paint_modal_shp`; it creates RT_DIALOG `0xBBB/0xF5` with own proc `0x004E1FE0`. Evidence: `0x004E1D34..0x004E1D47`.
- Do not add a cancel/discard path for Back/OK/Sound; scoped close buttons write result `1` and result `1` persists. Evidence: `0x004E233A..0x004E23AD`, `0x004E1D9A..0x004E1DAB`.
- Do not treat all sliders as direct values; `0x529` and `0x52A` are inverted with `6 - pos`, while `0x52B` and `0x50F` are direct. Evidence: init/apply ranges at `0x004E207F..0x004E226D` and `0x004E1DEF..0x004E1FD2`.
- Do not reuse `0xBBB` geometry for `0xF5`; `0xF5` sliders are 148 DLU wide and include difficulty/scrollcoasting/static controls absent from `0xBBB`. Evidence: resource reads `0x00C01B18`, `0x00BF9F58`.
- Do not wire `0x51A` ScrollCoasting from this proc without separate evidence; it is present in `0xF5` bytes but not referenced by `0x004E1FE0` or `0x004E1DE0`. Evidence: function bodies plus template bytes.

## 13. Stale Docs / Replacement Wording

- `docs/plans/2026-06-01-shell-substrate-slice5-plan.md`: replace broad wording "`EVERY close button sets result=1`" with: "In active template `0xBBB`, `0x52C` and `0x52D` set result `1` only for `WM_COMMAND` hiword `0` and active byte `==1`, additionally setting `g_GameState` 4 and 6 respectively; `0x686` sets result `1` unconditionally. Template `0xF5` lacks `0x52C/0x52D`; its verified close path in this proc is `0x686`."
- `docs/plans/2026-06-01-shell-substrate-slice5b-kickoff.md`: same replacement applies to the line that says "every close button - OK/Sound/Back - yields 1"; keep the high-level `result==1 -> persist`, but qualify which controls exist in each template and which active/hiword gates apply.

## Sources

- Ghidra decompile/disassembly: `0x0048C8B0`, `0x004E1D00`, `0x004E1DE0`, `0x004E1FE0`, `0x005FAD10`, `0x005FA620`, `0x005FA350`, `0x00623120`.
- Ghidra xrefs/callees: callers of `0x004E1D00`; xrefs to `0x004E1DE0`, `0x005FAD10`, `0x004E1FE0`; xrefs to globals `0x00A8E9A0`, `0x00A8EB60`.
- Ghidra memory: RT_DIALOG bytes at `0x00C01B18` and `0x00BF9F58`; strings at `0x00826444`, `0x00822730..0x00822848`, `0x00833000..0x00833328`.
- Prior docs checked: `docs/research/OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md`, `docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`, `docs/research/UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`, `docs/plans/2026-06-01-shell-substrate-slice5-plan.md`, `docs/plans/2026-06-01-shell-substrate-slice5b-kickoff.md`.
- Rust files scanned/read: `src/ui/shell/modal.rs`, `src/app.rs`, `src/app_sim_tick.rs`, `src/audio/music.rs`, `src/util/ini_writer.rs`, `src/ui/main_menu_dialogs.rs`.
