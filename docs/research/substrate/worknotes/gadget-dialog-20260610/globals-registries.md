---
title: "UI Substrate Study — Lane: Globals, Registries, Static Tables, COM/vtable Surface (both frameworks)"
date: 2026-06-10
status: worknotes (live-verified this session unless marked DOC-INHERITED / INFERRED)
scope: cross-cutting singleton-state ledger for Framework A (GadgetClass/LinkClass) and Framework B (Win32 RT_DIALOG shell)
session: gadget-dialog-20260610 multi-agent study
---

# Globals / Registries / Static Tables / COM Surface — State Ledger

Method: every anchor address from prior docs was re-verified live (decompile or raw
`read_memory` byte decode) before use; writer/reader census via `get_bulk_xrefs`;
function identities from bodies, not labels. Confidence axes given for load-bearing
role claims: content / identity / binding.

Evidence tags: **[V]** = VERIFIED-LIVE this session (cited call), **[D]** = DOC-INHERITED
(cited doc), **[I]** = INFERRED. Default verdict for any behavioral difference: DRIFT.

---

## 1. FRAMEWORK A — GadgetClass singleton state

All four focus globals verified from the body of `GadgetClass__Input @ 0x004E1640`
(verified via decompile_function 0x004E1640) and the writer census
(get_bulk_xrefs 0x008B3E88/8C/90/94).

### 1.1 State ledger — Framework A

| Global | Addr | Type | Role | Writers (census, collapsed) | Reset point | Rust-service owner |
|---|---|---|---|---|---|---|
| g_StickyFocus | `0x008B3E88` | `GadgetClass*` | total input capture (mouse+kbd) until release | set/cleared by `Sticky_Process 0x004E1970` (press 0x11 sets when `IsSticky`, release 0x44 clears if owner — byte-decoded, see §3.2); cleared by `Input 0x004E1640` on list change (`0x004e1667`); cleared by both dtors `0x004E1390` (`004e13bc`) / `0x004E1A60` (`004e1a8c`) if `this` owns it | list-head change in Input; gadget destruction | `ui::gadget::FocusState` |
| g_CurrentGadgetList | `0x008B3E8C` | `GadgetClass*` | list head of last Input() call; identity check resets focus | `Input` (`004e1659`); zeroed by `Clear_Attached_List 0x00488690` (raw bytes `C7 05 8C 3E 8B 00 00000000 C3` = `mov [0x008B3E8C],0; ret`, verified via read_memory 0x00488690); cleared by dtors if `this == head` (`004e13cd`, `004e1a9a`) | vtable slot +0x38 call; dtor | `ui::gadget::FocusState` |
| g_KeyboardFocus | `0x008B3E90` | `GadgetClass*` | keyboard-only focus (EditClass) | `Set_Focus 0x004E19A0` (clears old via vtbl+0x48/+0x54 then sets; sets flag bit 0x100 on gadget) [V decompile 0x004E19A0]; `Clear_Focus 0x004E19D0` (only if `this` owns; clears bit 0x100) [V decompile 0x004E19D0]; `Input` reset on list change (`004e166c`); dtors (`004e13ac/13d7`, `004e1a7f/1aa4`) | list change; destruction; Clear_Focus | `ui::gadget::FocusState` |
| g_HoveredGadget | `0x008B3E94` | `GadgetClass*` | last Hit_Test result; drives Mouse_Enter (+0x70) / Mouse_Leave (+0x74) before dispatch | ONLY `Input` (`004e16f0` write, `004e16d9` read) — fully closed | every Input call | `ui::gadget::FocusState` |
| GScreen Buttons head | `0x00A8EF54` | `GadgetClass*` | THE in-game gadget list (sidebar buttons, tabs, radar etc.) | clears: `FUN_004f42a0` and `FUN_004f42e0` — both are byte-identical `mov [0x00A8EF54],0; ret` stubs (read_memory 0x004F42A0 / 0x004F42E0); set: add-helper in unbounded region `~0x004F43F0` (write `004f443c`); remove-helper `0x004F4450` writes head = `gadget->vtbl[+0x24]()` (Unlink → new head) [V decompile 0x004F4450] | scenario-lifecycle Init_Clear chain (§7) | `ui::gadget::GadgetTree` (in-game) |
| WWKeyboard singleton ptr | `0x0087F770` | `WWKeyboardClass*` | keyboard/mouse event queue object; `+0`/`+4` read as queued-event mouse X/Y in `Input`; methods `Check 0x0054F000`, `Get 0x0054F050`, `Down 0x0054F5C0` are `__thiscall` on it (ECX load at `004f4394` in GScreenClass__Input disasm) [V disassemble 0x004F4320] | written at init `006bc2ae/b6`, `006bea78` (video/window init region); ~150 readers across both frameworks | app lifetime | `input::Keyboard` (shared service) |
| modifier VK pairs | `0x00A8EBF8/FC`, `0x00A8EC00/04`, `0x00A8EC08/0C` | 6× u32 | configurable virtual-key code pairs polled via `Keyboard::Down` inside `Input` (left/right variants of three modifiers) [I — pairing/role inferred from poll pattern in 0x004E1640] | single writer each: `0x005fa38b/9f/b3` (one init fn `~0x005FA380`) | once at init | `input::Keyboard` |
| Hit_Test seed dims | `0x007F5BE8`=1024, `0x007F5BF4`=768 | const i32 | initial best-area seed in `Hit_Test 0x004E15A0` = **constant 1024×768 = 786432 px²**, NOT live screen size — census shows **zero writers** for both (get_bulk_xrefs 0x007F5BE8 / 0x007F5BF4; values via read_memory 0x007F5BE0) | n/a (ROM-like) | n/a | `ui::gadget` const |

