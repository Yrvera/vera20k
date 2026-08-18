# Combat/Smudge Reduce_Tiberium Side Effects Trace

**Scenario:** A weapon or crater/smudge effect reduces or clears a standard Riparius ore cell during active Yuri's Revenge gameplay.

**Scope guard:** This trace is only about the shared `CellClass::Reduce_Tiberium` side effects reached from combat/crater paths. It does not trace harvester cargo, refinery unload, TIBTRE spawning, or TS weed/vein gameplay.

**Status:** PARTIAL. The live YR entry points and shared side-effect bundle are verified. Exact `Apply_area_damage` stack amount at the `0x00489665` call remains UNCHECKED, so any combat amount equality claim is blocked.

## Pipeline

1. **Weapon AoE trigger**
   - gamemd: `Apply_area_damage @ 0x00489280` scans affected cells using `WarheadType.CellSpread`; xref to `CellClass::Reduce_Tiberium @ 0x00489665`.
   - Rust: `tick_combat_with_fog` calls `destroy_ore_at_impact` from impact/death-explosion paths, then `destroy_ore_at_impact` calls `miner::reduce_tiberium`.
   - Verdict: UNCHECKED for exact affected-cell list and amount. Current Rust uses `cell_spread.to_num::<u32>()` plus `cells_in_spread`; gamemd uses `ftol(wh->CellSpread)` and static cell tables. I did not compute both sets for a concrete warhead.

2. **Crater/smudge trigger**
   - gamemd: `AnimClass::Start @ 0x00424F00` runs once when the animation starts. If height `< 30` and `AnimType.Crater` is true, it calls `CellClass::Reduce_Tiberium(6)` before `Debris_Smoke`.
   - Rust: `try_dispatch_anim_smudge` uses `CRATER_ORE_REDUCTION = 6`, calls `reduce_tiberium(...)`, then attempts crater placement.
   - Verdict: PASS for the hardcoded reduction amount `6` and order before smudge placement.

3. **Standard YR liveness**
   - gamemd: `Apply_area_damage` is reached from standard weapon/superweapon/damaging anim paths; `AnimClass::Start` crater logic is reached from `AnimList` animations with `Crater=yes`. `SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md` marks this active in YR.
   - TS boundary: This is not the TS `Weeder` path. `Tiberium` naming is inherited, but the ore/crater code is YR-live. Vein-specific behavior is adjacent and not traced here.
   - Verdict: PASS for active YR path identity.

## Stage Findings

| Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|
| Crater reduction amount | `Reduce_Tiberium(6)` | `CRATER_ORE_REDUCTION = 6` | PASS |
| Crater reduction ordering | reduce before `Debris_Smoke`; reduction occurs even if crater placement fails later | reduce before `SmudgeGrid::try_place`; unit test covers overlay-blocked placement | PASS |
| Weapon reduction gate | `Apply_area_damage` reduces only after overlay/caller gates: overlay chain/tiberium flags, warhead/caller gate, `allowTiberiumChain` | `destroy_ore_at_impact` reduces any resource node for any positive `base_damage / 10` | FAIL |
| Weapon reduction amount | call exists at `0x00489665`, but exact pushed amount not derived in this slot | `ore_damage = (base_damage / 10).max(0) as u16` | UNCHECKED |
| Full-removal overlay mutation | `OverlayTypeIndex=-1`, `OverlayData=0`, then `RecalcAttributes` | `resource_nodes.remove(cell)` only | FAIL |
| Partial overlay mutation | subtracts amount from `CellClass+0x11E OverlayData`; returns amount | subtracts `amount * base` from `ResourceNode.remaining`; no overlay data update | FAIL |
| Full-removal return for density | returns pre-removal `OverlayData`; e.g. `OverlayData=11` returns `11` | returns `density_levels` derived from `remaining / base` | UNCHECKED for this combat/smudge scenario because current Rust resource-node seeding may not equal gamemd `OverlayData` |
| Resource node sync | gamemd has no separate Rust-style node; cell overlay/data is authoritative | resource node can change while overlay grid remains unchanged | FAIL |
| Growth queue side effect | density-11 branch calls `AddToGrowthQueue`; for `OverlayData=11` this is net no-op because callee checks `< 11` before enqueue | no growth queue equivalent in helper | NOT-IMPLEMENTED |
| Spread queue side effect | full removal clears spread bitmap entry for all tib types and reseeds eligible neighbors into removed type's spread queue | no queue bitmap clear or neighbor reseed | NOT-IMPLEMENTED |
| Radar/tactical dirty | full removal marks terrain dirty for radar; both partial/full dirty tactical screen rect | no radar/tactical dirty event from helper | NOT-IMPLEMENTED |
| Ownership boundary | shared `CellClass` method used by harvesters, weapon AoE, anim crater, map radius, wall extension | helper lives in `sim::miner` and is reused by combat/smudge | FAIL |

