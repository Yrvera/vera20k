# Power Bar Pixel Rendering Layout — Trace Report

**Scope:** Pixel-level draw order, frame indices, x/y/width/height, blink substitution.
**Scenario:** max_segments=50, surplus=5, output=3, drain=7, empty=35. Fixture 2: is_flashing()=true.
**Sources:** `gamemd.exe` decompile of `PowerClass__Draw` at `0x0063fb20` (live Ghidra MCP);
`SidebarClass__InitSidebarRect` at `0x006a5200`; `POWER_BAR_RENDERING.md` (verified RE doc).
**Rust sources:** `src/app_sidebar_build.rs` `render_power_bar()` ~L221;
`src/sidebar/layout_spec.rs` L46–90; `src/sidebar/power_bar_anim.rs`.

---

## Pipeline Diagram

```
TRIGGER: render frame requested
 │
 ├─ anim.segment_counts() → (n_empty=35, n_surplus=5, n_output=3, n_drain=7)
 │
 ├─ bar_x = layout.sidebar_x + spec.power_bar_x = sidebar_left + 6.0
 ├─ bar_top = layout.tabs_y + spec.power_bar_top_y = tabs_y + 4.0
 ├─ tile_h = 3.0, bar_w = 10.0
 │
 ├─ Draw loop (y starts at bar_top):
 │   1. 35 × frame 0 (empty/dark)           y += 3 each
 │   2. is_flashing()? → if n_surplus>0: 1 × frame 4 (blink)   y += 3
 │   3. (n_surplus - 1) × frame 1 (green)   y += 3 each
 │   4. n_output × frame 2 (yellow)         y += 3 each
 │   5. n_drain × frame 3 (red)             y += 3 each
 │
 └─ GPU: overlay_pipeline, depth = base_depth - 0.00002 = 0.00046
         chrome sprites drawn at depth 0.00048 → bar wins depth test, renders in front
```

---

## Stage Results

---

### STAGE 1 — POWERP.SHP Frame Index Mapping

```
Input:   POWERP.SHP loaded from sidec0x.mix, 5 frames
gamemd:  frame 0=empty/dark, 1=green, 2=yellow, 3=red, 4=blink  (from Draw_It decompile)
ours:    atlas.powerp_frames[0..=4], same assignment (sidebar_chrome.rs L93-94 comment)
         code: bg=frames[0], surplus=frames[1], output=frames[2], drain=frames[3], blink=frames[4]
Verdict: PASS
```

---

### STAGE 2 — Draw Order Top-to-Bottom (Fixture 1: is_flashing()=false)

```
Input:   n_empty=35, n_surplus=5, n_output=3, n_drain=7, flashing=false
gamemd:  1) 35× frame 0, 2) no blink (flash_counter=0), 3) 5× frame 1,
         4) 3× frame 2, 5) 7× frame 3
ours:    1) 35× frame 0, 2) skip blink (not flashing), 3) 5× frame 1,
         4) 3× frame 2, 5) 7× frame 3
         Code: surplus loop runs for 0..n_surplus (full range, surplus_drawn=0)
Verdict: PASS
```

---

### STAGE 3 — Blink Frame Substitution (Fixture 2: is_flashing()=true)

```
Input:   n_empty=35, n_surplus=5, n_output=3, n_drain=7, flashing=true

gamemd (verified from decompile at 0x0063fb20):
  if (flash_counter > 0):
    uVar3 = flash_counter & 0x80000001   ← signed mod-2 (positive→ &1)
    if uVar3 == 0 (i.e. counter is EVEN):
      CC_Draw_Shape(POWERP.SHP, frame=4, ...)   ← draw blink AT boundary
      y += 3
      iVar2 = 1                                  ← budget consumed
  [NO n_surplus > 0 guard — blink drawn even if surplus_count == 0]
  surplus loop: while iVar2 < surplus_count → draws (surplus - 1) more frame 1

  For this fixture (surplus=5):
    blink drawn, then 4× frame 1, 3× frame 2, 7× frame 3
  For surplus=0 case:
    blink drawn (iVar2=1), surplus loop skips (0 < 0 = false), continues to output

ours (app_sidebar_build.rs L273-298):
  if flashing && n_surplus > 0 {   ← EXTRA guard not in gamemd!
    draw blink; y += 3; surplus_drawn = 1;
  }
  surplus loop: for surplus_drawn..n_surplus

  For this fixture (surplus=5): SAME as gamemd — blink + 4× frame 1 ✓
  For surplus=0 case: OUR CODE SKIPS THE BLINK — gamemd would still draw it ✗

Verdict: FAIL (surplus=0 case)
  In our scenario (surplus=5): PASS — blink draws, 4 remaining surplus frames follow.
  Diverges when surplus==0 with flash active: gamemd draws blink at empty/output boundary,
  we draw nothing. Triggers whenever power exactly meets demand during a power change event.
```

---

