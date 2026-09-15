# Launcher Options 0xD5 Backup / Reset Semantics — Ghidra Research Report

**Address(es):** `0x0052D9A0` (live caller), `0x0055FC80` (launcher owner), `0x0055FDB0..0x0056047D` (primary dialog proc bytes), `0x0055FAA0` (accepted-control projection), `0x00ABCE70` (backup object)

**Investigation Mode:** exhaustive-slice

**Claimed Scope:** active-YR launcher Options primary RT_DIALOG `0xD5`: complete retail resource inventory; initialization, message, command, slider-preview, resolution-list, and parent-result behavior; parent loop/re-entry/final-write semantics; every direct or interior reference to the `0xB8`-byte backup; ordinary shell composition and bound retail assets; and current Rust production seams required to implement this parent surface.

**Non-Scope:** profile parsing/serialization and boot/runtime projection already closed by `OPTIONS_PROFILE_TRANSACTION_GHIDRA_REPORT.md`; RT_DIALOG `0xD7` Network internals; keyboard-editor internals and `KEYBOARDMD.INI`; invalid display recovery; in-game Options `0xBBB`/`0xF5`; online-service semantics; and pixel-perfect implementation/capture work.

**Confidence:** High for the semantic parent and verified composition; Medium for the explicitly deferred exact combo-arrow visual state

**Active in YR:** Yes

## 1. Verdict

The stale “backup restores on cancel” claim is false. `OptionsClass__ShowLauncherDialog @ 0x0055FC80` copies the live `OptionsClass` object to `0x00ABCE70` before every primary pass, but no command, parent branch, child callback, or backup-interior reference restores any field. The only runtime reads of that backup are `ScreenWidth` and `ScreenHeight` at `0x00ABCE94/0x00ABCE98` while selecting the initial resolution-combo entry.

The launcher is an always-apply parent. After every primary modal pump—Back, Network, Keyboard, or pump termination—it calls `OptionsClass__ApplyFromLauncherDialog @ 0x0055FAA0`. Network and Keyboard then open their child surfaces and loop through a newly copied primary snapshot. The final non-child result writes the complete profile once and restores the prior `g_GameActive` byte. Escape is not a native cancel: the primary proc ignores `IDCANCEL=2`, so default-dialog Escape does not close or roll back the surface.

Control `0x5CD` is **Network**, not resolution. The resolution selector is combo `0x6ED` inside the primary dialog. This corrects the older navigation report.

The current Rust surface is only an egui placeholder: it has no retained controls, preview, profile projection, result routing, resolution enumeration, apply boundary, persistence boundary, or native Escape/terminal-pump semantics. Existing process-profile, presentation, and audio owners are sufficient to implement the parent without adding a second settings owner or any simulation dependency.

## 2. Evidence identity and liveness

- Active executable: `C:\Users\enok\Documents\Command and Conquer Red Alert II\gamemd.exe`
- Size: `5,286,504` bytes
- SHA-256: `1CDD1180E49024FBDA8AD568CAAC2E86E856063FF67AB38F62B7D2C7BB84298C`
- PE resource: RT_DIALOG id `0xD5`, language `0x0409`, RVA `0x7F60F8`, file offset `0x4FA0F8`, length `2,018` bytes
- Rust comparison point: `origin/main c9b97157f42e932c9360fdad436e91a6fe5e0835` (`ae2f15c1..c9b97157` changes only `README.md`, so production source is unchanged)

`Main__PrepareSession @ 0x0052D9A0` receives main-menu result `5`, sets the next shell selector to `0x12`, calls `OptionsClass__ShowLauncherDialog`, and returns to the pre-session shell loop. This is an ordinary active-YR main-menu route, not a TS-only or editor-only residue.

## 3. Profile fields projected by the parent

The parent reads or writes only the following already-verified singleton fields. Other bytes in the copied `0xB8` object are inert for this dialog.

| Live address / offset | Type | Parent control | Native parent projection |
|---|---|---|---|
| `0x00A8EB64 / +0x04` | `i32` Difficulty | `0x50F` | direct position `0..2` |
| `0x00A8EB70 / +0x10` | `i32` ScrollRate | `0x52A` | control position `6 - value`; accept writes `6 - position` |
| `0x00A8EB78 / +0x18` | `i32` DetailLevel | `0x52B` | init collapses zero/nonzero; accept maps `0 -> 0`, nonzero `-> 2` |
| `0x00A8EB7E / +0x1E` | byte UnitActionLines | `0x601` | checked iff nonzero; accept true iff `BM_GETCHECK == 1` |
| `0x00A8EB7F / +0x1F` | byte ShowHidden | `0x604` | same checkbox rule |
| `0x00A8EB80 / +0x20` | byte ToolTips | `0x602` | same checkbox rule |
| `0x00A8EB84 / +0x24` | `i32` ScreenWidth | `0x6ED` | selected pair; no immediate display-mode apply |
| `0x00A8EB88 / +0x28` | `i32` ScreenHeight | `0x6ED` | selected pair; no immediate display-mode apply |
| `0x00A8EB95 / +0x35` | byte AllowHiResModes | list gate | maximum mode `4096x4096` when true, `1024x768` when false |
| `0x00A8EB98 / +0x38` | `f32` SoundVolume | `0x532` | bounded position `0..10`, preview/accept `position * 0.1` |
| `0x00A8EB9C / +0x3C` | `f32` VoiceVolume | `0x536` | same |
| `0x00A8EBA0 / +0x40` | `f32` ScoreVolume | `0x52F` | same; accepted zero also requests current/default Theme queue-then-stop, subject to the Theme/audio gates in §6 |

The resolution initializer compares enumerated pairs with backup addresses `0x00ABCE94/0x00ABCE98`, not the live pair. That distinction only preserves the selection that existed at the start of the current primary pass. A combo selection writes both backup and live width/height; a child return then starts a new pass and recopies the now-live object.

## 4. Owner transaction at 0x0055FC80

The owner performs this exact process transaction:

