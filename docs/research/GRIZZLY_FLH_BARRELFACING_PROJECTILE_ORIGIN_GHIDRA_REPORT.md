# Grizzly FLH BarrelFacing Projectile Origin -- Ghidra Research Report

**Address(es):** `0x006F3AD0`, `0x006FDD50`, `0x00736F78..0x00736FAC`, `0x007365E1..0x007365E8`
**Investigation Mode:** exhaustive-slice attempted; final status partial because no callable live Ghidra MCP tool was exposed in this slot.
**Claimed Scope:** Consolidates existing binary-backed reports for stock Grizzly/`MTNK` FLH source coordinate, muzzle flash source, FLH selection, and turret/body split.
**Non-Scope:** Full projectile trajectory, damage, ROF/burst cadence, building turret fire origins, and runtime pixel capture.
**Confidence:** High for helper identity and shared projectile/muzzle source; Medium for Grizzly-specific barrel-vs-body conclusion because the exact `RateTimer::Current` receiver inside `TechnoClass::GetFLH` was not freshly re-decompiled here.
**Active in YR:** Yes for stock `MTNK` fire path; exact field identity in the `GetFLH` locomotor branch remains deferred.

## 1. Overview

Stock Grizzly uses `[MTNK] Image=GTNK`, `[GTNK] PrimaryFireFLH=150,0,100`, `Turret=yes`, `Primary=105mm`, and `ElitePrimary=105mmE`. Existing binary-backed reports verify that `TechnoClass::Fire_At` calls virtual `GetFLH` before bullet creation and muzzle flash creation, and that the same world/lepton source coordinate feeds projectile, muzzle anim, report sound, and related effects.

The unresolved point for this slot is the exact concrete facing source used by the `TechnoClass::GetFLH @ 0x006F3AD0` locomotor branch for a turreted UnitClass side shot. Prior docs say the branch reads body facing and also subtracts a `RateTimer::Current` value described as a turret angle, but this slot could not re-open Ghidra to prove whether that timer is `TurretFacing +0x388`, `BarrelFacing +0x3A0`, or another facing helper. Therefore this report should not be used as final proof that stock Grizzly FLH follows barrel facing until a live Ghidra pass resolves that receiver.

## 2. Class Layout / Key Offsets

| Field | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| BodyFacing | `TechnoClass+0x370` | body/render facing `FacingClass` | `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`; `TECHNOCLASS_VTABLE_COMPLETE.md` | Yes |
| TurretFacing | `TechnoClass+0x388` | turret facing `FacingClass` | `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` | Yes |
| BarrelFacing | `TechnoClass+0x3A0` | live aim facing for single-turret tanks | `UnitClass::Fire_At_Target @ 0x00736F78..0x00736FAC` | Yes |
| `Turret=yes` | `TechnoType+0xCA1` | routes Grizzly aim to BarrelFacing branch | `rulesmd.ini:6612`; prior Grizzly turret report | Yes |
| Normal FLH slot | `TechnoType + 0x898 + weapon_idx*0x1C + 4/8/0xC` | normal weapon FLH triplet | `TechnoClass::GetFLH @ 0x006F3AD0`; `GetWeapon @ 0x0070E140` | Yes |
| Elite FLH slot | `TechnoType + 0xA94 + weapon_idx*0x1C + 4/8/0xC` | elite weapon FLH triplet if present | `GetWeapon @ 0x0070E140`; helpers `0x007177C0/0x007177E0` | Yes |

## 3. Core Logic

### Fire Origin Helper

Existing FLH report verifies `TechnoClass::GetFLH @ 0x006F3AD0` is the generic unit/techno fire-origin helper. `TechnoClass::Fire_At @ 0x006FDD50` calls virtual `vtable+0xB0` around the `0x006FE260` range and stores the returned `CoordStruct` before bullet allocation/init, muzzle report sound, muzzle anim, lasers, waves, EBolts, particle systems, and bullet trajectory use it.

