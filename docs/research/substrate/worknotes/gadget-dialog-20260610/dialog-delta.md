---
title: Framework B (Win32 shell dialog substrate) — verify-and-delta pass
date: 2026-06-10
status: worknotes (verified-live this session, read-only Ghidra)
lane: dialog-delta (gadget-dialog-20260610 study)
prior: docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md (2026-05-31)
---

# Framework B delta pass — worknotes

Every claim below marked VERIFIED-LIVE was read from the binary this session via the
cited Ghidra MCP call. DOC-INHERITED = taken from a prior doc, not contradicted but not
re-read this session. Default verdict for any unproven difference is DRIFT.

**Headline finding:** the prior study's §2.4 record map mixes two offset conventions
(bucket-relative vs data-root-relative) row by row. Live, the registry bucket is the
0x208 allocation itself; the *data root* handed around by every helper is `bucket+4`
(`FUN_00624760` returns `bucket+1` as int*, verified via `decompile_function 0x00624760`).
A corrected, single-convention record map is in §3.

---

## 1. DELTA REPORT — prior-study load-bearing claims re-verified

| # | Prior claim | Verdict | Live evidence |
|---|---|---|---|
| 1 | Factory `0x00622650`: CreateDialogIndirectParamA (modeless, parent g_hWnd), push LIFO stack `DAT_00b72d28`(HWND)/`DAT_00b72d2c`(id) stride 8, depth `DAT_00b72f50`, top mirror `DAT_00b72f44/48`, then register keyboard routing `FUN_005d4e70` | **VERIFIED** | `decompile_function 0x00622650`. Template from `FUN_004a3b40`; entry pre-zeroed, depth incremented before create, rolled back on failure; lParam = `{ushort templateId; dword param3}` on stack. Never DialogBoxParam. |
| 2 | Common DLGPROC `0x00622b50` handles INITDIALOG/DESTROY/PAINT/hit-test | **VERIFIED + extended** | `decompile_function 0x00622b50`. Cases: `0x110` (two paths, see §2.1), `2` WM_DESTROY → `FUN_005d4ed0` unregister + `DAT_00a8ed8c--` + `SetFocus(g_hWnd)`, `0xf` WM_PAINT → record lookup, paint-suppress byte data+0xBC → ValidateRect-only, else `WM_PAINT_Handler` then deferred-slide byte data+0xBE → msg `0x4e2` to ctrl `0x71a` + `FUN_006071e0` + clear, `0x84` WM_NCHITTEST → tooltip refresh of ctrl `0x695` via `FUN_006040b0` (fallback CSF `ownrdraw.cpp` id `0x7a5`), `0x14`→1, `0x2b` WM_DRAWITEM → `FUN_006213a0`, `0x497`→ sends `0x4a9`, `0x4ec`→ `EnumChildWindows(FUN_0060aa60)`, `0x132..0x138` → `GetStockObject(4)`. **No WM_COMMAND case** — result codes are written by per-dialog procs (see #12). |
| 3 | Subclass setup `0x0060f9a0` classifies by Win32 class name, exact cascade of §2.2, Button by style bits, USERDATA override, installs shared wndproc | **VERIFIED** | `decompile_function 0x0060f9a0`. strcmp cascade order: ScrollBar→`0x0061C690`/kind 8, ListBox→`0x00618D40`/4, ComboBox→`0x00617250`/3, msctls_trackbar32→`0x0061D950`/7, msctls_progress32→`LAB_0061d6d0`/6, NewEdit→`0x00614B30`/1, Edit(`DAT_00833728`)→`0x00614190`/1, Static→`0x006153E0`/2, SysTabControl32→`LAB_006137d0`/0xA, Button: `(s&7)==7`→`0x0061E700`, `(s&0xB)==0xB`→`0x00612B70`, `(s&3)==3`→`0x006163A0`, `(s&9)==9`→`0x00616980` (all kind 0), msctls_hotkey32→`LAB_0061eca0`/9, no-match→`LAB_00612a60`/kind 0xB. `GetWindowLongA(-0x15)` (GWL_USERDATA) block: `[0]` = override class-name string, `[1]` copied to record data+0x54. `SetWindowLongA(hwnd, GWL_WNDPROC(-4), 0x610ca0)`. Allocates 0x208 record (`operator_new(0x208)` + `FUN_00623340`), inserts into `DAT_00ac1b00`, chain at bucket+0x204. Queries text via `CallWindowProcA(orig, WM_GETTEXT)`; sends `0x497`. |
| 4 | Shared wndproc `0x00610ca0` is the input/paint dispatch heart | **VERIFIED + extended** | `decompile_function 0x00610ca0`. See §2.2 — the `DAT_00ac18c0` "paint proc" is actually a full per-class subclass WNDPROC called via `CallWindowProcA`; topmost array `DAT_00ac1de8` implements modal input exclusion; +0x1FC (data) slide state machine 1→2→3 runs inline and calls `FUN_00608260`. |
| 5 | Per-control 0x208 record, initializer `0x00623340` (zero, kind 0xB, font g_GAME_FNT) | **VERIFIED** (offsets corrected, §3) | `decompile_function 0x00623340`: zeroes 0x200 bytes at data root, data+0x68=0xB, data+0x64=g_GAME_FNT, data+0x3C=new wstring obj, data+0x90=-1. |
| 6 | WM_PAINT composer `0x00621E90` mode-1/mode-2 + flip | **VERIFIED + extended** | `decompile_function 0x00621e90`. Lazy 16-bpp offscreen BSurface at data+0x10 (w*h*2, vtable__XSurface→vtable__BSurface). data+0xB0==1: `RightPanel__Draw(data+0xD4=='\0')` → `Background_Overlay(data+0x74, data+0xE0, data+0xE4)` → overlay markers data+0xD5→`Sidebar_TopHighlight`, data+0xD6→`Minimap_Button`, data+0xDB→`RadarBackground`; in-game (`FUN_0069bbe0`!=0) uses `LeftPanel__Draw(0)` instead. data+0xB0==2: captures AlternateSurface region then `CC_Draw_Shape` of per-side modal bg SHP (side from `g_ScenarioClass_Instance+0x34b8`: 0/1/else → `DAT_00b0fc84/88/8c`; non-game → `FUN_0072b030`). **NEW:** any other mode (0) draws `dbak6440.pcx` centered (`PTR_s_dbak6440_pcx_00833654`). Ends with single full-rect blit offscreen→`DAT_00887310` (AlternateSurface vtbl+8). |
| 7 | Teardown `0x00622720`: slide-out, DestroyWindow, LIFO compact, focus restore to previous dialog else g_hWnd | **VERIFIED** | `decompile_function 0x00622720`. Order: `FUN_0054f720` → slide-out `FUN_00608070` → `DestroyWindow` → linear scan of `DAT_00b72d28`, memmove-compact (`FUN_007ca090`), depth--, top mirror reset, `SetForegroundWindow`+`SetFocus` to new top, or `g_hWnd` when empty. |
| 8 | Modal pump `0x00623120` is the loop **body**; keeps sim+net advancing | **VERIFIED (with precision)** | `decompile_function 0x00623120`. `Process_NetworkMessages()` always; then **front-end branch** (`g_GameMode==0||5 || DAT_00a8d60e || DAT_00a8dab4`) → `Network_ServiceLoop()` only; **else (in-game)** → `FUN_0055cbf0()` gate then `Main_Tick()`, returns 1 if Main_Tick returns nonzero. So sim advances behind modals **only in-game**; in the front-end the pump services network/UI only. The loop lives in each owner (confirmed in 3 owners: `0x00558dd0`, `0x00531cc0`, Main_Game case 8). |
| 9 | Reposition: `0x0060c4a0` = pass, `0x0060c540` = include-test + slide-marker setter; include-set = {0xE2,0x6B,0x100,0x102} | **VERIFIED for roles; DRIFT on the id set** | `decompile_function 0x0060c4a0`: `MoveWindow(0,0,g_ScreenWidth,g_ScreenHeight)` + `EnumChildWindows(ResizeShellChildControl_0060C0C0)`. `decompile_function 0x0060c540`: reads dialog id at bucket+0x70 (= data+0x6C) and tests a **55-id include-set** (full list §5.1), not 4 ids; on match sets data+0xB0=1 (paint-mode 1) and data+0xBD=1 (slide-IN gate) and returns 1. The prior doc's "include-set = 0xE2/0x6B/0x100/0x102" is **WRONG**; those 4 are merely the offline-reachable members. Per-child re-anchor allow-listing is done inside `ResizeShellChildControl` via predicate `0x00608cd0` (caller verified via `get_function_callers 0x00608cd0`). |
| 10 | Slide-in `0x006071E0`: SHP-frame sweep, 30 ms/tick, bound = max stagger + 6, eligibility by dialog-id allow-list | **VERIFIED** | `decompile_function 0x006071e0`: `Sleep(0x1e)` per tick; schedule[i]=i+1 (1-tick stagger) + 3 special slots; loop bound = max(schedule)+6; frames via `CC_Draw_Shape` driven by `tick - schedule[i]`, direction ±1 (in vs out); per-tick blit to `DAT_00887310` then `DAT_00887308`; end: slide-in → msg `0x4ed` to parent; slide-out → `VocClass__PlayAtPos` + Bink drain + msg `0x4ec`. Triggers: `0x00608260` (slide-IN; gate = data+0xBD && data+0xB0==1 && IsWindowVisible; plays Voc at start, disables window, `EnumChildWindows(LAB_00606800)`) and `0x00608070` (slide-OUT; same gate; sets data+0xBE=1, InvalidateRect, then **pumps inline** (same body as `0x00623120`) until WM_PAINT runs the slide and clears data+0xBE, 5000 ms timeout) — both `decompile_function` this session. Eligibility = the 55-id set of `0x0060c540` **or** explicit `FUN_00608380(hwnd)` (sets data+0xBD=1; single caller = save-game runner `0x00558dd0`, `get_function_callers 0x00608380`). Mode-2 modals can never slide (gate requires paint-mode 1). |
| 11 | `Main_Game 0x0052D9A0` = return-code navigation state machine | **VERIFIED** | `decompile_function 0x0052d9a0`. State codes seen live: 0x12 → main-menu runner `FUN_00531cc0`; 1/4 → `FUN_0060d380(1)` (SP / movies family); 2 → g_GameMode=4 (WOL); 3 → g_GameMode=3 (LAN/IPX); 5 → `OptionsClass__ShowLauncherDialog` → 0x12; 6 → quit-confirm via `FUN_005d3490` (CSF Init.CPP 0x9c6/0x9c7/0x9c8) → on OK `OptionsClass__WriteToINI` + code 7; 7 → outro movie pump (≤3000 ticks) → return 0 (graceful exit); 8 → campaign chooser (factory + `SetWindowLongA(hDlg,8,&result)` + CenterChildWindow + pump + teardown; difficulty map `DAT_00a8eb64` → scenario+0x610/+0x60c; scenario index → scenario+0x34cc); 9 → load-game flow (`FUN_005587f0`); 0xb → g_GameMode=5 → skirmish runner `FUN_006ae2c0` (cancel → code 1); 0x10/0x11 → per-mode network/WOL launch (modem cases 1/2 need serial hw); 0xd/0xe/0xf → credits/movie playback. Then `ScenarioClass__Start_Scenario`. |
| 12 | Result channel: dialog writes int through "GWLP_USER" pointer; owner loop spins on sentinel | **VERIFIED (naming corrected)** | The index used is **8 = DWL_USER**, not GWLP_USERDATA(-21): `SetWindowLongA(hDlg, 8, &result)` seen live in `0x00531cc0` (sentinel 0x12), `0x00558dd0` (sentinel -1), Main_Game case 8 (sentinel -1). GWL_USERDATA(-0x15) is a *different* channel: per-control override block read in `0x0060f9a0` and freed at WM_NCDESTROY in `0x00610ca0`. Prior doc's "GWLP_USER" wording should be read as DWL_USER(8). |
| 13 | `0x00623340` record init constants | **VERIFIED** | see #5. |
| 14 | Keyboard-routing array `DAT_00abfc94` / count `DAT_00abfca0`, append `0x005d4e70`, prune `0x005d4ed0`, scanned by `Process_NetworkMessages` for IsDialogMessageA in registration order | **VERIFIED + extended** | `decompile_function 0x005d4e70 / 0x005d4ed0 / 0x005d4d50`. Storage is a wwlib growable vector (object `DAT_00abfc90`, cap `DAT_00abfc98`, grow-step `DAT_00abfca4`, resizable flag `DAT_00abfc9d`). **NEW:** after the IsDialogMessageA scan there is a second registry — accelerator pairs `{HACCEL,HWND}` at `DAT_00abfcbc`, count `DAT_00abfcc8`, walked with `TranslateAcceleratorA`; then optional hook `DAT_00abfd34`; then Translate/Dispatch. |
| 15 | Init bridge `0x00622820` subclasses children + sets slide-group markers by dialog id | **VERIFIED + id sets dumped** | `decompile_function 0x00622820`. Body = same subclass passes as DLGPROC's lParam≠0 path, then writes dialog id (its param_2) to data+0x6C, then marker bytes: **data+0xD5**=1 for id ∈ {0xBC,0xBD,0x102,0xC2,0xC9,0xBC6,0x105,0x6B,0x113}; **data+0xD6**=1 for {0xBC,0xBD,0x102,0xC2,0xC9,0xBC6}; **data+0xD7**=1 for {0x103,0xBC7}; **data+0xD8**=1 for {0x108,0xBC6}. If include-test fails: **data+0xB0=2** for id ∈ mode-2 modal set (19 ids, §5.2); if include-test passes: MoveWindow fullscreen + `ResizeShellChildControl` enum. Ends with `EnumChildWindows(FUN_0060a330)` + `FUN_0060a5b0` + `FUN_00777060`. |

DRIFT-corrections #1–#4 of the prior doc (pump=body, 0x0060c540=test-not-pass, two +0xC1-setters,
0x120/0xCE not in include-set) all re-confirmed live this session.

