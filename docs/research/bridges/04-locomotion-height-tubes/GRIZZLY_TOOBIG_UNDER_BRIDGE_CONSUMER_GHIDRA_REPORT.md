# Grizzly TooBigToFitUnderBridge Consumer -- Ghidra Research Report

**Address(es):** `0x0074774E`, `0x0073B140`, `0x0073C5F0`, `0x0073F0A0`, `0x004D9C60`, `0x00429F54`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Consumer/liveness semantics of `UnitTypeClass+0xE16` / `TooBigToFitUnderBridge=` for stock `[MTNK]` Grizzly movement under bridge cells.  
**Non-Scope:** Full bridge collapse/repair, non-ground locomotors, low-bridge tube construction, and complete rendering implementation beyond identifying the flag's actual consumers.  
**Confidence:** High  
**Active in YR:** Yes for parse, draw consumers, unit A* entry, and bridge height checks. No movement consumer for `+0xE16` was found in the verified slice.

## Target Question

Does stock `gamemd.exe` reject under-bridge pathfinding or cell entry for `[MTNK]` because `TooBigToFitUnderBridge=true`, or is the current Rust comment that movement is not gated by the flag closer to binary behavior?

## Non-Goals

- Do not investigate full bridge destruction, repair hut behavior, or bridge fallout.
- Do not investigate aircraft/ship-specific locomotor behavior except as needed to prove this flag is not the Grizzly movement gate.
- Do not modify Rust, INI, or in-repo docs.

## Evidence Needed To Mark COMPLETE

- INI/default source plus binary reader address for `[MTNK] TooBigToFitUnderBridge=true`.
- Decompile plus assembly/xref evidence for all verified `+0xE16` consumers in this slice.
- Decompile plus caller/vtable evidence for the live `UnitClass::Can_Enter_Cell` path used by A* and its bridge sub-check.
- Explicit negative evidence that the `UnitClass::Can_Enter_Cell` / bridge traversal path gates on cell flags, height, MovementZone/SpeedType, occupancy, and tube state, not `UnitType+0xE16`.

## Stop Conditions

- Stop when the parser, draw readers, A* cell-entry entry point, and bridge sub-check are re-opened and the conflict with `MTNK.md` is resolved.
- Stop without expanding into collapse/repair or non-ground locomotors once `+0xE16` is proven non-movement for stock Grizzly.
- If a hidden computed `+0xE16` reader appears in the movement path, downgrade to PARTIAL and queue a broader byte-pattern audit.

## 1. Overview

For stock `[MTNK]`, `TooBigToFitUnderBridge=true` is live content and parses into `UnitTypeClass+0xE16`, but the verified consumers are rendering consumers, not movement/pathfinding consumers. `UnitClass::Can_Enter_Cell` and its bridge sub-check decide bridge legality from cell flags, height deltas, bridgehead/tube state, locomotor passability, overlays, and occupancy lists; the decompile contains no read of `+0xE16`.

Player-visible movement implication: a Grizzly should not be rejected from an under-bridge route merely because this flag is true. Whether the underlying bridge-under cell is passable is still controlled by ordinary terrain/bridge/pathfinding rules.

## 2. Class Layout / Key Offsets

