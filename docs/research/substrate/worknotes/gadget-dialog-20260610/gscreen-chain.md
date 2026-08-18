# GScreen Chain Lane — Orchestration Spine Worknotes (2026-06-10)

**Lane scope:** GScreenClass + per-frame input/draw cycle; how Framework A (gadget tree)
and Framework B (RT_DIALOG shell dialogs) coexist at runtime.
**Status:** IN PROGRESS — sections appended incrementally.
**Evidence discipline:** every address claim cites the live MCP call. Tags:
VERIFIED-LIVE (this session) / DOC-INHERITED (cited doc) / INFERRED.

## 0. Prior-doc baseline (DOC-INHERITED)

- `GSCREEN_RTACTICAL_GHIDRA_REPORT.md` (664 ln): GScreenClass base vtable 0x007EA6FC
  (22 slots), `g_DisplayChain` = MouseClass* @ 0x00887640 (0x556C bytes), `g_Tactical`
  @ 0x00887324; Main_Tick 0x0055D360 order = Input → LogicClass::AI → Map::Logic →
  RenderFrame_main (vtable[15] = 0x004F4480, NOT overridden down the chain).
  GScreenClass::Input 0x004F4320 = vtable slot 9 (+0x24).
- `MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md` (2026, post-correction):
  pump = Process_NetworkMessages 0x005D4D50 first; offline modes {0,5} or blockers
  (DAT_00A8D60E / DAT_00A8DAB4) → Network_ServiceLoop 0x0048D080 only, NO Main_Tick;
  LAN(3)/WOL(4) → guarded Main_Tick via reentrancy byte DAT_00ABCD58 (read by
  FUN_0055CBF0). Offline in-game Options FREEZES sim; battlefield does NOT animate.
- In-game options: State_Machine 0x0048C8B0 case 5 → OptionsClass__ShowInGameDialog
  0x004E1D00 → dialog 0xBBB (g_GameActive==1) / 0xF5, proc 0x004E1FE0, loops on pump.

(Everything below is re-verified live unless tagged otherwise.)

---

## 1. GScreenClass::Input 0x004F4320 — full contract (VERIFIED-LIVE)

Decompiled + disassembled (verified via `decompile_function 0x004F4320`,
`disassemble_function 0x004F4320`). Signature: `__thiscall Input(this, uint* outKey,
int* outX, int* outY)` — `RET 0xC`, thiscall, this = display-chain object.

Exact body order:

1. `*outX = vtbl[+0x2C](obj @ [0x00887640])`, `*outY = vtbl[+0x30](obj @ [0x00887640])`
   — the object polled for mouse X/Y is the global at **0x00887640**, which is NOT the
   display chain (see §1.1) but the low-level mouse singleton (WWMouseClass*). Written
   in WinMain @ 0x006BDF25 (verified via `get_xrefs_to 0x00887640`).
2. If `DAT_00A8EF54 == 0` (gadget list head, "Buttons" — empty):
   `*outKey = Keyboard_Check() & 0xFFFF` where Keyboard_Check = `0x0054F000` thiscall on
   keyboard singleton ptr @ **0x0087F770**; if nonzero, `*outKey = Keyboard_Get()
   & 0xFFFF` (`0x0054F050`, same object). (verified via disassembly 0x004F4394..0x004F43BC)
3. Else (gadget root present):
   a. `DAT_00A8EF54->vtbl[+0x5C]()` = **GadgetClass::Any_Redraw_Pending** (0x004E1A00,
      per GADGET_UI_FRAMEWORK §8 slot 23 — reads DAT_00A8EF54 at 0x004F43F9 confirm the
      undefined fn 0x004F43F0 is the list-membership helper, NOT this). If any gadget is
      dirty → call `this->vtbl[+0x38](0)` = **Flag_To_Redraw(0)** (0x004F42F0): sets
      `g_Tactical+0xD7D = 1` (tactical dirty flag); param==0 so chain RedrawFlag (+0xC)
      untouched (verified via `decompile_function 0x004F42F0`).
      **DRIFT vs GSCREEN_RTACTICAL §7:** doc calls the +0x5C call "Hit_Test-ish" and the
      +0x38 call "Clear_Gadget_Root (DAT_00A8EF54 = 0)" — both WRONG. +0x38/slot 14 is
      Flag_To_Redraw; the doc's own §3 row idx-14 ("same body as idx 8") is also wrong.
   b. Surface swap: save `[0x00887314]` (named g_PrimarySurface in decomp), set it to
      `[0x0088730C]`, call `DAT_00A8EF54->vtbl[+0x28]` = **GadgetClass::Input 0x004E1640**
      on the list head, store FULL untruncated result in `*outKey` (button IDs with
      0x8000 flag survive; keyboard path masks to 16 bits, gadget path does not),
      restore `[0x00887314]`. (verified via disassembly 0x004F4367..0x004F4391)
4. Unconditionally: copy x,y to a stack pair and call `this->vtbl[+0x28](outKey, &xyCopy)`
   = chain **AI(key*, point*)** — final override = 0x005BDDC0 (see §2). NOTE: AI gets a
   COPY of x/y; mutations do not propagate to Main_Tick's locals. The key pointer IS
   shared — AI can rewrite/consume the key.

### 1.1 Label drift: 0x00887640 is NOT the display chain

