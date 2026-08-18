---
title: Shell Dialog Framework as an Engine Substrate Service
date: 2026-05-31
status: synthesis (verified-from-binary this session)
source: ghidra + research-index + live src read
scope: gamemd.exe Win32-native shell dialog substrate (front-end menus + in-game modals), NOT the in-game GadgetClass tree
---

# Shell Dialog Framework — Substrate Service Study

**Rule for the whole document: Rust-native *structure*, gamemd-native *semantics*.**
We do not port Win32 `HWND`/`SetWindowLongA`/`CreateDialogIndirectParamA`/subclassing.
We reproduce the *observable* behavior contract those mechanisms produce, using a
Rust-native dialog/control descriptor service.

This study was produced by a 7-agent decode workflow; every address below was
re-verified in Ghidra this session (read-only) with the cited MCP call inline.
Where a prior doc was wrong, the correction is marked **DRIFT-CORRECTED**.

---

## 0. Two frameworks — fix the scope first

gamemd.exe has **two parallel UI frameworks**. This study is about the second one.

| | Framework A — `GadgetClass`/`LinkClass` | Framework B — **Win32 shell dialogs** (this doc) |
|---|---|---|
| Used by | In-game sidebar, radar, tabs, command bar, cameos | Main menu, single-player, skirmish setup, options, load/save, quit/validation modals, movies, WOL |
| Model | Retained-mode widget tree, doubly-linked list, "smaller-area-wins" hit-test | Native modeless `HWND` dialogs from RT_DIALOG templates, owner-draw subclassed, hand-pumped |
| Authority doc | `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md` | this doc + `skirmish-ui/*`, `MAIN_MENU_*`, `SHELL_*` |
| Source file | wwlib gadget code | `D:\ra2mdpost\ownrdraw.cpp` (string at `FUN_0060f9a0`) |

They never share dispatch. A faithful Rust port keeps them as two separate
services. The "shell dialog framework" the request targets is **Framework B**.

---

## 1. Verified active-YR responsibilities

The shell dialog substrate is the engine's **front-end and modal UI service** —
everything the player sees when *not* in a live tactical scenario, plus the
in-game options/validation modals. Its verified responsibilities:

1. **Create** a dialog from a PE `RT_DIALOG` template (`FUN_00622650`, modeless
   `CreateDialogIndirectParamA` under the game window — never `DialogBoxParam`).
2. **Register** it into two independent registries: a LIFO display/focus stack
   and a flat keyboard-routing array (for `IsDialogMessageA`).
3. **Subclass** every child control + the parent at `WM_INITDIALOG`
   (`FUN_0060f9a0`), classifying each by Win32 class name → owner-draw paint proc
   + a 0x208-byte per-control record, behind one shared wndproc `0x00610ca0`.
4. **Re-anchor** children for the current resolution (`FUN_0060c4a0` reposition
   pass, gated by include-test `FUN_0060c540`); DLU→pixel happens once at create.
5. **Compose** each frame: offscreen BSurface ← right-panel chrome ← MNSCRN
   background ← owner-draw controls, then flip to the alternate surface
   (`WM_PAINT_Handler 0x00621E90`).
6. **Animate** a one-shot first-paint "slide-in" of the controls
   (`FUN_006071E0`), eligibility gated by a dialog-id allow-list.
7. **Pump** a hand-rolled message loop (`FUN_00623120` body) that keeps sim +
   network advancing *behind* even "modal" dialogs.
8. **Route results** out-of-band through a `GWLP_USER` result pointer + a per-loop
   sentinel; `Main_Game 0x0052D9A0` is the navigation state machine that maps each
   dialog's return code to the next dialog or to a scenario launch.
9. **Seed** the skirmish dialog from `rules(md).ini [MultiplayerDialogSettings]`
   (`RulesClass__ReadMultiplayerDialogSettings 0x00671EA0`).
10. **Tear down** with a stack-compacting cleanup that restores foreground/focus to
    the *previous* dialog, not the game window (`FUN_00622720`).

---

## 2. Inventory (verified this session)

### 2.1 Class methods / entry functions (`*` = Ghidra renamed; others are `FUN_`/`LAB_`)

