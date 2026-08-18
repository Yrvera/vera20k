# A3 grounding — minimap/radar click region behavior contract

**Subsystem:** gadget/dialog control substrate — A3 part 2 (minimap/radar click catcher)
**Binary:** gamemd.exe (image base 0x00400000)
**Date:** 2026-06-10
**Author:** RE grounding agent (live Ghidra session)
**Authority order:** binary -> Ghidra -> docs. Seeds came from
`GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md`; treated as starting
points, all load-bearing facts re-verified live this session and cited inline.

> Confidence legend: **VERIFIED-LIVE** = read out of Ghidra this session with the
> cited call. **INFERRED** = reasoned from verified facts, not directly read.

---

## 0. TL;DR behavior contract

- The minimap "gadget" is a single global object at **0x00B04A10**
  (`vtable__RTacticalClass__RadarClass` @ 0x007F02BC), a **plain GadgetClass
  subclass** (NOT ControlClass — it has no ID at +0x24 and no Peer at +0x28).
- Constructed with **rect = (X=0, Y=0, W=0, H=0)**, **event mask Flags = 0x9F**,
  **IsSticky = 1**. (`decompile 0x00652870`, `disassemble 0x00652870`,
  `decompile 0x004e12f0`.)
- It overrides exactly **two** vtable slots vs base GadgetClass: slot 0
  (scalar-deleting destructor, standard for any subclass) and **slot 31 =
  Action** (0x006539D0). All other slots are byte-identical to
  `vtable__GadgetClass`. So for *behavior*, it overrides **ONLY Action**.
  (`read_memory 0x007e92bc` vs `read_memory 0x007f02bc`, 132 bytes each.)
- **It posts NO result key.** Unlike sidebar buttons (ControlClass::Action posts
  ID|0x8000), RTacticalClass::Action **acts directly on the Display / camera /
  cursor**: it reads the mouse pixel, maps it to a radar cell, and either
  drives object selection + cursor (left click) or **centers the tactical view
  on the clicked cell** (right click / right-press). (`decompile 0x006539D0`.)
- Because rect is (0,0,0,0) it is effectively never won by a *rectangle*
  hit-test; it relies on **IsSticky capture + the radar-active gate + an
  internal radar-surface bounds self-check** inside Action. The hit region is
  `g_RadarSurfaceOriginX/Y .. +Width/+Height` (0x00880c84..0x00880c90),
  re-tested inside Action — NOT the gadget's own X/Y/W/H.

---

## 1. The minimap region init (0x00652870)

`decompile 0x00652870`:

```c
void FUN_00652870(void) {
  GadgetClass__Constructor(0,0,0,0,0x9f,1);          // this=0xb04a10 (thiscall)
  _DAT_00b04a10 = &vtable__RTacticalClass__RadarClass; // 0x007F02BC
  FUN_007c978a(&LAB_006528a0);                        // register static-init thunk
  return;
}
```

`disassemble 0x00652870` (shows the real thiscall + arg order):

```
00652870: PUSH 0x1            ; param7 IsSticky = 1
00652872: PUSH 0x9f           ; param6 Flags   = 0x9F
00652877: PUSH 0x0            ; param5 H = 0
00652879: PUSH 0x0            ; param4 W = 0
0065287b: PUSH 0x0            ; param3 Y = 0
0065287d: PUSH 0x0            ; param2 X = 0
0065287f: MOV  ECX,0xb04a10   ; this = &g_RTactical (the global object)
00652884: CALL 0x004e12f0     ; GadgetClass__Constructor
00652889: PUSH 0x6528a0
0065288e: MOV  dword [0x00b04a10],0x7f02bc  ; install RTactical vtable
00652898: CALL 0x007c978a     ; register init thunk LAB_006528a0
0065289e: RET
```

### 1a. Field writes — confirmed against the constructor body

`decompile 0x004e12f0` (`GadgetClass__Constructor`, thiscall, param_1 = this):

