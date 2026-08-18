# Weapon AoE Bridge Damage Entry - Ghidra Research Report

**Address(es):** `0x00489280` (`Apply_area_damage`) primary; `0x00587180` (`ApplyDamageToCell`); `0x006B8CA0` / `0x006B8B30` SpecialFlags reader/writer; `0x0066BBC9` (`RulesClass::ReadCombatDamage`); `0x0075D3A0` (`WarheadTypeClass::ReadINI_Body`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard weapon/superweapon area-damage entry into bridge tile damage: `DestroyableBridges`/scenario flag gate, warhead `Wall=yes`, impact-Z layer tests, random damage comparison, low/high overlay dispatch, and Rust-facing combat deltas.
**Non-Scope:** C4/CABHUT collapse except contrast notes; bridge collapse state-machine internals after dispatch; bridge fallout ordering; full superweapon impact-Z construction beyond already cited reports.
**Confidence:** HIGH for the entry gates, INI/key sources, random comparison, impact-Z window, dispatch order, and Rust deltas scanned here.
**Active in YR:** Yes for standard weapon/superweapon AoE callers when the scenario SpecialFlags bit 0x8000 is enabled and the warhead has `Wall=yes`; stock YR skirmish default is enabled.

## 0. Working Notes Required By Swarm Prompt

Target question: How does standard weapon/superweapon `Apply_area_damage` enter bridge tile damage, and what Rust combat/rules deltas remain?

Non-goals: Do not re-investigate C4/CABHUT collapse, the high/low collapse state machines, TubeClass zones, or deck-unit fallout.

Evidence needed to mark COMPLETE: decompile plus caller/xref evidence for `Apply_area_damage`; decompile plus INI/default evidence for `DestroyableBridges`, `BridgeStrength`, `IonCannonWarhead`, and warhead `Wall`; Rust scan naming affected surfaces and proposed tests.

Stop conditions: stop once the outer gate, warhead gate, Z gate, RNG compare, high/low dispatch, and Rust-facing mismatches are all resolved or explicitly deferred.

## 1. Overview

Standard AoE bridge tile damage is a late block inside `Apply_area_damage`, after object splash target collection/application and after rocker/push handling. The block is live in YR, but it is not entered just because a weapon has splash damage: `ScenarioClass SpecialFlags & 0x8000` and `WarheadType+0x144 Wall` must both be true. When entered, bridge tile damage is attempted only for the impact cell, not every CellSpread cell, and each candidate path uses `IonCannonWarhead` bypass or `Random(1, BridgeStrength) < damage`.

This is separate from object layer selection. Object splash chooses one occupant list (`CellClass+0xE4` ground or `+0xE8` bridge/deck) from the impact cell and impact Z earlier in the same function. `Wall=yes` gates bridge tile/overlay damage; it does not select which objects receive splash damage.

## 2. Class Layout / Key Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `*g_ScenarioClass_Instance & 0x8000` | SpecialFlags bit 15, `DestroyableBridges` | reader `0x006B8CA0`; consumer `0x00489280`; writer `0x006B8B30` | Yes, default-on; campaign/map-editor map override conditional |
| `WarheadType+0x144` | `Wall=` bool | parser `0x0075D3A0` writes `param_1+0x144` after key `Wall`; consumer `0x00489280` tests `param_4+0x144` | Yes |
| `Rules+0x1740` | `[CombatDamage] BridgeStrength=` | parser `0x0066CD80`; consumer reads before `Random__RandomRanged(1, Rules+0x1740)` in `0x00489280` | Yes |
| `Rules+0xFF0` | `[CombatDamage] IonCannonWarhead=` | parser `0x0066CA9B`; consumer compares `param_4 == Rules+0xFF0` | Yes, as a bridge-damage special warhead identity |
| `CellClass+0x140 & 0x100` | structural bridge flag for tile/Z gate and object layer selector | `0x00489562..0x0048958D`, bridge tile block in `0x00489F00..0x0048A214` | Yes |
| `CellClass+0x11B` | signed terrain level byte used by Z window | `0x00489F77` / `0x0048A0A5` decompile reads `this->Level` | Yes |
| `DAT_0089E864` | bridge-height addend used in object selector and bridge tile Z window | `0x0048957A`; `0x00489F77`; `0x0048A0A5` | Yes |
| `DAT_0089E870` | terrain level-height multiplier in bridge tile Z window | `0x00489F77`; `0x0048A0A5` | Yes |
| `CellClass+0x44` / `OverlayTypeIndex` | low/high direct overlay identity | direct ranges `0x4A..0x63`, `0xCD..0xE6` at `0x0048A214..0x0048A2C4` | Yes |

## 3. Core Logic

### 3.1 Outer Gates

`Apply_area_damage` first exits entirely if damage/warhead arguments are absent or scenario no-damage bit 0x20 is set. Later, after object splash is processed, it re-acquires the impact cell and runs:

```text
if (Scenario.SpecialFlags & 0x8000) == 0:
    skip bridge tile damage
if warhead.Wall == false:
    skip bridge tile damage
```

Active in YR: Yes. Evidence: decompile `0x00489280`, bridge block; assembly range sampled read-only `0x00489F00..0x0048A2D0`; caller list includes `WarheadTypeClass__Detonate @ 0x004690B0`, `LightningStorm__GroundStrike @ 0x0053A300`, `PsychicDominator__MindControlArea @ 0x0053B080`, `SuperClass__Launch @ 0x006CC390`, `AnimClass__AI @ 0x00423AC0`, and other standard damage paths.

### 3.2 `DestroyableBridges` Source

`DestroyableBridges` is not read by `RulesClass::ReadCombatDamage`, despite stock rules placing `DestroyableBridges=yes` under `[CombatDamage]`. The binary reader is `FUN_006B8CA0`, which reads `[SpecialFlags] DestroyableBridges` through `CCINIClass__ReadBool` and writes bit `0xF` / value `0x8000` into the first dword passed as `param_1`.

The reader is called from `ScenarioClass::Read_INI_Basic @ 0x00689E90`, with the direct call at the start of the function. The SpecialFlags read of `DestroyableBridges` is itself conditional: it is inside `(g_GameMode == 0) || (g_IsMapEditor != 0)`, so normal skirmish/multiplayer keeps the runtime default. The writer `FUN_006B8B30` also saves the bit under `[SpecialFlags]`.

Active in YR: Yes, but map override is conditional. Standard YR skirmish uses the runtime default (on); campaign/map editor can read a map `[SpecialFlags]` override. Evidence: `0x006B8CA0`, `0x006B8B30`, caller `ScenarioClass__Read_INI_Basic @ 0x00689E90`, stock `ini/rulesmd.ini:804` placement under `[CombatDamage]` with no binary reader in `RulesClass::ReadCombatDamage @ 0x0066BBC9`.

### 3.3 Warhead `Wall=yes`

`WarheadTypeClass__ReadINI_Body @ 0x0075D3A0` reads `Wall` and writes the result to `WarheadType+0x144`. `Apply_area_damage` tests this byte at the bridge block gate and again around state-machine bridge paths. Warheads without `Wall=yes` can still perform ordinary object splash on the selected occupant layer, but they cannot damage bridge tiles.

Active in YR: Yes. Evidence: parser `0x0075D508` region in `WarheadTypeClass__ReadINI_Body`; consumer `0x00489280`; stock warheads include many `Wall=yes` entries, including Prism/Tesla comments in `ini/rulesmd.ini:27335` and `ini/rulesmd.ini:27359`.

### 3.4 Impact-Z Window

For structural bridge tile/state-machine paths, `Apply_area_damage` checks an impact-Z window before attempting damage:

```text
skip if impact_z > (cell.Level + 1) * LevelHeight + BridgeHeight
skip if impact_z <= (cell.Level - 2) * LevelHeight + BridgeHeight
```

So the accepted window is:

```text
(cell.Level - 2) * LevelHeight + BridgeHeight < impact_z
and impact_z <= (cell.Level + 1) * LevelHeight + BridgeHeight
```

The upper bound is inclusive because only `BridgeHeight + (Level+1)*LevelHeight < impact_z` skips. The lower bound is exclusive because `impact_z <= BridgeHeight + (Level-2)*LevelHeight` skips. Direct low/high overlay ranges skip this structural Z window.

Active in YR: Yes for structural bridge tile/state-machine candidates. Evidence: `0x00489280` decompile, structural bridge checks in high/low tile blocks; read-only disassembly range sampled `0x00489F00..0x0048A214`.

### 3.5 Random Damage Comparison

For each bridge candidate path, non-Ion warheads roll:

```text
roll = Random__RandomRanged(1, Rules.BridgeStrength)
damage path passes only if roll < damage
```

Equality fails. `IonCannonWarhead` bypasses the roll. On the `ApplyDamageToCell` state-machine paths, Ion also enables retry: first attempt plus up to three retries if the state-machine call returns false. Direct low/high overlay `DestroyBridge_*` paths are single-shot.

Active in YR: Yes. Evidence: `0x00489280` bridge blocks compare `param_4 == Rules+0xFF0` or call `Random__RandomRanged(1, Rules+0x1740)`, then compare `< damage`; `Rules+0x1740` parser at `0x0066CD80`; `Rules+0xFF0` parser at `0x0066CA9B`; stock defaults `ini/rulesmd.ini:816` and `ini/rulesmd.ini:874`.

### 3.6 Low/High Dispatch

The bridge damage block evaluates four broad shapes:

1. high/structural bridge tile identity to `ApplyDamageToCell`;
2. low/structural bridge tile identity to `ApplyDamageToCell`;
3. direct low bridge overlay range `0x4A..0x63` to `DestroyBridge_Low`;
4. direct high bridge overlay range `0xCD..0xE6` to `DestroyBridge_High`.

`ApplyDamageToCell @ 0x00587180` is the topology-aware dispatcher reached by the structural tile paths; prior high-bridge report verifies it can dispatch high/low state machines and direct overlay destroy functions. The direct overlay ranges in `Apply_area_damage` use strict outer comparisons: `0x49 < overlay < 100` and `0xCC < overlay < 0xE7`.

Active in YR: Yes. Evidence: `Apply_area_damage @ 0x00489280`, direct overlay region `0x0048A214..0x0048A2C4`; `ApplyDamageToCell @ 0x00587180`; `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`.

## 4. INI Keys

| Key | Section | Retail YR value | Binary read | Effect | Active in YR |
|---|---|---:|---|---|---|
| `DestroyableBridges` | map `[SpecialFlags]` | no stock rules section; runtime default on | `0x006B8CA0` | SpecialFlags bit `0x8000`, outer bridge AoE gate | Yes; map override only campaign/editor |
| `DestroyableBridges` | rules `[CombatDamage]` | `yes`, `ini/rulesmd.ini:804` | no read in `RulesClass::ReadCombatDamage` | no-op for binary rules parser | No as rules key |
| `BridgeStrength` | `[CombatDamage]` | `1500`, `ini/rulesmd.ini:816` | `0x0066CD80` to `Rules+0x1740` | upper bound for inclusive `Random(1, BridgeStrength)` | Yes |
| `IonCannonWarhead` | `[CombatDamage]` | `IonCannonWH`, `ini/rulesmd.ini:874` | `0x0066CA9B` to `Rules+0xFF0` | bypasses BridgeStrength RNG and enables retry on state-machine path | Yes as bridge-damage special identity |
| `Wall` | warhead section | varies | `0x0075D508` to `WarheadType+0x144` | required for bridge tile damage | Yes |
| `CellSpread` | warhead section | varies | `0x0075D3EB` to `WarheadType+0x124` | object splash radius; not required for direct CellSpread=0 bridge tile hit when a wall warhead impacts a cell | Yes |

## 5. Integration Points

`Apply_area_damage` has live standard YR callers: weapon detonation via `WarheadTypeClass__Detonate @ 0x004690B0`, damaging anim ticks via `AnimClass__AI @ 0x00423AC0`, `LightningStorm__GroundStrike @ 0x0053A300`, `PsychicDominator__MindControlArea @ 0x0053B080`, `SuperClass__Launch @ 0x006CC390`, and several unit/terrain effects. This makes the bridge AoE gate active for normal combat and superweapons, not a TS-only leftover.

C4/CABHUT contrast: hut-death collapse does not use this outer `DestroyableBridges`/`Wall` AoE entry and must remain a separate path. Active in YR: Yes for contrast only; evidence from `DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md` and already-settled parent context.

## 6. Current Rust Implementation Status

Scanned Rust surfaces:

- `src/rules/ruleset.rs:715` defines `BridgeRules`; `src/rules/ruleset.rs:754` currently reads `DestroyableBridges` from `[CombatDamage]`, and `src/rules/ruleset.rs:2517` has a regression test saying gamemd reads `[CombatDamage]`. This is now contradicted by binary evidence.
- `src/rules/ruleset.rs:751` correctly reads `[CombatDamage] BridgeStrength`.
- `src/rules/warhead_type.rs:151` parses warhead `Wall`.
- `src/sim/combat/mod.rs:996`, `src/sim/combat/mod.rs:1878`, and `src/sim/combat/mod.rs:1914` emit bridge damage events only when `warhead.wall && damage > 0`.
- `src/sim/bridge_state/mod.rs:788` and `src/sim/world/bridge_orchestrator.rs:63` carry and apply the destroyable gate at runtime.
- `src/sim/world/bridge_orchestrator.rs:1228` rolls inclusive `1..=BridgeStrength` and requires `roll < damage`, matching the binary's strict comparison.
- `src/sim/combat/combat_aoe.rs:206` selects object splash layer by strict `impact_z > level + bridge_height/2`, matching prior layer report; this is separate from tile damage.

Main Rust delta: the source of `DestroyableBridges` is wrong. Rust should not treat stock rules `[CombatDamage] DestroyableBridges=no` as the binary's rules parser behavior. The field should represent runtime SpecialFlags/default behavior, with map `[SpecialFlags]` override only in the modes the binary reads, or at minimum the current rules parser/test comments must stop asserting the opposite.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Apply_area_damage` bridge tile entry | verified | `0x00489280`; disassembly range `0x00489F00..0x0048A2D0` sampled | none for entry gates |
| `Apply_area_damage` object layer selector distinction | verified from prior + spot-check | `0x00489560..0x004896D8`; `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` | bridge-tolerance edge still outside this target |
| `DestroyableBridges` reader/source | verified | `0x006B8CA0`, `0x00689E90`, `0x006B8B30` | exact constructor literal for runtime default still deferred to prior doc |
| `RulesClass::ReadCombatDamage` absence of `DestroyableBridges` and presence of `BridgeStrength`/`IonCannonWarhead` | verified | `0x0066BBC9`, `0x0066CA9B`, `0x0066CD80` | none |
| warhead `Wall` parser | verified | `0x0075D3A0`, key read at `0x0075D508` | none |
| impact-Z bridge tile window | verified | `0x00489280`; high/low structural branches | exact named semantics of height globals comes from sibling bridge docs |
| random comparison and Ion bypass | verified | `0x00489280`; `Rules+0x1740`, `Rules+0xFF0` | exact RNG sequence across multiple simultaneous events is sibling scope |
| low/high state-machine internals after `ApplyDamageToCell` | deferred | out of scope | slot 3 / high bridge state machine |
| C4/CABHUT collapse | deferred | out of scope | slot 1 |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-1 - Is AoE bridge damage entered through a live YR function? -> Yes, `Apply_area_damage` has standard weapon/superweapon/damaging anim callers.` (evidence: `get_function_callers Apply_area_damage`, `0x004690B0`, `0x0053A300`, `0x0053B080`, `0x006CC390`)
- `[RESOLVED] OQ-2 - What gates the bridge tile damage block? -> SpecialFlags bit `0x8000` and warhead `Wall` at `+0x144`.` (evidence: `0x00489280`)
- `[RESOLVED] OQ-3 - Where is `DestroyableBridges` read? -> `[SpecialFlags]`, bit 15, in `FUN_006B8CA0`, called by `ScenarioClass__Read_INI_Basic`.` (evidence: `0x006B8CA0`, caller `0x00689E90`)
- `[RESOLVED] OQ-4 - Is `[CombatDamage] DestroyableBridges` a binary rules key? -> No; `RulesClass::ReadCombatDamage` does not read it, though stock rules contain the line.` (evidence: `0x0066BBC9`, `ini/rulesmd.ini:804`)
- `[RESOLVED] OQ-5 - Where is `BridgeStrength` read and how compared? -> Read to `Rules+0x1740`, inclusive roll `Random(1, BridgeStrength)`, path passes only on strict `< damage`.` (evidence: `0x0066CD80`, `0x00489280`)
- `[RESOLVED] OQ-6 - Where is `IonCannonWarhead` read and what does it do? -> Read to `Rules+0xFF0`; matching warhead bypasses the RNG gate and allows retry on state-machine path.` (evidence: `0x0066CA9B`, `0x00489280`)
- `[RESOLVED] OQ-7 - Is the bridge tile Z gate inclusive or exclusive? -> Upper bound inclusive, lower bound exclusive as documented in section 3.4.` (evidence: `0x00489280`)
- `[RESOLVED] OQ-8 - Does `Wall=yes` select object splash layer? -> No; object layer selector is earlier and independent; `Wall` gates tile/overlay damage.` (evidence: `0x00489560..0x004896D8`, `0x00489F00..0x0048A2D0`)
- `[RESOLVED] OQ-9 - Does Rust already model strict RNG compare? -> Yes in the orchestrator; inclusive roll and `< damage` are present.` (evidence: `src/sim/world/bridge_orchestrator.rs:1228`)
- `[RESOLVED] OQ-10 - Does Rust parse the correct DestroyableBridges source? -> No, it currently parses `[CombatDamage]`, contrary to binary evidence.` (evidence: `src/rules/ruleset.rs:754`, `0x006B8CA0`)
- `[DEFERRED] OQ-11 - What exact constructor instruction initializes SpecialFlags bit 15?` (category: `requires-different-system-context`; reason: prior report establishes default-on behavior, but this slice only needed reader/consumer/source proof; next-step-if-pursued: audit ScenarioClass/SpecialFlags constructor/reset literal)
- `[DEFERRED] OQ-12 - Full high/low state-machine outcomes after `ApplyDamageToCell`.` (category: `out-of-scope`; reason: assigned to bridge collapse state-machine swarm slots; next-step-if-pursued: use slot 3/4 reports)

Deferred pile is small and does not affect the entry-gate/Rust handoff conclusions.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `DestroyableBridges` is a scenario SpecialFlags bit, not a `[CombatDamage]` rules key; normal skirmish default stays on | `0x006B8CA0`, `0x00689E90`, `0x006B8B30`; no read in `0x0066BBC9`; stock `ini/rulesmd.ini:804` is no-op for binary rules parser | mismatch: `src/rules/ruleset.rs:754` reads `[CombatDamage]`, tests assert SpecialFlags ignored because of wrong premise | `src/rules/ruleset.rs`, map/scenario SpecialFlags loader surface, `BridgeRuntimeState::from_resolved_terrain` caller | stop treating `[CombatDamage] DestroyableBridges` as the authoritative rules default; feed bridge destroyable flag from runtime default plus scenario/map `[SpecialFlags]` in the same modes as binary | rules INI with `[CombatDamage] DestroyableBridges=no` still initializes bridge state destroyable in stock-skirmish setup; campaign/map SpecialFlags override can disable AoE bridge tile damage | proposed test `test_combatdamage_destroyablebridges_no_is_noop_for_bridge_damage_default`; do not "fix" by hardcoding all maps on/off |
| Bridge tile damage requires `warhead.Wall` but object splash layer does not | `0x0075D3A0` parser `Wall -> +0x144`; `0x00489280` bridge gate; object selector `0x00489560..0x004896D8` | mostly matches: event emission guarded by `warhead.wall`, object AoE uses `AoELayerContext`; keep separation tested | `src/sim/combat/mod.rs`, `src/sim/combat/combat_aoe.rs`, `src/rules/warhead_type.rs` | preserve event emission only for wall warheads while allowing non-wall CellSpread object splash on selected layer | non-wall splash on bridge damages selected-layer occupants but emits no `BridgeDamageEvent`; wall splash emits bridge event | proposed test `test_non_wall_cellspread_hits_bridge_occupants_without_bridge_tile_event`; do not use `Wall` as object-layer selector |
| Non-Ion bridge tile damage uses inclusive `Random(1, BridgeStrength)` and strict `roll < damage`; equality fails | `0x00489280`; `0x0066CD80`; stock `ini/rulesmd.ini:816`; Rust `src/sim/world/bridge_orchestrator.rs:1228` | none observed for comparison; add boundary regression if absent | `src/sim/world/bridge_orchestrator.rs`, bridge state tests | preserve equality-fails boundary and deterministic RNG draw order | configured RNG roll equal to damage does not collapse bridge; roll one below damage can collapse | proposed test `test_bridge_damage_rng_equal_damage_does_not_pass`; do not change to `<=` |
| Structural bridge tile candidates use impact-Z window before state-machine dispatch; direct overlay ranges do not | `0x00489280` structural branches; direct ranges `0x0048A214..0x0048A2C4`; Rust `path_matches_cell` Z tests | mostly present; ensure tests cover lower-exclusive/upper-inclusive boundaries against binary formula | `src/sim/bridge_state/mod.rs`, `src/sim/world/bridge_orchestrator.rs` | preserve state-machine-only Z window and direct-overlay bypass | impact exactly at lower bound skips state-machine path, impact at upper bound passes; direct raw overlay path ignores this window | proposed test `test_bridge_damage_z_window_lower_exclusive_upper_inclusive`; do not apply Z gate to direct low/high overlays |

### Negative Facts / Do Not Do

- Do not parse rules `[CombatDamage] DestroyableBridges` as the binary's bridge damage gate; binary reads `[SpecialFlags] DestroyableBridges` into scenario bit `0x8000`. Evidence: `0x006B8CA0`, `0x0066BBC9`, `ini/rulesmd.ini:804`.
- Do not use `Wall=yes` to decide object splash layer; it only gates tile/overlay bridge damage. Evidence: `0x00489560..0x004896D8` vs `0x00489F00..0x0048A2D0`.
- Do not change the random comparison to `<=`; equality fails. Evidence: `0x00489280`.
- Do not apply `DestroyableBridges` to C4/CABHUT collapse through this entry; that is a separate hut-death path. Evidence: `DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md`; parent settled facts.
- Do not treat Ion trigger/sidebar semantics as necessary for this standard weapon AoE entry; only the warhead pointer identity at `Rules+0xFF0` matters here. Evidence: `0x0066CA9B`, `0x00489280`.

### Remaining Uncertainty

- The exact constructor/reset instruction that makes SpecialFlags bit 15 default-on was not re-traced in this slice; the consumer/source proof is complete, but a future default-literal audit would make the default proof stronger.
- Exact high/low state-machine outcomes after `ApplyDamageToCell` are intentionally delegated to other bridge-collapse swarm slots.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`: replace "Retail rules have `DestroyableBridges=yes`, but this pass verified the flag use, not the INI writer that maps the key to the flag." with "The consumer flag is SpecialFlags bit `0x8000`; later verification shows the binary reads `DestroyableBridges` from scenario/map `[SpecialFlags]`, not rules `[CombatDamage]`. The stock `[CombatDamage] DestroyableBridges=yes` line is not the rules parser source."
- `src/rules/ruleset.rs:2518` is code, not a research doc, but its test comment is stale relative to binary evidence: replace the premise "gamemd reads DestroyableBridges from [CombatDamage]" with the SpecialFlags-source fact when implementing.

## Sources

- Ghidra decompiled/read:
  - `0x00489280` `Apply_area_damage`
  - `0x00587180` `ApplyDamageToCell`
  - `0x006B8CA0` SpecialFlags reader
  - `0x006B8B30` SpecialFlags writer
  - `0x00689E90` `ScenarioClass__Read_INI_Basic`
  - `0x0066BBC9` `RulesClass__ReadCombatDamage`
  - `0x0075D3A0` `WarheadTypeClass__ReadINI_Body`
  - `0x004690B0` `WarheadTypeClass__Detonate` as a standard caller
- Ghidra caller evidence:
  - `get_function_callers Apply_area_damage`: `WarheadTypeClass__Detonate`, `AnimClass__AI`, `LightningStorm__GroundStrike`, `PsychicDominator__MindControlArea`, `SuperClass__Launch`, `NukeGroundZero__ApplyDamage`, and other live callers.
  - `get_function_callers FUN_006B8CA0`: `ScenarioClass__Read_INI_Basic`.
- Read-only disassembly ranges sampled:
  - `0x00489560..0x004896D8`
  - `0x00489F00..0x0048A2D0`
  - `0x006B8E00..0x006B8E40`
  - `0x00689EA0..0x00689EB5`
- Docs referenced:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- INI checked:
  - `ini/rulesmd.ini:804`, `ini/rulesmd.ini:816`, `ini/rulesmd.ini:874`
  - `ini/rules.ini:664`, `ini/rules.ini:676`, `ini/rules.ini:695`
- Rust scanned:
  - `src/rules/ruleset.rs`
  - `src/rules/warhead_type.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/combat/combat_aoe.rs`
  - `src/sim/bridge_state/mod.rs`
  - `src/sim/world/bridge_orchestrator.rs`