| Address | Role | Verify call |
|---|---|---|
| `0x00622650` | Dialog **factory** — CreateDialogIndirectParamA + push LIFO stack + register routing | `decompile_function 0x00622650` |
| `0x00622b50` | **Common shell DLGPROC** — WM_INITDIALOG / WM_DESTROY / WM_PAINT / hit-test | `decompile_function 0x00622b50` |
| `0x00622820` | Init bridge — subclass children + set slide-group markers by dialog id | `decompile_function 0x00622820` |
| `0x00622800` | Show — ShowWindow(SW_SHOWNORMAL)+SetForegroundWindow | `decompile_function 0x00622800` |
| `0x00622720` | **Teardown** — slide-out, DestroyWindow, LIFO compact, restore focus | `decompile_function 0x00622720` |
| `0x00623120` | **Pump tick (body, not loop)** — Process_NetworkMessages first; then `Network_ServiceLoop`-only for mode 0/5 or blocker globals; otherwise guarded `FUN_0055CBF0` -> `Main_Tick`, returning 1 only when `Main_Tick` returns nonzero | `decompile_function 0x00623120`; `MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md` |
| `0x005d4d50` | `Process_NetworkMessages` — Peek/Get loop, IsDialogMessageA per registered HWND | `decompile_function 0x005d4d50` |
| `0x005d4e70` / `0x005d4ed0` | Register / unregister HWND in keyboard-routing array | `decompile_function 0x005d4e70` |
| `0x0060f9a0` | **Owner-draw subclass setup** (`ownrdraw.cpp`) — classify, install wndproc, alloc record | `decompile_function 0x0060f9a0` |
| `0x00610ca0` | **Shared subclass wndproc** — input/paint dispatch heart | `decompile_function 0x00610ca0` |
| `0x00623340` | 0x208 record initializer (zero, kind=0xB, font=g_GAME_FNT) | `decompile_function 0x00623340` |
| `0x0060c4a0` | **Reposition pass** — expand parent fullscreen + EnumChildWindows(ResizeShellChildControl) | `decompile_function 0x0060c4a0` |
| `0x0060c540` | **Include-test + slide-marker setter** (DRIFT-CORRECTED; *not* the pass) | `decompile_function 0x0060c540` |
| `0x0060c0c0` | `ResizeShellChildControl` — per-child first-match-wins re-anchor | `decompile_function 0x0060c0c0` |
| `0x00621E90` | **WM_PAINT_Handler** — mode-1/mode-2 parent composition + flip | `decompile_function 0x00621E90` |
| `0x0060CF00` | Dialog background table — id → (convert, small SHP, large SHP) | `decompile_function 0x0060CF00` |
| `0x0072E450` / `0x0072E730` | RightPanel__Draw / Background_Overlay | `get_function_by_address` |
| `0x00612B70` | OwnerDraw_Button — paint-type fork (+0xB0), frames 2/3/4 | `decompile_function 0x00612B70` |
| `0x006153E0` | OwnerDraw_Static — kind 0..4, text/image/SHP/movie | `decompile_function 0x006153E0` |
| `0x0061D950` `0x006163A0` `0x00617250` `0x00618D40` `0x00616980` `0x0061E700` | Trackbar / Checkbox / ComboBox / ListBox / RadioVariant / ButtonVariant owner-draw | `get_function_by_address` |
| `0x00621040` | `ShellText__DrawInRect` — 0x00BBGGRR color permutation, default yellow | `decompile_function 0x00621040` |
| `0x006071E0` | **Slide animation loop** — SHP-frame sweep, 30ms/tick, N+8 ticks | `decompile_function 0x006071E0` |
| `0x00608260` / `0x00608070` | Slide-IN / slide-OUT triggers (gate +0xC1 && +0xB4 && visible) | `decompile_function 0x00608260` |
| `0x00531CC0` / `0x00531F60` | Main-menu 0xE2 runner / proc (button → 1..6) | `decompile_function 0x00531CC0` |
| `0x0060D380` | Generic SP/Movies dialog runner (cases 1/4) | `decompile_function 0x0060D380` |
| `0x006AE2C0` | Skirmish 0x102 runner (Start 0x617 / Back 0x5C0) | `get_function_by_address 0x006AE2C0` |
| `0x005D3490` | **Generic CSF modal helper** — template 0xCE/0x120/0x121 by text-slot presence | `decompile_function 0x005D3490` |
| `0x0052D9A0` | **Main_Game** — navigation state machine | `decompile_function 0x0052D9A0` |
| `0x00671EA0` | `RulesClass__ReadMultiplayerDialogSettings` — INI → skirmish defaults | `decompile_function 0x00671EA0` |

### 2.2 Owner-draw class-routing table (static dispatch, in `FUN_0060f9a0`)

`GetClassNameA` → strcmp cascade (fixed order), then Button by `GWL_STYLE` low byte:

| Win32 class | Owner-draw proc | kind |
|---|---|---|
| ScrollBar | `OwnerDraw_ScrollBar_0061C690` | 8 |
| ListBox | `OwnerDraw_ListBox_00618D40` | 4 |
| ComboBox | `OwnerDraw_ComboBox_00617250` | 3 |
| msctls_trackbar32 | `OwnerDraw_Trackbar_0061D950` | 7 |
| msctls_progress32 | `LAB_0061D6D0` | 6 |
| NewEdit / Edit | `OwnerDraw_NewEdit_00614B30` / `OwnerDraw_Edit_00614190` | 1 |
| Static | `OwnerDraw_Static_006153E0` | 2 |
| SysTabControl32 | `LAB_006137D0` | 0xA |
| Button `(s&7)==7` | `OwnerDraw_ButtonVariant_0061E700` | 0 |
| Button `(s&0xB)==0xB` | `OwnerDraw_Button_00612B70` | 0 |
| Button `(s&3)==3` | `OwnerDraw_Checkbox_006163A0` | 0 |
| Button `(s&9)==9` | `OwnerDraw_RadioVariant_00616980` | 0 |
| msctls_hotkey32 | `LAB_0061ECA0` | 9 |
| *(no match)* | `LAB_00612A60` | 0xB |

