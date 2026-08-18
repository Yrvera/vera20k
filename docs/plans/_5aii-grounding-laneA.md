# Slice 5a-ii — Lane A Grounding (Chrome Owner-Draw Assets)

**Date:** 2026-06-15
**Scope:** Read-only doc grounding for the 0xBBB / 0xF5 Options chrome owner-draw assets.
**Primary source doc:** `docs/research/OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`
(status: ghidra/verified). Corroborating docs cited inline per fact.
**Method:** full read of the source doc + `research_search` on SIDEBTTN / SIDEBAR.PAL /
owner-draw button / SDBTNANM + `research_related` on the source doc. No binary re-derivation
this lane; confidence is inherited from the cited Ghidra docs and is labelled per fact.

---

## (1) Owner-draw button TYPE routing

**Confirmed (verified, inherited from source doc + corroborating docs).**

Both 0xBBB and 0xF5 reuse the common shell owner-draw subclass framework. `OptionsClass`
shows RT_DIALOG `0xBBB` when active-scenario byte `0x00A8E9A0 == 1`, else `0xF5`, passing
proc `0x004E1FE0` (source doc "Liveness And Resource Selection", decompile `0x004E1D00`,
asm `0x004E1D2A..0x004E1D47`). The proc delegates to common shell proc `FUN_00622B50`, so
the standard subclass/classifier path applies to both resources.

The button paint TYPE is written into the per-control record at `+0xB0` by `FUN_0060A330`,
gated on the scenario-active predicate `FUN_0069BBE0()`:

- **Active 0xBBB Back `0x686` + Keyboard `0x52C` + Sound `0x52D` → TYPE 2** (active scenario,
  `FUN_0069BBE0()!=0`; type-2 write at asm `0x0060A581`). TYPE 2 draws `DAT_00B0F9EC` =
  **SIDEBTTN.SHP** through **SIDEBAR.PAL** (`FUN_0072F4B0 -> DAT_00B0FBE8`). Source doc
  "Owner-Draw Route Matrix" + "Button Assets, Frames, And Draw Order" + decompile
  `0x00612B70` asm `0x00612EE8..0x00612F56`.
  - Note: the 3 active 0xBBB buttons are Keyboard `0x52C` (425,149), Sound `0x52D` (425,122),
    Back `0x686` (425,346) per the already-verified template; only these route to TYPE 2.
- **Non-active 0xF5 Back `0x686` → TYPE 1** (no scenario, `FUN_0069BBE0()==0`; type-1 write
  at asm `0x0060A47C`). TYPE 1 draws `g_SDBTNANM_SHP` = **SDBTNANM.SHP** through the
  `FUN_0072E2C0 -> DAT_00B0FBDC` convert path (SDBTNANM.PAL family). Source doc same sections,
  decompile `0x00612B70` asm `0x00612EAA..0x00612EE1`.
- **TYPE 3 (MNBTTN.SHP / MAINBTTN.PAL) is NOT reached** for either Options resource:
  `FUN_00609E20` (the type-3 allow-list) excludes `0xBBB` and `0xF5` (source doc "Button
  Classifier And Negative Type-3 Proof"; raw-byte scan of `0x00609E20` lists `0xCE`/`0x120`/
  `0x121` etc., not `0xBBB`/`0xF5`). TYPE 3 = `MNBTTN.SHP` confirmed independently by
  `docs/research/skirmish-ui/MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md` (dialog 0xCE,
  control 0x5AE).

### Frame meaning per type — 0/1/2 vs 2/4/3

The paint fork at `OwnerDraw_Button_00612B70` reads `+0xB0` (`piVar17[0x2C]`) and selects
frames. The two Options-relevant branches use DIFFERENT frame numberings — this is a
load-bearing detail for the Rust atlas mapping:

| Type | SHP / PAL | released/default | hover/flash/timer | pressed | Evidence |
|---:|---|---:|---:|---:|---|
| 1 (0xF5 Back) | SDBTNANM.SHP / SDBTNANM.PAL convert | **frame 2** | **frame 3** | **frame 4** | source doc; decompile `0x00612B70` asm `0x00612EAA..0x00612EE1` |
| 2 (0xBBB buttons) | SIDEBTTN.SHP / SIDEBAR.PAL convert | **frame 0** | **frame 2** | **frame 1** | source doc; decompile `0x00612B70` asm `0x00612EE8..0x00612F56` |

- **TYPE 2 frame mapping (the 0xBBB case): frame 0 = released, frame 1 = pressed, frame 2 =
  timer/highlight.** Verified, source doc "Button Assets, Frames, And Draw Order" row "type 2".
  This is the canonical answer to the task's "what does frame 0/1/2 mean" question:
  **0 = released/normal, 1 = pressed, 2 = timer/highlight (flash) state** — NOT 0/1/2 =
  normal/hover/pressed. Hover is the timer-driven flash frame (2), pressed is frame 1.
- **TYPE 1 frame mapping (the 0xF5 case): frame 2 = released, frame 3 = flash/hover, frame
  4 = pressed.** Verified and cross-confirmed by:
  - `docs/research/BUTTON_FADE_EFFECT_VISUAL_GHIDRA_REPORT.md` §3 (decompile `0x00612B70`):
    `iVar14==1` → `g_SDBTNANM_SHP`, `local_f0=0x2` default, `0x3` when flash bool `+0xC5` set,
    `0x4` when pressed. The flash bool toggles at 1 Hz via a 1000 ms `SetTimer` (message
    `0x4DC` enables; WM_TIMER `0x113` flips `+0xC5`).
  - `docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`
    "Verified Finding": released=2, pressed=4, hover/timer=3.
  - `docs/research/traces/MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md` Stage 1 (PASS).

  The source doc's "default 2, pressed 4, timer/highlight 3" column (line 104) is the SAME
  mapping, just listed in a non-monotonic column order — NOT a contradiction. Flag: do not
  read it as "frames 2/4/3 in sequence."

### CSF caption over the glyph — drawn AFTER the art, same draw call

**Confirmed (verified).** Draw order inside the button callback is: select SHP/convert +
frame, call `CC_Draw_Shape` to blit the glyph, THEN draw the button caption text with
`FUN_00621040` if a text pointer is present and the custom-image bypass is not set (source
doc "Button Assets, Frames, And Draw Order", final paragraph). So the CSF caption
(e.g. Back `0x686` = "GUI:..." back string, Keyboard, Sound captions) is painted ON TOP of
the SHP glyph, centered in the button rect via `FUN_00621040`.

- Pressed state SINKS the text rect (text offset down on press); this is part of the same
  callback (source doc). For TYPE 1 the press also corresponds to a +2 px Y text shift in the
  generic shell pattern (see BUTTON_FADE / main-menu trace family).
- `FUN_00621040` is the shell text wrapper (rect/color/v-center contract verified in
  `docs/research/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`).
  Enabled label color is the shell default; disabled uses the shell disabled color.
- **The final disabled-alpha overlay is gated to TYPE 0 only** — it does NOT apply to TYPE 1
  or TYPE 2 Options buttons (source doc). Relevant because VisualDetails `0x52B` and its
  labels are WS_DISABLED in the template, but VisualDetails is a TRACKBAR, not one of these
  buttons; none of the three active buttons takes the disabled-alpha path.

---

## (2) Button screen-X anchoring

**Confirmed (verified).** Resource DLU rects are the input; native then runs the common
child-resize dispatcher `ResizeShellChildControl_0060C0C0`, which fans out per control to one
of the `FUN_0060Bxxx` helpers, each followed by `FUN_0060B950` (source doc "Rect Anchoring").

Helper routing for the scoped Options controls (source doc, with asm contexts):