1. Save `g_GameActive @ 0x00A8E9A0` and set it to zero for the entire parent/child loop.
2. Copy `0x2E` dwords (`0xB8` bytes) from live `0x00A8EB60` to backup `0x00ABCE70`.
3. Set primary result to `-1` and create RT_DIALOG `0xD5` with proc address `0x0055FDB0`. A null creation result retries indefinitely.
4. Install the result pointer, enter modal service, and pump until the result becomes nonnegative or `ProcessModalServicePump` returns `1`.
5. Always call `OptionsClass__ApplyFromLauncherDialog`, then destroy/clean the primary dialog.
6. Result `0x5CD`: call `SessionClass__ReadMultiPlayerSettings @ 0x006980C0`, create RT_DIALOG `0xD7` with proc `0x00560480`, pump/clean it if creation succeeds, and return to step 2.
7. Result `0x5CE`: call `0x005FBEF0`, which opens the keyboard editor (RT_DIALOG `0xA3` / proc `0x005FB320`), then return to step 2.
8. Every other result—including Back `0x5CB` or the unchanged `-1` after terminal pump—calls `OptionsClass__WriteToINI @ 0x005FAD10`, restores the saved `g_GameActive` byte, and returns.

There is no rollback, cancel branch, result-dependent apply suppression, or child-result write. Re-entry packs whatever the prior accepted control snapshot put into the live singleton.

## 5. Primary proc 0x0055FDB0..0x0056047D

Ghidra has no function boundary at `0x0055FDB0`. The slice was therefore exhausted by disassembling the contiguous bytes through the terminal epilogue at `0x0056047A`; no function was created and no metadata was changed.

Every message first calls common shell handler `0x00622B50`. A nonzero common-handler result returns immediately. The primary-specific branches are:

### 5.1 WM_COMMAND

- `0x5CD` and `0x5CE`: free and zero the mode-pair list at `0x00ABCF58`, write the exact control id into the owner result, and return. The notification high word is not filtered.
- `0x686` (Main Menu/Back): only notification zero is admitted; it frees/zeros the list and writes result `0x5CB`.
- `0x6ED` (resolution combo): only `CBN_SELCHANGE = 1` is admitted. The proc centers the child relative to `g_hWnd`, obtains `CB_GETCURSEL` and `CB_GETITEMDATA`, indexes the `0x00ABCF58` width/height-pair array, writes the chosen pair to backup `+0x24/+0x28`, and writes it to the live object if it differs. It does not resize the window or invoke a display-mode setter.
- All other commands return zero. In particular, `IDCANCEL = 2` is ignored.

`WM_DESTROY = 2` frees and zeroes `0x00ABCF58`.

### 5.2 Custom initialization message 0x497

The proc sets initialization-suppression byte `0x00ABCF54` to zero, initializes whichever child handles exist, and sets it to one at the end. A missing child handle simply skips that control.

- Detail `0x52B`: range `0..1`; position is `0` only when DetailLevel is zero, otherwise `1`.
- Resolution `0x6ED`: enumerate 16-bit modes through `0x004A4900`, inclusive minimum `640x480`. Maximum is `4096x4096` when AllowHiResModes is true and `1024x768` otherwise. The false branch additionally admits only widths `640`, `800`, or `1024`. Pairs are sorted ascending by width then height and may preserve duplicates. The exact label format at `0x0082A2C4` is `%d x %d x 16`. Each item receives its original sorted-pair index as item data. The display loop continues after each width/height match and overwrites the remembered displayed-row index, so the **last displayed matching duplicate** becomes the selection; otherwise selection remains `-1`. Empty enumeration leaves an empty combo.
- Difficulty `0x50F`: range `0..2`, requested position from the live field; an out-of-range request is rejected and the fresh control remains at zero.
- UnitActionLines `0x601`, ShowHidden `0x604`, ToolTips `0x602`: `BM_SETCHECK` with normalized nonzero truth.
- Scroll `0x52A`: range `0..6`, requested position `6 - live ScrollRate`. The fresh control starts at zero; its set-position message accepts only an in-range request, so an out-of-range projection is rejected and remains zero rather than clamping to an endpoint.
- Audio availability is `0x00407000` / `DAT_0087E728 != 0`. Score `0x52F`, Sound `0x532`, and Voice `0x536` each receive range `0..10` and are enabled/disabled together from that result.
- Each initial volume request is produced by `profile_f32 * 10.0 + 0.5` followed by helper `0x007C5F00` under x87 control word `0x0E7F`, hence truncation toward zero. The fresh control starts at zero and its set-position message accepts only `0..10`; an out-of-range request is rejected and remains zero, not clamped to an endpoint. For NaN/infinity, the x87 indefinite qword has low dword zero and selects zero. A Rust-native projection must therefore map non-finite or out-of-range results to zero and preserve an in-range truncated result.

The live text tables are:

| Control | Position -> CSF token |
|---|---|
| Detail `0x52B` | `0 TXT_LOW`, `1 TXT_HIGH` (the proc forces every nonzero position to table index 2) |
| Difficulty `0x50F` | `0 TXT_EASY`, `1 TXT_NORMAL`, `2 TXT_HARD` |
| Scroll `0x52A` | `0 TXT_SLOWEST`, `1 TXT_SLOWER`, `2 TXT_SLOW`, `3 TXT_MEDIUM`, `4 TXT_FAST`, `5 TXT_FASTER`, `6 TXT_FASTEST` |

### 5.3 WM_HSCROLL 0x114

Only request code `SB_THUMBTRACK = 5` is handled.

Custom initialization sets trackbar positions but never writes the three linked value statics, so every fresh parent keeps the resource captions `GUI:HigherDetail`, `GUI:Harder`, and `GUI:Faster` regardless of the projected positions. Generic trackbar code notifies the parent only after an actual quantized value change. Therefore only the first changed thumbtrack swaps a caption to the tables below; an unchanged movement leaves the current caption untouched.

- Detail, Difficulty, and Scroll replace their linked label with the token above.
- Score, Sound, and Voice preview only after `0x00ABCF54 != 0`. The proc reads the current slider position and calls the matching setter with `position * 0.1`. Score has no cue. Sound and Voice pass preview=true: after storing the volume, both play Rules `[AudioVisual] GenericBeep`; Sound uses call-local volume `1.0`, while Voice uses the selected voice multiplier. Initialization emits no preview, and accepted apply passes preview=false, so it emits no beep.
- An unrecognized sender or any other request code returns zero without mutation.

