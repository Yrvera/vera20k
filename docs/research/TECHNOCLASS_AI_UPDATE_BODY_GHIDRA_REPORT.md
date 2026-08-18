# TechnoClass::AI_Update (0x006F9E50) — full body decode — GHIDRA REPORT

**Status:** VERIFIED from the live binary, 2026-06-11. Supersedes the docs-grounded
`TECHNOCLASS_AI_UPDATE_BODY_SYNTHESIS.md` (its UNCHECKED gaps U1–U6 are now resolved).
**Function:** `TechnoClass__AI_Update @ 0x006F9E50`, body `0x006F9E50–0x006FB004`
(`get_function_by_address 0x006F9E50`).
**Method:** `decompile_function 0x006F9E50` + `disassemble_function 0x006F9E50` (full),
`decompile_function 0x007178C0` / `0x0065C7E0` / `0x0062DC50`, `get_function_callees 0x006F9E50`.
**Confidence:** content HIGH (decompiled this session), positions HIGH (single linear asm read),
RNG stream HIGH (ECX read from asm at both call sites).
**For:** Slice S4 (`docs/plans/2026-06-10-s4-techno-common-prepost-design.md`). The damage-particle
RNG truth table (§4) is the **S4b golden input**.

The spine: `UnitClass::AI (0x007360C0) → FootClass::AI (0x004DA530) → TechnoClass::AI_Update
(0x006F9E50) → MissionClass::Mission_Dispatch (0x005B3060)` then locomotor Process (FootClass).

## 0. Key globals (read from asm)

| Global | Addr | Role |
|---|---|---|
| `g_CurrentFrameCounter` | `0x00a8ed84` | frame counter; `frame & 4`, `frame & 0x8000000f`, `frame % Rules+N` |
| `g_RulesClass_Instance` | `0x008871e0` | RulesClass; +0x1700 ConditionYellow (double), +0x1708 ConditionRed, +0x558/+0x55c red-band spawn-chance, +0x560/+0x564 yellow-band, +0x30/+0x38 power heal/drain rate, +0x314/+0x318 ProduceCash, +0x17f4 EMPulseSparkles anim |
| `g_ScenarioClass` (Scen) | `0x00a8b230` | **Scen->Random RNG at Scen+0x218** — the damage-particle stream |
| `g_GameMode` | `0x00a8b238` | 0 = SP/campaign (gates an AI target-clear branch) |

## 1. Pre/post split

`+0xC4` AI-tick counter increment then `Mission_Dispatch` — `0x006fa646`:
```
006fa646 MOV EDX,[ESI+0xc4]; 006fa64e INC EDX; 006fa64f MOV [ESI+0xc4],EDX   ; +0xC4++
006fa655 CALL 0x005b3060                                                     ; Mission_Dispatch
```
Everything before `0x006fa646` is the **pre-mission block**; everything after `0x006fa655` to the
return is the **post-mission block**.

## 2. Body map (verified, in address order)

