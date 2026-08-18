# Spine Rung 11 — K. Periodic spawn re-anchor / retreat driver

**Driver:** `FUN_0054e4d0` @ `0x0054e4d0`, called `__fastcall` with `ECX = 0xabc5f8`.
**Body site:** `LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`, call at `0x0055b4eb`–`0x0055b4f0`
(`MOV ECX,0xabc5f8` / `CALL 0x0054e4d0`), between `BombClass__UpdateAll` (`0x00438bf0`,
rung J) and `FUN_0055bb40` (`0x0055bb40`, rung L TeamClass temp-list build). Order confirmed.

Verified via `decompile_function 0x0054e4d0`, `disassemble_function 0x0054e4d0`,
`disassemble_function 0x0055afb0` (the call-site bytes), `get_function_callers 0x0054e4d0`
(sole caller = `LogicClassPerTickUpdateLiveVector`).

---

## Purpose (one line)

Walks the global **spawn-retreat / re-anchor list** at `0xabc5f8` and, on a ~30-frame
internal timer, steps each registered spawn object (Aircraft-Carrier hornet,
Dreadnought/Boomer missile, etc.) through its return/re-anchor motion toward a stored cell.

## What it walks / does

Receiver `this = 0xabc5f8` is a small global struct that is BOTH a `RateTimer`-style
countdown (first 3 dwords) AND a dynamic-vector head:

- `this+0x00` (`*param_1`)  = timer start frame (sentinel `0xffffffff` = inactive)
- `this+0x04` (`param_1[1]`) = timer "current"/payload dword (written from `local_8`)
- `this+0x08` (`param_1[2]`) = timer duration (reset to `0x1e` = 30 frames each fire)
- `this+0x10` (`param_1[4]`) = entries array base (node ptrs)
- `this+0x1c` (`param_1[7]`) = entry count

Per entry node (allocated in `SpawnRetreat__Push` @ `0x0054e3b0`, an 8-byte node):
- `node[0]` = spawn-owner object pointer (`ESI`/`*piVar1`)
- `node[1]` = stored target/return cell (or 0 → compute facing-step this tick)

For each entry the driver:
1. sets `*(obj+0x2fc) = 1` (a redraw/active flag; `piVar1[0xbf]`),
2. if `node[1] == 0`: reads `RateTimer__Current(obj+0x388)` → facing nibble
   `(val>>0xc)+1 >>1 & 7`, calls `MapCoord_StepByDir_GetCell(facing)` to derive a neighbor
   cell, then virtual `vt+0x1bc` (read coord), `vt+0x3c8` (set/move toward cell),
   `vt+0x1e8(1,0)` (mark dirty/redraw),
3. else (`node[1] != 0`): just `vt+0x3c8(node[1])` then `vt+0x1e8(1,0)`.

Verified via `decompile_function 0x0054e4d0` and `disassemble_function 0x0054e4d0`
(call targets `0x004c93d0` = `RateTimer__Current`, `0x00481810` = `MapCoord_StepByDir_GetCell`;
`get_function_callees 0x0054e4d0` lists exactly those two).

**List identity** — `get_xrefs_to 0xabc5f8` shows the writers/registrants:
`SpawnManagerClass__AI` (`0x006b7a32`–`0x006b7a51` writes `[0xabc5f8]=frame`,
`[0xabc5fc]=objptr`, `[0xabc600]=2` and calls `0x0054e3b0` ECX=0xabc5f8 to push),
`SpawnManagerClass__Kill_All_Spawns`, `SpawnManagerClass__ClearAllTargets`,
`Detach_From_All_Lists`. So the list holds spawn objects queued for retreat/re-anchor.
The push helper `SpawnRetreat__Push` @ `0x0054e3b0` (verified via `decompile_function`)
allocates `operator_new(8)` `{objptr, cell}`, gated on `*(obj->Type+0xd68) != 0`
(spawn-capable flag), and appends at `[this+0x10][this+0x1c++]`.

