# EBolt System — Ghidra Report

Electric bolts / lightning arcs used by the Tesla Coil, Tesla Tank, Tesla Trooper,
Shock Trooper, Eiffel Coil. Drawn as a recursive midpoint-displaced triple-line
segment between a source and target.

**IMPORTANT NOTE ON LABELS.** The Ghidra project contains a set of functions at
`0x00659110..0x00659FF0` labeled `EBolt__*`. **Those labels are wrong** — a
sibling research pass identified that entire range as the **RadBeam / Laser-draw**
system, and the `EBolt` RTTI string happens to match both because `RadBeam` and
`EBolt` use structurally similar class shapes. The real EBolt system lives in
the `0x004C1E10..0x004C2AFF` range. This report analyzes the **real** EBolt
system at those addresses; the stale labels at 0x006591xx are documented
separately and should be renamed to `RadBeam__*`.

## Top-line verdict

**LIVE IN YR.** Actively used by every tesla-family weapon in a standard
skirmish. No `SpecialFlags` gate; no TS-only caller chain.

- `IsElectricBolt=true` weapons fire in every YR match (`ElectricBolt`,
  `TankBolt`, `AssaultBolt`, `CoilBolt`, `OPCoilBolt`, `EiffelBolt`,
  `CRElectricBolt`, `ElectricBoltE` — `ini/rulesmd.ini:23856-24891`).
- Spawn: `TechnoClass__SpawnElectricBoltEffect @ 0x006FD570` is called from
  `TechnoClass__Fire_At @ 0x006FDD50` in the `weaponType+0x151` branch.
- Update/draw: **`FUN_004C2830`** (unnamed; proposed label `EBoltMgr__UpdateAndDrawAll`)
  is called from `TacticalClass_Draw @ 0x006D3D10` each render frame.
- Cleanup: **`FUN_004C29E0`** (proposed `EBoltMgr__ClearAll`) frees the vector
  on scenario teardown.

## Standalone vs embedded

**Standalone class.** EBolt is a small heap-allocated object. `FUN_006FD460`
calls `operator_new(0x30)` and `FUN_004C1E10` (ctor) then `FUN_004C2A60` (init).

- RTTI strings present: `.?AV?$VectorClass@PAVEBolt@@@@` @ `0x00820718` and
  `.?AV?$DynamicVectorClass@PAVEBolt@@@@` @ `0x00820740`.
- Instances live in a global `DynamicVectorClass<EBolt*>` whose header
  begins at **`0x008A0E88`** (allocator vtable at 0x008A0E88, storage pointer
  `0x008A0E8C`, `IsAllocated` byte `0x008A0E95`, capacity `0x008A0E90`,
  count `0x008A0E98`, growth step `0x008A0E9C`).
- No vtable on the EBolt object itself — ctor writes `*this = 0`, not a vtable
  pointer. EBolt is a POD-ish visual effect, driven entirely by the global
  update loop.

## Struct layout (0x30 / 48 bytes)

Ctor `FUN_004C1E10` writes fields `[0]..[10]` (as `int *` indexing), giving
12 × 4 = 48 bytes = **0x30**. This matches the `operator_new(0x30)` in the
spawner. `[0xB]` is the flag byte written at the end of `FUN_006FD460`.

