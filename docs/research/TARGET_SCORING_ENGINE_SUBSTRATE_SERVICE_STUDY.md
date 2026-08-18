# Target Scoring — Engine Substrate Service Study & Replacement-Boundary Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics.
**Bar:** active in a standard local skirmish. AI-house base-plan threat use is flagged DEFERRED-AI; per-unit passive/guard/area-guard acquisition (OpportunityFire) IS in scope — it is per-object combat behavior, not house-level AI.
**Confidence posture:** The scoring family (Greatest_Threat / Evaluate_Candidate / Calculate_Threat_Score / Scan_Cell_For_Target) is already heavily decoded by a prior doc family. This study is a **synthesis** into a substrate-service boundary, NOT a fresh decode. I re-verified the load-bearing facts **live this session**: I decompiled `TechnoClass__Calculate_Threat_Score @ 0x0070CD10`, `TechnoClass__Greatest_Threat @ 0x006F8DF0`, `TechnoClass__Evaluate_Candidate @ 0x006F7CA0`, `TechnoClass__Scan_Cell_For_Target @ 0x006F8960`, `TechnoClass__ThreatAvoidance_Modifier @ 0x006F79A0`, `TechnoClass__ShouldRetaliate @ 0x007087C0`, `FUN_004D9920 @ 0x004D9920`, `BuildingClass__IsOccupied @ 0x00458DD0`, `RulesClass__ReadCombatDamage @ 0x0066BBB0`, disassembled `0x0070CD10`, and read memory at the TechnoClass/BuildingClass vtable +0x400/+0x404 slots, the score base const, the avoidance const, and the early-return const. Everything tagged **LIVE-VERIFIED** below was read out of Ghidra this run; everything tagged **DOC-SOURCED** is corroborated by a prior verified report but was not re-read live this session. **Default verdict for any unproven equivalence is DRIFT** — there is no internal-only escape hatch for active per-unit target choice.

**Pass-2 gate closures (2026-06-04, all LIVE this run):** (1) **+0x400 = IsOccupied, NOT Is_Sensor** — base Techno/Unit/Infantry vtable +0x400 is a `return 0` stub (`0x0041BFB0`); BuildingClass overrides it with `IsOccupied @ 0x00458DD0`; +0x404 = `GetHalfFoundationSize @ 0x00458E00` (the doc's prior "Is_Sensor/Get_Sensor_Range" labels were WRONG — there is **no sensor branch**, it is the garrison occupancy branch). (2) **Score uses FIVE coefficients and TWO effectiveness terms, not four/one** — the decompiler's `* 0.0` was an artifact; the real coeff is the 2nd double; base const `DAT_007F4E90 = 100000.0`. (3) **Scan-order tie source pinned** — concentric square-ring perimeter walk × intra-cell `+0xE8→+0x30` linked-list (tail-most passing occupant) × array-index order for the flat path; never stable-id. (4) **ftol order** — score is truncated to int FIRST, then all post-score modifiers (PreferWounded, force-to-1, building bonuses) are integer; ThreatAvoidance multiplies and re-truncates. NEW consumer `ShouldRetaliate @ 0x007087C0` calls Calculate_Threat_Score directly. See §2b, §2f, §2g, §5, and **§10 (Pass 2 — Expansion)**.

---

## Executive Summary

**Verdict: the current Rust per-unit target picker (`acquire_best_target` in `src/sim/combat/combat_targeting.rs`) ranks by `(dist_sq, threat_class, stable_id)` nearest-first, while gamemd ranks every candidate by an *integer threat score* and keeps the best only on STRICTLY GREATER, leaving scan order as the tie-break — so the two engines pick visibly different targets the moment two in-range enemies differ in threat or sit at the same distance.** This is the single biggest player-visible parity gap in the family: a Grizzly driving past a wounded high-value enemy and a healthy weak one will pick differently in VERA than in gamemd. Compounding gaps: (a) the score itself is never computed (no `SpecialThreatValue`, no strength/distance/effectiveness coefficients, no `EnemyHouseThreatBonus`, no `ThreatAvoidance` modifier parsed or applied); (b) the Rust scanner is a flat O(N) store iteration with no ring-expansion and no quarter/half-radius early return, so even with correct scoring it would consider a different candidate *set* and thus a different "first equal-score" winner; (c) the cloak-needs-sensor gate, the bridge-layer (`OnBridge`) gate, the limbo gate, and the local-human object-discovery gate are missing or approximated by TS-style fog; (d) acquisition runs only for entities with an `OrderIntent` (AttackMove/Guard) — ordinary moving `OpportunityFire` units never passively acquire; (e) Rust scans every tick over sorted entity ids, while gamemd gates passive acquisition behind a per-unit cadence timer and a one-shot ConvoyDisbanded scan. The proposed replacement is an additive, shadow-first **target-scoring service** (`ThreatScore` fixed-point value-type + a pure `ScoreTerms` evaluator + a `CandidateFilter` gate set + a `GreatestThreat` ring-scanner that owns the strictly-greater selection and early-return), slotting under master-TODO item #6 (target-acquisition cadence). Rollout mirrors the proven Mission/Radio rhythm: shadow → invert hash-invariant → drop shadow asserts → authoritative → bump `SNAPSHOT_VERSION` → parity harness, gated behind a P0 research checkpoint that must pin the float→int score conversion order and the +0x400 slot identity before scoring becomes authoritative.

---

## Table of Contents

- §1. Verified active-YR responsibilities of the target-scoring family
- §2. Full inventory (functions, fields, globals, tables, vtable slots, legacy/dormant)
- §3. Active vs inactive/legacy/deferred split
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements C1–C24)
- §6. Rust-native replacement boundary
- §7. Old ad hoc Rust logic to retire/fold
- §8. Migration slices + acceptance tests (P0–P7)
- §9. Sources & verification ledger
- §10. Pass 2 — Expansion (gate closures + completeness sweep)

---

## 1. Verified active-YR responsibilities of the target-scoring family