```c
param_1[3]  = param_2;        // +0x0C  X      = 0
param_1[4]  = param_3;        // +0x10  Y      = 0
param_1[5]  = param_4;        // +0x14  W      = 0
param_1[6]  = param_5;        // +0x18  H      = 0
param_1[1]=0; param_1[2]=0;   // +0x04/+0x08 (unused pair)
*(u8*)(param_1+7)        = 0; // +0x1C  IsToRedraw = 0
*(u8*)((int)param_1+0x1e)= 0; // +0x1E  IsDisabled = 0
*(u8*)((int)param_1+0x1d)= param_7; // +0x1D IsSticky = 1
param_1[8]  = param_6;        // +0x20  Flags  = 0x9F
*param_1 = &vtable__GadgetClass;
if (param_7 != 0) param_1[8] = param_6 | 5;  // sticky => OR in LEFTPRESS|LEFTRELEASE
```

**VERIFIED-LIVE.** Field offsets exactly match the study's gadget layout
(+0x0C X, +0x10 Y, +0x14 W, +0x18 H, +0x1C IsToRedraw, +0x1D IsSticky,
+0x1E IsDisabled, +0x20 Flags). The sticky side-effect `Flags |= 5` adds bits
0x1 (LEFTPRESS) + 0x4 (LEFTRELEASE); both are already present in 0x9F so the
**stored Flags stays 0x9F**.

### 1b. Event mask 0x9F — exact bit decode

0x9F = 1001_1111b. Against the study's event-flag table:

| bit  | name        | set in 0x9F? |
|------|-------------|--------------|
| 0x01 | LEFTPRESS   | YES |
| 0x02 | LEFTHELD    | YES |
| 0x04 | LEFTRELEASE | YES |
| 0x08 | LEFTUP      | YES |
| 0x10 | RIGHTPRESS  | YES |
| 0x20 | RIGHTHELD   | **NO (excluded)** |
| 0x40 | RIGHTRELEASE| **NO (excluded)** |
| 0x80 | RIGHTUP     | YES |
| 0x100| KEYBOARD    | NO (mask is a byte here, 0x100 not in 0x9F) |

So the minimap subscribes to: **all four left events** (press/held/release/up)
+ **right-press** + **right-up**, and is **deliberately NOT subscribed to
right-held (0x20) or right-release (0x40)**. This matters: a right *drag* on the
minimap fires RIGHTPRESS then RIGHTUP but no RIGHTHELD/RIGHTRELEASE stream.
**VERIFIED-LIVE** (`disassemble 0x00652870` arg + `decompile 0x004e12f0`).

> Note: because this is a plain GadgetClass (no ID), the "ControlClass posts
> ID|0x4000 on right-release iff mask has RIGHTPRESS" rule is irrelevant here —
> there is no Peer and no result key path. See section 2.

### 1c. Vtable + override scope (study claim: "overrides ONLY Action")

`read_memory 0x007f02bc` (RTactical vtable) vs `read_memory 0x007e92bc`
(`vtable__GadgetClass`, label from `list_globals vtable__GadgetClass`), 132 bytes
(33 slots) each:

- **Slot 0** (+0x00): GadgetClass 0x004E1A60 vs RTactical **0x00658780** — DIFFERENT
  (scalar-deleting destructor; every subclass overrides this).
- **Slots 1..30**: byte-identical between the two vtables.
- **Slot 31** (+0x7C): GadgetClass 0x004E1530 vs RTactical **0x006539D0** —
  DIFFERENT = the **Action** override.
- **Slot 32** (+0x80): both 0x004E13F0 — identical.

Confirmed independently: the only [DATA] xref to 0x006539D0 is from **0x007F0338**
(`get_xrefs_to 0x006539d0`), and 0x007F0338 - 0x007F02BC = 0x7C = slot 31. So the
Action address lands in the Action slot of the RTactical vtable.
**VERIFIED-LIVE.** => For behavior, RTacticalClass overrides **only Action**.
(The study's "overrides ONLY Action" is correct; slot-0 destructor is the
universal subclass detail, not a behavior override.)

### 1d. Registration mechanism + when