| Offset | Size | Type         | Field                                   | Confidence | Source |
|-------:|-----:|--------------|-----------------------------------------|------------|--------|
| 0x00   | 12   | CoordStruct  | `StartCoord` (X,Y,Z world)              | high | Init args in `FUN_004C2A60` param_2/3/4 |
| 0x0C   | 12   | CoordStruct  | `EndCoord` (X,Y,Z world)                | high | Init args param_5/6/7 |
| 0x18   | 4    | int          | `ZAdjust` (depth bias from screen ΔY)   | high | Init arg param_8 |
| 0x1C   | 4    | int          | `RandomSeed` (RandomRanged(0, 0x100))   | high | Set in FUN_004C2A60 |
| 0x20   | 4    | void*        | `pAttachedSource` (TechnoClass*)        | high | Accessed in update loop `local_24[8]`; on expire, cleared from Techno+0x6DC |
| 0x24   | 4    | int          | `SourceFLHIndex` / anim token           | medium | Passed to `vtable+0xB0` in update loop |
| 0x28   | 4    | int          | `AgeFrames` (counts up)                 | high | `piVar2[7] = piVar2[7] + 1` per frame |
| 0x2C   | 1    | bool         | `IsAlternateColor` (copied from WeaponType+0x153) | high | Written by FUN_006FD460 at `unaff_EBX + 0x2c`; read in update loop as `(char)piVar2[0xb]` to pick palette row |
| 0x28   | 4    | int          | `Intensity` / life counter (field at `[10]`; shifted right each frame until 0) | high | `*piVar4 = *piVar4 >> 1;` each frame; removal condition `local_24[10] == 0` |

**Corrected field layout** (`[N]` = int-index × 4 bytes):

| Int idx | Byte offset | Field                                    |
|--------:|------------:|------------------------------------------|
| 0–2     | 0x00–0x08  | `EndCoord.X/Y/Z` (param_2/3/4 = *param_1/[1]/[2]) |
| 3–5     | 0x0C–0x14  | `StartCoord.X/Y/Z` (param_5/6/7 = [3]/[4]/[5]) |
| 6       | 0x18       | `ZAdjust` (param_8 = [6])                |
| 7       | 0x1C       | `AgeFrames` (incremented per frame)      |
| 8       | 0x20       | `pAttachedSource` (TechnoClass*, cleared on expiry via `[8]+0x6DC = 0`) |
| 9       | 0x24       | `SourceFLHIndex` (passed to source's `vtable+0xB0` to re-read origin each frame) |
| 10      | 0x28       | `Intensity` — initialized to 0x10000 by ctor; right-shifted once per frame; when zero, bolt expires |
| 0xB     | 0x2C       | `IsAlternateColor` (bool, low byte; copied from WeaponType+0x153) |

**Note on start/end ordering:** `FUN_004C2A60(startX, startY, startZ, endX, endY, endZ, zAdjust)`
stores start at `[0..2]` and end at `[3..5]`. The update loop in `FUN_004C2830`
then re-reads the start position from the attached techno every frame
(replacing `[0..2]`) — so `[0..2]` is the **source end of the arc** (the one
that moves with the firing unit) and `[3..5]` is the **fixed target end**.

The per-frame re-read in the update loop confirms: `*local_24 = *piVar4;
local_24[1] = iVar5; local_24[2] = iVar1;` where `piVar4` is the result of
`pAttachedSource->vtable+0xB0(GetFLH)`.

## Constructor

**`FUN_004C1E10 @ 0x004C1E10`** — the true EBolt constructor.
```
*this = 0;           // [0] start X
[1] = 0;             // start Y
[2] = 0;             // start Z
[3] = 0;             // end X
[4] = 0;             // end Y
[5] = 0;             // end Z
[6] = 0;             // ZAdjust
[7] = 0;             // AgeFrames
[8] = 0;             // pAttachedSource
[9] = 0;             // SourceFLHIndex
[10] = 0x10000;      // Intensity (65536 — see note below)
(byte)[0xB] = 0;     // IsAlternateColor
```

**Intensity initial value of `0x10000`** determines bolt lifetime: since it is
right-shifted one bit per frame and removal happens when it reaches 0, a bolt
lives **16 frames** (after which the shift yields 0). No INI tuning.

**`FUN_004C2A60 @ 0x004C2A60`** — the init-after-ctor routine (the "Create"
analog). Takes start XYZ, end XYZ, zAdjust (7 params). Writes coords, assigns
a random seed at `[7]`, and spawns a **ParticleSystemClass** of type
`g_RulesClass_Instance + 0x1020` (the "electric sparks" particle system
referenced by the bolt — this is what gives the arc its glowing sparkles).
It also inserts `this` into the global `DynamicVectorClass<EBolt*>` at
`0x008A0E8C`.

## Vtable

**None.** EBolt is not an AbstractClass descendant. No vtable pointer is
stored in the object. Lifecycle is owned by the global manager in
`FUN_004C2830`.

## Lifecycle

### Spawn

**`TechnoClass__SpawnElectricBoltEffect @ 0x006FD570`** is the single entry
point from the combat system. It is called by `TechnoClass__Fire_At` in the
`IsElectricBolt` branch (weapon+0x151).

It:
1. Gets firer's muzzle coord via `vtable+0x2E4`.
2. Gets target pointer via `vtable+0x3F8`.
3. Gets firer FLH via `vtable+0xB0`.
4. Calls **`FUN_006FD460`** — the actual EBolt spawn wrapper.
5. If firer type == 1 (building, e.g. Tesla Coil) and firer has `+0x24 != 0`
   and no active attached bolt at `+0x1B7`, stores the new bolt pointer there
   and calls `FUN_004C2BD0`. This is the Tesla Coil "persistent charge arc"
   hookup.

**`FUN_006FD460`** — EBolt spawn wrapper:
1. `pvVar3 = operator_new(0x30)` — allocate the 48-byte EBolt.
2. `iVar4 = FUN_004C1E10(pvVar3)` — run the ctor.
3. If firer's `vtable+0x2C` returns 6 (i.e. firer is a `BuildingClass`),
   compute a Z adjust by screen-projecting both endpoints and taking the
   negative-clamped ΔY. This is the "bolt lifts a bit to top of coil" adjust.
4. Call weapon target's `vtable+0x58` or `+0xA4` (depending on +0x14 & 2 flag,
   i.e. FootClass vs not) to get the target's coord.