This is the player-observable contract the family owns. "Target scoring" = the three-stage service **Calculate_Threat_Score (pure terms) → Evaluate_Candidate (per-candidate gate + score + post-modifiers) → Greatest_Threat (scanner + strictly-greater selection)**, plus the FootClass wrapper that adjusts flags. Each line is the *behavior*, not the C++ structure.

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| T1 | **The per-unit "what do I shoot at" decision** for idle/guard/area-guard/attack-move/opportunity-fire units and garrisoned buildings: scan a region, gate candidates, score them, return the single best target pointer (or null). | VERIFIED | `TechnoClass__Greatest_Threat @ 0x006F8DF0` returns `local_4c` (best ptr) (LIVE-VERIFIED). |
| T2 | **Integer-score ranking with STRICTLY-GREATER selection**: best score initialized to −1; a candidate replaces the current best only when `new_score > best_score`; equal scores keep the earlier scan-order candidate. | VERIFIED | `local_50 = 0xffffffff`; every update guarded by `if ((int)local_50 < (int)param_3)` in all 5 candidate loops (LIVE-VERIFIED). |
| T3 | **Two scan topologies**: (a) flat array scan over `g_AircraftClass_Array` then `g_TechnoClass_Array` when `(flags & 3)==0`; (b) expanding concentric-square ring scan calling `Scan_Cell_For_Target` per cell when `(flags & 1\|2)` set, with **early return at quarter and half scan radius** once any candidate exists. | VERIFIED | flat branch + ring branch + `iStack_48 == (radius)>>2 \|\| iStack_48 == radius/2` early return (LIVE-VERIFIED). |
| T4 | **Scan-radius derivation**: from selected weapon range (`>>8`+1) plus `GuardRange` bonus (`type+0x68C >>8`); degenerate `range<0 && mission==GUARD(5)` → `0x200` (2 cells); no-weapon fallback to sight; **garrison-occupancy override** (`vtable+0x400` IsOccupied true, buildings only) = `GetHalfFoundationSize()+1+Rules->OccupyWeaponRange (+0xF48)` — REPLACES the normal radius. There is **no sensor branch**. | VERIFIED (corrected Pass 2) | `iStack_34` derivation; `cVar2 = vtable+0x400(); if(cVar2){ iStack_34 = vtable+0x404()+1+Rules+0xf48 }` (decompile_function 0x006F8DF0); +0x400 = `IsOccupied` (read_memory 0x007F4D60 → 0x0041BFB0 stub for Techno, read_memory 0x007E42BC → 0x00458DD0 for Building; decompile_function 0x00458DD0); +0x404 = `GetHalfFoundationSize` (decompile_function 0x00458E00); Rules+0xF48 = `OccupyWeaponRange` (decompile_function 0x0066BBB0). |
| T5 | **Candidate legality gating** (Evaluate_Candidate): fire-error-5 reject, InLimbo (`+0x81`), zero-health, fully-cloaked-without-sensor, local-human object discovery (`+0x41A/+0x41B`), warhead-Verses-vs-armor ≤ 0, submarine-submerged, weapon range, Can_Fire_At, LegalTarget/Insignificant, bridge-`OnBridge` layer match, zone compatibility. | VERIFIED | Evaluate_Candidate filter pipeline (DOC-SOURCED, multiple reports with assembly ranges). |
| T6 | **Score math** (Calculate_Threat_Score): base constant `100000.0` + **attacker-weapon-effectiveness** (A·Verses[candArmor]) + **candidate-weapon-threat-to-me** (±B·candVerses[myArmor], `*0.0`-artifact corrected) + `C·SpecialThreatValue` + `EnemyHouseThreatBonus` (if candidate is owner's designated enemy) + `D·health-ratio` + `max(0, dist − range)·E` (distance term), with a per-owner coefficient-set switch (Rules "Dumb" defaults vs per-type). **FIVE coefficient doubles per branch (A/B/C/D/E), not four.** | VERIFIED (corrected Pass 2) | `disassemble_function 0x0070CD10` (FMULs at 0xceb2/0xcec7/0xcee0/0xcf49/0xcf5f); base const `read_memory 0x007F4E90` = 100000.0. See §2b. |
| T7 | **Post-score modifiers** (in Evaluate_Candidate after score write): float→int conversion; PreferWounded health doubling/halving; enemy-house force-to-1; building/special bonuses & defenseless-zero under special flags; `ThreatAvoidance_Modifier` multiply; final clamp (accept ≥ 1, reject 0). | VERIFIED | Evaluate_Candidate post-score block `0x006F8721..0x006F8948` (DOC-SOURCED). |
| T8 | **Owner/alliance/enemy-house filtering**: `Is_Ally_ByObject` prefilter; reject ordinary allies for weapon-bearing units; AttackFriendlies / Berserk / mind-control-tethered overrides; `enemy_only` restricts to `Owner+0x5600` (designated enemy house index). | VERIFIED | Greatest_Threat ally/`param_4`/`Owner+0x5600` checks (LIVE-VERIFIED); special override flags (DOC-SOURCED). |
| T9 | **Per-weapon target-mask flag folding**: the UnitClass scanner wrapper `0x00743190` ORs the selected weapons' target-class bits into the threat flags before scanning; the FootClass wrapper `FUN_004D9920` forces weapon-range mode (bit 0 set, bit 1 clear) while ConvoyDisbanded (`+0x688`) is set, then clears that flag on no-target. | VERIFIED | `FUN_004d9920` body (LIVE-VERIFIED); `0x00743190` mask-OR (DOC-SOURCED). |
| T10 | **Gattling dual-weapon best-of**: when `IsGattling` (`type+0x6B0`), the scanner maintains separate primary/secondary target+score arrays (fields `+0x440..+0x46C`), each tracking its own best. | VERIFIED | `field_0x440`/`field_0x460` array updates in both ring sub-loops (LIVE-VERIFIED). |
| T11 | **Garrison building scan mode**: a garrisoned building uses cell-based scanning with scan radius `GetHalfFoundationSize()+1+OccupyWeaponRange` (replaces, not adds). | VERIFIED (formula) / UNCHECKED (slot) | Formula DOC-SOURCED (GARRISON §15a/§15b); the exact vtable slot that *selects* this branch is flagged UNCHECKED — see §2g note. |

---

## 2. Full inventory

### 2a. Functions in the family

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| TechnoClass::Greatest_Threat | 0x006F8DF0 — **vtable+0x3C4** | Main scanner: derive radius, dual topology, gate (delegating to Evaluate_Candidate), strictly-greater select, quarter/half early-return, cell-threat fallback. Body 0x006F8DF0–0x006F9DAE | YES | LIVE-VERIFIED (`decompile_function 0x006F8DF0`; `get_function_by_address 0x006F8DF0`) |
| TechnoClass::Evaluate_Candidate | 0x006F7CA0 | Per-candidate legality filter pipeline + calls Calculate_Threat_Score + post-score modifiers + final accept clamp. Body 0x006F7CA0–0x006F895A. Returns 1 (valid) / 0 (rejected), writes score out-param | YES | LIVE-VERIFIED (`get_function_by_address 0x006F7CA0`); body DOC-SOURCED |
| TechnoClass::Calculate_Threat_Score | 0x0070CD10 | Pure score terms. Body 0x0070CD10–0x0070D0CF. Returns float10 | YES | LIVE-VERIFIED (`decompile_function 0x0070CD10`) |
| TechnoClass::Scan_Cell_For_Target | 0x006F8960 | Walk one cell's occupant linked list, gate, keep best-of-cell via Evaluate_Candidate, zone filter. Body 0x006F8960–0x006F8C0F | YES | LIVE-VERIFIED (`get_function_by_address 0x006F8960`); body DOC-SOURCED |
| TechnoClass::ThreatAvoidance_Modifier | 0x006F79A0 | Scan ring (radius `Rules+0x1430 ThreatAvoidanceRadius >>8`) around target cell for allied buildings; multiply score by `DAT_007E1738` = **0.5** per allied building found → discourages attacks near our base. Gated by attacker's own building having `+0x146` set; returns 1.0 otherwise. | YES | LIVE-VERIFIED (`decompile_function 0x006F79A0`; const `read_memory 0x007E1738` = 0.5); called ONLY from Evaluate_Candidate (get_function_callers 0x006F79A0) |
| **TechnoClass::ShouldRetaliate** (NEW consumer) | 0x007087C0 | **Retaliation target-switch gate**: when a unit is hit, calls `Calculate_Threat_Score(current_target)` vs `Calculate_Threat_Score(attacker)` (both with `&g_NullCoord` → cells); if `score(current) > score(attacker)` returns 0 (keep current target, don't retaliate). Also a final Verses[armor] ≤ 0.01 reject. **Second direct consumer of the score outside the scanner.** | YES | LIVE-VERIFIED (`decompile_function 0x007087C0`; get_function_callers 0x0070CD10 → Evaluate_Candidate + ShouldRetaliate). Pass-2 find: was absent from inventory. |
| TechnoClass::Cell_Threat_Fallback | 0x006F8C10 | Cell-level threat used only when no object candidate found yet (returns a cell to steer toward); ranked by `piStack_3c` strictly-greater | YES | LIVE-VERIFIED (called as `TechnoClass__Cell_Threat_Fallback` in both ring sub-loops of 0x006F8DF0) |
| FootClass::Greatest_Threat (wrapper) | 0x004D9920 — **vtable+0x3C4 override** | Force weapon-range scan (bit0 set / bit1 clear) while `+0x688` ConvoyDisbanded set; delegate to TechnoClass::Greatest_Threat; clear `+0x688` on no-target | YES | LIVE-VERIFIED (`decompile_function 0x004D9920`) |
| UnitClass scanner wrapper | 0x00743190 — **UnitClass vtable+0x3C4** | OR selected-weapon target-mask bits, then call FUN_004D9920 | YES | DOC-SOURCED (GRIZZLY_PASSIVE §2; vtable read 0x007F6034→0x00743190) |
| Second mask-OR wrapper (direct Greatest_Threat caller) | 0x00445F00 | OR both weapons' target-mask bits (via FUN_00772A90) into flags, force bit0 (weapon-range mode), then call TechnoClass::Greatest_Threat directly (NOT via FUN_004D9920) — a sibling mask-OR entry point on a different class path | YES | LIVE-VERIFIED (`get_function_callers 0x006F8DF0` lists FUN_00445F00; `decompile_function 0x00445F00` confirms `vtable+0x3F8` weapon reads → `FUN_00772A90` mask OR → `Greatest_Threat(uVar2 \| 1, ...)`). Reviewer-added; was a direct caller missing from the inventory. |
| UnitClass passive driver (Retaliate_And_Scan) | 0x00709820 — **UnitClass vtable+0x39C** | Schedule next passive-scan timer (`+0x4FC`/`+0x50C`), clear stale invalid TarCom, if no TarCom call +0x3C4, set TarCom via +0x3C8 | YES | DOC-SOURCED (GRIZZLY_PASSIVE §1/§2; vtable read 0x007F600C→0x00709820) |
| TechnoClass::AI_Update / Passive_Target_Acquire (caller) | 0x006FA6B7.. / 0x00709492.. | Writes `g_CurrentFrameCounter` to `+0x4FC`, gets coords via vtable+0x48, calls vtable+0x39C | YES | DOC-SOURCED (GRIZZLY_PASSIVE §2) |
| FootClass::Greatest_Threat_Scan (the MISNOMER) | 0x004D5690 — **vtable+0x53C** | **NOT a scanner.** Approach driver / firing-position search for a unit that ALREADY has a TarCom (angular fan, subcell snap, pathfind gate). Out of THIS family's scope — belongs to the approach/movement substrate | YES | DOC-SOURCED (GREATEST_THREAT_SCAN §1 misnomer) |
| InfantryClass +0x53C override | 0x00522340 | A-move/spyplane mission preprocessor before delegating to 0x004D5690 — approach family, not scoring | YES | DOC-SOURCED (GREATEST_THREAT_SCAN §8) |
| UnitClass +0x53C override | 0x007414E0 | Crush-ram preprocessor before delegating — approach family, not scoring | YES | DOC-SOURCED (GREATEST_THREAT_SCAN §8) |

### 2b. Score formula inputs (Calculate_Threat_Score, LIVE-VERIFIED algebra — CORRECTED Pass 2)

The live **disassembly** (`disassemble_function 0x0070CD10`) resolves the exact term-by-term arithmetic and corrects two Pass-1 errors: (a) **FIVE** coefficient doubles per branch, not four; (b) the "effectiveness * 0.0" was a decompiler artifact — the real multiplier is the 2nd coefficient (B), so the candidate-weapon term is a **real** term.

```c
// attacker(ECX); candidate(ESI); ref coords(param_3). attackerType = vtable+0x84.
// candidate->vtable+0x88 == 0  → return 0.0  (const @ 0x007E2800)
if (*(char *)(attacker.Owner + 0x1fb) == 0) {   // Owner=attacker+0x21C; NOT human → "Dumb" Rules
    A=Rules+0x1068; B=Rules+0x1070; C=Rules+0x1078; D=Rules+0x1080; E=Rules+0x1088;
} else {                                         // human / per-type set
    A=Type+0x2C8;   B=Type+0x2D0;   C=Type+0x2D8;  D=Type+0x2E0;   E=Type+0x2E8;
}
attackerWeapon = attacker.GetWeapon(SelectWeaponAgainst(candidate));   // vtable+0x2E4 → +0x3F8
acc = 0;
// term B: candidate's weapon effectiveness against ME (only RTTI 6/0xF/1/2 candidates):
if (candidate is bldg/inf/unit/air) {
    cw = candidate.GetWeapon(SelectWeaponAgainst(attacker));
    if (cw && cw.Warhead(+0xAC)) {
        v = candWarhead.Verses[attackerType.Armor(+0x9C)];      // Warhead+0xA0 + armor*8
        acc = (candidate.Target(+0x2B4)==attacker) ? -(B*v) : (B*v);   // FCHS if cand targets me
    }
    acc += C * candidateType.SpecialThreatValue(+0x2C0);        // term C
    if (attacker.Owner+0x5600 != -1 && == candidate.Owner+0x30) // EnemyHouseThreatBonus
        acc += Rules+0x1090;
}
if (attackerWeapon && attackerWeapon.Warhead(+0xAC))            // term A: MY weapon vs candidate
    acc += A * attackerWarhead.Verses[candidateType.Armor(+0x9C)];
acc += D * candidate.GetHealthRatio();                          // term D (0x005F5C60, double)
R = attackerWeapon ? attackerWeapon.Range(+0xB4) : attackerType.Sight(+0x5B8);
R = (R + (R>>31 & 0xff)) >> 8;                                  // → cells
if (ref == g_NullCoord) { d = ftol(Sqrt(Δ²)); d = (d+(d>>31&0xff))>>8; }  // distance in CELLS
else                    { d = ftol(Sqrt(Δ²)); }                            // distance in LEPTONS
dist_term = (d - R) < 0 ? 0 : (d - R);                          // SETS/DEC/AND clamp-neg-to-zero
return (float10)dist_term * E  +  acc  +  100000.0;             // base const _DAT_007F4E90
```

**Confirmed live (this run):** coeff switch on `Owner+0x1FB`; **Dumb** doubles `Rules+0x1068/+0x1070/+0x1078/+0x1080/+0x1088`; **per-type** `Type+0x2C8/+0x2D0/+0x2D8/+0x2E0/+0x2E8` — **FIVE each** (verified at `disassemble_function 0x0070CD10`, 0x0070CD58–0x0070CE23); SpecialThreatValue `type+0x2C0`; EnemyHouseThreatBonus `Rules+0x1090` gated by `Owner+0x5600` matching candidate house index; distance clamp `(d<0)-1 & d`; base const `read_memory 0x007F4E90` = **100000.0** (0x40F86A0000000000); early-return const `read_memory 0x007E2800` = 0.0; `Sqrt_Approx @ 0x004CAC40`, `Math__ftol @ 0x007C5F00` (matches the cross-family ftol truncate-toward-zero finding). **CORRECTIONS to Pass 1: (1) FIVE coeffs not four; (2) term B is REAL (`* 0.0` was a Ghidra artifact for an unresolved stack double `[ESP+0x18]` = coeff B); (3) base const = 100000.0, not "likely 0.5".** `ThreatPosed=` (`type+0x670`) still NOT a direct term.

**DRIFT NOTE (NEW):** the my-coord (NullCoord) path returns distance in **CELLS**, the explicit-ref-coord path in **LEPTONS** (no `>>8`). The scanner (Greatest_Threat / Scan_Cell_For_Target / ShouldRetaliate) always passes `&g_NullCoord` → cells. A Rust port must NOT unconditionally divide distance by 256; it must branch on whether the reference is the "use-my-coord" sentinel.

**BASE-CONST observability:** with base = 100000 and stock E=−10, D=−200, the `100000` dominates; most legal candidates land near 100000 and ranking is decided by the small ± deltas (effectiveness, special-threat, enemy bonus, health, distance). This makes the **integer truncation order and the exact coefficient set load-bearing for ranking ties** — a Rust port that accumulates in a narrower type or truncates at a different boundary can flip near-equal candidates.

**Confirmed live:** coefficient-source switch on `Owner+0x1FB`; Rules offsets `+0x1068/+0x1070/+0x1080/+0x1088`; Type offsets `+0x2C8/+0x2D0/+0x2E0/+0x2E8`; `SpecialThreatValue` at candidate `type+0x2C0`; `EnemyHouseThreatBonus` at `Rules+0x1090` gated by `Owner+0x5600` (designated enemy index) matching candidate house index; distance clamp-negative-to-zero `(d<0)-1 & d`; base const `_DAT_007F4E90`. **Only 4 coefficient doubles are read per branch** (not 5). `ThreatPosed=` (`type+0x670`) is **NOT** a direct term here. (LIVE-VERIFIED `decompile_function 0x0070CD10`.)

### 2c. INI keys (rulesmd.ini / rules.ini) feeding the score & gates

| INI key | Field | Stock default | Role | Evidence |
|---|---|---|---|---|
| `SpecialThreatValue=` | TechnoType+0x2C0 (double) | 0 | multiplicand of **C** coeff (special-threat term) | DOC-SOURCED (GRIZZLY_MINIMAL §4; ReadINI 0x00715726) |
| `TargetEffectivenessCoefficientDefault=` (**A**) | Rules+0x1068 / Type+0x2C8 | −200 | MY-weapon-effectiveness (Verses[candArmor]) coeff | LIVE-VERIFIED offset (disassemble_function 0x0070CD10) |
| (**B** coeff) | Rules+0x1070 / Type+0x2D0 | — | candidate-weapon-threat-to-me (Verses[myArmor]) coeff; the `* 0.0` artifact was actually this | LIVE-VERIFIED offset |
| `TargetSpecialThreatCoefficientDefault=` (**C**) | Rules+0x1078 / Type+0x2D8 | 200 | SpecialThreatValue multiplicand | LIVE-VERIFIED offset (CORRECTED: was 0x1080/0x2E0 in Pass 1) |
| `TargetStrengthCoefficientDefault=` (**D**) | Rules+0x1080 / Type+0x2E0 | −200 | strength (health-ratio) coeff | LIVE-VERIFIED offset |
| `TargetDistanceCoefficientDefault=` (**E**) | Rules+0x1088 / Type+0x2E8 | −10 | distance-beyond-range penalty coeff | LIVE-VERIFIED offset |
| `EnemyHouseThreatBonus=` | Rules+0x1090 (double) | 400 | added when candidate is owner's designated enemy (`Owner+0x5600`) | LIVE-VERIFIED offset |
| (score base const) | DAT_007F4E90 (double) | **100000.0** | constant added to every legal score | LIVE-VERIFIED (read_memory 0x007F4E90) |
| `GuardRange=` | TechnoType+0x68C (leptons) | 0 | scan-radius bonus | DOC-SOURCED |
| `OccupyWeaponRange=` | Rules+0x0F48 (`[CombatDamage]`, int) | — | **garrison** scan-radius term (replaces normal radius for occupied buildings). **NOT GuardAreaTargetingDelay** (Pass-1 error). | LIVE-VERIFIED (decompile_function 0x0066BBB0 writes `OccupyWeaponRange` → +0xF48; read in 0x006F8DF0 garrison branch) |
| `ConditionYellow=` | Rules+0x16F8 | — | repair-target health threshold (heal weapons / allies) | DOC-SOURCED |
| `ConditionRed=` | Rules+0x1708 | — | friendly-building-defense switch threshold | DOC-SOURCED |
| `ThreatAvoidanceRadius` | Rules+0x1430 | — | avoidance-modifier scan radius | DOC-SOURCED |
| `OpportunityFire=` | TechnoType+0x6AF (bool) | — | gates ordinary-move passive acquisition (opens the passive driver) | DOC-SOURCED (OPPORTUNITY_FIRE) |
| `OccupyWeaponRange=` | Rules `[CombatDamage]`+0x0F48-adjacent (cells) | — | garrison scan-radius term | DOC-SOURCED (GARRISON §15a) |
| `PreferWounded` | TechnoType+0x394 | — | score doubling/halving by target health | DOC-SOURCED |
| `DontScore=` | TechnoType+0xD20 (bool) | — | gates the player-controlled early-return (don't auto-acquire) | LIVE-VERIFIED read at 0x006F8DF0 entry |

### 2d. Singleton / global state

| Name | Address | Role | Active-in-YR | Hash-relevant? | Evidence |
|---|---|---|---|---|---|
| g_TechnoClass_Array / _Count | (global) | flat scan source (all technos) | YES | n/a (read-only) | LIVE-VERIFIED (read in 0x006F8DF0) |
| g_AircraftClass_Array / _Count | (global) | flat scan source (aircraft, flag 0x4 path) | YES | n/a | LIVE-VERIFIED |
| g_RulesClass_Instance | 0x008871E0 (ptr) | coefficient + bonus + threshold source | YES | static | LIVE-VERIFIED (read at Rules+0x1068.. in 0x0070CD10) |
| DAT_00A8EC34 | 0x00A8EC34 | **Greatest_Threat call counter** — incremented at entry, read NOWHERE else | YES | **NO** — pure profiling artifact, not gameplay/hash | LIVE-VERIFIED: `get_xrefs_to 0x00A8EC34` returns ONLY the self read+write inside 0x006F8DF0 |
| g_NullCoord_Chrono_X/Y/Z | (global) | "use my-coord" sentinel for reference coords | YES | static | LIVE-VERIFIED (passed as `&g_NullCoord_Chrono_X`) |
| DAT_007F4E90 | 0x007F4E90 | score base constant = **100000.0** (0x40F86A0000000000) | YES | static | LIVE-VERIFIED (read_memory 0x007F4E90; returned in 0x0070CD10) |
| DAT_007E1738 | 0x007E1738 | ThreatAvoidance per-building multiplier = **0.5** (0x3FE0000000000000) | YES | static | LIVE-VERIFIED (read_memory 0x007E1738; multiplied per allied building in 0x006F79A0) |
| DAT_007E2800 | 0x007E2800 | early-return const = 0.0 (candidate with no weapon-presence) | YES | static | LIVE-VERIFIED (read_memory 0x007E2800) |
| `Owner+0x1580` | HouseClass | **force-to-1 enemy-house index** (Evaluate_Candidate post-score), gated by `Owner+0x249` — DISTINCT from `Owner+0x5600` | YES | YES | LIVE-VERIFIED (decompile_function 0x006F7CA0 LAB_006f875f) |
| `+0x4FC` (per-unit) | TechnoClass instance | last passive-scan frame stamp (cadence) | YES | YES (sim state) | DOC-SOURCED (GRIZZLY_PASSIVE §2) |
| `+0x50C` (per-unit) | TechnoClass instance | TarCom-changed flag for stale clearing | YES | YES | DOC-SOURCED |
| `+0x688` (per-unit) | FootClass instance | ConvoyDisbanded one-shot scan flag | YES | YES | LIVE-VERIFIED (read/write in 0x004D9920) |
| `Owner+0x5600` | HouseClass | designated enemy house index (enemy_only + bonus) | YES | YES | LIVE-VERIFIED (read in 0x006F8DF0 + 0x0070CD10) |

### 2e. Static tables / data

| Table | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| Coefficient block (Rules "Dumb" set) | Rules+0x1068..+0x108C | **FIVE** doubles A(+0x1068)/B(+0x1070)/C(+0x1078)/D(+0x1080)/E(+0x1088) for non-human owners | YES | LIVE-VERIFIED (disassemble_function 0x0070CD10) |
| Per-type coefficient block | TechnoType+0x2C8..+0x2EC | **FIVE** doubles A(+0x2C8)/B(+0x2D0)/C(+0x2D8)/D(+0x2E0)/E(+0x2E8) per-type for human owners | YES | LIVE-VERIFIED (disassemble_function 0x0070CD10) |
| Warhead Verses table | Warhead+0xA0 (8 bytes/armor) | effectiveness lookup `Verses[armor]` | YES | DOC-SOURCED |
| Threat-flag bit semantics | (in-code) | bit0 weapon-range, bit1 guard-range, bit2 neutral, bit3 air-priority, bit4 allies, bit8 house-only, bit14 enemy-house-only | YES | LIVE-VERIFIED (flag-folding into uStack_58 in 0x006F8DF0) |

### 2f. Vtable / COM slots used

| Slot | Class | Target | Role | Evidence |
|---|---|---|---|---|
| +0x3C4 | TechnoClass | Greatest_Threat 0x006F8DF0 | the scanner | LIVE-VERIFIED (called as base) |
| +0x3C4 | FootClass | FUN_004D9920 | wrapper override | LIVE-VERIFIED |
| +0x3C4 | UnitClass | 0x00743190 | mask-OR wrapper | DOC-SOURCED (vtable read) |
| +0x39C | UnitClass | 0x00709820 | passive driver (sets TarCom) | DOC-SOURCED (vtable read) |
| +0x3C8 | TechnoClass | Set_ArchiveTarget / Set TarCom | commit acquired target | DOC-SOURCED |
| +0x3BC | TechnoClass | GetFireError | fire-error-5 reject gate | DOC-SOURCED |
| +0x3A8 | TechnoClass | Can_Fire_At | final fire validation | DOC-SOURCED |
| +0x168 | TechnoClass | Effective_Weapon_Range | scan-radius weapon range | LIVE-VERIFIED (called in 0x006F8DF0) |
| +0x31C | UnitClass | 0x00707E60 | scan-range override query | DOC-SOURCED |
| +0x400 | TechnoClass (stub) / BuildingClass (override) | `0x0041BFB0` (return 0) / `BuildingClass__IsOccupied 0x00458DD0` | **IsOccupied** — garrison scan-radius branch selector. RESOLVED: NOT Is_Sensor. Base techno/unit/inf return 0 (branch never taken); only occupied buildings return 1. | LIVE-VERIFIED (read_memory 0x007F4D60 → 0x0041BFB0; read_memory 0x007E42BC → 0x00458DD0; decompile both) |
| +0x404 | TechnoClass (stub) / BuildingClass (override) | `0x0041BFC0` (return 0) / `BuildingClass__GetHalfFoundationSize 0x00458E00` | **GetHalfFoundationSize** = `min(width,height)/2`. NOT Get_Sensor_Range. | LIVE-VERIFIED (read_memory 0x007F4D64 → 0x0041BFC0; read_memory 0x007E42C0 → 0x00458E00; decompile 0x00458E00) |
| +0x408 | TechnoClass | (occupant count) | read by `IsOccupied` (`vtable+0x408 > 0`) to confirm a garrisoned building | LIVE-VERIFIED (called in 0x00458DD0) |
| +0x84 | TechnoClass | Class_Of (TypeClass) | type-field reads | LIVE-VERIFIED |
| +0x2C | TechnoClass | Get_RTTI_ID | type discriminator (6=bldg, 0xF=inf, 1=unit, 2=air) | LIVE-VERIFIED |
| +0x88 | ObjectClass | (candidate weapon presence) | gate in Calculate_Threat_Score | LIVE-VERIFIED |
| +0x3F8 | TechnoClass | Get_Weapon | weapon for effectiveness/range | LIVE-VERIFIED |
| +0x2E4 | TechnoClass | SelectWeaponAgainst / Likely_Coord | weapon select for scoring | LIVE-VERIFIED |
| +0x48 | TechnoClass | Get_Coord | distance reference | LIVE-VERIFIED |

### 2g. RESOLVED (Pass 2): the +0x400 slot identity — IsOccupied, NOT Is_Sensor

**VERIFIED — there is no sensor branch.** The Pass-1 "Is_Sensor_Type / Get_Sensor_Range" reading of `vtable+0x400`/`+0x404` was a label/inference error. Live this run:
- TechnoClass vtable base = `0x007F4960` (Greatest_Threat at +0x3C4 = 0x007F4D24, get_xrefs_to 0x006F8DF0 → DATA 0x007F4D24). +0x400 slot (0x007F4D60) = `0x0041BFB0` = `return 0;` stub; +0x404 (0x007F4D64) = `0x0041BFC0` = `return 0;` stub. So **base technos/units/infantry never take the branch** (read_memory 0x007F4D60).
- BuildingClass vtable +0x400 (0x007E42BC) = `BuildingClass__IsOccupied 0x00458DD0` (CanBeOccupied `+0x157B` && CanOccupyFire `+0x157C` && occupant-count `vtable+0x408 > 0`); +0x404 (0x007E42C0) = `BuildingClass__GetHalfFoundationSize 0x00458E00` (`min(width,height)/2`). (read_memory 0x007E42BC/0x007E42C0; decompile_function 0x00458DD0 / 0x00458E00.)
- The garrison scan radius therefore = `GetHalfFoundationSize() + 1 + Rules.OccupyWeaponRange` and **REPLACES** the normal radius (matches GARRISON_SYSTEM §15a). `Rules+0xF48 = OccupyWeaponRange` from `[CombatDamage]` (decompile_function 0x0066BBB0). UnitClass vtable +0x400 (0x007F6070) also binds the stub (read_memory) — confirming ordinary units never enter the branch. **P0.2 CLOSED.**
- Corroborating docs (same identity): GARRISON_SYSTEM_GHIDRA_REPORT §15a, BUILDINGCLASS_MISSION_ATTACK §IsOccupied, TECHNOCLASS_INRANGE §5, BUNKER_SYSTEM §5.

### 2h. Legacy / dormant TS paths in this surface

| Item | Status | Evidence |
|---|---|---|
| `DAT_00A8EC34` Greatest_Threat call counter | **DORMANT / profiling-only** — incremented but never read elsewhere; not gameplay/hash. Do NOT model. | LIVE-VERIFIED (`get_xrefs_to 0x00A8EC34` = self read+write only) |
| Fog-of-war "previously-seen darkening" coupling | TS-legacy; OFF by stock-YR default. The local-human gate uses object bytes `+0x41A/+0x41B`, NOT TS cell-fog. Do NOT gate acquisition on TS fog. | DOC-SOURCED (GRIZZLY_CLOAK_BRIDGE §4; AGENTS) |
| Underground/subterranean detection (`+0x3D5` probabilistic) | TS-ghost; the `+0x3D5` byte IS read as a reject gate but its "underground random detection" meaning is **unproven** — sibling docs disagree (HasSight vs in-playfield). Treat as a generic map-state gate; do NOT implement as TS underground RNG. | DOC-SOURCED (GRIZZLY_CLOAK_BRIDGE §2 + Remaining Uncertainty) |
| Tunnel/subterranean | Not present in this family. | per project rule (no tunnel) |
| `param_2 & 0x100` house-only quick scan (bit 8) | Active in YR but AI-planning use (returns a house, not a unit). DEFERRED-AI. | LIVE-VERIFIED (flag fold `uStack_58 = 0x8042`) |
| dry-run path of the approach driver (not this family) | dead in shipping binary | DOC-SOURCED (GREATEST_THREAT_SCAN §8) |

---

## 3. Active vs inactive/legacy/deferred split

### ACTIVE-YR — must be reproduced (player-observable contract)

| Item | One-line rationale |
|---|---|
| Integer-score ranking + STRICTLY-GREATER selection + scan-order tie-break (T2) | Determines which of several in-range enemies a unit shoots — visible every engagement with ≥2 candidates. |
| Score terms: effectiveness, SpecialThreatValue, strength(health), range, distance-penalty, EnemyHouseThreatBonus (T6) | The numbers that make a wounded/high-value/closer enemy win — wrong terms = wrong target. |
| Per-owner coefficient switch (Rules "Dumb" set vs per-type) keyed on `Owner+0x1FB` (T6) | Human-owned and AI/neutral-owned units score differently; both occur in skirmish. |
| Post-score modifiers: PreferWounded, ThreatAvoidance, enemy-house force-to-1, final clamp ≥1/reject-0 (T7) | Shapes the score after the base math; PreferWounded and avoidance are visible target-preference behaviors. |
| Candidate gates: cloak-needs-sensor, InLimbo, zero-health, Verses≤0, bridge-OnBridge-match, local-human discovery, range, Can_Fire_At (T5) | What is even eligible — over/under-acquisition is immediately visible (Grizzly shooting a cloaked unit, or ignoring a valid one). |
| Ring-expanding scan with quarter/half-radius early return (T3) | Changes the candidate *set* considered and thus the first equal-score winner; visible in crowded fights. |
| Scan-radius derivation incl. GuardRange bonus, GUARD degenerate 0x200, sensor bonus (T4) | How far a unit looks; wrong radius = acquires too early/late. |
| Alliance/AttackFriendlies/Berserk/mind-control/enemy_only filtering (T8) | Who counts as an enemy — mind-controlled and berserk units visibly attack differently. |
| Per-weapon target-mask folding + FootClass ConvoyDisbanded one-shot + UnitClass mask-OR (T9) | Anti-air/anti-ground weapon target eligibility; convoy-escort reacquire behavior. |
| Gattling dual-weapon best-of (T10) | Gattling units track two targets; visible for Gattling Tank / flak. |
| Passive-acquire cadence: per-unit timer (`+0x4FC`), stale clear (`+0x50C`), OpportunityFire gate | gamemd does NOT scan every tick; cadence affects when a moving unit picks up a passerby. |

### DEFERRED-AI — active in YR but out of scope now (leave a clean seam)

| Item | One-line rationale |
|---|---|
| House-only quick scan (flag bit 8 / `param_2 & 0x100`, returns a house) | AI base-target planning; human play never reaches it. |
| AI-house "Dumb" coefficient path as an AI *planning* input | The coefficient switch itself is in scope (it fires for neutral/AI-owned candidates a human shoots); AI *consumption* of house-level threat is deferred. |
| `ThreatPosed=` consumers in other AI threat systems | Not the direct Calculate_Threat_Score term; AI-only elsewhere. |
| Cell_Threat_Fallback as an AI steering output | The fallback CELL (vs object) result is consumed by AI/guard steering; the object-best path is what human combat needs first. |

### TS-LEGACY / DORMANT — do NOT implement as default

| Item | One-line rationale |
|---|---|
| `DAT_00A8EC34` call counter | Profiling artifact; never read; not hash/gameplay. |
| TS fog "previously-seen" darkening as acquisition gate | Off by stock YR default; gate is object-discovery bytes, not TS cell fog. |
| `+0x3D5` "underground probabilistic detection" wording | Meaning unproven; implement as a generic map-state reject, not TS underground RNG. |
| Approach driver `0x004D5690` (the misnomer) | Not a scanner; belongs to the approach/movement substrate, not target scoring. |

---

## 4. Comparison against the current Rust architecture

The current per-unit target picker is a **single flat function** `acquire_best_target` in `src/sim/combat/combat_targeting.rs` (lines 167–286), driven by free functions, with a thin `acquire_best_target_for_entity` wrapper (lines 92–145). There is no score concept, no ring scan, no cadence timer, no candidate-filter module — gating and ranking are inlined into one loop. Acquisition is invoked from three places, all every-tick over sorted ids.

### 4.1 Structural map

| Concern | gamemd authority | Current Rust | Verdict |
|---|---|---|---|
| Selection key | integer threat score, strictly-greater, scan-order tie | `(dist_sq, threat_class, stable_id)` nearest-first, stable-id tie (`combat_targeting.rs:278`) | **DRIFT (the headline gap)** |
| Score computation | `Calculate_Threat_Score` 6-term formula + modifiers | none — no score is computed at all | **MISSING** |
| Scan topology | ring-expanding squares + quarter/half early return (or flat array for AI) | flat `entities.values()` iteration, no early return (`combat_targeting.rs:179`) | **DRIFT** — different candidate set/order |
| Candidate gate | `Evaluate_Candidate` ~14-stage pipeline | inline: alive, not-inside-transport, hostile, fog-visible, weapon-compatible, Verses>1%, in-range (`combat_targeting.rs:183-275`) | **PARTIAL** — missing cloak-sensor, bridge-OnBridge, InLimbo-as-such, local-human discovery, PreferWounded |
| Cadence | per-unit `+0x4FC` timer, OpportunityFire gate | none — runs only for `OrderIntent` entities, every tick (`world_orders.rs:50-81`) | **DRIFT + MISSING** — no passive OpportunityFire path; no cadence |
| Coefficients / threat fields | parsed from rulesmd `[General]` + per-type | NOT parsed (no SpecialThreatValue/coefficients/EnemyHouseThreatBonus in `src/rules`) | **MISSING** (per GRIZZLY_MINIMAL Current Rust Delta) |
| Garrison scan range | `halfFoundation+1+OccupyWeaponRange` (cells) | `scan_range_override: Option<SimFixed>` threaded from `combat/mod.rs` (`combat_targeting.rs:174,232`) | **PARTIAL** — override plumbing exists; value derivation lives ad hoc in mod.rs |

### 4.2 Behavior table (default DRIFT)

| # | gamemd behavior (verified) | Current Rust | Verdict | Player-visible? | Trigger frequency |
|---|---|---|---|---|---|
| 1 | Rank by integer score, replace only on `new > best`, equal keeps scan order (T2) | `(dist_sq, threat_class, stable_id)` nearest-first; `rank >= current` keeps earlier (so it IS strictly-greater-on-the-tuple, but on the WRONG key) (`combat_targeting.rs:278-282`) | **DRIFT (HIGH)** | YES — picks a different target whenever score ordering ≠ distance ordering | every fight with ≥2 in-range enemies |
| 2 | Score = 100000 + A·myWarhead.Verses[candArmor] + ±B·candWarhead.Verses[myArmor] + C·SpecialThreatValue + [enemyBonus] + D·healthRatio + max(0,d−R)·E (T6/§2b, 5 coeffs, 2 effectiveness terms) | no score computed | **MISSING (HIGH)** | YES — wounded/high-value targets never preferred | every engagement |
| 3 | Distance is a *penalty term beyond weapon range*, not the primary key (T6) | distance (`dist_sq`) is the primary sort key | **DRIFT** | YES — nearest always wins regardless of value | every engagement |
| 4 | Cloaked target requires positive `SensorCountForHouse` unless same owner (T5) | no cloak/sensor gate; uses fog visibility only (`combat_targeting.rs:198`) | **DRIFT** | YES — Grizzly would shoot an unsensed cloaked enemy | every cloaked-unit encounter |
| 5 | Bridge reject only when BOTH cells are bridge cells (`+0x140 & 0x100`) and `OnBridge` differs (T5) | no candidate-layer bridge gate in this helper (`on_bridge` exists on entity but unused here) | **DRIFT** | YES — acquires across bridge layers | every bridge fight |
| 6 | Local-human object discovery via `+0x41A/+0x41B`, NOT TS fog (T5) | `FogState::is_cell_visible` (TS-ish cell gate) (`combat_targeting.rs:199`) | **DRIFT** | YES — wrong default-YR visibility semantics | every fog interaction |
| 7 | Ring-expanding scan, quarter/half early return (T3) | flat full-store scan, no early return | **DRIFT (determinism)** | YES (which equal-score target wins) | every crowded fight |
| 8 | Passive OpportunityFire acquisition for ordinary moving units, gated by cadence timer (T1/cadence) | only `OrderIntent` (AttackMove/Guard) entities acquire; no OpportunityFire path (`world_orders.rs:56`) | **MISSING (HIGH)** | YES — moving units never auto-pick up passersby | every move past an enemy |
| 9 | PreferWounded score doubling/halving (T7) | none | **DRIFT** | YES for units with the flag | per-type, fires every fight for those units |
| 10 | ThreatAvoidance multiply near friendly buildings (T7) | none | **DRIFT** | borderline — AI/guard feel; visible as "prefers isolated targets" | every fight near a base |
| 11 | EnemyHouseThreatBonus + enemy-house force-to-1 (T6/T7/T8) | none; no designated-enemy concept | **DRIFT** | YES in 1v1 with a declared enemy | most MP matches |
| 12 | Gattling dual-weapon best-of (T10) | single best target | **DRIFT** | YES for Gattling units | per Gattling unit |
| 13 | `threat_class` (armed>unarmed>building) as a coarse tie-break | implemented as tie-break #2 (`combat_targeting.rs:147-156,277`) | **DRIFT** — gamemd has no such class key; it falls out of the score | borderline — only changes equal-distance ties | occasional |
| 14 | ConvoyDisbanded one-shot weapon-range scan (T9) | none | **DRIFT** | YES after a convoy disbands | rare (convoy scripts) |
| 15 | Garrison scan radius `halfFoundation+1+OccupyWeaponRange` (T11) | override plumbed; derivation ad hoc in mod.rs | **PARTIAL** | YES for garrisoned buildings | every garrison defense |

### 4.3 What is MISSING outright

- **No score at all** — the entire `Calculate_Threat_Score`/`Evaluate_Candidate` score pipeline is absent; Rust never computes a numeric threat. (#1/#2)
- **No threat-field parsing** — `SpecialThreatValue`, `TargetSpecialThreatCoefficientDefault`, `TargetStrengthCoefficientDefault`, `TargetDistanceCoefficientDefault`, `TargetEffectivenessCoefficientDefault`, `EnemyHouseThreatBonus`, `PreferWounded` are not in `src/rules` (per GRIZZLY_MINIMAL Current Rust Delta + GRIZZLY_PASSIVE §6).
- **No ring scan / early return** — flat store iteration only. (#7)
- **No passive-acquire cadence** and **no OpportunityFire passive path** — acquisition is `OrderIntent`-only and every-tick. (#8)
- **No cloak-sensor, bridge-OnBridge, or designated-enemy-house** concepts in this helper. (#4/#5/#11)
- **No ThreatAvoidance modifier**. (#10)

### 4.4 Determinism / hash notes (default DRIFT, surfaced not triaged)

- The Rust scan order is `entities.values()` (BTreeMap → stable_id order), so its "equal-score scan-order tie" is **stable-id order**, which gamemd explicitly is NOT (gamemd preserves *array/scan* order, i.e. live reveal/insertion order). Once scoring lands, the tie-break source (scan order) must match the native live-object array order, not stable-id order — otherwise replays diverge. **UNCHECKED** whether the project's planned native live-object vector (master-TODO #1) will supply the matching order; flagged for P0/P-tie.
- `DAT_00A8EC34` is a non-hashed profiling counter; Rust correctly has no analog — do not add one.
- The cadence timer (`+0x4FC`) is **sim state** (it changes when a unit next scans). If the Rust port adds a cadence timer it MUST be hashed; if it keeps every-tick scanning it diverges from gamemd timing. **DRIFT** until cadence is modeled.

---

## 5. gamemd-native behavior contract (testable statements)

Each is a TESTABLE invariant the substrate must satisfy. These are the §8 acceptance-test targets.

**C1 — Score-ranked selection.** Among legal in-range candidates, the chosen target is the one with the highest integer threat score, NOT the nearest. *(T2/T6; LIVE 0x006F8DF0 `local_50 < score`.)*

**C2 — Strictly-greater replacement.** The best is replaced only when `new_score > best_score`; on equality the earlier-scanned candidate is kept. *(T2; LIVE — every loop guards `if ((int)local_50 < (int)param_3)`.)*

**C3 — Scan-order tie-break, not stable-id (PINNED Pass 2).** Equal-score ties resolve to the native scan order, never to stable entity id. The exact order (LIVE decompile_function 0x006F8DF0 + 0x006F8960):
- **Ring path (`flags & 3 != 0`):** ring index `r = 0 … radius_cells`. For each `r`, two inner loops walk the square perimeter: loop 1 over `dx = −r … +r` processing `(cx+dx, cy−r)` then `(cx+dx, cy+r)`; loop 2 over `dy = 1−r … r−1` processing `(cx−r, cy+dy)` then `(cx+r, cy+dy)`. (r=0 visits the center once.)
- **Within a cell** (`Scan_Cell_For_Target`): walk the occupant list from `cell+0xE8` (fallback `cell+0xE4`) via `obj+0x30` next-pointer; the **tail-most** occupant passing the alliance/range pre-filter is the cell's candidate (Evaluate_Candidate is then called on it).
- **Flat path (`flags & 3 == 0`):** `g_AircraftClass_Array[0..count]` then `g_TechnoClass_Array[0..count]` in array-index order.
A Rust port's scan order must reproduce ring-perimeter × intra-cell-list-tail × array-index, NOT BTreeMap stable-id order, or equal-score ties diverge.

**C4 — Best init = −1, null target valid.** Initial best score is −1 and initial best target is null; a scan that finds nothing returns null. *(LIVE `local_50=0xffffffff`, `local_4c=0`.)*

**C5 — Coefficient source switch.** If the attacker's owner is NOT human-controlled (`Owner+0x1FB == 0`), use the Rules "Dumb" coefficient set (`+0x1068/+0x1070/+0x1078/+0x1080/+0x1088`); otherwise use the per-type set (`Type+0x2C8/+0x2D0/+0x2D8/+0x2E0/+0x2E8`). **Exactly FIVE doubles per branch (A/B/C/D/E)** — CORRECTED Pass 2. *(LIVE disassemble_function 0x0070CD10.)*

**C5b — Two effectiveness terms + base const.** The score is `100000.0 + A·(myWarhead.Verses[candArmor]) + (±B·candWarhead.Verses[myArmor]) + C·SpecialThreatValue + [EnemyHouseThreatBonus] + D·healthRatio + max(0,d−R)·E`. Term B is negated when the candidate currently targets the attacker; B was the `* 0.0` Ghidra artifact, it is a REAL term. Base const = 100000.0 (`read_memory 0x007F4E90`). *(LIVE disassemble_function 0x0070CD10.)*

**C5c — Distance units depend on the reference.** When the reference coords == `g_NullCoord` ("use my coord", as all scanner/retaliation callers pass), the distance is reduced to **cells** (`ftol >>8`); with an explicit reference coord the distance stays in **leptons** (no `>>8`). A port must branch on the sentinel, not always divide by 256. *(LIVE 0x0070CD10 0x0070CFBC JZ split.)*

**C6 — SpecialThreatValue, not ThreatPosed.** The direct special-threat score term is `SpecialThreatValue` (`type+0x2C0` × B-coeff), NOT `ThreatPosed` (`type+0x670`). *(GRIZZLY_MINIMAL §4/§5; LIVE `type+0x2c0` read.)*

**C7 — Distance is a beyond-range penalty.** The distance term = `max(0, dist_cells − weapon_range_cells) × DistanceCoeff` (clamp-negative-to-zero); in-range targets get no distance penalty; distance is never the primary key. *(LIVE `(d<0)-1 & d` clamp; coeff `+0x1088`/`+0x2E8`.)*

**C8 — Strength term.** `GetHealthRatio(target) × StrengthCoeff` is added (stock StrengthCoeff = −200, so healthier targets score LOWER → wounded preferred via the coefficient sign). *(LIVE health-ratio multiply.)*

**C9 — EnemyHouseThreatBonus.** When the target's house index equals the attacker owner's designated enemy index (`Owner+0x5600`, ≠ −1), add `Rules+0x1090` (stock 400). *(LIVE 0x0070CD10 enemy-house add.)*

**C10 — PreferWounded modifier.** If `type+0x394` set: target health < half → score ×2; target health == 0 → score ÷2. *(TARGET_ACQUISITION §4; DOC-SOURCED.)*

**C11 — Enemy-house force-to-1.** If `Owner+0x249` is set AND the attacker owner's force-to-1 enemy index `Owner+0x1580 != -1` AND the candidate's owner is NOT `g_HouseClass_Array[that index]`, the post-score (integer) is forced to **1**. **CORRECTED Pass 2: the field is `Owner+0x1580`, NOT `Owner+0x5600`** (the latter is the enemy_only filter + EnemyHouseThreatBonus; the force-to-1 uses a different index). *(LIVE decompile_function 0x006F7CA0 LAB_006f875f.)*

**C12 — ThreatAvoidance multiply.** After the integer score, if `ThreatAvoidance_Modifier != 1.0`, multiply the score (as float) by it and `ftol`-truncate again. The modifier is `0.5^(allied buildings within ThreatAvoidanceRadius=Rules+0x1430 of the target cell)`; gated by the attacker's own building having `+0x146` set. **`DAT_007E1738 = 0.5` exactly** (Pass-1 said only "<1.0"). *(LIVE decompile_function 0x006F79A0; read_memory 0x007E1738.)*

**C12b — ftol order (P0.1 RESOLVED).** `Calculate_Threat_Score` returns float10; Evaluate_Candidate `ftol`-truncates it to int FIRST (`*piStack_c = ftol(...)`), THEN applies PreferWounded (integer ×2/÷2), enemy-house force-to-1 (integer), building/special bonuses (integer +N×1000), and finally — only if avoidance ≠ 1.0 — multiplies by the float modifier and `ftol`-truncates a second time. Truncate toward zero (matches cross-family `_ftol2` CW 0x0E7F). *(LIVE decompile_function 0x006F7CA0.)*

**C13 — Final accept clamp.** After all modifiers, `if (score != 0) { if (score < 1) score = 1; accept } else reject`. So negative scores clamp to 1 and ACCEPT; only an exactly-zero score rejects. *(LIVE decompile_function 0x006F7CA0.)*

**C14 — Verses gate.** Reject candidates whose selected warhead `Verses[target_armor] ≤ 0`. (1% "Suppressed" also blocks passive acquisition.) *(GRIZZLY_CLOAK_BRIDGE; current Rust already does the Verses>1% gate.)*

**C15 — Cloak needs sensor.** A fully-cloaked target (`CloakState == 2`) is rejected unless the target cell has positive `SensorCountForHouse(attacker_house)` OR shares the attacker's owner. Fog visibility is NOT sufficient. *(GRIZZLY_CLOAK_BRIDGE §3; DOC-SOURCED.)*

**C16 — Bridge-layer gate is scoped.** Reject on bridge-layer mismatch ONLY when BOTH attacker and target cells have the structural-bridge bit (`cell+0x140 & 0x100`) and the two `OnBridge` (`+0x8C`) flags differ. Never a generic Z/deck mismatch. *(GRIZZLY_CLOAK_BRIDGE §5; DOC-SOURCED.)*

**C17 — Limbo / dead / discovery gates.** Reject `InLimbo` (`+0x81 != 0`), zero-health (`+0x6C == 0`), and (for a human owner) candidates lacking local-human discovery bytes `+0x41A/+0x41B` per RA2 object-discovery semantics — NOT TS cell fog. *(GRIZZLY_CLOAK_BRIDGE §2/§4; DOC-SOURCED.)*

**C18 — Scan radius derivation.** Radius (cells) = `(weapon_range_leptons >> 8) + 1 + (GuardRange_leptons >> 8)`; if `weapon_range < 0 && mission == GUARD(5)` then radius = `0x200` (2 cells); no-weapon fallback uses sight. **Garrison override (CORRECTED Pass 2): if `vtable+0x400 IsOccupied()` is true (occupied buildings only) the radius is REPLACED by `GetHalfFoundationSize() + 1 + Rules.OccupyWeaponRange (+0xF48)`.** There is NO sensor branch — the Pass-1 "GetSensorRange()+1+GuardAreaTargetingDelay" was a misread. *(LIVE 0x006F8DF0 iStack_34; decompile_function 0x00458DD0/0x00458E00; decompile_function 0x0066BBB0.)*

**C19 — Ring scan + early return.** When `(flags & 3) != 0`, scan concentric squares outward from the origin cell; once any candidate exists, return immediately at ring index == `radius>>2` (quarter) or `radius/2` (half). *(LIVE 0x006F8DF0 ring loop + early-return compare.)*

**C20 — Flat array scan for the AI/no-range path.** When `(flags & 3) == 0`, scan `g_AircraftClass_Array` (if neutral bit) then all of `g_TechnoClass_Array`, same strictly-greater selection. *(LIVE 0x006F8DF0 flat branch.)*

**C21 — Alliance / enemy_only filtering.** Reject ordinary allies for weapon-bearing units (unless AttackFriendlies/Berserk/mind-control-tethered); when `enemy_only`, restrict to candidates whose house index == `Owner+0x5600`. *(LIVE 0x006F8DF0 ally + `param_4` checks.)*

**C22 — Weapon target-mask folding.** The selected weapon(s)' target-class bits are OR-ed into the scan flags before scanning (UnitClass wrapper `0x00743190`); a unit with only an anti-ground weapon will not acquire air targets and vice-versa. *(GRIZZLY_PASSIVE §2; DOC-SOURCED.)*

**C23 — ConvoyDisbanded one-shot.** A unit with `+0x688` set gets exactly one weapon-range-mode scan (bit0 set, bit1 cleared); the flag clears if that scan finds nothing. *(LIVE 0x004D9920.)*

**C24 — DontScore + player gate.** If the attacker type has `DontScore` (`type+0xD20`) AND its owner is player-controlled, Greatest_Threat returns null immediately (no auto-acquisition). *(LIVE 0x006F8DF0 entry.)*

---

## 6. Rust-native replacement boundary

A cohesive **target-scoring service** that mirrors the gamemd three-stage contract with clean Rust, owned by `sim/combat`, slotting under master-TODO #6 (target-acquisition cadence). It owns three things the current code scatters or omits: the **score** (a fixed-point value computed from pure terms), the **candidate filter** (the gate set), and the **scanner** (topology + strictly-greater selection + cadence). It exposes nothing to render/ui/audio/net (layering invariant holds — this is pure sim).

### 6.1 Module layout (proposed)

```
src/sim/combat/targeting/
  mod.rs            // service entry: greatest_threat(...) + cadence integration
  threat_score.rs   // ThreatScore value-type + ScoreTerms evaluator (pure; Calculate_Threat_Score)
  candidate_gate.rs // CandidateFilter pipeline (Evaluate_Candidate gates + post-score modifiers)
  scan.rs           // ring-expanding + flat-array topology, quarter/half early return
  cadence.rs        // per-unit passive-acquire timer + OpportunityFire gate
```

Rules-side additions (parse, `src/rules`): `SpecialThreatValue`, the four target coefficients (+effectiveness), `EnemyHouseThreatBonus`, `GuardAreaTargetingDelay`, `PreferWounded`, `ThreatAvoidanceRadius`, `OpportunityFire`. These are data, not behavior — they belong in `rules/`, consumed by `sim/`.

### 6.2 Types (sketch — fixed-point, no f32/f64)

```rust
/// Integer threat score. gamemd ftol's a float10 to int before ranking, so the
/// SELECTION key is integer. Internal accumulation uses fixed-point to mirror the
/// float math, then truncates once (matching the single ftol) before compare.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreatScore(i32);            // post-ftol integer (the rank key)

struct ScoreAccum(SimFixed);            // fixed-point accumulator, truncated to ThreatScore once

/// Pure score terms — the Calculate_Threat_Score analog. No mutation, no I/O.
pub(crate) fn score_terms(
    attacker: &AttackerSnapshot,
    attacker_obj: &ObjectType,
    candidate: &GameEntity,
    candidate_obj: &ObjectType,
    selected: &SelectedWeapon,
    ref_coords: LeptonCoord,
    coeffs: &ThreatCoeffs,              // resolved per C5 (Dumb-vs-type switch)
    enemy_house_idx: Option<HouseIdx>,  // Owner+0x5600
) -> ScoreAccum;

/// Resolved per attacker per scan (C5).
pub(crate) struct ThreatCoeffs {
    pub special: SimFixed,   // A (+0x1068 / +0x2C8)
    pub special_b: SimFixed, // B (+0x1070 / +0x2D0) — SpecialThreatValue multiplicand
    pub strength: SimFixed,  // (+0x1080 / +0x2E0)
    pub distance: SimFixed,  // (+0x1088 / +0x2E8)
    pub enemy_bonus: SimFixed, // Rules+0x1090
}

/// The gate set — Evaluate_Candidate analog. Returns Some(post-modifier score) or None.
pub(crate) fn evaluate_candidate(
    ctx: &ScanCtx,
    candidate: &GameEntity,
) -> Option<ThreatScore>;   // None = rejected (C13–C17, C21); Some = accepted, clamped >=1

/// Scanner — Greatest_Threat analog. Owns topology + strictly-greater + early return.
pub fn greatest_threat(
    ctx: &ScanCtx,
) -> Option<u64>;           // best stable_id, or None (C1–C4, C18–C20, C24)

/// Everything the scan reads, gathered once (avoids borrow churn; deterministic).
pub(crate) struct ScanCtx<'a> {
    pub entities: &'a EntityStore,
    pub rules: &'a RuleSet,
    pub interner: &'a StringInterner,
    pub attacker: &'a AttackerSnapshot,
    pub attacker_obj: &'a ObjectType,
    pub flags: ThreatFlags,             // bit0 weapon-range, bit1 guard-range, bit2 neutral, ...
    pub scan_origin: LeptonCoord,
    pub enemy_only: bool,
    pub terrain: Option<&'a ResolvedTerrainGrid>,
    pub fog: Option<&'a FogState>,      // used ONLY for ally/discovery, not as cloak gate
    pub scan_radius_cells: u16,         // derived per C18 (incl. garrison override)
    pub live_order: &'a [u64],          // native scan order for the tie-break (C3)
}
```

### 6.3 Selection rule (the heart — C1/C2/C3)

```rust
// Iterate candidates in NATIVE SCAN ORDER (live_order / ring-cell-walk), NOT stable-id order.
let mut best: Option<(ThreatScore, u64)> = None;   // (score, stable_id)
for &sid in scan_order {
    let Some(score) = evaluate_candidate(ctx, entity) else { continue };
    match best {
        Some((b, _)) if score > b => best = Some((score, sid)),  // STRICTLY greater
        None                      => best = Some((score, sid)),
        _ => {}                                                  // equal/lower: keep earlier
    }
}
best.map(|(_, sid)| sid)
```

The tie-break is *positional* (first in scan order wins on equality) — there is no id comparison. This is the single most important structural change vs the current `(dist_sq, threat_class, stable_id)` tuple.

### 6.4 Ownership / layering

- Lives entirely in `sim/combat/targeting`; depends on `rules/`, `sim/components`, `sim/entity_store`, `map/resolved_terrain`. Never touches render/ui/sidebar/audio/net.
- Cadence state (`last_scan_frame`, ConvoyDisbanded one-shot) is **per-entity sim state** → must be a hashed field on `GameEntity` (or the mission/nav substrate), advanced on the native frame basis (master-TODO #4).
- The scanner consumes the **native live-object order** (master-TODO #1) for its scan/tie-break; until that lands, a documented temporary order (keys_sorted) is used behind a shadow assert, NOT as the authoritative tie source.

---

## 7. Old ad hoc Rust logic to retire / fold into the service

| Rust symbol (file:loc) | What it does today | Disposition |
|---|---|---|
| `acquire_best_target` (`src/sim/combat/combat_targeting.rs:167`) | flat scan, inline gates, `(dist_sq, threat_class, stable_id)` rank | **RETIRE the ranking + topology**; fold its gates into `candidate_gate.rs` and replace the body with `greatest_threat`. The gate logic (alive/hostile/visible/weapon/range) is reusable; the rank key is the DRIFT to delete. |
| `acquire_best_target_for_entity` (`combat_targeting.rs:92`) | snapshot-builder wrapper | **KEEP as a thin adapter** that builds `ScanCtx` and calls `greatest_threat`. |
| `threat_class` (`combat_targeting.rs:147`) | armed>unarmed>building coarse class for tie-break | **RETIRE** — gamemd has no such key; the ordering falls out of the score. Removing it is a behavior change to verify (it currently breaks equal-distance ties). |
| The `(dist_sq, ...)` rank tuple + `best: Option<(i64,u8,u64)>` (`combat_targeting.rs:177,278-282`) | nearest-first selection | **RETIRE** — replace with `Option<(ThreatScore, u64)>` positional strictly-greater (§6.3). |
| `scan_range_override: Option<SimFixed>` threading (`combat_targeting.rs:174,232`) + garrison range derivation in `combat/mod.rs` (the `garrison_retarget_range` plumb, mod.rs ~1371/1870) | garrison scan radius | **FOLD** into `ScanCtx.scan_radius_cells` derived by C18 (gated on the §2g +0x400 resolution). |
| `tick_order_intents_pre_combat` (`src/sim/world/world_orders.rs:50`) | acquires only for `OrderIntent` entities, every tick | **EXTEND** to a cadence-gated passive path that also covers ordinary `OpportunityFire` movers; keep AttackMove/Guard coords on OrderIntent. |
| Inline `FogState::is_cell_visible` as the visibility gate (`combat_targeting.rs:198`) | TS-ish cell fog | **REPLACE** with object-discovery semantics (C17) + the cloak-sensor gate (C15); fog stays for ally checks only. |
| 3× duplicated `acquire_best_target` retarget call sites in `combat/mod.rs` (~1870, ~1950, ~1967) | re-acquire on friendly-target / invisible-target / dead-target | **FOLD** behind the single `greatest_threat` entry; the three triggers stay, the scanner unifies. |

---

## 8. Migration slices + acceptance tests

Shadow-first, dependency-ordered, each independently shippable, mirroring the Mission/Radio rhythm. **P0 is a BLOCKING research gate** — no scoring becomes authoritative until it closes.

### P0 — Research gate (mostly CLOSED Pass 2)

1. **Float→int conversion order — CLOSED (C12b).** ftol truncates the float10 score to int FIRST; PreferWounded/force-to-1/bonuses are integer; ThreatAvoidance multiplies-then-truncates a second time. Truncate toward zero. *(LIVE decompile_function 0x006F7CA0.)*
2. **+0x400 slot identity — CLOSED (§2g).** = `IsOccupied` (base = `return 0` stub, BuildingClass override `0x00458DD0`); +0x404 = `GetHalfFoundationSize`. NO sensor branch. Garrison radius = `GetHalfFoundationSize()+1+OccupyWeaponRange`. *(read_memory + decompile this run.)*
3. **Tie-break order source — PINNED (C3).** Native order = ring-perimeter walk × intra-cell `+0xE8→+0x30` list (tail-most) × array-index. **Remaining UNCHECKED:** how the project's planned live-object vector (master-TODO #1) supplies cell-occupant insertion order to match `cell+0xE8` chaining; until that lands, equal-score replay parity is UNCHECKED. Next query when the live-object vector exists: confirm Rust cell-occupant ordering equals gamemd reveal/unlimbo insertion order.
4. **Effectiveness algebra — CLOSED (§2b, C5/C5b).** TWO terms: `A·myWarhead.Verses[candArmor]` + `±B·candWarhead.Verses[myArmor]`; FIVE coeffs; base 100000.0. Reduced to exact arithmetic above.

**Net: P0 is no longer blocking.** Only residual is the cell-occupant insertion-order mapping (item 3) which depends on master-TODO #1 landing first; it does not block P1–P5 shadow work.

### P1 — Parse threat data (additive, no behavior change)

Add to `rules/`: `SpecialThreatValue`, the 4 target coefficients + effectiveness, `EnemyHouseThreatBonus`, `GuardAreaTargetingDelay`, `PreferWounded`, `ThreatAvoidanceRadius`, `OpportunityFire`. No consumer yet.
- **Test `threat_fields_parse_stock_defaults`**: assert `[General] TargetStrengthCoefficientDefault == -200`, `TargetDistanceCoefficientDefault == -10`, `EnemyHouseThreatBonus == 400`, `[MTNK] SpecialThreatValue` parses (absent → 0), `OpportunityFire` true for MTNK.

### P2 — Pure score evaluator (shadow)

Implement `threat_score.rs` (C5–C9) as a pure function. Compute it alongside the existing nearest-first pick but DO NOT use it for selection. Log/shadow-assert nothing yet.
- **Test `score_terms_coeff_switch_dumb_vs_type`**: AI-owned attacker reads Dumb set; human-owned reads per-type set.
- **Test `score_distance_is_beyond_range_penalty_only`**: two equal-non-distance candidates, one in-range one beyond — only the beyond-range one takes a penalty; in-range distance contributes 0.
- **Test `score_special_threat_uses_specialthreatvalue_not_threatposed`**: candidate A `SpecialThreatValue=1`/low ThreatPosed beats candidate B high `ThreatPosed`/zero SpecialThreatValue with other terms controlled. *(mirrors GRIZZLY_MINIMAL handoff)*

### P3 — Candidate gate module (shadow)

Implement `candidate_gate.rs` (C13–C17, C21) returning `Option<ThreatScore>`. Run in shadow next to the existing inline gates; shadow-assert the accept/reject set matches the current gates on the OVERLAPPING gates (alive/hostile/visible/weapon/range), and record the NEW gates (cloak-sensor, bridge-OnBridge, discovery) as additive.
- **Test `grizzly_passive_scan_rejects_unsensed_cloaked_enemy`** (C15).
- **Test `grizzly_passive_scan_rejects_bridge_cell_onbridge_mismatch`** (C16).
- **Test `candidate_gate_rejects_limbo_and_zero_health`** (C17).

### P4 — Scanner + strictly-greater selection (shadow → invert)

Implement `scan.rs` + `greatest_threat` (C1–C4, C18–C20). First run in shadow: compute the score-ranked pick AND the legacy nearest pick; shadow-assert they agree on single-candidate cases. Then INVERT: make score-ranked authoritative, keep nearest as a debug shadow.
- **Test `grizzly_passive_scan_prefers_higher_threat_over_nearest`** (C1): farther high-score enemy chosen over nearer low-score. *(the headline parity fix)*
- **Test `greatest_threat_strictly_greater_keeps_scan_order_on_tie`** (C2/C3): two equal-score candidates → first in scan order wins; swapping scan order swaps the winner; stable-id order does NOT decide.
- **Test `greatest_threat_empty_returns_none`** (C4).

### P5 — Scan topology: ring + early return (shadow → authoritative)

Implement ring-expanding squares with quarter/half early return (C19) and the flat-array path (C20). Shadow-assert the chosen target matches the flat-scan result in non-crowded cases; document the crowded-case divergence as the intended early-return behavior.
- **Test `ring_scan_early_returns_at_quarter_radius`** (C19): a candidate at quarter radius prevents a higher-score candidate beyond half radius from being considered.
- **Test `scan_radius_includes_guardrange_and_guard_degenerate`** (C18).

### P6 — Cadence + OpportunityFire passive path (shadow → authoritative)

Add per-entity passive-scan cadence timer + ConvoyDisbanded one-shot (C23) and wire ordinary moving `OpportunityFire` units into acquisition (retiring the `OrderIntent`-only gate per §7). Hash the new timer field; bump `SNAPSHOT_VERSION`.
- **Test `opportunity_fire_mover_acquires_on_cadence_not_every_tick`**: a moving MTNK acquires a passerby on its scan-cadence frame, not every tick.
- **Test `convoy_disbanded_one_shot_weapon_range_scan`** (C23).
- **Test `passive_scan_uses_weapon_range_not_guardrange_for_mtnk`** (GRIZZLY_PASSIVE handoff): MTNK with no GuardRange uses 105mm Range=5.

### P7 — Post-score modifiers + parity harness (authoritative)

Add PreferWounded (C10), enemy-house force-to-1 (C11), ThreatAvoidance (C12), final clamp (C13). Drop all shadow asserts; make the service the sole target picker; add a deterministic replay parity harness over a fixed skirmish seed.
- **Test `prefer_wounded_doubles_score_below_half_health`** (C10).
- **Test `enemy_house_bonus_and_force_to_one`** (C9/C11).
- **Test `threat_scoring_replay_hash_stable`**: fixed-seed skirmish replay produces a stable state hash across runs (lockstep/MP guard).

---

## 9. Sources & verification ledger

**LIVE-VERIFIED this session (2026-06-04):**
- `get_function_by_address 0x006F8DF0` → TechnoClass__Greatest_Threat (body 006f8df0–006f9dae).
- `get_function_by_address 0x006F7CA0` → TechnoClass__Evaluate_Candidate (body 006f7ca0–006f895a).
- `get_function_by_address 0x0070CD10` → TechnoClass__Calculate_Threat_Score (body 0070cd10–0070d0cf).
- `get_function_by_address 0x006F8960` → TechnoClass__Scan_Cell_For_Target (body 006f8960–006f8c0f).
- `decompile_function 0x0070CD10` → coefficient switch on `Owner+0x1FB`; Rules `+0x1068/+0x1070/+0x1080/+0x1088` vs Type `+0x2C8/+0x2D0/+0x2E0/+0x2E8`; SpecialThreatValue `type+0x2C0`; EnemyHouseThreatBonus `Rules+0x1090` gated by `Owner+0x5600`; distance clamp `(d<0)-1 & d`; base const `_DAT_007F4E90`; 4 doubles/branch.
- `decompile_function 0x006F8DF0` → call counter `DAT_00A8EC34++` at entry; `local_50=-1`/`local_4c=0` init; strictly-greater `if ((int)local_50 < (int)param_3)` in all 5 candidate loops; flat (flags&3==0) + ring (flags&1|2) topology; quarter/half early return; sensor branch `vtable+0x400`/`+0x404`/`Rules+0xF48`; DontScore+IsPlayerControl entry return-0; `Owner+0x5600` enemy_only + bonus; GuardRange `type+0x68C`; Gattling `+0x440/+0x460` arrays; Cell_Threat_Fallback.
- `decompile_function 0x004D9920` → ConvoyDisbanded `+0x688` forces bit0/clears bit1, delegates, clears flag on no-target.
- `get_xrefs_to 0x00A8EC34` → ONLY self read (006f8df3) + write (006f8e09) → confirms profiling-only, non-hash.
- `get_function_callers 0x006F8DF0` → FUN_00445f00, FUN_004D9920.

**DOC-SOURCED (prior verified reports, not re-read live this session):**
- `TARGET_ACQUISITION_GHIDRA_REPORT.md` §§2–9 — scanner architecture, Evaluate_Candidate filter pipeline, score modifiers, struct offsets.
- `GRIZZLY_GREATEST_THREAT_SCORE_MINIMAL_FORMULA_GHIDRA_REPORT.md` — strictly-greater ranking, ring early-return, SpecialThreatValue-not-ThreatPosed, stock coefficient defaults, minimal Rust ranker handoff.
- `GRIZZLY_PASSIVE_TARGET_SCANNER_VTABLE_39C_GHIDRA_REPORT.md` — passive driver chain (vtable+0x39C → 0x00709820 → +0x3C4 → 0x00743190 → FUN_004D9920 → Greatest_Threat), weapon-range fallback, filters, current Rust delta.
- `GRIZZLY_OPPORTUNITYFIRE_VISIBILITY_CLOAK_BRIDGE_FILTERS_GHIDRA_REPORT.md` — cloak-sensor gate, local-human discovery vs TS fog, scoped bridge-OnBridge gate, limbo/dead gates.
- `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md` — the misnomer (0x004D5690 is the approach driver, NOT a scanner); FUN_004D9920 wrapper semantics; ConvoyDisbanded.
- `GARRISON_SYSTEM_GHIDRA_REPORT.md` §10/§15a/§15b — garrison scan-radius formula + cell-scan mode (+0x400 slot identity UNCHECKED).
- `OPPORTUNITY_FIRE_GHIDRA_REPORT.md` — OpportunityFire at type+0x6AF gates passive acquisition.

**Rust source read:** `src/sim/combat/combat_targeting.rs` (full), `src/sim/combat/mod.rs` (acquisition call sites ~559/1371/1611/1870/1950/1967/2432), `src/sim/world/world_orders.rs` (tick_order_intents_pre/post_combat).

**Master roadmap:** `docs/plans/2026-05-29-core-engine-substrate-todo.md` item #6 (target-acquisition cadence), #1 (native live-object vector for scan order), #4 (native-frame timing for cadence).

**INI:** `ini/rulesmd.ini` `[General]` target coefficients + `EnemyHouseThreatBonus`, `[MTNK] OpportunityFire/ThreatPosed/SpecialThreatValue`, `[105mm] Range`; `ini/rules.ini` fallback.

**UNCHECKED / blocking (Pass 2 status):** (1) ftol order — **CLOSED** (C12b, decompile_function 0x006F7CA0); (2) +0x400 identity — **CLOSED** = IsOccupied (read_memory 0x007F4D60/0x007E42BC + decompile_function 0x00458DD0/0x00458E00); (3) effectiveness algebra — **CLOSED** (§2b, disassemble_function 0x0070CD10). **Remaining UNCHECKED (non-blocking):** cell-occupant insertion-order mapping for equal-score replay parity — depends on master-TODO #1 native live-object vector; next query when it exists: verify Rust cell-occupant order == gamemd `cell+0xE8` reveal/unlimbo chaining. Also UNCHECKED: which class' vtable binds the second mask-OR wrapper `0x00445F00` (likely BuildingClass +0x3C4; get_xrefs_to 0x00445F00 → 0x007E4280 DATA, i.e. BuildingClass vtable region — see §10).

---

## Reviewer follow-ups (adversarial pass 2026-06-04)

Adversarial review re-verified the load-bearing binary claims LIVE this session; the doc holds up. Verdict: GREEN with one completeness patch applied.

**Re-verified LIVE (read out of Ghidra this review):**
- `get_xrefs_to 0x00A8EC34` → only self read (006f8df3) + write (006f8e09). Profiling-only / non-hash claim CONFIRMED.
- `decompile_function 0x006F8DF0` → `DAT_00a8ec34++` at entry; `local_4c=0`/`local_50=0xffffffff` init; strictly-greater `(int)local_50 < (int)…` in every candidate loop; `vtable+0x84` DontScore-`+0xd20` + `IsPlayerControl` entry return-0; ~~sensor branch~~ **garrison branch (SUPERSEDED by §10/§2g — it is `vtable+0x400 IsOccupied`, not sensor)** `vtable+0x400`→`vtable+0x404`+`Rules+0xf48`; quarter/half early return `iStack_48 == iStack_34>>2 || == iStack_34/2`; GuardRange `+0x68c`; Gattling `field_0x440/0x460`; `Owner+0x5600` enemy_only. C1–C4, C18–C24 CONFIRMED.
- `decompile_function 0x0070CD10` → coeff switch on `Owner+0x1FB`; ~~Dumb set `…` (4 doubles)~~ **SUPERSEDED by §2b/§10 — FIVE doubles `+0x1068/0x1070/0x1078/0x1080/0x1088` (Dumb) and `+0x2C8/0x2D0/0x2D8/0x2E0/0x2E8` (per-type); the `*0.0` was a Ghidra artifact for coeff B**; SpecialThreatValue candidate `type+0x2c0`; EnemyHouseThreatBonus `Rules+0x1090` gated by `Owner+0x5600`; distance clamp `(d<0)-1 & d`; base `_DAT_007f4e90` = 100000.0. C5–C9 CONFIRMED (with the FIVE-doubles + two-effectiveness-terms correction).
- `decompile_function 0x004D9920` → `+0x688` set ⇒ `param_2 & 0xfffffffd | 1` (bit0 set / bit1 clear), delegate, clear `+0x688` on null. C23 CONFIRMED.
- `get_function_by_address 0x004D5690` → `FootClass__Greatest_Threat_Scan` (body 004d5690–004d6a95). Misnomer (approach driver, out of family) CONFIRMED by symbol identity.
- `get_function_by_address 0x006F7CA0` → body 006f7ca0–006f895a. Matches §2a.

**Rust retire-list re-verified (Grep/Read):** `threat_class @ combat_targeting.rs:147`; `best: Option<(i64,u8,u64)> @ :177`; `(dist_sq, class, stable_id)` rank tuple @ `:278`; `is_cell_visible` fog gate @ `:198-199`; `order_intent.is_some()` acquisition gate @ `world_orders.rs:56`; three `acquire_best_target` retarget call sites @ `mod.rs:1870/1950/1967`; `garrison_retarget_range` @ `mod.rs:1787`. All present and doing what the doc says.

**PATCH applied:** §2a gained `0x00445F00`, a second weapon-mask-OR wrapper that calls `TechnoClass::Greatest_Threat` directly (it was in `get_function_callers 0x006F8DF0` but absent from the inventory). It mirrors `0x00743190`'s mask-OR role on a different class path. **Residual (non-blocking):** which class' vtable binds `0x00445F00` was not adjudicated this pass (likely Infantry/Building +0x3C4 sibling of UnitClass `0x00743190`); confirm its vtable slot when the mask-folding work lands so the C22 weapon-target-mask contract covers all entry classes, not just UnitClass.

**Burden-of-proof spot-check:** no DRIFT was downgraded to internal-only without proof — every §4.2 row defaults to DRIFT/MISSING with player-visibility + trigger frequency stated, consistent with the bar. The DAT_00A8EC34 "do not model" is the only equivalence claim and it is backed by an exhaustive xref check (bit-identical: zero gameplay reads), which is acceptable evidence.

**Program-fit / TS-legacy:** shadow-first P0→P7 rollout, SNAPSHOT_VERSION bump gated to P6 (cadence field), no sim→render dependency, no software-blitter port; fog-darkening / `+0x3D5` underground RNG / house-only bit-8 scan are all correctly quarantined as DORMANT/DEFERRED. No substrate is designed around a TS-only path.

---

## 10. Pass 2 — Expansion (gate closures + completeness sweep, 2026-06-04)

All addresses/offsets below read out of Ghidra THIS run. This section both records the gate resolutions and folds the material findings into §2/§5/§7/§8.

### 10.1 Gate closures (JOB A)

| Gate | Pass-1 status | Pass-2 verdict | Evidence (this run) |
|---|---|---|---|
| +0x400 slot identity | UNCHECKED (Is_Sensor vs IsOccupied) | **WRONG→VERIFIED: `IsOccupied`** (no sensor branch) | TechnoClass vtable base 0x007F4960 (get_xrefs_to 0x006F8DF0 → DATA 0x007F4D24=+0x3C4). read_memory 0x007F4D60 (+0x400)=0x0041BFB0=`return 0` stub; read_memory 0x007F4D64 (+0x404)=0x0041BFC0=`return 0` stub. read_memory 0x007E42BC (BuildingClass +0x400)=0x00458DD0=`BuildingClass__IsOccupied`; 0x007E42C0 (+0x404)=0x00458E00=`GetHalfFoundationSize`. UnitClass +0x400 (read_memory 0x007F6070)=0x0041BFB0 stub. decompile_function 0x00458DD0/0x00458E00. |
| Score effectiveness algebra | DOC-ONLY ("4 doubles", "* 0.0") | **WRONG→VERIFIED: 5 doubles, 2 effectiveness terms, base 100000.0** | disassemble_function 0x0070CD10 (coeff loads 0x0070CD58–0x0070CE23; FMULs 0xceb2/0xcec7/0xcee0/0xcf49/0xcf5f; clamp 0xd0a4–0xd0b1). read_memory 0x007F4E90=100000.0; 0x007E2800=0.0. |
| Native scan-order tie source | UNCHECKED | **PINNED** (ring-perimeter × intra-cell `+0xE8→+0x30` tail × array-index) | decompile_function 0x006F8DF0 (ring loops) + 0x006F8960 (cell list walk `cell+0xE8`/`+0xE4`, `obj+0x30` next). |
| ftol order vs modifiers | UNCHECKED | **VERIFIED** (truncate-first, integer modifiers, 2nd truncate on avoidance) | decompile_function 0x006F7CA0. |
| Rules+0xF48 INI key | "GuardAreaTargetingDelay" (DOC) | **WRONG→VERIFIED: `OccupyWeaponRange`** (`[CombatDamage]`) | decompile_function 0x0066BBB0 (`s_OccupyWeaponRange_0083b064` → ReadInt → +0xF48). |

### 10.2 NEW consumers / methods found (JOB B sweep)

| Symbol | Address | Role | How found | Status |
|---|---|---|---|---|
| `TechnoClass::ShouldRetaliate` | 0x007087C0 | **Retaliation target-switch gate** — calls `Calculate_Threat_Score(current_target)` vs `(attacker)`; keeps current target if it scores higher; final Verses ≤ 0.01 reject. Second direct score consumer outside the scanner. Player-visible: governs whether a unit abandons its target to shoot back. | get_function_callers 0x0070CD10 | LIVE-VERIFIED (decompile_function 0x007087C0). **NEW — added to §2a, contract C-set should add a retaliation invariant.** |
| `TechnoClass::ThreatAvoidance_Modifier` | 0x006F79A0 | Already listed; now LIVE — confirmed sole caller is Evaluate_Candidate; const 0.5; gated by attacker building `+0x146`; radius `Rules+0x1430`. | get_function_callers 0x006F79A0 | Upgraded DOC-SOURCED → LIVE-VERIFIED. |
| `BuildingClass::IsOccupied` | 0x00458DD0 | +0x400 override (garrison gate). | read_memory + decompile | LIVE-VERIFIED. |
| `BuildingClass::GetHalfFoundationSize` | 0x00458E00 | +0x404 override (garrison radius). | read_memory + decompile | LIVE-VERIFIED. |
| `Sqrt_Approx` | 0x004CAC40 | distance sqrt in score + Evaluate_Candidate. | get_function_callees / disassembly | LIVE-VERIFIED (get_function_by_address). |
| `Math__ftol` | 0x007C5F00 | the single float→int truncate (matches cross-family ftol gate). | disassembly call | LIVE-VERIFIED. |

### 10.3 NEW globals / fields found

| Symbol | Address/offset | Role | Status |
|---|---|---|---|
| `DAT_007F4E90` | 0x007F4E90 = **100000.0** | score base const (Pass-1 had no value; TARGET_ACQUISITION guessed "~0.5") | LIVE-VERIFIED (read_memory). DOMINATES the score → ranking decided by small deltas. |
| `DAT_007E1738` | 0x007E1738 = **0.5** | ThreatAvoidance per-building multiplier (Pass-1 said only "<1.0") | LIVE-VERIFIED (read_memory). |
| `DAT_007E2800` | 0x007E2800 = **0.0** | early-return const (candidate lacks weapon-presence flag `vtable+0x88`) | LIVE-VERIFIED (read_memory). |
| `Owner+0x1580` | HouseClass | **force-to-1 enemy index** (Evaluate_Candidate), gated by `Owner+0x249` — DISTINCT from `Owner+0x5600` (enemy_only + bonus). Pass-1 conflated them in C11. | LIVE-VERIFIED (decompile 0x006F7CA0). |
| coeff **C** offset | Rules+0x1078 / Type+0x2D8 | SpecialThreatValue multiplier (Pass-1 placed SpecialThreat coeff at 0x1080/0x2E0 — that is the STRENGTH coeff D). | LIVE-VERIFIED (disassemble 0x0070CD10). |
| `Rules+0x16F8` (ConditionYellow) | used in Scan_Cell_For_Target heal-target branch + Evaluate_Candidate ally-repair branch | already listed; confirmed read live in 0x006F8960. | LIVE-VERIFIED. |
| `Rules+0x1708` (ConditionRed) | building-defense switch in Scan_Cell_For_Target + Evaluate_Candidate ally-building branch | confirmed read live. | LIVE-VERIFIED. |

### 10.4 Vtable-class resolution (reviewer residual closed)

`0x00445F00` (the second weapon-mask-OR direct caller of Greatest_Threat) is bound at **BuildingClass vtable +0x3C4** — get_xrefs_to 0x00445F00 → DATA 0x007E4280, and BuildingClass vtable base = 0x007E3EBC (since its +0x400 = 0x007E42BC = `IsOccupied`), so 0x007E4280 = base+0x3C4. So the C22 weapon-target-mask folding has THREE entry classes: UnitClass +0x3C4 (`0x00743190`), FootClass +0x3C4 (`0x004D9920`, the base wrapper), and BuildingClass +0x3C4 (`0x00445F00`). The Rust mask-folding must cover building scanners too. LIVE-VERIFIED (get_xrefs_to 0x00445F00; read_memory 0x007E42BC anchors the base).

### 10.5 Edge cases / TS-legacy re-examined

- **No sensor/"detector" branch exists** in Greatest_Threat — the Pass-1 sensor framing was a phantom. The only override is garrison occupancy (buildings). No detector-range mechanic to model here. (The cloak-needs-sensor gate is separate, in Evaluate_Candidate via `CellClass__SensorCountForHouse` — that IS real and stays, C15.)
- **`+0x3D5` reject gate** confirmed read in Evaluate_Candidate (decompile 0x006F7CA0: `this->field_0x3d5 == 0` OR mission-timer/`vtable+0x1c8 < -0x14` → reject). Its semantics remain unproven (treat as generic map-state reject, per §2h) — NOT TS underground RNG. Status unchanged: DOC-level meaning UNCHECKED, gate-existence VERIFIED.
- **Building/special post-score bonus flags** (`piStack_1c & 0x800/0x8000/0x10000/0x1000/0x2000`, each adding occupant×1000 or forcing 0/1) are AI-planning (the flags are only set on the house-only/AI scan path `0x8042`). Confirmed live in Evaluate_Candidate; keep DEFERRED-AI. The per-unit human-combat path (`uStack_58` without bit 8) does not set them.
- **`g_ScenarioClass+0x800` "no-target list"** (Evaluate_Candidate: scans `Rules+0xB40[count 0xB4C]` and rejects matching types) is a scenario/mission gate, active only when scenario flag 0x800 set — flag for mission scripting, not standard skirmish per-unit combat. Status: ACTIVE-conditional, low skirmish frequency.

### 10.6 Burden-of-proof re-applied to own doc (Pass 2)

- The Pass-1 "exactly 4 doubles" and "* 0.0 effectiveness" claims were stated as LIVE-VERIFIED but were decompiler artifacts → reclassified WRONG and corrected from the **disassembly** (decompile alone was insufficient; this is a re-decoder-ring lesson: float-stack code needs the asm, not the C). 
- The "+0x400 Is_Sensor" claim was DOC/inference, not bit-verified → WRONG, corrected by vtable byte reads.
- The C11 force-to-1 field `Owner+0x5600` was an assumption carried from the enemy_only filter → WRONG, it is `Owner+0x1580`.
- No remaining equivalence/internal-only downgrade lacks proof. `DAT_00A8EC34` non-hash remains the only equivalence claim, backed by exhaustive xref (unchanged).

### 10.7 Contract / slice deltas from Pass 2

- **§5 contract:** C3 pinned, C5 corrected to FIVE coeffs, C5b/C5c/C12b added, C11 field corrected, C12 const corrected, C18 garrison-not-sensor. **Add a new C25 — Retaliation score-comparison:** *a unit hit by an attacker retains its current target if `Calculate_Threat_Score(current) > Calculate_Threat_Score(attacker)` (both my-coord), else may switch; final Verses[armor] ≤ 0.01 rejects the attacker as a retaliation target.* (LIVE 0x007087C0.) This belongs in the per-unit combat scope (retaliation is observable) and should get a P3/P7 test once the score evaluator exists.
- **§7 retire-list:** the Rust port must additionally route the **retaliation** decision (combat/mod.rs retaliation path) through the same score evaluator, not a separate ad-hoc rule — otherwise retaliation target-switching drifts from gamemd even after the scanner is fixed.
- **§8 slices:** P0 downgraded from BLOCKING to "mostly closed" (only the cell-occupant insertion-order mapping remains, dependent on master-TODO #1). P2 must implement FIVE coeffs and BOTH effectiveness terms with base 100000.0 and the my-coord/explicit-coord distance-unit branch. A new test belongs in P7: `retaliation_keeps_higher_scored_current_target` (C25).

### 10.8 Remaining blocking gates

**None blocking shadow work.** One UNCHECKED dependency for *authoritative* equal-score replay parity: the Rust cell-occupant ordering must equal gamemd's `cell+0xE8` reveal/unlimbo insertion chain — resolvable only after master-TODO #1 (native live-object vector) lands. Next query then: trace cell-occupant insertion order in `ObjectClass::Mark`/unlimbo and diff against the Rust EntityStore cell index.