---

## 2. Extended mechanics (new this session)

### 2.1 Two WM_INITDIALOG paths in `0x00622b50` (verified via decompile_function 0x00622b50)
- `lParam != 0` (factory-created): full subclass passes (`FUN_0060f760(1)` → `LAB_0060f320` →
  `FUN_0060f9a0` → `FUN_0060f760(0)`), `FUN_0060d2c0`, background select `FUN_0060cf00` (skipped
  if `FUN_0069bbe0()` != 0, i.e. in-game), overlay setters `FUN_0060caf0/c930/ccc0/cdb0`,
  `LAB_0060aab0`, include-test → `FUN_0060c4a0` (reposition) else `FUN_0060c7d0` (role unverified,
  presumed centering), then `FUN_0060a330` + `FUN_0060a5b0` + `FUN_00777060` + `SetFocus(dialog)`.
  Note: this path does **not** write the dialog id into the record; the id is written by the init
  bridge `0x00622820` (called from per-dialog procs) — a dialog whose proc never calls the bridge
  keeps id=0 and falls into the centered/mode-default path.
- `lParam == 0` (re-init): skips subclassing, sets record id=0, reruns background/overlay/include
  logic.

### 2.2 Shared wndproc `0x00610ca0` internals (verified via decompile_function 0x00610ca0)
- `DAT_00ac18c0` maps HWND → **class-specific subclass WNDPROC** (e.g. `OwnerDraw_Button_00612B70`),
  invoked via `CallWindowProcA`; `DAT_00ac1b48` maps HWND → original Win32 wndproc. "Paint proc"
  in the prior doc undersells it: these procs also own input + timers for their control class.
