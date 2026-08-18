# Skirmish Checkboxes and Trackbars - Ghidra Research Report

**Address(es):** `0x006AE6E0`, `0x006ACEE0`, `0x006163A0`, `0x0061D950`, `0x00697F10`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Offline YR Skirmish dialog `0x102` checkboxes `0x54E`, `0x693`, `0x696`, `0x69A`, `0x69D` and trackbars `0x529`, `0x511`, `0x50C`.  
**Non-Scope:** In-game Options dialog, network host/guest lobby variants, full session packet packing, and unrelated Skirmish controls.  
**Confidence:** High.  
**Active in YR:** Yes. `Main_Game @ 0x0052D9A0` calls `FUN_006AE2C0` when `g_GameMode == 5`; that modal path creates/runs the Skirmish shell and exits on Start `0x617` or Back `0x5C0`.

## 1. Overview

The standard offline Skirmish options at the lower part of dialog `0x102` are ordinary Win32 child controls routed through Westwood owner-draw callbacks. `FUN_006AE6E0` initializes the three trackbars from Skirmish/session globals and Rules defaults, initializes five checkbox checked states, and `FUN_006ACEE0` reads all eight controls only when Start/Back is applied.

Active in YR: Yes. Evidence: `FUN_006AE3F0 @ 0x006AE3F0` dispatches custom init message `0x497` to `FUN_006AE6E0` and `WM_COMMAND 0x111` to `FUN_006ACEE0`; `FUN_006AE2C0 @ 0x006AE2C0` is reached by `Main_Game @ 0x0052D9A0` for `g_GameMode == 5`.

## 2. Control Map

| Control | Resource text / role | Init source | Apply write |
|---:|---|---|---|
| `0x529` | `GUI:GameSpeed` trackbar | `DAT_00A8B3CC` copied to `DAT_00A8B268`, then visual pos `6 - value` | `DAT_00A8B268 = 6 - TB_GETPOS`; `DAT_00A8EB60 = DAT_00A8B268`; `DAT_00A8B3CC = DAT_00A8B268` |
| `0x511` | `GUI:Credits` trackbar | `DAT_00A8B3D0` copied to `DAT_00A8B25C`; range from Rules `+0x1480/+0x1488`; step from `+0x148C` | `DAT_00A8B25C = TB_GETPOS`; `DAT_00A8B3D0 = DAT_00A8B25C` |
| `0x50C` | `GUI:UnitCount` trackbar | `DAT_00A8B3D4` copied to `DAT_00A8B270`; range from Rules `+0x1490/+0x1498` | `DAT_00A8B270 = TB_GETPOS`; `DAT_00A8B3D4 = DAT_00A8B270` |
| `0x54E` | `GUI:ShortGame` checkbox | `DAT_00A8B3D8` -> `DAT_00A8B262` -> `BM_SETCHECK` | `DAT_00A8B262 = checked`; `DAT_00A8B3D8 = DAT_00A8B262` |
| `0x69A` | `GUI:SuperWeaponsAllowed` checkbox | `DAT_00A8B3D9` -> `DAT_00A8B263` -> `BM_SETCHECK` | `DAT_00A8B263 = checked`; `DAT_00A8B3D9 = DAT_00A8B263` |
| `0x69D` | `GUI:BuildOffAlly` checkbox | `DAT_00A8B3DA` -> `DAT_00A8B264` -> `BM_SETCHECK` | `DAT_00A8B264 = checked`; `DAT_00A8B3DA = DAT_00A8B264` |
| `0x693` | `GUI:MCVRepacks` checkbox | `DAT_00A8B3DB` -> `DAT_00A8B320` -> `BM_SETCHECK` | `DAT_00A8B320 = checked`; `DAT_00A8B3DB = DAT_00A8B320` |
| `0x696` | `GUI:CratesAppear` checkbox | `DAT_00A8B3DC` -> `DAT_00A8B261` -> `BM_SETCHECK` | `DAT_00A8B261 = checked`; `DAT_00A8B3DC = DAT_00A8B261` |

Active in YR: Yes for all rows. Evidence: `FUN_006AE6E0 @ 0x006AE6E0` initializes each listed ID; `FUN_006ACEE0 @ 0x006ACEE0` reads and writes the same globals in the Start/Back apply block.

## 3. Trackbar Logic

