# Grizzly Turret Rotation / Body-Fire Split - Ghidra Report

**Target question:** For stock `[MTNK]` Grizzly (`Turret=yes`, `ROT=5`, `Primary=105mm`), does YR aim/fire with the turret independently from the hull, and what should Rust encode?
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** UnitClass turret aiming/firing gate for Grizzly-class turreted vehicles: `UnitClass::AI`, `Fire_At_Target`, `Facing_Update`, `FacingClass` rotation, and stock MTNK INI data.
**Non-goals:** voxel draw FLH matrix, projectile flight, damage, OpportunityFire scanning, and non-unit building turrets except as negative facts.
**Confidence:** High for the scoped UnitClass/Grizzly behavior; Medium for exact `GetFireError` return-code names because the large TechnoClass gate remains only selectively rechecked here.
**Active in YR:** Yes. `[MTNK]` is a stock YR vehicle and the verified functions are reached from live `UnitClass::AI`.

## 1. Overview

Stock Grizzly uses the generic UnitClass turret path. `Turret=yes` does not require the hull to face the target before firing; it causes the fire-facing branch to rotate `TechnoClass+0x3A0` (BarrelFacing) toward the target while leaving the body facing to locomotion. Once the turret-facing gate is satisfied on a later tick, `Fire_At_Target` calls the actual fire virtual without first aligning the hull.

## 2. Class Layout / Key Offsets

| Field | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| Unit type pointer | `UnitClass+0x6C4` | points to UnitType/TechnoType data | constructor writes `param_1[0x1B1]`; `Facing_Update` reads `[ESI+0x6C4]` | Yes |
| `Turret=yes` | `Type+0xCA1` | gates independent turret behavior | `Fire_At_Target @ 0x00736F78..0x00736FAC`; `Facing_Update @ 0x00736BE2..0x00736C03` | Yes |
| `ROT` | `Type+0x71C` | unit ROT byte consumed by turret/barrel FacingClass | constructor `0x00735570..0x0073558D`; `[MTNK] ROT=5` | Yes |
| TurretFacing | `TechnoClass+0x388` | second FacingClass, mirrored/locked in some branches | `Facing_Update @ 0x00736BCF..0x00736BDD`; existing UnitClass turret report | Yes |
| BarrelFacing | `TechnoClass+0x3A0` | fire-aim facing used for single-turret tanks | `Fire_At_Target @ 0x00736FA6..0x00736FAC`; `Facing_Update latch @ 0x00736BF3..0x00736C03` | Yes |
| Rotate latch | `TechnoClass+0x4A0` | 0/1 latch from `BarrelFacing.is_rotating` used by TechnoClass AI for turret rotate sound/anim | `Facing_Update @ 0x00736BF3..0x00736C03`; `TechnoClass::AI_Update @ 0x006F9FA9..0x006F9FC7` | Yes |

## 3. INI Keys

| Key | Stock value | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `[MTNK] Turret` | `yes` | enables independent turret path | `ini/rulesmd.ini:6612`; binary reads `Type+0xCA1` | Yes |
| `[MTNK] ROT` | `5` | turret/barrel step is `5 << 8 = 1280` 16-bit units per frame | `ini/rulesmd.ini:6624`; `SetROT @ 0x004C9680` | Yes |
| `[MTNK] Primary` | `105mm` | selected Grizzly weapon | `ini/rulesmd.ini:6608` | Yes |
| `[105mm] Range` | `5` | normal fire range for the scoped acceptance scenario | `ini/rulesmd.ini:23325..23334` | Yes |
| `[MTNK] Image` / `[GTNK] PrimaryFireFLH` | `GTNK` / `150,0,100` | visual muzzle is Grizzly art; not needed for the fire gate except later fire-origin rendering | `ini/rulesmd.ini:6606`; `ini/artmd.ini:898..903` | Yes |

## 4. Core Logic

`UnitClass::AI @ 0x007360C0` runs `Fire_At_Target` before `Facing_Update`:

1. `0x007365E1`: call `UnitClass::Fire_At_Target`.
2. `0x007365E8`: call `UnitClass::Facing_Update`.

This ordering means the fire decision reads the previous tick/frame facing. A target order cannot both start turret rotation and fire in the same `UnitClass::AI` pass unless the facing was already good before the pass.

When the fire gate returns the facing/aim-needed case, `UnitClass::Fire_At_Target @ 0x00736DF0` checks the type flags:

1. Reads `Type+0xE11` and `Type+0xCA1`.
2. If `Type+0xE11 == 0` and `Type+0xCA1 != 0`, it computes target facing by `FUN_005F3DB0`.
3. It calls `RateTimer__Set` on `ESI+0x3A0` only (`BarrelFacing`), at `0x00736FA6..0x00736FAC`.
4. It does not call body-facing or locomotor facing setters in this turret branch.

