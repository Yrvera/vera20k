# GACNST IsDeployable Special Branch -- Ghidra Research Report

**Address(es):** `0x007393C0` (`UnitClass::Deploy`), branch `0x00739855..0x00739926`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** only the post-success `DeploysInto` branch that runs after the new construction-yard building exists, narrowed to AMCV -> GACNST-facing behavior.
**Non-Scope:** generic deploy placement, MCV destruction/health transfer, slave-miner manager transfer, building unlimbo internals, full AI build queue algorithm.
**Confidence:** High for the branch gate and direct writes; Medium for human-readable names of some house flags where older docs disagree.
**Active in YR:** Conditional. Yes for standard multiplayer/skirmish AI houses deploying a `ConstructionYard=yes` target; no for the local/player-control house because the branch requires `HouseClass__IsPlayerControl(owner) == 0`.

## 1. Overview

After `UnitClass::Deploy` creates and unlimbos the target building, it has a narrow construction-yard setup branch. For AMCV -> GACNST, stock data satisfies the type side of the gate (`DeploysInto=GACNST`, `ConstructionYard=yes`), but the runtime side requires multiplayer and a non-player-control owner.

The branch updates AI/base-planning state for the owning house. It does not contain local tactical view centering, shroud reveal, sidebar tab switching, or any GACNST-specific hardcoded check.

## 2. Key Offsets

| Offset | Type | Verified purpose in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `Unit+0x21C` | ptr | owner `HouseClass*` | `0x00739855`, `0x007398BF`, repeated owner loads | Yes |
| `Building+0x520` | ptr | `BuildingTypeClass*` | `0x00739868`, then reads `+0x16B9` | Yes |
| `BuildingType+0x16B9` | byte | `ConstructionYard=yes` gate | `0x00739868..0x00739876`; `rulesmd.ini [GACNST]` | Yes |
| `House+0x1EC` | byte | multiplayer `IsPlayerControl` / human-control flag returned by `0x0050B730` | `HouseClass__IsPlayerControl @ 0x0050B730`; docs field map | Conditional |
| `House+0x5490` | CellStruct | primary base center | `FUN_0050E000 @ 0x0050E000` writes param to `+0x5490` | Yes |
| `House+0x5708` | ptr | base-plan node array; first node cell is `items+4` | `0x007398DE..0x007398EA`; prior docs node format | Yes |
| `House+0x5714` | int | base-plan node count | `FUN_00505180 @ 0x005051B4..0x005051D1` | Yes |
| `House+0x5750` | CellStruct | base-plan center cell | `0x007398ED..0x007398F3` | Yes |
| `House+0x1EE/+0x1F2/+0x1F3` | bytes | AI/base-start flags set to `1` by this branch | `0x007398F9..0x00739919`; names from prior house docs are not all equally proven here | Conditional |

## 3. Core Logic

Verified branch gate in `UnitClass::Deploy`:

```text
after new building placement succeeds:
  HouseClass__IsPlayerControl(owner)
  if result == 0
     and new_building.Type.ConstructionYard != 0
     and g_GameMode != 0:
       run construction-yard setup branch
```

Evidence: `0x00739855 CALL 0x0050B730`, `0x00739860 JNZ 0x0073992B`, `0x00739868..0x00739876` checks `BuildingType+0x16B9`, `0x0073987C..0x00739883` checks `g_GameMode` at `0x00A8B238`.

When the branch runs, the exact order is:

1. Convert the new building lepton position to a packed cell using signed-adjusted `>> 8` for X and Y.
   Evidence: `0x00739889..0x007398C9`.
2. Call `FUN_0050E000(owner, cell)`, which writes `House+0x5490 = cell`.
   Evidence: call at `0x007398CE`; callee `0x0050E000`.
3. Call `FUN_00505180(owner)`.
   Evidence: call at `0x007398D9`.
   - In multiplayer, if `House+0x1EC == 0`, this calls `HouseClass__Recalculate_Alliances @ 0x00501640`.
   - If `House+0x5714 == 0`, it temporarily clears `g_MapEditorMode`, calls `HouseClass__AI_RecalcBuildOptions @ 0x005054B0`, then restores `g_MapEditorMode`.
