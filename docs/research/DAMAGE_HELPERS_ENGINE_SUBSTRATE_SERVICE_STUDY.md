# Damage Helpers — Engine Substrate Service Study & Replacement-Boundary Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics.
**Bar:** active in a standard local skirmish; MP-only / SpecialFlags / TS-legacy behavior is flagged DORMANT or DEAD.
**Scope:** the **damage-application math service** — the pure-ish function family that converts (raw weapon damage + warhead + target armor + distance + attacker/defender vet/country state + immunity gates) into a final integer HP delta, plus the health-subtraction / death-state classifier it feeds. It is master-TODO item #5 (combat/projectile/warhead pipeline). It does NOT own projectile flight, target acquisition, retaliation scheduling, or warhead *side effects* (Temporal/EMP/MindControl state machines) — those are adjacent systems; this study touches them only where the damage number depends on them.

> **2026-07-13 active-binary correction (supersedes R8/D10 and any conflicting
> attacker labels below):** `disassemble_function(address="0x006fdd50",
> program="gamemd.exe")` proves the positive gate at
> `0x006fe32f..0x006fe331`, exact `(house * unit) * integer Damage` grouping at
> `0x006fe33d..0x006fe34d`, and special-zero rejoin rather than return at
> `0x006fe328..0x006fe3df`. Ordinary non-positive and special-zero values can
> continue through `0x006fe3e3..0x006fe455`. The later identities are civilian
> garrison → tank bunker → open-topped, as audited in
> `TANK_BUNKER_COMBAT_SURFACE_GHIDRA_REPORT.md`; deploy/gattling is stale label
> drift. The Verses parser correction is also binding:
> `disassemble_function(address="0x0075d590", program="gamemd.exe")`,
> `read_memory(address="0x00847c40", length=128, program="gamemd.exe")`,
> `decompile_function(address="0x00528a10", program="gamemd.exe")`, and
> `disassemble_function(address="0x007caf30", program="gamemd.exe")` prove the
> 0x80-byte bounded read, eleven-`100%%` missing fallback, present-empty skip,
> fixed 11 stores, `strtok` empty-token collapse, and native short-list null
> fault recorded in the corrected D1 gate.

**Provenance / confidence posture.** Originally a synthesis; **Pass 2 (2026-06-04) closed every P0 gate by live full-body disassembly/decompile** of the whole receiver and attacker chain. The load-bearing core is now bit-VERIFIED:
- `ApplyWarheadDamage @ 0x00489180` — `decompile_function 0x00489180` + `disassemble_function 0x00489180` (LIVE-VERIFIED): `ScenarioFlags & 0x20` early-out via `g_ScenarioClass_Instance` (`MOV EAX,[0x00a8b230]; TEST [EAX],0x20`), healing block `(7 < armor_index) - 1 & damage`, the `damage*PAM != damage && cellspread_leptons != 0` falloff branch, **three** `Math__ftol` calls (cellSpreadLeptons, falloff, Verses-product) at 0x004891e4/0x00489220/0x00489244, MaxDamage cap at `[g_Rules+0x16C8]` (`MOV ECX,[0x008871e0]; MOV ECX,[ECX+0x16c8]; CMP EAX,ECX; JL`).
- `TechnoClass::ReceiveDamage @ 0x00701900` — `decompile_function`/`disassemble_function 0x00701900` (LIVE-VERIFIED Pass 2 — was DOC-ONLY in Pass 1). Full receiver pre-pipeline + immunity gate order now read from the body.
- `Fire_At @ 0x006fdd50` (TechnoClassFireAtSpawnsBullet) — `decompile_function` + region disassembly (LIVE-VERIFIED Pass 2 — was DOC-ONLY). Attacker mult order + flag gating now read from the body.
- `FUN_006fdb80` (pre-fire estimate) — `decompile_function 0x006fdb80` (LIVE-VERIFIED Pass 2). Attacker + defender mult order confirmed.
- `ObjectClass::ReceiveDamage @ 0x005f5390` — `decompile_function 0x005f5390` (LIVE-VERIFIED). Overkill clamp, healing, state classify, death credit.
- `HouseClass::GetArmorMultForType @ 0x0050bd30` — `decompile_function 0x0050bd30` (LIVE-VERIFIED): `switch(target->WhatAmI())` → HouseTypeClass(`+0x34`)+0x108 (case 3 Infantry), +0x100 (case 0x10 Aircraft), +0x104 (case 0x28 Building), +0x110 (case 7 & `param_2[0x382]==5` → flying), +0x10c (case 7 default ground), `_DAT_007e2ac8`=1.0 default.
- `VeterancyClass::IsVeteran @ 0x0074ff90` — `decompile_function 0x0074ff90`: `1.0 (_DAT_007e2ac8) <= vet < 2.0`. **Label drift noted:** 2.0f locally mislabeled `_g_BridgeDiag_BothSides_2_0`; it is the elite threshold float at `0x007e37b4`. Value (2.0) verified.
- **ProneDamage (+0xF8) DEAD — VERIFIED by exhaustive byte-pattern sweep** (`search_byte_patterns` over every x87 qword-read encoding of disp32 `f8000000` across the whole image). Method + result in §3 / Pass 2.

A few side-effect/RNG/voice branches inside `0x00701900` and `0x006fdd50` remain DOC-ONLY (cited). The §9 ledger separates bit-VERIFIED from DOC-ONLY. **Default verdict for any unproven equivalence is DRIFT** — no internal-only escape hatch for active damage math, ordering, truncation, or immunity gating.

