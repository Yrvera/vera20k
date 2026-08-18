# Airburst — Sub-Weapon Spawn at Detonation

This doc is the canonical reference for the **airburst mechanism** in gamemd.exe:
the sub-projectile spawn block at the end of `WarheadTypeClass::Detonate` that fires
when a primary bullet's `Airburst=yes` flag is set.

The mechanism produces **exactly 9 sub-bullets** in a 3×3 cell footprint around the
impact point. Used by the V3 Rocket's cluster detonation. Architecturally distinct
from `Burst=` (per-tick weapon shots) and `Cluster=` (repeat-warhead-detonations);
see §2.

Out-of-scope:
- The per-target damage transform → [`damage_formula.md`](damage_formula.md)
- Sub-bullet flight after spawn → [`bullet_lifecycle.md`](bullet_lifecycle.md), [`projectile_rot_homing.md`](projectile_rot_homing.md)
- Warhead Detonate dispatch (the parent function) → [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md)
- Shrapnel (separate mechanism, different field, different dispatch) → [`bullet_lifecycle.md`](bullet_lifecycle.md) §6 (when written)
- Cluster=N loop → [`damage_formula.md`](damage_formula.md) §5 cross-reference; this doc covers it briefly in §2

---

## 1. Function identity

| Field | Value |
|---|---|
| Containing function | `WarheadTypeClass::Detonate` at `0x004690B0` |
| Spawn block location | end of function, ~`0x00469E90 – 0x0046A2FF` |
| Upstream caller | `BulletClass::BulletDetonation` at `0x00468D80` (single call site in normal play) |

### Confidence

- **Content: HIGH** — Live decomp of `BulletClass::BulletDetonation 0x00468D80` (2026-05-17) confirms the Airburst gate (`BulletType+0x294`) and the dual-path fork (Cluster loop vs single Detonate call).
- **Identity: HIGH** — single string match for `"Airburst"` and `"AirburstWeapon"`, both into `BulletTypeClass::ReadINI`.
- **Binding: HIGH** — BulletDetonation is the only live caller path for combat-time airburst; the spawn block is reached every time a V3 rocket detonates in a vanilla skirmish.

---

## 2. Airburst vs Burst vs Cluster

| Mechanic | Where | Count source | Spawn geometry | Homing? |
|---|---|---|---|---|
| `Burst=N` on WeaponTypeClass | Multiple `Fire_At` calls over N ticks | `WeaponType+0x9C` | Same muzzle, same target, N successive ticks (3–5 frame inter-shot delay) | Re-resolved per tick |
| `Cluster=N` on BulletTypeClass | Loop inside `BulletClass::BulletDetonation` (Airburst=no path) | `BulletType+0x2AC` | N warhead detonations with 256–512 lepton random scatter around impact (no new bullets) | N/A |
| `Airburst=yes` on BulletTypeClass | End of `WarheadTypeClass::Detonate` | **Hardcoded 8 + 1** | 8 sub-bullets at neighbor cells + 1 at impact cell | Yes if `AirburstWeapon.Projectile.ROT > 0` |

**Critical:** Airburst spawns **real `BulletClass` instances**, each with their own
warhead, detonation, homing, trailer, animations. Cluster doesn't — it just re-runs
the warhead detonation at random offsets. Burst doesn't — it fires the same weapon
N times.

**Cluster is DEAD when Airburst=yes.** `BulletClass::BulletDetonation` gates the
Cluster loop on `Airburst==0`, so `[V3AirburstP].Cluster=9` is a no-op (left in for
flavor by the original INI authors).

---

## 3. Field offsets (verified)

### BulletTypeClass

