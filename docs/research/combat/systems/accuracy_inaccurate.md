# Accuracy & Inaccurate Scatter

This doc is the canonical reference for **projectile accuracy** in gamemd.exe:

- `Inaccurate=` flag (`BulletType+0x2A2`) — gates the target-snap re-read during detonation
- `FlakScatter=` flag (`BulletType+0x2A3`) — combined with `Inviso=` applies horizontal scatter at launch
- `BallisticScatter=` Rules-class constant (`Rules+0x1734`) — max scatter distance
- The full Flak-cannon-style scatter formula
- `Proximity=` flag (`BulletType+0x29F`) — DEAD-READ (parsed but unused)
- How accuracy composes with Arc / homing / Inviso / Burst

Out-of-scope:
- Bullet flight after launch → [`bullet_lifecycle.md`](bullet_lifecycle.md)
- The Arc and ROT homing math themselves → [`projectile_arc_gravity.md`](projectile_arc_gravity.md), [`projectile_rot_homing.md`](projectile_rot_homing.md)
- Cell-spread falloff at impact → [`damage_formula.md`](damage_formula.md) §5

---

## 1. The flags

| Field | Offset | INI key | Class | Default |
|---|---|---|---|---|
| `Inaccurate` | `+0x2A2` | `Inaccurate=` | BulletTypeClass | `false` |
| `FlakScatter` | `+0x2A3` | `FlakScatter=` | BulletTypeClass | `false` |
| `Proximity` | `+0x29F` | `Proximity=` | BulletTypeClass | `false` — **DEAD-READ** (see §6) |
| `BallisticScatter` | `+0x1734` | `BallisticScatter=` | RulesClass `[CombatDamage]` | per-INI; defaults vary by version |

### Parser locations (verified live 2026-05-17)

| String | Address | Xref into ReadINI |
|---|---|---|
| `"Inaccurate"` | `0x0081B0AC` | `BulletTypeClass::ReadINI 0x0046C0EF` |
| `"FlakScatter"` | `0x0081B0A0` | `BulletTypeClass::ReadINI 0x0046C105` |
| `"BallisticScatter"` | `0x0083ADA0` | `RulesClass::ReadCombatDamage 0x0066CD53` |

### Confidence

- **Content: HIGH** — all three flag offsets cross-verified against [`../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`](../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md) §6.1 and the live xrefs.
- **Identity: HIGH** — single INI key string per flag, single xref per parser.
- **Binding: HIGH** for `Inaccurate` and `FlakScatter` (both have verified consumers in `BulletClass::Fire` and `BulletClass::Detonate`). HIGH for `BallisticScatter` (single consumer in the scatter formula). NOT-USED for `Proximity` (parsed but no read site found — see §6).

---

## 2. `Inaccurate` — what it actually does

The Inaccurate flag has **one** runtime effect, in `BulletClass::Detonate`:

```c
// At detonation, before applying pre-impact damage:
if (this->Type->Inaccurate == 0) {     // i.e., bullet IS accurate
    // 1. Read target's CURRENT position (re-snap to where target is NOW)
    if (this->Target != NULL) {
        CoordStruct* tpos = this->Target->GetCoords();
        int dist = ftol(sqrt(dx² + dy² + dz²));
        if (dist < 32 && !this->Type->Airburst && !this->Type->Inaccurate) {
            this->Target->GetCoords();   // re-read (side-effect-only; result discarded)
        }
    }
    // 2. Apply pre-impact damage to turret buildings within 42 leptons,
    //    or airborne targets within 128 leptons (the "near-miss" damage).
    ...
}
```

If `Inaccurate=yes` is set, the entire block is skipped. The bullet detonates wherever
it physically arrived (i.e., where its velocity carried it from launch), **without**
re-reading the target's current position. Targets that moved during the projectile's
flight escape the snap, and the pre-impact near-miss damage isn't applied either.

### Key behavioral consequence