5. **Copy the weapon's `IsAlternateColor` flag** (WeaponType+0x153) into the
   EBolt's byte offset `+0x2C`.
6. Call `FUN_004C2A60(startXYZ, endXYZ, zAdjust)` to init the bolt and insert
   into the global vector.

**Also used by `BulletClass__SpawnShrapnel @ 0x0046A310`** as a second caller
of `FUN_006FD460`. Shrapnel-on-hit electric bolts from shrapnel-warhead bullets.

### Update / Draw

**`FUN_004C2830`** (proposed label: `EBoltMgr__UpdateAndDrawAll`) is called
from `TacticalClass_Draw @ 0x006D3D10` each render frame. It:

1. Iterate the global vector (at `DAT_008A0E8C`, count `DAT_008A0E98`) from
   last to first.
2. For each EBolt:
   - If it has an attached source (`[8] != 0`), call the source's
     `vtable+0xB0(coord_out, [9], 0, 0, 0)` to **re-read the muzzle FLH** and
     overwrite the start coord at `[0..2]`. This is what makes the arc
     follow the shooter as it moves.
   - If `[10]` (intensity) is nonzero, project both endpoints to screen via
     `TacticalClass::CoordsToClient2`, clip against the tactical viewport
     (`FUN_007BC2B0`), and call **`FUN_004C1F20`** — the core bolt line drawer.
     Draw color parameter is `10` normally, `6` (= `10 & ~4`) when
     `IsAlternateColor` is set (`(-(uint)((char)piVar2[0xb] != '\0') & 0xFFFFFFFB) + 10`).
     Increment `AgeFrames`.
   - **`*piVar4 = *piVar4 >> 1;`** — halve the intensity each frame.
   - If `[10]` is now 0, remove from vector (swap-down compaction), clear
     the source techno's `+0x6DC` back-ref, and `operator delete` (`FUN_007C8B3D`).

### Expire

As above — once intensity right-shifts down to 0 (after 16 frames starting
from 0x10000), the bolt is deleted and its source techno's `+0x6DC` back-ref
is cleared so the techno can spawn a new one.

## Rendering path

**`FUN_004C1F20`** is the inner line drawer — this is the core of how an
electric bolt looks. Proposed label: `EBolt__DrawLine` or
`EBolt__DrawRecursiveBolt`. Parameters:

```
FUN_004c1f20(startX, startY, startZ,
             endX,   endY,   endZ,
             colorParam)   // 10 or 6
```

