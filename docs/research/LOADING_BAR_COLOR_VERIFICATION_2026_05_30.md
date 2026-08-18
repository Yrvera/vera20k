# Loading-bar color source — binary verification (2026-05-30)

Resolves the contradiction between the UI parity audit task notes (backing =
local player ColorScheme+0x308 HSV, player-dependent) and the
`ls-progress-backing-shade-approximated` finding's "correction" (fixed static
HSV from `&DAT_00887734`, player-independent).

## Verdict: the task notes are CORRECT for the skirmish loading screen.

The findings-file "correction" analyzed the **wrong progress-bar context** —
the preview-dialog branch of the random map generator (`FUN_00598960`, which
uses `PROGBAR2.SHP` and named strings "RMG: Init random map" /
"RMG: Creating starting points"), not the selected-map or active in-game RMG
loading bar (`PROGBARM.SHP`, drawn via the `g_GameMode != 0` path).

## 2026-07-27 audit correction

The original wording collapsed preview-dialog RMG and active in-game RMG into
one renderer. Live callers show two contexts:

- Random-map setup-dialog callers pass a non-null dialog HWND to
  `FUN_00598960`: `0x0059664C`, `0x00596182`, and `0x00596A66` pass
  `preview=1`, while `0x00596A49` passes `preview=0`. Its dialog-state branch
  loads `PROGBAR2.SHP` and drives the dialog meter directly.
- `ScenarioClass__Read_Scenario` loads `PROGBARM.SHP` at `0x006847F6`, then
  calls `FUN_00598960` at `0x00684989` with `(preview=0, hwnd=0)`. Active
  in-game RMG milestones use `FUN_0069AE90`, which reaches the same non-dialog
  `ProgressMeterClass` redraw as selected-map loading. In `g_GameMode != 0`,
  that redraw resolves slot 0 through `FUN_00642BB0(0)`, so both paths use the
  local session's `ColorScheme`.

(corrected 2026-07-27: was "the random map generator uses `PROGBAR2.SHP`";
binary shows `PROGBAR2.SHP` only in the preview-dialog context and
`PROGBARM.SHP` for active in-game RMG via `get_function_xrefs 0x00598960`,
`get_assembly_context 0x006847F6,0x00684989,0x0059664C,0x00596182,0x00596A49,0x00596A66`,
and `batch_decompile 0x00598960,0x0069AE90,0x00643AE0` —
`INFERENCE_HARDENED` / RMG-context collapse.)

## Evidence chain (Ghidra MCP, this session)

- `disassemble_function 0x00643400` (bar/backing draw):
  - Backing rect = (param_2+3, param_3+3), size = PROGBARM frame-0 W×H
    (`0x00643425`/`0x00643443` add 3).
  - `+0x71` set → fill: `LEA ECX,[EAX+0x308]` (`0x006434ae`, EAX=param_7) →
    `CALL FUN_00517440`; pushed `&param_7` is the **output** buffer, not input.
  - Bar convert: `MOV ECX,[EAX+0x30c]` (`0x00643486`) → CC_Draw_Shape remap.
  - So `param_7` is a **ColorScheme pointer**; +0x308 = HSV, +0x30c = convert.
- `decompile_function 0x00517440`: `__thiscall`, reads H=this[0], S=this[1],
  V=this[2] (this = ColorScheme+0x308), writes 3-byte RGB to the arg buffer,
  returns it. Standard 6-sextant HSV→RGB.
- `disassemble_function 0x00643720` @ `0x006439c3`: `FUN_00643400`'s param_7 =
  `FUN_00643720`'s **first stack arg** ([ESP+0x60]).