The preview setters are `0x005FA4A0` for Score, `0x005FA510` for Sound, and `0x005FA590` for Voice. `ini/rulesmd.ini` contains `GenericBeep=GenericBeep`. Preview does not persist and the accepted-boundary apply repeats all three setters without the cue.

## 6. Accepted-control projection at 0x0055FAA0

The apply helper asks for every child separately. A null handle preserves that profile field.

1. Detail `0x52B`: position zero becomes `0`, nonzero becomes `2`; a change calls display refresh `0x004AE450`.
2. Difficulty `0x50F`: store position directly.
3. UnitActionLines `0x601`: true only for `BM_GETCHECK == 1`, then call `TechnoClass__SetDrawHealthBarsFlag`.
4. ShowHidden `0x604` and ToolTips `0x602`: same exact checkbox truth rule.
5. Scroll `0x52A`: store `6 - position`.
6. Score `0x52F`, Sound `0x532`, Voice `0x536`: call setters with `position * 0.1`.

After the Score setter, an exact resulting zero selects Theme logical active identity `+0x00`, falling back to retained logical identity `+0x04` only when active is `-1`; it then calls `ThemeClass__Queue_Song @ 0x00720B20`, followed by `ThemeClass__Stop(0) @ 0x00720EA0`. Both callees first require all three admission gates: `DAT_00A8EC74 != 0` (Theme data initialized), `FUN_00407000() != 0` / `DAT_0087E728 != 0` (audio device available), and `DAT_00A8ED64 == 0` (startup suppression inactive). Queue writes the chosen identity to pending `+0x08` only after those gates and never starts it inline. Stop repeats the same gates, then branches on logical active `+0x00`, not physical StreamPlayer activity: when active is `-1`, Stop returns and preserves any just-written pending identity; otherwise it stops and clears active/retained/pending `+0x00/+0x04/+0x08`. If any shared gate fails, both calls return without changing those logical slots. In the ordinary audio-ready launcher target, `DAT_00A8EC74` was set to one after Theme MIX initialization, `DAT_00A8ED64` remains zero, and the device predicate passes; `-NOAUDIO`, uninitialized audio, and the non-default startup-suppressed route exercise the preserving early return instead. `ThemeClass__AI @ 0x007209D0`, `ThemeClass__Next_Song @ 0x00720A80`, and play helper `0x00720BB0` prove `+0x04` is retained/requested Theme identity, not a generic physical-player flag. Sound and Voice have no analogous stop behavior.

## 7. Backup exhaustion

Every aligned address from `0x00ABCE70` through `0x00ABCF24` was queried, together with the unaligned interior fields used by the class layout.

- Full-object writes: the owner `REP MOVSD` copy and the static/default initialization path.
- Default thunk `0x0055F970`: load `ECX = 0x00ABCE70` and tail-jump to `OptionsClass__SetDefaults @ 0x005FA350`. It is initialization, not restoration.
- Runtime reads: only `0x00ABCE94` and `0x00ABCE98` in primary resolution initialization.
- Runtime writes: chosen combo pair writes those same two fields.
- No other backup field has a runtime consumer, comparison, restore, or child callback.

This proves that neither full nor selected rollback exists. Because both child branches return only through the owner and the owner recopies live state before re-entry, they cannot restore through an indirect path omitted from the primary proc.

## 8. Resource 0xD5 inventory

The PE contains a `DIALOGEX` resource with style `0x40000040`, extended style zero, rectangle `0,0,533,369` DLU, font `8 pt MS Sans Serif`, charset `1`, and exactly 31 children.

| ID | Class/style | Caption | DLU rect | Verified role/state |
|---|---|---|---|---|
| `0x686` | Button `0x5000000B` | `GUI:MainMenu` | `425,346,108,23` | Back; result `0x5CB` |
| `0x5CE` | Button `0x5000000B` | `GUI:Keyboard` | `424,122,108,23` | keyboard child |
| `0x694` | Static `0x50020001` | `GUI:OptionsMenu` | `424,1,108,10` | right-panel heading |
| `0x5CD` | Button `0x5000000B` | `GUI:Network` | `424,149,108,23` | Network child `0xD7` |
| `0x52A` | trackbar `0x50000018` | `Slider2` | `238,203,120,13` | ScrollRate |
| `-1` | Static | `GUI:ScrollRate` | `238,188,60,10` | scroll label |
| `0x52B` | trackbar `0x50000018` | `Slider3` | `89,53,120,13` | coarse DetailLevel |
| `-1` | Static | `GUI:VisualDetails` | `89,38,60,10` | detail label |
| `0x673` | Static | `GUI:HigherDetail` | `149,38,60,10` | dynamic detail value |
| `0x672` | Static | `GUI:Faster` | `298,188,60,10` | dynamic scroll value |
| `0x50F` | trackbar `0x50000018` | `Slider3` | `92,128,120,13` | Difficulty |
| `-1` | Static | `GUI:Difficulty` | `92,113,60,10` | difficulty label |
| `0x670` | Static | `GUI:Harder` | `152,113,60,10` | dynamic difficulty value |
| `0x601` | Button `0x50008003` | `GUI:TargetLines` | `93,203,130,10` | UnitActionLines checkbox |
| `0x602` | Button `0x50008003` | `GUI:Tooltips` | `93,188,130,10` | ToolTips checkbox |
| `0x604` | Button `0x50008003` | `GUI:ShowHidden` | `93,218,130,10` | ShowHidden checkbox; visible/enabled |
| `0x6ED` | ComboBox `0x50000213` | empty | `234,53,120,74` | resolution list |
| `-1` | Static | `GUI:MusicVolume` | `82,274,85,10` | score label |
| `0x52F` | trackbar `0x50010018` | `Slider1` | `82,289,85,13` | ScoreVolume |
| `-1` | Static | `GUI:SoundVolume` | `179,274,85,10` | sound label |
| `0x532` | trackbar `0x50010018` | `Slider2` | `179,289,85,13` | SoundVolume |
| `-1` | Static | `GUI:VoiceVolume` | `277,274,85,10` | voice label |
| `0x536` | trackbar `0x50010018` | `Slider2` | `277,289,85,13` | VoiceVolume |
| `0x6EE` | Static | `GUI:DisplayOptions` | `20,18,140,10` | group heading |
| `0x6EF` | Static | `GUI:GameOptions` | `20,93,298,10` | group heading |
| `0x6F0` | Static | `GUI:UIOptions` | `20,168,298,10` | group heading |
| `0x6F1` | Static | `GUI:AudioOptions` | `20,254,298,10` | group heading |
| `0x6F2` | Static | `GUI:SetResolution` | `234,38,120,10` | resolution heading |
| `0x695` | Static | `GUI:Blank` | `2,355,303,12` | shared footer/help text |
| `0x71C` | Static `0x50000007`, exstyle `0x20` | empty | `446,29,61,33` | owner-draw `SDWRNANM` decoration |
| `0x603` | Button `0x48008003` | `GUI:AssumeNoObserve` | `239,187,130,10` | hidden and disabled dormant checkbox |