### Algorithm: recursive midpoint displacement

This is a **classic fractal lightning algorithm** (not the cosine/sinusoidal
sweep I mistakenly described earlier). Summary:

1. Compute `delta = start - end`, `length = ftol(sqrt(|delta|²))`.
2. If `length == 0`, return (zero-length bolt).
3. Initialize a stack of 8 segment records (arrays `local_2c8[]` and `local_2a4[]`).
4. **Subdivision loop** — while `length > 0x40` and stack depth < 8:
   - Compute midpoint M = (start + end) / 2 (component-wise, right-shifted 1).
   - On the first subdivision, prep per-subdivision cosine offsets
     `local_308[0..5]` via `Cos_lookup((seed * K) / (7 + i))` — a per-bolt
     deterministic but seed-varied angular offset.
   - **Add randomized jitter to M:**
     - If `length <= 0x80` (small remaining segment): M += Random(-1, 1) × jitter.
     - Else (longer segment): M += Random(-jitter, +jitter) for each component.
   - Generate **two child endpoints** M' and M'' displaced from M by
     Random(-jitter, +jitter) (possibly halved). These become the branch
     fork points.
   - Push the (M'', end, ...) frame onto the stack.
   - Recurse into (start, M, ...) with halved `length` and halved `jitter`.
5. At recursion base (segment too short to subdivide), **draw three line
   segments** via the primary-surface line primitive:
   - `start → mid`  (main trunk)
   - `mid   → end`  (main trunk)
   - forked-branch endpoint pair — gives the bolt its characteristic "Y-forks"
6. Pop the stack and repeat until empty.

**Palette path (key detail for the Rust port):**

```c
if (*(int *)(DAT_0087F6C4 + 4) == 1) {    // 8-bit surface
    colorIdx = (byte)*(DAT_0087F6C4+0x174 + colorParam);
} else {                                   // 16-bit surface
    colorIdx = (ushort)*(ushort *)(DAT_0087F6C4+0x174 + colorParam*2);
}
```

`DAT_0087F6C4` is the game's primary palette/surface descriptor. `+0x174` is
a remap/color LUT whose 10th byte (or 10th ushort for 16-bit) is the "tesla
white-blue" color, and the 6th (colorParam=6 when `IsAlternateColor`) is the
alternate color. For the third (fork-branch) draw call, the LUT index is
**hardcoded to 0xF** (byte) / `0x1E` (ushort) → a darker accent color for the
branch segments.

**Three distinct line-blits per bolt per frame:**
1. Primary trunk start → mid, color `colorParam` (10 or 6).
2. Primary trunk mid → end, color `colorParam`.
3. Fork/branch, color index 15 (hardcoded darker/accent).

All use `g_PrimarySurface->vtable[0x34]` (DirectDraw DrawLine_ZBuffered), with
Z-bias = `AdjustForZ(screenPoint) - 2`.

**Color data source:** `DAT_0087F6C4+0x174` is a **palette LUT inside the
DirectDraw primary surface descriptor**, not a rules.ini-controlled color. The
specific indices 10 and 6 and 15 are hardcoded in the binary.

### Jitter characteristics
- Subdivision depth: up to 8 levels.
- Jitter magnitude scales with remaining segment length (`(length * 0x17) >> 8`
  and `(length * 0x66) >> 8` — i.e. ~9% and ~40% of length).
- Randomness uses the engine RNG (`Random__RandomRanged`), so **bolt shape is
  deterministic given the same seed** — the `RandomSeed` field at +0x1C is the
  per-bolt seed (but note: the update re-draws the bolt each frame using the
  engine's current RNG state, not the cached seed; the seed influences the
  cosine offsets).

## INI keys

All controlled by a single WeaponTypeClass flag. No standalone `[EBolt]`
tuning section exists.

