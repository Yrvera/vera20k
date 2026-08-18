# Spine Rung #9 — I. Tiberium SPREAD driver (all types)

**Driver:** `TiberiumClass__SpreadDriver_AllTypes` @ `0x007221B0`
**Inner worker:** `TiberiumClass__SpreadProcessor` @ `0x00722440`
**Body call site:** `LogicClass::PerTickUpdate` live-vector body `0x0055AFB0`, at `LAB_0055b4d7` —
`GrowthDriver_AllTypes()` (rung H) then `SpreadDriver_AllTypes()` (rung I, this) then `BombClass__UpdateAll()` (rung J).

Status: **VERIFIED** (binary). Active in YR. Not TS legacy.

---

## Purpose (one line)

Per tick, walks every `TiberiumClass` (ore/gem type) and, for each whose per-type spread timer has
elapsed, runs the spread processor that picks a 1..N step budget from **Scen->Random** and pushes ore
outward into adjacent placeable cells (priority-queue flood). This is the visible "ore field grows
into neighboring cells" behavior.

## What it walks / does

`decompile_function 0x007221B0` + `disassemble_function 0x007221B0`:

- Array base `g_TiberiumClass_Array @ [0x00b0f4ec]`, count `g_TiberiumClass_Array_Count @ [0x00b0f4f8]`.
  (Disasm: `MOV ECX,[0x00b0f4ec]` @ `0x007221d4`; `MOV EAX,[0x00b0f4f8]` @ `0x007221c3`.)
- Frame counter `g_CurrentFrameCounter @ [0x00a8ed84]` (`MOV EDX,[0x00a8ed84]` @ `0x007221ee`).
- Per-type timer fields on each `TiberiumClass`: `+0x100` = last-run start frame, `+0x108` = interval.
  Elapsed check (matches the rung-anchor description): if `+0x100 == -1` then due iff `+0x108 == 0`;
  else `remaining = +0x108 - (frame - +0x100)`; due iff `remaining <= 0`.
- On due: `CALL TiberiumClass__SpreadProcessor 0x00722440` (`CALL 0x00722440` @ `0x00722200`), then
  re-arm: `+0x100 = frame`, `+0x104 = <stack value loaded at entry, EBX = [ESP+0xc]>`, `+0x108 = type[+0x9c]`
  (the per-type reschedule interval). The `+0x104` write is the EBX value loaded at function entry
  (`MOV EBX,[ESP+0xc]` @ `0x007221cf`) — a caller-stack value, not a live computation (decomp's
  uninitialized `local_8`); not RNG-relevant.

## Exact gate (confirmed)

`*(char*)(g_ScenarioClass_Instance + 0x34a6) != 0` — ore/tiberium-growth-enabled flag.
Disasm: `MOV EAX,[0x00a8b230]` (g_ScenarioClass_Instance) `; MOV CL,[EAX+0x34a6] ; TEST CL,CL ; JZ exit`
(`0x007221b0`–`0x007221c1`). Identical gate to rung H (Growth driver). The body call site
(`0x0055AFB0` @ `LAB_0055b4d7`) is **unconditional** — gating lives inside the driver, so the driver
is always entered each tick and self-skips when the flag is 0 (empty-fast-path when count is 0 too).

This matches the rung-anchor ladder exactly; no correction needed.

## RNG — DRAWS, stream = Scen->Random

The driver itself draws no RNG. The **inner SpreadProcessor `0x00722440` draws exactly once per
processed Tiberium type** (i.e. once per due type per tick).

Draw site (`disassemble_function 0x00722440`):
```
007224ae: MOV EAX,[0x00a8b230]    ; EAX = g_ScenarioClass_Instance
007224b3: LEA ECX,[EAX + 0x218]   ; ECX (this) = Scen + 0x218  == Scen->Random
007224b9: CALL 0x0065c780         ; Random__Next(this = Scen+0x218)
007224be: CDQ / XOR EAX,EDX / SUB EAX,EDX   ; abs(rnd)
007224ca: IDIV ESI                ; abs(rnd) % iVar5
007224ce: MOV EDI,EDX / INC EDI   ; step budget = (abs(rnd) % iVar5) + 1
```

- `Random__Next @ 0x0065c780` is `__fastcall`, `this` in ECX (`decompile_function 0x0065c780`:
  R(250,103) XOR lagged-Fibonacci advance over `this+0xC` state, cursors at `this+4`/`this+8`).