`0x603` is never initialized or read by the primary proc and lacks `WS_VISIBLE` while carrying `WS_DISABLED`. It is dormant TS-era residue. `ShowHidden` `0x604` is an ordinary visible retail control; the older report's “debug/not shown” classification is wrong.

## 9. Shell composition and retail assets

Common handler `0x00622B50` owns initialization/subclassing, paint, destruction, footer/help, and owner-draw dispatch. `ShellDialogConfigureMode1ForKnownResource @ 0x0060C540` admits resource `0xD5` and configures mode 1. The `0xD5` defaults selected through `0x0060CF00` are shell palette conversion `0x00B0FBCC`, small/large main screen `MNSCRNS/MNSCRNL`, and background-overlay record `0x0072E280`.

`0xD5` is absent from the optional allowlists in `0x0060CAF0`, `0x0060C930`, `0x0060CCC0`, and `0x0060CDB0`. Therefore top highlight, minimap, radar, and the extra optional flag are all false. Paint handler `0x00621E90` reads record byte `+0xD4`, passes true to the right-panel draw when that byte is zero, and D5 configuration leaves it zero. Consequently the repeated `SDBTNANM` frame-10 row overlay is **not** drawn for this dialog; the child owner-draw buttons render themselves. Record byte `+0xD5` gates a different later optional overlay and is not the cause of this parameter.

The ordinary order is:

| Order | Owner | Asset/frame and condition | Palette | Role |
|---:|---|---|---|---|
| 1 | `0x0072E450` | fill logical-shell margins | n/a | cleared container |
| 2 | `0x0072E450` | `SDTP` frame 0 | `SHELL.PAL` | right-panel top |
| 3 | `0x0072E450` | repeated `SDBTNBKGD` frame 0 rows | `SHELL2.PAL` | right-panel body |
| 4 | `0x0072E450` | `SDBTNANM` frame 10 only when parameter is zero | `SDBTNANM.PAL` | inactive for `0xD5` |
| 5 | `0x0072E450` | `SDBTM` frame 0 | `SHELL.PAL` | right-panel bottom |
| 6 | `0x0072E450` | `LWSCRNS` at width 640, else `LWSCRNL` | `SHELL.PAL` | lower strip |
| 7 | `0x0072E730` | `MNSCRNS` at width 640, else `MNSCRNL` | `SHELL.PAL` | dialog background overlay |
| 8 | common owner-draw | `SDWRNANM` current frame in control `0x71C` | `SHELL2.PAL` | animated warning decoration |
| 9 | common control paths | checkboxes, tracks, combo, text, buttons | embedded/bound palettes | interactive foreground |

Layout uses a centered/clipped logical `800x600` region: horizontal margin is `(width-800)/2` at widths `>= 1024` (the binary test is `0x3FF < width`), and vertical margin is `(height-600)/2` at heights `>= 768` (test `0x2FF < height`). Thus `1024x768` uses origin `(112,84)`. The right panel is right-aligned to the effective logical region; button rows fill the height between `SDTP` and `SDBTM`; the lower strip is anchored at the logical bottom. The resource dialog itself is centered by the shell helper.

Control `0x71C` is configured as owner-draw static kind 4. Its loader pointer resolves `SDWRNANM.SHP` and the palette path resolves `SHELL2.PAL`. It starts at frame zero with running state enabled. The bootstrap timer is 1 ms and the first qualifying timer requires elapsed time strictly greater than 1 ms; it then reads the SHP's 91-frame count and installs a nominal 91 ms timer. Paint draws the current frame, increments afterward, and wraps at 91. Display order is `0..90` cyclic and paint-driven; `0xD5` never sends the stop message.

### 9.1 Owner-draw control behavior

Common `WM_PAINT` draws the parent/right-panel composition first; invalidated children paint afterward. `0x0060F9A0` subclasses button style `(style & 0xB) == 0xB`, checkbox style `(style & 3) == 3`, trackbar, ComboBox, and Static controls onto the corresponding common owner-draw procedures.

For `0x686/0x5CD/0x5CE`, `0x0060A330` uses the SessionClass scenario-active byte at `+0x30D8`, not `g_GameActive`. Ordinary pre-session launcher entry selects type 1:

| State | SHP / palette | Released | Hover/flash | Pressed |
|---|---|---:|---:|---:|
| normal D5, scenario inactive | `SDBTNANM.SHP` / conversion `0x00B0FBDC` | 2 | 3 | 4 |
| hypothetical D5 reuse while scenario active | `SIDEBTTN.SHP` / `SIDEBAR.PAL` | 0 | 2 | 1 |

The normal draw order is cached parent background, SHP with `CC_Draw_Shape` flags `0x400`, then BitFont label. Pressed text shifts +2 px horizontally and +4 px vertically. Disabled controls use grey text and block input; mouse down/double-click plays the GUI-button sound unless the control record suppresses it.

Checkboxes `0x601/0x602/0x604` use `CUE_I.PCX` unchecked and `CCE_I.PCX` only when state equals 1. D5 never selects the right/left variants. The icon is 18x18 and text starts 26 px after it. The hit test is strictly the unsigned `x < 18 && y < 18` icon square: clicking the label does nothing. A toggle normalizes `old != 1`, invalidates, plays the GUI-click sound, and sends parent `WM_COMMAND` with the new bool in the high word. Dormant `0x603` is subclassed but neither paints nor receives input while hidden/disabled.