| Key                  | Struct          | Offset | Type | Default | Verified | Notes |
|----------------------|-----------------|-------:|------|--------:|----------|-------|
| `IsElectricBolt=`    | WeaponTypeClass | 0x151  | bool | false   | Confirmed (string at `0x008492E4`) | Enables the visual; causes Fire_At to call `SpawnElectricBoltEffect` |
| `IsAlternateColor=`  | WeaponTypeClass | 0x153  | bool | false   | Confirmed | Copied into EBolt+0x2C; selects palette LUT index 6 instead of 10 |
| `DrawBoltAsLaser=`   | WeaponTypeClass | 0x152  | bool | false   | Parse-only | **DEAD CODE.** Westwood removed the implementation — see `rulesmd.ini:23927-23929`. |
| `IsRadBeam=`         | WeaponTypeClass | 0x154  | bool | false   | Confirmed | Separate system (RadBeam at 0x006591xx); not an EBolt. |

No INI key controls bolt color, lifetime, jitter, fork count, or segment
subdivision depth. All hardcoded:
- **Lifetime** = 16 frames (intensity `0x10000` right-shifted to 0).
- **Subdivision** = up to 8 levels.
- **Jitter** = ~9% and ~40% of remaining segment length.
- **Colors** = primary-surface LUT indices 10 (normal), 6 (alt), 15 (fork accent).

**No `BoltColor` / `ElectricBoltColor` / `[AudioVisual]` tuning keys.** Verified
via `search_strings "BoltColor"` returning 0 hits.

**Particle system:** `FUN_004C2A60` spawns `g_RulesClass_Instance + 0x1020`
as the accompanying particle system. The RulesClass offset 0x1020 should map
to a string key in rules.ini (an AnimType or ParticleSystemType name) — worth
a follow-up `ReadINI` decompile on RulesClass to identify the exact INI key,
but this is the "sparks around the bolt" particle flavor.

## Call graph

```
TechnoClass::Fire_At                           @ 0x006FDD50
 └─[WeaponType+0x151 == IsElectricBolt]
    └── TechnoClass::SpawnElectricBoltEffect   @ 0x006FD570
        ├── vtable+0x2E4 (GetFiringCoord)
        ├── vtable+0x3F8 (GetTarget)
        ├── vtable+0xB0  (GetFLH)
        ├── FUN_006FD460                        @ 0x006FD460   (EBolt spawn wrapper)
        │   ├── operator_new(0x30)
        │   ├── FUN_004C1E10                    @ 0x004C1E10   (EBolt ctor)
        │   ├── [copy WeaponType+0x153 into EBolt+0x2C]
        │   └── FUN_004C2A60                    @ 0x004C2A60   (EBolt init + vector insert)
        │       ├── Random__RandomRanged(0, 0x100)
        │       └── ParticleSystemClass::Constructor (RulesClass+0x1020)
        └── [if firer is building] FUN_004C2BD0 (attach bolt to Techno+0x1B7)

BulletClass::SpawnShrapnel                     @ 0x0046A310
 └── FUN_006FD460                               (second spawn site — shrapnel bolts)

TacticalClass::Draw                            @ 0x006D3D10
 └── FUN_004C2830                               @ 0x004C2830   (update+draw all bolts)
     ├── [for each EBolt, reverse] re-read source FLH via vtable+0xB0
     ├── TacticalClass::CoordsToClient2
     ├── FUN_007BC2B0 (viewport clip)
     ├── FUN_004C1F20                           @ 0x004C1F20   (recursive midpoint bolt drawer)
     │   ├── Sqrt_Approx
     │   ├── Cos_lookup (per-bolt seed offsets)
     │   ├── Random__RandomRanged (jitter)
     │   └── g_PrimarySurface->vtable[0x34] × 3 per base segment (DrawLine_ZBuffered)
     ├── [intensity >> 1]
     └── [if intensity == 0] FUN_007C8B3D (operator delete), clear Techno+0x6DC

Scenario teardown
 └── FUN_004C29E0                               @ 0x004C29E0   (EBoltMgr::ClearAll)
     └── [free all pointers; zero the vector; drop allocation]
```