## Current Rust Evidence

- `src/sim/miner/mod.rs:395` defines `reduce_tiberium(resource_nodes, cell, amount)`.
- `src/sim/miner/mod.rs:420` partial reduction subtracts from `ResourceNode.remaining`.
- `src/sim/miner/mod.rs:424` full reduction removes the resource node.
- `src/sim/combat/smudge_dispatch.rs:63` sets `CRATER_ORE_REDUCTION = 6`.
- `src/sim/combat/smudge_dispatch.rs:140`, `:226`, and `:290` call the miner helper from crater paths.
- `src/sim/combat/mod.rs:1117` computes weapon ore damage as `base_damage / 10`.
- `src/sim/combat/mod.rs:1126` calls the same miner helper for weapon AoE cells.
- `src/sim/combat/mod.rs:1938` unconditionally invokes ore destruction for warhead impacts.
- `src/sim/overlay_grid.rs:93` has `clear_overlay`, but the shared reduction helper does not receive `OverlayGrid`.

## Player-Visible Consequences

1. Full crater/weapon ore removal can leave the visible overlay, passability/land classification, minimap terrain, and future growth/spread state stale because only `resource_nodes` changes.
2. Partial crater/weapon ore reduction can change economy/resource state without changing visible ore density frame.
3. Ore patch regrowth after a crater or weapon clears a cell will diverge because gamemd reseeds the spread queue on full removal.
4. Weapons that should fail the gamemd ore-reduction gate may still reduce ore in Rust because Rust does not use the overlay/warhead/caller gate at the reduction point.
5. The helper location in `sim::miner` encourages miner-specific semantics to leak into combat/smudge behavior; gamemd uses one shared `CellClass` boundary.

## TS/YR Boundary Notes

- Verified YR-active: `AnimClass::Start -> Reduce_Tiberium(6)` for crater animations, `Apply_area_damage -> Reduce_Tiberium` for weapon cell-side effects, and `CellClass::Reduce_Tiberium` side effects.
- Not used for this scenario: TS `Weeder`, weed overlay harvesting, TS fog, and vein damage semantics.
- Adjacent doc conflict: older projectile/warhead docs disagree about whether warhead `Tiberium=` gates standard ore reduction. The current live decompile of `Apply_area_damage` shows the reduction branch is guarded by overlay flags, warhead/caller state, and `param_5`. This report treats the decompile as higher priority and leaves a dedicated gate audit as follow-up.

## Sources

- Ghidra MCP read-only `get_function_xrefs 0x00480A80`.
- Ghidra MCP read-only `decompile_function 0x00480A80`.
- Ghidra MCP read-only `decompile_function 0x00489280`.
- Ghidra MCP read-only `decompile_function 0x00424F00`.
- `docs/research/REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`.
- `docs/research/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md`.
- `docs/research/combat/systems/splash_cellspread.md`.
- `src/sim/miner/mod.rs`.
- `src/sim/combat/mod.rs`.
- `src/sim/combat/smudge_dispatch.rs`.
- `src/sim/overlay_grid.rs`.