| Offset | Owner | Type | Purpose | Evidence | Active in YR |
|---:|---|---|---|---|---|
| `+0xE16` | `UnitTypeClass` | byte bool | `TooBigToFitUnderBridge=` parsed value | `UnitTypeClass__ReadINI @ 0x0074774E` calls `CCINIClass__ReadBool(..., s_TooBigToFitUnderBridge_00845DC8, current +0xE16)` and stores to `+0xE16`; store instruction at `0x00747778` | Yes |
| `+0x6C4` / `param_1[0x1B1]` | Unit/Techno instance | type pointer | Type pointer used by `UnitClass` readers | `UnitClass__Can_Enter_Cell` and draw decompiles read type fields through this pointer | Yes |
| `+0x140` | `CellClass` | flags dword | high-bridge/bridgehead flags used by entry logic | `UnitClass__Can_Enter_Cell @ 0x0073F0A0`, `CheckBridgeTraversal @ 0x004D9C60` | Yes |
| `+0x11B` | `CellClass` | signed byte | ground height / level used for bridge height delta | `0x0073F0A0`, `0x004D9C60` | Yes |
| `+0x124` / `+0x128` | `CellClass` | occupancy bitfields | ground/bridge occupancy snapshots | `0x0073F0A0` | Yes |
| `+0xE4` / `+0xE8` | `CellClass` | object lists | ground/bridge object list heads | `0x0073F0A0` | Yes |

## 3. Core Logic

### Parser

`UnitTypeClass__ReadINI @ 0x0074774E` parses the key after `CarriesCrate` and before `HalfDamageSmokeLocation`:

```text
ReadBool(section, "TooBigToFitUnderBridge", current UnitType+0xE16)
store AL -> UnitType+0xE16
```

Assembly context around the store shows `MOV byte ptr [EDI + 0xe16], AL` at `0x00747778`. The decompile resolves the key string as `s_TooBigToFitUnderBridge_00845DC8`.

Active in YR: Yes. Stock `ini/rulesmd.ini [MTNK]` sets `TooBigToFitUnderBridge=true` and `MovementZone=Normal`.

### Actual Runtime Consumers

Two live `UnitClass` draw functions read `+0xE16`:

1. `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`, vtable data xref `0x007F61CC`.
   - Assembly: `0x0073B1B0: MOV AL, byte ptr [ECX + 0xe16]`.
   - Decompile: if `Type+0xE16 != 0` and `TechnoClass__IsOnBridge_ForFiring()` is true and `FUN_00703E70()` bridge-piece-neighbor count is zero, enable split SHP sprite/shadow blit.
   - Active in YR: Yes. This is draw phase, not movement.

2. `UnitClass__Draw_Body_And_Turret @ 0x0073C5F0`, vtable data xref `0x007F61C8`.
   - Assembly: `0x0073CE0D: MOV AL, byte ptr [ECX + 0xe16]`.
   - Decompile: only in the no-turret draw branch; if bridge-edge predicate holds, call vtable `+0x50C` with Z bias `0xfffffff0` (-16) instead of the normal pre-call path.
   - Active in YR: Conditional. The function is live; the `+0xE16` body-draw effect is gated by `Turret=no`. Stock Grizzly has `Turret=yes`, so Grizzly skips this body-draw `+0xE16` branch but can still hit the sibling sprite/shadow draw consumer.

### Movement / Cell-Entry Path

`AStar_main_loop` calls the virtual A* cell-entry slot at `0x00429F54`:

```text
CALL dword ptr [EDX + 0x1ac]
```

The `UnitClass` vtable binds that slot to `UnitClass__Can_Enter_Cell @ 0x0073F0A0` (`get_function_xrefs` reports data xref `0x007F5E1C`). This is the live unit/vehicle A* legality function.

`UnitClass__Can_Enter_Cell @ 0x0073F0A0`:

- Preselects ground vs bridge layer using `Cell+0x140 & 0x100` and `abs(path_height - Cell+0x11B) < 2`.
- Handles tunnel/tube cases before bridge traversal.
- Calls vtable `+0x1B0`, bound for `UnitClass` to `CheckBridgeTraversal @ 0x004D9C60`.
- Re-snapshots bridge occupancy bits when `path_height == Cell+0x11B + 4`.
- Calls `FootClass__LocomotorPassabilityCheck`.
- Handles overlay/wall and object-list soft blockers.
- Does not read `UnitType+0xE16`.

`CheckBridgeTraversal @ 0x004D9C60`:

