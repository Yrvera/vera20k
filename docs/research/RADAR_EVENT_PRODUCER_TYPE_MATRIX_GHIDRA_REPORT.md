# Radar Event Producer Type Matrix - Ghidra Research Report

**Address(es):** `0x0065FA70`, `0x004F93E0`, `0x0073851D`, `0x0071B04C`, `0x0070DAD7`, `0x004FB631`, `0x004D98FE`, `0x0044B960`, `0x0044BDB2`, `0x0045722F`, `0x00448477`, `0x00430F08`, `0x004316E5`, `0x004468A8`, `0x00467EA7`, `0x00539F89`, `0x006CC4BE`, `0x006CC4D2`, `0x006CCDD7`, `0x006CCF2F`, `0x006CD8E0`, `0x00519BB6`, `0x004582C5`, `0x006DF1CA`
**Investigation Mode:** exhaustive-slice for `CreateRadarEvent` producer call-site type arguments; coverage-map for full upstream gameplay branch semantics.
**Claimed Scope:** matrix of `CreateRadarEvent` producers, type numbers, source coordinate derivation at the call site, standard-YR liveness, renderer consequence, and dirty/update side effects.
**Non-Scope:** radar event geometry/color proof, generic minimap terrain dirty caller matrix, `MarkTerrainDirty` caller inventory, radar viewport rectangle overlay, minimap gadget click provenance, and full internal branch audit of every producer function.
**Confidence:** High for call-site type arguments and immediate post-call EVA gating from assembly context; Medium for semantic producer labels inherited from existing reports where the full producer body was not re-drained here.
**Active in YR:** Yes/Conditional by producer; see matrix.

## Summary

`CreateRadarEvent @ 0x0065FA70` is the single native event allocator. Static call-site evidence confirms a 25-call matrix plus one dynamic trigger-action caller. The compiled engine does not directly emit type `0` combat events; type `0` is reachable through map trigger data. Native bullet impacts and several superweapon launch paths emit type `13`, which the verified renderer treats as default black/no-draw. Bridge repair emits type `14`, also no-draw, but its return value gates `EVA_BridgeRepaired`.

The key Rust-facing consequence is producer mapping, not visual geometry: future Rust must distinguish native visible event types `1/2/3/4/5/11/12` from non-drawing queue/ring/EVA types `6/7/8/9/10/13/14/15/16`, and it must not use the public INI label "Combat" to justify visible diamonds for ordinary weapon impacts.

## Target and Non-Scope

Target question: Which systems enqueue each `RadarEventClass` type, what coordinates do they pass, which are active in standard YR, and what does the known renderer do with each?

Non-goals:

- Do not redo `DrawRadarEvent` geometry/color beyond citing `RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`.
- Do not redo radar/minimap terrain dirty caller inventory.
- Do not investigate viewport rectangle overlay or minimap input provenance.
- Do not modify Rust.

Evidence needed to mark COMPLETE:

- Fresh Ghidra read-only evidence that `CreateRadarEvent` dedups by event type and source cell.
- Fresh Ghidra assembly context for call sites proving `ECX` type arguments and coordinates passed by `PUSH`.
- Cross-check against existing high-confidence `RADAR_EVENT_CLASS_GHIDRA_REPORT.md` caller table.
- Rust scan of current event enum/producers/tests.

Stop conditions:

- Stop before decompiling every upstream producer as a whole subsystem.
- Stop before generic line raster or terrain dirty work.
- Stop before code changes.

## Verified Binary Findings

### Event allocation and side effects

`CreateRadarEvent @ 0x0065FA70` takes the type in `ECX` and a packed cell argument on the stack. If the type-config row's unique byte is set, it scans existing events of the same type, computes integer distance from the new source cell to `event+0x20`, and returns `0` when the distance is less than that type row's dedup distance. If not suppressed, it allocates `0x40` bytes, calls `InitRadarEvent`, and returns `1`.

Evidence: fresh decompile of `0x0065FA70`; existing `RADAR_EVENT_CLASS_GHIDRA_REPORT.md` type table at `DAT_007F0998`.

Renderer consequence is from sibling report `RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`: types `0/3/4` draw white, `1/2/11/12` draw yellow, `5` draws cyan, and default types `6/7/8/9/10/13/14/15/16` enqueue but skip visible diamond drawing.

