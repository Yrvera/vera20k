# TIBTRE Stock Immune Wood Damage Trace

Scenario: force-fire a stock standard YR `Wood=yes` warhead weapon at a stock `TIBTRE01` terrain cell. Concrete stock weapon data used for the numeric input is `[HoverMissile] Damage=25 Warhead=HE`; `[HE]` has `Wood=yes`.

Scope: this trace only covers the stock `Immune=yes` terrain damage gate and the Rust terrain-object damage path for the hit cell. It does not trace projectile flight timing, all `CellSpread` perimeter cells, TIBTRE non-immune destruction effects, or TIBTRE ore spawning.

## Verdict

PASS: 5 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Evidence

- Stock data: `ini/rulesmd.ini:1644` registers `46=TIBTRE01`; `ini/rulesmd.ini:28109-28121` defines `TIBTRE01` with `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, and `Immune=yes`.
- Stock defaults: `ini/rulesmd.ini:398` sets `TreeStrength=200`; `TERRAIN_CLASS_GHIDRA_REPORT.md:930-946` verifies TerrainType defaults `Armor=Wood(6)`, `Strength=-1 -> TreeStrength`, `RadarInvisible=true`, `LegalTarget=false`, `Insignificant=true`, occupation masks `7`.
- Weapon data: `ini/rulesmd.ini:22557-22564` has `HoverMissile Damage=25 Warhead=HE`; `ini/rulesmd.ini:26545-26554` has `[HE] CellSpread=.5`, `Wood=yes`, and wood armor `Verses[6]=75%`.
- GameMD damage gate: read-only Ghidra decompile of `TerrainClass::Take_Damage @ 0x0071B920` confirms `param_4 == 0 -> return 0`, then the only damage branch requires `Warhead+0x147 != 0` and `TerrainTypeClass+0x233 Immune == 0` before `ObjectClass::ReceiveDamage`.
- Existing report cross-check: `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md:84-90` states stock TIBTRE ordinary damage is skipped by `Immune=yes`; `TERRAIN_CLASS_GHIDRA_REPORT.md:1052-1057` states damage dispatch reaches `vtable[+0x16C]` and `Take_Damage` gates on `warhead.Wood && !type.Immune`.
- Current Rust: `src/rules/terrain_object_type.rs:82-85` parses default `Armor=wood`, `Strength` from tree strength, and `Immune`; `src/rules/warhead_type.rs:154-155` parses `Wall` and `Wood`; `src/sim/terrain_object.rs:161-178` returns `Ignored` when `base_damage <= 0`, `!warhead.wood`, no live object, or `terrain_type.immune`.

## Pipeline

`force-fire cell` -> `weapon/warhead data` -> `area/object damage dispatch` -> `TerrainClass::Take_Damage / Rust terrain damage helper` -> `health/lifecycle/spawner state` -> `screen remains unchanged`

## Stage Results

| Stage | GameMD concrete output | Rust concrete output | Verdict |
|---|---|---|---|
| Stock terrain type data | `TIBTRE01 Immune=yes`; inherited `Armor=Wood(6)`, `Strength=TreeStrength=200`, `SpawnsTiberium=yes`, `IsAnimated=yes`. | RuleSet parses `[TerrainTypes]`, `TreeStrength=200`, and `TIBTRE01`; `TerrainObjectType` stores `immune=true`, `armor="wood"`, `strength=200`, spawn/animation flags true. | PASS |
| Stock weapon/warhead data | `HoverMissile Damage=25`, `Warhead=HE`; `HE Wood=yes`; `HE Verses[wood]=75%`. | `WeaponType.damage=25`; `WarheadType.wood=true`; `verses[6]=75`. | PASS |
| Force-fire reaching terrain damage | Prior report verifies force-fire bypasses cursor gating and reaches `Apply_area_damage`, which dispatches terrain object `vtable[+0x16C]`. | Combat emits `TerrainDamageEvent { rx, ry, damage: base_damage, warhead_ref }` only when `warhead.wood && base_damage > 0`, then `World` applies it to terrain objects. | UNCHECKED: exact player command, projectile impact tick, and full `CellSpread` object iteration were not computed side-by-side. |
| Immune gate | `Take_Damage` sees `Warhead+0x147 Wood = 1` and `TerrainType+0x233 Immune = 1`; the conjunctive branch fails before `ObjectClass::ReceiveDamage`; return remains `0`; HP subtract count is `0`. | `damage_terrain_object_at_cell` sees `warhead.wood=true`, live object present, then `terrain_type.immune=true` and returns `TerrainDamageResult::Ignored` before armor/verses/HP subtraction. | PASS |
| Health/lifecycle state after hit | No `ReceiveDamage` call means health remains `200`, no destroy return code, no Limbo/UnInit call from this damage path. | Health remains initial `terrain_type.strength=200`; lifecycle remains `Live`; no `limbo_terrain_object_at_cell` call. The in-source exact unit scenario at `src/sim/terrain_object.rs:283-305` asserts the same shape with `TreeStrength=10`: result `Ignored`, lifecycle `Live`, health unchanged. | PASS |
| Spawner and derived indices after hit | Since no Limbo/destructor path runs, the live TerrainClass remains in the object list; its `TerrainClass::AI` owner path remains eligible to tick later. | No removal path executes; `terrain_spawners`, `terrain_object_cells`, `tiberium_spawning_terrain_cells`, and `terrain_occupation_bits` are not mutated by the immune return. | PASS |
| Player-visible result | TIBTRE should still stand and remain capable of later ore-spawn animation/tick; no terrain-destruction explosion from this hit. | Sim state keeps the TIBTRE live; rendering still depends on existing terrain-object render pipeline. | UNCHECKED: this trace did not run a rendered frame comparison or pixel check. |

## Findings

No FAIL or NOT-IMPLEMENTED findings for the concrete stock `Immune=yes` damage gate. Current Rust matches the critical outcome for the hit cell: a stock `TIBTRE01` hit by a `Wood=yes` warhead is ignored by terrain damage, keeps HP/lifecycle unchanged, and keeps derived spawner indices.

## Adjacent Findings

- Rust currently emits one terrain damage event at the resolved impact cell in the inspected combat path. Exact parity for every GameMD `CellSpread=.5` cell-object iteration belongs to an AoE terrain-damage trace, not this stock immune center-cell trace.
- Non-immune TIBTRE destruction effects, including explosion animation, chained `C4Warhead` area damage, and exact UnInit ordering, are explicitly outside this stock immune trace.
- Force-fire command legality and projectile travel timing were not computed here; the relevant GameMD evidence only confirms that force-fire can reach terrain `Take_Damage`.

## Sources

- `ini/rulesmd.ini`
- `docs/research/TERRAIN_CLASS_GHIDRA_REPORT.md`
- `docs/research/TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`
- Read-only Ghidra decompile: `TerrainClass::Take_Damage @ 0x0071B920`
- `src/rules/terrain_object_type.rs`
- `src/rules/warhead_type.rs`
- `src/sim/combat/mod.rs`
- `src/sim/world/mod.rs`
- `src/sim/terrain_object.rs`