4. Write the packed cell into `(*(House+0x5708) + 4)`, i.e. the cell slot of the first 16-byte base-plan node.
   Evidence: `0x007398DE..0x007398EA`; prior `HOUSECLASS_GHIDRA_REPORT.md` documents node format `[TypeIndex, CellStruct, Flags, Reserved]`.
5. Write the same cell to `House+0x5750`.
   Evidence: `0x007398ED..0x007398F3`.
6. Set `House+0x1EE = 1`, `House+0x1F2 = 1`, and `House+0x1F3 = 1`.
   Evidence: `0x007398F9..0x00739919`.
7. Call `FUN_0050C920(owner)`.
   Evidence: call at `0x00739926`.
   - This helper chooses `House+0x5494` if valid, otherwise `House+0x5490`.
   - It calls `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` with a 4-by-4 search setup around that base cell.
   - For each owned techno in `g_TechnoClass_Array`, it skips RTTI `6` and `2`, and skips units whose type has `DeploysInto` pointing to a `ConstructionYard=yes` building.
   - For remaining owned technos, if a valid base center exists, it calls vtable `+0x480` with the nearby cell, calls `TechnoClass__SetGhostCell(0)`, then sets mission `2`.

## 4. INI Keys

| Key | Stock YR value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[AMCV] DeploysInto` | `GACNST` | Generic deploy target, no AMCV hardcode | `rulesmd.ini:6969..6977`; `UnitType+0x404` read in `UnitClass::Deploy` | Yes |
| `[GACNST] ConstructionYard` | `yes` | Satisfies branch type gate | `rulesmd.ini:11622..11625`; `BuildingType+0x16B9` check | Yes |
| `[GACNST] Factory` | `BuildingType` | Makes the spawned building a building factory for normal production systems; the special branch only recalculates AI build options | `rulesmd.ini:11630`; Rust production scan | Yes |
| `[GACNST] UndeploysInto` | `AMCV` | Not read by this branch | `rulesmd.ini:11631`; no branch read in `0x00739855..0x00739926` | Out of scope |
| `[GACNST] Sight` | `8` | Not read by this branch; any reveal is generic vision, not this setup block | `rulesmd.ini:11632`; no branch reveal call | Yes globally |
| `artmd.ini [GACNST] Foundation` | `4x4` | Generic origin/placement already handled before this branch | `artmd.ini:1599..1602`; non-scope here | Yes |

## 5. Integration Points

Entry into this branch is only through successful `UnitClass::Deploy`; xrefs to the deploy function include `UnitClass__Mission_Deploy_Building`, `UnitClass__Mission_Deploy`, `UnitClass__PerCellProcess`, and `UnitClass__DeployHelper`.

The branch is not a local UI/sidebar path. The only production-facing call inside it is `HouseClass__AI_RecalcBuildOptions`, and that call is conditional on `House+0x5714 == 0`. For player houses in multiplayer/skirmish, the branch is skipped because `HouseClass__IsPlayerControl` returns nonzero.

`FUN_0050C920` has player-visible AI movement implications: nearby owned non-building/non-aircraft/non-ConYard-deployer technos can be assigned a destination near the base and mission `2` after an AI ConYard deploys.

## 6. Current Rust Implementation Status

Rust `src/sim/world/world_spawn.rs::deploy_mcv` despawns the MCV, spawns the target building, preserves selection, and starts a 30-tick `BuildingUp`. It does not update `HouseState.base_center`, any base-plan list, or any AI starting-base flags.

Rust has a `HouseState.base_center` field in `src/sim/house_state.rs`, and AI code reads a base center through `src/sim/ai.rs::find_base_center`. Production/sidebar surfaces are data-driven through `Factory=BuildingType` (`src/sim/production`) and `app_sidebar_render.rs`, but no Rust equivalent of the AI-only binary setup branch was found in this scan.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Deploy` branch gate | verified | `0x00739855..0x00739883` | none |
| Cell conversion from building location | verified | `0x00739889..0x007398C9` | none |
| `HouseClass::SetPrimaryCenter` | verified | `0x0050E000`; call `0x007398CE` | none |
| `FUN_00505180` helper | verified for direct side effects | `0x00505180..0x005051DE` | full AI build-option internals out of scope |
| `HouseClass__AI_RecalcBuildOptions` | touched-not-exhausted | `0x005054B0` decompile | full build-order algorithm not needed for this branch |
| Base-plan first-node cell write | verified | `0x007398DE..0x007398EA`; prior node format docs | exact TypeIndex value established by AI recalc, not this branch |
| AI/base flags `+0x1EE/+0x1F2/+0x1F3` | verified writes, names partial | `0x007398F9..0x00739919`; older house docs | semantic names should be verified in a house-flag audit |
| `FUN_0050C920` helper | verified for called side effects | `0x0050C920..0x0050CAC4` | exact mission enum label not re-derived here |
| View centering / shroud reveal inside branch | verified absent | no display/reveal calls in `0x00739855..0x00739926` | none |
| Generic deploy conversion | not-touched | prior docs | explicitly non-scope |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is the branch GACNST-hardcoded? -> No; it checks the new building type's `ConstructionYard` byte at `+0x16B9`.` (evidence: `0x00739868..0x00739876`; `rulesmd.ini [GACNST]`)
- `[RESOLVED] OQ-2 -- Does the branch run for the local/player-control house? -> No in multiplayer; `HouseClass__IsPlayerControl` returns `House+0x1EC`, and nonzero skips the branch.` (evidence: `0x0050B730`, `0x00739855..0x00739862`)
- `[RESOLVED] OQ-3 -- Does the branch update house base center? -> Yes, it writes packed cell to `House+0x5490`.` (evidence: `0x007398CE`, `0x0050E000`)
- `[RESOLVED] OQ-4 -- Does it update base-plan state? -> Yes, it writes first node cell `items+4` and `House+0x5750`.` (evidence: `0x007398DE..0x007398F3`)
- `[RESOLVED] OQ-5 -- Does it recalc production/sidebar availability? -> It calls AI build-option recalculation only through `FUN_00505180`, only if `House+0x5714 == 0`; no player sidebar call is present.` (evidence: `0x005051B4..0x005051D1`)
- `[RESOLVED] OQ-6 -- Does it reveal shroud or center the tactical view? -> No direct reveal/display/viewport call appears in this branch.` (evidence: full branch `0x00739855..0x00739926`)
- `[RESOLVED] OQ-7 -- Does it set starting-base/AI flags? -> Yes, three bytes are set to `1`: `+0x1EE`, `+0x1F2`, `+0x1F3`.` (evidence: `0x007398F9..0x00739919`)
- `[RESOLVED] OQ-8 -- Does it register the Construction Yard in a separate construction-yard registry? -> No separate registry write was found in this branch; it updates house base-plan/base-center state instead.` (evidence: full branch `0x00739855..0x00739926`)
- `[RESOLVED] OQ-9 -- Are nearby unit side effects present? -> Yes, `FUN_0050C920` can assign owned eligible technos a nearby base destination and mission `2`.` (evidence: `0x0050C9D5..0x0050CAAA`)
- `[DEFERRED] OQ-10 -- What are the exact user-facing names of `House+0x1F2` and `House+0x1F3`?` (category: `requires-different-system-context`; reason: this branch proves the writes, not all readers; next-step-if-pursued: audit all xrefs to `+0x1F2/+0x1F3`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| AI/non-player-control construction-yard deploy updates house base center to the new building cell. | `0x00739889..0x007398CE`, `0x0050E000` | missing | `src/sim/world/world_spawn.rs::deploy_mcv`; `src/sim/house_state.rs::HouseState.base_center`; AI base lookup | On AI-owned AMCV -> GACNST in skirmish/multiplayer, set owner base center from the spawned ConYard origin cell. | AI MCV deploys; subsequent AI decisions using `find_base_center` use the new GACNST cell without waiting for another building. | Do not apply this as a local-player camera/reveal behavior; binary branch is skipped for player-control houses. |
| AI build/base-plan state is refreshed, but only through AI-specific branch logic. | `0x00505180`, `0x005054B0`, `0x007398DE..0x00739919` | missing/unmodeled | AI/base-planning state; production availability scan | Add an AI-base initialization hook if Rust models AI base plans/flags; keep player sidebar availability data-driven from factories. | AI ConYard deploy makes AI building choices available; human sidebar remains driven by ordinary factory/prerequisite logic. | Do not hardcode AMCV/GACNST; gate on `DeploysInto` target with `ConstructionYard=yes` and owner control state. |
| The branch can push owned eligible technos to a passable cell near the base and set mission `2`. | `0x0050C920..0x0050CAAA` | missing | future AI movement/order surface | If implementing AI base-start parity, issue equivalent movement/guard order for owned non-building/non-aircraft/non-ConYard-deployer technos. | AI starting units near deployed ConYard disperse toward a nearby passable base cell. | Do not re-route MCV-like deployers or aircraft; binary explicitly skips RTTI `6`, `2`, and deployers whose target is a ConYard. |

### Stale Docs / Follow-up Docs

- `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md:173`: replace `Handle fog/shroud reveal for local player` with `For non-human owners, the generic deploy success path sets building bytes +0x6CA/+0x6CB; the separate ConstructionYard setup branch is AI/non-player-control only and contains no local view-centering or shroud-reveal call.`
- `docs/research/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md:679`: replace `UnitClass::Deploy (0x007393c0) -- updates base center when MCV deploys` with `UnitClass::Deploy updates `House+0x5490` only in multiplayer when the deployed building is `ConstructionYard=yes` and the owner is not player-control; local/player-control MCV deploy skips this special branch.`

## Negative Facts / Do Not Do

- Do not hardcode AMCV or GACNST; the binary checks `DeploysInto` target type and `ConstructionYard=yes`.
- Do not run this branch for the local/player-control house in multiplayer; the binary skips it when `HouseClass__IsPlayerControl` is nonzero.
- Do not add tactical camera centering or shroud reveal as a side effect of this branch; no such call exists in `0x00739855..0x00739926`.
- Do not treat `UndeploysInto=AMCV` as part of this branch; it is not read here.
- Do not treat `House+0x1EE/+0x1F2/+0x1F3` semantic names as fully proven by this report; only the writes are proven here.

## Remaining Uncertainty

- Exact semantic names/readers for `House+0x1F2` and `House+0x1F3` remain outside this slice; the branch writes are verified.
- Full `HouseClass__AI_RecalcBuildOptions` build-order ordering is not exhausted here; this report only claims that the ConYard branch invokes it under the `House+0x5714 == 0` condition.

## Sources

- Ghidra read-only decompiles/disassembly: `UnitClass::Deploy @ 0x007393C0`, `HouseClass__IsPlayerControl @ 0x0050B730`, `HouseClass__IsHumanPlayer @ 0x0050B6F0`, `FUN_0050E000`, `FUN_00505180`, `HouseClass__Recalculate_Alliances @ 0x00501640`, `HouseClass__AI_RecalcBuildOptions @ 0x005054B0`, `FUN_0050C920`.
- INI: `ini/rulesmd.ini` `[AMCV]`, `[GACNST]`; `ini/artmd.ini` `[GACNST]`.
- Prior docs referenced: `MCV_DEPLOY_GHIDRA_REPORT.md`, `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, `HOUSECLASS_GHIDRA_REPORT.md`, `HOUSECLASS_VERIFIED_FIELD_MAP.md`.
- Rust scan: `src/sim/world/world_spawn.rs::deploy_mcv`, `src/sim/house_state.rs::HouseState`, `src/sim/ai.rs`, `src/sim/production`, `src/app_sidebar_render.rs`.
