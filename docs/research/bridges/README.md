# Bridge Research Index

This folder collects bridge-primary `gamemd.exe` research. It is organized by
mechanism so bridge work can be decoded in the same order the engine consumes it:
map/load state, cell flags and zones, traversal, locomotion height, damage and
repair, then presentation.

## Read First

1. [00-system-models/BRIDGE_SYSTEM.md](00-system-models/BRIDGE_SYSTEM.md) - current full bridge system reference.
2. [00-system-models/BRIDGE_PARITY_GAP_SYSTEM_MODEL_SYNTHESIS.md](00-system-models/BRIDGE_PARITY_GAP_SYSTEM_MODEL_SYNTHESIS.md) - known parity gaps and Rust-facing risk.
3. [00-system-models/BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md](00-system-models/BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md) - bridge layer pathfinding model.
4. [00-system-models/BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md](00-system-models/BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md) - destruction, collapse, and fallout overview.

## Folder Map

- [00-system-models](00-system-models/) - synthesis docs, gap summaries, verification amendments, and current source-of-truth bridge models.
- [01-assets-map-load-overlay](01-assets-map-load-overlay/) - bridge assets, SHP metadata, theater tables, map-load stamping, direction tables, and overlay/body setup.
- [02-cell-state-layering-zones](02-cell-state-layering-zones/) - `CellClass` bridge flags, on-bridge object lists, low/high layer state, zone records, zone refresh, and pavement/cell-list helpers.
- [03-traversal-pathfinding-entry](03-traversal-pathfinding-entry/) - `Can_Enter_Cell`, bridge traversal checks, A* bridge costs, pathfinder bridge passability, low-bridge tie order, and bridge/tunnel entry.
- [04-locomotion-height-tubes](04-locomotion-height-tubes/) - bridge height handling in locomotors, low-bridge `TubeClass`, hover/jumpjet cases, paradrop/parachute layer selection, and stock low-bridge route traces.
- [05-damage-collapse-repair-cabhut](05-damage-collapse-repair-cabhut/) - weapon/AOE bridge damage, high/low collapse state machines, bridge repair, cabhut/C4 entry, repair huts, engineer behavior, sound/rules keys, and the per-symbol bridge repair decode set.
- [06-render-presentation-audio](06-render-presentation-audio/) - bridge render path, display tables, railing source, radar/minimap pixels, under-deck occlusion, cloak shader interaction, and presentation/audio traces.
- [07-cross-system-consumers](07-cross-system-consumers/) - AI, cursor/action, refinery, cloak/visibility, and other consumers whose primary behavior is outside the bridge system but whose bridge branch is important.
- [08-traces](08-traces/) - concrete end-to-end bridge traces, including collapse slots, cabhut collapse, engineer repair, height/pick traces, and low-bridge path traces.

## Related Docs Left In Their System Folders

These are intentionally not moved because their owning system is broader than
bridges. They should still be checked during bridge work:

- [../pathfinding/fn-pathfinder_update_bridge_pass.md](../pathfinding/fn-pathfinder_update_bridge_pass.md)
- [../pathfinding/_system.md](../pathfinding/_system.md)
- [../chronominer-locomotion/fn-cell-has-bridge-overlay.md](../chronominer-locomotion/fn-cell-has-bridge-overlay.md)
- [../chronominer-locomotion/global-bridge-z-offset-teleport.md](../chronominer-locomotion/global-bridge-z-offset-teleport.md)
- [../coord-cell-conversions/_system.md](../coord-cell-conversions/_system.md)
- [../INDEX_PATHFINDING_LOCOMOTION.md](../INDEX_PATHFINDING_LOCOMOTION.md)
- [../COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md](../COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md)
- [../GETEFFECTIVEHEIGHT_PLUS4_UNIT_GHIDRA_REPORT.md](../GETEFFECTIVEHEIGHT_PLUS4_UNIT_GHIDRA_REPORT.md)
- [../MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md](../MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md)

## Decoding Order

For new bridge investigations, use this order unless the bug points directly at a
later stage:

1. Establish cell setup from `01-assets-map-load-overlay`.
2. Confirm active layer state and zone records from `02-cell-state-layering-zones`.
3. Trace path entry and `Can_Enter_Cell` gates from `03-traversal-pathfinding-entry`.
4. Trace per-locomotor height and layer handling from `04-locomotion-height-tubes`.
5. If damage, repair, or cabhut is involved, switch to `05-damage-collapse-repair-cabhut`.
6. Verify player-visible output with `06-render-presentation-audio` and the relevant trace in `08-traces`.

## Organization Notes

- Files were moved here on 2026-05-24 from the formerly flat `docs/research`
  bridge cluster.
- General unit, pathfinding, chronominer, skirmish, and coordinate docs are not
  duplicated here. This folder indexes the bridge-relevant ones instead.
- `05-damage-collapse-repair-cabhut/bridge-repair-mechanic` preserves the
  existing per-symbol decode package for bridge repair and hut death.