Trackbars use `TRAKGRIP.PCX` and optional `TROFL/TROFM/TROFR` numeric plaque pieces. D5 sends message `0x4AC` with zero to Detail `0x52B`, Difficulty `0x50F`, and Scroll `0x52A`, clearing the plaque flag and giving all three reserve zero. Score, Sound, and Voice retain the plaque and reserve 50 client pixels. Generic `GenericClick` change sound is suppressed for all three audio sliders and retained for Detail/Difficulty/Scroll.

For client width `w`, reserve `r`, minimum zero, maximum/range `n`, and step one, `TrackBar_ProcessMouse @ 0x0061D950` uses:

`usable_span = max(1, w - r - 13)`

`track_x = clamp(raw_mouse_x - 6, 1, w - r - 12)`

`relative = min(n, ((track_x - 1) * (n + 1)) / usable_span)`

followed by the general step quantizer. Initial `WM_LBUTTONDOWN` explicitly requires only `mouse_y > client_bottom - 18`; the normal Win32 client delivery supplies the implicit upper client bound. Mouse-down inside the current thumb captures and begins a tracking drag without jumping. Once captured, `WM_MOUSEMOVE` recomputes from x without reapplying a y-strip test, so thumb dragging remains x-driven outside that strip. A rail mouse-down jumps once but does not become a tracking drag. Only an actual quantized change sends parent `WM_HSCROLL` with low word `5` and high word absolute value.

At the ordinary 96-DPI D5 baseline, runtime client widths are 180 pixels for the 120-DLU Detail/Difficulty/Scroll controls and 128 pixels for the 85-DLU audio controls:

| Controls | Range | Width/reserve | Usable span/max | Raw-x thresholds |
|---|---:|---|---|---|
| Detail | `0..1` | `180/0` | `167/168` | position 1 at `91` |
| Difficulty | `0..2` | `180/0` | `167/168` | positions 1/2 at `63/119` |
| Scroll | `0..6` | `180/0` | `167/168` | positions 1..6 at `31/55/79/103/127/151` |
| Score/Sound/Voice | `0..10` | `128/50` | `65/66` | positions 1..10 at `13/19/25/31/37/43/49/55/61/67` |

The resource widths are DLUs while the owner reads runtime `GetClientRect`, so non-96-DPI tests must use the width-parametric formula rather than hardcoding these canonical pixels. The set-position message rejects out-of-range values rather than clamping; the set-range path clamps the current position. Disabled audio controls receive the normal grey/alpha owner-draw state and block input.

Combo `0x6ED` has a primitive/bevel collapsed face, fixed 24-pixel height, and a 20-pixel arrow region. Selected text starts at x+2 and is truncated to fit before that arrow. Mouse down plays `GUIComboOpen` before hit-testing, but the popup toggles only for strict `x > client_width - 20`; clicking the text area does not open it. The popup is one pixel below the face, uses whole-row height, adds a 20-pixel owner-draw scrollbar on overflow, clamps its top index, and sends parent notification 1 only after a valid row selection. An empty list/selection `-1` paints no selected text.

Arrow helper `0x00620720` maps direction zero/nonzero to down/up and pressed zero/nonzero to released/pressed. The D5 callsite passes one recovered local as both arguments, admitting only `DNARROWR.PCX` for zero or `UPARROWP.PCX` for nonzero. D5 never requests the `G` prefix. The producer/meaning of the large-stack local is not recovered, so no “open means nonzero” claim is made.

Title static `0x694` is kind 1 with initial reveal index 1, 30 ms interval, step 1, tail 8, no sound, and starts after the shell transition via custom `0x4EC -> 0x4EE`. Footer `0x695` is also kind 1 during normal no-scenario D5 (15 ms, step 3, tail 16); the resource text begins blank and hover/status command `0x4B2` supplies it. Value labels `0x670/0x672/0x673` and ordinary captions remain kind 0.

For `0x71C`, timer invalidation and frame advancement are deliberately separate: the first qualifying timer requires elapsed `> 1`, then switches to nominal 91 ms invalidation, while every admitted paint draws and advances the frame. Extra unrelated paints can therefore advance the animation sooner than one frame per timer. Destruction kills the timer.

### 9.2 Asset evidence

The release asset CLI read and rendered these exact retail entries:

| Asset | Retail catalog source | Bytes | Canvas | Frames | Palette / target use |
|---|---|---:|---:|---:|---|
| `MNSCRNS.SHP` | `ra2.mix -> neutral.mix` | 211,488 | 472x448 | 1 | `SHELL.PAL` |
| `MNSCRNL.SHP` | same | 359,008 | 632x568 | 1 | `SHELL.PAL` |
| `SDTP.SHP` | same | 66,920 | 168x199 | 2 | `SHELL.PAL`; frame 0 |
| `SDBTNBKGD.SHP` | same | 7,088 | 168x42 | 1 | `SHELL2.PAL` |
| `SDBTNANM.SHP` | same | 111,800 | 156x42 | 17 | `SDBTNANM.PAL`; no D5 row overlay |
| `SDBTM.SHP` | same | 10,952 | 168x65 | 1 | `SHELL.PAL` |
| `LWSCRNS.SHP` | same | 15,136 | 472x32 | 1 | `SHELL.PAL` |
| `LWSCRNL.SHP` | same | 20,256 | 632x32 | 1 | `SHELL.PAL` |
| `SDWRNANM.SHP` | same | 446,272 | 92x53 | 91 | `SHELL2.PAL` |

Every listed SHP frame is full-canvas, format 0, uncompressed, and contains no index-zero pixels. All 91 `SDWRNANM` rendered frames are unique. All 17 `SDBTNANM` frames rendered; frames 10 and 16 are pixel-identical.

Verified control PCXs include `CUE_I/CCE_I` (checkbox, 18x18), `TROFL/TROFM/TROFR` (track plaque, 8x24/114x24/10x24), `TRAKGRIP` (12x22), the down/up arrow families (18x22), and scrollbar `SBGRIPT/M/B`. The combo callsite at `0x006178F8..0x0061791D` passes the same recovered local as both direction and pressed arguments to `0x00620720`; that helper maps `(0,0)` to `DNARROWR.PCX` and nonzero/nonzero to `UPARROWP.PCX`, with a `G` prefix only when the control record's disabled byte `+0xCD == 1`. D5 never sends the disable message `0x4F1`, so the normal prefix is non-`G`; the producer of that aliased local is not cleanly recovered, so the exact open-state arrow is MEDIUM confidence and no `DNARROWP` binding is asserted. `localmd.mix` wins over differing `local.mix` copies for checkbox icons, grip, arrows, and scrollbar pieces; substituting the base copies is visible drift.

