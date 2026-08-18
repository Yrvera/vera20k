# Bridge Collapse Slot 1 - Weapon Damage Gate Trace

**Scenario:** A non-Ion `Wall=yes` weapon detonates on a bridge cell while the effective `SpecialFlags::DestroyableBridges` bit is enabled. Concrete stock weapon used for numeric thresholds: `[105mm] Damage=65`, `Warhead=AP`; `[AP] Wall=yes`; `[CombatDamage] BridgeStrength=1500`.

**Scope:** weapon bridge-damage gate only: AoE entry, `Wall=yes`, active destroyable flag, `BridgeStrength` RNG gate, and collapse dispatch/no-dispatch. C4/CABHUT, debris, sound, fallout, bridge render, and full high/low collapse state-machine internals are adjacent, not traced here.

**Ghidra mode:** read-only. Spot-checks used `Apply_area_damage @ 0x00489280` decompile and caller list only; no mutating Ghidra tools were used.

## Verdict

Overall slot verdict: **PARTIAL**. The gate predicate, stock threshold math, and Rust dispatch condition match the verified binary evidence. Exact PRNG sample equality was not dynamically computed against a running `gamemd.exe`, so PRNG stream identity remains **UNCHECKED** under the trace-swarm literal-equality rule.

Verdict tally: **PASS: 7 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0**

## Pipeline

```text
Weapon detonation
  -> Apply_area_damage / Rust combat bridge event emission
  -> effective DestroyableBridges bit gate
  -> warhead Wall=yes gate
  -> BridgeStrength RandomRanged(1, 1500) gate
  -> dispatch collapse only when roll < 65
```

## Stage Table

### 1. Live standard-YR weapon entry

**gamemd:** `Apply_area_damage @ 0x00489280` has live standard YR callers including `WarheadTypeClass__Detonate @ 0x004690B0`, `AnimClass__AI`, `LightningStorm__GroundStrike`, `PsychicDominator__MindControlArea`, `SuperClass__Launch`, and others. This confirms the checked path is active in standard YR and not dormant TS legacy.

**Rust:** weapon combat drains through `tick_combat`, producing bridge damage events in `src/sim/combat/mod.rs`.

**Output compared:** standard weapon detonation can reach the bridge-damage gate in both engines: `true == true`.

**Verdict:** PASS.

### 2. Stock concrete inputs

**INI values:**

- `ini/rulesmd.ini:23325..23331`: `[105mm] Damage=65`, `Warhead=AP`.
- `ini/rulesmd.ini:26794..26797`: `[AP] Wall=yes`.
- `ini/rulesmd.ini:816`: `BridgeStrength=1500`.

**gamemd:** `Rules+0x1740` is `[CombatDamage] BridgeStrength`, consumed by `Random__RandomRanged(1, Rules+0x1740)` in `Apply_area_damage`.

**Rust:** `BridgeRules::default().strength = 1500`; `BridgeRules::from_ini` reads `[CombatDamage] BridgeStrength` and stores it as `u16`.

**Output compared:** `damage = 65`, `wall = true`, `bridge_strength = 1500` in both engines for this scenario.

**Verdict:** PASS.

### 3. `Wall=yes` bridge tile gate

**gamemd:** `Apply_area_damage @ 0x00489280` skips bridge tile damage unless `WarheadType+0x144` is nonzero. `WarheadTypeClass__ReadINI_Body @ 0x0075D3A0` reads `Wall` into that byte.

**Rust:** `src/sim/combat/mod.rs:1878` and `src/sim/combat/mod.rs:1914` gate bridge event emission on `warhead.wall && weapon.damage > 0`; death AoE has the same shape at `src/sim/combat/mod.rs:996`.

**Output compared:** AP has `Wall=yes`, so the bridge-tile gate is open in both engines: `true == true`.

**Verdict:** PASS.

### 4. Effective `DestroyableBridges` gate

**gamemd:** `Apply_area_damage @ 0x00489280` checks `(*g_ScenarioClass_Instance & 0x8000) != 0` before bridge tile damage. `SPECIALFLAGS_DESTROYABLEBRIDGES_DEFAULT_AND_MODES_GHIDRA_REPORT.md` verifies bit `0x8000` defaults on and is the active `DestroyableBridges` bit.

**Rust:** `src/sim/world/bridge_orchestrator.rs:62..66` returns early unless `sim.bridge_state.as_ref().is_destroyable()` is true. `src/sim/bridge_state/mod.rs:792..795` exposes the active flag. `src/app_init_helpers.rs:362..364` resolves the flag from map/session mode before constructing `BridgeRuntimeState`.

