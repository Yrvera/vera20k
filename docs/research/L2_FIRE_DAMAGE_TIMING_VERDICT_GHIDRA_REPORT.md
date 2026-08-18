# L2 Task 0 — Fire Damage Timing: batch-vs-inline verdict (Ghidra)

**Status:** VERIFIED (live Ghidra this session) — gating artifact for AI-shell Plan 2 Slice **L2 Task 3**.
**Date:** 2026-06-04
**Question (from Plan 2 L2 Task 0):** Does a unit's shot land its HP damage **synchronously inside the fire call** (inline-hitscan — an early kill removes a later attacker's target the same tick), or via a **munition that resolves on a later pass** (deferred-projectile)?
**Default verdict until proven (CLAUDE.md burden of proof):** DRIFT. Resolved below.

---

## Verdict (one paragraph)

**DEFERRED-PROJECTILE — uniform across weapon effect types.** The per-tick fire
driver `UnitClass::Fire_At_Target @ 0x00736df0` selects a weapon, runs the
fire-error gate, and on success dispatches the weapon discharge through
`vtable+0x3cc` (`TechnoClass::Fire_At @ 0x006FDD50`); neither applies HP.
`TechnoClass::Fire_At` **allocates and launches a munition** (BulletClass / Wave
/ Laser / DiskLaser / RadBeam) and returns — its callee set contains
`BulletClassAllocate`, `BulletClass__Init`, `BulletClass__SetOwner`,
trajectory math, `Random__RandomRanged`, and the fire sound, but **no**
HP-application function. Actual HP damage is applied later by the **munition's
own AI/detonation pass**: `BulletClass::BulletDetonation @ 0x00468D80`
→ `WarheadTypeClass::Detonate @ 0x004690B0` → `Apply_area_damage @ 0x00489280`
→ per-target `ReceiveDamage`. The callers of the damage-application functions are
detonation/munition-AI/special paths — **`Fire_At` is not among them**.
Therefore, **within a single firing pass no shot changes any target's HP**, so an
early kill cannot remove a later attacker's target that tick. There is **no
inline-hitscan path** and the timing is not mixed-per-weapon for HP application.

---

## Evidence (live this session — cite inline)

