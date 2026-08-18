# TechnoClass::AI_Update (0x006F9E50) — body map — DOCS-GROUNDED SYNTHESIS

> **SUPERSEDED 2026-06-11 by `TECHNOCLASS_AI_UPDATE_BODY_GHIDRA_REPORT.md`** (full live decode;
> U1–U6 resolved). Key corrections from the verified decode: the damage-particle RNG draws from
> **Scen->Random**, not g_MainRng (idiom `[0x00a8b230]+0x218`); `FUN_007178c0` and
> `ParticleSystemClass__Constructor` draw **nothing**; the draw count is 0/1/2; early-return "B" is
> the post-**rocking** IsAlive (the "self-heal death" label was wrong — self-heal only *adds* HP).
> Read the GHIDRA_REPORT, not this file, for implementation.

**Status:** SYNTHESIS, not a from-binary decode. Assembled 2026-06-10 from existing
**verified** Ghidra docs while no live gamemd instance was available. Every landmark below is
individually cited to a verified source doc; the **contiguous ordering between landmarks is
INFERRED from in-function byte-address monotonicity** (a reasonable but not guaranteed
assumption — VC++6 can reorder basic blocks). The live decode (task: `TECHNOCLASS_AI_UPDATE_
BODY_GHIDRA_REPORT.md`, design §9) must VERIFY this map and fill the UNCHECKED gaps.

**Purpose:** unblock Slice S4 design (`docs/plans/2026-06-10-s4-techno-common-prepost-design.md`)
as far as existing verified research allows, and isolate the precise binary-only gaps so the
live decode is a fast verify-and-fill, not a from-scratch read.

**Confidence axes** (per project RE discipline):
- *Content* (what each landmark does): HIGH — each cited to a verified doc.
- *Position* (where it sits in the body): MEDIUM — inferred from byte addresses; not a single
  contiguous decompile read this session.
- *Completeness* (is every step here): LOW — the pre-block step list is partial; only landmarks
  with a published address are placed.

---

## 1. The pre/post split

`UnitClass::AI (0x007360C0) → FootClass::AI (0x004DA530) → TechnoClass::AI_Update (0x006F9E50)`.
Inside AI_Update the body splits at the **Mission_Dispatch call site `0x006FA655`**
(→ `MissionClass::Mission_Dispatch 0x005B3060`), with the `+0xC4` AI-tick-counter increment
immediately before it. Verified: `TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md` §3.2 row 1
(`0x006FA655`); S-design "central per-object spine" (`+0xC4`++ then dispatch, after a large
pre-block).

```
0x006F9E50  entry
   … PRE-MISSION BLOCK (steps ~1–20) …
0x006FA655  +0xC4++  then  Mission_Dispatch(0x005B3060)        ← SPLIT
   … POST-MISSION BLOCK (steps ~23–42) …
0x006FAxxx  return
```

---

## 2. Body map (address-ordered landmarks)

| # | Addr (in-fn) | Step | Work | Field / gate | RNG | Early-ret | Source doc |
|---|---|---|---|---|---|---|---|
| A | ~0x006FA224–236 | pre | **RockingUpdate** (vehicle/ship body rock) | gated `vtable[0x298]` → `TypeClass+0xB0 == 0` | none | — | `BODY_ROCKING` §5 |
| B | 0x006FA23C | pre | **IsAlive early-return #1** ("alive after rocking?") | `this+0x90 == 0` ⇒ return | none | **YES** | `BODY_ROCKING` §5 |
| C | 0x006FA2AE–2D3 | pre | **Behind-hidden-object** marker create/destroy | `CanBeHidden` + hidden-occupancy; marker ptr `+0x12C` | none | — | `BEHIND_HIDDEN_OBJECT_VISUAL_PATH` §7 |
| D | (pre, addr UNCHECKED) | pre | one-shot flag clear, turret-anim loop sound, temporal/chrono visual, gap visual, **cloak** tick/auto-recloak/visual, **`+0x70` health smoothing**, target validation, **SpawnManager/SlaveManager** AI (`vtable+0x5C` on `+0x2d0`/`+0x2d8`) | per-system | cloak/spawn may draw — UNCHECKED | — | `TECHNOCLASS_AI_MIGRATION_BOUNDARY` §3.2; `BUILDINGCLASS_UPDATE_AI_TICK` ph11 (order NOT authoritative); `CLOAKING_STEALTH_SYSTEM`; `SPAWN_MANAGER_CLASS` |
| — | **0x006FA655** | 21–22 | **`+0xC4`++ then Mission_Dispatch** | `+0xC4`, `+0xAC` | dispatch handlers may draw | — | `TECHNOCLASS_AI_MIGRATION_BOUNDARY` §3.2 |
| E | (post, addr UNCHECKED) | 27 | **IsAlive early-return #2** (died in dispatch) | IsAlive ⇒ return | none | **YES** | S-design D4 step 27 (addr UNCHECKED) |
| F | 0x006FA699–6C1 | 23 | **Passive/opportunity acquire** | missions **{2,10,5}**; `OpportunityFire` `+0x6AF`; scanner `vtable+0x39C`; writes frame→`+0x4FC`; scan timer `+0x180/+0x188` (45-frame) | — (target set, no fire) | — | `GRIZZLY_OPPORTUNITYFIRE_CONSUMER`; `GRIZZLY_OPPORTUNITYFIRE_FIRST_SHOT_TIMING` |
| G | ~0x006FA6xx | 40 | **Damage-particle (spark/smoke) spawn** | gate `+0x308==NULL` + `HealthRatio<ConditionYellow` + has-`DamageParticleSystems`; band `Rules+0x558/+0x560` | **`g_MainRng`**: spawn-prob roll + `FUN_007178c0` offset + ctor lifetime + list-pick — **COUNT/ADDR UNCHECKED** | — | `PARTICLESYSTEMCLASS` §8.6.1; `PER_FRAME_RNG_CONSUMPTION_ORDER` §3.1 (particles → g_MainRng) |
| H | 0x006FABC4–AC2A | 38 | **Timer-cluster / unload accumulator** | RTTI-6 **buildings skip via `goto`** → units-only | UNCHECKED | — | `TECHNOCLASS_AI_UPDATE_UNLOAD_ACCUMULATOR_ORDERING`; migration §3.2 |
| I | "very end" | 42 | **EMP recovery** | `EMPLockRemaining (+0x504) > 0 ⇒ --` ; on reaching 0: **building** → `RestoreOnlineEffects 0x00452410` + radar recalc; **foot** → locomotor `Unlock (vtable+0x58)` + clear EMP sparkles | none | — | `RADIATION_EMP` §2.6; `TECHNOCLASS_SYSTEMS` §6.3 |