### Producer matrix

| Producer / call site | Type | Coordinate source at call site | Post-call coupling | Renderer result | Active in standard YR? | Evidence |
|---|---:|---|---|---|---|---|
| `HouseClass::NotifyUnderAttack` own base path | `3` BaseUnderAttack | attacked object's coords converted to cell by vtable `+0x48` result and `>> 8` | `AL` gates `EVA_OurBaseIsUnderAttack` | visible white | Yes, when local human player's base object is attacked | asm `0x004F953F MOV ECX,0x3`; call `0x004F9544`; EVA string ptr after call |
| `HouseClass::NotifyUnderAttack` harvester path | `4` HarvesterUnderAttack | attacked object's coords converted to cell | `AL` gates `EVA_OreMinerUnderAttack` | visible white | Yes, when local human harvester-like unit is attacked | asm `0x004F94DF MOV ECX,0x4`; call `0x004F94E4` |
| `UnitClass::ReceiveDamage` harvester path | `4` HarvesterUnderAttack | damaged unit coords converted to cell | `AL` gates `EVA_OreMinerUnderAttack` | visible white | Yes, when local qualifying unit takes damage | asm `0x00738509 MOV ECX,0x4`; call `0x0073851D` |
| `TemporalClass::InitiateWarp` harvester path | `4` HarvesterUnderAttack | temporal target coords converted to cell | `AL` gates `EVA_OreMinerUnderAttack` | visible white | Conditional; local unit target with resource-collector-like type flag | asm `0x0071B038 MOV ECX,0x4`; call `0x0071B04C`; prior doc OQ2 |
| `TechnoClass::IdleAnimDispatch` enemy sensed path | `5` EnemyObjectSensed | sensed object's coords converted to cell | no EVA in immediate context | visible cyan | Yes, when the local player senses/discovers an enemy object through this path | asm `0x0070DAC3 MOV ECX,0x5`; call `0x0070DAD7` |
| `HouseClass::Place_Production` | `6` UnitReady | produced object's/object-type placement coords converted to cell | `AL` gates `EVA_UnitReady` | no-draw | Yes, production delivery path | asm `0x004FB62C MOV ECX,0x6`; call `0x004FB631` |
| unlabeled `TechnoClass` loss notifier body `0x004D98D4..0x004D9919` | `7` UnitLost | techno coords via vtable `+0x1B8` | `AL` gates `EVA_UnitLost` | no-draw | Yes, local human owner loss notification | asm `0x004D98F9 MOV ECX,0x7`; call `0x004D98FE` |
| `BuildingClass::MissionRepairAndProduce` site A | `8` UnitRepaired | repaired object coords via vtable `+0x1B8` | `AL` gates `EVA_UnitRepaired` | no-draw | Yes, repair/building factory completion path | asm `0x0044B95A MOV ECX,0x8`; call `0x0044B960` |
| `BuildingClass::MissionRepairAndProduce` site B | `8` UnitRepaired | repaired object coords via vtable `+0x1B8` | `AL` gates `EVA_UnitRepaired` | no-draw | Yes, second repair/produce branch | asm `0x0044BDAD MOV ECX,0x8`; call `0x0044BDB2` |
| `BuildingClass::OnSpyInfiltrate` dispatcher | `9` SpyInfiltration | infiltrated building coords via vtable `+0x1B8` | return writes a local success byte used by later branch logic | no-draw | Yes, spy infiltration of live spyable buildings | asm `0x00457229 MOV ECX,0x9`; call `0x0045722F`; `TEST AL` then stores byte |
| `BuildingClass::ChangeOwner` no-NeedsEngineer captured path | `10` BuildingCaptured | captured building coords via vtable `+0x1B8` | `AL` gates `EVA_BuildingCaptured` | no-draw | Conditional; captured building type/path | asm `0x00448472 MOV ECX,0xA`; call `0x00448477` |
| `RadarClass::PlaceBeacon` detected branch | `11` BeaconPlaced | beacon row/col cell derived in beacon path | `AL` gates `EVA_BeaconDetected` | visible yellow | Yes in multiplayer beacon behavior | asm `0x00430F03 MOV ECX,0xB`; call `0x00430F08` |
| `FUN_00431450` beacon helper | `11` BeaconPlaced | helper row/col cell | no immediate EVA in context | visible yellow | Conditional; beacon helper branch | asm `0x004316D1 MOV ECX,0xB`; call `0x004316E5`; prior doc OQ3 |
| `BuildingClass::OnConstructionComplete` | `12` ConstructionComplete | completed building cell | followed by owner/human checks and post-completion calls, not immediate EVA gate | visible yellow | Yes, construction completion path | asm `0x00446894 MOV ECX,0xC`; call `0x004468A8` |
| `BulletClass::AI` impact/detonation path | `13` ImpactSilent | bullet/target coords via vtable `+0x1B8` path | no EVA; continues to anim lookup | no-draw | Yes, normal weapon impact path | asm `0x00467EA2 MOV ECX,0xD`; call `0x00467EA7` |
| `LightningStorm::Start` | `13` ImpactSilent | storm target cell | no immediate EVA | no-draw | Yes for Lightning Storm superweapon | asm `0x00539F84 MOV ECX,0xD`; call `0x00539F89` |
| `SuperClass::Launch` site A | `13` ImpactSilent | launch/target cell near `EBX+0x62` | no immediate EVA | no-draw | Conditional by launched superweapon kind | asm `0x006CC4B9 MOV ECX,0xD`; call `0x006CC4BE` |
| `SuperClass::Launch` site B | `13` ImpactSilent | cell from `EDI`/target object | no immediate EVA | no-draw | Conditional by launched superweapon kind | asm `0x006CC4CA MOV ECX,0xD`; call `0x006CC4D2` |
| `SuperClass::Launch` site C | `13` ImpactSilent | cell from `ESI`/target object | no immediate EVA | no-draw | Conditional by launched superweapon kind | asm `0x006CCDCF MOV ECX,0xD`; call `0x006CCDD7` |
| `SuperClass::Launch` site D | `13` ImpactSilent | cell from `EBP` | after `PlayEVA` block | no-draw | Conditional by launched superweapon kind | asm `0x006CCF29 MOV ECX,0xD`; call `0x006CCF2F` |
| `SuperClass::Launch` site E | `13` ImpactSilent | launch target from superweapon targeting helper | no immediate EVA | no-draw | Conditional by launched superweapon kind | asm `0x006CD8DB MOV ECX,0xD`; call `0x006CD8E0` |
| `InfantryClass::PerCellProcess` / CABHUT bridge repair branch | `14` BridgeRepaired | engineer/cell coords via vtable `+0x1B8` | `AL` gates `EVA_BridgeRepaired`; optional repair sound follows | no-draw | Yes, stock `BridgeRepairHut=yes` and engineer repair | asm `0x00519BB1 MOV ECX,0xE`; call `0x00519BB6`; bridge report |
| `BuildingClass::CheckAutoSellOrCivilian` | `15` StructureAbandoned | building coords via vtable `+0x1B8` | `AL` gates `EVA_StructureAbandoned` | no-draw | Conditional; auto-sell/civilian-abandon path | asm `0x004582C0 MOV ECX,0xF`; call `0x004582C5` |
| `HouseClass::NotifyUnderAttack` ally branch | `16` AllyUnderAttack | attacked allied object coords converted to cell | `AL` gates `EVA_OurAllyIsUnderAttack` plus sound | no-draw | Yes in allied multiplayer/skirmish attack conditions | asm `0x004F959B MOV ECX,0x10`; call `0x004F95A0` |
| `TriggerAction::Execute` | dynamic from `[ESI+0x90]` | trigger action target cell from trigger data/context | no immediate EVA | depends on selected type | Conditional; mission/map trigger action can request arbitrary type, including type `0` | asm `0x006DF1C3 MOV ECX,[ESI+0x90]`; call `0x006DF1CA` |