`FUN_007c978a(&LAB_006528a0)` -> `FUN_007c970c` (`decompile 0x007c978a`,
`decompile 0x007c970c`). This is **NOT** the GadgetClass `Buttons` linked-list
insert. It is the MSVC **static-initializer registration table**: `FUN_007c970c`
appends `param_1` into a growable global pointer array
(`DAT_00b78bbc`/`DAT_00b78bc0`, with realloc growth via `FUN_007d0f45`).
`LAB_006528a0` is a once-guarded init thunk (it checks/sets a flag byte at
0x00B048A8 then `MOV ECX,0xb04a10; JMP` into the real constructor body) —
standard C++ run-time static init. **VERIFIED-LIVE.**

> So "registration into the Buttons list and when": the minimap object is
> **constructed at program static-init time**, not inserted into a live dialog
> Buttons list here. Whether/when it is linked into the in-game gadget walk list
> is governed by the radar-active state, not by this init. The actual gameplay
> dispatch to its Action happens through the in-game input/gadget walk while the
> radar is active (gated by DAT_00880c94/DAT_00880c98 — see 2a). **The exact
> add-to-active-list call site is UNKNOWN this session — see blockers.**

---

## 2. The minimap Action (0x006539D0)

`decompile 0x006539D0`. Signature recovered: `undefined4 Action(byte param_1,
undefined4 param_2)` where **param_1 = the event-flag byte** (one or more of the
0x9F mask bits for the event being delivered) and **param_2 = the key/peer
slot** passed straight through to the base chain.

### 2a. Top gates (early-outs)

```c
if ((param_1 & 0x22) != 0) return 0;     // ignore LEFTHELD(0x02) and RIGHTHELD(0x20)
if (!(DAT_00880c98==1 && DAT_00880c94==1)) return 0;  // radar must be active
```

- `param_1 & 0x22`: bails on **LEFTHELD (0x02)** or **RIGHTHELD (0x20)**.
  (0x20 won't normally arrive — it's not even in the subscription mask; 0x02
  *is* subscribed, and is explicitly dropped here. So held events do nothing.)
- `DAT_00880c94`/`DAT_00880c98` = radar-active state flags (both read 0 in this
  static image — game not loaded). Action no-ops unless radar is in the live
  active mode. **VERIFIED-LIVE** (`decompile 0x006539D0`,
  `read_memory 0x00880c94`).

### 2b. Mouse pixel -> radar cell, and the radar-surface self-hit-test

```c
if ((param_1 & 0x55) == 0) {                       // not a press/release/keyboard edge
    iVar8     = (**(*g_DisplayChain + 0x2c))();     // live mouse X  (hover/up path)
    iStack_28 = (**(*g_DisplayChain + 0x30))();     // live mouse Y
} else {                                            // press/release: use cached pixel
    iVar8     = *DAT_0087f770;                       // cached mouse X
    iStack_28 = DAT_0087f770[1];                     // cached mouse Y
}
iStack_2c = iVar8 - (radar-mirror adjust for DAT_00a8eb7c);
if (iStack_2c <  g_RadarSurfaceOriginX ||
    iStack_2c >= g_RadarSurfaceOriginX + g_RadarSurfaceWidth  ||
    iStack_28 <  g_RadarSurfaceOriginY ||
    iStack_28 >= g_RadarSurfaceOriginY + g_RadarSurfaceHeight) {
    FUN_005bdc80(0,0);   // clear radar hover cursor/highlight
    return 0;            // outside the minimap content rect -> do nothing
}
RadarClass__GetObjectAtRadarPixel(&iStack_2c, &sStack_3c, &iStack_38); // pixel -> cell (sStack_3c/3a) + object
```

**Key insight (VERIFIED-LIVE):** the gadget's own rect is (0,0,0,0); the real
hit region is the **radar surface rect** `g_RadarSurfaceOriginX/Y +
Width/Height` (0x00880c84..0x00880c90, named via `list_globals RadarSurface`),
**re-tested inside Action**. `0x55` = LEFTPRESS|LEFTRELEASE|RIGHTPRESS|RIGHTUP|
KEYBOARD-low (i.e. the "edge" events use the cached click pixel; hover/held-style
delivery uses the live mouse). `RadarClass__GetObjectAtRadarPixel` maps the
radar pixel to a cell + object under it.