`GWL_USERDATA[0]` can override the effective class string before classification —
a control can masquerade as another class.

### 2.3 Singleton state / registries / static tables

**Two dialog registries (distinct lifetimes):**
- **LIFO display/focus stack**: `DAT_00b72d28` (HWND), `DAT_00b72d2c` (id), stride 8,
  depth `DAT_00b72f50`, top mirror `DAT_00b72f44`/`DAT_00b72f48`. Pushed by factory,
  compacted by teardown.
- **Keyboard-routing array**: `DAT_00abfc94` (HWND[]), count `DAT_00abfca0`. Appended
  by `FUN_005d4e70`, pruned at `WM_DESTROY` by `FUN_005d4ed0`. Scanned by
  `Process_NetworkMessages` for `IsDialogMessageA` before generic dispatch.

**Owner-draw runtime registries (HWND-keyed hashtables):**
- `DAT_00AC18C0` — HWND → owner-draw paint proc (bucket `{HWND, proc, next}`).
- `DAT_00AC1B48` — HWND → original WndProc.
- `DAT_00AC1B00` — HWND → 0x208 record (bucket *is* the record; chain at `[0x81]`).
- `DAT_00AC1DE8` — z-order/topmost HWND array; `DAT_00AC48DC` — paint-depth counter;
  `DAT_00AC48D4` — one-time color/PCX-preload guard.

**Surfaces / theme:** `DAT_00887310` AlternateSurface (composition target),
`DAT_0088730C`/`DAT_00887308` Hidden/Primary (Bink only), `DAT_00ac18a4` default
shell text color `0xFFFF` (yellow), `DAT_00ac1cb4` disabled text `#9F0000`.

**Static dims:** `DAT_007F5BE0=640`, `…E4=800`, `…E8=1024`, `…EC=480`, `…F0=600`;
sidebar inset `0x007F5BF8=168`; SDBTNANM cell 156×42 (`g_SDBTNANM_SHP[+2]/[+4]`).

**Skirmish-default struct** (`RulesClass + 0x1480..0x14BB`): 11 ints (money/unit
sliders, TechLevel, GameSpeed, AIDifficulty, AIPlayers) + 16 bools
(Bases/Crates/ShortGame/FogOfWar/… — byte map in §5).

### 2.4 The 0x208 per-control record (offsets from data root = bucket+4)

| Off | Field | Meaning |
|---|---|---|
| `+0x00` | DLGPROC/dialog param | set in `FUN_0060f9a0` |
| `+0x14` | per-dialog offscreen BSurface (lazy) | parent paint target |
| `+0x28`/`+0x2C` | owned wide-text buffer / text-dirty flag | `0x4B2`/`0x4B4` |
| `+0x38` | focus flag | WM_SETFOCUS/KILLFOCUS |
| `+0x68` | **control-kind code 0..0xB** | classifier output |
| `+0x70` (int idx 0x1c) / `+0x6C` | dialog resource id — include-test/slide/bg read the id via int-index `0x1c` (= byte `+0x70`); `+0x6C` also carries an id in some reposition branches (confirm per-branch before wiring) | reposition + slide + bg lookups |
| `+0x70` | static SHP-anim kind / typewriter | kind-4 statics |
| `+0x74`/`+0xE0`/`+0xE4` | bg convert / small SHP / large SHP | `FUN_0060CF00` |
| `+0xB0` | **parent paint-mode (1/2) & Button paint-asset (0 PCX / 1 SDBTNANM / 2 / 3)** | `WM_PAINT_Handler`; `LAB_0060A330` sets 1 on 0xE2 |
| `+0xB4` | slide-eligible marker (=1) | `FUN_0060c540` |
| `+0xC1` | **slide-IN gate byte** (two setters) | `FUN_0060c540`, `FUN_00608380` |
| `+0xC2` | slide-OUT deferred dirty byte | `FUN_00608070` |
| `+0xC5` | hover/flash byte → Button frame 3 | `0x4DC`/`0x113`/timer |
| `+0xD5..+0xD8` | per-dialog slide-group / SDBTNANM-frame-10 (WOL) gates | `FUN_00622820`/`FUN_00608440` |
| `+0xD9..+0xDC` | paint-overlay flags (`FUN_0060CAF0/C930/CCC0/CDB0`) | checkbox icon variants etc. |
| `+0x1FC` | **first-paint slide state machine (1→2→3)** | `0x00612690` in wndproc |
| `+0x204` | hash-chain next ptr | registry |