- **Modal input exclusion:** topmost array `DAT_00ac1de8` (count `DAT_00ac1de0`, cap `DAT_00ac1de4`).
  Msg `0x4a9` pushes a window to top (`SetWindowPos`, guard byte `DAT_00ac48e8`); WM_DESTROY removes.
  While the array is non-empty, messages to windows that are not the top entry (or its ancestors)
  are swallowed: mouse `0x200..0x209` blocked, key ranges `0xA0..0xA9`/`0x100..0x108` blocked,
  `0x113` (WM_TIMER), `0x49b`, `0x4ad` blocked; `0x104/0x105/0x106/0x112` pass. This is the
  modal-exclusivity mechanism for stacked dialogs.
- **Reentrancy guard:** hashtable `DAT_00ac1858` keyed (msg,hwnd) drops re-entrant messages except
  `0x111/0x104/0x105/0x112/0x106`.
- **Custom message map** (record = data root): `0x49a` swap data+0x24; `0x49c` swap data+0x14;
  `0x4aa` swap data+0x18; `0x49d` swap data+0x20 (and mirror into the record of the window at
  data+0x0C, field +0x24); `0x49e/0x49f` restore/save GDI font+bk+text colors into data+0x1E8..0x1F4;
  `0x4a0` query data+0x0C; `0x4b2` set owned wide text at data+0x28 (+dirty flag data+0x2C=0;
  kills 1s timer / sends `0x4ee` for kind-1 edits); `0x4b3` get text; `0x4b4` set-from-ANSI
  (dirty=1); `0x4b5` query length; `0x4ce` query !dirty; `0x4d1` swap data+0x30; `0x4eb` set data+0x50.
- WM_SETFOCUS(7): for Button/ListBox classes refocuses; sets focus flag data+0x38=1 (+`FUN_00777e00`);
  WM_KILLFOCUS(8) clears. WM_NCDESTROY(0x82): frees offscreen surface (data+0x10), removes record
  (`FUN_00624ca0`/`FUN_006233a0`), frees + zeroes the GWL_USERDATA block.
- **First-paint slide state machine** at data+0x1FC: after a parent paint completes,
  1 → set 2 → call `FUN_00608260` (slide-in) → if it ran, set 3. Marked state 2 also forced when
  paint-depth `DAT_00ac48dc` > 1.
- Paint bookkeeping: paint-depth `DAT_00ac48dc`; dirty-union rect `DAT_0083367c/DAT_00833680/
  DAT_00ac48e0/DAT_00ac48e4`; final front-blit `DAT_00887310` (Alternate) → `DAT_00887308` when
  depth returns to 0; ComboBox class forces palette refresh (`FUN_0072aa10(-1)` twice).