- Returns only 0 or 7.
- Uses direction, parent/candidate cells, `Cell+0x140` flags, `Cell+0x11B` height, and `Cell+0x11C` slope/ramp byte.
- Allows `abs(diff)==4` high-bridge transitions only through the bridge/bridgehead flag shape.
- Does not read `UnitType+0xE16`.

Active in YR: Yes. This is the path used by stock Grizzly movement/pathfinding.

## 4. INI Keys

| Key | Stock MTNK value | Binary storage/effect | Active in YR |
|---|---|---|---|
| `TooBigToFitUnderBridge=` | `true` in `ini/rulesmd.ini [MTNK]` | Parsed to `UnitTypeClass+0xE16`; consumed by draw code only in this verified slice | Yes |
| `MovementZone=` | `Normal` in `ini/rulesmd.ini [MTNK]` | Stored in `TechnoTypeClass+0x5B4`; consumed by passability/zone rows outside this flag slice | Yes |
| `Turret=` | `yes` in `ini/rulesmd.ini [MTNK]` | `TechnoTypeClass+0xCA1`; makes Grizzly skip the no-turret body-draw `+0xE16` branch | Yes |

## 5. Integration Points

| Function / site | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitTypeClass__ReadINI @ 0x0074774E` | Parses `TooBigToFitUnderBridge` to `+0xE16` | decompile plus `0x00747778` store | Yes |
| `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140` | Draw/sprite/shadow consumer | decompile plus `0x0073B1B0` assembly read and vtable data xref `0x007F61CC` | Yes |
| `UnitClass__Draw_Body_And_Turret @ 0x0073C5F0` | Draw/body consumer for no-turret branch | decompile plus `0x0073CE0D` assembly read and vtable data xref `0x007F61C8` | Conditional; live function, `+0xE16` branch gated by `Turret=no` |
| `AStar_main_loop @ 0x00429F54` | Calls object `Can_Enter_Cell` vtable slot `+0x1AC` | assembly context | Yes |
| `UnitClass__Can_Enter_Cell @ 0x0073F0A0` | Live unit movement/cell-entry legality | decompile plus vtable data xref `0x007F5E1C` | Yes |
| `CheckBridgeTraversal @ 0x004D9C60` | Bridge height/bridgehead sub-check | decompile from `+0x1B0` path | Yes |

## 6. Current Rust Implementation Status

| Surface | Current status | Rust-facing implication |
|---|---|---|
| `src/rules/object_type.rs` | Parses/carries `too_big_to_fit_under_bridge` | Keep parsing; the field is real content. |
| `src/sim/world/world_spawn.rs` / entity state | Copies the parsed flag to entities | Harmless for future render consumers. |
| `src/sim/movement/movement_path.rs::merge_path_blocks` | Current comment says gamemd does not gate movement on the flag and returns only entity blocks | Matches this binary finding. |
| `src/sim/pathfinding/core.rs`, `cell_entry.rs`, `zone_search.rs`, `zone_build.rs` | Bridge/pathfinding logic is keyed by layer, path height, MovementZone/SpeedType, and cell flags; no current `TooBig` movement gate in the scanned surfaces | Correct direction for this flag; remaining bridge parity work belongs to bridge layer/height semantics, not `TooBig`. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `[MTNK]` INI values | verified | `ini/rulesmd.ini [MTNK]`: `TooBigToFitUnderBridge=true`, `Turret=yes`, `MovementZone=Normal` | none |
| `UnitTypeClass__ReadINI` parse | verified | decompile `0x0074774E`; assembly store `0x00747778` | none |
| `UnitClass__Draw_Sprite_With_BridgeFudge` consumer | verified | decompile `0x0073B140`; assembly read `0x0073B1B0`; xref `0x007F61CC` | exact visual output belongs to render follow-up |
| `UnitClass__Draw_Body_And_Turret` consumer | verified | decompile `0x0073C5F0`; assembly read `0x0073CE0D`; xref `0x007F61C8` | exact visual output belongs to render follow-up |
| `AStar` live cell-entry dispatch | verified | assembly `0x00429F54`; UnitClass vtable data xref `0x007F5E1C` | none |
| `UnitClass__Can_Enter_Cell` movement gate | verified | decompile `0x0073F0A0`; no `+0xE16` read in cell-entry body | no byte-pattern audit repeated this pass; prior report has one |
| `CheckBridgeTraversal` sub-check | verified | decompile `0x004D9C60`; no `+0xE16` read; checks cell flags/heights | none for this flag |
| Ship/non-ground locomotor consumers | deferred | out-of-scope by dispatch prompt | only needed for naval/aircraft-specific movement reports |
| Full render bridge-fudge implementation | deferred | draw consumers identified but not fully visual-tested here | render-specific follow-up if needed |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Is the key live for stock MTNK? -> Yes, stock rules set it true and UnitTypeClass parses it to +0xE16.` (evidence: `ini/rulesmd.ini [MTNK]`; `0x0074774E`, `0x00747778`)
- `[RESOLVED] OQ-2 -- Does A* call UnitClass::Can_Enter_Cell for vehicles? -> Yes, A* calls vtable +0x1AC, UnitClass binds it to 0x0073F0A0.` (evidence: `0x00429F54`, xref `0x007F5E1C`)
- `[RESOLVED] OQ-3 -- Does UnitClass::Can_Enter_Cell read +0xE16? -> No read appears in the decompile; bridge logic uses cell flags, height, tube, locomotor passability, overlays, and occupancy.` (evidence: `0x0073F0A0`)
- `[RESOLVED] OQ-4 -- Does CheckBridgeTraversal read +0xE16? -> No; it uses direction, parent/candidate cells, cell flags, heights, and slope byte.` (evidence: `0x004D9C60`)
- `[RESOLVED] OQ-5 -- What consumers read +0xE16 in the verified slice? -> Two UnitClass draw vtable methods: sprite/shadow split-blit and no-turret body Z bias.` (evidence: `0x0073B1B0`, `0x0073CE0D`)
- `[RESOLVED] OQ-6 -- Is the body-draw TooBig branch active for stock Grizzly? -> The function is live, but Grizzly has Turret=yes, so the no-turret body branch is skipped for MTNK.` (evidence: `ini/rulesmd.ini [MTNK]`; `0x0073C5F0`)
- `[RESOLVED] OQ-7 -- Can the flag explain pathfinder rejection of under-bridge cells for MTNK? -> No. Any rejection must come from ordinary terrain/bridge/height/passability state, not +0xE16.` (evidence: `0x0073F0A0`, `0x004D9C60`)
- `[RESOLVED] OQ-8 -- Does current Rust hard-block movement on this flag? -> No in the current scanned code; `merge_path_blocks` explicitly leaves movement ungated.` (evidence: `src/sim/movement/movement_path.rs`)
- `[DEFERRED] OQ-9 -- Are there non-UnitClass locomotor-specific +0xE16 movement consumers?` (category: out-of-scope; reason: prompt only requires stock Grizzly/MTNK UnitClass movement; next-step-if-pursued: separate naval/aircraft flag consumer sweep)
- `[DEFERRED] OQ-10 -- Exact pixel output of bridge-fudge rendering?` (category: out-of-scope; reason: this slot is movement consumer semantics, not render parity; next-step-if-pursued: render-focused trace/screenshot comparison)