**Companion:** the in-flight engine-substrate program (master TODO: `docs/plans/2026-05-29-core-engine-substrate-todo.md` §5). This substrate **slots into** that program — it does not invent a parallel architecture. Format mirrors `FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.

---

## Executive Summary

**Verdict: the current Rust damage path is a single-multiply shortcut that diverges from gamemd on at least six player-visible axes, and the divergences compound on every hit.** Rust computes `final = base_damage * verses_pct * falloff_pct / 10000` (one integer multiply chain, `src/sim/combat/combat_aoe.rs::aoe_damage_at_distance` and the direct-hit twin in `src/sim/combat/mod.rs:2205`). gamemd computes a **four-stage `ftol`-truncated** pipeline split across attacker-side (Fire_At) and receiver-side (TechnoClass→ObjectClass→ApplyWarheadDamage), each stage truncating toward zero, with a MaxDamage cap, a country armor multiplier, a veteran/elite firepower-and-armor pair, a minimum-1 floor, and a battery of immunity gates (IronCurtain, TypeImmune, Radiation/Psionic/Poison, AffectsAllies). The single sharpest correctness gap is the **`ProneDamage` trap**: Rust *implements* a prone-damage multiplier (`apply_prone_damage_modifier`, `prone_damage_basis_points`) that gamemd **does not read at all in YR** — so Rust deals 50–70% of correct damage when any warhead hits prone infantry. The proposed replacement is an additive, shadow-first **damage math service** (`sim/combat/damage/` — a pure `DamagePipeline` with explicit attacker-side and receiver-side stages, fixed-point throughout, taking `WarheadType` + an `ArmorClass` index + a `VetCountryMods` value-type) that reproduces the verified contract to the last truncation. Rollout follows the proven Mission/Radio rhythm — shadow → invert hash-invariant → drop shadow asserts → authoritative → `SNAPSHOT_VERSION` bump → parity harness. **The P0 research checkpoint is now CLOSED (Pass 2):** the `ftol` order (`ftol(ftol(lerp)*Verses)`), VeteranArmor=Rules+0x688, VeteranCombat=Rules+0x670, the full Fire_At attacker-mult order, the ordered receiver immunity gates (D11), and the ProneDamage-DEAD claim are all bit-VERIFIED by live disassembly. Pass 2 also surfaced that the receiver country-armor is a **divide** (not a multiply) and folds in per-unit Firepower/Armor multipliers (TechnoClass+0x160/+0x158) the original analysis missed — both are DRIFT in the current Rust. The only residual is a cheap, non-blocking INI check of whether any stock-YR single hit exceeds MaxDamage (keep the cap regardless).

---

## Table of Contents

- §1. Active-YR responsibilities of the damage helper family
- §2. Full inventory (functions, globals, tables, offsets, vtable slots, legacy)
- §3. Active-YR vs INACTIVE/LEGACY (TS) split
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements D1–D24)
- §6. Rust-native replacement boundary
- §7. Old ad hoc Rust logic to retire
- §8. Migration slices + acceptance tests (P0–P8)
- §9. Sources & Verification Ledger

---

## 1. Active-YR responsibilities of the damage helper family

This is what the damage helpers **own** in a normal YR skirmish — the player-observable contract a Rust replacement must reproduce. Each line is the *behavior*, not the C++ structure.

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| R1 | **The armor-vs-warhead-vs-distance kernel** (`ApplyWarheadDamage @ 0x00489180`): given a (possibly already attacker-modified) integer damage, the warhead, the target's armor index (0..10), and the lepton distance from impact, produce the final per-target HP delta — distance falloff → Verses multiply → MaxDamage cap. | YES | LIVE-VERIFIED `decompile_function 0x00489180` (this session) |
| R2 | **The healing gate**: negative damage (`damage < 0`) bypasses falloff/Verses entirely and is **blocked for armor index ≥ 8** (special_1/special_2); otherwise returns the raw negative value for the caller to add back to HP. | YES | LIVE-VERIFIED `0x00489180`: `return (7 < param_4) - 1 & param_1;` |
| R3 | **The global no-damage flag**: `ScenarioFlags & 0x20` set → every call returns 0 (used by certain scenario states). | YES (gate exists) | LIVE-VERIFIED `0x00489180`: `(*g_ScenarioClass_Instance & 0x20)` |
| R4 | **The receiver-side TechnoClass pre-pipeline** (`TechnoClass::ReceiveDamage @ 0x00701900`): country armor multiplier → veteran/elite armor divisor → min-1 floor → TypeImmune → IronCurtain → WarpingOut → ForceShield/bunker → Radiation/Psionic/Poison/AffectsAllies immunity → Psychedelic short-circuit, **then** falls through to ObjectClass. | YES | DAMAGE_MATH §4, RECEIVE_DAMAGE §2 |
| R5 | **The core HP deduction + state classifier** (`ObjectClass::ReceiveDamage @ 0x005f5390` — single function, no separate "core/dispatch" split): calls R1, applies building min-1, caps damage to remaining HP (overkill clamp `else { *dmg = currentHP }`), classifies Yellow (`Strength/2`)/Red (`Rules+0x1708`)/Dead transitions, subtracts HP, fires trigger events, dispatches death via vtable+0xE0/+0xE4/+0xDC. **Also contains the VeinholeMonster (WhatAmI==0xF) `ftol` HP clamp — TS-legacy, do not model.** | YES | LIVE `decompile 0x005f5390` (reviewer, this session) |
| R6 | **The defender country armor multiplier** (`HouseClass::GetArmorMultForType @ 0x0050bd30`): per-target-type (Infantry/GroundUnit/FlyingUnit/Aircraft/Building) float from the *defender's* HouseTypeClass; default 1.0. | YES | LIVE-VERIFIED `decompile_function 0x0050bd30` (this session) |
| R7 | **Veteran/elite damage modifiers**: defender-side ARMOR ability divides incoming damage by `VeteranArmor` = **Rules+0x688** (double, default 1.5); attacker-side FIREPOWER ability multiplies outgoing damage by `VeteranCombat` = **Rules+0x670** (double, default 1.1). Vet level read by `IsVeteran @ 0x0074ff90` (1.0≤v<2.0) / `IsElite @ 0x00750010` (v≥2.0). Ability gates (LIVE): defender ARMOR vet `type+0x29d` / elite `type+0x2af`; attacker FIREPOWER vet `type+0x29e` / elite `type+0x2b0`. | YES | LIVE-VERIFIED `disassemble 0x00701900` (FDIV [Rules+0x688] at 0x007019cb), `disassemble 0x006fe3c8` (FMUL [Rules+0x670]) |
| R8 | **The attacker-side Fire_At damage build** (`Fire_At @ 0x006fdd50`): Wave/special stores zero; otherwise raw `weapon+0xa4 Damage` enters country/per-unit and veterancy scaling only when strictly positive. The first conversion groups `(HouseClass+0x188 country FirePower × TechnoClass+0x160 per-unit FirepowerMult) × integer Damage`; enabled veterancy converts separately. Civilian-garrison, tank-bunker, then open-topped stages remain reachable after non-positive/special zero and each converts separately. | YES | LIVE-VERIFIED `disassemble_function 0x006fdd50`, especially `0x006fe328..0x006fe455`; identities cross-checked in `TANK_BUNKER_COMBAT_SURFACE_GHIDRA_REPORT.md` |
| R9 | **The pre-fire overkill estimator** (`FUN_006fdb80`): re-runs attacker FirePower+VeteranCombat (param_1 attacker, `+0x29e/0x2b0`), then defender GetArmorMultForType + VeteranArmor divisor (param_2, `+0x29d/0x2af`), then R1 at distance 0; subtracts the result from `target.EstimatedHealth` (+0x70) so multiple shooters don't all pile onto one nearly-dead target. | YES | LIVE-VERIFIED `decompile 0x006fdb80` |
| R10 | **Area distribution** (`Apply_area_damage @ 0x00489280`): collects targets within `CellSpread` radius, passes the **same** base damage to each with that target's **individual** distance (falloff is computed per-target *inside* R1, not here), halves distance for in-air aircraft, applies bridge-infantry selection. | YES | DAMAGE_MATH §6 (DOC-ONLY) |
| R11 | **Weapon-selection Verses gate** (used by `SelectWeaponAgainst @ 0x006f3330` and retaliation `0x007087c0`): a weapon whose warhead Verses[targetArmor]==0 is unusable (switch to other weapon); >0 but ≤1% suppresses auto-acquire/retaliation but allows force-fire. | YES | DAMAGE_MATH §9; RECEIVE_DAMAGE §10 cond 11 |

---

## 2. Full inventory

Functions, globals, tables, offsets, vtable slots. Addresses tagged LIVE (re-verified this session) or DOC (cited report).

### 2a. Functions

| Name | Address | Role | Active-YR | Evidence |
|---|---|---|---|---|
| `ApplyWarheadDamage` (a.k.a. `WarheadTypeClass__GetDamage`) | 0x00489180 | The armor/Verses/distance kernel. Sig `int __fastcall(int damage, WarheadType* wh, int armorIndex, int distance)`. | YES | LIVE `decompile 0x00489180` |
| `Apply_area_damage` | 0x00489280 | AoE target collection + per-target ReceiveDamage(vtable+0x16c) dispatch; also bridge/wall/overlay/tiberium destruction + rocking. | YES | LIVE `decompile 0x00489280` |
| `HouseClass::GetArmorMultForType` | 0x0050bd30 | Defender country armor multiplier (type-switched). | YES | LIVE `decompile 0x0050bd30` |
| `TechnoClass::ReceiveDamage` | 0x00701900 | Receiver-side pre-pipeline (country/vet divisors + ordered immunity gates). vtable+0x16C. | YES | LIVE `decompile/disassemble 0x00701900` (Pass 2) |
| `ObjectClass::ReceiveDamage` | 0x005f5390 | **The single** core damage function: calls R1 (`FUN_00489180`), HP deduction, overkill clamp, building min-1, healing+max clamp, Yellow/Red/Dead classify, death credit. | YES | LIVE `decompile 0x005f5390` (reviewer, this session) |
| ~~`ObjectClass::ReceiveDamage` (core) @ 0x005f8c90~~ | — | **REMOVED — WRONG ADDRESS.** `0x005f8c90` is inside `CDFileClass__Constructor` (body 005f8110–005f8cda), not a damage function. There is no separate "core/dispatch" split; the whole pipeline is `0x005f5390`. | n/a | LIVE `get_function_by_address 0x005f8c90` → CDFileClass__Constructor; `0x005f5390` body ends 005f584c (reviewer) |
| `InfantryClass::ReceiveDamage` | 0x005227f0 | Infantry override; adds an alliance gate. Does **NOT** read ProneDamage. | YES | DOC GGI §9.1 |
| `BuildingClass::ReceiveDamage` | 0x00442230 | Building override; wall immunity, dead-already, min-1. | YES | DOC BUILDING_DAMAGE_DESTRUCTION §14 |
| `Fire_At` (attacker-side damage build) | 0x006fdd50 | Positive-only grouped FirePower/per-unit stage, veterancy, then civilian-garrison/tank-bunker/open-topped containment; special zero rejoins containment. | YES | LIVE `disassemble_function 0x006fdd50`, rechecked 2026-07-13 |
| `FUN_006fdb80` (pre-fire estimate) | 0x006fdb80 | EstimatedHealth overkill prevention (attacker+defender mults + R1@dist0). | YES | LIVE `decompile 0x006fdb80` (Pass 2) |
| `SelectWeaponAgainst` | 0x006f3330 | Verses==0 → weapon switch. | YES | DOC DAMAGE_MATH §9 |
| `FUN_007087c0` (retaliation gate) | 0x007087c0 | Verses>0.01 condition among others. | YES | DOC RECEIVE_DAMAGE §10 |
| `IsVeteran` | 0x0074ff90 | `1.0 <= vet < 2.0`. | YES | LIVE `decompile 0x0074ff90` |
| `IsElite` | 0x00750010 | `vet >= 2.0`. | YES | DOC RECEIVE_DAMAGE §11; threshold const LIVE (0x007e37b4=2.0) |
| `Math__ftol` | 0x007c5f00 | Truncate-toward-zero float→int (the rounding boundary). | YES | DOC GGI §8.3 |
| `IsIronCurtainActive` | 0x0041bf40 | `frame - start < duration`. | YES | DOC GGI §9.2 (live read of vtable slot) |
| `IsWarpingOut` | 0x0070c5b0 | reads +0x270. | YES | DOC RECEIVE_DAMAGE §5 |
| `WarheadTypeClass::Detonate` | 0x004690b0 | Top of the damage chain; dispatches Apply_area_damage + non-damage effects. | YES | DOC RECEIVE_DAMAGE §9 |

### 2b. Globals / singletons / constants

| Symbol / address | Meaning | Active-YR | Evidence |
|---|---|---|---|
| `g_ScenarioClass_Instance` (`& 0x20`) | Global "no damage" scenario flag. | YES (gate) | LIVE `0x00489180` |
| `g_RulesClass_Instance + 0x16C8` | MaxDamage cap (constructor fallback 1000; stock merged value 10000). | YES | LIVE `0x00489180`; constructor/read evidence in `GATE_DAMAGE_MAXDAMAGE_CLAMP_RESOLUTION_GHIDRA_REPORT.md` |
| `Rules + 0x1700` | ConditionYellow ratio (double) — gates the damage-smoke particle effect in TechnoClass, **NOT** the Yellow state classify (that uses integer Strength>>1, see D18). | YES | LIVE `disassemble 0x0070282f` (`FCOMP [g_Rules+0x1700]`) |
| `Rules + 0x1708` | ConditionRed ratio (double, ~0.25) — used by ObjectClass Red classify. | YES | LIVE `decompile 0x005f5390` (`(double)MaxHP * [g_Rules+0x1708]`) |
| `Rules + 0x100/0x104/0x108/0x10c/0x110` | (in HouseTypeClass, NOT Rules) Armor{Aircraft,Buildings,Infantry,Units,Aircraft-flying}Mult. See note. | YES | LIVE `0x0050bd30` reads `HouseType+0x34`+these |
| `Rules + 0x8c` | RetaliationDelay (frames). | YES | DOC RECEIVE_DAMAGE §11 |
| `0x007e2224` = 256.0f | Leptons per cell (CellSpread × this). | YES | DOC DAMAGE_MATH §12 |
| `0x007e2ac8` = 1.0f | Veteran threshold. | YES | LIVE `0x0074ff90` |
| `0x007e37b4` = 2.0f | Elite threshold (locally mislabeled `_g_BridgeDiag_BothSides_2_0`). | YES | LIVE (referenced by `0x0074ff90`) |
| `0x007e3808` = 0.01 (double) | Percent→fraction (`100%`→1.0) parse constant. | YES | DOC DAMAGE_MATH §12 |
| `CellSpreadTable @ 0x007ed3d0` | Cells-per-integer-radius `[1,9,21,37,61,89,121,161,205,253,309,369]`. | YES | DOC DAMAGE_MATH §6; mirrored in Rust `cell_spread.rs` |
| `Rules + 0x688` = VeteranArmor (double, default 1.5) | Defender vet/elite armor **divisor** (`FDIV`). | YES | LIVE `disassemble 0x007019cb` (`FDIV double [EAX+0x688]`, EAX=g_Rules) |
| `Rules + 0x670` = VeteranCombat (double, default 1.1) | Attacker vet/elite firepower **mult** (`FMUL`). | YES | LIVE `disassemble 0x006fe3d2` (`FMUL double [ECX+0x670]`, ECX=g_Rules) |
| `Rules + 0xf40` (float) | Occupy/civilian-garrison damage multiplier stage (attacker); older deploy label was drift. | YES | LIVE `disassemble_function 0x006fdd50` plus identity audit in `TANK_BUNKER_COMBAT_SURFACE_GHIDRA_REPORT.md` |
| `HouseClass + 0x188` (double) | Per-country FirePower mult (attacker side, Fire_At). | YES | LIVE `disassemble 0x006fe33d` (`FLD double [ECX+0x188]`, ECX=attacker HouseClass `[ESI+0x21c]`) |
| `TechnoClass + 0x160` (double) | Per-unit FirepowerMultiplier (attacker, folded into FirePower stage). | YES | LIVE `disassemble 0x006fe343` (`FMUL double [ESI+0x160]`) |
| `TechnoClass + 0x158` (double) | Per-unit ArmorMultiplier (receiver, folded into country-armor divisor). | YES | LIVE `disassemble 0x00701957` (`FMUL double [ESI+0x158]`) |

**Note on the country-mult offsets:** `0x0050bd30` reads `*(HouseTypeClass + 0x34-deref + {0x100,0x104,0x108,0x10c,0x110})`. DAMAGE_MATH §3 and RECEIVE_DAMAGE §2 disagree on whether these are HouseTypeClass or Rules offsets; the LIVE decompile this session settles it: **`param_1+0x34` is the HouseTypeClass pointer**, so these are **HouseTypeClass** fields (`Armor*Mult` per-country). The `Rules+0x100..` rows in RECEIVE_DAMAGE §2 are a **mislabel** — same numeric offsets, wrong base class.

### 2c. WarheadTypeClass damage-relevant offsets

| Offset | Field | Used by damage math? | Evidence |
|---|---|---|---|
| +0x98 | Deform | NO (terrain deform, not HP) | DOC DAMAGE_MATH §2 |
| +0xA0 | Verses[11] (double) | **YES — the core armor table** | LIVE `0x00489180` reads `wh + 0xA0 + armor*8` |
| +0xF8 | ProneDamage (double) | **DEAD — never read in YR (VERIFIED)** | LIVE: parsed at `0x0075de31` (`*(double*)(ESI+0xf8)=ReadDouble()`); exhaustive `search_byte_patterns` sweep of every x87 qword-read of disp32 `f8000000` finds ZERO reads of WarheadType+0xF8 — the only `+0xF8` qword reads (0x004689b8/db, 0x00468ad2/f9, 0x004668d9, 0x00467b1a/da3/df0) are **BulletClass velocity-Z** in `BulletClassFireRevealArmAndSubmit`/`BulletClassAiHomingDetonationPath`, not WarheadType (Pass 2). |
| +0x124 | CellSpread (float, cells) | YES (× 256 → lepton radius) | DOC DAMAGE_MATH §2 |
| +0x12C | PercentAtMax (float, default 1.0) | YES (falloff edge) | DOC DAMAGE_MATH §2; LIVE `0x00489180` reads `wh+0x12C` into fVar1 |
| +0x144 | Wall | YES (gates wall/bridge damage) | DOC DAMAGE_MATH §2 |
| +0x156 | Poison | YES (immunity gate) | DOC RECEIVE_DAMAGE §9 |
| +0x16D | Psychedelic | YES (MC short-circuit) | DOC RECEIVE_DAMAGE §9 |
| +0x177 | Radiation | YES (immunity gate) | DOC RECEIVE_DAMAGE §9 |
| +0x178 | PsychicDamage | YES (immunity gate) | DOC RECEIVE_DAMAGE §9 |
| +0x179 | AffectsAllies (default true) | YES (ally gate) | DOC WARHEADTYPECLASS_REINVESTIGATION §3.1 |

### 2d. Legacy / dormant within the family

| Item | Status | Evidence |
|---|---|---|
| **ProneDamage (+0xF8)** | **DEAD DATA in YR — VERIFIED.** Parsed at ReadINI (`0x0075de31`), never read during damage (exhaustive whole-image byte sweep, Pass 2). | LIVE `search_byte_patterns` sweep + `get_function_by_address` on every hit |
| **Deform (+0x98) / DeformThreshold (+0x100)** | Terrain deformation; not an HP modifier. Out of damage-math scope. | DOC DAMAGE_MATH §2 |
| **VeinholeMonster NUKE-survival** (`WhatAmI==0x0F`, health→`ftol(maxHP*0.25)`) | TS-legacy veinhole; not a stock-YR unit. | DOC DAMAGE_MATH §5 step 8 (flag as TS) |
| **Fog-of-war damage darkening** | Not part of this family; flagged here only to confirm it does NOT gate damage. | CLAUDE.md TS note |

---

## 3. Active-YR vs INACTIVE/LEGACY split

| ACTIVE in standard YR skirmish (must reproduce) | INACTIVE / LEGACY (must NOT model) |
|---|---|
| Distance falloff: `lerp(1.0, PercentAtMax, dist/cellSpreadLeptons)` with the `damage*PAM != damage && cellSpread!=0` branch guard | **ProneDamage multiplier** — never read in YR; modeling it deals 50–70% wrong damage |
| Verses[armorIndex] double multiply, applied AFTER falloff | **Deform / DeformThreshold** terrain crater (not HP) |
| Two-stage `ftol` inside the kernel (falloff result truncated, then Verses product truncated) + attacker-side `ftol` per mult | **VeinholeMonster nuke-survival** clamp (`WhatAmI==0x0F`) — TS veinhole, no stock-YR unit |
| MaxDamage cap (`Rules+0x16C8`, 10000) | **Fog-of-war "previously seen" darkening** (separate system; does not gate damage) |
| Healing path: negative damage bypasses falloff/Verses; blocked for armor ≥ 8 | — |
| `ScenarioFlags & 0x20` global no-damage gate | — |
| Country armor mult (`HouseType` per-type, default 1.0) | — |
| Vet/elite ARMOR divisor (÷VeteranArmor, default 1.5) — defender side | — |
| Vet/elite FIREPOWER mult (×VeteranCombat, default 1.1) — attacker side | — |
| Min-1 damage floor (positive hits; buildings always ≥1 unless CanC4) | — |
| Immunity gates: TypeImmune, IronCurtain, WarpingOut, ForceShield/bunker, Radiation/Psionic/Poison, AffectsAllies | — |
| Overkill clamp: damage capped to remaining HP before subtraction | — |
| Yellow/Red/Dead state classification + condition thresholds | — |
| AoE: same base damage to all targets, per-target distance; in-air aircraft distance halved | — |
| Pre-fire EstimatedHealth overkill estimator | — |

---

## 4. Comparison against the current Rust architecture

The damage math is **scattered across `rules/` (parse) and `sim/combat/` (apply) with no single service**, and the apply path is a one-multiply shortcut.

| Concern | Rust location | State | Verdict |
|---|---|---|---|
| Warhead parse (Verses, CellSpread, PercentAtMax, bools) | `src/rules/warhead_type.rs` | Verses parsed as `Vec<u8>` percent (0..200), CellSpread `SimFixed`, PercentAtMax `u8` | Parse OK, but **Verses as u8% loses gamemd's `double` precision** — DRIFT once the multiply order is matched (D5) |
| AoE damage application | `src/sim/combat/combat_aoe.rs::apply_aoe_damage` + `aoe_damage_at_distance` | `base*verses*falloff/10000` single i64 multiply | **DRIFT** — wrong truncation order (D5), no MaxDamage cap (D6), no country/vet mults (D8/D9) |
| Direct (CellSpread=0) hit | `src/sim/combat/mod.rs:2205` `base_damage * verses_pct / 100` | inline twin of the AoE formula | **DRIFT** same as above; also no min-1 floor (D7) |
| Prone modifier | `mod.rs::apply_prone_damage_modifier` + `warhead_type.rs::prone_damage_basis_points` | **fully implemented and applied** | **WRONG — must be RETIRED** (R-2). gamemd never reads ProneDamage in YR (GGI §9.1) |
| Armor index lookup | `mod.rs::armor_index` (`ARMOR_NAMES` table) | correct 11-name order | OK |
| CellSpread table | `src/sim/combat/cell_spread.rs` | counts match gamemd `[1,9,21,...369]` | OK (table parity verified) |
| HP subtraction | `mod.rs:1680` `health.current.saturating_sub(*damage)` | subtracts; **no overkill clamp** to remaining HP first | **DRIFT** — gamemd caps damage to remaining HP (D11); affects kill-credit/overkill estimator parity |
| IronCurtain/ForceShield gate | `mod.rs:1671` `superweapon::invulnerability::is_invulnerable` → fully nullify | exists but coarse | Partial — no TypeImmune, no WarpingOut, no per-warhead immunity (Radiation/Psionic/Poison), no AffectsAllies (D12–D16) |
| Country firepower/armor mult | — | **MISSING** | DRIFT (D8/D10). Note: armor side is a DIVIDE; attacker firepower folds HouseClass+0x188 (country) × TechnoClass+0x160 (per-unit FirepowerMultiplier). |
| Per-unit Firepower/Armor multiplier (TechnoClass+0x160 / +0x158) | — | **MISSING** | DRIFT — these per-unit mults (FirepowerMultiplier / ArmorMultiplier, applied with the country mult) are not modeled at all. |
| Vet/elite combat/armor mult | `combat_weapon.rs` uses veterancy only for *weapon selection* (elite primary swap); **no firepower/armor damage scaling** | **MISSING** | DRIFT (D9/D10). VeteranArmor=Rules+0x688 ÷, VeteranCombat=Rules+0x670 ×; ability gates 0x29d/0x2af (armor) and 0x29e/0x2b0 (firepower). |
| MaxDamage cap | — | **MISSING** | DRIFT (D6) |
| Condition Yellow/Red/Dead classify | `mod.rs:1681` `refresh_building_damage_state_gate` + `apply_fear_from_damage` (uses condition thresholds) | partial — building gate + fear; no unified state-return enum | Partial (D17) |
| Verses weapon-select gate | `combat_weapon.rs::verses_gate` (Blocked/Suppressed/Normal at 0/1/>1) | correct thresholds | OK (D18) |
| `[CombatDamage]` defaults / bridge warheads | `src/rules/combat_damage.rs`, `src/rules/bridge_warheads.rs` | particle/warhead-name parse only | Out of damage-math scope; leave as-is |
| Healing (negative damage) | — | **not handled** (damage is `i32`→`u16`, negatives clamped to 0 in `apply_prone_damage_modifier`) | DRIFT (D2) — no heal path, no armor≥8 heal block |
| Attacker-side Fire_At mults | — | base damage passed raw (+ garrison `OccupyDamageMultiplier` only) | DRIFT (D8/D10) |

**Where logic is scattered ad hoc:** the falloff/Verses math is **duplicated** between `combat_aoe.rs::aoe_damage_at_distance` (AoE) and `mod.rs:2205` (direct hit) — two copies that must stay in sync, and both are the wrong formula. The garrison `OccupyDamageMultiplier` is folded into `base_damage` *before* both paths (`mod.rs:2146`), which is the only attacker-side mult present. Immunity is a single coarse `is_invulnerable` check at the HP-subtraction site (`mod.rs:1671`), divorced from the warhead-specific gates.

---

## 5. gamemd-native BEHAVIOR CONTRACT (testable statements)

This is the heart of the doc: the exact observable semantics any Rust replacement must reproduce. Each `Dn` is a parity assertion.

### 5.1 The kernel — `ApplyWarheadDamage` (R1), LIVE-VERIFIED

**D1 — Early-outs (in order):** return 0 if `damage == 0`, OR `g_ScenarioClass_Instance & 0x20`, OR `warhead == NULL`. (LIVE `0x00489180`.)

**D2 — Healing:** if `damage < 0`: return `(armorIndex > 7) ? 0 : damage`. I.e. special_1(9)/special_2(10) and any armor index ≥ 8 cannot be healed; all other armors heal by the full negative value, **bypassing falloff and Verses**. (LIVE `0x00489180`: `(7 < param_4) - 1 & param_1`.)

**D3 — Distance falloff (positive damage):**
```
cellSpreadLeptons = ftol(CellSpread * 256.0)
if (damage * PercentAtMax != damage)  AND  (cellSpreadLeptons != 0):
    falloff = PercentAtMax*damage + (1 - PercentAtMax)*damage*(cellSpreadLeptons - distance)/cellSpreadLeptons
            = damage * lerp(1.0, PercentAtMax, distance/cellSpreadLeptons)
    falloff = ftol(falloff)            # FIRST interior ftol