### Type zero

No direct compiled call site found in the verified xref table loads `ECX=0` before `CreateRadarEvent`. Type `0` draws white if spawned, but in stock engine code it is only reachable through `TriggerAction::Execute` dynamic map data. This corrects the tempting assumption that ordinary combat bullet impacts use type `0`; they use type `13`.

### Dirty/update side effects

All producer paths share `CreateRadarEvent` side effects: event array insertion, source-cell ring insertion for Spacebar cycling, and optional dedup suppression. The producer call itself does not call `RadarClass::MarkCellDirty` or `MarkTerrainDirty`. Visible event pixels are drawn later by `RadarClass::Update -> TickAndDrawRadarEvents`, after terrain/object minimap pixels and before spy-satellite overlay, per `RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`.

## Active in Standard YR?

- Active common paths: base/harvester/ally under attack, unit ready/lost/repaired, construction complete, bullet impacts, bridge repair, beacons, spy infiltration, building captured/abandoned, and standard superweapon launch branches where their owning superweapon exists in stock YR.
- Conditional but standard-capable paths: `TemporalClass::InitiateWarp` harvester warning, beacon helper variants, `SuperClass::Launch` branches by superweapon kind, and `TriggerAction::Execute` by map trigger data.
- Not found as a direct compiled producer: type `0` Combat. It is trigger-action reachable.
- Psychic/reveal note: passive SpySat and gap/reveal flows covered by sibling reports mutate shroud/fog and call radar refresh; no new `CreateRadarEvent` producer was verified here for Psychic Reveal. Psychic Dominator/superweapon launch falls under `SuperClass::Launch` type `13`/no-draw from the existing xref set.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary bullet impacts and several superweapon launch sites emit type `13`, which does not draw | asm `0x00467EA2..0x00467EA7`, `0x00539F84..0x00539F89`, `0x006CC4B9..0x006CD8E0`; renderer report `0x00660050` | mismatch: `src/sim/world/mod.rs:1520` pushes `RadarEventType::Combat`, a visible type `0`, for combat events | `src/sim/radar.rs`, `src/sim/world/mod.rs`, combat/superweapon event emitters, `src/render/minimap.rs` | Add a native `ImpactSilent`/type-13 event and route bullet/superweapon impact producers to no-draw queue/ring behavior | A normal weapon impact queues a Spacebar event but shows no minimap diamond | `test_bullet_impact_queues_type13_without_visible_ping`; do not equate INI Combat with weapon impacts |
| `CreateRadarEvent` return gates specific EVA calls for attack, repair, ready/lost/repaired, captured/abandoned, and beacon-detected paths | fresh assembly contexts show `TEST AL,AL` before `VoxClass::PlayEVA` for those sites | partial: bridge repair now carries `eva_allowed`; most other producer/EVA couplings are missing/unchecked | future audio/EVA bridge, `src/sim/radar.rs`, gameplay producer hooks | Preserve per-producer EVA gating only where native tests `AL`; do not globally gate every EVA by radar event | Duplicate base-under-attack within dedup radius suppresses `EVA_OurBaseIsUnderAttack`, while non-dedup construction complete does not use the same coupling | `test_base_under_attack_eva_is_gated_by_type3_radar_dedup`; risk: wrong audio spam/rate-limit |
| Native event enum is 17 slots plus dynamic trigger action; visible draw table is not the same as semantic producer labels | `RADAR_EVENT_CLASS_GHIDRA_REPORT.md` table; fresh contexts for types `3..16`; renderer report `0x00660050` | partial: Rust has 7 enum variants and wrong colors for `Dropzone` and `EnemyObjectSensed`; many no-draw semantic types missing | `src/sim/radar.rs`, `src/render/minimap.rs`, `src/rules/radar_event_config.rs` | Model native numeric types and draw/no-draw switch separately from semantic labels/EVA labels | Type 5 EnemyObjectSensed draws cyan; type 11 BeaconPlaced draws yellow; type 14 BridgeRepaired queues but does not draw | `test_radar_event_numeric_type_draw_table_matches_gamemd`; risk: label-driven color drift |
| Type `0` has no direct stock compiled producer in the xref set and is dynamic-trigger reachable | prior xref pass plus fresh dynamic context `0x006DF1C3 MOV ECX,[ESI+0x90]` | mismatch risk: Rust uses `Combat` type as a normal combat producer | `src/sim/radar.rs`, trigger/map action implementation | Reserve type `0` for trigger-action/native event type data unless a later direct producer is proven | Map trigger action requesting type 0 draws a white event; normal combat impact does not | `test_trigger_action_can_emit_type0_combat_ping_but_weapon_impact_uses_type13`; risk: stock combat overdraw |
| Bridge repair type `14` is no-draw but gates EVA and ring insertion | asm `0x00519BB1..0x00519BC9`; bridge report | mostly present now: Rust has `BridgeRepaired` and bridge repair tests, but needs to remain non-drawing and type-number-compatible | `src/sim/radar.rs`, `src/sim/world/world_orders.rs`, `src/app_sim_tick.rs` | Preserve event-before-mutation, no visible diamond, and dedup-gated EVA | Two engineer repairs within 8 cells only allow the first `EVA_BridgeRepaired`; minimap shows no diamond | existing/extend `bridge_repaired_event_suppresses_and_does_not_draw`; risk: regression to visible bridge ping |

