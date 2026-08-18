# Grizzly Greatest Threat Score Minimal Formula - Ghidra Report

**Target:** `GRIZZLY_GREATEST_THREAT_SCORE_MINIMAL_FORMULA`  
**Primary functions:** `TechnoClass__Greatest_Threat @ 0x006F8DF0`, `TechnoClass__Evaluate_Candidate @ 0x006F7CA0`, `TechnoClass__Calculate_Threat_Score @ 0x0070CD10`  
**Investigation mode:** narrow `/re-swarm` slot 3  
**Status:** COMPLETE for minimal stock MTNK passive-acquisition ranking; full all-object threat model remains out of scope.

## Working Notes

- Target question: Bound the minimal `Greatest_Threat` / `Evaluate_Candidate` scoring behavior needed for stock MTNK OpportunityFire passive move-by target choice.
- Non-goals: Do not re-resolve the settled UnitClass scanner chain; do not exhaustively model all threat flags, all target classes, AI house planning, or projectile/fire timing.
- Evidence needed to mark COMPLETE: Verify strict best-score update and tie behavior, ring scan/early return, score formula inputs that can beat nearest-first, and whether `ThreatPosed=` is directly consumed.
- Stop conditions: Stop once stock MTNK can be handed to Rust as a bounded ranker and remaining formula details are isolated as nonblocking or conditional.

## Summary

Stock MTNK passive acquisition is not nearest-first. The local cell/ring scanner feeds legal candidates to `TechnoClass__Evaluate_Candidate`, which writes an integer score. `TechnoClass__Greatest_Threat` keeps the candidate only when the new score is strictly greater than the current best score; equal scores preserve earlier scan order. The nearby scan can also return early once a candidate has been found at the quarter-radius or half-radius boundary, so the implementation should not scan the whole world and sort by stable id.

For a normal stock Grizzly with `[105mm] Range=5`, the minimal score comes from `TechnoClass__Calculate_Threat_Score @ 0x0070CD10`, then post-processing in `Evaluate_Candidate`. The verified stock formula inputs include selected weapon effectiveness, target `SpecialThreatValue`, target health ratio/strength, selected weapon range, distance beyond weapon range, `EnemyHouseThreatBonus`, and the target cell's `ThreatAvoidanceCoefficient` modifier. `ThreatPosed=` was not found as a direct term in this scoring slice; it is parsed into a TechnoType int elsewhere, but the direct score term at `type+0x2C0` is `SpecialThreatValue`, not `ThreatPosed`.

## Verified Binary Findings

### 1. Strict greater-than ranking, equal-score tie preservation

**Active in YR:** Yes.

`Greatest_Threat` initializes best score to `-1` and updates only on strict greater-than. In the nearby-cell path, the compare/update pattern appears at:

- `0x006F954B..0x006F955F`: compares current best against candidate score and updates only if current best is lower.
- Repeated equivalent blocks at `0x006F96F8..0x006F970C`, `0x006F98DD..0x006F98F1`, `0x006F9A91..0x006F9AA5`.
- Global-array paths use the same strict-greater pattern, e.g. `0x006F9C34..0x006F9C44` and `0x006F9D7F..0x006F9D8F`.

Because the branch skips update when `current >= new`, equal scores do not replace the earlier candidate. For Rust, stable-id sorting is therefore not a parity tie-breaker for this path; scan order is.

### 2. Ring scan with early return at quarter/half radius

**Active in YR:** Yes.

The nearby scan expands around the current cell using ring index stored at `[ESP+0x24]` and scan radius `[ESP+0x38]`. After a candidate exists, it returns early when the current ring equals either roughly one quarter or one half of the scan radius:

- `0x006F9AE2..0x006F9B0C`: checks best candidate and compares ring index against `(radius + sign_adjust) >> 2` and `radius / 2`.
- `0x006F9B49..0x006F9B52`: returns best candidate.

This means later, farther cells may not be considered once an acceptable nearer-ring candidate exists by those boundaries. This is player-visible only in crowded edge cases, but it is a real part of the passive scan contract.

### 3. Score source is `Calculate_Threat_Score`, then integer ftol, then modifiers

**Active in YR:** Yes.

`Evaluate_Candidate` performs legality/range/visibility checks, then calls `TechnoClass__Calculate_Threat_Score @ 0x0070CD10`:

- Call and score write: `0x006F86FE..0x006F8719` calls `0x0070CD10`, converts float to int with `0x007C5F00`, and stores through the score out-param.
- Health-state modifier: `0x006F8721..0x006F875F` halves, doubles, or leaves score based on attacker type field `+0x394` and candidate health/current strength.
- Enemy-house override: `0x006F875F..0x006F8792` can force score to `1` for a configured enemy-house case.
- Flagged building/special bonuses: `0x006F8792..0x006F883C` can zero or add large building/special values for non-stock flag combinations.
- Threat avoidance modifier: `0x006F88BF..0x006F8928` multiplies score by `TechnoClass__ThreatAvoidance_Modifier @ 0x006F79A0` when not 1.0.
- Final accept clamps negative positive scores to at least `1`, rejects zero: `0x006F8928..0x006F8948`.