**DRIFT-correction to GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md §3:** the doc calls
`DAT_007F5BE8/DAT_007F5BF4` "screen width, height". They are the static max-res
constants 1024/768 (full block dump §5.1). Behavioral contract: a gadget whose
`W*H > 786432` can never win a hit-test even when it contains the point. At
800×600 the seed is still 786432.

**Resolved GADGET doc §11 open question — top-level caller of GadgetClass::Input:**
`GScreenClass__Input @ 0x004F4320` calls `Buttons->vtbl[+0x28]` (= GadgetClass::Input)
on `[0x00A8EF54]` at `004f4382`, with **`g_PrimarySurface (0x00887314)` swapped to the
hidden/composite surface `[0x0088730C]` for the duration of the call and restored
after** (`004f4367–004f438b`) [V disassemble_function 0x004F4320]. So gadget drawing
(which happens inside Input — Draw_Me per gadget) targets the hidden surface. If the
list is empty it falls back to raw `Keyboard::Check/Get`. Afterwards it dispatches
`this->vtbl[+0x28]` on the GScreenClass object itself with (key, mouse-pt).
Confidence: content HIGH (disasm), identity HIGH (named fn, body matches GScreen
semantics), binding HIGH (reached from GScreen tick chain per GSCREEN_RTACTICAL doc
[D]).

### 1.2 Other Framework A globals touched

| Global | Addr | Role | Evidence |
|---|---|---|---|
| g_DisplayChain | `0x00887640` | display/surface-chain singleton; vtbl `+0x2C/+0x30` = current mouse X/Y getters used by `Input` and `GScreenClass::Input` | [V] disasm 0x004F4320 (`MOV ECX,[0x00887640]; CALL [EAX+0x2c]`); writers at video init `006bdf25/006be3e3`, save/load `00560ccb..`, `006becf0` (census) |
| g_PrimarySurface | `0x00887314` | current primary draw-target pointer; swapped around gadget draw (GScreen Input `004f4374/438b` AND a second pair `004f4492/459d` in the GScreen render path), also swapped by sidebar (`006a6c74/006a70c5`) and movie player (`0073b55b..`) — 24 writers total (census) | [V] get_bulk_xrefs 0x00887314 |
| HiddenSurface | `0x0088730C` | composite/hidden surface; gadget-draw target during Input swap; heavily read in-game (sidebar `0068xxxx`, radar `005bxxxx`, loading `004b6xxx-004b9xxx`) | [V] census; scope-corrects SHELL doc §2.3's "(Bink only)" parenthetical — that holds for the shell context only |
| AlternateSurface | `0x00887310` | shell composition flip target; in-game readers too (sidebar `0068fxxx`, tactical `0069xxxx`) | [V] census; writers at surface init `0053400d/005343fe`, `00560e26`, focus-restore `006bb923` |
| PrimarySurface (real) | `0x00887308` | true front surface; writers only at video init `006bdca7`, `00560cf6/d32/f18`, `006bb97b` | [V] census |

---

## 2. FRAMEWORK B — shell dialog singleton state

### 2.1 The two dialog registries (re-verified)

**LIFO display/focus stack** — verified via decompile_function 0x00622650 (factory)
and 0x00622720 (teardown):

| Global | Addr | Type | Role |
|---|---|---|---|
| stack HWND column | `0x00B72D28` | `{HWND,id}[ ]` stride 8 | entry i at `0x00B72D28 + i*8` |
| stack id column | `0x00B72D2C` | u32 | dialog template id |
| depth | `0x00B72F50` | i32 | factory pre-increments (slot zeroed first, filled after CreateDialogIndirectParamA success; decremented on failure) |
| top mirror HWND | `0x00B72F44` | HWND | written by factory, teardown (new top or 0), and WOL factory |
| top mirror id | `0x00B72F48` | u32 | same |