## Negative Facts / Do Not Do

- Do not use `RadarEventType::Combat` / type `0` for ordinary weapon impacts; verified bullet path uses type `13` at `0x00467EA7`.
- Do not make type `13` or type `14` visible just because they enqueue; renderer report verifies both are default black/no-draw in `DrawRadarEvent @ 0x00660050`.
- Do not treat `RadarEventSuppressionDistances` INI comments as a complete runtime type table; sibling report verifies the parsed arrays are not copied into `DAT_007F0998`.
- Do not globally gate every EVA on radar-event success; only call sites that test `AL` before `VoxClass::PlayEVA` should be coupled.
- Do not sort or coalesce producer events by semantic label; native dedup is same numeric type plus per-row unique flag and distance.

## Remaining Uncertainty

- Full upstream branch semantics inside every producer function were not re-drained; this report verifies the event call sites, not every gameplay precondition.
- Dynamic `TriggerAction::Execute` map data type values were not enumerated across stock maps.
- Exact superweapon-kind mapping for each `SuperClass::Launch` type-13 site remains a follow-up if per-superweapon event provenance is needed.
- Psychic Reveal did not show a new verified `CreateRadarEvent` producer in this slice; sibling SpySat/gap/reveal docs should remain the source for refresh-based minimap effects.