### STAGE 4 — Blink Flash Parity (Even/Odd Counter)

```
Input:   flashes_remaining starts at 10 (even), counts to 0

gamemd:  counter & 0x80000001 == 0 → blink (even values only)
         sequence: 10=blink, 9=skip, 8=blink, ..., 2=blink, 1=skip, 0=done
         starts at 10 (even) → first frame IS blinking

ours:    is_flashing() = flashes_remaining > 0 && flashes_remaining % 2 == 0
         same: 10=true(blink), 9=false, 8=true, ..., 2=true, 1=false

Verdict: PASS
```

---

### STAGE 5 — X Offset (Bar Horizontal Position)

```
Input:   Allied (country_index=0) and Soviet (country_index=1) cases

gamemd (from decompile at 0x0063fb20):
  if g_ScenarioClass_Instance[0x34b8] == 0: iVar4 = 5   (Allied)
  else:                                      iVar4 = 0   (Soviet/Yuri)
  draw at x = iVar4 within sidebar surface (= pixels from sidebar left edge)

ours (layout_spec.rs L86):
  power_bar_x: 6.0 (stock, no faction distinction)
  bar_x = layout.sidebar_x + 6.0 = sidebar_left + 6

gamemd Allied: sidebar_left + 5
gamemd Soviet: sidebar_left + 0
ours (all factions): sidebar_left + 6

Allied difference: 1 pixel too far right
Soviet difference: 6 pixels too far right (significant)

Verdict: FAIL
  No per-faction x-offset distinction. Soviet bar is 6px off.
  Allied bar is 1px off. Both wrong.
```

---

### STAGE 6 — Y Start Position (Bar Top Y)

```
Input:   g_SidebarWidth, bar layout offsets

gamemd (from InitSidebarRect at 0x006a5200, Draw_It at 0x0063fb20):
  g_SidebarWidth = 0x9E = 158 (hardcoded Y of side1.shp top = below radar)
  bar y_start = g_SidebarWidth + 0x45 = 158 + 69 = 227
  0x45 = 69 = SIDE1_HEIGHT → bar starts at side1_top + side1_height = TOP OF TABS STRIP
  bar_top_offset_from_side1_top = 69 = side1_height
  → bar_top = tabs_y (zero additional inset below tabs)

ours (layout_spec.rs L87, app_sidebar_build.rs L231):
  bar_top = layout.tabs_y + spec.power_bar_top_y
  power_bar_top_y = 4.0 (stock)
  → bar_top = tabs_y + 4.0

Difference: our bar starts 4px BELOW where gamemd places it.
The top 4px of our bar is below the tabs strip; gamemd's bar starts flush with tabs top.

Verdict: FAIL — power_bar_top_y should be 0.0 for pixel-perfect alignment.
```

---

### STAGE 7 — Bar Width

```
gamemd: CC_Draw_Shape draws at native SHP frame width (not stretched).
        POWERP.SHP width not directly measured here but assumed ~8-10px from tooltip width=8px.
        The tooltip hit-zone width = 8px (from Register_Tooltip at 0x006403a0).

ours:   bar_w = power_bar_width = 10.0 (stock)
        Drawn via push_chrome_sized with explicit [bar_w, tile_h] size.
        If actual SHP frame is narrower than 10px, GPU stretches it (Nearest filter).

Verdict: UNCHECKED — actual POWERP.SHP frame pixel width not confirmed from binary.
         Tooltip zone = 8px suggests native frame may be 8px wide, not 10.
         Known issue documented in POWER_BAR_RENDERING.md §3 "Size Mismatch".
```

---

### STAGE 8 — Segment Height (Tile Height = 3px)

```
gamemd: y += 3 per segment (hardcoded in Draw_It: "iVar5 = iVar5 + 3")
ours:   SEGMENT_HEIGHT_PX = 3 (power_bar_anim.rs L14); power_bar_tile_height = 3.0 (stock)

Note: layout_spec.rs L53-54 comment says "native powerp.shp is 2px" — this comment
is WRONG. The binary hardcodes y+=3, and POWER_BAR_RENDERING.md confirms "3px tall".
The stock default (3.0) is correct; only the doc comment is wrong.

Verdict: PASS (code is correct; doc comment is wrong)
```

---

### STAGE 9 — Z-Order: Chrome Behind, Bar In Front

```
gamemd: Power bar and chrome drawn to the same sidebar DirectDrawSurface via CC_Draw_Shape
        (painter's algorithm, no Z-buffer). Draw order within the class hierarchy:
        PowerClass::Draw_It draws bar segments, then calls RadarClass::Draw_It.
        SidebarClass chrome pieces (side1, side2, side3) order relative to bar: UNCHECKED.
        Expected: chrome SHPs have transparent pixels in the bar column, so order irrelevant.

ours:   chrome at depth 0.00048; bar at fill_depth = base_depth - 0.00002 = 0.00046.
        LessEqual pipeline: lower depth wins. Bar at 0.00046 passes against chrome's 0.00048.
        Bar renders in front of chrome. Functionally correct if chrome has transparent bar column.

Verdict: UNCHECKED for exact draw order, but functional outcome (bar visible in front) is correct.
```