For normal positive weapon indices, `GetFLH` calls `GetWeapon @ 0x0070E140` and reads the selected slot FLH triplet. For elite units, `GetWeapon` probes the elite slot table and falls back to the normal slot when the elite slot/weapon pointer is absent. Grizzly art has no `ElitePrimaryFireFLH`, so the elite `105mmE` path still uses `[GTNK] PrimaryFireFLH=150,0,100`.

### Facing / Barrel Ambiguity

Prior `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` records the `GetFLH` locomotor branch as:

- `GetBodyFacing()` through vtable `+0x304`/documented body-facing accessor.
- A `RateTimer::Current()` value described in the doc as "turret angle".
- 32-way quantization of both values, with body angle minus that timer-derived angle.
- Matrix translate/rotate/translate, then addition to `GetRenderCoords`.

This is enough to reject a simplistic "screen-only FLH" model and enough to prove that `GetFLH` has a timer/facing interaction beyond `SimFireEvent.facing`. It is not enough, without fresh Ghidra, to prove whether the timer-derived value is the Grizzly `BarrelFacing +0x3A0` or a different facing state.

### Muzzle Flash Selection

`TechnoClass::Fire_At` separately uses `GetTurretFacing_Raw` (`vtable+0x308`) only for 8-directional muzzle anim list selection (`Anim=MGUN-N,...`). Stock Grizzly rookie weapon `[105mm] Anim=GUNFIRE` and elite `[105mmE] Anim=VTMUZZLE` are single anim entries, so this directional anim-index path does not decide the Grizzly source coordinate. The Grizzly muzzle anim still spawns at the `GetFLH` world coordinate.

## 4. INI Keys

| Key | Stock value | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `[MTNK] Image` | `GTNK` | Grizzly reads `[GTNK]` art block | `ini/rulesmd.ini:6606` | Yes |
| `[MTNK] Turret` | `yes` | enables UnitClass independent turret aim branch | `ini/rulesmd.ini:6612`; `0x00736F78..0x00736FAC` | Yes |
| `[MTNK] Primary` | `105mm` | rookie/veteran primary weapon | `ini/rulesmd.ini:6608` | Yes |
| `[MTNK] ElitePrimary` | `105mmE` | elite primary weapon | `ini/rulesmd.ini:6647` | Yes |
| `[GTNK] PrimaryFireFLH` | `150,0,100` | fire-origin triplet used by Grizzly art | `ini/artmd.ini:898..903` | Yes |
| `[GTNK] ElitePrimaryFireFLH` | absent | elite falls back to normal FLH slot | absence in `ini/artmd.ini:898..903`; `GetWeapon @ 0x0070E140` fallback | Yes |
| `[105mm] Anim` | `GUNFIRE` | non-elite muzzle anim | `ini/rulesmd.ini:23325..23333` | Yes |
| `[105mmE] Anim` | `VTMUZZLE` | elite muzzle anim | `ini/rulesmd.ini:24786..24796` | Yes |

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Fire gate | `UnitClass::Fire_At_Target` sets `BarrelFacing +0x3A0` for turreted Grizzly when aim is needed | `0x00736F78..0x00736FAC`; prior Grizzly turret report | Yes |
| Fire ordering | fire decision occurs before same-tick `Facing_Update` | `UnitClass::AI @ 0x007365E1` then `0x007365E8` | Yes |
| Fire source | `TechnoClass::Fire_At` calls `GetFLH` before bullet/muzzle/sound creation | `0x006FDD50`, vtable `+0xB0` call around `0x006FE260` | Yes |
| Source consumers | bullet, muzzle anim, report sound, and special effects share the returned source coordinate | `FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`; `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` | Yes |
| Directional muzzle anim | 8-entry anim lists use `GetTurretFacing_Raw`, but Grizzly's `GUNFIRE`/`VTMUZZLE` are single-entry weapon anims | `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`; `rulesmd.ini` | Conditional; not active for stock Grizzly anim selection |

## 6. Current Rust Implementation Status