- Receiver is **`Scen+0x218`** = `Scen->Random`, the persisted/deterministic lockstep gameplay stream
  — confirmed against `random-scenario.md` line 14/23 (`g_ScenarioClass_Instance @ 0x00A8B230`,
  `Scen->Random @ Scen+0x218`). NOT `g_MainRng (0x00886B88)`, NOT `g_MapGenRng (0x00ABE890)`.
- This is consistent with the sibling growth path: `random-scenario.md` line 99 already records
  `TiberiumClass__GrowthProcessor 0x00722f00` ore growth as Scen->Random; spread uses the same stream.

**Draw count & purpose:** 1 draw per due type → `step_budget = (abs(value) % N) + 1`, where
`N = clamp(Math__ftol(spread_count_density * type[+0xa0]), 5, 0x19)` (`Math__ftol 0x007c5f00`
@ `0x0072248a`; clamp `[5..0x19]` @ `0x0072248f`–`0x007224ac`). `N` derives from the type's spread
density float at `+0xa0` (the same field the early-out at `0x0072246f` compares against `_DAT_007e3810`
to bail when density is non-positive). The step budget caps how many cells this type spreads this tick;
the spread itself (`CellClass__SpreadTiberium 0x00483780`, neighbor scan via
`MapCoord_StepByDir_GetCell 0x00481810` + `CellClass__CanPlaceTiberium 0x004838e0`) consumes **no
further RNG** — it is a deterministic priority-queue (binary-heap) flood, so the single Random__Next is
the only draw on this rung per due type.

> Lockstep note: draw count is data-dependent — **one Scen->Random draw per Tiberium type that is due
> this tick** (a type with elapsed timer + positive density). Early-outs that skip the draw entirely:
> empty spread queue (`+0xf4` null / `*+0xf4 == 0`) or non-positive density (`+0xa0 <= _DAT_007e3810`),
> both checked **before** the `Random__Next` call (`0x00722448`–`0x0072247a`). The growth driver (rung H)
> runs immediately before and also draws Scen->Random, so the spread draws follow growth draws in the
> per-tick stream order.

## Active in YR? Yes. Tiberian Sun legacy? No.

- Ore/gems are the YR economy; field expansion into adjacent cells is standard, player-visible skirmish
  behavior. Gate `Scen+0x34a6` is the ore-grows session flag (on by default in skirmish), not a
  Special/FogOfWar opt-in bit. Reachable and visible every normal YR match with ore on the map.
- Body call site is unconditional in the live-vector tick; no Special/SpecialFlags gating, no mode gate.
- No dead/TS-only branch on this path: the processor's only legacy-flavored field is the `+0x104`
  re-arm write (caller-stack carry), which is bookkeeping, not behavior.

## Evidence (Ghidra calls)

- `decompile_function 0x007221B0`, `disassemble_function 0x007221B0` — driver, gate `Scen+0x34a6`,
  array `[0x00b0f4ec]`/count `[0x00b0f4f8]`, timer `+0x100`/`+0x108`, call to `0x00722440`.
- `decompile_function 0x0055AFB0` — body order: `GrowthDriver_AllTypes()` → `SpreadDriver_AllTypes()`
  → `BombClass__UpdateAll()` at `LAB_0055b4d7` (call unconditional).
- `get_function_callees 0x007221B0` — sole callee `TiberiumClass__SpreadProcessor @ 0x00722440`.
- `decompile_function 0x00722440`, `disassemble_function 0x00722440` — single `Random__Next` draw,
  receiver `LEA ECX,[EAX+0x218]` after `MOV EAX,[0x00a8b230]`; step-budget `% N + 1`; `N` clamp `[5..0x19]`.
- `get_function_callees 0x00722440` — `Random__Next @ 0x0065c780`, `Math__ftol @ 0x007c5f00`,
  `CellClass__SpreadTiberium 0x00483780`, `CellClass__CanPlaceTiberium 0x004838e0`,
  `MapCoord_StepByDir_GetCell 0x00481810`, `TiberiumClass__RebuildSpreadQueue 0x007228b0`.
- `decompile_function 0x0065c780` — `Random__Next` is `__fastcall(this in ECX)`, lagged-Fibonacci.
- Cross-ref `random-scenario.md` lines 14/23/99 — `g_ScenarioClass_Instance @ 0x00A8B230`,
  `Scen->Random @ Scen+0x218`, ore growth = Scen->Random (sibling confirmation).