Teardown `0x00622720`: slide-out (`FUN_00608070`) → `DestroyWindow` → find entry →
`memmove`-compact (`FUN_007ca090`) → zero vacated tail slot → decrement → if stack
non-empty `SetForegroundWindow+SetFocus` to new top, else to `g_hWnd`. Teardown does
NOT touch the keyboard-routing vector (that prunes at WM_DESTROY via `FUN_005d4ed0`).

**Consumers beyond factory/teardown (census get_bulk_xrefs 0x00B72D28/0x00B72F50):**
the `0x00775700–0x00775Bxx` WOL family:
- `FUN_00775700` — **second (WOL-side) dialog factory** [V decompile]: same stack +
  depth + `FUN_005d4e70` routing registration, plus `EnumChildWindows(FUN_00775bc0)`
  rescale pass and `DAT_00A8ED8C += 1`; optional `ShowWindow`; sets top mirrors.
- `FUN_00775940` — find-HWND-by-dialog-id, scanning the stack **top-down** (latest
  instance of an id wins) [V decompile 0x00775940].
- `FUN_00775B10` — returns `DAT_00B72F44` (topmost-HWND accessor) [V decompile].
- teardown-like and compaction variants at `0x007757E0..0x007758xx` (same stack
  arithmetic; not individually decompiled — census only).
- `0x004c9126` reads top-mirror HWND (load/save dialog region).

**Keyboard-routing registry** — NOT a bare array: it is a static Westwood
DynamicVectorClass-shaped object at **`0x00ABFC90`** [V decompile_function
0x005D4E70 / 0x005D4ED0]:

| Field | Addr | Meaning |
|---|---|---|
| vtable | `0x00ABFC90` | vtbl `+0x08` = grow/resize, `+0x10` = index-of (used by unregister) |
| Items (HWND*) | `0x00ABFC94` | heap array |
| Capacity | `0x00ABFC98` | |
| growable flag | `0x00ABFC9D` | byte |
| ActiveCount | `0x00ABFCA0` | |
| GrowthStep | `0x00ABFCA4` | |

`FUN_005d4e70` = append (grows via vtbl+8 when full); `FUN_005d4ed0` = find via
vtbl+0x10 then compact-shift. Constructed/initialized by code at `0x005d4c40..`
(writes `005d4c57..005d4cb7`, census). `Process_NetworkMessages 0x005D4D50`
[V decompile] walks `Items[0..ActiveCount)` calling `IsDialogMessageA` per HWND in
**registration order** before generic dispatch — confirms contract C3 of the SHELL doc.

**NEW — third registry: accelerator tables.** `Process_NetworkMessages` also walks
`{HACCEL, HWND}` pairs at base **`0x00ABFCBC`** (stride 8), count **`0x00ABFCC8`**,
calling `TranslateAcceleratorA` after the IsDialogMessage pass; plus an optional
message-filter callback pointer **`0x00ABFD34`** consulted before
Translate/DispatchMessage [V decompile 0x005D4D50]. Writers for the accel registry
in the same `0x005d4cxx` init region (census).

### 2.2 Owner-draw runtime registries (HWND-keyed hashtables)

Common shape, fully exposed by `FUN_0060cf00` / `FUN_00609e20` / `FUN_006040b0`
[V decompiles]: bucket index = `hashfn() & ((1 << log2buckets) - 1)`; bucket head
in table; chain via record `[0x81*4 = +0x204]`.

| Table | Base | Companion fields (record table only, verified) | Writers (census) | Readers |
|---|---|---|---|---|
| HWND → paint proc | `0x00AC18C0` | shape inferred-by-analogy [I] | insert/remove in subclass complex: `0060f004`, `0060f0fe`; second pair `006008f3`, `006009c9` (unbounded region `0x00600700–0x00600C60`, no Ghidra fn boundaries) | lookup inside `FUN_0060f9a0` body (`0060ff32/ff89` — body spans 0060f9a0–00610350), shared wndproc `00610d1d`, scrollbar `0061c2xx` |
| HWND → original WndProc | `0x00AC1B48` | [I] | `0060e9fc`, `0060eaf2`; `00600723`, `006007f9` | every owner-draw proc calls through it (~40 read sites, census) |
| HWND → 0x208 record | `0x00AC1B00` | `0x00AC1B04` = table-exists/nonempty flag; `0x00AC1B0C` = log2 bucket count; `0x00AC1B18` = hash-fn pointer [V decompile 0x0060CF00] | `0060ed52`; `00600ac3`, `00600bdd` (same unbounded region) | ~150 read sites: whole `0x0060xxxx` owner-draw complex, common DLGPROC `00622beb`, WM_PAINT composer `00621ebd`, DialogControl ctor `00624178` |
| z-order/topmost HWND array | `0x00AC1DE8` | — | `00610edf`, `00611527`, `006117af` | `0x00611xxx` paint-order helpers, `00612910` |
| paint-depth counter | `0x00AC48DC` | — | `006111bf`, `006125fb` | `00611f1f`, `006122a3`, `006125e2/f5` |
| one-time color/PCX preload guard | `0x00AC48D4` | — | `0060fb57` (inside `FUN_0060f9a0`) | `0060fa1e` (same fn) — strictly subclass-setup-local [V] |

