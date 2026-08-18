# Plan-grounding lane — BEHAVIOR-CONTRACT DETAILS (A0/A1/A4/A5 + D-B3 + R1)

Date: 2026-06-10. Lane: plan-grounding-contract. Ghidra MCP read-only; worktree anchors verified against
`C:/Users/enok/Documents/ra2-uigadget-worktree` @ commit 7b79a186 (branch ui-gadget-substrate).

Provenance tags per claim:
- **[V]** VERIFIED-LIVE this lane (MCP call cited inline)
- **[LANE]** verified-live in a sibling lane worknote this study (file§ cited) — lane-grade evidence
- **[DOC]** DOC-INHERITED from a named prior Ghidra report (not re-read from binary this lane unless stated)
- **[WT]** verified in the worktree checkout @ 7b79a186 (file:line cited)
- **[UNK]** unknown — listed in §7

---

## 1. A0 — G-clause exact algorithms (every constant named)

Source of truth: gadget-core.md §§2–10 (asm-verified) + study §5.1. Re-stated here at implementer
pseudocode level so no re-research is needed. "Tick" = one Input call on a list head.

### 1.1 Named constants

| Constant | Value | Meaning |
|---|---|---|
| FLAG_LEFTPRESS | 0x1 | queued LMB-down event |
| FLAG_LEFTHELD | 0x2 | polled LMB held (idle ticks only) |
| FLAG_LEFTRELEASE | 0x4 | queued LMB-up event |
| FLAG_LEFTUP | 0x8 | polled LMB not-held (idle ticks only) |
| FLAG_RIGHTPRESS | 0x10 | queued RMB-down |
| FLAG_RIGHTHELD | 0x20 | polled RMB held (idle only) |
| FLAG_RIGHTRELEASE | 0x40 | queued RMB-up |
| FLAG_RIGHTUP | 0x80 | polled RMB not-held (idle only) |
| FLAG_KEYBOARD | 0x100 | queued non-mouse event |
| PRESS_BITS | 0x11 | LEFTPRESS\|RIGHTPRESS (Sticky_Process acquire test) |
| RELEASE_BITS | 0x44 | LEFTRELEASE\|RIGHTRELEASE (release test / G22 strip) |
| KEY_LMB_DOWN / KEY_RMB_DOWN | 0x001 / 0x002 | queue key codes |
| KEY_LMB_UP / KEY_RMB_UP | 0x801 / 0x802 | queue key codes (low byte 1/2 ⇒ event-coords source) |
| MOD_SHIFT / MOD_CTRL / MOD_ALT | 0x1 / 0x2 / 0x4 | modifier word bits (G9) |
| Default modifier VKs | 0x10 / 0x11 / 0x12 | SHIFT/CTRL/ALT pairs, OptionsClass::SetDefaults 0x005FA350 |
| RESULT_BUTTON | 0x8000 | ControlClass result `ID\|0x8000` |
| RESULT_RIGHT | 0x4000 | extra OR iff RIGHTRELEASE fired AND gadget mask has 0x10 |
| HIT_SEED_W × HIT_SEED_H | 1024 × 768 = **786,432 px²** | Hit_Test best-area seed; .rdata consts 0x007F5BE8/0x007F5BF4, zero writers — NOT live resolution. A gadget with area > 786,432 can never win. |
| STICKY_FORCED_MASK_BITS | 0x05 | ctor: `sticky ⇒ Flags \|= 5` (G4) |
| ToggleClass ctor defaults | flags=5, sticky=1, Kind=0, IsPressed=0, IsOn=0 | 0x00723E60 [LANE gadget-core §7] |

Gadget fields: +0x0C X, +0x10 Y, +0x14 W, +0x18 H, +0x1C IsToRedraw u8, +0x1D IsSticky u8,
+0x1E IsDisabled u8, +0x20 Flags u32; Control +0x24 ID, +0x28 Peer; Toggle +0x2C IsPressed u8,
+0x2D IsOn u8, +0x30 Kind u32. [LANE gadget-core §3.3]

### 1.2 Tick pipeline (Input 0x004E1640, asm-accurate)