**Spawners (live-in-YR):**
- Tesla Coil firing `[CoilBolt]` / `[OPCoilBolt]`
- Tesla Tank firing `[TankBolt]`
- Tesla Trooper firing `[ElectricBolt]` / `[ElectricBoltE]`
- Tesla Trooper charging a Coil: `[AssaultBolt]` (uses `IsAlternateColor`)
- Shock Trooper firing `[CRElectricBolt]`
- Eiffel Coil firing `[EiffelBolt]`
- Shrapnel warheads (via `BulletClass::SpawnShrapnel` second caller)

**Updaters:** `TacticalClass::Draw` via `FUN_004C2830`.

**Drawers:** `FUN_004C1F20` (recursive) → `g_PrimarySurface->vtable[0x34]`.

## Tiberian Sun legacy check

Clean. Every tesla-family weapon listed above is live in a stock YR skirmish.
No `SpecialFlags` gate anywhere in the call chain. The `DrawBoltAsLaser=` flag
is explicitly commented as removed-code by Westwood in the stock INI.

## Open questions

1. **RulesClass+0x1020 particle system.** What INI key maps to this offset?
   Likely something like `ElectricSparksParticle=` or similar in `[Rules]`
   section — not yet traced in this pass.
2. **Palette LUT at `DAT_0087F6C4+0x174`.** This is the primary-surface
   color/remap table. Which palette is it (unittem.pal? game's blue-shift
   remap?) and is it overwritten by any rules.ini color? A quick RulesClass
   ReadINI scan would confirm.
3. **Label cleanup.** The 0x006591xx range is currently labeled `EBolt__*`
   but is actually `RadBeam__*` per the sibling research. The labels should
   be renamed. Conversely, the real EBolt functions at 0x004C1E10,
   0x004C1F20, 0x004C2830, 0x004C29E0, 0x004C2A60, 0x006FD460 are currently
   unnamed — they should get `EBolt__*` labels. **This pass does NOT apply
   those renames** to avoid colliding with another session's work; proposed
   names are listed below.
4. **`FUN_004C2BD0`** — the building-attach hookup called by
   `SpawnElectricBoltEffect`. Not decompiled; likely sets Techno+0x1B7 back-ref
   and marks the coil's "charging" state. Worth a follow-up.

## Proposed labels (not applied this pass)

To avoid stomping another session's labels, **no `save_program` was called.**
The following renames should be applied in a dedicated labeling pass:

| Current                  | Proposed                                 | Address     |
|--------------------------|------------------------------------------|-------------|
| (unnamed) FUN_004C1E10   | `EBolt__Constructor`                     | 0x004C1E10  |
| (unnamed) FUN_004C1F20   | `EBolt__DrawRecursiveBolt`               | 0x004C1F20  |
| (unnamed) FUN_004C2830   | `EBoltMgr__UpdateAndDrawAll`             | 0x004C2830  |
| (unnamed) FUN_004C29E0   | `EBoltMgr__ClearAll`                     | 0x004C29E0  |
| (unnamed) FUN_004C2A60   | `EBolt__Init`                            | 0x004C2A60  |
| (unnamed) FUN_004C2BD0   | `BuildingClass__AttachElectricBolt`      | 0x004C2BD0  |
| (unnamed) FUN_006FD460   | `TechnoClass__CreateElectricBolt`        | 0x006FD460  |
| `EBolt__*` (WRONG)       | `RadBeam__*` (per sibling agent)         | 0x00659110..0x00659FF0 |

**Existing correct labels used as references (not modified):**

| Address     | Label                                     |
|-------------|-------------------------------------------|
| 0x006FD570  | `TechnoClass__SpawnElectricBoltEffect`    |
| 0x006FDD50  | `TechnoClass__Fire_At`                    |
| 0x006D3D10  | `TacticalClass_Draw`                      |
| 0x0046A310  | `BulletClass__SpawnShrapnel`              |

## Confidence summary

- **Class identity + struct size (0x30):** High. `operator_new(0x30)` visible;
  ctor writes exactly 12 ints + 1 byte.