### 2.3 Owner-draw Button `0x00612B70` (verified via decompile_function 0x00612b70)
- Record fields: data+0xE8 bit0 = pressed/checked; data+0xBC = paint-suppress; data+0xC4 =
  hover-active (set/cleared by msg `0x4dc` wParam-style lParam 1/0, arms `SetTimer(id 0, 1000 ms)`);
  data+0xC5 = flash phase, toggled每 WM_TIMER (0x113) → 1 Hz flash; data+0x10 = lazy per-control
  BSurface (background captured from AlternateSurface at creation); data+0x14/data+0x18 = custom
  image handles (normal/pressed) for asset-0 path; data+0x64 = font; data+0xB0 = paint-asset code.
- Click sound: WM_LBUTTONDOWN/DBLCLK (`0x201/0x203`) → `VocClass__PlayAtPos(1.0f, …)` unless
  suppressed (data+0xBC). Voc index passed in register (not recovered this session).
- Paint-asset fork (data+0xB0): **0** = 3-piece PCX `b%c%c_li%d/mi%d/ri%d.pcx` with state char
  'u'/'d' and the second char **hardcoded 'e'** (enabled), size selector %d ∈ {0x18,0x1e} (24/30 px)
  by control height; disabled rendering = `AlphaBlendRect(0,0x80)` darkening (no `bud_*` art) —
  prior doc's REFUTED/DEAD verdicts for `bue_*30`-on-0xE2 and `bud_*` family stand. Also plays a
  Voc on the u→d edge (`DAT_00833684` tracks last state). **1** = `g_SDBTNANM_SHP`
  (convert `FUN_0072e2c0`): frame 2 idle / 3 when flash-phase set / 4 pressed. **2** =
  `DAT_00b0f9ec` SHP (convert `FUN_0072f4b0`): frames 0/2/1. **3** = `DAT_00b0facc` SHP (convert
  `FUN_0072b050`): frames 0/2/1 (modal OK family). Disabled text color: per-side RGB565 constants
  (`DAT_00b0fa95` shell, `DAT_00b0fb15`/`DAT_00b0fb1a` per `scenario+0x34b8`).
- Text via `FUN_00621040` (ShellText__DrawInRect), default color `DAT_00ac18a4` (=0xFFFF → yellow),
  pressed text offset present (exact +x/+y deltas not recovered from decomp this session — prior
  doc says +2y/+1x, NOT re-verified).

### 2.4 Paint-asset / re-anchor / slide-participation predicates
(callers verified via `get_function_callers 0x00608cd0` → `FUN_0060a180`, `FUN_0060a330`,
`ResizeShellChildControl_0060C0C0`; `get_function_callers 0x00609e20` → `FUN_0060a330`)
- `FUN_0060a330` (run at end of every init, `decompile_function 0x0060a330`): for owner-draw
  buttons (style&0xB==0xB, record kind==0): predicate `0x00608cd0(dlgId, ctlId)` or `FUN_00609730`
  → data+0xB0 = 1 (front-end) or 2 (in-game, `FUN_0069bbe0`!=0); predicate `0x00609e20` →
  data+0xB0 = 3. So a control's paint-asset and the parent's paint-mode share field data+0xB0.
- `0x00608cd0` = (dialog id, control id) → bool; also gates which children re-anchor
  (ResizeShellChildControl) and which slide (`FUN_0060a180` from the slide loop). Structure:
  ctl `0x694` (title) allowed for ~47 ids; ctl `0x71c` for ~31 ids; ctl `0x468` for
  {0xBC,0xBD,0x102,0xC2,0xC9,0xBC6,0x105,0x6B,0x113}; then per-dialog button lists (0xE2:
  0x686,0x578,0x55C,0x683,0x55F,0x684; 0x100: 0x689,0x688,0x579; 0x101: 0x68E,0x68D,0x68F;
  0x102: 0x6EC,0x5AA,0x5A8,0x617; 0x94: 0x40E; 0xB7: 0x40F; 0x2B4: 0x6C7; 0x2B5: 0x6C8; 0x105:
  0x6C4,0x6C2,0x6C3,0x6C5; 0xB5: 0x522,0x521,0x51E,0x51F,0x520; 0xBBA: 0x522,0x521; 0xBBB:
  0x52C,0x52D; 0xD5: 0x5CE,0x5CD; 0xFB: 6,1; 0xD8: 0x6CB; 0xD7: 1; 0xBC/0xC2: 0x6EC,0x588,0x5AA,
  0x5A8,0x5C2; 0xBD/0xC9: 0x59F,0x6EC,0x5A8; 0xBB: 0x6CD,0x6CC,0x6CE; 0xB6: 0x6C9,0x712,0x524;
  0x6B: 0x583,0x6C5; 0xBC6: 0x6D1,0x6EC,0x5A8; 0x10F/0x11D: 0x539,0x6EC,0x6EB; 0x109: 1,0x625,
  0x53B; 0x10E/0x11C: 0x6E0,0x6E1,0x6E2,0x6E4,0x771,0x55F; 0x114: 0x687,0x6EC,0x6EB,0x688,0x7A3;
  0x113: 0x61F,0x688,0x7A3,0x7A6,0x689,0x62D,0x62B,0x53D; 0x2BC: 1,0x63A; 0x116: 0x62B,0x62D,
  0x5C2,0x702,0x703; 0xFF: 0x521,0x522; 0x122/0x112/0xFE/0xE6/0xF3/0xF4/0xE7(+0x576): 1; several
  branches additionally accept 0x5C2). (`decompile_function 0x00608cd0`)
- `0x00609e20` = modal-OK predicate (paint-asset 3): 0x10D:{0x5C1,0x5C0}; 0xD9:{0x5C1,0x6CA};
  0xA3:{0x4CF,0x4D0}; 0x105:{0x620,0x621}; 0xF0:{0x5C0}; 0xCE:{0x5AE}; 0x120:{0x5AE,2};
  0x121:{0x5AE,0x5AF,2}; 0xEA:{0x640..0x647}; 0xB8:{0x531,0x535}; 0x11F:{0x5C0};
  0x115:{0x702,0x686}; 0xD3:{2}; 0xCF:{0x5C2,2}; 0xC3:{1,2}; 0x11B/0x11E:{0x5C2,0x686};
  0xE1:{1,2}; 0xC4:{0x58D,0x58E}; 0xD0:{1}; 0xFC:{1,2}; 0x126:{6,7}. (`decompile_function 0x00609e20`)

