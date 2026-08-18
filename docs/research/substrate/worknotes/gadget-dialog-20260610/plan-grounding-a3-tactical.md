# A3 grounding — full-tactical click catcher behavior contract

**Status:** GROUNDING WORKNOTE (read-only RE; no Rust written). Feeds the `ui::gadget` migration plan (substrate program, Slice A-series).
**Date:** 2026-06-11
**Scope:** A3 part 1 — the invisible full-tactical-screen click catcher gadget (study global `0x008A06F8`, vtable `0x007E608C`, init `0x004A86E0`, Action `0x004AAC10`). Decodes the *behavior contract* the Rust `ClickRegion` GadgetBehavior + driver routing must reproduce. Does NOT decode the DisplayClass command/cursor internals (out of scope — the Rust driver maps a catcher-consumed event back onto the EXISTING Rust tactical handlers).
**Authority order:** binary → Ghidra → docs. Parent study (`GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md`) seed addresses are treated as starting points and re-verified here.
**Every load-bearing claim is tagged VERIFIED-LIVE (read this session, MCP call cited) or INFERRED.**

> **Ghidra-DB note (label drift / undefined functions):** Neither seed address has a *defined function* in the current Ghidra DB — `get_function_by_address 0x004A86E0` and `0x004AAC10` both return "No function found", and `decompile_function` / `disassemble_function` fail on both. The code *is* present and valid (`disassemble_bytes dry_run` succeeds for both). All instruction-level claims below are **hand-decoded from `read_memory` byte dumps this session**; each cites the exact read. This is the same byte-decode discipline the parent study used for `0x004A86E0` (verdict `live-gadget-census` evidence e) and `0x00723EC0` (verdict `toggle-action-decode`). Function bodies that *are* defined (FUN_00692300, FUN_004a8930, FUN_00653010, Set_View_Dimensions, base Action `0x004E1530`, Sticky_Process `0x004E1970`) were `decompile_function`'d / dumped directly.

---

## 0. One-paragraph contract (TL;DR)

The full-tactical catcher is a base-`GadgetClass`-shaped, **invisible, sticky, Action-only** gadget covering the entire tactical viewport. It is the lowest-priority gadget on the in-game Buttons list, so the smallest-area hit-test naturally lets any overlapping sidebar/command button win, and only "bare tactical" clicks reach it. Its event mask is **`0x7F`** = every mouse bit *except* RIGHTUP (`0x80`) and KEYBOARD (`0x100`): LEFTPRESS/HELD/RELEASE/UP + RIGHTPRESS/HELD/RELEASE. Its own Action does all the DisplayClass cursor/object-pick/command work, then **unconditionally tail-calls the base `GadgetClass::Action`**, which (a) flags itself to redraw, (b) runs `Sticky_Process` — **press (`0x11`) acquires `g_StickyFocus`, release (`0x44`) releases it**, because the catcher's `IsSticky` byte is 1 — and (c) **returns 1 (event consumed)**, stopping the broadcast walk. Registration: `FUN_004a8930` adds `&DAT_008a06f8` to the Buttons list via the GScreen chain's `Add_A_Button` slot (`+0x30` → `0x004F4410`) **only when `g_IsMapEditor == 0`**; the rect (X/Y/W/H) is *not* set at static-init time (those stay 0) — it is written later by `Set_View_Dimensions` to the live tactical viewport (`g_RadarViewportOffsetX/Y/Width/Height`).

---

## 1. Init `0x004A86E0` — static-init construct-and-register thunk

**VERIFIED-LIVE: `read_memory 0x004A86E0` (64 bytes) this session.** Hand-decode of the 36-byte body:

```
6a 01                 push 1                     ; arg6 sticky = 1
6a 7f                 push 0x7F                  ; arg5 flags  = 0x7F
6a 00                 push 0                     ; arg4 H = 0
6a 00                 push 0                     ; arg3 W = 0
6a 00                 push 0                     ; arg2 Y = 0
6a 00                 push 0                     ; arg1 X = 0
b9 f8 06 8a 00        mov ecx, 0x008A06F8        ; this = the global catcher gadget
e8 fa 8b 03 00        call 0x004E12F0            ; GadgetClass::ctor(x,y,w,h,flags,sticky)
68 10 87 4a 00        push 0x004A8710            ; atexit dtor thunk
c7 05 f8 06 8a 00  8c 60 7e 00   mov [0x008A06F8], 0x007E608C   ; overwrite vtable ptr
e8 80 10 32 00        call 0x007C97C1            ; atexit(register dtor)
59                    pop ecx
c3                    ret
```

Call-target arithmetic (all checked): ctor call site `0x004A86F1`, next `0x004A86F6`, rel `0x00038BFA` → **`0x004E12F0` = GadgetClass ctor** (matches study `0x004E12F0`). The vtable store writes **`0x007E608C`** into `[0x008A06F8+0]` — i.e. the constructed base gadget is re-vtabled to the tactical-screen gadget vtable.

