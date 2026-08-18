# Spine Rung #4 — D. Shroud-regrowth (shroud creep) pass — NOT "lighting fade"

**Rung:** order 4 ("D") in `LogicClass::PerTickUpdate` (`LogicClassPerTickUpdateLiveVector`
@ `0x0055AFB0`).
**Driver:** `FUN_004acac0` @ `0x004acac0`, called via the `Math__ftol` (`0x007c5f00`) timer
recompute.
**Body site:** `0055b205`–`0055b28c` inside `0x0055AFB0`.
**Status:** VERIFIED from binary this session.

> **LABEL CORRECTION.** The ladder seed calls this rung "Map lighting fade — ambient
> channel." That is WRONG. The driver `FUN_004acac0` is the **shroud-regrowth / shroud-creep
> pass** (ShroudGrow). It walks every map cell, re-asserts shroud over cells flagged for
> regrowth, and queues their redraw. It touches nothing in the ambient-color/lighting path.
> The "lighting fade" mislabel almost certainly arose from this rung sitting adjacent to the
> two genuine lighting-channel rungs (F `0x004acbc0`, G `0x004ae4c0`) and sharing the same
> 3-field Scenario timer shape. Verified shroud purpose via callee chain (Cell flag bit 0x8
> "shrouded?" / bit 0x10, `FUN_004acda0` 8-neighbour shroud propagation, `FUN_004adff0`
> per-player reveal-notify) + the INI key bindings below.

Verification calls used:
- `decompile_function 0x0055AFB0` + `disassemble_function 0x0055AFB0` (body region — the
  disassembly `0055b205`–`0055b28c` is ground-truth for the gate + receiver).
- `decompile_function 0x004acac0` + `disassemble_function 0x004acac0` (driver).
- `decompile_function 0x004acda0` (8-neighbour shroud-flag clear/propagate),
  `0x004adff0` (per-player reveal-notify walk over display layer `0x008a0390`).
- `get_function_callees 0x004acac0` and `0x004acda0` (RNG-absence proof).
- `decompile_function 0x007c5f00` (`Math__ftol` = FPU round-to-long, NOT RNG).
- `read_memory 0x007e2800` (= 0.0, the gate compare constant), `0x007e27f8`
  (= `0x408c200000000000` = **900.0**, the interval multiplier).
- `read_memory 0x0087f7e8` (driver receiver slot = global MapClass pointer).
- INI-key bindings: `get_assembly_context 0x0066a693` (ShroudGrow → `Rules+0x17f0`),
  `0x0066b4be` (ShroudRate → `Rules+0x1640`); `read_memory 0x83a59c` = "ShroudGrow",
  `0x83a2f4` = "ShroudRate".
- Cross-check: `docs/research/PERTICKUPDATE_UNNAMED_CALLEE_RESOLUTION_GHIDRA_REPORT.md`
  (Order 5 — `FUN_004ACAC0`: Shroud Regrowth Pass) — independently reached the same identity.

---

## Purpose (one line)

When `ShroudGrow=yes`, periodically re-grow shroud over the whole map (the "shadow creep"
that re-conceals previously-revealed cells); driver is gated OFF in stock YR.

## What it walks / does

`__fastcall`, single arg = `this` = the global `MapClass` instance (receiver `ECX = 0x87f7e8`,
confirmed at body site `0055b273 MOV ECX,0x87f7e8` immediately before `0055b27b CALL 0x004acac0`;
`0x0087f7e8` is the MapClass pointer global, same one used by the adjacent bridge/lighting
rungs). The driver runs **two full 0x200 × 0x200 cell sweeps** (the engine cell grid),
bounds-checked per cell via `Cell_in_bounds_check` (`0x00568300`), cell fetched via
`MapClass__Get_CellClass` (`0x005657a0`):

- **Pass 1** (`004acad0`–`004acb28`): for every in-bounds cell whose `cell+0x12C` has bit
  `0x8` set AND bit `0x10` clear (`TEST CL,0x10 / TEST CL,0x8`), OR in flag bit `0x20` at
  `cell+0x140` (marks the cell as "scheduled to re-shroud this pass"). Bit 0x8 = "this cell
  is currently revealed/visible-eligible for regrowth"; bit 0x10 = "permanently revealed /
  exempt."
- **Pass 2** (`004acb2c`–`004acb94`): for every cell with `cell+0x140 & 0x20` set, clear that
  bit (`& 0xFFFFFFDF`) and call `FUN_004acda0` (`0x004acda0`) on the cell. That helper, when
  the cell still has `cell+0x12C & 0x8`, clears the cell's reveal bits (`cell+0x12C & 0xE7`),
  calls `FUN_006da7d0` (cell redraw-dirty / Tactical queue append), then walks the 8 compass
  neighbours via `g_DirectionOffsets` and clears each neighbour's `cell+0x12C & 0xEF`,
  re-queuing each for redraw — i.e. it re-conceals the cell and degrades its neighbours'
  edge reveal. This is shroud creeping back in.
- **Finalize:** `FUN_004adff0(0,0)` (`0x004adff0`) walks the display-object layer
  (`0x008a0390`) and, per visible techno, fires a vtable reveal/look method (`*+0x120`)
  gated by `HouseClass__IsHumanPlayer` (local player re-reveals around its own units) and,
  for AI/observer with `Rules+0x17e7` set, allied re-reveal — so units immediately re-light
  their own sight radius after the global re-shroud. Then `(*this+0x38)(1)` — a MapClass
  vtable call (full-map shroud/redraw flush).

## Gate / mode condition

**CONFIRMED, and the precise field identities resolved.** At `0055b205`:

```
0055b205 MOV EBX,[0x008871e0]              ; EBX = g_RulesClass_Instance
0055b20b MOV AL,byte ptr [EBX + 0x17f0]    ; Rules+0x17f0 = ShroudGrow (bool)
0055b211 TEST AL,AL
0055b213 JZ 0x0055b28e                     ; ShroudGrow == 0  -> SKIP driver entirely
0055b215 FLD  double ptr [EBX + 0x1640]    ; Rules+0x1640 = ShroudRate (double, minutes)
0055b21b FCOMP double ptr [0x007e2800]     ; compare against 0.0
0055b221 FNSTSW AX
0055b223 TEST AH,0x40
0055b226 JNZ 0x0055b28e                    ; ShroudRate == 0.0 -> SKIP
... (Scenario 3-field timer at Scen+0x1218/+0x1220 checks remaining==0; if not expired, skip)
0055b259 FLD  double ptr [EBX + 0x1640]    ; ShroudRate
0055b25f FMUL double ptr [0x007e27f8]      ; × 900.0  -> interval in frames
0055b265 CALL 0x007c5f00                   ; Math__ftol: round to long  (NOT RNG)
0055b26e MOV [ESI],EDI                     ; Scen+0x1218 = current frame (timer start)
0055b270 MOV [ESI+0x4],ECX                 ; Scen+0x121C scratch
0055b278 MOV [ESI+0x8],EAX                 ; Scen+0x1220 = recomputed interval
0055b273 MOV ECX,0x87f7e8                  ; this = global MapClass
0055b27b CALL 0x004acac0                   ; <-- the driver
```

Gate = `*(char*)(Rules+0x17f0) != 0` (**ShroudGrow**) AND `*(double*)(Rules+0x1640) != 0.0`
(**ShroudRate**) AND the Scenario timer `Scen+0x486/+0x488` (byte `0x1218`/`0x1220`) has
expired. This matches the ladder seed's gate exactly. Field identities (verified this
session from the binary):

| Field | Offset | INI key | String addr | Read/store call |
|---|---|---|---|---|
| `Rules+0x17f0` (bool) | byte `0x17f0` | **ShroudGrow** | `0x83a59c` ("ShroudGrow") | `MOV [ESI+0x17f0],AL` @ `0x0066a693` |
| `Rules+0x1640` (double) | dbl `0x1640` | **ShroudRate** | `0x83a2f4` ("ShroudRate") | `FSTP [ESI+0x1640]` @ `0x0066b4be` |

The `900.0` multiplier converts ShroudRate (minutes per shroud-creep step, per stock INI
comment) into frames: 1 minute ≈ 900 frames at the 15 fps logic rate. So the timer fires
every `ShroudRate × 900` frames when active.

## RNG

**Draws NO RNG.** No RNG instance (`Scen->Random`, `g_MainRng`, `g_MapGenRng`) is loaded as
a receiver anywhere in the driver or any callee. `Math__ftol` (`0x007c5f00`) is an FPU
round-to-long conversion, not a random draw (verified by decompile). Full callee set:

| Callee | Addr | RNG? |
|---|---|---|
| `Cell_in_bounds_check` | `0x00568300` | none (coord bounds test) |
| `MapClass__Get_CellClass` | `0x005657a0` | none (cell array index) |
| `FUN_004acda0` (8-neighbour re-shroud) | `0x004acda0` | none (flag clear + `FUN_006da7d0` redraw) |
| `FUN_004adff0` (per-player reveal-notify) | `0x004adff0` | none (display-layer walk + vtable look) |
| `Math__ftol` | `0x007c5f00` | none (FPU round) |

`rng_stream = none`, `draws_rng = false`. Contributes nothing to the lockstep RNG-draw
order. Because the entire rung is gated off in stock YR (ShroudGrow=no), it is doubly absent
from the RNG stream there.

## Active in YR / TS-legacy

**Active in YR: NO (gated off in stock YR).** Stock `rulesmd.ini`: `ShroudGrow=no`
(line 677, comment "Does the shroud grow back over time?") → the very first gate byte
(`Rules+0x17f0`) is 0 → `JZ 0x0055b28e` skips the driver every tick. `ShroudRate=4`
(line 762, "minutes between each shroud creep process") is moot because the outer ShroudGrow
gate already fails. **Player never sees shroud regrow in a normal YR skirmish.**

**Tiberian Sun legacy / TS-adjacent: YES.** Shroud-regrowth ("shadow creep") is the TS-era
fog/shroud mechanic; it is wired into the YR binary but disabled by the stock YR default,
the same family as the FogOfWar darkening this project treats as TS-only. Per CLAUDE.md the
Rust engine implements shroud-only (black for unexplored, stays revealed once seen) — so
this rung needs **no Rust implementation** for stock parity. If `ShroudGrow=yes` support is
ever added, it must run at this anchor position (order 4, after the Scenario cell-action
timer reset at `0055b1d8` and before the `frame % 120` bridge-shroud rung E), and only it
(not the RNG order) shifts — it draws no RNG.

## Caller context

`FUN_004acac0` is reached only from this per-tick body site (`0x0055b27b`). Its receiver is
the global MapClass (`0x87f7e8`). No other live per-tick caller; the rung's identity is
entirely owned by `LogicClass::PerTickUpdate`.

## Position in lockstep ladder

Runs after Rung C (placement-flag clear at `0055b1d8`) and before Rung E
(`MapClass__RecalcBridgeShroudFlags`, `frame % 120`, at `0055b29a`). Draws no RNG, so its
presence/absence (and its stock-off state) shifts no downstream rung's RNG-draw index. For
stock YR it is a pure no-op (gate fails before any state mutation). Its position only matters
if ShroudGrow is ever enabled, in which case it mutates cell shroud bits before the bridge
shroud / lighting rungs read them.
