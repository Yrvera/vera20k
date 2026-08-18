# Movies & Credits Dialog Playback FUN_005BED40 - Ghidra Research Report

**Address(es):** `0x005BED40` primary playback, `0x0052D9A0` `Main_Game`, callback thunks at `0x0052D790` and `0x0052D870`, list population `0x005FC000`, movie list parse `0x00674550`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `Main_Game` case 4 Movies & Credits shell path through selected movie / Sneak Preview / Credits, centered on `FUN_005BED40`
**Non-Scope:** campaign end `FinalMovie`, in-game briefing/final movie paths, exact Bink frame loop internals already covered by focused Bink reports, full dialog template byte layout
**Confidence:** High for dispatch, list data, `FUN_005BED40` branch/order, and YR activity; Medium for resource-template child existence of control `0x71A` because proc references it but RT_DIALOG bytes were not parsed
**Active in YR:** Yes for standard YR main menu, conditional on `g_GameMode == 0` inside `FUN_005BED40`

## Working Notes Gate

- Target question: How does standard YR route Movies & Credits dialog selection into movie playback through `FUN_005BED40`, and what file/name/object/timer/exit semantics matter for Rust?
- Non-goals: Do not re-investigate the main-menu RA2TS Bink frame loop, campaign final movies, or full Bink SDK decode/audio internals.
- Evidence needed to mark COMPLETE: decompile plus assembly/caller proof for case 4, dialog `0x101`, dialog `0x129`, list population, `FUN_005BED40`, and YR activation.
- Stop conditions: Stop after every dialog selection path is either verified or explicitly deferred; do not follow campaign `FinalMovie` or unrelated movie users.

## 1. Overview

`Main_Game` case 4 opens an intermediate Movies & Credits dialog (`RT_DIALOG 0x101`, proc thunk `0x0052D790`). That panel returns `0x0D` for Sneak Preview, `0x0E` for Movies, `0x0F` for Credits, or `0x12` for Back. Sneak Preview calls `FUN_005BED40` directly with hardcoded `RENEGADE.BIK`; Movies opens picker dialog `0x129`, stores a selected `[Movies]` table entry pointer, gates it through a CD/unlock helper, and passes the table entry's first field to `FUN_005BED40`; Credits does not call `FUN_005BED40` and instead runs the credits renderer at `0x004C3E30`.

Active in YR: Yes. Evidence: `Main_Game @ 0x0052DD93..0x0052DD9F` reaches dialog `0x101` from case 4, and `FUN_005BED40 @ 0x005BED61..0x005BED6A` allows playback only when `g_GameMode == 0`, which is the standard main-menu shell mode.

## 2. Class Layout / Key Offsets

### Movie table row used by dialog `0x129`

Rows are 12-byte triples populated by `FUN_00674550` and consumed by `FUN_005FC000`.