### 2.5 Lifecycle exemplar — Load/Save/Delete runner `0x00558dd0`
(`decompile_function 0x00558dd0`) **Label drift:** Ghidra names it `CDFileClass__Constructor`;
the body is the Load(mode1, dlg 0xB7, listbox 0x525, button 0x40F)/Save(mode2, dlg 0x2B4, listbox
0x527, edit 0x526, button 0x6C7)/Delete(mode3, dlg 0x2B5, listbox 0x528, button 0x6C8) dialog
runner (strings `D:\ra2mdpost\LoadDlg.CPP`). Full contract live: factory → `GetDlgItem` wiring →
`SetWindowLongA(hDlg, 8, ctx)` → `FUN_00622800` show → `while (ctx.result == -1) FUN_00623120()`
(plus `FUN_00532100()` background anim when `g_GameActive=='\0'`) → on save success
`FUN_006083e0()` + saved-modal `FUN_005d3490` + `FUN_00608380(hDlg)` (re-arm slide) →
`FUN_00622720` teardown. Save filename pattern `SAVE_%04lX.%3s` with `_rand()`.

### 2.6 Main-menu runner `0x00531cc0`
(`decompile_function 0x00531cc0`) Factory (template in ECX) → `SetWindowLongA(hDlg, 8, &code)`,
sentinel **0x12** → `CenterChildWindow` → show → pump loop. Control `0x71a` = the animated
background: positioned at centered 800×600 origin, msg `0x4e3`(1) enables movie mode, msg `0x4e4`
passes Bink name `"Ra2ts_l"` (or `"Ra2ts_s"` at 640 width). Cheat-string parser (PENGO table at
`PTR_s_PENGO_00825c2c`, flags at `PTR_DAT_00825c28`). Teardown + `FUN_00661850(systime.ms)` RNG
touch on exit. Result codes themselves are written by the dialog proc (per prior doc `0x00531F60`,
DOC-INHERITED).

---

## 3. Corrected 0x208 record map — ALL offsets data-root-relative (data = bucket+4)

Bucket layout: `bucket+0x00` = HWND key, `bucket+0x04..0x203` = 0x200-byte data, `bucket+0x204` =
hash-chain next. Helpers (`FUN_00624760` et al.) return the data root.

| data off | bucket off | Field (live evidence) |
|---|---|---|
| +0x00 | +0x04 | init lParam from subclass pass (`0x0060f9a0`) |
| +0x0C | +0x10 | linked HWND for paint-mirror (msg `0x49d` path, `0x00610ca0`) |
| +0x10 | +0x14 | lazy offscreen BSurface (composer `0x00621e90`, button `0x00612b70`, freed at WM_NCDESTROY) |
| +0x14 / +0x18 | +0x18/+0x1C | custom image handles normal/pressed (msgs `0x49c`/`0x4aa`; asset-0 button) |
| +0x20 | +0x24 | paint-mirror state (msg `0x49d`) |
| +0x24 | +0x28 | msg `0x49a` slot |
| +0x28 / +0x2C | +0x2C/+0x30 | owned wide text / text-dirty flag (msgs `0x4b2..0x4b5`, `0x4ce`) |
| +0x30 | +0x34 | msg `0x4d1` slot |
| +0x38 | +0x3C | focus flag (WM_SETFOCUS/KILLFOCUS) |
| +0x3C | +0x40 | wstring obj from initializer `0x00623340` |
| +0x50 | +0x54 | msg `0x4eb` slot |
| +0x54 | +0x58 | GWL_USERDATA[1] copy (`0x0060f9a0`) |
| +0x64 | +0x68 | font (g_GAME_FNT default) |
| +0x68 | +0x6C | control-kind code 0..0xB |
| +0x6C | +0x70 | dialog resource id (written by bridge `0x00622820`; read by `0x0060c540`, `0x00608cd0`, `0x00609e20`, `0x0060cf00`, `0x006040b0` as bucket idx 0x1c) — resolves the prior doc's +0x70-vs-+0x6C hedge: ONE field, data+0x6C |
| +0x74 | +0x78 | background convert (`0x0060cf00`) |
| +0x90 | +0x94 | initialized -1 (`0x00623340`) |
| +0xB0 | +0xB4 | **dual-role:** parent paint-mode (1 shell / 2 modal SHP / other → dbak6440.pcx fallback) AND button paint-asset (0 PCX / 1 SDBTNANM / 2 / 3 modal-OK). Writers: `0x0060c540` (=1), `0x00622820` (=2), `FUN_0060a330` (=1/2/3) |
| +0xBC | +0xC0 | paint-suppress flag (DLGPROC WM_PAINT + button proc) |
| +0xBD | +0xC1 | slide-IN gate (writers `0x0060c540`, `0x00608380`; readers `0x00608260`, `0x00608070`) |
| +0xBE | +0xC2 | deferred-slide pending byte (set by `0x00608070`; consumed+cleared by DLGPROC WM_PAINT) |
| +0xC4 | +0xC8 | hover-active (msg `0x4dc`, arms 1000 ms timer) |
| +0xC5 | +0xC9 | hover flash phase (toggled per WM_TIMER → 1 Hz; button frame 3) |
| +0xD4 | +0xD8 | right-panel draw variant flag (composer) |
| +0xD5..+0xD8 | +0xD9..+0xDC | chrome-overlay markers by dialog id (bridge `0x00622820`): +0xD5 Sidebar_TopHighlight, +0xD6 Minimap_Button, +0xD7 {0x103,0xBC7}, +0xD8 {0x108,0xBC6} |
| +0xDB | +0xDF | RadarBackground overlay marker (read by composer + slide loop; writer = one of `FUN_0060caf0/c930/ccc0/cdb0`, not individually traced) |
| +0xE0 / +0xE4 | +0xE4/+0xE8 | background small / large SHP (`0x0060cf00`, composer) |
| +0xE8 | +0xEC | state bits; bit0 = pressed/checked (button proc) |
| +0x1E8..+0x1F4 | — | saved GDI font/bkmode/bkcolor/textcolor (msgs `0x49f`/`0x49e`) |
| +0x1FC | +0x200 | first-paint slide state machine 1→2→3 (`0x00610ca0`) |

**The prior doc's §2.4 rows +0xB4 ("slide-eligible"), +0xC1, +0xC2, +0xC5, +0x14, +0x70 were
bucket-relative; its rows +0xB0, +0x74/+0xE0/+0xE4, +0x68, +0x28/+0x2C, +0x38, +0xD5.., +0x1FC
were data-root-relative.** In data-root terms there is no separate "+0xB4 slide-eligible" field —
that write IS paint-mode=1.

