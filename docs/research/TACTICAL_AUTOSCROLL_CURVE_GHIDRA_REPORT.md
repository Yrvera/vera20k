---
title: Tactical Auto-Scroll Acceleration Curve (Ghidra Research Report)
date: 2026-04-22
---

# Tactical Auto-Scroll — Acceleration Curve & Deadzone Report

**Addresses (primary):**
- `FUN_00692B60` — Edge scroll handler (main function) @ `0x00692B60`
- `FUN_00692F30` — Scroll input dispatcher (calls edge scroll) @ `0x00692F30`
- `GetRadarTimer` @ `0x006C8C40` — **`timeGetTime() >> 4` (16 ms resolution)**
- `FUN_004A9840` — Scroll execute (direction + distance) @ `0x004A9840`
- Speed table @ `0x0083E748` — 9 × int32
- Edge-fraction constants @ `0x0083E738` (X=0.16) / `0x0083E740` (Y=0.21)
- Keyboard scroll multiplier @ `0x0082A034` = float `2.5`

**Confidence:** HIGH for acceleration curve (disassembled instruction-by-instruction),
HIGH for timing basis (GetRadarTimer), HIGH for dead-zone zone computation,
MEDIUM for keyboard scroll distance (two paths; configurable keybinds).

**Active in YR:** Yes.

**Relationship to prior report:** Extends
`ra2-rust-game-docs/ScrollClass_research.md` (1304 lines). That report covered
the structure, field offsets, direction dispatch, map clamping, and circular
buffer — but did NOT derive the **exact per-tick pixel distance formula** or
identify which register indexes the speed table. This report closes both.

---

## 1. Overview

Edge auto-scroll pans the tactical view when the cursor is within 1 px of any
screen edge (or in the sidebar column). A **CoastLevel** ramp provides
acceleration from near-zero to maximum speed; a real-time 16 ms timer drives
the ramp so acceleration feel is consistent regardless of frame rate.

The key — and previously unverified — finding: **the per-frame scroll distance
is indexed by `8 - CoastLevel`, not by direction.** Prior reports called this
out as the "coast multiplier" but the exact table lookup formula and the
register carrying the index (ESI, via SIB byte `B5`) were not decompiled.

---

## 2. The acceleration curve (THE finding)

### 2.1 Per-frame formula

From the instruction at `0x00692E12`:

```
fild  [esi*4 + 0x0083E748]        ; load speed_table[ESI]
fmul  [ecx + 0x5B8]                ; × RulesClass.ScrollMultiplier (default 0.07)
ftol                               ; truncate to int
```

Where `ESI = 8 - CoastLevel` (set by `mov esi, 8; sub esi, [this+0x5548]`).

### 2.2 Speed table at `0x0083E748`

| Index | Value | `× 0.07` | Effective px/frame |
|---|---|---|---|
| 0 | 448 | 31.36 | 31 |
| 1 | 384 | 26.88 | 26 |
| 2 | 320 | 22.40 | 22 |
| 3 | 256 | 17.92 | 17 |
| 4 | 192 | 13.44 | 13 |
| 5 | 128 | 8.96 | 8 |
| 6 | 64 | 4.48 | 4 |
| 7 | 32 | 2.24 | 2 |
| 8 | 16 | 1.12 | 1 |

### 2.3 Coast → distance mapping (CRITICAL for parity)

With `ESI = 8 - CoastLevel`:

| CoastLevel | Index `8-C` | px/frame |
|---|---|---|
| 0 | 8 | **1** |
| 1 | 7 | 2 |
| 2 | 6 | 4 |
| 3 | 5 | 8 |
| 4 | 4 | **13** |
| 5 | 3 | 17 |
| 6 | 2 | 22 |
| 7 | 1 | **26** |

Reaching CoastLevel 0 starts scrolling at 1 px/frame (barely moving); reaching
max CoastLevel gives the full scroll speed. The distance-per-frame sequence
is roughly `1, 2, 4, 8, 13, 17, 22, 26` — a mostly-geometric early ramp that
linearizes toward the top.

### 2.4 Max CoastLevel ≡ effective max speed