`SHELL.PAL` and `SHELL2.PAL` are reachable `ra2.mix -> cache.mix` winners. `SDBTNANM.PAL` is catalogued in `neutral.mix`. The CLI's `palette-for` heuristic incorrectly falls back to `unittem.pal` for `MNSCRNS` and `SDWRNANM`; the live binary bindings above are authoritative.

The CLI currently marks `ra2.mix -> neutral.mix` as `name_lookup_reachable:false`, so `find` returns `winner:null` plus a catalog-only record for these shell SHPs even though `info` and explicit `render` read their exact bytes. Native production mount precedence is independently established by `0x0072E450` mounting `NTRLMD.MIX` then `NEUTRAL.MIX` before `0x0072EB50` loads the assets. The CLI-model reachability mismatch is a tooling residual, not uncertainty about the retail bytes or native binding; it must be fixed only if the implementation chooses that CLI path as a production dependency.

### 9.3 Asset role matrix

| Asset | Exact frame(s) | Loaded | Drawn | Visible in ordinary D5 | Content / preview | Chrome / container | Overlay | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `MNSCRNS/MNSCRNL` | 0 | yes | yes | one by shell width | main dialog backdrop | yes | background overlay | no | `0x72E730` and loader |
| `SDTP` | 0 | yes | yes | yes | right-panel top | yes | no | frame 1 | `0x72E450` |
| `SDBTNBKGD` | 0 repeated | yes | yes | yes | row backing | yes | no | no | `0x72E450` |
| `SDBTNANM` | button 2/3/4 | yes | child owner-draw | yes | button state | no | foreground | row frame 10 inactive | `0x60A330/0x612B70` |
| `SDBTM` | 0 | yes | yes | yes | right-panel bottom | yes | no | no | `0x72E450` |
| `LWSCRNS/LWSCRNL` | 0 | yes | yes | one by shell width | lower strip | yes | no | no | `0x72E450` |
| `SDWRNANM` | 0..90 cyclic | yes | child owner-draw | yes | animated decoration | no | foreground | no | `0x60A5B0/0x6153E0` |
| `SIDEBTTN` | 0/2/1 | available | conditional child draw | no in ordinary pre-session D5 | scenario-active button state | no | foreground | yes for target | scenario predicate `+0x30D8` |
| `CUE_I/CCE_I` | single | yes | checkbox owner-draw | yes by state | checkbox state | no | foreground | `CCE_IR/IL` | `0x6163A0` |
| `TROF* / TRAKGRIP` | single | yes | track owner-draw | yes by control | numeric value / grip | no | foreground | Detail/Difficulty/Scroll plaques hidden; audio plaques active | `0x61D950` and D5 `0x4AC` calls |
| arrow family | single | yes | combo owner-draw | yes | combo affordance | no | foreground | `G*` disabled prefix unused | `0x617250/0x620720` |

## 10. Rules/art authority

No repository rules/art file owns the launcher profile. A cold exact-key scan found only `GameSpeed` in `ini/rules.ini` and `ini/rulesmd.ini` and object-level `DetailLevel=2` entries in `art.ini/artmd.ini`. Those are simulation/art authorities with unrelated meanings. `RA2MD.INI [Options]/[Video]/[Audio]` remains the process-profile authority already implemented by `RetailOptionsProfile`.

## 11. Current Rust production evidence

At `origin/main c9b97157` (production source unchanged from `ae2f15c1`; the intervening commits edit only `README.md`):

| Surface | Verified current state | Required parent delta |
|---|---|---|
| `src/ui/main_menu_dialogs.rs` | `OptionsDialogState` is a unit struct; draw text explicitly says resolution/volumes/scroll/tooltips/game speed are unimplemented; only action is Close; missing CSF keys currently produce a synthetic marker rather than the supplied fallback | retained bounded controls, exact initial/dynamic labels and readable fallbacks, preview/cue events, Back/Network/Keyboard results |
| `src/app/shell_main_menu.rs` | Options opens `Default`; Close simply drops state | construct from singleton profile; apply after parent result; loop child routes; final persist |
| `src/app/frontend/state.rs` | holds only `Option<OptionsDialogState>` | sufficient storage location; no second owner |
| `src/app/persistence/options_profile.rs` | complete `RetailOptionsProfile` and byte-preserving commit exist | reuse; preserve non-parent fields |
| `src/app/persistence/options.rs` | process-profile projection/persist helpers and presentation owners exist | add launcher projection/transaction beside in-game transaction |
| `src/audio/sfx.rs` | distinct `set_sound_volume` and `set_voice_volume`; call-local playback volume exists | live slider preview, accepted reapply, and ordered `GenericBeep` cues |
| `src/audio/music.rs` | `MusicPlayer::set_volume` and physical-player `stop` exist, but no distinct Theme active/retained/pending identities | score preview; logical queue-then-stop zero transition without physical-state conflation |
| `src/rules/ruleset.rs` | neighboring GUI cues are parsed; `[AudioVisual] GenericBeep` is not | add the existing Rules key to audio-visual ownership |
| `src/app/frontend/skirmish_session.rs` | `OfflineSkirmishRuntime` owns cached six-field local multiplayer preferences and reads them at construction | narrow re-read before the parent-owned Network child route |
| `src/app/handler.rs` | Escape closes every egui-only main-menu modal; `CloseRequested` exits directly | options-specific Escape consume/no-op; terminal close applies/writes open launcher state |
| Winit production surface | no `video_modes`/monitor enumeration caller exists | Rust-native pair enumeration at open; no immediate resize on selection |

This parent is process/shell state only. It must not enter `sim/`, replay data, save data, command scheduling, or deterministic ticks.

## 12. Edge and failure semantics