The duplicate insert/remove pairs in `0x006007xx–0x00600Cxx` are in code Ghidra has
not bounded; their existence is census-verified, their caller (suspected: a second
subclass/teardown path) is **unattributed — see YELLOW**.

### 2.3 Theme/text/dim/misc globals

| Global | Addr | Value | Role | Writers |
|---|---|---|---|---|
| default shell text color | `0x00AC18A4` | 0xFFFF (yellow) [D SHELL §2.3] | enabled text | single writer `0060fa3f` inside `FUN_0060f9a0` one-time init [V census] |
| disabled text color | `0x00AC1CB4` | #9F0000 [D] | disabled Static/Checkbox/Trackbar | single writer `0060fa14` (same init) [V census] |
| static dim block | `0x007F5BE0..0x007F5BFC` | **640, 800, 1024, 480, 600, 768, 168, 32** | min/std/max W; min/std H; max H; sidebar inset; tab(?) | zero writers (constants) [V read_memory 0x007F5BE0×32] |
| pump activity counter | `0x00AA0430` | — | incremented once per pump-entry by every pump variant (`00432cdd`, `005d4d77` ProcessNetMsgs, `00759a3f`, `00775a7f`, `0077f925`, `005dc459`); read by PeekMessage wrappers `0053e7ef/9ac/bb0` | [V census] |
| app HINSTANCE | `0x00B732F0` | — | CreateDialog instance arg | writer `006bb9cc` (window init) [V census] |
| open-shell-dialog counter | `0x00A8ED8C` | i32 | incremented by WOL factory `0077577c/586a` AND common DLGPROC sites `00622b9b/00622eed`; **read by GScreen flip region `004f4b5a` and sidebar `006a6a00`** — a cross-framework coupling: in-game code checks whether shell dialogs are open | [V census] role label [I] |
| WOL rescale ref-dims cache | `0x00B72F54/58` | lazy | client size of reference template `0xC6`, measured once by instantiating+destroying it | single writer `00775cc9` in `FUN_00775bc0` [V decompile] |
| WOL target design dims | `0x00849DE0/E4` | **300, 163** | numerator of WOL child rescale (`MulDiv`-style by hand) | zero writers [V read_memory 0x00849DE0] |

**DRIFT-correction to seed/anchor list:** `FUN_00775bc0` is NOT a WOL subclass pass —
it is a **per-child resolution rescale** (`GetClassNameA`; ComboBox gets
`CB_GETDROPPEDCONTROLRECT 0x152` sizing; scale = rect × `[0x00849DE0/E4]` ÷ cached
template-0xC6 client dims; `MoveWindow`) [V decompile 0x00775BC0]. WOL = live code,
dormant service (servers offline) — enumerate, do not implement as default.

### 2.4 ToolTip singleton — shared by BOTH frameworks

**`0x00887368` = global ToolTipManager instance pointer.** Identified live:
`Process_NetworkMessages` does `if ([0x00887368]) FUN_00724200(&msg)` with the global
in ECX; `FUN_00724200` [V decompile] is `__thiscall(this, MSG*)`:
- timer id `0x54544950` = ASCII "TTIP";
- WM_MOUSEMOVE (0x200): if delay `this[+0x228]` set and suppression byte
  `DAT_00A8F7D8 == 0` → `SetTimer(hwnd, 'TTIP', delay)`; else immediate region scan;
- WM_TIMER 'TTIP': `GetCursorPos+ScreenToClient`, walk rect array `this[+0x238]`
  (count `this[+0x244]`), **inclusive-both-edges** point test
  (`x <= px <= x+w`, `y <= py <= y+h`) — NOTE: different from gadget hit-test's
  half-open rect (parity nugget);
- match → `this[+4] = region`, `FUN_00724ad0()` (show), re-arm with `this[+0x230]`;
- button msgs (0x201/202/204/205/207/208) kill timer and hide via vtbl `+0x08`.
- enable gate: byte `this[+0x0C] == 1`.

Census (get_bulk_xrefs 0x00887368): ~70 readers across **both** frameworks —
in-game CommandBar `006d0xxx`, Sidebar `006a7xxx/006abxxx/006acxxx`, Tab `00654xxx`,
options dialog `004e1f88`, Main_Game `0052daf9`, shell `0055d0xx` — these
register/unregister tooltip regions on the shared manager. Writers (construction)
at `007777b8/c3/00777803` — unbounded code region, see YELLOW.
Suppression byte `0x00A8F7D8`: writer `00537efa`, readers `00724251` (the hook),
`004ae576`, `00537ef0` [V census].