### 2.5 vtable / COM slots

The substrate is **wndproc-dispatched, not vtable-dispatched** — there is no
GadgetClass-style vtable here. The only COM/vtable indirection is on the draw
surface object: `DSurface`/`BSurface` vtable `+8` = blit (every composition blit +
the final flip), `+0x14` = fill rect, `+0x78` = surface info; the Bink movie object
uses its own vtable (`+0x18` open, `+0x28` ExplicitDraw). These belong to the
render backend, not the dialog substrate.

---

## 3. Active-YR vs TS-legacy / dormant partition

**Default verdict is DRIFT; items below are downgraded only with cited evidence.**

### Active (reproduce exactly)
- Factory + two registries + LIFO focus restore + GWLP_USER result channel + pump.
- Owner-draw subclass + 0x208 record + shared wndproc dispatch.
- Reposition pass for the include-set dialogs **0xE2, 0x6B, 0x100, 0x102**.
- Mode-1 composition (right-panel → MNSCRN → controls → flip); mode-2 PUDLGBG modal.
- First-paint slide for the ~58-id allow-list (incl. 0xE2/0x94/0x6B/0x100/0x101/0x102).
- `Main_Game` navigation; `FUN_005D3490` modal family; `[MultiplayerDialogSettings]`.
- `MenuSlideIn` (GUIMoveInSound) at slide start.

### Legacy / dormant / refuted (do NOT implement as default)
| Path | Status | Evidence |
|---|---|---|
| PCX `bue_*30`/`bde_*30` 3-piece button art on 0xE2 | **REFUTED** — SDBTNANM frames 2/3/4 is live | `LAB_0060A330` sets `+0xB0=1`; `decompile 0x00612B70` |
| `bud_*`/`bdd_*` disabled-art PCX family | **DEAD** — format `%c` hardcoded `'e'`; disabled = AlphaBlendRect | disasm `0x00612B70` |
| SDBTNANM frame-10 overlay (`+0xD8`) | **WOL-only** — 4 setters, all WOL procs (0x113/0xC4/0x130) | `get_xrefs_to 0x00608440` |
| Static `0x71C` SHP-animation | **INACTIVE** on 0xE2 — no writer of its kind/SHP | `STATIC_0X71C` doc |
| `0x4DC` "hover" message | **network-dialog-only** (ctrl 0x59F, netdlg2/wonline) | `HOVER_DISPATCHER` doc |
| `0x120`/`0xCE` in reposition include-set | **NOT included** — modal-centered only | `decompile 0x0060c540` |
| Modem dialogs (g_GameMode 1/2) | **dead** — needs serial hardware | `MAIN_GAME` doc |
| WWOnline (case 2 / g_GameMode 4) | **live code, servers offline** | `MAIN_GAME` doc |
| `GaugeClass` / `Dial8Class` | **absent from binary** ("TS ghosts") | `GADGET` §1.1 |
| `FogOfWar` "previously-seen" darkening | **TS legacy, default-off** | `[MultiplayerDialogSettings] FogOfWar=no` |
| WOL background rows in `FUN_0060CF00` (0xE6/0xF3/0x10F/0x113/…) | **WOL-only** | `decompile 0x0060CF00` |
| ShellButtonSlideSound at slide end | **active code, silent output** — stock key empty | `rulesmd.ini:712` empty |

**DRIFT-CORRECTED vs prior docs/seeds (re-verified this session):**
1. `FUN_00623120` is the pump **loop body**, not the loop (the loop is in each owner).
2. `FUN_0060c540` is the **include-test + slide-marker setter**; the actual
   EnumChildWindows reposition pass is `FUN_0060c4a0`.
3. `+0xC1` has **two** setters (`FUN_0060c540` register-store at `0x0060c7bf` +
   `FUN_00608380`); the old "single setter, slide only on Load/Save" conclusion was
   a byte-pattern-search false negative. The slide **is** generic on first paint.
4. `0x120`/`0xCE` are **not** in the reposition include-set.

---

## 4. Current Rust architecture (live read this session)

No shared substrate. Three pixel-parity shells each re-implement the same
primitives; one mechanism (`app_shell_transition.rs`) is already generic.

