# CellClass+0x100 Hidden Occupancy Readers - Ghidra Research Report

**Address(es):** `0x00487E00`, `0x006FA2AE`, `0x005683C0`, `0x005687F0`, `0x005666C0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** live gamemd.exe readers of `CellClass+0x100` that are relevant to buildings and the hidden-occupancy counter written by building `CanHideThings` / `OccupyHeight` / `AddOccupy` / `RemoveOccupy`.
**Non-Scope:** re-investigation of AddOccupy/RemoveOccupy parsing, base foundation occupancy, full `UnitClass::Can_Enter_Cell`, full tactical renderer, and unrelated non-Cell structures that also have a `+0x100` field.
**Confidence:** High for the identified reader set and classifications; Medium for "no passability/targeting/selection reader" because that conclusion combines direct decompilation of the candidate paths with byte-pattern filtering rather than a whole-program semantic proof.
**Active in YR:** Yes for the AI/render hiding reader and writer-side maintenance readers; Conditional for map resize copy/restore.

## 1. Overview

`CellClass+0x100` is the dword hidden-occupancy counter maintained by building hidden-occupancy writers. The only semantic downstream reader found in standard object gameplay is `FUN_00487E00`, called from `TechnoClass__AI_Update`, and it drives the "Behind" marker/hidden-object visual path for `TechnoTypeClass+0x724 CanBeHidden`.

No direct building-relevant reader was found in the verified passability, targeting, selection, radar, or ordinary building body render paths. The other building-relevant reads are writer-side read-modify-write/underflow guards and a conditional map-resize copy/restore path.

## 2. Key Offsets

| Offset | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x100` | dword hidden-occupancy counter | writer evidence at `0x0056865F`, `0x0056872B`, `0x005687C2`, `0x00568A91`, `0x00568B73`; reader `0x00487E00` | Yes/Conditional depending on writer gate |
| `CellClass+0xE4` | ground object list, used by reader to detect buildings in the same cell | `FUN_00487E00 @ 0x00487E00` walks `cell+0xE4` | Yes |
| `TechnoTypeClass+0x724` | `CanBeHidden`, default true, parsed from `CanBeHidden=` | constructor `0x00710AF0`; parser `0x007121F2..0x00712202`; string `0x008444F8` | Yes |
| `TechnoClass+0x12C` | existing "Behind" anim/object pointer that is destroyed when the object is no longer hidden | `TechnoClass__AI_Update @ 0x006FA2C0..0x006FA2D3` | Yes |
| `RulesClass+0xB8` | `[General] Behind` AnimType pointer used when creating the behind marker | `FUN_0070F1D0`, corroborated by `RULESCLASS_FIELDS.csv:42` | Yes |

## 3. Reader Inventory And Classification

| Reader | What it reads | Classification | Active in YR | Evidence |
|---|---|---|---|---|
| `FUN_00487E00 @ 0x00487E00` | `Cell+0x100` twice: first as nonzero boolean, then special-cases exactly `1` when a building object is present in `cell+0xE4` | Behind-building hiding/occlusion | Yes. Called from `TechnoClass__AI_Update` for every techno that reaches the `CanBeHidden` gate; `CanBeHidden` defaults true and is parsed from INI. | decompile `0x00487E00`; assembly `0x00487E2A` and `0x00487E3E`; xref from `0x006FA2AE`; `CanBeHidden` parser `0x007121F2` |
| `TechnoClass__AI_Update @ 0x006FA2AE` | does not read `Cell+0x100` directly; calls `FUN_00487E00` after deriving current cell from render/location coords | Integration point for behind marker creation/destruction | Yes. Standard per-techno AI update path. Buildings are excluded as hidden subjects by `WhatAmI()==2` check after the helper, but buildings remain the counter writers. | decompile `0x006FA2AE..0x006FA2D3` |
| `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` | read-modify-write increments; remove-offset loop reads before decrementing if nonzero | Counter maintenance / underflow guard, not a downstream player-visible consumer | Conditional. Only for building objects, type supports multi-cell contents, and `BuildingType+0x1766 CanHideThings` is true. | assembly `0x0056865F`, `0x0056872B`, `0x005687C2..0x005687CD`; prior writer reports |
| `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0` | reads before decrementing height/add hidden cells if nonzero | Counter maintenance / underflow guard, not a downstream player-visible consumer | Conditional. Same building and `CanHideThings` gate. | assembly `0x00568A91..0x00568A9C`, `0x00568B73..0x00568B7E`; prior writer reports |
| `MapClass__Resize @ 0x005666C0` | copies/restores `Cell+0x100` as part of a full `CellClass` state snapshot | Another player-visible system only under map resize/editor/save-load-style relocation; not ordinary skirmish passability/rendering | Conditional. Live code, but not standard YR battle tick behavior. | decompile `0x005666C0`, direct copy at `0x00565EB5`/restore at `0x005666C7` |