---

## 4. Owner-draw control census (classifier-dispatch verified live; per-proc internals as noted)

| Win32 class (cascade order) | kind | subclass WNDPROC | internals evidence |
|---|---|---|---|
| ScrollBar | 8 | `OwnerDraw_ScrollBar_0061C690` | dispatch VERIFIED-LIVE (`0x0060f9a0`); internals DOC-INHERITED (skirmish-ui scroll docs) |
| ListBox | 4 | `OwnerDraw_ListBox_00618D40` | dispatch VERIFIED-LIVE; internals DOC-INHERITED (choose-map/listbox docs) |
| ComboBox | 3 | `OwnerDraw_ComboBox_00617250` | dispatch VERIFIED-LIVE; geometry/paint DOC-INHERITED (`SKIRMISH_COMBO_OWNERDRAW_GEOMETRY`: 24 px face, 20 px arrow) |
| msctls_trackbar32 | 7 | `OwnerDraw_Trackbar_0061D950` | dispatch VERIFIED-LIVE; internals DOC-INHERITED (skirmish trackbar docs) |
| msctls_progress32 | 6 | `LAB_0061D6D0` | dispatch VERIFIED-LIVE; internals UNCHECKED |
| NewEdit | 1 | `OwnerDraw_NewEdit_00614B30` | dispatch VERIFIED-LIVE; DOC-INHERITED (player-name edit doc: 0x6A0 uses 00614190) |
| Edit | 1 | `OwnerDraw_Edit_00614190` | dispatch VERIFIED-LIVE |
| Static | 2 | `OwnerDraw_Static_006153E0` | dispatch VERIFIED-LIVE; full paint DOC-INHERITED (`OWNERDRAW_STATIC_006153E0_FULL_PAINT`); kind-2 movie statics confirmed in use live (ctrl 0x71A Bink, `0x00531cc0`) |
| SysTabControl32 | 0xA | `LAB_006137D0` | dispatch VERIFIED-LIVE; internals UNCHECKED |
| Button (s&7)==7 | 0 | `OwnerDraw_ButtonVariant_0061E700` | dispatch VERIFIED-LIVE; internals UNCHECKED |
| Button (s&0xB)==0xB | 0 | `OwnerDraw_Button_00612B70` | **fully VERIFIED-LIVE** (§2.3) |
| Button (s&3)==3 | 0 | `OwnerDraw_Checkbox_006163A0` | dispatch VERIFIED-LIVE; internals DOC-INHERITED (skirmish checkbox docs) |
| Button (s&9)==9 | 0 | `OwnerDraw_RadioVariant_00616980` | dispatch VERIFIED-LIVE |
| msctls_hotkey32 | 9 | `LAB_0061ECA0` | dispatch VERIFIED-LIVE; internals UNCHECKED |
| (no match) | 0xB | `LAB_00612A60` | dispatch VERIFIED-LIVE |

Asset families confirmed live: SDBTNANM.SHP (asset-1 buttons, frames 2/3/4), 3-piece PCX
`b{u,d}e_{li,mi,ri}{24,30}.pcx` (asset-0), `DAT_00b0f9ec` SHP (asset-2, frames 0/2/1),
`DAT_00b0facc` SHP (asset-3 modal OK, frames 0/2/1 — prior docs identify this family as
MNBTTN, DOC-INHERITED), `dbak6440.pcx` (mode-0 parent fallback), per-side modal bg SHPs
`DAT_00b0fc80/84/88/8c` (mode-2), MNSCRN small/large via `0x0060cf00` default row
(`FUN_0072e280` + `DAT_00b0fb50`/`DAT_00b0fa04`).

---

## 5. Static id tables (dumped live)

### 5.1 Include-set of `0x0060c540` — 55 dialog ids (fullscreen-expand + reposition + slide gate + paint-mode 1)
`0x6B, 0x73, 0x94, 0xA3, 0xB5, 0xB6, 0xB7, 0xB8, 0xBB, 0xBC, 0xBD, 0xC2, 0xC9, 0xD4, 0xD5,
0xD6, 0xD7, 0xD8, 0xE2, 0xE6, 0xE7, 0xEA, 0xF3, 0xF4, 0xF5, 0xFB, 0xFE, 0xFF, 0x100, 0x101,
0x102, 0x103, 0x105, 0x108, 0x10B, 0x10C, 0x10E, 0x10F, 0x112, 0x113, 0x114, 0x116, 0x117,
0x11C, 0x11D, 0x122, 0x125, 0x129, 0x2B4, 0x2B5, 0x2BC, 0xBBA, 0xBBB, 0xBC6, 0xBC7`
(verified via `decompile_function 0x0060c540`)

### 5.2 Mode-2 modal set of `0x00622820` — 19 dialog ids (paint-mode 2, SHP modal background, centered, never slide)
`0xC3, 0xC4, 0xCE, 0xCF, 0xD0, 0xD3, 0xD9, 0xE1, 0xF0, 0xFC, 0x10D, 0x115, 0x11B, 0x11E,
0x11F, 0x120, 0x121, 0x126, 0x130` (verified via `decompile_function 0x00622820`)
Disjoint from §5.1 — confirms prior DRIFT-correction #4.

### 5.3 Background table `0x0060cf00` — id → (convert, small SHP, large SHP) into data+0x74/+0xE0/+0xE4
0x94→(FUN_0072dae0, DAT_00b0fa6c ×2); 0x103,0xBC7,0xBC6→(FUN_0072d450, g_RadarFrameOpen_SHP ×2);
0x108→(FUN_0072d820, g_MinimapMovie_SHP ×2); 0x6B→(FUN_0072d210, DAT_00b0fb50, DAT_00b0fab8);
{0x102,0xBC,0xBD,0xC2,0xC9}→(FUN_0072d030, DAT_00b0fb50, DAT_00b0fa18);
0x113→(FUN_0072ce50, …, DAT_00b0faac); 0x114→(FUN_0072cab0, …, DAT_00b0fa60);
{0x10E,0x11C}→(FUN_0072c8d0, …, DAT_00b0f9e8); {0x10F,0x11D}→(FUN_0072c6f0, …, DAT_00b0fb0c);
{0xE6,0xF3,0xF4}→(FUN_0072c510, …, DAT_00b0fa64); 0xE7→(FUN_0072c330, …, DAT_00b0fa30);
0x116→(FUN_0072c150, …, DAT_00b0fa4c); 0x117→(FUN_0072bf70, …, DAT_00b0fa9c);
0x112→(FUN_0072bd90, …, DAT_00b0fa98); 0x2BC→(FUN_0072bbb0, …, DAT_00b0fb2c);
0xD6→(FUN_0072b9d0, …, DAT_00b0fb54); **default**→(FUN_0072e280, DAT_00b0fb50, DAT_00b0fa04)
(= MNSCRN family; covers 0xE2/0x100/0x101 and all other shell ids).
(verified via `decompile_function 0x0060cf00`)