### 2.5 Pump body globals (sim-visible)

`FUN_00623120` [V decompile + disassemble]:
```
Process_NetworkMessages();           // 0x005D4D50
if (g_GameMode[0x00A8B238] in {0,5} || byte[0x00A8D60E] || dword[0x00A8DAB4])
    Network_ServiceLoop();           // 0x0048D080
else if (!FUN_0055cbf0())
    if (Main_Tick())                 // 0x0055D360
        return 1;
return 0;
```
- `g_GameMode @ 0x00A8B238` (745 refs; writers clustered in Main_Game `0052dd61..`,
  menu procs, WOL `0077b5xx`) [V census].
- `0x00A8D60E` byte — gates pump to service-only path; writers in Main_Game/options
  (`0052e1a5/27a`, `0052f5f6`), radar/net regions (`005b...`, `005dc/5dd...`).
  Role label **[I]** (session/connection state); precise meaning unverified.
- `0x00A8DAB4` dword — same gate; writers in movie/score regions (`0064xxxx`,
  `00683xxx`, `00695xxx`, `00785xxx`); read by slide helpers `006081f4/0060d3f5`.
  Suspected active-movie handle/flag **[I]**.

---

## 3. Static tables & dumps

### 3.1 vtables (read_memory dumps)

**GadgetClass vtable @ 0x007E92BC** (140 bytes read; verified via read_memory
0x007E92BC): slots 0..32 match GADGET doc §8 exactly (0: `0x004E1A60` deleting dtor;
10 (+0x28): `0x004E1640` Input; 14 (+0x38): `0x00488690`; 18 (+0x48): `0x004E1960`;
20/21/22: `0x004E19A0/19D0/19F0`; 27 (+0x6C): `0x004E1550`; 31 (+0x7C): `0x004E1530`;
32 (+0x80): `0x004E13F0`).

**DRIFT-correction to GADGET doc §8 (slots 33/34):** the vtable **ends at +0x80**
(33 slots). The next dwords are NOT GadgetClass slots: `0x007E9340 = 0x00800AE0`
is the RTTI/COL pointer of the next vtable, and **`0x007E9344` is the LinkClass
vtable base** — referenced as DATA from `LinkClass__Constructor` at `00556561` and
`005565a5` (verified via get_xrefs_to 0x007E9344); its slot 0 = `0x004E1AD0`
(deleting dtor). The doc's "+0x84 likely terminator / +0x88 LinkClass helper" is a
misread of neighbor-vtable bytes.

**ControlClass vtable @ 0x007E528C** (160 bytes; read_memory 0x007E528C): slot 0
`0x0048E660`; overrides confirmed — +0x44 `0x0048E610` Get_ID, +0x4C `0x0048E650`,
+0x6C `0x0048E620` Draw_Me, +0x7C `0x0048E5A0` Action, +0x80 inherited `0x004E13F0`,
**+0x84 `0x0048E600` Set_Peer = last slot (34 slots)**. Next dword `0x007FD370` is
the following vtable's COL.

**ListClass vtable @ 0x007ED10C** (head 16 bytes; read_memory): slot 0 `0x00558060`
(deleting dtor — matches doc), slots 1/2 `0x004E14A0/0x004E14B0` (Get_Next/Prev
thunks), slot 3 `0x00557E10`. Other class vtables (Toggle `0x007E8118`, ShapeButton
`0x007E8088`, DropList `0x007E7FCC`, CheckList `0x007E4F84`, ColorList `0x007E5054`,
Edit `0x007E81A4`) **[D]** GADGET doc — not re-dumped this session.

### 3.2 Sticky_Process @ 0x004E1970 (raw-byte decode — no Ghidra fn boundary)

```
8A 41 1D        mov al,[ecx+0x1D]       ; IsSticky
84 C0 / 8A 44 24 04 / 74 0C            ; load flags byte, skip if !IsSticky
A8 11 / 74 08   test al,0x11            ; LEFT|RIGHT PRESS
89 0D 88 3E 8B 00                       ; g_StickyFocus = this
EB 08 / 39 0D 88 3E 8B 00 / 75 0E      ; else: if (g_StickyFocus != this) ret
A8 44 / 74 0A   test al,0x44            ; LEFT|RIGHT RELEASE
C7 05 88 3E 8B 00 00000000             ; g_StickyFocus = 0
C2 04 00        ret 4
```
(verified via read_memory 0x004E1970 ×48). Capture is acquired on PRESS only when
`IsSticky`, released on RELEASE only by the owner. This is the vtbl+0x78 slot called
by base `Action 0x004E1530`.

### 3.3 Background "table" `FUN_0060CF00` [V decompile]

