# Bridge Collapse Slot 3 CombatDamage Isolation Trace

**Scenario:** Rules INI contains `[CombatDamage] DestroyableBridges=no`; effective `SpecialFlags::DestroyableBridges` bit `0x8000` remains enabled by default/session. Trace only whether that rules key can clear the weapon bridge-collapse gate.

**Verdict:** COMPLETE for this narrow gate-isolation scenario. Rust now matches the verified gamemd ownership for this key: `[CombatDamage] DestroyableBridges` does not clear the bridge-collapse gameplay gate.

## Pipeline

`Rules INI parse -> scenario/session SpecialFlags resolution -> bridge runtime flag -> combat Wall=yes bridge event -> bridge orchestrator outer gate -> bridge damage dispatch`

## Stage Results

| Stage | gamemd output | Rust output | Verdict |
|---|---:|---:|---|
| `[CombatDamage] DestroyableBridges=no` read by rules parser | no read, no write to bridge gate; gate delta `0` | no read by `BridgeRules::from_ini`; `destroyable_by_default = true`; gate delta `0` | PASS |
| default/session active `DestroyableBridges` bit | reset/session bit `0x8000` remains set, effective bit value `1` | default/session mode resolves to `true`, effective gate value `1` | PASS |
| bridge runtime gate value passed to collapse dispatcher | active scenario bit `0x8000` consumed by `Apply_area_damage`; bit value `1` | `BridgeRuntimeState.bridge_destroyable_flag = true`; `is_destroyable() = true` | PASS |
| weapon bridge damage with `Wall=yes` | `Apply_area_damage` can enter bridge tile damage when bit `0x8000 == 1` and `Wall == 1` | combat emits `BridgeDamageEvent` for `warhead.wall == true`; orchestrator does not bail when `is_destroyable() == true` | PASS |
| end-to-end visual collapse after RNG/state-machine/debris/audio | outside this slot; not recomputed here | outside this slot; not recomputed here | UNCHECKED |

## Evidence

### gamemd.exe

Verified active in standard YR, not dormant TS legacy:

- `SPECIALFLAGS_DESTROYABLEBRIDGES_DEFAULT_AND_MODES_GHIDRA_REPORT.md` verifies reset/default sets bit `0x8000` on through the reset writer at `0x006B8AE0`; scenario constructor and staging reset call this path.
- The same report verifies map `[SpecialFlags] DestroyableBridges` is the bit owner for campaign/editor and session staging owns normal skirmish/multiplayer.
- `WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md` verifies the live standard YR `Apply_area_damage` consumer at `0x00489280`: bridge tile damage requires active scenario `SpecialFlags & 0x8000` and warhead `Wall=yes`.
- That report also verifies `RulesClass::ReadCombatDamage @ 0x0066BBC9` does not read `[CombatDamage] DestroyableBridges`, while stock retail INI still contains the line at `ini/rulesmd.ini:804`.
- Retail data checked in this trace: `ini/rulesmd.ini:804` has `[CombatDamage] DestroyableBridges=yes`, `ini/rulesmd.ini:816` has `BridgeStrength=1500`, and `ini/rulesmd.ini:3029` has `[MultiplayerDialogSettings] BridgeDestruction=yes`.

For the concrete scenario where a modded rules INI changes `[CombatDamage] DestroyableBridges=no`, gamemd's bridge-collapse gate is numerically unchanged: the rules parser performs zero write to SpecialFlags bit `0x8000`, so the effective gate remains `1` when default/session keeps it enabled.

### Rust

- `src/rules/ruleset.rs:715` defines `BridgeRules` and documents that `[CombatDamage] DestroyableBridges=` is not the gameplay gate.
- `src/rules/ruleset.rs:751` reads `[CombatDamage] BridgeStrength`, but `src/rules/ruleset.rs:757` sets `destroyable_by_default = true` without reading `[CombatDamage] DestroyableBridges`.
- `src/rules/ruleset.rs:2501` has the focused regression test `combatdamage_destroyablebridges_no_does_not_clear_default_bridge_flag` asserting that `[CombatDamage] DestroyableBridges=no` leaves `destroyable_by_default == true`.
- `src/map/basic.rs:11` defines `BridgeDestroyabilityMode` and `src/map/basic.rs:50` resolves the effective flag: campaign/editor uses map override or default `true`; skirmish/multiplayer uses session `bridge_destruction`.
- `src/app_init.rs:450` selects `SkirmishOrMultiplayer { bridge_destruction: session.options.bridges_destroyable }` when a skirmish launch session exists, else `CampaignOrEditor`.
- `src/app_init_helpers.rs:362` computes the effective bridge destroyable bool from map/session mode and passes it to `BridgeRuntimeState::from_resolved_terrain` at `src/app_init_helpers.rs:369`.
- `src/sim/bridge_state/mod.rs:740` stores the bool and `src/sim/bridge_state/mod.rs:794` exposes it as `is_destroyable()`.
- `src/sim/combat/mod.rs:1878` and `src/sim/combat/mod.rs:1914` emit bridge damage only for `warhead.wall && weapon.damage > 0`.
- `src/sim/world/mod.rs:1297` forwards combat bridge events to the bridge orchestrator.
- `src/sim/world/bridge_orchestrator.rs:63` reads `sim.bridge_state`; if `is_destroyable()` is false, it returns before bridge damage dispatch. In this scenario, the traced Rust value is `true`, so the gate remains open.

## Player-Visible Findings

No FAIL or NOT-IMPLEMENTED finding was found for this exact slot. A player using rules INI `[CombatDamage] DestroyableBridges=no` while the effective SpecialFlags/session bit remains enabled should still be able to collapse bridges with valid `Wall=yes` weapon bridge damage in Rust, matching gamemd.exe for this gate.

## Adjacent Findings

These are not traced in this slot and do not affect the above PASS verdict:

- Debris RNG and `BridgeVoxelMax` usage may still diverge from gamemd's `BlowUpBridge` presentation path.
- Collapse sound/report routing may still be incomplete.
- Fallout scoping for deck DropIn/debris over destroyed sets needs its own trace.
- Legacy no-session quickplay/main-menu loading is treated as campaign/editor for this flag; this slot only verifies the concrete default/session-enabled scenario.

## Tests

No cargo tests were run in this subagent slot because the slot was permitted to write exactly one file and Cargo would write build/test artifacts under `target/`. Existing focused tests observed in source cover this isolation behavior, but they were not executed during this slot.

## Verdict Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0