| Offset | Field | Type | INI key | Effect |
|---|---|---|---|---|
| `+0x294` | `Airburst` | bool | `Airburst=` | Gates the entire 9-spawn block |
| `+0x2AC` | `Cluster` | int | `Cluster=` | Sub-detonation loop (only when Airburst=no) |
| `+0x2B0` | `AirburstWeapon` | `WeaponTypeClass*` | `AirburstWeapon=` | Source of sub-bullet type / damage / speed / warhead |
| `+0x2B4` | `ShrapnelWeapon` | `WeaponTypeClass*` | `ShrapnelWeapon=` | Separate shrapnel system (see [`bullet_lifecycle.md`](bullet_lifecycle.md)) |
| `+0x2B8` | `ShrapnelCount` | int | `ShrapnelCount=` | Shrapnel count |

### WeaponTypeClass (fields consumed by the airburst spawn)

| Offset | Field | INI key | Usage |
|---|---|---|---|
| `+0xA0` | `Projectile` | `Projectile=` | Type of each sub-bullet (`p2` of `BulletClass::Init`) |
| `+0xA4` | `Damage` | `Damage=` | Damage per sub-bullet (`p5` of Init, written to `Bullet+0x6C` Strength) |
| `+0xA8` | `Speed` | `Speed=` | Velocity magnitude: `Speed/10` leptons/tick |
| `+0xAC` | `Warhead` | `Warhead=` | Warhead per sub-bullet (`p6` of Init, written to `Bullet+0x128`) |

### Ignored fields on AirburstWeapon

`Burst=`, `ROF=`, `Range=`, `MinimumRange=`, `Report=`, `Anim=`, `Inaccurate=`,
`FlakScatter=` on the AirburstWeapon are **not read** by the spawn block. Only the
four fields above (Projectile, Damage, Speed, Warhead) matter.

### Confidence

- **Content: HIGH** — all offsets cross-verified against [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md) and [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md).
- **Identity: HIGH** — single INI key per field.
- **Binding: HIGH** — each field has a single live consumer in the spawn block.

---

## 4. The dispatch fork in `BulletClass::BulletDetonation` (verified live)

`BulletClass::BulletDetonation` at `0x00468D80`, decompiled live 2026-05-17:

```c
// After pre-impact damage (the Inaccurate-gated re-snap block from accuracy_inaccurate.md)
if (BulletType[0x294] == 0) {                 // Airburst=no
    int i = 0;
    if (0 < BulletType[0x2AC]) {              // Cluster > 0
        while (WarheadTypeClass::Detonate(), this->IsAlive) {
            r = Random::RandomRanged(0x100, 0x200);   // 256–512 leptons
            FUN_0049f420(r, 0);                       // randomize impact coords
            i++;
            if (BulletType[0x2AC] <= i) return;
        }
    }
} else {                                       // Airburst=yes
    WarheadTypeClass::Detonate();             // single call; sub-bullets spawn inside
}
```

So Airburst-yes calls Detonate **once**, and the spawn-9-sub-bullets block runs at
the end of that Detonate.

---

## 5. The 9-spawn block at end of `WarheadTypeClass::Detonate`

Verbatim structure (Ghidra decomp at addresses `~0x00469E90` onward, parent bullet
pointer = `param_1`, all `[N]` indices are `int*` so byte offset = `N*4`):