### Pre-mission block (entry → 0x006fa646)
| Addr | Step | Field / gate | RNG | Early-ret |
|---|---|---|---|---|
| 006f9e5b | clear one-shot flag | `+0x431` | — | — |
| 006f9e6c | turret-anim looping sound | gate `Type+0xCD5`; `AnimClass__UpdateLoopingSound` | — | — |
| 006f9eaf | `TechnoClass__UpdateTemporalVisual` (chrono visual) | — | — | — |
| 006f9eb6 | `TechnoClass__UpdateGapVisual` | — | — | — |
| 006f9ebb | voc handle validate/play | `+0x4f0/+0x4f4` | — | — |
| 006f9f0d | timer `+0x29c` countdown (gate `+0x298`) → on expiry clear target `vtable+0x3c8(0)` + `Assign vtable+0x1e8(0xf or 5)` by `Owner[0x1ec]` | `+0x298/+0x29c` | — | — |
| **006f9f6e** | **health smoothing `+0x70`**: snap-down to Health (`+0x6c`) if `+0x70>Health`; on `frame&4` & `+0x70<Health` → `+0x70++` (clamp to ≥ -0x1e) | `+0x70` (display), `+0x6c` (real Health) | — | — |
| 006f9f9f | turret-anim/sound block | gate `Type+0xCA1`; `+0x4a0/+0x49c`, AnimClass detach/play | — | — |
| 006fa054 | low-power EVA voice on volume-category change | `+0x13c`, `Volume__GetCategory`, `VoxClass__PlayEVA` | — | — |
| 006fa14b | **ProduceCash** (Oil Derrick): `frame % Rules+0x314 == 0` → `Spend_Money`+`Add_Credits` | gate `+0x1d0`, `Type+0x5ed` | — | — |
| 006fa1c5 | mind-control link cleanup | `+0x1cc/+0x1d4` | — | — |
| 006fa224 | **RockingUpdate**: `vtable+0x298` gate → `vtable+0x41c` | — | — |
| **006fa23c** | **EARLY-RETURN B**: `if ([ESI+0x90] (IsAlive) == 0) return` (jumps 0x006faffd) | `+0x90` IsAlive | — | **YES** |
| 006fa24a | "Behind" hidden-object marker create/destroy | gate `Type+0x724` (CanBeHidden) + `FUN_00487e00`; marker `+0x12c`; `FUN_0070f1d0` | — | — |
| 006fa30c | target validation (ally/dock/bunker) → may clear target `vtable+0x3c8(0)` | `+0x11c`, `+0x2b4` Target | — | — |
| 006fa472 | periodic target re-validate (`frame&0x8000000f==0`, skip missions 8/0x11) | `+0xac` mission, `+0x2b4` | — | — |
| 006fa4d1 | `Type+0xCA2` → `FUN_0070ed10` ×2 (anim) | — | — | — |
| 006fa4fb | Gattling stage visual `+0x124` | gate `Type+0x810`; `+0x2f8/+0x2f4/+0x2ec`, `Type+0x808` | — | — |
| 006fa5be | `FUN_004a5150` → `FUN_004a5360` (spyplane/garage check) | — | — | — |
| 006fa5d6 | timer `+0x41e` decrement | `+0x41e` | — | — |
| 006fa5e8 | passive-acquired target clear (`+0x50c` set & mission in {0,7,0xd,0xe,0x10,0x12,0x13,0x14,0x16,0x17,0x1c,0x18}) → `vtable+0x3c8(0)` | `+0x50c`, `+0xac` | — | — |

### `0x006fa646`: **+0xC4++ → Mission_Dispatch (0x005b3060)** ← SPLIT

### Post-mission block (0x006fa65a → return)
| Addr | Step | Field / gate | RNG | Early-ret |
|---|---|---|---|---|
| 006fa65a | **Passive/opportunity acquire**: scan-timer `+0x180/+0x188`; gate `vtable+0x4c4`; missions `{2,0xa,5}` (Move/Harvest/Guard) AND `FUN_00709290` (OpportunityFire `Type+0x6AF`) → set `+0x4fc=frame`, scanner `vtable+0x39c`, on target change `+0x50c=1` | `+0x180/+0x188/+0x4fc/+0x50c/+0xac` | **NONE** | — |
| 006fa6f5 | Ivan/C4 bomb detonate | `+0x38/+0x81`, `BombClass` | — | — |
| 006fa717 | **SlaveManager** AI `(*(+0x2d8))[+0x5c]()` | `+0x2d8` | (own body) | — |
| 006fa726 | **CaptureManager** `CaptureManagerClass__Update` | `+0x2bc` | (own body) | — |
| **006fa735** | **EARLY-RETURN E (U3)**: `if ([ESI+0x90] (IsAlive) == 0) return` | `+0x90` IsAlive | — | **YES** |
| 006fa743 | **Self-heal**: `vtable+0x294` gate → `Health(+0x6c)++`; if recovered ≥ ConditionYellow → destroy smoke `+0x310` | `+0x6c`, `+0x310` | — | — |
| 006fa793 | **power heal/drain**: WhatAmI/strength gates; `frame % Rules+0x30/+0x38`; heal/damage Health via `HouseClass` power output/drain | `+0x6c`, `+0x21c` Owner | — | — |
| 006fa941 | `vtable+0x410(0)` (update) | — | — | — |
| 006fa94c | **SpawnManager** AI `(*(+0x2d0))[+0x5c]()` | `+0x2d0` | (own body) | — |
| 006fa95b | **Cloak** visual: `CloakState(+0x220)==0` uncloaked & visible → `vtable+0x420`; `==2` cloaked & not-visible → `vtable+0x420` | `+0x220` | — | — |
| 006fa9d8 | target-clear (SP-AI, no weapon range, not ally) | `+0x2b4`, `GetWeaponRange` | — | — |
| 006faaef | target validation (WhatAmI/sight) → `vtable+0x3c8(0)` | `+0x2b4` | — | — |
| 006fabb8 | **timer-cluster / unload accumulator**: WhatAmI==6 (building) **skips** (`JZ 0x006fac31`); else timer `+0x100/+0x108/+0x10c/+0x110` → accumulate `+0xf8` | `+0xf8/+0x100..` | — | — |
| 006fac31 | anim stage change `+0xf0`: `StageClass__Stage_Changed`→`vtable+0x124(2)`; building anim-facing dirty | `+0xf0` | — | — |
| **006facd1** | **DAMAGE-PARTICLE spawn** (see §4) | gate `Type+0xC8F` + `<ConditionYellow` + `vtable+0x1c8>-10` + `+0x308==0` + Spark-list | **Scen->Random ×0/1/2** | — |
| 006faf01 | `vtable+0x4a0(0)` (update) | — | — | — |
| 006faf0d | **EMP recovery**: `if (--[ESI+0x504](EMPLockRemaining)==0)`: building→`RestoreOnlineEffects 0x00452410`+radar (`return`); **FootClass→locomotor `+0x674` Unlock `vtable+0x58`** + clear EMPulseSparkle anims | `+0x504`, `+0x674` | — | building branch **YES** (0x006faf81) |