`OwnerDraw_Trackbar_0061D950` stores per-control state in the owner-draw state object: dragging at `[0x3A]`, thumb-drag flag at `[0x3B]`, range span at `[0x3C]`, relative current value at `[0x3D]`, minimum at `[0x3E]`, pixel thumb offset at `[0x3F]`, step at `[0x40]`, numeric-display flag at `[0x41]`, and sound-suppression flag at `[0x42]`.

Active in YR: Yes. Evidence: callback routing doc maps `msctls_trackbar32` controls to `0x0061D950`; `FUN_006AE6E0` sends range/position messages to `0x529`, `0x511`, `0x50C`.

Important constants and formulas:

| Detail | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Range message `0x406` | `LOWORD(lParam)` is min, `HIWORD(lParam)` is max; span is `max - min`; current is clamped before storing | `OwnerDraw_Trackbar_0061D950 @ 0x0061E48x` branch | Yes |
| Position message `0x405` | Accepts absolute value only if `0 <= value - min <= span`; stores relative value `value - min` | `0x0061D950` branch for `0x405` | Yes |
| Get-position `0x400` | Returns quantized absolute value `((min + relative) / step) * step` using integer truncation | `0x0061D950` branch for `0x400` | Yes |
| Default step/display | If stored step is `0`, callback sets `step = 1`, `numeric_display = 1`, and reserves `0x32` px for the value plaque | `0x0061D950` pre-message state normalization | Yes |
| Active width | `(client_width - value_display_width) - 0x0D`, saturated to `1` when below `2` | `0x0061D950` computes `uStack_140` | Yes |
| Mouse mapping | Mouse X is clamped to `[1, client_right - value_display_width - 0x0C]`; raw value is `((x - 1) * (span + 1)) / active_width`, saturated at span | `0x0061D950` drag/click branches | Yes |
| Notification | Value/range/min changes invalidate and send parent `WM_HSCROLL 0x114` with low word `5`, high word current absolute value | `0x0061D950` final `SendMessageA` | Yes |
| Click sound | Plays `VocClass__PlayAtPos(1.0, 0)` only when value changed, sound suppression flag `[0x42] == 0`, and branch permits sound | `0x0061D950` final branch | Yes |

Initialization specifics:

| Control | Range | Initial visual position | Step | Displayed value |
|---:|---|---|---|---|
| `0x529` | `0..6` | `6 - DAT_00A8B268` | `1` | visual position, not stored speed code |
| `0x511` | Rules `MinMoney..MaxMoney` = YR `5000..10000` | `DAT_00A8B25C` | Rules `MoneyIncrement` = YR `100` | credits value |
| `0x50C` | Rules `MinUnitCount..MaxUnitCount` = YR `0..10` | `DAT_00A8B270` | `1` | unit count |

Active in YR: Yes. Evidence: `FUN_006AE6E0 @ 0x006AE6E0`; YR INI values from `ini/rulesmd.ini:3017-3041`. The `GameSpeed` inversion is also confirmed by `FUN_006ACEE0`, which writes `6 - TB_GETPOS`.

Trackbar PCX/text assets:

| Asset / draw | Use | Evidence | Active in YR |
|---|---|---|---|
| `trakgrip.pcx` | Thumb/grip, direct blit | `FUN_006BA140("trakgrip.pcx")` in `0x0061D950` paint branch | Yes |
| `trofm.pcx` | Tiled middle of numeric value plaque | `FUN_006BA140("trofm.pcx")` + `FUN_006BA3E0` | Yes |
| `trofl.pcx` | Left cap of value plaque | `FUN_006BA140("trofl.pcx")` | Yes |
| `trofr.pcx` | Right cap of value plaque | `FUN_006BA140("trofr.pcx")` | Yes |
| Numeric text | Formatted integer with `FUN_007CA564`, drawn in right `0x31` px rect using `FUN_00621040` | `0x0061D950` paint branch | Yes |
| Disabled visual | If `WS_DISABLED 0x08000000` is set, alpha pass uses `DAT_00AC4898` and disabled text color `DAT_00AC1CB4` | `0x0061D950` paint branch | Conditional: only when disabled |

## 4. Checkbox Logic

`OwnerDraw_Checkbox_006163A0` stores checked state at owner-draw state `[0x3A]` and two PCX-family variant bytes at direct byte offsets `+0xD9` and `+0xDA`.

