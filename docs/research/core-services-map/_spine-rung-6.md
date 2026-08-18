# Spine Rung 6 — F. Map lighting fade, second channel (Special 0x1000)

Verification of LogicClass::PerTickUpdate rung #6. Driver `FUN_004acbc0` invoked from the
spine body `LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`, at the rung-6 gate/call site
`0055b2c4`–`0055b337`.

Authority: binary -> Ghidra -> docs. All addresses below cite the verifying Ghidra MCP call.

## Verdict (one line)

Rung 6 is the **fog-of-war "previously seen" re-shroud / re-darken driver** (second
lighting/shroud channel), gated on the Special `0x1000` FogOfWar bit. It is **NOT a color
fade** — it walks every map cell, re-applies "seen-but-not-visible" shroud edge state, and
creates fogged building snapshots. It draws **no RNG**. It is **inactive in a standard YR
skirmish** (FogOfWar defaults OFF) — TS-legacy / opt-in path.

## Position in the ladder

- Runs immediately after rung 5 (`RecalcBridgeShroudFlags`, frame % 120) at `LAB_0055b29a`.
- Runs immediately before rung 7 (IonStorm/weather color tween) at `LAB_0055b33d`.
- Structural twin of rung 4 (`FUN_004acac0`, ambient/first channel) — same CellIterator
  shroud/lighting-recalc shape, but rung 4 is the always-on ambient channel and rung 6 is
  the second channel additionally gated on `0x1000`.

## Exact gate (CONFIRMED, with one correction)

From the disassembly at `0055b2c4`–`0055b2dd`
(verified via `disassemble_function 0x0055afb0`):

```
0055b2c4: MOV EAX, dword ptr [EBP]          ; EBP = g_ScenarioClass_Instance (0x00a8b230)
0055b2c7: TEST AH, 0x10                      ; AH bit 4 == bit 12 of EAX == 0x1000
0055b2ca: JZ  0x0055b33d                     ; skip rung 6 if Special 0x1000 clear
0055b2cc: FLD double ptr [EBX + 0x1648]      ; EBX = g_RulesClass_Instance (0x008871e0)
0055b2d2: FCOMP double ptr [0x007e2800]      ; compare against 0.0
0055b2d8: FNSTSW AX
0055b2da: TEST AH, 0x40
0055b2dd: JNZ 0x0055b33d                     ; skip if Rules+0x1648 == 0.0
```

- `(*g_ScenarioClass_Instance & 0x1000) != 0` — the Special/FogOfWar bit. CONFIRMED.
  (`*Scen` = Scen+0x0, the Special flags dword. Bit `0x1000` is the FogOfWar/Special bit.)
- `*(double*)(g_RulesClass_Instance + 0x1648) != 0.0` — CONFIRMED. The compared constant
  at `0x007e2800` is genuinely `0.0` (read_memory `0x007e2800` len 8 = `0000000000000000`,
  verified via `read_memory 0x007e2800`). NOTE the plan said the second comparand was a
  generic "Const_0_0"; it is the literal double at `0x007e2800` == 0.0 — same meaning.

### Timer offsets (CONFIRMED, with byte-offset detail)

Plan listed Scen+0x489 / Scen+0x48b as the dword-index timer slots. The disassembly uses
byte offsets `[EBP + 0x1224]` and `[EBP + 0x122c]`:

- `0x1224 / 4 = 0x489` — last-update frame counter (set to `g_CurrentFrameCounter`).
- `0x1228 / 4 = 0x48a` — stores a local (`[ESP+0x18]`, the int part / scratch).
- `0x122c / 4 = 0x48b` — recharge interval (= `ftol(Rules+0x1648 * const[0x007e27f8])`).

Verified via `disassemble_function 0x0055afb0` (`0055b2df`/`0055b2e5`/`0055b2eb`,
`0055b315`/`0055b319`/`0055b323`). Plan offsets CONFIRMED.

When the timer has elapsed: `Math__ftol` @ `0x007c5f00` (`0055b310`, confirmed via
`get_function_by_address 0x007c5f00` -> "Math__ftol"), then `FUN_004acbc0` @ `0055b326`.
The interval uses `Rules+0x1648 * [0x007e27f8]` (a frames-per-second scale constant).

## What the driver does (CONFIRMED)

`FUN_004acbc0` (verified via `decompile_function 0x004acbc0` +
`disassemble_function 0x004acbc0`). `ECX = 0x87f7e8` (g_MapClass `this`):

1. **Cell pass A** — `CellIterator_Init` (`0x00578350`) then loop `CellIterator_Next`
   (`0x00578290`). For each cell, read flags at `+0x140`. If bit `0x1` (mapped/visible)
   is clear AND bit `0x2` (seen/shrouded-known) is set, OR in bit `0x40` (mark for
   re-shroud).