```
tick(head, focus, input) -> u16:
  # G5 fresh-list reset
  list_changed = (focus.current_list != head)
  if list_changed: focus.sticky = None; focus.keyboard = None; focus.current_list = head

  # queue read
  key = if input.queue_has_event() { input.queue_get() & 0xFFFF } else { 0 }

  # G6 coordinate source
  if (key & 0xFF) == 1 or (key & 0xFF) == 2:    # covers 0x001/0x002/0x801/0x802
      (x, y) = queued event coords (WWKeyboard +0/+4 equivalent)
  else:                                          # keyboard event OR idle tick
      (x, y) = live mouse position

  # G7 hover transitions — BEFORE dispatch, every tick (incl. the click tick)
  hit = hit_test(head, x, y)                     # §1.3
  if hit != focus.hovered:
      old.mouse_leave() if old; focus.hovered = hit; new.mouse_enter() if new

  # G8 flag assembly — NEVER both event-bits and held-bits in one tick
  flags = match key { 0 => 0, 0x001 => 0x1, 0x002 => 0x10, 0x801 => 0x4, 0x802 => 0x40, _ => 0 }
  modifier = (SHIFT down?1:0) | (CTRL down?2:0) | (ALT down?4:0)      # G9, polled fresh
  if key == 0:
      flags |= LMB_down ? 0x2 : 0x8
      flags |= RMB_down ? 0x20 : 0x80
  elif flags == 0:
      flags = 0x100                              # queued non-mouse event

  # G10/G11 tier 1 — sticky (exclusive)
  if focus.sticky:
      g = focus.sticky
      g.draw_me(false)                           # pre-draw
      g.clicked_on(&key, flags, x, y, 0)         # modifier hardwired 0
      (focus.sticky or g).draw_me(false)         # post-draw — re-reads global; released holder still post-drawn
      return key

  # tier 2 — keyboard focus (only when flags & 0x100)
  if focus.keyboard and (flags & 0x100):
      same pre-draw / clicked_on(..., 0) / post-draw pattern; return key

  # G12 tier 3 — broadcast walk head→tail
  for g in list:
      g.draw_me(list_changed)                    # forced=1 only on fresh list
      if !g.disabled and g.clicked_on(&key, flags, x, y, modifier) != 0:
          g.draw_me(false)                       # consumer's extra draw
          break                                  # gadgets after consumer get NEITHER call
  return key                                     # possibly rewritten to ID|0x8000[|0x4000] (G13)
```

### 1.3 Hit_Test 0x004E15A0 (G14)

```
hit_test(head, mx, my):
  best = None; best_area = 786_432                # signed i32 math
  for g in head..tail:                            # forward walk
      if g.disabled: continue
      if !(g.X <= mx < g.X+g.W and g.Y <= my < g.Y+g.H): continue   # HALF-OPEN
      if (g.W * g.H) <= best_area:                # signed <=  ⇒ equal area: LATER gadget wins
          best = g; best_area = g.W*g.H
  return best
```

### 1.4 Clicked_On 0x004E13F0 (G15)

```
clicked_on(g, key*, flags, mx, my, modifier):
  flags &= g.Flags                                # mask FIRST
  if g != sticky_holder and (flags & 0x100) == 0
     and (flags == 0 or (u32)(mx - g.X) >= g.W or (u32)(my - g.Y) >= g.H):
      return 0
  # i.e. dispatch iff: sticky holder (even masked-0)  OR  masked 0x100 (keyboard bypasses bounds)
  #                    OR  masked != 0 AND point inside half-open rect (unsigned-compare trick)
  return g.action(flags, key*, modifier)
```

### 1.5 Sticky_Process 0x004E1970 (G17)

```
sticky_process(g, flags):
  if g.IsSticky and (flags & 0x11): sticky = g          # acquire on press
  elif sticky != g: return                              # only holder may release
  if flags & 0x44: sticky = None                        # release on release
  # flags containing both 0x11|0x44: acquires then immediately releases
```

### 1.6 Base/Control Action (G16/G21/G13)

- Base `Action 0x004E1530`: `flags==0 → return 0`; else `IsToRedraw=1; sticky_process(flags); return 1`.
- `ControlClass::Action 0x0048E5A0`:
```
if flags != 0:
    *key = (ID == 0) ? 0 : ID | 0x8000
    if (flags & 0x40) and (g.Flags & 0x10): *key = ID | 0xC000     # G13 right-release marker
if g.Peer: g.Peer.peer_callback(flags, key, g)
return base_action(flags, key, 0)
```
- `ControlClass::Draw_Me 0x0048E620`: draw Peer unforced FIRST, then base dirty-gate (G21).

### 1.7 G22 — Toggle/button machine (ToggleClass::Action 0x00723EC0) as a STATE TABLE

Preliminaries executed on EVERY Action call, in this order:
1. `inside = (u32)(live_mouse_x − X) < W && (u32)(live_mouse_y − Y) < H` — **LIVE mouse**
   (WWMouse), NOT the queued event coords; half-open.
2. If `flags == 0` (reachable only as sticky holder, G15): hover-track —
   `inside && !IsPressed → IsPressed=1,dirty`; `!inside && IsPressed → IsPressed=0,dirty`.
3. `sticky_process(flags)` (capture acquire/release happens HERE, before branches).

| # | State (IsPressed) | Event (masked flags) | Transition / output |
|---|---|---|---|
| 1 | 0 or 1 | contains PRESS (0x11) | IsPressed=1; dirty; capture acquired (step 3); `ControlClass::Action(flags & ~0x11, …)` (press bits stripped → no ID unless other bits remain); **force `*key = 0`; return 1 (consumed)** — "silent press" |
| 2 | 1 (captured) | flags==0 sticky re-dispatch, cursor outside | IsPressed=0; dirty (visual pop-out). Tail `Action(0)` returns 0 |
| 3 | 0 (captured) | flags==0 sticky re-dispatch, cursor back inside | IsPressed=1; dirty (pop back in) |
| 4 | 0 | contains RELEASE (0x44) | **strip release bits** (`flags &= ~0x44`) → tail `Action(flags)`; fires nothing unless other masked bits remain — drag-off cancel outcome |
| 5 | 1 | RELEASE, cursor inside | Kind==1: `IsOn = !IsOn` (flip); Kind==2: `if !IsOn IsOn=1` (latch-ON only, never off); Kind==0: IsOn untouched. IsPressed=0; dirty; release bits KEPT → tail posts `ID\|0x8000` (\|0x4000 per G13) — **fire on release-inside** |
| 6 | 1 | RELEASE, cursor outside | IsPressed=0; dirty; release bits **NOT stripped** → still fires. Boundary case only: reachable when press and release were processed with NO intervening masked-0 tick (which would have popped IsPressed via row 2) |
| 7 | any | held bits only (0x2/0x20, if mask includes them) | falls through both branches → tail `Action(flags)` posts `ID\|0x8000` **every tick** = G23 hold-repeat |