else:
    falloff = damage                   # PAM==1.0 OR CellSpread==0 → flat
if (falloff <= 0): falloff = 0         # zero-crossing floor
```
The branch guard is a **float equality** (`damage*PAM != damage`): when PercentAtMax is exactly 1.0 the falloff branch is skipped (flat damage). Distance is **NOT clamped** inside the kernel — the caller (Apply_area_damage) pre-filtered to `distance <= cellSpreadLeptons`. (LIVE `0x00489180` confirms the two-condition branch + interior ftol; falloff algebra from DAMAGE_MATH §1 + WARHEADTYPECLASS §2.2.)

**D4 — Verses multiply:** `scaled = ftol(falloff * Verses[armorIndex])` where `Verses` is a **`double`** at `wh+0xA0 + armorIndex*8`. (LIVE; falloff already an int, re-promoted to float, multiplied by the double, truncated again — SECOND/THIRD ftol.)

**D5 — Truncation order is load-bearing (ASSEMBLY-VERIFIED Pass 2).** From `disassemble_function 0x00489180`, the kernel makes **three** `Math__ftol @ 0x007c5f00` calls, in order:
  1. `0x004891e4` — `ftol(CellSpread * 256.0)` → cellSpreadLeptons (stored `[ESP+0x10]`). `FLD [EDI+0x124]; FMUL [0x007e2224]` precedes it.
  2. `0x00489220` — `ftol(falloff)` → ESI. Reached only if `FCOMP` shows `damage*PAM != damage` (`TEST AH,0x40` false) AND `cellSpreadLeptons != 0` (`TEST ECX,ECX` nonzero). The lerp `FILD/FIMUL/FIDIV/FADD` runs in 80-bit x87 *before* this ftol.
  3. `0x00489244` — `ftol(falloff_int * Verses[armor])`: `FILD [ESP+0x1c]` (the zero-floored falloff int), `FMUL double [EDI+EDX*8+0xA0]`. The double[11] Verses table at +0xA0 with stride 8 confirmed.
So the contract is exactly `ftol( ftol(lerp) * Verses )` — **two truncations on the damage value** (plus one on cellSpreadLeptons). Rust's `base*verses*falloff/10000` single-divide diverges in the last digit (WARHEADTYPECLASS §2.4: 99-dmg 0.5×verses + 0.5 falloff yields 24 vs 25). **DRIFT until matched.** Also Verses is a `double` in gamemd; Rust's `u8` (0..200) loses fractional Verses — DRIFT for any non-integer Verses in INI.

**D6 — MaxDamage cap:** `if (scaled >= Rules+0x16C8) return Rules+0x16C8;`
(constructor/missing-key fallback 1000; stock merged value 10000). Applied to the
*kernel output*, per target. (LIVE.)

### 5.2 Receiver-side pre-pipeline — `TechnoClass::ReceiveDamage` (R4), DOC

Order, for positive damage with `ignoreDefenses==false` — **ASSEMBLY-VERIFIED Pass 2** from `disassemble_function 0x00701900` (gate addresses inline):

**D7 — Country armor divisor, then min-1 floor:** `*dmg = ftol( *dmg / (GetArmorMultForType(target) × TechnoClass+0x158) )`. **CORRECTION (Pass 2): this is a DIVIDE, not a multiply, and folds in a per-unit ArmorMultiplier (TechnoClass+0x158).** From 0x00701945: `CALL GetArmorMultForType; FMUL double [ESI+0x158]; FDIVR [ESP+0x14]; CALL ftol`. So armor-mult > 1 makes the target *tougher* (incoming ÷ mult). Later `CMP [EBX],1; JGE; MOV [EBX],1` (min-1, positive only, 0x007019d8).

**D8 — Country mult source:** `GetArmorMultForType` switches on target `WhatAmI()`: case 3 → HouseType+0x108 (Infantry), case 0x10 → +0x100 (Aircraft), case 0x28 → +0x104 (Building), case 7 with `param_2[0x382]==5` (flying locomotor) → +0x110, case 7 default → +0x10c (ground), default → 1.0. Read from the **defender's** HouseTypeClass (`HouseType+0x34`). (LIVE `0x0050bd30`.)

**D9 — Vet/elite ARMOR divisor (defender), ASSEMBLY-VERIFIED:** after D7, if defender `IsVeteran` (1.0≤v<2.0) with `type+0x29d` (VeteranAbilities ARMOR) set, OR `IsElite` (v≥2.0) with `type+0x29d` OR `type+0x2af` (EliteAbilities ARMOR/incl. carry-over), then `*dmg = ftol(*dmg / VeteranArmor)` where **VeteranArmor = Rules+0x688 (double, 1.5)**. From 0x00701984–0x007019d6: `IsVeteran/IsElite`, byte tests `[EBP+0x29d]`/`[EBP+0x2af]`, then `FILD [EBX]; FDIV double [g_Rules+0x688]; ftol`. Ability offsets + Rules offset now PINNED (were UNCHECKED).

**D10 — Attacker-side build (Fire_At, R8), ASSEMBLY-VERIFIED and corrected 2026-07-13:** Wave/special stores zero and rejoins containment. Ordinary `Damage <= 0` skips country/per-unit/veterancy. Positive damage executes `(HouseClass+0x188 country FirePower × TechnoClass+0x160 unit FirepowerMult) × integer weapon Damage` before one `Math__ftol`, then the enabled VeteranCombat conversion. Civilian-garrison → tank-bunker → open-topped conversions follow and remain reachable for non-positive/special-zero values. See `0x006fe328..0x006fe455`; the older deploy/gattling identities are label drift.

**D11 — Immunity gates (receiver, short-circuit to 0), ASSEMBLY-VERIFIED order** from 0x00701a3b onward:
  1. WarpingOut: `vtable+0x160` true → `FUN_0048a620` warp anim, `*dmg=0`, return 0 (0x00701a3b).
  2. ForceShield/invuln: `vtable+0x1d4` true → `*dmg=0`, return 0 (0x00701aad).
  3. Bunker/wall (WhatAmI path): if occupying-building `field_0x2e4` and `WhatAmI==6` then `warhead+0x146` (Wall) gates; non-building cell-match → `*dmg=0` (0x00701b67–0x00701bf6).
  4. Radiation: `warhead+0x177` && `type+0xd37` (ImmuneToRadiation) → 0 (0x00701bfe).
  5. `warhead+0x178` (PsychicDamage) && `type+0xd36` → 0 (0x00701c31).
  6. Poison: `warhead+0x156` && `type+0xd3b` → 0 (0x00701c64).
  7. `!AffectsAllies` (`warhead+0x179==0`) && attacker present && `IsAlliedWith(attackerHouse, owner)` (`0x004f9a50`) → 0 (0x00701c97).
  8. Psychedelic/MindControl: `warhead+0x16d` → if allied (`0x004f9a50`) return 0; if `type+0xd35` (ImmuneToPsionics... see note) return 0; if `WhatAmI==6` (building) return 0; else `FUN_00489180(armor, NULL-warhead)` (damage-only MC bookkeeping) → return 1 (0x00701cd7–0x00701dc9).
TypeImmune is checked **earlier**, before the armor mults: 0x007019e3 — if attacker present, `type+0xc8c` (TypeImmune) set, attacker WhatAmI == target WhatAmI, and same owner (`+0x21c`), return 0. AffectsAllies default true (constructor +0x179=1).

**D12 — Psychedelic (MindControl) short-circuit:** if `wh.Psychedelic`: return 0 if attacker allied, or target ImmuneToPsionics, or target is a building; else apply MC, return 1. (Damage aspect only — MC state machine is a separate system.)

### 5.3 Core HP deduction — `ObjectClass::ReceiveDamage` (R5), DOC

**D13 — Verses call site:** if `!ignoreDefenses`, `ObjectClass::ReceiveDamage @ 0x005f5390` calls R1 (`FUN_00489180`) with the target's `Armor` index (read via vtable+0x88 → +0x9c) and the impact distance. (This is where D1–D6 actually run.) LIVE-VERIFIED `decompile 0x005f5390` (reviewer): `iVar4 = FUN_00489180(*(undefined4 *)(iVar4 + 0x9c), warhead)`.

**D14 — Building min-1:** a Building without CanC4 always takes ≥1 damage (`if (*dmg < 1) *dmg = 1`). (DAMAGE_MATH §5 step 2.)

**D15 — Zero early-out:** if `*dmg == 0` after armor, return 0 (no state change).

**D16 — Healing apply:** if `*dmg < 0`, `Health -= *dmg` then clamp to `Strength` (max HP). (DAMAGE_MATH §5 step 4.)

**D17 — Overkill clamp:** `if (*dmg >= currentHealth) *dmg = currentHealth;` — damage never exceeds remaining HP. (DAMAGE_MATH §5 step 5.) **Rust currently skips this** (saturating_sub absorbs it for HP, but the *reported damage value* and EstimatedHealth bookkeeping differ).

**D18 — State classification (return code), ASSEMBLY-VERIFIED** (`decompile 0x005f5390`): 0 Unaffected, 1 damaged-no-threshold, 2 ConditionYellow, 3 ConditionRed, 4 NowDead (HP==0), 5 PostMortem (delay-kill / IsAlive==false). **CORRECTION (Pass 2):** Yellow uses **integer `Strength >> 1`** (`iVar3>>1 <= prevHP && (prevHP - dmg) < iVar3>>1`), NOT the `Rules+0x1700` double. Red uses the `Rules+0x1708` double ratio (`MaxHP*[Rules+0x1708] < prevHP && (prevHP - dmg) < that`). The `Rules+0x1700` (ConditionYellow ratio) is read in `TechnoClass::ReceiveDamage` (`FCOMP [Rules+0x1700]` at 0x0070282f) — but only to gate the *damage smoke/leak particle* effect, NOT the Yellow state classify. MaxHP for the test = `vtable+0x88 → +0xa0` (Strength).

**D19 — Death credit:** on HP==0, `RegisterDestruction(attacker)` if `attackerHouse==0 || attackerHouse==attacker.Owner`, else `RegisterDestruction(house)`, then `MarkForDeath`. (DOC RECEIVE_DAMAGE §7.)

### 5.4 AoE distribution — `Apply_area_damage` (R10), DOC

**D20 — Same base, per-target distance:** every target in radius gets the **same** `baseDamage` and its **own** distance; the falloff is computed per-target inside R1, NOT here. (DAMAGE_MATH §6 "Critical finding".)

**D21 — In-air aircraft distance halved:** `if (WhatAmI==2 && IsInAir()) distance /= 2;` before the eligibility test. (DAMAGE_MATH §6.)

**D22 — Eligibility:** target damaged only if `Health>0 && !InLimbo && distance <= maxRadiusLeptons`; attacker self skipped unless C4Warhead or IsSelfHealing; ProtectedFromAOE types skipped; dead/limbo skipped. (DAMAGE_MATH §6.)

### 5.5 Selection / estimation

**D23 — Verses weapon-select gate:** Verses[targetArmor]==0 → weapon cannot engage (switch to other slot); the retaliation gate additionally refuses if Verses ≤ 0.01. (DOC DAMAGE_MATH §9, RECEIVE_DAMAGE §10.) Rust `verses_gate` already encodes 0/1/>1.

**D24 — Pre-fire estimate:** `FUN_006fdb80` re-applies attacker mults + R1 at distance 0 and subtracts from `target.EstimatedHealth (+0x70)`; periodically resynced to real HP. Determines whether overkill-avoidance retargets. (DOC DAMAGE_MATH §7.)

**Parse defaults (D-parse, corrected):** the Verses constructor holds 1.0 for
all 11; a missing key parses the eleven-`100%%` fallback, while present
trimmed-empty retains the constructor values. Percent tokens use
`atoi(str)*0.01`; bare tokens use `strtod` f64. PercentAtMax defaults 1.0;
CellSpread defaults 0; AffectsAllies defaults true. MaxDamage's constructor/
missing-key fallback is 1000 and stock merged rules override it to 10000.
ConditionRed defaults 0.25; VeteranArmor 1.5; VeteranCombat 1.1.

---

## 6. Rust-native REPLACEMENT BOUNDARY design

**Layering:** lives in `sim/combat/damage/` — depends on `rules/` (`WarheadType`, `ObjectType` for armor/Strength) and `util/fixed_math`. **NEVER** depends on render/ui/audio/net. Pure functions over value-types; no entity-store reach-in (callers extract the inputs). All math in `SimFixed`.

### 6.1 Types

```rust
// sim/combat/damage/mod.rs