2. `FUN_004adff0(0,1)` (`0x004adff0`) — re-cloak/re-fog pass over the display object layer
   `DisplayLayerEntry_008a0390`; calls each object's vtable `+0x120` cloak hook for buildings
   that are fogged-and-allied or human-owned with the fog flag. No RNG.
3. **Cell pass B** — iterate again; for each cell with bit `0x40` set, clear it and call
   `FUN_004acc50(&cell_coord)` (`0x004acc50`).

`FUN_004acc50` (verified via `decompile_function 0x004acc50`,
callers verified via `get_function_callers 0x004acc50` = only `FUN_004acbc0` and itself):
- Resets the cell's shroud byte `+0x121` to `0xfe`, clears flag bits `&0xfffffffc`.
- If cell flag bit `0x8` set, `FUN_006da7d0` (radar/tactical dirty-mark, no RNG).
- 8-neighbor loop using `g_DirectionOffsets`: recomputes each neighbor's shroud edge
  bitmask via `Shroud_EdgeBitmask_Calculator`; recurses into fully-shrouded neighbors;
  re-darkens partially-shrouded ones and dirty-marks them.
- If original flags had bit `0x3`, calls `FUN_00486a70`.

`FUN_00486a70` (verified via `decompile_function 0x00486a70`): **itself gated again on
`(*g_ScenarioClass_Instance & 0x1000) != 0`** at entry. Spawns shroud-snapshot objects on
adjacent cells; the building-snapshot leaf `FUN_00457aa0` (verified via
`decompile_function 0x00457aa0`) calls `BuildingClass__CreateFoggedSnapshot` — the
fog-of-war "remembered building" ghost. No RNG anywhere in this subtree.

This is the classic fog-of-war "previously seen but not currently visible" darkening +
remembered-building-snapshot machinery — exactly the behavior CLAUDE.md flags as TS-legacy
and OFF by default in YR.

## RNG draws

**NONE.** `draws_rng = false`, `rng_stream = none`.

The whole subtree is deterministic: `CellIterator_Init`/`_Next` (`0x00578350`/`0x00578290`)
are plain row/column grid stepping (verified via `decompile_function` of both — pure pointer
arithmetic, no random); `Shroud_EdgeBitmask_Calculator` is pure cell-state math;
`FUN_006da7d0` is dirty-flagging; object creation uses `operator_new`, not RNG. No
`Scen->Random`, `g_MainRng`, or `g_MapGenRng` receiver appears on any call in the chain.
Lockstep RNG-draw order is unaffected by this rung whether or not the gate is taken.

## Active-in-YR

**NO — conditional, gated OFF by default.** `gate = (*Scen & 0x1000) != 0 AND Rules+0x1648 != 0.0`.

- `Scen` bit `0x1000` is the FogOfWar/Special bit. Per CLAUDE.md (TS-legacy section),
  `[MultiplayerDialogSettings] FogOfWar` defaults to `false` in YR, and the
  "previously seen but not visible" darkening is exactly the behavior gated behind
  `SpecialFlags & 0x1000`. The inner helper `FUN_00486a70` re-checks the same bit,
  reinforcing that this entire channel is the FogOfWar path.
- In a standard YR skirmish the bit is clear -> rung 6 is skipped every tick (`JZ 0055b33d`).
- Verdict: **Tiberian Sun legacy / opt-in**. `ts_legacy = true`, `active_in_yr = conditional`
  (only when FogOfWar is explicitly enabled). When inactive it is a single `TEST AH,0x10 / JZ`
  with no side effects and no RNG, so it does not perturb the lockstep contract.

## Ghidra calls cited

- `decompile_function 0x004acbc0`, `disassemble_function 0x004acbc0` (driver body)
- `decompile_function 0x0055afb0`, `disassemble_function 0x0055afb0` (spine body, rung-6 site `0055b2c4`–`0055b337`)
- `get_function_by_address 0x007c5f00` (Math__ftol)
- `read_memory 0x007e2800` (== 0.0 comparand)
- `decompile_function 0x004adff0` (display-layer re-cloak pass, no RNG)
- `decompile_function 0x004acc50`, `get_function_callers 0x004acc50` (recursive re-shroud, callers = driver + self)
- `decompile_function 0x00486a70` (snapshot spawner, re-gated on 0x1000)
- `decompile_function 0x00457aa0` (BuildingClass__CreateFoggedSnapshot leaf)
- `decompile_function 0x00578350`, `decompile_function 0x00578290` (CellIterator, deterministic)
- `decompile_function 0x004acac0` (rung-4 twin, for structural comparison)
- `get_function_callers 0x004acbc0` (driver called only from the spine)