For `Turret=yes` Grizzly, that is the live branch. The non-turret/alternate branch beginning at `0x00736FB6` is the branch that involves `ESI+0x388` plus `ESI+0x3A0`; Grizzly should not be modeled from that path for normal tank aiming.

`FacingClass` is timer-based, not a hand-stepped per-render-frame turn:

1. `SetROT @ 0x004C9680`: if input `> 0x7E`, store `0x7F`; then store `(byte << 8)` at `FacingClass+0x14`.
2. `RateTimer__Set @ 0x004C9220`: snapshot animated current into Prev, write new destination, set start frame to `g_CurrentFrameCounter`, and duration to `abs((short)(dest-prev)) / ROT`.
3. `RateTimer__Current @ 0x004C93D0`: returns destination immediately if expired, ROT is zero, duration is zero, or `abs(diff)/ROT < 1`; otherwise returns `dest - sign(diff) * ROT * remaining`.
4. `CDTimerClass__Remaining @ 0x004C9480`: returns true only while ROT is positive and timer duration remains.

For Grizzly `ROT=5`, the per-frame turret step is `0x0500` 16-bit units, about 7.03 degrees per 15-fps game frame. A 90-degree turn is `0x4000 / 0x0500 = 12` full frames with truncation, so a side target should not be fired at immediately after target assignment.

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| AI ordering | fire gate before facing update | `0x007365E1` then `0x007365E8` | Yes |
| Aim request | turreted fire-facing path sets BarrelFacing, not hull | `0x00736F78..0x00736FAC` | Yes |
| Actual fire | when gate is OK, `vtable+0x3CC` fires without hull-align write in the case-0 path | `0x00736F61..0x00736F6D` | Yes |
| Facing update latch | per-tick latch records whether BarrelFacing is still rotating | `0x00736BF3..0x00736C03` | Yes |
| Techno AI consumer | latch toggles turret rotate sound/anim state | `TechnoClass::AI_Update @ 0x006F9FA9..0x006F9FC7` | Yes |

## 6. Current Rust Implementation Status

Rust already has the important scaffolding:

| Surface | Current status | Delta |
|---|---|---|
| `src/sim/movement/facing_class.rs` | Implements timer-based `FacingClass`, `ROT << 8`, snap-on-small-rotation, retarget snapshot, and `is_rotating`. | Matches the verified generic algorithm. |
| `src/sim/movement/turret.rs` | Drives `barrel_facing` toward attack targets or body facing when idle. | Good directionally; acceptance should pin MTNK `ROT=5` timing, not only fast/slow synthetic ROT. |
| `src/sim/combat/mod.rs` | Fire gate requires `barrel.current(binary_frame) == desired` and `!is_rotating(binary_frame)`. | Good target behavior; add Grizzly-specific side-fire scenario. |
| `src/sim/world/mod.rs` | Combat runs before turret rotation in Phase 5. | Matches `UnitClass::AI` order for this slice. |
| `src/app_instances/units.rs` | Renders body at hull facing and turret/barrel at `barrel_facing`. | Rendering split matches the verified sim split; FLH details are out of this report. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock MTNK data | verified | `rulesmd.ini:6603..6648`; `artmd.ini:898..903` | none |
| `UnitClass::AI` order | verified | decompile plus asm `0x007365E1`, `0x007365E8` | none |
| `UnitClass::Fire_At_Target` turret branch | verified | decompile plus asm `0x00736F78..0x00736FAC` | exact symbolic name for code 2 remains inherited from broader Techno report |
| `UnitClass::Facing_Update` latch | verified | decompile plus asm `0x00736BE2..0x00736C03` | none |
| `FacingClass` timer math | verified | `0x004C9220`, `0x004C93D0`, `0x004C9480`, `0x004C9680` | none |
| `FUN_005F3DB0` facing helper | verified | decompile `0x005F3DB0` | exact screen-coordinate convention should be reused from existing facing docs |
| FLH origin with turret facing | deferred | out-of-scope | use FLH_TURRET_AND_VISUAL_OFFSETS report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-GRZ-TUR-001` - Does `Turret=yes` route Grizzly into a turret-only aim branch? -> Yes; `Type+0xCA1 != 0` branch sets `ESI+0x3A0` only. (evidence: `0x00736F78..0x00736FAC`; `rulesmd.ini:6612`)
- `[RESOLVED] OQ-GRZ-TUR-002` - Does normal UnitClass fire align the hull before firing? -> No hull-align write appears in the scoped turret fire-ready path; actual fire is `vtable+0x3CC` after the gate. (evidence: `0x00736F61..0x00736F6D`; `0x00736F78..0x00736FAC`)
- `[RESOLVED] OQ-GRZ-TUR-003` - What is Grizzly turret ROT timing? -> `ROT=5` becomes `0x0500` 16-bit units/frame. (evidence: `rulesmd.ini:6624`; `0x004C9680`; `0x00735570..0x0073558D`)
- `[RESOLVED] OQ-GRZ-TUR-004` - Does the first tick after acquisition fire? -> Not for a newly misaligned turret; `Fire_At_Target` runs before `Facing_Update`, so the new set affects the next gate. (evidence: `0x007365E1..0x007365E8`)
- `[DEFERRED] OQ-GRZ-TUR-005` - Does FLH world origin use exactly BarrelFacing or another render-facing bucket? (category: out-of-scope; reason: prompt excluded voxel/FLH except where necessary; next-step-if-pursued: use FLH_TURRET_AND_VISUAL_OFFSETS report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Grizzly `Turret=yes` aims `BarrelFacing` independently; hull need not point at target. | `0x00736F78..0x00736FAC`; `rulesmd.ini:6612` | none observed for split; needs Grizzly acceptance | `src/sim/movement/turret.rs`, `src/sim/combat/mod.rs` | Keep hull facing separate from turret facing during attack. | MTNK moving/facing north attacks target east; hull stays on movement/body course while turret rotates east and only then fires. | Do not rotate hull to target as a precondition for turreted fire. |
| Fire decision runs before same-tick turret update. | `0x007365E1` before `0x007365E8` | mostly implemented | `src/sim/world/mod.rs`, combat turret tests | First target-acquisition tick uses previous facing; rotation starts for later frame. | Newly ordered MTNK with target 90 degrees away does no damage on first tick. | Do not call turret rotation before combat for units. |
| `ROT=5` means `0x0500` 16-bit units/frame and 90-degree turn takes about 12 frames before fire is eligible. | `rulesmd.ini:6624`; `0x004C9680`; `0x004C9220` | generic tests exist; MTNK-specific test missing | `src/sim/movement/facing_class.rs`, `src/sim/combat/combat_turret_facing_tests.rs` | Encode Grizzly timing, not synthetic-only ROT. | MTNK north vs east target cannot fire before 12 frames and can fire after rotation completes plus normal gate. | Do not use degrees/tick floating math or `ROT=5` as 5 raw 16-bit units. |

## 10. Negative Facts / Do Not Do

- Do not add an MTNK/Grizzly hardcoded branch; all observed behavior is generic UnitClass plus INI data.
- Do not require hull alignment for `Turret=yes` Grizzly fire.
- Do not model turret rotation as a simple per-Rust-tick angular delta if the binary-frame `FacingClass` state is available.
- Do not use `PrimaryFireFLH=150,0,100` as evidence for fire eligibility; it is a muzzle-origin/render fact, not the gate.
- Do not use the non-turret/body branch at `0x00736FB6..0x0073701C` for stock Grizzly aiming.

## Stale Docs / Follow-up Docs

Replacement wording for `units/allied/MTNK.md` lines 583-585:

> `Turret=yes` is live generic UnitClass behavior, not MTNK-specific code. For stock Grizzly, the fire-facing branch in `UnitClass::Fire_At_Target @ 0x00736F78..0x00736FAC` computes target facing and sets `TechnoClass+0x3A0` (`BarrelFacing`) when `Type+0xCA1` is true. The hull does not have to align before firing; `UnitClass::AI @ 0x007365E1..0x007365E8` runs fire decision before facing update, so a newly misaligned turret starts rotating this tick and can fire only after the `FacingClass` timer reaches the target on a later tick. `[MTNK] ROT=5` stores `0x0500` in the turret/barrel `FacingClass`.

## Sources

- Ghidra decompiled/read-only: `UnitClass::Constructor @ 0x007353C0`; `UnitClass::AI @ 0x007360C0`; `UnitClass::Fire_At_Target @ 0x00736DF0`; `UnitClass::Facing_Update @ 0x00736990`; `RateTimer__Set @ 0x004C9220`; `RateTimer__Current @ 0x004C93D0`; `CDTimerClass__Remaining @ 0x004C9480`; `FUN_004C9680`; `FUN_005F3DB0`; `TechnoClass::AI_Update @ 0x006F9E50`.
- Assembly spot checks: `0x00735570..0x0073558D`, `0x007365E1..0x007365E8`, `0x00736F78..0x00736FAC`, `0x00736F61..0x00736F6D`, `0x00736BE2..0x00736C03`, `0x004C9680..0x004C9692`.
- Existing report cross-check: `docs/research/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini`; `ini/artmd.ini`.

**Status:** COMPLETE for the scoped Grizzly turret/body/fire split.