`MaxCoastLevel = 7 - ScrollRate` (capped by `if (8 - CoastLevel < ScrollRate+1)`).

| `[Options] ScrollRate` | Max Coast | Max px/frame |
|---|---|---|
| 0 (fastest) | 7 | 26 |
| 1 | 6 | 22 |
| 2 | 5 | 17 |
| 3 (default) | 4 | **13** |
| 4 | 3 | 8 |
| 5 | 2 | 4 |
| 6 (slowest) | 1 | 2 |

So the default-game scroll rate caps at ~13 px/frame. At 60 FPS that's ~780
px/sec (about half a screen width per second on 1280×960).

### 2.5 Ramp timing (16 ms per coast step)

`GetRadarTimer()` is `timeGetTime() >> 4` — **one "radar tick" = 16 ms of
real time**. The coast interval (`DAT_00B05640`) is set to 1 tick, so
CoastLevel increments by 1 every 16 ms while the cursor is at an edge.

| ScrollRate | Ramp 0→Max | Real time to max speed |
|---|---|---|
| 0 | 0 → 7 (7 steps) | 112 ms |
| 3 (default) | 0 → 4 (4 steps) | **64 ms** |
| 6 | 0 → 1 (1 step) | 16 ms |

**Scroll feels crisp:** default settings reach full speed in 64 ms — fast enough
to feel immediate but slow enough that micro-movements to the edge don't cause
runaway scroll.

### 2.6 Deceleration

When the cursor leaves the edge region, the function returns **without
scrolling** but decrements CoastLevel every 16 ms (same rate as acceleration)
until it reaches 0.

```
if !at_edge:
    if (GetRadarTimer() - last_coast_tick) >= coast_interval (1):
        CoastLevel = max(CoastLevel - 1, 0)
        last_coast_tick = GetRadarTimer()
    return  # no scroll this frame
```

**There is no inertia / coasting of scroll itself.** When the cursor moves
off-edge, the tactical view stops at that frame. CoastLevel decay only
preserves "ramp state" — if the cursor re-enters the edge within the decay
window, scrolling resumes at partial speed rather than restarting from 1
px/frame.

---

## 3. Dead zone / direction computation

### 3.1 Edge detection (what counts as "at edge")

```
at_edge = mouse.y < 1
       or mouse.x < 1
       or mouse.x >= composition_width + sidebar_width - 1
       or mouse.y >= composition_height - 1
```

So the **edge band is exactly 1 pixel thick**. Sub-pixel precision — cursor
position `(0, any)` or `(SCREEN_W-1, any)` triggers. The sidebar counts as
"off-screen right" for scroll purposes.

### 3.2 Nine-zone direction computation

When `at_edge` is true OR coast is already ramping, direction is computed
from the cursor's 9-zone position. The axes are partitioned:

- **X zones** (using `DAT_0083E738 = 0.16`):
  - `mouse.x < screen_W × 0.16` → LEFT (ref X = 0)
  - `mouse.x ≤ screen_W × (1 − 0.16) = 0.84 × screen_W` → CENTER (ref X = W/2)
  - `else` → RIGHT (ref X = W − 1)
- **Y zones** (using `DAT_0083E740 = 0.21`):
  - `mouse.y < screen_H × 0.21` → TOP (ref Y = 0)
  - `mouse.y ≤ screen_H × 0.79` → CENTER (ref Y = H/2)
  - `else` → BOTTOM (ref Y = H − 1)

| | X-LEFT | X-CENTER | X-RIGHT |
|---|---|---|---|
| Y-TOP | NW | N | NE |
| Y-CENTER | W | (no scroll direction) | E |
| Y-BOTTOM | SW | S | SE |

The direction is then the `atan2` angle from **screen center** to the
**reference point** `(refX, refY)`:

```
refX = 0, W/2, or W-1  based on zone
refY = 0, H/2, or H-1
angle = atan2(H/2 - refY, refX - W/2)  # in Westwood binary-angle units
```

Quantized to 8 compass sectors for the Scroll call.

### 3.3 Dead zone