## 3. The three early-returns (corrects the "three early-return" framing)

1. **B** — `0x006fa244`, `[ESI+0x90]` (IsAlive) after RockingUpdate (`vtable+0x41c`). The S4-design
   "step 12 self-heal death" label was **WRONG**: this is the post-rocking IsAlive (rocking's
   wide-amplitude callback can destroy the object). Self-heal (`vtable+0x294`, 0x006fa743) *adds*
   Health — it never kills.
2. **E** — `0x006fa73d`, `[ESI+0x90]` (IsAlive) after SlaveManager + CaptureManager, **before**
   self-heal. This is the post-dispatch IsAlive (U3).
3. **Building EMP-restore return** — `0x006faf81`, building branch of EMP recovery only.

## 4. Damage-particle RNG — the S4b truth table (U2, lockstep-critical)

Block `0x006facd1–0x006faee0`. **Outer gate** (`0x006facd9`): `Type+0xC8F != 0` (emits damage
particles) AND `GetHealthRatio() < Rules+0x1700` (< ConditionYellow) AND `vtable+0x1c8() > -10`.
Then build the filtered list of `DamageParticleSystems` (`Type+0x77c` data / `Type+0x788` count)
where each system's `+0x2b4 == 3` (BehavesLike = Spark) — **no RNG in the list build**.
**Inner gate** (`0x006fadb3`): `+0x308 == 0` (no live damage-particle system) AND filtered count > 0.

| Case | Draws (all **Scen->Random**, Scen+0x218) |
|---|---|
| Outer gate fails | **0** |
| Outer passes, inner fails (`+0x308≠0` or no Spark systems) | **0** |
| Gate passes, prob-roll FAILS | **1** — `RandomRanged(0,0x7ffffffe)` @ `0x006fae24` |
| Gate passes, prob-roll SUCCEEDS | **2** — roll @ `0x006fae24` + list-pick `RandomRanged(0,count-1)` @ `0x006faeb3` |