### 5.4 Tooltip map `0x006040b0`
Signature `(parentHwnd → record id, GetDlgCtrlID(child)) → char* CSF key` (e.g.
`STT_MainButtonSinglePlayer`). Covers **50 dialog ids** / **381 STT_ strings** total. Dialog ids
covered: 0x6B,0x73,0x94,0xA3,0xB5,0xB6,0xB7,0xB8,0xBB,0xBC,0xBD,0xC2,0xC3,0xC9,0xD5,0xD7,0xD9,
0xE2,0xE6,0xE7,0xEA,0xF3,0xF4,0xFE,0xFF,0x100,0x101,0x102,0x103,0x105,0x108,0x10C,0x10E,0x10F,
0x112,0x114,0x116,0x117,0x11C,0x11D,0x122,0x125,0x129,0x2B4,0x2B5,0x2BC,0xBBA,0xBBB,0xBC6,0xBC7.
Consumed from DLGPROC `0x84` and wndproc WM_MOUSEMOVE paths into static `0x695` via msg `0x4b2`;
miss → empty; CSF resolve failure → `ownrdraw.cpp` string `0x7a5`.
(verified via `decompile_function 0x006040b0`, constants extracted from saved dump)

### 5.5 Slide-group / chrome-overlay marker id sets — see Delta #15.

---

## 6. Singleton state inventory (writers verified live)

| Global | Role | Writers (live) |
|---|---|---|
| `DAT_00b72d28`/`DAT_00b72d2c` (stride 8), `DAT_00b72f50` depth, `DAT_00b72f44`/`DAT_00b72f48` top mirror | dialog LIFO display/focus stack | factory `0x00622650` push; teardown `0x00622720` compact |
| `DAT_00abfc94` HWND[], `DAT_00abfca0` count (vector obj `DAT_00abfc90`, cap `DAT_00abfc98`, grow `DAT_00abfca4`, flag `DAT_00abfc9d`) | keyboard-routing array (IsDialogMessageA, registration order) | `0x005d4e70` append, `0x005d4ed0` remove-compact |
| `DAT_00abfcbc` {HACCEL,HWND}[], `DAT_00abfcc8` count; hook `DAT_00abfd34` | accelerator registry + message hook in pump | scanned by `0x005d4d50` (writers not traced this session) |
| `DAT_00ac18c0`/count `DAT_00ac18c4` (+hash fn `DAT_00ac18d8`, bits `DAT_00ac18cc`, load-factor rehash `FUN_00624be0`) | HWND → class subclass WNDPROC | `0x0060f9a0` insert; reader `0x00610ca0` |
| `DAT_00ac1b48`/count `DAT_00ac1b4c` | HWND → original wndproc | `0x0060f9a0` insert |
| `DAT_00ac1b00`/count `DAT_00ac1b04` (hash `DAT_00ac1b18`, bits `DAT_00ac1b0c`, rehash `FUN_00624fc0`) | HWND → 0x208 record (bucket IS the record; chain bucket+0x204) | `0x0060f9a0` insert; `0x00610ca0` WM_NCDESTROY remove |
| `DAT_00ac1de8`/count `DAT_00ac1de0`/cap `DAT_00ac1de4`; guard `DAT_00ac48e8` | topmost/modal-exclusion window stack | wndproc msg `0x4a9` push, WM_DESTROY remove (`0x00610ca0`) |
| `DAT_00ac1858`/count `DAT_00ac185c` (+`DAT_00ac1860/64/70`) | (msg,hwnd) reentrancy guard table | `0x00610ca0` (lazy init flag bit `DAT_00ac461c&1`) |
| `DAT_00ac48dc` | paint-depth counter | `0x00610ca0` |
| `DAT_00ac48d4` | one-time RGB-shift/PCX preload guard | `0x0060f9a0` (uses `FUN_004bbc30..80` DD shifts, `FUN_0061f210`) |
| `DAT_00ac48b4` | live offscreen-surface count | composer/button/wndproc |
| `DAT_0083367c`,`DAT_00833680`,`DAT_00ac48e0`,`DAT_00ac48e4` | union dirty rect for the end-of-paint front blit | `0x00610ca0` |
| `DAT_00833684` | last button up/down char ('u'/'d') for click-edge sound | `0x00612b70` |
| `DAT_00ac18a4`=0xFFFF yellow, `DAT_00ac1cb4`=0x9F disabled red, `DAT_00ac1cb0`=0xEEEEEE, `DAT_00ac184c`=0xFFFFFF, etc. | shell color singletons (0x00BBGGRR permuted) | re-written on every `0x0060f9a0` call |
| `DAT_00887310` Alternate (composition target), `DAT_00887308` front blit target | surfaces | composer/slide/wndproc |
| `DAT_00a8ed8c` | open-shell-dialog counter | DLGPROC `0x110`++ / `2`-- |
| `DAT_00ac48a8` | current init parent HWND | DLGPROC init path + bridge |

---

## 7. Active vs dormant RT_DIALOG census (offline YR skirmish lens)