```c
if (BulletType[0x294] != 0) {                                 // Airburst=yes
    iVar12 = BulletType[0x2B0];                               // WeaponType* AirburstWeapon
    sub_type = AirburstWeapon[0xA0]/4;                        // BulletType* sub_projectile
    impact_cell = this->GetOccupiedCell();                    // vtable[0x1BC] @ 0x005F6960

    // ── 8-loop: one sub-bullet per adjacent cell ──
    int dir = 0;
    int counter = 8;
    do {
        neighbor = Pathfinding_update_continued(cell_ctx, dir);   // 0x00481810
        damage  = AirburstWeapon[0xA4];                            // Damage
        warhead = AirburstWeapon[0xAC];                            // Warhead
        firer   = parent[0x2C];                                    // parent.Firer = parent+0xB0
        dir = (dir + 1) & 7;                                       // next direction

        CoCreateInstance(CLSID_BulletClass=DAT_007E96E0, ...);     // allocate new bullet
        new_bullet->Init(
            type    = sub_type,
            target  = neighbor,                              // NEIGHBOR CELL (CellClass*)
            firer   = firer,
            damage  = damage,
            warhead = warhead,
            speed   = 0x32,                                  // HARDCODED 50 (TargetSpeed)
            bright  = 0
        );

        // Initial velocity construction (§7)
        rand   = Random::RandomRanged(0, 0x20);              // 0..32
        angle  = (rand<<8 - 0x3FFF) * (-2π/65536);
        speed  = AirburstWeapon[0xA8] / 10;
        VelX = -cos(angle) * speed;
        VelY = -sin(angle) * speed;
        VelZ = 0.0;

        // Launch from parent bullet's current position
        launch_pos = parent.Location;                        // parent+0x9C/+0xA0/+0xA4
        new_bullet->Fire(&launch_pos, &velocity);            // vtable[0x1F0] = 0x00468670

        counter--;
    } while (counter != 0);

    // ── 9th sub-bullet: targeted at the impact cell itself ──
    damage  = AirburstWeapon[0xA4];
    warhead = AirburstWeapon[0xAC];
    firer   = parent[0x2C];

    CoCreateInstance(CLSID_BulletClass, ...);
    new_bullet->Init(
        type    = sub_type,
        target  = impact_cell,                               // IMPACT CELL
        firer   = firer,
        damage  = damage,
        warhead = warhead,
        speed   = 0x32,
        bright  = 0
    );
    // ... same random velocity setup
    new_bullet->Fire(&launch_pos, &velocity);
}
```

### Key observations