- Empty mode enumeration: combo is empty, selection `-1`, width/height unchanged.
- Current pair absent: list exists but selection remains `-1`.
- Invalid combo selection: native code assumes a valid `CBN_SELCHANGE` item and would index the list; Rust must not synthesize a selection event without a valid item.
- Duplicate enumerated pairs may remain after sorting; the last displayed pair matching backup width/height becomes the selected displayed row.
- Null child handles skip initialization/apply and preserve the field.
- Out-of-range initialization requests are rejected by the fresh trackbar and leave its position at zero; the retained profile is not normalized until unconditional parent apply projects that control zero through the field mapping (Difficulty/volumes become zero, while ScrollRate becomes `6 - 0 = 6`).
- Audio unavailable disables all three sliders together; the retained volume fields remain unchanged unless handles still yield accepted positions.
- First pass and every child re-entry copy current live state to the backup.
- Modal-pump termination with result still `-1` nevertheless applies controls, writes, restores `g_GameActive`, and exits.
- Primary creation failure retries; Network-child creation failure skips that child and returns to the primary.
- Default-dialog Escape emits ignored `IDCANCEL` and therefore does not apply, write, close, or roll back.
- No replay, save, restore, scenario tick, or simulation owner participates.

## 13. Coverage ledger

| Area | Status | Evidence |
|---|---|---|
| Main-menu liveness | verified | decompile `0x0052D9A0`, case 5 |
| owner loop/results/terminal paths | verified | decompile + disassembly `0x0055FC80` |
| primary proc every byte/branch | verified | 554 instructions, `0x0055FDB0..0x0056047A` |
| accepted projection | verified | decompile/disassembly `0x0055FAA0` |
| Score-zero Theme identities and admission gates | verified | `0x0055FAA0`, `0x007209D0`, `0x00720A80`, `0x00720B20`, `0x00720BB0`, `0x00720EA0`; `DAT_00A8EC74`, `DAT_0087E728`, `DAT_00A8ED64` |
| Sound/Voice `GenericBeep` preview | verified | primary `0x0056000F..0x00560110`, setters `0x005FA510/0x005FA590`, `rulesmd.ini` |
| initial vs changed captions | verified | resource statics plus `0x0055FF68..0x00560008`, table `0x0082A248` |
| trackbar geometry/admission | verified | `TrackBar_ProcessMouse @ 0x0061D950`, D5 plaque-clear call |
| exact combo-arrow visual state | touched-not-exhausted / EXACTIFICATION-RESIDUAL | callsite admits `DNARROWR` or `UPARROWP`, but the shared local's producer/meaning remains unrecovered |
| backup full/interior xrefs | verified | aligned and field-interior `get_xrefs_to` sweep |
| Network/keyboard child identity | verified-parent-boundary | `0x006980C0`, RT_DIALOG `0xD7` / `0xA3`; internals non-scope |
| RT_DIALOG `0xD5` | verified | direct PE resource parse, 31/31 children |
| common shell flags/paint | verified | `0x00622B50`, `0x0060C540`, four allowlists, `0x00621E90`, `0x0072E450`, `0x0072E730` |
| control `0x71C` animation | verified | common owner-draw init/timer/paint plus 91-frame asset |
| retail asset contents | verified | release CLI `find/info/compare/render` and all-frame inspection |
| native retail asset mount | verified | live binary mounts `NTRLMD` then `NEUTRAL` |
| CLI neutral-name reachability | known residual | catalog-only `winner:null` despite readable entry |
| rules/art authority | verified-negative | exact-key `rg` scan |
| Rust production state | verified | `origin/main c9b97157` source scan plus README-only diff from `ae2f15c1` |

## 14. Open-question closure and exactification residual

All 32 starting semantic-parent questions are closed. One touched visual exactification is explicitly deferred rather than folded into the closure claim:

- **Combo arrow state (`touched-not-exhausted`, EXACTIFICATION-RESIDUAL):** `0x006178F8..0x0061791D` passes one recovered local as both arrow-helper arguments, admitting `DNARROWR.PCX` for zero or `UPARROWP.PCX` for nonzero, but the producer/meaning of that large-stack local is not recovered. Popup admission/selection semantics are verified; an exact open/closed filename claim is not. This does not block the code-native semantic parent, but it blocks any future claim of exact combo-arrow pixels until a narrow visual investigation resolves it.

The original questions close as follows:

- `OQ-01..03`: active caller, resource/proc creation, and per-pass copy are verified.
- `OQ-04..05`: exhaustive backup xrefs prove only resolution reads/writes and no restoration.
- `OQ-06..08`: every primary message/command is covered; Back is final apply/write, not reset; Escape is ignored.
- `OQ-09..18`: every slider, checkbox, label table, volume projection, preview gate, range, rounding, and sender branch is verified.
- `OQ-19`: premise corrected—`0x5CD` opens Network `0xD7`; resolution is `0x6ED` in the parent.
- `OQ-20`: `0x5CE -> 0x005FBEF0 -> 0xA3/0x005FB320` keyboard child; parent changes remain live.
- `OQ-21..23`: null handles, empty/invalid lists, clamps, destroy, terminal pump, repeated loops, and game-active lifetime are covered.
- `OQ-24`: strictly process/UI-owned.
- `OQ-25`: hidden disabled `0x603` is dormant; `0x604` is live.
- `OQ-26..28`: complete resource, ordered composition, and every optional flag are covered.
- `OQ-29..30`: current placeholder and reusable production seams are mapped.
- `OQ-31`: the old rollback claim is explicitly disproved.
- `OQ-32`: exhaustive backup-interior references and owner-only child returns exclude indirect restoration.

### Adversarial cold checks