---

## Gate / mode condition

**Internal self-timer only — NOT game-mode gated.** From `0x0054e4d0`:

```
EDX = this[0];  EAX = this[2](dur)
if (EDX != -1) {                 // timer running
    ECX = g_CurrentFrameCounter(0x00a8ed84) - EDX
    if (ECX >= EAX) goto FIRE    // elapsed
    EAX = EAX - ECX
}
if (EAX != 0) return;            // not yet due
FIRE: this[0]=frame; this[1]=local_8; this[2]=0x1e;  // re-arm 30 frames
      if (this[0x1c] > 0) { ...walk entries... }
```

So it fires when the 30-frame interval elapses (or `this[0] == -1` inactive sentinel with
`dur==0`). When the entry count `this[0x1c] <= 0` the body is a no-op apart from re-arming
the timer. The ladder's "internal self-timer (this+0/+2), ~30-frame interval" is **confirmed**
(constant `0x1e` at `0x0054e500`). Unconditional call site (no mode/Special bit guard around
`0x0055b4eb`).

---

## RNG: NONE drawn in this rung

**No RNG draw.** Neither `FUN_0054e4d0` nor its two direct callees touch any RNG instance:

- `RateTimer__Current` @ `0x004c93d0` (verified `decompile_function` + `disassemble_function`)
  is a pure timer-interpolation read: from a 16-bit rate struct (`[ECX]`, `[ECX+4]`=start,
  `[ECX+8]`=dur, `[ECX+0x14]`=step) it computes a current value vs `g_CurrentFrameCounter`
  and writes it via the out-param. No call to `Scen->Random` / `g_MainRng` / `g_MapGenRng`.
- `MapCoord_StepByDir_GetCell` @ `0x00481810` (verified `decompile_function`): adds
  `g_DirectionOffsets[dir]` to a packed cell and fetches the neighbor `CellClass`. Pure map
  arithmetic, no RNG.

The facing value fed to the step is the **timer-interpolated** value, so the per-tick
re-anchor motion is fully deterministic. The three per-entry virtuals (`vt+0x1bc`, `+0x3c8`,
`+0x1e8`) are coordinate read / set-position / mark-dirty on the spawn object; they are not
RNG draws in this path. **rng_stream = none; rng_notes = none.**

> Note: the ladder text "consumes `RateTimer__Current` @ 0055b4eb" is accurate as to the
> function consumed, but `RateTimer__Current` is a timer read, **not** an RNG draw. This rung
> contributes **zero** to the per-tick RNG-draw order.

---

## Active-in-YR / Tiberian-Sun legacy

**Active in YR: yes (conditional on a spawn being recalled).** The list is populated by
`SpawnManagerClass__AI` for spawn-capable launchers — Aircraft Carrier (hornets),
Dreadnought / Boomer (missiles) — all standard YR units. The driver runs every tick but
only does work on the ~30-frame interval AND when entries are queued (a spawn is mid-retreat
/ re-anchor). **Not** TS-legacy: SpawnManager spawn/retreat is live RA2/YR behavior, visible
when a carrier's planes return or a sub's missiles are recalled.

When no spawn is retreating (the common case most ticks), the rung re-arms its timer and
returns — observably inert but still part of the fixed tick order.

---

## Summary for the ladder

- order: 11
- label: K. Periodic spawn re-anchor / retreat driver
- driver: `FUN_0054e4d0` @ `0x0054e4d0`, ECX=`0xabc5f8`
- gate: internal 30-frame self-timer (`this+0` start vs `g_CurrentFrameCounter`, `this+8`
  dur, re-arm `0x1e`); call site unconditional; body no-op when `this+0x1c <= 0`
- draws RNG: **no** — `RateTimer__Current` is timer interp, not RNG
- rng_stream: none
- active in YR: yes (when a carrier/dreadnought/boomer spawn is recalled)
- TS legacy: no
