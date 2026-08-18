# WarheadTypeClass — Re-Investigation Report

**Date:** 2026-04-24
**Target:** `gamemd.exe` (Yuri's Revenge 1.001), image base `0x00400000`
**Confidence:** HIGH (all findings verified from live Ghidra decompilation + raw disassembly)

## Relationship to existing docs

Two prior reports cover this class:

- `ra2-rust-game-docs/WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md` — struct layout, ReadINI (verified 2026-04-19, still accurate).
- `ra2-rust-game-docs/WARHEAD_DETONATE_GHIDRA_REPORT.md` — Detonate + Apply_area_damage flow.

This document **extends** the layout doc with new findings and **corrects** errors in the Detonate doc. Anything not restated here is unchanged.

---

## 1. Corrections to WARHEAD_DETONATE_GHIDRA_REPORT.md

Re-decompilation of `Apply_area_damage` at `0x00489280` produced three gate-flag corrections. The earlier report confused the offsets of `Wall` (0x144) and `Conventional` (0x14D), and `Tiberium` (0x148) and `Wood` (0x147).

### 1.1 Wall-overlay destruction gate

The existing doc (step 6a) said:
```
if (warhead->WallAbsoluteDestroyer || warhead->Conventional
    || (warhead->Wood && overlayType.material == 6))
```
**Wrong — it reads `warhead+0x144` (Wall), not 0x14D (Conventional).**

Verified gate (offsets from ReadINI at `0x0075D3A0`):
```
if (overlayType->IsWall /*+0x2A8*/ &&
    (warhead->WallAbsoluteDestroyer /*+0x145*/ ||
     warhead->Wall                   /*+0x144*/ ||
     (warhead->Wood                  /*+0x147*/ && overlayType.Material == 6)))
{
    CellClass::DestroyOverlay();
}
```

Practical effect:
- `Conventional=yes` has **no bearing** on wall-overlay destruction.
- A warhead without `Wall=yes`, `WallAbsoluteDestroyer=yes`, or `Wood=yes` (on wood-material walls) cannot damage walls.
- The existing Rust `BridgeDamageEvent` gate on `warhead.wall` is correct.

### 1.2 Bridge damage gate

Existing doc step 10 "Bridge Damage":
```
if (warhead->Conventional) { ... probabilistic destruction ... }
```
**Wrong — code reads `param_4+0x144` (Wall), not 0x14D (Conventional).**

Actual gate (decompiled at `0x00489f77` / `0x0048a0a5`):
```
if (warhead->Wall /*+0x144*/ &&
    (warhead == Rules->C4Warhead ||
     RandomRanged(1, Rules->BridgeStrength) < damageCount))
{
    ApplyDamageToCell();           // high bridge
    ApplyDamageToCell();           // low bridge
}
```

So `Wall=yes` is what permits both wall-overlay *and* bridge destruction. `Conventional` never appears in Apply_area_damage.

### 1.3 Tiberium/vein destruction gate

Existing doc:
```
if (!typeEntry->IsVein || warhead->Wood) {   // "Wood" flag = can destroy veins
    if (destroyTiberium) CellClass::Reduce_Tiberium();
}
```
**Wrong — code reads `param_4+0x148` (Tiberium), not 0x147 (Wood).**

Actual gate (`0x00489280` around `iStack_60+0x2A9`):
```
if (overlayType->IsTiberium /*+0x2B1*/ &&
    (!overlayType->IsVein   /*+0x2A9*/ || warhead->Tiberium /*+0x148*/) &&
    destroyTiberium)
{
    CellClass::Reduce_Tiberium();
}
```

Practical effect:
- **Regular ore** (IsVein=0): destroyed by any warhead whose caller passed `destroyTiberium=1`. `Tiberium=yes` is **not required**.
- **Veins** (IsVein=1): require `warhead.Tiberium=yes` to be destroyed.
- In YR maps there are no veins, so `Tiberium=yes` is effectively a no-op.

The existing Rust comment at [src/sim/combat/mod.rs:565-566](src/sim/combat/mod.rs#L565-L566) already states this correctly.

### 1.4 Conventional flag — where is it actually used?

`Conventional` (0x14D) is NOT read by `Apply_area_damage` or `FUN_00489180`. It is only used in:
- `WarheadTypeClass::ReadINI` at `0x0075D4EE` (parse + store)
- Crater/impact-anim selection code (gates which crater sprite plays).

It is a cosmetic / crater-selection flag, not a damage gate.

---

## 2. Damage Falloff Formula — `FUN_00489180` at `0x00489180`

The layout doc punts to this helper ("the falloff calculation happens inside ReceiveDamage"). Here is the verified, complete formula decoded from the raw disassembly.

### 2.1 Signature (`__fastcall`, `RET 0x8`)

| Slot | Purpose |
|------|---------|
| ECX  | `damage` (signed int) |
| EDX  | `warhead` (`WarheadTypeClass*`) |
| stack [esp+4] | `armor_index` (0..10, matches Verses array index) |
| stack [esp+8] | `distance` (int, leptons) |

Called from: `ObjectClass::ReceiveDamage` (`0x005F5390`), `TechnoClass::ReceiveDamage` (`0x00701900`), `FUN_006fdb80`. Vtable-dispatched via `vtable+0x16C`.

### 2.2 Pseudocode (from the assembly at `0x00489180`)

```
if (damage == 0) return 0;
if ((Scenario.Flags & 0x20) != 0) return 0;   // global "no damage" flag
if (warhead == NULL) return 0;

// -------- healing (negative damage) --------
if (damage < 0) {
    // Special armor types (>=8: Special_1, Special_2, or beyond) cannot be healed.
    return (armor_index >= 8) ? 0 : damage;
}

// -------- linear distance falloff --------
// Constant at 0x007e2224 = 256.0  (cells -> leptons)
cellspread_leptons = ftol(warhead->CellSpread * 256.0);

if (damage * warhead->PercentAtMax != damage  &&  cellspread_leptons > 0) {
    // damage falls off linearly from 100% at dist=0 to PercentAtMax at dist=cellspread
    falloff_damage = damage * PercentAtMax
                   + damage * (1 - PercentAtMax) * (cellspread_leptons - distance)
                                               / cellspread_leptons;
    // Equivalently:
    //     = damage * (1 - (1 - PercentAtMax) * distance / cellspread_leptons)
    falloff_damage = ftol(falloff_damage);
} else {
    falloff_damage = damage;   // PAM == 1.0 (no falloff) or CellSpread == 0 (single target)
}

// Floor at zero (SETLE CL ; DEC ECX ; AND ECX,ESI)
if (falloff_damage <= 0) falloff_damage = 0;

// -------- armor multiplier --------
// Verses[armor] is double, at warhead+0xA0 + armor_index*8
scaled = ftol(falloff_damage * warhead->Verses[armor_index]);

// -------- global cap --------
// Rules->MaxDamage at g_RulesClass_Instance + 0x16C8
if (scaled >= Rules->MaxDamage) return Rules->MaxDamage;
return scaled;
```

### 2.3 Properties

- **Falloff direction:** 100% at impact center → `PercentAtMax * 100%` at the edge of CellSpread. Linear.
- **Distance is NOT clamped inside this function.** The caller (Apply_area_damage) already filtered to `distance <= cellspread_leptons`, so extrapolation below PAM does not occur in normal play. If an external caller supplied `distance > cellspread_leptons`, the formula would produce a value below `damage * PAM`, potentially floored to 0 by the `SETLE` clamp.
- **PercentAtMax default is 1.0.** When PAM = 1.0, `damage * PAM == damage` is true and the FPU compare (`FCOMP` + `FNSTSW`/`TEST AH,0x40`) takes the no-falloff branch. Warheads without CellSpread (single-target) or with PAM=1.0 deal flat damage across the radius.
- **CellSpread = 0 path:** `cellspread_leptons = 0` takes the no-falloff branch, and the target receives flat `damage * Verses[armor]`.
- **Healing (damage<0) bypasses falloff and Verses**, and is blocked for armor indexes ≥ 8 (Special_1, Special_2). Regular healing passes through with no modification.
- **Verses is applied AFTER falloff**, then clamped to `Rules->MaxDamage` (0x16C8).
- **Each ftol call rounds toward zero** (IEEE 754 default on x87 in YR; `Math__ftol` uses the FPU rounding mode set at startup). There are three ftol calls total, so small per-target quantization is baked in.

### 2.4 Implications for current Rust code

`src/sim/combat/combat_aoe.rs` implements:
```
t = distance / cell_spread                        // clamped [0,1]
falloff_pct = 100 + (percent_at_max - 100) * t
damage = base_damage * verses_pct * falloff_pct / 10000
```

This matches the binary's intent, but three minor differences exist:
1. **Order of operations.** Binary: `ftol(ftol(damage*falloff) * verses)`. Rust: `damage * verses_pct * falloff_pct / 10000`. Rounding-order differs — e.g. a 99-damage warhead at 0.5x verses + 0.5 falloff could yield 24 vs 25 depending on order. For parity, match the binary: apply falloff → ftol → multiply by Verses → ftol.
2. **Global cap.** Rust has no `Rules->MaxDamage` clamp. Verify whether any gameplay path exceeds the cap; if so, we need the cap for parity.
3. **Zero-crossing floor.** Binary floors negative interpolated damage at 0 before applying Verses. Rust's formula can't go negative since it uses integer percentages, but confirm there's no path that produces a negative intermediate.

---

## 3. TechnoClass::ReceiveDamage — Immunity Gates and AffectsAllies

Verified at `0x00701900`. These gates run **before** the damage falloff is applied and short-circuit to zero damage on hit.

| Gate | Warhead flag | Target field | Effect |
|------|--------------|--------------|--------|
| Radiation immunity | `Radiation` (+0x177) | TypeClass+0xD37 | `*damage = 0; return 0;` |
| Psionics immunity | `PsychicDamage` (+0x178) | TypeClass+0xD36 | `*damage = 0; return 0;` |
| Poison immunity | `Poison` (+0x156) | TypeClass+0xD3B | `*damage = 0; return 0;` |
| **AffectsAllies** | `AffectsAllies==0` (+0x179) | Source owner allied with target owner | `*damage = 0; return 0;` |
| Psychedelic | `Psychedelic` (+0x16D) | — | Special path (see below) |

### 3.1 AffectsAllies semantics

```
if (warhead->AffectsAllies == 0 &&
    source != NULL &&
    HouseClass::IsAlliedWith(source->Owner, this->Owner))
{
    *damage = 0; return 0;
}
```

- Default value is **true** (constructor sets +0x179 = 1), so a warhead with no explicit key hits allies normally.
- Only 2 YR warheads set `AffectsAllies=no` in rulesmd.ini (per the ini scan). Most friendly-fire behavior is via weapon-level logic, not this flag.

### 3.2 Psychedelic (+0x16D) branch

If `warhead.Psychedelic`, a special branch:
1. Return 0 if source is allied with target's owner (ally check via `HouseClass::IsAlliedWith`).
2. Return 0 if target TypeClass has immunity (`+0xD35`).
3. Return 0 if target is a Building (WhatAmI == 6).
4. Otherwise call `FUN_00489180` with 0 armor — computes falloff-only damage, stores to +0x29C.
5. Set `+0x298 = 1` (Psychedelic state flag).
6. Call `vtable[0x3C8]` (`ReceiveParasite`-like handler, here used as "swap team").
7. Call `vtable[0x1E8]` (notify state change).
8. Return 1.

This is the Yuri Prime "psi blast" mind-swap — temporarily controls the target.

---

## 4. DelayKill System — `CausesDelayKill`, `DelayKillFrames`, `DelayKillAtMax`

Active in YR. Verified at `0x00701900` (TechnoClass::ReceiveDamage, after ObjectClass::ReceiveDamage returns 4 = "died").

### 4.1 Trigger conditions (all must hold)

1. `ObjectClass::ReceiveDamage` returned 4 (target died this call).
2. `target.WhatAmI() == 6` (Building).
3. `warhead.CausesDelayKill` (+0x130) is set.
4. `target.TypeClass` has `EligibleForDelayKill` set at TypeClass+0x1551.

### 4.2 Behavior when triggered

```
delayFrames = ftol(warhead->DelayKillFrames
                 + dist_ratio * (warhead->DelayKillAtMax - 1.0) * DelayKillFrames);
// (exact FPU sequence interpolates the frame count; same pattern as PercentAtMax)

if (target->IsBeingDelayKilled /*+0x6DF*/) {
    // Already being delay-killed — take the shorter remaining time
    remaining = DelayKillFrames_remaining
              - (CurrentFrame - DelayKillStartFrame);
    if (remaining <= delayFrames) goto restoreHealth;
}
// Not yet delay-killed: mark it
target->IsBeingDelayKilled  = 1;                   // +0x6DF
target->DelayKillStartFrame = CurrentFrame;        // +0x528
target->DelayKillAuxField   = iStack_78;           // +0x52C (purpose unclear — possibly facing/seed)
target->DelayKillFrames     = delayFrames;         // +0x530

restoreHealth:
target->IsAlive = true;
target->Health  = 1;          // keep alive at 1 HP until timer expires
return 5;                     // tell caller "handled specially"
```

### 4.3 Coverage in YR content

Only `[OilExplosionWH]` in rulesmd.ini sets these keys (`CausesDelayKill=yes`, `DelayKillFrames=5`, `DelayKillAtMax=7.0`). Used by oil-derrick destruction — the building stays up for a few frames before collapsing with a fire burst. The `DelayKillFrames=5` base with `DelayKillAtMax=7.0` means a dead-center hit delays ~5 frames, edge-of-spread hits delay up to ~35 frames.

**Not TS legacy — active in YR**, contrary to the layout doc's "Likely TS legacy or unused feature" note.

---

## 5. Sonic (+0x14B) — Force-Detach Parasite

Verified at `FootClass::ReceiveDamage` (`0x004D7330`):

```
if (warhead != NULL &&
    warhead->Sonic /*+0x14B*/ &&
    target->AttachedParasite /*this+0x174 in Foot*/ != NULL)
{
    WarpAttachClass::Detach();
    if (source != NULL) (*source->vtable[0x3C8])(0);   // notify attacker
}
```

**Effect:** A Sonic warhead hitting a host that has a parasite attached (Terror Drone, Squid) forcibly detaches the parasite. This is the Dolphin-vs-Giant-Squid interaction. The warhead does not need to deal damage — the detach happens before the damage pipeline proceeds.

Not documented in the Detonate report; spot-confirmed in the layout doc's verification note.

---

## 6. Summary of Lookup Points for Each Warhead Flag

The layout doc confirms parsing. This table shows **where each flag is actually consumed**, to help future implementation work.

| Flag | Offset | Consumer function | Behavior |
|------|--------|-------------------|----------|
| `Verses` | 0xA0 | FUN_00489180 | Armor multiplier applied after falloff |
| `ProneDamage` | 0xF8 | (confirmed in Rust; caller-applied multiplier) | Damage × prone multiplier for prone infantry |
| `CellSpread` | 0x124 | Apply_area_damage, FUN_00489180 | Radius for target collection + falloff denominator |
| `PercentAtMax` | 0x12C | FUN_00489180 | Edge-of-spread damage fraction |
| `CausesDelayKill` | 0x130 | TechnoClass::ReceiveDamage | Defers building death (§4) |
| `DelayKillFrames` | 0x134 | TechnoClass::ReceiveDamage | Delay duration base |
| `DelayKillAtMax` | 0x138 | TechnoClass::ReceiveDamage | Delay duration at edge of spread |
| `CombatLightSize` | 0x13C | FUN_0048A620 | Size of combat-light smudge |
| `Particle` | 0x140 | Apply_area_damage (end of function) | Spawn ParticleSystem at impact |
| `Wall` | 0x144 | Apply_area_damage | **Gates wall-overlay + bridge destruction** (§1.1-1.2) |
| `WallAbsoluteDestroyer` | 0x145 | Apply_area_damage | Wall-overlay gate (no HP check) |
| `PenetratesBunker` | 0x146 | (not found in reach of this investigation) | Likely in occupant-damage forwarding |
| `Wood` | 0x147 | Apply_area_damage | Wall-overlay gate when material==6 (wood walls) |
| `Tiberium` | 0x148 | Apply_area_damage | **Gates VEIN destruction only** (§1.3) — no effect in YR |
| `OrganicImmune` | 0x149 | (auto-computed; Verses[4]==0 && Verses[6]==0) | Not a flag-style consumer |
| `Sparky` | 0x14A | (unresolved in this pass) | Spark visuals — likely art/anim code |
| `Sonic` | 0x14B | FootClass::ReceiveDamage | Force-detach parasite (§5) |
| `Fire` | 0x14C | (death anim selection) | Fire death anim variant |
| `Conventional` | 0x14D | Crater-anim selection | Cosmetic only (§1.4) |
| `Rocker` / `DirectRocker` | 0x14E / 0x14F | Detonate | Push effect on targets |
| `Bright` | 0x150 | Detonate | Spawn combat-light on impact |
| `CLDisable{Red,Green,Blue}` | 0x151-0x153 | Detonate | Mask color channels in combat-light |
| `EMEffect` | 0x154 | (not verified in this pass — likely EMPulseLocomotion) | EMP disable |
| `MindControl` | 0x155 | Detonate | Capture target |
| `Poison` | 0x156 | TechnoClass::ReceiveDamage + Detonate | Immunity check + apply |
| `IvanBomb` | 0x157 | Detonate | BombClass::Attach |
| `ElectricAssault` | 0x158 | Detonate | `FUN_0062A980` (Tesla-like discharge helper) |
| `Parasite` | 0x159 | Detonate | `FUN_0041D830` parasite-attach |
| `Temporal` | 0x15A | Detonate | TemporalClass::InitiateWarp |
| `IsLocomotor` | 0x15B | Detonate + Apply_area_damage knockback | Magnetron hijack + ChronoShift buildings + force deploy |
| `Locomotor` | 0x15C (16B CLSID) | Detonate | CLSID applied via target->vtable[0x3D8] |
| `Airstrike` | 0x16C | Detonate | Trigger airstrike (`FUN_00452820`) |
| `Psychedelic` | 0x16D | TechnoClass::ReceiveDamage | Mind-swap branch (§3.2) |
| `BombDisarm` | 0x16E | Detonate | BombClass::Defuse |
| `Paralyzes` | 0x170 | (unresolved; likely in Parasite-attach path) | Only ParasitePlus sets this |
| `Culling` | 0x174 | (unresolved) | Only ParasitePlus sets this; "kills if Red HP" per INI |
| `MakesDisguise` | 0x175 | Detonate (vtable 0x46C) | Force disguise on target |
| `NukeMaker` | 0x176 | Detonate | Spawn downward nuke |
| `Radiation` | 0x177 | TechnoClass::ReceiveDamage + Detonate | Immunity check + RadSite creation |
| `PsychicDamage` | 0x178 | TechnoClass::ReceiveDamage | Immunity check (+0xD36 on TypeClass) |
| `AffectsAllies` | 0x179 | TechnoClass::ReceiveDamage | Ally-filter (§3.1); default true |
| `Bullets` | 0x17A | (unresolved; cosmetic bullet/impact anim) | SA-style hit anims |
| `Veinhole` | 0x17B | (unresolved; TS-only terrain) | Dormant in YR |
| `Shake{X,Y}{lo,hi}` | 0x17C-0x188 | Detonate (step 1) | Randomized screen-shake amount |
| `Particle` (again) | 0x140 | Apply_area_damage tail | Spawn ParticleSystem |
| `AnimList` | 0x104 | Detonate (crater anim) | Damage-indexed anim selection |
| `Debris*`, `MinDebris`, `MaxDebris` | 0x18C-0x1C8 | Detonate (step 5) | Spawn voxel/anim debris |

Unresolved flags (Paralyzes, Culling, Bullets, Sparky, CellInset) are only used by 1-2 warheads each in rulesmd.ini and do not affect mainline combat. Their exact dispatch should be traced when those specific units are implemented (Squid/ParasitePlus, SA-type warheads, Desolator deploy).

---

## 7. Rust Implementation Status (parity audit)

Per the parallel scan of `src/`:

### Implemented correctly
- `Verses` (11-armor table), `CellSpread`, `PercentAtMax` falloff, `ProneDamage`, `Wall`-gated bridge damage, `AnimList` damage-indexed selection, `InfDeath` animation variant.

### Implemented but with divergences to resolve
- **Falloff order of operations.** Rust does `damage × verses% × falloff% / 10000`. Binary does `ftol(damage × falloff) × verses`, then `ftol`, then clamp to `Rules.MaxDamage`. Fix order to match binary for exact parity.
- **Global damage cap.** Rust has no `Rules.MaxDamage` clamp. Add `Rules.MaxDamage` parsing and final-clamp if any path exceeds it in practice.
- **Ore destruction gate.** Rust destroys ore unconditionally — this is correct (§1.3), since regular ore is gated only by the caller's `destroyTiberium` arg, not by `warhead.Tiberium`. Comment in code is accurate. `warhead.Tiberium=yes` gates veins (dormant in YR), so no ore-destruction code change needed.

### Parsed but not wired
- Immunity gates: `Radiation`, `PsychicDamage`, `Poison` vs target TypeClass immunity flags (§3). These are short-circuit gates in `TechnoClass::ReceiveDamage`.
- `AffectsAllies=no` ally filter (§3.1). Only 2 YR warheads use this.
- `Psychedelic` mind-swap branch (§3.2).
- `Sonic` force-detach-parasite hook (§5).
- `CausesDelayKill` / `DelayKillFrames` / `DelayKillAtMax` deferred building death (§4).
- Rocker, Bright, CL-disable flags (screen-shake already not wired), DebrisTypes spawning, crater smudge.

### Not parsed (missing from Rust struct)
- `BombDisarm`, `NukeMaker`, `Paralyzes`, `MinDebris`, `MaxDebris`, `ShakeXlo/Xhi/Ylo/Yhi`, `CombatLightSize`, `PenetratesBunker`, `Psychedelic`, `PsychicDamage`, `AffectsAllies`, `CausesDelayKill`, `DelayKillFrames`, `DelayKillAtMax`, `CellInset`, `Veinhole`.

Most of these can wait until the corresponding unit/feature is implemented.

---

## 8. Open Questions

| # | Question | Why it matters | Suggested probe |
|---|----------|----------------|-----------------|
| 1 | Exact FPU round-to-int semantics in `Math__ftol` | Determines parity at half-damage edge cases | Disassemble `0x007C5F00` |
| 2 | Where is `Paralyzes` (+0x170) read? | ParasitePlus freeze effect | Trace callers of `FUN_0041D830` (parasite attach) + grep offset 0x170 |
| 3 | Where is `Culling` (+0x174) read? | ParasitePlus "kill if Red HP" | Likely in InfantryClass damage path or `FUN_0041D830`; not yet found |
| 4 | Where is `Bullets` (+0x17A) read? | SA-style impact visuals | Likely in crater/bullet-anim selection; check `FUN_0048A4F0` callers |
| 5 | Where is `Sparky` (+0x14A) read? | Spark effects on impact | Probably in Detonate's spark-sprite spawn; scan Detonate disassembly for 0x14A |
| 6 | Is `Rules.MaxDamage` (at `0x16C8`) ever exceeded? | Whether we need the clamp in Rust | Calibrate with high-damage weapons (Prism Tower charge, Apoc cannon) |
| 7 | Exact FPU sequence for DelayKill frame-count interpolation | Parity of oil-derrick collapse timing | Disassemble `0x00701900` around the DelayKill trigger |
| 8 | `Psychedelic` interaction with mind-controlled targets | Edge case for Yuri Prime | Check the `+0x298` / vtable 0x3C8 handler |

---

## Sources

### Decompiled / disassembled in this pass
- `WarheadTypeClass::ReadINI_Body` @ `0x0075D3A0`
- `WarheadTypeClass::Detonate` @ `0x004690B0`
- `Apply_area_damage` @ `0x00489280`
- `FUN_00489180` @ `0x00489180` (damage falloff helper — full x86 disassembly)
- `ObjectClass::ReceiveDamage` @ `0x005F5390`
- `TechnoClass::ReceiveDamage` @ `0x00701900`
- `FootClass::ReceiveDamage` @ `0x004D7330`
- `InfantryClass::ReceiveDamage` (filter) @ `0x005227F0`
- Constant `0x007E2224` = `0x43800000` (float 256.0) — cell→lepton scale

### Prior research referenced
- `ra2-rust-game-docs/WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`
- `ra2-rust-game-docs/WARHEAD_DETONATE_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/RECEIVE_DAMAGE_GHIDRA_REPORT.md`
- `ra2-rust-game-docs/DAMAGE_MATH_GHIDRA_REPORT.md`

### INI / Rust scanned
- `ini/rulesmd.ini` [Warheads] + [General] + all 105 warhead sections
- `src/rules/warhead_type.rs`
- `src/sim/combat/mod.rs`, `src/sim/combat/combat_aoe.rs`