### 2c. Left-button path (param_1 & 0x50 == 0, i.e. NOT right-press/right-up)

For left events, when the cell differs from the last (DAT_00b048c0/c2), it:
- computes cell-center lepton coords (`cell*0x100 + 0x80`),
- gets ground height + shroud status,
- if a current command/placement mode is active (`DAT_008809a0` switch:
  build-place=0x14, sell, repair-ish action codes 0x25/0x26/0x29/0x41..0x48, etc.)
  selects an action code; otherwise queries the best object's action
  (`SelectBestObjectForAction` -> vtable +0x70 / +0x74),
- **on LEFTUP (param_1 & 8):** `DisplayClass__SetCursorFromAction(...)` — sets the
  mouse cursor to reflect the action available at that minimap cell,
- **on LEFTRELEASE (param_1 & 4)** (and not band-boxing, DAT_00a8ed9d==0):
  `DisplayClass__BandBox_LeftUp(...)` — commits the action (issue
  move/attack/etc. at the clicked minimap cell, exactly like a left click on the
  tactical map), then `goto LAB_00653ec3`.
- Otherwise falls through to `FUN_005bda80(0,1)` (radar hover feedback).

So **left click on the minimap issues the current command at that cell** (move /
attack / place / sell / repair as gated by command mode), and left-up updates the
cursor glyph. **VERIFIED-LIVE** (`decompile 0x006539D0`).

### 2d. Right-button path = camera jump/center (param_1 & 0x11)

```c
if (((param_1 & 0x11) != 0) && (cell != 0,0)) {
    // clamp the target cell into the visible-viewport-derived bounds
    //   using g_RadarViewportWidth/Height (0x00886fa8/ac) and DAT_0087f8dc/e0
    ... clamp sStack_3c / sStack_3a ...
    MapClass__Get_CellClass(&cell);
    coords = CellClass__Get_Center_Coords(cell);
    FUN_006d6070(&coords);   // == scroll/center the tactical view on those coords
    FUN_004f42f0(1);         // refresh / redraw tactical
    if (g_ZBuffer != 0) *(u32*)(g_ZBuffer+0x24) = 0x8000;  // mark full redraw
}
LAB_00653ec3:
FUN_004e1530(0,param_2,0);  // chain to base GadgetClass::Action (redraw + slot 0x78)
return 1;
```

`param_1 & 0x11` = **LEFTPRESS(0x01) | RIGHTPRESS(0x10)**. The camera-center
branch fires on a **press** edge (left or right press) with a valid clamped
cell, mapping the clicked minimap cell to its center world coords and calling
`FUN_006d6070` = the tactical-view center/scroll routine (same one used by the
tail helper FUN_00653ea0). The clamp uses the radar viewport size
(`g_RadarViewportWidth/Height` 0x00886fa8/ac, `list_globals RadarViewport`) so
the centered cell keeps the whole viewport on-map. **VERIFIED-LIVE.**

> Net player-visible behavior: a click on the minimap (press edge) **jumps the
> tactical camera** so the clicked location is centered; a left click also
> issues the active command at that cell; held events are ignored; up/release
> finalize cursor + command. There is no continuous "camera follows cursor while
> dragging" stream from RIGHTHELD because 0x20 is not subscribed and is
> explicitly dropped — a drag re-jumps only on the next press edge. **The
> drag-scroll feel of "press captures so the drag stays with the minimap" is
> delivered by the sticky-capture flag (1c) keeping subsequent events routed to
> this gadget, NOT by a held-event loop inside Action.**

### 2e. Result protocol