- **Field layout (StartCoord/EndCoord order, Intensity right-shift, etc):** High.
  All 11 fields cross-referenced in init, update loop, and expire path.
- **Lifetime = 16 frames via intensity `0x10000`:** High.
- **Recursive midpoint displacement render:** High. The subdivision loop and
  three-line-blit pattern are clear in `FUN_004C1F20`.
- **Color comes from primary-surface LUT indices 10 / 6 / 15:** High. The
  `IsAlternateColor` path (`& ~4`) is unambiguous.
- **INI key coverage:** High. Only `IsElectricBolt` and `IsAlternateColor` are
  read.
- **TS-legacy gating:** Clean — no gates.
- **Particle system companion at RulesClass+0x1020:** Medium — the offset is
  verified but its INI key name is not.

---

## Follow-up investigation (round 2, 2026-04-21)

### Q1 — RulesClass+0x1020 particle system INI key — **RESOLVED**

**Answer: `DefaultSparkSystem=` in `[CombatDamage]`.**

- String `"DefaultSparkSystem"` is at `0x0083AE80`.
- Xref: read at `0x0066CB4A` inside the newly-labeled
  `RulesClass__ReadCombatDamage @ 0x0066BBB0`.
- The reader calls `CCINIClass__ReadString`, then `FUN_00644890` (the
  ParticleSystemType-by-name lookup) and stores the resolved
  `ParticleSystemTypeClass*` into `param_1 + 0x1020` — exactly the offset the
  EBolt `Init` code reads.
- This is the `[CombatDamage]` section (string pointer
  `PTR_s_CombatDamage_007f0c84`), not `[General]` or `[AudioVisual]`.
- Stock YR `rulesmd.ini` [CombatDamage] `DefaultSparkSystem=Sparks`. That is
  the ParticleSystem spawned alongside every electric bolt.

New label applied: `FUN_0066bbb0` → `RulesClass__ReadCombatDamage`. (The
function reads ~60 keys from [CombatDamage] including `AmmoCrateDamage`,
`IonCannonDamage`, `Scorches`, `SplashList`, `DefaultSparkSystem`,
`DefaultLargeRedSmokeSystem`, `BerzerkAllowed`, `TurboBoost`, `AtomDamage`,
`BallisticScatter`, `BridgeStrength`, `Crush`, etc.)

### Q2 — `BuildingClass__AttachElectricBolt @ 0x004C2BD0` — **RESOLVED**

Decompile of `FUN_004C2BD0` (`param_1` = Building Techno, `param_2` = target
Techno, `param_3` = muzzle coord pointer):

```c
void __thiscall BuildingClass__AttachElectricBolt(
    TechnoClass* firingBldg, TechnoClass* target, Coord3D* muzzleCoord)
{
    if (target == NULL) return;
    if (target->vtable+0x2C() != 1)  return;  // target is a Building
    if (target[0x24] == 0)           return;  // target has "is-attached" flag
    if (target[0x81] != 0)           return;  // target is destroyed/disabled
    firingBldg[0x20] = target;                // Building+0x20 = attached target
    firingBldg[0x24] = muzzleCoord;           // Building+0x24 = stored muzzle
}
```

**Key correction to the original report:** The Techno+0x1B7 back-ref
(`activeAttachedBolt` → EBolt*) is NOT written by `AttachElectricBolt`. That
assignment happens in the caller `TechnoClass__SpawnElectricBoltEffect`
itself, BEFORE this function is called:
```c
if (iVar5 == 1 && firer[0x24] != 0 && firer[0x1b7] == 0) {
    firer[0x1b7] = iVar4;                     // store EBolt* back-ref
    BuildingClass__AttachElectricBolt(firer, uVar2);
}
```
So `AttachElectricBolt` only sets TWO fields: Building+0x20 (target pointer)
and Building+0x24 (muzzle coord). These appear to be the "persistent
target of my charge arc" fields used by Tesla Coil-family buildings when
charging via a Tesla Trooper (AssaultBolt). Offsets 0x20/0x24 are inside
the BuildingClass body (not TechnoClass base), consistent with
Tesla-Coil-specific state.