G23: hold-repeat is purely the mask property — repeat rate = Input call rate, **no timer, no
initial delay, no acceleration**. Live consumer: GaugeClass::Action 0x004E2830 gate
`(flags&1) || ((flags&2) && this==sticky)`. Sidebar strip scroll mask = 0x55 (no held bits) ⇒ NO
repeat; one page per click on release. [LANE gadget-core §7.1/§9/§10]

### 1.8 List/lifecycle clauses (G1–G5, G18–G20, G24–G25) — implementer summary

- G1/G2/G3: intrusive doubly-linked list; `add(after)` / `add_tail` / `add_head` each
  **self-remove first** (never in two lists); `remove` repairs neighbors + returns new head;
  GadgetClass::Remove additionally Clear_Focus()es itself first; `zap` = zero own links, no repair.
- G4 ctor defaults: geometry+mask+IsSticky set, all else zero; `sticky → Flags |= 0x05`.
- G18 focus: Set_Focus steals (old holder: Flag_To_Redraw + Clear_Focus + Flags&=~0x100; new:
  Flags|=0x100); Clear_Focus self-conditional; Has_Focus = pointer equality; Enable/Disable/
  Remove/destruction all force Clear_Focus.
- G19: Flag_To_Redraw sets only the local dirty byte; Draw_Me(0) no-ops unless dirty then clears;
  Enable/Disable set dirty unconditionally; Any_Redraw_Pending scans tail-ward only.
- G20: draw order = walk order = registration order (tail renders on top, consistent with G14
  later-wins); plus per-frame dirty sweep via Draw_All(+0x2C) from the render frame (dual pump);
  no full-frame clear.
- G24: destruction clears keyboard (incl. the 0x100 bit), sticky, current_list if self — but in
  gamemd NOT hover. **Rust closure (study §6.1): removal/destruction clears hover too; no
  Mouse_Leave on a dead handle** (A0 acceptance test).
- G25: Clear_Attached_List zeroes only current_list ⇒ next tick takes the G5 reset path — the
  sanctioned page-swap mechanism.

---

## 2. A1 — sidebar buttons onto the substrate (grounding + NEW verified facts)

### 2.1 Button identity table — Kind values pinned [V: decompile_function 0x006A5310 this lane]

SidebarClass::Init_IO 0x006A5310 writes the static ShapeButton records (all built by ctorA
0x0069DCF0 → ToggleClass ctor: flags=5, sticky=1, Kind=0; ctor zeroes ShapeButton fields and
presets +0x50 = DAT_0087F6C4 [V: decompile 0x0069DCF0]):

| Button | Record base | ID | Flags mask | Kind | Notes |
|---|---|---|---|---|---|
| 4 tab buttons | 0x00B07C48, stride 0x60 | 0xCB+i (i=0..3) | 5 (ctor default) | **2 (latch-ON)** | X = tabX + i·tabW; IsOn=0; byte +0x40=1; +0x44=−0x1E0; start **Disabled** (Disable 0x004E1460 called per button); IsOn driven externally via Turn_On/Turn_Off 0x00723EA0/0x00723EB0 from SidebarClass on tab switch |
| repair | 0x00B0B3A0 | 0x65 | 5 | **1 (flip)** | IsOn=0; byte +0x40=1 |
| sell | 0x00B07DF8 | 0x66 | 5 | **1 (flip)** | same shape |
| strip scroll-down (+page) | 0x00B0B328 | 0xC9 | **0x55** | 0 | no held bits ⇒ NO repeat (G23) |
| strip scroll-up (−page) | 0x00B0B408 | 0xC8 | **0x55** | 0 | same |

This resolves study §9.1's YELLOW "sidebar single-button global↔role bindings":
0x00B0B3A0 = repair (0x65), 0x00B07DF8 = sell (0x66), 0x00B0B328 = 0xC9 scroll-down,
0x00B0B408 = 0xC8 scroll-up. (0x8065→FUN_004AC8C0 repair-mode, 0x8066→FUN_004AC660 sell-mode
per SidebarClass::AI [LANE gadget-core §10 / gscreen-chain §4.1].)

Flags mask 5 on tabs/repair/sell ⇒ left-button only (no right-click response, no 0x4000 results).
Mask 0x55 on scroll ⇒ right-release fires `ID|0xC000`; consumer masks `key & ~0x4000` so
right-click scrolls identically [LANE gadget-core §10].

### 2.2 Consumption contract (fire-on-RELEASE authority flip)