/// 0..10 armor class index (none..special_2). Newtype over usize to stop
/// raw-int confusion with Verses/percent values.
pub struct ArmorClass(pub u8);

/// Verses stored as fixed-point fraction (gamemd `double`), NOT u8 percent —
/// preserves sub-1% and fractional INI values. Parsed in rules/warhead_type.rs.
pub struct VersesTable { values: [SimFixed; 11] }   // wh+0xA0 double[11]

/// Attacker + defender modifiers gathered by the caller from house/vet state.
/// Defaults all 1.0 → no-op (matches "no country/no vet").
pub struct CombatMods {
    // Attacker side (Fire_At), applied in this order, each ftol-truncated:
    pub attacker_country_firepower: SimFixed, // HouseClass+0x188 (country FirePower)
    pub attacker_unit_firepower: SimFixed,    // TechnoClass+0x160 (per-unit FirepowerMultiplier) — folded with country in one FMUL stage
    pub attacker_vet_combat: SimFixed,        // VeteranCombat=Rules+0x670 if vet+type0x29e / elite+type0x2b0, else 1.0
    pub attacker_civilian_garrison: SimFixed, // OccupyDamageMultiplier, else 1.0
    pub attacker_tank_bunker: SimFixed,       // BunkerDamageMultiplier, else 1.0
    pub attacker_open_topped: SimFixed,       // OpenToppedDamageMultiplier, else 1.0
    // Defender side (TechnoClass::ReceiveDamage), DIVIDE then DIVIDE, each ftol:
    pub defender_country_armor: SimFixed,     // GetArmorMultForType × TechnoClass+0x158 — incoming is DIVIDED by this product
    pub defender_unit_armor: SimFixed,        // TechnoClass+0x158 (per-unit ArmorMultiplier), folded with country in the divide
    pub defender_vet_armor: SimFixed,         // VeteranArmor=Rules+0x688, incoming DIVIDED if vet+type0x29d / elite+type0x2af
}