| Surface | Current status | Delta |
|---|---|---|
| `src/sim/world/mod.rs::SimFireEvent` | carries `facing: u8`, type, weapon slot/id, veterancy, and target; does not carry barrel/turret facing or computed world source | likely insufficient for turreted FLH parity |
| `src/sim/combat/mod.rs` | emits fire events with `snap.facing` only | missing explicit `BarrelFacing`/world source snapshot if binary uses the turret timer in origin |
| `src/app_fire_effects.rs::resolve_fire_origin_from_art` | computes screen-space offset from art FLH and `SimFireEvent.facing`; leaves `rx/ry/z` at attacker position | mismatch with binary world-coordinate source contract |
| `src/app_instances/units.rs` | renders body and turret/barrel separately using `barrel_facing` | visual split exists, but fire-origin event does not consume it |
| `src/rules/flh.rs` / `src/rules/art_data.rs` | parses primary/secondary/elite FLH | Grizzly data available; origin transform/facing source remains incomplete |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock MTNK/GTNK data | verified | `rulesmd.ini`, `artmd.ini` | none |
| `UnitClass::Fire_At_Target` turret branch | verified-inherited | `0x00736F78..0x00736FAC` from Grizzly turret report | none for aim split |
| `TechnoClass::Fire_At` source call | verified-inherited | `0x006FDD50`, `vtable+0xB0` from FLH/Anim reports | none for helper identity |
| `TechnoClass::GetFLH` source-coordinate contract | verified-inherited | `0x006F3AD0` from FLH report | none for world source contract |
| `GetFLH` exact timer receiver for turreted UnitClass | deferred | no live Ghidra MCP exposed in this slot | decompile/asm at `0x006F3AD0` around `RateTimer::Current` receiver and concrete object offset |
| Grizzly projectile/muzzle origin barrel-vs-body conclusion | touched-not-exhausted | cross-doc synthesis only | needs fresh Ghidra receiver proof |
| Rust parity comparison | verified-by-scan | `src/sim/world/mod.rs`, `src/app_fire_effects.rs`, `src/app_instances/units.rs` | implementation design out of scope |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-GRZ-FLH-001 -- What stock art FLH does Grizzly use? -> [GTNK] PrimaryFireFLH=150,0,100; no ElitePrimaryFireFLH override.` (evidence: `ini/artmd.ini:898..903`)
- `[RESOLVED] OQ-GRZ-FLH-002 -- Does elite Grizzly have a distinct elite FLH? -> No; inherited GetWeapon fallback means normal FLH is used when no elite FLH slot exists.` (evidence: `0x0070E140`, `0x007177C0`, `0x007177E0`; `ini/artmd.ini:898..903`)
- `[RESOLVED] OQ-GRZ-FLH-003 -- Which helper computes normal unit fire origin? -> `TechnoClass::GetFLH @ 0x006F3AD0`.` (evidence: `FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-GRZ-FLH-004 -- Does Fire_At use GetFLH before projectile creation? -> Yes; `TechnoClass::Fire_At @ 0x006FDD50` calls vtable `+0xB0` before bullet/muzzle/sound work.` (evidence: `0x006FDD50`)
- `[RESOLVED] OQ-GRZ-FLH-005 -- Does `Turret=yes` aim Grizzly via BarrelFacing? -> Yes for fire gate/aiming; UnitClass sets `+0x3A0` in the turret branch.` (evidence: `0x00736F78..0x00736FAC`)
- `[DEFERRED] OQ-GRZ-FLH-006 -- Does `TechnoClass::GetFLH` read `BarrelFacing +0x3A0` specifically for turreted Grizzly source orientation?` (category: `requires-different-system-context`; reason: no live Ghidra MCP tool was exposed to verify the `RateTimer::Current` receiver/offset in `0x006F3AD0`; next-step-if-pursued: inspect assembly/decompile around the `RateTimer::Current` call and identify whether `this+0x388`, `this+0x3A0`, or another helper is passed)
- `[DEFERRED] OQ-GRZ-FLH-007 -- Does the final transformed origin visibly track barrel facing in a side-shot pixel capture?` (category: `needs-runtime-debugger`; reason: static receiver proof should come first, then runtime capture can validate projection; next-step-if-pursued: freeze hull north/turret east and compare muzzle/bullet start pixel against barrel tip)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fire origin is a world/lepton `CoordStruct` returned by `GetFLH`, not a render-only screen offset. | `0x006F3AD0`, `0x006FDD50` | mismatch: `FireOrigin.rx/ry/z` stays at attacker position | `src/app_fire_effects.rs`, `src/sim/world/mod.rs`, future sim projectile source | Carry or compute deterministic world source before projection. | Grizzly projectile visual/sound/muzzle all start from the same non-center FLH world point. | Do not keep FLH only as `screen_x/screen_y` decoration. |
| Grizzly aim state is split: hull can differ from `BarrelFacing +0x3A0`. | `0x00736F78..0x00736FAC`; `rulesmd.ini:6612` | partial: render has barrel facing, fire event does not carry it | `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/app_fire_effects.rs` | Future fire-origin code must have access to the authoritative turret/barrel facing or proven equivalent. | Hull north, turret east, Grizzly fires after aim completes; origin should be validated against barrel-side shot. | Do not assume `SimFireEvent.facing` is sufficient for turreted units without resolving `GetFLH` receiver. |
| Stock Grizzly elite uses `105mmE` but no distinct `ElitePrimaryFireFLH`; FLH remains `150,0,100`. | `ini/rulesmd.ini:6647`; `ini/artmd.ini:898..903`; `GetWeapon @ 0x0070E140` | likely matches FLH parser fallback; needs Grizzly test | `src/rules/flh.rs`, `src/app_fire_effects.rs` | Resolve elite Grizzly FLH to normal GTNK primary FLH. | Elite MTNK fire event resolves `PrimaryFireFLH=150,0,100` while weapon anim/report come from `105mmE`. | Do not invent an elite FLH from weapon name or `VTMUZZLE`. |

## Negative Facts / Do Not Do

- Do not use `PrimaryFireFLH=150,0,100` as a fire-eligibility gate; it is origin data only.
- Do not treat `GUNFIRE`/`VTMUZZLE` facing selection as proof of projectile origin facing; stock Grizzly does not use an 8-entry directional weapon anim list.
- Do not implement Grizzly FLH as a pure screen offset with `rx/ry/z` left at the attacker cell.
- Do not assume body facing is correct for turreted fire origin until `GetFLH`'s `RateTimer::Current` receiver is resolved.
- Do not add a Grizzly-specific hardcoded branch; all verified pieces are generic UnitClass/TechnoClass plus INI/art data.

## Remaining Uncertainty

The critical unresolved item is the exact receiver/field passed into `RateTimer::Current` inside `TechnoClass::GetFLH @ 0x006F3AD0` for a turreted UnitClass with DriveLocomotion. Existing reports strongly suggest a turret-angle interaction, and separate reports prove Grizzly aims via `BarrelFacing +0x3A0`, but this slot did not have live Ghidra access to prove the final barrel-vs-body origin claim. A follow-up should inspect the assembly/decompile around that call and identify the concrete offset.

## Proposed Rust Test Names

- `grizzly_fire_event_captures_turret_facing_for_flh_origin`
- `grizzly_side_shot_flh_origin_uses_barrel_facing_not_hull_facing`
- `elite_grizzly_uses_primary_flh_when_elite_primary_flh_missing`
- `grizzly_muzzle_projectile_and_report_share_flh_world_source`

## Stale Docs / Follow-up Docs

Replacement wording for the deferred MTNK FLH note:

`Stock Grizzly/MTNK uses [GTNK] PrimaryFireFLH=150,0,100 for both rookie and elite fire because no ElitePrimaryFireFLH is present. Binary-backed reports verify TechnoClass::Fire_At calls TechnoClass::GetFLH and uses the returned world/lepton source for bullet, muzzle anim, report sound, and related effects. The Grizzly-specific question of whether GetFLH's orientation resolves from BarrelFacing+0x3A0 or another facing source remains open until the RateTimer::Current receiver inside GetFLH@0x006F3AD0 is rechecked in live Ghidra. Do not implement a body-facing-only turreted FLH origin from current docs.`

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_VTABLE_COMPLETE.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- Rust scanned: `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/app_fire_effects.rs`, `src/app_instances/units.rs`, `src/rules/flh.rs`, `src/rules/art_data.rs`

**Status:** PARTIAL