## 4. `FUN_00487E00` Semantics

Pseudocode:

```text
if game active:
  for obj in cell.ground_object_list:
    if obj.WhatAmI() == building:
      if obj != null and cell.hidden_counter == 1:
        return false
      break
return cell.hidden_counter != 0
```

The exact `== 1` carve-out matters. If a cell has a building object in the ground content list and the hidden counter is exactly one, the helper reports not hidden. If the counter is greater than one, it still reports hidden. If no building is encountered, any nonzero hidden counter reports hidden.

Active in YR: Yes. Evidence: the helper has a single code xref from `TechnoClass__AI_Update @ 0x006FA2AE`, the caller first checks `TechnoType+0x724 CanBeHidden`, and `CanBeHidden` defaults true in `TechnoTypeClass__Constructor @ 0x00710AF0` then is parsed at `0x007121F2..0x00712202`.

## 5. Integration Point

`TechnoClass__AI_Update` computes the techno's current cell, calls `MapClass__Get_CellClass`, then checks:

1. `TechnoType+0x724 CanBeHidden` must be true.
2. `FUN_00487E00(cell)` must return true.
3. `WhatAmI()` must not be `2` after the helper returns true.

If any of those fail, existing `Techno+0x12C` is destroyed. If they pass, `FUN_0070F1D0` is called and may create/use the `[General] Behind` animation (`RulesClass+0xB8`) or draw the fallback marker path.

Classification: behind-building hiding/occlusion, not passability, targeting, selection, or ordinary building rendering. Buildings mainly affect this path as writers of the counter; buildings themselves are excluded as hidden subjects by the post-helper `WhatAmI()==2` branch.

Active in YR: Yes. Evidence: `TechnoClass__AI_Update @ 0x006FA2AE..0x006FA2D3`, `FUN_0070F1D0`, `RULESCLASS_FIELDS.csv:42`, `ini/rulesmd.ini` inherits/uses `[General] Behind` through RulesClass.

## 6. Negative Classifications