The prob-roll compares `(double)roll * 0x007e3570 < band` where band = `Rules+0x558/+0x55c`
(red, ratio < ConditionRed `Rules+0x1708`) or `Rules+0x560/+0x564` (yellow). On success:
`operator_new(0x100)`, deterministic offset `FUN_007178c0` (**no RNG** — `IsometricPixelToWorld`+
`Sqrt`, corrects PARTICLESYSTEMCLASS §8.6.1's "random visual offset"), then list-pick draw, then
`ParticleSystemClass__Constructor 0x0062dc50` (**no RNG** — sets up the system; individual
`ParticleClass` draws happen later in the particle-pool tick, a different phase). Result stored at
`+0x308`. (OOM edge: roll succeeds but `operator_new` returns null → 1 draw, no spawn.)

**Stream proof:** both calls `MOV ECX,[0x00a8b230]; (ADD/LEA) ECX,+0x218; CALL 0x0065c7e0`
(`Random__RandomRanged` is `__thiscall`, RNG instance in ECX). The pointer-deref+`0x218` idiom is
`Scen->Random` (g_MainRng would be a direct address `0x00886B88`). **This corrects
`PER_FRAME_RNG_CONSUMPTION_ORDER §3.1`'s "particles (all types) → g_MainRng"**: the AI_Update
damage-particle SPAWN roll is **Scen->Random**, not g_MainRng. (The later ParticleClass-pool draws
may still be g_MainRng; that is a separate phase, not S4b.)

## 5. U1–U6 resolution

- **U1** (pre-block list/order): §2 pre-block table — fully enumerated.
- **U2** (damage-particle draw count/stream/position): §4 — **Scen->Random, 0/1/2, @0x006fae24 + 0x006faeb3.**
- **U3** (post-dispatch IsAlive addr): **E @ 0x006fa73d** (`+0x90`), after CaptureManager.
- **U4** (which in-body steps draw RNG): **only the damage-particle block** (Scen->Random ×0–2).
  Mission_Dispatch + Slave/Capture/Spawn managers draw in their *own* bodies (separate slices);
  self-heal/power/cloak/passive-acquire/EMP/anim: **no RNG**.
- **U5** (`+0x70` render-only): `+0x70` is display-smoothed Health (snap-down + `frame&4` lerp-up);
  all gameplay reads real Health `+0x6c`. No sim gate reads `+0x70` in this function → **render-only**
  (a full cross-function reader scan is the only residual, but the in-body evidence is decisive).
- **U6** (contiguous order): §2 — fully linear, no block reordering surprises.

## 6. S4 implications

- **S4b** must draw from the **Scen->Random**-equivalent Rust stream (NOT the g_MainRng-equivalent),
  ×0/1/2 per §4, at the post-mission position after the anim-stage step. Consumption-only/stream-align
  (user-chosen): reproduce the gate + roll + (on success) the list-pick draw so the Scen->Random
  stream stays bit-aligned; the particle visual stays render-side. The gate is data-dependent —
  model `Type+0xC8F` (emits-damage-particles) + ConditionYellow + `+0x308`-empty + Spark-system-count
  precisely or the count drifts.
- **S4a** flip: the bracket is `pre → +0xC4 → Mission_Dispatch → post`; early-returns **B**
  (post-rocking) and **E** (post-dispatch, `+0x90`) are the two unit IsAlive guards. Most pre/post
  steps are separate systems (EMP/self-heal/cloak/spawn/slave) or absent in Rust — S4a owns ordering
  + the two guards, not relocating each step.
- **S4c** passive-acquire: missions `{2,0xa,5}` + `FUN_00709290` (`OpportunityFire Type+0x6AF`) +
  scanner `vtable+0x39c`, scan-timer `+0x180/+0x188`, **no RNG** — safe as a shadow.
- **EMP** (if/when modeled): `+0x504` decrements per-tick; FootClass branch = locomotor `+0x674`
  Unlock + clear EMPulseSparkle anims. Iron-curtain/temporal are elsewhere (frame-anchored).

## 7. Passive-acquire gate — `FUN_00709290` + `FUN_007091d0` (for S4c)

The mission-`{2,0xa,5}` block (§2, `0x006fa6ae`) calls **`FUN_00709290`** — the gate is NOT just
"`OpportunityFire == true`" (`decompile 0x00709290` / `0x007091d0`):

- **`FUN_007091d0` (base "can-acquire" eligibility, prerequisite):** returns 1 iff `vtable+0x1dc()==0`
  (not in a disabling state), `+0x2dc == 0`, `Type+0xd99 != 0`, not a disabled building, the
  weapon/equip gate `vtable+0x2ac()` is true, NOT capture-managed (`+0x2bc != 0 && FUN_004722a0`),
  and NOT (`vtable+0x330() && IsPlayerControl()`).
- **`FUN_00709290` (OpportunityFire gate):** requires `FUN_007091d0` true, then eligible iff
  **`OpportunityFire` (`Type+0x6af`) set** (Move/Harvest/Guard), **OR mission==Guard(5) with a valid
  weapon** (`vtable+0x3e4`/`+0x3f8`, weapon`+0x150`) **even without OpportunityFire**; plus an AI
  Move-mission-with-team early-eligible special case and a Move+vehicle+`Type+0xd6a` path.
- **Net for S4c:** **Guard-mission units passively acquire (weapon permitting) regardless of
  OpportunityFire**; Move/Harvest require OpportunityFire (with team/type exceptions). The S4c shadow
  gate is therefore `FUN_007091d0` ∧ (OpportunityFire ∨ (Guard ∧ weapon)) over missions `{2,5,10}` —
  **not** a one-field check.
- **UNCHECKED (decode before a field-accurate S4c gate):** vtable slots `0x1dc`/`0x2ac`/`0x330`/`0x3e4`
  semantics, type flags `Type+0xd99`/`+0xd6a`, `+0x2dc`, `FUN_004722a0`. These gate exact eligibility.