| Concern | main_menu_shell (0xE2) | single_player_shell (0x100) | skirmish_shell (0x102) |
|---|---|---|---|
| `RectPx` + `mul_div_round`/`dlu_rect` | `layout.rs:14-123` | imports + own copy `layout.rs:50-74` | 3rd copy `layout.rs:42-67,229-245` |
| right-panel/lower-strip geometry | `layout.rs:147-208` | `layout.rs:90-149` | `layout.rs:447-476` + render recompute |
| SDBTNANM snap / back rect | `layout.rs:289-302` | `layout.rs:169-195` | `layout.rs:490-516` |
| owner-draw button hit + press-match | `state.rs:90-129` | `state.rs:60-116` | `state/hit_test.rs:292-331` + `app.rs:1426-1437` |
| button paint (`push_entry_*`, wave) | `app_main_menu_shell_render.rs:47-292` | `app_single_player_shell_render.rs:36-300` | `app_skirmish_shell_render/chrome.rs` |
| combo / trackbar / checkbox / listbox | — | — | `state/combos.rs`, `state/trackbars.rs`, `layout.rs` (skirmish-only) |
| modal | egui (`main_menu_dialogs.rs`) | — | pixel-parity `…/modals.rs` + own scroll math |
| first-paint slide | **shared** `app_shell_transition.rs` | shared | shared |

**The clean seam already exists and the layering rule is honored:**
`ui/<shell>/{layout,state}.rs` are render-agnostic (RectPx + ids + hit/state, no
asset/render deps, per `ui/mod.rs:10-12`); `render/<shell>_chrome.rs` +
`app_<shell>_render.rs` own the atlas + sprite construction; the app layer routes
input (`app.rs:1364-1566`). Data flows one way: ui produces rects+actions,
render/app consume them. `render/shell_text.rs:draw_in_rect` and
`render/shell_transition_pass.rs:ShellRenderTarget` are the existing shared seams.

---

## 5. gamemd-native behavior contract (what the Rust service MUST reproduce)

**C1 — Lifecycle ordering (per open):** create → push LIFO stack → register
keyboard routing → WM_INITDIALOG (alloc record, classify+subclass children, set
resource id, set slide markers, SetFocus) → publish `&result` (sentinel) → show →
pump. All four registry/stack effects happen in factory order.

**C2 — Pump keeps shell responsiveness live, but sim advance is MODE-GATED.**
`FUN_00623120` always calls `Process_NetworkMessages` first; campaign/offline-skirmish
modes 0 and 5, or the `DAT_00A8D60E`/`DAT_00A8DAB4` blockers, take a
`Network_ServiceLoop`-only branch and do **not** call `Main_Tick`. Only non-offline
modes — practically LAN 3 and WOL/Internet 4 — can call `Main_Tick`, and only when
`DAT_00ABCD58`/`FUN_0055CBF0` says no tick is already active. **Offline in-game Options
FREEZES world/frame advancement** while the dialog stays message-responsive (the
battlefield does NOT animate behind it); network Options can advance through `Main_Tick`.
[Corrected 2026-06-12 per `docs/research/MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`;
the prior "pump keeps the world live / battlefield animates behind options" wording was
WRONG for the offline client.]

**C3 — Keyboard routing is per-registered-dialog, in registration order**
(Tab/Enter/Esc/accelerators), independent of the LIFO focus stack.

**C4 — Result channel:** dialog writes a small int through the result pointer on
WM_COMMAND; owner loop spins until int ≠ sentinel. Exact codes:
main-menu 0x683→1, 0x684→2, 0x578→3, 0x686→4, 0x55C→5, 0x3EE→6; SP 0x579→0x0B,
0x686→0x12, 0x688→8, 0x689→9; skirmish Start 0x617, Back 0x5C0; options OK 0x5CB.
**Single Player is an intermediate dialog (0x100), not a direct jump to skirmish.**

**C5 — Teardown restores focus to the previous dialog** (LIFO compact), or the game
window when the stack empties; keyboard-routing entry is pruned at destroy.

**C6 — Subclass classification** is by Win32 class name then Button style bits in
the exact cascade of §2.2; USERDATA can override the class; no match → kind 0xB.

**C7 — DLU→pixel once, then pixel-only re-anchoring.** `x_px=MulDiv(x_dlu,6,4)`,
`y_px=MulDiv(y_dlu,13,8)` (MS Sans Serif 8pt, round-half-up). Then, *only for
include-set dialogs* (0xE2/0x6B/0x100/0x102): expand parent to full screen, and
re-anchor only allow-listed children; **all other children keep their create-time
rect relative to the expanded parent** (the 0x102 "preserve" policy). Centering
delta = `max(0,(parent_w-800)/2)` (0 at 800×600, 112 at 1024). Every branch ends
with a 1-px finalizer (`FUN_0060B950`: e.g. 0xE2/0x694 Y+7,H+1). Owner-draw button
hit rects follow the **snapped** rect (Start 644,241,156,42), not the resource rect.

**C8 — Paint composition order (mode-1):** lazy offscreen BSurface → RightPanel
chrome (SDTP → SDBTNBKGD column → SDBTM → LWSCRN*) → MNSCRN background **on top** →
conditional overlays → single full-rect flip to AlternateSurface. Owner-draw
controls paint into their own surface and blit to the same alternate. Inverting
right-panel vs background order changes the visible result.