- `disassemble_function 0x00643ae0` @ `0x00643bef`: that first stack arg = EAX:
  - `[0x00a8b238]` (g_GameMode) `== 0` → `EAX = FUN_0068ca50("AlliedLoad"/
    "SovietLoad")` by side (`+0x80`) — **campaign** path, fixed named schemes.
  - `!= 0` → `EAX = FUN_00642bb0(0)` → `FUN_00696f20` (house color priority,
    +0x53) → `SessionClass__PriorityToColorScheme` → `g_ColorSchemeArray[idx]`.
  - `EDI` (=ProgressClass+0x50 / `&DAT_00887734`) is the **second** arg
    (label/text), NOT the color — this is what the bad "correction" misread.
- `decompile_function 0x00552D60` (loading renderer): `if (g_GameMode==0)` loads
  ScenarioClass mission-briefing strings and returns early WITHOUT calling
  `FUN_00640a40` (mmpb markers). The mmpb/country-art skirmish path is the
  `g_GameMode != 0` branch ⇒ **skirmish ≠ 0** ⇒ uses `FUN_00642bb0(0)` =
  local player's lobby scheme. (Matches the already-verified mmpb finding's
  "game mode 5 (Skirmish), non-campaign branch".)
- `SessionClass__PriorityToColorScheme 0x0069A310`: `read_memory 0x0083ed14` =
  `[3,11,21,29,13,25,17,15,5]`; `DAT_0083ed1c` (0x0083ed1c) = `5`; priority
  `-2 (0xfffffffe)` → 5; `< 9` → LUT; `≥ 9` → unchanged. Byte-exact to task.
- `ColorScheme` construction at `0x0068C710` stores the converter returned by
  `0x0068C3B0` at `+0x30C`. The builder copies the base palette, then writes
  exactly 16 generated RGB triples to palette indices 16..31 before creating
  that converter. For shade `i=0..15`, the verified schedule is:
  `mod_s = trunc(S * sin(50° + i*(40°/15)))` and
  `mod_v = trunc(V * cos(20° + i*(70°/15)))`, except shade 0 overrides the
  cosine angle to `π/16`; fixed H plus `(mod_s,mod_v)` then passes through
  `0x00517440`. `0x004CACB0` is sine (direct table index) and `0x004CAD00` is
  cosine (the same table with a `+0x800` quarter-period offset).
  (`disassemble_bytes 0x0068C3B0..0x0068C4A7`;
  `batch_decompile 0x0068C710,0x004CACB0,0x004CAD00`.)

## HSV→RGB algorithm (port target, from 0x00517440)

H,S,V each 0..255. `f = (H*6) % 255`, `region = (H*6) / 255` (trunc).
`p = (255-S)*V/255`, `q = (255 - f*S/255)*V/255`,
`t = (255 - (255-f)*S/255)*V/255` (all integer-truncating). Standard sextants:
region 0→(V,t,p) 1→(q,V,p) 2→(p,V,t) 3→(p,q,V) 4→(t,p,V) 5→(V,p,q) as (R,G,B).
H=255 → region 6, f=0 ⇒ red (same as region 0).

## Implication

`ls-progress-backing-shade-approximated`'s `correction`/`reasoning` (fixed static
HSV, "ColorScheme+0x308 never appears") is WRONG; the gamemd mechanism is the
player-dependent ColorScheme+0x308 HSV and its `+0x30C` full-band converter.
The `ls-composition-text-backing-fill-color-formula` verifier was right.

Current Rust no longer uses the earlier uniform-tint approximation for this
bar: dev `31d68dec` corrected the shared 16-shade house-ramp schedules, and dev
`8a9176fd` applies the selected player's complete ramp to `PROGBARM` palette
indices 16..31 (`src/rules/house_colors.rs::build_scheme_ramp`,
`src/render/loading_screen_chrome.rs::progress_palette_with_player_ramp`, and
`src/app_loading.rs::NativeLoadingScreenState::resolve_player_colors`). This
closes the mechanism-level stale finding, but exact native/Rust ramp bytes,
the `PROGBARM` source-index mask, and final pixel parity remain **UNVERIFIED**:
Rust uses `f64` trig/truncation rather than the native lookup-table/x87 path,
and no native pixel oracle has certified the rendered bar.