| Control(s) | Predicate | Resize helper | Then | Asm context |
|---|---|---|---|---|
| 0xBBB active `0x52C`/`0x52D` | `FUN_00608CD0` true, record kind 0 | `FUN_0060B000` | `FUN_0060B950` | `0x0060C1B9..0x0060C1CF` |
| 0xBBB/0xF5 Back `0x686` | `FUN_00609730` true | `FUN_0060B350` | `FUN_0060B950` | `0x0060C21C..0x0060C22E` |
| Title/static allow-list `0x694` (+0xF5 `0x71C`) | static allow-list | `FUN_0060B1D0` | `FUN_0060B950` | (source doc) |
| All other 0xBBB/0xF5 controls (trackbars, checkboxes, value labels) | exclusion list | `FUN_0060B7A0` | `FUN_0060B950` | `0x0060C3A9..0x0060C3F8` |

### Right-edge button anchoring formula (the buttons) — SIDEBTTN canvas dependency

**Confirmed (verified).** `FUN_0060B000` (the `0x52C`/`0x52D` helper) and `FUN_0060B350`
(the `0x686` Back helper) BOTH branch on `FUN_0069BBE0()` and anchor the button's X to the
right edge using the button SHP's canvas width MINUS a constant offset:

- **Active scenario (0xBBB):** use **`DAT_00B0F9EC` (SIDEBTTN.SHP) dimensions** and right-edge
  offset **`0x93` (147)**.
  - `FUN_0060B000` active branch: reads `0x00B0F9EC`, subtracts `0x93` — asm `0x0060B124..0x0060B13F`.
  - `FUN_0060B350` active branch: reads `0x00B0F9EC`, subtracts `0x93` — asm `0x0060B3FC..0x0060B414`.
- **No scenario (0xF5 shell):** use **`g_SDBTNANM_SHP` (`0x00B0FAC4`) dimensions** and right-edge
  offset **`0x9C` (156)**.
  - `FUN_0060B000` no-scenario branch: reads `0x00B0FAC4`, subtracts `0x9C` — asm `0x0060B0AF..0x0060B0C3`.
  - `FUN_0060B350` no-scenario branch: reads `0x00B0FAC4`, subtracts `0x9C` — asm `0x0060B3AE..0x0060B3C4`.

The "offset 0x93" the task asks about is this **right-edge button X subtrahend in the active
SIDEBTTN path**. It is a property of `FUN_0060B000`/`FUN_0060B350` (the button anchoring
helpers), keyed on the SIDEBTTN.SHP canvas width — i.e. the button is positioned relative to
the panel's right edge by the SHP dimensions, not the DLU rect's literal X. The exact closed
form (e.g. `x = panel_right - (shp_width - 0x93)`) is not transcribed verbatim in the source
doc; the doc states the two ingredients (read `DAT_00B0F9EC` dimensions; subtract `0x93`) with
asm citations but does not spell the full arithmetic. **Flag: the precise sign/term order of
the formula is INFERRED at the doc level — read the asm at `0x0060B124..0x0060B13F` /
`0x0060B3FC..0x0060B414` before hardcoding the exact expression in Rust.** Confidence: helper
identity + global + offset constant = VERIFIED; full formula = PARTIAL (needs asm walk).

### Centered vs right-edge — which controls get which

**Confirmed (verified).**

- **Right-edge button anchoring (SIDEBTTN-canvas dependent):** ONLY the buttons — active
  `0x52C`/`0x52D` (via `FUN_0060B000`) and Back `0x686` (via `FUN_0060B350`). These are the
  controls whose screen-X depends on the SIDEBTTN.SHP (or SDBTNANM.SHP) canvas size and the
  `0x93`/`0x9C` offset.
- **Centered screen offsets:** the "ordinary" controls (trackbars `0x529`/`0x52A`/`0x52B`,
  checkboxes `0x601`/`0x602`/`0x604`, value labels) go through `FUN_0060B7A0`. **`FUN_0060B7A0`
  only moves these when `FUN_0069BBE0()!=0` (active 0xBBB)** — it adds centered screen offsets
  and clamps to zero. For shell/non-active `0xF5` it returns WITHOUT moving them (they stay at
  their resource-converted DLU positions). Source doc "Rect Anchoring".