Not a data table — a switch on record dialog-id (record int-index `0x1C` = bucket
+0x70) writing `(convert, smallSHP, largeSHP)` = record idx `0x1E/0x39/0x3A`
(bucket `+0x78/+0xE4/+0xE8`; data-root `+0x74/+0xE0/+0xE4` — matches SHELL doc §2.4).
Asset-pointer pool in `.data 0x00B0F9E8–0x00B0FB54`: id 0x94 → `DAT_00b0fa6c`(×2);
0x108 → `g_MinimapMovie_SHP`; 0xBC6/0xBC7/0x103 → `g_RadarFrameOpen_SHP`;
0x6B → `DAT_00b0fb50`+`DAT_00b0fab8`; 0x102/0xBC/0xBD/0xC2/0xC9 →
`DAT_00b0fb50`+`DAT_00b0fa18`; 0x113/0x114/0x10E|0x11C/0x10F|0x11D/0xE6|0xF3|0xF4/
0xE7/0x116/0x117/0x112/0x2BC/0xD6 each have rows; **default row** (any other id) →
`FUN_0072e280` + `DAT_00b0fb50`+`DAT_00b0fa04`. WOL ids (0xE6/0xF3/0x10F/0x113…)
remain dormant-but-present [D SHELL §3].

### 3.4 Reposition include predicate `FUN_00608CD0` [V decompile, full dump]

`(parent dialog-id from record, GetDlgCtrlID(child)) → bool`. Shared rows: ctrl
`0x694` (title) allowed for ~45 ids; ctrl `0x71C` for ~30 ids; ctrl `0x468` for
{0xBC,0xBD,0x102,0xC2,0xC9,0xBC6,0x105,0x6B,0x113}. Per-dialog rows (selection):
0xE2 → {0x686,0x578,0x55C,0x683,0x55F,0x684}; 0x100 → {0x689,0x688,0x579};
0x101 → {0x68E,0x68D,0x68F}; 0x102 → {0x6EC,0x5AA,0x5A8,0x617,(0x5C2)};
0x129 → {0x745}; 0x94 → {0x40E}; modal families 0xE6/0xF3/0xF4/0x112/0x122/0xFE/
0xD7 → {1}; 0xE7 → {1,0x576}; full dump in session transcript. This is the
allow-list consulted per child during the re-anchor pass; the dialog-level
include-test remains `FUN_0060C540` [D SHELL].

### 3.5 Control-id → STT tooltip-key map `FUN_006040B0` [V decompile, 51KB]

Same record-hashtable prologue → giant `(dialog-id, ctrl-id) → char* "STT:*"`
switch returning pointers to `.data` strings (`0x00833xxx–0x00835xxx`), e.g.
0xE2: 0x683→`STT:MainButtonSinglePlayer`(0x00835784), 0x684→`STT:MainButtonWWOnline`,
0x578→`STT:MainButtonNetwork`, 0x686→`STT:MainButtonMovies`, 0x55C→`STT:MainButtonOptions`,
0x3EE→`STT:MainButtonExitGamemd`, 0x55F→`STT:MainButtonYuriWebSite`;
0x100: 0x688→`STT:SingleButtonNewCampaign`, 0x689→`STT:SingleButtonLoadSavedGame`,
0x579→`STT:SingleButtonSkirmish`, 0x686→`STT:SingleButtonBack`. (Read first ~2KB +
symbol census of the 51KB decompile; the rest is the same pattern per dialog id —
full output saved in session tool-results.)

### 3.6 Per-(dialog,ctrl) predicate `FUN_00609E20` [V decompile, full dump]

Anchor list called this "allow-list"; it is a second `(dialog-id, ctrl-id) → bool`
predicate over OK/Cancel-like ids (0x120 → {0x5AE,2}; 0x121 → {0x5AE,0x5AF,2};
0xCE → {0x5AE}; 0x10D → {0x5C1,0x5C0}; 0xEA → {0x640..0x647}; 0xC3/0xE1/0xFC →
{1,2}; 0x126 → {6,7}; …). Exact consumer semantics **unattributed** (see YELLOW);
do not wire as the slide allow-list without tracing callers.

### 3.7 Click-sound state **[D]**

No standalone click-sound globals: shell button sounds are RulesClass fields —
`GUIMainButtonSound` (+0x188, MenuClick), `GenericClick` (+0x70C), combo open/close
(+0x1A4/…) — played via `VocClass__PlayAtPos 0x00750920` from inside the owner-draw
procs (button call sites `0x00613759..0x00613771`). Source:
`SHELL_UI_SOUND_PLAYBACK_PLUMBING_GHIDRA_REPORT.md`,
`skirmish-ui/SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md` (both
ghidra/verified). Rust owner: `audio` service keyed off rules, invoked from shell
controller — never from sim.

---

## 4. COM / vtable surface