**C9 — Button art = SDBTNANM frames 2 (idle) / 3 (hover, ~1 Hz flash via +0xC5) /
4 (pressed)** on the main menu; pressed text sinks +2y,+1x. Mode-2 modals use
PUDLGBGN.SHP + DIALOGN.PAL background and MNBTTN.SHP owner-draw OK button — never a
flat bevelled rectangle, never skirmish PCX art.

**C10 — Text:** all shell text routes through the `0x00BBGGRR` permutation;
enabled = yellow `#FFFF00`; disabled Static/Checkbox/Trackbar = `#9F0000`; single
1-bpp glyph blit, no shadow.

**C11 — First-paint slide:** one-shot per dialog (state 1→2→3), eligibility by
dialog-id allow-list (not "skirmish only"); SHP-frame-index sweep (not an x/y
ramp), 30 ms/tick (`Sleep(0x1e)`), **loop bound = max per-cell stagger + 6**
(≈ N+6; matches the existing Rust `WAVE_TAIL_TICKS = 6` in
`app_shell_transition.rs` — NOT N+8), 1-tick per-cell stagger; `MenuSlideIn` at
start, `ShellButtonSlideSound` at end (silent in stock); slide-out deferred via +0xC2.

**C12 — Navigation is one result-routed loop**, not independent per-dialog handlers;
the return-code→next-dialog table is load-bearing (e.g. quit confirm → write INI →
graceful return cascade, no process-kill).

**C13 — Modal text-slot routing (verified) + template selection (UNCHECKED).**
*Verified:* `FUN_005D3490` routes up to 4 CSF text ptrs to control ids
`0x5b0`/`0x5ae`/`2`/`0x5af` by slot presence, installs the result pointer at
`GWLP_USER`, and pumps the common loop. CSF text resolved live (never hardcode
English). **[UNCHECKED]** the body+OK→`0xCE` / +cancel→`0x120` / +3rd→`0x121`
*template-id* selection is NOT visible inside `FUN_005D3490` (it calls the factory
with a caller-supplied template); **trace a caller of `0x005D3490` before wiring
`ModalKind`.**

**C14 — Skirmish defaults from `[MultiplayerDialogSettings]`** seed the 0x102
controls exactly (`+0x1480..+0x14BB`): MinMoney/Money/MaxMoney/MoneyIncrement,
MinUnit/Unit/MaxUnit, TechLevel(10), GameSpeed(1), AIDifficulty, AIPlayers,
Bridge(yes), ShadowGrow(no), Shroud(yes), Bases(yes), TiberiumGrows(yes),
Crates(yes), CaptureTheFlag, HarvesterTruce, MultiEngineer(no), AlliesAllowed,
ShortGame(yes), FogOfWar(**no**), MCVRedeploys(yes), SuperWeapons, BuildOffAlly,
AllyChangeAllowed(yes).

---

## 6. Rust-native replacement boundary

A single `ui::shell` substrate service that encodes the contract above with plain
Rust data + a one-way pipeline. No Win32 concepts cross into sim/.

```
ui/shell/                         (render-agnostic; depends only on sim/ + rules)
  geom.rs        RectPx, mul_div_round, dlu_rect, center_offset, RightPanelGeom,
                 sdbtnanm_snap, lower_strip   ← the ONE copy
  descriptor.rs  DialogId, DialogDescriptor { id, controls, bg_kind, slide_eligible,
                 reposition_policy }, ControlDescriptor { id, kind, dlu_rect,
                 csf_key, tooltip_key, group, enabled }
                 ControlKind = Button | Checkbox | Radio | Combo | Listbox |
                               Trackbar | Edit | Static | ScrollBar
  layout.rs      layout_pass(&DialogDescriptor, screen_w, screen_h)
                 -> Vec<(ControlId, RectPx)>      // C7: DLU→px once, include-set
                                                  // re-anchor, 1px finalizers
  input.rs       DialogController { stack: Vec<DialogInstance>, kbd_route: Vec<…> }
                 - hit_test (owner-draw rect, smaller-area N/A: dialogs don't overlap)
                 - press/release "must match" state (C4/C6)
                 - on_event(ptr|key) -> Option<ShellAction>
  result.rs      ShellAction enum + result-code map (C4); navigation table (C12)
  slide.rs       SlideState per dialog (Idle→Running(tick)→Done), frame-index
                 schedule (C11); reuses today's app_shell_transition logic
  modal.rs       ModalKind::{Body, BodyOk(0xCE), Confirm(0x120), ThreeButton(0x121)}
                 selected by text-slot presence (C13)
  defaults.rs    MultiplayerDialogSettings parse → control seed values (C14)

render/shell_paint/                (asset-coupled; consumes RectPx + kind + state)
  trait OwnerDrawControl { fn paint(&self, atlas, rect: RectPx,
                                    st: ControlPaintState, out: &mut Vec<Sprite>) }
  impls: button (SDBTNANM 2/3/4, C9), checkbox, trackbar, combo, listbox, static…
  parent_compose(): offscreen ← right-panel ← MNSCRN ← controls ← flip  (C8)
  text: render/shell_text.rs::draw_in_rect (already shared; yellow default, C10)

app/                               (orchestration only)
  drives DialogController, pumps service_tick (C2: sim+net advance behind modals),
  pushes/pops dialogs (C1/C5), feeds RectPx+state into render/shell_paint.
```