---

### STAGE 10 — Concrete Pixel Rows for Fixture 1 (is_flashing()=false)

```
bar_top = tabs_y + 4 (ours) vs tabs_y + 0 (gamemd) — already flagged in Stage 6.
Starting from our bar_top (noting the 4px offset discrepancy):

Row 0..104  (35 segments × 3px):   frame 0 (empty/dark)
Row 105..119 (5 segments × 3px):   frame 1 (green/surplus)
Row 120..128 (3 segments × 3px):   frame 2 (yellow/output)
Row 129..149 (7 segments × 3px):   frame 3 (red/drain)

Total height covered: 50 segments × 3px = 150px ✓ (= max_segments × SEGMENT_HEIGHT_PX)

Gamemd pixel rows would be identical but shifted 4px up (tabs_y instead of tabs_y+4).
```

---

### STAGE 11 — Concrete Pixel Rows for Fixture 2 (is_flashing()=true, surplus=5)

```
Row 0..104  (35 segments × 3px):   frame 0 (empty/dark)     ← empty band
Row 105..107 (1 segment × 3px):    frame 4 (blink)          ← at empty/surplus boundary
Row 108..119 (4 segments × 3px):   frame 1 (green)          ← remaining surplus
Row 120..128 (3 segments × 3px):   frame 2 (yellow/output)
Row 129..149 (7 segments × 3px):   frame 3 (red/drain)

Both gamemd and ours produce this layout when surplus>0. PASS for this fixture.
```

---

### STAGE 12 — YR vs TS: POWERP.SHP Provenance

```
gamemd: g_PowerBarSHP loaded by PowerClass::Init_IO via LoadSHP("POWERP.SHP").
        Called from SidebarClass::LoadSHPs (0x006a5840) — confirmed active in YR skirmish.
        LoadSHPs is called every game start/sidebar init: not TS-gated.
        POWERP.SHP is a YR sidebar art file (in sidec01/02/02md.mix), not TS-only.

ours:   loaded from the same MIX archive (sidec01.mix/sidec02.mix/sidec02md.mix).

Verdict: PASS — not a TS legacy code path.
```

---

## Failures Summary

| # | Stage | Severity | Description |
|---|-------|----------|-------------|
| F1 | Blink (surplus=0) | Medium | Blink frame not drawn when surplus==0 and flash active. gamemd always draws blink at empty/filled boundary; our guard `if n_surplus > 0` skips it. Fires when power exactly meets demand during a power-change event. |
| F2 | X offset (Soviet) | High | Soviet bar is 6px too far right (gamemd=0px, ours=6px). Visible every match as Soviet/Yuri. |
| F3 | X offset (Allied) | Low | Allied bar is 1px too far right (gamemd=5px, ours=6px). Fires every Allied match. |
| F4 | Y start | Medium | Bar starts 4px below gamemd position (power_bar_top_y=4, should be 0). Fires every match. |

| # | Stage | Status | Description |
|---|-------|--------|-------------|
| U1 | Bar width | UNCHECKED | Actual POWERP.SHP frame pixel width not confirmed; may be 8px (tooltip zone) not 10px. |
| U2 | Z-order (gamemd) | UNCHECKED | Exact gamemd draw order (chrome before/after bar) not traced via SidebarClass::Draw_It. |

---

## Verdict Tally

**PASS: 6 | FAIL: 4 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0**

---

## Top 5 Player-Visible Failures

1. **F2 — Soviet x-offset 6px wrong**: Power bar appears 6px to the right of correct position for every Soviet and Yuri match. Cosmetically obvious. Fix: faction-dispatch power_bar_x (0 for Soviet/Yuri, 5 for Allied).
2. **F4 — Y start 4px low**: Bar top is 4px below correct position every match. Misaligns bar with the tabs strip. Fix: set power_bar_top_y=0.0 in stock spec.
3. **F1 — Blink skipped at surplus=0**: When power exactly meets demand and a power change fires, the blink frame doesn't appear. Visible flash missing for several seconds. Fix: remove `n_surplus > 0` guard from blink draw path.
4. **F3 — Allied x-offset 1px wrong**: Bar 1px right for Allied. Subtle but present every Allied match. Fix: same faction-dispatch as F2.
5. **U1 — Bar width possibly wrong**: If POWERP.SHP native frame is 8px wide (matching tooltip zone) but we render at 10px, texture stretches. Known issue per POWER_BAR_RENDERING.md; needs SHP frame dimension dump to confirm.

---

## Status

**PARTIAL** — z-order gamemd draw ordering unconfirmed via SidebarClass::Draw_It (read-only Ghidra constraint sufficient for core findings; functional outcome is correct). All other scope items traced.