- **Title `0x694` finalizer:** `FUN_0060B950` has an Options-specific title nudge — when
  non-active (0xF5) it applies the shared title y/height nudge; when active (0xBBB) it returns
  WITHOUT that title nudge. No scoped non-title trackbar/checkbox one-pixel Options nudge was
  found. Source doc "Rect Anchoring".

So: in the active 0xBBB layout, ordinary controls are centered (B7A0) and the three buttons
are right-edge-anchored to the SIDEBTTN canvas; in the shell 0xF5 layout, ordinary controls
stay at resource positions (no B7A0 move), the title gets the nudge, and Back is right-edge-
anchored to the SDBTNANM canvas with offset `0x9C`.

---

## (3) OVERLAY composition — no opaque full-screen panel art

**Confirmed (verified).** There is NO evidence of an opaque full-screen Options panel
background blit. The source doc enumerates the full live paint surface for both resources —
button glyphs (TYPE 1/2 SHP), checkbox icons (PCX), trackbar plaque/rail/thumb (PCX), and
text statics — and finds NO opaque panel-art route:

- Statics use the TEXT path (`FUN_00621040`), normal color `DAT_00AC18A4`, disabled
  `DAT_00AC1CB4`. Image/PCX/SHP static subpaths exist in the static callback but **no
  `0x004E1FE0` message activates them** for ordinary Options statics (source doc "Trackbar,
  Checkbox, And Static Paint Paths"). Footer `0x695` is "GUI:Blank" text, not art.
- The only `0xF5` image-static candidate is `0x71C` — resource-present and allow-listed in the
  common helpers, but **no activation message was found** in `0x004E1FE0`; its visible image
  behavior is explicitly DEFERRED (source doc "Remaining Uncertainty"). This is shell-only
  (0xF5), not in active 0xBBB.
- Consequence: Options chrome composes as discrete owner-draw widgets layered OVER the
  existing scene/shell background — it is an overlay of glyph + icon + plaque + text pieces,
  NOT a single opaque dialog bitmap. Confidence: VERIFIED for the active 0xBBB resource;
  the one open hole (0xF5 `0x71C`) is shell-only and image activation unproven.

---

## (4) Palette remap / draw-order facts for the glyphs

**Confirmed (verified).**

Palette / ConvertClass per button type:

- **TYPE 2 (0xBBB buttons): SIDEBAR.PAL.** SIDEBTTN.SHP (`DAT_00B0F9EC`) is converted through
  the SIDEBAR.PAL ConvertClass `DAT_00B0FBE8` via `FUN_0072F4B0`. Source doc rows + decompile
  `0x00612B70`. SIDEBAR.PAL ConvertClass identity independently confirmed:
  - `DAT_00B0FBE4` = SIDEBAR.PAL raw buffer, `DAT_00B0FBE8` = its ConvertClass, table entry
    `[0x00844BF0]` →
    `docs/research/SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md` §3.1/§3.2 and
    `docs/research/ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md` §10 ("`0xb0fbe8` =
    SIDEBAR.PAL always (all sides)" — NOT side-branched, so the 0xBBB button palette is the
    same for Allied/Soviet/Yuri).
- **TYPE 1 (0xF5 Back): SDBTNANM.PAL family** via `FUN_0072E2C0 -> DAT_00B0FBDC` (source doc;
  cross-confirmed by BUTTON_FADE §3 and the main-menu SDBTNANM dispatch docs).

Asset-identity correction (load-bearing, VERIFIED):

- `DAT_00B0F9EC` is loaded from **SIDEBTTN.SHP**, NOT SIDE2B.SHP. Loader `FUN_0072FA10`:
  string table `[0x00844CFC] -> 0x008450F4 = "SIDEBTTN.SHP"`, SHP loader call `0x0072FAC4`,
  result stored to `0x00B0F9EC` at `0x0072FAD4`. `SIDE2B.SHP` is `[0x00844D20] -> 0x008450A4`
  and stores to a DIFFERENT global `0x00B0FA00` (asm `0x0072FB1D..0x0072FB3D`). Source doc
  "DAT_00B0F9EC Asset Identity Correction".
- **Flag (stale-doc fix already prescribed, NOT a new finding):** the source doc's "Stale Doc
  Replacement Wording Found" instructs replacing the `DAT_00b0f9ec = SIDE2B.SHP` row in
  `docs/research/SIDEBAR_RADAR_POSITIONING.md` with the SIDEBTTN.SHP mapping. Lane A does not
  patch other docs; surfacing only.

Draw order (per button, VERIFIED): (1) select SHP + convert (SIDEBAR.PAL or SDBTNANM.PAL) +
frame; (2) `CC_Draw_Shape` blit the glyph; (3) `FUN_00621040` caption text on top (if text
ptr present and custom-image bypass not set), with pressed-state text-rect sink; (4)
disabled-alpha overlay — **TYPE 0 only**, so skipped for these Options buttons. Source doc
"Button Assets, Frames, And Draw Order".

Other glyph palettes (context, VERIFIED in source doc, for completeness — not the buttons):
- Checkboxes: default PCX icons `cue_i.pcx` / `cce_i.pcx`, 18×18, drawn at checkbox origin;
  label text shifted right by `0x1A` (26 px). No variant message sent by `0x004E1FE0`.
- Trackbars: PCX `trofl.pcx` / `trofm.pcx` / `trofr.pcx` (plaque/rail) + `trakgrip.pcx`
  (thumb), plus primitive bevel `FUN_006208F0` + value text `FUN_00621040`.

---

## Caveats, TS-legacy, and stale flags

- **No TS-legacy concern in this lane.** Both 0xBBB and 0xF5 paths are live YR shell/Options
  chrome (active-byte selected). No fog-of-war, tunnel, or SpecialFlags-gated path is in scope.
- **Active-vs-shell is the key gate, not TS:** 0xBBB requires `FUN_0069BBE0()!=0` (in-game
  over an active scenario); 0xF5 is the menu/shell Options. Both are reachable in normal YR.
- **0xF5 `0x71C` image static — DEFERRED / UNCHECKED.** Resource-present + helper-allow-listed
  but no activation message found; visible image behavior unproven. Shell-only.
- **TYPE-2 right-edge formula — PARTIAL.** Helper (`FUN_0060B000`/`FUN_0060B350`), global
  (`DAT_00B0F9EC`), and offset (`0x93`) are VERIFIED; the exact arithmetic expression is not
  transcribed verbatim — walk asm `0x0060B124..0x0060B13F` / `0x0060B3FC..0x0060B414` before
  hardcoding.
- **Exact pixel/canvas dims — UNCHECKED.** Decoded SIDEBTTN.SHP canvas dimensions were NOT
  dumped; native reads the SHP header dynamically. Rust should parse the real retail
  SIDEBTTN.SHP header at pack/load time, not hardcode width/height. (Source doc "Remaining
  Uncertainty".)
- **Confidence inheritance:** all "VERIFIED" facts here are inherited from ghidra/verified
  source docs (decompile/asm-cited), not re-derived from the binary this lane. The source doc
  itself flags that no retail framebuffer RGB diff was captured — route/asset/frame/anchor
  mechanisms are proven, pixel-exact equality is not.

---

## Source docs used

- `docs/research/OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md` (primary)
- `docs/research/BUTTON_FADE_EFFECT_VISUAL_GHIDRA_REPORT.md` (TYPE-1 frame/flash logic)
- `docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`
- `docs/research/traces/MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md`
- `docs/research/MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md` (type-1 SDBTNANM dispatch)
- `docs/research/skirmish-ui/MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md` (TYPE-3 = MNBTTN, negative)
- `docs/research/SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md` §3.1/§3.2 (SIDEBAR.PAL ConvertClass)
- `docs/research/ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md` §10 (0xb0fbe8 = SIDEBAR.PAL all sides)
- `docs/research/RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md` (B000/B350/B1D0/B7A0/B950 family)
- `docs/research/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md` (FUN_00621040 caption)
