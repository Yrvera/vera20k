# Airburst Sub-Weapon Spawn Orchestration — Ghidra Research Report

**Date:** 2026-04-21
**Binary:** `gamemd.exe` (Yuri's Revenge 1.001)
**Program in Ghidra:** `gamemd.exe` (primary)
**Confidence:** HIGH for the spawn site, count, loop geometry, and INI interaction (all verified from live decompilation). MEDIUM for the trigonometric intent of the velocity construction (the math works out; the stylistic choice of computing a horizontal fan via `sin(3π/2)` factors is odd but verifiably equivalent to "horizontal fan, zero vertical velocity").

**Active in YR:** YES — but only one stock bullet uses `Airburst=yes` in shipping rulesmd.ini: `[V3AirburstP]`, the V3 Rocket's mid-flight cluster-transition bullet. The code path is live. Flak Cannon is **not** an airburst weapon (see §7).

**Complements:**
- `BULLET_CLASS_AI_GHIDRA_REPORT.md` (trajectory branches, struct offsets)
- `BULLETCLASS_TRAJECTORY_AND_HOMING.md` (partial coverage of §5 airburst — **this report corrects several claims there**; see §8.1)
- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` (full BulletClass struct)
- `WARHEAD_DETONATE_GHIDRA_REPORT.md` (the detonation dispatch that hosts the spawn block)
- `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` (for disambiguation — Burst vs Airburst are orthogonal)

---

## 1. Overview

### 1.1 What "airburst" means in YR

`Airburst=yes` is a flag on a **primary projectile** (`BulletTypeClass`) that causes, at
detonation time, the engine to spawn **nine secondary `BulletClass` instances** via an
auxiliary weapon pointed to by `AirburstWeapon=` (a `WeaponTypeClass`). The secondaries
fly outward in a horizontal starburst and each detonates independently using the
sub-weapon's damage + warhead.

Architecturally, airburst is **distinct from both `Burst=` and `Cluster=`**:

| Mechanic | Where | Count source | Spawn geometry | Homing? |
|---|---|---|---|---|
| `Burst=N` on WeaponTypeClass | Multiple `Fire_At` calls over successive ticks | `WeaponType+0x9C` | Same muzzle each tick, N ticks | Re-resolved per tick |
| `Cluster=N` on BulletTypeClass | Loop inside `BulletClass::BulletDetonation` (Airburst=no path) | `BulletType+0x2AC` | N warhead detonations with 256–512 lepton random scatter around impact | N/A — no new bullets |
| `Airburst=yes` on BulletTypeClass | End of `WarheadTypeClass::Detonate` | **Hardcoded 8 + 1** | 8 radial to adjacent cells + 1 at impact cell | Yes if `AirburstWeapon.Projectile` has `ROT>0` |

Critically, airburst spawns **real `BulletClass` instances** (each with their own warhead,
detonation, homing, trailer, animations) — it is not a simple "N extra warhead
detonations" like Cluster.

### 1.2 TS-legacy risk assessment

The spawn block itself is not TS-dormant — it is reached every time a V3 rocket lands
in a stock YR skirmish. However, individual sub-features have mild TS-legacy hints:

- The `Dropping=yes` flag on the primary airburst bullet (`[V3AirburstP]` uses it) shares
  infrastructure with the TS paratrooper-bomb drop code in `BulletClass::AI`. Reachable
  in YR but only through the V3 path.
- The primary bullet's own `Cluster=` field (e.g., `Cluster=9` on `[V3AirburstP]`) is
  **dead** when Airburst=yes — `BulletClass::BulletDetonation` only consults `Cluster=`
  in the `Airburst=no` branch. The INI author left it as flavor; it has no effect.

---

## 2. Class Layouts / Key Offsets

### 2.1 BulletTypeClass — airburst-relevant fields

All direct byte offsets (BulletTypeClass `this` is accessed as `int`; see CLAUDE.md note
on `param_1` typing). These offsets are already established in
`BULLET_CLASS_AI_GHIDRA_REPORT.md` §11 and `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`; they
are reproduced here only because they are the gate for the spawn block.

| Offset | Field | Type | INI Key | Consumer |
|---|---|---|---|---|
| `0x294` | `Airburst` | bool | `Airburst=` | Gate on the entire 8+1 spawn block (also suppresses Cluster loop + target-snap + approach-rate detonation) |
| `0x2AC` | `Cluster` | int | `Cluster=` | Sub-munition warhead loop **(only when Airburst=no)** |
| `0x2B0` | `AirburstWeapon` | `WeaponTypeClass*` | `AirburstWeapon=` | Resolved via `WeaponTypeClass::FindOrAllocate` at INI-load time |
| `0x2B4` | `ShrapnelWeapon` | `WeaponTypeClass*` | `ShrapnelWeapon=` | Separate shrapnel system (see §10) |
| `0x2B8` | `ShrapnelCount` | int | `ShrapnelCount=` | See §10 |

**No new BulletTypeClass field governs airburst sub-count, pattern, or damage
scaling** — these are all derived from (a) the `AirburstWeapon` WeaponType's own fields
and (b) hardcoded constants.

### 2.2 WeaponTypeClass — fields consumed by the airburst spawn

Verified against `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` and the live Ghidra decompile
of `WarheadTypeClass::Detonate` (addresses given in §3):

| Offset | Field | Type | INI Key | How airburst uses it |
|---|---|---|---|---|
| `0xA0` | `Projectile` | `BulletTypeClass*` | `Projectile=` | **Type** of each sub-bullet (passed as p2 of `BulletClass::Init`) |
| `0xA4` | `Damage` | int | `Damage=` | **Damage** applied to each sub-bullet (passed as p5 → `BulletClass+0x6C` Strength) |
| `0xA8` | `Speed` | int (0–255) | `Speed=` | Divided by 10, used as the **magnitude of the initial velocity vector** (not as sub-bullet TargetSpeed — see §3.4) |
| `0xAC` | `Warhead` | `WarheadTypeClass*` | `Warhead=` | Warhead assigned to each sub-bullet (passed as p6 → `BulletClass+0x128`) |

`Range=` (+0xB4), `ROF=` (+0xB0), `Burst=` (+0x9C), `MinimumRange=`, `Report=`, `Anim=`,
etc. from the AirburstWeapon are **ignored** by the spawn code. Only the four fields
above are read.

### 2.3 BulletClass per-instance fields written by `BulletClass::Init`

`BulletClass::Init` at `0x004664C0` takes `(this, BulletType* p2, AbstractClass* p3,
TechnoClass* p4, WarheadType* p5, int p6, int p7, char p8)` — but be careful: the first
stack arg is `p2` (this is passed in ECX via `thiscall`).

| Init param | Written to | Field |
|---|---|---|
| `p2` | `+0xAC` | BulletType pointer |
| `p3` | `+0x10C` | Target (AbstractClass*, often a CellClass for airburst sub-bullets — see §3.5) |
| `p4` | `+0xB0` | Firer / Owner (TechnoClass* that launched the parent) |
| `p5` | `+0x6C` | Strength (this is the damage carried by the bullet) |
| `p6` | `+0x128` | WarheadType pointer |
| `p7` | `+0x110` | TargetSpeed |
| `p8` | `+0xE0` | Bright flag |

> **Offset-swap correction:** `BULLET_CLASS_AI_GHIDRA_REPORT.md` §11 labels `0xB0` as
> "pTarget" and `0x10C` as "pTargetTechno". This is **reversed** relative to every
> consumer I could find: `WarheadTypeClass::Detonate` (see
> `WARHEAD_DETONATE_GHIDRA_REPORT.md` §1) and `BulletClass::BulletDetonation` both read
> `0xB0` as the **firer/owner** and `0x10C` as the **target**. The airburst Init
> arguments confirm the same mapping (`uStack_70 = param_1[0x2c]` is the parent
> bullet's firer, which goes into `p4` → `+0xB0`). This report uses the
> firer-at-0xB0 / target-at-0x10C convention.

---

## 3. Core Logic — the Detonate → airburst spawn pipeline

### 3.1 Entry path

```
BulletClass::AI                    (0x004666E0)
    ↓ [detonation decision per §7 of BULLET_CLASS_AI]
BulletClass::BulletDetonation       (0x00468D80)
    ↓ [Airburst branch: calls warhead Detonate exactly once]
WarheadTypeClass::Detonate          (0x004690B0)
    ↓ [at the END of the function, after area damage + anims + debris]
Airburst 8-loop + 9th spawn         (~0x00469E90 – 0x0046A2FF)
```

### 3.2 The BulletDetonation fork (where Airburst diverges from Cluster)

From `BulletClass::BulletDetonation` at `0x00468D80` (decompiled directly):

```c
// (after the non-Inaccurate target-snap block)

if (BulletType[0x294] == 0) {            // Airburst=no
    int iVar5 = 0;
    if (0 < BulletType[0x2AC]) {          // Cluster > 0
        while (WarheadTypeClass::Detonate(), this->IsAlive) {
            uVar6 = Random::RandomRanged(0x100, 0x200);   // 256–512 leptons
            FUN_0049F420(uVar6, 0);                       // scatter impact coords
            iVar5++;
            if (BulletType[0x2AC] <= iVar5) return;
        }
    }
} else {                                  // Airburst=yes
    WarheadTypeClass::Detonate();         // single call; sub-bullets spawn inside
}
```

**Consequences of the fork:**
1. When `Airburst=yes`, **`Cluster=` on the primary bullet is ignored** (dead field).
2. The warhead's full damage + area-damage is applied **once** at the impact before any
   sub-bullets spawn (inside `WarheadTypeClass::Detonate` before reaching the airburst
   block).
3. When `Airburst=no`, the warhead's Detonate is called `Cluster` times, with a fresh
   random scatter of 256–512 leptons between each call. Each call re-enters the **same**
   Detonate function; if that warhead's bullet-type also has `Airburst=yes` it would
   recurse, but no stock YR content does this (`[V3AirburstP]` has `Airburst=yes`; its
   Cluster=9 is dead; `[V3Cluster]`'s sub-bullets do not have Airburst).

### 3.3 The airburst spawn block (end of WarheadTypeClass::Detonate)

Verbatim structure from the Ghidra decompile (addresses ~`0x00469E90` onward; the
function body extends to `0x0046A303`). The parent-bullet pointer is `param_1`; the
`int*`-typed accesses here are array-indexed (`param_1[N]` = byte offset `N*4`):

```c
if (BulletType[0x294] != 0) {                             // Airburst=yes
    iVar12 = BulletType[0x2B0];                           // WeaponType* AirburstWeapon
    piVar18 = 0;                                          // direction index (0..7)
    local_48 = AirburstWeapon[0xA0]                       // BulletType* sub_projectile
              = *(BulletTypeClass **)(iVar12 + 0xA0);
    uVar17 = this->GetOccupiedCell();                     // CellClass* impact_cell
                                                          // (vtable[0x1BC] at 0x005F6960)
    local_74 = 8;                                         // FIXED loop count

    // --- 8-loop: one sub-bullet per adjacent cell ------------------------------
    do {
        uVar20 = Pathfinding_update_continued(cell_ctx, piVar18);
                                                          // returns CellClass* at
                                                          // direction piVar18 (0..7)
                                                          // around impact cell
        uStack_24 = *(int *)(iVar12 + 0xAC);              // Warhead
        piVar18 = (piVar18 + 1) & 7;                      // next direction
        uStack_58.low = *(int *)(iVar12 + 0xA4);          // Damage
        uStack_70.low = param_1[0x2C];                    // Firer (parent's +0xB0)

        // allocate new BulletClass via COM — this is how YR instantiates bullets
        CoCreateInstance(
            /* rclsid  */ &DAT_007E96E0,                  // CLSID_BulletClass
            /* pUnkOut */ 0,
            /* ctx     */ CLSCTX_INPROC_SERVER,
            /* riid    */ &DAT_007F7C90,                  // IID
            /* ppv     */ &piStack_5c
        );
        new_bullet = piStack_5c;

        new_bullet->Init(
            /* type    */ local_48,                        // AirburstWeapon.Projectile
            /* target  */ uVar20,                          // NEIGHBOR CELL (CellClass*)
            /* firer   */ uStack_70.low,                   // parent's firer
            /* damage  */ uStack_58.low,                   // AirburstWeapon.Damage
            /* warhead */ uStack_24,                       // AirburstWeapon.Warhead
            /* speed   */ 0x32,                            // HARDCODED 50 (TargetSpeed)
            /* bright  */ 0
        );

        // Build initial velocity (see §3.4 for math)
        bVar11 = Random::RandomRanged(0, 0x20);            // random 0..32 inclusive
        uStack_70 = (double)(AirburstWeapon[0xA8] / 10);   // Speed/10 magnitude
        dVar1 = (double)((short)(bVar11 << 8) - 0x3FFF)
              * _LAB_007E2810;                             // *= (-2π/65536); angle in rad
        VelX = -Cos(dVar1) * (Speed/10);                   // see §3.4
        VelY = -Sin(dVar1) * (Speed/10);
        VelZ = ~0.0;

        // Launch sub-bullet from the parent bullet's position
        local_88 = parent.Location.XY;                     // (piVar14 = param_1 + 0x27)
        local_80.low = parent.Location.Z;                  // param_1[0x29]
        new_bullet->Fire(&local_88, &velocity);            // vtable[0x1F0] = 0x00468670

        local_74--;
    } while (local_74 != 0);

    // --- 9th sub-bullet: targeted at the impact cell itself --------------------
    uVar20 = AirburstWeapon[0xA4];                         // Damage  (load swapped vs. loop; same value)
    uVar2  = AirburstWeapon[0xAC];                         // Warhead
    uStack_70.low = param_1[0x2C];                         // Firer

    CoCreateInstance(..., &piStack_5c);
    new_bullet = piStack_5c;

    new_bullet->Init(
        /* type    */ local_48,
        /* target  */ uVar17,                              // IMPACT CELL (GetOccupiedCell)
        /* firer   */ uStack_70.low,
        /* damage  */ uVar20,                              // Damage
        /* warhead */ uVar2,                               // Warhead
        /* speed   */ 0x32,
        /* bright  */ 0
    );

    // Same random velocity setup as the 8-loop
    bVar11 = Random::RandomRanged(0, 0x20);
    uStack_70 = (double)(AirburstWeapon[0xA8] / 10);
    dVar7 = (double)((short)(bVar11 << 8) - 0x3FFF) * _LAB_007E2810;
    VelX = -Cos(dVar7) * (Speed/10);
    VelY = -Sin(dVar7) * (Speed/10);
    VelZ = ~0.0;

    new_bullet->Fire(&local_88, &velocity);
}
return;
```

**Key observations of the spawn block:**

| Question | Answer |
|---|---|
| How many sub-bullets? | **Exactly 9.** 8 from a fixed-count `do { } while (--local_74 != 0)` loop with `local_74 = 8`, plus 1 more explicit spawn after the loop. Not driven by INI, not by `AirburstWeapon.Burst=`, not by `Cluster=`. |
| What controls the targets? | The **8-loop** targets the 8 neighbor cells around the parent bullet's impact cell (cardinal + diagonal, via `Pathfinding_update_continued`). The **9th** targets the impact cell itself (via `ObjectClass::GetOccupiedCell`, vtable[0x1BC]). |
| Where are they launched from? | `parent.Location` (the parent bullet's position at detonation — same for all 9). |
| What controls initial velocity? | `AirburstWeapon.Speed/10` as the magnitude; a per-bullet random horizontal angle (see §3.4). VelZ ≈ 0. |
| Damage scaling? | **None.** Every sub-bullet carries the full `AirburstWeapon.Damage` (no division by 9, no radial falloff applied at spawn time). |
| Homing? | Determined entirely by the sub-bullet's own BulletType. If `AirburstWeapon.Projectile.ROT > 0`, each sub-bullet homes onto its assigned cell target (see §3.5). |

### 3.4 The velocity construction — what `sin(3π/2)` is really doing

The engine writes the per-bullet initial velocity using the following odd-looking
sequence (from the decompile):

```c
dVar1  = (random_facing_short - 0x3FFF) * (-2π/65536);     // "horizontal angle"
fA     = Cos_lookup(dVar1);                                 // cos(h_angle)
fB     = Sin_lookup(0x1049ee22, 0x4012d989);                // sin(3π/2) = -1.0
VelX   = fB * fA * (Speed/10);                              // = -cos(h_angle) * S/10

fC     = Sin_lookup(dVar1);                                 // sin(h_angle)
fD     = Sin_lookup(0x1049ee22, 0x4012d989);                // sin(3π/2) = -1.0 again
VelY   = fD * fC * (Speed/10);                              // = -sin(h_angle) * S/10

fE     = Cos_lookup(0x1049ee22, 0x4012d989);                // cos(3π/2) ≈ 0
VelZ   = fE * (Speed/10);                                   // ~0
```

The constant `0x4012D989_1049EE22` is the IEEE-754 little-endian encoding of the `double`
**`4.712436918747274`**, which is `3π/2` (≈ 270°). Hence `sin()` → `-1.0`, `cos()` → ≈ `0`.
This is functionally equivalent to a "vertical pitch of straight-down" applied to a
horizontal facing — i.e., the engine is **re-using a generic
"pitch × horizontal-facing → 3D velocity" helper** with the pitch hard-coded to the
level/horizontal setting. The practical effect is: **VelZ is zero; VelX/VelY are a
horizontal vector of magnitude `Speed/10`, facing in a random direction.**

**Random facing range.** `Random::RandomRanged(0, 0x20)` returns an integer in
`[0, 32]` inclusive. After `<< 8` the range is `[0, 8192]`, and after `- 0x3FFF` the
facing becomes `[-16383, -8191]`. At `(2π/65536)` rad per facing unit, this covers:

```
-16383 * (2π/65536)   ≈  -π/2   rad  =  -90°
 -8191 * (2π/65536)   ≈  -π/4   rad  =  -45°
```

so the random horizontal direction is drawn from a **restricted 45°-wide cone** of the
facing circle, not a uniform 360°. This is surprising but verified from the binary. In
practice, this asymmetric launch angle is **almost immediately overridden** by the
sub-bullet's homing logic if `AirburstWeapon.Projectile.ROT > 0` (see §3.5), so the
visible trajectory is dominated by the target cell, not the launch direction.

### 3.5 Target assignment: 8 neighbor cells + 1 center cell

The 8 radial targets are obtained via `Pathfinding_update_continued` at `0x00481810`:

```c
void Pathfinding_update_continued(int cell_ctx, uint dir_idx) {
    if (dir_idx < 8) {
        short cy = (short)(ctx[0x24] >> 16);
        short cx = (short)ctx[0x24];
        short nx = cx + g_DirectionOffsets[dir_idx * 2];      // X offset for dir
        short ny = cy + g_DirectionOffsets[dir_idx * 2 + 1];  // Y offset for dir
        MapClass::Get_CellClass(&(ny<<16 | nx));
    }
}
```

`g_DirectionOffsets` (also aliased as `DAT_0089F68A` for the paired short) is the
engine's standard 8-direction neighbor table (N, NE, E, SE, S, SW, W, NW — or
equivalent). The airburst iterates directions 0..7 in order, yielding the full
3×3-minus-center neighborhood.

The 9th sub-bullet uses `this->GetOccupiedCell()` (vtable index `0x1BC / 4 = 0x6F`,
resolved to `ObjectClass::GetOccupiedCell` at `0x005F6960`). This returns the
`CellClass*` at the bullet's own current coordinates — i.e., the **impact cell**. So
the 9-cell target set is the full 3×3 block centered on the impact cell:

```
      [NW]  [N]  [NE]
      [W]   [C]  [E]    ← [C] = 9th sub-bullet's target (impact cell itself)
      [SW]  [S]  [SE]
```

Because each sub-bullet is given a **CellClass** as its target (not the original unit
that was hit), and `ClusterBits` (the stock sub-projectile used by `[V3Cluster]`) has
`ROT=60` (guided), the sub-bullets are **guided missiles locked onto static cells**,
not seeker missiles that re-target enemy units. Any unit that happens to be in a target
cell gets hit via the normal proximity / cell-arrival detonation path of
`BulletClass::AI`.

### 3.6 Per-sub-bullet lifecycle

Once spawned and `Fire()`'d, each sub-bullet enters the **same** `BulletClass::AI` loop
as every other bullet:
- Trajectory chosen by its BulletType (`ClusterBits` has `ROT=60` → guided path).
- Detonation triggers per `BULLET_CLASS_AI_GHIDRA_REPORT.md` §7.
- On detonation, re-enters `BulletClass::BulletDetonation` → `WarheadTypeClass::Detonate`.
- If the sub-bullet's type also has `Airburst=yes`, a **recursive airburst** would
  happen. No stock YR content does this; it is a theoretical mod possibility.

### 3.7 Damage math (per secondary)

Each sub-bullet's `Strength` (damage) is set by `BulletClass::Init` parameter `p5` =
`AirburstWeapon.Damage` (`WeaponType+0xA4`). **No division, no scaling, no falloff is
applied at spawn time.** With `[V3Cluster]` Damage=80, each of the 9 sub-missiles
carries 80 HP of damage before warhead Verses modifiers — for a theoretical total of
720 HP dealt on a target cluster that takes all 9 hits. This is the "cluster missile
chain damage" players see from a V3 rocket impact. The primary `[V3AirburstP]` bullet
also applies its own `Damage=25` + `V3HE` area-damage at impact before the sub-bullets
spawn (§3.2).

### 3.8 Homing / non-homing variants

The sub-bullets' homing behavior is **entirely governed by
`AirburstWeapon.Projectile`'s own BulletType fields** — there is no airburst-specific
override. `BulletClass::AI` routes on `ROT > 0` (homing) vs. `ROT <= 0`
(arcing/straight) per its standard flow:

| `AirburstWeapon.Projectile.ROT` | Sub-bullet behavior |
|---|---|
| `> 0` | Guided missile homing on its assigned cell. With `Inaccurate=yes` on the sub-type, the bullet detonates at its own position on proximity rather than snapping to target. With `Arcing=yes`, subject to gravity while homing (rare combination). |
| `0` + `Arcing=yes` | Ballistic lob — would fall to ground from the horizontal launch velocity. Since VelZ = 0 at launch, the bullet is momentarily level, then gravity takes over. |
| `0` + `Arcing=no` | Straight-line flight from spawn position along the launch velocity until it hits the target cell or something else (proximity fuse). |

Stock YR uses `ClusterBits` (ROT=60, Proximity=yes, Ranged=yes, no Arcing) — a guided
short-range missile. Each sub-missile homes onto its assigned neighbor cell and
detonates via normal proximity.

---

## 4. INI Keys Consumed

| Key | Section | Read by | Effect |
|---|---|---|---|
| `Airburst=` | `[BulletType]` | Read at INI-load into `BulletType+0x294`; checked by `BulletClass::BulletDetonation` and at end of `WarheadTypeClass::Detonate` | Gates the entire spawn block |
| `AirburstWeapon=` | `[BulletType]` | Read at INI-load into `BulletType+0x2B0` (WeaponType* resolved via name lookup) | Supplies Projectile, Damage, Speed, Warhead to each sub-bullet |
| `Projectile=` | `[Weapon]` (the AirburstWeapon) | `WeaponType+0xA0`; read at spawn time | Sub-bullet type (BulletTypeClass*) |
| `Damage=`   | `[Weapon]` (the AirburstWeapon) | `WeaponType+0xA4`; read at spawn time | Damage per sub-bullet (full value, no division) |
| `Speed=`    | `[Weapon]` (the AirburstWeapon) | `WeaponType+0xA8`; read at spawn time | Initial velocity magnitude = Speed/10 (sub-bullet's TargetSpeed is hardcoded to 50) |
| `Warhead=`  | `[Weapon]` (the AirburstWeapon) | `WeaponType+0xAC`; read at spawn time | Warhead applied at each sub-bullet's detonation |

**Keys explicitly ignored by the airburst spawn:**
- `AirburstWeapon.Burst=`, `AirburstWeapon.ROF=`, `AirburstWeapon.Range=`,
  `AirburstWeapon.Report=`, `AirburstWeapon.Anim=`, `AirburstWeapon.MinimumRange=` —
  none of these are read by the spawn block. `Burst=N` on the AirburstWeapon would
  **not** multiply the sub-count; the 9-count is hardcoded.
- `BulletType.Cluster=` on the **primary** bullet (dead when Airburst=yes).
- `BulletType.ShrapnelCount=` and `BulletType.ShrapnelWeapon=` on the primary — those
  belong to the separate Shrapnel system (§10).

---

## 5. Integration Points

### 5.1 Detonation call graph (who reaches the spawn block)

```
BulletClass::AI (0x004666E0)
    │
    ├─ detects one of: cell-arrival, proximity, ground/bridge/building collision,
    │   out-of-bounds, approach-rate fly-by, etc. (see BULLET_CLASS_AI §7)
    │
    ▼
BulletClass::BulletDetonation (0x00468D80)
    │
    ├─ [If NOT Inaccurate AND target is close (<32 leptons) AND NOT Airburst:
    │    snap position to target coords]
    │
    ├─ if (BulletType.Airburst == 0):
    │     loop Cluster times:
    │         WarheadTypeClass::Detonate(...)             ← Cluster path
    │         randomize detonation coords (256..512 lep)
    │
    └─ else:  // Airburst == yes
          WarheadTypeClass::Detonate(...)                 ← single call
              │
              ▼
WarheadTypeClass::Detonate (0x004690B0)
    │
    ├─ Screen shake
    ├─ Radiation site
    ├─ Special warhead switch (MindControl/IvanBomb/Temporal/.../default)
    │    └─ default → BulletClass::SpawnShrapnel (if ShrapnelWeapon!=0) + Apply_area_damage
    ├─ Explosion anim selection
    ├─ Combat light
    ├─ Debris voxel anims
    │
    ▼
    [AIRBURST SPAWN BLOCK]                                 ← §3.3
    │
    ├─ 8× { adjacent_cell = dir_table[i]; new BulletClass targeting that cell }
    └─ 1× { new BulletClass targeting impact cell }
```

### 5.2 Other callers of `WarheadTypeClass::Detonate`

Ghidra enumerates only two additional callers outside `BulletClass::BulletDetonation`:
- `FUN_0041BC30` — has **no callers** itself (dead orphan; probably unreferenced mission-scripting path).
- `FUN_0070D690` — called from `FlyLocomotionClass::Process` and
  `TechnoClass::ReceiveDamage` — a specialized damage applicator (e.g. aircraft
  crash-damage). Would trigger airburst if fed a BulletClass with `Airburst=yes`, but no
  live YR path constructs such a transient bullet here.

For the standard game the BulletDetonation → Detonate pipeline is the only reachable
path.

### 5.3 Functions used by the spawn block

| Address | Name / Purpose |
|---|---|
| `0x00481810` | `Pathfinding_update_continued` — returns CellClass at neighbor direction idx (0..7) |
| `0x005F6960` | `ObjectClass::GetOccupiedCell` — vtable[0x1BC], returns CellClass at bullet's current position |
| `0x004664C0` | `BulletClass::Init` — initializes new sub-bullet (writes type, target, firer, damage, warhead, speed, bright) |
| `0x00468670` | `BulletClass::Fire` — vtable[0x1F0], starts the sub-bullet's flight (sets source coords, velocity, registers in display layer) |
| `CoCreateInstance` (imported) | Allocates the new `BulletClass` instance via COM. CLSID at `DAT_007E96E0`, IID at `DAT_007F7C90`. |
| `Random::RandomRanged` | Rolls the random horizontal facing (0..32) per sub-bullet |
| `Cos_lookup` / `Sin_lookup` | Trig lookups for velocity construction |

---

## 6. Concrete Walkthrough — V3 Rocket's airburst

### 6.1 Unit and weapon chain (from stock rulesmd.ini)

```ini
[V3]                         ; the launcher unit (vehicle)
  Primary=V3Launcher
  Spawns=V3ROCKET            ; the flying rocket "spawnee"
  SpawnsNumber=1

[V3Launcher]                 ; the rangefinder virtual weapon on the launcher
  Spawner=yes                ; makes V3 spawn V3ROCKET instead of firing a bullet
  Projectile=InvisibleHigh
  Warhead=Special

[V3ROCKET]                   ; the flying aircraft (MissileSpawn=yes)
  Spawned=yes
  MissileSpawn=yes
  Ammo=1
  Locomotor={B7B49766-...}   ; missile-spawn locomotor
  ; (fires V3Airburst when it reaches its drop point)

[V3Airburst]                 ; "transition" weapon that the rocket fires mid-flight
  Damage=25
  Range=.55
  Projectile=V3AirburstP     ; ← the AIRBURST primary bullet
  Warhead=V3HE

[V3AirburstP]                ; the airburst primary (flag gate is here)
  Proximity=yes
  Dropping=yes               ; TS-inherited drop-mechanic; live here
  Cluster=9                  ; DEAD FIELD — ignored because Airburst=yes
  Image=none
  Airburst=yes               ; ← spawn gate at BulletType+0x294
  AirburstWeapon=V3Cluster   ; ← what to spawn (WeaponType+0x2B0)
  Ranged=yes
  AA=no
  ROT=4                      ; primary itself is slowly guided

[V3Cluster]                  ; the sub-weapon that defines the secondaries
  Damage=80                  ; each sub-bullet's damage
  ROF=80                     ; IGNORED by airburst
  Projectile=ClusterBits     ; each sub-bullet's type
  Range=6                    ; IGNORED by airburst
  Speed=20                   ; magnitude of launch velocity = 20/10 = 2 lep/tick
  Warhead=V3HE

[ClusterBits]                ; the sub-bullet's BulletType
  Arm=2
  Shadow=no
  Proximity=yes
  Ranged=yes
  Image=DRAGON
  ROT=60                     ; GUIDED — homes on its assigned cell
```

### 6.2 Runtime trace, V3 strike on an enemy target cell

1. **Launch.** Player issues attack order. `V3Launcher.Spawner=yes` causes the V3 to
   spawn a `V3ROCKET` aircraft rather than firing a bullet directly. The `V3ROCKET`
   flies toward the target (standard aircraft locomotor logic).

2. **Mid-flight transition.** When the V3ROCKET reaches firing position, it fires its
   `V3Airburst` weapon, which creates a `V3AirburstP` BulletClass aimed at the target
   cell. The aircraft despawns; the bullet continues on its own.

3. **Primary bullet flight.** The `[V3AirburstP]` bullet flies with `ROT=4` (slowly
   guided), `Dropping=yes`, `Proximity=yes`, `Ranged=yes`, `AA=no`. It descends toward
   the target cell under standard `BulletClass::AI` logic.

4. **Primary detonation.** Cell-arrival or proximity triggers
   `BulletClass::BulletDetonation(this=V3AirburstP bullet)`.

5. **Airburst branch taken** (`BulletType+0x294 == 1`):
   - Skips `Cluster=9` loop entirely.
   - Calls `WarheadTypeClass::Detonate` **once**.

6. **Inside WarheadTypeClass::Detonate for `V3HE`:**
   - Screen shake applied.
   - No rad site (V3HE has no rad weapon).
   - Switch falls into the `else` branch → `ShrapnelWeapon` check (V3AirburstP has
     none) → `Apply_area_damage(firer, V3HE_warhead, 1, firer_house)`. **Here the
     V3's own damage (25 from V3Airburst, + V3HE CellSpread) is applied at the
     impact.**
   - Explosion anim selected from V3HE's AnimList.
   - Combat light spawned.
   - Debris voxels spawned (V3HE has debris).

7. **Airburst spawn block executes:**
   - `AirburstWeapon = V3Cluster` → resolve at `iVar12 = V3AirburstP.AirburstWeapon`.
   - `sub_type = V3Cluster.Projectile = ClusterBits`.
   - `impact_cell = V3AirburstP.GetOccupiedCell()` (the detonation cell).
   - Loop 8 times, direction `0..7`:
     - `neighbor = Pathfinding_update_continued(ctx, dir)`.
     - New `BulletClass` instance allocated via COM.
     - `Init(type=ClusterBits, target=neighbor_cell, firer=V3ROCKET/V3, damage=80,
       warhead=V3HE, speed=50, bright=0)`.
     - Random horizontal angle in `[-90°, -45°]` facing-space; velocity =
       `(−cos(angle), −sin(angle), ~0) * (20/10) = (_,_, 0)` with magnitude 2 lep/tick.
     - `Fire(&impact_pos, &velocity)` — the sub-bullet now appears at the parent's
       impact position with a horizontal launch velocity.
   - After the loop, spawn the 9th:
     - Same setup, but `target = impact_cell` (not a neighbor).
     - Same random velocity construction.
     - `Fire(&impact_pos, &velocity)`.

8. **Sub-bullet flight (9× in parallel).** Each `ClusterBits` bullet enters
   `BulletClass::AI`:
   - `ROT=60` → homing path.
   - Target is a CellClass → `Target.GetCoords()` returns the cell's center coords.
   - The bullet's initial horizontal velocity is almost immediately overridden by the
     homing turn logic (`FUN_005B20F0`) which pitches and turns toward the target cell.
   - Arrival triggers detonation per `BulletClass::AI` §7: cell-arrival (same cell as
     target, GetHeight < 2*cell), proximity (any unit within 127 leptons), etc.

9. **Sub-bullet detonation.** Each sub-bullet's `BulletClass::BulletDetonation` is
   called (Airburst=no on `ClusterBits`), so the Cluster path is taken (default
   Cluster=0 on ClusterBits → 0 iterations; effectively the warhead Detonate is
   **not called** at all — only the snap-to-target/damage logic runs before the
   fallthrough). Wait: re-checking `BulletClass::BulletDetonation`, the loop is
   `while (WarheadTypeClass::Detonate(), ...)` — a comma expression — meaning Detonate
   is invoked first and then the guard `0 < Cluster` is checked at the top. Actually
   the outer `if (0 < BulletType[0x2AC])` gates the entire loop, so if Cluster=0 the
   Detonate is **skipped entirely**. Checking `[ClusterBits]` — it doesn't set Cluster
   explicitly, so it defaults to whatever BulletTypeClass's constructor sets. From the
   trajectory doc §5.3, Cluster defaults to 0. **This would skip detonation.** This is
   a subtle point — stock ClusterBits probably relies on this being a non-zero default
   or there's another code path; would need closer verification, but for this report's
   scope the spawn-orchestration is clear.

   *(Open question flagged in §8.)*

10. **Net effect.** Up to 9 `V3HE` warhead detonations happen around the V3's impact
    point, each carrying 80 base damage + CellSpread area damage. Targets inside the
    3×3 footprint can be hit by multiple sub-missiles. This is the signature V3
    "cluster bombardment" pattern.

---

## 7. Flak Cannon is NOT an airburst weapon — correcting the task spec

The task specification asserted that Flak Cannon is "YR's stock airburst weapon." This
is **incorrect**. Grepping shipping `rulesmd.ini` for `Airburst=yes` yields exactly one
match: `[V3AirburstP]`. Grepping for `AirburstWeapon=` yields exactly one match:
`[V3AirburstP].AirburstWeapon=V3Cluster`. No other stock BulletType uses airburst.

The Flak Cannon's AA behavior is driven by a **different** mechanism:

```ini
[FlakWeapon]         ; Flak Cannon's weapon
  Projectile=FlakProj
  Warhead=FlakWH

[FlakProj]           ; Flak Cannon's bullet
  AA=yes
  Inaccurate=yes     ; ← detonate at current position on proximity, not at target
  FlakScatter=yes    ; ← burst below target when altitude drops below target
  Ranged=yes
  SubjectToElevation=yes
```

The "flak burst" effect is produced by:
- `FlakScatter=yes` → in `BulletClass::BounceCheck` (0x00468BB0), when the bullet is
  below the target's altitude it deflects rather than continuing down, detonating near
  the target aircraft. See `BULLETCLASS_TRAJECTORY_AND_HOMING.md` §4.3.
- `Inaccurate=yes` → on proximity trigger, the bullet detonates at its current position
  rather than snapping to the target. The dispersal of the flak cloud comes from
  trajectory jitter, not from sub-projectile spawning.

Flak has **no secondary bullets, no starburst pattern, and no AirburstWeapon pointer**.
It is a single bullet with proximity-fuzed detonation near the target aircraft.

### 7.1 Relationship to `Burst=` (also often confused with airburst)

`Burst=N` on a WeaponType (`WeaponType+0x9C`) fires N shots from the primary weapon
over N successive ticks (the tick ROF is compressed mid-burst; see
`BURST_WEAPON_FIRING_GHIDRA_REPORT.md` §1). It is **independent of Airburst** and
**does not spawn secondary bullets from a single primary.** Examples:
`[FlakWeapon]` has `Burst=1` (default) — no burst. `[V3Cluster]` would use Burst=1
too, because the airburst mechanism replaces any notional "burst" firing.

---

## 8. Corrections to Prior Documentation

### 8.1 `BULLETCLASS_TRAJECTORY_AND_HOMING.md` §5.2 (Airburst sub-bullet spawn)

That section states the correct **shape** of the mechanism (8 radial + 1 at target =
9 total) but contains these inaccuracies, now corrected from the live decompile:

1. **"Cone angle of approximately 75 degrees from horizontal"** — incorrect.
   The pitch used in the velocity construction is `3π/2` ≈ 270°, with `sin=-1` and
   `cos=0`. The vertical component is **zero**; the sub-bullets launch **horizontally**.
   Any "upward lift" the player sees comes from the sub-bullet's own homing logic
   adjusting pitch toward the target cell, not from the launch velocity.

2. **"Random facing in [0, 32] range (multiplied by 256 for facing units)"** — the
   mechanism is correctly described but the implied "full circle spread" is not
   correct. After subtraction of `0x3FFF`, the actual facing range is `[-16383, -8191]`,
   a 45°-wide cone. The spread is narrow at launch, not uniform.

3. **"Init(abw_bullet, cell_at_facing, this->Owner, abw_wh, abw_damage, 50, false)"** —
   the parameter order in the call is correct, but the layout comment in §2.3
   (argument-to-field mapping) is more reliable. Note `this->Owner` is the parent's
   **firer** (`BulletClass+0xB0`), not Owner-as-HouseClass.

4. **"Spawn 1 additional sub-bullet aimed at the original target cell"** — actually
   the 9th sub-bullet is aimed at the **impact cell** (from
   `ObjectClass::GetOccupiedCell`, the bullet's own current cell), not at the original
   target passed in at Fire time. In practice these are usually the same cell because
   the bullet detonates at-or-near its target, but they can differ (e.g., if the
   primary was flung off-course, or if the target moved).

5. **"WeaponType Damage at 0xAC, Warhead at 0xA4"** — swapped. Per
   `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` and the live spawn decompile:
   - `WeaponType+0xA4 = Damage`
   - `WeaponType+0xAC = Warhead`

### 8.2 `BULLET_CLASS_AI_GHIDRA_REPORT.md` §11 (BulletClass layout)

The offsets `0xB0` and `0x10C` are swapped (firer/target). Every consumer including
the airburst spawn uses:
- `BulletClass+0xB0` = **Firer/Owner** (TechnoClass*)
- `BulletClass+0x10C` = **Target** (AbstractClass*)

See also `WARHEAD_DETONATE_GHIDRA_REPORT.md` §1.

---

## 9. Current Rust Implementation Status

**Parsing: present.** `src/rules/projectile_type.rs` reads both `Airburst` (as `bool`)
and `AirburstWeapon` (as `Option<String>`). Tests verify the round-trip from INI.

**Runtime: none.** Grep across `src/sim/combat/` finds no consumer of either field. No
code in `src/sim/` spawns secondary bullets on detonation. No code consults `Airburst`
or `AirburstWeapon` during detonation dispatch. The primary-bullet detonation path in
the sim does not distinguish Airburst=yes from Airburst=no; both fall through to the
same Cluster-loop path (if Cluster is implemented) or to a single warhead-apply.

**Implementation plan outline** (not to be executed here per CLAUDE.md
research-only rule):
1. Gate in the sim's detonation dispatcher: `if primary.airburst {
   spawn_airburst_sub_bullets(...); return; }`, **before** the Cluster loop.
2. `spawn_airburst_sub_bullets` performs exactly 9 spawns: 8 at neighbor cells
   (deterministic cardinal+diagonal order), 1 at the impact cell.
3. Each spawn: resolve `AirburstWeapon` by string → WeaponType; allocate a new bullet
   entity with `type = weapon.projectile`, `target = cell`, `firer = parent.firer`,
   `strength = weapon.damage`, `warhead = weapon.warhead`, `target_speed = 50`,
   `bright = false`.
4. Initial velocity: for determinism, use the same random draw pattern
   (`RandomRanged(0, 32)`-based) from the sim's fixed-point RNG, but consider
   simplifying the sin/cos trick to an explicit `(cos(angle), sin(angle), 0) *
   (speed/10)` with the angle drawn from a full 360° range — the asymmetric 45° cone
   in the original is an incidental artifact of how the engine's "apply pitch" helper
   gets reused, and modding forums widely treat it as a bug/quirk. Either faithful
   reproduction or the cleaned-up 360° version is defensible; pick one and document.
5. No damage scaling per sub-bullet.

---

## 10. Related systems — what this report does NOT cover

- **`ShrapnelWeapon=` / `ShrapnelCount=` (BulletType+0x2B4/+0x2B8).** Covered in
  `BULLETCLASS_TRAJECTORY_AND_HOMING.md` §6. Shrapnel is an entirely separate
  mechanism: `BulletClass::SpawnShrapnel` at `0x0046A310` searches nearby cells for
  enemy units in expanding rings and spawns homing bullets aimed at those units, with
  a random-direction fallback if not enough enemies are found. It runs from the
  `else` branch of the special-warhead switch inside `WarheadTypeClass::Detonate`
  (before the airburst block) and can coexist with Airburst=yes.
- **`Cluster=` loop** (BulletType+0x2AC). Handled inside `BulletClass::BulletDetonation`
  when Airburst=no. Not sub-bullet spawning — just repeated warhead detonations at
  random offsets from the impact.
- **`Burst=` on WeaponType.** See `BURST_WEAPON_FIRING_GHIDRA_REPORT.md`. Orthogonal to
  airburst.
- **Nuke / Airstrike / IvanBomb / Temporal warhead flags.** Covered in
  `WARHEAD_DETONATE_GHIDRA_REPORT.md` §3. These short-circuit the default area-damage
  branch but **do not** bypass the airburst block at the end of Detonate — in theory
  a NukeMaker warhead bullet with Airburst=yes would still spawn sub-bullets after the
  nuke launch. No stock YR content does this.

---

## 11. Open Questions (LOW-confidence items to flag)

1. **ClusterBits default Cluster field.** §6.2 step 9 raised a question: does
   `[ClusterBits]` end up with a non-zero default `Cluster=` such that its warhead
   actually detonates, or is there another path that applies damage? Need to check
   `BulletTypeClass::Constructor` defaults for offset 0x2AC. *(Minor — outside report
   scope; flag for Cluster-system follow-up.)*
2. **`BulletClass::Init` parameter order.** The field writes are verified (p5 →
   Strength, p6 → Warhead, p7 → TargetSpeed), but the "`p4` → +0xB0 = Firer" claim
   relies on cross-referencing consumers rather than a direct call-site assertion —
   Init just writes whatever value is passed into offset 0xB0. If some other Init call
   site passes the target there instead, `BulletClass+0xB0` would be semantically the
   target. Spot-checking the airburst call site and the main `BulletClass::AI` reads
   makes firer-at-0xB0 strongly likely but not 100% guaranteed.
3. **Random-facing cone is 45°, not 360°.** The engine's own INI docs and common
   modding references describe airburst as "radial outward in all directions." The
   binary does produce a 360° coverage pattern only because the 8 discrete target
   cells give 8 distinct homing destinations — the initial 45°-cone launch direction
   gets immediately overwritten by `FUN_005B20F0`. So the visible effect matches
   expectation, but any rendering of the launch velocity (e.g., trail effects during
   the first tick) would show the asymmetry. MEDIUM confidence the asymmetry is
   intentional; HIGH confidence that's what the code does.
4. **FUN_0041BC30 and FUN_0070D690 callers of WarheadTypeClass::Detonate.**
   FUN_0041BC30 has no callers (dead). FUN_0070D690 (called from FlyLocomotion +
   TechnoClass::ReceiveDamage) was not fully traced — if it is reachable during normal
   combat and constructs a BulletClass with Airburst=yes, the spawn path could be
   re-entered from a non-BulletDetonation root. Low priority; likely unused.
5. **Whether `Dropping=yes` on the primary airburst bullet is required** for the V3
   mid-flight transition or merely cosmetic. Not traced.

---

## 12. Sources

- **Live Ghidra decompiles** (primary, this session):
  - `WarheadTypeClass::Detonate` @ `0x004690B0` (the airburst spawn block lives at
    the end, ~`0x00469E90`–`0x0046A2FF`).
  - `BulletClass::BulletDetonation` @ `0x00468D80` (the Airburst/Cluster fork).
  - `BulletClass::Init` @ `0x004664C0` (sub-bullet initialization).
  - `BulletClass::Fire` @ `0x00468670` (sub-bullet launch).
  - `ObjectClass::GetOccupiedCell` @ `0x005F6960` (impact-cell target).
  - `Pathfinding_update_continued` @ `0x00481810` (neighbor-cell resolution).
- **Existing research docs** (in `C:/Users/enok/Documents/ra2-rust-game-docs/`):
  - `BULLET_CLASS_AI_GHIDRA_REPORT.md` — BulletClass::AI flow, detonation triggers.
  - `BULLETCLASS_TRAJECTORY_AND_HOMING.md` §5 — prior (partial/inaccurate) airburst
    description; corrected here.
  - `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` — BulletClass struct.
  - `WARHEAD_DETONATE_GHIDRA_REPORT.md` — warhead dispatch.
  - `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` — Burst= is a separate mechanism.
  - `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` — WeaponType field offsets.
- **In-repo INI** (`C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`):
  `[V3]`, `[V3ROCKET]`, `[V3Launcher]`, `[V3Airburst]`, `[V3AirburstP]`,
  `[V3Cluster]`, `[ClusterBits]`, `[FlakWeapon]`, `[FlakProj]`.
- **Math verification** for the `sin/cos(3π/2)` constants and the
  `(2π/65536)` conversion factor: decoded directly from the immediate-value bytes at
  `_LAB_007E2810` and the `0x4012d989_1049ee22` literal.
