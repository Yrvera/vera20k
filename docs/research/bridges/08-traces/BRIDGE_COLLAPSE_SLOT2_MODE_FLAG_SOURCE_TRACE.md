# Bridge Collapse Slot 2 Mode Flag Source Trace

Scenario: a map contains `[SpecialFlags] DestroyableBridges=no`. Compare campaign/editor behavior against normal skirmish/multiplayer with session `[MultiplayerDialogSettings] BridgeDestruction=yes` and `BridgeDestruction=no`; verify which source controls effective SpecialFlags bit `0x8000` and whether bridge weapon collapse follows that bit.

Scope: only the mode-specific `DestroyableBridges` flag source and the weapon bridge-damage outer gate. This trace does not verify BridgeStrength RNG, high/low bridge state machines, debris, audio, C4, or CABHUT collapse.

## Pipeline

`map INI [SpecialFlags]` -> `mode/session owner selection` -> `BridgeRuntimeState bridge_destroyable_flag` -> `BridgeDamageEvent` batch -> `apply_bridge_damage_events` outer gate -> downstream bridge damage paths only if bit is set.

## Concrete Expected Values

Input map line: `DestroyableBridges=no`, parsed boolean `false`.

| Case | gamemd active bit 0x8000 | Rust effective flag | Weapon bridge-damage gate |
|---|---:|---:|---|
| Campaign/editor | 0 | false | skip bridge tile damage |
| Skirmish/session `BridgeDestruction=yes` | 1 | true | continue to bridge damage dispatch |
| Skirmish/session `BridgeDestruction=no` | 0 | false | skip bridge tile damage |

## Stage Trace

| Stage | Our output | gamemd output | Verdict | Evidence |
|---|---:|---:|---|---|
| Parse map `[SpecialFlags] DestroyableBridges=no` | `SpecialFlagsSection.destroyable_bridges = Some(false)` | Reader sees key value `false` when the mode permits this key | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/map/basic.rs:77`; gamemd report `SPECIALFLAGS_DESTROYABLEBRIDGES_DEFAULT_AND_MODES_GHIDRA_REPORT.md:55` |
| Campaign/editor owner selection | effective flag `false` / bit 0 | active SpecialFlags bit `0x8000` cleared | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/map/basic.rs:50`; gamemd report lines 65-71 |
| Skirmish with `BridgeDestruction=yes` | effective flag `true` / bit 1 | map key ignored; staging/default bit remains set and is copied active | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/map/basic.rs:52`; `C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs:451`; gamemd report lines 73-75 |
| Skirmish with `BridgeDestruction=no` | effective flag `false` / bit 0 | map key ignored; session byte clears staging bit `0x8000`; active copy has bit clear | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/map/basic.rs:52`; gamemd report lines 73-75 |
| Apply chosen source during bridge-state construction | `BridgeRuntimeState::from_resolved_terrain(..., destroyable, ...)` receives the effective flag | active `ScenarioClass.SpecialFlags & 0x8000` is the runtime consumer bit | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/app_init_helpers.rs:362`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs:519`; gamemd report lines 99-101 |
| Weapon bridge-damage outer gate when flag is false | returns early; no bridge dispatch/collapse from this weapon gate | `Apply_area_damage` skips bridge tile damage when `(SpecialFlags & 0x8000) == 0` | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:62`; weapon report lines 47-53 |
| Weapon bridge-damage outer gate when flag is true | continues to BridgeStrength/path dispatch | `Apply_area_damage` continues to `Wall=yes` and bridge damage paths | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:62`; weapon report lines 22 and 47-53 |
| Local skirmish session option source | `SkirmishLaunchOptions.bridges_destroyable` defaults true and flows into init mode | `BridgeDestruction=yes` is the retail lobby/default source for staging ownership | PASS | Rust `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_launch.rs:118`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/game_options.rs:56`; gamemd report lines 79-81 |
| Network multiplayer session option source | no separate multiplayer launch/session bridge-destruction handoff found in this trace | gamemd multiplayer uses session staging `DAT_00A8E960` and `DAT_00A8B260` | NOT-IMPLEMENTED | Rust search found only `SkirmishLaunchSession`; gamemd report lines 32-34 and 73-75 |
| Actual full collapse outcome after the allowed gate | not computed in this slot | not computed in this slot | UNCHECKED | Downstream RNG/state/render/audio are assigned to other bridge-collapse slots |

## Findings

No FAIL findings for the implemented local campaign/editor and local skirmish paths. The concrete effective-bit decisions match gamemd for this scenario:

- Campaign/editor with map `DestroyableBridges=no`: bit `0x8000` is cleared and weapon bridge tile damage is skipped.
- Local skirmish with session `BridgeDestruction=yes`: map `DestroyableBridges=no` is ignored, bit `0x8000` is set, and weapon bridge damage is allowed to enter downstream dispatch.
- Local skirmish with session `BridgeDestruction=no`: map `DestroyableBridges=no` is ignored, session disables bit `0x8000`, and weapon bridge tile damage is skipped.

Player-visible NOT-IMPLEMENTED item: true network multiplayer launch/session ownership is not represented by a distinct multiplayer session path in the scanned Rust code. Current evidence covers the local skirmish session path, not an end-to-end network multiplayer lobby path.

## Adjacent Findings

- `[CombatDamage] DestroyableBridges` is intentionally not the source for this gate. That is covered by slot 3 and by `WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md`.
- C4/CABHUT bridge collapse does not use this SpecialFlags/weapon AoE gate and is outside this slot.
- BridgeStrength RNG, collapse state-machine results, debris, sound, and render timing are outside this slot and remain unchecked here.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/SPECIALFLAGS_DESTROYABLEBRIDGES_DEFAULT_AND_MODES_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/map/basic.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_init_helpers.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_launch.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/game_options.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs`

## Verdict Tally

PASS: 8 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

Status: COMPLETE