| System | Classification result | Active in YR | Evidence |
|---|---|---|---|
| Behind-building hiding/occlusion | Affected directly by `FUN_00487E00` and `TechnoClass__AI_Update` | Yes | `0x00487E00`, `0x006FA2AE`, `0x0070F1D0` |
| Passability/path blocking | No direct downstream `Cell+0x100` reader verified; central `UnitClass::Can_Enter_Cell` uses cell object lists and building-specific passability branches, not this counter | Yes path exists; `Cell+0x100` effect not found there | decompiled `0x0073F0A0`; prior `BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md` |
| Targeting | No material `Cell+0x100` reader found in the scoped static reader scan | No verified effect | byte-pattern scan for `+0x100` readers; no targeting candidate tied to `CellClass` |
| Selection/clicking | No material `Cell+0x100` reader found | No verified effect | scoped scan plus prior footprint consumer docs; no selection xref to `FUN_00487E00` |
| Ordinary building body rendering | Building draw reads cell height/lighting fields such as `Cell+0x10A`, not `Cell+0x100` | Yes draw path exists; `Cell+0x100` effect not found there | `BuildingClass_DrawBody @ 0x0043D290` |
| Radar/minimap | No material `Cell+0x100` reader found; radar color paths use terrain/overlay/object-list state | No verified effect | `CellClass__GetRadarColor @ 0x0047C060`; prior footprint consumer report |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct semantic reader `FUN_00487E00` | verified | `0x00487E00`, assembly `0x00487E2A`, `0x00487E3E` | none |
| Caller and player-visible effect | verified | `TechnoClass__AI_Update @ 0x006FA2AE..0x006FA2D3`, `FUN_0070F1D0` | exact visual asset frame behavior of `Behind` marker is outside this slot |
| `CanBeHidden` gate/default | verified | string `0x008444F8`, xref `0x007121F2`, constructor `0x00710AF0` | none |
| Writer-side read-modify-write guards | verified | `0x0056865F`, `0x0056872B`, `0x005687C2`, `0x00568A91`, `0x00568B73` | none for reader classification |
| Map resize copy/restore reader | touched-not-exhausted | `MapClass__Resize @ 0x005666C0`; copies `Cell+0x100` | full map-editor/save-load lifecycle not in scope |
| Passability candidate | verified negative for direct `Cell+0x100` use in central path | `UnitClass::Can_Enter_Cell @ 0x0073F0A0` | full passability remains covered by separate reports |
| Targeting/selection broad scan | touched-not-exhausted | byte-pattern scans for `+0x100`; no material `CellClass` reader candidate | whole-program semantic proof would require dynamic watchpoints or a decompiler export pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - What is the live downstream reader? -> `FUN_00487E00`, called from `TechnoClass__AI_Update`.` (evidence: `0x00487E00`, xref `0x006FA2AE`)
- `[RESOLVED] OQ-2 - Does the reader have edge cases? -> Yes: building present plus counter exactly `1` returns false; counter greater than `1` returns true.` (evidence: `0x00487E3E`, `0x00487E2A`)
- `[RESOLVED] OQ-3 - Is this standard YR-active or TS-only? -> YR-active; caller is standard `TechnoClass__AI_Update`; gate is `CanBeHidden`, default true, parsed by `TechnoTypeClass__ReadINI`.` (evidence: `0x006FA2AE`, `0x00710AF0`, `0x007121F2`)
- `[RESOLVED] OQ-4 - Does passability consume the counter? -> No direct read in the checked central unit cell-entry path; passability is object-list/building-branch driven in that path.` (evidence: `0x0073F0A0`)
- `[RESOLVED] OQ-5 - Do writer functions also read the counter? -> Yes, as read-modify-write increments and nonzero decrement guards; these are maintenance readers, not downstream consumers.` (evidence: `0x0056865F`, `0x005687C2`, `0x00568A91`, `0x00568B73`)
- `[RESOLVED] OQ-6 - Is there a live non-battle-tick reader? -> Yes, `MapClass__Resize` snapshots/restores the field with other `CellClass` fields.` (evidence: `0x005666C0`)
- `[DEFERRED] OQ-7 - Exact `Behind` animation art/frame behavior once `FUN_0070F1D0` fires.` (category: out-of-scope; reason: target is `Cell+0x100` reader classification, not the animation renderer)
- `[DEFERRED] OQ-8 - Whole-program dynamic confirmation that no obscure targeting/selection path reads the field through an aliased pointer.` (category: needs-runtime-debugger; reason: static byte-pattern/decompile scan found no material candidate, but dynamic read watchpoint validation was outside the read-only static workflow)

## 9. Negative Facts / Do Not Do

| Do not | Evidence | Active in YR |
|---|---|---|
| Do not use `AddOccupy` / `RemoveOccupy` cells as real building foundation, placement, selection, C4, or path blockers. They feed hidden occupancy only, while base foundation paths use `BuildingTypeClass+0xDFC` / vtable foundation lists. | Writers `0x005683C0`, `0x005687F0`; placement validators `0x00716150`, `0x0045EE70`; picker-equivalent Rust surfaces currently use foundation dimensions. | Yes for base foundation consumers; Conditional for hidden counter writers via `CanHideThings`. |
| Do not treat `CellClass+0x100 != 0` as a global passability or targeting blocker. The only verified semantic battle reader is the behind-building hiding helper. | `FUN_00487E00 @ 0x00487E00`; no direct read found in checked `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, targeting, or selection candidates. | Yes for the hiding reader; No verified effect for passability/targeting/selection. |
| Do not hide buildings themselves with the behind marker just because their own cells write the counter. The caller rejects `WhatAmI()==2` after the helper reports hidden. | `TechnoClass__AI_Update @ 0x006FA2AE..0x006FA2D3`. | Yes. |
| Do not reduce `FUN_00487E00` to a plain nonzero counter check. A building in `CellClass+0xE4` plus counter exactly `1` returns false; counter greater than `1` still returns true. | `FUN_00487E00 @ 0x00487E00`, branch sites `0x00487E2A`, `0x00487E3E`. | Yes. |
| Do not hardcode retail `BEHIND` as the only possible marker identity in engine logic. The reader path resolves through `[General] Behind` (`RulesClass+0xB8`); `BEHIND` is retail YR data. | `FUN_0070F1D0`; `RULESCLASS_FIELDS.csv:42`; `rulesmd.ini [General] Behind=BEHIND` as corroborating data. | Yes in retail YR; custom rules may replace or null it. |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `CellClass+0x100` is a hidden-object counter consumed by `FUN_00487E00` / `TechnoClass__AI_Update`, not a normal movement, placement, targeting, or selection footprint. | `0x00487E00`, `0x006FA2AE`; negative passability check at `0x0073F0A0`; placement reports `0x00716150`, `0x0045EE70`. Active in YR: Yes for hiding, no verified direct effect for the others. | Rust has helpers separating `building_base_foundation_cells` and `building_hidden_occupancy_cells`, but the compatibility alias `building_footprint_cells` still returns hidden occupancy and may keep older call sites risky. | `src/sim/production/production_tech.rs`; `src/sim/pathfinding/core.rs`; `src/app_init.rs`; `src/app_sim_tick.rs`; `src/sim/world/world_spawn.rs`; `src/app_entity_pick.rs`. | Keep base foundation/path/click/C4 blockers on `building_base_foundation_cells`; introduce a distinct hidden-counter state only for behind-object visibility. | GAREFN `AddOccupy1=-1,0` should not block movement or building click selection outside the 4x3 base foundation, but should contribute to hidden-object marker eligibility. Proposed test: `test_addoccupy_hidden_counter_does_not_expand_foundation_blockers`. | Conflating hidden occupancy with real footprint creates too-large blockers and wrong click/C4/placement behavior. |
| `FUN_00487E00` has a building/counter edge case: if the cell object list contains a building and the hidden counter is exactly `1`, it returns false; if the counter is greater than `1`, it returns true. | `FUN_00487E00 @ 0x00487E00`, branch evidence `0x00487E2A`, `0x00487E3E`. Active in YR: Yes. | No Rust hidden-counter reader or `CanBeHidden`/Behind consumer was found in the scanned surfaces. | Future sim/render bridge for hidden-object visibility; likely `app_sim_tick.rs` or render-side marker lifecycle plus sim cell state. | Implement the exact helper semantics, including the `== 1` carve-out, before creating/removing the behind marker. | A non-building techno in a cell with counter `1` and a building object in `Cell+0xE4` should not receive the marker; with counter `2` it should. Proposed test: `test_hidden_counter_building_cell_one_does_not_hide_but_two_does`. | A boolean `hidden_counter > 0` shortcut will over-show the marker on ordinary building cells. |
| The player-visible effect is behind-building marker/occlusion for non-building technos with `CanBeHidden=true`; buildings are excluded as hidden subjects. | `TechnoClass__AI_Update @ 0x006FA2AE..0x006FA2D3`; `TechnoTypeClass+0x724` constructor/parser `0x00710AF0`, `0x007121F2`; follow-up report `BEHIND_HIDDEN_OBJECT_VISUAL_PATH_GHIDRA_REPORT.md`. Active in YR: Yes. | Rust parses building hidden-occupancy art data, but no `CanBeHidden` marker lifecycle or `[General] Behind` render path was found by `rg "CanBeHidden|BEHIND|Behind=" src`. | `src/rules/ruleset.rs` / type rules for `CanBeHidden`; render/anim systems for `[General] Behind`; `app_selection_brackets.rs` only indirectly for draw ordering if marker layering interacts. | Resolve `CanBeHidden` and `[General] Behind`, attach one marker to eligible non-building technos while hidden, destroy it when gates fail; keep building subjects excluded. | Infantry standing behind a `CanHideThings` refinery hidden cell gets the `[General] Behind` marker; the refinery itself never gets one. Proposed test: `test_canbehidden_nonbuilding_gets_behind_marker_building_subject_does_not`. | Hardcoding only building render occlusion or hiding the unit sprite itself would mismatch the active YR marker path. |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`: replace deferred wording "exact downstream consumers of `Cell+0x100` deferred" and "Which exact passability sub-branch consumes `Cell+0x100`?" with: "Resolved by `CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`: the only verified semantic battle reader is `FUN_00487E00` via `TechnoClass__AI_Update` for the `CanBeHidden` / `[General] Behind` marker path; no direct passability, targeting, selection, or ordinary building-render reader was found in the scoped static scan. Writer-side reads are read-modify-write/underflow guards."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`: replace `Coverage Ledger` wording "`exact downstream Cell+0x100 readers are sibling slot 1`" and deferred `OQ-5` with: "Resolved by `CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`: placement validators do not consume `CellClass+0x100`; the verified downstream battle effect is behind-object marker eligibility for non-building technos, not placement acceptance."