| Offset | Type | Purpose | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x00` | `char *` | Movie basename / token passed as `ECX` to `FUN_005BED40` after selection | `0x0052DEAB` loads `ECX = [EDI]`; sample `0x00832C20 -> 0x0082618C = "A00_F00E"` | Yes |
| `+0x04` | `char *` | CSF key used as list display text input | `0x005FC078` loads `ECX = [EBX+4]` before `StringTable__LoadString`; sample `0x00832C24 -> "Name:IntroMovie"` | Yes |
| `+0x08` | `int` | Availability/unlock/campaign-side gate value passed to `0x004790E0` before playback | `0x0052DE95` loads `[EDI+8]`, pushes it, then `0x0052DE9D` calls function pointer `[0x007E4C30] = 0x004790E0` | Yes |

### `FUN_005BED40` stack/local points

| Location | Type | Purpose | Evidence | Active in YR |
|---|---|---|---|---|
| `ECX` at entry | `char *` | requested movie token or filename | Sneak `0x0052DE55 = "RENEGADE.BIK"`; Movies `0x0052DEAB = [EDI]` | Yes |
| `EDX` at entry | pointer/handle-ish arg | forwarded into local stack slot; passed to VQA loop as `param_2` | `0x005BED49`, VQA call decompile `FUN_005BFF60(param_2,0)` | Yes, exact owner type not exhausted |
| `[ESP+0x50]` / decompile `local_100` | `char[256]` | resolved physical filename from BIK-before-VQA resolver | `0x005BED50` passes output buffer to `0x005C0640`; `0x005BED7B..0x005BED98` tests extension | Yes |
| stack Bink object near `[ESP+0x20]` | Bink object | constructed by `0x00432690`, then looped by `0x00432C70`, cleaned by `0x00432700` | `0x005BEDE8..0x005BEDEC`, `0x005BEE4D`, `0x005BEEF5` | Yes for `.bik` |

## 3. Core Logic

### 3.1 `Main_Game` case 4 panel dispatch

Assembly at `0x0052DD93..0x0052DD9F`:

- `PUSH 0x1`
- `MOV EDX,0x52D790`
- `MOV ECX,0x101`
- `CALL 0x0060D380`
- return value copied to `ESI` and sent through the main switch loop

Active in YR: Yes. This is reached from main menu return code `4` (`MoviesAndCredits` button in Rust already maps to return code 4).

### 3.2 Dialog `0x101` control mapping

`0x0052D790` is a callback thunk, not a Ghidra-recognized function boundary. Read-only assembly verifies:

- It first delegates to generic dialog handling `0x00622B50` at `0x0052D7A3..0x0052D7B0`.
- For `WM_PAINT` (`0x0F`), it calls `GetDlgItem(hwnd, 0x71A)` and sends message `0x4F0` with zero `wParam/lParam` at `0x0052D824..0x0052D83A`; this keeps the left Bink panel draw path active on this sub-panel.
- For `WM_COMMAND` (`0x111`), it masks the control id to 16 bits, subtracts `0x686`, bounds-checks `0..9`, reads byte table `0x0052D85C`, and jumps via `0x0052D848`.

Return writers:

| Control | Return | Meaning | Evidence | Active in YR |
|---:|---:|---|---|---|
| `0x68D` | `0x0D` | Sneak Preview | `0x0052D7EC..0x0052D7F7` writes `0x0D` | Yes |
| `0x68E` | `0x0E` | Movies picker | `0x0052D7FA..0x0052D805` writes `0x0E` | Yes |
| `0x68F` | `0x0F` | Credits | `0x0052D808..0x0052D813` writes `0x0F` | Yes |
| `0x686` | `0x12` | Back to main menu | `0x0052D816..0x0052D821` writes `0x12` | Yes |

### 3.3 Sneak Preview return `0x0D`

Assembly at `0x0052DE4C..0x0052DE68`:

- pushes four stack args: `EBX`, `1`, `1`, `1`
- sets `EDX = EBP`
- sets `ECX = 0x0082634C`, verified string `"RENEGADE.BIK"`
- sets next loop state `ESI = 4`
- calls `FUN_005BED40`
- then calls `0x0052FEC0` with `DL=1, ECX=0`, likely shell surface rebuild

Active in YR: Yes, when the standard shell Movies & Credits panel's Sneak Preview button is selected. File playback is conditional on `RENEGADE.BIK` resolving in archives/loose path.

### 3.4 Movies return `0x0E`

Assembly at `0x0052DE72..0x0052DEC9`:

1. Open picker: `PUSH 1`, `EDX = 0x0052D870`, `ECX = 0x129`, `CALL 0x0060D380`.
2. If picker returns `-1` or `0`, set next loop state `ESI = 4` and do not play.
3. If non-null/non-cancel, load `[EDI+8]`, call function pointer `[0x007E4C30] = 0x004790E0`, and abort playback if returned `AL == 0`.
4. On success, load `ECX = [EDI]`, push `EBX`, `1`, `1`, `1`, set `EDX = EBP`, call `FUN_005BED40`.
5. Rebuild shell surface via `0x0052FEC0`, then return to main loop.

Important correction: the selected movie name passed to `FUN_005BED40` is `[EDI]`, not `[EDI+8]`. `[EDI+8]` is a gate passed to `0x004790E0`.

Active in YR: Yes for standard YR shell movie picker. `0x004790E0` is active as an availability gate; if its `param_2 == -2`, it returns `0xFFFFFF01`; if `DAT_0089E3A0 == 1`, it returns `1`; otherwise it writes `-1` to `this+4` then calls `0x004A8270(param_2)`.

### 3.5 Dialog `0x129` picker

`0x0052D870` is also a callback thunk, not a function boundary. Verified assembly:

- Entry grabs listbox `0x744` via `GetDlgItem` at `0x0052D879..0x0052D884`.
- Delegates to `0x00622B50` first at `0x0052D89A..0x0052D8A5`.
- For message `0x497`, if the listbox hwnd is non-null, it calls `FUN_005FC000` with `ECX = 0x00A8EB60` and the listbox hwnd pushed at `0x0052D8D4..0x0052D8DE`, then sends `0x186` (`LB_SETCURSEL`) twice around `DAT_00825C80` at `0x0052D8E3..0x0052D903`.
- For `WM_COMMAND` (`0x111`), it recognizes:
  - `0x686` Back: writes `-1` into the result pointer when non-null (`0x0052D92B`, `0x0052D94A..0x0052D94E`).
  - `0x744` listbox: only when high word notification is zero (`SHR EBX,16`; `JNZ` skips), marks play/select flag at `0x0052D943`.
  - `0x745` Play: marks play/select flag at `0x0052D937`.
- If play/select flag is set and listbox hwnd exists, it sends `0x188` (`LB_GETCURSEL`) and stores selected index to `DAT_00825C80`; if selection is not `-1`, it sends `0x199` (`LB_GETITEMDATA`) and writes the resulting table pointer into the result pointer (`0x0052D960..0x0052D98D`).

Active in YR: Yes for picker dialog `0x129` opened by case `0x0E`.

### 3.6 List population from `[Movies]`

`FUN_00674550` reads the `[Movies]` section from the active art INI, with section string `0x007F0CE4 = "Movies"` and default value `0x00817474 = "<none>"`.

Verified logic:

- Check section exists: `FUN_00526810("Movies")`.
- Count entries: `FUN_00526960("Movies")`.
- For each entry index, get key name with `FUN_00526CC0("Movies", index)`.
- Read string value with max length `0x20` into local buffer.
- Reject empty / `<none>` / duplicate-ish entries via `0x0048DF30` and availability checks.
- Store token pointer from `FUN_007D5408(local_20)` into global movie table array `DAT_00ABF394` and increment count `DAT_00ABF3A0`.

`FUN_005FC000` populates the visible listbox from three ranges:

- intro row at `0x00832C20` if `[0]` and `[4]` are non-null;
- Soviet range from `0x00832CA0`, bounded by `OptionsClass+0x4C + 1`;
- Allied range from `0x00832C30`, bounded by `OptionsClass+0x50 + 1`.

For each row, it requires `[row+0] != 0` and `[row+4] != 0`, loads display text through `StringTable__LoadString` with `ECX = [row+4]`, source path `0x00833370 = "D:\\ra2mdpost\\Options.CPP"`, fallback id `0x4F1`, then listbox-adds message `0x4CD` and attaches the row pointer through `0x19A` (`LB_SETITEMDATA`).

Active in YR: Yes. `ini/artmd.ini` has `[Movies]` entries `1=A00_F00e` through `60=S08_F01e`, with key `48` omitted, so 59 YR entries. Base `ini/art.ini` has inherited/older movie names and is superseded by `artmd.ini` in standard YR md data.

### 3.7 `FUN_005BED40` playback function

Entry assembly at `0x005BED40..0x005BED6A`:

- allocates `0x140` stack bytes and saves registers.
- stores incoming `EDX` at `[ESP+0x0C]`.
- pushes `1`, sets `EDX` to a local output filename buffer, then calls `CDFileClass__Constructor @ 0x005C0640` with `ECX` still holding the requested movie token.
- if resolver returns false, returns.
- if `g_GameMode != 0`, returns without playing.
- calls `FUN_0054F720` before extension test.

Resolver `0x005C0640` copies the token up to the dot, appends `.BIK` (`0x0082419C`), tries file open, and if missing appends `.VQA` (`0x008241A4`) and tries again. If an output buffer was supplied, it copies the resolved filename back. Active in YR: Yes.

Bink branch at `0x005BED7B..0x005BEEF5`:

1. `strrchr(local_filename, '.')`.
2. Compare extension case-insensitively against `.bik` (`0x0082D9CC`) through `0x007C8D20`.
3. Log `"Play_Movie() as Bink!\n"`.
4. Lock/display prep via `g_DisplayChain + 0x0C`.
5. If the fourth stack byte parameter (`param_5`) is non-zero, call primary surface vtable `+0x18` with zero and `FUN_004F4780(0)`.
6. Construct stack Bink object through `FUN_00432690(local_filename)`.
7. If `DAT_00ABF35C == 0`, pause EVA, call `0x00408200`, `0x00408270`, set two volume targets, and set `DAT_00ABF35C = 1`.
8. Call `0x00406EA0`, `0x0040A7C0(2)`, `FUN_00432C70()` main Bink playback loop, `0x0040A850`, `0x00406EC0`.
9. If audio pause flag is set, call `0x00408230`, `0x004082F0`, `0x004083D0`, set volume targets, unpause EVA, and clear `DAT_00ABF35C`.
10. If the second stack byte parameter (`param_3`) equals `1`, restore primary surface via vtable `+0x18` and `FUN_004F4780(0)`.
11. Call `FUN_0054F720`, display-chain vtable `+0x10`, `FUN_004F42F0(2)`, then `FUN_00432700()` Bink cleanup.

VQA branch:

- If extension is absent or not `.bik`, it constructs a VQA movie object via the older VQ path, optionally stretches if `param_4 == 1`, `DAT_00A8EB94 == 1`, `DAT_008A0DEE == 1`, and source dimensions are smaller than display dimensions.
- It runs the same display-chain lock / EVA pause / `0x0040A7C0(2)` / `0x0040A850` / `0x00406EC0` sandwich around `FUN_005BFF60(param_2, 0)`.
- It cleans VQA state through `0x005BFF00`, `0x005C01F0`, and `0x007C8B3D`.

Active in YR: Yes for shell playback. VQA fallback is conditional on `.BIK` not resolving or requested/resolved extension not being `.bik`; it is part of the live resolver contract, even if stock YR movie entries normally resolve to `.BIK`.

### 3.8 Bink loop exit for this path

This path uses `FUN_00432C70`, not the main-menu owner-draw timer loop. `FUN_00432C70` exits when either:

- Bink current marker reaches/exceeds total marker or wraps below last marker (`0x00432C70` decompile uses handle `+0x0C`, `+0x08`, object `+0x30`), or
- input path returns key/control `0x81B` through `FUN_0054F000` / `FUN_0054F050`, with the guard that `*param_1 == 0`.

Active in YR: Yes for blocking movie playback through `FUN_005BED40`. Exact key enum/name for `0x81B` is not claimed here.

## 4. INI Keys

| File / Section | Key(s) | Default/value seen | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|---|
| `ini/artmd.ini [Movies]` | numeric keys `1..60`, `48` absent | 59 values: `A00_F00e` through `S08_F01e` | Source tokens for picker movie rows | INI lines `19546..19605`, binary parser `0x00674550` | Yes |
| `ini/art.ini [Movies]` | base numeric keys | TS/RA2-era names such as `CAP_TRAT`, `COUP`, `VEGAWIN` | Base fallback data, superseded by YR md list in standard YR | INI lines `14565..`, same binary parser | Conditional fallback, not normal YR md list |
| `ini/battlemd.ini FinalMovie` | `FinalMovie=` | empty in listed campaign sections | Not used by this dialog path | INI grep; no case-4 binary reader found in this slice | No for this slice |

## 5. Integration Points

| Point | Caller / callee | Evidence | Active in YR |
|---|---|---|---|
| Main menu button to case 4 | Rust already maps `MoviesAndCredits0x686 -> return code 4`; binary case 4 opens dialog `0x101` | `src/ui/main_menu_shell/state.rs`; `0x0052DD93..0x0052DD9F` | Yes |
| Dialog `0x101` left movie repaint | `WM_PAINT` sends `0x4F0` to child `0x71A` | `0x0052D824..0x0052D83A` | Yes if child exists in template |
| Dialog `0x129` populate | message `0x497` calls `FUN_005FC000(0x00A8EB60, listbox 0x744)` | `0x0052D8C8..0x0052D8DE` | Yes |
| Selected row to playback | `LB_GETITEMDATA` result written to dialog result; `Main_Game` passes `[row+0]` to `FUN_005BED40` | `0x0052D979..0x0052D98D`; `0x0052DEAB..0x0052DEB6` | Yes |
| BIK-before-VQA resolver | `FUN_005BED40` calls `0x005C0640`; resolver appends `.BIK` then `.VQA` | `0x005BED50..0x005BED54`; `0x005C0714`, `0x005C0724` | Yes |
| Credits path | return `0x0F` calls `0x004C3E30`, then restarts `INTRO` | `0x0052DED3..0x0052DEF2` | Yes |

## 6. Current Rust Implementation Status

Rust currently has the main shell button identity and the main-menu RA2TS Bink surface, but it does not model the Movies & Credits sub-panel, movie picker, credits renderer, or blocking movie playback path.

| Rust surface | Status vs this slice |
|---|---|
| `src/ui/main_menu_shell/state.rs` | Has main button return code 4 for `MoviesAndCredits`; no return-code state machine for `0x101` sub-options or `0x129` picker. |
| `src/app.rs` | Handles `MainMenuShellAction::MoviesAndCredits` as a shell action bucket, but no native-like dialog stack for Sneak / Movies / Credits. |
| `src/app_main_menu_shell_render.rs` | Draws only the initial `0xE2` shell and RA2TS background movie; no `0x101` or `0x129` controls/listbox. |
| `src/render/bink_movie.rs` | Generic enough to decode/upload Bink frames, but timing/loop differences are already covered in Bink loop reports; no blocking `Play_Movie` wrapper with BIK-before-VQA, EVA/volume/display-chain side effects, VQA fallback, or `0x00432C70` exit semantics. |
| `src/assets/asset_manager.rs` | Archive list includes `movies01.mix` and `movies02.mix`, so movie data can be loaded, but `[Movies]` picker table and CSF row labels are not surfaced as a game UI model. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Main_Game` case 4 opens dialog `0x101` | verified | `0x0052DD93..0x0052DD9F` | none |
| Dialog `0x101` return code mapping | verified | `0x0052D7CC..0x0052D821` | RT_DIALOG visual byte layout not parsed |
| Dialog `0x101` left Bink repaint | verified | `0x0052D824..0x0052D83A` | actual child template existence of `0x71A` deferred |
| Sneak Preview `RENEGADE.BIK` path | verified | `0x0052DE4C..0x0052DE68`, string `0x0082634C` | file availability in every install language not checked |
| Movies picker dialog `0x129` open | verified | `0x0052DE72..0x0052DE83` | none |
| Picker listbox/play/back behavior | verified | `0x0052D870..0x0052D996` assembly contexts | exact Windows notification names beyond numeric high-word check not claimed |
| Picker list population | verified | `0x005FC000` decompile and assembly | exact CSF localized strings not dumped |
| `[Movies]` INI parser | verified | `0x00674550` decompile; `artmd.ini` lines | exact global table allocation limit not exhausted |
| Selected row to `FUN_005BED40` | verified | `0x0052DE95..0x0052DEB6` | none |
| `FUN_005BED40` BIK-before-VQA resolver | verified | `0x005BED50..0x005BED98`; `0x005C0640` | precise archive priority covered by resolver docs, not re-proven here |
| `FUN_005BED40` Bink wrapper order | verified | `0x005BEDA8..0x005BEEF5` | internal Bink frame loop details referenced to existing Bink reports |
| `FUN_005BED40` VQA wrapper order | verified | `0x005BEF05..0x005BF16E` decompile | exact VQA decoder internals out of scope |
| Credits path | verified | `0x0052DED3..0x0052DEF2`; prior `0x004C3E30` decompile | credits text layout/scroll internals not re-exhausted |
| Campaign `FinalMovie` | deferred | INI grep only | separate campaign movie investigation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is `Main_Game` case 4 active in standard YR? -> Yes, it opens `0x101` from return code 4 in shell game mode.` (evidence: `0x0052DD93..0x0052DD9F`)
- `[RESOLVED] OQ2 - Does dialog `0x101` directly play movies? -> No, it returns sub-codes: Sneak `0x0D`, Movies `0x0E`, Credits `0x0F`, Back `0x12`.` (evidence: `0x0052D7EC..0x0052D821`)
- `[RESOLVED] OQ3 - Does dialog `0x101` keep a Bink panel draw active? -> It sends `0x4F0` to child `0x71A` on paint after generic handling fails.` (evidence: `0x0052D824..0x0052D83A`)
- `[RESOLVED] OQ4 - What file does Sneak Preview use? -> Hardcoded `RENEGADE.BIK`.` (evidence: `0x0052DE55`, string `0x0082634C`)
- `[RESOLVED] OQ5 - How is the movie picker opened? -> Dialog `0x129` with callback thunk `0x0052D870`.` (evidence: `0x0052DE72..0x0052DE7E`)
- `[RESOLVED] OQ6 - Which controls does picker use? -> listbox `0x744`, play `0x745`, back `0x686`.` (evidence: `0x0052D879`, `0x0052D921..0x0052D98D`)
- `[RESOLVED] OQ7 - What event populates the picker list? -> message `0x497` calls `FUN_005FC000` then sets current selection through `0x186`.` (evidence: `0x0052D8C8..0x0052D903`)
- `[RESOLVED] OQ8 - Where do picker items come from? -> `[Movies]` parsed by `0x00674550`, listed by `0x005FC000`.` (evidence: `0x00674550`, `0x005FC000`)
- `[RESOLVED] OQ9 - Does selected `[row+8]` name the movie? -> No. `[row+8]` is checked by `0x004790E0`; `[row+0]` is passed to `FUN_005BED40`.` (evidence: `0x0052DE95..0x0052DEB6`)
- `[RESOLVED] OQ10 - Does `FUN_005BED40` run in non-shell game modes? -> It returns unless `g_GameMode == 0`.` (evidence: `0x005BED61..0x005BED6A`)
- `[RESOLVED] OQ11 - Does playback try BIK before VQA? -> Yes, resolver `0x005C0640` appends `.BIK`, then `.VQA`, and returns resolved filename.` (evidence: `0x005BED50..0x005BED54`; `0x005C0714..0x005C0786`)
- `[RESOLVED] OQ12 - What class/object is used for `.bik` in this path? -> Stack Bink object constructed by `FUN_00432690`, looped through `FUN_00432C70`, cleaned by `FUN_00432700`.` (evidence: `0x005BEDE8..0x005BEEF5`)
- `[RESOLVED] OQ13 - Does this path use the owner-draw timer loop? -> No, it uses blocking loop `FUN_00432C70`; owner-draw timer loop remains a different path.` (evidence: `0x005BEE4D`; `0x00432C70` decompile)
- `[RESOLVED] OQ14 - How does blocking Bink playback exit? -> End/wrap predicate or input result `0x81B` through `FUN_0054F000/FUN_0054F050`.` (evidence: `0x00432C70` decompile)
- `[RESOLVED] OQ15 - Does Credits use `FUN_005BED40`? -> No, it calls `0x004C3E30` and then restarts `INTRO`.` (evidence: `0x0052DED3..0x0052DEF2`)
- `[DEFERRED] OQ16 - What exact RT_DIALOG pixel/control template exists for `0x101` and `0x129`?` (category: `out-of-scope`; reason: this slice verifies proc behavior, not resource-byte layout; next-step-if-pursued: parse RT_DIALOG resources and compare rects/styles)
- `[DEFERRED] OQ17 - What exact localized CSF strings appear for each movie row?` (category: `out-of-scope`; reason: binary proves CSF key path, but string table dump is a UI-content pass; next-step-if-pursued: dump row `[+4]` keys and resolve CSF text)
- `[DEFERRED] OQ18 - What is the exact campaign unlock meaning of row `[+8]` values and `OptionsClass+0x4C/+0x50`?` (category: `requires-different-system-context`; reason: this slice verifies the gate call and list bounds, not progression state writes; next-step-if-pursued: investigate campaign completion/options writes feeding those fields)
- `[DEFERRED] OQ19 - What are exact VQA decoder pixels/audio for fallback files?` (category: `requires-different-system-context`; reason: fallback contract is verified but VQA decode parity is a separate media pipeline; next-step-if-pursued: VQA playback swarm)
- `[DEFERRED] OQ20 - Are all movie files present/resolvable in every retail language mix ordering?` (category: `needs-runtime-debugger`; reason: binary resolver is verified; install-specific archives need runtime asset survey; next-step-if-pursued: run asset survey across configured retail install)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | generic dialog handler `0x00622B50` from `0x0052D790` | always first in proc | generic shell/dialog chrome | dialog-managed | generic shell path | Yes | parent chrome / base handling |
| 2 | `GetDlgItem(0x71A)`, `SendMessage(0x4F0)` at `0x0052D824..0x0052D83A` | only when generic handler did not consume `WM_PAINT` | existing left Bink panel | child control `0x71A` | owner-draw Bink path | Conditional: proc active, template child not byte-parsed | background movie panel |
| 3 | generic dialog handler `0x00622B50` from `0x0052D870` | always first in picker proc | generic picker chrome/list controls | dialog-managed | generic shell path | Yes | picker parent chrome |
| 4 | listbox add `0x4CD` / itemdata `0x19A` in `0x005FC000` | on message `0x497`; row token and CSF key both non-null | CSF text from row `[+4]`, data row pointer | listbox `0x744` | Windows/listbox text path | Yes | movie picker content |
| 5 | blocking Bink playback `FUN_00432C70` | after `FUN_005BED40` resolves `.bik` | selected `.BIK` or `RENEGADE.BIK` | Bink object/surface | BinkCopyToBuffer/DirectDraw path covered by Bink reports | Yes for `.bik` | full-screen/blocking movie |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `RENEGADE.BIK` | Conditional | Yes if found | Yes | Sneak Preview content | No | No | No | No | `0x0052DE55`, `0x0082634C` |
| `[Movies]` `.BIK` entries such as `A00_F00E` | Conditional on row and file resolution | Yes if selected and found | Yes | Movie picker playback content | No | No | No | No | `0x0052DEAB`, `0x005BED40` |
| `[Movies]` row CSF keys such as `Name:IntroMovie` | Yes via string table | Yes in listbox | Yes | Picker text | No | No | No | No | `0x005FC078..0x005FC093`, `0x00832F64` |
| RA2TS/menu Bink panel child `0x71A` | Existing shell state | Repainted by `0x4F0` | Conditional | Background panel | No | No | No | No | `0x0052D824..0x0052D83A` |
| `CREDITSMD.TXT` | Yes in credits path | Text scroll, not Bink | Yes | Credits roll | No | No | No | No | prior `0x004C3E30` decompile; case call `0x0052DED8` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Movies & Credits main button opens dialog `0x101`, whose sub-buttons return `0x0D/0x0E/0x0F/0x12`; `0x0E` opens picker `0x129` | `0x0052DD93..0x0052DD9F`; `0x0052D7EC..0x0052D821`; `0x0052DE72..0x0052DE7E` | missing | `src/ui/main_menu_shell/state.rs`, needed movies/credits shell state/render modules | Add a native-like dialog state sequence instead of treating return code 4 as terminal/no-op | `main_menu_movies_button_opens_movies_credits_subpanel_and_back_returns_0x12` | Do not jump directly from main menu button to a flat movie list; native has an intermediate panel. |
| Picker row itemdata is a 12-byte row; `[+0]` is movie token passed to `FUN_005BED40`, `[+4]` is CSF text key, `[+8]` is availability gate checked before playback | `0x005FC078..0x005FC0A8`; `0x0052DE95..0x0052DEB6` | missing | rules/artmd movie table loader, CSF UI model, picker selection state | Build rows from `artmd.ini [Movies]`, display CSF `[+4]`, preserve gate semantics before playback, pass `[+0]` token to movie resolver | `movies_picker_selected_row_uses_movie_token_not_gate_field_for_playback` | Do not use `[+8]` as the movie name; that is stale and wrong. |
| `FUN_005BED40` resolves requested token by trying `.BIK` first, then `.VQA`, then branches; `.BIK` playback is blocking through `FUN_00432C70` and exits on movie end/wrap or key/control `0x81B` | `0x005BED50..0x005BED98`; `0x005C0640`; `0x005BEE4D`; `0x00432C70` | missing / main-menu-only Bink surface exists | `src/render/bink_movie.rs`, `src/assets/asset_manager.rs`, needed blocking movie player/app state | Implement a general movie playback wrapper separate from owner-draw RA2TS shell playback, with BIK-before-VQA fallback and blocking exit semantics | `play_movie_resolver_prefers_bik_over_vqa_and_uses_blocking_exit_key_0x81b` | Do not reuse the RA2TS owner-draw timer loop as this path's mechanism; it is a different loop. |

Stale Docs / Follow-up Docs:

- Replace any wording saying "Case `0x0E` passes `EDI[+8]` / selected item data movie name to `FUN_005BED40`" with: "Case `0x0E` first passes row `[+8]` to availability gate `0x004790E0`; if accepted, it passes row `[+0]` as `ECX` to `FUN_005BED40`."
- Replace any wording implying Credits is a movie file playback with: "Credits bypasses `FUN_005BED40` and calls credits renderer `0x004C3E30`, then queues/starts `INTRO`."

## Negative Facts / Do Not Do

- Do not treat the Movies & Credits button as directly opening the movie picker; native opens intermediate dialog `0x101`.
- Do not pass row `[+8]` to the movie resolver as a filename; binary passes `[+0]`.
- Do not skip the row availability gate before playback; native calls `[0x007E4C30] = 0x004790E0` and aborts if `AL == 0`.
- Do not model this blocking playback with the main-menu `0x71A` owner-draw timer loop; `FUN_005BED40` uses `FUN_00432C70`.
- Do not delete VQA fallback from the contract; BIK-first is verified, but VQA fallback is live code in the same shell playback function.

## Sources

- Ghidra decompile/read-only assembly: `0x005BED40`, `0x005C0640`, `0x0052D9A0`, `0x005FC000`, `0x00674550`, `0x00432C70`, `0x004790E0`
- Read-only assembly contexts: `0x0052D790..0x0052D845`, `0x0052D870..0x0052D996`, `0x0052DD93..0x0052DEF2`, `0x005BED40..0x005BF16E`
- Memory strings: `0x0082634C = "RENEGADE.BIK"`, `0x0082D9CC = ".bik"`, `0x0082419C = ".BIK"`, `0x008241A4 = ".VQA"`, `0x00833370 = "D:\\ra2mdpost\\Options.CPP"`, `0x00832F64 = "Name:IntroMovie"`
- INI checked: `ini/artmd.ini [Movies]`, `ini/art.ini [Movies]`, `ini/battlemd.ini FinalMovie`, `ini/battle.ini FinalMovie`
- Prior docs referenced: `MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md`, focused Bink reports listed in parent context