1. Re-decompiled owner `0x0055FC80` without relying on the prior report: apply remains unconditional and no restore branch appears.
2. Cold-read `0x0055FAA0`: `0x52F` is Score, `0x532` Sound, `0x536` Voice; this catches the older swapped mapping.
3. Re-disassembled the no-boundary primary proc through its actual epilogue: `0x5CD` is Network and `0x6ED` is the only resolution selector.
4. Rechecked all four optional-shell allowlists: `0xD5` is absent from each, so no optional top/minimap/radar layer is active.
5. Parsed the PE resource independently of Ghidra labels: exactly 31 children, including visible `ShowHidden` and hidden/disabled `AssumeNoObserve`.
6. Rendered every `SDWRNANM` frame with the binary-bound palette: 91 unique frames; no inferred unit palette accepted.
7. Cold-scanned current `origin/main` rather than the feature checkout: the Options surface is still open-level only.
8. Re-read the Detail table use rather than trusting the first report pass: D5 forces nonzero to table index 2, proving `TXT_HIGH`; the earlier `TXT_MEDIUM` claim was wrong and is corrected above.
9. Followed duplicate modes through enumeration, sort, whitelist, add, item-data, and final selection: no deduplication occurs and the last displayed match wins.
10. Decompiled the preview setters and Theme queue/stop consumers: Sound/Voice cues, the three shared Theme/audio admission gates, and logical active/retained/pending state are load-bearing; none may be collapsed into a generic volume setter or physical-player check.

Post-correction semantic-parent zero-add pass: re-scanning owner callees, every primary branch, backup interior xrefs, resource children, shell allowlists, visual loaders, asset bindings, Rules/audio keys, and Rust callers added no further semantic-parent question. The exact combo-arrow visual state remains the named touched-not-exhausted residual above.

## 15. Implementation handoff

The dependency-coherent Rust mechanism is the launcher Options **parent transaction**, not the Network or keyboard child implementations:

- Build a bounded dialog snapshot from the retained singleton profile and a Rust-native enumerated/sorted resolution-pair list.
- Keep the profile authoritative; dialog state is a projection, never a second persisted settings owner.
- Preserve resource captions until a changed thumbtrack; Detail's changed tokens are `TXT_LOW/TXT_HIGH`.
- Preview admitted audio slider motion through existing Music/SFX consumers; Sound/Voice play the parsed `GenericBeep` afterward with native local multipliers, while Score and accepted reapply are cue-silent.
- Model Theme logical active/retained/pending identities separately from rodio activity so Score zero queues active-else-retained before logical stop only when Theme-initialized, common-audio-available, and startup-suppression-inactive gates all pass; a failed gate preserves every logical slot.
- Preserve native trackbar admission through pure integer helpers: initial down uses the strict lower y bound, captured thumb motion remains x-driven, rail clicks jump once, Detail/Difficulty/Scroll use reserve 0, audio uses reserve 50, and only changed values notify/cue.
- On Back, Network, Keyboard, or terminal close: pack the parent snapshot and apply existing consumers. Network/Keyboard route to distinct child seams and re-enter without persisting; final Back/terminal persists once.
- Before Network routing, re-read only the six cached local `[MultiPlayer]` preferences through `OfflineSkirmishRuntime` ownership.
- Do not resize/apply a display mode when the combo selection changes.
- Consume Escape while this parent is open.
- Preserve fields absent from this parent and preserve unrelated INI bytes through the existing writer.
- If child surfaces remain deferred, expose explicit route actions without inventing their internals.
- Keep all behavior out of `sim/` and test the projection, result routing, preview, apply ordering, non-finite/out-of-range rejection edges, Escape admission, and terminal commit through headless operation seams.

The visual ledger establishes the retail target, but exact SHP composition is not required to prove the parent transaction unless the implementation elects to replace the established code-native shell style in this same mechanism. The exact combo-arrow state and CLI neutral-name reachability remain named visual/tooling residuals; neither is a semantic blocker for an egui parent that does not claim pixel parity or consume the asset CLI at runtime.

## 16. Ghidra annotation candidates

No metadata mutation was made or authorized. Candidate names for a later certainty-gated sync:

- `0x0055FDB0`: launcher Options `0xD5` primary dialog proc (currently no function boundary)
- `0x00ABCF54`: launcher audio-preview initialization suppression flag
- `0x00ABCF58`: launcher resolution width/height pair list
- `0x0082A27C`: launcher resolution combo HWND

Creating a function at `0x0055FDB0` should be a separate metadata decision, not a side effect of this investigation.

## Sources and literal validation

- Active `gamemd.exe` identified above; Ghidra program info reported PE x86, image base `0x00400000`, 10,073 functions.
- Ghidra calls: `decompile_function 0x0052D9A0`, `decompile_function 0x0055FC80`, `decompile_function 0x0055FAA0`, `disassemble_bytes 0x0055FDB0..0x0056047D` (554 instructions, not truncated), Theme `0x007209D0/0x00720A80/0x00720B20/0x00720BB0/0x00720EA0`, preview setters `0x005FA510/0x005FA590`, trackbar owner `0x0061D950`, duplicate enumeration/sort/selection ranges, `get_xrefs_to` for every backup-aligned/interior address, common-shell decompiles/disassemblies, and cold string/memory reads.
- Retail resource parser: RT_DIALOG `0xD5` / language `0x0409` direct PE bytes.
- Release asset CLI commands: `asset archives`, `asset find <name>`, `asset info <name> --limit 128`, `asset compare <name> --no-render`, and explicit `asset render` with `SHELL.PAL`, `SHELL2.PAL`, or `SDBTNANM.PAL`.
- All-frame sheets: `C:\Users\enok\AppData\Local\Temp\vera20k-d5-asset-audit-01a05797\render\SDWRNANM.SHP\SDWRNANM.SHP.sheet.png` and `...\SDBTNANM.SHP\SDBTNANM.SHP.sheet.png`.
- `docs/research/OPTIONS_PROFILE_TRANSACTION_GHIDRA_REPORT.md` (read fully; profile transaction, explicit OQ-31 deferral).
- `docs/research/OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md` (read fully as stale navigation evidence; rollback and several mappings corrected here).
- `origin/main c9b97157f42e932c9360fdad436e91a6fe5e0835` Rust production source; `ae2f15c1..c9b97157` is README-only.
- Literal rules/art scan: `rg -n -i "^\s*(GameSpeed|Difficulty|ScrollRate|DetailLevel|UnitActionLines|ShowHidden|ToolTips|SoundVolume|VoiceVolume|ScoreVolume|ScreenWidth|ScreenHeight)\s*=" ini --glob '*.ini'`; only rules GameSpeed and art DetailLevel collisions returned.
- Preview-cue authority: exact `[AudioVisual] GenericBeep=GenericBeep` in `ini/rulesmd.ini` plus setter-body reads of the corresponding Rules slot.