/// What the receiver-side gates decide before the kernel runs.
pub enum DamageGate { Pass, Nullified, MindControlled }

/// Result of the full pipeline.
pub struct DamageOutcome {
    pub hp_delta: i32,        // negative = heal; positive = damage to subtract
    pub state: DamageState,   // Unaffected/Yellow/Red/Dead/PostMortem
}
pub enum DamageState { Unaffected, Damaged, Yellow, Red, Dead, PostMortem }
```

### 6.2 The kernel (R1) — one canonical implementation, replacing BOTH Rust copies

```rust
/// gamemd ApplyWarheadDamage. Pure. Reproduces D1–D6 incl. double-ftol order.
pub fn apply_warhead_damage(
    damage: i32,
    wh: &WarheadType,
    armor: ArmorClass,
    distance_leptons: i32,
    scenario_no_damage: bool,     // ScenarioFlags & 0x20
    max_damage: i32,              // Rules.MaxDamage
) -> i32 {
    if damage == 0 || scenario_no_damage { return 0; }
    if damage < 0 {                                   // D2 healing
        return if armor.0 >= 8 { 0 } else { damage };
    }
    let cs_leptons = ftol(wh.cell_spread * 256);      // D3
    let falloff = if pam_changes_damage(damage, wh.percent_at_max) && cs_leptons != 0 {
        ftol(lerp_damage(damage, wh.percent_at_max, distance_leptons, cs_leptons))
    } else { damage };
    let falloff = falloff.max(0);                     // zero-crossing floor
    let scaled = ftol(SimFixed::from(falloff) * wh.verses[armor.0]); // D4 (double)
    scaled.min(max_damage)                            // D6
}
```
`ftol` = truncate-toward-zero (`sim_to_i32` already does this). The single kernel is called by BOTH the AoE per-target loop and the direct-hit path — retiring the `mod.rs:2205` duplicate.

### 6.3 Receiver pipeline (R4 + R5)

```rust
/// TechnoClass + ObjectClass receiver side. Gates, then kernel, then classify.
pub fn receive_damage(
    incoming: i32, wh: &WarheadType, target: &TargetDamageView,
    mods: &CombatMods, gates: &ImmunityInputs, ...) -> DamageOutcome