Adversarial corner checks:

- If `TooBig=false`, MTNK movement under bridges is unchanged because movement never reads the flag.
- If `Turret=yes`, MTNK skips the no-turret body-draw branch but not necessarily the sprite/shadow draw method.
- If an under-bridge cell is water/impassable for `MovementZone=Normal`, MTNK may still be rejected, but that is ordinary passability, not `TooBig`.
- If a bridgehead transition height diff is illegal, `CheckBridgeTraversal` can return 7, again independent of `TooBig`.
- If a stale doc says "TooBig blocks pathing", it is stale for MTNK movement.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `TooBigToFitUnderBridge` is parsed and true for stock MTNK, but is not a movement gate | `ini/rulesmd.ini [MTNK]`; `0x0074774E`; `0x0073F0A0`; `0x004D9C60` | none observed in current `movement_path.rs` | `src/sim/movement/movement_path.rs`, `src/sim/pathfinding/*` | Keep `TooBig` out of path block generation and cell-entry checks | Grizzly under-bridge route availability is identical when only this flag toggles and terrain/bridge data stays the same | Do not reintroduce a hard under-bridge block keyed by `too_big_to_fit_under_bridge` |
| Grizzly `Turret=yes` means the body-draw `+0xE16` Z-bias branch is skipped for MTNK | `ini/rulesmd.ini [MTNK]`; `0x0073C5F0` | render consumer not implemented/unchecked | render/unit draw surfaces | If bridge-fudge render is implemented, gate the body Z-bias by the no-turret draw path; keep MTNK out of that branch | MTNK on bridge edge does not use the no-turret body `-16` path | Do not apply no-turret body Z-bias to all TooBig units |
| The sibling sprite/shadow draw method reads `+0xE16` without the Turret pre-dispatch | `0x0073B140`; `0x0073B1B0` | render consumer not implemented/unchecked | render/unit SHP shadow/sprite draw | Future render parity can use this flag for split-blit/shadow behavior, not movement | A TooBig turreted unit at a bridge-edge cell is eligible for sprite/shadow bridge-fudge conditions | Do not delete the parsed field as "unused"; it is used by render code |