For ordinary stock MTNK move-by fire, the important consequence is that rank is a computed score, not `(distance, class, id)`.

### 4. Minimal stock score terms include effectiveness, SpecialThreatValue, strength, range, and distance

**Active in YR:** Yes.

`TechnoClass__Calculate_Threat_Score @ 0x0070CD10` chooses either global default target coefficients or object-specific coefficients, then combines weapon and target terms:

- Coefficient source: `0x0070CD48..0x0070CDBC` uses attacker TechnoType coefficient fields when a type byte at attacker type `+0x1FB` is set; otherwise `0x0070CDBE..0x0070CE27` reads Rules offsets `+0x1068..+0x108C`.
- Weapon effectiveness terms: `0x0070CE27..0x0070CF54` selects attacker and candidate weapons and adds warhead-verses/effectiveness contributions.
- `SpecialThreatValue` term: `0x0070CED2..0x0070CEF0` multiplies target type `+0x2C0` by the target special-threat coefficient; `TechnoTypeClass::ReadINI @ 0x00715726..0x00715748` reads `SpecialThreatValue` into `+0x2C0`.
- Enemy-house bonus: `0x0070CEF0..0x0070CF19` adds `Rules+0x1090`; rulesmd has `EnemyHouseThreatBonus=400`.
- Health/strength term: `0x0070CF58..0x0070CF69` multiplies `ObjectClass__GetHealthRatio` by a coefficient before adding to the score.
- Range and distance term: `0x0070CF6D..0x0070D0C0` converts selected weapon range to cells, computes distance, subtracts weapon-range cells, clamps negative to zero, multiplies by distance coefficient, and adds it.

The relevant stock defaults in `ini/rulesmd.ini` are `TargetSpecialThreatCoefficientDefault=200`, `TargetStrengthCoefficientDefault=-200`, `TargetDistanceCoefficientDefault=-10`, and `EnemyHouseThreatBonus=400`. `TargetEffectivenessCoefficientDefault=-200` is present too and participates through selected weapon effectiveness.

### 5. `ThreatPosed=` is not the direct score term in this slice

**Active in YR:** Conditional for other AI consumers; not direct for this minimal `Calculate_Threat_Score` term.

The earlier Grizzly scanner report correctly established that target selection is threat-score based, but the direct field seen in `Calculate_Threat_Score` is not `ThreatPosed=`:

- `SpecialThreatValue` reader: `TechnoTypeClass::ReadINI @ 0x00715726..0x00715748` reads string `0x0084342C` into TechnoType `+0x2C0` as a double.
- `Calculate_Threat_Score` direct read: `0x0070CEDC..0x0070CEE6` multiplies target type `+0x2C0`.
- Existing docs identify `ThreatPosed` parser separately as TechnoType `+0x670` near `0x007149CE`; no direct `+0x670` read was found inside `0x0070CD10` or the post-score block `0x006F86FE..0x006F8948`.

Inference: `ThreatPosed=` may still be consumed by other AI threat systems, but it should not be used as the direct replacement for `SpecialThreatValue` in the stock MTNK passive target score.

## Minimal Rust-Compatible Ranking Model

For a first parity patch of stock Grizzly passive acquisition, implement a separate YR-style passive ranker rather than reusing nearest-first `acquire_best_target` unchanged:

1. Filter candidates using the already-known `Evaluate_Candidate` gates: live, hostile/legal, visible/sensed, weapon-compatible, in `[105mm] Range=5`, and bridge-layer compatible when bridge data is available.
2. Score legal candidates with a small subset of `Calculate_Threat_Score`:
   - selected weapon effectiveness / verses term,
   - target `SpecialThreatValue`,
   - target health ratio and strength,
   - selected weapon range,
   - distance beyond selected weapon range, clamped at zero,
   - `EnemyHouseThreatBonus` only when the owner-house condition is represented.
3. Keep the candidate only when `new_score > best_score`.
4. Preserve scan-order ties. Do not use stable id as a tie-breaker for this passive path.

For common stock MBT-vs-MBT scenarios, `SpecialThreatValue` is usually absent/zero, so the differentiators are weapon effectiveness, strength/health, range, and distance penalty. A farther target can beat a nearer target when its non-distance terms exceed the extra distance penalty.

## Current Rust Delta

Current Rust `src/sim/combat/combat_targeting.rs` documents and implements nearest-first ranking: `best: Option<(i64, u8, u64)>`, then `rank = (dist_sq, class, candidate.stable_id)`. `threat_class` only breaks equal-distance cases by broad armed/object category. `src/sim/world/world_orders.rs` currently calls acquisition for entities with `order_intent`, not ordinary moving `OpportunityFire` units.