| Claim | Evidence (MCP call this session) | Confidence |
|---|---|---|
| `UnitClass::Fire_At_Target @ 0x00736df0` is the per-tick fire **driver**; it selects weapon (`vtable+0x2e4`), runs fire-error (`vtable+0x3c0`), and on `err==0` dispatches discharge via `vtable+0x3cc`; applies no HP. | `decompile_function 0x736df0` | content HIGH; identity HIGH (body matches FIRE_AT_PIPELINE driver shape) |
| `TechnoClass::Fire_At @ 0x006FDD50` allocates/launches a munition (`BulletClassAllocate`, `BulletClass__Init`, `BulletClass__SetOwner`, `WaveClass__Constructor`, `DiskLaserClass__Constructor`, `SpawnLaser/RadBeam/ElectricBolt`) + RNG scatter + fire sound; **no** `ReceiveDamage`/`Apply_area_damage`/`Detonate`/`BulletDetonation` callee. | `get_function_callees 0x6FDD50` | content HIGH; binding of `vtable+0x3cc → 0x6FDD50` INFERRED (see Unverified) |
| `Apply_area_damage @ 0x00489280` is reached from `WarheadTypeClass::Detonate @ 0x004690B0` and munition-AI/special drivers (`DiskLaserClass::AI`, `FlyLocomotionClass::Process`, `LightningStorm::GroundStrike`, `NukeGroundZero`, `PsychicDominator`, `BombClass::Detonate`, `TerrainClass::Take_Damage`, `InfantryClass::PerCellProcess`) — **not** `Fire_At`. | `get_function_callers 0x489280` | HIGH |
| `BulletClass::BulletDetonation @ 0x00468D80` is reached from `BulletClassAiHomingDetonationPath @ 0x004666e0` (the bullet's own AI), **not** `Fire_At`. | `get_function_callers 0x468D80` | HIGH |

Converging prior verified docs (cross-reference, not relied on for the verdict):
`FIRE_AT_PIPELINE_GHIDRA_REPORT.md` §8, `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md` §5/§13
("Is damage immediate? RESOLVED — real damage via `BulletClass::BulletDetonation`; fire tick only updates estimated health"),
`GRIZZLY_105MM_PROJECTILE_ARC_IMPACT_TIMING_GHIDRA_REPORT.md`, `DAMAGE_MATH_GHIDRA_REPORT.md` §11,
`WARHEAD_DETONATE_GHIDRA_REPORT.md`, `RECEIVE_DAMAGE_GHIDRA_REPORT.md`.

---

## What this means for L2 (combat + turret absorb)

1. **The Rust batched-combat model is NOT inline-kill DRIFT — L2 may keep it.**
   The current Rust combat (Plan 2 L2 §1: P2 collects all `damage_events` at
   pre-damage HP → P4 applies → P6 deaths) matches gamemd in the load-bearing
   respect: gamemd also applies **no HP during the firing pass**. L2 Task 3 may
   route Unit `damage_events` into the existing aggregated P4/P6 batch unchanged;
   it does **not** need to thread per-object damage-apply + death into the
   per-object fire step. This was the dominant L2 risk (Plan 2 §8 risk #1) — it is
   now retired in the deferred direction.

2. **Same-tick fire-time write to watch — targeting "estimated health."**
   `TechnoClass::Fire_At` updates a target-side estimated/anticipated-health
   bookkeeping value at fire time (so AI does not over-commit multiple attackers
   to a target already taking lethal incoming fire). This is the **one**
   cross-attacker, same-tick state change in the firing pass. It affects target
   **selection**, not HP/death. The per-object fire walk in L2 must reproduce this
   write **at the same per-object position in live-LOGIC order**, or AI target
   distribution drifts. **Status: doc-sourced (GGI/GRIZZLY/DAMAGE_MATH), NOT
   re-decompiled this session — verify the exact field offset + write site in
   `0x006FDD50` before L2 Task 3 flips.** (YELLOW)

3. **Impact-tick delay is a separate, pre-existing DRIFT — out of L2 scope.**
   gamemd applies HP on a *later* tick (munition flight time); the Rust batch
   applies it the *same* tick (P4). That instant-damage-vs-projectile-flight
   difference is real DRIFT but is **not introduced by L2** and is **not** in L2's
   scope (it is BulletClass-AI migration territory). Do not "fix" projectile
   timing inside L2 — preserve the same-tick batch and flag the delay separately.

4. **Fire consumes RNG (`Random__RandomRanged`) — scatter/inaccuracy.**
   `TechnoClass::Fire_At` draws RNG for projectile scatter. The Rust combat path
   currently consumes **zero** RNG (Plan 2 L2 §7, grep-confirmed), i.e. the
   scatter draw is unmodeled today. L2 (fire+facing only) keeps zero-RNG, but the
   eventual full fire absorb must consume this draw at the matching per-object
   position. Flagged as a downstream gap, not an L2-Task-3 blocker.

---

## Unverified / inferred (YELLOW — verify before relying)

- **`vtable+0x3cc → 0x006FDD50` binding** for the UnitClass vtable is INFERRED
  from the driver decompile + the function-name map, **not** `read_memory`-verified
  this session (per `feedback_vtable_binding_verification`, a one-call
  `read_memory` of the UnitClass vtable slot should confirm it before L2 Task 3).
  The verdict does **not** depend on it: no `Fire_At`-family function calls any
  HP-application function regardless of the exact slot value.
- **Estimated-health field offset + write site** (item 2 above) — doc-sourced.

## Bottom line for the gate

L2 Task 3's damage-application shape is **decided: keep the batched P4/P6 model**
(deferred matches gamemd). The remaining L2 pre-flip verifications are the two
YELLOW items above (vtable slot read; estimated-health write site), both small
and both about *fire-time target selection*, not the damage batch.
