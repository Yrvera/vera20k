# Bridge RNG Call Order Classification - Ghidra Research Report

**Address(es):** `0x00489280` (`Apply_area_damage`), `0x0047DD70` (`CellClass::BlowUpBridge`), `0x0065C7E0` (`Random__RandomRanged`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** bridge damage `BridgeStrength` RNG gate and per-cell bridge debris RNG order/ranges in current Rust `src/sim/world/bridge_orchestrator.rs`.  
**Non-Scope:** bridge geometry/pathing, bridge repair RNG, map-init destruction, renderer/audio effects, and non-bridge RNG users.  
**Confidence:** High for damage gate and debris call order; Medium for modded empty-list edge behavior because retail INI keeps both lists non-empty.  
**Active in YR:** Yes for damage gate and `BlowUpBridge`; `BridgeVoxelMax=` parsing is No for standard YR debris behavior (dormant/TS legacy per prior exhaustive report).

## 0. Working Notes

Target question: classify current Rust bridge damage/debris RNG use against verified gamemd/YR call order and ranges.

Non-goals: do not investigate unrelated bridge geometry/pathing, do not patch Rust, do not re-audit the core RNG helper beyond settled contract.

Evidence needed to mark COMPLETE: decompile plus assembly context for each bridge RNG call site, current Rust scan for matching functions/tests, and INI/default evidence for active standard YR keys.

Stop conditions: stop after bridge damage gate and `spawn_bridge_debris` are classified GREEN/YELLOW/RED with implementation handoff; defer modded-list crash semantics or non-bridge visual/audio details.

## 1. Overview

The damage gate is implementation-ready and currently GREEN: gamemd calls `RandomRanged(1, Rules.BridgeStrength)`, compares the roll with effective damage using strict `<`, and bypasses the roll for `IonCannonWarhead`; Rust uses `next_range_u32_inclusive(1, bridge_strength)` and strict `<` in the same path-order model.

The debris path is RED after the RNG rewrite. Gamemd's `BlowUpBridge` does not use small `RandomRanged(0,19)` or `RandomRanged(0,1)` gates. It uses normalized probability rolls `RandomRanged(0, 0x7FFFFFFE)` for the 95% outer gate, two position jitter draws, and the 50% metallic gate. Rust currently uses `next_range_u32(20)`, `next_range_u32(0xFFFF)` twice, and `next_range_u32(2)`, which changes rejection probability, call-count edge cases, and stream advancement.

## 2. Key Offsets / Inputs

| Item | Binary location | Meaning | Active in YR |
|---|---:|---|---|
| `Scenario + 0x218` | `0x00489FEF`, `0x0047DE4E`, `0x0047DF8B` | Scenario-owned `Random` object passed to `Random__RandomRanged` | Yes |
| `Rules + 0x1740` | `0x00489FE0`, `0x0048A165`, `0x0048A231`, `0x0048A28B` | `BridgeStrength` upper bound | Yes |
| `Rules + 0xFF0` | `0x00489FD8`, `0x0048A15D`, `0x0048A229`, `0x0048A283` | `IonCannonWarhead` pointer; bypasses damage RNG | Yes |
| `Rules + 0x140/+0x14C` | `0x0047DF76..0x0047DF91` | `MetallicDebris` data pointer / active count | Yes |
| `Rules + 0x15C/+0x168` | `0x0047DFF4..0x0047E004` | `BridgeExplosions` data pointer / active count | Yes |
| `BridgeVoxelMax=` | prior report places scalar at `Rules+0x624` | Parsed but no live `BlowUpBridge` reader in standard YR | No |

INI defaults checked in repo: `rulesmd.ini` has `BridgeVoxelMax=3` line 419, `MetallicDebris=...` line 528, `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070` line 529, `DestroyableBridges=yes` line 804, `BridgeStrength=1500` line 816, and `IonCannonWarhead=IonCannonWH` line 874.

## 3. Core Logic

### 3.1 Damage Gate - GREEN

Verified behavior:

1. `Apply_area_damage @ 0x00489280` checks the bridge-damage master gates before the bridge tile damage block: scenario special flag `0x8000` and `WarheadType + 0x144` (`Wall=`). Active in YR: Yes, `DestroyableBridges=yes` and standard wall-capable warheads reach this code.
2. Each non-Ion bridge path loads `Rules+0x1740`, pushes high bound then low bound `1`, loads `Scenario+0x218`, and calls `0x0065C7E0`.
3. Assembly compares the returned roll against stack damage and uses `JGE` to skip, so equality fails. The condition is `roll < damage`.
4. `IonCannonWarhead` pointer equality to `Rules+0xFF0` jumps around the random call and proceeds directly to the damage helper. State-machine paths then get retries only for Ion.

Assembly evidence:

- High state-machine: `0x00489FD8` compares warhead to `Rules+0xFF0`; `0x00489FE0` reads `Rules+0x1740`; `0x00489FEC PUSH EAX`, `0x00489FED PUSH 0x1`, `0x00489FEF LEA ECX,[Scenario+0x218]`, `0x00489FF5 CALL 0x0065C7E0`; `0x00489FFA CMP EAX,[ESP+0x24]`; `0x00489FFE JGE` skips.
- Low state-machine: same pattern at `0x0048A15D..0x0048A182`.
- Low direct: same pattern at `0x0048A229..0x0048A24E`.
- High direct: same pattern at `0x0048A283..0x0048A2A8`.

Current Rust status:

- `src/sim/world/bridge_orchestrator.rs:1100..1118` evaluates paths in `HighStateMachine`, `LowStateMachine`, `LowDirect`, `HighDirect` order.
- `src/sim/world/bridge_orchestrator.rs:1111..1117` bypasses the roll for `is_ion_cannon`, otherwise calls `next_range_u32_inclusive(1, bridge_strength)` and requires `(roll as u16) < damage`.
- `src/sim/world/world_tests.rs:1424..1478` pins one non-Ion gate draw for a fixture where only HighDirect matches and debris lists are empty.

Classification: GREEN for range, strictness, Ion bypass, and path-order shape.

### 3.2 Debris RNG - RED

Verified binary `CellClass::BlowUpBridge @ 0x0047DD70` order:

1. If `g_IsMapEditor != 0`, return before all gameplay debris work. Active in YR: Conditional; normal gameplay has map editor flag clear.
2. Kill ground-list occupants and destroy/drop deck-list occupants before debris RNG. Active in YR: Yes.
3. Queue the cell in a collapsed-bridge buffer before debris RNG. Active in YR: Yes, though downstream consumer is outside this slot.
4. Gate the entire debris block on `Rules+0x168 > 0`, which is `BridgeExplosions.ActiveCount`, not `BridgeVoxelMax`. Active in YR: Yes; retail count is 4.
5. Outer probability draw: `RandomRanged(0, 0x7FFFFFFE)` from `Scenario+0x218`; compare `roll * scale < 0.95`; fail skips the whole debris block. Active in YR: Yes.
6. Build position at cell center and bridge-height Z, then consume two more `RandomRanged(0, 0x7FFFFFFE)` draws for in-cell jitter. Active in YR: Yes.
7. Metallic gate: another `RandomRanged(0, 0x7FFFFFFE)` and compare to 0.5. Active in YR: Yes.
8. If metallic gate passes and allocation succeeds, pick `MetallicDebris` with `RandomRanged(0, MetallicDebris.ActiveCount - 1)` and spawn an `AnimClass` with delay `0`, loop flag `1`, z offset `0x600`. Active in YR: Yes with retail count 20.
9. Allocate BridgeExplosion unconditionally after the metallic branch; if allocation succeeds, draw `RandomRanged(1,5)` for start delay, then `RandomRanged(0, BridgeExplosions.ActiveCount - 1)` for the anim type. Active in YR: Yes with retail count 4.

Assembly/decompile evidence:

- `0x0047DE33` reads `Rules+0x168`; `0x0047DE3B JLE` skips the debris block if the count is not positive.
- `0x0047DE47 PUSH 0x7FFFFFFE`, `0x0047DE4C PUSH 0`, `0x0047DE54 CALL 0x0065C7E0`, then float compare against `_DAT_007e4f58` (0.95 threshold).
- `0x0047DE97..0x0047DEC6` consumes the first jitter draw using the same `0, 0x7FFFFFFE` range; `0x0047DEF7..0x0047DF43` consumes the second and then the metallic 50% gate.
- `0x0047DF7C` reads `Rules+0x14C`, `0x0047DF82 DEC EAX`, `0x0047DF89 PUSH 0`, `0x0047DF91 CALL 0x0065C7E0` for `MetallicDebris` index.
- `0x0047DFD7 PUSH 5`, `0x0047DFD9 PUSH 1`, `0x0047DFE1 CALL 0x0065C7E0` for explosion delay.
- `0x0047DFF4` reads `Rules+0x168`, `0x0047E000 DEC EAX`, `0x0047E002 PUSH 0`, `0x0047E004 CALL 0x0065C7E0` for `BridgeExplosions` index.

Current Rust mismatch:

- `src/sim/world/bridge_orchestrator.rs:932..934` returns only when both explosion and metallic lists are empty. Binary returns before RNG when `BridgeExplosions.ActiveCount <= 0`, regardless of `MetallicDebris`.
- `src/sim/world/bridge_orchestrator.rs:938` uses `next_range_u32(20) == 0` for the outer 95% gate. Binary uses `RandomRanged(0, 0x7FFFFFFE)` plus a floating threshold. After the parity RNG rewrite, this is not just a distribution issue: `next_range_u32(20)` can reject masked values `20..31` and consume extra raw draws, unlike the binary gate which rejects only `0x7FFFFFFF`.
- `src/sim/world/bridge_orchestrator.rs:944..945` uses `next_range_u32(0xFFFF)` for jitter draws. Binary uses normalized `RandomRanged(0, 0x7FFFFFFE)` for both.
- `src/sim/world/bridge_orchestrator.rs:955` uses `next_range_u32(2) == 0` for metallic 50%. Binary uses normalized `RandomRanged(0, 0x7FFFFFFE)` thresholded at 0.5.
- `src/sim/world/bridge_orchestrator.rs:959` gates metallic slot/spawn on `rules.bridge_rules.voxel_max > 0`. Binary `BlowUpBridge` does not read `BridgeVoxelMax`; prior exhaustive bridge report marks it dormant/TS-only in YR.
- `src/sim/world/bridge_orchestrator.rs:1374..1412` predicts the same small-range Rust calls on a parallel Rust RNG. This test proves internal self-consistency, not binary parity.

Classification: RED for debris call ranges and gates. The current code can visibly produce different debris/explosion choices and can silently desync the shared RNG stream after bridge collapse.

## 4. Current Rust Implementation Status

| Surface | Status | Evidence | Classification |
|---|---|---|---|
| `run_dispatch_loop` damage RNG | Uses direct inclusive `1..=BridgeStrength`, strict `<`, Ion bypass, fixed path order | `src/sim/world/bridge_orchestrator.rs:1100..1118`; binary `0x00489FD8..0x0048A2A8` | GREEN |
| `spawn_bridge_debris` outer gate | Uses `next_range_u32(20) == 0` and list-empty shortcut | `src/sim/world/bridge_orchestrator.rs:932..940`; binary `0x0047DE33..0x0047DE72` | RED |
| `spawn_bridge_debris` jitter | Consumes two draws but from wrong range | `src/sim/world/bridge_orchestrator.rs:942..945`; binary `0x0047DE97..0x0047DF43` | RED |
| `spawn_bridge_debris` metallic gate | Uses `next_range_u32(2)` and `BridgeVoxelMax` | `src/sim/world/bridge_orchestrator.rs:954..960`; binary `0x0047DF43..0x0047DF91` | RED |
| `spawn_bridge_debris` explosion delay/slot | Uses `RandomRanged(1,5)` and `RandomRanged(0,count-1)` shape through wrappers when `explosion_count > 0` | `src/sim/world/bridge_orchestrator.rs:979..983`; binary `0x0047DFD7..0x0047E004` | GREEN for these two draws, after earlier gate/order is fixed |

## 5. Integration Points

Damage RNG is reached from `Apply_area_damage` bridge tile damage, after the object splash collection/application path, and before `ApplyDamageToCell`, `DestroyBridge_Low`, or `DestroyBridge_High`. Active in YR: Yes, gated by `DestroyableBridges=yes`, `Wall=yes`, and bridge identity.

Debris RNG is reached from `CellClass::BlowUpBridge` after occupant handling and collapsed-cell queueing. Current Rust calls `spawn_bridge_debris` from both normal damage collapse and hut-collapse cascade (`src/sim/world/bridge_orchestrator.rs:118` and `:327`), which is the right integration shape for per-cell collapse effects, but the RNG contract inside the helper is wrong.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Random__RandomRanged @ 0x0065C7E0` contract | verified from prior report and spot decompile | `0x0065C7E0..0x0065C88A`; `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md` | none for this slot |
| BridgeStrength high SM gate | verified | `0x00489FD8..0x0048A004` | none |
| BridgeStrength low SM gate | verified | `0x0048A15D..0x0048A188` | none |
| BridgeStrength low direct gate | verified | `0x0048A229..0x0048A250` | none |
| BridgeStrength high direct gate | verified | `0x0048A283..0x0048A2AA` | none |
| `BlowUpBridge` outer debris gate | verified | `0x0047DE33..0x0047DE72` | none |
| `BlowUpBridge` jitter/probability order | verified | `0x0047DE54`, `0x0047DEC6`, `0x0047DF43` | exact pixel-offset formula not required because Rust currently discards jitter |
| `BlowUpBridge` metallic slot pick | verified | `0x0047DF7C..0x0047DF91` | modded empty `MetallicDebris` edge deferred |
| `BlowUpBridge` explosion delay/slot pick | verified | `0x0047DFD7..0x0047E004` | none |
| `BridgeVoxelMax=` live use in debris | verified absent for YR debris by prior exhaustive report; spot decompile found no `BlowUpBridge` read | `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md:1332..1337`; `0x0047DD70` decompile | no further work in this slot |
| Rust bridge orchestrator damage/debris scan | verified | `src/sim/world/bridge_orchestrator.rs:925..1001`, `:1068..1118`, tests `:1369..1464` | patch needed, not done here |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does non-Ion bridge damage use inclusive or exclusive upper bound? -> Inclusive `RandomRanged(1, BridgeStrength)`.` (evidence: `0x00489FEC..0x00489FF5`, `0x0065C7E0` contract)
- `[RESOLVED] OQ-2 - Does equality pass damage gate? -> No; `JGE` skips, so only `roll < damage` passes.` (evidence: `0x00489FFA..0x00489FFE`)
- `[RESOLVED] OQ-3 - Does IonCannon consume a BridgeStrength roll? -> No; warhead pointer equality to `Rules+0xFF0` jumps around the random call.` (evidence: `0x00489FD8..0x00489FDE`)
- `[RESOLVED] OQ-4 - What is the path order for bridge RNG gates? -> High SM, low SM, low direct, high direct in the decompiled `Apply_area_damage` order; Rust matches this order.` (evidence: `0x00489F27..0x0048A2C4`; Rust `bridge_orchestrator.rs:1100..1106`)
- `[RESOLVED] OQ-5 - What gates `BlowUpBridge` debris RNG? -> `Rules+0x168 > 0` (`BridgeExplosions.ActiveCount`) plus 95% normalized random gate.` (evidence: `0x0047DE33..0x0047DE72`; `BRIDGEEXPLOSIONS_RULES_OFFSETS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-6 - Are the 95% and 50% debris gates small integer ranges? -> No; both use `RandomRanged(0, 0x7FFFFFFE)` then float threshold comparison.` (evidence: `0x0047DE47..0x0047DE72`, `0x0047DF32..0x0047DF61`)
- `[RESOLVED] OQ-7 - Are jitter draws consumed even if Rust ignores offsets? -> Yes; two normalized draws occur before the metallic gate.` (evidence: `0x0047DE97..0x0047DF43`)
- `[RESOLVED] OQ-8 - Does `BridgeVoxelMax` gate metallic debris in YR? -> No for standard YR `BlowUpBridge`; it is parsed but dormant/TS-only, while live code reads `BridgeExplosions.ActiveCount` and `MetallicDebris.ActiveCount`.` (evidence: `0x0047DD70`; `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md:1332..1337`)
- `[RESOLVED] OQ-9 - Does current Rust debris test prove binary parity? -> No; it predicts the same Rust helper sequence used by the implementation, including wrong small ranges.` (evidence: `src/sim/world/bridge_orchestrator.rs:1387..1405`)
- `[DEFERRED] OQ-10 - What exactly happens in gamemd if a mod empties `MetallicDebris` but leaves `BridgeExplosions` non-empty?` (category: `needs-runtime-debugger`; reason: binary appears to call `RandomRanged(0, count-1)` only after retail count assumptions; runtime mod behavior/crash safety was outside scope; next-step-if-pursued: test a modded INI under debugger.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Non-Ion bridge damage rolls `RandomRanged(1, BridgeStrength)` and passes only when `roll < damage`; IonCannon bypasses the roll. | `0x00489FD8..0x00489FFE`, `0x0048A15D..0x0048A182`, `0x0048A229..0x0048A24E`, `0x0048A283..0x0048A2A8` | none observed | `src/sim/world/bridge_orchestrator.rs::run_dispatch_loop` | Keep inclusive helper and strict less-than; keep path-order call-count tests. | Non-Ion high-direct hit consumes one gate draw; Ion hit consumes zero gate draws before state-machine retries. Proposed test: `bridge_damage_gate_uses_inclusive_strength_and_strict_less_than`. | Do not change to `<=` or exclusive `0..BridgeStrength`; equality must fail. |
| `BlowUpBridge` outer/jitter/metallic probability draws all use `RandomRanged(0, 0x7FFFFFFE)` from shared scenario RNG, not small integer ranges. | `0x0047DE47..0x0047DE72`, `0x0047DE97..0x0047DF43` | mismatch: Rust uses `next_range_u32(20)`, `next_range_u32(0xFFFF)` twice, and `next_range_u32(2)` | `src/sim/world/bridge_orchestrator.rs::spawn_bridge_debris` | Add/use a normalized probability draw helper or direct inclusive calls preserving `0x7FFFFFFE` range; consume exactly the binary sequence. | A seed fixture predicts post-debris RNG state using `R(0,0x7FFFFFFE)` gate, jitter, jitter, metallic gate, optional slot, delay, explosion slot. Proposed test: `bridge_debris_uses_normalized_probability_draw_order`. | Do not approximate 95% with `R(0,19)` or 50% with `R(0,1)`; after rejection sampling these can consume a different number of raw draws. |
| `BridgeVoxelMax` does not gate YR `BlowUpBridge`; debris block is gated by `BridgeExplosions.ActiveCount > 0`, and metallic slot uses `MetallicDebris.ActiveCount`. | `0x0047DE33..0x0047DE3B`, `0x0047DF7C..0x0047DF91`, `0x0047DFF4..0x0047E004`; `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md:1332..1337` | mismatch: Rust gates metallic on `rules.bridge_rules.voxel_max > 0`; comments/tests assert `BridgeVoxelMax` behavior | `src/sim/world/bridge_orchestrator.rs::spawn_bridge_debris`; `src/rules/ruleset.rs::BridgeRules::voxel_max` bridge consumer comments/tests | Remove `BridgeVoxelMax` from YR debris gating; gate whole helper on non-empty `bridge_explosions`; only use list counts for slot picks. | Setting `BridgeVoxelMax=0` with stock `BridgeExplosions` still permits explosion and metallic RNG/spawns; empty `BridgeExplosions` consumes no debris RNG. Proposed test: `bridge_debris_ignores_dormant_bridge_voxel_max`. | Do not use stale `Rules+0x168 = BridgeVoxelMax`; `+0x168` is `BridgeExplosions.ActiveCount`. |

### Negative Facts / Do Not Do

- Do not treat `BridgeStrength` as a damage amount for `BlowUpBridge` occupant kills. Evidence: `CellClass::BlowUpBridge @ 0x0047DD70` passes object health pointer and `Rules+0xFA8` (`C4Warhead`), while `BridgeStrength` only appears in `Apply_area_damage` gate. Active in YR: Yes.
- Do not replace `RandomRanged(0, 0x7FFFFFFE)` probability gates with `RandomRanged(0,19)` or `RandomRanged(0,1)`. Evidence: `0x0047DE47..0x0047DE72` and `0x0047DF32..0x0047DF61`. Active in YR: Yes.
- Do not make `BridgeVoxelMax=0` suppress YR 2D `BridgeExplosions` or `MetallicDebris` in `BlowUpBridge`. Evidence: live decompile reads `Rules+0x168` as `BridgeExplosions.ActiveCount`; prior report marks `BridgeVoxelMax` dormant/TS-only. Active in YR: No for `BridgeVoxelMax`.
- Do not rely on Rust's current debris draw-order test as binary proof. Evidence: `src/sim/world/bridge_orchestrator.rs:1387..1405` mirrors Rust's own wrong small-range calls. Active in YR: N/A, test-only fact.
- Do not create a separate bridge RNG stream. Evidence: bridge damage and debris load `0x00A8B230` and pass `Scenario+0x218` to `Random__RandomRanged`. Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `docs/research/BRIDGE_SYSTEM.md`: replace `if Rules->BridgeVoxelMax > 0:` with `if Rules->BridgeExplosions.ActiveCount > 0:` and replace the offset row `| +0x168 | BridgeVoxelMax | 3 |` with `| +0x168 | BridgeExplosions.ActiveCount | 4 in stock YR |`.
- `docs/research/BRIDGE_SYSTEM_VERIFY_DOC_AMENDMENTS.md`: replace wording that says `+0x168` is `BridgeVoxelMax` with `+0x168 is BridgeExplosions.ActiveCount; BridgeVoxelMax is parsed separately and is dormant/TS-only for standard YR debris`.
- `docs/research/RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`: current stale Rust status lines still describe old xorshift/modulo; replace with `Rust SimRng has since been rewritten to the 250-word gamemd-style state, but bridge debris call sites still need range/order auditing; see BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md.`

## Sources

- Ghidra decompile/read-only assembly: `Apply_area_damage @ 0x00489280`, `CellClass::BlowUpBridge @ 0x0047DD70`, `Random__RandomRanged @ 0x0065C7E0`.
- Assembly contexts sampled: `0x00489FD8..0x00489FFE`, `0x0048A15D..0x0048A182`, `0x0048A229..0x0048A24E`, `0x0048A283..0x0048A2A8`, `0x0047DE33..0x0047E004`.
- Rust scan: `src/sim/world/bridge_orchestrator.rs`, `src/sim/world/world_tests.rs`, `src/rules/ruleset.rs`.
- INI scan: `ini/rulesmd.ini`, `ini/rules.ini`.
- Prior docs: `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`, `BRIDGEEXPLOSIONS_RULES_OFFSETS_GHIDRA_REPORT.md`, `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`, `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`, `PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md`.

## Status

COMPLETE.