The display chain singleton is a **static instance at 0x0087F7E8**, constructed by CRT
static initializer `0x0040D190`: `MOV ECX,0x87F7E8; CALL 0x005BDA40 (most-derived ctor);
atexit(0x0040D1B0)` (verified via `disassemble_function 0x0040D190`,
`get_function_by_address 0x005BDA4A` → ctor entry 0x005BDA40 body 0x005BDA40..75).
GSCREEN_RTACTICAL's "g_DisplayChain = MouseClass* at 0x00887640" conflates two globals:
0x00887640 is a pointer written in WinMain (0x006BDF25) — the low-level mouse object
used for Get_Mouse_X/Y (+0x2C/+0x30) and cursor work in RenderFrame_main.
Identity axes for 0x00887640 = "WWMouse-like mouse singleton": content HIGH (slots +0x2C/
+0x30 consumed as x/y by Input), identity MEDIUM (WinMain write site not decompiled this
session), binding HIGH (read by Input/RenderFrame/State_Machine/Main_Game).

### 1.2 Callers of GScreenClass::Input (binding: HIGH)

`get_function_callers 0x004F4320` + `get_xrefs_to 0x004F4320`: exactly 4 call sites —
- `Main_Tick @ 0x0055D360` (call at 0x0055D8AB) — gameplay frame.
- `FUN_0055E160` (call at 0x0055E253) — non-gameplay frame wrapper (DOC-INHERITED:
  loading/paused/cinematic frame service; GSCREEN §8).
- `FUN_00648350` (call at 0x006485B7), `FUN_00648710` (call at 0x00649827) — radar/map
  satellite views? UNVERIFIED identity (see YELLOW). Both 0x0064xxxx = RadarClass region.
Plus 9 DATA refs = the 9 chain vtables (slot 9, +0x24), enumerated in §2.

---

## 2. The layered chain — real order, vtables, override matrix (VERIFIED-LIVE)

All 9 vtables found by data-xrefs to inherited Input (each vtable holds 0x004F4320 at
+0x24). Bases = xref addr − 0x24. Class identity from ctor write sites
(`get_xrefs_to <vtable>`) + method-region clustering + inheritance direction proven by
which slots are shared vs re-overridden (a derived vtable can never "revert" a slot to a
base impl, so shared-slot direction is decisive).

**Chain order (base → most-derived):**

| # | Class | vtable | ctor write evidence |
|---|-------|--------|---------------------|
| 1 | GScreenClass | 0x007EA6FC | (base; read_memory 0x007EA6FC) |
| 2 | MapClass | 0x007ED404 | write @ 0x0056516D in MapClass ctor |
| 3 | DisplayClass | 0x007E6114 | write @ 0x004A8830 in DisplayClass ctor |
| 4 | RadarClass | 0x007F0344 | write @ 0x00652B45 in RadarClass ctor |
| 5 | PowerClass | 0x007EFF54 | write @ 0x0063F71C in PowerClass ctor |
| 6 | SidebarClass | 0x007F3058 | write @ 0x006A4EDF/0x006A4FE1 in SidebarClass ctor |
| 7 | TabClass | 0x007EDFB4 | write @ 0x006CFE86 (ctor; Ghidra label "INoticeSink__Constructor" = DRIFT) |
| 8 | ScrollClass | 0x007F1094 | write @ 0x006922C5 in ScrollClass ctor |
| 9 | **MouseClass (FINAL)** | **0x007E1964** | write @ 0x005BDA4A in ctor 0x005BDA40; ALSO written by static-init dtor region 0x0040D295 |

Inheritance-direction proof for 7→8→9: TabClass introduces One_Time 0x006D0260 /
Init_IO 0x006D03A0 / AI 0x006D0680 / Draw_It 0x006D0A20 (all clustered 0x006D02..0A);
ScrollClass vtable shares [5]=0x006D0260,[7]=0x006A5030,[16]=0x006D0A20 but re-overrides
[10]=0x006922E0 and [8]=thunk; MouseClass shares [8]=thunk,[16]=0x006D0A20 but
re-overrides [5]=0x005BDF30,[7]=0x005BDF50,[10]=0x005BDDC0,[18..21]=0x005BDA80/0x005BDC80/
0x005BDAA0/0x005BDAB0. (verified via read_memory of all 9 vtables)

**DRIFT:** 0x006D0A20, called "MouseClass::Draw" across SIDEBAR/GSCREEN docs, is
**TabClass::Draw_It** by vtable ownership (introduced in TabClass vtable 0x007EDFB4 slot
16, inherited unchanged by Scroll + Mouse). Dispatch target identical, so no behavioral
consequence — naming only.

**Key slot map of the LIVE vtable 0x007E1964** (what g_chain @ 0x0087F7E8 actually runs;
verified via read_memory 0x007E1964):

| Slot | +off | Live target | Role (content-verified where noted) |
|------|------|------------|--------------------------------------|
| 5 | 0x14 | 0x005BDF30 | MouseClass::One_Time |
| 6 | 0x18 | 0x004F42B0 | Init compound: calls vt[7] then vt[8] (DOC-INHERITED GSCREEN §3) |
| 7 | 0x1C | 0x005BDF50 | MouseClass::Init_Clear |
| 8 | 0x20 | 0x0040D270 | **JMP-thunk → 0x006D03A0 = TabClass::Init_IO** (bytes `E9 2B 31 2C 00` verified via read_memory 0x0040D270; ILT incremental-link stub, NOT a no-op) |
| 9 | 0x24 | 0x004F4320 | GScreenClass::Input (never overridden) |
| 10 | 0x28 | 0x005BDDC0 | MouseClass::AI — the input cascade entry (§4) |
| 11 | 0x2C | 0x004F43F0 | Is_A_Button (list-membership test; undefined-in-Ghidra code, reads Buttons head at 0x004F43F9) |
| 12 | 0x30 | 0x004F4410 | **Add_A_Button** (verified via decompile FUN_004f4410: membership-test → append via gadget vtbl+0x10 or set head) |
| 13 | 0x34 | 0x004F4450 | **Remove_A_Button** (Ghidra label "Hide_Cameo_Slots" = DRIFT; verified via decompile: head = btn->Unlink()) |
| 14 | 0x38 | 0x004F42F0 | Flag_To_Redraw(level) — sets g_Tactical+0xD7D, RedrawFlag=level unless already 2 |
| 15 | 0x3C | 0x004F4480 | RenderFrame_main (never overridden) |
| 16 | 0x40 | 0x006D0A20 | TabClass::Draw_It (chrome draw; body ends in SidebarClass::Draw per DOC-INHERITED GSCREEN §6) |
| 17 | 0x44 | 0x004F45B0 | Blit/post-render hook (DOC-INHERITED) |