## Negative Facts / Do Not Do

- Do not say stock Grizzly cannot path under bridges because `TooBigToFitUnderBridge=true`; that claim is not supported by the verified movement functions.
- Do not add `too_big_to_fit_under_bridge` to `PathGrid`, `cell_entry`, `zone_search`, or `zone_build` as a hard movement blocker.
- Do not special-case MTNK/Grizzly for bridge pathing; the binary uses generic UnitClass cell-entry and bridge traversal.
- Do not treat bridge rejection as disproving this finding; ordinary terrain/passability and height/bridgehead checks can still reject a cell.
- Do not remove the field from parsed data; it has real draw consumers.

## Stale Docs / Follow-up Docs

Replacement wording for `docs/research/units/allied/MTNK.md` lines that currently say the Grizzly cannot path under bridges:

> `TooBigToFitUnderBridge=true` is live stock MTNK content and parses to `UnitTypeClass+0xE16`, but verified `gamemd.exe` movement/pathfinding does not gate Grizzly cell entry on this flag. The verified consumers are UnitClass draw methods for bridge-edge sprite/shadow/body Z handling; under-bridge movement legality comes from ordinary terrain, bridge height/bridgehead, MovementZone/SpeedType, tube, overlay, and occupancy logic.

## Remaining Uncertainty

None for the target question. The exact pixel-level draw result of the bridge-fudge consumers remains out-of-scope.

## Sources

- Ghidra decompiles: `UnitTypeClass__ReadINI @ 0x0074774E`; `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`; `UnitClass__Draw_Body_And_Turret @ 0x0073C5F0`; `UnitClass__Can_Enter_Cell @ 0x0073F0A0`; `CheckBridgeTraversal @ 0x004D9C60`.
- Ghidra assembly contexts: `0x0073B1B0`, `0x0073CE0D`, `0x00429F54`, `0x00747778`.
- Ghidra xrefs: `0x007F61CC`, `0x007F61C8`, `0x007F5E1C`.
- Prior report cross-check: `docs/research/TOO_BIG_TO_FIT_UNDER_BRIDGE_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini`.
- Rust scanned: `src/sim/movement/movement_path.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/pathfinding/core.rs`.