Mapping of gamemd mechanism → Rust-native model (semantics preserved, plumbing dropped):

| gamemd | Rust-native |
|---|---|
| `HWND` + 3 hashtables | a `DialogInstance { id, controls: Vec<ControlState>, slide, surface }` owned by the controller |
| LIFO `DAT_00b72d28` stack + focus restore | `DialogController.stack: Vec<DialogInstance>` (push/pop-compact) |
| keyboard-routing array + `IsDialogMessageA` | `kbd_route: Vec<DialogId>` walked in registration order for Tab/Enter/Esc |
| `GWLP_USER` result pointer + sentinel | `on_event` returns `Option<ShellAction>`; controller owns the value |
| owner-draw subclass + 0x208 record | `ControlState { kind, rect, pressed, hovered, focus, paint_asset, … }` |
| `WM_PAINT` mode-1/2 | `parent_compose()` + `OwnerDrawControl::paint` |
| `+0x1FC` slide state machine | `SlideState` enum |
| `FUN_005D3490` template family | `ModalKind` |
| `Main_Game` switch | `navigation table` keyed by `ShellAction` |

This is a **consolidation, not a re-layering**: the one-way `ui → render/app` flow
already exists; the substrate just gives the three shells one shared home.

---

## 7. Ad-hoc Rust logic to retire

(Quoting the live read; line numbers as of this session.)

1. **Three `RectPx` + `mul_div_round`/`dlu_rect`/`center_offset` copies** →
   `ui/shell/geom.rs`. Delete `main_menu_shell/layout.rs:14-123`,
   `single_player_shell/layout.rs:50-74`, `skirmish_shell/layout.rs:42-67,229-245,427-429`.
2. **Three `right_panel_rects`/`lower_strip_rect`** → one `RightPanelGeom`. Delete
   `main_menu_shell/layout.rs:147-208`, `single_player_shell/layout.rs:90-149`,
   `skirmish_shell/layout.rs:447-476` + render recompute in `chrome.rs:585-612`.
3. **Three `owner_draw_button_snap_rect`/`back_rect`** → one. Delete
   `single_player_shell/layout.rs:169-195`, `skirmish_shell/layout.rs:490-516`,
   fold `main_menu_shell/layout.rs:289-302`.
4. **Three button hit/press-match copies** → one descriptor-driven hit/press model.
   Delete `main_menu_shell/state.rs:90-129`, `single_player_shell/state.rs:60-116`,
   `skirmish_shell/state/hit_test.rs:292-331`, `app.rs:1426-1437`.
5. **Duplicated paint emitters** (`push_entry_sized`/`push_clipped_top`/
   `push_button_wave_frame`/`build_chrome_instances` + pressed-offset recompute) in
   `app_main_menu_shell_render.rs:47-292` and `app_single_player_shell_render.rs:36-300`
   → one `OwnerDrawControl` paint pass.
6. **Three `render_shp_entry`/`render_pcx_entry`/`pack_entries` atlas builders**
   (`main_menu_shell_chrome.rs`, `skirmish_shell_chrome.rs`, `loading_screen_chrome.rs`)
   → one atlas-pack utility.
7. **Within skirmish, two scrollbar/thumb/track implementations** (combo dropdown
   `combos.rs:152-257` vs choose-map listbox `layout.rs:702-831`) → one `Listbox`
   scroll model.
8. **Three per-shell `app.rs` dispatch clusters** (`:1364-1566`) → one
   `DialogController` event router.