Buttons list head **DAT_00A8EF54** total writer set (verified via `get_xrefs_to
0x00A8EF54`, exhaustive): One_Time 0x004F42A0 (=0), base Init_IO 0x004F42E0 (=0),
Add_A_Button 0x004F4410, Remove_A_Button 0x004F4450. Readers: Input, RenderFrame_main
(gadget hook), Is_A_Button. **No other system touches the head** — registration is 100%
via Add_A_Button.

Add_A_Button semantics (verified via decompile FUN_004f4410 + decompile 0x00556700):
if `Is_A_Button(btn)` → return 0 (no double-insert). If head exists →
`btn->vtbl[+0x10](head)` = LinkClass::Add_Tail-like: unlink self, walk to tail via
head's vtbl+0x1C, link self AFTER tail. Else head = btn. **List order = insertion
order, appended at tail.** Per GADGET_UI_FRAMEWORK §3 (DOC-INHERITED), GadgetClass::Input
walks the list head→tail calling Handle_Input (first-registered gets first shot) and
Draw_Me in the same order (last-registered draws last = on top).

---

## 3. Who builds the Buttons list, and when (VERIFIED-LIVE)

### 3.1 Init_IO does NOT register gadgets — it initializes button objects

Live chain Init_IO = TabClass::Init_IO 0x006D03A0 (via 0x0040D270 thunk):
- calls SidebarClass::Init_IO 0x006A5310 (Ghidra label `SidebarClass__Init`), then (if
  not map editor, and FUN_0072F430() true) initializes the two tab/toggle button
  records at DAT_00B0CCxx with **IDs 0xF0 / 0xF1** (240/241) + Set_Position
  (FUN_004E1A20) + shape setup (FUN_0069DE00). (verified via decompile 0x006D03A0)
- SidebarClass::Init_IO 0x006A5310: calls RadarClass::Init_IO 0x00653010 first, then
  InitLayoutConstants + InitSidebarRect(0), then initializes static button records:
  ID **0x65** (101, DAT_00B0B3xx) and **0x66** (102, DAT_00B07Exx) = repair/sell pair;
  4 records at DAT_00B07C6C stride 0x60 with IDs **0xCB..0xCE** (203..206) = tab-select
  buttons; IDs **0xC9/0xC8** (201/200, DAT_00B0B345/DAT_00B0B425) = strip scroll
  buttons; then 4× SidebarClass__InitSelectZones (per tab strip); then for net modes
  3/4 the observer/diplomacy row state. **No Add_A_Button call anywhere in this
  function.** (verified via decompile 0x006A5310)