1. **Framework B is wndproc-dispatched, not vtable-dispatched** — re-confirmed; the
   only vtable indirections seen live this session are the DSurface blit/fill slots
   (render backend) and the DynamicVectorClass vtbl (+0x08 grow, +0x10 index-of) on
   the routing registry [V decompile 0x005D4E70/0x005D4ED0].
2. **Genuine COM `QueryInterface` adjacent to GScreen:** `FUN_004F4240`
   [V decompile] is a textbook QI — compares a 16-byte IID against two static GUIDs
   `DAT_007F7C90` and `DAT_007EA6E8`, returns `0x80004003`/`0x80004002` (E_POINTER/
   E_NOINTERFACE), AddRef via vtbl+4. Which object's vtable carries it is
   **unattributed** (YELLOW). It does not touch any ledger global.
3. **Dead/dormant gadget slots:** base `Mouse_Enter +0x70` / `Mouse_Leave +0x74` are
   RET stubs (doc [D], consistent with Input body [V]); `Clear_Attached_List +0x38`
   has no symbolic callers (vtable-only dispatch; get_function_callers 0x00488690
   returns none — the address has no fn boundary). `Peer_Callback +0x4C` base is a
   RET 0xC stub [D].
4. **LinkClass vtable @ 0x007E9344** (slot 0 `0x004E1AD0`) — newly pinned, see §3.1.

---

## 5. Allocation / ownership / lifecycle resets

| Event | What happens to ledger state | Evidence |
|---|---|---|
| shell dialog open | factory `0x00622650`: stack slot zeroed → Create → slot filled → `FUN_005d4e70` routing append → top mirrors set. WOL factory `0x00775700` additionally bumps `0x00A8ED8C` and rescales children | [V] decompiles |
| WM_INITDIALOG | `FUN_0060f9a0` (body 0060f9a0–00610350): one-time color init (`0x00AC18A4/0x00AC1CB4` single writers) + PCX preload guard `0x00AC48D4`; per-control: classify, insert into 3 hashtables, alloc 0x208 record (initializer `0x00623340` [D MODAL/SHELL]) | [V] census + doc |
| WM_DESTROY | routing prune `FUN_005d4ed0` (find via vtbl+0x10, compact); hashtable removals at `0060f0fe`/`0060eaf2`/(record removal in unbounded region) | [V] decompile + census |
| dialog close | teardown `0x00622720`: LIFO compact + focus restore to previous dialog or `g_hWnd` | [V] decompile |
| scenario lifecycle (Init_Clear chain) | `FUN_00565800` clears GScreen Buttons head (`FUN_004f42a0`) as its FIRST action, then re-inits owner pools; `FUN_004a8930` clears head (`FUN_004f42e0`) then — if `!g_IsMapEditor` — re-adds the static BSS gadget `DAT_008A06F8` via `this->vtbl[+0x30]` (constructed at `0x004a86xx`, writer `004a86fb`; the full-tactical-screen button by role) | [V] decompiles 0x00565800 / 0x004A8930; class identities [I], see YELLOW |
| sidebar reconfig | gadgets removed from Buttons list via `0x004F4450` (callers: `SidebarClass__Action/AddCameo/SwitchTab/ToggleSidebar` + `FUN_006a6820/83e0/94b0` — get_function_callers 0x004F4450); Ghidra label `Hide_Cameo_Slots` is a caller-derived name; body is a generic remove-from-Buttons helper (**label drift recorded**) | [V] |
| gadget destruction | both dtors (`0x004E1390`, `0x004E1A60`) defensively clear all three focus globals if `this` owns them — destruction anywhere can mutate focus state | [V] census §1.1 |

Gadget heap ownership: ListClass/DropList/CheckList own sub-gadgets and item records
(`operator new(8)` per CheckList row) [D GADGET §6]; sidebar buttons are members of
sidebar objects added/removed from the shared Buttons list (not heap-shared across
systems — the only cross-system sharing is the LIST, not the gadgets) [V callers of
0x004F4450 + D SIDEBAR doc].

---

## 6. Sim-visible-state touchpoints (lockstep audit)

1. **Pump advances the sim**: `FUN_00623120` calls `Main_Tick 0x0055D360` (or
   `Network_ServiceLoop 0x0048D080`) every pumped frame while any shell/modal dialog
   is open [V decompile+disasm]. Rust port: the shell service tick must drive the
   same app-level tick, never a sim-internal function directly.
2. **Sidebar production clicks do NOT mutate sim directly** — they post network
   events (0x0E PRODUCE, 0x0F SUSPEND, 0x10 ABANDON, 0x0B PLACE; event string table
   `0x0082091C`) into the lockstep queue [D SIDEBAR_SYSTEM_GHIDRA_REPORT.md §15,
   ghidra/verified]. Rust sidebar must emit commands, not call sim.