```
where `TargetDamageView` is a caller-built value-type (armor, strength, current_hp, whatami, is_building, canc4, vet level, immunity flags, ally relationship) — keeps the service decoupled from `GameEntity`. Order is D7→D9 (mods) → D11 gates → D13 kernel → D14 building-min-1 → D17 overkill clamp → D18 classify.

### 6.4 Attacker pipeline (R8)

```rust
/// Fire_At damage build. Returns the base damage stored on the projectile.
pub fn fire_damage(weapon_damage: i32, mods: &CombatMods, ...) -> i32
```
Gated behind P0 (order confirmation). Until then, callers pass `weapon.damage` and the receiver applies what it can prove.

### 6.5 Ownership / call sites
- `sim/combat/combat_aoe.rs` calls `apply_warhead_damage` per target inside its existing target loop (replacing `aoe_damage_at_distance`).
- `sim/combat/mod.rs` direct-hit path calls the same kernel (replacing the inline `base*verses/100`).
- The receiver pipeline runs at the `mod.rs:1669` damage-apply phase, replacing the coarse `is_invulnerable` check + `saturating_sub` with `receive_damage` → `hp_delta`/`state`.

---

## 7. Old ad hoc Rust logic to RETIRE / fold into the new service

| # | Retire / fold | File:symbol | Why |
|---|---|---|---|
| R-1 | `aoe_damage_at_distance` (AoE falloff/verses) | `src/sim/combat/combat_aoe.rs` | Wrong truncation order (D5), no MaxDamage (D6); fold into `apply_warhead_damage`. |
| R-2 | **`apply_prone_damage_modifier` + `prone_damage_basis_points`** | `src/sim/combat/mod.rs::apply_prone_damage_modifier`; `src/rules/warhead_type.rs::parse_prone_damage_basis_points` (+ field, + `is_prone_for_damage` callers) | **WRONG behavior.** ProneDamage is dead data in YR (GGI §9.1); applying it deals 50–70% wrong damage to prone infantry. Keep the INI parse only if save round-trip needs it, but **never apply**. |
| R-3 | Inline direct-hit formula `base_damage * verses_pct / 100` | `src/sim/combat/mod.rs:2205` | Duplicate of R-1; fold into the single kernel. |
| R-4 | Coarse immunity nullify | `src/sim/combat/mod.rs:1671` (`is_invulnerable` → full nullify) | Replace with the ordered D11 gate set (TypeImmune, WarpingOut, per-warhead immunity, AffectsAllies). Keep IronCurtain/ForceShield, add the rest. |
| R-5 | Raw `saturating_sub` HP apply | `src/sim/combat/mod.rs:1680` | Add D17 overkill clamp + D18 state return so kill credit / EstimatedHealth / condition transitions are gamemd-shaped. |
| R-6 | `verses_pct: u8` on `SelectedWeapon` / `WarheadType.verses: Vec<u8>` | `src/rules/warhead_type.rs`, `src/sim/combat/combat_weapon.rs` | Migrate to `SimFixed` Verses to preserve fractional/double precision (D5). Keep the `verses_gate` 0/1/>1 thresholds (those read the *percent*, which can stay derived). |

`src/rules/combat_damage.rs` (particle defaults) and `src/rules/bridge_warheads.rs` (warhead names) are **out of scope** — leave them.

---

## 8. Migration SLICES + ACCEPTANCE TESTS

Shadow-first, dependency-ordered, each independently shippable. Mirrors the Mission/Radio rhythm (shadow → invert → authoritative → SNAPSHOT_VERSION bump → parity harness). **No math becomes authoritative before P0 closes.**

**P0 — Research gate — ✅ CLOSED (Pass 2, 2026-06-04).** All five sub-gates resolved by live disassembly/decompile; evidence in §9 "CLOSED in Pass 2":
1. ✅ `ftol` order — three calls, `ftol(ftol(lerp)*Verses)` (D5, `disassemble 0x00489180`).
2. ✅ VeteranArmor = Rules+0x688, VeteranCombat = Rules+0x670.
3. ✅ Fire_At order: positive-only `(FirePower×unit)×Damage` → VeteranCombat;
   then civilian-garrison → tank-bunker → open-topped, including after special zero.
4. ⚠️ MaxDamage exceedance — downgraded from blocking to non-blocking (cap is per-target output; keep it, verify against INI as a cheap follow-up — see §9).
5. ✅ Ability offsets — FIREPOWER 0x29e/0x2b0 (attacker), ARMOR 0x29d/0x2af (defender).
- *Acceptance:* MET — every item LIVE-VERIFIED with Ghidra call cited inline. Authoritative changes (P6/P7) are unblocked except for the optional MaxDamage-INI check.

**P1 — Pure kernel + fold both copies (no behavior flip yet).** Add `sim/combat/damage/` with `apply_warhead_damage`; keep it behind a shadow flag that asserts equality with the existing path for matched inputs.
- *Test:* `kernel_matches_worked_example` — 100 dmg, Verses 0.5 (Heavy), CellSpread 1.0, PAM 0.25, dist 128 → **31** (DAMAGE_MATH §1 worked example).
- *Test:* `kernel_double_ftol_order` — input where single-multiply and double-ftol diverge (99 dmg, 0.5 verses, 0.5 falloff) asserts the **double-ftol** result.
- *Test:* `kernel_healing_blocked_special_armor` — negative damage, armor index 9 → 0; armor index 5 → full negative.
- *Test:* `kernel_pam_one_is_flat` — PercentAtMax==1.0 → flat damage at all distances (branch guard).
- *Test:* `kernel_maxdamage_cap` — Verses 2.0 × huge base → clamped to 10000.

**P2 — Retire ProneDamage application (R-2).** Drop `apply_prone_damage_modifier` from both damage paths; keep parse if needed.
- *Test:* `prone_infantry_takes_full_damage` — GGI M60 (SA warhead) vs prone GI deals the SAME damage as standing GI (was 70% before the fix). This is the sharpest player-visible fix; ship early.

**P3 — Receiver gates (R-4).** Add the ordered D11 gate set as a value-type pipeline; shadow against the coarse `is_invulnerable`.
- *Test:* `type_immune_same_owner_zeroes` — same-type same-owner attack on TypeImmune → 0.
- *Test:* `affects_allies_default_hits_ally`; `affects_allies_no_blocks_ally`.
- *Test:* `radiation_immune_zeroes`, `poison_immune_zeroes`, `psionic_immune_zeroes`.

**P4 — Overkill clamp + state classification (R-5).** Add D17/D18; return `DamageState`; wire condition transitions.
- *Test:* `overkill_clamped_to_remaining_hp` — 500 dmg to a 50-HP unit reports 50, not 500 (affects EstimatedHealth).
- *Test:* `yellow_red_dead_transitions` — crossing `Strength/2` → Yellow, `Strength*0.25` → Red, 0 → Dead.

**P5 — Verses precision migration (R-6).** Verses → `SimFixed`; keep gate thresholds.
- *Test:* `fractional_verses_preserved` — a warhead with `Verses=0.005` (or `1.5%`) on a high-HP target yields the gamemd double result, not the u8-rounded one.

**P6 — Country + vet/elite mults (R-4 cont., D7–D10).** Add `CombatMods`; defender country armor + vet armor divisor + attacker firepower/vet-combat. Order per P0.
- *Test:* `veteran_armor_divides` — vet unit with ARMOR ability takes `ftol(dmg/1.5)`.
- *Test:* `country_armor_mult_applies` — defender HouseType ArmorUnitsMult != 1.0 scales incoming.
- *Test:* `min_one_floor_positive` — sub-1 positive result clamps to 1; building-no-CanC4 also ≥1.

**P7 — Authoritative + SNAPSHOT_VERSION bump.** Drop shadow asserts; make the service the only damage path; bump `SNAPSHOT_VERSION` (damage outputs are hash-relevant).
- *Test:* `damage_path_is_authoritative` — no remaining call to the retired formulas (grep gate in CI).

**P8 — Global parity harness.** Deterministic replay of a scripted skirmish (mixed warheads vs mixed armors at varying distances, vet tiers, country bonuses), assert per-tick state-hash matches a recorded baseline.
- *Test:* `damage_parity_replay` — golden-hash replay (mirrors the Slice-8 global parity harness pattern in recent commits).

---

## Pass 2 — Expansion (2026-06-04): newly-found consumers, globals, gates, and edge cases

Systematic `get_function_callers` / `get_xrefs_to` / full-body disassembly sweep. Everything here is bit-VERIFIED this run with the Ghidra call cited.

### P2.1 — Complete consumer list of the kernel R1 (`ApplyWarheadDamage @ 0x00489180`)
`get_function_callers 0x00489180` → **exactly 3**:
- `ObjectClass::ReceiveDamage @ 0x005f5390` — the normal armor-vs-warhead path (D13).
- `TechnoClass::ReceiveDamage @ 0x00701900` — **NEW: a second kernel call site** inside the **Psychedelic/MindControl** branch (0x00701d64): `FUN_00489180(target.Armor (vtable+0x88→+0x9c), NULL warhead)`. With a NULL warhead the kernel early-outs `warhead==NULL → return 0`, so this computes the MC "damage" as 0 and stores it (`field_0x29c`), returning 1. Damage-number impact: zero. The doc previously documented only the ObjectClass call; this site exists and must be reproduced (MC applies no HP delta).
- `FUN_006fdb80 @ 0x006fdb80` — pre-fire estimate (R9/D24).

### P2.2 — Complete consumer list of the AoE distributor (`Apply_area_damage @ 0x00489280`)
`get_function_callers 0x00489280` → **18 distinct systems** (doc previously named only Detonate):
`WarheadTypeClass__Detonate (0x004690b0)`, `BombClass__Detonate (0x00438720)`, `NukeGroundZero__ApplyDamage (0x004251f0)`, `LightningStorm__GroundStrike (0x0053a300)`, `PsychicDominator__MindControlArea (0x0053b080)`, `Wave_splash_forces (0x0053cbe0)`, `DiskLaserClass__AI (0x004a7340)`, `SuperClass__Launch (0x006cc390)`, `TerrainClass__Take_Damage (0x0071b920)`, `InfantryClass__PerCellProcess (0x00519630)`, `FlyLocomotionClass__Process (0x004cd600)`, `AnimClass__AI (0x00423ac0)`, `AnimClass__Middle (0x00424ce0)`, `VoxelAnimClass__AI (0x00749f30)`, plus `FUN_0048a700`, `FUN_00663030`, `FUN_006e0490`, `FUN_006e2390`. All funnel the SAME per-target dispatch (D20–D22). Material: the damage service's AoE entry is shared across superweapons, animations, terrain, and per-cell processing — not just projectile detonation.

### P2.3 — `Apply_area_damage` internals the doc omitted (LIVE `decompile 0x00489280`)
- **ScenarioFlags&0x20 early-out** ALSO present here (returns true with no damage), mirroring the kernel.
- **In-air aircraft halving** confirmed: `WhatAmI==2 && vtable+0x54 → distance /= 2` (0x00489a59 region), D21 exact.
- **Self-skip** exact: target damaged unless `piVar12==attacker` AND not (`attacker type+0xca0` C4-self) AND not `bVar21` (warhead == `Rules+0xfac`, the AreaFire/special warhead).
- **Eligibility** exact: `Strength(+0x6c→[0x1b]) > 0 && IsAlive([0x1d]) && InLimbo(+0x81)==0 && distance <= maxRadiusLeptons`.
- **NEW globals/tables:** ring-offset tables `DAT_00abd490` (dx) / `DAT_00abd492` (dy) walked per CellSpread ring; count table `DAT_007ed3d0` (the documented CellSpreadTable). Bridge/overlay destruction uses `g_Rules+0xff0` (the bridge-destroyer warhead), `g_Rules+0xfa8` (chain-detonation warhead, recursive `Apply_area_damage` call), `g_Rules+0x1740` (bridge-destroy random chance), `g_Rules+0x1734` (spread-angle). Wall/tiberium: `warhead+0x144` (Wall), `warhead+0x145/0x146/0x147`, overlay flags `+0x2a8/0x2a9/0x2b0/0x2b1`.
- **Rocking/shake:** `uStack_68 = areaFraction * 0.01`, capped at `_DAT_007e3cc8` (→4.0), then `vtable+0x3d8(coord, force)` rocks each nearby techno if `warhead+0x14e` (Rocker) set and force > `_DAT_007e5138`. This is the Deform/rocking path — distinct from HP damage, flagged for the rocking subsystem not this one.

### P2.4 — ProneDamage-DEAD: sweep method + result (the load-bearing P2 gate)
**Method:** `search_byte_patterns` (whole image, gamemd.exe) for every x87 instruction that reads a **qword (double)** at displacement `+0xF8` (disp32 little-endian `f8 00 00 00`), across all base registers and the relevant float opcodes:
`DD {80,81,82,83,85,86,87} f8000000` (FLD /0), `DC {88..8f} f8000000` (FMUL /1), `DC {90..97}` (FCOM /2), `DC {98..9f}` (FCOMP /3), `DC {A0..A7}` (FSUB /4), `DC {B0..B7}` (FDIV /6).
**Result:** the ONLY matches are `DD 83 f8000000` (4 hits) and `DD 85 f8000000` (4 hits) and `DC 8b f8000000` (2 hits) — all inside `BulletClassFireRevealArmAndSubmit @ 0x00468670` and `BulletClassAiHomingDetonationPath @ 0x004666e0`, where `*(double*)(param_1 + 0x3e)` = **BulletClass velocity-Z** (a 3-vector at +0xE8/+0xF0/+0xF8 normalized via Sqrt). `get_function_by_address` on each hit confirmed the owning function and `decompile_function 0x00468670` confirmed the receiver is BulletClass, not WarheadType. **Zero reads of WarheadType+0xF8 (ProneDamage) exist anywhere.** Combined with the parse-only write at `0x0075de31`, ProneDamage is conclusively dead data in YR. **R-2 / P2 are unblocked and VERIFIED.**

### P2.5 — Receiver-side facts beyond D7–D18
- **Bullet/Ammo cost (NEW, not a damage mult):** `0x00701adb` — if `type+0x6b1` set, the receiver decrements `this->Ammo (+0x2fc)` by `ftol(type+0x6b4 * (dmg/maxHP) * ...)`. Side-effect, not HP; flag for the ammo subsystem.
- **Threat-score feedback (NEW):** after ObjectClass returns, `0x00701e0c` — `HouseClass__Update_Threat_Score(ftol(dmg/maxHP × vtable+0xac), attackerHouse)` updates AI threat. Out of damage-number scope (AI), but it consumes the final clamped `*dmg`.
- **Retaliation gate** (`0x007087c0`) and vet-retaliation overrides (`type+0x29f`, `type+0x2b1`) run after damage; `Rules+0x17ed` (MultiplayerDebug/forced-retaliate) and `Rules+0x8c` (RetaliationDelay) read here. Confirms R11/D23 neighborhood; out of kernel scope.

### P2.6 — Fire_At facts beyond D10
- **Damage forced to 0** when `weapon+0x130 (IsWave/Wave) || weapon+0x129`
  (`0x006fe328`), which skips country/per-unit/veterancy but rejoins the
  containment stages. A Wave/special-spawn weapon can therefore execute those
  zero-valued conversion calls before carrying 0 stored damage.
- **Burst-spread / inaccuracy** uses `Rules+0x1734` (spread) and the Cos/Sin lookup tables — projectile scatter, not damage.
- **DiskLaser early bullet path** (`weapon+0x14a`) returns before the normal bullet allocate — separate projectile type, same damage build already computed in `uStack_a4`.

### P2.7 — Updated retire/slice impact
- §7 R-1..R-6 unchanged in intent, but the new per-unit mults (TechnoClass+0x158/+0x160) and the **divide** semantics of the armor side must be in the P6 `CombatMods` (see updated §6.1 struct).
- New acceptance test for P6: `per_unit_firepower_armor_mult_applies` (FirepowerMultiplier/ArmorMultiplier from the unit, not just the country).
- New acceptance test for P3: `mindcontrol_warhead_applies_zero_hp` (Psychedelic path returns 1 with no HP delta — the second kernel call site, P2.1).
- New acceptance test for P4: `yellow_uses_integer_strength_halved` (Yellow crossing uses `MaxHP>>1`, not the ConditionYellow ratio — D18 correction).

---

## 9. Sources & Verification Ledger

**LIVE-VERIFIED this session (2026-06-04):**
- `ApplyWarheadDamage @ 0x00489180` — `decompile_function 0x00489180`. Confirms D1 (ScenarioFlags&0x20, damage==0, warhead==NULL), D2 (`(7<armor)-1 & damage`), D3 branch guard + interior ftol, D4 Verses read + final ftol, D6 cap at `g_RulesClass_Instance+0x16C8`.
- `HouseClass::GetArmorMultForType @ 0x0050bd30` — `decompile_function 0x0050bd30`. Confirms D8 switch + HouseType(+0x34) offsets 0x100/0x104/0x108/0x10c/0x110, default `_DAT_007e2ac8`=1.0. **Resolves the HouseTypeClass-vs-Rules base-class mislabel in RECEIVE_DAMAGE §2.**
- `VeterancyClass::IsVeteran @ 0x0074ff90` — `decompile_function 0x0074ff90`. Confirms `1.0 <= v < 2.0`. **Label drift recorded:** 2.0f constant locally named `_g_BridgeDiag_BothSides_2_0` (polluted; it is the elite threshold at 0x007e37b4).

**LIVE-VERIFIED Pass 2 (full-body decompile + disassembly, 2026-06-04):**
- `TechnoClass::ReceiveDamage @ 0x00701900` — `decompile_function` + `disassemble_function 0x00701900`. Full receiver pre-pipeline: country-armor DIVIDE (GetArmorMultForType × TechnoClass+0x158, 0x00701945–61), VeteranArmor=Rules+0x688 (0x007019cb), min-1 (0x007019d8), TypeImmune (0x007019e3), WarpingOut/ForceShield/bunker/Radiation/PsychicDamage/Poison/AffectsAllies/Psychedelic gate order (0x00701a3b–0x00701dc9), second kernel call site (0x00701d64), ObjectClass call (0x00701df8), threat-score feedback (0x00701e0c).
- `Fire_At / TechnoClassFireAtSpawnsBullet @ 0x006fdd50` —
  `disassemble_function` rechecked 2026-07-13. Positive gate and grouping at
  `0x006fe32f..0x006fe34d`; VeteranCombat follows; civilian-garrison,
  tank-bunker, open-topped continue through `0x006fe3e3..0x006fe455`; special
  zero rejoins that tail.
- `FUN_006fdb80 @ 0x006fdb80` — `decompile_function`. Pre-fire estimate: attacker FirePower+VeteranCombat (0x29e/0x2b0), defender GetArmorMultForType+VeteranArmor (0x29d/0x2af), R1@dist0.
- `ApplyWarheadDamage @ 0x00489180` — `disassemble_function`. Three ftol (0x004891e4/0x00489220/0x00489244), cap at [g_Rules+0x16c8] (0x0048924f).
- `Apply_area_damage @ 0x00489280` — `decompile_function`. D20–D22, ring tables 0x00abd490/0x00abd492, bridge/wall/overlay/tiberium destruction, rocking.
- `InfantryClass::ReceiveDamage @ 0x005227f0` — `decompile_function`. Alliance/cell gate only; no +0xF8.
- `WarheadTypeClass__ReadINI @ 0x0075de31` — `decompile_function`. ProneDamage parse `*(double*)(ESI+0xf8)=ReadDouble()`; Verses[11] double loop at +0xA0 (atoi×0.01 / strtod per `%`).
- **ProneDamage byte sweep** — `search_byte_patterns` (every x87 qword-read of disp32 f8000000) + `get_function_by_address` on every hit + `decompile_function 0x00468670`. Zero WarheadType+0xF8 reads; all hits are BulletClass velocity.
- Rules offsets pinned: VeteranCombat=Rules+0x670 (FMUL), VeteranArmor=Rules+0x688 (FDIV), Occupy/civilian-garrison multiplier=Rules+0xf40, MaxDamage=Rules+0x16c8; ability bytes 0x29d/0x29e/0x2af/0x2b0.

**LIVE-VERIFIED by REVIEWER (adversarial pass, 2026-06-04):**
- `ObjectClass::ReceiveDamage @ 0x005f5390` — `decompile_function 0x005f5390`. Confirms it is the SINGLE core damage function (calls `FUN_00489180`, HP deduct, overkill clamp D17, building-min-1 D14, healing+max clamp D16, Yellow=`Strength/2` / Red=`Rules+0x1708` / Dead D18, death credit vtable+0xE0/E4/DC D19, VeinholeMonster WhatAmI==0xF ftol clamp = TS-legacy). **The doc's prior "core @ 0x005f8c90" was WRONG** — `get_function_by_address 0x005f8c90` → `CDFileClass__Constructor`. Corrected in §1 R5, §2a, §5.3 D13.
- `TechnoClass::ReceiveDamage @ 0x00701900` — `get_function_by_address` confirms name+entry. (body not re-decompiled this pass; receiver-gate order D7–D12 remains DOC-ONLY.)
- `Fire_At @ 0x006fdd50` — `get_function_by_address` → `TechnoClassFireAtSpawnsBullet`. Identity confirmed; attacker-mult order (D10) still DOC-ONLY/P0.
- `BuildingClass::ReceiveDamage @ 0x00442230`, `InfantryClass::ReceiveDamage @ 0x005227f0`, `IsElite @ 0x00750010`, `Apply_area_damage @ 0x00489280` — `get_function_by_address`/`decompile` confirm name+entry. `InfantryClass::ReceiveDamage` decomp shows ONLY an alliance/cell gate — corroborates "does NOT read ProneDamage." `Apply_area_damage` decomp confirms D20 (same `param_2` base to each target, per-target distance via vtable+0x16c), D21 (`WhatAmI==2 && vtable+0x54` → `dist/2`).
- Constants `0x007e37b4`=2.0f, `0x007e2ac8`=1.0f — `read_memory` confirms (`00000040`, `0000803f`).

**DOC-SOURCED (corroborated ≥2 reports, not re-read live this session):**
- `docs/research/DAMAGE_MATH_GHIDRA_REPORT.md` — §1 (kernel + worked example), §3 (country mult), §4 (TechnoClass pipeline), §5 (ObjectClass HP/state), §6 (Apply_area_damage), §7 (pre-fire estimate), §8 (Fire_At mults), §9 (weapon select), §10 (vet constants), §11 (full flow), §12 (constants).
- `docs/research/RECEIVE_DAMAGE_GHIDRA_REPORT.md` — §1–§11 (TechnoClass::ReceiveDamage flow, immunity gates, state transitions, retaliation, struct offsets).
- `docs/research/WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md` — §2 (kernel pseudocode + §2.4 Rust-drift notes), §3 (immunity gates, AffectsAllies), §6 (flag lookup table), §7 (Rust parity status).
- `docs/research/GGI_GHIDRA_REPORT.md` — §3.6 (kernel summary), §8.3 (ftol rounding mode), **§9.1 (ProneDamage DEAD — the parity trap)**, §9.2 (IsIronCurtainActive inherited).
- `docs/research/OBJECTCLASS_GHIDRA_REPORT.md` §3.3; `docs/research/BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` §13/§14; `docs/research/TERRAIN_CLASS_GHIDRA_REPORT.md` §13.6/§21.2 (RTTI=6 is Building).

**CLOSED in Pass 2 (all formerly UNCHECKED/blocking — now bit-VERIFIED):**
- ✅ `ftol` ordering — `disassemble 0x00489180`: three ftol at 0x004891e4/0x00489220/0x00489244; contract `ftol(ftol(lerp)*Verses)`.
- ✅ VeteranArmor = **Rules+0x688** (FDIV, 0x007019cb); VeteranCombat = **Rules+0x670** (FMUL, 0x006fe3d2).
- ✅ Fire_At attacker-mult order — `disassemble_function 0x006fdd50`
  (`0x006fe328..0x006fe455`): positive-only grouped country/unit stage →
  VeteranCombat → civilian-garrison → tank-bunker → open-topped; special zero
  rejoins the containment tail.
- ✅ VeteranAbilities ability bytes — defender ARMOR vet `type+0x29d` / elite `type+0x2af`; attacker FIREPOWER vet `type+0x29e` / elite `type+0x2b0`. (The earlier "+0x29e vs +0x29d" inconsistency was attacker-vs-defender confusion: FIREPOWER=0x29e/0x2b0, ARMOR=0x29d/0x2af.)
- ✅ ProneDamage-DEAD — exhaustive whole-image byte sweep (see §3/Pass 2).
- ✅ D11 receiver immunity gate order — `disassemble 0x00701900` (see D11).

**Remaining UNCHECKED (non-blocking, lower-leverage):**
- **MaxDamage exceedance in stock YR:** the cap (`Rules+0x16C8`, stock 10000;
  missing-key fallback 1000) is per-target-output. Trigger frequency does not
  change the parity requirement; keep the exact signed inclusive cap.
- **Pre-fire estimate predicate parity:** do not reuse the removed gattling/deploy
  labels. Reconcile each estimate predicate against the civilian-garrison,
  tank-bunker, and open-topped identities before porting it.
- ApplyWarheadDamage's 80-bit x87 lerp intermediates (between ftol #2 inputs) cannot be bit-reproduced by an f64/fixed pipeline in adversarial cases; the ftol at each boundary makes this unobservable at the int result for all sampled inputs, but a boundary-spanning bit test on the lerp is the precise remaining query if last-ULP Verses parity is ever required.

---

## Reviewer follow-ups (adversarial pass, 2026-06-04)

**Patched this pass (clear errors):** the fabricated `ObjectClass::ReceiveDamage (core) @ 0x005f8c90` — wrong address (it is `CDFileClass__Constructor`). There is one core function, `0x005f5390`. Fixed §1 R5, §2a, §5.3 D13, §9 ledger.

**Status after Pass 2 (2026-06-04) — all five reviewer items resolved:**
1. ✅ **ProneDamage-is-DEAD — VERIFIED.** Exhaustive whole-image `search_byte_patterns` sweep of every x87 qword-read encoding of disp32 `f8000000` (FLD `DD /0`, FMUL `DC /1`, FCOM/FCOMP `DC /2,/3`, FSUB `DC /4`, FDIV `DC /6` across all base registers + SIB) finds the ONLY `+0xF8` qword reads are BulletClass velocity-Z in two bullet functions — not WarheadType. Parse-only confirmed at `0x0075de31`. P2 is unblocked. (See §3 / Pass 2.)
2. ✅ **D8 case-7 field:** `param_2[0x382]==5` = byte offset 0xE08; this is the FlyLocomotor discriminator (case-7 → flying vs ground). Switch + 0x100/0x104/0x108/0x10c/0x110 offsets live-verified. The exact INI semantics of +0xE08 are not load-bearing for the mult selection.
3. ✅ **D10 / D11 ORDER — VERIFIED.** Both `0x00701900` and `0x006fdd50` were full-body decompiled AND region-disassembled this pass; immunity gate order (D11) and Fire_At mult order (D10) read from the body with addresses cited.
4. ✅ **Verses double-vs-u8 (D5):** binary side confirmed (`wh+0xA0` double[11], two damage-side ftol). Rust `u8` loss = correctly-stated DRIFT.
5. ✅ **Truncation ORDER:** disassembled; three ftol, contract pinned (D5).

**New corrections Pass 2 surfaced (beyond the reviewer list):**
- D7 country-armor is a **DIVIDE** (`FDIVR`), not multiply, and folds in **TechnoClass+0x158** per-unit ArmorMultiplier — the doc previously said "× GetArmorMultForType" (wrong operator, missing per-unit field).
- D10 FirePower stage folds in **TechnoClass+0x160** per-unit FirepowerMultiplier (× HouseClass+0x188 country FirePower) — doc previously listed only country FirePower.
- D18 Yellow uses **integer Strength>>1**, not the Rules+0x1700 double (that gates the smoke particle only).
- The kernel R1 has a **second caller inside TechnoClass::ReceiveDamage** (the Psychedelic/MC path at 0x00701d64), not only ObjectClass — see Pass 2 §expansion.

**Rust files examined:** `src/rules/combat_damage.rs`, `src/rules/warhead_type.rs`, `src/rules/bridge_warheads.rs`, `src/sim/combat/mod.rs` (armor_index, apply_prone_damage_modifier, lepton_distance_sq_raw, direct-hit fire path ~2140–2256, HP-apply phase ~1669–1697), `src/sim/combat/combat_aoe.rs`, `src/sim/combat/cell_spread.rs`, `src/sim/combat/combat_weapon.rs`.
