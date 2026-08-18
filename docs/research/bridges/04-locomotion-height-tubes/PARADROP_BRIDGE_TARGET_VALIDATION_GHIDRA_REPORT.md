# Paradrop Bridge Target Validation -- Ghidra Research Report

**Address(es):** `0x006CC390` (`SuperClass::Launch` cases 5 and 6), `0x0056DC20` (`Find_Nearby_Passable_Cell`), `0x00485060` (`CellClass::IsOnBridgeSurface`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** launch-time target validation and bridge-surface replacement for `Type=ParaDrop` and `Type=AmerParaDrop` only.  
**Non-Scope:** parachuted infantry landing layer, `Drop_Payload`, PDPLANE edge spawn, paradrop cadence, cursor hit-testing.  
**Confidence:** High for launch branch, call arguments, helper bridge filter, fallback behavior; Medium for user-visible destroyed-bridge click implications because this pass did not run a live map scenario.  
**Active in YR:** Yes. `rulesmd.ini` binds `[ParaDropSpecial] Type=ParaDrop` and `[AmericanParaDropSpecial] Type=AmerParaDrop`; `FUN_006CB920` calls `SuperClass::Launch` on normal fired-superweapon paths.

## Working Notes

Target question: When ParaDrop/AmerParaDrop is fired at a high bridge or bridge-related cell, what exact target validation and replacement does `SuperClass::Launch` perform?  
Non-goals: Do not investigate later parachute descent or landing-on-bridge behavior after `Drop_Payload`.  
Evidence needed to mark COMPLETE: decompile plus assembly for cases 5/6, exact `Find_Nearby_Passable_Cell` pushed arguments, helper branch semantics, and the spawner target argument.  
Stop conditions: Stop after launch-time replacement/abort behavior is resolved for cases 5/6 and no unresolved material question remains inside this slice.

## 1. Overview

The stale high-level claim "paradrop rejects bridge targets" is not accurate for the current binary. Cases 5 and 6 first detect whether the clicked target cell is a bridge surface. If it is, they call `Find_Nearby_Passable_Cell`, but they only replace the target when the returned cell is valid and `CellClass::IsOnBridgeSurface` is false. If the helper returns the sentinel, a null/sentinel cell, or another bridge-surface cell, launch continues with the original clicked bridge cell.

Active in YR: Yes. This is directly inside the live `SuperClass::Launch` cases selected by stock YR paradrop superweapon type values.

## 2. Class Layout / Key Offsets

| Struct / global | Offset / value | Meaning in this slice | Active in YR |
|---|---:|---|---|
| `SuperClass` | `+0x28` | `SuperWeaponTypeClass*`; case switch reads type enum at `Type+0xB4`. | Yes |
| `SuperClass` | `+0x2C` | Owner `HouseClass*`; spawner receives this in `ECX`. | Yes |
| `SuperClass` | `+0x6F` | Charged/ready gate checked before cases 5/6 continue. | Yes |
| `SuperWeaponTypeClass` | `+0xB4` | Type enum: 5 = `ParaDrop`, 6 = `AmerParaDrop`. | Yes |
| `HouseClass` | `+0x1E8` | Side selector for case 5: 0 Allies, 2 Yuri, else Soviet. | Yes |
| `CellClass` | `+0x38` | Surface/tile index used by `IsOnBridgeSurface`. | Yes |
| `CellClass` | `+0x140 bit 0x100` | Structural/high-bridge attribute used by the nearby-cell helper's optional bridge exclusion. | Yes |
| `DAT_00AA0738` | global | First bridge-surface tile index accepted by `IsOnBridgeSurface`; valid range is `[DAT_00AA0738, DAT_00AA0738 + 14)`. | Yes |
| `DAT_00ABDC50` | global | Map sentinel cell object. | Yes |
| `DAT_00ABD480` | global | Invalid/sentinel cell coordinate returned by `Find_Nearby_Passable_Cell` on failure. | Yes |

## 3. Core Logic

### 3.1 Cases 5 and 6 target flow

Verified behavior:

1. Case 5 and case 6 both start with the same ready gate: return if `SuperClass+0x6F` is zero. The case 6 gate is visible at `0x006CD537`; case 5 reaches its clicked-cell lookup at `0x006CD310` after the same gate and type lookup. Active in YR: Yes.
2. Both cases resolve the clicked cell with `MapClass::Get_CellClass` using the launch target pointer. Case 5 stores the original cell in `EBP`; case 6 stores it in `EDI`. If the cell pointer is null or `DAT_00ABDC50`, execution jumps to the common post-case cleanup without spawning. Active in YR: Yes.
3. Both cases call `CellClass::IsOnBridgeSurface` on the clicked cell. This helper is not a pathfinding query; it checks whether `CellClass+0x38` is within a 14-entry bridge-surface tile range. Active in YR: Yes.
4. If the clicked cell is not a bridge surface, the original cell is passed to the paradrop spawner. Active in YR: Yes.
5. If the clicked cell is a bridge surface, Launch calls `Find_Nearby_Passable_Cell` and reads the output coordinate. If that coordinate is not the invalid sentinel and maps to a valid non-sentinel cell whose `IsOnBridgeSurface` returns false, Launch replaces the target cell with that returned cell. Active in YR: Yes.
6. If the helper fails or returns a bridge-surface cell, Launch does not abort and does not clear the target. It keeps the original clicked bridge cell and proceeds into the per-side/per-list spawner loop. Active in YR: Yes.
7. For each valid list entry, the spawner receives the selected cell pointer as the target stack argument. Case 5 Allied/Yuri/Soviet calls at `0x006CD41A`, `0x006CD48C`, and `0x006CD4E4`; case 6 calls at `0x006CD64E`. Active in YR: Yes.

### 3.2 Exact nearby-cell call arguments

The assembly call in both cases pushes 15 stack arguments before calling `0x0056DC20` with `ECX = 0x0087F7E8` (the map object). The decoded call is:

| Parameter | Paradrop value | Verified effect in `0x0056DC20` | Active in YR |
|---|---:|---|---|
| `out_cell` | local output coord | Receives candidate or `DAT_00ABD480`. | Yes |
| `target` | clicked target coord pointer | Search center. | Yes |
| `param_4` | `0` | Forwarded into `CellRect::CheckPassability`. | Yes |
| `movement_zone` | `-1` | "Any zone"; only `0xFFFF` is normalized to `-1`, and paradrop already passes `-1`. | Yes |
| `param_6` | `0` | Forwarded into passability. | Yes |
| `bridge_height_input` | `0` | The helper does not add `+4` to base height for a bridge target because this flag is false. | Yes |
| `param_8` | `1` | Forwarded into passability. | Yes |
| `param_9` | `1` | Forwarded into passability. | Yes |
| `param_10` | `0` | Forwarded into passability. | Yes |
| `height_tol` | `0` | Height-difference gate is disabled. | Yes |
| `obstacle_free` | `0` | `Is_Current_Cell_Obstacle_Free` gate is disabled. | Yes |
| `allow_bridge_surface` | `1` | Bridge flag exclusion is disabled; bridge cells may enter the candidate list. | Yes |
| `preferred` | pointer to local `(0,0)` coord | If candidates are available, choose closest candidate to `(0,0)` after projection grouping, not random. | Yes |
| `skip_iter` | `0` | Do not skip the first half of each ring step. | Yes |
| `occupancy_check` | `0` | `CellRect::CheckOccupancy` gate is disabled. | Yes |

Correction to prior doc: `allow_bridge_surface` is `1`, not `0`. In `Find_Nearby_Passable_Cell`, the actual bridge-exclusion condition is "if this parameter is zero and `cell+0x140 & 0x100` is set, reject the candidate." Paradrop passes one, so this helper does not itself exclude bridge candidates.

### 3.3 Search radius, order, and selection

`Find_Nearby_Passable_Cell` starts from the clicked coordinate, looks up the cell, then computes a max ring count from map fields `this+0xF4 + this+0xF8`, capped at `0x20`. On normal maps this means rings `0..31` can be scanned; `0x18` is a candidate-buffer cap, not a 24-cell radius.

For each ring, the helper walks perimeter positions in four edge groups. It stops expanding once either 24 candidates have been collected or at least one candidate was found on the current ring. Candidate gates include on-screen check, `CellRect::CheckPassability`, optional height tolerance, optional obstacle-free check, optional bridge-surface exclusion, optional occupancy check, and a projection round-trip when `bridge_height_input` is zero.

After collection, candidates are split into projection-stable and projection-changing buckets via `FUN_006D6410`. If the preferred coordinate is not the invalid sentinel, the helper chooses the candidate closest to the preferred coordinate. Since paradrop passes a local `(0,0)` preferred coordinate, it chooses the candidate closest to `(0,0)` within the preferred bucket. If no candidates were collected, it writes `DAT_00ABD480`.

Active in YR: Yes. The helper is directly called by live cases 5/6.

## 4. INI Keys

| Key | Section | Stock YR value | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `Type=ParaDrop` | `[ParaDropSpecial]` | `ParaDrop` | Selects `SuperClass::Launch` case 5. | Yes |
| `Action=ParaDrop` | `[ParaDropSpecial]` | `ParaDrop` | Cursor/action binding before launch; not part of this validation branch. | Yes |
| `Type=AmerParaDrop` | `[AmericanParaDropSpecial]` | `AmerParaDrop` | Selects `SuperClass::Launch` case 6. | Yes |
| `Action=AmerParaDrop` | `[AmericanParaDropSpecial]` | `AmerParaDrop` | Cursor/action binding before launch; not part of this validation branch. | Yes |
| `AmerParaDropInf` / `AmerParaDropNum` | `[General]` | `E1` / `8` | Case 6 list/count. Spawner only runs if list counts match and entries are valid. | Yes |
| `AllyParaDropInf` / `AllyParaDropNum` | `[General]` | `E1` / `6` | Case 5 Allied branch. | Yes |
| `SovParaDropInf` / `SovParaDropNum` | `[General]` | `E2` / `9` | Case 5 fallback branch. | Yes |
| `YuriParaDropInf` / `YuriParaDropNum` | `[General]` | `INIT` / `6` | Case 5 Yuri branch. | Yes |

## 5. Integration Points

`FUN_006CB920` is the immediate caller of `SuperClass::Launch`; it handles ready/suspend bookkeeping and then calls Launch for ordinary fired superweapons. That caller is active for stock YR superweapon use. After target validation, cases 5/6 call `FUN_0065E660`, which creates the PDPLANE and assigns its destination/target from the exact cell pointer selected by Launch.

This slice did not re-investigate the PDPLANE spawner beyond confirming the target cell argument. Active in YR: Yes for the caller chain and spawner calls.

## 6. Current Rust Implementation Status

Current Rust `src/sim/superweapon/paradrop.rs` line 44 explicitly defers bridge rejection and uses the clicked target unchanged. That is closer to the binary fallback than the stale "abort on bridge" description, but still missing the binary's attempted replacement when a nearby non-bridge candidate is found.

Current Rust path/terrain surfaces expose bridge facts in `PathCell`/resolved terrain (`bridge_walkable`, `has_bridge_deck`, `has_structural_bridge`, `bridge_deck_level`), but `paradrop::launch` only receives `Option<&PathGrid>` and has no helper equivalent to the binary's launch-time "bridge surface tile range" predicate.

Active in YR: Rust status is an implementation comparison, not binary behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SuperClass::Launch` case 5 bridge branch | verified | Decompile `0x006CC390`; assembly `0x006CD310..0x006CD4FE` | none for launch-time target replacement |
| `SuperClass::Launch` case 6 bridge branch | verified | Decompile `0x006CC390`; assembly `0x006CD537..0x006CD66A` | none for launch-time target replacement |
| `CellClass::IsOnBridgeSurface` | verified | Decompile `0x00485060` | Exact runtime value of `DAT_00AA0738` not needed for this slice |
| `MapClass::Get_CellClass` sentinel behavior | verified | Decompile `0x005657A0` | none |
| `Find_Nearby_Passable_Cell` argument list | verified | Case 5 assembly `0x006CD33C..0x006CD375`; case 6 assembly `0x006CD57D..0x006CD5B6`; callee `RET 0x3C` at `0x0056E797` | none |
| `Find_Nearby_Passable_Cell` bridge-filter semantics | verified | Callee branches at `0x0056DE6C..0x0056DE80`, `0x0056E082..0x0056E096`, `0x0056E2C5..0x0056E2D9`, `0x0056E4C7..0x0056E4DB` | none |
| Search max radius vs candidate cap | verified | Callee `0x0056DCE1..0x0056DCF9`, candidate cap checks `0x0056DF2A`, `0x0056E141`, `0x0056E383`, `0x0056E578` | none |
| Destroyed bridge vs under-bridge click behavior | touched-not-exhausted | `IsOnBridgeSurface` uses `Cell+0x38`, not bridge intactness; no live scenario run | Runtime map state for destroyed bridge tile index remains a separate visual/click test |
| Parachute landing on bridge | deferred | User non-scope | handled by separate landing-layer report |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Which launch cases are in scope? -> Cases 5 and 6 only, selected by `Type=ParaDrop`/`Type=AmerParaDrop`.` (evidence: `rulesmd.ini`; `SuperClass::Launch 0x006CC390`)
- `[RESOLVED] OQ-02 -- Is this path active in YR? -> Yes, stock rules bind both superweapon types and `FUN_006CB920` calls Launch on fired-superweapon paths.` (evidence: `rulesmd.ini`; `FUN_006CB920`)
- `[RESOLVED] OQ-03 -- What tests a clicked bridge target? -> `CellClass::IsOnBridgeSurface`, which checks `Cell+0x38` against a 14-entry bridge-surface tile range.` (evidence: `0x00485060`)
- `[RESOLVED] OQ-04 -- Does Launch abort when the clicked cell is null/sentinel? -> Yes, it skips spawn loops and falls through cleanup.` (evidence: `0x006CD319..0x006CD327`, `0x006CD562..0x006CD56C`)
- `[RESOLVED] OQ-05 -- Does Launch abort when the clicked cell is a bridge surface? -> No, it attempts replacement; if replacement fails, original bridge cell remains the spawner target.` (evidence: `0x006CD32D..0x006CD3BF`, `0x006CD56E..0x006CD600`)
- `[RESOLVED] OQ-06 -- What are the exact helper arguments? -> `out,target,0,-1,0,0,1,1,0,0,0,1,&(0,0),0,0` with map object in `ECX`.` (evidence: `0x006CD33C..0x006CD375`, `0x006CD57D..0x006CD5B6`)
- `[RESOLVED] OQ-07 -- Does the helper exclude bridge cells for paradrop? -> No; the exclusion parameter is passed as one, so the `cell+0x140 & 0x100` rejection branch is disabled.` (evidence: `0x0056DE6C..0x0056DE80`; case call pushes)
- `[RESOLVED] OQ-08 -- What happens if helper returns sentinel? -> Launch leaves the original bridge cell in the target register and proceeds.` (evidence: `0x006CD37A..0x006CD3C1`, `0x006CD5BB..0x006CD602`)
- `[RESOLVED] OQ-09 -- What happens if helper returns another bridge-surface cell? -> Launch rejects only the replacement and proceeds with the original bridge cell.` (evidence: `0x006CD3B4..0x006CD3BF`, `0x006CD5F5..0x006CD600`)
- `[RESOLVED] OQ-10 -- What target is passed to the spawner? -> The current selected cell pointer: replacement if valid non-bridge, else original clicked cell.` (evidence: `PUSH EBP` at `0x006CD41A/0x006CD48C/0x006CD4E4`; `PUSH EDI` at `0x006CD64E`)
- `[RESOLVED] OQ-11 -- Is the search radius 24? -> No. `0x18` is the candidate cap. Ring limit is `min(map+0xF4 + map+0xF8, 0x20)`, so normal maps search up to ring 31 if no candidate is found earlier.` (evidence: `0x0056DCE1..0x0056DCF9`, `0x0056DF2A`)
- `[RESOLVED] OQ-12 -- Is the helper selection random? -> Not for paradrop, because preferred is local `(0,0)`, not the invalid sentinel; it chooses closest to preferred after bucket split.` (evidence: `0x0056E689..0x0056E797`)
- `[RESOLVED] OQ-13 -- Is occupancy checked? -> No, paradrop passes occupancy flag zero.` (evidence: `0x006CD343..0x006CD370`; optional occupancy branch `0x0056DE86..0x0056DEAA`)
- `[RESOLVED] OQ-14 -- Is height tolerance checked? -> No, paradrop passes height tolerance zero.` (evidence: optional height branch `0x0056DE1B..0x0056DE52`)
- `[RESOLVED] OQ-15 -- Is obstacle-free checked? -> No, paradrop passes obstacle-free zero.` (evidence: optional obstacle branch `0x0056DE52..0x0056DE6C`)
- `[RESOLVED] OQ-16 -- Does case 5 differ from case 6 in bridge validation? -> No material difference; both use the same bridge branch and helper argument pattern.` (evidence: paired case ranges above)
- `[DEFERRED] OQ-17 -- What exact tile index does a destroyed bridge deck click report at runtime?` (category: `needs-runtime-debugger`; reason: this requires a live scenario or map-state trace after bridge destruction; next-step-if-pursued: inspect `Cell+0x38` before/after bridge collapse under the tactical cursor)

Adversarial corner cases answered: null/sentinel clicked cell skips spawn; helper sentinel keeps original bridge; helper bridge result keeps original bridge; candidate cap is not radius; optional height/obstacle/occupancy gates are off for paradrop.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Bridge-surface clicked target attempts nearby replacement but does not abort if replacement fails. | `0x006CD32D..0x006CD3C1`, `0x006CD56E..0x006CD602` | Missing replacement attempt; current code always uses click target. | `src/sim/superweapon/paradrop.rs::launch` plus a map/path helper surface | If clicked cell is bridge-surface, run a nearby passable search; replace only when returned cell is valid and non-bridge. | `paradrop_bridge_click_replaces_target_when_nearby_non_bridge_found` | Do not implement "bridge click aborts launch"; binary keeps launching with original bridge target on failed replacement. |
| Helper bridge filter is permissive for paradrop; Launch does the non-bridge check after the helper returns. | Case call pushes `allow_bridge_surface=1`; callee exclusion only when param is zero | Current code lacks both helper call and post-check. | New helper or extension around `PathGrid`/resolved terrain | Keep bridge-surface filtering as a post-return replacement condition, not as a hard helper filter, unless a cleaner Rust helper can reproduce the same final output. | `paradrop_bridge_search_returning_bridge_keeps_original_target` | Do not copy the stale `bridge_filter=0 excludes bridges` claim. |
| `0x18` is candidate cap, not search radius; ring limit is map dimensions sum capped at `0x20`. | `0x0056DCE1..0x0056DCF9`; cap checks at `0x0056DF2A` etc. | Any future fixed radius 24 would mismatch. | Nearby-cell search helper if implemented | Search outward by rings until first candidate ring or cap/exhaustion; do not stop at radius 24. | `paradrop_bridge_search_can_consider_beyond_24_candidate_cap_not_radius` | Avoid naming the search limit `PARADROP_BRIDGE_RADIUS_24`. |
| Replacement uses `CellClass::IsOnBridgeSurface` tile-range semantics, not simply "has structural bridge". | `0x00485060`; Launch post-check calls same helper | Rust currently has bridge structural/deck flags but no equivalent surface predicate in paradrop launch. | Map/resolved terrain bridge-surface exposure, then `paradrop::launch` | Expose a launch-time bridge-surface predicate consistent with clicked cell/tile state; use it for both initial and returned-cell checks. | `paradrop_bridge_surface_predicate_does_not_treat_all_bridge_related_cells_as_rejected` | Do not reject every bridge-related cell, bridgehead, under-bridge ground cell, or destroyed bridge cell without matching `IsOnBridgeSurface` semantics. |
| Spawner receives the selected cell pointer after replacement logic. | `PUSH EBP` / `PUSH EDI` at spawner call sites | Current Rust passes unchanged `(target_rx,target_ry)` through spawn, mission, sound event. | `spawn_pdplane`, sound event target, aircraft mission target | Use the final selected target consistently for spawned carrier destination and launch event location if parity requires event at final target. | `paradrop_spawner_receives_replacement_cell_after_bridge_search` | Do not replace only the visual marker while leaving aircraft destination on the clicked bridge. |

Stale Docs / Follow-up Docs:

- In `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`, replace "dispatcher validates target (rejects bridge surfaces, finds nearby passable cell)" with "dispatcher attempts to replace bridge-surface targets with a nearby valid non-bridge cell; if no such replacement is accepted, it launches at the original bridge cell."
- Replace "Find_Nearby_Passable_Cell bridge_filter=0 excludes bridges and radius 24" with "paradrop passes the bridge-surface exclusion flag as enabled/allowing bridges; `0x18` is the candidate cap, while ring search is capped at 32."

## Negative Facts / Do Not Do

- Do not abort the whole paradrop launch just because the click target is a bridge surface. Active in YR: Yes; binary proceeds.
- Do not treat the nearby helper as excluding bridges for paradrop. Active in YR: Yes; argument value permits bridge candidates.
- Do not treat `0x18` as a radius. Active in YR: Yes; it is the candidate-list cap.
- Do not broaden launch-time rejection to all bridge-related cells. Active in YR: Yes; the checked predicate is the `Cell+0x38` bridge-surface tile range.
- Do not investigate or implement parachute landing-on-bridge behavior from this report; that is outside this launch slice.

## Remaining Uncertainty

The only material uncertainty inside this slice is the exact runtime `Cell+0x38` value for destroyed bridge remnants and under-bridge clicks in a live scenario. The code path is clear: if `IsOnBridgeSurface` returns false, no replacement is attempted. What remains is a runtime map-state question, not a Launch branch question.

## Sources

- Ghidra: `SuperClass::Launch` at `0x006CC390`
- Ghidra: `FUN_006CB920` immediate Launch caller
- Ghidra: `CellClass::IsOnBridgeSurface` at `0x00485060`
- Ghidra: `MapClass::Get_CellClass` at `0x005657A0`
- Ghidra: `Find_Nearby_Passable_Cell` at `0x0056DC20`
- Ghidra: `FUN_0065E660` paradrop spawner at `0x0065E660` spot-check for target-cell argument
- Prior doc checked: `C:/Users/enok/Documents/ra2-rust-game-docs/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`
- Rust checked: `src/sim/superweapon/paradrop.rs`, `src/sim/pathfinding/core.rs`, `src/map/resolved_terrain.rs`
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`