**No result ID is posted.** RTacticalClass is a plain GadgetClass (no +0x24 ID,
no +0x28 Peer), so the ControlClass result-key path (`*param_3 = ID|0x8000`,
`|0x4000` on right-release) is never taken. The base Action it chains to is
`FUN_004e1530` (`decompile 0x004e1530`), which only sets IsToRedraw (+0x1C=1) and
calls vtable slot 0x78; it does **not** write a result key. Compare/contrast:
`GadgetClass__Action @ 0x0048e5a0` (the function with the ID|0x8000 / |0x4000
logic) is actually **ControlClass::Action** — a label-drift note: that label
reads ID at +0x24 and Peer at +0x28, which are ControlClass fields, so the
"GadgetClass__Action" name is misleading; it is the ControlClass override, not
the base. RTacticalClass does NOT use it. **VERIFIED-LIVE**
(`decompile 0x0048e5a0`, `decompile 0x004e1530`, `get_function_callers 0x004e1530`).

### 2f. Which buttons it responds to — summary

| event delivered | param_1 bit | Action does |
|-----------------|-------------|-------------|
| LEFTPRESS  | 0x01 | camera-center branch (2d) + (falls into left path 2c) |
| LEFTHELD   | 0x02 | **ignored** (early `&0x22` return) |
| LEFTRELEASE| 0x04 | commit command at cell (BandBox_LeftUp) |
| LEFTUP     | 0x08 | set cursor-from-action (hover glyph) |
| RIGHTPRESS | 0x10 | camera-center branch (2d) |
| RIGHTHELD  | 0x20 | not subscribed; also `&0x22` would drop it |
| RIGHTRELEASE| 0x40 | not subscribed |
| RIGHTUP    | 0x80 | live-mouse hover path (cursor/feedback), no command |

**INFERRED** mapping of edge-vs-action is read directly from the param_1 masks in
`decompile 0x006539D0`; the routing of which physical mouse event becomes which
bit is the standard GadgetClass dispatch and is consistent with the 0x9F mask.

---

## 3. The two radar-frame ShapeButtons (0x00B04978, 0x00B04910) — A6 scope, NOT A3

Two sibling globals near the minimap, each registered the same static-init way:

- `decompile 0x006528d5` -> `ShapeButtonClass__Constructor()` then register
  `LAB_006528f0`. `LAB_006528f0` once-guards (flag byte 0x00B048A8 bit 0x02)
  then `MOV ECX,0xB04978; JMP` into the real init.
- `decompile 0x00652925` -> `ShapeButtonClass__Constructor()` then register
  `LAB_00652940`. `LAB_00652940` once-guards (bit 0x04) then `MOV ECX,0xB04910`.

`get_function_by_address 0x0069dcf0` confirms the constructor =
`ShapeButtonClass__Constructor`. `decompile 0x0069dcf0` shows it builds a
ShapeButtonClass on top of `ToggleClass__Constructor(0,0,0,0,0)` with **zero
geometry/ID** at construction. So **their X/Y/W/H/ID/shape are assigned later**,
at radar-activation/draw time. **VERIFIED-LIVE.**

Identity / role: both globals are referenced by **`RadarClass__Draw`**
(0x006531c6 / 0x006531d1) and the radar-mode functions `FUN_00653010`,
`FUN_00654320`, `FUN_00652e50` (`get_xrefs_to 0x00B04978`,
`get_xrefs_to 0x00B04910`). These are the **radar-frame ShapeButtons** drawn in
the command bar / radar frame (ToggleClass-derived => they DO post a result key
via ControlClass::Action when clicked, unlike the minimap catcher). Player role:
the radar/minimap **mode toggle buttons** in the radar frame (e.g. toggle between
radar views / minimap modes). **IDs UNKNOWN this session** — not assigned at the
construction sites; would need the radar-activation code (FUN_00653010 /
FUN_00654320) decoded to read the assigned IDs.

**Scoping verdict:** these two are **A6 (command-bar / radar buttons)**, not A3.
They are normal ControlClass-style ShapeButtons (result-key protocol). Scope them
**out** of the A3 minimap-catcher plan; record their globals/identity here so A6
can pick them up. **INFERRED scope, VERIFIED-LIVE identity.**

