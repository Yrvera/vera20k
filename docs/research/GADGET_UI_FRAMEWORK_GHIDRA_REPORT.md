---
title: Gadget UI Framework (Ghidra Research Report)
date: 2026-04-22
---

# Gadget UI Framework — Ghidra Research Report

**Addresses (primary):**
- `GadgetClass::Input` @ `0x004E1640`
- `GadgetClass::Hit_Test` @ `0x004E15A0`
- `GadgetClass::Clicked_On` @ `0x004E13F0`
- `GadgetClass::Constructor` @ `0x004E12F0`
- `ControlClass::Action` @ `0x0048E5A0`
- `ControlClass::Constructor` @ `0x0048E520`
- `vtable__GadgetClass` @ `0x007E92BC`
- `vtable__ControlClass` @ `0x007E528C`
- `vtable__ListClass` @ `0x007ED10C`

**Confidence:** High for base GadgetClass, hit-test, input/draw dispatch, and
per-class Action overrides (ToggleClass, ListClass, DropListClass, CheckListClass
verified from decompiled Action/Draw/Peer callbacks). High for shell-dialog
framework classification.

**Active in YR:** Mixed (see Section 1.1).

**Revision history:**
- 2026-04-22 (initial): base hit-test + input dispatch + vtable mapping.
- 2026-04-22 (follow-up): corrected `vtable+0x6C` identification (it is
  `Draw_Me(forced)`, not a pre-dispatch hook — see Section 5.1). Added Sections
  6.1–6.5 with per-class Action/Draw internals. Added Section 12 (in-game
  dispatch chain).

---

## 1. Overview

gamemd.exe contains **two parallel UI frameworks**:

1. **GadgetClass hierarchy** — the Westwood WWlib "gadget" framework inherited
   from Tiberian Sun. Retained-mode widget tree backed by a doubly-linked
   `LinkClass` list; each gadget knows its own rect and flags. Used for **in-game**
   UI (sidebar cameos, tabs, radar chrome, text labels, shape buttons).

2. **DialogControl hierarchy** — a Win32-dialog-backed wrapper used for **shell**
   UI: Create Game lobby, Faction Select, Options, Load/Save, WOL/Westwood Online
   dialogs. These are native `HWND` modal windows driven by `DialogBoxParam` and a
   manual message pump (`OptionsClass__ShowInGameDialog` @ `0x004E1D00`).

Hit-test order and draw order are governed by the **GadgetClass** framework —
widgets owned by the same linked list compete on a "smaller area wins" rule with
linked-list order as the tie-breaker. This is the key parity anchor.

### 1.1 Class survey (what exists in gamemd.exe)

| Class | Present? | Address(es) | Notes |
|---|---|---|---|
| `GadgetClass` | Yes (HIGH) | ctor `0x004E12F0`, vtable `0x007E92BC` | Base. Linked-list retained-mode widget. |
| `ControlClass` | Yes (HIGH) | ctor `0x0048E520`, vtable `0x007E528C` | Adds `ID` + `Peer` pointer. |
| `ListClass` | Yes (HIGH) | dtor `0x00557600`, vtable `0x007ED10C` | Composes `SliderClass` + 2× `ShapeButtonClass` (up/down). |
| `DropListClass` | Yes (HIGH) | dtor `0x004B55F0`, vtable `vtable__DropListClass` | Composes `ListClass` + `ShapeButtonClass` + `EditClass`. |
| `EditClass` | Yes (HIGH) | dtor `0x004C30B0`, vtable `vtable__EditClass` | Text input. |
| `ColorListClass` | Yes (HIGH) | dtor `0x004887F0`, vtable `vtable__ColorListClass` | Derives from `ListClass`. |
| `CheckListClass` | Yes (HIGH) | dtor `0x004882D0`, vtable `vtable__CheckListClass` | Derives from `ListClass`. |
| `SliderClass` | Yes (HIGH) | dtor `0x006B1D00`, vtable `vtable__SliderClass` | Supports `GaugeClass` role. |
| `ShapeButtonClass` | Yes (HIGH) | dtor `0x0069DCF0`, vtable `vtable__ShapeButtonClass` | Derives `ToggleClass`. |
| `TextLabelClass` | Yes (HIGH) | ctor `0x0072A440`, vtable `vtable__TextLabelClass` | Derives `GadgetClass` (not Control). |
| **`GaugeClass`** | **NO** — not in binary | — | **See "TS ghosts" note.** Role filled by `SliderClass`. |
| **`Dial8Class`** | **NO** — not in binary | — | **See "TS ghosts" note.** Fully stripped from YR. |
| `FactionSelectDialogControl` | Yes (HIGH) | `0x007AF500` (WorldDominationTour subclass) | Win32-backed. |
| `LoadOptionsClass` | Yes (HIGH) | ctor `0x00558740`, dtor `0x0055A0D0` | Record-list for save/load dialog. Not a GadgetClass. |
| `DialogControl` / `SimpleDialogControl` / `SimpleWonlineDialogControl` | Yes (HIGH) | `0x00624130` / `0x00624110` / many | Win32 dialog wrappers for shell UI. |

**Tiberian Sun ghosts:** `GaugeClass` and `Dial8Class` do **NOT exist** in
gamemd.exe. String and function searches return nothing. In earlier Westwood
games they were dial-style rotating-needle controls; RA2/YR replaces them with
`SliderClass` (for scalar gauges) and had no need for directional dials. **Do NOT
implement GaugeClass or Dial8Class — they are not part of the YR parity surface.**

---

## 2. `GadgetClass` layout (byte offsets from `this`)

Verified from `GadgetClass::Constructor` @ `0x004E12F0` and accessor functions.

| Offset | Type | Field | Set by | Notes |
|---|---|---|---|---|
| `+0x00` | `void**` | `vtable` | ctor | `vtable__GadgetClass` for base; derived classes overwrite. |
| `+0x04` | `GadgetClass*` | `Next` | `LinkClass` base | Doubly-linked sibling list. |
| `+0x08` | `GadgetClass*` | `Prev` | `LinkClass` base | Zeroed by `GadgetClass::Constructor`. |
| `+0x0C` | `int` | `X` | ctor param_2 | Screen pixel. |
| `+0x10` | `int` | `Y` | ctor param_3 | Screen pixel. |
| `+0x14` | `int` | `Width` | ctor param_4 | |
| `+0x18` | `int` | `Height` | ctor param_5 | |
| `+0x1C` | `u8` | `IsToRedraw` | Flag_To_Redraw | Dirty flag. Read by `vtable+0x60` accessor `0x004886A0` (`mov al,[ecx+0x1C]`). |
| `+0x1D` | `u8` | `IsSticky` | ctor param_7 | If non-zero, OR `Flags` with `0x05`. |
| `+0x1E` | `u8` | `IsDisabled` | Enable/Disable | Non-zero = skipped by Hit_Test and Input. |
| `+0x20` | `u32` | `Flags` | ctor param_6 | Event-mask. Bit `0x100` = has keyboard focus. |

