# TIBTRE SpreadTiberium Force/Type Gate - Ghidra Research Report

**Address(es):** `0x0071C730` (`TerrainClass::AI`), `0x00483780` (`CellClass::SpreadTiberium`), `0x005FDD20` (`CellClass::OverlayToTiberiumIndex`), `0x00721D10` (`TiberiumClass::ReadINI_All`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `TerrainClass::AI` TIBTRE call into `CellClass::SpreadTiberium`: argument order/value, force=true behavior, `TiberiumSpreads` bypass, source-cell type selection/default, and the type-0-vs-type-1 report conflict.  
**Non-Scope:** `AnimClass` meteor/crystal bouncer tiberium, full `CanPlaceTiberium` rejection-gate audit, full `PlaceTiberium` overlay/queue side effects, and terrain-object lifecycle outside the source-cell type implication.  
**Confidence:** High for the claimed slice.  
**Active in YR:** Yes. Standard YR TIBTRE terrain types set `SpawnsTiberium=yes`, `IsAnimated=yes`, and `AnimationProbability=.003` in `ini/rulesmd.ini`; `TerrainClass::AI` is the live terrain object tick path.

## 0. Investigation Setup

- Target question: Verify `CellClass::SpreadTiberium` when called from `TerrainClass::AI` for TIBTRE: exact argument order/value, force=true behavior, `TiberiumSpreads` bypass, source-cell tiberium type selection/default, and the type 0 vs hardcoded 1/Riparius discrepancy.
- Non-goals: No AnimClass meteor/crystal bouncer spread, no full `CanAcceptTiberium` drain beyond the force/type gate, no Rust edits.
- Evidence needed to mark COMPLETE: decompile plus assembly context for the TerrainClass call and SpreadTiberium branches; INI/default evidence for `[Tiberiums]`; current Rust scan for affected surfaces.
- Stop conditions: every force/type/gate question resolved or explicitly deferred; only this report and the shared claims file written.

## 1. Overview

TIBTRE terrain spawning calls `CellClass::SpreadTiberium(force=true)` from `TerrainClass::AI`. The call-site literal `1` is not a tiberium type. It is the single stack argument consumed by `SpreadTiberium` as the force flag.

With force=true, `SpreadTiberium` bypasses the `SpecialFlags.TiberiumSpreads` bit check and still derives the tiberium type from the source cell overlay. If the source cell has no tiberium overlay, the forced path defaults the tiberium type index to `0`, which is stock `[Tiberiums] 0=Riparius`. Active in YR: Yes, through stock TIBTRE01/02/03 terrain objects.

## 2. Key Offsets / Fields

| Field | Offset / value | Meaning in this slice | Active in YR |
|---|---:|---|---|
| `TerrainClass + 0xC8` | type pointer | TerrainTypeClass pointer used by AI tick | Yes |
| `TerrainTypeClass + 0x2B1` | bool | `SpawnsTiberium`; required before midpoint spawn call | Yes, TIBTRE sets it |
| `TerrainTypeClass + 0x2B3` | bool | `IsAnimated`; required both to start animation and to spawn at midpoint | Yes, TIBTRE sets it |
| `CellClass + 0x44` | int | source cell overlay type index, passed to `OverlayToTiberiumIndex` | Yes |
| `CellClass + 0x11C` | byte | slope/flat byte; only relevant to non-force preflight and later placement | Yes |
| `CellClass + 0x11E` | byte | overlay data/density; only relevant to non-force preflight and later placement | Yes |
| `Scenario/SpecialFlags bit 0x80` | bit 7 | `TiberiumSpreads` gate for non-forced spread | Conditional; bypassed by TIBTRE |
| `TiberiumClass + 0x98` | int | type array index returned by `OverlayToTiberiumIndex` | Yes |
| `TiberiumClass + 0xE0` | ptr | image overlay type used to recognize overlay ranges | Yes |
| `TiberiumClass + 0xE8/+0xEC` | int | flat/slope image counts used by overlay range recognition | Yes |

## 3. Core Logic

### TerrainClass call-site

Verified behavior -> `TerrainClass::AI` checks `type.SpawnsTiberium` and `type.IsAnimated`, waits for current terrain animation frame to equal `image_frame_count / 2`, resets the animation state, resolves the terrain object's own cell, then calls `CellClass::SpreadTiberium` with stack argument `1`.

Evidence -> decompile `0x0071C730`; assembly context `0x0071C84D..0x0071C8D0` shows:

- `CMP byte ptr [ECX + 0x2b1], BL`
- `CMP byte ptr [ECX + 0x2b3], BL`
- midpoint compare at `0x0071C871..0x0071C87C`
- `PUSH 0x1` at `0x0071C8A3`
- `CALL 0x00565730` to get the cell
- `MOV ECX,EAX; CALL 0x00483780` at `0x0071C8CE..0x0071C8D0`

Active in YR: Yes. TIBTRE types in stock `rulesmd.ini` satisfy both flags.

### SpreadTiberium force gate

Verified behavior -> `CellClass::SpreadTiberium` reads its only explicit argument as a byte force flag. If the flag is nonzero, execution jumps over the non-force `TiberiumSpreads` preflight.

Evidence -> decompile `0x00483780`; assembly context `0x00483784..0x00483799` shows `MOV BL, byte ptr [ESP + 0x14]`, `TEST BL,BL`, then `JNZ 0x004837EB`. The skipped block starts with `MOV EAX,[0x00A8B230]` and `TEST byte ptr [EAX],0x80`, then early-outs if the bit is clear. `FUN_006B8B30` writes that same bit as `TiberiumSpreads`, and `FUN_006B8CA0` reads `[SpecialFlags] TiberiumSpreads` into bit 7 in scenario/map contexts.

Active in YR: Yes for normal spread when force=false; bypassed for TIBTRE because the force argument is `1`.

### Type selection/default

Verified behavior -> After the force/non-force preflight, `SpreadTiberium` calls `OverlayToTiberiumIndex` on the source cell overlay. If force=false and the helper returns `-1`, it returns false. If force=true and the helper returns `-1`, it writes local tiberium type index `0`. It then selects `g_TiberiumClass_Array[type_index]`, iterates up to 8 adjacent cells from a random start direction, and calls `PlaceTiberium(type_index, 3)` on the first accepted cell.

Evidence -> decompile `0x00483780`; assembly context `0x004837EB..0x0048381D` shows the second `OverlayToTiberiumIndex` call, `TEST BL,BL`, the forced `CMP EAX,-1` branch, and `MOV dword ptr [ESP + 0x20],0x0` when forced with no source overlay type. Assembly context `0x004838BC..0x004838C5` shows `PUSH 0x3`, `PUSH EDX`, `MOV ECX,ESI`, `CALL 0x00487190`, which is `PlaceTiberium(type_index, 3)`.

Active in YR: Yes. TIBTRE cells commonly have no ore overlay after terrain placement, so the forced default-to-0 branch is the relevant stock path.

### OverlayToTiberiumIndex behavior

Verified behavior -> `OverlayToTiberiumIndex` returns `-1` if the cell has no overlay or the overlay type does not have the tiberium/ore overlay flag at `OverlayTypeClass + 0x2A9`. If the overlay is in a TiberiumClass image range, it returns that TiberiumClass array index from `+0x98`. If the overlay is flagged tiberium but not actually in any registered range, it logs and returns `0`.

Evidence -> decompile `0x005FDD20`, including its `param_1 == -1` early return, `OverlayTypeClass + 0x2A9` gate, loop over `g_TiberiumClass_Array`, range checks against `TiberiumClass +0xE0/+0xE8/+0xEC`, and fallback return `0`.

Active in YR: Yes. This helper is directly called by `SpreadTiberium`.

## 4. INI Keys

| INI key / section | Stock YR value | Binary reader evidence | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `[TIBTRE01..03] SpawnsTiberium` | `yes` | `TerrainTypeClass::ReadINI @ 0x0071DEA0` in prior reports; live AI reads `+0x2B1` at `0x0071C853` | Enables TIBTRE midpoint spawn path | Yes |
| `[TIBTRE01..03] IsAnimated` | `yes` | `TerrainTypeClass::ReadINI @ 0x0071DEA0`; live AI reads `+0x2B3` at `0x0071C745` and `0x0071C85B` | Required for animation and spawn path | Yes |
| `[SpecialFlags] TiberiumSpreads` | map/scenario override; normal spread gate | `FUN_006B8B30` writes bit 7; `FUN_006B8CA0` reads bit 7 from `[SpecialFlags]` | Gates non-forced spread only | Conditional; bypassed by TIBTRE |
| `[General] TiberiumSpreads` | `yes` in `rulesmd.ini` | current Rust reads it at `src/rules/ruleset.rs:898`; binary slice here only proves scenario/special flag bit usage | Background default for normal spread, not a TIBTRE gate | Conditional |
| `[Tiberiums] 0` | `Riparius` | `TiberiumClass::ReadINI_All @ 0x00721D10` iterates `[Tiberiums]`; stock `rulesmd.ini` has `0=Riparius` | Forced no-overlay source defaults to type index 0 | Yes |
| `[Tiberiums] 1` | `Cruentus` | same as above; stock `rulesmd.ini` has `1=Cruentus` | Refutes "type 1 = Riparius" wording | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `TerrainClass::AI @ 0x0071C730` | owner tick; invokes `SpreadTiberium(1)` at animation midpoint | decompile plus call assembly `0x0071C8A3..0x0071C8D0` | Yes |
| `CellClass::SpreadTiberium @ 0x00483780` | force/type/gate owner for adjacent spawn | decompile plus branch assembly `0x00483784..0x004838C5`; xref count 2 | Yes |
| `TiberiumClass::SpreadProcessor @ 0x00722440` | other caller; calls `SpreadTiberium(0)` for global spread | decompile shows call with argument 0 | Yes, but not TIBTRE |
| `CellClass::OverlayToTiberiumIndex @ 0x005FDD20` | source-cell overlay to tiberium type index | decompile | Yes |
| `CellClass::PlaceTiberium @ 0x00487190` | target-cell placement after force/type selection | decompile plus spread call assembly | Yes |

## 6. Current Rust Implementation Status

Current Rust surface:

- `src/sim/terrain_spawn.rs:81` ticks terrain spawners and does not read `OreGrowthConfig.spreads`; this matches the verified force=true bypass for the `TiberiumSpreads` flag.
- `src/sim/terrain_spawn.rs:146` calls an additive placement helper after choosing an accepted adjacent cell.
- `src/sim/terrain_spawn.rs:213` always inserts `ResourceType::Ore`, which matches the default no-source-overlay TIBTRE path because stock type index `0` is Riparius/ore.
- `src/sim/terrain_spawn.rs:248` resolves `default_ore_overlay_id` as the first overlay name starting with `TIB`; this is an approximation of binary `TiberiumClass[0].Image` overlay selection and should be replaced or backed by real TiberiumClass image metadata once that exists.
- `src/sim/ore_growth.rs:82` applies normal `TiberiumSpreads` to the global ore spread config, which is correct for the `TiberiumClass::SpreadProcessor` force=false path but must not be shared as a gate for TIBTRE.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TerrainClass::AI` argument value/order | verified | `0x0071C8A3..0x0071C8D0` | none |
| `SpreadTiberium` force flag stack read | verified | `0x00483784..0x0048378F` | none |
| `TiberiumSpreads` non-force gate and bypass | verified | `0x0048378D..0x00483799`, `FUN_006B8B30`, `FUN_006B8CA0` | none |
| `SpreadTiberium` type source from source-cell overlay | verified | `0x004837EB..0x0048381D`, `0x005FDD20` | none |
| force=true no-overlay default | verified | `0x0048380C..0x00483819` | none |
| `PlaceTiberium(type,3)` argument order | verified | `0x004838BC..0x004838C5`, `0x00487190` | none |
| stock type 0/type 1 names | verified | `0x00721D10`, `ini/rulesmd.ini [Tiberiums]` | none |
| `CanPlaceTiberium` full rejection gates | touched-not-exhausted | `0x004838E0` | slot 3 owns this in the parent swarm |
| `PlaceTiberium` full overlay/queue side effects | touched-not-exhausted | `0x00487190` | slot 4 owns this in the parent swarm |
| `AnimClass` meteor/crystal bouncer tiberium | deferred | prior `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md` | out-of-scope for this slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the `1` passed by `TerrainClass::AI` a type or force flag? -> It is the single stack argument to `SpreadTiberium`, read as a byte force flag.` (evidence: `0x0071C8A3..0x0071C8D0`, `0x00483784`)
- `[RESOLVED] OQ-2 - Does force=true bypass `TiberiumSpreads`? -> Yes; nonzero force jumps over the bit-7 test.` (evidence: `0x0048378D..0x00483799`)
- `[RESOLVED] OQ-3 - What is the source of the tiberium type for TIBTRE spread? -> Source cell overlay through `OverlayToTiberiumIndex`; no explicit type is passed by TerrainClass.` (evidence: `0x004837EB`, `0x005FDD20`)
- `[RESOLVED] OQ-4 - What happens if the TIBTRE source cell has no ore overlay? -> force=true defaults `type_index=0`.` (evidence: `0x0048380C..0x00483819`)
- `[RESOLVED] OQ-5 - Which stock YR type is index 0? -> `Riparius`.` (evidence: `TiberiumClass::ReadINI_All @ 0x00721D10`, `ini/rulesmd.ini [Tiberiums] 0=Riparius`)
- `[RESOLVED] OQ-6 - Which stock YR type is index 1? -> `Cruentus`, not Riparius.` (evidence: `ini/rulesmd.ini [Tiberiums] 1=Cruentus`)
- `[RESOLVED] OQ-7 - Does `SpreadTiberium` pass density 3 to placement? -> Yes, `PlaceTiberium(type_index, 3)`.` (evidence: `0x004838BC..0x004838C5`)
- `[RESOLVED] OQ-8 - Is the TIBTRE path live in standard YR? -> Yes, stock TIBTRE has `SpawnsTiberium=yes` and `IsAnimated=yes`; `TerrainClass::AI` reads both before the call.` (evidence: `0x0071C853..0x0071C861`, `ini/rulesmd.ini [TIBTRE01..03]`)
- `[RESOLVED] OQ-9 - Is the normal global spread caller distinct? -> Yes, `TiberiumClass::SpreadProcessor` calls `SpreadTiberium(0)`.` (evidence: `0x00722440` decompile; xref count 2 for `0x00483780`)
- `[RESOLVED] OQ-10 - Does current Rust incorrectly gate TIBTRE on `TiberiumSpreads`? -> No direct gate observed in `terrain_spawn.rs`; it is separate from `ore_growth.rs`.` (evidence: `src/sim/terrain_spawn.rs:81`, `src/sim/ore_growth.rs:82`)
- `[DEFERRED] OQ-11 - Are all `CanPlaceTiberium` target-cell rejection gates represented in Rust?` (category: `out-of-scope`; reason: parent slot 3 owns this exact target; next-step-if-pursued: use `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES` report)
- `[DEFERRED] OQ-12 - Are all `PlaceTiberium` overlay/queue/radar effects represented in Rust?` (category: `out-of-scope`; reason: parent slot 4 owns this exact target; next-step-if-pursued: use `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS` report)

## 9. Negative Facts / Do Not Do

- Do not implement `TerrainClass::AI` as passing hardcoded tiberium type `1`. The call-site `1` is force=true. Active in YR: Yes; evidence `0x0071C8A3..0x0071C8D0` and `0x00483784`.
- Do not document stock type `1` as Riparius. Stock `[Tiberiums] 1=Cruentus`; Riparius is type `0`. Active in YR: Yes; evidence `ini/rulesmd.ini [Tiberiums]` plus `TiberiumClass::ReadINI_All @ 0x00721D10`.
- Do not gate TIBTRE terrain spawning on Rust `OreGrowthConfig.spreads` / `TiberiumSpreads`. That gate applies to `SpreadTiberium(0)` normal spread, not the forced TIBTRE call. Active in YR: Yes/Conditional; evidence `0x0048378D..0x00483799`.
- Do not infer the target tiberium type from the terrain object's name (`TIBTRE*`). The binary derives it from the source cell overlay, falling back to type 0 only when that overlay does not identify a type. Active in YR: Yes; evidence `0x004837EB..0x00483819`.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| TIBTRE passes `force=true`; `TiberiumSpreads` is bypassed | `0x0071C8A3..0x0071C8D0`, `0x0048378D..0x00483799` | none observed for `terrain_spawn.rs`; normal `ore_growth.rs` still uses spreads | `src/sim/terrain_spawn.rs:81`, `src/sim/ore_growth.rs:82` | Keep terrain spawner spawning independent of `OreGrowthConfig.spreads` / map `TiberiumSpreads=no` | `tibtre_force_spawn_ignores_tiberium_spreads_false`: configure normal ore growth spreads=false, force a TIBTRE roll, assert an adjacent ore node is still placed | Do not reuse `OreGrowthConfig.spreads` as a global tiberium-spawn gate |
| No-source-overlay TIBTRE defaults to type index 0, stock Riparius/ore | `0x0048380C..0x00483819`, `0x00721D10`, `ini/rulesmd.ini [Tiberiums]` | mostly matched by always inserting `ResourceType::Ore`; overlay id fallback is approximate | `src/sim/terrain_spawn.rs:213`, `src/sim/terrain_spawn.rs:248` | Preserve `ResourceType::Ore` for source cells without a tiberium overlay; when real TiberiumClass metadata exists, resolve overlay from type 0 image instead of first `TIB*` name | `tibtre_empty_source_cell_defaults_to_riparius_type_zero`: seed a spawner on a source cell with no overlay metadata and assert spawned node is Ore/Riparius, not Gem/Cruentus | Do not change default to type 1; type 1 is Cruentus/gems |
| Source-cell overlay, when present and recognized, controls spawned type | `0x004837EB..0x0048381D`, `0x005FDD20` | missing/unchecked; current `terrain_spawn.rs` always places Ore | future shared tiberium metadata plus `src/sim/terrain_spawn.rs` | When overlay-backed source cells are possible, derive type through overlay-to-tiberium mapping and place matching resource type | `tibtre_source_overlay_type_controls_spawned_resource_type`: seed a TIBTRE source cell with a recognized Cruentus overlay, force spawn, assert spawned resource type is gem/Cruentus | Do not key type from terrain name or use default-ore overlay when source overlay identifies a different type |
| `PlaceTiberium` receives `(type_index, 3)` after target acceptance | `0x004838BC..0x004838C5`, `0x00487190` | density 3 already represented; full target side effects owned by slot 4 | `src/sim/terrain_spawn.rs:146`, `src/sim/terrain_spawn.rs:193` | Keep density 3 for TIBTRE spread; reconcile with slot 4 for overlay data, queues, and dirty/radar effects | `tibtre_spread_calls_density_three_place_semantics`: force spawn on an empty accepted cell and assert density/stock corresponds to binary `param_3=3` | Do not use normal spread's density-1/growth semantics for TIBTRE |

### Stale Docs / Follow-up Docs

- `TERRAIN_CLASS_GHIDRA_REPORT.md` section 4.1 line claiming "The tiberium type passed to `SpreadTiberium` is hardcoded `1` (Riparius/green)" is stale/wrong for this slice. Replacement wording: "TerrainClass::AI passes `force=true` (`1`) to `CellClass::SpreadTiberium`; `SpreadTiberium` derives the tiberium type from the source cell overlay and defaults forced no-overlay sources to type index `0` (stock Riparius)."
- `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md` default type 0/Riparius wording is confirmed for forced no-source-overlay TIBTRE.

## Sources

- Ghidra decompilation: `TerrainClass::AI @ 0x0071C730`
- Ghidra assembly context: `0x0071C84D..0x0071C8D0`
- Ghidra decompilation and assembly context: `CellClass::SpreadTiberium @ 0x00483780`
- Ghidra decompilation: `CellClass::OverlayToTiberiumIndex @ 0x005FDD20`
- Ghidra decompilation: `CellClass::PlaceTiberium @ 0x00487190`
- Ghidra decompilation: `CellClass::CanPlaceTiberium @ 0x004838E0` (touched only)
- Ghidra decompilation: `TiberiumClass::ReadINI_All @ 0x00721D10`
- Ghidra decompilation: `FUN_006B8B30`, `FUN_006B8CA0` for `[SpecialFlags] TiberiumSpreads` bit 7
- `ini/rulesmd.ini` `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]`, `[Tiberiums]`
- Current Rust scan: `src/sim/terrain_spawn.rs`, `src/sim/ore_growth.rs`, `src/rules/ruleset.rs`