## Stale-Doc Replacement Wording

- Replace any wording that says "type 0 Combat is used for bullet impacts" with: "Compiled weapon-impact code uses `CreateRadarEvent(13, cell)`, which enqueues/ring-buffers but does not draw; type `0` is visible white when spawned by dynamic trigger data, and no direct compiled stock caller was found."
- Replace any wording that says "BridgeRepaired drives a minimap blip" with: "Bridge repair creates type `14`; it can gate `EVA_BridgeRepaired` and update the event ring, but the native renderer skips a visible diamond for type `14`."
- Replace any wording that says "all 17 type slots are pushed by live YR code paths" with: "Types `3..16` have verified compiled producers, type `0` is dynamic trigger-action reachable, and type `1/2` visibility is verified by renderer/table but compiled producer liveness was not re-proven in this slot beyond existing `RADAR_EVENT_CLASS` claims."

## Status

COMPLETE for the requested producer type matrix from verified `CreateRadarEvent` call-site assembly contexts and prior xref table reconciliation.

PARTIAL only for exhaustive internal preconditions of every upstream producer and stock-map enumeration of dynamic trigger-action values.

## Sources

- Ghidra decompile: `CreateRadarEvent @ 0x0065FA70`.
- Ghidra assembly contexts: `0x004F9544`, `0x004F94E4`, `0x004F95A0`, `0x0073851D`, `0x0071B04C`, `0x0070DAD7`, `0x004FB631`, `0x004D98FE`, `0x0044B960`, `0x0044BDB2`, `0x0045722F`, `0x00448477`, `0x00430F08`, `0x004316E5`, `0x004468A8`, `0x00467EA7`, `0x00539F89`, `0x006CC4BE`, `0x006CC4D2`, `0x006CCDD7`, `0x006CCF2F`, `0x006CD8E0`, `0x00519BB6`, `0x004582C5`, `0x006DF1CA`.
- Existing reports: `docs/research/RADAR_EVENT_CLASS_GHIDRA_REPORT.md`, `docs/research/RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md`, `docs/research/bridges/06-render-presentation-audio/BRIDGE_PRESENTATION_RADAR_DIRTY_GHIDRA_REPORT.md`, `docs/research/SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md`, `docs/research/GAP_RADAR_SHROUD_MINIMAP_INTERACTION_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/radar.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`, `src/render/minimap.rs`, `src/app_sim_tick.rs`, `src/app_input.rs`.