---

## 4. Observable contract for the Rust gadget walk

1. **Same retained list.** The minimap catcher resolves through the same gadget
   walk as the rest of the in-game UI gadgets (sidebar buttons, scroll buttons,
   tactical catcher). It is a GadgetClass like the others; its Action is invoked
   with the per-event flag byte. **VERIFIED-LIVE** (it's in a GadgetClass vtable,
   chains to base Action). The exact in-game enqueue point into the active walk
   list is UNKNOWN (blocker §5).

2. **Sticky capture.** IsSticky=1 (+0x1D). The constructor also OR'd LEFTPRESS|
   LEFTRELEASE into the mask (no-op vs 0x9F). Sticky means a **press on the
   minimap captures input** so subsequent events keep routing to this gadget
   until release — that is what keeps a minimap **drag-scroll** bound to the
   minimap region even if the cursor leaves the radar rect mid-drag. Action does
   NOT implement a held-loop; the "stays with the minimap" guarantee is the
   sticky-capture flag at the walk level, not Action code. **VERIFIED-LIVE**
   (flag) + **INFERRED** (capture semantics from the standard sticky contract;
   the held-event loop is absent in Action which corroborates this).

3. **Ordering vs tactical catcher and sidebar (overlap question).** The minimap
   gadget has rect (0,0,0,0), so a pure smallest-area rect tie-break is moot for
   *it* — it never wins by rectangle; it wins by **sticky capture (on a prior
   press) and/or by being earlier in the walk while the radar is active**, and
   it self-rejects any click outside `g_RadarSurfaceOrigin..+Size` (§2b). The
   minimap content sits **inside the sidebar/radar frame** (a separate screen
   region from the big tactical catcher), so in practice **the minimap rect and
   the tactical catcher do not overlap** — the minimap surface is within the
   right-side radar panel, the tactical catcher covers the left play area. There
   is therefore no real "smallest-area lets minimap win over the big catcher"
   contest in normal play; they occupy disjoint screen regions, and the minimap
   self-gates on the radar surface bounds regardless. **INFERRED** (geometry
   reasoning from the radar-surface self-test; the literal screen coords of the
   sidebar/radar panel vs tactical viewport are set elsewhere and were not read
   this session — see blocker §5).

---

## Rust port contract

The Rust driver maps a **minimap-consumed gadget event onto the existing Rust
minimap-drag / camera handlers** — it does NOT need a new result-ID path.

- Register the minimap catcher in the same retained gadget walk used for sidebar/
  scroll/tactical gadgets, with: **rect = the radar content rect**
  (`g_RadarSurfaceOriginX/Y, g_RadarSurfaceWidth/Height` equivalents),
  **mask = 0x9F** (left press/held/release/up + right-press + right-up; NOT
  right-held, NOT right-release), **sticky = true**.
- On dispatch, the driver receives the event-flag byte and routes:
  - **press edge (LEFTPRESS 0x01 or RIGHTPRESS 0x10):** acquire **sticky
    capture** for the minimap region, then **center the tactical camera** on the
    cell under the clicked radar pixel (existing Rust minimap->camera-center /
    minimap-drag handler). This is the camera jump.
  - **LEFTRELEASE (0x04):** commit the active command at that minimap cell
    (existing left-click command handler, cell sourced from the radar pixel),
    then end any drag.
  - **LEFTUP (0x08) / RIGHTUP (0x80):** update the hover cursor glyph from the
    action available at the hovered radar cell (existing cursor-from-action
    path); no command issued.
  - **held (0x02 / 0x20):** **ignore** (matches the `&0x22` early-out).
  - **outside the radar content rect:** clear the radar hover highlight and do
    nothing (matches the surface self-test early `return 0`).
- **No result key.** Action operates directly on camera/Display/cursor; the Rust
  side should call the camera/command/cursor handlers directly, not post an
  ID|0x8000 message. (Contrast: sidebar buttons DO post result keys.)