## Sources

- Ghidra decompiled/read-only: `0x00487E00`, `0x006FA2AE`, `0x0070F1D0`, `0x005683C0`, `0x005687F0`, `0x005666C0`, `0x00712170`, `0x00710AF0`, `0x0073F0A0`, `0x0043D290`, `0x0047C060`.
- Ghidra assembly context: `0x00487E2A`, `0x00487E3E`, `0x0056865F`, `0x0056872B`, `0x005687C2`, `0x005687CD`, `0x00568A91`, `0x00568A9C`, `0x00568B73`, `0x00568B7E`.
- Prior reports: `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`; `BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`; `BUILDING_FOOTPRINT_CONSUMER_DISCREPANCY_AUDIT_GHIDRA_REPORT.md`; `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`.
- INI/docs: `ini/art.ini:41..43` comments; `ini/artmd.ini` retail `CanHideThings` / `OccupyHeight` / `AddOccupy` / `RemoveOccupy` examples; `RULESCLASS_FIELDS.csv:42`; `RULESCLASS_CONSTRUCTOR_DEFAULTS.csv:44`.
- Rust scan: `src/sim/production/production_tech.rs` hidden/base foundation helpers; `src/app_entity_pick.rs` foundation click path; `rg "CanBeHidden|BEHIND|Behind=" src` found no marker lifecycle.
