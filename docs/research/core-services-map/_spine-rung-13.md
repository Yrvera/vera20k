# LogicClass::PerTickUpdate — Rung 13 (M): DiskLaserClass update (reverse walk)

**Status:** VERIFIED from binary this session.
**Parent:** `LogicClass::PerTickUpdate` @ `0x0055AFB0` (label `LogicClassPerTickUpdateLiveVector`).
**Authority:** binary -> Ghidra. Body site is keyed to the **disassembly** at
`disassemble_function 0x0055AFB0`; the decompiler mislabels the loop bound as
`g_DiskLaserClass_Array_Count` but the actual globals are correct (see below).

---

## Order / position

- **Order:** 13 of 28. Runs immediately **after** Rung L (TeamClass cull-and-tick,
  `0055b582`–`0055b59f`) and immediately **before** Rung N (`FUN_005ff390` @ `0x005ff390`,
  `0055b5be`).

## Body site (exact)

`disassemble_function 0x0055AFB0`, instructions `0055b5a1`–`0055b5be`:

```
0055b5a1  MOV  EAX,[0x008a0218]          ; count   = DAT_008a0218 (DiskLaser array count)
0055b5a6  LEA  ESI,[EAX + -0x1]          ; idx     = count - 1   (reverse walk)
0055b5a9  TEST ESI,ESI
0055b5ab  JL   0x0055b5be                ; skip whole loop if count-1 < 0 (empty list)
0055b5ad  MOV  ECX,[0x008a020c]          ; base    = DAT_008a020c (DiskLaser array pointer)
0055b5b3  MOV  ECX,[ECX + ESI*0x4]       ; this    = base[idx]   (DiskLaserClass*)
0055b5b6  MOV  EDX,[ECX]                 ; vtable  = *this
0055b5b8  CALL [EDX + 0x5c]              ; this->vt+0x5C()  (__thiscall, ECX=this, no args)
0055b5bb  DEC  ESI
0055b5bc  JNS  0x0055b5ad                ; loop while idx >= 0
0055b5be  CALL 0x005ff390               ; --> Rung N
```

- **Loop bound global:** `DAT_008a0218` (count). **Array base global:** `DAT_008a020c`.
- Decomp's `g_DiskLaserClass_Array` / `g_DiskLaserClass_Array_Count` map to these two
  addresses — the *name* is correct here (array does hold DiskLaserClass objects); only the
  "mislabel" caveat in the spine refers to the count-vs-base symbol-overlap. Confirmed the
  array is DiskLaser-typed via xrefs (see below).

## Purpose (one line)

Per-tick advance of every active **Floating-Disc ring-laser visual + terminal area-damage**
sequence — walks the global DiskLaserClass array in reverse and calls each object's AI slot.

## What it walks / does

- Walks `DAT_008a020c[ DAT_008a0218 ]` **in reverse** (count-1 down to 0).
- Calls **vt+0x5C** per object = `DiskLaserClass::AI` @ `0x004a7340`.
  - Vtable binding **verified by live memory read**: DiskLaserClass primary vtable base is
    `0x007e5fb8` (set in the constructor at `004a7a3a: MOV [ESI],0x7e5fb8`); slot at
    `0x007e5fb8 + 0x5C = 0x007e6014`; `read_memory 0x007e6014` = bytes `40 73 4a 00` =
    little-endian `0x004a7340`. (Not relying on the Ghidra comment.)
- Array population/teardown confirms element type is DiskLaserClass:
  `get_xrefs_to 0x008a020c` / `0x008a0218` show writers/readers in
  `DiskLaserClass__Constructor` (`0x004a7a30`), `DiskLaserClass__ScalarDeletingDestructor`,
  and `Detach_From_All_Lists` (`0x00725977`). Constructor pushes `param_1` (the new disc) at
  `count*4`, `count++`.

### DiskLaserClass::AI @ 0x004a7340 (`decompile_function 0x004a7340`)

State machine on `this+0x30`:
- `< 0`  : mark for removal — calls `FUN_004a7fe0` (a DynamicVector push onto a cleanup
  queue; `decompile_function 0x004a7fe0` — **no RNG**), returns.
- `> 0`  : decrement countdown, return (no visual this frame).
- `== 0` : one ring step. Reads source/target positions, checks weapon `Range` (`weapon+0xb4`),
  bails to removal if source `InLimbo` (`techno+0x425`). Computes ring-segment offsets from a
  **static rotation table** `DAT_008a0180` (deterministic; not RNG). Then either:
  - **FIRE branch** (ring wrapped): spawns terminal laser via `LaserDrawClass__Constructor`
    (`0x0054fe60` — **no RNG**, `decompile_function 0x0054fe60`), calls **`Apply_area_damage`**
    (`0x00489280`) with `warhead = weapon+0xac`, plays `weapon->Report` via `VocClass__PlayAt`
    (audio cue, no sim RNG), sets `state = -1`.
  - **else**: spawns **two** `LaserDrawClass` rotating ring beams (no RNG), sets `state = 1`,
    increments ring counter `this+0x38`.

## Gate / mode condition

- **Confirmed: unconditional reverse loop.** No mode/flag gate. The only guard is the
  emptiness check `count-1 < 0` (`TEST ESI,ESI; JL 0x0055b5be`). If no Floating Disc is
  currently firing, the array is empty and the rung is a no-op.