`ControlClass` extends:

| Offset | Type | Field | Notes |
|---|---|---|---|
| `+0x24` | `int` | `ID` | Value returned by `Action` (OR'd with `0x8000`). |
| `+0x28` | `GadgetClass*` | `Peer` | Optional sibling receiving vtable+0x4C callback on every Action. |

`LoadOptionsClass` (not a gadget, relevant for save/load dialog):

| Offset | Type | Notes |
|---|---|---|
| `+0x04` | | List head (inherits) |
| `+0x08` | `SaveRecord**` | Pointer table (inherits) |
| `+0x20` | `void*` | Vtable-ish: `PTR_FUN_007ED30C` / `PTR_FUN_007ED32C` |
| `+0x24` | `SaveRecord*` | Record buffer |
| `+0x28` | `int` | Record count |
| `+0x2C` | `u8` | `OwnsBuffer` |
| `+0x30` | `int` | `MaxVisible` = `10` |

---

## 3. Hit-test algorithm — `GadgetClass::Hit_Test` @ `0x004E15A0`

Pseudocode (verified):

```
fn hit_test(head: *Gadget, mx, my) -> *Gadget {
    let mut winner = NULL;
    // DAT_007F5BE8, DAT_007F5BF4 = screen width, height
    let mut best_w = SCREEN_W;
    let mut best_h = SCREEN_H;

    let mut g = head;
    while g != NULL {
        if g.IsDisabled == 0 {
            let w = g.Width;
            let h = g.Height;
            let inside = g.X <= mx && mx <  g.X + w
                      && g.Y <= my && my <  g.Y + h;
            // NB: the tie-breaker compares areas, and the
            // condition is `new_area <= saved_area` — i.e. on ties,
            // the LATER gadget in the list wins.
            let smaller_or_equal = (w * h) <= (best_w * best_h);
            if inside && smaller_or_equal {
                winner  = g;
                best_w  = w;
                best_h  = h;
            }
        }
        g = g.vtable.Get_Next();   // vtable+0x04
    }
    winner
}
```

**Parity invariants (the bits that must match exactly):**

- **Half-open rectangle**: `X <= mx < X+W`, `Y <= my < Y+H`. Right and bottom
  edges are NOT in bounds. Off-by-one on the right edge will desync hit-tests.
- **Smaller-area wins**. When two enabled gadgets both contain the point, the
  one with smaller `W*H` takes the hit. This is how a close button drawn on top
  of a list box takes priority even though it's a sibling, not a child.
- **Last-in-list wins on equal area**. The condition `new_area <= saved_area`
  (not strict `<`) means the later gadget in list-walk order replaces the
  earlier. Combined with drawing-in-list-order, this means "later-drawn siblings
  are on top" for same-sized widgets.
- **`IsDisabled` skips the gadget entirely** — it cannot be hit-tested. Disable
  is the only way to exclude a gadget from hit-testing (there is no `z_index`).
- **Initial reference size is screen W × H** — so the first in-bounds gadget
  always wins on the first iteration (screen-area >= any gadget's area).

---

## 4. Input dispatch — `GadgetClass::Input` @ `0x004E1640`

### 4.1 Global state

| Address | Symbol | Meaning |
|---|---|---|
| `0x008B3E8C` | `g_CurrentGadgetList` | Head of gadget list passed to the last `Input` call. If the caller passes a different head, state resets. |
| `0x008B3E88` | `g_StickyFocus` | Gadget that has captured ALL input (mouse + keyboard) until released. Overrides hit-test. |
| `0x008B3E90` | `g_KeyboardFocus` | Gadget that receives keyboard-only events (EditClass fields). Set by vtable+0x50 (`Set_Focus`), cleared by vtable+0x54 (`Clear_Focus`). |
| `0x008B3E94` | `g_HoveredGadget` | Result of the last `Hit_Test`. Used to drive Mouse_Enter / Mouse_Leave transitions. |

### 4.2 Event-flag bit layout (`Flags` arg passed to `Handle_Input`)

Confirmed by reading the classification block in `Input`:

| Bit | Meaning |
|---|---|
| `0x0001` | `LEFTPRESS` — left button just pressed (event `0x001` from queue) |
| `0x0002` | `LEFTHELD` — left button currently down (polled when no event) |
| `0x0004` | `LEFTRELEASE` — left button just released (event `0x801`) |
| `0x0008` | `LEFTUP` — left button not currently down (polled) |
| `0x0010` | `RIGHTPRESS` — right button just pressed (event `0x002`) |
| `0x0020` | `RIGHTHELD` — right button currently down (polled) |
| `0x0040` | `RIGHTRELEASE` — right button just released (event `0x802`) |
| `0x0080` | `RIGHTUP` — right button not currently down (polled) |
| `0x0100` | `KEYBOARD` — a keyboard key event (also the per-gadget "has keyboard focus" flag) |

Mouse-held / mouse-up bits are **assembled every call from polled button state**,
even when the event is a keyboard event. So a button receives the exact button-up
bit on the same tick it sees a keyboard press.

### 4.3 Dispatch order (the bit that matters for parity)

```
fn Input(list_head):
    if list_head != g_CurrentGadgetList:
        g_StickyFocus   = NULL
        g_KeyboardFocus = NULL
        g_CurrentGadgetList = list_head

    (event_code, mx, my) = read_input_queue()     # cached mouse pos for mouse events

    new_hover = Hit_Test(list_head, mx, my)       # smallest-area, last-in-list
    if new_hover != g_HoveredGadget:
        if g_HoveredGadget != NULL:
            g_HoveredGadget.vtable[0x74]()        # Mouse_Leave (RET stub in base)
        g_HoveredGadget = new_hover
        if new_hover != NULL:
            new_hover.vtable[0x70]()              # Mouse_Enter (RET stub in base)

    event_flags = classify(event_code, polled_button_state)

    # Three-tier dispatch — this order is load-bearing:
    if g_StickyFocus != NULL:
        # 1) Sticky gadget captures everything.
        g_StickyFocus.vtable[0x6C]()              # Draw_Me(forced=0) — draw-if-dirty pre-dispatch
        g_StickyFocus.vtable[0x80](event_flags, mx, my)   # Handle_Input
        g_StickyFocus.vtable[0x6C](0)             # Draw_Me(forced=0) — draw again if dispatch set dirty
        return event_code

    if g_KeyboardFocus != NULL and (event_flags & 0x100):
        # 2) Keyboard-focused gadget gets keyboard events only.
        g_KeyboardFocus.vtable[0x6C]()
        g_KeyboardFocus.vtable[0x80](event_flags, mx, my)
        g_KeyboardFocus.vtable[0x6C](0)
        return event_code

    # 3) Broadcast walk the list, stop at first consumer.
    g = list_head
    while g != NULL:
        g.vtable[0x6C](list_changed ? 1 : 0)      # Draw_Me(forced) — 1 on fresh list, 0 otherwise
        if g.IsDisabled == 0:
            if g.vtable[0x80](event_flags, mx, my) != 0:   # Handle_Input
                break
        g = g.vtable[0x04]()                      # Get_Next
    return event_code
```

**CRITICAL:** `vtable+0x6C` is **`Draw_Me(forced: bool)`**, not a pre-dispatch
hook. The argument is whether to force-draw even when `IsToRedraw == 0`. So
**`GadgetClass::Input` is also the draw loop** — every input tick walks the
gadget list calling `Draw_Me` on each, and drawing happens at the same cadence
as dispatch. The sticky/focus branches call `Draw_Me` twice (before and after
dispatch) so that a click's press-visualization lands on the same tick as the
click itself. The list-walk branch only pre-draws each gadget; gadgets past the
event consumer don't re-draw this tick (their dirty flags persist).

**Invariants:**

- **Sticky focus > keyboard focus > list walk.** Never short-circuit this order.
- **Hover transitions fire BEFORE dispatch.** A click that moves hover to a new
  gadget still delivers Mouse_Enter to the new one before dispatching the click.
- **Handle_Input (vtable+0x80) is called in list order**, not hit-test order.
  The first gadget that returns non-zero consumes the event. This is NOT
  "hit-test → dispatch" — it's "broadcast until consumed." Hit-test is used only
  to update hover and to let individual gadgets' `Handle_Input` check bounds
  (via `Clicked_On`).
- **`Handle_Input` on the base `GadgetClass` is `Clicked_On` (vtable+0x80 = `0x004E13F0`).**
  It masks the event flags by the gadget's `Flags`, and dispatches only if the
  masked event is non-zero AND (gadget is sticky OR point is in bounds).

### 4.4 `Clicked_On` / `Action` — how events fire

`GadgetClass::Clicked_On` @ `0x004E13F0`:

```
fn Clicked_On(this, key, event_flags, mx, my, ...):
    event_flags &= this.Flags          # honor the gadget's event mask
    if this != g_StickyFocus
       and (event_flags & 0x100) == 0  # non-keyboard
       and (event_flags == 0
            or (mx - this.X) >= this.Width
            or (my - this.Y) >= this.Height):
        return 0                       # out of bounds or not interested
    return this.vtable[0x7C](event_flags, key, ...)  # Action
```

`ControlClass::Action` @ `0x0048E5A0` (overrides GadgetClass::Action):

```
fn Action(this, event_flags, out_result):
    if event_flags != 0:
        if this.ID == 0:
            *out_result = 0
        else:
            *out_result = this.ID | 0x8000        # KN_BUTTON marker
            if (event_flags & 0x40) != 0 and (this.Flags & 0x10) != 0:
                # RIGHTRELEASE with RIGHTPRESS in mask → also set 0x4000
                *out_result = (*out_result) | 0x4000
    if this.Peer != NULL:
        this.Peer.vtable[0x4C](event_flags, out_result, this)
    GadgetClass::Action_Continuation(event_flags, out_result, 0)
```

**Action returns the gadget ID OR'd with `0x8000`.** This is the KN_BUTTON marker
used by the TS/RA2 `KeyNumType` to distinguish button IDs from keyboard keys.
Right-release + right-press-in-mask additionally sets `0x4000` — a well-defined
"right-click" marker code.

---

## 5. Draw order

There is **no explicit Z-index field on GadgetClass.** Draw order is entirely
determined by linked-list walk order, AND drawing is driven by the same
`Input()` call that dispatches events.

### 5.1 `Draw_Me` lives in the vtable at `+0x6C`

**Base `GadgetClass::Draw_Me`** @ `0x004E1550`:

```
fn Draw_Me(this, forced):
    if forced == 0 and this.IsToRedraw == 0:
        return 0                # nothing to draw
    this.IsToRedraw = 0         # clear dirty flag
    return 1                    # "go ahead, subclass, draw yourself"
```

Derived classes override `vtable+0x6C` to do the actual rendering. Each override
starts by calling the base (or `ControlClass::Draw_Me`, which chains to the
`Peer`) — if that returns 0, the derived class skips drawing entirely.

**`ControlClass::Draw_Me`** @ `0x0048E620` extends this with peer chaining:

```
fn Draw_Me(this, forced):
    if this.Peer != NULL:
        this.Peer.vtable[0x6C](0)           # draw peer first
    return GadgetClass::Draw_Me(this, forced)
```

**`ShapeButtonClass::Draw_Me`** @ `0x0069DEB0` (the core in-game-button draw)
picks one of five SHP frames based on state:

| Frame | Condition |
|---|---|
| `0` | Normal (not disabled, not hovered, push-button) |
| `1` | Hovered (push-button `+0x2C != 0`) |
| `2` | Disabled (`+0x1E != 0`) — overrides everything |
| `3` | Toggle OFF (toggle-mode `+0x40 != 0`, `+0x2D == 0`, third-state bit `+0x34`) |
| `4` | Toggle ON (toggle-mode, `+0x2D != 0`, third-state bit) |

The target surface is `g_SidebarSurface` if `+0x4C != 0`, else `g_PrimarySurface`.
This is how the sidebar gets its own render surface without the rest of the UI.

### 5.2 List / drop-list draws delegate to `vtable+0x90 / +0x94`

`ListClass::Draw_Me` and `DropListClass::Draw_Me` use two additional vtable
slots on their own class:

- **`vtable+0x90` = Draw_List_Background** — the framed rectangle, scrollbar
  track, item-row rectangles.
- **`vtable+0x94` = Draw_Selected_Item / Draw_Entry** — per-row rendering.
  Subclasses like `ColorListClass` and `CheckListClass` override this to draw
  color swatches / check-boxes next to the row text without touching the outer
  layout.

### 5.3 Dirty-flag propagation

- `Flag_To_Redraw` (vtable+0x48 @ `0x004E1960`) sets `+0x1C = 1`.
- `Draw_Me` clears `+0x1C` when it actually draws.
- `Any_Redraw_Pending` (vtable+0x5C @ `0x004E1A00`) walks the list, returns 1
  if any gadget still has `+0x1C == 1`. Not actually required (Input calls
  `Draw_Me` on every gadget anyway), but useful to short-circuit out of a frame
  that has no UI changes.
- `Enable` / `Disable` force `+0x1C = 1` unconditionally (so enabling a
  previously-hidden gadget triggers a redraw on the next tick).

### 5.4 Implications for parity

- Adding order **IS** the Z-order — there is no separate `z_index` field.
- Because `Hit_Test` prefers "last-in-list on equal area" AND draw walk goes
  head-to-tail, **later-added gadgets are drawn ON TOP and are hit FIRST** for
  same-sized siblings. Rust port must match this direction.
- **Drawing and input are coupled.** Our Rust sim should not separate them into
  `draw()` + `process_input()` unless we want a free-running render thread.
  Matching original cadence means: every input-poll tick also draws.
- **No full-frame clear** — the UI redraws only the gadgets whose `IsToRedraw`
  is set. Our Rust UI framework should support per-widget dirty flags to match
  the scheduling of animated widgets (pulsing cameos, flashing build-queue).

---

## 6. Per-class notes

### 6.1 `ControlClass` (in-memory at +0x24 / +0x28)

Extends `GadgetClass` with `ID (+0x24)` and `Peer (+0x28)`. Constructor
@ `0x0048E520` passes `Sticky=1, Flags=5` to `GadgetClass::Constructor` — so
all ControlClass derivatives default to sticky with `FLAG_LEFTPRESS |
FLAG_LEFTRELEASE` in the event mask. Overrides: `vtable[0x44]` `Get_ID`
(returns `+0x24`), `vtable[0x6C]` `Draw_Me` (peer-chained), `vtable[0x7C]`
`Action` (posts `ID | 0x8000`), `vtable[0x84]` `Set_Peer`.

### 6.2 `ToggleClass` / `ShapeButtonClass` — the sidebar button backbone

`ToggleClass::Action` @ `0x00723EC0` is the core button state machine. It's
inherited by every `ShapeButtonClass` in the sidebar.

**Additional fields** (byte offsets from `this`):

| Offset | Field | Meaning |
|---|---|---|
| `+0x2C` | `IsPressed` | mouse is currently pressing this button |
| `+0x2D` | `IsToggled` | toggle button's on/off state |
| `+0x30` | `ButtonKind` | 0 = push, 1 = radio-style, 2 = toggle |
| `+0x34` | `IsThirdState` | selects frames `3..4` range (toggles only) |
| `+0x40` | `IsToggleable` | 0 = push, 1 = toggle/radio |
| `+0x48` | `ShapeOffsetX` | X offset for SHP draw within gadget rect |
| `+0x4C` | `UseSidebarSurface` | draw target flag |
| `+0x54` | `WasDrawnThisTick` | |
| `+0x55` | `DrawFlags` | bit `0x800` = a shape-draw flag passed to CC_Draw_Shape |
| `+0x58` | `Shape` | pointer to SHP asset |
| `+0x5C` | `OwnsShape` | free shape on destruction |

**Hover tracking (event_flags == 0, mouse move):**

```
in_hover = (mx - this.X) < this.Width
        && (my - this.Y) < this.Height   # unsigned compare, so mx < X fails too
if !in_hover and this.IsPressed:
    this.IsPressed = 0
    Flag_To_Redraw()
elif in_hover and !this.IsPressed:
    this.IsPressed = 1
    Flag_To_Redraw()
```

**LEFTPRESS / RIGHTPRESS** (`event_flags & 0x11`):

```
this.IsPressed = 1
Flag_To_Redraw()
# Acquire sticky capture — stripping the press bit so the capture handler
# doesn't re-fire this press as a new event.
Route_Event(event_flags & ~0x11, out_result, 0)   # FUN_0048E550
*out_result = 0
return 1                                          # consumed
```

**LEFTRELEASE / RIGHTRELEASE** (`event_flags & 0x44`):

```
if !this.IsPressed or !in_hover:
    # dragged off before release — cancel, no action fires
    clear IsPressed, Flag_To_Redraw, return
if this.ButtonKind == 1:
    this.IsToggled = (was_unchecked ? 1 : 0)     # single-select behavior
elif this.ButtonKind == 2:
    this.IsToggled ^= 1                           # true toggle
this.IsPressed = 0
Flag_To_Redraw()
# Route the click with release bits cleared so downstream sees "just fired"
Route_Event(event_flags & ~0x44, out_result, 0)
```

**Parity invariants:**
- Buttons fire on RELEASE only, not PRESS (matches Windows convention and
  cancel-on-drag-off).
- Dragging off the button before release cancels the fire — classic Westwood.
- PRESS acquires sticky-capture so the release event reaches the same button
  even if the mouse moves over another gadget.

### 6.3 `ListClass` — item layout and row-click math

`ListClass::Action` @ `0x00557830` and `ListClass::Get_Item` @ `0x00557AC0`
reveal the field layout:

| Offset | Field (index) | Meaning |
|---|---|---|
| `+0x38` | `Items` (0x0E) | `T**` pointer to item table (owned via `+0x41`) |
| `+0x44` | `ItemCount` (0x11) | number of rows |
| `+0x4C` | `RowHeight` (0x13) | pixel height per row |
| `+0x16C` | `SelectedIndex` (0x5B) | clamped to `[0, ItemCount-1]` |
| `+0x170` | `TopVisibleRow` (0x5C) | scroll offset |
| `+0x174` | `RowPainter` (0x5D) | ColorList: function-pointer table @ `0x007E4DB8`; CheckList: `IsReadOnly` byte |
| `+0x12C` | `LabelWidth` (0x4B) | affects inner vs outer list width |

**Click-to-select formula** (from `ListClass::Action`):

```
my = g_Display.Get_Mouse_Y()
row_index = ((my - this.Y) - 1) / this.RowHeight + this.TopVisibleRow
# clamp
if ItemCount - 1 <= row_index: row_index = ItemCount - 1
if row_index == -1:            row_index = 0
this.SelectedIndex = row_index
# then call GadgetClass::Action to fire the event
```

**Keyboard scroll:**

- `VK_UP (0x26)` → `this.vtable[0xA4](-1)` — scroll up one row
- `VK_DOWN (0x28)` → `this.vtable[0xA4](1)` — scroll down one row
- `LEFTRELEASE (0x04)` is a no-op (dispatched for completeness, no mutation)

**Get_Item bounds quirk:** returns `NULL` if `ItemCount == 0`, otherwise
clamps the index to `ItemCount - 1` on the HIGH end, but does **not** clamp
on the low end — `Get_Item(-1)` returns `Items[-1]` (undefined). The
`Action` path always clamps `SelectedIndex >= 0` before calling `Get_Item`,
so this is only an issue for direct callers.

### 6.4 `CheckListClass` — toggle-on-press

`CheckListClass::Action` @ `0x004884A0`:

```
if this.IsReadOnly (+0x174):                    # field 0x5D
    return 0
result = ListClass::Action(event_flags, out_result, 0)  # select a row
if event_flags & LEFTPRESS (0x01):
    rec = ListClass::Get_Item(this.SelectedIndex)
    if rec != NULL:
        rec.Checked (+0x04) ^= 1                # toggle
        this.Flag_To_Redraw()
return result
```

**Item record layout** (allocated by `CheckListClass::Add` @ `0x00488310`,
which does `op new(8); *rec = text_ptr; rec[+4] = 0`):

| Offset | Type | Meaning |
|---|---|---|
| `+0x00` | `char*` | item text |
| `+0x04` | `bool` | checked state |

Checked state changes on **LEFTPRESS**, not RELEASE — so the box flips the
moment you click, matching original responsiveness.

### 6.5 `DropListClass` — expand/collapse via list injection

`DropListClass::Peer_Callback` (vtable+0x4C) @ `0x004B50C0` is the heart of the
expand/collapse state machine.

**Additional fields:**

| Offset | Field (index) | Meaning |
|---|---|---|
| `+0x34` | `EditBuffer` (0x0D) | `char*` to selected-item text |
| `+0x38` | `EditCapacity` (0x0E) | inner edit's max length |
| `+0x48` | `IsExpanded` (0x12) | byte — is the drop-down visible? |
| `+0x50` | `DropButton` (0x14) | inner `ShapeButtonClass` (the arrow) |
| `+0xB0` | `InnerList` (0x2C) | inner `ListClass*` — the dropdown |
| `+0xBC..+0xC8` | `ExpandedRect` (0x2F..0x32) | X, Y+18, W, H of open list |
| `+0xF0` | `EditLength` (0x3C) | strlen cache |

**Drop button clicked (event_flags & LEFTRELEASE, sender == DropButton):**

```
if !this.IsExpanded:
    # EXPAND: position inner list 18 px below self and add to gadget list
    this.InnerList.SetRect(this.X, this.Y + 0x12, this.Width, this.InnerHeight)
    parent = this.vtable[0x18]()                  # LinkClass::Get_Link (returns prev-sibling)
    parent.vtable[0x0C](this.InnerList)            # LinkClass::Add_Tail (inject into list)
    this.InnerList.Flag_To_Redraw()
    this.IsExpanded = 1
else:
    # COLLAPSE: unlink inner list and post the Action
    this.InnerList.vtable[0x24]()                  # LinkClass::Remove (detach from list)
    this.IsExpanded = 0
    *out_result = this.ID | 0x8000                 # post the event
```

**Inner list fires (sender == InnerList):**

```
text = this.InnerList.vtable[0x9C](this.Slot)      # Get_Item_Text
strcpy(this.EditBuffer, text)
this.Flag_To_Redraw()
*out_result = this.ID | 0x8000
```

**Focus loss auto-collapses:** `DropListClass::Clear_Focus` (vtable+0x54)
@ `0x004B50A0` removes the inner list from the gadget list if expanded — so
clicking away closes the drop-down automatically.

**Parity invariants:**
- The drop-down is an actual `ListClass` **inserted into the active gadget
  linked list** during expand. It's NOT a special popup layer. Hit-test and
  Z-order follow the usual rules.
- The drop-down's Y is `self.Y + 0x12` (18 pixels below self's top-left).
- Losing keyboard focus (e.g. clicking outside) collapses the drop-down — the
  Rust port needs the same "click-outside-dismisses" behavior.
- Keyboard input while expanded flows through `vtable+0x98` (`Handle_Char`) on
  the DropListClass, which forwards to the inner `EditClass` for typeahead.

### 6.6 `EditClass`

Derives from `ControlClass`. Constructor @ `0x004C30B0` calls `Has_Focus`
and `Clear_Focus` to ensure this gadget isn't stale in the global focus
slot before binding. Participates in the `g_KeyboardFocus` system —
clicking an edit takes focus; clicking elsewhere (or ESC) releases it.

### 6.7 `ColorListClass` — row painter indirection

Derives from `ListClass`. Each row paints a color swatch preceding the text.
The swatch painter lives at `+0x174` (field 0x5D) as a pointer into
`PTR_FUN_007E4DB8` — a 16-entry vtable-like structure of shape-drawing
helpers in the `0x00477xxx` range (color-block rendering to a clipped rect).
No separate row-toggle behavior beyond `ListClass::Action`.

### 6.8 `TextLabelClass`

Derives directly from `GadgetClass` (NOT Control). Constructor @ `0x0072A440`
creates a **1×1 marker rect** at the requested (X, Y) — so labels are never
hit-tested meaningfully, but they're still in the linked list for drawing.
Stores color, font, x/y offset, STT string key.

### 6.9 `SliderClass`

Fills the "GaugeClass / progress bar" role for YR. Contains two sub-buttons
(up/down arrows) freed during destruction. Also used internally by `ListClass`
as the scrollbar.

---

## 7. Shell dialogs (Create Game / Faction Select / Load Options)

**RA2/YR shell dialogs are Win32 native dialog boxes, NOT GadgetClass trees.**

Evidence:

- `OptionsClass::ShowInGameDialog` @ `0x004E1D00` creates an `HWND`, uses
  `SetWindowLongA` to attach a result pointer, runs a manual message-pump loop
  via `FUN_00623120()`, and renders the game in the background while modal.
- `FactionSelectDialogControl::WorldDominationTour` @ `0x007AF500` owns three
  sub-widgets (destructor calls their scalar-deleting dtors) and has vtable
  `vtable__FactionSelectDialogControl__WorldDominationTour`. It derives from
  `DialogControl::OwnerDraw` (`0x00624130`), a Win32 `WM_DRAWITEM`
  owner-draw helper.
- `SimpleDialogControl::OwnerDraw` / `SimpleWonlineDialogControl` are the base
  Win32 wrappers for most WOL and skirmish shell controls.
- `LoadOptionsClass` @ `0x00558740` is NOT a GadgetClass — it's a bounded list
  of save-game records (max visible = `10`, record buffer at `+0x24`). Used to
  populate a Win32 listbox in the save/load dialog.
- String-table keys in binary (e.g. `STT:SkirmishButtonStartGame`,
  `STT:SkirmishComboCountry`, `STT:SkirmishSliderCredits`) are the
  dialog-control identifiers — Win32 dialog items referenced by string key.
- `RulesClass::ReadMultiplayerDialogSettings` @ `0x00671EA0` populates the
  skirmish-dialog defaults from `rules(md).ini [MultiplayerDialogSettings]`.
  These defaults drive the Win32 dialog's initial control state.

**Confirmed INI → dialog-state mapping** (from `RulesClass::ReadMultiplayerDialogSettings`):

| INI key | Field offset | Type | Control type |
|---|---|---|---|
| `MinMoney` | `+0x1480` | int | Slider min |
| `Money` | `+0x1484` | int | Slider current |
| `MaxMoney` | `+0x1488` | int | Slider max |
| `MoneyIncrement` | `+0x148C` | int | Slider step |
| `MinUnitCount` | `+0x1490` | int | Slider min |
| `UnitCount` | `+0x1494` | int | Slider current |
| `MaxUnitCount` | `+0x1498` | int | Slider max |
| `TechLevel` | `+0x149C` | int | Slider |
| `GameSpeed` | `+0x14A0` | int | Slider/combo |
| `AIDifficulty` | `+0x14A4` | int | Radio/combo |
| `AIPlayers` | `+0x14A8` | int | Spinner/combo |
| `BridgeDestruction` | `+0x14AC` | bool | Checkbox |
| `ShadowGrow` | `+0x14AD` | bool | Checkbox |
| `Shroud` | `+0x14AE` | bool | Checkbox |
| `Bases` | `+0x14AF` | bool | Checkbox |
| `TiberiumGrows` | `+0x14B0` | bool | Checkbox |
| `Crates` | `+0x14B1` | bool | Checkbox |
| `CaptureTheFlag` | `+0x14B2` | bool | Checkbox |
| `HarvesterTruce` | `+0x14B3` | bool | Checkbox |
| `MultiEngineer` | `+0x14B4` | bool | Checkbox |
| `AlliesAllowed` | `+0x14B5` | bool | Checkbox |
| `ShortGame` | `+0x14B6` | bool | Checkbox |
| `FogOfWar` | `+0x14B7` | bool | Checkbox |
| `MCVRedeploys` | `+0x14B8` | bool | Checkbox |
| `SuperWeaponsAllowed` | `+0x14B9` | bool | Checkbox |
| `BuildOffAlly` | `+0x14BA` | bool | Checkbox |
| `AllyChangeAllowed` | `+0x14BB` | bool | Checkbox |

**Implications for the Rust port:**

- **In-game UI** (sidebar, radar, EVA chrome, build queues, tooltips) is the
  parity target for the gadget hit-test rules in Sections 3–5.
- **Shell UI** (main menu, skirmish/MP setup, faction select, save/load) was
  Win32-native in the original. We do NOT need to replicate Win32 message
  pumping or dialog resources — we can build these dialogs using any retained-
  or immediate-mode strategy as long as we hit the same INI-driven behavior and
  produce the same outgoing skirmish settings.

---

## 8. vtable layout — `GadgetClass` (base) @ `0x007E92BC`

Byte offsets are into the vtable (not into `this`). Addresses confirmed by
reading vtable memory.

| Slot | Off | Address | Function |
|---|---|---|---|
| 0 | `+0x00` | `0x004E1A60` | `~GadgetClass` (scalar deleting) |
| 1 | `+0x04` | `0x004E14A0` | `Get_Next` (thunk → `LinkClass::Get_Next @ 0x00556620`) |
| 2 | `+0x08` | `0x004E14B0` | `Get_Prev` (thunk) |
| 3..8 | `+0x0C..+0x20` | `0x005566A0..0x005565F0` | `LinkClass` list manipulation |
| 9 | `+0x24` | `0x004E1480` | `Unlink` (calls vtable+0x54 then `LinkClass::Remove @ 0x00556730`) |
| 10 | `+0x28` | `0x004E1640` | **`Input`** (main entry point) |
| 11 | `+0x2C` | `0x004E1570` | `Flag_List_To_Redraw` (walks list calling vtable+0x6C) |
| 12 | `+0x30` | `0x004E14C0` | `Delete_List` (walks to head, destroys each) |
| 13 | `+0x34` | `0x004E1920` | `Extract_Gadget` (by ID — calls vtable+0x44) |
| 14 | `+0x38` | `0x00488690` | `Clear_Attached_List` (writes 0 to `g_CurrentGadgetList`) |
| 15 | `+0x3C` | `0x004E1460` | `Disable` (sets `IsDisabled=1`, flags redraw) |
| 16 | `+0x40` | `0x004E1450` | `Enable` (sets `IsDisabled=0`, flags redraw) |
| 17 | `+0x44` | `0x004AEBA0` | `Get_ID` (base returns 0) |
| 18 | `+0x48` | `0x004E1960` | **`Flag_To_Redraw`** (`+0x1C = 1`) |
| 19 | `+0x4C` | `0x0048E650` | `Peer_Callback` (base: RET 0xC — no-op with 3 args) |
| 20 | `+0x50` | `0x004E19A0` | **`Set_Focus`** (clears old focus, sets `g_KeyboardFocus = this`, sets flag bit `0x100`) |
| 21 | `+0x54` | `0x004E19D0` | **`Clear_Focus`** (clears flag bit `0x100`, clears `g_KeyboardFocus`) |
| 22 | `+0x58` | `0x004E19F0` | `Has_Focus` (returns `this == g_KeyboardFocus`) |
| 23 | `+0x5C` | `0x004E1A00` | `Any_Redraw_Pending` (walks list, returns 1 if any `IsToRedraw`) |
| 24 | `+0x60` | `0x004886A0` | `Get_IsToRedraw` (`mov al,[ecx+0x1C]`) |
| 25..26 | `+0x64..+0x68` | `0x004E1A20..0x004E1A40` | `Get_Rect` / `Set_Size` (`*(+0x14) = W; *(+0x18) = H`) |
| 27 | `+0x6C` | `0x004E1550` | **`Draw_Me(forced)`** (base: clears `+0x1C`, returns 1; ControlClass override peer-chains; derived classes do actual rendering) |
| 28 | `+0x70` | `0x004E1510` | **`Mouse_Enter`** (base: RET — override in derived) |
| 29 | `+0x74` | `0x004E1520` | **`Mouse_Leave`** (base: RET — override in derived) |
| 30 | `+0x78` | `0x004E1970` | `Sticky_Process` (reads `+0x1D`) |
| 31 | `+0x7C` | `0x004E1530` | **`Action`** (base: if event≠0 call vtable+0x78 and return 1) |
| 32 | `+0x80` | `0x004E13F0` | **`Clicked_On`** / `Handle_Input` |
| 33 | `+0x84` | `0x00800AE0` | (data-seg address — likely vtable terminator) |
| 34 | `+0x88` | `0x004E1AD0` | `LinkClass` helper (scalar-deleting wrapper) |

`ControlClass` vtable @ `0x007E528C` overrides:

- Slot 0 (`+0x00`): `~ControlClass` @ `0x0048E660`.
- Slot 17 (`+0x44`) `Get_ID`: `0x0048E610` → `mov eax, [ecx+0x24]; ret`.
- Slot 27 (`+0x6C`) `Draw_Me`: `0x0048E620` — forwards to `Peer.vtable+0x6C(0)` then calls base.
- Slot 31 (`+0x7C`) `Action`: `0x0048E5A0` — posts `ID | 0x8000` (and `|0x4000` on right-release).
- Slot 33 (`+0x84`) `Set_Peer`: `0x0048E600` → `mov [ecx+0x28], eax`.

---

## 9. Current Rust implementation status

From the scan (`src/ui/`, `src/sidebar/`, `src/app_spawn_pick.rs`):

- **GadgetClass framework**: NOT IMPLEMENTED. No widget tree, no `Gadget` trait, no
  linked-list sibling management, no sticky/keyboard focus model.
- **In-game sidebar** has its own bespoke hit-testing at
  `src/sidebar/mod.rs` (`compute_hit_test` around lines 294–345). The current
  implementation walks a flat `Vec` of rects and returns first match — it does
  NOT implement the smaller-area-wins tie-breaker. This is OK for the current
  sidebar (cameos never overlap), but will diverge the moment we add tooltips,
  dropdown controls, or overlapping widgets.
- **Shell UI** uses egui `ComboBox`, `Checkbox`, `Slider` via `src/ui/main_menu.rs`,
  `src/ui/pause_menu.rs`, `src/ui/in_game_hud.rs`. This is acceptable substitute
  for the Win32 shell dialogs provided we drive them from the same
  `[MultiplayerDialogSettings]` defaults — but **we are not reading those
  defaults yet.**
- **No `LoadOptionsClass` equivalent** — no save/load dialog at all.
- **No `FactionSelectDialogControl` equivalent** — country picker is defined but
  hidden (`SkirmishCountry` enum present in `main_menu.rs` lines 39–107, no UI
  surface).
- **Keyboard focus system**: not implemented. No Rust equivalent of
  `g_KeyboardFocus` or `Set_Focus` / `Has_Focus`. egui handles this internally
  for its own widgets only.

---

## 10. Parity implications (where this matters most)

Items ranked by "player can see it":

1. **Sidebar-to-tactical hit-test boundary.** Confirmed in CLAUDE.md as a parity
   issue. If we ever have two overlapping sidebar regions (e.g., tooltip + tab
   button), the smaller-area-wins rule must be honored. Current Rust sidebar
   does not — it returns the first-in-vec match.
2. **Build queue / cameo hold behavior.** `vtable+0x6C` hooks fire before and
   after dispatch — this is where the "cameo pulsing stops the moment you press"
   animation lives. Replicating needs pre/post hooks in our cameo dispatcher.
3. **Right-click semantics.** `ControlClass::Action` distinguishes RIGHTRELEASE
   from all other events via the `0x4000` bit. Sidebar cancel-production on
   right-click MUST check for the exact RIGHTRELEASE flag, not just "any right
   event," or repeat-right-clicks will double-trigger.
4. **Shell dialog defaults.** `[MultiplayerDialogSettings]` drives all skirmish
   slider/checkbox defaults. Any deviation from these defaults makes our skirmish
   setup feel wrong on first launch.
5. **EditClass focus.** Text input (player name, save-game-name) requires a
   keyboard-focus model. egui handles this for its own widgets, but if we ever
   move to a custom chrome-matching shell UI, we need the two-tier `g_KeyboardFocus`
   vs `g_StickyFocus` distinction.
6. **Hover enter/leave events.** Fires BEFORE dispatch. If we implement tooltips
   with a delay timer, the timer must start on Mouse_Enter, not on the first
   Mouse_Hover tick.

---

## 11. Open questions

- **Exact top-level caller of `GadgetClass::Input`**: confirmed it is only
  reachable via vtable dispatch (no symbolic callers). The in-game dispatch
  chain (Section 12) consumes the Action-ID result downstream, but the call
  site that does `result = list_head.vtable[0x28](...)` wasn't located
  symbolically. Likely inside `Keyboard::Check_Input` or `Main_Loop::AI`;
  finding it matters only if we want to time-slice the UI call relative to
  the game tick.
- **Anything inside `ShapeButtonClass` that further specializes `Clicked_On`**:
  `ShapeButtonClass` inherits `GadgetClass::Clicked_On` (vtable+0x80 unchanged
  at `0x004E13F0`) and only overrides `Action` (via `ToggleClass`) and
  `Draw_Me`. So the hit-test + bounds filter is pure base behavior — verified.
- **`GaugeClass` / `Dial8Class` absence**: verified missing from binary via
  function-name search and string search. If a future parity audit surfaces
  either name, re-check — but the current evidence is unambiguous.
- **`Route_Event` helper at `0x0048E550`**: called by `ToggleClass::Action`
  during press/release. Not decompiled further (it's shared with
  `ControlClass::Constructor`'s address neighborhood; likely a trampoline that
  sets `g_StickyFocus = this` and re-dispatches). Not blocking.
- **`CheckListClass::Add` memory management**: items are allocated with
  `operator new(8)` per row and stored in an item table. Not verified whether
  the list frees items on `Remove`, or only on destructor (`Destroy_All_Items`
  loop seen at `0x004882D0`). Matters for save/load of a multi-check dialog.

## 12. In-game dispatch chain (new — follows from Section 4)

Although `GadgetClass::Input` has no symbolic callers, the downstream consumer
chain for in-game events is fully mapped. When a sidebar button is released,
`ControlClass::Action` writes `ID | 0x8000` into the out-parameter, and the
dispatch cascade is:

```
<main loop> → GadgetClass::Input(list_head)     # returns ID|0x8000
            ↓
            FUN_005BDDC0  (radar animation tick)        @ 0x005BDDC0
            ↓
            DisplayClass::Dispatch (param2 = &id, param3 = &cell)  @ 0x006922E0
            ↓
            CommandBar_Dispatch                                     @ 0x006D0680
                - routes IDs `0x80D6..0x80EE` → command-bar buttons
                  (move/attack/force-fire/etc modes)
                - `0x80F0` → sell-mode toggle
                - `0x80F1` → repair-mode toggle
                - `0xC0D6..0xC0EE` → control-group key with CTRL modifier
                - fallback → SidebarClass::Action
            ↓
            SidebarClass::Action                                    @ 0x006A7780
                - routes IDs `0x80CB..0x80CE` → tab switches (4 tabs)
                - `0x8065` / `0x8066` → strip scroll up/down
                - per-strip cameo IDs → StripClass::AI dispatch
                - then calls PowerClass::AnimationTick
```

**Key observations:**

- **The range `0x8000..0x80FF` is the in-game action-ID namespace.** IDs in
  this range are posted by `ControlClass::Action` and interpreted as button
  clicks. `0x8000` is the KN_BUTTON marker bit; the low byte is the actual
  button ID.
- **`0xC000..0xC0FF` adds the `0x4000` flag** — set by `ControlClass::Action`
  on RIGHTRELEASE when the gadget's flag mask includes `0x10` (RIGHTPRESS).
  The command bar uses this to distinguish "left-click control group" from
  "right-click control group" (center-on-group).
- **Tab / tooling / queue buttons all post through the same Action path.**
  There's no special "sidebar event bus" — it's just gadgets with IDs,
  dispatched by a `switch`-on-ID chain.
- **`CommandBar_Dispatch` runs FIRST, then `SidebarClass::Action`.** This
  means the command-bar tools (move, attack, force-fire, sell, repair) take
  priority over sidebar button IDs when there's a namespace collision. For
  parity, the Rust port should preserve this order: command-bar-first, then
  sidebar.

**Rust port implication:** our sidebar hit-test handler currently returns
directly. To match original cadence, the flow should be:

1. Hit-test sidebar → get gadget + event flags.
2. Call gadget's `Action` → get action ID (or keyboard key).
3. Dispatch through `command_bar_dispatch(id)` first.
4. If not consumed, dispatch through `sidebar_action(id)`.
5. Update power-bar / credits-counter animations at end.

---

## Sources

**Ghidra addresses decompiled:**
- `0x004E1640 GadgetClass::Input`
- `0x004E15A0 GadgetClass::Hit_Test`
- `0x004E13F0 GadgetClass::Clicked_On`
- `0x004E12F0 GadgetClass::Constructor`
- `0x004E1390 / 0x004E1A60 GadgetClass::~GadgetClass`
- `0x0048E5A0 ControlClass::Action`
- `0x0048E520 ControlClass::Constructor`
- `0x0048E610 ControlClass::Get_ID` (thunk)
- `0x0048E620 ControlClass::Draw_Me` (peer-chained)
- `0x0048E600 ControlClass::Set_Peer`
- `0x0048E650 GadgetClass::Peer_Callback` (RET 0xC stub)
- `0x004E1460 Disable`, `0x004E1450 Enable`
- `0x004E1480 Unlink`, `0x004E14C0 Delete_List`, `0x004E14A0 Get_Next` (thunk),
  `0x004E14B0 Get_Prev` (thunk)
- `0x004E1510 Mouse_Enter` (RET stub), `0x004E1520 Mouse_Leave` (RET stub)
- `0x004E1530 Action`, `0x004E1550 Draw_Me (base)`, `0x004E1570 Flag_List_To_Redraw`
- `0x004E1920 Extract_Gadget`, `0x004E1960 Flag_To_Redraw`, `0x004E1970 Sticky_Process`
- `0x004E19A0 Set_Focus`, `0x004E19D0 Clear_Focus`, `0x004E19F0 Has_Focus`,
  `0x004E1A00 Any_Redraw_Pending`, `0x004E1A40 Set_Size`, `0x004E1AD0 LinkClass_Stub`
- `0x004AEBA0 GadgetClass::Get_ID` (return 0)
- `0x00488690 Clear_Attached_List`, `0x004886A0 Get_IsToRedraw`
- `0x00557600 / 0x00558060 ListClass::~ListClass`
- `0x00557830 ListClass::Action`
- `0x00557920 ListClass::Draw_Me` (raw bytes inspected, not full decompile)
- `0x00557AC0 ListClass::Get_Item`
- `0x004B55F0 DropListClass::~DropListClass`
- `0x004B50C0 DropListClass::Peer_Callback` (expand/collapse state machine)
- `0x004B50A0 DropListClass::Clear_Focus` (auto-collapse)
- `0x004C3110 DropListClass::Draw_Me` (raw bytes)
- `0x004C3190 DropListClass::Action`
- `0x004C3570 DropListClass::Set_Focus`
- `0x004C30B0 EditClass::~EditClass`
- `0x004882D0 CheckListClass::~CheckListClass`
- `0x004884A0 CheckListClass::Action` (toggle-on-LEFTPRESS)
- `0x00488310 CheckListClass::Add` (item allocation)
- `0x004887F0 ColorListClass::~ColorListClass`
- `0x006B1D00 SliderClass::~SliderClass`
- `0x0069DCF0 ShapeButtonClass::Constructor`
- `0x0069DEB0 ShapeButtonClass::Draw_Me` (5-frame state selection)
- `0x0069DE00 ShapeButtonClass::Set_Shape`
- `0x00723E60 ToggleClass::Constructor`
- `0x00723EC0 ToggleClass::Action` (raw bytes — press/release state machine)
- `0x0048E550 Route_Event` (sticky-capture re-dispatch; shared with ControlClass ctor neighborhood)
- `0x0072A440 TextLabelClass::Constructor`
- `0x00558740 / 0x0055A0D0 LoadOptionsClass::ctor/dtor`
- `0x007AF500 FactionSelectDialogControl::WorldDominationTour::dtor`
- `0x00624130 DialogControl::OwnerDraw::ctor`
- `0x00624110 SimpleDialogControl::OwnerDraw::ctor`
- `0x004E1D00 OptionsClass::ShowInGameDialog`
- `0x00671EA0 RulesClass::ReadMultiplayerDialogSettings`
- `0x006A7780 SidebarClass::Action` (in-game action dispatch, ID switch)
- `0x006D0680 CommandBar_Dispatch` (in-game command-bar action router)
- `0x006922E0 DisplayClass::Dispatch`
- `0x005BDDC0 (radar tick wrapper calling DisplayClass::Dispatch)`

**vtable memory reads:**
- `vtable__GadgetClass @ 0x007E92BC` (140 bytes)
- `vtable__ControlClass @ 0x007E528C` (160 bytes)
- `vtable__ListClass @ 0x007ED10C` (200 bytes)
- `vtable__ToggleClass @ 0x007E8118` (160 bytes)
- `vtable__ShapeButtonClass @ 0x007E8088` (160 bytes)
- `vtable__DropListClass @ 0x007E7FCC` (256 bytes)
- `vtable__CheckListClass @ 0x007E4F84` (200 bytes)
- `vtable__ColorListClass @ 0x007E5054` (200 bytes)
- `vtable__EditClass @ 0x007E81A4`
- `PTR_FUN_007E4DB8` (ColorListClass row painter table, 64 bytes)

**INI keys checked:**
- `[MultiplayerDialogSettings]`, `[Countries]`, `[Sides]`, `[Colors]`,
  `[Easy]`, `[Normal]`, `[Difficult]`, `[mpmodesmd.ini]` mode sections.

**String-table keys found:** `GUI:SkirmishScore`, `STT:SkirmishButtonStartGame`,
`STT:SkirmishComboCountry`, `STT:SkirmishComboColor`, `STT:SkirmishComboAIPlayer`,
`STT:SkirmishSliderUnit`, `STT:SkirmishSliderCredits`, `STT:SkirmishSliderSpeed`,
`STT:SkirmishCBoxCrates/BuildOffAlly/SWAllowed/ShortGame/Redeploys`,
`STT:SkirmishEditPlayer`, `STT:SkirmishLabelScenario/GameType`,
`STT:SkirmishMapThumbnail`, `STT:SkirmishPictureFlag`,
`STT:SkirmishButtonBack/ChooseMap`.

**Prior doc reports referenced:** none (no existing report on gadget framework
in `docs/` or `C:\Users\enok\Documents\ra2-rust-game-docs\`). Adjacent reports:
`SIDEBAR_SYSTEM_GHIDRA_REPORT.md`, `SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md`,
`MouseClass_research.md`.
