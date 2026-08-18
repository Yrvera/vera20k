# Grizzly GetFLH Turret Receiver 0x388 vs BarrelFacing 0x3A0 -- Ghidra Research Report

**Target:** `GRIZZLY_GETFLH_TURRET_RECEIVER_388_VS_BARRELFACING_3A0`
**Scope:** Resolve the receiver passed to `RateTimer::Current` inside `TechnoClass::GetFLH @ 0x006F3AD0` and its relationship to Grizzly/MTNK `BarrelFacing +0x3A0`.
**Status:** COMPLETE for the static fire-source facing expression and Rust-facing implication; runtime pixel capture remains optional validation.
**Active in YR:** Yes. This is the standard `TechnoClass::Fire_At -> vtable+0xB0/GetFLH` path used by stock MTNK weapons.

## Working Notes Gate

- Target question: Does stock Grizzly FLH source orientation follow `this+0x388`, `BarrelFacing+0x3A0`, body facing, or a derived relative angle at fire time?
- Non-goals: projectile impact physics, `Greatest_Threat`, factory delivery, non-turret buildings, non-MTNK multi-turret edge cases, and screenshot capture.
- Evidence needed to mark COMPLETE: decompile plus assembly proving `GetFLH` receiver/field; caller proof from `Fire_At`; fire-aim proof for `+0x3A0`; Rust field/event scan.
- Stop conditions: read-only Ghidra unavailable, function boundary missing, or inability to distinguish `+0x388` from `+0x3A0`.

## Summary

`TechnoClass::GetFLH @ 0x006F3AD0` does not directly read Grizzly `BarrelFacing +0x3A0` when computing the normal locomotor-backed FLH source. In the live locomotor branch it:

1. gets the object's facing through virtual `+0x2A8`,
2. calls `RateTimer::Current` with receiver `this + 0x388`,
3. quantizes both values to the same 32-way bucket family,
4. computes a relative angle as `quantized(vtable+0x2A8 facing) - quantized(Current(this+0x388))`,
5. applies that relative rotation on top of the locomotor matrix, then translates by type turret offset and FLH.

The Grizzly fire-aim branch in `UnitClass::Fire_At_Target` separately sets `this + 0x3A0` when `TechnoType+0xCA1 Turret=yes`. That proves `+0x3A0` is the fire gate/aiming timer, but it is not the timer sampled by `GetFLH`. The source orientation is therefore not "body facing only" and not "direct BarrelFacing +0x3A0"; it is a derived locomotor-relative angle that uses `this+0x388` as the timer term.

## Verified Binary Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| `GetFLH` locomotor branch samples `this+0x388` as the `RateTimer::Current` receiver. | `TechnoClass::GetFLH @ 0x006F3AD0`; assembly `0x006F3BC6..0x006F3BD3`: `LEA EDX,[ESP+0x18]`, `LEA ECX,[EBX+0x388]`, `PUSH EDX`, `CALL 0x004C93D0`. | Yes |
| `GetFLH` derives its rotation from two quantized facings: virtual `+0x2A8` result minus `Current(this+0x388)`. | `0x006F3BB9..0x006F3BC0` calls `vtable+0x2A8`; `0x006F3BD8..0x006F3C12` shifts both values by `0xA`, rounds, masks `0x1F`, subtracts `8`, multiplies by `PI/16`, and `FSUB`s them before `Matrix3x4_RotateZ @ 0x006F3C6D..0x006F3C79`. | Yes |
| Non-locomotor fallback uses only the virtual `+0x2A8` facing, proving the `+0x388` subtraction is specific to the locomotor/matrix path. | `GetFLH @ 0x006F3B56..0x006F3C1A` branches to fallback when object/flag/locomotor checks fail; fallback `0x006F3C1A..0x006F3C4E` calls `vtable+0x2A8` and does not call `RateTimer::Current(this+0x388)`. | Conditional; stock MTNK uses the locomotor path. |
| Stock Grizzly `Turret=yes` fire-aim path sets `this+0x3A0`, not `this+0x388`. | `UnitClass::Fire_At_Target @ 0x00736F78..0x00736FAC`: checks `Type+0xE11` false and `Type+0xCA1` true, computes target facing, then `LEA ECX,[ESI+0x3A0]`, `CALL 0x004C9220`. | Yes |
| `TechnoClass::Fire_At` calls virtual `GetFLH` and stores the returned source coordinate before projectile/effect work. | `TechnoClass::Fire_At @ 0x006FDD50`; assembly `0x006FE263..0x006FE282`: pushes output coord, `CALL dword ptr [EDI+0xB0]`, copies returned `x/y/z` to stack source. | Yes |

## Relationship Between `+0x388` And `+0x3A0`

Verified:

- `+0x3A0` is the stock Grizzly fire-aim timer set by the turreted `Fire_At_Target` branch.
- `+0x388` is the timer sampled by `GetFLH` for the locomotor-backed source-coordinate rotation.
- `Facing_Update @ 0x00736A00..0x00736C03` has paths that read or set both timers, but the stock Grizzly fire-aim branch at `0x00736F78..0x00736FAC` itself writes only `+0x3A0`.