The "no scroll direction" case (both axes center) cannot be reached in
practice because `at_edge` is only true when cursor is within 1 px of the
border, which by definition cannot be in both X-CENTER (16-84%) and Y-CENTER
(21-79%) simultaneously.

**There is no ring-shaped dead zone.** The 16 % / 21 % thresholds are NOT
"inactive regions" — they partition the edge stripes into LEFT/CENTER/RIGHT
sectors for directional computation. The actual dead zone is the **entire
screen interior** except the 1-pixel edge band.

**Asymmetric thresholds matter for parity:** X uses 16 %, Y uses 21 %. This is
load-bearing — a cursor at `(0, screen_H × 0.3)` (left edge, 30 % down the
screen) is in X-LEFT + Y-CENTER, so direction = W (pure west). A cursor at
`(0, screen_H × 0.19)` (left edge, 19 % down) is in X-LEFT + Y-TOP, so
direction = NW. The Y threshold is wider because screens are shorter than
they are wide — without asymmetry, pure N/S scrolling would be nearly
impossible.

---

## 4. Why `ScrollMultiplier = 0.07`?

The RulesClass field at `+0x5B8` is an IEEE-754 double read from
`[AudioVisual] ScrollMultiplier` (default 0.07).

**Not a frame-rate compensation.** The multiplier converts the integer
"speed table" value into pixels. Without it, max scroll would be 384 px/frame
(off-screen in one tick at default settings). With 0.07, max is 26 px/frame
— tuned for a ~15 FPS game-logic tick giving ~390 px/sec visual scroll.

**Modifying the INI changes the entire curve proportionally.** Setting
`ScrollMultiplier=0.14` doubles every pixel value (but keeps the 16 ms ramp).

---

## 5. Keyboard scroll

### 5.1 Direction bitmask

From the prior report: `DAT_00ABCE14` is a bitmask accumulator set by
`LogicClass::AI` on arrow key press/release events:

| Key | Bit | Direction |
|---|---|---|
| Up | `0x0001` | N (0) |
| Down | `0x0010` | S (4) |
| Left | `0x0100` | W (6) |
| Right | `0x1000` | E (2) |

Combinations produce diagonals by calling `Scroll()` once per orthogonal
direction in the same frame.

### 5.2 Distance — two paths

Disassembly in `Main_Tick @ 0x0055DCBE` has two distance calculations
guarded by keybind checks:

**Path A (normal arrow keys) — FIXED 52 px/frame:**
```
fild  [esp+0x24]                   ; = 21 (constant at 0x0082A030)
fmul  [0x0082A034]                 ; × 2.5 (double constant)
ftol                                ; = 52
```

The `21` literal is stored into the stack slot by `mov edx, [0x0082A030];
mov [esp+0x28], edx` earlier. Stack slot reshuffles to `[esp+0x24]` after
the query_key_state calls restore esp. **Value is hardcoded — NOT read
from INI.** Distance does NOT vary with ScrollRate.

**Path B (nav keys — Home/End or PageUp/PageDown):**
```
distance = max(MapWidth, MapHeight) × 256
```

This is the **"jump to map edge" scroll** — one frame of this and the
viewport clamps to the far side of the map.

### 5.3 Keyboard scroll bypasses CoastLevel

Keyboard scroll calls `FUN_004A9840` (Scroll) directly — it does not read
or increment CoastLevel. Each keyboard tick produces a fixed distance based
on the path above. No ramp, no deceleration.

**Parity implication:** holding an arrow key gives constant-velocity scroll
from frame 1. Holding the cursor at the edge gives ramped acceleration.
These feel different, and players rely on the distinction — keyboard is for
quick fixed-speed nav, mouse-edge is for smooth camera work.

---

## 6. Tick ordering & interactions

- Edge scroll runs in `ScrollClass::Input` (vtable +0x28) once per input tick,
  BEFORE the game tick advances.
- Deceleration happens in the SAME function that accelerates — one call per
  frame, one decision tree.
- `ScrollInhibited` (at `this+0x555A`) gates edge scroll entirely — returns
  early. RMB drag sets this; also set during smooth camera pan (trigger
  actions) via `AnimSpeed != 0` check at `0x00693060`.