---

## 3. Corrections this synthesis makes to the S4 design / slice contract

1. **EMP recovery is NOT building-only.** `RADIATION_EMP` §2.6 shows a **FootClass branch**
   (locomotor `Unlock` + clear EMP-sparkle anims) alongside the building branch. The S-design
   §9-S4 "step 42 building-EMP-restore … building-only and deferred" is incomplete: the foot
   branch is unit-relevant. (Moot in practice until EMP is modeled in sim — it is **not**, per
   the S4 design §10 inventory — but the slice contract wording should be corrected.)
2. **EMP DOES decrement per tick** (`EMPLockRemaining--`). The S4a "assert no per-tick decrement"
   guard applies to **iron-curtain / force-shield / temporal** (frame-anchored `frame-start<dur`),
   **NOT** to EMP. Do not lump EMP with them.
3. **"Three early-returns" reconciled:** within AI_Update there are (at least) **two** IsAlive
   early-returns — **B** (`+0x90==0` after the lethal pre-block ops incl. rocking/self-heal,
   `0x006FA23C`) and **E** (post-dispatch, step 27, addr UNCHECKED). The "step-12 self-heal
   death" routes through **B**. The "third" point (building EMP-restore) is **not an
   early-return** and additionally sits **outside** AI_Update at `0x0043FE3E` in
   `BuildingClass::Update` (`BUILDINGCLASS_UPDATE_AI_TICK` ph12). So: 2 IsAlive returns inside
   AI_Update; the building post-parent IsAlive is a separate BuildingClass check.
4. **Damage-particle is post-dispatch, adjacent to passive-acquire** (~0x006FA6xx, just after
   `F` at 0x006FA699). It draws from `g_MainRng` (synced) — lockstep-critical — confirming the
   S4b consumption-only/stream-align decision needs the exact draw count from the live decode.

---

## 4. What remains strictly Ghidra-only (the decode must fill)

- **U1** The full **pre-block step list + order** (`D`): one-shot flag, turret sound, cloak,
  `+0x70` smoothing, target validation, spawn/slave — exact sequence + which draw RNG.
- **U2** **Damage-particle exact draw count + stream + position** (`G`) for the three gate cases
  (fail / pass-roll-fail / pass-roll-succeed). **The S4b golden input — get byte-exact.**
- **U3** The **post-dispatch IsAlive early-return address** (`E`, step 27).
- **U4** Whether **cloak / spawn-manager** steps consume `g_MainRng` in-body (would also be S4b
  lockstep surface, not just damage-particle).
- **U5** Confirm `+0x70` is read **only** by render (so S4 keeps it unhashed).
- **U6** Confirm the contiguous order B→C→D→dispatch→E→F→G→H→I (block-reorder check).

---

## 5. Net effect on S4

S4a's authoritative content is **narrower** than "fold steps 1–42": most pre/post systems are
separate existing services (or absent — EMP/self-heal not modeled, design §10). S4a =
**(i)** the pre/post bracket ordering around the S2 dispatch, **(ii)** the IsAlive early-returns
B (pre) and E (post, step 27 — the implementable one). S4b = the damage-particle `g_MainRng`
consumption (gated on U2/U4). S4c = passive-acquire shadow (`F`) on the now-parsed
`OpportunityFire`. The live decode verifies §2 and resolves U1–U6.