- All five+two buttons run the §1.7 machine: press = silent + capture; release-inside = fire
  `ID|0x8000`; drag-off cancels (row 4); per-tick masked-0 re-dispatch gives the pressed-visual
  pop-out/pop-in (rows 2/3).
- SidebarClass::AI 0x006A7780 consumes: `0x80CB..0x80CE` tab select (full cameo Remove/Add swap +
  sound); `0x8065`/`0x8066` repair/sell mode toggles (UI state only); scroll
  `(key & ~0x4000) == 0x80C9` → +1 page (end-guard) / `== 0x80C8` → −1 page (zero-guard), page
  rows = `(strip_px_height …)/0x32` (50-px rows, two columns), and it **clears the scroll
  button's IsPressed directly** (writes 0x00B0B354 / 0x00B0B434 = 0; no other readers of those
  bytes). [LANE gadget-core §10]
- TabClass::AI consumes 0x80F0/0x80F1 (collapse/expand) — out of A1 scope (A6).
- Scroll VISUAL: StripClass::AI animates 50 px/tick with RowHeight 50 ⇒ one-row snap per tick;
  one page per CLICK is the input-side truth (G23); SIDEBAR_TIMING §5.3's "repeat while held"
  mechanism claim is refuted DRIFT. [DOC SIDEBAR_TIMING §5.1–5.2 for animation; LANE for mask]

---

## 3. A4 — shared tooltip service contract (provenance per item)

### 3.1 Manager state (this = ToolTipManager)