- Keyboard scroll runs in `Main_Tick` after the input phase, so keyboard +
  edge scroll stack additively in the same frame.
- Radar minimap hover suppresses ALL edge scroll (`FUN_0063AB60` early-out).

---

## 7. Constants summary (for parity)

| Constant | Value | Source | Purpose |
|---|---|---|---|
| X edge fraction | `0.16` | double @ `0x0083E738` | X zone partition |
| Y edge fraction | `0.21` | double @ `0x0083E740` | Y zone partition |
| Edge band width | **1 pixel** | hardcoded | "at-edge" test |
| Speed table | `{448, 384, 320, 256, 192, 128, 64, 32, 16}` | @ `0x0083E748` | px before multiplier |
| ScrollMultiplier | `0.07` | `[AudioVisual]` | Final scale factor |
| CoastLevel max | `7 − ScrollRate` | cap expression | Effective peak speed |
| Coast tick period | `16 ms` | `timeGetTime() >> 4` | Accel/decel cadence |
| ScrollRate range | `0..6` | `[Options]` | 0 = fastest |
| ScrollRate default | `3` | OptionsClass ctor | Default binds to 13 px/frame max |
| Keyboard base | `21` | int @ `0x0082A030` | Arrow-key pre-multiplier |
| Keyboard scale | `2.5` | double @ `0x0082A034` | Multiplier |
| Keyboard distance | **52 px/frame fixed** | `ftol(21 × 2.5)` | Arrow key scroll — NOT ScrollRate-dependent |
| Keyboard "jump" distance | `max(W, H) × 256` | computed | Home/End-style nav |

---

## 8. Current Rust implementation status

From `src/` scan (no prior research doc attribution needed):

| Behavior | Rust state |
|---|---|
| Edge-of-screen auto-scroll | **NOT IMPLEMENTED** — camera is fixed / controlled elsewhere |
| Coast level acceleration | **NOT IMPLEMENTED** |
| 16 ms ramp timer | **NOT IMPLEMENTED** |
| Arrow-key scroll | **NOT IMPLEMENTED** |
| RMB drag scroll | **NOT IMPLEMENTED** |
| Nine-zone direction compute | **NOT IMPLEMENTED** |
| Map-bounds clamping (scroll) | May exist in tactical render path, not as a scroll-time check |

This system is a clean greenfield — no existing code to interact with. The
prior report + this report together give a complete spec.

---

## 9. Parity notes — what the player will feel

Ranked by visible impact:

1. **The 16 ms ramp cadence.** Players don't count milliseconds but they feel
   the acceleration. A shorter ramp feels twitchy; a longer ramp feels
   sluggish. Must be exactly 16 ms/step.
2. **1-pixel edge band.** Wider triggers accidental scroll when the cursor
   is near (but not at) an edge; narrower makes the edge hard to hit. RA2
   uses exactly 1 px.
3. **The geometric-then-linear px/frame sequence (1, 2, 4, 8, 13, 17, 22, 26).**
   Any linearization (like `distance = coast × step`) changes feel. Use the
   exact table lookup.
4. **Asymmetric 16 % / 21 % zone thresholds.** Getting pure cardinal-direction
   scrolls requires the wider Y threshold — a symmetric 16/16 would make N/S
   scroll almost impossible because Y center is a larger fraction of the screen.
5. **No inertia.** The view stops immediately when the cursor leaves the edge.
   Decay only preserves CoastLevel state; does not continue motion. Adding
   inertia would feel wrong.
6. **Keyboard scroll constant-velocity.** Holding Up arrow should not
   accelerate. Holding mouse-at-top should.
7. **Sidebar column counts as "edge-right".** Cursor in the sidebar triggers
   east scroll. Players rely on this for "sneak a peek right" muscle memory.

---

## 10. Open questions (resolved in follow-up pass)

### 10.1 Direction encoding — RESOLVED

The atan2 output IS in the dispatcher's direction convention. No remap.

**Westwood `Math__atan2` returns a byte-angle where 0 = North, 64 = East,
128 = South, 192 = West, going CLOCKWISE from North** (compass convention
with Y-up math input). Verified by working through cases:

- Top-center cursor → vector `(0, +H/2)` math-up → byte 0 → quantize → dir 0 = N ✓
- Top-right cursor → vector `(+W/2, +H/2)` math-up-right → byte ~32 → dir 1 = NE ✓
- Top-left cursor → vector `(-W/2, +H/2)` math-up-left → byte ~224 → dir 7 = NW ✓

The quantizer `((byte >> 4) + 1) >> 1 & 7` is a rounding bucket-to-nearest-
octant. No remap needed.

### 10.2 Keyboard scroll `[esp+0x24]` source — RESOLVED

**Value = `0x15 = 21`, hardcoded constant at `0x0082A030`.** Single reference
(from `Main_Tick` at `0x0055DCA8`); NOT INI-backed.

```
mov  edx, [0x0082A030]    ; edx = 21
push eax                    ; ... (key code)
mov  [esp+0x28], edx       ; stack slot = 21
call query_key_state        ; stdcall, restores esp
; ... similar checks for other keys ...
fild [esp+0x24]             ; loads 21 (same address as [esp+0x28] before push)
fmul qword [0x0082A034]    ; × 2.5 (double)
ftol                        ; → 52
```

**Normal arrow-key scroll distance = `ftol(21 × 2.5) = 52 pixels per frame`,
FIXED.** Does NOT vary with ScrollRate. Keyboard scroll is always "fast" —
the `[Options] ScrollRate` setting only affects edge-scroll ramp/cap.

### 10.3 Key 2 modifier in edge scroll — RESOLVED

**"Key 2" = right mouse button.** The Westwood input-query convention used
throughout scroll code: `FUN_0054F5C0(1)` tests LMB, `(2)` tests RMB.

**Behavior:** when RMB is held during edge scroll, `ESI = 8 - CoastLevel`
is modified:
```
if key_state(RMB):
    ESI += 1                # lower effective speed
    if ESI < 4: ESI = 4     # but don't drop below coast-4 equivalent
else:
    if ESI > 8: ESI = 8     # normal cap (redundant; already ≤ 8)
```

With RMB held, effective CoastLevel is capped between `~3` and `~7` (depending
on ScrollRate), producing 4-22 px/frame instead of 1-26. The practical effect:
**edge scroll slows while RMB is pressed** — likely to prevent edge scroll
from running away during "RMB press but not yet dragged far enough to set
ScrollInhibited" window. Minor corner case.

### 10.4 RMB drag vs edge scroll interaction — still open

Prior report mentions RMB drag has its own edge-acceleration (`× 4` within
10 px). This path wasn't re-decompiled in this pass. `ScrollInhibited` gates
edge scroll, so they shouldn't co-fire; but confirm when porting both.

---

## Sources

**Ghidra addresses decompiled / disassembled:**
- `0x00692B60 FUN_00692B60` — edge scroll (decompile + instruction trace of fild/fmul)
- `0x00692E12 fild [ESI*4 + 0x0083E748]` — SIB byte `B5` confirms ESI as index
- `0x006C8C40 GetRadarTimer` = `timeGetTime() >> 4`
- `0x0055DCBE..0x0055DD2A` — keyboard scroll paths (disassembled)
- `0x0083E738 / 0x0083E740` — double constants 0.16 / 0.21
- `0x0083E748..0x0083E768` — speed table (9 ints, read as raw bytes)
- `0x0082A034` — keyboard × 2.5 float constant

**Prior report extended:**
- `ra2-rust-game-docs/ScrollClass_research.md` (sections 3.3, 4.2, 4.4 — now verified)

**INI keys confirmed:**
- `[AudioVisual] ScrollMultiplier = 0.07`  (RulesClass +0x5B8)
- `[Options] ScrollRate = 3`               (OptionsClass +0x10)
- `[Options] AutoScroll = true`            (OptionsClass +0x14)

**Files referenced but not new:**
- `ra2-rust-game-docs/MouseClass_research.md` — input queue that feeds `ScrollClass::Input`
- `docs/GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md` — `DAT_00A8ED9C` as global input inhibit
