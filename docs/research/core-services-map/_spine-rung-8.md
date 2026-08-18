# Spine Rung #8 — H. Tiberium GROWTH driver (all types)

**Status:** VERIFIED from binary this session. Image base 0x400000.
**Rung:** #8 of `LogicClass::PerTickUpdate` (`LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`).
**Driver:** `TiberiumClass__GrowthDriver_AllTypes` @ `0x00722C40`.
**Authority:** binary -> Ghidra. Disassembly is ground truth; decompiler structure for the
driver is confusing (the `do/while` timer test) so the gate/fields were re-read from asm.

Verification calls used (all this session):
- `decompile_function 0x00722C40` + `disassemble_function 0x00722C40` (the driver)
- `decompile_function 0x0055AFB0` (spine body; confirms rung ordering at `LAB_0055b4d7`)
- `get_xrefs_to 0x00722C40` / `get_function_callers 0x00722C40` (single caller = the spine)
- `decompile_function 0x00722f00` + `disassemble_function 0x00722f00` (`GrowthProcessor`, the RNG-bearing child)
- `decompile_function 0x00722af0` + `disassemble_function 0x00722af0` (`AddToSpreadQueue`, 3rd RNG site)
- `decompile_function 0x007233a0` (`RebuildGrowthQueue`, no RNG)
- `get_function_by_address 0x0065c780` (`Random__Next` = the RandomClass draw)
- `get_function_by_address 0x007c5f00` (`Math__ftol`, float->long, NOT RNG)

---

## Order placement (the lockstep contract)

In the spine body `0x0055AFB0`, after the weather/IonStorm block exits at `LAB_0055b4d7`,
the very first three tail calls are, in order:

```
LAB_0055b4d7:
  TiberiumClass__GrowthDriver_AllTypes();   <- RUNG 8 (this rung)  call @ 0055b4d7
  TiberiumClass__SpreadDriver_AllTypes();   <- RUNG 9 (I)
  BombClass__UpdateAll();                   <- RUNG 10 (J)
```

Confirmed via `decompile_function 0x0055AFB0` (the three calls appear consecutively) and
`get_xrefs_to 0x00722C40` -> `From 0055b4d7 in LogicClassPerTickUpdateLiveVector
[UNCONDITIONAL_CALL]`. GROWTH precedes SPREAD; this ordering matters for RNG-draw order
because **both** rungs 8 and 9 consume the same Scen->Random stream (see below).

`get_function_callers 0x00722C40` returns exactly ONE caller (`@ 0055afb0`) — the driver is
a per-tick rung only, never reached from map-generation or any other path. This is why its
RNG binds to the in-game Scen->Random stream, not g_MapGenRng.

---

## Purpose (one line)