| Question | Answer |
|---|---|
| How many sub-bullets? | **Exactly 9.** Hardcoded `counter=8` loop + 1 explicit spawn. Not driven by INI. |
| What controls targets? | 8-loop targets 8 neighbor cells via `Pathfinding_update_continued(0..7)`. 9th targets impact cell via `ObjectClass::GetOccupiedCell`. |
| Launch position? | `parent.Location` (parent bullet's coords at detonation — same for all 9). |
| Initial velocity? | `AirburstWeapon.Speed/10` magnitude × random horizontal angle. VelZ = 0. |
| Damage scaling? | **None.** Each sub-bullet carries the FULL `AirburstWeapon.Damage`. No division, no falloff. |
| Homing? | Determined by sub-bullet's BulletType (`ROT > 0` → homing). |

---

## 6. Sub-bullet target assignment — 8 neighbor cells + 1 center

`Pathfinding_update_continued` at `0x00481810` is the engine's 8-direction neighbor
lookup:

```c
void Pathfinding_update_continued(int cell_ctx, uint dir_idx) {
    if (dir_idx < 8) {
        short cy = (short)(ctx[0x24] >> 16);
        short cx = (short)ctx[0x24];
        short nx = cx + g_DirectionOffsets[dir_idx * 2];
        short ny = cy + g_DirectionOffsets[dir_idx * 2 + 1];
        MapClass::Get_CellClass(&(ny<<16 | nx));
    }
}
```

`g_DirectionOffsets` (paired-short table) is the standard 8-direction neighbor offset
table. The airburst iterates `dir = 0..7` in order, covering the full 3×3-minus-center
neighborhood.

The 9th sub-bullet uses `this->GetOccupiedCell()` (vtable slot `0x1BC`, resolved to
`ObjectClass::GetOccupiedCell` at `0x005F6960`) — returns the `CellClass*` at the
bullet's current coords. So the target set is the **full 3×3 block**:

```
      [NW]  [N]  [NE]
      [W]   [C]  [E]    ← [C] = 9th sub-bullet target (impact cell)
      [SW]  [S]  [SE]
```

Each sub-bullet's `Target` is a `CellClass*` — **not** the original unit. With
`ROT > 0` on the sub-bullet's projectile, the sub-bullet homes onto the static cell
center. Any unit in that cell gets hit via normal proximity / cell-arrival detonation.

---

## 7. Velocity construction — what `sin(3π/2)` is actually doing

The engine builds the per-bullet initial velocity through a generic
"pitch × horizontal-facing → 3D velocity" helper, with the pitch hardcoded to `3π/2`
(≈270°):

```c
rand  = Random::RandomRanged(0, 0x20);
angle = ((rand << 8) - 0x3FFF) * (-2π/65536);   // horizontal angle in rad
S     = AirburstWeapon.Speed / 10;

// VelX = sin(3π/2) * cos(angle) * S = -cos(angle) * S
// VelY = sin(3π/2) * sin(angle) * S = -sin(angle) * S
// VelZ = cos(3π/2) * S             ≈ 0 * S = 0
```

The IEEE-754 literal `0x4012D989_1049EE22` decodes to the `double` value
`4.712436918747274` = `3π/2`. `sin(3π/2) = -1.0`, `cos(3π/2) ≈ 0`. Plain meaning:
**VelZ is zero; VelX/VelY form a horizontal vector of magnitude `Speed/10`, facing in
the random direction.**

### Random direction range — narrow 45° cone, NOT full 360°

`Random::RandomRanged(0, 0x20)` returns `[0, 32]`. After `<< 8`, range is `[0, 8192]`.
After `- 0x3FFF`, the facing-unit value becomes `[-16383, -8191]`. At `(2π/65536)` rad
per facing unit:

```
-16383 × (2π/65536)  ≈  -π/2   =  -90°
 -8191 × (2π/65536)  ≈  -π/4   =  -45°
```

The launch direction is drawn from a **45°-wide cone**, not a uniform 360°. This is
verifiable in the decomp.

**Why doesn't this matter visually?** Each sub-bullet's homing logic
(if `Projectile.ROT > 0`, e.g. `ROT=60` on `[ClusterBits]`) immediately overrides the
launch velocity in `FUN_005B20F0`, pitching and turning the bullet toward its
assigned cell target. The full 360° pattern players see comes from the 8 different
target cells, not from the launch direction. The 45°-cone artifact only manifests in
the first tick's trail effect.

### Confidence

- **Content: HIGH** — verified IEEE-754 constant decode + the multiplication pattern in the decomp.
- **Identity: HIGH** — the helper is reused engine-wide for velocity construction.
- **Binding: MEDIUM** — the 45°-cone is observed; its intent (engineering choice vs bug) is unverified.

---

## 8. Per-sub-bullet damage & flight

### Damage

Each sub-bullet's `Strength` is `BulletClass::Init` parameter `p5` =
`AirburstWeapon.Damage`. **No division, no falloff applied at spawn time.** With
`[V3Cluster].Damage=80`, each of 9 sub-missiles carries 80 HP damage before warhead
Verses modifiers — a theoretical max total of 720 HP if all 9 sub-bullets hit the
same target cluster.

The primary `[V3AirburstP]` ALSO applies its own `[V3Airburst].Damage=25` +
`[V3HE]` area-damage at impact, **before** the sub-bullets spawn (the
`WarheadTypeClass::Detonate` flow does area-damage first, then the airburst block
runs at the end).

### Homing / non-homing variants

The sub-bullets' homing is entirely from the sub-bullet's own BulletType. No
airburst-specific override:

| `AirburstWeapon.Projectile.ROT` | Sub-bullet behavior |
|---|---|
| `> 0` | Guided missile homing on its assigned cell. |
| `0` + `Arcing=yes` | Ballistic lob — VelZ=0 at launch, gravity takes over. |
| `0` + `Arcing=no` | Straight flight from spawn position along the launch velocity. |

Stock YR uses `[ClusterBits]` with `ROT=60`, `Proximity=yes`, `Ranged=yes`, no
Arcing — a guided short-range missile that homes onto its assigned neighbor cell.

---

## 9. Concrete walkthrough — V3 Rocket cluster strike

The single shipping use of airburst in YR.

### INI chain (from `ini/rulesmd.ini`)

```ini
[V3]
  Primary=V3Launcher
  Spawns=V3ROCKET
  SpawnsNumber=1

[V3Launcher]
  Spawner=yes
  Projectile=InvisibleHigh
  Warhead=Special

[V3ROCKET]
  Spawned=yes
  MissileSpawn=yes
  Ammo=1
  ; (fires V3Airburst when reaches drop point)

[V3Airburst]
  Damage=25
  Range=.55
  Projectile=V3AirburstP
  Warhead=V3HE

[V3AirburstP]              ; ← the AIRBURST primary bullet
  Proximity=yes
  Dropping=yes
  Cluster=9                ; DEAD — ignored because Airburst=yes
  Image=none
  Airburst=yes             ; ← gate at BulletType+0x294
  AirburstWeapon=V3Cluster ; ← BulletType+0x2B0
  Ranged=yes
  AA=no
  ROT=4

[V3Cluster]                ; defines the secondaries
  Damage=80                ; each sub-bullet's damage
  ROF=80                   ; IGNORED
  Projectile=ClusterBits
  Range=6                  ; IGNORED
  Speed=20                 ; magnitude = 20/10 = 2 lep/tick
  Warhead=V3HE

[ClusterBits]              ; sub-bullet type
  Arm=2
  Shadow=no
  Proximity=yes
  Ranged=yes
  Image=DRAGON
  ROT=60                   ; GUIDED
```

### Runtime trace

1. **Launch.** Player attacks. `V3Launcher.Spawner=yes` makes V3 spawn a `V3ROCKET` aircraft. Aircraft flies toward target.
2. **Mid-flight transition.** V3ROCKET reaches firing position; fires `V3Airburst` → creates `V3AirburstP` BulletClass aimed at target cell. Aircraft despawns.
3. **Primary bullet flight.** `[V3AirburstP]` descends with `ROT=4` (slow homing).
4. **Primary detonation.** Cell-arrival or proximity triggers `BulletDetonation`.
5. **Airburst branch taken.** Cluster loop skipped; `WarheadTypeClass::Detonate` called once.
6. **`Detonate` body executes:**
   - Screen shake, no rad site
   - `Apply_area_damage` with V3HE warhead → V3's 25 damage + V3HE CellSpread area-damage hits the impact
   - Explosion anim from V3HE.AnimList
   - Combat light, debris voxels
7. **Airburst spawn block at end of Detonate:**
   - 8 sub-bullets, one per neighbor cell (`Pathfinding_update_continued 0..7`)
   - Each: `Init(type=ClusterBits, target=neighbor, firer=V3ROCKET, damage=80, warhead=V3HE, speed=50, bright=0)` + random horizontal launch velocity (`(20/10)=2 lep/tick`)
   - 1 more sub-bullet, `target=impact_cell`
8. **Sub-bullet flight (9× parallel).** Each `ClusterBits` enters `BulletClass::AI`:
   - `ROT=60` → homing path
   - Target is a CellClass → homes on cell center
   - Initial 45°-cone horizontal velocity immediately overridden by homing turn logic
   - Arrival triggers proximity detonation (Proximity=yes, Arm=2 = 2-tick fuse delay)
9. **Sub-bullet detonation.** Each `ClusterBits` runs `BulletDetonation`. Cluster (default 0 on ClusterBits — see open follow-up) so the cluster-loop is skipped, but the Detonate-fork doesn't reach Detonate via the loop guard either. Subtle: ClusterBits's actual detonation pathway uses the pre-impact damage block in BulletDetonation against nearby units within 42/128 leptons. **Verify exact path** — open follow-up.
10. **Net effect.** Up to 9 V3HE warhead detonations around the V3's impact, each 80 base damage + CellSpread area-damage. Multiple sub-missiles can hit the same target cluster. Signature V3 "cluster bombardment."

---

## 10. Flak Cannon is NOT an airburst weapon

Common misconception: Flak Cannon uses `Airburst=yes`. **It does not.** Grepping
shipping `rulesmd.ini`:

- `Airburst=yes`: exactly ONE match → `[V3AirburstP]`
- `AirburstWeapon=`: exactly ONE match → `[V3AirburstP].AirburstWeapon=V3Cluster`

Flak Cannon's AA behavior is the separate `Inaccurate=yes + FlakScatter=yes`
mechanism documented in [`accuracy_inaccurate.md`](accuracy_inaccurate.md). No
sub-bullets, no starburst pattern.

```ini
[FlakProj]
  AA=yes
  Inaccurate=yes
  FlakScatter=yes
  Ranged=yes
  SubjectToElevation=yes
```

---

## 11. INI keys consumed by airburst

| Key | Section | Used as |
|---|---|---|
| `Airburst=` | `[BulletType]` (primary) | `BulletType+0x294` — gates entire spawn block |
| `AirburstWeapon=` | `[BulletType]` (primary) | `BulletType+0x2B0` — WeaponType* resolved at INI load |
| `Projectile=` | `[Weapon]` (AirburstWeapon) | `WeaponType+0xA0` — sub-bullet type |
| `Damage=` | `[Weapon]` (AirburstWeapon) | `WeaponType+0xA4` — sub-bullet damage |
| `Speed=` | `[Weapon]` (AirburstWeapon) | `WeaponType+0xA8` — launch velocity magnitude = Speed/10 |
| `Warhead=` | `[Weapon]` (AirburstWeapon) | `WeaponType+0xAC` — sub-bullet warhead |

**Explicitly ignored by the spawn block:**
- `AirburstWeapon.Burst=`, `.ROF=`, `.Range=`, `.MinimumRange=`, `.Report=`, `.Anim=` — none read.
- `BulletType.Cluster=` on the **primary** (dead when Airburst=yes).
- `BulletType.ShrapnelCount=` / `.ShrapnelWeapon=` on the primary (separate system, runs elsewhere in Detonate).

---

## 12. Hardcoded constants in the spawn

| Constant | Value | Location |
|---|---|---|
| Sub-bullet count | **9** (8 loop + 1 explicit) | inline literal in spawn block |
| Sub-bullet `TargetSpeed` (Init p7) | **50** (`0x32`) | hardcoded immediate |
| `bright` (Init p8) | **0** (false) | hardcoded immediate |
| Pitch in velocity construction | **3π/2** (`4.712...`) | IEEE-754 literal `0x4012D989_1049EE22` |
| Random facing range | **`[0, 32]`** (5 bits) | `Random::RandomRanged(0, 0x20)` |
| Facing-to-rad conversion | **`(2π/65536)`** | `_LAB_007E2810` |

---

## 13. TS-legacy filter

- **Spawn block itself**: LIVE in YR. Reached every V3 rocket impact.
- **`Dropping=yes`** on `[V3AirburstP]`: shares infra with TS paratrooper-bomb-drop code in `BulletClass::AI`. Reachable in YR but only through the V3 path. Not airburst-specific.
- **`Cluster=N` on the primary when `Airburst=yes`**: dead-field. The INI key is parsed; the value is stored; no consumer reads it in this branch. INI authors left it for flavor.
- The two extra `WarheadTypeClass::Detonate` callers found by xref (`FUN_0041BC30` is itself dead; `FUN_0070D690` is reached from FlyLocomotion + ReceiveDamage as an aircraft-crash damage applicator) are unlikely to ever construct a BulletClass with `Airburst=yes`, so the recursive-airburst risk is theoretical.

No TS-only dead branches in the airburst spawn itself.

---

## 14. Edge cases

| Case | Behavior |
|---|---|
| `AirburstWeapon=` is empty / not in rules | `BulletType+0x2B0 == NULL` at spawn time. The `iVar12 = BulletType[0x2B0]; sub_type = AirburstWeapon[0xA0]` would dereference NULL — crash. The INI parser likely refuses to set Airburst=yes without a valid AirburstWeapon. Open follow-up #1. |
| Primary bullet's BulletType has Airburst=yes but no `Image=`/visible projectile | Works fine — the primary still exists as a BulletClass and reaches Detonate. The visual is just invisible. |
| Sub-bullet's `Projectile` is itself `Airburst=yes` | Theoretical recursive airburst (9 → 81 → 729 → ...). No stock YR content does this. The engine doesn't bound the recursion. **Modder risk.** |
| Sub-bullet's warhead has `Tiberium=yes` | Each sub-bullet's detonation can chain-explode tiberium overlays normally. With 9 detonations in a 3×3 block, full tiberium clearance is likely. |
| All 9 sub-bullets hit the same target | Target takes 9× `AirburstWeapon.Damage` × Verses[armor]. Capped by MaxDamage per detonation (default 10000), but each hit is independent — so a 80-damage sub-bullet can deal 9 × 80 × Verses = up to 720 damage total. |
| Primary impact cell is on the map edge | The 8 neighbor-cell targets include cells outside the map. `Pathfinding_update_continued` returns whatever `MapClass::Get_CellClass` returns for off-map coords (likely the dummy out-of-bounds cell). Sub-bullets targeted there may instantly self-destruct or fly out-of-bounds. Edge-case behavior not traced. |
| Player ForceFires onto an empty cell | Same path. The primary bullet impacts the cell, 9 sub-bullets spawn at 3×3 around it. AffectsAllies still applies per sub-bullet impact. |

---

## 15. Open follow-ups

1. **AirburstWeapon validation at INI load.** Does the parser refuse to set `Airburst=yes` if `AirburstWeapon=` is unset? Or does it crash at runtime? Trace `BulletTypeClass::ReadINI`. Priority: MEDIUM.
2. **ClusterBits default Cluster field.** The walkthrough §9 step 9 raises a question: does `[ClusterBits]` end up with a non-zero default `BulletType+0x2AC` such that its warhead detonates? Or does its damage come from the pre-impact 42/128-lepton near-miss block? Decompile `BulletTypeClass::Constructor` defaults. Priority: MEDIUM.
3. **The 45°-cone launch direction.** Intentional design or engine quirk? The modding community generally treats it as a quirk, but it's been present since the function was written. Priority: LOW (visible only in trail rendering).
4. **`Bullet+0xB0` (Firer) vs `+0x10C` (Target) cross-confirmation.** Existing canonical docs differ on which field is which. The airburst spawn writes `parent[0x2C]` (= `parent+0xB0`) into `p4` and that goes to `new+0xB0` per Init. The existing `WARHEAD_DETONATE_GHIDRA_REPORT.md` §1 reads `+0xB0` as the firer. Priority: LOW (cross-doc consistency check).
5. **Dropping=yes role.** Does `Dropping=yes` actually drive the V3 mid-flight transition mechanic, or is it cosmetic? Trace consumers. Priority: LOW.

---

## 16. Sources

- Live decompilation of `BulletClass::BulletDetonation` at `0x00468D80` (2026-05-17) — confirmed Airburst gate at `BulletType+0x294`, Cluster=Airburst-dead behavior, scatter offset `FUN_0049f420`.
- Live xref of `"Airburst"` and `"AirburstWeapon"` strings, both into `BulletTypeClass::ReadINI`.
- Existing canonical doc: [`../../AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md`](../../AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md) — primary source for the spawn block walkthrough; this doc supersedes it for the airburst spec while preserving all verified findings.
- Existing canonical docs: [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md), [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md), [`../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`](../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md).
- INI quotes from `ini/rulesmd.ini` — `[V3]`, `[V3ROCKET]`, `[V3Launcher]`, `[V3Airburst]`, `[V3AirburstP]`, `[V3Cluster]`, `[ClusterBits]`, `[FlakProj]`.
- Cross-references: [`damage_formula.md`](damage_formula.md), [`accuracy_inaccurate.md`](accuracy_inaccurate.md), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md), [`splash_cellspread.md`](splash_cellspread.md), [`bullet_lifecycle.md`](bullet_lifecycle.md), [`projectile_rot_homing.md`](projectile_rot_homing.md).