- **Accurate bullets (default)**: re-snap to target on impact. Even a slow projectile against a moving target will hit if the target didn't move beyond the 32-lepton snap radius. The bullet's trajectory looks like it curves slightly at the end. A near-miss within 42-128 leptons still does damage.
- **Inaccurate bullets**: hit wherever they geometrically arrive. If the target moved more than the bullet's CellSpread, the target may take no damage at all. No near-miss damage.

Used by:
- Flak weapons (the visible "near-miss" puffs of flak that don't damage moving aircraft).
- Lobbed weapons (V3, Tank Howitzer) — the projectile's arc carries it where it was aimed, not where target moved.
- Most `Arcing=yes` projectiles are also `Inaccurate=yes`.

### Confidence

- **Content: HIGH** — decomp around line 806 of `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` matches the verified Detonate body.
- **Identity: HIGH** — `BulletType+0x2A2` matches `BULLETTYPECLASS_GHIDRA_REPORT` and the live ReadINI xref.
- **Binding: HIGH** — the Detonate read is the sole consumer of the flag.

### Notable: Inaccurate is NOT angle-jitter

A common misunderstanding: `Inaccurate=yes` does **NOT** mean "the projectile is fired
with a random angle." Inaccurate bullets fly in a perfectly straight line from the
muzzle FLH at the moment of fire. They just don't re-correct on impact.

The actual random scatter at fire time is the **FlakScatter+Inviso** combination —
see §3.

---

## 3. `FlakScatter + Inviso` — the actual random-scatter formula

When BOTH `FlakScatter=yes` AND `Inviso=yes` are set on a BulletType, `BulletClass::Fire`
applies a random horizontal scatter to the target coordinate AT LAUNCH:

```c
// In BulletClass::Fire, after computing source/target coords:
float dx = target.X - source.X;
float dy = source.Y - target.Y;          // note: Y inverted (engine convention)
float dz = target.Z - source.Z;
double dist = sqrt(dx² + dy² + dz²);

int scatter_range  = RulesClass.BallisticScatter * 2;        // Rules+0x1734
int rand_scatter   = Random::RandomRanged(0, scatter_range);
int dist_int       = ftol(dist);
int owner_modifier = *(int *)(this->Owner + 0xB4);           // per-firer scale
int jitter_distance = (rand_scatter * dist_int) / owner_modifier;

int rand_facing = Random::RandomRanged(0, 0x7FFFFFFE);       // random 31-bit
short facing_norm = ftol(rand_facing) - 0x3FFF;
double angle_rad  = facing_norm * (2*PI / 65536);            // radians from 0..2π

new_target.X = cos(angle_rad) * jitter_distance + source.X;
new_target.Y = sin(angle_rad) * jitter_distance + source.Y;
new_target.Z = target.Z;                                     // Z unchanged
```

### Plain semantics

The displacement is **proportional to distance**:
- Close target: small jitter.
- Far target: large jitter (linear in dist).

The maximum jitter is `BallisticScatter × 2` leptons, scaled by `(dist / owner_modifier)`.
With `BallisticScatter = 256` (1 cell) and `owner_modifier = 256`:
- Target 1 cell away: jitter ∈ [0, 256 × (256/256)] = [0, 256] leptons.
- Target 5 cells away: jitter ∈ [0, 256 × (1280/256)] = [0, 1280] leptons = 5 cells.

The direction is random (uniform 0..2π).

### What "Inviso" means here

`Inviso=yes` bullets are **instant-hit** — they don't fly as visible projectiles.
The bullet snaps to the (scattered) impact point during Fire, and its velocity is
zeroed. Used for hitscan-style weapons (Tesla Coil zap, some Flak Cannon shots,
sonic-wave weapons in part).

Without `Inviso`, FlakScatter has a **different effect** (the AA-bounce trigger
documented in [`bullet_lifecycle.md`](bullet_lifecycle.md) — flak bursts when the
bullet drops below the target's altitude). The scatter formula in this section
specifically requires the `Inviso AND FlakScatter` combination.

### `Owner+0xB4` — what is it?

This is a per-firer integer scale. It's read off the **firing TechnoClass** instance,
not the bullet type. The field at `+0xB4` on TechnoClass is likely the unit's
`RangeAdjustment` or `AccuracyModifier` — needs verification. Open follow-up #1.

A value of 256 = "1 cell of correction per cell of distance" (i.e., scatter = `BallisticScatter × 2 × (dist/256)`). Higher values reduce scatter (better accuracy);
lower values amplify scatter (worse accuracy).

### Confidence

- **Content: HIGH** — formula reproduced from `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §5.2 with verified register flow.
- **Identity: HIGH** — single code site in `BulletClass::Fire`.
- **Binding: HIGH** — triggered for every Inviso+FlakScatter combination, which is the standard Flak Cannon AA pattern.

---

## 4. `BallisticScatter` — the global scatter cap

| Field | Value |
|---|---|
| Offset | `Rules+0x1734` |
| INI key | `BallisticScatter=` |
| INI section | `[CombatDamage]` |
| Parser | `RulesClass::ReadCombatDamage 0x0066CD53` (ReadRange) |
| Storage | int (packed high/low range — `ReadRange` parses min/max pair, stored together) |

### Quoted from rulesmd.ini

```
; from [CombatDamage] section
BallisticScatter=256       ; max scatter in leptons (1 cell)
```

(Verify exact value from current rulesmd.ini — flag for the quote.)

### Single consumer

`BulletClass::Fire` (in §3 above). No other read site found in the scatter system.

---

## 5. Composition with other bullet flags

| Flag combo | Effect |
|---|---|
| `Inaccurate=no, FlakScatter=no` (default) | Bullet flies straight, re-snaps on impact. The "ideal" tank shell. |
| `Inaccurate=yes, FlakScatter=no` | Bullet flies straight, NO re-snap on impact. Lobbed shells, V3, Tank Howitzer. |
| `Inaccurate=no, FlakScatter=yes, Inviso=no` | Re-snap on impact, FlakScatter trips the AA-bounce mechanism (below target altitude) but does NOT do launch-time scatter. |
| `Inaccurate=yes, FlakScatter=yes, Inviso=no` | Same as above but no re-snap either. The bounce-flak-pattern at altitude. |
| `Inaccurate=*, FlakScatter=yes, Inviso=yes` | Launch-time horizontal scatter formula (§3). The classic Flak Cannon "pattern around aircraft." |
| `Inaccurate=yes, Arcing=yes` | The standard lobbed-projectile combination. Aim is set at fire, bullet follows ballistic arc, lands where physics carry it. Used by V3, Tank Howitzer, Prism Tank lobs. |
| `Inaccurate=yes, ROT>0 (homing)` | The bullet still homes (re-aims each tick), but does NOT re-snap on detonation. If the target slips out of the snap radius mid-final-tick, the homing curve carries the bullet to where the target WAS at the last tick, not where it is now. |

---

## 6. `Proximity` — parsed but unused (DEAD-READ)

`Proximity=yes` is parsed by `BulletTypeClass::ReadINI` and stored at `BulletType+0x29F`.
**No code path reads this byte.** Binary-pattern search (`9F 02 00 00` little-endian
encoding of offset 0x29F as part of a memory operand) finds zero reads in
`BulletClass::AI`, `BulletClass::Fire`, `BulletClass::Detonate`, or any other
combat-path function. The flag is effectively dead.

Source: [`../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`](../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md) §7.

### Why mention it here?

Because some INIs and old documentation reference `Proximity=yes` as if it gates
"detonate when close enough." That behavior is actually implemented by the **Arm
field + ProximityDetector** mechanism (see [`bullet_lifecycle.md`](bullet_lifecycle.md)
§5.3), NOT by this flag. Don't confuse them.

The four byte-pattern hits for `9F 02` in the binary are reads from
`TechnoTypeClass+0x29F`, not BulletTypeClass — i.e., a different field on a different
class.

---

## 7. Edge cases

| Case | Behavior |
|---|---|
| `Inaccurate=yes` against a stationary target | No effect — the bullet hits the spot it was aimed at, and the target is there. Damage applies normally. |
| `Inaccurate=yes` against a target moving at 1 cell / 5 ticks | Bullet aimed at launch position; by impact time target has moved ~32+ leptons. With Inaccurate, the bullet lands at the launch position and may miss the target if its CellSpread is small. |
| `BallisticScatter = 0` | `scatter_range = 0`, `rand_scatter = 0`, jitter_distance = 0 → no scatter even with Inviso+FlakScatter. The flak pattern degenerates to a single point. |
| `BallisticScatter = -1` or negative | The Random::RandomRanged call would behave per its negative-handling rules; in practice INI parser likely clamps to 0+. Open follow-up #2. |
| Bullet's `Owner+0xB4 == 0` | Division by zero in the formula. In practice every TechnoClass instance has this field initialized to non-zero by its TypeClass-derived default; if a mod somehow leaves it zero, the engine would crash. Likely defensive: `Owner+0xB4` defaults to 256 or 100. Open follow-up #1. |
| Inviso+FlakScatter with target on a different elevation level | Z is unchanged (`new_target.Z = target.Z`), so the scatter is purely horizontal. The flak burst always appears at target's altitude. |
| Pre-impact damage when `Inaccurate=yes` | Skipped. The 32-lepton snap and 42/128-lepton near-miss damage do NOT apply. |

---

## 8. TS-legacy filter

- `Inaccurate=` is fully LIVE in YR (used by V3, every lobbed weapon, Flak, Prism splash).
- `FlakScatter=` is LIVE — Flak Cannon, Flak Track all use it.
- `BallisticScatter=` is LIVE — single consumer in active code path.
- `Proximity=` is parsed-but-dead-read in both TS and YR. The flag predates the BulletType reorganization that moved its functionality to the Arm+ProximityDetector system. **TS-legacy parser artifact, no active code path in YR.**

---

## 9. Open follow-ups

1. **`TechnoClass+0xB4` field identity.** Used as the divisor in the scatter formula (`owner_modifier`). Likely `AccuracyModifier` / `RangeAdjustment` / similar. Verify by tracing writes. Priority: HIGH for parity (a wrong divisor changes scatter pattern significantly).
2. **`BallisticScatter` actual default value in retail rulesmd.ini.** Quote from `[CombatDamage]` section. Priority: MEDIUM.
3. **`BallisticScatter` ReadRange semantics.** The `ReadRange` parser typically stores a min/max pair into one int (packed). Confirm whether the scatter formula uses only one half or both. Priority: MEDIUM.
4. **Confirm `Proximity=` is dead-read.** Re-run the byte-pattern search for `9F 02` and verify all hits are TechnoTypeClass reads. Priority: LOW.
5. **`Inaccurate` interaction with `Airburst=`.** The pre-impact snap is gated on `!Airburst` — i.e., Airburst weapons never re-snap regardless of Inaccurate. Verify whether Airburst inherits Inaccurate semantics or is independent. Priority: MEDIUM.
6. **Pre-impact "near-miss" damage radius.** §2 cites 42 leptons for ground targets and 128 leptons for airborne. Verify these constants. Priority: MEDIUM.
7. **The 32-lepton "snap-on-final" radius.** The decomp shows `dist < 32` for the side-effect-only second `GetCoords` call. Is 32 a Rules constant or hardcoded? Priority: LOW.

---

## 10. Sources

- Live xrefs of `"Inaccurate"` `0x0081B0AC`, `"FlakScatter"` `0x0081B0A0`, `"BallisticScatter"` `0x0083ADA0` to their respective ReadINI functions (verified 2026-05-17).
- Existing canonical doc: [`../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`](../../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md) — §5.2 (FlakScatter+Inviso formula), §6.1 (flag offsets), §7 (Proximity dead-read analysis), §9.1.1 (BulletClass::Detonate Inaccurate gate).
- BulletTypeClass layout: [`../../BULLETTYPECLASS_GHIDRA_REPORT.md`](../../BULLETTYPECLASS_GHIDRA_REPORT.md).
- Cross-references: [`damage_formula.md`](damage_formula.md), [`bullet_lifecycle.md`](bullet_lifecycle.md), [`projectile_arc_gravity.md`](projectile_arc_gravity.md), [`range_min_max.md`](range_min_max.md).