The direct rules data support is also incomplete for this target: the focused scan did not find parsed `SpecialThreatValue`, target threat coefficients, or `EnemyHouseThreatBonus` in `src/rules` or `src/sim`.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Passive scan ranks by score and updates only on `new_score > best_score`; equal scores preserve scan order. Evidence: `0x006F954B..0x006F955F`, `0x006F96F8..0x006F970C`. | Rust ranks nearest-first with stable-id tie. | `src/sim/combat/combat_targeting.rs` plus a passive-acquire caller. | Two legal Grizzly move-by candidates at different distances: farther candidate with higher score is chosen; equal-score candidates keep scan order. | `grizzly_passive_scan_scores_before_distance_and_preserves_scan_order_ties` | Stable-id sorting or nearest-first will visibly pick the wrong target in mixed-threat move-bys. |
| Direct score term is `SpecialThreatValue`, not `ThreatPosed`. Evidence: `0x00715726..0x00715748` reads `SpecialThreatValue` into `+0x2C0`; `0x0070CEDC..0x0070CEE6` consumes `+0x2C0`. | Rust appears not to parse these threat-score fields yet; using `ThreatPosed` would encode the wrong term. | `src/rules/object_type.rs`, `src/rules/ruleset.rs`, `src/sim/combat/combat_targeting.rs`. | Candidate A has `SpecialThreatValue=1` and lower/zero `ThreatPosed`; candidate B has higher `ThreatPosed` but zero `SpecialThreatValue`; passive scan chooses A when other terms are controlled. | `grizzly_passive_scan_uses_special_threat_value_not_threatposed` | Old docs and intuition call this "threat"; implementing `ThreatPosed` first would bake in a stale misconception. |
| Distance only penalizes beyond selected weapon range in `Calculate_Threat_Score`; in-range legality is still checked separately. Evidence: `0x006F80F4..0x006F8178` range gate; `0x0070CF6D..0x0070D0C0` range-minus-distance term. | Rust currently uses distance as primary rank over all in-range targets. | `src/sim/combat/combat_targeting.rs`. | Two in-range candidates with equal non-distance terms but different distances should favor nearer only by the score's distance coefficient, not as an absolute first key. | `grizzly_passive_scan_distance_is_score_penalty_not_primary_key` | Nearest-first hides higher-score target choices and makes modded `SpecialThreatValue`/coefficient cases wrong. |

## Negative Facts / Do Not Do

- Do not implement stock Grizzly passive target choice as nearest-first; `Greatest_Threat` compares score, not distance tuple.
- Do not use stable id as the tie-breaker for this path; strict `>` means equal scores keep scan order.
- Do not treat `ThreatPosed=` as the direct `Calculate_Threat_Score` term; the verified direct term is `SpecialThreatValue` at TechnoType `+0x2C0`.
- Do not scan the whole entity store and sort globally if aiming for exact parity; the ring scan can return early at quarter/half radius after a candidate exists.
- Do not merge this with attack-move/guard ranking blindly; this report is scoped to stock MTNK OpportunityFire passive move-by acquisition.

## Remaining Uncertainty

- The exact weapon-effectiveness coefficient algebra is verified structurally but not reduced to a single clean Rust formula for every armor/warhead edge case.
- The exact cell iteration order should be copied from the ring loop only if exact equal-score replay parity is required. For the minimal handoff, preserving insertion/scan order is enough to avoid stable-id replacement.
- Other AI consumers of `ThreatPosed=` remain out of scope; this report only rules it out as the direct `Calculate_Threat_Score` term for the investigated passive scan score.

## Stale-Doc Replacement Wording

Replace wording that says Grizzly passive selection is driven by `ThreatPosed=` with:

`OpportunityFire` passive acquisition uses `TechnoClass__Greatest_Threat`, which ranks legal candidates by the integer score returned from `TechnoClass__Evaluate_Candidate`. The score is based on `TechnoClass__Calculate_Threat_Score` terms such as selected weapon effectiveness, target `SpecialThreatValue`, target health/strength, selected weapon range, distance beyond range, enemy-house bonus, and cell threat-avoidance modifiers. `ThreatPosed=` is parsed for other AI threat systems but was not found as the direct score term in this Grizzly passive acquisition slice. Equal scores preserve scan order because the binary updates the best target only on strict greater-than.

## Sources

- Ghidra decompile/disassembly: `TechnoClass__Greatest_Threat @ 0x006F8DF0`, `TechnoClass__Evaluate_Candidate @ 0x006F7CA0`, `TechnoClass__Calculate_Threat_Score @ 0x0070CD10`, `TechnoClass__Scan_Cell_For_Target @ 0x006F8960`, `FUN_004D9920 @ 0x004D9920`.
- INI source: `ini/rulesmd.ini` `[General]` target coefficients, `[MTNK] ThreatPosed=15`, `[HTNK] ThreatPosed=40`, `[105mm] Range=5`.
- Rust reconnaissance: `src/sim/combat/combat_targeting.rs`, `src/sim/world/world_orders.rs`.