Per-tick **ore (Tiberium) growth scheduler**: for each TiberiumClass type, when that type's
per-type growth timer expires, run one growth pass (grow a random handful of eligible ore
cells from the type's growth priority-queue) and re-arm the timer for the next interval.

---

## What it walks / does

`disassemble_function 0x00722C40`:
- Iterates `g_TiberiumClass_Array` (base `[0x00b0f4ec]`) for `g_TiberiumClass_Array_Count`
  (`[0x00b0f4f8]`) entries — one TiberiumClass per ore type.
- Per entry, reads two timer fields on the TiberiumClass object:
  - `+0x11c` = last-update frame stamp (`-1` sentinel = "fire now / unset")
  - `+0x124` = remaining/interval frames
  - `+0x120` = captured FP value written alongside (intermediate of the ftol re-arm).
- **Timer test** (`0x00722c82`-`0x00722c97`): if `+0x11c == -1` and `+0x124 == 0`, fire;
  else if `(g_CurrentFrameCounter - last) >= interval`, fire. Otherwise skip this type.
- **On fire** (`0x00722c99`): `CALL 0x00722f00` = `TiberiumClass__GrowthProcessor` (this is
  the real growth work + RNG), then **re-arm**: pick FP constant `[0x007e5138]` if
  `*(byte*)(Scen) & 0x40` else `[0x007e1718]`, multiply by `FILD [obj+0xa8]` (per-type rate
  field), `CALL 0x007c5f00` (Math__ftol) -> store frame stamp (`+0x11c = g_CurrentFrameCounter`),
  FP (`+0x120`), and new interval (`+0x124`).
- `g_CurrentFrameCounter` is `[0x00a8ed84]`.

Note the `Scen & 0x40` branch selecting the re-arm constant: bit 0x40 of the ScenarioClass
flags word picks a faster vs slower growth-interval constant. (Not the 0x1000 FogOfWar bit;
0x40 is a separate Special-flags bit — selects growth cadence, both branches active.)

---

## Exact gate (confirmed)

`0x00722c40`-`0x00722c51`:
```
MOV EAX,[0x00a8b230]          ; EAX = g_ScenarioClass_Instance
MOV CL, byte ptr [EAX+0x34a6] ; CL = Scen->OreGrowthEnabled byte
TEST CL,CL / JZ ...           ; if 0, skip entire driver
```
**Gate = `*(char*)(g_ScenarioClass_Instance + 0x34a6) != 0`** — ore-growth-enabled flag.
This matches the plan's stated gate exactly. The same `Scen+0x34a6` byte gates Rung 9
(SPREAD) — both are off together when ore growth is disabled in the match.

Second implicit gate: the body loop only runs if `g_TiberiumClass_Array_Count > 0`
(`0x00722c5e TEST EAX,EAX / JLE`). Always > 0 in a loaded YR map (ore + gems types exist).

---

## RNG: YES — draws from Scen->Random (via the GrowthProcessor child)

The driver body `0x00722C40` itself draws **NO RNG**. Its only float helper is `Math__ftol`
(`0x007c5f00`, plain float->long truncation, verified `get_function_by_address`), used for
the interval re-arm. **All RNG for this rung is drawn inside the children it calls.**

`Random__Next` resolves to **`0x0065c780`** (`get_function_by_address 0x0065c780`
-> `Random__Next`). Stream binding is per-callsite ECX. Every draw site in this rung's
subtree loads the **same receiver**:
```
MOV EAX/ECX,[0x00a8b230]   ; g_ScenarioClass_Instance
LEA/ADD ECX, EAX+0x218     ; ECX = Scen+0x218  == Scen->Random (embedded RandomClass)
CALL 0x0065c780            ; Random__Next
```
**Stream = `Scen->Random`** for all draws. (Consistent with
`reference_rng_instance_routing_truth.md`: Scen->Random lives at Scen+0x218.)

### Draw sites and count (per growth pass that fires)

1. **`TiberiumClass__GrowthProcessor` @ 0x00722f00, first draw** — `0x00722f6f LEA ECX,[EAX+0x218]`
   / `0x00722f75 CALL 0x0065c780`. Result `% iVar4 + 1` where `iVar4` is the rate-derived
   count clamped to `[5, 0x32]`. **Purpose:** how many cells to grow this pass (1..count).
   Drawn **once per fired type per pass.**

2. **`TiberiumClass__GrowthProcessor` @ 0x00722f00, second draw** — inside the per-cell loop,
   `0x00723044 LEA ECX,[EAX+0x218]` / `0x0072304a CALL 0x0065c780`. Result `% 0x32` added to
   `g_CurrentFrameCounter` -> next-growth frame jitter for the cell just pushed back onto the
   growth queue. Drawn **once per grown cell** that is still below the per-cell max
   (`[cell+0x11e] < 0xb`).

3. **`TiberiumClass__AddToSpreadQueue` @ 0x00722af0** (called by GrowthProcessor per grown
   cell) — `0x00722b61 ADD ECX,0x218` / `0x00722b67 CALL 0x0065c780`. Result `% 0x32` +
   `g_CurrentFrameCounter` -> spread-queue scheduling jitter. Drawn **once per cell that
   passes the can-spread / not-already-queued test.**

`TiberiumClass__RebuildGrowthQueue` (`0x007233a0`, called from GrowthProcessor only when the
growth queue under-runs) draws **NO RNG** — it sets the per-cell jitter field to 0 and does a
full-map `CellIterator` rescan (`decompile_function 0x007233a0`).

**Total draws when a type fires its growth pass:** `1 + (grown-cells <0xb)*1 + (spreadable
cells)*1`, all to Scen->Random. When NO type's timer is due this tick, **0 draws**. The
draw-order within a pass is deterministic (clamp-count draw first, then per-cell loop in
priority-queue pop order), which is what makes this rung lockstep-safe.

---

## Active in YR / Tiberian Sun legacy

**Active in YR: YES** (conditional on ore-growth being enabled, which is the standard
skirmish default). Ore growth is a core, player-visible YR mechanic: ore fields slowly
regrow/spread over the match, directly affecting economy. The `Scen+0x34a6` flag derives
from the match's ore-growth setting; in a normal skirmish with ore growth on, this rung
fires regularly and its effect (ore cells thickening / new ore appearing) is visible.

**TS legacy: NO** (the active path). The function name says "Tiberium" because the class is
inherited from the TS codebase, but the growth mechanic itself is live in RA2/YR (it is the
ore-growth system, not the TS chemical-tiberium variants). The `Scen & 0x40` constant-select
branch and both re-arm constants are reachable. Not gated behind the FogOfWar/0x1000
Special bit. Skip flags: none of the known TS-only dormant flags apply here.

---

## Field/global reference (verified this session)

| Symbol | Address/offset | Meaning | Verified by |
|---|---|---|---|
| `g_ScenarioClass_Instance` | `[0x00a8b230]` | ScenarioClass ptr | asm `0x00722c40` |
| ore-growth-enabled gate | `Scen + 0x34a6` (byte) | driver gate | asm `0x00722c4e` |
| Scen->Random | `Scen + 0x218` | RNG receiver (ECX) | asm `0x00722f6f/0x00723044/0x00722b61` |
| `g_TiberiumClass_Array` | `[0x00b0f4ec]` | per-type array base | asm `0x00722c6d` |
| `g_TiberiumClass_Array_Count` | `[0x00b0f4f8]` | type count | asm `0x00722c57/0x00722ce6` |
| `g_CurrentFrameCounter` | `[0x00a8ed84]` | gameplay frame | asm `0x00722c87` |
| obj last-update frame | `TiberiumClass + 0x11c` | timer stamp (-1=fire) | asm `0x00722c76` |
| obj re-arm FP | `TiberiumClass + 0x120` | ftol intermediate | asm `0x00722ce0` |
| obj interval frames | `TiberiumClass + 0x124` | remaining/interval | asm `0x00722c7c` |
| obj rate field | `TiberiumClass + 0xa8` | per-type growth rate | asm `0x00722cc6` |
| `Random__Next` | `0x0065c780` | RandomClass draw | `get_function_by_address` |
| `Math__ftol` | `0x007c5f00` | float->long (NOT RNG) | `get_function_by_address` |
| `TiberiumClass__GrowthProcessor` | `0x00722f00` | per-pass growth work + RNG | xref from driver |
| `TiberiumClass__AddToSpreadQueue` | `0x00722af0` | per-cell spread schedule + RNG | call @ `0x00723113` |
| `TiberiumClass__RebuildGrowthQueue` | `0x007233a0` | full rescan, no RNG | call @ `0x00722fa1` |

---

## One-line summary for the ladder

**H. Tiberium GROWTH driver @ 0x00722C40** — walks per-type `g_TiberiumClass_Array`, fires
a growth pass when each type's `+0x124` timer is due; gate `*(char*)(Scen+0x34a6)!=0`
(ore-growth-enabled). Draws **Scen->Random** (`0x0065c780`, ECX=Scen+0x218): 1 draw for
cells-to-grow per fired type, then 1 per grown cell (frame jitter) + 1 per spreadable cell
(via `AddToSpreadQueue`); 0 draws when no type's timer is due. Active in YR (standard ore
growth), not dead TS legacy.