Confirmed facts (item 1 of brief):
- **Flags (event mask) `0x7F`** — `push 0x7F`. VERIFIED-LIVE.
- **IsSticky = 1** — `push 1` (arg6). VERIFIED-LIVE. (ctor stores it at `+0x1D`; the parent study verified the ctor maps sticky→`+0x1D` and OR's `Flags |= 5`, decompile `0x004E12F0` — DOC-INHERITED, consistent with the Sticky_Process read in §5 which reads `[ecx+0x1D]`.)
- **Rect X/Y/W/H = 0,0,0,0 at static-init** — four `push 0`. VERIFIED-LIVE. The *real* viewport rect is set later (see §4).
- **Vtable = `0x007E608C`** (study `0x007E608C`). VERIFIED-LIVE: the `mov [0x008A06F8],0x007E608C` immediate; and `read_memory 0x008A06F8` currently reads all-zero only because the binary image is pre-run state (static init hasn't executed in the on-disk image).

### 1.1 The catcher vtable `0x007E608C` overrides ONLY Action

**VERIFIED-LIVE: `read_memory 0x007E608C` (132 bytes) this session.** Decoded against the GadgetClass base vtable layout (parent study §2.1):
- Slot 0 (`+0x00`) dtor = `0x004AEBB0` (study: dtor `0x004AEBB0`). ✔
- Slot 30 (`+0x78`) = `0x004E1970` = **Sticky_Process** (the shared base method — NOT overridden). ✔
- Slot 31 (`+0x7C`, Action) = `0x004AAC10` = **the catcher Action** (the one override). ✔ (`read_memory 0x007E6104` → byte 4-7 = `10 ac 4a 00`.)
- Slot 32 (`+0x80`, Clicked_On) = `0x004E13F0` = the **shared base** Clicked_On (NOT overridden — the dispatch spine is shared by all 20 family classes, study §2.2).

Cross-check vs study §2.2 row "Tactical-screen gadget (0x007E608C) | 33 slots | vs Gadget: dtor 0x004AEBB0; +0x7C Action 0x004AAC10 only (invisible)" — **MATCHES**. The base `Draw_Me` (`+0x6C`, dirty-gate stub) is inherited → the gadget is invisible (no painter), which is why it is "Action-only".

---

## 2. Action `0x004AAC10` — what it does, and that it consumes the event

**VERIFIED-LIVE: `read_memory 0x004AAC10` (208 bytes) + `0x004AACE0` (220 bytes) + `0x004AAD00` (40 bytes) this session.** Thiscall, signature `(this in ecx, flags arg [esp+0x30] on entry)`; `sub esp,0x2C; push esi` frame. Hand-decoded structure:

### 2.1 Coordinate fetch — press/release use the event queue, held/up use live mouse

```
8a 44 24 30           mov al, [esp+0x30]         ; al = event flags
56                    push esi
a8 55                 test al, 0x55              ; 0x55 = LEFTPRESS|LEFTRELEASE|RIGHTPRESS|RIGHTRELEASE
8b f1                 mov esi, ecx               ; this
... (zero locals) ...
74 14                 jz  <no L/R press/release> ; e.g. HELD / UP only
;  --- press/release branch: use the QUEUED event coords ---
a1 70 f7 87 00        mov eax, [0x0087F770]      ; WWKeyboard ptr (event queue)
8b 08 / 8b 40 04      mov ecx,[eax]; mov eax,[eax+4]   ; event coords at +0/+4
89 4c 24 1c / 89 44 24 20   store captured coords
eb 33                 jmp <common>
;  --- held/up branch: use LIVE mouse position ---
8b 0d 40 76 88 00     mov ecx, [0x00887640]      ; WWMouse ptr
ff 50 2c              call [vtbl+0x2C]           ; WWMouse Get-X
... call [vtbl+0x30]  (Get-Y) ... call [vtbl+0x34] ...
```

This is the documented split (parent study §2.4): on a discrete press/release the catcher uses the **latched event coordinates** from the WWKeyboard event queue (`[0x0087F770]`, coords at `+0`/`+4`); on held/up ticks it reads the **live mouse position** (`[0x00887640]`, WWMouse vtbl `+0x2C`/`+0x30`/`+0x34`). VERIFIED-LIVE.

### 2.2 The body forwards to the DisplayClass cursor/object resolver and the Tactical chain

The middle of the body calls into the world-space resolution chain (instance `[0x0087F7E8]`, the static DisplayClass/Tactical chain — parent study §1.1) and a screen-point resolver:

- `b9 e8 f7 87 00; e8 27 76 1e 00` → call **`FUN_00692300`** (target arithmetic: site `0x004AACD4`, next `0x004AACD9`, rel `0x001E7627` → `0x00692300`). **VERIFIED-LIVE: `decompile_function 0x00692300`.** FUN_00692300 is a **screen-point → cell/object resolver** (PickObjectAtScreenPoint-shape): adds `g_RadarViewportOffsetX/Y`, converts to a cell via `FUN_006d6590`/`FUN_006d2280`, computes ground height, tests shroud (`IsShrouded`), reads garrison/building flags, calls `Tactical__PickObjectAtScreenPoint`, and returns the object/cursor decision. This is the **command-cursor + object-under-mouse** machinery — exactly the "what does a tactical click resolve to" logic, NOT a result-ID post.
- A band-box / drag-distance compare against `[0x0088095C]`/`[0x0088095E]` (the captured press anchor) gates a follow-on call into the chain instance `0x0087F7E8` (`e8 a7 e4 ff ff`, a same-module backward call). **INFERRED (shape):** this is the band-box-selection / drag handling on the tactical surface — consistent with "press anchors, drag extends a selection box". Not decoded to leaf depth (out of scope; Rust reuses its own selection handlers).

**Key contract takeaways (item 2):**
- It does **NOT** post an `ID|0x8000` result. The catcher is a base-`GadgetClass`/SBGadget-shape (no `ID`, no `Peer` fields — `ControlClass` adds those at `+0x24`/`+0x28`; this gadget is `sizeof 0x24` base). It **acts directly** on the DisplayClass/Tactical state. VERIFIED-LIVE (no ID field; vtable inherits base `Get_ID` returning 0).
- It responds to mouse events across the full `0x7F` mask: it branches on press/release (`0x55`) vs held/up for *coordinate sourcing*, but the body runs for any nonzero masked flag. **Right-press IS in the mask** (`0x10`) and reaches the resolver — i.e. the catcher handles right-button events on the tactical surface (RA2 context/scroll/deselect uses right button). VERIFIED-LIVE (mask `0x7F` includes `0x10`/`0x40`).

### 2.3 The terminal: unconditional tail-call to base Action → Sticky_Process + return 1

**VERIFIED-LIVE: `read_memory 0x004AAD00` this session.** The function's exit:

```
8b 44 24 38           mov eax, [esp+0x38]        ; reload the event flags arg
6a 00                 push 0
50                    push eax                   ; flags
6a 00                 push 0
8b ce                 mov ecx, esi               ; this
e8 17 68 03 00        call 0x004E1530            ; base GadgetClass::Action(flags,...)
5e                    pop esi
83 c4 2c              add esp, 0x2C
c2 0c 00              ret 0xC                     ; thiscall, returns base Action's eax
```

Call-target arithmetic: site `0x004AAD14`, next `0x004AAD19`, rel `0x00036817` → **`0x004E1530` = base `GadgetClass::Action`**.

**Base `GadgetClass::Action` `0x004E1530`** — **VERIFIED-LIVE: `read_memory 0x004E1530` (48 bytes) this session:**

```
8b 44 24 04           mov eax, [esp+4]           ; flags
85 c0 / 74 12         test eax,eax; jz +0x12     ; flags==0 → return 0
8b 11                 mov edx, [ecx]             ; vtable
50                    push eax                   ; flags
c6 41 1c 01           mov byte [ecx+0x1C], 1     ; IsToRedraw = 1
ff 52 78              call [edx+0x78]            ; Sticky_Process(flags)   (vtbl +0x78 = 0x004E1970)
b8 01 00 00 00        mov eax, 1
c2 0c 00              ret 0xC                     ; <-- returns 1 (CONSUMED)
33 c0 / c2 0c 00      xor eax,eax; ret 0xC        ; flags==0 → returns 0
```

So with any nonzero masked flag the catcher → base Action **flags-to-redraw, runs Sticky_Process, and returns 1**. The catcher's own Action returns whatever base Action returned ⇒ **nonzero (consumed) for any real mouse event** ⇒ the broadcast walk stops at the catcher. VERIFIED-LIVE.

**Contract item 2(a):** consumes the event (returns 1) → broadcast walk stops. ✔
**Contract item 2(b):** press acquires sticky capture / release releases it — proven in §5 (Sticky_Process). ✔
**Contract item 2(c):** which buttons it responds to = its mask `0x7F` (§3). ✔

---

## 3. Mask `0x7F` — exact event-bit decode

Against the event-flag table (brief context, parent study §5): `0x1` LEFTPRESS, `0x2` LEFTHELD, `0x4` LEFTRELEASE, `0x8` LEFTUP, `0x10` RIGHTPRESS, `0x20` RIGHTHELD, `0x40` RIGHTRELEASE, `0x80` RIGHTUP, `0x100` KEYBOARD.

`0x7F` = `0b0111_1111` = bits 0..6:

| Bit | Value | Name | In `0x7F`? |
|---|---|---|---|
| 0 | 0x01 | LEFTPRESS | **yes** |
| 1 | 0x02 | LEFTHELD | **yes** |
| 2 | 0x04 | LEFTRELEASE | **yes** |
| 3 | 0x08 | LEFTUP | **yes** |
| 4 | 0x10 | RIGHTPRESS | **yes** |
| 5 | 0x20 | RIGHTHELD | **yes** |
| 6 | 0x40 | RIGHTRELEASE | **yes** |
| 7 | 0x80 | RIGHTUP | **NO** |
| 8 | 0x100 | KEYBOARD | **NO** |

**VERIFIED-LIVE (init `push 0x7F`).** Confirms the brief's hypothesis exactly: the catcher consumes left press/held/release/up and right press/held/release, but **NOT right-up** (`0x80`) and **NOT keyboard** (`0x100`). The mask is what the per-gadget `Clicked_On` filter (`0x004E13F0`, shared) ANDs the live event flags against before invoking Action — only masked-in bits reach this gadget. So:

- Left button: full lifecycle (down → drag → up → release) — band-box select / move-attack command.
- Right button: press/held/release (RA2 right-drag scroll, right-click deselect/cancel) — but the terminal right-*up* is left for someone else / ignored.
- Keyboard never routes here (keyboard goes to the focus-holder tier, not the broadcast walk).

**Rust ClickRegion implication:** the `ClickRegion` gadget's event mask = `LEFTPRESS|LEFTHELD|LEFTRELEASE|LEFTUP|RIGHTPRESS|RIGHTHELD|RIGHTRELEASE` (exclude RIGHTUP and KEYBOARD). It consumes (returns Handled) for any of those, which both stops the walk and drives sticky capture.

---

## 4. Registration + rect — WHEN and HOW it joins the Buttons list

### 4.1 Registration site `FUN_004a8930` — Add_A_Button, gated on `!g_IsMapEditor`

**VERIFIED-LIVE: `decompile_function 0x004a8930` this session:**

```c
void __fastcall FUN_004a8930(int *param_1) {       // param_1 = GScreen chain instance
  FUN_004f42e0();                                  // a Buttons-head zero-stub writer (study §2.4: 0x004F42E0)
  if (g_IsMapEditor == '\0') {
    FUN_005565f0();                                // LinkClass::Zap (study 0x005565F0)
    (**(code **)(*param_1 + 0x30))(&DAT_008a06f8); // chain vtbl +0x30 = Add_A_Button(catcher)
  }
  return;
}
```

- **`Add_A_Button` confirmation:** the called slot is `param_1` vtable `+0x30`. **VERIFIED-LIVE: `get_xrefs_to 0x004F4410`** lists a DATA xref `From 007e6144 [DATA]`, and **`read_memory 0x007E6138`** shows the dword at `0x007E6144` (= chain-vtbl base `0x007E6114` + `0x30`) = `0x004F4410` = **GScreenClass::Add_A_Button** (study §2.3, chain slot 12 = `+0x30`). So `(*param_1+0x30)(&DAT_008a06f8)` = `Add_A_Button(catcher)`. ✔
- **WHEN:** **only when `g_IsMapEditor == 0`** (i.e. a normal skirmish/campaign, NOT the WAE/in-game map editor). VERIFIED-LIVE. Matches study A8 / §2.5.
- **Caller:** `get_function_callers 0x004a8930` → **`FUN_00653010`** (sole caller). `decompile_function 0x00653010` shows it then sets up the two radar-frame mode buttons (IDs `0xF2`/`0xF3`, via `Set_Position 0x004E1A20` + `Set_Shape 0x0069DE00` + `Flag_To_Redraw 0x004E1960`), also gated on `!g_IsMapEditor`. So `FUN_00653010` is the **radar/tactical surface init** that registers the catcher and builds the radar buttons together — i.e. scenario/session UI setup. VERIFIED-LIVE.
- The `0x008A06F8` xref set (`get_xrefs_to 0x008A06F8`): writes at `0x004a86fb` (init vtable store), data refs at `0x004a8941`/`0x004a894d` (the Add_A_Button arg in FUN_004a8930), plus the init's own refs — a small, closed writer set. VERIFIED-LIVE.

### 4.2 Rect set by `Set_View_Dimensions` = the tactical viewport

The static init left X/Y/W/H = 0. The real rect is written by **`Set_View_Dimensions 0x004A8B00`**. **VERIFIED-LIVE: `get_xrefs_to 0x008A0704`/`0x008A070C`/`0x008A0710`** all report `... in Set_View_Dimensions [WRITE]`, and `decompile_function 0x004a8b00`:

```c
  _DAT_008a0704 = g_RadarViewportOffsetX;   // catcher +0x0C = X
  _DAT_008a0708 = g_RadarViewportOffsetY;   // catcher +0x10 = Y
  _DAT_008a070c = g_RadarViewportWidth;     // catcher +0x14 = W
  _DAT_008a0710 = g_RadarViewportHeight;    // catcher +0x18 = H
```

(`0x008A0704` = base `0x008A06F8` + `0x0C` = the X field; the four fields are contiguous +0x0C..+0x18.) So **the catcher rect == the tactical viewport** (`g_RadarViewportOffsetX/Y/Width/Height`), updated every time the view dimensions change (resolution change, sidebar collapse/expand re-anchoring the viewport). VERIFIED-LIVE. This also confirms the catcher rect tracks the *live* viewport, not a fixed 1024×768.

---

## 5. Sticky capture — press acquires, release releases (the drag-stays-with-tactical mechanism)

**Sticky_Process `0x004E1970`** — **VERIFIED-LIVE: `read_memory 0x004E1970` (48 bytes) this session.** Hand-decode (thiscall, flags arg `[esp+4]`):

```
8a 41 1d              mov al, [ecx+0x1D]         ; IsSticky byte
84 c0                 test al, al
8a 44 24 04           mov al, [esp+4]            ; flags
74 0c                 jz  <release-check>        ; not sticky → skip acquire
a8 11                 test al, 0x11              ; LEFTPRESS(0x1) | RIGHTPRESS(0x10)
74 08                 jz  <release-check>
89 0d 88 3e 8b 00     mov [0x008B3E88], ecx      ; g_StickyFocus = this   (ACQUIRE)
eb 08                 jmp <ret>
<release-check>:
39 0d 88 3e 8b 00     cmp [0x008B3E88], ecx      ; am I the current holder?
75 0e                 jnz <ret>
a8 44                 test al, 0x44              ; LEFTRELEASE(0x4) | RIGHTRELEASE(0x40)
74 0a                 jz  <ret>
c7 05 88 3e 8b 00 00000000   mov [0x008B3E88], 0 ; g_StickyFocus = NULL   (RELEASE)
<ret>: c2 04 00       ret 4
```

Contract (item 2(b)):
- **Acquire:** iff `IsSticky` (catcher = 1) AND flags has a **press** bit (`0x11` = left OR right press) → `g_StickyFocus [0x008B3E88] = this`. VERIFIED-LIVE.
- **Release:** iff `this == g_StickyFocus` AND flags has a **release** bit (`0x44` = left OR right release) → `g_StickyFocus = NULL`. VERIFIED-LIVE.
- This matches the parent study verdict `sticky-process-decode` (acquire `0x11`, release `0x44`).

**While captured**, the dispatcher (`GadgetClass::Input 0x004E1640`, study A1/A2) re-dispatches the holder every Input tick even when its masked flags are 0 — so a left-drag that started on the tactical area keeps feeding HELD/coordinate updates to the catcher's Action **even if the cursor crosses into the sidebar**, until the matching release. That is precisely the "box-select / drag stays with the tactical region" behavior the Rust port must reproduce. (Sticky-tier re-dispatch is DOC-INHERITED from study A2 / verdict `clickedon-sticky-kbd-bypass`; the acquire/release endpoints are VERIFIED-LIVE here.)

---

## 6. The OBSERVABLE contract (item 4) — what the Rust walk must reproduce

1. **One retained list for everything in-game.** Tactical clicks resolve through the *same* Buttons list (`[0x00A8EF54]`) and the *same* per-tick dispatch as sidebar buttons — there is no separate tactical input path. The catcher is just another gadget on that list (added by `Add_A_Button`). VERIFIED-LIVE (§4.1).
2. **Smallest-area-wins gives sidebar priority.** The catcher's rect is the *entire* tactical viewport (786k+ px²) — the largest live gadget. The hit-test picks the **smallest-area** gadget under the cursor (study G14, half-open rects, `<=` tie-break). So any sidebar/command/tab/cameo button overlapping the cursor (tiny rects) wins; the catcher only wins on "bare tactical" pixels. No explicit z-order needed — area ordering does it. VERIFIED-LIVE (rect = full viewport; hit-test rule DOC-INHERITED study G14/A5).
3. **Sticky capture keeps a drag with tactical.** Press on the catcher acquires `g_StickyFocus`; while held, the catcher (not whatever the cursor now overlaps) receives the event; release frees capture. A box-select that drags over the sidebar does not get hijacked by a sidebar button. VERIFIED-LIVE (§5).
4. **The catcher consumes the event** (returns 1) so the broadcast walk stops there — no double-dispatch to a lower gadget. VERIFIED-LIVE (§2.3).
5. **No result-ID protocol** for the catcher: unlike `ControlClass`/`ToggleClass` buttons (which post `ID|0x8000`), the catcher acts directly on world/cursor state. The Rust equivalent: a `ClickRegion` whose handler is a direct callback into the existing tactical selection/command code, not an ID enqueue. VERIFIED-LIVE (no ID/Peer fields; acts via DisplayClass calls §2.2).

---

## 7. Rust port contract — `ClickRegion` GadgetBehavior + driver routing

The Rust `ui::gadget` service must expose a `ClickRegion` gadget that reproduces the catcher's *behavior contract* (NOT its DisplayClass internals — those map onto the EXISTING Rust tactical selection/command handlers):

**Gadget (`ClickRegion` GadgetBehavior):**
- **Geometry:** rect = the live tactical viewport (Rust equivalent of `g_RadarViewportOffsetX/Y/Width/Height`), re-set whenever the viewport changes (resolution change, sidebar collapse/expand) — mirror `Set_View_Dimensions` writing the rect. Do NOT hardcode 1024×768.
- **Event mask:** `LEFTPRESS | LEFTHELD | LEFTRELEASE | LEFTUP | RIGHTPRESS | RIGHTHELD | RIGHTRELEASE` (i.e. `0x7F`); **exclude** RIGHTUP and KEYBOARD.
- **Sticky:** `IsSticky = true`. On a press bit (left or right) the dispatcher acquires capture for this gadget; on the matching release bit it releases. Implement once in the shared dispatcher's Sticky_Process equivalent (acquire on `LEFTPRESS|RIGHTPRESS`, release-if-holder on `LEFTRELEASE|RIGHTRELEASE`) — this is generic, not catcher-specific.
- **Invisible:** no Draw_Me (inherits the dirty-gate no-op). Never paints.
- **No result ID:** its `Action` returns `Handled` (consumes the event) and invokes a direct driver callback, rather than enqueuing an `ID|0x8000` result. Returning `Handled` stops the broadcast walk.
- **Registration:** added to the retained Buttons list (tail-append via the `Add_A_Button` equivalent) at scenario/session UI init, **only when not in map-editor mode**. Largest area on the list ⇒ lowest hit-test priority.

**Driver routing (the seam):**
- The driver receives the catcher-consumed event (flags + coordinates) and **maps it back onto the existing Rust tactical handlers**: left-press → begin selection / start band-box; left-held while captured → extend band-box / drag (coords from the captured-anchor compare, like `[0x0088095C/5E]`); left-release → finalize selection / issue move-attack command; right-press/held/release → tactical scroll / context / deselect, per current Rust semantics.
- **Coordinate sourcing parity:** on a discrete press/release use the *latched event* coordinates (the click position at event time); on held/up ticks use the *live* cursor position — mirroring the WWKeyboard-queue vs WWMouse split (§2.1). For lockstep/replay this matters only for the visual drag; the *command* coordinates are the press/release latched values.
- **Sticky drag-off:** while the catcher holds capture, route ALL mouse events to the tactical handlers even if the cursor is over the sidebar rect — do not let a sidebar gadget steal a mid-drag event. The driver does not re-hit-test while a sticky holder exists.
- The driver does NOT reimplement `FUN_00692300` (object-pick/cursor resolve) or the band-box engine — those are the existing Rust tactical systems; the seam only delivers the gadget-walk-resolved event to them.

---

## 8. UNKNOWN / blockers

- **[Y] Band-box / drag internals of Action not leaf-decoded.** §2.2's chain calls (`0x0087F7E8` methods + the `[0x0088095C/5E]` anchor compare) are identified by shape as the selection-box/drag path but not traced to leaf. **Not a blocker** for A3 part 1 — the Rust driver reuses its own selection code; the contract only needs "press anchors, drag extends, release commits, capture is sticky," all of which ARE verified. Flagged for completeness.
- **[Y] FUN_00692300 leaf semantics (cursor/object decision) not fully decoded.** Decompiled and identified as the screen-point→cell/object resolver feeding the command cursor; the exact cursor-state byte meanings (garrison `+0x6e7`, `+0x6ed==0x0F`, sensor checks) are out of scope here — they belong to a cursor/command-feedback grounding note, not the click-catcher contract.
- **[VERIFIED, not a blocker] Right-up (`0x80`) handler.** The catcher's mask excludes RIGHTUP. Where (if anywhere) right-up is consumed in YR is not part of this gadget; if the Rust port needs right-up behavior (e.g. end-of-right-drag-scroll), confirm it is NOT expected from the catcher. Mask exclusion is VERIFIED-LIVE; the *consumer* of right-up is UNKNOWN — not read this session.
- **[Resolved] Rect-is-zero-in-image.** `read_memory 0x008A0700` reads all-zero only because this is the pre-run on-disk image; `Set_View_Dimensions` writes the live rect at runtime (§4.2). Not a real blocker.
- **[Ghidra hygiene] Seed functions undefined in the DB.** `0x004A86E0` and `0x004AAC10` have no Ghidra function object (decompile/disassemble fail). All claims here are byte-hand-decodes from `read_memory`. A future labeling pass should `create_function` at both and at `Set_View_Dimensions`-adjacent globals, then re-verify the decompile matches this note. (Recorded so the next session doesn't mistake "no function found" for "address wrong".)

---

## 9. Evidence ledger (MCP calls this session)

| Claim | Call |
|---|---|
| init body (sticky=1, flags=0x7F, rect=0, ctor 0x004E12F0, vtable store 0x007E608C) | `read_memory 0x004A86E0` (64) |
| catcher vtable: dtor 0x004AEBB0, Sticky_Process 0x004E1970, Action 0x004AAC10, Clicked_On 0x004E13F0 | `read_memory 0x007E608C` (132), `read_memory 0x007E6104` (8) |
| Action body: 0x55 press/release coord-fetch split, WWKeyboard `0x0087F770`, WWMouse `0x00887640` | `read_memory 0x004AAC10` (208), `0x004AACE0` (220) |
| Action terminal: base-Action tail-call 0x004E1530, ret 0xC | `read_memory 0x004AAD00` (40) |
| Action calls screen-point resolver FUN_00692300 (PickObjectAtScreenPoint-shape) | `decompile_function 0x00692300` |
| base GadgetClass::Action: IsToRedraw=1, Sticky_Process via vtbl+0x78, return 1 / return 0 | `read_memory 0x004E1530` (48) |
| Sticky_Process: acquire on 0x11 iff IsSticky, release on 0x44 iff holder, g_StickyFocus=0x008B3E88 | `read_memory 0x004E1970` (48) |
| registration FUN_004a8930: zero-stub 0x004F42E0, Zap 0x005565F0, Add_A_Button(catcher) gated !g_IsMapEditor | `decompile_function 0x004a8930` |
| Add_A_Button = chain vtbl +0x30 = 0x004F4410 | `get_xrefs_to 0x004F4410`, `read_memory 0x007E6138` (24) |
| sole caller = FUN_00653010 (radar/tactical surface init, !g_IsMapEditor) | `get_function_callers 0x004a8930`, `decompile_function 0x00653010` |
| rect set by Set_View_Dimensions to g_RadarViewportOffsetX/Y/Width/Height | `get_xrefs_to 0x008A0704/0x008A070C/0x008A0710`, `decompile_function 0x004a8b00` |
| catcher global writer set | `get_xrefs_to 0x008A06F8` |
| seed addresses undefined in Ghidra DB | `get_function_by_address 0x004A86E0`, `0x004AAC10` (both "No function found") |
