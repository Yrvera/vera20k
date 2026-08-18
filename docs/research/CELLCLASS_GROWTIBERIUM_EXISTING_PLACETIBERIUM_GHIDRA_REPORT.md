# CellClass GrowTiberium and Existing-Cell PlaceTiberium - Ghidra Research Report

**Address(es):** `0x00483710` (`CellClass::GrowTiberium`), `0x00487190` (`CellClass::PlaceTiberium`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard-YR active `GrowTiberium` and the existing-cell/grow branch of `PlaceTiberium`
**Non-Scope:** new-cell germination except contrast, full spread processor, map-load queue rebuild, full save/load queue state
**Confidence:** High
**Active in YR:** Yes. `TiberiumClass::GrowthProcessor @ 0x00722F00` calls `CellClass::GrowTiberium @ 0x00483710` for matching-type growth queue entries.

## 0. Working Notes

**Target question:** What exact cell mutation does GameMD perform when a growth queue entry grows existing ore through `CellClass::GrowTiberium` and `CellClass::PlaceTiberium`?

**Non-goals:** Do not redo new-cell TIBTRE placement except to contrast queue/dirty side effects. Do not implement Rust. Do not claim the full growth/spread queue model.

**Evidence needed to mark COMPLETE:** decompile and assembly evidence for `GrowTiberium`, `PlaceTiberium` branch selection, branch gates, density add/clamp, queue calls, dirty/radar calls, return values, RNG calls, and GrowthProcessor integration.

**Stop conditions:** stop after every branch in `GrowTiberium` and the existing-cell branch of `PlaceTiberium` is resolved or explicitly deferred; do not expand into unrelated callers.

## 1. Overview

`CellClass::GrowTiberium` is a gated wrapper. If growth is enabled, the cell has a valid tiberium overlay, the cell is flat, density is below `MaxDensity - 1`, and the type has non-negative `GrowthPercentage`, it calls `CellClass::PlaceTiberium(current_type, 1)`.

The existing-cell branch of `PlaceTiberium` repeats the important gates, adds the requested density amount to `CellClass+0x11E`, clamps to `MaxDensity - 1`, dirties the tactical screen rect, calls `TiberiumClass::AddToSpreadQueue`, and returns `1`. It consumes no RNG itself and does not call `RadarClass::MarkTerrainDirty`.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type | Meaning | Evidence |
|---|---:|---|---|---|
| `ScenarioClass` | `+0x34A6` | byte bool | global tiberium growth enabled gate | `0x00483718..0x00483720`, `0x0048737C..0x0048738A` |
| `CellClass` | `+0x44` | overlay index/source | passed to `OverlayToTiberiumIndex` | `0x00483722..0x00483725`, `0x00487390..0x00487393` |
| `CellClass` | `+0x11C` | byte | slope index; must be zero for existing-cell growth | `0x00483738..0x00483740`, `0x004873A7..0x004873B2` |
| `CellClass` | `+0x11E` | byte | overlay density/data | `0x00483748..0x0048374D`, `0x004873F6..0x00487418` |
| `CellClass` | `+0x24` | coord | queue/dirty coordinate | `0x00487291`, `0x00487606..0x0048760A` |
| `TiberiumClass` | `+0x98` | int/type id | current queue processor type id | `0x0072300E` |
| `TiberiumClass` | `+0xB0` | double | `GrowthPercentage`; existing growth requires `>= 0.0` | `0x004873CF..0x004873E0` |
| `TiberiumClass` | `+0xE4` | int | `MaxDensity`; stock value 12, effective max data 11 | `0x004871B6`, `0x00487404..0x00487418` |

## 3. Core Logic

### 3.1 `GrowTiberium @ 0x00483710`

Pseudocode from decompile plus assembly:

```text
if ScenarioClass+0x34A6 == 0: return 0
type = OverlayToTiberiumIndex(cell->overlay)
if type == -1: return 0
tib = TiberiumClass_Array[type]
if cell->slope != 0: return 0
if cell->OverlayData >= tib->MaxDensity - 1: return 0
if tib->GrowthPercentage < 0.0: return 0
type = OverlayToTiberiumIndex(cell->overlay)
return cell->PlaceTiberium(type, 1)
```

Assembly evidence:

- `0x00483718..0x00483720` reads `ScenarioClass+0x34A6`, `TEST`, `JZ` failure.
- `0x00483722..0x0048372D` calls `OverlayToTiberiumIndex` and rejects `-1`.
- `0x00483738..0x00483740` tests `CellClass+0x11C`.
- `0x00483748..0x00483751` compares `OverlayData` against `MaxDensity - 1`.
- `0x00483761..0x00483766` rejects negative `GrowthPercentage`.
- `0x00483768..0x00483775` calls `OverlayToTiberiumIndex` again, then pushes `1`, pushes current type, sets `ECX=this`, and calls `PlaceTiberium`.

The second `OverlayToTiberiumIndex` call means the argument passed to `PlaceTiberium` is derived immediately before the call, not cached from before the floating-point gate.

### 3.2 Existing-Cell Branch Selection in `PlaceTiberium @ 0x00487190`

Entry loads `TiberiumClass_Array[param_2]` and immediately rejects when `param_3 >= TiberiumClass+0xE4`. Assembly `0x004871AF..0x004871BC` loads the class pointer and compares density amount against `MaxDensity`.

`PlaceTiberium` then calls `CanPlaceTiberium(tib_ptr)`. If that returns false, control enters the existing-cell branch at `0x0048737C`. Existing-cell growth is therefore not "grow any blocked cell"; it is "placement failed, then existing tiberium-specific gates passed."

### 3.3 Existing-Cell Branch Gates

The existing-cell branch returns `0` unless all of these pass:

1. `ScenarioClass+0x34A6 != 0` (`0x0048737C..0x0048738A`).
2. `OverlayToTiberiumIndex(cell->overlay) != -1` (`0x00487390..0x0048739B`).
3. `CellClass+0x11C == 0` (`0x004873A7..0x004873B2`).
4. `OverlayData < overlay_type.MaxDensity - 1` (`0x004873B8..0x004873C9`).
5. `GrowthPercentage >= 0.0` (`0x004873CF..0x004873E0`).
6. Recomputed overlay tiberium index equals requested `param_2` (`0x004873E6..0x004873F0`).

The type-match gate is after the `GrowthPercentage` comparison and after a second overlay-to-type lookup.

### 3.4 Density Mutation, Clamp, Return

Existing-cell mutation order is:

1. Read byte `CellClass+0x11E` into `AL`.
2. Add low byte of `param_3` (`BL`) to `AL`.
3. Write the raw byte sum back to `CellClass+0x11E`.
4. Load `MaxDensity`, compute `MaxDensity - 1`.
5. If unsigned-masked `AL` is not less than `MaxDensity - 1`, replace it with `MaxDensity - 1`.
6. Write the final byte to `CellClass+0x11E`.

Assembly evidence:

- `0x004873F6`: `MOV AL, byte ptr [ESI + 0x11e]`
- `0x004873FC`: `ADD AL, BL`
- `0x004873FE`: first write to `[ESI + 0x11e]`
- `0x00487404..0x00487412`: load max density and compare against `MaxDensity - 1`
- `0x00487414..0x00487418`: clamp branch and final write

For normal growth, `GrowTiberium` always passes `param_3 = 1`, so the practical result is `OverlayData += 1` up to `11` when stock `MaxDensity = 12`. For other callers of `PlaceTiberium`, the branch adds `param_3` and clamps to `MaxDensity - 1`.

### 3.5 Queue, Dirty, RNG, and Radar Side Effects

Existing-cell `PlaceTiberium`:

- calls tactical dirty rectangle helpers after the density write;
- calls `TiberiumClass::AddToSpreadQueue(cell_coord)`;
- returns `1`;
- does not call `TiberiumClass::AddToGrowthQueue`;
- does not call `RadarClass::MarkTerrainDirty`;
- does not consume RNG directly.

Assembly evidence:

- `0x0048741E..0x00487602` performs rect computation/dirty helper calls after the density write.
- `0x00487606..0x0048760A` passes `cell+0x24` and calls `0x00722AF0` (`AddToSpreadQueue`).
- `0x0048760F..0x0048761B` returns success.
- `RadarClass::MarkTerrainDirty @ 0x006551C0` appears in the new-cell branch before `0x00487379`; there is no matching call in the existing-cell branch.

`AddToSpreadQueue` may consume RNG only if its own `CanSpreadTiberium` and membership gates pass. Its decompile shows `Random::Next` after those gates and after capacity rebuild check. That RNG draw belongs to `AddToSpreadQueue`, not to `GrowTiberium` or the density mutation.

### 3.6 GrowthProcessor Integration

`TiberiumClass::GrowthProcessor @ 0x00722F00` pops an entry, gets the cell, and compares `CellClass::GetTiberiumType()` against `TiberiumClass+0x98`. Only matching entries call `GrowTiberium`.

After the call, GrowthProcessor checks `CellClass+0x11E`:

- if `< 0x0B`, it reinserts into the growth heap, sets the growth bitmap byte, and calls `AddToSpreadQueue`;
- otherwise it clears the growth bitmap byte.

Assembly evidence:

- `0x0072300E..0x0072301C`: compare current cell type to processor type and call `0x00483710`.
- `0x00723021..0x00723028`: compare post-growth `OverlayData` with `0x0B`.
- `0x0072302E..0x00723113`: reinsert growth entry and call `AddToSpreadQueue`.
- `0x0072311E..0x0072312B`: clear growth bitmap on full density.

This means a successful `GrowTiberium` can feed spread twice in the `< 11` case: once inside existing-cell `PlaceTiberium`, and once again in `GrowthProcessor` after reinsert. Both calls are real; `AddToSpreadQueue` has its own membership guard, so the second call may no-op if the first inserted the cell.

## 4. INI Keys

| Key / field | Default / stock effect | Binary use in this slice |
|---|---|---|
| `GrowthPercentage` on tiberium type | Riparius positive, Cruentus zero in stock YR reports | existing growth requires `>= 0.0`; GrowthProcessor requires `> 0.0` before processing |
| `MaxDensity` on tiberium type | stock reports show 12 | entry density guard and existing-cell clamp to `MaxDensity - 1` |
| scenario growth enabled flag | standard YR active when scenario permits growth | `ScenarioClass+0x34A6` gates both `GrowTiberium` and existing-cell `PlaceTiberium` |

## 5. Integration Points

| Function | Relationship | Evidence |
|---|---|---|
| `TiberiumClass::GrowthProcessor @ 0x00722F00` | live owner calling `GrowTiberium` | `0x0072301C` |
| `CellClass::GrowTiberium @ 0x00483710` | calls `PlaceTiberium(type, 1)` | `0x00483770..0x00483775` |
| `CellClass::PlaceTiberium @ 0x00487190` | existing-cell density mutation and spread feed | decompile and branch assembly |
| `TiberiumClass::AddToSpreadQueue @ 0x00722AF0` | called after existing-cell density mutation | `0x00487606..0x0048760A` |

## 6. Current Rust Implementation Status

Current Rust still uses a scan/reservoir growth processor in `src/sim/ore_growth.rs::tick_ore_growth`, with `ResourceNode.remaining` as the main stock representation. It grows by stock amounts and updates `OverlayGrid.overlay_data` from stock-derived density. This does not yet model the GameMD `GrowTiberium -> PlaceTiberium(type,1) -> post-growth queue branch` primitive.

Rust has partial native-shaped queue storage in `OreGrowthState::growth_queue` and `spread_queue`, and TIBTRE new-cell placement can enqueue growth queue entries, but those queue entries are not yet the live processor. `src/sim/terrain_spawn.rs::place_tiberium_empty` still inserts `ResourceNode` before overlay/grid and before growth queue enqueue, which is new-cell scope and not corrected by this report.

`src/sim/tiberium/mod.rs::reduce_tiberium` handles depletion and spread reseed, but this report does not audit reduction.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `GrowTiberium` entry gates | verified | `0x00483710` decompile; `0x00483718..0x00483775` assembly | none |
| `GrowTiberium` argument passing | verified | `0x00483770..0x00483775` pushes `1`, type, then calls `0x00487190` | none |
| `PlaceTiberium` existing branch gates | verified | `0x0048737C..0x004873F0` assembly | none |
| Existing-cell density add/clamp | verified | `0x004873F6..0x00487418` assembly | none |
| Existing-cell dirty and spread queue side effects | verified | `0x0048741E..0x0048761B` assembly | exact dirty rectangle helper internals out-of-scope |
| New-cell germination branch | touched-not-exhausted | `0x004871D0..0x00487379`; prior TIBTRE reports | out-of-scope except contrast |
| `AddToSpreadQueue` internals | touched-not-exhausted | `0x00722AF0` decompile | slot 5 owns full spread processor/queue audit |
| Rust queue processor | touched-not-exhausted | `src/sim/ore_growth.rs` scan/reservoir implementation | future implementation work |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is `GrowTiberium` active in standard YR? -> Yes, GrowthProcessor calls it for matching-type entries.` (evidence: `0x0072300E..0x0072301C`)
- `[RESOLVED] OQ2 - Does `GrowTiberium` mutate density itself? -> No; it gates and then calls `PlaceTiberium(type, 1)`.` (evidence: `0x00483768..0x00483775`)
- `[RESOLVED] OQ3 - What is the increment amount for growth? -> `1` density level from `PUSH 0x1`.` (evidence: `0x00483770`)
- `[RESOLVED] OQ4 - What is the max/clamp? -> Existing-cell branch clamps to `MaxDensity - 1`; stock max 12 means density 11.` (evidence: `0x00487404..0x00487418`)
- `[RESOLVED] OQ5 - Does existing-cell branch call growth queue? -> No, it calls spread queue only.` (evidence: `0x00487606..0x0048760A`)
- `[RESOLVED] OQ6 - Does existing-cell branch mark radar dirty? -> No; radar dirty call is in the new-cell branch only.` (evidence: `0x00487368`; no branch-local call before `0x0048761B`)
- `[RESOLVED] OQ7 - Does existing-cell branch consume RNG? -> No direct RNG; only downstream `AddToSpreadQueue` may consume after its gates.` (evidence: `0x0048737C..0x0048761B`, `0x00722AF0`)
- `[RESOLVED] OQ8 - Does `GrowthPercentage=0` block existing-cell `PlaceTiberium`? -> No; branch rejects only `< 0.0`, but GrowthProcessor itself exits for `<= 0.0`.` (evidence: `0x004873CF..0x004873E0`, `0x00722F00` decompile)
- `[RESOLVED] OQ9 - Does slope block existing-cell growth? -> Yes; `CellClass+0x11C != 0` returns failure.` (evidence: `0x00483738..0x00483740`, `0x004873A7..0x004873B2`)
- `[RESOLVED] OQ10 - Does GrowTiberium cache type before all gates? -> It recomputes type immediately before `PlaceTiberium`.` (evidence: `0x00483768..0x00483775`)
- `[RESOLVED] OQ11 - How should GrowthProcessor branch after the call? -> It reads post-growth `OverlayData`; `< 11` reinserts growth and calls spread queue, otherwise clears growth bitmap.` (evidence: `0x00723021..0x0072312B`)
- `[RESOLVED] OQ12 - Are dirty rectangle helper internals required for this slice? -> No; this slice only needs the call ordering relative to density and queue writes.` (evidence: `0x0048741E..0x0048760A`)
- `[DEFERRED] OQ13 - Exact tactical dirty rectangle geometry` (category: `out-of-scope`; reason: not required for growth queue/cell mutation primitive; next-step-if-pursued: trace `FUN_0047ff80`, `FUN_0047fb90`, `FUN_00487ee0`)
- `[DEFERRED] OQ14 - Full `AddToSpreadQueue` duplicate/RNG semantics` (category: `out-of-scope`; reason: sibling swarm slot owns spread processor/queue certainty; next-step-if-pursued: use spread processor report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Growth queue entries call `GrowTiberium`, which gates and calls `PlaceTiberium(type, 1)` | `0x0072301C`, `0x00483770..0x00483775` | missing; Rust scan grows stock directly | `src/sim/ore_growth.rs` | centralize existing-cell growth through a GameMD-shaped byte/data primitive | `growth_processor_calls_existing_place_tiberium_density_one` | do not mutate `ResourceNode.remaining` first and derive overlay byte later |
| Existing-cell `PlaceTiberium` adds requested density, writes raw sum, clamps to `MaxDensity - 1`, writes final byte | `0x004873F6..0x00487418` | mismatch/unchecked; Rust stock model skips raw byte write order | `src/sim/ore_growth.rs`, `src/sim/overlay_grid.rs`, resource bridge | overlay data byte is authoritative for density mutation and reaches 11 max | `existing_place_tiberium_adds_one_and_clamps_at_eleven` | do not clamp at 12 or branch on stock amount before byte mutation |
| Successful existing-cell placement calls `AddToSpreadQueue`; GrowthProcessor also calls `AddToSpreadQueue` after reinserting when post-growth data `< 11` | `0x00487606..0x0048760A`, `0x00723110..0x00723113` | partial queue storage; live processor not native | `src/sim/ore_growth.rs` | preserve both callsites and rely on spread queue membership guard for duplicate suppression | `growth_processor_existing_cell_feeds_spread_queue_with_membership_guard` | do not collapse the two calls into one unless binary-equivalence is proven |

## 10. Negative Facts / Do Not Do

- Do not treat `GrowthPercentage=0` as failing the existing-cell `PlaceTiberium` branch; the branch checks `< 0.0`, while GrowthProcessor has the stricter `<= 0.0` gate.
- Do not call `RadarClass::MarkTerrainDirty` for existing-cell growth; that call is new-cell branch only.
- Do not add growth queue entries from existing-cell `PlaceTiberium`; it calls `AddToSpreadQueue`, not `AddToGrowthQueue`.
- Do not treat queue priority as a wake-up timestamp for this primitive; GrowthProcessor pops first and only then calls `GrowTiberium`.
- Do not implement growth as stock/resource math first. The binary mutates `OverlayData` byte first and the queue processor branches on that byte.

## Stale Docs / Follow-up Docs

- `docs/research/CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`: wording that says Branch A has a `GrowthPercentage > 0` style gate should be replaced with: "The existing-cell branch rejects only when `GrowthPercentage < 0.0`; the live GrowthProcessor separately exits for `GrowthPercentage <= 0.0` before calling `GrowTiberium`."
- `docs/research/TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md`: append: "After a matching-type entry grows, `GrowTiberium` reaches `PlaceTiberium(type,1)`, whose existing-cell branch already calls `AddToSpreadQueue`; GrowthProcessor then calls `AddToSpreadQueue` again when post-growth `OverlayData < 11`, relying on the spread queue membership guard."

## Sources

- Ghidra decompiled: `0x00483710`, `0x00487190`, `0x00722F00`, `0x00722AF0`, `0x007235A0`.
- Ghidra assembly contexts: `0x00483718..0x00483775`, `0x0048737C..0x0048761B`, `0x0072300E..0x0072312B`.
- Existing docs used as maps only: `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md`.
- Rust files scanned: `src/sim/ore_growth.rs`, `src/sim/terrain_spawn.rs`, `src/sim/tiberium/mod.rs`.