- **Sticky keeps the drag with the minimap:** once a press captures, route all
  following mouse events to the minimap handler (re-center on each subsequent
  press edge; there is no per-frame held re-center in gamemd, so the Rust port
  should not add one — re-jump on press edges only, matching §2d/§2f). If the
  current Rust minimap-drag implementation re-centers continuously on motion,
  that is a **DRIFT** from gamemd (which only acts on press/release/up edges) and
  should be reconciled against this contract.
- Gate the whole thing on the radar-active state (the `DAT_00880c94 /
  DAT_00880c98 == 1` equivalents): no minimap input when the radar is not active.
- The two radar-frame ShapeButtons (mode toggles) are **out of scope for this A3
  plan** — handle them in A6 as normal result-key ShapeButtons.

---

## UNKNOWN / blockers

1. **In-game add-to-active-walk-list call site — UNKNOWN.** The init only does
   C++ static-init registration (§1d). Where/when the constructed RTactical
   object is enqueued into the *live* gadget walk (and in what position relative
   to sidebar/tactical catcher) was not read this session. Needs the radar-
   activation / in-game input setup decoded. Blocks the exact **walk ordering**
   guarantee in §4.3.
2. **Literal sidebar/radar-panel vs tactical-viewport screen coords — UNKNOWN.**
   The "they don't overlap" claim in §4.3 is inferred from the radar-surface
   self-test, not from reading the actual on-screen rectangles. If A3 needs to
   *prove* non-overlap (vs the tactical catcher and the sidebar), read the rect
   assignments for the tactical catcher and the radar surface globals at their
   in-game set sites.
3. **The two ShapeButton IDs and exact toggle roles — UNKNOWN.** Not assigned at
   construction (§3); read them from FUN_00653010 / FUN_00654320 when A6 is
   scoped. Does not block A3.
4. **Radar-mirror adjust (`DAT_00a8eb7c`, `g_RadarViewportWidth`) semantics** in
   §2b/§2d only spot-confirmed as a left/right mirror correction; the exact
   mirror condition (e.g. observer/PiP mirrored radar) was not fully traced.
   Low risk for the camera-center contract; note for completeness.
5. **`DAT_008809a0` command-mode enum values** (build-place=0x14, plus
   0x25/0x26/0x29/0x41..0x48 action codes) are read from the Action switch but
   not cross-referenced to the canonical command-mode enum this session. The
   Rust port should reuse its existing command-mode enum rather than these raw
   constants.

---

### Verification call index (this session)

- `decompile 0x00652870`, `disassemble 0x00652870` — init, ctor args, vtable install, registration.
- `decompile 0x004e12f0` — GadgetClass__Constructor field-write order + sticky `|5`.
- `decompile 0x006539d0` — minimap Action body (gates, hit-test, left/right paths).
- `read_memory 0x007f02bc` (132B) vs `read_memory 0x007e92bc` (132B) — vtable diff (only slot 0 + slot 31 differ).
- `get_xrefs_to 0x006539d0` — single DATA xref @ 0x007F0338 = Action slot 31.
- `list_globals vtable__GadgetClass`, `list_globals vtable__RTacticalClass` — vtable label addresses.
- `decompile 0x004e1530`, `get_function_callers 0x004e1530`, `decompile 0x0048e5a0` — base Action vs ControlClass::Action label-drift; no result key from RTactical.
- `decompile 0x007c978a`, `decompile 0x007c970c` — static-init registration table (not Buttons list).
- `decompile 0x006528d5`, `decompile 0x00652925`, `disassemble` both — two radar ShapeButton inits.
- `get_function_by_address 0x0069dcf0`, `decompile 0x0069dcf0` — ShapeButtonClass__Constructor (zero geometry at ctor).
- `get_xrefs_to 0x00B04978`, `get_xrefs_to 0x00B04910` — radar buttons referenced by RadarClass__Draw + radar-mode fns (A6 scope).
- `list_globals RadarSurface`, `list_globals RadarViewport`, `list_globals DisplayChain`, `read_memory 0x00880c94` — geometry + gate globals.