[V: decompile_function 0x00724000 this lane — upgrades S1's DOC-INHERITED 1000 ms to VERIFIED-LIVE]

| Field | Offset | Default | Meaning |
|---|---|---|---|
| HoveredTip (active record ptr) | +0x04 | 0 | non-null = tip visible |
| Window | +0x08 | ctor arg | SetTimer target |
| Enabled | +0x0C | 0 | gate; ProcessMessage requires == 1 |
| MousePos POINT | +0x10/+0x14 | — | client-space, GetCursorPos+ScreenToClient |
| ShowPos POINT | +0x18/+0x1C | — | copied from MousePos at show |
| Text buffer | +0x28.. | — | copied at show, cap 0x100 wchars |
| **DelayMs** | +0x228 | **1000** | hardcoded in ctor — NOT INI-driven |
| **DurationMs** | +0x230 | **10000** | auto-hide; hardcoded |
| Tip vector (Westwood DVC) | vtbl +0x234, Items +0x238, Cap +0x23C, Count +0x244, GrowStep +0x248 (=10) | — | registered records |
| id→record pair array | items +0x24C (stride 8 {id, record*}), count +0x250, cap +0x254 (+10 grow), sort-dirty byte +0x258, cache +0x25C | — | duplicate-reject + unregister-by-id index |

### 3.2 Tip record (0x1C bytes) [V: decompile 0x00724580, 0x00724200; cross: TOOLTIP_TEXT_SOURCE report claims 7/10/11]

| Offset | Field |
|---|---|
| +0x00 | ID (u32) |
| +0x04 / +0x08 | X / Y |
| +0x0C / +0x10 | W / H |
| +0x14 | **direct CSF label pointer** (char*) — 0 ⇒ resolve text via GetText(ID) virtual; non-0 ⇒ `StringTable::LoadString(key)` directly (sell/repair use `TXT_SELL_MODE`/`TXT_REPAIR_MODE`) [DOC TOOLTIP_TEXT_SOURCE claims 10/11; my decompile of 0x00724AD0 shows the branch, key-in-ECX detail is that report's asm read] |
| +0x18 | placement byte: 0 = normal, 1 = cameo placement class [DOC claim 10] |

Hit test [V: decompile 0x00724200]: `X <= px && px <= X+W && Y <= py && py <= Y+H` —
**INCLUSIVE both edges** (deliberately ≠ gadget half-open). **First match in registration order**
wins (linear walk; NOT smallest-area). Registration: Register 0x00724580 (duplicate-ID reject,
deep-copies the 0x1C prototype); Unregister-by-id 0x00724730 (hides first if it is the visible
tip, removes from both arrays, frees record) [V: decompiles this lane].

### 3.3 ProcessMessage state machine 0x00724200 [V: decompile this lane]

```
if Enabled != 1: return
match msg:
  WM_TIMER('TTIP' = 0x54544950):
      KillTimer
      if HoveredTip: Hide(); return            # duration expiry (or stale) — auto-hide
      cursor → client coords; record = first inclusive-rect match (reg order) else None
      HoveredTip = record; if Show(): SetTimer(DurationMs)        # auto-hide re-arm
  WM_MOUSEMOVE:
      if DelayMs != 0 and byte[0x00A8F7D8] == 0:
          KillTimer; SetTimer(DelayMs)         # every move RESTARTS the 1000 ms delay
          if HoveredTip: Hide()                # moving hides a visible tip
      else:                                    # DelayMs==0 (cameo highlight) or 0x00A8F7D8 set
          immediate hit-test + Show; if shown: SetTimer(DurationMs)
  WM_{L,R,M}BUTTON{DOWN,UP} (0x201/0x202/0x204/0x205/0x207/0x208):
      KillTimer; if HoveredTip: Hide(), HoveredTip = None         # kill-on-any-button
```
Cadence: ProcessMessage's ONLY caller is the Win32 pump (Process_NetworkMessages 0x005D4D50) —
wall-clock, decoupled from sim/frame [LANE gscreen-chain §7]. The frame loop only DRAWS the tip
(CCToolTip vtbl +0x0C = 0x00478E10 hook in RenderFrame step 8) [V: read_memory 0x007F74C4;
LANE gscreen-chain §5.2].

Enable(false) 0x007241A0 kills timer + hides immediately [DOC SIDEBAR_TIMING §2.4].

### 3.4 Cameo no-delay override (player-visible)

SelectClass::Mouse_Enter (HighlightOn 0x006AB990) **saves DelayMs (+0x228 → +0x22C) and sets it
to 0**; Mouse_Leave (0x006AB9E0) restores. ⇒ while a cameo is highlighted, tooltips show
IMMEDIATELY on mouse-move; tabs/scroll/power keep the 1000 ms delay.
[DOC TOOLTIP_TEXT_SOURCE claims 5/6 + Negative Facts — this supersedes SIDEBAR_TIMING §2.5's
"hovered-pointer save/restore" reading of +0x228.]

### 3.5 Text resolution chain (what text shows, and from where)

Show 0x00724AD0 [V: decompile]: record+0x14 set → CSF[direct label]; else
`CCToolTip::GetText(id) 0x00479050` [V: read_memory hand-decode]:
gate `WWMouse[0x00887640]->vtbl[+0x28]() >= 0` else NULL; then devirtualized chain call
0x006D1800 on the chain object 0x0087F7E8:

- `0x006D1800` (TabClass) = pure passthrough → SidebarClass::GetTooltipText [V: decompile].
- `SidebarClass::GetTooltipText 0x006AC210` [V: decompile]:
  1. PowerClass::GetTooltipText 0x00640450 first: id **999** = power bar →
     `swprintf(CSF#0x29E, House+0x53A4 drain, House+0x53A8 output)` [V: decompile]; else →
     0x00658770 (Radar, passthrough) → DisplayClass 0x004AE4F0: ids **500..0x384** = tactical
     map tips (shroud → CSF#0x13B8; object UIName via vtbl+0x90; disguise/sensor gating;
     byte 0x00A8F7D8 set → "%d,%d" cell-coord readout instead) [V: decompiles].
  2. id 0xC8 → CSF#0x13CD; 0xC9 → CSF#0x13D3; 0xCB..0xCE → CSF#0x13DB/0x13DD/0x13DF/0x13E1.
  3. id ≥ 1000 → bound-check vs visible cameo count
     `((sidebarH − DAT_00B0B4F8 − topAdj(0x1A, or 0x12 if Scenario+0x34B8)) − 7 + sidebarW)/0x32 × 2`
     → GetCameoTooltip(id − 1000); else NULL.
  4. Tail: if `FUN_004E1470() == 0`, result is joined with CSF#0x13F4 via format "%s\n%s"-shape
     (u-string @0x0083FC0C) into buffer 0x00B079B0 — gate identity [UNK §7.5].
- `SidebarClass::GetCameoTooltip 0x006A92E0` [V: decompile]:
  - index = arg + scrollPos(+0x44)×2; gates g_GameActive, idx < count(+0x54), count < 0x4B.
  - CameoEntry stride 0x34; kind(+0x5C)==0x1F (super weapon) → return SWType UIName ptr (+0x60)
    directly.
  - else TechnoType: normal mode (DAT_00884B8C==0) →
    `swprintf(buf 0x00B07BC4, CSF#0xC6E, UIName = type+0x60, cost = type->vtbl[+0x84](g_PlayerPtr))`;
    alternate mode → CSF#0xC6C (cost-only). Then **every 0x20 space → 0x0A newline** in the buffer.
  - (SIDEBAR_TIMING's "Name<LF>Cost<LF>Power" is approximate — the verified args are name + cost;
    the rendered line count is whatever CSF#0xC6E expands to after space→LF.)

NULL text (or empty) ⇒ Show fails ⇒ no tip. In-game tooltip TEXT therefore exists only for:
power bar (999), scroll (0xC8/0xC9), tabs (0xCB..0xCE), cameos (1000+), sell/repair (direct
keys), and the tactical 500..0x384 range.

### 3.6 Registration sites (who owns rects)

- SidebarClass::InitSurface 0x006ABF80 registers tabs/cameos/sell/repair/scroll (re-registered on
  chrome rebuild & side switch — 13 reads of [0x00887368]) [DOC TOOLTIP_TEXT_SOURCE claim 10;
  LANE gscreen-chain §3.3/§7].
- PowerClass 0x006403A0 (id 999) [DOC/LANE].
- Set_View_Dimensions 0x004A8960 re-registers the tactical viewport record
  `{id=500, x=vpX, y=vpY, w=vpW, h=vpH, key=0, byte=0}` (Unregister(500) then Register)
  [V: decompile this lane].
- TabClass::Activate registers/unregisters command-bar ids via 0x00724730 on collapse/expand
  [LANE gscreen-chain §3.2].

### 3.7 A4 Rust-service requirements distilled

Injectable wall clock; 1000 ms delay / 10000 ms duration constants; inclusive-edge rect test;
first-registered-wins; per-move delay restart + hide; kill-on-any-button (incl. middle);
auto-hide re-arm after show; unregister-by-id hides if active; duplicate-id register rejected;
per-record direct-text override (sell/repair) vs resolver callback (ids); cameo-hover
delay-to-zero override hook for the (future A2) cameo gadgets — for A4 expose
`set_delay_override(0)/restore()`; suppression/immediate byte semantics [UNK §7.3]; draw last in
frame (O10); text cap 256 wchars.

---

## 4. A5 — chat/system message TextLabel surface contract

### 4.1 MessageListClass struct (byte offsets; total ≈ 0x149C) [V: decompiles 0x005D3BA0/0x005D3A60/0x005D4430/0x005D4210]

| Offset | Field |
|---|---|
| +0x00 | head TextLabelClass* (the list's OWN gadget list — labels are **never** in the GScreen Buttons list) |
| +0x04 / +0x08 | MessageX / MessageY (label spawn + restack base) |
| +0x0C | MaxMessages (clamped ≤ **14** = 0xE) |
| +0x10 | MaxChars for the edit line (clamped ≤ 0x70 = 112) |
| +0x14 | LineHeight = **0x13 = 19 px** (hardcoded in Init) |
| +0x18 | byte (Init param 9; overflow-related) |
| +0x19 / +0x1A | IsEditing / edit-overlays-message-area flags |
| +0x1C / +0x20 | EditX / EditY (= MessageX/Y when Init edit coords are −1) |
| +0x24 | Edit TextLabelClass* (compose line; gets keyboard Set_Focus) |
| +0x28.. | edit wchar buffer; +0x2B0 cur idx; +0x2B4 start idx (prefix len); +0x2B8 caret char (u16) |
| +0x2BC / +0x2C0 | edit width limits (clamped vs MaxChars) |
| +0x2C4 | MaxWidth px (= Init width − 8) |
| +0x2C8 | **14 text slots × 0x144 bytes (162 wchars each)**, stride 0x144 |
| +0x1480 | u16[14] slot flags: **1 = free, 0 = in use** (Init sets all 1) |

Retail Init call (Set_View_Dimensions 0x004A8960 [V]):
`Init(x = tacticalX+3, y = tacticalY, maxMsg = 6, maxChars = 0x62 (98), 0xE, edit_x = −1,
edit_y = −1, byte = 0, 0x14, 0x62, width = tacticalW − 6)` ⇒ MaxWidth = tacticalW − 14,
6 visible messages, edit overlays message area. Second Init caller: Read_Scenario 0x00684620.

### 4.2 Add_Message FUN_005D3BA0 — exact algorithm [V: decompile + full disassembly this lane]

Signature (thiscall + 7 stack args): `Add_Message(prefix, color, text, scheme_idx, style, timeout, silent)`.

```
1. font = g_GAME_FNT (FUN_004A60D0 [V: decompile] — message labels use GAME.FNT)
   if text == NULL or font == NULL: return NULL
2. compose = prefix? (prefix + L":") : ""        # separator ":" = wide string @0x008306A4 [V: read_memory]
3. budget = MaxWidth(+0x2C4) − text_width(compose) − 8;  if budget <= 0: return NULL
4. fit = fit_chars(text, budget, max_chars = 0x6F (111), word_break = 1)   # FUN_00433F50
   if fit < 0: return NULL;  compose += text[0..fit]
5. count = walk list; +1 if (IsEditing && edit-overlays);  if MaxMessages < count+1:
       evict HEAD (oldest): head = head.remove(); free its slot (flag=1 matched by text-ptr);
       scalar-delete(1)
6. label = new(0x4C) TextLabelClass(compose, MessageX, MessageY, scheme_idx, style | 0x8000)
7. label+0x41 (typewriter flag): offline (mode∉{3,4}) = !silent;
       LAN (3) = !silent && byte[0x00A8D1F8]; WOL (4) = !silent && byte[0x00A8D1F9]
8. deadline (+0x24): timeout == −1 → 0 (never expires);
       else now + timeout, where now = DAT_00887340 + (timer_value − DAT_00887338 if
       DAT_00887338 ≠ −1); timer ticks = timeGetTime() >> 4 (**16 ms units, 62.5/s**,
       pause-aware composite) [V: GetRadarTimer decompile]
9. label+0x28 = color; label+0x38 = MaxWidth
10. find first FREE slot (flag != 0) of the 14; none → delete label, return NULL;
    flag = 0; zero + wcscpy compose into slot buffer; label+0x30 = buffer ptr
11. if !silent: VocClass__PlayAtPos(voc = RulesClass[0x008871E0]+0x6AC, 1.0f, 0)
        # +0x6AC = [AudioVisual] IncomingMessage (=MessageText)
        # [V: parse site 0x0066A7BF..0x0066A80B in RulesClass__ReadAudioVisual writes +0x6AC/+0x6C4]
12. insert at TAIL (vtbl+0x10 Add_Tail); restack: y = MessageY (+LineHeight if editing-overlay);
    for each label head→tail: label.Y = y; y += LineHeight
13. wrap recursion: if fit < len(text): skip control chars (< 0x20) after the break;
    if remainder non-empty: Add_Message(prefix, color, remainder, scheme_idx, style, timeout,
    silent = 1)    # wrapped lines: prefix re-included, NO sound
14. return label
```

### 4.3 Expiry — Manage FUN_005D4430 [V: decompile]; sole caller Main_Tick (once per game tick)

Walk head→tail: deadline(+0x24)==0 → keep; else now (same formula) **> deadline** → remove from
list, free slot (flag=1), scalar-delete; afterwards restack Y exactly as step 12. Returns 1 if
anything expired (caller redraws).

Known timeout values: beacon-placed system messages pass **0xE1 = 225 ticks ≈ 3.6 s**
[V: decompile RadarClass__PlaceBeacon 0x00430BA0, CSF#0x32B/0x32C texts]; chat messages pass the
session global DAT_00A8D748 (copied from DAT_00A8B394 at session init 0x0055EE0A) — ultimate
formula [UNK §7.1]; `[General] MessageDelay=.6` (minutes) exists in rules(md).ini (ini/rules.ini:617).

### 4.4 TextLabelClass (0x4C) [V: decompile ctor 0x0072A440; Draw_Me 0x0072A4A0 hand-decoded from read_memory ×448]

Ctor `(text, x, y, scheme_idx, style)`: GadgetClass ctor(x, y, **w=1, h=1, flags=0, sticky=0**)
⇒ **mask 0 + not sticky + no 0x100 ⇒ Clicked_On always early-outs — labels can never consume
clicks** (G15-mechanized). Fields: +0x24 deadline=0; +0x28 color=0; +0x2C style (caller already
ORs 0x8000); +0x30 text ptr; +0x34 scheme_idx; +0x38 MaxWidth=−1; +0x3C draw-x cache=0;
+0x40 hidden=0; +0x41 typewriter=0; +0x44 reveal count=0; +0x48 last-reveal time=0.
Vtable 0x007F5B44: overrides dtor, Draw_Me 0x0072A4A0, +slot33 Set_Text-shape 0x0072A660.

Draw_Me 0x0072A4A0 (slot +0x6C):
1. byte +0x40 set → return 0 (hidden).
2. base dirty gate (forced or IsToRedraw).
3. color = ColorScheme from `g_ColorSchemeArray[idx +0x34]` (items 0x00B054D4, count 0x00B054E0;
   out-of-range idx clamps to 0); RGB packed from scheme+0x308.
4. font = g_GAME_FNT; width clip vs +0x38 (negative → current surface width).
5. typewriter (+0x41): reveal counter +0x44 += elapsed_wall_ms >> 4 since last draw (+0x48);
   while revealing, plays Voc `RulesClass+0x6C4` = **[AudioVisual] MessageCharTyped**
   (=TextBleep) [V: parse site as above]; passes reveal count to the text draw (char-by-char
   appearance). (Exact per-char sound gating vs text length: content MEDIUM [UNK §7.8].)
6. draws text at (X, Y) to the current draw-target surface [0x00887314]; returns 1.

### 4.5 Draw pump + caller census

- MessageListClass::Draw = FUN_005D49A0 [V: decompile], called from RenderFrame_main step 7
  [LANE gscreen-chain §5.2]: draws the edit line forced + caret if editing, then
  `head->Draw_All(…)` over the message labels — **walk order = insertion order = top-to-bottom
  rows** (G20 analogue on the message list's own list).
- FUN_005D4210 is NOT a second display-add path — it is **Add_Edit** (begin chat compose):
  builds the edit label, Set_Focus (keyboard), +0x19=1; out of A5 display scope [V: decompile].
- Add_Message callers (player-visible census) [V: get_function_callers]: EventClass::Execute,
  net chat receive FUN_0048D1E0, HouseClass Update/MakeAlly/BreakAlliance/MPlayer_Defeated,
  TriggerAction__Execute (map text), RadarClass__PlaceBeacon, LightningStorm Start/Process,
  TypeSelect/waypoint handlers, etc. — fires for every system notification and chat line.

---

## 5. D-B3 — Esc-routing grounding (worktree @ 7b79a186) [WT]

Verified current state:
- `src/ui/shell/controller.rs:192-194` — `on_key` placeholder: returns
  `matches!(key, Enter|Escape) && !kbd_route.is_empty()`; consumes nothing, pops nothing.
- `src/app.rs:2106-2112` — keyboard Esc path: `main_menu_dialog_open()` → `close_main_menu_dialogs()`
  → `return` — **bypasses the controller entirely**.
- `src/app.rs:1950-1955` — `close_main_menu_dialogs` only sets the egui/modal Option fields
  (`exit_confirm_modal = None`, options/movies/campaign) — **never `shell_controller.pop()`**, so
  the 0x120 instance pushed at open stays on the stack until a later `reset_to`/`ensure_active`
  clobbers it.
- `src/app.rs:1929-1941` — `open_exit_confirm_modal` calls
  `shell_controller.ensure_active(DialogId(0x0120), true)`; NOTE `ensure_active`
  (controller.rs:107-112) = `reset_to` = **clears the whole stack** — despite the comment
  "0x120 over the menu's 0xE2", no LIFO stacking actually happens today.
- Cancel via mouse (`handle_exit_confirm_modal_mouse_up`, app.rs:1702-1727) also routes through
  `close_main_menu_dialogs` — same non-pop.
- Controller API available: `push/pop/reset_to/ensure_active/top_id/on_key/kbd_route`
  (controller.rs:77-135).

gamemd contract to satisfy (study §5-D3 + B-prior-doc C5): keyboard routing consults dialogs in
REGISTRATION order; teardown is a LIFO pop with focus restore to the new top (or main window).
Fix shape: open = real `push(0x120)` over the base shell id; Esc (and Cancel/OK teardown) =
route through the controller (`on_key` → resolve → `pop()`), focus/hover state restored to new
top; delete the app.rs bypass branch. All Esc paths (keyboard handler + modal mouse-up) must
converge on the same pop.

## 6. R1 — dead file delete (worktree verified) [WT]

`src/ui/in_game_hud.rs` = 210 lines; `draw_in_game_hud` defined at line 26; the ONLY reference
anywhere in `src/` is the module declaration `src/ui/mod.rs:16` (`pub mod in_game_hud;`).
Action: delete the file + the mod line; verify with `cargo check -p vera20k`.

---

## 7. UNKNOWNS (→ plan "Deferred Open Questions")

1. **Chat-message timeout formula.** DAT_00A8D748 ← DAT_00A8B394 at session init [V:
   0x0055EE0A]; ultimate writer/formula (lobby code, multiple writers) not traced; binding of
   `[General] MessageDelay=.6` to that value UNVERIFIED. Beacon literal 0xE1 = 225 ticks IS
   verified. Implementer: parametrize per-message timeout; default chat value needs one trace.
2. **CSF numeric-id → label mapping** for 0x13CD/0x13D3/0x13DB/0x13DD/0x13DF/0x13E1/0xC6E/0xC6C/
   0x13F4/0x29E/0x13B8/0x233/0x32B/0x32C — our CsfFile is label-keyed; needs one mapping pass
   (also flagged by the plan-grounding-ini lane).
3. **Byte 0x00A8F7D8 identity** — set ⇒ immediate tooltips (skips 1000 ms) AND cell-coord
   readout in tactical tips; SIDEBAR_TIMING calls it the paused flag; writer 0x00537EFA not
   decompiled. Affects A4's "paused = instant tooltips" behavior.
4. **DAT_00884B8C** (cameo tooltip CSF#0xC6C cost-only variant gate) identity (observer mode?).
5. **FUN_004E1470** (sidebar tooltip CSF#0x13F4 suffix gate) identity + player-visible meaning.
6. **WWMouse vtbl+0x28 ≥ 0 gate** in CCToolTip::GetText — semantics not enumerated.
7. **TextLabel +0x28 color field** — written by Add_Message (arg 2), no consumer located in the
   decoded draw path (Draw uses +0x34 scheme); possibly dead. Carry but don't render from it.
8. **Typewriter reveal/sound gating detail** in Draw_Me 0x0072A4A0 — counter advance verified;
   the compare bound for the per-char Voc partially decoded (content MEDIUM).
9. **Edit/compose line** (Add_Edit FUN_005D4210, caret +0x2B8, edit width limits) — verified
   shape, deliberately out of A5 display scope; needed only when chat INPUT ships.
10. **Tip record +0x18 placement byte** — value census verified (1 = cameos, else 0); placement
    MATH in ShowAt 0x00478BA0 not decoded (affects pixel position of the popup, not timing).
11. **MessageList Init param 6 (0xE)** — unused in the decompiled Init body; role unknown.
12. **StripClass::AI scroll-row visual cadence** — one-row-per-tick claim stands on
    SIDEBAR_TIMING §5.2 (patched 2026-05-20); input-side one-page-per-click is authoritative (G23).

## MCP call log (this lane)

decompile_function: 0x005D3BA0, 0x0072A440, 0x005D49A0, 0x005D4430, 0x005D4210, 0x005D3A60,
0x0048D1E0, 0x00430BA0, 0x004A60D0, 0x004A8960, 0x006A5310, 0x0069DCF0, 0x00724000, 0x00724580,
0x00724200, 0x00724AD0, 0x00724730, 0x007784A0, 0x00479050 (read_memory decode), 0x006A92E0,
0x006AC210, 0x006D1800, 0x00640450, 0x00658770, 0x004AE4F0, GetRadarTimer.
disassemble_function: 0x005D3BA0. read_memory: 0x008306A4, 0x0072A4A0×448, 0x007784A0,
0x007F74C4, 0x00479050×96, 0x0055EDE0, 0x0069B960, 0x0066A7B8. get_function_callers:
0x005D3BA0, 0x005D4430, 0x005D3A60. get_xrefs_to: 0x00887340, 0x00A8D748, 0x00A8B394,
0x0083A534, 0x0083A548. search_strings: IncomingMessage/MessageCharTyped.
search_functions: FUN_005d3*.