9. **Keep `app_shell_transition.rs`** — it is the already-generic model the rest
   follows (generalize, don't rewrite).

Do NOT touch (separate concerns): the egui pragmatic menu/dialogs
(`main_menu.rs`, `main_menu_dialogs.rs`, `pause_menu.rs`) unless a later slice folds
them into the same modal abstraction; the in-game GadgetClass/sidebar.

---

## 8. Migration slices + acceptance tests

Incremental, each slice ships green and is independently verifiable. The bar is
**indistinguishable from gamemd.exe**, so acceptance is pixel/timing/result parity.

**Slice 0 — extract `ui/shell/geom.rs` (pure refactor, zero behavior change).**
Move RectPx + DLU helpers + right-panel/snap/lower-strip into one module; the three
shells import it.
*Accept:* existing skirmish `state/tests.rs` (2147 lines) still green; a golden test
asserts `mul_div_round` matches `MulDiv` round-half-up for all odd DLU in 0..1024;
byte-identical computed rects for 0xE2/0x100/0x102 at 800×600 and 1024×768 vs the
pre-refactor values.

**Slice 1 — `DialogDescriptor`/`ControlDescriptor` + `layout_pass`; convert
main_menu_shell first.** 0xE2 becomes a descriptor table feeding the shared pass.
*Accept:* 0xE2 button rects = (644-snapped 156×42, Exit at template rect), title
0x694 with Y+7/H+1; screenshot diff vs current 0xE2 == 0 changed pixels; include-set
gating verified (0x120/0xCE excluded).

**Slice 2 — descriptor-driven hit/press model + `DialogController` router; migrate
0xE2 + 0x100 input.** Retire the per-shell hit/press copies and two `app.rs`
clusters.
*Accept:* press-must-match-release (press button A, drag to B, release = no fire);
SP intermediate dialog still routes 0x579→skirmish (not a direct jump); focus
restore on close returns to the parent dialog. Unit tests on the router cover each
control id → ShellAction. **(C3)** a controller test with two stacked registered
dialogs asserts Tab/Enter/Esc are offered to dialogs in *registration order*,
independent of the LIFO focus stack.

**Slice 3 — `OwnerDrawControl` paint trait; migrate 0xE2 + 0x100 chrome.** One paint
pass over descriptors; retire duplicated emitters and one atlas-pack copy.
*Accept:* SDBTNANM frames 2/3/4 with ~1 Hz hover flash; pressed text sink +2y/+1x;
yellow #FFFF00 / disabled #9F0000; screenshot diff == 0 vs current per state.

**Slice 4 — fold skirmish_shell (0x102) onto the substrate.** Combo/trackbar/
checkbox/listbox become `ControlKind`s; unify the two skirmish scroll models.
*Accept:* combo open/scroll/select, trackbar drag value, checkbox icon-vs-label hit,
choose-map listbox scroll all behave identically (existing `tests.rs` green);
`[MultiplayerDialogSettings]` defaults seed every control byte-exact (TechLevel 10,
GameSpeed 1, FogOfWar off, …).

**Slice 5 — modal substrate (`ModalKind`) + lifecycle/pump contract.** Quit-confirm
and validation modals use the template-by-text-slot rule and mode-2 SHP composition;
`service_tick` runs message/input/repaint always, and advances sim **only on
mode-gated branches** (LAN 3 / WOL 4), freezing offline {0 campaign, 5 skirmish}.
*Pre-req:* **RESOLVED (2026-06-12).** C13 template-id mapping resolved (`src/ui/shell/modal.rs`
+ binary citations); C2 pump resolved (`MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`).
No remaining Ghidra blocker.
*Accept:* validation modal renders PUDLGBGN.SHP+DIALOGN.PAL + MNBTTN owner-draw OK
(not a flat panel / not push_button_30 PCX); quit-confirm writes ra2md.ini before the
graceful return cascade; **(C2)** an automated assertion that during **offline skirmish**
the world tick delta is **0** per pumped frame while the modal is open (the world FREEZES;
the prior "advances ≥1 / battlefield animates" criterion was WRONG for the offline client)
while the dialog stays message-responsive; the network-mode (LAN/WOL) advance branch is a
separate test.

**Slice 6 — slide eligibility data-driven.** Drive `app_shell_transition` off the
dialog-id allow-list + per-dialog first-paint state; add the missing 0x100 (and 0x94
if/when present) first-paint slides.
*Accept:* every include-set shell slides once on first paint; 30 ms/tick,
loop bound = max stagger + 6 (≈ N+6, tail = 6 — reuse `WAVE_TAIL_TICKS`),
SHP-frame sweep (not x/y ramp); `MenuSlideIn` at start, silent end cue (stock);
non-listed transient dialogs do not slide.

---

## Sources

7-agent decode workflow `wf_02b12edf-d44`, this session (read-only Ghidra +
research-index + live `src/` read). All §2 addresses re-verified via the cited
`decompile_function`/`get_function_by_address`/`get_xrefs_to` calls. Prior docs
extended/corrected: `GADGET_UI_FRAMEWORK`, `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER`,
`SHELL_TRANSITION_ON_MAIN_MENU_CLICK` (DRIFT-corrected), `FUN_0060CF00_*`,
`RESIZESHELLCHILDCONTROL_*`, `MAIN_GAME_STATE_MACHINE_CASES`,
`VALIDATION_MODAL_0X005D3490_*`, `SDBTNANM_FRAME10_OVERLAY_GATE`,
`MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330`, `RT_DIALOG_0X120_RESOURCE_LAYOUT`, the
`skirmish-ui/` family. Open questions carried per-area in the workflow record
(notably: full per-dialog WM_COMMAND→result maps beyond 0xE2/0x100/0x102; 0x101/0x129
procs lack Ghidra fn boundaries; exhaustive `+0xD9/+0xDA/+0xDB` slide-group writers;
runtime pixel capture of `FUN_006071E0`).