Inference from the static evidence:

- The FLH source orientation should be modeled as a distinct source-facing input equivalent to binary `Current(this+0x388)`, combined with body/locomotor facing as `GetFLH` does.
- `BarrelFacing +0x3A0` may match `+0x388` in some steady states because `Facing_Update` can synchronize/retarget timers, but this investigation did not find a direct `GetFLH` read of `+0x3A0`.

## Current Rust Delta

Current Rust has only body-facing data in fire events:

- `src/sim/world/mod.rs:210..224`: `SimFireEvent` carries `facing: u8` and `FireOriginSnapshot::facing`, but no turret/source-facing timer equivalent to binary `this+0x388`.
- `src/sim/combat/mod.rs:1979..1995`: fire events snapshot `snap.facing` for both event `facing` and origin snapshot facing.
- `src/app_fire_effects.rs:232..247`: non-garrison FLH world offset uses `ev.facing`.
- `src/app_instances/units.rs` renders turret/barrel from `entity.barrel_facing`, but that is Rust's current single turret timer, closer to binary `+0x3A0` aim state than the now-proven `GetFLH` `+0x388` receiver.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `GetFLH` source rotation uses `Current(this+0x388)` in a locomotor-relative angle, not direct `+0x3A0`. | `SimFireEvent`/origin snapshot lacks a source-turret-facing field and app FLH uses body facing. | `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/app_fire_effects.rs` | MTNK hull facing north with a distinct FLH source-facing snapshot produces an east/west-shifted source according to the source-facing value, not body `facing`. | `grizzly_flh_origin_uses_source_turret_facing_snapshot` | Using body facing gives visibly wrong muzzle/projectile origin for side shots. |
| Grizzly fire-aim gate writes `+0x3A0` only; `GetFLH` samples `+0x388`. | Rust currently has one `barrel_facing` and may be tempted to reuse it for FLH. | `src/sim/movement/turret.rs`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs` | Construct a unit with body north, barrel aim east, and source timer north; fire origin follows source timer until an explicit synchronization rule updates it. | `grizzly_flh_origin_does_not_read_barrel_facing_directly` | Reusing `barrel_facing` as `+0x388` without modeling synchronization can lock in wrong pixels. |
| `Fire_At` consumes `GetFLH` world coordinate before projectile/muzzle/report work. | Some Rust paths still resolve presentation FLH above sim and use body-facing event data. | `src/app_fire_effects.rs`, future projectile source code | Projectile start, muzzle flash, and report sound share the same computed world source for MTNK. | `grizzly_projectile_muzzle_report_share_getflh_source` | Computing separate screen/world origins causes subtle visual/audio drift. |

## Negative Facts / Do Not Do

- Do not say `GetFLH` directly uses `BarrelFacing +0x3A0`; the proven receiver is `this+0x388` at `0x006F3BCA..0x006F3BD3`.
- Do not model Grizzly FLH as body-facing-only; the locomotor path subtracts a timer-derived facing from `this+0x388`.
- Do not use `PrimaryFireFLH=150,0,100` as proof of fire eligibility; it is source-coordinate data consumed after fire is allowed.
- Do not assume the 8-direction muzzle anim-facing path decides stock Grizzly source orientation; `Fire_At` calls `GetFLH` before anim/sound/projectile consumers, and stock `GUNFIRE`/`VTMUZZLE` are single anim names.
- Do not collapse `+0x388` and `+0x3A0` into one Rust concept without first preserving the `GetFLH` receiver distinction.

## Remaining Uncertainty

- No runtime pixel capture was taken for a hull-north/turret-east MTNK. Static evidence is sufficient for the receiver and formula; a capture would only validate projected pixels.
- The exact semantic name of `+0x388` should remain conservative, such as `flh_turret_facing` or `source_turret_facing`, until a broader turret-state investigation names every synchronization branch.

## Stale Doc Replacement Wording

`C:/Users/enok/Documents/ra2-rust-game-docs/GRIZZLY_FLH_BARRELFACING_PROJECTILE_ORIGIN_GHIDRA_REPORT.md` should replace its deferred receiver wording with:

> `TechnoClass::GetFLH @ 0x006F3AD0` has now been rechecked. In the live locomotor branch, the `RateTimer::Current` receiver is `this+0x388`, proven by assembly `0x006F3BCA..0x006F3BD3`. The source rotation is derived as the quantized virtual `+0x2A8` facing minus the quantized current value of `this+0x388`. Stock Grizzly `Fire_At_Target` still uses `BarrelFacing +0x3A0` for the turret fire gate, but `GetFLH` does not read `+0x3A0` directly. Rust fire-origin code therefore needs an explicit source-facing/turret-facing snapshot equivalent to binary `+0x388`, not just body facing and not an unproven direct reuse of `barrel_facing`.

`C:/Users/enok/Documents/ra2-rust-game-docs/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` should clarify the pseudocode line `short turretFacing = RateTimer::Current();` as:

> `short sourceTurretFacing = RateTimer::Current(this+0x388);` The adjacent Grizzly fire gate uses `+0x3A0`, but this `GetFLH` branch samples `+0x388`.