3. **No RNG consumption found in any UI function decompiled this session**
   (Input, Hit_Test, Sticky_Process, factory, teardown, pump body, wndproc helpers,
   tooltip hook): no calls into RNG entry points appear in any of the bodies.
   Scope-limited negative — a corpus-wide proof was not attempted (YELLOW).
4. **Timers**: tooltip uses Win32 `SetTimer`/`KillTimer` ('TTIP') and slide uses
   `Sleep(0x1E)`/wall-clock [V/D] — wall-clock only, no sim frame counter. The
   gadget framework's draw cadence is coupled to Input calls (per GScreen tick),
   not to timers.
5. **UNCHECKED**: options-dialog game-speed application path (direct write vs
   event), and `0x00A8D60E`/`0x00A8DAB4` writer semantics (whether any UI path can
   flip them mid-scenario).

---

## 7. UNVERIFIED (YELLOW)

- Identity of `FUN_00565800` and `FUN_004a8930` owner classes (which Init_Clear in
  the GScreen→Display→Radar→…→Sidebar chain). Roles verified; class binding not.
- `DAT_008A06F8` static gadget = "tactical screen button" by role analogy only; BSS
  (all zeros statically), constructed at `0x004a86xx`.
- Consumer/owner of predicate `FUN_00609E20` (callers not traced).
- The `0x006007xx–0x00600Cxx` duplicate hashtable insert/remove family: writer
  addresses census-verified; calling path unattributed (no Ghidra fn boundaries).
- ToolTipManager construction sites `0x007777B8/C3/0x00777803` (unbounded region).
- `0x00A8D60E` (byte) and `0x00A8DAB4` (dword) precise meanings.
- Roles of the 6 modifier-VK globals (`0x00A8EBF8..0x00A8EC0C`) — poll pattern
  verified, semantic pairing inferred.
- The two static GUIDs `0x007F7C90`/`0x007EA6E8` of QI `FUN_004F4240` and its host
  vtable.
- Companion-field layout of hashtables `0x00AC18C0` and `0x00AC1B48`
  (inferred by analogy with the record table's verified layout).

---

## 8. Rust service-boundary proposal (state → owner)

| State family | Owner | Notes |
|---|---|---|
| 4 gadget focus globals + Buttons head | `ui::gadget::GadgetTree` + `FocusState` (one struct, not 5 statics) | dtor-clears-focus invariant must be modeled (removal hook) |
| LIFO stack + top mirrors + routing vector + accel registry + open-dialog counter | `ui::shell::DialogController { stack, kbd_route, open_count }` | counter is read by in-game code → expose a query, not a global |
| 3 owner-draw hashtables + 0x208 records + z-order array + paint counters | per-`DialogInstance` control map (records die with dialog) | gamemd's process-global tables are an artifact of HWND keying |
| surfaces (`0x00887308/0C/10/14/40`) | `render::SurfaceChain` | primary-pointer *swap* pattern → explicit render-target parameter |
| ToolTipManager (`0x00887368`, suppress `0x00A8F7D8`) | one `app::Tooltips` service shared by both UIs | inclusive-edge rect test + 'TTIP' delay/repeat fields |
| theme colors, dim constants, WOL dims | consts in `ui::shell::geom` / theme | single-writer-at-init in gamemd → plain consts |
| keyboard singleton + modifier VKs | `input::Keyboard` | already the boundary used by both frameworks |
| click sounds | rules-driven `audio` lookups | RulesClass fields, not UI globals |

---

## Sources (this session)

decompile_function: 0x004E1640, 0x004E15A0, 0x004E19A0, 0x004E19D0, 0x00622650,
0x00622720, 0x005D4E70, 0x005D4ED0, 0x005D4D50, 0x004F4240, 0x004F4320 (also
disassembled), 0x004F4450, 0x00775700, 0x00775940, 0x00775B10, 0x00775BC0,
0x0060CF00, 0x00609E20, 0x00608CD0, 0x006040B0 (51KB, partial read + symbol census),
0x00623120 (also disassembled), 0x00724200, 0x00565800, 0x004A8930.
read_memory: 0x007F5BE0, 0x007E92BC, 0x007E528C, 0x007ED10C, 0x00488690, 0x004E1970,
0x004F42A0, 0x004F42E0, 0x00849DE0, 0x008A06F8.
get_bulk_xrefs: focus globals, list heads, stack/routing/accel, owner-draw tables,
surfaces, colors, pump globals, tooltip globals, modifier keys (3 batches).
get_xrefs_to: 0x007E9344. get_function_callers: 0x004F42A0, 0x004F42E0, 0x004F4450,
0x00488690.
Docs: GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md, SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md,
SIDEBAR_SYSTEM_GHIDRA_REPORT.md §15, SHELL_UI_SOUND_PLAYBACK_PLUMBING_GHIDRA_REPORT.md,
skirmish-ui sound family (via research_search).
