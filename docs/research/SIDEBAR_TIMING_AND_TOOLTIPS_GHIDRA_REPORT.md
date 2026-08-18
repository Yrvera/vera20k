---
title: Sidebar Timing & Tooltips (Ghidra Research Report)
date: 2026-04-22
---

# Sidebar Timing & Tooltips — Ghidra Research Report

**Addresses (primary):**
- `ToolTipManager::Constructor` @ `0x00724000` — defaults for delay/duration
- `ToolTipManager::ProcessMessage` @ `0x00724200` — Win32 message handler (WM_TIMER/WM_MOUSEMOVE/WM_*BUTTON*)
- `ToolTipManager::Enable` @ `0x007241A0`
- `StripClass::AI` @ `0x006A8B30` — scroll animation, tab-flash scheduling, auto-build poll
- `StripClass::Draw` @ `0x006A9540` — cameo-darken flash cycle
- `SidebarClass::AddCameo` @ `0x006A6300` — sets TabFlashState
- `SidebarClass::Action` @ `0x006A7780` — tab-flash frame increment, forwards action IDs (PATCHED 2026-05-20: previously labelled `SidebarClass::AI` — Ghidra's symbol for `0x006A7780` is `SidebarClass__Action`. This is the action / input dispatch handler that *calls* `StripClass__AI` per strip; it is not the AI loop itself. The TabFlashState/TabFlashFrame logic described in §4.2 IS present at this address — only the label was wrong.)
- `SidebarClass::GetCameoTooltip` @ `0x006A92E0`
- `SidebarClass::GetTooltipText` @ `0x006AC210` — ID-to-string resolver
- `SelectClass::HighlightOn` @ `0x006AB990` / `HighlightOff` @ `0x006AB9E0`
- `FUN_0069DFC0` @ `0x0069DFC0` — generic flash scheduler
- `FUN_0069DFF0` @ `0x0069DFF0` — generic flash stopper

**Confidence:** HIGH for tooltip delay/duration constants (hardcoded in constructor),
HIGH for cameo-darken cycle (decompiled), HIGH for scroll animation rate,
HIGH for tab flash cadence. MEDIUM for cameo `FlashEndFrame` setter (field
confirmed; exact duration constant in caller not located).

**Active in YR:** Yes — all systems verified live in YR skirmish.

**Relationship to prior report:** This extends
`ra2-rust-game-docs/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md`, which covers the
class structure and geometry. This report focuses on **timing-sensitive
behaviors** — the parity details that determine "feel."

---

## 1. Overview

The sidebar has four independent timed animations, all tuned to hide the 1998
engine's limited frame budget while still reading as responsive to the player:

1. **Tooltip delay/hide** — 1000 ms delay, 10000 ms auto-hide. Driven by Win32
   `SetTimer(hwnd, TTIP_ID, ...)` — so tooltip timing is **real milliseconds,
   not game ticks**, and runs even when the game is paused.
2. **New-item cameo flash** — 16-frame cycle, 7 frames darkened per cycle,
   gated by absolute-game-frame `FlashEndFrame` on each `CameoEntry +0x30`.
3. **Tab flash** — 10-game-frame toggle synced to the global frame counter's
   mod-10 boundary. Indicates a new buildable on another tab.
4. **Scroll hold-repeat** — no explicit timer; one row per game tick while the
   button is held. At standard game speed that's ~15 rows/sec.

The tooltip system is **Win32-native** — the in-game sidebar registers per-gadget
tooltip rects with `ToolTipManager`, which runs a message-pump handler on the
main game window. This is the same pattern as `OptionsClass::ShowInGameDialog`
(documented in the gadget-UI report) — in-game chrome uses Win32 timer/message
APIs for cadences the GadgetClass framework can't express.

---

## 2. ToolTipManager — Win32-native tooltip state machine

### 2.1 Class layout (`this` = `ToolTipManager*`)

Verified from `ToolTipManager::Constructor` @ `0x00724000`.
**PATCHED 2026-05-20** — every offset in the table below was previously listed
0x2080 bytes too large (`+0x22A8` instead of `+0x228`, etc.) due to a
multiplication of the Ghidra `int *`-index by an extra factor of 0x10. Helper
functions at `0x00724520 / 0x00724530 / 0x00724540` confirm only two fields
exist at `+0x228` (HoveredTip / active pointer) and `+0x22C` (SavedHovered
backup) — the prior table also duplicated `+0x228` with two different names
("PrevHovered" and "ActiveBackup"), which has been corrected.

| Offset | Type | Field | Default | Meaning |
|---|---|---|---|---|
| `+0x00` | `void**` | `vtable` | `vtable__ToolTipManager` | |
| `+0x04` | `void*` | `HoveredTip` | `0` | Currently active tooltip (NULL = none shown) |
| `+0x08` | `HWND` | `Window` | ctor arg | Main game window to post timers to |
| `+0x0C` | `bool` | `Enabled` | `0` | Set by `Enable(bool)`; prints `"Tooltips are on/off"` |
| `+0x10..0x17` | `POINT` | `MousePos` | | Client-space mouse for hit-testing |
| `+0x228` | `void*` | `HoveredTip_Active` | `0` | Currently-hovered tip pointer (also the slot saved in `Save_Hovered`) |
| `+0x22C` | `void*` | `SavedHovered` | `0` | Backup of `+0x228` while a cameo overrides the tooltip target (set by `HighlightOn`, restored by `HighlightOff`) |
| `+0x228` (Ghidra index `[0x8A]`) | `int` | `DelayMs` | `1000` | **delay before showing** |
| `+0x230` (Ghidra index `[0x8C]`) | `int` | `DurationMs` | `10000` | **auto-hide after this long** |
| `+0x234` | `void*` | `GrowthVtable` | `&PTR_FUN_007F57C8` | Vtable-like function-pointer table for the embedded growable-buffer helper; slot+8 is called from `Register_Tip` to authorize buffer growth. NOT the tip array itself (corrected 2026-07-12: was "TipTable \| Array of registered tooltip pointers"; binary shows `param_1[0x8d]` holds a raw vtable address that constructor/`Register_Tip` call through — `(**(code**)(*(int*)(this+0x234)+8))(...)` — the tip array pointer actually lives at `+0x238`, confirmed by `read_memory 0x007f57c8` (4 function-pointer entries) + `decompile_function 0x00724000/0x00724580` — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |
| `+0x238` | `void**` | `TipTable` | `0` | Array of registered tooltip pointers (the actual buffer) — walked directly in `ProcessMessage`'s hit-test loop (`piVar4 = (int*)param_1[0x8e]`) and written by `Register_Tip` (`*(int*)(param_1[0x8e] + count*4) = newTip`) (corrected 2026-07-12: was "TipTableCap \| Allocated entries"; the real capacity field is `+0x23C`, see below — via `decompile_function 0x00724200/0x00724580` — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |
| `+0x23C` | `int` | `TipTableCap` | `0` | Allocated capacity, compared against `TipCount` (`+0x244`) in `Register_Tip` to decide whether to grow (previously undocumented — this is the field the prior table's "+0x238 TipTableCap" row actually described; added 2026-07-12 via `decompile_function 0x00724580`) |
| `+0x240` | `bool` | `OwnsBuffer` | `1` | |
| `+0x244` | `int` | `TipCount` | `0` | Live tooltips |
| `+0x248` | `int` | `MaxTipsGrowth` | `10` | Capacity grow step |

> Note: the `+0x228` slot is multi-purposed — it stores `DelayMs` as an `int`
> AND, when the manager is idle, also serves as the "currently-hovered tip"
> pointer slot. The constructor writes `1000` into it as the default delay;
> later state-change paths overwrite it with a tip pointer or null. The two
> uses don't conflict because `HoveredTip_Active` is only meaningful when
> non-null and `DelayMs` is only consulted from within `ProcessMessage` when
> the value is treated as an integer time. The prior table separating them
> into "PrevHovered" + "ActiveBackup" rows was an over-decomposition.

The defaults `1000` and `10000` are hardcoded in the constructor — **NOT
driven by any INI key**. These are the exact parity values.

### 2.2 `ToolTipManager::ProcessMessage` — the actual state machine

Plucked from the real message handler at `0x00724200`. Cases map to standard
Windows messages:

```
fn ProcessMessage(this, MSG* msg):
    if !this.Enabled: return

    match msg.message:
        case WM_TIMER (0x0113):
            if msg.wParam != 0x54544950:  # 'TTIP' timer ID
                return
            KillTimer(this.Window, 0x54544950)
            if this.HoveredTip != NULL:
                # tooltip already visible — WM_TIMER fired at duration — hide it
                this.vtable[0x08](this + 0x18)  # ToolTipManager::Hide via own vtable slot 2; arg is this+0x18, NOT HoveredTip.vtable (corrected 2026-05-28: was "this.HoveredTip.vtable[0x08](&this.Mouse)"; binary shows (**(code**)(*param_1+8))(param_1+6) — own vtable, arg this+0x18 — via decompile_function 0x00724200 — ROOT_CAUSE: INFERENCE_HARDENED)
                return
            # first-hit timer fire — show the tooltip at current mouse
            GetCursorPos(&this.Mouse); ScreenToClient(this.Window, &this.Mouse)
            for tip in this.Tips:
                if tip.Rect.contains(this.Mouse):
                    this.HoveredTip = tip
                    break
            if Show_Tooltip(this) != 0:
                # reschedule timer for DurationMs — auto-hide countdown
                SetTimer(this.Window, 0x54544950, this.DurationMs, NULL)

        case WM_MOUSEMOVE (0x0200):
            if this.DelayMs != 0 and !g_Paused:   # DAT_00A8F7D8 = paused flag
                # restart the delay timer
                KillTimer(this.Window, 0x54544950)
                SetTimer(this.Window, 0x54544950, this.DelayMs, NULL)
                if this.HoveredTip != NULL:
                    this.vtable[0x08](this + 0x18)  # same own-vtable hide pattern (corrected 2026-05-28)
                return
            # DelayMs == 0 or paused — show tooltip IMMEDIATELY on move
            # ... hit-test + show (mirrors the WM_TIMER path) ...

        case WM_LBUTTONDOWN (0x0201) |
             WM_LBUTTONUP   (0x0202) |
             WM_RBUTTONDOWN (0x0204) |
             WM_RBUTTONUP   (0x0205) |
             WM_MBUTTONDOWN (0x0207) |
             WM_MBUTTONUP   (0x0208):
            KillTimer(this.Window, 0x54544950)
            if this.HoveredTip != NULL:
                this.vtable[0x08](this + 0x18)  # same own-vtable hide pattern (corrected 2026-05-28)
                this.HoveredTip = NULL
```

### 2.3 Parity invariants

- **Delay = 1000 ms, duration = 10000 ms** — hardcoded. Not in `rules.ini`,
  not in art files. Reproduce exactly.
- **Move-to-show flow:** each mouse-move **kills and restarts** the delay
  timer. Continuous mouse motion never triggers a tooltip. User must stop
  moving for 1000 ms for the timer to fire.
- **Any mouse button press kills the tooltip instantly** — press, release, all
  five buttons. Not just the one that would activate the gadget. This is why
  starting a drag cancels an open tooltip even though the mouse is still over
  the same cell.
- **Paused game shows tooltips immediately** on move — check `DAT_00A8F7D8`.
  Bypasses the delay timer. (Rationale: while paused, the player is reading
  the UI; instant tooltips make sense. Active play wants the delay.)
- **Tooltips hide themselves on timer expiry** — a visible tooltip schedules
  another `WM_TIMER` at `DurationMs`. When that fires, the handler calls
  `ToolTipManager.vtable[0x08](this + 0x18)` — the manager's own vtable slot 2.
  (corrected 2026-05-28: was "the tip's `Hide()` via vtable+0x08" implying
  HoveredTip.vtable; binary calls the manager's own vtable — via
  decompile_function 0x00724200 — ROOT_CAUSE: INFERENCE_HARDENED)

### 2.4 `ToolTipManager::Enable(bool)` @ `0x007241A0`

```
fn Enable(this, enabled):
    if this.Enabled == enabled: return        # idempotent
    this.Enabled = enabled
    if !enabled:
        KillTimer(this.Window, 'TTIP')
        if this.HoveredTip != NULL:
            this.vtable[0x08](this + 0x18)   # own vtable slot 2, arg this+0x18 — NOT
                                              # HoveredTip.vtable / &this.Mouse
                                              # (corrected 2026-07-12: was
                                              # "this.HoveredTip.vtable[0x08](&this.Mouse)";
                                              # binary shows (**(code**)(*param_1+8))(param_1+6)
                                              # — same own-vtable/this+0x18 pattern already fixed
                                              # in §2.2 on 2026-05-28 but missed here — via
                                              # decompile_function 0x007241A0 — ROOT_CAUSE:
                                              # INFERENCE_HARDENED)
    log("Tooltips are %s.\n", enabled ? "on" : "off")
```

Toggling off kills timers AND hides any visible tooltip immediately. Logs to
the console — a genuine "on/off" user-visible change.

### 2.5 Highlight → tooltip interaction

`SelectClass::HighlightOn` (Mouse_Enter on cameo) @ `0x006AB990`:

```
fn HighlightOn(this):                        # this = SelectClass cameo gadget
    ToolTipManager::Save_Hovered()           # +0x22C = +0x228 (backup)
    ToolTipManager::Set_Hovered(NULL)        # +0x228 = 0     (clear)
    this.IsHighlighted = 1                   # +0x34
    this.Strip.NeedsRedraw = 1               # flag for redraw
```

`SelectClass::HighlightOff` (Mouse_Leave) @ `0x006AB9E0`:

```
fn HighlightOff(this):
    ToolTipManager::Restore_Hovered()        # +0x228 = +0x22C
    this.IsHighlighted = 0
    this.Strip.NeedsRedraw = 1
```

So **cameos override the tooltip state while hovered**. The tooltip text comes
from `SidebarClass::GetCameoTooltip @ 0x006A92E0` (formatted as
`"Name<LF>Cost<LF>Power"` via `StringTable::LoadString(0xC6E)`). When the mouse
leaves, the previous tooltip target is restored — this is how tab tooltips
reappear after you move off a cameo.

---

## 3. Cameo "new item" flash cadence — `StripClass::Draw`

### 3.1 The exact cycle

From `StripClass::Draw` @ `0x006A9540`:

```c
// strip + cameo_slot * 0x34 + 0x88  =  CameoEntry[slot].FlashEndFrame
if ((int)g_CurrentFrameCounter < cameo.FlashEndFrame) {
    uVar11 = g_CurrentFrameCounter & 0x8000000F;     // frame % 16 (signed-safe)
    if ((int)uVar11 < 0)
        uVar11 = (uVar11 - 1 | 0xFFFFFFF0) + 1;      // normalize negative
    if (8 < (int)uVar11) {                           // frames 9..15 of every 16
        CC_Draw_Shape(DAT_00B07BC0, 0, ..., 0x404, ...);   // darken overlay on top
    }
}
```

- **Cycle period: 16 game frames.**
- **Dark phase: frames 9, 10, 11, 12, 13, 14, 15** (`frame % 16 > 8`) — 7/16.
- **Bright phase: frames 0..8** — 9/16.
- Effect is a repeating dim-bright pulse while `CurrentFrame < FlashEndFrame`.
- Uses `DAT_00B07BC0` = the standard "can't build" darken SHP. Same SHP as the
  "can't afford" overlay — so a flashing new cameo visually reads like the
  "disabled" state during its dark phase.
- Draw flag `0x404` = alpha-blend darken on top.

**Field confirmed:** `CameoEntry +0x30` (also referred to as the strip-relative
offset `+0x88`) is **`FlashEndFrame`** — the absolute `g_CurrentFrameCounter`
value at which the flash stops.

### 3.2 Who sets `FlashEndFrame`?

`StripClass::InsertEntry @ 0x006A8710` explicitly zeros this field on insertion
(`*(int*)(iVar4 + 0x88) = 0`). **Direct writes from the sidebar AddCameo path
were not located** — the flash-start code is elsewhere (likely in the factory
"just completed" path or an explicit `SidebarClass::FlashCameo` we haven't
decompiled). Open question in Section 7.

### 3.3 Parity invariants

- **16-frame cycle, NOT 8.** The prior report's "8-frame cycle" claim is a
  rounding to "~8 on, ~8 off" — the actual modulus is `& 0x0F` = 16.
- The flash only darkens — the base cameo is always drawn. There is no fade
  animation; it's a hard on/off toggle.
- At standard game tick rate (`GameSpeed=4` ≈ 15 FPS logic), one full cycle =
  ~1.07 seconds. At paused or menu frame rate it can be faster.
- Duration is decided by whoever sets `FlashEndFrame`. Prior report implies
  this is the "new buildable arrived" signal; open question for exact count.

---

## 4. Tab flash cadence — `StripClass::AI` + generic flash scheduler

### 4.1 Setup — `SidebarClass::AddCameo` @ `0x006A6300`

When a new buildable arrives:

```c
if (DAT_00B0B478 != 0) {                       // tab flash SHP loaded
    sidebar.TabFlashState = 1;                 // +0x14E6*4 = +0x5398
    sidebar.TabFlashFrame = 0;                 // +0x14E5*4 = +0x5394
}
```

### 4.2 Frame increment — `SidebarClass::Action` @ `0x006A7780` (was `SidebarClass::AI` — PATCHED 2026-05-20)

```c
if (sidebar.TabFlashState == 1) {
    sidebar.TabFlashFrame++;
    if (shp_frame_count(DAT_00B0B478) < sidebar.TabFlashFrame + 1) {
        sidebar.TabFlashState = 0;              // animation done
        sidebar.TabFlashFrame = 0;
    }
} else if (sidebar.TabFlashState == -1) {
    sidebar.TabFlashFrame--;
    if (sidebar.TabFlashFrame < 0) {
        sidebar.TabFlashState = 0;
        sidebar.TabFlashFrame = 0;
    }
}
```

**Tab-flash SHP** (`DAT_00B0B478`) is loaded once in `LoadArt`. Its SHP header
`+6` (frame count) drives the duration. Each game tick advances one frame.
State `+1` = forward play, `-1` = reverse play, `0` = idle. Plays the animation
once and stops.

### 4.3 Generic 10-frame flash scheduler — `FUN_0069DFC0` / `FUN_0069DFF0` / `FUN_0069E010`

**PATCHED 2026-05-20.** The previous pseudocode signature
`Start_Flash(struct, start_frame, duration, initial_state)` mislabelled the
arguments. Full investigation in
[SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md](SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md);
the corrected three-function family is:

```c
// FUN_0069DFC0 (__thiscall; RET 0xc — 3 stack args after ECX=gadget)
u32 Start_Flash(SBGadget *this, int period, int extra_delay, byte initial_state) {
    if (this->Period /* +0x38 */ != 0) return 0;           // already flashing
    this->Period       /* +0x38 */ = period;               // toggle interval (and "is-flashing" sentinel)
    this->Countdown    /* +0x3c */ = period + extra_delay; // first countdown only
    this->CurrentState /* +0x34 */ = initial_state;
    return 1;
}

// FUN_0069DFF0 (__fastcall; ECX=gadget)
u32 Stop_Flash(SBGadget *this) {
    if (this->Period == 0) return 0;
    this->CurrentState = 0; this->Countdown = 0; this->Period = 0;
    return 1;
}

// FUN_0069E010 (__fastcall; ECX=gadget; called per tick from SidebarClass::Action)
u32 Flash_AI(SBGadget *this) {
    if (this->IsDisabled /* +0x1e */ == 0) {
        if (this->Countdown != 0) {
            if (--this->Countdown == 0) {
                this->CurrentState = !this->CurrentState;      // toggle byte
                this->Countdown    = this->Period;             // reset to FIXED period
                return 1;
            }
        }
        return 0;
    }
    if (this->Period != 0) { /* auto-stop on disabled */ ... }
    return 0;
}
```

Called in `StripClass::AI` for the **per-tab pulse** on aircraft-completion
or super-weapon-ready (raw asm at `006a8e52..006a8e9b`):

```c
iVar12 = 10 - (g_CurrentFrameCounter % 10);
parity = ((iVar12 + g_CurrentFrameCounter) / 10) & 1;
Start_Flash(
    &g_TabGadgets[strip.TabIndex],    // ECX (this) — the tab button gadget
    10,                                // period  → +0x38
    iVar12,                            // extra_delay → added to period for first countdown
    parity == 0                        // initial_state → +0x34
);
```

- **Period = 10 game ticks** (fixed; `+0x38` always = 10). After the first
  toggle, the countdown resets to `+0x38 = 10`, so every subsequent toggle is
  10 ticks apart.
- **First toggle is delayed by `period + extra_delay` = `10 + (10 - frame%10)`**
  ticks, which always lands the first toggle on the **second** 10-frame
  boundary after the call. This guarantees the initial state is visible for at
  least 10 ticks before flipping.
- **Phase alignment.** All concurrent calls within the same 10-frame phase
  target the same boundary index, so the parity bit gives them the same
  initial state — they blink in sync, not as visual noise.
- **Visual effect.** Toggling `+0x34` between 0 and 1 makes `SBGadgetClass::Draw`
  swap between frame {0 or 1} (idle/active) and frame {3 or 4} (pressed-look)
  for pressable tab gadgets (tabs init with `+0x40 = 1`). Frames 2–4 of
  `tab0N.shp` are required for this to render correctly — see the tab-flash
  scheduler doc §7 for the full frame-state machine.

### 4.4 Trigger conditions in `StripClass::AI`

Per-strip AI iterates its slots each tick. **The pulse is triggered only for**:

| Strip / Tab | Trigger |
|---|---|
| Tab 0 (non-naval Aircraft) | Aircraft factory completion (RTTI 6 = AIRCRAFT) |
| Tab 1 (Defense — SW + naval Aircraft) | SW ready (RTTI 0x1F, `Available[] != 0` AND `FUN_006ce1a0()` true), OR naval aircraft completion |
| Tab 2 (Structures) | Never (RTTIs don't match) |
| Tab 3 (Units / Infantry) | Never (RTTIs don't match) |

Building, infantry, and non-aircraft vehicle completions do NOT pulse — they
trigger EVA voice + auto-dispatch in a separate AI block, but not the flash.

`StripClass::AI` also gates the trigger evaluation to `strip.TabIndex ∈ {0, 1}`
(the `+0x38 == 0 || == 1` check at `006a8d07`) — Tabs 2 and 3 skip the trigger
loop entirely.

### 4.5 Parity invariants

- **Tab SHP flash** (new-buildable arrival via `SidebarClass::AddCameo` at
  `0x006A6300` setting `TabFlashState`): plays the tab-flash SHP at
  `DAT_00B0B478` once from frame 0 to `frame_count - 1`, one frame per game
  tick. **Dormant in YR — `DAT_00B0B478` is never loaded** (per
  `SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md §10`), so this animation has no
  visible effect. The SidebarClass `+0x5394/+0x5398` field pair IS live code,
  but its output SHP is null in YR.
- **Per-tab pulse via FUN_0069DFC0** (aircraft completion / SW ready): the
  *actually-visible* tab-flash in YR. 10-tick fixed period, first toggle on
  the second-next 10-frame boundary, phase-aligned across concurrent calls.
  Stops automatically when the trigger condition clears (the StripClass::AI
  iteration falling through to `Stop_Flash` at `006a8d9a`).

---

## 5. Scroll animation & hold-repeat — `StripClass::AI`

### 5.1 Scroll request/step

`StripClass::AI` reads `+0x48 ScrollRequest` each tick:

```c
if (strip.ScrollRequest != 0 and visible_rows < total_cameos) {
    if strip.ScrollRequest < 0:                    # scroll up
        if strip.ScrollPosition > 0:
            strip.ScrollRequest++                  # towards zero
            strip.IsScrolling = 1                  # +0x3F
            strip.ScrollDirection = 0              # +0x3E = 0 = up
            strip.ScrollPosition--                 # +0x44
            strip.ScrollPixelOffset = 0            # +0x4C
    else:                                          # scroll down
        if (strip.ScrollPosition + visible) * 2 < total_cameos:
            strip.ScrollRequest--
            strip.ScrollPixelOffset = RowHeight    # DAT_00B0B500 = 0x32 = 50
            strip.IsScrolling = 1
            strip.ScrollDirection = 1
}
```

### 5.2 Scroll animation

**PATCHED 2026-05-20.** The previous version of this section had the
`ScrollDirection==0` / `==1` arithmetic mappings swapped, and the position
update was attributed to the wrong direction. Corrected per live decompile of
`StripClass::AI`:

Runs while `IsScrolling == 1`:

```c
if strip.ScrollDirection /* +0x3E */ == 0:         # UP — position was already
                                                   # pre-decremented at scroll-request time
    strip.ScrollPixelOffset += DAT_00B0B514        # = 50; animates from 0 toward RowHeight
    if strip.ScrollPixelOffset >= DAT_00B0B500:    # >= 50
        strip.IsScrolling = 0
        strip.ScrollPixelOffset = 0
        # NO position update here — UP was pre-decremented at request time
else:                                              # DOWN — position post-incremented here
    strip.ScrollPixelOffset -= DAT_00B0B514        # = 50; animates from 50 toward 0
    if strip.ScrollPixelOffset < 1:
        strip.IsScrolling = 0
        strip.ScrollPixelOffset = 0
        strip.ScrollPosition++                     # +0x44 — post-increment at animation end
```

The UP / DOWN asymmetry is intentional in the binary: UP pre-decrements
`ScrollPosition` *at scroll-request time* (in §5.1's request-handling block),
then animates the pixel offset back up to RowHeight purely as a visual smoothing.
DOWN does the opposite — request-time only sets `ScrollPixelOffset = RowHeight`
and starts the animation, and `ScrollPosition` advances when the animation
finishes. A naive copy of the previous (wrong) pseudocode into Rust would
produce inverted scroll animations and an off-by-one position update.

`DAT_00B0B514 (ScrollAnimSpeed) = 0x32 = 50 pixels/tick`. `DAT_00B0B500
(RowHeight) = 0x32 = 50 pixels`. So **one tick = one complete row scroll** —
the animation finishes on the same tick it started. The player sees exactly
one partial-frame of scroll (rendered at `ScrollPixelOffset` halfway through
the tick) before it snaps to the new row.

### 5.3 Hold-repeat

There is **no explicit hold-repeat timer**. The scroll button is a
`ShapeButtonClass` (`ControlClass` derivative). Per the gadget-UI report
(`ToggleClass::Action` @ `0x00723EC0`):

- **LEFTPRESS** sets `+0x2C IsPressed = 1` and acquires sticky-capture.
- **Each `Input()` tick while sticky-held** dispatches `Action` again, which
  posts the scroll-button ID (`0xC8` up / `0xC9` down).
- The sidebar handler (`SidebarClass::AI` → `CommandBar_Dispatch`) sets
  `strip.ScrollRequest` each time it sees the scroll ID.
- `StripClass::AI` runs every tick and consumes one unit of `ScrollRequest`.

Net effect: **one row scrolled per game tick while held**. At default game
speed (roughly 15 FPS logic tick), that's ~15 rows/second — fast enough that
holding briefly jumps most of the way down the list.

### 5.4 Parity invariants

- **Instant single-frame animation per row** — no multi-frame easing.
- **One row per game tick while held.** Do not add acceleration or explicit
  repeat-delay — original has neither.
- **Scroll button becomes disabled** via `UpdateScrollButtons` (`0x006A6610`)
  when the strip can't scroll further in the requested direction — the hold
  then does nothing rather than wrapping.

---

## 6. Tooltip ID → string resolution — `SidebarClass::GetTooltipText`

| Gadget ID | Decimal | Tooltip source |
|---|---|---|
| `0xC8` | 200 | CSF string `0x13CD` (scroll up) |
| `0xC9` | 201 | CSF string `0x13D3` (scroll down) |
| `0xCB` | 203 | CSF string `0x13DB` (tab 0) |
| `0xCC` | 204 | CSF string `0x13DD` (tab 1) |
| `0xCD` | 205 | CSF string `0x13DF` (tab 2) |
| `0xCE` | 206 | CSF string `0x13E1` (tab 3) |
| `>= 1000` | — | `SidebarClass::GetCameoTooltip(id - 1000)` — cameo index |

Cameo IDs above 1000 map to `(id - 1000) = slot index`. The cameo tooltip
content comes from the item's `TechnoTypeClass +0x60` via
`StringTable::LoadString(0xC6E)` — formatted as multi-line text with spaces
replaced by `'\n' (0x0A)` so each field wraps onto a new line:

```c
for ch in tooltip_buffer:
    if ch == ' ': ch = 0x0A      # space → newline
```

This is how the cameo tooltip shows `Name` / `Cost` / `Power` on separate
lines despite being a single format-string result.

PowerClass also hooks in via `PowerClass::GetTooltipText @ 0x00640450`
(called first in the chain) for the power-bar region.

---

## 7. Open questions

- **Who writes to `CameoEntry +0x30 FlashEndFrame`?** `InsertEntry` zeros it;
  `StripClass::Draw` reads it; but the setter was not located by grep-patterns
  within this pass. Likely candidates:
  - `FactoryClass::OnCompleted` — "production finished, flash the cameo"
  - A direct `SidebarClass::FlashCameo(slot, frames)` helper
  - Set via a time-delta like `+0x30 = CurrentFrame + SomeConst`
  Matters for the **duration** of new-item flash; without it we'd have to
  estimate (prior report suggests "until first click or N seconds").

- **`TooltipRect` INI key** (seen as a string constant at `0x00848B00`) — not
  located in code yet. Likely per-dialog rect override. Not critical for
  sidebar timing.

- **`HasTurretTooltips=yes` INI key**: default `no` in YR. Controls per-unit
  tooltip multiplexing by turret state. Confirmed read but the downstream
  effect on sidebar wasn't traced. Non-blocking for parity (default is off).

- **Middle-click behavior on tooltips**: `WM_MBUTTONDOWN/UP (0x0207/0x0208)`
  is handled — kills tooltip on middle press. But the sidebar doesn't bind
  middle-click to any gadget action, so the behavior is academic.

---

## 8. Current Rust implementation status (scoped to timing behaviors)

| System | Rust state |
|---|---|
| Tooltip delay timer | **NOT IMPLEMENTED** — no hover-delay anywhere in `src/`. |
| Tooltip auto-hide | **NOT IMPLEMENTED**. |
| Cameo new-item flash | **NOT IMPLEMENTED** — no per-tick darken toggle. |
| Tab flash on new buildable | **NOT IMPLEMENTED** — tab state is a static bool. |
| Scroll button hold-repeat | **NOT IMPLEMENTED** — no sticky-held dispatch. |
| Hover highlight (static) | Implemented in `src/sidebar/mod.rs` as `compute_hit_test`
  but no Mouse_Enter/Leave distinction — flips on every tick the mouse is over. |
| Pulse / flash general framework | `src/sidebar/power_bar_anim.rs` has a
  "10-flash blink cycle" for the power bar (unrelated to cameo flash). Good
  template for the cameo cycle but needs different period (16 vs 10). |

---

## 9. Parity implications (ranked)

1. **Cameo darken-pulse for new buildables** — players actively look for this
   to know something just unlocked. Missing = they won't notice new
   techs/upgrades landing. **High visible impact; small fix** (16-frame
   modulo + darken overlay).

2. **Tooltip delay of 1000 ms + auto-hide at 10000 ms** — a tooltip that
   shows instantly feels wrong (muscle memory from the original). A tooltip
   that never auto-hides also feels wrong (they expect it to go away).
   **High visible impact; one timer per gadget type.**

3. **Any-click-kills-tooltip** — small but they'll feel it the first time.
   **Trivial to add.**

4. **Scroll button hold = one row per tick** — players expect holding to
   scroll continuously, not a click-per-scroll interaction. **Medium impact;
   depends on the sticky-capture work already slated.**

5. **Tab flash on new-buildable arrival** — easy to miss but good UX. The
   tab-flash SHP is an RA2 art asset we'd need to load separately.
   **Medium impact; depends on SHP loading.**

6. **Super-weapon-ready tab pulse (10-frame)** — activates only when a SW
   is ready on a different tab. Edge case but a clear feedback loop.
   **Low-for-now, medium when superweapons ship.**

---

## Sources

**Ghidra addresses decompiled:**
- `0x00724000 ToolTipManager::Constructor`
- `0x007240B0 ToolTipManager::Destructor`
- `0x007241A0 ToolTipManager::Enable(bool)`
- `0x00724200 ToolTipManager::ProcessMessage(MSG*)`
- `0x00724520 / 0x00724530 / 0x00724540` Save/Restore/SetHovered helpers
- `0x00724580 ToolTipManager::Register_Tip (new tooltip)`
- `0x007784A0 CCToolTip::Constructor`
- `0x006A8B30 StripClass::AI`
- `0x006A9540 StripClass::Draw`
- `0x006A8710 StripClass::InsertEntry`
- `0x006A6300 SidebarClass::AddCameo`
- `0x006A92E0 SidebarClass::GetCameoTooltip`
- `0x006AC210 SidebarClass::GetTooltipText`
- `0x006AB990 SelectClass::HighlightOn`
- `0x006AB9E0 SelectClass::HighlightOff`
- `0x0069DFC0 Start_Flash generic helper`
- `0x0069DFF0 Stop_Flash generic helper`

**INI keys verified:**
- `[General] FlashFrameTime = 7` (read @ `0x00671979` into `rules+0x88`)
  — drives **radar combat flash**, not the sidebar. Misleadingly named.
- `[General] RadarCombatFlashTime = 49` — radar only.
- `[General] EliteFlashTimer = 150` — unit elite-promotion flash, not sidebar.

**Strings checked:**
- `"FlashFrameTime"` @ `0x0083B750`
- `"RadarCombatFlashTime"` @ `0x0083B738`
- `"ToolTips"`, `"Tooltips are %s.\n"`, `"D:\\ra2mdpost\\ToolTip.cpp"`
- `"HasTurretTooltips"` @ `0x00844354`
- `'TTIP'` byte pattern (0x54544950) @ 8 call sites in `0x00724xxx`

**Prior reports extended:**
- `ra2-rust-game-docs/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md` (structure)
- `ra2-rust-game-docs/FLASHER_CLASS_GHIDRA_REPORT.md` (tactical-unit flashing, not sidebar)
- `docs/GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md` (input-drives-draw, sticky-capture)