Active in YR: Yes. Evidence: callback routing doc maps Button style low bits `0x03` to `0x006163A0`; Skirmish resource uses checkbox style `0x50000003`; `FUN_006AE6E0` sends `0xF1` to each listed checkbox.

| Message / event | Behavior | Evidence | Active in YR |
|---:|---|---|---|
| `0xF0` | Returns stored checked state | `0x006163A0` branch | Yes |
| `0xF1` | Stores `wParam` as checked state and invalidates | `0x006163A0` branch; sent by `FUN_006AE6E0` | Yes |
| `WM_LBUTTONDOWN 0x201` / `WM_LBUTTONDBLCLK 0x203` | Toggles only if click X `< 0x12` and Y `< 0x12`; outside the 18x18 icon does nothing | `0x006163A0 @ 0x006166EE` | Yes |
| Toggle notification | Invalidates, plays `VocClass__PlayAtPos(1.0, 0)`, sends parent `WM_COMMAND 0x111` with control ID low word and new checked state high word | `0x006163A0` toggle branch | Yes |
| Parent command effect | `FUN_006ACEE0` has no checkbox-ID command cases before Start/Back; checked state is applied later by `BM_GETCHECK` reads | `FUN_006ACEE0 @ 0x006ACEE0` switch ranges | Yes |
| `0x4E5`, `0x4E6`, `0x4E7` | Set/query variant bytes; Skirmish init does not send them | `0x006163A0` branches | Conditional: helper active, but not used by standard Skirmish init for these controls |

Checkbox PCX/text assets:

| Variant state | Unchecked asset | Checked asset | Evidence | Active in YR |
|---|---|---|---|---|
| `+0xD9 == 0` default | `cue_i.pcx` | `cce_i.pcx` | `0x006163A0` formats `c%ce_i.pcx` with `u` or `c` | Yes |
| `+0xD9 != 0`, `+0xDA == 0` | `cue_i.pcx` | `cce_il.pcx` | `0x006163A0` variant branches | Conditional: helper active; not sent by Skirmish init |
| `+0xD9 != 0`, `+0xDA != 0` | `cce_ir.pcx` | `cce_i.pcx` | `0x006163A0` variant branches | Conditional: helper active; not sent by Skirmish init |

Paint details: icon destination is `18 x 18`; label text is drawn only if the state has label data, starts after the icon with a `0x1A` offset from icon height, uses `FUN_00621040`, and switches to disabled text color when `WS_DISABLED 0x08000000` is set. Missing PCX fallback is not robust: the paint path dereferences the surface returned by `FUN_006BA140`.

Active in YR: Yes for default paint path; Conditional for disabled and variant paths. Evidence: `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`.

## 5. INI Keys and Defaults

`SessionClass__ReadSkirmishSettings @ 0x00697F10` reads `[Skirmish]` overrides, falling back to `RulesClass` fields loaded from `[MultiplayerDialogSettings]` by `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`.

| Setting | `[Skirmish]` key | Rules fallback field | YR rulesmd value / fallback | Active in YR |
|---|---|---:|---|---|
| Game speed | `GameSpeed` | `+0x14A0` | `1` (`ini/rulesmd.ini:3026`) | Yes |
| Credits | `Credits` | `+0x1484` | `10000` via `Money` (`ini/rulesmd.ini:3019`) | Yes |
| Unit count | `UnitCount` | `+0x1494` | `10` (`ini/rulesmd.ini:3023`) | Yes |
| Short Game | `ShortGame` | `+0x14B6` | `yes` (`ini/rulesmd.ini:3039`) | Yes |
| Super Weapons Allowed | `SuperWeaponsAllowed` | `+0x14B9` | `yes` from Rules constructor; no explicit supplied `rulesmd.ini` key | Yes |
| Build Off Ally | `BuildOffAlly` | `+0x14BA` | `yes` from Rules constructor; no explicit supplied `rulesmd.ini` key | Yes |
| MCV Repacks | `MCVRepacks` | `+0x14B8` via `MCVRedeploys` | `yes` (`ini/rulesmd.ini:3041`) | Yes |
| Crates Appear | `CratesAppear` | `+0x14B1` via `Crates` | `yes` (`ini/rulesmd.ini:3034`) | Yes |

