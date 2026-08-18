# Spine Rung #15 — O. LaserDrawClass::UpdateAllAI

**Driver:** `0x00550150` `LaserDrawClass__UpdateAllAI` (void, no args, __cdecl)
**Body callsite:** `0x0055AFB0` `LogicClass::PerTickUpdate` (decomp label
`LogicClassPerTickUpdateLiveVector`), direct `CALL LaserDrawClass__UpdateAllAI`
between `FUN_005ff390()` (Rung N) and `LightningStorm__Process()` (Rung P).

**Verified 2026-06-25** from gamemd.exe (image base 0x00400000) via Ghidra MCP:
`decompile_function 0x00550150`, `disassemble_function 0x00550150`,
`decompile_function 0x0055AFB0`, `get_function_callers 0x00550150`,
`get_function_callees 0x00550150`, `read_memory 0x00abc878 / 0x00abc87c`,
`get_xrefs_to 0x00abc888`, `decompile_function 0x007c8b3d / 0x007c93e8`,
`disassemble_function 0x005509f0`.

---

## Purpose (one line)

Per-tick AI for every active beam in the global LaserDrawClass array — advances each
beam's animation step / repeat timer, flips the half-tick blink flag, and culls
(removes + `operator delete`) any beam whose `AnimStep` has reached its `DurationTotal`.

## What it walks / does

Reverse loop over `g_LaserDraw_Array` (`0x00abc87c`), index from
`g_LaserDraw_Count - 1` down to 0 (count at `0x00abc888`). Per element
(`LaserDrawClass*`, struct layout from `LASER_DRAW_CLASS_GHIDRA_REPORT.md §2`):

1. Read `SpawnFrame` (`+0x08`) and `RemainingTicks` (`+0x10`).
   - If `SpawnFrame != -1` and `elapsed = g_CurrentFrameCounter(0x00a8ed84) - SpawnFrame`
     `< RemainingTicks` → still alive this cycle; `RemainingTicks -= elapsed`; fall to
     the "alive" branch which writes `IsActive(+0x04) = 0`.
   - Else (expired / `SpawnFrame == -1`): if `StepIncrement(+0x14) == 0` → `IsActive = 0`
     (will be destroyed); else repeat-trigger: `IsActive = 1`, `AnimStep(+0x00) +=
     StepIncrement(+0x18)`, `SpawnFrame = g_CurrentFrameCounter`,
     `AnimParamB(+0x0C) = local_8` (uninitialized stack slot — carried verbatim),
     `RemainingTicks(+0x10) = StepIncrement(+0x14)`.
2. If `ToggleFlag(+0x50) != 0` → flip `ToggledState(+0x51)` (`state = (state == 0)`),
   the half-tick blink.
3. Expiry cull: if `AnimStep(+0x00) >= DurationTotal(+0x4c)` and element non-null →
   call DynamicVector "find index" functor `(*(DAT_00abc878 + 0x10))(&element)` to
   locate the slot, shift the tail of `g_LaserDraw_Array` down by one, decrement
   `g_LaserDraw_Count`, then `FUN_007c8b3d(element)` → `FUN_007c93e8` → `HeapFree`
   (`operator delete`).

## Gate — CONFIRMED unconditional

The call in `0x0055AFB0` is a plain `CALL`, no surrounding condition (verified in
decomp body; sits between `FUN_005ff390()` and `LightningStorm__Process()`). The
driver body opens with `iVar2 = g_LaserDraw_Count; do { iVar2--; if (iVar2 < 0)
return; ... }` — a self-bounded reverse loop that is a no-op when the array is empty.
No mode gate, no SpecialFlags gate. Matches the ladder description.

## RNG — NONE in this rung

**`draws_rng = false`.** `get_function_callees 0x00550150` returns exactly one
callee: `FUN_007c8b3d` (the `operator delete` wrapper). The `disassemble_function`
shows only two `CALL` targets in the whole body: `[EAX+0x10]` (the DynamicVector
find-index functor at `DAT_00abc878`) and `0x007c8b3d` (free). Neither touches an
RNG. No `Scen->Random`, no `g_MainRng`, no `g_MapGenRng` receiver is loaded anywhere
in the function.

Note for lockstep bookkeeping: the LaserDrawClass spread-jitter RNG (`RandomRanged`
per color channel when `LaserOuterSpread != 0`) lives in the **draw** path
(`LaserDrawClass::Draw` / `DrawBeamSpecial @ 0x005509F0`), which is invoked from
`TacticalClass::Draw` (the render pipeline), NOT from this logic-tick driver. So the
spread RNG is render-side and does not participate in the sim per-tick draw order.
The `DrawBeamSpecial` special path (`0x005509f0`) disassembled here contains no
`RandomRanged` either; spread jitter is in the non-special `Draw` branch. Render-side
RNG stream is out of scope for this rung's lockstep contract — Rung O itself
contributes **zero** draws to the per-tick RNG sequence.

## Active-in-YR — YES (visible); NOT Tiberian Sun legacy

The driver is unconditionally invoked every sim tick. LaserDrawClass is the live
backing renderer for Prism Tower beams, Mirage Tank, IFV/Battle-Fortress laser, Tank
Destroyer, Robot Tank, Disk Laser, and the Yuri Railgun trail — all stock YR weapons
with `IsLaser=yes`/`IsBigLaser=yes` (see `LASER_DRAW_CLASS_GHIDRA_REPORT.md §1, §6`).
The beams are player-visible. No `SpecialFlags` gating and no TS-only code path in the
tick loop. The class layout predates the LayerClass system (TS-era POD with no vtable),
but that is an intentional, still-live draw-order behavior in YR, not dormant code.
When no laser is active (no IsLaser weapon has fired recently) the loop is a no-op,
but that is data-driven idleness, not a disabled feature.

## Globals referenced (verified)

| Address | Role | Evidence |
|---------|------|----------|
| `0x00abc87c` | `g_LaserDraw_Array` (array of `LaserDrawClass*`) | `read_memory` (0 in static image; runtime-populated); xrefs in DestroyAll/UpdateAllAI/DrawAll/Constructor |
| `0x00abc888` | `g_LaserDraw_Count` (active count) | `get_xrefs_to 0x00abc888` (READ at 0x00550150, WRITE at 0x005501ff) |
| `0x00abc878` | DynamicVector functor table ptr (find-index hook at `+0x10`) | `read_memory` (0 in static image; runtime vtable); called at 0x005501ea |
| `0x00a8ed84` | `g_CurrentFrameCounter` | used as timer base at 0x0055017e / 0x005501a4 |

`FUN_007c8b3d (0x007c8b3d)` → `FUN_007c93e8 (0x007c93e8)`: confirmed `operator delete`
(falls through to `HeapFree(DAT_00b78b9c, 0, param_1)`).

## Relation to neighbors

- Runs immediately after Rung N (`FUN_005ff390`, laser/draw-segment timer purge).
- Runs immediately before Rung P (`LightningStorm__Process @ 0x0053A6C0`).
- Both neighbors and this rung are part of the unconditional non-object-loop block of
  `LogicClass::PerTickUpdate` that precedes the main live-object vector tick (Rung T).