## RNG

- **Body site (the reverse loop itself): draws NO RNG.** The loop only does an indirect
  virtual call.
- **`DiskLaserClass::AI` draws NO RNG of its own** — ring geometry comes from the static
  table `DAT_008a0180`; laser-segment spawn and audio do not draw.
- **Transitive RNG, FIRE tick only:** `DiskLaserClass::AI` calls `Apply_area_damage`
  (`0x00489280`), which **conditionally** draws RNG.
  - **Stream = Scen->Random** (the Scenario RNG), NOT g_MainRng/g_MapGenRng.
    Verified at the call sites in `disassemble_function 0x00489280`: every
    `Random__RandomRanged` call loads `[0x00a8b230]` (g_ScenarioClass_Instance) then
    `LEA ECX,[reg + 0x218]` (e.g. `00489fe6 MOV EDX,[0x00a8b230]; 00489fef LEA ECX,[EDX+0x218];
    00489ff5 CALL 0x0065c7e0`). `Scen+0x218` is the 250-word Scenario RNG state; helper is
    `Random__RandomRanged @ 0x0065c7e0`. (Matches `APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW`
    and `RANDOM_RANDOMRANGED_0065C7E0` reports.)
  - **Draw count / for what (all conditional):**
    1. Bridge structural damage blocks A/B/C/D: up to four `RandomRanged(1, Rules+0x1740 =
       BridgeStrength)` draws, sequential, each gated by `Scen.SpecialFlags & 0x8000`
       (`DestroyableBridges`) **AND** `warhead+0x144` (`Wall=`) and per-block Z/identity gates
       (call sites `0x00489fef`, `0x0048a173`, `0x0048a23f`, `0x0048a299`).
    2. Destroyable-overlay block (`LAB_0048a2c4`): on overlay-destroy (`overlay+0x2b0`), a
       `RandomRanged(0,99)` debris-VoxelAnim roll per debris type (call `0x0048a38e`) and a
       single `RandomRanged(0,99)` particle-system roll (call `0x0048a3dd`).
  - **For the Floating Disc specifically:** `[DiskWH]` in `ini/rulesmd.ini` (line 27526) has
    `Wall=no` and no `Wood=`, so `warhead+0x144 == 0` -> the **bridge RNG blocks (1) are
    NOT reached** by a disc shot. The destroyable-overlay debris/particle RNG (2) is **not**
    Wall-gated (it gates on the cell overlay's destroy-on-damage flag), so a disc FIRE that
    lands on a destroyable-overlay cell **can** still draw `RandomRanged(0,99)` from
    Scen->Random.

  Net: **rung 13 draws RNG = TRUE (conditional)**, stream **Scen->Random**, only on a disc's
  FIRE tick and only when its area damage lands on a destroyable-overlay cell (and never via
  the bridge blocks given DiskWH `Wall=no`). In the common case (disc fire over open ground)
  the rung draws **zero** RNG.

## Active in YR / TS-legacy

- **Active in YR: YES.** Not TS legacy. The DiskLaser system is the Floating Disc's primary
  weapon: `[DISK] Primary=DiskLaser` and `[DiskLaser] DiskLaser=yes` (`ini/rulesmd.ini`
  lines 8695 / 24255-24270; Elite variant `[DiskLaserE]` line 24754). `DiskLaserClass`
  objects are constructed from `TechnoClass::Fire_At` when the weapon's `DiskLaser=` flag is
  set. Floating Disc is a standard Yuri-side unit, buildable in a normal YR skirmish; its
  attack produces the visible expanding ring-laser, so this rung is player-visible whenever a
  disc attacks.

---

## Verification calls (inline)

- `decompile_function 0x0055AFB0`, `disassemble_function 0x0055AFB0` — body site
  `0055b5a1`–`0055b5be`; loop globals `0x008a0218` (count), `0x008a020c` (base).
- `get_xrefs_to 0x008a020c`, `get_xrefs_to 0x008a0218` — array holds DiskLaserClass; writers
  in constructor/destructor/Detach_From_All_Lists.
- `disassemble_function 0x004a7a70` (constructor at `0x004a7a30`) — vtable base `0x007e5fb8`;
  registers object into `0x008a020c` at `0x008a0218`.
- `read_memory 0x007e6014` = `40 73 4a 00` -> vt+0x5C = `0x004a7340` (binding verified, not
  label-trusted).
- `decompile_function 0x004a7340` — DiskLaserClass::AI state machine, static ring table
  `DAT_008a0180`, calls `Apply_area_damage`/`LaserDrawClass__Constructor`/`VocClass__PlayAt`/
  `FUN_004a7fe0`.
- `decompile_function 0x004a7fe0` — removal helper = DynamicVector push, no RNG.
- `decompile_function 0x0054fe60` — LaserDrawClass ctor, no RNG.
- `decompile_function 0x00489280`, `disassemble_function 0x00489280` — Apply_area_damage RNG
  call sites all use `ECX = [0x00a8b230]+0x218` (Scen->Random) via `Random__RandomRanged
  @ 0x0065c7e0`; Wall gate at `warhead+0x144` (`00489ec1`, `0048a14a`).
- `ini/rulesmd.ini` `[DiskWH]` (27526): `Wall=no`; `[DISK]`/`[DiskLaser]` confirm active-in-YR.
