# Bridge Runtime Deep-Dive — Ghidra Research Report

**Topic:** Phase 2/3 deepening of bridge runtime mechanics scoped but not executed in
[BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md](BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md).
Covers: HP storage / damage dispatch math, state-byte ladder, `_Low` vs `_High`
twin functions, `UpdateBridgeEdgeTiles` end-walker, the global "death-list" at
`DAT_0087F8C0`, vision/shroud behavior on collapse, audio dispatch, and corrections
to three load-bearing claims in the parent report.

**Addresses (primary):**
- `0x489280` — `Apply_area_damage` (bridge-damage entry, 4 parallel branches)
- `0x47DD70` — `BlowUpBridge` (corrects parent report's RulesClass offset labels)
- `0x571490` — `ProcessBridgeDamageStateMachine_Low`
- `0x576BA0` — `ProcessBridgeDamageStateMachine_High`
- `0x570AE0` — `MapClass__UpdateBridgeEdgeTiles_Low`
- `0x576200` — `MapClass__UpdateBridgeEdgeTiles_High`
- `0x578100` — `MapClass__RecalcBridgeShroudFlags` (120-frame poll, undocumented)
- `0x55AFB0` — `LogicClass__PerTickUpdate` (calls the poll)
- `0x519630` — `InfantryClass::PerCellProcess` (CABHUT C4 repair audio)
- `0x66CD60` — `RulesClass__ReadCombatDamage` (BridgeStrength, C4Warhead, IonCannonWarhead reader)

**Confidence:** HIGH overall. Three doc-correction findings cross-verified by
direct disassembly read of `RulesClass__ReadCombatDamage` and grep on the
`RulesClass__ReadGeneral` decompile text. Two LOW findings flagged in
Open Questions.

**Active in YR:** All findings active in retail YR with the following exceptions:
- Global death-list `DAT_0087F8C0..D0`: DEAD TS-legacy, every push silently
  dropped by BSS-zero state machine.
- `_Low`'s EW-collapse → `*_High` damage-helper call: live but possibly an
  unintentional carry-over (flagged for verification before any port).

---

## 1. Overview

The deferred-mechanics investigation produced an excellent map of the static
bridge code surface (RecalcAttributes, CheckBridgeTraversal, Can_Enter_Cell,
SetBridgeDirection, BlowUpBridge, Process_Drive_Track). It did NOT decompile
the cascade state machines, the edge-tile fixup walkers, or the audio/shroud
side-effects, and it mis-labeled three RulesClass offsets. This report fills
those gaps end-to-end.

**Headline new findings:**

1. **Bridge HP is NOT tracked per cell.** Every weapon hit on a bridge cell is
   a fresh stateless RNG roll: `RandomRanged(1, BridgeStrength) < raw_damage`.
   Two consecutive 50-damage hits are NOT equivalent to one 100-damage hit
   probabilistically; the player observes independent per-shot outcomes.
   IonCannon and C4 bypass the RNG entirely and retry up to 3 extra times.

2. **There is no bridge-specific collapse sound.** The audio a player hears
   when a bridge breaks is purely the weapon warhead's normal `ReportSound=`.
   The only bridge-specific audio in retail YR is the CABHUT-repair EVA voice
   ("EVA_BridgeRepaired", `RadarEvent` type 14) plus an optional
   `RepairBridgeSound=` SFX whose default is undefined (RulesClass `+0x248` =
   -1, and rulesmd.ini does not set it).

3. **`MapClass__RecalcBridgeShroudFlags @ 0x578100` runs every 120 frames** from
   `LogicClass__PerTickUpdate`. It clears explored+visible bits (`+0x12C &
   0x18`) on cells flagged with `0x20` in `+0x140`. This is a continuous
   poll, not a collapse-triggered event — but it's the mechanism that keeps
   bridge-edge shroud-bitmask caches synchronized as bridge cell geometry
   changes. Undocumented anywhere previously.

4. **Bridge cells block LOS via *height* (`+0x11B`), not via the bridge bit
   `0x100`**. No code special-cases the bridge flag for sight calculations.
   When collapse changes `+0x11B`, future reveal spirals naturally
   re-evaluate; no explicit invalidation.

5. **`_Low` is NOT a copy of `_High`.** They share the 18-state ladder
   semantically but differ in compiled register/stack layout, in the overlay
   tile-ID base globals (`DAT_00abad1c` low vs `DAT_00aa0e28` high), in the
   final direction enum (NWSE vs NESW), and one EW-collapse anomaly in `_Low`
   that invokes `*_High` damage helpers.

6. **The global death-list at `DAT_0087F8C0..D0` is dead TS-legacy.** BSS-zero
   init, no consumer, no allocator, no tick processor anywhere. The
   `BlowUpBridge` push at `0x47DDD5..0x47DE2D` always falls through the
   capacity-and-growth check and silently no-ops.

---

## 2. Corrections to BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md

The parent report (§3.7) attributes three RulesClass offsets that direct
disassembly read disproves. Treat the parent report's §3.7 as stale until
the corrections below are applied.

| Offset | Parent doc claim | **VERIFIED CORRECT VALUE** | Evidence |
|--------|------------------|----------------------------|----------|
| `g_RulesClass + 0xFA8` | "BridgeBlast weapon" | **`C4Warhead`** | `RulesClass__ReadCombatDamage @ 0x66C32C` reads `s_C4Warhead_0083b1d4` into `*(undefined4*)(param_1 + 0xfa8)`. String `"BridgeBlast"` does not exist in YR. The C4Warhead pointer is the parameter `BlowUpBridge @ 0x47DD70` passes to vtable+0x16C (Take_Damage) for occupants. |
| `g_RulesClass + 0x140` | "BridgeExplosions anim list ptr" | **`MetallicDebris` vector base ptr** | `RulesClass__ReadGeneral` parses `MetallicDebris` into a DynamicVectorClass anchored at `+0x140` (base) / `+0x14C` (size) / `+0x150` (cap). |
| `g_RulesClass + 0x15C` | (not labeled in parent) | **`BridgeExplosions` vector base ptr** | Same function parses `BridgeExplosions` into `+0x15C` (base) / `+0x168` (size) / `+0x16C/+0x170` (cap/sentinel). |

**Also verified at known offsets:**
- `+0x1740` = **`BridgeStrength`** (i32, default 100 in retail). Reader at
  `0x66CD60..0x66CD80`. Same function; reads via `CCINIClass::ReadInt`.
- `+0xFF0` = **`IonCannonWarhead`** (warhead ptr). Reader at `0x66CA9B..0x66CAA8`.
- `+0xFAC` = **`CrushWarhead`** (warhead ptr). Reader at `0x66C36A..0x66C377`.
- `+0x248` = **`RepairBridgeSound`** (sound ID, default `-1` if INI not set).
  Reader in `RulesClass::ReadAudioVisual @ 0x669F0A`. Retail rulesmd.ini does
  not set this key, so the SFX is never played in stock YR.

**Important read order in `BlowUpBridge`:** the function reads `+0xFA8`
(C4Warhead) for the `FirstObject` damage walk (vtable+0x16C = Take_Damage),
then reads `+0x140` (MetallicDebris base) and `+0x15C` (BridgeExplosions
base) for the random debris animation spawn. The parent report's labels
swap the second pair.

---

## 3. Damage Dispatch — Stateless RNG, No Per-Cell HP

### 3.1 The four parallel branches in `Apply_area_damage` (0x489280)

For each cell in the area-damage radius, four parallel code blocks each test
whether the cell is a bridge piece and run their own RNG + destroy attempt:

1. Low-bridge anchor zone (cell flag bit `0x200` over a `0x18/0x19` overlay)
2. High-bridge anchor zone (cell flag bit `0x200` over a `0xED/0xEE` overlay)
3. Low-bridge body overlay range `0x4A..0x63` (wood bridge body 26 frames)
4. High-bridge body overlay range `0xCD..0xE6` (concrete bridge body 26 frames)

**Outer gate (CORRECTED 2026-05-13 — `SpecialFlags & 0x8000`)** — all four
bridge damage branches are gated at the top by:

```c
if ((*g_ScenarioClass_Instance & 0x8000) == 0
    || *(char*)(param_4 + 0x144) == 0)
  goto LAB_0048a2c4;   // skips all bridge damage code
```

- `*g_ScenarioClass_Instance & 0x8000` = the **`DestroyableBridges`** map
  flag (set per-map via INI). When off, **no bridge damage of any kind
  fires**, regardless of warhead.
- `*(char*)(param_4 + 0x144)` = the warhead's `Bridge=yes` flag. When off,
  the warhead is incapable of damaging bridges (most warheads default off;
  C4Warhead, IonCannonWarhead, and a small set explicitly set it on).

Each branch evaluates an identical RNG gate (pseudo-code, **direct
disassembly read of `Apply_area_damage @ 0x489280`**):

```
warhead   = current weapon's WarheadTypeClass*
raw_dmg   = uStack_cc  (damage at this cell, post-falloff)
strength  = *(int*)(g_Rules + 0x1740)         // BridgeStrength, default 100

is_ion    = (warhead == *(g_Rules + 0xFF0))   // IonCannonWarhead

bypass    = is_ion || (RandomRanged(1, strength) < raw_dmg)

if (warhead.Bridge && bypass) {
  ok = ApplyDamageToCell(...)
  iVar = 3
  while (ok == 0) {
    if (!is_ion || iVar < 1) break          // (!) only IonCannon retries
    ok = ApplyDamageToCell(...)
    iVar--
  }
  // success path: TechnoClass__StopAllTargeting() + DirtyScreenRect
}
```

### 3.2 Tiny-detail captures (CORRECTED 2026-05-13)

The previous version of this doc claimed C4Warhead enters the retry loop.
**That was wrong.** Re-reading the Ghidra decompile in full revealed a
*second* reassignment of `bVar21` immediately before the bridge sections:

```c
// Top of function (used for self-damage policy in the top occupier loop):
bVar21 = param_4 == *(int*)(g_RulesClass + 0xFAC);   // is-CrushWarhead

// LATER, right before the bridge code (line ~`LAB_0048a0a5` predecessor):
bVar21 = param_4 != *(int*)(g_RulesClass + 0xFF0);   // is-NOT-IonCannon
```

The C++ compiler reused the local for two different predicates. The retry
loop uses the LATER value (`!is_ion`), so:

| Warhead | RNG bypass? | Retry loop? | Total ApplyDamageToCell attempts |
|---------|-------------|-------------|----------------------------------|
| `IonCannonWarhead` | YES (bypass) | YES (3 retries) | up to 4 |
| `C4Warhead` | no (RNG-gated) | **NO** (break immediately) | exactly 1 |
| `CrushWarhead` | no | NO | exactly 1 |
| Any other warhead with `Bridge=yes` | no | NO | exactly 1 |
| Any warhead with `Bridge=no` | N/A — outer gate skips | N/A | 0 |

**This is the load-bearing correction.** SEAL/Tanya C4 on a bridge does NOT
get 4 attempts like IonCannon; it gets exactly one RNG-gated attempt, the
same as a regular weapon. The behavioral asymmetry between C4 and IonCannon
is therefore much larger than the previous version of this doc implied.

Other tiny details (still valid):

- **`RandomRanged(1, strength)`** uses inclusive bounds. With `strength=100`
  the result is `[1, 100]`. A `damage = 100` shot has a 99/100 (=99%) chance
  to advance, NOT 100% — `damage == strength` can roll equal and fail.
- **`raw_dmg`** is post-falloff damage at the cell, NOT the warhead base
  damage. A 500-damage warhead with steep falloff at radius edge may pass
  only 30 to the gate.
- **IonCannon retry count = 3 extra (4 total).** Loop: `iVar = 3; while
  (cVar7 == '\0') { ...; iVar--; if (iVar < 1) break; }`. Decrement after
  call; the post-condition `iVar < 1` is the exit.
- **There is no shared HP pool.** Each of the four branches has its own RNG
  call. A single weapon impact rolls independently against the same
  BridgeStrength for the low-zone, high-zone, low-body, and high-body
  branches on cells in radius.
- **Per-branch screen-dirty rects.** On success, low-bridge branches dirty a
  `0x100 × 0x100` (256×256) rect; high-bridge branches dirty `0xc0 × 0xc0`
  (192×192). The high-bridge rect offset is `(-0x60, -0x18)` — a 96-px
  horizontal + 24-px vertical cushion, asymmetric. Low-bridge rect offset
  is `(-0x80, -0x80)`.

### 3.3 Where cells track state — `+0x11E` byte ladder (verified)

`CellClass+0x11E` is the **anchor-state byte**. For non-anchor body cells, the
state is read via the anchor pointer at `+0x2C`. The anchor selector pattern
appears throughout the cascade:

```c
anchor = (this->Flags & 0x80) ? this : *(CellClass**)(this + 0x2C);
state  = *(u8*)(anchor + 0x11E);
```

**The 18-state ladder is one byte per anchor.** No HP counter exists anywhere
in CellClass; the state byte is the only persistent damage state. Transitions
are driven by RNG-gated calls to `ProcessBridgeDamageStateMachine_{High,Low}`
which read the state byte, dispatch helpers, and write the next state.

| State | Axis | Action on transition | Next state written |
|-------|------|----------------------|---------------------|
| 0–5 | NS healthy | `UpdateRamp_NS_DamageA` + `UpdateRamp_NS_DamageB` | 6 |
| 6 | NS partial | `UpdateRamp_NS_CollapseA` + `UpdateRamp_NS_CollapseB` | 0 (reset); `+0x44 := -1` (overlay clear) |
| 7 | NS partial | `UpdateRamp_NS_CollapseA` only | 0 |
| 8 | NS partial | `UpdateRamp_NS_CollapseB` only | 0 |
| 9–0xE | EW healthy | `UpdateRamp_EW_DamageA` + `UpdateRamp_EW_DamageB` | 0xF |
| 0xF | EW partial | `UpdateRamp_EW_CollapseA` + `UpdateRamp_EW_CollapseB` | 0 |
| 0x10 | EW partial | `UpdateRamp_EW_CollapseB` only | 0 |
| 0x11 | EW partial | `UpdateRamp_EW_CollapseA` only | 0 |

**Repair path** (via `SetBridgeDirection`): writes `+0x11E := 0` (NS) or `9`
(EW) on the anchor and its swept neighbors, depending on the `direction`
parameter. So repair takes a bridge from any partial-collapse state straight
back to "healthy at the start of its axis ladder."

**Tiny detail — overlay-clear on partial collapse:** State 6 / 0xF transitions
also write `+0x44 = 0xFFFFFFFF` (OverlayTypeIndex = -1, the "no overlay"
sentinel). This is what visually replaces the bridge-body sprite with the
collapsed-ramp tile.

### 3.4 Low-bridge shore/ramp cells (state at `+0x11A`, not `+0x11E`)

`ProcessBridgeDamageStateMachine_High @ 0x576BA0` lines ~50–110 handle a
**parallel small state machine** for the shore/ramp cells of low-bridge
variants, using `+0x11A` (sub-tile index byte) as a 4-cell stage counter
incrementing 4 → 5 → continuing until 5, then triggering `BlowUpBridge` on
three adjacent body cells. **The shore-ramp state is on a different byte
(`+0x11A`) from the body-anchor state (`+0x11E`).** Iron-law: this is not the
same state machine, and the byte semantics overlap with the terrain-sub-type
byte used elsewhere — context-sensitive per cell role.

---

## 4. `_Low` vs `_High` State Machines — Compiled-Twin Verdict

### 4.1 Function sizes (NOT byte-identical)

| Variant | Entry | End | Body bytes |
|---------|-------|-----|------------|
| `_Low` @ `0x571490` | `0x571490` | `0x5721EC` | **0xD5C (3,420)** |
| `_High` @ `0x576BA0` | `0x576BA0` | `0x5778E9` | **0xD49 (3,401)** |

Delta: **19 bytes.** Not byte-identical, not even same size. They are
**structurally-similar / compiled-twin**: same algorithm, separately scheduled
by the compiler (different register allocation, different stack layout, two
distinct function bodies in the binary).

### 4.2 Prologue diverges immediately

| Off | `_Low` (`0x571490`) | `_High` (`0x576BA0`) |
|-----|---------------------|----------------------|
| +0 | `83 EC 54 53 55 8B 6C 24 60` | `83 EC 54 53 8B 5C 24 5C 55` |
| +9 | `8B 1D 24 F9 87 00` (`mov ebx, [DAT_0087F924]`) | `8B E9` (`mov ebp, ecx`) |
| +F | `89 4C 24 08 56` | `0F BF 43 02 0F BF 0B` |

`_Low` uses `local_50` + `sStack_52`; `_High` uses `local_54` (different temp
arrangement). The shared algorithm cannot have been emitted from a single
function source-of-truth — they're either two source functions or one source
function with conditional templating that compiles to two bodies.

### 4.3 State-table semantics: identical 18-state ladder, different base globals

Both functions branch on cases `0..0x11` (states 0–17), use the same
`+0x11E` state byte, and call the same family of helpers — but with different
*overlay-frame anchor* globals:

| Variant | Overlay base (the `-base + 1` index calc) |
|---------|-------------------------------------------|
| `_Low` | `DAT_00abad1c` (wood/low bridge overlay base) |
| `_High` | `DAT_00aa0e28` (concrete/high bridge overlay base) |

`DAT_00abad30` (NS damage base) and `DAT_00aa1028` (EW damage base) are
referenced by **both** — those are axis-specific, not variant-specific.

**Implication for parity:** state IDs 0–17 are identical between low and
high bridges, but the absolute overlay frame ID written via
`SetOverlayAndPropagate` differs by the variant's base. Two parallel constant
tables are required for any port — the state index alone is not enough to
compute the visual outcome.

### 4.4 Caller routing — confirmed via `ApplyDamageToCell @ 0x587180`

- `_High` is called when the adjacent bridge-head cell has overlay ID `0x18`
  or `0x19` (concrete-bridge butt), through the `flags & 0x100` path.
  (Note: `0xED/0xEE` is checked in `Apply_area_damage` upstream, but inside
  `ApplyDamageToCell` the *adjacency* dispatcher routes via `0x18/0x19`.)
- `_Low` is the fallback (`LAB_005872B9`) for the wooden-bridge overlay range
  (covers `0xED/0xEE` per outer condition).
- Direct-destruction paths bypass the state machine: overlay range
  `[0xCD..0xE6]` calls `DestroyBridge_High`; `[0x4A..0x63]` calls
  `DestroyBridge_Low`.

### 4.5 Helper dispatch deltas

- `_Low` calls `*_Low` ramp helpers (e.g., `UpdateRamp_NS_DamageA_Low`).
- `_High` calls `*_High` helpers.
- `_Low` final direction set: `CellClass__SetBridgeDirection_NWSE(uVar12, 0)`
  where `uVar12 ∈ {0, 6}`.
- `_High` final direction set: `CellClass__SetBridgeDirection_NESW(0/6, 0)`.
- `_Low` adjacency update: `MapClass__UpdateAdjacentBridges`.
- `_High` adjacency: `MapClass__UpdateAdjacentBridges_High`.

So **low-bridge runtime uses the NWSE diagonal pair** and high-bridge uses
the NESW pair. This matches the natural visual axis for each variant.

### 4.6 Anomaly: `_Low` EW-collapse calls `*_High` damage helpers

At approximately line 256 of the `_Low` decompile, the EW-damage degrade
branch at the end of the EW-collapse arm calls
`MapClass__UpdateRamp_EW_DamageA_High` and `..._DamageB_High` — **NOT
`*_Low`**. This is inconsistent with the rest of the function, which uses
`*_Low` helpers throughout.

Two possible explanations:
1. **Original Westwood bug** — copy-paste leftover from the `_High` template.
2. **Intentional shared-helper reuse** — the damage helpers in this corner
   may be visually identical regardless of bridge variant, so calling the
   `*_High` version is harmless.

**Iron-law: this is a real behavior, must be reproduced regardless of intent
when the parity bar is "indistinguishable from gamemd."** Worth a focused
follow-up to determine whether the `*_High` damage helpers visually differ
from the `*_Low` ones in this exact context.

### 4.7 Identical epilogue (verified)

Both functions reach a shared epilogue pattern at `LAB_005721B1` (`_Low`) /
`LAB_005778CC` (`_High`): `InvalidateBridgeZones` → conditional
`UpdateBridgeZonesHelper` → `return 1`. The final 6 bytes are byte-identical:
`83 C4 54 C2 04 00` (`add esp, 0x54; ret 4`).

---

## 5. `MapClass__UpdateBridgeEdgeTiles_{High,Low}` (0x576200 / 0x570AE0)

### 5.1 Purpose

After a damage or repair cascade, this function walks **up to 30 cells in a
straight line** from the bridge anchor along a given axis, looking for a
matching theater-loaded endpoint tile. When found, it back-walks the same
line and repairs the *one* segment immediately adjacent to a still-intact
piece by re-orienting it via `SetBridgeDirection` and clearing damage state
(`+0x11E = 0`, `+0x44 = -1`). Then it **tail-recurses** to handle the next
damaged seam in the same direction.

### 5.2 Compiled-twin verdict

Ghidra's `diff_functions` reports **412 of 412 instructions equal**, similarity
0.8286. Three actual behavioral differences:

| What | High (`0x576200`) | Low (`0x570AE0`) |
|------|-------------------|-------------------|
| Bridge tile-ID base | `DAT_00aa0e28` | `DAT_00abad1c` |
| Direction-setter | `SetBridgeDirection_NESW` | `SetBridgeDirection_NWSE` |
| Recursive callee | self (`0x576200`) | self (`0x570AE0`) |

All other code is identical. The theater bridge-endpoint tile-IDs
(`DAT_00abc1e8`, `DAT_00aa0e38`, `DAT_00abad30`, `DAT_00abc1d0`,
`DAT_00aa1540`, `DAT_00aa1028`) are shared by both — they're theater-resolved
globals (different per Temperate/Snow/Urban/etc.) but not variant-specific.

### 5.3 Walk pattern (the 30-cell linear scan)

```
step = (param_3 & 7)              // param_3 = 2 (NS) or 4 (EW)
cell = start_cell                 // param_2 = (x,y) anchor coord
for (i = 0; i < 30; i++) {
  coord  = *(short*)(cell + 0x24) // {short x, short y}
  coord += g_DirectionOffsets[step]
  idx    = coord.y * 0x200 + coord.x
  if (idx < 0 || idx >= 0x3FFFF) {
    cell = &g_dead_cell_sentinel  // DAT_00abdc50
    *(short[2]*)&DAT_00abdc74 = bad_coord  // standard MapClass decoy
    continue
  }
  cell = g_CellArray_Base[idx]
  if (cell == NULL) continue
  // check endpoint-tile match — see §5.4
}
```

Constants captured:
- **`0x1e` (30) = max scan length.** The longest standard RA2/YR bridge is
  well under this.
- **`y * 0x200 + x`** indexing = 512×512 map maximum.
- **Bounds compare** uses `JGE` after `JS` (signed semantics; `idx` must be
  non-negative AND `< 0x40000`).
- **Out-of-map fallback** is the standard MapClass "dead cell" decoy at
  `DAT_00abdc50` with the bad coord stashed at `DAT_00abdc74` — same decoy
  used by `MapClass::GetCellAt` etc.

### 5.4 Endpoint-tile match table

After each step, the function reads `tile_id = *(cell + 0x38)` and
`subtile = *(cell + 0x11A)`, computes `iVar5 = (tile_id - BridgeBase) + 1`,
then matches:

| `param_3` | Match conditions (any) | Required subtile @ `+0x11A` |
|-----------|------------------------|------------------------------|
| **2 (NS)** | `iVar5 == DAT_00abc1e8` | 4 |
| 2 (NS) | `iVar5 == DAT_00aa0e38` | 4 |
| 2 (NS) | `iVar5 ∈ {DAT_00abad30, +1, +2, +3}` | 4 |
| **4 (EW)** | `iVar5 == DAT_00abc1d0` | 2 |
| 4 (EW) | `iVar5 == DAT_00aa1540` | 2 |
| 4 (EW) | `iVar5 ∈ {DAT_00aa1028, +1, +2, +3}` | 2 |

NS scan requires subtile **`\x04`** (north-facing ramp); EW scan requires
subtile **`\x02`** (east-facing ramp). The `{base, base+1, base+2, base+3}`
cluster is the 4-frame ramp set per theater.

### 5.5 The actual edge fixup

If endpoint found within 30 cells, back-walk with a 2-cell sliding window
checking `*(cell + 0x140) & 0x80` (anchor flag). When the pattern transitions
from damaged-then-intact AND both `last_damaged_coord` and `current_coord`
sentinels are non-`-1`, perform the one-shot fixup:

1. Derive a cell **one step beyond, in the opposite direction**:
   `step_back = (param_3 - 4) & 7`. For `param_3=2` this is `6` (SW); for
   `param_3=4` this is `0` (NE). This is the 180°-rotated direction.
2. Call `SetBridgeDirection_{NESW|NWSE}` with arg `0` (NS) or `6` (EW). The
   `0`/`6` is the *ramp variant index* (the tile pose), not the walking
   direction.
3. Write `cell[0x11E] = 0` (clear damage state byte).
4. Write `cell[0x44] = 0xFFFFFFFF` (overlay clear).
5. Call `RadarClass__MarkTerrainDirty(cell + 0x24)` (radar redraw).
6. **Tail-recurse** into self with same `(param_2, param_3, param_4)` — walks
   again to handle the next damaged seam.

If during back-walk the current cell is NOT damaged AND no repair done yet,
call `RepairBridgeSegment(cell_coord_packed, last_seam_coord_packed)` once
(the "soft" repair for an already-intact seam that just needs the state fix).

### 5.6 Dirty-rect math (`param_4`, optional `int[4]` output)

When `param_4 != 0`, the function computes a screen-space bounding rect:

- Start cell center: `(*param_2 * 0x100 + 0x80, *param_2[1] * 0x100 + 0x80)`
  in world coords, with `Z = subtile[0x11B] * DAT_00abde88` (= `ZGRAN`,
  level-step in pixels).
- Same for found-endpoint cell.
- Pass each through `TacticalClass__CoordsToClient2`.
- Top-left = `(min(x1,x2) - 0x40, min(y1,y2) - 0x40)`. The `0x40` is a 64-px
  cushion on each side.
- Size = `(|dx| + 0x80, |dy| + 0x80)`. The `+0x80` matches the cushion both
  sides.
- Union-merge with existing rect in `param_4[0..3]` via clamp logic (zero-area
  rect is replaced; non-zero is expanded).

Caller (`UpdateAdjacentBridges{,_High}`) then does the actual screen
invalidation via `TacticalClass__DirtyScreenRect`.

### 5.7 Side effects (per fixup invocation)

`SetBridgeDirection`, `RadarClass__MarkTerrainDirty`, two cell-byte writes
(`+0x11E := 0`, `+0x44 := -1`), optional `RepairBridgeSegment`, dirty-rect
expansion via two `CoordsToClient2` calls. **No animation, no SFX call from
this function itself.**

---

## 6. The Global Death-List at `DAT_0087F8C0..D0` — DEAD TS-LEGACY

### 6.1 Data shape

The block at `0x0087F8BC..0x0087F8D0` is a TS-era `DynamicVectorClass<int>`:

| Address | Field | Type | Meaning |
|---------|-------|------|---------|
| `0x0087F8BC` | vtable ptr | `void**` | vtable hosts `IncreaseMaxSize` at `+0x8` |
| `0x0087F8C0` | `Items` | `int*` | array base; element type = packed CellStruct (`y<<16 \| x`) |
| `0x0087F8C4` | `Capacity` | `int` | starts 0 (BSS) |
| `0x0087F8C9` | `IsAllocated` | `bool` | offset +0xD into struct |
| `0x0087F8CC` | `Count` | `int` | next write index |
| `0x0087F8D0` | `GrowthStep` | `int` | starts 0 (BSS) |

The element pushed by `BlowUpBridge` is `*(dword*)(cell + 0x24)` — a packed
`{short x; short y;}` coord, NOT a CellClass pointer.

### 6.2 The only writer (in `BlowUpBridge @ 0x47DD70`)

```
if Count < Capacity:                                     → push directly
else if ((IsAllocated || Capacity == 0) && GrowthStep > 0
         && vtable.IncreaseMaxSize(GrowthStep+Capacity, 0)):
                                                          → push
else: silently drop
Items[Count++] = packed_cell_coord
```

### 6.3 Why the push is unreachable in retail YR

BSS-zero state: `Capacity=0, IsAllocated=0, GrowthStep=0`. The
`(IsAllocated != 0 || Capacity == 0) && GrowthStep > 0` chain evaluates
`true && false` → false. **Every push at runtime is silently dropped.**

### 6.4 Xref enumeration (complete)

- `0x0087F8C0` — 1 ref: READ at `0x47DE19` (inside BlowUpBridge)
- `0x0087F8C4` — 1 ref: READ at `0x47DDD5` (inside BlowUpBridge)
- `0x0087F8C8` — **no refs anywhere**
- `0x0087F8C9` — 1 ref: READ at `0x47DDE4` (inside BlowUpBridge)
- `0x0087F8CC` — 3 refs, all inside BlowUpBridge (`0x47DDDA` R,
  `0x47DE13` R, `0x47DE22` W)
- `0x0087F8D0` — 1 ref: READ at `0x47DDF2` (inside BlowUpBridge)
- `0x0087F8BC` — 2 refs: vtable read at `0x47DDFC`, `this`-load at
  `0x47DE07` (both inside BlowUpBridge)

**No constructor, no `IncreaseMaxSize` call site, no allocator, no reader
outside BlowUpBridge.** No init in Main_Tick, scenario startup, or save-load.

### 6.5 The pattern is alive elsewhere

Identical `DynamicVectorClass<int>` structure (with vtable at
`PTR_FUN_007e3890`) appears live in `ProcessBridgeDestruction_Low @ 0x5703A0`
as a **stack-allocated** vector drained by `FUN_00586990` and freed via
`FUN_007c8b3d`. That use is proper.

The TS pattern: original TS engine wired BlowUpBridge into a deferred
mechanic via this global vector; in YR Westwood gutted the wiring (kept the
push, removed the consumer and the initializer). BSS-zero ensures the push
no-ops.

### 6.6 Port verdict

**Do not port the list. No tick-ordering concern.** The visible behavior of
BlowUpBridge comes entirely from (a) the FirstObject/AltObject damage loops
via vtable+0x16C and vtable+0xEC, and (b) the random debris animation block.
The death-list push is silent dead code.

The parent report's "TS-era control flow worth flagging" note can be
downgraded to "TS-legacy, confirmed dead — no port action needed."

---

## 7. Vision / Shroud / LOS on Bridge Collapse

### 7.1 No explicit shroud or sight update in the collapse path

Decompiled every function in the collapse call chain. **None of them write
`cell + 0x12C`** (the shroud-bits dword):

- `BlowUpBridge @ 0x47DD70` — only `AnimClass__Constructor`, `Math__ftol`,
  `RandomRanged`, `operator_new` callees. Vtable+0x16C (Take_Damage) and
  vtable+0xEC (DropIn) are the only object touches. No reveal/sight.
- `SetBridgeDirection_NESW @ 0x47E040` / `_NWSE @ 0x47E470` — only flag-bit
  twiddling, `BlowUpBridge`, `RadarClass__MarkTerrainDirty`. No reveal.
- `ProcessBridgeDamageStateMachine_High @ 0x576BA0` / `_Low @ 0x571490` —
  overlay propagation, `BlowUpBridge` calls, ramp updates. No reveal.
- `ProcessBridgeDestruction_{High,Low}` (`0x573540`, `0x570050`) — geometry
  + recursion + `UpdateRamp` + `ToggleBridgePavement` +
  `TacticalClass__DirtyScreenRect`. No reveal.
- `MapClass__UpdateAdjacentBridges_High @ 0x576770` — endpoint resolve +
  DirtyScreenRect. No reveal.
- `MapClass__UpdateRamp_*_CollapseA/B_*` (8 functions at `0x56EF50`,
  `0x56F2F0`, `0x56F8B0`, `0x56FC80`, `0x572440`, `0x5727E0`, `0x572DA0`,
  `0x573170`) — only `BlowUpBridge`. No reveal.

### 7.2 The canonical shroud writers (for reference)

`CellClass__RevealShroudFlags @ 0x4876F0` is the **only** writer of the
shroud bits at `+0x12C`:

```
*(uint*)(cell + 0x12C) |= 0x18      // bit 0x08 = explored, 0x10 = visible
if (*(int*)(cell + 0x130) > 0)
  *(uint*)(cell + 0x140) |= 0x20    // some derived flag
```

`IsShrouded @ 0x586360` reads `(cell + 0x12C) & 0x08`. **Bridge collapse paths
never touch +0x12C.**

### 7.3 Bridge cells block LOS via *height*, not the bridge flag

`MapClass__RevealShroud @ 0x5673A0` (the reveal spiral) checks `cell + 0x11B`
(height level) and `cell + 0x140 & {0x01, 0x02}` (terrain block flags). It
does NOT special-case bit `0x100` (bridge bit).

**Implication:** when a bridge collapses, the ramp/overlay updates change
`cell + 0x11B` (height). Future reveal spirals naturally re-evaluate LOS at
the new height without any explicit invalidation. No code is needed in the
collapse path to update sight.

### 7.4 vtable+0xEC = `DropIn`, not destruction

`BlowUpBridge`'s call to vtable+0xEC for `AltObject` (bridge-layer occupants)
resolves to `ObjectClass__DropIn @ 0x5F4160`. Behavior:

- Flips bytes `+0x8D` and `+0x8F` to `1` (marks "in air / falling")
- `DisplayClass__RemoveFromLayer` then re-submits to a different layer
- Calls vtable+0xF4 (fall logic)

**It does NOT call `Limbo`, does NOT clear reveal, does NOT remove the unit's
reveal contribution.** The reveal stays in place until the unit actually dies
via fall damage through normal TechnoClass update — which goes through
standard death code, not bridge-specific.

### 7.5 NEW finding — `RecalcBridgeShroudFlags @ 0x578100`, 120-frame poll

Called every 120 frames (`0x78`) from `LogicClass__PerTickUpdate @ 0x55AFB0`
(at `LAB_0055B29A`). Mechanism:

1. Iterate every cell on the map.
2. If `cell + 0x140 & 0x20`:
   - Clear `cell + 0x12C &= 0xFFFFFFE7` (drops bits **0x08 and 0x10** —
     that's **explored AND visible**!)
   - Clear `cell + 0x140 &= 0xFFFFFFDC` (drops bits 0x20, 0x02, 0x01)
3. Second pass: rebuild edge bitmasks via `Shroud_EdgeBitmask_Calculator`.

**This is a periodic poll, not collapse-triggered.** It runs continuously
regardless of whether anything collapsed. The cells re-reveal on the next
normal reveal pass via `RevealShroudFlags`. The polling cadence is the
mechanism that lets per-edge shroud-bitmask caches stay synchronized as
bridge cell geometry changes during gameplay.

**Tiny detail — the 120-frame cadence is hard-coded.** No INI key controls
it. At 15 FPS sim that's 8 seconds; at 60 FPS sim (if speed slider is
maxed) it's 2 seconds. The bridge-edge shroud cache resyncs at that
cadence.

**Iron-law: this is the only continuous shroud-touching mechanism in the
bridge code surface, and it's been completely undocumented.** Any Rust port
that mirrors retail bridge shroud behavior needs this poll wired into the
sim tick.

### 7.6 `RadarClass__MarkTerrainDirty @ 0x6551C0` is pure radar bookkeeping

It scans a dedup buffer at `RadarClass + 0x1228` (sized at `+0x122C`, length
at `+0x1234`), appends the coord if not already present, and sets a single
flag byte at `RadarClass + 0x14D9`. No shroud, no sight, no cell flags
written. Pure minimap-redraw invalidation.

### 7.7 CABHUT destruction does NOT trigger a bridge-specific reveal change

CABHUT is a normal `BuildingClass`. Its reveal radius follows standard
building destruction logic in `BuildingClass::Limbo` / `Detach`. The
bridge collapse cascade never invokes a reveal-area function tied to the
bridge geometry. CABHUT death loses its own reveal radius via the standard
building path; the bridge itself contributes nothing.

---

## 8. Audio Dispatch

### 8.1 Bridge collapse — NO bridge-specific sound

Verified zero `VocClass__Play*` / `VoxClass__PlayEVA` calls in:
`BlowUpBridge`, `SetBridgeDirection_NESW`, `SetBridgeDirection_NWSE`,
`ProcessBridgeDamageStateMachine_High`, `ProcessBridgeDamageStateMachine_Low`,
`ProcessBridgeDestruction_High`, `ProcessBridgeDestruction_Low`,
`UpdateAdjacentBridges_High`, all 8 `UpdateRamp_*_Collapse*` functions.

**The only audio when a weapon damages a bridge is the warhead's normal
`ReportSound=`** from `CellClass::Receive_Damage`. Players hear weapon
impact, not bridge-specific destruction audio.

### 8.2 CABHUT C4 repair — the ONLY bridge-specific audio path

`InfantryClass::PerCellProcess @ 0x519630`, inside the
`BombDisarmer`/`C4`/spy-style entry branch (gated by
`BuildingTypeClass + 0x16B6 = BridgeRepairHut=yes`):

| Addr | Action |
|------|--------|
| `0x519BBE` | `cVar2 = HouseClass__IsHumanPlayer()` gate |
| `0x519BC4` | `mov ecx, 0xE` ; **`call CreateRadarEvent @ 0x65FA70`** — creates `RadarEvent` type **14 = "Bridge Repaired Event"**. String `"Bridge Repaired Event"` at `0x008397D8`, config table entry at `g_RadarEventTypeConfig + 14*0x10`. |
| `0x519BD4` | On success: **`call VoxClass__PlayEVA(0xFFFFFFFF) @ 0x752700`** — plays the EVA voice from that radar event (`"EVA_BridgeRepaired"` string at `0x00825538`) |
| `0x519BE5` | `if (*(int*)(g_RulesClass + 0x248) != -1)` — gate on `[AudioVisual] RepairBridgeSound=` being defined |
| `0x519BFC` | `local_18 = bridge X/Y/Z`, **`call VocClass__PlayAt(0) @ 0x7509E0`** — plays `RepairBridgeSound` SFX at the bridge location |

### 8.3 Tiny details

- **`RepairBridgeSound`** stored in RulesClass at int-offset `0x248`
  (word-offset `0x92`). INI key parsed in `RulesClass::ReadAudioVisual @
  0x669F0A`. **Default: `-1`** (no SFX set). **Retail rulesmd.ini does NOT
  define this key**, so the spatial SFX is never played in stock YR — only
  the EVA voice fires.
- **`RadarEvent` type 14** drives both the minimap blip and the EVA voice
  via the radar-event config table. `VoxClass__PlayEVA(0xFFFFFFFF)` reads
  the voice ID from the radar event's config slot.
- **`IsHumanPlayer` gate** — AI repairs do NOT play the EVA voice. Only the
  human player hears it. Verified at `0x519BBE`.

### 8.4 CABHUT building destruction — no bridge audio

`MapClass__UnregisterBridgeRepairHut @ 0x577920` (called from `FUN_007258D0`,
the generic BuildingClass/BuildingTypeClass Detach dispatcher) only manages
the `RulesClass + 0x1160 / 0x116C` repair-hut registry array. No VocClass /
VoxClass calls. CABHUT death plays its normal `Rules.BuildingDieSound` or
unit-typed `DieSound=`.

### 8.5 Summary

| Event | Audio | Frequency | Source |
|-------|-------|-----------|--------|
| Bridge weapon hit | Warhead `ReportSound=` only | Every shot | Generic weapon path |
| Bridge collapse animation | **NONE** | N/A | Verified absent |
| CABHUT C4 repair (SEAL/Tanya) | EVA voice + optional SFX | Per repair | `InfantryClass::PerCellProcess @ 0x519BC4` |
| CABHUT destruction (any cause) | Building's normal DieSound | Per destruction | Generic building path |

---

## 9. BlowUpBridge Anim Spawn — Corrected (Tiny Details)

`BlowUpBridge @ 0x47DD70` random debris/explosion logic, with the correct
RulesClass offset labels:

```
outer 95% gate:
  if (*(int*)(g_Rules + 0x168) > 0 AND rand01 < 0.95) {
    // position randomization
    rand_x = RandomRanged(0, 0x7FFFFFFE) >> something  // Math__ftol
    rand_y = RandomRanged(0, 0x7FFFFFFE) >> something
    pos    = cell.world_xyz + (rand_x, rand_y, z_jitter)

    // MetallicDebris spawn — 50% gate
    if (rand01 < 0.5) {
      idx       = RandomRanged(0, *(g_Rules + 0x14C) - 1)
      anim_type = (*(g_Rules + 0x140))[idx]              // MetallicDebris[idx]
      new AnimClass(anim_type, pos, loop_count=1)
    }

    // BridgeExplosions spawn — 100% (within outer 95%)
    idx       = RandomRanged(0, *(g_Rules + 0x168) - 1)
    anim_type = (*(g_Rules + 0x15C))[idx]                // BridgeExplosions[idx]
    frame     = RandomRanged(1, 5)
    new AnimClass(anim_type, pos, frame=frame)
  }
```

### Tiny-detail captures

- **Outer 95% gate** = `rand01 < 0.95`. So 1 in 20 BlowUpBridge invocations
  spawns NO debris and NO explosion. Player observable.
- **The MetallicDebris 50% gate is independent** of the BridgeExplosions
  spawn. So inside the outer 95%, you can get explosion-only (50%) or both
  (50%).
- **`BridgeExplosions` count gate (`+0x168 > 0`)** is the outer guard. If
  `BridgeExplosions=` is empty in INI, the entire block is skipped — no debris
  and no explosion. Retail rulesmd.ini ships with a populated list.
- **Frame start for BridgeExplosions** is `RandomRanged(1, 5)`, so any
  given explosion starts on one of the first 5 anim frames. **Iron-law:
  this is a per-spawn frame randomization — same anim type, different
  starting frame each invocation.**
- **`loop_count=1` on MetallicDebris**, no loop on BridgeExplosions. They're
  different anim classes.
- **Z-jitter** uses `(char)cell.Level * DAT_0089E7C0 + DAT_0089E7B4`. The
  exact constants weren't decoded but are theater-scale-dependent.
- **`g_IsMapEditor != 0`** skips the entire function. BlowUpBridge does
  nothing in editor mode.

---

## 10. INI Keys (Verified)

| Key | Section | RulesClass offset | Default in retail | Effect |
|-----|---------|-------------------|--------------------|--------|
| `BridgeStrength` | `[CombatDamage]` | `+0x1740` | 100 | RNG denominator: `RandomRanged(1, BridgeStrength) < damage` |
| `C4Warhead` | `[CombatDamage]` | `+0xFA8` | (warhead name) | Warhead passed to occupant Take_Damage in BlowUpBridge |
| `IonCannonWarhead` | `[CombatDamage]` | `+0xFF0` | (warhead name) | Bypasses RNG; triggers retry-3 loop |
| `CrushWarhead` | `[CombatDamage]` | `+0xFAC` | (warhead name) | Disables retry-3 loop (one attempt only) |
| `MetallicDebris` | `[General]` | `+0x140` (base) / `+0x14C` (size) | (list of anim types) | Random anim spawn in BlowUpBridge, 50% conditional gate |
| `BridgeExplosions` | `[General]` | `+0x15C` (base) / `+0x168` (size) | (list of anim types) | Random anim spawn in BlowUpBridge, outer 95% gate |
| `RepairBridgeSound` | `[AudioVisual]` | `+0x248` | **NOT SET** (-1) | Optional SFX on CABHUT C4 repair; never plays in stock YR |
| `CollapseChance` | `[CombatDamage]` | `+0x17CC` | 100 | **NOT a bridge mechanic** — cliff collapse RNG (% chance a cliff falls when hit). Adjacent terrain mechanic; flagged here because it appears next to BridgeStrength and is easy to confuse. |

**Per-CellClass state bytes** (from §3 + §7):

| Offset | Type | Meaning |
|--------|------|---------|
| `+0x2C` | `CellClass*` | Anchor back-pointer (set on non-anchor body cells; nulled on collapse) |
| `+0x44` | `int` | OverlayTypeIndex; `-1` = no overlay; written on partial-collapse |
| `+0x11A` | `u8` | Low-bridge shore/ramp stage counter (parallel small state machine) |
| `+0x11B` | `i8` | Cell height level (drives LOS; also `+= 4` per state transition) |
| `+0x11E` | `u8` | **Anchor state byte — 18 states (0–17)** |
| `+0x12C` | `u32` | Shroud bits: 0x08 = explored, 0x10 = visible (cleared by 120-frame poll on cells with `+0x140 & 0x20`) |
| `+0x140` | `u32` | Flags. Bit `0x80` = anchor; `0x100` = high-bridge; `0x200` = bridgehead; `0x400` = bridge destroyed marker; `0x800` = sentinel; `0x1000` / `0x10000` = state-driven; `0x20` = shroud-edge-dirty (mutated by 120-frame poll); `0x20000` = tube-anim spawned |

---

## 11. Integration Points / Tick Ordering

### Where bridge damage runs

1. **`Apply_area_damage @ 0x489280`** — invoked per cell in a weapon's area
   damage radius. Each cell with bridge-relevant overlay triggers a per-cell
   RNG roll + `ApplyDamageToCell`. This is in the combat phase of
   `advance_tick`.
2. **`ApplyDamageToCell @ 0x587180`** — adjacency dispatcher to
   `ProcessBridgeDamageStateMachine_High` or `_Low`.
3. **`ProcessBridgeDamageStateMachine_{High,Low}`** — state-byte transition,
   calls `UpdateRamp_*_DamageA/B_*` or `UpdateRamp_*_CollapseA/B_*`, which
   in turn call `BlowUpBridge` on individual cells. Same combat phase.
4. **`SetBridgeDirection_{NESW,NWSE}`** — invoked from collapse paths AND
   repair paths. Mutates cell flags including bit `0x80`. Same combat phase.
5. **`MapClass__UpdateBridgeEdgeTiles_{High,Low}`** — end-walker after the
   collapse cascade completes, called via `UpdateAdjacentBridges{,_High}`.
6. **`MapClass__RecalcBridgeShroudFlags @ 0x578100`** — runs every 120 frames
   from `LogicClass__PerTickUpdate @ 0x55AFB0`. **Independent of collapse
   events.** Always running.

### Tick-order parity for the Rust port

- Bridge damage must run in the combat/damage phase, BEFORE pathfinder grid
  rebuild that same tick, so units about to step onto a collapsed cell see
  the new state. Existing `bridge_orchestrator.rs` scaffolding already
  places it correctly.
- The 120-frame `RecalcBridgeShroudFlags` poll runs in the logic phase
  (`LogicClass__PerTickUpdate`), which is independent of the bridge damage
  cascade. Rust port should attach this to a periodic timer rather than
  inline into the bridge-state mutation path.
- `RadarClass__MarkTerrainDirty` calls are deferred-render concerns; no sim
  ordering implication.

---

## 12. Current Rust Implementation Status

| Subsystem | Status | Comments |
|-----------|--------|----------|
| `DestroyableBridges` outer gate (`SpecialFlags & 0x8000`) | Implemented (data) | `MapLoadFlags::destroyable_bridges` exists. **Verify the orchestrator actually checks it before per-cell rolls** — without this gate, weapons would damage bridges on maps where the original would not. |
| Warhead `Bridge=yes` (`+0x144`) inner gate | Verify | Each warhead's `Bridge=` flag must gate before the RNG roll. |
| Stateless RNG damage gate (`RandomRanged(1, BridgeStrength) < damage`) | Implemented | `sim/world/bridge_orchestrator.rs` — already uses this formula. Verify the `< damage` direction (NOT `<= damage`); the binary uses strict `<`. |
| 4-path branch (low-zone / high-zone / low-body / high-body) | Implemented | Verified to match per `PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md` |
| IonCannon bypass + 3 retries | Implemented | bridge_orchestrator.rs |
| **C4Warhead retry behavior — CORRECTED 2026-05-13** | Verify | Per Q3, C4 gets **exactly ONE RNG-gated attempt, NO retries**. Only IonCannon retries. If Rust currently retries C4 (as the prior version of this doc suggested), it's WRONG and must be changed. |
| CrushWarhead — no retries either | Verify | Same as C4 — exactly one attempt. Crush's distinction is in the self-damage policy at the top of `Apply_area_damage`, not in the bridge retry loop. |
| Per-branch screen-dirty rect dimensions | Likely not modeled | Low-bridge: `0x100 × 0x100` (256² px) centered with `-0x80` offset. High-bridge: `0xc0 × 0xc0` (192² px) with asymmetric `(-0x60, -0x18)` offset. |
| State byte `+0x11E` with 18-state ladder | Implemented as `DamageState` enum | Mapping verified; per-anchor state correct. |
| `_Low` vs `_High` separate constant tables (overlay base globals) | Partial | Rust uses one logical state machine. Should verify the absolute overlay frame IDs emit per low vs high. |
| `_Low` EW-collapse "anomaly" (calls `*_High` helpers) | **RESOLVED — no port action needed** | Per Q1: the four helper pairs are bit-identical; only the family-base global differs. Rust can use ONE parameterized helper. The binary's split is internal plumbing, not behavior. |
| `UpdateBridgeEdgeTiles` 30-cell scan + theater endpoint tiles | NOT implemented | Rust has no equivalent of the seam-walker. Bridge ends after damage will not visually fix up correctly. |
| Tail-recursion for multiple seams in same direction | NOT implemented | Multi-seam scenarios produce wrong-looking endpoints. |
| Bit `0x80` runtime mutation on collapse/repair | NOT implemented | The headline parity bug already named in parent report. Still open. |
| `RecalcBridgeShroudFlags` 120-frame poll | NOT implemented | Affects bridge-edge shroud caching. Not high-impact in single-frame terms; cumulative drift over a long match. |
| Cell shroud bits `+0x12C` (0x08 explored, 0x10 visible) | Implemented for normal cells; bridge poll path missing | The 120-frame clear-then-reveal pattern is the only bridge-specific shroud touch and is missing. |
| LOS through bridge cells via height (`+0x11B`) | Implemented via terrain height | Verify the height value updates on collapse — Rust currently treats `+0x11B` analog as map-load-static (same parity bug as bit 0x80). |
| `BlowUpBridge` debris/explosion anim spawn (MetallicDebris + BridgeExplosions) | Partial | Verify the 95% outer gate, the 50% MetallicDebris gate, the `RandomRanged(1,5)` frame start, and that the correct two RulesClass anim lists are used (the parent report had the labels swapped). |
| Bridge collapse audio | N/A — there is none | Confirm Rust does NOT play a bridge-specific cue. |
| CABHUT C4 repair audio (EVA voice + optional SFX) | NOT implemented (audio system pending) | When the audio path lands, route this via RadarEvent#14 → EVA_BridgeRepaired with optional RepairBridgeSound. |
| Global death-list (DAT_0087F8C0) | N/A — dead in binary | Do not implement. |

---

## 13. Open Questions — Follow-up Pass (2026-05-13)

### Q1 — `_Low` EW-collapse anomaly — **RESOLVED: harmless / intentional**

**Verdict:** the four `*_High` / `*_Low` damage-helper pairs are
**bit-identical**. `diff_functions` reports `similarity_score = 1.0`,
`body_equal = 82/82` (EW) or `80/80` (NS), zero added/removed instructions,
identical prologue/epilogue, identical strings, identical calls. The only
differences are TWO operand globals per pair:

| Helper-pair use | `*_High` | `*_Low` |
|-----------------|----------|---------|
| Overlay base subtracted from `cell+0x38` | `DAT_00aa0e28` (concrete) | `DAT_00abad1c` (wood) |
| Overlay base added in `SetOverlayAndPropagate` call | `DAT_00aa0e28` | `DAT_00abad1c` |

**Both globals are BSS-zero at static-image time** and populated at theater
load:
- `DAT_00abad1c` = base overlay frame index for the **wooden low bridge**
  overlay family
- `DAT_00aa0e28` = base overlay frame index for the **concrete high bridge**
  overlay family

The helpers exist as separate functions only because the family-base global
is bound at compile time per helper. They are functionally identical
helpers parameterized by their hardcoded base.

**The anomaly site is real but is NOT a bug.** `ProcessBridgeDamageStateMachine_Low`
internally branches on the cell's authored overlay base — `DAT_00abad30`
for low-bridge cells, `DAT_00aa1028` for high-bridge cells. When it reaches
the high-bridge EW-degrade branch, it correctly invokes the `*_High`
helpers because the cell's overlay belongs to the high-bridge family.
Calling `*_Low` helpers there would have been the actual bug (would
subtract the wrong base and write a frame from the wrong family).

**Helper-pair addresses (newly verified):**

| Helper | Address | Xrefs |
|--------|---------|-------|
| `UpdateRamp_EW_DamageA_High` | `0x00572B80` | 3 |
| `UpdateRamp_EW_DamageA_Low` | `0x0056F690` | 1 |
| `UpdateRamp_EW_DamageB_High` | `0x00572C90` | 3 |
| `UpdateRamp_EW_DamageB_Low` | `0x0056F7A0` | 1 |
| `UpdateRamp_NS_DamageA_High` | `0x00572230` | 2 |
| `UpdateRamp_NS_DamageA_Low` | `0x0056ED40` | 2 |
| `UpdateRamp_NS_DamageB_High` | `0x00572330` | 2 |
| `UpdateRamp_NS_DamageB_Low` | `0x0056EE40` | 2 |

**Rust port implication:** a single parameterized helper that takes the
cell's resolved overlay-base produces identical observable output. The
split is internal plumbing, not behavior — per CLAUDE.md's "internals
modernized, output preserved" rule.

### Q2 — Z-jitter constants `DAT_0089E7C0` / `DAT_0089E7B4` — **RESOLVED: theater-scale BSS globals**

Both globals are BSS-zero at static-image time and populated at theater load
(writer at `FUN_0047B240` and `FUN_0047B3A0` — the theater-init path).
Other read sites confirm their semantics:

- `DAT_0089E7C0` — read by `CellClass__RecalcAttributes @ 0x47D9C9`,
  `BlowUpBridge @ 0x47DE83`, `CellOverlay_TileDraw @ 0x4804DC`. Used as a
  per-level Z multiplier (`(char)cell.Level * DAT_0089E7C0`). This is the
  theater's height-per-level pixel scale (`ZGRAN`-style constant).
- `DAT_0089E7B4` — read by `BlowUpBridge @ 0x47DEA0`,
  `PlaceInfantryInCell @ 0x4814CB`. Used as the per-tile base Z offset
  (added after the per-level multiplication). The theater's base Z origin
  in pixels.

**Combined formula in BlowUpBridge:**

```
debris_z = (char)cell.Level * DAT_0089E7C0 + DAT_0089E7B4
```

So debris/explosion anim spawn position uses the theater's height-per-level
and base-Z constants. Different theaters (Temperate, Snow, Urban, NewUrban,
Lunar, Desert) load different values. **The exact values are not in the
binary** — they come from theater files (`.ini` / `.mix` content). The
binary just consumes them.

**Rust port implication:** the Z-jitter formula is straightforward to
reproduce IF the theater-loader pipeline already populates equivalent
per-theater constants. No new logic needed if those constants already exist
elsewhere (they're load-time data, not gameplay state).

### Q3 — Apply_area_damage retry semantics for C4Warhead — **RESOLVED: NO retries for C4 (CORRECTED)**

The previous version of this doc claimed C4Warhead participates in the
3-retry loop. **That was wrong.** Direct re-read of `Apply_area_damage @
0x489280` shows `bVar21` is reassigned just before the bridge sections:

```c
// Earlier (used in the top occupier loop for self-damage policy):
bVar21 = param_4 == *(int*)(g_RulesClass + 0xFAC);  // is-CrushWarhead

// Later (right before LAB_0048a0a5, used in the bridge retry loop):
bVar21 = param_4 != *(int*)(g_RulesClass + 0xFF0);  // is-NOT-IonCannon
```

In the bridge retry loop:

```c
cVar7 = ApplyDamageToCell();
iVar19 = 3;
while (cVar7 == '\0') {
  if ((bVar21) || (iVar19 < 1)) goto LAB_0048a049;  // bVar21 = !is_ion
  cVar7 = ApplyDamageToCell();
  iVar19 = iVar19 + -1;
}
```

So:
- `is_ion = true` → `bVar21 = false` → loop runs full 3 retries (4 total
  attempts).
- `is_ion = false` (everything else) → `bVar21 = true` → break
  immediately, exactly 1 attempt.

**C4Warhead, CrushWarhead, and every other `Bridge=yes` warhead get exactly
ONE RNG-gated attempt with no retries.** Only IonCannon retries.

§3 of this doc has been corrected with the full table; the previous
"C4Warhead also takes the retry path" claim is retracted.

### Q4 — `RecalcBridgeShroudFlags` cell iteration — **RESOLVED: full-map twice per call**

The poll at `0x578100` is a **two-pass full-map iteration**, not the
"single pre-filtered scan" suggested by the prior summary:

**Pass 1 — bridge-edge dirty reset:**
```c
iVar2 = MapClass__CellIterator_Next();
while (iVar2 != 0) {
  if ((*(byte*)(iVar2 + 0x140) & 0x20) != 0) {
    *(uint*)(iVar2 + 0x12C) &= 0xFFFFFFE7;  // clear shroud bits 0x08 + 0x10
    *(uint*)(iVar2 + 0x140) &= 0xFFFFFFDC;  // clear flags 0x20 + 0x02 + 0x01
    *(byte*)(iVar2 + 0x138) = 1;            // mark dirty
    FUN_006da7d0(iVar2);                    // re-submit for redraw
    if (*(char*)(iVar2 + 0x120) == -2)
      CellChangeNotify(iVar2, g_PlayerPtr, 1);
  }
  iVar2 = MapClass__CellIterator_Next();
}
```

**Pass 2 — edge-bitmask rebuild (every cell, not bridge-only):**
```c
piVar3 = MapClass__CellIterator_Next();
while (piVar3 != NULL) {
  // get cell's coord struct via vtable+0x48
  piVar4 = (int*)(**(code**)(*piVar3 + 0x48))(local_c);
  // shift+round to map-cell scale
  coord_shifted = ...;
  // recompute the edge bitmask
  cVar1 = Shroud_EdgeBitmask_Calculator(&coord, 0);
  if (cVar1 != (char)*(int*)((char*)piVar3 + 0x120)) {
    *(char*)((char*)piVar3 + 0x120) = cVar1;  // write new bitmask byte
    *(byte*)((char*)piVar3 + 0x138) = 1;       // mark dirty
    FUN_006da7d0(piVar3);
    if (cVar1 == -2)
      CellChangeNotify(piVar3, g_PlayerPtr, 1);
  }
  piVar3 = MapClass__CellIterator_Next();
}
```

**Cost:** **O(N) twice per 120-frame cycle** where N = full map cell count.
At 512×512 max that's ~524k cell-iterations per 120 sim frames. At
standard 15 sim FPS the poll runs every 8 seconds; at 60 FPS (max speed)
every 2 seconds.

**The iterator is generic (`MapClass__CellIterator_Next`).** It walks the
entire map area defined by `MapClass + 0xF4` (width) and the bounds set up
in the prologue (`+0x10C..+0x118`). No bridge pre-filter exists — pass 1
checks the `0x20` flag per-cell as it iterates, and pass 2 unconditionally
re-evaluates every cell's edge bitmask.

**Tiny-detail captures:**

- The shroud-bits clear `& 0xFFFFFFE7` drops `0x18` (bits 0x08 explored +
  0x10 visible) — same pattern as `RevealShroudFlags` but with the OR
  inverted to a clear.
- The flag clear `& 0xFFFFFFDC` drops `0x23` (0x20 + 0x02 + 0x01). Bit
  `0x20` is the "edge-dirty marker" the pass is processing; `0x01` and
  `0x02` are terrain block flags that may have been derived from the dirty
  state.
- `+0x138 = 1` is a per-cell "dirty for redraw" flag, set in both passes.
- `+0x120` is a single byte holding the cached edge bitmask, also used by
  `IsShrouded` and the reveal spiral.
- The `CellChangeNotify(..., g_PlayerPtr, 1)` calls only fire when
  `+0x120 == -2` — that's a sentinel meaning "force full re-evaluation."
  Doesn't fire on every cell.
- The shifted-coord math `(val + (val >> 31 & 0xFF)) >> 8` is a sign-aware
  divide-by-256 (negative values get a bias before the arithmetic shift).
  Converts world coords to map-cell coords.

**Rust port implication:** the 120-frame poll is genuinely O(map_area) per
invocation. For the Rust scale target of 30-player maps with potentially
larger maps, this is a real per-tick cost. Worth considering whether to
mirror the cost (and risk frame-time spikes every 8 seconds) or maintain
an indexed list of `0x20`-flagged cells for faster iteration. The behavior
must be reproduced; the data structure backing it can be redesigned.

### Q5 — `CollapseChance` cliff collapse mechanism — **OUT OF SCOPE** (unchanged)

Still out of scope. The `CollapseChance=100` key at `RulesClass + 0x17CC`
is a cliff-when-hit mechanic, not a bridge mechanic. Documented here only
to prevent confusion. Worth a separate `/re-investigate cliff-collapse-when-hit`
pass.

### New finding — `SpecialFlags & 0x8000` is the `DestroyableBridges` outer gate

Surfaced during Q3 verification. ALL bridge damage paths in
`Apply_area_damage` are gated by:

```c
if ((*g_ScenarioClass_Instance & 0x8000) == 0
    || *(char*)(param_4 + 0x144) == 0)
  goto LAB_0048a2c4;   // skip every bridge damage branch
```

`SpecialFlags & 0x8000` corresponds to the **`DestroyableBridges`** map INI
flag (consistent with the `destroyable_bridges` map-load override already
documented in `src/map/basic.rs`'s `MapLoadFlags`). When the map sets this
off, **no bridge damage of any kind fires** — IonCannon, C4, and regular
weapons all skip the entire bridge code path. The retail map default is on.

`param_4 + 0x144` is the WarheadTypeClass's `Bridge=yes` flag. Both
conditions must hold for the per-cell RNG roll to even be considered.

Rust port implication: the orchestrator must check the `DestroyableBridges`
flag before any per-cell roll, not just per-weapon. The existing
`MapLoadFlags::destroyable_bridges` is the right value to gate this on.

---

## 14. TS-Legacy Register (this report)

1. **Global death-list `DAT_0087F8C0..D0`** — dead code in retail YR. BSS-zero
   init, BlowUpBridge push always no-ops. No consumer. Safe to skip.
2. **Bit `0x2000` and `0x4000` on `CellClass+0x140`** (carried over from the
   parent report) — write-only in bridge code path, no readers anywhere.
3. **`RepairBridgeSound=` key** — INI-driven but rulesmd.ini does not set it,
   so the spatial SFX path at `0x519BFC` is dead in stock retail. Active only
   in mods that define the key.
4. **The `_Low` EW-collapse → `*_High` damage helpers anomaly** — flagged for
   verification; not confidently TS vs. live bug yet.

---

## 15. Sources

**Ghidra addresses decompiled (FULL):**
- `0x489280` — `Apply_area_damage`
- `0x47DD70` — `BlowUpBridge` (re-verified for offset labels)
- `0x571490` — `ProcessBridgeDamageStateMachine_Low`
- `0x576BA0` — `ProcessBridgeDamageStateMachine_High` (compared)
- `0x576200` — `UpdateBridgeEdgeTiles_High`
- `0x570AE0` — `UpdateBridgeEdgeTiles_Low` (412-of-412 instruction diff)
- `0x578100` — `RecalcBridgeShroudFlags`
- `0x66CD60` — `RulesClass__ReadCombatDamage`
- `0x519630` — `InfantryClass::PerCellProcess` (CABHUT C4 audio path)
- `0x47E040` — `SetBridgeDirection_NESW` (audio verification re-read)

**Ghidra addresses decompiled (LIGHT):**
- `0x4876F0` — `CellClass__RevealShroudFlags`
- `0x5673A0` — `MapClass__RevealShroud` (reveal spiral)
- `0x586360` — `IsShrouded`
- `0x5F4160` — `ObjectClass__DropIn` (vtable+0xEC resolution)
- `0x6551C0` — `RadarClass__MarkTerrainDirty`
- `0x65FA70` — `CreateRadarEvent`
- `0x752700` — `VoxClass__PlayEVA`
- `0x7509E0` — `VocClass__PlayAt`
- `0x55AFB0` — `LogicClass__PerTickUpdate` (poll site at `LAB_0055B29A`)

**Memory / string reads:**
- `0x0083ad90` — `"BridgeStrength"`
- `0x0083b1d4` — `"C4Warhead"`
- `0x0083aecc` — `"IonCannonWarhead"`
- `0x0083ad1c` — `"PlayerAutoCrush"` (adjacent)
- `0x0083accc` — `"CollapseChance"` (cliff, not bridge)
- `0x0083cedc` — `"BridgeExplosions"`
- `0x0083cef0` — `"MetallicDebris"`
- `0x0083a7fc` — `"RepairBridgeSound"`
- `0x008397D8` — `"Bridge Repaired Event"` (radar event)
- `0x00825538` — `"EVA_BridgeRepaired"`

**Xref enumerations:**
- `0x0087F8C0..D0` (global death-list — every reader/writer in the binary)
- `0x47DD70` (BlowUpBridge — every callee for audio verification)

**Docs referenced:**
- `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` (parent, corrects §3.7 offset
  labels and confirms most §3 findings)
- `BRIDGE_SYSTEM.md`
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- `PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md`
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`

**In-repo plans referenced:**
- `docs/plans/2026-05-12-bridge-mechanics-deferred-investigation-plan.md`
  (parent plan; Phase 1 success criteria met by `BRIDGE_DEFERRED_MECHANICS`;
  Phase 2/3 function inventory addressed by this report)

**INI files checked:**
- `ini/rulesmd.ini`:
  - line ~908: `CollapseChance=100` (cliff mechanic)
  - `[CombatDamage]` section keys verified for `BridgeStrength`, `C4Warhead`,
    `IonCannonWarhead`, `CrushWarhead`
  - `[General]` section verified for `MetallicDebris`, `BridgeExplosions`
  - `[AudioVisual]` section: `RepairBridgeSound` NOT defined

**Corrections to existing docs (deltas to apply):**

1. **`BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md §3.7`** — replace
   "`g_RulesClass+0xFA8 (BridgeBlast weapon)`" with
   "`g_RulesClass+0xFA8 = C4Warhead`". The string `"BridgeBlast"` does not
   exist in YR.
2. **`BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md §3.7`** — replace the
   "Random anim from `g_RulesClass+0x140` list" claim with "Random anim from
   `g_RulesClass+0x140` (`MetallicDebris`) list, with `+0x14C` = size".
   Note that `+0x15C` (size `+0x168`) is `BridgeExplosions`, NOT `+0x140`.
3. **`BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md §11.1`** — downgrade the
   "TS-era control flow worth flagging" caveat on the death-list to
   "TS-legacy, confirmed dead — no port action needed."