**ACTIVE offline (reachable from Main_Game without network/WOL):**
- `0xE2` main menu (state 0x12 → `0x00531cc0`) — VERIFIED-LIVE.
- `0x100` single player (code 1 → `FUN_0060d380(1)`; runner DOC-INHERITED `0x0060D380`) + tooltips live.
- `0x102` skirmish (code 0xB → g_GameMode 5 → `FUN_006ae2c0`) — VERIFIED-LIVE; `0x6B` choose-map (bg row + include-set + predicate live; binding to skirmish DOC-INHERITED).
- `0x94` campaign select (Main_Game case 8 inline runner) — VERIFIED-LIVE.
- `0xB7` load / `0x2B4` save / `0x2B5` delete (`0x00558dd0`) — VERIFIED-LIVE (save/delete reachable in-game; load also via SP menu code 9).
- Options family: `0xB5` (launcher via `OptionsClass__ShowLauncherDialog`, code 5) + sub-dialogs `0xBBA`, `0xBBB`, `0xFF`, `0xF5`, `0xD5` (in include-set + predicate/tooltip rows; individual reachability DOC-INHERITED).
- `0x101` movies & credits (codes 4/0xD/0xE/0xF region) — VERIFIED-LIVE in Main_Game; `0x10C` (include-set + tooltip row; identity DOC-INHERITED).
- Modal family (mode-2 set §5.2): quit-confirm (Main_Game case 6 via `0x005d3490`), validation/info modals `0xCE`/`0x120`/`0x121` — creation path VERIFIED-LIVE, template-id-per-caller still UNCHECKED (prior doc C13 stands).
- In-game shell: wartime options/load/save use the same substrate with `FUN_0069bbe0()!=0` → LeftPanel composition + paint-asset 2 — VERIFIED-LIVE branches.

**LIVE CODE, ONLINE-DEAD (WOL/ladder — servers offline; do not implement as default):**
`0x103, 0x105, 0x108, 0x109, 0x10B(?), 0x10D, 0x10E, 0x10F, 0x112, 0x113, 0x114, 0x115, 0x116,
0x117, 0x11B, 0x11C, 0x11D, 0x11E, 0x11F, 0x122, 0x125, 0x126, 0x129, 0x130, 0x2BC, 0xBC6, 0xBC7,
0xE6, 0xE7, 0xF3, 0xF4, 0xFE, 0xC3, 0xC4, 0xD0, 0xD3, 0xD9, 0xE1, 0xEA(?)` — gated behind
g_GameMode 4 / WOL API (`FUN_0077b2a0`) in Main_Game (VERIFIED-LIVE for the gate; per-id
WOL-membership largely DOC-INHERITED from MAIN_GAME/skirmish-ui docs). `+0xD8` overlay marker
(ids 0x108/0xBC6) and SDBTNANM frame-10 family are in this group.
- LAN/IPX network: `0xBC` host, `0xBD` guest, `0xC2`/`0xC9` WOL-variant lobbies — live code via
  g_GameMode 3 (`FUN_005db680`/`FUN_005dc350`), functional only with IPX networking. Not part of
  the offline parity surface but NOT dead code.
- Modem/serial (g_GameMode 1/2): dead without serial hardware — VERIFIED-LIVE branch exists in
  Main_Game (cases 1/2 of the 0x10 switch).

**REFUTED/DEAD (re-confirmed):** `bud_*/bdd_*` disabled PCX art (format char hardcoded 'e',
disabled = AlphaBlendRect — `decompile_function 0x00612b70`); `0x4DC` hover message has no
shell sender for main-menu buttons (button proc handles it, but senders are network-dialog only —
handler VERIFIED-LIVE, sender absence DOC-INHERITED `HOVER_DISPATCHER`).

---

## 8. Label drift recorded
- `0x00558dd0` labeled `CDFileClass__Constructor` — actually the Load/Save/Delete dialog runner
  (LoadDlg.CPP). Several Main_Game callsites display `CDFileClass__Constructor()` for what are
  CD-check/file helpers at other addresses; treat all `CDFileClass__Constructor` display names as
  navigation-hints only.
- `WM_PAINT_Handler` (0x00621E90), `OwnerDraw_*`, `Process_NetworkMessages`, `Main_Game` labels
  match verified behavior.

## 9. UNVERIFIED (YELLOW) — carried or newly opened
- DLU→pixel constants MulDiv(6,4)/(13,8) and the 1-px finalizer `FUN_0060B950` rows — DOC-INHERITED
  (DLU doc / prior study), not re-read this session.
- Pressed-text sink exact deltas (+2y/+1x) in `0x00612b70` — present but exact values not recovered
  from this decompile; needs disassembly-level read.
- `FUN_0060c7d0` assumed "center non-include dialogs" — role not decompiled this session.
- Per-dialog WM_COMMAND→result-code maps beyond 0xE2/0x100/0x102 (prior open question) — unchanged.
- C13 modal template-id selection (0xCE vs 0x120 vs 0x121 chosen by caller of `0x005d3490`) — still
  UNCHECKED; Main_Game case 6 confirms a 3-string call but the template id passed is in a register.
- Identity of `FUN_0069bbe0`'s receiver object: getter of byte +0x30D8 (content VERIFIED via
  `decompile_function 0x0069bbe0`); receiver presumed ScenarioClass/"game active" from caller
  context (binding MEDIUM — ECX at callsites not traced).
- Voc indices for click/slide sounds (ECX-passed) — names MenuSlideIn/ShellButtonSlideSound are
  DOC-INHERITED (rules ini + prior docs).
- `DAT_00b0facc` = MNBTTN.SHP and `DAT_00b0f9ec` identity — DOC-INHERITED.
- Writers of the accelerator registry `DAT_00abfcbc` — not traced.
- `0x10B`, `0xD4`, `0xFB`, `0x73`, `0xA3`, `0xD6`, `0xD7`, `0xD8` id naming — present in live
  tables; names not confirmed this session.

## Sources (this session, all read-only)
`decompile_function`: 0x00622650, 0x00622b50, 0x00622820, 0x00622720, 0x00623120, 0x0060f9a0,
0x0060c540, 0x0060c4a0, 0x00624760, 0x00608260, 0x00608070, 0x00608380, 0x00608cd0, 0x00609e20,
0x0060a330, 0x00558dd0, 0x00621e90, 0x006071e0, 0x0069bbe0, 0x00623340, 0x0060cf00, 0x006040b0,
0x00610ca0, 0x0052d9a0, 0x005d4e70, 0x005d4ed0, 0x00531cc0, 0x00612b70, 0x005d4d50.
`get_function_callers`: 0x00608cd0, 0x00609e20, 0x00608380.
research-index: `research_search` (dialog-id naming cross-refs).