**Output compared:** scenario says effective bit enabled, so the outer gate is open in both engines: `true == true`.

**Verdict:** PASS.

### 5. `[CombatDamage] DestroyableBridges` isolation

**gamemd:** verified reports show `[CombatDamage] DestroyableBridges` is stock INI text but not a `RulesClass::ReadCombatDamage` gameplay key. The active gate is SpecialFlags bit `0x8000`.

**Rust:** `src/rules/ruleset.rs:757` now sets `destroyable_by_default = true` and does not read `[CombatDamage] DestroyableBridges`; `src/rules/ruleset.rs:2501..2511` tests that `DestroyableBridges=no` under `[CombatDamage]` does not clear the default bridge flag.

**Output compared:** `[CombatDamage] DestroyableBridges=no` contributes no bridge-gate clear in both engines: `false/no-effect == false/no-effect`.

**Verdict:** PASS.

### 6. BridgeStrength roll bounds and comparison

**gamemd:** for non-Ion warheads, `Apply_area_damage @ 0x00489280` calls `Random__RandomRanged(1, Rules+0x1740)` and proceeds only when `roll < damage`. Equality fails.

**Rust:** `src/sim/world/bridge_orchestrator.rs:1235..1241` calls `rng.next_range_u32_inclusive(1, ctx.bridge_strength as u32)` and continues to the next path when `!(roll < damage)`.

**Concrete math:** with stock `105mm` damage `65` and `BridgeStrength=1500`:

- pass set: `roll = 1..64` (`64` values)
- fail set: `roll = 65..1500` (`1436` values)
- equality case: `roll = 65`, result is fail in both engines

**Output compared:** inclusive bounds `1..=1500` and strict threshold `roll < 65` match numerically.

**Verdict:** PASS.

### 7. Collapse dispatch only after threshold passes

**gamemd:** the same `roll < damage` condition encloses calls to `ApplyDamageToCell`, `DestroyBridge_Low`, or `DestroyBridge_High`; when the comparison fails, those calls are skipped for that candidate path.

**Rust:** `src/sim/world/bridge_orchestrator.rs:1237..1241` consumes the roll, and a failed comparison uses `continue` before any path driver runs; a passed comparison enters the selected state-machine/direct path at `src/sim/world/bridge_orchestrator.rs:1252..1265` and following match arms.

**Output compared:** for non-Ion `105mm`, a bridge collapse dispatch is reachable only for `roll <= 64` in both engines; `roll >= 65` does not dispatch that candidate path.

**Verdict:** PASS.

### 8. Per-event RNG draw count in Rust

**gamemd:** verified binary evidence shows one `Random__RandomRanged(1, BridgeStrength)` call per matching non-Ion bridge candidate path before dispatch.

**Rust:** `src/sim/world/world_tests.rs:1452..1506` pins a high-direct fixture where exactly one `next_range_u32_inclusive(1, bridge_strength)` draw occurs before collapse dispatch, with debris lists empty to avoid downstream RNG.

**Output compared:** Rust has a focused test for one draw in that fixture, but this trace did not run tests because the slot was constrained to write exactly one file and avoid unrelated build artifacts.

**Verdict:** UNCHECKED.

### 9. Exact PRNG sample identity against `gamemd.exe`

**gamemd:** `Random__RandomRanged(1, 1500)` is the verified helper call at the bridge gate.

**Rust:** `src/sim/rng.rs:128..153` implements sorted inclusive ranged draws with rejection sampling.

**Output compared:** no live gamemd seed/state was captured, and no bridge-gate sample value was compared against the same Rust seed/state in this trace.

**Verdict:** UNCHECKED.

## Player-Visible Findings

No FAIL or NOT-IMPLEMENTED findings in the slot-1 gate itself.

The remaining player-visible risk is desync/probability drift if the exact PRNG stream ever diverges despite matching call bounds and comparison. That is UNCHECKED here, not a demonstrated mismatch.

## Adjacent Findings

- Debris RNG, metallic debris gating, collapse report sound, and fallout scoping are outside this slot. They remain relevant to full bridge-collapse parity but are not part of the weapon `BridgeStrength` gate.
- C4/CABHUT bridge collapse intentionally does not route through this weapon gate.

## Sources

- `docs/research/WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md`
- `docs/research/SPECIALFLAGS_DESTROYABLEBRIDGES_DEFAULT_AND_MODES_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `src/sim/combat/mod.rs`
- `src/sim/world/bridge_orchestrator.rs`
- `src/sim/bridge_state/mod.rs`
- `src/rules/ruleset.rs`
- Read-only Ghidra spot-check: `Apply_area_damage @ 0x00489280`, callers for `Apply_area_damage`