**Gating:** This function is ONLY called when the firer's
`vtable+0x2C == 1` (IsBuilding), so in practice only Tesla Coils and
Eiffel Coils (firer type == Building) invoke this path. Both Tesla Tanks
and Tesla Troopers fire `IsElectricBolt` weapons but skip this code
because they are not buildings.

**Teardown:** The Building+0x20 back-ref is cleared when the attached
target's `+0x81` goes non-zero (destroyed) — this is checked on every
`AttachElectricBolt` call and prevents re-attach. The EBolt* at Techno+0x1B7
(which IS on the firer itself, not via this function) is cleared by the
EBolt update loop when the bolt's intensity reaches 0 (documented in the
main report).

### Q3 — Palette LUT at `DAT_0087F6C4 + 0x174` — **RESOLVED**

`DAT_0087F6C4` is a **`ConvertClass*`**, not a DirectDraw surface
descriptor. ConvertClass is Westwood's palette-to-remap-table converter
(`ConvertClass__Constructor @ 0x0048E740`, object size 0x188, RTTI
`ConvertClass`). The class is 8-bit palette input → 8-bit or 16-bit
remapped output, and holds internal lookup tables.

**Source PAL file: PALETTE.PAL.** Traced via the CCFileClass init at
`0x0052BA60` (the bootstrap function mis-named `CCFileClass__Constructor`
in Ghidra that actually does the full asset load). The sequence is:

1. ConvertClass at `0x0052BE36` → `DAT_0087F6B8` — from TEMPERAT.PAL
   (string `0x008260C8`).
2. ConvertClass at `0x0052BF08` → `DAT_0087F6C0` — from ANIM.PAL
   (string `0x008260A0`).
3. ConvertClass at `0x0052BFBC` → **`DAT_0087F6C4` — from PALETTE.PAL**
   (string `0x00826094`, bytes `"PALETTE.PAL"`).
4. ConvertClass at `0x0052C070` → `DAT_0087F6B4` — from UNITSNO.PAL.
5. ConvertClass at `0x0052C124` → `DAT_0087F6B0` — from CAMEO.PAL
   (string `0x008204E0`).
6. ConvertClass at `0x0052C1D8` → `DAT_0087F6C8` — from MOUSEPAL.PAL.

Each `ConvertClass__Constructor` is called with `PUSH 0x885780, PUSH 0x35`
where `0x885780` is the scratch PAL buffer (re-loaded via
`LoadFileFromMIX` before each ctor call) and `0x35 = 53` is a flag
(shadow/darken-factor parameter).

**Field `+0x174` decoded:** `param_1[0x5D]` in the ConvertClass ctor.
The layout:
- `param_1[0x5C]` = base pointer to `(BPP × 256)` remap bytes (BPP = 1 for
  8-bit, 2 for 16-bit).
- `param_1[0x5D]` = `base + ((BPP - 1) >> 1) * 0x100` — i.e. the midpoint
  row of the "depth-step" ramp used by the blitter.

So `*(int *)(DAT_0087F6C4 + 0x174)` returns the "mid-brightness" row of
the PALETTE.PAL remap table, and `[+N]` indexes into it where N is the
requested palette index (10 = tesla-white-blue, 6 = tesla-alt-color,
15 = fork-accent color).

**Bottom line for the Rust port:**
- Load `PALETTE.PAL` (game-root, 256×RGB triples).
- Expand to 256×RGB565 (or RGBA) depending on target display depth.
- The "electric arc" colors are hardcoded indices `10`, `6`, `15` into
  that remapped table. No INI key controls them.
- In 16-bit mode (modern builds), the LUT is ushort-sized, so the index
  is `colorParam * 2` for offsets 10/6/15 which become byte offsets
  20/12/30 into the table — consistent with the
  `*(ushort*)(+0x174 + param_7*2)` path in `EBolt__DrawRecursiveBolt`.

### Notes on label changes

Only one new function labeled: `FUN_0066bbb0` → `RulesClass__ReadCombatDamage`.
`save_program` called at end of session.