- PowerClass does NOT override Init_IO (vtable 0x007EFF54 slot 8 = 0x00653010 =
  RadarClass's — verified via read_memory 0x007EFF54).

### 3.2 Registration happens at activate/switch time, via Add_A_Button

Exhaustive caller set of Add_A_Button 0x004F4410 (verified via `get_function_callers
0x004F4410`): FUN_006A6820 (tab auto-switch helper), FUN_006A8330, SidebarClass::AI
0x006A7780, SidebarClass__AddCameo 0x006A6300, SidebarClass__SwitchTab 0x006A7590,
SidebarClass__ToggleSidebar 0x006A7D70, StripClass__ActivateButtons 0x006A93F0.

Tab switch (verified via decompile 0x006A6820 and the same inline logic in
SidebarClass::AI): Remove_A_Button on all **0x3C (60)** cameo gadget records of the
old tab (records at DAT_00B07E80, 0x38 bytes each), then Add_A_Button on the new tab's
visible cameo records — visible count = `((sidebarHeight − topStripAdj − DAT_00B0B4F8
− 7 + g_SidebarWidth)/0x32)*2` (two columns of 50-px rows; topStripAdj = 0x1A normal /
0x12 when Scenario+0x34B8 set; observer forced to 1 row).

TabClass::Activate 0x006D04F0 (verified via decompile): calls
SidebarClass__ToggleSidebar, then Add_A_Button/Remove_A_Button **via virtual slots
+0x30/+0x34 on the chain** for the 0xF0/0xF1 toggle records (DAT_00B0CC40/DAT_00B0CCB0)
and the 25-entry command-bar records (DAT_00B0C1C0, stride 0x60, range to 0xB0CB20),
plus ToolTipManager id register/unregister via FUN_00724730 (see §7).

### 3.3 Rebuild events (exhaustive for the Init_IO entry)

Callers of 0x006D03A0 (verified via `get_function_callers 0x006D03A0`):
1. **FUN_00560BF0 = Set_Video_Mode** (string "About to set video mode"; deletes/
   recreates Hidden/Alternate/Tile/Sidebar/Composite surfaces — strings name each —
   re-creates WWMouse via `operator_new(0x98)` + `WWMouseClass__Constructor(
   DAT_00887308 primary, g_hWnd)` stored to 0x00887640, then: Set_View_Dimensions →
   FUN_0072E1B0 → **Init_IO 0x006D03A0 → TabClass::Activate 0x006D04F0(1) →
   SidebarClass__InitSurface → Flag_To_Redraw(2)**. (verified via decompile 0x00560BF0)
   This is THE resolution-change rebuild. Surface identity bonus (from its strings):
   0x00887308=Primary, 0x0088730C=Hidden, 0x00887310=Alternate, 0x008872FC=Tile,
   0x00887300=Sidebar, 0x0088731C=Composite; pointer global 0x00887314 ("g_PrimarySurface"
   in decomps) holds the current draw-target surface.
2. **FUN_0067E440 = Load_Game** (string "LOADING GAME [%s]"): after dat load:
   FUN_00685120 → **Init_IO → Activate(1) → InitSurface** → Tiberium queue re-inits →
   RadarClass__RefreshRadar. (verified via decompile 0x0067E440)

Callers of TabClass::Activate 0x006D04F0 (verified via get_function_callers):
Set_Video_Mode, Load_Game, FUN_00684C30 (scenario-start init path, 0x00684xxx
ScenarioClass region — identity MEDIUM), Main_Game 0x0052D9A0. So scenario start and
session start go through Activate (and through One_Time/Init compound earlier), not
through ad-hoc list pokes.

Sidebar side switch (Allied/Soviet/Yuri chrome) is a SHP/surface reload
(SidebarClass__InitSurface reads ToolTipManager global 13× — re-registering tooltip
rects), not a Buttons-list rebuild; the gadget records persist. (content evidence:
xref list of 0x00887368; not further decompiled this session — confidence MEDIUM.)

---

## 4. Input consumption — the AI cascade and the UI→sim seam (VERIFIED-LIVE)

GScreenClass::Input ends with `this->vtbl[+0x28](key*, &xyCopy)`. Live dispatch chain
(each link verified by decompile or callee list):

```
MouseClass::AI      0x005BDDC0  cursor-shape animation timer (frame tables @ 0x0082D028
                                 stride 0x1C; sets WWMouse frame via [0x00887640]->vtbl[+4])
 └→ ScrollClass::AI 0x006922E0  (Ghidra label "DisplayClass__Dispatch" = DRIFT)
                                 calls FUN_00692F30 (edge-scroll service)
  └→ TabClass::AI   0x006D0680  (label "CommandBar_Dispatch") ★ button-ID consumption hub
   └→ SidebarClass::AI 0x006A7780 (label "SidebarClass__Action" = naming DRIFT; this is
                                 the chain AI/slot-10 override)
    └→ PowerClass::AI 0x0063FEA0 (label "PowerClass__AnimationTick"; power-bar calc)
     └→ RadarClass::AI 0x00653850 (label "Minimap_Chat_Dispatch"; radar mode/movie)
      └→ DisplayClass::AI 0x004A9700 (tactical input; sole callee = base)
       └→ GScreenClass::AI 0x004F4BB0 (sign-decay tick on +0x04/+0x08)
```
(cascade verified via decompile 0x005BDDC0, 0x006922E0, 0x006D0680, 0x006A7780 and
get_function_callees 0x0063FEA0 / 0x00653850 / 0x004A9700)

### 4.1 Button-ID protocol (ID | 0x8000, optional 0x4000)

ControlClass::Action posts `ID | 0x8000` as the Input result (DOC-INHERITED GADGET §4).
Verified consumption sites:

**TabClass::AI 0x006D0680** (verified via decompile):
- `0x80F0`: sidebar collapse — sound, ToolTip unregister loop (25 command-bar ids via
  FUN_006CFD40/FUN_00724730), Remove_A_Button(0xB0CCB0-record), Add_A_Button
  (0xB0CC40-record), byte +0x5544=0, Flag_To_Redraw(0).
- `0x80F1`: sidebar expand (gated by DAT_00A8B538==0) — symmetric.
- `0x80D6..0x80EE` (IDs 214..238): command-bar buttons. `(key & 0xFFFF7FFF) − 0xD6`
  compared against configured id slots (DAT_00B0CD24 etc.) → control-group ops
  (Team__AssignSelectedToGroup / ControlGroup__Recall / ControlGroup__CenterCamera),
  TypeSelect__Execute, waypoint/planning handlers (FUN_00731AF0, FUN_00731A30,
  FUN_00730F30, FUN_00730AF0, FUN_00730D60, FUN_00730EA0). These are LOCAL UI/selection
  commands, not networked sim events.
- `0xC0D6..0xC0EE` (0x4000 flag = second event type on same IDs): Team__ClearGroup.
- Also: per-frame tab flash timer (+0x5538/+0x5540 vs g_CurrentFrameCounter), radar-rect
  hover → mouse-shape call FUN_005BDC80, and **CreditsClass__AI(this+0x551C)** — the
  credits ticker ticks inside TabClass::AI.

**SidebarClass::AI 0x006A7780** (verified via decompile):
- `0x8065` repair-mode toggle → FUN_004AC8C0(-1); `0x8066` sell → FUN_004AC660(-1)
  (DisplayClass pending-mouse-mode setters — UI state, not sim).
- `0x80CB..0x80CE`: tab select → full cameo Remove/Add swap (§3.2) + sound.
- `(key & 0xFFFFBFFF) == (DAT_00B0B34C|0x8000)` (ID 201) page-down /
  `== (DAT_00B0B42C|0x8000)` (ID 200) page-up — the 0x4000-masked compare means both
  press flavors scroll.
- Per-frame even with no key: 4× StripClass__AI, repair-availability re-check vs
  g_PlayerPtr+0x2F0, 4× tab-flash FUN_0069E010 loop → dirty bits DAT_00B0B518 /
  DAT_00884B8F + Flag_To_Redraw(0).

### 4.2 Where UI ends and sim begins

- The AI cascade mutates ONLY UI state (selection, modes, scroll, flash, camera) —
  verified for Tab/Sidebar levels above. Production clicks (cameo SelectClass::Action →
  factory events) post networked EventClass commands (DOC-INHERITED
  SIDEBAR_SYSTEM/GADGET docs; not re-verified this session).
- **Queued-event execution point** (verified via `get_function_callers 0x0053B560`):
  Process_QueuedEvents 0x0053B560 is called from (a) Main_Tick's scenario-delay branch,
  (b) "LightningStorm__Process" 0x0053A6C0 — which is invoked from **LogicClass::Update
  0x0055AFB0** (the master sim tick; see §5). So in a normal tick, commands enter the
  sim inside LogicClass::Update, AFTER render (label "LightningStorm__Process" covering
  a queue-service path = suspected DRIFT; binding verified, identity of wrapper YELLOW).
- Therefore the seam for the Rust port: gadget/AI cascade = pure UI layer; the ONLY
  sim-facing artifacts are queued events consumed later in the same Main_Tick by the
  logic tick. `sim/` must never see gadget state — matches the #1 invariant.

---

## 5. One frame of Main_Tick 0x0055D360 (VERIFIED-LIVE) — corrected ordering

Verified via decompile_function 0x0055D360 (+ callee address list):

1. `g_GameActive == 0` → return 1. Set reentrancy byte DAT_00ABCD58 = 1.
2. While `g_GameRunning == 0`: offline {0,5} Sleep(500)+Process_NetworkMessages loop;
   network modes Sleep(10)+Process_NetworkMessages once.
3. Frame-budget setup (DAT_00887348/350; WOL mode-4 latency inflation via
   g_NetworkFrameBudget). Single-player speed override path.
4. Scenario-delay branch (`Scen+0x62C != 0`, offline only): Process_NetworkMessages →
   Network_ServiceLoop → **Process_QueuedEvents 0x0053B560** → TacticalClass::Update
   (g_Tactical->vtbl[+0x5C]) → RenderFrame_main → FUN_0055E160 → clear guard → return.
5. **Gameplay block — gated by `(DAT_00A8D5F8 & 2)==0 && g_GameState==0 &&
   g_GameRunning`:**
   a. GScreenClass::Input (call @ 0x0055D8AB) — gadget tree + AI cascade (§1, §4).
   b. Process_Command 0x0055DEE0 — keyboard hotkey processor (callees = Keyboard
      Check/Get 0x0054F000/0x0054F050 + handlers; identity MEDIUM).
   c. (debug overlay if DAT_00A8B8B4; mode-4 keepalive every 8th frame.)
   d. Map__Logic 0x004D2370 (tiny; 2× Get_CellClass — NOT the sim tick).
   e. **RenderFrame_main 0x004F4480 — render happens HERE.**
6. Record/playback service ((DAT_00A8D5F8 & 1)/(& 2)); playback re-renders separately.
7. FUN_00551A30 — single bubble-sort pass (vtbl+0xB8 comparator) on a vector.
8. **LogicClass::Update 0x0055AFB0 (label LogicClassPerTickUpdateLiveVector) — the
   master sim tick — UNCONDITIONAL once Main_Tick is entered** (mission hotkeys check,
   ion/chrono/storm timers, bridge-shroud recalc every 0x78 frames, Tiberium growth/
   spread, bombs, teams, disk lasers, lightning storm (→ Process_QueuedEvents), EMP,
   the Logic live-vector loop `obj->vtbl[+0x5C]()` for every registered object,
   TacticalClass::Update, factories, houses). (verified via decompile 0x0055AFB0)
9. Sound/radar service, Network_ServiceLoop, frame-time accounting.
10. If 4 pause bytes (DAT_00A83D49, DAT_00A8ECD0, DAT_008B41C0, DAT_00A83D48) all
    zero: `g_CurrentFrameCounter++`, then **FUN_0055E160** (§5.1), clear DAT_00ABCD58,
    return `g_GameActive == 0`.

**DRIFT vs GSCREEN_RTACTICAL §8:** the doc's order "Input → LogicClass::AI →
Map::Logic → RenderFrame_main" and invariant "Render happens AFTER Logic" are WRONG
for the object-sim tick: **RenderFrame_main runs BEFORE LogicClass::Update in the same
Main_Tick** — the rendered frame shows the PREVIOUS tick's sim state (1-tick visual
latency), and input is captured before both. Input-before-logic holds; render-after-
logic does not.

### 5.1 FUN_0055E160 = end-of-tick frame limiter (DRIFT vs GSCREEN doc)

Verified via decompile 0x0055E160. NOT a "non-gameplay frame wrapper": it is the
frame-budget sleep/throttle called at the END of every successful Main_Tick (and from
the scenario-delay branch):
- Offline {0,5}: sleep out the remaining radar-timer budget (FUN_004A4830 service in
  the loop).
- Network modes: busy-wait loop that services Network_ServiceLoop, and — when
  `g_GameState==0 && g_GameRunning` and >10 ms remain — runs **extra UI passes:
  GScreenClass::Input → Process_Command → TacticalClass::Update → RenderFrame_main**.
  So in LAN/WOL games the UI can sample input and re-render MULTIPLE times per sim
  tick while waiting for lockstep. This is the second of the 4 Input call sites.
- Tail: FPS accounting window (DAT_00ABCD40/48/4C, 60-tick reset).

### 5.2 RenderFrame_main 0x004F4480 (re-verified; corrected actor identities)

Order (verified via decompile):
1. Save surface-target global [0x00887314]; set = Composite 0x0088731C.
2. `[0x00887640]->vtbl[+0x40](Composite, 0)` — WWMouse cursor call (GSCREEN doc calls
   this "chain MouseClass::Draw predraw" — actor identity DRIFT: object is the 0x98-byte
   WWMouse, role = cursor erase/draw on target surface; exact slot role YELLOW).
3. Consume RedrawFlag (+0xC → 0).
4. If !FUN_0053BAE0(): TacticalClass_Draw(pass 0), (pass 1), then
   `this->vtbl[+0x40](RedrawFlag==2)` = **chain Draw_It = TabClass::Draw_It 0x006D0A20**
   (chrome BETWEEN tactical passes — sandwich confirmed), then TacticalClass_Draw(pass 2).
5. If sidebar-dirty DAT_00B0B519 && !editor: `[0x00887640]->vtbl[+0x40](g_SidebarSurface,
   1)`; clear flag.
6. **Gadget hook:** if Buttons head: `head->vtbl[+0x2C](0)` = Flag_List_To_Redraw walk
   = Draw_Me(0) (draw-if-dirty) over every registered gadget, every frame.
7. FUN_005D49A0 (message/chat overlay draw).
8. **Tooltip hook:** if [0x00887368] (ToolTipManager*): `->vtbl[+0xC](0)` (slot 3 draw).
9. Debug overlay; `[0x00887640]->vtbl[+0x3C](Composite, 0)` (cursor restore/show);
   `this->vtbl[+0x44]()` = Blit/present 0x004F45B0; restore [0x00887314].

---

## 6. Framework coexistence — in-game Framework-B dialog over Framework-A world

### 6.1 Modal pump re-verified live

FUN_00623120 (verified via decompile this session) matches
MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md exactly:
`Process_NetworkMessages 0x005D4D50` ALWAYS first; then if `g_GameMode∈{0,5}` or
DAT_00A8D60E or DAT_00A8DAB4 → `Network_ServiceLoop 0x0048D080` only, return 0; else
if `FUN_0055CBF0()` (returns reentrancy byte DAT_00ABCD58) == 0 → `Main_Tick()`,
return 1 iff Main_Tick returned nonzero.

In-game Options caller 0x004E1D00 (verified via decompile this session): result=-1 on
stack, hwnd = FUN_00622650(0), `SetWindowLongA(hwnd, 8, &result)`, FUN_00622800
(show), `while (result == -1) { if (FUN_00623120()==1) result=2; if (!g_GameActive)
FUN_00532100(); }`, result==1 → Apply 0x004E1DE0 + WriteToINI 0x005FAD10, teardown
FUN_00622720. (The `!g_GameActive → FUN_00532100` service call inside the loop is NOT
in the MODAL doc's §4 step list — minor addition.) Dialog-id choice (0xBBB vs 0xF5) not
visible in this decompile — DOC-INHERITED from MODAL doc assembly cites (YELLOW here).

### 6.2 Coexistence rules (derived from verified gates — the load-bearing matrix)

While an in-game Framework-B dialog (options 0xBBB, quit confirm, etc.) is pumping:

| Concern | Offline (mode 0/5) | LAN/WOL (3/4) |
|---|---|---|
| Gadget-tree Input (Buttons list) | **Never runs** — pump never calls Main_Tick, so GScreenClass::Input is unreachable | **Suppressed** — Main_Tick runs but gameplay block requires `g_GameState==0`; State_Machine case 5 holds g_GameState=5 (DOC-INHERITED MODAL §4) |
| AI cascade (scroll/tab/sidebar/credits tick) | Never runs (inside Input path) | Suppressed (same gate) |
| Tactical + chrome render (RenderFrame_main) | Never runs → battlefield frozen as last-rendered frame | Suppressed (same gate; also the 0x0055E160 bonus passes are g_GameState-gated) → stale frame while sim advances |
| Object sim (LogicClass::Update 0x0055AFB0) | Frozen (no Main_Tick) | **ADVANCES** (unconditional once Main_Tick entered) |
| Frame counter | Frozen | Advances (pause-byte gate only) |
| Win32 message routing (dialog keys, WM_PAINT, tooltips) | **Live** via Process_NetworkMessages → IsDialogMessageA/accelerators/hook/DispatchMessage (MODAL doc §3.1; pump head verified live) | Live (same) |
| Network service | Network_ServiceLoop direct | via Main_Tick |
| Reentrancy | n/a | DAT_00ABCD58 prevents pump→Main_Tick recursion when a tick is already active |

Single render exception: dialogs repaint via WM_PAINT through the OS, not through the
engine compositor — the tactical surface beneath is whatever the last RenderFrame_main
left (no recomposition).

---

## 7. Tooltip subsystem ownership (VERIFIED-LIVE + DOC-INHERITED)

- Manager instance pointer global: **0x00887368** (read in RenderFrame_main @
  0x004F4562 for the draw hook, and in Process_NetworkMessages @ 0x005D4DBC).
  (verified via get_xrefs_to 0x00887368)
- **Hover timing is owned by the Win32 message pump, not the gadget tree:**
  ToolTipManager::ProcessMessage 0x00724200's ONLY caller is Process_NetworkMessages
  0x005D4D50 (verified via get_function_callers 0x00724200). Per
  SIDEBAR_TIMING_AND_TOOLTIPS doc (DOC-INHERITED): WM_MOUSEMOVE restarts a real
  SetTimer (1000 ms delay / 10000 ms auto-hide); any button press kills the tip;
  ctor 0x00724000 holds defaults; HoveredTip at +0x04/+0x228.
- Rect registration sites (xref list of 0x00887368): SidebarClass__InitSurface (13
  reads — sidebar button rects re-registered on chrome (re)build),
  PowerClass__RegisterTooltip 0x006403A0±, FUN_00654320 (radar region),
  TabClass::Activate 0x006D04F0 (register/unregister by id via FUN_00724730 on
  expand/collapse — verified via decompile 0x006D04F0/0x006D0680), Main_Game,
  Set_View_Dimensions, scenario init fns 0x00683xxx-0x00686xxx.
- Consequence: tooltips keep functioning while a Framework-B dialog is pumping
  (messages still dispatched), and during gameplay they tick at OS-message cadence,
  decoupled from the 60 ms frame budget. The gadget tree only contributes DRAW (slot-3
  hook in RenderFrame_main) and rect registration; it never tracks hover time.

---

## 8. UNVERIFIED (YELLOW)

- FUN_00648350 / FUN_00648710 (two non-Main_Tick callers of GScreenClass::Input,
  0x0064xxxx radar region) — role unknown; not decompiled this session.
- WWMouse vtable slot roles +0x2C/+0x30 (= Get_Mouse_X/Y by consumption pattern) and
  +0x3C/+0x40 (cursor draw/restore by call sites): content inferred from arg/usage
  shape; the WWMouseClass function bodies were not decompiled this session.
- Process_Command 0x0055DEE0 = "keyboard command processor": callee shape only.
- "LightningStorm__Process" 0x0053A6C0 as the in-tick wrapper reaching
  Process_QueuedEvents — call edge verified, but the function body/label was not
  decompiled; the exact in-LogicClass::Update position of queue execution is unpinned.
- FUN_00684C30 = scenario-start initializer (region + call-pattern only).
- Dialog-id selection 0xBBB/0xF5 inside the Options path (MODAL doc cites assembly
  0x004E1D2A..47; this session's decompile shows it as FUN_00622650(0)).
- Map__Logic 0x004D2370's actual role (tiny; 2 cell lookups).
- FUN_0072F430 gate inside TabClass::Init_IO (YR sidebar-style check?) — not decoded.
- DAT_00A8B538 gate on 0x80F1 expand — identity unknown.

## 9. ORCHESTRATION CONTRACT — one frame of UI service (O1..O14)

O1. **Single chain object.** One static instance @ 0x0087F7E8 (GScreen→Map→Display→
    Radar→Power→Sidebar→Tab→Scroll→Mouse), final vtable 0x007E1964, built by CRT
    static init 0x0040D190 before WinMain; never reconstructed (only re-Init'd).
    [disassemble 0x0040D190; read_memory ×9 vtables]
O2. **Input stage.** Main_Tick calls GScreenClass::Input (chain slot 9) exactly once
    per gameplay tick @ 0x0055D8AB, gated by `(SpecialFlags&2)==0 && g_GameState==0
    && g_GameRunning`. Mouse x/y from WWMouse [0x00887640]; key from gadget list if
    Buttons≠0 (full 32-bit result, 0x8000|ID protocol), else keyboard Check/Get
    masked to 16 bits. [decompile 0x004F4320, 0x0055D360]
O3. **Gadget surface swap.** Gadget Input runs with draw-target global [0x00887314]
    temporarily set to HiddenSurface [0x0088730C]; restored immediately after.
    [disassemble 0x004F4320]
O4. **Pre-input dirty propagation.** If any gadget IsToRedraw, Input calls chain
    Flag_To_Redraw(0) → sets g_Tactical+0xD7D only (no full chrome repaint).
    [decompile 0x004F4320, 0x004F42F0]
O5. **AI cascade order is fixed:** Mouse(cursor anim) → Scroll(edge scroll) →
    Tab(button-ID hub + credits tick) → Sidebar(strips/tabs/scroll/repair-sell) →
    Power(bar anim) → Radar → Display(tactical) → GScreen(decay). Key pointer is
    shared down the chain; x/y are a copy. [§4 cites]
O6. **Button-ID consumption is layer-local:** 0xF0/0xF1 in TabClass; 0xD6..0xEE
    command bar in TabClass; 0x65/0x66, 0xC8/0xC9, 0xCB..0xCE in SidebarClass; all
    effects are UI-state only. Sim-affecting clicks become queued events only.
    [decompile 0x006D0680, 0x006A7780]
O7. **Buttons-list lifecycle:** initialized (objects/IDs/positions) by the Init_IO
    chain (Tab→Sidebar→Radar→Display→base-clear); REGISTERED dynamically at
    tail via Add_A_Button on activate/tab-switch/toggle; head cleared by One_Time/
    base Init_IO. Hit-test priority and draw order both = insertion order.
    [§2/§3 cites]
O8. **Rebuild events:** video-mode change (0x00560BF0) and load-game (0x0067E440)
    re-run Init_IO + TabClass::Activate(1) + InitSurface; scenario/session start runs
    Activate via 0x00684C30 / Main_Game. [get_function_callers 0x006D03A0/0x006D04F0]
O9. **Frame order (gameplay tick):** Input → Process_Command(hotkeys) → RenderFrame_
    main → LogicClass::Update(0x0055AFB0, incl. queued-event execution) → network
    service → frame-counter++ → frame-limiter 0x0055E160. **Render precedes the
    object-sim tick; the screen shows tick N−1 while input for tick N was already
    captured.** [decompile 0x0055D360, 0x0055AFB0]
O10. **Render composition:** cursor-prep → tactical pass0/pass1 → chain Draw_It
    (TabClass::Draw_It 0x006D0A20 → sidebar chrome) → tactical pass2 → conditional
    sidebar cursor/blit → gadget Draw_Me(0) walk → chat overlay → tooltip draw
    ([0x00887368] slot 3) → cursor restore → present (slot 17). [decompile 0x004F4480]
O11. **Network bonus UI passes:** in modes 3/4 the frame limiter may run extra
    {Input, Process_Command, TacticalClass::Update, RenderFrame_main} sequences
    while waiting out the lockstep budget — UI responsiveness can exceed sim rate.
    [decompile 0x0055E160]
O12. **Framework-B coexistence:** while a shell dialog pumps via 0x00623120 —
    offline: gadget input, render, sim, frame counter ALL frozen; Win32 messages
    (dialog keys/paint/tooltips) live. Network: sim + frame counter advance,
    gadget input + render stay suppressed via the g_GameState gate; reentrancy byte
    DAT_00ABCD58 forbids nested Main_Tick. [decompile 0x00623120, 0x004E1D00,
    0x0055D360; MODAL doc]
O13. **Tooltip ownership:** hover timing + show/hide = ToolTipManager driven ONLY by
    the Win32 message pump (ProcessMessage ← Process_NetworkMessages); engine frame
    loop only draws it. Tooltip cadence is wall-clock ms, not frames.
    [get_function_callers 0x00724200]
O14. **Sim isolation seam:** nothing below the queued-event boundary may read gadget/
    chain state; gamemd's own structure already respects this (AI cascade mutates UI
    globals only; sim consumes events inside LogicClass::Update). Rust `sim/` keeps
    the same seam. [§4.2]

## 10. Doc-drift ledger (for /verify-doc follow-up on GSCREEN_RTACTICAL_GHIDRA_REPORT.md)

1. §1: "g_DisplayChain = MouseClass* at 0x00887640" — WRONG: 0x00887640 = WWMouseClass*
   (0x98-byte cursor handler, created in WinMain/Set_Video_Mode); the chain is a static
   instance at 0x0087F7E8. [decompile 0x00560BF0; disassemble 0x0040D190]
2. §3 idx 14 "DAT_00a8ef54 = 0 (same body as idx 8)" — WRONG: 0x004F42F0 =
   Flag_To_Redraw(level) (g_Tactical+0xD7D, RedrawFlag). [decompile 0x004F42F0]
3. §3 idx 5/7/8 role names (Init_Clear/Set_Redraw_Full/Clear_Gadget_Root) —
   re-mapped: 5 = One_Time (Buttons=0), 7 = Init_Clear (RedrawFlag=2),
   8 = Init_IO (Buttons=0 base). Original-source-shape consistent; body-verified.
4. §5/§7: "MouseClass overrides slot 11 with Get_Mouse_X… 0x005BDDC0 =
   Process_Input" — slots 11/12/13 are Is_A_Button/Add_A_Button/Remove_A_Button on the
   CHAIN; the mouse-XY getters are WWMouse slots (+0x2C/+0x30) on 0x00887640.
   0x005BDDC0 = MouseClass::AI (slot 10). [read_memory 0x007E1964; decompiles]
5. §6/§10: "MouseClass::Draw 0x006D0A20" — vtable ownership says TabClass::Draw_It
   (introduced in 0x007EDFB4, inherited by Scroll+Mouse). Dispatch identical.
6. §8: "Render happens AFTER Logic" + "LogicClass::AI before Map::Logic" — WRONG
   per §5 above; object sim (0x0055AFB0) runs after RenderFrame_main.
7. §8: "FUN_0055E160 = non-gameplay frame wrapper" — it is the end-of-tick frame
   limiter with embedded network-mode UI service. [decompile 0x0055E160]
8. Ghidra label drift recorded: "Hide_Cameo_Slots" 0x004F4450 = Remove_A_Button;
   "DisplayClass__Dispatch" 0x006922E0 = ScrollClass::AI; "CommandBar_Dispatch"
   0x006D0680 = TabClass::AI; "SidebarClass__Action" 0x006A7780 = SidebarClass::AI;
   "Minimap_Chat_Dispatch" 0x00653850 = RadarClass::AI; "PowerClass__AnimationTick"
   0x0063FEA0 = PowerClass::AI; "INoticeSink__Constructor" @ 0x006CFE86-area =
   TabClass ctor; "SidebarClass__Init" 0x006A5310 = SidebarClass::Init_IO;
   "GlobalDisplaySingleton_StaticInit" 0x0040D190 writes 0x0087F7E8 (correct concept,
   object addr now pinned).

**Status: COMPLETE for lane scope.** Confidence axes on the two highest-stakes claims:
- Frame order O9: content HIGH (decompile), identity HIGH (Main_Tick entry verified by
  callers incl. modal pump), binding HIGH (call sites read from body).
- Chain identity table §2: content HIGH (vtable bytes), identity HIGH (ctor write sites
  + region clustering + inheritance direction proof), binding HIGH (static ctor →
  0x0087F7E8).