Active in YR: Yes. Evidence: `0x00697F10` directly reads these eight `[Skirmish]` keys; `0x00671EA0` reads the corresponding `[MultiplayerDialogSettings]` fallback fields; `RulesClass::Constructor @ 0x00665650` seeds `+0x14B9 = 1` and `+0x14BA = 1` before INI reads.

## 6. Current Rust Implementation Status

Rust has partial data coverage but not the standard dialog behavior. `src/sim/game_options.rs:11-78` has fields/defaults for all relevant gameplay options. `src/ui/main_menu.rs:134-154` exposes only selected map/countries, starting credits, start position, short game, and zoom. `src/ui/skirmish_shell/state.rs:35-86` carries only starting credits and short game into the experimental shell. `src/ui/skirmish_shell/layout.rs:29-51` does not define the checkbox or trackbar control IDs in this report.

Active in YR: Not applicable to Rust; this is implementation status, not binary behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish active path | verified | `Main_Game @ 0x0052D9A0`, `FUN_006AE2C0 @ 0x006AE2C0` | none |
| Dialog init for target controls | verified | `FUN_006AE6E0 @ 0x006AE6E0` | none |
| Start/Back apply reads | verified | `FUN_006ACEE0 @ 0x006ACEE0` | none |
| Trackbar owner-draw | verified | `OwnerDraw_Trackbar_0061D950 @ 0x0061D950` | none |
| Checkbox owner-draw | verified | `OwnerDraw_Checkbox_006163A0 @ 0x006163A0` | none |
| `[Skirmish]` reads | verified | `SessionClass__ReadSkirmishSettings @ 0x00697F10` | none |
| Rules fallback reads | verified | `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`; `RulesClass::Constructor @ 0x00665650` | none |
| Resource geometry/text | verified via prior doc | `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md:157-176` | no new resource dump in this slot |
| In-game Options dialog reuse | deferred | user scope excludes it except helper proof | separate targeted investigation if needed |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Which callbacks paint the target controls? Trackbars use `0x0061D950`; checkboxes use `0x006163A0`. Evidence: prior callback routing report plus live decompile of both callbacks.  
[RESOLVED] OQ2 - Are these controls active in standard offline YR Skirmish? Yes. Evidence: `Main_Game @ 0x0052D9A0`, `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_006AE6E0`.  
[RESOLVED] OQ3 - What are the trackbar ranges/defaults? Game speed `0..6` with visual `6 - stored`; credits Rules `MinMoney..MaxMoney` default `5000..10000`; unit count Rules `0..10`. Evidence: `FUN_006AE6E0`, `ini/rulesmd.ini:3017-3041`.  
[RESOLVED] OQ4 - Which checkbox maps to which option/global? Listed in section 2. Evidence: `FUN_006AE6E0` and `FUN_006ACEE0`.  
[RESOLVED] OQ5 - Do checkbox clicks immediately write globals? No; owner state changes and parent is notified, but target option globals are read/written in the Start/Back apply block. Evidence: `0x006163A0`, `0x006ACEE0`.  
[RESOLVED] OQ6 - Which PCX names are used? Trackbar: `trakgrip.pcx`, `trofl.pcx`, `trofm.pcx`, `trofr.pcx`; checkbox default: `cue_i.pcx`, `cce_i.pcx`; variants: `cce_il.pcx`, `cce_ir.pcx`. Evidence: `0x0061D950`, `0x006163A0`.  
[RESOLVED] OQ7 - What exact constructor seed values back absent `SuperWeaponsAllowed` and `BuildOffAlly` keys before rules INI load? Both seed to `1`. Evidence: `RulesClass::Constructor @ 0x00665650` direct byte writes to `this+0x14B9` and `this+0x14BA`.

## Sources

- Ghidra decompile: `Main_Game @ 0x0052D9A0`
- Ghidra decompile: `FUN_006AE2C0 @ 0x006AE2C0`
- Ghidra decompile: `FUN_006AE3F0 @ 0x006AE3F0`
- Ghidra decompile: `FUN_006AE6E0 @ 0x006AE6E0`
- Ghidra decompile: `FUN_006ACEE0 @ 0x006ACEE0`
- Ghidra decompile: `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`
- Ghidra decompile: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`
- Ghidra decompile: `SessionClass__ReadSkirmishSettings @ 0x00697F10`
- Ghidra decompile: `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`
- Ghidra decompile: `RulesClass::Constructor @ 0x00665650`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
